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
        .notice_ids_carrying_fields(&["UBL-FrameworkMaximumAmount", "UBL-FundingProgram"], None)
        .await
        .unwrap();
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
    assert!(db.notice_ids_carrying_fields(&["UBL-NoSuchThing"], None).await.unwrap().is_empty());
}
