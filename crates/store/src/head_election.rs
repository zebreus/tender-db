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
/// (`CA2BM`). `v9` (`## Rule v9 — decided`, panel wf_5cca11b1-622, after plan 2526's dated re-judge
/// failed its change:R1, change:R2 and released bars): R1 and R2 changes are HELD on five pure tests
/// of the head and the display spelling ([`HeadElectFns::serve_hold`]: [`HEAD_HOLD_REVIEW_BODY`],
/// [`HEAD_HOLD_ORGAN`], [`HEAD_HOLD_CODE_ADDED`], [`HEAD_HOLD_ADDRESS_ADDED`],
/// [`HEAD_HOLD_CAPS_VARIANT`]); the junk exemption's `digits` release is narrowed twice (a digit
/// glued to its neighbour in the elected key is the name's own, `SMD 3` → `smd3`; and the elected
/// name must be the head minus junk, at most one other word lost), a withheld release returning to
/// its hold and counted `release_withheld`; and the plan's uniform samples are salted with this
/// epoch ([`ORG_NAME_REFRESH_SAMPLE_SALT`]), so a re-judge's fresh samples are not the predictable
/// continuation of the last plan's. `v10` (`## Rule v10 — decided`, from the dated re-judges of plans
/// 2523, 2526 and 2527: pooled, 4 of 29 change:R1 items on a head still in print read wrong, against 1
/// of 61 on a head not printed for 3+ years): an R1 change whose head was printed within
/// [`HEAD_CURRENT_DAYS`] of the plan's `as_of` ([`HeadElectOrg::as_of`]) is HELD
/// [`HEAD_HOLD_CURRENT`], unless the junk exemption releases it; the plan stores its `as_of`, and the
/// wet pre-pass re-elects with it. `v11` (`## Rule v11 — decided`, panel wf_5d3b6fa8-186, after the
/// capped run 2555's spot check read 1 revert and 3 wrongs on the lowest ids): an R1 change still a
/// change after `head_current` is HELD [`HEAD_HOLD_TOP_STALE`] when a RIVAL cluster votes (one other
/// than the elected and the head's) and the elected spelling's newest dated item is older than
/// [`HEAD_TOP_STALE_DAYS`] before `as_of` (undated counts stale), unless the head was already released;
/// two serve holds join the five ([`HEAD_HOLD_DOUBLED`], [`HEAD_HOLD_SEAT_DROPPED`]) and a bind guard
/// joins `head_change_hold`'s ([`HEAD_HOLD_PLACEHOLDER_ID`]); plan entries gain the diagnostic
/// [`RecentReading::rival_newest`]; and a wet run may drop the stale-top targets
/// ([`OrgNameRefreshApplyArgs::exclude_stale_top`], the decision's pre-built fallback). `v12` (issue
/// 552 `## Unit 2 — decided`, after plan 2543's re-judge read all five wrong `unfit` releases on heads
/// whose only defect is a line break): a CONTROL-ONLY head ([`head_control_only`]: unfit only through
/// control whitespace) is released `unfit` from a `recent`, `residue` or `head_current` hold only when
/// the elected key's tokens sit inside one of its lines' keys or equal a contiguous run of whole lines
/// ([`head_lines_guard`]), else the release is withheld [`HEAD_WITHHELD_LINES`] and the hold stands;
/// and such a withheld head whose every line break is a WRAP ([`head_wraps`]), and whose collapsed form
/// the junk exemption would not release, changes to itself with its whitespace collapsed, rule
/// [`HeadRule::R0w`] (no recency, residue, `head_current`, `top_stale` or serve hold asked; the bind
/// guards are), whose vote key is the head's, so it stays in its own cluster and under its hold. [`HeadElectFns::family`] joins the injected functions for the wrap test.
pub const HEAD_ELECTION_EPOCH: &str = "524u2-v12";

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

/// Issue 524 rule `v9`: the serve hold when R1 or R2 would serve a procurement review body's name
/// (a review chamber or court) over a head that is not review-like — another entity's procedures
/// naming the chamber that heard a complaint (22028982 `Universitätsklinikum Freiburg` →
/// `Vergabekammer BW`). `crosswalk::head_serve_hold`'s first test.
pub const HEAD_HOLD_REVIEW_BODY: &str = "review_body";

/// Issue 524 rule `v9`: the serve hold when R1 or R2 would serve a name led by a contracting ORGAN
/// (`Junta de Gobierno …`, `Alcaldía …`, `Pleno …`) over a head that is not (4065575). Second.
pub const HEAD_HOLD_ORGAN: &str = "organ";

/// Issue 524 rule `v9`: the serve hold when R1 or R2 would serve the head's own words plus a code
/// (`Commune de Chiconi (976)`, 17116440). Third.
pub const HEAD_HOLD_CODE_ADDED: &str = "code_added";

/// Issue 524 rule `v9`: the serve hold when R1 or R2 would serve a name with a number appended after
/// a comma or semicolon (an address or seat, 8069375's `…, Ljubljana, Kolodvorska 11`), not a Czech or
/// Slovak school's. Fourth.
pub const HEAD_HOLD_ADDRESS_ADDED: &str = "address_added";

/// Issue 524 rule `v9`: the serve hold when R1 or R2 would serve an ALL-CAPS respelling of a
/// mixed-case head's same words (16508165). Fifth.
pub const HEAD_HOLD_CAPS_VARIANT: &str = "caps_variant";

/// Issue 524 rule `v10` (`## Rule v10 — decided`): the hold reason when R1 would replace a head still
/// in PRINT — its newest dated item ([`RecentReading::head_newest`]) within [`HEAD_CURRENT_DAYS`] before
/// the plan's `as_of` ([`HeadElectOrg::as_of`]). A head still printed is usually current, so a change
/// there tends to be cosmetic, a qualifier tail, an abbreviation or another entity on a shared
/// identifier (plan 2527's four change:R1 wrongs: 4605761, 13924072, 22529369, 23399855); a head not
/// printed for 3+ years is the outdated name the refresh exists to fix. Asked after the serve holds
/// ([`HEAD_SERVE_HOLDS`]), so a change v9 holds keeps its v9 reason, and after [`HEAD_HOLD_RECENT`],
/// [`HEAD_HOLD_RESIDUE`] and [`HEAD_HOLD_UNDATED`]. The junk exemption releases it as it releases those
/// two (a head it already released there ships: canary 3042). R0 and R2 are never held by it; an R1
/// change with no dated head item (no reading, or no `head_newest`) follows rule `v7`.
pub const HEAD_HOLD_CURRENT: &str = "head_current";

/// Issue 524 rule `v10`: the [`HEAD_HOLD_CURRENT`] window, 3 years — a head whose newest dated item is
/// at or after `as_of − HEAD_CURRENT_DAYS × 86,400` is current.
pub const HEAD_CURRENT_DAYS: i64 = 1_095;

/// Issue 524 rule `v11` (`## Rule v11 — decided`): the serve hold when R1 or R2 would serve a DOUBLED
/// name — its words (`crosswalk::head_serve_hold`'s `T`) are X then X again, X with a word of 3 or more
/// chars — over a head that is not doubled itself (1918631 `Instituto Foral de Bienestar Social` →
/// `Instituto Foral de Bienestar Social - Instituto Foral de Bienestar Social`). Sixth, after
/// [`HEAD_HOLD_CAPS_VARIANT`].
pub const HEAD_HOLD_DOUBLED: &str = "doubled";

/// Issue 524 rule `v11`: the serve hold when R1 or R2 would serve the head with its SEAT dropped — the
/// head's words are the display's (2 or more) plus a tail of 2–3 words with no digit led by the
/// preposition `w`, `we`, `v`, `ve` or `vo`, followed by whitespace in the head (5350675 `Ośrodek
/// Pomocy Społecznej w Kluczborku` → `Ośrodek Pomocy Społecznej`, a generic name). An agent tail
/// (`Sandefjord kommune v/VOIS`) still changes. Seventh, after [`HEAD_HOLD_DOUBLED`].
pub const HEAD_HOLD_SEAT_DROPPED: &str = "seat_dropped";

/// Issue 524 rule `v11`: the bind guard (`crosswalk::head_change_hold`'s last, for every rule) when the
/// organization's identifier is a PLACEHOLDER — `KEINEANGABE`, `UNBEKANNT` or `NICHTVORHANDEN` with
/// any trailing digits, any letter case, or all ASCII zeros: no identity votes behind it (22831984
/// `KEINEANGABE7`, served `Kapitel 2` → an AOK). Asked before R2's initialism hold and the serve holds.
pub const HEAD_HOLD_PLACEHOLDER_ID: &str = "placeholder_id";

/// Issue 524 rule `v11`: the hold reason when an R1 change, still a change after the serve holds and
/// [`HEAD_HOLD_CURRENT`], has a RIVAL — a voting cluster other than the elected one and the head's
/// (`clusters > 1 + (head_votes > 0)`) — and a STALE top: the elected cluster's newest dated item
/// ([`RecentReading::top_newest`]) before `as_of − HEAD_TOP_STALE_DAYS × 86,400` ([`head_top_stale`];
/// no reading, or no `top_newest`, is stale: it fails closed). Both failures that sank the capped run
/// 2555 need a rival: a superseding third name (1276213) and a revert to the head's name in another
/// spelling (1556789). A head the window, the residue holdout or `head_current` already released
/// (`read.released` is `Some`) ships (3042); the junk exemption is NOT asked, so it mints no release.
/// R0 and R2 are never held by it.
pub const HEAD_HOLD_TOP_STALE: &str = "top_stale";

/// Issue 524 rule `v11`: the [`HEAD_HOLD_TOP_STALE`] age, one year — an elected cluster whose newest
/// dated item is before `as_of − HEAD_TOP_STALE_DAYS × 86,400` is stale. The judge's labels hold alike
/// at every T from 45 to 625 days; 365 leaves 260 days to the youngest stale wrong (1345526).
pub const HEAD_TOP_STALE_DAYS: i64 = 365;

/// Issue 524 rule `v9`: the serve holds, in [`HeadElectFns::serve_hold`]'s order — the plan's summary
/// names each, at zero too. Rule `v11` adds [`HEAD_HOLD_DOUBLED`] and [`HEAD_HOLD_SEAT_DROPPED`].
pub const HEAD_SERVE_HOLDS: [&str; 7] = [
    HEAD_HOLD_REVIEW_BODY,
    HEAD_HOLD_ORGAN,
    HEAD_HOLD_CODE_ADDED,
    HEAD_HOLD_ADDRESS_ADDED,
    HEAD_HOLD_CAPS_VARIANT,
    HEAD_HOLD_DOUBLED,
    HEAD_HOLD_SEAT_DROPPED,
];

/// Issue 524 rule `v11`: whether an elected cluster's newest dated item, `top_newest` (unix seconds),
/// is STALE against a plan started at `as_of`: before `as_of − HEAD_TOP_STALE_DAYS × 86,400`, or
/// unknown (`None` fails closed). [`elect_head`]'s [`HEAD_HOLD_TOP_STALE`] reads it, and so does the
/// wet pre-pass's `exclude_stale_top` drift.
pub fn head_top_stale(top_newest: Option<i64>, as_of: i64) -> bool {
    top_newest.is_none_or(|at| at < as_of.saturating_sub(HEAD_TOP_STALE_DAYS * 86_400))
}

/// Issue 524 rule `v11`: [`head_top_stale`] read off a STORED plan entry's `top_newest` (`YYYY-MM-DD`,
/// UTC) — the `exclude_stale_top` filter's plan-time drop. The day is read as its first instant, so a
/// top printed on the cutoff's own day is stale (the side that drops: a filter fails closed), and a
/// missing or malformed date is stale too. The wet pre-pass then re-reads the exact instant.
pub fn head_top_stale_date(top_newest: Option<&str>, as_of: i64) -> bool {
    head_top_stale(top_newest.and_then(crate::rates::day_number).map(|day| day * 86_400), as_of)
}

/// Issue 524 rule `v12` (issue 552): the `release_withheld` reason when the junk exemption would have
/// released a CONTROL-ONLY head `unfit` ([`head_control_only`]) and the lines guard
/// ([`head_lines_guard`]) stops it: the elected key is neither inside one of the head's lines nor a
/// contiguous run of whole lines, so the elected name is not the head's own words cut at a break
/// (22289866 `Vergabekammer Rheinland-Pfalz\nbei dem Ministerium …` → a name without the Land; 22737299
/// → a truncated `… Bauüberwac - im Verkehrswesen GmbH`). The `recent`, `residue` or `head_current`
/// hold stands, unless every break is a wrap and the collapsed head would stay held: then
/// [`HeadRule::R0w`].
pub const HEAD_WITHHELD_LINES: &str = "lines";

/// Issue 524 rule `v12` (issue 552, `## Unit 2 — decided` point 3): the connector words a line before
/// a break may END in for the break to be a wrap ([`head_line_wrap`]) — a word with no `.`, compared
/// lower-cased: German, English, Romance, Polish, Greek, Dutch, Czech and Slovak particles a name runs
/// on after.
pub const HEAD_WRAP_CONNECTORS: [&str; 38] = [
    "und", "and", "de", "del", "della", "dei", "degli", "di", "du", "des", "der", "den", "of", "for", "für", "fur", "i",
    "z", "w", "ze", "we", "e", "ed", "et", "y", "και", "του", "της", "των", "en", "van", "voor", "u", "na", "do", "od",
    "a", "o",
];

/// Issue 524 rule `v12` (issue 552, `## Unit 2 — decided` point 1): the head with every whitespace run
/// collapsed to one space when the head is CONTROL-ONLY — unfit against itself only through control
/// whitespace — else `None`. All three must hold:
/// - [`HeadElectFns::unfit`] reads the head against itself `control`;
/// - the collapsed form `c` carries no control character and no U+FFFD (so the only control
///   characters were whitespace: a line break, a tab);
/// - `unfit(c, c)` is `None`, or is `lowercase` while [`HeadElectFns::not_a_name`] reads `c` as no
///   class (the fitness test [`HeadRule::R0w`] asks of `c` too).
///
/// Every other unfit head (control plus `long`, control plus mojibake, a C1 control) is released
/// `unfit` as before; a control-plus-lowercase head goes to the lines guard instead of bypassing rule
/// v8's lower-case protection unasked. 22354266 (`… S.C.p.A.\nper il tramite …`, 315 characters
/// collapsed) is `long` and not asked.
pub fn head_control_only(head: &str, fns: &HeadElectFns) -> Option<String> {
    if (fns.unfit)(head, head) != Some("control") {
        return None;
    }
    let collapsed = collapse_whitespace(head);
    collapsed_fit(&collapsed, fns).then_some(collapsed)
}

