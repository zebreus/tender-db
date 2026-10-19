//! Issue 530 unit 3, Build 1: rule 530-T, the within-notice GUID twin, as a PURE function of one notice
//! copy ([`guid_twin_decide`], [`guid_twin_refusals`]), and its name guard.
//!
//! Some German eForms platforms mint a fresh BT-501 UUID per ORG section per notice, so one body is a
//! row per notice. When ONE notice copy prints one name-safe name under two or more distinct v4 values,
//! the notice itself proves the values are no entity key (`530-triage/decision.md`, the pair-scoped (a)
//! "530-T", critic amendments A1–A12 adopted). The rule refuses those values' identifiers — the mention
//! then takes the issue-234 identifier-NULL path, `raw_identifier` kept — except values on the frozen,
//! value-keyed `KEEP_530` (A4), names that are not safe as a fallback key (A8), and groups with no
//! country.
//!
//! It sits beside `idgate::condemns`, not inside it: `condemns` sees one value, this rule needs the
//! whole notice. `condemns` and `idgate::uuid_v4` are unchanged. It lives under `project` because the
//! name guard is the projection's own (`guard_name_words`, `content_words`, `generic_body_name`), and
//! the gate (Build 2) runs in `NoticeState::mentions_and_dropped`.
//!
//! **Build 1 does not wire the gate.** Production ingest is unchanged: `mentions_and_dropped` still
//! binds every GUID (pinned by `production_ingest_still_binds_a_twin_notices_guids`). The gate cannot be
//! wired before the KEEP file exists, or it would refuse the SBH, GMH and Nürnberg pairs. The dry
//! `dissolve-guid-twins` planner (`store::Db::plan_guid_twins`) calls [`guid_twin_decide`] through a fn
//! pointer, so the plan, the gate and any rebuild decide with one ruler.
//!
//! **Identity becomes contextual once the gate is wired.** A tool that rebuilds a mention's identity
//! from its stored row alone ([`super::gated_identifier`]'s callers: 513's nhs-body canary, 511's guard
//! 3) sees a GUID where the fold refused one. None of them reads DE GUID mentions today; a future reader
//! re-derives the twin from the notice's stored mentions, as the planner does.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::sync::OnceLock;

use store::{GuidKeep, TwinBind, TwinDecision, TwinGroup, TwinView};

use super::{
    PREFIX_FUNCTION_WORDS, canonical_country, content_words, gated_identifier, generic_body_name, guard_name_words,
    merge_initials,
};

/// Issue 530 (decision §5, A8 ii): names that are not safe as a twin's fallback key, beside
/// `generic_body_name`'s list. A 530-ONLY list on purpose: adding these to `GENERIC_BODY_NAMES` would
/// change the head and prefix tokens of 481's buyer guard, a different feature. Seeded with the
/// decision's generics (the precision judge's GETs: `Zentrale Vergabestelle`, `Tribunal administratif`;
/// the one-word office names, which the content-word test already binds, listed so the list reads
/// whole), the polity umbrellas and the person placeholders of A8; R2 convictions are added here
/// (between deploys, through the job's `extra_unsafe_names`).
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
/// `Amt 66`, `Referat Beschaffung`) is read whole in R2 (decision R2 "Also read").
pub const GUID_TWIN_OFFICE_NOUNS: &[&str] = &[
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

/// Rule 530-T over ONE notice copy (a TED copy and a DÖE copy are each decided alone):
///
/// 1. Candidates: sections whose gated identifier ([`gated_identifier`] under the canonical country,
///    the gate's own path) has kind `national` and a value passing `idgate::uuid_v4`.
/// 2. Groups: by `(identifier.country, store::org_name_norm(name))` — the census's twin key made finer
///    by the register country, the key the 234 fallback resolves on. A candidate with an empty name
///    joins no group; one with no country forms a group that always binds (`no_country`).
/// 3. Twin: a group holding 2 or more DISTINCT values.
/// 4. Refusal: every section of a twin whose value is not in `keep` — unless the name is not safe
///    ([`guid_twin_unsafe_name`]) or every value is in `keep`, when the whole group binds.
///
/// Pure: no DB read, no state. The planner passes it to store as a `store::GuidTwinRule`.
pub fn guid_twin_decide(sections: &[TwinView<'_>], keep: &GuidKeep, extra_unsafe_names: &[String]) -> TwinDecision {
    let mut decision = TwinDecision::default();
    let mut groups: BTreeMap<(Option<String>, String), Vec<(String, String)>> = BTreeMap::new();
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
        groups.entry((id.country, norm)).or_default().push((s.section_id.to_owned(), id.value));
    }
    for ((country, name_norm), members) in groups {
        let distinct = members.iter().map(|(_, v)| v.as_str()).collect::<BTreeSet<_>>().len();
        if distinct < 2 {
            continue;
        }
        let bind = if country.is_none() {
            Some(TwinBind::NoCountry)
        } else if let Some(reason) = guid_twin_unsafe_name(&name_norm, extra_unsafe_names) {
            Some(reason)
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
        });
    }
    decision
}

