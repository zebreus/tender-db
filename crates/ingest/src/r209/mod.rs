//! The TED_EXPORT mapping profiles (`ted-export-r209` and `ted-export-r208`):
//! legacy TED XML notices (2011–2024) parsed into the same notice-parsed
//! layer as the eForms profile — sections plus typed value satellites
//! (docs/research/ted-legacy-mapping.md).
//!
//! One walker and one rule registry cover the whole family: element names are
//! essentially globally unique across the R2.0.7–R2.0.9 grammars, and the
//! era conventions the R2.0.8 standard forms need (D/M/Y split dates, FMTVAL
//! machine values, IDEM markers, deeply nested wrapper names) are exactly the
//! defence-form conventions that never migrated to R2.0.9. R2.0.7
//! (2011–2013) has no mirrored XSD; its empirically-mined delta rides in the
//! same inventory as `r208-observed` entries, and anything it publishes
//! beyond that quarantines rather than being guessed at (ADR-0004).
//! OTH_NOT/EEIG bodies are declared prose: coded header + text rows, a
//! corrigendum arriving as a version event whose diff is text.
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

pub mod parse;
pub mod rules;
pub mod value;

pub use parse::{Rejected, parse};

/// The era completeness checklist: every element and attribute the mirrored
/// XSDs declare (R2.0.9 S01+S05 and R2.0.8 S03+S05 unions) plus the
/// empirically-mined names real R2.0.7/early-R2.0.8 dailies publish beyond
/// them, vendored like the eForms profile vendors `fields.json`.
pub const INVENTORY_JSON: &str = include_str!("../../sdk/ted-export-inventory.json");

/// Parse a notice payload for a TED_EXPORT profile.
pub fn parse_payload(profile: &str, bytes: &[u8]) -> store::Parse {
    if !matches!(profile, "ted-export-r209" | "ted-export-r208") {
        return store::Parse::Pending;
    }
    let Ok(xml) = std::str::from_utf8(bytes) else {
        return store::Parse::Quarantined { reason: "not-utf8".into(), detail: None };
    };
    match parse(xml) {
        Ok(parsed) => store::Parse::Parsed(parsed),
        Err(Rejected { reason, detail }) => {
            store::Parse::Quarantined { reason: reason.into(), detail: Some(detail) }
        }
    }
}
