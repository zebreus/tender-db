//! UK Find a Tender end to end: the zip the fetcher assembles is walked by the
//! processor, every member dispatches through the `fts:ocds-1.1` profile, and
//! the notices land as identity-only rows (issue 342 commit (b)).
//!
//! The members are real releases of 3 September 2026, cut from the recorded
//! pages in `.scratch/tender-db/342-fts/recorded/` by `fts::member_bytes` —
//! byte-identical to what `fetch::assemble_fts_zip` writes into the package.
//!
//! Contains public sector information licensed under the Open Government
//! Licence v3.0.

use ingest::{process, project};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Member name → payload, as the fetcher names them: `<release id>.json`, and
/// the reserved `_noid/` prefix for a release the publisher sent without an id.
const MEMBERS: [(&str, &[u8]); 5] = [
    // UK4 tender, 5 lots with their own items and CPVs.
    ("083563-2026.json", include_bytes!("fixtures/fts/members/083563-2026.json")),
    // UK2 planning notice.
    ("083645-2026.json", include_bytes!("fixtures/fts/members/083645-2026.json")),
    // UK6 award+contract: one supplier, an award value (net and gross).
    ("083650-2026.json", include_bytes!("fixtures/fts/members/083650-2026.json")),
    // UK15 award+contract carrying amendments — the delta shape (plan D4).
    ("083685-2026.json", include_bytes!("fixtures/fts/members/083685-2026.json")),
    // A real release with its `id` removed: archived, then quarantined here.
    (
        "_noid/2026-09-03-p001-000.json",
        include_bytes!("fixtures/fts/members/_noid-2026-09-03-p001-000.json"),
    ),
];

/// The four members that carry a usable release id — the ones that must become
/// notices. The fifth is a publisher defect, quarantined by construction.
const NOTICE_IDS: [&str; 4] = ["083563-2026", "083645-2026", "083650-2026", "083685-2026"];

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tender-db-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// An archive holding one assembled FTS daily, registered exactly as
/// `fetch_fts` leaves it.
async fn fixture(name: &str) -> (PathBuf, store::Db) {
    fixture_of(name, &MEMBERS).await
}

/// The same archive, assembled from the members given.
async fn fixture_of(name: &str, members: &[(&str, &[u8])]) -> (PathBuf, store::Db) {
    let archive = temp_dir(name);
    std::fs::create_dir_all(archive.join("fts/daily")).unwrap();
    let path = archive.join("fts/daily/2026-09-03.zip");
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .last_modified_time(zip::DateTime::default());
    for (member, bytes) in members {
        zip.start_file(*member, opts).unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap().flush().unwrap();

    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    db.record_fetch(&store::Fetch {
        source: "fts".into(),
        kind: "daily".into(),
        period: "2026-09-03".into(),
        url: "https://www.find-tender.service.gov.uk/api/1.0/ocdsReleasePackages?limit=100\
              &updatedFrom=2026-09-02T22:00:00&updatedTo=2026-09-03T23:59:59"
            .into(),
        sha256: ingest::sha256_hex(&std::fs::read(&path).unwrap()),
        bytes: std::fs::metadata(&path).unwrap().len() as i64,
        fetched_at: 1,
        path: "fts/daily/2026-09-03.zip".into(),
    })
    .await
    .unwrap();
    (archive, db)
}

async fn run(db: &store::Db, archive: &Path) -> process::Report {
    process::process(db, archive, "fts", "daily", None, |_, _| {}, || false).await.unwrap()
}

async fn cell_i64(db: &store::Db, sql: &str) -> i64 {
    match db.scalar(sql).await.unwrap() {
        Some(store::turso::Value::Integer(n)) => n,
        other => panic!("{sql}: {other:?}"),
    }
}

async fn cell_text(db: &store::Db, sql: &str) -> Option<String> {
    match db.scalar(sql).await.unwrap() {
        Some(store::turso::Value::Text(s)) => Some(s),
        _ => None,
    }
}

/// The live release that a document parser could not hold (`1e9999`, 3 September
/// 2026) goes all the way to a notice row. Both layers read named strings and
/// leave publisher values raw, so the value nobody can represent is a value
/// nobody looked at — it is archived, dispatched and stored, byte for byte.
#[tokio::test]
async fn a_release_carrying_an_unrepresentable_number_still_becomes_a_notice() {
    let served = br#"{"version":"1.1","license":"OGL","releases":[
        {"id":"083529-2026","ocid":"ocds-h6vhtk-05f2a1",
         "tender":{"title":"Framework","lotDetails":{"maximumLotsBidPerSupplier":1e9999}}}]}"#;
    let page = ingest::fts::Page::read(served).unwrap();
    let member = page.member_bytes(page.releases().unwrap()[0]);

    let archive = temp_dir("fts-bignum");
    std::fs::create_dir_all(archive.join("fts/daily")).unwrap();
    let path = archive.join("fts/daily/2026-09-03.zip");
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
    zip.start_file("083529-2026.json", zip::write::SimpleFileOptions::default()).unwrap();
    zip.write_all(&member).unwrap();
    zip.finish().unwrap().flush().unwrap();

    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    db.record_fetch(&store::Fetch {
        source: "fts".into(),
        kind: "daily".into(),
        period: "2026-09-03".into(),
        url: "https://x".into(),
        sha256: ingest::sha256_hex(&std::fs::read(&path).unwrap()),
        bytes: std::fs::metadata(&path).unwrap().len() as i64,
        fetched_at: 1,
        path: "fts/daily/2026-09-03.zip".into(),
    })
    .await
    .unwrap();

    let r = run(&db, &archive).await;
    assert_eq!((r.notices, r.quarantined), (1, 0), "a notice, not a quarantine row");
    assert_eq!(
        cell_text(&db, "SELECT publication_id FROM notices").await.as_deref(),
        Some("083529-2026")
    );
    let _ = std::fs::remove_dir_all(&archive);
}

