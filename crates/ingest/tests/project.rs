//! Projection tests (issue 04), driven by the committed real-notice corpus.
//!
//! The headline case is `tests/fixtures/eforms-chain/`: one real Maltese
//! procedure published as CN → corrigendum → corrigendum → CAN over six months.
//! It must collapse into exactly one Tender with four versions, the corrigendum
//! must supersede the field it actually moved, and the change log must say so.

use ingest::{process, profile, project};
use store::{Db, Notice, NoticeValue, Parse, Parsed, Section, ValueRow};

/// The change log via the production reader path (`read::changes_since`) — the
/// tests exercise it now that `Db` no longer duplicates the query (issue 38).
async fn changes(db: &Db, cursor: i64, limit: i64) -> Vec<store::Change> {
    let readers = db.readers(1).expect("readers");
    let reader = readers.get().await.expect("reader");
    store::read::changes_since(&reader, cursor, limit, None).await.expect("changes")
}

const SOURCE: &str = "ted";

/// A scratch database with a fetch row to hang notices off (notices carry a
/// mandatory archive reference).
async fn scratch(name: &str) -> (Db, i64, String) {
    let path = format!("/tmp/tender-db-project-{name}-{}.db", std::process::id());
    let _ = std::fs::remove_file(&path);
    let db = Db::open(&path).await.expect("open scratch db");
    db.record_fetch(&store::Fetch {
        source: SOURCE.into(),
        kind: "daily".into(),
        period: "2026-00136".into(),
        url: "https://example.invalid/pkg".into(),
        sha256: "aa".into(),
        bytes: 1,
        fetched_at: 0,
        path: "ted/daily/2026-00136.tar.gz".into(),
    })
    .await
    .expect("record fetch");
    let fetch_id = db.current_packages(SOURCE, "daily", None).await.expect("packages")[0].fetch_id;
    (db, fetch_id, path)
}

/// Run a fixture through the real dispatch + parse chain and store it, exactly
/// as `process` would from an archived package.
async fn ingest(db: &Db, fetch_id: i64, relative: &str) {
    ingest_from(db, fetch_id, SOURCE, relative).await;
}

/// Ingest a fixture as a named Source — DÖE and TED share one procedure across
/// Sources (ADR-0003), so the pair test needs to place notices under both.
async fn ingest_from(db: &Db, fetch_id: i64, source: &str, relative: &str) {
    let path = format!("tests/fixtures/{relative}");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    ingest_bytes(db, fetch_id, source, relative, &bytes).await;
}

async fn ingest_bytes(db: &Db, fetch_id: i64, source: &str, relative: &str, bytes: &[u8]) {
    let profile::Disposition::Records(records) = profile::dispatch(relative, bytes) else {
        panic!("{relative}: dispatch skipped a fixture");
    };
    let [profile::Record::Notice(n)] = &records[..] else {
        panic!("{relative}: expected one notice record");
    };
    // Route by profile exactly as `process` does — eForms, TED_EXPORT
    // (r208/r209) and internal-OJS fixtures all ingest through here.
    let parse = process::parse_payload(&n.profile, bytes);
    assert!(matches!(parse, Parse::Parsed(_)), "{relative}: {parse:?}");
    let (published_at, dispatched_at) = match &parse {
        Parse::Parsed(parsed) => {
            let (p, d) = project::notice_instants(parsed);
            (Some(p), d)
        }
        _ => (None, None),
    };
    db.record_notice(
        &Notice {
            source: source.into(),
            publication_id: n.publication_id.clone(),
            content_hash: n.content_hash.clone(),
            profile: n.profile.clone(),
            declared_version: n.declared_version.clone(),
            fetch_id,
            member_path: n.member_path.clone(),
            ingested_at: 0,
            published_at,
            dispatched_at,
        },
        &parse,
    )
    .await
    .expect("record notice");
}

/// Assertions go through plain SQL against the canonical layer, because "the
/// canonical layer is queryable with plain SQL" is the promise under test.
async fn scalar(db: &Db, sql: &str) -> i64 {
    match db.scalar(sql).await.expect("query") {
        Some(turso::Value::Integer(i)) => i,
        other => panic!("{sql}: expected an integer, got {other:?}"),
    }
}

async fn deadline(db: &Db, seq: i64) -> i64 {
    scalar(
        db,
        &format!(
            "SELECT utc_seconds FROM tender_version_dates
              WHERE seq = {seq} AND field = 'submission_deadline'"
        ),
    )
    .await
}

async fn title(db: &Db, seq: i64) -> Option<String> {
    query_text(
        db,
        &format!(
            "SELECT value FROM tender_version_texts
              WHERE seq = {seq} AND field = 'title' AND lot_id IS NULL"
        ),
    )
    .await
}

async fn query_text(db: &Db, sql: &str) -> Option<String> {
    match db.scalar(sql).await.expect("query") {
        Some(turso::Value::Text(s)) => Some(s),
        _ => None,
    }
}

// ------------------------------------------------------------- the real chain

/// CN → 2 corrigenda → CAN: one Tender, four versions, in publication order.
#[tokio::test]
async fn the_real_procedure_chain_becomes_one_tender_with_four_versions() {
    let (db, fetch_id, path) = scratch("chain").await;
    for fixture in [
        "eforms-chain/4-can-29-380868-2026.xml",
        "eforms-chain/1-cn-16-831374-2025.xml",
        "eforms-chain/3-change-16-18902-2026.xml",
        "eforms-chain/2-change-16-6281-2026.xml",
    ] {
        // Deliberately ingested out of order: projection orders by publication,
        // not by the order notices happened to arrive.
        ingest(&db, fetch_id, fixture).await;
    }

    let report = project::project(&db, false).await.expect("project");
    assert_eq!(report.notices, 4);
    assert_eq!(report.tenders, 1);
    assert_eq!(report.islands, 0);
    assert_eq!(report.applied.versions_written, 4);

    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM tenders").await, 1);
    assert_eq!(
        query_text(&db, "SELECT procedure_key FROM tenders").await.as_deref(),
        Some("32c34097-960e-4d02-b04d-3ceac32cf020"),
        "the four notices share one BT-04"
    );
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM tender_versions").await, 4);

    // Version order is publication order: the CN first, the award last.
    assert_eq!(
        query_text(&db, "SELECT publication_id FROM tender_versions WHERE seq = 1").await.as_deref(),
        Some("00831374-2025")
    );
    assert_eq!(
        query_text(&db, "SELECT notice_subtype FROM tender_versions WHERE seq = 4").await.as_deref(),
        Some("29"),
        "the contract award notice closes the chain"
    );
    assert_eq!(
        scalar(
            &db,
            "SELECT COUNT(*) FROM tender_versions a JOIN tender_versions b
              ON b.seq = a.seq + 1 WHERE b.published_at < a.published_at"
        )
        .await,
        0,
        "published_at is monotonic across the chain"
    );

    let _ = std::fs::remove_file(&path);
}

/// Issue 174: the r208 era names its form-section submission deadline
/// `RECEIPT_LIMIT_DATE` (r209 renamed the element `DATE_RECEIPT_TENDERS`), and
/// the date mapping only knew the r209 name — so every 2011–2016 Tender
/// projected without a deadline while the parsed layer held it all along. The
/// coded section's `DT_DATE_FOR_SUBMISSION` stays unprojected in BOTH eras by
/// the same rule: the form value is the published instant, the coded one a
/// derived copy that can disagree with it.
#[tokio::test]
async fn an_r208_contract_notice_projects_its_submission_deadline() {
    let (db, fetch_id, path) = scratch("r208-deadline").await;
    ingest(&db, fetch_id, "r208/f02-000333-2014.xml").await;

    let report = project::project(&db, false).await.expect("project");
    assert_eq!(report.notices, 1);
    assert_eq!(report.tenders, 1);

    // RECEIPT_LIMIT_DATE 07/02/2014 + TIME 17:00 — the F02's IV.3.4 deadline.
    assert_eq!(deadline(&db, 1).await, 1_391_792_400, "2014-02-07 17:00 UTC");

    let _ = std::fs::remove_file(&path);
}

/// The corrigendum moved the submission deadline from 2026-01-21 to 2026-01-27
/// and changed nothing else. The projection must carry that through — and carry
/// everything the corrigendum was silent about forward unchanged.
#[tokio::test]
async fn a_corrigendum_supersedes_the_field_it_moved_and_carries_the_rest() {
    let (db, fetch_id, path) = scratch("supersede").await;
    for fixture in [
        "eforms-chain/1-cn-16-831374-2025.xml",
        "eforms-chain/2-change-16-6281-2026.xml",
        "eforms-chain/3-change-16-18902-2026.xml",
        "eforms-chain/4-can-29-380868-2026.xml",
    ] {
        ingest(&db, fetch_id, fixture).await;
    }
    project::project(&db, false).await.expect("project");

    // 2026-01-21T09:30+01:00 → 2026-01-27T09:30+01:00.
    assert_eq!(deadline(&db, 1).await, 1_768_984_200);
    assert_eq!(deadline(&db, 2).await, 1_769_502_600, "the corrigendum moved the deadline");

    // The title the corrigendum never mentions is still there at every version,
    // including under the award notice that closes the chain.
    let original = title(&db, 1).await.expect("the CN titles the procedure");
    assert_eq!(title(&db, 2).await.as_deref(), Some(original.as_str()));
    assert_eq!(title(&db, 4).await.as_deref(), Some(original.as_str()), "carried into the award");

    let _ = std::fs::remove_file(&path);
}

/// Diff-based change scoping (ADR-0001 amendment): the ops come from comparing
/// version payloads, so the chain reads added-then-changed and nothing is
/// emitted for a version that changed nothing.
#[tokio::test]
async fn the_change_log_reads_added_then_changed() {
    let (db, fetch_id, path) = scratch("changes").await;
    for fixture in [
        "eforms-chain/1-cn-16-831374-2025.xml",
        "eforms-chain/2-change-16-6281-2026.xml",
        "eforms-chain/3-change-16-18902-2026.xml",
        "eforms-chain/4-can-29-380868-2026.xml",
    ] {
        ingest(&db, fetch_id, fixture).await;
    }
    project::project(&db, false).await.expect("project");

    let tender_ops: Vec<(i64, String)> = changes(&db, 0, 100).await
        .into_iter()
        .filter(|c| c.entity_kind == "tender")
        .map(|c| (c.version_seq.unwrap_or(0), c.op))
        .collect();
    assert_eq!(
        tender_ops,
        vec![
            (1, "added".to_owned()),
            (2, "changed".to_owned()),
            (3, "changed".to_owned()),
            (4, "changed".to_owned()),
        ]
    );
    // The cursor is monotonic and the log is in ingestion order.
    let all = changes(&db, 0, 1000).await;
    assert!(all.windows(2).all(|w| w[0].cursor < w[1].cursor));
    assert!(all.iter().any(|c| c.entity_kind == "lot" && c.op == "added"));

    // Re-projecting an unchanged notice layer is a complete no-op.
    let before = all.len();
    let again = project::project(&db, false).await.expect("re-project");
    assert_eq!(again.applied.versions_written, 0);
    assert_eq!(again.applied.changes, 0);
    assert_eq!(changes(&db, 0, 1000).await.len(), before);

    // A rebuild reproduces the same canonical state and appends a fresh set of
    // change rows — the cursor is never renumbered.
    project::project(&db, true).await.expect("rebuild");
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM tender_versions").await, 4);
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM tenders").await, 1);
    let rebuilt = changes(&db, 0, 1000).await;
    assert!(rebuilt.len() > before, "the rebuild appended rather than rewrote");
    assert_eq!(rebuilt[0].cursor, 1, "the first cursor is untouched");

    let _ = std::fs::remove_file(&path);
}

