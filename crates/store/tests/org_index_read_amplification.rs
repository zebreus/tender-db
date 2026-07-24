//! Issue 62 root-cause probe: does inserting a row into a scattered-key index do
//! a random READ (a b-tree traversal to find the insert position / check
//! uniqueness) that turso's page cache cannot absorb once the index outgrows it?
//!
//! The wall-clock bulk-load test (`org_index_bulk_load.rs`) showed the OLD indexed
//! path is FLAT at laptop scale — but flat wall-clock is ambiguous: it can mean
//! "no reads" OR "reads served from the OS page cache in microseconds" (the whole
//! 96k test DB fits in RAM). This test disambiguates by counting turso's own read
//! bytes (`/proc/self/io` `rchar`, which counts pread bytes regardless of the OS
//! cache) under a deliberately small turso cache. If the scattered-index inserts
//! read far more than sequential appends, then the per-row insert DOES issue a
//! random read-seek — cheap here because it hits the OS cache, but exactly the
//! seek that becomes a disk seek on the 254GB prod DB (prod: 3028 random reads/s
//! at 4M notices). That is the mechanism issue 62 removes by deferring the indexes.
//!
//! It also answers the sharper question: is a UNIQUE index's uniqueness *probe* an
//! EXTRA read beyond a non-unique index's position-finding traversal? Compare the
//! non-unique and unique columns.

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

fn scattered(i: i64) -> i64 {
    (i.wrapping_mul(2_654_435_761)) & 0x0000_7FFF_FFFF_FFFF
}

const ROWS: i64 = 120_000;
const PER_TXN: i64 = 4_000;

async fn open(tag: &str) -> (String, turso::Connection) {
    let path = format!("/tmp/tender-db-orgread-{tag}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = turso::Builder::new_local(&path).build().await.unwrap();
    let c = db.connect().unwrap();
    drain(&c, "PRAGMA journal_mode = WAL").await;
    // Small cache so the scattered index working set exceeds it — the same kind of
    // condition prod hit, at a feasible scale.
    c.execute("PRAGMA cache_size = -256", ()).await.unwrap(); // 256 KiB
    (path, c)
}

/// Insert ROWS rows into `sql_insert`, keyed by `key(i)`, measuring turso read
/// bytes across the whole insert phase. Returns rchar bytes read.
async fn measure_inserts(c: &turso::Connection, sql_insert: &str, scatter_key: bool) -> u64 {
    drain(c, "PRAGMA wal_checkpoint(TRUNCATE)").await;
    let before = rchar();
    let mut i = 0i64;
    while i < ROWS {
        c.execute("BEGIN", ()).await.unwrap();
        for _ in 0..PER_TXN {
            let k = if scatter_key { scattered(i) } else { i };
            c.execute(sql_insert, (Value::Integer(i), Value::Integer(k))).await.unwrap();
            i += 1;
        }
        c.execute("COMMIT", ()).await.unwrap();
    }
    rchar() - before
}

#[tokio::test]
#[ignore = "heavy: read-amplification probe (~1-2 min); run with --ignored"]
async fn scattered_index_inserts_do_read_seeks_sequential_appends_do_not() {
    // A. bare table, sequential rowid append (the NEW deferred org/mention path):
    //    each insert lands on the rightmost leaf — one hot page — so ~no reads.
    let (pa, ca) = open("bare").await;
    ca.execute("CREATE TABLE t(id INTEGER PRIMARY KEY, k INTEGER NOT NULL) STRICT", ()).await.unwrap();
    let bare = measure_inserts(&ca, "INSERT INTO t(id,k) VALUES(?,?)", false).await;

    // B. non-unique scattered secondary index (organization_mentions_org): the PK
    //    is sequential, but the index on the scattered column forces a traversal to
    //    a random leaf per row.
    let (pb, cb) = open("nonuniq").await;
    cb.execute("CREATE TABLE t(id INTEGER PRIMARY KEY, k INTEGER NOT NULL) STRICT", ()).await.unwrap();
    cb.execute("CREATE INDEX t_k ON t(k)", ()).await.unwrap();
    let nonuniq = measure_inserts(&cb, "INSERT INTO t(id,k) VALUES(?,?)", true).await;

    // C. unique scattered secondary index (organizations_identity): same traversal,
    //    plus a uniqueness check — is that an EXTRA read over B?
    let (pc, cc) = open("uniq").await;
    cc.execute("CREATE TABLE t(id INTEGER PRIMARY KEY, k INTEGER NOT NULL) STRICT", ()).await.unwrap();
    cc.execute("CREATE UNIQUE INDEX t_k ON t(k)", ()).await.unwrap();
    let uniq = measure_inserts(&cc, "INSERT INTO t(id,k) VALUES(?,?)", true).await;

    let mib = |b: u64| b as f64 / 1_048_576.0;
    let per_row = |b: u64| b as f64 / ROWS as f64;
    eprintln!(
        "[orgread] {ROWS} inserts, 256KiB cache — rchar:\n  \
         A bare-sequential : {:8.2} MiB ({:6.0} B/row)\n  \
         B nonuniq-scatter : {:8.2} MiB ({:6.0} B/row)  = {:.0}x A\n  \
         C unique-scatter  : {:8.2} MiB ({:6.0} B/row)  = {:.0}x A, {:.2}x B",
        mib(bare), per_row(bare),
        mib(nonuniq), per_row(nonuniq), nonuniq as f64 / bare.max(1) as f64,
        mib(uniq), per_row(uniq), uniq as f64 / bare.max(1) as f64, uniq as f64 / nonuniq.max(1) as f64,
    );

    // The decisive claim: a scattered-index insert reads far more than a
    // sequential append. Sequential appends touch one hot leaf (≈0 reads); the
    // scattered inserts traverse to random leaves that evict from the small cache
    // and must be re-read — the read-seek that becomes a disk seek at prod scale.
    assert!(
        nonuniq > bare * 4 && uniq > bare * 4,
        "expected scattered-index inserts to read >>4x the sequential baseline \
         (bare={bare} nonuniq={nonuniq} uniq={uniq})"
    );

    for (p, _) in [(pa, ()), (pb, ()), (pc, ())] {
        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{p}{s}"));
        }
    }
}
