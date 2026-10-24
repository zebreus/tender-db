//! Issue 575 unit 1: the `hex-census` — the raw 8-4-4-16 BT-501 shape ("cf35", Avenue-Web's eSender key,
//! minted fresh per ORG section per notice) over every organization whose stored identifier is 32 hex
//! digits, with the bars unit 2 decides on. Writes NOTHING. Store never depends on ingest: the hex_hash
//! class (`idgate::census`), `idgate::uuid_v4`, the role mapping and KEEP_530 are injected.
//!
//! The stored identifier cannot tell the shape: `normalise_identifier_with` strips the dashes, so an
//! 8-4-4-16 value and an RFC 4122 one both store as 32 upper-case hex. Only
//! `organization_mentions.raw_identifier` keeps it, so the census classifies the raw value of every
//! mention ([`hex_raw_shape`], the one classifier unit 3's gate calls through [`is_cf35`]). A 32-hex row
//! with at least one cf35 mention is a CLASS row. The census reports:
//!
//! 1. the class's rows, mentions, notices and Tenders, single-use / multi-notice / multi-Tender (per row
//!    and per value) / stable, by country, first year, source, profile and role stratum;
//! 2. every ORG mention of each notice holding a cf35 mention: shape × role, shape × scheme, the buyer
//!    bar, blank names, the within-notice twins (option b's reach) and Avenue-Web's tenderer keys (U5);
//! 3. each cf35 mention's name-keyed fallback target (the resolver's `(org_name_norm(name),
//!    register_jurisdiction(country))`, identifier NULL, lowest id) with 530's A6 city screen;
//! 4. R2's random draw and the whole lists every unit 2 bar names.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use serde::Serialize;
use turso::Value;

use crate::canonical::{IN_CHUNK, org_name_norm, register_jurisdiction, unix_year};
use crate::guid_census::{
    GUID_CENSUS_MAX_ID_SQL, GUID_CENSUS_REST, GUID_ROLE_BUYER, GUID_ROLE_REVIEW_INFO, GUID_ROLE_TENDERER,
    guid_census_mentions_sql, guid_census_notices_sql, guid_census_tenders_sql,
};
use crate::guid_twins::{
    GUID_TWIN_CITY_FIELD, GUID_TWIN_LIST_CAP, GUID_TWIN_TARGET_COUNT_SQL, GUID_TWIN_TARGET_NEWEST,
    GUID_TWIN_TARGET_NEWEST_SQL, GuidKeep, guid_twin_bid_parties_sql, guid_twin_cities_sql, guid_twin_city_fold,
    guid_twin_notice_mentions_sql, guid_twin_parties_sql, guid_twin_role_after_scope, guid_twin_sample_key,
    guid_twin_targets_sql,
};
use crate::{Db, int, opt_int_of, opt_text_of, text};

/// Issue 575 unit 1: organization ids per window of the walk (the GUID census's).
pub const HEX_CENSUS_WINDOW: i64 = 10_000;
/// Issue 575 unit 1: 32-hex rows per batch; every IN chunk is at most [`IN_CHUNK`] keys.
pub const HEX_CENSUS_BATCH: usize = IN_CHUNK;
/// Issue 575 unit 1: R2's salt (unit 2's random draw).
pub const HEX_CENSUS_SALT: u64 = 575_001;
/// Issue 575 unit 1: R2's random strata (unit 2: "a random 60 stratified by role (SProvider, review,
/// mediator, AddInfo)").
pub const HEX_CENSUS_R2_STRATA: [(&str, usize); 4] = [("sprovider", 15), ("review", 15), ("mediator", 15), ("add_info", 15)];
/// Issue 575 unit 1: the targets listed by cf35 mentions (unit 2 reads the top 30).
pub const HEX_CENSUS_TOP_TARGETS: usize = 50;
/// Issue 575 unit 1: the most keys a capped breakdown names, the largest; the rest is summed under
/// [`GUID_CENSUS_REST`].
pub const HEX_CENSUS_KEYS_MAX: usize = 60;
/// Issue 575 unit 1: a raw BT-501's shape ([`hex_raw_shape`]), in report order.
pub const HEX_CENSUS_SHAPES: [&str; 7] = ["cf35", "rfc36", "hex32", "tenderer_key", "digits", "other", "none"];
/// Issue 575 unit 1: a cf35 mention's role stratum, in PRECEDENCE order ([`hex_stratum`]).
pub const HEX_CENSUS_STRATA: [&str; 8] = ["buyer", "add_info", "sprovider", "mediator", "review", "tenderer", "other", "no_role"];
/// Issue 575 unit 1: the CCIRA keys listed whole (unit 2 reads every homonym), matched on
/// `guid_twin_city_fold(name_norm)`, so the short form, the SGAR form and the fixture's long form
/// ("… de réglement amiable des différends …") all match.
pub const HEX_CENSUS_CCIRA_PREFIX: &str = "comite consultatif interregional";
/// Issue 575 unit 1: the indexes every read seeks; the census refuses before reading without one.
pub const HEX_CENSUS_INDEXES: [&str; 5] = [
    "organization_mentions_org",
    "tender_versions_notice",
    "tender_version_parties_mention_key",
    "tender_version_bid_parties_mention_key",
    "organizations_name_country",
];
/// Issue 575 unit 1: one window of the walk — a rowid range, the identifier's length a residual filter
/// (16+: the hex_hash class's floor, so the window also re-takes issue 300's total), so a window reads at
/// most [`HEX_CENSUS_WINDOW`] rows.
pub(crate) const HEX_CENSUS_ORGS_SQL: &str =
    "SELECT id, country, identifier, name FROM organizations WHERE id > ? AND id <= ? AND length(identifier) >= 16";

/// The report's `not_measured`, exactly.
const HEX_CENSUS_NOT_MEASURED: [&str; 3] = [
    "corpus_eu_scheme_total: 365's 252,658 is unscoped and needs a full organization_mentions scan; by_shape_scheme counts the scheme inside the cf35 notices",
    "cf35 mentions on rows whose stored identifier is not 32 hex, outside the cf35 notices (merge survivors); cf35_off_class counts those inside them",
    "the A6 reverse walk beyond each target's newest 50 mentions",
];

/// Issue 575 unit 1: a raw BT-501's shape (one of [`HEX_CENSUS_SHAPES`]), trimmed and case-blind:
/// `cf35` = 35 characters, dashes at 8, 13 and 18, hex elsewhere (ColdFusion `CreateUUID`, 8-4-4-16);
/// `rfc36` = 36 characters, dashes at 8, 13, 18 and 23; `hex32` = 32 hex; `tenderer_key` = four
/// dash-separated digit groups (Avenue-Web's `<procedure>-1-<lot>-<n>`); `digits` = all digits; `other`;
/// `none` = NULL or blank.
pub fn hex_raw_shape(raw: Option<&str>) -> &'static str {
    let v = raw.map(str::trim).unwrap_or_default();
    if v.is_empty() {
        return "none";
    }
    let b = v.as_bytes();
    if b.len() == 35 && hex_dashed_at(b, &[8, 13, 18]) {
        return "cf35";
    }
    if b.len() == 36 && hex_dashed_at(b, &[8, 13, 18, 23]) {
        return "rfc36";
    }
    if b.len() == 32 && b.iter().all(u8::is_ascii_hexdigit) {
        return "hex32";
    }
    let groups: Vec<&str> = v.split('-').collect();
    if groups.len() == 4 && groups.iter().all(|g| !g.is_empty() && g.bytes().all(|c| c.is_ascii_digit())) {
        return "tenderer_key";
    }
    if b.iter().all(u8::is_ascii_digit) {
        return "digits";
    }
    "other"
}

/// A dash at each of `dashes` and a hex digit everywhere else.
fn hex_dashed_at(b: &[u8], dashes: &[usize]) -> bool {
    b.iter().enumerate().all(|(i, c)| if dashes.contains(&i) { *c == b'-' } else { c.is_ascii_hexdigit() })
}

/// Issue 575: the platform key's published shape (unit 3's gate calls this, so the census and the gate
/// agree by construction).
pub fn is_cf35(raw: &str) -> bool {
    hex_raw_shape(Some(raw)) == "cf35"
}

/// Issue 575 unit 1: a mention's stratum from its sorted, distinct roles (parties as stored, bid parties
/// prefixed `bid-`), in [`HEX_CENSUS_STRATA`] precedence. eForms roles are read after their scope
/// (`guid_twin_role_after_scope`: `Lot-`/`Part-`/`Procedure-`); the legacy names (`EFORMS_ROLE_MEANING`)
/// map alike (`further-information`, `service-provider`, `mediation-body`, `review-body`,
/// `appeal-information`). `buyer` and `tenderer` come from the injected `role_class`; `review` also takes
/// every other role `role_class` puts in `review_or_info` (`Lot-TenderReceipt`, the legislation bodies).
pub fn hex_stratum(roles: &[String], role_class: fn(&str) -> &'static str) -> &'static str {
    // Each role takes the first stratum whose test it meets; the mention the lowest of those, which is
    // "the first stratum any role meets".
    let mut best = if roles.is_empty() { 7 } else { 6 };
    for role in roles {
        let r = role.as_str();
        let (after, class) = (guid_twin_role_after_scope(r), role_class(r));
        let stratum = if class == GUID_ROLE_BUYER {
            0
        } else if after == "AddInfo" || r == "further-information" {
            1
        } else if after == "SProvider" || r == "service-provider" {
            2
        } else if after == "Mediator" || r == "mediation-body" {
            3
        } else if matches!(after, "ReviewOrg" | "ReviewInfo")
            || matches!(r, "review-body" | "appeal-information")
            || class == GUID_ROLE_REVIEW_INFO
        {
            4
        } else if class == GUID_ROLE_TENDERER || r.starts_with("bid-") {
            5
        } else {
            6
        };
        best = best.min(stratum);
    }
    HEX_CENSUS_STRATA[best]
}

fn hex32(v: &str) -> bool {
    v.len() == 32 && v.bytes().all(|b| b.is_ascii_hexdigit())
}

/// A scheme as reported: trimmed, lower-cased, "(none)" for NULL/blank.
fn scheme_key(s: Option<&str>) -> String {
    match s.map(str::trim) {
        Some(t) if !t.is_empty() => t.to_lowercase(),
        _ => "(none)".into(),
    }
}

/// A shape's bit in a row's shape set: its index in [`HEX_CENSUS_SHAPES`].
fn shape_bit(shape: &str) -> u8 {
    1 << HEX_CENSUS_SHAPES.iter().position(|s| *s == shape).unwrap_or(5)
}

/// Issue 575 unit 1: one population's rows and what their mentions span. Every row is in exactly one of
/// `no_mentions`, `no_tender`, `one_tender` and `multi_tender`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct HexTally {
    pub rows: u64,
    pub mentions: u64,
    /// Rows with no mention left (orphans awaiting the sweep).
    pub no_mentions: u64,
    pub one_mention: u64,
    /// Rows whose mentions' notices are versions of no Tender.
    pub no_tender: u64,
    /// Single-use: the mentions span exactly one distinct Tender.
    pub one_tender: u64,
    pub multi_tender: u64,
    /// The mentions span more than one distinct notice (a TED+DÖE pair of one award counts here).
    pub multi_notice: u64,
    /// Rows with at least `stable_mentions` mentions.
    pub stable: u64,
}

impl HexTally {
    fn add(&mut self, mentions: u64, notices: u64, tenders: u64, stable: bool) {
        self.rows += 1;
        self.mentions += mentions;
        match (mentions, tenders) {
            (0, _) => self.no_mentions += 1,
            (_, 0) => self.no_tender += 1,
            (_, 1) => self.one_tender += 1,
            _ => self.multi_tender += 1,
        }
        self.one_mention += u64::from(mentions == 1);
        self.multi_notice += u64::from(notices > 1);
        self.stable += u64::from(stable);
    }

    /// Sum another tally into this one (a capped breakdown's rest).
    fn absorb(&mut self, other: &HexTally) {
        self.rows += other.rows;
        self.mentions += other.mentions;
        self.no_mentions += other.no_mentions;
        self.one_mention += other.one_mention;
        self.no_tender += other.no_tender;
        self.one_tender += other.one_tender;
        self.multi_tender += other.multi_tender;
        self.multi_notice += other.multi_notice;
        self.stable += other.stable;
    }
}

