//! Issue 455: `read::resolve_org` follows `org_merge_log` from an organization
//! id a merge removed to its live survivor, so the API can redirect a stale id
//! instead of answering a bare 404 or a certified-empty filtered page.

use store::read::{MERGE_HOPS, OrgResolution, resolve_org};

async fn exec(conn: &turso::Connection, sql: &str) {
    conn.execute(sql, ()).await.unwrap_or_else(|e| panic!("{sql}: {e}"));
}

async fn org(conn: &turso::Connection, id: i64) {
    exec(conn, &format!(
        "INSERT INTO organizations (id, name, name_norm, country, provisional, created_at) \
         VALUES ({id}, 'org {id}', 'org {id}', 'GB', 0, 0)"
    ))
    .await;
}

async fn merged(conn: &turso::Connection, keep: i64, loser: i64, at: i64) {
    exec(conn, &format!(
        "INSERT INTO org_merge_log (keep, loser, rule, evidence, at) VALUES ({keep}, {loser}, 'r2', '{{}}', {at})"
    ))
    .await;
}

#[tokio::test]
async fn a_merged_away_id_resolves_through_the_ledger_to_its_live_survivor() {
    let path = format!("/tmp/tender-db-455-resolve-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let _db = store::Db::open(&path).await.expect("open");
    let raw = turso::Builder::new_local(&path).build().await.expect("raw open");
    let conn = raw.connect().expect("connect");

    // A two-hop chain: 1 merged into 2, and 2 later merged into the live 3.
    org(&conn, 3).await;
    merged(&conn, 2, 1, 100).await;
    merged(&conn, 3, 2, 200).await;
    // A loser merged twice (into 10, restored by a later mint, then into 20): the
    // NEWEST merge is where it lives now.
    org(&conn, 10).await;
    org(&conn, 20).await;
    merged(&conn, 10, 5, 100).await;
    merged(&conn, 20, 5, 300).await;
    // A cycle with no live row on it.
    merged(&conn, 7, 6, 100).await;
    merged(&conn, 6, 7, 200).await;
    // A chain one hop longer than the bound: 100 → 101 → … → the live 100 + MERGE_HOPS + 1.
    let hops = MERGE_HOPS as i64;
    org(&conn, 100 + hops + 1).await;
    for loser in 100..=100 + hops {
        merged(&conn, loser + 1, loser, 100).await;
    }

    assert_eq!(resolve_org(&conn, 3).await.unwrap(), OrgResolution::Live);
    assert_eq!(resolve_org(&conn, 1).await.unwrap(), OrgResolution::MergedInto(3), "two hops");
    assert_eq!(resolve_org(&conn, 2).await.unwrap(), OrgResolution::MergedInto(3), "one hop");
    assert_eq!(resolve_org(&conn, 5).await.unwrap(), OrgResolution::MergedInto(20), "newest merge wins");
    assert_eq!(resolve_org(&conn, 6).await.unwrap(), OrgResolution::Unknown, "a cycle ends the walk");
    assert_eq!(resolve_org(&conn, 999).await.unwrap(), OrgResolution::Unknown, "never an org");
    assert_eq!(
        resolve_org(&conn, 101).await.unwrap(),
        OrgResolution::MergedInto(100 + hops + 1),
        "exactly MERGE_HOPS hops resolve"
    );
    assert_eq!(resolve_org(&conn, 100).await.unwrap(), OrgResolution::Unknown, "one hop past the bound");

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}
