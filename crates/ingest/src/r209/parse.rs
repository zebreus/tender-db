//! The exhaustive-consumption walker for TED_EXPORT notices (ADR-0004).
//!
//! Same contract as the eForms walker: every element, attribute and text node
//! is claimed by a rule or the notice quarantines whole, naming the path.
//! Matching is by **local name** — namespace spellings vary across the era
//! even for one schema revision (ted-legacy-mapping.md §1).
//!
//! ## Section-id convention (the projection relies on this)
//!
//! Legacy notices publish no eForms-style section registry, so sections are
//! synthesized deterministically, numbered in document order per notice:
//! `LOT-1..n` (OBJECT_DESCR / defence lot annexes), `RES-1..n` (award
//! blocks), `CHG-1..n` (F14 changes), `MOD-1..n` (F20 modification blocks),
//! `ORG-1..n` (every inline party address block). The published item/lot
//! numbers stay available as values (`TED-ITEM` on the section, `TED-LOT_NO`
//! inside it) — they are what the projection joins on; the synthetic ids only
//! address rows. An organization's role is recorded on the *enclosing*
//! section as an id-ref whose field id names the role-bearing element
//! (`TED-ADDRESS_CONTRACTOR` → `ORG-3`), mirroring eForms' OPT-300 pattern.
//!
//! ## Language policy (CONTEXT.md: EN + original only)
//!
//! The ORIGINAL form copy is parsed fully. If the original language is not
//! English and an English TRANSLATION copy exists, that copy contributes its
//! *text* values only (codes are identical by construction — measured);
//! structure is matched positionally, which works because translation copies
//! replicate the original's element tree. All other translation copies, and
//! non-kept `ML_TI_DOC`/`AA_NAME` languages, are claimed and skipped as
//! `translation-copy` content.

use std::collections::HashMap;

use store::{NoticeValue, Parsed, Section, ValueRow};

use super::rules::{self, IdKind, Rule, Unit};
use super::value;

/// The notice root's section id — same convention as the eForms profile.
const ROOT_SECTION: &str = "PROCEDURE";

/// Form-root local names of the defence forms (R2.0.8 grammar, published
/// through 2024 inside R2.0.9-era packages).
pub const DEFENCE_FORMS: [&str; 4] = [
    "PRIOR_INFORMATION_DEFENCE",
    "CONTRACT_DEFENCE",
    "CONTRACT_AWARD_DEFENCE",
    "CONTRACT_CONCESSIONAIRE_DEFENCE",
];

/// Sibling pairs publishing one instant as two elements.
const DATE_TIME_PAIRS: [(&str, &str); 3] = [
    ("DATE_RECEIPT_TENDERS", "TIME_RECEIPT_TENDERS"),
    ("DATE_OPENING_TENDERS", "TIME_OPENING_TENDERS"),
    ("DATE", "TIME"),
];

/// Wrappers whose contents would otherwise collide on one field id within one
/// section: an F14 change carries the same `DATE`/`TEXT` shape twice (old and
/// new); an F20 modification block restates `SHORT_DESCR`/`VAL_TOTAL`/
/// contractors both as-modified (`DESCRIPTION_PROCUREMENT`) and as-diff
/// (`INFO_MODIFICATIONS`); `REF_NOTICE` wraps the chain-edge `NO_DOC_OJS`
/// beside the notice's own; a defence award pairs an initial-estimate
/// `VALUE_COST` with the final one. Fields inside them are prefixed with the
/// wrapper name: `TED-OLD_VALUE.DATE`, `TED-REF_NOTICE.NO_DOC_OJS`.
const FIELD_PREFIX_WRAPPERS: [&str; 6] = [
    "OLD_VALUE",
    "NEW_VALUE",
    "DESCRIPTION_PROCUREMENT",
    "INFO_MODIFICATIONS",
    "REF_NOTICE",
    "INITIAL_ESTIMATED_TOTAL_VALUE_CONTRACT",
];

