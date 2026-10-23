//! Issue 300 Stage 1, the repair half: `repair_placeholder_orgs_batch`
//! dissolves orgs whose identifier the injected predicate condemns —
//! re-resolving each mention through the resolver's no-identifier branch
//! (issue 578: the register's country, the lowest id, one row per key, the
//! issue-351 wall for a country-less name), moving
//! party/bid-party rows by their exact (mention_notice_id,
//! mention_section_id), repointing winner rows via caused_by_notice_id with
//! the one-mention-per-notice guard, and deleting the org. A winner row no
//! tier resolves goes to tier 5 (issue 309): the row is deleted, its tender
//! stamped epoch-stale and its causing notices re-queued, so the next
//! incremental fold re-derives the winner set from the re-bound mentions.
//! Since issue 579 every touched tender is stamped and re-queued the same
//! way, so that fold also writes the moved rows in its own order and a later
//! refold rewrites no table.
//! Dry-run counts and writes nothing; a second wet run finds nothing
//! (dissolved orgs left scope).

use store::canonical::{BidParty, BidState, Fact, LotResultState, LotState, Round, TenderProjection, TenderVersion};
use store::turso::Value;

fn bad(_country: Option<&str>, _kind: &str, value: &str) -> bool {
    value == "BAD"
}

/// The name fields and language normaliser the job passes from ingest (issue 542).
const FIELDS: &[&str] = &["TED-OFFICIALNAME"];

fn normalize(lang: Option<&str>) -> Option<String> {
    match lang?.to_ascii_uppercase().as_str() {
        "DE" | "DEU" => Some("DEU".into()),
        "FR" | "FRA" => Some("FRA".into()),
        other => Some(other.to_owned()),
    }
}

/// The resolver's issue-351 inputs the job passes from ingest (issue 578): an N2-shaped
/// key, and a wall low enough that three carriers make a name generic.
fn n2(name: &str) -> String {
    name.to_lowercase().chars().filter(char::is_ascii_alphanumeric).collect()
}
const CAP: usize = 2;

