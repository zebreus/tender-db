//! Issue 425 — does turso's own per-statement deadline stop the queries `/v1/sql`
//! cannot stop today, how far past the deadline, and is the connection usable after?
//!
//! `/v1/sql` has no way to stop a query: its 10 s cap answers 408 while the
//! computation runs on to completion, holding one of the endpoint's reader
//! connections (issue 417 counts and caps those; 2026-09-18 two of them held the
//! endpoint for 13.5 min). turso 0.7.2 checks a per-statement deadline before every
//! VDBE instruction; the vendored SDK (crates/vendor/turso) passes
//! `set_query_timeout` through. This file measures it on the shapes that matter:
//!
//! * a non-yielding aggregate over a table-valued function (the shape issue 51's
//!   tokio timeout could never stop),
//! * a nested-loop join explosion,
//! * a full sort (a sorter can do a lot of work inside ONE instruction — the
//!   granularity is per instruction, so it is measured, not assumed),
//! * a GROUP BY over every row.
//!
//! (An unbounded recursive CTE is not on the list because it cannot run at all:
//! turso 0.7.2 refuses `WITH RECURSIVE` at parse — "Recursive CTEs are not yet
//! supported" — measured here 2026-09-27.)
//!
//! `TENDER_TIMEOUT_PROBE_ROWS` sizes the table (default small, so the gate stays
//! fast; the recorded measurement used 3,000,000 in release). Every shape must end
//! in `Error::Interrupt` within a bounded overshoot, and a plain `SELECT` on the
//! same connection must work afterwards.

use std::time::{Duration, Instant};
use store::turso;

const DEADLINE: Duration = Duration::from_millis(300);
/// Generous on purpose: debug builds, a loaded CI box. The measured overshoots are
/// printed and recorded on issue 425; this bound only catches "did not stop".
const SLACK: Duration = Duration::from_secs(10);

async fn drain(conn: &turso::Connection, sql: &str) -> (Result<u64, turso::Error>, Duration) {
    let started = Instant::now();
    let result = async {
        let mut rows = conn.query(sql, ()).await?;
        let mut n = 0u64;
        while rows.next().await?.is_some() {
            n += 1;
        }
        Ok(n)
    }
    .await;
    (result, started.elapsed())
}

#[tokio::test(flavor = "multi_thread")]
async fn the_engine_deadline_stops_every_offender_and_the_connection_survives() {
    let rows: i64 = std::env::var("TENDER_TIMEOUT_PROBE_ROWS").ok().and_then(|v| v.parse().ok()).unwrap_or(200_000);
    let path = format!("/tmp/tender-db-timeout-probe-{}.db", std::process::id());
    let _ = std::fs::remove_file(&path);
    let db = turso::Builder::new_local(&path).build().await.expect("open");
    let conn = db.connect().expect("connect");
    conn.execute_batch("CREATE TABLE big (id INTEGER PRIMARY KEY, k INTEGER, v INTEGER)").await.expect("schema");
    conn.execute(
        "INSERT INTO big (k, v) SELECT value % 1000, (value * 2654435761) % 1000003 FROM generate_series(1, ?)",
        (rows,),
    )
    .await
    .expect("fill");

    conn.set_query_timeout(DEADLINE).expect("set_query_timeout");

    let offenders = [
        ("series aggregate", "SELECT count(*) FROM generate_series(1, 100000000000)".to_owned()),
        ("nested-loop join", "SELECT count(*) FROM big a, big b WHERE a.v + b.v = -1".to_owned()),
        ("full sort", "SELECT id FROM big ORDER BY (v * 7919) % 1000003, k LIMIT 1".to_owned()),
        ("group by", "SELECT (v * 31) % 100003 AS g, count(*) FROM big GROUP BY g ORDER BY 2 DESC LIMIT 1".to_owned()),
    ];

    let mut report = Vec::new();
    for (label, sql) in &offenders {
        let (result, took) = drain(&conn, sql).await;
        let overshoot = took.saturating_sub(DEADLINE);
        eprintln!("[425] rows={rows} {label:<18} {:>8.3}s  overshoot {:>7.3}s  {:?}", took.as_secs_f64(), overshoot.as_secs_f64(),
            result.as_ref().map_err(|e| e.to_string()));
        // The connection must still work — the thing a stuck reader never did.
        let (after, _) = drain(&conn, "SELECT count(*) FROM big WHERE id <= 10").await;
        report.push((label, result, took, after));
    }
    let _ = std::fs::remove_file(&path);

    for (label, result, took, after) in report {
        assert!(
            matches!(result, Err(turso::Error::Interrupt(_))),
            "{label}: the engine deadline must interrupt it — got {result:?} after {took:?}"
        );
        assert!(took < DEADLINE + SLACK, "{label}: stopped, but {took:?} is far past the {DEADLINE:?} deadline");
        assert!(matches!(after, Ok(1)), "{label}: the connection must be usable after the interrupt — got {after:?}");
    }
}

/// Issue 120 (the REST half of 425): the bound is per POOL. A pool bounded with
/// `Readers::bound_statements` stops a statement at the deadline and takes the
/// connection back usable; a pool opened beside it on the same database — the shape
/// every job's reads have (the weekly data-quality windows run for minutes by design)
/// — carries no deadline and runs the same kind of statement past it to the end.
#[tokio::test(flavor = "multi_thread")]
async fn a_bounded_pool_stops_its_statements_and_an_unbounded_one_does_not() {
    const POOL_DEADLINE: Duration = Duration::from_millis(50);
    let path = format!("/tmp/tender-db-bounded-pool-{}.db", std::process::id());
    let _ = std::fs::remove_file(&path);
    let db = store::Db::open(&path).await.expect("open");
    let bounded = db.readers(1).expect("bounded pool");
    bounded.bound_statements(POOL_DEADLINE);
    let free = db.readers(1).expect("unbounded pool");
    assert_eq!(bounded.statement_deadline(), Some(POOL_DEADLINE));
    assert_eq!(free.statement_deadline(), None, "a pool is unbounded unless someone bounds it");

    let reader = bounded.get().await.expect("reader");
    let (stopped, took) = drain(&reader, "SELECT count(*) FROM generate_series(1, 100000000000)").await;
    drop(reader);
    // A pool of one hands the SAME connection back: it must still work.
    let reader = bounded.get().await.expect("reader");
    let (after, _) = drain(&reader, "SELECT 1").await;
    drop(reader);

    let reader = free.get().await.expect("reader");
    let (finished, ran) = drain(&reader, "SELECT count(*) FROM generate_series(1, 5000000)").await;
    drop(reader);
    drop(db);
    let _ = std::fs::remove_file(&path);

    assert!(matches!(stopped, Err(turso::Error::Interrupt(_))), "the bounded pool must stop it — got {stopped:?} after {took:?}");
    assert!(took < POOL_DEADLINE + SLACK, "stopped, but {took:?} is far past the {POOL_DEADLINE:?} deadline");
    assert!(matches!(after, Ok(1)), "the stopped connection must come back from the pool usable — got {after:?}");
    assert!(matches!(finished, Ok(1)), "the unbounded pool must run the statement to its end — got {finished:?}");
    assert!(ran > POOL_DEADLINE, "precondition: the finite statement must outlast the deadline to prove anything — ran {ran:?}");
}