/// Issue 575 unit 1: the class — the 32-hex rows with at least one cf35 mention.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct HexClass {
    pub all: HexTally,
    /// The class rows' cf35 mentions (phase 1's count).
    pub cf35_mentions: u64,
    /// Their mentions of any other shape (`none` included).
    pub other_shape_mentions: u64,
    /// Rows whose mentions hold cf35 and any other shape but `none`.
    pub mixed_shape_rows: u64,
    /// Distinct notices of cf35 mentions.
    pub notices: u64,
    /// Distinct Tenders those notices are versions of.
    pub tenders: u64,
    /// Stored values (`GuidKeep::key`) held by two or more class rows.
    pub values_multi_row: u64,
    /// Values whose class rows together span two or more Tenders (530 F4: per value). Unit 2's bar.
    pub values_multi_tender: u64,
    /// Rows whose identifier is on the injected KEEP (bar 0).
    pub keep_530_overlap: u64,
    /// Rows whose identifier passes `uuid_v4` (about 1/64; census 2655's `ted other` cell).
    pub v4_rows: u64,
    /// The lowest cf35 notice id and the earliest publication of a cf35 notice.
    pub first_notice_id: Option<i64>,
    pub first_published: Option<i64>,
    /// By `organizations.country` (`none` for NULL).
    pub by_country: BTreeMap<String, HexTally>,
    /// By the UTC year of the row's earliest notice (`none` without a dated notice): the era.
    pub by_first_year: BTreeMap<String, HexTally>,
    /// By the row's notices' distinct `notices.source` values joined by `+`.
    pub by_source: BTreeMap<String, HexTally>,
    /// By the row's notices' distinct `notices.profile` values joined by `+`, the largest
    /// [`HEX_CENSUS_KEYS_MAX`], the rest under [`GUID_CENSUS_REST`].
    pub by_profile: BTreeMap<String, HexTally>,
    /// By the stratum of the row's first cf35 mention (phase 2; `no_role` when phase 2 met none).
    pub by_stratum: BTreeMap<String, HexTally>,
}

/// Issue 575 unit 1: the within-notice twins — `(notice, name_norm)` groups whose cf35 mentions carry two
/// or more distinct values (unit 2 option b's reach, 530-T's shape).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct HexTwin {
    pub notices: u64,
    pub groups: u64,
    pub mentions: u64,
    /// Class rows with a mention in a twin group.
    pub rows: u64,
    /// Of those, rows whose stratum is `sprovider` (the eSender is printed once, so 0 expected).
    pub sprovider_rows: u64,
}

/// Issue 575 unit 1: Avenue-Web's `<procedure>-1-<lot>-<n>` tenderer keys inside the cf35 notices (unit 5's
/// size): their mentions, the orgs they bind, the distinct procedures (first group) and the stripped keys
/// two distinct raws share (lot 11 / n 1 and lot 1 / n 11 both store `<procedure>11111`).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct HexTendererKeys {
    pub mentions: u64,
    pub rows: u64,
    pub procedures: u64,
    pub collisions: u64,
}

/// Issue 575 unit 1: phase 2 — every ORG mention of every notice holding a cf35 mention.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct HexNoticeMentions {
    pub notices: u64,
    pub mentions: u64,
    pub by_shape: BTreeMap<String, u64>,
    /// Each role of a mention counted once; `(none)` for a mention with no party or bid-party row.
    pub by_shape_role: BTreeMap<String, BTreeMap<String, u64>>,
    /// The published scheme, trimmed and lower-cased; `(none)` for NULL.
    pub by_shape_scheme: BTreeMap<String, BTreeMap<String, u64>>,
    pub cf35_mentions: u64,
    /// The mention's org is a walked class row.
    pub cf35_on_class_rows: u64,
    /// The org id is at or under `max_id` but no walked class row (another stored identifier: a merge
    /// survivor).
    pub cf35_off_class: u64,
    /// The org was minted after the walk began.
    pub cf35_above_max_id: u64,
    /// BAR: a buyer role on a cf35 mention.
    pub cf35_buyer_mentions: u64,
    /// `org_name_norm(name)` empty (511 guard 3; unit 2 option ii).
    pub cf35_blank_name_mentions: u64,
    /// A blank name or no country: no fallback key.
    pub cf35_unkeyed_mentions: u64,
    pub cf35_by_stratum: BTreeMap<String, u64>,
    /// The mention's role set joined by `+` (`(none)`), the largest [`HEX_CENSUS_KEYS_MAX`].
    pub cf35_role_sets: BTreeMap<String, u64>,
    pub twin: HexTwin,
    pub tenderer_key: HexTendererKeys,
}

/// Issue 575 unit 1: 530's A6 city screen over the cf35 mentions' fallback targets.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct HexCityScreen {
    /// Keyed cf35 mentions whose ORG section publishes a BT-513 city, and those that do not.
    pub mentions_with_city: u64,
    pub mentions_without_city: u64,
    /// (a) Mentions whose city is not in their found target's non-empty city set, and the targets.
    pub city_absent_mentions: u64,
    pub city_absent_targets: u64,
    /// (b) Found targets whose newest mentions name two or more cities.
    pub multi_city_targets: u64,
    /// (c) Keys, found or missing, whose cf35 mentions name two or more cities.
    pub planned_multi_city_keys: u64,
}

/// Issue 575 unit 1: one fallback key of the cf35 mentions and its identifier-NULL row.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct HexTarget {
    pub name_norm: String,
    pub country: Option<String>,
    /// The first cf35 mention's published name under the key.
    pub published_name: String,
    /// The identifier-NULL row, lowest id; `None` = missing (a dissolve would mint it).
    pub id: Option<i64>,
    pub row_name: Option<String>,
    /// Its `organization_mentions` count.
    pub target_mentions: u64,
    pub cf35_mentions: u64,
    pub cf35_rows: u64,
    /// The folded BT-513 cities of its newest `GUID_TWIN_TARGET_NEWEST` mentions, sorted, distinct.
    pub cities: Vec<String>,
    /// Its cf35 mentions' folded cities, sorted, distinct.
    pub planned_cities: Vec<String>,
    /// (A6 a) cf35 mentions whose city is not in a non-empty `cities`.
    pub city_absent: u64,
    /// (A6 b) `cities` holds two or more.
    pub multi_city: bool,
    /// (A6 c) `planned_cities` holds two or more.
    pub multi_city_planned: bool,
}

/// Issue 575 unit 1: phase 3 — the targets.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct HexTargets {
    pub keys: u64,
    pub found: u64,
    pub missing: u64,
    pub mentions_to_found: u64,
    pub mentions_to_missing: u64,
    pub city: HexCityScreen,
    /// The largest [`HEX_CENSUS_TOP_TARGETS`] by cf35 mentions, then name_norm and country.
    pub top: Vec<HexTarget>,
    /// Every target with `city_absent > 0`, `multi_city` or `multi_city_planned`, in the same order, the
    /// first `GUID_TWIN_LIST_CAP`.
    pub city_flagged: Vec<HexTarget>,
    /// Every key whose folded name starts with [`HEX_CENSUS_CCIRA_PREFIX`], in the same order, capped alike.
    pub ccira: Vec<HexTarget>,
}

/// Issue 575 unit 1: one class row as the census read it — a sample or a canary.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct HexRowSample {
    pub org_id: i64,
    pub country: Option<String>,
    /// The served name, `organizations.name`.
    pub name: String,
    pub identifier: String,
    /// The row's first cf35 mention seen in phase 2 (lowest notice, then section): its raw and scheme.
    pub raw: Option<String>,
    pub scheme: Option<String>,
    pub mentions: u64,
    pub cf35_mentions: u64,
    pub notices: u64,
    pub tenders: u64,
    /// The first three of its distinct Tenders.
    pub tender_ids: Vec<i64>,
    /// The first cf35 mention's notice — `None` when phase 2 met none of the row's cf35 mentions (an unseen
    /// row, not an unkeyed one: `raw`, `scheme`, `section_id` and every `target_*` are then `None` too).
    pub notice_id: Option<i64>,
    pub section_id: Option<String>,
    pub roles: Vec<String>,
    pub stratum: &'static str,
    pub first_year: Option<i64>,
    pub twin: bool,
    /// The first cf35 mention's fallback key (`None` when unkeyed) and its found row (`None` when missing or
    /// unkeyed). All three are `None` when phase 2 met none of the row's cf35 mentions (`notice_id` `None`).
    pub target_name_norm: Option<String>,
    pub target_country: Option<String>,
    pub target_id: Option<i64>,
}

/// Issue 575 unit 1: one twin group — a notice whose cf35 mentions under one name carry two or more values.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct HexTwinSample {
    pub notice_id: i64,
    pub name_norm: String,
    /// The members' org ids and distinct values (`GuidKeep::key`), in section order.
    pub org_ids: Vec<i64>,
    pub values: Vec<String>,
}

/// Issue 575 unit 1: a dash-stripped tenderer key two distinct raws share.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct HexKeyCollision {
    pub stripped: String,
    pub raws: Vec<String>,
}

/// Issue 575 unit 1: what unit 2 reads straight off the report.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct HexSamples {
    /// R2: [`HEX_CENSUS_R2_STRATA`], each stratum's class rows in salted order
    /// (`guid_twin_sample_key(id, HEX_CENSUS_SALT)`).
    pub r2_random: Vec<HexRowSample>,
    /// Every class row with a buyer-stratum cf35 mention, by id, the first `GUID_TWIN_LIST_CAP`.
    pub buyer: Vec<HexRowSample>,
    /// Every class row whose value spans two or more Tenders over its class rows (530 F4: per value, so
    /// every row spanning two itself), by id, capped alike: unit 3's frozen keep-list source.
    pub multi_tender: Vec<HexRowSample>,
    /// Every stable class row, by mentions descending then id, capped alike.
    pub stable: Vec<HexRowSample>,
    /// Every KEEP overlap, by id, capped alike.
    pub keep_530: Vec<HexRowSample>,
    /// The first `samples` rows mixing shapes, by id.
    pub mixed_shape: Vec<HexRowSample>,
    /// The first `samples` rows with a blank-name cf35 mention, by id.
    pub blank_name: Vec<HexRowSample>,
    /// The first `samples` twin groups, by notice id.
    pub twin_groups: Vec<HexTwinSample>,
    /// The first `samples` tenderer-key collisions, by stripped key.
    pub tenderer_key_collisions: Vec<HexKeyCollision>,
}

/// Issue 575 unit 1: [`Db::hex_census`]'s inputs.
pub struct HexCensusArgs<'a> {
    /// Issue 300's class: `idgate::census(..).hex_hash` in the job.
    pub hex_hash: fn(&str) -> bool,
    /// `idgate::uuid_v4` in the job.
    pub is_v4: fn(&str) -> bool,
    /// A party role's class (the supervisor's `guid_role_class`): buyer and tenderer in [`hex_stratum`].
    pub role_class: fn(&str) -> &'static str,
    /// KEEP_530 (`ingest::project::guid_twin::keep_530()` in the job).
    pub keep: &'a GuidKeep,
    /// A class row with at least this many mentions is stable.
    pub stable_mentions: u64,
    /// The listing per capped sample list.
    pub samples: usize,
    /// Organization ids per window: [`HEX_CENSUS_WINDOW`] in the job.
    pub window: i64,
    /// 32-hex rows per batch, clamped to 1..=[`HEX_CENSUS_BATCH`].
    pub batch: usize,
    /// Class rows reported in `canaries` whatever they hold; any other id is listed in `canaries_absent`.
    pub canaries: &'a [i64],
    pub stop: &'a (dyn Fn() -> bool + Sync),
    pub progress: &'a (dyn Fn(u64, &str) + Sync),
}

