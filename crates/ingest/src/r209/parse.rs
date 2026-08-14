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
/// wrapper name: `TED-OLD_VALUE.DATE`, `TED-REF_NOTICE.NO_DOC_OJS`. The
/// R2.0.8 F02 restates the estimated total inside `F02_FRAMEWORK/
/// TOTAL_ESTIMATED` beside the QUANTITY_SCOPE one — same disambiguation.
const FIELD_PREFIX_WRAPPERS: [&str; 7] = [
    "OLD_VALUE",
    "NEW_VALUE",
    "DESCRIPTION_PROCUREMENT",
    "INFO_MODIFICATIONS",
    "REF_NOTICE",
    "INITIAL_ESTIMATED_TOTAL_VALUE_CONTRACT",
    "TOTAL_ESTIMATED",
];

/// Attributes that qualify the element they sit on and are stored as
/// `TED-<ELEMENT>.<ATTR>` code rows when not consumed by the rule itself:
/// `@PUBLICATION` (content withheld from the OJ), `@TYPE`/`@FORMAT` (coded
/// VALUES kinds), `@VALUE`/`@CTYPE` on markers, `@CHOICE`, REF_OJS's
/// `@CLASS`/`@LAST`, the S01-era per-lot `OBJECT_CONTRACT/@ITEM`, the
/// R2.0.8-era qualifiers `@PROCEDURE` (annex-D variants), `@STATUS`/`@OBJECT`
/// (ICAR corrigendum ops) and `@SERVICES_CATEGORY` (F04 works block), and
/// `@REASON` — the justification code on each annex-D negotiated-procedure
/// choice (`PURCHASE_SUPPLIES_ADVANTAGEOUS_TERMS/@REASON="SUPPLIER_WINDING_UP_BUSINESS"`),
/// recurring across 2011 CONTRACT_AWARD/VEAT/utilities awards (issue 31).
const CAPTURED_ATTRIBUTES: [&str; 14] = [
    "PUBLICATION", "TYPE", "VALUE", "CTYPE", "CHOICE", "CLASS", "LAST", "FORMAT", "ITEM",
    "PROCEDURE", "STATUS", "OBJECT", "SERVICES_CATEGORY", "REASON",
];

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
        adopted: Default::default(),
        alias: no_alias,
        overlay: no_overlay,
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

