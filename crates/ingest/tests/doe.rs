//! DÖE (oeffentlichevergabe.de) profile tests: the eforms-de-2.x national
//! profiles (SDK-DE inventories) and the empirically-inventoried
//! `eforms-sdk-0.1` below-threshold dialect, both in their real serializer
//! shapes — the JAXB `ns2`…`ns9` numeric channel included (issue 12).

use ingest::process;
use ingest::profile::{self, Disposition, Record};
use ingest::{eforms, package};
use std::io::Write;
use std::path::{Path, PathBuf};
use store::{NoticeValue, Parse, Parsed};

const NUMERIC_CN: &str = "doe/sdk-0.1-numeric-cn-25599482-1.xml";
const UUID_CAN: &str = "doe/sdk-0.1-uuid-can-427d4645-163c-419d-93a9-5f5ce05ff9b7-1.xml";
const DE_CAN: &str = "doe/eforms-de-2.1-can-15063f7d-0f02-42f6-960a-96e35c9cc374-01.xml";
const PAIR_DOE: &str = "doe-ted-pair/doe-cn-ebb72363-832d-4cea-8db6-04999414ea8c-01.xml";
const PAIR_TED: &str = "doe-ted-pair/ted-cn-00373130-2026.xml";

fn dispatch_fixture(relative: &str) -> (ingest::profile::NoticeRecord, Vec<u8>) {
    let path = format!("tests/fixtures/{relative}");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let Disposition::Records(records) = profile::dispatch(relative, &bytes) else {
        panic!("{relative}: dispatch skipped a DÖE fixture");
    };
    let [Record::Notice(_)] = &records[..] else {
        panic!("{relative}: expected exactly one notice record");
    };
    let Some(Record::Notice(notice)) = records.into_iter().next() else { unreachable!() };
    (notice, bytes)
}

fn parse_fixture(relative: &str) -> Parsed {
    let (notice, bytes) = dispatch_fixture(relative);
    match eforms::parse_payload(&notice.profile, &bytes) {
        Parse::Parsed(parsed) => parsed,
        Parse::Quarantined { reason, detail } => {
            panic!("{relative} quarantined: {reason}: {}", detail.unwrap_or_default())
        }
        Parse::Pending => panic!("{relative}: no parser ran"),
    }
}

fn values<'a>(parsed: &'a Parsed, section: &str, field: &str) -> Vec<&'a NoticeValue> {
    parsed
        .values
        .iter()
        .filter(|v| v.section_id == section && v.field_id == field)
        .map(|v| &v.value)
        .collect()
}

fn value<'a>(parsed: &'a Parsed, section: &str, field: &str) -> &'a NoticeValue {
    let found = values(parsed, section, field);
    assert_eq!(found.len(), 1, "expected one {field} in {section}, got {}", found.len());
    found[0]
}

fn text(parsed: &Parsed, section: &str, field: &str) -> String {
    match value(parsed, section, field) {
        NoticeValue::Text { value, .. } => value.clone(),
        other => panic!("{field} is not text: {other:?}"),
    }
}

fn section_of_kind<'a>(parsed: &'a Parsed, kind: &str) -> &'a store::Section {
    parsed
        .sections
        .iter()
        .find(|s| s.kind == kind)
        .unwrap_or_else(|| panic!("no section of kind {kind}"))
}

// ------------------------------------------------------------ exhaustiveness

/// ADR-0004 on the DÖE corpus: every fixture of every DÖE profile — both
/// sdk-0.1 serializer channels and eforms-de-2.1 — is consumed exhaustively.
#[test]
fn every_doe_fixture_is_consumed_exhaustively() {
    for relative in [NUMERIC_CN, UUID_CAN, DE_CAN, PAIR_DOE] {
        let parsed = parse_fixture(relative);
        assert!(!parsed.values.is_empty(), "{relative}: parsed but empty");
        for v in &parsed.values {
            assert!(
                parsed.sections.iter().any(|s| s.id == v.section_id),
                "{relative}: {} references unknown section {}",
                v.field_id,
                v.section_id
            );
        }
    }
}