/// Issue 575 unit 1: [`Db::hex_census`]'s answer. Read-only.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct HexCensusReport {
    pub window: i64,
    pub stable_mentions: u64,
    pub samples_per_list: usize,
    pub salt: u64,
    /// The highest organization id when the walk started; nothing above it was walked.
    pub max_id: i64,
    pub windows: u64,
    /// Rows whose identifier is 16 or more characters long, the window statement's filter.
    pub id_rows_16plus: u64,
    /// Of those, issue 300's `hex_hash` class (its 273,982 in run 1332, re-taken).
    pub hex_hash_rows: u64,
    /// Rows whose identifier is 32 hex digits: the walked population.
    pub hex32_rows: u64,
    pub hex32_mentions: u64,
    /// Distinct notices read by primary key, summed over the batches.
    pub notices_read: u64,
    pub class: HexClass,
    /// EVERY 32-hex row by `<sources> <shapes>`: the row's distinct `notices.source` values joined by `+`
    /// (`(no notice)` for a mention with no notice row), a space, and the distinct shapes of its mentions
    /// in [`HEX_CENSUS_SHAPES`] order joined by `+` (`(no mention)` for either half on an orphan); the
    /// largest [`HEX_CENSUS_KEYS_MAX`], the rest under [`GUID_CENSUS_REST`].
    pub by_shape: BTreeMap<String, HexTally>,
    /// `uuid_v4` passes per the same key.
    pub v4_by_shape: BTreeMap<String, u64>,
    pub notice_mentions: HexNoticeMentions,
    pub targets: HexTargets,
    pub samples: HexSamples,
    /// Class rows only; any other id is absent.
    pub canaries: Vec<HexRowSample>,
    pub canaries_absent: Vec<i64>,
    pub not_measured: Vec<&'static str>,
    pub stopped: bool,
}

/// A small string interner: a class row keeps one `u32` per breakdown key instead of the key.
#[derive(Default)]
struct Keys {
    at: HashMap<String, u32>,
    keys: Vec<String>,
}

impl Keys {
    fn id(&mut self, key: &str) -> u32 {
        if let Some(&id) = self.at.get(key) {
            return id;
        }
        let id = self.keys.len() as u32;
        self.keys.push(key.to_owned());
        self.at.insert(key.to_owned(), id);
        id
    }
}

/// A 32-hex row found by the window walk, waiting for its batch.
struct Pending {
    id: i64,
    country: Option<String>,
    identifier: String,
    name: String,
}

/// A class row's first cf35 mention in phase 2 (lowest notice, then section).
struct First {
    notice: i64,
    section: String,
    raw: Option<String>,
    scheme: Option<String>,
    roles: Vec<String>,
    /// Its fallback key's index in [`HexWalk::targets`]; `None` when unkeyed.
    target: Option<u32>,
}

/// One class row, kept for phases 2 and 3 and the finish: about 250 bytes with its strings.
struct ClassRow {
    id: i64,
    country: Option<String>,
    identifier: String,
    name: String,
    mentions: u64,
    cf35: u64,
    notices: u64,
    /// Every distinct Tender of its notices, sorted.
    tender_ids: Vec<i64>,
    first_year: Option<i64>,
    source: u32,
    profile: u32,
    shapes: u8,
    v4: bool,
    kept: bool,
    first: Option<First>,
    stratum: &'static str,
    twin: bool,
    /// A cf35 mention of a blank name; a buyer-stratum cf35 mention.
    blank_name: bool,
    buyer: bool,
}

/// One fallback key, as phases 2 and 3 fill it.
#[derive(Default)]
struct Target {
    name_norm: String,
    country: Option<String>,
    published_name: String,
    id: Option<i64>,
    row_name: Option<String>,
    target_mentions: u64,
    newest: Vec<(i64, String)>,
    cities: BTreeSet<String>,
    cf35_mentions: u64,
    rows: HashSet<i64>,
    /// The cf35 mentions' folded cities, each with its mention count.
    planned: BTreeMap<String, u64>,
}

/// One ORG mention of a cf35 notice.
struct NoticeMention {
    section: String,
    org: i64,
    name: Option<String>,
    country: Option<String>,
    raw: Option<String>,
    scheme: Option<String>,
}

/// One `(notice, name_norm)` group of cf35 mentions.
#[derive(Default)]
struct NameGroup {
    values: Vec<String>,
    orgs: Vec<i64>,
    mentions: u64,
}

/// The walk's state across batches and phases.
#[derive(Default)]
struct HexWalk {
    rows: Vec<ClassRow>,
    row_at: HashMap<i64, u32>,
    sources: Keys,
    profiles: Keys,
    /// The class rows' cf35 notices, and the Tenders those notices are versions of.
    cf35_notices: BTreeSet<i64>,
    class_tenders: HashSet<i64>,
    by_shape: HashMap<String, HexTally>,
    v4_by_shape: HashMap<String, u64>,
    role_sets: HashMap<String, u64>,
    targets: Vec<Target>,
    target_at: HashMap<(String, Option<String>), u32>,
    tenderer_rows: HashSet<i64>,
    tenderer_procedures: HashSet<u64>,
    /// A tenderer key with its dashes dropped -> the distinct raws storing as it.
    stripped: BTreeMap<String, BTreeSet<String>>,
}

