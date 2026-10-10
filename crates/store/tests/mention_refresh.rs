//! Issues 434 and 439: what the mention resolver does with a mention it has
//! already recorded, and which row a duplicated key binds to.
//!
//! 434: the resolver used to return early for ANY recorded `(notice, section)`,
//! and issue 248's keep-set means a re-parse no longer deletes a mention whose
//! section survives — so no parse fix ever reached a standing mention. What is
//! pinned: a mention whose published facts changed is re-resolved through the
//! new-mention path and rewritten IN PLACE (same rowid, no second row); an
//! unchanged one costs nothing and mints nothing; a mention gaining a country
//! re-binds to the country-scoped provisional row; the counters say so.
//!
//! 439: the identity-triple preload and the name probes bind the LOWEST-id row
//! among duplicates, and the orders that make it so cost no sorter.

use store::turso::Value;
use store::{Db, Identifier, Mention};

async fn fresh(name: &str) -> (Db, String) {
    let path = format!("/tmp/tender-db-{name}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = Db::open(&path).await.unwrap();
    // The resolver writes mentions against notices/sections these tests do not
    // seed; the FK layer is the fold's business, not the resolver's.
    db.set_foreign_keys(false).await.unwrap();
    (db, path)
}

fn cleanup(path: &str) {
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

fn mention(notice: i64, name: &str, country: Option<&str>) -> Mention {
    Mention {
        notice_id: notice,
        section_id: "ORG-1".into(),
        name: name.into(),
        country: country.map(Into::into),
        raw_identifier: None,
        scheme: None,
        identifier: None,
        variants: Vec::new(),
    }
}

async fn int(db: &Db, sql: &str) -> i64 {
    match db.scalar(sql).await.unwrap() {
        Some(Value::Integer(n)) => n,
        other => panic!("{sql}: {other:?}"),
    }
}

async fn text(db: &Db, sql: &str) -> Option<String> {
    match db.scalar(sql).await.unwrap() {
        Some(Value::Text(s)) => Some(s),
        Some(Value::Null) | None => None,
        other => panic!("{sql}: {other:?}"),
    }
}

/// One resolver run, as a fold opens and closes it: the ids, and what the
/// issue-434 refresh did.
async fn run(db: &Db, mentions: &[Mention]) -> (Vec<i64>, store::MentionRefresh) {
    let mut resolver = db.mention_resolver(None, None, None, None, None, None, 0).await.unwrap();
    let ids = db.resolve_mentions(&mut resolver, mentions, 0).await.unwrap();
    let refresh = Db::mention_refresh(&resolver);
    db.finish_mention_resolver(resolver).await.unwrap();
    (ids, refresh)
}

/// The store half of issue 434's contract (the fold half, with the party row
/// following, is `a_refold_refreshes_a_recorded_mention_whose_published_name_changed`
/// in ingest): unchanged → kept, no work; changed → re-resolved and rewritten
/// in place.
#[tokio::test]
async fn a_stale_mention_is_rewritten_in_place_and_an_unchanged_one_is_kept() {
    let (db, path) = fresh("refresh-in-place").await;
    let (first, _) = run(&db, &[mention(1, "Stadtwerke Alt", Some("DE"))]).await;
    let old = first[0];
    let rowid = int(&db, "SELECT rowid FROM organization_mentions").await;
    // A folded Tender per notice, at the current epoch, so the stamp a re-bind
    // owes its Tender is observable (and its absence where nothing re-binds).
    for (tender, notice) in [(10, 1), (20, 2)] {
        db.execute_for_test(&format!(
            "INSERT INTO tenders (id, source, kind, created_at, projection_epoch)
             VALUES ({tender}, 'ted', 'procedure', 0, {})",
            store::canonical::PROJECTION_EPOCH
        ))
        .await
        .unwrap();
        db.execute_for_test(&format!(
            "INSERT INTO tender_versions (tender_id, seq, caused_by_notice_id, published_at,
                                          publication_id)
             VALUES ({tender}, 1, {notice}, 0, 'pub-{notice}')"
        ))
        .await
        .unwrap();
    }
    let epoch = |tender: i64| format!("SELECT projection_epoch FROM tenders WHERE id = {tender}");

    // A later run over the SAME published facts: the stored binding stands, and
    // nothing is written or minted.
    let (again, refresh) = run(&db, &[mention(1, "Stadtwerke Alt", Some("DE"))]).await;
    assert_eq!(again[0], old);
    assert_eq!(refresh, store::MentionRefresh { refreshed: 0, rebound: 0, retired: 0, retire_corrections: 0, tenders_stamped: 0 });
    assert_eq!(int(&db, "SELECT COUNT(*) FROM organizations").await, 1, "nothing minted");

    // The parse now publishes another name for the same section.
    let (after, refresh) = run(&db, &[mention(1, "Stadtwerke Neu", Some("DE"))]).await;
    let new = after[0];
    assert_ne!(new, old, "the new name resolves through the new-mention path");
    assert_eq!(refresh, store::MentionRefresh { refreshed: 1, rebound: 1, retired: 0, retire_corrections: 0, tenders_stamped: 1 });
    assert_eq!(
        int(&db, &epoch(10)).await,
        0,
        "the re-bind stamps its notice's Tender stale, so the fold rewrites its party rows \
         (an unchanged chain at the current epoch early-returns)"
    );
    assert_eq!(
        int(&db, &epoch(20)).await,
        store::canonical::PROJECTION_EPOCH,
        "…and only that Tender"
    );
    assert_eq!(int(&db, "SELECT COUNT(*) FROM organization_mentions").await, 1, "no second row");
    assert_eq!(
        int(&db, "SELECT rowid FROM organization_mentions").await,
        rowid,
        "rewritten IN PLACE — a DELETE + INSERT is ~2.2 s of FK proving per row on prod"
    );
    assert_eq!(int(&db, "SELECT organization_id FROM organization_mentions").await, new);
    assert_eq!(
        text(&db, "SELECT name FROM organization_mentions").await.as_deref(),
        Some("Stadtwerke Neu")
    );
    assert_eq!(
        text(&db, &format!("SELECT name FROM organizations WHERE id = {new}")).await.as_deref(),
        Some("Stadtwerke Neu")
    );
    // The old row is left standing, mention-less: which job reaps such rows is
    // recorded on issue 434, not decided here.
    assert_eq!(
        int(&db, &format!("SELECT COUNT(*) FROM organization_mentions WHERE organization_id = {old}"))
            .await,
        0
    );

    // And a refresh that re-resolves onto the SAME organization rewrites the
    // facts but does not count as a re-bind: an identifier-bearing mention
    // whose name changed stays on its registration.
    let with_id = |name: &str| Mention {
        notice_id: 2,
        section_id: "ORG-2".into(),
        name: name.into(),
        country: Some("DE".into()),
        raw_identifier: Some("DE129273398".into()),
        scheme: Some("VAT".into()),
        identifier: Some(Identifier {
            country: Some("DE".into()),
            kind: "vat".into(),
            value: "DE129273398".into(),
        }),
        variants: Vec::new(),
    };
    let (reg, _) = run(&db, &[with_id("Landkreis Muster")]).await;
    let (reg2, refresh) = run(&db, &[with_id("Landratsamt Muster")]).await;
    assert_eq!(reg2[0], reg[0], "the registration still binds its own row");
    assert_eq!(refresh, store::MentionRefresh { refreshed: 1, rebound: 0, retired: 0, retire_corrections: 0, tenders_stamped: 0 });
    assert_eq!(
        int(&db, &epoch(20)).await,
        store::canonical::PROJECTION_EPOCH,
        "a refresh that keeps the organization owes the Tender nothing: no party row moves"
    );
    assert_eq!(
        text(&db, "SELECT name FROM organization_mentions WHERE notice_id = 2").await.as_deref(),
        Some("Landratsamt Muster"),
        "the mention row carries the name the notice publishes now"
    );

    // A NULL stored name is the same fact as an empty one: never a refresh.
    db.execute_for_test(
        "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name)
         VALUES (3, 'ORG-1', 1, NULL)",
    )
    .await
    .unwrap();
    let (_, refresh) = run(&db, &[mention(3, "", None)]).await;
    assert_eq!(refresh.refreshed, 0, "NULL and '' both say 'no name published'");

    drop(db);
    cleanup(&path);
}

/// Issue 434, consequence (2): the text era's `TXT-CY` is re-homed onto ORG-1
/// by the re-parse, but every standing ORG-1 mention kept `country` NULL
/// (Deutsche Bahn's row 24630207 holds 2,332 of them). A mention that GAINS a
/// country re-binds to the country-scoped provisional row issue 234's reuse
/// keys on `(name_norm, country)`.
#[tokio::test]
async fn a_mention_gaining_a_country_rebinds_to_the_country_scoped_row() {
    let (db, path) = fresh("refresh-country").await;
    // The standing DE-scoped row, from a notice that always published its country.
    let (scoped, _) = run(&db, &[mention(2, "Deutsche Bahn AG", Some("DE"))]).await;
    // The text-era mention, recorded before the re-parse read its country.
    let (bare, _) = run(&db, &[mention(1, "Deutsche Bahn AG", None)]).await;
    assert_ne!(bare[0], scoped[0], "premise: a country-less mention has its own row");
    assert_eq!(
        text(&db, &format!("SELECT country FROM organizations WHERE id = {}", bare[0])).await,
        None
    );

    let (refreshed, refresh) = run(&db, &[mention(1, "Deutsche Bahn AG", Some("DE"))]).await;
    assert_eq!(refreshed[0], scoped[0], "the mention re-binds to the DE-scoped row");
    assert_eq!(refresh.refreshed, 1);
    assert_eq!(refresh.rebound, 1);
    assert_eq!(
        text(&db, "SELECT country FROM organization_mentions WHERE notice_id = 1").await.as_deref(),
        Some("DE")
    );
    assert_eq!(
        int(&db, "SELECT organization_id FROM organization_mentions WHERE notice_id = 1").await,
        scoped[0]
    );
    assert_eq!(int(&db, "SELECT COUNT(*) FROM organizations").await, 2, "nothing new minted");

    drop(db);
    cleanup(&path);
}

/// Issue 439: the identity index is not unique (issue 62), so one triple can
/// stand on several rows — prod's VAT DE811335517 on 8, where row 1448
/// ('Vergabekammer Südbayern') holds 2,204 'Vergabekammer Nordbayern' mentions.
/// The preload used a plain `insert` over an unordered scan, so the binding row
/// was the LAST the scan produced. It is now the lowest id, and the name
/// probes follow the same rule. RED on the old preload: it bound 9000.
#[tokio::test]
async fn the_triple_preload_and_the_name_probes_bind_the_lowest_id_among_duplicates() {
    let (db, path) = fresh("lowest-id").await;
    for id in [5000, 1448, 9000] {
        db.execute_for_test(&format!(
            "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm,
                                        provisional, created_at)
             VALUES ({id}, 'DE', 'vat', 'DE811335517', 'Vergabekammer {id}',
                     'vergabekammer {id}', 0, 0)"
        ))
        .await
        .unwrap();
    }
    for id in [700, 300] {
        db.execute_for_test(&format!(
            "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm,
                                        provisional, created_at)
             VALUES ({id}, 'DE', NULL, NULL, 'Stadt Muster', 'stadt muster', 1, 0)"
        ))
        .await
        .unwrap();
    }
    for id in [800, 400] {
        db.execute_for_test(&format!(
            "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm,
                                        provisional, created_at)
             VALUES ({id}, NULL, NULL, NULL, 'Gemeinde Muster', 'gemeinde muster', 1, 0)"
        ))
        .await
        .unwrap();
    }
    let vat = Mention {
        notice_id: 1,
        section_id: "ORG-1".into(),
        name: "Vergabekammer Nordbayern".into(),
        country: Some("DE".into()),
        raw_identifier: Some("DE811335517".into()),
        scheme: Some("VAT".into()),
        identifier: Some(Identifier {
            country: Some("DE".into()),
            kind: "vat".into(),
            value: "DE811335517".into(),
        }),
        variants: Vec::new(),
    };
    let (ids, _) = run(&db, &[vat, mention(2, "Stadt Muster", Some("DE"))]).await;
    assert_eq!(ids[0], 1448, "the triple binds the lowest-id row, whatever the scan order");
    assert_eq!(ids[1], 300, "the (name_norm, country) probe binds the lowest-id twin");

    // The country-less probe (issue 351's reuse) needs the injected `norm` and
    // a tier verdict to reach its probe; with none injected it mints, so that
    // arm is pinned through the SQL it runs, below.

    // The orders must cost nothing: no sorter on the preload's full walk, and
    // the name probes still seek an index. Built the way a healthy box has them.
    db.build_organization_indexes().await.unwrap();
    for (sql, what) in [
        (
            "SELECT id, country, identifier_kind, identifier FROM organizations \
              WHERE identifier IS NOT NULL ORDER BY id",
            "the triple preload",
        ),
        (
            "SELECT id FROM organizations \
              WHERE name_norm = 'stadt muster' AND country = 'DE' AND identifier IS NULL \
              ORDER BY id LIMIT 1",
            "the country-scoped name probe",
        ),
        (
            "SELECT id FROM organizations \
              WHERE name_norm = 'gemeinde muster' AND +country IS NULL \
                AND +identifier IS NULL ORDER BY id LIMIT 1",
            "the country-less name probe",
        ),
    ] {
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
        println!("{what}: {plan}");
        // turso 0.7.2 spells an ORDER BY sort `USE SORTER FOR ORDER BY`; only a heap
        // sort is `USE TEMP B-TREE`, so a TEMP-B-TREE-only check passes over a sort.
        assert!(!plan.contains("TEMP B-TREE") && !plan.contains("SORTER"), "{what} must not sort: {plan}");
        if what != "the triple preload" {
            assert!(plan.contains("INDEX"), "{what} must seek an index, not walk the table: {plan}");
        }
    }
    assert_eq!(
        int(
            &db,
            "SELECT id FROM organizations WHERE name_norm = 'gemeinde muster' AND +country IS NULL \
               AND +identifier IS NULL ORDER BY id LIMIT 1"
        )
        .await,
        400
    );

    drop(db);
    cleanup(&path);
}

