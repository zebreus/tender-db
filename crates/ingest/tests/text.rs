//! `text` profile tests, driven by the committed real-record corpus: the
//! complete 1993-01-02 English daily (the splitter fixture), single 2000 /
//! 2005 / 2008 records, and the vendored field-code inventory.
//!
//! Two guarantees, mirroring the XML suites. ADR-0004: every line of a real
//! record is claimed or the record quarantines whole. ADR-0002 (era-scoped):
//! every code of the vendored inventory has a rule and vice versa.

use ingest::profile::{self, Disposition, PackageContext, Record};
use ingest::text::{self, rules};
use store::{NoticeValue, Parse, Parsed};

/// Dispatch fixture bytes under a real member name and parse every record.
fn ingest_fixture(fixture: &str, member_path: &str) -> Vec<(String, Parse)> {
    let path = format!("tests/fixtures/text/{fixture}");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let Disposition::Records(records) = profile::dispatch(member_path, &bytes) else {
        panic!("{fixture}: dispatch skipped a text-era member");
    };
    records
        .into_iter()
        .map(|record| match record {
            Record::Notice(n) => {
                let (start, end) = n.span.expect("text records carry their span");
                (n.publication_id, text::parse_payload(&n.member_path, &bytes[start..end]))
            }
            Record::Quarantine(q) => panic!("{fixture}: quarantined at dispatch: {}", q.reason),
        })
        .collect()
}

fn parse_one(fixture: &str, member_path: &str) -> Parsed {
    let mut records = ingest_fixture(fixture, member_path);
    assert_eq!(records.len(), 1, "{fixture}: expected a single record");
    match records.remove(0).1 {
        Parse::Parsed(parsed) => parsed,
        Parse::Quarantined { reason, detail } => {
            panic!("{fixture} quarantined: {reason}: {}", detail.unwrap_or_default())
        }
        Parse::Pending => panic!("{fixture}: no parser ran"),
    }
}

fn values<'a>(parsed: &'a Parsed, field: &str) -> Vec<&'a NoticeValue> {
    parsed.values.iter().filter(|v| v.field_id == field).map(|v| &v.value).collect()
}

fn value<'a>(parsed: &'a Parsed, field: &str) -> &'a NoticeValue {
    let found = values(parsed, field);
    assert_eq!(found.len(), 1, "expected one {field}, got {}", found.len());
    found[0]
}

fn text_value(parsed: &Parsed, field: &str) -> (Option<String>, String) {
    match value(parsed, field) {
        NoticeValue::Text { lang, value } => (lang.clone(), value.clone()),
        other => panic!("{field} is not text: {other:?}"),
    }
}

// ------------------------------------------------------------ exhaustiveness

/// The whole 1993-01-02 English delivery: 199 records split out, every one
/// parsed exhaustively, and the RN chain-edge fill rate matches the research
/// measurement (105 of 199 — ted-legacy-mapping.md §3).
#[test]
fn the_1993_daily_splits_and_parses_completely() {
    let records = ingest_fixture(
        "1993-daily-en-19930102.txt",
        "EN_19930102_1993001_ISO_ORG.zip!EN_19930102_1993001_ISO_ORG",
    );
    assert_eq!(records.len(), 199);

    let mut with_rn = 0;
    for (id, parse) in &records {
        match parse {
            Parse::Parsed(parsed) => {
                assert!(!parsed.values.is_empty(), "{id}: parsed but empty");
                if !values(parsed, "TXT-RN").is_empty() {
                    with_rn += 1;
                }
            }
            other => panic!("{id}: {other:?}"),
        }
    }
    assert_eq!(with_rn, 105);
}

// ------------------------------------------------------------- value mapping

