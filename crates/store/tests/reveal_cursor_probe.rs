//! Issue 274 probe (D5 aftermath): can the reveal recheck walk FieldsPrivacy
//! sections in bounded, cursor-resumable slices off the EXISTING `notice_sections_kind`
//! index — or does it need a `(kind, notice_id)` composite?
//!
//! Context: the capped rewrite (76e33cc) bounded only the reveal-EXISTS pass. Its
//! population aggregates (`dated`/`due` and the `by_field` group-by) still join the
//! ENTIRE FieldsPrivacy cohort against `notice_dates`/`notice_codes` per run, which on
//! prod ran 18+ min at one saturated core with a 17.5 GB cgroup peak and no
//! cancellation point (2026-08-24, service restarted twice to get rid of it). The fix
//! walks a bounded slice per run behind a persisted wrapping cursor, D4-style — IF a
//! slice can be fetched by index seek rather than scan+sort.
//!
//! Case A: rowid cursor over the existing `(kind)` index. SQLite serves
//! `kind = ? AND rowid > ? ORDER BY rowid` straight off the index (index entries are
//! (kind, rowid)); turso's planner may not.
//! Case B: notice_id cursor over an added `(kind, notice_id)` composite.
//! Case C: the slice-CTE aggregate join, to confirm the planner drives from the
//! bounded slice and not from a LIKE-scan of `notice_dates`.

use std::time::Instant;
use store::turso::{self, Value};

const SECTIONS: i64 = 300_000; // every 10th is FieldsPrivacy => 30k cohort
const SLICE: i64 = 2_000;

async fn drain(conn: &turso::Connection, sql: &str) {
    let mut rows = conn.query(sql, ()).await.unwrap();
    while rows.next().await.unwrap().is_some() {}
}

async fn time(conn: &turso::Connection, sql: &str, p: Vec<Value>) -> (f64, usize) {
    let mut best = f64::MAX;
    let mut n = 0;
    for run in 0..3 {
        let t = Instant::now();
        let mut rows = conn.query(sql, p.clone()).await.unwrap();
        n = 0;
        while rows.next().await.unwrap().is_some() {
            n += 1;
        }
        if run > 0 {
            best = best.min(t.elapsed().as_secs_f64());
        }
    }
    (best, n)
}

async fn plan(conn: &turso::Connection, label: &str, sql: &str, p: Vec<Value>) {
    let mut rows = conn.query(&format!("EXPLAIN QUERY PLAN {sql}"), p).await.unwrap();
    println!("\n{label}:");
    while let Some(r) = rows.next().await.unwrap() {
        println!("  {}", r.get_value(3).unwrap().as_text().cloned().unwrap_or_default());
    }
}