/// A notice that publishes no procedure key is still a Tender — a single-notice
/// island (CONTEXT.md), never dropped and never guessed into someone else's
/// procedure. The BRIN and the PIN in the corpus are both real instances.
#[tokio::test]
async fn notices_without_a_procedure_key_become_island_tenders() {
    let (db, fetch_id, path) = scratch("island").await;
    for fixture in [
        "eforms/brin-x01-00497689-2026.xml",
        "eforms/pin-4-00496860-2026.xml",
        "eforms/cn-16-00494343-2026.xml",
    ] {
        ingest(&db, fetch_id, fixture).await;
    }
    let report = project::project(&db, false).await.expect("project");

    assert_eq!(report.tenders, 3, "three notices, three unrelated Tenders");
    assert_eq!(report.islands, 2, "the BRIN and the PIN carry no BT-04");
    assert_eq!(
        scalar(&db, "SELECT COUNT(*) FROM tenders WHERE island_notice_id IS NOT NULL").await,
        2
    );
    // A business registration notice is a Tender of its own kind, not a
    // procurement procedure.
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM tenders WHERE kind = 'registration'").await, 1);
    // Every island has exactly one version — that is what makes it an island.
    assert_eq!(
        scalar(
            &db,
            "SELECT COUNT(*) FROM tenders t WHERE t.island_notice_id IS NOT NULL
              AND (SELECT COUNT(*) FROM tender_versions v WHERE v.tender_id = t.id) != 1"
        )
        .await,
        0
    );

    let _ = std::fs::remove_file(&path);
}

// ------------------------------------------------------- organization merging

/// Two notices, three organizations: two of them publish the same VAT id in
/// different lexical forms and must collapse into one canonical profile; the
/// third publishes junk and must stay provisional and alone. The notices are
/// synthetic because the merge rule is about identifier *values*, and no two
/// committed fixtures happen to share an organization.
#[tokio::test]
async fn mentions_merge_only_on_a_plausible_official_identifier() {
    let (db, fetch_id, path) = scratch("orgs").await;

    let notice = |publication_id: &str, orgs: &[(&str, &str, &str)]| {
        let mut parsed = Parsed {
            sections: vec![Section { id: "PROCEDURE".into(), kind: "Procedure".into(), parent: None }],
            values: vec![ValueRow {
                section_id: "PROCEDURE".into(),
                field_id: "BT-04-notice".into(),
                ordinal: 0,
                value: NoticeValue::Id {
                    scheme: None,
                    value: format!("procedure-{publication_id}"),
                    is_ref: false,
                },
            }],
        };
        for (section, name, identifier) in orgs {
            parsed.sections.push(Section {
                id: (*section).into(),
                kind: "Organization".into(),
                parent: Some("PROCEDURE".into()),
            });
            parsed.values.push(ValueRow {
                section_id: (*section).into(),
                field_id: "BT-500-Organization-Company".into(),
                ordinal: 0,
                value: NoticeValue::Text { lang: Some("ENG".into()), value: (*name).into() },
            });
            // The official identifier hangs off the Organization's legal-entity
            // child, not off the Organization itself — the shape every real
            // eForms notice uses (14 813 of them on the 2026-136 daily).
            let legal_entity = format!("{section}-legal");
            parsed.sections.push(Section {
                id: legal_entity.clone(),
                kind: "CompanyLegalEntity".into(),
                parent: Some((*section).into()),
            });
            parsed.values.push(ValueRow {
                section_id: legal_entity,
                field_id: "BT-501-Organization-Company".into(),
                ordinal: 0,
                value: NoticeValue::Id {
                    scheme: Some("VAT".into()),
                    value: (*identifier).into(),
                    is_ref: false,
                },
            });
            // The buyer role, as an id-ref out of a procedure-level section.
            parsed.values.push(ValueRow {
                section_id: "PROCEDURE".into(),
                field_id: "OPT-300-Procedure-Buyer".into(),
                ordinal: parsed.values.len() as i64,
                value: NoticeValue::Id {
                    scheme: None,
                    value: (*section).into(),
                    is_ref: true,
                },
            });
        }
        (
            Notice {
                source: SOURCE.into(),
                publication_id: publication_id.into(),
                content_hash: format!("hash-{publication_id}"),
                profile: "eforms:eforms-sdk-1.13".into(),
                declared_version: None,
                fetch_id,
                member_path: format!("{publication_id}.xml"),
                ingested_at: 0,
                published_at: None,
                dispatched_at: None,
            },
            Parse::Parsed(parsed),
        )
    };

    let (a, parse_a) = notice("00000001-2026", &[
        ("ORG-0001", "Acme BV", "NL804595859B01"),
        ("ORG-0002", "Junk Ltd", "Romania"),
    ]);
    let (b, parse_b) = notice("00000002-2026", &[
        // The same VAT id, spaced and lowercased the way real eSenders write it.
        ("ORG-0001", "ACME B.V.", "nl 8045 95859 b01"),
        ("ORG-0002", "Other Junk Ltd", "n/a"),
    ]);
    db.record_notice(&a, &parse_a).await.expect("notice a");
    db.record_notice(&b, &parse_b).await.expect("notice b");

    let report = project::project(&db, false).await.expect("project");
    assert_eq!(report.mentions, 4);

    // Four mentions, three profiles: the VAT pair merged, the two junk ids did
    // not — and no mention was destroyed in the process.
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM organization_mentions").await, 4);
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM organizations").await, 3);
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM organizations WHERE provisional = 0").await, 1);
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM organizations WHERE provisional = 1").await, 2);
    assert_eq!(
        query_text(&db, "SELECT identifier FROM organizations WHERE provisional = 0").await.as_deref(),
        Some("NL804595859B01"),
        "merged on the normalised form, not the raw string"
    );
    assert_eq!(
        scalar(&db, "SELECT mentions FROM v_organizations WHERE provisional = 0").await,
        2
    );
    // Both notices resolved their buyer role onto that one canonical profile.
    assert_eq!(
        scalar(
            &db,
            "SELECT COUNT(DISTINCT tender_id) FROM tender_version_parties p
               JOIN organizations o ON o.id = p.organization_id
              WHERE o.provisional = 0 AND p.role = 'Procedure-Buyer'"
        )
        .await,
        2
    );

    let _ = std::fs::remove_file(&path);
}

/// The current-state views the API and the SQL endpoint read: `MAX(seq)` per
/// Tender, with a usable title even when only the lots carry one.
#[tokio::test]
async fn the_current_state_views_show_the_newest_version() {
    let (db, fetch_id, path) = scratch("views").await;
    for fixture in [
        "eforms-chain/1-cn-16-831374-2025.xml",
        "eforms-chain/2-change-16-6281-2026.xml",
        "eforms-chain/3-change-16-18902-2026.xml",
        "eforms-chain/4-can-29-380868-2026.xml",
    ] {
        ingest(&db, fetch_id, fixture).await;
    }
    project::project(&db, false).await.expect("project");

    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM v_tenders").await, 1);
    assert_eq!(scalar(&db, "SELECT seq FROM v_tenders").await, 4, "current is the newest version");
    assert!(query_text(&db, "SELECT title FROM v_tenders").await.is_some());
    assert!(scalar(&db, "SELECT COUNT(*) FROM v_lots").await > 0);
    assert_eq!(
        scalar(&db, "SELECT COUNT(*) FROM v_lots WHERE title IS NULL").await,
        0,
        "every current lot is titled"
    );
    // The app's list reads the same view.
    assert_eq!(db.list_tenders(10).await.expect("list").len(), 1);

    let _ = std::fs::remove_file(&path);
}

/// Processing a package twice, then projecting, must not double anything —
/// notice identity dedups the notices and the version-per-notice constraint
/// dedups the projection.
#[tokio::test]
async fn reprocessing_and_reprojecting_a_package_changes_nothing() {
    let (db, fetch_id, path) = scratch("idempotent").await;
    ingest(&db, fetch_id, "eforms/cn-16-00494343-2026.xml").await;
    ingest(&db, fetch_id, "eforms/cn-16-00494343-2026.xml").await;
    project::project(&db, false).await.expect("project");

    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM notices").await, 1);
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM tender_versions").await, 1);
    let mentions = scalar(&db, "SELECT COUNT(*) FROM organization_mentions").await;

    project::project(&db, false).await.expect("re-project");
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM tender_versions").await, 1);
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM organization_mentions").await, mentions);

    let _ = std::fs::remove_file(&path);
}

/// An empty notice layer projects to an empty canonical layer rather than to an
/// error — the stages are independent (CONTEXT.md).
#[tokio::test]
async fn an_empty_notice_layer_projects_to_nothing() {
    let (db, _, path) = scratch("empty").await;
    assert_eq!(project::project(&db, false).await.expect("project"), project::Report::default());
    let _ = std::fs::remove_file(&path);
}

// ------------------------------------------------------------ results layer

