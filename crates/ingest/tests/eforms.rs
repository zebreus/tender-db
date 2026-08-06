//! eForms profile tests, driven by the committed real-notice corpus
//! (`tests/fixtures/README.md`).
//!
//! Two guarantees are under test. ADR-0004: every element, attribute and text
//! node of a real notice is claimed, or the notice is quarantined whole.
//! ADR-0002: every field id of every vendored SDK version has a recorded
//! mapping decision — the completeness harness.

use ingest::eforms::{self, sdk};
use ingest::profile::{self, Disposition, Record};
use store::{NoticeValue, Parse, Parsed};

/// Run a fixture through the real chain: profile dispatch, then the profile's
/// parser — the same two steps `process` performs on an archived package.
fn ingest_fixture(relative: &str) -> Parse {
    let path = format!("tests/fixtures/{relative}");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let Disposition::Records(records) = profile::dispatch(relative, &bytes) else {
        panic!("{relative}: dispatch skipped an eForms fixture");
    };
    let [Record::Notice(notice)] = &records[..] else {
        panic!("{relative}: expected exactly one notice record");
    };
    eforms::parse_payload(&notice.profile, &bytes)
}

fn parse_fixture(relative: &str) -> Parsed {
    match ingest_fixture(relative) {
        Parse::Parsed(parsed) => parsed,
        Parse::Quarantined { reason, detail } => {
            panic!("{relative} quarantined: {reason}: {}", detail.unwrap_or_default())
        }
        Parse::Pending => panic!("{relative}: no parser ran"),
    }
}

fn fixtures(dir: &str) -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(format!("tests/fixtures/{dir}"))
        .expect("fixture directory")
        .filter_map(|e| {
            let name = e.ok()?.file_name().to_string_lossy().into_owned();
            name.ends_with(".xml").then(|| format!("{dir}/{name}"))
        })
        .collect();
    out.sort();
    out
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

fn kind_of(parsed: &Parsed, section: &str) -> String {
    parsed
        .sections
        .iter()
        .find(|s| s.id == section)
        .unwrap_or_else(|| panic!("no section {section}"))
        .kind
        .clone()
}

// ------------------------------------------------------------ exhaustiveness

/// The acceptance condition of ADR-0004 on real data: nothing in a published
/// TED eForms notice goes unclaimed. Every notice type in the corpus is here —
/// CN, CAN, framework CAN, withheld CAN, change, PIN, VEAT, BRIN — plus a
/// four-notice procedure chain.
#[test]
fn every_ted_eforms_fixture_is_consumed_exhaustively() {
    let corpus: Vec<String> = fixtures("eforms").into_iter().chain(fixtures("eforms-chain")).collect();
    assert_eq!(corpus.len(), 15, "corpus changed; update the expectation");

    for relative in corpus {
        match ingest_fixture(&relative) {
            Parse::Parsed(parsed) => {
                assert!(!parsed.values.is_empty(), "{relative}: parsed but empty");
                // Every value belongs to a section that exists.
                for v in &parsed.values {
                    assert!(
                        parsed.sections.iter().any(|s| s.id == v.section_id),
                        "{relative}: {} references unknown section {}",
                        v.field_id,
                        v.section_id
                    );
                }
            }
            other => panic!("{relative}: {other:?}"),
        }
    }
}

// ------------------------------------------------------------- value mapping

