//! Projection: the notice-parsed layer → the versioned canonical layer.
//!
//! Deterministic and rebuildable (ADR-0001): canonical state is a pure function
//! of the parsed notices plus the rules below, so it can always be thrown away
//! and re-derived. Nothing here reads XML — that happened in [`crate::process`].
//!
//! The rules, in the order they apply:
//!
//! 1. **Identity.** Notices sharing a procedure key (BT-04 `ContractFolderID`
//!    for TED eForms) are one Tender. A notice publishing no key becomes a
//!    single-notice *island* Tender rather than being dropped or guessed into
//!    someone else's procedure (CONTEXT.md); it upgrades by re-projection if
//!    linkage ever appears.
//! 2. **Order.** A Tender's notices are ordered by publication date (BT-05
//!    dispatch date where no publication date exists), then by publication id.
//!    Declared version numbers are advisory and are not used — real chains have
//!    gaps and cross-type sequences.
//! 3. **Supersession.** Each notice yields one version, resolved as *this
//!    notice's values over the previous version's*: a field the notice
//!    republishes replaces the earlier one wholesale (all languages of a title
//!    together, all CPV codes of one role together); a field it is silent about
//!    carries forward. That is what makes an award notice a complete Tender
//!    state rather than a fragment.
//! 4. **Lots.** Lots are resolved per version by the id the notice published.
//!    Two versions share a Lot exactly when they publish the same id — nothing
//!    is matched across versions by position, title or order, because
//!    framework/DPS call-off rounds relabel lots per round.
//! 5. **Organizations.** Every Organization section becomes a mention. Mentions
//!    merge into one canonical profile only on an exact normalised official
//!    identifier that passes the plausibility gate below; everything else is a
//!    provisional profile of that one mention.
//!
//! Change scoping is diff-based (ADR-0001 amendment) and lives in `store`,
//! which has both the old and the new version in hand.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use store::{Db, Fact, Identifier, LotState, Mention, NoticeValue, Parsed, TenderProjection, TenderVersion};

/// What a projection run did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub notices: u64,
    pub tenders: u64,
    pub islands: u64,
    pub mentions: u64,
    pub applied: store::Applied,
}

/// The canonical fields this layer carries, as data. Source field ids are
/// matched by their business-term stem (`BT-21-Lot`, `BT-21-Procedure` and
/// `BT-21-Part` are all the same canonical `title`), so a term keeps one
/// canonical name wherever the SDK mounts it.
const TEXTS: &[(&str, &str)] = &[("BT-21", "title"), ("BT-24", "description")];
const AMOUNTS: &[(&str, &str)] = &[
    ("BT-27", "estimated_value"),
    ("BT-271", "framework_maximum"),
    ("BT-161", "result_value"),
];
const CLASSIFICATIONS: &[(&str, &str)] =
    &[("BT-262", "main"), ("BT-263", "additional"), ("BT-5071", "place")];
/// The date/time pairs issue 03 stores as one instant, so `(d)` is the whole
/// deadline and there is no `(t)` row to reunite here.
const DATES: &[(&str, &str)] = &[
    ("BT-131(d)", "submission_deadline"),
    ("BT-1311(d)", "participation_deadline"),
    ("BT-132(d)", "opening_date"),
    ("BT-13(d)", "additional_information_deadline"),
    ("BT-536", "duration_start"),
    ("BT-537", "duration_end"),
];

/// Sections that are Lots in the canonical sense — Parts and LotsGroups are
/// Lots with a kind flag (CONTEXT.md).
const LOT_KINDS: &[&str] = &["Lot", "LotsGroup", "Part"];

const PROCEDURE_KEY_FIELD: &str = "BT-04-notice";
const PUBLICATION_DATE_FIELD: &str = "OPP-012-notice";
const DISPATCH_DATE_FIELD: &str = "BT-05(a)-notice";
const SUBTYPE_FIELD: &str = "OPP-070-notice";
const ORGANIZATION_KIND: &str = "Organization";
const ORG_NAME_FIELD: &str = "BT-500-Organization-Company";
const ORG_IDENTIFIER_FIELD: &str = "BT-501-Organization-Company";
const ORG_COUNTRY_FIELD: &str = "BT-514-Organization-Company";
/// Business Registration Information Notices carry no procurement procedure;
/// CONTEXT.md makes them minimal Tenders of their own kind.
const REGISTRATION_SUBTYPE: &str = "X01";

