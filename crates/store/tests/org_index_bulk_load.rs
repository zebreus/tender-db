//! Issue 62 (issue 60's third sub-fix): the Organization bulk load must stay FLAT
//! as the corpus grows. On a full rebuild, Phase 1 inserted every org into
//! `organizations_identity` UNIQUE(country, identifier_kind, identifier) and every
//! mention into `organization_mentions_org` ON (organization_id) — both keys
//! non-monotonic with insert order. At prod scale (254GB / 12.4M notices) this
//! thrashed: at ~4M notices the per-500k interval hit 591s (×1.78 steepening),
//! disk 99.9% util, ~3028 random reads/s, /health starved. The fix drops both
//! indexes for the rebuild bulk load — orgs then append by sequential id, mentions
//! by sequential PK — and rebuilds each index once, sorted, at the end.
//!
//! SCALE CAVEAT (measured, not assumed). Unlike `plan_ojs_node`'s `INSERT OR
//! IGNORE` — whose per-row existence *probe* is a read that thrashes at only 96k
//! rows (see `plan_bulk_load.rs`, 3.1× there) — a plain `INSERT` into these org
//! indexes is cache-tolerant at any laptop-reachable scale: turso pins a
//! transaction's dirty index pages in RAM and flushes them coalesced to the WAL,
//! so per-batch time reflects tree growth, not a random-seek storm. Isolated
//! measurements (256 KiB cache): identity-only 1.11× and mentions-org-only 1.12×
//! at 320k rows — both FLAT. The thrash only appears once the index working set
//! plus everything else the projection touches blows past a multi-hundred-MB
//! cache (millions of rows), which a unit test cannot feasibly reach. So this test
//! does NOT try to reproduce the OLD superlinearity — the prod telemetry above is
//! that evidence. It guards the property the fix owns and CAN prove: the deferred
//! bulk-load path stays flat, and it exercises both write patterns so a future
//! regression that reintroduces a laptop-scale slope is caught.

use std::time::Instant;

use store::turso::{self, Value};

async fn drain(c: &turso::Connection, sql: &str) {
    let mut rows = c.query(sql, ()).await.unwrap();
    while rows.next().await.unwrap().is_some() {}
}

/// A value scattered across a wide range (a stand-in for an org identity / an
/// older org id — neither monotonic with insert order) — so any index maintenance
/// lands at a random b-tree position.
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
    let path = format!("/tmp/tender-db-orgbulk-{tag}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = turso::Builder::new_local(&path).build().await.unwrap();
    let c = db.connect().unwrap();
    drain(&c, "PRAGMA journal_mode = WAL").await;
    // A deliberately small cache so the index working set exceeds it — the same
    // *kind* of condition prod hit, though prod's absolute scale is unreachable here.
    c.execute("PRAGMA cache_size = -512", ()).await.unwrap(); // 512 KiB
    (path, c)
}

async fn create_tables(c: &turso::Connection) {
    c.execute(
        "CREATE TABLE organizations(
             id INTEGER PRIMARY KEY AUTOINCREMENT, country TEXT, identifier_kind TEXT,
             identifier TEXT, name TEXT NOT NULL, provisional INTEGER NOT NULL,
             created_at INTEGER NOT NULL) STRICT",
        (),
    )
    .await
    .unwrap();
    c.execute(
        "CREATE TABLE organization_mentions(
             notice_id INTEGER NOT NULL, section_id TEXT NOT NULL,
             organization_id INTEGER NOT NULL, name TEXT,
             PRIMARY KEY (notice_id, section_id)) STRICT",
        (),
    )
    .await
    .unwrap();
}