/// The contract notice, asserted in depth: this is the shape every consumer of
/// the notice-parsed layer sees.
#[test]
fn contract_notice_extracts_its_business_content() {
    let cn = parse_fixture("eforms/cn-16-00494343-2026.xml");

    // Notice identity and type.
    assert!(matches!(value(&cn, "PROCEDURE", "OPP-070-notice"), NoticeValue::Code { code, .. } if code == "16"));
    assert!(
        matches!(value(&cn, "PROCEDURE", "OPP-010-notice"), NoticeValue::Id { value, .. } if value == "00494343-2026")
    );

    // Title, in the notice's own language — CONTEXT.md's "original".
    assert_eq!(text(&cn, "LOT-0001", "BT-21-Lot"), "Dachdecker-/Spenglerarbeiten");
    assert!(
        matches!(value(&cn, "LOT-0001", "BT-21-Lot"), NoticeValue::Text { lang, .. } if lang.as_deref() == Some("DEU"))
    );

    // Buyer: a role section holding buyer-specific attributes plus a reference
    // into the notice's organization register — the buyer is a *role* an
    // Organization plays, not a second organization record.
    let buyer = cn
        .sections
        .iter()
        .find(|s| s.kind == "ContractingParty")
        .expect("a buyer role section");
    assert!(matches!(
        value(&cn, &buyer.id, "OPT-300-Procedure-Buyer"),
        NoticeValue::Id { value, is_ref: true, .. } if value == "ORG-0001"
    ));
    assert!(matches!(
        value(&cn, &buyer.id, "BT-11-Procedure-Buyer"),
        NoticeValue::Code { code, .. } if code == "la"
    ));
    assert_eq!(text(&cn, "ORG-0001", "BT-500-Organization-Company"), "Stadt Marktheidenfeld");
    assert_eq!(text(&cn, "ORG-0001", "BT-513-Organization-Company"), "Marktheidenfeld");
    assert_eq!(kind_of(&cn, "ORG-0001"), "Organization");

    // Lots, addressed by the identifier the notice published.
    assert_eq!(kind_of(&cn, "LOT-0001"), "Lot");
    assert!(matches!(value(&cn, "LOT-0001", "BT-137-Lot"), NoticeValue::Id { value, .. } if value == "LOT-0001"));

    // Submission deadline: 2026-08-24 14:00 at UTC+2, stored as the instant
    // plus the buyer's offset.
    assert_eq!(
        *value(&cn, "LOT-0001", "BT-131(d)-Lot"),
        NoticeValue::Date { utc_seconds: 1_787_572_800, offset_minutes: 120, has_time: true }
    );

    // CPV: one main classification, three additional ones.
    assert!(matches!(
        value(&cn, "PROCEDURE", "BT-262-Procedure"),
        NoticeValue::Classification { scheme, code } if scheme == "cpv" && code == "45261210"
    ));
    let additional: Vec<_> = cn
        .values
        .iter()
        .filter(|v| v.field_id == "BT-263-Procedure")
        .map(|v| match &v.value {
            NoticeValue::Classification { code, .. } => code.as_str(),
            other => panic!("not a classification: {other:?}"),
        })
        .collect();
    assert_eq!(additional, ["45260000", "44112500", "44112400"]);

    // NUTS is a classification too, not a plain code.
    assert!(matches!(
        value(&cn, "ORG-0001", "BT-507-Organization-Company"),
        NoticeValue::Classification { scheme, code } if scheme == "nuts" && code == "DE26A"
    ));

    // A duration measure keeps its unit rather than becoming a bare number.
    assert_eq!(
        *value(&cn, "LOT-0001", "BT-98-Lot"),
        NoticeValue::Number { value: 60.0, unit: Some("DAY".into()) }
    );
}

/// Money is exact integer cents plus its currency (CONTEXT.md).
#[test]
fn award_notice_amounts_are_exact_cents() {
    let can = parse_fixture("eforms/can-29-00495054-2026.xml");
    assert_eq!(
        *value(&can, "PROCEDURE", "BT-161-NoticeResult"),
        NoticeValue::Amount { cents: 300_000_000, currency: "NOK".into() }
    );
    // Per-bid values, each under the bid (eForms "LotTender") it belongs to.
    assert_eq!(
        *value(&can, "TEN-0001", "BT-720-Tender"),
        NoticeValue::Amount { cents: 349_864_000, currency: "NOK".into() }
    );
    assert_eq!(kind_of(&can, "TEN-0001"), "LotTender");
    assert_eq!(kind_of(&can, "CON-0001"), "SettledContract");
}

/// The withheld-field mechanism: the notice publishes *that* a value is
/// suppressed, why, and until when. Each block is its own section so the four
/// business terms stay grouped.
#[test]
fn withheld_fields_keep_their_reason_and_release_date() {
    let can = parse_fixture("eforms/can-withheld-29-00495618-2026.xml");
    let privacy: Vec<_> = can.sections.iter().filter(|s| s.kind == "FieldsPrivacy").collect();
    assert!(privacy.len() >= 5, "expected several withheld blocks, got {}", privacy.len());

    let block = privacy
        .iter()
        .find(|s| {
            values(&can, &s.id, "BT-195(BT-759)-LotResult")
                .first()
                .is_some_and(|v| matches!(v, NoticeValue::Code { code, .. } if code == "rec-sub-cou"))
        })
        .expect("the withheld received-submission count");

    // Which field, why, until when — and whose section it belongs to.
    assert!(matches!(
        value(&can, &block.id, "BT-197(BT-759)-LotResult"),
        NoticeValue::Code { code, .. } if code == "oth-int"
    ));
    assert!(text(&can, &block.id, "BT-196(BT-759)-LotResult").starts_with("Er wordt geen"));
    assert!(matches!(
        value(&can, &block.id, "BT-198(BT-759)-LotResult"),
        NoticeValue::Date { utc_seconds: 1_911_254_400, .. }
    ));
    assert!(block.parent.is_some());
}