/// Run the projection over every parsed notice. With `rebuild`, the canonical
/// layer's content is dropped first and re-derived from scratch — the change
/// log is kept and appended to, never renumbered.
pub async fn project(db: &Db, rebuild: bool) -> turso::Result<Report> {
    if rebuild {
        db.clear_canonical().await?;
    }
    let now = unix_now();
    let mut report = Report::default();

    // Derive each notice's canonical reading, resolving its organizations as we
    // go: mentions are keyed by (notice, section), so this is idempotent.
    let mut states = Vec::new();
    for notice in db.parsed_notices().await? {
        let parsed = db.parsed_notice(notice.id).await?;
        let mut state = NoticeState::read(&notice, &parsed);
        let mentions = state.take_mentions(notice.id, &parsed);
        report.mentions += mentions.len() as u64;
        let ids = db.resolve_mentions(&mentions, now).await?;
        state.bind_organizations(&mentions, &ids);
        report.notices += 1;
        states.push(state);
    }

    for projection in group(states) {
        report.tenders += 1;
        report.islands += u64::from(projection.procedure_key.is_none());
        report.applied.add(db.apply_tender(&projection, now).await?);
    }
    Ok(report)
}

/// One notice read in canonical terms, before it is folded into a chain.
struct NoticeState {
    notice_id: i64,
    source: String,
    publication_id: String,
    procedure_key: Option<String>,
    published_at: i64,
    subtype: Option<String>,
    /// Tender-scoped facts, and one bucket per lot the notice published.
    facts: BTreeSet<Fact>,
    lots: Vec<LotState>,
    /// Role references awaiting their canonical organization id: (scope,
    /// role, ORG section id).
    roles: Vec<(Scope, String, String)>,
}

/// Where a value belongs: the Tender itself, or one of its Lots.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Scope {
    Tender,
    Lot(String),
}

impl NoticeState {
    fn read(notice: &store::NoticeRef, parsed: &Parsed) -> NoticeState {
        let sections: HashMap<&str, &store::Section> =
            parsed.sections.iter().map(|s| (s.id.as_str(), s)).collect();

        let mut lots: BTreeMap<String, LotState> = parsed
            .sections
            .iter()
            .filter(|s| LOT_KINDS.contains(&s.kind.as_str()))
            .map(|s| {
                (s.id.clone(), LotState { key: s.id.clone(), kind: s.kind.clone(), facts: BTreeSet::new() })
            })
            .collect();

        let mut facts = BTreeSet::new();
        let mut roles = Vec::new();
        for value in &parsed.values {
            let scope = scope_of(&sections, &value.section_id);
            let stem = stem(&value.field_id);
            let fact = match &value.value {
                NoticeValue::Text { lang, value: v } => canonical_name(TEXTS, stem)
                    .map(|field| Fact::Text { field, lang: lang.clone(), value: v.clone() }),
                NoticeValue::Amount { cents, currency } => canonical_name(AMOUNTS, stem)
                    .map(|field| Fact::Amount { field, cents: *cents, currency: currency.clone() }),
                NoticeValue::Classification { scheme, code } => canonical_name(CLASSIFICATIONS, stem)
                    .map(|field| Fact::Classification {
                        field,
                        scheme: scheme.clone(),
                        code: code.clone(),
                    }),
                NoticeValue::Date { utc_seconds, offset_minutes, has_time } => {
                    canonical_name(DATES, stem).map(|field| Fact::Date {
                        field,
                        utc_seconds: *utc_seconds,
                        offset_minutes: *offset_minutes,
                        has_time: *has_time,
                    })
                }
                NoticeValue::Id { value: target, is_ref: true, .. } => {
                    if let Some(role) = role_name(&value.field_id) {
                        roles.push((scope.clone(), role, target.clone()));
                    }
                    None
                }
                _ => None,
            };
            if let Some(fact) = fact {
                match &scope {
                    Scope::Tender => {
                        facts.insert(fact);
                    }
                    Scope::Lot(key) => {
                        if let Some(lot) = lots.get_mut(key) {
                            lot.facts.insert(fact);
                        }
                    }
                }
            }
        }

        NoticeState {
            notice_id: notice.id,
            source: notice.source.clone(),
            publication_id: notice.publication_id.clone(),
            procedure_key: first_id(parsed, PROCEDURE_KEY_FIELD).filter(|k| !k.trim().is_empty()),
            published_at: first_date(parsed, PUBLICATION_DATE_FIELD)
                .or_else(|| first_date(parsed, DISPATCH_DATE_FIELD))
                .unwrap_or(0),
            subtype: first_code(parsed, SUBTYPE_FIELD),
            facts,
            lots: lots.into_values().collect(),
            roles,
        }
    }

