//! Issue 239: turso 0.7.0 pushed no predicate into ANY view — a PK point read
//! through `v_tenders` materialised the whole corpus (>10 s on prod, cost
//! independent of the filter's selectivity). The views were documented NOT
//! FILTERABLE and analysts pointed at the base tables instead.
//!
//! This probe pins what the CURRENT turso does with a filtered view query, as a
//! PLAN, not a stopwatch (the issue-80 lesson: laptop clocks cannot tell a seek
//! from a scan at fixture scale). The day an upgrade makes these seek, the
//! inverted assertions fail and say so — that is the cue to re-measure on prod
//! and lift the NOT FILTERABLE warnings from `/v1/docs` and the view comments.

use store::turso;

async fn open(name: &str) -> (store::Db, turso::Connection) {
    let path = format!("/tmp/tender-db-viewpush-{name}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(&path).await.unwrap();
    let raw = turso::Builder::new_local(&path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    (db, conn)
}

async fn plan_of(conn: &turso::Connection, sql: &str) -> String {
    let mut rows = conn.query(&format!("EXPLAIN QUERY PLAN {sql}"), ()).await.unwrap();
    let mut plan = String::new();
    while let Some(row) = rows.next().await.unwrap() {
        if let Ok(turso::Value::Text(detail)) = row.get_value(3) {
            plan.push_str(&detail);
            plan.push('\n');
        }
    }
    assert!(!plan.is_empty(), "no plan came back for: {sql}");
    plan
}

/// The tripwire. Measured 2026-08-23 under turso 0.7.2: every filtered view
/// query still plans as `SCAN <view>` — the view is materialised in full and
/// the caller's filter applied afterwards. `v_tenders WHERE id = <pk>` drives a
/// full scan of `tender_versions` (14.15M rows on prod) with a per-row seek
/// into `tenders`, which is why the point read measured >10 s there.
///
/// If this test FAILS after a turso bump, that is good news, not a regression:
/// re-run the issue-239 prod measurements, and if they hold, lift the NOT
/// FILTERABLE warnings from the `v_*` view comments (canonical.rs) and the
/// `/v1/sql` + `/v1/docs` guidance (sql.rs) that points analysts at base-table
/// joins instead.
#[tokio::test]
async fn turso_still_pushes_no_predicate_into_views() {
    let (_db, conn) = open("plans").await;
    for sql in [
        "SELECT seq FROM v_tender_current WHERE tender_id = 42",
        "SELECT id, title FROM v_tenders WHERE id = 42",
        "SELECT COUNT(*) FROM v_tenders WHERE id < 5000",
        "SELECT id FROM v_lots WHERE tender_id = 42",
        "SELECT id, mentions FROM v_organizations WHERE id = 42",
    ] {
        let plan = plan_of(&conn, sql).await;
        println!("=== {sql}\n{plan}");
        assert!(
            plan.lines().next().unwrap_or_default().starts_with("SCAN v_"),
            "a filtered view query no longer materialises the view — turso learned \
             predicate pushdown. Re-measure issue 239 on prod and lift the NOT \
             FILTERABLE docs. Plan for {sql}:\n{plan}"
        );
    }
}

/// The controls that let the assertion above mean something: on the base table
/// the same point read must SEEK (this is the shape `/v1/docs` sends analysts
/// to — if a turso bump broke it, that IS a regression), and a
/// seek-defeated filter must SCAN, proving the plan text can say both.
#[tokio::test]
async fn base_table_point_read_still_seeks() {
    let (_db, conn) = open("controls").await;
    let seek = plan_of(&conn, "SELECT current_seq FROM tenders WHERE id = 42").await;
    assert!(
        seek.contains("SEARCH tenders USING INTEGER PRIMARY KEY"),
        "the documented base-table point read stopped seeking: {seek}"
    );
    let defeated = plan_of(&conn, "SELECT current_seq FROM tenders WHERE id + 0 = 42").await;
    assert!(defeated.contains("SCAN"), "the control cannot distinguish a seek from a scan: {defeated}");
}