/// The real Maltese chain's award notice (issue 13): the CAN's LotResult /
/// LotTender / SettledContract / TenderingParty sections become canonical
/// lot_results, bids and contracts — with the winner resolved through the
/// notice's own graph (RES → CON → TEN → TPA → ORG) and award-side party
/// roles scoped to their Lot rather than the Tender.
#[tokio::test]
async fn the_award_notice_yields_lot_results_bids_and_contracts() {
    let (db, fetch_id, path) = scratch("results").await;
    for fixture in [
        "eforms-chain/1-cn-16-831374-2025.xml",
        "eforms-chain/2-change-16-6281-2026.xml",
        "eforms-chain/3-change-16-18902-2026.xml",
        "eforms-chain/4-can-29-380868-2026.xml",
    ] {
        ingest(&db, fetch_id, fixture).await;
    }
    project::project(&db, false).await.expect("project");

    // Results exist exactly from the award version on.
    assert_eq!(
        scalar(&db, "SELECT COUNT(*) FROM tender_version_lot_results WHERE seq < 4").await,
        0
    );
    assert_eq!(
        scalar(&db, "SELECT COUNT(*) FROM tender_version_lot_results WHERE seq = 4").await,
        1
    );
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM bids").await, 1);
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM contracts").await, 1);

    // The decision, the awarded value (the winning bid's BT-720), and the lot
    // link — the CAN references the CN's LOT-0001, which resolves to the same
    // canonical Lot.
    assert_eq!(
        query_text(&db, "SELECT decision FROM v_lot_results").await.as_deref(),
        Some("selec-w")
    );
    assert_eq!(scalar(&db, "SELECT awarded_cents FROM v_lot_results").await, 23_968_954);
    assert_eq!(
        query_text(&db, "SELECT lot_key FROM v_lot_results").await.as_deref(),
        Some("LOT-0001")
    );
    // Received-submission statistics: 11 tenders.
    assert_eq!(
        scalar(
            &db,
            "SELECT count FROM tender_version_result_stats WHERE seq = 4 AND kind = 'tenders'"
        )
        .await,
        11
    );

    // The winner is the organization the notice mentions as ORG-0002, reached
    // through contract CON-0001 → bid TEN-0001 → party TPA-0001.
    assert_eq!(
        scalar(
            &db,
            "SELECT COUNT(*) FROM v_lot_results v
               JOIN organization_mentions m ON m.organization_id = v.winner_organization_id
              WHERE m.section_id = 'ORG-0002'"
        )
        .await,
        1
    );
    // The bid carries its consortium; the contract carries the buyer's id and
    // the settled bid's value.
    assert_eq!(
        query_text(&db, "SELECT role FROM tender_version_bid_parties WHERE seq = 4").await.as_deref(),
        Some("tenderer")
    );
    assert_eq!(
        query_text(&db, "SELECT buyer_contract_id FROM tender_version_contracts WHERE seq = 4")
            .await
            .as_deref(),
        Some("127804511")
    );
    assert_eq!(
        scalar(&db, "SELECT cents FROM tender_version_contracts WHERE seq = 4").await,
        23_968_954
    );

    // Issue 04's noted limitation is closed: the award-side Tenderer role is
    // scoped to its Lot, not the Tender.
    assert_eq!(
        scalar(
            &db,
            "SELECT COUNT(*) FROM tender_version_parties
              WHERE seq = 4 AND role = 'Tenderer' AND lot_id IS NOT NULL"
        )
        .await,
        1
    );

    // The competitor question of the spec, straight over the SQL view:
    // top organization by awarded cents.
    assert_eq!(
        scalar(
            &db,
            "SELECT SUM(awarded_cents) FROM v_lot_results
              GROUP BY winner_organization_id ORDER BY SUM(awarded_cents) DESC LIMIT 1"
        )
        .await,
        23_968_954
    );

    // The winner filter the API adds sees the same thing.
    let readers = db.readers(1).expect("readers");
    let reader = readers.get().await.expect("reader");
    let winner = scalar(&db, "SELECT winner_organization_id FROM v_lot_results").await;
    let filter = store::read::Filter { winner: Some(winner), ..store::read::Filter::default() };
    let rows = store::read::tenders(&reader, &filter, store::read::Scope::Page { after: 0, limit: 10 })
        .await
        .expect("tenders");
    assert_eq!(rows.len(), 1, "winner=<org> finds the tender the org won");
    let none = store::read::Filter { winner: Some(winner + 999), ..store::read::Filter::default() };
    let rows = store::read::tenders(&reader, &none, store::read::Scope::Page { after: 0, limit: 10 })
        .await
        .expect("tenders");
    assert!(rows.is_empty());

    // The detail payload carries the results layer.
    let detail = store::read::tender_detail(&reader, 1).await.expect("detail").expect("tender 1");
    assert_eq!(detail.lot_results.len(), 1);
    assert_eq!(detail.lot_results[0].winners.len(), 1);
    assert_eq!(detail.bids.len(), 1);
    assert_eq!(detail.contracts.len(), 1);

    let _ = std::fs::remove_file(&path);
}

/// Framework/DPS rounds (ted-empirical-checks.md §1/§3): repeated CANs under
/// one BT-04 accumulate — a later round must never delete or supersede an
/// earlier round's results, and a reused round-local lot id must not merge
/// two rounds' decisions.
#[tokio::test]
async fn framework_rounds_accumulate_without_deleting_earlier_results() {
    let (db, fetch_id, path) = scratch("fa-rounds").await;
    ingest(&db, fetch_id, "eforms/can-fa-29-00495185-2026.xml").await;

    // Round two: the same framework (same BT-04) publishing a second CAN a
    // month later — its own notice id (BT-701), and the round-local label
    // LOT-0000 reused for a different call-off (the verified HU relabeling
    // pattern).
    let round1 = std::fs::read("tests/fixtures/eforms/can-fa-29-00495185-2026.xml").expect("fixture");
    let round2 = String::from_utf8(round1)
        .expect("utf8")
        .replace("00495185-2026", "00495186-2026")
        .replace("0054cd60-111a-49db-9b1f-ad41591a140b", "0054cd60-111a-49db-9b1f-ad41591a140c")
        .replace("2026-07-17+02:00", "2026-08-17+02:00")
        .replace("Huur van zero emissie veegmachines", "Ronde 2: veegwagens op afroep")
        .replace(
            "<efbc:StatisticsNumeric>0</efbc:StatisticsNumeric>",
            "<efbc:StatisticsNumeric>3</efbc:StatisticsNumeric>",
        );
    ingest_bytes(&db, fetch_id, SOURCE, "eforms/can-fa-29-00495186-2026.xml", round2.as_bytes()).await;

    let report = project::project(&db, false).await.expect("project");
    assert_eq!(report.tenders, 1, "two rounds, one framework Tender");
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM tender_versions").await, 2);

    // Round one is visible alone at version 1; version 2 is the additive
    // union — nothing removed, nothing superseded.
    assert_eq!(
        scalar(&db, "SELECT COUNT(*) FROM tender_version_lot_results WHERE seq = 1").await,
        1
    );
    assert_eq!(
        scalar(&db, "SELECT COUNT(*) FROM tender_version_lot_results WHERE seq = 2").await,
        2
    );
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM lot_results").await, 2);
    assert_eq!(scalar(&db, "SELECT COUNT(DISTINCT notice_id) FROM lot_results").await, 2);

    // Both rounds label their call-off LOT-0000: one Lot identity, but the
    // decisions stay two distinct results and the lot's own content is
    // versioned — round two's relabeling did not overwrite round one.
    assert_eq!(
        scalar(&db, "SELECT COUNT(DISTINCT lot_id) FROM tender_version_lot_results").await,
        1
    );
    let title = |seq: i64| async move {
        format!(
            "SELECT value FROM tender_version_texts
              WHERE seq = {seq} AND field = 'title' AND lot_id IS NOT NULL"
        )
    };
    let round1_title = query_text(&db, &title(1).await).await.expect("round 1 lot title");
    let round2_title = query_text(&db, &title(2).await).await.expect("round 2 lot title");
    assert_ne!(round1_title, round2_title, "each round keeps its own lot content");

    // Each round's statistics stay attached to that round's result.
    assert_eq!(
        scalar(
            &db,
            "SELECT s.count FROM tender_version_result_stats s
               JOIN lot_results r ON r.id = s.lot_result_id
               JOIN tender_versions v ON v.tender_id = s.tender_id AND v.seq = 2
              WHERE s.seq = 2 AND r.notice_id = v.caused_by_notice_id"
        )
        .await,
        3
    );

    // The change log reads as accumulation: two lot_result additions, never a
    // removal or a rewrite of round one.
    let ops: Vec<(i64, String)> = changes(&db, 0, 100).await
        .into_iter()
        .filter(|c| c.entity_kind == "lot_result")
        .map(|c| (c.version_seq.unwrap_or(0), c.op))
        .collect();
    assert_eq!(ops, vec![(1, "added".to_owned()), (2, "added".to_owned())]);

    let _ = std::fs::remove_file(&path);
}

// ---------------------------------------------------- legacy OJS chains (09/10/11)

const R209: &str = "ted-export-r209";
const TEXT: &str = "text";

fn sec(id: &str, kind: &str, parent: Option<&str>) -> Section {
    Section { id: id.into(), kind: kind.into(), parent: parent.map(str::to_owned) }
}

/// An OJS chain edge: an `is_ref` id with scheme "ojs", exactly as the legacy
/// parsers emit `REF_NOTICE/NO_DOC_OJS`, `NOTICE_NUMBER_OJ` and text-era `RN`.
fn ojs_edge(section: &str, field: &str, target: &str) -> ValueRow {
    ValueRow {
        section_id: section.into(),
        field_id: field.into(),
        ordinal: 0,
        value: NoticeValue::Id { scheme: Some("ojs".into()), value: target.into(), is_ref: true },
    }
}

fn ted_text(section: &str, field: &str, value: &str) -> ValueRow {
    ValueRow {
        section_id: section.into(),
        field_id: field.into(),
        ordinal: 0,
        value: NoticeValue::Text { lang: Some("ENG".into()), value: value.into() },
    }
}

fn ted_date(section: &str, field: &str, utc: i64) -> ValueRow {
    ValueRow {
        section_id: section.into(),
        field_id: field.into(),
        ordinal: 0,
        value: NoticeValue::Date { utc_seconds: utc, offset_minutes: 0, has_time: false },
    }
}

fn ted_amount(section: &str, field: &str, cents: i64) -> ValueRow {
    ValueRow {
        section_id: section.into(),
        field_id: field.into(),
        ordinal: 0,
        value: NoticeValue::Amount { cents, currency: "EUR".into() },
    }
}

fn ted_ref(section: &str, field: &str, target: &str) -> ValueRow {
    ValueRow {
        section_id: section.into(),
        field_id: field.into(),
        ordinal: 0,
        value: NoticeValue::Id { scheme: None, value: target.into(), is_ref: true },
    }
}

fn legacy_record(fetch_id: i64, publication_id: &str, profile: &str, parsed: Parsed) -> (Notice, Parse) {
    (
        Notice {
            source: SOURCE.into(),
            publication_id: publication_id.into(),
            content_hash: format!("hash-{publication_id}"),
            profile: profile.into(),
            declared_version: None,
            fetch_id,
            member_path: format!("{publication_id}.xml"),
            ingested_at: 0,
            published_at: None,
            dispatched_at: None,
        },
        Parse::Parsed(parsed),
    )
}

/// A legacy contract notice: a title, a dispatch date (for ordering), a
/// submission deadline, and any OJS back-references.
fn legacy_cn(fetch_id: i64, pub_id: &str, day: i64, deadline: i64, refs: &[&str]) -> (Notice, Parse) {
    let mut parsed = Parsed {
        sections: vec![sec("PROCEDURE", "Notice", None)],
        values: vec![
            ted_text("PROCEDURE", "TED-TITLE", "Roof works"),
            ted_date("PROCEDURE", "TED-DS_DATE_DISPATCH", day * 86_400),
            ted_date("PROCEDURE", "TED-DATE_RECEIPT_TENDERS", deadline),
        ],
    };
    for r in refs {
        parsed.values.push(ojs_edge("PROCEDURE", "TED-REF_NOTICE.NO_DOC_OJS", r));
    }
    legacy_record(fetch_id, pub_id, R209, parsed)
}