/// Attributes that qualify the element they sit on and are stored as
/// `TED-<ELEMENT>.<ATTR>` code rows when not consumed by the rule itself:
/// `@PUBLICATION` (content withheld from the OJ), `@TYPE`/`@FORMAT` (coded
/// VALUES kinds), `@VALUE`/`@CTYPE` on markers, `@CHOICE`, REF_OJS's
/// `@CLASS`/`@LAST`, and the S01-era per-lot `OBJECT_CONTRACT/@ITEM`.
const CAPTURED_ATTRIBUTES: [&str; 9] =
    ["PUBLICATION", "TYPE", "VALUE", "CTYPE", "CHOICE", "CLASS", "LAST", "FORMAT", "ITEM"];

#[derive(Debug, PartialEq)]
pub struct Rejected {
    pub reason: &'static str,
    pub detail: String,
}

/// Parse one TED_EXPORT notice into the notice-parsed layer.
pub fn parse(xml: &str) -> Result<Parsed, Rejected> {
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| Rejected { reason: "unparsable-xml", detail: e.to_string() })?;
    let root = doc.root_element();
    if root.tag_name().name() != "TED_EXPORT" {
        return Err(Rejected {
            reason: "unexpected-root",
            detail: root.tag_name().name().to_owned(),
        });
    }

    let mut walk = Walk {
        parsed: Parsed {
            sections: vec![Section { id: ROOT_SECTION.into(), kind: "Notice".into(), parent: None }],
            values: Vec::new(),
        },
        ordinals: HashMap::new(),
        counters: HashMap::new(),
        keep_langs: kept_languages(&doc),
        translating: false,
    };

    // Root attributes: DOC_ID is the Notice identity the dispatcher already
    // recorded, VERSION is the declared schema revision on the notice row,
    // EDITION restates REF_OJS (NO_OJ + DATE_PUB, both mapped).
    for attr in root.attributes() {
        if !matches!(attr.name(), "DOC_ID" | "EDITION" | "VERSION") && !is_xsi(&attr) {
            return Err(unclaimed("attribute", &format!("/TED_EXPORT/@{}", attr.name())));
        }
    }

    walk.no_stray_text(root, "/TED_EXPORT")?;
    let ctx = Ctx { section: ROOT_SECTION, parent: "", prefix: "", lang: None, currency: None };
    for child in root.children().filter(|c| c.is_element()) {
        if child.tag_name().name() == "FORM_SECTION" {
            walk.form_section(child)?;
        } else {
            walk.element(child, &ctx, "/TED_EXPORT")?;
        }
    }
    Ok(walk.parsed)
}

/// Languages whose translated titles/buyer names are kept: English plus the
/// notice's original language (CONTEXT.md language policy).
fn kept_languages(doc: &roxmltree::Document<'_>) -> Vec<String> {
    let mut keep = vec!["EN".to_owned()];
    if let Some(orig) = doc
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name() == "LG_ORIG")
        .and_then(|n| n.text())
    {
        let orig = orig.trim().to_uppercase();
        if !orig.is_empty() && !keep.contains(&orig) {
            keep.push(orig);
        }
    }
    keep
}

#[derive(Clone, Copy)]
struct Ctx<'a> {
    section: &'a str,
    /// Parent element's local name, for the context-override rules.
    parent: &'a str,
    /// Field-id prefix from the innermost [`FIELD_PREFIX_WRAPPERS`] wrapper.
    prefix: &'a str,
    lang: Option<&'a str>,
    currency: Option<&'a str>,
}

fn field_id(prefix: &str, name: &str) -> String {
    if prefix.is_empty() { format!("TED-{name}") } else { format!("TED-{prefix}.{name}") }
}

struct Walk {
    parsed: Parsed,
    ordinals: HashMap<(String, String), i64>,
    /// Per-prefix section counters (`LOT`, `RES`, `CHG`, `MOD`, `ORG`).
    counters: HashMap<&'static str, u32>,
    keep_langs: Vec<String>,
    /// Translation mode: sections are matched positionally (not created) and
    /// only text values are emitted, in the copy's language.
    translating: bool,
}