/// [`head_control_only`]'s fitness test of the collapsed head, and [`HeadRule::R0w`]'s.
fn collapsed_fit(collapsed: &str, fns: &HeadElectFns) -> bool {
    !collapsed.chars().any(|c| c.is_control() || c == '\u{FFFD}')
        && match (fns.unfit)(collapsed, collapsed) {
            None => true,
            Some("lowercase") => (fns.not_a_name)(collapsed).is_none(),
            Some(_) => false,
        }
}

/// Issue 524 rule `v12`: a head's LINES — split on exactly the characters that are both control and
/// whitespace (`\t`, `\n`, `\x0b`, `\x0c`, `\r`, `\x85`), each trimmed, empty ones dropped.
fn head_lines(head: &str) -> Vec<&str> {
    head.split(|c: char| c.is_control() && c.is_whitespace()).map(str::trim).filter(|line| !line.is_empty()).collect()
}

/// Issue 524 rule `v12` (issue 552, `## Unit 2 — decided` point 2): the LINES GUARD on a control-only
/// head's `unfit` release — `true` (released) when the elected cluster's vote key `elected_key`, as a
/// token set (split on spaces), is a subset of one line's [`HeadElectFns::vote_key`] tokens, or equals
/// the union of the tokens of a contiguous run of whole lines; `false` (withheld,
/// [`HEAD_WITHHELD_LINES`]) otherwise, and for an empty elected key. So the elected name may be the
/// head's first line (14770 `Landkreis Saalekreis\nDomplatz 9\n06217 Merseburg` → `Landkreis
/// Saalekreis`) or its lines rejoined, never a splice of words from several lines or a foreign wording.
pub fn head_lines_guard(head: &str, elected_key: &str, fns: &HeadElectFns) -> bool {
    if elected_key.is_empty() {
        return false;
    }
    let elected: HashSet<&str> = elected_key.split(' ').collect();
    let keys: Vec<String> = head_lines(head).into_iter().map(fns.vote_key).collect();
    let lines: Vec<HashSet<&str>> = keys.iter().map(|key| key.split(' ').collect()).collect();
    if lines.iter().any(|line| elected.is_subset(line)) {
        return true;
    }
    (0..lines.len()).any(|from| {
        let mut run: HashSet<&str> = HashSet::new();
        lines[from..].iter().any(|line| {
            run.extend(line.iter().copied());
            run == elected
        })
    })
}

/// Issue 524 rule `v12` (issue 552, `## Unit 2 — decided` point 3): whether the break between line
/// `a` and the next line `b` of a head is a WRAP — one name broken mid-phrase. `a` must not end in a
/// legal form ([`HeadElectFns::family`] names none on its tail: `Sp. z o.o.`, `S.C.p.A.` and the other
/// multi-token forms included) and must not end in `,`, `;` or `:`; and one of these holds:
/// - `a`'s last word ends in a hyphen (`-` or `–`) with no space before it (`Planungs-`; a spaced
///   ` -` is no wrap);
/// - `a`'s last word has no `.` and is a connector ([`HEAD_WRAP_CONNECTORS`]);
/// - `b` starts with a lower-case letter.
pub fn head_line_wrap(a: &str, b: &str, fns: &HeadElectFns) -> bool {
    let a = a.trim_end();
    if (fns.family)(a).is_some() || a.ends_with([',', ';', ':']) {
        return false;
    }
    let last = a.split_whitespace().last().unwrap_or_default();
    let hyphen = last.chars().count() > 1 && last.ends_with(['-', '–']);
    let connector = !last.contains('.') && HEAD_WRAP_CONNECTORS.contains(&last.to_lowercase().as_str());
    hyphen || connector || b.chars().next().is_some_and(char::is_lowercase)
}

/// Issue 524 rule `v12`: whether a head has at least one line break and EVERY break is a wrap
/// ([`head_line_wrap`] over each pair of consecutive [`head_lines`]) — [`HeadRule::R0w`]'s shape test.
/// 22661109 `Szkoła Podstawowa nr 13\nz Oddziałami …` wraps; 22737299's second line ends in `GmbH`
/// and 22299001's `Stadt Remagen\nBürgermeister …` starts a person's line, so neither does.
pub fn head_wraps(head: &str, fns: &HeadElectFns) -> bool {
    let lines = head_lines(head);
    lines.len() >= 2 && lines.windows(2).all(|pair| head_line_wrap(pair[0], pair[1], fns))
}

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
    /// Rule `v11`, a DIAGNOSTIC no verdict reads: the newest `at` of any item of a cluster other than
    /// the elected one and the head's (a rival's), `None` when no rival item is dated. Emitted so issue
    /// 548 can measure the direct supersede test (`rival_newest > top_newest`) the v11 decision rejected
    /// as a gate.
    pub rival_newest: Option<i64>,
    /// The organization's newest item is a head item (rule `v8`: R1 and R2 hold [`HEAD_HOLD_RESIDUE`]
    /// on it). `false` when no item is dated, or none of the head's is.
    pub newest_is_head: bool,
    /// Voting mentions the dates did not cover.
    pub undated: u64,
    /// k ≥ [`HEAD_RECENT_MIN_PROCEDURES`] and 3·h_W ≥ 2·k.
    pub fires: bool,
    /// A hold the junk exemption released — the window fired on an R1 change (H ≥ 2, `recent`), (rule
    /// `v8`) the newest item is the head's on an R1 or R2 change (`residue`), or (rule `v10`) the head
    /// was printed within [`HEAD_CURRENT_DAYS`] of `as_of` on an R1 change (`head_current`): `unfit`
    /// (the head is unfit to serve even against itself, for any reason but `lowercase`) or `digits`
    /// (its vote key has an all-digit token the elected key lacks: an address number, a postcode, a
    /// NIP, `LOT 2`'s `2` — since rule `v9` one not glued to its neighbour in the elected key, and
    /// only where the elected name drops at most one other word of the head).
    pub released: Option<&'static str>,
    /// Rule `v9`: a `digits` release v8 would have made and v9 withholds, so the `recent`, `residue`
    /// or (rule `v10`) `head_current` hold stands: `glue` (every such number is glued to a
    /// neighbouring head word in the elected key: 4295185's `SMD 3` → `Smd3`, the name's own digit) or
    /// `junk_only` (the elected name also drops more than one word of 3+ letters the head prints:
    /// 9814's owner and street); rule `v12`, an `unfit` release of a control-only head the lines guard
    /// stops: [`HEAD_WITHHELD_LINES`] (on a hold, or on the [`HeadRule::R0w`] change it led to).
    pub release_withheld: Option<&'static str>,
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
    /// Rule `v12` (issue 552): the WRAP COLLAPSE — a control-only head ([`head_control_only`]) whose
    /// `unfit` release the lines guard withheld ([`HEAD_WITHHELD_LINES`]) and whose every line break
    /// is a wrap ([`head_wraps`]) changes to itself with its whitespace collapsed (22661109
    /// `Szkoła Podstawowa nr 13\nz Oddziałami …` → one line), unless a bind guard
    /// ([`HeadElectFns::hold`]) names a reason, which holds it as for R0n. No recency, residue,
    /// `head_current`, `top_stale` or serve hold is asked. Its vote key is the head's
    /// (`crosswalk::head_vote_key` collapses whitespace), so the written head stays in its own cluster;
    /// and it is made only when the junk exemption would not release the collapsed head (no bare number
    /// the elected key lacks, v8's `digits`), so the holds that protected it keep standing on later
    /// plans. Otherwise the `lines` hold stands.
    R0w,
}

impl HeadRule {
    pub const fn label(self) -> &'static str {
        match self {
            HeadRule::R0e => "R0e",
            HeadRule::R0n => "R0n",
            HeadRule::R1 => "R1",
            HeadRule::R2 => "R2",
            HeadRule::R0w => "R0w",
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
    /// (with the display spelling, `None` when none is fit) or [`HEAD_HOLD_UNDATED`] (`None`), for
    /// R1 and R2 (rule `v8`) [`HEAD_HOLD_RESIDUE`] (with the display spelling), for R1 and R2 (rule
    /// `v9`) a serve hold's ([`HeadElectFns::serve_hold`]: [`HEAD_SERVE_HOLDS`]), for R1 (rule `v10`)
    /// [`HEAD_HOLD_CURRENT`], or for R1 (rule `v11`) [`HEAD_HOLD_TOP_STALE`] — with the spelling it would
    /// have served. Rule `v11`'s bind guard [`HEAD_HOLD_PLACEHOLDER_ID`] is one of [`HeadElectFns::hold`]'s.
    /// Rule `v12`: a bind guard's reason on an [`HeadRule::R0w`] wrap collapse, with the collapsed head.
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
/// `crosswalk::head_spelling_unfit`, `crosswalk::head_change_hold`, `crosswalk::head_r2_hold`,
/// `crosswalk::head_twin_key`, (rule `v9`) `crosswalk::head_serve_hold` and (rule `v12`)
/// `crosswalk::head_tail_family` — one value, to the dry planner and the wet pre-pass alike, so they
/// cannot elect differently.
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
    /// `(old, new, identifier_kind, identifier)`: why a change is held for review, or `None` — the bind
    /// guards (`family`, `gb_family`, `consortium_removed`, `nhs_check` and, since rule `v11`,
    /// [`HEAD_HOLD_PLACEHOLDER_ID`]), asked of every rule's change.
    pub hold: fn(&str, &str, Option<&str>, Option<&str>) -> Option<&'static str>,
    /// Issue 524 unit 2a.1, `(old, new)`: why an R2 change is held as a shorthand of the head it would
    /// replace (since unit 2a.2, an initialism of a run of its words), or `None`. Asked of R2 changes
    /// only, after [`HeadElectFns::hold`]: R1's majority is not second-guessed.
    pub r2_hold: fn(&str, &str) -> Option<&'static str>,
    /// Issue 524 unit 2a.2: the key two spellings are TWINS under — lower-cased, diacritics folded,
    /// every non-alphanumeric character dropped (`crosswalk::head_twin_key`). The display order
    /// prefers the twin with more marked letters, and only among twins.
    pub twin_key: fn(&str) -> String,
    /// Issue 524 rule `v9`, `(old, new, r2)`: why an R1 or R2 change is HELD on the two names alone,
    /// or `None` — a review body's name served for another entity, an organ for its entity, a code
    /// added to the head's words, a number added after a comma or semicolon (an address, whatever
    /// else the name adds), an ALL-CAPS respelling of a mixed-case head, and (rule `v11`) a doubled
    /// name or the head with its seat dropped
    /// (`crosswalk::head_serve_hold`; [`HEAD_SERVE_HOLDS`] in its order). Asked after
    /// [`HeadElectFns::hold`] and [`HeadElectFns::r2_hold`], never for R0 (a defective head always
    /// changes); the `recent`, `residue` and `undated` holds decide before it. `r2` is the rule. Pure:
    /// no DB read, so the plan and the pre-pass agree by construction.
    pub serve_hold: fn(&str, &str, bool) -> Option<&'static str>,
    /// Issue 524 rule `v12` (issue 552): the legal form a LINE ends in, or `None`
    /// (`crosswalk::head_tail_family`: the line's last words joined, the multi-token forms `Sp. z
    /// o.o.` and `S.C.p.A.` included) — [`head_line_wrap`] reads no wrap after a line that ends in its
    /// legal form. Not [`OrgNameRefreshArgs::family`], which reads a whole head's family for the plan's
    /// family moves.
    pub family: fn(&str) -> Option<&'static str>,
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
    /// Rule `v10`: the plan's `as_of` (unix seconds, the planner's start), read only with `dated`: an
    /// R1 change whose newest dated head item is at or after `as_of − HEAD_CURRENT_DAYS × 86,400` holds
    /// [`HEAD_HOLD_CURRENT`]. The planner passes its own start and the wet pre-pass the STORED plan's,
    /// so a head printed after the plan reads `head_current` at wet time and drifts.
    pub as_of: i64,
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
///      `CA2BM`) the elected key lacks. Rule `v9` narrows `digits` twice: a number glued in the
///      elected key to the head word before or after it (`prev + t` or `t + next` is an elected
///      token) does not count — the name's own digit, 4295185's `SMD 3` → `smd3` —; and the elected
///      key may lack at most ONE head word of 3+ chars with no ASCII digit, so the elected name is the
///      head minus junk (one lost word lets an expanded acronym through: `SDIS 49` → `Service
///      départemental …`), not a different wording (9814 drops the owner and the street). A release
///      either test stops is WITHHELD: the hold stands, read `release_withheld` (`glue` |
///      `junk_only`).
/// 6. **R2**: H ≤ 1, the top is unique, T ≥ [`HEAD_ELECT_TOP_MIN`], and T × 4 ≥ V (25%).
/// 7. Otherwise KEEP (`bar`).
///
/// **The residue holdout** (rule `v8`): an R1 or R2 change (5 or 6, past the window) whose
/// organization's single newest item is a head item (`newest_is_head`) is HELD `residue`
/// ([`HEAD_HOLD_RESIDUE`], `new` the display spelling) unless the junk exemption releases it. Only
/// with dates: an undated R1 with H ≥ 2 holds `undated` before it, and an undated H ≤ 1 change (no
/// dates, or `Some(&[])`) changes as before. R0 is never held.
///
/// **The serve holds** (rule `v9`): an R1 or R2 change that reaches `serve` with a fit display
/// spelling, past the bind guards and R2's initialism hold, is HELD when
/// [`HeadElectFns::serve_hold`] names a reason ([`HEAD_SERVE_HOLDS`]), with the spelling it would
/// serve. R0 is never asked.
///
/// **The current-head hold** (rule `v10`): an R1 change that comes out of `serve` a CHANGE, whose
/// dated reading has a `head_newest` at or after `as_of − HEAD_CURRENT_DAYS × 86,400`
/// ([`HeadElectOrg::as_of`]), is HELD `head_current` ([`HEAD_HOLD_CURRENT`]) with the spelling it would
/// serve — unless the junk exemption releases it (counted `released`, as for `recent` and `residue`;
/// a release it withholds is read `release_withheld`), or the window or the residue holdout already
/// released the head. R0 and R2 are never held by it; an undated R1 (no reading, or no dated head
/// item) follows rule `v7`.
///
/// **The stale-top hold** (rule `v11`): an R1 change that comes out of `serve` AND `head_current` a
/// CHANGE is HELD `top_stale` ([`HEAD_HOLD_TOP_STALE`]) with the spelling it would serve when a RIVAL
/// votes (`clusters > 1 + (head_votes > 0)`: a cluster other than the elected one and the head's, so
/// with H = 0 two clusters are a rival) and the top is STALE ([`head_top_stale`]: the reading's
/// `top_newest` before `as_of − HEAD_TOP_STALE_DAYS × 86,400`; no reading or no `top_newest` is stale)
/// — unless the window, the residue holdout or `head_current` already released the head
/// (`read.released` is `Some`). It never asks the junk exemption, so it mints no release. R0 and R2
/// are never held by it. The serve holds' order (rule `v11`): the bind guards (with
/// [`HEAD_HOLD_PLACEHOLDER_ID`], every rule), R2's initialism hold, then the seven serve holds.
///
/// **The lines guard and the wrap collapse** (rule `v12`, issue 552): wherever the junk exemption is
/// asked (the window, the residue holdout, `head_current`), a CONTROL-ONLY head ([`head_control_only`])
/// is released `unfit` only when [`head_lines_guard`] passes — the elected key inside one line's key,
/// or equal to a contiguous run of whole lines; otherwise the release is withheld
/// ([`HEAD_WITHHELD_LINES`], read `release_withheld`) and the hold stands, unless the head has a break
/// and every break is a wrap ([`head_wraps`]) and the junk exemption would not release the collapsed
/// head (it must stay held on later plans): then it CHANGES to itself with its whitespace collapsed,
/// rule [`HeadRule::R0w`] (a bind guard's reason holds that change, as for R0n), and no later hold
/// (`head_current`, `top_stale`) or serve hold is asked. Every other unfit head releases as before.
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
                .or_else(|| if rule == HeadRule::R2 { (fns.r2_hold)(org.head, &new) } else { None })
                // Rule v9: the serve holds, for R1 and R2 only — R0's defective head always changes.
                .or_else(|| match rule {
                    HeadRule::R1 | HeadRule::R2 => (fns.serve_hold)(org.head, &new, rule == HeadRule::R2),
                    // R0w never reaches `serve` (`wrap_collapse` decides it).
                    HeadRule::R0e | HeadRule::R0n | HeadRule::R0w => None,
                });
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
                Junk::Released(why) => read.released = Some(why),
                held => {
                    // Rule v9: a release v8 would have made returns to the hold, read as withheld.
                    if let Junk::Withheld(why) = held {
                        read.release_withheld = Some(why);
                        // Rule v12: a lines-withheld head whose every break is a wrap: R0w.
                        if let Some(verdict) = wrap_collapse(why, org, &head_key, tied[0], fns) {
                            return counted(verdict, reading);
                        }
                    }
                    let new = display_spelling(elected, org.head, fns);
                    return counted(HeadVerdict::Hold { reason: HEAD_HOLD_RECENT, rule, new }, reading);
                }
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
            Junk::Released(why) => read.released = Some(why),
            held => {
                if let Junk::Withheld(why) = held {
                    read.release_withheld = Some(why);
                    if let Some(verdict) = wrap_collapse(why, org, &head_key, tied[0], fns) {
                        return counted(verdict, reading);
                    }
                }
                let new = display_spelling(elected, org.head, fns);
                return counted(HeadVerdict::Hold { reason: HEAD_HOLD_RESIDUE, rule, new }, reading);
            }
        }
    }
    let verdict = serve(rule, elected);
    // Rule v10: an R1 change on a head still in print — after the serve holds, so a change v9 holds
    // keeps its reason. A head the window or the residue holdout already released ships; otherwise the
    // junk exemption is asked as they ask it.
    if let (HeadRule::R1, HeadVerdict::Change { new, .. }) = (rule, &verdict)
        && let Some(read) = reading.as_mut()
        && read.released.is_none()
        && read.head_newest.is_some_and(|at| at >= org.as_of.saturating_sub(HEAD_CURRENT_DAYS * 86_400))
    {
        match recent_release(org.head, &head_key, tied[0], fns) {
            Junk::Released(why) => read.released = Some(why),
            held => {
                if let Junk::Withheld(why) = held {
                    read.release_withheld = Some(why);
                    if let Some(verdict) = wrap_collapse(why, org, &head_key, tied[0], fns) {
                        return counted(verdict, reading);
                    }
                }
                let new = Some(new.clone());
                return counted(HeadVerdict::Hold { reason: HEAD_HOLD_CURRENT, rule, new }, reading);
            }
        }
    }
    // Rule v11: an R1 change with a rival cluster on a stale top — after `head_current`, so every v10
    // hold keeps its reason. A head already released (window, residue or head_current) ships; the junk
    // exemption is not asked. No reading, or no `top_newest`, is stale: it fails closed.
    if let (HeadRule::R1, HeadVerdict::Change { new, .. }) = (rule, &verdict)
        && reading.as_ref().is_none_or(|read| read.released.is_none())
        && clusters.len() as u64 > 1 + u64::from(head_votes > 0)
        && head_top_stale(reading.as_ref().and_then(|read| read.top_newest), org.as_of)
    {
        let new = Some(new.clone());
        return counted(HeadVerdict::Hold { reason: HEAD_HOLD_TOP_STALE, rule, new }, reading);
    }
    counted(verdict, reading)
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
    // Rule v11's diagnostic: the newest item of a cluster neither the elected one nor the head's.
    let rival_newest =
        ranked.iter().filter(|item| item.3 != head_key && Some(item.3) != elected_key).map(|item| item.0).max();
    RecentReading {
        head_procs: k,
        head_in_window: h_w,
        head_newest: newest(head_key),
        top_newest: elected_key.and_then(newest),
        rival_newest,
        newest_is_head: ranked.first().is_some_and(|item| item.1),
        undated: voting.saturating_sub(dated_voters),
        fires: k >= HEAD_RECENT_MIN_PROCEDURES && 3 * h_w >= 2 * k,
        released: None,
        release_withheld: None,
    }
}

