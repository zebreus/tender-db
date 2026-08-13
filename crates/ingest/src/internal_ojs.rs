//! The `internal-ojs` mapping profile: the 2008 OPOCE `INTERNAL_OJS` export
//! (DTD R2.0.5), ~28k real S-series notices published across ~22 daily
//! packages in May 2008 — the whole 2008 verify coverage gap (issue 41).
//!
//! It is not a from-scratch 577-element profile. 85% of an INTERNAL_OJS form
//! body is the very r208/r209 TED_EXPORT vocabulary the [`crate::r209`] walker
//! already maps, so this profile is a thin delta over it (ADR-0002, issue 41's
//! recorded design):
//!
//! 1. a **new envelope** — the `INTERNAL_OJS` root plus `TECHNICAL_INFO` /
//!    `BIB_INFO` / `BIB_DOC_S`, carrying identity (`NO_DOC_OJS`, `HEADING`) and
//!    the bare-text CODIF backbone (`SECTOR`, `MARKET`, `PROC`, … — single-char
//!    codes, *not* r209's `@CODE`-attribute form) plus the dispatch/receipt
//!    dates and `ISO_COUNTRY`;
//! 2. a **`_SUM` alias table** ([`SUM_ALIASES`]): the summary form reuses the
//!    r209 element names with a `_SUM` suffix (`CONTRACT_SUM` → `CONTRACT`,
//!    `FD_CONTRACT_AWARD_SUM` → `FD_CONTRACT_AWARD`, …). The table is explicit
//!    and checked in — deliberately *not* a runtime strip-the-suffix rule — so
//!    that if a `_SUM` element ever diverged from its base it would be a visible
//!    diff line and the pinning test would catch it, and a `_SUM` element absent
//!    from the table fails the exhaustive-consumption walk loudly rather than
//!    being silently normalised (issue 41, lead 2026-07-21).
//!
//! The walker itself is [`crate::r209::parse::parse_internal_ojs`], driven by
//! this module's [`alias`] and [`envelope_rule`]. Traps where INTERNAL_OJS
//! shares a name with r209 but not a shape are handled by the overlay
//! ([`envelope_rule`]); the ones already covered by an r209 rule's text
//! fallback (`ORIGINAL_CPV` / `ORIGINAL_NUTS`, bare-text here vs `@CODE` in
//! r209) are left to it.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::r209::rules::{Kind, Rule};

/// This profile's id, as written by the dispatcher and matched by the
/// processor.
pub const PROFILE: &str = "internal-ojs";

/// Field mapping for one INTERNAL_OJS payload. Non-`internal-ojs` profiles stay
/// `Pending` (identity only), mirroring the r209/eForms parsers.
pub fn parse_payload(profile: &str, bytes: &[u8]) -> store::Parse {
    if profile != PROFILE {
        return store::Parse::Pending;
    }
    let Ok(xml) = std::str::from_utf8(bytes) else {
        return store::Parse::Quarantined { reason: "not-utf8".into(), detail: None };
    };
    // The R2.0.5 payload is a real notice behind a DTD; the strip is the same
    // XXE-safe one the dispatcher used to reach the root (issue 36).
    let stripped = crate::profile::strip_doctype(xml);
    match crate::r209::parse::parse_internal_ojs(&stripped, alias, envelope_rule) {
        Ok(parsed) => store::Parse::Parsed(parsed),
        Err(crate::r209::Rejected { reason, detail }) => {
            store::Parse::Quarantined { reason: reason.into(), detail: Some(detail) }
        }
    }
}

/// Normalise a `_SUM` summary element to its base r209 element via the explicit
/// [`SUM_ALIASES`] table; every other name passes through unchanged.
pub fn alias(name: &str) -> &str {
    table().get(name).copied().unwrap_or(name)
}

fn table() -> &'static HashMap<&'static str, &'static str> {
    static TABLE: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    TABLE.get_or_init(|| SUM_ALIASES.iter().copied().collect())
}

