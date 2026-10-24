//! Issue 561: every version stores the Tender's elected value as of its own prefix
//! (`tender_versions.value_eur_cents`), so the SSE At probe reads an earlier version at its own
//! value instead of the 547/551 superset. The column is QUIET (ADR-0017 D1.4/D3 as amended): a
//! version that differs only there is refreshed with one UPDATE, written as no correction and
//! counted `values_refreshed`, so the epoch bump 4 -> 5 fills a Tender on any fold that touches
//! it without an all-profile refold and without a change row.
//!
//! What this file pins, every case through the FOLD (`apply_tenders`), in the style of
//! `value_move_change_rows.rs`:
//! - the quiet refresh in each compare mode (`On` refreshes in place, `Shadow` plans it and
//!   rewrites, `Off` rewrites and counts nothing);
//! - the lazy fill: an append to an epoch-4 Tender fills its kept versions, an epoch-5 Tender's
//!   kept versions are trusted;
//! - the shrink restatement (D561-5): a chain cut back to its kept prefix restates the kept
//!   head's value at today's rates, or the head gate would refuse the batch;
//! - the epoch gate's runtime cost (D561-14): a gated probe of an earlier version does not walk
//!   the version's amount rows, the superset fallback does.

use store::canonical::{Applied, Fact, LotState, RefoldCompare, TenderProjection, TenderVersion};
use store::read::{self, Filter, Scope};
use store::turso::{self, Value};

const DAY: i64 = 86_400;
/// 2026-01-02T12:00:00Z: every chain's first version.
const T0: i64 = 1_767_355_200;
/// The fold's `now`, after every publication.
const NOW: i64 = T0 + 100 * DAY;