impl Walk {
    fn element(&mut self, el: roxmltree::Node<'_, '_>, ctx: &Ctx<'_>, parent_path: &str) -> Result<(), Rejected> {
        let name = el.tag_name().name();
        let path = format!("{parent_path}/{name}");

        // Multilingual title/buyer-name copies outside the kept languages are
        // deliberate translation-copy skips, not unclaimed content.
        if matches!(name, "ML_TI_DOC" | "AA_NAME")
            && el.attribute("LG").is_some_and(|lg| !self.keeps(lg))
        {
            return Ok(());
        }

        let Some(rule) = rules::rule(ctx.parent, name) else {
            return Err(unclaimed("element", &path));
        };
        if let Rule::Ignore(_) = rule {
            return Ok(());
        }

        let lang = el.attribute("LG").or(ctx.lang);
        let currency = el.attribute("CURRENCY").or(ctx.currency);
        let captures = self.check_attributes(el, rule, &path)?;

        let field = field_id(ctx.prefix, name);
        let child_prefix = if FIELD_PREFIX_WRAPPERS.contains(&name) { name } else { ctx.prefix };
        let text = direct_text(el);

        // Attribute captures of a section-opening element belong to the
        // section it opens; everything else qualifies the current section.
        if !matches!(rule, Rule::Section(_) | Rule::Org) {
            self.emit_captures(ctx.section, &field, &captures);
        }

        match rule {
            Rule::Ignore(_) => unreachable!("returned above"),
            Rule::FormRoot => {
                return Err(Rejected {
                    reason: "form-outside-form-section",
                    detail: path,
                });
            }
            Rule::Group => {
                self.no_stray_text(el, &path)?;
                self.children(el, &Ctx { section: ctx.section, parent: name, prefix: child_prefix, lang, currency }, &path)?;
            }
            Rule::Marker => {
                self.emit(ctx.section, &field, NoticeValue::Integer(1));
                self.no_stray_text(el, &path)?;
                self.children(el, &Ctx { section: ctx.section, parent: name, prefix: child_prefix, lang, currency }, &path)?;
            }
            Rule::Section(kind) => {
                let id = self.open_section(kind.prefix(), kind.kind(), ctx.section, &path)?;
                self.emit_captures(&id, &field, &captures);
                if let Some(item) = el.attribute("ITEM") {
                    self.emit(&id, "TED-ITEM", NoticeValue::Id {
                        scheme: None,
                        value: item.trim().to_owned(),
                        is_ref: false,
                    });
                }
                self.no_stray_text(el, &path)?;
                self.children(el, &Ctx { section: &id, parent: name, prefix: child_prefix, lang, currency }, &path)?;
            }
            Rule::Org => {
                let id = self.open_section("ORG", "Organization", ctx.section, &path)?;
                self.emit_captures(&id, &field, &captures);
                // The role is the element's own name, except for the generic
                // defence CONTACT_DATA blocks, whose role lives on the wrapper.
                let role = if name.starts_with("CONTACT_DATA") { ctx.parent } else { name };
                self.emit(ctx.section, &field_id(ctx.prefix, role), NoticeValue::Id {
                    scheme: None,
                    value: id.clone(),
                    is_ref: true,
                });
                self.no_stray_text(el, &path)?;
                self.children(el, &Ctx { section: &id, parent: name, prefix: child_prefix, lang, currency }, &path)?;
            }
            Rule::Text => {
                for row in text_rows(el) {
                    self.emit_text(ctx.section, &field, lang, row);
                }
            }
            Rule::CodeAttr(attrs) => {
                // Element text is the redundant display label; children (e.g.
                // ANNEX_D justifications, FD_* form bodies) are real content.
                if let Some(code) = attrs.iter().find_map(|a| el.attribute(*a)) {
                    self.emit(ctx.section, &field, NoticeValue::Code {
                        list: None,
                        code: code.trim().to_owned(),
                    });
                }
                self.children(el, &Ctx { section: ctx.section, parent: name, prefix: child_prefix, lang, currency }, &path)?;
            }
            Rule::CodeText => {
                if !text.is_empty() {
                    self.emit(ctx.section, &field, NoticeValue::Code { list: None, code: text });
                }
                self.no_element_children(el, &path)?;
            }
            Rule::Cpv | Rule::Nuts => {
                let scheme = if rule == Rule::Cpv { "cpv" } else { "nuts" };
                let code = el.attribute("CODE").map(str::trim).map(str::to_owned)
                    .filter(|c| !c.is_empty())
                    .or_else(|| (!text.is_empty()).then(|| text.clone()));
                if let Some(code) = code {
                    self.emit(ctx.section, &field, NoticeValue::Classification {
                        scheme: scheme.into(),
                        code,
                    });
                }
                self.no_element_children(el, &path)?;
            }
            Rule::Amount => {
                let lexical = el.attribute("FMTVAL").map(str::to_owned)
                    .or_else(|| (!text.is_empty()).then(|| text.clone()));
                if let Some(lexical) = lexical {
                    let currency = currency.ok_or_else(|| Rejected {
                        reason: "unrepresentable-value",
                        detail: format!("amount without a currency in scope at {path}"),
                    })?;
                    let cents = value::cents(&lexical)
                        .map_err(|e| unrepresentable(&e, &path))?;
                    self.emit(ctx.section, &field, NoticeValue::Amount {
                        cents,
                        currency: currency.to_owned(),
                    });
                }
                self.no_element_children(el, &path)?;
            }
            Rule::Number(unit) => {
                let lexical = el.attribute("FMTVAL").map(str::to_owned)
                    .or_else(|| (!text.is_empty()).then(|| text.clone()));
                if let Some(lexical) = lexical {
                    let unit = match unit {
                        Unit::Fixed(u) => Some(u.to_owned()),
                        Unit::FromTypeAttr => el.attribute("TYPE").map(str::to_owned),
                    };
                    let number: f64 = lexical.trim().parse()
                        .map_err(|_| unrepresentable(&format!("not a number: {lexical}"), &path))?;
                    self.emit(ctx.section, &field, NoticeValue::Number { value: number, unit });
                }
                self.no_element_children(el, &path)?;
            }
            Rule::Integer => {
                if !text.is_empty() {
                    let n: i64 = text.parse()
                        .map_err(|_| unrepresentable(&format!("not an integer: {text}"), &path))?;
                    self.emit(ctx.section, &field, NoticeValue::Integer(n));
                }
                self.no_element_children(el, &path)?;
            }
            Rule::Date => {
                if !text.is_empty() {
                    let paired = paired_time(el, name);
                    let date = match &paired {
                        Some(time) => value::date_with_time(&text, time),
                        None => value::date(&text),
                    }
                    .map_err(|e| unrepresentable(&e, &path))?;
                    self.emit(ctx.section, &field, date);
                }
                self.no_element_children(el, &path)?;
            }
            Rule::Time => {
                // Emit nothing when a paired date sibling stored the instant.
                if !text.is_empty() && paired_date(el, name).is_none() {
                    let time = value::time_only(&text).map_err(|e| unrepresentable(&e, &path))?;
                    self.emit(ctx.section, &field, time);
                }
                self.no_element_children(el, &path)?;
            }
            Rule::DateTime => {
                if !text.is_empty() {
                    let dt = value::datetime(&text).map_err(|e| unrepresentable(&e, &path))?;
                    self.emit(ctx.section, &field, dt);
                }
                self.no_element_children(el, &path)?;
            }
            Rule::DateParts => {
                let mut parts: HashMap<&str, String> = HashMap::new();
                for child in el.children().filter(|c| c.is_element()) {
                    match child.tag_name().name() {
                        part @ ("DAY" | "MONTH" | "YEAR" | "TIME") => {
                            parts.insert(part, direct_text(child));
                        }
                        other => {
                            return Err(unclaimed("element", &format!("{path}/{other}")));
                        }
                    }
                }
                if let (Some(d), Some(m), Some(y)) = (parts.get("DAY"), parts.get("MONTH"), parts.get("YEAR")) {
                    let date = value::date_from_parts(d, m, y, parts.get("TIME").map(String::as_str))
                        .map_err(|e| unrepresentable(&e, &path))?;
                    self.emit(ctx.section, &field, date);
                }
            }
            Rule::DatePart => {
                // DAY/MONTH/YEAR belong inside a DateParts container, which
                // consumes them before rule lookup; standalone is unclaimed.
                return Err(unclaimed("element", &path));
            }
            Rule::Id(kind) => {
                if !text.is_empty() {
                    let (scheme, is_ref) = match kind {
                        IdKind::Plain => (None, false),
                        IdKind::Ref => (Some("ojs".to_owned()), true),
                        IdKind::National => (Some("national".to_owned()), false),
                    };
                    self.emit(ctx.section, &field, NoticeValue::Id { scheme, value: text, is_ref });
                }
                self.no_element_children(el, &path)?;
            }
        }
        Ok(())
    }

