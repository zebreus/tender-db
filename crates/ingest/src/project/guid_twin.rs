//! Issue 530 unit 3: rule 530-T, the within-notice GUID twin, as a PURE function of one notice copy
//! ([`guid_twin_decide`], [`guid_twin_refusals`]), its name guard, and (Build 2) the gate that applies
//! it in the projection ([`guid_twin_gate`]) with the frozen KEEP_530 ([`keep_530`]).
//!
//! Some German eForms platforms mint a fresh BT-501 UUID per ORG section per notice, so one body is a
//! row per notice. When ONE notice copy prints one name-safe name under two or more distinct v4 values,
//! the notice itself proves the values are no entity key (`530-triage/decision.md`, the pair-scoped (a)
//! "530-T", critic amendments A1–A12 adopted). The rule refuses those values' identifiers — the mention
//! then takes the issue-234 identifier-NULL path, `raw_identifier` kept — except values on the frozen,
//! value-keyed `KEEP_530` (A4), names that are not safe as a fallback key (A8), groups with no
//! country, and role-incoherent groups (A7).
//!
//! **Build 1b (R2, 2026-10-19, wf_5d7ad2e8-333: FAILED on 3 W; fixes F1/F2 decided).**
//! - F1, names: R2 convicted two keys whose fold lands on a row that already holds another body —
//!   `Vergabekammer NRW` (target 3353580, an umbrella over VK Rheinland and VK Westfalen) and
//!   `Geschäftsstelle der Vergabekammer` (target 6622773, Thuringia's chamber; the planned rows were VK
//!   Westfalen printing the same function name). Both, and three precautionary spellings, are on
//!   [`GUID_TWIN_UNSAFE_NAMES`] as a LIST: a `Vergabekammer <Land>` pattern would refuse the right folds
//!   into single-chamber Länder (Niedersachsen, Baden-Württemberg, Brandenburg, Schleswig-Holstein).
//!   `geschäftsstelle` joins [`GUID_TWIN_OFFICE_NOUNS`] (stored folded, as compared).
//! - F2, roles (A7 as written): a twin group whose sections' roles BY THIS NOTICE COPY'S OWN OPT-300 /
//!   OPT-301 references ([`guid_twin_section_roles`]; never `tender_version_parties`) are
//!   role-incoherent ([`store::guid_twin_role_mix`]: Lot-ReviewOrg beside a buyer-side role —
//!   Procedure-Buyer, Procedure-SProvider, Lot-/Part-AddInfo, Lot-/Part-TenderReceipt — or a tenderer
//!   beside a role outside the tenderer class) binds whole ([`TwinBind::RoleMix`]). R2's W3 was the AOK
//!   Niedersachsen copy pair 24253392 / 26483158, whose Lot-ReviewOrg section printed the AOK's name
//!   over VK Bund's phone and NUTS. The guard binds the whole group, so it also binds the right folds of
//!   a buyer that printed ITSELF into the review slot (28 of R2's 30 role-incoherent reads were that
//!   shape): it trades those folds — the rows stay split as today, under (c) — for never attaching a
//!   review body's role to the buyer's row.
//!
//! **Build 2: the gate is WIRED.** `NoticeState::mentions_and_dropped` calls [`guid_twin_gate`] right
//! after it sets each mention's gated identifier and before its `extracted` loop (so `ExtractedWinners`
//! sees the final identity): a refused section's `identifier` becomes `None`, its `raw_identifier` and
//! `scheme` stay as published. The gate reads the COMPILED statics only — [`keep_530`] (the committed
//! `crates/ingest/sdk/guid-keep-530.tsv`, [`GUID_KEEP_530_VERSION`]) and [`GUID_TWIN_UNSAFE_NAMES`],
//! never a run's `extra_unsafe_names` — and the ORG sections alone (`ORG-…`, the range the dry planner
//! reads, so the stock plan and the gate see one section set). The dry and wet `dissolve-guid-twins`
//! runs pass the same statics to the planner (A3 iii), so the plan, the gate and any rebuild decide with
//! one ruler. The buyer key and the parsed-side link-guard tokens (issue 369's `buyer_key`, 481's
//! `buyer_tokens_of` over `NoticeState::published_mentions`) keep the published identity: the deploy
//! moves no Tender key. The resolved-organization token `o:<id>` (`add_buyer_org_tokens`) follows the
//! binding — a refused buyer gets the name-keyed row; before the wet run that touches only notices first
//! resolved after the deploy (recorded mentions keep their organization under `publishes`), after it
//! every rehomed notice when its stamped Tender re-projects. Dropping `o:<GUID row>` loses no overlap
//! (the published identifier and `r:` tokens cover it on every notice of that single-Tender row);
//! adding `o:<name row>` can only widen overlap, so no link the guard admitted before is refused.
//!
//! **Identity is now contextual.** A tool that rebuilds a mention's identity from its stored row alone
//! ([`super::gated_identifier`]'s callers: 513's nhs-body canary, 511's guard 3) sees a GUID where the
//! fold refused one. None of them reads DE GUID mentions today; a reader that must agree with the fold
//! re-derives the twin from the notice's stored mentions, as the planner and the recurrence arm do.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::sync::OnceLock;

use store::{GuidKeep, GuidTwinSectionRoles, NoticeValue, Parsed, TwinBind, TwinDecision, TwinGroup, TwinView};

use super::{
    DE1_FIELD_ALIASES, PREFIX_FUNCTION_WORDS, canonical_country, content_words, gated_identifier, generic_body_name,
    guard_name_words, merge_initials, role_name,
};

/// Issue 530 (decision §5, A8 ii): names that are not safe as a twin's fallback key, beside
/// `generic_body_name`'s list. A 530-ONLY list on purpose: adding these to `GENERIC_BODY_NAMES` would
/// change the head and prefix tokens of 481's buyer guard, a different feature. Seeded with the
/// decision's generics (the precision judge's GETs: `Zentrale Vergabestelle`, `Tribunal administratif`;
/// the one-word office names, which the content-word test already binds, listed so the list reads
/// whole), the polity umbrellas and the person placeholders of A8; R2 convictions are added here.
/// Build 2: COMPILED is the only way in — the gate never reads a run's `extra_unsafe_names`, and a wet
/// `dissolve-guid-twins` refuses them (A3 iii), so a dry re-run with extra names is a measurement and an
/// R2 finding ships as a name on this list (the R2 re-read at salt 530003, `530-triage/r2b`, read
/// 60 R / 0 N / 0 W: nothing added).
pub const GUID_TWIN_UNSAFE_NAMES: &[&str] = &[
    // The decision's generic office and body names.
    "Vergabekammer",
    "Vergabestelle",
    "Zentrale Vergabestelle",
    "Beschaffungsstelle",
    "Tiefbauamt",
    "Hochbauamt",
    "Bauamt",
    "Bauverwaltung",
    "Gemeindeverwaltung",
    "Stadtverwaltung",
    "Finanzbehörde",
    "Tribunal administratif",
    // A8 ii: polity umbrellas (with `<Freistaat|Land|Bundesland|Kanton|Canton> <one word>`, below).
    "Bundesrepublik Deutschland",
    "Freie und Hansestadt Hamburg",
    "Freie Hansestadt Bremen",
    "Republik Österreich",
    // The umbrella shape's hyphenated Länder and cantons, listed so a space-written spelling (`Land
    // Nordrhein Westfalen`, no hyphen to group it as one word) binds too: folded words meet either way.
    // The two-word cantons have no hyphen at all.
    "Land Baden-Württemberg",
    "Land Mecklenburg-Vorpommern",
    "Land Nordrhein-Westfalen",
    "Land Rheinland-Pfalz",
    "Land Sachsen-Anhalt",
    "Land Schleswig-Holstein",
    "Kanton Basel-Stadt",
    "Kanton Basel-Landschaft",
    "Kanton St. Gallen",
    "Kanton Appenzell Ausserrhoden",
    "Kanton Appenzell Innerrhoden",
    // A8 ii: person placeholders (no GUID rows under them today; this guards future notices).
    "Natürliche Person",
    "Natürliche Personen",
    "Privatperson",
    // R2 (2026-10-19, wf_5d7ad2e8-333; `530-triage/r2/r2-result.md` F1): names that more than one
    // review chamber prints, so their fallback row already mixes bodies. EXACT names, never a
    // `Vergabekammer <Land>` pattern: that would refuse the right folds into the single-chamber Länder
    // (Vergabekammer Niedersachsen, Baden-Württemberg, Brandenburg, Schleswig-Holstein; Rheinland and
    // Westfalen are one chamber each). The list matches the WHOLE folded name, so the MV key
    // `Geschäftsstelle der Vergabekammern beim Ministerium … Mecklenburg-Vorpommern` stays out.
    // The two convicted keys: W1, target 3353580 (holds VK Rheinland, Köln, and VK Westfalen, Münster);
    // W2, target 6622773 (Thuringia's chamber, Weimar; the planned rows were VK Westfalen).
    "Vergabekammer NRW",
    "Geschäftsstelle der Vergabekammer",
    // Precautionary (no planned key under them in job 2659; future notices): the long NRW spelling
    // (folded words meet with or without its hyphen), the umbrella over Nordbayern and Südbayern, the
    // bare plural.
    "Vergabekammer Nordrhein-Westfalen",
    "Vergabekammer Bayern",
    "Geschäftsstelle der Vergabekammern",
];

/// A8 i: legal-form tokens, dropped before the content-word test and both generic lists, so
/// `Stadtwerke GmbH` reads as `stadtwerke` (one content word, and generic) and not as two words.
/// Matched on folded words with initials merged (`e.V.` → `ev`, `S.A.R.L.` → `sarl`, `AöR` → `aor`).
pub const GUID_TWIN_LEGAL_FORMS: &[&str] =
    &["gmbh", "mbh", "ggmbh", "ag", "kg", "kgaa", "se", "ug", "ev", "aor", "co", "ohg", "gbr", "sa", "sarl", "sas"];

/// A8 ii: the polity umbrella heads: `<head> <one content word>` (`Freistaat Bayern`, `Land Berlin`,
/// `Kanton Zürich`, `Canton de Genève`, `Land Nordrhein-Westfalen`, `Der Freistaat Bayern`) names a
/// state, not the body that buys under it ([`polity_umbrella`]).
pub const GUID_TWIN_POLITY_HEADS: &[&str] = &["freistaat", "land", "bundesland", "kanton", "canton"];

/// R2's office nouns: a planned key of exactly 2 content words led by one (`Vergabekammer Rheinland`,
/// `Amt 66`, `Referat Beschaffung`) is read whole in R2 (decision R2 "Also read"). Stored FOLDED, as
/// [`office_two_words`] compares them against [`guid_twin_name_words`] (`ä` → `a`, so `Geschäftsstelle`
/// is `geschaftsstelle`; pinned by `every_office_noun_is_stored_folded`).
pub const GUID_TWIN_OFFICE_NOUNS: &[&str] = &[
    // R2 (2026-10-19) F1: a `Geschäftsstelle X` key is read whole.
    "geschaftsstelle",
    "amt",
    "vergabestelle",
    "vergabekammer",
    "bauamt",
    "referat",
    "abteilung",
    "fachbereich",
    "zentrale",
    "hochbauamt",
    "tiefbauamt",
    "dezernat",
    "sachgebiet",
    "stabsstelle",
    "dienststelle",
    "beschaffungsstelle",
];

/// A name as the 530 guard reads it: the buyer guard's folded words ([`guard_name_words`]), initials
/// merged ([`merge_initials`], as `generic_body_name` builds its set), legal-form tokens dropped (A8 i).
pub fn guid_twin_name_words(name: &str) -> Vec<String> {
    merge_initials(&guard_name_words(name)).into_iter().filter(|w| !GUID_TWIN_LEGAL_FORMS.contains(&w.as_str())).collect()
}

/// A8 ii's umbrella shape, `<Freistaat|Land|Bundesland|Kanton|Canton> <one word>`, read on the name's
/// SOURCE tokens (its whitespace-separated words, [`store::org_name_norm`]) rather than the folded words,
/// which split on hyphens: so a hyphenated state (`Nordrhein-Westfalen`, `Basel-Stadt`, `Bâle-Ville`) is
/// one word, as A8 means it. Tokens that fold to nothing, to function words only (a leading `Der`,
/// `Das`, `Le`, the `de` of `Canton de Genève`) or to one legal form are skipped first; then the first
/// token must fold to exactly one head and exactly one token may follow it.
fn polity_umbrella(name: &str) -> bool {
    let tokens: Vec<Vec<String>> = store::org_name_norm(name)
        .split_whitespace()
        .map(|token| merge_initials(&guard_name_words(token)))
        .filter(|words| {
            !words.is_empty()
                && !words.iter().all(|w| PREFIX_FUNCTION_WORDS.contains(&w.as_str()))
                && !(words.len() == 1 && GUID_TWIN_LEGAL_FORMS.contains(&words[0].as_str()))
        })
        .collect();
    tokens.len() == 2 && tokens[0].len() == 1 && GUID_TWIN_POLITY_HEADS.contains(&tokens[0][0].as_str())
}

/// [`GUID_TWIN_UNSAFE_NAMES`] folded as [`guid_twin_name_words`] folds a name, built once.
fn unsafe_names() -> &'static HashSet<String> {
    static SET: OnceLock<HashSet<String>> = OnceLock::new();
    SET.get_or_init(|| GUID_TWIN_UNSAFE_NAMES.iter().map(|n| guid_twin_name_words(n).join(" ")).collect())
}

/// Why `name` is not safe as a twin group's fallback key, or `None` when it is (decision §5, A8):
/// fewer than 2 content words once legal forms are dropped; `generic_body_name`; the 530 list or a polity
/// umbrella; or one of the run's `extra` names (folded the same way).
pub fn guid_twin_unsafe_name(name: &str, extra: &[String]) -> Option<TwinBind> {
    let words = guid_twin_name_words(name);
    if content_words(&words) < 2 {
        return Some(TwinBind::ContentWords);
    }
    if generic_body_name(&words) {
        return Some(TwinBind::Generic);
    }
    let joined = words.join(" ");
    if unsafe_names().contains(&joined)
        || (GUID_TWIN_POLITY_HEADS.contains(&words[0].as_str()) && content_words(&words) == 2)
        || polity_umbrella(name)
    {
        return Some(TwinBind::UnsafeList);
    }
    if extra.iter().any(|e| guid_twin_name_words(e).join(" ") == joined) {
        return Some(TwinBind::Extra);
    }
    None
}

/// Whether a (safe) name is a 2-content-word key led by an office noun: R2 reads every such key.
fn office_two_words(name: &str) -> bool {
    let words = guid_twin_name_words(name);
    let content: Vec<&String> = words.iter().filter(|w| content_words(std::slice::from_ref(*w)) == 1).collect();
    content.len() == 2 && GUID_TWIN_OFFICE_NOUNS.contains(&content[0].as_str())
}

/// A7 (Build 1b): the ORG role one id value of a notice copy references, as the projection reads it —
/// the mapping rule 530-T's role guard and the dry planner share (store takes it as a
/// `store::GuidTwinRoleRef`). A DE-1.x alias source ([`DE1_FIELD_ALIASES`]) is read as its eForms id
/// and counts as a reference whatever its stored flag, as `normalise_de1_parse` marks it
/// (`de1_mark_reference`); then an OPT-300 / OPT-301 id-ref outside the `ojs` scheme (a chain edge)
/// names [`role_name`]'s role (`OPT-301-Lot-ReviewOrg` → `Lot-ReviewOrg`). Anything else names none:
/// identifiers, other references, and the legacy `TED-` address roles (rule 530-T reads eForms
/// references only). So the raw stored copy and the normalised parse the gate sees read alike.
pub fn guid_twin_role_ref(field_id: &str, scheme: Option<&str>, is_ref: bool) -> Option<String> {
    let (field_id, is_ref) = match DE1_FIELD_ALIASES.iter().find(|(de1, _)| *de1 == field_id) {
        Some((_, eforms)) => (*eforms, true),
        None => (field_id, is_ref),
    };
    if !is_ref || scheme == Some("ojs") || !(field_id.starts_with("OPT-300-") || field_id.starts_with("OPT-301-")) {
        return None;
    }
    role_name(field_id)
}

/// A7 (Build 1b): the roles ONE notice copy gives its sections by its own OPT-300 / OPT-301
/// references — `section_id -> roles`, sorted and distinct — never by `tender_version_parties` (a
/// party row is the projection's verdict, after the role census may have moved a body; the guard reads
/// what the notice printed). The ONE place the role read is spelled: the gate (Build 2) calls it in
/// `mentions_and_dropped` on the parsed copy, raw or normalised alike ([`guid_twin_role_ref`]); the dry
/// planner folds the copy's stored `notice_ids` rows with the same [`store::guid_twin_fold_roles`] and
/// [`guid_twin_role_ref`]. Pure.
pub fn guid_twin_section_roles(parsed: &Parsed) -> GuidTwinSectionRoles {
    store::guid_twin_fold_roles(
        parsed.values.iter().filter_map(|v| match &v.value {
            NoticeValue::Id { value, is_ref, scheme } => Some((v.field_id.as_str(), scheme.as_deref(), value.as_str(), *is_ref)),
            _ => None,
        }),
        guid_twin_role_ref,
    )
}

/// Rule 530-T over ONE notice copy (a TED copy and a DÖE copy are each decided alone):
///
/// 1. Candidates: sections whose gated identifier ([`gated_identifier`] under the canonical country,
///    the gate's own path) has kind `national` and a value passing `idgate::uuid_v4`.
/// 2. Groups: by `(identifier.country, store::org_name_norm(name))` — the census's twin key made finer
///    by the register country, the key the 234 fallback resolves on. A candidate with an empty name
///    joins no group; one with no country forms a group that always binds (`no_country`).
/// 3. Twin: a group holding 2 or more DISTINCT values.
/// 4. Refusal: every section of a twin whose value is not in `keep` — unless the whole group binds:
///    the name is not safe ([`guid_twin_unsafe_name`]); or (A7, Build 1b) the members' roles by the
///    copy's own references ([`TwinView::roles`], [`guid_twin_section_roles`]) are role-incoherent
///    ([`store::guid_twin_role_mix`]: `Lot-ReviewOrg` or `Part-ReviewOrg` beside a buyer-side role —
///    `Procedure-Buyer`, `Procedure-SProvider`, `Lot-`/`Part-AddInfo`, `Lot-`/`Part-TenderReceipt`,
///    each read after its `Procedure-`/`Lot-`/`Part-` scope — or a `Tenderer`, `Tenderer-MainCont` or
///    `Tenderer-SubCont` beside any other role); or every value is in `keep`. The role guard binds a
///    buyer that printed itself into the review slot too: those right folds are traded for never giving
///    a review body's (or a bidder's) role to the buyer's row (the rows stay split, under (c)).
///
/// Pure: no DB read, no state. The planner passes it to store as a `store::GuidTwinRule`.
pub fn guid_twin_decide(sections: &[TwinView<'_>], keep: &GuidKeep, extra_unsafe_names: &[String]) -> TwinDecision {
    let mut decision = TwinDecision::default();
    type Members = (Vec<(String, String)>, BTreeSet<String>);
    let mut groups: BTreeMap<(Option<String>, String), Members> = BTreeMap::new();
    for s in sections {
        let country = s.country.map(canonical_country);
        let Some(id) = gated_identifier(s.raw_identifier, s.scheme, country.as_deref()) else { continue };
        if id.kind != "national" || !crate::idgate::uuid_v4(&id.value) {
            continue;
        }
        decision.candidates += 1;
        let norm = store::org_name_norm(s.name);
        if norm.is_empty() {
            decision.unnamed += 1;
            continue;
        }
        let group = groups.entry((id.country, norm)).or_default();
        group.0.push((s.section_id.to_owned(), id.value));
        group.1.extend(s.roles.iter().cloned());
    }
    for ((country, name_norm), (members, roles)) in groups {
        let distinct = members.iter().map(|(_, v)| v.as_str()).collect::<BTreeSet<_>>().len();
        if distinct < 2 {
            continue;
        }
        let roles: Vec<String> = roles.into_iter().collect();
        let bind = if country.is_none() {
            Some(TwinBind::NoCountry)
        } else if let Some(reason) = guid_twin_unsafe_name(&name_norm, extra_unsafe_names) {
            Some(reason)
        } else if store::guid_twin_role_mix(&roles).is_some() {
            Some(TwinBind::RoleMix)
        } else if members.iter().all(|(_, v)| keep.contains(v)) {
            Some(TwinBind::Keep)
        } else {
            None
        };
        let refused: Vec<String> = match bind {
            Some(_) => Vec::new(),
            None => members.iter().filter(|(_, v)| !keep.contains(v)).map(|(s, _)| s.clone()).collect(),
        };
        decision.refused.extend(refused.iter().cloned());
        decision.groups.push(TwinGroup {
            office_two_words: bind.is_none() && office_two_words(&name_norm),
            country,
            name_norm,
            members,
            distinct_values: distinct,
            bind,
            refused,
            roles,
        });
    }
    decision
}

/// Rule 530-T's candidate test on ONE stored mention, the gate's own path ([`guid_twin_decide`]'s
/// step 1): `Some((value, register country))` when its gated identifier ([`gated_identifier`] under the
/// canonical country) has kind `national` and passes `idgate::uuid_v4` — the identity a rebuild binds
/// the mention to when the rule does not refuse it. The recurrence arm's reverse-drift half and its
/// restore read it (`store::GuidTwinGatedV4`).
pub fn guid_twin_gated_v4(raw: Option<&str>, scheme: Option<&str>, country: Option<&str>) -> Option<(String, Option<String>)> {
    let country = country.map(canonical_country);
    let id = gated_identifier(raw, scheme, country.as_deref())?;
    (id.kind == "national" && crate::idgate::uuid_v4(&id.value)).then_some((id.value, id.country))
}

/// The section ids whose identifier rule 530-T refuses in one notice copy ([`guid_twin_decide`]'s
/// `refused`): what the gate (Build 2) sets to `identifier = None`.
pub fn guid_twin_refusals(sections: &[TwinView<'_>], keep: &GuidKeep, extra_unsafe_names: &[String]) -> BTreeSet<String> {
    guid_twin_decide(sections, keep, extra_unsafe_names).refused
}

// ---- Build 2: the frozen KEEP_530 and the gate ----

/// Build 2: the version of the committed KEEP file, pinned to its `# version:` header
/// (`keep_530_file_is_frozen_as_decided`). A KEEP-add commit bumps both together (`keep-530-job2660`
/// is job 2660's freeze, unappended). The dry plan reports it and the wet run refuses a stored plan read
/// under any other `keep_version` or `keep_size`, so a KEEP-add commit forces a fresh dry plan either way.
pub const GUID_KEEP_530_VERSION: &str = "keep-530-job2660";

/// The committed KEEP_530 (`530-triage/decision.md` "KEEP_530", 513's `nhs-ods.tsv` precedent): job
/// 2660's `keep_candidates`, generated by `530-triage/build2/gen_guid_keep.py`.
const GUID_KEEP_530_TSV: &str = include_str!("../../sdk/guid-keep-530.tsv");

/// One row of the KEEP file: the decision's columns (country, value, org id at freeze, mentions,
/// tenders, reason); an empty cell is `None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuidKeep530Row {
    pub country: Option<String>,
    /// The `GuidKeep::key` form (undashed, upper case).
    pub value: String,
    pub org_id: i64,
    pub mentions: Option<u64>,
    pub tenders: Option<u64>,
    pub reason: String,
}

/// The KEEP file's rows, parsed once. A malformed line PANICS (the `nhs_ods::parse` idiom): the file
/// is compiled in, so a bad commit fails every test that touches the gate, never a fold in production.
pub fn keep_530_rows() -> &'static [GuidKeep530Row] {
    static ROWS: OnceLock<Vec<GuidKeep530Row>> = OnceLock::new();
    ROWS.get_or_init(|| parse_keep_530(GUID_KEEP_530_TSV))
}