/// The INTERNAL_OJS-specific rules, consulted before the r209 registry.
///
/// Two families: the ~20 genuinely new envelope/backbone elements, and a few
/// overrides where INTERNAL_OJS reuses an r209 name with a different shape —
/// `ISO_COUNTRY` is a bare-text code here (r209 reads it from `@VALUE`, so the
/// r209 rule would drop it), and `SERVICE_CATEGORY` / `SERVICE_CATEGORY_PUB`
/// carry the code in `@VALUE` (r209 reads it from the element text). Matching is
/// by name; the new names are globally unique and the overrides are safe
/// era-wide (the shared names only occur in these positions in the corpus).
pub fn envelope_rule(_parent: &str, name: &str) -> Option<Rule> {
    Some(match name {
        // Envelope containers.
        "TECHNICAL_INFO" | "BIB_INFO" | "BIB_DOC_S" => Rule::Group,
        // Bare-text single-char CODIF codes + the OJ language, and the
        // `ISO_COUNTRY` override (bare text here, not r209's `@VALUE`).
        "SECTOR" | "NAT_NOTICE" | "MARKET" | "PROC" | "MARKET_ORG" | "TYPE_BID" | "AWARD_CRIT"
        | "MAIN_ACTIVITIES" | "LG_OJ" | "ISO_COUNTRY" => Rule::CodeText,
        // Dispatch / receipt dates. `DEADLINE_REC` pairs a wall clock in one
        // value (`20080611 12:00`); the rest are bare compact dates.
        "DATE_DISP" | "DATE_REC" | "DEADLINE_REQ" => Rule::Date,
        "DEADLINE_REC" => Rule::DateTime,
        // Overrides: the service category moved to `@VALUE`; `SERVICE_CATEGORY_PUB`
        // may then wrap a bare `AGREEMENT_PUBLICATION` marker.
        "SERVICE_CATEGORY" | "SERVICE_CATEGORY_PUB" => Rule::CodeAttr(VALUE_ATTR),
        "AGREEMENT_PUBLICATION" => Rule::Marker,
        // A repeatable object-description lot wrapper, and the coded service
        // category on `@CATEGORY`.
        "LOTS" => Rule::Section(Kind::Lot),
        "SERVICES" => Rule::CodeAttr(CATEGORY_ATTR),
        _ => return None,
    })
}

const VALUE_ATTR: &[&str] = &["VALUE"];
const CATEGORY_ATTR: &[&str] = &["CATEGORY"];

/// Element names this profile decides on its own (the envelope/backbone
/// delta), for the completeness test. The `_SUM` aliases and the 85% shared
/// vocabulary are decided by the r209 registry through [`alias`].
pub fn decided_names() -> impl Iterator<Item = &'static str> {
    // Keep in step with `envelope_rule`.
    [
        "TECHNICAL_INFO", "BIB_INFO", "BIB_DOC_S", "SECTOR", "NAT_NOTICE", "MARKET", "PROC",
        "MARKET_ORG", "TYPE_BID", "AWARD_CRIT", "MAIN_ACTIVITIES", "LG_OJ", "ISO_COUNTRY",
        "DATE_DISP", "DATE_REC", "DEADLINE_REQ", "DEADLINE_REC", "SERVICE_CATEGORY",
        "SERVICE_CATEGORY_PUB", "AGREEMENT_PUBLICATION", "LOTS", "SERVICES",
    ]
    .into_iter()
}