/// Parse one INTERNAL_OJS notice (the 2008 OPOCE R2.0.5 era) by reusing this
/// walker. The 85% of the form body that is r209 vocabulary needs no help; the
/// `alias` maps the `_SUM` summary elements onto their base r209 element, and
/// the `overlay` supplies the ~20 envelope/backbone rules and the handful of
/// same-name-different-shape overrides (`crate::internal_ojs`). The envelope
/// (`TECHNICAL_INFO` + `BIB_INFO`) is walked generically; the single form body
/// (there is one language per file, so no ORIGINAL/TRANSLATION dance) is walked
/// as one form copy in full — never in translation mode.
pub fn parse_internal_ojs(
    xml: &str,
    alias: fn(&str) -> &str,
    overlay: fn(&str, &str) -> Option<Rule>,
) -> Result<Parsed, Rejected> {
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| Rejected { reason: "unparsable-xml", detail: e.to_string() })?;
    let root = doc.root_element();
    if root.tag_name().name() != "INTERNAL_OJS" {
        return Err(Rejected { reason: "unexpected-root", detail: root.tag_name().name().to_owned() });
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
        adopted: Default::default(),
        alias,
        overlay,
    };

    // The S-series heading (2110 contract, 3310/… award, 02A0 EEIG) is the
    // notice's own coded backbone, carried on the root as an attribute rather
    // than the r209 `HEADING` element.
    for attr in root.attributes() {
        match attr.name() {
            "HEADING" => walk.emit(ROOT_SECTION, "TED-HEADING", NoticeValue::Code {
                list: None,
                code: attr.value().trim().to_owned(),
            }),
            _ if is_xsi(&attr) => {}
            other => return Err(unclaimed("attribute", &format!("/INTERNAL_OJS/@{other}"))),
        }
    }
    walk.no_stray_text(root, "/INTERNAL_OJS")?;

    let ctx = Ctx { section: ROOT_SECTION, parent: "INTERNAL_OJS", prefix: "", lang: None, currency: None };
    for child in root.children().filter(|c| c.is_element()) {
        let raw = child.tag_name().name();
        if matches!(raw, "TECHNICAL_INFO" | "BIB_INFO") {
            walk.element(child, &ctx, "/INTERNAL_OJS")?;
        } else if walk.rule("INTERNAL_OJS", raw) == Some(Rule::FormRoot) {
            walk.form_copy(child, "/INTERNAL_OJS")?;
        } else {
            return Err(unclaimed("element", &format!("/INTERNAL_OJS/{raw}")));
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
    /// Sections a secondary copy opened that the primary never had (issue
    /// 201: the Belgian FR co-original carrying a third ORGANISATION, the EN
    /// translations carrying awards the defective ES original lost). Inside
    /// them nothing is shared with the primary, so full emission applies —
    /// the text-only suppression would silently drop their codes and dates.
    adopted: std::collections::HashSet<String>,
    /// Element-name normalisation applied before rule lookup and field-id
    /// synthesis. The `ted-export` profiles use identity; the INTERNAL_OJS
    /// profile ([`parse_internal_ojs`]) maps its `_SUM` summary aliases onto
    /// the base r209 element so one rule set serves both.
    alias: fn(&str) -> &str,
    /// Era rules consulted *before* the r209 registry: the INTERNAL_OJS
    /// envelope/backbone elements and its few same-name-different-shape
    /// overrides. Returns `None` for `ted-export`.
    overlay: fn(&str, &str) -> Option<Rule>,
}

/// Identity alias / empty overlay: the `ted-export` walker matches element
/// names verbatim against the r209 registry.
fn no_alias(name: &str) -> &str {
    name
}
fn no_overlay(_parent: &str, _name: &str) -> Option<Rule> {
    None
}

impl Walk {
    /// The rule for an element: the era overlay wins, else the r209 registry
    /// keyed by the alias-normalised name (identity + empty overlay for
    /// `ted-export`, so this is exactly `rules::rule` there).
    fn rule(&self, parent: &str, raw: &str) -> Option<Rule> {
        (self.overlay)(parent, raw).or_else(|| rules::rule(parent, (self.alias)(raw)))
    }

    fn element(&mut self, el: roxmltree::Node<'_, '_>, ctx: &Ctx<'_>, parent_path: &str) -> Result<(), Rejected> {
        let raw = el.tag_name().name();
        let name = (self.alias)(raw);
        let path = format!("{parent_path}/{name}");

        // Multilingual title/buyer-name copies outside the kept languages are
        // deliberate translation-copy skips, not unclaimed content.
        if matches!(raw, "ML_TI_DOC" | "AA_NAME")
            && el.attribute("LG").is_some_and(|lg| !self.keeps(lg))
        {
            return Ok(());
        }

        let Some(rule) = self.rule(ctx.parent, raw) else {
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
                // Early R2.0.8 revisions may put the award amount directly on
                // the block (`AWARD_AND_CONTRACT_VALUE FMTVAL= CURRENCY=`);
                // same degrade-to-raw-text policy as the Amount rule.
                if let Some(lexical) = el.attribute("FMTVAL") {
                    match (currency, value::cents(lexical)) {
                        (Some(currency), Ok(cents)) => {
                            self.emit(&id, &field, NoticeValue::Amount {
                                cents,
                                currency: currency.to_owned(),
                            });
                        }
                        _ => self.emit_text(&id, &field, lang, lexical.to_owned()),
                    }
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
            Rule::TextGroup => {
                // R2.0.7 publishes the value as direct text; R2.0.8+ nests it
                // in structured children. Both are claimed.
                let text = direct_text(el);
                if !text.is_empty() {
                    self.emit_text(ctx.section, &field, lang, text);
                }
                self.children(el, &Ctx { section: ctx.section, parent: name, prefix: child_prefix, lang, currency }, &path)?;
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
                    match (currency, value::cents(&lexical)) {
                        (Some(currency), Ok(cents)) => {
                            self.emit(ctx.section, &field, NoticeValue::Amount {
                                cents,
                                currency: currency.to_owned(),
                            });
                        }
                        // Prose in a money slot ("10 000 per laureaat" —
                        // measured on the 2011–2016 dailies) or a value with
                        // no currency in scope: the raw text is kept, the
                        // typed value stays absent. Quarantine is for
                        // unconsumed structure, not low-quality values
                        // (ted-legacy-mapping.md §8.2).
                        _ => self.emit_text(ctx.section, &field, lang, lexical),
                    }
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
                    match lexical.trim().parse::<f64>() {
                        Ok(number) => {
                            self.emit(ctx.section, &field, NoticeValue::Number { value: number, unit });
                        }
                        // "2 meses a contar…" in a duration slot: raw kept.
                        Err(_) => self.emit_text(ctx.section, &field, lang, lexical),
                    }
                }
                self.no_element_children(el, &path)?;
            }
            Rule::Integer => {
                if !text.is_empty() {
                    match text.parse::<i64>() {
                        Ok(n) => self.emit(ctx.section, &field, NoticeValue::Integer(n)),
                        // "3-5" in a count slot: raw kept.
                        Err(_) => self.emit_text(ctx.section, &field, lang, text),
                    }
                }
                self.no_element_children(el, &path)?;
            }
            Rule::Date => {
                if !text.is_empty() {
                    let paired = paired_time(el, name);
                    // A junk paired time ("12:15 Uhr" — measured) must not
                    // sink the date it rides with: fall back to the bare
                    // date, then to raw text (§8.2 value policy).
                    let date = match &paired {
                        Some(time) => value::date_with_time(&text, time),
                        None => Err(String::new()),
                    }
                    .or_else(|_| value::date(&text));
                    match date {
                        Ok(date) => self.emit(ctx.section, &field, date),
                        Err(_) => self.emit_text(ctx.section, &field, lang, text),
                    }
                }
                self.no_element_children(el, &path)?;
            }
            Rule::Time => {
                if !text.is_empty() {
                    match paired_date(el, name) {
                        // The paired date stored the instant — unless this
                        // clock is junk the pair could not absorb; then the
                        // raw wall-clock text is kept here.
                        Some(date) => {
                            if value::date_with_time(&date, &text).is_err() {
                                self.emit_text(ctx.section, &field, lang, text);
                            }
                        }
                        None => match value::time_only(&text) {
                            Ok(time) => self.emit(ctx.section, &field, time),
                            Err(_) => self.emit_text(ctx.section, &field, lang, text),
                        },
                    }
                }
                self.no_element_children(el, &path)?;
            }
            Rule::DateTime => {
                if !text.is_empty() {
                    match value::datetime(&text) {
                        Ok(dt) => self.emit(ctx.section, &field, dt),
                        Err(_) => self.emit_text(ctx.section, &field, lang, text),
                    }
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
                    match value::date_from_parts(d, m, y, parts.get("TIME").map(String::as_str)) {
                        Ok(date) => self.emit(ctx.section, &field, date),
                        // Junk split-date digits: raw parts kept as text.
                        Err(_) => {
                            let raw = match parts.get("TIME") {
                                Some(t) => format!("{y}-{m}-{d} {t}"),
                                None => format!("{y}-{m}-{d}"),
                            };
                            self.emit_text(ctx.section, &field, lang, raw);
                        }
                    }
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
        let mut originals = Vec::new();
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
                Some("ORIGINAL") => originals.push(child),
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
        if originals.is_empty() {
            return Err(Rejected { reason: "no-original-form", detail: "FORM_SECTION".into() });
        }

        // Bilingual buyers (Belgium FR+NL, Bolzano DE+IT — measured, 3 per
        // 1529 in the 2019 daily) publish several ORIGINAL copies. The one
        // matching LG_ORIG is walked fully; every further copy — the extra
        // originals and the English translation — contributes its texts only,
        // matched positionally against the primary's structure.
        let primary = originals
            .iter()
            .position(|form| {
                form.attribute("LG").is_some_and(|lg| {
                    self.keep_langs.get(1).is_some_and(|orig| orig.eq_ignore_ascii_case(lg))
                })
            })
            .unwrap_or(0);
        let snapshot = self.counters.clone();
        self.form_copy(originals[primary], "/TED_EXPORT/FORM_SECTION")?;
        let primary_lang = originals[primary].attribute("LG").unwrap_or_default();
        let secondaries = originals
            .iter()
            .enumerate()
            .filter(|&(i, _)| i != primary)
            .map(|(_, form)| *form)
            .chain(english.filter(|_| !primary_lang.eq_ignore_ascii_case("EN")));
        for form in secondaries {
            self.counters = snapshot.clone();
            self.translating = true;
            let result = self.form_copy(form, "/TED_EXPORT/FORM_SECTION");
            self.translating = false;
            result?;
        }
        Ok(())
    }

    /// Walk one form copy (`F02_2014`, `CONTRACT_AWARD_DEFENCE`, the aliased
    /// INTERNAL_OJS `CONTRACT_SUM` → `CONTRACT`, …).
    fn form_copy(&mut self, form: roxmltree::Node<'_, '_>, parent_path: &str) -> Result<(), Rejected> {
        let name = (self.alias)(form.tag_name().name());
        let path = format!("{parent_path}/{name}");
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
                Rule::Section(_) => matches!(attr_name, "ITEM" | "FMTVAL"),
                // A declared text blob claims its whole subtree, layout and
                // formatting attributes (`@QUOTE`, `@SEP`, `@KEY`…) included.
                Rule::Text => true,
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
            // A secondary copy carrying a section the primary lacks: published
            // content, not drift — open and ADOPT it (issue 201). Bilingual
            // co-originals carry extra organisations and change blocks; and in
            // the two members where a TRANSLATION diverged (045641_2014,
            // 275223_2021), the ORIGINAL was the defective copy — one award
            // where 23 translations carry three, an F14 whose original has no
            // CHANGE at all — so rejecting on divergence lost real content
            // every time it fired. Adopted sections are trailing extras in
            // every observed member; a mid-sequence extra would shift the
            // positional text merge, which the corpus has never shown.
            if !self.parsed.sections.iter().any(|s| s.id == id) {
                self.parsed.sections.push(Section {
                    id: id.clone(),
                    kind: kind.to_owned(),
                    parent: Some(parent.to_owned()),
                });
                self.adopted.insert(id.clone());
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
    /// copy's language-bearing text differs from the original. ADOPTED
    /// sections are the exception: nothing in them is shared with the
    /// primary, so suppression would silently drop their codes and dates.
    fn emit(&mut self, section: &str, field: &str, value: NoticeValue) {
        if self.translating && !self.adopted.contains(section) {
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

/// Block-level names inside a declared text subtree: each starts a text row
/// of its own (recursively), instead of being glued into the parent's text.
/// `P` is the R2.0.9 paragraph; the rest is the R2.0.8 btx_oth prose
/// vocabulary of OTH_NOT/EEIG bodies (numbered marks, for/read corrigenda,
/// tables). Inline markup (`FT` sub/superscripts, `EM`, …) stays glued.
const TEXT_BLOCKS: [&str; 44] = [
    "P", "ADDED", "ADDRESS_NOT_MANDATORY", "ADDRESS_NOT_STRUCT", "ANNOTATION", "BLK",
    "BLK_BTX", "BLK_BTX_SEQ", "CELL", "CORPUS", "CORREC", "DEL", "FOR", "FOR_READ",
    "GR_ANNOTATION", "GR_SEQ", "GR_TBL", "HEADER_COL", "HEADER_ROW", "INT_FOR", "INT_GRTBL",
    "INT_LI", "INT_MLI", "INT_OBJ_NOT", "INT_READ", "INT_TBL", "ITEM", "MARK_LIST", "MIXED",
    "MLI_OCCUR", "NEW", "NOTES", "NO_MARK", "OLD", "READ", "REPL", "ROW", "ROW_TXT",
    "STI_DOC", "TBL", "TI_DOC", "TI_MARK", "TOC", "TXT_MARK",
];

/// A Text-rule element's rows: one per block-level child (recursively), plus
/// one for any direct text and inline markup (`FT` sub/superscripts, btx
/// lists) — the whole subtree is consumed as text, nothing needs rules of
/// its own.
fn text_rows(el: roxmltree::Node<'_, '_>) -> Vec<String> {
    let mut rows = Vec::new();
    let mut own = String::new();
    for child in el.children() {
        if child.is_element() && TEXT_BLOCKS.contains(&child.tag_name().name()) {
            flush(&mut own, &mut rows);
            rows.extend(text_rows(child));
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