/// The whole chain the daily push re-enables: an assembled zip in, one notice
/// row per release out, no `unparsable-xml` anywhere. Identity only — the
/// field parser is commit (c), so every row is `pending` (never quarantined:
/// an unparsed profile is a gap, not a defect, ADR-0004).
#[tokio::test]
async fn fts_zip_package_processes_end_to_end() {
    let (archive, db) = fixture("fts").await;
    let r = run(&db, &archive).await;

    assert_eq!(r.members, MEMBERS.len() as u64);
    // No silent drops: every member is accounted for by exactly one outcome,
    // and nothing in an FTS package is skipped by policy.
    assert_eq!(r.members, r.ingested + r.skipped);
    assert_eq!(r.skipped, 0);
    assert_eq!(r.ingested, MEMBERS.len() as u64);
    // Every release with an id is a notice; the `_noid/` member is the only
    // quarantine, and it is the publisher's defect, not a dispatch failure.
    assert_eq!(r.notices, NOTICE_IDS.len() as u64);
    assert_eq!(r.quarantined, 1);
    assert_eq!(r.duplicates, 0);
    // Commit (c) moved this rung: every notice the dispatcher accepts now
    // PARSES, and none of them trips a value the mapping cannot represent.
    // Before (c) this asserted `parsed == 0` — the identity-only rung — and
    // that expectation is what the parser was supposed to retire.
    assert_eq!(r.parsed, NOTICE_IDS.len() as u64);
    assert_eq!(r.parse_quarantined, 0);

    // One profile across the package, taken from the OCDS package version.
    assert_eq!(
        db.notice_counts_by_profile().await.unwrap(),
        vec![("fts:ocds-1.1".to_string(), NOTICE_IDS.len() as i64)]
    );
    assert_eq!(
        db.quarantine_counts_by_reason().await.unwrap(),
        vec![("missing-publication-id".to_string(), 1)]
    );

    // Identity is the release id verbatim, and the member path is the reclaim
    // key — both must survive the walk unchanged.
    for id in NOTICE_IDS {
        assert_eq!(
            cell_text(
                &db,
                &format!(
                    "SELECT member_path FROM notices WHERE source = 'fts' AND publication_id = '{id}'"
                ),
            )
            .await,
            Some(format!("{id}.json")),
            "{id} did not land under its own member"
        );
    }
    assert_eq!(
        cell_i64(&db, "SELECT COUNT(*) FROM notices WHERE parse_state = 'pending'").await,
        0,
        "commit (c) gave the fts profile a parser, so no row stays pending"
    );
    assert_eq!(
        cell_i64(&db, "SELECT COUNT(*) FROM notices WHERE parse_state = 'parsed'").await,
        NOTICE_IDS.len() as i64,
        "every accepted release parses"
    );
    assert_eq!(
        cell_text(&db, "SELECT DISTINCT declared_version FROM notices WHERE source = 'fts'").await,
        Some("1.1".to_owned())
    );
    // The defective member is kept whole, under the prefix the fetcher gave it.
    assert_eq!(
        cell_text(&db, "SELECT member_path FROM quarantine").await,
        Some("_noid/2026-09-03-p001-000.json".to_owned())
    );

    // Idempotent: the 2 h window overlap re-serves yesterday's tail, and a
    // monthly re-walks days the daily already landed, so a second pass over the
    // same bytes must add nothing (plan D3).
    let again = run(&db, &archive).await;
    assert_eq!(again.notices, 0);
    assert_eq!(again.duplicates, NOTICE_IDS.len() as u64);
    assert_eq!(
        cell_i64(&db, "SELECT COUNT(*) FROM notices").await,
        NOTICE_IDS.len() as i64,
        "re-processing the same package must not duplicate a release"
    );

    let _ = std::fs::remove_dir_all(&archive);
}