/// The first 1993 record in depth: the 1993 vintage carries the pre-CPV
/// CC/CT product classification and the OJ page, and its deadline pairs a
/// wall clock with the compact date.
#[test]
fn a_1993_record_maps_its_coded_header() {
    let records = ingest_fixture(
        "1993-daily-en-19930102.txt",
        "EN_19930102_1993001_ISO_ORG.zip!EN_19930102_1993001_ISO_ORG",
    );
    let (id, parse) = &records[0];
    assert_eq!(id, "52472-1992");
    let Parse::Parsed(rec) = parse else { panic!("{parse:?}") };

    assert!(matches!(value(rec, "TXT-ND"),
        NoticeValue::Id { value, is_ref: false, .. } if value == "52472-1992"));
    assert!(matches!(value(rec, "TXT-OJ"),
        NoticeValue::Id { value, .. } if value == "1/1993"));
    assert_eq!(*value(rec, "TXT-PG"), NoticeValue::Integer(73));

    // Dates: publication, reception/dispatch, and the timed deadline.
    assert_eq!(
        *value(rec, "TXT-PD"),
        NoticeValue::Date { utc_seconds: 725_932_800, offset_minutes: 0, has_time: false }
    );
    assert_eq!(
        *value(rec, "TXT-DT"),
        NoticeValue::Date { utc_seconds: 727_977_600, offset_minutes: 0, has_time: true }
    );

    // The coded backbone — the same TD/NC/PR/AA code lists the XML eras use.
    for (field, code) in
        [("TXT-TD", "3"), ("TXT-NC", "2"), ("TXT-PR", "4"), ("TXT-AA", "3"), ("TXT-CY", "FR")]
    {
        assert!(matches!(value(rec, field),
            NoticeValue::Code { code: c, .. } if c == code), "{field}");
    }

    // Pre-CPV product classification, scheme `cc`.
    assert!(matches!(value(rec, "TXT-CC"),
        NoticeValue::Classification { scheme, code } if scheme == "cc" && code == "3140"));
    assert_eq!(text_value(rec, "TXT-CT"), (Some("EN".into()), "STRUCTURAL METAL PRODUCTS".into()));

    // Title (with its authenticity note continuation), buyer, one-row body.
    let (lang, title) = text_value(rec, "TXT-TI");
    assert_eq!(lang.as_deref(), Some("EN"));
    assert_eq!(title, "F-Paris: lighting supports\n(Only the original text is authentic)");
    assert_eq!(text_value(rec, "TXT-AU").1, "MAIRIE DE PARIS");
    let (lang, body) = text_value(rec, "TXT-TX");
    assert_eq!(lang.as_deref(), Some("EN"));
    assert!(body.starts_with(" 1.  Awarding authority: Mairie de Paris"));
    assert!(body.ends_with("Notice received on: 24. 12. 1992."));
}

/// Issue 31: `RP` code `2` (international financing) is published as the lead
/// institution plus one continuation line per co-financier. That continuation
/// under a then-scalar field quarantined the whole record; `RP` is now a
/// per-line list, so the lead is the typed code and each co-financier is
/// claimed (a code-less line degrades to raw text — never dropped, never fatal).
#[test]
fn rp_lists_every_co_financing_institution() {
    let rec = parse_one(
        "1993-rp-list-224-1993.txt",
        "EN_19930109_1993006_ISO_ORG.zip!EN_19930109_1993006_ISO_ORG",
    );
    let rp = values(&rec, "TXT-RP");
    assert_eq!(rp.len(), 3, "the lead institution plus its two co-financiers");
    assert!(matches!(rp[0], NoticeValue::Code { code, .. } if code == "2"), "lead is the typed code");
    assert!(
        matches!(rp[2], NoticeValue::Text { value, .. } if value == "European Central Bank"),
        "co-financiers are claimed as text",
    );
    // The record's other multi-line fields still parse (regression guard).
    assert!(matches!(value(&rec, "TXT-ND"), NoticeValue::Id { value, .. } if value == "224-1993"));
}