/// The section ids whose identifier rule 530-T refuses in one notice copy ([`guid_twin_decide`]'s
/// `refused`): what the gate (Build 2) sets to `identifier = None`.
pub fn guid_twin_refusals(sections: &[TwinView<'_>], keep: &GuidKeep, extra_unsafe_names: &[String]) -> BTreeSet<String> {
    guid_twin_decide(sections, keep, extra_unsafe_names).refused
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
            })
            .collect()
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

    /// The gate is NOT wired in Build 1: production ingest of a twin notice is unchanged — every GUID
    /// still binds as the national identifier it always did, and the raw value stays as published.
    #[test]
    fn production_ingest_still_binds_a_twin_notices_guids() {
        let m = mentions(26_483_825, &goettingen());
        assert!(!guid_twin_refusals(&views(&m), &none(), &[]).is_empty(), "the rule would refuse here");
        let ids: Vec<(&str, Option<(&str, &str, &str)>)> = m
            .iter()
            .map(|m| {
                (
                    m.section_id.as_str(),
                    m.identifier.as_ref().map(|i| (i.country.as_deref().unwrap_or(""), i.kind.as_str(), i.value.as_str())),
                )
            })
            .collect();
        assert_eq!(
            ids,
            vec![
                ("ORG-0001", Some(("DE", "national", "037C1508D9454E739D6E65099A79BEC1"))),
                ("ORG-0002", Some(("DE", "national", "5B1C2D3E4F5A4B6C8D7E8F9A0B1C2D3E"))),
                ("ORG-0003", Some(("DE", "national", "6A1B2C3D4E5F4A6B9C7D8E9F0A1B2C3D"))),
                ("ORG-0004", Some(("DE", "national", "7B2C3D4E5F6A4B7CAD8E9F0A1B2C3D4E"))),
                ("ORG-0005", Some(("DE", "national", "6C2D3E4F5A6B4C7D9E8F9A0B1C2D3E4F"))),
                ("ORG-0006", None),
            ],
            "mentions_and_dropped binds every GUID: Build 1 changes no identity"
        );
        assert_eq!(m[0].raw_identifier.as_deref(), Some("037c1508-d945-4e73-9d6e-65099a79bec1"));
        assert_eq!(m[0].country.as_deref(), Some("DE"));
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
            TwinView { section_id: "ORG-1", name: " ", country: Some("DE"), raw_identifier: Some("037c1508-d945-4e73-9d6e-65099a79bec1"), scheme: None },
            TwinView { section_id: "ORG-2", name: "", country: Some("DE"), raw_identifier: Some("5b1c2d3e-4f5a-4b6c-8d7e-8f9a0b1c2d3e"), scheme: None },
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
    /// given R1 controls.
    async fn run_plan(db: &store::Db, not_planned: &[i64], planned: &[(i64, i64)]) -> store::GuidTwinPlan {
        let stop = || false;
        let progress = |_: u64, _: &str| {};
        let plan = db
            .plan_guid_twins(store::GuidTwinPlanArgs {
                is_guid: crate::idgate::uuid_v4,
                rule: guid_twin_decide,
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
    /// with the plan's own KEEP. Returns the planner's set.
    async fn assert_parity(conn: &store::turso::Connection, plan: &store::GuidTwinPlan) -> BTreeSet<(i64, String)> {
        assert!(!plan.planned_mention_keys_truncated);
        let keep: GuidKeep = plan.keep_candidates.iter().map(|c| c.value.clone()).collect();
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
        let mut by_rule: BTreeSet<(i64, String)> = BTreeSet::new();
        for (n, ms) in &stored {
            let views: Vec<TwinView<'_>> = ms
                .iter()
                .map(|(sec, name, country, raw, scheme)| TwinView {
                    section_id: sec,
                    name,
                    country: country.as_deref(),
                    raw_identifier: raw.as_deref(),
                    scheme: scheme.as_deref(),
                })
                .collect();
            by_rule.extend(guid_twin_refusals(&views, &keep, &[]).into_iter().map(|s| (*n, s)));
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
    ///   Bau AG as a bid tenderer (H) and Lot-AddInfo (I): two role-incoherent refused groups. Their
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

        // Role composition: buyer + Lot-ReviewOrg (Klinikum), a bid tenderer + Lot-AddInfo (Muster Bau).
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
        let planned = assert_parity(&conn, &plan).await;
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
        assert_parity(&conn, &plan).await;
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
        assert_parity(&conn, &plan).await;
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
        assert_parity(&conn, &plan).await;
        drop(conn);
        cleanup(&path);
    }
}
