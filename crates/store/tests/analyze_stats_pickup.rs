//! Issue 429 step 4 — does a connection that was ALREADY open see statistics
//! another connection's ANALYZE wrote? MEASURED 2026-09-27: **no** — and a schema
//! change is what makes it.
//!
//! It matters because the server's readers are long-lived connections on one
//! `turso::Database`, and the weekly ANALYZE job runs on the writer. turso loads
//! `sqlite_stat1` into a connection's schema at (re)parse; on prod `sqlite_stat1`
//! already exists (the three internal tables), so a later ANALYZE changes no
//! schema and nothing obviously forces the readers to reparse.
//!
//! The probe shape is one where stats visibly flip the plan: `t` is large with a
//! low-cardinality indexed column, `u` is small, and without stats both look like a
//! million rows — so the planner seeks `t_k`; with stats it scans the small `u` and
//! seeks `t` by primary key (measured, issue 428's toy run).
//!
//! What this pins, both halves, so a turso upgrade that changes either one fails
//! here instead of silently:
//!
//! * a reader opened before the ANALYZE keeps planning with the statistics it
//!   loaded at open — the writer and any NEW connection see the fresh ones, so the
//!   server would run with two plans for one query until its next restart;
//! * a schema change (a throwaway CREATE + DROP) makes every connection reparse at
//!   its next statement, and the reparse reloads `sqlite_stat1` — which is how the
//!   weekly `analyze` job hands its statistics to the readers without a restart.

use store::turso;

async fn plan(conn: &turso::Connection) -> Vec<String> {
    let mut rows = conn
        .query("EXPLAIN QUERY PLAN SELECT * FROM t JOIN u ON u.t_id = t.id WHERE t.k = 'b'", ())
        .await
        .expect("explain");
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.expect("row") {
        if let Ok(turso::Value::Text(detail)) = row.get_value(3) {
            out.push(detail);
        }
    }
    out
}

#[tokio::test(flavor = "multi_thread")]
async fn a_pooled_reader_keeps_old_statistics_until_a_schema_change() {
    let path = format!("/tmp/tender-db-analyze-pickup-{}.db", std::process::id());
    let _ = std::fs::remove_file(&path);
    let db = turso::Builder::new_local(&path).build().await.expect("open");
    let writer = db.connect().expect("writer");
    writer
        .execute_batch(
            "CREATE TABLE t (id INTEGER PRIMARY KEY, k TEXT, v INTEGER);
             CREATE INDEX t_k ON t(k);
             CREATE TABLE u (id INTEGER PRIMARY KEY, t_id INTEGER, x TEXT);
             CREATE INDEX u_t ON u(t_id);
             CREATE TABLE z (id INTEGER PRIMARY KEY, w TEXT);
             CREATE INDEX z_w ON z(w);",
        )
        .await
        .expect("schema");
    writer.execute("BEGIN", ()).await.expect("begin");
    for i in 0..20_000 {
        let k = if i % 100 == 0 { "b" } else { "a" };
        writer.execute("INSERT INTO t (k, v) VALUES (?, ?)", (k, i)).await.expect("t");
    }
    for i in 0..3_000 {
        writer.execute("INSERT INTO u (t_id, x) VALUES (?, 'x')", (i % 50,)).await.expect("u");
    }
    writer.execute("INSERT INTO z (w) VALUES ('w')", ()).await.expect("z");
    writer.execute("COMMIT", ()).await.expect("commit");
    // Prod's state: `sqlite_stat1` already exists, holding rows for other tables.
    writer.execute("ANALYZE z", ()).await.expect("analyze z");

    // The long-lived reader, opened BEFORE the stats for `t`/`u` exist.
    let reader = db.connect().expect("reader");
    let before = plan(&reader).await;

    writer.execute("ANALYZE t", ()).await.expect("analyze t");
    writer.execute("ANALYZE u", ()).await.expect("analyze u");

    let reader_after = plan(&reader).await;
    let writer_after = plan(&writer).await;
    let fresh_after = plan(&db.connect().expect("fresh")).await;

    // The candidate refresh: a schema change makes every connection reparse at its
    // next statement, and the reparse reloads `sqlite_stat1`.
    writer.execute_batch("CREATE TABLE stats_refresh_bump (x INTEGER); DROP TABLE stats_refresh_bump;").await.expect("bump");
    let reader_bumped = plan(&reader).await;
    let _ = std::fs::remove_file(&path);

    eprintln!("before (reader):  {before:?}");
    eprintln!("after  (reader):  {reader_after:?}");
    eprintln!("after  (writer):  {writer_after:?}");
    eprintln!("after  (fresh):   {fresh_after:?}");
    eprintln!("bumped (reader):  {reader_bumped:?}");

    // The instrument must discriminate: stats change this plan at all.
    assert_ne!(before, fresh_after, "the probe shape must flip with stats, or it proves nothing");
    assert_eq!(writer_after, fresh_after, "the analyzing connection sees its own stats");
    assert_eq!(
        reader_after, before,
        "a reader opened before ANALYZE keeps its old statistics — if this now fails, turso refreshes \
         them on its own and the job's schema bump is no longer needed"
    );
    assert_eq!(reader_bumped, fresh_after, "after a schema change the same reader plans with the new statistics");
}
