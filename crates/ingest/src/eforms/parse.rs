//! The exhaustive-consumption walker (ADR-0004).
//!
//! Document and match index are descended together. Every element, every
//! attribute and every text node must be claimed by an SDK node or field; the
//! first thing that is not aborts the notice with the path that was not
//! claimed. There is no partial result — a notice is parsed whole or
//! quarantined whole. A claimed value that does not convert is not a gap in
//! that sense: whether it costs the notice or only its typed form is the
//! field's STRICT/SOFT class ([`soft`], issue 433).

use std::collections::HashMap;

use store::{NoticeValue, Parsed, Section, ValueRow};

use super::index::{self, Branch, FieldInfo};
use super::sdk::Decision;
use super::value;

/// The notice root's section id — the same string change notices use to
/// address the procedure-level section (BT-13716).
const ROOT_SECTION: &str = "PROCEDURE";

/// Attributes that are XML plumbing rather than notice content, and so are
/// claimed without being mapped. Namespace declarations never reach here:
/// roxmltree reports them separately from attributes.
const IGNORED_ATTRIBUTES: [(&str, &str); 2] = [
    ("http://www.w3.org/2001/XMLSchema-instance", "schemaLocation"),
    ("http://www.w3.org/2001/XMLSchema-instance", "noNamespaceSchemaLocation"),
];

/// Attributes that qualify the value of the element carrying them — the code
/// list a code belongs to, the currency of an amount, and so on. The SDK models
/// most of these as fields in their own right (`attributeOf`), and [`value`]
/// consumes them either way.
///
/// The "either way" is load-bearing: published TED notices carry a few of these
/// where the SDK's inventory declares none — `cbc:NoticeLanguageCode/@listName`
/// and `efac:Changes/efbc:ChangedNoticeIdentifier/@schemeName` on almost every
/// notice, `cbc:CompanyID/@schemeID` (the ISO 6523 register that issued a
/// registration number) on many. Rejecting those would quarantine most of TED
/// over an SDK metadata gap; ignoring them silently would drop real data. They
/// are therefore claimed here *and* stored: `@listName` as a code's list,
/// `@schemeName`/`@schemeID` as an identifier's scheme.
/// `languageLocaleID` is the DÖE sdk-0.1 serializer's companion to
/// `languageID` (both carry the same language code in the wild); it qualifies
/// the text like the others and is consumed with it.
const VALUE_ATTRIBUTES: [&str; 9] = [
    "listName",
    "listID",
    "schemeName",
    "schemeID",
    "languageID",
    "languageLocaleID",
    "currencyID",
    "unitCode",
    // `@name` is a code's human-readable genericode display label — a redundant
    // echo of the code itself (publishers attach it to e.g. DocumentTypeCode).
    // Claimed as a value qualifier like the others (issue 18 mop-up).
    "name",
];

/// The date fields whose unconvertible value keeps its raw text instead of
/// quarantining the notice (issue 433) — each one read by NOTHING in the
/// fold, the resolver or a canonical view, so the typed value's absence
/// moves no served date. Exact ids, never a pattern: every other date stays
/// STRICT whether or not anything reads it. Strict is the class default
/// because the fold keys instants and dates (a silently absent
/// BT-145-Contract or BT-131 would move or drop a served date); an unread
/// date such as BT-127-notice is strict too until it is listed here.
///
/// - `BT-803(d)-notice` / `BT-803(t)-notice`: the eSender's
///   `efbc:TransmissionDate`/`Time` (issue 141's envelope stamp, claimed on
///   every minor by `index.rs`). Not the dispatch axis — that is BT-05(a)
///   (`project.rs` `DISPATCH_DATE_FIELDS`) — and in no fold table. 13
///   notices were held whole on a zoneless `2024-09-03` here (2026-09-27).
///   No zone is guessed: `/v1/sql` serves `notice_dates` verbatim, and an
///   assumed `Z` would read exactly like a published one.
/// - `DE1-TransmissionDate` / `DE1-TransmissionTime`: the SAME element pair
///   on eForms-DE 1.x notices. That dialect's inventory
///   (`fields-de-1.x.json`) declares the stamp under these ids, and
///   `index.rs` grafts the BT-803 ids gap-only, so the DE 1.x spelling is
///   the one this walk sees there. Read by nothing either (in neither
///   `DE1_FIELD_ALIASES` nor a date axis); listed so the rule follows the
///   element, not the dialect.
const SOFT_DATE_FIELDS: [&str; 4] =
    ["BT-803(d)-notice", "BT-803(t)-notice", "DE1-TransmissionDate", "DE1-TransmissionTime"];