/// Issue 35: the 1995-98 vintages carry the main object classification as `OC`
/// (one CPV code per line, like `PC`) paired with `ON`, the English description
/// per code (like `CT` labels `CC`). Both were unmapped, so the whole record —
/// a real EN notice — quarantined (~577k members, ~all the single code `OC`).
/// Now `OC` is claimed as CPV classifications and `ON` as English text.
#[test]
fn oc_and_on_are_claimed_as_cpv_and_description() {
    let rec = parse_one(
        "1995-oc-cpv-4149-1995.txt",
        "EN_19950201_1995021_ISO_ORG.zip!EN_19950201_1995021_ISO_ORG",
    );

    // OC is the primary CPV code, claimed as a cpv classification (not raw text).
    let oc = value(&rec, "TXT-OC");
    assert!(
        matches!(oc, NoticeValue::Classification { scheme, code } if scheme == "cpv" && code == "71101000"),
        "OC is a cpv classification, got {oc:?}",
    );
    // ON is the English object description, claimed as text.
    let on = values(&rec, "TXT-ON");
    assert!(
        matches!(on[0], NoticeValue::Text { lang: Some(l), value } if l == "EN" && value.starts_with("Leasing or rental of private cars")),
        "ON is EN text, got {:?}", on[0],
    );
    // The record's own identity and its separate PC/CC classifications still parse
    // (regression guard: OC/ON sit alongside them, not in place of them).
    assert!(matches!(value(&rec, "TXT-ND"), NoticeValue::Id { value, .. } if value == "4149-1995"));
    assert_eq!(values(&rec, "TXT-PC").len(), 3, "the additional CPV list is unaffected");
    assert!(matches!(value(&rec, "TXT-CC"),
        NoticeValue::Classification { scheme, code } if scheme == "cc" && code == "8400"));
}

/// The 2005 award record: the RN chain edge XML-era notices terminate on,
/// and the multi-value continuation format of PC/PN.
#[test]
fn the_2005_award_record_carries_the_chain_edge() {
    let rec = parse_one(
        "2005-can-154-2005.txt",
        "EN_20050101_001_UTF8_ORG.ZIP!EN_20050101_2005001_UTF8_ORG",
    );

    assert!(matches!(value(&rec, "TXT-ND"),
        NoticeValue::Id { value, is_ref: false, .. } if value == "154-2005"));
    assert_eq!(
        *value(&rec, "TXT-RN"),
        NoticeValue::Id { scheme: Some("ojs".into()), value: "108785-2003".into(), is_ref: true }
    );
    assert!(matches!(value(&rec, "TXT-TD"),
        NoticeValue::Code { code, .. } if code == "7"));

    // Three CPV codes across continuation lines, labels alongside.
    let cpv: Vec<_> = values(&rec, "TXT-PC")
        .iter()
        .map(|v| match v {
            NoticeValue::Classification { scheme, code } if scheme == "cpv" => code.clone(),
            other => panic!("not cpv: {other:?}"),
        })
        .collect();
    assert_eq!(cpv, ["28216100", "28811200", "40530000"]);
    assert_eq!(values(&rec, "TXT-PN").len(), 3);

    // Award-side facts: dispatch date, winners prose, no tender deadline.
    assert_eq!(
        *value(&rec, "TXT-DS"),
        NoticeValue::Date { utc_seconds: 1_103_846_400, offset_minutes: 0, has_time: false }
    );
    let (_, winners) = text_value(&rec, "TXT-CO");
    assert!(winners.contains("Grahams Engineering Ltd."));
    assert!(winners.contains("NSG Environmental Ltd."));
    assert!(values(&rec, "TXT-DT").is_empty());
    assert_eq!(text_value(&rec, "TXT-TW").1, "DIDCOT");
}

/// The 2008 contract notice: a timed deadline, and the bilingual body pair —
/// the English `TX` rendering plus the original-language `OT` blob.
#[test]
fn the_2008_record_keeps_both_bodies_and_the_deadline_clock() {
    let rec = parse_one(
        "2008-cn-723-2008.txt",
        "en_20080103_001_utf8_org.zip!EN_20080103_2008001_UTF8_ORG",
    );

    assert_eq!(
        *value(&rec, "TXT-DT"),
        NoticeValue::Date { utc_seconds: 1_202_385_600, offset_minutes: 0, has_time: true }
    );
    assert!(matches!(value(&rec, "TXT-OL"),
        NoticeValue::Code { code, .. } if code == "FR"));
    assert_eq!(text_value(&rec, "TXT-IA").1, "http://www.enpc.fr");
    assert_eq!(values(&rec, "TXT-PC").len(), 2);

    let (lang, tx) = text_value(&rec, "TXT-TX");
    assert_eq!(lang.as_deref(), Some("EN"));
    assert!(tx.starts_with("CONTRACT NOTICE"));
    let (lang, ot) = text_value(&rec, "TXT-OT");
    assert_eq!(lang, None, "per-block language of OT is undeclared");
    assert!(ot.starts_with("AVIS DE MARCHÉ"));
}