/// A legacy award notice: an `AWARD_CONTRACT` (RES-) block naming its winner
/// inline and carrying the awarded value, referencing a previous publication.
fn legacy_award(
    fetch_id: i64,
    pub_id: &str,
    day: i64,
    winner: &str,
    cents: i64,
    refs: &[&str],
) -> (Notice, Parse) {
    let mut parsed = Parsed {
        sections: vec![
            sec("PROCEDURE", "Notice", None),
            sec("RES-1", "LotResult", Some("PROCEDURE")),
            sec("ORG-1", "Organization", Some("RES-1")),
        ],
        values: vec![
            ted_text("PROCEDURE", "TED-TITLE", "Roof works — award"),
            ted_date("PROCEDURE", "TED-DS_DATE_DISPATCH", day * 86_400),
            // the inline winner address block and its role reference
            ted_text("ORG-1", "TED-OFFICIALNAME", winner),
            ted_ref("RES-1", "TED-ADDRESS_CONTRACTOR", "ORG-1"),
            ted_amount("RES-1", "TED-VAL_TOTAL", cents),
            ValueRow {
                section_id: "RES-1".into(),
                field_id: "TED-NB_TENDERS_RECEIVED".into(),
                ordinal: 0,
                value: NoticeValue::Integer(4),
            },
        ],
    };
    for r in refs {
        parsed.values.push(ojs_edge("PROCEDURE", "TED-REF_NOTICE.NO_DOC_OJS", r));
    }
    legacy_record(fetch_id, pub_id, R209, parsed)
}

/// Legacy notices chain into one Tender by transitive OJS reference, keyed by
/// the earliest publication, ordered by dispatch date, and the award section
/// resolves its winner and value directly.
#[tokio::test]
async fn legacy_notices_chain_into_one_tender_by_ojs_reference() {
    let (db, fetch_id, path) = scratch("legacy-chain").await;
    // Ingested out of publication order; the award references the CN by its OJS
    // display form, the CN is the root.
    let (award, pa) = legacy_award(fetch_id, "000200-2019", 30, "Builders Ltd", 1_500_000, &["2019/S 001-000001"]);
    let (cn, pc) = legacy_cn(fetch_id, "000001-2019", 5, 728_000_000, &[]);
    db.record_notice(&award, &pa).await.expect("award");
    db.record_notice(&cn, &pc).await.expect("cn");

    let report = project::project(&db, false).await.expect("project");
    assert_eq!(report.notices, 2);
    assert_eq!(report.tenders, 1, "the award chains onto its contract notice");
    assert_eq!(report.islands, 0);

    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM tender_versions").await, 2);
    assert_eq!(
        query_text(&db, "SELECT procedure_key FROM tenders").await.as_deref(),
        Some("ojs:2019-000001"),
        "the Tender is keyed by the earliest OJS number in the component"
    );
    // Publication order is dispatch order: the CN (day 5) before the award (30).
    assert_eq!(
        query_text(&db, "SELECT publication_id FROM tender_versions WHERE seq = 1").await.as_deref(),
        Some("000001-2019")
    );
    // The award resolves winner and value directly from the inline block.
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM lot_results").await, 1);
    assert_eq!(scalar(&db, "SELECT awarded_cents FROM v_lot_results").await, 1_500_000);
    assert_eq!(
        query_text(
            &db,
            "SELECT o.name FROM v_lot_results v JOIN organizations o ON o.id = v.winner_organization_id"
        )
        .await
        .as_deref(),
        Some("Builders Ltd")
    );
    // The Tender carries the title as a canonical fact (legacy TED-TITLE mapping).
    assert!(query_text(&db, "SELECT title FROM v_tenders").await.is_some());

    let _ = std::fs::remove_file(&path);
}

/// A late edge that joins two existing Tenders is an ADR-0003-style merge: the
/// members re-project under the surviving earliest-OJS key and the absorbed
/// key's rows are retired with `removed` change events.
#[tokio::test]
async fn a_late_edge_merges_two_legacy_tenders() {
    let (db, fetch_id, path) = scratch("legacy-merge").await;
    // Two independent contract notices, each its own Tender.
    let (cn1, p1) = legacy_cn(fetch_id, "000001-2019", 5, 728_000_000, &[]);
    let (cn2, p2) = legacy_cn(fetch_id, "000002-2019", 6, 728_100_000, &[]);
    db.record_notice(&cn1, &p1).await.expect("cn1");
    db.record_notice(&cn2, &p2).await.expect("cn2");
    let first = project::project(&db, false).await.expect("project 1");
    assert_eq!(first.tenders, 2, "two unconnected components");
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM tenders").await, 2);

    // A bridging notice referencing BOTH: REF_NOTICE to 2019/1 and
    // NOTICE_NUMBER_OJ to 2019/2. It arrives later and joins the components.
    let mut bridge = Parsed {
        sections: vec![sec("PROCEDURE", "Notice", None)],
        values: vec![
            ted_text("PROCEDURE", "TED-TITLE", "Corrigendum bridging both"),
            ted_date("PROCEDURE", "TED-DS_DATE_DISPATCH", 40 * 86_400),
            ojs_edge("PROCEDURE", "TED-REF_NOTICE.NO_DOC_OJS", "000001-2019"),
            ojs_edge("PROCEDURE", "TED-NOTICE_NUMBER_OJ", "000002-2019"),
        ],
    };
    bridge.sections.push(sec("CHG-1", "Change", Some("PROCEDURE")));
    let (bn, pb) = legacy_record(fetch_id, "000300-2019", R209, bridge);
    db.record_notice(&bn, &pb).await.expect("bridge");

    let merged = project::project(&db, false).await.expect("project 2");
    assert_eq!(merged.absorbed, 1, "one component was absorbed into the other");
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM tenders").await, 1, "one surviving Tender");
    assert_eq!(
        query_text(&db, "SELECT procedure_key FROM tenders").await.as_deref(),
        Some("ojs:2019-000001"),
        "the survivor is the earliest OJS number"
    );
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM tender_versions").await, 3);
    // The absorbed identity emitted a `removed` tender change event.
    assert!(
        changes(&db, 0, 1000).await
            .iter()
            .any(|c| c.entity_kind == "tender" && c.op == "removed"),
        "the absorbed Tender was retired with a removed event"
    );
    // Re-projection is now a no-op — the merge is stable.
    let again = project::project(&db, false).await.expect("project 3");
    assert_eq!(again.absorbed, 0);
    assert_eq!(again.applied.versions_written, 0);

    let _ = std::fs::remove_file(&path);
}

/// An F14 corrigendum joins its referenced Tender as a version event and its
/// typed NEW_VALUE date supersedes the submission deadline — the legacy answer
/// to ADR-0001's "how did the deadline move?".
#[tokio::test]
async fn an_f14_corrigendum_moves_the_deadline_as_a_version_event() {
    let (db, fetch_id, path) = scratch("legacy-f14").await;
    let (cn, pc) = legacy_cn(fetch_id, "000001-2019", 5, 728_000_000, &[]);
    // The F14 references the CN and publishes a new deadline as a typed change.
    let corrigendum = Parsed {
        sections: vec![sec("PROCEDURE", "Notice", None), sec("CHG-1", "Change", Some("PROCEDURE"))],
        values: vec![
            ted_date("PROCEDURE", "TED-DS_DATE_DISPATCH", 20 * 86_400),
            ojs_edge("PROCEDURE", "TED-REF_NOTICE.NO_DOC_OJS", "000001-2019"),
            ted_date("CHG-1", "TED-NEW_VALUE.DATE", 728_600_000),
            ted_text("CHG-1", "TED-NEW_VALUE.TEXT", "Deadline extended"),
        ],
    };
    let (f14, pf) = legacy_record(fetch_id, "000119-2019", R209, corrigendum);
    db.record_notice(&cn, &pc).await.expect("cn");
    db.record_notice(&f14, &pf).await.expect("f14");
    project::project(&db, false).await.expect("project");

    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM tenders").await, 1);
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM tender_versions").await, 2);
    assert_eq!(deadline(&db, 1).await, 728_000_000, "the CN's original deadline");
    assert_eq!(deadline(&db, 2).await, 728_600_000, "the corrigendum moved it");
    // The change log records the corrigendum as a `changed` version event.
    assert!(
        changes(&db, 0, 100).await
            .iter()
            .any(|c| c.entity_kind == "tender" && c.op == "changed" && c.version_seq == Some(2))
    );

    let _ = std::fs::remove_file(&path);
}

/// XML-era chains cross into the text era backwards: a 2011 award referencing a
/// text-era `RN` number terminates at the real text-era record rather than
/// dangling, forming one cross-era Tender.
#[tokio::test]
async fn xml_era_chains_terminate_at_a_text_era_record() {
    let (db, fetch_id, path) = scratch("cross-era").await;
    // A text-era record, publication id in the text-era `number-year` form.
    let text_record = Parsed {
        sections: vec![sec("PROCEDURE", "Notice", None)],
        values: vec![
            ted_text("PROCEDURE", "TXT-TI", "Historic contract notice"),
            ted_date("PROCEDURE", "TXT-DS", 1_100_000_000),
        ],
    };
    let (text, pt) = legacy_record(fetch_id, "295856-2007", TEXT, text_record);
    // A 2011 award referencing that 2007 text-era number.
    let (award, pa) =
        legacy_award(fetch_id, "000181-2011", 400, "Old Winner SA", 900_000, &["2007/S 243-295856"]);
    db.record_notice(&text, &pt).await.expect("text");
    db.record_notice(&award, &pa).await.expect("award");
    let report = project::project(&db, false).await.expect("project");

    assert_eq!(report.tenders, 1, "the 2011 award chains onto the 2007 text-era record");
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM tender_versions").await, 2);
    assert_eq!(
        query_text(&db, "SELECT procedure_key FROM tenders").await.as_deref(),
        Some("ojs:2007-295856"),
        "keyed by the earliest (text-era) publication"
    );

    let _ = std::fs::remove_file(&path);
}