    fn children(&mut self, el: roxmltree::Node<'_, '_>, ctx: &Ctx<'_>, path: &str) -> Result<(), Rejected> {
        for child in el.children().filter(|c| c.is_element()) {
            self.element(child, ctx, path)?;
        }
        Ok(())
    }

    /// The FORM_SECTION dispatcher: identity fields, then one full walk of the
    /// ORIGINAL copy and a text-only walk of the English translation (if the
    /// original is not English). Remaining copies are translation-copy skips.
    fn form_section(&mut self, el: roxmltree::Node<'_, '_>) -> Result<(), Rejected> {
        let mut original = None;
        let mut english = None;
        for child in el.children().filter(|c| c.is_element()) {
            let name = child.tag_name().name();
            if name == "NOTICE_UUID" {
                let text = direct_text(child);
                if !text.is_empty() {
                    self.emit(ROOT_SECTION, "TED-NOTICE_UUID", NoticeValue::Id {
                        scheme: None,
                        value: text,
                        is_ref: false,
                    });
                }
                continue;
            }
            if rules::rule("FORM_SECTION", name) != Some(Rule::FormRoot) {
                return Err(unclaimed("element", &format!("/TED_EXPORT/FORM_SECTION/{name}")));
            }
            match child.attribute("CATEGORY") {
                Some("ORIGINAL") if original.is_some() => {
                    return Err(Rejected {
                        reason: "multiple-original-forms",
                        detail: name.to_owned(),
                    });
                }
                Some("ORIGINAL") => original = Some(child),
                Some("TRANSLATION") => {
                    if child.attribute("LG").is_some_and(|lg| lg.eq_ignore_ascii_case("EN")) {
                        english.get_or_insert(child);
                    }
                }
                other => {
                    return Err(Rejected {
                        reason: "form-without-category",
                        detail: format!("{name} CATEGORY={other:?}"),
                    });
                }
            }
        }
        let Some(original) = original else {
            return Err(Rejected { reason: "no-original-form", detail: "FORM_SECTION".into() });
        };

        // Section counters snapshot: the translation walk re-runs the same
        // structure and must synthesize the same section ids.
        let snapshot = self.counters.clone();
        self.form_copy(original)?;
        let original_lang = original.attribute("LG").unwrap_or_default();
        if !original_lang.eq_ignore_ascii_case("EN")
            && let Some(english) = english
        {
            self.counters = snapshot;
            self.translating = true;
            let result = self.form_copy(english);
            self.translating = false;
            result?;
        }
        Ok(())
    }