async fn seed(path: &str) -> (store::Db, store::turso::Connection) {
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(path).await.unwrap();
    let raw = store::turso::Builder::new_local(path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    conn.execute("BEGIN", ()).await.unwrap();

    // 50 = condemned stranger-merger with three mentions; 51 = condemned and
    // winner-ambiguous (two mentions on one notice, no role signal — the
    // tier-5 dissolve-then-refold case); 52 = clean identifier;
    // 60 = standing provisional the "Alpha City" mention must REUSE.
    for (id, ident, name, prov) in [
        (50, Some("BAD"), "Stranger Merger", 0),
        (51, Some("BAD"), "Ambiguous", 0),
        (52, Some("49371185"), "Gymnázium", 0),
        (53, Some("BAD"), "Tier Two", 0),
        (54, Some("BAD"), "Carried", 0),
        (55, Some("BAD"), "Two Names", 0),
        (56, Some("BAD"), "Chain Names", 0),
        (57, Some("BAD"), "Sibling Pair", 0),
        (60, None, "Alpha City", 1),
    ] {
        conn.execute(
            "INSERT INTO organizations (id, country, name, name_norm, provisional, identifier_kind, identifier, created_at)
             VALUES (?, 'CZ', ?, ?, ?, CASE WHEN ? IS NULL THEN NULL ELSE 'national' END, ?, 0)",
            (
                Value::Integer(id),
                Value::Text(name.into()),
                Value::Text(name.to_lowercase()),
                Value::Integer(prov),
                match ident {
                    Some(s) => Value::Text(s.into()),
                    None => Value::Null,
                },
                match ident {
                    Some(s) => Value::Text(s.into()),
                    None => Value::Null,
                },
            ),
        )
        .await
        .unwrap();
    }
    // 50's mentions: reuse (Alpha City == org 60), fresh named (Beta Corp),
    // fresh nameless. 51's: two on one notice (the winner ambiguity).
    for (nid, sid, org, name, country) in [
        (100, "S-1", 50, "Alpha City", Some("CZ")),
        (101, "S-1", 50, "Beta Corp", Some("CZ")),
        (102, "S-1", 50, "", None),
        (200, "S-1", 51, "One", Some("CZ")),
        (200, "S-2", 51, "Two", Some("CZ")),
        (300, "S-1", 53, "Buyer Org", Some("CZ")),
        (300, "S-2", 53, "Winner Co", Some("CZ")),
        (400, "S-1", 54, "Carried Co", Some("CZ")),
        (500, "S-1", 55, "Buyer B", Some("CZ")),
        (500, "S-2", 55, "Winner W", Some("CZ")),
        // Org 56 — the eForms flagship shape: mentioned under DIFFERENT
        // names on two notices of one tender chain (defeats tier 3), org
        // sections siblings not RES-descendants (defeats descent), but
        // exactly ONE mention per origin notice (tier 4b).
        (600, "ORG-1", 56, "Name Alpha", Some("SI")),
        (601, "ORG-1", 56, "Name Beta", Some("SI")),
        // Org 57 — the eForms multi-mention origin (org 15176's measured
        // blocking shape): buyer and winner BOTH placeholder-merged as
        // sibling sections; only the party roles pin the winner (tier 4c).
        (700, "S-1", 57, "Buyer Sib", Some("DE")),
        (700, "S-2", 57, "Winner Sib", Some("DE")),
    ] {
        conn.execute(
            "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, country)
             VALUES (?, ?, ?, ?, ?)",
            (
                Value::Integer(nid),
                Value::Text(sid.into()),
                Value::Integer(org),
                Value::Text(name.into()),
                match country {
                    Some(c) => Value::Text(c.into()),
                    None => Value::Null,
                },
            ),
        )
        .await
        .unwrap();
    }
    // Tier 5 needs the stamp+requeue observable: tender 2's row (stamped
    // epoch-stale) and its causing notice 200 (re-queued, projected 1 → 0).
    conn.execute(
        &format!(
            "INSERT INTO tenders (id, source, kind, created_at, projection_epoch)
             VALUES (2, 'ted', 'procedure', 0, {})",
            store::canonical::PROJECTION_EPOCH
        ),
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO notices (id, source, publication_id, content_hash, profile, fetch_id,
                              member_path, ingested_at, parse_state, projected)
         VALUES (200, 'ted', 'pub-200', 'h200', 'eforms:test', 1, 'p', 0, 'parsed', 1)",
        (),
    )
    .await
    .unwrap();
    // Versions: tender 1 caused by notice 100 (org 50's winner context),
    // tender 2 caused by notice 200 (org 51's — ambiguous).
    for (t, seq, n) in
        [(1i64, 1i64, 100i64), (2, 1, 200), (3, 1, 300), (4, 1, 400), (4, 2, 401), (5, 1, 500), (6, 1, 600), (6, 2, 601), (7, 1, 701)]
    {
        conn.execute(
            "INSERT INTO tender_versions (tender_id, seq, caused_by_notice_id, publication_id, published_at)
             VALUES (?, ?, ?, ?, 0)",
            (Value::Integer(t), Value::Integer(seq), Value::Integer(n), Value::Text(format!("pub-{t}-{seq}"))),
        )
        .await
        .unwrap();
    }
    // lot_results parents for the winner rows — the dissolve's target-row
    // INSERT runs on the Db connection, where foreign_keys is ON.
    for lr in [10i64, 11, 12, 13, 14] {
        conn.execute(
            "INSERT INTO lot_results (id, tender_id, notice_id, result_key)
             VALUES (?, 1, 100, ?)",
            (Value::Integer(lr), Value::Text(format!("RES-{lr}"))),
        )
        .await
        .unwrap();
    }
    // Org 55's lot_result carries its ORIGIN (notice 500, RES-1) — tier 4's
    // input. The origin notice's section tree: S-2 sits UNDER RES-1, S-1
    // (the buyer) does not.
    conn.execute(
        "INSERT INTO lot_results (id, tender_id, notice_id, result_key)
         VALUES (15, 5, 500, 'RES-1')",
        (),
    )
    .await
    .unwrap();
    // Org 56's winner originates on notice 600 (single mention there).
    conn.execute(
        "INSERT INTO lot_results (id, tender_id, notice_id, result_key)
         VALUES (16, 6, 600, 'RES-0001')",
        (),
    )
    .await
    .unwrap();
    // Org 57's winner originates on 700; the causing notice 701 mentions
    // nothing (carried), the origin holds TWO sibling mentions.
    conn.execute(
        "INSERT INTO lot_results (id, tender_id, notice_id, result_key)
         VALUES (17, 7, 700, 'RES-0001')",
        (),
    )
    .await
    .unwrap();
    for (sid, kind) in [("RES-0001", "LotResult"), ("S-1", "Organization"), ("S-2", "Organization")] {
        conn.execute(
            "INSERT INTO notice_sections (notice_id, section_id, kind, parent_section_id)
             VALUES (700, ?, ?, NULL)",
            (Value::Text(sid.into()), Value::Text(kind.into())),
        )
        .await
        .unwrap();
    }
    // The role signal on the ORIGIN notice: S-1 buyer-family, S-2 winner.
    conn.execute(
        "INSERT INTO tender_version_parties (tender_id, seq, role, organization_id, mention_notice_id, mention_section_id)
         VALUES (7, 1, 'Procedure-Buyer', 57, 700, 'S-1')",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO tender_version_parties (tender_id, seq, role, organization_id, mention_notice_id, mention_section_id)
         VALUES (7, 1, 'winner', 57, 700, 'S-2')",
        (),
    )
    .await
    .unwrap();
    for (sid, kind, parent) in
        [("RES-1", "LotResult", None), ("S-1", "Organization", None), ("S-2", "Organization", Some("RES-1"))]
    {
        conn.execute(
            "INSERT INTO notice_sections (notice_id, section_id, kind, parent_section_id)
             VALUES (500, ?, ?, ?)",
            (
                Value::Text(sid.into()),
                Value::Text(kind.into()),
                match parent {
                    Some(x) => Value::Text(x.into()),
                    None => Value::Null,
                },
            ),
        )
        .await
        .unwrap();
    }
    // Party row rides mention (100, S-1); bid-party rides (101, S-1).
    conn.execute(
        "INSERT INTO tender_version_parties (tender_id, seq, role, organization_id, mention_notice_id, mention_section_id)
         VALUES (1, 1, 'buyer', 50, 100, 'S-1')",
        (),
    )
    .await
    .unwrap();
    // Tier-2 signal on org 53: S-1 is buyer-family, S-2 is not (issue 309).
    conn.execute(
        "INSERT INTO tender_version_parties (tender_id, seq, role, organization_id, mention_notice_id, mention_section_id)
         VALUES (3, 1, 'Procedure-Buyer', 53, 300, 'S-1')",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO tender_version_parties (tender_id, seq, role, organization_id, mention_notice_id, mention_section_id)
         VALUES (3, 1, 'winner', 53, 300, 'S-2')",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO tender_version_bid_parties (tender_id, seq, bid_id, role, organization_id, mention_notice_id, mention_section_id)
         VALUES (1, 1, 7, 'tenderer', 50, 101, 'S-1')",
        (),
    )
    .await
    .unwrap();
    // Winner rows: lot_result 10 moves cleanly to org 60; lot_result 11
    // already has a row on 60, so the move is a duplicate removal. Tender 2's
    // winner sits on the ambiguous org 51.
    // Org 54: winner on seq 1 (tier 1) AND carried forward onto seq 2, whose
    // causing notice 401 never mentions it (tier 3 — the rounds-accumulate
    // shape that skipped the whole flagship set on prod).
    for (t, seq, lr, org) in [
        (1i64, 1i64, 10i64, 50i64),
        (1, 1, 11, 50),
        (1, 1, 11, 60),
        (2, 1, 12, 51),
        (3, 1, 13, 53),
        (4, 1, 14, 54),
        (4, 2, 14, 54),
        (5, 1, 15, 55),
        // Carried onto seq 2 (caused by 601, where org 56 is "Name Beta" —
        // two distinct chain names): only tier 4b resolves, via origin 600.
        (6, 2, 16, 56),
        (7, 1, 17, 57),
    ] {
        conn.execute(
            "INSERT INTO tender_version_result_winners (tender_id, seq, lot_result_id, organization_id)
             VALUES (?, ?, ?, ?)",
            (Value::Integer(t), Value::Integer(seq), Value::Integer(lr), Value::Integer(org)),
        )
        .await
        .unwrap();
    }
    // Issue 484 unit 3: lot_result 10's winner IS its notice's own buyer — the repoint
    // must carry the flag, not re-insert the key columns alone. On 11 the TARGET row
    // (org 60) is flagged and the collapsing source (org 50) is not — the survivor's
    // flag must be cleared (the fold's every-section rule).
    conn.execute(
        "UPDATE tender_version_result_winners SET is_buyer = 1 \
          WHERE lot_result_id = 10 OR (lot_result_id = 11 AND organization_id = 60)",
        (),
    )
    .await
    .unwrap();
    conn.execute("COMMIT", ()).await.unwrap();
    (db, conn)
}

async fn count(conn: &store::turso::Connection, sql: &str) -> i64 {
    let mut rows = conn.query(sql, ()).await.unwrap();
    let row = rows.next().await.unwrap().unwrap();
    let store::turso::Value::Integer(n) = row.get_value(0).unwrap() else { panic!("count") };
    n
}

#[tokio::test]
async fn the_dissolve_splits_condemned_orgs_and_queues_ambiguous_winners_for_refold() {
    let (db, conn) = seed("test-dissolve.db").await;

    // Dry-run: full preview, nothing written.
    let (dry, _) = db.repair_placeholder_orgs_batch(bad, FIELDS, normalize, n2, CAP, 10_000, 0, true).await.expect("dry");
    assert_eq!((dry.scanned, dry.condemned), (8, 7), "50, 51, 53-57 condemned, 52 clean");
    assert_eq!((dry.parties, dry.bid_parties), (5, 1), "dry run previews the blast radius");
    assert_eq!(
        (dry.dissolved, dry.skipped),
        (7, 0),
        "51 t5 refold; 53 t2; 54 t3; 55 t4 descent; 56 t4b origin; 57 t4c roles"
    );
    assert_eq!(dry.mentions, 14, "50:3 + 51:2 + 53:2 + 54:1 + 55:2 + 56:2 + 57:2");
    assert_eq!(dry.variant_orgs, 14, "issue 542: 60 reused + 13 fresh mints, an upper bound");
    assert_eq!(dry.variants_written, 0, "dry run writes no name variants");
    assert_eq!((dry.fresh, dry.reused), (13, 1), "all named fresh but Alpha City reuses 60");
    assert_eq!(
        (dry.winners_deleted, dry.refold_tenders, dry.refold_notices),
        (1, 7, 1),
        "tier 5 previews the ambiguous row; issue 579: every touched tender (1-7) goes to the \
         refold, and the requeue reaches the one notice this seed records"
    );
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE id = 50").await, 1);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations").await, 9, "dry run minted nothing");
    assert_eq!(count(&conn, "SELECT projected FROM notices WHERE id = 200").await, 1);
    assert_eq!(
        count(&conn, "SELECT projection_epoch FROM tenders WHERE id = 2").await,
        store::canonical::PROJECTION_EPOCH,
        "dry run stamped nothing"
    );

    // Wet run.
    let (wet, _) = db.repair_placeholder_orgs_batch(bad, FIELDS, normalize, n2, CAP, 10_000, 0, false).await.expect("wet");
    assert_eq!((wet.dissolved, wet.skipped), (7, 0));
    assert_eq!((wet.fresh, wet.reused), (13, 1));
    assert_eq!((wet.parties, wet.bid_parties), (5, 1));
    assert_eq!(wet.winners, 8, "50:2 + 53:1 + 54:2 + 55:1 + 56:1 + 57:1");
    assert_eq!(wet.winner_dups, 1, "lot_result 11 already stood on org 60");
    assert_eq!(
        (wet.winners_deleted, wet.refold_tenders, wet.refold_notices),
        (1, 7, 1),
        "preview-exact: the wet tier-5 and refold counts match the dry ones"
    );
    assert_eq!(wet.tender_changes, 7, "tenders 1-7 all touched — 2 via the tier-5 delete");
    assert_eq!(wet.variant_orgs, 14, "issue 542: 60 plus the 13 fresh provisionals");
    assert_eq!(wet.variants_written, 0, "this seed carries no labelled name texts");

    // Org 50 is gone; 51 dissolved via tier 5; the Alpha City mention sits on
    // 60; Beta Corp and the nameless mention sit on fresh provisionals.
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE id = 50").await, 0);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE id = 51").await, 0);
    assert_eq!(
        count(&conn, "SELECT organization_id FROM organization_mentions WHERE notice_id = 100").await,
        60
    );
    let beta = count(&conn, "SELECT organization_id FROM organization_mentions WHERE notice_id = 101").await;
    assert!(beta > 60, "Beta Corp minted fresh (id {beta})");
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM organizations WHERE name = 'Beta Corp' AND provisional = 1 AND identifier IS NULL").await,
        1
    );
    // Rows followed their mentions.
    assert_eq!(
        count(&conn, "SELECT organization_id FROM tender_version_parties WHERE tender_id = 1").await,
        60
    );
    assert_eq!(
        count(&conn, "SELECT organization_id FROM tender_version_bid_parties WHERE tender_id = 1").await,
        beta
    );
    // Winners: lot_result 10 moved to 60; 11 deduped to ONE row on 60; the
    // ambiguous tender-2 row is DELETED (tier 5) — the refold re-derives it.
    assert_eq!(
        count(&conn, "SELECT organization_id FROM tender_version_result_winners WHERE lot_result_id = 10").await,
        60
    );
    assert_eq!(
        count(&conn, "SELECT is_buyer FROM tender_version_result_winners WHERE lot_result_id = 10").await,
        1,
        "issue 484: the repointed row keeps its own is_buyer"
    );
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM tender_version_result_winners WHERE lot_result_id <> 10 AND is_buyer IS NOT NULL").await,
        0,
        "…an unflagged row stays NULL, and 11's flagged target is cleared by its unflagged duplicate"
    );
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM tender_version_result_winners WHERE lot_result_id = 11").await,
        1
    );
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM tender_version_result_winners WHERE lot_result_id = 12").await,
        0,
        "the honestly ambiguous winner row is gone, not guessed"
    );
    // The tier-5 mentions re-resolved per-section like any other: two fresh
    // provisionals carry tender 2's notice-200 bindings for the refold to read.
    let one = count(&conn, "SELECT organization_id FROM organization_mentions WHERE notice_id = 200 AND section_id = 'S-1'").await;
    let two = count(&conn, "SELECT organization_id FROM organization_mentions WHERE notice_id = 200 AND section_id = 'S-2'").await;
    assert!(one > 60 && two > 60 && one != two, "per-section fresh provisionals ({one}, {two})");
    // The issue-179 pair landed: tender 2 epoch-stale, notice 200 re-queued —
    // the next incremental fold rewrites tender 2's winner set wholesale from
    // the re-bound mentions (delete_version + write_version, keep = 0).
    assert_eq!(
        count(&conn, "SELECT projection_epoch FROM tenders WHERE id = 2").await,
        0,
        "tender 2 stamped epoch-stale"
    );
    assert_eq!(
        count(&conn, "SELECT projected FROM notices WHERE id = 200").await,
        0,
        "causing notice 200 re-queued for the delta"
    );
    // The change feed heard about all of it.
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM changes WHERE entity_kind = 'organization' AND entity_id = 50 AND op = 'removed'").await,
        1
    );
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM changes WHERE entity_kind = 'organization' AND entity_id = 60 AND op = 'changed'").await,
        1
    );
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM changes WHERE entity_kind = 'tender' AND entity_id = 1 AND op = 'changed'").await,
        1
    );
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM changes WHERE entity_kind = 'organization' AND entity_id = 51 AND op = 'removed'").await,
        1,
        "the tier-5 org's removal reached the feed"
    );
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM changes WHERE entity_kind = 'tender' AND entity_id = 2 AND op = 'changed'").await,
        1,
        "the tier-5 tender's winner change reached the feed"
    );

    // The tier-2 winner followed S-2's target, not the buyer's.
    let winner_co = count(&conn, "SELECT id FROM organizations WHERE name = 'Winner Co'").await;
    assert!(winner_co > 60);
    assert_eq!(
        count(&conn, "SELECT organization_id FROM tender_version_result_winners WHERE lot_result_id = 13").await,
        winner_co
    );
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE id = 53").await, 0);

    // The carried-forward winner rows both followed Carried Co's target.
    let carried = count(&conn, "SELECT id FROM organizations WHERE name = 'Carried Co'").await;
    assert!(carried > 60);
    assert_eq!(
        count(&conn, &format!("SELECT COUNT(*) FROM tender_version_result_winners WHERE lot_result_id = 14 AND organization_id = {carried}")).await,
        2,
        "both seq rows moved to Carried Co's target"
    );
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM tender_version_result_winners WHERE lot_result_id = 14 AND organization_id = 54").await,
        0,
        "no lot_result-14 rows remain on the dissolved org"
    );
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE id = 54").await, 0);

    // Tier 4: the winner followed the RES-descendant section's target
    // ("Winner W"), not the buyer's.
    let winner_w = count(&conn, "SELECT id FROM organizations WHERE name = 'Winner W'").await;
    assert_eq!(
        count(&conn, "SELECT organization_id FROM tender_version_result_winners WHERE lot_result_id = 15").await,
        winner_w
    );
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE id = 55").await, 0);

    // Tier 4b: the carried eForms-shape winner followed origin 600's single
    // mention ("Name Alpha"), not seq-2's "Name Beta".
    let alpha = count(&conn, "SELECT id FROM organizations WHERE name = 'Name Alpha'").await;
    assert_eq!(
        count(&conn, "SELECT organization_id FROM tender_version_result_winners WHERE lot_result_id = 16").await,
        alpha
    );
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE id = 56").await, 0);

    // Tier 4c: the sibling-pair winner followed the winner-role section.
    let winner_sib = count(&conn, "SELECT id FROM organizations WHERE name = 'Winner Sib'").await;
    assert_eq!(
        count(&conn, "SELECT organization_id FROM tender_version_result_winners WHERE lot_result_id = 17").await,
        winner_sib
    );
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE id = 57").await, 0);

    // A rerun finds every dissolved org gone: only clean 52 stays in scope.
    let (again, _) = db.repair_placeholder_orgs_batch(bad, FIELDS, normalize, n2, CAP, 10_000, 0, false).await.expect("rerun");
    assert_eq!((again.condemned, again.dissolved, again.skipped), (0, 0, 0));
}