/// One synthetic FTS release: a UK6 award under `ocid`, published by `buyer`
/// (a GB-PPON party id, the form the crosswalk's GB arm keys on).
fn release(ocid: &str, id: &str, day: u32, buyer: &str, name: &str, title: &str) -> Vec<u8> {
    let pkg = serde_json::json!({
        "version": "1.1",
        "publisher": {"name": "Find a Tender (test)"},
        "releases": [{
            "ocid": ocid,
            "id": id,
            "date": format!("2026-09-{day:02}T10:00:00+01:00"),
            "tag": ["award"],
            "language": "en",
            "initiationType": "tender",
            "buyer": {"id": format!("GB-PPON-{buyer}"), "name": name},
            "parties": [{
                "id": format!("GB-PPON-{buyer}"),
                "name": name,
                "identifier": {"scheme": "GB-PPON", "id": buyer},
                "address": {"country": "GB"},
                "roles": ["buyer"]
            }],
            "tender": {
                "id": format!("{id}-t"),
                "title": title,
                "legalBasis": {"scheme": "CELEX", "id": "32014L0025"},
                "documents": [{"id": format!("{id}-d"), "documentType": "awardNotice", "noticeType": "UK6"}]
            }
        }]
    });
    serde_json::to_vec(&pkg).unwrap()
}

/// Issue 386 unit 1: a reused ocid is not always one procurement. Find a Tender's
/// utilities qualification systems publish every participating utility's awards
/// under the REGISTER's ocid, and keying on the ocid alone folded seven of
/// tender 7954584's eighteen contracts under a buyer that never awarded them.
/// Both directions are pinned here through the real walk — ingest, parse, plan,
/// fold: two buyers' releases under one ocid become TWO Tenders, each carrying
/// only its own buyer, while a single-buyer chain under one ocid stays ONE Tender
/// with every release as a version.
#[tokio::test]
async fn releases_under_one_ocid_from_two_buyers_fold_to_two_tenders_and_a_one_buyer_chain_to_one() {
    let a1 = release("ocds-t-register", "900001-2026", 1, "ANGL-0001-AAAA", "Anglian Test Water", "Supply of Pipe & Fittings");
    let b1 = release("ocds-t-register", "900002-2026", 2, "SHET-0002-BBBB", "Scottish Hydro Test", "Super Grid Transformers");
    let a2 = release("ocds-t-register", "900003-2026", 3, "ANGL-0001-AAAA", "Anglian Test Water", "Supply of Pipe & Fittings (2)");
    let c1 = release("ocds-t-chain", "900011-2026", 1, "TAXI-0003-CCCC", "Taxi Test Council", "Taxi Vehicles");
    let c2 = release("ocds-t-chain", "900012-2026", 2, "TAXI-0003-CCCC", "Taxi Test Council", "Taxi Vehicles");
    let c3 = release("ocds-t-chain", "900013-2026", 3, "TAXI-0003-CCCC", "Taxi Test Council", "Taxi Vehicles");
    let members: [(&str, &[u8]); 6] = [
        ("900001-2026.json", &a1),
        ("900002-2026.json", &b1),
        ("900003-2026.json", &a2),
        ("900011-2026.json", &c1),
        ("900012-2026.json", &c2),
        ("900013-2026.json", &c3),
    ];
    let (archive, db) = fixture_of("fts-386", &members).await;
    let r = run(&db, &archive).await;
    assert_eq!(r.parsed, 6, "every synthetic release parses: {r:?}");
    assert_eq!(r.parse_quarantined, 0);

    project::project(&db, false).await.expect("project");

    // The register: two buyers, two Tenders — and the split is visible in the served
    // key, which carries the ocid AND the buyer it was split on.
    assert_eq!(
        cell_i64(&db, "SELECT COUNT(*) FROM tenders WHERE source = 'fts' AND procedure_key LIKE 'refused:ocds-t-register:%'").await,
        2,
        "two utilities under one register ocid are two Tenders"
    );
    assert_eq!(
        cell_i64(&db, "SELECT COUNT(*) FROM tenders WHERE source = 'fts' AND procedure_key = 'ocds-t-register'").await,
        0,
        "nothing is keyed on the bare register ocid any more"
    );
    // Anglian's two releases are one Tender with two versions; Scottish Hydro's one is its own.
    assert_eq!(
        cell_i64(&db, "SELECT MAX(current_seq) FROM tenders WHERE procedure_key LIKE 'refused:ocds-t-register:%'").await,
        2,
        "one buyer's two releases are two versions of ONE Tender"
    );
    // Every Tender's buyer set is exactly one organization: no Tender carries another
    // utility's buyer, which is what 7954584 did.
    assert_eq!(
        cell_i64(
            &db,
            "SELECT COUNT(*) FROM (SELECT p.tender_id FROM tender_version_parties p JOIN tenders t ON t.id = p.tender_id \
              WHERE t.source = 'fts' AND p.role IN ('buyer', 'Procedure-Buyer') \
              GROUP BY p.tender_id HAVING COUNT(DISTINCT p.organization_id) > 1)",
        )
        .await,
        0,
        "no FTS Tender carries two buyers"
    );
    // The single-buyer chain keeps its ocid and all three releases as versions.
    assert_eq!(
        cell_i64(&db, "SELECT COUNT(*) FROM tenders WHERE source = 'fts' AND procedure_key = 'ocds-t-chain'").await,
        1,
        "a single-buyer chain is one Tender under its ocid"
    );
    assert_eq!(
        cell_i64(&db, "SELECT current_seq FROM tenders WHERE procedure_key = 'ocds-t-chain'").await,
        3,
        "every release of the chain is a version of it"
    );

    let _ = std::fs::remove_dir_all(&archive);
}

