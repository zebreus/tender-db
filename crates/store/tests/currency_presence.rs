//! Issue 371: `?currency=<absent>` must answer empty from a probe, not from a walk.
//!
//! `walks()` routes `currency` to the isolated reader pool (it is a per-row `EXISTS`
//! over `tender_version_amounts`, whose only index is `(tender_id, seq)`), and
//! `reachable()` had no leg for it. So `?currency=XXX` — a code present nowhere — was
//! ADMITTED, paid the full density-bounded walk, and held one of `SLOTS = 4` global
//! isolation slots for 29.78 s to answer nothing. Four such requests shed every other
//! isolated request on an unauthenticated surface.
//!
//! The leg seeks a PRESENT-SET the projection maintains (`tender_currency_presence`),
//! not an index on the column: the question is "does ANY row carry this value", asked
//! of a column with a few dozen distinct values over tens of millions of rows, and an
//! index would answer it by paying write amplification on every fold.
//!
//! **The direction of the set's error is the whole design.** It is a SUPERSET: entries
//! are only ever added, never removed when the last row carrying one goes away. A stale
//! entry makes the probe ADMIT and the read degrades to exactly the walk this guard
//! avoids — slow, correct. A MISSING entry would make the probe answer "no such
//! currency" while matching rows exist — fast, WRONG, silent. Every test here is written
//! to catch the second direction.
//!
//! On the MECHANISM. The guard changes SPEED and never RESULTS, so no assertion on a
//! returned page can distinguish a short-circuit from a walk that happened to find
//! nothing, and a wall-clock assertion would be a check that passes for the wrong reason
//! on a loaded box. Two things are asserted instead:
//!   * `read::reachable_for_test` — the verdict itself; and
//!   * a read that CANNOT complete as a query (the tables it drives from are dropped)
//!     but still answers, which is only possible if the walk never started.

use store::canonical::{Fact, TenderProjection, TenderVersion};
use store::read::{self, Collection, Filter, Scope};
use store::turso::{self, Value};

fn path(name: &str) -> String {
    format!("/tmp/tender-db-currency-{name}-{}.db", std::process::id())
}

