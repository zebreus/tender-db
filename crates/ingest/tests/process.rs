//! Processor integration tests against a synthetic package that mirrors the
//! real era ladder: eForms UBL, TED_EXPORT R2.0.9, unversioned legacy
//! TED_EXPORT, a text-era bundle nested in a per-language ZIP, the skipped
//! sibling variants, and one unrecognised file.

use ingest::process;
use std::io::Write;
use std::path::{Path, PathBuf};

// --- fixture payloads, trimmed from real files on the sample ladder ---------

const EFORMS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ContractNotice xmlns="urn:oasis:names:specification:ubl:schema:xsd:ContractNotice-2"
    xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2"
    xmlns:efbc="http://data.europa.eu/p27/eforms-ubl-extension-basic-components/1">
  <efbc:NoticePublicationID schemeName="ojs-notice-id">00001505-2024</efbc:NoticePublicationID>
  <cbc:CustomizationID>eforms-sdk-1.7</cbc:CustomizationID>
</ContractNotice>"#;

// Real packages vary the root prefix freely; dispatch must key off the
// namespace URI, not the prefix.
const EFORMS_PREFIXED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<urn:ContractAwardNotice xmlns:urn="urn:oasis:names:specification:ubl:schema:xsd:ContractAwardNotice-2"
    xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2"
    xmlns:efbc="http://data.europa.eu/p27/eforms-ubl-extension-basic-components/1">
  <efbc:NoticePublicationID>00002000-2024</efbc:NoticePublicationID>
  <cbc:CustomizationID>eforms-sdk-1.13</cbc:CustomizationID>
</urn:ContractAwardNotice>"#;

// BusinessRegistrationInformationNotice is rooted in the eForms `p27` family
// rather than UBL — it appears once or twice in a real daily and must dispatch
// as eForms, not quarantine.
const EFORMS_BRIN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<BusinessRegistrationInformationNotice
    xmlns="http://data.europa.eu/p27/eforms-business-registration-information-notice/1"
    xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2"
    xmlns:efbc="http://data.europa.eu/p27/eforms-ubl-extension-basic-components/1">
  <efbc:NoticePublicationID>00430133-2025</efbc:NoticePublicationID>
  <cbc:CustomizationID>eforms-sdk-1.13</cbc:CustomizationID>
</BusinessRegistrationInformationNotice>"#;

const TED_R209: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<TED_EXPORT xmlns="http://publications.europa.eu/resource/schema/ted/R2.0.9/publication"
    VERSION="R2.0.9.S03.E01" DOC_ID="000001-2019" EDITION="2019001">
  <CODED_DATA_SECTION><NOTICE_DATA><NO_DOC_OJS>2019/S 1-000001</NO_DOC_OJS></NOTICE_DATA></CODED_DATA_SECTION>
</TED_EXPORT>"#;

// Early 2011: no VERSION attribute, no version in the namespace or
// schemaLocation — 177 of 1817 files in the real 2011-001 daily look like this.
const TED_UNVERSIONED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<TED_EXPORT xmlns="http://publications.europa.eu/TED_schema/Export"
    xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
    xsi:schemaLocation="http://publications.europa.eu/TED_schema/Export TED_EXPORT.xsd"
    DOC_ID="000036-2011" EDITION="2011001">
  <CODED_DATA_SECTION><NOTICE_DATA><NO_DOC_OJS>2011/S 1-000036</NO_DOC_OJS></NOTICE_DATA></CODED_DATA_SECTION>
</TED_EXPORT>"#;

const UNKNOWN_ROOT: &str = r#"<?xml version="1.0"?><SomethingElse xmlns="http://example.invalid/v1"/>"#;

const TEXT_DOC: &str = "  **********************************************\n  \
    ***  T E D   D A I L Y - D E L I V E R Y   ***\n  \
    **********************************************\n\n\
    1.00/067192\n\
    TI: F-Paris: lighting supports\n\
    PD: 19930102\n\
    ND: 52472-1992\n\
    OJ: 1/1993\n\
    TX:  1.  Awarding authority: Mairie de Paris.\n\n\
    1.00/067191\n\
    TI: I-Naples: meals\n\
    PD: 19930102\n\
    ND: 54411-1992\n\
    OJ: 1/1993\n\
    TX:  1.  Awarding authority: Unita sanitaria locale n. 43.\n";

// --- fixture construction ---------------------------------------------------

fn zip_of(name: &str, body: &str) -> Vec<u8> {
    let mut w = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    w.start_file(name, opts).unwrap();
    w.write_all(body.as_bytes()).unwrap();
    w.finish().unwrap().into_inner()
}