    /// Walk one form copy (`F02_2014`, `CONTRACT_AWARD_DEFENCE`, …).
    fn form_copy(&mut self, form: roxmltree::Node<'_, '_>) -> Result<(), Rejected> {
        let name = form.tag_name().name();
        let path = format!("/TED_EXPORT/FORM_SECTION/{name}");
        for attr in form.attributes() {
            match attr.name() {
                // CATEGORY/LG steer this dispatcher; VERSION is the notice
                // row's declared_version; FORM is stored below.
                "CATEGORY" | "LG" | "VERSION" | "FORM" => {}
                other if is_xsi(&attr) => _ = other,
                other => return Err(unclaimed("attribute", &format!("{path}/@{other}"))),
            }
        }
        if let Some(form_no) = form.attribute("FORM") {
            self.emit(ROOT_SECTION, "TED-FORM", NoticeValue::Code {
                list: None,
                code: form_no.trim().to_owned(),
            });
        }
        let lang = form.attribute("LG");
        let ctx = Ctx { section: ROOT_SECTION, parent: name, prefix: "", lang, currency: None };
        self.no_stray_text(form, &path)?;
        self.children(form, &ctx, &path)
    }

    /// Verify every attribute is claimed; return the qualifier attributes to
    /// store (as `(attribute, value)` pairs) once the target section is known.
    fn check_attributes(
        &mut self,
        el: roxmltree::Node<'_, '_>,
        rule: Rule,
        path: &str,
    ) -> Result<Vec<(String, String)>, Rejected> {
        let mut captures = Vec::new();
        for attr in el.attributes() {
            let attr_name = attr.name();
            let consumed = match rule {
                Rule::CodeAttr(attrs) => attrs.contains(&attr_name),
                Rule::Cpv | Rule::Nuts => attr_name == "CODE",
                Rule::Amount => attr_name == "FMTVAL",
                Rule::Number(Unit::FromTypeAttr) => matches!(attr_name, "TYPE" | "FMTVAL"),
                Rule::Number(_) => attr_name == "FMTVAL",
                Rule::Section(_) => attr_name == "ITEM",
                _ => false,
            };
            if consumed
                || attr_name == "LG"       // language context
                || attr_name == "CURRENCY" // currency context
                || is_xsi(&attr)
            {
                continue;
            }
            if CAPTURED_ATTRIBUTES.contains(&attr_name) {
                captures.push((attr_name.to_owned(), attr.value().trim().to_owned()));
                continue;
            }
            return Err(unclaimed("attribute", &format!("{path}/@{attr_name}")));
        }
        Ok(captures)
    }