/// Issue 510: a recorded mention of a void-lot party is retired with the party and
/// bid-party rows that name it, and both their Tenders and the notice's own are
/// stamped stale for the fold's Phase 2. An absent key writes nothing; a second
/// retire finds nothing; the organization stays for the orphan sweep.
#[tokio::test]
async fn retiring_a_mention_takes_its_party_rows_and_stamps_their_tenders() {
    let (db, path) = fresh("retire-mention").await;
    let (ids, _) = run(&db, &[mention(1, "Infructueux", Some("FR"))]).await;
    let org = ids[0];
    // Tender 10's version is caused by notice 1; Tender 30's by notice 3, but a party
    // row there still names notice 1's mention (a party a later version carries).
    for (tender, notice) in [(10, 1), (30, 3)] {
        db.execute_for_test(&format!(
            "INSERT INTO tenders (id, source, kind, created_at, projection_epoch)
             VALUES ({tender}, 'ted', 'procedure', 0, {})",
            store::canonical::PROJECTION_EPOCH
        ))
        .await
        .unwrap();
        db.execute_for_test(&format!(
            "INSERT INTO tender_versions (tender_id, seq, caused_by_notice_id, published_at, publication_id)
             VALUES ({tender}, 1, {notice}, 0, 'pub-{notice}')"
        ))
        .await
        .unwrap();
        db.execute_for_test(&format!(
            "INSERT INTO tender_version_parties (tender_id, seq, lot_id, role, organization_id,
                                                 mention_notice_id, mention_section_id)
             VALUES ({tender}, 1, NULL, 'winner', {org}, 1, 'ORG-1')"
        ))
        .await
        .unwrap();
    }
    db.execute_for_test(&format!(
        "INSERT INTO tender_version_bid_parties (tender_id, seq, bid_id, role, organization_id,
                                                 mention_notice_id, mention_section_id)
         VALUES (10, 1, 1, 'tenderer', {org}, 1, 'ORG-1')"
    ))
    .await
    .unwrap();

    let mut resolver = db.mention_resolver(None, None, None, None, None, None, 0).await.unwrap();
    let keys = vec![(1, "ORG-1".to_owned()), (9, "ORG-9".to_owned())];
    let changes_before = int(&db, "SELECT COUNT(*) FROM changes").await;
    assert_eq!(db.retire_mentions(&mut resolver, &keys, 77).await.unwrap(), 1, "the absent key writes nothing");
    assert_eq!(db.retire_mentions(&mut resolver, &keys, 78).await.unwrap(), 0, "idempotent");
    let refresh = Db::mention_refresh(&resolver);
    db.finish_mention_resolver(resolver).await.unwrap();
    assert_eq!((refresh.retired, refresh.tenders_stamped, refresh.retire_corrections), (1, 2, 2), "{refresh:?}");
    // ADR-0017 D5: the deleted party rows were served, so each Tender is announced once,
    // seq-less, in the retire's own transaction (the fold's compare reads after the delete
    // and would see nothing to correct for a party-only footprint).
    assert_eq!(
        int(
            &db,
            "SELECT COUNT(*) FROM changes WHERE entity_kind = 'tender' AND entity_id IN (10, 30) \
              AND version_seq IS NULL AND op = 'changed' AND changed_at = 77"
        )
        .await,
        2
    );
    assert_eq!(int(&db, "SELECT COUNT(*) FROM changes").await, changes_before + 2, "the idempotent call announces nothing");

    assert_eq!(int(&db, "SELECT COUNT(*) FROM organization_mentions").await, 0);
    assert_eq!(int(&db, "SELECT COUNT(*) FROM tender_version_parties").await, 0);
    assert_eq!(int(&db, "SELECT COUNT(*) FROM tender_version_bid_parties").await, 0);
    assert_eq!(int(&db, "SELECT COUNT(*) FROM tenders WHERE projection_epoch = 0").await, 2, "both Tenders stamped");
    assert_eq!(int(&db, &format!("SELECT COUNT(*) FROM organizations WHERE id = {org}")).await, 1, "the sweep's job");
    cleanup(&path);
}
