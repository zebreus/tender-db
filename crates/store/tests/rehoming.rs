//! Issue 317 Unit A: moving a reviewed mention to the row it actually
//! describes. The 311 campaign found consortium-vehicle rows holding their
//! member's SOLO mentions; this is the repair, and its guards matter more
//! than its happy path — it moves entity references.

use store::turso::Value;
use store::RehomingVerdict;

async fn count(conn: &store::turso::Connection, sql: &str) -> i64 {
    let mut rows = conn.query(sql, ()).await.unwrap();
    let row = rows.next().await.unwrap().unwrap();
    let Value::Integer(n) = row.get_value(0).unwrap() else { panic!("count") };
    n
}

fn verdict(org: i64, notice: i64, target: Option<i64>, conf: &str) -> RehomingVerdict {
    RehomingVerdict {
        case_org_id: org,
        notice_id: notice,
        section_id: "ORG-1".into(),
        action: "rehome".into(),
        target_org_id: target,
        target_name: Some("Dobler GmbH".into()),
        rationale: "the notice names the member alone".into(),
        confidence: conf.into(),
    }
}

#[tokio::test]
async fn a_reviewed_mention_moves_with_its_derived_rows_and_nothing_else_does() {
    let path = "test-rehoming.db";
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(path).await.unwrap();
    let raw = store::turso::Builder::new_local(path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    // 1 = the vehicle holding everything; 2 = the member it should go to.
    for (id, name) in [(1i64, "Bietergemeinschaft Dobler / Oberall"), (2, "Dobler GmbH")] {
        conn.execute(
            "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
             VALUES (?, 'DE', NULL, NULL, ?, ?, 0, 0)",
            (Value::Integer(id), Value::Text(name.into()), Value::Text(name.to_lowercase())),
        )
        .await
        .unwrap();
    }
    // Four mentions on the vehicle. 900 is the one under review and carries a
    // full derived stack: party + bid-party + winner. 901 gets a MEDIUM
    // verdict, 902 a `keep`, 903 a verdict naming a target that does not
    // exist — none of those may move.
    for notice in [900i64, 901, 902, 903] {
        conn.execute(
            "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, country, raw_identifier)
             VALUES (?, 'ORG-1', 1, 'Dobler GmbH', 'DE', NULL)",
            (Value::Integer(notice),),
        )
        .await
        .unwrap();
    }
    // The FK targets the wet path needs: the apply runs with foreign keys ON
    // (only this test connection has them off), so a fixture that skips them
    // passes the seed and fails the move.
    conn.execute(
        "INSERT INTO fetches (id, source, kind, period, url, sha256, bytes, fetched_at, path)
         VALUES (1, 'ted', 'daily', 'p', 'u', 'aa', 1, 0, 'p')",
        (),
    )
    .await
    .unwrap();
    for notice in [900i64, 901, 902, 903] {
        conn.execute(
            "INSERT INTO notices (id, source, publication_id, content_hash, profile, fetch_id, member_path, ingested_at)
             VALUES (?, 'ted', 'pub-' || ?, 'h', 'eforms', 1, 'm', 0)",
            (Value::Integer(notice), Value::Integer(notice)),
        )
        .await
        .unwrap();
    }
    conn.execute(
        "INSERT INTO tenders (id, source, kind, created_at) VALUES (7, 'ted', 'procedure', 0)",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO tender_versions (tender_id, seq, caused_by_notice_id, publication_id, published_at)
         VALUES (7, 1, 900, 'pub-7-1', 0)",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO lot_results (id, tender_id, notice_id, result_key) VALUES (11, 7, 900, 'RES-1')",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO bids (id, tender_id, notice_id, bid_key) VALUES (3, 7, 900, 'TEN-1')",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO tender_version_parties (tender_id, seq, role, organization_id, mention_notice_id, mention_section_id)
         VALUES (7, 1, 'winner', 1, 900, 'ORG-1')",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO tender_version_bid_parties (tender_id, seq, bid_id, role, organization_id, mention_notice_id, mention_section_id)
         VALUES (7, 1, 3, 'tenderer', 1, 900, 'ORG-1')",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO tender_version_result_winners (tender_id, seq, lot_result_id, organization_id)
         VALUES (7, 1, 11, 1)",
        (),
    )
    .await
    .unwrap();

    db.record_rehoming(
        "fusion",
        &[
            verdict(1, 900, Some(2), "high"),   // the one that moves
            verdict(1, 901, Some(2), "medium"), // not high: parked
            RehomingVerdict { action: "keep".into(), ..verdict(1, 902, Some(2), "high") },
            verdict(1, 903, Some(999), "high"), // target does not exist
        ],
        10,
    )
    .await
    .unwrap();

    // DRY: the concrete move list, nothing written.
    let dry = db.apply_rehoming(true, Some(1), 20).await.unwrap();
    assert_eq!(
        (dry.pending, dry.eligible),
        (4, 2),
        "eligible is a claim about the VERDICT — rehome, high, names a target — so the \
         medium and the `keep` are out while the one naming a non-existent org is IN, \
         and gets rejected further down where the data is checked"
    );
    assert_eq!(dry.moved, 1);
    assert_eq!(dry.missing_target, 1, "the verdict naming org 999 is counted, never invented");
    assert_eq!(
        dry.plan,
        vec![(900, "ORG-1".to_owned(), 1, 2, "Dobler GmbH".to_owned())],
        "the plan names the destination — the thing counts alone cannot show to be wrong"
    );
    assert_eq!(count(&conn, "SELECT organization_id FROM organization_mentions WHERE notice_id = 900").await, 1);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM changes").await, 0, "a dry run is silent");

    // WET.
    let wet = db.apply_rehoming(false, Some(2), 30).await.unwrap();
    assert_eq!((wet.moved, wet.parties, wet.bid_parties, wet.winners), (1, 1, 1, 1));
    assert_eq!(wet.tenders, 1);
    assert_eq!(
        count(&conn, "SELECT organization_id FROM organization_mentions WHERE notice_id = 900").await,
        2,
        "the mention moved to the member"
    );
    for (table, sql) in [
        ("party", "SELECT organization_id FROM tender_version_parties WHERE tender_id = 7"),
        ("bid party", "SELECT organization_id FROM tender_version_bid_parties WHERE tender_id = 7"),
        ("winner", "SELECT organization_id FROM tender_version_result_winners WHERE tender_id = 7"),
    ] {
        assert_eq!(count(&conn, sql).await, 2, "the {table} row followed the mention");
    }
    // The other three mentions stand exactly where they were.
    for notice in [901i64, 902, 903] {
        assert_eq!(
            count(&conn, &format!("SELECT organization_id FROM organization_mentions WHERE notice_id = {notice}")).await,
            1,
            "mention {notice} must not move"
        );
    }
    // Nothing was merged or deleted: this repair moves references, never rows.
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations").await, 2);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM org_merge_log").await, 0);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organization_mentions").await, 4);

    // The pre-image is on the row, so the move is reversible from it alone.
    let mut rows = conn
        .query("SELECT applied_action FROM org_mention_rehoming WHERE notice_id = 900", ())
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    let Value::Text(action) = row.get_value(0).unwrap() else { panic!("text") };
    assert_eq!(action, "rehomed from 1");
    drop(rows);

    // Change events: both orgs and the tender.
    for (kind, id) in [("organization", 1i64), ("organization", 2), ("tender", 7)] {
        assert_eq!(
            count(&conn, &format!(
                "SELECT COUNT(*) FROM changes WHERE entity_kind = '{kind}' AND entity_id = {id} AND op = 'changed'"
            )).await,
            1,
            "{kind} {id} changed"
        );
    }

    // Idempotent: the applied verdict has left the pending set.
    let again = db.apply_rehoming(false, Some(3), 40).await.unwrap();
    assert_eq!((again.pending, again.moved), (3, 0));
    assert_eq!(again.missing_target, 1, "the missing-target verdict stays pending — it is fixable");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM changes").await, 3, "no second wave");
}

