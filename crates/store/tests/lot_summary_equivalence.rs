//! Issue 115: the set-based lot summary must answer EXACTLY what the six
//! correlated subqueries answered.
//!
//! The fix moved four summary fields — title, value, currency, deadline — out of
//! SQL and into Rust. That is precisely the kind of rewrite that preserves the
//! common case and quietly changes the edges: which language wins, which of two
//! equal amounts supplies the currency, what an unlabelled `lang` sorts as.
//!
//! So this test does not assert what the author believes the picks should be. It
//! runs the ORIGINAL subquery SQL, verbatim, as an oracle over deliberately
//! awkward data, and requires the new code to agree with it row for row. If the
//! two ever diverge the test names the field and the lot.
//!
//! TWO picks have since diverged from the oracle ON PURPOSE, and this fixture
//! carries no row that triggers either, which is why it still passes unchanged:
//!
//!   * value — issue 389 unit 1 refuses a candidate the FOLD would refuse (a
//!     withheld marker, a sentinel, an amount over the ceiling). Pinned in
//!     `lot_value_election.rs`.
//!   * deadline — issue 389 unit 2 falls back to the procedure's deadline when the
//!     lot publishes none. Pinned in `lot_deadline_scope.rs`.
//!
//! Both are cases where agreeing with the pre-115 SQL would mean keeping a defect,
//! so they are pinned in their own files rather than by weakening this oracle. If
//! a future fixture row here starts tripping one, that is the signal to split it
//! out — not to relax the assertion.

use store::read::{self, Filter, Scope};
use store::turso::{self, Value};

const TENDER: i64 = 1;

/// The lots read exactly as it was before the fix: identity from the same joins,
/// summary from six correlated scalar subqueries. Kept as SQL text rather than as
/// a description of it, so the oracle cannot drift into agreeing by construction.
///
/// Pre-115 this was ONE shape for both scopes — the tender-scoped page and the
/// global stream differed only in the tail after `WHERE 1 = 1`, which is why the
/// body is shared here and each test appends its own tail. That is the pre-fix
/// builder's own structure, not a convenience of the test.
const ORACLE_BODY: &str = "
SELECT l.id, l.tender_id, l.lot_key, vl.kind, v.seq,
       (SELECT s.value FROM tender_version_texts s
         WHERE s.tender_id = t.id AND s.seq = v.seq AND s.lot_id = l.id AND s.field = 'title'
         ORDER BY (s.lang = 'ENG') DESC LIMIT 1),
       (SELECT MAX(a.cents) FROM tender_version_amounts a
         WHERE a.tender_id = t.id AND a.seq = v.seq AND a.lot_id = l.id),
       (SELECT s.currency FROM tender_version_amounts s
         WHERE s.tender_id = t.id AND s.seq = v.seq AND s.lot_id = l.id
         ORDER BY s.cents DESC LIMIT 1),
       (SELECT s.utc_seconds FROM tender_version_dates s
         WHERE s.tender_id = t.id AND s.seq = v.seq AND s.lot_id = l.id
           AND s.field = 'submission_deadline' ORDER BY s.utc_seconds DESC LIMIT 1),
       (SELECT s.offset_minutes FROM tender_version_dates s
         WHERE s.tender_id = t.id AND s.seq = v.seq AND s.lot_id = l.id
           AND s.field = 'submission_deadline' ORDER BY s.utc_seconds DESC LIMIT 1),
       (SELECT s.has_time FROM tender_version_dates s
         WHERE s.tender_id = t.id AND s.seq = v.seq AND s.lot_id = l.id
           AND s.field = 'submission_deadline' ORDER BY s.utc_seconds DESC LIMIT 1)
  FROM lots l
  JOIN tenders t ON t.id = l.tender_id
  JOIN tender_versions v ON v.tender_id = t.id
   AND v.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id)
  JOIN tender_version_lots vl
    ON vl.tender_id = t.id AND vl.seq = v.seq AND vl.lot_id = l.id
 WHERE 1 = 1";

/// The tender-scoped tail: the row-value cursor `1830d50` gave this read.
const SCOPED_TAIL: &str = " AND l.tender_id = ? AND (l.tender_id, l.id) > (?, ?)
 ORDER BY l.id LIMIT 1000";

