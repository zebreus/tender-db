//! `ted-export-r208` profile tests, driven by the committed real-notice
//! corpus: the 2014 F02 (R2.0.8.S02), the 2011 F02 (R2.0.7 — no XSD ever
//! published; absorbed empirically per docs/architecture.md), and a 2014
//! OTH_NOT whose prose body is mapped as declared text.
//!
//! The completeness harness (rules<->inventory both ways) lives in r209.rs:
//! one registry and one vendored inventory cover the whole TED_EXPORT family.

use ingest::process::parse_payload;
use ingest::profile::{self, Disposition, Record};
use store::{NoticeValue, Parse, Parsed};

fn ingest_fixture(relative: &str) -> (String, Parse) {
    let path = format!("tests/fixtures/{relative}");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let Disposition::Records(records) = profile::dispatch(relative, &bytes) else {
        panic!("{relative}: dispatch skipped a TED_EXPORT fixture");
    };
    let [Record::Notice(notice)] = &records[..] else {
        panic!("{relative}: expected exactly one notice record");
    };
    (notice.profile.clone(), parse_payload(&notice.profile, &bytes))
}

fn parse_fixture(relative: &str) -> Parsed {
    let (profile, parse) = ingest_fixture(relative);
    assert_eq!(profile, "ted-export-r208", "{relative}: wrong profile");
    match parse {
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

/// ADR-0004 on real data: every committed R2.0.8-era fixture is consumed
/// exhaustively — no quarantine, no pending, no dangling section refs.
#[test]
fn every_r208_fixture_is_consumed_exhaustively() {
    let mut names: Vec<String> = std::fs::read_dir("tests/fixtures/r208")
        .expect("fixture directory")
        .filter_map(|e| {
            let name = e.ok()?.file_name().to_string_lossy().into_owned();
            name.ends_with(".xml").then(|| format!("r208/{name}"))
        })
        .collect();
    names.sort();
    assert_eq!(names.len(), 5, "corpus changed; update the expectation");

    for relative in names {
        let parsed = parse_fixture(&relative);
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

/// The 2014 F02 (`CONTRACT` root, R2.0.8.S02 grammar), asserted in depth:
/// identity, buyer, values, the D/M/Y+time deadline, and the coded backbone.
#[test]
fn r208_contract_notice_extracts_its_business_content() {
    let cn = parse_fixture("r208/f02-000333-2014.xml");

    // Identity and OP-curated codes: same coded backbone as every era.
    assert!(matches!(value(&cn, "PROCEDURE", "TED-NO_DOC_OJS"),
        NoticeValue::Id { value, is_ref: false, .. } if value == "2014/S 001-000333"));
    assert!(matches!(value(&cn, "PROCEDURE", "TED-TD_DOCUMENT_TYPE"),
        NoticeValue::Code { code, .. } if code == "3"));
    assert!(matches!(value(&cn, "PROCEDURE", "TED-FORM"),
        NoticeValue::Code { code, .. } if code == "2"));

    // Buyer: the R2.0.8 address block is CA_CE_CONCESSIONAIRE_PROFILE, an
    // ORG section role-referenced from the notice root; the name sits in the
    // nested ORGANISATION/OFFICIALNAME (2014 grammar).
    let NoticeValue::Id { value: buyer, is_ref: true, .. } =
        value(&cn, "PROCEDURE", "TED-CA_CE_CONCESSIONAIRE_PROFILE")
    else {
        panic!("buyer role ref missing")
    };
    assert_eq!(text(&cn, buyer, "TED-OFFICIALNAME"), "Kingstown Works Limited");
    assert!(matches!(value(&cn, buyer, "TED-COUNTRY"),
        NoticeValue::Code { code, .. } if code == "UK"));

    // Money: the coded GLOBAL value ("900 000", display-formatted) and the
    // in-form framework estimate (FMTVAL machine value under the
    // TOTAL_ESTIMATED wrapper) agree, in integer cents.
    let estimate = NoticeValue::Amount { cents: 90_000_000, currency: "GBP".into() };
    assert_eq!(*value(&cn, "PROCEDURE", "TED-VALUE"), estimate);
    assert_eq!(*value(&cn, "PROCEDURE", "TED-TOTAL_ESTIMATED.VALUE_COST"), estimate);

    // Deadline: the D/M/Y+TIME split date is one instant, agreeing with the
    // coded DT_DATE_FOR_SUBMISSION.
    let deadline = NoticeValue::Date { utc_seconds: 1_391_792_400, offset_minutes: 0, has_time: true };
    assert_eq!(*value(&cn, "PROCEDURE", "TED-RECEIPT_LIMIT_DATE"), deadline);
    assert_eq!(*value(&cn, "PROCEDURE", "TED-DT_DATE_FOR_SUBMISSION"), deadline);

    // Procedure/boolean grammar: marker elements and captured qualifiers.
    assert_eq!(*value(&cn, "PROCEDURE", "TED-PT_RESTRICTED"), NoticeValue::Integer(1));
    assert_eq!(*value(&cn, "PROCEDURE", "TED-DIV_INTO_LOT_NO"), NoticeValue::Integer(1));
    assert_eq!(*value(&cn, "PROCEDURE", "TED-CONTRACT_COVERED_GPA"), NoticeValue::Integer(1));
    assert!(matches!(value(&cn, "PROCEDURE", "TED-CONTRACT_COVERED_GPA.VALUE"),
        NoticeValue::Code { code, .. } if code == "YES"));
    // CPV main + three additional, all lotless → on the notice root.
    let cpvs = values(&cn, "PROCEDURE", "TED-CPV_CODE");
    assert_eq!(cpvs.len(), 4);
    assert!(matches!(cpvs[0],
        NoticeValue::Classification { scheme, code } if scheme == "cpv" && code == "34928530"));
}

/// The 2011 F02 is R2.0.7 (`VERSION="R2.0.7.S03.E01"`, no XSD on the OP
/// archive): the delta the era actually publishes — inline ORGANISATION
/// text, `@VALUE` variants of later marker children — parses under the same
/// registry.
#[test]
fn r207_delta_is_absorbed_empirically() {
    let cn = parse_fixture("r208/f02-r207-001441-2011.xml");

    // 2011 publishes the buyer name as ORGANISATION text (no OFFICIALNAME).
    let NoticeValue::Id { value: buyer, is_ref: true, .. } =
        value(&cn, "PROCEDURE", "TED-CA_CE_CONCESSIONAIRE_PROFILE")
    else {
        panic!("buyer role ref missing")
    };
    assert_eq!(
        text(&cn, buyer, "TED-ORGANISATION"),
        "Southend University Hospital NHS Foundation Trust"
    );

    // 2011 puts codes in @VALUE where 2014 uses child elements.
    assert!(matches!(value(&cn, "PROCEDURE", "TED-NOTICE_INVOLVES"),
        NoticeValue::Code { code, .. } if code == "ESTABLISHMENT_FRAMEWORK_AGREEMENT"));
    assert!(matches!(value(&cn, "PROCEDURE", "TED-PURCHASING_ON_BEHALF.VALUE"),
        NoticeValue::Code { code, .. } if code == "NO"));

    // Annex-B lot blocks synthesize LOT sections in document order.
    let lots: Vec<_> = cn.sections.iter().filter(|s| s.kind == "Lot").collect();
    assert_eq!(lots.len(), 3);
    assert!(matches!(value(&cn, "LOT-2", "TED-LOT_NUMBER"),
        NoticeValue::Id { value, .. } if value == "2"));
    assert_eq!(text(&cn, "LOT-1", "TED-LOT_TITLE"), "Electrical goods and supplies");

    // Weighted award criteria: six CRITERIA_DEFINITION rows of free text.
    assert_eq!(values(&cn, "PROCEDURE", "TED-CRITERIA").len(), 6);
    assert_eq!(values(&cn, "PROCEDURE", "TED-WEIGHTING").len(), 6);

    // The D/M/Y deadline without a published time stays date-only.
    assert_eq!(
        *value(&cn, "PROCEDURE", "TED-RECEIPT_LIMIT_DATE"),
        NoticeValue::Date { utc_seconds: 1_296_777_600, offset_minutes: 0, has_time: false }
    );
}

/// Issue 31: a 2011 F03 award whose ANNEX_D justifies a negotiated procedure
/// without competition. The choice element carries the reason on `@REASON`
/// (`PURCHASE_SUPPLIES_ADVANTAGEOUS_TERMS REASON="SUPPLIER_WINDING_UP_BUSINESS"`),
/// which used to be an unclaimed attribute and quarantined the whole award; it
/// is now claimed as the annex-D justification code.
#[test]
fn f03_annex_d_negotiated_reason_is_claimed() {
    let award = parse_fixture("r208/f03-annexd-neg-022211-2011.xml");
    assert!(
        matches!(
            value(&award, "PROCEDURE", "TED-PURCHASE_SUPPLIES_ADVANTAGEOUS_TERMS.REASON"),
            NoticeValue::Code { code, .. } if code == "SUPPLIER_WINDING_UP_BUSINESS"
        ),
        "the negotiated-procedure justification reason must be claimed as a code",
    );
}

/// OTH_NOT is the era's catch-all corrigendum: coded header + chain edge +
/// prose body mapped as declared text (a version event whose diff is prose —
/// the typed F14 diffs do not exist in this era).
#[test]
fn oth_not_yields_chain_edge_and_prose_body() {
    let f = parse_fixture("r208/oth-not-000030-2014.xml");

    // The OP-curated chain edge, and the corrigendum document-type code.
    assert!(matches!(value(&f, "PROCEDURE", "TED-REF_NOTICE.NO_DOC_OJS"),
        NoticeValue::Id { value, is_ref: true, .. } if value == "2013/S 223-387629"));
    assert!(matches!(value(&f, "PROCEDURE", "TED-TD_DOCUMENT_TYPE"),
        NoticeValue::Code { code, .. } if code == "2"));

    // The body is declared text in the original language (PT) plus every
    // translation copy the notice carries — this 2014 corrigendum ships all
    // 24, and under the flipped dispatch default (issue 304 stage 1,
    // 2026-09-02) they all land, each labelled. Nothing structured, nothing
    // dropped, nothing unlabelled. Before the flip this pinned `["EN", "PT"]`;
    // that shape is now the explicit opt-out, asserted below.
    let langs_of = |contents: &[&NoticeValue]| -> Vec<String> {
        contents
            .iter()
            .map(|v| match v {
                NoticeValue::Text { lang, .. } => {
                    lang.clone().expect("every prose copy carries its language")
                }
                other => panic!("CONTENTS is not text: {other:?}"),
            })
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    };
    let contents = values(&f, "PROCEDURE", "TED-CONTENTS");
    assert!(!contents.is_empty(), "prose body missing");
    let langs = langs_of(&contents);
    assert!(langs.iter().any(|l| l == "PT"), "the PT original is missing: {langs:?}");
    assert!(langs.iter().any(|l| l == "EN"), "the EN translation is missing: {langs:?}");
    assert!(
        langs.len() > 2,
        "the default policy must keep the corrigendum's other translation copies too — \
         only {langs:?} landed, which is v1's EnOnly shape"
    );
    assert!(
        contents.iter().any(|v| matches!(v,
            NoticeValue::Text { lang, value } if lang.as_deref() == Some("EN")
                && value.contains("Instead of"))),
        "for/read prose not captured: {contents:?}"
    );

    // The explicit opt-out still parses v1's way: original + English only.
    let bytes = std::fs::read("tests/fixtures/r208/oth-not-000030-2014.xml").unwrap();
    let en_only = match ingest::r209::parse_payload(
        "ted-export-r208",
        &bytes,
        ingest::r209::TranslationPolicy::EnOnly,
    ) {
        Parse::Parsed(p) => p,
        other => panic!("EnOnly parse: {other:?}"),
    };
    assert_eq!(
        langs_of(&values(&en_only, "PROCEDURE", "TED-CONTENTS")),
        ["EN", "PT"],
        "EnOnly stays available and keeps exactly the original plus the English copy"
    );

    // The structured island the prose body allows: the header CPV.
    assert!(matches!(value(&f, "PROCEDURE", "TED-CPV_CODE"),
        NoticeValue::Classification { scheme, code } if scheme == "cpv" && code == "45000000"));
}

/// Issue 139: the 2010-03-10 converted daily ships its 1,898 TED_EXPORT
/// members behind an inline `<!DOCTYPE TED_EXPORT SYSTEM "TED_EXPORT.dtd">`.
/// The dispatcher strips it to reach the root, so the member passes dispatch
/// as a notice — the deep parse must strip the same way, or the member
/// re-quarantines as unparsable-xml for a declaration the parser never
/// needed. Prepending the era's exact DOCTYPE to a committed fixture must
/// change nothing about its parse.
#[test]
fn notice_behind_a_doctype_parses_identically() {
    let relative = "r208/f02-000333-2014.xml";
    let path = format!("tests/fixtures/{relative}");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let xml = std::str::from_utf8(&bytes).unwrap();
    let at = xml.find("?>").map(|i| i + 2).unwrap_or(0);
    let doctyped =
        format!("{}<!DOCTYPE TED_EXPORT SYSTEM \"TED_EXPORT.dtd\">{}", &xml[..at], &xml[at..]);

    let Disposition::Records(records) = profile::dispatch(relative, doctyped.as_bytes()) else {
        panic!("doctyped member skipped at dispatch");
    };
    let [Record::Notice(notice)] = &records[..] else {
        panic!("doctyped member did not dispatch as one notice");
    };

    let Parse::Parsed(with_dtd) = parse_payload(&notice.profile, doctyped.as_bytes()) else {
        panic!("doctyped member failed the deep parse (issue 139 regression)");
    };
    let Parse::Parsed(plain) = parse_payload(&notice.profile, &bytes) else {
        panic!("fixture must parse without the DOCTYPE");
    };
    assert_eq!(with_dtd.values, plain.values, "the strip must be lossless");
    assert_eq!(with_dtd.sections.len(), plain.sections.len());
}
