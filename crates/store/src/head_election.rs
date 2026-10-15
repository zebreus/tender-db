//! Issue 524 unit 2a: the served head's ELECTION — the pure rule [`elect_head`] and the DRY planner
//! [`Db::plan_org_name_refresh`] the `refresh-org-names` job runs.
//!
//! `organizations.name` is written at mint, from the first mention's first-seen spelling, and never
//! again, so merges and mention drops leave heads that few or none of an organization's mentions say.
//! Census job 2492 (issue 524 unit 1) counted 56,144 minority heads and 1,585 absent ones; 31558903 is
//! served as `Sheffield Hallam University` over 16 Sheffield mentions, 22713056 as the pointer `s.o.`.
//! ADR-0013's 2026-10-15 amendment decides the rule. This module is its ONE implementation, so the dry
//! plan, unit 2b's wet re-check and unit 2c's weekly run cannot disagree about a head.
//!
//! The rule needs ingest's key and name functions, and store never depends on ingest, so they are
//! injected ([`HeadElectFns`]), as [`Db::org_head_drift`] takes its keys. Unit 2a writes NOTHING: the
//! planner reads, and the caller stores the plan as a report.

use std::collections::{BTreeMap, HashMap, HashSet};

use serde::Serialize;
use turso::Value;

use crate::canonical::{
    HEAD_DRIFT_AWARD_SUMMARY, HeadDriftCount, IN_CHUNK, ORG_HEAD_DRIFT_ORGS_SQL, all_caps, append_change,
    org_head_drift_merged_sql, org_head_drift_mentions_sql, org_name_norm, placeholders,
};
use crate::checkpoint::{CheckpointMode, checkpoint_on};
use crate::{Db, int, opt_int_of, opt_text_of, text};

/// Issue 524 unit 2a: the election rule's epoch, stamped on every plan. Bump it whenever
/// [`elect_head`] or an injected function it is run with changes what it decides, so unit 2b's wet
/// path refuses a plan another rule wrote. `v2`: the 2a review's ALL-CAPS test reads `ß`, `ª` and
/// `º` as caseless (`all_caps`), which reorders display spellings. `v3` (unit 2a.1): the vote key
/// folds Latin diacritics, the display order prefers the diacritic-bearing spelling, and R2 holds a
/// shorthand of the head ([`HeadElectFns::r2_hold`]). `v4` (unit 2a.2): the marked-letter preference
/// applies between true twins only ([`HeadElectFns::twin_key`]), the family hold reads folded names,
/// a `mojibake` spelling is unfit, and R2 holds a tightened initialism and no head cut short. `v5`
/// (unit 2a.3): the twin swap replaces only a winner with no marked letter, the family hold joins
/// dotted forms, mojibake knows `€`, `™`, `È™`/`È›` and a word-final `Ã`, and the initialism holds a
/// bare initialism, skips particles and knows the review's designations.
pub const HEAD_ELECTION_EPOCH: &str = "524u2-v5";

/// Issue 524 unit 2: the fewest votes a cluster must hold to replace a real name (R1 and R2) — no
/// cluster of one or two mentions ever moves a head.
pub const HEAD_ELECT_TOP_MIN: u64 = 3;

/// Issue 524 unit 2a: the most `planned` (and, apart, `held`) entries a plan lists; past it the list is
/// cut and `truncated` (`held_truncated`) is set, and unit 2b refuses a truncated plan. The counts are
/// never capped.
pub const ORG_NAME_REFRESH_LISTING_CAP: usize = 100_000;

/// Issue 524 unit 2a: the voting spellings an entry lists, most mentions first.
pub const ORG_NAME_REFRESH_SPELLINGS: usize = 5;

/// Issue 524 unit 2: the hold reason when the winning cluster has no FIT spelling to serve.
pub const HEAD_HOLD_NO_FIT: &str = "no_fit";

/// Issue 524 unit 2: the rule a head CHANGE (or a held one) is elected by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HeadRule {
    /// R0, the head is empty after trim (`not_a_name("")` is `None`, so this is its own test).
    R0e,
    /// R0, the head is a non-name: a `not_a_name` class other than `award_summary` (placeholder,
    /// blank, void_lot).
    R0n,
    /// A cluster other than the head's holds at least 55% of the votes, at least
    /// [`HEAD_ELECT_TOP_MIN`] and at least twice the head's.
    R1,
    /// The head's cluster holds at most one vote, and a unique top cluster at least
    /// [`HEAD_ELECT_TOP_MIN`] and at least 25%.
    R2,
}

impl HeadRule {
    pub const fn label(self) -> &'static str {
        match self {
            HeadRule::R0e => "R0e",
            HeadRule::R0n => "R0n",
            HeadRule::R1 => "R1",
            HeadRule::R2 => "R2",
        }
    }
}

/// Issue 524 unit 2: why a head is KEPT byte for byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HeadKeep {
    /// `identifier IS NULL`: `name_norm` is the issue-234 bind key there, on every resolver lookup.
    E1,
    /// The org's triple carries a `wrong` identifier verdict, or a `related` one not yet applied (the
    /// `ORG_COLUMNS` predicate): the bind is the defect (issue 456 design 1), not the head.
    E2,
    /// No mention votes.
    E3,
    /// ADR-0013 D4's ENG leg: the head's dominant script differs from the elected spelling's (TED's
    /// Latin rendering over Cyrillic or Greek mentions, 7912592 `Obshtina Belovo`).
    D4,
    /// The head's cluster is the top cluster or tied for it.
    S1,
    /// Neither R1 nor R2 clears its bar.
    Bar,
}

impl HeadKeep {
    pub const fn label(self) -> &'static str {
        match self {
            HeadKeep::E1 => "E1",
            HeadKeep::E2 => "E2",
            HeadKeep::E3 => "E3",
            HeadKeep::D4 => "D4",
            HeadKeep::S1 => "S1",
            HeadKeep::Bar => "bar",
        }
    }
}

/// Issue 524 unit 2: what [`elect_head`] decides for one organization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeadVerdict {
    Keep(HeadKeep),
    /// A change the rule elected but does not apply: [`HEAD_HOLD_NO_FIT`] (the winning cluster has no
    /// fit spelling, so `new` is `None`), a bind guard's reason (the injected [`HeadElectFns::hold`]:
    /// `family`, `gb_family`, `consortium_removed`, `nhs_check`), or, for R2 only, a shorthand's
    /// ([`HeadElectFns::r2_hold`]: `initialism`), with the spelling it would have
    /// served.
    Hold { reason: &'static str, rule: HeadRule, new: Option<String> },
    Change { rule: HeadRule, new: String },
}

impl HeadVerdict {
    /// `keep` | `hold` | `change`.
    pub fn kind(&self) -> &'static str {
        match self {
            HeadVerdict::Keep(_) => "keep",
            HeadVerdict::Hold { .. } => "hold",
            HeadVerdict::Change { .. } => "change",
        }
    }

    /// The keep reason, the hold reason, or the change's rule.
    pub fn reason(&self) -> &'static str {
        match self {
            HeadVerdict::Keep(keep) => keep.label(),
            HeadVerdict::Hold { reason, .. } => reason,
            HeadVerdict::Change { rule, .. } => rule.label(),
        }
    }

    /// `kind:reason` (`change:R1`, `keep:S1`, `hold:no_fit`): the plan's sample and canary key.
    pub fn label(&self) -> String {
        format!("{}:{}", self.kind(), self.reason())
    }

    /// The rule of a change or a hold.
    pub fn rule(&self) -> Option<HeadRule> {
        match self {
            HeadVerdict::Keep(_) => None,
            HeadVerdict::Hold { rule, .. } | HeadVerdict::Change { rule, .. } => Some(*rule),
        }
    }

    /// The spelling a change serves, or a bind-held change would have.
    pub fn new_name(&self) -> Option<&str> {
        match self {
            HeadVerdict::Keep(_) => None,
            HeadVerdict::Hold { new, .. } => new.as_deref(),
            HeadVerdict::Change { new, .. } => Some(new),
        }
    }
}

/// Issue 524 unit 2: the ingest functions [`elect_head`] runs with (store never depends on ingest). The
/// app passes `crosswalk::head_vote_key`, `partyname::not_a_name` as its class label,
/// `crosswalk::head_spelling_unfit`, `crosswalk::head_change_hold` and `crosswalk::head_r2_hold`.
#[derive(Clone, Copy)]
pub struct HeadElectFns {
    /// The key mentions vote under: `altid_name_key` of the diacritic-folded name, with every
    /// legal-form token dropped.
    pub vote_key: fn(&str) -> String,
    /// A name's non-name class (`void_lot` | `placeholder` | `blank` | [`HEAD_DRIFT_AWARD_SUMMARY`]) or
    /// `None`. A mention of any class but `award_summary` does not vote; a head of one is R0's.
    pub not_a_name: fn(&str) -> Option<&'static str>,
    /// `(candidate, head)`: why `candidate` may not be served in place of `head`, or `None` when fit.
    pub unfit: fn(&str, &str) -> Option<&'static str>,
    /// `(old, new, identifier_kind, identifier)`: why a change is held for review, or `None`.
    pub hold: fn(&str, &str, Option<&str>, Option<&str>) -> Option<&'static str>,
    /// Issue 524 unit 2a.1, `(old, new)`: why an R2 change is held as a shorthand of the head it would
    /// replace (since unit 2a.2, an initialism of a run of its words), or `None`. Asked of R2 changes
    /// only, after [`HeadElectFns::hold`]: R1's majority is not second-guessed.
    pub r2_hold: fn(&str, &str) -> Option<&'static str>,
    /// Issue 524 unit 2a.2: the key two spellings are TWINS under — lower-cased, diacritics folded,
    /// every non-alphanumeric character dropped (`crosswalk::head_twin_key`). The display order
    /// prefers the twin with more marked letters, and only among twins.
    pub twin_key: fn(&str) -> String,
}

/// Issue 524 unit 2: the organization [`elect_head`] decides for.
#[derive(Debug, Clone, Copy)]
pub struct HeadElectOrg<'a> {
    /// The served head, `organizations.name`.
    pub head: &'a str,
    pub identifier_kind: Option<&'a str>,
    pub identifier: Option<&'a str>,
    /// E2: the org's `(identifier, identifier_kind, country)` carries a `wrong` verdict or an unapplied
    /// `related` one. The dry planner probes it for the candidates only (orgs the rule would change or
    /// hold), so it passes `false` first and re-elects a candidate the probe finds.
    pub bind_defect: bool,
}

/// Issue 524 unit 2: [`elect_head`]'s verdict with the counts it was read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadElection {
    pub verdict: HeadVerdict,
    /// V: the voting mentions.
    pub voting: u64,
    /// T: the top cluster's votes.
    pub top: u64,
    /// H: the votes of the cluster holding the head's vote key, 0 when none does.
    pub head_votes: u64,
    /// The largest cluster beside the top one: `top` again on a tie.
    pub second: u64,
    /// Distinct vote keys among the voters.
    pub clusters: u64,
}

/// One vote key's mentions and its spellings.
struct Cluster<'a> {
    votes: u64,
    spellings: BTreeMap<&'a str, u64>,
}

