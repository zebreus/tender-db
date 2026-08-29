//! Issue 311: recorded per-case review verdicts and the safe apply subset
//! (identifier strip on high-confidence wrong-identifier verdicts). The
//! reviewer never writes entities; this machinery is the only applier, and
//! every apply keeps its pre-image in the review row.

use store::CaseReview;
use store::turso::Value;

fn review(org: i64, verdict: &str, confidence: &str) -> CaseReview {
    CaseReview {
        case_org_id: org,
        verdict: verdict.into(),
        diagnosis: "lead-member-identifier".into(),
        handling: "strip".into(),
        rationale: "test".into(),
        confidence: confidence.into(),
    }
}

async fn count(conn: &store::turso::Connection, sql: &str) -> i64 {
    let mut rows = conn.query(sql, ()).await.unwrap();
    let row = rows.next().await.unwrap().unwrap();
    let Value::Integer(n) = row.get_value(0).unwrap() else { panic!("count") };
    n
}

#[tokio::test]
async fn verdicts_record_and_only_the_safe_subset_applies() {
    let path = "test-case-reviews.db";
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(path).await.unwrap();
    let raw = store::turso::Builder::new_local(path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    for (id, ident) in
        [(60i64, Some("FN576377P")), (61, Some("DE111111111")), (62, Some("ATU222")), (63, None)]
    {
        conn.execute(
            "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
             VALUES (?, 'AT', CASE WHEN ? IS NULL THEN NULL ELSE 'national' END, ?, 'Biege X', 'biege x', 0, 0)",
            (
                Value::Integer(id),
                match ident { Some(v) => Value::Text(v.into()), None => Value::Null },
                match ident { Some(v) => Value::Text(v.into()), None => Value::Null },
            ),
        )
        .await
        .unwrap();
    }

    // 60: strips. 61: medium confidence — skipped. 62: sound verdict —
    // skipped. 63: eligible but already identifier-less — no-op stamp.
    db.record_case_reviews(
        "biege-pilot",
        &[
            review(60, "consortium-vehicle-wrong-identifier", "high"),
            review(61, "consortium-vehicle-wrong-identifier", "medium"),
            review(62, "consortium-vehicle-sound", "high"),
            review(63, "consortium-vehicle-wrong-identifier", "high"),
        ],
        1000,
    )
    .await
    .unwrap();
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM org_case_reviews").await, 4);

    let dry = db.apply_case_reviews(true, Some(7), 2000).await.unwrap();
    assert_eq!((dry.pending, dry.eligible, dry.stripped, dry.noop), (4, 2, 1, 1));
    assert_eq!(
        dry.plan,
        vec![(60, "Biege X".to_owned(), "FN576377P".to_owned())],
        "the dry plan names the concrete strip"
    );
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM organizations WHERE identifier IS NOT NULL").await,
        3,
        "dry run wrote nothing"
    );

    let wet = db.apply_case_reviews(false, Some(8), 3000).await.unwrap();
    assert_eq!((wet.pending, wet.eligible, wet.stripped, wet.noop), (4, 2, 1, 1));
    // 60 stripped, pre-image kept; 61/62 untouched and still pending; 63
    // stamped as a no-op.
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM organizations WHERE id = 60 AND identifier IS NULL AND identifier_kind IS NULL").await,
        1
    );
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM org_case_reviews WHERE case_org_id = 60 AND applied_action LIKE '%identifier-stripped%' AND applied_action LIKE '%FN576377P%'").await,
        1,
        "the pre-image lives in applied_action"
    );
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM organizations WHERE id = 61 AND identifier = 'DE111111111'").await,
        1
    );
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM org_case_reviews WHERE applied_at IS NULL").await,
        2,
        "the ineligible verdicts stay pending (61 medium, 62 sound)"
    );
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM changes WHERE entity_kind = 'organization' AND entity_id = 60 AND op = 'changed'").await,
        1
    );

    // Idempotence: a re-run finds nothing eligible among the unapplied.
    let again = db.apply_case_reviews(false, Some(9), 4000).await.unwrap();
    assert_eq!((again.pending, again.eligible, again.stripped, again.noop), (2, 0, 0, 0));

    // A re-record (the operator re-POST after an apply — the panel's
    // scenario) updates the verdict fields but PRESERVES the applied stamp
    // and its pre-image: the executed action's history stands, and nothing
    // re-applies.
    db.record_case_reviews(
        "biege-pilot",
        &[review(60, "consortium-vehicle-wrong-identifier", "high")],
        5000,
    )
    .await
    .unwrap();
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM org_case_reviews WHERE case_org_id = 60 AND applied_action LIKE '%FN576377P%' AND reviewed_at = 5000").await,
        1,
        "re-record refreshed the verdict fields but kept the pre-image"
    );
    let rerun = db.apply_case_reviews(false, Some(10), 6000).await.unwrap();
    assert_eq!(
        (rerun.pending, rerun.eligible, rerun.stripped, rerun.noop),
        (2, 0, 0, 0),
        "the applied case stays applied after a re-record"
    );

    // The dry plan names the concrete strips (the reviewability gate).
    let dry2 = db.apply_case_reviews(true, Some(11), 7000).await.unwrap();
    assert!(dry2.plan.is_empty(), "nothing left to strip, plan empty");
}
