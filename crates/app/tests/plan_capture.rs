//! Issues 428/429 — the statement capture records what turso prepares, once each,
//! in the format `plan-probe plan` reads, and does nothing unless asked.
//!
//! Its own test binary on purpose: the capture is a process-global tracing
//! subscriber, and this file is the only one whose process sets the variable.
#![cfg(feature = "server")]

#[tokio::test(flavor = "multi_thread")]
async fn records_each_distinct_prepared_statement_once() {
    let out = format!("/tmp/tender-db-plan-capture-{}.sql", std::process::id());
    let db_path = format!("/tmp/tender-db-plan-capture-{}.db", std::process::id());
    let _ = std::fs::remove_file(&out);
    let _ = std::fs::remove_file(&db_path);

    // SAFETY: set before any other thread of this test binary reads the
    // environment; this is the binary's only test.
    unsafe { std::env::set_var("TENDER_PLAN_CAPTURE", &out) };
    assert!(tender_db::plan_capture::install_from_env(), "the capture installs when the variable names a file");
    assert!(tender_db::plan_capture::install_from_env(), "and a second call is a no-op that still reports it");

    let db = store::turso::Builder::new_local(&db_path).build().await.expect("open");
    let conn = db.connect().expect("connect");
    conn.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, k TEXT)", ()).await.expect("create");
    for k in ["a", "b", "a"] {
        // The same statement three times, with different bound values: one entry.
        conn.execute("INSERT INTO t (k) VALUES (?)", (k,)).await.expect("insert");
    }
    let mut rows = conn.query("SELECT k FROM t WHERE k = ?", ("a",)).await.expect("select");
    while rows.next().await.expect("row").is_some() {}

    let text = std::fs::read_to_string(&out).expect("capture file");
    let _ = std::fs::remove_file(&out);
    let _ = std::fs::remove_file(&db_path);

    let blocks: Vec<&str> = text.split("-- name: ").skip(1).collect();
    let sql: Vec<String> =
        blocks.iter().map(|b| b.split_once('\n').map(|(_, s)| s.trim().to_owned()).unwrap_or_default()).collect();
    for want in ["CREATE TABLE t (id INTEGER PRIMARY KEY, k TEXT)", "INSERT INTO t (k) VALUES (?)", "SELECT k FROM t WHERE k = ?"] {
        let n = sql.iter().filter(|s| s.as_str() == want).count();
        assert_eq!(n, 1, "{want:?} is recorded exactly once, params never inlined — got {n} in:\n{text}");
    }
    // Every block carries a name the probe can address.
    assert!(blocks.iter().all(|b| b.split_once('\n').is_some_and(|(name, _)| !name.trim().is_empty())));
}