    fn emit_captures(&mut self, section: &str, base: &str, captures: &[(String, String)]) {
        for (attr, code) in captures {
            self.emit(section, &format!("{base}.{attr}"), NoticeValue::Code {
                list: None,
                code: code.clone(),
            });
        }
    }

    fn open_section(
        &mut self,
        prefix: &'static str,
        kind: &str,
        parent: &str,
        path: &str,
    ) -> Result<String, Rejected> {
        let n = self.counters.entry(prefix).or_insert(0);
        *n += 1;
        let id = format!("{prefix}-{n}");
        if self.translating {
            // The translation copy replicates the original's structure; a
            // section it computes must already exist or the copies diverge.
            if !self.parsed.sections.iter().any(|s| s.id == id) {
                return Err(Rejected {
                    reason: "translation-structure-mismatch",
                    detail: format!("{id} at {path}"),
                });
            }
        } else {
            self.parsed.sections.push(Section {
                id: id.clone(),
                kind: kind.to_owned(),
                parent: Some(parent.to_owned()),
            });
        }
        Ok(id)
    }

    fn keeps(&self, lang: &str) -> bool {
        self.keep_langs.iter().any(|k| k.eq_ignore_ascii_case(lang))
    }

    /// Emit a non-text value — suppressed in translation mode, where only the
    /// copy's language-bearing text differs from the original.
    fn emit(&mut self, section: &str, field: &str, value: NoticeValue) {
        if self.translating {
            return;
        }
        self.push(section, field, value);
    }

    fn emit_text(&mut self, section: &str, field: &str, lang: Option<&str>, text: String) {
        let value = NoticeValue::Text { lang: lang.map(str::to_owned), value: text };
        self.push(section, field, value);
    }