/// The unchained-award metric per era: an award that never chained to a
/// contract notice is a single-notice award Tender; a chained one is not.
#[tokio::test]
async fn unchained_awards_are_counted_per_era() {
    let (db, fetch_id, path) = scratch("unchained").await;
    // A CN + its award = one chained (multi-notice) award Tender.
    let (cn, pc) = legacy_cn(fetch_id, "000001-2019", 5, 728_000_000, &[]);
    let (chained, pch) =
        legacy_award(fetch_id, "000200-2019", 30, "Chained Winner", 100, &["000001-2019"]);
    // An award referencing nothing = one unchained single-notice award Tender.
    let (lone, pl) = legacy_award(fetch_id, "000500-2019", 40, "Lone Winner", 200, &[]);
    for (n, p) in [(&cn, &pc), (&chained, &pch), (&lone, &pl)] {
        db.record_notice(n, p).await.expect("notice");
    }
    project::project(&db, false).await.expect("project");

    let linkage = db.award_linkage().await.expect("linkage");
    let r209 = linkage.iter().find(|(era, ..)| era == R209).expect("an r209 row");
    assert_eq!(r209.1, 2, "two award Tenders (the chained one and the lone one)");
    assert_eq!(r209.2, 1, "exactly one is a single-notice, unchained award");

    let _ = std::fs::remove_file(&path);
}

/// The era boundary holds: an eForms notice never joins a legacy OJS chain even
/// when a legacy reference collides with its publication number — straddling
/// procedures are two Tenders (the accepted decision).
#[tokio::test]
async fn eforms_notices_do_not_join_legacy_chains() {
    let (db, fetch_id, path) = scratch("era-boundary").await;
    // An eForms notice whose publication number is 000900-2024.
    let eforms = Parsed {
        sections: vec![sec("PROCEDURE", "Procedure", None)],
        values: vec![ValueRow {
            section_id: "PROCEDURE".into(),
            field_id: "BT-04-notice".into(),
            ordinal: 0,
            value: NoticeValue::Id { scheme: None, value: "efp-1".into(), is_ref: false },
        }],
    };
    let (ef, pe) = legacy_record(fetch_id, "000900-2024", "eforms:eforms-sdk-1.13", eforms);
    // A legacy notice referencing 2024/S ...-000900 — the same OJS number.
    let (award, pa) =
        legacy_award(fetch_id, "000901-2024", 50, "Legacy Winner", 300, &["2024/S 010-000900"]);
    db.record_notice(&ef, &pe).await.expect("eforms");
    db.record_notice(&award, &pa).await.expect("legacy");
    let report = project::project(&db, false).await.expect("project");

    assert_eq!(report.tenders, 2, "the eForms notice stays its own Tender");
    // The eForms notice is keyed by BT-04, the legacy award by its own OJS number.
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM tenders WHERE procedure_key = 'efp-1'").await, 1);
    assert_eq!(
        scalar(&db, "SELECT COUNT(*) FROM tenders WHERE procedure_key LIKE 'ojs:%'").await,
        1
    );

    let _ = std::fs::remove_file(&path);
}

/// A correction — a change notice republishing the same logical notice
/// (BT-701) — replaces the round it corrects instead of double-counting it
/// (ted-empirical-checks.md: same BT-701 + efac:Changes ⇒ correction;
/// otherwise versions are additive).
#[tokio::test]
async fn a_correction_replaces_its_round_instead_of_duplicating_it() {
    let (db, fetch_id, path) = scratch("correction").await;

    let notice = |publication_id: &str, day: i64, cents: i64, corrects: bool| {
        let section = |id: &str, kind: &str, parent: Option<&str>| Section {
            id: id.into(),
            kind: kind.into(),
            parent: parent.map(str::to_owned),
        };
        let id_value = |section: &str, field: &str, value: &str, is_ref: bool| ValueRow {
            section_id: section.into(),
            field_id: field.into(),
            ordinal: 0,
            value: NoticeValue::Id { scheme: None, value: value.into(), is_ref },
        };
        let code = |section: &str, field: &str, value: &str| ValueRow {
            section_id: section.into(),
            field_id: field.into(),
            ordinal: 0,
            value: NoticeValue::Code { list: None, code: value.into() },
        };
        let mut parsed = Parsed {
            sections: vec![
                section("PROCEDURE", "Procedure", None),
                section("LOT-0001", "Lot", Some("PROCEDURE")),
                section("RES-0001", "LotResult", Some("PROCEDURE")),
                section("TEN-0001", "LotTender", Some("PROCEDURE")),
                section("TPA-0001", "TenderingParty", Some("PROCEDURE")),
                section("CON-0001", "SettledContract", Some("PROCEDURE")),
                section("ORG-0001", "Organization", Some("PROCEDURE")),
            ],
            values: vec![
                id_value("PROCEDURE", "BT-04-notice", "proc-correction", false),
                id_value("PROCEDURE", "BT-701-notice", "LOGICAL-1", false),
                ValueRow {
                    section_id: "PROCEDURE".into(),
                    field_id: "OPP-012-notice".into(),
                    ordinal: 0,
                    value: NoticeValue::Date {
                        utc_seconds: day * 86_400,
                        offset_minutes: 0,
                        has_time: false,
                    },
                },
                code("RES-0001", "BT-142-LotResult", "selec-w"),
                id_value("RES-0001", "BT-13713-LotResult", "LOT-0001", true),
                id_value("RES-0001", "OPT-315-LotResult", "CON-0001", true),
                ValueRow {
                    section_id: "TEN-0001".into(),
                    field_id: "BT-720-Tender".into(),
                    ordinal: 0,
                    value: NoticeValue::Amount { cents, currency: "EUR".into() },
                },
                id_value("TEN-0001", "BT-13714-Tender", "LOT-0001", true),
                id_value("TEN-0001", "OPT-310-Tender", "TPA-0001", true),
                id_value("TPA-0001", "OPT-300-Tenderer", "ORG-0001", true),
                id_value("CON-0001", "BT-3202-Contract", "TEN-0001", true),
                ValueRow {
                    section_id: "ORG-0001".into(),
                    field_id: "BT-500-Organization-Company".into(),
                    ordinal: 0,
                    value: NoticeValue::Text { lang: Some("ENG".into()), value: "Winner GmbH".into() },
                },
                id_value("ORG-0001", "BT-501-Organization-Company", "DE123456789", false),
            ],
        };
        if corrects {
            parsed.sections.push(section("ND-Change#0", "Change", Some("PROCEDURE")));
        }
        (
            Notice {
                source: SOURCE.into(),
                publication_id: publication_id.into(),
                content_hash: format!("hash-{publication_id}"),
                profile: "eforms:eforms-sdk-1.13".into(),
                declared_version: None,
                fetch_id,
                member_path: format!("{publication_id}.xml"),
                ingested_at: 0,
                published_at: None,
                dispatched_at: None,
            },
            Parse::Parsed(parsed),
        )
    };

    let (original, parse_a) = notice("20000001-2026", 1, 10_000, false);
    let (correction, parse_b) = notice("20000002-2026", 2, 25_000, true);
    db.record_notice(&original, &parse_a).await.expect("original");
    db.record_notice(&correction, &parse_b).await.expect("correction");
    project::project(&db, false).await.expect("project");

    // One Tender, two versions — and the corrected version holds exactly one
    // round: the correction replaced LOGICAL-1's results, it did not add a
    // duplicate award.
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM tender_versions").await, 2);
    assert_eq!(
        scalar(&db, "SELECT COUNT(*) FROM tender_version_lot_results WHERE seq = 2").await,
        1
    );
    assert_eq!(scalar(&db, "SELECT awarded_cents FROM v_lot_results").await, 25_000);
    // The corrected result's origin is the correction notice.
    assert_eq!(
        scalar(
            &db,
            "SELECT COUNT(*) FROM v_lot_results v JOIN notices n ON n.id = v.notice_id
              WHERE n.publication_id = '20000002-2026'"
        )
        .await,
        1
    );
    // Winner resolution across the synthetic graph, and the lot-scoped role.
    assert_eq!(
        query_text(
            &db,
            "SELECT o.name FROM v_lot_results v JOIN organizations o ON o.id = v.winner_organization_id"
        )
        .await
        .as_deref(),
        Some("Winner GmbH")
    );
    assert_eq!(
        scalar(
            &db,
            "SELECT COUNT(*) FROM tender_version_parties
              WHERE role = 'Tenderer' AND lot_id IS NOT NULL"
        )
        .await,
        2,
        "the Tenderer role is Lot-scoped in both versions"
    );
    // The diff reads replacement: the original round removed, the corrected
    // one added.
    let ops: Vec<(i64, String)> = changes(&db, 0, 100).await
        .into_iter()
        .filter(|c| c.entity_kind == "lot_result")
        .map(|c| (c.version_seq.unwrap_or(0), c.op))
        .collect();
    assert_eq!(
        ops,
        vec![(1, "added".to_owned()), (2, "added".to_owned()), (2, "removed".to_owned())]
    );

    let _ = std::fs::remove_file(&path);
}

// -------------------------------------------------- issue 18: publication dates

/// `notice_instants` sources publication and dispatch per era: OJEU stamp,
/// legacy OJ date, DÖE requested/portal date, with dispatch its own axis.
#[test]
fn published_and_dispatched_resolve_per_era() {
    use store::{NoticeValue, ValueRow};
    let date = |field: &str, utc: i64| ValueRow {
        section_id: "PROCEDURE".into(),
        field_id: field.into(),
        ordinal: 0,
        value: NoticeValue::Date { utc_seconds: utc, offset_minutes: 0, has_time: false },
    };
    let instants =
        |rows: Vec<ValueRow>| project::notice_instants(&Parsed { sections: vec![], values: rows });

    // TED eForms: the OJEU PublicationDate (OPP-012) over the dispatch (BT-05).
    assert_eq!(
        instants(vec![date("BT-05(a)-notice", 100), date("OPP-012-notice", 200)]),
        (200, Some(100))
    );
    // DÖE eforms-de: no OJEU stamp → the requested/portal date; dispatch kept.
    assert_eq!(
        instants(vec![date("BT-05(a)-notice", 100), date("BT-738-notice", 250)]),
        (250, Some(100))
    );
    // DÖE sdk-0.1 numeric island: only a requested publication date, no dispatch.
    assert_eq!(instants(vec![date("SDK01-RequestedPublicationDate", 300)]), (300, None));
    // DÖE sdk-0.1 with an issue date as its dispatch.
    assert_eq!(
        instants(vec![date("SDK01-IssueDate", 90), date("SDK01-RequestedPublicationDate", 300)]),
        (300, Some(90))
    );
    // Legacy TED: the OJ DATE_PUB over the dispatch fields.
    assert_eq!(
        instants(vec![date("TED-DS_DATE_DISPATCH", 10), date("TED-DATE_PUB", 20)]),
        (20, Some(10))
    );
    // Text era: PD over DS.
    assert_eq!(instants(vec![date("TXT-DS", 5), date("TXT-PD", 8)]), (8, Some(5)));
    // Dispatch-only notice: published_at falls back to it.
    assert_eq!(instants(vec![date("BT-05(a)-notice", 100)]), (100, Some(100)));
}

