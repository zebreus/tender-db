//! Task 5: can an app-layer scan budget ABORT a warm walk, or only refuse before one
//! starts?
//!
//! This decides #5's direction. A budget that aborts mid-flight bounds worst-case
//! work for the whole matches-late class — every filter, including `?cpv=`, which the
//! country junction cannot serve. A budget that can only refuse *before* starting
//! needs a cost oracle to decide what to refuse, and we do not have one (no
//! selectivity statistics), so it would not cover the class at all.
//!
//! The hypothesis to falsify (team-lead's, and it is the pessimistic one): turso
//! 0.7.0's `Statement::step` yields only on IO, so a walk whose pages are already in
//! cache never returns to the executor, and `tokio::time::timeout` — which can only
//! act at an await point — never gets the chance to fire.
//!
//! The load-bearing case is a filter matching NOTHING over a large warm table. The
//! first `rows.next().await` must then scan the entire table before it can answer
//! "no rows", so that single await is where a whole walk happens. If a timeout cannot
//! interrupt THAT, it cannot bound anything.
//!
//! Run: `cargo test -p store --test scan_budget_probe -- --ignored --nocapture`

use std::time::{Duration, Instant};
use store::turso::{self, Value};

const ROWS: i64 = 4_000_000;
/// Deliberately far below the walk's duration: if the budget can act at all, it acts
/// here, and the elapsed time reports which happened.
const BUDGET: Duration = Duration::from_millis(50);

async fn drain(conn: &turso::Connection, sql: &str) {
    let mut rows = conn.query(sql, ()).await.unwrap();
    while rows.next().await.unwrap().is_some() {}
}

#[tokio::test]
#[ignore = "probe: task 5 scan-budget feasibility; run with --ignored"]
async fn current_thread_runtime() {
    probe("current_thread").await;
}

/// The flavour PRODUCTION runs. On a current-thread runtime a non-yielding poll
/// trivially starves the timer, so that result alone would not settle it — the timer
/// could plausibly fire on another worker. It cannot: `timeout` polls the inner
/// future and the sleep from the SAME task, so a poll that never returns `Pending`
/// keeps control regardless of how many threads exist. That is reasoning, so it is
/// measured here rather than asserted.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "probe: task 5 scan-budget feasibility; run with --ignored"]
async fn multi_thread_runtime() {
    probe("multi_thread").await;
}

async fn probe(flavour: &str) {
    let path = format!("/tmp/tender-db-budget-{flavour}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    store::Db::open(&path).await.unwrap();
    let db = turso::Builder::new_local(&path).build().await.unwrap();
    let conn = db.connect().unwrap();
    drain(&conn, "PRAGMA journal_mode = WAL").await;
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();

    let seeded = Instant::now();
    conn.execute("BEGIN", ()).await.unwrap();
    for i in 0..ROWS {
        if i > 0 && i % 500_000 == 0 {
            conn.execute("COMMIT", ()).await.unwrap();
            conn.execute("BEGIN", ()).await.unwrap();
        }
        conn.execute(
            "INSERT INTO organizations (id, country, identifier_kind, identifier, name, provisional, created_at)
             VALUES (?, 'DE', 'vat', ?, ?, 0, 1700000000)",
            (
                Value::Integer(i),
                Value::Text(format!("{i:012}")),
                Value::Text(format!("org {i}")),
            ),
        )
        .await
        .unwrap();
    }
    conn.execute("COMMIT", ()).await.unwrap();
    println!("\n[{flavour}] {ROWS} organizations seeded in {:.1}s", seeded.elapsed().as_secs_f64());

    // Matches nothing, so the first row cannot be produced until the whole table has
    // been walked. No index on `country` here — this is the pre-117 shape, which is
    // exactly the walk a budget would need to bound.
    const WALK: &str = "SELECT o.id FROM organizations o
                         WHERE o.country = 'ZZ' AND o.id > 0 ORDER BY o.id LIMIT 50";

    // Warm the cache, and measure the unbudgeted walk.
    let mut baseline = f64::MAX;
    for _ in 0..3 {
        let t = Instant::now();
        let mut rows = conn.query(WALK, ()).await.unwrap();
        while rows.next().await.unwrap().is_some() {}
        baseline = baseline.min(t.elapsed().as_secs_f64());
    }
    println!("[{flavour}] unbudgeted warm walk: {baseline:.4}s");
    assert!(
        baseline > BUDGET.as_secs_f64() * 4.0,
        "the walk must be much longer than the budget for this probe to mean anything \
         — walk {baseline:.4}s vs budget {:?}",
        BUDGET
    );

    // THE QUESTION. A budget wrapped around the whole read, exactly as an app-layer
    // guard would be written.
    let t = Instant::now();
    let outcome = tokio::time::timeout(BUDGET, async {
        let mut rows = conn.query(WALK, ()).await.unwrap();
        let mut n = 0;
        while rows.next().await.unwrap().is_some() {
            n += 1;
        }
        n
    })
    .await;
    let elapsed = t.elapsed().as_secs_f64();

    match outcome {
        Err(_) => println!("\nBUDGET FIRED after {elapsed:.4}s (budget {:?})", BUDGET),
        Ok(n) => println!("\nbudget did NOT fire: the read completed in {elapsed:.4}s, {n} rows"),
    }

    let aborted_promptly = outcome.is_err() && elapsed < baseline / 2.0;
    println!(
        "\n[{flavour}] VERDICT: an app-layer timeout {} abort a warm walk.\n  \
         walk {baseline:.4}s, budget {:?}, returned after {elapsed:.4}s",
        if aborted_promptly { "CAN" } else { "CANNOT" },
        BUDGET
    );
    if !aborted_promptly {
        println!(
            "  -> the walk ran to completion regardless of the budget, so a scan budget\n     \
             at the app layer can only REFUSE BEFORE STARTING, which needs a cost\n     \
             oracle (selectivity stats) we do not have. #5 cannot be a general budget\n     \
             without vendoring turso's interrupt."
        );
    }

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}