/// A mention that moved since its verdict was recorded is a NO-OP, stamped so
/// it leaves the pending set instead of being retried forever — the guard the
/// issue-311/312 apply paths learned the hard way.
#[tokio::test]
async fn a_mention_that_moved_since_the_review_is_a_stamped_noop() {
    let path = "test-rehoming-noop.db";
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(path).await.unwrap();
    let raw = store::turso::Builder::new_local(path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    for id in [1i64, 2, 3] {
        conn.execute(
            "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
             VALUES (?, 'DE', NULL, NULL, 'n', 'n', 0, 0)",
            (Value::Integer(id),),
        )
        .await
        .unwrap();
    }
    // The verdict says the mention is on org 1; a merge has since moved it
    // to org 3.
    conn.execute(
        "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, country, raw_identifier)
         VALUES (900, 'ORG-1', 3, 'x', NULL, NULL)",
        (),
    )
    .await
    .unwrap();
    db.record_rehoming("fusion", &[verdict(1, 900, Some(2), "high")], 10).await.unwrap();

    let r = db.apply_rehoming(false, Some(1), 20).await.unwrap();
    assert_eq!((r.eligible, r.moved, r.noop), (1, 0, 1));
    assert_eq!(
        count(&conn, "SELECT organization_id FROM organization_mentions WHERE notice_id = 900").await,
        3,
        "the mention stays where it actually is — the verdict's premise is stale"
    );
    // Stamped, so it does not come back forever.
    let again = db.apply_rehoming(false, Some(2), 30).await.unwrap();
    assert_eq!((again.pending, again.noop), (0, 0));
    let mut rows = conn
        .query("SELECT applied_action FROM org_mention_rehoming WHERE notice_id = 900", ())
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    let Value::Text(action) = row.get_value(0).unwrap() else { panic!("text") };
    assert!(action.starts_with("no-op:"), "{action}");
}

/// Re-recording a verdict replaces the judgement but NEVER the applied stamp
/// or its pre-image: once a mention has moved, that row is the only way back.
#[tokio::test]
async fn re_recording_never_erases_an_applied_stamp() {
    let path = "test-rehoming-restamp.db";
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(path).await.unwrap();
    let raw = store::turso::Builder::new_local(path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    for id in [1i64, 2] {
        conn.execute(
            "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
             VALUES (?, 'DE', NULL, NULL, 'n', 'n', 0, 0)",
            (Value::Integer(id),),
        )
        .await
        .unwrap();
    }
    conn.execute(
        "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, country, raw_identifier)
         VALUES (900, 'ORG-1', 1, 'x', NULL, NULL)",
        (),
    )
    .await
    .unwrap();
    db.record_rehoming("fusion", &[verdict(1, 900, Some(2), "high")], 10).await.unwrap();
    db.apply_rehoming(false, Some(1), 20).await.unwrap();

    // A second review of the same mention, with a different rationale.
    db.record_rehoming(
        "fusion-round-2",
        &[RehomingVerdict {
            rationale: "second look".into(),
            ..verdict(1, 900, Some(2), "medium")
        }],
        50,
    )
    .await
    .unwrap();
    let mut rows = conn
        .query(
            "SELECT rationale, confidence, applied_at, applied_action \
               FROM org_mention_rehoming WHERE notice_id = 900",
            (),
        )
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert_eq!(row.get_value(0).unwrap(), Value::Text("second look".into()), "judgement replaced");
    assert_eq!(row.get_value(1).unwrap(), Value::Text("medium".into()));
    assert_eq!(row.get_value(2).unwrap(), Value::Integer(20), "applied stamp SURVIVES");
    assert_eq!(
        row.get_value(3).unwrap(),
        Value::Text("rehomed from 1".into()),
        "and so does the pre-image — it is the only way back"
    );
    drop(rows);
    // And the re-record did not put it back in the pending set.
    let r = db.apply_rehoming(true, None, 60).await.unwrap();
    assert_eq!(r.pending, 0);
}