/// Whether a value this field cannot convert costs only its typed form — the
/// element's raw text is kept as a text row under the same field id, "raw
/// kept, typed absent" — rather than the whole notice (issue 433).
///
/// Quarantine is for unconsumed structure, not low-quality values
/// (ted-legacy-mapping.md §8.2, which ADR-0004's amendment points to); the
/// r209 and text-era parsers already degrade exactly this way. Measured on
/// 2026-09-27: 326 notices held whole as `unrepresentable-value`, ~310 of
/// them on junk in an integer or indicator field (`_DEFAULT_VALUE_CHANGE_ME_`
/// as a BT-44 prize rank, `prima` as a BT-171 tender rank, `True` or `.00`,
/// a BT-113 of 10^40) — none of which the fold reads — each costing the
/// notice's buyer, lots, values and award.
///
/// SOFT: every `Integers` (integers and indicators) and `Numbers` field, and
/// the listed [`SOFT_DATE_FIELDS`]. The class is by type, so ONE soft field
/// is fold-read, by decision: BT-759-LotResult (and its DE 1.x source
/// `DE1-NoticeResult-LotResult-ReceivedSubmissionsStatistics-StatisticsNumeric`),
/// the received-submissions count. Junk there (`keine`) now keeps its raw
/// text and drops only that block's `tender_version_result_stats` row — the
/// fold emits a statistic solely as a (kind, count) pair (`project.rs`
/// `read_results`) — where it used to hold the whole notice. Statistics are
/// served per block and never summed, so the absent row reads as "not
/// published", not as a smaller total.
///
/// STRICT, by type and whether or not anything reads the field: every
/// `Amounts` field — the fold sums bid values (BT-720) into contract and
/// awarded values, so a silently absent one would under-report — and every
/// `Dates` field not listed. Unread strict ones (BT-710-LotResult,
/// BT-127-notice) keep quarantining. `Codes`, `Texts`, `Ids` and
/// `Classifications` cannot fail to convert.
fn soft(field: &FieldInfo) -> bool {
    matches!(field.decision, Decision::Integers | Decision::Numbers)
        || SOFT_DATE_FIELDS.contains(&field.id.as_str())
}

#[derive(Debug, PartialEq)]
pub struct Rejected {
    pub reason: &'static str,
    pub detail: String,
}

/// Parse one eForms notice against the field inventory its `CustomizationID`
/// declares — refined by `cbc:ProfileID` where one customization tracks two
/// EU SDK bases (eForms-DE 2.1; [`super::sdk::resolve`]).
pub fn parse(xml: &str, customization: &str) -> Result<Parsed, Rejected> {
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| Rejected { reason: "unparsable-xml", detail: e.to_string() })?;
    let profile_id = doc
        .root_element()
        .children()
        .find(|n| {
            n.is_element()
                && n.tag_name().name() == "ProfileID"
                && n.tag_name().namespace() == super::xpath::namespace("cbc")
        })
        .and_then(|n| n.text())
        .map(str::trim);
    let index = super::sdk::resolve(customization, profile_id)
        .and_then(index::for_customization)
        .ok_or_else(|| Rejected {
            reason: "unknown-customization",
            detail: format!("no vendored SDK metadata for {customization}"),
        })?;

    let mut walk = Walk {
        parsed: Parsed {
            sections: vec![Section { id: ROOT_SECTION.into(), kind: "Notice".into(), parent: None }],
            values: Vec::new(),
        },
        ordinals: HashMap::new(),
        anonymous: HashMap::new(),
    };
    let root = doc.root_element();
    walk.claim_attributes(root, &[index], "/")?;
    walk.children(root, &[index], ROOT_SECTION, "")?;
    Ok(walk.parsed)
}