/// DÖE exports carry no publication number; identity is the notice id plus
/// its declared version — the member file name's stem, derived from the
/// document itself.
#[test]
fn doe_notice_identity_is_id_plus_version() {
    let (n, _) = dispatch_fixture(NUMERIC_CN);
    assert_eq!(n.publication_id, "25599482-1");
    assert_eq!(n.profile, "eforms:eforms-sdk-0.1");

    let (n, _) = dispatch_fixture(UUID_CAN);
    assert_eq!(n.publication_id, "427d4645-163c-419d-93a9-5f5ce05ff9b7-1");

    let (n, _) = dispatch_fixture(DE_CAN);
    assert_eq!(n.publication_id, "15063f7d-0f02-42f6-960a-96e35c9cc374-01");
    assert_eq!(n.profile, "eforms:eforms-de-2.1");
}

// ------------------------------------------------------------- sdk-0.1

/// The numeric channel in depth: JAXB single-line serialization, `ns2`…`ns9`
/// prefixes, empty `ContractFolderID`, no organization register — the
/// permanent ~40%-of-German-volume dialect.
#[test]
fn sdk01_numeric_cn_extracts_buyer_title_and_deadline() {
    let cn = parse_fixture(NUMERIC_CN);

    // Buyer: the numeric channel publishes one flat name string, no register.
    let buyer = section_of_kind(&cn, "ContractingParty");
    assert!(
        text(&cn, &buyer.id, "SDK01-ContractingParty-Party-PartyName-Name")
            .starts_with("VGem Volkach, Marktplatz 1, 97332 Volkach")
    );

    // Title and description, procedure-level.
    assert_eq!(text(&cn, "PROCEDURE", "SDK01-ProcurementProject-Name"), "Lose Möblierung");

    // National procedure code from the German codelist family (open domain).
    assert!(matches!(
        value(&cn, "PROCEDURE", "SDK01-TenderingProcess-ProcedureCode"),
        NoticeValue::Code { list, code }
            if code == "de-restricted-wo-call" && list.as_deref() == Some("procurement-procedure-type")
    ));

    // The single pseudo-lot every sdk-0.1 notice carries.
    let lot = section_of_kind(&cn, "Lot");
    assert_eq!(lot.id, "LOT-0000");

    // Submission deadline: EndDate + EndTime pair as one instant, offset kept.
    assert_eq!(
        *value(
            &cn,
            "LOT-0000",
            "SDK01-ProcurementProjectLot-TenderingProcess-TenderSubmissionDeadlinePeriod-EndDate"
        ),
        NoticeValue::Date { utc_seconds: 1_786_572_000, offset_minutes: 120, has_time: true }
    );

    // NUTS arrives as a classification even from the numeric serializer.
    assert!(matches!(
        value(&cn, "PROCEDURE", "SDK01-ProcurementProject-RealizedLocation-Address-CountrySubentityCode"),
        NoticeValue::Classification { scheme, code } if scheme == "nuts" && code == "DE2"
    ));

    // The empty ContractFolderID is claimed but carries no value: the numeric
    // channel has no procedure identity, by design (no cross-source merge).
    assert!(values(&cn, "PROCEDURE", "SDK01-ContractFolderID").is_empty());
}