/// The unfiltered tail — the global `/v1/lots` stream, whose page spans many
/// Tenders and so many versions. `1830d50` left this arm on the plain cursor and
/// `2751ce3` left its SQL alone; only the decoration moved.
const STREAM_TAIL: &str = " AND l.id > ? ORDER BY l.id LIMIT ?";

#[derive(Debug, PartialEq)]
struct Summary {
    tender_id: i64,
    lot_key: String,
    kind: String,
    title: Option<String>,
    value_cents: Option<i64>,
    currency: Option<String>,
    deadline: Option<(i64, i64, bool)>,
}

/// The same fields the oracle reads, taken off what `read::lots` returns.
fn summary(r: read::LotRow) -> Summary {
    Summary {
        tender_id: r.tender_id,
        lot_key: r.lot_key,
        kind: r.kind,
        title: r.title,
        value_cents: r.value_cents,
        currency: r.currency,
        deadline: r.deadline.map(|d| (d.utc_seconds, d.offset_minutes, d.has_time)),
    }
}

async fn drain(conn: &turso::Connection, sql: &str) {
    let mut rows = conn.query(sql, ()).await.unwrap();
    while rows.next().await.unwrap().is_some() {}
}

fn opt_i(row: &turso::Row, i: usize) -> Option<i64> {
    row.get_value(i).ok().and_then(|v| v.as_integer().copied())
}
fn opt_s(row: &turso::Row, i: usize) -> Option<String> {
    row.get_value(i).ok().and_then(|v| v.as_text().cloned())
}

async fn oracle(conn: &turso::Connection, tail: &str, params: Vec<Value>) -> Vec<Summary> {
    let mut rows = conn.query(&format!("{ORACLE_BODY}{tail}"), params).await.unwrap();
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.unwrap() {
        out.push(Summary {
            tender_id: opt_i(&row, 1).unwrap(),
            lot_key: opt_s(&row, 2).unwrap(),
            kind: opt_s(&row, 3).unwrap(),
            title: opt_s(&row, 5),
            value_cents: opt_i(&row, 6),
            currency: opt_s(&row, 7),
            deadline: opt_i(&row, 8).map(|utc| {
                (utc, opt_i(&row, 9).unwrap_or(0), opt_i(&row, 10).unwrap_or(0) != 0)
            }),
        });
    }
    out
}

/// Run the pre-fix SQL and `read::lots` over the same page, and require them to
/// agree row for row. Returns the oracle's answer, so a caller can then check that
/// the oracle itself says something.
async fn agree(
    conn: &turso::Connection,
    label: &str,
    tail: &str,
    params: Vec<Value>,
    filter: &Filter,
    scope: Scope,
) -> Vec<Summary> {
    let expected = oracle(conn, tail, params).await;
    let got: Vec<Summary> =
        read::lots(conn, filter, scope).await.unwrap().into_iter().map(summary).collect();
    for (want, have) in expected.iter().zip(got.iter()) {
        assert_eq!(
            want, have,
            "{label}: tender {} lot {} disagrees with the pre-fix SQL",
            want.tender_id, want.lot_key
        );
    }
    assert_eq!(expected.len(), got.len(), "{label}: row count differs from the pre-fix SQL");

    // Issue 221: `lots_identity` is `lots` WITHOUT the summary decoration — the same
    // match set, same order, just no title/value/deadline. `read_matches` (the SSE
    // diff classifier) leans on exactly that equivalence to skip the whole-slice
    // `summarise` per lot change, so pin it here where a fixture already drives every
    // filter/scope shape through the query.
    let ident = read::lots_identity(conn, filter, scope).await.unwrap();
    let full = read::lots(conn, filter, scope).await.unwrap();
    let key = |r: &read::LotRow| (r.id, r.tender_id, r.lot_key.clone(), r.kind.clone(), r.seq);
    assert_eq!(
        ident.iter().map(key).collect::<Vec<_>>(),
        full.iter().map(key).collect::<Vec<_>>(),
        "{label}: lots_identity must match lots on the identity columns"
    );
    assert!(
        ident.iter().all(|r| r.title.is_none() && r.value_cents.is_none() && r.deadline.is_none()),
        "{label}: lots_identity carries no summary decoration"
    );
    expected
}