struct Walk {
    parsed: Parsed,
    /// Repeat counter per (section, field), giving each value its ordinal.
    ordinals: HashMap<(String, String), i64>,
    /// Instance counter per SDK node id, for sections the notice does not name.
    anonymous: HashMap<String, usize>,
}

impl Walk {
    /// Visit every element child of `element`, matching each against `branch`.
    fn children(
        &mut self,
        element: roxmltree::Node<'_, '_>,
        branches: &[&Branch],
        section: &str,
        path: &str,
    ) -> Result<(), Rejected> {
        for child in element.children() {
            if child.is_element() {
                self.element(child, branches, section, path)?;
            } else if child.is_text()
                && !branches.iter().any(|b| b.field.is_some())
                && !child.text().unwrap_or_default().trim().is_empty()
            {
                // Text under an element the SDK does not declare a field for:
                // content that would otherwise be dropped silently.
                return Err(unclaimed("text", path));
            }
        }
        Ok(())
    }

    fn element(
        &mut self,
        element: roxmltree::Node<'_, '_>,
        parent_branches: &[&Branch],
        section: &str,
        parent_path: &str,
    ) -> Result<(), Rejected> {
        let path = format!("{parent_path}/{}", qualified(element));
        let mut branches: Vec<&Branch> =
            parent_branches.iter().flat_map(|b| b.select(element)).collect();
        // Relaxed claim: the element is known here by name, but the notice
        // omits (or misspells) the discriminator the SDK's predicates key on.
        // It is claimed as a container; binding a value below stays exact.
        let relaxed = branches.is_empty();
        if relaxed {
            branches = parent_branches.iter().flat_map(|b| b.select_by_name(element)).collect();
        }
        if branches.is_empty() {
            return Err(unclaimed("element", &path));
        }
        if branches.iter().any(|b| b.ignored.is_some()) {
            return Ok(());
        }
        self.claim_attributes(element, &branches, &path)?;

        // A repeatable node (or a privacy block) opens a section of its own, so
        // repeated siblings stay distinguishable and their values stay grouped.
        let section = match branches.iter().find_map(|b| b.node.as_ref()) {
            Some(node) => {
                let id = self.section_id(element, node);
                match self.parsed.sections.iter().find(|s| s.id == id) {
                    // Publishers re-publish a section id (issue 201, 23
                    // members): the DÖE sdk-1.0 serializer repeats the whole
                    // Organization block once per beneficial owner, and a 2024
                    // eSender registers the same org twice, full then sparse.
                    // Same id + same kind is the same entity, so the repeat
                    // MERGES — its values append to the existing section.
                    Some(existing) if existing.kind == node.kind => {}
                    // A colliding id of a DIFFERENT kind is a real anomaly:
                    // merging would file one entity's values under another's
                    // label, so the member still quarantines loudly.
                    Some(_) => {
                        return Err(Rejected {
                            reason: "duplicate-section-id",
                            detail: format!("{id} is published twice, at {path}"),
                        });
                    }
                    None => self.parsed.sections.push(Section {
                        id: id.clone(),
                        kind: node.kind.clone(),
                        parent: Some(section.to_owned()),
                    }),
                }
                id
            }
            None => section.to_owned(),
        };

        if let Some(field) = branches.iter().find_map(|b| b.field.as_ref()) {
            // Under a relaxed claim the candidates may name different business
            // terms for the same element — exactly the ambiguity the missing
            // discriminator caused. Storing either would be a guess, so the
            // notice is quarantined and says so.
            if relaxed && branches.iter().filter_map(|b| b.field.as_ref()).any(|f| f.id != field.id) {
                let mut candidates: Vec<&str> =
                    branches.iter().filter_map(|b| b.field.as_ref()).map(|f| f.id.as_str()).collect();
                candidates.dedup();
                return Err(Rejected {
                    reason: "ambiguous-field",
                    detail: format!("{path} could be any of {candidates:?}"),
                });
            }
            self.value(element, parent_branches, field, &section, &path)?;
        }
        self.children(element, &branches, &section, &path)
    }

