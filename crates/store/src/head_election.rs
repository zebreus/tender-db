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
    org_head_drift_merged_sql, org_name_norm, placeholders,
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
/// bare initialism, skips particles and knows the review's designations. `v6` (unit 2b, dry plan 2515):
/// the display spelling is chosen among the fit spellings with their whitespace collapsed, a run's
/// spellings counted together, so no change serves a doubled space (often a dropped `&`); the vote
/// key reads a spelling and its collapsed form alike (`crosswalk::head_vote_key`), so a served head
/// keys into its own cluster. `v7` (unit 2b, the recency decision, panel wf_ce13ee62-9e9, after the
/// capped run 2517 put five orgs back to a FORMER name): R1 with H ≥ 2 is HELD (`recent`) while the
/// head's own procedures are the organization's newest ([`HEAD_HOLD_RECENT`]), or (`undated`) when
/// the caller passed no dates. `v8` (the dated re-judge of plan 2523: its residue sample read 9 of 30
/// wrong, and 2 of its 10 junk releases were wrong): R1 and R2 are HELD (`residue`) when the
/// organization's single newest item is the head's ([`HEAD_HOLD_RESIDUE`]), and the junk exemption
/// no longer releases an all-lower-case head or one whose only new digits sit inside a word
/// (`CA2BM`).
pub const HEAD_ELECTION_EPOCH: &str = "524u2-v8";

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

/// Issue 524 rule `v7`: the hold reason when R1 (H ≥ 2) would replace a head whose own procedures are
/// the organization's newest — a rename the all-history majority would revert (2517's 2186, 3262,
/// 3694, 6867, 10671). See [`elect_head`]'s window.
pub const HEAD_HOLD_RECENT: &str = "recent";

/// Issue 524 rule `v7`: the hold reason when R1 (H ≥ 2) has no dates to read the window from
/// (`HeadElectOrg::dated` is `None`). It fails closed: a caller without dates can never write an R1
/// change the window might have stopped. A finished plan must count none.
pub const HEAD_HOLD_UNDATED: &str = "undated";

/// Issue 524 rule `v8`: the hold reason when R1 or R2 would replace a head that names the
/// organization's single NEWEST item: on its newest instant (a date-only notice's day) none of its
/// other names is printed, since the window ranks a non-head item newer on a tie. The residue holdout:
/// by dates an H = 1 or single-procedure rename looks like a newest stray, and the re-judge of plan
/// 2523 found the head the better name there more often (an organ served for its entity, one
/// notice's lot repeats, misspellings). The junk exemption releases it as it releases
/// [`HEAD_HOLD_RECENT`]; R0 is never held.
pub const HEAD_HOLD_RESIDUE: &str = "residue";

/// Issue 524 rule `v7`: the fewest head procedures (k) the window fires at. At 2 the 2/3 bar means
/// both newest items are the head's (2.2% by chance among 10 items, against 18% for 2 of 3).
pub const HEAD_RECENT_MIN_PROCEDURES: u64 = 2;

/// Issue 524 rule `v7`: one dated mention — its spelling, its procedure (the smallest `tender_id` of
/// the notice's `tender_versions` rows, so an issue-278 ghost reads as one procedure) and that
/// version's `published_at`. The caller drops a mention whose notice has no usable version (`at ≤ 0`,
/// or past the job's start by more than a day) and counts it undated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DatedVote<'a> {
    pub spelling: &'a str,
    pub tender: i64,
    pub at: i64,
}

/// Issue 524 rule `v7`: what the window read, whenever the caller passed dates.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RecentReading {
    /// k: the head cluster's items (distinct `(vote key, tender)` pairs).
    pub head_procs: u64,
    /// h_W: the head's items among the newest k items of the organization.
    pub head_in_window: u64,
    /// The newest head item's and the elected cluster's newest item's `at`.
    pub head_newest: Option<i64>,
    pub top_newest: Option<i64>,
    /// The organization's newest item is a head item (rule `v8`: R1 and R2 hold [`HEAD_HOLD_RESIDUE`]
    /// on it). `false` when no item is dated, or none of the head's is.
    pub newest_is_head: bool,
    /// Voting mentions the dates did not cover.
    pub undated: u64,
    /// k ≥ [`HEAD_RECENT_MIN_PROCEDURES`] and 3·h_W ≥ 2·k.
    pub fires: bool,
    /// A hold the junk exemption released — the window fired on an R1 change (H ≥ 2, `recent`), or
    /// (rule `v8`) the newest item is the head's on an R1 or R2 change (`residue`): `unfit` (the head
    /// is unfit to serve even against itself, for any reason but `lowercase`) or `digits` (its vote
    /// key has an all-digit token the elected key lacks: an address number, a postcode, a NIP, `LOT
    /// 2`'s `2`).
    pub released: Option<&'static str>,
}

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
    /// `family`, `gb_family`, `consortium_removed`, `nhs_check`), for R2 only a shorthand's
    /// ([`HeadElectFns::r2_hold`]: `initialism`), for R1 with H ≥ 2 (rule `v7`) [`HEAD_HOLD_RECENT`]
    /// (with the display spelling, `None` when none is fit) or [`HEAD_HOLD_UNDATED`] (`None`), or for
    /// R1 and R2 (rule `v8`) [`HEAD_HOLD_RESIDUE`] (with the display spelling) — with the spelling it
    /// would have served.
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
    /// Rule `v7`: the organization's dated mentions, read by the caller for the candidates (`None`
    /// otherwise). R1 with H ≥ 2 reads its window from them, and holds `undated` without them; R1 and
    /// R2 read the residue holdout from them (rule `v8`), and change as before without them.
    pub dated: Option<&'a [DatedVote<'a>]>,
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
    /// Rule `v7`: the window's reading, whenever the caller passed dates.
    pub recent: Option<RecentReading>,
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
/// 5. **R1**: T × 20 ≥ V × 11 (55%), T ≥ [`HEAD_ELECT_TOP_MIN`], T ≥ 2H, and the top is unique. Then,
///    only when H ≥ 2 (rule `v7`, the recency decision): with no dates it is HELD `undated`
///    ([`HEAD_HOLD_UNDATED`], `new` `None`: it fails closed); when the WINDOW fires and the junk
///    exemption does not apply it is HELD `recent` ([`HEAD_HOLD_RECENT`], `new` the display spelling).
///    - An ITEM is a distinct (vote key, tender) pair of the voting dated mentions, at its rows'
///      newest `at`; k is the head cluster's items. Every item of the organization is ranked newest
///      first: `at` descending, then on an equal `at` every non-head item before every head item,
///      then tender descending, then key ascending. W is the first k; h_W the head items in it. The
///      window fires when k ≥ [`HEAD_RECENT_MIN_PROCEDURES`] and 3·h_W ≥ 2·k (both newest at k = 2,
///      2 of 3, 4 of 6); a third cluster's items count against the head.
///    - The junk exemption releases it to R1 (counted `released`): the head is unfit against itself
///      ([`HeadElectFns::unfit`]: control, mojibake, long, non_name — since rule `v8` not
///      `lowercase`, a deliberate lower-case brand: 3909's `act legal s.r.o., advokátní kancelář`),
///      or its vote key has a token of decimal digits only (`char::is_ascii_digit` across the token,
///      rule `v8`: an address number, a postcode, a NIP/REGON, `LOT 2`'s `2` — never 1846's acronym
///      `CA2BM`) the elected key lacks.
/// 6. **R2**: H ≤ 1, the top is unique, T ≥ [`HEAD_ELECT_TOP_MIN`], and T × 4 ≥ V (25%).
/// 7. Otherwise KEEP (`bar`).
///
/// **The residue holdout** (rule `v8`): an R1 or R2 change (5 or 6, past the window) whose
/// organization's single newest item is a head item (`newest_is_head`) is HELD `residue`
/// ([`HEAD_HOLD_RESIDUE`], `new` the display spelling) unless the junk exemption releases it. Only
/// with dates: an undated R1 with H ≥ 2 holds `undated` before it, and an undated H ≤ 1 change (no
/// dates, or `Some(&[])`) changes as before. R0 is never held.
///
/// The elected cluster serves its DISPLAY spelling: among the fit spellings only
/// ([`HeadElectFns::unfit`], read on each spelling as published and on its collapsed form), each with
/// its whitespace collapsed (trimmed, every run one space) and the spellings that collapse alike
/// counted together (rule `v6`), the winner is not-ALL-CAPS before ALL-CAPS, then the most mentions,
/// then the byte-wise smallest — never the census's count-first `top_spelling`, which serves
/// `UNIVERSITY OF SHEFFIELD`. So no change serves a doubled space (a head the rule keeps stays byte
/// for byte, its doubled space included), `Acme  GmbH` 2 and `Acme GmbH` 2 serve `Acme GmbH` with 4
/// over `Acme-GmbH` 3, and dry plan 2515's `Julius Brune GmbH  Co. KG` (its `&` dropped), tied 2–2
/// with `Julius Brune GmbH & Co. KG`, loses on bytes once collapsed (`&` 0x26 before `C` 0x43, where
/// the doubled space's 0x20 had won). The served head is then the collapsed form, not always a
/// spelling some mention published byte for byte; its `name_norm` (`org_name_norm`, which collapses
/// whitespace too) and its vote key (`crosswalk::head_vote_key`, likewise) are those of the spellings
/// it stands for. A display equal to the head byte for byte is a KEEP (`S1`), never a change to
/// itself. Then, when the winner carries no marked (non-ASCII) letter — an accent-stripped spelling —
/// the most-mentioned fit spelling of its case that is its TWIN ([`HeadElectFns::twin_key`]: equal
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
    let counted = |verdict, recent| HeadElection {
        verdict,
        voting,
        top,
        head_votes,
        second,
        clusters: clusters.len() as u64,
        recent,
    };
    // Rule v7: the window, read against the cluster the rest of the rule elects (R0 re-reads it
    // against its own pick), whenever the caller passed dates.
    let main_key = if clusters.is_empty() {
        None
    } else if head_votes == top {
        Some(head_key.as_str())
    } else {
        Some(tied[0])
    };
    let reading = org.dated.map(|dated| recent_reading(dated, fns, &head_key, main_key, voting));

    if org.identifier.is_none() {
        return counted(HeadVerdict::Keep(HeadKeep::E1), reading);
    }
    if org.bind_defect {
        return counted(HeadVerdict::Keep(HeadKeep::E2), reading);
    }
    if clusters.is_empty() {
        return counted(HeadVerdict::Keep(HeadKeep::E3), reading);
    }
    let serve = |rule: HeadRule, cluster: &Cluster<'_>| match display_spelling(cluster, org.head, fns) {
        None => HeadVerdict::Hold { reason: HEAD_HOLD_NO_FIT, rule, new: None },
        // The belt (rule v6's review): a display equal to the head is no change; the head keeps.
        Some(new) if new == org.head => HeadVerdict::Keep(HeadKeep::S1),
        Some(new) => {
            let held = (fns.hold)(org.head, &new, org.identifier_kind, org.identifier)
                .or_else(|| if rule == HeadRule::R2 { (fns.r2_hold)(org.head, &new) } else { None });
            match held {
                Some(reason) => HeadVerdict::Hold { reason, rule, new: Some(new) },
                None => HeadVerdict::Change { rule, new },
            }
        }
    };

    // R0: a defective head. `not_a_name("")` is None, so the empty head is tested on its own.
    let empty = org.head.trim().is_empty();
    if empty || (fns.not_a_name)(org.head).is_some_and(|class| class != HEAD_DRIFT_AWARD_SUMMARY) {
        let rule = if empty { HeadRule::R0e } else { HeadRule::R0n };
        // Fit as `display_spelling` reads it (as published AND collapsed, the v7 review), so R0 never
        // elects a tied cluster that then holds `no_fit` while another had a servable spelling.
        let rank = |cluster: &Cluster<'_>| {
            let fit: Vec<(&str, u64)> = cluster
                .spellings
                .iter()
                .filter(|(spelling, _)| fit_to_serve(spelling, org.head, fns))
                .map(|(spelling, n)| (*spelling, *n))
                .collect();
            (fit.iter().any(|(spelling, _)| !all_caps(spelling)), fit.iter().map(|(_, n)| n).sum::<u64>())
        };
        // `max_by` keeps the LAST of equals, so the key order is reversed to make the smallest win.
        let (key, elected) = tied
            .iter()
            .map(|key| (*key, &clusters[*key]))
            .max_by(|a, b| rank(a.1).cmp(&rank(b.1)).then_with(|| b.0.cmp(a.0)))
            .expect("a voter makes a top cluster");
        // R0 never consults the window; its reading names R0's own pick.
        let reading = org.dated.map(|dated| recent_reading(dated, fns, &head_key, Some(key), voting));
        return counted(serve(rule, elected), reading);
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
        return counted(HeadVerdict::Keep(HeadKeep::D4), reading);
    }
    if head_votes == top {
        return counted(HeadVerdict::Keep(HeadKeep::S1), reading);
    }
    let rule = if unique && top * 20 >= voting * 11 && top >= HEAD_ELECT_TOP_MIN && top >= 2 * head_votes {
        HeadRule::R1
    } else if head_votes <= 1 && unique && top >= HEAD_ELECT_TOP_MIN && top * 4 >= voting {
        HeadRule::R2
    } else {
        return counted(HeadVerdict::Keep(HeadKeep::Bar), reading);
    };
    let mut reading = reading;
    // Rule v7: R1's window, only when the head's own cluster holds 2 or more votes (R2's H ≤ 1 never
    // reads it).
    if rule == HeadRule::R1 && head_votes >= 2 {
        let Some(read) = reading.as_mut() else {
            return counted(HeadVerdict::Hold { reason: HEAD_HOLD_UNDATED, rule, new: None }, None);
        };
        if read.fires {
            match recent_release(org.head, &head_key, tied[0], fns) {
                None => {
                    let new = display_spelling(elected, org.head, fns);
                    return counted(HeadVerdict::Hold { reason: HEAD_HOLD_RECENT, rule, new }, reading);
                }
                Some(why) => read.released = Some(why),
            }
        }
    }
    // Rule v8: the residue holdout — the head names the organization's single newest item. A head the
    // window's exemption already released is released here too (the exemption reads the same keys).
    if let Some(read) = reading.as_mut()
        && read.newest_is_head
        && read.released.is_none()
    {
        match recent_release(org.head, &head_key, tied[0], fns) {
            None => {
                let new = display_spelling(elected, org.head, fns);
                return counted(HeadVerdict::Hold { reason: HEAD_HOLD_RESIDUE, rule, new }, reading);
            }
            Some(why) => read.released = Some(why),
        }
    }
    counted(serve(rule, elected), reading)
}

