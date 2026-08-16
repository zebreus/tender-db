//! Issue 216 (deadline half): `backfill_current_deadline` stamps each tender's
//! head-version submission deadline — the SAME value the fold's `head_deadline`
//! writes for new heads and the list row's `pick` reads back, so all three answers
//! agree. Constructed rows cover the cases a fixture would not reliably supply:
//! the MAX over several deadline rows, a LOT-level deadline, a non-head version
//! whose (newer) deadline must NOT win, a deadline-less tender staying NULL, and
//! the batch walk being restartable + idempotent.

use store::turso::{self, Value};

async fn open(name: &str) -> (store::Db, turso::Connection) {
    let path = format!("/tmp/tender-db-dlbf-{name}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(&path).await.unwrap();
    let raw = turso::Builder::new_local(&path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    (db, conn)
}

async fn tender(conn: &turso::Connection, id: i64, current_seq: i64) {
    conn.execute(
        "INSERT INTO tenders (id, source, procedure_key, kind, current_seq, current_published_at, created_at)
         VALUES (?, 'ted', ?, 'procedure', ?, 100, 0)",
        (Value::Integer(id), Value::Text(format!("pk-{id}")), Value::Integer(current_seq)),
    )
    .await
    .unwrap();
}

async fn date(conn: &turso::Connection, tender: i64, seq: i64, lot: Option<i64>, utc: i64) {
    conn.execute(
        "INSERT INTO tender_version_dates (tender_id, seq, lot_id, field, utc_seconds, offset_minutes, has_time)
         VALUES (?, ?, ?, 'submission_deadline', ?, 0, 1)",
        (
            Value::Integer(tender),
            Value::Integer(seq),
            lot.map(Value::Integer).unwrap_or(Value::Null),
            Value::Integer(utc),
        ),
    )
    .await
    .unwrap();
}

async fn stamped(conn: &turso::Connection, id: i64) -> Option<i64> {
    let mut rows = conn
        .query("SELECT current_deadline FROM tenders WHERE id = ?", [Value::Integer(id)])
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    row.get_value(0).unwrap().as_integer().copied()
}

#[tokio::test]
async fn the_backfill_stamps_head_deadlines_in_batches() {
    let (db, conn) = open("stamp").await;

    // 1: two tender-level deadline rows on the head — the MAX (7000) wins.
    tender(&conn, 1, 2).await;
    date(&conn, 1, 2, None, 5000).await;
    date(&conn, 1, 2, None, 7000).await;
    // ...and an OLD version with a LATER date that must not leak into the head.
    date(&conn, 1, 1, None, 9999).await;

    // 2: the deadline lives on a LOT row of the head version.
    tender(&conn, 2, 1).await;
    date(&conn, 2, 1, Some(77), 6000).await;

    // 3: no deadline anywhere → stays NULL.
    tender(&conn, 3, 1).await;

    // 4: only a non-head version has one → head has none → NULL.
    tender(&conn, 4, 3).await;
    date(&conn, 4, 2, None, 4000).await;

    // Walk with batch=2 so the walk needs several transactions.
    let mut after = 0;
    let mut total = 0;
    loop {
        let (rows, next) = db.backfill_current_deadline(2, after).await.unwrap();
        if rows == 0 {
            break;
        }
        total += rows;
        after = next;
    }
    assert_eq!(total, 4, "every tender is visited exactly once");

    assert_eq!(stamped(&conn, 1).await, Some(7000), "MAX over the head's rows");
    assert_eq!(stamped(&conn, 2).await, Some(6000), "a lot-level deadline counts");
    assert_eq!(stamped(&conn, 3).await, None, "no deadline stays NULL");
    assert_eq!(stamped(&conn, 4).await, None, "a non-head deadline must not win");

    // Idempotent: a second full walk recomputes identical values.
    let mut after = 0;
    loop {
        let (rows, next) = db.backfill_current_deadline(3, after).await.unwrap();
        if rows == 0 {
            break;
        }
        after = next;
    }
    assert_eq!(stamped(&conn, 1).await, Some(7000));
    assert_eq!(stamped(&conn, 4).await, None);
}