/// Issue 542: a wet window re-derives the name variants of every org its mentions landed
/// on, from all of that org's mentions and inside its own transaction, so no
/// whole-corpus `backfill-org-name-variants` walk has to follow. Orgs it did not
/// re-bind keep their rows, even when they are mentioned on a re-bound notice.
#[tokio::test]
async fn the_dissolve_rederives_the_name_variants_of_the_orgs_it_rebinds() {
    let path = "test-dissolve-variants.db";
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(path).await.unwrap();
    let raw = store::turso::Builder::new_local(path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();

    // 70 merges two strangers under a condemned identifier; 80 is the standing
    // provisional "Alpha City" must reuse; 90 is a clean org co-mentioned on notice 100.
    for (id, ident, name, prov) in
        [(70, Some("BAD"), "Merger", 0), (80, None, "Alpha City", 1), (90, Some("12345678"), "Co Org", 0)]
    {
        conn.execute(
            "INSERT INTO organizations (id, country, name, name_norm, provisional, identifier_kind, identifier, created_at)
             VALUES (?, 'CZ', ?, ?, ?, CASE WHEN ? IS NULL THEN NULL ELSE 'national' END, ?, 0)",
            (
                Value::Integer(id),
                Value::Text(name.into()),
                Value::Text(name.to_lowercase()),
                Value::Integer(prov),
                ident.map_or(Value::Null, |s| Value::Text(s.into())),
                ident.map_or(Value::Null, |s| Value::Text(s.into())),
            ),
        )
        .await
        .unwrap();
    }
    // Inserted out of key order on purpose, so rowid order cannot stand in for the
    // (notice, section) order the re-derivation must apply.
    for (nid, sid, org, name) in [
        // 80's own LATER mention: its text, not notice 100's, is the one the walk leaves.
        (900, "S-1", 80, "Alpha City"),
        (100, "S-1", 70, "Alpha City"),
        (101, "S-1", 70, "Beta Corp"),
        (100, "S-2", 90, "Co Org"),
    ] {
        conn.execute(
            "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, country)
             VALUES (?, ?, ?, ?, 'CZ')",
            (Value::Integer(nid), Value::Text(sid.into()), Value::Integer(org), Value::Text(name.into())),
        )
        .await
        .unwrap();
    }
    for (nid, sid, ord, lang, value) in [
        (900, "S-1", 0, Some("DE"), "Alpha City Later"),
        (100, "S-1", 0, Some("DE"), "Alpha City Alt"),
        // Two French texts on one section, the later ordinal inserted first: the first
        // one in key order (field, then ordinal) wins, not the first row written.
        (101, "S-1", 1, Some("FR"), "Beta Second"),
        (101, "S-1", 0, Some("FR"), "Beta Corp SA"),
        (101, "S-1", 2, None, "Beta Unlabelled"),
        (100, "S-2", 0, Some("DE"), "Co Org Old"),
    ] {
        conn.execute(
            "INSERT INTO notice_texts (notice_id, section_id, field_id, ordinal, lang, value)
             VALUES (?, ?, 'TED-OFFICIALNAME', ?, ?, ?)",
            (
                Value::Integer(nid),
                Value::Text(sid.into()),
                Value::Integer(ord),
                lang.map_or(Value::Null, |l| Value::Text(l.into())),
                Value::Text(value.into()),
            ),
        )
        .await
        .unwrap();
    }
    // 80's FRA row has no labelled text behind it: REPLACE never deletes, so it stays.
    for (org, lang, name) in [
        (70, "DEU", "Merger"),
        (90, "DEU", "Co Org Current"),
        (80, "DEU", "Alpha City Old"),
        (80, "FRA", "Ville Alpha"),
    ] {
        conn.execute(
            "INSERT INTO organization_names (org_id, lang, name, name_norm) VALUES (?, ?, ?, ?)",
            (Value::Integer(org), Value::Text(lang.into()), Value::Text(name.into()), Value::Text(name.to_lowercase())),
        )
        .await
        .unwrap();
    }

    let (dry, _) = db.repair_placeholder_orgs_batch(bad, FIELDS, normalize, n2, CAP, 10_000, 0, true).await.expect("dry");
    assert_eq!((dry.condemned, dry.mentions, dry.fresh, dry.reused), (1, 2, 1, 1));
    assert_eq!((dry.variant_orgs, dry.variants_written), (2, 0), "80 reused + one fresh mint, nothing written");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organization_names").await, 4, "dry run wrote nothing");

    // The re-derivation commits with its window or not at all: a change event that fails
    // AFTER it (80 is reused, so its `changed` event follows the re-derivation) rolls the
    // whole window back, satellite writes included.
    conn.execute(
        "CREATE TRIGGER t542 BEFORE INSERT ON changes WHEN NEW.entity_kind = 'organization' AND NEW.op = 'changed'
         BEGIN SELECT RAISE(ABORT, 'fixture: the change feed refuses'); END",
        (),
    )
    .await
    .unwrap();
    let err = db.repair_placeholder_orgs_batch(bad, FIELDS, normalize, n2, CAP, 10_000, 0, false).await.expect_err("refused");
    assert!(err.to_string().contains("fixture: the change feed refuses"), "{err}");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations").await, 3, "70 still stands, nothing minted");
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM organization_names WHERE org_id = 80 AND lang = 'DEU' AND name = 'Alpha City Old'").await,
        1,
        "the re-derivation rolled back with the window"
    );
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organization_names").await, 4);
    conn.execute("DROP TRIGGER t542", ()).await.unwrap();

    let (wet, _) = db.repair_placeholder_orgs_batch(bad, FIELDS, normalize, n2, CAP, 10_000, 0, false).await.expect("wet");
    assert_eq!((wet.dissolved, wet.mentions), (1, 2));
    assert_eq!((wet.variant_orgs, wet.variants_written), (2, 2), "80's DEU and Beta's FRA");
    let beta = count(&conn, "SELECT organization_id FROM organization_mentions WHERE notice_id = 101").await;
    assert!(beta > 90, "Beta Corp minted fresh (id {beta})");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organization_names WHERE org_id = 70").await, 0, "died with 70");
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM organization_names WHERE org_id = 80 AND lang = 'DEU' AND name = 'Alpha City Later'").await,
        1,
        "the reused target keeps its latest notice's text, not the re-bound notice's"
    );
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM organization_names WHERE org_id = 80 AND lang = 'FRA' AND name = 'Ville Alpha'").await,
        1,
        "a language with no labelled text keeps its row"
    );
    assert_eq!(
        count(&conn, &format!("SELECT COUNT(*) FROM organization_names WHERE org_id = {beta} AND lang = 'FRA' AND name = 'Beta Corp SA'")).await,
        1
    );
    assert_eq!(count(&conn, &format!("SELECT COUNT(*) FROM organization_names WHERE org_id = {beta}")).await, 1);
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM organization_names WHERE org_id = 90 AND name = 'Co Org Current'").await,
        1,
        "a co-mentioned org the dissolve did not re-bind is left alone"
    );

    drop(conn);
    drop(db);
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

