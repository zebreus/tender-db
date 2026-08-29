//! Issue 300 Stage 4 Unit 2: the org_match_keys scratch satellite and the
//! org_candidate_edges store — STRICT + CHECK enforcement, the index-free
//! open (no index in the schema batch), and the reset path's idempotence.

use store::turso::Value;

async fn count(conn: &store::turso::Connection, sql: &str) -> i64 {
    let mut rows = conn.query(sql, ()).await.unwrap();
    let row = rows.next().await.unwrap().unwrap();
    let Value::Integer(n) = row.get_value(0).unwrap() else { panic!("count") };
    n
}

#[tokio::test]
async fn stage4_tables_enforce_their_contracts_and_reset_cleanly() {
    let path = "test-stage4-schema.db";
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(path).await.unwrap();
    let raw = store::turso::Builder::new_local(path).build().await.unwrap();
    let conn = raw.connect().unwrap();

    // The load table takes rows with any of the three key kinds…
    for kind in ["n2", "n3", "n3s"] {
        conn.execute(
            "INSERT INTO org_match_keys (org_id, key_kind, key) VALUES (1, ?, 'alfa §sro')",
            (Value::Text(kind.into()),),
        )
        .await
        .unwrap();
    }
    // …and rejects an unknown kind (the CHECK ships 'n3s' from day one so
    // the constraint never needs DDL churn).
    assert!(
        conn.execute("INSERT INTO org_match_keys (org_id, key_kind, key) VALUES (1, 'n9', 'x')", ())
            .await
            .is_err(),
        "unknown key_kind must fail the CHECK"
    );
    // No index rides the schema batch — the build job creates it post-load.
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND tbl_name='org_match_keys'").await,
        0,
        "the load table opens index-free"
    );

    // Edges: ordered pair enforced, duplicate (a,b,rule) upserts collide.
    conn.execute(
        "INSERT INTO org_candidate_edges (org_a, org_b, rule, tier, score, evidence, first_seen, last_seen)
         VALUES (10, 20, 'e3-name', 'E3', 1.0, '{}', 0, 0)",
        (),
    )
    .await
    .unwrap();
    assert!(
        conn.execute(
            "INSERT INTO org_candidate_edges (org_a, org_b, rule, tier, score, evidence, first_seen, last_seen)
             VALUES (20, 10, 'e3-name', 'E3', 1.0, '{}', 0, 0)",
            (),
        )
        .await
        .is_err(),
        "org_a < org_b is a CHECK, reversed pairs must fail"
    );
    assert!(
        conn.execute(
            "INSERT INTO org_candidate_edges (org_a, org_b, rule, tier, score, evidence, first_seen, last_seen)
             VALUES (10, 20, 'e3-name', 'E3', 2.0, '{}', 1, 1)",
            (),
        )
        .await
        .is_err(),
        "the (a, b, rule) PK holds"
    );

    // The reset empties the keys at O(1), zeroes the watermark, and leaves
    // the edge store standing; running it twice is a no-op.
    conn.execute("UPDATE projection_state SET org_match_keys_watermark = 42 WHERE id = 0", ())
        .await
        .unwrap();
    db.reset_org_match_keys().await.unwrap();
    db.reset_org_match_keys().await.unwrap();
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM org_match_keys").await, 0);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM org_candidate_edges").await, 1);
    assert_eq!(
        count(&conn, "SELECT org_match_keys_watermark FROM projection_state WHERE id = 0").await,
        0,
        "the reset clears the build watermark"
    );
    // The recreated table still enforces its CHECK (DDL replayed intact).
    assert!(
        conn.execute("INSERT INTO org_match_keys (org_id, key_kind, key) VALUES (1, 'n9', 'x')", ())
            .await
            .is_err()
    );

    // Reopening an existing file takes the IF NOT EXISTS path cleanly.
    drop(conn);
    drop(db);
    let db2 = store::Db::open(path).await.unwrap();
    drop(db2);
}