/// The uuid channel in depth: real procedure id, eSender block, an award —
/// and the dialect's offsetless dates, read as UTC rather than quarantined.
#[test]
fn sdk01_uuid_can_extracts_award_and_regulatory_domain() {
    let can = parse_fixture(UUID_CAN);

    // RegulatoryDomain is an open code domain (de-uvgo/de-vob/de-vol/de-hhr/…):
    // stored as the code published, never validated against a closed set.
    assert!(matches!(
        value(&can, "PROCEDURE", "SDK01-RegulatoryDomain"),
        NoticeValue::Code { code, .. } if code == "de-hhr"
    ));

    // The procedure uuid is real in this channel.
    assert!(matches!(
        value(&can, "PROCEDURE", "SDK01-ContractFolderID"),
        NoticeValue::Id { value, .. } if value == "3d2aac86-4286-4ae2-9bc1-08eb1cc61f80"
    ));

    // Award result: its own section, winner below it.
    let result = section_of_kind(&can, "TenderResult");
    assert!(matches!(
        value(&can, &result.id, "SDK01-TenderResult-TenderResultCode"),
        NoticeValue::Code { code, .. } if code == "selec-w"
    ));
    // `2000-01-01` — no offset, a systematic dialect trait: read as UTC.
    assert_eq!(
        *value(&can, &result.id, "SDK01-TenderResult-AwardDate"),
        NoticeValue::Date { utc_seconds: 946_684_800, offset_minutes: 0, has_time: false }
    );
    let winner = section_of_kind(&can, "WinningParty");
    assert_eq!(winner.parent.as_deref(), Some(result.id.as_str()));
    assert_eq!(
        text(&can, &winner.id, "SDK01-TenderResult-WinningParty-Party-PartyName-Name"),
        "1. Firma: IABG mbH"
    );

    assert!(
        text(&can, &section_of_kind(&can, "ContractingParty").id, "SDK01-ContractingParty-Party-PartyName-Name")
            .starts_with("Planungsamt der Bundeswehr")
    );
}

// ------------------------------------------------------------- eforms-de

/// The eforms-de-2.1 profile in depth, parsed against the vendored SDK-DE
/// inventory: national code values, the declared EU base, exact cents.
#[test]
fn eforms_de_can_extracts_national_codes_and_amounts() {
    let can = parse_fixture(DE_CAN);

    // The EU-base declaration is content under SDK-DE (OPT-002-notice-DET).
    assert!(matches!(
        value(&can, "PROCEDURE", "OPT-002-notice-DET"),
        NoticeValue::Id { value, .. } if value == "eforms-sdk-1.13"
    ));

    // German legal basis and notice subtype (E-subtypes are part of the same
    // open per-profile domain).
    assert!(matches!(
        value(&can, "PROCEDURE", "BT-01-notice"),
        NoticeValue::Code { code, .. } if code == "32014L0024"
    ));
    assert!(matches!(
        value(&can, "PROCEDURE", "OPP-070-notice"),
        NoticeValue::Code { code, .. } if code == "29"
    ));

    // National codelist value for BT-11 — the classification TED rewrites to
    // `cga` on its side; DÖE keeps the richer original.
    let buyer = section_of_kind(&can, "ContractingParty");
    assert!(matches!(
        value(&can, &buyer.id, "BT-11-Procedure-Buyer"),
        NoticeValue::Code { list, code }
            if code == "omu-bbeh" && list.as_deref() == Some("buyer-legal-type")
    ));

    assert_eq!(
        text(&can, "ORG-0000", "BT-500-Organization-Company"),
        "Wasserstraßen- und Schifffahrtsamt Elbe"
    );

    // Money: exact integer cents.
    assert_eq!(
        *value(&can, "PROCEDURE", "BT-161-NoticeResult"),
        NoticeValue::Amount { cents: 36_106_050, currency: "EUR".into() }
    );
    assert_eq!(
        *value(&can, "TEN-0000", "BT-720-Tender"),
        NoticeValue::Amount { cents: 36_106_050, currency: "EUR".into() }
    );
}

/// The ADR-0003 reference pair: the same procedure's CN as DÖE published it
/// and as TED republished it. Both parse exhaustively, and the systematic
/// national-code conversion the research found is visible in the parsed
/// values — the DÖE original carries the richer German classification.
#[test]
fn doe_ted_pair_shows_the_national_code_conversion() {
    let doe = parse_fixture(PAIR_DOE);
    let ted = parse_fixture(PAIR_TED);

    let code_of = |parsed: &Parsed| {
        let buyer = section_of_kind(parsed, "ContractingParty");
        match value(parsed, &buyer.id, "BT-11-Procedure-Buyer") {
            NoticeValue::Code { code, .. } => code.clone(),
            other => panic!("BT-11 is not a code: {other:?}"),
        }
    };
    assert_eq!(code_of(&doe), "omu-bbeh");
    assert_eq!(code_of(&ted), "cga");

    // Same procedure identity on both sides.
    let folder = |parsed: &Parsed| match value(parsed, "PROCEDURE", "BT-04-notice") {
        NoticeValue::Id { value, .. } => value.clone(),
        other => panic!("BT-04 is not an id: {other:?}"),
    };
    assert_eq!(folder(&doe), folder(&ted));
}

