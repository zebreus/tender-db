//! The eForms mapping profile: UBL 2.3 notices (2022→) parsed into the
//! notice-parsed layer, exhaustively.
//!
//! Three pieces, deliberately separable so the r209/r208/text profiles can
//! reuse the shape:
//!
//! - [`sdk`] — the vendored `fields.json` per accepted SDK minor, plus the
//!   mapping registry: one recorded decision per field id (ADR-0002).
//! - [`index`] — those xpaths folded into a match tree, using the [`xpath`]
//!   subset the SDK actually writes.
//! - [`parse`] — a simultaneous descent of document and match tree in which
//!   nothing may go unclaimed (ADR-0004).

pub mod index;
pub mod parse;
pub mod sdk;
pub mod value;
pub mod xpath;

pub use parse::{Rejected, parse};

/// The `CustomizationID` of an `eforms:<customization>` profile id.
pub fn customization(profile: &str) -> Option<&str> {
    profile.strip_prefix("eforms:")
}

/// Parse a notice payload for an eForms profile, mapping every outcome onto
/// what the store records.
pub fn parse_payload(profile: &str, bytes: &[u8]) -> store::Parse {
    let Some(customization) = customization(profile) else {
        return store::Parse::Pending;
    };
    let Ok(xml) = std::str::from_utf8(bytes) else {
        return store::Parse::Quarantined { reason: "not-utf8".into(), detail: None };
    };
    match parse(xml, customization) {
        Ok(parsed) => store::Parse::Parsed(parsed),
        Err(Rejected { reason, detail }) => {
            store::Parse::Quarantined { reason: reason.into(), detail: Some(detail) }
        }
    }
}
