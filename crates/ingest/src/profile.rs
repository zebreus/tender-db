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
use std::borrow::Cow;

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
    /// Byte range of this record within the member, for members that carry
    /// several records (text era). `None` = the whole member is the payload.
    pub span: Option<(usize, usize)>,
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

/// Package-level facts a single member cannot know but dispatch policy needs.
/// Built from the tar-level entry names before the walk
/// (`package::entry_names`).
#[derive(Default)]
pub struct PackageContext {
    /// The package also ships the English UTF8 text-era variant. Mid-era
    /// dailies (~2004–2007) carry the same English delivery twice, as
    /// `_ISO_` and `_UTF8_`; the ISO rendering is lossy (non-Latin-1 scripts
    /// mangled — measured on the 2005 daily), so UTF8 supersedes it.
    pub en_utf8_text: bool,
    /// Same fact for the `_CF<n>` companion delivery (issue 181): the
    /// supersedence decision is PER DELIVERY CLASS, because a package can
    /// carry a UTF8 main delivery without a UTF8 companion — one flag for
    /// both would silently drop the companion's ISO rendering with nothing
    /// superseding it.
    pub en_utf8_cf: bool,
}

impl PackageContext {
    pub fn from_entry_names<S: AsRef<str>>(names: &[S]) -> Self {
        let (mut en_utf8_text, mut en_utf8_cf) = (false, false);
        for n in names {
            let stem = n.as_ref().rsplit('/').next().unwrap_or(n.as_ref());
            let stem = stem
                .strip_suffix(".zip")
                .or_else(|| stem.strip_suffix(".ZIP"))
                .unwrap_or(stem);
            if let Some(m) = text_era_member(stem)
                // A `CS<n>` correction sheet is neither delivery class: a
                // `en_…_utf8_cs1.txt` must not count as the UTF8 main
                // delivery and sweep the ISO ORG member (issue 180).
                && !m.correction
                && m.language.eq_ignore_ascii_case("en")
                && m.variant.eq_ignore_ascii_case("utf8")
            {
                if m.companion {
                    en_utf8_cf = true;
                } else {
                    en_utf8_text = true;
                }
            }
        }
        Self { en_utf8_text, en_utf8_cf }
    }
}

/// Classify one package member, without package-level context (single-file
/// callers; equivalent to a package that ships nothing else).
pub fn dispatch(member_path: &str, bytes: &[u8]) -> Disposition {
    dispatch_with(member_path, bytes, &PackageContext::default())
}

/// Classify one package member.
pub fn dispatch_with(member_path: &str, bytes: &[u8], ctx: &PackageContext) -> Disposition {
    // The text era is recognised by member naming, before any XML attempt: its
    // payloads are not XML, and its `_meta_` sibling variant *is* XML but is a
    // duplicate representation we do not ingest.
    if let Some(name) = text_era_member(member_path) {
        return dispatch_text(member_path, &name, bytes, ctx);
    }

    let Ok(xml) = std::str::from_utf8(bytes) else {
        return one(quarantine(member_path, bytes, None, "not-utf8", None));
    };
    // Strip any `<!DOCTYPE …>` before parsing: roxmltree refuses every DTD, and
    // the 2008 OPOCE `INTERNAL_OJS` export is real notices behind a DTD (issue
    // 36). The strip is XXE-safe — see `strip_doctype`; it never processes a DTD,
    // it only lets the plain body be parsed-or-refused normally.
    let stripped = strip_doctype(xml);
    let doc = match roxmltree::Document::parse(&stripped) {
        Ok(doc) => doc,
        Err(e) => return one(quarantine(member_path, bytes, None, "unparsable-xml", Some(e.to_string()))),
    };
    let root = doc.root_element();
    let ns = root.tag_name().namespace().unwrap_or_default();
    let local = root.tag_name().name();

    if local == "TED_EXPORT" {
        one(dispatch_ted_export(member_path, bytes, &root, ns))
    } else if local == "INTERNAL_OJS" {
        dispatch_internal_ojs(member_path, bytes, &root)
    } else if EFORMS_ROOT_NS.iter().any(|family| ns.starts_with(family)) {
        one(dispatch_eforms(member_path, bytes, &doc))
    } else {
        one(quarantine(member_path, bytes, None, "unknown-root", Some(format!("{{{ns}}}{local}"))))
    }
}