/// The junk exemption's answer for a head the window (`recent`), the residue holdout (`residue`) or
/// (rule `v10`) the current-head hold (`head_current`) would protect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Junk {
    /// Not junk: the hold stands.
    No,
    /// Junk (`unfit` | `digits`): released, the change goes on to `serve`, read `released`.
    Released(&'static str),
    /// Rule `v9`: a `digits` release v8 would have made and v9 does not (`glue` | `junk_only`), or (rule
    /// `v12`) an `unfit` release of a control-only head the lines guard stops ([`HEAD_WITHHELD_LINES`]):
    /// the hold stands, read `release_withheld` — for `lines`, unless [`wrap_collapse`] answers.
    Withheld(&'static str),
}

/// Rule `v12` (issue 552): the WRAP COLLAPSE of a head whose release was withheld `why` — `Some` only
/// for [`HEAD_WITHHELD_LINES`] on a control-only head ([`head_control_only`], whose collapsed form
/// passes the fitness test) with a break and every break a wrap ([`head_wraps`]), and whose collapsed
/// form would itself stay held: the junk exemption ([`recent_release`]) asked of it against the same
/// `head_key` (the collapsed head keys as the raw one) and `elected_key` does not release it. A
/// collapsed head with a bare number the elected key lacks (`Szkoła Podstawowa nr 13\nw Tychach`
/// against `Szkoła Podstawowa w Tychach`) would be released `digits` at the next plan and change R1 to
/// the very spelling the lines guard withheld, so its `lines` hold stands instead (the unit 3 review).
/// Then a change to the collapsed head, rule [`HeadRule::R0w`], or a hold with it when a bind guard
/// ([`HeadElectFns::hold`]) names a reason, as for R0n. No other hold is asked.
fn wrap_collapse(
    why: &'static str,
    org: &HeadElectOrg<'_>,
    head_key: &str,
    elected_key: &str,
    fns: &HeadElectFns,
) -> Option<HeadVerdict> {
    if why != HEAD_WITHHELD_LINES || !head_wraps(org.head, fns) {
        return None;
    }
    let collapsed = head_control_only(org.head, fns)?;
    // The written head must keep the hold the raw head had: never a two-step route around the guard.
    if matches!(recent_release(&collapsed, head_key, elected_key, fns), Junk::Released(_)) {
        return None;
    }
    Some(match (fns.hold)(org.head, &collapsed, org.identifier_kind, org.identifier) {
        Some(reason) => HeadVerdict::Hold { reason, rule: HeadRule::R0w, new: Some(collapsed) },
        None => HeadVerdict::Change { rule: HeadRule::R0w, new: collapsed },
    })
}

/// The junk exemption (rule `v7`, narrowed by `v8` and `v9`, asked by `v10` too): whether a head the
/// window (`recent`), the residue holdout (`residue`) or the current-head hold (`head_current`) would
/// protect is NOT protected — `unfit` when it is unfit to serve
/// even against itself for any reason but `lowercase` (control, mojibake, long, non_name; a lower-case
/// head is a deliberate brand, 3909's `act legal s.r.o., advokátní kancelář`, the registered current
/// name a v7 release reverted), `digits` when its vote key has a token of ASCII decimal digits only
/// the elected cluster's key lacks (a buyer template appending an address number, a postcode or a NIP,
/// a `LOT 2` prefix). A token the two keys share does not count (`nr 5` on both sides is a rename,
/// held), and a token with a letter does not either (v8: 1846's acronym `CA2BM`, printed on every
/// recent notice, was released by v7's `char::is_numeric` test).
///
/// Rule `v9` (plan 2526's `digits` releases read 5 wrong of 26) narrows `digits` twice; a release
/// either stops is [`Junk::Withheld`]:
/// - **glue**: a number does not count when the elected key holds it glued to the head-key token
///   before or after it (`prev + t` or `t + next`): the name's own digit (4295185 `SMD 3` →
///   `Smd3`, keyed `smd3`). When every number is glued, `glue`.
/// - **junk-only**: the elected key may lack at most one head-key token of 3 or more chars with no
///   ASCII digit — the elected name is the head minus its junk, one lost word allowed so an expanded
///   acronym still releases (`SDIS 49` → `Service départemental …`). More is a different wording, not
///   a stripped template (9814's head names the owner, his town and street; plan 2526's Croatian
///   obrt cohort): `junk_only`.
///
/// Rule `v12` (issue 552) narrows `unfit` for a CONTROL-ONLY head ([`head_control_only`]: unfit only
/// through control whitespace): released only when [`head_lines_guard`] passes on the elected key, else
/// [`Junk::Withheld`] `lines` ([`HEAD_WITHHELD_LINES`]). Every other unfit head releases as before.
fn recent_release(head: &str, head_key: &str, elected_key: &str, fns: &HeadElectFns) -> Junk {
    match (fns.unfit)(head, head) {
        // `unfit` answers `lowercase` before `non_name`: a lower-case non-name head still releases.
        Some("lowercase") if (fns.not_a_name)(head).is_none() => {}
        // Rule v12: a head whose only defect is control whitespace releases through the lines guard.
        Some("control") if head_control_only(head, fns).is_some() => {
            return if head_lines_guard(head, elected_key, fns) {
                Junk::Released("unfit")
            } else {
                Junk::Withheld(HEAD_WITHHELD_LINES)
            };
        }
        Some(_) => return Junk::Released("unfit"),
        None => {}
    }
    let elected: HashSet<&str> = elected_key.split(' ').collect();
    let tokens: Vec<&str> = head_key.split(' ').collect();
    // v8's digits: a whole number of the head key the elected key lacks.
    let numbers: Vec<usize> = (0..tokens.len())
        .filter(|&at| {
            let token = tokens[at];
            !token.is_empty() && token.chars().all(|c| c.is_ascii_digit()) && !elected.contains(token)
        })
        .collect();
    if numbers.is_empty() {
        return Junk::No;
    }
    // v9 glue: the number glued to its neighbour is an elected token.
    let glued = |at: usize| {
        let token = tokens[at];
        (at > 0 && elected.contains(format!("{}{token}", tokens[at - 1]).as_str()))
            || tokens.get(at + 1).is_some_and(|next| elected.contains(format!("{token}{next}").as_str()))
    };
    if numbers.iter().all(|&at| glued(at)) {
        return Junk::Withheld("glue");
    }
    // v9 junk-only: at most one lost word.
    let lost: HashSet<&str> = tokens
        .iter()
        .copied()
        .filter(|t| t.chars().count() >= 3 && !t.chars().any(|c| c.is_ascii_digit()) && !elected.contains(t))
        .collect();
    if lost.len() > 1 { Junk::Withheld("junk_only") } else { Junk::Released("digits") }
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
    /// `R0n` `R1` `R2`, rule `v12`'s `R0w`).
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
    /// exemption's release (`unfit` | `digits`) of a fired window, (rule `v8`) a residue hold or
    /// (rule `v10`) a `head_current` hold — `head_newest` is what `head_current` reads.
    pub head_procs: Option<u64>,
    pub head_in_window: Option<u64>,
    pub head_newest: Option<String>,
    pub top_newest: Option<String>,
    /// Rule `v11`, a diagnostic no verdict reads ([`RecentReading::rival_newest`], `YYYY-MM-DD`): the
    /// newest dated item of a cluster other than the elected one and the head's.
    pub rival_newest: Option<String>,
    pub newest_is_head: Option<bool>,
    pub undated_mentions: Option<u64>,
    pub recent_released: Option<&'static str>,
    /// Rule `v9`: the `digits` release v9 withheld from this hold (`glue` | `junk_only`), or (rule
    /// `v12`) the control-only head's `unfit` release the lines guard withheld (`lines`, on its hold or
    /// on the `R0w` change it led to), `None` otherwise ([`RecentReading::release_withheld`]).
    pub release_withheld: Option<&'static str>,
}

/// Issue 524 unit 2a: [`Db::plan_org_name_refresh`]'s answer — what `refresh-org-names` WOULD do.
/// Read-only; the job stores it as the `org-name-refresh-plan` report.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct OrgNameRefreshPlan {
    /// [`HEAD_ELECTION_EPOCH`].
    pub epoch: &'static str,
    /// Rule `v10`: the planner's start (unix seconds), every election's [`HeadElectOrg::as_of`] — the
    /// instant [`HEAD_HOLD_CURRENT`]'s 3 years count back from. A wet run re-elects with THIS value
    /// (`OrgNameRefreshApplyArgs::as_of`), so the plan and the pre-pass agree; the app refuses a stored
    /// plan without it. `0` on a stopped plan.
    pub as_of: i64,
    /// `as_of` as `YYYY-MM-DD` (UTC); empty on a stopped plan.
    pub as_of_date: String,
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
    /// window, (rule `v8`) an R1 or R2 residue hold, or (rule `v10`) an R1 `head_current` hold — by why
    /// (`unfit`, `digits`), and the first
    /// [`ORG_NAME_REFRESH_RELEASED_LISTED`] of their ids by org id. A release that `serve` then held
    /// (no fit spelling, a bind guard) is no change and is not counted; its entry still reads
    /// `recent_released`.
    pub recent_released: BTreeMap<&'static str, u64>,
    pub recent_released_ids: Vec<i64>,
    /// Rule `v9`: the HOLDS (in the population) whose `digits` release v9 withheld — v8 would have
    /// released them, so the `recent`, `residue` or (rule `v10`) `head_current` hold stands — by why
    /// (`glue`, `junk_only`; rule `v12`: `lines`, a control-only head's `unfit` release the lines guard
    /// withheld — its `R0w` changes are changes, not counted here; see
    /// [`RecentReading::release_withheld`]), and the first [`ORG_NAME_REFRESH_RELEASED_LISTED`] of their
    /// ids by org id: the decision's accepted cost (plan 2526: 15 withheld, labelled 5 wrong, 2
    /// neutral, 8 right), listed so it can be read.
    pub release_withheld: BTreeMap<&'static str, u64>,
    pub release_withheld_ids: Vec<i64>,
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
    /// the smallest `((org_id XOR salt) × 2654435761) mod 2³²` — uniform over the class, not its oldest
    /// ids; since rule `v9` salted with the epoch ([`ORG_NAME_REFRESH_SAMPLE_SALT`]).
    pub samples: BTreeMap<String, Vec<HeadRefreshEntry>>,
    /// The canaries the walk passed, in walk order; a missing id yields no row.
    pub canaries: Vec<HeadRefreshEntry>,
    pub stopped: bool,
}

