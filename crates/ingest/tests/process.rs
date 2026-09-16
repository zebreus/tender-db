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
    process::process(db, archive, "ted", "daily", None, |_, _| {}, || false).await.unwrap()
}

// --- tests ------------------------------------------------------------------

/// Issue 406: a `process` walk stops when asked, and the stop is SAFE because
/// re-running finishes the job.
///
/// The flag alone would be a weak test — a walk that stopped and lost the package
/// would pass it. What makes stopping safe is that each member is its own
/// transaction and the dedup path skips what is already held, so both halves are
/// asserted: what the stopped walk wrote is still there, and a second walk with no
/// stop reaches exactly the state an uninterrupted walk would have.
///
/// This lever is why the issue exists: a `process` degraded by issue 404's
/// unindexed lookup ran for six hours, could not be cancelled, and held the queue
/// against the deploy that fixed it.
#[tokio::test]
async fn a_stopped_walk_keeps_what_it_wrote_and_a_re_run_finishes_the_package() {
    let (archive, db) = fixture("process-stop").await;

    // Stop after the first member: `should_stop` is checked AFTER each member is
    // committed, so exactly one record must survive.
    let seen = std::cell::Cell::new(0u64);
    let stopped = process::process(&db, &archive, "ted", "daily", None, |_, _| {}, || {
        seen.set(seen.get() + 1);
        true
    })
    .await
    .unwrap();
    assert!(stopped.cancelled, "the walk must report that it stopped: {stopped:?}");
    let after_stop = cell_i64(&db, "SELECT COUNT(*) FROM notices").await.unwrap_or(0);
    assert!(after_stop > 0, "the member that committed before the stop is kept");

    // The reference: what an uninterrupted walk reaches from scratch.
    let (whole_archive, whole_db) = fixture("process-whole").await;
    let whole = run(&whole_db, &whole_archive).await;
    let want = cell_i64(&whole_db, "SELECT COUNT(*) FROM notices").await.unwrap_or(0);
    assert!(after_stop < want, "the stop must actually have cut the walk short: {after_stop} vs {want}");

    // Re-running the stopped one finishes it, and lands on the same corpus.
    let second = run(&db, &archive).await;
    assert!(!second.cancelled);
    assert_eq!(
        cell_i64(&db, "SELECT COUNT(*) FROM notices").await.unwrap_or(0),
        want,
        "a re-run reaches exactly the uninterrupted state: {second:?} vs {whole:?}"
    );

    let _ = std::fs::remove_dir_all(&archive);
    let _ = std::fs::remove_dir_all(&whole_archive);
}

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

    let monthly = process::process(&db, &archive, "ted", "monthly", None, |_, _| {}, || false).await.unwrap();

    // The nested daily's members surface as the monthly's own, under
    // `<daily>.tar.gz/<file>` paths — same counts as the standalone daily.
    assert_eq!(monthly.members, daily.members);
    assert_eq!(monthly.skipped, daily.skipped);
    // Every notice identity is already known: nothing new, all duplicates.
    assert_eq!(monthly.notices, 0);
    assert_eq!(monthly.duplicates, daily.notices);

    let _ = std::fs::remove_dir_all(&archive);
}