/// Issue 542: the re-derivation's per-org read runs inside a dissolve window's write
/// transaction, so its plan is pinned: the org's mentions through
/// `organization_mentions_org`, each section's texts through the `notice_texts` primary
/// key, never a walk of either table.
#[tokio::test]
async fn the_name_variant_rederivation_seeks_both_indexes() {
    let (db, _conn) = seed("test-dissolve-plan.db").await;
    let sql = store::canonical::org_name_rederive_sql(1)
        .replace("om.organization_id = ?", "om.organization_id = 50")
        .replace("IN (?)", "IN ('TED-OFFICIALNAME')");
    assert!(!sql.contains('?'), "every parameter is inlined: {sql}");
    let plan = db
        .measure_rows(&format!("EXPLAIN QUERY PLAN {sql}"))
        .await
        .unwrap()
        .into_iter()
        .map(|row| match row.get(3) {
            Some(Value::Text(t)) => t.clone(),
            other => format!("{other:?}"),
        })
        .collect::<Vec<_>>()
        .join(" | ");
    println!("rederive plan: {plan}");
    assert!(plan.contains("organization_mentions_org"), "the mentions come through the org index: {plan}");
    assert!(
        plan.contains("sqlite_autoindex_notice_texts") || plan.contains("PRIMARY KEY"),
        "each section's texts come through the notice_texts key: {plan}"
    );
    for table in ["om", "nt", "organization_mentions", "notice_texts"] {
        assert!(!plan.contains(&format!("SCAN {table} ")) && !plan.ends_with(&format!("SCAN {table}")), "{table} is walked: {plan}");
    }
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("test-dissolve-plan.db{s}"));
    }
}