impl HexWalk {
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

/// The largest `max` keys of `by` by rows (ties by key), the rest summed under [`GUID_CENSUS_REST`].
fn capped_tallies(by: HashMap<String, HexTally>, max: usize) -> BTreeMap<String, HexTally> {
    let mut all: Vec<(String, HexTally)> = by.into_iter().collect();
    all.sort_by(|a, b| b.1.rows.cmp(&a.1.rows).then_with(|| a.0.cmp(&b.0)));
    let mut out: BTreeMap<String, HexTally> = BTreeMap::new();
    for (i, (key, tally)) in all.into_iter().enumerate() {
        if i < max {
            out.insert(key, tally);
        } else {
            out.entry(GUID_CENSUS_REST.to_owned()).or_default().absorb(&tally);
        }
    }
    out
}

/// The same for plain counts.
fn capped_counts(by: HashMap<String, u64>, max: usize) -> BTreeMap<String, u64> {
    let mut all: Vec<(String, u64)> = by.into_iter().collect();
    all.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let mut out: BTreeMap<String, u64> = BTreeMap::new();
    for (i, (key, n)) in all.into_iter().enumerate() {
        if i < max {
            out.insert(key, n);
        } else {
            *out.entry(GUID_CENSUS_REST.to_owned()).or_default() += n;
        }
    }
    out
}

impl Db {
    /// Issue 575 unit 1: the `hex-census` ([`HexCensusReport`] names what it counts).
    ///
    /// This writes nothing. Phase 1 walks `organizations` by rowid windows of `args.window` ids up to the
    /// highest id at its start ([`HEX_CENSUS_ORGS_SQL`]: identifier 16+ long, which re-takes issue 300's
    /// `hex_hash` total), keeps the rows whose identifier is 32 hex digits, and reads them in batches of
    /// `args.batch` (at most [`HEX_CENSUS_BATCH`]): their mentions through `organization_mentions_org`
    /// (each raw BT-501 classified by [`hex_raw_shape`]), the distinct notices of those mentions by
    /// primary key and the Tenders they are versions of through `tender_versions_notice`. A row with a
    /// cf35 mention is a class row. Phase 2 reads every ORG mention of each class row's cf35 notices (the
    /// `organization_mentions` primary-key prefix), their party and bid-party roles through
    /// `tender_version_parties_mention_key` / `tender_version_bid_parties_mention_key`, and their BT-513
    /// cities (the `notice_texts` prefix). Phase 3 seeks each cf35 mention's fallback key through
    /// `organizations_name_country`, each found target's mention count and newest
    /// `GUID_TWIN_TARGET_NEWEST` mentions through `organization_mentions_org`, and those mentions' cities.
    /// Every statement is a rowid window, an IN chunk of at most [`IN_CHUNK`] keys or one target seek.
    ///
    /// The stop flag is read before every window, every batch, every IN chunk of every read, and every
    /// 64 target seeks (missing targets included); a stopped run returns an empty report marked
    /// `stopped`. Each of [`HEX_CENSUS_INDEXES`] must stand, so the census refuses before reading.
    ///
    /// Memory is one record per class row, about 250 bytes with its strings (about 60 MB at 230k rows);
    /// the targets and each phase-2 chunk are small.
    pub async fn hex_census(&self, args: HexCensusArgs<'_>) -> turso::Result<HexCensusReport> {
        for index in HEX_CENSUS_INDEXES {
            if !self.has_index(index).await? {
                return Err(turso::Error::Error(format!(
                    "hex-census refused: {index} is absent (a rebuild?), so every chunk would scan its table; \
                     run `reindex` first. Nothing was read"
                )));
            }
        }
        let stopped = || HexCensusReport { stopped: true, ..Default::default() };
        let mut report = HexCensusReport {
            window: args.window.max(1),
            stable_mentions: args.stable_mentions,
            samples_per_list: args.samples,
            salt: HEX_CENSUS_SALT,
            not_measured: HEX_CENSUS_NOT_MEASURED.to_vec(),
            ..Default::default()
        };
        let reader = self.reader().await?;
        let mut rows = reader.query(GUID_CENSUS_MAX_ID_SQL, ()).await?;
        report.max_id = match rows.next().await? {
            Some(row) => opt_int_of(&row, 0).unwrap_or(0),
            None => 0,
        };
        drop(rows);

        // Phase 1: the walk.
        let width = args.batch.clamp(1, HEX_CENSUS_BATCH);
        let mut walk = HexWalk::default();
        let mut pending: Vec<Pending> = Vec::new();
        let mut lo = 0i64;
        loop {
            // Full batches first, each behind the stop flag (and each of its chunks behind it too).
            while pending.len() >= width || (lo >= report.max_id && !pending.is_empty()) {
                if (args.stop)() {
                    return Ok(stopped());
                }
                let batch: Vec<Pending> = pending.drain(..pending.len().min(width)).collect();
                if !self.hex_census_batch(&reader, &args, &mut walk, &mut report, batch).await? {
                    return Ok(stopped());
                }
            }
            if lo >= report.max_id {
                break;
            }
            if (args.stop)() {
                return Ok(stopped());
            }
            let hi = lo.saturating_add(report.window).min(report.max_id);
            let mut rows = reader.query(HEX_CENSUS_ORGS_SQL, (Value::Integer(lo), Value::Integer(hi))).await?;
            while let Some(row) = rows.next().await? {
                report.id_rows_16plus += 1;
                let identifier = text(&row, 2);
                if (args.hex_hash)(&identifier) {
                    report.hex_hash_rows += 1;
                }
                if hex32(&identifier) {
                    report.hex32_rows += 1;
                    pending.push(Pending { id: int(&row, 0), country: opt_text_of(&row, 1), identifier, name: text(&row, 3) });
                }
            }
            drop(rows);
            report.windows += 1;
            lo = hi;
            (args.progress)(
                lo.max(0) as u64,
                &format!(
                    "walking organizations: id {lo} of {}, {} 32-hex row(s), {} cf35 row(s)",
                    report.max_id,
                    report.hex32_rows,
                    walk.rows.len()
                ),
            );
        }

        // Phase 2: every ORG mention of the cf35 notices, ascending, four reads per IN chunk.
        let notice_ids: Vec<i64> = walk.cf35_notices.iter().copied().collect();
        for (k, chunk) in notice_ids.chunks(IN_CHUNK).enumerate() {
            let params: Vec<Value> = chunk.iter().map(|&id| Value::Integer(id)).collect();
            if (args.stop)() {
                return Ok(stopped());
            }
            let mut by_notice: BTreeMap<i64, Vec<NoticeMention>> = BTreeMap::new();
            let mut rows = reader.query(&guid_twin_notice_mentions_sql(chunk.len()), params.clone()).await?;
            while let Some(row) = rows.next().await? {
                by_notice.entry(int(&row, 0)).or_default().push(NoticeMention {
                    section: text(&row, 1),
                    org: int(&row, 2),
                    name: opt_text_of(&row, 3),
                    country: opt_text_of(&row, 4),
                    raw: opt_text_of(&row, 5),
                    scheme: opt_text_of(&row, 6),
                });
            }
            drop(rows);
            let mut roles: HashMap<(i64, String), BTreeSet<String>> = HashMap::new();
            for (sql, prefix) in [(guid_twin_parties_sql(chunk.len()), ""), (guid_twin_bid_parties_sql(chunk.len()), "bid-")] {
                if (args.stop)() {
                    return Ok(stopped());
                }
                let mut rows = reader.query(&sql, params.clone()).await?;
                while let Some(row) = rows.next().await? {
                    roles.entry((int(&row, 0), text(&row, 1))).or_default().insert(format!("{prefix}{}", text(&row, 2)));
                }
            }
            if (args.stop)() {
                return Ok(stopped());
            }
            // The city a mention publishes: the BT-513 text of its ORG section, lowest ordinal, folded.
            let mut cities: HashMap<(i64, String), (i64, String)> = HashMap::new();
            let mut city_params = params;
            city_params.push(Value::Text(GUID_TWIN_CITY_FIELD.to_owned()));
            let mut rows = reader.query(&guid_twin_cities_sql(chunk.len()), city_params).await?;
            while let Some(row) = rows.next().await? {
                let (ordinal, city) = (int(&row, 2), guid_twin_city_fold(&text(&row, 3)));
                if city.is_empty() {
                    continue;
                }
                let key = (int(&row, 0), text(&row, 1));
                match cities.get(&key) {
                    Some((have, _)) if *have <= ordinal => {}
                    _ => {
                        cities.insert(key, (ordinal, city));
                    }
                }
            }
            drop(rows);
            for (notice, mut mentions) in by_notice {
                mentions.sort_by(|a, b| a.section.cmp(&b.section));
                hex_census_notice(&args, &mut walk, &mut report, notice, &mentions, &roles, &cities);
            }
            let done = ((k + 1) * IN_CHUNK).min(notice_ids.len());
            (args.progress)(done as u64, &format!("reading cf35 notices: {done} of {}", notice_ids.len()));
        }

        // Phase 3a: the identifier-NULL row of each key, the lowest id winning (the resolver's LIMIT 1).
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
                if let Some(&at) = walk.target_at.get(&(norm, country)) {
                    let target = &mut walk.targets[at as usize];
                    if target.id.is_none_or(|have| id < have) {
                        target.id = Some(id);
                        target.row_name = opt_text_of(&row, 3);
                    }
                }
            }
        }
        // Phase 3b: each found target's mention count and newest mentions.
        let total = walk.targets.len();
        for (k, target) in walk.targets.iter_mut().enumerate() {
            // The checkpoint counts every target, missing ones included, so at most 64 seek pairs run
            // between two stop reads.
            if k % 64 == 0 {
                if (args.stop)() {
                    return Ok(stopped());
                }
                (args.progress)(k as u64, &format!("seeking fallback targets: {k} of {total}"));
            }
            let Some(id) = target.id else { continue };
            let mut rows = reader.query(GUID_TWIN_TARGET_COUNT_SQL, vec![Value::Integer(id)]).await?;
            target.target_mentions = match rows.next().await? {
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
        // Phase 3c: the newest mentions' cities.
        let wanted: HashSet<(i64, String)> = walk.targets.iter().flat_map(|t| t.newest.iter().cloned()).collect();
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
        for t in &mut walk.targets {
            t.cities = t.newest.iter().filter_map(|key| cities.get(key).map(|(_, c)| c.clone())).collect();
        }
        drop(reader);
        Ok(hex_census_finish(report, walk, &args))
    }

    /// One batch of 32-hex rows: their mentions, notices and Tenders, tallied into `by_shape` and, for a
    /// row with a cf35 mention, kept as a class row. `false` when the stop flag fired before one of its
    /// chunks: `walk` and `report` are then half-folded, and the caller discards both.
    async fn hex_census_batch(
        &self,
        reader: &crate::read::Reader,
        args: &HexCensusArgs<'_>,
        walk: &mut HexWalk,
        report: &mut HexCensusReport,
        batch: Vec<Pending>,
    ) -> turso::Result<bool> {
        let at: HashMap<i64, usize> = batch.iter().enumerate().map(|(i, p)| (p.id, i)).collect();
        let ids: Vec<i64> = batch.iter().map(|p| p.id).collect();
        // Per row: mentions, cf35 mentions, notice ids, cf35 notice ids, the shapes' bits.
        let mut mentions = vec![0u64; batch.len()];
        let mut cf35 = vec![0u64; batch.len()];
        let mut notices: Vec<Vec<i64>> = vec![Vec::new(); batch.len()];
        let mut cf35_notices: Vec<Vec<i64>> = vec![Vec::new(); batch.len()];
        let mut shapes = vec![0u8; batch.len()];
        for chunk in ids.chunks(IN_CHUNK) {
            if (args.stop)() {
                return Ok(false);
            }
            let params: Vec<Value> = chunk.iter().map(|&id| Value::Integer(id)).collect();
            let mut rows = reader.query(&guid_census_mentions_sql(chunk.len()), params).await?;
            while let Some(row) = rows.next().await? {
                let Some(&i) = at.get(&int(&row, 0)) else { continue };
                let notice = int(&row, 1);
                let shape = hex_raw_shape(opt_text_of(&row, 3).as_deref());
                report.hex32_mentions += 1;
                mentions[i] += 1;
                notices[i].push(notice);
                shapes[i] |= shape_bit(shape);
                if shape == "cf35" {
                    cf35[i] += 1;
                    cf35_notices[i].push(notice);
                }
            }
        }
        let mut distinct: BTreeSet<i64> = BTreeSet::new();
        for list in notices.iter_mut().chain(cf35_notices.iter_mut()) {
            list.sort_unstable();
            list.dedup();
            distinct.extend(list.iter().copied());
        }
        let distinct: Vec<i64> = distinct.into_iter().collect();
        report.notices_read += distinct.len() as u64;

        // The notices by primary key: (source, profile, published_at).
        let mut notice_info: HashMap<i64, (String, String, Option<i64>)> = HashMap::with_capacity(distinct.len());
        for chunk in distinct.chunks(IN_CHUNK) {
            if (args.stop)() {
                return Ok(false);
            }
            let params: Vec<Value> = chunk.iter().map(|&id| Value::Integer(id)).collect();
            let mut rows = reader.query(&guid_census_notices_sql(chunk.len()), params).await?;
            while let Some(row) = rows.next().await? {
                notice_info.insert(int(&row, 0), (text(&row, 1), text(&row, 2), opt_int_of(&row, 3)));
            }
        }
        // The Tenders those notices are versions of.
        let mut notice_tenders: HashMap<i64, Vec<i64>> = HashMap::new();
        for chunk in distinct.chunks(IN_CHUNK) {
            if (args.stop)() {
                return Ok(false);
            }
            let params: Vec<Value> = chunk.iter().map(|&id| Value::Integer(id)).collect();
            let mut rows = reader.query(&guid_census_tenders_sql(chunk.len()), params).await?;
            while let Some(row) = rows.next().await? {
                notice_tenders.entry(int(&row, 0)).or_default().push(int(&row, 1));
            }
        }

        for (i, p) in batch.into_iter().enumerate() {
            let mut tenders: Vec<i64> =
                notices[i].iter().filter_map(|n| notice_tenders.get(n)).flatten().copied().collect();
            tenders.sort_unstable();
            tenders.dedup();
            let mut sources: BTreeSet<&str> = BTreeSet::new();
            let mut profiles: BTreeSet<&str> = BTreeSet::new();
            let mut first_published: Option<i64> = None;
            for n in &notices[i] {
                match notice_info.get(n) {
                    Some((source, profile, published)) => {
                        sources.insert(source.as_str());
                        profiles.insert(profile.as_str());
                        if let Some(when) = *published {
                            first_published = Some(first_published.map_or(when, |f: i64| f.min(when)));
                        }
                    }
                    None => {
                        sources.insert("(no notice)");
                    }
                }
            }
            let source_key =
                if sources.is_empty() { "(no mention)".to_owned() } else { sources.into_iter().collect::<Vec<_>>().join("+") };
            let shape: Vec<&str> = HEX_CENSUS_SHAPES
                .iter()
                .enumerate()
                .filter(|(bit, _)| shapes[i] & (1 << bit) != 0)
                .map(|(_, s)| *s)
                .collect();
            let shape_key = if shape.is_empty() { "(no mention)".to_owned() } else { shape.join("+") };
            let key = format!("{source_key} {shape_key}");
            let row_tenders = tenders.len() as u64;
            let stable = mentions[i] >= args.stable_mentions;
            walk.by_shape.entry(key.clone()).or_default().add(mentions[i], notices[i].len() as u64, row_tenders, stable);
            let v4 = (args.is_v4)(&p.identifier);
            if v4 {
                *walk.v4_by_shape.entry(key).or_default() += 1;
            }
            if cf35[i] == 0 {
                continue;
            }
            // A class row: its cf35 notices and their Tenders are the class's.
            for n in &cf35_notices[i] {
                walk.cf35_notices.insert(*n);
                if let Some(ts) = notice_tenders.get(n) {
                    walk.class_tenders.extend(ts.iter().copied());
                }
                report.class.first_notice_id = Some(report.class.first_notice_id.map_or(*n, |f| f.min(*n)));
                if let Some(when) = notice_info.get(n).and_then(|info| info.2) {
                    report.class.first_published = Some(report.class.first_published.map_or(when, |f| f.min(when)));
                }
            }
            let profile_key =
                if profiles.is_empty() { "(no notice)".to_owned() } else { profiles.into_iter().collect::<Vec<_>>().join("+") };
            let source = walk.sources.id(&source_key);
            let profile = walk.profiles.id(&profile_key);
            let kept = args.keep.contains(&p.identifier);
            walk.row_at.insert(p.id, walk.rows.len() as u32);
            walk.rows.push(ClassRow {
                id: p.id,
                country: p.country,
                identifier: p.identifier,
                name: p.name,
                mentions: mentions[i],
                cf35: cf35[i],
                notices: notices[i].len() as u64,
                tender_ids: tenders,
                first_year: first_published.map(unix_year),
                source,
                profile,
                shapes: shapes[i],
                v4,
                kept,
                first: None,
                stratum: "no_role",
                twin: false,
                blank_name: false,
                buyer: false,
            });
        }
        Ok(true)
    }
}

/// Phase 2, one notice: every ORG mention by shape, role and scheme; for a cf35 mention its class, its
/// stratum, its name, its fallback key and city, and its row's first mention; then the notice's twin
/// groups.
fn hex_census_notice(
    args: &HexCensusArgs<'_>,
    walk: &mut HexWalk,
    report: &mut HexCensusReport,
    notice: i64,
    mentions: &[NoticeMention],
    roles: &HashMap<(i64, String), BTreeSet<String>>,
    cities: &HashMap<(i64, String), (i64, String)>,
) {
    let nm = &mut report.notice_mentions;
    nm.notices += 1;
    let mut groups: BTreeMap<String, NameGroup> = BTreeMap::new();
    for m in mentions {
        let key = (notice, m.section.clone());
        let shape = hex_raw_shape(m.raw.as_deref());
        let role_list: Vec<String> = roles.get(&key).map(|set| set.iter().cloned().collect()).unwrap_or_default();
        nm.mentions += 1;
        *nm.by_shape.entry(shape.to_owned()).or_default() += 1;
        let by_role = nm.by_shape_role.entry(shape.to_owned()).or_default();
        if role_list.is_empty() {
            *by_role.entry("(none)".to_owned()).or_default() += 1;
        }
        for role in &role_list {
            *by_role.entry(role.clone()).or_default() += 1;
        }
        *nm.by_shape_scheme.entry(shape.to_owned()).or_default().entry(scheme_key(m.scheme.as_deref())).or_default() += 1;
        if shape == "tenderer_key" {
            let raw = m.raw.as_deref().unwrap_or_default().trim();
            nm.tenderer_key.mentions += 1;
            walk.tenderer_rows.insert(m.org);
            if let Some(procedure) = raw.split('-').next().and_then(|g| g.parse::<u64>().ok()) {
                walk.tenderer_procedures.insert(procedure);
            }
            walk.stripped.entry(raw.replace('-', "")).or_default().insert(raw.to_owned());
            continue;
        }
        if shape != "cf35" {
            continue;
        }
        nm.cf35_mentions += 1;
        let class = walk.row_at.get(&m.org).map(|&i| i as usize);
        match class {
            Some(_) => nm.cf35_on_class_rows += 1,
            None if m.org > report.max_id => nm.cf35_above_max_id += 1,
            None => nm.cf35_off_class += 1,
        }
        let stratum = hex_stratum(&role_list, args.role_class);
        *nm.cf35_by_stratum.entry(stratum.to_owned()).or_default() += 1;
        let set = if role_list.is_empty() { "(none)".to_owned() } else { role_list.join("+") };
        *walk.role_sets.entry(set).or_default() += 1;
        if stratum == "buyer" {
            nm.cf35_buyer_mentions += 1;
        }
        let norm = org_name_norm(m.name.as_deref().unwrap_or(""));
        if norm.is_empty() {
            nm.cf35_blank_name_mentions += 1;
        }
        let country = m.country.as_deref().map(str::trim).filter(|c| !c.is_empty()).map(|c| register_jurisdiction(c).to_owned());
        let target = match &country {
            Some(country) if !norm.is_empty() => {
                let at = walk.target(&norm, Some(country.as_str()));
                let t = &mut walk.targets[at as usize];
                t.cf35_mentions += 1;
                t.rows.insert(m.org);
                if t.published_name.is_empty() {
                    t.published_name = m.name.clone().unwrap_or_default();
                }
                match cities.get(&key) {
                    Some((_, city)) => {
                        *t.planned.entry(city.clone()).or_default() += 1;
                        report.targets.city.mentions_with_city += 1;
                    }
                    None => report.targets.city.mentions_without_city += 1,
                }
                Some(at)
            }
            _ => {
                nm.cf35_unkeyed_mentions += 1;
                None
            }
        };
        if !norm.is_empty() {
            let group = groups.entry(norm.clone()).or_default();
            let value = GuidKeep::key(m.raw.as_deref().unwrap_or_default());
            if !group.values.contains(&value) {
                group.values.push(value);
            }
            if !group.orgs.contains(&m.org) {
                group.orgs.push(m.org);
            }
            group.mentions += 1;
        }
        if let Some(i) = class {
            let row = &mut walk.rows[i];
            row.blank_name |= norm.is_empty();
            row.buyer |= stratum == "buyer";
            // Notices ascend and sections ascend within one, so the first set is the lowest.
            if row.first.is_none() {
                row.first = Some(First {
                    notice,
                    section: m.section.clone(),
                    raw: m.raw.clone(),
                    scheme: m.scheme.clone(),
                    roles: role_list.clone(),
                    target,
                });
                row.stratum = stratum;
            }
        }
    }
    // The notice's twin groups: one name, two or more distinct cf35 values.
    let mut twin_notice = false;
    for (norm, group) in groups {
        if group.values.len() < 2 {
            continue;
        }
        twin_notice = true;
        let nm = &mut report.notice_mentions;
        nm.twin.groups += 1;
        nm.twin.mentions += group.mentions;
        for org in &group.orgs {
            if let Some(&i) = walk.row_at.get(org) {
                walk.rows[i as usize].twin = true;
            }
        }
        if report.samples.twin_groups.len() < args.samples {
            report.samples.twin_groups.push(HexTwinSample {
                notice_id: notice,
                name_norm: norm,
                org_ids: group.orgs,
                values: group.values,
            });
        }
    }
    if twin_notice {
        report.notice_mentions.twin.notices += 1;
    }
}

/// The tallies, targets, samples and canaries over every class row the walk kept. A sync free fn, off
/// the async frame.
fn hex_census_finish(mut report: HexCensusReport, mut walk: HexWalk, args: &HexCensusArgs<'_>) -> HexCensusReport {
    let cap = args.samples;
    let stable = |row: &ClassRow| row.mentions >= args.stable_mentions;
    let cf35_bit = shape_bit("cf35");
    let none_bit = shape_bit("none");

    // The class's tallies and breakdowns, in id order.
    let mut by_profile: HashMap<String, HexTally> = HashMap::new();
    let mut values: HashMap<String, (u64, BTreeSet<i64>)> = HashMap::new();
    for row in &walk.rows {
        let (s, n, t) = (stable(row), row.notices, row.tender_ids.len() as u64);
        let class = &mut report.class;
        class.all.add(row.mentions, n, t, s);
        class.by_country.entry(row.country.clone().unwrap_or_else(|| "none".to_owned())).or_default().add(row.mentions, n, t, s);
        class
            .by_first_year
            .entry(row.first_year.map_or_else(|| "none".to_owned(), |y| y.to_string()))
            .or_default()
            .add(row.mentions, n, t, s);
        class.by_source.entry(walk.sources.keys[row.source as usize].clone()).or_default().add(row.mentions, n, t, s);
        by_profile.entry(walk.profiles.keys[row.profile as usize].clone()).or_default().add(row.mentions, n, t, s);
        class.by_stratum.entry(row.stratum.to_owned()).or_default().add(row.mentions, n, t, s);
        class.cf35_mentions += row.cf35;
        class.other_shape_mentions += row.mentions - row.cf35;
        class.mixed_shape_rows += u64::from(row.shapes & cf35_bit != 0 && row.shapes & !(cf35_bit | none_bit) != 0);
        class.keep_530_overlap += u64::from(row.kept);
        class.v4_rows += u64::from(row.v4);
        let value = values.entry(GuidKeep::key(&row.identifier)).or_default();
        value.0 += 1;
        value.1.extend(row.tender_ids.iter().copied());
        if row.twin {
            report.notice_mentions.twin.rows += 1;
            report.notice_mentions.twin.sprovider_rows += u64::from(row.stratum == "sprovider");
        }
    }
    report.class.by_profile = capped_tallies(by_profile, HEX_CENSUS_KEYS_MAX);
    report.class.notices = walk.cf35_notices.len() as u64;
    report.class.tenders = walk.class_tenders.len() as u64;
    report.class.values_multi_row = values.values().filter(|(rows, _)| *rows >= 2).count() as u64;
    report.class.values_multi_tender = values.values().filter(|(_, tenders)| tenders.len() >= 2).count() as u64;

    // Every 32-hex row by shape key, capped; the uuid_v4 passes under the same keys.
    let by_shape = capped_tallies(std::mem::take(&mut walk.by_shape), HEX_CENSUS_KEYS_MAX);
    for (key, n) in std::mem::take(&mut walk.v4_by_shape) {
        let key = if by_shape.contains_key(&key) { key } else { GUID_CENSUS_REST.to_owned() };
        *report.v4_by_shape.entry(key).or_default() += n;
    }
    report.by_shape = by_shape;
    report.notice_mentions.cf35_role_sets = capped_counts(std::mem::take(&mut walk.role_sets), HEX_CENSUS_KEYS_MAX);

    // Avenue-Web's tenderer keys (unit 5).
    let tk = &mut report.notice_mentions.tenderer_key;
    tk.rows = walk.tenderer_rows.len() as u64;
    tk.procedures = walk.tenderer_procedures.len() as u64;
    tk.collisions = walk.stripped.values().filter(|raws| raws.len() >= 2).count() as u64;
    report.samples.tenderer_key_collisions = walk
        .stripped
        .iter()
        .filter(|(_, raws)| raws.len() >= 2)
        .take(cap)
        .map(|(stripped, raws)| HexKeyCollision { stripped: stripped.clone(), raws: raws.iter().cloned().collect() })
        .collect();

    // The targets and the A6 screen.
    let mut targets: Vec<HexTarget> = Vec::with_capacity(walk.targets.len());
    for t in &walk.targets {
        let city_absent: u64 = if t.cities.is_empty() {
            0
        } else {
            t.planned.iter().filter(|(city, _)| !t.cities.contains(*city)).map(|(_, n)| *n).sum()
        };
        let target = HexTarget {
            name_norm: t.name_norm.clone(),
            country: t.country.clone(),
            published_name: t.published_name.clone(),
            id: t.id,
            row_name: t.row_name.clone(),
            target_mentions: t.target_mentions,
            cf35_mentions: t.cf35_mentions,
            cf35_rows: t.rows.len() as u64,
            cities: t.cities.iter().cloned().collect(),
            planned_cities: t.planned.keys().cloned().collect(),
            city_absent,
            multi_city: t.cities.len() >= 2,
            multi_city_planned: t.planned.len() >= 2,
        };
        let r = &mut report.targets;
        r.keys += 1;
        if target.id.is_some() {
            r.found += 1;
            r.mentions_to_found += target.cf35_mentions;
        } else {
            r.missing += 1;
            r.mentions_to_missing += target.cf35_mentions;
        }
        r.city.city_absent_mentions += city_absent;
        r.city.city_absent_targets += u64::from(city_absent > 0);
        r.city.multi_city_targets += u64::from(target.multi_city);
        r.city.planned_multi_city_keys += u64::from(target.multi_city_planned);
        targets.push(target);
    }
    targets.sort_by(|a, b| {
        b.cf35_mentions.cmp(&a.cf35_mentions).then_with(|| a.name_norm.cmp(&b.name_norm)).then_with(|| a.country.cmp(&b.country))
    });
    report.targets.top = targets.iter().take(HEX_CENSUS_TOP_TARGETS).cloned().collect();
    report.targets.city_flagged = targets
        .iter()
        .filter(|t| t.city_absent > 0 || t.multi_city || t.multi_city_planned)
        .take(GUID_TWIN_LIST_CAP)
        .cloned()
        .collect();
    report.targets.ccira = targets
        .iter()
        .filter(|t| guid_twin_city_fold(&t.name_norm).starts_with(HEX_CENSUS_CCIRA_PREFIX))
        .take(GUID_TWIN_LIST_CAP)
        .cloned()
        .collect();

    // The samples: R2's draw, the whole lists every bar names, the capped ones.
    let sample = |row: &ClassRow| {
        let first = row.first.as_ref();
        let target = first.and_then(|f| f.target).map(|at| &walk.targets[at as usize]);
        HexRowSample {
            org_id: row.id,
            country: row.country.clone(),
            name: row.name.clone(),
            identifier: row.identifier.clone(),
            raw: first.and_then(|f| f.raw.clone()),
            scheme: first.and_then(|f| f.scheme.clone()),
            mentions: row.mentions,
            cf35_mentions: row.cf35,
            notices: row.notices,
            tenders: row.tender_ids.len() as u64,
            tender_ids: row.tender_ids.iter().take(3).copied().collect(),
            notice_id: first.map(|f| f.notice),
            section_id: first.map(|f| f.section.clone()),
            roles: first.map(|f| f.roles.clone()).unwrap_or_default(),
            stratum: row.stratum,
            first_year: row.first_year,
            twin: row.twin,
            target_name_norm: target.map(|t| t.name_norm.clone()),
            target_country: target.and_then(|t| t.country.clone()),
            target_id: target.and_then(|t| t.id),
        }
    };
    let samples = &mut report.samples;
    for (stratum, n) in HEX_CENSUS_R2_STRATA {
        let mut picked: Vec<&ClassRow> = walk.rows.iter().filter(|r| r.stratum == stratum).collect();
        picked.sort_by_key(|r| guid_twin_sample_key(r.id, HEX_CENSUS_SALT));
        samples.r2_random.extend(picked.into_iter().take(n).map(sample));
    }
    let multi_value = |row: &ClassRow| values.get(&GuidKeep::key(&row.identifier)).is_some_and(|(_, t)| t.len() >= 2);
    samples.buyer = walk.rows.iter().filter(|r| r.buyer).take(GUID_TWIN_LIST_CAP).map(sample).collect();
    samples.multi_tender = walk.rows.iter().filter(|r| multi_value(*r)).take(GUID_TWIN_LIST_CAP).map(sample).collect();
    let mut stable_rows: Vec<&ClassRow> = walk.rows.iter().filter(|r| stable(*r)).collect();
    stable_rows.sort_by(|a, b| b.mentions.cmp(&a.mentions).then(a.id.cmp(&b.id)));
    samples.stable = stable_rows.into_iter().take(GUID_TWIN_LIST_CAP).map(sample).collect();
    samples.keep_530 = walk.rows.iter().filter(|r| r.kept).take(GUID_TWIN_LIST_CAP).map(sample).collect();
    samples.mixed_shape = walk
        .rows
        .iter()
        .filter(|r| r.shapes & cf35_bit != 0 && r.shapes & !(cf35_bit | none_bit) != 0)
        .take(cap)
        .map(sample)
        .collect();
    samples.blank_name = walk.rows.iter().filter(|r| r.blank_name).take(cap).map(sample).collect();

    // The canaries, in the given order.
    for id in args.canaries {
        match walk.row_at.get(id) {
            Some(&i) => report.canaries.push(sample(&walk.rows[i as usize])),
            None => report.canaries_absent.push(*id),
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guid_census::GUID_ROLE_OTHER;

    /// Issue 575 unit 1: every shape of a raw BT-501, trimmed and case-blind.
    #[test]
    fn a_raw_bt501_value_is_shaped_cf35_rfc36_hex32_tenderer_key_digits_other_or_none() {
        for raw in [
            "FD29FC2F-A2BF-49A1-9B234B870B531C27",
            "fd29fc2f-a2bf-49a1-9b234b870b531c27",
            "  FD29FC2F-A2BF-49A1-9B234B870B531C27  ",
        ] {
            assert_eq!(hex_raw_shape(Some(raw)), "cf35", "{raw}");
        }
        assert_eq!(hex_raw_shape(Some("FD29FC2F-A2BF-49A1-9B23-4B870B531C27")), "rfc36");
        assert_eq!(hex_raw_shape(Some("FD29FC2FA2BF49A19B234B870B531C27")), "hex32");
        for raw in [
            "FD29FC2F-A2BF-49A1-9B234B870B531C2",
            "GD29FC2F-A2BF-49A1-9B234B870B531C27",
            "1748141-1-0",
            "1748141-1-0-1-2",
            "DE 123",
        ] {
            assert_eq!(hex_raw_shape(Some(raw)), "other", "{raw}");
        }
        assert_eq!(hex_raw_shape(Some("1748141-1-0-1")), "tenderer_key");
        assert_eq!(hex_raw_shape(Some("67034")), "digits");
        assert_eq!(hex_raw_shape(None), "none");
        assert_eq!(hex_raw_shape(Some("  ")), "none");
        assert!(is_cf35("FD29FC2F-A2BF-49A1-9B234B870B531C27"));
        assert!(!is_cf35("FD29FC2F-A2BF-49A1-9B23-4B870B531C27"));
        for shape in ["cf35", "rfc36", "hex32", "tenderer_key", "digits", "other", "none"] {
            assert!(HEX_CENSUS_SHAPES.contains(&shape));
        }
    }

    /// The supervisor's role mapping, in miniature.
    fn role_class(role: &str) -> &'static str {
        match role {
            "Procedure-Buyer" | "buyer" | "purchasing-body" => GUID_ROLE_BUYER,
            "Tenderer" | "Tenderer-MainCont" | "Tenderer-SubCont" | "winner" => GUID_ROLE_TENDERER,
            r if r.starts_with("Lot-") || r.starts_with("Part-") => GUID_ROLE_REVIEW_INFO,
            _ => GUID_ROLE_OTHER,
        }
    }

    /// Issue 575 unit 1: a mention's roles take the first stratum of the precedence.
    #[test]
    fn a_role_set_takes_its_stratum_in_precedence() {
        for (roles, expected) in [
            (vec!["Procedure-SProvider"], "sprovider"),
            (vec!["service-provider"], "sprovider"),
            (vec!["Lot-ReviewOrg"], "review"),
            (vec!["Part-ReviewInfo"], "review"),
            (vec!["review-body"], "review"),
            (vec!["appeal-information"], "review"),
            (vec!["Lot-TenderReceipt"], "review"),
            (vec!["Lot-Mediator"], "mediator"),
            (vec!["mediation-body"], "mediator"),
            (vec!["Lot-AddInfo"], "add_info"),
            (vec!["further-information"], "add_info"),
            (vec!["Lot-AddInfo", "Procedure-Buyer"], "buyer"),
            (vec!["Lot-AddInfo", "Lot-ReviewOrg"], "add_info"),
            (vec!["Tenderer"], "tenderer"),
            (vec!["bid-tenderer"], "tenderer"),
            (vec!["Contract-Signatory"], "other"),
            (vec![], "no_role"),
        ] {
            let owned: Vec<String> = roles.iter().map(|r| (*r).to_owned()).collect();
            assert_eq!(hex_stratum(&owned, role_class), expected, "{roles:?}");
            assert!(HEX_CENSUS_STRATA.contains(&expected));
        }
    }

    // ---- the fixture census ----

    /// Issue 300's class as the job injects it (`idgate::census(..).hex_hash`), mirrored.
    fn hex_hash(value: &str) -> bool {
        value.len() >= 16
            && value.bytes().all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(&b))
            && value.bytes().any(|b| b.is_ascii_alphabetic())
    }

    /// `ingest::idgate::uuid_v4`, mirrored: store never depends on ingest.
    fn uuid_v4(value: &str) -> bool {
        let hex: Vec<u8> = value.bytes().filter(|b| *b != b'-').collect();
        hex.len() == 32
            && hex.iter().all(u8::is_ascii_hexdigit)
            && hex[12] == b'4'
            && matches!(hex[16].to_ascii_lowercase(), b'8' | b'9' | b'a' | b'b')
    }

    async fn open(name: &str, tender_indexes: bool) -> (crate::Db, turso::Connection, String) {
        let path = format!("/tmp/tender-db-hex-census-{name}-{}.db", std::process::id());
        cleanup(&path);
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

    /// An organization; identifier NULL makes it a provisional name-keyed row (a fallback target).
    async fn org(conn: &turso::Connection, id: i64, country: &str, identifier: Option<&str>, name: &str) {
        conn.execute(
            "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, 0)",
            (
                Value::Integer(id),
                Value::Text(country.into()),
                identifier.map_or(Value::Null, |_| Value::Text("national".into())),
                identifier.map_or(Value::Null, |v| Value::Text(v.into())),
                Value::Text(name.into()),
                Value::Text(org_name_norm(name)),
                Value::Integer(if identifier.is_some() { 0 } else { 1 }),
            ),
        )
        .await
        .unwrap();
    }

    /// A notice of `source`, published at `at`, causing version `seq` of `tender`.
    async fn notice(conn: &turso::Connection, id: i64, source: &str, profile: &str, at: i64, tender: i64, seq: i64) {
        conn.execute(
            "INSERT INTO notices (id, source, publication_id, content_hash, profile, fetch_id, member_path, ingested_at,
                                  published_at, parse_state)
             VALUES (?, ?, ?, 'h', ?, 1, 'm', 0, ?, 'parsed')",
            (
                Value::Integer(id),
                Value::Text(source.into()),
                Value::Text(format!("{id}-2024")),
                Value::Text(profile.into()),
                Value::Integer(at),
            ),
        )
        .await
        .unwrap();
        conn.execute(
            "INSERT INTO tender_versions (tender_id, seq, caused_by_notice_id, published_at, publication_id)
             VALUES (?, ?, ?, ?, ?)",
            (Value::Integer(tender), Value::Integer(seq), Value::Integer(id), Value::Integer(at), Value::Text(format!("{id}-2024"))),
        )
        .await
        .unwrap();
    }

    /// A mention of `org` in `notice`'s section `section`, as published, in `role` on the notice's
    /// version (`tender`, `seq`).
    #[allow(clippy::too_many_arguments)]
    async fn mention(
        conn: &turso::Connection,
        notice: i64,
        section: &str,
        org: i64,
        name: Option<&str>,
        country: &str,
        raw: Option<&str>,
        scheme: Option<&str>,
        role: &str,
        tender: i64,
        seq: i64,
    ) {
        let opt = |v: Option<&str>| v.map_or(Value::Null, |v| Value::Text(v.into()));
        conn.execute(
            "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, country, raw_identifier, scheme)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            (
                Value::Integer(notice),
                Value::Text(section.into()),
                Value::Integer(org),
                opt(name),
                Value::Text(country.into()),
                opt(raw),
                opt(scheme),
            ),
        )
        .await
        .unwrap();
        conn.execute(
            "INSERT INTO tender_version_parties (tender_id, seq, lot_id, role, organization_id, mention_notice_id, mention_section_id)
             VALUES (?, ?, NULL, ?, ?, ?, ?)",
            (
                Value::Integer(tender),
                Value::Integer(seq),
                Value::Text(role.into()),
                Value::Integer(org),
                Value::Integer(notice),
                Value::Text(section.into()),
            ),
        )
        .await
        .unwrap();
    }

