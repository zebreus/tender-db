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
    assert_eq!(reasons, vec![("unknown-root".to_string(), 1)]);

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
    assert_eq!(total, 1);

    let profiles = db.notice_counts_by_profile().await.unwrap();
    assert_eq!(profiles.iter().map(|(_, n)| n).sum::<i64>(), first.notices as i64);

    let _ = std::fs::remove_dir_all(&archive);
}