/// Issue 524 unit 2: the ELECTION RULE — whether an organization's served head stays, changes, or is
/// held, from its mentions' spellings. Pure: the dry planner, the wet re-check and the weekly run all
/// call it.
///
/// `votes` is every mention spelling with its count; this decides who votes, so every caller counts
/// alike. A mention votes unless its name is blank or a `not_a_name` class other than
/// `award_summary` (the census's voters). Voters group into clusters by [`HeadElectFns::vote_key`];
/// V is all votes, T the top cluster's, H the head's cluster's (0 if none). The steps run in order
/// and the first that decides wins:
///
/// 1. **E1** `identifier IS NULL`, **E2** a bind-defect verdict, **E3** no voter: KEEP.
/// 2. **R0**, a defective head — empty after trim, or a `not_a_name` class other than
///    `award_summary`: elect the top cluster; on a tie, one with a fit not-ALL-CAPS spelling first,
///    then the one with more fit mentions, then the smallest vote key. It skips D4.
/// 3. **D4**: the head's dominant script (Latin, Cyrillic, Greek, other) differs from the elected
///    cluster's — its leading spelling, all of whose spellings share its key's letters: KEEP. A name
///    with no letter has no script and never trips it.
/// 4. **S1**: the head's cluster is the top cluster or tied for it (H = T): KEEP byte for byte.
/// 5. **R1**: T × 20 ≥ V × 11 (55%), T ≥ [`HEAD_ELECT_TOP_MIN`], T ≥ 2H, and the top is unique.
/// 6. **R2**: H ≤ 1, the top is unique, T ≥ [`HEAD_ELECT_TOP_MIN`], and T × 4 ≥ V (25%).
/// 7. Otherwise KEEP (`bar`).
///
/// The elected cluster serves its DISPLAY spelling: among the fit spellings only
/// ([`HeadElectFns::unfit`]), the winner is not-ALL-CAPS before ALL-CAPS, then the most mentions, then
/// the byte-wise smallest — never the census's count-first `top_spelling`, which serves `UNIVERSITY OF
/// SHEFFIELD`. Then, when the winner carries no marked (non-ASCII) letter — an accent-stripped spelling
/// — the most-mentioned fit spelling of its case that is its TWIN ([`HeadElectFns::twin_key`]: equal
/// once lower-cased, folded and stripped of punctuation) and does carry one is served instead, then
/// bytes: `Dotación y Equipamiento, S. L.` over `Dotacion y Equipamiento S.L.`. A winner that has an
/// accent already is kept (unit 2a.3): its one-mention typo twin never beats it (`sociálných` 1 over
/// `sociálnych` 717 on 1526605, `Zakłąd` over `Zakład`). A cluster is wider than twins — its vote
/// key also drops a trading-as clause, a parenthetical, a legal form, `the` and `&` — so `Acme Ltd t/a
/// Café Rouge` (1) never beats `Acme Ltd` (20) on its accent (the 2a.1 review). No fit spelling is a
/// HOLD ([`HEAD_HOLD_NO_FIT`]); a change the bind guards ([`HeadElectFns::hold`]) name, or an R2 change
/// [`HeadElectFns::r2_hold`] reads as an initialism of the head, is a HOLD with the spelling it would
/// serve.
pub fn elect_head(votes: &[(&str, u64)], org: &HeadElectOrg<'_>, fns: &HeadElectFns) -> HeadVerdict {
    elect_head_counted(votes, org, fns).verdict
}

/// [`elect_head`] with the counts its verdict was read from (the plan lists them).
pub fn elect_head_counted(votes: &[(&str, u64)], org: &HeadElectOrg<'_>, fns: &HeadElectFns) -> HeadElection {
    let mut clusters: BTreeMap<String, Cluster<'_>> = BTreeMap::new();
    for &(spelling, count) in votes {
        if count == 0 || spelling.trim().is_empty() {
            continue;
        }
        if (fns.not_a_name)(spelling).is_some_and(|class| class != HEAD_DRIFT_AWARD_SUMMARY) {
            continue;
        }
        let cluster =
            clusters.entry((fns.vote_key)(spelling)).or_insert_with(|| Cluster { votes: 0, spellings: BTreeMap::new() });
        cluster.votes += count;
        *cluster.spellings.entry(spelling).or_default() += count;
    }
    let voting: u64 = clusters.values().map(|c| c.votes).sum();
    let top = clusters.values().map(|c| c.votes).max().unwrap_or(0);
    // In key order, so the first is the smallest vote key at the top count.
    let tied: Vec<&str> = clusters.iter().filter(|(_, c)| c.votes == top).map(|(k, _)| k.as_str()).collect();
    let unique = tied.len() == 1;
    let head_key = (fns.vote_key)(org.head);
    let head_votes = clusters.get(&head_key).map_or(0, |c| c.votes);
    let second = if unique { clusters.values().map(|c| c.votes).filter(|&v| v < top).max().unwrap_or(0) } else { top };
    let counted = |verdict| HeadElection { verdict, voting, top, head_votes, second, clusters: clusters.len() as u64 };

    if org.identifier.is_none() {
        return counted(HeadVerdict::Keep(HeadKeep::E1));
    }
    if org.bind_defect {
        return counted(HeadVerdict::Keep(HeadKeep::E2));
    }
    if clusters.is_empty() {
        return counted(HeadVerdict::Keep(HeadKeep::E3));
    }
    let serve = |rule: HeadRule, cluster: &Cluster<'_>| match display_spelling(cluster, org.head, fns) {
        None => HeadVerdict::Hold { reason: HEAD_HOLD_NO_FIT, rule, new: None },
        Some(new) => {
            let held = (fns.hold)(org.head, new, org.identifier_kind, org.identifier)
                .or_else(|| if rule == HeadRule::R2 { (fns.r2_hold)(org.head, new) } else { None });
            match held {
                Some(reason) => HeadVerdict::Hold { reason, rule, new: Some(new.to_owned()) },
                None => HeadVerdict::Change { rule, new: new.to_owned() },
            }
        }
    };

    // R0: a defective head. `not_a_name("")` is None, so the empty head is tested on its own.
    let empty = org.head.trim().is_empty();
    if empty || (fns.not_a_name)(org.head).is_some_and(|class| class != HEAD_DRIFT_AWARD_SUMMARY) {
        let rule = if empty { HeadRule::R0e } else { HeadRule::R0n };
        let rank = |cluster: &Cluster<'_>| {
            let fit: Vec<(&str, u64)> = cluster
                .spellings
                .iter()
                .filter(|(spelling, _)| (fns.unfit)(spelling, org.head).is_none())
                .map(|(spelling, n)| (*spelling, *n))
                .collect();
            (fit.iter().any(|(spelling, _)| !all_caps(spelling)), fit.iter().map(|(_, n)| n).sum::<u64>())
        };
        // `max_by` keeps the LAST of equals, so the key order is reversed to make the smallest win.
        let elected = tied
            .iter()
            .map(|key| (*key, &clusters[*key]))
            .max_by(|a, b| rank(a.1).cmp(&rank(b.1)).then_with(|| b.0.cmp(a.0)))
            .map(|(_, cluster)| cluster)
            .expect("a voter makes a top cluster");
        return counted(serve(rule, elected));
    }

    // The cluster the rest of the rule reads: the head's own when it holds the top count (S1's case),
    // else the top cluster, the smallest vote key on a tie (R1 and R2 need it unique anyway).
    let elected = if head_votes == top { &clusters[&head_key] } else { &clusters[tied[0]] };
    let leading = elected
        .spellings
        .iter()
        .max_by(|a, b| spelling_order((*a.0, *a.1), (*b.0, *b.1)))
        .map(|(spelling, _)| *spelling)
        .unwrap_or_default();
    if let (Some(head_script), Some(elected_script)) = (dominant_script(org.head), dominant_script(leading))
        && head_script != elected_script
    {
        return counted(HeadVerdict::Keep(HeadKeep::D4));
    }
    if head_votes == top {
        return counted(HeadVerdict::Keep(HeadKeep::S1));
    }
    if unique && top * 20 >= voting * 11 && top >= HEAD_ELECT_TOP_MIN && top >= 2 * head_votes {
        return counted(serve(HeadRule::R1, elected));
    }
    if head_votes <= 1 && unique && top >= HEAD_ELECT_TOP_MIN && top * 4 >= voting {
        return counted(serve(HeadRule::R2, elected));
    }
    counted(HeadVerdict::Keep(HeadKeep::Bar))
}

/// The display order of two spellings of one cluster for `max_by`: not-ALL-CAPS before ALL-CAPS, then
/// the most mentions, then the byte-wise smallest.
fn spelling_order(a: (&str, u64), b: (&str, u64)) -> std::cmp::Ordering {
    all_caps(b.0).cmp(&all_caps(a.0)).then(a.1.cmp(&b.1)).then_with(|| b.0.cmp(a.0))
}

/// Marked letters: alphabetic and not ASCII (`ă`, `ó`, `ł`; Cyrillic and Greek count too, but a
/// cluster's twins share a script).
fn marked_letters(spelling: &str) -> usize {
    spelling.chars().filter(|c| c.is_alphabetic() && !c.is_ascii()).count()
}

/// The cluster's display spelling, or `None` when no spelling is fit: the best FIT spelling in
/// [`spelling_order`]; when that winner has no marked letter, the most-mentioned (then byte-wise
/// smallest) fit twin of its case class under [`HeadElectFns::twin_key`] that has one.
fn display_spelling<'a>(cluster: &Cluster<'a>, head: &str, fns: &HeadElectFns) -> Option<&'a str> {
    let fit: Vec<(&'a str, u64)> = cluster
        .spellings
        .iter()
        .filter(|(spelling, _)| (fns.unfit)(spelling, head).is_none())
        .map(|(spelling, n)| (*spelling, *n))
        .collect();
    let winner = fit.iter().copied().max_by(|a, b| spelling_order(*a, *b))?;
    if marked_letters(winner.0) > 0 {
        return Some(winner.0);
    }
    let (case, twin) = (all_caps(winner.0), (fns.twin_key)(winner.0));
    let marked_twin = fit
        .into_iter()
        .filter(|(spelling, _)| {
            marked_letters(spelling) > 0 && all_caps(spelling) == case && (fns.twin_key)(spelling) == twin
        })
        .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(a.0)));
    Some(marked_twin.map_or(winner.0, |(spelling, _)| spelling))
}

/// D4's scripts. Only a difference between two of them keeps a head.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Script {
    Latin,
    Cyrillic,
    Greek,
    Other,
}

/// The script most of `name`'s letters are written in — Latin, Cyrillic, Greek or other, the first of
/// those on a tie — or `None` for a name with no letter.
fn dominant_script(name: &str) -> Option<Script> {
    let mut counts = [0u32; 4];
    for c in name.chars().filter(|c| c.is_alphabetic()) {
        let at = match u32::from(c) {
            // Basic Latin, the Latin-1 letters, Latin Extended-A/B, IPA, Latin Extended Additional,
            // Extended-C/D/E and the fullwidth forms.
            0x41..=0x5A | 0x61..=0x7A | 0xAA | 0xBA | 0xC0..=0x2AF | 0x1E00..=0x1EFF | 0x2C60..=0x2C7F
            | 0xA720..=0xA7FF | 0xAB30..=0xAB6F | 0xFB00..=0xFB06 | 0xFF21..=0xFF3A | 0xFF41..=0xFF5A => 0,
            0x400..=0x52F | 0x1C80..=0x1C8F | 0x2DE0..=0x2DFF | 0xA640..=0xA69F => 1,
            0x370..=0x3FF | 0x1F00..=0x1FFF => 2,
            _ => 3,
        };
        counts[at] += 1;
    }
    let most = counts.iter().copied().max().filter(|&n| n > 0)?;
    let at = counts.iter().position(|&n| n == most)?;
    Some([Script::Latin, Script::Cyrillic, Script::Greek, Script::Other][at])
}

// ---- The dry planner.

/// Issue 524 unit 2a: one rule-chosen candidate's identifier-verdict probe — the `ORG_COLUMNS`
/// predicate (read.rs) on the `org_identifier_verdicts` primary key.
pub(crate) const HEAD_ELECT_VERDICT_PROBE_SQL: &str = "SELECT 1 FROM org_identifier_verdicts \
     WHERE identifier = ? AND identifier_kind = ? AND country = ? \
       AND (verdict = 'wrong' OR (verdict = 'related' AND applied_at IS NULL)) LIMIT 1";

/// Issue 524 unit 2a: a D4-kept org's ENG satellite, a seek on the `organization_names` primary key
/// `(org_id, lang)`.
pub(crate) const HEAD_ELECT_ENG_SQL: &str = "SELECT name FROM organization_names WHERE org_id = ? AND lang = 'ENG'";

/// Issue 524 unit 2a: a scoped plan's page — [`ORG_HEAD_DRIFT_ORGS_SQL`]'s columns, in its order, for
/// `n` named ids.
pub(crate) fn org_name_refresh_ids_sql(n: usize) -> String {
    format!(
        "SELECT id, country, identifier, provisional, name, identifier_kind, name_norm FROM organizations \
         WHERE id IN ({}) ORDER BY id",
        placeholders(n)
    )
}

/// Issue 524 unit 2a: [`Db::plan_org_name_refresh`]'s inputs.
pub struct OrgNameRefreshArgs<'a> {
    pub fns: HeadElectFns,
    /// A head's legal-form family, for the entries' `family_old` / `family_new` and the plan's
    /// `family_moves` (the app passes `crosswalk::head_family`).
    pub family: fn(&str) -> Option<&'static str>,
    /// The plan elects organizations with at least this many mention rows (the census population);
    /// fewer is `below_min`.
    pub min_mentions: u64,
    /// The uniform samples listed per rule and per reason.
    pub samples: usize,
    /// Organizations per page of the id walk: `ORG_HEAD_DRIFT_PAGE` in the job.
    pub page: i64,
    /// The most `planned` and `held` entries listed: [`ORG_NAME_REFRESH_LISTING_CAP`] in the job.
    pub listing_cap: usize,
    /// Plan only these organizations (deduplicated; read by id in [`IN_CHUNK`] chunks) instead of
    /// walking every one.
    pub org_ids: Option<&'a [i64]>,
    /// Organizations reported in `canaries` whatever their verdict, below the minimum included.
    pub canaries: &'a [i64],
    pub stop: &'a (dyn Fn() -> bool + Sync),
    pub progress: &'a (dyn Fn(u64, &str) + Sync),
}