/// Issue 196: the pre-recursion walker (before 1e4df1c) recorded a monthly's
/// inner dailies as raw binary MEMBERS — one not-utf8 row per whole
/// `<daily>.tar.gz`, a path the walker never yields again once it descends
/// containers, so no reprocess could dispatch or resolve those rows. A held
/// CONTAINER now dispatches the members inside it, and the first record out of
/// it resolves the container row via the container stamp address.
#[tokio::test]
async fn reclaiming_a_held_container_resolves_its_whole_container_row() {
    let (archive, db) = fixture("reclaim-container").await;
    let daily = run(&db, &archive).await;

    // The monthly carrying the same daily, exactly as the walker sees prod's
    // 2026-06 — but with a stale whole-container row recorded under its fetch.
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
    hold_member(&archive.join("test.db"), 2, "20260601_2026137.tar.gz", "not-utf8", "").await;

    // The bucket's work list carries the container path itself...
    let held = db.quarantine_held_member_files(2, "not-utf8", None, None).await.unwrap();
    assert_eq!(held.len(), 1, "the whole-container row is the bucket");

    // ...and reprocessing dispatches the members INSIDE it: the known
    // identities dedup as already-parsed, and the first of them resolves the
    // container row.
    let report =
        process::reclaim_package(&db, &monthly_path, "ted", 2, held, |_, _, _| {}).await.unwrap();
    // The container's members all dispatch: the 4 genuinely-parsed notices
    // dedup as already-parsed (resolving the row), and the 3 stub notices plus
    // the 1 garbage member re-fail exactly as they would in their own daily.
    assert_eq!((report.already, report.still_held), (4, 4));
    assert!(
        cell_i64(&db, "SELECT reprocessed_at FROM quarantine WHERE member_path = '20260601_2026137.tar.gz'")
            .await
            .is_some(),
        "the whole-container rejection row is resolved"
    );

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
    let report = process::process(&db, &archive, "ted", "daily", None, |_, _| {}, || false).await.unwrap();

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

async fn cell_text(db: &store::Db, sql: &str) -> Option<String> {
    match db.scalar(sql).await.unwrap() {
        Some(store::turso::Value::Text(s)) => Some(s),
        _ => None,
    }
}

/// Overwrite a held row's reason/detail in place — the stale-label state issue 87
/// is about: a row quarantined under a since-fixed defect's label, which a failed
/// re-parse then used to leave untouched.
async fn relabel(db_path: &Path, member_path: &str, reason: &str, detail: &str) {
    let raw = store::turso::Builder::new_local(db_path.to_str().unwrap()).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute(
        "UPDATE quarantine SET reason = ?, detail = ? WHERE member_path = ?",
        (reason, detail, member_path),
    )
    .await
    .unwrap();
}

/// Rewind a parsed notice to the pre-fix state a stale quarantine row records:
/// empty parsed layer, `quarantined`, projected, plus a held quarantine row —
/// exactly what OC/SDK members looked like before their fix shipped. Writes over
/// a raw connection to the same file (the store write API is intentionally
/// narrow); `id` is a trusted i64, inlined.
async fn make_held(db_path: &Path, id: i64) {
    make_held_as(db_path, id, "unknown-field-code", "line 1: OC").await
}

/// [`make_held`] with the bucket's own reason/detail — the reason a member was
/// held decides which reprocess bucket later picks it up.
async fn make_held_as(db_path: &Path, id: i64, reason: &str, detail: &str) {
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
             SELECT id, fetch_id, member_path, content_hash, profile, ?, ?, 0
               FROM notices WHERE id = {id}"
        ),
        (reason, detail),
    )
    .await
    .unwrap();
}