/// A real TED eForms CAN stores the OJEU publication date as `published_at` and
/// the (earlier) dispatch date as `dispatched_at`, on both the version and the
/// notice row (issue 18).
#[tokio::test]
async fn a_ted_eforms_notice_stores_publication_and_dispatch_separately() {
    let (db, fetch_id, path) = scratch("dates").await;
    ingest(&db, fetch_id, "eforms/can-29-00495054-2026.xml").await;
    project::project(&db, false).await.expect("project");

    let opp012 =
        scalar(&db, "SELECT utc_seconds FROM notice_dates WHERE field_id = 'OPP-012-notice'").await;
    let bt05 =
        scalar(&db, "SELECT utc_seconds FROM notice_dates WHERE field_id = 'BT-05(a)-notice'").await;
    assert!(opp012 > bt05, "the OJEU publication is after dispatch");

    assert_eq!(
        scalar(&db, "SELECT published_at FROM tender_versions").await,
        opp012,
        "published_at is the OJEU publication date"
    );
    assert_eq!(
        scalar(&db, "SELECT dispatched_at FROM tender_versions").await,
        bt05,
        "dispatched_at is the notice dispatch date"
    );
    // The notice row carries the same pair for the /v1/notices surface.
    assert_eq!(scalar(&db, "SELECT published_at FROM notices").await, opp012);
    assert_eq!(scalar(&db, "SELECT dispatched_at FROM notices").await, bt05);

    let _ = std::fs::remove_file(&path);
}


// ----------------------------------------------- issue 12: cross-source merge

/// One procedure published on both TED and DÖE (a shared BT-04 UUID) collapses
/// into a single Tender: TED publication identity, DÖE content retained
/// (ADR-0003).
#[tokio::test]
async fn a_procedure_on_both_sources_merges_into_one_tender() {
    let (db, fetch_id, path) = scratch("pair").await;
    ingest_from(&db, fetch_id, "ted", "doe-ted-pair/ted-cn-00373130-2026.xml").await;
    ingest_from(
        &db,
        fetch_id,
        "doe",
        "doe-ted-pair/doe-cn-ebb72363-832d-4cea-8db6-04999414ea8c-01.xml",
    )
    .await;

    let report = project::project(&db, false).await.expect("project");
    assert_eq!(report.notices, 2);
    assert_eq!(report.tenders, 1, "one procedure, one Tender across both Sources");

    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM tenders").await, 1);
    assert_eq!(
        scalar(&db, "SELECT COUNT(*) FROM tender_versions").await,
        2,
        "one version per Source notice, interleaved in one chain"
    );
    // ADR-0003: the shared BT-04 UUID identifies the Tender, and its primary
    // Source label is TED (publication identity from the OJEU gazette).
    assert_eq!(
        query_text(&db, "SELECT procedure_key FROM tenders").await.as_deref(),
        Some("1af86e3c-411f-4c2e-aacc-ecac61717472"),
        "both notices share one BT-04 procedure UUID"
    );
    assert_eq!(query_text(&db, "SELECT source FROM tenders").await.as_deref(), Some("ted"));
    // The TED reading is a version of the one Tender (its publication id is the
    // identity)...
    assert_eq!(
        scalar(
            &db,
            "SELECT COUNT(*) FROM tender_versions v JOIN notices n ON n.id = v.caused_by_notice_id
              WHERE n.source = 'ted'"
        )
        .await,
        1,
    );
    // ...and DÖE's richer national content is retained in the notice layer.
    assert!(
        scalar(
            &db,
            "SELECT COUNT(*) FROM notice_codes c JOIN notices n ON n.id = c.notice_id
              WHERE n.source = 'doe'"
        )
        .await
            > 0,
        "the DÖE notice's national codes are present"
    );

    let _ = std::fs::remove_file(&path);
}

// ------------------------------------------------- issue 29: DÖE sdk-0.1 mapping

/// The DÖE sdk-0.1 dialect (~40 % of German volume) must project its `SDK01-*`
/// content into the canonical layer like any other era — before issue 29 these
/// notices parsed cleanly but projected to empty island Tenders (0 % on every
/// field). Its buyer is an inline `ContractingParty` and its winner an inline
/// `WinningParty`, neither an eForms `Organization` section.
#[tokio::test]
async fn sdk01_projects_title_buyer_and_winner() {
    let (db, fetch_id, path) = scratch("sdk01").await;
    ingest_from(&db, fetch_id, "doe", "doe/sdk-0.1-numeric-cn-25599482-1.xml").await;
    ingest_from(&db, fetch_id, "doe", "doe/sdk-0.1-uuid-can-427d4645-163c-419d-93a9-5f5ce05ff9b7-1.xml").await;
    project::project(&db, false).await.expect("project");

    // Title and description, at Tender scope, resolved from SDK01-ProcurementProject-*.
    assert_eq!(
        query_text(
            &db,
            "SELECT value FROM tender_version_texts \
              WHERE field = 'title' AND lot_id IS NULL AND value = 'Lose Möblierung' LIMIT 1"
        )
        .await,
        Some("Lose Möblierung".to_owned()),
        "the CN's title projects from SDK01-ProcurementProject-Name",
    );
    assert!(scalar(&db, "SELECT COUNT(*) FROM tender_version_texts WHERE field = 'description'").await > 0);
    // The realized-location NUTS and the lot's submission deadline.
    assert!(
        scalar(&db, "SELECT COUNT(*) FROM tender_version_classifications WHERE scheme = 'nuts' AND field = 'place'").await > 0
    );
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM tender_version_dates WHERE field = 'submission_deadline'").await, 1);

    // The buyer: the inline ContractingParty becomes a party with role 'buyer'.
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM tender_version_parties WHERE role = 'buyer'").await, 2);
    assert!(
        query_text(
            &db,
            "SELECT o.name FROM tender_version_parties p JOIN organizations o ON o.id = p.organization_id
              WHERE p.role = 'buyer' AND o.name LIKE 'VGem Volkach%' LIMIT 1"
        )
        .await
        .is_some(),
        "the ContractingParty is the buyer"
    );

    // The winner: the CAN's TenderResult materialises a lot_result naming the WinningParty.
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM lot_results").await, 1);
    assert_eq!(
        query_text(
            &db,
            "SELECT o.name FROM tender_version_result_winners w JOIN organizations o ON o.id = w.organization_id LIMIT 1"
        )
        .await,
        Some("1. Firma: IABG mbH".to_owned()),
        "the WinningParty is the resolved winner"
    );

    let _ = std::fs::remove_file(&path);
}

// ---------------------------------- issue 34: sdk-0.1 ContractFolderID as a key

/// A minimal synthetic notice carrying a single id field on its PROCEDURE root —
/// enough to exercise Tender identity/merging without a full fixture.
async fn record_key_only(db: &Db, fetch_id: i64, source: &str, pub_id: &str, profile: &str, field: &str, value: &str) {
    let parsed = Parsed {
        sections: vec![Section { id: "PROCEDURE".into(), kind: "Notice".into(), parent: None }],
        values: vec![ValueRow {
            section_id: "PROCEDURE".into(),
            field_id: field.into(),
            ordinal: 0,
            value: NoticeValue::Id { scheme: None, value: value.into(), is_ref: false },
        }],
    };
    let notice = Notice {
        source: source.into(),
        publication_id: pub_id.into(),
        content_hash: pub_id.into(),
        profile: profile.into(),
        declared_version: None,
        fetch_id,
        member_path: pub_id.into(),
        ingested_at: 0,
        published_at: Some(0),
        dispatched_at: None,
    };
    db.record_notice(&notice, &Parse::Parsed(parsed)).await.expect("record synthetic");
}

/// A uuid-bearing sdk-0.1 notice merges with its TED twin on the shared BT-04
/// uuid (ADR-0003), while a non-uuid folder id stays an island — the numeric
/// channel's local ids must never merge.
#[tokio::test]
async fn sdk01_uuid_folder_merges_with_ted_twin_but_non_uuid_stays_island() {
    let (db, fetch_id, path) = scratch("sdk01-merge").await;
    // The real sdk-0.1 CAN publishes ContractFolderID 3d2aac86-…; its TED twin
    // publishes the same uuid as BT-04.
    let shared = "3d2aac86-4286-4ae2-9bc1-08eb1cc61f80";
    ingest_from(&db, fetch_id, "doe", "doe/sdk-0.1-uuid-can-427d4645-163c-419d-93a9-5f5ce05ff9b7-1.xml").await;
    record_key_only(&db, fetch_id, "ted", "00499999-2026", "eforms:eforms-sdk-1.13", "BT-04-notice", shared).await;
    // A second sdk-0.1 notice whose folder id is a non-uuid local number.
    record_key_only(&db, fetch_id, "doe", "88887777-1", "eforms:eforms-sdk-0.1", "SDK01-ContractFolderID", "LOCAL-12345").await;

    let report = project::project(&db, false).await.expect("project");
    assert_eq!(report.notices, 3);

    // Two Tenders: the merged uuid one (DÖE + TED), and the non-uuid island.
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM tenders").await, 2);
    assert_eq!(
        scalar(&db, &format!("SELECT COUNT(*) FROM tenders WHERE procedure_key = '{shared}'")).await,
        1,
        "the shared uuid keys exactly one Tender",
    );
    // That Tender carries both Sources' notices — the merge.
    assert_eq!(
        scalar(
            &db,
            &format!(
                "SELECT COUNT(DISTINCT n.source) FROM tender_versions v \
                 JOIN notices n ON n.id = v.caused_by_notice_id \
                 JOIN tenders t ON t.id = v.tender_id WHERE t.procedure_key = '{shared}'"
            ),
        )
        .await,
        2,
        "DÖE and TED readings merged into one Tender",
    );
    // The non-uuid sdk-0.1 notice is an island (no procedure key).
    assert_eq!(
        scalar(&db, "SELECT COUNT(*) FROM tenders WHERE procedure_key IS NULL AND island_notice_id IS NOT NULL").await,
        1,
        "the non-uuid folder id stays an island",
    );

    let _ = std::fs::remove_file(&path);
}

// ------------------------------------------- issue 59: progress heartbeat

/// The projection reports a live heartbeat in BOTH phases (issue 59): a run over
/// millions of notices takes many minutes, so silence must never be mistaken for
/// a hang. Here a recording sink captures the events over a small corpus.
#[tokio::test]
async fn the_projection_reports_progress_in_both_phases() {
    let (db, fetch_id, path) = scratch("progress").await;
    for fixture in [
        "eforms/brin-x01-00497689-2026.xml",
        "eforms/pin-4-00496860-2026.xml",
        "eforms/cn-16-00494343-2026.xml",
    ] {
        ingest(&db, fetch_id, fixture).await;
    }

    let mut events: Vec<project::Progress> = Vec::new();
    project::project_with_progress(&db, false, 20_000, |p| events.push(p)).await.expect("project");

    // Phase 1 planning heartbeat, ending at the full total.
    let planning: Vec<_> =
        events.iter().filter_map(|e| match e {
            project::Progress::Planning { notices, total } => Some((*notices, *total)),
            _ => None,
        }).collect();
    assert!(!planning.is_empty(), "Phase 1 emitted no planning heartbeat");
    assert_eq!(planning.last().copied(), Some((3, 3)), "Phase 1 heartbeat reaches the full count");

    // The phase transition, and a Phase-2 apply heartbeat reaching every Tender.
    assert!(
        events.iter().any(|e| matches!(e, project::Progress::Grouped { tenders: 3, .. })),
        "no Grouped transition event"
    );
    let applied_max = events
        .iter()
        .filter_map(|e| match e {
            project::Progress::Applying { tenders, .. } => Some(*tenders),
            _ => None,
        })
        .max();
    assert_eq!(applied_max, Some(3), "Phase 2 apply heartbeat reaches every Tender");

    let _ = std::fs::remove_file(&path);
}