/// Rule `v7`: the window over `dated` — see [`elect_head`] step 5. `dated` is filtered with the voter
/// rule `votes` is, and keyed by [`HeadElectFns::vote_key`]; `elected_key` is the cluster the rule
/// elects (its newest item is `top_newest`), `voting` V (the undated count is V less the dated voters).
fn recent_reading(
    dated: &[DatedVote<'_>],
    fns: &HeadElectFns,
    head_key: &str,
    elected_key: Option<&str>,
    voting: u64,
) -> RecentReading {
    let mut keys: HashMap<&str, String> = HashMap::new();
    let mut items: HashMap<(String, i64), i64> = HashMap::new();
    let mut dated_voters = 0u64;
    for row in dated {
        if row.spelling.trim().is_empty()
            || (fns.not_a_name)(row.spelling).is_some_and(|class| class != HEAD_DRIFT_AWARD_SUMMARY)
        {
            continue;
        }
        dated_voters += 1;
        let key = keys.entry(row.spelling).or_insert_with(|| (fns.vote_key)(row.spelling)).clone();
        let at = items.entry((key, row.tender)).or_insert(row.at);
        *at = (*at).max(row.at);
    }
    // (at, is_head, tender, key): newest first; on an equal `at` a non-head item ranks newer.
    let mut ranked: Vec<(i64, bool, i64, &str)> =
        items.iter().map(|((key, tender), at)| (*at, key == head_key, *tender, key.as_str())).collect();
    ranked.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then(b.2.cmp(&a.2)).then_with(|| a.3.cmp(b.3)));
    let k = ranked.iter().filter(|item| item.1).count() as u64;
    let h_w = ranked.iter().take(k as usize).filter(|item| item.1).count() as u64;
    let newest = |key: &str| ranked.iter().filter(|item| item.3 == key).map(|item| item.0).max();
    RecentReading {
        head_procs: k,
        head_in_window: h_w,
        head_newest: newest(head_key),
        top_newest: elected_key.and_then(newest),
        newest_is_head: ranked.first().is_some_and(|item| item.1),
        undated: voting.saturating_sub(dated_voters),
        fires: k >= HEAD_RECENT_MIN_PROCEDURES && 3 * h_w >= 2 * k,
        released: None,
    }
}

/// The junk exemption (rule `v7`, narrowed by `v8`): why a head the window (`recent`) or the residue
/// holdout (`residue`) would protect is NOT protected — `unfit` when it is unfit to serve even
/// against itself for any reason but `lowercase` (control, mojibake, long, non_name; a lower-case head
/// is a deliberate brand, 3909's `act legal s.r.o., advokátní kancelář`, the registered current name
/// a v7 release reverted), `digits` when its vote key has a token of ASCII decimal digits only the
/// elected cluster's key lacks (a buyer template appending an address number, a postcode or a NIP, a
/// `LOT 2` prefix). A token the two keys share does not count (`nr 5` on both sides is a rename,
/// held), and a token with a letter does not either (v8: 1846's acronym `CA2BM`, printed on every
/// recent notice, was released by v7's `char::is_numeric` test).
fn recent_release(head: &str, head_key: &str, elected_key: &str, fns: &HeadElectFns) -> Option<&'static str> {
    match (fns.unfit)(head, head) {
        // `unfit` answers `lowercase` before `non_name`: a lower-case non-name head still releases.
        Some("lowercase") if (fns.not_a_name)(head).is_none() => {}
        Some(_) => return Some("unfit"),
        None => {}
    }
    let elected: HashSet<&str> = elected_key.split(' ').collect();
    head_key
        .split(' ')
        .any(|token| !token.is_empty() && token.chars().all(|c| c.is_ascii_digit()) && !elected.contains(token))
        .then_some("digits")
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