/// The `# version:` header of a KEEP file.
pub fn keep_530_file_version(tsv: &str) -> Option<&str> {
    tsv.lines().filter_map(|l| l.strip_prefix("# version:")).map(str::trim).next()
}

/// The committed file's text (its header and rows), for the frozen-file test and the report.
pub fn keep_530_tsv() -> &'static str {
    GUID_KEEP_530_TSV
}

fn parse_keep_530(tsv: &str) -> Vec<GuidKeep530Row> {
    let opt = |s: &str| (!s.is_empty()).then(|| s.to_owned());
    let num = |s: &str, line: &str| -> Option<u64> {
        (!s.is_empty()).then(|| s.parse().unwrap_or_else(|_| panic!("guid-keep-530.tsv: a bad count {s:?} in {line:?}")))
    };
    tsv.lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|line| {
            let f: Vec<&str> = line.split('\t').collect();
            if f.len() != 6 {
                panic!("guid-keep-530.tsv: a row without six fields: {line:?}");
            }
            GuidKeep530Row {
                country: opt(f[0]),
                value: f[1].to_owned(),
                org_id: f[2].parse().unwrap_or_else(|_| panic!("guid-keep-530.tsv: a bad org id in {line:?}")),
                mentions: num(f[3], line),
                tenders: num(f[4], line),
                reason: f[5].to_owned(),
            }
        })
        .collect()
}

/// KEEP_530 as the gate and the planner read it: the committed file's values, value-keyed (A4:
/// `GuidKeep` normalises a published dashed lower-case value to the stored key, the country ignored),
/// built once.
pub fn keep_530() -> &'static GuidKeep {
    static KEEP: OnceLock<GuidKeep> = OnceLock::new();
    KEEP.get_or_init(|| keep_530_rows().iter().map(|r| r.value.as_str()).collect())
}

/// The section range rule 530-T reads in the projection: ORG sections (`ORG-…`), exactly the range the
/// dry planner reads back from `organization_mentions` (`section_id >= 'ORG-' AND section_id <
/// 'ORG.'`), so the gate and the stock plan see one section set. Legacy parties are `ORG-<n>` sections
/// (the r209, text and FTS parsers mint them), so both the gate and the planner cover them; only
/// sdk-0.1 parties fall outside the range, and they carry no identifier. An eForms copy whose
/// publisher-supplied org ids fall outside `ORG-…` is read by neither.
pub fn guid_twin_org_section(section_id: &str) -> bool {
    section_id >= "ORG-" && section_id < "ORG."
}

/// The gate's views of one parsed copy: its ORG-section mentions ([`guid_twin_org_section`]), each
/// with the roles the copy's own references give it (`roles`, from [`guid_twin_section_roles`]).
pub fn guid_twin_gate_views<'a>(mentions: &'a [store::Mention], roles: &'a GuidTwinSectionRoles) -> Vec<TwinView<'a>> {
    mentions
        .iter()
        .filter(|m| guid_twin_org_section(&m.section_id))
        .map(|m| TwinView {
            section_id: &m.section_id,
            name: &m.name,
            country: m.country.as_deref(),
            raw_identifier: m.raw_identifier.as_deref(),
            scheme: m.scheme.as_deref(),
            roles: roles.get(m.section_id.as_str()).map_or(&[], Vec::as_slice),
        })
        .collect()
}

/// Build 2: rule 530-T applied to one notice copy's mentions in the projection, with the COMPILED
/// statics ([`keep_530`], [`GUID_TWIN_UNSAFE_NAMES`], no extra names): every section the rule refuses
/// gets `identifier = None` (`raw_identifier` and `scheme` stay as published), so the resolver binds it
/// through the issue-234 identifier-NULL arm — the `(org_name_norm, register_jurisdiction(country))`
/// row, `ORDER BY id LIMIT 1`, or one minted. Returns how many it refused. See [`guid_twin_gate_with`].
pub fn guid_twin_gate(parsed: &Parsed, mentions: &mut [store::Mention]) -> usize {
    guid_twin_gate_with(parsed, mentions, keep_530())
}

/// [`guid_twin_gate`] with an explicit KEEP (tests). Cheap on the common path: it returns before any
/// work unless two ORG mentions carry a national `uuid_v4` identifier (nearly every notice; sdk-0.1
/// never sets one), and reads the copy's role references only when the rule finds a twin group (the
/// group set depends on neither the roles nor KEEP, as the planner's `guid_twin_has_group` reads it).
pub fn guid_twin_gate_with(parsed: &Parsed, mentions: &mut [store::Mention], keep: &GuidKeep) -> usize {
    let candidates = mentions
        .iter()
        .filter(|m| {
            guid_twin_org_section(&m.section_id)
                && m.identifier.as_ref().is_some_and(|i| i.kind == "national" && crate::idgate::uuid_v4(&i.value))
        })
        .count();
    if candidates < 2 {
        return 0;
    }
    let refused = {
        let none = GuidTwinSectionRoles::new();
        if guid_twin_decide(&guid_twin_gate_views(mentions, &none), keep, &[]).groups.is_empty() {
            return 0;
        }
        let roles = guid_twin_section_roles(parsed);
        guid_twin_refusals(&guid_twin_gate_views(mentions, &roles), keep, &[])
    };
    let mut n = 0;
    for m in mentions.iter_mut().filter(|m| refused.contains(&m.section_id)) {
        m.identifier = None;
        n += 1;
    }
    n
}

#[cfg(test)]
mod tests {
    use super::super::{NoticeState, ORG_COUNTRY_FIELD, ORG_IDENTIFIER_FIELD, ORG_NAME_FIELD, ORGANIZATION_KIND};
    use super::*;
    use store::{Mention, NoticeValue, Parsed, Section, ValueRow};

    const GOETTINGEN: &str = "Landkreis Göttingen";
    const ROMMEL: &str = "Fenster- und Fassadenbau Rommel GmbH";
    const CHAMBER: &str = "Vergabekammer Niedersachsen beim Nds. Ministerium für Wirtschaft, Arbeit, Verkehr und Digitalisierung";
    const SBH: &str = "SBH | Schulbau Hamburg";
    /// 255 Vergabekammer des Bundes, as published (dashed lower case) and as stored.
    const BUND_RAW: &str = "0a9ea480-08e4-4ab6-bf12-d722d0ad54b6";
    const BUND: &str = "0A9EA48008E44AB6BF12D722D0AD54B6";

    /// One ORG section of a fixture notice: (section id, name, alpha-3 country or none, raw BT-501).
    type Org<'a> = (&'a str, &'a str, Option<&'a str>, Option<&'a str>);

    /// A parsed eForms notice whose ORG sections publish these names, countries and BT-501 values (as
    /// published: the dashed lower-case UUID shape, A4).
    fn parsed(orgs: &[Org<'_>]) -> Parsed {
        let mut p = Parsed {
            sections: vec![Section { id: "PROC".into(), kind: "Notice".into(), parent: None }],
            values: Vec::new(),
        };
        for (section, name, country, raw) in orgs {
            p.sections.push(Section { id: (*section).into(), kind: ORGANIZATION_KIND.into(), parent: None });
            p.values.push(ValueRow {
                section_id: (*section).into(),
                field_id: ORG_NAME_FIELD.into(),
                ordinal: 0,
                value: NoticeValue::Text { lang: Some("DEU".into()), value: (*name).into() },
            });
            if let Some(country) = country {
                p.values.push(ValueRow {
                    section_id: (*section).into(),
                    field_id: ORG_COUNTRY_FIELD.into(),
                    ordinal: 0,
                    value: NoticeValue::Code { list: None, code: (*country).into() },
                });
            }
            if let Some(raw) = raw {
                p.values.push(ValueRow {
                    section_id: (*section).into(),
                    field_id: ORG_IDENTIFIER_FIELD.into(),
                    ordinal: 0,
                    value: NoticeValue::Id { scheme: None, value: (*raw).into(), is_ref: false },
                });
            }
        }
        p
    }

    /// The mentions production ingest makes of a notice: `mentions_and_dropped`, the gate's input.
    fn mentions(notice_id: i64, orgs: &[Org<'_>]) -> Vec<Mention> {
        NoticeState::mentions_and_dropped(false, notice_id, &parsed(orgs)).0
    }

    fn views(mentions: &[Mention]) -> Vec<TwinView<'_>> {
        mentions
            .iter()
            .map(|m| TwinView {
                section_id: &m.section_id,
                name: &m.name,
                country: m.country.as_deref(),
                raw_identifier: m.raw_identifier.as_deref(),
                scheme: m.scheme.as_deref(),
                roles: &[],
            })
            .collect()
    }

    /// The gate's views of a parsed copy: its mentions, each with the roles the copy's own references
    /// give it ([`guid_twin_section_roles`]).
    fn views_with<'a>(mentions: &'a [Mention], roles: &'a GuidTwinSectionRoles) -> Vec<TwinView<'a>> {
        let mut out = views(mentions);
        for v in &mut out {
            v.roles = roles.get(v.section_id).map_or(&[], Vec::as_slice);
        }
        out
    }

    /// One role reference of a fixture copy: (referring section, field id, target ORG section).
    type Ref<'a> = (&'a str, &'a str, &'a str);

    /// [`parsed`] plus the copy's role references, as published (`is_ref` set, as eForms types them).
    fn parsed_with(orgs: &[Org<'_>], refs: &[Ref<'_>]) -> Parsed {
        let mut p = parsed(orgs);
        for (section, field, target) in refs {
            p.values.push(ValueRow {
                section_id: (*section).into(),
                field_id: (*field).into(),
                ordinal: 0,
                value: NoticeValue::Id { scheme: None, value: (*target).into(), is_ref: true },
            });
        }
        p
    }

    /// Rule 530-T over a parsed copy with its role references, the gate's way: `mentions_and_dropped`,
    /// then [`guid_twin_section_roles`] on the same parse.
    fn decide_with(orgs: &[Org<'_>], refs: &[Ref<'_>], keep: &GuidKeep) -> TwinDecision {
        let p = parsed_with(orgs, refs);
        let m = NoticeState::mentions_and_dropped(false, 9, &p).0;
        let roles = guid_twin_section_roles(&p);
        guid_twin_decide(&views_with(&m, &roles), keep, &[])
    }