/// The declared-ISO decode path on real Latin-1 bytes: the 2000 record's
/// Spanish original text carries á/é/í/ó high bytes.
#[test]
fn latin1_declared_members_decode_their_high_bytes() {
    let rec = parse_one(
        "2000-pin-130-2000.txt",
        "EN_20000104_001_ISO_ORG.ZIP!EN_20000104_2000001_ISO_ORG",
    );

    let (_, ot) = text_value(&rec, "TXT-OT");
    assert!(ot.contains("Bilbao Ría 2000"), "decoded OT: {}", &ot[..80]);
    assert!(ot.contains("José María Olábarri"));

    // Date-only deadline and the five-code CPV list.
    assert_eq!(
        *value(&rec, "TXT-DT"),
        NoticeValue::Date { utc_seconds: 976_924_800, offset_minutes: 0, has_time: false }
    );
    assert_eq!(values(&rec, "TXT-PC").len(), 5);
}

// -------------------------------------------------------------- era policy

/// Mid-era dailies ship the English delivery in both encodings; with package
/// context the lossy ISO twin is a documented skip, not a doubled ingest.
#[test]
fn iso_variant_is_superseded_when_the_package_ships_utf8() {
    let ctx = PackageContext::from_entry_names(&[
        "EN_20050101_001_ISO_ORG.ZIP",
        "EN_20050101_001_UTF8_ORG.ZIP",
        "FR_20050101_001_ISO_ORG.ZIP",
    ]);
    assert!(ctx.en_utf8_text);

    let bytes = std::fs::read("tests/fixtures/text/2005-can-154-2005.txt").unwrap();
    let iso = "EN_20050101_001_ISO_ORG.ZIP!EN_20050101_2005001_ISO_ORG";
    match profile::dispatch_with(iso, &bytes, &ctx) {
        Disposition::Skipped(reason) => assert_eq!(reason, "text-era-iso-superseded-by-utf8"),
        Disposition::Records(_) => panic!("superseded ISO variant was ingested"),
    }
    // Without a UTF8 twin (1993–2004), the ISO delivery is the one ingested.
    let alone = PackageContext::from_entry_names(&["EN_19930102_1993001_ISO_ORG.zip"]);
    assert!(!alone.en_utf8_text);
    assert!(matches!(profile::dispatch_with(iso, &bytes, &alone), Disposition::Records(_)));
}

// -------------------------------------------------------------- completeness

/// The era-scoped ADR-0002 harness: every inventory code has a rule, every
/// rule names an inventory code.
#[test]
fn every_inventory_code_has_a_rule_and_vice_versa() {
    let inventory = inventory_codes();
    let missing: Vec<&str> =
        inventory.iter().map(String::as_str).filter(|c| rules::rule(c).is_none()).collect();
    assert!(missing.is_empty(), "inventory codes without a rule: {missing:?}");

    let stray: Vec<&str> =
        rules::decided_codes().filter(|c| !inventory.iter().any(|i| i == c)).collect();
    assert!(stray.is_empty(), "rules for codes the inventory does not declare: {stray:?}");

    assert_eq!(inventory.len(), 36, "the sampled-vintage sweep found 34 codes, + OC/ON (issue 35)");
}

fn inventory_codes() -> Vec<String> {
    let json: serde_json::Value =
        serde_json::from_str(text::INVENTORY_JSON).expect("vendored inventory");
    json["fields"]
        .as_array()
        .expect("fields array")
        .iter()
        .map(|f| f["code"].as_str().expect("code").to_owned())
        .collect()
}
