//! Per-file profile dispatch: decide which mapping profile a payload belongs
//! to and extract its Notice identity. No field mapping happens here.
//!
//! Dispatch keys off the XML root's **namespace URI + local name** — never the
//! prefix, because real TED packages vary it freely (`ContractNotice`,
//! `urn:ContractNotice`, `ns8:ContractNotice` and `cn:ContractNotice` all occur
//! within a single day) — plus the eForms `CustomizationID`.
//!
//! Profiles, and how they were verified against the sample era ladder:
//!
//! | profile              | era         | detected by                              |
//! |----------------------|-------------|------------------------------------------|
//! | `text`               | 1993–2010   | tagged-text member inside a per-language ZIP |
//! | `ted-export-r208`    | 2011–2019   | root `TED_EXPORT`, version R2.0.7/R2.0.8 or unversioned |
//! | `ted-export-r209`    | 2015–2025   | root `TED_EXPORT`, version R2.0.9        |
//! | `eforms:<custom>`    | 2023–       | UBL root namespace + `CustomizationID`   |
//!
//! Anything else quarantines (ADR-0004): unknown content is never silently
//! dropped.

use crate::sha256_hex;

/// What one package member turned into. A member is always exactly one of
/// these — that is the "no silent drops" invariant the processor asserts.
pub enum Disposition {
    /// The member yielded records. XML members yield exactly one; a text-era
    /// bundle yields one per notice in the day's document.
    Records(Vec<Record>),
    /// Deliberately not ingested, per a documented project decision. Counted
    /// and reported, never silent.
    Skipped(&'static str),
}

pub enum Record {
    Notice(NoticeRecord),
    Quarantine(QuarantineRecord),
}

pub struct NoticeRecord {
    pub publication_id: String,
    pub content_hash: String,
    pub profile: String,
    pub declared_version: Option<String>,
    /// Member path, suffixed with `#<n>` for one record of a text bundle.
    pub member_path: String,
}

pub struct QuarantineRecord {
    pub content_hash: String,
    pub profile: Option<String>,
    pub reason: String,
    pub detail: Option<String>,
    pub member_path: String,
}

/// eForms root namespace families. Most notice types are UBL documents, but
/// BusinessRegistrationInformationNotice is rooted in the eForms `p27` family
/// instead — it is a first-class notice type (CONTEXT.md: BRIN notices become
/// minimal Tenders of a distinct kind), not an anomaly to quarantine.
const EFORMS_ROOT_NS: [&str; 2] =
    ["urn:oasis:names:specification:ubl:schema:xsd:", "http://data.europa.eu/p27/"];

/// Classify one package member.
pub fn dispatch(member_path: &str, bytes: &[u8]) -> Disposition {
    // The text era is recognised by member naming, before any XML attempt: its
    // payloads are not XML, and its `_meta_` sibling variant *is* XML but is a
    // duplicate representation we do not ingest.
    if let Some(name) = text_era_member(member_path) {
        return dispatch_text(member_path, &name, bytes);
    }

    let Ok(xml) = std::str::from_utf8(bytes) else {
        return one(quarantine(member_path, bytes, None, "not-utf8", None));
    };
    let doc = match roxmltree::Document::parse(xml) {
        Ok(doc) => doc,
        Err(e) => return one(quarantine(member_path, bytes, None, "unparsable-xml", Some(e.to_string()))),
    };
    let root = doc.root_element();
    let ns = root.tag_name().namespace().unwrap_or_default();
    let local = root.tag_name().name();

    if local == "TED_EXPORT" {
        one(dispatch_ted_export(member_path, bytes, &root, ns))
    } else if EFORMS_ROOT_NS.iter().any(|family| ns.starts_with(family)) {
        one(dispatch_eforms(member_path, bytes, &doc))
    } else {
        one(quarantine(member_path, bytes, None, "unknown-root", Some(format!("{{{ns}}}{local}"))))
    }
}

/// Legacy TED_EXPORT XML (2011–2025). The declared version lives in one of
/// four places depending on the year, so they are tried in order of authority;
/// early-2011 files carry no version marker at all and fall back to the R2.0.8
/// profile, which per docs/architecture.md also covers R2.0.7.
fn dispatch_ted_export(
    member_path: &str,
    bytes: &[u8],
    root: &roxmltree::Node<'_, '_>,
    ns: &str,
) -> Record {
    let schema_location = root
        .attribute(("http://www.w3.org/2001/XMLSchema-instance", "schemaLocation"))
        .unwrap_or_default();
    let declared_version = root
        .attribute("VERSION")
        .map(str::to_owned)
        .or_else(|| version_token(schema_location))
        .or_else(|| version_token(ns))
        .or_else(|| {
            root.descendants().find_map(|n| n.attribute("VERSION").and_then(version_token))
        });

    let profile = match declared_version.as_deref() {
        Some(v) if v.starts_with("R2.0.9") => "ted-export-r209",
        _ => "ted-export-r208",
    };

    // DOC_ID is the publication number (`000036-2011`) and is present on every
    // file across the ladder. The CODED_DATA_SECTION's NO_DOC_OJS holds the
    // same identity in OJS reference form (`2011/S 1-000036`); DOC_ID is used
    // because it shares its shape with the eForms publication number.
    match root.attribute("DOC_ID") {
        Some(id) => Record::Notice(NoticeRecord {
            publication_id: id.trim().to_owned(),
            content_hash: sha256_hex(bytes),
            profile: profile.into(),
            declared_version,
            member_path: member_path.into(),
        }),
        None => quarantine(member_path, bytes, Some(profile.into()), "missing-publication-id", None),
    }
}

/// eForms UBL (2023–). Sub-profiles are per CustomizationID, which also carries
/// national dialects (`eforms-de`, the DÖE `sdk-0.1` channel).
fn dispatch_eforms(member_path: &str, bytes: &[u8], doc: &roxmltree::Document<'_>) -> Record {
    // Matched on local name only: the extension namespaces are versioned and
    // differ across SDK releases and national dialects.
    let customization = first_text(doc, "CustomizationID");
    let Some(customization) = customization else {
        return quarantine(member_path, bytes, None, "missing-customization-id", None);
    };
    let profile = format!("eforms:{customization}");

    // efbc:NoticePublicationID (`00001505-2024`); present on every eForms file
    // in the samples. The file name carries the same number, so it is the
    // fallback rather than a second source of truth.
    let publication_id = first_text(doc, "NoticePublicationID").or_else(|| publication_id_from_name(member_path));
    match publication_id {
        Some(id) => Record::Notice(NoticeRecord {
            publication_id: id,
            content_hash: sha256_hex(bytes),
            profile,
            declared_version: Some(customization),
            member_path: member_path.into(),
        }),
        None => quarantine(member_path, bytes, Some(profile), "missing-publication-id", None),
    }
}

/// Tagged-text era (1993–2010). One member is the whole day's English document:
/// notices separated by a `1.00/067192`-style record marker, each carrying its
/// OJS notice number on an `ND:` line. Records are split out so a Notice stays
/// one publication event; the file is not otherwise parsed.
fn dispatch_text(member_path: &str, name: &TextEraName, bytes: &[u8]) -> Disposition {
    // CONTEXT.md: the text era is English-only for now (the model stays
    // multilingual; the raw archive keeps every language).
    if !name.language.eq_ignore_ascii_case("en") {
        return Disposition::Skipped("text-era-non-english");
    }
    // `_meta_` is a parallel XML-ish rendering of the very same notices as the
    // `_utf8_`/`_iso_` tagged text — ingesting both would double every notice.
    if name.variant.eq_ignore_ascii_case("meta") {
        return Disposition::Skipped("text-era-meta-variant");
    }

    let starts = record_starts(bytes);
    if starts.is_empty() {
        return one(quarantine(member_path, bytes, Some("text".into()), "text-no-records", None));
    }

    let mut records = Vec::with_capacity(starts.len());
    for (i, &start) in starts.iter().enumerate() {
        let end = starts.get(i + 1).copied().unwrap_or(bytes.len());
        let record = &bytes[start..end];
        let path = format!("{member_path}#{i}");
        records.push(match tagged_field(record, b"ND") {
            Some(id) => Record::Notice(NoticeRecord {
                publication_id: id,
                content_hash: sha256_hex(record),
                profile: "text".into(),
                declared_version: None,
                member_path: path,
            }),
            None => quarantine(&path, record, Some("text".into()), "missing-publication-id", None),
        });
    }
    Disposition::Records(records)
}

/// Text-era member naming, e.g. `EN_19930102_1993001_ISO_ORG.zip!EN_19930102_1993001_ISO_ORG`
/// or `en_20100102_001_utf8_org.zip!EN_20100102_2010001_UTF8_ORG`.
struct TextEraName {
    language: String,
    variant: String,
}

fn text_era_member(member_path: &str) -> Option<TextEraName> {
    let name = member_path.rsplit(['!', '/']).next()?;
    let parts: Vec<&str> = name.split('_').collect();
    // <lg>_<date>_<issue>_<variant>_ORG
    let [language, date, _issue, variant, org] = parts[..] else { return None };
    let ok = language.len() == 2
        && language.chars().all(|c| c.is_ascii_alphabetic())
        && date.len() == 8
        && date.chars().all(|c| c.is_ascii_digit())
        && org.eq_ignore_ascii_case("org")
        && ["iso", "utf8", "meta"].iter().any(|v| v.eq_ignore_ascii_case(variant));
    ok.then(|| TextEraName { language: language.into(), variant: variant.into() })
}

/// Byte offsets of each `1.00/067192`-style record marker line.
fn record_starts(bytes: &[u8]) -> Vec<usize> {
    let mut starts = Vec::new();
    let mut offset = 0;
    for line in bytes.split_inclusive(|&b| b == b'\n') {
        if is_record_marker(line) {
            starts.push(offset);
        }
        offset += line.len();
    }
    starts
}

/// `<digits>.<digits>/<digits>` alone on its line.
fn is_record_marker(line: &[u8]) -> bool {
    let line = line.trim_ascii();
    let Some((version, number)) = split_once(line, b'/') else { return false };
    let Some((major, minor)) = split_once(version, b'.') else { return false };
    [major, minor, number].iter().all(|p| !p.is_empty() && p.iter().all(u8::is_ascii_digit))
}

/// Value of a `TAG: value` line in a text-era record, e.g. `ND: 52472-1992`.
fn tagged_field(record: &[u8], tag: &[u8]) -> Option<String> {
    for line in record.split(|&b| b == b'\n') {
        if let Some(rest) = line.strip_prefix(tag).and_then(|r| r.strip_prefix(b":")) {
            let value = rest.trim_ascii();
            if !value.is_empty() {
                return Some(String::from_utf8_lossy(value).into_owned());
            }
        }
    }
    None
}

/// `…/00001505_2024.xml` → `00001505-2024`.
fn publication_id_from_name(member_path: &str) -> Option<String> {
    let stem = member_path.rsplit(['!', '/']).next()?.strip_suffix(".xml")?;
    let (number, year) = stem.split_once('_')?;
    let ok = !number.is_empty()
        && number.bytes().all(|b| b.is_ascii_digit())
        && year.len() == 4
        && year.bytes().all(|b| b.is_ascii_digit());
    ok.then(|| format!("{number}-{year}"))
}

/// First `R2.0.x…` token in a namespace URI or schemaLocation.
fn version_token(text: &str) -> Option<String> {
    let start = text.find("R2.0.")?;
    let token: String =
        text[start..].chars().take_while(|c| !c.is_whitespace() && *c != '/').collect();
    Some(token)
}

/// Text of the first element with this local name, namespace-agnostic.
fn first_text(doc: &roxmltree::Document<'_>, local_name: &str) -> Option<String> {
    doc.descendants()
        .find(|n| n.is_element() && n.tag_name().name() == local_name)
        .and_then(|n| n.text())
        .map(|t| t.trim().to_owned())
        .filter(|t| !t.is_empty())
}

fn split_once(bytes: &[u8], sep: u8) -> Option<(&[u8], &[u8])> {
    let i = bytes.iter().position(|&b| b == sep)?;
    Some((&bytes[..i], &bytes[i + 1..]))
}

fn quarantine(
    member_path: &str,
    bytes: &[u8],
    profile: Option<String>,
    reason: &str,
    detail: Option<String>,
) -> Record {
    Record::Quarantine(QuarantineRecord {
        content_hash: sha256_hex(bytes),
        profile,
        reason: reason.into(),
        detail,
        member_path: member_path.into(),
    })
}

fn one(record: Record) -> Disposition {
    Disposition::Records(vec![record])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_tokens_come_from_every_marker_shape() {
        // Root VERSION attribute (2019+), schemaLocation (2014), namespace (2019 R2.0.8).
        assert_eq!(
            version_token("http://publications.europa.eu/TED_schema/Export/R2.0.8.S02.E01 TED_EXPORT.xsd"),
            Some("R2.0.8.S02.E01".into())
        );
        assert_eq!(
            version_token("http://publications.europa.eu/resource/schema/ted/R2.0.9/publication"),
            Some("R2.0.9".into())
        );
        // 2011 files carry no marker anywhere.
        assert_eq!(version_token("http://publications.europa.eu/TED_schema/Export"), None);
    }

    #[test]
    fn text_era_names_carry_language_and_variant() {
        let n = text_era_member("EN_19930102_1993001_ISO_ORG.zip!EN_19930102_1993001_ISO_ORG").unwrap();
        assert_eq!((n.language.as_str(), n.variant.as_str()), ("EN", "ISO"));
        let n = text_era_member("en_20100102_001_utf8_org.zip!EN_20100102_2010001_UTF8_ORG").unwrap();
        assert_eq!((n.language.as_str(), n.variant.as_str()), ("EN", "UTF8"));
        // XML-era members must not be mistaken for text-era ones.
        assert!(text_era_member("20110104_001/000036_2011.xml").is_none());
    }

    #[test]
    fn record_markers_delimit_text_notices() {
        assert!(is_record_marker(b"1.00/067192\n"));
        assert!(is_record_marker(b"1.0/000006\r\n"));
        assert!(!is_record_marker(b"ND: 52472-1992\n"));
        assert!(!is_record_marker(b"  ***  T E D  ***\n"));
    }

    #[test]
    fn publication_ids_fall_back_to_the_file_name() {
        assert_eq!(
            publication_id_from_name("20240102_1/00001505_2024.xml"),
            Some("00001505-2024".into())
        );
        assert_eq!(publication_id_from_name("junk.txt"), None);
    }
}
