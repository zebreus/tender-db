//! Issue 286: the provisional-org merge repoints party/bid/winner rows in place,
//! changing what the `buyer`/`winner`/`bidder` org-role filters return on
//! current-version rows — so every touched Tender must get a `tender` `changed`
//! change-feed row, or a `/v1/changes` / SSE subscriber never learns the Tender's
//! membership moved. These tests seed the org layer directly (the merge's inputs)
//! and assert the merge emits those Tender events (and emits nothing on a dry run).

use store::turso::{self, Value};

async fn open(name: &str) -> (store::Db, turso::Connection, String) {
    let path = format!("/tmp/tender-db-orgmerge-ev-{name}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(&path).await.expect("open");
    let raw = turso::Builder::new_local(&path).build().await.expect("raw");
    let conn = raw.connect().expect("connect");
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.expect("fk off");
    (db, conn, path)
}

/// Two provisional orgs of ONE (name_norm, country) group — the merge collapses
/// them, keep = min id = 1, loser = 2. The loser is referenced by a party row on
/// Tender 100 and winner rows on Tenders 100 and 200, so the touched set is
/// {100, 200} (100 reached through two legs — the dedup case).
async fn seed(conn: &turso::Connection) {
    for id in [1i64, 2] {
        conn.execute(
            "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
             VALUES (?, 'DE', NULL, NULL, ?, 'acme', 1, 1700000000)",
            (Value::Integer(id), Value::Text(format!("Acme {id}"))),
        )
        .await
        .expect("insert org");
    }
    // A buyer party on Tender 100 naming the loser.
    conn.execute(
        "INSERT INTO tender_version_parties (tender_id, seq, lot_id, role, organization_id, mention_notice_id, mention_section_id)
         VALUES (100, 1, NULL, 'buyer', 2, 1, 'S1')",
        (),
    )
    .await
    .expect("insert party");
    // Winner rows naming the loser, on Tenders 100 (again — dedup) and 200.
    conn.execute(
        "INSERT INTO tender_version_result_winners (tender_id, seq, lot_result_id, organization_id)
         VALUES (100, 1, 10, 2), (200, 1, 20, 2)",
        (),
    )
    .await
    .expect("insert winners");
}

async fn scalar(conn: &turso::Connection, sql: &str) -> i64 {
    let mut rows = conn.query(sql, ()).await.expect("query");
    match rows.next().await.expect("row") {
        Some(row) => row.get_value(0).unwrap().as_integer().copied().unwrap_or(0),
        None => 0,
    }
}

#[tokio::test]
async fn a_merge_emits_a_tender_changed_event_for_each_repointed_tender() {
    let (db, conn, path) = open("real").await;
    seed(&conn).await;

    let report = db
        .merge_provisional_organizations_batch(100, "", false)
        .await
        .expect("merge");

    // The group collapsed and the reference rows moved.
    assert_eq!(report.groups, 1, "one (name_norm, country) group");
    assert_eq!(report.removed, 1, "one loser org removed");
    assert_eq!(report.parties, 1, "the party row repointed");
    assert_eq!(report.winners, 2, "both winner rows repointed");

    // The fix: two DISTINCT touched Tenders (100 reached via party AND winner is
    // counted once), so two `tender` `changed` rows.
    assert_eq!(report.tender_changes, 2, "one change per touched Tender, deduped");
    assert_eq!(
        scalar(
            &conn,
            "SELECT COUNT(*) FROM changes WHERE entity_kind='tender' AND op='changed' AND version_seq IS NULL",
        )
        .await,
        2,
        "a seq-less tender changed row per touched Tender (issue 286)"
    );
    for tid in [100, 200] {
        assert_eq!(
            scalar(
                &conn,
                &format!("SELECT COUNT(*) FROM changes WHERE entity_kind='tender' AND entity_id={tid}"),
            )
            .await,
            1,
            "Tender {tid} got exactly one change row"
        );
    }

    // And the org-side events are still emitted (issue 285: survivor `changed`).
    assert_eq!(
        scalar(&conn, "SELECT COUNT(*) FROM changes WHERE entity_kind='organization' AND op='removed' AND entity_id=2").await,
        1,
        "loser removed"
    );
    assert_eq!(
        scalar(&conn, "SELECT COUNT(*) FROM changes WHERE entity_kind='organization' AND op='changed' AND entity_id=1").await,
        1,
        "survivor changed"
    );

    // The repoints actually landed: loser gone, references now name the survivor.
    assert_eq!(scalar(&conn, "SELECT COUNT(*) FROM organizations WHERE id=2").await, 0, "loser deleted");
    assert_eq!(
        scalar(&conn, "SELECT COUNT(*) FROM tender_version_parties WHERE organization_id=1").await,
        1,
        "party now names the survivor"
    );
    assert_eq!(
        scalar(&conn, "SELECT COUNT(*) FROM tender_version_result_winners WHERE organization_id=1").await,
        2,
        "winners now name the survivor"
    );

    drop(db);
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

#[tokio::test]
async fn a_dry_run_emits_no_tender_events_and_moves_nothing() {
    let (db, conn, path) = open("dry").await;
    seed(&conn).await;

    let report = db
        .merge_provisional_organizations_batch(100, "", true)
        .await
        .expect("dry-run merge");

    // A dry run sizes the work but writes nothing.
    assert_eq!(report.groups, 1, "the group is still counted");
    assert_eq!(report.tender_changes, 0, "no tender events on a dry run");
    assert_eq!(scalar(&conn, "SELECT COUNT(*) FROM changes").await, 0, "no change rows at all");
    assert_eq!(scalar(&conn, "SELECT COUNT(*) FROM organizations WHERE id=2").await, 1, "loser untouched");

    drop(db);
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}
