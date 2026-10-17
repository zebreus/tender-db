//! Issue 560: a version that moved only the Tender's elected EUR value wrote no lot change
//! row, so a `/v1/lots?min_value=`/`max_value=` subscriber (a lot's value bound reads its
//! Tender's value, issue 490 decision 6) kept a lot its bound now excludes: a lots stream reads
//! only `lot` rows. The decision (A-EUR, 2026-10-17): a version whose elected EUR value moved
//! is a `changed` row for the Tender and for every lot present on both sides, beside 533's
//! closure flip, compared in EUR cents (NULL to a value and back count), with
//! - the net-move rule: the run's last version is also compared against the value before the
//!   run's first written version, because SSE probes a version that is not the head as a
//!   superset on both sides;
//! - the seeds: a kept prefix's value (its figures, then `set_head` of its last version), the
//!   STORED head value on a pure append, and the compared prefix's last value in `On` mode.
//!
//! What this file pins, every case through the FOLD (`apply_tenders`), in the style of
//! `closed_by_notice.rs`: which change rows each version writes.

use store::canonical::{Fact, LotState, TenderProjection, TenderVersion};
use store::turso::{self, Value};

const DAY: i64 = 86_400;
/// 2026-01-02T12:00:00Z: every chain's first version.
const T0: i64 = 1_767_355_200;
/// The fold's `now`, after every publication.
const NOW: i64 = T0 + 100 * DAY;

fn path(name: &str) -> String {
    format!("/tmp/tender-db-560-{name}-{}.db", std::process::id())
}

