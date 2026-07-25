//! Cross-commit golden for the Phase-2 fold apply path (task #3, prepared-statement
//! writes). The other projection tests — `project_fold_source`, `project_resume`,
//! `project_equivalence` — compare two runs that SHARE `apply_tenders`, so a change
//! that alters `apply_tenders`' output UNIFORMLY (a reorder, a subtle bind change)
//! moves both arms together and those tests stay green while the derived layer
//! silently diverges from what it produced before. This test is the guard against
//! that: it pins the canonical layer of a fixed rich corpus to a byte-for-byte
//! golden captured BEFORE the prepared-statement conversion.
//!
//! The golden file `fixtures/golden/project_apply.snapshot` was captured on the
//! commit immediately before the fold apply path switched from `conn.execute(fresh
//! SQL)` to reused prepared statements. It MUST NOT be regenerated to make a change
//! pass — a diff here means the derived layer moved, which is exactly the
//! ADR-0001 byte-identical violation this test exists to catch. Regenerate it only
//! for a DELIBERATE, reviewed change to the derived layer's content.
//!
//! The corpus is the real Maltese CN → corrigendum → corrigendum → CAN chain (one
//! keyed Tender, four versions, with lots, lot results, contracts, parties, amounts,
//! dates, classifications and the resulting change log) plus two island fixtures —
//! every AUTOINCREMENT surrogate id (tenders, lots, bids, contracts, lot_results)
//! and the `changes.entity_id` values that depend on their INSERT order are in the
//! digest.

use ingest::project::Phase2;
use ingest::{eforms, profile, project};
use store::{Db, Notice, Parse};

const SOURCE: &str = "ted";

async fn scratch() -> (Db, i64, String) {
    let path = format!("/tmp/tender-db-projgolden-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
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

/// Ingest a fixture through the real dispatch + parse chain, exactly as `process`
/// would from an archived package.
async fn ingest(db: &Db, fetch_id: i64, relative: &str) {
    let bytes = std::fs::read(format!("tests/fixtures/{relative}")).expect("fixture");
    let profile::Disposition::Records(records) = profile::dispatch(relative, &bytes) else {
        panic!("{relative}: dispatch skipped a fixture");
    };
    let [profile::Record::Notice(n)] = &records[..] else {
        panic!("{relative}: expected one notice record");
    };
    let parse = eforms::parse_payload(&n.profile, &bytes);
    let (published_at, dispatched_at) = match &parse {
        Parse::Parsed(parsed) => {
            let (p, d) = project::notice_instants(parsed);
            (Some(p), d)
        }
        _ => panic!("{relative}: not parsed"),
    };
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
            published_at,
            dispatched_at,
        },
        &parse,
    )
    .await
    .expect("record notice");
}

