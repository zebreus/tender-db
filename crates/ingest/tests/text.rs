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

/// The winners of that same 199-record delivery (issue 244, slice 3). A real-data gate
/// on the extractor rather than a hand-written body: the fixture's `6.  Supplier(s):`
/// item appears in every shape the 1993 supplies form uses — plain `Name, address`,
/// lot-keyed (`1:`, `A:`, `1/2:`, `1, 2, 3 and 4:`), several winners `;`-separated, and
/// the non-answers (`Various.`, a bare count).
///
/// The count is asserted exactly, so a regression that starts inventing names — or one
/// that stops reading a shape — fails here rather than on prod. Every name is also
/// checked for the two ways a bad boundary shows up: a lot reference left on the front,
/// and a value long enough to be an address rather than a company.
#[test]
fn the_1993_daily_yields_its_award_winners() {
    let records = ingest_fixture(
        "1993-daily-en-19930102.txt",
        "EN_19930102_1993001_ISO_ORG.zip!EN_19930102_1993001_ISO_ORG",
    );

    let mut names = Vec::new();
    for (_, parse) in &records {
        let Parse::Parsed(parsed) = parse else { continue };
        for row in &parsed.values {
            if row.field_id == "TED-OFFICIALNAME" {
                if let NoticeValue::Text { value, .. } = &row.value {
                    names.push(value.clone());
                }
            }
        }
    }

    // 117 winners from 199 records. The number is exact on purpose: it moved from 40 to
    // 117 when lot-keyed and period-separated lists were read rather than swallowed, and
    // a regression in either direction should fail here.
    assert_eq!(names.len(), 117, "winners read from the 1993 daily: {names:#?}");
    for name in &names {
        assert!(!name.is_empty(), "an empty winner name");
        assert!(
            !name.starts_with(|c: char| c.is_ascii_digit()),
            "a lot reference survived into the name: {name:?}"
        );
        assert!(name.len() <= 80, "this is an address, not a company: {name:?}");
        assert!(!name.eq_ignore_ascii_case("various"), "a non-answer became a company");
    }

    // The prices of the same delivery. 1993 writes the lira as `Lit 1 000 000 000` and
    // its ranges as `Lit 2 610/Lit 3 289`, neither of which is a shape a value may be
    // claimed from — so a LOW count here is the correct answer, and asserting it is how
    // a future loosening of parse_money announces itself.
    let mut prices = Vec::new();
    for (_, parse) in &records {
        let Parse::Parsed(parsed) = parse else { continue };
        for row in &parsed.values {
            if row.field_id == "TED-VAL_TOTAL" {
                if let NoticeValue::Amount { cents, currency } = &row.value {
                    prices.push((*cents, currency.clone()));
                }
            }
        }
    }
    for (cents, currency) in &prices {
        assert!(*cents > 0, "a zero price was claimed");
        assert_eq!(currency.len(), 3, "not a currency code: {currency:?}");
    }
    assert_eq!(prices.len(), 0, "1993 states its money in shapes none of which qualify: {prices:?}");

    // Spot-checks: one plain, one lot-keyed, one that the comma must keep whole.
    assert!(names.iter().any(|n| n == "Motorola Limited"), "{names:#?}");
    assert!(names.iter().any(|n| n == "Ailsa Truck and Bus Limited"), "{names:#?}");
    assert!(names.iter().any(|n| n == "SAF (Soudure Artogene Franccaise)"), "{names:#?}");
    // An initial before a period is part of the name, not a lot reference.
    assert!(names.iter().any(|n| n == "H. Meyer GmbH"), "{names:#?}");
    assert!(names.iter().any(|n| n == "B. Braun Medical"), "{names:#?}");
    // The fourteen-lot period-separated list is fourteen winners, not one long string.
    assert_eq!(names.iter().filter(|n| *n == "Discol").count(), 5, "{names:#?}");
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

    // Issue 255 slice 3: the SECTIONED form spells the label differently —
    // `VI.3)  Date of contract award: 25.11.2004.` — and the same reader must claim it.
    //
    // TWICE, because this notice names two suppliers and so yields two result blocks, and
    // the date it states is the notice's single award date: each award carries it. The
    // alternative — putting it on the first block only — would leave the second award
    // undated for no reason the source gives.
    let award_date =
        NoticeValue::Date { utc_seconds: 1_101_340_800, offset_minutes: 0, has_time: false };
    assert_eq!(values(&rec, "TED-CONTRACT_AWARD_DATE"), [&award_date, &award_date]);

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
    assert!(ctx.en_utf8_text.contains("20050101"));

    let bytes = std::fs::read("tests/fixtures/text/2005-can-154-2005.txt").unwrap();
    let iso = "EN_20050101_001_ISO_ORG.ZIP!EN_20050101_2005001_ISO_ORG";
    match profile::dispatch_with(iso, &bytes, &ctx) {
        Disposition::Skipped(reason) => assert_eq!(reason, "text-era-iso-superseded-by-utf8"),
        Disposition::Records(_) => panic!("superseded ISO variant was ingested"),
    }
    // Without a UTF8 twin (1993–2004), the ISO delivery is the one ingested.
    let alone = PackageContext::from_entry_names(&["EN_19930102_1993001_ISO_ORG.zip"]);
    assert!(alone.en_utf8_text.is_empty());
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

/// Issue 255 slice 3: the era's award DATE, gated on the same committed daily the winners
/// are gated on. `Date of award:` appears in dozens of the 199 records, and every claimed
/// date must be a real 1992-1993 calendar date on a result block — a rolled-over typo
/// (`31.2.1993`) or a two-digit year must yield nothing rather than a neighbouring day.
#[test]
fn the_1993_daily_yields_its_award_dates() {
    let records = ingest_fixture(
        "1993-daily-en-19930102.txt",
        "EN_19930102_1993001_ISO_ORG.zip!EN_19930102_1993001_ISO_ORG",
    );

    let mut dates = Vec::new();
    for (_, parse) in &records {
        let Parse::Parsed(parsed) = parse else { continue };
        for row in &parsed.values {
            if row.field_id == "TED-CONTRACT_AWARD_DATE" {
                let NoticeValue::Date { utc_seconds, has_time, .. } = &row.value else {
                    panic!("the award date must be a Date");
                };
                assert!(!has_time, "the era states a calendar date, never a clock");
                dates.push((row.section_id.clone(), *utc_seconds));
            }
        }
    }

    // 72 award dates from 199 records, against the same fixture's 117 winners — the two
    // differ because a date belongs to the award BLOCK while winners are per-organization,
    // and because a body can state a winner without a date or the reverse. Exact on
    // purpose, like the winner count.
    //
    // 72 became 79 with slice 9: seven of this daily's award records are dated but
    // winner-SILENT — four fill the supplier item with `Various.` (54814/54818/54826/
    // 54827-1992, the residue read's own 1993 specimen among them) and three print no
    // supplier heading at all — and each now mints a bare result for its date to land
    // on instead of vanishing. The winner count below stays 117: silence mints a
    // result, never an organization.
    assert_eq!(dates.len(), 79);

    // Every one lands on a result block, never on the root...
    assert!(
        dates.iter().all(|(section, _)| section.starts_with("RES-")),
        "an award date belongs to an award: {dates:?}"
    );
    // ...and every one is a real date in the era of this daily (1993-01-02). A rolled-over
    // typo or a mis-scanned year would land outside it, which is what makes this a gate
    // rather than a count: 1990-01-01 .. 1994-01-01.
    for (section, utc) in &dates {
        assert!(
            (631_152_000..757_382_400).contains(utc),
            "{section}: {utc} is not a date this daily could state"
        );
    }
}
