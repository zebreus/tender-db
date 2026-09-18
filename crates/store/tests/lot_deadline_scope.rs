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

use store::read::{self, DeadlineScope, Filter, Scope, Status};
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

/// Issue 370 unit 4, the half issue 389 handed over: the row SAYS which scope its
/// deadline came from. On the lots, `procedure` for the two that inherit and `lot`
/// for the one that published its own; on the tender, the elected deadline's own
/// scope — the procedure's here, because it is the newest of the two.
#[tokio::test]
async fn the_row_says_which_scope_its_deadline_came_from() {
    let conn = fixture("scope").await;
    lot(&conn, 1, "LOT-1").await;
    lot(&conn, 2, "LOT-2").await;
    deadline(&conn, None, TENDER_DEADLINE, 0).await;
    lot(&conn, 3, "LOT-3").await;
    deadline(&conn, Some(3), LOT_DEADLINE, 120).await;

    let filter = Filter { tender: Some(TENDER), now: NOW, ..Filter::default() };
    let lots = read::lots(&conn, &filter, Scope::Page { after: 0, limit: 1000 }).await.unwrap();
    let scopes: Vec<(String, Option<DeadlineScope>)> =
        lots.into_iter().map(|r| (r.lot_key, r.deadline_scope)).collect();
    assert_eq!(
        scopes,
        vec![
            ("LOT-1".to_owned(), Some(DeadlineScope::Procedure)),
            ("LOT-2".to_owned(), Some(DeadlineScope::Procedure)),
            ("LOT-3".to_owned(), Some(DeadlineScope::Lot)),
        ]
    );

    let tenders = read::tenders(&conn, &filter, Scope::Page { after: 0, limit: 10 }).await.unwrap();
    assert_eq!(tenders.len(), 1);
    assert_eq!(tenders[0].deadline.map(|d| d.utc_seconds), Some(TENDER_DEADLINE));
    assert_eq!(tenders[0].deadline_scope, Some(DeadlineScope::Procedure));
}

/// The tender side of the same marker: when the newest deadline is a LOT's, the
/// tender's elected deadline is that lot-level date and the row says `lot`.
#[tokio::test]
async fn a_tender_whose_newest_deadline_is_a_lots_says_lot() {
    let conn = fixture("scope-lot").await;
    lot(&conn, 1, "LOT-1").await;
    deadline(&conn, None, LOT_DEADLINE, 0).await;
    deadline(&conn, Some(1), TENDER_DEADLINE, 60).await;

    let filter = Filter { tender: Some(TENDER), now: NOW, ..Filter::default() };
    let tenders = read::tenders(&conn, &filter, Scope::Page { after: 0, limit: 10 }).await.unwrap();
    assert_eq!(tenders[0].deadline.map(|d| (d.utc_seconds, d.offset_minutes)), Some((TENDER_DEADLINE, 60)));
    assert_eq!(tenders[0].deadline_scope, Some(DeadlineScope::Lot));
    // The lot itself published it: `lot`, not inherited.
    let lots = read::lots(&conn, &filter, Scope::Page { after: 0, limit: 10 }).await.unwrap();
    assert_eq!(lots[0].deadline_scope, Some(DeadlineScope::Lot));
}

/// No deadline anywhere: no scope either — the marker never outruns the date.
#[tokio::test]
async fn no_deadline_means_no_scope() {
    let conn = fixture("scope-none").await;
    lot(&conn, 1, "LOT-1").await;
    let filter = Filter { tender: Some(TENDER), now: NOW, ..Filter::default() };
    let lots = read::lots(&conn, &filter, Scope::Page { after: 0, limit: 10 }).await.unwrap();
    assert_eq!((lots[0].deadline, lots[0].deadline_scope), (None, None));
    let tenders = read::tenders(&conn, &filter, Scope::Page { after: 0, limit: 10 }).await.unwrap();
    assert_eq!((tenders[0].deadline, tenders[0].deadline_scope), (None, None));
}
