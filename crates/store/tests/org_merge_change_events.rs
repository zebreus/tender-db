//! Issue 286: the provisional-org merge repoints party/bid/winner rows in place,
//! changing what the `buyer`/`winner`/`bidder` org-role filters return on
//! current-version rows — so every touched Tender must get a `tender` `changed`
//! change-feed row, or a `/v1/changes` / SSE subscriber never learns the Tender's
//! membership moved. These tests seed the org layer directly (the merge's inputs)
//! and assert the merge emits those Tender events (and emits nothing on a dry run).
//! Issue 583: the merge also stamps those Tenders stale and re-queues their notices, so
//! the next fold writes the repointed rows in its own order and a later refold rewrites
//! no table.

use store::canonical::{BidParty, BidState, Fact, LotResultState, LotState, Round, TenderProjection, TenderVersion};
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

/// Issue 484 unit 3: the merge's winner collapse and `is_buyer`. A loser row that
/// moves keeps its flag (the UPDATE touches `organization_id` only); a loser row that
/// duplicates a survivor row on the same result is deleted, and an unflagged survivor
/// stays unflagged (the fused org is bound by a non-buyer section there).
#[tokio::test]
async fn a_merge_keeps_each_surviving_winner_rows_own_is_buyer() {
    let (db, conn, path) = open("is-buyer").await;
    seed(&conn).await;
    conn.execute("UPDATE tender_version_result_winners SET is_buyer = 1 WHERE organization_id = 2", ())
        .await
        .expect("flag the loser's rows");
    // The survivor already won result 10, unflagged: the loser's duplicate goes.
    conn.execute(
        "INSERT INTO tender_version_result_winners (tender_id, seq, lot_result_id, organization_id) VALUES (100, 1, 10, 1)",
        (),
    )
    .await
    .expect("survivor row");

    let report = db.merge_provisional_organizations_batch(100, "", false).await.expect("merge");
    assert_eq!(report.removed, 1);

    assert_eq!(scalar(&conn, "SELECT COUNT(*) FROM tender_version_result_winners WHERE organization_id = 2").await, 0);
    assert_eq!(
        scalar(&conn, "SELECT COUNT(*) FROM tender_version_result_winners WHERE lot_result_id = 10").await,
        1,
        "the duplicate collapsed onto the survivor's row"
    );
    assert_eq!(
        scalar(&conn, "SELECT COUNT(*) FROM tender_version_result_winners WHERE lot_result_id = 10 AND is_buyer IS NULL").await,
        1,
        "the surviving row keeps its own (unflagged) value"
    );
    assert_eq!(
        scalar(&conn, "SELECT is_buyer FROM tender_version_result_winners WHERE lot_result_id = 20 AND organization_id = 1").await,
        1,
        "a moved row keeps its flag"
    );

    drop(db);
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

/// Issue 484 unit 3 (review): the reverse collapse. A FLAGGED survivor row whose
/// loser duplicate on the same result was NOT flagged is cleared — the fold flags an
/// org only when every section that bound it in the result is buyer-equal, and the
/// fused org is now bound by the loser's non-buyer section too. Both flagged: it stays.
#[tokio::test]
async fn a_merge_clears_a_survivor_flag_when_an_unflagged_duplicate_collapses_onto_it() {
    let (db, conn, path) = open("is-buyer-clear").await;
    seed(&conn).await;
    // Result 10: survivor flagged, loser unflagged → cleared. Result 20: both flagged
    // → stays 1.
    conn.execute("UPDATE tender_version_result_winners SET is_buyer = 1 WHERE lot_result_id = 20", ())
        .await
        .expect("flag the loser's row on 20");
    conn.execute(
        "INSERT INTO tender_version_result_winners (tender_id, seq, lot_result_id, organization_id, is_buyer)
         VALUES (100, 1, 10, 1, 1), (200, 1, 20, 1, 1)",
        (),
    )
    .await
    .expect("flagged survivor rows");

    let report = db.merge_provisional_organizations_batch(100, "", false).await.expect("merge");
    assert_eq!(report.removed, 1);

    assert_eq!(scalar(&conn, "SELECT COUNT(*) FROM tender_version_result_winners WHERE organization_id = 2").await, 0);
    assert_eq!(
        scalar(&conn, "SELECT COUNT(*) FROM tender_version_result_winners WHERE lot_result_id = 10 AND is_buyer IS NULL").await,
        1,
        "an unflagged duplicate clears the survivor's flag"
    );
    assert_eq!(
        scalar(&conn, "SELECT is_buyer FROM tender_version_result_winners WHERE lot_result_id = 20 AND organization_id = 1").await,
        1,
        "two flagged rows collapse onto a flagged one"
    );
    assert_eq!(scalar(&conn, "SELECT COUNT(*) FROM tender_version_result_winners").await, 2);

    drop(db);
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

// ---- Issue 583: a merge leaves its Tenders where the next refold verifies them ----

/// The fold's `now`, after the award's publication (issue 579's fixture).
const NOW: i64 = 1_767_441_600;
/// The survivor sorts before the sibling and the loser after it, so the loser's rows moved
/// onto the survivor in place sit where the fold would not write them.
const KEEP: i64 = 10;
const BUYER: i64 = 20;
const SIBLING: i64 = 30;
const LOSER: i64 = 50;

/// Notice 900's Tender with `org` on mention ORG-2 beside the sibling on ORG-3: tender-scope
/// parties (the buyer, then both as `winner`), RES-1 won by BOTH (a multi-winner lot result:
/// an in-place repoint reorders its winner rows only then), RES-2 by the sibling, one bid on
/// LOT-1 by both.
fn award(org: i64) -> TenderProjection {
    projection(900, "pk-583", "winner", org, vec![org, SIBLING])
}

/// Notice 901's Tender, where `org` is only a party and a bidder: both results are the
/// sibling's, so no winner row names `org`.
fn party_only(org: i64) -> TenderProjection {
    projection(901, "pk-583-b", "tenderer", org, vec![SIBLING])
}

/// Ordered as the projection orders it: the parties a `BTreeSet<Fact>`, each bid's parties
/// sorted, each lot result's winners sorted and deduped (ingest's project.rs).
fn projection(notice: i64, procedure_key: &str, role: &str, org: i64, mut res1: Vec<i64>) -> TenderProjection {
    res1.sort_unstable();
    res1.dedup();
    let party = |role: &str, org: i64, section: &str| Fact::Party {
        role: role.into(),
        organization_id: org,
        notice_id: notice,
        section_id: section.into(),
    };
    let result = |key: &str, lot: &str, winners: Vec<i64>| LotResultState {
        key: key.into(),
        lot_key: Some(lot.into()),
        decision: None,
        reason: None,
        awarded_cents: None,
        awarded_currency: None,
        decided: None,
        winners,
        buyer_winners: Vec::new(),
        statistics: Vec::new(),
        published_lot_no: None,
        group: false,
    };
    let lot = |key: &str| LotState { key: key.into(), kind: "Lot".into(), facts: Default::default(), published_lot_no: None };
    let mut bidders = vec![
        BidParty { role: "tenderer".into(), organization_id: org, section_id: "ORG-2".into() },
        BidParty { role: "tenderer".into(), organization_id: SIBLING, section_id: "ORG-3".into() },
    ];
    bidders.sort();
    TenderProjection {
        source: "ted".into(),
        procedure_key: Some(procedure_key.into()),
        island_notice_id: None,
        kind: "procedure".into(),
        versions: vec![TenderVersion {
            caused_by_notice_id: notice,
            published_at: NOW - 86_400,
            dispatched_at: None,
            notice_subtype: None,
            original_lang: None,
            publication_id: format!("{notice}-2026"),
            facts: [party("buyer", BUYER, "ORG-1"), party(role, org, "ORG-2"), party(role, SIBLING, "ORG-3")]
                .into_iter()
                .collect(),
            lots: vec![lot("LOT-1"), lot("LOT-2")],
            rounds: vec![Round {
                notice_id: notice,
                logical_notice_id: None,
                lot_results: vec![result("RES-1", "LOT-1", res1), result("RES-2", "LOT-2", vec![SIBLING])],
                bids: vec![BidState {
                    key: "TEN-1".into(),
                    lot_key: Some("LOT-1".into()),
                    cents: None,
                    currency: None,
                    quality: None,
                    parties: bidders,
                }],
                contracts: Vec::new(),
            }],
            group_members: Vec::new(),
            stated_totals: Vec::new(),
        }],
    }
}

/// Both Tenders folded with the loser on ORG-2. `class_country` puts the survivor and the
/// loser (one name, no identifier, provisional) in the merge's class: a country for issue
/// 234's merge, none for the p0 echo fold. The buyer and the sibling carry identifiers, so
/// neither merge reaches them.
async fn seed_583(name: &str, class_country: Option<&str>) -> (store::Db, turso::Connection, String) {
    let (db, conn, path) = open(name).await;
    db.set_refold_compare(store::RefoldCompare::On);
    // The compare refuses to run without the by-version indexes.
    db.build_tender_indexes().await.unwrap();
    for (id, country, identifier, org_name) in [
        (KEEP, class_country, None, "Kreis Zwei"),
        (BUYER, Some("CZ"), Some("CZ-BUYER-20"), "Buyer Office"),
        (SIBLING, Some("CZ"), Some("49371185"), "Sibling Works"),
        (LOSER, class_country, None, "Kreis Zwei"),
    ] {
        conn.execute(
            "INSERT INTO organizations (id, country, name, name_norm, provisional, identifier_kind, identifier, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, 0)",
            (
                Value::Integer(id),
                country.map_or(Value::Null, |c| Value::Text(c.into())),
                Value::Text(org_name.into()),
                Value::Text(org_name.to_lowercase()),
                Value::Integer(identifier.is_none() as i64),
                identifier.map_or(Value::Null, |_| Value::Text("national".into())),
                identifier.map_or(Value::Null, |i| Value::Text(i.into())),
            ),
        )
        .await
        .unwrap();
    }
    for notice in [900i64, 901] {
        conn.execute(
            "INSERT INTO notices (id, source, publication_id, content_hash, profile, fetch_id,
                                  member_path, ingested_at, parse_state, projected)
             VALUES (?, 'ted', ?, ?, 'eforms:test', 1, 'p', 0, 'parsed', 1)",
            (Value::Integer(notice), Value::Text(format!("{notice}-2026")), Value::Text(format!("h{notice}"))),
        )
        .await
        .unwrap();
        for (section, org, org_name) in
            [("ORG-1", BUYER, "Buyer Office"), ("ORG-2", LOSER, "Kreis Zwei"), ("ORG-3", SIBLING, "Sibling Works")]
        {
            conn.execute(
                "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, country)
                 VALUES (?, ?, ?, ?, 'CZ')",
                (Value::Integer(notice), Value::Text(section.into()), Value::Integer(org), Value::Text(org_name.into())),
            )
            .await
            .unwrap();
        }
    }
    db.apply_tenders(&[award(LOSER), party_only(LOSER)], NOW, false).await.unwrap();
    (db, conn, path)
}

/// After a merge moved the loser onto the survivor: both Tenders stamped stale and their
/// notices re-queued in the merge's own transaction; the trailing fold rewrites the five
/// tables the in-place repoint left out of order (the award's parties, bid parties and
/// winners, the other's parties and bid parties); a later refold with the compare on rewrites
/// none. Unstamped (before issue 583) the trailing fold skipped both Tenders, and that refold
/// rewrote all five, announcing corrections that moved nothing.
async fn assert_merge_reaches_the_fold_fixed_point(db: &store::Db, conn: &turso::Connection) {
    assert_eq!(
        scalar(conn, "SELECT COUNT(*) FROM tender_version_parties WHERE organization_id = 50").await,
        0,
        "the loser's rows moved"
    );
    assert_eq!(
        scalar(conn, "SELECT COUNT(*) FROM tenders WHERE projection_epoch = 0").await,
        2,
        "issue 583: both touched Tenders stamped stale"
    );
    assert_eq!(
        scalar(conn, "SELECT COUNT(*) FROM notices WHERE id IN (900, 901) AND projected = 0").await,
        2,
        "and their notices re-queued"
    );
    let trailing = db.apply_tenders(&[award(KEEP), party_only(KEEP)], NOW, false).await.unwrap();
    assert_eq!(
        (trailing.tenders_corrected, trailing.tables_rewritten),
        (2, 5),
        "the repointed rows put in the fold's order: {trailing:?}"
    );
    db.set_projection_epoch_for_test(-1).await.unwrap();
    let refold = db.apply_tenders(&[award(KEEP), party_only(KEEP)], NOW, false).await.unwrap();
    assert_eq!(
        (refold.tenders_verified, refold.tenders_corrected, refold.tables_rewritten),
        (2, 0, 0),
        "the fold's fixed point: {refold:?}"
    );
}

/// Issue 583: issue 234's merge (one immediate transaction per batch) stamps every Tender its
/// repoint touched; a dry run stamps nothing.
#[tokio::test]
async fn a_provisional_merge_then_a_refold_rewrites_no_table() {
    let (db, conn, path) = seed_583("parity-234", Some("CZ")).await;
    let dry = db.merge_provisional_organizations_batch(100, "", true).await.expect("dry");
    assert_eq!((dry.groups, dry.removed), (1, 1), "{dry:?}");
    assert_eq!(
        scalar(&conn, "SELECT COUNT(*) FROM tenders WHERE projection_epoch = 0").await,
        0,
        "a dry run stamps nothing"
    );
    let wet = db.merge_provisional_organizations_batch(100, "", false).await.expect("wet");
    assert_eq!(
        (wet.removed, wet.mentions, wet.parties, wet.bid_parties, wet.winners, wet.winner_dups, wet.tender_changes),
        (1, 2, 2, 2, 1, 0, 2),
        "{wet:?}"
    );
    assert_merge_reaches_the_fold_fixed_point(&db, &conn).await;
    drop(db);
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

fn echo_n2(name: &str) -> String {
    name.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

/// Issue 583: the p0 echo fold (`fold_provisional_plan`: one BEGIN/COMMIT per chunk, the loop
/// shape r2, e0, r3, altid, rekey and p1 share) stamps every Tender its repoint touched.
#[tokio::test]
async fn a_provisional_echo_fold_then_a_refold_rewrites_no_table() {
    let (db, conn, path) = seed_583("parity-p0", None).await;
    let stop = || false;
    let progress = |_: u64, _: &str| {};
    let wet = db
        .fold_provisional_echoes(store::ProvisionalFoldArgs {
            n2: echo_n2,
            stoplist_cap: 8,
            dry_run: false,
            max_groups: None,
            expect_groups: None,
            job_id: Some(583),
            stop: &stop,
            progress: &progress,
        })
        .await
        .expect("wet");
    assert_eq!(
        (wet.merged_groups, wet.removed, wet.mentions, wet.parties, wet.bid_parties, wet.winners, wet.tender_changes),
        (1, 1, 2, 2, 2, 1, 2),
        "{wet:?}"
    );
    assert_merge_reaches_the_fold_fixed_point(&db, &conn).await;
    drop(db);
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}
