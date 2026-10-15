//! Issue 88 follow-up: the FIELD-scoped refold cohort. The load-bearing
//! properties: the sweep finds exactly the notices whose value layer carries a
//! requested field id (either channel, deduped across both), the by-ids requeue
//! touches only parsed+projected rows, and the notice-cohort stamp ages exactly
//! the tenders those notices caused — the issue-179 pair, field-scoped.

use store::turso::{self, Value};

async fn open(name: &str) -> (store::Db, turso::Connection) {
    let path = format!("/tmp/tender-db-refoldf-{name}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(&path).await.unwrap();
    let raw = turso::Builder::new_local(&path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    (db, conn)
}

async fn notice(conn: &turso::Connection, id: i64, projected: i64) {
    conn.execute(
        "INSERT INTO notices (id, source, publication_id, content_hash, profile, fetch_id,
                              member_path, ingested_at, parse_state, projected)
         VALUES (?, 'ted', ?, ?, 'eforms:eforms-sdk-1.7', 1, 'p.xml', 0, 'parsed', ?)",
        (
            Value::Integer(id),
            Value::Text(format!("pub-{id}")),
            Value::Text(format!("h-{id}")),
            Value::Integer(projected),
        ),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn the_field_sweep_finds_exactly_the_carriers_and_the_pair_scopes_to_them() {
    let (db, conn) = open("sweep").await;
    // Four notices: 1 carries the amount id, 2 carries the text id, 3 carries
    // BOTH (must not double-count), 4 carries an unrelated id.
    for id in 1..=4i64 {
        notice(&conn, id, 1).await;
    }
    for (nid, fid) in [(1, "UBL-FrameworkMaximumAmount"), (3, "UBL-FrameworkMaximumAmount")] {
        conn.execute(
            "INSERT INTO notice_amounts (notice_id, section_id, field_id, ordinal, cents, currency)
             VALUES (?, 'PROC', ?, 0, 100, 'EUR')",
            (Value::Integer(nid), Value::Text(fid.into())),
        )
        .await
        .unwrap();
    }
    for (nid, fid) in [(2, "UBL-FundingProgram"), (3, "UBL-FundingProgram"), (4, "BT-21")] {
        conn.execute(
            "INSERT INTO notice_texts (notice_id, section_id, field_id, ordinal, lang, value)
             VALUES (?, 'PROC', ?, 0, 'ENG', 'x')",
            (Value::Integer(nid), Value::Text(fid.into())),
        )
        .await
        .unwrap();
    }
    // Tenders: notice 1 caused tender 10, notice 3 caused tender 30; notice 2 is
    // unfolded (no version row) — the stamp must survive carriers with no tender.
    for (tid, nid) in [(10i64, 1i64), (30, 3)] {
        // Explicitly CURRENT epoch — the column defaults to 0 (the stale value),
        // which would make the stamp assertion below pass vacuously.
        conn.execute(
            "INSERT INTO tenders (id, source, procedure_key, kind, current_seq, created_at, projection_epoch)
             VALUES (?, 'ted', ?, 'procedure', 1, 0, ?)",
            (
                Value::Integer(tid),
                Value::Text(format!("pk-{tid}")),
                Value::Integer(store::canonical::PROJECTION_EPOCH),
            ),
        )
        .await
        .unwrap();
        conn.execute(
            "INSERT INTO tender_versions (tender_id, seq, published_at, publication_id, caused_by_notice_id)
             VALUES (?, 1, 1, ?, ?)",
            (Value::Integer(tid), Value::Text(format!("pub-{nid}")), Value::Integer(nid)),
        )
        .await
        .unwrap();
    }

    let carriers = db
        .notice_ids_carrying_fields(&["UBL-FrameworkMaximumAmount", "UBL-FundingProgram"], None, &|| false)
        .await
        .unwrap()
        .ids;
    assert_eq!(carriers, vec![1, 2, 3], "both channels, deduped, the non-carrier excluded");

    let requeued = db.unmark_projected_by_ids(&carriers).await.unwrap();
    assert_eq!(requeued, 3);
    // Idempotent: already re-queued rows are not re-counted.
    assert_eq!(db.unmark_projected_by_ids(&carriers).await.unwrap(), 0);

    let stamped = db.stamp_stale_for_notices(&carriers).await.unwrap();
    assert_eq!(stamped, 2, "the two tenders the carriers caused; the unfolded carrier is fine");
    let mut rows = conn
        .query("SELECT id FROM tenders WHERE projection_epoch = 0 ORDER BY id", ())
        .await
        .unwrap();
    let mut stale = Vec::new();
    while let Some(row) = rows.next().await.unwrap() {
        stale.push(row.get_value(0).unwrap().as_integer().copied().unwrap());
    }
    assert_eq!(stale, vec![10, 30], "exactly the carriers' tenders are aged");

    // An id nothing carries sweeps to an empty cohort.
    assert!(db.notice_ids_carrying_fields(&["UBL-NoSuchThing"], None, &|| false).await.unwrap().ids.is_empty());
}

/// Issue 543 unit 3: the sweep reads its stop before every window and, stopped, says where:
/// the table and window it did not read, and only what the finished windows found. A stop
/// before the first window reads nothing at all. The sweep is read-only, so either way
/// nothing is written.
#[tokio::test]
async fn the_field_sweep_stops_before_its_next_window_and_names_where() {
    let (db, conn) = open("stop").await;
    for id in 1..=2i64 {
        notice(&conn, id, 1).await;
    }
    conn.execute(
        "INSERT INTO notice_amounts (notice_id, section_id, field_id, ordinal, cents, currency)
         VALUES (1, 'PROC', 'UBL-FrameworkMaximumAmount', 0, 100, 'EUR')",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO notice_texts (notice_id, section_id, field_id, ordinal, lang, value)
         VALUES (2, 'PROC', 'UBL-FundingProgram', 0, 'ENG', 'x')",
        (),
    )
    .await
    .unwrap();
    let fields = ["UBL-FrameworkMaximumAmount", "UBL-FundingProgram"];
    let tables = ["notice_amounts", "notice_texts"];

    // Asked to stop before anything: no window read, no carrier.
    let at_once = db.notice_ids_carrying_fields(&fields, Some(&tables), &|| true).await.unwrap();
    assert!(at_once.ids.is_empty(), "{at_once:?}");
    assert_eq!(
        at_once.stopped,
        Some(store::CarrierSweepStop { table: "notice_amounts".into(), table_no: 1, tables: 2, at: 0, max_id: 2 })
    );

    // One window per table on this corpus: the stop lands before the second table's, so
    // the first table's carrier is the only one found.
    let checks = std::sync::atomic::AtomicUsize::new(0);
    let second = || checks.fetch_add(1, std::sync::atomic::Ordering::Relaxed) >= 1;
    let part = db.notice_ids_carrying_fields(&fields, Some(&tables), &second).await.unwrap();
    assert_eq!(part.ids, vec![1]);
    assert_eq!(
        part.stopped,
        Some(store::CarrierSweepStop { table: "notice_texts".into(), table_no: 2, tables: 2, at: 0, max_id: 2 })
    );

    // Not stopped: both carriers, and no stop recorded.
    let whole = db.notice_ids_carrying_fields(&fields, Some(&tables), &|| false).await.unwrap();
    assert_eq!(whole, store::CarrierSweep { ids: vec![1, 2], stopped: None });
    assert_eq!(
        conn.query("SELECT COUNT(*) FROM notices WHERE projected = 1", ())
            .await
            .unwrap()
            .next()
            .await
            .unwrap()
            .unwrap()
            .get_value(0)
            .unwrap(),
        Value::Integer(2),
        "a sweep writes nothing"
    );
}