/// A .tar.gz shaped like a TED daily, spanning every era.
fn write_package(path: &Path) {
    let gz = flate2::write::GzEncoder::new(
        std::fs::File::create(path).unwrap(),
        flate2::Compression::fast(),
    );
    let mut tar = tar::Builder::new(gz);

    let mut add = |name: &str, bytes: &[u8]| {
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append_data(&mut header, name, bytes).unwrap();
    };

    add("20240102_1/00001505_2024.xml", EFORMS.as_bytes());
    add("20240102_1/00002000_2024.xml", EFORMS_PREFIXED.as_bytes());
    add("20240102_1/00430133_2025.xml", EFORMS_BRIN.as_bytes());
    add("20240102_1/00000001_2019.xml", TED_R209.as_bytes());
    add("20240102_1/00000036_2011.xml", TED_UNVERSIONED.as_bytes());
    add("20240102_1/garbage.xml", UNKNOWN_ROOT.as_bytes());
    // Text era: the English tagged-text bundle is ingested; the other-language
    // and `_meta_` sibling variants are skipped by documented policy.
    add(
        "EN_19930102_1993001_ISO_ORG.zip",
        &zip_of("EN_19930102_1993001_ISO_ORG", TEXT_DOC),
    );
    add("DE_19930102_1993001_ISO_ORG.zip", &zip_of("DE_19930102_1993001_ISO_ORG", TEXT_DOC));
    add("en_20100102_001_meta_org.zip", &zip_of("EN_20100102_2010001_META_ORG", "<part id=\"x\"/>"));

    tar.into_inner().unwrap().finish().unwrap();
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tender-db-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Archive + db with the package registered, exactly as the fetcher leaves it.
async fn fixture(name: &str) -> (PathBuf, store::Db) {
    let archive = temp_dir(name);
    std::fs::create_dir_all(archive.join("ted/daily")).unwrap();
    write_package(&archive.join("ted/daily/2026-00137.tar.gz"));

    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    db.record_fetch(&store::Fetch {
        source: "ted".into(),
        kind: "daily".into(),
        period: "2026-00137".into(),
        url: "https://ted.europa.eu/packages/daily/202600137".into(),
        sha256: "aa".into(),
        bytes: 1,
        fetched_at: 1,
        path: "ted/daily/2026-00137.tar.gz".into(),
    })
    .await
    .unwrap();
    (archive, db)
}

async fn run(db: &store::Db, archive: &Path) -> process::Report {
    process::process(db, archive, "ted", "daily", None, |_, _| {}).await.unwrap()
}

// --- tests ------------------------------------------------------------------

#[tokio::test]
async fn dispatches_every_era_and_accounts_for_every_file() {
    let (archive, db) = fixture("process").await;
    let r = run(&db, &archive).await;

    // 6 XML files + 3 members unwrapped from nested ZIPs.
    assert_eq!(r.members, 9);
    // Non-English and `_meta_` text-era variants.
    assert_eq!(r.skipped, 2);
    // 3 eForms + 2 TED_EXPORT + 1 garbage + 1 English text bundle.
    assert_eq!(r.ingested, 7);
    // 5 XML notices + 2 records inside the text bundle.
    assert_eq!(r.notices, 7);
    assert_eq!(r.quarantined, 1);
    assert_eq!(r.duplicates, 0);

    // Field mapping runs per profile. The three eForms payloads are
    // deliberately truncated stubs — they carry no UBLExtensions block, so the
    // eForms parser rejects them, which is exactly the ADR-0004 behaviour under
    // test: identity is recorded, nothing is imported, the reason is kept. The
    // R2.0.9 stub and the unversioned 2011 (r208-profile) stub are tiny but
    // fully-claimable TED_EXPORTs and parse, as do the two text-era records.
    assert_eq!(r.parsed, 4);
    assert_eq!(r.parse_quarantined, 3);

    // No silent drops: every member is accounted for by exactly one outcome.
    assert_eq!(r.members, r.ingested + r.skipped);

    let profiles = db.notice_counts_by_profile().await.unwrap();
    assert_eq!(
        profiles,
        vec![
            ("eforms:eforms-sdk-1.13".to_string(), 2),
            ("eforms:eforms-sdk-1.7".to_string(), 1),
            ("ted-export-r208".to_string(), 1),
            ("ted-export-r209".to_string(), 1),
            ("text".to_string(), 2),
        ]
    );

    let reasons = db.quarantine_counts_by_reason().await.unwrap();
    assert_eq!(
        reasons,
        vec![
            // The three truncated stubs (two 1.13, one 1.7 — all now vendored, so
            // each is parsed against its SDK and rejected for its missing body).
            ("unclaimed-content".to_string(), 3),
            // Not a notice at all — rejected before a profile was chosen.
            ("unknown-root".to_string(), 1),
        ]
    );

    let _ = std::fs::remove_dir_all(&archive);
}

/// TED monthly packages are plain tars whose members are the month's daily
/// `.tar.gz` files. The walker descends them in-stream; a monthly processed
/// after its standalone daily yields nothing but duplicates (identity dedup),
/// which is what makes the monthly-first backfill safe to mix with dailies.
#[tokio::test]
async fn monthly_of_nested_dailies_dedupes_against_the_daily() {
    let (archive, db) = fixture("process-monthly").await;
    let daily = run(&db, &archive).await;

    // A monthly: plain (uncompressed) tar carrying the very same daily.
    std::fs::create_dir_all(archive.join("ted/monthly")).unwrap();
    let monthly_path = archive.join("ted/monthly/2026-06.tar");
    let daily_bytes = std::fs::read(archive.join("ted/daily/2026-00137.tar.gz")).unwrap();
    let mut tar = tar::Builder::new(std::fs::File::create(&monthly_path).unwrap());
    let mut header = tar::Header::new_gnu();
    header.set_size(daily_bytes.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    tar.append_data(&mut header, "20260601_2026137.tar.gz", &daily_bytes[..]).unwrap();
    tar.into_inner().unwrap();

    db.record_fetch(&store::Fetch {
        source: "ted".into(),
        kind: "monthly".into(),
        period: "2026-06".into(),
        url: "https://ted.europa.eu/packages/monthly/2026-06".into(),
        sha256: "bb".into(),
        bytes: 1,
        fetched_at: 2,
        path: "ted/monthly/2026-06.tar".into(),
    })
    .await
    .unwrap();

    let monthly = process::process(&db, &archive, "ted", "monthly", None, |_, _| {}).await.unwrap();

    // The nested daily's members surface as the monthly's own, under
    // `<daily>.tar.gz/<file>` paths — same counts as the standalone daily.
    assert_eq!(monthly.members, daily.members);
    assert_eq!(monthly.skipped, daily.skipped);
    // Every notice identity is already known: nothing new, all duplicates.
    assert_eq!(monthly.notices, 0);
    assert_eq!(monthly.duplicates, daily.notices);

    let _ = std::fs::remove_dir_all(&archive);
}

#[tokio::test]
async fn reprocessing_is_idempotent() {
    let (archive, db) = fixture("process-idem").await;
    let first = run(&db, &archive).await;
    let second = run(&db, &archive).await;

    // Same work seen, but nothing new written: every notice is a known identity.
    assert_eq!(second.members, first.members);
    assert_eq!(second.notices, 0);
    assert_eq!(second.duplicates, first.notices);

    // The quarantine table does not grow either.
    let total: i64 = db.quarantine_counts_by_reason().await.unwrap().iter().map(|(_, n)| n).sum();
    assert_eq!(total, 4);

    let profiles = db.notice_counts_by_profile().await.unwrap();
    assert_eq!(profiles.iter().map(|(_, n)| n).sum::<i64>(), first.notices as i64);

    let _ = std::fs::remove_dir_all(&archive);
}

/// A package the walker cannot read *at all* (here the real upstream-truncated
/// 1996 bundle, standing in as a whole unreadable package) must not abort a
/// whole-source run: it is recorded as a `corrupt-package` quarantine and the
/// other packages still process. The job never dies on one bad file (ADR-0004).
#[tokio::test]
async fn a_corrupt_package_is_quarantined_and_the_run_continues() {
    let (archive, db) = fixture("corrupt-package").await;

    // A second daily whose bytes are not a readable archive.
    let corrupt = include_bytes!("fixtures/corrupt/SV_19960208_1996027_ISO_ORG.zip");
    std::fs::write(archive.join("ted/daily/2026-00138.tar.gz"), corrupt).unwrap();
    db.record_fetch(&store::Fetch {
        source: "ted".into(),
        kind: "daily".into(),
        period: "2026-00138".into(),
        url: "https://ted.europa.eu/packages/daily/202600138".into(),
        sha256: "bb".into(),
        bytes: corrupt.len() as i64,
        fetched_at: 2,
        path: "ted/daily/2026-00138.tar.gz".into(),
    })
    .await
    .unwrap();

    // The whole-source run completes despite the unreadable package.
    let report = process::process(&db, &archive, "ted", "daily", None, |_, _| {}).await.unwrap();

    // The good package's notices still landed.
    assert!(report.notices > 0, "the readable package still processed: {report:?}");

    // The unreadable package is recorded as a corrupt-package quarantine.
    let q = db.recent_quarantine(50).await.unwrap();
    assert!(
        q.iter().any(|e| e.profile.as_deref() == Some("corrupt-package")
            && e.member_path.contains("2026-00138")),
        "the unreadable package is quarantined for triage: {q:?}"
    );

    let _ = std::fs::remove_dir_all(&archive);
}

// --- reprocess / reclaim (issues 71/72/73) ----------------------------------

async fn count(db: &store::Db, table: &str, id: i64) -> i64 {
    match db.scalar(&format!("SELECT COUNT(*) FROM {table} WHERE notice_id = {id}")).await.unwrap() {
        Some(store::turso::Value::Integer(n)) => n,
        other => panic!("count({table}): {other:?}"),
    }
}
async fn cell_i64(db: &store::Db, sql: &str) -> Option<i64> {
    match db.scalar(sql).await.unwrap() {
        Some(store::turso::Value::Integer(n)) => Some(n),
        _ => None,
    }
}

/// Rewind a parsed notice to the pre-fix state a stale quarantine row records:
/// empty parsed layer, `quarantined`, projected, plus a held quarantine row —
/// exactly what OC/SDK members looked like before their fix shipped. Writes over
/// a raw connection to the same file (the store write API is intentionally
/// narrow); `id` is a trusted i64, inlined.
async fn make_held(db_path: &Path, id: i64) {
    let raw = store::turso::Builder::new_local(db_path.to_str().unwrap()).build().await.unwrap();
    let conn = raw.connect().unwrap();
    for table in [
        "notice_sections", "notice_texts", "notice_codes", "notice_classifications",
        "notice_amounts", "notice_dates", "notice_integers", "notice_numbers", "notice_ids",
    ] {
        conn.execute(&format!("DELETE FROM {table} WHERE notice_id = {id}"), ()).await.unwrap();
    }
    conn.execute(&format!("UPDATE notices SET parse_state = 'quarantined', projected = 1 WHERE id = {id}"), ())
        .await
        .unwrap();
    conn.execute(
        &format!(
            "INSERT INTO quarantine(notice_id, fetch_id, member_path, content_hash, profile, reason, detail, first_seen)
             SELECT id, fetch_id, member_path, content_hash, profile, 'unknown-field-code', 'line 1: OC', 0
               FROM notices WHERE id = {id}"
        ),
        (),
    )
    .await
    .unwrap();
}

/// The reprocess pass re-parses one held member in place from the archive, and
/// leaves every already-parsed notice untouched — the two properties the reclaim
/// program was missing (issues 71/72/73), end to end through the real walker.
#[tokio::test]
async fn reclaim_package_reclaims_a_held_member_and_never_touches_parsed_ones() {
    let (archive, db) = fixture("reclaim").await;
    run(&db, &archive).await;
    let pkg = archive.join("ted/daily/2026-00137.tar.gz");

    // Over a fully-parsed package the reprocess is a safe no-op: the 4 parsed
    // notices are AlreadyParsed, the 3 eForms stubs still fail (StillHeld), and
    // nothing is reclaimed or corrupted.
    let noop = process::reclaim_package(&db, &pkg, "ted", 1, |_, _, _| {}).await.unwrap();
    assert_eq!((noop.reclaimed, noop.already, noop.still_held), (0, 4, 3));

    // Rewind one genuinely-parsed notice to the held state, then reprocess.
    let id = cell_i64(&db, "SELECT MIN(id) FROM notices WHERE parse_state = 'parsed'").await.unwrap();
    let sections = count(&db, "notice_sections", id).await;
    assert!(sections > 0);
    make_held(&archive.join("test.db"), id).await;
    assert_eq!(count(&db, "notice_sections", id).await, 0);

    let report = process::reclaim_package(&db, &pkg, "ted", 1, |_, _, _| {}).await.unwrap();
    assert_eq!(report.reclaimed, 1, "exactly the rewound member is reclaimed");
    assert_eq!(report.already, 3, "the other parsed notices are left as-is");
    assert_eq!(report.still_held, 3, "the eForms stubs are still held");

    // Its parsed layer is restored, the watermark cleared, and the row flagged.
    assert_eq!(
        db.scalar(&format!("SELECT parse_state FROM notices WHERE id = {id}")).await.unwrap(),
        Some(store::turso::Value::Text("parsed".into()))
    );
    assert_eq!(count(&db, "notice_sections", id).await, sections, "the parsed layer is rebuilt");
    assert_eq!(cell_i64(&db, &format!("SELECT projected FROM notices WHERE id = {id}")).await, Some(0));
    assert!(cell_i64(&db, &format!("SELECT reprocessed_at FROM quarantine WHERE notice_id = {id}")).await.is_some());
    assert!(db.unprojected_parsed_notice_ids().await.unwrap().contains(&id));

    let _ = std::fs::remove_dir_all(&archive);
}