    /// Every Organization section of the notice, normalised for merging.
    ///
    /// An Organization's own values are spread over its subtree rather than
    /// sitting on the section itself: the name is on the Organization, but the
    /// official identifier hangs off its `CompanyLegalEntity` child (14 813 of
    /// them on the 2026-136 daily). So a mention collects from the whole
    /// subtree, keyed by the enclosing Organization.
    fn take_mentions(&mut self, notice_id: i64, parsed: &Parsed) -> Vec<Mention> {
        let sections: HashMap<&str, &store::Section> =
            parsed.sections.iter().map(|s| (s.id.as_str(), s)).collect();

        let mut mentions: BTreeMap<&str, Mention> = parsed
            .sections
            .iter()
            .filter(|s| s.kind == ORGANIZATION_KIND)
            .map(|s| {
                (
                    s.id.as_str(),
                    Mention {
                        notice_id,
                        section_id: s.id.clone(),
                        name: String::new(),
                        country: None,
                        raw_identifier: None,
                        scheme: None,
                        identifier: None,
                    },
                )
            })
            .collect();

        for value in &parsed.values {
            let Some(owner) = enclosing(&sections, &value.section_id, &[ORGANIZATION_KIND]) else {
                continue;
            };
            let Some(mention) = mentions.get_mut(owner) else { continue };
            match (value.field_id.as_str(), &value.value) {
                (ORG_NAME_FIELD, NoticeValue::Text { value, .. }) => mention.name.clone_from(value),
                (ORG_COUNTRY_FIELD, NoticeValue::Code { code, .. }) => {
                    mention.country = Some(code.clone());
                }
                (ORG_IDENTIFIER_FIELD, NoticeValue::Id { value, scheme, .. }) => {
                    mention.raw_identifier = Some(value.clone());
                    mention.scheme.clone_from(scheme);
                }
                _ => {}
            }
        }

        mentions
            .into_values()
            .map(|mut m| {
                m.identifier = m
                    .raw_identifier
                    .as_deref()
                    .and_then(|raw| normalise_identifier(raw, m.country.as_deref()));
                m
            })
            .collect()
    }

    /// Turn the notice-local role references into party facts now that each
    /// mention has a canonical Organization.
    fn bind_organizations(&mut self, mentions: &[Mention], ids: &[i64]) {
        let by_section: HashMap<&str, i64> = mentions
            .iter()
            .map(|m| m.section_id.as_str())
            .zip(ids.iter().copied())
            .collect();
        for (scope, role, target) in std::mem::take(&mut self.roles) {
            // A reference to something that is not an Organization section (a
            // touchpoint, a lot, a result) is notice-layer detail, not a party.
            let Some(&organization_id) = by_section.get(target.as_str()) else { continue };
            let fact = Fact::Party {
                role,
                organization_id,
                notice_id: self.notice_id,
                section_id: target,
            };
            match scope {
                Scope::Tender => {
                    self.facts.insert(fact);
                }
                Scope::Lot(key) => {
                    if let Some(lot) = self.lots.iter_mut().find(|l| l.key == key) {
                        lot.facts.insert(fact);
                    }
                }
            }
        }
    }

    fn kind(&self) -> &'static str {
        match self.subtype.as_deref() {
            Some(REGISTRATION_SUBTYPE) => "registration",
            _ => "procedure",
        }
    }
}

/// Group notices into Tenders and fold each group's chain.
fn group(states: Vec<NoticeState>) -> Vec<TenderProjection> {
    let mut groups: BTreeMap<(String, Option<String>, i64), Vec<NoticeState>> = BTreeMap::new();
    for state in states {
        // Islands are keyed by their own notice so they can never collide;
        // keyed procedures share a bucket per (source, key).
        let island = if state.procedure_key.is_none() { state.notice_id } else { 0 };
        groups
            .entry((state.source.clone(), state.procedure_key.clone(), island))
            .or_default()
            .push(state);
    }

    groups
        .into_values()
        .map(|mut chain| {
            chain.sort_by(|a, b| {
                (a.published_at, &a.publication_id, a.notice_id)
                    .cmp(&(b.published_at, &b.publication_id, b.notice_id))
            });
            let first = &chain[0];
            let projection_kind = first.kind().to_owned();
            let procedure_key = first.procedure_key.clone();
            let island_notice_id = procedure_key.is_none().then_some(first.notice_id);
            TenderProjection {
                source: first.source.clone(),
                procedure_key,
                island_notice_id,
                kind: projection_kind,
                versions: fold(&chain),
            }
        })
        .collect()
}