// -------------------------------------------------- the eForms-DE 1.x dialect

/// eForms-DE 1.x names every leaf by its element path (`DE1-*`) because the
/// national 1.x line shipped no SDK (issue 75). This is the guard the sdk-0.1
/// case lacked (issue 29 → 85): before the alias fold, all 218 876 reclaimed
/// DE-1.x notices projected to Tender versions carrying *nothing* — a version row
/// with no title, no CPV/NUTS, no amount, no lot and no buyer. Shaped after the
/// real prod notice 26195620 (eforms-de-1.1, DÖE), whose parse layer is rich and
/// whose Tender was empty.
fn de1_value(section: &str, field: &str, value: NoticeValue) -> ValueRow {
    ValueRow { section_id: section.into(), field_id: field.into(), ordinal: 0, value }
}

fn de1_notice(fetch_id: i64, pub_id: &str, profile: &str) -> (Notice, Parse) {
    de1_notice_keyed(fetch_id, pub_id, profile, "3f2504e0-4f89-41d3-9a0c-0305e82c3301")
}

fn de1_notice_keyed(fetch_id: i64, pub_id: &str, profile: &str, folder: &str) -> (Notice, Parse) {
    let parsed = Parsed {
        sections: vec![
            sec("PROCEDURE", "Notice", None),
            // The predicate-free inventory's single lot node: the kind is the
            // element name, and only the section id says it is a Lot.
            sec("LOT-0001", "ProcurementProjectLot", Some("PROCEDURE")),
            sec("ORG-0001", "Organization", Some("PROCEDURE")),
            sec("ND-PartyName#0", "PartyName", Some("ORG-0001")),
            sec("ND-ContractingParty#0", "ContractingParty", Some("PROCEDURE")),
            sec("ORG-0002", "Organization", Some("PROCEDURE")),
            sec("ND-PartyName#1", "PartyName", Some("ORG-0002")),
            sec("ND-AppealTerms#0", "AppealTerms", Some("PROCEDURE")),
        ],
        values: vec![
            de1_value(
                "PROCEDURE",
                "DE1-ContractFolderID",
                NoticeValue::Id { scheme: None, value: folder.into(), is_ref: false },
            ),
            de1_value(
                "PROCEDURE",
                "DE1-NoticeSubType-SubTypeCode",
                NoticeValue::Code { list: Some("notice-subtype".into()), code: "29".into() },
            ),
            de1_value(
                "PROCEDURE",
                "DE1-IssueDate",
                NoticeValue::Date { utc_seconds: 1_700_000_000, offset_minutes: 60, has_time: false },
            ),
            de1_value(
                "PROCEDURE",
                "DE1-ProcurementProject-Name",
                NoticeValue::Text {
                    lang: Some("DEU".into()),
                    value: "Neugestaltung der Alten Holstenstraße".into(),
                },
            ),
            de1_value(
                "PROCEDURE",
                "DE1-ProcurementProject-MainCommodityClassification-ItemClassificationCode",
                NoticeValue::Classification { scheme: "cpv".into(), code: "71240000".into() },
            ),
            de1_value(
                "LOT-0001",
                "DE1-ProcurementProjectLot-ProcurementProject-Name",
                NoticeValue::Text {
                    lang: Some("DEU".into()),
                    value: "Freianlagenplanung gem. §§ 38 HOAI".into(),
                },
            ),
            de1_value(
                "LOT-0001",
                "DE1-ProcurementProjectLot-ProcurementProject-Description",
                NoticeValue::Text {
                    lang: Some("DEU".into()),
                    value: "Auftragsgegenstand sind Planungsleistungen zur Entwicklung.".into(),
                },
            ),
            de1_value(
                "LOT-0001",
                "DE1-ProcurementProjectLot-ProcurementProject-RealizedLocation-Address-CountrySubentityCode",
                NoticeValue::Classification { scheme: "nuts".into(), code: "DE600".into() },
            ),
            de1_value(
                "LOT-0001",
                "DE1-ProcurementProjectLot-ProcurementProject-RequestedTenderTotal-EstimatedOverallContractAmount",
                NoticeValue::Amount { cents: 590_000_000, currency: "EUR".into() },
            ),
            de1_value(
                "LOT-0001",
                "DE1-ProcurementProjectLot-TenderingProcess-TenderSubmissionDeadlinePeriod-EndDate",
                NoticeValue::Date { utc_seconds: 1_705_000_000, offset_minutes: 60, has_time: true },
            ),
            // The buyer: an Organization section carrying name/id/country, pointed
            // at by the ContractingParty's reference (eForms' OPT-300 pattern).
            de1_value(
                "ND-PartyName#0",
                "DE1-Organizations-Organization-Company-PartyName-Name",
                NoticeValue::Text { lang: Some("DEU".into()), value: "Bezirksamt Bergedorf".into() },
            ),
            de1_value(
                "ORG-0001",
                "DE1-Organizations-Organization-Company-PostalAddress-Country-IdentificationCode",
                NoticeValue::Code { list: Some("country".into()), code: "DEU".into() },
            ),
            // `is_ref: FALSE` — and that is not an oversight, it is the whole point
            // of issue 98. The vendored DE-1.x inventory types every identifier
            // `id`, never `id-ref` (a reference is lexically indistinguishable from
            // an identifier, so the empirical generator could not tell them apart),
            // and `value::convert` derives `is_ref` from exactly that type. So no
            // DE-1.x reference ever reaches the projection flagged, and this fixture
            // must reproduce that or it tests a parse layer that does not exist.
            //
            // It previously said `true`, which is why this test passed green while
            // the cohort projected 0% buyers in production: the fixture asserted the
            // behaviour we wished the parse layer had.
            de1_value(
                "ND-ContractingParty#0",
                "DE1-ContractingParty-Party-PartyIdentification-ID",
                NoticeValue::Id { scheme: None, value: "ORG-0001".into(), is_ref: false },
            ),
            // A second role from the class the alias table did not cover at all
            // (issue 98): the review body, the single most frequent reference in
            // the real cohort at 693 values per 400 notices.
            de1_value(
                "ND-AppealTerms#0",
                "DE1-TenderingTerms-AppealTerms-AppealReceiverParty-PartyIdentification-ID",
                NoticeValue::Id { scheme: None, value: "ORG-0002".into(), is_ref: false },
            ),
            de1_value(
                "ND-PartyName#1",
                "DE1-Organizations-Organization-Company-PartyName-Name",
                NoticeValue::Text {
                    lang: Some("DEU".into()),
                    value: "Vergabekammer Hamburg".into(),
                },
            ),
        ],
    };
    legacy_record(fetch_id, pub_id, profile, parsed)
}

#[tokio::test]
async fn eforms_de_1x_path_shaped_fields_land_as_canonical_facts() {
    let (db, fetch_id, path) = scratch("de1x-facts").await;
    let (notice, parse) = de1_notice(fetch_id, "de1-000001", "eforms:eforms-de-1.1");
    db.record_notice(&notice, &parse).await.expect("de-1.1 notice");

    let report = project::project(&db, false).await.expect("project");
    assert_eq!(report.notices, 1);
    assert_eq!(report.tenders, 1);

    // The version is not a shell: every fact kind the dialect carries lands.
    assert_eq!(
        title(&db, 1).await.as_deref(),
        Some("Neugestaltung der Alten Holstenstraße"),
        "DE1-ProcurementProject-Name must fold onto the canonical title"
    );
    assert_eq!(
        query_text(
            &db,
            "SELECT value FROM tender_version_texts WHERE field = 'description' AND lot_id IS NOT NULL"
        )
        .await
        .as_deref(),
        Some("Auftragsgegenstand sind Planungsleistungen zur Entwicklung."),
        "the German lot description survives intact"
    );
    assert_eq!(
        query_text(&db, "SELECT code FROM tender_version_classifications WHERE field = 'main'")
            .await
            .as_deref(),
        Some("71240000"),
        "main-object CPV"
    );
    assert_eq!(
        query_text(&db, "SELECT code FROM tender_version_classifications WHERE field = 'place'")
            .await
            .as_deref(),
        Some("DE600"),
        "realized-location NUTS"
    );
    assert_eq!(
        scalar(&db, "SELECT cents FROM tender_version_amounts WHERE field = 'estimated_value'").await,
        590_000_000,
        "the lot's estimated value"
    );
    assert_eq!(deadline(&db, 1).await, 1_705_000_000, "the tender-submission deadline");

    // The predicate-free `ProcurementProjectLot` node still yields a Lot.
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM lots").await, 1, "the lot is projected");
    assert_eq!(
        query_text(&db, "SELECT lot_key FROM lots").await.as_deref(),
        Some("LOT-0001"),
        "and keeps its own key"
    );

    // The buyer resolves through the Organization register.
    assert_eq!(
        query_text(
            &db,
            "SELECT o.name FROM tender_version_parties p JOIN organizations o ON o.id = p.organization_id
              WHERE p.role LIKE '%uyer%'"
        )
        .await
        .as_deref(),
        Some("Bezirksamt Bergedorf"),
        "the ContractingParty reference must resolve to the buyer organization"
    );

    // Issue 98. The reference arrives `is_ref: false`, as the real parse layer
    // delivers it, so these two assertions FAIL on the pre-98 projection: without
    // `de1_mark_reference` the role arm never sees the reference and no party row
    // is written at all. This is the regression gate for the whole organization
    // class — the class that was 0% in production while this test was green.
    assert_eq!(
        scalar(&db, "SELECT COUNT(*) FROM tender_version_parties").await,
        2,
        "both organization references must become parties (issue 98)"
    );
    assert_eq!(
        query_text(
            &db,
            "SELECT o.name FROM tender_version_parties p JOIN organizations o ON o.id = p.organization_id
              WHERE p.role = 'Lot-ReviewOrg'"
        )
        .await
        .as_deref(),
        Some("Vergabekammer Hamburg"),
        "the review body — a role the alias table did not cover before issue 98"
    );
    // Provenance, not presence: a party must be evidenced by THIS notice's own
    // mention. In production the cohort showed a 35% buyer rate that was entirely
    // carried forward from merged TED twins (`mention_notice_id` pointing at the
    // twin), which is what made 0% look like success.
    assert_eq!(
        scalar(
            &db,
            "SELECT COUNT(*) FROM tender_version_parties p
              JOIN tender_versions v ON v.tender_id = p.tender_id AND v.seq = p.seq
             WHERE p.mention_notice_id = v.caused_by_notice_id"
        )
        .await,
        2,
        "every party must be evidenced by the DE-1.x notice itself, not inherited"
    );

    // Identity: the folder id keys the Tender (so a TED twin can merge onto it),
    // and the subtype is read for fold order.
    assert_eq!(
        query_text(&db, "SELECT procedure_key FROM tenders").await.as_deref(),
        Some("3f2504e0-4f89-41d3-9a0c-0305e82c3301"),
        "DE1-ContractFolderID is the BT-04 procedure key"
    );
    assert_eq!(
        query_text(&db, "SELECT notice_subtype FROM tender_versions").await.as_deref(),
        Some("29")
    );
    assert_eq!(
        scalar(&db, "SELECT published_at FROM tender_versions").await,
        1_700_000_000,
        "DE1-IssueDate resolves the instant (no publication stamp on a DÖE notice)"
    );

    let _ = std::fs::remove_file(&path);
}

