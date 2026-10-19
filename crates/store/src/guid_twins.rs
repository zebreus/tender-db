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
    /// more Tenders; stable at R1's fixed threshold ([`GUID_TWIN_R1_STABLE_MENTIONS`] mentions, or a
    /// stable id) whatever the run's knob; a value in KEEP.
    pub tripwire_multi_tender: u64,
    pub tripwire_stable: u64,
    pub tripwire_keep: u64,
    /// Planned rows spanning no Tender (their notices cause no version). Reported, not a bar: the
    /// decision's R1 tripwires are the three above.
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
    /// A3 iii: (census twin ∧ ≥ 2 Tenders) ∪ stable values missing from KEEP (tautological in Build 1).
    pub keep_self_check_missing: Vec<String>,
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
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct GuidTwinExhibit {
    pub guid_rows: u64,
    pub planned: u64,
    pub partial: u64,
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
}

impl Walk {
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
            ..Default::default()
        };
        let reader = self.reader().await?;
        let mut rows = reader.query(GUID_CENSUS_MAX_ID_SQL, ()).await?;
        plan.max_id = match rows.next().await? {
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
            let mut reasons = Vec::new();
            if row.stable {
                reasons.push("stable");
            }
            if row.tenders >= 2 {
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
                tenders: Some(row.tenders),
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
        let keep: GuidKeep = candidates.keys().collect();
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
                .filter(|(_, mentions)| guid_twin_has_group(&args, &keep, mentions))
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
                guid_twin_notice(&args, &keep, &empty_keep, &mut walk, &mut plan, notice, &mentions, section_roles);
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
        drop(reader);

        guid_twin_finish(&args, &mut plan, &mut walk, &identified, &survivors, &roles, &keep, &stable_ids);
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
    // The census twin over GUID-bound mentions (no country in the key), now and after the plan.
    let mut by_norm: HashMap<String, Vec<(u32, bool)>> = HashMap::new();
    for m in mentions {
        let Some(&r) = walk.row_at.get(&m.org) else { continue };
        let norm = org_name_norm(&m.name);
        if norm.is_empty() {
            continue;
        }
        by_norm.entry(norm).or_default().push((r, decision.refused.contains(&m.section)));
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

/// The plan's tallies, screens, bars and samples, from the finished walk.
#[allow(clippy::too_many_arguments)]
fn guid_twin_finish(
    args: &GuidTwinPlanArgs<'_>,
    plan: &mut GuidTwinPlan,
    walk: &mut Walk,
    identified: &HashMap<i64, Option<String>>,
    survivors: &HashSet<i64>,
    roles: &HashMap<(i64, String), BTreeSet<String>>,
    keep: &GuidKeep,
    stable_ids: &HashSet<i64>,
) {
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
        // A3 iii's self-check (tautological in Build 1: KEEP comes from this walk).
        if ((row.twin && row.tenders >= 2) || row.stable) && !keep.contains(&row.identifier) {
            plan.bars.keep_self_check_missing.push(GuidKeep::key(&row.identifier));
        }
        if row.refused == 0 {
            continue;
        }
        // The tripwires, over every planned row. The stable one reads R1's fixed threshold, never the
        // run's knob (`row.stable`), so a raised knob cannot hide a 100-mention row from it.
        let multi = row.tenders >= 2;
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
    // The controls.
    let control = |org: i64, expect: &'static str, expected_target: Option<i64>| {
        let row = walk.row_at.get(&org).map(|&r| row_sample(&walk.rows[r as usize]));
        let ok = match (expect, &row) {
            ("not_planned", None) => true,
            ("not_planned", Some(r)) => r.planned == "no",
            (_, Some(r)) => {
                r.planned == "whole"
                    && match expected_target {
                        Some(t) => r.targets == vec![Some(t)],
                        None => r.targets.iter().all(Option::is_some),
                    }
            }
            (_, None) => false,
        };
        GuidTwinControl { org_id: org, expect, expected_target, found: row.is_some(), row, ok }
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
    // The decision's bars (`tripwire_no_tender` and `target_country_unchecked` are reported only).
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
        && b.role_ref_disagree == 0;
    plan.bars.pass = pass;
    for reason in TwinBind::ALL {
        plan.guard.entry(reason.label()).or_default();
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
}