/// Resolve the chain: each version is the notice's own values laid over the
/// previous version's, per field.
fn fold(chain: &[NoticeState]) -> Vec<TenderVersion> {
    let mut versions: Vec<TenderVersion> = Vec::with_capacity(chain.len());
    for state in chain {
        let previous = versions.last();
        let mut facts = previous.map(|p| p.facts.clone()).unwrap_or_default();
        supersede(&mut facts, &state.facts);

        let mut lots: Vec<LotState> = previous.map(|p| p.lots.clone()).unwrap_or_default();
        for published in &state.lots {
            match lots.iter_mut().find(|l| l.key == published.key) {
                Some(carried) => {
                    carried.kind.clone_from(&published.kind);
                    supersede(&mut carried.facts, &published.facts);
                }
                None => lots.push(published.clone()),
            }
        }
        lots.sort_by(|a, b| a.key.cmp(&b.key));

        versions.push(TenderVersion {
            caused_by_notice_id: state.notice_id,
            published_at: state.published_at,
            notice_subtype: state.subtype.clone(),
            publication_id: state.publication_id.clone(),
            facts,
            lots,
        });
    }
    versions
}

/// Supersession per field: a field the notice republishes replaces the carried
/// one entirely; a field it is silent about is left alone.
fn supersede(carried: &mut BTreeSet<Fact>, published: &BTreeSet<Fact>) {
    let republished: BTreeSet<(&str, &str)> = published.iter().map(Fact::key).collect();
    carried.retain(|fact| !republished.contains(&fact.key()));
    carried.extend(published.iter().cloned());
}

/// Which Lot a value belongs to — the nearest enclosing Lot section, or the
/// Tender when there is none.
fn scope_of(sections: &HashMap<&str, &store::Section>, section_id: &str) -> Scope {
    match enclosing(sections, section_id, LOT_KINDS) {
        Some(lot) => Scope::Lot(lot.to_owned()),
        None => Scope::Tender,
    }
}

/// The nearest section of one of `kinds`, starting at `section_id` itself and
/// walking up the parent chain. This is how a value finds the entity it
/// describes: eForms hangs values off the deepest node that carries them, and
/// the canonical scope is the nearest enclosing entity above it.
fn enclosing<'a>(
    sections: &HashMap<&str, &'a store::Section>,
    section_id: &str,
    kinds: &[&str],
) -> Option<&'a str> {
    let mut current = section_id;
    // Bounded by the section count: the parent chain is a tree, but guard
    // anyway so malformed data cannot spin.
    for _ in 0..sections.len().max(1) {
        let section = sections.get(current)?;
        if kinds.contains(&section.kind.as_str()) {
            return Some(section.id.as_str());
        }
        current = section.parent.as_deref()?;
    }
    None
}

/// The business-term stem of a source field id: `BT-21-Lot` → `BT-21`,
/// `BT-131(d)-Lot` → `BT-131(d)`.
fn stem(field_id: &str) -> &str {
    let mut parts = field_id.match_indices('-');
    parts.next();
    match parts.next() {
        Some((i, _)) => &field_id[..i],
        None => field_id,
    }
}

fn canonical_name(table: &[(&str, &str)], stem: &str) -> Option<String> {
    table.iter().find(|(source, _)| *source == stem).map(|(_, name)| (*name).to_owned())
}

/// The role an id-ref names: `OPT-300-Procedure-Buyer` → `Procedure-Buyer`.
/// The OPT-300/301 families are exactly eForms' organization references.
fn role_name(field_id: &str) -> Option<String> {
    for prefix in ["OPT-300-", "OPT-301-"] {
        if let Some(rest) = field_id.strip_prefix(prefix) {
            return Some(rest.to_owned());
        }
    }
    None
}

/// Normalise an official identifier and gate it on plausibility before letting
/// it merge two mentions into one Organization
/// (docs/research/ted-legacy-mapping.md §6: 16% of real ids are junk).
///
/// A VAT id carries its country in its own prefix and is scoped by it; a
/// national registry number is only unique inside its country, so it is scoped
/// by the mention's country and stays separate when that is unknown.
pub fn normalise_identifier(raw: &str, country: Option<&str>) -> Option<Identifier> {
    let value: String = raw
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect();

    if value.len() < 4 || !value.chars().any(|c| c.is_ascii_digit()) {
        return None;
    }
    // All-zero and single-character fillers ("0000", "999999", "n/a" variants
    // that survived the digit test).
    if value.chars().filter(|c| c.is_ascii_digit()).all(|c| c == '0') {
        return None;
    }
    if value.chars().skip(1).all(|c| c == value.as_bytes()[0] as char) {
        return None;
    }

    let vat_prefix: String = value.chars().take(2).collect();
    let is_vat = vat_prefix.chars().all(|c| c.is_ascii_alphabetic())
        && value[2..].chars().any(|c| c.is_ascii_digit());
    if is_vat {
        Some(Identifier { country: Some(vat_prefix), kind: "vat".into(), value })
    } else {
        Some(Identifier {
            country: country.map(str::to_owned),
            kind: "national".into(),
            value,
        })
    }
}