async fn open(name: &str) -> (store::Db, turso::Connection) {
    let path = path(name);
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(&path).await.unwrap();
    db.set_foreign_keys(false).await.unwrap();
    let raw = turso::Builder::new_local(&path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    (db, conn)
}

fn amount(field: &str, cents: i64, currency: &str) -> Fact {
    Fact::Amount { field: field.into(), cents, currency: currency.into(), tax_basis: None, quality: None }
}

/// A procedure-scope estimated value in EUR, as an R2.0.9 CN's II.1.5 folds.
fn total(eur_cents: i64) -> Fact {
    amount("estimated_value", eur_cents, "EUR")
}

/// A Tender-scope submission deadline: a fact that moves the Tender, never its value or a lot.
fn deadline_at(at: i64) -> Fact {
    Fact::Date { field: "submission_deadline".into(), utc_seconds: at, offset_minutes: 0, has_time: true }
}

fn lot(key: &str, facts: &[Fact]) -> LotState {
    LotState { key: key.into(), kind: "Lot".into(), facts: facts.iter().cloned().collect() }
}

/// One version as the fold carries it: `facts` and `lots` are the head's union.
fn version(notice: i64, published_at: i64, facts: &[Fact], lots: &[LotState]) -> TenderVersion {
    TenderVersion {
        caused_by_notice_id: notice,
        published_at,
        dispatched_at: None,
        notice_subtype: None,
        original_lang: None,
        publication_id: format!("{notice}-2026"),
        facts: facts.iter().cloned().collect(),
        lots: lots.to_vec(),
        rounds: Vec::new(),
        group_members: Vec::new(),
    }
}

fn projection(name: &str, versions: Vec<TenderVersion>) -> TenderProjection {
    TenderProjection {
        source: "ted".into(),
        procedure_key: Some(format!("pk-560-{name}")),
        island_notice_id: None,
        kind: "procedure".into(),
        versions,
    }
}

async fn scalar(conn: &turso::Connection, sql: &str, params: Vec<Value>) -> Option<i64> {
    let mut rows = conn.query(sql, params).await.unwrap_or_else(|e| panic!("{e}: {sql}"));
    let row = rows.next().await.unwrap()?;
    row.get_value(0).unwrap().as_integer().copied()
}

/// The change log's Tender and lot rows for one case after `cursor`, as `tender@2:changed` and
/// `lot:LOT-1@2:changed` (a seq-less row reads `@-`), in cursor order.
async fn feed(conn: &turso::Connection, name: &str, cursor: i64) -> Vec<String> {
    let tender = scalar(conn, "SELECT id FROM tenders WHERE procedure_key = ?", vec![Value::Text(format!("pk-560-{name}"))])
        .await
        .expect("the case's tender");
    let mut rows = conn
        .query(
            "SELECT c.entity_kind, c.version_seq, c.op, l.lot_key FROM changes c \
               LEFT JOIN lots l ON c.entity_kind = 'lot' AND l.id = c.entity_id \
              WHERE c.cursor > ? AND ((c.entity_kind = 'tender' AND c.entity_id = ?) \
                 OR (c.entity_kind = 'lot' AND l.tender_id = ?)) \
              ORDER BY c.cursor",
            vec![Value::Integer(cursor), Value::Integer(tender), Value::Integer(tender)],
        )
        .await
        .unwrap();
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.unwrap() {
        let text = |i: usize| match row.get_value(i).unwrap() {
            Value::Text(s) => s,
            other => panic!("column {i}: {other:?}"),
        };
        let seq = match row.get_value(1).unwrap() {
            Value::Integer(n) => n.to_string(),
            _ => "-".to_owned(),
        };
        let entity = match text(0).as_str() {
            "lot" => format!("lot:{}", text(3)),
            kind => kind.to_owned(),
        };
        out.push(format!("{entity}@{seq}:{}", text(2)));
    }
    out
}

async fn last_cursor(conn: &turso::Connection) -> i64 {
    scalar(conn, "SELECT COALESCE(MAX(cursor), 0) FROM changes", vec![]).await.unwrap_or(0)
}

async fn head_value(conn: &turso::Connection, name: &str) -> Option<i64> {
    scalar(
        conn,
        "SELECT current_value_eur_cents FROM tenders WHERE procedure_key = ?",
        vec![Value::Text(format!("pk-560-{name}"))],
    )
    .await
}

/// The decided cases, each folded from scratch in one run unless it says otherwise.
#[tokio::test]
async fn a_tender_value_move_is_a_change_row_for_the_tender_and_its_lots() {
    let (db, conn) = open("cases").await;
    let one = |cents: i64| [lot("LOT-1", &[total(cents)])];

    // The procedure total only: an R2.0.9 CN of EUR 1,000,000 with LOT-1 at EUR 300,000, then
    // an F14 correcting II.1.5 to EUR 400,000. The lot's facts do not move.
    let lot_1 = one(300_000_00);
    let procedure_only = projection(
        "procedure-only",
        vec![version(1, T0, &[total(1_000_000_00)], &lot_1), version(2, T0 + DAY, &[total(400_000_00)], &lot_1)],
    );
    // A sibling lot's figure moves the election (no procedure figure, so the head is the larger
    // lot's): LOT-2 moves EUR 500,000 to EUR 800,000, LOT-1 keeps its facts.
    let sibling = projection(
        "sibling",
        vec![
            version(11, T0, &[], &[lot("LOT-1", &[total(300_000_00)]), lot("LOT-2", &[total(500_000_00)])]),
            version(12, T0 + DAY, &[], &[lot("LOT-1", &[total(300_000_00)]), lot("LOT-2", &[total(800_000_00)])]),
        ],
    );
    // NULL to a value and back: a CN with no figure, a corrigendum stating one, a corrigendum
    // withdrawing it. The lot never carries a figure.
    let bare = [lot("LOT-1", &[])];
    let null_and_back = projection(
        "null-and-back",
        vec![
            version(21, T0, &[], &bare),
            version(22, T0 + DAY, &[total(400_000_00)], &bare),
            version(23, T0 + 2 * DAY, &[], &bare),
        ],
    );
    // The same figure restated with the same facts: no row for anyone.
    let restated = projection(
        "restated",
        vec![version(31, T0, &[total(1_000_000_00)], &lot_1), version(32, T0 + DAY, &[total(1_000_000_00)], &lot_1)],
    );
    db.apply_tenders(&[procedure_only, sibling, null_and_back, restated], NOW, false).await.unwrap();

    assert_eq!(head_value(&conn, "procedure-only").await, Some(400_000_00), "the head is the corrected total");
    assert_eq!(
        feed(&conn, "procedure-only", 0).await,
        ["tender@1:added", "lot:LOT-1@1:added", "tender@2:changed", "lot:LOT-1@2:changed"],
        "the corrigendum is a change of LOT-1 too"
    );
    assert_eq!(head_value(&conn, "sibling").await, Some(800_000_00));
    assert_eq!(
        feed(&conn, "sibling", 0).await,
        [
            "tender@1:added",
            "lot:LOT-1@1:added",
            "lot:LOT-2@1:added",
            "tender@2:changed",
            "lot:LOT-1@2:changed",
            "lot:LOT-2@2:changed",
        ],
        "LOT-1's facts did not move, its Tender's value did"
    );
    assert_eq!(head_value(&conn, "null-and-back").await, None);
    assert_eq!(
        feed(&conn, "null-and-back", 0).await,
        [
            "tender@1:added",
            "lot:LOT-1@1:added",
            "tender@2:changed",
            "lot:LOT-1@2:changed",
            "tender@3:changed",
            "lot:LOT-1@3:changed",
        ],
        "NULL to a value is a move, and a value to NULL"
    );
    assert_eq!(
        feed(&conn, "restated", 0).await,
        ["tender@1:added", "lot:LOT-1@1:added"],
        "an unmoved value with unmoved facts writes nothing at version 2"
    );
}

/// A GBP Tender whose two versions differ only in their publication date: the rate alone moves
/// its EUR value (the fold converts at each version's own date), so the Tender and its lot are
/// both `changed` at version 2 — a non-EUR Tender crossing a bound on the rate.
#[tokio::test]
async fn a_rate_only_move_of_a_gbp_tender_is_a_change_row_for_the_tender_and_its_lot() {
    let (db, conn) = open("gbp").await;
    let rates = vec![
        ("GBP".to_owned(), "2026-01-01".to_owned(), 0.8, "ecb".to_owned()),
        ("GBP".to_owned(), "2026-01-05".to_owned(), 0.9, "ecb".to_owned()),
    ];
    db.upsert_currency_rates(&rates).await.unwrap();
    assert_eq!(db.reload_rates_lookup().await.unwrap(), 2);
    let facts = [amount("estimated_value", 800_000_00, "GBP")];
    let lots = [lot("LOT-1", &[amount("estimated_value", 300_000_00, "GBP")])];
    let v1 = version(41, T0, &facts, &lots);
    let mut v2 = v1.clone();
    v2.caused_by_notice_id = 42;
    v2.publication_id = "42-2026".into();
    v2.published_at = T0 + 4 * DAY;
    db.apply_tenders(&[projection("gbp", vec![v1, v2])], NOW, false).await.unwrap();
    assert_eq!(head_value(&conn, "gbp").await, Some(888_888_89), "GBP 800,000 at 0.9 on 2026-01-06");
    assert_eq!(
        feed(&conn, "gbp", 0).await,
        ["tender@1:added", "lot:LOT-1@1:added", "tender@2:changed", "lot:LOT-1@2:changed"],
        "facts identical, EUR 1,000,000 then EUR 888,888.89"
    );
}

/// A daily append (keep = 1: fold v1, then v1 + v2) seeds the comparison with the kept
/// prefix's value and the stored head's, so the first appended version is compared with what
/// subscribers hold. On a pure append the two seeds agree, so this pins the outcome, not either
/// seed: without one of them the other still finds the move, and a wrong one invents one. The
/// kept-prefix seed alone, where no stored head is compared at the first written version, is
/// pinned by `a_late_notice_inserted_mid_chain_compares_against_the_kept_prefix_and_the_run`.
#[tokio::test]
async fn an_append_compares_its_first_version_against_the_kept_prefix() {
    let (db, conn) = open("append").await;
    let lots = [lot("LOT-1", &[total(300_000_00)])];
    let v1 = version(51, T0, &[total(1_000_000_00)], &lots);
    let moved = version(52, T0 + DAY, &[total(400_000_00)], &lots);
    db.apply_tenders(&[projection("append-moved", vec![v1.clone()])], NOW, false).await.unwrap();
    let cursor = last_cursor(&conn).await;
    let a = db.apply_tenders(&[projection("append-moved", vec![v1.clone(), moved])], NOW, false).await.unwrap();
    assert_eq!(a.versions_written, 1, "a pure append writes the new version only: {a:?}");
    assert_eq!(feed(&conn, "append-moved", cursor).await, ["tender@2:changed", "lot:LOT-1@2:changed"]);

    // The same append, with the value unmoved and a Tender-scope fact that moved: the tender's
    // row only.
    let v1b = version(61, T0, &[total(1_000_000_00)], &lots);
    let unmoved = version(
        62,
        T0 + DAY,
        &[total(1_000_000_00), Fact::Date { field: "submission_deadline".into(), utc_seconds: NOW, offset_minutes: 0, has_time: true }],
        &lots,
    );
    db.apply_tenders(&[projection("append-unmoved", vec![v1b.clone()])], NOW, false).await.unwrap();
    let cursor = last_cursor(&conn).await;
    db.apply_tenders(&[projection("append-unmoved", vec![v1b, unmoved])], NOW, false).await.unwrap();
    assert_eq!(feed(&conn, "append-unmoved", cursor).await, ["tender@2:changed"], "no lot row: nothing of the lot moved");
}

/// The net-move rule (the adversarial check's amendment): one fold run appends v2 (EUR
/// 1,000,000 to 2,000,000) and v3 (EUR 2,000,000 again). SSE probes v2 superset against
/// superset (v3 is the head), so v2's row alone can leave a `max_value` subscriber holding a
/// lot outside its bound; v3 moved nothing against v2, but the run's net value moved, so v3
/// is a lot row too and probes the head.
#[tokio::test]
async fn a_run_whose_net_value_moved_announces_its_last_version() {
    let (db, conn) = open("net").await;
    let lots = [lot("LOT-1", &[total(300_000_00)])];
    let deadline =
        |at: i64| Fact::Date { field: "submission_deadline".into(), utc_seconds: at, offset_minutes: 0, has_time: true };
    let v1 = version(71, T0, &[total(1_000_000_00)], &lots);
    let v2 = version(72, T0 + DAY, &[total(2_000_000_00)], &lots);
    let v3 = version(73, T0 + 2 * DAY, &[total(2_000_000_00), deadline(NOW)], &lots);
    db.apply_tenders(&[projection("net", vec![v1.clone()])], NOW, false).await.unwrap();
    let cursor = last_cursor(&conn).await;
    db.apply_tenders(&[projection("net", vec![v1.clone(), v2.clone(), v3.clone()])], NOW, false).await.unwrap();
    assert_eq!(
        feed(&conn, "net", cursor).await,
        ["tender@2:changed", "lot:LOT-1@2:changed", "tender@3:changed", "lot:LOT-1@3:changed"],
        "v3's value equals v2's, but not the value before the run"
    );

    // A run whose net value did NOT move (EUR 1,000,000 to 2,000,000 and back) announces each
    // step, and the back step is a move of its own.
    let back = version(74, T0 + 3 * DAY, &[total(1_000_000_00)], &lots);
    let w1 = version(81, T0, &[total(1_000_000_00)], &lots);
    let w2 = version(82, T0 + DAY, &[total(2_000_000_00)], &lots);
    let w3 = version(83, T0 + 2 * DAY, &[total(2_000_000_00), deadline(NOW)], &lots);
    let mut w4 = back;
    w4.caused_by_notice_id = 84;
    w4.publication_id = "84-2026".into();
    w4.facts.insert(deadline(NOW));
    db.apply_tenders(&[projection("net-flat", vec![w1.clone()])], NOW, false).await.unwrap();
    let cursor = last_cursor(&conn).await;
    db.apply_tenders(&[projection("net-flat", vec![w1, w2, w3, w4])], NOW, false).await.unwrap();
    assert_eq!(
        feed(&conn, "net-flat", cursor).await,
        ["tender@2:changed", "lot:LOT-1@2:changed", "tender@3:changed", "tender@4:changed", "lot:LOT-1@4:changed"],
        "v3 moved nothing and is not the run's last; v4 moved back"
    );
}

/// Issue 495's `On` mode: a stale Tender's compared prefix replays no history (its corrections
/// are ADR-0017 D3's seq-less rows), but the first appended version is compared against the
/// compared prefix's LAST value. The prefix carries two different values, EUR 1,000,000 then
/// EUR 400,000, so a comparison against nothing, or against the prefix's first value, fails
/// one of the two appends.
#[tokio::test]
async fn an_on_mode_refold_compares_the_first_appended_version_against_the_prefix_value() {
    let (db, conn) = open("on").await;
    db.set_refold_compare(store::RefoldCompare::On);
    // The compare refuses to run without the by-version indexes.
    db.build_tender_indexes().await.unwrap();
    let lots = [lot("LOT-1", &[total(300_000_00)])];
    let prefix = |base: i64| {
        vec![version(base + 1, T0, &[total(1_000_000_00)], &lots), version(base + 2, T0 + DAY, &[total(400_000_00)], &lots)]
    };
    db.apply_tenders(&[projection("on-moved", prefix(90)), projection("on-unmoved", prefix(100))], NOW, false)
        .await
        .unwrap();
    db.set_projection_epoch_for_test(-1).await.unwrap();
    let cursor = last_cursor(&conn).await;

    let mut moved = prefix(90);
    moved.push(version(93, T0 + 2 * DAY, &[total(1_000_000_00)], &lots));
    let mut unmoved = prefix(100);
    unmoved.push(version(103, T0 + 2 * DAY, &[total(400_000_00)], &lots));
    let a = db
        .apply_tenders(&[projection("on-moved", moved), projection("on-unmoved", unmoved)], NOW, false)
        .await
        .unwrap();
    assert_eq!(a.tenders_corrected, 0, "the prefixes compare identical: {a:?}");
    assert_eq!(
        feed(&conn, "on-moved", cursor).await,
        ["tender@3:changed", "lot:LOT-1@3:changed"],
        "EUR 400,000 (the prefix's last) to 1,000,000; no row for the prefix"
    );
    assert_eq!(
        feed(&conn, "on-unmoved", cursor).await,
        Vec::<String>::new(),
        "EUR 400,000 restated with the prefix's facts: nothing at all"
    );
}

/// A late notice inserted mid-chain, through the fold: the stored chain is [v1 EUR 1,000,000,
/// v3 EUR 2,000,000 + a deadline]; v2 (EUR 2,000,000), published between them, arrives late and
/// the fold rewrites [v1, v2, v3] with keep = 1 of the 2 stored versions, so no stored head is
/// compared at the first written version. v2 is a move only against the kept prefix's value (the
/// seed), and v3, equal to v2 and to the stored head, only against the value before the run (the
/// net-move rule): each lot row is pinned by one seed alone.
#[tokio::test]
async fn a_late_notice_inserted_mid_chain_compares_against_the_kept_prefix_and_the_run() {
    let (db, conn) = open("mid").await;
    let lots = [lot("LOT-1", &[total(300_000_00)])];
    let v1 = version(111, T0, &[total(1_000_000_00)], &lots);
    let v2 = version(112, T0 + DAY, &[total(2_000_000_00)], &lots);
    let v3 = version(113, T0 + 2 * DAY, &[total(2_000_000_00), deadline_at(NOW)], &lots);
    db.apply_tenders(&[projection("mid", vec![v1.clone(), v3.clone()])], NOW, false).await.unwrap();
    let cursor = last_cursor(&conn).await;
    let a = db.apply_tenders(&[projection("mid", vec![v1, v2, v3])], NOW, false).await.unwrap();
    assert_eq!(
        (a.versions_written, a.versions_removed),
        (2, 1),
        "keep = 1: the stored v3 goes, v2 and v3 are written: {a:?}"
    );
    assert_eq!(head_value(&conn, "mid").await, Some(2_000_000_00));
    assert_eq!(
        feed(&conn, "mid", cursor).await,
        ["tender@2:changed", "lot:LOT-1@2:changed", "tender@3:changed", "lot:LOT-1@3:changed"],
        "v2 moved from the kept prefix's EUR 1,000,000; v3 equals v2 but not the value before the run"
    );
}

/// A rewrite that keeps part of a stored chain and replaces its tail starts from a head
/// subscribers already hold: [n1 EUR 1,000,000, n2 EUR 400,000] served EUR 400,000. n2 leaves and
/// n3 (EUR 1,000,000) takes its place, keep = 1 of 2: v2 equals the kept prefix's value and the
/// run's seed, so only the STORED head shows the move, and a `max_value=50000000` lots subscriber
/// holding LOT-1 needs its row. In `tail` n3 also states a deadline; in `tail-bare` nothing else
/// of the Tender moved, so the value move alone writes the Tender's row too.
#[tokio::test]
async fn a_replaced_tail_compares_its_head_against_the_stored_head() {
    let (db, conn) = open("tail").await;
    let lots = [lot("LOT-1", &[total(300_000_00)])];
    let cases = [("tail", 120, vec![total(1_000_000_00), deadline_at(NOW)]), ("tail-bare", 130, vec![total(1_000_000_00)])];
    for (name, base, restated) in cases {
        let n1 = version(base + 1, T0, &[total(1_000_000_00)], &lots);
        let n2 = version(base + 2, T0 + DAY, &[total(400_000_00)], &lots);
        db.apply_tenders(&[projection(name, vec![n1.clone(), n2])], NOW, false).await.unwrap();
        assert_eq!(head_value(&conn, name).await, Some(400_000_00), "{name}: the stored head");
        let cursor = last_cursor(&conn).await;
        let n3 = version(base + 3, T0 + 2 * DAY, &restated, &lots);
        let a = db.apply_tenders(&[projection(name, vec![n1, n3])], NOW, false).await.unwrap();
        assert_eq!((a.versions_written, a.versions_removed), (1, 1), "{name}: keep = 1 of 2 stored: {a:?}");
        assert_eq!(head_value(&conn, name).await, Some(1_000_000_00), "{name}: the new head");
        assert_eq!(
            feed(&conn, name, cursor).await,
            ["tender@2:changed", "lot:LOT-1@2:changed"],
            "{name}: EUR 400,000 served, EUR 1,000,000 now"
        );
    }
}

/// An `On` refold that appends past a compared prefix announces the appended versions by their
/// transition rows, so ADR-0017 D3's head compare stands aside, and those rows diff fresh against
/// fresh. When the stored head value (what subscribers hold) differs from the re-derived prefix's
/// (an election or a rate that moved without a refold of the Tender, as issue 551's drain window
/// left), the appended head is compared against the stored value too. Here the stored head reads
/// EUR 400,000 by hand while the chain re-derives to EUR 1,000,000, and the appended version keeps
/// EUR 1,000,000: its lot row is the only event a `max_value=50000000` lots subscriber gets. The
/// control, its stored head untouched, writes the Tender's row alone.
#[tokio::test]
async fn an_on_mode_refold_with_an_append_compares_its_head_against_the_stored_head() {
    let (db, conn) = open("on-stored").await;
    db.set_refold_compare(store::RefoldCompare::On);
    // The compare refuses to run without the by-version indexes.
    db.build_tender_indexes().await.unwrap();
    let lots = [lot("LOT-1", &[total(300_000_00)])];
    let v1 = |base: i64| version(base + 1, T0, &[total(1_000_000_00)], &lots);
    let v2 = |base: i64| version(base + 2, T0 + DAY, &[total(1_000_000_00), deadline_at(NOW)], &lots);
    db.apply_tenders(&[projection("on-stored", vec![v1(140)]), projection("on-stored-ctl", vec![v1(150)])], NOW, false)
        .await
        .unwrap();
    db.execute_for_test("UPDATE tenders SET current_value_eur_cents = 40000000 WHERE procedure_key = 'pk-560-on-stored'")
        .await
        .unwrap();
    db.set_projection_epoch_for_test(-1).await.unwrap();
    let cursor = last_cursor(&conn).await;
    let a = db
        .apply_tenders(
            &[projection("on-stored", vec![v1(140), v2(140)]), projection("on-stored-ctl", vec![v1(150), v2(150)])],
            NOW,
            false,
        )
        .await
        .unwrap();
    assert_eq!(a.tenders_corrected, 0, "the prefixes compare identical and an appended head is no D3 correction: {a:?}");
    assert_eq!(head_value(&conn, "on-stored").await, Some(1_000_000_00));
    assert_eq!(
        feed(&conn, "on-stored", cursor).await,
        ["tender@2:changed", "lot:LOT-1@2:changed"],
        "EUR 400,000 served, EUR 1,000,000 at the appended head"
    );
    assert_eq!(feed(&conn, "on-stored-ctl", cursor).await, ["tender@2:changed"], "the deadline alone: no lot row");
}