/// Remove an XML `<!DOCTYPE …>` declaration, internal subset and all, so a
/// DTD-bearing document can reach [`roxmltree`], which refuses any DTD outright.
///
/// This is deliberately a *strip*, never DTD processing: no entity — parameter,
/// internal general, or external — is ever defined or expanded. A legitimate
/// payload (the OPOCE `INTERNAL_OJS` notices, whose only DTD content is an unused
/// parameter entity) parses from its plain body; a hostile payload that defines
/// an internal general entity and references it in the body is left with an
/// *undefined* entity reference, which roxmltree then refuses. So stripping can
/// never enable an XXE fetch or a billion-laughs expansion — it only turns a
/// blanket "DTD present" refusal into a normal parse-or-refuse of the body.
pub(crate) fn strip_doctype(xml: &str) -> Cow<'_, str> {
    let Some(start) = xml.find("<!DOCTYPE") else { return Cow::Borrowed(xml) };
    let rest = &xml[start..];
    let first_gt = rest.find('>');
    // An internal subset `[ … ]` may itself contain `>` (inside entity values or
    // declarations), so when a `[` opens before the first `>`, the declaration
    // really ends at the `>` after the subset's closing `]`.
    let end = match (first_gt, rest.find('[')) {
        (Some(gt), Some(br)) if br < gt => {
            rest[br..].find(']').and_then(|c| rest[br + c..].find('>').map(|g| br + c + g))
        }
        (gt, _) => gt,
    };
    // No closing `>` at all: leave it for roxmltree to reject as malformed.
    let Some(end) = end else { return Cow::Borrowed(xml) };
    Cow::Owned(format!("{}{}", &xml[..start], &xml[start + end + 1..]))
}

/// The 2008 OPOCE internal export (DTD R2.0.5): real S-series notices, one file
/// per language for ~28k notices — the whole 2008 verify gap (issue 41). The
/// per-language siblings are the same notice; EN is ingested and the rest are
/// skipped as documented duplicates, the text era's language policy. Identity is
/// the member's `<doc>_<year>` stem in `<doc>-<year>` form, the same key the
/// text channel uses for 2008 (issue 36 confirmed no text twin).
fn dispatch_internal_ojs(member_path: &str, bytes: &[u8], root: &roxmltree::Node<'_, '_>) -> Disposition {
    let Some((publication_id, lang)) = internal_ojs_identity(member_path) else {
        return one(quarantine(
            member_path,
            bytes,
            Some(crate::internal_ojs::PROFILE.into()),
            "missing-publication-id",
            None,
        ));
    };
    if !lang.eq_ignore_ascii_case("en") {
        return Disposition::Skipped("internal-ojs-non-english");
    }
    // R2.0.5 is fixed for the era by the DTD; the form root also carries it as a
    // VERSION attribute, which is the honest per-payload reading.
    let declared_version =
        root.descendants().find_map(|n| n.attribute("VERSION")).map(str::to_owned);
    one(Record::Notice(NoticeRecord {
        publication_id,
        content_hash: sha256_hex(bytes),
        profile: crate::internal_ojs::PROFILE.into(),
        declared_version,
        member_path: member_path.into(),
        span: None,
    }))
}

/// `…/114238_2008.en` → (`114238-2008`, `en`): the publication id in
/// `<doc>-<year>` form and the file's language (its extension).
fn internal_ojs_identity(member_path: &str) -> Option<(String, String)> {
    let name = member_path.rsplit(['!', '/']).next()?;
    let (stem, lang) = name.rsplit_once('.')?;
    let (number, year) = stem.split_once('_')?;
    let ok = !number.is_empty()
        && number.bytes().all(|b| b.is_ascii_digit())
        && year.len() == 4
        && year.bytes().all(|b| b.is_ascii_digit())
        && lang.len() == 2
        && lang.bytes().all(|b| b.is_ascii_alphabetic());
    ok.then(|| (format!("{number}-{year}"), lang.to_owned()))
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
            span: None,
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

    // efbc:NoticePublicationID (`00001505-2024`); present on every TED eForms
    // file in the samples. The file name carries the same number, so it is the
    // fallback rather than a second source of truth. DÖE exports carry neither
    // (`efac:Publication` is TED-side metadata), so their identity is the
    // notice id plus its declared version — reliable on DÖE, per
    // docs/research/eforms-de-profile.md — which is also the member file
    // name's stem (`<uuid|numeric>-<version>.xml`).
    let publication_id = first_text(doc, "NoticePublicationID")
        .or_else(|| publication_id_from_name(member_path))
        .or_else(|| notice_id_and_version(doc));
    match publication_id {
        Some(id) => Record::Notice(NoticeRecord {
            publication_id: id,
            content_hash: sha256_hex(bytes),
            profile,
            declared_version: Some(customization),
            member_path: member_path.into(),
            span: None,
        }),
        None => quarantine(member_path, bytes, Some(profile), "missing-publication-id", None),
    }
}