/// Rule `v6`: a spelling as it may be served — trimmed, and every run of whitespace one space (the
/// collapse [`org_name_norm`] makes, without its case fold or punctuation strip). A tab or a newline
/// never gets here: [`HeadElectFns::unfit`] refuses a control character first. Other whitespace (a
/// no-break space) becomes a plain space.
fn collapse_whitespace(spelling: &str) -> String {
    spelling.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A spelling the rule may serve in place of `head`: [`HeadElectFns::unfit`] reads it fit as published
/// AND once collapsed (the collapse can make a word-final mojibake shape: `SocietÃ\u{2003}Cooperativa`
/// is fit raw and `SocietÃ Cooperativa` is not). The display and R0's tie-break both read this.
fn fit_to_serve(spelling: &str, head: &str, fns: &HeadElectFns) -> bool {
    (fns.unfit)(spelling, head).is_none() && (fns.unfit)(&collapse_whitespace(spelling), head).is_none()
}

/// The cluster's display spelling, or `None` when no spelling is fit: the FIT spellings (fit as
/// published AND once collapsed, the 2b review: the collapse can make a word-final mojibake shape)
/// collapsed by [`collapse_whitespace`], the counts of those that collapse alike summed;
/// the best of those in [`spelling_order`]; when that winner has no marked letter, the most-mentioned
/// (then byte-wise smallest) fit twin of its case class under [`HeadElectFns::twin_key`] that has one.
fn display_spelling(cluster: &Cluster<'_>, head: &str, fns: &HeadElectFns) -> Option<String> {
    let mut collapsed: BTreeMap<String, u64> = BTreeMap::new();
    for (spelling, n) in cluster.spellings.iter().filter(|(spelling, _)| fit_to_serve(spelling, head, fns)) {
        *collapsed.entry(collapse_whitespace(spelling)).or_default() += *n;
    }
    let fit: Vec<(&str, u64)> = collapsed.iter().map(|(spelling, n)| (spelling.as_str(), *n)).collect();
    let winner = fit.iter().copied().max_by(|a, b| spelling_order(*a, *b))?;
    if marked_letters(winner.0) > 0 {
        return Some(winner.0.to_owned());
    }
    let (case, twin) = (all_caps(winner.0), (fns.twin_key)(winner.0));
    let marked_twin = fit
        .into_iter()
        .filter(|(spelling, _)| {
            marked_letters(spelling) > 0 && all_caps(spelling) == case && (fns.twin_key)(spelling) == twin
        })
        .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(a.0)));
    Some(marked_twin.map_or(winner.0, |(spelling, _)| spelling).to_owned())
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

/// Issue 524 rule `v7`: a page's mention names with their notices, `n` org ids at a time, through
/// `organization_mentions_org` — the census's read (`canonical::org_head_drift_mentions_sql`, left as it is)
/// plus `notice_id`, which sits in the row the walk already fetches.
pub(crate) fn org_name_refresh_mentions_sql(n: usize) -> String {
    format!(
        "SELECT organization_id, name, scheme, notice_id FROM organization_mentions WHERE organization_id IN ({})",
        placeholders(n)
    )
}

/// Issue 524 rule `v7`: the versions `n` notices caused, through `tender_versions_notice`. No GROUP BY
/// (no sorter): the smallest `tender_id` per notice is kept in Rust.
pub(crate) fn org_name_refresh_dates_sql(n: usize) -> String {
    format!(
        "SELECT caused_by_notice_id, tender_id, published_at FROM tender_versions WHERE caused_by_notice_id IN ({})",
        placeholders(n)
    )
}

/// Issue 524 rule `v7`: the indexes the planner and the wet path refuse without, and why each may be
/// absent: the mention walk's and the merge read's (a rebuild's Phase 1), and the date read's
/// (`tender_versions_notice`, a `DEFERRED_TENDER_INDEXES` member a rebuild's fold drops).
const ORG_NAME_REFRESH_INDEXES: [(&str, &str); 3] = [
    ("organization_mentions_org", "a rebuild's Phase 1?"),
    ("organization_merged_identifiers_org", "a rebuild's Phase 1?"),
    ("tender_versions_notice", "a rebuild's fold?"),
];

/// Issue 524 rule `v7`: what one date read found.
#[derive(Default)]
struct NoticeDates {
    /// Notice id → (its procedure: the smallest `tender_id` among its versions, that version's `at`),
    /// for the notices with a usable date.
    dates: HashMap<i64, (i64, i64)>,
    notices: u64,
    version_rows: u64,
}

/// Issue 524 rule `v7`: read the procedures and dates of `notices` ([`org_name_refresh_dates_sql`], in
/// [`IN_CHUNK`] chunks), keeping the smallest `tender_id` per notice (an issue-278 ghost doubles a
/// notice into a second tender), then dropping a notice whose kept `at` is ≤ 0 or later than
/// `horizon` (the job's start plus a day): its mentions count as undated.
async fn read_notice_dates(reader: &turso::Connection, notices: &[i64], horizon: i64) -> turso::Result<NoticeDates> {
    let mut out = NoticeDates { notices: notices.len() as u64, ..Default::default() };
    for chunk in notices.chunks(IN_CHUNK) {
        let params: Vec<Value> = chunk.iter().map(|&id| Value::Integer(id)).collect();
        let mut rows = reader.query(&org_name_refresh_dates_sql(chunk.len()), params).await?;
        while let Some(row) = rows.next().await? {
            out.version_rows += 1;
            let (notice, tender, at) = (int(&row, 0), int(&row, 1), int(&row, 2));
            let kept = out.dates.entry(notice).or_insert((tender, at));
            if tender < kept.0 {
                *kept = (tender, at);
            }
        }
    }
    out.dates.retain(|_, (_, at)| *at > 0 && *at <= horizon);
    Ok(out)
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
    /// Rule `v7`, each `None` when the org was not dated (only the candidates are): k, h_W, the
    /// newest head item and the elected cluster's newest item (`YYYY-MM-DD`, UTC), whether the
    /// organization's newest item is the head's, the voting mentions no date covered, and the junk
    /// exemption's release (`unfit` | `digits`) of a fired window or (rule `v8`) a residue hold.
    pub head_procs: Option<u64>,
    pub head_in_window: Option<u64>,
    pub head_newest: Option<String>,
    pub top_newest: Option<String>,
    pub newest_is_head: Option<bool>,
    pub undated_mentions: Option<u64>,
    pub recent_released: Option<&'static str>,
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
    /// Rule `v7`, two populations (the v7 review). `dated_orgs` and `undated_mentions` count the
    /// population's dated candidates and their voting mentions no usable date covered. The READ counts
    /// `dated_notices` and `dated_version_rows` count what the date reads asked and returned: per page,
    /// for every candidate of the page (a canary below `min_mentions` included), so a notice naming
    /// orgs on two pages is read, and counted, once per page. They size the read; they are no
    /// denominator for the other two.
    pub dated_orgs: u64,
    pub dated_notices: u64,
    pub dated_version_rows: u64,
    pub undated_mentions: u64,
    /// Rule `v7`: the CHANGES (in the population) the junk exemption released from a hold — R1's fired
    /// window, or (rule `v8`) an R1 or R2 residue hold — by why (`unfit`, `digits`), and the first
    /// [`ORG_NAME_REFRESH_RELEASED_LISTED`] of their ids by org id. A release that `serve` then held
    /// (no fit spelling, a bind guard) is no change and is not counted; its entry still reads
    /// `recent_released`.
    pub recent_released: BTreeMap<&'static str, u64>,
    pub recent_released_ids: Vec<i64>,
    /// Rule `v7`: the planned changes whose organization's NEWEST item is a head item, by rule. Since
    /// rule `v8` holds that residue (`hold:residue`, in `by_hold`), these are the ones that still SHIP:
    /// an R0 change (never held) or a change the junk exemption released. Sampled as
    /// `residue:head_newest`.
    pub residue: BTreeMap<&'static str, HeadDriftCount>,
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

/// Issue 524 rule `v7`: the most released ids a plan lists — since the v8 review 500, so the dated
/// re-judge reads EVERY release (v8 adds the residue's releases to v7's window releases, plan 2523's
/// 10); past it the tally still counts them all.
pub const ORG_NAME_REFRESH_RELEASED_LISTED: usize = 500;

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
    /// Rule `v7`: the notice of each named mention, per exact spelling (no extra I/O: the mention
    /// walk's rows carry it).
    notices: HashMap<String, Vec<i64>>,
}

impl PageOrg {
    fn votes(&self) -> Vec<(&str, u64)> {
        self.spellings.iter().map(|(spelling, n)| (spelling.as_str(), *n)).collect()
    }

    fn elect(&self, fns: &HeadElectFns, bind_defect: bool, dated: Option<&[DatedVote<'_>]>) -> HeadElection {
        let org = HeadElectOrg {
            head: &self.head,
            identifier_kind: self.identifier_kind.as_deref(),
            identifier: self.identifier.as_deref(),
            bind_defect,
            dated,
        };
        elect_head_counted(&self.votes(), &org, fns)
    }

    /// Rule `v7`: this org's voting mentions with a usable date (`dates`: notice id → the notice's
    /// procedure and `at`); the undated count is read later, as V less these ([`recent_reading`]).
    fn dated<'s>(&'s self, fns: &HeadElectFns, dates: &HashMap<i64, (i64, i64)>) -> Vec<DatedVote<'s>> {
        let mut out = Vec::new();
        for (spelling, notices) in &self.notices {
            if (fns.not_a_name)(spelling).is_some_and(|class| class != HEAD_DRIFT_AWARD_SUMMARY) {
                continue;
            }
            for notice in notices {
                if let Some(&(tender, at)) = dates.get(notice) {
                    out.push(DatedVote { spelling, tender, at });
                }
            }
        }
        out
    }

    /// Rule `v7`: the distinct notices of this org's voting mentions.
    fn voting_notices<'s>(&'s self, fns: &'s HeadElectFns) -> impl Iterator<Item = i64> + 's {
        self.notices
            .iter()
            .filter(|(spelling, _)| !(fns.not_a_name)(spelling).is_some_and(|class| class != HEAD_DRIFT_AWARD_SUMMARY))
            .flat_map(|(_, notices)| notices.iter().copied())
    }
}