/// The DE→EU version map (docs/research/eforms-de-profile.md §1): eForms-DE
/// 2.1 tracks two EU bases, disambiguated by ProfileID, defaulting to the
/// empirically dominant 1.13 when absent; 2.0 has a single base.
#[test]
fn eforms_de_versions_resolve_to_their_eu_base() {
    use eforms::sdk::resolve;
    assert_eq!(resolve("eforms-de-2.0", None), Some("eforms-de-2.0"));
    assert_eq!(
        resolve("eforms-de-2.1", Some("eforms-sdk-1.13")),
        Some("eforms-de-2.1@eforms-sdk-1.13")
    );
    assert_eq!(
        resolve("eforms-de-2.1", Some("eforms-sdk-1.14")),
        Some("eforms-de-2.1@eforms-sdk-1.14")
    );
    assert_eq!(resolve("eforms-de-2.1", None), Some("eforms-de-2.1@eforms-sdk-1.13"));
    // eforms-de-1.x has no SDK-DE artifact and is not vendored: quarantine.
    assert_eq!(resolve("eforms-de-1.1", None), None);
    assert_eq!(resolve("eforms-sdk-0.1", None), Some("eforms-sdk-0.1"));
}

// ------------------------------------------------------------- zip packages

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tender-db-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A ZIP shaped like a DÖE daily export: flat `<id>-<version>.xml` members.
fn write_doe_package(path: &Path) {
    let mut w = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for fixture in [DE_CAN, NUMERIC_CN, UUID_CAN] {
        let name = fixture.rsplit('/').next().unwrap();
        let name = name.split_once("-cn-").or_else(|| name.split_once("-can-")).unwrap().1;
        w.start_file(name, opts).unwrap();
        w.write_all(&std::fs::read(format!("tests/fixtures/{fixture}")).unwrap()).unwrap();
    }
    w.finish().unwrap();
}

/// DÖE archives are plain ZIPs, not tarballs; the walker tells them apart by
/// magic and the processor handles them end to end.
#[tokio::test]
async fn doe_zip_package_processes_end_to_end() {
    let archive = temp_dir("doe-zip");
    std::fs::create_dir_all(archive.join("doe/daily")).unwrap();
    let pkg = archive.join("doe/daily/2026-07-18.zip");
    write_doe_package(&pkg);

    let names = package::entry_names(&pkg).unwrap();
    assert_eq!(names.len(), 3);
    assert!(names.contains(&"25599482-1.xml".to_string()));

    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    db.record_fetch(&store::Fetch {
        source: "doe".into(),
        kind: "daily".into(),
        period: "2026-07-18".into(),
        url: "https://oeffentlichevergabe.de/api/notice-exports?pubDay=2026-07-18&format=eforms.zip"
            .into(),
        sha256: "aa".into(),
        bytes: 1,
        fetched_at: 1,
        path: "doe/daily/2026-07-18.zip".into(),
    })
    .await
    .unwrap();

    let r = process::process(&db, &archive, "doe", "daily", None, |_, _| {}).await.unwrap();
    assert_eq!(r.members, 3);
    assert_eq!(r.notices, 3);
    assert_eq!(r.parsed, 3);
    assert_eq!(r.parse_quarantined, 0);
    assert_eq!(r.quarantined, 0);
    assert_eq!(r.skipped, 0);

    let profiles = db.notice_counts_by_profile().await.unwrap();
    assert_eq!(
        profiles,
        vec![
            ("eforms:eforms-de-2.1".to_string(), 1),
            ("eforms:eforms-sdk-0.1".to_string(), 2),
        ]
    );

    let _ = std::fs::remove_dir_all(&archive);
}
