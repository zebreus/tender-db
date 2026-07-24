//! Issue 60 (superlinear Phase-1 at 12.4M): the projection's grouping-plan bulk
//! load must stay FLAT as the plan grows — per-notice time must not rise. The
//! regression was `plan_ojs_node`'s PRIMARY KEY: keys are OJS numbers (random
//! w.r.t. insert order), so per-row `INSERT OR IGNORE` was a random-position
//! probe that thrashed once the node table outgrew the page cache. The fix
//! appends only sequential rows during Phase 1 and builds the node table once,
//! sorted, afterwards.
//!
//! This models both write patterns against turso under a deliberately small cache
//! (so the working set exceeds it at a feasible scale) and times each batch:
//! the OLD random-key pattern degrades batch over batch; the NEW append-only
//! pattern stays flat. (The real store code is exercised for correctness by the
//! projection + equivalence tests; this isolates the I/O-scaling property.)

use std::time::Instant;

use store::turso::{self, Value};

async fn drain(c: &turso::Connection, sql: &str) {
    let mut rows = c.query(sql, ()).await.unwrap();
    while rows.next().await.unwrap().is_some() {}
}

/// A key scattered across a wide range (a stand-in for OJS numbers, which are not
/// monotonic with insert order) — so PK inserts land at random b-tree positions.
fn scattered(i: i64) -> i64 {
    (i.wrapping_mul(2_654_435_761)) & 0x0000_7FFF_FFFF_FFFF
}

const BATCHES: i64 = 24;
const PER_BATCH: i64 = 4_000;

/// Ratio of the last three batches' mean time to the first three's. Flat ≈ 1.
fn tail_over_head(times: &[f64]) -> f64 {
    let head: f64 = times[..3].iter().sum::<f64>() / 3.0;
    let tail: f64 = times[times.len() - 3..].iter().sum::<f64>() / 3.0;
    tail / head.max(1e-9)
}

async fn open(tag: &str) -> (String, turso::Connection) {
    let path = format!("/tmp/tender-db-planbulk-{tag}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = turso::Builder::new_local(&path).build().await.unwrap();
    let c = db.connect().unwrap();
    drain(&c, "PRAGMA journal_mode = WAL").await;
    // A small cache so the node working set exceeds it at this scale — the same
    // condition the 512 MiB cache hit at 12.4M notices.
    c.execute("PRAGMA cache_size = -512", ()).await.unwrap(); // 512 KiB
    (path, c)
}

/// OLD pattern: random-key `INSERT OR IGNORE` into a PK node table per row.
#[tokio::test]
#[ignore = "heavy: bounded-cache bulk-load timing (~1 min); run with --ignored"]
async fn old_random_key_node_inserts_are_superlinear() {
    let (path, c) = open("old").await;
    c.execute("CREATE TABLE node(key INTEGER PRIMARY KEY, label INTEGER NOT NULL) STRICT", ())
        .await
        .unwrap();

    let mut times = Vec::new();
    for batch in 0..BATCHES {
        let t = Instant::now();
        c.execute("BEGIN", ()).await.unwrap();
        for i in 0..PER_BATCH {
            let key = scattered(batch * PER_BATCH + i);
            c.execute("INSERT OR IGNORE INTO node(key,label) VALUES(?,?)", (Value::Integer(key), Value::Integer(key)))
                .await
                .unwrap();
        }
        c.execute("COMMIT", ()).await.unwrap();
        times.push(t.elapsed().as_secs_f64());
    }
    let ratio = tail_over_head(&times);
    eprintln!("[planbulk] OLD random-key node inserts: tail/head = {ratio:.2}x");
    assert!(ratio > 1.8, "expected superlinear degradation under a bounded cache, got {ratio:.2}x");

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

/// NEW pattern: append edges (sequential rowid) per row; the node table is built
/// once, sorted, at the end — so per-batch time stays flat.
#[tokio::test]
#[ignore = "heavy: bounded-cache bulk-load timing (~1 min); run with --ignored"]
async fn new_append_only_plan_load_is_flat() {
    let (path, c) = open("new").await;
    c.execute("CREATE TABLE edge(a INTEGER NOT NULL, b INTEGER NOT NULL) STRICT", ()).await.unwrap();
    c.execute("CREATE TABLE node(key INTEGER PRIMARY KEY, label INTEGER NOT NULL) STRICT", ())
        .await
        .unwrap();

    let mut times = Vec::new();
    for batch in 0..BATCHES {
        let t = Instant::now();
        c.execute("BEGIN", ()).await.unwrap();
        for i in 0..PER_BATCH {
            let own = scattered(batch * PER_BATCH + i);
            let edge = scattered(batch * PER_BATCH + i + 1);
            // Sequential appends only (rowid) — no random index maintenance.
            c.execute("INSERT INTO edge(a,b) VALUES(?,?)", (Value::Integer(own), Value::Integer(edge))).await.unwrap();
            c.execute("INSERT INTO edge(a,b) VALUES(?,?)", (Value::Integer(edge), Value::Integer(own))).await.unwrap();
        }
        c.execute("COMMIT", ()).await.unwrap();
        times.push(t.elapsed().as_secs_f64());
    }
    let ratio = tail_over_head(&times);
    eprintln!("[planbulk] NEW append-only load: tail/head = {ratio:.2}x");
    assert!(ratio < 1.6, "append-only plan load should stay flat as it grows, got {ratio:.2}x");

    // The one-time sorted node build afterwards — the deferred work.
    let build = Instant::now();
    c.execute(
        "INSERT INTO node(key,label) SELECT k,k FROM (SELECT a AS k FROM edge UNION SELECT b FROM edge) ORDER BY k",
        (),
    )
    .await
    .unwrap();
    eprintln!("[planbulk] NEW deferred sorted node build: {:.2}s", build.elapsed().as_secs_f64());

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}