/// Issue 524 rule `v7`: the most released ids a plan lists — since the v8 review 500, so the dated
/// re-judge reads EVERY release (v8 adds the residue's releases to v7's window releases, plan 2523's
/// 10); past it the tally still counts them all.
pub const ORG_NAME_REFRESH_RELEASED_LISTED: usize = 500;

/// Issue 524 rule `v9`: the uniform samples' salt — the low 32 bits of FNV-1a 64 of
/// [`HEAD_ELECTION_EPOCH`] (`524u2-v12`: 2,780,007,269; v11's `524u2-v11` was 2,780,005,964, v10's
/// `524u2-v10` 2,780,006,399 and v9's `524u2-v9` 460,574,797). The
/// protocol's dated re-judge reads "fresh" samples, and an unsalted hash makes every plan's class
/// sample the same first ids: plan 2526's panel read the next 30 of each list ahead. Salted, each epoch draws anew, and a plan's samples stay
/// reproducible from its epoch.
pub const ORG_NAME_REFRESH_SAMPLE_SALT: u64 = epoch_salt(HEAD_ELECTION_EPOCH);

/// FNV-1a 64 of `epoch`, its low 32 bits.
const fn epoch_salt(epoch: &str) -> u64 {
    let bytes = epoch.as_bytes();
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut at = 0;
    while at < bytes.len() {
        hash ^= bytes[at] as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        at += 1;
    }
    hash & 0xFFFF_FFFF
}