    /// A BT-513 city of one ORG section.
    async fn city(conn: &turso::Connection, notice: i64, section: &str, value: &str) {
        conn.execute(
            "INSERT INTO notice_texts (notice_id, section_id, field_id, ordinal, lang, value)
             VALUES (?, ?, 'BT-513-Organization-Company', 0, NULL, ?)",
            (Value::Integer(notice), Value::Text(section.into()), Value::Text(value.into())),
        )
        .await
        .unwrap();
    }

    /// 2024-06-01, 2025-06-01 and 2026-09-22, UTC.
    const Y2024: i64 = 1_717_200_000;
    const Y2025: i64 = 1_748_736_000;
    const Y2026: i64 = 1_790_035_200;
    const SDK: &str = "eforms:eforms-sdk-1.10";
    const AVENUE: &str = "Avenue-Web Systèmes";
    const TA_PARIS: &str = "Tribunal administratif de Paris";
    const GREFFE_PARIS: &str = "Greffe du Tribunal administratif de Paris";
    const TA_SAINT_DENIS: &str = "Tribunal administratif de Saint-Denis";
    const CCIRA_SGAR: &str = "Comité Consultatif Interrégional - Prefecture de Région SGAR";
    const CCIRA_LONG: &str =
        "Comité consultatif interrégional de réglement amiable des différends ou litiges relatifs aux marchés publics";

