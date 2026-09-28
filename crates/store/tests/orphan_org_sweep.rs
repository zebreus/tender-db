//! Issue 443: organizations no recorded mention points at any more. Issue 434's
//! refresh re-binds a mention in place and leaves the row it left, and every other
//! org delete is merge-shaped, so nothing removed an unreferenced row.
//! `sweep_orphan_orgs_batch` deletes a row with no mention, no party /
//! bid-party / winner row and no review-table entry, provisional or not (step
//! 4), with its name variants, publishing `organization removed`, under foreign
//! keys ON. What is pinned: each keep arm (mentioned, referenced, under review),
//! the identifier-bearing orphan swept with its identity in the pre-image, the
//! dry run writing nothing, the windows' watermark, a second run finding
//! nothing, and the pre-image table's step-4 columns reaching an existing
//! database.

use store::turso::Value;

async fn seed(name: &str) -> store::Db {
    let path = format!("/tmp/tender-db-orphan-sweep-{name}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(&path).await.unwrap();
    db.build_organization_indexes().await.unwrap();
    db.build_tender_indexes().await.unwrap();
    // Fixture rows only: the party rows below reference versions and mentions
    // that do not exist, so this raw connection writes with foreign keys off.
    // The sweep itself runs on the writer, where they are ON.
    let conn = store::turso::Builder::new_local(&path).build().await.unwrap().connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    for (id, name, country, provisional) in [
        (1, "Alpha", Some("DE"), 1), // mentioned — stays
        (2, "", Some("FR"), 1),      // nameless orphan — swept
        (3, "Beta", None, 1),        // country-less orphan — swept
        (4, "Gamma", Some("DE"), 1), // named orphan — swept
        (5, "Ident", Some("DE"), 0), // identifier-bearing orphan — swept (step 4)
        (6, "Delta", Some("DE"), 1), // a party row still names it — stays
        (7, "Eps", Some("DE"), 1),   // under a case review — stays
        (8, "Zeta", Some("DE"), 1),  // a merge verdict's member — stays
        (9, "Eta", Some("DE"), 1),   // a winner row still names it — stays
        (10, "Theta", Some("DE"), 1), // a bid-party row still names it — stays
        (11, "Iota", Some("AT"), 1), // named orphan — swept
        (12, "Kappa", Some("DE"), 1), // a re-homing verdict's case — stays
        (13, "Lambda", Some("DE"), 1), // a re-homing verdict's target — stays
        (14, "Mu", Some("DE"), 1),    // under a country verdict — stays
        (15, "Nu", Some("DE"), 1),    // a dropped name variant's owner — stays
    ] {
        conn.execute(
            "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
             VALUES (?, ?, CASE WHEN ? = 0 THEN 'national' END, CASE WHEN ? = 0 THEN 'X1' END, ?, lower(?), ?, 0)",
            (
                Value::Integer(id),
                country.map_or(Value::Null, |c: &str| Value::Text(c.into())),
                Value::Integer(provisional),
                Value::Integer(provisional),
                Value::Text(name.into()),
                Value::Text(name.into()),
                Value::Integer(provisional),
            ),
        )
        .await
        .unwrap();
    }
    for sql in [
        "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, country) \
         VALUES (100, 'ORG-1', 1, 'Alpha', 'DE')",
        "INSERT INTO organization_names (org_id, lang, name, name_norm) VALUES (1, 'DEU', 'Alpha', 'alpha')",
        "INSERT INTO organization_names (org_id, lang, name, name_norm) VALUES (4, 'DEU', 'Gamma', 'gamma')",
        "INSERT INTO organization_names (org_id, lang, name, name_norm) VALUES (4, 'ENG', 'Gamma', 'gamma')",
        "INSERT INTO tender_version_parties (tender_id, seq, role, organization_id, mention_notice_id, mention_section_id) \
         VALUES (1, 1, 'buyer', 6, 101, 'ORG-1')",
        "INSERT INTO tender_version_result_winners (tender_id, seq, lot_result_id, organization_id) VALUES (1, 1, 1, 9)",
        "INSERT INTO tender_version_bid_parties (tender_id, seq, bid_id, role, organization_id, mention_notice_id, mention_section_id) \
         VALUES (1, 1, 1, 'tenderer', 10, 102, 'ORG-1')",
        "INSERT INTO org_case_reviews (case_org_id, cohort, verdict, diagnosis, handling, rationale, confidence, reviewed_at) \
         VALUES (7, 'c1', 'keep', 'd', 'h', 'r', 'high', 0)",
        "INSERT INTO org_merge_verdicts (country, scheme, key, cohort, members, action, rationale, confidence, reviewed_at) \
         VALUES ('DE', 'national', 'k', 'c1', '[8,99]', 'keep', 'r', 'high', 0)",
        "INSERT INTO org_mention_rehoming (case_org_id, notice_id, section_id, cohort, action, target_org_id, rationale, confidence, reviewed_at) \
         VALUES (12, 200, 'ORG-1', 'c1', 'rehome', 13, 'r', 'high', 0)",
        "INSERT INTO org_country_verdicts (org_id, cohort, action, from_country, to_country, rationale, confidence, reviewed_at) \
         VALUES (14, 'c1', 'move', 'DE', 'AT', 'r', 'high', 0)",
        "INSERT INTO org_name_drops (org_id, lang, name, name_norm, key, target_org, dropped_at) \
         VALUES (15, 'DEU', 'Nu', 'nu', 'nu', 1, 0)",
    ] {
        conn.execute(sql, ()).await.unwrap();
    }
    db
}

async fn count(db: &store::Db, sql: &str) -> i64 {
    match db.scalar(sql).await.unwrap() {
        Some(Value::Integer(n)) => n,
        other => panic!("{sql}: {other:?}"),
    }
}

/// Run every window to the end, `batch` rows at a time, summing the counts.
async fn sweep(db: &store::Db, batch: i64, dry_run: bool) -> store::OrphanOrgSweep {
    let mut total = store::OrphanOrgSweep::default();
    let mut after = 0;
    loop {
        let (w, next) = db.sweep_orphan_orgs_batch(batch, after, dry_run, Some(7)).await.unwrap();
        if w.scanned == 0 {
            assert_eq!(next, after, "an empty window keeps the watermark");
            break;
        }
        assert!(next > after, "the watermark advances");
        after = next;
        total.add(&w);
    }
    total
}

#[tokio::test]
async fn the_review_tables_name_the_keep_set() {
    let db = seed("keep").await;
    let keep = db.org_ids_under_review().await.unwrap();
    for id in [7, 8, 99, 12, 13, 14, 15] {
        assert!(keep.contains(&id), "{id} is under review: {keep:?}");
    }
    assert!(!keep.contains(&4), "{keep:?}");
}

#[tokio::test]
async fn a_dry_run_counts_every_class_and_writes_nothing() {
    let db = seed("dry").await;
    let before = count(&db, "SELECT COUNT(*) FROM changes").await;
    let r = sweep(&db, 4, true).await;
    assert_eq!(r.scanned, 15);
    assert_eq!((r.nameless, r.countryless, r.named), (1, 1, 12), "2 / 3 / 4..15: {r:?}");
    assert_eq!(r.identified, 1, "5, tallied inside `named`: {r:?}");
    assert_eq!(r.referenced, 3, "party, winner and bid-party: {r:?}");
    assert_eq!(r.protected, 6, "case review, merge verdict, re-homing case and target, country verdict, name drop: {r:?}");
    assert_eq!(r.swept, 5, "2, 3, 4, 5 and 11: {r:?}");
    assert_eq!(r.names, 2, "org 4's two variants: {r:?}");
    assert_eq!(count(&db, "SELECT COUNT(*) FROM organizations").await, 15, "nothing deleted");
    assert_eq!(count(&db, "SELECT COUNT(*) FROM organization_names").await, 3);
    assert_eq!(count(&db, "SELECT COUNT(*) FROM changes").await, before, "no change published");
    assert_eq!(count(&db, "SELECT COUNT(*) FROM org_sweep_log").await, 0, "no pre-image logged");
}

#[tokio::test]
async fn a_wet_run_deletes_the_unreferenced_orphans_with_foreign_keys_on() {
    let db = seed("wet").await;
    assert!(db.foreign_keys_enabled().await.unwrap(), "the writer enforces foreign keys");
    // A window of 3 puts orphans on both sides of every boundary.
    let r = sweep(&db, 3, false).await;
    assert_eq!(r.swept, 5, "{r:?}");
    assert_eq!(r.names, 2, "{r:?}");
    let mut left = Vec::new();
    for id in 1..=15 {
        if count(&db, &format!("SELECT COUNT(*) FROM organizations WHERE id = {id}")).await == 1 {
            left.push(id);
        }
    }
    assert_eq!(left, vec![1, 6, 7, 8, 9, 10, 12, 13, 14, 15], "only the five unreferenced orphans went");
    assert_eq!(
        count(&db, "SELECT COUNT(*) FROM organization_names WHERE org_id = 4").await,
        0,
        "a swept row's variants go with it"
    );
    assert_eq!(count(&db, "SELECT COUNT(*) FROM organization_names WHERE org_id = 1").await, 1);
    assert_eq!(
        count(
            &db,
            "SELECT COUNT(*) FROM changes WHERE entity_kind = 'organization' AND op = 'removed' \
              AND entity_id IN (2, 3, 4, 5, 11)"
        )
        .await,
        5,
        "each deletion is published"
    );
    assert!(db.foreign_keys_enabled().await.unwrap(), "and enforcement is still on after");

    // Each swept row is restorable from its pre-image, variants included.
    assert_eq!(count(&db, "SELECT COUNT(*) FROM org_sweep_log").await, 5);
    assert_eq!(
        count(
            &db,
            "SELECT COUNT(*) FROM org_sweep_log WHERE org_id = 4 AND name = 'Gamma' AND name_norm = 'gamma' \
               AND country = 'DE' AND created_at = 0 AND job_id = 7 AND provisional = 1 \
               AND identifier_kind IS NULL AND identifier IS NULL \
               AND names = '[[\"DEU\",\"Gamma\",\"gamma\"],[\"ENG\",\"Gamma\",\"gamma\"]]'"
        )
        .await,
        1,
        "org 4's row and both variants"
    );
    assert_eq!(
        count(
            &db,
            "SELECT COUNT(*) FROM org_sweep_log WHERE org_id = 5 AND name = 'Ident' AND country = 'DE' \
               AND identifier_kind = 'national' AND identifier = 'X1' AND provisional = 0"
        )
        .await,
        1,
        "the identifier-bearing row keeps its identity in the pre-image"
    );
    assert_eq!(
        count(&db, "SELECT COUNT(*) FROM org_sweep_log WHERE org_id = 3 AND country IS NULL AND names = '[]'").await,
        1,
        "a country-less row with no variants"
    );

    let again = sweep(&db, 3, false).await;
    assert_eq!(again.swept, 0, "a second run finds nothing: {again:?}");
    assert_eq!((again.referenced, again.protected, again.identified), (3, 6, 0), "{again:?}");
}

/// The keep-set is read inside each window on the writer, not once per run: a
/// verdict recorded while a long wet sweep is under way protects its org from the
/// next window on.
#[tokio::test]
async fn a_verdict_recorded_mid_run_protects_its_org_from_the_next_window() {
    let db = seed("midrun").await;
    // The first window (ids 1-3) sweeps 2 and 3.
    let (w, next) = db.sweep_orphan_orgs_batch(3, 0, false, None).await.unwrap();
    assert_eq!((w.swept, next), (2, 3), "{w:?}");
    // A reviewer's verdict on org 4 lands between windows, through the writer.
    db.record_case_reviews(
        "late",
        &[store::CaseReview {
            case_org_id: 4,
            verdict: "keep".into(),
            diagnosis: "d".into(),
            handling: "h".into(),
            rationale: "r".into(),
            confidence: "high".into(),
        }],
        0,
    )
    .await
    .unwrap();
    let (w, _) = db.sweep_orphan_orgs_batch(3, next, false, None).await.unwrap();
    assert_eq!(w.protected, 1, "{w:?}");
    assert_eq!(count(&db, "SELECT COUNT(*) FROM organizations WHERE id = 4").await, 1, "org 4 stays");
}

/// Step 4 added the identity columns to `org_sweep_log`, which prod created
/// without them on 2026-09-28 — the issue-372 trap (a fresh test database gets
/// the column from `CREATE TABLE`, an existing one only from `MIGRATIONS`).
/// Pre-create the step-1 shape, open, and write a step-4 pre-image.
#[tokio::test]
async fn an_existing_sweep_log_gains_the_identity_columns() {
    let path = format!("/tmp/tender-db-orphan-sweep-migrate-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    {
        let conn = store::turso::Builder::new_local(&path).build().await.unwrap().connect().unwrap();
        conn.execute(
            "CREATE TABLE org_sweep_log (
                 org_id INTEGER NOT NULL PRIMARY KEY, name TEXT NOT NULL, name_norm TEXT,
                 country TEXT, created_at INTEGER NOT NULL, names TEXT NOT NULL,
                 swept_at INTEGER NOT NULL, job_id INTEGER
             ) STRICT",
            (),
        )
        .await
        .unwrap();
        conn.execute(
            "INSERT INTO org_sweep_log VALUES (1, 'Old', 'old', NULL, 0, '[]', 0, NULL)",
            (),
        )
        .await
        .unwrap();
    }
    let db = store::Db::open(&path).await.unwrap();
    assert_eq!(
        count(&db, "SELECT COUNT(*) FROM org_sweep_log WHERE org_id = 1 AND provisional IS NULL AND identifier IS NULL")
            .await,
        1,
        "the step-1 table survived the open (its row is there), so only the ALTERs added the columns; \
         a row logged before step 4 reads NULL"
    );
    db.build_organization_indexes().await.unwrap();
    db.build_tender_indexes().await.unwrap();
    let conn = store::turso::Builder::new_local(&path).build().await.unwrap().connect().unwrap();
    conn.execute(
        "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at) \
         VALUES (9, 'MC', 'national', 'RCI77S01656', 'Monaco Digital', 'monaco digital', 0, 0)",
        (),
    )
    .await
    .unwrap();
    let (w, _) = db.sweep_orphan_orgs_batch(10, 0, false, None).await.unwrap();
    assert_eq!((w.swept, w.identified), (1, 1), "{w:?}");
    assert_eq!(
        count(&db, "SELECT COUNT(*) FROM org_sweep_log WHERE org_id = 9 AND identifier = 'RCI77S01656' AND provisional = 0")
            .await,
        1
    );
}