/// The aspects of the canonical layer this fix must NOT move. Same shape as
/// `project_incremental.rs::snapshot` — grouping identity, the version chain, and
/// every fact satellite — minus `tender_version_parties`, which is the one table
/// issue 98 is allowed to change. Keep the two in sync if either grows a table.
const LAYER_DIGESTS: &[(&str, &str)] = &[
    ("tenders", "SELECT group_concat(r, x'0a') FROM (SELECT id||'|'||coalesce(procedure_key,'')||'|'||coalesce(island_notice_id,-1)||'|'||kind||'|'||source AS r FROM tenders ORDER BY id)"),
    ("versions", "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||caused_by_notice_id||'|'||published_at AS r FROM tender_versions ORDER BY tender_id, seq)"),
    ("texts", "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||field||'|'||coalesce(lang,'')||'|'||value||'|'||coalesce(lot_id,-1) AS r FROM tender_version_texts ORDER BY tender_id, seq, field, lang, value, lot_id)"),
    ("classifications", "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||field||'|'||scheme||'|'||code||'|'||coalesce(lot_id,-1) AS r FROM tender_version_classifications ORDER BY tender_id, seq, field, scheme, code, lot_id)"),
    ("amounts", "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||field||'|'||cents||'|'||currency||'|'||coalesce(lot_id,-1) AS r FROM tender_version_amounts ORDER BY tender_id, seq, field, cents, lot_id)"),
    ("dates", "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||field||'|'||utc_seconds||'|'||coalesce(lot_id,-1) AS r FROM tender_version_dates ORDER BY tender_id, seq, field, utc_seconds, lot_id)"),
    ("lots", "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||lot_key AS r FROM lots ORDER BY tender_id, lot_key)"),
    ("version_lots", "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||lot_id||'|'||kind AS r FROM tender_version_lots ORDER BY tender_id, seq, lot_id)"),
    ("lot_results", "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||lot_result_id||'|'||coalesce(decision,'') AS r FROM tender_version_lot_results ORDER BY tender_id, seq, lot_result_id)"),
    ("organizations", "SELECT group_concat(r, x'0a') FROM (SELECT id||'|'||coalesce(country,'')||'|'||coalesce(identifier,'')||'|'||name||'|'||provisional AS r FROM organizations ORDER BY id)"),
    ("mentions", "SELECT group_concat(r, x'0a') FROM (SELECT notice_id||'|'||section_id||'|'||organization_id AS r FROM organization_mentions ORDER BY notice_id, section_id)"),
];

/// Issue 98 must be **surgical**: it may add party rows and move nothing else.
///
/// A gate asserting only "parties are now non-zero" would pass a fix that also
/// perturbed the fact layer or the grouping — and grouping is what a re-fold
/// renumbers, so a silent perturbation there is the expensive kind of wrong.
///
/// The two inputs differ in exactly one respect: whether the notice carries its
/// organization-role references at all. That isolates the fix, because a
/// reference the pre-98 projection could not see is *behaviourally identical to
/// an absent one*: `is_ref` gates only the role arm, and both the role arm and
/// the fall-through produce no `Fact`. `first_id` — which resolves the procedure
/// key, and so the grouping — matches `Id { value, .. }` and never reads
/// `is_ref`, so identity cannot move either.
///
/// So: every digest identical, parties the sole difference. That is the same
/// invariant the post-re-fold verification must see against production — same
/// tender, version, fact, lot and result counts, only party rows appearing. A
/// tender or version count that MOVES is a stop-and-investigate signal, not a
/// proceed.
#[tokio::test]
async fn the_de1_reference_flag_adds_parties_and_moves_nothing_else() {
    let (with_refs, f1, p1) = scratch("de1x-refs-on").await;
    let (without_refs, f2, p2) = scratch("de1x-refs-off").await;

    let (notice, parse) = de1_notice(f1, "de1-000001", "eforms:eforms-de-1.1");
    with_refs.record_notice(&notice, &parse).await.expect("with refs");

    // The same notice with only the organization-role references removed.
    let (notice, mut parse) = de1_notice(f2, "de1-000001", "eforms:eforms-de-1.1");
    if let store::Parse::Parsed(parsed) = &mut parse {
        parsed.values.retain(|v| {
            !v.field_id.ends_with("PartyIdentification-ID") && !v.field_id.ends_with("Tenderer-ID")
        });
    }
    without_refs.record_notice(&notice, &parse).await.expect("without refs");

    project::project(&with_refs, false).await.expect("project with refs");
    project::project(&without_refs, false).await.expect("project without refs");

    // The references are the only source of parties, and they do produce them.
    assert_eq!(
        scalar(&without_refs, "SELECT COUNT(*) FROM tender_version_parties").await,
        0,
        "without the references there are no parties — so parties below are attributable to them"
    );
    assert_eq!(
        scalar(&with_refs, "SELECT COUNT(*) FROM tender_version_parties").await,
        2,
        "the buyer and the review body both land (issue 98)"
    );

    // And nothing else moved — grouping, chain, and every fact satellite.
    //
    // Each digest is checked non-empty first. Comparing two NULLs is a gate that
    // passes because it measured nothing, which is the failure mode that let the
    // org class ship: `lot_results` is legitimately empty for a contract notice,
    // so it is named as the one permitted exception rather than silently allowed.
    for (label, sql) in LAYER_DIGESTS {
        let left = query_text(&with_refs, sql).await;
        let right = query_text(&without_refs, sql).await;
        if *label != "lot_results" {
            assert!(
                left.as_deref().is_some_and(|d| !d.is_empty()),
                "digest `{label}` is empty — it would compare equal without measuring anything"
            );
        }
        assert_eq!(
            left, right,
            "issue 98 must not move `{label}`: the fix may add parties and nothing else"
        );
    }

    let _ = std::fs::remove_file(&p1);
    let _ = std::fs::remove_file(&p2);
}

/// eForms-DE **2.x** is a real SDK fork emitting ordinary `BT-*` ids, so the
/// alias fold must leave it alone. Same notice shape, 2.0 profile: the `DE1-*`
/// ids stay unmapped and the version stays empty — proving the fold is what
/// produces the facts above, and that it is scoped to the 1.x line.
#[tokio::test]
async fn the_de1_alias_fold_is_scoped_to_the_1x_line() {
    let (db, fetch_id, path) = scratch("de1x-scope").await;
    let (notice, parse) = de1_notice(fetch_id, "de2-000001", "eforms:eforms-de-2.0");
    db.record_notice(&notice, &parse).await.expect("de-2.0 notice");

    project::project(&db, false).await.expect("project");
    assert_eq!(
        scalar(&db, "SELECT COUNT(*) FROM tender_version_texts").await,
        0,
        "eforms-de-2.x must not be folded through the 1.x alias table"
    );

    let _ = std::fs::remove_file(&path);
}

/// The DE-1.x folder id keys a Tender only when it is a genuine uuid (issue 34's
/// rule, applied to issue 85's alias). Two notices sharing a *portal-local*
/// reference must stay two island Tenders — ungated they would collapse into one,
/// and at cohort scale that is an unrecoverable wrong merge inside a run that is
/// already renumbering.
#[tokio::test]
async fn a_non_uuid_de1_folder_id_does_not_merge_notices() {
    let (db, fetch_id, path) = scratch("de1x-folder-gate").await;
    let (a, pa) = de1_notice_keyed(fetch_id, "de1-a", "eforms:eforms-de-1.1", "VG-2024-0815");
    let (b, pb) = de1_notice_keyed(fetch_id, "de1-b", "eforms:eforms-de-1.1", "VG-2024-0815");
    db.record_notice(&a, &pa).await.expect("a");
    db.record_notice(&b, &pb).await.expect("b");

    let report = project::project(&db, false).await.expect("project");
    assert_eq!(report.tenders, 2, "a shared portal-local reference must NOT merge two procedures");
    assert_eq!(report.islands, 2, "each stays an island until a real key appears");
    assert_eq!(
        scalar(&db, "SELECT COUNT(*) FROM tenders WHERE procedure_key IS NOT NULL").await,
        0,
        "a non-uuid folder id never becomes a procedure key"
    );
    // The gate costs no content: both are still full Tenders, just unmerged.
    assert_eq!(
        scalar(
            &db,
            "SELECT COUNT(*) FROM tender_version_texts WHERE field = 'title' AND lot_id IS NULL"
        )
        .await,
        2,
        "each island still carries its own title"
    );

    let _ = std::fs::remove_file(&path);
}

/// The converse, and the reason the gate is free: a genuine uuid still merges, so
/// no legitimate DÖE↔TED twin is lost to the gate.
#[tokio::test]
async fn a_uuid_de1_folder_id_still_merges_the_procedure() {
    let (db, fetch_id, path) = scratch("de1x-folder-merge").await;
    let uuid = "3f2504e0-4f89-41d3-9a0c-0305e82c3301";
    let (a, pa) = de1_notice_keyed(fetch_id, "de1-c", "eforms:eforms-de-1.1", uuid);
    let (b, pb) = de1_notice_keyed(fetch_id, "de1-d", "eforms:eforms-de-1.1", uuid);
    db.record_notice(&a, &pa).await.expect("a");
    db.record_notice(&b, &pb).await.expect("b");

    let report = project::project(&db, false).await.expect("project");
    assert_eq!(report.tenders, 1, "a shared BT-04 uuid is the ADR-0003 merge");
    assert_eq!(
        query_text(&db, "SELECT procedure_key FROM tenders").await.as_deref(),
        Some(uuid)
    );

    let _ = std::fs::remove_file(&path);
}