#[tokio::test]
#[ignore = "probe: D5 cursor shape; run with --ignored --nocapture"]
async fn a_fields_privacy_slice_is_a_seek_not_a_scan() {
    let path = format!("/tmp/tender-db-revealcur-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    store::Db::open(&path).await.unwrap();
    let db = turso::Builder::new_local(&path).build().await.unwrap();
    let conn = db.connect().unwrap();
    drain(&conn, "PRAGMA journal_mode = WAL").await;
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    // Reconstruct the pre-274 world: the schema now ships the composite, so the
    // baseline case must put the bare (kind) index back to have anything to measure.
    conn.execute("DROP INDEX notice_sections_kind_notice", ()).await.unwrap();
    conn.execute("CREATE INDEX notice_sections_kind ON notice_sections(kind)", ()).await.unwrap();

    let seeded = Instant::now();
    conn.execute("BEGIN", ()).await.unwrap();
    for i in 1..=SECTIONS {
        if i % 100_000 == 0 {
            conn.execute("COMMIT", ()).await.unwrap();
            conn.execute("BEGIN", ()).await.unwrap();
        }
        // ~10 sections per notice; every 10th section is FieldsPrivacy.
        let notice = 1 + i / 10;
        let privacy = i % 10 == 0;
        let kind = if privacy { "FieldsPrivacy" } else { "Lot" };
        let sid = format!("SEC-{:04}", i % 10);
        conn.execute(
            "INSERT INTO notice_sections (notice_id, section_id, kind) VALUES (?, ?, ?)",
            (Value::Integer(notice), Value::Text(sid.clone()), Value::Text(kind.into())),
        )
        .await
        .unwrap();
        if privacy {
            conn.execute(
                "INSERT INTO notice_dates (notice_id, section_id, field_id, ordinal, utc_seconds, offset_minutes, has_time)
                 VALUES (?, ?, 'BT-198(BT-105)', 0, 1700000000, 0, 0)",
                (Value::Integer(notice), Value::Text(sid.clone())),
            )
            .await
            .unwrap();
            conn.execute(
                "INSERT INTO notice_codes (notice_id, section_id, field_id, ordinal, code)
                 VALUES (?, ?, 'BT-195(BT-105)', 0, 'pro-typ')",
                (Value::Integer(notice), Value::Text(sid)),
            )
            .await
            .unwrap();
        }
    }
    conn.execute("COMMIT", ()).await.unwrap();
    println!("\n{SECTIONS} sections seeded in {:.1}s", seeded.elapsed().as_secs_f64());

    // Case A: rowid cursor over the existing (kind) index.
    const A: &str = "SELECT rowid, notice_id, section_id FROM notice_sections
         WHERE kind = 'FieldsPrivacy' AND rowid > ? ORDER BY rowid LIMIT ?";
    // Case B: notice_id cursor; the composite index is created below.
    const B: &str = "SELECT notice_id, section_id FROM notice_sections
         WHERE kind = 'FieldsPrivacy' AND notice_id > ? ORDER BY notice_id LIMIT ?";
    // Case C: the aggregates the job needs, driven from a bounded slice.
    const C: &str = "WITH slice AS (
             SELECT rowid AS rid, notice_id, section_id FROM notice_sections
              WHERE kind = 'FieldsPrivacy' AND rowid > ? ORDER BY rowid LIMIT ?)
         SELECT COUNT(*), COUNT(d.notice_id),
                COALESCE(SUM(CASE WHEN d.utc_seconds <= 1800000000 THEN 1 ELSE 0 END), 0),
                MAX(s.rid)
           FROM slice s
           LEFT JOIN notice_dates d
             ON d.notice_id = s.notice_id AND d.section_id = s.section_id
                AND d.field_id LIKE 'BT-198%'";

    // Middle-of-the-cohort cursor so a scan-based plan pays visibly.
    let mid = SECTIONS / 2;

    let (ta, na) = time(&conn, A, vec![Value::Integer(mid), Value::Integer(SLICE)]).await;
    println!("\nA rowid cursor / (kind) index:      {ta:.4}s  {na} rows");
    let (tc, nc) = time(&conn, C, vec![Value::Integer(mid), Value::Integer(SLICE)]).await;
    println!("C slice aggregates:                 {tc:.4}s  {nc} rows");

    conn.execute(
        "CREATE INDEX notice_sections_kind_notice ON notice_sections(kind, notice_id)",
        (),
    )
    .await
    .unwrap();
    let (tb, nb) = time(&conn, B, vec![Value::Integer(mid / 10), Value::Integer(SLICE)]).await;
    println!("B notice cursor / composite index:  {tb:.4}s  {nb} rows");

    plan(&conn, "A plan", A, vec![Value::Integer(mid), Value::Integer(SLICE)]).await;
    plan(&conn, "B plan", B, vec![Value::Integer(mid / 10), Value::Integer(SLICE)]).await;
    plan(&conn, "C plan", C, vec![Value::Integer(mid), Value::Integer(SLICE)]).await;

    assert_eq!(na, SLICE as usize, "A must page a full slice");
    assert_eq!(nb, SLICE as usize, "B must page a full slice");

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}