    /// Every attribute must be an SDK attribute-field or explicit plumbing.
    fn claim_attributes(
        &self,
        element: roxmltree::Node<'_, '_>,
        branches: &[&Branch],
        path: &str,
    ) -> Result<(), Rejected> {
        for attr in element.attributes() {
            let claimed = branches.iter().any(|b| b.attributes.contains_key(attr.name()))
                || (branches.iter().any(|b| b.field.is_some())
                    && VALUE_ATTRIBUTES.contains(&attr.name()))
                || IGNORED_ATTRIBUTES
                    .iter()
                    .any(|&(ns, name)| attr.namespace() == Some(ns) && attr.name() == name);
            if !claimed {
                return Err(unclaimed("attribute", &format!("{path}/@{}", attr.name())));
            }
        }
        Ok(())
    }

    fn value(
        &mut self,
        element: roxmltree::Node<'_, '_>,
        parent_branches: &[&Branch],
        field: &FieldInfo,
        section: &str,
        path: &str,
    ) -> Result<(), Rejected> {
        // eForms splits a deadline into two sibling elements — a `date` field
        // and a `time` field — which are one instant. SDK 1.15 added
        // `dateFieldId`/`timeFieldId` to say so, but 1.12–1.14 do not carry it,
        // so the pair is recognised by UBL's own naming convention instead:
        // `IssueDate`/`IssueTime`, `EndDate`/`EndTime`, `StartDate`/`StartTime`.
        // The date claims the time; the time then emits no row of its own.
        let paired = counterpart(element, parent_branches, field);
        let text = element.text().unwrap_or_default();
        if field.kind == "time" {
            if let Some((date_field, date)) = paired {
                // Unless the pair does not convert and its DATE is soft (issue
                // 433): the date then keeps its raw text, and this clock keeps
                // its own under its own id, so neither published half is
                // dropped. A strict date's failure quarantines the notice
                // whatever this half does.
                if soft(date_field) && value::timestamp_for(date_field, date.trim(), Some(text)).is_err() {
                    self.keep_raw(section, field, text);
                }
                return Ok(());
            }
        }

        let value = match (&paired, field.kind.as_str()) {
            (Some((_, time)), "date") => value::timestamp_for(field, text.trim(), Some(time))
                .map(Some)
                .map_err(|e| value::Error(format!("{}: {e}", field.id))),
            _ => value::convert(field, text, |name| element.attribute(name).map(str::to_owned)),
        };
        let value = match value {
            Ok(Some(value)) => value,
            Ok(None) => return Ok(()),
            // A soft field's failure costs only the typed value; the raw text
            // stays under the same id and ordinal sequence (issue 433). For a
            // pair the date field's class decides, as its id already governs
            // the pair's zoneless readings (`value::timestamp_for`).
            Err(_) if soft(field) => {
                self.keep_raw(section, field, text);
                return Ok(());
            }
            Err(e) => {
                return Err(Rejected { reason: "unrepresentable-value", detail: format!("{} at {path}", e.0) });
            }
        };
        self.push(section, field, value);
        Ok(())
    }

    /// "Raw kept, typed absent" (issue 433; ted-legacy-mapping.md §8.2): the
    /// trimmed element text as a text row under the field's own id. The table
    /// it lands in is the marker — a text row under an integer or date field
    /// id says the published value did not convert — so no synthetic id and
    /// no schema change is needed, and `/v1/notices/{id}/content` and
    /// `/v1/sql` serve it as published. Whitespace alone carries no value,
    /// exactly as an empty element does (`value::convert`).
    fn keep_raw(&mut self, section: &str, field: &FieldInfo, text: &str) {
        let text = text.trim();
        if !text.is_empty() {
            self.push(section, field, NoticeValue::Text { lang: None, value: text.to_owned() });
        }
    }

    fn push(&mut self, section: &str, field: &FieldInfo, value: NoticeValue) {
        let ordinal = self.ordinals.entry((section.to_owned(), field.id.clone())).or_insert(-1);
        *ordinal += 1;
        self.parsed.values.push(ValueRow {
            section_id: section.to_owned(),
            field_id: field.id.clone(),
            ordinal: *ordinal,
            value,
        });
    }