fn wipe(path: &str) {
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

/// A second connection over the same file, which is how the other read-layer tests
/// reach `read::*` (the `Db`'s own connection is private).
async fn reader(path: &str) -> turso::Connection {
    let db = turso::Builder::new_local(path).build().await.unwrap();
    let conn = db.connect().unwrap();
    let mut rows = conn.query("PRAGMA journal_mode = WAL", ()).await.unwrap();
    while rows.next().await.unwrap().is_some() {}
    conn
}

/// One Tender at seq 1 carrying one amount, written by RAW SQL — the shape a fixture
/// builds when it is not driving the fold. Such a fixture must write the present-set
/// too (the fold does both in one transaction); `presence` says whether it does, so the
/// subset hazard can be exercised deliberately.
async fn seed_raw(conn: &turso::Connection, currency: &str, presence: bool) {
    conn.execute(
        "INSERT INTO tenders (id, source, procedure_key, kind, current_seq, current_published_at, created_at)
         VALUES (1, 'ted', 'pk-1', 'procedure', 1, 1700000000, 1700000000)",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO tender_versions (tender_id, seq, published_at, publication_id, caused_by_notice_id)
         VALUES (1, 1, 1700000000, 'pub-1', 1)",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO tender_version_amounts (tender_id, seq, lot_id, field, cents, currency)
         VALUES (1, 1, NULL, 'value', 1000, ?)",
        (Value::Text(currency.to_owned()),),
    )
    .await
    .unwrap();
    if presence {
        conn.execute(
            "INSERT OR IGNORE INTO tender_currency_presence(currency) VALUES(?)",
            (Value::Text(currency.to_owned()),),
        )
        .await
        .unwrap();
    }
}

fn currency(code: &str) -> Filter {
    Filter { currency: Some(code.to_owned()), ..Filter::default() }
}

async fn tender_ids(conn: &turso::Connection, f: &Filter) -> Vec<i64> {
    read::tenders(conn, f, Scope::Page { after: 0, limit: 100 })
        .await
        .unwrap()
        .into_iter()
        .map(|t| t.id)
        .collect()
}

/// How many present-set rows carry `code` — 0 or 1, since it is the primary key.
async fn recorded(db: &store::Db, code: &str) -> i64 {
    count(db, &format!("SELECT COUNT(*) FROM tender_currency_presence WHERE currency = '{code}'")).await
}

async fn count(db: &store::Db, sql: &str) -> i64 {
    match db.scalar(sql).await.unwrap() {
        Some(Value::Integer(n)) => n,
        other => panic!("expected a count from `{sql}`, got {other:?}"),
    }
}

async fn set_size(db: &store::Db) -> i64 {
    count(db, "SELECT COUNT(*) FROM tender_currency_presence").await
}

/// THE DEFECT AND THE FIX, on the mechanism.
///
/// `?currency=XXX` must be answered by the probe — asserted on `reachable`'s verdict,
/// not on a clock — while a code the corpus does carry still returns its rows.
#[tokio::test]
async fn an_absent_currency_is_answered_by_the_probe_and_a_present_one_still_returns_rows() {
    let path = path("probe");
    wipe(&path);
    let db = store::Db::open(&path).await.unwrap();
    db.set_foreign_keys(false).await.unwrap();
    // A fresh file has no amount rows, so `Db::open` attests the present-set covers
    // everything it carries: the guard is live from the first request.
    assert!(db.currency_presence_complete().await.unwrap(), "a fresh file needs no backfill");
    let conn = reader(&path).await;
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    seed_raw(&conn, "EUR", true).await;

    // The MECHANISM. `reachable` is the short-circuit itself: `false` means `tenders()`
    // returns an empty page without building or running the filtered query at all.
    for collection in [Collection::Tenders, Collection::Lots] {
        assert!(
            !read::reachable_for_test(&conn, &currency("XXX"), collection).await.unwrap(),
            "{collection:?}: a currency no row carries must be short-circuited, not walked"
        );
        assert!(
            read::reachable_for_test(&conn, &currency("EUR"), collection).await.unwrap(),
            "{collection:?}: a currency the corpus carries must be admitted to the query"
        );
    }

    // And the ANSWERS the mechanism produces, which must be exactly what the walk would
    // have produced: the present code returns its Tender, the absent one an empty page.
    assert_eq!(tender_ids(&conn, &currency("EUR")).await, vec![1], "a present currency matches");
    assert!(tender_ids(&conn, &currency("XXX")).await.is_empty(), "an absent currency is empty");

    // A present currency must not rescue an absent companion, and vice versa: the legs
    // are a conjunction, so any one absent leg empties the page.
    assert!(
        tender_ids(&conn, &Filter { country: Some("ZZ".into()), ..currency("EUR") }).await.is_empty(),
        "a present currency must not rescue an absent country"
    );

    // THE WALK NEVER STARTS — proven without a stopwatch. Drop the tables the filtered
    // query drives from: any read that reaches the query now FAILS, so an `Ok(empty)`
    // can only have come from the short-circuit ahead of it.
    conn.execute("DROP TABLE tender_versions", ()).await.unwrap();
    assert!(
        read::tenders(&conn, &currency("XXX"), Scope::Page { after: 0, limit: 100 })
            .await
            .unwrap()
            .is_empty(),
        "the absent-currency page must be answered before the query is built — with \
         tender_versions gone, a walk could only have errored"
    );
    assert!(
        read::tenders(&conn, &currency("EUR"), Scope::Page { after: 0, limit: 100 }).await.is_err(),
        "the negative control: a PRESENT currency is admitted, reaches the query, and \
         the query has no tender_versions to join — if this passed, the assertion \
         above would prove nothing about the short-circuit"
    );

    wipe(&path);
}

/// The SUBSET hazard, and the flag that exists to make it unreachable.
///
/// A file whose amount rows predate `tender_currency_presence` holds a set that is
/// MISSING codes its rows carry, and answering from that would hide real data. So the
/// guard declines entirely until the coverage flag attests the standing corpus — which
/// is exactly the behaviour that shipped before the set existed: a walk, and correct.
#[tokio::test]
async fn the_guard_declines_until_the_present_set_covers_the_corpus() {
    let path = path("uncovered");
    wipe(&path);
    let db = store::Db::open(&path).await.unwrap();
    db.set_foreign_keys(false).await.unwrap();
    let conn = reader(&path).await;
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();

    // The pre-371 file, reconstructed: amounts present, present-set empty, coverage
    // withdrawn. This is the state the `backfill-currencies` job exists to leave.
    seed_raw(&conn, "DEM", false).await;
    db.set_currency_presence_complete(false).await.unwrap();

    assert!(
        read::reachable_for_test(&conn, &currency("DEM"), Collection::Tenders).await.unwrap(),
        "an uncovered set must not be read from — the code IS present and the guard \
         would hide it"
    );
    assert_eq!(
        tender_ids(&conn, &currency("DEM")).await,
        vec![1],
        "and the full query, which the decline falls through to, still returns the row"
    );
    assert!(
        read::reachable_for_test(&conn, &currency("XXX"), Collection::Tenders).await.unwrap(),
        "the cost of declining: an ABSENT code walks too, exactly as it did before the \
         set existed. Slow and correct is the price of never being wrong"
    );

    // The sweep the job runs, then the attestation. Only now may the guard answer.
    let mut watermark = 0i64;
    loop {
        let (rows, next) = db.backfill_currency_presence(1_000, watermark).await.unwrap();
        if rows == 0 {
            break;
        }
        watermark = next;
    }
    db.set_currency_presence_complete(true).await.unwrap();
    assert_eq!(set_size(&db).await, 1, "the sweep found exactly the standing code");
    assert_eq!(recorded(&db, "DEM").await, 1, "…and it is the one the corpus carries");
    assert!(
        !read::reachable_for_test(&conn, &currency("XXX"), Collection::Tenders).await.unwrap(),
        "covered: the absent code is now short-circuited"
    );
    assert_eq!(
        tender_ids(&conn, &currency("DEM")).await,
        vec![1],
        "and the present one still returns its row — the sweep must not have built a subset"
    );

    wipe(&path);
}

/// One Tender, one version, one amount fact — the fold's own path, so the present-set
/// is maintained by the projection rather than by the fixture.
fn projection(key: &str, notice: i64, code: &str) -> TenderProjection {
    TenderProjection {
        source: "ted".into(),
        procedure_key: Some(key.into()),
        island_notice_id: None,
        kind: "procedure".into(),
        versions: vec![TenderVersion {
            caused_by_notice_id: notice,
            published_at: 1_700_000_000,
            dispatched_at: None,
            notice_subtype: None,
            original_lang: None,
            publication_id: format!("{notice}-2024"),
            facts: [Fact::Amount {
                field: "value".into(),
                cents: 1_000,
                currency: code.into(),
                tax_basis: None,
                quality: None,
            }]
            .into_iter()
            .collect(),
            lots: Vec::new(),
            rounds: Vec::new(),
            group_members: Vec::new(),
        }],
    }
}

/// The projection MAINTAINS the set — the property the guard's correctness rests on.
///
/// Written through the real `apply_tenders`, in the same transaction as the amount rows
/// (`Pending::flush`), because a set written afterwards could be observed as a subset by
/// a reader between the two commits, and a subset is the one state that answers wrongly.
#[tokio::test]
async fn the_fold_writes_every_currency_it_stores() {
    let path = path("fold");
    wipe(&path);
    let db = store::Db::open(&path).await.unwrap();
    db.set_foreign_keys(false).await.unwrap();

    db.apply_tenders(&[projection("k1", 1, "EUR"), projection("k2", 2, "SEK")], 1_700_000_000, false)
        .await
        .unwrap();
    assert_eq!(recorded(&db, "EUR").await, 1, "EUR recorded");
    assert_eq!(recorded(&db, "SEK").await, 1, "SEK recorded");
    assert_eq!(set_size(&db).await, 2, "and nothing else");

    // A code seen for the first time in a LATER batch is added; one already recorded is
    // a no-op (`INSERT OR IGNORE` into the primary key), so the set never duplicates.
    db.apply_tenders(&[projection("k3", 3, "EUR"), projection("k4", 4, "HRK")], 1_700_000_000, false)
        .await
        .unwrap();
    assert_eq!(recorded(&db, "HRK").await, 1, "a new code joins the set");
    assert_eq!(recorded(&db, "EUR").await, 1, "a repeat does not duplicate (INSERT OR IGNORE)");
    assert_eq!(set_size(&db).await, 3, "three codes for three codes");

    // The set covers what the fold wrote — never less. Asked of the amounts table itself
    // rather than of the list above, so a fold that starts writing a code by some other
    // route fails HERE rather than silently hiding rows at read time.
    assert_eq!(
        count(
            &db,
            "SELECT COUNT(*) FROM (SELECT DISTINCT currency FROM tender_version_amounts
              WHERE currency NOT IN (SELECT currency FROM tender_currency_presence))",
        )
        .await,
        0,
        "a code in tender_version_amounts and not in the present-set makes the set a \
         SUBSET, which answers `?currency=<that code>` empty for rows that exist"
    );

    // And the guard, end to end over a fold-built corpus.
    let conn = reader(&path).await;
    assert!(
        !read::reachable_for_test(&conn, &currency("XXX"), Collection::Tenders).await.unwrap(),
        "absent after a real fold"
    );
    assert!(
        read::reachable_for_test(&conn, &currency("HRK"), Collection::Tenders).await.unwrap(),
        "present after a real fold"
    );

    wipe(&path);
}

