//! `internal-ojs` profile tests (issue 41): the 2008 OPOCE INTERNAL_OJS export
//! (DTD R2.0.5), driven by byte-exact real notices across the form shapes the
//! era uses — a `_SUM` summary contract notice, a `_SUM` award, and a full
//! (non-summary) contract notice — plus the committed EEIG fixture.
//!
//! Same two guarantees as the r209 suite. ADR-0004: every element, attribute
//! and text node is claimed or the notice quarantines whole. ADR-0002
//! (era-scoped): every envelope/backbone element the profile decides has a
//! rule, and the `_SUM` alias table pins its reuse of the r209 registry.

use ingest::process::parse_payload;
use ingest::profile::{self, Disposition, Record};
use store::{NoticeValue, Parse, Parsed};

fn ingest_fixture(relative: &str) -> (String, Parse) {
    let path = format!("tests/fixtures/{relative}");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let member = format!("20080502_2008085.tar.gz/{relative}");
    let Disposition::Records(records) = profile::dispatch(&member, &bytes) else {
        panic!("{relative}: dispatch skipped an INTERNAL_OJS fixture");
    };
    let [Record::Notice(notice)] = &records[..] else {
        panic!("{relative}: expected exactly one notice record");
    };
    (notice.profile.clone(), parse_payload(&notice.profile, &bytes))
}

