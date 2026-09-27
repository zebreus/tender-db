//! Issue 438 (replacing issue 425's `query_timeout_probe.rs`) — the engine facts the
//! API's read deadline stands on. `tender-db`'s `v1::stop` stops a read with a timer
//! that calls turso's `interrupt()` from another thread, repeating every 50 ms until
//! the read lets go of its connection, and never lends an interrupted connection
//! again. That is sound only if:
//!
//! 1. `interrupt()` from another thread stops the shapes a tokio timeout cannot — a
//!    non-yielding aggregate over a table-valued function (issue 51's), a nested-loop
//!    join, a full sort (a sorter can do a lot inside ONE instruction, and the flag is
//!    read between instructions) and a GROUP BY over every row — and the connection
//!    works after;
//! 2. an interrupt with nothing running is IGNORED (the repeats land between
//!    statements all the time, and must not poison the next one), while a statement
//!    paused between rows IS stopped at its next step (what a read past its limit
//!    that is between rows gets, and what the API test holding a statement open
//!    across a stray timer would see);
//! 3. [`store::Reader::discard`] keeps a connection out of the pool.
//!
//! (Issue 425 measured turso's per-statement deadline, `set_query_timeout`, here. It
//! stopped the same shapes within 0–71 ms, but while one is set turso reads the clock
//! before every VDBE instruction — 1.7–2.9× slower reads — and issue 438 took it off
//! every serving connection.)
//!
//! `TENDER_INTERRUPT_PROBE_ROWS` sizes the table (default small, so the gate stays
//! fast).

use std::time::{Duration, Instant};
use store::turso;

/// When the interrupting thread fires, after the statement started.
const AFTER: Duration = Duration::from_millis(300);
/// Generous on purpose: debug builds, a loaded CI box. This bound only catches "did
/// not stop".
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

/// `conn.interrupt()` from another OS thread `after` from now — the timer's position:
/// not on the thread the statement runs on, which a non-yielding step never lets go.
fn interrupt_after(conn: &turso::Connection, after: Duration) -> std::thread::JoinHandle<()> {
    let conn = conn.clone();
    std::thread::spawn(move || {
        std::thread::sleep(after);
        conn.interrupt().expect("interrupt");
    })
}

async fn scratch(name: &str, rows: i64) -> (turso::Database, String) {
    let path = format!("/tmp/tender-db-interrupt-probe-{name}-{}.db", std::process::id());
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
    (db, path)
}

#[tokio::test(flavor = "multi_thread")]
async fn an_interrupt_from_another_thread_stops_every_offender_and_the_connection_survives() {
    let rows: i64 = std::env::var("TENDER_INTERRUPT_PROBE_ROWS").ok().and_then(|v| v.parse().ok()).unwrap_or(200_000);
    let (db, path) = scratch("offenders", rows).await;
    let conn = db.connect().expect("connect");

    let offenders = [
        ("series aggregate", "SELECT count(*) FROM generate_series(1, 100000000000)"),
        ("nested-loop join", "SELECT count(*) FROM big a, big b WHERE a.v + b.v = -1"),
        ("full sort", "SELECT id FROM big ORDER BY (v * 7919) % 1000003, k LIMIT 1"),
        ("group by", "SELECT (v * 31) % 100003 AS g, count(*) FROM big GROUP BY g ORDER BY 2 DESC LIMIT 1"),
    ];

    let mut report = Vec::new();
    for (label, sql) in offenders {
        let interrupter = interrupt_after(&conn, AFTER);
        let (result, took) = drain(&conn, sql).await;
        interrupter.join().expect("the interrupting thread");
        eprintln!(
            "[438] rows={rows} {label:<18} {:>8.3}s  overshoot {:>7.3}s  {:?}",
            took.as_secs_f64(),
            took.saturating_sub(AFTER).as_secs_f64(),
            result.as_ref().map_err(|e| e.to_string())
        );
        // The connection must still work — the thing a stuck reader never did.
        let (after, _) = drain(&conn, "SELECT count(*) FROM big WHERE id <= 10").await;
        report.push((label, result, took, after));
    }
    drop(conn);
    drop(db);
    let _ = std::fs::remove_file(&path);

    for (label, result, took, after) in report {
        assert!(
            matches!(result, Err(turso::Error::Interrupt(_))),
            "{label}: the interrupt must stop it — got {result:?} after {took:?}"
        );
        assert!(took < AFTER + SLACK, "{label}: stopped, but {took:?} is far past the {AFTER:?} interrupt");
        assert!(matches!(after, Ok(1)), "{label}: the connection must be usable after the interrupt — got {after:?}");
    }
}