    /// Rule 530-T over a parsed notice, through the real mention path.
    fn refused(orgs: &[Org<'_>], keep: &GuidKeep) -> Vec<String> {
        let m = mentions(9, orgs);
        guid_twin_refusals(&views(&m), keep, &[]).into_iter().collect()
    }

    fn decide(orgs: &[Org<'_>], keep: &GuidKeep) -> TwinDecision {
        let m = mentions(9, orgs);
        guid_twin_decide(&views(&m), keep, &[])
    }

    fn none() -> GuidKeep {
        GuidKeep::default()
    }

    /// The Göttingen award (notice 26483825): the buyer under two GUIDs (Procedure-Buyer on ORG-0001,
    /// Lot-AddInfo on ORG-0002), the review chamber under two more, the winner Rommel under a fifth,
    /// DÖE's own ORG-0006 with none.
    fn goettingen() -> Vec<Org<'static>> {
        vec![
            ("ORG-0001", GOETTINGEN, Some("DEU"), Some("037c1508-d945-4e73-9d6e-65099a79bec1")),
            ("ORG-0002", GOETTINGEN, Some("DEU"), Some("5b1c2d3e-4f5a-4b6c-8d7e-8f9a0b1c2d3e")),
            ("ORG-0003", CHAMBER, Some("DEU"), Some("6a1b2c3d-4e5f-4a6b-9c7d-8e9f0a1b2c3d")),
            ("ORG-0004", CHAMBER, Some("DEU"), Some("7b2c3d4e-5f6a-4b7c-ad8e-9f0a1b2c3d4e")),
            ("ORG-0005", ROMMEL, Some("DEU"), Some("6c2d3e4f-5a6b-4c7d-9e8f-9a0b1c2d3e4f")),
            ("ORG-0006", "Deutsches Ausschreibungsblatt GmbH", Some("DEU"), None),
        ]
    }

    /// R0: the Göttingen-shaped twin refuses both of the buyer's values (and both of the chamber's),
    /// never the winner's, named once; the mentions keep their published raw values.
    #[test]
    fn a_goettingen_shaped_twin_refuses_both_values() {
        let d = decide(&goettingen(), &none());
        assert_eq!(
            d.refused.iter().map(String::as_str).collect::<Vec<_>>(),
            vec!["ORG-0001", "ORG-0002", "ORG-0003", "ORG-0004"]
        );
        assert_eq!((d.candidates, d.unnamed), (5, 0), "five v4 sections; ORG-0006 has no BT-501");
        assert_eq!(d.groups.len(), 2, "the buyer and the chamber, each a twin; Rommel is named once");
        let buyer = d.groups.iter().find(|g| g.name_norm == "landkreis göttingen").expect("the buyer's group");
        assert_eq!(buyer.country.as_deref(), Some("DE"), "the register country, alpha-2");
        assert_eq!(buyer.distinct_values, 2);
        assert_eq!(buyer.bind, None);
        assert_eq!(buyer.members[0].1, "037C1508D9454E739D6E65099A79BEC1", "the stored (gated) form of the value");
        let chamber = d.groups.iter().find(|g| g.name_norm != "landkreis göttingen").expect("the chamber's group");
        assert!(!chamber.office_two_words, "a long specific name is no two-word office key");
    }

    /// Build 2: the gate IS wired — production ingest of the Göttingen award refuses the buyer's and the
    /// chamber's twinned values (`identifier = None`, the raw value and scheme kept as published), and
    /// leaves Rommel (named once) and DÖE's ORG-0006 (no BT-501) as they were. (Build 1 pinned the
    /// opposite: every GUID still bound.)
    #[test]
    fn production_ingest_refuses_a_twin_notices_guids() {
        let m = mentions(26_483_825, &goettingen());
        let ids: Vec<(&str, Option<(&str, &str, &str)>, Option<&str>)> = m
            .iter()
            .map(|m| {
                (
                    m.section_id.as_str(),
                    m.identifier.as_ref().map(|i| (i.country.as_deref().unwrap_or(""), i.kind.as_str(), i.value.as_str())),
                    m.raw_identifier.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            ids,
            vec![
                ("ORG-0001", None, Some("037c1508-d945-4e73-9d6e-65099a79bec1")),
                ("ORG-0002", None, Some("5b1c2d3e-4f5a-4b6c-8d7e-8f9a0b1c2d3e")),
                ("ORG-0003", None, Some("6a1b2c3d-4e5f-4a6b-9c7d-8e9f0a1b2c3d")),
                ("ORG-0004", None, Some("7b2c3d4e-5f6a-4b7c-ad8e-9f0a1b2c3d4e")),
                (
                    "ORG-0005",
                    Some(("DE", "national", "6C2D3E4F5A6B4C7D9E8F9A0B1C2D3E4F")),
                    Some("6c2d3e4f-5a6b-4c7d-9e8f-9a0b1c2d3e4f")
                ),
                ("ORG-0006", None, None),
            ],
            "mentions_and_dropped runs rule 530-T: the twinned keys are refused, raw values kept"
        );
        assert_eq!(m[0].country.as_deref(), Some("DE"));
        // The ungated path (the plan row's buyer side) still binds every GUID as published.
        let published = NoticeState::published_mentions(false, 26_483_825, &parsed(&goettingen()));
        assert!(published[..5].iter().all(|m| m.identifier.as_ref().is_some_and(|i| i.kind == "national")));
        // Rule 530-T step 4: the gate touches `identifier` ONLY. `RecordedMention::publishes` compares name,
        // country, raw and scheme, so clearing or rewriting any of them on a refused section would make every
        // recorded twin mention re-resolve on its next refold, outside the manifest ("the deploy causes no
        // churn"). The same copy with an admitted scheme on every BT-501 (`DENIED_SCHEMES` is empty, so each
        // value is still a national v4 candidate and the same four sections are refused).
        let mut p = parsed(&goettingen());
        for v in p.values.iter_mut().filter(|v| v.field_id == ORG_IDENTIFIER_FIELD) {
            if let NoticeValue::Id { scheme, .. } = &mut v.value {
                *scheme = Some("ORG-REG".into());
            }
        }
        let gated = NoticeState::mentions_and_dropped(false, 26_483_825, &p).0;
        let published = NoticeState::published_mentions(false, 26_483_825, &p);
        assert_eq!(published[0].scheme.as_deref(), Some("ORG-REG"), "premise: the scheme is published");
        assert_eq!(gated.len(), published.len());
        for (k, (g, p)) in gated.iter().zip(&published).enumerate() {
            if k < 4 {
                assert_eq!(*g, Mention { identifier: None, ..p.clone() }, "{k}: a refused section keeps every other field");
            } else {
                assert_eq!(g, p, "{k}: an unrefused section is untouched");
            }
        }
    }

    /// R0: the Südbayern three-notice shape (one GUID per notice, no twin) binds — and so would the
    /// three notices read as one (they never are: each copy is decided alone). Both sections of each
    /// copy ARE v4 national candidates: they bind because no name is printed twice, not because the
    /// rule never saw them.
    #[test]
    fn the_suedbayern_shape_without_a_twin_binds() {
        const SUEDBAYERN: &str = "Regierung von Oberbayern - Vergabekammer Südbayern";
        for raw in
            ["1a2b3c4d-5e6f-4071-8293-a4b5c6d7e8f9", "2b3c4d5e-6f7a-4182-9304-b5c6d7e8f9a0", "3c4d5e6f-7a8b-4293-a415-c6d7e8f9a0b1"]
        {
            let orgs = [
                ("ORG-0001", "Staatliches Bauamt Rosenheim", Some("DEU"), Some("4d5e6f7a-8b9c-43a4-b526-d7e8f9a0b1c2")),
                ("ORG-0004", SUEDBAYERN, Some("DEU"), Some(raw)),
            ];
            let d = decide(&orgs, &none());
            assert_eq!((d.candidates, d.groups.len(), d.refused.len()), (2, 0, 0), "{raw}: no name twice in the notice");
        }
    }

    /// R0: a DB-platform GUID recurring across notices binds. The rule decides one copy at a time, so
    /// "recurring" is, per copy, the body named once beside another body (here) or twice under the SAME
    /// value, dashed or not (one distinct value is no twin). The cross-notice property itself — a row
    /// spanning many notices stays unplanned — is the planner test's 3866 / 254 not-planned controls.
    #[test]
    fn a_db_platform_guid_recurring_across_notices_binds() {
        const DB: &str = "DB InfraGO AG – Geschäftsbereich Fahrweg (Bukr 16)";
        const RAW: &str = "fb197f94-7578-4673-8a57-4642ae120532";
        let orgs = [("ORG-0001", DB, Some("DEU"), Some(RAW)), ("ORG-0002", "Vergabekammer des Bundes", Some("DEU"), Some(BUND_RAW))];
        let d = decide(&orgs, &none());
        assert_eq!((d.candidates, d.groups.len(), d.refused.len()), (2, 0, 0), "{d:?}");
        let twice = [("ORG-0001", DB, Some("DEU"), Some(RAW)), ("ORG-0002", DB, Some("DEU"), Some("FB197F94757846738A574642AE120532"))];
        let m = mentions(9, &twice);
        let values: Vec<Option<&str>> = m.iter().map(|m| m.identifier.as_ref().map(|i| i.value.as_str())).collect();
        assert_eq!(values, vec![Some("FB197F94757846738A574642AE120532"); 2], "one stored value, dashed or not");
        let d = guid_twin_decide(&views(&m), &none(), &[]);
        assert_eq!(d.candidates, 2, "both are v4 national candidates");
        assert!(d.refused.is_empty() && d.groups.is_empty(), "one value is no twin: {d:?}");
    }

    /// R0 (A4): an SBH-shaped KEEP pair binds — through `mentions_and_dropped` on a parsed fixture
    /// whose BT-501 values are published dashed lower case, with KEEP holding the stored form.
    #[test]
    fn an_sbh_keep_pair_binds_through_the_real_mention_path() {
        let orgs = [
            ("ORG-0001", SBH, Some("DEU"), Some("3e5c7a9b-1d2f-4a6b-8c0d-e1f2a3b4c5d6")),
            ("ORG-0002", SBH, Some("DEU"), Some("4f6d8b0c-2e3a-4b7c-9d1e-f2a3b4c5d6e7")),
        ];
        let keep: GuidKeep = ["3E5C7A9B1D2F4A6B8C0DE1F2A3B4C5D6", "4F6D8B0C2E3A4B7C9D1EF2A3B4C5D6E7"].into_iter().collect();
        let d = decide(&orgs, &keep);
        assert!(d.refused.is_empty(), "{d:?}");
        assert_eq!(d.groups.len(), 1, "still a twin");
        assert_eq!(d.groups[0].bind, Some(TwinBind::Keep));
        assert_eq!(refused(&orgs, &none()), vec!["ORG-0001", "ORG-0002"], "without KEEP the pair would be refused");
    }

    /// R0: a KEEP value twinned with a fresh one refuses only the fresh one.
    #[test]
    fn a_keep_value_beside_a_fresh_value_refuses_only_the_fresh_one() {
        let orgs = [
            ("ORG-0001", SBH, Some("DEU"), Some("3e5c7a9b-1d2f-4a6b-8c0d-e1f2a3b4c5d6")),
            ("ORG-0002", SBH, Some("DEU"), Some("5a7e9c1d-3f4b-4c8d-ae2f-a3b4c5d6e7f8")),
            ("ORG-0003", SBH, Some("DEU"), Some("6b8f0d2e-4a5c-4d9e-bf3a-b4c5d6e7f8a9")),
        ];
        let keep: GuidKeep = ["3E5C7A9B1D2F4A6B8C0DE1F2A3B4C5D6"].into_iter().collect();
        assert_eq!(refused(&orgs, &keep), vec!["ORG-0002", "ORG-0003"]);
    }

    /// R0 (A4): 255's value published under another country is still kept — KEEP ignores the country.
    #[test]
    fn the_bund_value_under_another_country_is_still_kept() {
        let keep: GuidKeep = [BUND].into_iter().collect();
        let orgs = [
            ("ORG-0001", "Vergabekammer des Bundes", Some("AUT"), Some(BUND_RAW)),
            ("ORG-0002", "Vergabekammer des Bundes", Some("AUT"), Some("8c9d0e1f-2a3b-4c4d-8e5f-6a7b8c9d0e1f")),
        ];
        let d = decide(&orgs, &keep);
        assert_eq!(d.refused.iter().collect::<Vec<_>>(), vec!["ORG-0002"], "only the fresh value");
        assert_eq!(d.groups[0].country.as_deref(), Some("AT"));
    }

    /// R0 (decision §5): a bare `Vergabekammer` twin binds (one content word), and so does a `Zentrale
    /// Vergabestelle` twin (the 530 list).
    #[test]
    fn a_bare_vergabekammer_twin_and_a_zentrale_vergabestelle_twin_bind() {
        for (name, reason) in [
            ("Vergabekammer", TwinBind::ContentWords),
            ("Die Vergabekammer", TwinBind::ContentWords),
            ("Zentrale Vergabestelle", TwinBind::UnsafeList),
            ("Tribunal administratif", TwinBind::UnsafeList),
        ] {
            let orgs = [
                ("ORG-0001", name, Some("DEU"), Some("1f2e3d4c-5b6a-4978-8a9b-0c1d2e3f4a5b")),
                ("ORG-0002", name, Some("DEU"), Some("2a3b4c5d-6e7f-4089-9b0c-1d2e3f4a5b6c")),
            ];
            let d = decide(&orgs, &none());
            assert!(d.refused.is_empty(), "{name}");
            assert_eq!(d.groups[0].bind, Some(reason), "{name}");
        }
    }

    /// R0 (A8): a `Stadtwerke GmbH` twin binds (the legal form is dropped: one content word, and
    /// generic), and so does a `Freistaat Bayern` twin (a polity umbrella).
    #[test]
    fn a_stadtwerke_gmbh_twin_and_a_freistaat_bayern_twin_bind() {
        for (name, reason) in [("Stadtwerke GmbH", TwinBind::ContentWords), ("Freistaat Bayern", TwinBind::UnsafeList)] {
            let orgs = [
                ("ORG-0001", name, Some("DEU"), Some("1f2e3d4c-5b6a-4978-8a9b-0c1d2e3f4a5b")),
                ("ORG-0002", name, Some("DEU"), Some("2a3b4c5d-6e7f-4089-9b0c-1d2e3f4a5b6c")),
            ];
            let d = decide(&orgs, &none());
            assert!(d.refused.is_empty(), "{name}");
            assert_eq!(d.groups[0].bind, Some(reason), "{name}");
        }
    }

    /// R0 (A8 ii): the umbrella shape counts a hyphenated state as one word and skips a leading article:
    /// `Land Nordrhein-Westfalen`, `Kanton Basel-Stadt`, `Canton de Bâle-Ville`, `Der Freistaat Bayern` and
    /// `Das Land Berlin` twins each bind as unsafe; a named body acting for the state still refuses.
    #[test]
    fn hyphenated_and_articled_polity_umbrellas_bind() {
        for name in [
            "Land Nordrhein-Westfalen",
            "Land Nordrhein Westfalen",
            "Kanton Basel-Stadt",
            "Canton de Bâle-Ville",
            "Der Freistaat Bayern",
            "Das Land Berlin",
            "Kanton St. Gallen",
        ] {
            let orgs = [
                ("ORG-0001", name, Some("DEU"), Some("1f2e3d4c-5b6a-4978-8a9b-0c1d2e3f4a5b")),
                ("ORG-0002", name, Some("DEU"), Some("2a3b4c5d-6e7f-4089-9b0c-1d2e3f4a5b6c")),
            ];
            let d = decide(&orgs, &none());
            assert!(d.refused.is_empty(), "{name}");
            assert_eq!(d.groups[0].bind, Some(TwinBind::UnsafeList), "{name}");
        }
        for name in [
            "Freistaat Bayern vertreten durch das Staatliche Bauamt Passau",
            "Land Nordrhein-Westfalen vertreten durch den Bau- und Liegenschaftsbetrieb NRW",
        ] {
            let orgs = [
                ("ORG-0001", name, Some("DEU"), Some("1f2e3d4c-5b6a-4978-8a9b-0c1d2e3f4a5b")),
                ("ORG-0002", name, Some("DEU"), Some("2a3b4c5d-6e7f-4089-9b0c-1d2e3f4a5b6c")),
            ];
            assert_eq!(refused(&orgs, &none()), vec!["ORG-0001", "ORG-0002"], "{name}: a body, refused");
        }
    }

    /// R0 (A11): a regional code is grouped under its register country — a parsed copy publishing
    /// Réunion as `REU` groups under FR, and an REU section and an FRA section of one name fall into ONE
    /// group (the 234 target is keyed the same way).
    #[test]
    fn a_regional_country_groups_under_its_register_country() {
        const SDIS: &str = "SDIS de La Réunion";
        let orgs = [
            ("ORG-0001", SDIS, Some("REU"), Some("1f2e3d4c-5b6a-4978-8a9b-0c1d2e3f4a5b")),
            ("ORG-0002", SDIS, Some("FRA"), Some("2a3b4c5d-6e7f-4089-9b0c-1d2e3f4a5b6c")),
        ];
        let m = mentions(9, &orgs);
        assert_eq!(m[0].country.as_deref(), Some("RE"), "the mention keeps the regional code");
        let d = guid_twin_decide(&views(&m), &none(), &[]);
        assert_eq!(d.groups.len(), 1, "{d:?}");
        assert_eq!(d.groups[0].country.as_deref(), Some("FR"));
        assert_eq!(d.refused.iter().collect::<Vec<_>>(), vec!["ORG-0001", "ORG-0002"]);
    }

    /// R0: a twin with no country binds (the group key needs one), and an unnamed section joins no group.
    #[test]
    fn a_twin_with_no_country_binds_and_an_unnamed_section_joins_no_group() {
        let orgs = [
            ("ORG-0001", GOETTINGEN, None, Some("037c1508-d945-4e73-9d6e-65099a79bec1")),
            ("ORG-0002", GOETTINGEN, None, Some("5b1c2d3e-4f5a-4b6c-8d7e-8f9a0b1c2d3e")),
        ];
        let d = decide(&orgs, &none());
        assert!(d.refused.is_empty());
        assert_eq!(d.groups[0].bind, Some(TwinBind::NoCountry));
        // A hand-built view: an empty published name.
        let views = [
            TwinView { section_id: "ORG-1", name: " ", country: Some("DE"), raw_identifier: Some("037c1508-d945-4e73-9d6e-65099a79bec1"), scheme: None, roles: &[] },
            TwinView { section_id: "ORG-2", name: "", country: Some("DE"), raw_identifier: Some("5b1c2d3e-4f5a-4b6c-8d7e-8f9a0b1c2d3e"), scheme: None, roles: &[] },
        ];
        let d = guid_twin_decide(&views, &none(), &[]);
        assert_eq!((d.candidates, d.unnamed, d.groups.len()), (2, 2, 0));
    }

    /// R0: the TED copy and the DÖE copy are each decided alone — the DÖE copy names the buyer twice
    /// and refuses both values; the TED copy names it once under one of them and binds.
    #[test]
    fn each_copy_is_decided_alone() {
        let doe = goettingen();
        let ted = [
            ("ORG-0001", GOETTINGEN, Some("DEU"), Some("037c1508-d945-4e73-9d6e-65099a79bec1")),
            ("ORG-0005", ROMMEL, Some("DEU"), Some("6c2d3e4f-5a6b-4c7d-9e8f-9a0b1c2d3e4f")),
        ];
        assert_eq!(refused(&doe, &none()), vec!["ORG-0001", "ORG-0002", "ORG-0003", "ORG-0004"]);
        assert!(refused(&ted, &none()).is_empty(), "the TED copy alone holds no twin");
    }

    /// The guard's reading of names: legal forms, initials, umbrellas, two-word office keys, extras.
    #[test]
    fn the_530_name_guard_reads_names_as_decided() {
        let unsafe_ = |name: &str| guid_twin_unsafe_name(name, &[]);
        assert_eq!(unsafe_(GOETTINGEN), None);
        assert_eq!(unsafe_(CHAMBER), None);
        assert_eq!(unsafe_(SBH), None);
        assert_eq!(unsafe_("Muster Bau GmbH & Co. KG"), None, "two content words survive the legal forms");
        assert_eq!(unsafe_("Muster e.V."), Some(TwinBind::ContentWords), "`e.V.` merges to `ev`, a legal form");
        assert_eq!(unsafe_("Stadtwerke"), Some(TwinBind::ContentWords));
        assert_eq!(unsafe_("Landkreis"), Some(TwinBind::ContentWords));
        assert_eq!(unsafe_("Centre hospitalier universitaire"), Some(TwinBind::Generic));
        assert_eq!(unsafe_("Land Berlin"), Some(TwinBind::UnsafeList));
        assert_eq!(unsafe_("Kanton Zürich"), Some(TwinBind::UnsafeList));
        assert_eq!(unsafe_("Canton de Genève"), Some(TwinBind::UnsafeList), "a function word is no content word");
        assert_eq!(unsafe_("Freistaat Bayern vertreten durch das Staatliche Bauamt Passau"), None);
        assert_eq!(unsafe_("Land Nordrhein-Westfalen"), Some(TwinBind::UnsafeList), "a hyphenated state is one word");
        assert_eq!(unsafe_("Kanton Basel-Stadt"), Some(TwinBind::UnsafeList));
        assert_eq!(unsafe_("Canton de Bâle-Ville"), Some(TwinBind::UnsafeList));
        assert_eq!(unsafe_("Der Freistaat Bayern"), Some(TwinBind::UnsafeList), "a leading article is skipped");
        assert_eq!(unsafe_("Das Land Berlin"), Some(TwinBind::UnsafeList));
        assert_eq!(unsafe_("Land Nordrhein-Westfalen vertreten durch den Bau- und Liegenschaftsbetrieb NRW"), None);
        assert_eq!(unsafe_("Landkreis Nordrhein-Westfalen"), None, "only the umbrella heads");
        assert_eq!(unsafe_("Land Berlin Senatsverwaltung"), None, "a second word after the state is a body");
        assert_eq!(unsafe_("Bundesrepublik Deutschland"), Some(TwinBind::UnsafeList));
        assert_eq!(unsafe_("Freie und Hansestadt Hamburg"), Some(TwinBind::UnsafeList));
        assert_eq!(unsafe_("Republik Österreich"), Some(TwinBind::UnsafeList));
        assert_eq!(unsafe_("Natürliche Personen"), Some(TwinBind::UnsafeList));
        assert_eq!(unsafe_("Privatperson"), Some(TwinBind::ContentWords));
        assert_eq!(unsafe_("Tribunal administratif de Paris"), None, "the specific court is a body");
        let extra = vec!["Stadt  Neustadt".to_owned()];
        assert_eq!(guid_twin_unsafe_name("STADT NEUSTADT", &extra), Some(TwinBind::Extra), "folded alike");
        assert_eq!(guid_twin_unsafe_name("Stadt Neustadt", &[]), None, "a homonym only R2 can convict");
        assert!(office_two_words("Vergabekammer Rheinland"));
        assert!(office_two_words("Amt für Hochbau"), "the function word is skipped");
        assert!(!office_two_words(GOETTINGEN));
        assert!(!office_two_words("Vergabekammer Rheinland Köln"));
    }

    /// The run's `extra_unsafe_names` bind a group whose name they fold to; a VAT twin is no candidate.
    #[test]
    fn extra_unsafe_names_bind_and_a_vat_twin_is_no_candidate() {
        let orgs = [
            ("ORG-0001", "Stadt Neustadt", Some("DEU"), Some("1f2e3d4c-5b6a-4978-8a9b-0c1d2e3f4a5b")),
            ("ORG-0002", "Stadt Neustadt", Some("DEU"), Some("2a3b4c5d-6e7f-4089-9b0c-1d2e3f4a5b6c")),
        ];
        let m = mentions(9, &orgs);
        assert_eq!(guid_twin_refusals(&views(&m), &none(), &[]).len(), 2);
        let d = guid_twin_decide(&views(&m), &none(), &["stadt neustadt".to_owned()]);
        assert!(d.refused.is_empty());
        assert_eq!(d.groups[0].bind, Some(TwinBind::Extra));
        let vat = [("ORG-0001", GOETTINGEN, Some("DEU"), Some("DE308703297")), ("ORG-0002", GOETTINGEN, Some("DEU"), Some("DE811569869"))];
        let d = decide(&vat, &none());
        assert_eq!((d.candidates, d.refused.len()), (0, 0), "only v4 national keys are candidates");
    }

    // ---- Build 1b, F1: R2's names (a list, not a pattern) ----

    /// The MV key R2 read R (rand2 item 26): the list matches whole names, so it stays out.
    const MV_KEY: &str = "Geschäftsstelle der Vergabekammern beim Ministerium für Wirtschaft, Infrastruktur, Tourismus und Arbeit \
                          Mecklenburg-Vorpommern";

    /// R2 F1: every spelling R2's W rows print — the BT-500 of the planned mentions and the target keys
    /// of plan 2659's `r2_city_flagged[9]` (3353580) and `[15]` (6622773), as published and as folded —
    /// and the three precautionary names bind as `unsafe_list`; the single-chamber names a `Vergabekammer
    /// <Land>` pattern would have caught stay safe, and so does the MV key.
    #[test]
    fn r2s_umbrella_chamber_names_bind_and_single_chambers_stay_safe() {
        let unsafe_ = |name: &str| guid_twin_unsafe_name(name, &[]);
        for name in [
            // W1 (target 3353580) and W2 (target 6622773), as the rows print them and as keyed.
            "Vergabekammer NRW",
            "vergabekammer nrw",
            "VERGABEKAMMER NRW",
            "Geschäftsstelle der Vergabekammer",
            "geschäftsstelle der vergabekammer",
            "GESCHÄFTSSTELLE DER VERGABEKAMMER",
            // Precautionary.
            "Vergabekammer Nordrhein-Westfalen",
            "Vergabekammer Nordrhein Westfalen",
            "Vergabekammer Bayern",
            "Geschäftsstelle der Vergabekammern",
        ] {
            assert_eq!(unsafe_(name), Some(TwinBind::UnsafeList), "{name}");
        }
        for name in [
            "Vergabekammer Rheinland",
            "Vergabekammer Westfalen",
            "Vergabekammer Niedersachsen",
            CHAMBER,
            "Regierung von Oberbayern - Vergabekammer Südbayern",
            "Vergabekammer Südbayern",
            "Vergabekammer Nordbayern",
            "Vergabekammer BW",
            "Vergabekammer Schleswig-Holstein",
            "Vergabekammer des Landes Brandenburg",
            "Vergabekammer Westfalen bei der Bezirksregierung Münster",
            "Vergabekammer Rheinland c/o Bezirksregierung Köln",
            MV_KEY,
        ] {
            assert_eq!(unsafe_(name), None, "{name}: a single chamber, safe");
        }
        // As twins, through the real mention path: the convicted names bind whole, a single chamber's
        // twin still refuses.
        let twin = |name: &str| -> TwinDecision {
            let orgs = [
                ("ORG-0001", name, Some("DEU"), Some("1f2e3d4c-5b6a-4978-8a9b-0c1d2e3f4a5b")),
                ("ORG-0002", name, Some("DEU"), Some("2a3b4c5d-6e7f-4089-9b0c-1d2e3f4a5b6c")),
            ];
            decide(&orgs, &none())
        };
        for name in ["Vergabekammer NRW", "Geschäftsstelle der Vergabekammer"] {
            let d = twin(name);
            assert!(d.refused.is_empty(), "{name}: {d:?}");
            assert_eq!(d.groups[0].bind, Some(TwinBind::UnsafeList), "{name}");
        }
        for name in ["Vergabekammer Westfalen", "Vergabekammer Niedersachsen", MV_KEY] {
            assert_eq!(twin(name).refused.len(), 2, "{name}: refused");
        }
    }

    /// The office nouns are compared against [`guid_twin_name_words`], so each is stored as it folds
    /// (`geschäftsstelle` → `geschaftsstelle`), and a two-word `Geschäftsstelle X` key is an office key.
    #[test]
    fn every_office_noun_is_stored_folded() {
        for noun in GUID_TWIN_OFFICE_NOUNS {
            assert_eq!(guid_twin_name_words(noun), vec![(*noun).to_owned()], "{noun} must be stored folded");
        }
        assert_eq!(guid_twin_name_words("Geschäftsstelle"), vec!["geschaftsstelle".to_owned()]);
        assert!(office_two_words("Geschäftsstelle Vergabewesen"));
        assert!(office_two_words("GESCHÄFTSSTELLE Vergabewesen"));
        assert!(office_two_words("Geschäftsstelle der Vergabekammer"), "the function word is skipped");
        assert!(!office_two_words("Geschäftsstelle Vergabekammer Südbayern"), "three content words");
        // A safe `Geschäftsstelle X` twin is refused and read whole in R2; the convicted name binds, so
        // its group is no office read.
        let orgs = |name| {
            [
                ("ORG-0001", name, Some("DEU"), Some("1f2e3d4c-5b6a-4978-8a9b-0c1d2e3f4a5b")),
                ("ORG-0002", name, Some("DEU"), Some("2a3b4c5d-6e7f-4089-9b0c-1d2e3f4a5b6c")),
            ]
        };
        let d = decide(&orgs("Geschäftsstelle Vergabewesen"), &none());
        assert_eq!((d.refused.len(), d.groups[0].office_two_words), (2, true));
        let d = decide(&orgs("Geschäftsstelle der Vergabekammer"), &none());
        assert_eq!((d.refused.len(), d.groups[0].office_two_words), (0, false));
    }

    // ---- Build 1b, F2: A7's role guard (the notice's own role references) ----

    /// The AOK Niedersachsen award (TED 24253392, DÖE copy 26483158; R2's W3): the buyer (ORG-0001,
    /// Procedure-Buyer), its information desk (ORG-0002, Lot-AddInfo) and a review slot (ORG-0003,
    /// Lot-ReviewOrg) all print the AOK's name under three v4 values; the winner Bechtle and the eSender
    /// (Procedure-SProvider, no v4 value) are named once. Values as published.
    const AOK: &str = "AOK - Die Gesundheitskasse für Niedersachsen";
    const AOK_V: [&str; 4] = [
        "617a9b4a-867f-4946-9210-0658d0a6ca70",
        "dfe86ded-2374-46d0-abcc-28456aa60795",
        "26a934f6-ecc1-4faf-9c24-50bf06f55811",
        "e215f915-8828-4309-a44f-7d637f09f83d",
    ];

    fn aok() -> Vec<Org<'static>> {
        vec![
            ("ORG-0001", AOK, Some("DEU"), Some(AOK_V[0])),
            ("ORG-0002", AOK, Some("DEU"), Some(AOK_V[1])),
            ("ORG-0003", AOK, Some("DEU"), Some(AOK_V[2])),
            ("ORG-0004", "Bechtle GmbH", Some("DEU"), Some(AOK_V[3])),
            ("ORG-0005", "Deutsches Ausschreibungsblatt GmbH", Some("DEU"), Some("0204:994-DOEVD-83")),
        ]
    }

    /// The TED copy's role references, as published.
    const AOK_REFS: [Ref<'static>; 6] = [
        ("ND-ContractingParty#0", "OPT-300-Procedure-Buyer", "ORG-0001"),
        ("LOT-0001", "OPT-301-Lot-AddInfo", "ORG-0002"),
        ("LOT-0001", "OPT-301-Lot-ReviewOrg", "ORG-0003"),
        ("PROCEDURE", "OPT-301-Lot-ReviewOrg", "ORG-0003"),
        ("ND-Tenderer#0", "OPT-300-Tenderer", "ORG-0004"),
        ("ND-ServiceProviderParty#0", "OPT-300-Procedure-SProvider", "ORG-0005"),
    ];

    /// R0 (R2 F2): the AOK shape binds whole as `role_mix` through the gate's path (the parsed copy's
    /// mentions, its own references read by [`guid_twin_section_roles`]), its roles named; without the
    /// review slot it is the Göttingen shape (Buyer + AddInfo + TenderReceipt) and refuses. The fixture
    /// is R2's R0 shape (Buyer and AddInfo on two values, ReviewOrg on a third, as the published AOK
    /// copy); the bare Procedure-Buyer + Lot-ReviewOrg pair on two values is pinned in
    /// `the_role_guard_reads_the_groups_own_references_after_their_scope`.
    #[test]
    fn an_aok_shaped_buyer_and_review_org_twin_binds_as_role_mix() {
        let d = decide_with(&aok(), &AOK_REFS, &none());
        assert!(d.refused.is_empty(), "{d:?}");
        assert_eq!(d.groups.len(), 1, "the AOK twin; Bechtle is named once");
        let g = &d.groups[0];
        assert_eq!(g.bind, Some(TwinBind::RoleMix));
        assert_eq!(g.roles, vec!["Lot-AddInfo", "Lot-ReviewOrg", "Procedure-Buyer"]);
        assert_eq!(g.distinct_values, 3);
        // Without any reference (as Build 1 read it), the same copy refuses all three.
        assert_eq!(decide(&aok(), &none()).refused.len(), 3);
        // Buyer + AddInfo + TenderReceipt, no review slot: coherent, refused.
        let receipt = [
            ("ND-ContractingParty#0", "OPT-300-Procedure-Buyer", "ORG-0001"),
            ("LOT-0001", "OPT-301-Lot-AddInfo", "ORG-0002"),
            ("LOT-0001", "OPT-301-Lot-TenderReceipt", "ORG-0003"),
        ];
        let d = decide_with(&aok(), &receipt, &none());
        assert_eq!(d.refused.iter().map(String::as_str).collect::<Vec<_>>(), vec!["ORG-0001", "ORG-0002", "ORG-0003"]);
        assert_eq!(d.groups[0].roles, vec!["Lot-AddInfo", "Lot-TenderReceipt", "Procedure-Buyer"]);
        // KEEP over every value does not hide the role mix: the role guard reads first.
        let keep: GuidKeep = AOK_V[..3].iter().collect();
        assert_eq!(decide_with(&aok(), &AOK_REFS, &keep).groups[0].bind, Some(TwinBind::RoleMix));
    }

    /// A7's role set, read after the scope: a `Part-ReviewOrg` beside a Procedure-SProvider binds; a
    /// review body beside review information (a chamber in both slots) refuses; a review slot on ANOTHER
    /// name does not touch the buyer's group; a tenderer beside an information desk binds
    /// (`tenderer_mixed`); and a twin with no reference at all refuses (an sdk-0.1 copy, a legacy copy).
    #[test]
    fn the_role_guard_reads_the_groups_own_references_after_their_scope() {
        let two = |name: &'static str| -> Vec<Org<'static>> {
            vec![("ORG-0001", name, Some("DEU"), Some(AOK_V[0])), ("ORG-0002", name, Some("DEU"), Some(AOK_V[1]))]
        };
        let bind = |orgs: &[Org<'_>], refs: &[Ref<'_>]| decide_with(orgs, refs, &none()).groups[0].bind;
        let part = [("ND-ServiceProviderParty#0", "OPT-300-Procedure-SProvider", "ORG-0001"), ("LOT-0001", "OPT-301-Part-ReviewOrg", "ORG-0002")];
        assert_eq!(bind(&two(AOK), &part), Some(TwinBind::RoleMix), "a Part- scope is a Lot- scope");
        let buyer_review = [("ND-ContractingParty#0", "OPT-300-Procedure-Buyer", "ORG-0001"), ("LOT-0001", "OPT-301-Lot-ReviewOrg", "ORG-0002")];
        assert_eq!(bind(&two(AOK), &buyer_review), Some(TwinBind::RoleMix), "buyer + review slot on two values, no AddInfo: the brief's AOK pair");
        let chamber = [("LOT-0001", "OPT-301-Lot-ReviewInfo", "ORG-0001"), ("LOT-0001", "OPT-301-Lot-ReviewOrg", "ORG-0002")];
        assert_eq!(bind(&two(CHAMBER), &chamber), None, "a chamber in both review slots is one body");
        let tenderer = [("ND-Tenderer#0", "OPT-300-Tenderer", "ORG-0001"), ("LOT-0001", "OPT-301-Lot-AddInfo", "ORG-0002")];
        assert_eq!(bind(&two(ROMMEL), &tenderer), Some(TwinBind::RoleMix), "a bidder under an information desk's name");
        let subcont = [("ND-Tenderer#0", "OPT-300-Tenderer", "ORG-0001"), ("ND-SubContractor#0", "OPT-301-Tenderer-SubCont", "ORG-0002")];
        assert_eq!(bind(&two(ROMMEL), &subcont), None, "one class: coherent");
        assert_eq!(bind(&two(AOK), &[]), None, "no reference, no role: refused");
        // The review slot names the VK Bund section, not an AOK one: the AOK buyer twin refuses.
        let mut orgs = two(AOK);
        orgs.push(("ORG-0003", "Vergabekammer des Bundes", Some("DEU"), Some(BUND_RAW)));
        let refs = [
            ("ND-ContractingParty#0", "OPT-300-Procedure-Buyer", "ORG-0001"),
            ("LOT-0001", "OPT-301-Lot-AddInfo", "ORG-0002"),
            ("LOT-0001", "OPT-301-Lot-ReviewOrg", "ORG-0003"),
        ];
        let d = decide_with(&orgs, &refs, &none());
        assert_eq!(d.refused.iter().map(String::as_str).collect::<Vec<_>>(), vec!["ORG-0001", "ORG-0002"]);
    }

    /// The role mapping is the projection's: an OPT-300/OPT-301 id-ref names its suffix; a DE-1.x alias
    /// source names its eForms role whatever its stored flag (as `normalise_de1_parse` marks it); an
    /// unflagged eForms id, an `ojs` chain edge, an OPT-302 reference, a legacy `TED-` address block, an
    /// identifier and a DE-1.x non-role alias name none. A raw DE-1.x copy and its normalised parse read
    /// one map.
    #[test]
    fn the_role_mapping_reads_references_as_the_projection_does() {
        let r = |field: &str, scheme: Option<&str>, is_ref: bool| guid_twin_role_ref(field, scheme, is_ref);
        assert_eq!(r("OPT-300-Procedure-Buyer", None, true).as_deref(), Some("Procedure-Buyer"));
        assert_eq!(r("OPT-301-Part-ReviewOrg", Some("organization"), true).as_deref(), Some("Part-ReviewOrg"));
        assert_eq!(r("OPT-301-Lot-ReviewOrg", None, false), None, "an eForms id typed as no reference");
        assert_eq!(r("OPT-301-Lot-ReviewOrg", Some("ojs"), true), None, "a chain edge");
        assert_eq!(r("OPT-302-Organization", None, true), None);
        assert_eq!(r("TED-ADDRESS_CONTRACTOR", None, true), None, "legacy address roles are not read");
        assert_eq!(r("BT-501-Organization-Company", None, false), None);
        assert_eq!(
            r("DE1-TenderingTerms-AppealTerms-AppealReceiverParty-PartyIdentification-ID", None, false).as_deref(),
            Some("Lot-ReviewOrg"),
            "a DÖE copy parsed before issue 98's inventory fix"
        );
        assert_eq!(r("DE1-ContractingParty-Party-PartyIdentification-ID", None, true).as_deref(), Some("Procedure-Buyer"));
        assert_eq!(r("DE1-Organizations-Organization-Company-PartyLegalEntity-CompanyID", None, false), None);
        assert_eq!(r("DE1-Organizations-Organization-Company-PartyIdentification-ID", None, false), None);

        // A raw DÖE parse (DE-1.x ids, one stored unflagged) and its normalised form: one map.
        let mut raw = parsed(&aok()[..4]);
        for (section, field, target, is_ref) in [
            ("ND-ContractingParty#0", "DE1-ContractingParty-Party-PartyIdentification-ID", "ORG-0001", false),
            ("LOT-0001", "DE1-ProcurementProjectLot-TenderingTerms-AdditionalInformationParty-PartyIdentification-ID", "ORG-0002", true),
            ("PROCEDURE", "DE1-TenderingTerms-AppealTerms-AppealReceiverParty-PartyIdentification-ID", "ORG-0003", true),
            ("ND-Tenderer#0", "DE1-NoticeResult-TenderingParty-Tenderer-ID", "ORG-0004", true),
            ("ORG-0001", "DE1-Organizations-Organization-Company-PartyIdentification-ID", "ORG-0001", false),
        ] {
            raw.values.push(ValueRow {
                section_id: section.into(),
                field_id: field.into(),
                ordinal: 0,
                value: NoticeValue::Id { scheme: None, value: target.into(), is_ref },
            });
        }
        let mut normalised = raw.clone();
        super::super::normalise_de1_parse(&mut normalised);
        let want: GuidTwinSectionRoles = [
            ("ORG-0001", "Procedure-Buyer"),
            ("ORG-0002", "Lot-AddInfo"),
            ("ORG-0003", "Lot-ReviewOrg"),
            ("ORG-0004", "Tenderer"),
        ]
        .into_iter()
        .map(|(s, r)| (s.to_owned(), vec![r.to_owned()]))
        .collect();
        assert_eq!(guid_twin_section_roles(&raw), want);
        assert_eq!(guid_twin_section_roles(&normalised), want, "the gate's normalised parse reads the same");
        // And the gate decides the DÖE copy as it decides the TED copy.
        let m = NoticeState::mentions_and_dropped(false, 26_483_158, &normalised).0;
        let roles = guid_twin_section_roles(&normalised);
        let d = guid_twin_decide(&views_with(&m, &roles), &none(), &[]);
        assert_eq!((d.refused.len(), d.groups[0].bind), (0, Some(TwinBind::RoleMix)));
    }

    // ---- The planner on a fixture DB, and its parity with the rule (and the gate's input path) ----

    use store::turso::Value;

    async fn exec(conn: &store::turso::Connection, sql: &str, params: Vec<Value>) {
        conn.execute(sql, params).await.unwrap();
    }

    fn txt(s: &str) -> Value {
        Value::Text(s.into())
    }

    async fn org_in(conn: &store::turso::Connection, id: i64, country: &str, identifier: Option<&str>, name: &str) {
        exec(
            conn,
            "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, 0)",
            vec![
                Value::Integer(id),
                txt(country),
                identifier.map_or(Value::Null, |_| txt("national")),
                identifier.map_or(Value::Null, |i| txt(&GuidKeep::key(i))),
                txt(name),
                txt(&store::org_name_norm(name)),
                Value::Integer(i64::from(identifier.is_none())),
            ],
        )
        .await;
    }

    async fn org(conn: &store::turso::Connection, id: i64, identifier: Option<&str>, name: &str) {
        org_in(conn, id, "DE", identifier, name).await;
    }

    async fn notice(conn: &store::turso::Connection, id: i64, source: &str, tender: i64, seq: i64) {
        exec(
            conn,
            "INSERT INTO notices (id, source, publication_id, content_hash, profile, fetch_id, member_path, ingested_at,
                                  published_at, parse_state)
             VALUES (?, ?, ?, 'h', 'eforms:eforms-sdk-1.10', 1, 'm', 0, 1717200000, 'parsed')",
            vec![Value::Integer(id), txt(source), txt(&format!("{id}-2024"))],
        )
        .await;
        exec(
            conn,
            "INSERT INTO tender_versions (tender_id, seq, caused_by_notice_id, published_at, publication_id)
             VALUES (?, ?, ?, 1717200000, ?)",
            vec![Value::Integer(tender), Value::Integer(seq), Value::Integer(id), txt(&format!("{id}-2024"))],
        )
        .await;
    }

    /// A recorded mention bound to `org`, published under `country`, with its party role on the notice's
    /// version (`bid:` a bid party; empty: no party row) and city.
    #[allow(clippy::too_many_arguments)]
    async fn mention_in(
        conn: &store::turso::Connection,
        notice: i64,
        section: &str,
        org: i64,
        name: &str,
        country: &str,
        raw: Option<&str>,
        role: &str,
        version: (i64, i64),
        city: Option<&str>,
    ) {
        exec(
            conn,
            "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, country, raw_identifier, scheme)
             VALUES (?, ?, ?, ?, ?, ?, NULL)",
            vec![Value::Integer(notice), txt(section), Value::Integer(org), txt(name), txt(country), raw.map_or(Value::Null, txt)],
        )
        .await;
        if !role.is_empty() {
            let sql = if role.starts_with("bid:") {
                "INSERT INTO tender_version_bid_parties (tender_id, seq, bid_id, role, organization_id, mention_notice_id,
                                                         mention_section_id) VALUES (?, ?, 1, ?, ?, ?, ?)"
            } else {
                "INSERT INTO tender_version_parties (tender_id, seq, lot_id, role, organization_id, mention_notice_id,
                                                     mention_section_id) VALUES (?, ?, NULL, ?, ?, ?, ?)"
            };
            exec(
                conn,
                sql,
                vec![
                    Value::Integer(version.0),
                    Value::Integer(version.1),
                    txt(role.trim_start_matches("bid:")),
                    Value::Integer(org),
                    Value::Integer(notice),
                    txt(section),
                ],
            )
            .await;
        }
        if let Some(city) = city {
            exec(
                conn,
                "INSERT INTO notice_texts (notice_id, section_id, field_id, ordinal, lang, value) VALUES (?, ?, ?, 0, 'DEU', ?)",
                vec![Value::Integer(notice), txt(section), txt(store::GUID_TWIN_CITY_FIELD), txt(city)],
            )
            .await;
        }
    }

    /// [`mention_in`] under DE.
    #[allow(clippy::too_many_arguments)]
    async fn mention(
        conn: &store::turso::Connection,
        notice: i64,
        section: &str,
        org: i64,
        name: &str,
        raw: Option<&str>,
        role: &str,
        version: (i64, i64),
        city: Option<&str>,
    ) {
        mention_in(conn, notice, section, org, name, "DE", raw, role, version, city).await;
    }

    /// A lot-result winner row of a Tender version.
    async fn winner(conn: &store::turso::Connection, version: (i64, i64), org: i64) {
        exec(
            conn,
            "INSERT INTO tender_version_result_winners (tender_id, seq, lot_result_id, organization_id) VALUES (?, ?, 1, ?)",
            vec![Value::Integer(version.0), Value::Integer(version.1), Value::Integer(org)],
        )
        .await;
    }

    fn role_class(role: &str) -> &'static str {
        match role {
            "Procedure-Buyer" => store::GUID_ROLE_BUYER,
            "Tenderer" => store::GUID_ROLE_TENDERER,
            r if r.starts_with("Lot-") => store::GUID_ROLE_REVIEW_INFO,
            _ => store::GUID_ROLE_OTHER,
        }
    }

    /// A fresh fixture DB with the planner's indexes, and a raw connection with foreign keys off.
    async fn fixture(name: &str) -> (store::Db, store::turso::Connection, String) {
        let path = format!("/tmp/tender-db-ingest-guid-twin-{name}-{}.db", std::process::id());
        cleanup(&path);
        let db = store::Db::open(&path).await.unwrap();
        db.build_organization_indexes().await.unwrap();
        db.build_tender_indexes().await.unwrap();
        let raw = store::turso::Builder::new_local(&path).build().await.unwrap();
        let conn = raw.connect().unwrap();
        conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
        (db, conn, path)
    }

    fn cleanup(path: &str) {
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{suffix}"));
        }
    }

    /// The dry plan with the real `uuid_v4` and rule 530-T, the decision's stable ids and knob, and the
    /// given R1 controls, over the walk's in-memory KEEP (Build 1's; [`run_plan_with`] passes a KEEP).
    async fn run_plan(db: &store::Db, not_planned: &[i64], planned: &[(i64, i64)]) -> store::GuidTwinPlan {
        run_plan_with(db, None, not_planned, planned).await
    }

    /// [`run_plan`] with an explicit KEEP (Build 2: `Some(keep_530())`, the compiled file the job reads).
    async fn run_plan_with(
        db: &store::Db,
        keep: Option<&GuidKeep>,
        not_planned: &[i64],
        planned: &[(i64, i64)],
    ) -> store::GuidTwinPlan {
        let stop = || false;
        let progress = |_: u64, _: &str| {};
        let plan = db
            .plan_guid_twins(store::GuidTwinPlanArgs {
                is_guid: crate::idgate::uuid_v4,
                rule: guid_twin_decide,
                keep,
                keep_version: if keep.is_some() { GUID_KEEP_530_VERSION } else { "walk" },
                unsafe_names: GUID_TWIN_UNSAFE_NAMES,
                under_review: None,
                rehomed: None,
                role_ref: guid_twin_role_ref,
                role_class,
                stable_ids: &[254, 255, 8_378],
                stable_mentions: store::GUID_TWIN_R1_STABLE_MENTIONS,
                extra_unsafe_names: &[],
                samples: 30,
                window: 5_000_000,
                batch: 4,
                not_planned,
                planned,
                expected: &[(23_014_307, None)],
                exhibits: &[("DE", "landkreis göttingen")],
                stop: &stop,
                progress: &progress,
            })
            .await
            .unwrap();
        assert!(!plan.stopped);
        plan
    }

    /// PARITY: the planner's refused `(notice, section)` set — its planned mentions and its off-GUID
    /// refusals — is exactly what [`guid_twin_refusals`] gives over the stored mentions, notice by notice,
    /// with the plan's own KEEP and each copy's roles read the GATE's way: [`guid_twin_section_roles`]
    /// over the copy as the projection loads it (`parsed_by_ids`), normalised as the projection
    /// normalises it (`normalise_de1`). Returns the planner's set.
    async fn assert_parity(db: &store::Db, conn: &store::turso::Connection, plan: &store::GuidTwinPlan) -> BTreeSet<(i64, String)> {
        let keep: GuidKeep = plan.keep_candidates.iter().map(|c| c.value.clone()).collect();
        assert_parity_with(db, conn, plan, &keep).await
    }

    /// [`assert_parity`] with the KEEP the plan read (Build 2: the compiled one, passed by hand).
    async fn assert_parity_with(
        db: &store::Db,
        conn: &store::turso::Connection,
        plan: &store::GuidTwinPlan,
        keep: &GuidKeep,
    ) -> BTreeSet<(i64, String)> {
        assert!(!plan.planned_mention_keys_truncated);
        let mut planned: BTreeSet<(i64, String)> =
            plan.planned_mention_keys.iter().map(|k| (k.notice_id, k.section_id.clone())).collect();
        planned.extend(plan.off_guid.samples.iter().map(|o| (o.notice_id, o.section_id.clone())));
        let mut rows = conn
            .query("SELECT notice_id, section_id, name, country, raw_identifier, scheme FROM organization_mentions ORDER BY 1, 2", ())
            .await
            .unwrap();
        type Stored = (String, String, Option<String>, Option<String>, Option<String>);
        let mut stored: BTreeMap<i64, Vec<Stored>> = BTreeMap::new();
        while let Some(r) = rows.next().await.unwrap() {
            let s = |i: usize| r.get_value(i).ok().and_then(|v| v.as_text().cloned());
            let n = match r.get_value(0).unwrap() {
                Value::Integer(n) => n,
                _ => unreachable!(),
            };
            stored.entry(n).or_default().push((s(1).unwrap(), s(2).unwrap_or_default(), s(3), s(4), s(5)));
        }
        drop(rows);
        let mut copies = db.parsed_by_ids(&stored.keys().copied().collect::<Vec<_>>()).await.unwrap();
        super::super::normalise_de1(&mut copies);
        let roles: BTreeMap<i64, GuidTwinSectionRoles> =
            copies.iter().map(|(n, parsed)| (n.id, guid_twin_section_roles(parsed))).collect();
        let none = GuidTwinSectionRoles::new();
        let mut by_rule: BTreeSet<(i64, String)> = BTreeSet::new();
        for (n, ms) in &stored {
            let section_roles = roles.get(n).unwrap_or(&none);
            let views: Vec<TwinView<'_>> = ms
                .iter()
                .map(|(sec, name, country, raw, scheme)| TwinView {
                    section_id: sec,
                    name,
                    country: country.as_deref(),
                    raw_identifier: raw.as_deref(),
                    scheme: scheme.as_deref(),
                    roles: section_roles.get(sec).map_or(&[], Vec::as_slice),
                })
                .collect();
            by_rule.extend(guid_twin_refusals(&views, keep, &[]).into_iter().map(|s| (*n, s)));
        }
        assert_eq!(planned, by_rule, "the planner plans exactly the rule's refusals");
        planned
    }

    // The fixture's GUIDs, as published (dashed lower case).
    const A: &str = "037c1508-d945-4e73-9d6e-65099a79bec1"; // Göttingen buyer, 22759261
    const B: &str = "5b1c2d3e-4f5a-4b6c-8d7e-8f9a0b1c2d3e"; // Göttingen Lot-AddInfo, 22759262
    const C: &str = "6c2d3e4f-5a6b-4c7d-9e8f-9a0b1c2d3e4f"; // Rommel, 22759265
    const D: &str = "6a1b2c3d-4e5f-4a6b-9c7d-8e9f0a1b2c3d"; // chamber ReviewInfo, 23014306 (partial)
    const E: &str = "7b2c3d4e-5f6a-4b7c-ad8e-9f0a1b2c3d4e"; // chamber ReviewOrg, 23014307
    const F: &str = "8c3d4e5f-6a7b-4c8d-be9f-0a1b2c3d4e5f"; // Klinikum buyer, 30000101
    const G: &str = "9d4e5f6a-7b8c-4d9e-8f0a-1b2c3d4e5f6a"; // Klinikum ReviewOrg, 30000102
    const H: &str = "ae5f6a7b-8c9d-4e0f-9a1b-2c3d4e5f6a7b"; // Muster Bau bid tenderer, 30000103
    const I: &str = "bf6a7b8c-9d0e-4f1a-ab2c-3d4e5f6a7b8c"; // Muster Bau Lot-AddInfo, 30000104
    const J: &str = "c07b8c9d-0e1f-4a2b-bc3d-4e5f6a7b8c9d"; // bare Vergabekammer, 30000105
    const K: &str = "d18c9d0e-1f2a-4b3c-8d4e-5f6a7b8c9d0e"; // bare Vergabekammer, 30000106
    const L: &str = "3e5c7a9b-1d2f-4a6b-8c0d-e1f2a3b4c5d6"; // SBH 3866 (two Tenders: KEEP)
    const M: &str = "4f6d8b0c-2e3a-4b7c-9d1e-f2a3b4c5d6e7"; // SBH 3868 (two Tenders: KEEP)
    const N: &str = "5a7e9c1d-3f4b-4c8d-ae2f-a3b4c5d6e7f8"; // SBH fresh, 30000110
    const P: &str = "e29d0e1f-2a3b-4c4d-9e5f-6a7b8c9d0e1f"; // Göttingen, 22800001 (partner off GUID)
    const Q: &str = "f3ae1f2a-3b4c-4d5e-af6a-7b8c9d0e1f2a"; // Göttingen, bound to 2294259 itself
    const R: &str = "a1b2c3d4-e5f6-4a7b-8c9d-0e1f2a3b4c5d"; // DB InfraGO fresh, 30000111 (254's partner)
    const S: &str = "a3b4c5d6-e7f8-4a9b-8c0d-1e2f3a4b5c6d"; // Göttingen, 23014305 (named once: kept)
    const DB: &str = "fb197f94-7578-4673-8a57-4642ae120532"; // 254, stable

    /// Every shape the plan reports, one notice copy each (tender, seq in brackets):
    /// - 24262502 (TED) and 26483825 (DÖE), tender 948285: the Göttingen award. Both copies name the
    ///   buyer twice (A → 22759261, B → 22759262) and Rommel once (C). The DÖE copy also names the
    ///   chamber twice (D → 23014306, E → 23014307), the TED copy once (D): 23014306 is PARTIAL (its DÖE
    ///   mention refused, its TED one kept), 23014307 whole, and the chamber has no identifier-NULL row
    ///   (a missing target).
    /// - 2294259: Göttingen's DE identifier-NULL row, mentions in Göttingen and Hannover (a multi-city
    ///   target, flag b); one planned copy publishes Duderstadt (flag a; with the others' Göttingen, flag c).
    ///   VAT row 22183653 of the same name is never a target.
    /// - 40000001 [990001]: Klinikum Musterstadt as Procedure-Buyer (F) and Lot-ReviewOrg (G), and Muster
    ///   Bau AG as a bid tenderer (H) and Lot-AddInfo (I): two PARTY-role-incoherent refused groups. The
    ///   fixture stores no `notice_ids` references, so A7's role guard (which reads the copy's own
    ///   references, never `tender_version_parties`) sees no roles here and the groups stay refused;
    ///   only the party-role composition (step 4d) lists them as incoherent. The reference-driven
    ///   `role_mix` bind is the AOK test's. Their
    ///   identifier-NULL rows: 30000201 (51 mentions; only the OLDEST is in Andernstadt, the newest 50 in
    ///   Musterstadt: one city, not flagged) and 30000202 (two mentions in two cities: flag b alone).
    /// - 40000002 [990002]: a bare `Vergabekammer` twin (J, K): bound by the guard.
    /// - 40000003/4 [990003/990004]: SBH 3866 (L) and 3868 (M), each in two Tenders (KEEP); 40000005
    ///   [990005] twins them (bound: keep); 40000006 [990006] twins 3866 with a fresh N (only N refused).
    /// - 40000007 [990007]: Göttingen under P (22800001) and Q bound to 2294259 (an off-GUID refusal,
    ///   already on its target): 22800001 is planned, but its partner is off GUID rows.
    /// - 40000008 [990008]: 254 (DB, stable), named once: a not-planned control. 40000009 [990009]
    ///   twins it with a fresh R (30000111): 254 is kept (KEEP: stable, and now two Tenders), R planned —
    ///   so 254 is a census twin before the plan and not after (A9).
    /// - 40000010 [990010]: a Göttingen GUID row named once (S → 23014305): the exhibit's kept row.
    async fn seed(conn: &store::turso::Connection) {
        org(conn, 254, Some(DB), "DB InfraGO AG").await;
        org(conn, 2_294_259, None, GOETTINGEN).await;
        org(conn, 22_183_653, None, GOETTINGEN).await; // re-keyed below to its VAT
        exec(conn, "UPDATE organizations SET identifier_kind = 'vat', identifier = 'DE308703297' WHERE id = 22183653", vec![]).await;
        org(conn, 22_759_261, Some(A), GOETTINGEN).await;
        org(conn, 22_759_262, Some(B), GOETTINGEN).await;
        org(conn, 22_759_265, Some(C), ROMMEL).await;
        org(conn, 23_014_305, Some(S), GOETTINGEN).await;
        org(conn, 23_014_306, Some(D), CHAMBER).await;
        org(conn, 23_014_307, Some(E), CHAMBER).await;
        org(conn, 30_000_101, Some(F), "Klinikum Musterstadt").await;
        org(conn, 30_000_102, Some(G), "Klinikum Musterstadt").await;
        org(conn, 30_000_103, Some(H), "Muster Bau AG").await;
        org(conn, 30_000_104, Some(I), "Muster Bau AG").await;
        org(conn, 30_000_105, Some(J), "Vergabekammer").await;
        org(conn, 30_000_106, Some(K), "Vergabekammer").await;
        org(conn, 3_866, Some(L), SBH).await;
        org(conn, 3_868, Some(M), SBH).await;
        org(conn, 30_000_110, Some(N), SBH).await;
        org(conn, 30_000_111, Some(R), "DB InfraGO AG").await;
        org(conn, 22_800_001, Some(P), GOETTINGEN).await;
        org(conn, 30_000_201, None, "Klinikum Musterstadt").await;
        org(conn, 30_000_202, None, "Muster Bau AG").await;

        // The target's own mentions: two cities.
        notice(conn, 20_000_001, "ted", 900_001, 1).await;
        mention(conn, 20_000_001, "ORG-0001", 2_294_259, GOETTINGEN, None, "Procedure-Buyer", (900_001, 1), Some("Göttingen")).await;
        notice(conn, 20_000_002, "ted", 900_002, 1).await;
        mention(conn, 20_000_002, "ORG-0001", 2_294_259, GOETTINGEN, None, "Procedure-Buyer", (900_002, 1), Some("Hannover")).await;
        // Klinikum's identifier-NULL row: 51 mentions, the oldest (lowest rowid) the only one elsewhere.
        for k in 0..51i64 {
            let n = 50_000_001 + k;
            notice(conn, n, "ted", 950_001 + k, 1).await;
            let city = if k == 0 { "Andernstadt" } else { "Musterstadt" };
            mention(conn, n, "ORG-0001", 30_000_201, "Klinikum Musterstadt", None, "Procedure-Buyer", (950_001 + k, 1), Some(city))
                .await;
        }
        // Muster Bau's identifier-NULL row: two mentions, two cities.
        for (n, city) in [(50_000_101i64, "Bremen"), (50_000_102, "Hamburg")] {
            notice(conn, n, "ted", n - 49_000_000, 1).await;
            mention(conn, n, "ORG-0001", 30_000_202, "Muster Bau AG", None, "Tenderer", (n - 49_000_000, 1), Some(city)).await;
        }

        // The Göttingen award, both copies.
        for (n, source, seq) in [(24_262_502i64, "ted", 1), (26_483_825, "doe", 2)] {
            notice(conn, n, source, 948_285, seq).await;
            let v = (948_285, seq);
            let city = if source == "doe" { "Duderstadt" } else { "Göttingen" };
            mention(conn, n, "ORG-0001", 22_759_261, GOETTINGEN, Some(A), "Procedure-Buyer", v, Some(city)).await;
            mention(conn, n, "ORG-0002", 22_759_262, GOETTINGEN, Some(B), "Lot-AddInfo", v, Some("Göttingen")).await;
            mention(conn, n, "ORG-0003", 23_014_306, CHAMBER, Some(D), "Lot-ReviewInfo", v, Some("Hannover")).await;
            if source == "doe" {
                mention(conn, n, "ORG-0004", 23_014_307, CHAMBER, Some(E), "Lot-ReviewOrg", v, Some("Hannover")).await;
            }
            mention(conn, n, "ORG-0005", 22_759_265, ROMMEL, Some(C), "Tenderer", v, None).await;
        }
        // Role-incoherent twins.
        notice(conn, 40_000_001, "doe", 990_001, 1).await;
        let v = (990_001, 1);
        mention(conn, 40_000_001, "ORG-0001", 30_000_101, "Klinikum Musterstadt", Some(F), "Procedure-Buyer", v, None).await;
        mention(conn, 40_000_001, "ORG-0002", 30_000_102, "Klinikum Musterstadt", Some(G), "Lot-ReviewOrg", v, None).await;
        mention(conn, 40_000_001, "ORG-0003", 30_000_103, "Muster Bau AG", Some(H), "bid:tenderer", v, None).await;
        mention(conn, 40_000_001, "ORG-0004", 30_000_104, "Muster Bau AG", Some(I), "Lot-AddInfo", v, None).await;
        // A guard-bound twin.
        notice(conn, 40_000_002, "doe", 990_002, 1).await;
        mention(conn, 40_000_002, "ORG-0001", 30_000_105, "Vergabekammer", Some(J), "Lot-ReviewOrg", (990_002, 1), None).await;
        mention(conn, 40_000_002, "ORG-0002", 30_000_106, "Vergabekammer", Some(K), "Lot-ReviewInfo", (990_002, 1), None).await;
        // SBH: two KEEP rows (each two Tenders), a KEEP twin, and a KEEP value beside a fresh one.
        for (n, tender) in [(40_000_003i64, 990_003i64), (40_000_004, 990_004)] {
            notice(conn, n, "ted", tender, 1).await;
            mention(conn, n, "ORG-0001", 3_866, SBH, Some(L), "Procedure-Buyer", (tender, 1), None).await;
            mention(conn, n, "ORG-0002", 3_868, SBH, Some(M), "Procedure-Buyer", (tender, 1), None).await;
        }
        notice(conn, 40_000_005, "ted", 990_005, 1).await;
        mention(conn, 40_000_005, "ORG-0001", 3_866, SBH, Some(L), "Procedure-Buyer", (990_005, 1), None).await;
        mention(conn, 40_000_005, "ORG-0002", 3_868, "SBH | Schulbau Hamburg ", Some(M), "Lot-AddInfo", (990_005, 1), None).await;
        notice(conn, 40_000_006, "ted", 990_006, 1).await;
        mention(conn, 40_000_006, "ORG-0001", 3_866, SBH, Some(L), "Procedure-Buyer", (990_006, 1), None).await;
        mention(conn, 40_000_006, "ORG-0002", 30_000_110, SBH, Some(N), "Lot-AddInfo", (990_006, 1), None).await;
        // A partner bound off GUID rows (to the target itself).
        notice(conn, 40_000_007, "doe", 990_007, 1).await;
        mention(conn, 40_000_007, "ORG-0001", 22_800_001, GOETTINGEN, Some(P), "Procedure-Buyer", (990_007, 1), None).await;
        mention(conn, 40_000_007, "ORG-0002", 2_294_259, GOETTINGEN, Some(Q), "Lot-AddInfo", (990_007, 1), None).await;
        // 254, named once; then twinned with a fresh value in another Tender (A9).
        notice(conn, 40_000_008, "ted", 990_008, 1).await;
        mention(conn, 40_000_008, "ORG-0001", 254, "DB InfraGO AG", Some(DB), "Procedure-Buyer", (990_008, 1), None).await;
        notice(conn, 40_000_009, "ted", 990_009, 1).await;
        mention(conn, 40_000_009, "ORG-0001", 254, "DB InfraGO AG", Some(DB), "Procedure-Buyer", (990_009, 1), None).await;
        mention(conn, 40_000_009, "ORG-0002", 30_000_111, "DB InfraGO AG", Some(R), "Lot-AddInfo", (990_009, 1), None).await;
        // A Göttingen GUID row named once: kept.
        notice(conn, 40_000_010, "doe", 990_010, 1).await;
        mention(conn, 40_000_010, "ORG-0001", 23_014_305, GOETTINGEN, Some(S), "Procedure-Buyer", (990_010, 1), None).await;
    }

    /// Issue 530 unit 3: the dry planner on a fixture of every shape — whole, partial, target, missing
    /// target, the city flags, the role mix, guard and KEEP exclusions, off-GUID refusals, the post-state,
    /// the bars and controls (passing, then driven to FAIL) — and its PARITY with the rule: per notice
    /// copy, the refused sections the planner plans are exactly what [`guid_twin_refusals`] gives, both
    /// over the stored mentions (the planner's input) and over `mentions_and_dropped` of the parsed
    /// copies (the gate's input, A2's post-state check).
    #[tokio::test]
    async fn the_dry_planner_plans_per_mention_and_agrees_with_the_rule() {
        let (db, conn, path) = fixture("main").await;
        seed(&conn).await;
        let plan = run_plan(&db, &[254, 3_866, 3_868, 22_759_265, 23_014_305], &[(22_759_261, 2_294_259), (22_759_262, 2_294_259)]).await;

        // KEEP: 254 (stable by id, and two Tenders), 3866 and 3868 (two Tenders each).
        let keep_values: Vec<(&str, &str)> = plan.keep_candidates.iter().map(|c| (c.value.as_str(), c.reason.as_str())).collect();
        assert_eq!(
            keep_values,
            vec![
                ("3E5C7A9B1D2F4A6B8C0DE1F2A3B4C5D6", "multi_tender"),
                ("4F6D8B0C2E3A4B7C9D1EF2A3B4C5D6E7", "multi_tender"),
                ("FB197F94757846738A574642AE120532", "stable+multi_tender"),
            ]
        );
        assert!(plan.bars.keep_self_check_missing.is_empty());

        // The plan: whole rows, the one partial row, and the mentions.
        assert_eq!(
            plan.planned_row_ids,
            vec![
                22_759_261, 22_759_262, 22_800_001, 23_014_307, 30_000_101, 30_000_102, 30_000_103, 30_000_104, 30_000_110,
                30_000_111
            ]
        );
        assert_eq!(plan.partial_row_ids, vec![23_014_306], "its DÖE mention refused, its TED one kept");
        assert_eq!((plan.planned.rows, plan.planned.mentions), (10, 12));
        assert_eq!((plan.planned_partial.rows, plan.planned_partial.mentions), (1, 1));
        assert_eq!(plan.partial.by_sources.get("refused doe / kept ted"), Some(&1));
        assert_eq!(plan.refused_mentions, 14, "13 on GUID rows, and Q, bound to the target itself");
        assert_eq!(plan.off_guid.by_kind.iter().map(|(k, n)| (*k, *n)).collect::<Vec<_>>(), vec![("identifier_null", 1)]);
        assert_eq!(plan.off_guid.already_on_target, 1);
        // R1's census bar counts whole AND partial rows: the partial chamber row 23014306 (its DÖE partner
        // 23014307 is a GUID row) is comparable; 22800001's partner Q is off GUID rows.
        assert_eq!(plan.bars.planned_rows_off_guid_partner, 1, "22800001");
        assert_eq!(plan.bars.planned_rows_comparable, 10, "9 whole + the partial 23014306");
        assert_eq!(plan.bars.planned_rows_comparable_whole, 9);
        assert_eq!(plan.bars.planned_rows_above_census, 0);
        assert_eq!(plan.keep_partner_mentions, 2, "3866 beside the fresh N, 254 beside the fresh R");
        // An empty KEEP would refuse the SBH pair in 40000003/4/5 (6 mentions), 3866 in 40000006 and 254
        // in 40000009.
        assert_eq!((plan.keep_spared.rows, plan.keep_spared.mentions), (3, 8));

        // The guard and KEEP exclusions.
        assert_eq!(plan.guard["content_words"].groups, 1, "the bare Vergabekammer twin");
        assert_eq!(plan.guard["content_words"].mentions, 2);
        assert_eq!(plan.guard["keep"].groups, 3, "the SBH pair, in 40000003, 40000004 and 40000005");
        assert_eq!(plan.guard["no_country"].groups, 0);
        assert_eq!(
            (plan.guard["role_mix"].groups, plan.bars.tripwire_role_mix, plan.role_ref_rows_read),
            (0, 0, 0),
            "no notice_ids references in this seed: the guard reads references only; the party-role report still \
             lists Klinikum/Muster Bau"
        );
        assert!(plan.role_ref_notices > 0, "the role read ran for the twin copies and found nothing");
        assert_eq!(plan.bars.role_ref_disagree, 0, "the real rule's role union is the planner's read");

        // Targets: Göttingen's identifier-NULL row (never the VAT row), Klinikum's and Muster Bau's; the
        // chamber, SBH and DB InfraGO have none.
        let gott = plan.controls.iter().find(|c| c.org_id == 22_759_261).unwrap();
        assert!(gott.ok, "{gott:?}");
        let rows_of = |id: i64| plan.planned_samples.iter().chain(&plan.partial.samples).find(|r| r.org_id == id).cloned().unwrap();
        assert_eq!(rows_of(22_759_262).targets, vec![Some(2_294_259)]);
        assert_eq!(rows_of(30_000_101).targets, vec![Some(30_000_201)]);
        assert_eq!(rows_of(30_000_103).targets, vec![Some(30_000_202)]);
        assert_eq!(rows_of(23_014_307).targets, vec![None], "the chamber has no identifier-NULL row");
        assert_eq!(rows_of(30_000_110).targets, vec![None]);
        assert_eq!(rows_of(30_000_111).targets, vec![None]);
        // Missing: the chamber (2 rows, 2 mentions), SBH's fresh N (1, 1), DB InfraGO's fresh R (1, 1).
        assert_eq!((plan.missing_targets.rows, plan.missing_targets.mentions), (4, 4));
        assert_eq!((plan.targets, plan.targets_found), (6, 3));
        assert_eq!((plan.bars.target_country_mismatch, plan.bars.target_country_unchecked), (0, 0));
        let largest = plan.largest_target.clone().unwrap();
        assert_eq!((largest.target_id, largest.planned_mentions, largest.mentions), (Some(2_294_259), 5, 3));

        // The city screen. 2294259 holds Göttingen and Hannover (flag b); the DÖE copy's Duderstadt is
        // absent from that set (flag a), and with the other copies' Göttingen its planned mentions show two
        // cities (flag c). Klinikum's 30000201 reads its newest 50 mentions only: Musterstadt alone, so the
        // oldest's Andernstadt flags nothing. Muster Bau's 30000202 has two cities (flag b alone).
        assert_eq!(plan.city.multi_city_targets, 2, "2294259 and 30000202");
        assert_eq!(plan.city.targets_with_cities, 3);
        assert_eq!((plan.city.city_absent_mentions, plan.city.city_absent_rows, plan.city.city_absent_targets), (1, 1, 1));
        assert_eq!((plan.city.planned_mentions_with_city, plan.city.planned_mentions_without_city), (6, 7));
        assert_eq!(
            (plan.city.planned_multi_city_keys, plan.city.planned_multi_city_mentions, plan.city.planned_multi_city_rows),
            (1, 5, 3)
        );
        assert_eq!(plan.city.flagged_count, 2);
        let flagged: Vec<Option<i64>> = plan.city.flagged.iter().map(|t| t.target_id).collect();
        assert_eq!(flagged, vec![Some(2_294_259), Some(30_000_202)]);
        assert_eq!(plan.city.flagged[0].cities, vec!["gottingen", "hannover"]);
        assert_eq!(plan.city.flagged[0].planned_cities, vec!["duderstadt", "gottingen"]);
        assert!(plan.city.flagged[0].multi_city_planned);
        let muster = &plan.city.flagged[1];
        assert!(muster.multi_city && muster.city_absent == 0 && !muster.multi_city_planned, "{muster:?}");
        let klinikum = plan.r2_top_targets.iter().find(|t| t.target_id == Some(30_000_201)).unwrap();
        assert_eq!((klinikum.mentions, klinikum.cities.clone(), klinikum.multi_city), (51, vec!["musterstadt".to_owned()], false));

        // Role composition by PARTY roles (`tender_version_parties`, bid parties, winners), not by the
        // notice references the guard reads: buyer + Lot-ReviewOrg (Klinikum), a bid tenderer + Lot-AddInfo
        // (Muster Bau) — refused here because the seed stores no references.
        assert_eq!(plan.roles.buyer_with_review_org, 1);
        assert_eq!(plan.roles.tenderer_mixed, 1);
        assert_eq!(plan.roles.incoherent_count, 2);
        assert_eq!(plan.roles.refused_groups, 8);
        assert_eq!(
            plan.roles.by_role_set.get("Lot-AddInfo+Procedure-Buyer"),
            Some(&5),
            "Göttingen ×3, SBH's and DB InfraGO's fresh twins"
        );
        assert_eq!(plan.roles.by_role_set.get("Procedure-Buyer+Lot-ReviewOrg"), None, "role sets are sorted");
        assert_eq!(plan.roles.by_role_set.get("Lot-ReviewOrg+Procedure-Buyer"), Some(&1));

        // A9: the census twin now and after the plan. Before: Göttingen 2, the chamber 2, Klinikum 2, Muster
        // Bau 2, the bare chamber 2, SBH 3, DB InfraGO 2 (254 and R). After: the guard-bound bare chamber and
        // the SBH KEEP pair; 254 — stable, two Tenders — stops being a twin once its only partner is planned.
        assert_eq!((plan.post_state.twin_rows, plan.post_state.twin_rows_after), (15, 4));
        assert_eq!((plan.post_state.twin_stable_rows, plan.post_state.twin_stable_rows_after), (1, 0));
        assert_eq!((plan.post_state.twin_multi_tender_rows, plan.post_state.twin_multi_tender_rows_after), (3, 2));
        let db_row = plan.controls.iter().find(|c| c.org_id == 254).and_then(|c| c.row.clone()).unwrap();
        assert!(db_row.twin && !db_row.twin_after && db_row.stable && db_row.planned == "no", "{db_row:?}");

        // The bars and controls: 254, 3866, 3868, Rommel and 23014305 not planned; Göttingen planned to
        // 2294259.
        assert!(plan.bars.not_planned_failed.is_empty(), "{:?}", plan.bars.not_planned_failed);
        assert!(plan.bars.planned_failed.is_empty(), "{:?}", plan.bars.planned_failed);
        assert_eq!(plan.bars.tripwire_multi_tender + plan.bars.tripwire_stable + plan.bars.tripwire_keep, 0);
        assert_eq!(plan.bars.tripwire_no_tender, 0);
        assert!(plan.bars.stable_mentions_is_decision);
        assert!(plan.bars.pass, "{:#?}", plan.bars);
        let chamber = plan.controls.iter().find(|c| c.org_id == 23_014_307).unwrap();
        assert!(!chamber.ok && chamber.expect == "expected", "planned whole, but its target is missing: reported, not a bar");
        let exhibit = &plan.exhibits["DE landkreis göttingen"];
        assert_eq!((exhibit.guid_rows, exhibit.planned, exhibit.partial, exhibit.kept), (4, 3, 0, 1));
        assert_eq!(exhibit.kept_ids, vec![23_014_305]);
        assert!(exhibit.partial_ids.is_empty(), "no partial Göttingen row in this fixture");
        assert_eq!((plan.guid_rows, plan.distinct_values), (18, 18));
        assert_eq!(plan.candidate_notices, 12);
        assert_eq!(plan.planned_mention_keys.len(), 13);

        // The breakdowns of planned (whole) rows: by country, first year (every notice is 2024) and role.
        let cells = |m: &BTreeMap<String, store::GuidTwinCount>| -> Vec<(String, u64, u64)> {
            m.iter().map(|(k, c)| (k.clone(), c.rows, c.mentions)).collect()
        };
        assert_eq!(cells(&plan.planned_by_country), vec![("DE".to_owned(), 10, 12)]);
        assert_eq!(cells(&plan.planned_by_first_year), vec![("2024".to_owned(), 10, 12)]);
        assert_eq!(
            cells(&plan.planned_by_role),
            vec![("buyer".to_owned(), 3, 4), ("review_or_info".to_owned(), 6, 7), ("tenderer_winner".to_owned(), 1, 1)],
            "buyer: 22759261 (2), 22800001, 30000101; review_or_info: 22759262 (2), 23014307, 30000102, 30000104, \
             30000110, 30000111; tenderer: the bid party 30000103"
        );

        // R2. The random strata: every planned row (10 whole, 1 partial) fits, buyer block first, each
        // block in salted-key order. The top targets hold every planned key, the largest first.
        assert_eq!(plan.r2_random.len(), 11);
        let roles: Vec<&str> = plan.r2_random.iter().map(|r| r.role).collect();
        let mut expect = vec![store::GUID_ROLE_BUYER; 3];
        expect.extend([store::GUID_ROLE_REVIEW_INFO; 7]);
        expect.push(store::GUID_ROLE_TENDERER);
        assert_eq!(roles, expect);
        let key = |id: i64| store::guid_twin_sample_key(id, store::GUID_TWIN_SALT);
        let ids: Vec<i64> = plan.r2_random.iter().map(|r| r.org_id).collect();
        for block in [&ids[..3], &ids[3..10]] {
            assert!(block.windows(2).all(|w| key(w[0]) < key(w[1])), "salted order: {block:?}");
        }
        assert_eq!(plan.r2_top_targets.len(), 6);
        assert_eq!(plan.r2_top_targets[0].target_id, Some(2_294_259), "the largest target leads");
        assert!(plan.r2_top_targets.iter().any(|t| t.name_norm == largest.name_norm && t.country == largest.country));
        assert_eq!(plan.r2_top_targets.iter().filter(|t| t.target_id.is_none()).count(), 3, "the missing keys too");

        // PARITY over the stored mentions, then the gate's input path: the award's two copies, parsed and
        // folded by mentions_and_dropped.
        let planned = assert_parity(&db, &conn, &plan).await;
        let keep: GuidKeep = plan.keep_candidates.iter().map(|c| c.value.clone()).collect();
        for (n, source) in [(24_262_502i64, "ted"), (26_483_825, "doe")] {
            let mut orgs: Vec<Org<'_>> = vec![
                ("ORG-0001", GOETTINGEN, Some("DEU"), Some(A)),
                ("ORG-0002", GOETTINGEN, Some("DEU"), Some(B)),
                ("ORG-0003", CHAMBER, Some("DEU"), Some(D)),
            ];
            if source == "doe" {
                orgs.push(("ORG-0004", CHAMBER, Some("DEU"), Some(E)));
            }
            orgs.push(("ORG-0005", ROMMEL, Some("DEU"), Some(C)));
            let m = mentions(n, &orgs);
            let gate: BTreeSet<(i64, String)> = guid_twin_refusals(&views(&m), &keep, &[]).into_iter().map(|s| (n, s)).collect();
            let plan_n: BTreeSet<(i64, String)> = planned.iter().filter(|(pn, _)| *pn == n).cloned().collect();
            assert_eq!(gate, plan_n, "{source} copy: the stock plan's refusals equal a fresh projection under the gate");
        }

        // R1 FAILS when a control misses: a not-planned control planned whole (30000101) or partial
        // (23014306), a planned control onto the wrong target (22759261 onto the VAT row), a planned control
        // that is only partial (23014306), and one that is no GUID row at all.
        let failing = run_plan(
            &db,
            &[30_000_101, 23_014_306],
            &[(22_759_261, 22_183_653), (23_014_306, 2_294_259), (999_999_999, 2_294_259)],
        )
        .await;
        assert_eq!(failing.bars.not_planned_failed, vec![30_000_101, 23_014_306]);
        assert_eq!(failing.bars.planned_failed, vec![22_759_261, 23_014_306, 999_999_999]);
        assert!(!failing.bars.pass);
        drop(conn);
        cleanup(&path);
    }

    /// Issue 530 unit 3 (R2, A6): keys with NO identifier-NULL row are read like found targets — a
    /// missing key mints one row that takes every planned mention of its key, across notices. Two
    /// office-noun two-word keys, neither with a row: both are in the office-key read, both in the top
    /// targets, named from their published name and ranked by planned mentions; `Amt 66`, refused in a
    /// Neustadt notice and an Altstadt notice, is flagged by its planned mentions' two cities (flag c).
    #[tokio::test]
    async fn the_r2_lists_read_keys_with_no_identifier_null_row() {
        let (db, conn, path) = fixture("missing").await;
        const W: [&str; 6] = [
            "10a1b2c3-d4e5-4f60-8a1b-2c3d4e5f6071",
            "20a1b2c3-d4e5-4f60-9a1b-2c3d4e5f6072",
            "30a1b2c3-d4e5-4f60-aa1b-2c3d4e5f6073",
            "40a1b2c3-d4e5-4f60-ba1b-2c3d4e5f6074",
            "50a1b2c3-d4e5-4f60-8a1b-2c3d4e5f6075",
            "60a1b2c3-d4e5-4f60-9a1b-2c3d4e5f6076",
        ];
        for (k, name) in ["Amt 66", "Amt 66", "Amt 66", "Amt 66", "Referat Beschaffung", "Referat Beschaffung"].iter().enumerate() {
            org(&conn, 101 + k as i64, Some(W[k]), name).await;
        }
        for (n, city, base) in [(1i64, Some("Neustadt"), 0usize), (2, Some("Altstadt"), 2), (3, None, 4)] {
            notice(&conn, n, "doe", 10 + n, 1).await;
            let name = if base < 4 { "Amt 66" } else { "Referat Beschaffung" };
            let org_id = 101 + base as i64;
            mention(&conn, n, "ORG-0001", org_id, name, Some(W[base]), "Procedure-Buyer", (10 + n, 1), city).await;
            mention(&conn, n, "ORG-0002", org_id + 1, name, Some(W[base + 1]), "Lot-AddInfo", (10 + n, 1), city).await;
        }
        let plan = run_plan(&db, &[], &[]).await;
        assert_eq!(plan.planned_row_ids, vec![101, 102, 103, 104, 105, 106]);
        assert_eq!((plan.targets, plan.targets_found), (2, 0));
        assert_eq!((plan.missing_targets.rows, plan.missing_targets.mentions), (6, 6));
        let missing: Vec<(&str, Option<i64>, i64, u64)> = plan
            .missing_target_samples
            .iter()
            .map(|t| (t.name.as_str(), t.target_id, t.sample_id, t.planned_mentions))
            .collect();
        assert_eq!(missing, vec![("Amt 66", None, 101, 4), ("Referat Beschaffung", None, 105, 2)], "ranked, named");
        let norms = |list: &[store::GuidTwinTargetSample]| list.iter().map(|t| t.name_norm.clone()).collect::<Vec<_>>();
        assert_eq!(plan.r2_office_two_words_count, 2);
        assert_eq!(norms(&plan.r2_office_two_words), vec!["amt 66", "referat beschaffung"]);
        assert_eq!(norms(&plan.r2_top_targets), vec!["amt 66", "referat beschaffung"], "both missing keys");
        assert_eq!(norms(&plan.r2_city_flagged), vec!["amt 66"]);
        let amt = &plan.r2_city_flagged[0];
        assert_eq!((amt.planned_cities.clone(), amt.multi_city_planned, amt.city_count), (vec!["altstadt".to_owned(), "neustadt".to_owned()], true, 0));
        assert_eq!(
            (plan.city.planned_multi_city_keys, plan.city.planned_multi_city_mentions, plan.city.planned_multi_city_rows),
            (1, 4, 4)
        );
        assert_eq!((plan.city.flagged_count, plan.city.multi_city_targets, plan.city.city_absent_mentions), (1, 0, 0));
        let largest = plan.largest_target.clone().unwrap();
        assert_eq!((largest.target_id, largest.sample_id, largest.name.as_str(), largest.planned_mentions), (None, 101, "Amt 66", 4));
        assert!(plan.r2_top_targets.iter().any(|t| t.name_norm == largest.name_norm && t.country == largest.country));
        assert_parity(&db, &conn, &plan).await;
        drop(conn);
        cleanup(&path);
    }

    /// Issue 530 unit 3 (A11): a twin published under a regional code (RE) is grouped under its register
    /// country (FR) and lands on the FR identifier-NULL row — never the RE row of the same name, though
    /// that one has the lower id — and A11's bar, read from the stored mention country, passes.
    #[tokio::test]
    async fn a_regional_twin_lands_on_its_register_countrys_row() {
        let (db, conn, path) = fixture("regional").await;
        const SDIS: &str = "SDIS de La Réunion";
        org_in(&conn, 60, "RE", None, SDIS).await;
        org_in(&conn, 61, "FR", None, SDIS).await;
        org_in(&conn, 62, "FR", Some("1f2e3d4c-5b6a-4978-8a9b-0c1d2e3f4a5b"), SDIS).await;
        org_in(&conn, 63, "FR", Some("2a3b4c5d-6e7f-4089-9b0c-1d2e3f4a5b6c"), SDIS).await;
        notice(&conn, 70, "ted", 80, 1).await;
        let a = Some("1f2e3d4c-5b6a-4978-8a9b-0c1d2e3f4a5b");
        let b = Some("2a3b4c5d-6e7f-4089-9b0c-1d2e3f4a5b6c");
        mention_in(&conn, 70, "ORG-0001", 62, SDIS, "RE", a, "Procedure-Buyer", (80, 1), None).await;
        mention_in(&conn, 70, "ORG-0002", 63, SDIS, "RE", b, "Lot-AddInfo", (80, 1), None).await;
        let plan = run_plan(&db, &[], &[]).await;
        assert_eq!(plan.planned_row_ids, vec![62, 63]);
        for s in &plan.planned_samples {
            assert_eq!(s.targets, vec![Some(61)], "the FR row, not the RE row 60: {s:?}");
        }
        assert_eq!((plan.targets, plan.targets_found), (1, 1));
        assert_eq!((plan.missing_targets.rows, plan.missing_targets.mentions), (0, 0));
        assert_eq!((plan.bars.target_country_mismatch, plan.bars.target_country_unchecked), (0, 0));
        assert_eq!(plan.largest_target.as_ref().and_then(|t| t.country.clone()).as_deref(), Some("FR"));
        assert_parity(&db, &conn, &plan).await;
        drop(conn);
        cleanup(&path);
    }

    /// Issue 530 unit 3 (A7): result winners are read by the notice's own version. A winner section
    /// beside a Lot-AddInfo one is a role-incoherent group (`result-winner` in its roles); a winner beside
    /// a bid tenderer is one class (coherent: "not a winner" read by class); and a winner row of another
    /// seq, or of an org outside the group, tags no section.
    #[tokio::test]
    async fn role_composition_reads_result_winners_by_version() {
        let (db, conn, path) = fixture("winners").await;
        const Y: [&str; 6] = [
            "1a1b2c3d-4e5f-4a6b-8c7d-8e9f0a1b2c31",
            "2a1b2c3d-4e5f-4a6b-9c7d-8e9f0a1b2c32",
            "3a1b2c3d-4e5f-4a6b-ac7d-8e9f0a1b2c33",
            "4a1b2c3d-4e5f-4a6b-bc7d-8e9f0a1b2c34",
            "5a1b2c3d-4e5f-4a6b-8c7d-8e9f0a1b2c35",
            "6a1b2c3d-4e5f-4a6b-9c7d-8e9f0a1b2c36",
        ];
        const WEISS: &str = "Weiss Elektro GmbH";
        const GRUEN: &str = "Grün Bau GmbH";
        const BLAU: &str = "Blau Technik GmbH";
        for (id, value, name) in [(301, Y[0], WEISS), (302, Y[1], WEISS), (401, Y[2], GRUEN), (402, Y[3], GRUEN), (501, Y[4], BLAU), (502, Y[5], BLAU)] {
            org(&conn, id, Some(value), name).await;
        }
        // A winner beside a Lot-AddInfo section: incoherent. The decoy winner rows (another seq of the
        // Tender, another org) must not tag ORG-0002.
        notice(&conn, 101, "doe", 201, 1).await;
        mention(&conn, 101, "ORG-0001", 301, WEISS, Some(Y[0]), "", (201, 1), None).await;
        winner(&conn, (201, 1), 301).await;
        mention(&conn, 101, "ORG-0002", 302, WEISS, Some(Y[1]), "Lot-AddInfo", (201, 1), None).await;
        winner(&conn, (201, 2), 302).await;
        winner(&conn, (201, 1), 999).await;
        // A winner beside a bid tenderer: one class, coherent.
        notice(&conn, 102, "doe", 202, 1).await;
        mention(&conn, 102, "ORG-0001", 401, GRUEN, Some(Y[2]), "bid:tenderer", (202, 1), None).await;
        mention(&conn, 102, "ORG-0002", 402, GRUEN, Some(Y[3]), "", (202, 1), None).await;
        winner(&conn, (202, 1), 402).await;
        // No member is this version's winner: a winner row on another seq, and one of an outside org.
        notice(&conn, 103, "doe", 203, 1).await;
        mention(&conn, 103, "ORG-0001", 501, BLAU, Some(Y[4]), "Procedure-Buyer", (203, 1), None).await;
        mention(&conn, 103, "ORG-0002", 502, BLAU, Some(Y[5]), "Lot-AddInfo", (203, 1), None).await;
        winner(&conn, (203, 2), 501).await;
        winner(&conn, (203, 1), 999).await;
        let plan = run_plan(&db, &[], &[]).await;
        assert_eq!(plan.roles.refused_groups, 3);
        let sets: Vec<(&str, u64)> = plan.roles.by_role_set.iter().map(|(k, n)| (k.as_str(), *n)).collect();
        assert_eq!(
            sets,
            vec![("Lot-AddInfo+Procedure-Buyer", 1), ("Lot-AddInfo+result-winner", 1), ("bid-tenderer+result-winner", 1)]
        );
        assert_eq!((plan.roles.tenderer_mixed, plan.roles.buyer_with_review_org, plan.roles.incoherent_count), (1, 0, 1));
        assert_eq!(plan.roles.incoherent[0].notice_id, 101);
        assert_eq!(plan.roles.incoherent[0].roles, vec!["Lot-AddInfo", "result-winner"]);
        assert_parity(&db, &conn, &plan).await;
        drop(conn);
        cleanup(&path);
    }

    /// A stored id value of a fixture copy (`notice_ids`).
    #[allow(clippy::too_many_arguments)]
    async fn id_row(
        conn: &store::turso::Connection,
        notice: i64,
        section: &str,
        field: &str,
        ordinal: i64,
        scheme: Option<&str>,
        value: &str,
        is_ref: bool,
    ) {
        exec(
            conn,
            "INSERT INTO notice_ids (notice_id, section_id, field_id, ordinal, scheme, value, is_ref) VALUES (?, ?, ?, ?, ?, ?, ?)",
            vec![
                Value::Integer(notice),
                txt(section),
                txt(field),
                Value::Integer(ordinal),
                scheme.map_or(Value::Null, txt),
                txt(value),
                Value::Integer(i64::from(is_ref)),
            ],
        )
        .await;
    }

    /// Build 1b (R2 F2, W3): the AOK Niedersachsen award in both copies — TED 24253392 with OPT-300/301
    /// references, its DÖE copy 26483158 with DE-1.x references (the buyer's stored unflagged, as a copy
    /// parsed before issue 98's inventory fix) — rows 22752175/76/77 under one name as Procedure-Buyer,
    /// Lot-AddInfo and Lot-ReviewOrg, beside a Göttingen-shaped coherent twin (Buyer + AddInfo +
    /// TenderReceipt). The planner reads the role references of the three twin copies, binds both AOK
    /// copies as `role_mix` (so 22752175-77 are NOT planned: R1's controls pass) and still plans the
    /// coherent twin; its role self-check stays 0 and the party-based incoherent list no longer holds
    /// the AOK. PARITY: the planner's stored-role read equals the gate's [`guid_twin_section_roles`] over
    /// each copy as the projection loads it, raw and normalised, and its refusals equal the rule's (the
    /// gate's decision on the parsed TED copy binds the same group as `role_mix`).
    #[tokio::test]
    async fn the_planner_binds_the_aok_role_mix_from_the_notices_own_references() {
        let (db, conn, path) = fixture("aok").await;
        const TED: i64 = 24_253_392;
        const DOE: i64 = 26_483_158;
        const MUSTER: i64 = 24_253_393;
        const MUSTER_NAME: &str = "Stadt Musterhausen";
        const MV: [&str; 2] = ["5d1e2f3a-4b5c-4d6e-8f7a-8b9c0d1e2f3a", "6e2f3a4b-5c6d-4e7f-9a8b-9c0d1e2f3a4b"];
        org(&conn, 7_861, None, AOK).await;
        for (k, value) in AOK_V[..3].iter().enumerate() {
            org(&conn, 22_752_175 + k as i64, Some(value), AOK).await;
        }
        org(&conn, 22_752_178, Some(AOK_V[3]), "Bechtle GmbH").await;
        org(&conn, 30_100_001, Some(MV[0]), MUSTER_NAME).await;
        org(&conn, 30_100_002, Some(MV[1]), MUSTER_NAME).await;
        for (n, source, seq) in [(TED, "ted", 1), (DOE, "doe", 2)] {
            notice(&conn, n, source, 990_100, seq).await;
            let v = (990_100, seq);
            for (k, (section, role)) in [("ORG-0001", "Procedure-Buyer"), ("ORG-0002", "Lot-AddInfo"), ("ORG-0003", "Lot-ReviewOrg")]
                .into_iter()
                .enumerate()
            {
                mention(&conn, n, section, 22_752_175 + k as i64, AOK, Some(AOK_V[k]), role, v, Some("Hannover")).await;
            }
            mention(&conn, n, "ORG-0004", 22_752_178, "Bechtle GmbH", Some(AOK_V[3]), "", v, None).await;
        }
        exec(&conn, "UPDATE notices SET profile = 'eforms:eforms-de-1.1' WHERE id = 26483158", vec![]).await;
        // The TED copy's references, an identifier beside them (no role).
        for (section, field, target) in AOK_REFS {
            let scheme = field.contains("ReviewOrg").then_some("organization");
            id_row(&conn, TED, section, field, 0, scheme, target, true).await;
        }
        id_row(&conn, TED, "ORG-0001", "BT-501-Organization-Company", 0, None, AOK_V[0], false).await;
        // The DÖE copy's references: DE-1.x ids, the buyer's stored unflagged; its own ORG ids (no role).
        for (section, field, target, is_ref) in [
            ("ND-ContractingParty#0", "DE1-ContractingParty-Party-PartyIdentification-ID", "ORG-0001", false),
            ("LOT-0001", "DE1-ProcurementProjectLot-TenderingTerms-AdditionalInformationParty-PartyIdentification-ID", "ORG-0002", true),
            ("LOT-0001", "DE1-TenderingTerms-AppealTerms-AppealReceiverParty-PartyIdentification-ID", "ORG-0003", true),
            ("PROCEDURE", "DE1-TenderingTerms-AppealTerms-AppealReceiverParty-PartyIdentification-ID", "ORG-0003", true),
            ("ND-Tenderer#0", "DE1-NoticeResult-TenderingParty-Tenderer-ID", "ORG-0004", true),
            ("ORG-0001", "DE1-Organizations-Organization-Company-PartyIdentification-ID", "ORG-0001", false),
        ] {
            id_row(&conn, DOE, section, field, 0, None, target, is_ref).await;
        }
        // The coherent twin: Buyer, and AddInfo + TenderReceipt on the second section.
        notice(&conn, MUSTER, "ted", 990_101, 1).await;
        mention(&conn, MUSTER, "ORG-0001", 30_100_001, MUSTER_NAME, Some(MV[0]), "Procedure-Buyer", (990_101, 1), None).await;
        mention(&conn, MUSTER, "ORG-0002", 30_100_002, MUSTER_NAME, Some(MV[1]), "Lot-AddInfo", (990_101, 1), None).await;
        for (section, field, target) in [
            ("ND-ContractingParty#0", "OPT-300-Procedure-Buyer", "ORG-0001"),
            ("LOT-0001", "OPT-301-Lot-AddInfo", "ORG-0002"),
            ("LOT-0001", "OPT-301-Lot-TenderReceipt", "ORG-0002"),
        ] {
            id_row(&conn, MUSTER, section, field, 0, None, target, true).await;
        }

        let plan = run_plan(&db, &[22_752_175, 22_752_176, 22_752_177], &[]).await;
        assert!(plan.bars.not_planned_failed.is_empty(), "{:?}", plan.bars.not_planned_failed);
        assert_eq!(plan.planned_row_ids, vec![30_100_001, 30_100_002], "the coherent twin alone");
        assert!(plan.bars.pass, "{:#?}", plan.bars);
        assert_eq!(plan.bars.tripwire_role_mix, 0);
        assert_eq!(plan.bars.role_ref_disagree, 0, "the real rule's roles are the planner's own read");
        let role_mix = plan.guard["role_mix"];
        assert_eq!((role_mix.groups, role_mix.mentions), (2, 6), "both AOK copies, three sections each");
        let clause = plan.roles.role_mix_by_clause["buyer_review_org"];
        assert_eq!((clause.groups, clause.mentions), (2, 6));
        let samples: Vec<(i64, Vec<String>, Vec<i64>)> = plan.guard_samples["role_mix"]
            .iter()
            .map(|g| (g.notice_id, g.ref_roles.clone(), g.org_ids.clone()))
            .collect();
        let aok_roles: Vec<String> = ["Lot-AddInfo", "Lot-ReviewOrg", "Procedure-Buyer"].map(str::to_owned).to_vec();
        let aok_rows = vec![22_752_175, 22_752_176, 22_752_177];
        assert_eq!(samples, vec![(TED, aok_roles.clone(), aok_rows.clone()), (DOE, aok_roles.clone(), aok_rows)]);
        assert_eq!(plan.roles.refused_groups, 1, "the coherent twin");
        assert_eq!(plan.roles.incoherent_count, 0, "the AOK copies bind, so the party-based list no longer holds them");
        assert_eq!(plan.role_ref_notices, 3, "the three copies holding a twin group");
        assert_eq!(plan.role_ref_rows_read, 6 + 6 + 3, "TED's six id-refs, all six DE-1.x rows, Musterhausen's three");
        assert_eq!(plan.role_refs, 5 + 4 + 3, "distinct (section, role) pairs");

        // PARITY of the role read: the planner's stored read, the gate's over the loaded copy (raw and
        // normalised as the projection normalises it).
        let stored = db.guid_twin_stored_roles(&[TED, DOE, MUSTER], guid_twin_role_ref).await.unwrap();
        let want = |pairs: &[(&str, &[&str])]| -> GuidTwinSectionRoles {
            pairs.iter().map(|(s, r)| ((*s).to_owned(), r.iter().map(|x| (*x).to_owned()).collect())).collect()
        };
        assert_eq!(
            stored[&TED],
            want(&[
                ("ORG-0001", &["Procedure-Buyer"]),
                ("ORG-0002", &["Lot-AddInfo"]),
                ("ORG-0003", &["Lot-ReviewOrg"]),
                ("ORG-0004", &["Tenderer"]),
                ("ORG-0005", &["Procedure-SProvider"]),
            ])
        );
        assert_eq!(
            stored[&DOE],
            want(&[("ORG-0001", &["Procedure-Buyer"]), ("ORG-0002", &["Lot-AddInfo"]), ("ORG-0003", &["Lot-ReviewOrg"]), ("ORG-0004", &["Tenderer"])])
        );
        assert_eq!(stored[&MUSTER], want(&[("ORG-0001", &["Procedure-Buyer"]), ("ORG-0002", &["Lot-AddInfo", "Lot-TenderReceipt"])]));
        let raw = db.parsed_by_ids(&[TED, DOE, MUSTER]).await.unwrap();
        let mut normalised = raw.clone();
        super::super::normalise_de1(&mut normalised);
        for copies in [&raw, &normalised] {
            let gate: BTreeMap<i64, GuidTwinSectionRoles> = copies.iter().map(|(n, p)| (n.id, guid_twin_section_roles(p))).collect();
            assert_eq!(gate, stored, "the gate's role read equals the planner's");
        }
        assert!(normalised[2].1.values.iter().any(|v| v.field_id == "OPT-301-Lot-ReviewOrg"), "the DÖE copy was normalised");
        // PARITY of the decision: the planner's refusals are the rule's over the stored copies with the
        // gate's roles; and the gate's decision on the parsed TED copy binds the AOK group as role_mix.
        let planned = assert_parity(&db, &conn, &plan).await;
        assert!(planned.iter().all(|(n, _)| *n == MUSTER), "{planned:?}");
        let gate = decide_with(&aok(), &AOK_REFS, &GuidKeep::default());
        assert_eq!(gate.groups[0].bind, Some(TwinBind::RoleMix));
        assert_eq!(gate.groups[0].roles, aok_roles);
        drop(conn);
        cleanup(&path);
    }

    // ---- Build 2: the frozen KEEP, the wired gate, the resolver, rebuild parity ----

    /// The committed KEEP_530: its header pins [`GUID_KEEP_530_VERSION`] and names job 2660; job 2660's
    /// 7,312 rows are a FLOOR (KEEP is add-only: a reviewed append from the recurrence arm or a dry plan's
    /// `keep_add_commit` passes, a removal does not), every row six fields, strictly sorted by value (no
    /// duplicate key), every value the 32-hex key form passing `uuid_v4`, every row with a country and a
    /// decided reason (each of job 2660's reason counts a floor); the decision's pinned values are in it by
    /// value, and a published dashed lower-case value finds its key (A4).
    #[test]
    fn keep_530_file_is_frozen_as_decided() {
        let tsv = keep_530_tsv();
        assert_eq!(keep_530_file_version(tsv), Some(GUID_KEEP_530_VERSION));
        assert!(tsv.contains("queue job 2660") && tsv.contains("body.keep_candidates"), "the header names its source");
        assert!(tsv.lines().filter(|l| !l.starts_with('#')).all(|l| l.split('\t').count() == 6));
        let rows = keep_530_rows();
        assert!(rows.len() >= 7_312, "job 2660's freeze is a floor: {}", rows.len());
        assert_eq!(keep_530().len(), rows.len(), "distinct values");
        assert!(rows.windows(2).all(|w| w[0].value < w[1].value), "strictly sorted by value");
        let mut reasons: BTreeMap<&str, usize> = BTreeMap::new();
        for r in rows {
            assert!(r.value.len() == 32 && r.value.chars().all(|c| c.is_ascii_digit() || ('A'..='F').contains(&c)), "{r:?}");
            assert!(crate::idgate::uuid_v4(&r.value), "{r:?}");
            assert_eq!(GuidKeep::key(&r.value), r.value, "the GuidKeep key form");
            assert!(r.country.is_some() && r.org_id > 0, "{r:?}");
            assert!(r.reason.split('+').all(|x| ["stable", "multi_tender", "merged"].contains(&x)), "{r:?}");
            *reasons.entry(r.reason.as_str()).or_default() += 1;
        }
        for (reason, floor) in [("multi_tender", 6_967), ("stable", 13), ("stable+multi_tender", 332)] {
            assert!(reasons.get(reason).copied().unwrap_or(0) >= floor, "{reason}: {reasons:?}");
        }
        for (org, value) in [
            (254, "FB197F94757846738A574642AE120532"),
            (255, "0A9EA48008E44AB6BF12D722D0AD54B6"),
            (8_378, "9A9357BC0F794735BAE87CD270AEBE9E"),
            (3_866, "62F0F0AF9431411A8F96BE7B21DAAFB8"),
            (3_868, "85B322A2FC6745D7B38D202DD3DDDEC4"),
            (2_529, "5359C289128E40CF80A26E8B2148C363"),
            (2_530, "5EC2681225144A6685C58F38C0D0BDC6"),
            (21_983_972, "FD9B6FCE0B6847A99D6695761A9D5F0C"),
            (21_983_973, "9CC94E7110D04DE18AD5B4F807FB5D76"),
            (21_987_063, "0A2212A7F7EB4653B89E224ECA3ACA27"),
        ] {
            assert!(keep_530().contains(value), "{org}: {value} must be on KEEP");
            assert_eq!(rows.iter().find(|r| r.value == value).map(|r| r.org_id), Some(org), "{value}");
        }
        assert!(keep_530().contains("fb197f94-7578-4673-8a57-4642ae120532"), "published dashed lower case");
        assert!(!keep_530().contains("037c1508-d945-4e73-9d6e-65099a79bec1"), "Göttingen's per-notice key is no KEEP value");
    }

    /// A4 through the REAL path: `mentions_and_dropped` with the compiled KEEP, values published dashed
    /// lower case — SBH 3866/3868 twinned bind; 3866 beside a fresh value refuses only the fresh one;
    /// 255 published under AUT beside a fresh value keeps 255 and refuses the fresh one. Without KEEP
    /// the SBH pair would be refused.
    #[test]
    fn the_gate_reads_the_compiled_keep_through_the_real_path() {
        const SBH_3866: &str = "62f0f0af-9431-411a-8f96-be7b21daafb8";
        const SBH_3868: &str = "85b322a2-fc67-45d7-b38d-202dd3dddec4";
        const FRESH: &str = "5a7e9c1d-3f4b-4c8d-ae2f-a3b4c5d6e7f8";
        let ids = |orgs: &[Org<'_>]| -> Vec<Option<String>> {
            mentions(9, orgs).into_iter().map(|m| m.identifier.map(|i| i.value)).collect()
        };
        let pair = [("ORG-0001", SBH, Some("DEU"), Some(SBH_3866)), ("ORG-0002", SBH, Some("DEU"), Some(SBH_3868))];
        assert_eq!(ids(&pair), vec![Some(GuidKeep::key(SBH_3866)), Some(GuidKeep::key(SBH_3868))], "a KEEP pair binds");
        let beside = [("ORG-0001", SBH, Some("DEU"), Some(SBH_3866)), ("ORG-0002", SBH, Some("DEU"), Some(FRESH))];
        assert_eq!(ids(&beside), vec![Some(GuidKeep::key(SBH_3866)), None]);
        const VK: &str = "Vergabekammer des Bundes";
        let bund = [("ORG-0001", VK, Some("AUT"), Some(BUND_RAW)), ("ORG-0002", VK, Some("AUT"), Some(FRESH))];
        assert_eq!(ids(&bund), vec![Some(BUND.to_owned()), None], "255 kept under another country");
        let p = parsed(&pair);
        let mut published = NoticeState::published_mentions(false, 9, &p);
        assert_eq!(guid_twin_gate_with(&p, &mut published, &GuidKeep::default()), 2, "without KEEP: refused");
        assert_eq!(guid_twin_gate(&p, &mut NoticeState::published_mentions(false, 9, &p)), 0);
    }

    /// R0's shapes, decided by the WIRED gate in `mentions_and_dropped`: the Göttingen twin refuses the
    /// buyer's and the chamber's keys; a bare `Vergabekammer`, `Zentrale Vergabestelle`, `Stadtwerke
    /// GmbH` and `Freistaat Bayern` twin binds; a twin with no country binds; the Südbayern shape (no
    /// twin) and a DB-platform key named once bind; the TED copy (named once) binds beside a DÖE copy
    /// that refuses; the AOK role mix binds by the copy's own references.
    #[test]
    fn the_gate_decides_the_r0_shapes_in_mentions_and_dropped() {
        let refused = |p: &Parsed| -> Vec<String> {
            NoticeState::mentions_and_dropped(false, 9, p)
                .0
                .into_iter()
                .filter(|m| m.identifier.is_none() && m.raw_identifier.is_some())
                .map(|m| m.section_id)
                .collect()
        };
        let twin = |name: &'static str, country: Option<&'static str>| -> Parsed {
            parsed(&[
                ("ORG-0001", name, country, Some("1f2e3d4c-5b6a-4978-8a9b-0c1d2e3f4a5b")),
                ("ORG-0002", name, country, Some("2a3b4c5d-6e7f-4089-9b0c-1d2e3f4a5b6c")),
            ])
        };
        assert_eq!(refused(&parsed(&goettingen())), vec!["ORG-0001", "ORG-0002", "ORG-0003", "ORG-0004"]);
        for name in ["Vergabekammer", "Zentrale Vergabestelle", "Stadtwerke GmbH", "Freistaat Bayern"] {
            assert!(refused(&twin(name, Some("DEU"))).is_empty(), "{name} binds");
        }
        assert!(refused(&twin(GOETTINGEN, None)).is_empty(), "no country binds");
        assert_eq!(refused(&twin(GOETTINGEN, Some("DEU"))).len(), 2, "the same twin with a country refuses");
        let suedbayern = parsed(&[
            ("ORG-0001", "Staatliches Bauamt Rosenheim", Some("DEU"), Some("4d5e6f7a-8b9c-43a4-b526-d7e8f9a0b1c2")),
            ("ORG-0004", "Regierung von Oberbayern - Vergabekammer Südbayern", Some("DEU"), Some("1a2b3c4d-5e6f-4071-8293-a4b5c6d7e8f9")),
        ]);
        assert!(refused(&suedbayern).is_empty());
        let db_platform = parsed(&[
            ("ORG-0001", "DB InfraGO AG – Geschäftsbereich Fahrweg (Bukr 16)", Some("DEU"), Some("fb197f94-7578-4673-8a57-4642ae120532")),
            ("ORG-0002", "Vergabekammer des Bundes", Some("DEU"), Some(BUND_RAW)),
        ]);
        assert!(refused(&db_platform).is_empty());
        let ted = parsed(&[
            ("ORG-0001", GOETTINGEN, Some("DEU"), Some("037c1508-d945-4e73-9d6e-65099a79bec1")),
            ("ORG-0005", ROMMEL, Some("DEU"), Some("6c2d3e4f-5a6b-4c7d-9e8f-9a0b1c2d3e4f")),
        ]);
        assert!(refused(&ted).is_empty(), "the TED copy names the buyer once");
        assert_eq!(refused(&parsed_with(&aok(), &AOK_REFS)), Vec::<String>::new(), "the AOK role mix binds");
        assert_eq!(refused(&parsed(&aok())).len(), 3, "without references the same copy refuses");
        // The gate's section scope is the planner's (`ORG-` up to `ORG.`): the same refusing twin under ids
        // just outside the range binds, whatever KEEP holds — so neither the gate nor the stock plan sees
        // it, and the two never disagree; the legacy parsers' short `ORG-<n>` ids are inside the range.
        let v4a = "1f2e3d4c-5b6a-4978-8a9b-0c1d2e3f4a5b";
        let v4b = "2a3b4c5d-6e7f-4089-9b0c-1d2e3f4a5b6c";
        let outside = parsed(&[("ORG0001", GOETTINGEN, Some("DEU"), Some(v4a)), ("ORG.0002", GOETTINGEN, Some("DEU"), Some(v4b))]);
        assert!(refused(&outside).is_empty(), "outside the ORG- range: never decided");
        let mut published = NoticeState::published_mentions(false, 9, &outside);
        assert_eq!(guid_twin_gate_with(&outside, &mut published, &GuidKeep::default()), 0, "not even under an empty KEEP");
        assert!(published.iter().all(|m| m.identifier.is_some()), "both keep their keys");
        let legacy = parsed(&[("ORG-1", GOETTINGEN, Some("DEU"), Some(v4a)), ("ORG-2", GOETTINGEN, Some("DEU"), Some(v4b))]);
        assert_eq!(refused(&legacy).len(), 2, "the legacy ORG-<n> shape is in range");
    }

    /// The gate's candidate test on a stored mention (the recurrence arm's reverse-drift half and its
    /// restore): a dashed lower-case v4 raw under `DEU` is the national key a rebuild binds, under the
    /// register country; a VAT-shaped value, a non-v4 hex value and no value are none.
    #[test]
    fn the_gated_v4_test_returns_the_identity_a_rebuild_binds() {
        assert_eq!(
            guid_twin_gated_v4(Some("037c1508-d945-4e73-9d6e-65099a79bec1"), None, Some("DEU")),
            Some(("037C1508D9454E739D6E65099A79BEC1".to_owned(), Some("DE".to_owned())))
        );
        assert_eq!(guid_twin_gated_v4(Some("DE308703297"), None, Some("DEU")), None);
        assert_eq!(guid_twin_gated_v4(Some("037c1508-d945-1e73-9d6e-65099a79bec1"), None, Some("DEU")), None, "version nibble 1");
        assert_eq!(guid_twin_gated_v4(None, None, Some("DEU")), None);
    }

    /// OQ1: the gate changes a mention's binding, never the plan row's buyer key or its parsed-side
    /// link-guard tokens — issue 369's `buyer_key` and 481's `buyer_tokens_of` read the PUBLISHED identity,
    /// so the deploy moves no Tender's key. The resolved-organization token `o:<id>` (`add_org_tokens`,
    /// review round 3, F6) DOES follow the binding: the gated binding (the name-keyed 2294259) and the
    /// ungated one (the GUID row 22759261) differ by exactly those two `o:` tokens, and the published
    /// identifier's token stays in both sets (so dropping `o:<GUID row>` loses no overlap).
    #[test]
    fn the_guid_twin_gate_leaves_the_buyer_key_and_tokens_alone() {
        let refs = [("ND-ContractingParty#0", "OPT-300-Procedure-Buyer", "ORG-0001"), ("LOT-0001", "OPT-301-Lot-AddInfo", "ORG-0002")];
        let p = parsed_with(&goettingen(), &refs);
        let gated = NoticeState::mentions_and_dropped(false, 9, &p).0;
        assert!(gated[0].identifier.is_none(), "premise: the gate refuses the buyer's key");
        assert_eq!(super::super::buyer_key(false, 9, &p).as_deref(), Some("DE:national:037C1508D9454E739D6E65099A79BEC1"));
        let buyers = super::super::buyer_mentions(false, 9, &p);
        assert_eq!(buyers.len(), 1);
        assert!(buyers[0].identifier.is_some(), "the buyer side keeps the published key");
        let published = NoticeState::published_mentions(false, 9, &p);
        let parsed_side = super::super::buyer_tokens_of(&buyers);
        assert_eq!(parsed_side, super::super::buyer_tokens_of(&published[..1]));
        // The resolved-organization token follows the binding.
        let sections = vec!["ORG-0001".to_owned()];
        let mut with_gated = parsed_side.clone();
        let mut with_ungated = parsed_side.clone();
        let gated_orgs: std::collections::HashMap<String, i64> = [("ORG-0001".to_owned(), 2_294_259)].into_iter().collect();
        let ungated_orgs: std::collections::HashMap<String, i64> = [("ORG-0001".to_owned(), 22_759_261)].into_iter().collect();
        super::super::add_org_tokens(&mut with_gated, &sections, Some(&gated_orgs));
        super::super::add_org_tokens(&mut with_ungated, &sections, Some(&ungated_orgs));
        let (name_row, guid_row) = (store::buyer_token("o:2294259"), store::buyer_token("o:22759261"));
        let only_gated: Vec<u32> = with_gated.iter().copied().filter(|t| !with_ungated.contains(t)).collect();
        let only_ungated: Vec<u32> = with_ungated.iter().copied().filter(|t| !with_gated.contains(t)).collect();
        assert_eq!((only_gated, only_ungated), (vec![name_row], vec![guid_row]), "the sets differ by exactly the two o: tokens");
        let identifier_tokens: Vec<u32> = super::super::buyer_guard_tokens(&buyers[0]).iter().map(|t| store::buyer_token(t)).collect();
        assert!(!identifier_tokens.is_empty(), "premise: the published identifier gives guard tokens");
        assert!(
            identifier_tokens.iter().all(|t| with_gated.contains(t) && with_ungated.contains(t)),
            "the published identifier's tokens stay in both sets"
        );
    }

    async fn org_no_country(conn: &store::turso::Connection, id: i64, name: &str) {
        exec(
            conn,
            "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
             VALUES (?, NULL, NULL, NULL, ?, ?, 1, 0)",
            vec![Value::Integer(id), txt(name), txt(&store::org_name_norm(name))],
        )
        .await;
    }

    /// R0's last line, through the resolver: the gated Göttingen award resolves both buyer mentions onto
    /// the DE identifier-NULL row, never the VAT row of the same name nor the country-NULL row; without
    /// that row, exactly one row is minted for both (DE, identifier NULL, provisional). The chamber's two
    /// mentions share one row too; Rommel keeps its key.
    #[tokio::test]
    async fn the_resolver_lands_a_gated_twin_on_the_name_keyed_row() {
        for with_row in [true, false] {
            let (db, conn, path) = fixture(if with_row { "resolve-row" } else { "resolve-mint" }).await;
            db.set_foreign_keys(false).await.unwrap();
            org(&conn, 22_183_653, None, GOETTINGEN).await;
            exec(&conn, "UPDATE organizations SET identifier_kind = 'vat', identifier = 'DE308703297', provisional = 0 WHERE id = 22183653", vec![]).await;
            org_no_country(&conn, 16_393, GOETTINGEN).await;
            if with_row {
                org(&conn, 2_294_259, None, GOETTINGEN).await;
            }
            let ms = mentions(26_483_825, &goettingen());
            let mut resolver = db.mention_resolver(None, None, None, None, None, None, 0).await.unwrap();
            let ids = db.resolve_mentions(&mut resolver, &ms, 100).await.unwrap();
            db.finish_mention_resolver(resolver).await.unwrap();
            assert_eq!(ids.len(), 6);
            assert_eq!(ids[0], ids[1], "both buyer mentions on one row");
            assert!(ids[0] != 22_183_653 && ids[0] != 16_393, "never the VAT row or the country-NULL row");
            if with_row {
                assert_eq!(ids[0], 2_294_259);
            } else {
                let mut rows = conn
                    .query("SELECT id, country, identifier, provisional FROM organizations WHERE name_norm = 'landkreis göttingen' AND identifier IS NULL AND country = 'DE'", ())
                    .await
                    .unwrap();
                let row = rows.next().await.unwrap().expect("one minted row");
                assert_eq!(row.get_value(0).unwrap(), Value::Integer(ids[0]));
                assert_eq!(row.get_value(3).unwrap(), Value::Integer(1), "provisional");
                assert!(rows.next().await.unwrap().is_none(), "exactly one");
            }
            assert_eq!(ids[2], ids[3], "the chamber's two mentions share one row");
            let mut rows = conn.query("SELECT identifier FROM organizations WHERE id = ?", vec![Value::Integer(ids[4])]).await.unwrap();
            let rommel = rows.next().await.unwrap().unwrap().get_value(0).unwrap();
            assert_eq!(rommel, txt("6C2D3E4F5A6B4C7D9E8F9A0B1C2D3E4F"), "Rommel keeps its key");
            drop(rows);
            drop(conn);
            cleanup(&path);
        }
    }

    /// Review round 3 (F7): the deploy moves nothing. decision.md "Afterwards" ("the deploy itself causes no
    /// churn") and operations.md ("the stock moves with the wet run, once") rest on one fact: a RECORDED
    /// mention whose four published facts (name, country, raw identifier, scheme) are unchanged keeps its
    /// organization under the gated fold (`RecordedMention::publishes` never compares the identifier the
    /// gate clears, nor the resolver's answer). The Göttingen award's mentions recorded on their GUID rows
    /// stay there when the gated fold re-resolves the copy — although the resolver's own answer for the
    /// refused four is the name-keyed 2294259 — with no refresh, no rebind, no new row. The control: a
    /// changed stored name re-resolves that one mention onto 2294259 (a rebind), the others stay put.
    #[tokio::test]
    async fn a_recorded_guid_twin_mention_stays_put_under_the_gated_fold() {
        let (db, conn, path) = fixture("recorded-stays").await;
        db.set_foreign_keys(false).await.unwrap();
        let orgs = goettingen();
        let raw_of = |section: &str| orgs.iter().find(|o| o.0 == section).and_then(|o| o.3).unwrap();
        org(&conn, 22_759_261, Some(raw_of("ORG-0001")), GOETTINGEN).await;
        org(&conn, 22_759_262, Some(raw_of("ORG-0002")), GOETTINGEN).await;
        org(&conn, 23_014_306, Some(raw_of("ORG-0003")), CHAMBER).await;
        org(&conn, 23_014_307, Some(raw_of("ORG-0004")), CHAMBER).await;
        org(&conn, 22_759_265, Some(raw_of("ORG-0005")), ROMMEL).await;
        // The name-keyed row the resolver's own answer would be: the test is not vacuous.
        org(&conn, 2_294_259, None, GOETTINGEN).await;
        org(&conn, 2_300_000, None, "Deutsches Ausschreibungsblatt GmbH").await;
        let ms = mentions(26_483_825, &orgs);
        for m in &ms[..4] {
            assert!(m.identifier.is_none() && m.raw_identifier.is_some(), "premise: the gate refuses {}", m.section_id);
        }
        assert!(ms[4].identifier.is_some(), "premise: Rommel, named once, keeps its key");
        // Recorded as the ungated fold bound them, each row copying its Mention's four facts verbatim.
        let bound = [22_759_261i64, 22_759_262, 23_014_306, 23_014_307, 22_759_265, 2_300_000];
        for (m, org_id) in ms.iter().zip(bound) {
            exec(
                &conn,
                "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, country, raw_identifier, scheme)
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
                vec![
                    Value::Integer(m.notice_id),
                    txt(&m.section_id),
                    Value::Integer(org_id),
                    txt(&m.name),
                    m.country.as_deref().map_or(Value::Null, txt),
                    m.raw_identifier.as_deref().map_or(Value::Null, txt),
                    m.scheme.as_deref().map_or(Value::Null, txt),
                ],
            )
            .await;
        }
        let count = |sql: &'static str| {
            let conn = &conn;
            async move {
                let mut rows = conn.query(sql, ()).await.unwrap();
                match rows.next().await.unwrap().unwrap().get_value(0).unwrap() {
                    Value::Integer(n) => n,
                    other => panic!("{other:?}"),
                }
            }
        };
        let orgs_before = count("SELECT COUNT(*) FROM organizations").await;
        let mut r = db.mention_resolver(None, None, None, None, None, None, 0).await.unwrap();
        let ids = db.resolve_mentions(&mut r, &ms, 100).await.unwrap();
        let refresh = store::Db::mention_refresh(&r);
        db.finish_mention_resolver(r).await.unwrap();
        assert_eq!(ids, bound.to_vec(), "every recorded mention keeps its organization");
        assert!(ids[..4].iter().all(|&id| id != 2_294_259), "never the resolver's own answer");
        assert_eq!(
            refresh,
            store::MentionRefresh { refreshed: 0, rebound: 0, retired: 0, retire_corrections: 0, tenders_stamped: 0 }
        );
        assert_eq!(count("SELECT COUNT(*) FROM organizations").await, orgs_before, "no row minted");
        let mut rows = conn.query("SELECT organization_id FROM organization_mentions ORDER BY section_id", ()).await.unwrap();
        let mut stored = Vec::new();
        while let Some(row) = rows.next().await.unwrap() {
            if let Value::Integer(n) = row.get_value(0).unwrap() {
                stored.push(n);
            }
        }
        drop(rows);
        assert_eq!(stored, bound.to_vec(), "every organization_mentions row unchanged");
        // The control: a changed published fact re-resolves that mention under the gate.
        exec(&conn, "UPDATE organization_mentions SET name = 'Landkreis Goettingen (alt)' WHERE notice_id = 26483825 AND section_id = 'ORG-0002'", vec![])
            .await;
        let mut r = db.mention_resolver(None, None, None, None, None, None, 0).await.unwrap();
        let ids = db.resolve_mentions(&mut r, &ms, 100).await.unwrap();
        let refresh = store::Db::mention_refresh(&r);
        db.finish_mention_resolver(r).await.unwrap();
        assert_eq!((ids[0], ids[1]), (22_759_261, 2_294_259), "only the changed mention moves, onto the name-keyed row");
        assert_eq!((refresh.refreshed, refresh.rebound), (1, 1));
        drop(conn);
        cleanup(&path);
    }

    /// Review round 3 (F5): A1's recurrence arm admits a target mention by the GATE's candidate test
    /// (`guid_twin_gated_v4` over raw, scheme and country), never by `uuid_v4` on the raw value. A BT-501
    /// that wraps the UUID in braces is refused by the gate, planned and logged by the wet run — and must
    /// count: both braced mentions are refused values (one value in two Tenders: a KEEP-add candidate with
    /// its two-mention cohort), and after that value joins KEEP each is STOCK reverse drift, also where the
    /// braced mention is its copy's only target mention (its plain partner rehomed elsewhere).
    #[tokio::test]
    async fn the_recurrence_arm_reads_a_target_mention_by_the_gates_candidate_test() {
        const BRACED: &str = "{037c1508-d945-4e73-9d6e-65099a79bec1}";
        assert!(!crate::idgate::uuid_v4(BRACED), "premise: uuid_v4 on the raw value refuses the braces");
        assert_eq!(guid_twin_gated_v4(Some(BRACED), None, Some("DE")), Some((GuidKeep::key(A), Some("DE".to_owned()))), "premise");
        let (db, conn, path) = fixture("recur-braced").await;
        db.set_foreign_keys(false).await.unwrap();
        org(&conn, 22_759_261, Some(A), GOETTINGEN).await;
        org(&conn, 22_759_262, Some(B), GOETTINGEN).await;
        org(&conn, 22_800_001, Some(P), GOETTINGEN).await;
        org(&conn, 2_294_259, None, GOETTINGEN).await;
        org(&conn, 2_300_000, None, "Landkreis Göttingen Altkreis Duderstadt").await;
        for (n, tender, partner, raw) in [(26_483_825i64, 948_285i64, 22_759_262i64, B), (26_483_900, 948_300, 22_800_001, P)] {
            notice(&conn, n, "doe", tender, 1).await;
            mention(&conn, n, "ORG-0001", 22_759_261, GOETTINGEN, Some(BRACED), "", (tender, 1), None).await;
            mention(&conn, n, "ORG-0002", partner, GOETTINGEN, Some(raw), "", (tender, 1), None).await;
        }
        let empty = GuidKeep::default();
        let plan = run_plan_with(&db, Some(&empty), &[], &[]).await;
        assert_eq!(plan.planned_row_ids, vec![22_759_261, 22_759_262, 22_800_001], "the planner reads the gated value");
        for (k, w) in store::guid_twin_windows(&plan.wet_mentions, store::GUID_TWIN_WET_WINDOW_MENTIONS).iter().enumerate() {
            db.dissolve_guid_twin_window(store::GuidTwinWindowArgs {
                window: w,
                job_id: 1,
                window_no: k as u64 + 1,
                gate_since_notice_id: Some(26_483_824),
                fields: super::super::ORG_NAME_FIELD_IDS,
                normalize: super::super::normalize_lang,
            })
            .await
            .unwrap();
        }
        assert_eq!(db.guid_twin_log(1).await.unwrap().len(), 4, "every refused mention logged onto 2294259");
        let recur = |keep: GuidKeep| {
            let db = &db;
            async move {
                let stop = || false;
                let progress = |_: u64, _: &str| {};
                db.guid_twin_recurrence(store::GuidTwinRecurrenceArgs {
                    is_guid: crate::idgate::uuid_v4,
                    gated_v4: guid_twin_gated_v4,
                    rule: guid_twin_decide,
                    role_ref: guid_twin_role_ref,
                    keep: &keep,
                    since_notice_id: None,
                    window: 1_000,
                    samples: 10,
                    stop: &stop,
                    progress: &progress,
                })
                .await
                .unwrap()
            }
        };
        let r = recur(GuidKeep::default()).await;
        assert_eq!((r.guid_mentions, r.refused_mentions), (4, 4), "the braced mentions count: {r:?}");
        assert_eq!(r.candidate_count, 1, "{r:?}");
        let c = &r.candidates[0];
        assert_eq!((c.value.clone(), c.tenders), (GuidKeep::key(A), 2));
        assert_eq!(c.cohort, vec![(26_483_825, "ORG-0001".to_owned(), 2_294_259), (26_483_900, "ORG-0001".to_owned(), 2_294_259)]);
        // A reviewed rehome moved copy 26483825's plain partner off the target: its braced mention is the
        // copy's only target mention. The value joins KEEP: both braced mentions are stock reverse drift.
        exec(&conn, "UPDATE organization_mentions SET organization_id = 2300000 WHERE notice_id = 26483825 AND section_id = 'ORG-0002'", vec![])
            .await;
        let r = recur([A].into_iter().collect()).await;
        assert_eq!(r.reverse_drift_stock, 2, "{r:?}");
        let cohort: Vec<(i64, String)> = r.reverse_drift.iter().flat_map(|v| v.cohort.iter().map(|(n, s, _)| (*n, s.clone()))).collect();
        assert_eq!(cohort, vec![(26_483_825, "ORG-0001".to_owned()), (26_483_900, "ORG-0001".to_owned())]);
        assert_eq!(r.reverse_drift[0].value, GuidKeep::key(A), "keyed by the gated value, the identifier a rebuild stores");
        drop(conn);
        cleanup(&path);
    }

    // The real KEEP values the compiled-constant fixtures publish (dashed lower case).
    const SBH_3866_RAW: &str = "62f0f0af-9431-411a-8f96-be7b21daafb8";
    const SBH_3868_RAW: &str = "85b322a2-fc67-45d7-b38d-202dd3dddec4";

    /// PARITY on the COMPILED constants (A3 iii): over an ungated stock — the Göttingen award in both
    /// copies, the real SBH 3866/3868 pair, 3866 beside a fresh value, 254 beside a fresh value — the
    /// planner run with `keep_530()` refuses exactly what the gate refuses: the rule over the stored copies
    /// with the same KEEP, and `mentions_and_dropped` over the parsed copies (identifier refused, raw a v4
    /// national key), with no KEEP handed to the gate. A KEEP without 254's value fails the compiled
    /// self-check, which names the value and its KEEP-add line.
    #[tokio::test]
    async fn the_planner_and_the_gate_agree_on_the_compiled_constants() {
        let (db, conn, path) = fixture("compiled").await;
        org(&conn, 254, Some(DB), "DB InfraGO AG").await;
        org(&conn, 2_294_259, None, GOETTINGEN).await;
        org(&conn, 22_759_261, Some(A), GOETTINGEN).await;
        org(&conn, 22_759_262, Some(B), GOETTINGEN).await;
        org(&conn, 22_759_265, Some(C), ROMMEL).await;
        org(&conn, 23_014_306, Some(D), CHAMBER).await;
        org(&conn, 23_014_307, Some(E), CHAMBER).await;
        org(&conn, 3_866, Some(SBH_3866_RAW), SBH).await;
        org(&conn, 3_868, Some(SBH_3868_RAW), SBH).await;
        org(&conn, 30_000_110, Some(N), SBH).await;
        org(&conn, 30_000_111, Some(R), "DB InfraGO AG").await;
        let copies: Vec<(i64, &str, i64, Vec<Org<'static>>)> = vec![
            (
                24_262_502,
                "ted",
                1,
                vec![
                    ("ORG-0001", GOETTINGEN, Some("DEU"), Some(A)),
                    ("ORG-0002", GOETTINGEN, Some("DEU"), Some(B)),
                    ("ORG-0003", CHAMBER, Some("DEU"), Some(D)),
                    ("ORG-0005", ROMMEL, Some("DEU"), Some(C)),
                ],
            ),
            (
                26_483_825,
                "doe",
                2,
                vec![
                    ("ORG-0001", GOETTINGEN, Some("DEU"), Some(A)),
                    ("ORG-0002", GOETTINGEN, Some("DEU"), Some(B)),
                    ("ORG-0003", CHAMBER, Some("DEU"), Some(D)),
                    ("ORG-0004", CHAMBER, Some("DEU"), Some(E)),
                    ("ORG-0005", ROMMEL, Some("DEU"), Some(C)),
                ],
            ),
            (40_000_005, "ted", 1, vec![("ORG-0001", SBH, Some("DEU"), Some(SBH_3866_RAW)), ("ORG-0002", SBH, Some("DEU"), Some(SBH_3868_RAW))]),
            (40_000_006, "ted", 1, vec![("ORG-0001", SBH, Some("DEU"), Some(SBH_3866_RAW)), ("ORG-0002", SBH, Some("DEU"), Some(N))]),
            (40_000_008, "ted", 1, vec![("ORG-0001", "DB InfraGO AG", Some("DEU"), Some(DB))]),
            (40_000_009, "ted", 1, vec![("ORG-0001", "DB InfraGO AG", Some("DEU"), Some(DB)), ("ORG-0002", "DB InfraGO AG", Some("DEU"), Some(R))]),
        ];
        let org_of = |raw: &str| -> i64 {
            match raw {
                A => 22_759_261,
                B => 22_759_262,
                C => 22_759_265,
                D => 23_014_306,
                E => 23_014_307,
                SBH_3866_RAW => 3_866,
                SBH_3868_RAW => 3_868,
                N => 30_000_110,
                R => 30_000_111,
                DB => 254,
                other => panic!("{other}"),
            }
        };
        for (n, source, seq, orgs) in &copies {
            let tender = if *n < 30_000_000 { 948_285 } else { *n - 39_010_000 };
            notice(&conn, *n, source, tender, *seq).await;
            for (section, name, _, raw) in orgs {
                mention(&conn, *n, section, org_of(raw.unwrap()), name, *raw, "", (tender, *seq), None).await;
            }
        }
        let keep = keep_530();
        let plan = run_plan_with(&db, Some(keep), &[254, 3_866, 3_868, 22_759_265], &[(22_759_261, 2_294_259), (22_759_262, 2_294_259)]).await;
        assert_eq!((plan.keep_source, plan.keep_size, plan.keep_version.as_str()), ("compiled", keep_530().len() as u64, GUID_KEEP_530_VERSION));
        assert_eq!(plan.planned_row_ids, vec![22_759_261, 22_759_262, 23_014_307, 30_000_110, 30_000_111]);
        assert_eq!(plan.partial_row_ids, vec![23_014_306]);
        assert!(plan.bars.keep_self_check_missing.is_empty() && plan.keep_add_commit.is_empty());
        assert!(plan.bars.pass, "{:#?}", plan.bars);
        let planned = assert_parity_with(&db, &conn, &plan, keep).await;
        // The gate's side: the parsed copies through mentions_and_dropped, the compiled KEEP inside it.
        let mut by_gate: BTreeSet<(i64, String)> = BTreeSet::new();
        for (n, _, _, orgs) in &copies {
            for m in mentions(*n, orgs) {
                let v4 = gated_identifier(m.raw_identifier.as_deref(), m.scheme.as_deref(), m.country.as_deref())
                    .is_some_and(|i| i.kind == "national" && crate::idgate::uuid_v4(&i.value));
                if m.identifier.is_none() && v4 {
                    by_gate.insert((*n, m.section_id));
                }
            }
        }
        assert_eq!(planned, by_gate, "the planner on the compiled constants refuses exactly what the gate refuses");
        // A KEEP without 254's value: the self-check (254 is stable) names it and its line.
        let db_key = GuidKeep::key(DB);
        let without: GuidKeep = keep_530_rows().iter().map(|r| r.value.as_str()).filter(|v| *v != db_key).collect();
        let failing = run_plan_with(&db, Some(&without), &[254], &[]).await;
        assert_eq!(failing.bars.keep_self_check_missing, vec![db_key.clone()]);
        assert_eq!(failing.keep_add_commit, vec![format!("DE\t{db_key}\t254\t2\t2\tstable+multi_tender")]);
        assert!(!failing.bars.pass);
        drop(conn);
        cleanup(&path);
    }

    /// The identity a (notice, section) is bound to: an identified row's key, or a name-keyed row's
    /// (country, name_norm).
    async fn identity_of(conn: &store::turso::Connection, notice: i64, section: &str) -> (i64, String) {
        let mut rows = conn
            .query(
                "SELECT o.id, o.identifier, o.country, o.name_norm FROM organization_mentions m JOIN organizations o \
                   ON o.id = m.organization_id WHERE m.notice_id = ? AND m.section_id = ?",
                vec![Value::Integer(notice), txt(section)],
            )
            .await
            .unwrap();
        let row = rows.next().await.unwrap().unwrap_or_else(|| panic!("({notice}, {section}) is bound"));
        let s = |i: usize| row.get_value(i).ok().and_then(|v| v.as_text().cloned());
        let id = match row.get_value(0).unwrap() {
            Value::Integer(n) => n,
            _ => unreachable!(),
        };
        let class = match s(1) {
            Some(identifier) => format!("identified {identifier}"),
            None => format!("name-keyed {} {}", s(2).unwrap_or_default(), s(3).unwrap_or_default()),
        };
        (id, class)
    }

    /// A2's parity, and the MINT decision's: the stock — the ungated fold of a DÖE copy naming the
    /// Göttingen buyer under A and B and its TED copy naming it once under A (so A is PARTIAL, B whole),
    /// plus an `Amt 66` twin in two notices with no identifier-NULL row — planned with the compiled KEEP
    /// and dissolved by the wet windows, equals a fresh projection of the same copies under the gate: per
    /// (notice, section) the same identity class, and the same partition of the mentions into rows. Both
    /// with the Göttingen identifier-NULL row present on both sides (the same row) and with none (each
    /// side mints one; the missing key is minted ONCE across its two notices on each side).
    #[tokio::test]
    async fn a_two_copy_stock_dissolve_equals_a_rebuild_under_the_gate() {
        const AMT: [&str; 4] = [
            "10a1b2c3-d4e5-4f60-8a1b-2c3d4e5f6071",
            "20a1b2c3-d4e5-4f60-9a1b-2c3d4e5f6072",
            "30a1b2c3-d4e5-4f60-aa1b-2c3d4e5f6073",
            "40a1b2c3-d4e5-4f60-ba1b-2c3d4e5f6074",
        ];
        let copies: Vec<(i64, &str, i64, i64, Vec<Org<'static>>)> = vec![
            (1, "doe", 11, 1, vec![("ORG-0001", "Amt 66", Some("DEU"), Some(AMT[0])), ("ORG-0002", "Amt 66", Some("DEU"), Some(AMT[1]))]),
            (2, "doe", 12, 1, vec![("ORG-0001", "Amt 66", Some("DEU"), Some(AMT[2])), ("ORG-0002", "Amt 66", Some("DEU"), Some(AMT[3]))]),
            (
                24_262_502,
                "ted",
                948_285,
                1,
                vec![("ORG-0001", GOETTINGEN, Some("DEU"), Some(A)), ("ORG-0005", ROMMEL, Some("DEU"), Some(C))],
            ),
            (
                26_483_825,
                "doe",
                948_285,
                2,
                vec![
                    ("ORG-0001", GOETTINGEN, Some("DEU"), Some(A)),
                    ("ORG-0002", GOETTINGEN, Some("DEU"), Some(B)),
                    ("ORG-0005", ROMMEL, Some("DEU"), Some(C)),
                ],
            ),
        ];
        let role = |section: &str| match section {
            "ORG-0001" => "Procedure-Buyer",
            "ORG-0002" => "Lot-AddInfo",
            _ => "bid:tenderer",
        };
        for with_row in [true, false] {
            let tag = if with_row { "row" } else { "mint" };
            // The stock: the ungated fold, as production holds it before the dissolve.
            let (stock, conn, path) = fixture(&format!("a2-stock-{tag}")).await;
            stock.set_foreign_keys(false).await.unwrap();
            if with_row {
                org(&conn, 2_294_259, None, GOETTINGEN).await;
            }
            let rows: [(i64, &str, &str); 7] = [
                (22_759_261, A, GOETTINGEN),
                (22_759_262, B, GOETTINGEN),
                (22_759_265, C, ROMMEL),
                (101, AMT[0], "Amt 66"),
                (102, AMT[1], "Amt 66"),
                (103, AMT[2], "Amt 66"),
                (104, AMT[3], "Amt 66"),
            ];
            for (id, raw, name) in rows {
                org(&conn, id, Some(raw), name).await;
            }
            for (n, source, tender, seq, orgs) in &copies {
                notice(&conn, *n, source, *tender, *seq).await;
                for (section, name, _, raw) in orgs {
                    let id = rows.iter().find(|r| Some(r.1) == *raw).unwrap().0;
                    mention(&conn, *n, section, id, name, *raw, role(section), (*tender, *seq), None).await;
                }
            }
            let plan = run_plan_with(&stock, Some(keep_530()), &[], &[]).await;
            assert_eq!(plan.planned_row_ids, vec![101, 102, 103, 104, 22_759_262], "{tag}");
            assert_eq!(plan.partial_row_ids, vec![22_759_261], "{tag}: A is partial (its TED mention is kept)");
            for (k, w) in store::guid_twin_windows(&plan.wet_mentions, store::GUID_TWIN_WET_WINDOW_MENTIONS).iter().enumerate() {
                stock
                    .dissolve_guid_twin_window(store::GuidTwinWindowArgs {
                        window: w,
                        job_id: 1,
                        window_no: k as u64 + 1,
                        gate_since_notice_id: None,
                        fields: super::super::ORG_NAME_FIELD_IDS,
                        normalize: super::super::normalize_lang,
                    })
                    .await
                    .unwrap();
            }
            // The rebuild: a fresh store resolving the same copies' gated mentions, in notice order.
            let (fresh, fconn, fpath) = fixture(&format!("a2-fresh-{tag}")).await;
            fresh.set_foreign_keys(false).await.unwrap();
            if with_row {
                org(&fconn, 2_294_259, None, GOETTINGEN).await;
            }
            let mut resolver = fresh.mention_resolver(None, None, None, None, None, None, 0).await.unwrap();
            for (n, _, _, _, orgs) in &copies {
                fresh.resolve_mentions(&mut resolver, &mentions(*n, orgs), 100).await.unwrap();
            }
            fresh.finish_mention_resolver(resolver).await.unwrap();
            // Per (notice, section): the identity class, and the partition into rows.
            let mut stock_rows: BTreeMap<i64, BTreeSet<(i64, String)>> = BTreeMap::new();
            let mut fresh_rows: BTreeMap<i64, BTreeSet<(i64, String)>> = BTreeMap::new();
            for (n, _, _, _, orgs) in &copies {
                for (section, ..) in orgs {
                    let (sid, sclass) = identity_of(&conn, *n, section).await;
                    let (fid, fclass) = identity_of(&fconn, *n, section).await;
                    assert_eq!(sclass, fclass, "{tag}: ({n}, {section}) has one identity class in the stock and the rebuild");
                    stock_rows.entry(sid).or_default().insert((*n, (*section).to_owned()));
                    fresh_rows.entry(fid).or_default().insert((*n, (*section).to_owned()));
                    if with_row && *n == 26_483_825 && *section != "ORG-0005" {
                        assert_eq!((sid, fid), (2_294_259, 2_294_259), "{tag}: the same identifier-NULL row");
                    }
                }
            }
            let partition = |m: BTreeMap<i64, BTreeSet<(i64, String)>>| m.into_values().collect::<BTreeSet<_>>();
            assert_eq!(partition(stock_rows), partition(fresh_rows), "{tag}: the same partition into rows");
            let amt = identity_of(&conn, 1, "ORG-0001").await.0;
            assert_eq!(identity_of(&conn, 2, "ORG-0002").await.0, amt, "{tag}: the missing key minted once across two notices");
            drop(conn);
            drop(fconn);
            cleanup(&path);
            cleanup(&fpath);
        }
    }
}