fn parse_fixture(relative: &str) -> Parsed {
    match ingest_fixture(relative).1 {
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

fn code(parsed: &Parsed, section: &str, field: &str) -> String {
    match value(parsed, section, field) {
        NoticeValue::Code { code, .. } => code.clone(),
        other => panic!("{field} is not a code: {other:?}"),
    }
}

fn text(parsed: &Parsed, section: &str, field: &str) -> String {
    match value(parsed, section, field) {
        NoticeValue::Text { value, .. } => value.clone(),
        other => panic!("{field} is not text: {other:?}"),
    }
}

const FIXTURES: [&str; 4] = [
    "internal_ojs/114238_2008.en", // EEIG (heading 02A0), the minimal committed fixture
    "internal_ojs/115165_2008.en", // CONTRACT_SUM contract notice (heading 2110), orig FR
    "internal_ojs/114382_2008.en", // CONTRACT_AWARD_SUM award (heading 1180), orig PL
    "internal_ojs/115908_2008.en", // full CONTRACT notice (heading 3310), orig EN
];

// ------------------------------------------------------------ exhaustiveness

/// ADR-0004 on real data: nothing in a published INTERNAL_OJS notice goes
/// unclaimed — the envelope backbone, the `_SUM` summary forms, and the full
/// r209-vocabulary forms alike.
#[test]
fn every_internal_ojs_fixture_is_consumed_exhaustively() {
    for relative in FIXTURES {
        let (profile, parse) = ingest_fixture(relative);
        assert_eq!(profile, "internal-ojs");
        match parse {
            Parse::Parsed(parsed) => {
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
            other => panic!("{relative} ({profile}): {other:?}"),
        }
    }
}

// ------------------------------------------------------------- envelope

/// The coded backbone every INTERNAL_OJS notice carries: identity in OJS-ref
/// form, the heading, the bare-text CODIF single-char codes, the CPV as a
/// classification (bare text, not r209's `@CODE`), `ISO_COUNTRY` as a code (the
/// override — r209's `@VALUE` rule would drop it), and the dispatch date.
#[test]
fn envelope_backbone_maps_identity_and_codif_codes() {
    let cn = parse_fixture("internal_ojs/115165_2008.en");

    assert!(matches!(value(&cn, "PROCEDURE", "TED-NO_DOC_OJS"),
        NoticeValue::Id { value, is_ref: false, .. } if value == "2008/S 85-115165"));
    assert_eq!(code(&cn, "PROCEDURE", "TED-HEADING"), "2110");
    // Bare-text single-char CODIF codes (SECTOR=6, MARKET=2, PROC=1, …).
    assert_eq!(code(&cn, "PROCEDURE", "TED-SECTOR"), "6");
    assert_eq!(code(&cn, "PROCEDURE", "TED-MARKET"), "2");
    assert_eq!(code(&cn, "PROCEDURE", "TED-PROC"), "1");
    assert_eq!(code(&cn, "PROCEDURE", "TED-TYPE_BID"), "3");
    assert_eq!(code(&cn, "PROCEDURE", "TED-MAIN_ACTIVITIES"), "C");
    // ISO_COUNTRY is a code from element text (the override), not dropped.
    assert_eq!(code(&cn, "PROCEDURE", "TED-ISO_COUNTRY"), "FR");
    // ORIGINAL_CPV is a classification from bare text (r209's `@CODE` fallback).
    assert!(matches!(value(&cn, "PROCEDURE", "TED-ORIGINAL_CPV"),
        NoticeValue::Classification { scheme, code } if scheme == "cpv" && code == "21125400"));
    // Dispatch date (DATE_DISP) and the OJ publication date (REF_OJS/DATE_PUB).
    assert_eq!(
        *value(&cn, "PROCEDURE", "TED-DATE_DISP"),
        NoticeValue::Date { utc_seconds: 1_207_872_000, offset_minutes: 0, has_time: false }
    );
    assert_eq!(
        *value(&cn, "PROCEDURE", "TED-DATE_PUB"),
        NoticeValue::Date { utc_seconds: 1_209_686_400, offset_minutes: 0, has_time: false }
    );
    // The combined receipt deadline (`20080505 12:00`) is one instant.
    assert_eq!(
        *value(&cn, "PROCEDURE", "TED-DEADLINE_REC"),
        NoticeValue::Date { utc_seconds: 1_209_988_800, offset_minutes: 0, has_time: true }
    );
}

// ---------------------------------------------------- form-body reuse (_SUM)

/// The `_SUM` summary contract form maps through the r209 walker via the alias:
/// the buyer address block, the CPV, and the free-text description all land as
/// their base `TED-*` fields.
#[test]
fn sum_contract_form_reuses_the_r209_walker() {
    let cn = parse_fixture("internal_ojs/115165_2008.en");

    // FD_CONTRACT_SUM's contract-nature attribute rides on the aliased field.
    assert_eq!(code(&cn, "PROCEDURE", "TED-FD_CONTRACT.CTYPE"), "SUPPLIES");
    // The form declares its (summary) form number.
    assert_eq!(code(&cn, "PROCEDURE", "TED-FORM"), "2_SUM");

    // Buyer: an ORG section, role-referenced from the notice root under the
    // r209 role element name.
    let NoticeValue::Id { value: buyer, is_ref: true, .. } =
        value(&cn, "PROCEDURE", "TED-CA_CE_CONCESSIONAIRE_PROFILE")
    else {
        panic!("buyer role ref missing")
    };
    assert_eq!(text(&cn, buyer, "TED-ORGANISATION"), "Réunion des musées nationaux");
    assert_eq!(code(&cn, buyer, "TED-COUNTRY"), "FR");

    // CPV main, from the summary object description.
    assert!(matches!(value(&cn, "PROCEDURE", "TED-CPV_CODE"),
        NoticeValue::Classification { scheme, code } if scheme == "cpv" && code == "21125400"));
    // Description free text.
    assert_eq!(text(&cn, "PROCEDURE", "TED-DESCRIPTION_SUM"), "Printing paper.");
}

/// The `_SUM` award form: the synthesized award section (from the aliased
/// `AWARD_OF_CONTRACT_SUM ITEM`), the exact award value, and the chain edge to
/// the prior notice carried in the envelope's `REF_NOTICE`.
#[test]
fn sum_award_form_yields_result_section_value_and_chain_edge() {
    let award = parse_fixture("internal_ojs/114382_2008.en");

    // The chain edge to the corrected/prior notice: REF_NOTICE/NO_DOC_OJS is an
    // OJS reference, distinct from the notice's own NO_DOC_OJS.
    assert!(matches!(value(&award, "PROCEDURE", "TED-NO_DOC_OJS"),
        NoticeValue::Id { value, is_ref: false, .. } if value == "2008/S 85-114382"));
    assert!(matches!(value(&award, "PROCEDURE", "TED-REF_NOTICE.NO_DOC_OJS"),
        NoticeValue::Id { value, is_ref: true, scheme } if value == "2008/S 34-046827"
            && scheme.as_deref() == Some("ojs")));

    // The award block opened a synthesized LotResult section (RES-1) with its
    // published item number inside.
    let res = award.sections.iter().find(|s| s.kind == "LotResult").expect("award section");
    assert_eq!(res.id, "RES-1");
    assert!(matches!(value(&award, "RES-1", "TED-ITEM"), NoticeValue::Id { value, .. } if value == "1"));

    // The final contract value: space-grouped thousands + decimal comma →
    // exact integer cents, currency from the wrapping element.
    assert_eq!(
        *value(&award, "PROCEDURE", "TED-VALUE_COST"),
        NoticeValue::Amount { cents: 2_097_647_290, currency: "PLN".into() }
    );
}

// ------------------------------------------------ full (non-summary) form

/// A full CONTRACT form (no `_SUM`) reuses the r209 registry directly, and the
/// two same-name-different-shape traps are handled: `SERVICE_CATEGORY` carries
/// its code in `@VALUE`, and multiple bare-text `ORIGINAL_CPV` codes are each a
/// classification.
#[test]
fn full_contract_form_handles_the_shared_name_traps() {
    let cn = parse_fixture("internal_ojs/115908_2008.en");

    assert_eq!(code(&cn, "PROCEDURE", "TED-HEADING"), "3310");
    assert_eq!(code(&cn, "PROCEDURE", "TED-FORM"), "2");
    // SERVICE_CATEGORY's code moved to @VALUE (r209 reads it from text); the
    // overlay reads @VALUE so the code lands on the field itself, not a capture.
    assert_eq!(code(&cn, "PROCEDURE", "TED-SERVICE_CATEGORY"), "12");
    // Three ORIGINAL_CPV codes in the envelope, each a bare-text classification.
    let cpvs = values(&cn, "PROCEDURE", "TED-ORIGINAL_CPV");
    assert_eq!(cpvs.len(), 3);
    assert!(cpvs.iter().all(|v| matches!(v, NoticeValue::Classification { scheme, .. } if scheme == "cpv")));
    // The request-deadline-for-documents date (DEADLINE_REQ, bare date).
    assert_eq!(
        *value(&cn, "PROCEDURE", "TED-DEADLINE_REQ"),
        NoticeValue::Date { utc_seconds: 1_212_105_600, offset_minutes: 0, has_time: false }
    );
}

// -------------------------------------------------------- language policy

/// Only the English sibling is ingested; the ~21 other languages are the same
/// notice and dispatch skips them as documented duplicates. The non-English
/// payload still parses on its own (byte-exact), proving the skip is a policy
/// choice, not a parser limitation.
#[test]
fn non_english_siblings_are_skipped_but_still_parseable() {
    let bytes = std::fs::read("tests/fixtures/internal_ojs/115165_2008.fr").expect("fr fixture");
    let member = "20080502_2008085.tar.gz/115165/opoce-input/115165_2008.fr";
    assert!(matches!(
        profile::dispatch(member, &bytes),
        Disposition::Skipped("internal-ojs-non-english")
    ));

    // Fed to the parser directly, the French original (a full CONTRACT form) is
    // consumed exhaustively too.
    match parse_payload("internal-ojs", &bytes) {
        Parse::Parsed(parsed) => assert!(!parsed.values.is_empty()),
        other => panic!("fr original did not parse: {other:?}"),
    }
}

// -------------------------------------------------------------- completeness

/// The era-scoped ADR-0002 half this profile owns: every envelope/backbone
/// element it decides resolves to a rule (via its own overlay or, for the
/// shared-name overrides, the r209 registry). The `_SUM` alias table and the
/// 85% shared vocabulary are pinned by the r209 suite and the alias unit test.
#[test]
fn every_decided_envelope_element_resolves() {
    for name in ingest::internal_ojs::decided_names() {
        let via_overlay = ingest::internal_ojs::envelope_rule("", name).is_some();
        let via_r209 = ingest::r209::rules::rule("", name).is_some();
        assert!(via_overlay || via_r209, "{name} decided but has no rule");
    }
}