/// Issue 524 unit 2a: one organization in the plan — a planned change, a held one, a sample or a
/// canary. A planned entry is unit 2b's input: `old_name` / `old_name_norm` are the compare-and-set
/// pre-image, `new_name` / `new_name_norm` what it writes.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct HeadRefreshEntry {
    pub org_id: i64,
    /// `keep` | `hold` | `change`.
    pub verdict: &'static str,
    /// The keep reason (`E1` `E2` `E3` `D4` `S1` `bar`), the hold reason, or the change's rule (`R0e`
    /// `R0n` `R1` `R2`).
    pub reason: &'static str,
    /// The rule of a change or a hold.
    pub rule: Option<&'static str>,
    pub old_name: String,
    pub old_name_norm: Option<String>,
    /// What a change serves, or a bind-held change would have; `None` for a keep and a no-fit hold.
    pub new_name: Option<String>,
    /// [`org_name_norm`] of `new_name`: `name_norm` follows the head (ADR-0013's amendment, item 7).
    pub new_name_norm: Option<String>,
    /// V, T, H, the second cluster and the distinct clusters ([`HeadElection`]).
    pub voting: u64,
    pub top: u64,
    pub head: u64,
    pub second: u64,
    pub clusters: u64,
    /// Mention rows, nameless and non-name ones included.
    pub n: u64,
    /// A merge survivor: an `organization_merged_identifiers` row with `loser <> org_id`, as the census
    /// reads it.
    pub merged: bool,
    pub family_old: Option<&'static str>,
    pub family_new: Option<&'static str>,
    pub country: Option<String>,
    pub identifier: Option<String>,
    pub provisional: bool,
    /// A D4 keep's ENG satellite against the head: `equal` (byte for byte), `agrees` (the same vote
    /// key), `differs`, or `absent` (no ENG row).
    pub eng: Option<&'static str>,
    /// In the plan's population (`min_mentions` or more). Only a canary can read `false`; its verdict
    /// is computed all the same and counted nowhere.
    pub in_population: bool,
    /// The top [`ORG_NAME_REFRESH_SPELLINGS`] voting spellings, most mentions first, then byte order.
    pub spellings: Vec<(String, u64)>,
}

/// Issue 524 unit 2a: [`Db::plan_org_name_refresh`]'s answer — what `refresh-org-names` WOULD do.
/// Read-only; the job stores it as the `org-name-refresh-plan` report.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct OrgNameRefreshPlan {
    /// [`HEAD_ELECTION_EPOCH`].
    pub epoch: &'static str,
    pub min_mentions: u64,
    pub samples_per_reason: usize,
    pub listing_cap: usize,
    /// The distinct `org_ids` a scoped plan was asked for; `None` for the whole walk.
    pub scope_ids: Option<u64>,
    pub orgs_walked: u64,
    pub mentions_walked: u64,
    /// Organizations under `min_mentions` mention rows, and those rows.
    pub below_min: HeadDriftCount,
    /// The elected organizations (`min_mentions` or more mention rows) and their mention rows: the
    /// denominators of everything below.
    pub population: HeadDriftCount,
    /// Elected organizations whose head is empty after trim (R0e's class).
    pub empty_heads: HeadDriftCount,
    pub changes: HeadDriftCount,
    pub holds: HeadDriftCount,
    pub keeps: HeadDriftCount,
    /// Changes by rule; keeps by reason; holds by reason. E2 counts only the orgs the rule would
    /// otherwise change or hold — the verdict probe reads the candidates alone.
    pub by_rule: BTreeMap<&'static str, HeadDriftCount>,
    pub by_keep: BTreeMap<&'static str, HeadDriftCount>,
    pub by_hold: BTreeMap<&'static str, HeadDriftCount>,
    /// The planned changes' mention rows — the upper bound of the tender reach a wet run's joined
    /// `parties[].organization_name` would show (no tender row is written).
    pub planned_mentions: u64,
    /// The planned changes on merge survivors.
    pub planned_merged: HeadDriftCount,
    /// The planned changes by legal-form family move ([`OrgNameRefreshArgs::family`] of the old and the
    /// new head): `none->none`, `none->x` (a family gained), `x->none` (one lost), `x->x` (the same
    /// family), `x->y` (two different families). The hold fn holds an `x->y` it reads within ONE table
    /// (`legal_form_family` or `gb_legal_family`); a pair read from different tables (`sarl` -> `ltd`) is
    /// not held, so it is counted here for the reviewer.
    pub family_moves: BTreeMap<&'static str, u64>,
    /// The D4 keeps' ENG satellites (see [`HeadRefreshEntry::eng`]): whether TED's ENG rendering is the
    /// head the guard keeps (issue 524's open risk 3).
    pub d4_eng: BTreeMap<&'static str, u64>,
    /// Identifier-verdict probes run (one per candidate).
    pub verdict_probes: u64,
    /// The planned changes, by org id, the first `listing_cap`.
    pub planned: Vec<HeadRefreshEntry>,
    pub truncated: bool,
    /// The held changes, by org id, the first `listing_cap`.
    pub held: Vec<HeadRefreshEntry>,
    pub held_truncated: bool,
    /// Per `verdict:reason` (`change:R1`, `keep:S1`, `hold:no_fit`), the `samples_per_reason` orgs with
    /// the smallest `(org_id × 2654435761) mod 2³²` — uniform over the class, not its oldest ids.
    pub samples: BTreeMap<String, Vec<HeadRefreshEntry>>,
    /// The canaries the walk passed, in walk order; a missing id yields no row.
    pub canaries: Vec<HeadRefreshEntry>,
    pub stopped: bool,
}

/// The uniform sample's order: Knuth's multiplicative hash of the id, mod 2³², then the id.
fn sample_key(org_id: i64) -> (u64, i64) {
    ((org_id as u64).wrapping_mul(2_654_435_761) & 0xFFFF_FFFF, org_id)
}

/// The `cap` smallest-keyed entries offered, kept sorted.
#[derive(Default)]
struct Uniform {
    items: Vec<((u64, i64), HeadRefreshEntry)>,
}

impl Uniform {
    fn admits(&self, cap: usize, key: (u64, i64)) -> bool {
        self.items.len() < cap || self.items.last().is_some_and(|(last, _)| key < *last)
    }

    fn insert(&mut self, cap: usize, key: (u64, i64), entry: HeadRefreshEntry) {
        let at = self.items.partition_point(|(k, _)| *k < key);
        self.items.insert(at, (key, entry));
        self.items.truncate(cap);
    }
}

/// One organization of a page, as the planner reads it.
struct PageOrg {
    id: i64,
    country: Option<String>,
    identifier: Option<String>,
    provisional: bool,
    head: String,
    identifier_kind: Option<String>,
    name_norm: Option<String>,
    /// Mention rows, nameless ones included.
    n: u64,
    /// Named mentions per exact spelling.
    spellings: HashMap<String, u64>,
}

impl PageOrg {
    fn votes(&self) -> Vec<(&str, u64)> {
        self.spellings.iter().map(|(spelling, n)| (spelling.as_str(), *n)).collect()
    }

    fn elect(&self, fns: &HeadElectFns, bind_defect: bool) -> HeadElection {
        let org = HeadElectOrg {
            head: &self.head,
            identifier_kind: self.identifier_kind.as_deref(),
            identifier: self.identifier.as_deref(),
            bind_defect,
        };
        elect_head_counted(&self.votes(), &org, fns)
    }
}

/// One page org's election and what the candidate reads added.
struct Judged {
    at: usize,
    election: HeadElection,
    in_population: bool,
    merged: bool,
    eng: Option<&'static str>,
}

impl Judged {
    fn candidate(&self) -> bool {
        matches!(self.election.verdict, HeadVerdict::Change { .. } | HeadVerdict::Hold { .. })
    }
}