    fn push(&mut self, section: &str, field: &str, value: NoticeValue) {
        let ordinal = self.ordinals.entry((section.to_owned(), field.to_owned())).or_insert(-1);
        *ordinal += 1;
        self.parsed.values.push(ValueRow {
            section_id: section.to_owned(),
            field_id: field.to_owned(),
            ordinal: *ordinal,
            value,
        });
    }

    fn no_stray_text(&self, el: roxmltree::Node<'_, '_>, path: &str) -> Result<(), Rejected> {
        let stray = el
            .children()
            .any(|c| c.is_text() && !c.text().unwrap_or_default().trim().is_empty());
        if stray {
            return Err(unclaimed("text", path));
        }
        Ok(())
    }

    fn no_element_children(&self, el: roxmltree::Node<'_, '_>, path: &str) -> Result<(), Rejected> {
        if let Some(child) = el.children().find(|c| c.is_element()) {
            return Err(unclaimed("element", &format!("{path}/{}", child.tag_name().name())));
        }
        Ok(())
    }
}

/// The wall-clock text of the time element paired with this date, if any.
fn paired_time<'a>(el: roxmltree::Node<'a, '_>, date_name: &str) -> Option<String> {
    let time_name = DATE_TIME_PAIRS.iter().find(|(d, _)| *d == date_name)?.1;
    sibling_text(el, time_name)
}

/// The date element paired with this time, if any (it stores the instant).
fn paired_date<'a>(el: roxmltree::Node<'a, '_>, time_name: &str) -> Option<String> {
    let date_name = DATE_TIME_PAIRS.iter().find(|(_, t)| *t == time_name)?.0;
    sibling_text(el, date_name)
}

fn sibling_text(el: roxmltree::Node<'_, '_>, name: &str) -> Option<String> {
    el.parent_element()?
        .children()
        .find(|c| c.is_element() && c.tag_name().name() == name)
        .map(direct_text)
        .filter(|t| !t.is_empty())
}

/// The element's own text content (not descendants), trimmed.
fn direct_text(el: roxmltree::Node<'_, '_>) -> String {
    el.children()
        .filter(|c| c.is_text())
        .map(|c| c.text().unwrap_or_default())
        .collect::<String>()
        .trim()
        .to_owned()
}

/// A Text-rule element's rows: one per `<P>` paragraph, plus one for any
/// direct text and inline markup (`FT` sub/superscripts, btx tables/lists) —
/// the whole subtree is consumed as text, nothing needs rules of its own.
fn text_rows(el: roxmltree::Node<'_, '_>) -> Vec<String> {
    let mut rows = Vec::new();
    let mut own = String::new();
    for child in el.children() {
        if child.is_element() && child.tag_name().name() == "P" {
            flush(&mut own, &mut rows);
            let text = subtree_text(child);
            if !text.is_empty() {
                rows.push(text);
            }
        } else if child.is_element() {
            own.push_str(&subtree_text(child));
        } else if child.is_text() {
            own.push_str(child.text().unwrap_or_default());
        }
    }
    flush(&mut own, &mut rows);
    rows
}

fn flush(own: &mut String, rows: &mut Vec<String>) {
    let text = own.trim();
    if !text.is_empty() {
        rows.push(text.to_owned());
    }
    own.clear();
}

fn subtree_text(el: roxmltree::Node<'_, '_>) -> String {
    let mut out = String::new();
    for d in el.descendants() {
        if d.is_text() {
            out.push_str(d.text().unwrap_or_default());
        }
    }
    out.trim().to_owned()
}

fn is_xsi(attr: &roxmltree::Attribute<'_, '_>) -> bool {
    attr.namespace() == Some("http://www.w3.org/2001/XMLSchema-instance")
}

fn unclaimed(what: &str, path: &str) -> Rejected {
    Rejected { reason: "unclaimed-content", detail: format!("unclaimed {what} at {path}") }
}

fn unrepresentable(err: &str, path: &str) -> Rejected {
    Rejected { reason: "unrepresentable-value", detail: format!("{err} at {path}") }
}
