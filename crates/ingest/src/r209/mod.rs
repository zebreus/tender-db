//! The `ted-export-r209` mapping profile: TED_EXPORT XML notices (2016–2024)
//! parsed into the same notice-parsed layer as the eForms profile — sections
//! plus typed value satellites (docs/research/ted-legacy-mapping.md).
//!
//! Field ids are the source's own terms with a `TED-` prefix
//! (`TED-VAL_TOTAL`, `TED-REFERENCE_NUMBER`): the notice-parsed layer stores
//! "the source's own terms" by design, and the mechanical element→BT tally
//! that would justify writing eForms BT ids here is still open research
//! (ted-legacy-mapping.md §5.2, "unverified estimate"). The projection maps
//! `TED-*` fields onto the canonical shape exactly as it maps `BT-*` fields —
//! and the ~23 legacy elements with no eForms equivalent (REFERENCE_NUMBER,
//! VAL_RANGE_TOTAL, URL_NATIONAL_PROCEDURE, …) need no special satellite:
//! the prefix already keeps them distinguishable.
//!
//! This module also owns the **defence forms** (F16–F19): they never migrated
//! to R2.0.9 and keep their R2.0.8 grammar through 2024, so files dispatched
//! as `ted-export-r208` whose FORM_SECTION carries a defence form are parsed
//! here. Non-defence r208 files stay `Pending` for issue 10.

pub mod parse;
pub mod rules;
pub mod value;

pub use parse::{Rejected, parse};

/// The era completeness checklist: every element and attribute the mirrored
/// XSDs declare (R2.0.9 S01+S05 union, plus the defence-form subset of
/// R2.0.8.S05), vendored like the eForms profile vendors `fields.json`.
pub const INVENTORY_JSON: &str = include_str!("../../sdk/r209-inventory.json");

/// Parse a notice payload for a TED_EXPORT profile.
pub fn parse_payload(profile: &str, bytes: &[u8]) -> store::Parse {
    let Ok(xml) = std::str::from_utf8(bytes) else {
        return store::Parse::Quarantined { reason: "not-utf8".into(), detail: None };
    };
    match profile {
        "ted-export-r209" => {}
        // R2.0.8-namespace files: only the defence forms belong to this era's
        // grammar; everything else waits for the r208 profile (issue 10).
        "ted-export-r208" if has_defence_form(xml) => {}
        _ => return store::Parse::Pending,
    }
    match parse(xml) {
        Ok(parsed) => store::Parse::Parsed(parsed),
        Err(Rejected { reason, detail }) => {
            store::Parse::Quarantined { reason: reason.into(), detail: Some(detail) }
        }
    }
}

fn has_defence_form(xml: &str) -> bool {
    let Ok(doc) = roxmltree::Document::parse(xml) else { return false };
    doc.root_element()
        .children()
        .filter(|c| c.is_element() && c.tag_name().name() == "FORM_SECTION")
        .flat_map(|fs| fs.children())
        .any(|c| c.is_element() && parse::DEFENCE_FORMS.contains(&c.tag_name().name()))
}