fn first_id(parsed: &Parsed, field_id: &str) -> Option<String> {
    parsed.values.iter().find(|v| v.field_id == field_id).and_then(|v| match &v.value {
        NoticeValue::Id { value, .. } => Some(value.clone()),
        _ => None,
    })
}

fn first_code(parsed: &Parsed, field_id: &str) -> Option<String> {
    parsed.values.iter().find(|v| v.field_id == field_id).and_then(|v| match &v.value {
        NoticeValue::Code { code, .. } => Some(code.clone()),
        _ => None,
    })
}

fn first_date(parsed: &Parsed, field_id: &str) -> Option<i64> {
    parsed.values.iter().find(|v| v.field_id == field_id).and_then(|v| match &v.value {
        NoticeValue::Date { utc_seconds, .. } => Some(*utc_seconds),
        _ => None,
    })
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn business_term_stems_survive_their_context_suffix() {
        assert_eq!(stem("BT-21-Lot"), "BT-21");
        assert_eq!(stem("BT-21-Procedure"), "BT-21");
        assert_eq!(stem("BT-131(d)-Lot"), "BT-131(d)");
        assert_eq!(stem("OPP-070-notice"), "OPP-070");
        assert_eq!(stem("BT-04"), "BT-04");
    }

    #[test]
    fn roles_come_from_the_organization_reference_families() {
        assert_eq!(role_name("OPT-300-Procedure-Buyer").as_deref(), Some("Procedure-Buyer"));
        assert_eq!(role_name("OPT-301-Lot-Mediator").as_deref(), Some("Lot-Mediator"));
        assert_eq!(role_name("BT-137-Lot"), None);
    }

    /// The plausibility gate of docs/research/ted-legacy-mapping.md §6: merging
    /// is only allowed to happen on identifiers that can actually identify.
    #[test]
    fn only_plausible_identifiers_are_allowed_to_merge() {
        let vat = normalise_identifier("NL 8045.95859B01", Some("NLD")).expect("a VAT id");
        assert_eq!(vat.kind, "vat");
        assert_eq!(vat.value, "NL804595859B01");
        // The prefix, not the mention's country, scopes a VAT id.
        assert_eq!(vat.country.as_deref(), Some("NL"));
        // The same id written differently normalises to the same profile.
        assert_eq!(normalise_identifier("nl804595859b01", None), Some(vat));

        let national = normalise_identifier("65993390", Some("CZE")).expect("a registry number");
        assert_eq!(national.kind, "national");
        assert_eq!(national.country.as_deref(), Some("CZE"));

        // Junk from the real corpus, all of which must stay provisional.
        assert_eq!(normalise_identifier("Romania", None), None); // no digit
        assert_eq!(normalise_identifier("n/a", None), None); // too short, no digit
        assert_eq!(normalise_identifier("000000", None), None); // all zeros
        assert_eq!(normalise_identifier("111111", None), None); // filler
        assert_eq!(normalise_identifier("12", None), None); // too short
    }

    #[test]
    fn supersession_replaces_a_field_wholesale_and_leaves_others_alone() {
        let text = |field: &str, lang: &str, value: &str| Fact::Text {
            field: field.into(),
            lang: Some(lang.into()),
            value: value.into(),
        };
        let mut carried: BTreeSet<Fact> =
            [text("title", "ENG", "Roof works"), text("title", "DEU", "Dacharbeiten"),
             text("description", "ENG", "unchanged")]
                .into_iter()
                .collect();

        // A corrigendum republishing only the English title drops the stale
        // German one — they are one field — but not the description.
        supersede(&mut carried, &[text("title", "ENG", "Roof works, revised")].into_iter().collect());

        assert_eq!(carried.len(), 2);
        assert!(carried.contains(&text("title", "ENG", "Roof works, revised")));
        assert!(carried.contains(&text("description", "ENG", "unchanged")));
    }
}
