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

use store::read::{self, Filter, Scope};
use store::turso::{self, Value};

const TENDER: i64 = 1;

/// The lots read exactly as it was before the fix: identity from the same joins,
/// summary from six correlated scalar subqueries. Kept as SQL text rather than as
/// a description of it, so the oracle cannot drift into agreeing by construction.
const ORACLE: &str = "
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
 WHERE 1 = 1 AND l.tender_id = ? AND (l.tender_id, l.id) > (?, ?)
 ORDER BY l.id LIMIT 1000";

#[derive(Debug, PartialEq)]
struct Summary {
    lot_key: String,
    kind: String,
    title: Option<String>,
    value_cents: Option<i64>,
    currency: Option<String>,
    deadline: Option<(i64, i64, bool)>,
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

async fn oracle(conn: &turso::Connection) -> Vec<Summary> {
    let mut rows = conn
        .query(ORACLE, (Value::Integer(TENDER), Value::Integer(TENDER), Value::Integer(0)))
        .await
        .unwrap();
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.unwrap() {
        out.push(Summary {
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

async fn text(conn: &turso::Connection, lot: i64, lang: Option<&str>, value: &str, field: &str) {
    conn.execute(
        "INSERT INTO tender_version_texts (tender_id, seq, lot_id, field, lang, value)
         VALUES (?, 1, ?, ?, ?, ?)",
        (
            Value::Integer(TENDER),
            Value::Integer(lot),
            Value::Text(field.to_owned()),
            lang.map_or(Value::Null, |l| Value::Text(l.to_owned())),
            Value::Text(value.to_owned()),
        ),
    )
    .await
    .unwrap();
}

async fn amount(conn: &turso::Connection, lot: i64, cents: i64, currency: &str) {
    conn.execute(
        "INSERT INTO tender_version_amounts (tender_id, seq, lot_id, field, cents, currency)
         VALUES (?, 1, ?, 'value', ?, ?)",
        (
            Value::Integer(TENDER),
            Value::Integer(lot),
            Value::Integer(cents),
            Value::Text(currency.to_owned()),
        ),
    )
    .await
    .unwrap();
}

async fn date(conn: &turso::Connection, lot: i64, field: &str, utc: i64, offset: i64, has: i64) {
    conn.execute(
        "INSERT INTO tender_version_dates
             (tender_id, seq, lot_id, field, utc_seconds, offset_minutes, has_time)
         VALUES (?, 1, ?, ?, ?, ?, ?)",
        (
            Value::Integer(TENDER),
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
    text(&conn, 1, Some("DEU"), "eins-de", "title").await;
    text(&conn, 1, Some("ENG"), "eins-en", "title").await;
    text(&conn, 1, Some("ENG"), "eins-en-again", "title").await;
    // 2: an unlabelled language against a real one. `(lang = 'ENG')` is NULL for
    //    the unlabelled row, and NULL sorts below 0 under DESC, so FRA wins.
    text(&conn, 2, None, "zwei-null", "title").await;
    text(&conn, 2, Some("FRA"), "zwei-fr", "title").await;
    // 3: a description but no title — the field filter must exclude it.
    text(&conn, 3, Some("ENG"), "drei-desc", "description").await;
    // 4: two amounts tied at the top with different currencies; the first wins.
    amount(&conn, 4, 100, "EUR").await;
    amount(&conn, 4, 500, "GBP").await;
    amount(&conn, 4, 500, "USD").await;
    amount(&conn, 4, 250, "CHF").await;
    // 5: deadlines out of order, plus a date of another field that must be ignored
    //    even though it is later than every deadline.
    date(&conn, 5, "submission_deadline", 1_800_000_500, 60, 1).await;
    date(&conn, 5, "submission_deadline", 1_800_009_000, 120, 0).await;
    date(&conn, 5, "submission_deadline", 1_800_000_100, 180, 1).await;
    date(&conn, 5, "planned_start", 1_900_000_000, 240, 1).await;
    // 6: a Part with the full set.
    text(&conn, 6, Some("ENG"), "sechs-en", "title").await;
    amount(&conn, 6, 900, "SEK").await;
    date(&conn, 6, "submission_deadline", 1_800_000_000, 0, 0).await;
    // 7: nothing at all — every summary field stays empty.
    // 8: a Tender-level row (lot_id NULL) must not leak onto a lot.
    conn.execute(
        "INSERT INTO tender_version_texts (tender_id, seq, lot_id, field, lang, value)
         VALUES (?, 1, NULL, 'title', 'ENG', 'tender-level')",
        (Value::Integer(TENDER),),
    )
    .await
    .unwrap();
    text(&conn, 8, Some("SWE"), "acht-sv", "title").await;

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

    let expected = oracle(&conn).await;
    let got: Vec<Summary> = read::lots(
        &conn,
        &Filter { tender: Some(TENDER), ..Filter::default() },
        Scope::Page { after: 0, limit: 1000 },
    )
    .await
    .unwrap()
    .into_iter()
    .map(|r| Summary {
        lot_key: r.lot_key,
        kind: r.kind,
        title: r.title,
        value_cents: r.value_cents,
        currency: r.currency,
        deadline: r.deadline.map(|d| (d.utc_seconds, d.offset_minutes, d.has_time)),
    })
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
        lot_key: "LOT-7".into(), kind: "Lot".into(),
        title: None, value_cents: None, currency: None, deadline: None,
    });
    assert_eq!(expected[7].title.as_deref(), Some("acht-sv"), "a Tender-level title is not a lot's");
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
