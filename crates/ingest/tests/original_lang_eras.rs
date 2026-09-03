//! Issue 344: ADR-0013 D3's "original" leg for the national eForms generations
//! that predate SDK-DE — eForms-DE 1.x and the DÖE sdk-0.1 dialect. Both
//! publish `cbc:NoticeLanguageCode` at the root exactly like every SDK profile,
//! and their empirical inventories list it (`DE1-NoticeLanguageCode`,
//! `SDK01-NoticeLanguageCode`); the leg must resolve to it, or every version
//! from those eras carries no original language while the notice says one.

use ingest::profile::{self, Disposition, Record};
use ingest::{eforms, project};
use store::{NoticeValue, Parse, Parsed};

fn parse_fixture(relative: &str) -> Parsed {
    let path = format!("tests/fixtures/{relative}");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let Disposition::Records(records) = profile::dispatch(relative, &bytes) else {
        panic!("{relative}: dispatch skipped the fixture");
    };
    let Some(Record::Notice(notice)) = records.into_iter().next() else {
        panic!("{relative}: expected a notice record");
    };
    match eforms::parse_payload(&notice.profile, &bytes) {
        Parse::Parsed(parsed) => parsed,
        Parse::Quarantined { reason, detail } => {
            panic!("{relative} quarantined: {reason}: {}", detail.unwrap_or_default())
        }
        Parse::Pending => panic!("{relative}: no parser ran"),
    }
}

/// Where the language element landed, for the failure message: every value whose
/// field id mentions the notice language, in any representation.
fn language_values(parsed: &Parsed) -> Vec<String> {
    parsed
        .values
        .iter()
        .filter(|v| v.field_id.contains("NoticeLanguage") || v.field_id.contains("702"))
        .map(|v| {
            let shown = match &v.value {
                NoticeValue::Code { list, code } => format!("Code {code} (list {list:?})"),
                NoticeValue::Text { lang, value } => format!("Text {value:?} (lang {lang:?})"),
                other => format!("{other:?}"),
            };
            format!("{}/{}: {shown}", v.section_id, v.field_id)
        })
        .collect()
}

#[test]
fn the_national_eforms_generations_before_sdk_de_carry_their_original_language() {
    for relative in [
        "doe/eforms-de-1.1-cn-7d69b0f7.xml",
        "doe/eforms-de-1.2-can-799811c4.xml",
        "eforms/doe-sdk01-subcontract-rate.xml",
    ] {
        let parsed = parse_fixture(relative);
        assert_eq!(
            project::original_lang(&parsed).as_deref(),
            Some("DEU"),
            "{relative}: the notice publishes <cbc:NoticeLanguageCode>DEU</…>; the parse carries {:?}",
            language_values(&parsed)
        );
    }
}
