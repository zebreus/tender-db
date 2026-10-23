//! Issue 530 unit 3, Build 1: rule 530-T's shared types, and the DRY `dissolve-guid-twins` planner.
//!
//! Rule 530-T (`530-triage/decision.md`, critic amendments A1–A12 adopted): within ONE notice copy, the
//! ORG sections whose gated BT-501 identifier is a `uuid_v4` national key are grouped by
//! `(register_jurisdiction(country), org_name_norm(name))`; a group holding two or more DISTINCT values
//! refuses every section's identifier whose value is not on the frozen, value-keyed `KEEP_530`, unless
//! the group's name is not safe as a fallback key, the group has no country, or (Build 1b, A7) the
//! group's roles by the notice copy's OWN OPT-300/OPT-301 references are role-incoherent
//! ([`guid_twin_role_mix`]; read through one fold, [`guid_twin_fold_roles`], and one injected
//! mapping, [`GuidTwinRoleRef`], by the gate and the planner alike). The rule is a pure
//! function of the notice (`ingest::project::guid_twin::guid_twin_decide`); store never depends on
//! ingest, so it is injected here as a [`GuidTwinRule`] fn pointer, the way `idgate::condemns` is
//! injected into `repair_placeholder_orgs_batch`. The planner and (in Build 2) the gate decide with
//! that one ruler.
//!
//! Build 1 writes NOTHING and changes no identity: the gate is not wired, and [`Db::plan_guid_twins`] is
//! read-only. It plans per MENTION (A2): a GUID row whose every mention the rule refuses is planned
//! whole; a row with a refused and a kept mention is partial, and only its refused mentions are planned
//! (rehomes). Each refused mention's target is the identifier-NULL row of its group key (A11), the row
//! the issue-234 resolver path binds (`ORDER BY id LIMIT 1`), never an identified row (300 §10); a key
//! with no such row is reported as a missing target. KEEP is computed in memory from the same walk
//! (Build 1): the stable rows, every row spanning two or more Tenders, and every merged v4 identifier.
//!
//! Build 1b also checks the rule against its own role read — the role tripwire and
//! `bars.role_ref_disagree` read the planner's `notice_ids` read of each group's sections, never the
//! roles the rule hands back — and moves R2's random draw to the fresh re-read's salt
//! ([`GUID_TWIN_SALT`], 530003), skipping the 60 rows R2 read at 530002 ([`GUID_TWIN_R2_READ`]).
//!
//! **Build 2.** The gate is wired (`ingest::project::guid_twin::guid_twin_gate`), and the planner reads
//! the COMPILED KEEP and names the gate reads ([`GuidTwinPlanArgs::keep`], `keep_version`,
//! `unsafe_names`; A3 iii): the walk still computes the KEEP candidates and reports them, and the KEEP
//! self-check is no longer a tautology — a (twin ∧ ≥ 2 Tenders) ∪ stable value missing from the
//! compiled file fails the plan and is listed as a KEEP-add commit (`keep_add_commit`). Two more
//! tripwires guard the dissolve: a whole row that is a merge survivor, or one a review table names. The
//! plan carries every planned mention uncapped for the wet run ([`GuidTwinPlan::wet_mentions`]), which
//! [`guid_twin_windows`] packs into windows BY TARGET KEY and [`Db::dissolve_guid_twin_window`] applies
//! one `BEGIN IMMEDIATE` each: a whole row through 440's id-set dissolve, a partial row by rehoming only
//! its refused mentions — both through `Db::dissolve_org_mentions` (440's per-org body, the target found
//! exactly as the issue-234 resolver arm finds or MINTS it), each moved mention logged in
//! `guid_twin_dissolve_log` (the restore manifest). Each window re-reads its rows' pre-images and the
//! review keep-set on the writer, and times its hold. [`Db::guid_twin_recurrence`] is A1's refused-value
//! recurrence arm, which also reads the REVERSE drift an add-only constant change leaves (mentions on
//! identifier-NULL rows the current rule binds); [`Db::restore_guid_twin_value`] moves one drifted value
//! onto the row a rebuild binds it to. A dry plan records the log's high-water mark, and reads a control
//! absent from its walk against the log, so a fresh plan after a wet window passes R1.
//!
//! **Review round 3.** Windows close on an estimated re-read budget too ([`GUID_TWIN_WET_WINDOW_REREAD`],
//! [`guid_twin_rebound_estimate`]: 542's re-derivation re-reads every mention of every re-bound row under
//! the lock, F1). A mention a reviewed rehoming verdict placed is neither planned (`verdict_kept`) nor
//! drift in either direction, and a window refuses one recorded after its re-plan (F3). "Spans >= 2
//! Tenders" is counted per VALUE over all of its GUID rows, in the planner and in A1 (F4). A1 admits a
//! target mention by the gate's own candidate test (F5) and reports the highest pre-gate notice above a
//! too-low boundary (F9).

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::time::Instant;

use serde::Serialize;
use turso::Value;

use crate::canonical::{
    IN_CHUNK, buyer_name_fold, org_name_norm, party_role_counts_sql, placeholders, register_jurisdiction, unix_year,
};
use crate::guid_census::{
    GUID_CENSUS_MAX_ID_SQL, GUID_CENSUS_ORGS_SQL, GUID_ROLE_BUYER, GUID_ROLE_CLASSES, GUID_ROLE_NONE,
    GUID_ROLE_REVIEW_INFO, GUID_ROLE_TENDERER, guid_census_bidders_sql, guid_census_winners_sql,
};
use crate::{Db, int, opt_int_of, opt_text_of, text};

// ---- Rule 530-T's shared types (the gate in ingest and this planner) ----

/// One ORG section of ONE notice copy as rule 530-T reads it: the published name, the mention's
/// country as the fold stores it (alpha-2), the raw BT-501 with its declared scheme, and the roles the
/// notice copy's OWN OPT-300/OPT-301 references give the section (Build 1b, A7). The gate builds these
/// from `mentions_and_dropped`'s mentions and `ingest::project::guid_twin::guid_twin_section_roles` over
/// the parsed copy, the planner from `organization_mentions` rows and the copy's stored `notice_ids`
/// references folded by [`guid_twin_fold_roles`] with the same mapping — never from
/// `tender_version_parties`.
#[derive(Debug, Clone, Copy)]
pub struct TwinView<'a> {
    pub section_id: &'a str,
    pub name: &'a str,
    pub country: Option<&'a str>,
    pub raw_identifier: Option<&'a str>,
    pub scheme: Option<&'a str>,
    /// The section's notice-reference roles, sorted and distinct (`Procedure-Buyer`, `Lot-ReviewOrg`,
    /// …); empty when the copy references it in no role.
    pub roles: &'a [String],
}

/// The mapping of one stored or parsed id value to the ORG role it references:
/// `(field_id, scheme, is_ref) -> role`. `ingest::project::guid_twin::guid_twin_role_ref` — the
/// projection's own `role_name` reading of OPT-300/OPT-301, DE-1.x aliases resolved — injected, as
/// [`GuidTwinRule`] is: store never depends on ingest.
pub type GuidTwinRoleRef = fn(&str, Option<&str>, bool) -> Option<String>;

/// One notice copy's roles by referenced section: `section_id -> roles` (sorted, distinct).
pub type GuidTwinSectionRoles = BTreeMap<String, Vec<String>>;

/// Rule 530-T's role read, in ONE place for the gate and the planner: each id value
/// `(field_id, scheme, target section, is_ref)` of one notice copy that `role_ref` maps to a role
/// gives the TARGET section that role. The gate feeds it the parsed copy's values, the planner the
/// copy's stored `notice_ids` rows; both pass the same `role_ref`, so both read one map.
pub fn guid_twin_fold_roles<'a>(
    refs: impl IntoIterator<Item = (&'a str, Option<&'a str>, &'a str, bool)>,
    role_ref: GuidTwinRoleRef,
) -> GuidTwinSectionRoles {
    let mut sets: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (field_id, scheme, target, is_ref) in refs {
        if let Some(role) = role_ref(field_id, scheme, is_ref) {
            sets.entry(target.to_owned()).or_default().insert(role);
        }
    }
    sets.into_iter().map(|(section, roles)| (section, roles.into_iter().collect())).collect()
}

/// A7's role scopes: an eForms role is `<scope>-<role>` (`Procedure-Buyer`, `Lot-ReviewOrg`,
/// `Part-ReviewOrg`); the guard reads the role AFTER its scope, so a `Lot-` and a `Part-` variant of
/// one role are alike.
pub const GUID_TWIN_ROLE_SCOPES: [&str; 3] = ["Procedure", "Lot", "Part"];
/// A7's buyer-side roles, after the scope: `Buyer`, `SProvider`, `AddInfo`, `TenderReceipt` — so
/// exactly Procedure-Buyer, Procedure-SProvider, Lot-/Part-AddInfo and Lot-/Part-TenderReceipt (and a
/// `Procedure-`/`Lot-`/`Part-` variant of any of them).
pub const GUID_TWIN_BUYER_SIDE_ROLES: [&str; 4] = ["Buyer", "SProvider", "AddInfo", "TenderReceipt"];
/// A7's review body, after the scope: `ReviewOrg` (Lot-ReviewOrg, Part-ReviewOrg).
pub const GUID_TWIN_REVIEW_ORG_ROLE: &str = "ReviewOrg";
/// A7's tenderer class among the notice-reference roles (OPT-300-Tenderer, OPT-301-Tenderer-MainCont,
/// OPT-301-Tenderer-SubCont): a winner is a tenderer the result names, so "not a winner" is any role
/// outside this class.
pub const GUID_TWIN_TENDERER_ROLES: [&str; 3] = ["Tenderer", "Tenderer-MainCont", "Tenderer-SubCont"];

/// A role after its [`GUID_TWIN_ROLE_SCOPES`] scope (`Part-ReviewOrg` → `ReviewOrg`); a role with no
/// such scope (`Tenderer-SubCont`, `LotResult-Paying`, `Contract-Signatory`) is read whole.
pub fn guid_twin_role_after_scope(role: &str) -> &str {
    match role.split_once('-') {
        Some((scope, rest)) if GUID_TWIN_ROLE_SCOPES.contains(&scope) => rest,
        _ => role,
    }
}

/// A7's role-incoherent twin, over a group's notice-reference roles (`530-triage/decision.md` A7,
/// decided as written by R2 F2): `Some("buyer_review_org")` when they hold [`GUID_TWIN_REVIEW_ORG_ROLE`]
/// and any [`GUID_TWIN_BUYER_SIDE_ROLES`] role; `Some("tenderer_mixed")` when they hold a
/// [`GUID_TWIN_TENDERER_ROLES`] role and any role outside that class; else `None`. Such a group binds
/// whole ([`TwinBind::RoleMix`]): the twin proves one name printed twice, not one body — a review body
/// or a bidder entered under the buyer's name would gain the buyer's row.
pub fn guid_twin_role_mix(roles: &[String]) -> Option<&'static str> {
    let after: Vec<&str> = roles.iter().map(|r| guid_twin_role_after_scope(r)).collect();
    if after.contains(&GUID_TWIN_REVIEW_ORG_ROLE) && after.iter().any(|r| GUID_TWIN_BUYER_SIDE_ROLES.contains(r)) {
        return Some("buyer_review_org");
    }
    let tenderer = |r: &String| GUID_TWIN_TENDERER_ROLES.contains(&r.as_str());
    if roles.iter().any(tenderer) && !roles.iter().all(tenderer) {
        return Some("tenderer_mixed");
    }
    None
}

/// `KEEP_530`: the v4 values that keep binding inside a twin group. VALUE-keyed (A4): matched on the
/// undashed upper-case value, the country ignored (a v4 collision across countries is negligible, and
/// ignoring the country can only spare more). Build 1 computes it in memory from the planner's walk;
/// Build 2 loads the committed `guid-keep-530.tsv`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuidKeep {
    values: HashSet<String>,
}

impl GuidKeep {
    /// The lookup key of a value as published or stored: its ASCII letters and digits, upper-cased
    /// (`037c1508-d945-…` and `037C1508D945…` are one key).
    pub fn key(value: &str) -> String {
        value.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_uppercase()).collect()
    }

    pub fn insert(&mut self, value: &str) -> bool {
        self.values.insert(Self::key(value))
    }

    pub fn contains(&self, value: &str) -> bool {
        self.values.contains(&Self::key(value))
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

impl<S: AsRef<str>> FromIterator<S> for GuidKeep {
    fn from_iter<I: IntoIterator<Item = S>>(iter: I) -> Self {
        let mut keep = GuidKeep::default();
        for value in iter {
            keep.insert(value.as_ref());
        }
        keep
    }
}

/// Why a twin group (two or more distinct v4 values under one name) binds whole instead of refusing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TwinBind {
    /// No country: the group key needs one, so the refusal never lands on a country-NULL row.
    NoCountry,
    /// Fewer than 2 content words once legal-form tokens are dropped (A8 i).
    ContentWords,
    /// `generic_body_name`: a kind of body, not a body (481's measured list).
    Generic,
    /// `GUID_TWIN_UNSAFE_NAMES`: the 530-only generics, polity umbrellas and person placeholders (A8 ii).
    UnsafeList,
    /// The run's `extra_unsafe_names` (an R2 finding, re-run without a deploy).
    Extra,
    /// A7 (Build 1b, R2 F2): the group's notice-reference roles are role-incoherent
    /// ([`guid_twin_role_mix`]): Lot-ReviewOrg beside a buyer-side role, or a tenderer beside a role
    /// outside the tenderer class.
    RoleMix,
    /// Every value of the group is on KEEP: nothing fresh to refuse (an SBH-shaped pair).
    Keep,
}

impl TwinBind {
    pub const ALL: [TwinBind; 7] = [
        TwinBind::NoCountry,
        TwinBind::ContentWords,
        TwinBind::Generic,
        TwinBind::UnsafeList,
        TwinBind::Extra,
        TwinBind::RoleMix,
        TwinBind::Keep,
    ];

    pub fn label(self) -> &'static str {
        match self {
            TwinBind::NoCountry => "no_country",
            TwinBind::ContentWords => "content_words",
            TwinBind::Generic => "generic",
            TwinBind::UnsafeList => "unsafe_list",
            TwinBind::Extra => "extra",
            TwinBind::RoleMix => "role_mix",
            TwinBind::Keep => "keep",
        }
    }
}

/// One twin group of one notice copy and what the rule did with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TwinGroup {
    /// The identifier's country (`register_jurisdiction` of the mention's), `None` without one.
    pub country: Option<String>,
    pub name_norm: String,
    /// `(section_id, normalised value)` of every candidate section of the group, in section order.
    pub members: Vec<(String, String)>,
    pub distinct_values: usize,
    /// `Some(reason)` when the whole group binds; `None` when its fresh values are refused.
    pub bind: Option<TwinBind>,
    /// The refused sections (empty when `bind` is set).
    pub refused: Vec<String>,
    /// The folded name has exactly 2 content words and the first is an office noun (`Amt`,
    /// `Vergabestelle`, `Referat`, …): R2 reads every such planned key.
    pub office_two_words: bool,
    /// The members' notice-reference roles, sorted and distinct ([`TwinView::roles`]): what A7's role
    /// guard read.
    pub roles: Vec<String>,
}

/// What rule 530-T decided for one notice copy.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TwinDecision {
    /// The sections whose identifier the rule refuses.
    pub refused: BTreeSet<String>,
    /// Every twin group (two or more distinct values), bound or refused, by key.
    pub groups: Vec<TwinGroup>,
    /// Sections whose gated identifier is a `uuid_v4` national key.
    pub candidates: usize,
    /// Candidates with an empty name: they join no group.
    pub unnamed: usize,
}

/// Rule 530-T, injected: `(sections of one notice copy, KEEP, extra unsafe names) -> decision`.
pub type GuidTwinRule = fn(&[TwinView<'_>], &GuidKeep, &[String]) -> TwinDecision;

// ---- The dry planner ----

/// Organization ids per window of the GUID-row walk (the census's).
pub const GUID_TWIN_WINDOW: i64 = 10_000;
/// GUID rows per batch; every IN chunk is at most [`IN_CHUNK`] keys.
pub const GUID_TWIN_BATCH: usize = IN_CHUNK;
/// The highest organization id census 2655 read: twin rows minted above it are counted apart from the
/// R1 bar, which the census sized.
pub const GUID_TWIN_CENSUS_MAX_ID: i64 = 31_970_152;
/// R1: planned rows, whole or partial (every twin partner on a GUID row, id at or under the census's
/// max), must not exceed census 2655's single-use pair twins: a partial row's refused copy makes it a
/// census twin as surely as a whole row's, so both count against the census's reach.
pub const GUID_TWIN_CENSUS_BAR: u64 = 11_258;
/// R1's fixed stable threshold (the decision's "Thresholds in one place": census `stable_mentions`'s
/// default): the stable tripwire reads it whatever the run's `stable_mentions` knob says, and a run at
/// any other knob cannot pass R1 (`GuidTwinBars::stable_mentions_is_decision`).
pub const GUID_TWIN_R1_STABLE_MENTIONS: u64 = 100;
/// The R2 salt (decision: fixed in the job before the dry run). Build 1b: 530003, the fresh re-read's
/// (decision A5, "on any W, the fresh re-read is 60 rows at salt 530003"; R2 F3). R2 read its random 60
/// at 530002 — those rows are [`GUID_TWIN_R2_READ`], skipped by the random draw.
pub const GUID_TWIN_SALT: u64 = 530_003;
/// The 60 GUID rows R2 read as its random sample at salt 530002 (2026-10-19, wf_5d7ad2e8-333:
/// `530-triage/r2/r2-result.json` `items.r2_random[*].key`), sorted. The fresh re-read at
/// [`GUID_TWIN_SALT`] excludes them (R2 F3, `rerun.r2.exclude`: "the 60 rows already read (take the
/// next in salted order)"): each stratum of the random draw skips them before taking its size, so the
/// next rows in salted order fill it. Only the random draw: the flagged-class picks ([`GUID_TWIN_READ_ALL`])
/// read targets and groups, and R2 re-reads any of those it has not read.
pub const GUID_TWIN_R2_READ: [i64; 60] = [
    22_760_026, 22_770_771, 22_776_193, 22_795_050, 22_795_894, 22_807_165, 22_808_070, 22_808_978, 22_810_161, 22_811_966,
    22_816_858, 22_822_634, 22_824_042, 22_830_288, 22_831_663, 22_832_955, 22_834_378, 22_835_704, 22_838_236, 22_839_698,
    22_840_798, 22_840_993, 22_842_358, 22_843_947, 22_845_186, 22_846_852, 22_851_601, 22_856_837, 22_858_429, 22_858_436,
    22_859_791, 22_860_164, 22_865_917, 22_868_220, 22_879_585, 22_886_247, 22_886_295, 22_891_825, 22_895_106, 22_897_538,
    22_898_487, 22_898_599, 22_898_749, 22_914_331, 22_914_565, 22_914_741, 22_933_591, 22_957_396, 22_959_819, 22_965_730,
    22_966_357, 22_974_691, 22_975_386, 22_987_736, 23_001_169, 23_011_460, 23_029_156, 23_053_387, 23_070_066, 23_761_350,
];
/// The newest mentions of a target read for its city set (A6).
pub const GUID_TWIN_TARGET_NEWEST: i64 = 50;
/// The city a mention publishes (A6): the BT-513 text of its ORG section, first ordinal.
pub const GUID_TWIN_CITY_FIELD: &str = "BT-513-Organization-Company";
/// The most entries any listing carries; its count is always whole.
pub const GUID_TWIN_LIST_CAP: usize = 2_000;
/// R2: a flagged class read whole up to this size; past it the top 30 plus 15 salted.
pub const GUID_TWIN_READ_ALL: usize = 45;
/// R2's random stratum (A5): buyer, review_or_info, other or tenderer.
pub const GUID_TWIN_R2_STRATA: [(&str, usize); 3] = [("buyer", 25), ("review_or_info", 25), ("other_or_tenderer", 10)];
/// The report's role-set breakdown names this many sets, the largest; the rest are summed.
pub const GUID_TWIN_ROLE_SETS_MAX: usize = 60;

/// The indexes every read seeks: the census's five, the 234 key probe, the two mention-keyed party
/// indexes and the merged-identifier holder index. Without one a chunk scans its table, so the planner
/// refuses before reading.
pub const GUID_TWIN_INDEXES: [&str; 9] = [
    "organization_mentions_org",
    "tender_versions_notice",
    "tender_version_parties_org_role",
    "tender_version_bid_parties_org",
    "tender_version_result_winners_org",
    "organizations_name_country",
    "tender_version_parties_mention_key",
    "tender_version_bid_parties_mention_key",
    "organization_merged_identifiers_org",
];

/// The GUID rows' mentions, `n` org ids at a time, through `organization_mentions_org`.
pub(crate) fn guid_twin_row_mentions_sql(n: usize) -> String {
    format!(
        "SELECT organization_id, notice_id FROM organization_mentions WHERE organization_id IN ({})",
        placeholders(n)
    )
}

/// Notices by primary key: the source and the publication instant.
pub(crate) fn guid_twin_notices_sql(n: usize) -> String {
    format!("SELECT id, source, published_at FROM notices WHERE id IN ({})", placeholders(n))
}

/// The Tender versions the notices cause, through `tender_versions_notice`.
pub(crate) fn guid_twin_versions_sql(n: usize) -> String {
    format!(
        "SELECT caused_by_notice_id, tender_id, seq FROM tender_versions WHERE caused_by_notice_id IN ({})",
        placeholders(n)
    )
}

/// Every ORG mention of `n` notices: the `organization_mentions` primary-key prefix on `notice_id`,
/// the ORG range of `section_id` (`NHS_BODY_PARTY_MENTIONS_SQL`'s idiom), so the rule sees what the
/// gate sees (A3 i).
pub(crate) fn guid_twin_notice_mentions_sql(n: usize) -> String {
    format!(
        "SELECT notice_id, section_id, organization_id, name, country, raw_identifier, scheme \
           FROM organization_mentions WHERE notice_id IN ({}) AND section_id >= 'ORG-' AND section_id < 'ORG.'",
        placeholders(n)
    )
}

/// Build 1b (A7): the ORG-role references of `n` notice copies — the `notice_ids` primary-key prefix on
/// `notice_id`. The id-refs (`is_ref = 1`), and every DE-1.x row whatever its flag: a DÖE copy parsed
/// before issue 98's inventory fix stores its role references `is_ref = 0`, and the projection marks
/// them references as it aliases them (`normalise_de1_parse`), so the gate reads them as roles and the
/// planner must too. The role mapping (`GuidTwinPlanArgs::role_ref`) drops every row that names no
/// role. The OR is no `is_ref = 1` predicate alone, so `notice_ids_target` (partial on `is_ref = 1`)
/// cannot serve it; the primary key does (pinned by `the_twin_planner_reads_by_index_at_the_chunk_width`).
pub(crate) fn guid_twin_role_refs_sql(n: usize) -> String {
    format!(
        "SELECT notice_id, field_id, scheme, value, is_ref FROM notice_ids WHERE notice_id IN ({}) \
           AND (is_ref = 1 OR (field_id >= 'DE1-' AND field_id < 'DE1.'))",
        placeholders(n)
    )
}

/// Build 2 (A1): the newest notice id, O(1) by the rowid walk backwards.
pub(crate) const GUID_TWIN_NOTICE_MAX_ID_SQL: &str = "SELECT id FROM notices ORDER BY id DESC LIMIT 1";

/// The highest rowid of `organization_merged_identifiers`: O(1) because turso serves `ORDER BY rowid
/// DESC` by walking the table b-tree backwards (it plans an explicit `rowid` as it plans an INTEGER
/// PRIMARY KEY alias), so no sorter reads the table before the LIMIT.
pub(crate) const GUID_TWIN_MERGED_MAX_SQL: &str =
    "SELECT rowid FROM organization_merged_identifiers ORDER BY rowid DESC LIMIT 1";

/// One rowid window of the merged identifiers, the v4 lengths a filter on it (A3 ii).
pub(crate) const GUID_TWIN_MERGED_SQL: &str = "SELECT identifier, country, org_id FROM organization_merged_identifiers \
     WHERE rowid > ? AND rowid <= ? AND length(identifier) IN (32, 36)";

/// Bound rows that are not GUID rows, by primary key.
pub(crate) fn guid_twin_orgs_sql(n: usize) -> String {
    format!("SELECT id, identifier FROM organizations WHERE id IN ({})", placeholders(n))
}

/// Which of `n` orgs hold a merged identifier (a merge survivor), through
/// `organization_merged_identifiers_org`.
pub(crate) fn guid_twin_survivors_sql(n: usize) -> String {
    format!("SELECT DISTINCT org_id FROM organization_merged_identifiers WHERE org_id IN ({})", placeholders(n))
}

/// The identifier-NULL rows of `n` names, through `organizations_name_country` (the 234 probe's
/// index); the country is matched in Rust and the lowest id wins, as the resolver's `LIMIT 1` does.
pub(crate) fn guid_twin_targets_sql(n: usize) -> String {
    format!(
        "SELECT id, name_norm, country, name FROM organizations WHERE name_norm IN ({}) AND identifier IS NULL",
        placeholders(n)
    )
}

/// A target's mention count: its range of the covering `organization_mentions_org`.
pub(crate) const GUID_TWIN_TARGET_COUNT_SQL: &str = "SELECT COUNT(*) FROM organization_mentions WHERE organization_id = ?";

/// A target's newest mentions: the reverse walk of `organization_mentions_org` (513's idiom), so no
/// sorter reads every mention of a large target before the LIMIT.
pub(crate) const GUID_TWIN_TARGET_NEWEST_SQL: &str =
    "SELECT notice_id, section_id FROM organization_mentions WHERE organization_id = ? ORDER BY rowid DESC LIMIT ?";

/// The BT-513 city of every ORG section of `n` notices: the `notice_texts` primary-key prefix.
pub(crate) fn guid_twin_cities_sql(n: usize) -> String {
    format!(
        "SELECT notice_id, section_id, ordinal, value FROM notice_texts WHERE notice_id IN ({}) \
           AND section_id >= 'ORG-' AND section_id < 'ORG.' AND field_id = ?",
        placeholders(n)
    )
}

/// The party roles of `n` notices' mentions, through `tender_version_parties_mention_key`.
pub(crate) fn guid_twin_parties_sql(n: usize) -> String {
    format!(
        "SELECT mention_notice_id, mention_section_id, role FROM tender_version_parties \
          WHERE mention_notice_id IN ({})",
        placeholders(n)
    )
}

/// The bid-party roles of `n` notices' mentions, through `tender_version_bid_parties_mention_key`.
pub(crate) fn guid_twin_bid_parties_sql(n: usize) -> String {
    format!(
        "SELECT mention_notice_id, mention_section_id, role FROM tender_version_bid_parties \
          WHERE mention_notice_id IN ({})",
        placeholders(n)
    )
}

/// The lot-result winners of `n` Tenders, the table's primary-key prefix.
pub(crate) fn guid_twin_result_winners_sql(n: usize) -> String {
    format!(
        "SELECT tender_id, seq, organization_id FROM tender_version_result_winners WHERE tender_id IN ({})",
        placeholders(n)
    )
}

/// [`Db::plan_guid_twins`]'s inputs.
pub struct GuidTwinPlanArgs<'a> {
    /// The class: `ingest::idgate::uuid_v4`.
    pub is_guid: fn(&str) -> bool,
    /// Rule 530-T: `ingest::project::guid_twin::guid_twin_decide`.
    pub rule: GuidTwinRule,
    /// Build 2 (A3 iii): the KEEP the rule reads — `Some(ingest::project::guid_twin::keep_530())`, the
    /// compiled file the gate reads, in every job. `None` is Build 1's in-memory KEEP (the walk's
    /// candidates; the planner's structural tests), whose self-check is a tautology.
    pub keep: Option<&'a GuidKeep>,
    /// The KEEP's version as reported (`GUID_KEEP_530_VERSION`; `walk` for `None`).
    pub keep_version: &'a str,
    /// The compiled unsafe-name list the rule reads (`GUID_TWIN_UNSAFE_NAMES`), reported so a wet run
    /// can refuse a plan read under another list.
    pub unsafe_names: &'a [&'a str],
    /// Every org id a review or verdict table names (`Db::org_ids_under_review`): a whole planned row
    /// among them trips `tripwire_under_review` (its delete would leave a verdict without a referent).
    /// `None`: not checked (reported as such).
    pub under_review: Option<&'a HashSet<i64>>,
    /// Review round 3 (F3): every `(notice_id, section_id)` a reviewed rehoming verdict names
    /// (`org_mention_rehoming`, [`Db::guid_twin_rehomed_keys`]): a person put that mention where it sits,
    /// so a refused one is neither planned nor counted toward its row being whole or partial (it trips no
    /// tripwire); it is reported as `verdict_kept`. `None`: not read (the planner's structural tests).
    pub rehomed: Option<&'a HashSet<(i64, String)>>,
    /// A7's notice-reference role mapping: `ingest::project::guid_twin::guid_twin_role_ref`, the one
    /// the gate's `guid_twin_section_roles` reads the parsed copy with.
    pub role_ref: GuidTwinRoleRef,
    /// A party role's class, one of `GUID_ROLE_CLASSES` (the census's `guid_role_class`).
    pub role_class: fn(&str) -> &'static str,
    /// GUID rows stable whatever their mentions (254, 255, 8378).
    pub stable_ids: &'a [i64],
    /// A GUID row with at least this many mentions is stable too.
    pub stable_mentions: u64,
    /// Names the run also treats as unsafe (R2 findings, folded by the rule).
    pub extra_unsafe_names: &'a [String],
    /// The listing per sample list.
    pub samples: usize,
    /// Organization ids per window: [`GUID_TWIN_WINDOW`] in the job.
    pub window: i64,
    /// GUID rows per batch, clamped to 1..=[`GUID_TWIN_BATCH`].
    pub batch: usize,
    /// R1's not-planned controls.
    pub not_planned: &'a [i64],
    /// R1's planned controls with the target each must get.
    pub planned: &'a [(i64, i64)],
    /// Reported expectations that are not bars: `(org, Some(target))`, or `None` for "planned, any target".
    pub expected: &'a [(i64, Option<i64>)],
    /// `(country, org_name_norm)` names whose GUID rows are tallied planned / kept (Göttingen).
    pub exhibits: &'a [(&'a str, &'a str)],
    pub stop: &'a (dyn Fn() -> bool + Sync),
    pub progress: &'a (dyn Fn(u64, &str) + Sync),
}

/// Rows and mentions of one cell.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct GuidTwinCount {
    pub rows: u64,
    pub mentions: u64,
}

/// One guard reason's exclusions.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct GuidTwinGuardCount {
    /// Twin groups that bound whole for this reason.
    pub groups: u64,
    /// Their candidate sections (mentions).
    pub mentions: u64,
}

/// One KEEP candidate (the decision's columns): Build 2 commits this list as `guid-keep-530.tsv`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GuidKeepCandidate {
    pub country: Option<String>,
    /// The undashed upper-case value.
    pub value: String,
    /// The GUID row at freeze, or the merged identifier's live holder.
    pub org_id: i64,
    /// `None` for a merged value with no GUID row of its own.
    pub mentions: Option<u64>,
    pub tenders: Option<u64>,
    /// `stable`, `multi_tender`, `merged`, joined by `+` when several hold.
    pub reason: String,
}

/// One planned or controlled GUID row.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct GuidTwinRowSample {
    pub org_id: i64,
    pub identifier: String,
    pub name: String,
    pub country: Option<String>,
    pub role: &'static str,
    pub mentions: u64,
    /// Its ORG mentions the rule refuses.
    pub refused: u64,
    pub tenders: u64,
    /// The first 3 distinct notice ids of its mentions.
    pub notice_ids: Vec<i64>,
    /// `whole`, `partial` or `no`.
    pub planned: &'static str,
    /// The targets of its refused mentions (`None`: a missing target).
    pub targets: Vec<Option<i64>>,
    pub first_year: Option<i64>,
    pub in_keep: bool,
    pub stable: bool,
    /// The census-style twin before and after the plan (A9).
    pub twin: bool,
    pub twin_after: bool,
    /// The cities its refused mentions publish (folded).
    pub cities: Vec<String>,
}

/// One control and whether it met its expectation.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct GuidTwinControl {
    pub org_id: i64,
    /// `not_planned`, `planned` (a bar) or `expected` (reported only).
    pub expect: &'static str,
    pub expected_target: Option<i64>,
    /// Found as a GUID row by the walk.
    pub found: bool,
    pub row: Option<GuidTwinRowSample>,
    /// Absent from the walk but logged by an earlier wet run (`guid_twin_dissolve_log`): for a planned
    /// or expected control, met when every logged row of it is a whole dissolve onto its expected target
    /// (a fresh dry plan after a wet window passes R1); for a not-planned control, a miss (a wet run
    /// dissolved what must stay).
    pub dissolved_by_wet: bool,
    pub ok: bool,
}

/// One target key (an identifier-NULL row the refused mentions would join, or a missing one they would
/// mint).
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct GuidTwinTargetSample {
    pub target_id: Option<i64>,
    /// The key's stable id for R2's salted order and tie-breaks: the target id, or for a missing key
    /// the smallest org id among its planned rows (unique: a GUID row is never an identifier-NULL row).
    pub sample_id: i64,
    /// The target row's name; for a missing key, the name a planned mention publishes.
    pub name: String,
    pub name_norm: String,
    pub country: Option<String>,
    /// Its mention rows today (0 for a missing target).
    pub mentions: u64,
    /// 3 distinct notice ids of its newest mentions.
    pub notice_ids: Vec<i64>,
    /// The folded cities over its newest [`GUID_TWIN_TARGET_NEWEST`] mentions, the first 10.
    pub cities: Vec<String>,
    pub city_count: usize,
    pub planned_mentions: u64,
    pub planned_rows: u64,
    /// Planned mentions whose city is absent from its non-empty city set (A6 flag a).
    pub city_absent: u64,
    /// Its existing mentions show 2 or more cities (A6 flag b).
    pub multi_city: bool,
    /// The folded cities its planned mentions publish, the first 10, and how many there are.
    pub planned_cities: Vec<String>,
    pub planned_city_count: usize,
    /// Its planned mentions alone show 2 or more cities (flag c, an A6 extension: it sees a missing key
    /// whose minted row would take several towns' mentions, and a target whose own mentions carry no
    /// city, which flags a and b cannot).
    pub multi_city_planned: bool,
    pub office_two_words: bool,
}

/// One twin group, as listed (guard exclusions, role-incoherent groups).
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct GuidTwinGroupSample {
    pub notice_id: i64,
    pub name_norm: String,
    pub country: Option<String>,
    pub sections: Vec<String>,
    pub org_ids: Vec<i64>,
    pub values: Vec<String>,
    pub refused: Vec<String>,
    /// `bind` label, or `refused`.
    pub outcome: &'static str,
    /// The sections' roles (party roles; `bid-tenderer`, `bid-subcontractor`; `result-winner`).
    pub roles: Vec<String>,
    /// The members' roles by the notice copy's own OPT-300/OPT-301 references, as the PLANNER read them
    /// from `notice_ids` (Build 1b): the union over the group's sections of the copy's section → roles
    /// map — what A7's role guard must have read, taken independently of the roles the rule returned.
    pub ref_roles: Vec<String>,
    /// The roles the rule returned for the group ([`TwinGroup::roles`]), filled only where they differ
    /// from `ref_roles` (`bars.role_ref_disagree_samples`): a rule that mis-keyed them. Empty (and not
    /// serialized) elsewhere — and for a rule that returned NONE, which only a disagreement sample shows.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rule_roles: Vec<String>,
}

/// One planned mention (A2: the plan's unit is the `(notice_id, section_id)` mention).
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct GuidTwinMentionKey {
    pub notice_id: i64,
    pub section_id: String,
    /// The GUID row it is bound to today.
    pub org_id: i64,
    /// The identifier-NULL row it would join; `None` mints.
    pub target: Option<i64>,
    /// Its row is planned whole (every mention refused); else the mention is a rehome.
    pub whole: bool,
}

/// A refused mention bound to a row that is not a GUID row (A3 i), by bound kind. Read in candidate
/// notices only (a notice with a mention on a walked GUID row), so every count is a LOWER BOUND on what
/// the gate refuses off GUID rows: a twin notice whose every v4 value already sits off GUID rows is not
/// read.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct GuidTwinOffGuid {
    /// `identifier_null`, `merge_survivor`, `other_identified`, `guid_above_max_id` (a v4 row the walk
    /// did not reach: minted above its max id) or `absent`.
    pub by_kind: BTreeMap<&'static str, u64>,
    /// Of them, mentions already on their target.
    pub already_on_target: u64,
    pub samples: Vec<GuidTwinOffGuidSample>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct GuidTwinOffGuidSample {
    pub notice_id: i64,
    pub section_id: String,
    pub org_id: i64,
    pub kind: &'static str,
    pub value: String,
    pub target: Option<i64>,
}

/// The TED/DÖE copies that disagree: rows with a refused and a kept mention (A2's partial rows).
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct GuidTwinPartial {
    pub rows: u64,
    pub refused_mentions: u64,
    pub kept_mentions: u64,
    /// `refused <sources> / kept <sources>` of each partial row's notices.
    pub by_sources: BTreeMap<String, u64>,
    pub samples: Vec<GuidTwinRowSample>,
}

/// The city screen (A6).
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct GuidTwinCityScreen {
    pub planned_mentions_with_city: u64,
    pub planned_mentions_without_city: u64,
    /// Targets whose newest mentions publish at least one city.
    pub targets_with_cities: u64,
    /// (a) planned mentions whose city is absent from a non-empty target city set, and their rows and
    /// targets.
    pub city_absent_mentions: u64,
    pub city_absent_rows: u64,
    pub city_absent_targets: u64,
    /// (b) targets whose existing mentions show 2 or more cities.
    pub multi_city_targets: u64,
    /// (c, an A6 extension) target keys, found or missing, whose planned mentions alone show 2 or more
    /// cities, and their planned mentions and (distinct) planned rows.
    pub planned_multi_city_keys: u64,
    pub planned_multi_city_mentions: u64,
    pub planned_multi_city_rows: u64,
    /// Every flagged target key (a, b or c; found or missing), by planned mentions, the first
    /// [`GUID_TWIN_LIST_CAP`].
    pub flagged: Vec<GuidTwinTargetSample>,
    pub flagged_count: u64,
}

/// The role composition of the refused groups (A7), by PARTY roles (`tender_version_parties`, bid
/// parties, result winners): reported, so R2 can read what the projection made of them. The guard itself
/// (Build 1b, [`TwinBind::RoleMix`]) reads the notice's own references, not these; the guard's
/// exclusions are `guard.role_mix` and `role_mix_by_clause`, its self-check `bars.tripwire_role_mix`.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct GuidTwinRoles {
    /// The groups the role guard bound, by [`guid_twin_role_mix`]'s clause (`buyer_review_org`,
    /// `tenderer_mixed`) over the planner's own read of their references: groups and their candidate
    /// mentions. `(rule only)`: the rule bound `role_mix` where the planner's read shows no mix.
    pub role_mix_by_clause: BTreeMap<String, GuidTwinGuardCount>,
    pub refused_groups: u64,
    /// Refused groups by the sorted set of their sections' roles, the largest
    /// [`GUID_TWIN_ROLE_SETS_MAX`]; the rest under `(rest)`.
    pub by_role_set: BTreeMap<String, u64>,
    /// A buyer-family role with `Lot-ReviewOrg` (or the legacy review body) in one group.
    pub buyer_with_review_org: u64,
    /// A tenderer or winner role with any role outside that class. "Not a winner" (A7) is read by class
    /// (`GUID_ROLE_TENDERER`, the census's `tenderer_winner`): a result winner beside a bid tenderer is
    /// one class, so coherent.
    pub tenderer_mixed: u64,
    /// Every incoherent group (either above), by notice id, the first [`GUID_TWIN_LIST_CAP`].
    pub incoherent: Vec<GuidTwinGroupSample>,
    pub incoherent_count: u64,
}

/// A9: the census's twin over GUID-bound mentions (one notice, one `org_name_norm`, two or more GUID
/// rows), now and after the plan; R3 compares the census re-run against `*_after`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct GuidTwinPostState {
    pub twin_rows: u64,
    pub twin_rows_after: u64,
    pub twin_stable_rows: u64,
    pub twin_stable_rows_after: u64,
    /// Twin rows spanning two or more Tenders, now and after.
    pub twin_multi_tender_rows: u64,
    pub twin_multi_tender_rows_after: u64,
}

/// R1's bars, each computed; `pass` is all of them (the fields marked "reported" are not bars).
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct GuidTwinBars {
    /// Planned rows, whole or partial, whose every twin partner sits on a GUID row, at or under the
    /// census's max id: the figure compared against [`GUID_TWIN_CENSUS_BAR`].
    pub planned_rows_comparable: u64,
    /// Of them, the whole rows (the dissolve count).
    pub planned_rows_comparable_whole: u64,
    pub census_bar: u64,
    pub within_census_bar: bool,
    /// Planned rows (whole or partial) minted above the census's max id (counted apart).
    pub planned_rows_above_census: u64,
    /// Planned rows (whole or partial) with a partner off GUID rows (reported, read in R2, not compared).
    pub planned_rows_off_guid_partner: u64,
    /// The decision's tripwires over planned rows (whole or partial), each must be 0: spanning 2 or
    /// more Tenders (review round 3, F4: the row's VALUE over all of its GUID rows); stable at R1's fixed
    /// threshold ([`GUID_TWIN_R1_STABLE_MENTIONS`] mentions, or a stable id) whatever the run's knob; a
    /// value in KEEP.
    pub tripwire_multi_tender: u64,
    pub tripwire_stable: u64,
    pub tripwire_keep: u64,
    /// Planned rows spanning no Tender (their notices cause no version). Not an R1 bar (R1's bar line
    /// names the three above, and jobs 2659/2660 were read under it), but the wet re-plan refuses on it:
    /// the decision's Dissolve plan dissolves a row only when its mentions span exactly 1 distinct
    /// Tender, and its Wet run re-checks every tripwire.
    pub tripwire_no_tender: u64,
    pub tripwire_rows: Vec<i64>,
    /// The run's `stable_mentions` is R1's fixed [`GUID_TWIN_R1_STABLE_MENTIONS`]: another knob moves
    /// KEEP off the decision's, so it never passes R1.
    pub stable_mentions_is_decision: bool,
    /// Not-planned controls found planned (whole or partial), and planned controls that missed.
    pub not_planned_failed: Vec<i64>,
    pub planned_failed: Vec<i64>,
    /// A11, per planned mention: its target key's country, or its found row's stored country, is not
    /// `register_jurisdiction(mention.country)` — the rule's group country checked against the stored
    /// mention's own, so a key built on another country (an RE mention keyed RE, not FR) shows here.
    /// The other half of A11 (the target's identifier is NULL) holds by construction: only
    /// identifier-NULL rows are probed.
    pub target_country_mismatch: u64,
    /// Planned mentions whose stored country is not a 2-letter code, so the A11 check cannot read it
    /// (reported, not a bar).
    pub target_country_unchecked: u64,
    /// A3 iii: (census twin ∧ ≥ 2 Tenders) ∪ stable values missing from KEEP — against the COMPILED file
    /// since Build 2 (tautological for the walk's in-memory KEEP); per VALUE over all of its GUID rows
    /// since review round 3 (F4), each value once. Any entry fails `pass`.
    pub keep_self_check_missing: Vec<String>,
    /// Build 2: whole planned rows that hold a merged identifier (a merge survivor: deleting it would
    /// strand `organization_merged_identifiers`), and whole planned rows a review table names (deleting
    /// one strands a verdict). Each must be 0; the first [`GUID_TWIN_LIST_CAP`] ids are listed.
    pub tripwire_merge_survivor: u64,
    pub merge_survivor_rows: Vec<i64>,
    pub tripwire_under_review: u64,
    pub under_review_rows: Vec<i64>,
    /// Whether the review keep-set was read (`GuidTwinPlanArgs::under_review`).
    pub under_review_checked: bool,
    /// Build 1b's role self-check, a tripwire that must be 0: REFUSED groups whose notice-reference
    /// roles are role-incoherent ([`guid_twin_role_mix`]: Lot-ReviewOrg beside a buyer-side role, or a
    /// tenderer beside another role). It reads the PLANNER's own OPT-300/OPT-301 read of the copy's
    /// `notice_ids` (the union of the group's sections' roles), independent of the roles the rule
    /// returns, so it also catches a rule that loses them (and with them A7's guard). The rule binds every
    /// such group, so a non-zero count is a rule that did not apply A7's guard to what the notice says;
    /// each is listed in `role_mix_refused` (the first [`GUID_TWIN_LIST_CAP`]).
    pub tripwire_role_mix: u64,
    pub role_mix_refused: Vec<GuidTwinGroupSample>,
    /// Twin groups (bound or refused) whose roles as the rule returned them ([`TwinGroup::roles`]) are
    /// not the planner's own read of the same sections — must be 0: the rule's role union and the
    /// planner's run over the same members and the same section → roles map. The first `samples` are
    /// listed, each with its `rule_roles` beside the planner's `ref_roles`.
    pub role_ref_disagree: u64,
    pub role_ref_disagree_samples: Vec<GuidTwinGroupSample>,
    pub pass: bool,
}

/// [`Db::plan_guid_twins`]'s answer. Read-only: Build 1 writes nothing.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct GuidTwinPlan {
    pub window: i64,
    pub stable_ids: Vec<i64>,
    pub stable_mentions: u64,
    pub extra_unsafe_names: Vec<String>,
    pub samples_per_list: usize,
    pub salt: u64,
    /// The highest organization id when the walk started.
    pub max_id: i64,
    pub windows: u64,
    pub notice_chunks: u64,
    pub runtime_ms: u64,
    /// GUID rows walked, and the distinct values over them (A12: "merge-inert" on a current number).
    pub guid_rows: u64,
    pub distinct_values: u64,
    pub guid_mentions: u64,
    /// Notices with a mention on a GUID row: every ORG mention of each is read.
    pub candidate_notices: u64,
    pub org_mentions_read: u64,
    /// ORG sections whose gated identifier is a v4 national key, and those with no name.
    pub candidate_mentions: u64,
    pub unnamed_candidates: u64,
    /// Build 1b (A7): the notice copies whose own role references were read — exactly those holding a
    /// twin group (two or more distinct values), the only copies where the role guard can bind (it only
    /// removes refusals) — the `notice_ids` rows read for them (id-refs and DE-1.x rows), the distinct
    /// (section, role) pairs those rows gave, and the read's wall time (inside `runtime_ms`).
    pub role_ref_notices: u64,
    pub role_ref_rows_read: u64,
    pub role_refs: u64,
    pub role_ref_ms: u64,
    /// Twin groups (two or more distinct values) and those the rule refuses.
    pub twin_groups: u64,
    pub refused_groups: u64,
    pub refused_mentions: u64,
    /// Guard exclusions by reason (`no_country`, `content_words`, `generic`, `unsafe_list`, `extra`,
    /// `role_mix`, `keep`: [`TwinBind::ALL`]), and the first `samples` groups of each.
    pub guard: BTreeMap<&'static str, GuidTwinGuardCount>,
    pub guard_samples: BTreeMap<&'static str, Vec<GuidTwinGroupSample>>,
    /// KEEP sections beside refused fresh ones (the "KEEP partner" exclusion).
    pub keep_partner_mentions: u64,
    /// What KEEP spares against an empty KEEP: mentions, and GUID rows with a spared mention.
    pub keep_spared: GuidTwinCount,
    /// The KEEP candidates (stable ∪ ≥ 2 Tenders ∪ merged), by value; Build 2 commits them.
    pub keep_size: u64,
    pub keep_by_reason: BTreeMap<String, u64>,
    pub keep_candidates: Vec<GuidKeepCandidate>,
    /// The plan: rows whose every mention is refused (whole) and rows with a refused and a kept one
    /// (partial), with the planned mentions of each.
    pub planned: GuidTwinCount,
    pub planned_partial: GuidTwinCount,
    pub planned_row_ids: Vec<i64>,
    pub partial_row_ids: Vec<i64>,
    /// Every planned mention in walk order, the first [`GUID_TWIN_LIST_CAP`].
    pub planned_mention_keys: Vec<GuidTwinMentionKey>,
    pub planned_mention_keys_truncated: bool,
    /// Planned (whole) rows by country, role and first year.
    pub planned_by_country: BTreeMap<String, GuidTwinCount>,
    pub planned_by_role: BTreeMap<String, GuidTwinCount>,
    pub planned_by_first_year: BTreeMap<String, GuidTwinCount>,
    pub partial: GuidTwinPartial,
    pub off_guid: GuidTwinOffGuid,
    /// Target keys, those with an identifier-NULL row, and those without (the mention would mint).
    pub targets: u64,
    pub targets_found: u64,
    pub missing_targets: GuidTwinCount,
    pub missing_target_samples: Vec<GuidTwinTargetSample>,
    pub largest_target: Option<GuidTwinTargetSample>,
    pub city: GuidTwinCityScreen,
    pub roles: GuidTwinRoles,
    pub post_state: GuidTwinPostState,
    pub bars: GuidTwinBars,
    pub controls: Vec<GuidTwinControl>,
    /// Per exhibit name (`country name_norm`): GUID rows, planned whole, partial, and the kept ids.
    pub exhibits: BTreeMap<String, GuidTwinExhibit>,
    /// R2: the random planned rows, stratified (A5), at [`GUID_TWIN_SALT`] (530003, the fresh re-read),
    /// skipping the rows R2 read at 530002.
    pub r2_random: Vec<GuidTwinRowSample>,
    /// The rows R2 read at 530002 ([`GUID_TWIN_R2_READ`]: 60), and how many planned rows (whole or
    /// partial) the random draw skipped as already read.
    pub r2_read_excluded: u64,
    pub r2_random_excluded: u64,
    /// R2: the 30 targets with the most planned mentions, plus any of the 20 with the most planned rows.
    pub r2_top_targets: Vec<GuidTwinTargetSample>,
    /// R2: planned keys of 2 content words led by an office noun (all, or the top 30 plus 15 salted).
    pub r2_office_two_words: Vec<GuidTwinTargetSample>,
    pub r2_office_two_words_count: u64,
    /// R2: the flagged targets and role-incoherent groups to read (all, or the top 30 plus 15 salted).
    pub r2_city_flagged: Vec<GuidTwinTargetSample>,
    pub r2_incoherent: Vec<GuidTwinGroupSample>,
    pub planned_samples: Vec<GuidTwinRowSample>,
    pub stopped: bool,
    /// Build 2: `compiled` (the job's KEEP, `GuidTwinPlanArgs::keep`) or `walk` (Build 1's in-memory
    /// KEEP), its version, and the unsafe-name list the rule read. `keep_size` is that KEEP's size.
    pub keep_source: &'static str,
    pub keep_version: String,
    pub unsafe_names: Vec<String>,
    /// Build 2 (A3 iii): `keep_self_check_missing`'s rows as KEEP file lines (`country`, `value`,
    /// `org_id`, `mentions`, `tenders`, `reason`, tab-separated): the KEEP-add commit a failed check asks
    /// for. Advisory beside it: walk candidates (merged values, multi-Tender rows that are no census
    /// twin) absent from the compiled file, counted with the first [`GUID_TWIN_LIST_CAP`] lines.
    pub keep_add_commit: Vec<String>,
    pub keep_candidates_not_in_file: u64,
    pub keep_candidates_not_in_file_lines: Vec<String>,
    /// The highest notice id when the plan started. A wet run refuses a `gate_since_notice_id` above it
    /// (A1's boundary is read before the deploy, so it can never exceed a post-deploy plan's start).
    pub notice_max_id: i64,
    /// The highest `guid_twin_dissolve_log` rowid when the plan started (0 for an empty log): a wet run
    /// on this plan adds back exactly the rows logged after it (a staged run, a restarted job), and its
    /// manifest covers them (the campaign on this plan).
    pub log_high_water: i64,
    /// OQ1: notice copies holding a refused section the copy references as `Procedure-Buyer` (the
    /// planner's own read of the twin copies' references): the copies whose buyer the gate rebinds while
    /// the plan row's buyer key keeps the published identity.
    pub refused_buyer_copies: u64,
    /// Review round 3 (F3): refused mentions a reviewed rehoming verdict names
    /// ([`GuidTwinPlanArgs::rehomed`]) — left where the verdict put them, neither planned nor off-GUID —
    /// and the first `samples` of them (`kind` `guid_row` or `off_guid`; `target` unset).
    pub verdict_kept: u64,
    pub verdict_kept_samples: Vec<GuidTwinOffGuidSample>,
    /// Review round 3 (F4): values on two or more GUID rows (a DE + AT pair, issue 62's duplicate triple),
    /// each with its rows, their Tender counts and the union — "spans >= 2 Tenders" (the KEEP candidates,
    /// the several-Tenders tripwire, the compiled self-check) is counted per VALUE over all of its rows,
    /// as A1's recurrence arm counts it. The first [`GUID_TWIN_LIST_CAP`], and how many.
    pub multi_row_values: Vec<GuidTwinMultiRowValue>,
    pub multi_row_value_count: u64,
    /// Every planned mention, uncapped, for the wet run ([`guid_twin_windows`]); not serialized.
    #[serde(skip)]
    pub wet_mentions: Vec<GuidTwinWetMention>,
}

/// Build 2: one planned mention as the wet run applies it — the `(notice, section)` key, the GUID row it
/// sits on (id, value, country, name: the restore manifest's pre-image), its target KEY
/// `(org_name_norm, register_jurisdiction(country))`, the key's identifier-NULL row when the plan found
/// one, the name a missing key's row is minted under (the published name of the key's lowest planned
/// `(notice, section)`), and whether its row is planned whole.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct GuidTwinWetMention {
    pub notice_id: i64,
    pub section_id: String,
    pub org_id: i64,
    pub org_value: String,
    pub org_country: Option<String>,
    pub org_name: String,
    pub key_name_norm: String,
    pub key_country: Option<String>,
    pub target: Option<i64>,
    pub mint_name: String,
    pub whole: bool,
    /// Review round 3 (F1): the target row's existing mentions when the plan read it (0 for a missing
    /// key): 542's re-derivation re-reads every one of them inside the window that re-binds the target,
    /// however few mentions the window moves there — the window's cost driver.
    pub target_mentions: u64,
    /// The mentions the row KEEPS (0 for a whole row, which is deleted): a rehomed row is re-bound too, so
    /// its kept mentions are re-read by the same re-derivation.
    pub row_kept_mentions: u64,
}

/// Review round 3 (F4): one value held by two or more GUID rows ([`GuidTwinPlan::multi_row_values`]).
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct GuidTwinMultiRowValue {
    /// The undashed upper-case value ([`GuidKeep::key`]).
    pub value: String,
    /// Its GUID rows by id, and each row's own distinct Tenders.
    pub rows: Vec<i64>,
    pub row_tenders: Vec<u64>,
    /// The distinct Tenders over every row's mentions: what the tripwire and the self-check read.
    pub tenders: u64,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct GuidTwinExhibit {
    pub guid_rows: u64,
    pub planned: u64,
    pub partial: u64,
    /// The partial rows' ids: a rehome keeps the row and its identifier, so they stay GUID rows after the
    /// wet run (A10's surviving set is `kept_ids` ∪ `partial_ids`).
    pub partial_ids: Vec<i64>,
    pub kept: u64,
    pub kept_ids: Vec<i64>,
}

/// One GUID row as the planner keeps it.
struct Row {
    id: i64,
    country: Option<String>,
    identifier: String,
    name: String,
    mentions: u64,
    notices: Vec<i64>,
    tenders: u64,
    first_year: Option<i64>,
    role: &'static str,
    stable: bool,
    /// Refused ORG mentions on this row, and their indexes in `Walk::planned`.
    refused: u64,
    planned: Vec<u32>,
    /// False once a refused group of its mentions has a partner off GUID rows.
    comparable: bool,
    twin: bool,
    twin_after: bool,
    keep_spared: bool,
}

/// One notice as the walk keeps it.
#[derive(Default)]
struct NoticeInfo {
    source: u32,
    year: Option<i64>,
    versions: Vec<(i64, i64)>,
}

/// A refused mention on a GUID row: a planned mention.
struct Planned {
    notice: i64,
    section: String,
    row: u32,
    target: u32,
    city: Option<String>,
    /// The stored mention's country (A11's check reads it).
    country: Option<String>,
    /// The mention's published name (a missing key's mint name, Build 2).
    name: String,
}

/// A refused mention bound to a row that is not a GUID row.
struct OffGuid {
    notice: i64,
    section: String,
    org: i64,
    value: String,
    target: u32,
}

/// A refused group, kept for the role composition.
struct RefusedGroup {
    notice: i64,
    name_norm: String,
    country: Option<String>,
    /// (section, bound org, value, refused)
    members: Vec<(String, i64, String, bool)>,
    /// The members' notice-reference roles as the PLANNER read them ([`GuidTwinGroupSample::ref_roles`]),
    /// not the rule's echo: the role tripwire reads these.
    ref_roles: Vec<String>,
}

/// A target key and what is known of its row.
#[derive(Default)]
struct Target {
    name_norm: String,
    country: Option<String>,
    id: Option<i64>,
    /// The row's name once found; until then (and for a missing key) a refused mention's published name.
    name: String,
    /// The found row's stored country (A11's check reads it).
    row_country: Option<String>,
    mentions: u64,
    newest: Vec<(i64, String)>,
    cities: BTreeSet<String>,
    planned_mentions: u64,
    planned_rows: HashSet<u32>,
    planned_cities: BTreeSet<String>,
    city_absent: u64,
    office_two_words: bool,
}

#[derive(Default)]
struct Walk {
    rows: Vec<Row>,
    row_at: HashMap<i64, u32>,
    notices: HashMap<i64, NoticeInfo>,
    sources: Vec<String>,
    source_at: HashMap<String, u32>,
    planned: Vec<Planned>,
    off_guid: Vec<OffGuid>,
    groups: Vec<RefusedGroup>,
    targets: Vec<Target>,
    target_at: HashMap<(String, Option<String>), u32>,
    /// Review round 3 (F4): each value's ([`GuidKeep::key`]) distinct Tenders over EVERY GUID row holding
    /// it, and those rows (walk indexes, by id): "spans >= 2 Tenders" is counted per value.
    value_tenders: HashMap<String, BTreeSet<i64>>,
    value_rows: HashMap<String, Vec<u32>>,
}

impl Walk {
    /// One row's distinct Tenders: its notices' versions.
    fn row_tenders(&self, row: &Row) -> BTreeSet<i64> {
        row.notices
            .iter()
            .filter_map(|n| self.notices.get(n))
            .flat_map(|info| info.versions.iter().map(|(t, _)| *t))
            .collect()
    }

    /// Fills [`Walk::value_tenders`] and [`Walk::value_rows`] once every row and its notices are read.
    fn index_values(&mut self) {
        let mut tenders: HashMap<String, BTreeSet<i64>> = HashMap::new();
        let mut rows: HashMap<String, Vec<u32>> = HashMap::new();
        for (i, row) in self.rows.iter().enumerate() {
            let key = GuidKeep::key(&row.identifier);
            tenders.entry(key.clone()).or_default().extend(self.row_tenders(row));
            rows.entry(key).or_default().push(i as u32);
        }
        self.value_tenders = tenders;
        self.value_rows = rows;
    }

    /// A row's value's distinct Tenders over all of its GUID rows (F4).
    fn value_tender_count(&self, row: &Row) -> u64 {
        self.value_tenders.get(&GuidKeep::key(&row.identifier)).map_or(row.tenders, |t| t.len() as u64)
    }
    fn source(&mut self, source: &str) -> u32 {
        if let Some(&id) = self.source_at.get(source) {
            return id;
        }
        let id = self.sources.len() as u32;
        self.sources.push(source.to_owned());
        self.source_at.insert(source.to_owned(), id);
        id
    }

    fn target(&mut self, name_norm: &str, country: Option<&str>) -> u32 {
        let key = (name_norm.to_owned(), country.map(str::to_owned));
        if let Some(&at) = self.target_at.get(&key) {
            return at;
        }
        let at = self.targets.len() as u32;
        self.targets.push(Target { name_norm: key.0.clone(), country: key.1.clone(), ..Default::default() });
        self.target_at.insert(key, at);
        at
    }
}

/// `((id XOR salt) × 2654435761) mod 2³²`, ties by id: R2's salted-uniform order (`head_election`'s).
pub fn guid_twin_sample_key(id: i64, salt: u64) -> (u64, i64) {
    (((id as u64) ^ salt).wrapping_mul(2_654_435_761) & 0xFFFF_FFFF, id)
}

/// A published city, folded: lower case, Latin diacritics folded, punctuation dropped, spaces collapsed.
pub fn guid_twin_city_fold(city: &str) -> String {
    let folded = buyer_name_fold(&org_name_norm(city));
    folded
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// The top `top` of `items` by `rank` (descending), plus `salted` more of the rest at the salt: what
/// R2 reads of a class larger than [`GUID_TWIN_READ_ALL`]; a smaller class is read whole.
fn r2_pick<T: Clone>(items: &[T], rank: impl Fn(&T) -> (u64, i64), salt_id: impl Fn(&T) -> i64) -> Vec<T> {
    let mut order: Vec<usize> = (0..items.len()).collect();
    order.sort_by_key(|&i| {
        let (r, id) = rank(&items[i]);
        (std::cmp::Reverse(r), id)
    });
    if items.len() <= GUID_TWIN_READ_ALL {
        return order.into_iter().map(|i| items[i].clone()).collect();
    }
    let mut picked: Vec<usize> = order.iter().copied().take(30).collect();
    let mut rest: Vec<usize> = order.iter().copied().skip(30).collect();
    rest.sort_by_key(|&i| guid_twin_sample_key(salt_id(&items[i]), GUID_TWIN_SALT));
    picked.extend(rest.into_iter().take(15));
    picked.into_iter().map(|i| items[i].clone()).collect()
}

/// Whether R2 already read `org_id` in its random sample ([`GUID_TWIN_R2_READ`]).
pub fn guid_twin_r2_read(org_id: i64) -> bool {
    GUID_TWIN_R2_READ.binary_search(&org_id).is_ok()
}

/// R2's random planned rows (A5): `rows` as `(role class, org id)`; per stratum of
/// [`GUID_TWIN_R2_STRATA`], in that order (buyer, review_or_info, other or tenderer), the rows with the
/// smallest [`guid_twin_sample_key`] at [`GUID_TWIN_SALT`], up to the stratum's size — the rows R2 read
/// already ([`GUID_TWIN_R2_READ`]) skipped first, so the next in salted order take their places.
/// Indexes into `rows`, and how many rows were skipped as read.
fn r2_random_pick(rows: &[(&str, i64)]) -> (Vec<usize>, u64) {
    let mut strata: BTreeMap<&str, Vec<((u64, i64), usize)>> = BTreeMap::new();
    let mut skipped = 0u64;
    for (i, &(role, id)) in rows.iter().enumerate() {
        if guid_twin_r2_read(id) {
            skipped += 1;
            continue;
        }
        let stratum = match role {
            GUID_ROLE_BUYER => "buyer",
            GUID_ROLE_REVIEW_INFO => "review_or_info",
            _ => "other_or_tenderer",
        };
        strata.entry(stratum).or_default().push((guid_twin_sample_key(id, GUID_TWIN_SALT), i));
    }
    let mut out = Vec::new();
    for (stratum, n) in GUID_TWIN_R2_STRATA {
        if let Some(list) = strata.get_mut(stratum) {
            list.sort_unstable();
            out.extend(list.iter().take(n).map(|&(_, i)| i));
        }
    }
    (out, skipped)
}

/// R2's top targets (decision R2 (1)): the 30 target keys with the most planned mentions, ties by
/// `sample_id`, then any of the 20 with the most planned rows not among them. Keys, found or missing,
/// are told apart by `(name_norm, country)`, never by `target_id` (every missing key's is `None`).
fn r2_top_targets(mut targets: Vec<GuidTwinTargetSample>) -> Vec<GuidTwinTargetSample> {
    targets.sort_by_key(|s| (std::cmp::Reverse(s.planned_mentions), s.sample_id));
    let mut picked: Vec<GuidTwinTargetSample> = targets.iter().take(30).cloned().collect();
    targets.sort_by_key(|s| (std::cmp::Reverse(s.planned_rows), s.sample_id));
    for s in targets.into_iter().take(20) {
        if !picked.iter().any(|p| p.name_norm == s.name_norm && p.country == s.country) {
            picked.push(s);
        }
    }
    picked
}

impl Db {
    /// Issue 530 unit 3, Build 1: the DRY `dissolve-guid-twins` plan ([`GuidTwinPlan`] names what it
    /// reports). Writes nothing.
    ///
    /// 1. Walks `organizations` by rowid windows of `args.window` ids (the census's statement) up to the
    ///    highest id at its start, keeping rows whose identifier passes `args.is_guid`, and reads them in
    ///    batches: their mentions through `organization_mentions_org`, the new notices by primary key,
    ///    the Tender versions through `tender_versions_notice`, and the row's role class through the
    ///    party, bid-party and winner `org` indexes (the census's reads).
    /// 2. Walks `organization_merged_identifiers` by rowid windows for the merged v4 values (A3 ii) and
    ///    computes KEEP: stable rows, rows spanning ≥ 2 Tenders, merged values.
    /// 3. Reads EVERY ORG mention of each candidate notice (one with a mention on a GUID row) in IN
    ///    chunks of notice ids on the primary-key prefix, and applies `args.rule` to each notice copy
    ///    alone, with KEEP and again with an empty KEEP (what KEEP spares). Build 1b (A7): a copy whose
    ///    rule run (roles not yet read) shows a twin group first has its own role references read
    ///    ([`guid_twin_role_refs_sql`], the `notice_ids` primary-key prefix, one IN statement per
    ///    chunk) and folded by [`guid_twin_fold_roles`] with `args.role_ref`, and the rule decides it
    ///    with those roles. A copy with no twin group needs none: the role guard only binds groups.
    /// 4. Classifies refused mentions bound off GUID rows (by primary key and the survivor index),
    ///    finds each target (the 234 index), counts its mentions and reads its newest
    ///    [`GUID_TWIN_TARGET_NEWEST`] (reverse index walk), reads the BT-513 cities of the planned
    ///    mentions and the targets' mentions (`notice_texts` prefix), and the refused groups' roles
    ///    (mention-keyed party indexes, winners by the result table's prefix).
    ///
    /// Every statement is a rowid window or an IN chunk of at most [`IN_CHUNK`] keys, or one bounded
    /// seek. The stop flag is read before every window, batch and chunk, and every 64 target seeks; a
    /// stopped plan is an empty report marked `stopped`. Each of [`GUID_TWIN_INDEXES`] must stand.
    pub async fn plan_guid_twins(&self, args: GuidTwinPlanArgs<'_>) -> turso::Result<GuidTwinPlan> {
        let started = Instant::now();
        for index in GUID_TWIN_INDEXES {
            if !self.has_index(index).await? {
                return Err(turso::Error::Error(format!(
                    "dissolve-guid-twins refused: {index} is absent (a rebuild?), so a chunk would scan its \
                     table; run `reindex` first. Nothing was read"
                )));
            }
        }
        let stopped = || GuidTwinPlan { stopped: true, ..Default::default() };
        let mut plan = GuidTwinPlan {
            window: args.window.max(1),
            stable_ids: args.stable_ids.to_vec(),
            stable_mentions: args.stable_mentions,
            extra_unsafe_names: args.extra_unsafe_names.to_vec(),
            samples_per_list: args.samples,
            salt: GUID_TWIN_SALT,
            r2_read_excluded: GUID_TWIN_R2_READ.len() as u64,
            keep_source: if args.keep.is_some() { "compiled" } else { "walk" },
            keep_version: args.keep_version.to_owned(),
            unsafe_names: args.unsafe_names.iter().map(|n| (*n).to_owned()).collect(),
            ..Default::default()
        };
        let reader = self.reader().await?;
        let mut rows = reader.query(GUID_CENSUS_MAX_ID_SQL, ()).await?;
        plan.max_id = match rows.next().await? {
            Some(row) => opt_int_of(&row, 0).unwrap_or(0),
            None => 0,
        };
        drop(rows);
        // Build 2 (A1): the newest notice at the plan's start, the wet run's estimated gate boundary.
        let mut rows = reader.query(GUID_TWIN_NOTICE_MAX_ID_SQL, ()).await?;
        plan.notice_max_id = match rows.next().await? {
            Some(row) => opt_int_of(&row, 0).unwrap_or(0),
            None => 0,
        };
        drop(rows);
        // Build 2: the dissolve log's high-water mark at the plan's start.
        let mut rows = reader.query(GUID_TWIN_LOG_HIGH_WATER_SQL, ()).await?;
        plan.log_high_water = match rows.next().await? {
            Some(row) => opt_int_of(&row, 0).unwrap_or(0),
            None => 0,
        };
        drop(rows);

        // 1. The GUID rows.
        let width = args.batch.clamp(1, GUID_TWIN_BATCH);
        let mut walk = Walk::default();
        let mut pending: Vec<(i64, Option<String>, String, String)> = Vec::new();
        let mut lo = 0i64;
        loop {
            while pending.len() >= width || (lo >= plan.max_id && !pending.is_empty()) {
                if (args.stop)() {
                    return Ok(stopped());
                }
                let batch: Vec<_> = pending.drain(..pending.len().min(width)).collect();
                if !self.guid_twin_rows_batch(&reader, &args, &mut walk, batch).await? {
                    return Ok(stopped());
                }
            }
            if lo >= plan.max_id {
                break;
            }
            if (args.stop)() {
                return Ok(stopped());
            }
            let hi = lo.saturating_add(plan.window).min(plan.max_id);
            let mut rows = reader.query(GUID_CENSUS_ORGS_SQL, (Value::Integer(lo), Value::Integer(hi))).await?;
            while let Some(row) = rows.next().await? {
                let identifier = text(&row, 2);
                if (args.is_guid)(&identifier) {
                    pending.push((int(&row, 0), opt_text_of(&row, 1), identifier, text(&row, 3)));
                }
            }
            drop(rows);
            plan.windows += 1;
            lo = hi;
            (args.progress)(
                lo.max(0) as u64,
                &format!("issue 530: GUID rows to org id {lo} of {}, {} found", plan.max_id, walk.rows.len()),
            );
        }
        let stable_ids: HashSet<i64> = args.stable_ids.iter().copied().collect();
        for row in &mut walk.rows {
            row.stable = stable_ids.contains(&row.id) || row.mentions >= args.stable_mentions;
        }
        plan.guid_rows = walk.rows.len() as u64;
        plan.distinct_values = walk.rows.iter().map(|r| r.identifier.as_str()).collect::<HashSet<_>>().len() as u64;
        plan.guid_mentions = walk.rows.iter().map(|r| r.mentions).sum();
        // Review round 3 (F4): each value's Tenders over all of its GUID rows.
        walk.index_values();

        // 2. The merged v4 values, then KEEP.
        let mut merged: Vec<(String, Option<String>, i64)> = Vec::new();
        let mut rows = reader.query(GUID_TWIN_MERGED_MAX_SQL, ()).await?;
        let merged_max = match rows.next().await? {
            Some(row) => opt_int_of(&row, 0).unwrap_or(0),
            None => 0,
        };
        drop(rows);
        let mut lo = 0i64;
        while lo < merged_max {
            if (args.stop)() {
                return Ok(stopped());
            }
            let hi = lo.saturating_add(plan.window).min(merged_max);
            let mut rows = reader.query(GUID_TWIN_MERGED_SQL, (Value::Integer(lo), Value::Integer(hi))).await?;
            while let Some(row) = rows.next().await? {
                let identifier = text(&row, 0);
                if (args.is_guid)(&identifier) {
                    merged.push((identifier, opt_text_of(&row, 1), int(&row, 2)));
                }
            }
            drop(rows);
            plan.windows += 1;
            lo = hi;
        }
        let mut candidates: BTreeMap<String, GuidKeepCandidate> = BTreeMap::new();
        for row in &walk.rows {
            // F4: a value's Tenders over all of its GUID rows, as the self-check and A1 count them.
            let value_tenders = walk.value_tender_count(row);
            let mut reasons = Vec::new();
            if row.stable {
                reasons.push("stable");
            }
            if value_tenders >= 2 {
                reasons.push("multi_tender");
            }
            if reasons.is_empty() {
                continue;
            }
            let value = GuidKeep::key(&row.identifier);
            let entry = candidates.entry(value.clone()).or_insert_with(|| GuidKeepCandidate {
                country: row.country.clone(),
                value,
                org_id: row.id,
                mentions: Some(row.mentions),
                tenders: Some(value_tenders),
                reason: String::new(),
            });
            for reason in reasons {
                if !entry.reason.split('+').any(|r| r == reason) {
                    if !entry.reason.is_empty() {
                        entry.reason.push('+');
                    }
                    entry.reason.push_str(reason);
                }
            }
        }
        for (identifier, country, holder) in &merged {
            let value = GuidKeep::key(identifier);
            let entry = candidates.entry(value.clone()).or_insert_with(|| GuidKeepCandidate {
                country: country.clone(),
                value,
                org_id: *holder,
                mentions: None,
                tenders: None,
                reason: String::new(),
            });
            if !entry.reason.split('+').any(|r| r == "merged") {
                if !entry.reason.is_empty() {
                    entry.reason.push('+');
                }
                entry.reason.push_str("merged");
            }
        }
        // Build 2 (A3 iii): the rule reads the COMPILED KEEP when the job passes it; the walk's candidates
        // stay the report's list and the self-check's other side.
        let walk_keep: GuidKeep = candidates.keys().collect();
        let keep: &GuidKeep = args.keep.unwrap_or(&walk_keep);
        let empty_keep = GuidKeep::default();
        plan.keep_size = keep.len() as u64;
        for c in candidates.values() {
            *plan.keep_by_reason.entry(c.reason.clone()).or_default() += 1;
        }
        plan.keep_candidates = candidates.into_values().collect();

        // 3. Every ORG mention of each candidate notice, the rule per notice copy.
        let mut notice_ids: Vec<i64> = walk.notices.keys().copied().collect();
        notice_ids.sort_unstable();
        plan.candidate_notices = notice_ids.len() as u64;
        for (k, chunk) in notice_ids.chunks(IN_CHUNK).enumerate() {
            if (args.stop)() {
                return Ok(stopped());
            }
            let params: Vec<Value> = chunk.iter().map(|&id| Value::Integer(id)).collect();
            let mut rows = reader.query(&guid_twin_notice_mentions_sql(chunk.len()), params).await?;
            let mut by_notice: BTreeMap<i64, Vec<MentionRow>> = BTreeMap::new();
            while let Some(row) = rows.next().await? {
                by_notice.entry(int(&row, 0)).or_default().push(MentionRow {
                    section: text(&row, 1),
                    org: int(&row, 2),
                    name: text(&row, 3),
                    country: opt_text_of(&row, 4),
                    raw: opt_text_of(&row, 5),
                    scheme: opt_text_of(&row, 6),
                });
            }
            drop(rows);
            plan.notice_chunks += 1;
            // A7's roles, read only for the copies holding a twin group (the guard binds groups only,
            // so a copy without one decides alike with or without roles).
            let twins: Vec<i64> = by_notice
                .iter()
                .filter(|(_, mentions)| guid_twin_has_group(&args, keep, mentions))
                .map(|(notice, _)| *notice)
                .collect();
            let roles = if twins.is_empty() {
                HashMap::new()
            } else {
                if (args.stop)() {
                    return Ok(stopped());
                }
                let at = Instant::now();
                let roles = guid_twin_read_roles(&reader, &twins, args.role_ref, &mut plan).await?;
                plan.role_ref_ms += at.elapsed().as_millis() as u64;
                roles
            };
            let no_roles = GuidTwinSectionRoles::new();
            for (notice, mentions) in by_notice {
                plan.org_mentions_read += mentions.len() as u64;
                let section_roles = roles.get(&notice).unwrap_or(&no_roles);
                guid_twin_notice(&args, keep, &empty_keep, &mut walk, &mut plan, notice, &mentions, section_roles);
            }
            (args.progress)(
                ((k + 1) * IN_CHUNK).min(notice_ids.len()) as u64,
                &format!(
                    "issue 530: rule 530-T over notice {} of {} candidate notices, {} refused mention(s)",
                    ((k + 1) * IN_CHUNK).min(notice_ids.len()),
                    notice_ids.len(),
                    plan.refused_mentions
                ),
            );
        }

        // 4a. Refused mentions bound off GUID rows: the bound row's kind.
        let off_orgs: Vec<i64> = walk.off_guid.iter().map(|o| o.org).collect::<BTreeSet<_>>().into_iter().collect();
        // The bound row's identifier (`None`: identifier NULL); an org with no row is absent.
        let mut identified: HashMap<i64, Option<String>> = HashMap::new();
        let mut survivors: HashSet<i64> = HashSet::new();
        for chunk in off_orgs.chunks(IN_CHUNK) {
            if (args.stop)() {
                return Ok(stopped());
            }
            let params: Vec<Value> = chunk.iter().map(|&id| Value::Integer(id)).collect();
            let mut rows = reader.query(&guid_twin_orgs_sql(chunk.len()), params.clone()).await?;
            while let Some(row) = rows.next().await? {
                identified.insert(int(&row, 0), opt_text_of(&row, 1));
            }
            drop(rows);
            let mut rows = reader.query(&guid_twin_survivors_sql(chunk.len()), params).await?;
            while let Some(row) = rows.next().await? {
                survivors.insert(int(&row, 0));
            }
        }

        // 4b. The targets: the identifier-NULL row of each key, its count and its newest mentions.
        let names: Vec<String> =
            walk.targets.iter().map(|t| t.name_norm.clone()).collect::<BTreeSet<_>>().into_iter().collect();
        for chunk in names.chunks(IN_CHUNK) {
            if (args.stop)() {
                return Ok(stopped());
            }
            let params: Vec<Value> = chunk.iter().map(|n| Value::Text(n.clone())).collect();
            let mut rows = reader.query(&guid_twin_targets_sql(chunk.len()), params).await?;
            while let Some(row) = rows.next().await? {
                let (id, norm, country) = (int(&row, 0), text(&row, 1), opt_text_of(&row, 2));
                if let Some(&at) = walk.target_at.get(&(norm, country.clone())) {
                    let target = &mut walk.targets[at as usize];
                    if target.id.is_none_or(|have| id < have) {
                        target.id = Some(id);
                        target.name = text(&row, 3);
                        target.row_country = country;
                    }
                }
            }
        }
        for (k, target) in walk.targets.iter_mut().enumerate() {
            // The checkpoint counts every target, missing ones included, so at most 64 seek pairs run
            // between two stop reads.
            if k % 64 == 0 && (args.stop)() {
                return Ok(stopped());
            }
            let Some(id) = target.id else { continue };
            let mut rows = reader.query(GUID_TWIN_TARGET_COUNT_SQL, vec![Value::Integer(id)]).await?;
            target.mentions = match rows.next().await? {
                Some(row) => int(&row, 0).max(0) as u64,
                None => 0,
            };
            drop(rows);
            let params = vec![Value::Integer(id), Value::Integer(GUID_TWIN_TARGET_NEWEST)];
            let mut rows = reader.query(GUID_TWIN_TARGET_NEWEST_SQL, params).await?;
            while let Some(row) = rows.next().await? {
                target.newest.push((int(&row, 0), text(&row, 1)));
            }
        }

        // 4c. The cities: each planned mention's, and each target's newest mentions'.
        let mut wanted: HashSet<(i64, String)> = HashSet::new();
        for p in &walk.planned {
            wanted.insert((p.notice, p.section.clone()));
        }
        for t in &walk.targets {
            for key in &t.newest {
                wanted.insert(key.clone());
            }
        }
        let city_notices: Vec<i64> = wanted.iter().map(|(n, _)| *n).collect::<BTreeSet<_>>().into_iter().collect();
        let mut cities: HashMap<(i64, String), (i64, String)> = HashMap::new();
        for chunk in city_notices.chunks(IN_CHUNK) {
            if (args.stop)() {
                return Ok(stopped());
            }
            let mut params: Vec<Value> = chunk.iter().map(|&id| Value::Integer(id)).collect();
            params.push(Value::Text(GUID_TWIN_CITY_FIELD.to_owned()));
            let mut rows = reader.query(&guid_twin_cities_sql(chunk.len()), params).await?;
            while let Some(row) = rows.next().await? {
                let key = (int(&row, 0), text(&row, 1));
                if !wanted.contains(&key) {
                    continue;
                }
                let (ordinal, city) = (int(&row, 2), guid_twin_city_fold(&text(&row, 3)));
                if city.is_empty() {
                    continue;
                }
                match cities.get(&key) {
                    Some((have, _)) if *have <= ordinal => {}
                    _ => {
                        cities.insert(key, (ordinal, city));
                    }
                }
            }
        }
        for p in &mut walk.planned {
            p.city = cities.get(&(p.notice, p.section.clone())).map(|(_, c)| c.clone());
        }
        for t in &mut walk.targets {
            t.cities = t.newest.iter().filter_map(|key| cities.get(key).map(|(_, c)| c.clone())).collect();
        }

        // 4d. The refused groups' roles: parties and bid parties by mention, winners by result row.
        let mut roles: HashMap<(i64, String), BTreeSet<String>> = HashMap::new();
        let group_notices: Vec<i64> = walk.groups.iter().map(|g| g.notice).collect::<BTreeSet<_>>().into_iter().collect();
        for chunk in group_notices.chunks(IN_CHUNK) {
            if (args.stop)() {
                return Ok(stopped());
            }
            let params: Vec<Value> = chunk.iter().map(|&id| Value::Integer(id)).collect();
            for (sql, prefix) in
                [(guid_twin_parties_sql(chunk.len()), ""), (guid_twin_bid_parties_sql(chunk.len()), "bid-")]
            {
                let mut rows = reader.query(&sql, params.clone()).await?;
                while let Some(row) = rows.next().await? {
                    roles.entry((int(&row, 0), text(&row, 1))).or_default().insert(format!("{prefix}{}", text(&row, 2)));
                }
            }
        }
        let mut versions: BTreeSet<i64> = BTreeSet::new();
        for notice in &group_notices {
            if let Some(info) = walk.notices.get(notice) {
                versions.extend(info.versions.iter().map(|(t, _)| *t));
            }
        }
        let versions: Vec<i64> = versions.into_iter().collect();
        let mut winners: HashSet<(i64, i64, i64)> = HashSet::new();
        for chunk in versions.chunks(IN_CHUNK) {
            if (args.stop)() {
                return Ok(stopped());
            }
            let params: Vec<Value> = chunk.iter().map(|&id| Value::Integer(id)).collect();
            let mut rows = reader.query(&guid_twin_result_winners_sql(chunk.len()), params).await?;
            while let Some(row) = rows.next().await? {
                winners.insert((int(&row, 0), int(&row, 1), int(&row, 2)));
            }
        }
        for g in &walk.groups {
            let Some(info) = walk.notices.get(&g.notice) else { continue };
            for (section, org, _, _) in &g.members {
                if info.versions.iter().any(|&(t, s)| winners.contains(&(t, s, *org))) {
                    roles.entry((g.notice, section.clone())).or_default().insert("result-winner".to_owned());
                }
            }
        }
        // 4e (Build 2). Whole planned rows that are merge survivors: deleting one strands its
        // `organization_merged_identifiers` rows (the survivor index, IN chunks).
        let whole_ids: Vec<i64> =
            walk.rows.iter().filter(|r| r.refused > 0 && r.refused >= r.mentions).map(|r| r.id).collect();
        let mut whole_survivors: HashSet<i64> = HashSet::new();
        for chunk in whole_ids.chunks(IN_CHUNK) {
            if (args.stop)() {
                return Ok(stopped());
            }
            let params: Vec<Value> = chunk.iter().map(|&id| Value::Integer(id)).collect();
            let mut rows = reader.query(&guid_twin_survivors_sql(chunk.len()), params).await?;
            while let Some(row) = rows.next().await? {
                whole_survivors.insert(int(&row, 0));
            }
        }
        // 4f (Build 2). The controls the walk did not find: what an earlier wet run logged for them (a
        // seek per control on `guid_twin_dissolve_log_org`).
        let control_ids: BTreeSet<i64> = args
            .not_planned
            .iter()
            .copied()
            .chain(args.planned.iter().map(|(id, _)| *id))
            .chain(args.expected.iter().map(|(id, _)| *id))
            .filter(|id| !walk.row_at.contains_key(id))
            .collect();
        let mut logged_controls: HashMap<i64, Vec<(i64, bool)>> = HashMap::new();
        for id in control_ids {
            let mut rows = reader.query(GUID_TWIN_LOG_ORG_SQL, (Value::Integer(id),)).await?;
            while let Some(row) = rows.next().await? {
                logged_controls.entry(id).or_default().push((int(&row, 0), int(&row, 1) != 0));
            }
        }
        drop(reader);

        let reads = GuidTwinReads {
            identified: &identified,
            survivors: &survivors,
            whole_survivors: &whole_survivors,
            roles: &roles,
            logged_controls: &logged_controls,
        };
        guid_twin_finish(&args, &mut plan, &mut walk, &reads, keep, &stable_ids);
        plan.runtime_ms = started.elapsed().as_millis() as u64;
        Ok(plan)
    }

    /// One batch of GUID rows: their mentions, new notices, versions and role class, into `walk`.
    /// `false` when the stop flag fired before one of its chunks.
    async fn guid_twin_rows_batch(
        &self,
        reader: &crate::read::Reader,
        args: &GuidTwinPlanArgs<'_>,
        walk: &mut Walk,
        batch: Vec<(i64, Option<String>, String, String)>,
    ) -> turso::Result<bool> {
        let at: HashMap<i64, usize> = batch.iter().enumerate().map(|(i, p)| (p.0, i)).collect();
        let ids: Vec<i64> = batch.iter().map(|p| p.0).collect();
        let mut mentions = vec![0u64; batch.len()];
        let mut notices: Vec<Vec<i64>> = vec![Vec::new(); batch.len()];
        for chunk in ids.chunks(IN_CHUNK) {
            if (args.stop)() {
                return Ok(false);
            }
            let params: Vec<Value> = chunk.iter().map(|&id| Value::Integer(id)).collect();
            let mut rows = reader.query(&guid_twin_row_mentions_sql(chunk.len()), params).await?;
            while let Some(row) = rows.next().await? {
                let Some(&i) = at.get(&int(&row, 0)) else { continue };
                mentions[i] += 1;
                notices[i].push(int(&row, 1));
            }
        }
        let mut fresh: BTreeSet<i64> = BTreeSet::new();
        for list in &mut notices {
            list.sort_unstable();
            list.dedup();
            fresh.extend(list.iter().copied().filter(|n| !walk.notices.contains_key(n)));
        }
        let fresh: Vec<i64> = fresh.into_iter().collect();
        for chunk in fresh.chunks(IN_CHUNK) {
            if (args.stop)() {
                return Ok(false);
            }
            let params: Vec<Value> = chunk.iter().map(|&id| Value::Integer(id)).collect();
            let mut rows = reader.query(&guid_twin_notices_sql(chunk.len()), params.clone()).await?;
            while let Some(row) = rows.next().await? {
                let source = walk.source(&text(&row, 1));
                let info = walk.notices.entry(int(&row, 0)).or_default();
                info.source = source;
                info.year = opt_int_of(&row, 2).map(unix_year);
            }
            drop(rows);
            let mut rows = reader.query(&guid_twin_versions_sql(chunk.len()), params).await?;
            while let Some(row) = rows.next().await? {
                walk.notices.entry(int(&row, 0)).or_default().versions.push((int(&row, 1), int(&row, 2)));
            }
        }
        // A notice with no `notices` row still counts as a candidate (its mentions are read).
        let unknown = walk.source("(no notice)");
        for n in &fresh {
            walk.notices.entry(*n).or_insert_with(|| NoticeInfo { source: unknown, ..Default::default() });
        }
        let class_bit = |class: &str| 1u8 << GUID_ROLE_CLASSES.iter().position(|c| *c == class).unwrap_or(3);
        let mut role_bits = vec![0u8; batch.len()];
        for chunk in ids.chunks(IN_CHUNK) {
            if (args.stop)() {
                return Ok(false);
            }
            let params: Vec<Value> = chunk.iter().map(|&id| Value::Integer(id)).collect();
            let mut rows = reader.query(&party_role_counts_sql(chunk.len()), params.clone()).await?;
            while let Some(row) = rows.next().await? {
                if let Some(&i) = at.get(&int(&row, 0)) {
                    role_bits[i] |= class_bit((args.role_class)(&text(&row, 1)));
                }
            }
            drop(rows);
            for sql in [guid_census_bidders_sql(chunk.len()), guid_census_winners_sql(chunk.len())] {
                let mut rows = reader.query(&sql, params.clone()).await?;
                while let Some(row) = rows.next().await? {
                    if let Some(&i) = at.get(&int(&row, 0)) {
                        role_bits[i] |= class_bit(GUID_ROLE_TENDERER);
                    }
                }
            }
        }
        for (i, (id, country, identifier, name)) in batch.into_iter().enumerate() {
            let mut tenders: Vec<i64> = notices[i]
                .iter()
                .filter_map(|n| walk.notices.get(n))
                .flat_map(|info| info.versions.iter().map(|(t, _)| *t))
                .collect();
            tenders.sort_unstable();
            tenders.dedup();
            let first_year = notices[i].iter().filter_map(|n| walk.notices.get(n).and_then(|info| info.year)).min();
            let bits = role_bits[i];
            let role = match GUID_ROLE_CLASSES.iter().enumerate().find(|(bit, _)| bits & (1 << bit) != 0) {
                Some((_, class)) => *class,
                None => GUID_ROLE_NONE,
            };
            walk.row_at.insert(id, walk.rows.len() as u32);
            walk.rows.push(Row {
                id,
                country,
                identifier,
                name,
                mentions: mentions[i],
                notices: std::mem::take(&mut notices[i]),
                tenders: tenders.len() as u64,
                first_year,
                role,
                stable: false,
                refused: 0,
                planned: Vec::new(),
                comparable: true,
                twin: false,
                twin_after: false,
                keep_spared: false,
            });
        }
        Ok(true)
    }

    /// Build 1b (A7): the roles each of `notices`' copies gives its sections by its own stored
    /// references — the planner's read ([`guid_twin_read_roles`]: [`guid_twin_role_refs_sql`] in IN
    /// chunks of [`IN_CHUNK`], folded by [`guid_twin_fold_roles`] with `role_ref`), exposed so a parity
    /// test can hold it against the gate's `guid_twin_section_roles` over the parsed copy. A notice with
    /// no role reference is absent. Read-only.
    pub async fn guid_twin_stored_roles(
        &self,
        notices: &[i64],
        role_ref: GuidTwinRoleRef,
    ) -> turso::Result<BTreeMap<i64, GuidTwinSectionRoles>> {
        let reader = self.reader().await?;
        let mut plan = GuidTwinPlan::default();
        let mut out: BTreeMap<i64, GuidTwinSectionRoles> = BTreeMap::new();
        for chunk in notices.chunks(IN_CHUNK) {
            out.extend(guid_twin_read_roles(&reader, chunk, role_ref, &mut plan).await?);
        }
        Ok(out)
    }
}

/// One ORG mention of a candidate notice.
struct MentionRow {
    section: String,
    org: i64,
    name: String,
    country: Option<String>,
    raw: Option<String>,
    scheme: Option<String>,
}

/// One notice copy's mentions as rule 530-T reads them, with the copy's notice-reference roles.
fn guid_twin_views<'a>(mentions: &'a [MentionRow], roles: &'a GuidTwinSectionRoles) -> Vec<TwinView<'a>> {
    mentions
        .iter()
        .map(|m| TwinView {
            section_id: &m.section,
            name: &m.name,
            country: m.country.as_deref(),
            raw_identifier: m.raw.as_deref(),
            scheme: m.scheme.as_deref(),
            roles: roles.get(&m.section).map_or(&[], Vec::as_slice),
        })
        .collect()
}

/// Whether rule 530-T finds a twin group (two or more distinct values under one key) in a notice copy,
/// roles not yet read: the copies whose role references the planner reads (A7). The group set does
/// not depend on the roles or on KEEP, only each group's outcome does.
fn guid_twin_has_group(args: &GuidTwinPlanArgs<'_>, keep: &GuidKeep, mentions: &[MentionRow]) -> bool {
    let none = GuidTwinSectionRoles::new();
    !(args.rule)(&guid_twin_views(mentions, &none), keep, args.extra_unsafe_names).groups.is_empty()
}

/// The role references of `notices` (at most [`IN_CHUNK`]: ONE statement on the `notice_ids` primary
/// key, [`guid_twin_role_refs_sql`]), folded per notice copy by [`guid_twin_fold_roles`] with
/// `role_ref` — the gate's fold and mapping over the stored copy. Tallies `role_ref_*` into `plan`.
async fn guid_twin_read_roles(
    reader: &crate::read::Reader,
    notices: &[i64],
    role_ref: GuidTwinRoleRef,
    plan: &mut GuidTwinPlan,
) -> turso::Result<HashMap<i64, GuidTwinSectionRoles>> {
    debug_assert!(notices.len() <= IN_CHUNK);
    let params: Vec<Value> = notices.iter().map(|&id| Value::Integer(id)).collect();
    let mut rows = reader.query(&guid_twin_role_refs_sql(notices.len()), params).await?;
    // (field_id, scheme, target, is_ref) per notice.
    let mut refs: HashMap<i64, Vec<(String, Option<String>, String, bool)>> = HashMap::new();
    while let Some(row) = rows.next().await? {
        plan.role_ref_rows_read += 1;
        refs.entry(int(&row, 0)).or_default().push((text(&row, 1), opt_text_of(&row, 2), text(&row, 3), int(&row, 4) != 0));
    }
    drop(rows);
    plan.role_ref_notices += notices.len() as u64;
    let mut out: HashMap<i64, GuidTwinSectionRoles> = HashMap::new();
    for (notice, list) in refs {
        let roles = guid_twin_fold_roles(
            list.iter().map(|(field, scheme, target, is_ref)| (field.as_str(), scheme.as_deref(), target.as_str(), *is_ref)),
            role_ref,
        );
        plan.role_refs += roles.values().map(|r| r.len() as u64).sum::<u64>();
        if !roles.is_empty() {
            out.insert(notice, roles);
        }
    }
    Ok(out)
}

/// Rule 530-T over one notice copy (with its notice-reference roles), folded into the walk: the guard
/// tallies, the refused groups and mentions (planned on GUID rows, or off them), what KEEP spares, and
/// the census twin before and after.
#[allow(clippy::too_many_arguments)]
fn guid_twin_notice(
    args: &GuidTwinPlanArgs<'_>,
    keep: &GuidKeep,
    empty_keep: &GuidKeep,
    walk: &mut Walk,
    plan: &mut GuidTwinPlan,
    notice: i64,
    mentions: &[MentionRow],
    section_roles: &GuidTwinSectionRoles,
) {
    let views = guid_twin_views(mentions, section_roles);
    let decision = (args.rule)(&views, keep, args.extra_unsafe_names);
    plan.candidate_mentions += decision.candidates as u64;
    plan.unnamed_candidates += decision.unnamed as u64;
    let by_section: HashMap<&str, &MentionRow> = mentions.iter().map(|m| (m.section.as_str(), m)).collect();
    let org_of = |section: &str| by_section.get(section).map_or(0, |m| m.org);
    for g in &decision.groups {
        plan.twin_groups += 1;
        // The group's notice-reference roles by the PLANNER's own read (the copy's section -> roles map
        // over the group's sections), never the rule's echo: the role tripwire and the clause tally read
        // these, so a rule that loses or mis-keys the roles shows (`bars.role_ref_disagree`).
        let ref_roles: Vec<String> = g
            .members
            .iter()
            .filter_map(|(s, _)| section_roles.get(s))
            .flatten()
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let sample = |outcome: &'static str| GuidTwinGroupSample {
            notice_id: notice,
            name_norm: g.name_norm.clone(),
            country: g.country.clone(),
            sections: g.members.iter().map(|(s, _)| s.clone()).collect(),
            org_ids: g.members.iter().map(|(s, _)| org_of(s)).collect(),
            values: g.members.iter().map(|(_, v)| v.clone()).collect(),
            refused: g.refused.clone(),
            outcome,
            roles: Vec::new(),
            ref_roles: ref_roles.clone(),
            rule_roles: if g.roles == ref_roles { Vec::new() } else { g.roles.clone() },
        };
        if g.roles != ref_roles {
            plan.bars.role_ref_disagree += 1;
            if plan.bars.role_ref_disagree_samples.len() < args.samples {
                let outcome = g.bind.map_or("refused", TwinBind::label);
                plan.bars.role_ref_disagree_samples.push(sample(outcome));
            }
        }
        if let Some(reason) = g.bind {
            let tally = plan.guard.entry(reason.label()).or_default();
            tally.groups += 1;
            tally.mentions += g.members.len() as u64;
            if reason == TwinBind::RoleMix {
                // "(rule only)": the rule bound role_mix, the planner's read of the copy shows no mix.
                let clause = guid_twin_role_mix(&ref_roles).unwrap_or("(rule only)");
                let by = plan.roles.role_mix_by_clause.entry(clause.to_owned()).or_default();
                by.groups += 1;
                by.mentions += g.members.len() as u64;
            }
            let list = plan.guard_samples.entry(reason.label()).or_default();
            if list.len() < args.samples {
                list.push(sample(reason.label()));
            }
            continue;
        }
        plan.refused_groups += 1;
        plan.keep_partner_mentions += (g.members.len() - g.refused.len()) as u64;
        let all_guid = g.members.iter().all(|(s, _)| walk.row_at.contains_key(&org_of(s)));
        let target = walk.target(&g.name_norm, g.country.as_deref());
        walk.targets[target as usize].office_two_words |= g.office_two_words;
        for section in &g.refused {
            plan.refused_mentions += 1;
            let org = org_of(section);
            let value = g.members.iter().find(|(s, _)| s == section).map(|(_, v)| v.clone()).unwrap_or_default();
            let mention = by_section.get(section.as_str());
            if walk.targets[target as usize].name.is_empty() {
                // A missing key's name in the report (a found target's row name replaces it in 4b).
                walk.targets[target as usize].name = mention.map(|m| m.name.clone()).unwrap_or_default();
            }
            // F3: a reviewed rehoming verdict put this mention where it sits — never planned, never counted
            // toward its row being whole or partial.
            if args.rehomed.is_some_and(|set| set.contains(&(notice, section.clone()))) {
                plan.verdict_kept += 1;
                if plan.verdict_kept_samples.len() < args.samples {
                    plan.verdict_kept_samples.push(GuidTwinOffGuidSample {
                        notice_id: notice,
                        section_id: section.clone(),
                        org_id: org,
                        kind: if walk.row_at.contains_key(&org) { "guid_row" } else { "off_guid" },
                        value: value.clone(),
                        target: None,
                    });
                }
                continue;
            }
            match walk.row_at.get(&org).copied() {
                Some(r) => {
                    let row = &mut walk.rows[r as usize];
                    row.refused += 1;
                    row.comparable &= all_guid;
                    row.planned.push(walk.planned.len() as u32);
                    walk.planned.push(Planned {
                        notice,
                        section: section.clone(),
                        row: r,
                        target,
                        city: None,
                        country: mention.and_then(|m| m.country.clone()),
                        name: mention.map(|m| m.name.clone()).unwrap_or_default(),
                    });
                }
                None => walk.off_guid.push(OffGuid { notice, section: section.clone(), org, value, target }),
            }
        }
        walk.groups.push(RefusedGroup {
            notice,
            name_norm: g.name_norm.clone(),
            country: g.country.clone(),
            members: g
                .members
                .iter()
                .map(|(s, v)| (s.clone(), org_of(s), v.clone(), g.refused.contains(s)))
                .collect(),
            ref_roles,
        });
    }
    // OQ1: a refused section the copy references as its Procedure-Buyer (the planner's read).
    if decision.refused.iter().any(|s| section_roles.get(s).is_some_and(|r| r.iter().any(|r| r == "Procedure-Buyer"))) {
        plan.refused_buyer_copies += 1;
    }
    // What KEEP spares: refused under an empty KEEP, bound under this one.
    if !keep.is_empty() {
        let bare = (args.rule)(&views, empty_keep, args.extra_unsafe_names);
        for section in bare.refused.difference(&decision.refused) {
            plan.keep_spared.mentions += 1;
            if let Some(&r) = walk.row_at.get(&org_of(section)) {
                walk.rows[r as usize].keep_spared = true;
            }
        }
    }
    // The census twin over GUID-bound mentions (no country in the key), now and after the plan (a
    // verdict-kept mention stays where it is, F3).
    let mut by_norm: HashMap<String, Vec<(u32, bool)>> = HashMap::new();
    for m in mentions {
        let Some(&r) = walk.row_at.get(&m.org) else { continue };
        let norm = org_name_norm(&m.name);
        if norm.is_empty() {
            continue;
        }
        let moves = decision.refused.contains(&m.section)
            && !args.rehomed.is_some_and(|set| set.contains(&(notice, m.section.clone())));
        by_norm.entry(norm).or_default().push((r, moves));
    }
    for members in by_norm.values() {
        let before: BTreeSet<u32> = members.iter().map(|(r, _)| *r).collect();
        if before.len() > 1 {
            for r in &before {
                walk.rows[*r as usize].twin = true;
            }
        }
        let after: BTreeSet<u32> = members.iter().filter(|(_, refused)| !refused).map(|(r, _)| *r).collect();
        if after.len() > 1 {
            for r in &after {
                walk.rows[*r as usize].twin_after = true;
            }
        }
    }
}

/// Whether a stored mention country can be read by A11's check: a 2-letter code (the fold stores the
/// canonical alpha-2), upper-cased; anything else (a legacy alpha-3 or a name) is left unchecked.
fn guid_twin_alpha2(country: &str) -> Option<String> {
    let c = country.trim();
    (c.len() == 2 && c.chars().all(|ch| ch.is_ascii_alphabetic())).then(|| c.to_ascii_uppercase())
}

/// The reads [`guid_twin_finish`] folds in: the off-GUID rows' identifiers and survivors, the whole
/// rows that are survivors (Build 2), and the refused groups' party roles.
struct GuidTwinReads<'a> {
    identified: &'a HashMap<i64, Option<String>>,
    survivors: &'a HashSet<i64>,
    whole_survivors: &'a HashSet<i64>,
    roles: &'a HashMap<(i64, String), BTreeSet<String>>,
    /// Build 2: the controls absent from the walk that an earlier wet run logged, `org -> [(target,
    /// whole)]` (`guid_twin_dissolve_log`, by `guid_twin_dissolve_log_org`).
    logged_controls: &'a HashMap<i64, Vec<(i64, bool)>>,
}

/// A KEEP file line (`country`, `value`, `org_id`, `mentions`, `tenders`, `reason`, tab-separated; an
/// empty cell is NULL): the form `keep_add_commit` lists and `guid-keep-530.tsv` holds.
pub fn guid_keep_line(c: &GuidKeepCandidate) -> String {
    let opt = |v: Option<u64>| v.map_or_else(String::new, |n| n.to_string());
    format!(
        "{}\t{}\t{}\t{}\t{}\t{}",
        c.country.as_deref().unwrap_or(""),
        c.value,
        c.org_id,
        opt(c.mentions),
        opt(c.tenders),
        c.reason
    )
}

/// The plan's tallies, screens, bars and samples, from the finished walk.
fn guid_twin_finish(
    args: &GuidTwinPlanArgs<'_>,
    plan: &mut GuidTwinPlan,
    walk: &mut Walk,
    reads: &GuidTwinReads<'_>,
    keep: &GuidKeep,
    stable_ids: &HashSet<i64>,
) {
    let (identified, survivors, roles) = (reads.identified, reads.survivors, reads.roles);
    let cap = args.samples;
    // The targets' tallies, the city screen and A11's country check, per planned mention.
    for p in &walk.planned {
        let t = &mut walk.targets[p.target as usize];
        t.planned_mentions += 1;
        t.planned_rows.insert(p.row);
        match &p.city {
            Some(city) => {
                plan.city.planned_mentions_with_city += 1;
                t.planned_cities.insert(city.clone());
                if !t.cities.is_empty() && !t.cities.contains(city) {
                    t.city_absent += 1;
                    plan.city.city_absent_mentions += 1;
                }
            }
            None => plan.city.planned_mentions_without_city += 1,
        }
        match p.country.as_deref().and_then(guid_twin_alpha2) {
            Some(code) => {
                let expected = register_jurisdiction(&code);
                let key_ok = t.country.as_deref() == Some(expected);
                let row_ok = t.id.is_none() || t.row_country.as_deref() == Some(expected);
                if !(key_ok && row_ok) {
                    plan.bars.target_country_mismatch += 1;
                }
            }
            None => plan.bars.target_country_unchecked += 1,
        }
    }
    let city_absent_rows: HashSet<u32> = walk
        .planned
        .iter()
        .filter(|p| {
            let t = &walk.targets[p.target as usize];
            p.city.as_ref().is_some_and(|c| !t.cities.is_empty() && !t.cities.contains(c))
        })
        .map(|p| p.row)
        .collect();
    plan.city.city_absent_rows = city_absent_rows.len() as u64;
    let target_sample = |t: &Target| {
        let mut notice_ids: Vec<i64> = Vec::new();
        for (n, _) in &t.newest {
            if !notice_ids.contains(n) && notice_ids.len() < 3 {
                notice_ids.push(*n);
            }
        }
        // A missing key's stable id: the smallest org id among its planned rows.
        let sample_id = t
            .id
            .unwrap_or_else(|| t.planned_rows.iter().map(|&r| walk.rows[r as usize].id).min().unwrap_or(i64::MAX));
        GuidTwinTargetSample {
            target_id: t.id,
            sample_id,
            name: t.name.clone(),
            name_norm: t.name_norm.clone(),
            country: t.country.clone(),
            mentions: t.mentions,
            notice_ids,
            cities: t.cities.iter().take(10).cloned().collect(),
            city_count: t.cities.len(),
            planned_mentions: t.planned_mentions,
            planned_rows: t.planned_rows.len() as u64,
            city_absent: t.city_absent,
            multi_city: t.cities.len() >= 2,
            planned_cities: t.planned_cities.iter().take(10).cloned().collect(),
            planned_city_count: t.planned_cities.len(),
            multi_city_planned: t.planned_cities.len() >= 2,
            office_two_words: t.office_two_words,
        }
    };
    // R2's lists read every planned key, found or missing: a missing key mints one row that takes
    // every planned mention of its (name_norm, country), the same homonym risk as a found target.
    let mut flagged: Vec<GuidTwinTargetSample> = Vec::new();
    let mut planned_targets: Vec<GuidTwinTargetSample> = Vec::new();
    let mut office: Vec<GuidTwinTargetSample> = Vec::new();
    let mut missing: Vec<GuidTwinTargetSample> = Vec::new();
    let mut largest: Option<GuidTwinTargetSample> = None;
    let mut multi_city_planned_rows: HashSet<u32> = HashSet::new();
    for t in &walk.targets {
        plan.targets += 1;
        let s = target_sample(t);
        if t.id.is_some() {
            plan.targets_found += 1;
            if !t.cities.is_empty() {
                plan.city.targets_with_cities += 1;
            }
            if t.city_absent > 0 {
                plan.city.city_absent_targets += 1;
            }
            if t.cities.len() >= 2 {
                plan.city.multi_city_targets += 1;
            }
        } else if t.planned_mentions > 0 {
            plan.missing_targets.rows += t.planned_rows.len() as u64;
            plan.missing_targets.mentions += t.planned_mentions;
            missing.push(s.clone());
        }
        if t.planned_mentions == 0 {
            continue;
        }
        let flag_c = t.planned_cities.len() >= 2;
        if flag_c {
            plan.city.planned_multi_city_keys += 1;
            plan.city.planned_multi_city_mentions += t.planned_mentions;
            multi_city_planned_rows.extend(t.planned_rows.iter().copied());
        }
        if t.city_absent > 0 || t.cities.len() >= 2 || flag_c {
            flagged.push(s.clone());
        }
        if t.office_two_words {
            office.push(s.clone());
        }
        if largest.as_ref().is_none_or(|l| {
            (t.planned_mentions, std::cmp::Reverse(s.sample_id)) > (l.planned_mentions, std::cmp::Reverse(l.sample_id))
        }) {
            largest = Some(s.clone());
        }
        planned_targets.push(s);
    }
    plan.city.planned_multi_city_rows = multi_city_planned_rows.len() as u64;
    plan.largest_target = largest;
    missing.sort_by(|a, b| {
        b.planned_mentions.cmp(&a.planned_mentions).then_with(|| (&a.name_norm, &a.country).cmp(&(&b.name_norm, &b.country)))
    });
    plan.missing_target_samples = missing.into_iter().take(cap).collect();
    let by_planned = |s: &GuidTwinTargetSample| (s.planned_mentions, s.sample_id);
    flagged.sort_by_key(|s| (std::cmp::Reverse(s.planned_mentions), s.sample_id));
    plan.city.flagged_count = flagged.len() as u64;
    plan.r2_city_flagged = r2_pick(&flagged, by_planned, |s| s.sample_id);
    plan.city.flagged = flagged.into_iter().take(GUID_TWIN_LIST_CAP).collect();
    plan.r2_office_two_words_count = office.len() as u64;
    plan.r2_office_two_words = r2_pick(&office, by_planned, |s| s.sample_id);
    plan.r2_top_targets = r2_top_targets(planned_targets);

    // Off-GUID refused mentions by bound kind.
    for o in &walk.off_guid {
        let kind = if survivors.contains(&o.org) {
            "merge_survivor"
        } else {
            match identified.get(&o.org) {
                Some(None) => "identifier_null",
                // A v4 row the walk did not reach: minted above the walk's max id.
                Some(Some(identifier)) if (args.is_guid)(identifier) => "guid_above_max_id",
                Some(Some(_)) => "other_identified",
                None => "absent",
            }
        };
        *plan.off_guid.by_kind.entry(kind).or_default() += 1;
        let target = walk.targets[o.target as usize].id;
        if target == Some(o.org) {
            plan.off_guid.already_on_target += 1;
        }
        if plan.off_guid.samples.len() < cap {
            plan.off_guid.samples.push(GuidTwinOffGuidSample {
                notice_id: o.notice,
                section_id: o.section.clone(),
                org_id: o.org,
                kind,
                value: o.value.clone(),
                target,
            });
        }
    }

    // Role composition of the refused groups. A7's "a tenderer or winner role with any role that is
    // not a winner" is read by CLASS: a bid party or a result winner is the census's `tenderer_winner`
    // (`GUID_ROLE_TENDERER`), so a winner beside a bid tenderer is coherent, and the mix is a role of
    // that class beside one outside it.
    let class_of = |role: &str| -> &'static str {
        if role.starts_with("bid-") || role == "result-winner" { GUID_ROLE_TENDERER } else { (args.role_class)(role) }
    };
    let mut by_set: HashMap<String, u64> = HashMap::new();
    let mut incoherent: Vec<GuidTwinGroupSample> = Vec::new();
    for g in &walk.groups {
        plan.roles.refused_groups += 1;
        let mut set: BTreeSet<String> = BTreeSet::new();
        for (section, ..) in &g.members {
            if let Some(r) = roles.get(&(g.notice, section.clone())) {
                set.extend(r.iter().cloned());
            }
        }
        let key = if set.is_empty() { "(no role)".to_owned() } else { set.iter().cloned().collect::<Vec<_>>().join("+") };
        *by_set.entry(key).or_default() += 1;
        let buyer = set.iter().any(|r| class_of(r) == GUID_ROLE_BUYER);
        let review_org = set.iter().any(|r| r == "Lot-ReviewOrg" || r == "review-body");
        let tenderer = set.iter().any(|r| class_of(r) == GUID_ROLE_TENDERER);
        let other = set.iter().any(|r| class_of(r) != GUID_ROLE_TENDERER);
        let a = buyer && review_org;
        let b = tenderer && other;
        plan.roles.buyer_with_review_org += u64::from(a);
        plan.roles.tenderer_mixed += u64::from(b);
        // Build 1b's self-check: a refused group whose NOTICE-REFERENCE roles (the planner's own read,
        // not the rule's echo) are incoherent is one the role guard should have bound.
        let ref_mixed = guid_twin_role_mix(&g.ref_roles).is_some();
        if a || b || ref_mixed {
            let sample = GuidTwinGroupSample {
                notice_id: g.notice,
                name_norm: g.name_norm.clone(),
                country: g.country.clone(),
                sections: g.members.iter().map(|m| m.0.clone()).collect(),
                org_ids: g.members.iter().map(|m| m.1).collect(),
                values: g.members.iter().map(|m| m.2.clone()).collect(),
                refused: g.members.iter().filter(|m| m.3).map(|m| m.0.clone()).collect(),
                outcome: "refused",
                roles: set.into_iter().collect(),
                ref_roles: g.ref_roles.clone(),
                rule_roles: Vec::new(),
            };
            if ref_mixed {
                plan.bars.tripwire_role_mix += 1;
                if plan.bars.role_mix_refused.len() < GUID_TWIN_LIST_CAP {
                    plan.bars.role_mix_refused.push(sample.clone());
                }
            }
            if a || b {
                incoherent.push(sample);
            }
        }
    }
    let mut sets: Vec<(String, u64)> = by_set.into_iter().collect();
    sets.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    for (i, (key, n)) in sets.into_iter().enumerate() {
        if i < GUID_TWIN_ROLE_SETS_MAX {
            plan.roles.by_role_set.insert(key, n);
        } else {
            *plan.roles.by_role_set.entry("(rest)".to_owned()).or_default() += n;
        }
    }
    plan.roles.incoherent_count = incoherent.len() as u64;
    plan.r2_incoherent = r2_pick(&incoherent, |g| (g.refused.len() as u64, g.notice_id), |g| g.notice_id);
    plan.roles.incoherent = incoherent.into_iter().take(GUID_TWIN_LIST_CAP).collect();

    // The rows: whole, partial, tripwires, breakdowns, post-state, samples.
    let row_sample = |row: &Row| {
        let whole = row.refused > 0 && row.refused >= row.mentions;
        let mut targets: Vec<Option<i64>> =
            row.planned.iter().map(|&p| walk.targets[walk.planned[p as usize].target as usize].id).collect();
        targets.sort_unstable();
        targets.dedup();
        let mut cities: Vec<String> = row.planned.iter().filter_map(|&p| walk.planned[p as usize].city.clone()).collect();
        cities.sort_unstable();
        cities.dedup();
        GuidTwinRowSample {
            org_id: row.id,
            identifier: row.identifier.clone(),
            name: row.name.clone(),
            country: row.country.clone(),
            role: row.role,
            mentions: row.mentions,
            refused: row.refused,
            tenders: row.tenders,
            notice_ids: row.notices.iter().take(3).copied().collect(),
            planned: if whole {
                "whole"
            } else if row.refused > 0 {
                "partial"
            } else {
                "no"
            },
            targets,
            first_year: row.first_year,
            in_keep: keep.contains(&row.identifier),
            stable: row.stable,
            twin: row.twin,
            twin_after: row.twin_after,
            cities,
        }
    };
    let mut r2_rows: Vec<(&'static str, i64)> = Vec::new();
    let mut r2_at: Vec<u32> = Vec::new();
    let mut exhibits: BTreeMap<String, GuidTwinExhibit> = BTreeMap::new();
    // F4: per VALUE over all of its GUID rows — the Tenders (`walk.value_tenders`), whether any of its
    // rows is a census twin or stable, and its rows by id (the lowest names its KEEP line).
    let value_twin: HashSet<String> =
        walk.rows.iter().filter(|r| r.twin).map(|r| GuidKeep::key(&r.identifier)).collect();
    let value_stable: HashSet<String> =
        walk.rows.iter().filter(|r| r.stable).map(|r| GuidKeep::key(&r.identifier)).collect();
    let mut multi_rows: Vec<(&String, &Vec<u32>)> = walk.value_rows.iter().filter(|(_, rows)| rows.len() >= 2).collect();
    multi_rows.sort();
    plan.multi_row_value_count = multi_rows.len() as u64;
    for (value, rows) in multi_rows.into_iter().take(GUID_TWIN_LIST_CAP) {
        let mut rows: Vec<&Row> = rows.iter().map(|&r| &walk.rows[r as usize]).collect();
        rows.sort_by_key(|r| r.id);
        plan.multi_row_values.push(GuidTwinMultiRowValue {
            value: value.clone(),
            rows: rows.iter().map(|r| r.id).collect(),
            row_tenders: rows.iter().map(|r| r.tenders).collect(),
            tenders: walk.value_tenders.get(value).map_or(0, |t| t.len() as u64),
        });
    }
    let mut self_checked: HashSet<String> = HashSet::new();
    for (i, row) in walk.rows.iter().enumerate() {
        // Whole when every mention is refused (`>=`: a mention recorded between the two reads of the
        // walk cannot make a whole row read partial the wrong way round).
        let whole = row.refused > 0 && row.refused >= row.mentions;
        let partial = row.refused > 0 && !whole;
        let norm = org_name_norm(&row.name);
        for (country, name) in args.exhibits {
            if row.country.as_deref() == Some(*country) && norm == *name {
                let e = exhibits.entry(format!("{country} {name}")).or_default();
                e.guid_rows += 1;
                if whole {
                    e.planned += 1;
                } else if partial {
                    e.partial += 1;
                    e.partial_ids.push(row.id);
                } else {
                    e.kept += 1;
                    e.kept_ids.push(row.id);
                }
            }
        }
        if row.keep_spared {
            plan.keep_spared.rows += 1;
        }
        // A9's post-state (census twin over GUID-bound mentions).
        if row.twin {
            plan.post_state.twin_rows += 1;
            plan.post_state.twin_stable_rows += u64::from(row.stable);
            plan.post_state.twin_multi_tender_rows += u64::from(row.tenders >= 2);
        }
        if row.twin_after && !whole {
            plan.post_state.twin_rows_after += 1;
            plan.post_state.twin_stable_rows_after += u64::from(row.stable);
            plan.post_state.twin_multi_tender_rows_after += u64::from(row.tenders >= 2);
        }
        // A3 iii's self-check: against the COMPILED file since Build 2 (a tautology for the walk's KEEP).
        // F4: per VALUE — (any of its rows a census twin ∧ its rows span >= 2 Tenders) ∨ any of its rows
        // stable — listed once, on its lowest-id row (the walk reads rows in id order).
        let value = GuidKeep::key(&row.identifier);
        let value_tenders = walk.value_tender_count(row);
        let stable_v = value_stable.contains(&value);
        if ((value_twin.contains(&value) && value_tenders >= 2) || stable_v)
            && !keep.contains(&row.identifier)
            && self_checked.insert(value.clone())
        {
            let first = walk.value_rows.get(&value).and_then(|rows| rows.iter().map(|&r| &walk.rows[r as usize]).min_by_key(|r| r.id));
            let first = first.unwrap_or(row);
            plan.bars.keep_self_check_missing.push(value.clone());
            let reason = match (stable_v, value_tenders >= 2) {
                (true, true) => "stable+multi_tender",
                (true, false) => "stable",
                _ => "multi_tender",
            };
            plan.keep_add_commit.push(guid_keep_line(&GuidKeepCandidate {
                country: first.country.clone(),
                value: value.clone(),
                org_id: first.id,
                mentions: Some(first.mentions),
                tenders: Some(value_tenders),
                reason: reason.to_owned(),
            }));
        }
        if row.refused == 0 {
            continue;
        }
        // The tripwires, over every planned row. The stable one reads R1's fixed threshold, never the
        // run's knob (`row.stable`), so a raised knob cannot hide a 100-mention row from it. F4: the
        // several-Tenders one reads the row's VALUE over all of its GUID rows (a value split over two
        // single-Tender rows in two Tenders already links two Tenders); the no-Tender one stays per row.
        let multi = value_tenders >= 2;
        let none = row.tenders == 0;
        let stable = stable_ids.contains(&row.id) || row.mentions >= GUID_TWIN_R1_STABLE_MENTIONS;
        let kept = keep.contains(&row.identifier);
        plan.bars.tripwire_multi_tender += u64::from(multi);
        plan.bars.tripwire_no_tender += u64::from(none);
        plan.bars.tripwire_stable += u64::from(stable);
        plan.bars.tripwire_keep += u64::from(kept);
        if (multi || none || stable || kept) && plan.bars.tripwire_rows.len() < GUID_TWIN_LIST_CAP {
            plan.bars.tripwire_rows.push(row.id);
        }
        r2_rows.push((row.role, row.id));
        r2_at.push(i as u32);
        // R1's census bar counts every planned row, whole or partial: a partial row is a census twin too.
        if row.id > GUID_TWIN_CENSUS_MAX_ID {
            plan.bars.planned_rows_above_census += 1;
        } else if row.comparable {
            plan.bars.planned_rows_comparable += 1;
            plan.bars.planned_rows_comparable_whole += u64::from(whole);
        } else {
            plan.bars.planned_rows_off_guid_partner += 1;
        }
        if whole {
            // Build 2's dissolve tripwires: a merge survivor, a row a review table names.
            if reads.whole_survivors.contains(&row.id) {
                plan.bars.tripwire_merge_survivor += 1;
                if plan.bars.merge_survivor_rows.len() < GUID_TWIN_LIST_CAP {
                    plan.bars.merge_survivor_rows.push(row.id);
                }
            }
            if args.under_review.is_some_and(|set| set.contains(&row.id)) {
                plan.bars.tripwire_under_review += 1;
                if plan.bars.under_review_rows.len() < GUID_TWIN_LIST_CAP {
                    plan.bars.under_review_rows.push(row.id);
                }
            }
            plan.planned.rows += 1;
            plan.planned.mentions += row.refused;
            plan.planned_row_ids.push(row.id);
            for (map, key) in [
                (&mut plan.planned_by_country, row.country.clone().unwrap_or_else(|| "none".to_owned())),
                (&mut plan.planned_by_role, row.role.to_owned()),
                (&mut plan.planned_by_first_year, row.first_year.map_or_else(|| "none".to_owned(), |y| y.to_string())),
            ] {
                let c = map.entry(key).or_default();
                c.rows += 1;
                c.mentions += row.refused;
            }
            if plan.planned_samples.len() < cap {
                plan.planned_samples.push(row_sample(row));
            }
        } else {
            plan.planned_partial.rows += 1;
            plan.planned_partial.mentions += row.refused;
            plan.partial_row_ids.push(row.id);
            plan.partial.rows += 1;
            plan.partial.refused_mentions += row.refused;
            plan.partial.kept_mentions += row.mentions.saturating_sub(row.refused);
            let refused_notices: BTreeSet<i64> = row.planned.iter().map(|&p| walk.planned[p as usize].notice).collect();
            let sources = |notices: &mut dyn Iterator<Item = &i64>| -> String {
                let set: BTreeSet<&str> = notices
                    .filter_map(|n| walk.notices.get(n))
                    .map(|info| walk.sources[info.source as usize].as_str())
                    .collect();
                if set.is_empty() { "(none)".to_owned() } else { set.into_iter().collect::<Vec<_>>().join("+") }
            };
            let refused_src = sources(&mut refused_notices.iter());
            let kept_src = sources(&mut row.notices.iter().filter(|n| !refused_notices.contains(n)));
            *plan.partial.by_sources.entry(format!("refused {refused_src} / kept {kept_src}")).or_default() += 1;
            if plan.partial.samples.len() < cap {
                plan.partial.samples.push(row_sample(row));
            }
        }
    }
    plan.exhibits = exhibits;
    // Build 2: the walk's KEEP candidates the compiled file lacks and the self-check did not list
    // (merged values, multi-Tender rows that are no census twin): advisory.
    let missing: HashSet<&str> = plan.bars.keep_self_check_missing.iter().map(String::as_str).collect();
    for c in &plan.keep_candidates {
        if !keep.contains(&c.value) && !missing.contains(c.value.as_str()) {
            plan.keep_candidates_not_in_file += 1;
            if plan.keep_candidates_not_in_file_lines.len() < GUID_TWIN_LIST_CAP {
                plan.keep_candidates_not_in_file_lines.push(guid_keep_line(c));
            }
        }
    }
    // Build 2: every planned mention for the wet run, uncapped; a key's mint name is the published name
    // of its lowest planned (notice, section).
    let mut mint_at: HashMap<u32, (i64, &str, &str)> = HashMap::new();
    for p in &walk.planned {
        let entry = mint_at.entry(p.target).or_insert((p.notice, p.section.as_str(), p.name.as_str()));
        if (p.notice, p.section.as_str()) < (entry.0, entry.1) {
            *entry = (p.notice, p.section.as_str(), p.name.as_str());
        }
    }
    for p in &walk.planned {
        let row = &walk.rows[p.row as usize];
        let t = &walk.targets[p.target as usize];
        plan.wet_mentions.push(GuidTwinWetMention {
            notice_id: p.notice,
            section_id: p.section.clone(),
            org_id: row.id,
            org_value: row.identifier.clone(),
            org_country: row.country.clone(),
            org_name: row.name.clone(),
            key_name_norm: t.name_norm.clone(),
            key_country: t.country.clone(),
            target: t.id,
            mint_name: mint_at.get(&p.target).map(|m| m.2.to_owned()).unwrap_or_default(),
            whole: row.refused >= row.mentions,
            target_mentions: if t.id.is_some() { t.mentions } else { 0 },
            row_kept_mentions: row.mentions.saturating_sub(row.refused),
        });
    }
    for p in &walk.planned {
        if plan.planned_mention_keys.len() >= GUID_TWIN_LIST_CAP {
            plan.planned_mention_keys_truncated = true;
            break;
        }
        let row = &walk.rows[p.row as usize];
        plan.planned_mention_keys.push(GuidTwinMentionKey {
            notice_id: p.notice,
            section_id: p.section.clone(),
            org_id: row.id,
            target: walk.targets[p.target as usize].id,
            whole: row.refused >= row.mentions,
        });
    }
    plan.bars.census_bar = GUID_TWIN_CENSUS_BAR;
    plan.bars.within_census_bar = plan.bars.planned_rows_comparable <= GUID_TWIN_CENSUS_BAR;
    // R2's random stratum, at the re-read's salt, the rows R2 read at 530002 skipped.
    let (picked, skipped) = r2_random_pick(&r2_rows);
    plan.r2_random_excluded = skipped;
    for i in picked {
        plan.r2_random.push(row_sample(&walk.rows[r2_at[i] as usize]));
    }
    // The controls. A control absent from the walk is read against the dissolve log: a planned one a
    // wet run already dissolved whole onto its target is met (else a fresh dry plan after a partial
    // wet run could never pass R1), a not-planned one a wet run dissolved is a miss, and one with no
    // log row is as before (a planned row that vanished another way still fails R1).
    let control = |org: i64, expect: &'static str, expected_target: Option<i64>| {
        let row = walk.row_at.get(&org).map(|&r| row_sample(&walk.rows[r as usize]));
        let logged = reads.logged_controls.get(&org).filter(|rows| !rows.is_empty());
        let dissolved_by_wet = row.is_none() && logged.is_some();
        let ok = match (expect, &row) {
            ("not_planned", None) => logged.is_none(),
            ("not_planned", Some(r)) => r.planned == "no",
            (_, Some(r)) => {
                r.planned == "whole"
                    && match expected_target {
                        Some(t) => r.targets == vec![Some(t)],
                        None => r.targets.iter().all(Option::is_some),
                    }
            }
            (_, None) => logged.is_some_and(|rows| {
                rows.iter().all(|&(target, whole)| whole && expected_target.is_none_or(|t| t == target))
            }),
        };
        GuidTwinControl { org_id: org, expect, expected_target, found: row.is_some(), row, dissolved_by_wet, ok }
    };
    for &org in args.not_planned {
        let c = control(org, "not_planned", None);
        if !c.ok {
            plan.bars.not_planned_failed.push(org);
        }
        plan.controls.push(c);
    }
    for &(org, target) in args.planned {
        let c = control(org, "planned", Some(target));
        if !c.ok {
            plan.bars.planned_failed.push(org);
        }
        plan.controls.push(c);
    }
    for &(org, target) in args.expected {
        plan.controls.push(control(org, "expected", target));
    }
    plan.bars.stable_mentions_is_decision = args.stable_mentions == GUID_TWIN_R1_STABLE_MENTIONS;
    plan.bars.under_review_checked = args.under_review.is_some();
    // The decision's R1 bars (`target_country_unchecked` is reported only; `tripwire_no_tender` is no R1
    // bar, but the wet run's re-plan refuses on it: the Dissolve plan's exactly-1-Tender tripwire).
    let b = &plan.bars;
    let pass = b.within_census_bar
        && b.tripwire_multi_tender == 0
        && b.tripwire_stable == 0
        && b.tripwire_keep == 0
        && b.stable_mentions_is_decision
        && b.not_planned_failed.is_empty()
        && b.planned_failed.is_empty()
        && b.target_country_mismatch == 0
        && b.keep_self_check_missing.is_empty()
        && b.tripwire_role_mix == 0
        && b.role_ref_disagree == 0
        && b.tripwire_merge_survivor == 0
        && b.tripwire_under_review == 0;
    plan.bars.pass = pass;
    for reason in TwinBind::ALL {
        plan.guard.entry(reason.label()).or_default();
    }
}

// ---- Build 2: the wet run's windows, the window apply, the manifest, and A1's recurrence arm ----

/// Build 2: the planned mentions one wet window takes, at most — unless one target-key component is
/// larger (its rows and keys are applied together: 7615 Vergabekammer Rheinland's +1,346 mentions are
/// one window, so 542's re-derivation reads that row once, not once per org-id window).
pub const GUID_TWIN_WET_WINDOW_MENTIONS: usize = 2_000;

/// Review round 3 (F1): the re-read budget of one wet window — the existing mentions 542's
/// re-derivation re-reads under the lock ([`guid_twin_rebound_estimate`]). Planned mentions alone
/// misjudge the cost: plan 2660's `vergabekammer …` run fits under the 2,000-mention cap with about 330k
/// re-read mentions (target 622 alone holds 111k). A component over the budget sits alone in its window
/// (622 is the floor).
pub const GUID_TWIN_WET_WINDOW_REREAD: u64 = 120_000;

/// Build 2: one wet window — the target keys `(org_name_norm, register country)` it owns, its whole
/// and partial rows (sorted), and its planned mentions (by notice, section). Keys never span two
/// windows, and neither do rows: a row whose mentions go to two keys ties them into one component.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct GuidTwinWindow {
    pub keys: Vec<(String, Option<String>)>,
    pub whole: Vec<i64>,
    pub partial: Vec<i64>,
    pub mentions: Vec<GuidTwinWetMention>,
    /// Review round 3 (F1): the mentions 542's re-derivation is estimated to re-read inside this window
    /// ([`guid_twin_rebound_estimate`]); the applied window reports what it read
    /// (`GuidTwinWindowDone::rebound_target_mentions`).
    pub rebound_estimate: u64,
}

/// Review round 3 (F1): what 542's re-derivation re-reads when `mentions` are applied — every mention of
/// every re-bound row once the moves are done: each distinct target key's existing mentions
/// ([`GuidTwinWetMention::target_mentions`]) plus the planned mentions it receives, and each distinct
/// partial row's kept mentions ([`GuidTwinWetMention::row_kept_mentions`]). Keys and rows never span two
/// windows, so a window's estimate is the sum of its components'.
pub fn guid_twin_rebound_estimate(mentions: &[GuidTwinWetMention]) -> u64 {
    let mut keys: BTreeMap<(&str, Option<&str>), (u64, u64)> = BTreeMap::new();
    let mut partial: BTreeMap<i64, u64> = BTreeMap::new();
    for m in mentions {
        let k = keys.entry((m.key_name_norm.as_str(), m.key_country.as_deref())).or_default();
        k.0 = k.0.max(m.target_mentions);
        k.1 += 1;
        if !m.whole {
            partial.insert(m.org_id, m.row_kept_mentions);
        }
    }
    keys.values().map(|(existing, planned)| existing + planned).sum::<u64>() + partial.values().sum::<u64>()
}

/// Build 2 (decision "Windows go by target key"): the wet run's windows, PURE. A union-find over the
/// target keys and the GUID rows (a mention joins its key and its row) gives atomic components; they
/// are ordered by their smallest key and packed greedily up to `max` planned mentions and (review round
/// 3, F1) [`GUID_TWIN_WET_WINDOW_REREAD`] estimated re-read mentions, a component over either alone in
/// its window. Deterministic for one plan.
pub fn guid_twin_windows(wet: &[GuidTwinWetMention], max: usize) -> Vec<GuidTwinWindow> {
    guid_twin_windows_within(wet, max, GUID_TWIN_WET_WINDOW_REREAD)
}

/// [`guid_twin_windows`] under an explicit re-read budget (`reread`, clamped to at least 1): before a
/// component joins the current window, the window closes when the component would take it past `max`
/// planned mentions or past `reread` estimated re-read mentions ([`guid_twin_rebound_estimate`]).
pub fn guid_twin_windows_within(wet: &[GuidTwinWetMention], max: usize, reread: u64) -> Vec<GuidTwinWindow> {
    fn find(parent: &mut [usize], mut x: usize) -> usize {
        while parent[x] != x {
            parent[x] = parent[parent[x]];
            x = parent[x];
        }
        x
    }
    let mut parent: Vec<usize> = Vec::new();
    let mut key_at: BTreeMap<(String, Option<String>), usize> = BTreeMap::new();
    let mut row_at: BTreeMap<i64, usize> = BTreeMap::new();
    let mut node_of: Vec<usize> = Vec::with_capacity(wet.len());
    for m in wet {
        let key = (m.key_name_norm.clone(), m.key_country.clone());
        let k = *key_at.entry(key).or_insert_with(|| {
            parent.push(parent.len());
            parent.len() - 1
        });
        let r = *row_at.entry(m.org_id).or_insert_with(|| {
            parent.push(parent.len());
            parent.len() - 1
        });
        let (a, b) = (find(&mut parent, k), find(&mut parent, r));
        if a != b {
            parent[a.max(b)] = a.min(b);
        }
        node_of.push(k);
    }
    // Components in the order of their smallest key (`key_at` iterates keys in order).
    let mut order: Vec<usize> = Vec::new();
    let mut comp_keys: HashMap<usize, Vec<(String, Option<String>)>> = HashMap::new();
    for (key, &k) in &key_at {
        let root = find(&mut parent, k);
        if !comp_keys.contains_key(&root) {
            order.push(root);
        }
        comp_keys.entry(root).or_default().push(key.clone());
    }
    let mut comp_mentions: HashMap<usize, Vec<GuidTwinWetMention>> = HashMap::new();
    for (m, &k) in wet.iter().zip(&node_of) {
        let root = find(&mut parent, k);
        comp_mentions.entry(root).or_default().push(m.clone());
    }
    let mut out: Vec<GuidTwinWindow> = Vec::new();
    let mut current = GuidTwinWindow::default();
    let close = |w: &mut GuidTwinWindow, out: &mut Vec<GuidTwinWindow>| {
        if w.mentions.is_empty() {
            return;
        }
        let mut w = std::mem::take(w);
        w.keys.sort();
        w.mentions.sort_by(|a, b| (a.notice_id, &a.section_id).cmp(&(b.notice_id, &b.section_id)));
        let whole: BTreeSet<i64> = w.mentions.iter().filter(|m| m.whole).map(|m| m.org_id).collect();
        let partial: BTreeSet<i64> = w.mentions.iter().filter(|m| !m.whole).map(|m| m.org_id).collect();
        w.whole = whole.into_iter().collect();
        w.partial = partial.into_iter().collect();
        w.rebound_estimate = guid_twin_rebound_estimate(&w.mentions);
        out.push(w);
    };
    let (max, reread) = (max.max(1), reread.max(1));
    let mut current_reread = 0u64;
    for root in order {
        let mentions = comp_mentions.remove(&root).unwrap_or_default();
        let cost = guid_twin_rebound_estimate(&mentions);
        if !current.mentions.is_empty()
            && (current.mentions.len() + mentions.len() > max || current_reread.saturating_add(cost) > reread)
        {
            close(&mut current, &mut out);
            current_reread = 0;
        }
        current.keys.extend(comp_keys.remove(&root).unwrap_or_default());
        current.mentions.extend(mentions);
        current_reread = current_reread.saturating_add(cost);
    }
    close(&mut current, &mut out);
    out
}

/// One row the window minted for a missing key (decision "Missing targets: MINT").
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct GuidTwinMinted {
    pub target_id: i64,
    pub country: String,
    pub name_norm: String,
    pub name: String,
    /// The window's mentions that landed on it.
    pub mentions: u64,
}

/// [`Db::dissolve_guid_twin_window`]'s inputs.
pub struct GuidTwinWindowArgs<'a> {
    pub window: &'a GuidTwinWindow,
    /// The wet job's id (the log's `job_id`).
    pub job_id: i64,
    pub window_no: u64,
    /// A1's boundary the wet run was given, logged on every row (`guid_twin_dissolve_log`): the value the
    /// recurrence arm, the restore and every later wet run are anchored to ([`Db::guid_twin_recorded_boundary`]).
    pub gate_since_notice_id: Option<i64>,
    /// 542's name-variant re-derivation's field list and language normaliser (ingest's).
    pub fields: &'a [&'a str],
    pub normalize: fn(Option<&str>) -> Option<String>,
}

/// What one committed window did.
#[derive(Debug, Default, Clone, Serialize)]
pub struct GuidTwinWindowDone {
    pub window_no: u64,
    pub keys: u64,
    pub whole: u64,
    pub partial: u64,
    pub mentions_moved: u64,
    /// 440's counts (mentions, parties, bid parties, winners, tier 5, reused, fresh, variants…).
    pub counts: crate::OrgDissolve,
    pub minted: Vec<GuidTwinMinted>,
    /// Pre-existing identifier-NULL rows that took mentions (announced `organization changed`).
    pub reused_targets: Vec<i64>,
    /// Tenders stamped stale, their notices re-queued: every Tender a moved row touched (issue 579), tier 5's
    /// (whose ambiguous winner rows were deleted) among them.
    pub refold_tenders: Vec<i64>,
    /// The window's cost on the single writer: the wait for the writer connection, the time it held it
    /// (`BEGIN IMMEDIATE` to COMMIT: every verdict POST waits that long), and the mentions 542's
    /// re-derivation re-read under the lock (every mention of every re-bound row, the cost driver).
    pub wait_ms: u64,
    pub hold_ms: u64,
    pub rebound_target_mentions: u64,
    /// Review round 3 (F1): the window's estimate of that re-read ([`GuidTwinWindow::rebound_estimate`]),
    /// beside the measured `rebound_target_mentions`.
    pub rebound_estimate: u64,
}

/// One row of `guid_twin_dissolve_log` (the restore manifest).
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct GuidTwinLogRow {
    /// The wet job that moved the mention (a campaign's manifest spans its staged and restarted jobs).
    pub job_id: i64,
    pub window_no: i64,
    pub org_id: i64,
    pub org_country: Option<String>,
    pub org_kind: Option<String>,
    pub org_value: String,
    pub org_name: String,
    pub whole: bool,
    pub notice_id: i64,
    pub section_id: String,
    pub target_id: i64,
    pub target_minted: bool,
}

pub(crate) const GUID_TWIN_LOG_INSERT_SQL: &str = "INSERT INTO guid_twin_dissolve_log (job_id, at, window_no, \
     gate_since_notice_id, org_id, org_country, org_kind, org_value, org_name, whole, notice_id, section_id, target_id, \
     target_minted) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";
/// A1's boundary as the first wet window recorded it (the log is at most one row per moved mention).
pub(crate) const GUID_TWIN_LOG_BOUNDARY_SQL: &str = "SELECT gate_since_notice_id FROM guid_twin_dissolve_log \
     WHERE gate_since_notice_id IS NOT NULL ORDER BY rowid LIMIT 1";
/// One job's manifest rows, through `guid_twin_dissolve_log_job`.
pub(crate) const GUID_TWIN_LOG_JOB_SQL: &str = "SELECT job_id, window_no, org_id, org_country, org_kind, org_value, \
     org_name, whole, notice_id, section_id, target_id, target_minted FROM guid_twin_dissolve_log WHERE job_id = ?";
/// Every manifest row logged after a high-water rowid (the rows the wet runs on one dry plan committed:
/// a staged run, a restarted job): a rowid range, at most one row per moved mention.
pub(crate) const GUID_TWIN_LOG_AFTER_SQL: &str = "SELECT job_id, window_no, org_id, org_country, org_kind, org_value, \
     org_name, whole, notice_id, section_id, target_id, target_minted FROM guid_twin_dissolve_log WHERE rowid > ?";
/// Every manifest row, with its rowid (A1's input: the targets, and each logged mention's landing row).
pub(crate) const GUID_TWIN_LOG_ALL_SQL: &str = "SELECT rowid, notice_id, section_id, target_id, org_id, org_country, \
     org_name, org_value FROM guid_twin_dissolve_log";
/// The log's high-water mark (a dry plan records it; a wet run on that plan reads the rows above it).
pub(crate) const GUID_TWIN_LOG_HIGH_WATER_SQL: &str = "SELECT rowid FROM guid_twin_dissolve_log ORDER BY rowid DESC LIMIT 1";
/// One org's logged moves, through `guid_twin_dissolve_log_org` (the planner's absent controls).
pub(crate) const GUID_TWIN_LOG_ORG_SQL: &str = "SELECT target_id, whole FROM guid_twin_dissolve_log WHERE org_id = ?";
/// A mention's bound org, by primary key (the window's still-on-plan guard).
pub(crate) const GUID_TWIN_MENTION_ORG_SQL: &str =
    "SELECT organization_id FROM organization_mentions WHERE notice_id = ? AND section_id = ?";
/// A whole row's recorded mentions, through `organization_mentions_org`.
pub(crate) const GUID_TWIN_ORG_MENTIONS_SQL: &str =
    "SELECT notice_id, section_id FROM organization_mentions WHERE organization_id = ?";
/// A planned row's current pre-image, by primary key (the manifest logs what the row IS, not the plan).
pub(crate) const GUID_TWIN_ORG_PREIMAGE_SQL: &str = "SELECT country, identifier_kind, identifier FROM organizations WHERE id = ?";

fn wet_refused(msg: String) -> turso::Error {
    turso::Error::Error(format!("dissolve-guid-twins window refused: {msg}; nothing of this window is kept"))
}

impl Db {
    /// Build 2: apply ONE wet window in one `BEGIN IMMEDIATE` (any error rolls the whole window back):
    ///
    /// 1. every planned mention must still sit on its planned GUID row (`apply_rehoming`'s guard), every
    ///    planned row must still carry its planned identifier (its pre-image — country, identifier kind,
    ///    value — read here and logged as it is), a whole row's recorded mentions must be exactly its
    ///    planned ones, and no whole row may be under review: the review keep-set is re-read on the
    ///    writer inside the window, as the 443 sweep does, so a verdict POSTed during the re-plan or
    ///    between windows refuses the window instead of losing its referent;
    /// 2. whole rows dissolve through [`Db::dissolve_org_mentions`] with no selector, partial rows rehome
    ///    exactly their planned mentions — the target found as the issue-234 resolver arm finds it
    ///    (`DissolveTarget::Resolver`), a missing key's row minted ONCE under the plan's mint name;
    /// 3. A11: each moved mention must land on its planned target, or for a key the plan found no row
    ///    for, on the row the window resolved for that key;
    /// 4. every Tender a moved row touched is stamped stale and its notices re-queued (the issue-179 pair):
    ///    tier 5's, and since issue 579 the rest, whose moved rows sit where the fold would not write them,
    ///    so the trailing `project` gives them the fold's own row order; the re-bound rows' name variants
    ///    re-derived (542), every moved mention logged in `guid_twin_dissolve_log`, and the changes
    ///    announced: `organization removed` (whole rows) and `added` (mints), inside the rebind;
    ///    `organization changed` for reused targets and rehomed rows, `tender changed` for every Tender a
    ///    moved row touched.
    ///
    /// The change-feed doorbell rings after COMMIT, best-effort (the rederive-eur rule: a doorbell that
    /// fails to ring is logged, not returned — failing would report a committed window as not applied).
    /// The caller checkpoints between windows. The window times its wait for the writer and its hold of
    /// it (`wait_ms`, `hold_ms`).
    pub async fn dissolve_guid_twin_window(&self, args: GuidTwinWindowArgs<'_>) -> turso::Result<GuidTwinWindowDone> {
        let w = args.window;
        let now = crate::now_unix();
        let waited = Instant::now();
        let conn = self.conn().await;
        let wait_ms = waited.elapsed().as_millis() as u64;
        let held = Instant::now();
        let mut done = Db::immediate(&conn, async {
            let mut done = GuidTwinWindowDone {
                window_no: args.window_no,
                keys: w.keys.len() as u64,
                whole: w.whole.len() as u64,
                partial: w.partial.len() as u64,
                ..Default::default()
            };
            // 1. The still-on-plan guard.
            let mut by_row: BTreeMap<i64, BTreeSet<(i64, String)>> = BTreeMap::new();
            let mut planned_of: HashMap<(i64, String), &GuidTwinWetMention> = HashMap::new();
            for m in &w.mentions {
                let mut rows = conn
                    .query(GUID_TWIN_MENTION_ORG_SQL, (Value::Integer(m.notice_id), Value::Text(m.section_id.clone())))
                    .await?;
                let here = match rows.next().await? {
                    Some(row) => Some(int(&row, 0)),
                    None => None,
                };
                drop(rows);
                if here != Some(m.org_id) {
                    return Err(wet_refused(format!(
                        "window {}: mention ({}, {}) sits on {here:?}, the plan has it on {}",
                        args.window_no, m.notice_id, m.section_id, m.org_id
                    )));
                }
                // F3: a rehoming verdict on a planned mention, recorded after the re-plan (the re-plan
                // leaves a verdict's mention unplanned): a person placed it, so no window moves it.
                if guid_twin_has_verdict(&conn, m.notice_id, &m.section_id).await? {
                    return Err(wet_refused(format!(
                        "window {}: mention ({}, {}) has a rehoming verdict (org_mention_rehoming) recorded after the \
                         re-plan; a person placed it — re-run the dry plan",
                        args.window_no, m.notice_id, m.section_id
                    )));
                }
                by_row.entry(m.org_id).or_default().insert((m.notice_id, m.section_id.clone()));
                planned_of.insert((m.notice_id, m.section_id.clone()), m);
            }
            // Each planned row's pre-image as it stands: still there, still its planned identifier.
            let mut preimage: HashMap<i64, (Option<String>, Option<String>)> = HashMap::new();
            for (&id, _) in &by_row {
                let planned_value = w.mentions.iter().find(|m| m.org_id == id).map(|m| m.org_value.clone()).unwrap_or_default();
                let mut rows = conn.query(GUID_TWIN_ORG_PREIMAGE_SQL, (Value::Integer(id),)).await?;
                let row = match rows.next().await? {
                    Some(row) => Some((opt_text_of(&row, 0), opt_text_of(&row, 1), opt_text_of(&row, 2))),
                    None => None,
                };
                drop(rows);
                match row {
                    Some((country, kind, Some(identifier))) if identifier == planned_value => {
                        preimage.insert(id, (country, kind));
                    }
                    Some((_, _, identifier)) => {
                        return Err(wet_refused(format!(
                            "window {}: row {id} now carries identifier {identifier:?}, the plan read {planned_value:?}",
                            args.window_no
                        )));
                    }
                    None => {
                        return Err(wet_refused(format!("window {}: planned row {id} no longer exists", args.window_no)));
                    }
                }
            }
            // No whole row under review (the keep-set re-read on the writer, inside this window).
            let keep = Db::review_keep_set(&conn).await?;
            let under_review: Vec<i64> = w.whole.iter().copied().filter(|id| keep.contains(id)).collect();
            if !under_review.is_empty() {
                return Err(wet_refused(format!(
                    "window {}: whole row(s) {under_review:?} now under review (a verdict was recorded after the re-plan)",
                    args.window_no
                )));
            }
            drop(keep);
            let mut targets = crate::canonical::ResolverTargets::default();
            for m in &w.mentions {
                if let Some(country) = &m.key_country {
                    targets.mint_names.entry((m.key_name_norm.clone(), country.clone())).or_insert_with(|| m.mint_name.clone());
                }
            }
            let mut report = crate::OrgDissolve::default();
            let (mut touched, mut changed, mut refold, mut rebound) =
                (BTreeSet::new(), BTreeSet::new(), BTreeSet::new(), BTreeSet::new());
            let mut moved: Vec<(i64, String, i64)> = Vec::new();
            // 2. Whole rows, then partial rows.
            for &id in &w.whole {
                let planned = by_row.get(&id).cloned().unwrap_or_default();
                let mut recorded: BTreeSet<(i64, String)> = BTreeSet::new();
                let mut rows = conn.query(GUID_TWIN_ORG_MENTIONS_SQL, (Value::Integer(id),)).await?;
                while let Some(row) = rows.next().await? {
                    recorded.insert((int(&row, 0), text(&row, 1)));
                }
                drop(rows);
                if recorded != planned {
                    return Err(wet_refused(format!(
                        "window {}: whole row {id} records {} mention(s), the plan {} — it is no longer whole",
                        args.window_no,
                        recorded.len(),
                        planned.len()
                    )));
                }
                Box::pin(self.dissolve_org_mentions(
                    &conn,
                    id,
                    None,
                    &mut crate::canonical::DissolveTarget::Resolver(&mut targets),
                    now,
                    false,
                    &mut report,
                    &mut touched,
                    &mut changed,
                    &mut refold,
                    &mut rebound,
                    &mut moved,
                ))
                .await?;
            }
            for &id in &w.partial {
                let set = by_row.get(&id).cloned().unwrap_or_default();
                Box::pin(self.dissolve_org_mentions(
                    &conn,
                    id,
                    Some(&set),
                    &mut crate::canonical::DissolveTarget::Resolver(&mut targets),
                    now,
                    false,
                    &mut report,
                    &mut touched,
                    &mut changed,
                    &mut refold,
                    &mut rebound,
                    &mut moved,
                ))
                .await?;
            }
            // 3. A11: the landed target is the planned one (or, for a missing key, the key's row).
            for (notice, section, target) in &moved {
                let Some(m) = planned_of.get(&(*notice, section.clone())) else {
                    return Err(wet_refused(format!("window {}: ({notice}, {section}) moved but was not planned", args.window_no)));
                };
                let key_row = m.key_country.as_ref().and_then(|c| targets.cache.get(&(m.key_name_norm.clone(), c.clone())));
                let ok = key_row == Some(target) && m.target.is_none_or(|t| t == *target);
                if !ok {
                    return Err(wet_refused(format!(
                        "window {}: ({notice}, {section}) landed on {target}, the plan's target is {:?} for key ({:?}, {:?})",
                        args.window_no, m.target, m.key_name_norm, m.key_country
                    )));
                }
            }
            done.rebound_estimate = w.rebound_estimate;
            // 4. The refold pair (every touched Tender since issue 579), 542's variants, the log, the change rows.
            if !refold.is_empty() {
                let ids: Vec<i64> = refold.iter().copied().collect();
                let notices = Self::notice_ids_of_tenders(&conn, &ids).await?;
                report.refold_notices += Self::requeue_notice_ids(&conn, &notices, false).await?;
                Self::stamp_tenders_stale(&conn, ids).await?;
            }
            report.refold_tenders = refold.len() as u64;
            report.tender_changes = touched.len() as u64;
            report.variant_orgs = rebound.len() as u64;
            for &org in &rebound {
                let mut rows = conn.query(GUID_TWIN_TARGET_COUNT_SQL, (Value::Integer(org),)).await?;
                if let Some(row) = rows.next().await? {
                    done.rebound_target_mentions += int(&row, 0).max(0) as u64;
                }
            }
            report.variants_written = Self::rederive_org_name_variants(&conn, args.fields, args.normalize, &rebound).await?;
            let mut minted_mentions: BTreeMap<i64, u64> = BTreeMap::new();
            for (notice, section, target) in &moved {
                let m = planned_of[&(*notice, section.clone())];
                let minted = targets.minted.contains_key(target);
                if minted {
                    *minted_mentions.entry(*target).or_default() += 1;
                }
                let (country, kind) = preimage.get(&m.org_id).cloned().unwrap_or_default();
                conn.execute(
                    GUID_TWIN_LOG_INSERT_SQL,
                    vec![
                        Value::Integer(args.job_id),
                        Value::Integer(now),
                        Value::Integer(args.window_no as i64),
                        args.gate_since_notice_id.map_or(Value::Null, Value::Integer),
                        Value::Integer(m.org_id),
                        country.map_or(Value::Null, Value::Text),
                        kind.map_or(Value::Null, Value::Text),
                        Value::Text(m.org_value.clone()),
                        Value::Text(m.org_name.clone()),
                        Value::Integer(i64::from(m.whole)),
                        Value::Integer(*notice),
                        Value::Text(section.clone()),
                        Value::Integer(*target),
                        Value::Integer(i64::from(minted)),
                    ],
                )
                .await?;
            }
            for &tid in &touched {
                crate::canonical::append_change(&conn, "tender", tid, None, "changed", now).await?;
            }
            for &org in &changed {
                crate::canonical::append_change(&conn, "organization", org, None, "changed", now).await?;
            }
            done.mentions_moved = moved.len() as u64;
            done.minted = targets
                .minted
                .iter()
                .map(|(&id, (country, name_norm, name))| GuidTwinMinted {
                    target_id: id,
                    country: country.clone(),
                    name_norm: name_norm.clone(),
                    name: name.clone(),
                    mentions: minted_mentions.get(&id).copied().unwrap_or(0),
                })
                .collect();
            let rehomed: BTreeSet<i64> = w.partial.iter().copied().collect();
            done.reused_targets = changed.iter().copied().filter(|id| !rehomed.contains(id)).collect();
            done.refold_tenders = refold.into_iter().collect();
            done.counts = report;
            Ok(done)
        })
        .await?;
        done.wait_ms = wait_ms;
        done.hold_ms = held.elapsed().as_millis() as u64;
        if let Err(e) = self.publish_cursor(&conn).await {
            eprintln!("[issue 530] guid-twin window {} committed; doorbell: {e}", args.window_no);
        }
        Ok(done)
    }

    /// Build 2: one wet job's manifest rows (`guid_twin_dissolve_log`), by org then mention.
    pub async fn guid_twin_log(&self, job_id: i64) -> turso::Result<Vec<GuidTwinLogRow>> {
        self.guid_twin_log_rows(GUID_TWIN_LOG_JOB_SQL, job_id).await
    }

    /// Build 2: every manifest row logged after the rowid `high_water`, whichever wet job wrote it — the
    /// rows the wet runs on one stored dry plan (`high_water` its `log_high_water`) already committed,
    /// which the next wet run's tuple reconcile adds back to its re-plan (a staged run, a restarted job)
    /// and its manifest reports (the campaign on that plan).
    pub async fn guid_twin_log_after(&self, high_water: i64) -> turso::Result<Vec<GuidTwinLogRow>> {
        self.guid_twin_log_rows(GUID_TWIN_LOG_AFTER_SQL, high_water).await
    }

    /// Build 2: A1's boundary as the wet run recorded it — the `gate_since_notice_id` of the first logged
    /// row (every wet window logs it, and a wet run refuses another value once one is recorded); `None`
    /// before any wet window committed.
    pub async fn guid_twin_recorded_boundary(&self) -> turso::Result<Option<i64>> {
        let reader = self.reader().await?;
        let mut rows = reader.query(GUID_TWIN_LOG_BOUNDARY_SQL, ()).await?;
        Ok(match rows.next().await? {
            Some(row) => opt_int_of(&row, 0),
            None => None,
        })
    }

    /// Build 2: `(country, name_norm, name)` of the given rows, by primary key (the manifest names a
    /// staged run's earlier mints; a row since gone is absent).
    pub async fn guid_twin_rows_named(&self, ids: &[i64]) -> turso::Result<BTreeMap<i64, (Option<String>, String, String)>> {
        let reader = self.reader().await?;
        let mut out = BTreeMap::new();
        for &id in ids {
            let mut rows = reader.query("SELECT country, name_norm, name FROM organizations WHERE id = ?", (Value::Integer(id),)).await?;
            if let Some(row) = rows.next().await? {
                out.insert(id, (opt_text_of(&row, 0), opt_text_of(&row, 1).unwrap_or_default(), text(&row, 2)));
            }
        }
        Ok(out)
    }

    async fn guid_twin_log_rows(&self, sql: &str, key: i64) -> turso::Result<Vec<GuidTwinLogRow>> {
        let reader = self.reader().await?;
        let mut rows = reader.query(sql, vec![Value::Integer(key)]).await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(GuidTwinLogRow {
                job_id: int(&row, 0),
                window_no: int(&row, 1),
                org_id: int(&row, 2),
                org_country: opt_text_of(&row, 3),
                org_kind: opt_text_of(&row, 4),
                org_value: text(&row, 5),
                org_name: text(&row, 6),
                whole: int(&row, 7) != 0,
                notice_id: int(&row, 8),
                section_id: text(&row, 9),
                target_id: int(&row, 10),
                target_minted: int(&row, 11) != 0,
            });
        }
        out.sort_by(|a, b| (a.org_id, a.notice_id, &a.section_id).cmp(&(b.org_id, b.notice_id, &b.section_id)));
        Ok(out)
    }
}

/// Build 2's tuple reconcile (decision "Wet run": the re-plan, not the dry list, is what runs; its count
/// is the dry count, less the guard's additions, plus the rows minted since): the dry plan's rows
/// `{(id, whole)}` must equal the re-plan's rows at or under the dry plan's `max_id` plus the rows the
/// wet runs on it already logged at or under that max (an earlier attempt's committed windows), and no
/// row may be both re-planned and logged. Rows above `max_id` were minted since: the re-plan's are
/// listed (`minted_since`) and run, the logged ones (`logged_minted_since`: a staged or restarted run
/// already applied them) are listed; neither is compared.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct GuidTwinReconcile {
    pub dry_rows: u64,
    pub replan_rows: u64,
    pub logged_rows: u64,
    pub minted_since: Vec<i64>,
    /// Logged rows above the dry plan's `max_id`: minted since, already applied (left out of the compare).
    pub logged_minted_since: Vec<i64>,
    /// In the dry plan, neither re-planned nor logged (a row the plan no longer takes).
    pub dropped: Vec<(i64, bool)>,
    /// Re-planned or logged, not in the dry plan, at or under its max (a row the dry plan did not take).
    pub added: Vec<(i64, bool)>,
    /// Both re-planned and logged (a committed window's row planned again), whatever its id.
    pub replanned_logged: Vec<i64>,
    pub ok: bool,
}

/// [`GuidTwinReconcile`] over `(id, whole)` sets.
pub fn guid_twin_reconcile(
    dry: &BTreeSet<(i64, bool)>,
    dry_max_id: i64,
    replan: &BTreeSet<(i64, bool)>,
    logged: &BTreeSet<(i64, bool)>,
) -> GuidTwinReconcile {
    let mut r = GuidTwinReconcile {
        dry_rows: dry.len() as u64,
        replan_rows: replan.len() as u64,
        logged_rows: logged.len() as u64,
        ..Default::default()
    };
    let logged_ids: BTreeSet<i64> = logged.iter().map(|(id, _)| *id).collect();
    let mut have: BTreeSet<(i64, bool)> = BTreeSet::new();
    for &(id, whole) in logged {
        if id > dry_max_id {
            r.logged_minted_since.push(id);
        } else {
            have.insert((id, whole));
        }
    }
    r.logged_minted_since.dedup();
    for &(id, whole) in replan {
        if logged_ids.contains(&id) {
            r.replanned_logged.push(id);
        }
        if id > dry_max_id {
            r.minted_since.push(id);
            continue;
        }
        have.insert((id, whole));
    }
    r.dropped = dry.difference(&have).copied().collect();
    r.added = have.difference(dry).copied().collect();
    r.ok = r.dropped.is_empty() && r.added.is_empty() && r.replanned_logged.is_empty();
    r
}

/// A1 (Build 2): the gate's candidate test on a stored mention, `(raw, scheme, country)` →
/// `Some((value, register country))` when its gated identifier is a national uuid_v4 — exactly the
/// identity a rebuild binds the mention to when rule 530-T does not refuse it
/// (`ingest::project::guid_twin::guid_twin_gated_v4`).
pub type GuidTwinGatedV4 = fn(Option<&str>, Option<&str>, Option<&str>) -> Option<(String, Option<String>)>;

/// A1 (Build 2): [`Db::guid_twin_recurrence`]'s inputs.
pub struct GuidTwinRecurrenceArgs<'a> {
    pub is_guid: fn(&str) -> bool,
    /// The gate's candidate test ([`GuidTwinGatedV4`]): the reverse-drift half reads it.
    pub gated_v4: GuidTwinGatedV4,
    pub rule: GuidTwinRule,
    pub role_ref: GuidTwinRoleRef,
    /// The compiled KEEP (`keep_530()`): a value already on it is no candidate.
    pub keep: &'a GuidKeep,
    /// Also walk the copies of notices above this id (A1's boundary: the highest notice id at or below
    /// which every parsed notice was projected before Build 2's deploy, as the wet run recorded it): the
    /// identifier-NULL rows their refused sections are bound to join the targets, and their unrefused v4
    /// sections bound to identifier-NULL rows are reverse drift.
    pub since_notice_id: Option<i64>,
    /// Notice ids per window of that walk.
    pub window: i64,
    pub samples: usize,
    pub stop: &'a (dyn Fn() -> bool + Sync),
    pub progress: &'a (dyn Fn(u64, &str) + Sync),
}

/// One refused value at two or more Tenders outside KEEP: a KEEP-add candidate, with its restore cohort.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct GuidTwinRecurrenceValue {
    pub value: String,
    pub tenders: u64,
    pub refused_mentions: u64,
    /// The value's own GUID row (an `organizations.identifier` seek): the lowest id, which names the
    /// value's KEEP line; `own_rows` (review round 3, F4) is every row holding the value, by id, and
    /// `own_row_mentions` their mentions together — `tenders` counts the Tenders of all of them.
    pub own_row: Option<i64>,
    pub own_rows: Vec<i64>,
    pub own_row_mentions: u64,
    /// `(notice_id, section_id, target_id)` of every refused mention, uncapped: the restore list.
    pub cohort: Vec<(i64, String, i64)>,
}

/// A1 (Build 2): one `(value, register country)` pair in REVERSE drift — its mentions sit on
/// identifier-NULL rows although the CURRENT rule binds them (a value added to KEEP, or its name to
/// `GUID_TWIN_UNSAFE_NAMES`, after the mentions moved). What a restore ([`Db::restore_guid_twin_value`])
/// moves, and where. One entry per pair, never per value alone: the resolver keys an identified row on
/// `(country, kind, value)`, so a value published under DE and under AT binds two rows in a rebuild.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct GuidTwinDriftValue {
    /// The gated value (the identifier a rebuild binds) and its register country.
    pub value: String,
    pub country: Option<String>,
    /// The published name of the lowest drifted `(notice, section)`: a mint's name (a rebuild mints the
    /// row from the first mention it resolves).
    pub name: String,
    /// `own_row` (the identity `(country, national, value)` stands: an `organizations` row, or a merged
    /// identifier's survivor) or `mint` (one row, as a rebuild mints it).
    pub restore: &'static str,
    pub own_row: Option<i64>,
    /// The GUID row a wet run dissolved for this value under this country (`guid_twin_dissolve_log`'s
    /// pre-image), if any.
    pub dissolved_org_id: Option<i64>,
    pub dissolved_name: Option<String>,
    /// `(notice_id, section_id, current row)` of every drifted mention, uncapped, by mention.
    pub cohort: Vec<(i64, String, i64)>,
}

/// A1's report (`guid-twin-recurrence`): pass means no candidate, no reverse drift and no forward drift.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct GuidTwinRecurrence {
    pub since_notice_id: Option<i64>,
    pub notice_max_id: i64,
    pub targets_from_log: u64,
    pub targets_from_since: u64,
    pub since_notices_read: u64,
    pub since_twin_copies: u64,
    pub since_refused_mentions: u64,
    /// Refused mentions of post-`since` copies bound to an IDENTIFIED row: the gate refuses them, yet
    /// they sit there — a refold never moves them (`RecordedMention::publishes` keeps a mention whose
    /// published facts did not change), so something rebound them or they were recorded before the
    /// gate re-decided their copy. Counted, listed; those on a GUID row are `forward_drift_since`.
    pub anomaly_count: u64,
    pub anomalies: Vec<GuidTwinOffGuidSample>,
    pub targets: u64,
    pub target_mentions_read: u64,
    /// Target mentions the gate's candidate test admits (review round 3, F5: [`GuidTwinGatedV4`] over the
    /// stored raw value, scheme and country — the gated identifier is a national uuid_v4 key, exactly as
    /// the gate and the planner read it), the copies they sit in, and those the rule refuses (re-decided
    /// per copy with the compiled statics).
    pub guid_mentions: u64,
    pub notices_decided: u64,
    pub refused_mentions: u64,
    pub values: u64,
    pub single_tender_values: u64,
    pub candidate_count: u64,
    pub candidates: Vec<GuidTwinRecurrenceValue>,
    /// REVERSE drift: mentions that sit on an identifier-NULL row although the current rule binds them —
    /// rule 530-T's constants are add-only, and a KEEP add (or an unsafe-name add) after the deploy
    /// leaves every mention the gate or a wet run already moved where it is: a refold never moves it
    /// back (`RecordedMention::publishes`), while a rebuild binds it by identifier. Two halves: `stock`,
    /// a logged mention still on its logged target that the current rule binds by a national v4 identity
    /// (its gated identifier still a v4 key, no longer refused; never every GUID-raw mention on a target: a
    /// verdict or a merge can put one there legitimately); `since`, a
    /// mention of a post-`since` copy whose gated identifier is a national v4 key, bound to an
    /// identifier-NULL row, that the current rule does not refuse (every copy read, twin or not, so a
    /// pair whose values were both added is still seen) and that no reviewed rehoming verdict placed there.
    /// Distinct mentions; each value is listed whole with its restore target. Pass needs 0.
    pub reverse_drift_mentions: u64,
    pub reverse_drift_stock: u64,
    pub reverse_drift_since: u64,
    /// Logged mentions still on their logged target, not refused, whose gated identifier is no longer a
    /// national v4 key at all (an idgate condemn, a scheme on issue 365's refused list, a re-parsed raw):
    /// a rebuild binds them through the 234 arm — where they already sit — so they are NO drift and no
    /// restore moves them. Reported only; never in `reverse_drift_mentions` or `pass`.
    pub reverse_drift_stock_ungated: u64,
    /// Mentions a reviewed rehoming verdict names (`org_mention_rehoming`, by its primary key), in either
    /// half (`since`, and since review round 3, F3, `stock` — a `keep` verdict on a logged mention still
    /// on its logged target): a person put them where they sit, so they are no drift and a restore never
    /// moves them.
    pub reverse_drift_verdict_skipped: u64,
    pub reverse_drift_values: u64,
    pub reverse_drift: Vec<GuidTwinDriftValue>,
    /// FORWARD drift, reverse drift's mirror: mentions the current rule refuses that still sit on a GUID
    /// row. A 434 refresh rebinds a changed sibling on its own (a re-parse filling a country, re-decoding
    /// a name, adding an ORG section turns a copy into a twin) and keeps the unchanged one where it was,
    /// so a refold and a rebuild diverge until a dry re-plan and a wet re-run move it. `stock`: a refused
    /// section of a copy step 2 decides that sits on a GUID row (a LOWER BOUND: step 2 reads only copies
    /// with a mention on a target); `since`: the anomalies of post-`since` copies on a GUID row. Pass
    /// needs 0; the remedy is a dry plan (it plans exactly these mentions) and a wet run on it.
    pub forward_drift_stock: u64,
    pub forward_drift_since: u64,
    pub forward_drift: Vec<GuidTwinOffGuidSample>,
    /// Review round 3 (F3): refused mentions on a GUID row that a reviewed rehoming verdict names (a
    /// Risk-2 wrong fold the reviewer rehomed onto the body's GUID row): no forward drift, never in
    /// `pass`, never planned (the planner reads the same verdicts).
    pub forward_drift_verdict_skipped: u64,
    /// Review round 3 (F9): the highest notice id among `forward_drift_since`'s copies that look folded
    /// BEFORE the gate — every refused section on an identified row, none on an identifier-NULL row
    /// unless a verdict placed it there. Before any wet window such a post-boundary copy means the
    /// boundary is below the deploy (the wet run and the restore refuse naming it, and it is the value to
    /// pass). A refresh residue — the copy's re-resolved refused sibling on the name-keyed row, its
    /// unchanged one on its GUID row — is excluded: it is counted in `forward_drift_since` and
    /// `forward_drift_since_residue` only.
    pub forward_drift_since_max_notice: Option<i64>,
    /// `forward_drift_since`'s mentions in a refresh residue's copy (a refused section of the same copy
    /// sits on an identifier-NULL row no verdict placed): real drift, remedied by a wet run, never read as
    /// a pre-gate fold.
    pub forward_drift_since_residue: u64,
    /// `forward_drift_since`'s GUID rows outside a residue's copy, each with the highest notice id of its
    /// drifted mentions (the wet run checks them against its re-plan's rows). Not serialized.
    #[serde(skip)]
    pub forward_drift_since_rows: BTreeMap<i64, i64>,
    pub pass: bool,
    pub runtime_ms: u64,
    pub stopped: bool,
}

/// The mentions of `n` target orgs with their raw values, schemes and countries (the gate's candidate
/// test reads all three, F5), through `organization_mentions_org`.
pub(crate) fn guid_twin_target_mentions_sql(n: usize) -> String {
    format!(
        "SELECT organization_id, notice_id, section_id, raw_identifier, scheme, country FROM organization_mentions \
          WHERE organization_id IN ({})",
        placeholders(n)
    )
}

/// One notice-id window of ORG mentions (the `organization_mentions` primary key's range).
pub(crate) const GUID_TWIN_SINCE_MENTIONS_SQL: &str = "SELECT notice_id, section_id, organization_id, name, country, \
     raw_identifier, scheme FROM organization_mentions WHERE notice_id > ? AND notice_id <= ? \
     AND section_id >= 'ORG-' AND section_id < 'ORG.'";

/// A value's own GUID rows, every one by id (review round 3, F4: a value on two rows — DE and AT, or
/// issue 62's duplicate triple — counts the Tenders of both), through `organizations_identifier_id`.
pub(crate) const GUID_TWIN_OWN_ROW_SQL: &str = "SELECT id FROM organizations WHERE identifier = ? ORDER BY id";
/// A row's mention notices, through `organization_mentions_org`.
pub(crate) const GUID_TWIN_ROW_NOTICES_SQL: &str = "SELECT notice_id FROM organization_mentions WHERE organization_id = ?";
/// A reviewed rehoming verdict on one mention, by `org_mention_rehoming`'s primary key.
pub(crate) const GUID_TWIN_REHOMING_VERDICT_SQL: &str =
    "SELECT 1 FROM org_mention_rehoming WHERE notice_id = ? AND section_id = ? LIMIT 1";
/// The row a rebuild binds a national value to: the identity triple's row (through
/// `organizations_identifier_id`, the kind and country filtered; a country-less value matches a
/// country-less row, as the resolver keys it), else a merged identifier's survivor (the merged table's
/// primary-key prefix).
pub(crate) const GUID_TWIN_IDENTITY_ROW_SQL: &str = "SELECT id FROM organizations WHERE identifier = ? \
     AND identifier_kind = 'national' AND country IS ? ORDER BY id LIMIT 1";
pub(crate) const GUID_TWIN_IDENTITY_MERGED_SQL: &str = "SELECT org_id FROM organization_merged_identifiers \
     WHERE identifier = ? AND identifier_kind = 'national' AND country IS ? LIMIT 1";

/// Every `(notice_id, section_id)` a rehoming verdict names, whatever its action (F3: the planner's
/// [`GuidTwinPlanArgs::rehomed`]). A hand-review sized table, read whole once.
pub(crate) const GUID_TWIN_REHOMED_KEYS_SQL: &str = "SELECT notice_id, section_id FROM org_mention_rehoming";

/// Whether a reviewed rehoming verdict names `(notice, section)` ([`GUID_TWIN_REHOMING_VERDICT_SQL`], the
/// table's primary key): a person put that mention where it sits, so no drift read counts it and no
/// window moves it (F3).
async fn guid_twin_has_verdict(conn: &turso::Connection, notice: i64, section: &str) -> turso::Result<bool> {
    let mut rows = conn.query(GUID_TWIN_REHOMING_VERDICT_SQL, (Value::Integer(notice), Value::Text(section.to_owned()))).await?;
    Ok(rows.next().await?.is_some())
}

impl Db {
    /// Review round 3 (F3): every `(notice_id, section_id)` a rehoming verdict names — the planner's
    /// [`GuidTwinPlanArgs::rehomed`] (one read of a hand-review sized table).
    pub async fn guid_twin_rehomed_keys(&self) -> turso::Result<HashSet<(i64, String)>> {
        let reader = self.reader().await?;
        let mut rows = reader.query(GUID_TWIN_REHOMED_KEYS_SQL, ()).await?;
        let mut out = HashSet::new();
        while let Some(row) = rows.next().await? {
            out.insert((int(&row, 0), text(&row, 1)));
        }
        Ok(out)
    }
}

/// The row [`GUID_TWIN_IDENTITY_ROW_SQL`] / [`GUID_TWIN_IDENTITY_MERGED_SQL`] give `(value, country)`.
async fn guid_twin_identity_row(conn: &turso::Connection, value: &str, country: Option<&str>) -> turso::Result<Option<i64>> {
    let country = country.map_or(Value::Null, |c| Value::Text(c.to_owned()));
    for sql in [GUID_TWIN_IDENTITY_ROW_SQL, GUID_TWIN_IDENTITY_MERGED_SQL] {
        let mut rows = conn.query(sql, (Value::Text(value.to_owned()), country.clone())).await?;
        if let Some(row) = rows.next().await? {
            return Ok(Some(int(&row, 0)));
        }
    }
    Ok(None)
}

/// The rule over each copy of `by_notice` (twin copies get their role references first): the refused
/// `(notice, section)` keys.
async fn guid_twin_refused_in(
    reader: &crate::read::Reader,
    rule: GuidTwinRule,
    role_ref: GuidTwinRoleRef,
    keep: &GuidKeep,
    by_notice: &BTreeMap<i64, Vec<MentionRow>>,
) -> turso::Result<(BTreeSet<(i64, String)>, u64)> {
    let none = GuidTwinSectionRoles::new();
    let twins: Vec<i64> = by_notice
        .iter()
        .filter(|(_, ms)| !rule(&guid_twin_views(ms, &none), keep, &[]).groups.is_empty())
        .map(|(n, _)| *n)
        .collect();
    let mut roles: HashMap<i64, GuidTwinSectionRoles> = HashMap::new();
    let mut tally = GuidTwinPlan::default();
    for chunk in twins.chunks(IN_CHUNK) {
        roles.extend(guid_twin_read_roles(reader, chunk, role_ref, &mut tally).await?);
    }
    let mut refused = BTreeSet::new();
    for n in &twins {
        let ms = &by_notice[n];
        let section_roles = roles.get(n).unwrap_or(&none);
        for s in rule(&guid_twin_views(ms, section_roles), keep, &[]).refused {
            refused.insert((*n, s));
        }
    }
    Ok((refused, twins.len() as u64))
}

/// One reverse-drifted mention as the recurrence arm finds it.
struct DriftMention {
    row: i64,
    value: String,
    country: Option<String>,
    name: String,
}

/// One logged mention (`guid_twin_dissolve_log`, the latest row per mention).
struct LoggedMention {
    rowid: i64,
    target: i64,
    org_id: i64,
    org_country: Option<String>,
    org_name: String,
    org_value: String,
}

impl Db {
    /// A1 (Build 2): the refused-value recurrence arm of `dissolve-guid-twins` (report
    /// `guid-twin-recurrence`). The census cannot see a refused value — it sits on an identifier-NULL
    /// row, its value only in the unindexed `raw_identifier` — so this walks the rows refused values
    /// land on:
    ///
    /// 1. Targets: every `target_id` of `guid_twin_dissolve_log`, plus (with `since_notice_id`) the
    ///    identifier-NULL rows bound by the sections rule 530-T refuses in the twin copies of notices
    ///    above it (the `organization_mentions` primary key by notice-id windows, ORG sections; a
    ///    refused section bound to an identified row is an anomaly). In the same walk, every copy's
    ///    unrefused section whose gated identifier is a national v4 key and that is bound to an
    ///    identifier-NULL row is reverse drift (`since`).
    /// 2. The targets' mentions (`organization_mentions_org`, IN chunks) that the gate's candidate test
    ///    admits (`args.gated_v4` over the stored raw value, scheme and country — the gated identifier a
    ///    national uuid_v4 key, keyed by that gated value; review round 3, F5); each such copy re-decided
    ///    whole with the compiled statics, keeping the refused ones — a logged mention still on its
    ///    logged target whose gated identifier is still a national v4 key and that the rule no longer
    ///    refuses is reverse drift (`stock`; a logged mention a later merge or verdict moved off its
    ///    target is not read), and a refused section of such a copy that sits on a GUID row is forward
    ///    drift (`stock`). Either half skips a mention a reviewed rehoming verdict names (F3).
    /// 3. Per refused value: the distinct Tenders over its refused mentions plus the mentions of EVERY
    ///    GUID row holding it (an `organizations_identifier_id` seek; F4). A value at two or more Tenders
    ///    outside KEEP is a KEEP-add candidate, listed with its whole restore cohort. Per drifted value:
    ///    its whole cohort and its restore target (the identity's row, else a mint). Pass means no
    ///    candidate and no drift.
    ///
    /// Read-only. Refused before any read while one of [`GUID_TWIN_INDEXES`] or
    /// `organizations_identifier_id` (the own-row and identity seeks) is absent. The stop flag is read
    /// before every window and chunk and every 64 seeks; a stopped run is an empty report marked
    /// `stopped`.
    pub async fn guid_twin_recurrence(&self, args: GuidTwinRecurrenceArgs<'_>) -> turso::Result<GuidTwinRecurrence> {
        let started = Instant::now();
        for index in GUID_TWIN_INDEXES.into_iter().chain(["organizations_identifier_id"]) {
            if !self.has_index(index).await? {
                return Err(turso::Error::Error(format!(
                    "dissolve-guid-twins recurrence refused: {index} is absent (a rebuild?), so its reads would scan \
                     a table; run `reindex` first. Nothing was read"
                )));
            }
        }
        let stopped = || GuidTwinRecurrence { stopped: true, ..Default::default() };
        let mut out = GuidTwinRecurrence { since_notice_id: args.since_notice_id, ..Default::default() };
        let reader = self.reader().await?;
        let mut rows = reader.query(GUID_TWIN_NOTICE_MAX_ID_SQL, ()).await?;
        out.notice_max_id = match rows.next().await? {
            Some(row) => opt_int_of(&row, 0).unwrap_or(0),
            None => 0,
        };
        drop(rows);
        // 1. The log: the targets, each logged mention's landing row, the dissolved rows' pre-images.
        let mut targets: BTreeSet<i64> = BTreeSet::new();
        let mut logged: HashMap<(i64, String), LoggedMention> = HashMap::new();
        let mut rows = reader.query(GUID_TWIN_LOG_ALL_SQL, ()).await?;
        while let Some(row) = rows.next().await? {
            let m = LoggedMention {
                rowid: int(&row, 0),
                target: int(&row, 3),
                org_id: int(&row, 4),
                org_country: opt_text_of(&row, 5),
                org_name: text(&row, 6),
                org_value: text(&row, 7),
            };
            targets.insert(m.target);
            let key = (int(&row, 1), text(&row, 2));
            if logged.get(&key).is_none_or(|old| old.rowid < m.rowid) {
                logged.insert(key, m);
            }
        }
        drop(rows);
        out.targets_from_log = targets.len() as u64;
        let mut preimage_of_value: BTreeMap<(String, Option<String>), (i64, String)> = BTreeMap::new();
        for m in logged.values() {
            let entry = preimage_of_value
                .entry((GuidKeep::key(&m.org_value), m.org_country.clone()))
                .or_insert((m.org_id, m.org_name.clone()));
            if m.org_id < entry.0 {
                *entry = (m.org_id, m.org_name.clone());
            }
        }
        let mut drift: BTreeMap<(i64, String), DriftMention> = BTreeMap::new();

        // 1b. Post-deploy copies.
        if let Some(since) = args.since_notice_id {
            let mut lo = since;
            let mut from_since: BTreeSet<i64> = BTreeSet::new();
            let mut since_candidates: Vec<(i64, String, DriftMention)> = Vec::new();
            let mut since_guid_rows: Vec<(i64, String, i64, GuidTwinOffGuidSample)> = Vec::new();
            while lo < out.notice_max_id {
                if (args.stop)() {
                    return Ok(stopped());
                }
                let hi = lo.saturating_add(args.window.max(1)).min(out.notice_max_id);
                let mut rows = reader.query(GUID_TWIN_SINCE_MENTIONS_SQL, (Value::Integer(lo), Value::Integer(hi))).await?;
                let mut by_notice: BTreeMap<i64, Vec<MentionRow>> = BTreeMap::new();
                while let Some(row) = rows.next().await? {
                    by_notice.entry(int(&row, 0)).or_default().push(MentionRow {
                        section: text(&row, 1),
                        org: int(&row, 2),
                        name: text(&row, 3),
                        country: opt_text_of(&row, 4),
                        raw: opt_text_of(&row, 5),
                        scheme: opt_text_of(&row, 6),
                    });
                }
                drop(rows);
                out.since_notices_read += by_notice.len() as u64;
                let (refused, twins) = guid_twin_refused_in(&reader, args.rule, args.role_ref, args.keep, &by_notice).await?;
                out.since_twin_copies += twins;
                out.since_refused_mentions += refused.len() as u64;
                let bound: BTreeMap<i64, Vec<(i64, String, String)>> = refused.iter().fold(BTreeMap::new(), |mut acc, (n, s)| {
                    if let Some(m) = by_notice[n].iter().find(|m| &m.section == s) {
                        acc.entry(m.org).or_insert_with(Vec::new).push((*n, s.clone(), m.raw.clone().unwrap_or_default()));
                    }
                    acc
                });
                // Every copy's unrefused sections whose gated identifier is a national v4 key.
                let mut unrefused: BTreeMap<i64, Vec<(i64, String, String, Option<String>, String)>> = BTreeMap::new();
                for (n, ms) in &by_notice {
                    for m in ms {
                        if refused.contains(&(*n, m.section.clone())) {
                            continue;
                        }
                        if let Some((value, country)) = (args.gated_v4)(m.raw.as_deref(), m.scheme.as_deref(), m.country.as_deref()) {
                            unrefused.entry(m.org).or_default().push((*n, m.section.clone(), value, country, m.name.clone()));
                        }
                    }
                }
                let orgs: Vec<i64> = bound.keys().chain(unrefused.keys()).copied().collect::<BTreeSet<_>>().into_iter().collect();
                // Each copy's refused sections bound to an identifier-NULL row (the gate's name-keyed
                // binding), by notice: a copy carrying one is a refresh residue, not a pre-gate fold.
                let mut name_keyed: BTreeMap<i64, Vec<String>> = BTreeMap::new();
                for chunk in orgs.chunks(IN_CHUNK) {
                    let params: Vec<Value> = chunk.iter().map(|&id| Value::Integer(id)).collect();
                    let mut rows = reader.query(&guid_twin_orgs_sql(chunk.len()), params).await?;
                    while let Some(row) = rows.next().await? {
                        let org = int(&row, 0);
                        match opt_text_of(&row, 1) {
                            None => {
                                if let Some(refused_here) = bound.get(&org) {
                                    from_since.insert(org);
                                    for (n, s, _) in refused_here {
                                        name_keyed.entry(*n).or_default().push(s.clone());
                                    }
                                }
                                for (n, s, value, country, name) in unrefused.get(&org).into_iter().flatten() {
                                    since_candidates.push((*n, s.clone(), DriftMention {
                                        row: org,
                                        value: value.clone(),
                                        country: country.clone(),
                                        name: name.clone(),
                                    }));
                                }
                            }
                            Some(identifier) => {
                                let guid_row = (args.is_guid)(&identifier);
                                for (n, s, raw) in bound.get(&org).into_iter().flatten() {
                                    out.anomaly_count += 1;
                                    let sample = GuidTwinOffGuidSample {
                                        notice_id: *n,
                                        section_id: s.clone(),
                                        org_id: org,
                                        kind: if guid_row { "guid_row" } else { "other_identified" },
                                        value: GuidKeep::key(raw),
                                        target: None,
                                    };
                                    if guid_row {
                                        since_guid_rows.push((*n, s.clone(), org, sample.clone()));
                                    }
                                    if out.anomalies.len() < args.samples {
                                        out.anomalies.push(sample);
                                    }
                                }
                            }
                        }
                    }
                }
                // Forward drift (since): a refused section on a GUID row — unless a reviewed rehoming
                // verdict put it there (F3). F9: each drifted row with its highest notice — only from a
                // copy that looks folded BEFORE the gate: every refused section of a pre-gate fold sits on
                // an identified row, while a refresh residue (a re-parse turned the copy into a twin after
                // the gate) carries its re-resolved sibling on the name-keyed row (one no verdict placed).
                // A residue is real drift whose remedy is a wet run, never evidence of a low boundary.
                let mut residue_of: BTreeMap<i64, bool> = BTreeMap::new();
                for (n, s, org, sample) in since_guid_rows.drain(..) {
                    if guid_twin_has_verdict(&reader, n, &s).await? {
                        out.forward_drift_verdict_skipped += 1;
                        continue;
                    }
                    out.forward_drift_since += 1;
                    let residue = match residue_of.get(&n) {
                        Some(&known) => known,
                        None => {
                            let mut found = false;
                            for keyed in name_keyed.get(&n).into_iter().flatten() {
                                if !guid_twin_has_verdict(&reader, n, keyed).await? {
                                    found = true;
                                    break;
                                }
                            }
                            residue_of.insert(n, found);
                            found
                        }
                    };
                    if residue {
                        out.forward_drift_since_residue += 1;
                    } else {
                        out.forward_drift_since_max_notice = out.forward_drift_since_max_notice.max(Some(n));
                        let top = out.forward_drift_since_rows.entry(org).or_insert(n);
                        *top = (*top).max(n);
                    }
                    if out.forward_drift.len() < args.samples {
                        out.forward_drift.push(sample);
                    }
                }
                // A mention a reviewed rehoming verdict placed is no drift.
                for (n, s, m) in since_candidates.drain(..) {
                    if guid_twin_has_verdict(&reader, n, &s).await? {
                        out.reverse_drift_verdict_skipped += 1;
                        continue;
                    }
                    out.reverse_drift_since += 1;
                    drift.insert((n, s), m);
                }
                lo = hi;
                (args.progress)(lo.max(0) as u64, &format!("issue 530 A1: copies to notice {lo} of {}", out.notice_max_id));
            }
            out.targets_from_since = from_since.difference(&targets).count() as u64;
            targets.extend(from_since);
        }
        out.targets = targets.len() as u64;

        // 2. The targets' mentions the gate's candidate test admits, and their copies re-decided. F5: the
        // gate's own test (`gated_v4`: the identifier gated under the mention's scheme and country is a
        // national uuid_v4 key), never `is_guid` on the raw value — a raw BT-501 wrapping the UUID in
        // braces, whitespace or other punctuation is a candidate to the gate and the planner alike — and
        // the GATED value keys it, the identifier a rebuild stores.
        let target_list: Vec<i64> = targets.into_iter().collect();
        let mut on_target: HashMap<(i64, String), (i64, String)> = HashMap::new();
        for chunk in target_list.chunks(IN_CHUNK) {
            if (args.stop)() {
                return Ok(stopped());
            }
            let params: Vec<Value> = chunk.iter().map(|&id| Value::Integer(id)).collect();
            let mut rows = reader.query(&guid_twin_target_mentions_sql(chunk.len()), params).await?;
            while let Some(row) = rows.next().await? {
                out.target_mentions_read += 1;
                let raw = opt_text_of(&row, 3);
                let Some((value, _)) =
                    (args.gated_v4)(raw.as_deref(), opt_text_of(&row, 4).as_deref(), opt_text_of(&row, 5).as_deref())
                else {
                    continue;
                };
                out.guid_mentions += 1;
                on_target.insert((int(&row, 1), text(&row, 2)), (int(&row, 0), value));
            }
        }
        let notices: Vec<i64> = on_target.keys().map(|(n, _)| *n).collect::<BTreeSet<_>>().into_iter().collect();
        out.notices_decided = notices.len() as u64;
        // value -> [(notice, section, target)]
        let mut by_value: BTreeMap<String, Vec<(i64, String, i64)>> = BTreeMap::new();
        for chunk in notices.chunks(IN_CHUNK) {
            if (args.stop)() {
                return Ok(stopped());
            }
            let params: Vec<Value> = chunk.iter().map(|&id| Value::Integer(id)).collect();
            let mut rows = reader.query(&guid_twin_notice_mentions_sql(chunk.len()), params).await?;
            let mut by_notice: BTreeMap<i64, Vec<MentionRow>> = BTreeMap::new();
            while let Some(row) = rows.next().await? {
                by_notice.entry(int(&row, 0)).or_default().push(MentionRow {
                    section: text(&row, 1),
                    org: int(&row, 2),
                    name: text(&row, 3),
                    country: opt_text_of(&row, 4),
                    raw: opt_text_of(&row, 5),
                    scheme: opt_text_of(&row, 6),
                });
            }
            drop(rows);
            let (refused, _) = guid_twin_refused_in(&reader, args.rule, args.role_ref, args.keep, &by_notice).await?;
            // Forward drift (stock): a refused section off the targets, on a row to be read below.
            let mut off_target: BTreeMap<i64, Vec<(i64, String, String)>> = BTreeMap::new();
            for key in &refused {
                if let Some((target, value)) = on_target.get(key) {
                    out.refused_mentions += 1;
                    by_value.entry(value.clone()).or_default().push((key.0, key.1.clone(), *target));
                } else if let Some(m) = by_notice.get(&key.0).and_then(|ms| ms.iter().find(|m| m.section == key.1)) {
                    off_target.entry(m.org).or_default().push((key.0, key.1.clone(), m.raw.clone().unwrap_or_default()));
                }
            }
            let off_orgs: Vec<i64> = off_target.keys().copied().collect();
            let mut on_guid_rows: Vec<(i64, String, i64, String)> = Vec::new();
            for chunk in off_orgs.chunks(IN_CHUNK) {
                let params: Vec<Value> = chunk.iter().map(|&id| Value::Integer(id)).collect();
                let mut rows = reader.query(&guid_twin_orgs_sql(chunk.len()), params).await?;
                while let Some(row) = rows.next().await? {
                    let org = int(&row, 0);
                    let Some(identifier) = opt_text_of(&row, 1) else { continue };
                    if !(args.is_guid)(&identifier) {
                        continue;
                    }
                    for (n, s, raw) in off_target.get(&org).into_iter().flatten() {
                        on_guid_rows.push((*n, s.clone(), org, raw.clone()));
                    }
                }
            }
            // F3: a refused mention a reviewed rehoming verdict put on a GUID row is no forward drift.
            for (n, s, org, raw) in on_guid_rows {
                if guid_twin_has_verdict(&reader, n, &s).await? {
                    out.forward_drift_verdict_skipped += 1;
                    continue;
                }
                out.forward_drift_stock += 1;
                if out.forward_drift.len() < args.samples {
                    out.forward_drift.push(GuidTwinOffGuidSample {
                        notice_id: n,
                        section_id: s,
                        org_id: org,
                        kind: "guid_row",
                        value: GuidKeep::key(&raw),
                        target: None,
                    });
                }
            }
            // The stock half of the reverse drift: a logged mention still on its logged target whose gated
            // identifier is still a national v4 key and that the current rule no longer refuses. One the
            // gate no longer admits as a v4 identity (a condemn, a refused scheme, a re-parsed raw) is no
            // drift: a rebuild binds it through the 234 arm, where it sits. F3: nor is one a reviewed
            // rehoming verdict names (a `keep` verdict on its logged target) — a person put it there.
            let mut stock: Vec<((i64, String), DriftMention)> = Vec::new();
            for (n, ms) in &by_notice {
                for m in ms {
                    let key = (*n, m.section.clone());
                    let Some(l) = logged.get(&key) else { continue };
                    if m.org != l.target || refused.contains(&key) {
                        continue;
                    }
                    let Some((value, country)) = (args.gated_v4)(m.raw.as_deref(), m.scheme.as_deref(), m.country.as_deref())
                    else {
                        out.reverse_drift_stock_ungated += 1;
                        continue;
                    };
                    stock.push((key, DriftMention { row: m.org, value, country, name: m.name.clone() }));
                }
            }
            for (key, m) in stock {
                if guid_twin_has_verdict(&reader, key.0, &key.1).await? {
                    out.reverse_drift_verdict_skipped += 1;
                    continue;
                }
                out.reverse_drift_stock += 1;
                drift.insert(key, m);
            }
        }

        // 3. Tenders per value: its refused mentions' and its own row's.
        out.values = by_value.len() as u64;
        let mut tenders_of_notice: HashMap<i64, Vec<i64>> = HashMap::new();
        let mut want: BTreeSet<i64> = by_value.values().flatten().map(|(n, _, _)| *n).collect();
        // value -> (its GUID rows by id, every mention notice of them): F4, every row holding the value.
        let mut own: BTreeMap<String, (Vec<i64>, Vec<i64>)> = BTreeMap::new();
        for (k, value) in by_value.keys().enumerate() {
            if k % 64 == 0 && (args.stop)() {
                return Ok(stopped());
            }
            let mut rows = reader.query(GUID_TWIN_OWN_ROW_SQL, vec![Value::Text(value.clone())]).await?;
            let mut row_ids: Vec<i64> = Vec::new();
            while let Some(row) = rows.next().await? {
                row_ids.push(int(&row, 0));
            }
            drop(rows);
            if row_ids.is_empty() {
                continue;
            }
            let mut notices: Vec<i64> = Vec::new();
            for &id in &row_ids {
                let mut rows = reader.query(GUID_TWIN_ROW_NOTICES_SQL, vec![Value::Integer(id)]).await?;
                while let Some(row) = rows.next().await? {
                    notices.push(int(&row, 0));
                }
                drop(rows);
            }
            want.extend(notices.iter().copied());
            own.insert(value.clone(), (row_ids, notices));
        }
        let want: Vec<i64> = want.into_iter().collect();
        for chunk in want.chunks(IN_CHUNK) {
            if (args.stop)() {
                return Ok(stopped());
            }
            let params: Vec<Value> = chunk.iter().map(|&id| Value::Integer(id)).collect();
            let mut rows = reader.query(&guid_twin_versions_sql(chunk.len()), params).await?;
            while let Some(row) = rows.next().await? {
                tenders_of_notice.entry(int(&row, 0)).or_default().push(int(&row, 1));
            }
        }
        for (value, cohort) in by_value {
            let mut tenders: BTreeSet<i64> = BTreeSet::new();
            for (n, _, _) in &cohort {
                tenders.extend(tenders_of_notice.get(n).into_iter().flatten().copied());
            }
            let (own_row, own_rows, own_mentions) = match own.get(&value) {
                Some((ids, notices)) => {
                    for n in notices {
                        tenders.extend(tenders_of_notice.get(n).into_iter().flatten().copied());
                    }
                    (ids.first().copied(), ids.clone(), notices.len() as u64)
                }
                None => (None, Vec::new(), 0),
            };
            if tenders.len() < 2 {
                out.single_tender_values += 1;
                continue;
            }
            out.candidate_count += 1;
            if out.candidates.len() < GUID_TWIN_LIST_CAP {
                out.candidates.push(GuidTwinRecurrenceValue {
                    value,
                    tenders: tenders.len() as u64,
                    refused_mentions: cohort.len() as u64,
                    own_row,
                    own_rows,
                    own_row_mentions: own_mentions,
                    cohort,
                });
            }
        }

        // 3b. The drifted (value, register country) pairs, each with its whole cohort and its restore
        // target (the resolver keys an identified row on the triple, so DE and AT are two rows).
        out.reverse_drift_mentions = drift.len() as u64;
        let mut by_drift: BTreeMap<(String, Option<String>), GuidTwinDriftValue> = BTreeMap::new();
        for ((n, s), m) in drift {
            let v = by_drift.entry((GuidKeep::key(&m.value), m.country.clone())).or_insert_with(|| GuidTwinDriftValue {
                value: m.value.clone(),
                country: m.country.clone(),
                name: m.name.clone(),
                ..Default::default()
            });
            v.cohort.push((n, s, m.row));
        }
        for (k, (key, v)) in by_drift.iter_mut().enumerate() {
            if k % 64 == 0 && (args.stop)() {
                return Ok(stopped());
            }
            v.own_row = guid_twin_identity_row(&reader, &v.value, v.country.as_deref()).await?;
            v.restore = if v.own_row.is_some() { "own_row" } else { "mint" };
            if let Some((id, name)) = preimage_of_value.get(key) {
                v.dissolved_org_id = Some(*id);
                v.dissolved_name = Some(name.clone());
            }
        }
        drop(reader);
        out.reverse_drift_values = by_drift.len() as u64;
        out.reverse_drift = by_drift.into_values().collect();
        out.pass = out.candidate_count == 0
            && out.reverse_drift_mentions == 0
            && out.forward_drift_stock + out.forward_drift_since == 0;
        out.runtime_ms = started.elapsed().as_millis() as u64;
        Ok(out)
    }
}

/// A1's reverse drift, restored: [`Db::restore_guid_twin_value`]'s inputs.
pub struct GuidTwinRestoreArgs<'a> {
    pub value: &'a GuidTwinDriftValue,
    /// 542's name-variant re-derivation's field list and language normaliser (ingest's).
    pub fields: &'a [&'a str],
    pub normalize: fn(Option<&str>) -> Option<String>,
}

/// What one restored value did.
#[derive(Debug, Default, Clone, Serialize)]
pub struct GuidTwinRestoreDone {
    pub value: String,
    pub target_id: i64,
    /// The target was minted (`organization added`), not found standing (`organization changed`).
    pub minted: bool,
    /// `(notice_id, section_id, from_row)` of every mention moved onto the target.
    pub moved: Vec<(i64, String, i64)>,
    /// 440's counts (mentions, parties, bid parties, winners, tier 5, variants…).
    pub counts: crate::OrgDissolve,
    /// Tenders stamped stale, their notices re-queued: every Tender a moved row touched (issue 579).
    pub refold_tenders: Vec<i64>,
    /// Rows the restore emptied and deleted as the 443 sweep deletes one (`org_sweep_log` pre-image,
    /// `organization removed`): a row a rebuild under the current constants never mints (a wet window's
    /// mint for a key whose every value now binds).
    pub removed: Vec<i64>,
    /// Rows the restore emptied but left standing: a party, bid-party or winner row still names them, or
    /// a review table protects them — `sweep-orphan-orgs` decides them.
    pub emptied_kept: Vec<i64>,
    pub hold_ms: u64,
}

impl Db {
    /// A1's reverse drift, restored for ONE value in one `BEGIN IMMEDIATE` (any error rolls it back):
    /// every cohort mention must still sit on the row the recurrence arm read it on; the target is the
    /// row a rebuild binds the value to — the identity `(country, national, value)`'s row, else a merged
    /// identifier's survivor, else ONE row minted as the resolver mints an identified one (country,
    /// `national`, value, the lowest drifted mention's published name, not provisional; announced
    /// `organization added`); each current row then rehomes exactly its drifted mentions onto it through
    /// [`Db::dissolve_org_mentions`] (parties and bid parties by the mention key, winners by the 309
    /// origin tier, every touched Tender stamped stale and its notices re-queued, issue 579); a row the
    /// rehome EMPTIED is deleted as the 443 sweep deletes one when nothing else names it and no review
    /// protects it (`removed`; else `emptied_kept`, for `sweep-orphan-orgs`); the re-bound rows' name variants
    /// re-derived (542), and the changes announced (`organization changed` for each row that gave
    /// mentions and stands and for a standing target, `organization removed` for a deleted one, `tender
    /// changed` for each Tender a moved row touched). The doorbell after COMMIT is best-effort.
    pub async fn restore_guid_twin_value(&self, args: GuidTwinRestoreArgs<'_>) -> turso::Result<GuidTwinRestoreDone> {
        let v = args.value;
        let now = crate::now_unix();
        let conn = self.conn().await;
        let held = Instant::now();
        let refuse = |msg: String| turso::Error::Error(format!("dissolve-guid-twins restore refused: {msg}; nothing of this value is kept"));
        let mut done = Db::immediate(&conn, async {
            let mut done = GuidTwinRestoreDone { value: v.value.clone(), ..Default::default() };
            let mut by_row: BTreeMap<i64, BTreeSet<(i64, String)>> = BTreeMap::new();
            for (n, s, row) in &v.cohort {
                let mut rows = conn.query(GUID_TWIN_MENTION_ORG_SQL, (Value::Integer(*n), Value::Text(s.clone()))).await?;
                let here = match rows.next().await? {
                    Some(r) => Some(int(&r, 0)),
                    None => None,
                };
                drop(rows);
                if here != Some(*row) {
                    return Err(refuse(format!("value {}: mention ({n}, {s}) sits on {here:?}, the read had it on {row}", v.value)));
                }
                by_row.entry(*row).or_default().insert((*n, s.clone()));
            }
            let mut changed: BTreeSet<i64> = BTreeSet::new();
            let target = match guid_twin_identity_row(&conn, &v.value, v.country.as_deref()).await? {
                Some(id) => {
                    changed.insert(id);
                    id
                }
                None => {
                    conn.execute(
                        "INSERT INTO organizations(country, identifier_kind, identifier, name, name_norm, provisional, created_at) \
                         VALUES(?, 'national', ?, ?, ?, 0, ?)",
                        (
                            v.country.clone().map_or(Value::Null, Value::Text),
                            Value::Text(v.value.clone()),
                            Value::Text(v.name.clone()),
                            Value::Text(org_name_norm(&v.name)),
                            Value::Integer(now),
                        ),
                    )
                    .await?;
                    let id = crate::canonical::last_insert_rowid(&conn).await?;
                    crate::canonical::append_change(&conn, "organization", id, None, "added", now).await?;
                    done.minted = true;
                    id
                }
            };
            done.target_id = target;
            let mut report = crate::OrgDissolve::default();
            let (mut touched, mut refold, mut rebound) = (BTreeSet::new(), BTreeSet::new(), BTreeSet::new());
            let mut moved: Vec<(i64, String, i64)> = Vec::new();
            for (&row, set) in &by_row {
                if row == target {
                    return Err(refuse(format!("value {}: row {row} is already the target", v.value)));
                }
                Box::pin(self.dissolve_org_mentions(
                    &conn,
                    row,
                    Some(set),
                    &mut crate::canonical::DissolveTarget::Fixed(target),
                    now,
                    false,
                    &mut report,
                    &mut touched,
                    &mut changed,
                    &mut refold,
                    &mut rebound,
                    &mut moved,
                ))
                .await?;
                done.moved.extend(set.iter().map(|(n, s)| (*n, s.clone(), row)));
            }
            // A row the rehome emptied: swept as the 443 sweep sweeps one (a rebuild never mints it).
            for &row in by_row.keys() {
                match Db::sweep_emptied_org(&conn, row, now).await? {
                    None => {}
                    Some(true) => {
                        rebound.remove(&row);
                        changed.remove(&row);
                        report.dissolved += 1;
                        done.removed.push(row);
                    }
                    Some(false) => done.emptied_kept.push(row),
                }
            }
            if !refold.is_empty() {
                let ids: Vec<i64> = refold.iter().copied().collect();
                let notices = Self::notice_ids_of_tenders(&conn, &ids).await?;
                report.refold_notices += Self::requeue_notice_ids(&conn, &notices, false).await?;
                Self::stamp_tenders_stale(&conn, ids).await?;
            }
            report.refold_tenders = refold.len() as u64;
            report.tender_changes = touched.len() as u64;
            report.variant_orgs = rebound.len() as u64;
            report.variants_written = Self::rederive_org_name_variants(&conn, args.fields, args.normalize, &rebound).await?;
            for &tid in &touched {
                crate::canonical::append_change(&conn, "tender", tid, None, "changed", now).await?;
            }
            for &org in changed.iter().filter(|&&org| !(done.minted && org == target)) {
                crate::canonical::append_change(&conn, "organization", org, None, "changed", now).await?;
            }
            done.refold_tenders = refold.into_iter().collect();
            done.counts = report;
            Ok(done)
        })
        .await?;
        done.hold_ms = held.elapsed().as_millis() as u64;
        if let Err(e) = self.publish_cursor(&conn).await {
            eprintln!("[issue 530] guid-twin restore of value {} committed; doorbell: {e}", v.value);
        }
        Ok(done)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guid_census::GUID_ROLE_OTHER;

    /// The class as the job injects it (`ingest::idgate::uuid_v4`), mirrored: store never depends on
    /// ingest.
    fn uuid_v4(value: &str) -> bool {
        let hex: Vec<u8> = value.bytes().filter(|b| *b != b'-').collect();
        hex.len() == 32
            && hex.iter().all(u8::is_ascii_hexdigit)
            && hex[12] == b'4'
            && matches!(hex[16].to_ascii_lowercase(), b'8' | b'9' | b'a' | b'b')
    }

    /// The gate's candidate test as the job injects it (`ingest::project::guid_twin::guid_twin_gated_v4`),
    /// mirrored: a v4 raw is a national key under its mention's country (`DEU` read as `DE`).
    fn gated_v4(raw: Option<&str>, _scheme: Option<&str>, country: Option<&str>) -> Option<(String, Option<String>)> {
        let raw = raw.filter(|r| uuid_v4(r))?;
        Some((GuidKeep::key(raw), country.map(|c| if c == "DEU" { "DE".to_owned() } else { c.to_owned() })))
    }

    /// A miniature rule 530-T for the planner's structural tests (the real one, with its name and role
    /// guards, is `ingest::project::guid_twin`, whose tests pin the planner's parity with it): group the
    /// v4 raws by (country, lower-cased name), refuse the non-KEEP values of a group with two distinct
    /// values. It carries the members' roles into the group but applies NO role guard, so the planner's
    /// role self-check (`bars.tripwire_role_mix`) can be driven to fire.
    fn rule(sections: &[TwinView<'_>], keep: &GuidKeep, extra: &[String]) -> TwinDecision {
        mini_rule(sections, keep, extra, true)
    }

    /// [`rule`], but it LOSES the roles (every group's `roles` empty): the regression the role tripwire
    /// must still catch, because it reads the planner's own reference read, not the rule's echo.
    fn rule_without_roles(sections: &[TwinView<'_>], keep: &GuidKeep, extra: &[String]) -> TwinDecision {
        mini_rule(sections, keep, extra, false)
    }

    fn mini_rule(sections: &[TwinView<'_>], keep: &GuidKeep, _extra: &[String], carry_roles: bool) -> TwinDecision {
        let mut d = TwinDecision::default();
        let mut groups: BTreeMap<(Option<String>, String), (Vec<(String, String)>, BTreeSet<String>)> = BTreeMap::new();
        for s in sections {
            let Some(raw) = s.raw_identifier.filter(|r| uuid_v4(r)) else { continue };
            d.candidates += 1;
            let norm = org_name_norm(s.name);
            let country = s.country.map(|c| if c == "DEU" { "DE".to_owned() } else { c.to_owned() });
            let group = groups.entry((country, norm)).or_default();
            group.0.push((s.section_id.to_owned(), GuidKeep::key(raw)));
            if carry_roles {
                group.1.extend(s.roles.iter().cloned());
            }
        }
        for ((country, name_norm), (members, roles)) in groups {
            let distinct = members.iter().map(|(_, v)| v).collect::<BTreeSet<_>>().len();
            if distinct < 2 {
                continue;
            }
            let refused: Vec<String> = members.iter().filter(|(_, v)| !keep.contains(v)).map(|(s, _)| s.clone()).collect();
            let bind = refused.is_empty().then_some(TwinBind::Keep);
            d.refused.extend(refused.iter().cloned());
            d.groups.push(TwinGroup {
                country,
                name_norm,
                members,
                distinct_values: distinct,
                bind,
                refused,
                office_two_words: false,
                roles: roles.into_iter().collect(),
            });
        }
        d
    }

    /// A miniature notice-reference role mapping (the real one is
    /// `ingest::project::guid_twin::guid_twin_role_ref`): an OPT-300/OPT-301 id-ref names its suffix.
    fn role_ref(field_id: &str, _scheme: Option<&str>, is_ref: bool) -> Option<String> {
        if !is_ref {
            return None;
        }
        field_id.strip_prefix("OPT-300-").or_else(|| field_id.strip_prefix("OPT-301-")).map(str::to_owned)
    }

    fn role_class(role: &str) -> &'static str {
        match role {
            "Procedure-Buyer" => GUID_ROLE_BUYER,
            "Tenderer" => GUID_ROLE_TENDERER,
            r if r.starts_with("Lot-") => GUID_ROLE_REVIEW_INFO,
            _ => GUID_ROLE_OTHER,
        }
    }

    async fn open(name: &str, tender_indexes: bool) -> (crate::Db, turso::Connection, String) {
        let path = format!("/tmp/tender-db-guid-twins-{name}-{}.db", std::process::id());
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{suffix}"));
        }
        let db = crate::Db::open(&path).await.unwrap();
        db.build_organization_indexes().await.unwrap();
        if tender_indexes {
            db.build_tender_indexes().await.unwrap();
        }
        let raw = turso::Builder::new_local(&path).build().await.unwrap();
        let conn = raw.connect().unwrap();
        conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
        (db, conn, path)
    }

    fn cleanup(path: &str) {
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{suffix}"));
        }
    }

    async fn exec(conn: &turso::Connection, sql: &str, params: Vec<Value>) {
        conn.execute(sql, params).await.unwrap();
    }

    /// The Göttingen award in miniature: one notice, the buyer under two GUIDs (ORG-0001 / ORG-0002),
    /// and its identifier-NULL row 2294259.
    async fn seed(conn: &turso::Connection) {
        for (id, identifier) in [(22_759_261i64, "037C1508D9454E739D6E65099A79BEC1"), (22_759_262, "5B1C2D3E4F5A4B6C8D7E8F9A0B1C2D3E")] {
            exec(
                conn,
                "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
                 VALUES (?, 'DE', 'national', ?, 'Landkreis Göttingen', 'landkreis göttingen', 0, 0)",
                vec![Value::Integer(id), Value::Text(identifier.into())],
            )
            .await;
        }
        exec(
            conn,
            "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
             VALUES (2294259, 'DE', NULL, NULL, 'Landkreis Göttingen', 'landkreis göttingen', 1, 0)",
            vec![],
        )
        .await;
        exec(
            conn,
            "INSERT INTO notices (id, source, publication_id, content_hash, profile, fetch_id, member_path, ingested_at,
                                  published_at, parse_state)
             VALUES (26483825, 'doe', 'd', 'h', 'eforms:eforms-de-1.2', 1, 'm', 0, 1717200000, 'parsed')",
            vec![],
        )
        .await;
        exec(
            conn,
            "INSERT INTO tender_versions (tender_id, seq, caused_by_notice_id, published_at, publication_id)
             VALUES (948285, 1, 26483825, 1717200000, 'd')",
            vec![],
        )
        .await;
        for (section, org, raw) in [
            ("ORG-0001", 22_759_261i64, "037c1508-d945-4e73-9d6e-65099a79bec1"),
            ("ORG-0002", 22_759_262, "5b1c2d3e-4f5a-4b6c-8d7e-8f9a0b1c2d3e"),
        ] {
            exec(
                conn,
                "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, country, raw_identifier)
                 VALUES (26483825, ?, ?, 'Landkreis Göttingen', 'DE', ?)",
                vec![Value::Text(section.into()), Value::Integer(org), Value::Text(raw.into())],
            )
            .await;
        }
    }

    fn args<'a>(stop: &'a (dyn Fn() -> bool + Sync), progress: &'a (dyn Fn(u64, &str) + Sync)) -> GuidTwinPlanArgs<'a> {
        GuidTwinPlanArgs {
            is_guid: uuid_v4,
            rule,
            keep: None,
            keep_version: "walk",
            unsafe_names: &[],
            under_review: None,
            rehomed: None,
            role_ref,
            role_class,
            stable_ids: &[254, 255, 8_378],
            stable_mentions: 100,
            extra_unsafe_names: &[],
            samples: 30,
            window: 10_000_000,
            batch: GUID_TWIN_BATCH,
            not_planned: &[254],
            planned: &[(22_759_261, 2_294_259), (22_759_262, 2_294_259)],
            expected: &[],
            exhibits: &[("DE", "landkreis göttingen")],
            stop,
            progress,
        }
    }

    #[test]
    fn keep_is_value_keyed_and_the_sample_key_is_head_elections() {
        let keep: GuidKeep = ["FB197F94757846738A574642AE120532"].into_iter().collect();
        assert!(keep.contains("fb197f94-7578-4673-8a57-4642ae120532"), "dashed lower case, as published");
        assert!(keep.contains("FB197F94757846738A574642AE120532"));
        assert!(!keep.contains("0A9EA48008E44AB6BF12D722D0AD54B6"));
        assert_eq!(guid_twin_sample_key(31_558_903, 530_002).0, ((31_558_903u64 ^ 530_002) * 2_654_435_761) % (1 << 32));
        assert_eq!(guid_twin_sample_key(31_558_903, 530_003).0, ((31_558_903u64 ^ 530_003) * 2_654_435_761) % (1 << 32));
        assert_eq!(GUID_TWIN_SALT, 530_003, "Build 1b: the fresh re-read's salt (A5, R2 F3)");
        assert_eq!(guid_twin_city_fold("  Göttingen, "), "gottingen");
        assert_eq!(guid_twin_city_fold("Frankfurt am Main"), "frankfurt am main");
    }

    /// Issue 530 unit 3: the planner refuses without its indexes and reads nothing; a stop before any
    /// window, batch or chunk returns an empty report marked stopped; with neither it plans Göttingen.
    #[tokio::test]
    async fn the_twin_planner_refuses_without_its_indexes_and_stops_between_chunks() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let (db, conn, path) = open("refuse", false).await;
        seed(&conn).await;
        let stop = || false;
        let progress = |_: u64, _: &str| {};
        let err = db.plan_guid_twins(args(&stop, &progress)).await.expect_err("a deferred tender index is absent");
        assert!(err.to_string().contains("dissolve-guid-twins refused"), "{err}");
        db.build_tender_indexes().await.unwrap();
        let mut calls_total = 0;
        for at in 1..=40usize {
            let calls = AtomicUsize::new(0);
            let stop = || calls.fetch_add(1, Ordering::Relaxed) + 1 >= at;
            let r = db.plan_guid_twins(args(&stop, &progress)).await.unwrap();
            if r.stopped {
                assert_eq!(r.planned.rows, 0, "a stopped plan reports nothing (stop on call {at})");
            } else {
                calls_total = calls.load(Ordering::Relaxed);
                assert_eq!(r.planned.rows, 2, "{r:#?}");
                break;
            }
        }
        assert!(calls_total > 5, "the flag is read before every window, batch and chunk ({calls_total} reads)");
        let r = db.plan_guid_twins(args(&stop, &progress)).await.unwrap();
        assert_eq!(r.planned_row_ids, vec![22_759_261, 22_759_262]);
        assert!(r.bars.pass, "{:#?}", r.bars);
        assert_eq!(r.largest_target.as_ref().and_then(|t| t.target_id), Some(2_294_259));
        drop(conn);
        cleanup(&path);
    }

    /// Issue 530 unit 3: every chunked read the planner sends, at the width it sends, seeks its index:
    /// a `SCAN` would be a whole-table read per chunk on prod.
    #[tokio::test]
    async fn the_twin_planner_reads_by_index_at_the_chunk_width() {
        let (db, conn, path) = open("eqp", true).await;
        drop(conn);
        let reader = db.reader().await.unwrap();
        let ids: Vec<Value> = (1..=IN_CHUNK as i64).map(Value::Integer).collect();
        let names: Vec<Value> = (1..=IN_CHUNK).map(|i| Value::Text(format!("name {i}"))).collect();
        let mut cities = ids.clone();
        cities.push(Value::Text(GUID_TWIN_CITY_FIELD.into()));
        for (what, sql, params, needle) in [
            ("row mentions", guid_twin_row_mentions_sql(IN_CHUNK), ids.clone(), "organization_mentions_org"),
            ("notices", guid_twin_notices_sql(IN_CHUNK), ids.clone(), "SEARCH notices"),
            ("versions", guid_twin_versions_sql(IN_CHUNK), ids.clone(), "tender_versions_notice"),
            ("notice mentions", guid_twin_notice_mentions_sql(IN_CHUNK), ids.clone(), "SEARCH organization_mentions"),
            (
                "merged window",
                GUID_TWIN_MERGED_SQL.to_owned(),
                vec![Value::Integer(0), Value::Integer(GUID_TWIN_WINDOW)],
                "SEARCH organization_merged_identifiers",
            ),
            ("orgs", guid_twin_orgs_sql(IN_CHUNK), ids.clone(), "SEARCH organizations"),
            ("survivors", guid_twin_survivors_sql(IN_CHUNK), ids.clone(), "organization_merged_identifiers_org"),
            ("targets", guid_twin_targets_sql(IN_CHUNK), names, "organizations_name_country"),
            ("target count", GUID_TWIN_TARGET_COUNT_SQL.to_owned(), vec![Value::Integer(1)], "organization_mentions_org"),
            (
                "target newest",
                GUID_TWIN_TARGET_NEWEST_SQL.to_owned(),
                vec![Value::Integer(1), Value::Integer(GUID_TWIN_TARGET_NEWEST)],
                "organization_mentions_org",
            ),
            ("cities", guid_twin_cities_sql(IN_CHUNK), cities, "SEARCH notice_texts"),
            ("role refs", guid_twin_role_refs_sql(IN_CHUNK), ids.clone(), "sqlite_autoindex_notice_ids_1"),
            ("parties", guid_twin_parties_sql(IN_CHUNK), ids.clone(), "tender_version_parties_mention_key"),
            ("bid parties", guid_twin_bid_parties_sql(IN_CHUNK), ids.clone(), "tender_version_bid_parties_mention_key"),
            ("result winners", guid_twin_result_winners_sql(IN_CHUNK), ids.clone(), "SEARCH tender_version_result_winners"),
        ] {
            let mut rows = reader.query(&format!("EXPLAIN QUERY PLAN {sql}"), params).await.unwrap();
            let mut plan = String::new();
            while let Some(r) = rows.next().await.unwrap() {
                plan.push_str(&r.get_value(3).ok().and_then(|v| v.as_text().cloned()).unwrap_or_default());
                plan.push('\n');
            }
            println!("PLAN {what}:\n{plan}");
            assert!(plan.lines().any(|l| l.contains("SEARCH") && l.contains(needle)), "{what} must seek `{needle}`:\n{plan}");
            assert!(!plan.lines().any(|l| l.trim_start().starts_with("SCAN ")), "{what} scans:\n{plan}");
            // No sorter either: one would read a large target's whole range (or a chunk's whole
            // result) before the LIMIT. turso explains DISTINCT as a hash table, not a sorter.
            let upper = plan.to_uppercase();
            assert!(!upper.contains("SORTER") && !upper.contains("TEMP B-TREE"), "{what} sorts:\n{plan}");
        }
        drop(reader);
        cleanup(&path);
    }

    // ---- R2's sample selection (A5, A6, A7, R2 (1)) ----

    /// R2 reads a flagged class whole up to 45; past it, the top 30 by rank (ties by id), then the 15
    /// of the rest with the smallest salted key at [`GUID_TWIN_SALT`].
    #[test]
    fn r2_reads_a_class_whole_to_45_then_the_top_30_and_15_salted() {
        // Ranks `id % 7`: 7 ids each of ranks 6, 5, 4 and 3 fill 28 places; ranks 2 ties break by id.
        let items: Vec<(u64, i64)> = (1..=50i64).map(|id| ((id % 7) as u64, id)).collect();
        let picked = r2_pick(&items, |&(r, id)| (r, id), |&(_, id)| id);
        assert_eq!(picked.len(), 45);
        assert_eq!(&picked[..7], &[(6, 6), (6, 13), (6, 20), (6, 27), (6, 34), (6, 41), (6, 48)]);
        assert_eq!(&picked[28..30], &[(2, 2), (2, 9)], "rank 2 ties break by id");
        let top: BTreeSet<i64> = picked[..30].iter().map(|p| p.1).collect();
        let mut rest: Vec<i64> = items.iter().map(|p| p.1).filter(|id| !top.contains(id)).collect();
        assert_eq!(rest.len(), 20);
        rest.sort_by_key(|&id| guid_twin_sample_key(id, GUID_TWIN_SALT));
        assert_eq!(picked[30..].iter().map(|p| p.1).collect::<Vec<_>>(), rest[..15].to_vec(), "the 15 smallest salted keys");
        // 45 or fewer come back whole, in rank order.
        let whole = r2_pick(&items[..45], |&(r, id)| (r, id), |&(_, id)| id);
        assert_eq!(whole.len(), 45);
        assert!(whole.windows(2).all(|w| (std::cmp::Reverse(w[0].0), w[0].1) < (std::cmp::Reverse(w[1].0), w[1].1)));
    }

    /// R2's random 60 (A5): buyer 25, review_or_info 25, other or tenderer 10, each block in ascending
    /// salted-key order at [`GUID_TWIN_SALT`], and nothing past a stratum's cap.
    #[test]
    fn r2s_random_strata_take_25_25_10_in_salted_order() {
        let mut rows: Vec<(&str, i64)> = Vec::new();
        for i in 0..30 {
            rows.push((GUID_ROLE_REVIEW_INFO, 2_000 + i));
            rows.push((GUID_ROLE_BUYER, 1_000 + i));
        }
        for i in 0..4 {
            rows.push((GUID_ROLE_TENDERER, 3_000 + i));
            rows.push((GUID_ROLE_OTHER, 4_000 + i));
            rows.push((GUID_ROLE_NONE, 5_000 + i));
        }
        let (picked, skipped) = r2_random_pick(&rows);
        assert_eq!(skipped, 0, "no row R2 read");
        let picked: Vec<i64> = picked.into_iter().map(|i| rows[i].1).collect();
        let block = |ids: &mut dyn Iterator<Item = i64>, n: usize| {
            let mut v: Vec<i64> = ids.collect();
            v.sort_by_key(|&id| guid_twin_sample_key(id, GUID_TWIN_SALT));
            v.truncate(n);
            v
        };
        let buyers = block(&mut (1_000..1_030), 25);
        let review = block(&mut (2_000..2_030), 25);
        let other = block(&mut (3_000..3_004).chain(4_000..4_004).chain(5_000..5_004), 10);
        assert_eq!(picked.len(), 60);
        assert_eq!(picked[..25].to_vec(), buyers, "buyer first");
        assert_eq!(picked[25..50].to_vec(), review, "review_or_info second");
        assert_eq!(picked[50..].to_vec(), other, "other, tenderer and no-role rows share the last 10");
    }

    /// R2's read list: the 60 rows of r2-result.json's `r2_random`, distinct and sorted (the lookup is
    /// a binary search).
    #[test]
    fn r2s_read_list_holds_60_distinct_sorted_ids() {
        assert_eq!(GUID_TWIN_R2_READ.len(), 60);
        assert!(GUID_TWIN_R2_READ.windows(2).all(|w| w[0] < w[1]), "sorted and distinct");
        for id in GUID_TWIN_R2_READ {
            assert!(guid_twin_r2_read(id), "{id}");
        }
        assert!(!guid_twin_r2_read(22_759_261), "the Göttingen control was not in R2's random 60");
        assert!(guid_twin_r2_read(22_824_042) && guid_twin_r2_read(23_761_350), "the first item read, the largest id");
    }

    /// The fresh re-read (R2 F3): a row R2 already read is skipped even where its salted key ranks it
    /// inside its stratum's 25, and the next rows in salted order take their places; in a stratum under
    /// its cap a read row is simply absent. Built at the real salt over R2's real read ids, so the read
    /// rows inside the top 25 are found by the key, not hand-picked.
    #[test]
    fn r2s_random_draw_skips_the_rows_r2_read_and_fills_from_the_next_in_salted_order() {
        // 59 buyer rows: the 19 read ids below 22_839_000 and 40 fresh ids.
        let read: Vec<i64> = GUID_TWIN_R2_READ.iter().copied().filter(|&id| id < 22_839_000).collect();
        assert_eq!(read.len(), 19);
        let fresh: Vec<i64> = (1..=40).map(|k| 22_700_000 + k).collect();
        let mut rows: Vec<(&str, i64)> = read.iter().chain(&fresh).map(|&id| (GUID_ROLE_BUYER, id)).collect();
        // A review_or_info stratum of 3 (under its cap), one of them read.
        rows.extend([(GUID_ROLE_REVIEW_INFO, 22_870_001), (GUID_ROLE_REVIEW_INFO, 22_879_585), (GUID_ROLE_REVIEW_INFO, 22_870_002)]);
        let (picked, skipped) = r2_random_pick(&rows);
        assert_eq!(skipped, 20, "19 read buyers and 1 read review row");
        let picked: Vec<i64> = picked.into_iter().map(|i| rows[i].1).collect();
        assert_eq!(picked.len(), 25 + 2);
        // Without the skip the buyer block would be the 25 smallest keys of all 59 rows.
        let mut all: Vec<i64> = read.iter().chain(&fresh).copied().collect();
        all.sort_by_key(|&id| guid_twin_sample_key(id, GUID_TWIN_SALT));
        let read_in_top: Vec<i64> = all[..25].iter().copied().filter(|id| guid_twin_r2_read(*id)).collect();
        assert!(!read_in_top.is_empty(), "the fixture puts read rows inside the top 25 at the salt");
        // With it: the 25 smallest keys among the unread rows.
        let mut unread: Vec<i64> = fresh.clone();
        unread.sort_by_key(|&id| guid_twin_sample_key(id, GUID_TWIN_SALT));
        assert_eq!(picked[..25].to_vec(), unread[..25].to_vec(), "read rows skipped, the next in salted order fill the stratum");
        for id in &read_in_top {
            assert!(!picked.contains(id), "{id}: read at 530002, ranked inside the top 25 at 530003, skipped");
        }
        // The rows just past the top 25 in salted order take the read rows' places, one each.
        let past: Vec<i64> = all[25..].iter().copied().filter(|id| !guid_twin_r2_read(*id)).take(read_in_top.len()).collect();
        assert_eq!(past.len(), read_in_top.len());
        for id in &past {
            assert!(picked[..25].contains(id), "{id}: the next in salted order takes a read row's place");
        }
        assert!(!picked.contains(&all[25..].iter().copied().filter(|id| !guid_twin_r2_read(*id)).nth(read_in_top.len()).unwrap()));
        let mut review = vec![22_870_001, 22_870_002];
        review.sort_by_key(|&id| guid_twin_sample_key(id, GUID_TWIN_SALT));
        assert_eq!(picked[25..].to_vec(), review, "the read review row is absent");
    }

    /// R2's top targets: 30 by planned mentions (ties by `sample_id`), then the 20 by planned rows not
    /// among them — missing keys (every `target_id` `None`) told apart by key, so two of them both join.
    #[test]
    fn r2s_top_targets_add_the_top_by_rows_and_keep_missing_keys_apart() {
        let t = |name: &str, id: Option<i64>, sample_id: i64, mentions: u64, rows: u64| GuidTwinTargetSample {
            target_id: id,
            sample_id,
            name_norm: name.to_owned(),
            country: Some("DE".to_owned()),
            planned_mentions: mentions,
            planned_rows: rows,
            ..Default::default()
        };
        let mut targets: Vec<GuidTwinTargetSample> =
            (0..33i64).map(|i| t(&format!("found {i}"), Some(100 + i), 100 + i, 100 - i as u64, 1)).collect();
        targets.push(t("found tie", Some(99), 99, 100, 1));
        targets.push(t("missing a", None, 5_001, 10, 9));
        targets.push(t("missing b", None, 5_002, 10, 8));
        let picked = r2_top_targets(targets);
        assert_eq!(picked.len(), 32, "30 by mentions, and both missing keys by rows");
        assert_eq!((picked[0].sample_id, picked[1].sample_id), (99, 100), "a mention tie breaks by sample_id");
        assert_eq!(picked[29].name_norm, "found 28");
        assert_eq!((picked[30].name_norm.as_str(), picked[31].name_norm.as_str()), ("missing a", "missing b"));
        let keys: BTreeSet<(&str, Option<&str>)> = picked.iter().map(|p| (p.name_norm.as_str(), p.country.as_deref())).collect();
        assert_eq!(keys.len(), picked.len(), "no key twice");
    }

    // ---- The bars driven to FAIL, the merged KEEP arm, the off-GUID kinds, A11 ----

    const V1: &str = "11111111-1111-4111-8111-111111111111";
    const V2: &str = "22222222-2222-4222-8222-222222222222";
    const V3: &str = "33333333-3333-4333-8333-333333333333";
    const V4: &str = "44444444-4444-4444-8444-444444444444";
    const V5: &str = "55555555-5555-4555-9555-555555555555";
    const V6: &str = "66666666-6666-4666-a666-666666666666";
    const V7: &str = "77777777-7777-4777-b777-777777777777";
    const V8: &str = "88888888-8888-4888-8888-888888888888";
    const V9: &str = "99999999-9999-4999-9999-999999999999";
    const V10: &str = "aaaaaaaa-aaaa-4aaa-aaaa-aaaaaaaaaaaa";
    const V11: &str = "bbbbbbbb-bbbb-4bbb-bbbb-bbbbbbbbbbbb";

    fn opt(s: Option<&str>) -> Value {
        s.map_or(Value::Null, |s| Value::Text(s.into()))
    }

    async fn org_row(conn: &turso::Connection, id: i64, kind: Option<&str>, identifier: Option<&str>, name: &str, country: &str) {
        exec(
            conn,
            "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, 0)",
            vec![
                Value::Integer(id),
                Value::Text(country.into()),
                opt(kind),
                identifier.map_or(Value::Null, |i| Value::Text(GuidKeep::key(i))),
                Value::Text(name.into()),
                Value::Text(org_name_norm(name)),
                Value::Integer(i64::from(identifier.is_none())),
            ],
        )
        .await;
    }

    /// A notice; with `tender`, the version it causes.
    async fn notice_row(conn: &turso::Connection, id: i64, tender: Option<i64>) {
        exec(
            conn,
            "INSERT INTO notices (id, source, publication_id, content_hash, profile, fetch_id, member_path, ingested_at,
                                  published_at, parse_state)
             VALUES (?, 'doe', ?, 'h', 'eforms:eforms-de-1.2', 1, 'm', 0, 1717200000, 'parsed')",
            vec![Value::Integer(id), Value::Text(format!("{id}"))],
        )
        .await;
        if let Some(tender) = tender {
            exec(
                conn,
                "INSERT INTO tender_versions (tender_id, seq, caused_by_notice_id, published_at, publication_id)
                 VALUES (?, 1, ?, 1717200000, ?)",
                vec![Value::Integer(tender), Value::Integer(id), Value::Text(format!("{id}"))],
            )
            .await;
        }
    }

    async fn mention_row(conn: &turso::Connection, notice: i64, section: &str, org: i64, name: &str, country: &str, raw: Option<&str>) {
        exec(
            conn,
            "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, country, raw_identifier)
             VALUES (?, ?, ?, ?, ?, ?)",
            vec![
                Value::Integer(notice),
                Value::Text(section.into()),
                Value::Integer(org),
                Value::Text(name.into()),
                Value::Text(country.into()),
                opt(raw),
            ],
        )
        .await;
    }

    async fn merged_row(conn: &turso::Connection, identifier: &str, org: i64) {
        exec(
            conn,
            "INSERT INTO organization_merged_identifiers (identifier, identifier_kind, country, org_id, loser, rule)
             VALUES (?, 'national', 'DE', ?, ?, 'test')",
            vec![Value::Text(GuidKeep::key(identifier)), Value::Integer(org), Value::Integer(org + 1_000_000)],
        )
        .await;
    }

    /// R1 fails when it should: a planned row whose published value is not its row's (so the stable
    /// row's KEEP value does not spare it) trips the stable and KEEP tripwires; a raised knob cannot hide
    /// a 150-mention row from the fixed 100-mention tripwire, and such a run never passes; a row spanning
    /// no Tender is reported but is no bar (the decision's tripwires are >= 2 Tenders, >= 100 mentions
    /// and KEEP).
    #[tokio::test]
    async fn the_r1_tripwires_fire_and_fail_the_bars() {
        let (db, conn, path) = open("trip", true).await;
        seed(&conn).await;
        let stop = || false;
        let progress = |_: u64, _: &str| {};
        // No Tender: a second twin notice that causes no version.
        notice_row(&conn, 26_483_826, None).await;
        org_row(&conn, 22_759_263, Some("national"), Some(V3), "Landkreis Göttingen", "DE").await;
        org_row(&conn, 22_759_264, Some("national"), Some(V4), "Landkreis Göttingen", "DE").await;
        mention_row(&conn, 26_483_826, "ORG-0001", 22_759_263, "Landkreis Göttingen", "DE", Some(V3)).await;
        mention_row(&conn, 26_483_826, "ORG-0002", 22_759_264, "Landkreis Göttingen", "DE", Some(V4)).await;
        let r = db.plan_guid_twins(args(&stop, &progress)).await.unwrap();
        assert_eq!(r.planned_row_ids, vec![22_759_261, 22_759_262, 22_759_263, 22_759_264]);
        assert_eq!(r.bars.tripwire_no_tender, 2);
        assert_eq!(r.bars.tripwire_rows, vec![22_759_263, 22_759_264]);
        assert_eq!(r.bars.tripwire_stable + r.bars.tripwire_keep + r.bars.tripwire_multi_tender, 0);
        assert!(r.bars.stable_mentions_is_decision);
        assert!(r.bars.pass, "no Tender is reported, not a bar: {:#?}", r.bars);

        // A published value that is not the row's: 22759261 is stable (by id) and its identifier is in
        // KEEP, but the rule reads the published value, refuses it, and the row is planned.
        let other = Value::Text(V5.into());
        exec(&conn, "UPDATE organization_mentions SET raw_identifier = ? WHERE notice_id = 26483825 AND section_id = 'ORG-0001'", vec![other])
            .await;
        let r = db.plan_guid_twins(GuidTwinPlanArgs { stable_ids: &[22_759_261], ..args(&stop, &progress) }).await.unwrap();
        assert!(r.planned_row_ids.contains(&22_759_261));
        assert_eq!((r.bars.tripwire_stable, r.bars.tripwire_keep), (1, 1));
        assert_eq!(r.bars.tripwire_rows, vec![22_759_261, 22_759_263, 22_759_264]);
        assert!(!r.bars.pass);
        let back = Value::Text("037c1508-d945-4e73-9d6e-65099a79bec1".into());
        exec(&conn, "UPDATE organization_mentions SET raw_identifier = ? WHERE notice_id = 26483825 AND section_id = 'ORG-0001'", vec![back])
            .await;

        // The knob: 150 mentions of 22759261, one notice, one Tender.
        for k in 0..149 {
            let section = format!("ORG-1{k:03}");
            mention_row(&conn, 26_483_825, &section, 22_759_261, "Landkreis Göttingen", "DE", Some("037c1508-d945-4e73-9d6e-65099a79bec1"))
                .await;
        }
        let raised = GuidTwinPlanArgs { stable_ids: &[], stable_mentions: 500, ..args(&stop, &progress) };
        let r = db.plan_guid_twins(raised).await.unwrap();
        assert!(r.planned_row_ids.contains(&22_759_261), "not stable at 500, so not in KEEP: refused");
        assert_eq!(r.bars.tripwire_stable, 1, "R1's 100 reads it whatever the knob");
        assert!(r.bars.tripwire_rows.contains(&22_759_261));
        assert!(!r.bars.stable_mentions_is_decision);
        assert!(!r.bars.pass);
        let decided = GuidTwinPlanArgs { stable_ids: &[], ..args(&stop, &progress) };
        let r = db.plan_guid_twins(decided).await.unwrap();
        assert!(!r.planned_row_ids.contains(&22_759_261), "stable at 100: in KEEP, spared");
        assert_eq!(r.bars.tripwire_stable, 0);
        assert!(r.bars.stable_mentions_is_decision);
        assert_eq!(r.bars.planned_failed, vec![22_759_261], "a planned control kept is a miss");
        assert!(!r.bars.pass);
        drop(conn);
        cleanup(&path);
    }

    /// A3 ii and A3 i: a merged v4 value joins KEEP (`merged`, no mentions or Tenders of its own; joined
    /// with `stable` on a GUID row that also holds one) and spares the section publishing it; and the
    /// refused mentions bound off GUID rows are reported by every bound kind.
    #[tokio::test]
    async fn the_merged_keep_arm_and_every_off_guid_kind() {
        let (db, conn, path) = open("merged", true).await;
        // 254: a stable GUID row whose value is also merged.
        org_row(&conn, 254, Some("national"), Some(V11), "DB Netz", "DE").await;
        merged_row(&conn, V11, 254).await;
        notice_row(&conn, 1_001, Some(7_001)).await;
        mention_row(&conn, 1_001, "ORG-0001", 254, "DB Netz", "DE", Some(V11)).await;
        // A merge survivor holding V2: the section publishing V2 is spared, its fresh partner refused.
        org_row(&conn, 500, Some("vat"), Some("DE111111111"), "Muster Werke", "DE").await;
        merged_row(&conn, V2, 500).await;
        org_row(&conn, 600, Some("national"), Some(V1), "Muster Werke", "DE").await;
        notice_row(&conn, 1_002, Some(7_002)).await;
        mention_row(&conn, 1_002, "ORG-0001", 600, "Muster Werke", "DE", Some(V1)).await;
        mention_row(&conn, 1_002, "ORG-0002", 500, "Muster Werke", "DE", Some(V2)).await;
        // merge_survivor: a survivor whose only merged identifier is not v4 (so not in KEEP).
        org_row(&conn, 700, Some("vat"), Some("DE123456789"), "Blau Werke", "DE").await;
        exec(
            &conn,
            "INSERT INTO organization_merged_identifiers (identifier, identifier_kind, country, org_id, loser, rule)
             VALUES ('DE987654321', 'vat', 'DE', 700, 1700, 'test')",
            vec![],
        )
        .await;
        org_row(&conn, 800, Some("national"), Some(V4), "Blau Werke", "DE").await;
        notice_row(&conn, 1_003, Some(7_003)).await;
        mention_row(&conn, 1_003, "ORG-0001", 700, "Blau Werke", "DE", Some(V3)).await;
        mention_row(&conn, 1_003, "ORG-0002", 800, "Blau Werke", "DE", Some(V4)).await;
        // other_identified: a VAT row.
        org_row(&conn, 900, Some("vat"), Some("DE308703297"), "Grün Werke", "DE").await;
        org_row(&conn, 1_000, Some("national"), Some(V6), "Grün Werke", "DE").await;
        notice_row(&conn, 1_004, Some(7_004)).await;
        mention_row(&conn, 1_004, "ORG-0001", 900, "Grün Werke", "DE", Some(V5)).await;
        mention_row(&conn, 1_004, "ORG-0002", 1_000, "Grün Werke", "DE", Some(V6)).await;
        // identifier_null.
        org_row(&conn, 1_100, None, None, "Rot Werke", "DE").await;
        org_row(&conn, 1_200, Some("national"), Some(V8), "Rot Werke", "DE").await;
        notice_row(&conn, 1_005, Some(7_005)).await;
        mention_row(&conn, 1_005, "ORG-0001", 1_100, "Rot Werke", "DE", Some(V7)).await;
        mention_row(&conn, 1_005, "ORG-0002", 1_200, "Rot Werke", "DE", Some(V8)).await;
        // absent: bound to an org id with no row.
        org_row(&conn, 1_300, Some("national"), Some(V10), "Gelb Werke", "DE").await;
        notice_row(&conn, 1_006, Some(7_006)).await;
        mention_row(&conn, 1_006, "ORG-0001", 99_999, "Gelb Werke", "DE", Some(V9)).await;
        mention_row(&conn, 1_006, "ORG-0002", 1_300, "Gelb Werke", "DE", Some(V10)).await;

        let stop = || false;
        let progress = |_: u64, _: &str| {};
        let r = db
            .plan_guid_twins(GuidTwinPlanArgs { planned: &[], not_planned: &[254, 500], exhibits: &[], ..args(&stop, &progress) })
            .await
            .unwrap();
        let keep: Vec<(&str, i64, Option<u64>, Option<u64>, &str)> = r
            .keep_candidates
            .iter()
            .map(|c| (c.value.as_str(), c.org_id, c.mentions, c.tenders, c.reason.as_str()))
            .collect();
        assert_eq!(
            keep,
            vec![
                ("22222222222242228222222222222222", 500, None, None, "merged"),
                ("BBBBBBBBBBBB4BBBBBBBBBBBBBBBBBBB", 254, Some(1), Some(1), "stable+merged"),
            ],
            "the non-v4 merged identifier is no KEEP value"
        );
        assert_eq!(r.keep_by_reason.get("merged"), Some(&1));
        assert_eq!(r.keep_by_reason.get("stable+merged"), Some(&1));
        assert_eq!(r.planned_row_ids, vec![600, 800, 1_000, 1_200, 1_300]);
        assert_eq!(r.keep_partner_mentions, 1, "the survivor's V2 section beside the refused V1");
        assert_eq!((r.keep_spared.rows, r.keep_spared.mentions), (0, 1), "spared on a row that is no GUID row");
        assert!(!r.off_guid.samples.iter().any(|o| o.notice_id == 1_002), "the merged value's section is not refused");
        let kinds: Vec<(&str, u64)> = r.off_guid.by_kind.iter().map(|(k, n)| (*k, *n)).collect();
        assert_eq!(kinds, vec![("absent", 1), ("identifier_null", 1), ("merge_survivor", 1), ("other_identified", 1)]);
        assert!(r.bars.not_planned_failed.is_empty(), "{:?}", r.bars.not_planned_failed);
        drop(conn);
        cleanup(&path);
    }

    /// A11's bar is computed from the stored mention's country, not from the key it was looked up by: a
    /// rule that keys an RE mention under RE (the miniature rule here does; the real one maps it to FR)
    /// lands on the RE identifier-NULL row, and the bar counts both mentions; a mention whose country is
    /// no 2-letter code is counted unchecked.
    #[tokio::test]
    async fn the_target_country_bar_reads_the_mentions_register_country() {
        let (db, conn, path) = open("country", true).await;
        const SDIS: &str = "SDIS de La Réunion";
        org_row(&conn, 60, None, None, SDIS, "RE").await;
        org_row(&conn, 61, None, None, SDIS, "FR").await;
        org_row(&conn, 62, Some("national"), Some(V1), SDIS, "FR").await;
        org_row(&conn, 63, Some("national"), Some(V2), SDIS, "FR").await;
        notice_row(&conn, 70, Some(80)).await;
        mention_row(&conn, 70, "ORG-0001", 62, SDIS, "RE", Some(V1)).await;
        mention_row(&conn, 70, "ORG-0002", 63, SDIS, "RE", Some(V2)).await;
        org_row(&conn, 64, Some("national"), Some(V3), "Muster Stadtwerke Nord", "DE").await;
        org_row(&conn, 65, Some("national"), Some(V4), "Muster Stadtwerke Nord", "DE").await;
        notice_row(&conn, 71, Some(81)).await;
        mention_row(&conn, 71, "ORG-0001", 64, "Muster Stadtwerke Nord", "DEU", Some(V3)).await;
        mention_row(&conn, 71, "ORG-0002", 65, "Muster Stadtwerke Nord", "DEU", Some(V4)).await;
        let stop = || false;
        let progress = |_: u64, _: &str| {};
        let r = db
            .plan_guid_twins(GuidTwinPlanArgs { planned: &[], not_planned: &[], exhibits: &[], ..args(&stop, &progress) })
            .await
            .unwrap();
        assert_eq!(r.planned_row_ids, vec![62, 63, 64, 65]);
        let sdis = r.planned_samples.iter().find(|s| s.org_id == 62).unwrap();
        assert_eq!(sdis.targets, vec![Some(60)], "keyed RE by the miniature rule: the RE row");
        assert_eq!(r.bars.target_country_mismatch, 2, "register_jurisdiction(RE) is FR, the key and the row say RE");
        assert_eq!(r.bars.target_country_unchecked, 2, "DEU is no 2-letter code");
        assert!(!r.bars.pass);
        drop(conn);
        cleanup(&path);
    }

    // ---- Build 1b: A7's role guard (notice-reference roles) ----

    fn roles(list: &[&str]) -> Vec<String> {
        list.iter().map(|r| (*r).to_owned()).collect()
    }

    /// A7 as written: Lot-ReviewOrg beside a buyer-side role (Procedure-Buyer, Procedure-SProvider,
    /// Lot-AddInfo, Lot-TenderReceipt — read after the scope, so a `Part-` variant is alike), or a
    /// tenderer beside a role outside the tenderer class; a buyer-side group without a review body, a
    /// review body beside review information, and an all-tenderer group are coherent.
    #[test]
    fn the_role_mix_reads_roles_after_their_scope() {
        let mix = |list: &[&str]| guid_twin_role_mix(&roles(list));
        for buyer_side in ["Procedure-Buyer", "Procedure-SProvider", "Lot-AddInfo", "Lot-TenderReceipt", "Part-AddInfo", "Part-TenderReceipt"] {
            assert_eq!(mix(&[buyer_side, "Lot-ReviewOrg"]), Some("buyer_review_org"), "{buyer_side}");
            assert_eq!(mix(&[buyer_side, "Part-ReviewOrg"]), Some("buyer_review_org"), "{buyer_side}, Part- scope");
        }
        assert_eq!(mix(&["Lot-AddInfo", "Lot-ReviewOrg", "Procedure-Buyer"]), Some("buyer_review_org"), "the AOK group");
        assert_eq!(mix(&["Lot-AddInfo", "Lot-TenderReceipt", "Procedure-Buyer"]), None, "no review body: Göttingen-shaped");
        assert_eq!(mix(&["Lot-ReviewInfo", "Lot-ReviewOrg"]), None, "a chamber in both review slots");
        assert_eq!(mix(&["Lot-Mediator", "Lot-ReviewOrg"]), None, "the mediator is no buyer-side role");
        assert_eq!(mix(&["Lot-ReviewOrg"]), None);
        assert_eq!(mix(&[]), None);
        assert_eq!(mix(&["Lot-AddInfo", "Tenderer"]), Some("tenderer_mixed"));
        assert_eq!(mix(&["Procedure-Buyer", "Tenderer-SubCont"]), Some("tenderer_mixed"));
        assert_eq!(mix(&["Tenderer", "Tenderer-MainCont", "Tenderer-SubCont"]), None, "one class");
        assert_eq!(guid_twin_role_after_scope("Part-ReviewOrg"), "ReviewOrg");
        assert_eq!(guid_twin_role_after_scope("Procedure-Buyer"), "Buyer");
        assert_eq!(guid_twin_role_after_scope("Tenderer-SubCont"), "Tenderer-SubCont", "Tenderer is no scope");
        assert_eq!(guid_twin_role_after_scope("LotResult-Paying"), "LotResult-Paying");
    }

    /// The fold gives each referenced (target) section its roles, sorted and distinct, and drops every
    /// value the mapping names no role for.
    #[test]
    fn the_role_fold_keys_roles_by_their_target_section() {
        let refs = [
            ("OPT-300-Procedure-Buyer", None, "ORG-0001", true),
            ("OPT-301-Lot-AddInfo", None, "ORG-0002", true),
            ("OPT-301-Lot-ReviewOrg", Some("organization"), "ORG-0003", true),
            ("OPT-301-Lot-ReviewOrg", Some("organization"), "ORG-0003", true),
            ("OPT-301-Lot-TenderReceipt", None, "ORG-0001", true),
            ("BT-501-Organization-Company", None, "617a9b4a-867f-4946-9210-0658d0a6ca70", false),
            ("OPT-301-Lot-Mediator", None, "ORG-0009", false),
        ];
        let folded = guid_twin_fold_roles(refs, role_ref);
        let want: GuidTwinSectionRoles = [
            ("ORG-0001".to_owned(), roles(&["Lot-TenderReceipt", "Procedure-Buyer"])),
            ("ORG-0002".to_owned(), roles(&["Lot-AddInfo"])),
            ("ORG-0003".to_owned(), roles(&["Lot-ReviewOrg"])),
        ]
        .into_iter()
        .collect();
        assert_eq!(folded, want);
    }

    async fn ref_row(conn: &turso::Connection, notice: i64, section: &str, field: &str, ordinal: i64, target: &str, is_ref: bool) {
        exec(
            conn,
            "INSERT INTO notice_ids (notice_id, section_id, field_id, ordinal, scheme, value, is_ref) VALUES (?, ?, ?, ?, NULL, ?, ?)",
            vec![
                Value::Integer(notice),
                Value::Text(section.into()),
                Value::Text(field.into()),
                Value::Integer(ordinal),
                Value::Text(target.into()),
                Value::Integer(i64::from(is_ref)),
            ],
        )
        .await;
    }

    /// Build 1b: the planner reads the role references of the copies holding a twin group, and only
    /// those (a copy naming the body once is not read), counting the rows and the (section, role) pairs;
    /// its stored-roles read gives the map the fold gives. Its role self-check fires when the injected
    /// rule ignores A7: the miniature rule here refuses a buyer + Lot-ReviewOrg twin, and the tripwire
    /// lists it and fails R1 — while a buyer + Lot-AddInfo twin (coherent) passes it. The tripwire reads
    /// the planner's OWN reference read, so a rule that loses the roles is caught too (the tripwire still
    /// fires, and `role_ref_disagree` counts the rule against the planner even on a coherent group).
    #[tokio::test]
    async fn the_role_refs_are_read_for_twin_copies_and_the_role_tripwire_fires() {
        let (db, conn, path) = open("roles", true).await;
        seed(&conn).await;
        // The Göttingen copy's references: buyer and AddInfo (coherent).
        ref_row(&conn, 26_483_825, "ND-ContractingParty#0", "OPT-300-Procedure-Buyer", 0, "ORG-0001", true).await;
        ref_row(&conn, 26_483_825, "LOT-0001", "OPT-301-Lot-AddInfo", 0, "ORG-0002", true).await;
        ref_row(&conn, 26_483_825, "ORG-0001", "OPT-200-Organization-Company", 0, "ORG-0001", false).await;
        // A copy naming a body once: its references are never read.
        org_row(&conn, 700, Some("national"), Some(V1), "Blau Werke", "DE").await;
        notice_row(&conn, 2_001, Some(8_001)).await;
        mention_row(&conn, 2_001, "ORG-0001", 700, "Blau Werke", "DE", Some(V1)).await;
        ref_row(&conn, 2_001, "ND-ContractingParty#0", "OPT-300-Procedure-Buyer", 0, "ORG-0001", true).await;
        let stop = || false;
        let progress = |_: u64, _: &str| {};
        let r = db.plan_guid_twins(args(&stop, &progress)).await.unwrap();
        assert_eq!(r.planned_row_ids, vec![22_759_261, 22_759_262]);
        assert_eq!((r.role_ref_notices, r.role_ref_rows_read, r.role_refs), (1, 2, 2), "the twin copy alone, its two id-refs");
        // OQ1's exposure: the copy's REFUSED ORG-0001 is its Procedure-Buyer (Blau Werke's buyer section is
        // not refused). With ORG-0001's value on KEEP only the Lot-AddInfo section is refused: 0.
        assert_eq!(r.refused_buyer_copies, 1);
        let kept: GuidKeep = ["037C1508D9454E739D6E65099A79BEC1"].into_iter().collect();
        let r0 = db
            .plan_guid_twins(GuidTwinPlanArgs { keep: Some(&kept), keep_version: "keep-test", ..args(&stop, &progress) })
            .await
            .unwrap();
        assert_eq!((r0.refused_buyer_copies, r0.planned_row_ids.clone()), (0, vec![22_759_262]), "only the AddInfo value is refused");
        assert_eq!(r.bars.tripwire_role_mix, 0);
        assert!(r.bars.pass, "{:#?}", r.bars);
        let stored = db.guid_twin_stored_roles(&[26_483_825, 2_001, 9_999], role_ref).await.unwrap();
        assert_eq!(stored.len(), 2, "a notice with no reference is absent");
        assert_eq!(stored[&26_483_825]["ORG-0001"], roles(&["Procedure-Buyer"]));
        assert_eq!(stored[&26_483_825]["ORG-0002"], roles(&["Lot-AddInfo"]));

        // ORG-0002 is also the review body: buyer + Lot-ReviewOrg, refused by the miniature rule.
        ref_row(&conn, 26_483_825, "LOT-0001", "OPT-301-Lot-ReviewOrg", 0, "ORG-0002", true).await;
        let r = db.plan_guid_twins(args(&stop, &progress)).await.unwrap();
        assert_eq!(r.planned_row_ids, vec![22_759_261, 22_759_262], "the miniature rule applies no role guard");
        assert_eq!(r.bars.tripwire_role_mix, 1);
        let listed = &r.bars.role_mix_refused[0];
        assert_eq!((listed.notice_id, listed.ref_roles.clone()), (26_483_825, roles(&["Lot-AddInfo", "Lot-ReviewOrg", "Procedure-Buyer"])));
        assert_eq!(r.bars.role_ref_disagree, 0, "the miniature rule returns the roles it was given");
        assert!(!r.bars.pass, "a refused role-incoherent group fails R1");

        // A rule that LOSES the roles (every group's `roles` empty): the tripwire reads the planner's own
        // reference read, so it still fires, lists the roles read from storage, and the disagreement
        // between the rule and the planner is counted and fails R1 on its own.
        let lossy = || GuidTwinPlanArgs { rule: rule_without_roles, ..args(&stop, &progress) };
        let r = db.plan_guid_twins(lossy()).await.unwrap();
        assert_eq!(r.bars.tripwire_role_mix, 1, "the planner's read still sees buyer + Lot-ReviewOrg");
        let aok_like = roles(&["Lot-AddInfo", "Lot-ReviewOrg", "Procedure-Buyer"]);
        assert_eq!(r.bars.role_mix_refused[0].ref_roles, aok_like, "read from notice_ids, not echoed by the rule");
        assert_eq!(r.bars.role_ref_disagree, 1);
        let d = &r.bars.role_ref_disagree_samples[0];
        assert_eq!((d.notice_id, d.outcome, d.ref_roles.clone(), d.rule_roles.clone()), (26_483_825, "refused", aok_like, Vec::new()));
        assert!(!r.bars.pass);
        // Without the review slot the group is coherent (no tripwire), but the lost roles still fail R1.
        exec(&conn, "DELETE FROM notice_ids WHERE notice_id = 26483825 AND field_id = 'OPT-301-Lot-ReviewOrg'", vec![]).await;
        let r = db.plan_guid_twins(lossy()).await.unwrap();
        assert_eq!((r.bars.tripwire_role_mix, r.bars.role_ref_disagree), (0, 1));
        assert!(!r.bars.pass, "a rule that drops the roles is caught even on a coherent group");
        let r = db.plan_guid_twins(args(&stop, &progress)).await.unwrap();
        assert_eq!((r.bars.tripwire_role_mix, r.bars.role_ref_disagree), (0, 0));
        assert!(r.bars.pass, "{:#?}", r.bars);
        drop(conn);
        cleanup(&path);
    }

    // ---- Build 2: the wet run (windows, the window apply, the reconcile, the tripwires, A1) ----

    fn wet(notice: i64, section: &str, org: i64, key: &str, target: Option<i64>, whole: bool) -> GuidTwinWetMention {
        GuidTwinWetMention {
            notice_id: notice,
            section_id: section.to_owned(),
            org_id: org,
            org_value: format!("V{org}"),
            org_country: Some("DE".into()),
            org_name: key.to_owned(),
            key_name_norm: key.to_owned(),
            key_country: Some("DE".into()),
            target,
            mint_name: key.to_owned(),
            whole,
            target_mentions: 0,
            row_kept_mentions: 0,
        }
    }

    /// Build 2 (decision "Windows go by target key"): a row whose mentions go to two keys ties them into
    /// one component; components never split, pack greedily in the order of their smallest key, and one
    /// larger than the cap sits alone; the order is the plan's whatever the input order.
    #[test]
    fn wet_windows_go_by_target_key_and_never_split_a_component() {
        let mentions = vec![
            wet(1, "ORG-0001", 10, "b", Some(500), true),
            wet(1, "ORG-0002", 11, "b", Some(500), true),
            // Row 12 takes a mention to "c" and one to "d": c and d are one component.
            wet(2, "ORG-0001", 12, "c", None, true),
            wet(3, "ORG-0001", 12, "d", None, true),
            wet(3, "ORG-0002", 13, "d", None, false),
            wet(4, "ORG-0001", 14, "a", Some(400), true),
            // "e": three mentions, over a cap of 2, alone.
            wet(5, "ORG-0001", 15, "e", None, true),
            wet(5, "ORG-0002", 16, "e", None, true),
            wet(6, "ORG-0001", 17, "e", None, true),
        ];
        let windows = guid_twin_windows(&mentions, 2);
        let keys: Vec<Vec<&str>> = windows.iter().map(|w| w.keys.iter().map(|(k, _)| k.as_str()).collect()).collect();
        assert_eq!(keys, vec![vec!["a"], vec!["b"], vec!["c", "d"], vec!["e"]], "a+b would be 3 > 2; c+d tied by row 12");
        assert_eq!(windows[2].whole, vec![12]);
        assert_eq!(windows[2].partial, vec![13]);
        assert_eq!(windows[2].mentions.len(), 3, "a component over the cap is never split");
        assert_eq!(windows[3].mentions.len(), 3);
        let mut reversed = mentions.clone();
        reversed.reverse();
        assert_eq!(guid_twin_windows(&reversed, 2), windows, "deterministic for one plan");
        let one = guid_twin_windows(&mentions, 100);
        assert_eq!(one.len(), 1, "everything fits one window");
        assert_eq!(one[0].mentions.iter().map(|m| m.notice_id).collect::<Vec<_>>(), vec![1, 1, 2, 3, 3, 4, 5, 5, 6]);
        assert!(guid_twin_windows(&[], 10).is_empty());
    }

    /// Review round 3 (F1): windows close on the RE-READ budget too, not only on planned mentions — the
    /// plan-2660 shape: a run of keys with few planned mentions but heavy targets (622 at 111k, 2217 at
    /// 90k) fits one 2,000-mention window, so 542's re-derivation re-read every target's existing mentions
    /// under one hold. Each window's estimate is its distinct targets' existing mentions plus what they
    /// receive, plus each partial row's kept mentions; a component over the budget alone sits alone; the
    /// packing stays deterministic. Without the budget (the old packing) the three heavy keys share a
    /// window.
    #[test]
    fn wet_windows_close_on_the_reread_budget() {
        let heavy = |notice: i64, org: i64, key: &str, target: i64, existing: u64| GuidTwinWetMention {
            target_mentions: existing,
            ..wet(notice, "ORG-0001", org, key, Some(target), true)
        };
        let partial =
            GuidTwinWetMention { target_mentions: 49_000, row_kept_mentions: 7, ..wet(4, "ORG-0002", 13, "c", Some(3), false) };
        let mentions = vec![
            heavy(1, 10, "a", 1, 90_000),
            heavy(1, 11, "a", 1, 90_000),
            heavy(2, 12, "b", 2, 111_000),
            heavy(4, 14, "c", 3, 49_000),
            partial,
        ];
        assert_eq!(guid_twin_rebound_estimate(&mentions[..2]), 90_002, "one key: its existing mentions plus the two it takes");
        assert_eq!(guid_twin_rebound_estimate(&mentions[3..]), 49_002 + 7, "a partial row's kept mentions are re-read too");
        let old = guid_twin_windows_within(&mentions, GUID_TWIN_WET_WINDOW_MENTIONS, u64::MAX);
        assert_eq!(old.len(), 1, "planned mentions alone: one window re-reading ~250k");
        assert_eq!(old[0].rebound_estimate, 90_002 + 111_001 + 49_009);
        let windows = guid_twin_windows(&mentions, GUID_TWIN_WET_WINDOW_MENTIONS);
        let keys: Vec<Vec<&str>> = windows.iter().map(|w| w.keys.iter().map(|(k, _)| k.as_str()).collect()).collect();
        assert_eq!(keys, vec![vec!["a"], vec!["b"], vec!["c"]], "a+b and b+c are each over {GUID_TWIN_WET_WINDOW_REREAD}");
        assert_eq!(windows.iter().map(|w| w.rebound_estimate).collect::<Vec<_>>(), vec![90_002, 111_001, 49_009]);
        // One component over the budget alone sits alone, between light ones that pack together.
        let light = vec![heavy(1, 10, "a", 1, 10), heavy(2, 12, "b", 2, 150_000), heavy(4, 14, "c", 3, 10), heavy(5, 15, "d", 4, 10)];
        let windows = guid_twin_windows(&light, GUID_TWIN_WET_WINDOW_MENTIONS);
        let keys: Vec<Vec<&str>> = windows.iter().map(|w| w.keys.iter().map(|(k, _)| k.as_str()).collect()).collect();
        assert_eq!(keys, vec![vec!["a"], vec!["b"], vec!["c", "d"]], "the over-budget key alone; c and d share one");
        let mut reversed = light.clone();
        reversed.reverse();
        assert_eq!(guid_twin_windows(&reversed, GUID_TWIN_WET_WINDOW_MENTIONS), windows, "deterministic");
    }

    /// Build 2's tuple reconcile: the dry plan's (row, whole) set is the re-plan's (at or under its max)
    /// plus this job's logged rows; a row minted since is listed and runs; a dropped row, an added row, a
    /// whole↔partial flip and a logged row planned again each fail it.
    #[test]
    fn the_wet_reconcile_compares_row_whole_tuples() {
        let set = |v: &[(i64, bool)]| v.iter().copied().collect::<BTreeSet<_>>();
        let dry = set(&[(1, true), (2, true), (3, false)]);
        let r = guid_twin_reconcile(&dry, 100, &set(&[(2, true), (3, false), (150, true)]), &set(&[(1, true)]));
        assert!(r.ok, "{r:?}");
        assert_eq!(r.minted_since, vec![150]);
        let r = guid_twin_reconcile(&dry, 100, &set(&[(2, true), (3, true)]), &set(&[(1, true)]));
        assert!(!r.ok);
        assert_eq!((r.dropped.clone(), r.added.clone()), (vec![(3, false)], vec![(3, true)]), "a whole/partial flip");
        let r = guid_twin_reconcile(&dry, 100, &set(&[(1, true), (2, true), (3, false)]), &set(&[(1, true)]));
        assert_eq!(r.replanned_logged, vec![1]);
        assert!(!r.ok);
        let r = guid_twin_reconcile(&dry, 100, &set(&[(2, true), (3, false), (4, true)]), &set(&[(1, true)]));
        assert_eq!(r.added, vec![(4, true)]);
        let r = guid_twin_reconcile(&dry, 100, &set(&[(3, false)]), &set(&[(1, true)]));
        assert_eq!(r.dropped, vec![(2, true)]);
        // A row minted since the dry plan that a staged or restarted run on it already applied (logged above
        // its max): listed, never compared, so the next run on the SAME dry plan reconciles.
        let r = guid_twin_reconcile(&dry, 100, &set(&[(2, true), (3, false)]), &set(&[(1, true), (150, true)]));
        assert!(r.ok, "{r:?}");
        assert_eq!((r.logged_minted_since.clone(), r.added.clone()), (vec![150], vec![]));
        // One both logged and re-planned fails whatever its id.
        let r = guid_twin_reconcile(&dry, 100, &set(&[(2, true), (3, false), (150, false)]), &set(&[(1, true), (150, false)]));
        assert!(!r.ok, "{r:?}");
        assert_eq!(r.replanned_logged, vec![150]);
    }

    async fn version_row(conn: &turso::Connection, tender: i64, seq: i64, notice: i64) {
        exec(
            conn,
            "INSERT INTO tender_versions (tender_id, seq, caused_by_notice_id, published_at, publication_id)
             VALUES (?, ?, ?, 1717200000, ?)",
            vec![Value::Integer(tender), Value::Integer(seq), Value::Integer(notice), Value::Text(format!("{notice}"))],
        )
        .await;
    }

    async fn party_row(conn: &turso::Connection, version: (i64, i64), role: &str, org: i64, notice: i64, section: &str) {
        exec(
            conn,
            "INSERT INTO tender_version_parties (tender_id, seq, lot_id, role, organization_id, mention_notice_id,
                                                 mention_section_id) VALUES (?, ?, NULL, ?, ?, ?, ?)",
            vec![
                Value::Integer(version.0),
                Value::Integer(version.1),
                Value::Text(role.into()),
                Value::Integer(org),
                Value::Integer(notice),
                Value::Text(section.into()),
            ],
        )
        .await;
    }

    fn lang(l: Option<&str>) -> Option<String> {
        l.map(str::to_uppercase)
    }

    async fn rows_of(conn: &turso::Connection, sql: &str) -> Vec<Vec<Value>> {
        let mut rows = conn.query(sql, ()).await.unwrap();
        let mut out = Vec::new();
        while let Some(r) = rows.next().await.unwrap() {
            out.push((0..r.column_count()).map(|i| r.get_value(i).unwrap()).collect());
        }
        out
    }

    fn i(v: &Value) -> i64 {
        match v {
            Value::Integer(n) => *n,
            other => panic!("{other:?}"),
        }
    }

    /// The wet fixture: the Göttingen award in two copies (the DÖE copy names the buyer under V1 and V2,
    /// the TED copy under V1 alone: 22759261 PARTIAL, 22759262 whole) with its identifier-NULL target
    /// 2294259; and an `Amt 66` twin in two notices of two Tenders, four whole rows whose key has no
    /// identifier-NULL row (a missing key: one row is minted for all four), its first notice's buyer
    /// also a lot-result winner.
    async fn wet_seed(conn: &turso::Connection) {
        org_row(conn, 2_294_259, None, None, "Landkreis Göttingen", "DE").await;
        org_row(conn, 22_759_261, Some("national"), Some(V1), "Landkreis Göttingen", "DE").await;
        org_row(conn, 22_759_262, Some("national"), Some(V2), "Landkreis Göttingen", "DE").await;
        notice_row(conn, 26_483_825, None).await;
        notice_row(conn, 24_262_502, None).await;
        version_row(conn, 948_285, 1, 24_262_502).await;
        version_row(conn, 948_285, 2, 26_483_825).await;
        mention_row(conn, 26_483_825, "ORG-0001", 22_759_261, "Landkreis Göttingen", "DE", Some(V1)).await;
        mention_row(conn, 26_483_825, "ORG-0002", 22_759_262, "Landkreis Göttingen", "DE", Some(V2)).await;
        mention_row(conn, 24_262_502, "ORG-0001", 22_759_261, "Landkreis Göttingen", "DE", Some(V1)).await;
        party_row(conn, (948_285, 2), "Procedure-Buyer", 22_759_261, 26_483_825, "ORG-0001").await;
        party_row(conn, (948_285, 2), "Lot-AddInfo", 22_759_262, 26_483_825, "ORG-0002").await;
        party_row(conn, (948_285, 1), "Procedure-Buyer", 22_759_261, 24_262_502, "ORG-0001").await;
        for (k, value) in [V3, V4, V5, V6].into_iter().enumerate() {
            org_row(conn, 101 + k as i64, Some("national"), Some(value), "Amt 66", "DE").await;
        }
        for (n, tender, base) in [(1i64, 11i64, 0usize), (2, 12, 2)] {
            notice_row(conn, n, Some(tender)).await;
            let values = [V3, V4, V5, V6];
            // The first notice's first section publishes the key's mint name, the others its capitals.
            let first = if n == 1 { "Amt 66" } else { "AMT 66" };
            mention_row(conn, n, "ORG-0001", 101 + base as i64, first, "DE", Some(values[base])).await;
            mention_row(conn, n, "ORG-0002", 102 + base as i64, "AMT 66", "DE", Some(values[base + 1])).await;
            party_row(conn, (tender, 1), "Procedure-Buyer", 101 + base as i64, n, "ORG-0001").await;
        }
        exec(conn, "INSERT INTO lot_results (id, tender_id, notice_id, result_key) VALUES (1, 11, 1, 'RES-0001')", vec![]).await;
        exec(
            conn,
            "INSERT INTO tender_version_result_winners (tender_id, seq, lot_result_id, organization_id) VALUES (11, 1, 1, 101)",
            vec![],
        )
        .await;
    }

    /// Build 2: the wet windows on the fixture, staged — window 1 (the missing key), a re-plan that
    /// reconciles with the dry plan and the logged rows (the resume), then window 2 (Göttingen). Whole
    /// rows are dissolved (`organization removed`), the partial row keeps its TED mention and only its
    /// DÖE mention, with its party row, moves to 2294259 (`organization changed`, both); the missing key
    /// mints ONE row (DE, the lowest mention's published name, provisional, `organization added`) for its
    /// four mentions across two notices, and the winner follows its origin mention; every Tender a moved
    /// row touched is `tender changed` and stamped for the refold (issue 579), party rows alone included
    /// (Tender 12, and window 2's 948285); the manifest logs every moved mention, minted or not.
    #[tokio::test]
    async fn the_wet_windows_dissolve_whole_rows_rehome_partial_ones_and_mint_once() {
        // The Db writer keeps production's `foreign_keys = ON` (only the seeding connection is FK-off):
        // the mint-then-repoint, the rehome and the whole-row delete run under the mode production uses.
        let (db, conn, path) = open("wet", true).await;
        wet_seed(&conn).await;
        tender_row(&conn, 12).await;
        exec(&conn, "UPDATE notices SET projected = 1 WHERE id = 2", vec![]).await;
        let stop = || false;
        let progress = |_: u64, _: &str| {};
        let plan = db.plan_guid_twins(GuidTwinPlanArgs { not_planned: &[], planned: &[], ..args(&stop, &progress) }).await.unwrap();
        assert_eq!(plan.planned_row_ids, vec![101, 102, 103, 104, 22_759_262]);
        assert_eq!(plan.partial_row_ids, vec![22_759_261]);
        assert_eq!(plan.wet_mentions.len(), 6);
        // A10: the partial Göttingen row is listed apart, so the surviving set is kept ∪ partial.
        let exhibit = &plan.exhibits["DE landkreis göttingen"];
        assert_eq!((exhibit.planned, exhibit.partial, exhibit.partial_ids.clone(), exhibit.kept), (1, 1, vec![22_759_261], 0));
        let amt = plan.wet_mentions.iter().find(|m| m.org_id == 104).unwrap();
        assert_eq!((amt.target, amt.mint_name.as_str(), amt.key_name_norm.as_str()), (None, "Amt 66", "amt 66"));
        // F1: the re-read inputs — a missing key has no existing mentions; the partial row keeps its TED one.
        assert_eq!((amt.target_mentions, amt.row_kept_mentions), (0, 0));
        let rehome = plan.wet_mentions.iter().find(|m| m.org_id == 22_759_261).unwrap();
        assert_eq!((rehome.target_mentions, rehome.row_kept_mentions), (0, 1), "2294259 holds no mention yet");
        let dry: BTreeSet<(i64, bool)> =
            plan.planned_row_ids.iter().map(|&id| (id, true)).chain(plan.partial_row_ids.iter().map(|&id| (id, false))).collect();
        let windows = guid_twin_windows(&plan.wet_mentions, 2);
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].keys, vec![("amt 66".to_owned(), Some("DE".to_owned()))]);
        let w1 = db
            .dissolve_guid_twin_window(GuidTwinWindowArgs { window: &windows[0], job_id: 7, window_no: 1, gate_since_notice_id: None, fields: &["BT-500-Organization-Company"], normalize: lang })
            .await
            .unwrap();
        assert_eq!((w1.mentions_moved, w1.counts.dissolved, w1.counts.fresh, w1.counts.reused), (4, 4, 1, 0));
        assert_eq!(w1.minted.len(), 1, "one row per missing key");
        let minted = w1.minted[0].clone();
        assert_eq!((minted.country.as_str(), minted.name_norm.as_str(), minted.name.as_str(), minted.mentions), ("DE", "amt 66", "Amt 66", 4));
        assert_eq!(w1.counts.winners, 1, "the winner follows its origin mention");
        assert_eq!(w1.refold_tenders, vec![11, 12], "Tender 12 is touched only by org 103's party row");
        assert_eq!(rows_of(&conn, "SELECT projection_epoch FROM tenders WHERE id = 12").await[0][0], Value::Integer(0));
        assert_eq!(rows_of(&conn, "SELECT projected FROM notices WHERE id = 2").await[0][0], Value::Integer(0));
        let row = rows_of(&conn, &format!("SELECT country, name, provisional, identifier FROM organizations WHERE id = {}", minted.target_id)).await;
        assert_eq!(row[0], vec![Value::Text("DE".into()), Value::Text("Amt 66".into()), Value::Integer(1), Value::Null]);
        let on = rows_of(&conn, "SELECT DISTINCT organization_id FROM organization_mentions WHERE notice_id IN (1, 2)").await;
        assert_eq!(on, vec![vec![Value::Integer(minted.target_id)]]);
        assert_eq!(
            rows_of(&conn, "SELECT organization_id FROM tender_version_result_winners").await,
            vec![vec![Value::Integer(minted.target_id)]]
        );
        assert_eq!(rows_of(&conn, "SELECT COUNT(*) FROM organizations WHERE id BETWEEN 101 AND 104").await[0][0], Value::Integer(0));

        // The resume: a re-plan reconciles with the dry plan through the logged rows.
        let logged: BTreeSet<(i64, bool)> = db.guid_twin_log(7).await.unwrap().iter().map(|r| (r.org_id, r.whole)).collect();
        assert_eq!(logged.len(), 4);
        let replan = db.plan_guid_twins(GuidTwinPlanArgs { not_planned: &[], planned: &[], ..args(&stop, &progress) }).await.unwrap();
        let replanned: BTreeSet<(i64, bool)> = replan
            .planned_row_ids
            .iter()
            .map(|&id| (id, true))
            .chain(replan.partial_row_ids.iter().map(|&id| (id, false)))
            .collect();
        let reconcile = guid_twin_reconcile(&dry, plan.max_id, &replanned, &logged);
        assert!(reconcile.ok, "{reconcile:?}");
        let rest = guid_twin_windows(&replan.wet_mentions, 2);
        assert_eq!(rest.len(), 1, "only Göttingen is left");
        let w2 = db
            .dissolve_guid_twin_window(GuidTwinWindowArgs { window: &rest[0], job_id: 7, window_no: 2, gate_since_notice_id: None, fields: &["BT-500-Organization-Company"], normalize: lang })
            .await
            .unwrap();
        assert_eq!((w2.mentions_moved, w2.counts.dissolved, w2.counts.reused, w2.counts.fresh), (2, 1, 2, 0));
        assert_eq!(w2.reused_targets, vec![2_294_259]);
        assert_eq!(w2.refold_tenders, vec![948_285], "party rows only: the rehome's and 22759262's");
        let bound = rows_of(&conn, "SELECT notice_id, section_id, organization_id FROM organization_mentions WHERE notice_id > 100 ORDER BY 1, 2").await;
        assert_eq!(
            bound.iter().map(|r| (i(&r[0]), i(&r[2]))).collect::<Vec<_>>(),
            vec![(24_262_502, 22_759_261), (26_483_825, 2_294_259), (26_483_825, 2_294_259)],
            "the TED mention stays on the partial row"
        );
        let parties = rows_of(&conn, "SELECT mention_notice_id, organization_id FROM tender_version_parties WHERE tender_id = 948285 ORDER BY 1, 2").await;
        assert_eq!(
            parties.iter().map(|r| (i(&r[0]), i(&r[1]))).collect::<Vec<_>>(),
            vec![(24_262_502, 22_759_261), (26_483_825, 2_294_259), (26_483_825, 2_294_259)]
        );
        assert_eq!(rows_of(&conn, "SELECT COUNT(*) FROM organizations WHERE id = 22759261").await[0][0], Value::Integer(1), "it stays");
        assert_eq!(
            rows_of(&conn, "SELECT identifier FROM organizations WHERE id = 22759261").await[0][0],
            Value::Text(GuidKeep::key(V1)),
            "a rehomed row keeps its GUID identifier (A10 counts it among the surviving GUID rows)"
        );
        assert_eq!(rows_of(&conn, "SELECT COUNT(*) FROM organizations WHERE id = 22759262").await[0][0], Value::Integer(0));

        // The change rows.
        let changes = rows_of(&conn, "SELECT entity_kind, op, entity_id FROM changes ORDER BY 1, 2, 3").await;
        let count = |kind: &str, op: &str| changes.iter().filter(|c| c[0] == Value::Text(kind.into()) && c[1] == Value::Text(op.into())).count();
        assert_eq!(count("organization", "removed"), 5);
        assert_eq!(count("organization", "added"), 1);
        assert_eq!(count("organization", "changed"), 2, "2294259 (reused) and 22759261 (rehomed)");
        let tenders: BTreeSet<i64> = changes.iter().filter(|c| c[0] == Value::Text("tender".into())).map(|c| i(&c[2])).collect();
        assert_eq!(tenders, [11, 12, 948_285].into_iter().collect());

        // The manifest: every moved mention, its pre-image and target.
        let log = db.guid_twin_log(7).await.unwrap();
        assert_eq!(log.len(), 6);
        assert_eq!(log.iter().filter(|r| r.target_minted).count(), 4);
        let partial = log.iter().find(|r| r.org_id == 22_759_261).unwrap();
        assert_eq!(
            (partial.whole, partial.notice_id, partial.target_id, partial.org_value.clone()),
            (false, 26_483_825, 2_294_259, GuidKeep::key(V1))
        );
        let final_plan = db.plan_guid_twins(GuidTwinPlanArgs { not_planned: &[], planned: &[], ..args(&stop, &progress) }).await.unwrap();
        assert!(final_plan.planned_row_ids.is_empty() && final_plan.partial_row_ids.is_empty(), "{:?}", final_plan.partial_row_ids);
        // A1 on the manifest's targets: the refused values sit on single-Tender targets — no candidate.
        let rec = db
            .guid_twin_recurrence(GuidTwinRecurrenceArgs {
                is_guid: uuid_v4,
                gated_v4,
                rule,
                role_ref,
                keep: &GuidKeep::default(),
                since_notice_id: None,
                window: 1_000,
                samples: 10,
                stop: &stop,
                progress: &progress,
            })
            .await
            .unwrap();
        assert_eq!((rec.targets_from_log, rec.refused_mentions, rec.candidate_count), (2, 6, 0), "{rec:?}");
        assert_eq!(rec.reverse_drift_mentions, 0, "the same rule moved them: no reverse drift");
        assert!(rec.pass);
        // The window's timings and its cost driver are recorded.
        assert!(w2.rebound_target_mentions >= 3, "2294259's two moved mentions and 22759261's kept one: {w2:?}");
        assert_eq!((w2.rebound_estimate, w2.rebound_target_mentions), (3, 3), "F1: the window's estimate is what it re-read");
        drop(conn);
        cleanup(&path);
    }

    /// Build 2: a window refuses — and keeps nothing — when a planned mention no longer sits on its
    /// planned row, or a whole row records a mention the plan does not hold.
    #[tokio::test]
    async fn a_wet_window_refuses_when_the_store_moved_since_the_plan() {
        let (db, conn, path) = open("wet-refuse", true).await;
        db.set_foreign_keys(false).await.unwrap();
        wet_seed(&conn).await;
        let stop = || false;
        let progress = |_: u64, _: &str| {};
        let plan = db.plan_guid_twins(GuidTwinPlanArgs { not_planned: &[], planned: &[], ..args(&stop, &progress) }).await.unwrap();
        let windows = guid_twin_windows(&plan.wet_mentions, 2);
        // A new mention of 101 the plan never saw: 101 is no longer whole.
        notice_row(&conn, 3, Some(13)).await;
        mention_row(&conn, 3, "ORG-0009", 101, "Amt 66", "DE", Some(V3)).await;
        let err = db
            .dissolve_guid_twin_window(GuidTwinWindowArgs { window: &windows[0], job_id: 8, window_no: 1, gate_since_notice_id: None, fields: &[], normalize: lang })
            .await
            .expect_err("no longer whole");
        assert!(err.to_string().contains("no longer whole"), "{err}");
        assert_eq!(rows_of(&conn, "SELECT COUNT(*) FROM organizations WHERE id BETWEEN 101 AND 104").await[0][0], Value::Integer(4), "rolled back");
        assert!(db.guid_twin_log(8).await.unwrap().is_empty());
        // A planned mention moved elsewhere since the plan.
        exec(&conn, "UPDATE organization_mentions SET organization_id = 2294259 WHERE notice_id = 26483825 AND section_id = 'ORG-0002'", vec![]).await;
        let err = db
            .dissolve_guid_twin_window(GuidTwinWindowArgs { window: &windows[1], job_id: 8, window_no: 2, gate_since_notice_id: None, fields: &[], normalize: lang })
            .await
            .expect_err("moved");
        assert!(err.to_string().contains("sits on Some(2294259)"), "{err}");
        drop(conn);
        cleanup(&path);
    }

    /// Build 2: the resolver arm's target, exactly — a mention published under RE lands on the FR
    /// identifier-NULL row (the register's country), the LOWEST id of two, never the RE row; and a window
    /// whose planned key is not the key the resolver arm reads (an RE key) refuses (A11).
    #[tokio::test]
    async fn the_wet_target_is_the_resolver_arms_row() {
        let (db, conn, path) = open("wet-target", true).await;
        db.set_foreign_keys(false).await.unwrap();
        const SDIS: &str = "SDIS de La Réunion";
        org_row(&conn, 60, None, None, SDIS, "RE").await;
        org_row(&conn, 64, None, None, SDIS, "FR").await;
        org_row(&conn, 61, None, None, SDIS, "FR").await;
        org_row(&conn, 62, Some("national"), Some(V1), SDIS, "FR").await;
        notice_row(&conn, 70, Some(80)).await;
        mention_row(&conn, 70, "ORG-0001", 62, SDIS, "RE", Some(V1)).await;
        let mention = |key_country: &str, target: Option<i64>| GuidTwinWetMention {
            org_value: GuidKeep::key(V1),
            key_name_norm: org_name_norm(SDIS),
            key_country: Some(key_country.to_owned()),
            target,
            mint_name: SDIS.into(),
            ..wet(70, "ORG-0001", 62, SDIS, target, true)
        };
        let window = |m: GuidTwinWetMention| GuidTwinWindow { keys: vec![], whole: vec![62], partial: vec![], mentions: vec![m], ..Default::default() };
        let err = db
            .dissolve_guid_twin_window(GuidTwinWindowArgs { window: &window(mention("RE", Some(60))), job_id: 9, window_no: 1, gate_since_notice_id: None, fields: &[], normalize: lang })
            .await
            .expect_err("an RE key is not the resolver's");
        assert!(err.to_string().contains("landed on 61"), "{err}");
        let done = db
            .dissolve_guid_twin_window(GuidTwinWindowArgs { window: &window(mention("FR", Some(61))), job_id: 9, window_no: 1, gate_since_notice_id: None, fields: &[], normalize: lang })
            .await
            .unwrap();
        assert_eq!(done.reused_targets, vec![61], "the FR row, the lowest id of 61 and 64");
        assert_eq!(rows_of(&conn, "SELECT organization_id FROM organization_mentions").await, vec![vec![Value::Integer(61)]]);
        drop(conn);
        cleanup(&path);
    }

    /// Build 2's dissolve tripwires: a whole planned row that is a merge survivor, and one a review table
    /// names, each fail the plan; a partial row trips neither (it survives).
    #[tokio::test]
    async fn the_merge_survivor_and_under_review_tripwires_fail_the_plan() {
        let (db, conn, path) = open("wet-trip", true).await;
        wet_seed(&conn).await;
        merged_row(&conn, V7, 102).await;
        merged_row(&conn, V8, 22_759_261).await;
        let stop = || false;
        let progress = |_: u64, _: &str| {};
        let review: HashSet<i64> = [103, 22_759_261].into_iter().collect();
        let plan = db
            .plan_guid_twins(GuidTwinPlanArgs { not_planned: &[], planned: &[], under_review: Some(&review), ..args(&stop, &progress) })
            .await
            .unwrap();
        assert_eq!((plan.bars.tripwire_merge_survivor, plan.bars.merge_survivor_rows.clone()), (1, vec![102]));
        assert_eq!((plan.bars.tripwire_under_review, plan.bars.under_review_rows.clone()), (1, vec![103]));
        assert!(plan.bars.under_review_checked);
        assert!(!plan.bars.pass);
        drop(conn);
        cleanup(&path);
    }

    /// Build 2 (A3 iii): with a compiled KEEP the self-check is no tautology — a stable row's value
    /// missing from it fails the plan and is listed as a KEEP file line; walk candidates the file lacks
    /// otherwise are advisory; the report names the KEEP's source and version.
    #[tokio::test]
    async fn the_compiled_keep_self_check_lists_a_keep_add_commit() {
        let (db, conn, path) = open("wet-keep", true).await;
        seed(&conn).await;
        org_row(&conn, 254, Some("national"), Some(V9), "DB Netz", "DE").await;
        notice_row(&conn, 5_001, Some(9_001)).await;
        mention_row(&conn, 5_001, "ORG-0001", 254, "DB Netz", "DE", Some(V9)).await;
        merged_row(&conn, V10, 22_759_262).await;
        let stop = || false;
        let progress = |_: u64, _: &str| {};
        let compiled = GuidKeep::default();
        let plan = db
            .plan_guid_twins(GuidTwinPlanArgs { keep: Some(&compiled), keep_version: "keep-test", unsafe_names: &["Vergabekammer"], ..args(&stop, &progress) })
            .await
            .unwrap();
        assert_eq!((plan.keep_source, plan.keep_version.as_str(), plan.unsafe_names.clone()), ("compiled", "keep-test", vec!["Vergabekammer".to_owned()]));
        assert_eq!(plan.keep_size, 0);
        assert_eq!(plan.bars.keep_self_check_missing, vec![GuidKeep::key(V9)], "254 is stable by id");
        assert_eq!(plan.keep_add_commit, vec![format!("DE\t{}\t254\t1\t1\tstable", GuidKeep::key(V9))]);
        assert_eq!(plan.keep_candidates_not_in_file, 1, "the merged value: advisory");
        assert!(plan.keep_candidates_not_in_file_lines[0].ends_with("\tmerged"));
        assert!(!plan.bars.pass);
        let walk = db.plan_guid_twins(args(&stop, &progress)).await.unwrap();
        assert!(walk.bars.keep_self_check_missing.is_empty() && walk.keep_source == "walk");
        drop(conn);
        cleanup(&path);
    }

    /// A1: the recurrence arm reads the manifest's targets and, with `since_notice_id`, the identifier-NULL
    /// rows of later twin copies; a refused value whose mentions span two Tenders outside KEEP is a
    /// candidate with its whole cohort; a value at one Tender, and a KEEP value, are not — and a value's
    /// OWN GUID row counts: V2, refused once (one Tender), reaches a second Tender through its own row's
    /// non-twin mention (A1's second invisible case). Then the KEEP grows (V1 and V2 added after the
    /// deploy): their refused mentions stay on the identifier-NULL row 900 although the rule now binds
    /// them — reverse drift, `since` half — V1 restored onto a minted row (no identity row stands), V2
    /// onto its own row 960; the drift then reads 0.
    #[tokio::test]
    async fn the_recurrence_arm_lists_a_refused_value_in_a_second_tender() {
        let (db, conn, path) = open("wet-a1", true).await;
        db.set_foreign_keys(false).await.unwrap();
        // A twin copy after the deploy (notice 500): the gate refused both values, bound to the 234 row 900.
        org_row(&conn, 900, None, None, "Stadt Musterhausen", "DE").await;
        notice_row(&conn, 500, Some(50)).await;
        mention_row(&conn, 500, "ORG-0001", 900, "Stadt Musterhausen", "DE", Some(V1)).await;
        mention_row(&conn, 500, "ORG-0002", 900, "Stadt Musterhausen", "DE", Some(V2)).await;
        // V1 recurs in a second Tender, twinned again (notice 501, bound to 900 as well).
        notice_row(&conn, 501, Some(51)).await;
        mention_row(&conn, 501, "ORG-0001", 900, "Stadt Musterhausen", "DE", Some(V1)).await;
        mention_row(&conn, 501, "ORG-0002", 900, "Stadt Musterhausen", "DE", Some(V3)).await;
        // A refused section bound to an identified row: an anomaly.
        org_row(&conn, 950, Some("vat"), Some("DE308703297"), "Kreis Beispiel", "DE").await;
        notice_row(&conn, 502, Some(52)).await;
        mention_row(&conn, 502, "ORG-0001", 950, "Kreis Beispiel", "DE", Some(V4)).await;
        mention_row(&conn, 502, "ORG-0002", 950, "Kreis Beispiel", "DE", Some(V5)).await;
        // V2's own GUID row, minted by a non-twin notice of another Tender (named once: no twin group).
        org_row(&conn, 960, Some("national"), Some(V2), "Stadt Beispielhausen", "DE").await;
        notice_row(&conn, 503, Some(53)).await;
        mention_row(&conn, 503, "ORG-0001", 960, "Stadt Beispielhausen", "DE", Some(V2)).await;
        let stop = || false;
        let progress = |_: u64, _: &str| {};
        let run = |keep: GuidKeep, since: Option<i64>| {
            let db = &db;
            async move {
                db.guid_twin_recurrence(GuidTwinRecurrenceArgs {
                    is_guid: uuid_v4,
                    gated_v4,
                    rule,
                    role_ref,
                    keep: &keep,
                    since_notice_id: since,
                    window: 1,
                    samples: 10,
                    stop: &stop,
                    progress: &progress,
                })
                .await
                .unwrap()
            }
        };
        let none = run(GuidKeep::default(), None).await;
        assert_eq!((none.targets, none.candidate_count, none.reverse_drift_mentions), (0, 0, 0), "no manifest, no since");
        let r = run(GuidKeep::default(), Some(499)).await;
        assert_eq!((r.targets_from_since, r.targets, r.since_twin_copies), (1, 1, 3), "{r:?}");
        assert_eq!(r.anomaly_count, 2, "Kreis Beispiel's sections sit on a VAT row");
        assert_eq!((r.values, r.single_tender_values, r.candidate_count), (3, 1, 2), "{r:?}");
        let c = &r.candidates[0];
        assert_eq!((c.value.as_str(), c.tenders, c.refused_mentions), (GuidKeep::key(V1).as_str(), 2, 2));
        assert_eq!((c.own_row, c.own_row_mentions), (None, 0));
        assert_eq!(c.cohort, vec![(500, "ORG-0001".to_owned(), 900), (501, "ORG-0001".to_owned(), 900)]);
        let own = &r.candidates[1];
        assert_eq!(
            (own.value.as_str(), own.tenders, own.refused_mentions, own.own_row, own.own_row_mentions),
            (GuidKeep::key(V2).as_str(), 2, 1, Some(960), 1),
            "V2's refused mention is one Tender, its own row's the second"
        );
        assert_eq!(own.cohort, vec![(500, "ORG-0002".to_owned(), 900)]);
        assert_eq!(r.reverse_drift_mentions, 0, "the rule that refused them still refuses them");
        assert!(!r.pass);

        // V1 and V2 join KEEP after the deploy: the rule binds them now, but their mentions sit on 900.
        let grown: GuidKeep = [V1, V2].into_iter().collect();
        // A mention a reviewed rehoming verdict placed on 900 is no drift (a restore must never undo it).
        exec(
            &conn,
            "INSERT INTO org_mention_rehoming (case_org_id, notice_id, section_id, cohort, action, target_org_id, rationale,
                                               confidence, reviewed_at) VALUES (950, 501, 'ORG-0001', 'c', 'rehome', 900, 'r', 'high', 0)",
            vec![],
        )
        .await;
        let reviewed = run(grown.clone(), Some(499)).await;
        assert_eq!((reviewed.reverse_drift_since, reviewed.reverse_drift_verdict_skipped), (2, 1), "{reviewed:?}");
        exec(&conn, "DELETE FROM org_mention_rehoming", vec![]).await;
        let kept = run(grown.clone(), Some(499)).await;
        assert_eq!((kept.values, kept.single_tender_values, kept.candidate_count), (1, 1, 0), "only V3 is refused: {kept:?}");
        assert_eq!(
            (kept.reverse_drift_mentions, kept.reverse_drift_since, kept.reverse_drift_stock, kept.reverse_drift_values),
            (3, 3, 0, 2),
            "{kept:?}"
        );
        let v1 = &kept.reverse_drift[0];
        assert_eq!((v1.value.as_str(), v1.country.as_deref(), v1.restore, v1.own_row), (GuidKeep::key(V1).as_str(), Some("DE"), "mint", None));
        assert_eq!(v1.cohort, vec![(500, "ORG-0001".to_owned(), 900), (501, "ORG-0001".to_owned(), 900)]);
        assert_eq!(v1.name, "Stadt Musterhausen");
        let v2 = &kept.reverse_drift[1];
        assert_eq!((v2.restore, v2.own_row, v2.cohort.clone()), ("own_row", Some(960), vec![(500, "ORG-0002".to_owned(), 900)]));
        assert!(!kept.pass, "reverse drift fails the arm");

        // The restore: V1 onto ONE minted row (DE, national, V1, its first mention's name), V2 onto 960.
        let fields = ["BT-500-Organization-Company"];
        let d1 = db.restore_guid_twin_value(GuidTwinRestoreArgs { value: v1, fields: &fields, normalize: lang }).await.unwrap();
        assert!(d1.minted);
        assert_eq!(d1.moved.len(), 2);
        let row = rows_of(&conn, &format!("SELECT country, identifier_kind, identifier, name, provisional FROM organizations WHERE id = {}", d1.target_id)).await;
        assert_eq!(
            row[0],
            vec![
                Value::Text("DE".into()),
                Value::Text("national".into()),
                Value::Text(GuidKeep::key(V1)),
                Value::Text("Stadt Musterhausen".into()),
                Value::Integer(0)
            ]
        );
        let d2 = db.restore_guid_twin_value(GuidTwinRestoreArgs { value: v2, fields: &fields, normalize: lang }).await.unwrap();
        assert_eq!((d2.minted, d2.target_id, d2.moved.len()), (false, 960, 1));
        let bound = rows_of(&conn, "SELECT notice_id, section_id, organization_id FROM organization_mentions WHERE organization_id <> 950 ORDER BY 1, 2").await;
        assert_eq!(
            bound.iter().map(|r| (i(&r[0]), i(&r[2]))).collect::<Vec<_>>(),
            vec![(500, d1.target_id), (500, 960), (501, d1.target_id), (501, 900), (503, 960)],
            "the drifted mentions sit where a rebuild binds them; V3 stays on 900"
        );
        let changes = rows_of(&conn, "SELECT entity_kind, op, entity_id FROM changes ORDER BY 1, 2, 3").await;
        assert!(changes.contains(&vec![Value::Text("organization".into()), Value::Text("added".into()), Value::Integer(d1.target_id)]));
        assert!(changes.contains(&vec![Value::Text("organization".into()), Value::Text("changed".into()), Value::Integer(900)]));
        assert!(changes.contains(&vec![Value::Text("organization".into()), Value::Text("changed".into()), Value::Integer(960)]));
        // A stale cohort refuses: the read had (500, ORG-0001) on 900; it has moved since.
        let stale = GuidTwinDriftValue { cohort: vec![(500, "ORG-0001".to_owned(), 900)], ..v1.clone() };
        let err = db.restore_guid_twin_value(GuidTwinRestoreArgs { value: &stale, fields: &fields, normalize: lang }).await.expect_err("moved");
        assert!(err.to_string().contains("sits on Some("), "{err}");
        let after = run(grown, Some(499)).await;
        assert_eq!(after.reverse_drift_mentions, 0, "{after:?}");
        assert!(after.pass, "{after:?}");
        drop(conn);
        cleanup(&path);
    }

    async fn tender_row(conn: &turso::Connection, id: i64) {
        exec(
            conn,
            "INSERT INTO tenders (id, source, kind, created_at, projection_epoch) VALUES (?, 'doe', 'procedure', 0, ?)",
            vec![Value::Integer(id), Value::Integer(crate::canonical::PROJECTION_EPOCH)],
        )
        .await;
    }

    async fn winner_row(conn: &turso::Connection, lot_result: (i64, i64, i64), version: (i64, i64), org: i64, is_buyer: Option<i64>) {
        exec(
            conn,
            "INSERT INTO lot_results (id, tender_id, notice_id, result_key) VALUES (?, ?, ?, 'RES-0001')",
            vec![Value::Integer(lot_result.0), Value::Integer(lot_result.1), Value::Integer(lot_result.2)],
        )
        .await;
        exec(
            conn,
            "INSERT INTO tender_version_result_winners (tender_id, seq, lot_result_id, organization_id, is_buyer) VALUES (?, ?, ?, ?, ?)",
            vec![
                Value::Integer(version.0),
                Value::Integer(version.1),
                Value::Integer(lot_result.0),
                Value::Integer(org),
                is_buyer.map_or(Value::Null, Value::Integer),
            ],
        )
        .await;
    }

    async fn bid_party_row(conn: &turso::Connection, version: (i64, i64), org: i64, notice: i64, section: &str) {
        exec(
            conn,
            "INSERT INTO tender_version_bid_parties (tender_id, seq, bid_id, role, organization_id, mention_notice_id,
                                                     mention_section_id) VALUES (?, ?, 1, 'tenderer', ?, ?, ?)",
            vec![
                Value::Integer(version.0),
                Value::Integer(version.1),
                Value::Integer(org),
                Value::Integer(notice),
                Value::Text(section.into()),
            ],
        )
        .await;
    }

    /// A2's rehome through the wet window, by every derived table (`only = Some(set)` in 440's body): the
    /// partial row 22759261 keeps its TED mention and moves its DÖE one. A winner whose origin is the
    /// KEPT TED mention stays; one whose origin is the MOVED DÖE mention moves to 2294259 with its
    /// `is_buyer`; the moved mention's bid party follows it (and a whole row's bid party follows its
    /// mention to the minted row); a winner on a Tender no moved notice causes is untouched (not deleted,
    /// not stamped, not announced); a winner on the moved mention's Tender that no origin or causing tier
    /// decides — its tier-3 chain mixes the kept TED and the moved DÖE mention — is unresolved, so tier 5
    /// deletes it and stamps the Tender stale (A12), never a guess between the two rows.
    #[tokio::test]
    async fn a_partial_rows_rehome_moves_only_its_moved_mentions_winners_and_bid_parties() {
        let (db, conn, path) = open("wet-rehome", true).await;
        db.set_foreign_keys(false).await.unwrap();
        wet_seed(&conn).await;
        for t in [948_285i64, 990, 11, 12] {
            tender_row(&conn, t).await;
        }
        notice_row(&conn, 900, Some(990)).await;
        notice_row(&conn, 901, None).await;
        version_row(&conn, 948_285, 3, 901).await;
        exec(&conn, "UPDATE notices SET projected = 1", vec![]).await;
        winner_row(&conn, (10, 948_285, 24_262_502), (948_285, 1), 22_759_261, None).await;
        winner_row(&conn, (11, 948_285, 26_483_825), (948_285, 2), 22_759_261, Some(1)).await;
        winner_row(&conn, (12, 990, 900), (990, 1), 22_759_261, None).await;
        winner_row(&conn, (13, 948_285, 901), (948_285, 3), 22_759_261, None).await;
        bid_party_row(&conn, (948_285, 2), 22_759_261, 26_483_825, "ORG-0001").await;
        bid_party_row(&conn, (12, 1), 104, 2, "ORG-0002").await;
        let stop = || false;
        let progress = |_: u64, _: &str| {};
        let plan = db.plan_guid_twins(GuidTwinPlanArgs { not_planned: &[], planned: &[], ..args(&stop, &progress) }).await.unwrap();
        assert_eq!((plan.planned_row_ids.clone(), plan.partial_row_ids.clone()), (vec![101, 102, 103, 104, 22_759_262], vec![22_759_261]));
        let windows = guid_twin_windows(&plan.wet_mentions, 2);
        let apply = |k: usize| {
            let db = &db;
            let w = windows[k].clone();
            async move {
                db.dissolve_guid_twin_window(GuidTwinWindowArgs { window: &w, job_id: 14, window_no: k as u64 + 1, gate_since_notice_id: None, fields: &[], normalize: lang })
                    .await
                    .unwrap()
            }
        };
        let w1 = apply(0).await;
        assert_eq!(w1.counts.bid_parties, 1, "the whole row's bid party follows its mention");
        let minted = w1.minted[0].target_id;
        let w2 = apply(1).await;
        assert_eq!((w2.counts.winners, w2.counts.winners_deleted, w2.counts.bid_parties), (1, 1, 1), "{:?}", w2.counts);
        assert_eq!(w2.refold_tenders, vec![948_285]);
        let winners = rows_of(
            &conn,
            "SELECT tender_id, seq, lot_result_id, organization_id, is_buyer FROM tender_version_result_winners \
              WHERE tender_id IN (948285, 990) ORDER BY 1, 2, 3",
        )
        .await;
        assert_eq!(
            winners,
            vec![
                vec![Value::Integer(990), Value::Integer(1), Value::Integer(12), Value::Integer(22_759_261), Value::Null],
                vec![Value::Integer(948_285), Value::Integer(1), Value::Integer(10), Value::Integer(22_759_261), Value::Null],
                vec![Value::Integer(948_285), Value::Integer(2), Value::Integer(11), Value::Integer(2_294_259), Value::Integer(1)],
            ],
            "the kept mention's winner stays, the moved one's moves with is_buyer, the unrelated Tender's stays, the \
             undecidable one went to tier 5"
        );
        let bids = rows_of(&conn, "SELECT mention_notice_id, organization_id FROM tender_version_bid_parties ORDER BY 1").await;
        assert_eq!(
            bids.iter().map(|r| (i(&r[0]), i(&r[1]))).collect::<Vec<_>>(),
            vec![(2, minted), (26_483_825, 2_294_259)]
        );
        let epochs = rows_of(&conn, "SELECT id, projection_epoch FROM tenders WHERE id IN (948285, 990) ORDER BY 1").await;
        assert_eq!(
            epochs.iter().map(|r| (i(&r[0]), i(&r[1]))).collect::<Vec<_>>(),
            vec![(990, crate::canonical::PROJECTION_EPOCH), (948_285, 0)],
            "only the moved mention's Tender is stamped stale"
        );
        let projected = rows_of(&conn, "SELECT id, projected FROM notices WHERE id IN (900, 901, 24262502, 26483825) ORDER BY 1").await;
        assert_eq!(
            projected.iter().map(|r| (i(&r[0]), i(&r[1]))).collect::<Vec<_>>(),
            vec![(900, 1), (901, 0), (24_262_502, 0), (26_483_825, 0)],
            "Tender 948285's notices re-queued, Tender 990's not"
        );
        let changed = rows_of(&conn, "SELECT entity_id FROM changes WHERE entity_kind = 'tender' ORDER BY 1").await;
        let changed: BTreeSet<i64> = changed.iter().map(|r| i(&r[0])).collect();
        assert!(changed.contains(&948_285) && !changed.contains(&990), "{changed:?}");
        drop(conn);
        cleanup(&path);
    }

    /// A12's stale stamps through the wet window: a WHOLE planned row whose winner no tier decides — its
    /// two mentions on the result's origin notice publish two names (so tier 3's one-target premise
    /// fails), no RES descent, no non-buyer party — goes to tier 5: the winner row deleted, its Tender
    /// stamped stale (`projection_epoch` 0), its notice re-queued, the Tender in `refold_tenders` and
    /// announced changed.
    #[tokio::test]
    async fn a_whole_rows_undecidable_winner_goes_to_tier_five_and_is_stamped_stale() {
        let (db, conn, path) = open("wet-tier5", true).await;
        db.set_foreign_keys(false).await.unwrap();
        org_row(&conn, 200, Some("national"), Some(V7), "Amt 67", "DE").await;
        notice_row(&conn, 300, Some(310)).await;
        tender_row(&conn, 310).await;
        exec(&conn, "UPDATE notices SET projected = 1", vec![]).await;
        mention_row(&conn, 300, "ORG-0001", 200, "Amt 67", "DE", Some(V7)).await;
        mention_row(&conn, 300, "ORG-0002", 200, "Amt 68", "DE", Some(V7)).await;
        party_row(&conn, (310, 1), "Procedure-Buyer", 200, 300, "ORG-0001").await;
        winner_row(&conn, (20, 310, 300), (310, 1), 200, None).await;
        let m = |section: &str, key: &str| GuidTwinWetMention { org_value: GuidKeep::key(V7), ..wet(300, section, 200, key, None, true) };
        let window = GuidTwinWindow { keys: vec![], whole: vec![200], partial: vec![], mentions: vec![m("ORG-0001", "amt 67"), m("ORG-0002", "amt 68")], ..Default::default() };
        let w = db
            .dissolve_guid_twin_window(GuidTwinWindowArgs { window: &window, job_id: 15, window_no: 1, gate_since_notice_id: None, fields: &[], normalize: lang })
            .await
            .unwrap();
        assert_eq!((w.counts.dissolved, w.counts.winners, w.counts.winners_deleted, w.counts.refold_notices), (1, 0, 1, 1), "{:?}", w.counts);
        assert_eq!(w.refold_tenders, vec![310]);
        assert_eq!(w.counts.skipped, 0, "no shape skips since tier 5: the structural tripwire");
        assert_eq!(rows_of(&conn, "SELECT COUNT(*) FROM tender_version_result_winners").await[0][0], Value::Integer(0));
        assert_eq!(rows_of(&conn, "SELECT projection_epoch FROM tenders WHERE id = 310").await[0][0], Value::Integer(0));
        assert_eq!(rows_of(&conn, "SELECT projected FROM notices WHERE id = 300").await[0][0], Value::Integer(0));
        let changed = rows_of(&conn, "SELECT entity_id FROM changes WHERE entity_kind = 'tender'").await;
        assert_eq!(changed, vec![vec![Value::Integer(310)]]);
        drop(conn);
        cleanup(&path);
    }

    /// The award a fold writes for `notice`'s Tender with `moved` on its mention ORG-0001 beside the
    /// sibling 200 on ORG-0002: both tender-scope winners, RES-1 won by `moved` and RES-2 by 200, and
    /// one bid on LOT-1 by both, ordered as the projection orders them.
    fn award_579(notice: i64, moved: i64) -> crate::canonical::TenderProjection {
        use crate::canonical::{BidParty, BidState, Fact, LotResultState, LotState, Round, TenderProjection, TenderVersion};
        let party = |org: i64, section: &str| Fact::Party {
            role: "winner".into(),
            organization_id: org,
            notice_id: notice,
            section_id: section.into(),
        };
        let result = |key: &str, lot: &str, org: i64| LotResultState {
            key: key.into(),
            lot_key: Some(lot.into()),
            decision: None,
            reason: None,
            awarded_cents: None,
            awarded_currency: None,
            decided: None,
            winners: vec![org],
            buyer_winners: Vec::new(),
            statistics: Vec::new(),
            published_lot_no: None,
            group: false,
        };
        let lot = |key: &str| LotState { key: key.into(), kind: "Lot".into(), facts: BTreeSet::new(), published_lot_no: None };
        let mut bidders = vec![
            BidParty { role: "tenderer".into(), organization_id: moved, section_id: "ORG-0001".into() },
            BidParty { role: "tenderer".into(), organization_id: 200, section_id: "ORG-0002".into() },
        ];
        bidders.sort();
        TenderProjection {
            source: "doe".into(),
            procedure_key: Some(format!("pk-579-{notice}")),
            island_notice_id: None,
            kind: "procedure".into(),
            versions: vec![TenderVersion {
                caused_by_notice_id: notice,
                published_at: 1_717_200_000,
                dispatched_at: None,
                notice_subtype: None,
                original_lang: None,
                publication_id: format!("{notice}"),
                facts: [party(moved, "ORG-0001"), party(200, "ORG-0002")].into_iter().collect(),
                lots: vec![lot("LOT-1"), lot("LOT-2")],
                rounds: vec![Round {
                    notice_id: notice,
                    logical_notice_id: None,
                    lot_results: vec![result("RES-1", "LOT-1", moved), result("RES-2", "LOT-2", 200)],
                    bids: vec![BidState {
                        key: "TEN-1".into(),
                        lot_key: Some("LOT-1".into()),
                        cents: None,
                        currency: None,
                        quality: None,
                        parties: bidders,
                    }],
                    contracts: Vec::new(),
                }],
                group_members: Vec::new(),
                stated_totals: Vec::new(),
            }],
        }
    }

    /// Issue 579 (d) through the wet window: the whole row 100 (its key minted, after the sibling 200)
    /// and the partial row 300's moved mention (onto the identifier-NULL 50, before it; its other mention
    /// kept) each leave their Tender's moved party, bid-party and winner rows where the fold would not
    /// write them, and the refold's compare reads rows in order. The window stamps both Tenders and
    /// re-queues their notices though no tier 5 ran, the rehome's Tender included; the trailing fold puts
    /// the six tables in its own order, and a later refold with the compare on rewrites none.
    #[tokio::test]
    async fn a_wet_window_then_a_refold_rewrites_no_table() {
        let (db, conn, path) = open("wet-579", true).await;
        db.set_refold_compare(crate::RefoldCompare::On);
        org_row(&conn, 50, None, None, "Amt 88", "DE").await;
        org_row(&conn, 100, Some("national"), Some(V3), "Amt 77", "DE").await;
        org_row(&conn, 200, Some("national"), Some("DE811128135"), "Werk GmbH", "DE").await;
        org_row(&conn, 300, Some("national"), Some(V5), "Amt 88", "DE").await;
        for n in [900, 901, 950] {
            notice_row(&conn, n, None).await;
        }
        exec(&conn, "UPDATE notices SET projected = 1", vec![]).await;
        mention_row(&conn, 900, "ORG-0001", 100, "Amt 77", "DE", Some(V3)).await;
        mention_row(&conn, 900, "ORG-0002", 200, "Werk GmbH", "DE", Some("DE811128135")).await;
        mention_row(&conn, 901, "ORG-0001", 300, "Amt 88", "DE", Some(V5)).await;
        mention_row(&conn, 901, "ORG-0002", 200, "Werk GmbH", "DE", Some("DE811128135")).await;
        mention_row(&conn, 950, "ORG-0001", 300, "Amt 88", "DE", Some(V5)).await;
        db.apply_tenders(&[award_579(900, 100), award_579(901, 300)], 1_717_300_000, false).await.unwrap();

        let window = GuidTwinWindow {
            keys: vec![("amt 77".into(), Some("DE".into())), ("amt 88".into(), Some("DE".into()))],
            whole: vec![100],
            partial: vec![300],
            mentions: vec![
                GuidTwinWetMention { org_value: GuidKeep::key(V3), ..wet(900, "ORG-0001", 100, "amt 77", None, true) },
                GuidTwinWetMention { org_value: GuidKeep::key(V5), ..wet(901, "ORG-0001", 300, "amt 88", Some(50), false) },
            ],
            ..Default::default()
        };
        let w = db
            .dissolve_guid_twin_window(GuidTwinWindowArgs { window: &window, job_id: 16, window_no: 1, gate_since_notice_id: None, fields: &[], normalize: lang })
            .await
            .unwrap();
        let minted = w.minted[0].target_id;
        assert!(minted > 300, "{minted}");
        assert_eq!((w.counts.parties, w.counts.bid_parties, w.counts.winners, w.counts.winners_deleted), (2, 2, 2, 0), "{:?}", w.counts);
        let tenders = rows_of(&conn, "SELECT id FROM tenders WHERE procedure_key LIKE 'pk-579-%' ORDER BY id").await;
        assert_eq!(w.refold_tenders, tenders.iter().map(|r| i(&r[0])).collect::<Vec<_>>(), "both Tenders, with no tier 5");
        assert_eq!((w.counts.refold_tenders, w.counts.tender_changes, w.counts.refold_notices), (2, 2, 2), "{:?}", w.counts);
        let stamped = rows_of(&conn, "SELECT projection_epoch FROM tenders WHERE procedure_key LIKE 'pk-579-%'").await;
        assert!(stamped.iter().all(|r| i(&r[0]) == 0), "{stamped:?}");
        let projected = rows_of(&conn, "SELECT id, projected FROM notices ORDER BY 1").await;
        assert_eq!(
            projected.iter().map(|r| (i(&r[0]), i(&r[1]))).collect::<Vec<_>>(),
            vec![(900, 0), (901, 0), (950, 1)],
            "the kept mention's notice causes no version"
        );

        // The trailing fold binds the moved mentions where they now sit.
        let trailing = db.apply_tenders(&[award_579(900, minted), award_579(901, 50)], 1_717_300_000, false).await.unwrap();
        assert_eq!(
            (trailing.tenders_corrected, trailing.tables_rewritten),
            (2, 6),
            "each Tender's parties, bid parties and winners put in the fold's order: {trailing:?}"
        );
        db.set_projection_epoch_for_test(-1).await.unwrap();
        let refold = db.apply_tenders(&[award_579(900, minted), award_579(901, 50)], 1_717_300_000, false).await.unwrap();
        assert_eq!(
            (refold.tenders_verified, refold.tenders_corrected, refold.tables_rewritten),
            (2, 0, 0),
            "the fold's fixed point: {refold:?}"
        );
        drop(conn);
        cleanup(&path);
    }

    /// The review keep-set is re-read on the writer INSIDE each window (as the 443 sweep reads it): a
    /// case review, or a rehoming verdict naming a whole row as its target, recorded after the re-plan
    /// refuses the window — rolled back whole, nothing logged, nothing announced.
    #[tokio::test]
    async fn a_verdict_recorded_after_the_re_plan_refuses_the_window() {
        let (db, conn, path) = open("wet-review", true).await;
        db.set_foreign_keys(false).await.unwrap();
        wet_seed(&conn).await;
        let stop = || false;
        let progress = |_: u64, _: &str| {};
        let empty: HashSet<i64> = HashSet::new();
        let plan = db
            .plan_guid_twins(GuidTwinPlanArgs { not_planned: &[], planned: &[], under_review: Some(&empty), ..args(&stop, &progress) })
            .await
            .unwrap();
        assert_eq!(plan.bars.tripwire_under_review, 0, "nothing under review at the re-plan");
        let windows = guid_twin_windows(&plan.wet_mentions, 2);
        let apply = || {
            let db = &db;
            let w = windows[0].clone();
            async move { db.dissolve_guid_twin_window(GuidTwinWindowArgs { window: &w, job_id: 16, window_no: 1, gate_since_notice_id: None, fields: &[], normalize: lang }).await }
        };
        exec(
            &conn,
            "INSERT INTO org_case_reviews (case_org_id, cohort, verdict, diagnosis, handling, rationale, confidence, reviewed_at)
             VALUES (102, 'c', 'v', 'd', 'h', 'r', 'high', 0)",
            vec![],
        )
        .await;
        let err = apply().await.expect_err("a verdict after the re-plan");
        assert!(err.to_string().contains("under review") && err.to_string().contains("[102]"), "{err}");
        exec(&conn, "DELETE FROM org_case_reviews", vec![]).await;
        exec(
            &conn,
            "INSERT INTO org_mention_rehoming (case_org_id, notice_id, section_id, cohort, action, target_org_id, rationale,
                                               confidence, reviewed_at) VALUES (5, 7, 'ORG-0001', 'c', 'rehome', 104, 'r', 'high', 0)",
            vec![],
        )
        .await;
        let err = apply().await.expect_err("a rehoming verdict's target");
        assert!(err.to_string().contains("[104]"), "{err}");
        assert_eq!(rows_of(&conn, "SELECT COUNT(*) FROM organizations WHERE id BETWEEN 101 AND 104").await[0][0], Value::Integer(4));
        assert_eq!(rows_of(&conn, "SELECT COUNT(*) FROM organization_mentions WHERE organization_id BETWEEN 101 AND 104").await[0][0], Value::Integer(4));
        assert!(db.guid_twin_log(16).await.unwrap().is_empty());
        assert_eq!(rows_of(&conn, "SELECT COUNT(*) FROM changes WHERE op = 'removed'").await[0][0], Value::Integer(0));
        exec(&conn, "DELETE FROM org_mention_rehoming", vec![]).await;
        apply().await.expect("no verdict: the window applies");
        drop(conn);
        cleanup(&path);
    }

    /// The manifest logs each dissolved row's pre-image as it IS, read inside the window: a v4 row stored
    /// under `identifier_kind = 'vat'` (issue 325's class G residue) is logged `vat`, never a literal
    /// `national`; a planned row re-keyed since the plan, or gone, refuses its window.
    #[tokio::test]
    async fn the_manifest_logs_the_rows_own_kind_and_refuses_a_changed_row() {
        let (db, conn, path) = open("wet-kind", true).await;
        db.set_foreign_keys(false).await.unwrap();
        org_row(&conn, 210, Some("vat"), Some(V8), "Amt 69", "DE").await;
        org_row(&conn, 211, Some("national"), Some(V9), "Amt 69", "DE").await;
        notice_row(&conn, 320, Some(330)).await;
        mention_row(&conn, 320, "ORG-0001", 210, "Amt 69", "DE", Some(V8)).await;
        mention_row(&conn, 320, "ORG-0002", 211, "Amt 69", "DE", Some(V9)).await;
        let m = |section: &str, org: i64, value: &str| GuidTwinWetMention { org_value: GuidKeep::key(value), ..wet(320, section, org, "amt 69", None, true) };
        let window = GuidTwinWindow { keys: vec![], whole: vec![210, 211], partial: vec![], mentions: vec![m("ORG-0001", 210, V8), m("ORG-0002", 211, V9)], ..Default::default() };
        let apply = |w: GuidTwinWindow| {
            let db = &db;
            async move { db.dissolve_guid_twin_window(GuidTwinWindowArgs { window: &w, job_id: 17, window_no: 1, gate_since_notice_id: None, fields: &[], normalize: lang }).await }
        };
        exec(&conn, "UPDATE organizations SET identifier = 'X' WHERE id = 211", vec![]).await;
        let err = apply(window.clone()).await.expect_err("re-keyed");
        assert!(err.to_string().contains("row 211 now carries identifier Some(\"X\")"), "{err}");
        exec(&conn, "UPDATE organizations SET identifier = ? WHERE id = 211", vec![Value::Text(GuidKeep::key(V9))]).await;
        // A planned row that is gone (its mention still points at it): refused.
        mention_row(&conn, 320, "ORG-0003", 999, "Amt 69", "DE", Some(V10)).await;
        let gone = GuidTwinWindow { keys: vec![], whole: vec![999], partial: vec![], mentions: vec![m("ORG-0003", 999, V10)], ..Default::default() };
        let err = apply(gone).await.expect_err("gone");
        assert!(err.to_string().contains("planned row 999 no longer exists"), "{err}");
        exec(&conn, "DELETE FROM organization_mentions WHERE section_id = 'ORG-0003'", vec![]).await;
        let w = apply(window).await.unwrap();
        assert_eq!(w.counts.dissolved, 2);
        let log = db.guid_twin_log(17).await.unwrap();
        let kinds: Vec<(i64, Option<String>, Option<String>)> = log.iter().map(|r| (r.org_id, r.org_kind.clone(), r.org_country.clone())).collect();
        assert_eq!(
            kinds,
            vec![(210, Some("vat".into()), Some("DE".into())), (211, Some("national".into()), Some("DE".into()))],
            "the kind the row carried"
        );
        assert_eq!(log[0].job_id, 17);
        drop(conn);
        cleanup(&path);
    }

    /// A fresh dry plan after a wet window: the planned controls the window dissolved whole onto their
    /// target are met through the dissolve log (`dissolved_by_wet`), so R1 passes and a later wet run is
    /// not wedged; a planned control logged onto ANOTHER target, or absent with no log row (it vanished
    /// another way), is still a miss; a not-planned control a wet run dissolved is a miss. The plan
    /// records the log's high-water mark.
    #[tokio::test]
    async fn a_fresh_plan_after_a_wet_window_meets_the_controls_it_dissolved() {
        let (db, conn, path) = open("wet-fresh", true).await;
        db.set_foreign_keys(false).await.unwrap();
        seed(&conn).await;
        let stop = || false;
        let progress = |_: u64, _: &str| {};
        let plan = db.plan_guid_twins(args(&stop, &progress)).await.unwrap();
        assert!(plan.bars.pass, "{:#?}", plan.bars);
        assert_eq!(plan.log_high_water, 0, "an empty log");
        for (k, w) in guid_twin_windows(&plan.wet_mentions, GUID_TWIN_WET_WINDOW_MENTIONS).iter().enumerate() {
            db.dissolve_guid_twin_window(GuidTwinWindowArgs { window: w, job_id: 18, window_no: k as u64 + 1, gate_since_notice_id: None, fields: &[], normalize: lang })
                .await
                .unwrap();
        }
        let fresh = db.plan_guid_twins(args(&stop, &progress)).await.unwrap();
        assert!(fresh.bars.planned_failed.is_empty(), "{:?}", fresh.bars.planned_failed);
        assert!(fresh.bars.pass, "{:#?}", fresh.bars);
        assert_eq!(fresh.log_high_water, 2, "two logged mentions");
        let c = fresh.controls.iter().find(|c| c.org_id == 22_759_261).unwrap();
        assert!(c.dissolved_by_wet && c.ok && !c.found, "{c:?}");
        let np = fresh.controls.iter().find(|c| c.org_id == 254).unwrap();
        assert!(np.ok && !np.dissolved_by_wet, "an absent not-planned control with no log row is met");
        let wrong = db
            .plan_guid_twins(GuidTwinPlanArgs { not_planned: &[254, 22_759_262], planned: &[(22_759_261, 2_294_259)], ..args(&stop, &progress) })
            .await
            .unwrap();
        assert_eq!(wrong.bars.not_planned_failed, vec![22_759_262], "a wet run dissolved a not-planned control");
        let elsewhere = db.plan_guid_twins(GuidTwinPlanArgs { planned: &[(22_759_261, 16_393)], ..args(&stop, &progress) }).await.unwrap();
        assert_eq!(elsewhere.bars.planned_failed, vec![22_759_261], "logged onto another target");
        let vanished = db
            .plan_guid_twins(GuidTwinPlanArgs { planned: &[(22_759_261, 2_294_259), (22_759_299, 2_294_259)], ..args(&stop, &progress) })
            .await
            .unwrap();
        assert_eq!(vanished.bars.planned_failed, vec![22_759_299], "absent with no log row");
        assert!(!vanished.bars.pass);
        drop(conn);
        cleanup(&path);
    }

    /// A1's reverse drift, STOCK half: a value added to KEEP after the dissolve (a reviewed KEEP-add) is
    /// bound by the rule now, but its mention stays on the row the window minted — a refold never moves it
    /// back. The arm lists it with its pre-image (the GUID row the window dissolved) and a `mint` restore;
    /// the restore mints ONE identified row (DE, national, the value, the mention's name), moves the
    /// mention and its party there, sends the winner whose chain now mixes a moved and a kept mention to
    /// tier 5 (its Tender stamped stale), and the drift reads 0 after it.
    #[tokio::test]
    async fn a_keep_add_after_the_dissolve_is_stock_reverse_drift_and_restores() {
        // Foreign keys ON on the Db writer, as in production: the restore's mint and rehome included.
        let (db, conn, path) = open("wet-drift", true).await;
        wet_seed(&conn).await;
        tender_row(&conn, 11).await;
        let stop = || false;
        let progress = |_: u64, _: &str| {};
        let plan = db.plan_guid_twins(GuidTwinPlanArgs { not_planned: &[], planned: &[], ..args(&stop, &progress) }).await.unwrap();
        for (k, w) in guid_twin_windows(&plan.wet_mentions, GUID_TWIN_WET_WINDOW_MENTIONS).iter().enumerate() {
            db.dissolve_guid_twin_window(GuidTwinWindowArgs { window: w, job_id: 19, window_no: k as u64 + 1, gate_since_notice_id: None, fields: &[], normalize: lang })
                .await
                .unwrap();
        }
        let m = i(&rows_of(&conn, "SELECT organization_id FROM organization_mentions WHERE notice_id = 1 AND section_id = 'ORG-0001'").await[0][0]);
        let read = |keep: GuidKeep| {
            let db = &db;
            async move {
                db.guid_twin_recurrence(GuidTwinRecurrenceArgs {
                    is_guid: uuid_v4,
                    gated_v4,
                    rule,
                    role_ref,
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
        assert_eq!(read(GuidKeep::default()).await.reverse_drift_mentions, 0, "the rule that moved them");
        let grown: GuidKeep = [V3].into_iter().collect();
        let r = read(grown.clone()).await;
        assert_eq!((r.reverse_drift_mentions, r.reverse_drift_stock, r.reverse_drift_since), (1, 1, 0), "{r:?}");
        let v = r.reverse_drift[0].clone();
        assert_eq!(
            (v.value.as_str(), v.restore, v.own_row, v.dissolved_org_id, v.name.as_str()),
            (GuidKeep::key(V3).as_str(), "mint", None, Some(101), "Amt 66")
        );
        assert_eq!(v.cohort, vec![(1, "ORG-0001".to_owned(), m)]);
        assert!(!r.pass);
        let d = db.restore_guid_twin_value(GuidTwinRestoreArgs { value: &v, fields: &[], normalize: lang }).await.unwrap();
        assert!(d.minted && d.target_id != m);
        assert_eq!(d.moved, vec![(1, "ORG-0001".to_owned(), m)]);
        assert_eq!(
            rows_of(&conn, "SELECT organization_id FROM tender_version_parties WHERE mention_notice_id = 1 AND mention_section_id = 'ORG-0001'").await,
            vec![vec![Value::Integer(d.target_id)]],
            "the party follows the mention key"
        );
        assert_eq!((d.counts.winners_deleted, d.refold_tenders.clone()), (1, vec![11]), "{:?}", d.counts);
        assert_eq!(rows_of(&conn, "SELECT projection_epoch FROM tenders WHERE id = 11").await[0][0], Value::Integer(0));
        let after = read(grown).await;
        assert_eq!(after.reverse_drift_mentions, 0, "{after:?}");
        assert!(after.pass, "{after:?}");
        drop(conn);
        cleanup(&path);
    }

    // ---- Build 2, review round 2: A1's halves, the restore's sweep, the residue refusal ----

    /// The recurrence read with the test mirrors (`window` 1,000).
    async fn recur(db: &crate::Db, keep: &GuidKeep, since: Option<i64>) -> GuidTwinRecurrence {
        let stop = || false;
        let progress = |_: u64, _: &str| {};
        db.guid_twin_recurrence(GuidTwinRecurrenceArgs {
            is_guid: uuid_v4,
            gated_v4,
            rule,
            role_ref,
            keep,
            since_notice_id: since,
            window: 1_000,
            samples: 10,
            stop: &stop,
            progress: &progress,
        })
        .await
        .unwrap()
    }

    /// Every window of a fresh plan (no controls), applied under `job`.
    async fn wet_all(db: &crate::Db, job: i64) -> Vec<GuidTwinWindowDone> {
        wet_all_with(db, job, None).await
    }

    /// [`wet_all`] under a given KEEP (`None`: the walk's), the bars not read — a fixture whose value
    /// spans two Tenders over two GUID rows (F4: the walk's KEEP holds it) is applied under an empty one.
    async fn wet_all_with(db: &crate::Db, job: i64, keep: Option<&GuidKeep>) -> Vec<GuidTwinWindowDone> {
        let stop = || false;
        let progress = |_: u64, _: &str| {};
        let plan = db
            .plan_guid_twins(GuidTwinPlanArgs { not_planned: &[], planned: &[], keep, ..args(&stop, &progress) })
            .await
            .unwrap();
        let mut done = Vec::new();
        for (k, w) in guid_twin_windows(&plan.wet_mentions, GUID_TWIN_WET_WINDOW_MENTIONS).iter().enumerate() {
            done.push(
                db.dissolve_guid_twin_window(GuidTwinWindowArgs {
                    window: w,
                    job_id: job,
                    window_no: k as u64 + 1,
                    gate_since_notice_id: Some(1),
                    fields: &[],
                    normalize: lang,
                })
                .await
                .unwrap(),
            );
        }
        done
    }

    /// A1's stock half reads only what the CURRENT gate admits: a logged mention still on its target whose
    /// raw no longer gates to a national v4 key (a condemn, a refused scheme, a re-parse) is no drift — a
    /// rebuild binds it through the 234 arm, where it sits — so the restore never moves it nor mints its old
    /// value; its twin partner, now alone in the copy, binds by identifier and IS drift. The window logged
    /// the boundary it was given.
    #[tokio::test]
    async fn a_logged_mention_the_gate_no_longer_admits_is_not_stock_drift() {
        let (db, conn, path) = open("wet-ungated", true).await;
        db.set_foreign_keys(false).await.unwrap();
        wet_seed(&conn).await;
        assert_eq!(db.guid_twin_recorded_boundary().await.unwrap(), None, "no window yet");
        wet_all(&db, 21).await;
        assert_eq!(db.guid_twin_recorded_boundary().await.unwrap(), Some(1), "every logged row carries the boundary");
        exec(&conn, "UPDATE organization_mentions SET raw_identifier = 'DE123456789' WHERE notice_id = 1 AND section_id = 'ORG-0001'", vec![])
            .await;
        let none = GuidKeep::default();
        let r = recur(&db, &none, None).await;
        assert_eq!((r.reverse_drift_stock_ungated, r.reverse_drift_stock, r.reverse_drift_mentions), (1, 1, 1), "{r:?}");
        let cohorts: Vec<(i64, String)> = r.reverse_drift.iter().flat_map(|v| v.cohort.iter().map(|(n, s, _)| (*n, s.clone()))).collect();
        assert_eq!(cohorts, vec![(1, "ORG-0002".to_owned())], "only the partner, no longer a twin, binds by identifier");
        assert!(r.reverse_drift.iter().all(|v| v.value != GuidKeep::key(V3)), "never the logged pre-image's value");
        for v in &r.reverse_drift {
            let d = db.restore_guid_twin_value(GuidTwinRestoreArgs { value: v, fields: &[], normalize: lang }).await.unwrap();
            assert!(d.moved.iter().all(|(n, s, _)| (*n, s.as_str()) != (1, "ORG-0001")), "{d:?}");
        }
        let v3 = format!("SELECT COUNT(*) FROM organizations WHERE identifier = '{}'", GuidKeep::key(V3));
        assert_eq!(rows_of(&conn, &v3).await[0][0], Value::Integer(0), "V3 is never re-minted");
        let after = recur(&db, &none, None).await;
        assert_eq!(after.reverse_drift_mentions, 0, "{after:?}");
        assert!(after.pass, "{after:?}");
        drop(conn);
        cleanup(&path);
    }

    /// FORWARD drift, reverse drift's mirror: after the wet run a re-parse that turns a copy into a twin
    /// re-resolves the changed sibling and KEEPS the unchanged one on its GUID row (434's `publishes`) — the
    /// rule refuses both, a rebuild binds both name-keyed. The recurrence arm counts it (stock: a refused
    /// section of a decided copy on a GUID row) and fails; a dry plan plans exactly that row, and a wet
    /// window on it clears the drift.
    #[tokio::test]
    async fn a_refused_sibling_left_on_its_guid_row_is_forward_drift() {
        let (db, conn, path) = open("wet-forward", true).await;
        db.set_foreign_keys(false).await.unwrap();
        wet_seed(&conn).await;
        wet_all(&db, 22).await;
        let none = GuidKeep::default();
        assert!(recur(&db, &none, None).await.pass, "the campaign leaves no drift");
        // The unchanged sibling: (1, ORG-0002) on a GUID row of its own value, where 434 kept it.
        org_row(&conn, 130, Some("national"), Some(V4), "Amt 66", "DE").await;
        exec(&conn, "UPDATE organization_mentions SET organization_id = 130 WHERE notice_id = 1 AND section_id = 'ORG-0002'", vec![]).await;
        let r = recur(&db, &none, None).await;
        assert_eq!((r.forward_drift_stock, r.forward_drift_since), (1, 0), "{r:?}");
        let f = &r.forward_drift[0];
        assert_eq!((f.notice_id, f.section_id.as_str(), f.org_id, f.kind), (1, "ORG-0002", 130, "guid_row"));
        assert_eq!(r.reverse_drift_mentions, 0, "the logged mention left its target: no reverse drift");
        assert!(!r.pass, "forward drift fails the arm");
        let after = wet_all(&db, 23).await;
        assert_eq!(after.iter().map(|w| w.whole).sum::<u64>(), 1, "a dry plan plans exactly the drifted row: {after:?}");
        assert_eq!(rows_of(&conn, "SELECT COUNT(*) FROM organizations WHERE id = 130").await[0][0], Value::Integer(0));
        assert!(recur(&db, &none, None).await.pass);
        drop(conn);
        cleanup(&path);
    }

    /// A restore that moves the last mentions off a row deletes it as the 443 sweep deletes one: a wet
    /// window mints ONE row for a missing key (two values, no identifier-NULL row), both values then join
    /// KEEP; the first value's restore leaves the minted row its other mention, the second empties it —
    /// gone with its names, its pre-image in `org_sweep_log`, `organization removed` and no `changed` from
    /// that restore. A row a party still names is emptied but kept, for `sweep-orphan-orgs`.
    #[tokio::test]
    async fn a_restore_that_empties_a_minted_row_sweeps_it() {
        let (db, conn, path) = open("wet-emptied", true).await;
        db.set_foreign_keys(false).await.unwrap();
        org_row(&conn, 140, Some("national"), Some(V8), "Amt 70", "DE").await;
        org_row(&conn, 141, Some("national"), Some(V9), "Amt 70", "DE").await;
        notice_row(&conn, 340, Some(350)).await;
        mention_row(&conn, 340, "ORG-0001", 140, "Amt 70", "DE", Some(V8)).await;
        mention_row(&conn, 340, "ORG-0002", 141, "Amt 70", "DE", Some(V9)).await;
        let windows = wet_all(&db, 24).await;
        let m = windows[0].minted[0].target_id;
        let grown: GuidKeep = [V8, V9].into_iter().collect();
        let r = recur(&db, &grown, None).await;
        assert_eq!((r.reverse_drift_stock, r.reverse_drift_values), (2, 2), "{r:?}");
        let d1 = db.restore_guid_twin_value(GuidTwinRestoreArgs { value: &r.reverse_drift[0], fields: &[], normalize: lang }).await.unwrap();
        assert!(d1.removed.is_empty() && d1.emptied_kept.is_empty(), "{d1:?}");
        exec(&conn, "INSERT INTO organization_names (org_id, lang, name, name_norm) VALUES (?, 'DEU', 'Amt 70', 'amt 70')", vec![Value::Integer(m)])
            .await;
        let changes_before = i(&rows_of(&conn, "SELECT COUNT(*) FROM changes").await[0][0]);
        let d2 = db.restore_guid_twin_value(GuidTwinRestoreArgs { value: &r.reverse_drift[1], fields: &[], normalize: lang }).await.unwrap();
        assert_eq!((d2.removed.clone(), d2.counts.dissolved), (vec![m], 1), "{d2:?}");
        assert_eq!(rows_of(&conn, &format!("SELECT COUNT(*) FROM organizations WHERE id = {m}")).await[0][0], Value::Integer(0));
        assert_eq!(rows_of(&conn, &format!("SELECT COUNT(*) FROM organization_names WHERE org_id = {m}")).await[0][0], Value::Integer(0));
        let logged = rows_of(&conn, &format!("SELECT name, names FROM org_sweep_log WHERE org_id = {m}")).await;
        assert_eq!(logged.len(), 1, "the 443 pre-image");
        assert!(matches!(&logged[0][1], Value::Text(n) if n.contains("Amt 70")), "{logged:?}");
        let ops = rows_of(&conn, &format!("SELECT op FROM changes WHERE entity_kind = 'organization' AND entity_id = {m} ORDER BY rowid")).await;
        assert_eq!(ops.last(), Some(&vec![Value::Text("removed".into())]), "{ops:?}");
        let fresh = rows_of(
            &conn,
            &format!("SELECT op FROM changes WHERE entity_kind = 'organization' AND entity_id = {m} AND rowid > {changes_before}"),
        )
        .await;
        assert_eq!(fresh, vec![vec![Value::Text("removed".into())]], "removed, never changed, by the emptying restore");
        let after = recur(&db, &grown, None).await;
        assert_eq!(after.reverse_drift_mentions, 0, "{after:?}");
        assert!(after.pass, "{after:?}");

        // Emptied but still named by a party row of another mention: kept, listed for the sweep.
        org_row(&conn, 142, None, None, "Amt 71", "DE").await;
        notice_row(&conn, 341, Some(351)).await;
        mention_row(&conn, 341, "ORG-0001", 142, "Amt 71", "DE", Some(V10)).await;
        mention_row(&conn, 341, "ORG-0002", 142, "Amt 71", "DE", Some(V11)).await;
        party_row(&conn, (351, 1), "Procedure-Buyer", 142, 341, "ORG-0009").await;
        let both: GuidKeep = [V10, V11].into_iter().collect();
        let r = recur(&db, &both, Some(340)).await;
        assert_eq!(r.reverse_drift_since, 2, "{r:?}");
        let mut kept = Vec::new();
        for v in &r.reverse_drift {
            let d = db.restore_guid_twin_value(GuidTwinRestoreArgs { value: v, fields: &[], normalize: lang }).await.unwrap();
            assert!(d.removed.is_empty(), "{d:?}");
            kept.extend(d.emptied_kept);
        }
        assert_eq!(kept, vec![142], "emptied, still referenced: left for sweep-orphan-orgs");
        assert_eq!(rows_of(&conn, "SELECT COUNT(*) FROM organizations WHERE id = 142").await[0][0], Value::Integer(1));
        drop(conn);
        cleanup(&path);
    }

    /// The recurrence arm (and so the restore's drift read and the wet run's) refuses before any read while
    /// an index it seeks is absent — the planner's set, and `organizations_identifier_id` for the own-row
    /// and identity seeks — as the planner does.
    #[tokio::test]
    async fn the_recurrence_arm_refuses_without_its_indexes() {
        let (db, conn, path) = open("wet-a1-idx", true).await;
        let none = GuidKeep::default();
        let stop = || false;
        let progress = |_: u64, _: &str| {};
        let read = || {
            db.guid_twin_recurrence(GuidTwinRecurrenceArgs {
                is_guid: uuid_v4,
                gated_v4,
                rule,
                role_ref,
                keep: &none,
                since_notice_id: None,
                window: 1_000,
                samples: 10,
                stop: &stop,
                progress: &progress,
            })
        };
        assert!(read().await.is_ok());
        exec(&conn, "DROP INDEX organizations_identifier_id", vec![]).await;
        let err = read().await.expect_err("no identifier index");
        assert!(err.to_string().contains("organizations_identifier_id is absent") && err.to_string().contains("Nothing was read"), "{err}");
        exec(&conn, "DROP INDEX organization_mentions_org", vec![]).await;
        let err = read().await.expect_err("no mention index");
        assert!(err.to_string().contains("organization_mentions_org is absent"), "{err}");
        drop(conn);
        cleanup(&path);
    }

    /// One value refused under two countries (A4 matches KEEP ignoring the country): a KEEP add drifts
    /// both, and they are two (value, country) entries — each restored onto its OWN identity row, as the
    /// resolver keys an identified row on (country, kind, value) — with each country's dissolved pre-image.
    #[tokio::test]
    async fn a_value_drifting_under_two_countries_restores_onto_two_rows() {
        let (db, conn, path) = open("wet-2c", true).await;
        db.set_foreign_keys(false).await.unwrap();
        org_row(&conn, 150, Some("national"), Some(V10), "Stadt Zwei", "DE").await;
        org_row(&conn, 151, Some("national"), Some(V11), "Stadt Zwei", "DE").await;
        org_row(&conn, 152, Some("national"), Some(V10), "Stadt Drei", "AT").await;
        org_row(&conn, 153, Some("national"), Some(V6), "Stadt Drei", "AT").await;
        notice_row(&conn, 360, Some(370)).await;
        mention_row(&conn, 360, "ORG-0001", 150, "Stadt Zwei", "DE", Some(V10)).await;
        mention_row(&conn, 360, "ORG-0002", 151, "Stadt Zwei", "DE", Some(V11)).await;
        notice_row(&conn, 361, Some(371)).await;
        mention_row(&conn, 361, "ORG-0001", 152, "Stadt Drei", "AT", Some(V10)).await;
        mention_row(&conn, 361, "ORG-0002", 153, "Stadt Drei", "AT", Some(V6)).await;
        // V10 sits on two GUID rows in two Tenders: the walk's KEEP holds it since F4 (per value), so the
        // stock is dissolved under an empty KEEP — the value refused, as before its KEEP add.
        let windows = wet_all_with(&db, 25, Some(&GuidKeep::default())).await;
        assert_eq!(windows.iter().map(|w| w.counts.dissolved).sum::<u64>(), 4);
        let grown: GuidKeep = [V10].into_iter().collect();
        let r = recur(&db, &grown, None).await;
        let pairs: Vec<(String, Option<String>, Option<i64>)> =
            r.reverse_drift.iter().map(|v| (v.value.clone(), v.country.clone(), v.dissolved_org_id)).collect();
        assert_eq!(
            pairs,
            vec![(GuidKeep::key(V10), Some("AT".into()), Some(152)), (GuidKeep::key(V10), Some("DE".into()), Some(150))],
            "one entry per (value, country), each with its own country's pre-image"
        );
        let mut targets = Vec::new();
        for v in &r.reverse_drift {
            let d = db.restore_guid_twin_value(GuidTwinRestoreArgs { value: v, fields: &[], normalize: lang }).await.unwrap();
            assert!(d.minted, "{d:?}");
            targets.push(d.target_id);
        }
        assert_ne!(targets[0], targets[1]);
        let rows = rows_of(
            &conn,
            &format!("SELECT country, identifier_kind, identifier FROM organizations WHERE identifier = '{}' ORDER BY country", GuidKeep::key(V10)),
        )
        .await;
        assert_eq!(
            rows,
            vec![
                vec![Value::Text("AT".into()), Value::Text("national".into()), Value::Text(GuidKeep::key(V10))],
                vec![Value::Text("DE".into()), Value::Text("national".into()), Value::Text(GuidKeep::key(V10))],
            ],
            "two identity rows, as a rebuild binds the two mentions"
        );
        let bound = rows_of(&conn, "SELECT notice_id, organization_id FROM organization_mentions WHERE section_id = 'ORG-0001' ORDER BY 1").await;
        assert_eq!(bound.iter().map(|r| i(&r[1])).collect::<Vec<_>>(), vec![targets[1], targets[0]], "DE onto DE's row, AT onto AT's");
        let after = recur(&db, &grown, None).await;
        assert_eq!(after.reverse_drift_mentions, 0, "{after:?}");
        assert!(after.pass, "{after:?}");
        drop(conn);
        cleanup(&path);
    }

    /// The restore's merged-identifier branch: a drifted value whose identity row is gone but which a merge
    /// folded into a survivor binds to that SURVIVOR (`organization_merged_identifiers.org_id`, never the
    /// deleted `loser`) — no identified row minted beside it, `organization changed` for it, no `added`.
    #[tokio::test]
    async fn the_restore_binds_a_merged_identifiers_survivor() {
        let (db, conn, path) = open("wet-merged", true).await;
        db.set_foreign_keys(false).await.unwrap();
        org_row(&conn, 970, Some("vat"), Some("DE111111111"), "Stadt Musterhausen", "DE").await;
        merged_row(&conn, V6, 970).await;
        org_row(&conn, 900, None, None, "Stadt Musterhausen", "DE").await;
        notice_row(&conn, 600, Some(60)).await;
        mention_row(&conn, 600, "ORG-0001", 900, "Stadt Musterhausen", "DE", Some(V6)).await;
        mention_row(&conn, 600, "ORG-0002", 900, "Stadt Musterhausen", "DE", Some(V7)).await;
        assert_eq!(recur(&db, &GuidKeep::default(), Some(599)).await.reverse_drift_mentions, 0, "both refused");
        let grown: GuidKeep = [V6, V7].into_iter().collect();
        let r = recur(&db, &grown, Some(599)).await;
        let v6 = r.reverse_drift.iter().find(|v| v.value == GuidKeep::key(V6)).expect("V6 drifts").clone();
        assert_eq!((v6.restore, v6.own_row, v6.cohort.clone()), ("own_row", Some(970), vec![(600, "ORG-0001".to_owned(), 900)]));
        let v7 = r.reverse_drift.iter().find(|v| v.value == GuidKeep::key(V7)).expect("V7 drifts");
        assert_eq!(v7.restore, "mint");
        let d = db.restore_guid_twin_value(GuidTwinRestoreArgs { value: &v6, fields: &[], normalize: lang }).await.unwrap();
        assert_eq!((d.minted, d.target_id, d.moved.clone()), (false, 970, vec![(600, "ORG-0001".to_owned(), 900)]));
        let v6_rows = format!("SELECT COUNT(*) FROM organizations WHERE identifier = '{}'", GuidKeep::key(V6));
        assert_eq!(rows_of(&conn, &v6_rows).await[0][0], Value::Integer(0), "no identified row minted beside the survivor");
        let changes = rows_of(&conn, "SELECT entity_kind, op, entity_id FROM changes ORDER BY 1, 2, 3").await;
        let org = |op: &str, id: i64| vec![Value::Text("organization".into()), Value::Text(op.into()), Value::Integer(id)];
        assert!(changes.contains(&org("changed", 970)) && changes.contains(&org("changed", 900)), "{changes:?}");
        assert!(!changes.contains(&org("added", 970)), "{changes:?}");
        drop(conn);
        cleanup(&path);
    }

    /// A whole planned row something still names after its mentions moved refuses its window — under the
    /// writer's production `foreign_keys = ON` — and keeps nothing: (a) a party row on the row keyed to
    /// ANOTHER org's mention (a nested alias) is the residue refusal; (b) a winner that row's party picks
    /// (its chosen mention is not one of the row's own) is a clear refusal too, never a winner repointed onto
    /// id 0. Either way: the row, its mentions, party and winner stand, nothing is logged or announced, and
    /// the mint is rolled back.
    #[tokio::test]
    async fn a_whole_row_still_referenced_refuses_its_window_and_keeps_nothing() {
        let (db, conn, path) = open("wet-residue", true).await;
        org_row(&conn, 160, Some("national"), Some(V7), "Amt 80", "DE").await;
        org_row(&conn, 161, None, None, "Andere Stelle", "DE").await;
        notice_row(&conn, 380, Some(390)).await;
        tender_row(&conn, 390).await;
        mention_row(&conn, 380, "ORG-0001", 160, "Amt 80", "DE", Some(V7)).await;
        mention_row(&conn, 380, "ORG-0003", 160, "Amt 80", "DE", Some(V7)).await;
        mention_row(&conn, 380, "ORG-0002", 161, "Andere Stelle", "DE", None).await;
        party_row(&conn, (390, 1), "Tenderer", 160, 380, "ORG-0002").await;
        let m = |section: &str| GuidTwinWetMention { org_value: GuidKeep::key(V7), ..wet(380, section, 160, "amt 80", None, true) };
        let window = GuidTwinWindow { keys: vec![], whole: vec![160], partial: vec![], mentions: vec![m("ORG-0001"), m("ORG-0003")], ..Default::default() };
        let apply = || {
            let db = &db;
            let w = window.clone();
            async move {
                db.dissolve_guid_twin_window(GuidTwinWindowArgs {
                    window: &w,
                    job_id: 26,
                    window_no: 1,
                    gate_since_notice_id: Some(1),
                    fields: &[],
                    normalize: lang,
                })
                .await
            }
        };
        let untouched = || async {
            assert_eq!(rows_of(&conn, "SELECT COUNT(*) FROM organizations").await[0][0], Value::Integer(2), "no mint survives");
            assert_eq!(rows_of(&conn, "SELECT COUNT(*) FROM organization_mentions WHERE organization_id = 160").await[0][0], Value::Integer(2));
            assert_eq!(rows_of(&conn, "SELECT organization_id FROM tender_version_parties").await, vec![vec![Value::Integer(160)]]);
            assert!(db.guid_twin_log(26).await.unwrap().is_empty());
            assert_eq!(rows_of(&conn, "SELECT COUNT(*) FROM changes").await[0][0], Value::Integer(0));
        };
        let err = apply().await.expect_err("a nested-alias party row");
        assert!(err.to_string().contains("is still referenced after its mentions moved"), "{err}");
        untouched().await;
        exec(&conn, "INSERT INTO lot_results (id, tender_id, notice_id, result_key) VALUES (30, 390, 380, 'RES-0001')", vec![]).await;
        exec(&conn, "INSERT INTO tender_version_result_winners (tender_id, seq, lot_result_id, organization_id) VALUES (390, 1, 30, 160)", vec![])
            .await;
        let err = apply().await.expect_err("a winner of another org's mention");
        assert!(err.to_string().contains("follows mention (380, ORG-0002), which is not one of its mentions"), "{err}");
        untouched().await;
        assert_eq!(rows_of(&conn, "SELECT organization_id FROM tender_version_result_winners").await, vec![vec![Value::Integer(160)]]);
        assert_eq!(rows_of(&conn, "SELECT COUNT(*) FROM tender_version_result_winners WHERE organization_id = 0").await[0][0], Value::Integer(0));
        drop(conn);
        cleanup(&path);
    }

    // ---- Review round 3: rehoming verdicts (F3), per-value Tenders (F4), A1's lower bound (F9) ----

    /// A reviewed rehoming verdict on `(notice, section)`, as `apply-rehoming` leaves it.
    async fn verdict_row(conn: &turso::Connection, case: i64, notice: i64, section: &str, action: &str, target: Option<i64>) {
        exec(
            conn,
            "INSERT INTO org_mention_rehoming (case_org_id, notice_id, section_id, cohort, action, target_org_id, rationale,
                                               confidence, reviewed_at, applied_at) VALUES (?, ?, ?, 'c', ?, ?, 'r', 'high', 0, 0)",
            vec![Value::Integer(case), Value::Integer(notice), Value::Text(section.into()), Value::Text(action.into()), target.map_or(Value::Null, Value::Integer)],
        )
        .await;
    }

    /// F3: after the wet run, a Risk-2 review rehomes one refused mention from its target onto the body's
    /// GUID row R. The copy still refuses it, and it now sits on a GUID row — yet a person put it there:
    /// the recurrence arm counts it `forward_drift_verdict_skipped`, not forward drift (pass holds), and a
    /// dry plan that reads the verdicts plans nothing (`verdict_kept` 1), where one blind to them plans R
    /// partial and would move the mention back. With R in ANOTHER Tender the blind plan trips the
    /// several-Tenders tripwire (every later wet run refused); the verdict-reading plan passes.
    #[tokio::test]
    async fn a_rehoming_verdicts_mention_is_neither_forward_drift_nor_planned() {
        let (db, conn, path) = open("wet-verdict", true).await;
        db.set_foreign_keys(false).await.unwrap();
        wet_seed(&conn).await;
        wet_all(&db, 27).await;
        let none = GuidKeep::default();
        assert!(recur(&db, &none, None).await.pass, "the campaign leaves no drift");
        // R = 22759261 (single-Tender, its TED mention kept): the reviewer rehomes (26483825, ORG-0002) onto it.
        verdict_row(&conn, 2_294_259, 26_483_825, "ORG-0002", "rehome", Some(22_759_261)).await;
        exec(&conn, "UPDATE organization_mentions SET organization_id = 22759261 WHERE notice_id = 26483825 AND section_id = 'ORG-0002'", vec![]).await;
        let r = recur(&db, &none, None).await;
        assert_eq!((r.forward_drift_stock, r.forward_drift_verdict_skipped), (0, 1), "{r:?}");
        assert!(r.pass, "{r:?}");
        let rehomed = db.guid_twin_rehomed_keys().await.unwrap();
        assert_eq!(rehomed, [(26_483_825, "ORG-0002".to_owned())].into_iter().collect::<HashSet<_>>());
        let stop = || false;
        let progress = |_: u64, _: &str| {};
        let plan_with = |rehomed: Option<&HashSet<(i64, String)>>| {
            let db = &db;
            let (stop, progress) = (&stop, &progress);
            let rehomed = rehomed.cloned();
            async move {
                db.plan_guid_twins(GuidTwinPlanArgs { not_planned: &[], planned: &[], rehomed: rehomed.as_ref(), ..args(stop, progress) })
                    .await
                    .unwrap()
            }
        };
        let blind = plan_with(None).await;
        assert_eq!(blind.partial_row_ids, vec![22_759_261], "blind to the verdict, R is planned and the review undone");
        let plan = plan_with(Some(&rehomed)).await;
        assert!(plan.planned_row_ids.is_empty() && plan.partial_row_ids.is_empty(), "{:?}", plan.partial_row_ids);
        assert_eq!(plan.verdict_kept, 1);
        assert_eq!(
            (plan.verdict_kept_samples[0].notice_id, plan.verdict_kept_samples[0].org_id, plan.verdict_kept_samples[0].kind),
            (26_483_825, 22_759_261, "guid_row")
        );
        assert!(plan.bars.pass, "{:#?}", plan.bars);
        // R from another Tender: row 170, named once by notice 400 (Tender 410).
        org_row(&conn, 170, Some("national"), Some(V7), "Landkreis Göttingen", "DE").await;
        notice_row(&conn, 400, Some(410)).await;
        mention_row(&conn, 400, "ORG-0001", 170, "Landkreis Göttingen", "DE", Some(V7)).await;
        exec(&conn, "UPDATE org_mention_rehoming SET target_org_id = 170", vec![]).await;
        exec(&conn, "UPDATE organization_mentions SET organization_id = 170 WHERE notice_id = 26483825 AND section_id = 'ORG-0002'", vec![]).await;
        let blind = plan_with(None).await;
        assert_eq!((blind.bars.tripwire_multi_tender, blind.bars.pass), (1, false), "blind: R spans two Tenders");
        let plan = plan_with(Some(&rehomed)).await;
        assert_eq!((plan.bars.tripwire_multi_tender, plan.verdict_kept), (0, 1));
        assert!(plan.bars.pass && plan.partial_row_ids.is_empty(), "{:#?}", plan.bars);
        let r = recur(&db, &none, None).await;
        assert_eq!((r.forward_drift_stock, r.forward_drift_verdict_skipped), (0, 1), "{r:?}");
        assert!(r.pass, "{r:?}");
        drop(conn);
        cleanup(&path);
    }

    /// F3, the window: a rehoming verdict recorded between the re-plan and a window on one of the window's
    /// planned mentions refuses the window — rolled back whole, nothing logged — and without it the window
    /// applies. The verdict is on the PARTIAL row's mention (a `keep` on 22759261's DÖE mention): the
    /// review keep-set re-read inside the window guards whole rows only, so before this check the window
    /// moved the reviewed mention to the target and undid the review.
    #[tokio::test]
    async fn a_verdict_on_a_planned_mention_after_the_re_plan_refuses_its_window() {
        let (db, conn, path) = open("wet-verdict-window", true).await;
        db.set_foreign_keys(false).await.unwrap();
        wet_seed(&conn).await;
        let stop = || false;
        let progress = |_: u64, _: &str| {};
        let plan = db.plan_guid_twins(GuidTwinPlanArgs { not_planned: &[], planned: &[], ..args(&stop, &progress) }).await.unwrap();
        let windows = guid_twin_windows(&plan.wet_mentions, 2);
        assert_eq!(windows[1].keys, vec![("landkreis göttingen".to_owned(), Some("DE".to_owned()))]);
        assert_eq!((windows[1].whole.clone(), windows[1].partial.clone()), (vec![22_759_262], vec![22_759_261]));
        let apply = || {
            let db = &db;
            let w = windows[1].clone();
            async move { db.dissolve_guid_twin_window(GuidTwinWindowArgs { window: &w, job_id: 28, window_no: 1, gate_since_notice_id: None, fields: &[], normalize: lang }).await }
        };
        verdict_row(&conn, 22_759_261, 26_483_825, "ORG-0001", "keep", None).await;
        let err = apply().await.expect_err("a verdict after the re-plan");
        assert!(err.to_string().contains("(26483825, ORG-0001) has a rehoming verdict") && err.to_string().contains("after the re-plan"), "{err}");
        let on = rows_of(&conn, "SELECT organization_id FROM organization_mentions WHERE notice_id = 26483825 ORDER BY section_id").await;
        assert_eq!(on, vec![vec![Value::Integer(22_759_261)], vec![Value::Integer(22_759_262)]], "rolled back: nothing moved");
        assert!(db.guid_twin_log(28).await.unwrap().is_empty());
        exec(&conn, "DELETE FROM org_mention_rehoming", vec![]).await;
        apply().await.expect("no verdict: the window applies");
        drop(conn);
        cleanup(&path);
    }

    /// F3, the stock half of reverse drift: a `keep` verdict a reviewer recorded for a logged mention on
    /// its logged target is no drift after a later KEEP add (the `since` half already skipped such a
    /// mention), so no restore moves it away.
    #[tokio::test]
    async fn a_keep_verdict_on_a_logged_mention_is_no_stock_reverse_drift() {
        let (db, conn, path) = open("wet-verdict-stock", true).await;
        db.set_foreign_keys(false).await.unwrap();
        wet_seed(&conn).await;
        wet_all(&db, 29).await;
        let grown: GuidKeep = [V3].into_iter().collect();
        let r = recur(&db, &grown, None).await;
        assert_eq!((r.reverse_drift_stock, r.reverse_drift_verdict_skipped), (1, 0), "{r:?}");
        let on = i(&rows_of(&conn, "SELECT organization_id FROM organization_mentions WHERE notice_id = 1 AND section_id = 'ORG-0001'").await[0][0]);
        verdict_row(&conn, on, 1, "ORG-0001", "keep", None).await;
        let r = recur(&db, &grown, None).await;
        assert_eq!((r.reverse_drift_stock, r.reverse_drift_verdict_skipped, r.reverse_drift_mentions), (0, 1, 0), "{r:?}");
        assert!(r.pass, "{r:?}");
        drop(conn);
        cleanup(&path);
    }

    /// F4: one value on two GUID rows — issue 62's duplicate triple (two DE rows), and a DE row beside an AT
    /// row — each whole-refused in a twin copy of a DIFFERENT Tender. Per row each is single-Tender, but the
    /// value links two Tenders: the several-Tenders tripwire counts both rows, the compiled self-check lists
    /// the value ONCE (its lowest-id row's line, the union's Tenders), `multi_row_values` names it, and the
    /// walk's KEEP candidates hold it (so a walk-KEEP plan plans neither row); with the value in KEEP neither
    /// row is planned and the plan passes. When the lower-id row is untouched (named once) and only the
    /// higher one is planned, in another Tender, the tripwire still fires.
    #[tokio::test]
    async fn a_value_on_two_guid_rows_counts_its_tenders_per_value() {
        for second_country in ["DE", "AT"] {
            let (db, conn, path) = open(&format!("multi-row-{second_country}"), true).await;
            db.set_foreign_keys(false).await.unwrap();
            org_row(&conn, 180, Some("national"), Some(V7), "Stadt Eins", "DE").await;
            org_row(&conn, 181, Some("national"), Some(V7), "Stadt Eins", second_country).await;
            org_row(&conn, 182, Some("national"), Some(V8), "Stadt Eins", "DE").await;
            org_row(&conn, 183, Some("national"), Some(V9), "Stadt Eins", second_country).await;
            notice_row(&conn, 700, Some(710)).await;
            mention_row(&conn, 700, "ORG-0001", 180, "Stadt Eins", "DE", Some(V7)).await;
            mention_row(&conn, 700, "ORG-0002", 182, "Stadt Eins", "DE", Some(V8)).await;
            notice_row(&conn, 701, Some(711)).await;
            mention_row(&conn, 701, "ORG-0001", 181, "Stadt Eins", second_country, Some(V7)).await;
            mention_row(&conn, 701, "ORG-0002", 183, "Stadt Eins", second_country, Some(V9)).await;
            let stop = || false;
            let progress = |_: u64, _: &str| {};
            let plan = |keep: Option<GuidKeep>| {
                let db = &db;
                let (stop, progress) = (&stop, &progress);
                async move {
                    db.plan_guid_twins(GuidTwinPlanArgs {
                        keep: keep.as_ref(),
                        keep_version: "keep-test",
                        not_planned: &[],
                        planned: &[],
                        exhibits: &[],
                        ..args(stop, progress)
                    })
                    .await
                    .unwrap()
                }
            };
            let empty = plan(Some(GuidKeep::default())).await;
            assert_eq!(empty.planned_row_ids, vec![180, 181, 182, 183], "{second_country}");
            assert_eq!(empty.bars.tripwire_multi_tender, 2, "{second_country}: both rows of the value");
            assert_eq!(empty.bars.keep_self_check_missing, vec![GuidKeep::key(V7)], "{second_country}: listed once");
            assert_eq!(empty.keep_add_commit, vec![format!("DE\t{}\t180\t1\t2\tmulti_tender", GuidKeep::key(V7))], "{second_country}");
            assert_eq!(
                empty.multi_row_values,
                vec![GuidTwinMultiRowValue { value: GuidKeep::key(V7), rows: vec![180, 181], row_tenders: vec![1, 1], tenders: 2 }],
                "{second_country}"
            );
            assert_eq!(empty.multi_row_value_count, 1);
            assert!(!empty.bars.pass, "{second_country}");
            let walk = plan(None).await;
            assert_eq!(walk.planned_row_ids, vec![182, 183], "{second_country}: the walk's KEEP holds the value");
            let candidate = walk.keep_candidates.iter().find(|c| c.value == GuidKeep::key(V7)).expect("a walk candidate");
            assert_eq!((candidate.org_id, candidate.tenders, candidate.reason.as_str()), (180, Some(2), "multi_tender"));
            let kept = plan(Some([V7].into_iter().collect())).await;
            assert_eq!(kept.planned_row_ids, vec![182, 183], "{second_country}");
            assert_eq!((kept.bars.tripwire_multi_tender, kept.bars.keep_self_check_missing.len()), (0, 0));
            assert!(kept.bars.pass, "{second_country}: {:#?}", kept.bars);
            drop(conn);
            cleanup(&path);
        }
        // The lower-id row untouched (named once, Tender 730), the higher one planned in Tender 731.
        let (db, conn, path) = open("multi-row-untouched", true).await;
        db.set_foreign_keys(false).await.unwrap();
        org_row(&conn, 190, Some("national"), Some(V10), "Stadt Zwei", "DE").await;
        org_row(&conn, 191, Some("national"), Some(V10), "Stadt Zwei", "DE").await;
        org_row(&conn, 192, Some("national"), Some(V11), "Stadt Zwei", "DE").await;
        notice_row(&conn, 720, Some(730)).await;
        mention_row(&conn, 720, "ORG-0001", 190, "Stadt Zwei", "DE", Some(V10)).await;
        notice_row(&conn, 721, Some(731)).await;
        mention_row(&conn, 721, "ORG-0001", 191, "Stadt Zwei", "DE", Some(V10)).await;
        mention_row(&conn, 721, "ORG-0002", 192, "Stadt Zwei", "DE", Some(V11)).await;
        let stop = || false;
        let progress = |_: u64, _: &str| {};
        let empty = GuidKeep::default();
        let plan = db
            .plan_guid_twins(GuidTwinPlanArgs { keep: Some(&empty), keep_version: "keep-test", not_planned: &[], planned: &[], exhibits: &[], ..args(&stop, &progress) })
            .await
            .unwrap();
        assert_eq!(plan.planned_row_ids, vec![191, 192]);
        assert_eq!(plan.bars.tripwire_multi_tender, 1, "the planned row's value already links Tender 730");
        assert_eq!(plan.bars.tripwire_rows, vec![191]);
        assert_eq!(plan.keep_add_commit, vec![format!("DE\t{}\t190\t1\t2\tmulti_tender", GuidKeep::key(V10))], "the lowest-id row's line");
        assert!(!plan.bars.pass);
        drop(conn);
        cleanup(&path);
    }

    /// F4 at A1: a refused value's Tenders count EVERY GUID row holding it — its lowest-id row shares the
    /// refused mention's Tender, its second row adds another, so the value is a KEEP-add candidate (with
    /// the lowest row as `own_row` and both in `own_rows`); reading the lowest row alone it was not.
    #[tokio::test]
    async fn the_recurrence_arm_counts_every_guid_row_of_a_value() {
        let (db, conn, path) = open("wet-a1-rows", true).await;
        db.set_foreign_keys(false).await.unwrap();
        org_row(&conn, 900, None, None, "Stadt Musterhausen", "DE").await;
        notice_row(&conn, 500, Some(50)).await;
        mention_row(&conn, 500, "ORG-0001", 900, "Stadt Musterhausen", "DE", Some(V1)).await;
        mention_row(&conn, 500, "ORG-0002", 900, "Stadt Musterhausen", "DE", Some(V2)).await;
        org_row(&conn, 960, Some("national"), Some(V2), "Stadt Beispielhausen", "DE").await;
        org_row(&conn, 961, Some("national"), Some(V2), "Stadt Beispielhausen", "AT").await;
        notice_row(&conn, 505, None).await;
        version_row(&conn, 50, 2, 505).await;
        mention_row(&conn, 505, "ORG-0001", 960, "Stadt Beispielhausen", "DE", Some(V2)).await;
        notice_row(&conn, 506, Some(54)).await;
        mention_row(&conn, 506, "ORG-0001", 961, "Stadt Beispielhausen", "AT", Some(V2)).await;
        let r = recur(&db, &GuidKeep::default(), Some(499)).await;
        assert_eq!(r.candidate_count, 1, "{r:?}");
        let c = &r.candidates[0];
        assert_eq!(
            (c.value.as_str(), c.tenders, c.own_row, c.own_rows.clone(), c.own_row_mentions),
            (GuidKeep::key(V2).as_str(), 2, Some(960), vec![960, 961], 2)
        );
        drop(conn);
        cleanup(&path);
    }

    /// F9: before any wet window, a post-boundary copy whose refused section sits on a GUID row was folded
    /// before the gate. The recurrence arm reports the highest such notice and each such row, and skips a
    /// verdict-placed one.
    #[tokio::test]
    async fn a_post_boundary_copy_on_guid_rows_names_its_highest_notice() {
        let (db, conn, path) = open("wet-a1-low", true).await;
        db.set_foreign_keys(false).await.unwrap();
        wet_seed(&conn).await;
        let none = GuidKeep::default();
        let stop = || false;
        let progress = |_: u64, _: &str| {};
        let read = || {
            db.guid_twin_recurrence(GuidTwinRecurrenceArgs {
                is_guid: uuid_v4,
                gated_v4,
                rule,
                role_ref,
                keep: &none,
                since_notice_id: Some(0),
                window: 100_000_000,
                samples: 10,
                stop: &stop,
                progress: &progress,
            })
        };
        let r = read().await.unwrap();
        assert_eq!((r.forward_drift_since, r.forward_drift_since_max_notice), (6, Some(26_483_825)), "{r:?}");
        assert_eq!(r.forward_drift_since_rows.keys().copied().collect::<Vec<_>>(), vec![101, 102, 103, 104, 22_759_261, 22_759_262]);
        assert_eq!((r.forward_drift_since_rows[&101], r.forward_drift_since_rows[&22_759_261]), (1, 26_483_825));
        verdict_row(&conn, 22_759_262, 26_483_825, "ORG-0002", "keep", None).await;
        let r = read().await.unwrap();
        assert_eq!((r.forward_drift_since, r.forward_drift_verdict_skipped), (5, 1), "{r:?}");
        assert!(!r.forward_drift_since_rows.contains_key(&22_759_262));
        assert_eq!(r.forward_drift_since_residue, 0, "a pre-gate fold carries no name-keyed sibling: {r:?}");
        drop(conn);
        cleanup(&path);
    }

    /// F9, checked against a refresh residue: a post-boundary copy a re-parse turned into a twin AFTER the
    /// gate — its changed sibling re-resolved onto the name-keyed row, its unchanged one kept on its GUID
    /// row — is forward drift (counted, `forward_drift_since_residue`), never a pre-gate fold: no row and no
    /// highest notice for the low-boundary check. A name-keyed sibling a rehoming verdict placed is no
    /// gate binding, so the copy reads as folded before the gate again.
    #[tokio::test]
    async fn a_refresh_residue_is_forward_drift_but_no_pre_gate_fold() {
        let (db, conn, path) = open("wet-a1-residue", true).await;
        db.set_foreign_keys(false).await.unwrap();
        org_row(&conn, 900, None, None, "Stadt Musterhausen", "DE").await;
        org_row(&conn, 960, Some("national"), Some(V2), "Stadt Musterhausen", "DE").await;
        notice_row(&conn, 500, Some(50)).await;
        mention_row(&conn, 500, "ORG-0001", 900, "Stadt Musterhausen", "DE", Some(V1)).await;
        mention_row(&conn, 500, "ORG-0002", 960, "Stadt Musterhausen", "DE", Some(V2)).await;
        let none = GuidKeep::default();
        let r = recur(&db, &none, Some(0)).await;
        assert_eq!((r.forward_drift_since, r.forward_drift_since_residue), (1, 1), "{r:?}");
        assert_eq!(r.forward_drift_since_max_notice, None, "{r:?}");
        assert!(r.forward_drift_since_rows.is_empty(), "{r:?}");
        assert_eq!(r.forward_drift[0].org_id, 960, "still sampled as forward drift");
        assert!(!r.pass, "the residue is real drift: {r:?}");
        verdict_row(&conn, 900, 500, "ORG-0001", "keep", None).await;
        let r = recur(&db, &none, Some(0)).await;
        assert_eq!((r.forward_drift_since, r.forward_drift_since_residue, r.forward_drift_since_max_notice), (1, 0, Some(500)), "{r:?}");
        assert_eq!(r.forward_drift_since_rows.get(&960), Some(&500));
        drop(conn);
        cleanup(&path);
    }
}
