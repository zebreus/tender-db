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
//! EXCEPT a record whose own header says its original is Greek, which is
//! ISO-8859-7 bytes under the same `_ISO_` label (see [`decode_declared_iso`]) —
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
        decode_declared_iso(bytes)
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

/// Decode a declared-`_ISO_` record by the codepage the RECORD implies (issue 393
/// unit 3). The member name says only "ISO" — one label for every language
/// edition — and for a Greek original the bytes are ISO-8859-7, not 8859-1:
/// read as Windows-1252 they render `Γ. Χριστοφιλόπουλος ΑΕ` as
/// `Ã. ×ñéóôïöéëüðïõëïò ÁÅ`, and that string became the IDENTITY of ~1,150
/// provisional organizations (it is what `name_norm` is built from), each one
/// unable to ever meet its canonical Greek profile.
///
/// The header lines are ASCII under every ISO-8859 part, so a first pass under
/// Windows-1252 — which never fails: every byte sequence decodes — reads them
/// safely and decides. Only a record whose original language is Greek (`OL: EL`,
/// or `CY: GR` on the early-1990s records that predate the `OL:` line) is
/// re-decoded as ISO-8859-7; ASCII is identical under both, so the English
/// renderings and the header come out the same either way. Everything else
/// stays Windows-1252, deliberately: the Central-European and Cyrillic editions
/// that would want 8859-2 / 8859-5 exist only from 2004 / 2007, when the UTF8
/// twin supersedes the ISO member (`profile.rs`), and a Latin declaration keeps
/// a Spanish `Ó` an `Ó` even though the same byte is `Σ` in Greek.
fn decode_declared_iso(bytes: &[u8]) -> String {
    let latin = encoding_rs::WINDOWS_1252.decode(bytes).0;
    if declares_greek(&latin) {
        encoding_rs::ISO_8859_7.decode(bytes).0.into_owned()
    } else {
        latin.into_owned()
    }
}

/// `OL:` names the original language(s); Greek is `EL`. A record that predates
/// the line is Greek when its `CY:` is `GR`. Both are header lines at column 0 —
/// a body line is indented — so the scan cannot be fooled by prose.
fn declares_greek(text: &str) -> bool {
    let mut country_gr = false;
    for line in text.lines() {
        if let Some(langs) = line.strip_prefix("OL:") {
            return langs.split_whitespace().any(|l| l.eq_ignore_ascii_case("EL"));
        }
        if let Some(country) = line.strip_prefix("CY:") {
            country_gr = country.trim().eq_ignore_ascii_case("GR");
        }
    }
    country_gr
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
    fn an_iso_record_is_decoded_by_the_language_it_declares() {
        // `Γ. Χριστοφιλόπουλος ΑΕ` in ISO-8859-7: the bytes prod served as
        // `Ã. ×ñéóôïöéëüðïõëïò ÁÅ` (organization 27128935, issue 393 unit 3).
        let name: &[u8] = b"\xc3. \xd7\xf1\xe9\xf3\xf4\xef\xf6\xe9\xeb\xfc\xf0\xef\xf5\xeb\xef\xf2 \xc1\xc5";
        let record = |ol: &[u8]| [b"CY: GR\n" as &[u8], ol, b"CO: ", name, b"\n"].concat();
        assert!(decode_declared_iso(&record(b"OL: EL\n")).contains("CO: Γ. Χριστοφιλόπουλος ΑΕ"));
        // The same bytes under a Latin declaration keep the Windows-1252 reading:
        // the decision is the record's, not the bytes'. `OL:` outranks `CY:`.
        assert!(decode_declared_iso(&record(b"OL: ES\n")).contains("CO: Ã. ×ñéóôïöéëüðïõëïò ÁÅ"));
        // A Spanish `Ó` (0xD3 — `Σ` under 8859-7) survives a Latin declaration.
        assert!(decode_declared_iso(b"CY: ES\nOL: ES\nAU: \xd3RGANO\n").contains("AU: ÓRGANO"));
        // Before `OL:` existed (early 1990s), the country decides.
        let by_country = |cy: &[u8]| [b"CY: " as &[u8], cy, b"\nCO: ", name, b"\n"].concat();
        assert!(decode_declared_iso(&by_country(b"GR")).contains("Χριστοφιλόπουλος"));
        assert!(decode_declared_iso(&by_country(b"ES")).contains("×ñéóôïöéëüðïõëïò"));
    }

    #[test]
    fn utf8_variants_unescape_the_standard_entities_only() {
        assert_eq!(unescape("Azzurra &amp; Imasa &gt; 5"), "Azzurra & Imasa > 5");
        assert_eq!(unescape("no entities"), "no entities");
    }
}
