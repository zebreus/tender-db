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
    assert_eq!(names.len(), 12, "corpus changed; update the expectation");

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

/// Issue 364: the four previous-publication citations of one real 2011 F03 mean
/// four different things, and the payload says which. `PREVIOUS_NOTICE_BUYER_PROFILE_F3
/// CHOICE="PRIOR_INFORMATION_NOTICE"` is the PIN the procurement was called under —
/// one publication many unrelated procurements cite, so not a same-procedure
/// predecessor; `CNT_NOTICE_INFORMATION_S CHOICE="CONTRACT_NOTICE"` in the SAME
/// notice is this award's own contract notice, and it is. The two remaining
/// citations sit in the form's `OTHER_PREVIOUS_PUBLICATIONS` slot, which declares
/// nothing at all — the 7.8 % undeclared class, refused by default because
/// defaulting to "edge" is what welded 2,983 versions into one Tender.
///
/// Corroboration that the gate keeps the RIGHT one: the coded data section's
/// `REF_NOTICE/NO_DOC_OJS` — TED's own per-procedure predecessor pick — names
/// `2010/S 133-203552`, the contract notice, and not the PIN.
#[test]
fn a_pin_citation_and_its_contract_notice_are_told_apart_by_their_declared_kind() {
    use ingest::r209::rules;

    let f03 = parse_fixture("r208/f03-annexd-neg-022211-2011.xml");
    let citations: Vec<(String, String)> = f03
        .values
        .iter()
        .filter(|v| v.field_id == "TED-NOTICE_NUMBER_OJ")
        .map(|v| {
            let number = match &v.value {
                NoticeValue::Id { value, is_ref: true, scheme }
                    if scheme.as_deref() == Some("ojs") =>
                {
                    value.clone()
                }
                other => panic!("a citation is not an OJS reference: {other:?}"),
            };
            let kind = f03
                .values
                .iter()
                .find(|k| {
                    k.section_id == v.section_id
                        && k.field_id == format!("TED-NOTICE_NUMBER_OJ.{}", rules::CITATION_KIND_SUFFIX)
                        && k.ordinal == v.ordinal
                })
                .map(|k| match &k.value {
                    NoticeValue::Code { code, .. } => code.clone(),
                    other => panic!("a citation kind is not a code: {other:?}"),
                })
                .expect("every citation carries its declared kind");
            (number, kind)
        })
        .collect();

    assert_eq!(
        citations,
        vec![
            ("2010/S 66-098284".to_owned(), "PRIOR_INFORMATION_NOTICE".to_owned()),
            ("2010/S 133-203552".to_owned(), "CONTRACT_NOTICE".to_owned()),
            ("2010/S 237-360907".to_owned(), rules::KIND_UNDECLARED.to_owned()),
            ("2010/S 197-299903".to_owned(), rules::KIND_UNDECLARED.to_owned()),
        ]
    );

    // Only the contract notice is a same-procedure predecessor — the identity
    // layer admits exactly that one as a chain edge.
    let admitted: Vec<&str> = citations
        .iter()
        .filter(|(_, kind)| rules::kind_is_same_procedure(kind))
        .map(|(number, _)| number.as_str())
        .collect();
    assert_eq!(admitted, vec!["2010/S 133-203552"]);

    // And it is the one TED's own coded-data-section predecessor names.
    assert!(matches!(
        value(&f03, "PROCEDURE", "TED-REF_NOTICE.NO_DOC_OJS"),
        NoticeValue::Id { value, is_ref: true, .. } if value == "2010/S 133-203552"
    ));
}