/// Tagged-text era (1993–2010). One member is the whole day's English document:
/// notices separated by a `1.00/067192`-style record marker, each carrying its
/// OJS notice number on an `ND:` line. Records are split out so a Notice stays
/// one publication event; the file is not otherwise parsed.
fn dispatch_text(
    member_path: &str,
    name: &TextEraName,
    bytes: &[u8],
    ctx: &PackageContext,
) -> Disposition {
    // `CS<n>` correction sheets (issue 180): per-language duplicates of tiny
    // `ND:/FLD:/OLD:/NEW:` field-correction records — provably non-notice
    // (no notice bodies; measured across the 2000–2009 span). Skipped by
    // class, before the language policy, so the EN sheets resolve under the
    // same name as the rest. The correction content stays in the raw archive;
    // applying it to stored notices would be its own feature (issue 180).
    if name.correction {
        return Disposition::Skipped("text-era-correction-sheet");
    }
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
    // Mid-era dailies ship the English delivery in both encodings; ingesting
    // both would also double every notice, and the ISO rendering is the lossy
    // one (see [`PackageContext::en_utf8_text`]). Judged per delivery class:
    // an ISO companion is only superseded by a UTF8 COMPANION (issue 181).
    let utf8_twin = if name.companion { ctx.en_utf8_cf } else { ctx.en_utf8_text };
    if name.variant.eq_ignore_ascii_case("iso") && utf8_twin {
        return Disposition::Skipped("text-era-iso-superseded-by-utf8");
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
                span: Some((start, end)),
            }),
            None => quarantine(&path, record, Some("text".into()), "missing-publication-id", None),
        });
    }
    Disposition::Records(records)
}

/// Text-era member naming, e.g. `EN_19930102_1993001_ISO_ORG.zip!EN_19930102_1993001_ISO_ORG`
/// or `en_20100102_001_utf8_org.zip!EN_20100102_2010001_UTF8_ORG`. The last
/// part is the delivery class: `ORG` is the main daily delivery, `CF<n>` its
/// companion files (issue 181) — the SAME record format carrying additional or
/// republished notices, shipped per language/variant exactly like ORG. Before
/// they were recognised here, every companion member fell through to the XML
/// path and quarantined whole (~4.1k rows as not-utf8 / unknown-root /
/// unparsable-xml, depending on variant).
struct TextEraName {
    language: String,
    variant: String,
    /// A `CF<n>` companion member rather than the main `ORG` delivery.
    companion: bool,
    /// A `CS<n>` correction sheet (issue 180): plain-text `ND:/FLD:/OLD:/NEW:`
    /// field-correction records referencing earlier notices — no notice bodies.
    correction: bool,
}