/// Framework award and business-registration notices differ structurally from
/// a plain CAN; both must still parse.
#[test]
fn framework_award_and_business_registration_notices_parse() {
    let fa = parse_fixture("eforms/can-fa-29-00495185-2026.xml");
    assert!(fa.sections.iter().any(|s| s.kind == "LotResult"));

    // BRIN is rooted in eForms' own p27 namespace rather than UBL, and carries
    // a BusinessParty instead of lots.
    let brin = parse_fixture("eforms/brin-x01-00497689-2026.xml");
    assert!(brin.values.iter().any(|v| v.field_id.starts_with("OPP-")));
    assert!(brin.values.len() > 10, "BRIN parsed unexpectedly thin");
}

/// A `CustomizationID` outside the vendored range quarantines rather than
/// being parsed against a neighbouring version's metadata. (Vendored: the DÖE
/// profiles — eforms-de-2.x, eforms-de-1.x, sdk-0.1 — and EU SDK 1.0/1.3/1.5/1.6/1.7
/// (issue 74) + 1.8–1.15. EU minors with no notices in the corpus — e.g. 1.2 —
/// are deliberately not vendored, so they still quarantine.)
#[test]
fn customizations_outside_the_vendored_range_quarantine() {
    let xml = r#"<?xml version="1.0"?>
<ContractNotice xmlns="urn:oasis:names:specification:ubl:schema:xsd:ContractNotice-2"
    xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2">
  <cbc:CustomizationID>eforms-sdk-1.2</cbc:CustomizationID>
</ContractNotice>"#;
    match eforms::parse_payload("eforms:eforms-sdk-1.2", xml.as_bytes()) {
        Parse::Quarantined { reason, detail } => {
            assert_eq!(reason, "unknown-customization");
            assert!(detail.unwrap_or_default().contains("no vendored SDK metadata"));
        }
        other => panic!("should have quarantined, got {other:?}"),
    }
}

/// Issue 74: a real SDK 1.7 award notice bearing `efbc:CompanySizeCode` (BT-165)
/// — the field whose `//` + boolean-`or` node-set-join predicate blocked SDK
/// 1.0–1.7 — parses exhaustively (`parse_fixture` panics on any quarantine), and
/// every company size is claimed as its Organization's BT-165. A size code on an
/// org the join does not select still consumes via the walker's relaxed claim, so
/// none is dropped — which is why the whole notice parses whole.
#[test]
fn sdk_17_company_size_join_field_is_captured() {
    let parsed = parse_fixture("eforms/can-maximal-sdk17.xml");
    let sizes: Vec<&NoticeValue> = parsed
        .values
        .iter()
        .filter(|v| v.field_id == "BT-165-Organization-Company")
        .map(|v| &v.value)
        .collect();
    // The maximal example carries four `efbc:CompanySizeCode` elements; all four
    // are captured (none dropped), each as a code in an Organization section.
    assert_eq!(sizes.len(), 4, "all four company sizes are claimed as BT-165");
    for v in &sizes {
        assert!(matches!(v, NoticeValue::Code { .. }), "company size is a code: {v:?}");
    }
    assert!(
        parsed
            .values
            .iter()
            .filter(|v| v.field_id == "BT-165-Organization-Company")
            .all(|v| v.section_id != "PROCEDURE"),
        "each company size hangs off its Organization section, not the notice root"
    );
}