impl Db {
    /// Issue 524 unit 2a: the DRY `refresh-org-names` plan — [`elect_head`] over every organization with
    /// at least `min_mentions` mention rows, and what a wet run (unit 2b) would change, hold and keep.
    ///
    /// It walks exactly the census's pages ([`Db::org_head_drift`]): `organizations` by id, `args.page`
    /// rows at a time ([`ORG_HEAD_DRIFT_ORGS_SQL`]), each page's mention names through
    /// `organization_mentions_org` in [`IN_CHUNK`] id chunks, and it refuses before reading while
    /// that index or `organization_merged_identifiers_org` is absent (a rebuild's Phase 1). With
    /// `org_ids` it reads those rows by id instead. For the CANDIDATES only — orgs the rule would
    /// change or hold — it adds three reads: the identifier-verdict probe (E2, the `ORG_COLUMNS`
    /// predicate on the verdicts' primary key), the merge-survivor read the census makes, and, for the
    /// D4 keeps, the `organization_names` ENG seek. The stop flag is read before every page; a stopped
    /// walk returns an empty plan marked `stopped`.
    ///
    /// This writes nothing: no `organizations` row, no ledger, no change row. The caller stores the
    /// plan as a report.
    pub async fn plan_org_name_refresh(&self, args: OrgNameRefreshArgs<'_>) -> turso::Result<OrgNameRefreshPlan> {
        for index in ["organization_mentions_org", "organization_merged_identifiers_org"] {
            if !self.has_index(index).await? {
                return Err(turso::Error::Error(format!(
                    "refresh-org-names refused: {index} is absent (a rebuild's Phase 1?), so every chunk would \
                     scan its table; run `reindex` first. Nothing was read"
                )));
            }
        }
        let scope: Option<Vec<i64>> = args.org_ids.map(|ids| {
            let mut ids = ids.to_vec();
            ids.sort_unstable();
            ids.dedup();
            ids
        });
        let mut plan = OrgNameRefreshPlan {
            epoch: HEAD_ELECTION_EPOCH,
            min_mentions: args.min_mentions,
            samples_per_reason: args.samples,
            listing_cap: args.listing_cap,
            scope_ids: scope.as_ref().map(|ids| ids.len() as u64),
            ..Default::default()
        };
        let canaries: HashSet<i64> = args.canaries.iter().copied().collect();
        // By (verdict kind, reason); keyed `kind:reason` once the walk is done.
        let mut samples: BTreeMap<(&'static str, &'static str), Uniform> = BTreeMap::new();
        let reader = self.reader().await?;
        let mut after = 0i64;
        let mut next = 0usize;
        loop {
            if (args.stop)() {
                return Ok(OrgNameRefreshPlan { stopped: true, ..Default::default() });
            }
            let mut page: Vec<PageOrg> = Vec::new();
            let mut rows = match &scope {
                Some(ids) => {
                    if next >= ids.len() {
                        break;
                    }
                    let chunk = &ids[next..ids.len().min(next + IN_CHUNK)];
                    next += chunk.len();
                    let params: Vec<Value> = chunk.iter().map(|&id| Value::Integer(id)).collect();
                    reader.query(&org_name_refresh_ids_sql(chunk.len()), params).await?
                }
                None => {
                    reader
                        .query(ORG_HEAD_DRIFT_ORGS_SQL, (Value::Integer(after), Value::Integer(args.page.max(1))))
                        .await?
                }
            };
            while let Some(row) = rows.next().await? {
                page.push(PageOrg {
                    id: int(&row, 0),
                    country: opt_text_of(&row, 1),
                    identifier: opt_text_of(&row, 2),
                    provisional: int(&row, 3) != 0,
                    head: text(&row, 4),
                    identifier_kind: opt_text_of(&row, 5),
                    name_norm: opt_text_of(&row, 6),
                    n: 0,
                    spellings: HashMap::new(),
                });
            }
            drop(rows);
            match page.last() {
                Some(last) => after = last.id,
                // A scoped chunk whose ids are all gone reads nothing; the walk goes on.
                None if scope.is_some() => continue,
                None => break,
            }
            plan.orgs_walked += page.len() as u64;
            let at: HashMap<i64, usize> = page.iter().enumerate().map(|(i, org)| (org.id, i)).collect();

            // The page's mention names, as the census reads them.
            let ids: Vec<i64> = page.iter().map(|org| org.id).collect();
            for chunk in ids.chunks(IN_CHUNK) {
                let params: Vec<Value> = chunk.iter().map(|&id| Value::Integer(id)).collect();
                let mut rows = reader.query(&org_head_drift_mentions_sql(chunk.len()), params).await?;
                while let Some(row) = rows.next().await? {
                    let Some(&i) = at.get(&int(&row, 0)) else { continue };
                    plan.mentions_walked += 1;
                    let org = &mut page[i];
                    org.n += 1;
                    if let Some(name) = opt_text_of(&row, 1).filter(|name| !name.trim().is_empty()) {
                        *org.spellings.entry(name).or_default() += 1;
                    }
                }
            }

            // The election, before any candidate read.
            let mut judged: Vec<Judged> = Vec::new();
            for (i, org) in page.iter().enumerate() {
                let below = org.n < args.min_mentions;
                if below {
                    plan.below_min.add(org.n);
                    if !canaries.contains(&org.id) {
                        continue;
                    }
                }
                let election = org.elect(&args.fns, false);
                judged.push(Judged { at: i, election, in_population: !below, merged: false, eng: None });
            }

            // E2, for the candidates only: a bind-defect verdict on the triple keeps the head.
            for j in judged.iter_mut().filter(|j| j.candidate()) {
                let org = &page[j.at];
                let Some(identifier) = org.identifier.as_deref() else { continue };
                plan.verdict_probes += 1;
                let mut rows = reader
                    .query(
                        HEAD_ELECT_VERDICT_PROBE_SQL,
                        (
                            Value::Text(identifier.to_owned()),
                            Value::Text(org.identifier_kind.clone().unwrap_or_default()),
                            Value::Text(org.country.clone().unwrap_or_default()),
                        ),
                    )
                    .await?;
                if rows.next().await?.is_some() {
                    j.election = org.elect(&args.fns, true);
                }
            }

            // The merge survivors among the candidates, through `organization_merged_identifiers_org`.
            let asked: Vec<i64> = judged.iter().filter(|j| j.candidate()).map(|j| page[j.at].id).collect();
            let merged = merged_among(&reader, &asked).await?;
            for j in judged.iter_mut() {
                j.merged = merged.contains(&page[j.at].id);
            }

            // The D4 keeps' ENG satellites, one primary-key seek each.
            for j in judged.iter_mut().filter(|j| j.election.verdict == HeadVerdict::Keep(HeadKeep::D4)) {
                let org = &page[j.at];
                let mut rows = reader.query(HEAD_ELECT_ENG_SQL, (Value::Integer(org.id),)).await?;
                let eng = match rows.next().await? {
                    None => "absent",
                    Some(row) => {
                        let name = text(&row, 0);
                        if name == org.head {
                            "equal"
                        } else if (args.fns.vote_key)(&name) == (args.fns.vote_key)(&org.head) {
                            "agrees"
                        } else {
                            "differs"
                        }
                    }
                };
                j.eng = Some(eng);
            }

            for j in &judged {
                let org = &page[j.at];
                let entry = || refresh_entry(org, j, &args);
                if canaries.contains(&org.id) {
                    plan.canaries.push(entry());
                }
                if !j.in_population {
                    continue;
                }
                let verdict = &j.election.verdict;
                plan.population.add(org.n);
                if org.head.trim().is_empty() {
                    plan.empty_heads.add(org.n);
                }
                match verdict {
                    HeadVerdict::Keep(keep) => {
                        plan.keeps.add(org.n);
                        plan.by_keep.entry(keep.label()).or_default().add(org.n);
                        if let Some(eng) = j.eng {
                            *plan.d4_eng.entry(eng).or_default() += 1;
                        }
                    }
                    HeadVerdict::Hold { reason, .. } => {
                        plan.holds.add(org.n);
                        plan.by_hold.entry(*reason).or_default().add(org.n);
                        if plan.held.len() < args.listing_cap {
                            plan.held.push(entry());
                        } else {
                            plan.held_truncated = true;
                        }
                    }
                    HeadVerdict::Change { rule, new } => {
                        plan.changes.add(org.n);
                        plan.by_rule.entry(rule.label()).or_default().add(org.n);
                        plan.planned_mentions += org.n;
                        if j.merged {
                            plan.planned_merged.add(org.n);
                        }
                        let move_ = match ((args.family)(&org.head), (args.family)(new)) {
                            (None, None) => "none->none",
                            (None, Some(_)) => "none->x",
                            (Some(_), None) => "x->none",
                            (Some(was), Some(becomes)) if was == becomes => "x->x",
                            (Some(_), Some(_)) => "x->y",
                        };
                        *plan.family_moves.entry(move_).or_default() += 1;
                        if plan.planned.len() < args.listing_cap {
                            plan.planned.push(entry());
                        } else {
                            plan.truncated = true;
                        }
                    }
                }
                let key = sample_key(org.id);
                let uniform = samples.entry((verdict.kind(), verdict.reason())).or_default();
                if uniform.admits(args.samples, key) {
                    uniform.insert(args.samples, key, entry());
                }
            }
            (args.progress)(
                plan.orgs_walked,
                &format!(
                    "electing heads: {} organization(s) walked, {} elected, {} change(s) planned, {} held",
                    plan.orgs_walked, plan.population.orgs, plan.changes.orgs, plan.holds.orgs
                ),
            );
        }

        // The samples' and canaries' merge survivors: a keep's was never read on its page.
        plan.samples = samples
            .into_iter()
            .map(|((kind, reason), uniform)| {
                (format!("{kind}:{reason}"), uniform.items.into_iter().map(|(_, entry)| entry).collect())
            })
            .collect();
        let listed: Vec<i64> = plan
            .samples
            .values()
            .flatten()
            .chain(plan.canaries.iter())
            .map(|entry| entry.org_id)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        let merged = merged_among(&reader, &listed).await?;
        for entry in plan.samples.values_mut().flatten().chain(plan.canaries.iter_mut()) {
            entry.merged = merged.contains(&entry.org_id);
        }
        Ok(plan)
    }
}

/// Which of `ids` survived an identifier-bearing merge — the census's read
/// ([`org_head_drift_merged_sql`]), in [`IN_CHUNK`] chunks.
async fn merged_among(reader: &turso::Connection, ids: &[i64]) -> turso::Result<HashSet<i64>> {
    let mut merged = HashSet::new();
    for chunk in ids.chunks(IN_CHUNK) {
        let params: Vec<Value> = chunk.iter().map(|&id| Value::Integer(id)).collect();
        let mut rows = reader.query(&org_head_drift_merged_sql(chunk.len()), params).await?;
        while let Some(row) = rows.next().await? {
            merged.insert(int(&row, 0));
        }
    }
    Ok(merged)
}

/// One page org's plan entry.
fn refresh_entry(org: &PageOrg, j: &Judged, args: &OrgNameRefreshArgs<'_>) -> HeadRefreshEntry {
    let verdict = &j.election.verdict;
    let new_name = verdict.new_name().map(str::to_owned);
    let mut spellings: Vec<(String, u64)> = org
        .spellings
        .iter()
        .filter(|(spelling, _)| !(args.fns.not_a_name)(spelling).is_some_and(|class| class != HEAD_DRIFT_AWARD_SUMMARY))
        .map(|(spelling, n)| (spelling.clone(), *n))
        .collect();
    spellings.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    spellings.truncate(ORG_NAME_REFRESH_SPELLINGS);
    HeadRefreshEntry {
        org_id: org.id,
        verdict: verdict.kind(),
        reason: verdict.reason(),
        rule: verdict.rule().map(HeadRule::label),
        old_name: org.head.clone(),
        old_name_norm: org.name_norm.clone(),
        new_name_norm: new_name.as_deref().map(org_name_norm),
        family_old: (args.family)(&org.head),
        family_new: new_name.as_deref().and_then(args.family),
        new_name,
        voting: j.election.voting,
        top: j.election.top,
        head: j.election.head_votes,
        second: j.election.second,
        clusters: j.election.clusters,
        n: org.n,
        merged: j.merged,
        country: org.country.clone(),
        identifier: org.identifier.clone(),
        provisional: org.provisional,
        eng: j.eng,
        in_population: j.in_population,
        spellings,
    }
}

// ---- Issue 524 unit 2b: the wet path — apply a stored plan, and revert one job's renames.

/// Issue 524 unit 2b: the planned changes one write transaction of [`Db::apply_org_name_refresh`]
/// (and one of [`Db::revert_org_name_refresh`]) holds.
pub const ORG_NAME_REFRESH_WINDOW: usize = 2_000;

/// Issue 524 unit 2b: the compare-and-set rename. It writes only a row still holding the plan's
/// byte-exact pre-image, and only an identifier-bearing one: the election excludes `identifier IS
/// NULL` (E1, the issue-234 bind key), and an identifier a case review stripped since the plan must
/// not let a provisional row's key move.
pub(crate) const ORG_NAME_REFRESH_CAS_SQL: &str = "UPDATE organizations SET name = ?, name_norm = ? \
     WHERE id = ? AND name = ? AND name_norm IS ? AND identifier IS NOT NULL";

/// Issue 524 unit 2b: one ledger row per rename, in the rename's transaction.
pub(crate) const ORG_NAME_REFRESH_LEDGER_SQL: &str = "INSERT INTO org_name_refresh_log \
     (org_id, at, job_id, rule, old_name, old_name_norm, new_name, new_name_norm, evidence) \
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)";

/// Issue 524 unit 2b: an organization's latest ledger instant, a seek on the primary key. A rename's
/// `at` is `max(now, latest + 1)`, so a revert and a re-apply inside one second cannot collide on
/// `(org_id, at)`.
pub(crate) const ORG_NAME_REFRESH_LATEST_AT_SQL: &str = "SELECT MAX(at) FROM org_name_refresh_log WHERE org_id = ?";

/// Issue 524 unit 2b: the renames one job has committed. A recovered run's `max_groups` counts them,
/// whether or not a resume cursor survived (the 2b review).
pub(crate) const ORG_NAME_REFRESH_JOB_COUNT_SQL: &str = "SELECT COUNT(*) FROM org_name_refresh_log WHERE job_id = ?";

/// Issue 524 unit 2b: one job's unreverted renames, through `org_name_refresh_log_job`.
pub(crate) const ORG_NAME_REFRESH_JOB_ROWS_SQL: &str = "SELECT org_id, at, old_name, old_name_norm, new_name, \
     new_name_norm FROM org_name_refresh_log WHERE job_id = ? AND reverted_at IS NULL ORDER BY org_id, at";

/// Issue 524 unit 2b: the compare-and-set restore, guarded on the pair the rename wrote, and only on an
/// identifier-bearing row, as the rename is ([`ORG_NAME_REFRESH_CAS_SQL`]). An identifier a case review
/// stripped since the rename made `name_norm` the issue-234 bind key: restoring the old pair there would
/// move a provisional row's key, which unit 2 never does (the 2b review).
pub(crate) const ORG_NAME_REVERT_CAS_SQL: &str = "UPDATE organizations SET name = ?, name_norm = ? \
     WHERE id = ? AND name = ? AND name_norm IS ? AND identifier IS NOT NULL";

/// Issue 524 unit 2b: why a restore's compare-and-set missed — no row (gone), `1` (its identifier was
/// stripped since the rename), `0` (renamed again since).
pub(crate) const ORG_NAME_REVERT_MISS_SQL: &str = "SELECT identifier IS NULL FROM organizations WHERE id = ?";

/// Issue 524 unit 2b: the job's durable resume cursor, written in each window's transaction so it
/// commits with the window's renames (the 2b review: written after COMMIT, a death between the two
/// left a recovered capped run with no cursor). The SQL of `Db::record_job_progress`, on the window's
/// held writer connection (calling that method there would wait on the writer it holds).
pub(crate) const ORG_NAME_REFRESH_PROGRESS_SQL: &str = "UPDATE job_queue SET progress = ? WHERE id = ?";

/// Issue 524 unit 2b: the drift limit — drifted plus gone targets above it abort a wet run before any
/// write: `max(2% of the targets read, 5)`, the 2% rounded down. On a full prod run (~29,000 targets)
/// the 2% decides it (580); under 300 targets the floor does.
pub fn org_name_refresh_drift_limit(targets_read: u64) -> u64 {
    std::cmp::max(targets_read / 50, 5)
}

/// Issue 524 unit 2b: one planned change as the wet path takes it — a stored plan's `planned` entry
/// (the app reads the report; store carries no JSON parser).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrgNameRefreshTarget {
    pub org_id: i64,
    /// `R0e` | `R0n` | `R1` | `R2`: the rule the plan elected the change by.
    pub rule: String,
    /// The pre-image the plan read: the compare-and-set's guard.
    pub old_name: String,
    pub old_name_norm: Option<String>,
    pub new_name: String,
}