/// Every content table the fold apply path writes, plus the change log — keyed and
/// ordered so it is stable across runs. Surrogate ids ARE compared (a fresh rebuild
/// restarts them at 1 in fold order), so a reordered INSERT under the prepared-
/// statement conversion would shift an id and break the digest.
async fn snapshot(db: &Db) -> String {
    let digests = [
        ("tenders", "SELECT group_concat(r, x'0a') FROM (SELECT id||'|'||coalesce(procedure_key,'')||'|'||coalesce(island_notice_id,-1)||'|'||kind||'|'||source||'|'||coalesce(current_seq,-1) AS r FROM tenders ORDER BY id)"),
        ("tender_versions", "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||caused_by_notice_id||'|'||coalesce(publication_id,'')||'|'||coalesce(notice_subtype,'')||'|'||published_at||'|'||coalesce(dispatched_at,-1) AS r FROM tender_versions ORDER BY tender_id, seq)"),
        ("tender_version_lots", "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||lot_id||'|'||kind AS r FROM tender_version_lots ORDER BY tender_id, seq, lot_id)"),
        ("tender_version_texts", "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||field||'|'||coalesce(lang,'')||'|'||value||'|'||coalesce(lot_id,-1) AS r FROM tender_version_texts ORDER BY tender_id, seq, field, lang, value, lot_id)"),
        ("tender_version_dates", "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||field||'|'||utc_seconds||'|'||offset_minutes||'|'||has_time||'|'||coalesce(lot_id,-1) AS r FROM tender_version_dates ORDER BY tender_id, seq, field, utc_seconds, lot_id)"),
        ("tender_version_classifications", "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||field||'|'||scheme||'|'||code||'|'||coalesce(lot_id,-1) AS r FROM tender_version_classifications ORDER BY tender_id, seq, field, scheme, code, lot_id)"),
        ("tender_version_amounts", "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||field||'|'||cents||'|'||currency||'|'||coalesce(lot_id,-1) AS r FROM tender_version_amounts ORDER BY tender_id, seq, field, cents, lot_id)"),
        ("tender_version_parties", "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||role||'|'||organization_id||'|'||mention_notice_id||'|'||mention_section_id||'|'||coalesce(lot_id,-1) AS r FROM tender_version_parties ORDER BY tender_id, seq, role, organization_id, mention_section_id, lot_id)"),
        ("lots", "SELECT group_concat(r, x'0a') FROM (SELECT id||'|'||tender_id||'|'||lot_key AS r FROM lots ORDER BY id)"),
        ("bids", "SELECT group_concat(r, x'0a') FROM (SELECT id||'|'||tender_id||'|'||notice_id||'|'||bid_key AS r FROM bids ORDER BY id)"),
        ("contracts", "SELECT group_concat(r, x'0a') FROM (SELECT id||'|'||tender_id||'|'||notice_id||'|'||contract_key AS r FROM contracts ORDER BY id)"),
        ("lot_results", "SELECT group_concat(r, x'0a') FROM (SELECT id||'|'||tender_id||'|'||notice_id||'|'||result_key AS r FROM lot_results ORDER BY id)"),
        ("tender_version_lot_results", "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||lot_result_id||'|'||coalesce(lot_id,-1)||'|'||coalesce(decision,'')||'|'||coalesce(reason,'')||'|'||coalesce(awarded_cents,-1)||'|'||coalesce(awarded_currency,'') AS r FROM tender_version_lot_results ORDER BY tender_id, seq, lot_result_id)"),
        ("tender_version_result_winners", "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||lot_result_id||'|'||organization_id AS r FROM tender_version_result_winners ORDER BY tender_id, seq, lot_result_id, organization_id)"),
        ("tender_version_result_stats", "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||lot_result_id||'|'||kind||'|'||count AS r FROM tender_version_result_stats ORDER BY tender_id, seq, lot_result_id, kind)"),
        ("tender_version_bids", "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||bid_id||'|'||coalesce(lot_id,-1)||'|'||coalesce(cents,-1)||'|'||coalesce(currency,'') AS r FROM tender_version_bids ORDER BY tender_id, seq, bid_id)"),
        ("tender_version_bid_parties", "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||bid_id||'|'||role||'|'||organization_id||'|'||mention_notice_id||'|'||mention_section_id AS r FROM tender_version_bid_parties ORDER BY tender_id, seq, bid_id, role, organization_id, mention_section_id)"),
        ("tender_version_contracts", "SELECT group_concat(r, x'0a') FROM (SELECT tender_id||'|'||seq||'|'||contract_id||'|'||coalesce(buyer_contract_id,'')||'|'||coalesce(concluded_utc,-1)||'|'||coalesce(concluded_offset,-1)||'|'||coalesce(concluded_has_time,-1)||'|'||coalesce(cents,-1)||'|'||coalesce(currency,'') AS r FROM tender_version_contracts ORDER BY tender_id, seq, contract_id)"),
        ("changes", "SELECT group_concat(r, x'0a') FROM (SELECT entity_kind||'|'||op||'|'||coalesce(version_seq,-1)||'|'||entity_id AS r FROM changes ORDER BY cursor)"),
    ];
    let mut out = String::new();
    for (name, sql) in digests {
        let part = match db.scalar(sql).await.expect("digest query") {
            Some(turso::Value::Text(s)) => s,
            _ => String::new(),
        };
        out.push_str(&format!("--- {name} ---\n{part}\n"));
    }
    out
}

/// The canonical layer of a fixed rich corpus is byte-for-byte what the fold apply
/// path produced before the prepared-statement conversion. See the module header:
/// the golden file is a cross-commit anchor and must not be regenerated to pass.
///
/// Run on an explicit large-stack thread: turso's debug-build query execution
/// (the wide `group_concat` digests below in particular) is stack-hungry enough to
/// overflow libtest's default worker stack, so the test carries its own runtime
/// rather than depend on a `RUST_MIN_STACK` in the environment.
#[test]
fn fold_apply_output_matches_the_committed_golden() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime")
                .block_on(run())
        })
        .expect("spawn")
        .join()
        .expect("join");
}

async fn run() {
    let (db, fetch_id, path) = scratch().await;
    for fixture in [
        "eforms-chain/1-cn-16-831374-2025.xml",
        "eforms-chain/2-change-16-6281-2026.xml",
        "eforms-chain/3-change-16-18902-2026.xml",
        "eforms-chain/4-can-29-380868-2026.xml",
        "eforms/brin-x01-00497689-2026.xml",
        "eforms/pin-4-00496860-2026.xml",
    ] {
        ingest(&db, fetch_id, fixture).await;
    }

    project::project_with_progress_phase2(&db, true, 7, Phase2::Buckets, |_| {})
        .await
        .expect("projection");

    let got = snapshot(&db).await;
    // Opt-in regeneration for a DELIBERATE, reviewed derived-layer change only —
    // never to make a failing run pass (see the module header).
    if std::env::var_os("GOLDEN_CAPTURE").is_some() {
        std::fs::write(
            concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/golden/project_apply.snapshot"),
            &got,
        )
        .expect("write golden");
    }
    let golden = include_str!("fixtures/golden/project_apply.snapshot");
    assert_eq!(
        got, golden,
        "the fold apply path's canonical layer diverged from the committed golden \
         (fixtures/golden/project_apply.snapshot) — an ADR-0001 byte-identical \
         violation. Do NOT regenerate the golden to make this pass."
    );

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}