/// One page org's election and what the candidate reads added.
struct Judged {
    at: usize,
    election: HeadElection,
    /// The E2 probe found a bind-defect verdict (re-elections keep it).
    bind_defect: bool,
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
    /// `organization_mentions_org` in [`IN_CHUNK`] id chunks (with their notice ids, rule `v7`), and it
    /// refuses before reading while that index, `organization_merged_identifiers_org` (a rebuild's
    /// Phase 1) or `tender_versions_notice` (a rebuild's fold) is absent. With `org_ids` it reads those
    /// rows by id instead. For the CANDIDATES only — orgs the rule would change or hold — it adds the
    /// identifier-verdict probe (E2, the `ORG_COLUMNS` predicate on the verdicts' primary key), then
    /// (rule `v7`) the dates of every remaining candidate's voting mentions (`read_notice_dates`
    /// through `tender_versions_notice`) and a re-election with them, so `hold:undated` never survives
    /// a finished plan and every candidate entry carries its window; then the merge-survivor read the
    /// census makes, and, for the D4 keeps, the `organization_names` ENG seek. The stop flag is read before every page; a stopped
    /// walk returns an empty plan marked `stopped`.
    ///
    /// This writes nothing: no `organizations` row, no ledger, no change row. The caller stores the
    /// plan as a report.
    pub async fn plan_org_name_refresh(&self, args: OrgNameRefreshArgs<'_>) -> turso::Result<OrgNameRefreshPlan> {
        for (index, why) in ORG_NAME_REFRESH_INDEXES {
            if !self.has_index(index).await? {
                return Err(turso::Error::Error(format!(
                    "refresh-org-names refused: {index} is absent ({why}), so every chunk would scan its \
                     table; run `reindex` first. Nothing was read"
                )));
            }
        }
        // Rule v7: a date more than a day past the job's start is no date.
        let horizon = crate::now_unix() + 86_400;
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
                    notices: HashMap::new(),
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

            // The page's mention names, as the census reads them, with their notices (rule v7).
            let ids: Vec<i64> = page.iter().map(|org| org.id).collect();
            for chunk in ids.chunks(IN_CHUNK) {
                let params: Vec<Value> = chunk.iter().map(|&id| Value::Integer(id)).collect();
                let mut rows = reader.query(&org_name_refresh_mentions_sql(chunk.len()), params).await?;
                while let Some(row) = rows.next().await? {
                    let Some(&i) = at.get(&int(&row, 0)) else { continue };
                    plan.mentions_walked += 1;
                    let org = &mut page[i];
                    org.n += 1;
                    if let Some(name) = opt_text_of(&row, 1).filter(|name| !name.trim().is_empty()) {
                        org.notices.entry(name.clone()).or_default().push(int(&row, 3));
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
                let election = org.elect(&args.fns, false, None);
                judged.push(Judged { at: i, election, bind_defect: false, in_population: !below, merged: false, eng: None });
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
                    j.bind_defect = true;
                    j.election = org.elect(&args.fns, true, None);
                }
            }

            // Rule v7: date every remaining candidate's voting mentions — a change or a hold,
            // `hold:undated` included — and re-elect it with the dates, so every planned entry (and
            // its ledger row) carries them, and (rule v8) the residue is held.
            let asked: HashSet<i64> =
                judged.iter().filter(|j| j.candidate()).flat_map(|j| page[j.at].voting_notices(&args.fns)).collect();
            if judged.iter().any(Judged::candidate) {
                let mut asked: Vec<i64> = asked.into_iter().collect();
                asked.sort_unstable();
                let read = read_notice_dates(&reader, &asked, horizon).await?;
                plan.dated_notices += read.notices;
                plan.dated_version_rows += read.version_rows;
                for j in judged.iter_mut().filter(|j| j.candidate()) {
                    let org = &page[j.at];
                    let dated = org.dated(&args.fns, &read.dates);
                    j.election = org.elect(&args.fns, j.bind_defect, Some(&dated));
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
                if let Some(read) = &j.election.recent {
                    plan.dated_orgs += 1;
                    plan.undated_mentions += read.undated;
                    if let (Some(why), HeadVerdict::Change { .. }) = (read.released, verdict) {
                        *plan.recent_released.entry(why).or_default() += 1;
                        if plan.recent_released_ids.len() < ORG_NAME_REFRESH_RELEASED_LISTED {
                            plan.recent_released_ids.push(org.id);
                        }
                    }
                    // Rule v8 holds the residue; the changes still reading it ship (R0, or released).
                    if let (HeadVerdict::Change { rule, .. }, true) = (verdict, read.newest_is_head) {
                        plan.residue.entry(rule.label()).or_default().add(org.n);
                        let key = sample_key(org.id);
                        let uniform = samples.entry(("residue", "head_newest")).or_default();
                        if uniform.admits(args.samples, key) {
                            uniform.insert(args.samples, key, entry());
                        }
                    }
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
    let recent = j.election.recent;
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
        head_procs: recent.map(|r| r.head_procs),
        head_in_window: recent.map(|r| r.head_in_window),
        head_newest: recent.and_then(|r| r.head_newest).map(crate::rates::civil_date),
        top_newest: recent.and_then(|r| r.top_newest).map(crate::rates::civil_date),
        newest_is_head: recent.map(|r| r.newest_is_head),
        undated_mentions: recent.map(|r| r.undated),
        recent_released: recent.and_then(|r| r.released),
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

/// Issue 524 unit 2b: why a restore's compare-and-set missed — no row (gone), `1` (the row still holds
/// the pair the rename wrote, but its identifier was stripped since), `0` (anything else: renamed
/// again since, stripped or not). Bound `(new_name, new_name_norm, id)`. The re-review: a row renamed
/// again AND stripped is renamed again, so "restore the identifier and re-run" is never advice for it.
pub(crate) const ORG_NAME_REVERT_MISS_SQL: &str =
    "SELECT identifier IS NULL AND name = ? AND name_norm IS ? FROM organizations WHERE id = ?";

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
    /// Ready targets (within `max_groups`) in windows the run never committed — after a stop or a
    /// `window_error`, the failed window's included. 0 on a complete run, and on an aborted one
    /// (whose `ready` reads the same); 0 on a pre-pass stop too, where nothing is known to be ready
    /// (read `prepass_stopped_after`).
    pub left: u64,
    /// A cancel stopped the PRE-PASS after this many of its targets (the targets after a resume's
    /// cursor): nothing was written by this run, and `ready`, `already`, `drifted` and `gone` are
    /// not counted (`None` on every run whose pre-pass finished).
    pub prepass_stopped_after: Option<u64>,
    /// The first [`ORG_NAME_REFRESH_DRIFT_SAMPLES`] drifted or gone targets, by org id.
    pub drift_samples: Vec<OrgNameRefreshDrift>,
    /// [`ORG_NAME_REFRESH_APPLIED_SAMPLES`] committed renames, uniform over the run (the planner's
    /// sample order, the smallest `(org_id × 2654435761) mod 2³²`), listed by org id. A run of that
    /// many renames or fewer lists every one.
    pub applied_samples: Vec<OrgNameRefreshRename>,
    /// The 513 U4 `fold_error` shape (the 2b review): a window that failed AFTER at least one window
    /// of this job id committed ends the run there, and the report comes back Ok with the counts and
    /// samples, so the caller records them before it reports the failure. That covers a recovered job
    /// (`prior_applied` or `resumed_after` set) whose first window of this run fails (the re-review).
    /// Only a job with no committed window at all gets the bare error: nothing was written.
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
    /// The row still holds the pair the rename wrote, but a case review stripped its identifier since,
    /// so its `name_norm` is now an issue-234 bind key: left as it is, unstamped (a restored identifier
    /// lets a re-run take it up). A row renamed again counts `renamed_again`, stripped or not.
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
/// `Err(read)` when `stop`, read between [`IN_CHUNK`]-target chunks, ended it after `read` targets.
async fn read_readiness(
    reader: &turso::Connection,
    fns: &HeadElectFns,
    targets: &[OrgNameRefreshTarget],
    horizon: i64,
    stop: &(dyn Fn() -> bool + Sync),
) -> turso::Result<Result<Vec<Readiness>, u64>> {
    let mut out = Vec::with_capacity(targets.len());
    for (i, chunk) in targets.chunks(IN_CHUNK).enumerate() {
        // The v7 review: dating ~28.7k targets takes a while, so a cancel is read between chunks.
        // `Err(read)`: stopped after `read` targets, nothing written.
        if i > 0 && stop() {
            return Ok(Err(out.len() as u64));
        }
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
                    notices: HashMap::new(),
                },
            );
        }
        drop(rows);
        let mut rows = reader.query(&org_name_refresh_mentions_sql(chunk.len()), ids).await?;
        while let Some(row) = rows.next().await? {
            let Some(org) = orgs.get_mut(&int(&row, 0)) else { continue };
            org.n += 1;
            if let Some(name) = opt_text_of(&row, 1).filter(|name| !name.trim().is_empty()) {
                org.notices.entry(name.clone()).or_default().push(int(&row, 3));
                *org.spellings.entry(name).or_default() += 1;
            }
        }
        drop(rows);
        // The row checks first: gone, already renamed, a moved pre-image or a stripped identifier
        // decide without an election, so only the rest is dated (the v7 review).
        let mut decided: Vec<Option<Readiness>> = Vec::with_capacity(chunk.len());
        for target in chunk {
            let Some(org) = orgs.get(&target.org_id) else {
                decided.push(Some(Readiness::Gone));
                continue;
            };
            let new_norm = org_name_norm(&target.new_name);
            if org.head == target.new_name && org.name_norm.as_deref() == Some(new_norm.as_str()) {
                decided.push(Some(Readiness::Already));
                continue;
            }
            if org.head != target.old_name || org.name_norm != target.old_name_norm {
                let why = if org.head != target.old_name {
                    format!("the head is {:?} now, not the plan's {:?}", org.head, target.old_name)
                } else {
                    format!("the name_norm is {:?} now, not the plan's {:?}", org.name_norm, target.old_name_norm)
                };
                decided.push(Some(Readiness::Drifted(why)));
                continue;
            }
            if org.identifier.is_none() {
                decided.push(Some(Readiness::Drifted("the identifier is NULL now (E1)".to_owned())));
                continue;
            }
            decided.push(None);
        }
        // Rule v7: every remaining target is a change, so every one is dated, as the planner dated it.
        let mut asked: Vec<i64> = chunk
            .iter()
            .zip(&decided)
            .filter(|(_, decided)| decided.is_none())
            .filter_map(|(target, _)| orgs.get(&target.org_id))
            .flat_map(|org| org.voting_notices(fns))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        asked.sort_unstable();
        let dates = read_notice_dates(reader, &asked, horizon).await?.dates;
        for (target, decided) in chunk.iter().zip(decided) {
            if let Some(readiness) = decided {
                out.push(readiness);
                continue;
            }
            let org = &orgs[&target.org_id];
            let identifier = org.identifier.as_deref().unwrap_or_default();
            let dated = org.dated(fns, &dates);
            let mut election = org.elect(fns, false, Some(&dated));
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
                    election = org.elect(fns, true, Some(&dated));
                }
            }
            // A target that now reads `hold:recent` (a newer head procedure since the plan),
            // `hold:residue` (rule v8: its newest item is now the head's) or `hold:undated` is drifted
            // like any other verdict that is not the plan's change.
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
    Ok(Ok(out))
}

/// The ledger's `evidence`: the epoch, the counts the rename was elected by and (rule `v7`) the
/// window's reading — k, h_W and the newest head and elected items (`YYYY-MM-DD`, or `null`). Every
/// value is a number, a date or a fixed ASCII label, so nothing needs escaping.
fn refresh_evidence(rule: &str, election: &HeadElection) -> String {
    let recent = election.recent.unwrap_or_default();
    let number = |n: Option<u64>| n.map_or_else(|| "null".to_owned(), |n| n.to_string());
    let date = |at: Option<i64>| at.map_or_else(|| "null".to_owned(), |at| format!("\"{}\"", crate::rates::civil_date(at)));
    let dated = election.recent.is_some();
    format!(
        "{{\"epoch\":\"{HEAD_ELECTION_EPOCH}\",\"rule\":\"{rule}\",\"voting\":{},\"top\":{},\"head\":{},\"second\":{},\
         \"clusters\":{},\"head_procs\":{},\"head_in_window\":{},\"head_newest\":{},\"top_newest\":{}}}",
        election.voting,
        election.top,
        election.head_votes,
        election.second,
        election.clusters,
        number(dated.then_some(recent.head_procs)),
        number(dated.then_some(recent.head_in_window)),
        date(recent.head_newest),
        date(recent.top_newest),
    )
}

impl Db {
    /// Issue 524 unit 2b: the WET `refresh-org-names` — rename the heads a stored dry plan planned
    /// (ADR-0013's 2026-10-15 amendment), each with a byte-exact pre-image in `org_name_refresh_log`
    /// and one seq-less `organization changed` row.
    ///
    /// The caller has already refused a missing, truncated, other-epoch or other-binary plan (and,
    /// rule `v7`, one holding anything `undated`) and scoped `targets` to `org_ids`. This refuses
    /// while `organization_mentions_org`, `organization_merged_identifiers_org` or
    /// `tender_versions_notice` is absent, as the planner does. Then:
    /// 1. **Pre-pass.** Every target's organization is read and re-elected with `fns` as the planner
    ///    elected it (the verdict probe included). The row checks come first — `gone` when deleted,
    ///    `already` when the row holds the plan's new pair (a replayed window), `drifted` when its
    ///    pre-image moved or its identifier is NULL — and only the targets past them are dated, as the
    ///    planner dated its candidates, and re-elected: `ready` when that re-elects to the plan's rule
    ///    and spelling, else `drifted` (one reading `hold:recent`, `hold:residue` or `hold:undated`
    ///    now among them). Drifted plus gone above `max(2%, 5)` of the targets read aborts the run here, before any
    ///    write. The stop flag is read between [`IN_CHUNK`]-target chunks (not before the first): a
    ///    cancel there ends the run before any write with `prepass_stopped_after` set and none of the
    ///    pre-pass counts (they would be a part's).
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
        for (index, why) in ORG_NAME_REFRESH_INDEXES {
            if !self.has_index(index).await? {
                return Err(turso::Error::Error(format!(
                    "refresh-org-names (wet) refused: {index} is absent ({why}), so the pre-pass would scan its \
                     table; run `reindex` first. Nothing was read or written"
                )));
            }
        }
        // Rule v7: the pre-pass dates the targets past its row checks as the planner dated them.
        let horizon = crate::now_unix() + 86_400;
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
            report.prior_applied = self.org_name_refresh_job_renames(job).await?;
        }

        (args.progress)(0, &format!("pre-pass: re-electing {} planned organization(s)", targets.len()));
        let readiness = {
            let reader = self.reader().await?;
            read_readiness(&reader, &args.fns, &targets, horizon, args.stop).await?
        };
        let readiness = match readiness {
            Ok(readiness) => readiness,
            // A cancel between chunks: nothing written, and none of the pre-pass counts reported —
            // they would describe the part read. The limit is the whole run's, as the summary never
            // prints a pre-pass it did not finish (the v7 follow-up check).
            Err(read) => {
                report.stopped = true;
                report.prepass_stopped_after = Some(read);
                report.drift_limit = org_name_refresh_drift_limit(targets.len() as u64);
                return Ok(report);
            }
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
                // Rolled back. With no window of this job id committed, in this run or an earlier
                // attempt, nothing was written, so the error is the answer; otherwise the counts are.
                Err(e) if report.windows == 0 && report.prior_applied == 0 && report.resumed_after.is_none() => {
                    return Err(e);
                }
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
        report.left = (ready.len() as u64).saturating_sub(report.applied + report.raced);
        report.applied_samples = samples.into_by_org_id();
        Ok(report)
    }

    /// Issue 524 unit 2b re-review: the renames a job id has committed (its `org_name_refresh_log`
    /// rows, a seek on `org_name_refresh_log_job`). A recovered wet job refused, or failing before it
    /// writes, names this count: its dead earlier attempt stored no report.
    pub async fn org_name_refresh_job_renames(&self, job: i64) -> turso::Result<u64> {
        let reader = self.reader().await?;
        let mut rows = reader.query(ORG_NAME_REFRESH_JOB_COUNT_SQL, (Value::Integer(job),)).await?;
        Ok(match rows.next().await? {
            Some(row) => int(&row, 0).max(0) as u64,
            None => 0,
        })
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
                        let mut miss = conn
                            .query(
                                ORG_NAME_REVERT_MISS_SQL,
                                (
                                    Value::Text(row.new_name.clone()),
                                    Value::Text(row.new_name_norm.clone()),
                                    Value::Integer(row.org_id),
                                ),
                            )
                            .await?;
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
        DatedVote, HEAD_HOLD_NO_FIT, HEAD_HOLD_RECENT, HEAD_HOLD_RESIDUE, HEAD_HOLD_UNDATED, HeadElectFns, HeadElectOrg, HeadKeep,
        HeadRule, HeadVerdict, Script, dominant_script, elect_head, elect_head_counted, sample_key,
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
            n if n.starts_with("Awarded to ") || n.starts_with("awarded to ") => Some("award_summary"),
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
        HeadElectOrg { head, identifier_kind: Some("national"), identifier: Some("GB1"), bind_defect: false, dated: None }
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

    /// Rule `v6` (dry plan 2515: 137 of 28,707 planned names carried a whitespace run): the display
    /// spelling is chosen among the fit spellings with their whitespace collapsed, those that collapse
    /// alike counted together, so no change serves a doubled space.
    #[test]
    fn the_display_spelling_collapses_whitespace_and_counts_its_spellings_together() {
        let change = |new: &str| HeadVerdict::Change { rule: HeadRule::R1, new: new.into() };
        // A doubled space joins its clean form: 20 + 3 serve `Acme GmbH` (v5 served the 20 as spelled).
        let merged = [("Acme  GmbH", 20), ("Acme GmbH", 3), ("Head Body", 1)];
        assert_eq!(elect_head(&merged, &org("Head Body"), &FNS), change("Acme GmbH"));
        // The SUM decides: 2 + 2 beat `Acme-GmbH`'s 3. Unsummed, `Acme-GmbH` would win.
        let summed = [("Acme  GmbH", 2), ("Acme GmbH", 2), ("Acme-GmbH", 3), ("Head Body", 1)];
        assert_eq!(elect_head(&summed, &org("Head Body"), &FNS), change("Acme GmbH"));
        // 22609166's tie, its `&` dropped: raw, the doubled space's 0x20 sorts before `&` (0x26) and won
        // the byte tie-break; collapsed, `&` sorts before `C` (0x43), so the intact name is served.
        assert!("Julius Brune GmbH  Co. KG" < "Julius Brune GmbH & Co. KG");
        assert!("Julius Brune GmbH Co. KG" > "Julius Brune GmbH & Co. KG");
        let brune = [("Julius Brune GmbH  Co. KG", 2), ("Julius Brune GmbH & Co. KG", 2), ("Mehrwert Logistik", 1)];
        assert_eq!(elect_head(&brune, &org("Mehrwert Logistik"), &FNS), change("Julius Brune GmbH & Co. KG"));
        // A run with no clean twin is served collapsed (10589338's shape), and so is a leading or
        // trailing space and a no-break space.
        let lone = [("Itä-Suomen  aluehallintovirasto", 3), ("Head Body", 1)];
        assert_eq!(elect_head(&lone, &org("Head Body"), &FNS), change("Itä-Suomen aluehallintovirasto"));
        let edges = [(" Conduent Business Solutions (France) SAS ", 3), ("Head Body", 1)];
        assert_eq!(elect_head(&edges, &org("Head Body"), &FNS), change("Conduent Business Solutions (France) SAS"));
        let nbsp = [("Acme\u{a0}Services", 3), ("Head Body", 1)];
        assert_eq!(elect_head(&nbsp, &org("Head Body"), &FNS), change("Acme Services"));
        // `unfit` reads each spelling as published, BEFORE the collapse: a tab is a control character,
        // so that spelling is neither served nor counted toward its clean form (its 5 still vote).
        // `Acme-Services` 3 beats `Acme Services` 1; read after the collapse, 6 would have won.
        let tab = [("Acme\tServices", 5), ("Acme Services", 1), ("Acme-Services", 3), ("Head Body", 1)];
        assert_eq!(elect_head(&tab, &org("Head Body"), &FNS), change("Acme-Services"));
        // The served name keys as the spellings it stands for (`name_norm` collapses whitespace too).
        for votes in [&lone[..], &edges[..], &nbsp[..]] {
            let served = elect_head(votes, &org("Head Body"), &FNS);
            assert_eq!(
                crate::canonical::org_name_norm(served.new_name().unwrap()),
                crate::canonical::org_name_norm(votes[0].0),
                "{votes:?}"
            );
        }
        // The belt: a display equal to the head is kept, never a change to itself. Under a vote key
        // that reads whitespace (as `head_vote_key` did before the review) the head `Acme GmbH` keys
        // apart from its own `Acme  GmbH` votes, so S1 misses it and R1 elects them; the belt keeps it.
        let raw = HeadElectFns { vote_key: |s: &str| s.to_owned(), ..FNS };
        let itself = [("Acme  GmbH", 5), ("Other Body", 1)];
        assert_eq!(elect_head(&itself, &org("Acme GmbH"), &raw), HeadVerdict::Keep(HeadKeep::S1));
        assert_eq!(elect_head(&itself, &org("Old Body"), &raw), change("Acme GmbH"), "and R1 elects them");
        assert_eq!(super::collapse_whitespace(" a \u{a0} b  c "), "a b c");
        assert_eq!(super::collapse_whitespace("a b"), "a b");
    }

    #[test]
    fn the_bars_and_the_sticky_head() {
        // S1: tied for the top keeps the head byte for byte.
        let tie = [("Alpha", 3), ("Beta", 3)];
        assert_eq!(elect_head(&tie, &org("Beta"), &FNS), HeadVerdict::Keep(HeadKeep::S1));
        // R1's 55% bar, exactly: T x 20 >= V x 11. With H = 2 the 2x bar passes (11 >= 4) and R2 is off
        // (H > 1), so the share alone decides: 11 of 20 is 55% and changes, 11 of 21 is 52.4% and keeps.
        // Rule v7: H >= 2 reads the window, so the change needs dates (here none cover it: `Some(&[])`);
        // without any it fails closed, held `undated`.
        let none: [DatedVote<'_>; 0] = [];
        let dated_none = |head| HeadElectOrg { dated: Some(&none), ..org(head) };
        let at = [("Main", 11), ("Head", 2), ("Other", 7)];
        assert_eq!(elect_head(&at, &dated_none("Head"), &FNS), r1("Main"), "11/20 = 55%");
        assert_eq!(
            elect_head(&at, &org("Head"), &FNS),
            HeadVerdict::Hold { reason: HEAD_HOLD_UNDATED, rule: HeadRule::R1, new: None },
            "undated"
        );
        let under = [("Main", 11), ("Head", 2), ("Other", 8)];
        assert_eq!(elect_head(&under, &org("Head"), &FNS), HeadVerdict::Keep(HeadKeep::Bar), "11/21 < 55%");
        // R1's 2x bar on its own: 55% passes, twice the head does not.
        let half = [("Main", 11), ("Head", 9)];
        assert_eq!(elect_head(&half, &org("Head"), &FNS), HeadVerdict::Keep(HeadKeep::Bar), "11/20 but 11 < 2 x 9");
        let twice = [("Main", 12), ("Head", 6), ("Other", 2)];
        assert_eq!(elect_head(&twice, &dated_none("Head"), &FNS), r1("Main"), "60% and twice the head");
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

    // ---- Rule v7: the R1 recency hold (the recency decision's tests 1-12).

    const DAY: i64 = 86_400;

    /// `n` dated mentions of `spelling`, one per tender from `tender` on, a day apart from `day` on.
    fn items(spelling: &'static str, tender: i64, day: i64, n: i64) -> Vec<DatedVote<'static>> {
        (0..n).map(|i| DatedVote { spelling, tender: tender + i, at: (day + i) * DAY }).collect()
    }

    /// The votes the dated rows make, one per row (every mention dated).
    fn votes_of<'a>(rows: &[DatedVote<'a>]) -> Vec<(&'a str, u64)> {
        let mut counts: std::collections::BTreeMap<&'a str, u64> = std::collections::BTreeMap::new();
        for row in rows {
            *counts.entry(row.spelling).or_default() += 1;
        }
        counts.into_iter().collect()
    }

    /// Elect `head` from `rows`, every mention dated.
    fn dated(rows: &[DatedVote<'_>], head: &str) -> super::HeadElection {
        let org = HeadElectOrg { dated: Some(rows), ..org(head) };
        elect_head_counted(&votes_of(rows), &org, &FNS)
    }

    fn r1(new: &str) -> HeadVerdict {
        HeadVerdict::Change { rule: HeadRule::R1, new: new.into() }
    }

    fn recent(new: Option<&str>) -> HeadVerdict {
        HeadVerdict::Hold { reason: HEAD_HOLD_RECENT, rule: HeadRule::R1, new: new.map(str::to_owned) }
    }

    /// Rule v8's residue hold.
    fn residue(rule: HeadRule, new: &str) -> HeadVerdict {
        HeadVerdict::Hold { reason: HEAD_HOLD_RESIDUE, rule, new: Some(new.to_owned()) }
    }

    /// 1. A rename (2186's shape): the head's 4 procedures are all newer than every item of the
    /// cluster R1 elects, so the all-history majority would put the FORMER name back. Held, with the
    /// spelling R1 would have served.
    #[test]
    fn a_rename_is_held_recent() {
        let rows = [items("Old Name Agency", 1, 1, 10), items("New Name Agency", 101, 100, 4)].concat();
        let e = dated(&rows, "New Name Agency");
        assert_eq!(e.verdict, recent(Some("Old Name Agency")));
        let read = e.recent.expect("dated");
        assert_eq!((read.head_procs, read.head_in_window, read.newest_is_head, read.fires), (4, 4, true, true));
        assert_eq!((read.head_newest, read.top_newest, read.undated), (Some(103 * DAY), Some(10 * DAY), 0));
    }

    /// 2. 445: the head's 4 mentions are ONE procedure (three notices of tender 500), with elected
    /// items both older and newer than its first notice: k = 1, under the floor, so the window never
    /// fires. Rule v8: that one procedure's newest notice is still the organization's newest item (plan
    /// 2523 read 445 so: the head 2026-07-17, the elected cluster 2026-07-06), so the residue holdout
    /// holds it; one newer elected item and it changes.
    #[test]
    fn one_procedure_of_three_notices_is_one_item() {
        let mut rows = items("Kadea Berlin Logistik", 1, 1, 10);
        rows.extend(items("Kadea Berlin Logistik", 20, 120, 2));
        for day in [100, 101, 138, 138] {
            rows.push(DatedVote { spelling: "Kadea", tender: 500, at: day * DAY });
        }
        let e = dated(&rows, "Kadea");
        assert_eq!(e.head_votes, 4);
        let read = e.recent.unwrap();
        assert_eq!((read.head_procs, read.head_in_window, read.fires), (1, 1, false));
        assert_eq!(read.head_newest, Some(138 * DAY), "an item's time is its newest row's");
        assert!(read.newest_is_head);
        assert_eq!(e.verdict, residue(HeadRule::R1, "Kadea Berlin Logistik"));
        rows.push(DatedVote { spelling: "Kadea Berlin Logistik", tender: 30, at: 140 * DAY });
        assert_eq!(dated(&rows, "Kadea").verdict, r1("Kadea Berlin Logistik"));
    }

    /// 3. 31558903: H = 1, a single newest stray. The window never reaches it; the reading says the
    /// newest item is the head's, and rule v8 holds that residue — the decision's accepted cost (a
    /// missed fix, listed in 548, never a regression). Undated, it changes as before.
    #[test]
    fn a_single_newest_stray_is_held_residue() {
        let rows = [items("University of Sheffield", 1, 1, 16), items("Sheffield Hallam University", 99, 200, 1)].concat();
        let e = dated(&rows, "Sheffield Hallam University");
        assert_eq!(e.verdict, residue(HeadRule::R1, "University of Sheffield"));
        let read = e.recent.unwrap();
        assert!(read.newest_is_head && !read.fires && read.released.is_none());
        assert_eq!(elect_head(&votes_of(&rows), &org("Sheffield Hallam University"), &FNS), r1("University of Sheffield"));
    }

    /// 4. 3406: two head sections of one notice and its twin in the same tender are one item: k = 1.
    /// With an elected item newer than it, it changes (as the canary expects); were the head's one
    /// procedure the newest item, rule v8 would hold it `residue`.
    #[test]
    fn two_sections_and_a_twin_of_one_tender_are_one_item() {
        let mut rows = items("Staatliches Hochbauamt Freiburg", 1, 1, 6);
        rows.push(DatedVote { spelling: "Hochbauamt Freiburg", tender: 165, at: 200 * DAY });
        rows.push(DatedVote { spelling: "Hochbauamt Freiburg", tender: 165, at: 201 * DAY });
        rows.push(DatedVote { spelling: "Staatliches Hochbauamt Freiburg", tender: 170, at: 202 * DAY });
        let e = dated(&rows, "Hochbauamt Freiburg");
        assert_eq!((e.head_votes, e.recent.unwrap().head_procs), (2, 1));
        assert_eq!(e.verdict, r1("Staatliches Hochbauamt Freiburg"));
        rows.pop();
        assert_eq!(dated(&rows, "Hochbauamt Freiburg").verdict, residue(HeadRule::R1, "Staatliches Hochbauamt Freiburg"));
    }

    /// 5. 10603's shape as the recency decision dated it: a head item and an elected item at the same
    /// instant — the tie ranks the elected item newer. Here the head's two items and one elected item
    /// share the newest day: ranked head-first both newest would be the head's and it would hold;
    /// ranked as the rule ranks, W holds one, and (rule v8) the newest item is no head item either.
    /// (Plan 2523 dated the real 10603 fully: its head names the newest procedure, so it holds
    /// `recent`, and the v8 canary expects that.)
    #[test]
    fn on_an_equal_instant_the_elected_item_ranks_newer() {
        let mut rows = items("Visagino energija", 1, 1, 7);
        rows.push(DatedVote { spelling: "Visagino energija", tender: 30, at: 50 * DAY });
        rows.push(DatedVote { spelling: "VE Visagino", tender: 31, at: 50 * DAY });
        rows.push(DatedVote { spelling: "VE Visagino", tender: 32, at: 50 * DAY });
        let e = dated(&rows, "VE Visagino");
        let read = e.recent.unwrap();
        assert_eq!((read.head_procs, read.head_in_window, read.fires, read.newest_is_head), (2, 1, false, false));
        assert_eq!(e.verdict, r1("Visagino energija"));
    }

    /// 6. The floor: k = 2 both newest holds; k = 2 with one newer elected item changes; k = 1 never
    /// fires the window — here its one item is the newest, so rule v8's residue holdout holds it.
    #[test]
    fn the_window_floor_is_two_procedures() {
        let both = [items("Old Name Agency", 1, 1, 8), items("New Name Agency", 101, 100, 2)].concat();
        assert_eq!(dated(&both, "New Name Agency").verdict, recent(Some("Old Name Agency")));
        let mut newer = both.clone();
        newer.push(DatedVote { spelling: "Old Name Agency", tender: 300, at: 300 * DAY });
        assert_eq!(dated(&newer, "New Name Agency").verdict, r1("Old Name Agency"));
        let mut one = items("Old Name Agency", 1, 1, 8);
        one.push(DatedVote { spelling: "New Name Agency", tender: 101, at: 100 * DAY });
        one.push(DatedVote { spelling: "New Name Agency", tender: 101, at: 101 * DAY });
        let e = dated(&one, "New Name Agency");
        assert_eq!((e.head_votes, e.recent.unwrap().head_procs, e.recent.unwrap().fires), (2, 1, false));
        assert_eq!(e.verdict, residue(HeadRule::R1, "Old Name Agency"));
    }

    /// 7. The share: k = 6 with 2 elected items inside W holds (4 of 6), with 3 it changes (3 of 6;
    /// the third the newest item, so rule v8's residue holdout does not hold it either).
    #[test]
    fn the_window_needs_two_thirds_of_its_items() {
        let mut rows = items("Old Name Agency", 1, 1, 12);
        rows.extend(items("New Name Agency", 101, 100, 6));
        rows.push(DatedVote { spelling: "Old Name Agency", tender: 201, at: 104 * DAY });
        rows.push(DatedVote { spelling: "Old Name Agency", tender: 202, at: 102 * DAY });
        let e = dated(&rows, "New Name Agency");
        assert_eq!((e.recent.unwrap().head_procs, e.recent.unwrap().head_in_window), (6, 4));
        assert_eq!(e.verdict, recent(Some("Old Name Agency")));
        rows.push(DatedVote { spelling: "Old Name Agency", tender: 203, at: 106 * DAY });
        let e = dated(&rows, "New Name Agency");
        assert_eq!((e.recent.unwrap().head_in_window, e.recent.unwrap().newest_is_head), (3, false));
        assert_eq!(e.verdict, r1("Old Name Agency"));
    }

    /// 8. 2762: a third cluster's newer items count against the head.
    #[test]
    fn a_third_cluster_counts_against_the_head() {
        let rows = [
            items("Diputacion Provincial", 1, 1, 10),
            items("Presidencia Diputacion", 101, 100, 3),
            items("Pleno Provincial", 201, 103, 2),
        ]
        .concat();
        let e = dated(&rows, "Presidencia Diputacion");
        assert_eq!((e.recent.unwrap().head_procs, e.recent.unwrap().head_in_window), (3, 1));
        assert_eq!(e.verdict, r1("Diputacion Provincial"));
        // Without the third cluster, the head owns the newest three: held.
        let two = [items("Diputacion Provincial", 1, 1, 10), items("Presidencia Diputacion", 101, 100, 3)].concat();
        assert_eq!(dated(&two, "Presidencia Diputacion").verdict, recent(Some("Diputacion Provincial")));
    }

    /// 9. 178: the abbreviation until 2023-12, the full head on every procedure since 2024. The 2a.3
    /// "neutral" change is the FORMER usage: held.
    #[test]
    fn plaine_vallee_is_held_recent() {
        let full = "Communauté d'agglomération Plaine Vallée";
        let mut rows = Vec::new();
        for i in 0..66 {
            rows.push(DatedVote { spelling: "CA Plaine Vallée", tender: 1 + i / 3, at: (19_000 + i) * DAY });
        }
        for i in 0..32 {
            rows.push(DatedVote { spelling: full, tender: 100 + i / 3, at: (19_800 + i) * DAY });
        }
        let e = dated(&rows, full);
        assert_eq!((e.top, e.head_votes), (66, 32));
        assert_eq!(e.verdict, recent(Some("CA Plaine Vallée")));
    }

    /// 10. Junk: a recently repeated head unfit against itself (a control character) or carrying an
    /// all-digit token the elected key lacks (a NIP) is released to R1, and the release is read. Rule
    /// v8: a lower-case head is a brand, not junk (3909) — held.
    #[test]
    fn a_junk_head_is_released_to_r1() {
        let control = [items("Acme Group", 1, 1, 8), items("Acme\u{8c}Services", 101, 100, 3)].concat();
        let e = dated(&control, "Acme\u{8c}Services");
        assert_eq!(e.verdict, r1("Acme Group"));
        assert_eq!((e.recent.unwrap().fires, e.recent.unwrap().released), (true, Some("unfit")));
        let lower = [items("Acme Group", 1, 1, 8), items("acme services", 101, 100, 3)].concat();
        let e = dated(&lower, "acme services");
        assert_eq!(e.verdict, recent(Some("Acme Group")));
        assert_eq!((e.recent.unwrap().fires, e.recent.unwrap().released), (true, None));
        let nip = [items("Acme Polska", 1, 1, 8), items("Acme Polska NIP 5250001009", 101, 100, 2)].concat();
        let e = dated(&nip, "Acme Polska NIP 5250001009");
        assert_eq!(e.verdict, r1("Acme Polska"));
        assert_eq!(e.recent.unwrap().released, Some("digits"));
    }

    /// 11. Undated: no dates with H ≥ 2 holds `undated` (it fails closed); with H = 1 R1 changes;
    /// `Some(&[])` reads k = 0 and changes.
    #[test]
    fn r1_without_dates_fails_closed_only_where_the_window_applies() {
        let votes = [("Old Name Agency", 8), ("New Name Agency", 2)];
        assert_eq!(
            elect_head(&votes, &org("New Name Agency"), &FNS),
            HeadVerdict::Hold { reason: HEAD_HOLD_UNDATED, rule: HeadRule::R1, new: None }
        );
        let stray = [("Old Name Agency", 8), ("New Name Agency", 1)];
        assert_eq!(elect_head(&stray, &org("New Name Agency"), &FNS), r1("Old Name Agency"));
        let none: [DatedVote<'_>; 0] = [];
        let empty = HeadElectOrg { dated: Some(&none), ..org("New Name Agency") };
        let e = elect_head_counted(&votes, &empty, &FNS);
        assert_eq!(e.verdict, r1("Old Name Agency"));
        assert_eq!((e.recent.unwrap().head_procs, e.recent.unwrap().undated), (0, 10));
    }

    /// 12. R0 never consults dates, and R2 only for rule v8's residue holdout: undated, both still
    /// change; dated with the head's item newest, R2 holds `residue` and R0 still changes.
    #[test]
    fn r0_never_consults_dates_and_r2_only_for_the_residue() {
        let votes = [("Kalmar kommun", 4)];
        assert_eq!(elect_head(&votes, &org(""), &FNS).rule(), Some(HeadRule::R0e));
        assert_eq!(elect_head(&votes, &org(""), &FNS).kind(), "change");
        let stray = [("Main Body", 3), ("A Body", 2), ("B Body", 2), ("C Body", 2), ("D Body", 2), ("Head Body", 1)];
        assert_eq!(elect_head(&stray, &org("Head Body"), &FNS).rule(), Some(HeadRule::R2));
        assert_eq!(elect_head(&stray, &org("Head Body"), &FNS).kind(), "change");
        let rows = [items("Main Body", 1, 1, 3), items("Head Body", 101, 100, 1)].concat();
        let org_ = HeadElectOrg { dated: Some(&rows), ..org("Head Body") };
        assert_eq!(elect_head(&stray, &org_, &FNS), residue(HeadRule::R2, "Main Body"));
        // Not vacuous: an R0 head whose vote key OWNS the newest k = 2 items — the window fires, and
        // the newest item is the head's — still changes. The placeholder `s.o.` keys as `so`, and so
        // does the voter `S O`.
        let r0 = [items("Vergabekammer Nord", 1, 1, 4), items("S O", 101, 100, 2)].concat();
        let e = dated(&r0, "s.o.");
        assert_eq!(e.recent.map(|r| (r.head_procs, r.fires, r.newest_is_head)), Some((2, true, true)), "both fire");
        assert_eq!(e.verdict, HeadVerdict::Change { rule: HeadRule::R0n, new: "Vergabekammer Nord".into() });
    }

    /// The digit exemption compares against the ELECTED key (the v7 review): a digit token the two keys
    /// share is no junk — `nr 5` on both sides is a rename and is held — while a NEW digit token beside
    /// a shared one releases.
    #[test]
    fn a_shared_digit_token_is_no_junk() {
        let shared = [items("Szkola Podstawowa nr 5", 1, 1, 8), items("Szkola Podstawowa nr 5 im Jana Pawla", 101, 100, 2)]
            .concat();
        let e = dated(&shared, "Szkola Podstawowa nr 5 im Jana Pawla");
        assert_eq!(e.verdict, recent(Some("Szkola Podstawowa nr 5")));
        assert_eq!(e.recent.unwrap().released, None);
        let added = [items("Szkola Podstawowa nr 5", 1, 1, 8), items("Szkola Podstawowa nr 5 ul Lipowa 12", 101, 100, 2)]
            .concat();
        let e = dated(&added, "Szkola Podstawowa nr 5 ul Lipowa 12");
        assert_eq!(e.verdict, r1("Szkola Podstawowa nr 5"));
        assert_eq!(e.recent.unwrap().released, Some("digits"));
    }

    // ---- Rule v8: the residue holdout and the narrowed junk exemption.

    /// 3740262's shape (plan 2523: k = 47, h_W = 4, the head printed on the newest procedure): R1 with
    /// H ≥ 2 whose window stays silent is held `residue` with the display spelling it would serve.
    /// One elected item newer than the head's newest and it changes.
    #[test]
    fn the_residue_holdout_holds_r1_when_the_head_names_the_newest_item() {
        let mut rows = items("Junta Gobierno Fuengirola", 1, 1, 20);
        for (i, day) in [2, 4, 6, 8, 10, 30].into_iter().enumerate() {
            rows.push(DatedVote { spelling: "Ayuntamiento Fuengirola", tender: 101 + i as i64, at: day * DAY });
        }
        let e = dated(&rows, "Ayuntamiento Fuengirola");
        let read = e.recent.unwrap();
        assert_eq!((read.head_procs, read.head_in_window, read.fires, read.newest_is_head), (6, 1, false, true));
        assert_eq!(e.verdict, residue(HeadRule::R1, "Junta Gobierno Fuengirola"));
        assert_eq!(read.released, None);
        rows.push(DatedVote { spelling: "Junta Gobierno Fuengirola", tender: 50, at: 31 * DAY });
        assert_eq!(dated(&rows, "Ayuntamiento Fuengirola").verdict, r1("Junta Gobierno Fuengirola"));
    }

    /// Rule v8: no dates, an empty date list, or no dated head item never hold `residue` — an
    /// undated H ≤ 1 change changes as before.
    #[test]
    fn the_residue_holdout_needs_a_dated_head_item() {
        let stray = [("Old Name Agency", 8), ("New Name Agency", 1)];
        assert_eq!(elect_head(&stray, &org("New Name Agency"), &FNS), r1("Old Name Agency"));
        let none: [DatedVote<'_>; 0] = [];
        let empty = HeadElectOrg { dated: Some(&none), ..org("New Name Agency") };
        let e = elect_head_counted(&stray, &empty, &FNS);
        assert_eq!((e.verdict, e.recent.unwrap().newest_is_head), (r1("Old Name Agency"), false));
        // The head's one mention undated (its notice had no usable version): the elected items alone.
        let elected = items("Old Name Agency", 1, 1, 8);
        let partial = HeadElectOrg { dated: Some(&elected), ..org("New Name Agency") };
        let e = elect_head_counted(&stray, &partial, &FNS);
        let read = e.recent.unwrap();
        assert_eq!((e.verdict, read.newest_is_head, read.undated), (r1("Old Name Agency"), false, 1));
    }

    /// Rule v8: the junk exemption releases a residue hold as it releases a recent one — an address
    /// number or a NIP the elected key lacks, on R2 (H = 1) and on R1 — and the release is read; an
    /// acronym with a digit inside (1846's `CA2BM`) is no number, held by the residue holdout and by
    /// the window alike; and a lower-case head (3909) is protected by both.
    #[test]
    fn the_junk_exemption_releases_a_residue_hold_on_whole_numbers_only() {
        let r2 = |head: &'static str| {
            let rows = [items("Main Body", 1, 1, 3), items(head, 101, 100, 1)].concat();
            let votes = [("Main Body", 3), ("A Body", 2), ("B Body", 2), ("C Body", 2), ("D Body", 2), (head, 1)];
            let org_ = HeadElectOrg { dated: Some(&rows), ..org(head) };
            elect_head_counted(&votes, &org_, &FNS)
        };
        let address = r2("Head Body ul Lipowa 12");
        assert_eq!(address.verdict, HeadVerdict::Change { rule: HeadRule::R2, new: "Main Body".into() });
        assert_eq!(address.recent.unwrap().released, Some("digits"));
        let acronym = r2("CA2BM Head Body");
        assert_eq!(acronym.verdict, residue(HeadRule::R2, "Main Body"));
        assert_eq!(acronym.recent.unwrap().released, None);
        let lower = r2("head body");
        assert_eq!(lower.verdict, residue(HeadRule::R2, "Main Body"));

        let nip = [items("Acme Polska", 1, 1, 8), items("Acme Polska NIP 5250001009", 101, 100, 1)].concat();
        let e = dated(&nip, "Acme Polska NIP 5250001009");
        assert_eq!((e.head_votes, e.recent.unwrap().newest_is_head), (1, true));
        assert_eq!((e.verdict, e.recent.unwrap().released), (r1("Acme Polska"), Some("digits")));
        let ca2bm = [items("Deux Baies Montreuillois", 1, 1, 8), items("CA2BM Deux Baies", 101, 100, 3)].concat();
        let e = dated(&ca2bm, "CA2BM Deux Baies");
        assert_eq!(e.recent.unwrap().fires, true);
        assert_eq!((e.verdict, e.recent.unwrap().released), (recent(Some("Deux Baies Montreuillois")), None));
    }

    /// The v8 review: the `lowercase` carve-out keeps releasing a lower-case NON-NAME head — `unfit`
    /// answers `lowercase` before `non_name`, so the exemption asks `not_a_name` too. A lower-case
    /// award summary (it votes, so it is no R0 head) is junk and released; a lower-case brand is not.
    #[test]
    fn a_lower_case_non_name_head_is_still_released() {
        let summary = [items("Acme Group", 1, 1, 8), items("awarded to acme services", 101, 100, 3)].concat();
        let e = dated(&summary, "awarded to acme services");
        assert_eq!((FNS.unfit)("awarded to acme services", "awarded to acme services"), Some("lowercase"));
        assert_eq!((e.recent.unwrap().fires, e.recent.unwrap().released), (true, Some("unfit")));
        assert_eq!(e.verdict, r1("Acme Group"));
        let brand = [items("Acme Group", 1, 1, 8), items("acme services", 101, 100, 3)].concat();
        assert_eq!(dated(&brand, "acme services").verdict, recent(Some("Acme Group")));
    }

    /// Issue 524 unit 2a: the planner's candidate reads must seek, not scan: the scoped page by rowid,
    /// the verdict probe and the ENG satellite on their tables' primary keys, and (rule v7) the mention
    /// read with notices and the notice dates through their indexes.
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
            // Rule v7: the mention read carries `notice_id` through the same index, and the date read
            // seeks `tender_versions_notice`.
            (
                "mentions with notices",
                super::org_name_refresh_mentions_sql(crate::canonical::IN_CHUNK),
                (1..=crate::canonical::IN_CHUNK as i64).map(turso::Value::Integer).collect(),
                "organization_mentions_org",
            ),
            (
                "notice dates",
                super::org_name_refresh_dates_sql(crate::canonical::IN_CHUNK),
                (1..=crate::canonical::IN_CHUNK as i64).map(turso::Value::Integer).collect(),
                "tender_versions_notice",
            ),
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