/// Issue 141: TED's publication pipeline stamps the BT-803 transmission
/// instant onto published notices regardless of the minor they declare —
/// `efbc:TransmissionTime` is an SDK-1.5.0+ field, yet since ~2023-05 it is
/// stamped onto notices still declaring eforms-sdk-1.3 (whose inventory only
/// knows the date half). The stamp is publisher envelope metadata, not buyer
/// form content, so it must be claimed and stored where BT-803 belongs.
/// The fragment models the failing members' envelope: the stamp pair first
/// under `efext:EformsExtension`.
#[test]
fn ted_transmission_stamp_is_claimed_on_older_minors() {
    let xml = r#"<?xml version="1.0"?>
<ContractAwardNotice xmlns="urn:oasis:names:specification:ubl:schema:xsd:ContractAwardNotice-2"
    xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2"
    xmlns:ext="urn:oasis:names:specification:ubl:schema:xsd:CommonExtensionComponents-2"
    xmlns:efext="http://data.europa.eu/p27/eforms-ubl-extensions/1"
    xmlns:efbc="http://data.europa.eu/p27/eforms-ubl-extension-basic-components/1">
  <ext:UBLExtensions>
    <ext:UBLExtension>
      <ext:ExtensionContent>
        <efext:EformsExtension>
          <efbc:TransmissionDate>2023-06-01+02:00</efbc:TransmissionDate>
          <efbc:TransmissionTime>10:30:00+02:00</efbc:TransmissionTime>
        </efext:EformsExtension>
      </ext:ExtensionContent>
    </ext:UBLExtension>
  </ext:UBLExtensions>
  <cbc:CustomizationID>eforms-sdk-1.3</cbc:CustomizationID>
</ContractAwardNotice>"#;
    let parsed = match eforms::parse_payload("eforms:eforms-sdk-1.3", xml.as_bytes()) {
        Parse::Parsed(parsed) => parsed,
        other => panic!("sdk-1.3 notice with a TED transmission stamp must parse, got {other:?}"),
    };
    // The date half claims its time counterpart (UBL Date/Time pairing), so
    // the stamp lands as one instant under BT-803(d)-notice.
    assert_eq!(
        *value(&parsed, "PROCEDURE", "BT-803(d)-notice"),
        NoticeValue::Date { utc_seconds: 1_685_608_200, offset_minutes: 120, has_time: true }
    );
}

/// The negative control for the issue-141 carve-out: it claims exactly the
/// transmission stamp, not arbitrary unknown envelope content — a genuinely
/// unknown element in the same position still quarantines the notice whole.
#[test]
fn unknown_envelope_content_still_quarantines_on_older_minors() {
    let xml = r#"<?xml version="1.0"?>
<ContractAwardNotice xmlns="urn:oasis:names:specification:ubl:schema:xsd:ContractAwardNotice-2"
    xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2"
    xmlns:ext="urn:oasis:names:specification:ubl:schema:xsd:CommonExtensionComponents-2"
    xmlns:efext="http://data.europa.eu/p27/eforms-ubl-extensions/1"
    xmlns:efbc="http://data.europa.eu/p27/eforms-ubl-extension-basic-components/1">
  <ext:UBLExtensions>
    <ext:UBLExtension>
      <ext:ExtensionContent>
        <efext:EformsExtension>
          <efbc:TransmissionDate>2023-06-01+02:00</efbc:TransmissionDate>
          <efbc:BogusEnvelopeStamp>x</efbc:BogusEnvelopeStamp>
        </efext:EformsExtension>
      </ext:ExtensionContent>
    </ext:UBLExtension>
  </ext:UBLExtensions>
  <cbc:CustomizationID>eforms-sdk-1.3</cbc:CustomizationID>
</ContractAwardNotice>"#;
    match eforms::parse_payload("eforms:eforms-sdk-1.3", xml.as_bytes()) {
        Parse::Quarantined { reason, detail } => {
            assert_eq!(reason, "unclaimed-content");
            assert!(detail.unwrap_or_default().contains("BogusEnvelopeStamp"));
        }
        other => panic!("unknown envelope content should quarantine, got {other:?}"),
    }
}

/// Issue 78: real DÖE `eforms-sdk-1.0` notices parse exhaustively — the DÖE JAXB
/// serializer's structural quirks (UBO nested under `efac:Organization`, and the
/// appeal/tender-recipient bodies inlined as full UBL parties) are grafted onto
/// the SDK's Company/UBO subtrees so nothing goes unclaimed.
#[test]
fn doe_sdk10_serializer_quirks_are_consumed() {
    // Nested UBO: its efac:Nationality is claimed as BT-706, and the inlined
    // AppealReceiverParty's fields land via the Company graft.
    let ubo = parse_fixture("eforms/doe-sdk10-ubo-appeal.xml");
    assert!(
        ubo.values.iter().any(|v| v.field_id == "BT-706-UBO"),
        "the nested UBO's nationality is claimed as BT-706"
    );

    // Inlined tender-recipient party under a Lot's TenderingTerms.
    let tr = parse_fixture("eforms/doe-sdk10-tenderrecipient.xml");
    assert!(!tr.values.is_empty(), "the tender-recipient notice parses to fields");
}