    /// One ORG mention of the fixture: (section, org, name, country, raw, scheme, role).
    type FixtureMention = (&'static str, i64, Option<&'static str>, &'static str, Option<&'static str>, Option<&'static str>, &'static str);

    /// The fixture, from 575's reads and the cn-fa fixture (`cn-fa-expected-00586487-2024.xml`): notice 277's
    /// eSender, buyer, TA Paris twin and tenderer key; 586487's TA Saint-Denis twin and long CCIRA form;
    /// 24267121's SGAR CCIRA row; 27863637 and 44493442's per-section mints (and a dash-strip tenderer-key
    /// collision); then one shape each — a recurring greffe (two Tenders), a blank name, a buyer, a row
    /// mixing cf35 and hex32 across a TED+DÖE pair, a KEEP overlap, a hex32-only row, an rfc36 bidder, and
    /// the identifier-NULL fallback targets. Each org row is minted at its first mention, its identifier
    /// the raw value with its dashes dropped and upper-cased (`normalise_identifier_with`'s result).
    fn fixture() -> Vec<(i64, &'static str, &'static str, i64, i64, i64, Vec<FixtureMention>)> {
        let eu = Some("eu");
        vec![
            (277, "ted", SDK, Y2024, 900_001, 1, vec![
                ("ORG-0001", 859, Some(AVENUE), "FR", Some("FD29FBDB-9AE2-96A7-C8EFDA456F9EE7FE"), eu, "Procedure-SProvider"),
                ("ORG-0002", 860, Some("CNFPT - Direction de l'achat public"), "FR", Some("67034"), eu, "Procedure-Buyer"),
                ("ORG-0003", 861, Some(TA_PARIS), "FR", Some("FD29FC01-04FA-2E6D-2EB3FB1D8FFA4F7F"), eu, "Lot-ReviewOrg"),
                ("ORG-0004", 862, Some(TA_PARIS), "FR", Some("FD29FC2F-A2BF-49A1-9B234B870B531C27"), eu, "Lot-ReviewInfo"),
                ("ORG-0005", 863, Some("OPEN"), "FR", Some("1748141-1-0-1"), eu, "Tenderer"),
            ]),
            (586_487, "ted", SDK, Y2024, 900_004, 1, vec![
                ("ORG-0001", 22_900_001, Some(AVENUE), "FR", Some("D0E86039-C408-988C-36D640329E1FF921"), eu, "Procedure-SProvider"),
                ("ORG-0002", 22_900_002, Some("Département de la Réunion"), "RE", Some("30184"), eu, "Procedure-Buyer"),
                ("ORG-0003", 22_900_003, Some(TA_SAINT_DENIS), "RE", Some("D0E86056-B06B-F5C3-30CF1345EE066316"), eu, "Lot-ReviewOrg"),
                ("ORG-0004", 22_900_004, Some(CCIRA_LONG), "FR", Some("D0E86064-A547-F896-993DD7E12F048721"), eu, "Lot-Mediator"),
                ("ORG-0005", 22_900_005, Some(TA_SAINT_DENIS), "RE", Some("D0E86078-E2C8-D805-2F8D363577B05AD2"), eu, "Lot-ReviewInfo"),
            ]),
            (24_267_121, "ted", SDK, Y2024, 900_005, 1, vec![
                ("ORG-0001", 22_763_200, Some(AVENUE), "FR", Some("C779DE5A-1234-5678-9ABCDEF012345678"), eu, "Procedure-SProvider"),
                ("ORG-0004", 22_763_203, Some(CCIRA_SGAR), "FR", Some("C779DE6D-F66E-FD50-71593CD9E358F4ED"), eu, "Lot-Mediator"),
            ]),
            (27_863_637, "ted", SDK, Y2025, 900_002, 1, vec![
                ("ORG-0001", 24_606_517, Some(AVENUE), "FR", Some("35FACF6C-0A1B-2C3D-4E5F60718293A4B5"), eu, "Procedure-SProvider"),
                ("ORG-0002", 1_166_503, Some("Ville de Paris"), "FR", Some("5041"), eu, "Procedure-Buyer"),
                ("ORG-0003", 24_606_518, Some(TA_PARIS), "FR", Some("35FACF8B-1B2C-3D4E-5F60718293A4B5C6"), eu, "Lot-ReviewOrg"),
                ("ORG-0004", 24_606_519, Some(GREFFE_PARIS), "FR", Some("35FACF9C-2C3D-4E5F-60718293A4B5C6D7"), eu, "Lot-ReviewInfo"),
                ("ORG-0005", 24_606_520, Some("SOCIETE A"), "FR", Some("1668803-1-11-1"), eu, "Tenderer"),
                ("ORG-0006", 24_606_520, Some("SOCIETE A"), "FR", Some("1668803-1-1-11"), eu, "Tenderer"),
            ]),
            (44_493_442, "ted", SDK, Y2026, 900_003, 1, vec![
                ("ORG-0001", 30_934_846, Some(AVENUE), "FR", Some("54BD44A7-3D4E-5F60-718293A4B5C6D7E8"), eu, "Procedure-SProvider"),
                ("ORG-0002", 1_166_503, Some("Ville de Paris"), "FR", Some("5041"), eu, "Procedure-Buyer"),
                ("ORG-0003", 30_934_847, Some(TA_PARIS), "FR", Some("54BD44C8-4E5F-6071-8293A4B5C6D7E8F9"), eu, "Lot-ReviewOrg"),
                ("ORG-0004", 30_934_848, Some(GREFFE_PARIS), "FR", Some("54BD44DC-5F60-7182-93A4B5C6D7E8F90A"), eu, "Lot-ReviewInfo"),
            ]),
            (45_000_002, "ted", SDK, Y2026, 900_102, 1, vec![
                ("ORG-0001", 30_000_201, Some("Greffe Recurrent"), "FR", Some("AAAA0001-0000-1000-0000000000000001"), None, "Lot-ReviewInfo"),
                ("ORG-0002", 30_000_301, None, "FR", Some("AAAA0002-0000-1000-0000000000000002"), None, "Lot-ReviewOrg"),
                ("ORG-0003", 30_000_101, Some("OPH Testville"), "FR", Some("AAAA0003-0000-1000-0000000000000003"), None, "Procedure-Buyer"),
            ]),
            (45_000_003, "ted", SDK, Y2026, 900_103, 1, vec![
                ("ORG-0003", 30_000_201, Some("Greffe Recurrent"), "FR", Some("AAAA0001-0000-1000-0000000000000001"), None, "Lot-ReviewInfo"),
            ]),
            (45_000_004, "ted", SDK, Y2026, 900_104, 1, vec![
                ("ORG-0001", 30_000_401, Some("Mixed Body"), "FR", Some("BBBB0001-0000-1000-0000000000000001"), None, "Lot-ReviewOrg"),
                ("ORG-0002", 30_000_501, Some("Kept Body"), "FR", Some("CCCC0001-0000-1000-0000000000000001"), None, "Lot-AddInfo"),
            ]),
            // The DÖE copy of 45000004's award (tender 900104, version 2).
            (45_000_005, "doe", "eforms:eforms-de-1.2", Y2026, 900_104, 2, vec![
                ("ORG-0001", 30_000_401, Some("Mixed Body"), "FR", Some("bbbb0001000010000000000000000001"), None, "Lot-ReviewOrg"),
                ("ORG-0002", 30_000_601, Some("MD5 GmbH"), "DE", Some("0123456789abcdef0123456789abcdef"), None, "Procedure-Buyer"),
            ]),
            (45_000_006, "ted", SDK, Y2026, 900_106, 1, vec![
                ("ORG-0001", 8_378, Some("LEONHARD WEISS GmbH & Co. KG"), "DE", Some("9a9357bc-0d1e-4f2a-8b3c-4d5e6f7a8b9c"), None, "Tenderer"),
                ("ORG-0002", 1_163_000, Some(TA_PARIS), "FR", None, None, "Lot-ReviewOrg"),
                ("ORG-0003", 19_913_046, Some(CCIRA_SGAR), "FR", None, None, "Lot-Mediator"),
                ("ORG-0004", 1_163_999, Some(TA_SAINT_DENIS), "FR", None, None, "Lot-ReviewOrg"),
            ]),
        ]
    }

