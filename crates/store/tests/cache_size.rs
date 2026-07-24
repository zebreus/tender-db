//! Issue 60: the store's `cache_size` PRAGMA must actually change turso's read
//! behaviour — otherwise the projection's Phase-1 read amplification (a ~2 MB
//! default cache re-reading B-tree interior pages it can't hold, on the multi-
//! hundred-GB prod DB) has no cheap fix.
//!
//! This reproduces the *mechanism* at a feasible scale: repeatedly scan a table
//! larger than a tiny cache but smaller than a large one, and count read-syscall
//! bytes (`/proc/self/io` `rchar`). With the tiny cache every scan re-reads the
//! table from the file; with the large cache the table stays resident and the
//! repeat scans do almost no reads. That the two differ by orders of magnitude is
//! the proof that turso honors `cache_size` — the premise of the issue-60 fix
//! (`PRAGMA cache_size = -524288` in the store's connection PRAGMAS).

use store::turso::{self, Value};

fn rchar() -> u64 {
    let io = std::fs::read_to_string("/proc/self/io").expect("read /proc/self/io");
    for line in io.lines() {
        if let Some(v) = line.strip_prefix("rchar:") {
            return v.trim().parse().unwrap();
        }
    }
    panic!("no rchar in /proc/self/io");
}

async fn drain(c: &turso::Connection, sql: &str) {
    let mut rows = c.query(sql, ()).await.unwrap();
    while rows.next().await.unwrap().is_some() {}
}

#[tokio::test]
async fn turso_honors_cache_size_so_a_large_cache_stops_re_reads() {
    let path = format!("/tmp/tender-db-cachesize-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = turso::Builder::new_local(&path).build().await.unwrap();
    let c = db.connect().unwrap();
    drain(&c, "PRAGMA journal_mode = WAL").await;

    // ~40k rows of five i64 columns ≈ a couple MB: bigger than the tiny cache
    // below, smaller than the large one.
    c.execute("CREATE TABLE t(id INTEGER PRIMARY KEY, a INTEGER, b INTEGER, c INTEGER, d INTEGER)", ())
        .await
        .unwrap();
    c.execute("BEGIN", ()).await.unwrap();
    for i in 1..=40_000i64 {
        c.execute(
            "INSERT INTO t(a,b,c,d) VALUES(?,?,?,?)",
            (Value::Integer(i), Value::Integer(i * 2), Value::Integer(i * 3), Value::Integer(i * 4)),
        )
        .await
        .unwrap();
    }
    c.execute("COMMIT", ()).await.unwrap();
    drain(&c, "PRAGMA wal_checkpoint(TRUNCATE)").await;

    const SCANS: usize = 8;
    let scan = |c: turso::Connection| async move {
        for _ in 0..SCANS {
            drain(&c, "SELECT COALESCE(SUM(a + b + c + d), 0) FROM t").await;
        }
    };

    // Tiny cache (64 KiB): each scan re-reads the whole table from the file.
    c.execute("PRAGMA cache_size = -64", ()).await.unwrap();
    drain(&c, "SELECT COALESCE(SUM(a + b + c + d), 0) FROM t").await; // warm out first-touch
    let before_tiny = rchar();
    scan(c.clone()).await;
    let tiny = rchar() - before_tiny;

    // Large cache (256 MiB): the table stays resident, so repeat scans read ~nothing.
    c.execute("PRAGMA cache_size = -262144", ()).await.unwrap();
    drain(&c, "SELECT COALESCE(SUM(a + b + c + d), 0) FROM t").await; // fill the cache once
    let before_large = rchar();
    scan(c.clone()).await;
    let large = rchar() - before_large;

    eprintln!(
        "[cache_size] rchar over {SCANS} scans — tiny(64KiB): {:.2} MiB | large(256MiB): {:.2} MiB (ratio {:.0}x)",
        tiny as f64 / 1_048_576.0,
        large as f64 / 1_048_576.0,
        tiny as f64 / large.max(1) as f64,
    );
    // A large-enough cache eliminates the re-reads — turso honors cache_size, so
    // the issue-60 PRAGMA is a real fix, not a no-op.
    assert!(
        large * 4 < tiny,
        "turso ignored cache_size: large-cache reads ({large} B) not far below tiny-cache ({tiny} B)"
    );

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}
