//! `ted-export-r209` profile tests, driven by the committed real-notice
//! corpus (F02, F03, F05, F14, F20 and a defence F18 from the 2019-01-02
//! daily package).
//!
//! Two guarantees, mirroring the eForms suite. ADR-0004: every element,
//! attribute and text node of a real notice is claimed or the notice
//! quarantines whole. ADR-0002 (era-scoped): every element and attribute of
//! the vendored XSD inventory has a rule or a documented exclusion.

use ingest::process::parse_payload;
use ingest::profile::{self, Disposition, Record};
use ingest::r209::{self, rules};
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

fn text(parsed: &Parsed, section: &str, field: &str) -> String {
    match value(parsed, section, field) {
        NoticeValue::Text { value, .. } => value.clone(),
        other => panic!("{field} is not text: {other:?}"),
    }
}

// ------------------------------------------------------------ exhaustiveness

/// ADR-0004 on real data: nothing in a published R2.0.9-era notice goes
/// unclaimed — the standard forms and the R2.0.8-grammar defence form alike.
#[test]
fn every_r209_fixture_is_consumed_exhaustively() {
    let mut names: Vec<String> = std::fs::read_dir("tests/fixtures/r209")
        .expect("fixture directory")
        .filter_map(|e| {
            let name = e.ok()?.file_name().to_string_lossy().into_owned();
            name.ends_with(".xml").then(|| format!("r209/{name}"))
        })
        .collect();
    names.sort();
    assert_eq!(names.len(), 6, "corpus changed; update the expectation");

    for relative in names {
        let (profile, parse) = ingest_fixture(&relative);
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

/// The defence F18 is namespaced R2.0.8 and dispatches to that profile, but
/// its grammar lives here (defence forms never migrated); the fixture must
/// parse — not stay pending for issue 10.
#[test]
fn defence_forms_parse_under_the_r208_profile_id() {
    let (profile, parse) = ingest_fixture("r209/f18-defence-001420-2019.xml");
    assert_eq!(profile, "ted-export-r208");
    let parsed = match parse {
        Parse::Parsed(parsed) => parsed,
        other => panic!("defence F18: {other:?}"),
    };
    // Winner and buyer are organization mentions with the wrapper's role.
    let award = parsed.sections.iter().find(|s| s.kind == "LotResult").expect("award section");
    let NoticeValue::Id { value: winner_org, is_ref: true, .. } =
        value(&parsed, &award.id, "TED-ECONOMIC_OPERATOR_NAME_ADDRESS")
    else {
        panic!("winner role ref missing")
    };
    assert_eq!(text(&parsed, winner_org, "TED-OFFICIALNAME"), "Indaeltrac");
    // The defence D/M/Y split date became one instant.
    assert_eq!(
        *value(&parsed, &award.id, "TED-CONTRACT_AWARD_DATE"),
        NoticeValue::Date { utc_seconds: 1_544_745_600, offset_minutes: 0, has_time: false }
    );
    // FMTVAL machine values, currency from the wrapping element; the initial
    // estimate keeps its wrapper prefix, the final value is the plain field.
    assert_eq!(
        *value(&parsed, &award.id, "TED-VALUE_COST"),
        NoticeValue::Amount { cents: 168_110_000, currency: "RON".into() }
    );
    assert_eq!(
        *value(&parsed, &award.id, "TED-INITIAL_ESTIMATED_TOTAL_VALUE_CONTRACT.VALUE_COST"),
        NoticeValue::Amount { cents: 216_263_019, currency: "RON".into() }
    );
}

// ------------------------------------------------------------- value mapping

/// F02 contract notice, asserted in depth: title, buyer, CPV, deadline, and
/// the coded backbone every era shares.
#[test]
fn contract_notice_extracts_its_business_content() {
    let cn = parse_fixture("r209/f02-000245-2019.xml");

    // Identity and the OP-curated codes.
    assert!(matches!(value(&cn, "PROCEDURE", "TED-NO_DOC_OJS"),
        NoticeValue::Id { value, is_ref: false, .. } if value == "2019/S 001-000245"));
    assert!(matches!(value(&cn, "PROCEDURE", "TED-TD_DOCUMENT_TYPE"),
        NoticeValue::Code { code, .. } if code == "3"));
    assert!(matches!(value(&cn, "PROCEDURE", "TED-FORM"),
        NoticeValue::Code { code, .. } if code == "F02"));

    // Title in the original language.
    assert!(matches!(value(&cn, "PROCEDURE", "TED-TITLE"),
        NoticeValue::Text { lang, value } if lang.as_deref() == Some("EN")
            && value.starts_with("Upgrading of Interpretation Centre")));

    // Buyer: an ORG section, role-referenced from the notice root.
    let NoticeValue::Id { value: buyer, is_ref: true, .. } =
        value(&cn, "PROCEDURE", "TED-ADDRESS_CONTRACTING_BODY")
    else {
        panic!("buyer role ref missing")
    };
    assert_eq!(text(&cn, buyer, "TED-OFFICIALNAME"), "Mellieha Local Council");
    assert!(matches!(value(&cn, buyer, "TED-COUNTRY"),
        NoticeValue::Code { code, .. } if code == "MT"));
    assert!(matches!(value(&cn, buyer, "TED-NUTS"),
        NoticeValue::Classification { scheme, code } if scheme == "nuts" && code == "MT"));

    // CPV: main on the procedure, additional on the synthesized lot.
    assert!(matches!(value(&cn, "PROCEDURE", "TED-CPV_CODE"),
        NoticeValue::Classification { scheme, code } if scheme == "cpv" && code == "32321200"));
    assert_eq!(values(&cn, "LOT-1", "TED-CPV_CODE").len(), 7);

    // Submission deadline: the paired date+time pair is one instant, and the
    // coded section's DT_DATE_FOR_SUBMISSION agrees with it.
    let deadline = NoticeValue::Date { utc_seconds: 1_549_022_400, offset_minutes: 0, has_time: true };
    assert_eq!(*value(&cn, "PROCEDURE", "TED-DATE_RECEIPT_TENDERS"), deadline);
    assert_eq!(*value(&cn, "PROCEDURE", "TED-DT_DATE_FOR_SUBMISSION"), deadline);

    // Procedure type is element presence; duration keeps its unit.
    assert_eq!(*value(&cn, "PROCEDURE", "TED-PT_OPEN"), NoticeValue::Integer(1));
    assert_eq!(
        *value(&cn, "LOT-1", "TED-DURATION"),
        NoticeValue::Number { value: 2.0, unit: Some("MONTH".into()) }
    );
    assert_eq!(*value(&cn, "PROCEDURE", "TED-NO_LOT_DIVISION"), NoticeValue::Integer(1));
}

/// F03 award notice: winner, award value in exact cents, conclusion date.
#[test]
fn award_notice_extracts_winner_and_exact_award_value() {
    let can = parse_fixture("r209/f03-000988-2019.xml");

    let award = can.sections.iter().find(|s| s.kind == "LotResult").expect("award section");
    assert_eq!(award.id, "RES-1");

    // Winner: contractor org mention, role-referenced from the award block.
    let NoticeValue::Id { value: winner, is_ref: true, .. } =
        value(&can, "RES-1", "TED-ADDRESS_CONTRACTOR")
    else {
        panic!("winner role ref missing")
    };
    assert_eq!(text(&can, winner, "TED-OFFICIALNAME"), "Liberty Healthcare");
    // SME sits beside (not inside) the contractor address block; it lands on
    // the award section, ordinal-aligned with the contractor role refs.
    assert_eq!(*value(&can, "RES-1", "TED-SME"), NoticeValue::Integer(1));

    // Money is integer cents + currency, at award and procedure level.
    assert_eq!(
        *value(&can, "RES-1", "TED-VAL_TOTAL"),
        NoticeValue::Amount { cents: 1_326_000, currency: "GBP".into() }
    );
    assert_eq!(
        *value(&can, "PROCEDURE", "TED-VAL_TOTAL"),
        NoticeValue::Amount { cents: 1_326_000, currency: "GBP".into() }
    );

    // Conclusion date and bid statistics.
    assert_eq!(
        *value(&can, "RES-1", "TED-DATE_CONCLUSION_CONTRACT"),
        NoticeValue::Date { utc_seconds: 1_545_955_200, offset_minutes: 0, has_time: false }
    );
    assert_eq!(*value(&can, "RES-1", "TED-NB_TENDERS_RECEIVED"), NoticeValue::Integer(4));
}

/// F14 corrigendum: the REF_NOTICE chain edge and typed old/new change pairs.
#[test]
fn corrigendum_yields_chain_edge_and_typed_changes() {
    let f14 = parse_fixture("r209/f14-001311-2019.xml");

    // Both reference carriers point at the corrected notice (research §3:
    // coded REF_NOTICE primary, in-form NOTICE_NUMBER_OJ corroboration).
    assert!(matches!(value(&f14, "PROCEDURE", "TED-NO_DOC_OJS"),
        NoticeValue::Id { value, is_ref: false, .. } if value == "2019/S 001-001311"));
    let refs: Vec<_> = f14
        .values
        .iter()
        .filter(|v| matches!(&v.value, NoticeValue::Id { is_ref: true, scheme, .. } if scheme.as_deref() == Some("ojs")))
        .collect();
    assert_eq!(refs.len(), 2, "coded + in-form reference: {refs:?}");
    for r in refs {
        assert!(matches!(&r.value, NoticeValue::Id { value, .. } if value == "2018/S 237-542350"));
    }

    // Two change blocks, each with a machine-applicable date move: the typed
    // old/new pair stays distinguishable via the wrapper-prefixed field ids.
    let changes: Vec<_> = f14.sections.iter().filter(|s| s.kind == "Change").collect();
    assert_eq!(changes.len(), 2);
    assert_eq!(
        *value(&f14, "CHG-1", "TED-OLD_VALUE.DATE"),
        NoticeValue::Date { utc_seconds: 1_547_208_000, offset_minutes: 0, has_time: true }
    );
    assert_eq!(
        *value(&f14, "CHG-1", "TED-NEW_VALUE.DATE"),
        NoticeValue::Date { utc_seconds: 1_548_417_600, offset_minutes: 0, has_time: true }
    );
    assert!(text(&f14, "CHG-1", "TED-SECTION").starts_with("IV.2.2"));
}

/// F20 modification: before/after values in the modification section.
#[test]
fn modification_notice_extracts_before_and_after_values() {
    let f20 = parse_fixture("r209/f20-000591-2019.xml");
    let modification = f20.sections.iter().find(|s| s.kind == "Modification").expect("MOD section");
    assert_eq!(modification.id, "MOD-1");
    let amount = NoticeValue::Amount { cents: 32_250_000, currency: "GBP".into() };
    assert_eq!(*value(&f20, "MOD-1", "TED-INFO_MODIFICATIONS.VAL_TOTAL_BEFORE"), amount);
    assert_eq!(*value(&f20, "MOD-1", "TED-INFO_MODIFICATIONS.VAL_TOTAL_AFTER"), amount);
    // The award block the modification restates chains by CONTRACT_NO.
    assert!(matches!(value(&f20, "RES-1", "TED-CONTRACT_NO"),
        NoticeValue::Id { value, .. } if value == "30369"));
}

/// Multilingual policy: the 24-language ML_TITLES keep exactly EN + the
/// original language; a NATIONALID is stored raw for the projection's gate.
#[test]
fn language_policy_and_national_ids() {
    let f05 = parse_fixture("r209/f05-001315-2019.xml");
    // Original language is NL: EN + NL titles kept, 22 others skipped.
    let titles = values(&f05, "PROCEDURE", "TED-TI_TEXT");
    let langs: Vec<_> = titles
        .iter()
        .map(|v| match v {
            NoticeValue::Text { lang, .. } => lang.as_deref().unwrap_or_default(),
            other => panic!("not text: {other:?}"),
        })
        .collect();
    assert_eq!(langs, ["EN", "NL"]);

    // The Belgian buyer's NATIONALID: raw here; normalized by orgid.
    let NoticeValue::Id { value: buyer, .. } = value(&f05, "PROCEDURE", "TED-ADDRESS_CONTRACTING_BODY")
    else {
        panic!("buyer ref")
    };
    let NoticeValue::Id { scheme, value: raw, is_ref: false } = value(&f05, buyer, "TED-NATIONALID")
    else {
        panic!("national id missing")
    };
    assert_eq!(scheme.as_deref(), Some("national"));
    assert_eq!(raw, "0242.069.537_22553");
    assert!(ingest::orgid::normalize(raw).is_some());

    // Two lots, in document order, with their published numbers inside.
    assert!(matches!(value(&f05, "LOT-1", "TED-LOT_NO"), NoticeValue::Id { value, .. } if value == "1"));
    assert!(matches!(value(&f05, "LOT-2", "TED-LOT_NO"), NoticeValue::Id { value, .. } if value == "2"));
}

// -------------------------------------------------------------- completeness

/// The era-scoped ADR-0002 harness: every element of the vendored XSD
/// inventory has a rule (mapping or documented exclusion), and every rule
/// names an element the XSDs declare — no drift in either direction.
#[test]
fn every_inventory_element_has_a_rule_and_vice_versa() {
    let inventory = inventory();
    let missing: Vec<&str> = inventory
        .iter()
        .filter(|e| rules::rule("", &e.name).is_none())
        .map(|e| e.name.as_str())
        .collect();
    assert!(missing.is_empty(), "{} inventory elements without a rule: {missing:?}", missing.len());

    let names: std::collections::HashSet<&str> = inventory.iter().map(|e| e.name.as_str()).collect();
    let stray: Vec<&str> = rules::decided_names().filter(|n| !names.contains(n)).collect();
    assert!(stray.is_empty(), "rules for elements no mirrored XSD declares: {stray:?}");

    assert!(inventory.len() > 1300, "inventory suspiciously small: {}", inventory.len());
}

/// Every attribute the XSDs declare is claimed: consumed by its element's
/// rule, a global context attribute, a captured qualifier, or inside an
/// ignored subtree. This is the attribute half of "mapped-or-ignored".
#[test]
fn every_inventory_attribute_is_claimed() {
    use rules::{Rule, Unit};
    let unclaimed: Vec<String> = inventory()
        .iter()
        .flat_map(|e| e.attributes.iter().map(move |a| (e, a)))
        .filter(|(e, attr)| {
            let attr = attr.as_str();
            // The root and the form copies are handled by dedicated code.
            if e.name == "TED_EXPORT" {
                return !matches!(attr, "DOC_ID" | "EDITION" | "VERSION");
            }
            let rule = rules::rule("", &e.name).expect("checked above");
            if rule == Rule::FormRoot {
                return !matches!(attr, "CATEGORY" | "FORM" | "LG" | "VERSION");
            }
            let consumed = match rule {
                Rule::Ignore(_) | Rule::Text => true,
                Rule::CodeAttr(attrs) => attrs.contains(&attr),
                Rule::Cpv | Rule::Nuts => attr == "CODE",
                Rule::Amount => attr == "FMTVAL",
                Rule::Number(Unit::FromTypeAttr) => matches!(attr, "TYPE" | "FMTVAL"),
                Rule::Number(_) => attr == "FMTVAL",
                Rule::Section(_) => matches!(attr, "ITEM" | "FMTVAL"),
                _ => false,
            };
            !(consumed
                || matches!(attr, "LG" | "CURRENCY")
                || [
                    "PUBLICATION", "TYPE", "VALUE", "CTYPE", "CHOICE", "CLASS", "LAST", "FORMAT",
                    "ITEM", "PROCEDURE", "STATUS", "OBJECT", "SERVICES_CATEGORY",
                ]
                .contains(&attr))
        })
        .map(|(e, attr)| format!("{}/@{attr}", e.name))
        .collect();
    assert!(unclaimed.is_empty(), "unclaimed inventory attributes: {unclaimed:?}");
}

struct InventoryElement {
    name: String,
    attributes: Vec<String>,
}

fn inventory() -> Vec<InventoryElement> {
    let json: serde_json::Value = serde_json::from_str(r209::INVENTORY_JSON).expect("vendored inventory");
    json["elements"]
        .as_array()
        .expect("elements array")
        .iter()
        .map(|e| InventoryElement {
            name: e["name"].as_str().expect("name").to_owned(),
            attributes: e["attributes"]
                .as_array()
                .expect("attributes")
                .iter()
                .map(|a| a.as_str().expect("attribute").to_owned())
                .collect(),
        })
        .collect()
}