/// A Tender with one version per `seqs`; the last is its current one.
async fn tender(conn: &turso::Connection, id: i64, source: &str, seqs: &[i64]) {
    conn.execute(
        "INSERT INTO tenders (id, source, procedure_key, kind, current_seq, current_published_at, created_at)
         VALUES (?, ?, ?, 'procedure', ?, 1700000000, 1700000000)",
        (
            Value::Integer(id),
            Value::Text(source.to_owned()),
            Value::Text(format!("pk-{id}")),
            Value::Integer(*seqs.last().unwrap()),
        ),
    ).await.unwrap();
    for seq in seqs {
        // `caused_by_notice_id` must differ per version: `tender_versions` is UNIQUE
        // on `(tender_id, caused_by_notice_id)` — one version per causing notice.
        conn.execute(
            "INSERT INTO tender_versions (tender_id, seq, published_at, publication_id, caused_by_notice_id)
             VALUES (?, ?, 1700000000, ?, ?)",
            (
                Value::Integer(id),
                Value::Integer(*seq),
                Value::Text(format!("pub-{id}-{seq}")),
                Value::Integer(id * 100 + seq),
            ),
        ).await.unwrap();
    }
}

/// A Lot, published by one version of its Tender.
async fn lot(conn: &turso::Connection, id: i64, (owner, seq): (i64, i64), key: &str, kind: &str) {
    conn.execute(
        "INSERT INTO lots (id, tender_id, lot_key) VALUES (?, ?, ?)",
        (Value::Integer(id), Value::Integer(owner), Value::Text(key.to_owned())),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO tender_version_lots (tender_id, seq, lot_id, kind) VALUES (?, ?, ?, ?)",
        (
            Value::Integer(owner),
            Value::Integer(seq),
            Value::Integer(id),
            Value::Text(kind.to_owned()),
        ),
    )
    .await
    .unwrap();
}

async fn text(
    conn: &turso::Connection,
    (tender, seq): (i64, i64),
    lot: i64,
    lang: Option<&str>,
    value: &str,
    field: &str,
) {
    conn.execute(
        "INSERT INTO tender_version_texts (tender_id, seq, lot_id, field, lang, value)
         VALUES (?, ?, ?, ?, ?, ?)",
        (
            Value::Integer(tender),
            Value::Integer(seq),
            Value::Integer(lot),
            Value::Text(field.to_owned()),
            lang.map_or(Value::Null, |l| Value::Text(l.to_owned())),
            Value::Text(value.to_owned()),
        ),
    )
    .await
    .unwrap();
}

async fn amount(conn: &turso::Connection, (tender, seq): (i64, i64), lot: i64, cents: i64, currency: &str) {
    conn.execute(
        "INSERT INTO tender_version_amounts (tender_id, seq, lot_id, field, cents, currency)
         VALUES (?, ?, ?, 'value', ?, ?)",
        (
            Value::Integer(tender),
            Value::Integer(seq),
            Value::Integer(lot),
            Value::Integer(cents),
            Value::Text(currency.to_owned()),
        ),
    )
    .await
    .unwrap();
}