/// A full rebuild REPOPULATES the set.
///
/// The rebuild empties the tender-content layer (`reset_tender_layer`) and folds it
/// again from nothing. That is the one moment the set may SHRINK — every other path only
/// adds — so the wipe clears it and re-attests coverage (an empty layer is trivially
/// covered), and the refold rebuilds it fact by fact. Were the wipe to clear the set and
/// NOT the refold to repopulate it, every currency in the corpus would answer empty:
/// the subset failure, at maximum blast radius.
#[tokio::test]
async fn a_full_rebuild_clears_and_repopulates_the_set() {
    let path = path("rebuild");
    wipe(&path);
    let db = store::Db::open(&path).await.unwrap();
    db.set_foreign_keys(false).await.unwrap();

    db.apply_tenders(&[projection("k1", 1, "EUR")], 1_700_000_000, false).await.unwrap();
    // A code left over from a corpus that no longer exists — the superset case. It is
    // safe (it only costs a walk), and the rebuild is where it goes away.
    db.execute_for_test("INSERT INTO tender_currency_presence(currency) VALUES('OLD')")
        .await
        .unwrap();
    assert_eq!(set_size(&db).await, 2, "the folded code and the stale one");

    db.reset_tender_layer().await.unwrap();
    assert_eq!(set_size(&db).await, 0, "the wipe clears the set with the layer it describes");
    assert!(
        db.currency_presence_complete().await.unwrap(),
        "an empty layer is trivially covered, so the guard stays live across a rebuild"
    );

    db.apply_tenders(&[projection("k1", 1, "EUR")], 1_700_000_000, false).await.unwrap();
    assert_eq!(recorded(&db, "EUR").await, 1, "the refold repopulates the set");
    assert_eq!(
        set_size(&db).await,
        1,
        "…and the stale OLD entry is gone — a rebuild is the only shrink the design allows"
    );

    let conn = reader(&path).await;
    assert!(
        read::reachable_for_test(&conn, &currency("EUR"), Collection::Tenders).await.unwrap(),
        "a rebuilt corpus must not hide its own rows"
    );
    assert!(
        !read::reachable_for_test(&conn, &currency("OLD"), Collection::Tenders).await.unwrap(),
        "and the retired code short-circuits, because the rebuild re-derived the truth"
    );

    // `clear_canonical` — the other wipe, taken by `project --rebuild` — must agree.
    db.clear_canonical().await.unwrap();
    assert_eq!(set_size(&db).await, 0, "clear_canonical clears the set too");
    assert!(db.currency_presence_complete().await.unwrap(), "…and re-attests coverage");

    wipe(&path);
}