/// The explicit, checked-in `_SUM` → base-r209-element alias table, mined from
/// six May-2008 packages (`2008085`–`2008105`) with **0 drift** — every base is
/// an r209 element (`crates/ingest/src/r209/rules.rs`); see
/// `.scratch/tender-db/issues/41-sum-alias-table.tsv` for provenance and counts.
/// A `_SUM` element not listed here is *not* normalised: it fails the walk as
/// unclaimed content, surfacing the drift instead of hiding it.
#[rustfmt::skip]
pub const SUM_ALIASES: &[(&str, &str)] = &[
    ("ADMINISTRATIVE_INFORMATION_CONCESSION_SUM",             "ADMINISTRATIVE_INFORMATION_CONCESSION"),
    ("ADMINISTRATIVE_INFORMATION_CONTRACT_CONCESSIONAIRE_SUM", "ADMINISTRATIVE_INFORMATION_CONTRACT_CONCESSIONAIRE"),
    ("ADMINISTRATIVE_INFORMATION_CONTRACT_NOTICE_SUM",        "ADMINISTRATIVE_INFORMATION_CONTRACT_NOTICE"),
    ("ADMINISTRATIVE_INFORMATION_CONTRACT_UTILITIES_SUM",     "ADMINISTRATIVE_INFORMATION_CONTRACT_UTILITIES"),
    ("ADMINISTRATIVE_INFORMATION_DEF_SUM",                    "ADMINISTRATIVE_INFORMATION_DEF"),
    ("ADMINISTRATIVE_INFORMATION_DESIGN_CONTEST_NOTICE_SUM",  "ADMINISTRATIVE_INFORMATION_DESIGN_CONTEST_NOTICE"),
    ("ADMINISTRATIVE_INFORMATION_QUALIFICATION_SYSTEM_SUM",   "ADMINISTRATIVE_INFORMATION_QUALIFICATION_SYSTEM"),
    ("ADMINISTRATIVE_INFORMATION_SIMPLIFIED_CONTRACT_SUM",    "ADMINISTRATIVE_INFORMATION_SIMPLIFIED_CONTRACT"),
    ("AI_PROCEDURE_PERIODIC_INDICATIVE_SUM",                  "AI_PROCEDURE_PERIODIC_INDICATIVE"),
    ("ANNEX_I_SUM",                                           "ANNEX_I"),
    ("AUTHORITY_CONCESSION_SUM",                              "AUTHORITY_CONCESSION"),
    ("AUTHORITY_ENTITY_DESIGN_CONTEST_SUM",                   "AUTHORITY_ENTITY_DESIGN_CONTEST"),
    ("AUTHORITY_ENTITY_NOTICE_BUYER_PROFILE_SUM",             "AUTHORITY_ENTITY_NOTICE_BUYER_PROFILE"),
    ("AUTHORITY_ENTITY_SIMPLIFIED_CONTRACT_NOTICE_SUM",       "AUTHORITY_ENTITY_SIMPLIFIED_CONTRACT_NOTICE"),
    ("AUTHORITY_PERIODIC_INDICATIVE_SUM",                     "AUTHORITY_PERIODIC_INDICATIVE"),
    ("AUTHORITY_PRIOR_INFORMATION_SUM",                       "AUTHORITY_PRIOR_INFORMATION"),
    ("AWARD_AND_CONTRACT_VALUE_SUM",                          "AWARD_AND_CONTRACT_VALUE"),
    ("AWARD_CONTRACT_CONTRACT_AWARD_UTILITIES_SUM",           "AWARD_CONTRACT_CONTRACT_AWARD_UTILITIES"),
    ("AWARD_OF_CONTRACT_SUM",                                 "AWARD_OF_CONTRACT"),
    ("AWARD_PRIZES_SUM",                                      "AWARD_PRIZES"),
    ("BUYER_PROFILE_SUM",                                     "BUYER_PROFILE"),
    ("COMPLEMENTARY_INFORMATION_CONTRACT_NOTICE_SUM",         "COMPLEMENTARY_INFORMATION_CONTRACT_NOTICE"),
    ("CONCESSION_SUM",                                        "CONCESSION"),
    ("CONDITIONS_FOR_MORE_INFORMATION_SUM",                   "CONDITIONS_FOR_MORE_INFORMATION"),
    ("CONTACTING_AUTHORITY_INFORMATION_SUM",                  "CONTACTING_AUTHORITY_INFORMATION"),
    ("CONTACTING_AUTHORITY_INFO_SUM",                         "CONTACTING_AUTHORITY_INFO"),
    ("CONTRACTING_AUTHORITY_INFORMATION_SUM",                 "CONTRACTING_AUTHORITY_INFORMATION"),
    ("CONTRACTING_ENTITY_CONTRACT_AWARD_UTILITIES_SUM",       "CONTRACTING_ENTITY_CONTRACT_AWARD_UTILITIES"),
    ("CONTRACTING_ENTITY_QUALIFICATION_SYSTEM_SUM",           "CONTRACTING_ENTITY_QUALIFICATION_SYSTEM"),
    ("CONTRACTING_ENTITY_RESULT_DESIGN_CONTEST_SUM",          "CONTRACTING_ENTITY_RESULT_DESIGN_CONTEST"),
    ("CONTRACT_AWARD_SUM",                                    "CONTRACT_AWARD"),
    ("CONTRACT_AWARD_UTILITIES_SUM",                          "CONTRACT_AWARD_UTILITIES"),
    // The 2008 concession summaries — 7 notices in the corpus, held from the
    // first ingest until issue 194 (their siblings are issue 84/190's protected
    // 154). Mined from the real payloads, same as every entry here.
    ("CONTRACT_CONCESSIONAIRE_SUM",                           "CONTRACT_CONCESSIONAIRE"),
    ("CONTRACT_OBJECT_DESCRIPTION_SUM",                       "CONTRACT_OBJECT_DESCRIPTION"),
    ("CONTRACT_SUM",                                          "CONTRACT"),
    ("CONTRACT_UTILITIES_SUM",                                "CONTRACT_UTILITIES"),
    ("DESCRIPTION_AWARD_NOTICE_INFORMATION_SUM",              "DESCRIPTION_AWARD_NOTICE_INFORMATION"),
    ("DESCRIPTION_CONCESSION_SUM",                            "DESCRIPTION_CONCESSION"),
    ("DESCRIPTION_CONTRACT_AWARD_UTILITIES_SUM",              "DESCRIPTION_CONTRACT_AWARD_UTILITIES"),
    ("DESCRIPTION_CONTRACT_INFORMATION_SUM",                  "DESCRIPTION_CONTRACT_INFORMATION"),
    ("DESIGN_CONTEST_SUM",                                    "DESIGN_CONTEST"),
    ("FD_BUYER_PROFILE_SUM",                                  "FD_BUYER_PROFILE"),
    ("FD_CONCESSION_SUM",                                     "FD_CONCESSION"),
    ("FD_CONTRACT_AWARD_SUM",                                 "FD_CONTRACT_AWARD"),
    ("FD_CONTRACT_AWARD_UTILITIES_SUM",                       "FD_CONTRACT_AWARD_UTILITIES"),
    ("FD_CONTRACT_CONCESSIONAIRE_SUM",                        "FD_CONTRACT_CONCESSIONAIRE"),
    ("FD_CONTRACT_SUM",                                       "FD_CONTRACT"),
    ("FD_CONTRACT_UTILITIES_SUM",                             "FD_CONTRACT_UTILITIES"),
    ("FD_DESIGN_CONTEST_SUM",                                 "FD_DESIGN_CONTEST"),
    ("FD_PERIODIC_INDICATIVE_UTILITIES_SUM",                  "FD_PERIODIC_INDICATIVE_UTILITIES"),
    ("FD_PRIOR_INFORMATION_SUM",                              "FD_PRIOR_INFORMATION"),
    ("FD_QUALIFICATION_SYSTEM_UTILITIES_SUM",                 "FD_QUALIFICATION_SYSTEM_UTILITIES"),
    ("FD_RESULT_DESIGN_CONTEST_SUM",                          "FD_RESULT_DESIGN_CONTEST"),
    ("FD_SIMPLIFIED_CONTRACT_SUM",                            "FD_SIMPLIFIED_CONTRACT"),
    ("INTRODUCTION_PERIODIC_INDICATIVE_SUM",                  "INTRODUCTION_PERIODIC_INDICATIVE"),
    ("OBJECT_CONCESSION_SUM",                                 "OBJECT_CONCESSION"),
    ("OBJECT_CONTRACT_AWARD_UTILITIES_SUM",                   "OBJECT_CONTRACT_AWARD_UTILITIES"),
    ("OBJECT_CONTRACT_NOTICE_DESCRIPTION_SUM",                "OBJECT_CONTRACT_NOTICE_DESCRIPTION"),
    ("OBJECT_CONTRACT_NOTICE_SUM",                            "OBJECT_CONTRACT_NOTICE"),
    ("OBJECT_CONTRACT_INFORMATION_CONTRACT_AWARD_NOTICE_SUM", "OBJECT_CONTRACT_INFORMATION_CONTRACT_AWARD_NOTICE"),
    ("OBJECT_CONTRACT_INFORMATION_CONTRACT_UTILITIES_SUM",    "OBJECT_CONTRACT_INFORMATION_CONTRACT_UTILITIES"),
    ("OBJECT_CONTRACT_INFORMATION_SUM",                       "OBJECT_CONTRACT_INFORMATION"),
    ("OBJECT_CONTRACT_PERIODIC_INDICATIVE_SUM",               "OBJECT_CONTRACT_PERIODIC_INDICATIVE"),
    ("OBJECT_DESIGN_CONTEST_SUM",                             "OBJECT_DESIGN_CONTEST"),
    ("OBJECT_NOTICE_BUYER_PROFILE_SUM",                       "OBJECT_NOTICE_BUYER_PROFILE"),
    ("OBJECT_QUALIFICATION_SYSTEM_SUM",                       "OBJECT_QUALIFICATION_SYSTEM"),
    ("OBJECT_RESULT_DESIGN_CONTEST_SUM",                      "OBJECT_RESULT_DESIGN_CONTEST"),
    ("OBJECT_SIMPLIFIED_CONTRACT_NOTICE_SUM",                 "OBJECT_SIMPLIFIED_CONTRACT_NOTICE"),
    ("OBJECT_SUPPLY_SERVICE_PRIOR_INFORMATION_SUM",           "OBJECT_SUPPLY_SERVICE_PRIOR_INFORMATION"),
    ("OBJECT_WORKS_PRIOR_INFORMATION_SUM",                    "OBJECT_WORKS_PRIOR_INFORMATION"),
    ("PERIODIC_INDICATIVE_UTILITIES_SUM",                     "PERIODIC_INDICATIVE_UTILITIES"),
    ("PRIOR_INFORMATION_SUM",                                 "PRIOR_INFORMATION"),
    ("PROCEDURES_CONCESSION_SUM",                             "PROCEDURES_CONCESSION"),
    ("PROCEDURES_CONTRACT_NOTICE_SUM",                        "PROCEDURES_CONTRACT_NOTICE"),
    ("PROCEDURES_DESIGN_CONTEST_SUM",                         "PROCEDURES_DESIGN_CONTEST"),
    ("PROCEDURES_QUALIFICATION_SYSTEM_SUM",                   "PROCEDURES_QUALIFICATION_SYSTEM"),
    ("PROCEDURES_SIMPLIFIED_CONTRACT_NOTICE_SUM",             "PROCEDURES_SIMPLIFIED_CONTRACT_NOTICE"),
    ("PUBLIC_WORKS_CONCESSIONAIRE_CONTRACT_NOTICE_SUM",       "PUBLIC_WORKS_CONCESSIONAIRE_CONTRACT_NOTICE"),
    ("PROCEDURE_DEFINITION_CONTRACT_NOTICE_SUM",              "PROCEDURE_DEFINITION_CONTRACT_NOTICE"),
    ("PROCEDURE_DEFINITION_CONTRACT_NOTICE_UTILITIES_SUM",    "PROCEDURE_DEFINITION_CONTRACT_NOTICE_UTILITIES"),
    ("QUALIFICATION_SYSTEM_UTILITIES_SUM",                    "QUALIFICATION_SYSTEM_UTILITIES"),
    ("RESULTS_CONTEST_RESULT_DESIGN_CONTEST_SUM",             "RESULTS_CONTEST_RESULT_DESIGN_CONTEST"),
    ("RESULT_CONTEST_SUM",                                    "RESULT_CONTEST"),
    ("RESULT_DESIGN_CONTEST_SUM",                             "RESULT_DESIGN_CONTEST"),
    ("SIMPLIFIED_CONTRACT_SUM",                               "SIMPLIFIED_CONTRACT"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::r209::rules;

    #[test]
    fn alias_normalises_sum_elements_and_passes_others_through() {
        assert_eq!(alias("CONTRACT_SUM"), "CONTRACT");
        assert_eq!(alias("FD_CONTRACT_AWARD_SUM"), "FD_CONTRACT_AWARD");
        // A non-alias name is untouched — no runtime suffix strip.
        assert_eq!(alias("FD_CONTRACT"), "FD_CONTRACT");
        assert_eq!(alias("SECTOR"), "SECTOR");
    }

    /// The checked-in table pins the reuse: every base is a real r209 element,
    /// and each mapping is the mechanical `_SUM` strip — so a divergent or
    /// misspelled entry is a visible failing line, not a silent normalisation.
    #[test]
    fn sum_alias_table_pins_every_base_to_an_r209_element() {
        assert_eq!(SUM_ALIASES.len(), 77, "alias count changed; re-sweep and update the table");
        for &(alias, base) in SUM_ALIASES {
            assert_eq!(
                alias.strip_suffix("_SUM"),
                Some(base),
                "{alias} must map to its suffix-stripped base",
            );
            assert!(
                rules::rule("", base).is_some(),
                "alias base {base} is not an r209 element (drift — needs its own decision)",
            );
        }
        // No duplicate aliases.
        assert_eq!(table().len(), SUM_ALIASES.len(), "duplicate alias key");
    }

    /// The overlay must not shadow the r209 registry except where INTERNAL_OJS
    /// genuinely diverges: the deliberate overrides are exactly `ISO_COUNTRY`,
    /// `SERVICE_CATEGORY` and `SERVICE_CATEGORY_PUB`.
    #[test]
    fn overlay_only_overrides_the_three_documented_traps() {
        let overridden: Vec<&str> = decided_names()
            .filter(|n| rules::rule("", n).is_some())
            .collect();
        assert_eq!(overridden, ["ISO_COUNTRY", "SERVICE_CATEGORY", "SERVICE_CATEGORY_PUB"]);
        for name in &overridden {
            assert!(envelope_rule("", name).is_some());
        }
    }
}