// ---- Issue 578: the dissolve binds a mention where the resolver's no-identifier branch does ----

/// A fresh file with the Db open, and a raw connection that seeds with foreign keys off.
async fn open(path: &str) -> (store::Db, store::turso::Connection) {
    cleanup(path);
    let db = store::Db::open(path).await.unwrap();
    let raw = store::turso::Builder::new_local(path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    (db, conn)
}

fn cleanup(path: &str) {
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

fn opt(s: Option<&str>) -> Value {
    s.map_or(Value::Null, |s| Value::Text(s.into()))
}

async fn org(conn: &store::turso::Connection, id: i64, country: Option<&str>, identifier: Option<&str>, name: &str) {
    conn.execute(
        "INSERT INTO organizations (id, country, name, name_norm, provisional, identifier_kind, identifier, created_at)
         VALUES (?, ?, ?, ?, ?, CASE WHEN ? IS NULL THEN NULL ELSE 'national' END, ?, 0)",
        (
            Value::Integer(id),
            opt(country),
            Value::Text(name.into()),
            Value::Text(name.to_lowercase()),
            Value::Integer(identifier.is_none() as i64),
            opt(identifier),
            opt(identifier),
        ),
    )
    .await
    .unwrap();
}

async fn mention(conn: &store::turso::Connection, notice: i64, section: &str, org: i64, name: &str, country: Option<&str>) {
    conn.execute(
        "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, country) VALUES (?, ?, ?, ?, ?)",
        (Value::Integer(notice), Value::Text(section.into()), Value::Integer(org), Value::Text(name.into()), opt(country)),
    )
    .await
    .unwrap();
}

async fn text_of(conn: &store::turso::Connection, sql: &str) -> Option<String> {
    let mut rows = conn.query(sql, ()).await.unwrap();
    match rows.next().await.unwrap().unwrap().get_value(0).unwrap() {
        Value::Text(s) => Some(s),
        Value::Null => None,
        other => panic!("text: {other:?}"),
    }
}

/// An overseas mention binds the register country's lowest-id identifier-NULL row, as a
/// fold or a rebuild does: never the pre-358 row under the published code (440's shape
/// before the fix: 30920766, MQ, beside the FR row 2660729), nor the identified twin.
#[tokio::test]
async fn an_overseas_mention_binds_the_lowest_id_register_row() {
    let path = "test-dissolve-578-register.db";
    let (db, conn) = open(path).await;
    org(&conn, 50, Some("MQ"), Some("BAD"), "Rectorat de Martinique").await;
    // The SIRET twin sorts first on purpose: only `identifier IS NULL` keeps the mention off it.
    org(&conn, 69, Some("FR"), Some("17972430700030"), "Rectorat de Martinique").await;
    org(&conn, 70, Some("MQ"), None, "Rectorat de Martinique").await;
    org(&conn, 71, Some("FR"), None, "Rectorat de Martinique").await;
    org(&conn, 72, Some("FR"), None, "Rectorat de Martinique").await;
    mention(&conn, 100, "ORG-1", 50, "rectorat de Martinique", Some("MQ")).await;

    let (dry, _) = db.repair_placeholder_orgs_batch(bad, FIELDS, normalize, n2, CAP, 10_000, 0, true).await.expect("dry");
    assert_eq!((dry.condemned, dry.dissolved, dry.fresh, dry.reused), (1, 1, 0, 1));
    let (wet, _) = db.repair_placeholder_orgs_batch(bad, FIELDS, normalize, n2, CAP, 10_000, 0, false).await.expect("wet");
    assert_eq!((wet.dissolved, wet.fresh, wet.reused), (1, 0, 1));
    assert_eq!(count(&conn, "SELECT organization_id FROM organization_mentions WHERE notice_id = 100").await, 71);
    assert_eq!(
        text_of(&conn, "SELECT country FROM organization_mentions WHERE notice_id = 100").await.as_deref(),
        Some("MQ"),
        "the mention keeps the code the notice published"
    );
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations").await, 4, "50 gone, nothing minted");
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM changes WHERE entity_kind = 'organization' AND entity_id = 71 AND op = 'changed'").await,
        1
    );
    drop(conn);
    drop(db);
    cleanup(path);
}

/// Overseas mentions with no register-country row mint ONE row under the register's
/// country for the window, across orgs; the pre-358 row under the published code stays
/// unused, a nameless overseas mention mints under the register's country too, and the
/// dry run counts exactly what the wet run mints.
#[tokio::test]
async fn overseas_mentions_mint_one_register_row_per_key_dry_and_wet_alike() {
    let path = "test-dissolve-578-mint.db";
    let (db, conn) = open(path).await;
    org(&conn, 50, Some("RE"), Some("BAD"), "Merger A").await;
    org(&conn, 51, Some("RE"), Some("BAD"), "Merger B").await;
    org(&conn, 70, Some("RE"), None, "Region Reunion").await;
    mention(&conn, 100, "ORG-1", 50, "Region Reunion", Some("RE")).await;
    mention(&conn, 101, "ORG-1", 50, "Region Reunion", Some("RE")).await;
    mention(&conn, 102, "ORG-1", 50, "", Some("RE")).await;
    mention(&conn, 103, "ORG-1", 51, "Region Reunion", Some("RE")).await;

    let (dry, _) = db.repair_placeholder_orgs_batch(bad, FIELDS, normalize, n2, CAP, 10_000, 0, true).await.expect("dry");
    assert_eq!((dry.dissolved, dry.mentions), (2, 4));
    assert_eq!((dry.fresh, dry.reused), (2, 0), "one mint for the key, one for the nameless mention");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations").await, 3, "dry run minted nothing");
    let (wet, _) = db.repair_placeholder_orgs_batch(bad, FIELDS, normalize, n2, CAP, 10_000, 0, false).await.expect("wet");
    assert_eq!((wet.dissolved, wet.mentions, wet.fresh, wet.reused), (2, 4, 2, 0), "the dry counts were the wet run's");
    assert_eq!(wet.variant_orgs, dry.variant_orgs, "the dry preview of the re-bound orgs is exact");

    let region = count(
        &conn,
        "SELECT id FROM organizations WHERE name_norm = 'region reunion' AND country = 'FR' AND identifier IS NULL AND provisional = 1",
    )
    .await;
    assert_eq!(
        count(&conn, &format!("SELECT COUNT(*) FROM organization_mentions WHERE organization_id = {region}")).await,
        3,
        "both orgs' named mentions on the one FR row"
    );
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organization_mentions WHERE organization_id = 70").await, 0, "the RE row stays unused");
    let nameless = count(&conn, "SELECT organization_id FROM organization_mentions WHERE notice_id = 102").await;
    assert!(nameless != region && nameless > 70, "the nameless mention minted its own row ({nameless})");
    assert_eq!(text_of(&conn, &format!("SELECT country FROM organizations WHERE id = {nameless}")).await.as_deref(), Some("FR"));
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM changes WHERE entity_kind = 'organization' AND op = 'added'").await, 2);
    drop(conn);
    drop(db);
    cleanup(path);
}

/// A named country-less mention goes through the issue-351 wall: a name under it reuses its
/// standing NULL-country row, or mints one row per name; a generic name (over the wall,
/// nothing identified) mints a row per mention, never reusing a standing row of that name.
#[tokio::test]
async fn a_country_less_name_reuses_its_standing_row_under_the_wall_and_mints_over_it() {
    let path = "test-dissolve-578-countryless.db";
    let (db, conn) = open(path).await;
    org(&conn, 50, None, Some("BAD"), "Merger").await;
    org(&conn, 80, None, None, "Stadt Burghausen").await;
    // "gemeindetaufkirchen" is carried by three rows: over a cap of 2, none identified.
    for id in [9001, 9002, 9003] {
        org(&conn, id, None, None, "Gemeinde Taufkirchen").await;
        conn.execute(
            "INSERT INTO org_match_keys (org_id, key_kind, key) VALUES (?, 'n2', 'gemeindetaufkirchen')",
            (Value::Integer(id),),
        )
        .await
        .unwrap();
    }
    for (notice, name) in [
        (100, "Stadt Burghausen"),
        (101, "Stadt Burghausen"),
        (102, "Gemeinde Taufkirchen"),
        (103, "Gemeinde Taufkirchen"),
        (104, "Markt Neu"),
        (105, "Markt Neu"),
    ] {
        mention(&conn, notice, "ORG-1", 50, name, None).await;
    }

    let (dry, _) = db.repair_placeholder_orgs_batch(bad, FIELDS, normalize, n2, CAP, 10_000, 0, true).await.expect("dry");
    assert_eq!((dry.mentions, dry.fresh, dry.reused), (6, 3, 2), "Taufkirchen twice, Markt Neu once; Burghausen reused twice");
    let (wet, _) = db.repair_placeholder_orgs_batch(bad, FIELDS, normalize, n2, CAP, 10_000, 0, false).await.expect("wet");
    assert_eq!((wet.dissolved, wet.mentions, wet.fresh, wet.reused), (1, 6, 3, 2));

    let on = |notice: i64| format!("SELECT organization_id FROM organization_mentions WHERE notice_id = {notice}");
    assert_eq!(count(&conn, &on(100)).await, 80, "under the wall: the standing row");
    assert_eq!(count(&conn, &on(101)).await, 80);
    let (t1, t2) = (count(&conn, &on(102)).await, count(&conn, &on(103)).await);
    assert!(t1 > 9003 && t2 > 9003 && t1 != t2, "generic: a fresh row per mention ({t1}, {t2})");
    assert_eq!(count(&conn, &on(104)).await, count(&conn, &on(105)).await, "under the wall, no standing row: one mint per name");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE name_norm = 'markt neu' AND country IS NULL").await, 1);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE name_norm = 'gemeinde taufkirchen'").await, 5);
    drop(conn);
    drop(db);
    cleanup(path);
}