    async fn seed(conn: &turso::Connection) {
        let mut minted: HashSet<i64> = HashSet::new();
        for (notice_id, source, profile, at, tender, seq, mentions) in fixture() {
            notice(conn, notice_id, source, profile, at, tender, seq).await;
            for (section, org_id, name, country, raw, scheme, role) in mentions {
                if minted.insert(org_id) {
                    // The resolver stores register_jurisdiction(RE) = FR for the class rows; the buyer
                    // 22900002 keeps RE, and the two DE rows are DE.
                    let org_country = match org_id {
                        22_900_002 => "RE",
                        8_378 | 30_000_601 => "DE",
                        _ => "FR",
                    };
                    let identifier = raw.map(|r| r.replace('-', "").to_uppercase());
                    org(conn, org_id, org_country, identifier.as_deref(), name.unwrap_or("")).await;
                }
                mention(conn, notice_id, section, org_id, name, country, raw, scheme, role, tender, seq).await;
            }
        }
        // Two rows with no mention: 18 hex (issue 300's class, not 32 hex) and 20 characters (neither).
        org(conn, 30_000_701, "DE", Some("ABCDEF0123456789AB"), "Hash Short").await;
        org(conn, 30_000_801, "FR", Some("FR12345678901234567X"), "Long Number").await;
        for (notice_id, section, value) in [
            (277, "ORG-0001", "Seyssinet-Pariset"),
            (277, "ORG-0003", "Paris"),
            (277, "ORG-0004", "Paris"),
            (586_487, "ORG-0001", "Seyssinet-Pariset"),
            (586_487, "ORG-0003", "Saint-Denis de la Réunion"),
            (586_487, "ORG-0004", "Paris"),
            (586_487, "ORG-0005", "Saint-Denis de la Réunion"),
            (24_267_121, "ORG-0004", "Lyon"),
            (27_863_637, "ORG-0003", "Paris"),
            (27_863_637, "ORG-0004", "Paris"),
            (44_493_442, "ORG-0003", "PARIS"),
            (44_493_442, "ORG-0004", "Marseille"),
            (45_000_006, "ORG-0002", "Paris"),
            (45_000_006, "ORG-0003", "Marseille"),
            (45_000_006, "ORG-0004", "Saint-Denis de la Réunion"),
        ] {
            city(conn, notice_id, section, value).await;
        }
    }

