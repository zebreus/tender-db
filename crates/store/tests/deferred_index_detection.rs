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
    // `notices_source_id` is deferred TOO, and deliberately: it was in the schema
    // batch until the build was measured at 413s over 25.3M rows on the real file,
    // which would have made `Db::open` block for ~7 minutes on the deploy restart —
    // the start-up regression issues 82/83 removed.
    assert!(
        missing.contains("notices_source_id"),
        "notices_source_id must be deferred, not schema-batch: building it inside \
         Db::open is a multi-minute blocking boot at prod scale"
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
    db.build_notice_indexes().await.unwrap();

    let after = db.missing_deferred_indexes().await.unwrap();
    assert!(
        after.is_empty(),
        "after running EVERY builder nothing may still be reported missing, or the \
         detector's list has drifted from what the builders actually create — the \
         failure that makes a green meaningless. Still missing: {after:?}"
    );

    // Idempotent: the builders are `CREATE INDEX IF NOT EXISTS` loops, and the
    // detector must stay quiet on a second pass rather than re-reporting work done.
    db.build_organization_indexes().await.unwrap();
    db.build_tender_indexes().await.unwrap();
    db.build_notice_indexes().await.unwrap();
    assert!(db.missing_deferred_indexes().await.unwrap().is_empty());

    clean(&path);
}

#[tokio::test]
async fn dropping_one_index_is_noticed() {
    let (path, db) = open("dropped").await;
    db.build_organization_indexes().await.unwrap();
    db.build_tender_indexes().await.unwrap();
    db.build_notice_indexes().await.unwrap();
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

/// Issue 82: `tenders_current_published` (the newest-first list's covering index) was
/// created ONLY by `migrate()`, not by the projection's builder. A rebuild drops the
/// `tenders` table bare, so the index vanished and nothing rebuilt it until the next
/// boot — a ~49-minute stripped-index boot, and a full-scanning `/v1/tenders` list
/// until then. The fix put it in `DEFERRED_TENDER_INDEXES` so `build_tender_indexes`
/// OWNS it. This pins that ownership across the rebuild cycle: the strip removes it,
/// and the END-OF-FOLD builder — not the next boot — brings it back. If someone drops
/// it from the deferred set, this fails and issue 82 is caught before it ships.
#[tokio::test]
async fn the_newest_list_covering_index_survives_a_rebuild_via_the_builder() {
    let (path, db) = open("list_index_82").await;
    let missing = || async { db.missing_deferred_indexes().await.unwrap() };

    // migrate() built it at open over the empty table; the builder keeps it present.
    db.build_tender_indexes().await.unwrap();
    assert!(
        !missing().await.contains(&"tenders_current_published".to_owned()),
        "precondition: the covering index exists after open + build"
    );

    // A rebuild strips the tender indexes before the fold.
    db.strip_tender_indexes().await.unwrap();
    assert!(
        missing().await.contains(&"tenders_current_published".to_owned()),
        "the pre-fold strip must remove it — otherwise this test proves nothing"
    );

    // The end-of-fold builder must bring it back, so a COMPLETED rebuild leaves the
    // list indexed and the next boot has nothing to build (issue 82).
    db.build_tender_indexes().await.unwrap();
    assert!(
        !missing().await.contains(&"tenders_current_published".to_owned()),
        "build_tender_indexes must own tenders_current_published — if it is removed \
         from DEFERRED_TENDER_INDEXES, issue 82 (unindexed newest-first list + a \
         ~49-min stripped-index boot) returns"
    );

    clean(&path);
}

/// The size cap must actually refuse, not merely exist.
///
/// A bulk `CREATE INDEX` sorts the whole table and its peak RSS is linear in row
/// count (~45 B/row measured on the deployed turso), and it cannot be batched. So the
/// only protection against a future `changes`-sized index entering a deferred set is
/// to decline the build — and a cap nobody has watched refuse is a cap that might not.
///
/// Driven by lowering the cap, not by inserting hundreds of millions of rows. It used
/// to be driven by parking one row at a rowid above the cap, because `MAX(rowid)` was
/// the whole estimate — and that is exactly the estimate that refused a real index at
/// every boot on prod for two days (issue 388: rebuilds inflate rowids an order of
/// magnitude past the row count), so the cap now counts rows past its cheap bound and
/// the test lowers the cap instead.
#[tokio::test]
async fn an_oversized_table_is_refused_not_built() {
    let (path, db) = open("oversize").await;
    let conn = store::turso::Builder::new_local(&path).build().await.unwrap().connect().unwrap();

    // Two organizations against a cap of one: over by count, not by rowid.
    for id in [1, 2] {
        conn.execute(
            &format!(
                "INSERT INTO organizations (id, country, identifier_kind, identifier, name, provisional, created_at)
                 VALUES ({id}, 'DE', 'vat', 'X{id}', 'Org {id}', 0, 1700000000)"
            ),
            (),
        )
        .await
        .unwrap();
    }
    db.set_auto_index_row_cap_for_test(1);
    db.build_organization_indexes().await.unwrap();

    let missing = db.missing_deferred_indexes().await.unwrap();
    assert!(
        missing.contains(&"organizations_country_id".to_owned()),
        "the builder must REFUSE an oversized table and leave the index missing, so \
         the condition stays visible instead of the process dying mid-build — turso \
         cannot interrupt a running statement. Reported missing: {missing:?}"
    );
    // And it must not be a blanket failure: the tender indexes are on empty tables
    // and must still build under the same cap.
    db.build_tender_indexes().await.unwrap();
    let after = db.missing_deferred_indexes().await.unwrap();
    assert!(
        !after.contains(&"tenders_source_id".to_owned()),
        "refusing one oversized table must not stop the others building"
    );

    clean(&path);
}

/// Issue 388: the rowid space is not the row count. Every full rebuild deletes and
/// re-inserts every satellite row, so on prod `tender_version_result_winners` read
/// ~845M by `MAX(rowid)` against a table that cannot exceed ~20M rows, and the
/// covering index shipped for it was refused at every boot. One row parked past the
/// cap must build — the cheap bound trips, the exact count decides.
#[tokio::test]
async fn a_rowid_past_the_cap_over_a_small_table_is_built() {
    let (path, db) = open("highrowid").await;
    let conn = store::turso::Builder::new_local(&path).build().await.unwrap().connect().unwrap();
    conn.execute(
        "INSERT INTO organizations (id, country, identifier_kind, identifier, name, provisional, created_at)
         VALUES (250000000, 'DE', 'vat', 'X', 'Org', 0, 1700000000)",
        (),
    )
    .await
    .unwrap();

    db.build_organization_indexes().await.unwrap();
    let missing = db.missing_deferred_indexes().await.unwrap();
    assert!(
        !missing.contains(&"organizations_country_id".to_owned()),
        "one row past the cap in rowid space is a one-row table and must get its index: {missing:?}"
    );

    clean(&path);
}