async fn date(
    conn: &turso::Connection,
    (tender, seq): (i64, i64),
    lot: i64,
    field: &str,
    utc: i64,
    offset: i64,
    has: i64,
) {
    conn.execute(
        "INSERT INTO tender_version_dates
             (tender_id, seq, lot_id, field, utc_seconds, offset_minutes, has_time)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
        (
            Value::Integer(tender),
            Value::Integer(seq),
            Value::Integer(lot),
            Value::Text(field.to_owned()),
            Value::Integer(utc),
            Value::Integer(offset),
            Value::Integer(has),
        ),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn set_based_lot_summary_agrees_with_the_correlated_subqueries() {
    let path = format!("/tmp/tender-db-lotequiv-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    store::Db::open(&path).await.unwrap();
    let db = turso::Builder::new_local(&path).build().await.unwrap();
    let conn = db.connect().unwrap();
    drain(&conn, "PRAGMA journal_mode = WAL").await;
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();

    conn.execute(
        "INSERT INTO tenders (id, source, procedure_key, kind, current_seq, current_published_at, created_at)
         VALUES (?, 'ted', 'pk', 'procedure', 1, 1700000000, 1700000000)",
        (Value::Integer(TENDER),),
    ).await.unwrap();
    conn.execute(
        "INSERT INTO tender_versions (tender_id, seq, published_at, publication_id, caused_by_notice_id)
         VALUES (?, 1, 1700000000, 'pub', 1)",
        (Value::Integer(TENDER),),
    ).await.unwrap();

    // Eight lots, each an edge the rewrite could have got wrong.
    let kinds = ["Lot", "Lot", "Lot", "LotsGroup", "Lot", "Part", "Lot", "Lot"];
    for (i, kind) in kinds.iter().enumerate() {
        conn.execute(
            "INSERT INTO lots (id, tender_id, lot_key) VALUES (?, ?, ?)",
            (
                Value::Integer(i as i64 + 1),
                Value::Integer(TENDER),
                Value::Text(format!("LOT-{}", i + 1)),
            ),
        )
        .await
        .unwrap();
        conn.execute(
            "INSERT INTO tender_version_lots (tender_id, seq, lot_id, kind) VALUES (?, 1, ?, ?)",
            (
                Value::Integer(TENDER),
                Value::Integer(i as i64 + 1),
                Value::Text((*kind).to_owned()),
            ),
        )
        .await
        .unwrap();
    }

    // 1: a non-English title first, then two English ones — the second ENG must
    //    never displace the first.
    text(&conn, (TENDER, 1), 1, Some("DEU"), "eins-de", "title").await;
    text(&conn, (TENDER, 1), 1, Some("ENG"), "eins-en", "title").await;
    text(&conn, (TENDER, 1), 1, Some("ENG"), "eins-en-again", "title").await;
    // 2: an unlabelled language against a real one. `(lang = 'ENG')` is NULL for
    //    the unlabelled row, and NULL sorts below 0 under DESC, so FRA wins.
    text(&conn, (TENDER, 1), 2, None, "zwei-null", "title").await;
    text(&conn, (TENDER, 1), 2, Some("FRA"), "zwei-fr", "title").await;
    // 3: a description but no title — the field filter must exclude it.
    text(&conn, (TENDER, 1), 3, Some("ENG"), "drei-desc", "description").await;
    // 4: two amounts tied at the top with different currencies; the first wins.
    amount(&conn, (TENDER, 1), 4, 100, "EUR").await;
    amount(&conn, (TENDER, 1), 4, 500, "GBP").await;
    amount(&conn, (TENDER, 1), 4, 500, "USD").await;
    amount(&conn, (TENDER, 1), 4, 250, "CHF").await;
    // 5: deadlines out of order, plus a date of another field that must be ignored
    //    even though it is later than every deadline.
    date(&conn, (TENDER, 1), 5, "submission_deadline", 1_800_000_500, 60, 1).await;
    date(&conn, (TENDER, 1), 5, "submission_deadline", 1_800_009_000, 120, 0).await;
    date(&conn, (TENDER, 1), 5, "submission_deadline", 1_800_000_100, 180, 1).await;
    date(&conn, (TENDER, 1), 5, "planned_start", 1_900_000_000, 240, 1).await;
    // 6: a Part with the full set.
    text(&conn, (TENDER, 1), 6, Some("ENG"), "sechs-en", "title").await;
    amount(&conn, (TENDER, 1), 6, 900, "SEK").await;
    date(&conn, (TENDER, 1), 6, "submission_deadline", 1_800_000_000, 0, 0).await;
    // 7: nothing at all — every summary field stays empty.
    // 8: a Tender-level row (lot_id NULL) must not leak onto a lot.
    conn.execute(
        "INSERT INTO tender_version_texts (tender_id, seq, lot_id, field, lang, value)
         VALUES (?, 1, NULL, 'title', 'ENG', 'tender-level')",
        (Value::Integer(TENDER),),
    )
    .await
    .unwrap();
    text(&conn, (TENDER, 1), 8, Some("SWE"), "acht-sv", "title").await;

    // An issue-103 orphan: a `tender_version_lots` row of THIS Tender pointing at a
    // Lot that belongs to another one. Both shapes must exclude it, by different
    // means — the old one because it drives from `lots` filtered on `l.tender_id`,
    // the new one through its explicit `l.tender_id = vl.tender_id` guard. They
    // agree today, so this asserts nothing about the present; it exists to LOCK the
    // guard, so that deleting it turns the new shape into a superset of the old and
    // fails this test rather than silently leaking another Tender's Lot.
    conn.execute(
        "INSERT INTO tenders (id, source, procedure_key, kind, current_seq, current_published_at, created_at)
         VALUES (99, 'ted', 'pk-other', 'procedure', 1, 1700000000, 1700000000)",
        (),
    ).await.unwrap();
    conn.execute(
        "INSERT INTO lots (id, tender_id, lot_key) VALUES (999, 99, 'FOREIGN-LOT')",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO tender_version_lots (tender_id, seq, lot_id, kind) VALUES (?, 1, 999, 'Lot')",
        (Value::Integer(TENDER),),
    )
    .await
    .unwrap();

    let expected = oracle(
        &conn,
        SCOPED_TAIL,
        vec![Value::Integer(TENDER), Value::Integer(TENDER), Value::Integer(0)],
    )
    .await;
    let got: Vec<Summary> = read::lots(
        &conn,
        &Filter { tender: Some(TENDER), ..Filter::default() },
        Scope::Page { after: 0, limit: 1000 },
    )
    .await
    .unwrap()
    .into_iter()
    .map(summary)
    .collect();

    assert_eq!(expected.len(), 8, "the oracle must see all eight lots");
    for (want, have) in expected.iter().zip(got.iter()) {
        assert_eq!(want, have, "lot {} disagrees with the pre-fix SQL", want.lot_key);
    }
    assert_eq!(expected.len(), got.len(), "row count differs from the pre-fix SQL");

    // Spot-check that the oracle itself encodes the edges we think it does — a
    // silently-empty oracle would agree with anything.
    assert_eq!(expected[0].title.as_deref(), Some("eins-en"), "first ENG title wins");
    assert_eq!(expected[1].title.as_deref(), Some("zwei-fr"), "a labelled language beats NULL");
    assert_eq!(expected[2].title, None, "a description is not a title");
    assert_eq!(expected[3].value_cents, Some(500));
    assert_eq!(expected[3].currency.as_deref(), Some("GBP"), "first of the tied maxima");
    assert_eq!(expected[4].deadline, Some((1_800_009_000, 120, false)), "latest deadline, its own offset");
    assert_eq!(expected[6], Summary {
        tender_id: TENDER, lot_key: "LOT-7".into(), kind: "Lot".into(),
        title: None, value_cents: None, currency: None, deadline: None,
    });
    assert_eq!(expected[7].title.as_deref(), Some("acht-sv"), "a Tender-level title is not a lot's");
    // ADR-0013 D3: a requested language outranks the ENG default in the same
    // ladder — asserted against the SAME fixture so the default path above
    // stays byte-pinned by the oracle while the requested path is pinned here.
    // Lot 1 flips to its DEU variant; lot 2 (no DEU variant) keeps its
    // labelled-beats-NULL answer; lot 8 keeps its only (SWE) title.
    let with_lang: Vec<Summary> = read::lots(
        &conn,
        &Filter { tender: Some(TENDER), lang: Some("DEU".into()), ..Filter::default() },
        Scope::Page { after: 0, limit: 1000 },
    )
    .await
    .unwrap()
    .into_iter()
    .map(summary)
    .collect();
    assert_eq!(with_lang[0].title.as_deref(), Some("eins-de"), "requested DEU outranks ENG");
    assert_eq!(with_lang[1].title.as_deref(), Some("zwei-fr"), "no DEU variant — chain falls back");
    assert_eq!(with_lang[7].title.as_deref(), Some("acht-sv"), "single-language lots unchanged");

    assert!(
        !got.iter().any(|s| s.lot_key == "FOREIGN-LOT"),
        "another Tender's Lot reached this Tender's answer — the \
         `l.tender_id = vl.tender_id` containment guard in read::lots is gone (issue 103)"
    );

    // The kind filter still selects on the version's lot kind, through the new
    // driving table.
    let parts = read::lots(
        &conn,
        &Filter { tender: Some(TENDER), kind: Some("Part".into()), ..Filter::default() },
        Scope::Page { after: 0, limit: 1000 },
    )
    .await
    .unwrap();
    assert_eq!(parts.len(), 1, "one Part");
    assert_eq!(parts[0].lot_key, "LOT-6");
    assert_eq!(parts[0].title.as_deref(), Some("sechs-en"), "a filtered page is decorated too");

    // The cursor still means "after this lot id", now that it is a plain `l.id > ?`
    // against a vl-driven query.
    for after in [0i64, 1, 4, 8, 99] {
        let page = read::lots(
            &conn,
            &Filter { tender: Some(TENDER), ..Filter::default() },
            Scope::Page { after, limit: 1000 },
        )
        .await
        .unwrap();
        let want: Vec<String> =
            expected.iter().skip(after.max(0) as usize).map(|s| s.lot_key.clone()).collect();
        let have: Vec<String> = page.iter().map(|r| r.lot_key.clone()).collect();
        assert_eq!(want, have, "cursor after={after}");
    }

    // And the limit still bounds the page.
    let three = read::lots(
        &conn,
        &Filter { tender: Some(TENDER), ..Filter::default() },
        Scope::Page { after: 0, limit: 3 },
    )
    .await
    .unwrap();
    assert_eq!(three.len(), 3, "limit bounds a tender-scoped page");

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

// ------------------------------------------------- the unfiltered list

/// The test above pins the tender-scoped arm, where every row of a page shares one
/// `(tender_id, seq)`. The general `/v1/lots` list — the highest-traffic of the two
/// — does not: one page spans many Tenders, each at its own current `seq`, and
/// `summarise` reads a satellite slice PER VERSION and matches its rows back to the
/// page by `lot_id`. That matching has no counterpart in the SQL it replaced, where
/// each subquery was already pinned to its own row's `t.id` and `v.seq`. So it is
/// exactly the part of the fix the first test cannot reach, and it gets its own
/// oracle over data built to break it:
///
///   * three Tenders at three different current seqs, one of them reusing another's
///     STALE seq number, so a match on `seq` alone crosses Tenders;
///   * superseded versions carrying a later deadline, a larger amount and an English
///     title than the current one, so leaking a stale slice is visible in the value
///     rather than only in the row count;
///   * lot keys repeated across Tenders, so nothing may key on `lot_key`;
///   * a satellite row of one Tender's current version pointing at ANOTHER Tender's
///     lot — the issue-103 orphan shape, in the satellites rather than in
///     `tender_version_lots`;
///   * pages that cut across a Tender boundary, so a version's slice is read for a
///     page holding only some of its lots.
#[tokio::test]
async fn the_unfiltered_list_agrees_across_many_versions_in_one_page() {
    let path = format!("/tmp/tender-db-lotstream-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    store::Db::open(&path).await.unwrap();
    let db = turso::Builder::new_local(&path).build().await.unwrap();
    let conn = db.connect().unwrap();
    drain(&conn, "PRAGMA journal_mode = WAL").await;
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();

    // A is at seq 2, B at seq 1 — B's CURRENT seq is A's STALE one — and C at 3.
    tender(&conn, 10, "ted", &[1, 2]).await;
    tender(&conn, 20, "de", &[1]).await;
    tender(&conn, 30, "ted", &[2, 3]).await;
    let (a, b, c) = ((10, 2), (20, 1), (30, 3));

    // Ids ascend across Tenders, so an id-ordered page walks A, then B, then C. The
    // lot keys deliberately repeat: only `(tender_id, id)` identifies a lot.
    lot(&conn, 101, a, "LOT-1", "Lot").await;
    lot(&conn, 102, a, "LOT-2", "LotsGroup").await;
    lot(&conn, 201, b, "LOT-1", "Lot").await;
    lot(&conn, 202, b, "LOT-2", "Part").await;
    lot(&conn, 301, c, "LOT-1", "Lot").await;
    lot(&conn, 302, c, "LOT-2", "Lot").await;

    // A's current version: the language pick, and a tied maximum amount.
    text(&conn, a, 101, Some("DEU"), "a1-de", "title").await;
    text(&conn, a, 101, Some("ENG"), "a1-en", "title").await;
    amount(&conn, a, 102, 100, "EUR").await;
    amount(&conn, a, 102, 500, "GBP").await;
    amount(&conn, a, 102, 500, "USD").await;
    // A's SUPERSEDED version, every field beating the current one. Reading it would
    // change an answer, not merely add a row.
    text(&conn, (10, 1), 101, Some("ENG"), "a1-STALE", "title").await;
    amount(&conn, (10, 1), 102, 900_000, "XXX").await;
    date(&conn, (10, 1), 101, "submission_deadline", 1_900_000_000, 0, 1).await;

    // B's current version, at the seq number A superseded.
    text(&conn, b, 201, None, "b1-null", "title").await;
    text(&conn, b, 201, Some("FRA"), "b1-fr", "title").await;
    date(&conn, b, 201, "submission_deadline", 1_800_000_500, 60, 1).await;
    date(&conn, b, 201, "submission_deadline", 1_800_009_000, 120, 0).await;
    // B's lot 202 is bare, and must stay bare.

    // C's current version.
    text(&conn, c, 301, Some("SWE"), "c1-sv", "title").await;
    amount(&conn, c, 301, 900, "SEK").await;
    date(&conn, c, 301, "submission_deadline", 1_800_000_000, 0, 0).await;
    // C's superseded version, aimed at the lot that is otherwise bare.
    text(&conn, (30, 2), 302, Some("ENG"), "c2-STALE", "title").await;

    // The orphan: rows of C's CURRENT version naming a lot that belongs to B. The
    // pre-fix SQL cannot see them — its subqueries read `s.tender_id = t.id`, and
    // lot 202's `t` is B. Nothing may reach lot 202 through the fact that C's slice
    // mentions it.
    text(&conn, c, 202, Some("ENG"), "cross-tender", "title").await;
    amount(&conn, c, 202, 4242, "PLN").await;
    date(&conn, c, 202, "submission_deadline", 1_850_000_000, 30, 1).await;

    let page = |after: i64, limit: i64| {
        (vec![Value::Integer(after), Value::Integer(limit)], Scope::Page { after, limit })
    };

    // The whole list in one page: six lots, three versions, one call to `summarise`.
    let (params, scope) = page(0, 1000);
    let all = agree(&conn, "whole list", STREAM_TAIL, params, &Filter::default(), scope).await;

    // The oracle must actually encode the edges, or it would agree with anything.
    assert_eq!(all.len(), 6, "the oracle must see all six lots");
    assert_eq!(all[0].title.as_deref(), Some("a1-en"), "the current version's ENG title");
    assert_eq!(all[0].deadline, None, "a superseded version supplies no deadline");
    assert_eq!(all[1].value_cents, Some(500), "the current version's amount, not the stale one");
    assert_eq!(all[1].currency.as_deref(), Some("GBP"), "first of the tied maxima");
    assert_eq!(all[2].title.as_deref(), Some("b1-fr"), "B is read at its own seq");
    assert_eq!(all[2].deadline, Some((1_800_009_000, 120, false)), "the latest deadline");
    assert_eq!(
        all[3],
        Summary {
            tender_id: 20,
            lot_key: "LOT-2".into(),
            kind: "Part".into(),
            title: None,
            value_cents: None,
            currency: None,
            deadline: None,
        },
        "another Tender's satellite slice decorated this lot — `summarise` matched a \
         satellite row to a page row by `lot_id` alone, where the SQL it replaced \
         pinned `s.tender_id = t.id`"
    );
    assert_eq!(all[4].title.as_deref(), Some("c1-sv"), "C at seq 3");
    assert_eq!(all[5].title, None, "C's superseded seq 2 supplies no title");

    // Pages that cut across Tender boundaries: a version's slice is read for a page
    // holding only some of its lots, and the pages must still concatenate to the
    // whole list.
    let mut walked: Vec<Summary> = Vec::new();
    for after in [0i64, 102, 202] {
        let (params, scope) = page(after, 2);
        walked.extend(
            agree(&conn, &format!("page after={after}"), STREAM_TAIL, params, &Filter::default(), scope)
                .await,
        );
    }
    assert_eq!(walked, all, "paging the list does not answer what reading it whole does");

    // The list's own filters, on the same multi-version page. Pushed ahead of the
    // cursor, exactly as the pre-fix builder pushed them.
    let (mut params, scope) = page(0, 1000);
    params.insert(0, Value::Text("ted".into()));
    agree(
        &conn,
        "source=ted",
        " AND t.source = ? AND l.id > ? ORDER BY l.id LIMIT ?",
        params,
        &Filter { source: Some("ted".into()), ..Filter::default() },
        scope,
    )
    .await;

    let (mut params, scope) = page(0, 1000);
    params.insert(0, Value::Text("Lot".into()));
    let lots_only = agree(
        &conn,
        "kind=Lot",
        " AND vl.kind = ? AND l.id > ? ORDER BY l.id LIMIT ?",
        params,
        &Filter { kind: Some("Lot".into()), ..Filter::default() },
        scope,
    )
    .await;
    assert_eq!(lots_only.len(), 4, "four lots of kind Lot, spanning three Tenders");

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}