/// The two halves of turso's `sqlite3_interrupt` rule the timer's repeats rely on.
#[tokio::test(flavor = "multi_thread")]
async fn an_interrupt_with_nothing_running_is_ignored_and_a_paused_statement_is_stopped() {
    let (db, path) = scratch("idle", 100).await;
    let conn = db.connect().expect("connect");

    // Idle — never used, and then between two statements of one "read": ignored, so the
    // next statement runs to its end.
    conn.interrupt().expect("interrupt");
    let (fresh, _) = drain(&conn, "SELECT count(*) FROM big").await;
    conn.interrupt().expect("interrupt");
    let (between, _) = drain(&conn, "SELECT id FROM big WHERE id <= 5").await;

    // Paused between rows: the statement is still active, so the interrupt is taken
    // and its next step fails — and the statement after that runs normally.
    let paused = async {
        let mut rows = conn.query("SELECT id FROM big ORDER BY id", ()).await?;
        rows.next().await?.expect("a first row");
        conn.interrupt()?;
        let mut more = 0u64;
        while rows.next().await?.is_some() {
            more += 1;
        }
        Ok::<u64, turso::Error>(more)
    }
    .await;
    let (after, _) = drain(&conn, "SELECT count(*) FROM big").await;
    drop(conn);
    drop(db);
    let _ = std::fs::remove_file(&path);

    assert!(matches!(fresh, Ok(1)), "an interrupt on an idle connection must be ignored — got {fresh:?}");
    assert!(matches!(between, Ok(5)), "an interrupt between statements must be ignored — got {between:?}");
    assert!(
        matches!(paused, Err(turso::Error::Interrupt(_))),
        "a statement paused between rows must be stopped at its next step — got {paused:?}"
    );
    assert!(matches!(after, Ok(1)), "the statement after an interrupted one runs normally — got {after:?}");
}

/// `Reader::discard`: a connection whose borrow sent an interrupt is dropped, not
/// pooled (issue 438). `PRAGMA query_only` is per-connection state the pool never
/// sets, so it tells a reused connection from a fresh one.
#[tokio::test(flavor = "multi_thread")]
async fn a_discarded_reader_does_not_go_back_to_the_pool() {
    let path = format!("/tmp/tender-db-interrupt-probe-discard-{}.db", std::process::id());
    let _ = std::fs::remove_file(&path);
    let db = store::Db::open(&path).await.expect("open");
    let pool = db.readers(1).expect("pool");
    async fn query_only(conn: &turso::Connection) -> i64 {
        let mut rows = conn.query("PRAGMA query_only", ()).await.expect("read query_only");
        rows.next().await.expect("step").expect("a row").get::<i64>(0).expect("an integer")
    }

    let reader = pool.get().await.expect("reader");
    let (set, _) = drain(&reader, "PRAGMA query_only = 1").await;
    set.expect("set query_only");
    drop(reader);
    let mut reader = pool.get().await.expect("reader");
    let reused = query_only(&reader).await;
    reader.discard();
    drop(reader);
    let reader = pool.get().await.expect("reader");
    let fresh = query_only(&reader).await;
    drop(reader);
    drop(pool);
    drop(db);
    let _ = std::fs::remove_file(&path);

    assert_eq!(reused, 1, "precondition: a pool of one lends the same connection back");
    assert_eq!(fresh, 0, "a discarded connection must not be lent again");
}
