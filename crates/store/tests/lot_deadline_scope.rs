//! Issue 389 unit 2: a lot returned by `status=open` must carry the deadline that
//! decided it.
//!
//! The `status` filter on `/v1/lots` runs `version_predicates`, whose EXISTS has
//! no `lot_id` term — a procedure-scoped deadline opens every lot of the
//! procedure, and issue 275 pins that form deliberately on 273's status ≡
//! head-range equivalence. The ROW's `submission_deadline` read lot-scoped rows
//! only. The two readings of one stored fact met on prod: all 73 open-with-lots
//! tenders over ids 8,436,069–8,536,069 (source `ted`, r209 era, whose
//! `DATE_RECEIPT_TENDERS` is procedure-level by design) were returned as OPEN
//! serving `submission_deadline: null` — tender 8436333 six times over, in the
//! same response whose tender row read `2029-04-29T10:00:00+00:00`.
//!
//! What this file pins is the PAIR, because the two can only drift apart when a
//! test asserts one without the other: what `status=open` returns AND what the
//! returned rows say their deadline is.

use store::read::{self, Filter, Scope, Status};
use store::turso::{self, Value};

const TENDER: i64 = 1;
/// A fixed "now" — the filter takes it as a parameter rather than reading the
/// clock, so the test is not time-dependent.
const NOW: i64 = 1_789_344_000; // 2026-09-14T00:00Z
const TENDER_DEADLINE: i64 = 1_871_000_000; // well past NOW
const LOT_DEADLINE: i64 = 1_860_000_000; // EARLIER than the tender's, on purpose

async fn drain(conn: &turso::Connection, sql: &str) {
    let mut rows = conn.query(sql, ()).await.unwrap();
    while rows.next().await.unwrap().is_some() {}
}

async fn fixture(name: &str) -> turso::Connection {
    let path = format!("/tmp/tender-db-lotdeadline-{name}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    store::Db::open(&path).await.unwrap();
    let db = turso::Builder::new_local(&path).build().await.unwrap();
    let conn = db.connect().unwrap();
    drain(&conn, "PRAGMA journal_mode = WAL").await;
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    conn.execute(
        "INSERT INTO tenders (id, source, procedure_key, kind, current_seq, current_published_at, created_at, current_deadline)
         VALUES (?, 'ted', 'pk', 'procedure', 1, 1700000000, 1700000000, ?)",
        (Value::Integer(TENDER), Value::Integer(TENDER_DEADLINE)),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO tender_versions (tender_id, seq, published_at, publication_id, caused_by_notice_id)
         VALUES (?, 1, 1700000000, 'pub', 1)",
        (Value::Integer(TENDER),),
    )
    .await
    .unwrap();
    conn
}

async fn lot(conn: &turso::Connection, id: i64, key: &str) {
    conn.execute(
        "INSERT INTO lots (id, tender_id, lot_key) VALUES (?, ?, ?)",
        (Value::Integer(id), Value::Integer(TENDER), Value::Text(key.to_owned())),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO tender_version_lots (tender_id, seq, lot_id, kind) VALUES (?, 1, ?, 'Lot')",
        (Value::Integer(TENDER), Value::Integer(id)),
    )
    .await
    .unwrap();
}

/// A deadline as the projection stores it. `lot` is `None` for the
/// procedure-scoped row the r209 era publishes.
async fn deadline(conn: &turso::Connection, lot: Option<i64>, utc: i64, offset: i64) {
    conn.execute(
        "INSERT INTO tender_version_dates (tender_id, seq, lot_id, field, utc_seconds, offset_minutes, has_time)
         VALUES (?, 1, ?, 'submission_deadline', ?, ?, 1)",
        (
            Value::Integer(TENDER),
            lot.map_or(Value::Null, Value::Integer),
            Value::Integer(utc),
            Value::Integer(offset),
        ),
    )
    .await
    .unwrap();
}

async fn served(conn: &turso::Connection, status: Option<Status>) -> Vec<(String, Option<i64>)> {
    let filter =
        Filter { tender: Some(TENDER), status, now: NOW, ..Filter::default() };
    read::lots(conn, &filter, Scope::Page { after: 0, limit: 1000 })
        .await
        .unwrap()
        .into_iter()
        .map(|r| (r.lot_key, r.deadline.map(|d| d.utc_seconds)))
        .collect()
}

#[tokio::test]
async fn an_open_lot_carries_the_deadline_that_opened_it_whatever_its_scope() {
    let conn = fixture("open").await;

    // The r209 shape: ONE procedure-scoped deadline, lots that publish none.
    lot(&conn, 1, "LOT-1").await;
    lot(&conn, 2, "LOT-2").await;
    deadline(&conn, None, TENDER_DEADLINE, 0).await;

    // And one lot that DOES publish its own — EARLIER than the tender's, so the
    // assertion is about scope and not about which number is larger. Upward
    // inheritance takes MAX; downward inheritance must not, or a lot with a real
    // deadline of its own would be overwritten by the procedure's later one.
    lot(&conn, 3, "LOT-3").await;
    deadline(&conn, Some(3), LOT_DEADLINE, 120).await;

    let all = served(&conn, None).await;
    assert_eq!(
        all,
        vec![
            ("LOT-1".to_owned(), Some(TENDER_DEADLINE)),
            ("LOT-2".to_owned(), Some(TENDER_DEADLINE)),
            ("LOT-3".to_owned(), Some(LOT_DEADLINE)),
        ],
        "undated lots inherit the procedure's date; a lot's own date wins even when earlier"
    );

    // The PAIR: what the filter returns and what the rows say must agree. Before
    // this, the filter returned all three and two of them said `null`.
    let open = served(&conn, Some(Status::Open)).await;
    assert_eq!(open.len(), 3, "the procedure-scoped deadline opens every lot (issue 275)");
    for (key, deadline) in &open {
        assert!(
            deadline.is_some(),
            "{key} was RETURNED as open and must show the deadline that decided it"
        );
    }
    assert!(
        served(&conn, Some(Status::Closed)).await.is_empty(),
        "and the other arm is still consistent with the filter's own reading"
    );
}

/// The offset travels with the date, so an inherited deadline renders in the
/// buyer's published wall-clock rather than silently in UTC (CONTEXT.md).
#[tokio::test]
async fn an_inherited_deadline_keeps_the_published_offset() {
    let conn = fixture("offset").await;
    lot(&conn, 1, "LOT-1").await;
    deadline(&conn, None, TENDER_DEADLINE, -300).await;

    let rows = read::lots(
        &conn,
        &Filter { tender: Some(TENDER), now: NOW, ..Filter::default() },
        Scope::Page { after: 0, limit: 1000 },
    )
    .await
    .unwrap();
    let stamp = rows[0].deadline.expect("the lot inherits the procedure's deadline");
    assert_eq!(stamp.utc_seconds, TENDER_DEADLINE);
    assert_eq!(stamp.offset_minutes, -300, "the publisher's offset, not 0");
    assert!(stamp.has_time);
}

/// A tender that publishes no deadline at all still serves `null` — the fallback
/// adds an inherited fact, it does not invent one.
#[tokio::test]
async fn a_lot_of_a_deadline_less_tender_still_has_none() {
    let conn = fixture("none").await;
    lot(&conn, 1, "LOT-1").await;

    assert_eq!(served(&conn, None).await, vec![("LOT-1".to_owned(), None)]);
    assert!(
        served(&conn, Some(Status::Open)).await.is_empty(),
        "no deadline anywhere is not open — the filter and the row agree on that too"
    );
}
