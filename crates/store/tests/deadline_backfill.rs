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

/// Issue 375: a deadline BEYOND the horizon must not be stamped.
///
/// The test above cannot catch this and never could: its dates sit at 5000/7000
/// seconds against `current_published_at = 100`, comfortably inside ten years,
/// so the filtered and unfiltered rules agree on every input it has. That is the
/// general shape worth remembering — **a test pinning agreement between two
/// implementations only pins it where they were already going to agree**, and
/// discriminating inputs have to be chosen on purpose.
///
/// The value it guards is not hypothetical: the backfill shipped without the
/// horizon `head_deadline` grew in `aa732c5`, so running it re-stamped tender
/// 3323836's head deadline back to 3005-07-06 and returned it to `status=open`,
/// undoing issue 366's drain.
#[tokio::test]
async fn the_backfill_refuses_a_deadline_beyond_the_horizon() {
    let (db, conn) = open("horizon").await;

    // `tender()` publishes at 100, so the horizon ends at 100 + ten years.
    let horizon = store::canonical::DEADLINE_HORIZON_SECS;
    let plausible = 100 + 30 * 86_400;
    let millennium = 100 + horizon + 86_400;

    // 1: the 3323836 shape — ONE version publishing both, MAX would take the typo.
    tender(&conn, 1, 1).await;
    date(&conn, 1, 1, None, plausible).await;
    date(&conn, 1, 1, None, millennium).await;

    // 2: the typo is the ONLY deadline, so there is nothing to fall back to.
    tender(&conn, 2, 1).await;
    date(&conn, 2, 1, None, millennium).await;

    // 3: exactly ON the horizon is admitted — the election compares `<=`, and a
    //    boundary that drifts between the two implementations is the whole risk.
    tender(&conn, 3, 1).await;
    date(&conn, 3, 1, None, 100 + horizon).await;

    let mut after = 0;
    loop {
        let (rows, next) = db.backfill_current_deadline(10, after).await.unwrap();
        if rows == 0 {
            break;
        }
        after = next;
    }

    assert_eq!(
        stamped(&conn, 1).await,
        Some(plausible),
        "the real date wins over a thousand-year typo — the 3323836 case"
    );
    assert_eq!(
        stamped(&conn, 2).await,
        None,
        "a tender whose only deadline is beyond the horizon has none, not a typo"
    );
    assert_eq!(
        stamped(&conn, 3).await,
        Some(100 + horizon),
        "the horizon is inclusive, the same way head_deadline compares it"
    );
}