/// A country-less name whose issue-351 probe FAILS (here at prepare, which ends no
/// transaction) mints, the resolver's safe failure: it reads as generic, a fresh NULL-country
/// row per mention and never the standing row, and the window commits.
#[tokio::test]
async fn a_failed_country_less_probe_mints_and_the_window_commits() {
    let path = "test-dissolve-578-probe-failed.db";
    let (db, conn) = open(path).await;
    org(&conn, 50, None, Some("BAD"), "Merger").await;
    org(&conn, 80, None, None, "Stadt Burghausen").await;
    mention(&conn, 100, "ORG-1", 50, "Stadt Burghausen", None).await;
    mention(&conn, 101, "ORG-1", 50, "Stadt Burghausen", None).await;
    // The wall's carrier read then fails; nothing else the dissolve runs reads this table.
    conn.execute("DROP TABLE org_match_keys", ()).await.unwrap();

    let (dry, _) = db.repair_placeholder_orgs_batch(bad, FIELDS, normalize, n2, CAP, 10_000, 0, true).await.expect("dry");
    assert_eq!((dry.fresh, dry.reused), (2, 0));
    let (wet, _) = db.repair_placeholder_orgs_batch(bad, FIELDS, normalize, n2, CAP, 10_000, 0, false).await.expect("wet");
    assert_eq!((wet.dissolved, wet.mentions, wet.fresh, wet.reused), (1, 2, 2, 0));

    let on = |notice: i64| format!("SELECT organization_id FROM organization_mentions WHERE notice_id = {notice}");
    let (a, b) = (count(&conn, &on(100)).await, count(&conn, &on(101)).await);
    assert!(a > 80 && b > 80 && a != b, "a fresh row per mention, not one per name ({a}, {b})");
    for id in [a, b] {
        assert_eq!(text_of(&conn, &format!("SELECT country FROM organizations WHERE id = {id}")).await, None);
    }
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE id = 50").await, 0, "the window committed");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM changes WHERE entity_kind = 'organization' AND op = 'added'").await, 2);
    drop(conn);
    drop(db);
    cleanup(path);
}