/// Issue 393 unit 1: the R2.0.8 `TRANSLATION_SECTION > TRANSLITERATIONS >
/// TRANSLITERATED_ADDR` block (099900-2018, a Greek F03) is TED's Latin rendering
/// of the buyer's own name and address. It is claimed whole and opens nothing:
/// no `TED-TRANSLITERATED_ADDR` reference, no Organization section carrying the
/// Latin spelling — while the Greek contracting body is still there.
#[test]
fn a_transliterated_address_block_is_claimed_and_opens_no_party() {
    let p = parse_fixture("r208/f03-099900-2018.xml");
    assert!(
        p.values.iter().all(|v| v.field_id != "TED-TRANSLITERATED_ADDR"),
        "the transliteration block must not be referenced as a party"
    );
    let names: Vec<&str> = p
        .values
        .iter()
        .filter(|v| v.field_id == "TED-OFFICIALNAME")
        .filter_map(|v| match &v.value {
            NoticeValue::Text { value, .. } => Some(value.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        !names.iter().any(|n| n.starts_with("Perifereia Attikis")),
        "no Organization section carries the Latin twin: {names:?}"
    );
    assert!(
        names.iter().any(|n| n.starts_with("Περιφέρεια Αττικής")),
        "the Greek contracting body is still a party: {names:?}"
    );
}

const FMTVAL_FIXTURE: &str = "r208/f03-fmtval-mismatch-222043-2011.xml";

fn parse_variant(edit: impl Fn(String) -> String) -> Parsed {
    let path = format!("tests/fixtures/{FMTVAL_FIXTURE}");
    let xml = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let bytes = edit(xml).into_bytes();
    let Disposition::Records(records) = profile::dispatch(FMTVAL_FIXTURE, &bytes) else {
        panic!("dispatch skipped the variant");
    };
    let [Record::Notice(notice)] = &records[..] else { panic!("expected one notice record") };
    assert_eq!(notice.profile, "ted-export-r208");
    match parse_payload(&notice.profile, &bytes) {
        Parse::Parsed(parsed) => parsed,
        other => panic!("variant did not parse: {other:?}"),
    }
}

/// `(section, field, ordinal) -> cents` of every `VALUE_COST` amount, and the
/// unadopted representations the parser filed beside them.
#[allow(clippy::type_complexity)]
fn value_costs(parsed: &Parsed) -> (Vec<((String, String, i64), i64)>, Vec<((String, String, i64), String)>) {
    let suffix = ingest::r209::value::FMTVAL_MISMATCH_SUFFIX;
    let mut amounts = Vec::new();
    let mut marks = Vec::new();
    for v in &parsed.values {
        if let Some(field) = v.field_id.strip_suffix(suffix) {
            let NoticeValue::Text { value, .. } = &v.value else { panic!("a mark is text: {v:?}") };
            marks.push(((v.section_id.clone(), field.to_owned(), v.ordinal), value.clone()));
        } else if v.field_id.ends_with("VALUE_COST") {
            let NoticeValue::Amount { cents, currency } = &v.value else { panic!("not an amount: {v:?}") };
            assert_eq!(currency, "EUR");
            amounts.push(((v.section_id.clone(), v.field_id.clone(), v.ordinal), *cents));
        }
    }
    (amounts, marks)
}

/// Issue 471 unit 3, the parse half, on the real member behind tender 4490098
/// (`2011-07-15.tar.gz/20110715_134/222043_2011.xml`, TED's July-2011
/// generator; the member is an `R2.0.7.S03.E01` form under the r208 profile): `<VALUE_COST FMTVAL="49700000000000000">49 700` stores 49,700.00
/// EUR — the text — not 4.97×10¹⁶, and files the attribute beside it as the
/// record of the correction. The member's other two value elements are 10²
/// off their texts (`FMTVAL="5000000">50 000`), the same defect.
#[test]
fn a_scaled_fmtval_yields_to_its_element_text_and_is_kept_beside_it() {
    let parsed = parse_fixture(FMTVAL_FIXTURE);
    let (amounts, marks) = value_costs(&parsed);
    assert!(!amounts.is_empty());
    assert!(
        !parsed.values.iter().any(|v| matches!(v.value, NoticeValue::Amount { cents, .. } if cents > 100_000_000)),
        "no figure above EUR 1,000,000.00 survives: the 10^12 and 10^2 attributes are overruled",
    );
    // Every VALUE_COST disagreed, so every one is paired with exactly one mark.
    assert_eq!(marks.len(), amounts.len(), "{marks:?}");
    for (key, raw) in &marks {
        let cents = amounts.iter().find(|(k, _)| k == key).map(|(_, c)| *c)
            .unwrap_or_else(|| panic!("mark {key:?} pairs with no amount"));
        let expected = match raw.as_str() {
            "49700000000000000" | "4970000" => 4_970_000,
            "5000000" => 5_000_000,
            other => panic!("unexpected unadopted representation {other:?}"),
        };
        assert_eq!(cents, expected, "{key:?}");
    }
    assert!(marks.iter().any(|(_, raw)| raw == "49700000000000000"), "the 10^12 attribute is kept as data");
    // The coded VALUES block (no FMTVAL) agrees with the adopted text.
    assert!(values(&parsed, "PROCEDURE", "TED-VALUE").iter().all(|v| matches!(v,
        NoticeValue::Amount { cents: 4_970_000, .. })));
}

/// An agreeing `@FMTVAL` reads exactly as before the check: same figure,
/// nothing filed beside it.
#[test]
fn an_agreeing_fmtval_is_unchanged_and_unmarked() {
    let agreeing = parse_variant(|xml| {
        xml.replace(r#"FMTVAL="49700000000000000">49 700"#, r#"FMTVAL="49700">49 700"#)
            .replace(r#"FMTVAL="4970000">49 700"#, r#"FMTVAL="49700">49 700"#)
            .replace(r#"FMTVAL="5000000">50 000"#, r#"FMTVAL="50000">50 000"#)
    });
    let (amounts, marks) = value_costs(&agreeing);
    assert!(marks.is_empty(), "{marks:?}");
    let (adopted, _) = value_costs(&parse_fixture(FMTVAL_FIXTURE));
    assert_eq!(amounts, adopted, "the adopted texts are what an agreeing attribute says");
    assert!(!agreeing.values.iter().any(|v| v.field_id.ends_with(ingest::r209::value::FMTVAL_MISMATCH_SUFFIX)));
}

/// A text whose decimal point is a guess (`49.700`: thousands, or mills?) is
/// not a number to check against: the attribute is read as before, unmarked.
#[test]
fn an_ambiguous_element_text_leaves_the_fmtval_as_it_was() {
    let ambiguous = parse_variant(|xml| {
        xml.replace(r#"FMTVAL="49700000000000000">49 700"#, r#"FMTVAL="49700000000000000">49.700"#)
    });
    let (amounts, marks) = value_costs(&ambiguous);
    let unchecked: Vec<_> = amounts.iter().filter(|(_, c)| *c == 4_970_000_000_000_000_000).collect();
    assert!(!unchecked.is_empty(), "the attribute is still read: {amounts:?}");
    for (key, _) in &unchecked {
        assert!(!marks.iter().any(|(k, _)| k == key), "an unchecked figure carries no mark: {key:?}");
    }
    // The other two elements still disagree unambiguously and are still marked.
    assert_eq!(marks.len(), amounts.len() - unchecked.len());
}

/// Every unit-3 beside row (`.FMTVAL_MISMATCH` or `.FMTVAL_TEXT`) of a parse,
/// as `(section, field, ordinal, suffix, raw)`.
fn beside_rows(parsed: &Parsed) -> Vec<(String, String, i64, &'static str, String)> {
    use ingest::r209::value::{FMTVAL_MISMATCH_SUFFIX, FMTVAL_TEXT_SUFFIX};
    let mut out = Vec::new();
    for v in &parsed.values {
        for suffix in [FMTVAL_MISMATCH_SUFFIX, FMTVAL_TEXT_SUFFIX] {
            if let Some(field) = v.field_id.strip_suffix(suffix) {
                let NoticeValue::Text { value, .. } = &v.value else { panic!("a beside row is text: {v:?}") };
                out.push((v.section_id.clone(), field.to_owned(), v.ordinal, suffix, value.clone()));
            }
        }
    }
    out
}

/// Review finding 1: a 10^12 attribute whose cents overflow `i64`
/// (`FMTVAL="100000000000000000">100 000` is 10^19 cents) is still compared,
/// in `i128`, so its correct text is adopted and the attribute kept beside it —
/// before the fix it fell through as a raw-text row and nothing was marked.
#[test]
fn a_scaled_fmtval_too_large_for_the_stored_integer_still_yields_to_its_text() {
    let overflowing = parse_variant(|xml| {
        xml.replace(r#"FMTVAL="49700000000000000">49 700"#, r#"FMTVAL="100000000000000000">100 000"#)
    });
    let (amounts, marks) = value_costs(&overflowing);
    let adopted: Vec<_> = amounts.iter().filter(|(_, c)| *c == 10_000_000).collect();
    assert!(!adopted.is_empty(), "the text 100 000 is the stored figure: {amounts:?}");
    for (key, _) in &adopted {
        assert!(
            marks.iter().any(|(k, raw)| k == key && raw == "100000000000000000"),
            "the overflowing attribute is kept beside its amount: {key:?} {marks:?}"
        );
    }
    assert!(
        !overflowing.values.iter().any(|v| matches!(&v.value,
            NoticeValue::Text { value, .. } if value == "100000000000000000")
            && !v.field_id.ends_with(ingest::r209::value::FMTVAL_MISMATCH_SUFFIX)),
        "no raw-text fallback row for the attribute"
    );
}

/// Review findings 2, 5, 7, 8: a disagreement OUTSIDE the measured shape (here
/// the attribute 10^1 BELOW its text) keeps the attribute exactly as before the
/// check — no rescale, no mark — and only files the text beside it as
/// `.FMTVAL_TEXT`, so the class is countable in the parsed layer.
#[test]
fn a_disagreement_outside_the_measured_shape_keeps_the_attribute_and_files_the_text() {
    let disagreeing = parse_variant(|xml| {
        xml.replace(r#"FMTVAL="49700000000000000">49 700"#, r#"FMTVAL="4970">49 700"#)
    });
    let (amounts, marks) = value_costs(&disagreeing);
    let kept: Vec<_> = amounts.iter().filter(|(_, c)| *c == 497_000).collect();
    assert!(!kept.is_empty(), "the attribute 4,970.00 is still the stored figure: {amounts:?}");
    let beside = beside_rows(&disagreeing);
    for ((section, field, ordinal), _) in &kept {
        assert!(!marks.iter().any(|((s, f, o), _)| (s, f, o) == (section, field, ordinal)), "no mark");
        assert!(
            beside.iter().any(|(s, f, o, suffix, raw)| (s, f, o) == (section, field, ordinal)
                && *suffix == ingest::r209::value::FMTVAL_TEXT_SUFFIX && raw == "49 700"),
            "the disagreeing text is kept beside {section}/{field}#{ordinal}: {beside:?}"
        );
    }
}

/// No currency in scope: there is no amount to rescale or mark. The raw
/// attribute is the text row it always was, and the text is filed beside it.
#[test]
fn a_scaled_fmtval_with_no_currency_in_scope_stays_raw_text() {
    let uncurrencied = parse_variant(|xml| {
        xml.replace(
            r#"<COSTS_RANGE_AND_CURRENCY_WITH_VAT_RATE CURRENCY="EUR"><VALUE_COST FMTVAL="49700000000000000">"#,
            r#"<COSTS_RANGE_AND_CURRENCY_WITH_VAT_RATE><VALUE_COST FMTVAL="49700000000000000">"#,
        )
    });
    let raw: Vec<_> = uncurrencied
        .values
        .iter()
        .filter(|v| v.field_id.ends_with("VALUE_COST")
            && matches!(&v.value, NoticeValue::Text { value, .. } if value == "49700000000000000"))
        .collect();
    assert!(!raw.is_empty(), "the attribute is kept as raw text, as before the check");
    let beside = beside_rows(&uncurrencied);
    for v in &raw {
        assert!(
            beside.iter().any(|(s, f, o, suffix, text)| s == &v.section_id && f == &v.field_id
                && *o == v.ordinal && *suffix == ingest::r209::value::FMTVAL_TEXT_SUFFIX && text == "49 700"),
            "{v:?}: {beside:?}"
        );
    }
    assert!(
        !beside.iter().any(|(s, f, o, suffix, _)| *suffix == ingest::r209::value::FMTVAL_MISMATCH_SUFFIX
            && raw.iter().any(|v| (&v.section_id, &v.field_id, &v.ordinal) == (s, f, o))),
        "nothing to mark without an amount"
    );
}

/// Review finding 9: the committed r208 members that carry `@FMTVAL` and are
/// NOT the July-2011 exhibit agree with their texts (or have none to check),
/// so the check files nothing beside any of their amounts.
#[test]
fn no_other_committed_r208_fmtval_fixture_files_anything_beside_its_amounts() {
    for relative in [
        "r208/f02-000333-2014.xml",
        "r208/f03-099900-2018.xml",
        "r208/f03-annexd-neg-022211-2011.xml",
        "r208/f13-187010-2013.xml",
    ] {
        let parsed = parse_fixture(relative);
        assert!(
            parsed.values.iter().any(|v| matches!(v.value, NoticeValue::Amount { .. })),
            "{relative}: the fixture must still carry amounts for this to mean anything"
        );
        assert_eq!(beside_rows(&parsed), vec![], "{relative}");
    }
}
