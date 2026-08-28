//! Issue 300 Stage 1, the repair half: `repair_placeholder_orgs_batch`
//! dissolves orgs whose identifier the injected predicate condemns —
//! re-resolving each mention through the post-234 provisional path, moving
//! party/bid-party rows by their exact (mention_notice_id,
//! mention_section_id), repointing winner rows via caused_by_notice_id with
//! the one-mention-per-notice guard, and deleting the org. An org with an
//! unresolvable winner row is skipped whole. Dry-run counts and writes
//! nothing; a second wet run finds nothing (dissolved orgs left scope).

use store::turso::Value;

fn bad(_country: Option<&str>, _kind: &str, value: &str) -> bool {
    value == "BAD"
}

async fn seed(path: &str) -> (store::Db, store::turso::Connection) {
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(path).await.unwrap();
    let raw = store::turso::Builder::new_local(path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    conn.execute("BEGIN", ()).await.unwrap();

    // 50 = condemned stranger-merger with three mentions; 51 = condemned but
    // winner-ambiguous (two mentions on one notice); 52 = clean identifier;
    // 60 = standing provisional the "Alpha City" mention must REUSE.
    for (id, ident, name, prov) in [
        (50, Some("BAD"), "Stranger Merger", 0),
        (51, Some("BAD"), "Ambiguous", 0),
        (52, Some("49371185"), "Gymnázium", 0),
        (53, Some("BAD"), "Tier Two", 0),
        (54, Some("BAD"), "Carried", 0),
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
    // Versions: tender 1 caused by notice 100 (org 50's winner context),
    // tender 2 caused by notice 200 (org 51's — ambiguous).
    for (t, seq, n) in
        [(1i64, 1i64, 100i64), (2, 1, 200), (3, 1, 300), (4, 1, 400), (4, 2, 401)]
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
    ] {
        conn.execute(
            "INSERT INTO tender_version_result_winners (tender_id, seq, lot_result_id, organization_id)
             VALUES (?, ?, ?, ?)",
            (Value::Integer(t), Value::Integer(seq), Value::Integer(lr), Value::Integer(org)),
        )
        .await
        .unwrap();
    }
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
async fn the_dissolve_splits_condemned_orgs_and_skips_ambiguous_winners() {
    let (db, conn) = seed("test-dissolve.db").await;

    // Dry-run: full preview, nothing written.
    let (dry, _) = db.repair_placeholder_orgs_batch(bad, 10_000, 0, true).await.expect("dry");
    assert_eq!((dry.scanned, dry.condemned), (5, 4), "50, 51, 53, 54 condemned, 52 clean");
    assert_eq!((dry.parties, dry.bid_parties), (3, 1), "dry run previews the blast radius");
    assert_eq!(
        (dry.dissolved, dry.skipped),
        (3, 1),
        "51 skipped (no signal); 53 via tier 2; 54 via tier 3 (carried winner)"
    );
    assert_eq!(dry.mentions, 6, "50's three + 53's two + 54's one");
    assert_eq!((dry.fresh, dry.reused), (5, 1), "Beta/nameless/Buyer/Winner Co/Carried fresh; Alpha City reuses 60");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE id = 50").await, 1);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations").await, 6, "dry run minted nothing");

    // Wet run.
    let (wet, _) = db.repair_placeholder_orgs_batch(bad, 10_000, 0, false).await.expect("wet");
    assert_eq!((wet.dissolved, wet.skipped), (3, 1));
    assert_eq!((wet.fresh, wet.reused), (5, 1));
    assert_eq!((wet.parties, wet.bid_parties), (3, 1));
    assert_eq!(wet.winners, 5, "50's two + 53's tier-2 row + 54's two (one carried)");
    assert_eq!(wet.winner_dups, 1, "lot_result 11 already stood on org 60");
    assert_eq!(wet.tender_changes, 3, "tenders 1, 3, 4; tender 2's org was skipped");

    // Org 50 is gone; 51 untouched; the Alpha City mention sits on 60; Beta
    // Corp and the nameless mention sit on fresh provisionals.
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE id = 50").await, 0);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE id = 51").await, 1);
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
    // ambiguous tender-2 row still stands on 51.
    assert_eq!(
        count(&conn, "SELECT organization_id FROM tender_version_result_winners WHERE lot_result_id = 10").await,
        60
    );
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM tender_version_result_winners WHERE lot_result_id = 11").await,
        1
    );
    assert_eq!(
        count(&conn, "SELECT organization_id FROM tender_version_result_winners WHERE lot_result_id = 12").await,
        51
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

    // A rerun finds the dissolved org gone: only 51 (still skipped) remains.
    let (again, _) = db.repair_placeholder_orgs_batch(bad, 10_000, 0, false).await.expect("rerun");
    assert_eq!((again.condemned, again.dissolved, again.skipped), (1, 0, 1));
}