/// Issue 524 unit 2b: [`Db::apply_org_name_refresh`]'s inputs.
pub struct OrgNameRefreshApplyArgs<'a> {
    /// The functions the plan was elected with: the pre-pass re-elects every target with them.
    pub fns: HeadElectFns,
    /// The stored plan's `planned` entries, already scoped to `org_ids` by the caller.
    pub targets: Vec<OrgNameRefreshTarget>,
    /// Apply at most this many of the targets that are still ready (the first by org id). A recovered
    /// run counts the renames its job already committed (its ledger rows) against it.
    pub max_groups: Option<u64>,
    /// The job: the ledger's `job_id`, and whose resume cursor each window advances.
    pub job_id: Option<i64>,
    /// A resumed job's cursor: targets at or below this org id were committed by its earlier run.
    pub resume_after: Option<i64>,
    /// Targets per write transaction: [`ORG_NAME_REFRESH_WINDOW`] in the job.
    pub window: usize,
    pub stop: &'a (dyn Fn() -> bool + Sync),
    pub progress: &'a (dyn Fn(u64, &str) + Sync),
}

/// Issue 524 unit 2b: one target the pre-pass found drifted (or gone), and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OrgNameRefreshDrift {
    pub org_id: i64,
    pub why: String,
}

/// Issue 524 unit 2b: the most drift samples [`OrgNameRefreshApplied`] lists.
pub const ORG_NAME_REFRESH_DRIFT_SAMPLES: usize = 30;

/// Issue 524 unit 2b: one rename a wet run committed, as its report samples it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OrgNameRefreshRename {
    pub org_id: i64,
    pub rule: String,
    pub old_name: String,
    pub new_name: String,
}

/// Issue 524 unit 2b: the committed renames [`OrgNameRefreshApplied`] samples — the operator's spot
/// check, since `org_name_refresh_log` is not on `/v1/sql`'s allow-list.
pub const ORG_NAME_REFRESH_APPLIED_SAMPLES: usize = 30;

/// Issue 524 unit 2b: what [`Db::apply_org_name_refresh`] did.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct OrgNameRefreshApplied {
    /// Targets in scope (deduplicated), and those a resumed run skipped (at or below its cursor).
    pub targets: u64,
    pub resumed_past: u64,
    /// A resumed run: the cursor it resumed after.
    pub resumed_after: Option<i64>,
    /// The renames this job id committed before this run (its ledger rows): an earlier attempt of a
    /// recovered job, counted against `max_groups` whether or not its cursor survived.
    pub prior_applied: u64,
    /// The pre-pass: still the plan's change (`ready`), already renamed to the plan's new pair
    /// (`already`, a replayed window), changed since the plan (`drifted`), or deleted (`gone`).
    pub ready: u64,
    pub already: u64,
    pub drifted: u64,
    pub gone: u64,
    /// [`org_name_refresh_drift_limit`] of the targets read: drifted plus gone above it aborts the run
    /// before any write.
    pub drift_limit: u64,
    pub aborted: bool,
    /// Ready targets past `max_groups`, left for a later run.
    pub capped: u64,
    /// Renamed (the compare-and-set held), with one ledger row and one `organization changed` row each.
    pub applied: u64,
    /// Ready at the pre-pass but changed before the window's compare-and-set: nothing written.
    pub raced: u64,
    pub windows: u64,
    /// The last org id a committed window covered: the job's resume cursor.
    pub cursor: Option<i64>,
    pub stopped: bool,
    /// The first [`ORG_NAME_REFRESH_DRIFT_SAMPLES`] drifted or gone targets, by org id.
    pub drift_samples: Vec<OrgNameRefreshDrift>,
    /// [`ORG_NAME_REFRESH_APPLIED_SAMPLES`] committed renames, uniform over the run (the planner's
    /// sample order, the smallest `(org_id × 2654435761) mod 2³²`), listed by org id. A run of that
    /// many renames or fewer lists every one.
    pub applied_samples: Vec<OrgNameRefreshRename>,
    /// The 513 U4 `fold_error` shape (the 2b review): a window that failed AFTER at least one window
    /// committed ends the run there, and the report comes back Ok with the committed windows' counts
    /// and samples, so the caller records them before it reports the failure. A failure before the
    /// first commit is the error itself: nothing was written.
    pub window_error: Option<String>,
}

/// Issue 524 unit 2b: [`Db::revert_org_name_refresh`]'s inputs.
pub struct OrgNameRefreshRevertArgs<'a> {
    /// The `refresh-org-names` job whose renames to restore.
    pub job: i64,
    /// The reverting job, stamped as `reverted_job`.
    pub revert_job: Option<i64>,
    pub window: usize,
    pub stop: &'a (dyn Fn() -> bool + Sync),
    pub progress: &'a (dyn Fn(u64, &str) + Sync),
}

/// Issue 524 unit 2b: what [`Db::revert_org_name_refresh`] did.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct OrgNameRefreshReverted {
    pub job: i64,
    /// The job's ledger rows not yet reverted.
    pub ledger_rows: u64,
    /// Restored to the pre-image, stamped `reverted_at`, announced `organization changed`.
    pub reverted: u64,
    /// The row no longer holds the pair the rename wrote (renamed again since): left as it is.
    pub renamed_again: u64,
    /// The row still holds the pair, but a case review stripped its identifier since the rename, so
    /// its `name_norm` is now an issue-234 bind key: left as it is, unstamped (a restored identifier
    /// lets a re-run take it up).
    pub identifier_stripped: u64,
    /// The organization is gone (merged away, swept): nothing to restore.
    pub gone: u64,
    pub windows: u64,
    pub stopped: bool,
    /// As [`OrgNameRefreshApplied::window_error`]: a window that failed after one committed.
    pub window_error: Option<String>,
}

/// The [`ORG_NAME_REFRESH_APPLIED_SAMPLES`] smallest-keyed renames, kept sorted by key.
#[derive(Default)]
struct Sampled {
    items: Vec<((u64, i64), OrgNameRefreshRename)>,
}

impl Sampled {
    fn admits(&self, key: (u64, i64)) -> bool {
        self.items.len() < ORG_NAME_REFRESH_APPLIED_SAMPLES || self.items.last().is_some_and(|(last, _)| key < *last)
    }

    fn insert(&mut self, key: (u64, i64), rename: OrgNameRefreshRename) {
        let at = self.items.partition_point(|(k, _)| *k < key);
        self.items.insert(at, (key, rename));
        self.items.truncate(ORG_NAME_REFRESH_APPLIED_SAMPLES);
    }

    fn into_by_org_id(self) -> Vec<OrgNameRefreshRename> {
        let mut out: Vec<OrgNameRefreshRename> = self.items.into_iter().map(|(_, rename)| rename).collect();
        out.sort_by_key(|rename| rename.org_id);
        out
    }
}

/// One target's pre-pass reading.
enum Readiness {
    Ready(HeadElection),
    Already,
    Drifted(String),
    Gone,
}

/// The pre-pass: each target's organization read and re-elected as the planner read it (the same
/// page SQL, mention read, voters and verdict probe), then compared with what the plan recorded.
async fn read_readiness(
    reader: &turso::Connection,
    fns: &HeadElectFns,
    targets: &[OrgNameRefreshTarget],
) -> turso::Result<Vec<Readiness>> {
    let mut out = Vec::with_capacity(targets.len());
    for chunk in targets.chunks(IN_CHUNK) {
        let ids: Vec<Value> = chunk.iter().map(|t| Value::Integer(t.org_id)).collect();
        let mut orgs: HashMap<i64, PageOrg> = HashMap::new();
        let mut rows = reader.query(&org_name_refresh_ids_sql(chunk.len()), ids.clone()).await?;
        while let Some(row) = rows.next().await? {
            orgs.insert(
                int(&row, 0),
                PageOrg {
                    id: int(&row, 0),
                    country: opt_text_of(&row, 1),
                    identifier: opt_text_of(&row, 2),
                    provisional: int(&row, 3) != 0,
                    head: text(&row, 4),
                    identifier_kind: opt_text_of(&row, 5),
                    name_norm: opt_text_of(&row, 6),
                    n: 0,
                    spellings: HashMap::new(),
                },
            );
        }
        drop(rows);
        let mut rows = reader.query(&org_head_drift_mentions_sql(chunk.len()), ids).await?;
        while let Some(row) = rows.next().await? {
            let Some(org) = orgs.get_mut(&int(&row, 0)) else { continue };
            org.n += 1;
            if let Some(name) = opt_text_of(&row, 1).filter(|name| !name.trim().is_empty()) {
                *org.spellings.entry(name).or_default() += 1;
            }
        }
        drop(rows);
        for target in chunk {
            let Some(org) = orgs.get(&target.org_id) else {
                out.push(Readiness::Gone);
                continue;
            };
            let new_norm = org_name_norm(&target.new_name);
            if org.head == target.new_name && org.name_norm.as_deref() == Some(new_norm.as_str()) {
                out.push(Readiness::Already);
                continue;
            }
            if org.head != target.old_name || org.name_norm != target.old_name_norm {
                let why = if org.head != target.old_name {
                    format!("the head is {:?} now, not the plan's {:?}", org.head, target.old_name)
                } else {
                    format!("the name_norm is {:?} now, not the plan's {:?}", org.name_norm, target.old_name_norm)
                };
                out.push(Readiness::Drifted(why));
                continue;
            }
            let Some(identifier) = org.identifier.as_deref() else {
                out.push(Readiness::Drifted("the identifier is NULL now (E1)".to_owned()));
                continue;
            };
            let mut election = org.elect(fns, false);
            if matches!(election.verdict, HeadVerdict::Change { .. }) {
                let mut rows = reader
                    .query(
                        HEAD_ELECT_VERDICT_PROBE_SQL,
                        (
                            Value::Text(identifier.to_owned()),
                            Value::Text(org.identifier_kind.clone().unwrap_or_default()),
                            Value::Text(org.country.clone().unwrap_or_default()),
                        ),
                    )
                    .await?;
                if rows.next().await?.is_some() {
                    election = org.elect(fns, true);
                }
            }
            match &election.verdict {
                HeadVerdict::Change { rule, new } if rule.label() == target.rule && *new == target.new_name => {
                    out.push(Readiness::Ready(election));
                }
                other => out.push(Readiness::Drifted(format!(
                    "re-elected {}{}, not the plan's change:{} {:?}",
                    other.label(),
                    other.new_name().map(|n| format!(" {n:?}")).unwrap_or_default(),
                    target.rule,
                    target.new_name
                ))),
            }
        }
    }
    Ok(out)
}

/// The ledger's `evidence`: the epoch and the counts the rename was elected by. Every value is a
/// number or a fixed ASCII label, so nothing needs escaping.
fn refresh_evidence(rule: &str, election: &HeadElection) -> String {
    format!(
        "{{\"epoch\":\"{HEAD_ELECTION_EPOCH}\",\"rule\":\"{rule}\",\"voting\":{},\"top\":{},\"head\":{},\"second\":{},\
         \"clusters\":{}}}",
        election.voting, election.top, election.head_votes, election.second, election.clusters
    )
}

