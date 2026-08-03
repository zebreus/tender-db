//! Issue 111: the deferred indexes have no guaranteed builder, so a deploy that
//! adds one leaves it uncreated until somebody fires `Reindex` by hand. That is how
//! issue 117's DoS fix would ship without taking effect.
//!
//! `Db::missing_deferred_indexes` is the detection half — cheap enough for every
//! boot, so the caller can enqueue the existing background job instead of building
//! anything inline (building at boot is what issues 82/83 removed).
//!
//! This asserts the detector actually detects. The load-bearing case is the FIRST
//! one: a freshly opened database is exactly the state a deploy leaves behind, and a
//! detector that reported "nothing missing" there would be silently useless — which
//! is the failure mode of a check that has never been shown to fail.

use std::collections::HashSet;
use store::Db;

async fn open(tag: &str) -> (String, Db) {
    let path = format!("/tmp/tender-db-defidx-{tag}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = Db::open(&path).await.unwrap();
    (path, db)
}

fn clean(path: &str) {
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

#[tokio::test]
async fn a_fresh_database_reports_its_deferred_indexes_missing() {
    let (path, db) = open("fresh").await;

    let missing: HashSet<String> = db.missing_deferred_indexes().await.unwrap().into_iter().collect();
    assert!(
        !missing.is_empty(),
        "a freshly opened database has none of the deferred indexes, so the detector \
         must say so — an empty answer here means it cannot detect anything at all"
    );

    // The issue-117 indexes specifically: these are the ones whose absence leaves a
    // public endpoint walking a whole table, so name them rather than trusting a count.
    for name in ["organizations_country_id", "organizations_kind_id", "tenders_source_id"] {
        assert!(missing.contains(name), "{name} must be reported missing on a fresh db");
    }
    // `notices_source_id` is in the SCHEMA BATCH, not a deferred list — it builds
    // itself at open. If it ever shows up as missing, it has been moved into a
    // deferred set and has silently acquired the issue-111 obligation.
    assert!(
        !missing.contains("notices_source_id"),
        "notices_source_id is schema-batch and must exist at open; if this fails it \
         has been moved to a deferred list and now needs a builder like the rest"
    );

    clean(&path);
}

#[tokio::test]
async fn building_them_empties_the_report() {
    let (path, db) = open("built").await;

    let before = db.missing_deferred_indexes().await.unwrap();
    assert!(!before.is_empty(), "precondition: something must be missing to begin with");

    db.build_organization_indexes().await.unwrap();
    db.build_tender_indexes().await.unwrap();

    let after = db.missing_deferred_indexes().await.unwrap();
    assert!(
        after.is_empty(),
        "after running BOTH builders nothing may still be reported missing, or the \
         detector's list has drifted from what the builders actually create — the \
         failure that makes a green meaningless. Still missing: {after:?}"
    );

    // Idempotent: the builders are `CREATE INDEX IF NOT EXISTS` loops, and the
    // detector must stay quiet on a second pass rather than re-reporting work done.
    db.build_organization_indexes().await.unwrap();
    db.build_tender_indexes().await.unwrap();
    assert!(db.missing_deferred_indexes().await.unwrap().is_empty());

    clean(&path);
}

#[tokio::test]
async fn dropping_one_index_is_noticed() {
    let (path, db) = open("dropped").await;
    db.build_organization_indexes().await.unwrap();
    db.build_tender_indexes().await.unwrap();
    assert!(db.missing_deferred_indexes().await.unwrap().is_empty());

    // A rebuild strips the tender indexes; this is the state a truncated or
    // interrupted run leaves behind, and the one `Reindex` exists to repair.
    db.strip_tender_indexes().await.unwrap();

    let missing = db.missing_deferred_indexes().await.unwrap();
    assert!(
        missing.contains(&"tenders_source_id".to_owned()),
        "a stripped tender index must be reported missing — that is the whole point \
         of running the detector at boot. Reported: {missing:?}"
    );
    assert!(
        !missing.contains(&"organizations_country_id".to_owned()),
        "stripping the TENDER indexes must not implicate the organization ones"
    );

    clean(&path);
}