/// A whole org something still names after its mentions moved refuses its window, and
/// nothing of the window is kept, the org dissolved before it included: (a) a party row
/// keyed to another org's mention is the residue refusal; (b) a winner whose tier-2 pick is
/// that party row has no target, a clear refusal in the dry run and the wet run alike
/// (before issue 578, a winner repointed onto id 0 and a foreign-key abort).
#[tokio::test]
async fn a_winner_following_a_foreign_party_row_refuses_the_window_and_keeps_nothing() {
    let path = "test-dissolve-578-refusal.db";
    let (db, conn) = open(path).await;
    org(&conn, 50, Some("CZ"), Some("BAD"), "Clean Merger").await;
    org(&conn, 51, Some("CZ"), Some("BAD"), "Foreign Pick").await;
    org(&conn, 61, Some("CZ"), None, "Other Body").await;
    mention(&conn, 100, "ORG-1", 50, "Alpha", Some("CZ")).await;
    mention(&conn, 300, "ORG-1", 51, "Buyer X", Some("CZ")).await;
    mention(&conn, 300, "ORG-2", 51, "Bidder Y", Some("CZ")).await;
    mention(&conn, 300, "ORG-3", 61, "Other Body", Some("CZ")).await;
    conn.execute(
        "INSERT INTO tender_versions (tender_id, seq, caused_by_notice_id, publication_id, published_at)
         VALUES (3, 1, 300, 'pub-3-1', 0)",
        (),
    )
    .await
    .unwrap();
    // A non-buyer party row on 51, keyed to 61's mention: it follows no mention 51 moves.
    conn.execute(
        "INSERT INTO tender_version_parties (tender_id, seq, role, organization_id, mention_notice_id, mention_section_id)
         VALUES (3, 1, 'winner', 51, 300, 'ORG-3')",
        (),
    )
    .await
    .unwrap();
    let untouched = || async {
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations").await, 3, "50 and 51 stand, no mint survives");
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM organization_mentions WHERE organization_id = 50").await, 1);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM organization_mentions WHERE organization_id = 51").await, 2);
        assert_eq!(count(&conn, "SELECT organization_id FROM tender_version_parties").await, 51);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM changes").await, 0);
    };

    let err = db.repair_placeholder_orgs_batch(bad, FIELDS, normalize, n2, CAP, 10_000, 0, false).await.expect_err("residue");
    assert!(err.to_string().contains("is still referenced after its mentions moved"), "{err}");
    untouched().await;

    // The winner originates on a notice that names neither of 51's mentions, so the causing
    // notice's two mentions go to tier 2, and its one non-buyer party row picks ORG-3.
    conn.execute("INSERT INTO lot_results (id, tender_id, notice_id, result_key) VALUES (13, 3, 299, 'RES-1')", ())
        .await
        .unwrap();
    conn.execute(
        "INSERT INTO tender_version_result_winners (tender_id, seq, lot_result_id, organization_id) VALUES (3, 1, 13, 51)",
        (),
    )
    .await
    .unwrap();
    for dry_run in [true, false] {
        let err = db
            .repair_placeholder_orgs_batch(bad, FIELDS, normalize, n2, CAP, 10_000, 0, dry_run)
            .await
            .expect_err("a winner of another org's mention");
        assert!(err.to_string().contains("follows mention (300, ORG-3), which is not one of its mentions"), "{err}");
        untouched().await;
    }
    assert_eq!(count(&conn, "SELECT organization_id FROM tender_version_result_winners").await, 51);
    drop(conn);
    drop(db);
    cleanup(path);
}