impl Db {
    /// Issue 524 unit 2b: the WET `refresh-org-names` — rename the heads a stored dry plan planned
    /// (ADR-0013's 2026-10-15 amendment), each with a byte-exact pre-image in `org_name_refresh_log`
    /// and one seq-less `organization changed` row.
    ///
    /// The caller has already refused a missing, truncated, other-epoch or other-binary plan and
    /// scoped `targets` to `org_ids`. This refuses while `organization_mentions_org` or
    /// `organization_merged_identifiers_org` is absent, as the planner does. Then:
    /// 1. **Pre-pass.** Every target's organization is read and re-elected with `fns` as the planner
    ///    elected it (the verdict probe included). A target is `ready` when the row still holds the
    ///    plan's pre-image, still carries an identifier, and re-elects to the plan's rule and
    ///    spelling; `already` when it holds the plan's new pair (a replayed window); `gone` when
    ///    deleted; `drifted` otherwise. Drifted plus gone above `max(2%, 5)` of the targets read
    ///    aborts the run here, before any write.
    /// 2. **Windows** of `args.window` ready targets (the first `max_groups` by org id), each one
    ///    [`Db::immediate`] transaction of compare-and-set renames ([`ORG_NAME_REFRESH_CAS_SQL`]:
    ///    `name_norm` follows as [`org_name_norm`], the mint's derivation), their ledger rows and
    ///    their `organization changed` rows. A row changed since the pre-pass fails its compare and
    ///    is counted `raced`, with nothing written. The job's resume cursor (the window's last org id)
    ///    is written in the same transaction. After each COMMIT: the cursor doorbell, a TRUNCATE
    ///    checkpoint, the progress line, and the stop flag (read before every window). A window that
    ///    fails after one committed ends the run with `window_error` set (see there).
    ///
    /// Not re-checked per window: E2, the identifier-verdict probe, runs at the pre-pass only. A
    /// `wrong` or unapplied `related` verdict recorded during the run (`POST /admin/identifier-verdicts`
    /// writes outside the queue) does not stop a later window's rename of that org.
    ///
    /// No Tender or lot row is written: `parties[].organization_name` is a read-time join of the
    /// organization, so under ADR-0017 D5 the moved entity is the organization alone.
    pub async fn apply_org_name_refresh(
        &self,
        args: OrgNameRefreshApplyArgs<'_>,
    ) -> turso::Result<OrgNameRefreshApplied> {
        for index in ["organization_mentions_org", "organization_merged_identifiers_org"] {
            if !self.has_index(index).await? {
                return Err(turso::Error::Error(format!(
                    "refresh-org-names (wet) refused: {index} is absent (a rebuild's Phase 1?), so the pre-pass \
                     would scan its table; run `reindex` first. Nothing was read or written"
                )));
            }
        }
        let mut targets = args.targets;
        targets.sort_by_key(|t| t.org_id);
        targets.dedup_by_key(|t| t.org_id);
        let mut report = OrgNameRefreshApplied { targets: targets.len() as u64, ..Default::default() };
        if let Some(after) = args.resume_after {
            let before = targets.len();
            targets.retain(|t| t.org_id > after);
            report.resumed_past = (before - targets.len()) as u64;
            report.resumed_after = Some(after);
            report.cursor = Some(after);
        }
        // Whenever the run has a job id, not only on resume: a fresh id has no ledger rows, and a
        // recovered one's are counted against `max_groups` even if its cursor never landed.
        if let Some(job) = args.job_id {
            let reader = self.reader().await?;
            let mut rows = reader.query(ORG_NAME_REFRESH_JOB_COUNT_SQL, (Value::Integer(job),)).await?;
            report.prior_applied = match rows.next().await? {
                Some(row) => int(&row, 0).max(0) as u64,
                None => 0,
            };
        }

        (args.progress)(0, &format!("pre-pass: re-electing {} planned organization(s)", targets.len()));
        let readiness = {
            let reader = self.reader().await?;
            read_readiness(&reader, &args.fns, &targets).await?
        };
        let mut ready: Vec<(&OrgNameRefreshTarget, HeadElection)> = Vec::new();
        for (target, reading) in targets.iter().zip(readiness) {
            match reading {
                Readiness::Ready(election) => ready.push((target, election)),
                Readiness::Already => report.already += 1,
                Readiness::Gone | Readiness::Drifted(_) => {
                    let why = match reading {
                        Readiness::Drifted(why) => {
                            report.drifted += 1;
                            why
                        }
                        _ => {
                            report.gone += 1;
                            "the organization is gone".to_owned()
                        }
                    };
                    if report.drift_samples.len() < ORG_NAME_REFRESH_DRIFT_SAMPLES {
                        report.drift_samples.push(OrgNameRefreshDrift { org_id: target.org_id, why });
                    }
                }
            }
        }
        report.ready = ready.len() as u64;
        report.drift_limit = org_name_refresh_drift_limit(targets.len() as u64);
        if report.drifted + report.gone > report.drift_limit {
            report.aborted = true;
            return Ok(report);
        }
        if let Some(cap) = args.max_groups.map(|cap| cap.saturating_sub(report.prior_applied))
            && ready.len() as u64 > cap
        {
            report.capped = ready.len() as u64 - cap;
            ready.truncate(cap as usize);
        }

        let mut samples = Sampled::default();
        for window in ready.chunks(args.window.max(1)) {
            if (args.stop)() {
                report.stopped = true;
                break;
            }
            let now = crate::now_unix();
            let last = window.last().map(|(target, _)| target.org_id);
            let conn = self.conn().await;
            let committed = Self::immediate(&conn, async {
                let (mut applied, mut raced) = (Vec::new(), 0u64);
                for (target, election) in window {
                    let new_norm = org_name_norm(&target.new_name);
                    let changed = conn
                        .execute(
                            ORG_NAME_REFRESH_CAS_SQL,
                            (
                                Value::Text(target.new_name.clone()),
                                Value::Text(new_norm.clone()),
                                Value::Integer(target.org_id),
                                Value::Text(target.old_name.clone()),
                                target.old_name_norm.clone().map_or(Value::Null, Value::Text),
                            ),
                        )
                        .await?;
                    if changed == 0 {
                        raced += 1;
                        continue;
                    }
                    let mut latest = conn.query(ORG_NAME_REFRESH_LATEST_AT_SQL, (Value::Integer(target.org_id),)).await?;
                    let at = match latest.next().await? {
                        Some(row) => opt_int_of(&row, 0).map_or(now, |last| now.max(last + 1)),
                        None => now,
                    };
                    drop(latest);
                    conn.execute(
                        ORG_NAME_REFRESH_LEDGER_SQL,
                        vec![
                            Value::Integer(target.org_id),
                            Value::Integer(at),
                            args.job_id.map_or(Value::Null, Value::Integer),
                            Value::Text(target.rule.clone()),
                            Value::Text(target.old_name.clone()),
                            target.old_name_norm.clone().map_or(Value::Null, Value::Text),
                            Value::Text(target.new_name.clone()),
                            Value::Text(new_norm),
                            Value::Text(refresh_evidence(&target.rule, election)),
                        ],
                    )
                    .await?;
                    append_change(&conn, "organization", target.org_id, None, "changed", now).await?;
                    applied.push(*target);
                }
                // The resume cursor commits with the window (a job row that is absent, as in the store
                // tests, takes the UPDATE as a no-op).
                if let (Some(job), Some(last)) = (args.job_id, last) {
                    conn.execute(ORG_NAME_REFRESH_PROGRESS_SQL, (Value::Text(last.to_string()), Value::Integer(job)))
                        .await?;
                }
                Ok((applied, raced))
            })
            .await;
            let (applied, raced) = match committed {
                Ok(done) => done,
                // Rolled back; before the first commit nothing was written, so the error is the answer.
                Err(e) if report.windows == 0 => return Err(e),
                Err(e) => {
                    report.window_error = Some(format!(
                        "window {} (org ids {}..={}) rolled back: {e}",
                        report.windows + 1,
                        window.first().map(|(t, _)| t.org_id).unwrap_or_default(),
                        last.unwrap_or_default()
                    ));
                    break;
                }
            };
            report.applied += applied.len() as u64;
            report.raced += raced;
            report.windows += 1;
            for target in &applied {
                let key = sample_key(target.org_id);
                if samples.admits(key) {
                    samples.insert(
                        key,
                        OrgNameRefreshRename {
                            org_id: target.org_id,
                            rule: target.rule.clone(),
                            old_name: target.old_name.clone(),
                            new_name: target.new_name.clone(),
                        },
                    );
                }
            }
            report.cursor = last;
            // The window is committed: a failed doorbell must not drop its counts, so it ends the run
            // as a window error (the next publish catches subscribers up).
            if !applied.is_empty()
                && let Err(e) = self.publish_cursor(&conn).await
            {
                report.window_error = Some(format!(
                    "the change-cursor doorbell after window {} (through org id {}) failed: {e}",
                    report.windows,
                    last.unwrap_or_default()
                ));
                break;
            }
            if let Err(e) = checkpoint_on(&conn, CheckpointMode::Truncate).await {
                eprintln!("[refresh-org-names] checkpoint after a window: {e}");
            }
            drop(conn);
            (args.progress)(
                report.applied,
                &format!(
                    "renaming heads: {} applied, {} raced, {} window(s), through org id {}",
                    report.applied,
                    report.raced,
                    report.windows,
                    last.unwrap_or_default()
                ),
            );
        }
        report.applied_samples = samples.into_by_org_id();
        Ok(report)
    }

    /// Issue 524 unit 2b: undo one `refresh-org-names` job's renames. Each of the job's unreverted
    /// `org_name_refresh_log` rows is restored to its pre-image under a compare-and-set on the pair
    /// the rename wrote ([`ORG_NAME_REVERT_CAS_SQL`]), stamped `reverted_at` / `reverted_job`, and
    /// announced as one `organization changed` row, in windows of `args.window` through
    /// [`Db::immediate`]; the stop flag is read before every window, and a re-run takes up the
    /// rows still unreverted. A row renamed again since (by a later job, or a merge) is left as it
    /// is and counted `renamed_again`; one whose identifier a case review stripped since the rename
    /// counts `identifier_stripped` (its `name_norm` is a bind key now); a deleted organization counts
    /// `gone`. None of them is stamped. A window that fails after one committed ends the run with
    /// `window_error` set.
    ///
    /// The guard is the pair the rename wrote, so it is blind to an A→B→A→B history: reverting the
    /// first of two jobs that each wrote B restores A over the second's B and leaves the second's row
    /// unreverted (its revert then counts it renamed again). The served value is still a pre-image;
    /// only the ledger's account of it is off. Revert jobs newest first.
    pub async fn revert_org_name_refresh(
        &self,
        args: OrgNameRefreshRevertArgs<'_>,
    ) -> turso::Result<OrgNameRefreshReverted> {
        struct Row {
            org_id: i64,
            at: i64,
            old_name: String,
            old_name_norm: Option<String>,
            new_name: String,
            new_name_norm: String,
        }
        let mut report = OrgNameRefreshReverted { job: args.job, ..Default::default() };
        let rows: Vec<Row> = {
            let reader = self.reader().await?;
            let mut rows = reader.query(ORG_NAME_REFRESH_JOB_ROWS_SQL, (Value::Integer(args.job),)).await?;
            let mut out = Vec::new();
            while let Some(row) = rows.next().await? {
                out.push(Row {
                    org_id: int(&row, 0),
                    at: int(&row, 1),
                    old_name: text(&row, 2),
                    old_name_norm: opt_text_of(&row, 3),
                    new_name: text(&row, 4),
                    new_name_norm: text(&row, 5),
                });
            }
            out
        };
        report.ledger_rows = rows.len() as u64;
        for window in rows.chunks(args.window.max(1)) {
            if (args.stop)() {
                report.stopped = true;
                break;
            }
            let now = crate::now_unix();
            let conn = self.conn().await;
            let committed = Self::immediate(&conn, async {
                let (mut reverted, mut renamed_again, mut stripped, mut gone) = (0u64, 0u64, 0u64, 0u64);
                for row in window {
                    let changed = conn
                        .execute(
                            ORG_NAME_REVERT_CAS_SQL,
                            (
                                Value::Text(row.old_name.clone()),
                                row.old_name_norm.clone().map_or(Value::Null, Value::Text),
                                Value::Integer(row.org_id),
                                Value::Text(row.new_name.clone()),
                                Value::Text(row.new_name_norm.clone()),
                            ),
                        )
                        .await?;
                    if changed == 0 {
                        let mut miss = conn.query(ORG_NAME_REVERT_MISS_SQL, (Value::Integer(row.org_id),)).await?;
                        match miss.next().await? {
                            None => gone += 1,
                            Some(found) if int(&found, 0) != 0 => stripped += 1,
                            Some(_) => renamed_again += 1,
                        }
                        continue;
                    }
                    conn.execute(
                        "UPDATE org_name_refresh_log SET reverted_at = ?, reverted_job = ? WHERE org_id = ? AND at = ?",
                        (
                            Value::Integer(now),
                            args.revert_job.map_or(Value::Null, Value::Integer),
                            Value::Integer(row.org_id),
                            Value::Integer(row.at),
                        ),
                    )
                    .await?;
                    append_change(&conn, "organization", row.org_id, None, "changed", now).await?;
                    reverted += 1;
                }
                Ok((reverted, renamed_again, stripped, gone))
            })
            .await;
            let (reverted, renamed_again, stripped, gone) = match committed {
                Ok(done) => done,
                Err(e) if report.windows == 0 => return Err(e),
                Err(e) => {
                    report.window_error = Some(format!(
                        "window {} (org ids {}..={}) rolled back: {e}",
                        report.windows + 1,
                        window.first().map(|r| r.org_id).unwrap_or_default(),
                        window.last().map(|r| r.org_id).unwrap_or_default()
                    ));
                    break;
                }
            };
            report.reverted += reverted;
            report.renamed_again += renamed_again;
            report.identifier_stripped += stripped;
            report.gone += gone;
            report.windows += 1;
            if reverted > 0
                && let Err(e) = self.publish_cursor(&conn).await
            {
                report.window_error =
                    Some(format!("the change-cursor doorbell after window {} failed: {e}", report.windows));
                break;
            }
            if let Err(e) = checkpoint_on(&conn, CheckpointMode::Truncate).await {
                eprintln!("[revert-org-name-refresh] checkpoint after a window: {e}");
            }
            drop(conn);
            (args.progress)(
                report.reverted,
                &format!(
                    "restoring heads: {} reverted, {} renamed again, {} identifier stripped, {} gone",
                    report.reverted, report.renamed_again, report.identifier_stripped, report.gone
                ),
            );
        }
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        HEAD_HOLD_NO_FIT, HeadElectFns, HeadElectOrg, HeadKeep, HeadRule, HeadVerdict, Script, dominant_script,
        elect_head, elect_head_counted, sample_key,
    };