    fn args<'a>(
        stop: &'a (dyn Fn() -> bool + Sync),
        progress: &'a (dyn Fn(u64, &str) + Sync),
        keep: &'a GuidKeep,
    ) -> HexCensusArgs<'a> {
        HexCensusArgs {
            hex_hash,
            is_v4: uuid_v4,
            role_class,
            keep,
            stable_mentions: 100,
            samples: 30,
            // Narrow enough that the walk crosses several windows before its one batch.
            window: 5_000_000,
            batch: HEX_CENSUS_BATCH,
            canaries: &[859, 862, 22_763_203, 860],
            stop,
            progress,
        }
    }

    fn ids(list: &[HexRowSample]) -> Vec<i64> {
        list.iter().map(|s| s.org_id).collect()
    }

    /// Issue 575 unit 1: every count of the census on one fixture of each shape.
    #[tokio::test]
    async fn the_hex_census_sizes_the_cf35_class_its_bars_targets_and_tenderer_keys() {
        let (db, conn, path) = open("counts", true).await;
        seed(&conn).await;
        let keep: GuidKeep = ["CCCC0001-0000-1000-0000000000000001"].into_iter().collect();
        let stop = || false;
        let progress = |_: u64, _: &str| {};
        let r = db.hex_census(args(&stop, &progress, &keep)).await.unwrap();

        // The walk.
        assert!(!r.stopped);
        assert_eq!(r.max_id, 30_934_848);
        assert_eq!(r.windows, 7);
        assert_eq!(r.id_rows_16plus, 24, "22 32-hex rows, the 18-hex row and the 20-character one");
        assert_eq!(r.hex_hash_rows, 23, "every 32-hex row and the 18-hex one");
        assert_eq!(r.hex32_rows, 22);
        assert_eq!(r.hex32_mentions, 24);
        assert_eq!(r.salt, HEX_CENSUS_SALT);
        assert_eq!(r.not_measured.len(), 3);

        // The class: 20 rows with a cf35 mention (not the hex32-only 30000601, not the rfc36 8378).
        let all = r.class.all;
        assert_eq!((all.rows, all.mentions, all.one_mention), (20, 22, 18));
        assert_eq!((all.one_tender, all.multi_tender, all.no_tender, all.no_mentions), (19, 1, 0, 0));
        assert_eq!((all.multi_notice, all.stable), (2, 0), "30000201 and 30000401 span two notices");
        assert_eq!((r.class.cf35_mentions, r.class.other_shape_mentions, r.class.mixed_shape_rows), (21, 1, 1));
        assert_eq!((r.class.notices, r.class.tenders), (8, 8));
        assert_eq!((r.class.values_multi_row, r.class.values_multi_tender), (0, 1), "30000201 spans two Tenders");
        assert_eq!(r.class.keep_530_overlap, 1, "30000501");
        assert_eq!(r.class.v4_rows, 1, "862");
        assert_eq!((r.class.first_notice_id, r.class.first_published), (Some(277), Some(Y2024)));
        assert_eq!(r.class.by_country["FR"].rows, 20);
        assert_eq!(r.class.by_first_year["2024"].rows, 9);
        assert_eq!(r.class.by_first_year["2025"].rows, 3);
        assert_eq!(r.class.by_first_year["2026"].rows, 8);
        assert_eq!(r.class.by_source["ted"].rows, 19);
        assert_eq!(r.class.by_source["doe+ted"].rows, 1);
        for (stratum, rows) in [("sprovider", 5), ("review", 11), ("mediator", 2), ("add_info", 1), ("buyer", 1)] {
            assert_eq!(r.class.by_stratum[stratum].rows, rows, "{stratum}");
        }
        assert_eq!(r.by_shape["ted cf35"].rows, 19);
        assert_eq!(r.by_shape["doe+ted cf35+hex32"].rows, 1);
        assert_eq!(r.by_shape["doe hex32"].rows, 1);
        assert_eq!(r.by_shape["ted rfc36"].rows, 1);
        assert_eq!(r.v4_by_shape, BTreeMap::from([("ted cf35".to_owned(), 1), ("ted rfc36".to_owned(), 1)]));

        // Phase 2: every ORG mention of the eight cf35 notices.
        let nm = &r.notice_mentions;
        assert_eq!((nm.notices, nm.mentions), (8, 28));
        assert_eq!(
            nm.by_shape,
            BTreeMap::from([("cf35".to_owned(), 21), ("digits".to_owned(), 4), ("tenderer_key".to_owned(), 3)])
        );
        assert_eq!((nm.cf35_on_class_rows, nm.cf35_off_class, nm.cf35_above_max_id), (21, 0, 0));
        assert_eq!(nm.cf35_buyer_mentions, 1, "30000101");
        assert_eq!((nm.cf35_blank_name_mentions, nm.cf35_unkeyed_mentions), (1, 1), "30000301");
        fn counts(pairs: &[(&str, u64)]) -> BTreeMap<String, u64> {
            pairs.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect()
        }
        assert_eq!(
            nm.cf35_by_stratum,
            counts(&[("sprovider", 5), ("review", 12), ("mediator", 2), ("add_info", 1), ("buyer", 1)])
        );
        assert_eq!(
            nm.by_shape_role["cf35"],
            counts(&[
                ("Procedure-SProvider", 5),
                ("Lot-ReviewOrg", 6),
                ("Lot-ReviewInfo", 6),
                ("Lot-Mediator", 2),
                ("Lot-AddInfo", 1),
                ("Procedure-Buyer", 1),
            ])
        );
        assert_eq!(nm.by_shape_role["digits"]["Procedure-Buyer"], 4);
        assert_eq!(nm.by_shape_role["tenderer_key"]["Tenderer"], 3);
        assert_eq!(nm.by_shape_scheme["cf35"], counts(&[("eu", 15), ("(none)", 6)]));
        assert_eq!(nm.by_shape_scheme["digits"]["eu"], 4);
        assert_eq!(nm.twin, HexTwin { notices: 2, groups: 2, mentions: 4, rows: 4, sprovider_rows: 0 });
        assert_eq!(nm.tenderer_key, HexTendererKeys { mentions: 3, rows: 2, procedures: 2, collisions: 1 });

        // Phase 3: the targets.
        let t = &r.targets;
        assert_eq!((t.keys, t.found, t.missing, t.mentions_to_found, t.mentions_to_missing), (10, 3, 7, 7, 13));
        assert_eq!(
            t.city,
            HexCityScreen {
                mentions_with_city: 12,
                mentions_without_city: 8,
                city_absent_mentions: 1,
                city_absent_targets: 1,
                multi_city_targets: 0,
                planned_multi_city_keys: 1,
            }
        );
        assert_eq!(t.top.len(), 10);
        assert_eq!(t.top[0].name_norm, "avenue-web systèmes");
        assert_eq!((t.top[0].id, t.top[0].cf35_mentions, t.top[0].cf35_rows), (None, 5, 5));
        assert_eq!(t.top[1].name_norm, "tribunal administratif de paris");
        assert_eq!((t.top[1].id, t.top[1].target_mentions, t.top[1].cf35_mentions), (Some(1_163_000), 1, 4));
        assert_eq!(t.top[1].cities, vec!["paris".to_owned()]);
        let flagged: Vec<(&str, Option<i64>, u64, bool)> =
            t.city_flagged.iter().map(|x| (x.name_norm.as_str(), x.id, x.city_absent, x.multi_city_planned)).collect();
        assert_eq!(
            flagged,
            vec![
                ("greffe du tribunal administratif de paris", None, 0, true),
                ("comité consultatif interrégional - prefecture de région sgar", Some(19_913_046), 1, false),
            ]
        );
        // `-` sorts before `d`: the SGAR key, then the long form (missing).
        let ccira: Vec<(String, Option<i64>)> = t.ccira.iter().map(|x| (x.name_norm.clone(), x.id)).collect();
        assert_eq!(ccira, vec![(org_name_norm(CCIRA_SGAR), Some(19_913_046)), (org_name_norm(CCIRA_LONG), None)]);

        // The samples.
        let mut expected: Vec<i64> = Vec::new();
        for (stratum, members) in [
            ("sprovider", vec![859, 22_900_001, 22_763_200, 24_606_517, 30_934_846]),
            (
                "review",
                vec![
                    861, 862, 22_900_003, 22_900_005, 24_606_518, 24_606_519, 30_934_847, 30_934_848, 30_000_201,
                    30_000_301, 30_000_401,
                ],
            ),
            ("mediator", vec![22_900_004, 22_763_203]),
            ("add_info", vec![30_000_501]),
        ] {
            let mut members = members;
            members.sort_by_key(|&id| guid_twin_sample_key(id, HEX_CENSUS_SALT));
            let n = HEX_CENSUS_R2_STRATA.iter().find(|(s, _)| *s == stratum).map(|(_, n)| *n).unwrap();
            expected.extend(members.into_iter().take(n));
        }
        assert_eq!(r.samples.r2_random.len(), 19);
        assert_eq!(ids(&r.samples.r2_random), expected, "each stratum in salted order");
        assert_eq!(ids(&r.samples.buyer), vec![30_000_101]);
        assert_eq!(ids(&r.samples.multi_tender), vec![30_000_201]);
        assert_eq!(ids(&r.samples.mixed_shape), vec![30_000_401]);
        assert_eq!(ids(&r.samples.keep_530), vec![30_000_501]);
        assert_eq!(ids(&r.samples.blank_name), vec![30_000_301]);
        assert!(r.samples.stable.is_empty());
        let twins: Vec<(i64, Vec<i64>)> = r.samples.twin_groups.iter().map(|g| (g.notice_id, g.org_ids.clone())).collect();
        assert_eq!(twins, vec![(277, vec![861, 862]), (586_487, vec![22_900_003, 22_900_005])]);
        assert_eq!(
            r.samples.tenderer_key_collisions,
            vec![HexKeyCollision {
                stripped: "16688031111".to_owned(),
                raws: vec!["1668803-1-1-11".to_owned(), "1668803-1-11-1".to_owned()],
            }]
        );

        // The canaries: 860 (the buyer, `67034`) is no 32-hex row.
        assert_eq!(r.canaries.len(), 3);
        assert_eq!(r.canaries_absent, vec![860]);
        let canary = |id: i64| r.canaries.iter().find(|c| c.org_id == id).cloned().expect("canary");
        let esender = canary(859);
        assert_eq!((esender.stratum, esender.target_id), ("sprovider", None));
        assert_eq!(esender.target_name_norm.as_deref(), Some("avenue-web systèmes"), "keyed, missing");
        let review = canary(862);
        assert_eq!((review.stratum, review.twin, review.target_id), ("review", true, Some(1_163_000)));
        let ccira_row = canary(22_763_203);
        assert_eq!((ccira_row.stratum, ccira_row.target_id), ("mediator", Some(19_913_046)));

        // One row per batch: every count but `notices_read` (summed per batch) is the same.
        let one = db.hex_census(HexCensusArgs { batch: 1, ..args(&stop, &progress, &keep) }).await.unwrap();
        let (mut a, mut b) = (r.clone(), one);
        (a.notices_read, b.notices_read) = (0, 0);
        assert_eq!(a, b, "a batch of one reads what a batch of 512 does");

        // At `stable_mentions: 2` the two-mention rows are stable, listed by mentions then id.
        let low = db.hex_census(HexCensusArgs { stable_mentions: 2, ..args(&stop, &progress, &keep) }).await.unwrap();
        assert_eq!(low.class.all.stable, 2);
        assert_eq!(ids(&low.samples.stable), vec![30_000_201, 30_000_401]);
        drop(conn);
        cleanup(&path);
    }

    /// Issue 575 unit 1: without its indexes the census refuses before reading; a stop on any read of the
    /// flag returns an empty report marked stopped, and a flag that never fires lets it complete.
    #[tokio::test]
    async fn the_hex_census_refuses_without_its_indexes_and_stops_at_every_read() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let (db, conn, path) = open("refuse", false).await;
        seed(&conn).await;
        let keep = GuidKeep::default();
        let never = || false;
        let progress = |_: u64, _: &str| {};
        let err = db.hex_census(args(&never, &progress, &keep)).await.expect_err("the mention-key indexes are deferred");
        assert!(err.to_string().contains("hex-census refused"), "{err}");
        assert!(err.to_string().contains("tender_version_parties_mention_key is absent"), "{err}");
        db.build_tender_indexes().await.unwrap();

        // Every read of the flag in one full run.
        let calls = AtomicUsize::new(0);
        let count = || {
            calls.fetch_add(1, Ordering::Relaxed);
            false
        };
        let full = db.hex_census(args(&count, &progress, &keep)).await.unwrap();
        assert_eq!(full.class.all.rows, 20);
        let total = calls.load(Ordering::Relaxed);
        assert!(total >= 10, "the flag is read before every window, batch, chunk and 64 target seeks ({total} reads)");

        for at in 1..=total + 1 {
            let calls = AtomicUsize::new(0);
            let stop = || calls.fetch_add(1, Ordering::Relaxed) + 1 >= at;
            let r = db.hex_census(args(&stop, &progress, &keep)).await.unwrap();
            assert_eq!(r.stopped, at <= total, "a stop on call {at} of {total}");
            assert_eq!(r.class.all.rows, if at <= total { 0 } else { 20 }, "a stop on call {at} of {total}");
        }
        drop(conn);
        cleanup(&path);
    }

    /// Issue 575 unit 1: the window statement seeks by rowid (the reused statements are pinned by
    /// `the_guid_census_reads_by_index_at_the_chunk_width` and `the_twin_planner_reads_by_index_at_the_chunk_width`).
    #[tokio::test]
    async fn the_hex_census_window_seeks_by_rowid() {
        let (db, conn, path) = open("eqp", true).await;
        drop(conn);
        let reader = db.reader().await.unwrap();
        let params = vec![Value::Integer(0), Value::Integer(HEX_CENSUS_WINDOW)];
        let mut rows = reader.query(&format!("EXPLAIN QUERY PLAN {HEX_CENSUS_ORGS_SQL}"), params.clone()).await.unwrap();
        let mut plan = String::new();
        while let Some(r) = rows.next().await.unwrap() {
            plan.push_str(&r.get_value(3).ok().and_then(|v| v.as_text().cloned()).unwrap_or_default());
            plan.push('\n');
        }
        drop(rows);
        println!("PLAN window:\n{plan}");
        assert!(plan.lines().any(|l| l.contains("SEARCH organizations")), "the window seeks:\n{plan}");
        assert!(!plan.lines().any(|l| l.trim_start().starts_with("SCAN ")), "the window scans:\n{plan}");
        // The plan reads `rowid=?` whatever its bounds, so the bytecode is what shows the walk seeks to the
        // window's lower id and stops at its upper one.
        let mut rows = reader.query(&format!("EXPLAIN {HEX_CENSUS_ORGS_SQL}"), params).await.unwrap();
        let mut ops: Vec<(String, String)> = Vec::new();
        while let Some(r) = rows.next().await.unwrap() {
            let text = |i| r.get_value(i).ok().and_then(|v| v.as_text().cloned()).unwrap_or_default();
            ops.push((text(1), text(7)));
        }
        println!("EXPLAIN window:\n{ops:#?}");
        assert!(ops.iter().any(|(op, _)| op == "SeekGT"), "the window seeks its lower id: {ops:#?}");
        assert!(!ops.iter().any(|(op, _)| op == "Rewind"), "the window walks the table: {ops:#?}");
        assert!(
            ops.windows(2).any(|w| w[0].0 == "RowId" && matches!(w[1].0.as_str(), "Gt" | "Ge")),
            "the window stops at its upper id: {ops:#?}"
        );
        drop(rows);
        drop(reader);
        cleanup(&path);
    }
}