fn text_era_member(member_path: &str) -> Option<TextEraName> {
    let name = member_path.rsplit(['!', '/']).next()?;
    let parts: Vec<&str> = name.split('_').collect();
    // <lg>_<date>_<issue>_<variant>_<ORG|CFn|Cnn>: `CF<n>` is the 2000+
    // companion naming, `C<nn>` its 1999 predecessor (same record format,
    // measured on the 1999-09 daily). The `CS<n>` correction sheets ship
    // unzipped with the extension in the member name, and before ~2004 with
    // no variant token at all (`DA_20000112_007_CS1.TXT`), so the class token
    // is extension-stripped and the variant is optional for that class only.
    let (language, date, variant, class) = match parts[..] {
        [language, date, _issue, variant, class] => (language, date, Some(variant), class),
        [language, date, _issue, class] => (language, date, None, class),
        _ => return None,
    };
    let class = class
        .strip_suffix(".TXT")
        .or_else(|| class.strip_suffix(".txt"))
        .unwrap_or(class);
    let classified = |prefix: &str| {
        class.len() > prefix.len()
            && class[..prefix.len()].eq_ignore_ascii_case(prefix)
            && class[prefix.len()..].bytes().all(|b| b.is_ascii_digit())
    };
    let correction = classified("cs");
    // `COR` (no ordinal, 1999) is a corrected re-issue of the whole day's
    // delivery: the same record set as ORG with a handful of records fixed
    // (measured on 1999-07-10: 623 records, 3 differing). It dispatches like
    // a companion — the unchanged records dedupe by content hash, the
    // corrected ones become new versions of their notices.
    let companion =
        !correction && (classified("cf") || classified("c") || class.eq_ignore_ascii_case("cor"));
    let ok = language.len() == 2
        && language.chars().all(|c| c.is_ascii_alphabetic())
        && date.len() == 8
        && date.chars().all(|c| c.is_ascii_digit())
        && match variant {
            Some(v) => (class.eq_ignore_ascii_case("org") || companion || correction)
                && ["iso", "utf8", "meta"].iter().any(|x| x.eq_ignore_ascii_case(v)),
            // Only the correction sheets ever ship without a variant token.
            None => correction,
        };
    ok.then(|| TextEraName {
        language: language.into(),
        variant: variant.unwrap_or_default().into(),
        companion,
        correction,
    })
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

/// `<root cbc:ID>-<root cbc:VersionID>`, from the document itself — the DÖE
/// notice-version identity. Root-level children only: nested elements carry
/// their own `ID`s.
fn notice_id_and_version(doc: &roxmltree::Document<'_>) -> Option<String> {
    let child = |name: &str| {
        doc.root_element()
            .children()
            .find(|n| n.is_element() && n.tag_name().name() == name)
            .and_then(|n| n.text())
            .map(str::trim)
            .filter(|t| !t.is_empty())
    };
    Some(format!("{}-{}", child("ID")?, child("VersionID")?))
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
        assert_eq!((n.language.as_str(), n.variant.as_str(), n.companion), ("EN", "ISO", false));
        let n = text_era_member("en_20100102_001_utf8_org.zip!EN_20100102_2010001_UTF8_ORG").unwrap();
        assert_eq!((n.language.as_str(), n.variant.as_str(), n.companion), ("EN", "UTF8", false));
        // XML-era members must not be mistaken for text-era ones.
        assert!(text_era_member("20110104_001/000036_2011.xml").is_none());
    }

    /// Issue 181: the `CF<n>` companion delivery is the text era too — the same
    /// record format per language/variant. Before it was recognised, every
    /// companion member quarantined whole via the XML path.
    #[test]
    fn text_era_names_recognise_companion_files() {
        let n = text_era_member("EN_20030124_017_ISO_CF1.ZIP!EN_20030124_2003017_ISO_CF1").unwrap();
        assert_eq!((n.language.as_str(), n.variant.as_str(), n.companion), ("EN", "ISO", true));
        let n = text_era_member("bg_20070905_170_meta_cf1.zip!BG_20070905_2007170_META_CF1").unwrap();
        assert_eq!((n.variant.as_str(), n.companion), ("META", true));
        let n = text_era_member("DA_20030124_017_ISO_CF3.ZIP!DA_20030124_2003017_ISO_CF3").unwrap();
        assert!(n.companion);
        // The 1999 predecessor naming: C<nn> rather than CF<n>.
        let n = text_era_member("EN_19990901_169_ISO_C01.ZIP!EN_19990901_1999169_ISO_C01").unwrap();
        assert!(n.companion);
        // The 1999 corrected re-issue of a whole day (issue 180): same record
        // set as ORG with a few records fixed — a companion-style delivery.
        let n = text_era_member("EN_19990710_132_ISO_COR.ZIP!EN_19990710_1999132_ISO_COR").unwrap();
        assert!(n.companion && !n.correction);
        // 'CF' with no ordinal, or non-digits after it, is not the pattern.
        assert!(text_era_member("EN_20030124_017_ISO_CF.ZIP!EN_20030124_2003017_ISO_CF").is_none());
        assert!(text_era_member("EN_20030124_017_ISO_CFX.ZIP!EN_20030124_2003017_ISO_CFX").is_none());
    }

    /// Issue 181: companion dispatch inherits the text policies — meta and
    /// non-English skip; ISO is superseded only by a UTF8 COMPANION, never by
    /// the main delivery's UTF8 twin.
    #[test]
    fn companion_members_dispatch_with_class_aware_policies() {
        let rec = b"1.0/000001\nTI: X\nND: 99999-2003\n";
        // Meta companion: the documented duplicate-representation skip (the
        // language policy fires first, so this is the ENGLISH meta member).
        assert!(matches!(
            dispatch("en_20070905_170_meta_cf1.zip!EN_20070905_2007170_META_CF1", rec),
            Disposition::Skipped("text-era-meta-variant")
        ));
        // Non-English companion: the language policy.
        assert!(matches!(
            dispatch("FR_20030124_017_ISO_CF1.ZIP!FR_20030124_2003017_ISO_CF1", rec),
            Disposition::Skipped("text-era-non-english")
        ));
        // English ISO companion in a package whose MAIN delivery has UTF8 but
        // whose companion does not: NOT superseded — it dispatches as records.
        let ctx = PackageContext::from_entry_names(&[
            "EN_20040603_107_UTF8_ORG.ZIP",
            "EN_20040603_107_ISO_CF1.ZIP",
        ]);
        assert!(ctx.en_utf8_text && !ctx.en_utf8_cf);
        assert!(matches!(
            dispatch_with("EN_20040603_107_ISO_CF1.ZIP!EN_20040603_2004107_ISO_CF1", rec, &ctx),
            Disposition::Records(_)
        ));
        // With a UTF8 companion present, the ISO companion IS superseded.
        let ctx = PackageContext::from_entry_names(&[
            "EN_20040603_107_UTF8_CF1.ZIP",
            "EN_20040603_107_ISO_CF1.ZIP",
        ]);
        assert!(ctx.en_utf8_cf);
        assert!(matches!(
            dispatch_with("EN_20040603_107_ISO_CF1.ZIP!EN_20040603_2004107_ISO_CF1", rec, &ctx),
            Disposition::Skipped("text-era-iso-superseded-by-utf8")
        ));
        // And the English UTF8 companion itself yields records.
        assert!(matches!(
            dispatch("EN_20040603_107_UTF8_CF1.ZIP!EN_20040603_2004107_UTF8_CF1", rec),
            Disposition::Records(_)
        ));
    }

    /// Issue 180: `CS<n>` correction sheets — tiny per-language
    /// `ND:/FLD:/OLD:/NEW:` field-correction files, no notice bodies. They are
    /// recognised across both naming eras and skipped by class; before this,
    /// all 4,441 of them quarantined as `unparsable-xml: unknown token at 1:1`.
    #[test]
    fn correction_sheets_are_skipped_by_class() {
        // 2000-era shape: no variant token, uppercase, extension in the name.
        let n = text_era_member("DA_20000112_007_CS1.TXT").unwrap();
        assert!(n.correction && !n.companion);
        assert_eq!((n.language.as_str(), n.variant.as_str()), ("DA", ""));
        // 2009-era shape: variant token, lowercase.
        let n = text_era_member("sv_20090820_159_utf8_cs1.txt").unwrap();
        assert!(n.correction && !n.companion);
        // Higher ordinals are the same class.
        assert!(text_era_member("EN_20030124_017_CS3.TXT").unwrap().correction);
        // The class is CS + ordinal; a bare CS or non-digits are not it, and a
        // variant-less name is ONLY ever a correction sheet.
        assert!(text_era_member("EN_20030124_017_CS.TXT").is_none());
        assert!(text_era_member("EN_20030124_017_ORG.TXT").is_none());

        // Skipped by class — before the language policy, English included.
        let sheet = b"ND: 3261-2000\nFLD: RN\nOLD: 99-010933-002\nNEW: \n";
        assert!(matches!(
            dispatch("EN_20000112_007_CS1.TXT", sheet),
            Disposition::Skipped("text-era-correction-sheet")
        ));
        assert!(matches!(
            dispatch("DA_20000112_007_CS1.TXT", sheet),
            Disposition::Skipped("text-era-correction-sheet")
        ));

        // A correction sheet never counts as the UTF8 delivery: the ISO main
        // member must not be swept by a `en_…_utf8_cs1.txt` sibling.
        let ctx =
            PackageContext::from_entry_names(&["en_20090820_159_utf8_cs1.txt", "EN_20090820_159_ISO_ORG.ZIP"]);
        assert!(!ctx.en_utf8_text && !ctx.en_utf8_cf);
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

    #[test]
    fn strip_doctype_removes_the_declaration_and_only_it() {
        // No DTD: borrowed, untouched.
        assert_eq!(strip_doctype("<a>x [y] z</a>"), "<a>x [y] z</a>");
        // External-id DTD: gone, body kept (incl. a later '[' in the body).
        assert_eq!(
            strip_doctype("<!DOCTYPE r SYSTEM \"r.dtd\"><r>a [b] c</r>"),
            "<r>a [b] c</r>"
        );
        // Internal subset whose entity value contains '>': the whole subset goes.
        assert_eq!(
            strip_doctype("<?xml version=\"1.0\"?><!DOCTYPE r [<!ENTITY % t 'a>b'>]><r/>"),
            "<?xml version=\"1.0\"?><r/>"
        );
    }

    #[test]
    fn internal_ojs_english_member_dispatches_as_a_notice() {
        // A real 2008 OPOCE DTD notice (issue 41): the DTD is stripped, the body
        // parses, and its INTERNAL_OJS root now routes to a Notice of the
        // `internal-ojs` profile — identity is the member's <doc>-<year> stem.
        let bytes = include_bytes!("../tests/fixtures/internal_ojs/114238_2008.en");
        let Disposition::Records(records) =
            dispatch("20080502_2008085.tar.gz/114238/opoce-input/114238_2008.en", bytes)
        else {
            panic!("INTERNAL_OJS member was not dispatched to a record");
        };
        let [Record::Notice(n)] = &records[..] else { panic!("expected one notice") };
        assert_eq!(n.publication_id, "114238-2008");
        assert_eq!(n.profile, "internal-ojs");
        assert_eq!(n.declared_version.as_deref(), None); // the EEIG form root has no VERSION
    }

    #[test]
    fn internal_ojs_non_english_siblings_are_skipped() {
        // The ~22 per-language siblings are the same notice; only EN is ingested
        // (the text-era language policy), the rest are documented duplicate skips.
        let bytes = include_bytes!("../tests/fixtures/internal_ojs/114238_2008.en");
        let path = "20080502_2008085.tar.gz/114238/opoce-input/114238_2008.fr";
        assert!(matches!(dispatch(path, bytes), Disposition::Skipped("internal-ojs-non-english")));
    }

    #[test]
    fn a_hostile_dtd_is_refused_not_expanded() {
        // Stripping the DOCTYPE must never enable XXE or entity expansion. Both of
        // these define an internal general entity and reference it in the body;
        // once the DTD is stripped the reference is undefined, so roxmltree
        // refuses the document — it is quarantined, never resolved.
        let external = br#"<?xml version="1.0"?><!DOCTYPE INTERNAL_OJS [<!ENTITY xxe SYSTEM "file:///etc/passwd">]><INTERNAL_OJS>&xxe;</INTERNAL_OJS>"#;
        let billion = br#"<?xml version="1.0"?><!DOCTYPE lolz [<!ENTITY a "aa"><!ENTITY b "&a;&a;&a;">]><INTERNAL_OJS>&b;</INTERNAL_OJS>"#;
        for payload in [external.as_slice(), billion.as_slice()] {
            let Disposition::Records(records) =
                dispatch("20080502_2008085.tar.gz/999999/opoce-input/999999_2008.en", payload)
            else {
                panic!("hostile payload was not dispatched to a record");
            };
            let [Record::Quarantine(q)] = &records[..] else { panic!("expected one quarantine") };
            assert_eq!(q.reason, "unparsable-xml", "a hostile entity ref must be refused");
        }
    }
}