    /// The identifier the notice published for this section instance, or a
    /// synthetic one where the SDK node has no identifier field.
    fn section_id(&mut self, element: roxmltree::Node<'_, '_>, node: &index::NodeInfo) -> String {
        let published = node.identifier.as_ref().and_then(|path| {
            path.select(element).first().and_then(|n| n.text()).map(str::trim).filter(|t| !t.is_empty())
        });
        match published {
            Some(id) => id.to_owned(),
            None => {
                let n = self.anonymous.entry(node.id.clone()).or_insert(0);
                *n += 1;
                format!("{}#{}", node.id, *n - 1)
            }
        }
    }
}

/// The field and text of the element pairing with this one under UBL's
/// `…Date`/`…Time` naming, when both are fields of the matching SDK type.
/// The field travels with the text so the time half can ask whether its
/// DATE's field is soft (issue 433).
fn counterpart<'a, 'b>(
    element: roxmltree::Node<'a, '_>,
    parent_branches: &[&'b Branch],
    field: &FieldInfo,
) -> Option<(&'b FieldInfo, &'a str)> {
    let (own, other) = match field.kind.as_str() {
        "date" => ("Date", "Time"),
        "time" => ("Time", "Date"),
        _ => return None,
    };
    let stem = element.tag_name().name().strip_suffix(own)?;
    let wanted = format!("{stem}{other}");
    let (sibling, sibling_field) = element
        .parent_element()?
        .children()
        .filter(|c| c.is_element() && c.tag_name().name() == wanted)
        .find_map(|c| {
            parent_branches
                .iter()
                .flat_map(|b| b.select(c))
                .filter_map(|b| b.field.as_ref())
                .find(|f| f.kind == other.to_lowercase())
                .map(|f| (c, f))
        })?;
    Some((sibling_field, sibling.text()?))
}

fn qualified(element: roxmltree::Node<'_, '_>) -> String {
    match element.tag_name().namespace() {
        Some(ns) => format!("{{{ns}}}{}", element.tag_name().name()),
        None => element.tag_name().name().to_owned(),
    }
}

fn unclaimed(what: &str, path: &str) -> Rejected {
    Rejected { reason: "unclaimed-content", detail: format!("unclaimed {what} at {path}") }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(id: &str, decision: Decision, kind: &str) -> FieldInfo {
        FieldInfo { id: id.into(), decision, kind: kind.into(), code_list: None }
    }

    /// Issue 433's STRICT/SOFT split, pinned per class: every integer,
    /// indicator and number is soft; amounts and every date but the listed
    /// unread eSender stamp (both dialects' spellings) stay strict.
    #[test]
    fn soft_is_every_count_and_only_the_listed_dates() {
        for soft_field in [
            field("BT-44-Lot", Decision::Integers, "integer"),
            field("BT-113-Lot", Decision::Integers, "integer"),
            field("BT-661-Lot", Decision::Integers, "indicator"),
            field("BT-33-Procedure", Decision::Numbers, "number"),
            // The one fold-read soft field, soft BY DECISION (see `soft`): a
            // junk count drops its statistics row, not the notice.
            field("BT-759-LotResult", Decision::Numbers, "number"),
            field(
                "DE1-NoticeResult-LotResult-ReceivedSubmissionsStatistics-StatisticsNumeric",
                Decision::Numbers,
                "number",
            ),
            field("BT-803(d)-notice", Decision::Dates, "date"),
            field("BT-803(t)-notice", Decision::Dates, "time"),
            field("DE1-TransmissionDate", Decision::Dates, "date"),
            field("DE1-TransmissionTime", Decision::Dates, "time"),
        ] {
            assert!(soft(&soft_field), "{} is soft", soft_field.id);
        }
        for strict_field in [
            field("BT-720-Tender", Decision::Amounts, "amount"),
            // Strict by type, not by reader: nothing in the fold reads these.
            field("BT-710-LotResult", Decision::Amounts, "amount"),
            field("BT-127-notice", Decision::Dates, "date"),
            field("BT-145-Contract", Decision::Dates, "date"),
            field("BT-131(d)-Lot", Decision::Dates, "date"),
            field("BT-05(a)-notice", Decision::Dates, "date"),
            field("DE1-IssueDate", Decision::Dates, "date"),
            // An exact id list, not a stem: a look-alike stays strict.
            field("BT-803-notice", Decision::Dates, "date"),
        ] {
            assert!(!soft(&strict_field), "{} is strict", strict_field.id);
        }
    }
}