fn path(name: &str) -> String {
    format!("/tmp/tender-db-561-{name}-{}.db", std::process::id())
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

fn cleanup(name: &str) {
    let path = path(name);
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

fn amount(field: &str, cents: i64, currency: &str) -> Fact {
    Fact::Amount { field: field.into(), cents, currency: currency.into(), tax_basis: None, quality: None }
}

/// A procedure-scope estimated value in EUR, as an R2.0.9 CN's II.1.5 folds.
fn total(eur_cents: i64) -> Fact {
    amount("estimated_value", eur_cents, "EUR")
}

fn lot(key: &str, facts: &[Fact]) -> LotState {
    LotState { key: key.into(), kind: "Lot".into(), facts: facts.iter().cloned().collect(), published_lot_no: None }
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
        stated_totals: Vec::new(),
    }
}

fn projection(name: &str, versions: Vec<TenderVersion>) -> TenderProjection {
    TenderProjection {
        source: "ted".into(),
        procedure_key: Some(format!("pk-561-{name}")),
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

async fn tender_id(conn: &turso::Connection, name: &str) -> i64 {
    scalar(conn, "SELECT id FROM tenders WHERE procedure_key = ?", vec![Value::Text(format!("pk-561-{name}"))])
        .await
        .expect("the case's tender")
}

/// The change log's Tender and lot rows for one case after `cursor`, as `tender@2:changed` and
/// `lot:LOT-1@2:changed` (a seq-less row reads `@-`), in cursor order.
async fn feed(conn: &turso::Connection, name: &str, cursor: i64) -> Vec<String> {
    let tender = tender_id(conn, name).await;
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

/// The Tender's stored per-version values, by seq.
async fn values(conn: &turso::Connection, name: &str) -> Vec<Option<i64>> {
    let tender = tender_id(conn, name).await;
    let mut rows = conn
        .query("SELECT value_eur_cents FROM tender_versions WHERE tender_id = ? ORDER BY seq", vec![Value::Integer(tender)])
        .await
        .unwrap();
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.unwrap() {
        out.push(row.get_value(0).unwrap().as_integer().copied());
    }
    out
}

async fn epochs(conn: &turso::Connection) -> Vec<i64> {
    let mut rows = conn.query("SELECT projection_epoch FROM tenders ORDER BY id", ()).await.unwrap();
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.unwrap() {
        out.push(*row.get_value(0).unwrap().as_integer().unwrap());
    }
    out
}

async fn version_rowids(conn: &turso::Connection) -> Vec<(i64, i64, i64)> {
    let mut rows = conn.query("SELECT rowid, tender_id, seq FROM tender_versions ORDER BY rowid", ()).await.unwrap();
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.unwrap() {
        let int = |i: usize| *row.get_value(i).unwrap().as_integer().unwrap();
        out.push((int(0), int(1), int(2)));
    }
    out
}

/// A version that differs only in its stored value is refreshed with one UPDATE under `On`:
/// no version rewritten (its rowid stays), no table rewritten, no correction row, the Tender
/// verified and stamped 5. `values_refreshed` counts the versions whose fresh elected value is
/// non-NULL (4 of 6 here): a stored NULL equals a fresh NULL, so a version with nothing to
/// elect is not refreshed (the check's count correction). `Shadow` plans the same 4 and
/// rewrites like `Off`; `Off` rewrites and counts nothing. Each mode leaves the values back.
#[tokio::test]
async fn a_value_only_difference_is_refreshed_quietly() {
    let (db, conn) = open("quiet").await;
    db.set_refold_compare(RefoldCompare::On);
    let lot_1 = [lot("LOT-1", &[total(300_000_00)])];
    let projections = vec![
        projection(
            "a",
            vec![
                version(1, T0, &[total(1_000_000_00)], &lot_1),
                version(2, T0 + DAY, &[total(400_000_00)], &lot_1),
                version(3, T0 + 2 * DAY, &[total(1_000_000_00)], &lot_1),
            ],
        ),
        projection("b", vec![version(11, T0, &[], &[]), version(12, T0 + DAY, &[total(500_000_00)], &[])]),
        projection("c", vec![version(21, T0, &[total(0)], &[])]),
    ];
    db.apply_tenders(&projections, NOW, false).await.unwrap();
    db.build_tender_indexes().await.unwrap();
    let want = [
        ("a", vec![Some(1_000_000_00), Some(400_000_00), Some(1_000_000_00)]),
        ("b", vec![None, Some(500_000_00)]),
        ("c", vec![None]),
    ];
    for (name, values_want) in &want {
        assert_eq!(&values(&conn, name).await, values_want, "{name}: the first fold stores each version's value");
    }
    let rowids = version_rowids(&conn).await;
    assert_eq!(rowids.len(), 6);

    for mode in [RefoldCompare::On, RefoldCompare::Shadow, RefoldCompare::Off] {
        db.set_refold_compare(mode);
        conn.execute("UPDATE tender_versions SET value_eur_cents = NULL", ()).await.unwrap();
        db.set_projection_epoch_for_test(4).await.unwrap();
        let cursor = last_cursor(&conn).await;
        let applied: Applied = db.apply_tenders(&projections, NOW, false).await.unwrap();
        for (name, values_want) in &want {
            assert_eq!(&values(&conn, name).await, values_want, "{mode:?} {name}: the values are back");
        }
        assert_eq!(epochs(&conn).await, vec![5, 5, 5], "{mode:?}: every Tender stamped 5");
        match mode {
            RefoldCompare::On => {
                assert_eq!(applied.values_refreshed, 4, "On: the four non-NULL values refreshed: {applied:?}");
                assert_eq!((applied.tenders_verified, applied.tenders_corrected), (3, 0), "On: verified, not corrected");
                assert_eq!(applied.versions_written, 0, "On: no version rewritten");
                assert_eq!(applied.tables_rewritten, 0, "On: no table rewritten");
                assert_eq!(applied.correction_rows_planned, 0, "On: no correction row");
                assert_eq!(last_cursor(&conn).await, cursor, "On: no change row");
                assert_eq!(version_rowids(&conn).await, rowids, "On: every version keeps its row");
                let line = applied.compare_line().expect("a compare line");
                assert!(line.contains("3 tenders verified, 0 corrected, 4 version values refreshed quietly (issue 561)"), "{line}");
            }
            RefoldCompare::Shadow => {
                assert_eq!(applied.values_refreshed, 4, "Shadow: the same four planned: {applied:?}");
                assert_eq!((applied.tenders_verified, applied.tenders_corrected), (3, 0), "Shadow: verified");
            }
            RefoldCompare::Off => {
                assert_eq!(applied.values_refreshed, 0, "Off: nothing compared, nothing counted: {applied:?}");
            }
        }
    }
    drop((db, conn));
    cleanup("quiet");
}

/// The combined `On` path (deploy #4 review d4-correctness-3): one compared version whose quiet
/// value differs AND whose child table differs. `write_what_differs` runs the refresh UPDATE and
/// then the per-table DELETE/append in the same call: the value is back, the version keeps its
/// row (no whole rewrite), the child table is rewritten, the Tender is corrected, and the
/// refresh still counts under `values_refreshed`.
#[tokio::test]
async fn a_value_refresh_and_a_child_table_difference_in_one_version_are_both_written() {
    let (db, conn) = open("quiet-child").await;
    db.set_refold_compare(RefoldCompare::On);
    let lot_1 = [lot("LOT-1", &[total(300_000_00)])];
    let projections = vec![
        projection(
            "a",
            vec![
                version(1, T0, &[total(1_000_000_00)], &lot_1),
                version(2, T0 + DAY, &[total(400_000_00)], &lot_1),
                version(3, T0 + 2 * DAY, &[total(1_000_000_00)], &lot_1),
            ],
        ),
        projection("b", vec![version(11, T0, &[], &[]), version(12, T0 + DAY, &[total(500_000_00)], &[])]),
    ];
    db.apply_tenders(&projections, NOW, false).await.unwrap();
    db.build_tender_indexes().await.unwrap();
    let rowids = version_rowids(&conn).await;
    let a = tender_id(&conn, "a").await;
    let procedure_amount = format!(
        "SELECT COUNT(*) FROM tender_version_amounts WHERE tender_id = {a} AND seq = 2 AND lot_id IS NULL \
           AND field = 'estimated_value' AND cents = 40000000"
    );
    assert_eq!(scalar(&conn, &procedure_amount, vec![]).await, Some(1), "a's v2 carries its procedure total");

    conn.execute("UPDATE tender_versions SET value_eur_cents = NULL", ()).await.unwrap();
    conn.execute(&format!("DELETE FROM tender_version_amounts WHERE tender_id = {a} AND seq = 2 AND lot_id IS NULL"), ())
        .await
        .unwrap();
    db.set_projection_epoch_for_test(4).await.unwrap();
    let applied: Applied = db.apply_tenders(&projections, NOW, false).await.unwrap();

    assert_eq!(values(&conn, "a").await, vec![Some(1_000_000_00), Some(400_000_00), Some(1_000_000_00)], "a: values back");
    assert_eq!(values(&conn, "b").await, vec![None, Some(500_000_00)], "b: values back");
    assert_eq!(scalar(&conn, &procedure_amount, vec![]).await, Some(1), "a's v2 procedure total is written back");
    assert_eq!(version_rowids(&conn).await, rowids, "every version keeps its row: no whole rewrite");
    assert_eq!(epochs(&conn).await, vec![5, 5], "both stamped 5");
    assert_eq!((applied.tenders_verified, applied.tenders_corrected), (1, 1), "a corrected, b verified: {applied:?}");
    assert_eq!(applied.values_refreshed, 4, "a's three values and b's one, a's v2 included: {applied:?}");
    assert_eq!(applied.versions_written, 1, "a's v2 is the one version written (a table of it): {applied:?}");
    assert_eq!(applied.versions_removed, 0, "and none rewritten whole: {applied:?}");
    let amounts = store::canonical::Leaf::Amounts as usize;
    assert_eq!(applied.tables_rewritten_by_leaf[amounts], 1, "a's v2 amounts rewritten: {applied:?}");
    drop((db, conn));
    cleanup("quiet-child");
}

/// The lazy fill. An append to an epoch-4 Tender (stale: its stored chain is compared, then
/// the new version written) fills its kept versions quietly; an epoch-5 Tender's kept versions
/// are trusted, not re-read (a pure append). Neither gets a seq-less correction row: each
/// hears only its appended version's own transition rows.
#[tokio::test]
async fn an_append_fills_an_epoch_4_tenders_kept_versions_and_leaves_an_epoch_5_tenders_alone() {
    let (db, conn) = open("append").await;
    db.set_refold_compare(RefoldCompare::On);
    let lot_1 = [lot("LOT-1", &[total(300_000_00)])];
    let two = |n: i64| vec![version(n, T0, &[total(1_000_000_00)], &lot_1), version(n + 1, T0 + DAY, &[total(400_000_00)], &lot_1)];
    db.apply_tenders(&[projection("x", two(1)), projection("y", two(11))], NOW, false).await.unwrap();
    db.build_tender_indexes().await.unwrap();
    conn.execute("UPDATE tender_versions SET value_eur_cents = NULL", ()).await.unwrap();
    conn.execute("UPDATE tenders SET projection_epoch = 4 WHERE procedure_key = 'pk-561-x'", ()).await.unwrap();
    let cursor = last_cursor(&conn).await;

    let three = |n: i64| {
        let mut v = two(n);
        v.push(version(n + 2, T0 + 2 * DAY, &[total(1_000_000_00)], &lot_1));
        v
    };
    let applied = db.apply_tenders(&[projection("x", three(1)), projection("y", three(11))], NOW, false).await.unwrap();
    assert_eq!(values(&conn, "x").await, vec![Some(1_000_000_00), Some(400_000_00), Some(1_000_000_00)], "x: filled");
    assert_eq!(values(&conn, "y").await, vec![None, None, Some(1_000_000_00)], "y: an epoch-5 Tender's kept versions are trusted");
    assert_eq!(epochs(&conn).await, vec![5, 5]);
    assert_eq!(applied.values_refreshed, 2, "x's two kept versions: {applied:?}");
    assert_eq!((applied.tenders_verified, applied.tenders_corrected), (1, 0), "x verified");
    assert_eq!(applied.versions_written, 2, "each Tender's appended version");
    for name in ["x", "y"] {
        assert_eq!(feed(&conn, name, cursor).await, ["tender@3:changed", "lot:LOT-1@3:changed"], "{name}: no seq-less row");
    }
    drop((db, conn));
    cleanup("append");
}

/// D561-5: a chain cut back to its kept prefix (`keep == len < stored`) writes no version, so
/// the head is a kept one whose stored value was elected at its own fold's rates. The fold
/// restates it at today's (the head's) value; without that, a non-EUR Tender whose rate moved
/// since would fail the head gate's issue-561 check and wedge the project job.
#[tokio::test]
async fn a_shrunk_chain_restates_its_kept_heads_value_at_todays_rates() {
    let (db, conn) = open("shrink").await;
    db.upsert_currency_rates(&[("GBP".into(), "2026-01-01".into(), 0.8, "ecb".into())]).await.unwrap();
    db.reload_rates_lookup().await.unwrap();
    let gbp = [amount("estimated_value", 800_000_00, "GBP")];
    let g1 = version(1, T0, &gbp, &[]);
    let g2 = version(2, T0 + 4 * DAY, &gbp, &[]);
    db.apply_tenders(&[projection("g", vec![g1.clone(), g2])], NOW, false).await.unwrap();
    let head = |conn: &turso::Connection| {
        let conn = conn.clone();
        async move { scalar(&conn, "SELECT current_value_eur_cents FROM tenders WHERE procedure_key = 'pk-561-g'", vec![]).await }
    };
    assert_eq!(head(&conn).await, Some(1_000_000_00), "GBP 800,000 at 0.8");
    assert_eq!(values(&conn, "g").await, vec![Some(1_000_000_00), Some(1_000_000_00)]);

    db.upsert_currency_rates(&[("GBP".into(), "2026-01-02".into(), 0.5, "ecb".into())]).await.unwrap();
    db.reload_rates_lookup().await.unwrap();
    db.apply_tenders(&[projection("g", vec![g1])], NOW, false).await.expect("the head gate passes");
    assert_eq!(head(&conn).await, Some(1_600_000_00), "GBP 800,000 at 0.5");
    assert_eq!(values(&conn, "g").await, vec![Some(1_600_000_00)], "the kept head's value restated");
    drop((db, conn));
    cleanup("shrink");
}

/// D561-14, the check's BLOCKING runtime pin: turso plans the superset's ELSE leg as a join
/// loop, so the guard `projection_epoch < VERSION_VALUE_EPOCH` in each fallback subquery is
/// what keeps a gated probe of an earlier version from walking the version's whole amount
/// slice. The fixture: one Tender, two versions of 2,400 lots each EUR 1 m (procedure EUR 10 m,
/// then EUR 20 m). `max_value = 100` (EUR 1) puts no amount inside, so the EXISTS legs walk
/// every row; `min_value` EUR 5 bn lies above the lot sum, so the SUM leg reads every row. A
/// gated probe at seq 1 must be within 3x of the head probe (seq 2); at epoch 4 the fallback
/// must cost at least 5x the gated probe (the control: the fixture shows a walk). The check
/// measured 0.004 s against 0.10 s (`max_value`) and 0.06-0.09 s (`min_value`) on 2026-10-17;
/// the bounds leave a wide margin. Every probe returns no row.
///
/// The Tenders At statement's SELECT list runs its per-version subqueries (the title and amount
/// picks, the lot count) on this fixture even for a probe that returns nothing, about 0.1 s,
/// as much as the walk itself, so a walk there shows as about 2x, not 5x. So the Tenders shape is
/// timed twice: the whole statement (gated within 3x of the head) and its predicate alone, the
/// same FROM and WHERE under `SELECT t.id` (gated within 3x, and the 5x control), which is the
/// shape the check measured (`at561_ib.sql`). The Lots statement's SELECT list is light.
#[tokio::test]
async fn a_gated_version_probe_does_not_walk_the_versions_amounts() {
    let (db, conn) = open("gate-cost").await;
    let lots: Vec<LotState> = (0..2_400).map(|i| lot(&format!("LOT-{i:04}"), &[total(1_000_000_00)])).collect();
    db.apply_tenders(
        &[projection(
            "cost",
            vec![version(1, T0, &[total(10_000_000_00)], &lots), version(2, T0 + DAY, &[total(20_000_000_00)], &lots)],
        )],
        NOW,
        false,
    )
    .await
    .unwrap();
    db.build_tender_indexes().await.unwrap();
    let id = tender_id(&conn, "cost").await;
    let first_lot = scalar(&conn, "SELECT id FROM lots WHERE tender_id = ? AND lot_key = 'LOT-0000'", vec![Value::Integer(id)])
        .await
        .unwrap();
    assert_eq!(values(&conn, "cost").await, vec![Some(10_000_000_00), Some(20_000_000_00)]);

    // Best of three samples, each `repeat` runs of the statement; and the rows one run returned.
    let time = |(sql, params): (String, Vec<Value>), repeat: usize| {
        let conn = conn.clone();
        async move {
            let mut best = std::time::Duration::MAX;
            let mut n = 0;
            for _ in 0..3 {
                let started = std::time::Instant::now();
                for _ in 0..repeat {
                    let mut rows = conn.query(&sql, params.clone()).await.unwrap();
                    n = 0;
                    while rows.next().await.unwrap().is_some() {
                        n += 1;
                    }
                }
                best = best.min(started.elapsed());
            }
            (best, n)
        }
    };
    let filters = [
        ("max_value EUR 1", Filter { max_value: Some(100), now: NOW, ..Filter::default() }),
        ("min_value EUR 5 bn", Filter { min_value: Some(500_000_000_000), now: NOW, ..Filter::default() }),
    ];
    // (shape, repeat, whether the 5x control applies)
    let shapes = [("tenders", 5, false), ("tenders WHERE", 20, true), ("lots", 20, true)];
    let statement = |shape: &str, f: &Filter, seq: i64| match shape {
        "tenders" => read::tenders_at_statement(f, id, seq),
        "tenders WHERE" => {
            let (sql, params) = read::tenders_at_statement(f, id, seq);
            assert_eq!(sql.matches("FROM tenders t").count(), 1, "one tenders FROM:\n{sql}");
            let from = sql.find("FROM tenders t").unwrap();
            assert!(!sql[..from].contains('?'), "the SELECT list binds nothing:\n{sql}");
            (format!("SELECT t.id {}", &sql[from..]), params)
        }
        _ => read::lots_statement(f, Scope::At { id: first_lot, seq }),
    };
    let mut gated = Vec::new();
    for (label, f) in &filters {
        for (shape, repeat, _) in shapes {
            let (head, rows_head) = time(statement(shape, f, 2), repeat).await;
            let (at, rows_at) = time(statement(shape, f, 1), repeat).await;
            assert_eq!((rows_head, rows_at), (0, 0), "{label} {shape}: no probe matches");
            let ratio = at.as_secs_f64() / head.as_secs_f64();
            eprintln!("{label} {shape}: head {head:?}, gated {at:?} ({ratio:.2}x)");
            assert!(ratio <= 3.0, "{label} {shape}: a gated probe walked: {at:?} against the head's {head:?}");
            gated.push(at);
        }
    }
    db.set_projection_epoch_for_test(4).await.unwrap();
    let mut gated = gated.into_iter();
    for (label, f) in &filters {
        for (shape, repeat, control) in shapes {
            let at = gated.next().unwrap();
            let (fallback, rows) = time(statement(shape, f, 1), repeat).await;
            assert_eq!(rows, 0, "{label} {shape}: the fallback matches nothing either");
            let ratio = fallback.as_secs_f64() / at.as_secs_f64();
            eprintln!("{label} {shape}: fallback {fallback:?} ({ratio:.1}x the gated probe)");
            if control {
                assert!(ratio >= 5.0, "{label} {shape}: the control shows no walk ({fallback:?} against {at:?}): the fixture is too light");
            }
        }
    }
    drop((db, conn));
    cleanup("gate-cost");
}