/// The uniform sample's order: Knuth's multiplicative hash of the id XOR the epoch's salt (rule `v9`;
/// XOR keeps it a bijection of the id, so the sample stays uniform), mod 2³², then the id.
fn sample_key(org_id: i64) -> (u64, i64) {
    (((org_id as u64) ^ ORG_NAME_REFRESH_SAMPLE_SALT).wrapping_mul(2_654_435_761) & 0xFFFF_FFFF, org_id)
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

    /// Elect this org; `as_of` is the plan's (rule `v10`: [`HeadElectOrg::as_of`]).
    fn elect(
        &self,
        fns: &HeadElectFns,
        bind_defect: bool,
        dated: Option<&[DatedVote<'_>]>,
        as_of: i64,
    ) -> HeadElection {
        let org = HeadElectOrg {
            head: &self.head,
            identifier_kind: self.identifier_kind.as_deref(),
            identifier: self.identifier.as_deref(),
            bind_defect,
            dated,
            as_of,
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
        // Rule v10: the job's start is the plan's `as_of`, every election's (`head_current` counts back
        // from it), stored in the plan for the wet pre-pass. Rule v7: a date more than a day past it is
        // no date.
        let as_of = crate::now_unix();
        let horizon = as_of + 86_400;
        let scope: Option<Vec<i64>> = args.org_ids.map(|ids| {
            let mut ids = ids.to_vec();
            ids.sort_unstable();
            ids.dedup();
            ids
        });
        let mut plan = OrgNameRefreshPlan {
            epoch: HEAD_ELECTION_EPOCH,
            as_of,
            as_of_date: crate::rates::civil_date(as_of),
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
                let election = org.elect(&args.fns, false, None, as_of);
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
                    j.election = org.elect(&args.fns, true, None, as_of);
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
                    j.election = org.elect(&args.fns, j.bind_defect, Some(&dated), as_of);
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
                    // Rule v9: the holds a v8 `digits` release would have shipped.
                    if let (Some(why), HeadVerdict::Hold { .. }) = (read.release_withheld, verdict) {
                        *plan.release_withheld.entry(why).or_default() += 1;
                        if plan.release_withheld_ids.len() < ORG_NAME_REFRESH_RELEASED_LISTED {
                            plan.release_withheld_ids.push(org.id);
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
        rival_newest: recent.and_then(|r| r.rival_newest).map(crate::rates::civil_date),
        newest_is_head: recent.map(|r| r.newest_is_head),
        undated_mentions: recent.map(|r| r.undated),
        recent_released: recent.and_then(|r| r.released),
        release_withheld: recent.and_then(|r| r.release_withheld),
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
    /// `R0e` | `R0n` | `R1` | `R2` | (rule `v12`) `R0w`: the rule the plan elected the change by.
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
    /// Rule `v10`: the STORED plan's `as_of` ([`OrgNameRefreshPlan::as_of`]): the pre-pass re-elects
    /// with it, so a target whose head was printed after the plan reads `hold:head_current` and drifts,
    /// and the ledger's `evidence` names it.
    pub as_of: i64,
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
    /// Rule v9: the junk releases this run excludes (`exclude_released`). The caller already dropped
    /// the planned targets that shipped through one; the pre-pass also drifts a target whose
    /// re-election now ships only through one (its head named newer procedures since the plan and
    /// the exemption released it), so the filter holds at wet time, not only at plan time.
    pub exclude_released: &'a [String],
    /// Rule v11 (`exclude_stale_top`, the decision's pre-built fallback for a failed gate (a)): the
    /// caller already dropped the planned non-release R1 targets whose stored `top_newest` is missing
    /// or older than the plan's `as_of − HEAD_TOP_STALE_DAYS` ([`head_top_stale_date`]); the pre-pass
    /// also drifts a target whose re-election is a non-release R1 change on a stale top
    /// ([`head_top_stale`] of the re-read `top_newest`), so the filter holds at wet time too.
    pub exclude_stale_top: bool,
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
    /// sample order, the smallest `((org_id XOR salt) × 2654435761) mod 2³²`, rule `v9`'s
    /// [`ORG_NAME_REFRESH_SAMPLE_SALT`]), listed by org id. A run of that many renames or fewer lists
    /// every one.
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

/// Rule v9: the junk release (`released`, the election's own reading) a wet run's
/// `exclude_released` names, if any — the pre-pass's check that the filter still holds at wet time.
fn excluded_release(released: Option<&'static str>, exclude_released: &[String]) -> Option<&'static str> {
    released.filter(|why| exclude_released.iter().any(|x| x == why))
}

/// Rule v11: whether a wet run's `exclude_stale_top` drifts a re-elected change — an R1 change no junk
/// release shipped, whose reading's `top_newest` is stale against the plan's `as_of` (or unknown: no
/// reading, or no dated elected item, fails closed). The pre-pass's check that the filter still holds
/// at wet time.
fn excluded_stale_top(exclude_stale_top: bool, rule: HeadRule, recent: Option<&RecentReading>, as_of: i64) -> bool {
    exclude_stale_top
        && rule == HeadRule::R1
        && recent.is_none_or(|read| read.released.is_none())
        && head_top_stale(recent.and_then(|read| read.top_newest), as_of)
}

/// The pre-pass: each target's organization read and re-elected as the planner read it (the same
/// page SQL, mention read, voters and verdict probe), then compared with what the plan recorded.
/// `Err(read)` when `stop`, read between [`IN_CHUNK`]-target chunks, ended it after `read` targets.
async fn read_readiness(
    reader: &turso::Connection,
    fns: &HeadElectFns,
    targets: &[OrgNameRefreshTarget],
    as_of: i64,
    horizon: i64,
    exclude_released: &[String],
    exclude_stale_top: bool,
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
            let mut election = org.elect(fns, false, Some(&dated), as_of);
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
                    election = org.elect(fns, true, Some(&dated), as_of);
                }
            }
            // A target that now reads `hold:recent` (a newer head procedure since the plan),
            // `hold:residue` (rule v8: its newest item is now the head's), `hold:undated`, a serve
            // hold (rule v9: a pure test of the same head and spelling, so only a moved head or a new
            // display spelling makes one) or `hold:head_current` (rule v10: the head printed within 3
            // years of the plan's `as_of` — a print since the plan), (rule v11) `hold:top_stale` (a new
            // rival cluster, or the elected spelling's dates read differently) or a v11 serve or bind
            // hold is drifted like any other verdict that is not the plan's change. Rule v12: an `R0w`
            // target is ready when it re-elects `R0w` to the same collapsed head (its hold still
            // withheld `lines`); a guard that now passes, or a hold that no longer fires, drifts it.
            match &election.verdict {
                HeadVerdict::Change { rule, new } if rule.label() == target.rule && *new == target.new_name => {
                    match excluded_release(election.recent.as_ref().and_then(|r| r.released), exclude_released) {
                        None if excluded_stale_top(exclude_stale_top, *rule, election.recent.as_ref(), as_of) => {
                            out.push(Readiness::Drifted(format!(
                                "re-elected change:{} {:?}, but its elected spelling's newest dated item is {} — \
                                 older than {HEAD_TOP_STALE_DAYS} days before the plan's as_of, or undated — and \
                                 this run excludes a stale top (exclude_stale_top)",
                                target.rule,
                                target.new_name,
                                election
                                    .recent
                                    .and_then(|r| r.top_newest)
                                    .map_or_else(|| "undated".to_owned(), crate::rates::civil_date)
                            )))
                        }
                        None => out.push(Readiness::Ready(election)),
                        Some(why) => out.push(Readiness::Drifted(format!(
                            "re-elected change:{} {:?}, but it ships only through a `{why}` junk release, which \
                             this run excludes (exclude_released)",
                            target.rule, target.new_name
                        ))),
                    }
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

/// The ledger's `evidence`: the epoch, (rule `v10`) the plan's `as_of` (`YYYY-MM-DD`), the counts the
/// rename was elected by and (rule `v7`) the window's reading — k, h_W and the newest head and elected
/// items (`YYYY-MM-DD`, or `null`). Every value is a number, a date or a fixed ASCII label, so nothing
/// needs escaping.
fn refresh_evidence(rule: &str, election: &HeadElection, as_of: i64) -> String {
    let recent = election.recent.unwrap_or_default();
    let number = |n: Option<u64>| n.map_or_else(|| "null".to_owned(), |n| n.to_string());
    let date = |at: Option<i64>| at.map_or_else(|| "null".to_owned(), |at| format!("\"{}\"", crate::rates::civil_date(at)));
    let dated = election.recent.is_some();
    format!(
        "{{\"epoch\":\"{HEAD_ELECTION_EPOCH}\",\"as_of\":\"{}\",\"rule\":\"{rule}\",\"voting\":{},\"top\":{},\"head\":{},\"second\":{},\
         \"clusters\":{},\"head_procs\":{},\"head_in_window\":{},\"head_newest\":{},\"top_newest\":{}}}",
        crate::rates::civil_date(as_of),
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
    ///    now among them; since rule v9 one shipping only through a release `exclude_released` names,
    ///    and since rule v11 a non-release R1 on a stale top when `exclude_stale_top` is set). Drifted
    ///    plus gone above `max(2%, 5)` of the targets read aborts the run here, before any
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
        // Rule v7: the pre-pass dates the targets past its row checks as the planner dated them. Rule
        // v10: it re-elects them with the stored plan's `as_of`, so `head_current` reads as it did.
        let horizon = crate::now_unix() + 86_400;
        let as_of = args.as_of;
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
            read_readiness(
                &reader,
                &args.fns,
                &targets,
                as_of,
                horizon,
                args.exclude_released,
                args.exclude_stale_top,
                args.stop,
            )
            .await?
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
                            Value::Text(refresh_evidence(&target.rule, election, as_of)),
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
        DatedVote, HEAD_CURRENT_DAYS, HEAD_HOLD_CURRENT, HEAD_HOLD_NO_FIT, HEAD_HOLD_RECENT, HEAD_HOLD_RESIDUE, HEAD_HOLD_UNDATED,
        HeadElectFns, HeadElectOrg, HeadKeep, HeadRule, HeadVerdict, RecentReading, Script, dominant_script, elect_head,
        elect_head_counted, sample_key,
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

    /// Stand-in for `crosswalk::head_serve_hold` (rule v9): a new name led by `Organ of` the head is not
    /// led by is an organ for its entity.
    fn serve_hold(old: &str, new: &str, _r2: bool) -> Option<&'static str> {
        (new.starts_with("Organ of ") && !old.starts_with("Organ of ")).then_some(super::HEAD_HOLD_ORGAN)
    }

    /// Stand-in for `crosswalk::head_tail_family` (rule v12): the line's last word, letters only, a
    /// legal form.
    fn tail(line: &str) -> Option<&'static str> {
        let last: String = line.split_whitespace().last().unwrap_or("").chars().filter(|c| c.is_alphanumeric()).collect();
        match last.to_lowercase().as_str() {
            "gmbh" => Some("gmbh"),
            "ltd" | "limited" => Some("ltd"),
            "scpa" => Some("scpa"),
            _ => None,
        }
    }

    const FNS: HeadElectFns =
        HeadElectFns { vote_key: key, not_a_name: class, unfit, hold, r2_hold, twin_key: twin, serve_hold, family: tail };

    /// Rule v10: the plan's `as_of` the rule tests before v10 elect with — day 40,000 (2079), long after
    /// every dated test item (days under 20,000), so no head is current and their meaning is v9's. The
    /// v10 tests pass their own.
    const LATE_PLAN: i64 = 40_000 * 86_400;

    fn org(head: &str) -> HeadElectOrg<'_> {
        HeadElectOrg {
            head,
            identifier_kind: Some("national"),
            identifier: Some("GB1"),
            bind_defect: false,
            dated: None,
            as_of: LATE_PLAN,
        }
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
        // Rule v11: `Old Body` keys into no cluster (H = 0), so `Other Body` is a rival, and undated the
        // top fails closed as stale: R1 elects them and `top_stale` holds the change it would serve.
        assert_eq!(
            elect_head(&itself, &org("Old Body"), &raw),
            HeadVerdict::Hold { reason: super::HEAD_HOLD_TOP_STALE, rule: HeadRule::R1, new: Some("Acme GmbH".into()) },
            "and R1 elects them"
        );
        assert_eq!(elect_head(&itself[..1], &org("Old Body"), &raw), change("Acme GmbH"), "no rival: it changes");
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
        // Rule v7: H >= 2 reads the window, so the change needs dates (here only one elected item is
        // dated: no head item, so no window); without any it fails closed, held `undated`. Rule v11:
        // `Other` is a rival, so that one dated item is the day before the plan — a fresh top, which
        // `top_stale` never holds (undated, `Some(&[])`, it would: no `top_newest` fails closed).
        let fresh = [DatedVote { spelling: "Main", tender: 1, at: LATE_PLAN - DAY }];
        let dated_none = |head| HeadElectOrg { dated: Some(&fresh), ..org(head) };
        let at = [("Main", 11), ("Head", 2), ("Other", 7)];
        assert_eq!(elect_head(&at, &dated_none("Head"), &FNS), r1("Main"), "11/20 = 55%");
        let none: [DatedVote<'_>; 0] = [];
        assert_eq!(
            elect_head(&at, &HeadElectOrg { dated: Some(&none), ..org("Head") }, &FNS),
            HeadVerdict::Hold { reason: super::HEAD_HOLD_TOP_STALE, rule: HeadRule::R1, new: Some("Main".into()) },
            "undated top, a rival: rule v11 fails closed"
        );
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

    /// Rule v9 changed these values on purpose: the key is Knuth's hash of the id XOR the epoch's salt
    /// (FNV-1a 64 of the epoch, low 32 bits), so the dated re-judge's "fresh" samples are not the last
    /// plan's predictable next 30. Rule v12's epoch `524u2-v12` draws anew (salt 2,780,007,269); v11's
    /// `524u2-v11` salt was 2,780,005,964, its first three keys 3,424,993,341, 1,784,461,806 and
    /// 1,397,184,889; v10's `524u2-v10` was 2,780,006,399 (keys 1,744,442,526, 3,384,974,061 and
    /// 4,011,601,370); v9's `524u2-v9` was 460,574,797 (keys 1,624,961,164, 998,333,855 and
    /// 3,892,120,008). Unsalted (v2–v8) they read 2,654,435,761, 1,013,904,226 and 387,276,917.
    #[test]
    fn the_sample_key_is_knuths_hash_of_the_salted_id_mod_two_to_the_32() {
        assert_eq!(super::HEAD_ELECTION_EPOCH, "524u2-v12");
        assert_eq!(super::ORG_NAME_REFRESH_SAMPLE_SALT, 2_780_007_269);
        assert_eq!(super::epoch_salt("524u2-v12"), 2_780_007_269);
        assert_eq!(super::epoch_salt("524u2-v11"), 2_780_005_964, "v11's salt, as plans 2562 and 2566 drew it");
        assert_eq!(super::epoch_salt("524u2-v10"), 2_780_006_399, "v10's salt, as plan 2543 drew it");
        assert_eq!(super::epoch_salt("524u2-v9"), 460_574_797, "v9's salt, as plan 2527 drew it");
        assert_ne!(super::epoch_salt("524u2-v11"), super::ORG_NAME_REFRESH_SAMPLE_SALT, "each epoch its own draw");
        assert_eq!(sample_key(1), (411_149_348, 1));
        assert_eq!(sample_key(2), (4_079_489_335, 2));
        assert_eq!(sample_key(5), (2_678_308_192, 5));
        assert_eq!(sample_key(31_558_903), (514_407_922, 31_558_903));
        assert_eq!(sample_key(31_558_903).0, ((31_558_903u64 ^ 2_780_007_269) * 2_654_435_761) % (1 << 32));
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
        // The window does not hold it. Rule v11: the third cluster is a rival and the plan (LATE_PLAN)
        // comes long after the elected cluster's newest item, so `top_stale` holds what R1 elects.
        assert_eq!(
            e.verdict,
            HeadVerdict::Hold {
                reason: super::HEAD_HOLD_TOP_STALE,
                rule: HeadRule::R1,
                new: Some("Diputacion Provincial".into())
            }
        );
        assert_eq!(e.recent.unwrap().rival_newest, Some(104 * DAY), "the rival's newest item, a diagnostic");
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
        // Rule v9 moved this shape from `Head Body ul Lipowa 12` (v8's) to the elected name plus its
        // address: v9's junk-only test lets the elected name lose at most one head word, and `Head Body
        // ul Lipowa 12` → `Main Body` loses `head` AND `lipowa` — it is withheld now (asserted below).
        // The v8 point, a residue hold released for an address number on R2, is unchanged.
        let address = r2("Main Body ul Lipowa 12");
        assert_eq!(address.verdict, HeadVerdict::Change { rule: HeadRule::R2, new: "Main Body".into() });
        assert_eq!(address.recent.unwrap().released, Some("digits"));
        let other = r2("Head Body ul Lipowa 12");
        assert_eq!(other.verdict, residue(HeadRule::R2, "Main Body"));
        assert_eq!((other.recent.unwrap().released, other.recent.unwrap().release_withheld), (None, Some("junk_only")));
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

    // ---- Rule v9: the serve holds and the narrowed digits release.

    fn organ(rule: HeadRule, new: &str) -> HeadVerdict {
        HeadVerdict::Hold { reason: super::HEAD_HOLD_ORGAN, rule, new: Some(new.to_owned()) }
    }

    /// Rule v9: `serve_hold` is asked of R1 and R2 changes, with the display spelling, after the bind
    /// guards; never of R0 (a defective head always changes), and never the other way round (the
    /// head is the organ).
    #[test]
    fn the_serve_hold_is_asked_of_r1_and_r2_after_the_bind_guards_never_r0() {
        let r1 = [("Organ of Acme", 5), ("ORGAN OF ACME", 9), ("Acme", 1)];
        assert_eq!(elect_head(&r1, &org("Acme"), &FNS), organ(HeadRule::R1, "Organ of Acme"), "the display spelling");
        let r2 = [("Organ of Acme", 3), ("Nord Body", 2), ("Sued Body", 2), ("Ost Body", 2), ("West Body", 2), ("Acme", 1)];
        assert_eq!(elect_head(&r2, &org("Acme"), &FNS), organ(HeadRule::R2, "Organ of Acme"));
        let r0 = [("Organ of Acme", 3)];
        assert_eq!(
            elect_head(&r0, &org("s.o."), &FNS),
            HeadVerdict::Change { rule: HeadRule::R0n, new: "Organ of Acme".into() },
            "R0 is never asked"
        );
        assert_eq!(
            elect_head(&r0, &org(""), &FNS),
            HeadVerdict::Change { rule: HeadRule::R0e, new: "Organ of Acme".into() }
        );
        // The bind guard answers first.
        let plc = [("Organ of Bar Ltd", 5), ("Foo Holdings plc", 1)];
        assert_eq!(
            elect_head(&plc, &org("Foo Holdings plc"), &FNS),
            HeadVerdict::Hold { reason: "gb_family", rule: HeadRule::R1, new: Some("Organ of Bar Ltd".into()) }
        );
        let reverse = [("Acme", 5), ("Organ of Acme", 1)];
        assert_eq!(elect_head(&reverse, &org("Organ of Acme"), &FNS), r1_change("Acme"), "the organ head changes");
    }

    fn r1_change(new: &str) -> HeadVerdict {
        HeadVerdict::Change { rule: HeadRule::R1, new: new.into() }
    }

    /// Rule v9: `recent`, `residue` and `undated` keep precedence — the serve hold is asked only of a
    /// change that passes them — and a head the junk exemption releases still meets the serve holds.
    #[test]
    fn recent_residue_and_undated_decide_before_the_serve_hold() {
        let rename = [items("Organ of Acme", 1, 1, 8), items("Acme", 101, 100, 3)].concat();
        assert_eq!(dated(&rename, "Acme").verdict, recent(Some("Organ of Acme")));
        let newest = [items("Organ of Acme", 1, 1, 8), items("Acme", 101, 100, 1)].concat();
        assert_eq!(dated(&newest, "Acme").verdict, residue(HeadRule::R1, "Organ of Acme"));
        let undated = [("Organ of Acme", 8), ("Acme", 2)];
        assert_eq!(
            elect_head(&undated, &org("Acme"), &FNS),
            HeadVerdict::Hold { reason: HEAD_HOLD_UNDATED, rule: HeadRule::R1, new: None }
        );
        // Past both (a newer elected item), the serve hold answers, and the reading is kept.
        let mut forward = newest.clone();
        forward.push(DatedVote { spelling: "Organ of Acme", tender: 300, at: 300 * DAY });
        let e = dated(&forward, "Acme");
        assert_eq!(e.verdict, organ(HeadRule::R1, "Organ of Acme"));
        assert_eq!(e.recent.map(|r| (r.newest_is_head, r.released, r.release_withheld)), Some((false, None, None)));
        // Released from the window by its NIP (one lost word, `nip`), the change still meets the serve
        // hold: the release only lifts the date hold.
        let nip = [items("Organ of Acme Polska", 1, 1, 8), items("Acme Polska NIP 5250001009", 101, 100, 2)].concat();
        let e = dated(&nip, "Acme Polska NIP 5250001009");
        assert_eq!((e.recent.unwrap().fires, e.recent.unwrap().released), (true, Some("digits")));
        assert_eq!(e.verdict, organ(HeadRule::R1, "Organ of Acme Polska"));
    }

    /// Rule v9's glue: a number glued in the elected key to the head word beside it is the name's own,
    /// not an address — 4295185 `SMD 3` → `Smd3` (keyed `smd 3` / `smd3`), released by v8, now held
    /// under its v8 reason (here `residue`, and `recent` when the window fires) and read `glue`; a
    /// number glued to nothing the elected key holds still releases (14971333 `Infrastructura DF 5` →
    /// `Infrastructura S5`: `df5` is no elected token).
    #[test]
    fn a_number_glued_to_its_neighbour_in_the_elected_key_is_no_junk() {
        // k = 2 with one elected item in W (no window), the head the newest item: residue.
        let mut smd = items("Smd3", 1, 1, 10);
        smd.push(DatedVote { spelling: "SMD 3", tender: 50, at: 5 * DAY });
        smd.push(DatedVote { spelling: "SMD 3", tender: 60, at: 100 * DAY });
        let e = dated(&smd, "SMD 3");
        let read = e.recent.unwrap();
        assert_eq!((read.head_procs, read.fires, read.newest_is_head), (2, false, true));
        assert_eq!(e.verdict, residue(HeadRule::R1, "Smd3"));
        assert_eq!((read.released, read.release_withheld), (None, Some("glue")));
        // The window path withholds it alike.
        let window = [items("Smd3", 1, 1, 8), items("SMD 3", 101, 100, 3)].concat();
        let e = dated(&window, "SMD 3");
        assert_eq!(e.verdict, recent(Some("Smd3")));
        assert_eq!(e.recent.unwrap().release_withheld, Some("glue"));
        // Glued to the word after it too (`t + next`): `3 Dent` → `3dent`.
        let after = [items("Studio 3dent", 1, 1, 8), items("Studio 3 Dent", 101, 100, 3)].concat();
        let e = dated(&after, "Studio 3 Dent");
        assert_eq!((e.verdict, e.recent.unwrap().release_withheld), (recent(Some("Studio 3dent")), Some("glue")));
        // 14971333: released, as in v8.
        let df = [items("Infrastructura S5", 1, 1, 8), items("Infrastructura DF 5", 101, 100, 3)].concat();
        let e = dated(&df, "Infrastructura DF 5");
        assert_eq!((e.verdict, e.recent.unwrap().released), (r1("Infrastructura S5"), Some("digits")));
        assert_eq!(e.recent.unwrap().release_withheld, None);
    }

    /// Rule v9's junk-only: a digits release needs the elected name to be the head minus junk — at
    /// most one other head word of 3+ letters lost. 9814 (the activity phrase for the owner's
    /// head-and-address) and 1341's shape are withheld (`junk_only`); `SDIS 49` → `Service
    /// départemental …` (one lost word, the acronym) still releases.
    #[test]
    fn a_digits_release_needs_the_head_minus_junk() {
        let head = "Izrada sumskih sortimenata Dragan Sapina, Korenica, Zagrebacka 49/2";
        let mut rows = items("Izrada sumskih sortimenata", 1, 1, 13);
        rows.push(DatedVote { spelling: head, tender: 500, at: 100 * DAY });
        rows.push(DatedVote { spelling: head, tender: 500, at: 101 * DAY });
        let e = dated(&rows, head);
        let read = e.recent.unwrap();
        assert_eq!((e.head_votes, read.head_procs, read.fires, read.newest_is_head), (2, 1, false, true));
        assert_eq!(e.verdict, residue(HeadRule::R1, "Izrada sumskih sortimenata"));
        assert_eq!((read.released, read.release_withheld), (None, Some("junk_only")));
        let obrt = "Marcinkovic, Gradevinski Obrt, Jelka Marcinkovic Skugor, Sisak, Tina Ujevica 18";
        let rows = [items("Marcinkovic Gradevinski Obrt", 1, 1, 8), items(obrt, 101, 100, 3)].concat();
        let e = dated(&rows, obrt);
        assert_eq!((e.verdict, e.recent.unwrap().release_withheld), (recent(Some("Marcinkovic Gradevinski Obrt")), Some("junk_only")));
        let sdis = [items("Service Departemental Incendie Secours", 1, 1, 8), items("SDIS 49", 101, 100, 3)].concat();
        let e = dated(&sdis, "SDIS 49");
        assert_eq!(
            (e.verdict, e.recent.unwrap().released, e.recent.unwrap().release_withheld),
            (r1("Service Departemental Incendie Secours"), Some("digits"), None)
        );
        // `unfit` answers before either test: a control-character head with an address still releases.
        let control = [items("Acme Group", 1, 1, 8), items("Acme\u{8c}Services Dock Road 12", 101, 100, 3)].concat();
        assert_eq!(dated(&control, "Acme\u{8c}Services Dock Road 12").recent.unwrap().released, Some("unfit"));
    }

    // ---- Rule v10: the current-head hold.

    /// Elect `head` from `rows`, every mention dated, under a plan started at `as_of`.
    fn dated_at(rows: &[DatedVote<'_>], head: &str, as_of: i64) -> super::HeadElection {
        let org = HeadElectOrg { dated: Some(rows), as_of, ..org(head) };
        elect_head_counted(&votes_of(rows), &org, &FNS)
    }

    fn current(new: &str) -> HeadVerdict {
        HeadVerdict::Hold { reason: HEAD_HOLD_CURRENT, rule: HeadRule::R1, new: Some(new.to_owned()) }
    }

    /// The forward shape v10 is about: the head's 2 items (days 1 and 2) are older than every item of
    /// the cluster R1 elects (days 10–14), so neither the window nor the residue holdout fires, and R1
    /// changes the head — unless the head was printed within 3 years of the plan.
    fn forward(head: &'static str, elected: &'static str) -> Vec<DatedVote<'static>> {
        [items(head, 1, 1, 2), items(elected, 100, 10, 5)].concat()
    }

    /// Rule v10: an R1 change on a head printed within `HEAD_CURRENT_DAYS` (1,095) of the plan's `as_of`
    /// is HELD `head_current` with the spelling it would serve; the same organization planned more
    /// than 3 years after the head's newest item changes. The bound is inclusive: a head printed
    /// exactly `as_of − 1,095 days` holds, one second older changes.
    #[test]
    fn an_r1_change_on_a_head_printed_within_three_years_holds_head_current() {
        assert_eq!((HEAD_HOLD_CURRENT, HEAD_CURRENT_DAYS), ("head_current", 1_095));
        let rows = forward("Acme Works", "Acme Group");
        let head_newest = 2 * DAY;
        let e = dated_at(&rows, "Acme Works", head_newest + 100 * DAY);
        assert_eq!(e.verdict, current("Acme Group"));
        let read = e.recent.unwrap();
        assert_eq!((read.fires, read.newest_is_head, read.head_newest), (false, false, Some(head_newest)));
        assert_eq!((read.released, read.release_withheld), (None, None));
        assert_eq!(e.verdict.label(), "hold:head_current");
        // Planned more than 3 years after the head's newest item: the outdated name changes.
        assert_eq!(dated_at(&rows, "Acme Works", head_newest + 2_000 * DAY).verdict, r1("Acme Group"));
        // The boundary.
        let edge = head_newest + HEAD_CURRENT_DAYS * DAY;
        assert_eq!(dated_at(&rows, "Acme Works", edge).verdict, current("Acme Group"), "exactly 1,095 days: held");
        assert_eq!(dated_at(&rows, "Acme Works", edge + 1).verdict, r1("Acme Group"), "one second older: changes");
        // An H = 1 head (no window) is held alike.
        let stray = [items("Acme Works", 1, 1, 1), items("Acme Group", 100, 10, 5)].concat();
        assert_eq!(dated_at(&stray, "Acme Works", 20 * DAY).verdict, current("Acme Group"));
        assert_eq!(dated_at(&stray, "Acme Works", LATE_PLAN).verdict, r1("Acme Group"));
    }

    /// Rule v10 is asked after the serve holds and the bind guards: a change they hold keeps their
    /// reason on a current head, never `head_current`.
    #[test]
    fn a_serve_hold_or_a_bind_guard_on_a_current_head_keeps_its_reason() {
        let organ_rows = forward("Acme", "Organ of Acme");
        assert_eq!(dated_at(&organ_rows, "Acme", 20 * DAY).verdict, organ(HeadRule::R1, "Organ of Acme"));
        let plc = forward("Foo Holdings plc", "Bar Services Ltd");
        assert_eq!(
            dated_at(&plc, "Foo Holdings plc", 20 * DAY).verdict,
            HeadVerdict::Hold { reason: "gb_family", rule: HeadRule::R1, new: Some("Bar Services Ltd".into()) }
        );
        // And `recent` and `residue` decide before it, on heads as current as can be.
        let rename = [items("Old Name Agency", 1, 1, 8), items("New Name Agency", 101, 100, 3)].concat();
        assert_eq!(dated_at(&rename, "New Name Agency", 103 * DAY).verdict, recent(Some("Old Name Agency")));
        let newest = [items("Old Name Agency", 1, 1, 8), items("New Name Agency", 101, 100, 1)].concat();
        assert_eq!(dated_at(&newest, "New Name Agency", 101 * DAY).verdict, residue(HeadRule::R1, "Old Name Agency"));
    }

    /// Rule v10's junk exemption, as `recent` and `residue` have it: a current head unfit against
    /// itself (a control character) or carrying a whole number the elected key lacks (a NIP, one word
    /// lost) changes, read `released`; a number v9 withholds (`glue`) holds `head_current`, read
    /// `release_withheld`; a lower-case brand is no junk. A head the window already released ships
    /// (canary 3042's shape: its `(68)`).
    #[test]
    fn the_junk_exemption_releases_a_current_head() {
        let control = forward("Acme\u{8c}Services", "Acme Group");
        let e = dated_at(&control, "Acme\u{8c}Services", 20 * DAY);
        assert_eq!((e.verdict, e.recent.unwrap().released), (r1("Acme Group"), Some("unfit")));
        let nip = forward("Acme Polska NIP 5250001009", "Acme Polska");
        let e = dated_at(&nip, "Acme Polska NIP 5250001009", 20 * DAY);
        assert_eq!((e.verdict, e.recent.unwrap().released), (r1("Acme Polska"), Some("digits")));
        let glue = forward("Studio 3 Dent", "Studio 3dent");
        let e = dated_at(&glue, "Studio 3 Dent", 20 * DAY);
        assert_eq!(e.verdict, current("Studio 3dent"));
        assert_eq!((e.recent.unwrap().released, e.recent.unwrap().release_withheld), (None, Some("glue")));
        let brand = forward("acme services", "Acme Group");
        assert_eq!(dated_at(&brand, "acme services", 20 * DAY).verdict, current("Acme Group"));
        // 3042's shape: the window fires on the head's newest items and releases its number; v10 does
        // not take the release back.
        let colmar = [items("Hopitaux Civils de Colmar", 1, 1, 10), items("Hopitaux Civils de Colmar 68", 100, 100, 4)].concat();
        let e = dated_at(&colmar, "Hopitaux Civils de Colmar 68", 104 * DAY);
        assert_eq!(e.recent.map(|r| (r.fires, r.released)), Some((true, Some("digits"))));
        assert_eq!(e.verdict, r1("Hopitaux Civils de Colmar"));
    }

    /// Rule v10 holds R1 only: R0's defective head always changes, and R2 (excluded from wet runs by
    /// the v9 filter instead) keeps its own verdict on a current head.
    #[test]
    fn r0_and_r2_are_not_held_head_current() {
        let r0 = [items("Vergabekammer Nord", 1, 1, 4), items("S O", 101, 100, 2)].concat();
        let e = dated_at(&r0, "s.o.", 102 * DAY);
        assert_eq!(e.recent.and_then(|r| r.head_newest), Some(101 * DAY), "the head is current");
        assert_eq!(e.verdict, HeadVerdict::Change { rule: HeadRule::R0n, new: "Vergabekammer Nord".into() });
        let stray = [("Main Body", 3), ("A Body", 2), ("B Body", 2), ("C Body", 2), ("D Body", 2), ("Head Body", 1)];
        let rows = [items("Head Body", 1, 1, 1), items("Main Body", 100, 10, 3)].concat();
        let org_ = HeadElectOrg { dated: Some(&rows), as_of: 20 * DAY, ..org("Head Body") };
        let e = elect_head_counted(&stray, &org_, &FNS);
        assert_eq!(e.recent.and_then(|r| r.head_newest), Some(DAY), "the head is current");
        assert_eq!(e.verdict, HeadVerdict::Change { rule: HeadRule::R2, new: "Main Body".into() });
        // R2's own holds still answer: `residue` when the head names the newest item.
        let newest = [items("Main Body", 1, 1, 3), items("Head Body", 101, 100, 1)].concat();
        let org_ = HeadElectOrg { dated: Some(&newest), as_of: 101 * DAY, ..org("Head Body") };
        assert_eq!(elect_head(&stray, &org_, &FNS), residue(HeadRule::R2, "Main Body"));
    }

    /// Rule v10 reads only a dated head item: an undated R1 follows v7 whatever `as_of` says — H ≤ 1
    /// with no dates, an empty date list, or only the elected items dated changes; H ≥ 2 with no dates
    /// holds `undated` as before.
    #[test]
    fn an_undated_r1_follows_v7_under_v10() {
        let stray = [("Old Name Agency", 8), ("New Name Agency", 1)];
        let near = |dated| HeadElectOrg { dated, as_of: 9 * DAY, ..org("New Name Agency") };
        assert_eq!(elect_head(&stray, &near(None), &FNS), r1("Old Name Agency"));
        let none: [DatedVote<'_>; 0] = [];
        assert_eq!(elect_head(&stray, &near(Some(&none[..])), &FNS), r1("Old Name Agency"));
        let elected = items("Old Name Agency", 1, 1, 8);
        let e = elect_head_counted(&stray, &near(Some(&elected[..])), &FNS);
        assert_eq!((e.verdict, e.recent.unwrap().head_newest), (r1("Old Name Agency"), None));
        let pair = [("Old Name Agency", 8), ("New Name Agency", 2)];
        assert_eq!(
            elect_head(&pair, &near(None), &FNS),
            HeadVerdict::Hold { reason: HEAD_HOLD_UNDATED, rule: HeadRule::R1, new: None }
        );
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

    /// Rule v9's review: a wet run's `exclude_released` also holds at wet time — a target whose
    /// re-election ships only through an excluded junk release is not ready, while one that needs no
    /// release, or a release the run does not exclude, is.
    #[test]
    fn the_wet_filter_drifts_a_target_released_only_at_wet_time() {
        let digits = ["digits".to_owned()];
        assert_eq!(super::excluded_release(Some("digits"), &digits), Some("digits"));
        assert_eq!(super::excluded_release(Some("unfit"), &digits), None, "a release the run keeps");
        assert_eq!(super::excluded_release(None, &digits), None, "no release needed");
        assert_eq!(super::excluded_release(Some("digits"), &[]), None, "no filter");
    }

    // ---- Rule v11: the stale-top hold (`## Rule v11 — decided`, the protocol's unit pins).

    /// Plan 2543's `as_of` (2026-10-16 11:20:05 UTC): the judge aged every label against it.
    const PLAN_2543: i64 = 1_792_132_805;

    /// The unix instant of a `YYYY-MM-DD` day's start.
    fn on(date: &str) -> i64 {
        crate::rates::day_number(date).expect("a date") * DAY
    }

    /// One org's dated mentions: per `(spelling, n, newest)`, `n` items, one per tender, a day apart and
    /// ending on `newest`. Tenders never repeat across spellings.
    fn shape(clusters: &[(&'static str, i64, &str)]) -> Vec<DatedVote<'static>> {
        let mut rows = Vec::new();
        for (at, (spelling, n, newest)) in clusters.iter().enumerate() {
            let last = on(newest);
            for i in 0..*n {
                rows.push(DatedVote { spelling, tender: 1_000 * (at as i64 + 1) + i, at: last - i * DAY });
            }
        }
        rows
    }

    fn top_stale(new: &str) -> HeadVerdict {
        HeadVerdict::Hold { reason: super::HEAD_HOLD_TOP_STALE, rule: HeadRule::R1, new: Some(new.to_owned()) }
    }

    /// The protocol's `top_stale` pins on the labelled shapes (their V, T, H, clusters and the head's and
    /// the elected spelling's newest dates from plan 2543, aged from its `as_of`): job 2555's three
    /// wrongs and 16431242 (top age 423, 5 clusters) hold; 2857103 (2 clusters, top age 2,181) and
    /// 1800186 (a fresh top with a rival above H) change.
    #[test]
    fn top_stale_holds_the_labelled_stale_tops_with_a_rival_and_ships_the_rest() {
        assert_eq!((super::HEAD_HOLD_TOP_STALE, super::HEAD_TOP_STALE_DAYS), ("top_stale", 365));
        // 1556789: V 27, T 15, H 1, 4 clusters; head 2011-12-24, top 2020-03-11 (a revert spelling's rival).
        let rows = shape(&[
            ("Elected Name", 15, "2020-03-11"),
            ("Revert Name", 9, "2019-01-01"),
            ("Long Revert Name", 2, "2018-01-01"),
            ("Head Name", 1, "2011-12-24"),
        ]);
        let e = dated_at(&rows, "Head Name", PLAN_2543);
        assert_eq!((e.voting, e.top, e.head_votes, e.clusters), (27, 15, 1, 4));
        assert_eq!(e.verdict, top_stale("Elected Name"), "1556789");
        // 1276213: V 96, T 56, H 7, 3 clusters; head 2015-09-04, top 2023-11-20 (a superseding third name).
        let rows = shape(&[("Elected Name", 56, "2023-11-20"), ("Third Name", 33, "2026-09-01"), ("Head Name", 7, "2015-09-04")]);
        let e = dated_at(&rows, "Head Name", PLAN_2543);
        assert_eq!((e.voting, e.top, e.head_votes, e.clusters), (96, 56, 7, 3));
        assert_eq!(e.recent.map(|r| (r.fires, r.newest_is_head)), Some((false, false)), "past the window and the residue");
        assert_eq!(e.verdict, top_stale("Elected Name"), "1276213");
        // 1345526: V 143, T 128, H 1, 4 clusters; top 2025-01-29 (625 days, the youngest stale wrong).
        let rows = shape(&[
            ("Elected Name", 128, "2025-01-29"),
            ("Ville Name", 12, "2024-06-01"),
            ("Other Name", 2, "2023-01-01"),
            ("Head Name", 1, "2011-11-03"),
        ]);
        let e = dated_at(&rows, "Head Name", PLAN_2543);
        assert_eq!((e.top, e.head_votes, e.clusters), (128, 1, 4));
        assert_eq!(e.verdict, top_stale("Elected Name"), "1345526");
        // 16431242: V 16, T 10, H 1, 5 clusters; top 2025-08-19 (423 days) — a right fix it forgoes.
        let rows = shape(&[
            ("Elected Name", 10, "2025-08-19"),
            ("Second Name", 3, "2025-01-01"),
            ("Third Name", 1, "2024-01-01"),
            ("Fourth Name", 1, "2023-01-01"),
            ("Head Name", 1, "2020-12-24"),
        ]);
        let e = dated_at(&rows, "Head Name", PLAN_2543);
        assert_eq!((e.clusters, (PLAN_2543 - e.recent.unwrap().top_newest.unwrap()) / DAY), (5, 423));
        assert_eq!(e.verdict, top_stale("Elected Name"), "16431242");
        // 2857103: V 5, T 4, H 1, 2 clusters — no rival; top 2020-10-26 (2,181 days): it ships.
        let rows = shape(&[("Elected Name", 4, "2020-10-26"), ("Head Name", 1, "2012-11-03")]);
        let e = dated_at(&rows, "Head Name", PLAN_2543);
        assert_eq!((e.clusters, (PLAN_2543 - e.recent.unwrap().top_newest.unwrap()) / DAY), (2, 2_181));
        assert_eq!(e.verdict, r1("Elected Name"), "2857103");
        // 1800186: V 456, T 305, H 1, second 115, 10 clusters; a fresh top (2026-09-22): it ships.
        let mut clusters = vec![("Elected Name", 305, "2026-09-22"), ("Former Name", 115, "2025-12-01")];
        for name in ["Rival A", "Rival B", "Rival C", "Rival D", "Rival E", "Rival F", "Rival G"] {
            clusters.push((name, 5, "2024-01-01"));
        }
        clusters.push(("Head Name", 1, "2012-02-18"));
        let e = dated_at(&shape(&clusters), "Head Name", PLAN_2543);
        assert_eq!((e.voting, e.top, e.head_votes, e.second, e.clusters), (456, 305, 1, 115, 10));
        assert_eq!(e.verdict, r1("Elected Name"), "1800186");
    }

    /// `top_stale` counts a rival as `clusters > 1 + (H > 0)`: with H = 0 two clusters are a rival (the
    /// head keys into neither), with H ≥ 1 three are needed; and its age is strict — a top printed
    /// exactly `as_of − 365 days` is fresh, one second older is stale.
    #[test]
    fn top_stale_counts_a_rival_when_h_is_zero_and_reads_its_age_strictly() {
        let as_of = on("2026-10-16");
        let two = shape(&[("Elected Name", 5, "2020-01-01"), ("Other Name", 2, "2019-01-01")]);
        let e = dated_at(&two, "Absent Head", as_of);
        assert_eq!((e.head_votes, e.clusters), (0, 2));
        assert_eq!(e.verdict, top_stale("Elected Name"), "H = 0 and two clusters: a rival");
        let one = shape(&[("Elected Name", 5, "2020-01-01")]);
        assert_eq!(dated_at(&one, "Absent Head", as_of).verdict, r1("Elected Name"), "H = 0, one cluster: none");
        let pair = shape(&[("Elected Name", 5, "2020-01-01"), ("Head Name", 2, "2012-01-01")]);
        assert_eq!(dated_at(&pair, "Head Name", as_of).verdict, r1("Elected Name"), "H = 2 and two clusters: none");
        // The boundary, on the H = 0 rival shape.
        let edge = as_of - super::HEAD_TOP_STALE_DAYS * DAY;
        let at = |top: i64| {
            let rows = [
                DatedVote { spelling: "Elected Name", tender: 1, at: top },
                DatedVote { spelling: "Elected Name", tender: 2, at: top - DAY },
                DatedVote { spelling: "Elected Name", tender: 3, at: top - 2 * DAY },
                DatedVote { spelling: "Other Name", tender: 4, at: top - 3 * DAY },
            ];
            dated_at(&rows, "Absent Head", as_of).verdict
        };
        assert_eq!(at(edge), r1("Elected Name"), "exactly 365 days: fresh");
        assert_eq!(at(edge - 1), top_stale("Elected Name"), "one second older: stale");
        assert!(!super::head_top_stale(Some(edge), as_of) && super::head_top_stale(Some(edge - 1), as_of));
    }

    /// `top_stale` fails closed: no reading (no dates passed), or a reading with no dated elected item
    /// (`top_newest` `None`), is stale; the same shape with a fresh elected item changes.
    #[test]
    fn top_stale_fails_closed_on_a_missing_top_newest() {
        let as_of = on("2026-10-16");
        let votes = [("Elected Name", 5), ("Rival Name", 2), ("Head Name", 1)];
        // The head's item and the rival's dated, the elected cluster's not: the rival is the newest item.
        let partial = shape(&[("Rival Name", 2, "2026-01-01"), ("Head Name", 1, "2012-01-01")]);
        let e = elect_head_counted(&votes, &HeadElectOrg { dated: Some(&partial), as_of, ..org("Head Name") }, &FNS);
        let read = e.recent.unwrap();
        assert_eq!((read.top_newest, read.newest_is_head, read.released), (None, false, None));
        assert_eq!(e.verdict, top_stale("Elected Name"), "no dated elected item");
        let undated = HeadElectOrg { dated: None, as_of, ..org("Head Name") };
        assert_eq!(elect_head(&votes, &undated, &FNS), top_stale("Elected Name"), "no reading (H = 1: no undated hold)");
        assert!(super::head_top_stale(None, as_of));
        let mut fresh = partial.clone();
        fresh.push(DatedVote { spelling: "Elected Name", tender: 9_000, at: as_of - 10 * DAY });
        let e = elect_head_counted(&votes, &HeadElectOrg { dated: Some(&fresh), as_of, ..org("Head Name") }, &FNS);
        assert_eq!(e.verdict, r1("Elected Name"), "a fresh top ships");
    }

    /// A head the window, the residue holdout or `head_current` already RELEASED ships whatever its top
    /// (canary 3042: `(68)` released by the window, its top 2022-10-12 stale, 4 clusters); and
    /// `top_stale` never asks the junk exemption itself — the same digits head, released by nothing, is
    /// held, and no release is read.
    #[test]
    fn top_stale_exempts_a_released_head_and_mints_no_release() {
        let head = "Hopitaux Civils de Colmar 68";
        // 3042's shape: the head owns the newest four procedures (the window fires and releases `68`); a
        // rival votes; the plan is 1,900 days after the elected cluster's newest item.
        let window = [
            items("Hopitaux Civils de Colmar", 1, 1, 10),
            items("Groupe Hospitalier Colmar", 50, 20, 2),
            items(head, 100, 100, 4),
        ]
        .concat();
        let e = dated_at(&window, head, 2_000 * DAY);
        assert_eq!(e.recent.map(|r| (r.fires, r.released)), Some((true, Some("digits"))));
        assert!(super::head_top_stale(e.recent.unwrap().top_newest, 2_000 * DAY) && e.clusters == 3);
        assert_eq!(e.verdict, r1("Hopitaux Civils de Colmar"), "released by the window: it ships");
        // The head's procedures OLDER than the elected cluster's: no window, no residue. Within 3 years
        // of the plan, `head_current` asks the junk exemption, which releases `68`: it ships.
        let old = [
            items(head, 100, 1, 4),
            items("Groupe Hospitalier Colmar", 50, 20, 2),
            items("Hopitaux Civils de Colmar", 1, 50, 10),
        ]
        .concat();
        let e = dated_at(&old, head, 500 * DAY);
        assert_eq!(e.recent.map(|r| (r.fires, r.newest_is_head, r.released)), Some((false, false, Some("digits"))));
        assert_eq!(e.verdict, r1("Hopitaux Civils de Colmar"), "released by head_current: it ships");
        // The same organization planned 2,000 days on: no hold asks the exemption, so nothing releases
        // the head, and `top_stale` holds it without asking — no release is minted.
        let e = dated_at(&old, head, 2_000 * DAY);
        assert_eq!(e.recent.map(|r| (r.released, r.release_withheld)), Some((None, None)));
        assert_eq!(e.verdict, top_stale("Hopitaux Civils de Colmar"));
    }

    /// `top_stale` is R1's alone, asked last: R0 and R2 changes on a stale top with a rival ship; a serve
    /// hold, `head_current`, `recent` and `residue` keep their reasons on such a shape.
    #[test]
    fn top_stale_is_r1_only_and_asked_after_every_other_hold() {
        let as_of = 5_000 * DAY;
        // R2: a stray head under a 25% top, rivals galore, every item thousands of days old.
        let stray = [items("Main Body", 1, 10, 3), items("A Body", 10, 5, 2), items("B Body", 20, 5, 2), items("C Body", 30, 5, 2), items("D Body", 40, 5, 2), items("Head Body", 50, 1, 1)].concat();
        assert_eq!(dated_at(&stray, "Head Body", as_of).verdict, HeadVerdict::Change { rule: HeadRule::R2, new: "Main Body".into() });
        // R0: a placeholder head; the top and a rival stale.
        let r0 = [items("Vergabekammer Nord", 1, 10, 4), items("Other Body", 10, 5, 2)].concat();
        assert_eq!(dated_at(&r0, "s.o.", as_of).verdict, HeadVerdict::Change { rule: HeadRule::R0n, new: "Vergabekammer Nord".into() });
        // A serve hold first (the organ), on a stale top with a rival.
        let organ_rows = [items("Acme", 1, 1, 1), items("Organ of Acme", 10, 10, 5), items("Rival Body", 20, 5, 2)].concat();
        assert_eq!(dated_at(&organ_rows, "Acme", as_of).verdict, organ(HeadRule::R1, "Organ of Acme"));
        // `head_current` first: the head printed within 3 years, the top stale, a rival.
        let current_rows = [items("Acme Works", 1, 1, 1), items("Acme Group", 10, 10, 5), items("Rival Body", 20, 5, 2)].concat();
        let e = dated_at(&current_rows, "Acme Works", 500 * DAY);
        assert!(super::head_top_stale(e.recent.unwrap().top_newest, 500 * DAY));
        assert_eq!(e.verdict, current("Acme Group"));
        // `recent` and `residue` first.
        let rename = [items("Old Name Agency", 1, 1, 8), items("Rival Agency", 50, 5, 2), items("New Name Agency", 101, 100, 3)].concat();
        assert_eq!(dated_at(&rename, "New Name Agency", as_of).verdict, recent(Some("Old Name Agency")));
        let newest = [items("Old Name Agency", 1, 1, 8), items("Rival Agency", 50, 5, 2), items("New Name Agency", 101, 100, 1)].concat();
        assert_eq!(dated_at(&newest, "New Name Agency", as_of).verdict, residue(HeadRule::R1, "Old Name Agency"));
    }

    /// `rival_newest` is emitted — the newest item of a cluster neither the elected one nor the head's —
    /// and read by NO verdict: the same organization with its rival's items moved from oldest to newest
    /// elects the same, its reading differing in `rival_newest` alone.
    #[test]
    fn rival_newest_is_emitted_and_read_by_no_verdict() {
        let as_of = 1_200 * DAY;
        let old_rival = [items("Head Name", 1, 1, 1), items("Rival Name", 10, 5, 2), items("Elected Name", 20, 10, 5)].concat();
        let new_rival = [items("Head Name", 1, 1, 1), items("Rival Name", 10, 100, 2), items("Elected Name", 20, 10, 5)].concat();
        let (a, b) = (dated_at(&old_rival, "Head Name", as_of), dated_at(&new_rival, "Head Name", as_of));
        assert_eq!(a.verdict, top_stale("Elected Name"));
        assert_eq!(a.verdict, b.verdict, "the rival's dates move no verdict");
        let (ra, rb) = (a.recent.unwrap(), b.recent.unwrap());
        assert_eq!((ra.rival_newest, rb.rival_newest), (Some(6 * DAY), Some(101 * DAY)));
        assert_eq!(
            RecentReading { rival_newest: None, ..ra },
            RecentReading { rival_newest: None, ..rb },
            "and nothing else in the reading moves"
        );
        // A fresh top, the rival oldest or newest of all: a change either way, the same change.
        let fresh = |rival_day: i64| {
            let rows = [items("Head Name", 1, 1, 1), items("Rival Name", 10, rival_day, 2), items("Elected Name", 20, 1_490, 5)]
                .concat();
            let e = dated_at(&rows, "Head Name", 1_700 * DAY);
            (e.verdict, e.recent.unwrap().rival_newest)
        };
        assert_eq!(fresh(5), (r1("Elected Name"), Some(6 * DAY)));
        assert_eq!(fresh(1_600), (r1("Elected Name"), Some(1_601 * DAY)));
        // No rival: none read; the head's and the elected cluster's items never count.
        let pair = [items("Head Name", 1, 1, 1), items("Elected Name", 20, 10, 5)].concat();
        assert_eq!(dated_at(&pair, "Head Name", as_of).recent.unwrap().rival_newest, None);
    }

    /// The decision's no-move pin, on this module's side: v11 bumps the epoch and nothing about the vote
    /// key — the stand-in key `elect_head` runs with here keys exactly as before, and the clusters of
    /// every v11 shape are the vote key's (`crosswalk::head_vote_key` and `NAME_KEY_EPOCH` are pinned in
    /// ingest's own tests).
    #[test]
    fn the_v11_rule_moves_no_vote_key() {
        assert_eq!(key("Hopitaux Civils de Colmar 68"), "hopitaux civils de colmar 68");
        assert_eq!(key("The University of Sheffield"), "university of sheffield");
        assert_eq!(super::HEAD_SERVE_HOLDS.len(), 7);
        assert_eq!(&super::HEAD_SERVE_HOLDS[5..], &[super::HEAD_HOLD_DOUBLED, super::HEAD_HOLD_SEAT_DROPPED]);
        assert_eq!(super::HEAD_HOLD_PLACEHOLDER_ID, "placeholder_id");
    }

    /// The plan-time `exclude_stale_top` read: a stored day is its first instant (the cutoff's own day
    /// is stale — the filter fails closed), and a missing or malformed date is stale.
    #[test]
    fn a_stored_top_newest_reads_stale_from_its_day() {
        let as_of = on("2026-10-16") + 11 * 3_600; // 11:00 UTC, the cutoff 2025-10-16 11:00
        assert!(!super::head_top_stale_date(Some("2025-10-17"), as_of), "younger than a year");
        assert!(super::head_top_stale_date(Some("2025-10-16"), as_of), "the cutoff's own day");
        assert!(super::head_top_stale_date(Some("2025-10-15"), as_of));
        assert!(super::head_top_stale_date(None, as_of), "missing");
        assert!(super::head_top_stale_date(Some("2025-13-01"), as_of), "malformed");
        assert!(super::head_top_stale_date(Some("yesterday"), as_of), "malformed");
    }

    /// Rule v11's wet-time `exclude_stale_top` check: a non-release R1 change on a stale (or undated) top
    /// drifts when the filter is on; a released head, a fresh top, R2 or the filter off do not.
    #[test]
    fn the_stale_top_filter_drifts_a_stale_non_release_r1_at_wet_time() {
        let as_of = on("2026-10-16");
        let read = |top: Option<i64>, released: Option<&'static str>| RecentReading {
            top_newest: top,
            released,
            ..RecentReading::default()
        };
        let stale = read(Some(on("2024-01-01")), None);
        assert!(super::excluded_stale_top(true, HeadRule::R1, Some(&stale), as_of));
        assert!(!super::excluded_stale_top(false, HeadRule::R1, Some(&stale), as_of), "off");
        assert!(!super::excluded_stale_top(true, HeadRule::R2, Some(&stale), as_of), "R2");
        assert!(!super::excluded_stale_top(true, HeadRule::R1, Some(&read(Some(on("2024-01-01")), Some("digits"))), as_of), "a release");
        assert!(!super::excluded_stale_top(true, HeadRule::R1, Some(&read(Some(on("2026-10-01")), None)), as_of), "fresh");
        assert!(super::excluded_stale_top(true, HeadRule::R1, Some(&read(None, None)), as_of), "undated");
        assert!(super::excluded_stale_top(true, HeadRule::R1, None, as_of), "no reading");
    }

    // ---- Rule v12 (issue 552): the lines guard on a control-only head's release, and the wrap collapse.

    fn r0w(new: &str) -> HeadVerdict {
        HeadVerdict::Change { rule: HeadRule::R0w, new: new.to_owned() }
    }

    /// Point 1: a head is control-only when it reads `control` against itself, its collapsed form
    /// carries no control character or U+FFFD, and that form is fit (or lower-case and no non-name).
    #[test]
    fn a_control_only_heads_one_defect_is_control_whitespace() {
        use super::head_control_only;
        assert_eq!(head_control_only("Acme\nServices", &FNS).as_deref(), Some("Acme Services"));
        assert_eq!(head_control_only(" Acme\r\n\tServices \u{85}", &FNS).as_deref(), Some("Acme Services"));
        assert_eq!(head_control_only("acme\nservices", &FNS).as_deref(), Some("acme services"), "lower-case goes to the guard");
        assert_eq!(head_control_only("Acme\u{8c}Services", &FNS), None, "a C1 control is no whitespace");
        assert_eq!(head_control_only("Acme\n\u{fffd}Services", &FNS), None, "U+FFFD survives the collapse");
        assert_eq!(head_control_only("s.o.\n", &FNS), None, "a lower-case non-name once collapsed");
        assert_eq!(head_control_only("Acme Services", &FNS), None, "no control character");
        assert_eq!(super::HEAD_WITHHELD_LINES, "lines");
        assert_eq!(HeadRule::R0w.label(), "R0w");
    }

    /// Point 2: the lines guard — the elected key inside one line's key, or equal to the union of a
    /// contiguous run of whole lines. Lines split on the characters that are control AND whitespace,
    /// never on a no-break space.
    #[test]
    fn the_lines_guard_passes_one_line_or_a_run_of_whole_lines() {
        use super::head_lines_guard as guard;
        let head = "Landkreis Saalekreis\nDomplatz 9\n06217 Merseburg";
        assert!(guard(head, &key("Landkreis Saalekreis"), &FNS), "its first line");
        assert!(guard(head, &key("Saalekreis"), &FNS), "a subset of one line");
        assert!(guard(head, &key("Landkreis Saalekreis Domplatz 9"), &FNS), "lines 1-2 whole");
        assert!(guard(head, &key("Domplatz 9 06217 Merseburg"), &FNS), "lines 2-3 whole");
        assert!(!guard(head, &key("Landkreis Saalekreis Merseburg"), &FNS), "a splice of lines 1 and 3");
        assert!(!guard(head, &key("Landkreis Saalekreis Domplatz"), &FNS), "line 1 and part of line 2");
        assert!(!guard(head, &key("Kreisverwaltung Halle"), &FNS), "foreign");
        assert!(!guard(head, "", &FNS), "an empty elected key");
        for split in ["\t", "\r\n", "\u{b}", "\u{c}", "\u{85}"] {
            assert!(guard(&format!("Acme Group{split}High Street 5"), &key("Acme Group"), &FNS), "{split:?}");
        }
        assert_eq!(super::head_lines(" A\tB \r\n\r\nC\u{85}D\u{b}E\u{c}F "), vec!["A", "B", "C", "D", "E", "F"]);
        assert_eq!(super::head_lines("Acme Group\u{a0}High Street 5"), vec!["Acme Group\u{a0}High Street 5"], "a no-break space is no break");
        assert_eq!(super::head_lines("Acme\u{8c}Group"), vec!["Acme\u{8c}Group"], "a C1 control that is no whitespace");
    }

    /// Point 3: a break is a wrap when the line before it ends in no legal form and no `,` `;` `:`,
    /// and its last word ends in an unspaced hyphen, or is a connector with no `.`, or the next line
    /// starts lower-case.
    #[test]
    fn a_wrap_is_an_unspaced_hyphen_a_connector_or_a_lower_case_next_line() {
        use super::{head_line_wrap as wrap, head_wraps};
        assert!(wrap("Vergabekammer Rheinland-Pfalz", "bei dem Ministerium", &FNS), "lower-case next line");
        assert!(wrap("BPB - Büro für Planung und", "Bauüberwachung", &FNS), "a connector");
        assert!(wrap("Gesellschaft für", "Planung", &FNS) && wrap("Office FOR", "Planning", &FNS), "case-insensitive");
        assert!(wrap("Ingenieurbüro Planungs-", "Gesellschaft", &FNS), "an unspaced hyphen");
        assert!(wrap("Ingenieurbüro Planungs–", "Gesellschaft", &FNS), "an unspaced en dash");
        assert!(!wrap("Acme -", "Bau", &FNS), "a spaced dash");
        assert!(!wrap("Acme Bau u.", "Partner", &FNS), "a connector with a dot is an abbreviation");
        assert!(!wrap("Stadt Remagen", "Bürgermeister Björn Ingendahl", &FNS), "a person's line");
        assert!(!wrap("Acme Works GmbH", "zweigniederlassung", &FNS), "after a legal form");
        assert!(!wrap("X S.C.p.A.", "per il tramite della C.U.C.", &FNS), "after a multi-token legal form");
        for end in [",", ";", ":"] {
            assert!(!wrap(&format!("Acme und{end}"), "bau", &FNS), "{end}");
        }
        assert!(head_wraps("Szkoła Podstawowa nr 13\nz Oddziałami", &FNS));
        assert!(!head_wraps("Acme Group\n", &FNS), "no break");
        assert!(!head_wraps("BPB - Büro für Planung und\nBauüberwachung GmbH\nHohenzollernstr. 18", &FNS), "one break is no wrap");
    }

    /// The release paths: a control-only head the guard passes is released `unfit` (R1); one it stops
    /// whose breaks do not all wrap stays under its hold, read `release_withheld: lines`; one whose every
    /// break wraps changes to itself collapsed, R0w — from the window, the residue holdout and
    /// `head_current` alike, with `release_withheld: lines` read and no release counted.
    #[test]
    fn a_lines_withheld_head_that_wraps_changes_to_itself_collapsed() {
        // Released: the elected name is the head's first line.
        let first = forward("Acme Group\nHigh Street 5", "Acme Group");
        let e = dated_at(&first, "Acme Group\nHigh Street 5", 20 * DAY);
        assert_eq!((e.verdict, e.recent.unwrap().released), (r1("Acme Group"), Some("unfit")));
        // Withheld, no wrap (the first line ends in its legal form): head_current stands.
        let branch = forward("Acme Works GmbH\nBranch Office North", "Acme Works Office North");
        let e = dated_at(&branch, "Acme Works GmbH\nBranch Office North", 20 * DAY);
        assert_eq!(e.verdict, current("Acme Works Office North"));
        assert_eq!((e.recent.unwrap().released, e.recent.unwrap().release_withheld), (None, Some("lines")));
        // Withheld and every break a wrap: R0w, from each of the three holds.
        let (head, elected) = ("Vergabekammer Rheinland\nbei dem Ministerium Wirtschaft", "Vergabekammer Ministerium Wirtschaft");
        let collapsed = "Vergabekammer Rheinland bei dem Ministerium Wirtschaft";
        let e = dated_at(&forward(head, elected), head, 20 * DAY);
        assert_eq!(e.verdict, r0w(collapsed), "head_current's");
        assert_eq!((e.recent.unwrap().released, e.recent.unwrap().release_withheld), (None, Some("lines")));
        let window = [items(elected, 1, 1, 8), items(head, 101, 100, 3)].concat();
        let e = dated(&window, head);
        assert_eq!((e.recent.unwrap().fires, e.verdict.clone()), (true, r0w(collapsed)), "the window's");
        let stray = [items(elected, 1, 1, 5), items(head, 100, 100, 1)].concat();
        let e = dated(&stray, head);
        assert_eq!((e.recent.unwrap().newest_is_head, e.verdict.clone()), (true, r0w(collapsed)), "the residue holdout's");
        // A control-plus-lowercase head goes to the guard: it wraps, so R0w serves its own words.
        let lower = forward("acme services\nlimited liability", "Acme Group");
        assert_eq!(dated_at(&lower, "acme services\nlimited liability", 20 * DAY).verdict, r0w("acme services limited liability"));
        // A C1 control is no control-only head: released `unfit` as before v12.
        let c1 = forward("Acme\u{8c}Services", "Acme Group");
        assert_eq!(dated_at(&c1, "Acme\u{8c}Services", 20 * DAY).verdict, r1("Acme Group"));
        // R0 never asks the exemption, and an S1 head is kept: neither reaches R0w.
        let own = [items(head, 1, 1, 5), items(elected, 100, 10, 2)].concat();
        assert_eq!(dated_at(&own, head, 20 * DAY).verdict, HeadVerdict::Keep(HeadKeep::S1));
    }

    /// R0w asks the bind guards (a reason holds it, as for R0n, with the collapsed head) and nothing
    /// after: a rival on a stale top does not hold it `top_stale`.
    #[test]
    fn r0w_asks_the_bind_guards_and_no_later_hold() {
        let (head, elected) = ("Vergabekammer Rheinland\nbei dem Ministerium Wirtschaft", "Vergabekammer Ministerium Wirtschaft");
        let collapsed = "Vergabekammer Rheinland bei dem Ministerium Wirtschaft";
        let placeholder = HeadElectFns {
            hold: |_, _, _, identifier| (identifier == Some("KEINEANGABE")).then_some(super::HEAD_HOLD_PLACEHOLDER_ID),
            ..FNS
        };
        // The window's hold (asked before `serve`, so no bind guard has answered yet).
        let rows = [items(elected, 1, 1, 8), items(head, 101, 100, 3)].concat();
        let org = HeadElectOrg { dated: Some(&rows), identifier: Some("KEINEANGABE"), ..org(head) };
        let e = elect_head_counted(&votes_of(&rows), &org, &placeholder);
        assert_eq!(
            e.verdict,
            HeadVerdict::Hold { reason: super::HEAD_HOLD_PLACEHOLDER_ID, rule: HeadRule::R0w, new: Some(collapsed.into()) }
        );
        assert_eq!(e.recent.unwrap().release_withheld, Some("lines"));
        assert_eq!(dated(&rows, head).verdict, r0w(collapsed), "a real identifier: it changes");
        // A rival cluster and a top more than a year before as_of: R0w is decided at head_current, before
        // top_stale is reached.
        let rival = [items(head, 1, 1, 2), items(elected, 100, 10, 5), items("Landesamt Mainz", 200, 5, 1)].concat();
        let e = dated_at(&rival, head, 400 * DAY);
        assert!(e.clusters == 3 && super::head_top_stale(e.recent.unwrap().top_newest, 400 * DAY));
        assert_eq!(e.verdict, r0w(collapsed));
    }

    /// The durability test: each R0w-written head, re-elected on the same votes (the written head is
    /// its own cluster's spelling, the raw head's mentions still voting) with a fresh elected date, is
    /// a HOLD — the collapsed head is fit, so the junk exemption no longer releases it — never a change.
    #[test]
    fn an_r0w_written_head_is_held_on_the_same_votes() {
        let (head, elected) = ("Vergabekammer Rheinland\nbei dem Ministerium Wirtschaft", "Vergabekammer Ministerium Wirtschaft");
        let collapsed = "Vergabekammer Rheinland bei dem Ministerium Wirtschaft";
        assert_eq!(key(head), key(collapsed), "the written head keys into its own cluster");
        // head_current's shape, the elected spelling printed the day before the plan.
        let current_shape = [forward(head, elected), items(elected, 300, 19, 1)].concat();
        assert_eq!(dated_at(&current_shape, head, 20 * DAY).verdict, r0w(collapsed));
        let e = dated_at(&current_shape, collapsed, 20 * DAY);
        assert_eq!(e.verdict, current(elected));
        assert_eq!((e.recent.unwrap().released, e.recent.unwrap().release_withheld), (None, None));
        // The window's shape, the elected spelling printed after the head's newest item: it still fires.
        let window = [items(elected, 1, 1, 8), items(head, 101, 100, 3), items(elected, 300, 200, 1)].concat();
        assert_eq!(dated(&window, head).verdict, r0w(collapsed));
        assert_eq!(dated(&window, collapsed).verdict, recent(Some(elected)));
        // The residue holdout's shape (a fresh elected item would end the residue hold for the raw head
        // too: neither is then held, and the raw head changes R1 as before v12).
        let stray = [items(elected, 1, 1, 5), items(head, 100, 100, 1)].concat();
        assert_eq!(dated(&stray, head).verdict, r0w(collapsed));
        assert_eq!(dated(&stray, collapsed).verdict, residue(HeadRule::R1, elected));
        // The unit 3 review's shape: a bare number the elected key lacks (`13`, not glued, no 3+-letter
        // word lost). Collapsed, the head is fit, so the junk exemption would release it `digits` and the
        // next plan would change it R1 to the spelling the lines guard withheld: no R0w, the `lines`
        // hold (here `head_current`) stands on the raw head.
        let (head, elected) = ("Szkola Podstawowa nr 13\nw Tychach", "Szkola Podstawowa w Tychach");
        let collapsed = "Szkola Podstawowa nr 13 w Tychach";
        assert!(super::head_wraps(head, &FNS) && !super::head_lines_guard(head, &key(elected), &FNS));
        let e = dated_at(&forward(head, elected), head, 20 * DAY);
        assert_eq!(e.verdict, current(elected));
        assert_eq!((e.recent.unwrap().released, e.recent.unwrap().release_withheld), (None, Some("lines")));
        let e = dated_at(&forward(head, elected), collapsed, 20 * DAY);
        assert_eq!((e.verdict, e.recent.unwrap().released), (r1(elected), Some("digits")), "why: the collapsed head");
        // The same number in the elected key keeps it held: R0w (22661109's shape).
        let (head, elected) = ("Szkola Podstawowa nr 13\nw Tychach", "Szkola Podstawowa nr 13 Tychy");
        let e = dated_at(&forward(head, elected), head, 20 * DAY);
        assert_eq!(e.verdict, r0w(collapsed));
        assert_eq!(dated_at(&forward(head, elected), collapsed, 20 * DAY).verdict, current(elected));
    }
}
