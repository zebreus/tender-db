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

use ingest::process;
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
    let archive = temp_dir(name);
    std::fs::create_dir_all(archive.join("fts/daily")).unwrap();
    let path = archive.join("fts/daily/2026-09-03.zip");
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .last_modified_time(zip::DateTime::default());
    for (member, bytes) in MEMBERS {
        zip.start_file(member, opts).unwrap();
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
    process::process(db, archive, "fts", "daily", None, |_, _| {}).await.unwrap()
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
    // Identity-only rung: nothing parsed, nothing parse-quarantined.
    assert_eq!(r.parsed, 0);
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
        NOTICE_IDS.len() as i64,
        "the fts profile has no parser until commit (c): every row stays pending"
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