/// Insert one batch exactly as the projection's Phase-1 does: a fresh org per row
/// (identity scattered) and a mention that reuses an older org (its id scattered).
async fn insert_batch(c: &turso::Connection, batch: i64) {
    c.execute("BEGIN", ()).await.unwrap();
    for i in 0..PER_BATCH {
        let n = batch * PER_BATCH + i;
        let ident = format!("{:015}", scattered(n)); // scattered org identity
        c.execute(
            "INSERT INTO organizations(country, identifier_kind, identifier, name, provisional, created_at)
             VALUES('XX','VAT',?,?,0,0)",
            (Value::Text(ident), Value::Text(format!("Org {n}"))),
        )
        .await
        .unwrap();
        // A mention reusing an already-inserted (older, random) org.
        let older = 1 + scattered(n) % (n + 1);
        c.execute(
            "INSERT INTO organization_mentions(notice_id, section_id, organization_id, name)
             VALUES(?,?,?,?)",
            (Value::Integer(n), Value::Text("ORG-1".into()), Value::Integer(older), Value::Text(format!("m{n}"))),
        )
        .await
        .unwrap();
    }
    c.execute("COMMIT", ()).await.unwrap();
}

/// NEW pattern (the fix): bare tables during the bulk load — orgs append by
/// sequential id, mentions by sequential PK — then the two indexes are built once,
/// sorted, afterwards. Per-batch time stays flat.
#[tokio::test]
#[ignore = "heavy: bounded-cache bulk-load timing (~1 min); run with --ignored"]
async fn deferred_org_load_is_flat() {
    let (path, c) = open("new").await;
    create_tables(&c).await;

    let mut times = Vec::new();
    for batch in 0..BATCHES {
        let t = Instant::now();
        insert_batch(&c, batch).await;
        times.push(t.elapsed().as_secs_f64());
    }
    let ratio = tail_over_head(&times);
    eprintln!("[orgbulk] NEW deferred org load: tail/head = {ratio:.2}x");
    assert!(ratio < 1.6, "the deferred bulk-load path must stay flat as it grows, got {ratio:.2}x");

    // The one-time sorted index builds afterwards — the deferred work, strictly
    // less than the millions of random inserts it replaces.
    let build = Instant::now();
    c.execute(
        "CREATE UNIQUE INDEX organizations_identity ON organizations(country, identifier_kind, identifier)",
        (),
    )
    .await
    .unwrap();
    c.execute("CREATE INDEX organization_mentions_org ON organization_mentions(organization_id)", ())
        .await
        .unwrap();
    eprintln!("[orgbulk] NEW deferred index builds: {:.2}s", build.elapsed().as_secs_f64());

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

/// The OLD pattern (both org indexes present per row) at laptop scale — kept to
/// DOCUMENT, with a measurement, that the thrash is NOT reproducible here: a plain
/// INSERT into these indexes is cache-tolerant in turso (see the module comment),
/// so the OLD path is also flat at any scale a unit test can reach. This is why
/// the fix rests on prod telemetry + `project_equivalence`, not on a local
/// OLD-vs-NEW contrast. If turso's write behavior ever changes such that these
/// inserts DO thrash at this scale, this test's ratio rises and flags it.
#[tokio::test]
#[ignore = "heavy: bounded-cache bulk-load timing (~1 min); run with --ignored"]
async fn indexed_org_load_is_flat_at_laptop_scale() {
    let (path, c) = open("old").await;
    create_tables(&c).await;
    c.execute(
        "CREATE UNIQUE INDEX organizations_identity ON organizations(country, identifier_kind, identifier)",
        (),
    )
    .await
    .unwrap();
    c.execute("CREATE INDEX organization_mentions_org ON organization_mentions(organization_id)", ())
        .await
        .unwrap();

    let mut times = Vec::new();
    for batch in 0..BATCHES {
        let t = Instant::now();
        insert_batch(&c, batch).await;
        times.push(t.elapsed().as_secs_f64());
    }
    let ratio = tail_over_head(&times);
    eprintln!("[orgbulk] OLD indexed org inserts (laptop scale): tail/head = {ratio:.2}x");
    // Not a thrash at this scale — see the module comment; prod's 591s/×1.78 at
    // 4M notices is the real evidence. A generous ceiling catches a genuine
    // regression without asserting a contrast that does not exist here.
    assert!(ratio < 1.7, "even indexed, laptop-scale inserts stay roughly flat, got {ratio:.2}x");

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}