/// The reprocess pass re-parses one held member in place from the archive, and
/// (issue 77) parses ONLY the bucket's held members — walking the package but
/// dispatching just the held one, so a sparse bucket does a fraction of the work.
#[tokio::test]
async fn reclaim_package_reclaims_only_the_held_members() {
    let (archive, db) = fixture("reclaim").await;
    run(&db, &archive).await;
    let pkg = archive.join("ted/daily/2026-00137.tar.gz");

    // An empty held set (a bucket with nothing held here) parses nothing at all —
    // the whole package is walked but skipped, not even no-op'd.
    let empty = process::reclaim_package(&db, &pkg, "ted", 1, Default::default(), |_, _, _| {})
        .await
        .unwrap();
    assert_eq!((empty.reclaimed, empty.already, empty.still_held), (0, 0, 0));
    assert_eq!(empty.members, 9, "all members are walked, none parsed");

    // Rewind one genuinely-parsed notice to the held state, then reprocess just it.
    let id = cell_i64(&db, "SELECT MIN(id) FROM notices WHERE parse_state = 'parsed'").await.unwrap();
    let sections = count(&db, "notice_sections", id).await;
    assert!(sections > 0);
    make_held(&archive.join("test.db"), id).await;
    assert_eq!(count(&db, "notice_sections", id).await, 0);

    // The reprocess work list for this package + bucket is exactly the held member.
    let held =
        db.quarantine_held_member_files(1, "unknown-field-code", Some("%: OC"), None).await.unwrap();
    assert_eq!(held.len(), 1, "one held member file in this bucket");

    let report = process::reclaim_package(&db, &pkg, "ted", 1, held, |_, _, _| {}).await.unwrap();
    // Only the held member is dispatched: it reclaims, and the other 8 members
    // (4 parsed, 3 stubs, 1 garbage) are skipped — never counted as already/held.
    assert_eq!((report.reclaimed, report.already, report.still_held), (1, 0, 0));
    assert_eq!(report.members, 9, "the package is still walked, but only 1 parsed");

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

/// A 2008 OPOCE monthly, in miniature: one notice shipped as an English file
/// plus a French sibling — the shape that carries ~22 language files per notice.
fn write_internal_ojs_package(path: &Path) {
    let gz = flate2::write::GzEncoder::new(
        std::fs::File::create(path).unwrap(),
        flate2::Compression::fast(),
    );
    let mut tar = tar::Builder::new(gz);
    for lang in ["en", "fr"] {
        let bytes = std::fs::read(format!("tests/fixtures/internal_ojs/115165_2008.{lang}"))
            .expect("internal-ojs fixture");
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append_data(&mut header, format!("115165/opoce-input/115165_2008.{lang}"), &*bytes)
            .unwrap();
    }
    tar.into_inner().unwrap().finish().unwrap();
}

/// Issue 84: a reprocess must account for every held member it walks, including
/// the ones a **dispatch policy declines**.
///
/// The 2008 OPOCE export ships each notice once per language; dispatch ingests
/// the English file and skips the ~21 siblings as documented duplicates. Both
/// were quarantined together before issue 36 taught the parser to strip a DTD,
/// so both carry a stale `XML with DTD detected` row — but only the English one
/// can ever reclaim. A skipped member yields no record at all, so the reclaim
/// writes nothing for it and its row stays held forever; before this counter
/// existed the pass walked past it reporting *nothing*, and the outcomes did not
/// sum to the held set. That silence is why ~593k such rows read as an
/// outstanding reclaim backlog rather than as duplicates already ingested.
#[tokio::test]
async fn reclaim_accounts_for_held_members_a_dispatch_policy_skips() {
    let archive = temp_dir("reclaim-skipped");
    std::fs::create_dir_all(archive.join("ted/monthly")).unwrap();
    let pkg = archive.join("ted/monthly/2008-05.tar.gz");
    write_internal_ojs_package(&pkg);

    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    db.record_fetch(&store::Fetch {
        source: "ted".into(),
        kind: "monthly".into(),
        period: "2008-05".into(),
        url: "https://ted.europa.eu/packages/monthly/200805".into(),
        sha256: "bb".into(),
        bytes: 1,
        fetched_at: 1,
        path: "ted/monthly/2008-05.tar.gz".into(),
    })
    .await
    .unwrap();

    // A fresh ingest of this package today: the English file is a notice, the
    // French sibling is skipped by policy — neither is a quarantine.
    let r = process::process(&db, &archive, "ted", "monthly", None, |_, _| {}, || false).await.unwrap();
    assert_eq!((r.members, r.notices, r.skipped, r.quarantined), (2, 1, 1, 0));

    // Rewind to the pre-issue-36 state both members were in: a stale
    // `unparsable-xml` row each. The English one sits over a rewound notice
    // (parse-level); the French one never had a notice row (profile-level),
    // which is exactly how a member that failed before identity was recorded
    // looks in prod.
    let id = cell_i64(&db, "SELECT MIN(id) FROM notices WHERE parse_state = 'parsed'").await.unwrap();
    let sections = count(&db, "notice_sections", id).await;
    assert!(sections > 0);
    make_held_as(&archive.join("test.db"), id, "unparsable-xml", "XML with DTD detected").await;
    hold_member(
        &archive.join("test.db"),
        1,
        "115165/opoce-input/115165_2008.fr",
        "unparsable-xml",
        "XML with DTD detected",
    )
    .await;

    let held = db
        .quarantine_held_member_files(1, "unparsable-xml", Some("XML with DTD detected"), None)
        .await
        .unwrap();
    assert_eq!(held.len(), 2, "both language files are held in this bucket");

    let report = process::reclaim_package(&db, &pkg, "ted", 1, held, |_, _, _| {}).await.unwrap();

    // The English member reclaims; the French one is declined by policy and
    // REPORTED as such — it is not a failure (`still_held` is for members that
    // were parsed and refused) and not a no-op (`already` is for parsed rows).
    assert_eq!(report.reclaimed, 1);
    assert_eq!(report.skipped_by_policy, 1);
    assert_eq!((report.still_held, report.already), (0, 0));
    assert_eq!(
        report.reclaimed + report.still_held + report.already + report.skipped_by_policy,
        2,
        "every held member ends in exactly one reported outcome"
    );

    // The reclaimed member is whole again.
    assert_eq!(count(&db, "notice_sections", id).await, sections);

    // The skipped one is NOT reclaimed — `reprocessed_at` would claim it entered
    // the corpus, and it did not.
    assert_eq!(
        cell_i64(
            &db,
            "SELECT reprocessed_at FROM quarantine WHERE member_path = '115165/opoce-input/115165_2008.fr'"
        )
        .await,
        None,
        "a declined member is never RECLAIMED — it was not ingested"
    );
    // But the outcome is now recorded ON THE ROW, naming the policy that declined
    // it (issue 84's permanent half). Before this, the row was indistinguishable
    // from one nobody had ever examined, so it stayed on every future work list
    // and counted as outstanding work no reprocess could move.
    assert!(
        cell_i64(
            &db,
            "SELECT skipped_at FROM quarantine WHERE member_path = '115165/opoce-input/115165_2008.fr'"
        )
        .await
        .is_some(),
        "a declined member is flagged skipped, so it leaves the work list"
    );
    assert_eq!(
        db.scalar(
            "SELECT skipped_reason FROM quarantine WHERE member_path = '115165/opoce-input/115165_2008.fr'"
        )
        .await
        .unwrap(),
        Some(store::turso::Value::Text("internal-ojs-non-english".into())),
        "the row says WHICH policy declined it, not merely that one did"
    );

    let _ = std::fs::remove_dir_all(&archive);
}

/// Issue 87: a reclaim that FAILS AGAIN records the current failure on the row.
///
/// Two members, two failure shapes:
/// * an eForms stub — identity parses, content does not (`Parse::Quarantined`):
///   the notice-level `StillHeld` exit;
/// * `garbage.xml` — no identity at all (`Record::Quarantine`): the profile-level
///   member the reclaim used to walk past silently, uncounted and unwritten.
///
/// Both rows are first relabeled with a STALE reason, exactly the state the
/// DE-1.x residuals were in (241 rows still claiming "no vendored SDK metadata"
/// after the metadata WAS vendored). Before this fix a failed re-parse left that
/// label in place and wrote the real cause nowhere, so a "did any new reason
/// bucket appear for this cohort?" check was structurally unable to fail.
#[tokio::test]
async fn a_failed_reclaim_records_the_current_failure_on_the_row() {
    let (archive, db) = fixture("reclaim-stillheld").await;
    run(&db, &archive).await;
    let pkg = archive.join("ted/daily/2026-00137.tar.gz");
    let garbage = "20240102_1/garbage.xml";

    // What a re-parse of each member ACTUALLY produces today is what the fresh
    // ingest just recorded — capture it, so the test asserts "the row reflects
    // the current failure" without hard-coding parser messages.
    let stub = cell_text(
        &db,
        "SELECT member_path FROM quarantine WHERE notice_id IS NOT NULL ORDER BY id LIMIT 1",
    )
    .await
    .expect("a parse-level quarantine row from the fixture's eForms stubs");
    let stub_reason =
        cell_text(&db, &format!("SELECT reason FROM quarantine WHERE member_path = '{stub}'"))
            .await
            .unwrap();
    let garbage_reason =
        cell_text(&db, &format!("SELECT reason FROM quarantine WHERE member_path = '{garbage}'"))
            .await
            .unwrap();

    relabel(&archive.join("test.db"), &stub, "stale-reason", "written at first ingest").await;
    relabel(&archive.join("test.db"), garbage, "stale-reason", "written at first ingest").await;

    let held: std::collections::HashSet<String> = [stub.clone(), garbage.to_owned()].into();
    let report =
        process::reclaim_package(&db, &pkg, "ted", 1, held.clone(), |_, _, _| {}).await.unwrap();

    // Both failures are REPORTED — including the profile-level one, which used to
    // vanish from the outcome sum entirely — and the residual's shape rides along.
    assert_eq!((report.reclaimed, report.already, report.skipped_by_policy), (0, 0, 0));
    assert_eq!(report.still_held, 2, "both failure shapes count as still held");
    assert_eq!(report.still_held_reasons.get(&stub_reason), Some(&1));
    assert_eq!(report.still_held_reasons.get(&garbage_reason), Some(&1));

    for (path, current) in [(&stub, &stub_reason), (&garbage.to_owned(), &garbage_reason)] {
        let row = |col: &str| format!("SELECT {col} FROM quarantine WHERE member_path = '{path}'");
        // The row now names the CURRENT failure, with first-ingest provenance kept.
        assert_eq!(cell_text(&db, &row("reason")).await.as_ref(), Some(current));
        assert_eq!(cell_text(&db, &row("first_reason")).await.as_deref(), Some("stale-reason"));
        assert_eq!(
            cell_text(&db, &row("first_detail")).await.as_deref(),
            Some("written at first ingest")
        );
        // Attempted-and-failed is distinguishable from never-reached…
        assert_eq!(cell_i64(&db, &row("attempts")).await, Some(1));
        assert!(cell_i64(&db, &row("last_attempt_at")).await.is_some());
        // …and the member stays in the backlog: not reclaimed, not skipped, and
        // findable by its CURRENT reason on the next run.
        assert_eq!(cell_i64(&db, &row("reprocessed_at")).await, None);
        assert_eq!(cell_i64(&db, &row("skipped_at")).await, None);
    }
    assert!(
        db.quarantine_held_member_files(1, &stub_reason, None, None).await.unwrap().contains(&stub),
        "the failed member belongs to its new reason's bucket now"
    );

    // A second failed attempt counts, and first-ingest provenance is written ONCE.
    let again = process::reclaim_package(&db, &pkg, "ted", 1, held, |_, _, _| {}).await.unwrap();
    assert_eq!(again.still_held, 2);
    for path in [&stub, &garbage.to_owned()] {
        let row = |col: &str| format!("SELECT {col} FROM quarantine WHERE member_path = '{path}'");
        assert_eq!(cell_i64(&db, &row("attempts")).await, Some(2));
        assert_eq!(cell_text(&db, &row("first_reason")).await.as_deref(), Some("stale-reason"));
    }

    let _ = std::fs::remove_dir_all(&archive);
}

/// Insert a profile-level held row: a member quarantined before any notice
/// identity existed, keyed by `(fetch_id, member_path)` alone.
async fn hold_member(db_path: &Path, fetch_id: i64, member_path: &str, reason: &str, detail: &str) {
    let raw = store::turso::Builder::new_local(db_path.to_str().unwrap()).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute(
        "INSERT INTO quarantine(fetch_id, member_path, content_hash, reason, detail, first_seen)
         VALUES (?, ?, 'cc', ?, ?, 0)",
        (fetch_id, member_path, reason, detail),
    )
    .await
    .unwrap();
}

/// Issue 202: a corrupt UTF8 bundle must not supersede its readable ISO twin.
/// The first walk cannot know (single pass, names-only pre-scan): the UTF8
/// member quarantines whole and the ISO is skipped as superseded — but once
/// the ledger HOLDS the bundle as unreadable, the next walk excludes it from
/// the supersedence decision and the day ingests from the ISO copy.
///
/// The package is MONTHLY-shaped: a second day ships a READABLE UTF8 bundle.
/// Supersedence is judged per publication day, so that other day must not
/// keep the corrupt day's ISO suppressed (the first, package-global fix
/// passed a single-day fixture and then recovered 0 notices in production).
#[tokio::test]
async fn corrupt_utf8_bundle_stops_superseding_its_readable_iso() {
    let dir = temp_dir("iso-fallback");
    let archive = dir.as_path();
    std::fs::create_dir_all(archive.join("ted/daily")).unwrap();
    let pkg_path = archive.join("ted/daily/2005-00070.tar.gz");
    const OTHER_DAY: &str = "1.00/000001\n\
        TI: D-Bonn: bridges\n\
        PD: 20050410\n\
        ND: 100-2005\n\
        OJ: 71/2005\n\
        TX:  1.  Awarding authority: Stadt Bonn.\n";
    {
        let gz = flate2::write::GzEncoder::new(
            std::fs::File::create(&pkg_path).unwrap(),
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
        add(
            "EN_20050409_070_ISO_ORG.zip",
            &zip_of("EN_20050409_2005070_ISO_ORG", TEXT_DOC),
        );
        add("EN_20050409_070_UTF8_ORG.ZIP", b"PK\x03\x04 truncated, no central directory");
        add(
            "EN_20050410_071_UTF8_ORG.ZIP",
            &zip_of("EN_20050410_2005071_UTF8_ORG", OTHER_DAY),
        );
        tar.into_inner().unwrap().finish().unwrap();
    }
    let db = store::Db::open(dir.join("test.db").to_str().unwrap()).await.unwrap();
    db.record_fetch(&store::Fetch {
        source: "ted".into(),
        kind: "daily".into(),
        period: "2005-00070".into(),
        url: "u".into(),
        sha256: "h".into(),
        bytes: 1,
        fetched_at: 1,
        path: "ted/daily/2005-00070.tar.gz".into(),
    })
    .await
    .unwrap();

    // First walk: the UTF8 twin exists by NAME, so the ISO is skipped as
    // superseded — and the UTF8 bundle dies unreadable. Only the OTHER day's
    // readable UTF8 ingests; the corrupt day is lost.
    let first = process::process(&db, archive, "ted", "daily", None, |_, _| {}, || false).await.unwrap();
    assert_eq!(first.notices, 1, "only the other day's record ingests: {first:?}");
    assert_eq!(
        cell_i64(&db, "SELECT COUNT(*) FROM notices WHERE member_path LIKE '%20050409%'").await,
        Some(0),
        "the corrupt day is lost on the first walk"
    );
    assert_eq!(
        cell_i64(&db, "SELECT COUNT(*) FROM quarantine WHERE reason LIKE 'unreadable zip%'").await,
        Some(1),
        "the corrupt bundle is held"
    );

    // Second walk: the held unreadable bundle no longer supersedes ITS day —
    // the other day's intact UTF8 must not veto this — so the readable ISO
    // dispatches and the day's records ingest.
    let second = process::process(&db, archive, "ted", "daily", None, |_, _| {}, || false).await.unwrap();
    assert_eq!(second.notices, 2, "the day ingests from the ISO copy: {second:?}");
    assert_eq!(
        cell_i64(&db, "SELECT COUNT(*) FROM notices WHERE member_path LIKE '%20050409%'").await,
        Some(2),
        "the corrupt day's records come back through the ISO twin"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