// ---- Issue 579: a wet dissolve leaves its Tenders where the next refold verifies them ----

/// The fold's `now`, after the award's publication.
const NOW: i64 = 1_767_441_600;

/// The award a fold writes for notice 900 with `winner` on its mention ORG-2 beside the
/// sibling 30 on ORG-3: both are winners, and RES-1 is `winner`'s.
fn award(winner: i64) -> TenderProjection {
    projection(900, "pk-579", "winner", winner, winner)
}

/// Notice 901's Tender, where `org` is only a party and a bidder: both results are the
/// sibling 30's, so no winner row names `org`.
fn party_only(org: i64) -> TenderProjection {
    projection(901, "pk-579-b", "tenderer", org, 30)
}

/// The Tender a fold writes for `notice` with `org` on its mention ORG-2 beside the sibling
/// 30 on ORG-3: tender-scope parties (the buyer 20, then both under `role`), RES-1 won by
/// `res1` and RES-2 by 30, and one bid on LOT-1 by both, ordered as the projection orders
/// them (the parties a `BTreeSet<Fact>`, each bid's parties sorted).
fn projection(notice: i64, procedure_key: &str, role: &str, org: i64, res1: i64) -> TenderProjection {
    let party = |role: &str, org: i64, section: &str| Fact::Party {
        role: role.into(),
        organization_id: org,
        notice_id: notice,
        section_id: section.into(),
    };
    let result = |key: &str, lot: &str, org: i64| LotResultState {
        key: key.into(),
        lot_key: Some(lot.into()),
        decision: None,
        reason: None,
        awarded_cents: None,
        awarded_currency: None,
        decided: None,
        winners: vec![org],
        buyer_winners: Vec::new(),
        statistics: Vec::new(),
        published_lot_no: None,
        group: false,
    };
    let lot = |key: &str| LotState { key: key.into(), kind: "Lot".into(), facts: Default::default(), published_lot_no: None };
    let mut bidders = vec![
        BidParty { role: "tenderer".into(), organization_id: org, section_id: "ORG-2".into() },
        BidParty { role: "tenderer".into(), organization_id: 30, section_id: "ORG-3".into() },
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
            facts: [party("buyer", 20, "ORG-1"), party(role, org, "ORG-2"), party(role, 30, "ORG-3")]
                .into_iter()
                .collect(),
            lots: vec![lot("LOT-1"), lot("LOT-2")],
            rounds: vec![Round {
                notice_id: notice,
                logical_notice_id: None,
                lot_results: vec![result("RES-1", "LOT-1", res1), result("RES-2", "LOT-2", 30)],
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

/// Issue 579 (d): the dissolve's moved rows sit where the fold would not write them — the
/// party and bid-party rows where the condemned 10 sorted (before the sibling 30, its mint
/// after), the winner re-inserted after RES-2's — and the refold's compare reads rows in
/// order. So the window stamps every Tender it touched, not only tier 5's (none here), a
/// Tender it touched through party and bid-party rows alone (notice 901's) included: the
/// trailing fold rewrites the award's three tables and the other's two in its own order,
/// and a later refold with the compare on rewrites none. Unstamped, the trailing fold
/// skipped both Tenders and that refold rewrote all five, announcing corrections that moved
/// nothing.
#[tokio::test]
async fn a_wet_dissolve_then_a_refold_rewrites_no_table() {
    let path = "test-dissolve-579-parity.db";
    let (db, conn) = open(path).await;
    db.set_refold_compare(store::RefoldCompare::On);
    // The compare refuses to run without the by-version indexes.
    db.build_tender_indexes().await.unwrap();
    org(&conn, 10, Some("CZ"), Some("BAD"), "Placeholder Merger").await;
    org(&conn, 20, Some("CZ"), None, "Buyer Office").await;
    org(&conn, 30, Some("CZ"), Some("49371185"), "Sibling Works").await;
    for notice in [900, 901] {
        conn.execute(
            "INSERT INTO notices (id, source, publication_id, content_hash, profile, fetch_id,
                                  member_path, ingested_at, parse_state, projected)
             VALUES (?, 'ted', ?, ?, 'eforms:test', 1, 'p', 0, 'parsed', 1)",
            (Value::Integer(notice), Value::Text(format!("{notice}-2026")), Value::Text(format!("h{notice}"))),
        )
        .await
        .unwrap();
        mention(&conn, notice, "ORG-1", 20, "Buyer Office", Some("CZ")).await;
        mention(&conn, notice, "ORG-2", 10, "Gamma Works", Some("CZ")).await;
        mention(&conn, notice, "ORG-3", 30, "Sibling Works", Some("CZ")).await;
    }
    db.apply_tenders(&[award(10), party_only(10)], NOW, false).await.unwrap();

    let (wet, _) = db.repair_placeholder_orgs_batch(bad, FIELDS, normalize, n2, CAP, 10_000, 0, false).await.expect("wet");
    assert_eq!((wet.dissolved, wet.parties, wet.bid_parties, wet.winners, wet.winners_deleted), (1, 2, 2, 1, 0), "{wet:?}");
    assert_eq!((wet.mentions, wet.fresh), (2, 1), "both mentions on one mint");
    assert_eq!(
        (wet.tender_changes, wet.refold_tenders, wet.refold_notices),
        (2, 2, 2),
        "both touched Tenders are stamped and their notices re-queued, with no tier 5"
    );
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM tenders WHERE projection_epoch = 0").await, 2);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM notices WHERE id IN (900, 901) AND projected = 0").await, 2);
    let gamma = count(&conn, "SELECT organization_id FROM organization_mentions WHERE notice_id = 900 AND section_id = 'ORG-2'").await;
    assert!(gamma > 30, "Gamma Works minted after the sibling ({gamma})");

    // The trailing fold binds ORG-2 to the mint, as the mentions now say.
    let trailing = db.apply_tenders(&[award(gamma), party_only(gamma)], NOW, false).await.unwrap();
    assert_eq!(
        (trailing.tenders_corrected, trailing.tables_rewritten),
        (2, 5),
        "the award's parties, bid parties and winners, and the other's parties and bid parties, put in the \
         fold's order: {trailing:?}"
    );
    db.set_projection_epoch_for_test(-1).await.unwrap();
    let refold = db.apply_tenders(&[award(gamma), party_only(gamma)], NOW, false).await.unwrap();
    assert_eq!(
        (refold.tenders_verified, refold.tenders_corrected, refold.tables_rewritten),
        (2, 0, 0),
        "the fold's fixed point: {refold:?}"
    );
    drop(conn);
    drop(db);
    cleanup(path);
}