// -------------------------------------------------------------- completeness

/// ADR-0002's harness: walk the vendored `fields.json` of every accepted SDK
/// version and fail listing any field id without a mapping decision. This is
/// what makes "all business terms, no omissions" a checked claim rather than a
/// promise.
#[test]
fn every_sdk_field_has_a_mapping_decision() {
    for &(customization, _) in sdk::ACCEPTED {
        let sdk = sdk::load(customization).expect("vendored SDK");
        let decisions = sdk.decisions();
        let unaccounted: Vec<&str> =
            decisions.iter().filter(|(_, d)| d.is_none()).map(|(id, _)| *id).collect();
        assert!(
            unaccounted.is_empty(),
            "{customization} ({}): {} field ids have no mapping or documented exclusion: {unaccounted:?}",
            sdk.sdk_version,
            unaccounted.len()
        );
        // The empirical inventories (sdk-0.1, eforms-de-1.x) are smaller by
        // nature — a few hundred observed leaf paths; every SDK-derived
        // inventory carries 1200+ fields.
        let empirical = matches!(customization, "eforms-sdk-0.1" | "eforms-de-1.x");
        let min = if empirical { 250 } else { 700 };
        assert!(decisions.len() > min, "{customization}: only {} fields loaded", decisions.len());
    }
}

/// Every field's xpath must also be *reachable*: a decision that never gets
/// matched would be an accounting fiction. Building the index parses all 1256
/// xpaths of each version and fails on any shape the parser cannot represent.
#[test]
fn every_sdk_xpath_folds_into_the_match_index() {
    for &(customization, _) in sdk::ACCEPTED {
        let sdk = sdk::load(customization).expect("vendored SDK");
        eforms::index::build(sdk).unwrap_or_else(|e| panic!("{customization}: {e}"));
    }
}

/// The pinned versions this issue vendored, recorded so a bump is deliberate.
#[test]
fn the_pinned_sdk_versions_are_the_vendored_ones() {
    let versions: Vec<&str> = sdk::ACCEPTED.iter().map(|&(id, _)| id).collect();
    assert_eq!(
        versions,
        [
            "eforms-sdk-1.0",
            "eforms-sdk-1.3",
            "eforms-sdk-1.5",
            "eforms-sdk-1.6",
            "eforms-sdk-1.7",
            "eforms-sdk-1.8",
            "eforms-sdk-1.9",
            "eforms-sdk-1.10",
            "eforms-sdk-1.11",
            "eforms-sdk-1.12",
            "eforms-sdk-1.13",
            "eforms-sdk-1.14",
            "eforms-sdk-1.15",
            "eforms-de-2.0",
            "eforms-de-2.1@eforms-sdk-1.13",
            "eforms-de-2.1@eforms-sdk-1.14",
            "eforms-sdk-0.1",
            "eforms-de-1.x",
        ]
    );
    assert_eq!(sdk::load("eforms-sdk-1.15").unwrap().sdk_version, "eforms-sdk-1.15.0");
    // SDK-DE self-identifies via its national version; the vendored files are
    // tags 1.12.6 / 1.13.3 / 1.14.4 of gitlab.opencode.de SDK-eforms-de.
    assert_eq!(sdk::load("eforms-de-2.0").unwrap().sdk_version, "eforms-de-2.0.0");
    assert_eq!(sdk::load("eforms-de-2.1@eforms-sdk-1.13").unwrap().sdk_version, "eforms-de-2.1.0");
    assert_eq!(sdk::load("eforms-de-2.1@eforms-sdk-1.14").unwrap().sdk_version, "eforms-de-2.1.0");
    assert_eq!(sdk::load("eforms-sdk-0.1").unwrap().sdk_version, "eforms-sdk-0.1");
    // The eForms-DE 1.x minors all resolve to the one merged empirical inventory.
    assert_eq!(sdk::load("eforms-de-1.x").unwrap().sdk_version, "eforms-de-1.x");
    for minor in ["eforms-de-1.0", "eforms-de-1.1", "eforms-de-1.2"] {
        assert_eq!(sdk::resolve(minor, None), Some("eforms-de-1.x"));
    }
}