    /// Lower-cased words, `the` and the legal forms dropped, a run of single letters one initialism —
    /// the shape of `crosswalk::head_vote_key`.
    fn key(name: &str) -> String {
        let mut out: Vec<String> = Vec::new();
        let mut initials = String::new();
        // A trading-as clause is cut, as `altid_trim` cuts it.
        let name = name.split(" t/a ").next().unwrap_or(name);
        for word in name.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()) {
            let word: String = word.to_lowercase().chars().map(fold).collect();
            if word.chars().count() == 1 {
                initials.push_str(&word);
                continue;
            }
            if !initials.is_empty() {
                out.push(std::mem::take(&mut initials));
            }
            out.push(word);
        }
        if !initials.is_empty() {
            out.push(initials);
        }
        let all = out.join(" ");
        let kept: Vec<&str> = all
            .split(' ')
            .filter(|w| !matches!(*w, "the" | "ltd" | "limited" | "srl" | "plc" | "gmbh" | "sarl" | ""))
            .collect();
        if kept.is_empty() { all } else { kept.join(" ") }
    }

    /// The diacritics the tests spell, folded as `crosswalk::fold_diacritics` folds them.
    fn fold(c: char) -> char {
        match c {
            'ă' | 'á' | 'â' => 'a',
            'ý' => 'y',
            'î' => 'i',
            'à' => 'a',
            'ó' => 'o',
            'é' => 'e',
            'í' => 'i',
            other => other,
        }
    }

    /// Stand-in for `crosswalk::head_twin_key`: lower-cased, folded, punctuation dropped.
    fn twin(name: &str) -> String {
        name.to_lowercase().chars().map(fold).filter(|c| c.is_alphanumeric()).collect()
    }

    fn class(name: &str) -> Option<&'static str> {
        match name.trim() {
            "s.o." => Some("placeholder"),
            "-" => Some("blank"),
            n if n.starts_with("Awarded to ") => Some("award_summary"),
            _ => None,
        }
    }

    fn unfit(candidate: &str, _head: &str) -> Option<&'static str> {
        if candidate.chars().any(char::is_control) {
            return Some("control");
        }
        let cased = candidate.chars().any(|c| c.is_lowercase() || c.is_uppercase());
        if cased && !candidate.chars().any(char::is_uppercase) {
            return Some("lowercase");
        }
        class(candidate).map(|_| "non_name")
    }

    fn hold(old: &str, new: &str, _kind: Option<&str>, _identifier: Option<&str>) -> Option<&'static str> {
        let form = |name: &str| name.to_lowercase().split_whitespace().last().map(str::to_owned);
        match (form(old).as_deref(), form(new).as_deref()) {
            (Some("plc"), Some("ltd")) => Some("gb_family"),
            _ => None,
        }
    }

    /// Stand-in for `crosswalk::head_r2_hold`: the head cut to a prefix of itself.
    fn r2_hold(old: &str, new: &str) -> Option<&'static str> {
        let initials: String = old.split_whitespace().filter_map(|w| w.chars().next()).flat_map(char::to_lowercase).collect();
        let first = new.split_whitespace().next().unwrap_or("");
        let stands_out = first.len() >= 3 && first.chars().all(|c| c.is_ascii_uppercase());
        (stands_out && initials.contains(&first.to_lowercase())).then_some("initialism")
    }

    const FNS: HeadElectFns = HeadElectFns { vote_key: key, not_a_name: class, unfit, hold, r2_hold, twin_key: twin };

    fn org(head: &str) -> HeadElectOrg<'_> {
        HeadElectOrg { head, identifier_kind: Some("national"), identifier: Some("GB1"), bind_defect: false }
    }

    #[test]
    fn the_exclusions_come_first() {
        let votes = [("University of Sheffield", 5)];
        let mut nul = org("Sheffield Hallam University");
        nul.identifier = None;
        assert_eq!(elect_head(&votes, &nul, &FNS), HeadVerdict::Keep(HeadKeep::E1));
        let mut defect = org("Sheffield Hallam University");
        defect.bind_defect = true;
        assert_eq!(elect_head(&votes, &defect, &FNS), HeadVerdict::Keep(HeadKeep::E2));
        // Blank and non-name mentions do not vote; an award phrase does.
        let none = [("  ", 4), ("s.o.", 3), ("-", 2)];
        assert_eq!(elect_head(&none, &org("s.o."), &FNS), HeadVerdict::Keep(HeadKeep::E3));
        let award = [("Awarded to Bar Ltd", 3)];
        let counted = elect_head_counted(&award, &org("Foo"), &FNS);
        assert_eq!(counted.voting, 3);
        assert_eq!(counted.verdict, HeadVerdict::Hold { reason: HEAD_HOLD_NO_FIT, rule: HeadRule::R1, new: None });
    }

    /// R0 elects among the TIED top clusters: a fit mixed-case spelling first, then more fit mentions,
    /// then the smallest key; the empty head is R0e although `not_a_name("")` is `None`.
    #[test]
    fn a_defective_head_takes_the_top_cluster_and_its_tie_breaks() {
        let votes = [("ZETA AG", 2), ("Alpha AG", 2)];
        assert_eq!(
            elect_head(&votes, &org("s.o."), &FNS),
            HeadVerdict::Change { rule: HeadRule::R0n, new: "Alpha AG".into() }
        );
        let caps = [("ALPHA AG", 2), ("Zeta AG", 2)];
        assert_eq!(
            elect_head(&caps, &org("   "), &FNS),
            HeadVerdict::Change { rule: HeadRule::R0e, new: "Zeta AG".into() },
            "a mixed-case cluster beats a smaller key that has only ALL-CAPS"
        );
        let lower = [("alpha ag", 2), ("ZETA AG", 2)];
        assert_eq!(
            elect_head(&lower, &org(""), &FNS),
            HeadVerdict::Change { rule: HeadRule::R0e, new: "ZETA AG".into() },
            "more fit mentions beat a smaller key with none"
        );
        let unfit_top = [("alpha ag", 3), ("Zeta AG", 2)];
        assert_eq!(
            elect_head(&unfit_top, &org(""), &FNS),
            HeadVerdict::Hold { reason: HEAD_HOLD_NO_FIT, rule: HeadRule::R0e, new: None },
            "only the top cluster is elected"
        );
        // R0 skips D4: a Latin placeholder over Cyrillic mentions changes.
        let cyrillic = [("Община Белово", 2)];
        assert_eq!(
            elect_head(&cyrillic, &org("s.o."), &FNS),
            HeadVerdict::Change { rule: HeadRule::R0n, new: "Община Белово".into() }
        );
    }

    #[test]
    fn the_display_spelling_is_fit_then_mixed_case_then_most_mentions_then_byte_order() {
        let votes = [
            ("UNIVERSITY OF SHEFFIELD", 14),
            ("University of Sheffield", 1),
            ("The University of Sheffield", 1),
            ("Sheffield Hallam University", 1),
        ];
        let counted = elect_head_counted(&votes, &org("Sheffield Hallam University"), &FNS);
        assert_eq!(counted.verdict, HeadVerdict::Change { rule: HeadRule::R1, new: "The University of Sheffield".into() });
        assert_eq!(
            (counted.voting, counted.top, counted.head_votes, counted.second, counted.clusters),
            (17, 16, 1, 1, 2)
        );
        let more = [("University of Sheffield", 2), ("The University of Sheffield", 1), ("Hallam", 1)];
        assert_eq!(elect_head(&more, &org("Hallam"), &FNS).new_name(), Some("University of Sheffield"));
        let only_caps = [("UNIVERSITY OF SHEFFIELD", 3), ("university of sheffield", 9), ("Hallam", 1)];
        assert_eq!(elect_head(&only_caps, &org("Hallam"), &FNS).new_name(), Some("UNIVERSITY OF SHEFFIELD"));
    }

    #[test]
    fn the_bars_and_the_sticky_head() {
        // S1: tied for the top keeps the head byte for byte.
        let tie = [("Alpha", 3), ("Beta", 3)];
        assert_eq!(elect_head(&tie, &org("Beta"), &FNS), HeadVerdict::Keep(HeadKeep::S1));
        // R1's 55% bar, exactly: T x 20 >= V x 11. With H = 2 the 2x bar passes (11 >= 4) and R2 is off
        // (H > 1), so the share alone decides: 11 of 20 is 55% and changes, 11 of 21 is 52.4% and keeps.
        let at = [("Main", 11), ("Head", 2), ("Other", 7)];
        assert_eq!(elect_head(&at, &org("Head"), &FNS).rule(), Some(HeadRule::R1), "11/20 = 55%");
        let under = [("Main", 11), ("Head", 2), ("Other", 8)];
        assert_eq!(elect_head(&under, &org("Head"), &FNS), HeadVerdict::Keep(HeadKeep::Bar), "11/21 < 55%");
        // R1's 2x bar on its own: 55% passes, twice the head does not.
        let r1 = [("Main", 11), ("Head", 9)];
        assert_eq!(elect_head(&r1, &org("Head"), &FNS), HeadVerdict::Keep(HeadKeep::Bar), "11/20 but 11 < 2 x 9");
        let r1 = [("Main", 12), ("Head", 6), ("Other", 2)];
        assert_eq!(elect_head(&r1, &org("Head"), &FNS).rule(), Some(HeadRule::R1), "60% and twice the head");
        let near = [("Main", 10), ("Head", 2), ("Other", 7)];
        assert_eq!(elect_head(&near, &org("Head"), &FNS), HeadVerdict::Keep(HeadKeep::Bar), "52.6%, and H = 2 kills R2");
        // R2: a stray head (H <= 1) under a unique top of 3 at 25%.
        let stray = [("Main", 3), ("A", 2), ("B", 2), ("C", 2), ("D", 2), ("Head", 1)];
        assert_eq!(elect_head(&stray, &org("Head"), &FNS).rule(), Some(HeadRule::R2));
        let thin = [("Main", 3), ("A", 2), ("B", 2), ("C", 2), ("D", 2), ("E", 1), ("Head", 1)];
        assert_eq!(elect_head(&thin, &org("Head"), &FNS), HeadVerdict::Keep(HeadKeep::Bar), "3 of 13 is under 25%");
        let two = [("Main", 2), ("Head", 1)];
        assert_eq!(elect_head(&two, &org("Head"), &FNS), HeadVerdict::Keep(HeadKeep::Bar), "T = 2");
        let tied_top = [("Main", 3), ("Other", 3), ("Head", 1)];
        assert_eq!(elect_head(&tied_top, &org("Head"), &FNS), HeadVerdict::Keep(HeadKeep::Bar));
        // A bind guard holds the change with the spelling it would serve.
        let plc = [("Bar Services Ltd", 5), ("Foo Holdings plc", 1)];
        assert_eq!(
            elect_head(&plc, &org("Foo Holdings plc"), &FNS),
            HeadVerdict::Hold { reason: "gb_family", rule: HeadRule::R1, new: Some("Bar Services Ltd".into()) }
        );
    }

    /// An `award_summary` head is a name beside a non-name phrase: it votes, and it is NOT R0's
    /// defective head — tied for the top it is sticky, and under a cluster of two it stays (R0 would
    /// have replaced it with any top cluster).
    #[test]
    fn an_award_summary_head_is_not_defective() {
        let tied = [("Main Body", 3), ("Awarded to Bar Ltd", 3)];
        assert_eq!(elect_head(&tied, &org("Awarded to Bar Ltd"), &FNS), HeadVerdict::Keep(HeadKeep::S1));
        let small = [("Main Body", 2)];
        assert_eq!(elect_head(&small, &org("Awarded to Bar Ltd"), &FNS), HeadVerdict::Keep(HeadKeep::Bar));
        assert_eq!(
            elect_head(&small, &org("s.o."), &FNS),
            HeadVerdict::Change { rule: HeadRule::R0n, new: "Main Body".into() },
            "the same votes under a placeholder head: R0"
        );
    }

    /// S1 keeps a COSMETIC head — its cluster tops the votes, but no mention spells it byte for byte —
    /// as it is, never respelled to the cluster's display spelling.
    #[test]
    fn a_cosmetic_head_is_kept_byte_for_byte() {
        let votes = [("ACME TRADING", 5), ("Acme Trading", 2), ("Other Body", 1)];
        let counted = elect_head_counted(&votes, &org("Acme trading"), &FNS);
        assert_eq!(counted.verdict, HeadVerdict::Keep(HeadKeep::S1));
        assert_eq!((counted.top, counted.head_votes), (7, 7));
        assert_eq!(counted.verdict.new_name(), None);
    }

    /// R0's last tie-break: two tied top clusters alike in case and fit mentions elect the smaller vote
    /// key, whatever order the votes come in.
    #[test]
    fn a_defective_heads_tie_falls_to_the_smallest_vote_key() {
        let forward = [("Zeta Body", 2), ("Alpha Body", 2)];
        let backward = [("Alpha Body", 2), ("Zeta Body", 2)];
        for votes in [forward, backward] {
            assert_eq!(
                elect_head(&votes, &org("s.o."), &FNS),
                HeadVerdict::Change { rule: HeadRule::R0n, new: "Alpha Body".into() }
            );
        }
    }

    /// The 2a review: `ß` (capital `SS`), `ª` and `º` are lower case to `char::is_lowercase` but have no
    /// one-letter capital, so an ALL-CAPS writer's `STRAßENBAU` and `Nº` stay ALL-CAPS, and the mixed
    /// spelling is served.
    #[test]
    fn sharp_s_and_the_ordinal_indicators_do_not_make_a_spelling_mixed_case() {
        use crate::canonical::all_caps;
        assert!(all_caps("STRAßE"));
        assert!(all_caps("STRAẞE"), "the capital sharp s");
        assert!(!all_caps("Straße"));
        assert!(all_caps("AGRUPAMENTO DE ESCOLAS Nº 1"));
        assert!(!all_caps("Agrupamento de Escolas nº 1"));
        assert!(!all_caps("ºª 1"), "no upper-case letter at all");
        let strassen = [("LANDESBETRIEB STRAßENBAU NRW", 5), ("Landesbetrieb Straßenbau NRW", 2), ("Strassen NRW", 1)];
        assert_eq!(
            elect_head(&strassen, &org("Strassen NRW"), &FNS),
            HeadVerdict::Change { rule: HeadRule::R1, new: "Landesbetrieb Straßenbau NRW".into() }
        );
        let escolas = [("AGRUPAMENTO DE ESCOLAS Nº 1", 4), ("Agrupamento de Escolas nº 1", 1), ("Escola Norte", 1)];
        assert_eq!(
            elect_head(&escolas, &org("Escola Norte"), &FNS),
            HeadVerdict::Change { rule: HeadRule::R1, new: "Agrupamento de Escolas nº 1".into() }
        );
    }

    /// Issue 524 unit 2a.1: the R2 shorthand hold is asked of R2 alone. The same pair (an initialism of
    /// the head's first words) holds under R2's minority top and changes under R1's majority.
    #[test]
    fn the_shorthand_hold_is_r2_only() {
        let head = "Communauté de communes Terroir";
        let r2 = [("CDC Terroir", 3), ("Nord", 2), ("Sued", 2), ("Ost", 2), ("West", 2), (head, 1)];
        assert_eq!(
            elect_head(&r2, &org(head), &FNS),
            HeadVerdict::Hold { reason: "initialism", rule: HeadRule::R2, new: Some("CDC Terroir".into()) }
        );
        let r1 = [("CDC Terroir", 10), (head, 1)];
        assert_eq!(elect_head(&r1, &org(head), &FNS), HeadVerdict::Change { rule: HeadRule::R1, new: "CDC Terroir".into() });
        // An R2 change the hold does not read as a shorthand still changes.
        let other = [("Other Body", 3), ("Nord", 2), ("Sued", 2), ("Ost", 2), ("West", 2), (head, 1)];
        assert_eq!(elect_head(&other, &org(head), &FNS).kind(), "change");
    }

    /// Issue 524 unit 2a.1/2a.2: one vote key folds the diacritics, so an accented head and its ASCII
    /// ALL-CAPS twin are one cluster (13731240 is an S1 keep), and the diacritic-bearing spelling is
    /// served over its ASCII TWIN (equal but for accents and punctuation), after case and before votes.
    #[test]
    fn diacritics_are_one_cluster_and_the_marked_twin_is_served() {
        let dumbraveni = [("Comuna Dumbrăveni", 10), ("COMUNA DUMBRAVENI", 32)];
        let counted = elect_head_counted(&dumbraveni, &org("Comuna Dumbrăveni"), &FNS);
        assert_eq!(counted.verdict, HeadVerdict::Keep(HeadKeep::S1));
        assert_eq!((counted.clusters, counted.top, counted.head_votes), (1, 42, 42));
        let dotacion = [("Dotacion y Equipamiento S.L.", 4), ("Dotación y Equipamiento, S. L.", 1), ("Equipos Norte", 1)];
        assert_eq!(
            elect_head(&dotacion, &org("Equipos Norte"), &FNS),
            HeadVerdict::Change { rule: HeadRule::R1, new: "Dotación y Equipamiento, S. L.".into() }
        );
        // Case still comes first: the mixed-case ASCII spelling beats the ALL-CAPS accented one.
        let caps = [("COMUNA DUMBRĂVENI", 5), ("Comuna Dumbraveni", 1), ("Primaria Sat", 1)];
        assert_eq!(elect_head(&caps, &org("Primaria Sat"), &FNS).new_name(), Some("Comuna Dumbraveni"));
    }

    /// Issue 524 unit 2a.2 (the 2a.1 review's regression): a cluster joins more than twins — a
    /// trading-as clause, a legal form — and the marked-letter preference never reaches past twins, so
    /// the most-voted spelling is served.
    #[test]
    fn an_accent_outside_a_twin_does_not_win_the_display() {
        let trading = [("Acme Ltd", 20), ("Acme Ltd t/a Café Rouge", 1), ("Other Body", 1)];
        assert_eq!(elect_head(&trading, &org("Other Body"), &FNS).new_name(), Some("Acme Ltd"));
        let form = [("Acme GmbH", 20), ("Acme Sàrl", 1), ("Other Body", 1)];
        let counted = elect_head_counted(&form, &org("Other Body"), &FNS);
        assert_eq!(counted.clusters, 2, "GmbH and Sàrl are one vote key");
        assert_eq!(counted.verdict.new_name(), Some("Acme GmbH"));
        // An accent-stripped winner yields to its accented twin, whatever the votes.
        let twins = [("Acme Cafe Ltd", 20), ("Acme Café Ltd", 1), ("Acme Ltd t/a Café Rouge", 5), ("Other Body", 1)];
        assert_eq!(elect_head(&twins, &org("Other Body"), &FNS).new_name(), Some("Acme Café Ltd"));
    }

    /// Issue 524 unit 2a.3 (the 2a.2 review): the twin swap replaces only a winner with NO marked
    /// letter, and among the marked twins the most-mentioned is served, not the most-marked. So a
    /// correctly accented winner keeps its place against a one-mention typo twin.
    #[test]
    fn a_twin_swap_never_replaces_an_accented_winner() {
        // 1526605: 717 votes for `sociálnych` against 1 for the typo `sociálných`.
        let ministry = [
            ("Ministerstvo práce, sociálnych vecí a rodiny Slovenskej republiky", 717),
            ("Ministerstvo práce,sociálných vecí a rodiny Slovenskej republiky", 1),
            ("Urad Prace", 1),
        ];
        assert_eq!(
            elect_head(&ministry, &org("Urad Prace"), &FNS).new_name(),
            Some("Ministerstvo práce, sociálnych vecí a rodiny Slovenskej republiky")
        );
        // 13303943: the winner already has `România`; `FrieslandCâmpina` (12) does not displace it (23).
        let friesland = [("FrieslandCampina România S.A.", 23), ("FrieslandCâmpina România S.A.", 12), ("Lactate Sud", 1)];
        assert_eq!(elect_head(&friesland, &org("Lactate Sud"), &FNS).new_name(), Some("FrieslandCampina România S.A."));
        // An ASCII winner yields to the most-MENTIONED accented twin, not the most-marked one.
        let benoit = [("Ville de Saint-Benoit", 41), ("Ville de Saint-Benoît", 9), ("Ville de Saînt-Benoît", 1), ("Mairie", 1)];
        assert_eq!(elect_head(&benoit, &org("Mairie"), &FNS).new_name(), Some("Ville de Saint-Benoît"));
    }

    #[test]
    fn the_script_guard_keeps_a_latin_head_over_cyrillic_mentions_only() {
        let votes = [("ОБЩИНА БЕЛОВО", 29), ("Obshtina Belovo", 1)];
        assert_eq!(elect_head(&votes, &org("Obshtina Belovo"), &FNS), HeadVerdict::Keep(HeadKeep::D4));
        // No letter, no script: a digit head is not guarded.
        let digits = [("360 Solutions", 5)];
        assert_eq!(elect_head(&digits, &org("360"), &FNS).rule(), Some(HeadRule::R1));
        assert_eq!(dominant_script("Obshtina Belovo"), Some(Script::Latin));
        assert_eq!(dominant_script("ОБЩИНА БЕЛОВО"), Some(Script::Cyrillic));
        assert_eq!(dominant_script("Δήμος Αθηναίων"), Some(Script::Greek));
        assert_eq!(dominant_script("Łódź Ślęża"), Some(Script::Latin));
        assert_eq!(dominant_script("東京都"), Some(Script::Other));
        assert_eq!(dominant_script("360 / 24-7"), None);
        assert_eq!(dominant_script("ООО Ромашка Ltd"), Some(Script::Cyrillic), "the most letters win");
    }

    #[test]
    fn the_sample_key_is_knuths_hash_mod_two_to_the_32() {
        assert_eq!(sample_key(1), (2_654_435_761, 1));
        assert_eq!(sample_key(2), (1_013_904_226, 2));
        assert_eq!(sample_key(5), (387_276_917, 5));
        assert_eq!(sample_key(31_558_903).0, (31_558_903u64 * 2_654_435_761) % (1 << 32));
    }

    /// Issue 524 unit 2a: the planner's three candidate reads must seek, not scan: the scoped page by
    /// rowid, the verdict probe and the ENG satellite on their tables' primary keys.
    #[tokio::test]
    async fn the_refresh_plans_candidate_reads_seek() {
        let path = format!("/tmp/tender-db-524u2-eqp-{}.db", std::process::id());
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{suffix}"));
        }
        let db = crate::Db::open(&path).await.unwrap();
        db.build_organization_indexes().await.unwrap();
        let conn = db.reader().await.unwrap();
        let ids: Vec<turso::Value> = (1..=crate::canonical::IN_CHUNK as i64).map(turso::Value::Integer).collect();
        let text = |s: &str| turso::Value::Text(s.into());
        for (reader, sql, params, needle) in [
            ("scoped page", super::org_name_refresh_ids_sql(crate::canonical::IN_CHUNK), ids, "organizations"),
            (
                "verdict probe",
                super::HEAD_ELECT_VERDICT_PROBE_SQL.to_owned(),
                vec![text("GB1"), text("national"), text("GB")],
                "org_identifier_verdicts",
            ),
            ("eng satellite", super::HEAD_ELECT_ENG_SQL.to_owned(), vec![turso::Value::Integer(1)], "organization_names"),
        ] {
            let mut rows = conn.query(&format!("EXPLAIN QUERY PLAN {sql}"), params).await.unwrap();
            let mut plan = String::new();
            while let Some(r) = rows.next().await.unwrap() {
                plan.push_str(&r.get_value(3).ok().and_then(|v| v.as_text().cloned()).unwrap_or_default());
                plan.push('\n');
            }
            println!("PLAN {reader}:\n{plan}");
            assert!(
                plan.lines().any(|l| l.contains("SEARCH") && l.contains(needle)),
                "{reader} must seek — plan was:\n{plan}"
            );
            assert!(!plan.lines().any(|l| l.trim_start().starts_with("SCAN ")), "{reader} scans — plan was:\n{plan}");
        }
        drop(conn);
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{suffix}"));
        }
    }
}
