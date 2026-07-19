//! The `text` mapping profile: tagged-text era notices (1993–2010) parsed
//! into the same notice-parsed layer as the XML profiles — the coded header
//! fields typed, the bodies preserved as declared text
//! (docs/research/ted-legacy-mapping.md §7, CONTEXT.md "text era header-only").
//!
//! A text-era record is ~20–30 `XX:` header lines followed by prose blobs
//! (`TX:` the English body, `OT:` the original-language body, `AB:` the
//! abstract). The package walker has already split the day's single document
//! into records (`profile::dispatch`); this module parses one record.
//!
//! ## Encoding
//!
//! The member name declares the encoding (`EN_19930102_1993001_ISO_ORG` /
//! `EN_20050101_2005001_UTF8_ORG`). Declared-ISO members decode via
//! `encoding_rs::WINDOWS_1252` — the WHATWG meaning of the iso-8859-1 label,
//! which also absorbs the handful of C1 bytes (0x98) real ISO dailies contain
//! where pure Latin-1 would yield control characters. UTF8 members decode
//! strictly (a bad byte quarantines) and additionally carry XML entity
//! escapes (`&amp;`, `&lt;`, …) as a serialization artifact — measured
//! against the ISO twin of the same 2005 daily, which spells the same content
//! unescaped — so the five standard entities are unescaped for that variant
//! only.
//!
//! ## Language tags
//!
//! Only the English delivery is fetched (CONTEXT.md). English renderings
//! (`TI`, `AB`, `TX`, and the `PN`/`CT` classification labels) are tagged
//! `EN`; names and addresses (`AU`, `TW`, `CO`, `RG`, `IA`) carry no tag; the
//! `OT` body carries no tag either, because a bilingual original (Belgian
//! FR+NL — measured) publishes two `OT` blocks while the record declares just
//! one original language (`OL`).

pub mod parse;
pub mod rules;

pub use parse::{Rejected, parse};

/// The era completeness checklist: the header field-code inventory mined from
/// the real sample dailies (no official spec of the tagged format exists),
/// vendored like the XML profiles vendor their XSD inventories.
pub const INVENTORY_JSON: &str = include_str!("../../sdk/text-inventory.json");

/// Parse one text-era record. `member_path` is the record's member path (the
/// `#<n>`-suffixed nested-ZIP path), which names the declared encoding.
pub fn parse_payload(member_path: &str, bytes: &[u8]) -> store::Parse {
    let text = if declared_iso(member_path) {
        // Never fails: every byte sequence decodes under windows-1252.
        encoding_rs::WINDOWS_1252.decode(bytes).0.into_owned()
    } else {
        match std::str::from_utf8(bytes) {
            Ok(text) => unescape(text),
            Err(e) => {
                return store::Parse::Quarantined {
                    reason: "not-utf8".into(),
                    detail: Some(e.to_string()),
                };
            }
        }
    };
    match parse(&text) {
        Ok(parsed) => store::Parse::Parsed(parsed),
        Err(Rejected { reason, detail }) => {
            store::Parse::Quarantined { reason: reason.into(), detail: Some(detail) }
        }
    }
}

/// Whether the member name declares the ISO-8859-1 variant
/// (`<lg>_<date>_<issue>_ISO_ORG`, `#<n>` record suffix tolerated).
fn declared_iso(member_path: &str) -> bool {
    let name = member_path.rsplit(['!', '/']).next().unwrap_or(member_path);
    let name = name.split('#').next().unwrap_or(name);
    name.split('_').nth(3).is_some_and(|variant| variant.eq_ignore_ascii_case("iso"))
}

/// The five standard XML entities, in the UTF8 variants only (see module doc).
fn unescape(text: &str) -> String {
    if !text.contains('&') {
        return text.to_owned();
    }
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_member_name_declares_the_encoding() {
        assert!(declared_iso("EN_19930102_1993001_ISO_ORG.zip!EN_19930102_1993001_ISO_ORG#3"));
        assert!(!declared_iso("en_20080103_001_utf8_org.zip!EN_20080103_2008001_UTF8_ORG#0"));
    }

    #[test]
    fn utf8_variants_unescape_the_standard_entities_only() {
        assert_eq!(unescape("Azzurra &amp; Imasa &gt; 5"), "Azzurra & Imasa > 5");
        assert_eq!(unescape("no entities"), "no entities");
    }
}
