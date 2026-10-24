//! DÖE (oeffentlichevergabe.de) profile tests: the eforms-de-2.x national
//! profiles (SDK-DE inventories), the empirically-inventoried eForms-DE 1.x
//! generation (issue 75) and the `eforms-sdk-0.1` below-threshold dialect, all
//! in their real serializer shapes — the JAXB `ns2`…`ns9` numeric channel
//! included (issue 12).

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
// eForms-DE 1.x: the earlier national generation (issue 75). No SDK-DE artifact
// exists, so it is parsed against the merged empirical inventory (fields-de-1.x.json).
const DE1_CN: &str = "doe/eforms-de-1.1-cn-7d69b0f7.xml";
const DE1_CAN: &str = "doe/eforms-de-1.2-can-799811c4.xml";
// Issue 394 unit 1: the same DE-1.1 member with the placeholder OJS number the
// live cohort carries — `<efbc:NoticePublicationID schemeName="ojs-notice-id">
// 00000000-1900</…>` inside the eForms extension, beside a perfectly good
// notice id and version. 7,177 real notices look like this (measured corpus-wide
// 2026-09-16); before the guard, `grep -rn '00000000-1900' crates/` found nothing.
const DE1_PLACEHOLDER_PUBID: &str = "doe/eforms-de-1.1-cn-placeholder-pubid-7d69b0f7.xml";

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
    for relative in [NUMERIC_CN, UUID_CAN, DE_CAN, DE1_CN, DE1_CAN, PAIR_DOE] {
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

/// Issue 394 unit 1: a DÖE member that DOES carry a `NoticePublicationID` — the
/// all-zero placeholder — still keys on its own notice id plus version.
///
/// The election takes `NoticePublicationID` first, and the comment above it
/// assumed DÖE exports carry none. 7,177 of them do, and it is
/// `00000000-1900` every time: the identity the code's own comment names sits in
/// the SAME file (`<cbc:ID schemeName="notice-id">` + `<cbc:VersionID>`) and lost
/// the election. Every one of those notices then answered
/// `?publication_id=00000000-1900` while its real key answered nothing — the
/// field issue 217 shipped *because it is the key a consumer holds*, wrong in both
/// directions.
///
/// A shape guard on the value, not a `source = "doe"` reordering: the placeholder
/// is what is wrong, whoever emits it.
#[test]
fn a_placeholder_publication_number_loses_to_the_notices_own_id() {
    let (n, _) = dispatch_fixture(DE1_PLACEHOLDER_PUBID);
    assert_eq!(
        n.publication_id, "7d69b0f7-2605-448f-9495-676458dcddc2-01",
        "an all-zero OJS number is not a publication number; the notice id and \
         version are, and they are in the same file"
    );
    assert_eq!(n.profile, "eforms:eforms-de-1.1");

    // The control: the same member WITHOUT the placeholder keys identically, so
    // the guard changed nothing about how a DÖE notice is identified — it only
    // stopped one value from winning.
    let (plain, _) = dispatch_fixture(DE1_CN);
    assert_eq!(plain.publication_id, n.publication_id);
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
    // eForms-DE 1.x has no SDK-DE artifact either, so — like sdk-0.1 — all three
    // minors resolve to one merged empirical inventory (issue 75).
    assert_eq!(resolve("eforms-de-1.0", None), Some("eforms-de-1.x"));
    assert_eq!(resolve("eforms-de-1.1", None), Some("eforms-de-1.x"));
    assert_eq!(resolve("eforms-de-1.2", None), Some("eforms-de-1.x"));
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

    let r = process::process(&db, &archive, "doe", "daily", None, |_, _| {}, || false).await.unwrap();
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

/// eForms-DE 1.x has no SDK-DE `fields.json` artifact (issue 75), so it is
/// parsed against the merged empirical era inventory — every element path
/// observed across the archived eforms-de-1.0/1.1/1.2 corpus. Both minors must
/// parse exhaustively (ADR-0004) into the expected sections and business
/// content, exactly like the vendored 2.x profiles.
#[test]
fn eforms_de_1x_contract_notice_parses_against_the_empirical_inventory() {
    let cn = parse_fixture(DE1_CN);

    // The declared national customization is claimed content, not plumbing.
    assert!(matches!(
        value(&cn, "PROCEDURE", "DE1-CustomizationID"),
        NoticeValue::Id { value, .. } if value == "eforms-de-1.1"
    ));
    // German legal basis + notice subtype, procedure-level.
    assert!(matches!(
        value(&cn, "PROCEDURE", "DE1-RegulatoryDomain"),
        NoticeValue::Text { value, .. } if value == "32014L0024"
    ));
    assert!(matches!(
        value(&cn, "PROCEDURE", "DE1-NoticeSubType-SubTypeCode"),
        NoticeValue::Code { code, .. } if code == "16"
    ));

    // The organization register: sections keyed on the published ORG id (the
    // empirical inventory's deep identifier under efac:Company).
    let org = cn.sections.iter().find(|s| s.id == "ORG-7001").expect("ORG-7001 section");
    assert_eq!(org.kind, "Organization");
    // The org name sits in its PartyName block, which hangs off the org section.
    let name = cn
        .sections
        .iter()
        .filter(|s| s.parent.as_deref() == Some("ORG-7001"))
        .flat_map(|s| values(&cn, &s.id, "DE1-Organizations-Organization-Company-PartyName-Name"))
        .next()
        .expect("an org name");
    assert!(matches!(name, NoticeValue::Text { value, .. } if value == "Städtisches Klinikum Görlitz gGmbH"));
    // NUTS is a classification even here.
    assert!(matches!(
        value(&cn, "ORG-7001", "DE1-Organizations-Organization-Company-PostalAddress-CountrySubentityCode"),
        NoticeValue::Classification { scheme, code } if scheme == "nuts" && code == "DED2D"
    ));

    // Lots are addressed by their published id.
    let lot = section_of_kind(&cn, "ProcurementProjectLot");
    assert_eq!(lot.id, "LOT-0000");
}

/// The award-notice (eForms-DE 1.2) result layer — LotResult, LotTender,
/// SettledContract, TenderingParty — sections and parses whole.
#[test]
fn eforms_de_1x_award_notice_builds_the_result_layer() {
    let can = parse_fixture(DE1_CAN);
    assert!(matches!(
        value(&can, "PROCEDURE", "DE1-CustomizationID"),
        NoticeValue::Id { value, .. } if value == "eforms-de-1.2"
    ));
    for kind in ["LotResult", "LotTender", "SettledContract", "TenderingParty", "Organization"] {
        assert!(
            can.sections.iter().any(|s| s.kind == kind),
            "expected a {kind} section; got {:?}",
            can.sections.iter().map(|s| &s.kind).collect::<Vec<_>>()
        );
    }
    // Every value belongs to a real section (ADR-0004 exhaustiveness sanity).
    assert!(!can.values.is_empty());
    for v in &can.values {
        assert!(can.sections.iter().any(|s| s.id == v.section_id), "orphan {}", v.field_id);
    }
}

/// Issue 100: the result sections must be identified by the ids the REFERENCES
/// use, or the winner chain resolves to nothing.
///
/// The defect: `ND-LotResult`/`ND-LotTender`/`ND-SettledContract`/
/// `ND-TenderingParty` carried no `identifierFieldId`, so the parser synthesised
/// section ids (`ND-LotTender#0`) while the notice's own references kept the
/// published `TEN-`/`TPA-`/`CON-` ids. Lots and organizations resolved (they had
/// identifiers), every result-to-result hop did not — the projection's winner
/// chain LotResult →(OPT-320)→ LotTender →(OPT-310)→ TenderingParty → Tenderer
/// broke at the first hop and DE-1.x awards projected 2% winners.
///
/// This asserts the identity, not the projection: a section's id IS the id its
/// referrers name. The reference-holder positions must NOT be identified this way
/// (see the inventory's post-generation note), which the last assertion pins.
#[test]
fn de_1x_result_sections_are_identified_by_their_published_ids() {
    let can = parse_fixture(DE1_CAN);
    // Each entity must have a DEFINITION section carrying its published id. The
    // same kind may also appear as reference-holder sections (the nested
    // `LotResult/LotTender` etc.), which keep synthetic ids on purpose — so this
    // asserts existence, not that every section of the kind is published-id'd.
    for (kind, prefix) in [
        ("LotResult", "RES-"),
        ("LotTender", "TEN-"),
        ("SettledContract", "CON-"),
        ("TenderingParty", "TPA-"),
    ] {
        let ids: Vec<&String> =
            can.sections.iter().filter(|s| s.kind == kind).map(|s| &s.id).collect();
        assert!(
            ids.iter().any(|id| id.starts_with(prefix)),
            "{kind} needs a section identified by its published {prefix}… id — a wholly synthetic \
             set here is the issue-100 defect, since the references carry the published ids. Got \
             {ids:?}"
        );
    }

    // The chain the fixture actually publishes: the LotTender's tendering-party
    // reference names a TenderingParty section that EXISTS under that id, and
    // that party's Tenderer names an Organization. Both hops must resolve by id
    // for a winner to reach the canonical layer.
    let party_ref = can
        .values
        .iter()
        .find(|v| v.field_id == "DE1-NoticeResult-LotTender-TenderingParty-ID")
        .map(|v| match &v.value {
            NoticeValue::Id { value, .. } => value.clone(),
            other => panic!("expected an id reference, got {other:?}"),
        })
        .expect("the fixture's LotTender references a TenderingParty");
    assert!(
        can.sections.iter().any(|s| s.kind == "TenderingParty" && s.id == party_ref),
        "OPT-310 {party_ref:?} must name a real TenderingParty section; sections: {:?}",
        can.sections.iter().filter(|s| s.kind == "TenderingParty").map(|s| &s.id).collect::<Vec<_>>()
    );

    // And the reference holders stay anonymous: identifying them by the id they
    // POINT AT would give two sections the same id — the definition and the
    // pointer — which is why only the definition positions were wired.
    let tender_sections: Vec<&String> =
        can.sections.iter().filter(|s| s.kind == "LotTender").map(|s| &s.id).collect();
    let mut unique = tender_sections.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(
        tender_sections.len(),
        unique.len(),
        "no two sections may share an id: {tender_sections:?}"
    );
}
