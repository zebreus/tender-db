//! The exhaustive-consumption walker (ADR-0004).
//!
//! Document and match index are descended together. Every element, every
//! attribute and every text node must be claimed by an SDK node or field; the
//! first thing that is not aborts the notice with the path that was not
//! claimed. There is no partial result — a notice is parsed whole or
//! quarantined whole.

use std::collections::HashMap;

use store::{Parsed, Section, ValueRow};

use super::index::{self, Branch, FieldInfo};
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
const VALUE_ATTRIBUTES: [&str; 7] =
    ["listName", "listID", "schemeName", "schemeID", "languageID", "currencyID", "unitCode"];

#[derive(Debug, PartialEq)]
pub struct Rejected {
    pub reason: &'static str,
    pub detail: String,
}

/// Parse one eForms notice against the SDK version its `CustomizationID`
/// declares.
pub fn parse(xml: &str, customization: &str) -> Result<Parsed, Rejected> {
    let Some(index) = index::for_customization(customization) else {
        return Err(Rejected {
            reason: "unknown-customization",
            detail: format!("no vendored SDK metadata for {customization}"),
        });
    };
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| Rejected { reason: "unparsable-xml", detail: e.to_string() })?;

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
                if self.parsed.sections.iter().any(|s| s.id == id) {
                    return Err(Rejected {
                        reason: "duplicate-section-id",
                        detail: format!("{id} is published twice, at {path}"),
                    });
                }
                self.parsed.sections.push(Section {
                    id: id.clone(),
                    kind: node.kind.clone(),
                    parent: Some(section.to_owned()),
                });
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
        if field.kind == "time" && paired.is_some() {
            return Ok(());
        }

        let text = element.text().unwrap_or_default();
        let value = match (&paired, field.kind.as_str()) {
            (Some(time), "date") => value::timestamp(text.trim(), Some(time))
                .map(Some)
                .map_err(|e| value::Error(format!("{}: {e}", field.id))),
            _ => value::convert(field, text, |name| element.attribute(name).map(str::to_owned)),
        };
        let value = value.map_err(|e| Rejected { reason: "unrepresentable-value", detail: format!("{} at {path}", e.0) })?;
        let Some(value) = value else { return Ok(()) };

        let ordinal = self.ordinals.entry((section.to_owned(), field.id.clone())).or_insert(-1);
        *ordinal += 1;
        self.parsed.values.push(ValueRow {
            section_id: section.to_owned(),
            field_id: field.id.clone(),
            ordinal: *ordinal,
            value,
        });
        Ok(())
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

/// The text of the element pairing with this one under UBL's `…Date`/`…Time`
/// naming, when both are fields of the matching SDK type.
fn counterpart<'a>(
    element: roxmltree::Node<'a, '_>,
    parent_branches: &[&Branch],
    field: &FieldInfo,
) -> Option<&'a str> {
    let (own, other) = match field.kind.as_str() {
        "date" => ("Date", "Time"),
        "time" => ("Time", "Date"),
        _ => return None,
    };
    let stem = element.tag_name().name().strip_suffix(own)?;
    let wanted = format!("{stem}{other}");
    element
        .parent_element()?
        .children()
        .filter(|c| c.is_element() && c.tag_name().name() == wanted)
        .find(|&c| {
            parent_branches
                .iter()
                .flat_map(|b| b.select(c))
                .filter_map(|b| b.field.as_ref())
                .any(|f| f.kind == other.to_lowercase())
        })
        .and_then(|c| c.text())
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