/// Issue 386 unit 2b: ADR-0004's mapped-or-ignored checklist for `fts:ocds-1.1`,
/// pinned against the corpus. Every path a fixture release publishes — the
/// recorded pages and the cut members, the shapes this crosswalk was written
/// from — must have a disposition in `fts::checklist`: MAPPED to a field id, or
/// IGNORED with its reason. A path with neither is the silent drop this
/// checklist exists to make loud: `serde` skips unknown keys without a trace,
/// and that is how every FTS contract served `value: null` against a published
/// figure until a consumer noticed.
#[test]
fn every_published_fts_path_is_mapped_or_ignored_on_record() {
    use ingest::fts::checklist::{disposition, Disposition};
    use std::collections::BTreeMap;

    fn walk(v: &serde_json::Value, prefix: &str, paths: &mut BTreeMap<String, usize>) {
        match v {
            serde_json::Value::Object(map) => {
                for (k, x) in map {
                    let p = if prefix.is_empty() { k.clone() } else { format!("{prefix}.{k}") };
                    *paths.entry(p.clone()).or_default() += 1;
                    walk(x, &p, paths);
                }
            }
            serde_json::Value::Array(items) => {
                let p = format!("{prefix}[]");
                for x in items {
                    walk(x, &p, paths);
                }
            }
            _ => {}
        }
    }

    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/fts");
    let mut paths: BTreeMap<String, usize> = BTreeMap::new();
    let mut releases = 0;
    for dir in ["pages", "members"] {
        for entry in std::fs::read_dir(format!("{root}/{dir}")).expect("fixture dir") {
            let path = entry.expect("entry").path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let bytes = std::fs::read(&path).expect("read fixture");
            // The unrepresentable-number member is not JSON a lenient parser accepts;
            // the walk needs values, so a member serde refuses is skipped here (its
            // paths are the same as its siblings').
            let Ok(doc) = serde_json::from_slice::<serde_json::Value>(&bytes) else { continue };
            for r in doc["releases"].as_array().into_iter().flatten() {
                releases += 1;
                walk(r, "", &mut paths);
            }
        }
    }
    assert!(releases >= 5, "the fixtures hold releases: {releases}");
    assert!(paths.len() >= 150, "the census sees the publisher's shape: {} paths", paths.len());

    let undecided: Vec<&String> = paths.keys().filter(|p| disposition(p).is_none()).collect();
    assert!(
        undecided.is_empty(),
        "these published paths have no disposition in fts::checklist — map them or ignore them on record:\n  {}",
        undecided.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n  ")
    );
    // And the inventory is not vacuous: the value that issue 386 found dropped is on it, mapped.
    assert!(matches!(disposition("contracts[].value.amount"), Some(Disposition::Mapped(_))));
    let owed = paths.keys().filter(|p| matches!(disposition(p), Some(Disposition::Ignored(r)) if r.starts_with("owed"))).count();
    eprintln!("fts checklist: {} published paths over {releases} releases, {owed} ignored-as-owed", paths.len());
}
