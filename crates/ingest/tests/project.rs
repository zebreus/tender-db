//! Projection tests (issue 04), driven by the committed real-notice corpus.
//!
//! The headline case is `tests/fixtures/eforms-chain/`: one real Maltese
//! procedure published as CN → corrigendum → corrigendum → CAN over six months.
//! It must collapse into exactly one Tender with four versions, the corrigendum
//! must supersede the field it actually moved, and the change log must say so.

use ingest::{eforms, profile, project};
use store::{Db, Notice, NoticeValue, Parse, Parsed, Section, ValueRow};

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
    let path = format!("tests/fixtures/{relative}");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let profile::Disposition::Records(records) = profile::dispatch(relative, &bytes) else {
        panic!("{relative}: dispatch skipped a fixture");
    };
    let [profile::Record::Notice(n)] = &records[..] else {
        panic!("{relative}: expected one notice record");
    };
    let parse = eforms::parse_payload(&n.profile, &bytes);
    assert!(matches!(parse, Parse::Parsed(_)), "{relative}: {parse:?}");
    db.record_notice(
        &Notice {
            source: SOURCE.into(),
            publication_id: n.publication_id.clone(),
            content_hash: n.content_hash.clone(),
            profile: n.profile.clone(),
            declared_version: n.declared_version.clone(),
            fetch_id,
            member_path: n.member_path.clone(),
            ingested_at: 0,
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

    let tender_ops: Vec<(i64, String)> = db
        .changes_since(0, 100)
        .await
        .expect("changes")
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
    let all = db.changes_since(0, 1000).await.expect("changes");
    assert!(all.windows(2).all(|w| w[0].cursor < w[1].cursor));
    assert!(all.iter().any(|c| c.entity_kind == "lot" && c.op == "added"));

    // Re-projecting an unchanged notice layer is a complete no-op.
    let before = all.len();
    let again = project::project(&db, false).await.expect("re-project");
    assert_eq!(again.applied.versions_written, 0);
    assert_eq!(again.applied.changes, 0);
    assert_eq!(db.changes_since(0, 1000).await.expect("changes").len(), before);

    // A rebuild reproduces the same canonical state and appends a fresh set of
    // change rows — the cursor is never renumbered.
    project::project(&db, true).await.expect("rebuild");
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM tender_versions").await, 4);
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM tenders").await, 1);
    let rebuilt = db.changes_since(0, 1000).await.expect("changes");
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
