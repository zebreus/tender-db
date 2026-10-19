//! Issue 530 unit 1: the `guid-census` — the v4-GUID identifier class sized by the distinct Tenders each
//! row's mentions span, with the two refusal signals unit 2 weighs and the stable-platform rows as their
//! own population.
//!
//! Issue 312 measured the class for false merges (rows per distinct value, about 1.0) and for linking on a
//! 306-row consortium sample ("93% span more than one notice"), and declined the condemn. Some German
//! eForms platforms mint a fresh BT-501 UUID per ORG section per notice, so one organization becomes a row
//! per notice: Landkreis Göttingen holds 52 GUID rows at 2 mentions each, the TED and DÖE copies of one
//! award. A split leaves one row per key too, so 312's measure cannot see it, and counting notices counts
//! each DÖE+TED double copy twice. This census counts TENDERS, never notices, and reports:
//!
//! 1. the share of GUID rows whose mentions span ONE distinct Tender (`one_tender`, the single-use rows),
//!    by country, role class and platform;
//! 2. the within-notice twin signal: notices where two mentions of DIFFERENT GUID rows share an
//!    `org_name_norm` of their published names, and the single-use rows it reaches (unit 2 (a)) under
//!    both readings of that rule: the PAIR (only the rows of the name the notice holds twice, `twin`) and
//!    the NOTICE (every GUID row the twin notice mentions, `twin_notice`: the notice proves the platform
//!    mints per ORG section, so its winner's GUID is no key either), with the stable rows each would
//!    refuse;
//! 3. the cross-notice signal: rows whose `(org_name_norm(name), country)` already holds a different,
//!    EARLIER (lower id) GUID row with at most [`GUID_CENSUS_FEW_MENTIONS`] mentions, and the single-use
//!    rows it reaches (unit 2 (d));
//! 4. how often (3) fires on a name that also holds a STABLE GUID row, and the stable rows it reaches;
//! 5. 312's "span more than one notice" beside the same share on the distinct-Tender basis, for every
//!    GUID row, the stable population (the injected ids — 254, 255 and 8378 in the job — and every GUID
//!    row with at least `stable_mentions` mentions) and the rest.
//!
//! This writes NOTHING. The class predicate is injected (`ingest::idgate::uuid_v4` in the job: 32 hex
//! digits, dashed or not, any case, version nibble 4 and an RFC-4122 variant nibble — the shape issue 312
//! counted and unit 3 would gate on); store never depends on ingest. So is the role vocabulary
//! (`role_class`).

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::hash::{Hash, Hasher};

use serde::Serialize;
use turso::Value;

use crate::canonical::{IN_CHUNK, org_name_norm, party_role_counts_sql, placeholders, unix_year};
use crate::{Db, int, opt_int_of, opt_text_of, text};

/// Issue 530 unit 1: organization ids per window of the walk — a rowid range of `organizations`, so no
/// statement reads more than this many rows, whatever the class's density.
pub const GUID_CENSUS_WINDOW: i64 = 10_000;

/// Issue 530 unit 1: the cross-notice signal's bar (unit 2 (d)): an earlier GUID row under the same
/// `(name_norm, country)` with at most this many mentions — a TED+DÖE pair of one notice.
pub const GUID_CENSUS_FEW_MENTIONS: u64 = 2;

/// Issue 530 unit 1: the most keys `by_platform` and `by_profile` each name, the largest by rows; the
/// rest are summed under [`GUID_CENSUS_REST`].
pub const GUID_CENSUS_KEYS_MAX: usize = 60;

/// Issue 530 unit 1: the label of the summed rest of a capped breakdown.
pub const GUID_CENSUS_REST: &str = "(rest)";

/// Issue 530 unit 1: the most GUID mentions the twin signal keeps, 24 bytes each: about 192 MB at the cap,
/// where the buffer is grown exactly to it rather than doubled past it. That last growth copies, so for a
/// moment the old buffer is live too (about 290 MB allocated, about 200 MB resident). Past the cap the twin
/// counts are a lower bound and `mention_ceiling_hit` says so; every other count is whole.
pub const GUID_CENSUS_MENTION_CEILING: usize = 8_000_000;

/// Issue 530 unit 1: GUID rows per batch, each of its reads an IN chunk of at most this many keys.
pub const GUID_CENSUS_BATCH: usize = IN_CHUNK;

/// Issue 530 unit 1: the notice and Tender ids a sample carries, the lowest first.
pub const GUID_CENSUS_SAMPLE_IDS: usize = 3;

/// Issue 530 unit 1: the role classes, in the precedence a row whose mentions hold several takes
/// (`role_mixed` counts those rows). A class name is what the injected `role_class` returns.
pub const GUID_ROLE_BUYER: &str = "buyer";
pub const GUID_ROLE_TENDERER: &str = "tenderer_winner";
pub const GUID_ROLE_REVIEW_INFO: &str = "review_or_info";
pub const GUID_ROLE_OTHER: &str = "other";
pub const GUID_ROLE_CLASSES: [&str; 4] = [GUID_ROLE_BUYER, GUID_ROLE_TENDERER, GUID_ROLE_REVIEW_INFO, GUID_ROLE_OTHER];
/// A GUID row with no party, bid-party or winner row anywhere.
pub const GUID_ROLE_NONE: &str = "no_role";

/// The published BT-501 formatting of a mention's `raw_identifier`, one bit each: the platform's
/// fingerprint the corpus keeps on every mention (the stored identifier is normalised to undashed
/// upper case, so it cannot tell `037c1508-d945-…` from `FB197F94…`).
const RAW_SHAPES: [&str; 8] =
    ["dashed-lower", "dashed-upper", "dashed-mixed", "plain-lower", "plain-upper", "plain-mixed", "other", "none"];

/// Issue 530 unit 1: the highest organization id when the walk starts; rows minted after it are not read.
/// `ORDER BY id DESC LIMIT 1` rather than `MAX(id)`: the house idiom for an O(1) primary-key read
/// (`ghost_census`), which cannot decay into a scan the way an aggregate can.
pub(crate) const GUID_CENSUS_MAX_ID_SQL: &str = "SELECT id FROM organizations ORDER BY id DESC LIMIT 1";

/// Issue 530 unit 1: one window of the walk — a rowid range, the identifier's length a filter on it (32
/// undashed, 36 dashed), so a window reads at most [`GUID_CENSUS_WINDOW`] rows.
pub(crate) const GUID_CENSUS_ORGS_SQL: &str = "SELECT id, country, identifier, name FROM organizations \
     WHERE id > ? AND id <= ? AND length(identifier) IN (32, 36)";

/// Issue 530 unit 1: one batch's mentions, `n` org ids at a time, through `organization_mentions_org`.
pub(crate) fn guid_census_mentions_sql(n: usize) -> String {
    format!(
        "SELECT organization_id, notice_id, name, raw_identifier FROM organization_mentions \
          WHERE organization_id IN ({})",
        placeholders(n)
    )
}

/// Issue 530 unit 1: the batch's notices by primary key — the source, profile and publication instant.
pub(crate) fn guid_census_notices_sql(n: usize) -> String {
    format!("SELECT id, source, profile, published_at FROM notices WHERE id IN ({})", placeholders(n))
}

/// Issue 530 unit 1: the Tenders the batch's notices are versions of, through `tender_versions_notice`.
pub(crate) fn guid_census_tenders_sql(n: usize) -> String {
    format!(
        "SELECT caused_by_notice_id, tender_id FROM tender_versions WHERE caused_by_notice_id IN ({})",
        placeholders(n)
    )
}

/// Issue 530 unit 1: which of `n` org ids are a bid party anywhere, through `tender_version_bid_parties_org`.
pub(crate) fn guid_census_bidders_sql(n: usize) -> String {
    format!(
        "SELECT DISTINCT organization_id FROM tender_version_bid_parties WHERE organization_id IN ({})",
        placeholders(n)
    )
}

/// Issue 530 unit 1: which of `n` org ids won a lot result anywhere, through
/// `tender_version_result_winners_org`.
pub(crate) fn guid_census_winners_sql(n: usize) -> String {
    format!(
        "SELECT DISTINCT organization_id FROM tender_version_result_winners WHERE organization_id IN ({})",
        placeholders(n)
    )
}

/// Issue 530 unit 1: the indexes every read of the census seeks. A full rebuild drops the tender ones and
/// Phase 1 the mention one; without them each chunk would scan its table, so the census refuses first.
pub const GUID_CENSUS_INDEXES: [&str; 5] = [
    "organization_mentions_org",
    "tender_versions_notice",
    "tender_version_parties_org_role",
    "tender_version_bid_parties_org",
    "tender_version_result_winners_org",
];

/// Issue 530 unit 1: one population's GUID rows and what their mentions span. Every row is in exactly
/// one of `no_mentions`, `no_tender`, `one_tender` and `multi_tender`; `multi_notice` is issue 312's
/// basis over the same rows. The `one_tender_*` counts are the single-use rows each signal reaches.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct GuidTally {
    pub rows: u64,
    pub mentions: u64,
    /// Rows with no mention left (orphans awaiting the sweep).
    pub no_mentions: u64,
    /// Rows whose mentions' notices are versions of no Tender.
    pub no_tender: u64,
    /// Single-use: the mentions span exactly one distinct Tender.
    pub one_tender: u64,
    pub multi_tender: u64,
    /// The mentions span more than one distinct notice (issue 312's basis).
    pub multi_notice: u64,
    /// Rows in the stable population.
    pub stable: u64,
    /// Single-use rows the within-notice twin signal reaches, pair-scoped: a row of the name the notice
    /// holds under two GUIDs.
    pub one_tender_twin: u64,
    /// Single-use rows the twin signal reaches notice-scoped: any GUID row a twin notice mentions.
    pub one_tender_twin_notice: u64,
    /// Single-use rows the cross-notice signal reaches.
    pub one_tender_cross: u64,
    /// Single-use rows either signal reaches (the twin pair-scoped); `one_tender` less this is what both
    /// rules miss.
    pub one_tender_either: u64,
    /// The same with the twin notice-scoped.
    pub one_tender_either_notice: u64,
}

impl GuidTally {
    /// Sum another tally into this one (a capped breakdown's rest).
    fn absorb(&mut self, other: &GuidTally) {
        self.rows += other.rows;
        self.mentions += other.mentions;
        self.no_mentions += other.no_mentions;
        self.no_tender += other.no_tender;
        self.one_tender += other.one_tender;
        self.multi_tender += other.multi_tender;
        self.multi_notice += other.multi_notice;
        self.stable += other.stable;
        self.one_tender_twin += other.one_tender_twin;
        self.one_tender_twin_notice += other.one_tender_twin_notice;
        self.one_tender_cross += other.one_tender_cross;
        self.one_tender_either += other.one_tender_either;
        self.one_tender_either_notice += other.one_tender_either_notice;
    }

    fn add(&mut self, row: &GuidRow) {
        self.rows += 1;
        self.mentions += row.mentions;
        match (row.mentions, row.tenders) {
            (0, _) => self.no_mentions += 1,
            (_, 0) => self.no_tender += 1,
            (_, 1) => {
                self.one_tender += 1;
                self.one_tender_twin += u64::from(row.twin);
                self.one_tender_twin_notice += u64::from(row.twin_notice);
                self.one_tender_cross += u64::from(row.cross_with.is_some());
                self.one_tender_either += u64::from(row.twin || row.cross_with.is_some());
                self.one_tender_either_notice += u64::from(row.twin_notice || row.cross_with.is_some());
            }
            _ => self.multi_tender += 1,
        }
        self.multi_notice += u64::from(row.notices > 1);
        self.stable += u64::from(row.stable);
    }
}

/// Issue 530 unit 1: the within-notice twin signal (unit 2 (a)), under both readings of the rule: the
/// pair-scoped `rows` (the GUID rows of the name a notice holds twice) and the notice-scoped
/// `notice_rows` (every GUID row a twin notice mentions, named or not). Every pair-scoped row is a
/// notice-scoped one.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct GuidTwinSignal {
    /// Notices holding at least one twin group.
    pub notices: u64,
    /// `(notice, name_norm)` groups whose mentions sit on two or more GUID rows.
    pub groups: u64,
    /// GUID rows with a mention in a twin group.
    pub rows: u64,
    pub one_tender_rows: u64,
    /// Stable rows a twin group reaches (a pair-scoped rule would refuse their key).
    pub stable_rows: u64,
    /// GUID rows with any mention in a twin notice.
    pub notice_rows: u64,
    pub notice_one_tender_rows: u64,
    /// Stable rows a twin notice mentions: the cost of the notice-scoped rule, which would refuse their
    /// key on that notice.
    pub notice_stable_rows: u64,
}

/// Issue 530 unit 1: the cross-notice signal (unit 2 (d)) and its collisions with stable rows (item 4).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct GuidCrossSignal {
    /// Rows whose `(name_norm, country)` holds an earlier GUID row with at most `few_mentions` mentions.
    pub rows: u64,
    pub one_tender_rows: u64,
    /// `(name_norm, country)` keys where the signal fires.
    pub names: u64,
    /// Of those, the keys that also hold a stable GUID row: a name on both kinds of platform.
    pub names_with_stable: u64,
    /// Rows the signal reaches under such a key.
    pub rows_in_stable_names: u64,
    pub one_tender_rows_in_stable_names: u64,
    /// Stable rows the signal reaches: a key that went on to link many mentions, which a rule on this
    /// signal would have refused at its mint.
    pub stable_rows: u64,
}

/// Issue 530 unit 1: one GUID row as the census read it — a sample or a canary.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct GuidRowSample {
    pub org_id: i64,
    pub country: Option<String>,
    /// The served name, `organizations.name`.
    pub name: String,
    pub identifier: String,
    pub mentions: u64,
    pub notices: u64,
    pub tenders: u64,
    /// The first [`GUID_CENSUS_SAMPLE_IDS`] distinct notice ids of its mentions.
    pub notice_ids: Vec<i64>,
    /// The first [`GUID_CENSUS_SAMPLE_IDS`] distinct Tender ids those notices are versions of.
    pub tender_ids: Vec<i64>,
    pub role: &'static str,
    pub role_mixed: bool,
    /// `<sources> <raw BT-501 shape>`, see [`GuidCensusReport::by_platform`].
    pub platform: String,
    pub profile: String,
    /// The UTC year of its earliest notice's publication.
    pub first_year: Option<i64>,
    pub stable: bool,
    /// The pair-scoped twin: a row of a name its notice holds under two or more GUID rows.
    pub twin: bool,
    /// The notice-scoped twin: a row with a mention in a notice holding a twin group.
    pub twin_notice: bool,
    /// The earliest GUID row the cross-notice signal found under its `(name_norm, country)`.
    pub cross_with: Option<i64>,
}

/// Issue 530 unit 1: one twin group — a notice whose mentions of two or more GUID rows share a name.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct GuidTwinSample {
    pub notice_id: i64,
    pub org_ids: Vec<i64>,
    /// The rows' served names.
    pub names: Vec<String>,
    pub identifiers: Vec<String>,
    /// The members' first [`GUID_CENSUS_SAMPLE_IDS`] Tenders each, merged: the notice's Tender whenever a
    /// member is single-use.
    pub tender_ids: Vec<i64>,
}

/// Issue 530 unit 1: the first `samples` of each list by org id (by notice id for the twin groups; the
/// largest by mentions for `stable`).
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct GuidSamples {
    pub one_tender: Vec<GuidRowSample>,
    pub twin_groups: Vec<GuidTwinSample>,
    pub cross: Vec<GuidRowSample>,
    /// Rows the cross signal reaches under a key that also holds a stable row.
    pub cross_stable_names: Vec<GuidRowSample>,
    /// Rows outside the stable population whose mentions span several Tenders.
    pub multi_tender: Vec<GuidRowSample>,
    pub stable: Vec<GuidRowSample>,
    pub no_tender: Vec<GuidRowSample>,
}

/// Issue 530 unit 1: [`Db::guid_census`]'s inputs.
pub struct GuidCensusArgs<'a> {
    /// The class: `ingest::idgate::uuid_v4` in the job.
    pub is_guid: fn(&str) -> bool,
    /// A party role's class, one of [`GUID_ROLE_CLASSES`] (anything else reads as `other`).
    pub role_class: fn(&str) -> &'static str,
    /// GUID rows in the stable population whatever their mentions (254, 255, 8378 in the job).
    pub stable_ids: &'a [i64],
    /// A GUID row with at least this many mentions is in the stable population too.
    pub stable_mentions: u64,
    /// The listing per sample list.
    pub samples: usize,
    /// Organization ids per window: [`GUID_CENSUS_WINDOW`] in the job.
    pub window: i64,
    /// GUID rows per batch, clamped to 1..=[`GUID_CENSUS_BATCH`]: [`GUID_CENSUS_BATCH`] in the job.
    pub batch: usize,
    /// GUID rows reported in `canaries` whatever their class; an id the walk does not find as a GUID
    /// row is listed under `canaries_absent`.
    pub canaries: &'a [i64],
    pub stop: &'a (dyn Fn() -> bool + Sync),
    pub progress: &'a (dyn Fn(u64, &str) + Sync),
}

/// Issue 530 unit 1: [`Db::guid_census`]'s answer. Read-only.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct GuidCensusReport {
    pub window: i64,
    pub stable_ids: Vec<i64>,
    pub stable_mentions: u64,
    /// [`GUID_CENSUS_FEW_MENTIONS`], the cross-notice signal's bar.
    pub few_mentions: u64,
    pub samples_per_list: usize,
    /// The highest organization id when the walk started; nothing above it was read.
    pub max_id: i64,
    pub windows: u64,
    /// Rows whose identifier is 32 or 36 characters long, the window statement's filter.
    pub candidates: u64,
    /// Candidates with 32 hex digits and the version nibble 4 that the class predicate still refuses (a
    /// variant nibble outside 8-b): the rows the issue's Verify regex counts and the class does not.
    pub nibble_only: u64,
    pub mentions_walked: u64,
    /// Distinct notices read by primary key, summed over the batches.
    pub notices_read: u64,
    /// Every GUID row (items 1 and 5).
    pub all: GuidTally,
    /// The stable population (item 5).
    pub stable: GuidTally,
    pub not_stable: GuidTally,
    /// By `organizations.country` (`none` for NULL).
    pub by_country: BTreeMap<String, GuidTally>,
    /// By the row's role class: its party roles through `role_class`, a bid party or a lot-result winner
    /// as `tenderer_winner`; several classes take the first of [`GUID_ROLE_CLASSES`].
    pub by_role: BTreeMap<&'static str, GuidTally>,
    /// Rows whose mentions hold more than one role class.
    pub role_mixed: u64,
    /// By platform: the row's notices' distinct `notices.source` values joined by `+`, a space, and the
    /// distinct BT-501 formattings of its mentions' `raw_identifier` (`dashed-lower`, `plain-upper`, …,
    /// `other` for a value that is not a bare GUID, `none` for NULL) joined by `+`. The largest
    /// [`GUID_CENSUS_KEYS_MAX`] by rows; the rest under [`GUID_CENSUS_REST`].
    pub by_platform: BTreeMap<String, GuidTally>,
    /// By the row's notices' distinct `notices.profile` values joined by `+`, capped the same way.
    pub by_profile: BTreeMap<String, GuidTally>,
    /// By the UTC year of the row's earliest notice (`none` without a dated notice): the era.
    pub by_first_year: BTreeMap<String, GuidTally>,
    pub twin: GuidTwinSignal,
    pub cross: GuidCrossSignal,
    pub samples: GuidSamples,
    pub canaries: Vec<GuidRowSample>,
    pub canaries_absent: Vec<i64>,
    /// More GUID mentions than [`GUID_CENSUS_MENTION_CEILING`]: the twin counts are a lower bound.
    pub mention_ceiling_hit: bool,
    /// What a platform key could also read and this census does not.
    pub not_measured: Vec<&'static str>,
    pub stopped: bool,
}

/// One GUID row, kept for the whole walk: the twin and cross signals compare rows of different batches.
struct GuidRow {
    id: i64,
    country: Option<String>,
    identifier: String,
    name: String,
    norm: String,
    mentions: u64,
    /// Distinct notices, and the first [`GUID_CENSUS_SAMPLE_IDS`] of them by id (a sample's).
    notices: u64,
    notice_ids: Vec<i64>,
    /// Distinct Tenders, and the first [`GUID_CENSUS_SAMPLE_IDS`] of them by id.
    tenders: u64,
    tender_ids: Vec<i64>,
    role: &'static str,
    role_mixed: bool,
    platform: u32,
    profile: u32,
    first_year: Option<i64>,
    stable: bool,
    twin: bool,
    twin_notice: bool,
    cross_with: Option<i64>,
}

/// One GUID mention as the twin signal keeps it: 24 bytes (`hash` is 0 and `named` false for a mention
/// with no name, which joins no group but is still reached by its notice's).
#[derive(Clone, Copy)]
struct MentionKey {
    notice: i64,
    /// The hash of `org_name_norm` of the mention's published name.
    hash: u64,
    /// The row's index in [`GuidWalk::rows`].
    row: u32,
    named: bool,
}

const _: () = assert!(std::mem::size_of::<MentionKey>() == 24);

/// A small string interner: the census keeps one `u32` per row instead of its key.
#[derive(Default)]
struct Interner {
    at: HashMap<String, u32>,
    keys: Vec<String>,
}

impl Interner {
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

/// The walk's state across batches.
#[derive(Default)]
struct GuidWalk {
    rows: Vec<GuidRow>,
    /// One per GUID mention, up to [`GUID_CENSUS_MENTION_CEILING`].
    mentions: Vec<MentionKey>,
    sources: Interner,
    profiles: Interner,
    platforms: Interner,
    profile_keys: Interner,
}

/// A GUID row found by the window walk, waiting for its batch.
struct Pending {
    id: i64,
    country: Option<String>,
    identifier: String,
    name: String,
}

/// The index of a raw BT-501 value's formatting in [`RAW_SHAPES`].
fn raw_shape(raw: Option<&str>) -> usize {
    let raw = raw.map(str::trim).unwrap_or_default();
    if raw.is_empty() {
        return 7;
    }
    let bytes = raw.as_bytes();
    let dash = |i: usize| matches!(i, 8 | 13 | 18 | 23);
    let dashed = bytes.len() == 36
        && bytes.iter().enumerate().all(|(i, b)| if dash(i) { *b == b'-' } else { b.is_ascii_hexdigit() });
    let plain = bytes.len() == 32 && bytes.iter().all(u8::is_ascii_hexdigit);
    if !dashed && !plain {
        return 6;
    }
    let lower = bytes.iter().any(u8::is_ascii_lowercase);
    let upper = bytes.iter().any(u8::is_ascii_uppercase);
    let case = match (lower, upper) {
        (true, true) => 2,
        (false, true) => 1,
        _ => 0,
    };
    if dashed { case } else { 3 + case }
}

/// 32 hex digits once the dashes are gone, with the version nibble 4 — the issue's Verify regex,
/// which reads no variant nibble.
fn hex32_v4_nibble(value: &str) -> bool {
    let hex: Vec<u8> = value.bytes().filter(|b| *b != b'-').collect();
    hex.len() == 32 && hex.iter().all(u8::is_ascii_hexdigit) && hex[12] == b'4'
}

fn name_hash(norm: &str) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    norm.hash(&mut h);
    h.finish()
}

/// Push `item` while `list` holds fewer than `cap`.
fn listed<T>(list: &mut Vec<T>, cap: usize, item: impl FnOnce() -> T) {
    if list.len() < cap {
        list.push(item());
    }
}

/// The largest `max` keys of `by` by rows (ties by key), the rest summed under [`GUID_CENSUS_REST`].
fn capped(by: HashMap<u32, GuidTally>, keys: &Interner, max: usize) -> BTreeMap<String, GuidTally> {
    let mut all: Vec<(u32, GuidTally)> = by.into_iter().collect();
    all.sort_by(|a, b| b.1.rows.cmp(&a.1.rows).then_with(|| keys.keys[a.0 as usize].cmp(&keys.keys[b.0 as usize])));
    let mut out = BTreeMap::new();
    for (i, (key, tally)) in all.into_iter().enumerate() {
        if i < max {
            out.insert(keys.keys[key as usize].clone(), tally);
        } else {
            out.entry(GUID_CENSUS_REST.to_owned()).or_default().absorb(&tally);
        }
    }
    out
}

impl Db {
    /// Issue 530 unit 1: the `guid-census` ([`GuidCensusReport`] names what it counts).
    ///
    /// This writes nothing. It walks `organizations` by rowid windows of `args.window` ids up to the
    /// highest id at its start, keeping the rows whose identifier passes `args.is_guid`, and reads them
    /// in batches of `args.batch` (at most [`GUID_CENSUS_BATCH`]): their mentions through
    /// `organization_mentions_org`, the distinct notices of those mentions by primary key, the Tenders
    /// those notices are versions of through `tender_versions_notice`, their party roles through
    /// `tender_version_parties_org_role` (as issue 511's dry plan reads them), and whether each is a bid
    /// party or a lot-result winner through those tables' `org` indexes — every statement an IN chunk of
    /// at most [`IN_CHUNK`] keys. The stop flag is read before every window, every batch and every IN
    /// chunk of a batch's mention, notice, Tender and role reads (the first batch always carries 254 and
    /// 255, about 97k mentions and some 190 notice chunks on prod); a stopped walk returns an empty report
    /// marked `stopped`. Each of [`GUID_CENSUS_INDEXES`] must stand, so the census refuses before reading.
    ///
    /// Memory is one record per GUID row, about 450 bytes with its strings (about 100 MB at 175k rows),
    /// and one 24-byte [`MentionKey`] per GUID mention (the twin signal pairs mentions of rows in
    /// different batches), capped at [`GUID_CENSUS_MENTION_CEILING`]. The rows are not capped on purpose:
    /// the walk goes in id order, so a row ceiling would drop the highest ids first — the per-notice
    /// mints this census measures — and bias every share rather than undercount it.
    pub async fn guid_census(&self, args: GuidCensusArgs<'_>) -> turso::Result<GuidCensusReport> {
        for index in GUID_CENSUS_INDEXES {
            if !self.has_index(index).await? {
                return Err(turso::Error::Error(format!(
                    "guid-census refused: {index} is absent (a rebuild?), so every chunk would scan its table; \
                     run `reindex` first. Nothing was read"
                )));
            }
        }
        let mut report = GuidCensusReport {
            window: args.window.max(1),
            stable_ids: args.stable_ids.to_vec(),
            stable_mentions: args.stable_mentions,
            few_mentions: GUID_CENSUS_FEW_MENTIONS,
            samples_per_list: args.samples,
            not_measured: vec!["esender_party", "submission_url_host"],
            ..Default::default()
        };
        let reader = self.reader().await?;
        let mut rows = reader.query(GUID_CENSUS_MAX_ID_SQL, ()).await?;
        report.max_id = match rows.next().await? {
            Some(row) => opt_int_of(&row, 0).unwrap_or(0),
            None => 0,
        };
        drop(rows);

        let width = args.batch.clamp(1, GUID_CENSUS_BATCH);
        let mut walk = GuidWalk::default();
        let mut pending: Vec<Pending> = Vec::new();
        let mut lo = 0i64;
        loop {
            // Full batches first, each behind the stop flag (and each of its chunks behind it too).
            while pending.len() >= width || (lo >= report.max_id && !pending.is_empty()) {
                if (args.stop)() {
                    return Ok(GuidCensusReport { stopped: true, ..Default::default() });
                }
                let batch: Vec<Pending> = pending.drain(..pending.len().min(width)).collect();
                if !self.guid_census_batch(&reader, &args, &mut walk, &mut report, batch).await? {
                    return Ok(GuidCensusReport { stopped: true, ..Default::default() });
                }
            }
            if lo >= report.max_id {
                break;
            }
            if (args.stop)() {
                return Ok(GuidCensusReport { stopped: true, ..Default::default() });
            }
            let hi = lo.saturating_add(report.window).min(report.max_id);
            let mut rows = reader.query(GUID_CENSUS_ORGS_SQL, (Value::Integer(lo), Value::Integer(hi))).await?;
            while let Some(row) = rows.next().await? {
                report.candidates += 1;
                let identifier = text(&row, 2);
                if (args.is_guid)(&identifier) {
                    pending.push(Pending { id: int(&row, 0), country: opt_text_of(&row, 1), identifier, name: text(&row, 3) });
                } else if hex32_v4_nibble(&identifier) {
                    report.nibble_only += 1;
                }
            }
            drop(rows);
            report.windows += 1;
            lo = hi;
            (args.progress)(
                lo.max(0) as u64,
                &format!(
                    "walking organizations: id {lo} of {}, {} GUID row(s) read, {} mention(s)",
                    report.max_id,
                    walk.rows.len(),
                    report.mentions_walked
                ),
            );
        }
        drop(reader);
        Ok(guid_census_finish(report, walk, &args))
    }

    /// One batch of GUID rows: their mentions, notices, Tenders and roles, folded into `walk`. `false`
    /// when the stop flag fired before one of its chunks: `walk` and `report` are then half-folded, and
    /// the caller discards both.
    async fn guid_census_batch(
        &self,
        reader: &crate::read::Reader,
        args: &GuidCensusArgs<'_>,
        walk: &mut GuidWalk,
        report: &mut GuidCensusReport,
        batch: Vec<Pending>,
    ) -> turso::Result<bool> {
        let base = walk.rows.len();
        let at: HashMap<i64, usize> = batch.iter().enumerate().map(|(i, p)| (p.id, i)).collect();
        let ids: Vec<i64> = batch.iter().map(|p| p.id).collect();
        // Per row: mentions, notice ids, the raw shapes' bits.
        let mut mentions = vec![0u64; batch.len()];
        let mut notices: Vec<Vec<i64>> = vec![Vec::new(); batch.len()];
        let mut shapes = vec![0u16; batch.len()];
        for chunk in ids.chunks(IN_CHUNK) {
            if (args.stop)() {
                return Ok(false);
            }
            let params: Vec<Value> = chunk.iter().map(|&id| Value::Integer(id)).collect();
            let mut rows = reader.query(&guid_census_mentions_sql(chunk.len()), params).await?;
            while let Some(row) = rows.next().await? {
                let Some(&i) = at.get(&int(&row, 0)) else { continue };
                let notice = int(&row, 1);
                report.mentions_walked += 1;
                mentions[i] += 1;
                notices[i].push(notice);
                shapes[i] |= 1 << raw_shape(opt_text_of(&row, 3).as_deref());
                let (hash, named) = match opt_text_of(&row, 2).map(|name| org_name_norm(&name)) {
                    Some(norm) if !norm.is_empty() => (name_hash(&norm), true),
                    _ => (0, false),
                };
                let kept = &mut walk.mentions;
                if kept.len() < GUID_CENSUS_MENTION_CEILING {
                    // Grow exactly to the ceiling rather than double past it.
                    if kept.len() == kept.capacity() && kept.capacity().saturating_mul(2) > GUID_CENSUS_MENTION_CEILING {
                        kept.reserve_exact(GUID_CENSUS_MENTION_CEILING - kept.len());
                    }
                    // The walk-wide index (`base + i`), so rows of different batches pair.
                    kept.push(MentionKey { notice, hash, row: (base + i) as u32, named });
                } else {
                    report.mention_ceiling_hit = true;
                }
            }
        }
        let mut distinct: BTreeSet<i64> = BTreeSet::new();
        for list in &mut notices {
            list.sort_unstable();
            list.dedup();
            distinct.extend(list.iter().copied());
        }
        let distinct: Vec<i64> = distinct.into_iter().collect();
        report.notices_read += distinct.len() as u64;

        // The notices by primary key: (source, profile, published_at).
        let mut notice_info: HashMap<i64, (u32, u32, Option<i64>)> = HashMap::with_capacity(distinct.len());
        for chunk in distinct.chunks(IN_CHUNK) {
            if (args.stop)() {
                return Ok(false);
            }
            let params: Vec<Value> = chunk.iter().map(|&id| Value::Integer(id)).collect();
            let mut rows = reader.query(&guid_census_notices_sql(chunk.len()), params).await?;
            while let Some(row) = rows.next().await? {
                let source = walk.sources.id(&text(&row, 1));
                let profile = walk.profiles.id(&text(&row, 2));
                notice_info.insert(int(&row, 0), (source, profile, opt_int_of(&row, 3)));
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
        // Role classes, one bit per [`GUID_ROLE_CLASSES`] index.
        let class_bit = |class: &str| 1u8 << GUID_ROLE_CLASSES.iter().position(|c| *c == class).unwrap_or(3);
        let mut roles = vec![0u8; batch.len()];
        for chunk in ids.chunks(IN_CHUNK) {
            if (args.stop)() {
                return Ok(false);
            }
            let params: Vec<Value> = chunk.iter().map(|&id| Value::Integer(id)).collect();
            let mut rows = reader.query(&party_role_counts_sql(chunk.len()), params.clone()).await?;
            while let Some(row) = rows.next().await? {
                if let Some(&i) = at.get(&int(&row, 0)) {
                    roles[i] |= class_bit((args.role_class)(&text(&row, 1)));
                }
            }
            drop(rows);
            for sql in [guid_census_bidders_sql(chunk.len()), guid_census_winners_sql(chunk.len())] {
                let mut rows = reader.query(&sql, params.clone()).await?;
                while let Some(row) = rows.next().await? {
                    if let Some(&i) = at.get(&int(&row, 0)) {
                        roles[i] |= class_bit(GUID_ROLE_TENDERER);
                    }
                }
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
                    Some(&(source, profile, published)) => {
                        sources.insert(&walk.sources.keys[source as usize]);
                        profiles.insert(&walk.profiles.keys[profile as usize]);
                        if let Some(when) = published {
                            first_published = Some(first_published.map_or(when, |f: i64| f.min(when)));
                        }
                    }
                    None => {
                        sources.insert("(no notice)");
                    }
                }
            }
            let shape: Vec<&str> =
                RAW_SHAPES.iter().enumerate().filter(|(bit, _)| shapes[i] & (1 << bit) != 0).map(|(_, s)| *s).collect();
            let platform_key = format!(
                "{} {}",
                if sources.is_empty() { "(no mention)".to_owned() } else { sources.into_iter().collect::<Vec<_>>().join("+") },
                if shape.is_empty() { "(no mention)".to_owned() } else { shape.join("+") }
            );
            let profile_key =
                if profiles.is_empty() { "(no notice)".to_owned() } else { profiles.into_iter().collect::<Vec<_>>().join("+") };
            let platform = walk.platforms.id(&platform_key);
            let profile = walk.profile_keys.id(&profile_key);
            let bits = roles[i];
            let role = match GUID_ROLE_CLASSES.iter().enumerate().find(|(bit, _)| bits & (1 << bit) != 0) {
                Some((_, class)) => *class,
                None => GUID_ROLE_NONE,
            };
            walk.rows.push(GuidRow {
                id: p.id,
                norm: org_name_norm(&p.name),
                country: p.country,
                identifier: p.identifier,
                name: p.name,
                mentions: mentions[i],
                notices: notices[i].len() as u64,
                notice_ids: notices[i].iter().take(GUID_CENSUS_SAMPLE_IDS).copied().collect(),
                tenders: tenders.len() as u64,
                tender_ids: tenders.into_iter().take(GUID_CENSUS_SAMPLE_IDS).collect(),
                role,
                role_mixed: bits.count_ones() > 1,
                platform,
                profile,
                first_year: first_published.map(unix_year),
                stable: false,
                twin: false,
                twin_notice: false,
                cross_with: None,
            });
        }
        Ok(true)
    }
}

/// The signals, tallies and samples over every GUID row the walk kept.
fn guid_census_finish(mut report: GuidCensusReport, mut walk: GuidWalk, args: &GuidCensusArgs<'_>) -> GuidCensusReport {
    let stable_ids: HashSet<i64> = args.stable_ids.iter().copied().collect();
    for row in &mut walk.rows {
        row.stable = stable_ids.contains(&row.id) || row.mentions >= args.stable_mentions;
    }
    let cap = args.samples;

    // (2) The twin signal: mentions of one notice, one name, two or more GUID rows. Per notice run: its
    // named groups mark their rows `twin` (pair-scoped); a notice holding one marks every GUID row it
    // mentions, named or not, `twin_notice` (notice-scoped).
    walk.mentions.sort_unstable_by_key(|m| (m.notice, m.named, m.hash, m.row));
    let mut start = 0;
    while start < walk.mentions.len() {
        let notice = walk.mentions[start].notice;
        let mut end = start;
        while end < walk.mentions.len() && walk.mentions[end].notice == notice {
            end += 1;
        }
        let run = &walk.mentions[start..end];
        let mut twin_notice = false;
        let mut g = 0;
        while g < run.len() {
            let mut h = g;
            while h < run.len() && run[h].named == run[g].named && run[h].hash == run[g].hash {
                h += 1;
            }
            // Sorted by row within the group, so `dedup` leaves its distinct rows.
            let mut group: Vec<u32> = run[g..h].iter().map(|m| m.row).collect();
            group.dedup();
            if run[g].named && group.len() > 1 {
                twin_notice = true;
                report.twin.groups += 1;
                for &row in &group {
                    walk.rows[row as usize].twin = true;
                }
                listed(&mut report.samples.twin_groups, cap, || {
                    let mut tender_ids: Vec<i64> =
                        group.iter().flat_map(|&r| walk.rows[r as usize].tender_ids.iter().copied()).collect();
                    tender_ids.sort_unstable();
                    tender_ids.dedup();
                    GuidTwinSample {
                        notice_id: notice,
                        org_ids: group.iter().map(|&r| walk.rows[r as usize].id).collect(),
                        names: group.iter().map(|&r| walk.rows[r as usize].name.clone()).collect(),
                        identifiers: group.iter().map(|&r| walk.rows[r as usize].identifier.clone()).collect(),
                        tender_ids,
                    }
                });
            }
            g = h;
        }
        if twin_notice {
            report.twin.notices += 1;
            for m in run {
                walk.rows[m.row as usize].twin_notice = true;
            }
        }
        start = end;
    }
    walk.mentions = Vec::new();

    // (3) The cross signal: per (name_norm, country), in id order, the first row with at most
    // GUID_CENSUS_FEW_MENTIONS mentions reaches every later row of the key.
    let mut order: Vec<usize> = (0..walk.rows.len()).filter(|&i| !walk.rows[i].norm.is_empty()).collect();
    order.sort_unstable_by(|&a, &b| {
        let (a, b) = (&walk.rows[a], &walk.rows[b]);
        (&a.norm, &a.country, a.id).cmp(&(&b.norm, &b.country, b.id))
    });
    let mut stable_names: HashSet<usize> = HashSet::new(); // the first row index of each key holding a stable row
    let mut key_of: Vec<usize> = vec![usize::MAX; walk.rows.len()];
    let mut start = 0;
    while start < order.len() {
        let first = &walk.rows[order[start]];
        let mut end = start;
        while end < order.len() && walk.rows[order[end]].norm == first.norm && walk.rows[order[end]].country == first.country {
            end += 1;
        }
        let key = order[start];
        let mut few: Option<i64> = None;
        let mut fires = false;
        for &i in &order[start..end] {
            key_of[i] = key;
            let row = &mut walk.rows[i];
            if let Some(earlier) = few {
                row.cross_with = Some(earlier);
                fires = true;
            } else if row.mentions <= GUID_CENSUS_FEW_MENTIONS {
                few = Some(row.id);
            }
        }
        let holds_stable = order[start..end].iter().any(|&i| walk.rows[i].stable);
        if fires {
            report.cross.names += 1;
            if holds_stable {
                report.cross.names_with_stable += 1;
                stable_names.insert(key);
            }
        }
        start = end;
    }

    // The tallies, the breakdowns and the samples, in id order.
    let mut by_platform: HashMap<u32, GuidTally> = HashMap::new();
    let mut by_profile: HashMap<u32, GuidTally> = HashMap::new();
    let canaries: HashSet<i64> = args.canaries.iter().copied().collect();
    let mut found: HashSet<i64> = HashSet::new();
    let sample = |row: &GuidRow| GuidRowSample {
        org_id: row.id,
        country: row.country.clone(),
        name: row.name.clone(),
        identifier: row.identifier.clone(),
        mentions: row.mentions,
        notices: row.notices,
        tenders: row.tenders,
        notice_ids: row.notice_ids.clone(),
        tender_ids: row.tender_ids.clone(),
        role: row.role,
        role_mixed: row.role_mixed,
        platform: walk.platforms.keys[row.platform as usize].clone(),
        profile: walk.profile_keys.keys[row.profile as usize].clone(),
        first_year: row.first_year,
        stable: row.stable,
        twin: row.twin,
        twin_notice: row.twin_notice,
        cross_with: row.cross_with,
    };
    for (i, row) in walk.rows.iter().enumerate() {
        let one_tender = row.mentions > 0 && row.tenders == 1;
        report.all.add(row);
        (if row.stable { &mut report.stable } else { &mut report.not_stable }).add(row);
        report.by_country.entry(row.country.clone().unwrap_or_else(|| "none".to_owned())).or_default().add(row);
        report.by_role.entry(row.role).or_default().add(row);
        report.role_mixed += u64::from(row.role_mixed);
        by_platform.entry(row.platform).or_default().add(row);
        by_profile.entry(row.profile).or_default().add(row);
        report
            .by_first_year
            .entry(row.first_year.map_or_else(|| "none".to_owned(), |y| y.to_string()))
            .or_default()
            .add(row);
        if row.twin {
            report.twin.rows += 1;
            report.twin.one_tender_rows += u64::from(one_tender);
            report.twin.stable_rows += u64::from(row.stable);
        }
        if row.twin_notice {
            report.twin.notice_rows += 1;
            report.twin.notice_one_tender_rows += u64::from(one_tender);
            report.twin.notice_stable_rows += u64::from(row.stable);
        }
        if row.cross_with.is_some() {
            report.cross.rows += 1;
            report.cross.one_tender_rows += u64::from(one_tender);
            report.cross.stable_rows += u64::from(row.stable);
            if stable_names.contains(&key_of[i]) {
                report.cross.rows_in_stable_names += 1;
                report.cross.one_tender_rows_in_stable_names += u64::from(one_tender);
                listed(&mut report.samples.cross_stable_names, cap, || sample(row));
            }
            listed(&mut report.samples.cross, cap, || sample(row));
        }
        if one_tender {
            listed(&mut report.samples.one_tender, cap, || sample(row));
        } else if row.mentions > 0 && row.tenders == 0 {
            listed(&mut report.samples.no_tender, cap, || sample(row));
        } else if row.tenders > 1 && !row.stable {
            listed(&mut report.samples.multi_tender, cap, || sample(row));
        }
        if canaries.contains(&row.id) {
            found.insert(row.id);
            report.canaries.push(sample(row));
        }
    }
    let mut stable: Vec<&GuidRow> = walk.rows.iter().filter(|r| r.stable).collect();
    stable.sort_by(|a, b| b.mentions.cmp(&a.mentions).then(a.id.cmp(&b.id)));
    report.samples.stable = stable.into_iter().take(cap).map(sample).collect();
    report.canaries_absent = args.canaries.iter().copied().filter(|id| !found.contains(id)).collect();
    report.by_platform = capped(by_platform, &walk.platforms, GUID_CENSUS_KEYS_MAX);
    report.by_profile = capped(by_profile, &walk.profile_keys, GUID_CENSUS_KEYS_MAX);
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_raw_bt501_value_is_shaped_by_its_dashes_and_case() {
        assert_eq!(RAW_SHAPES[raw_shape(Some("037c1508-d945-4e73-9d6e-65099a79bec1"))], "dashed-lower");
        assert_eq!(RAW_SHAPES[raw_shape(Some("037C1508-D945-4E73-9D6E-65099A79BEC1"))], "dashed-upper");
        assert_eq!(RAW_SHAPES[raw_shape(Some("FB197F94757846738A574642AE120532"))], "plain-upper");
        assert_eq!(RAW_SHAPES[raw_shape(Some("fb197f94757846738a574642ae120532"))], "plain-lower");
        assert_eq!(RAW_SHAPES[raw_shape(Some("Fb197f94757846738a574642ae120532"))], "plain-mixed");
        assert_eq!(RAW_SHAPES[raw_shape(Some("DE 037c1508"))], "other");
        assert_eq!(RAW_SHAPES[raw_shape(None)], "none");
        assert_eq!(RAW_SHAPES[raw_shape(Some("  "))], "none");
        assert!(hex32_v4_nibble("0A9EA48008E44AB6CF12D722D0AD54B6"), "the nibble alone, whatever the variant");
        assert!(hex32_v4_nibble("0a9ea480-08e4-4ab6-cf12-d722d0ad54b6"), "dashed, any case");
        assert!(!hex32_v4_nibble("0A9EA48008E41AB6BF12D722D0AD54B6"));
    }

    // ---- the fixture census ----

    /// The class as the job injects it (`ingest::idgate::uuid_v4`), mirrored: store never depends on
    /// ingest.
    fn uuid_v4(value: &str) -> bool {
        let hex: Vec<u8> = value.bytes().filter(|b| *b != b'-').collect();
        hex.len() == 32
            && hex.iter().all(u8::is_ascii_hexdigit)
            && hex[12] == b'4'
            && matches!(hex[16].to_ascii_lowercase(), b'8' | b'9' | b'a' | b'b')
    }

    /// The supervisor's role mapping, in miniature.
    fn role_class(role: &str) -> &'static str {
        match role {
            "Procedure-Buyer" | "buyer" => GUID_ROLE_BUYER,
            "Tenderer" | "winner" => GUID_ROLE_TENDERER,
            r if r.starts_with("Lot-") => GUID_ROLE_REVIEW_INFO,
            _ => GUID_ROLE_OTHER,
        }
    }

    async fn open(name: &str, tender_indexes: bool) -> (crate::Db, turso::Connection, String) {
        let path = format!("/tmp/tender-db-guid-census-{name}-{}.db", std::process::id());
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

    async fn org(conn: &turso::Connection, id: i64, country: &str, identifier: &str, name: &str) {
        conn.execute(
            "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
             VALUES (?, ?, 'national', ?, ?, ?, 0, 0)",
            (
                Value::Integer(id),
                Value::Text(country.into()),
                Value::Text(identifier.into()),
                Value::Text(name.into()),
                Value::Text(name.to_lowercase()),
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

    /// A mention of `org` in `notice`'s section `section`, as published (`raw`), in `role` on the notice's
    /// version (`tender`, `seq`).
    #[allow(clippy::too_many_arguments)]
    async fn mention(
        conn: &turso::Connection,
        notice: i64,
        section: &str,
        org: i64,
        name: &str,
        raw: &str,
        role: &str,
        tender: i64,
        seq: i64,
    ) {
        conn.execute(
            "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, country, raw_identifier, scheme)
             VALUES (?, ?, ?, ?, 'DEU', ?, NULL)",
            (Value::Integer(notice), Value::Text(section.into()), Value::Integer(org), Value::Text(name.into()), Value::Text(raw.into())),
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

    const GOETTINGEN: &str = "Landkreis Göttingen";
    const SUEDBAYERN: &str = "Regierung von Oberbayern - Vergabekammer Südbayern";
    const ROMMEL: &str = "Fenster- und Fassadenbau Rommel GmbH";
    const WEISS: &str = "LEONHARD WEISS GmbH & Co. KG";
    /// 2024-06-01 and 2026-09-22, UTC.
    const Y2024: i64 = 1_717_200_000;
    const Y2026: i64 = 1_790_035_200;

    /// The fixture, one shape of issue 530 each:
    /// - Göttingen (tender 948285): the award's TED copy 24262502 and DÖE copy 26483825 each carry the
    ///   buyer under two GUIDs — Procedure-Buyer → 22759261, Lot-AddInfo → 22759262 — and the winner
    ///   Rommel under a third (22759265). A VAT row of the same name (22183653) is no GUID row.
    /// - Südbayern: three DÖE notices of three tenders, each minting its own review-chamber GUID row
    ///   (30971684, 30971734, 30971757), one mention each: no twin.
    /// - LEONHARD WEISS: the stable bidder 8378 recurring under one GUID across three notices of three
    ///   tenders, and an earlier single-use row 8000 of the same name under another GUID.
    /// - Two rows of two role classes, so the precedence and `role_mixed` are pinned: 8378 is also a
    ///   `Lot-ReviewOrg` party (tenderer_winner + review_or_info → tenderer_winner, where the reverse order
    ///   would read review_or_info), and Südbayern's 30971684 is also a bid party (the bid-party path:
    ///   review_or_info + tenderer_winner → tenderer_winner). Neither adds a mention, notice or Tender.
    /// - 30000001: a 32-hex value with the version nibble but a variant `C` (the Verify regex counts it,
    ///   the class does not); 30000002: 32 hex, no version nibble.
    async fn seed(conn: &turso::Connection) {
        // Göttingen.
        org(conn, 22_183_653, "DE", "DE308703297", GOETTINGEN).await;
        org(conn, 22_759_261, "DE", "037C1508D9454E739D6E65099A79BEC1", GOETTINGEN).await;
        org(conn, 22_759_262, "DE", "5B1C2D3E4F5A4B6C8D7E8F9A0B1C2D3E", GOETTINGEN).await;
        org(conn, 22_759_265, "DE", "6C2D3E4F5A6B4C7D9E8F9A0B1C2D3E4F", ROMMEL).await;
        for (notice_id, source, profile, seq) in
            [(24_262_502, "ted", "eforms:eforms-sdk-1.10", 1), (26_483_825, "doe", "eforms:eforms-de-1.2", 2)]
        {
            notice(conn, notice_id, source, profile, Y2024, 948_285, seq).await;
            for (section, org_id, name, raw, role) in [
                ("ORG-0001", 22_759_261, GOETTINGEN, "037c1508-d945-4e73-9d6e-65099a79bec1", "Procedure-Buyer"),
                // A trailing blank: the same `org_name_norm`, so still the twin.
                ("ORG-0002", 22_759_262, "Landkreis Göttingen ", "5b1c2d3e-4f5a-4b6c-8d7e-8f9a0b1c2d3e", "Lot-AddInfo"),
                ("ORG-0003", 22_759_265, ROMMEL, "6c2d3e4f-5a6b-4c7d-9e8f-9a0b1c2d3e4f", "Tenderer"),
            ] {
                mention(conn, notice_id, section, org_id, name, raw, role, 948_285, seq).await;
            }
        }
        // The VAT row's own contract-notice mention: no GUID, never read.
        notice(conn, 23_941_063, "ted", "eforms:eforms-sdk-1.10", Y2024 - 86_400 * 60, 948_285, 0).await;
        mention(conn, 23_941_063, "ORG-0001", 22_183_653, GOETTINGEN, "DE 308 703 297", "Procedure-Buyer", 948_285, 0).await;

        // Südbayern: one GUID per notice, no twin.
        for (k, (org_id, identifier)) in [
            (30_971_684, "1A2B3C4D5E6F40718293A4B5C6D7E8F9"),
            (30_971_734, "2B3C4D5E6F7A41829304B5C6D7E8F9A0"),
            (30_971_757, "3C4D5E6F7A8B4293A415C6D7E8F9A0B1"),
        ]
        .into_iter()
        .enumerate()
        {
            let notice_id = 46_692_092 + k as i64 * 100;
            let tender = 990_000 + k as i64;
            org(conn, org_id, "DE", identifier, SUEDBAYERN).await;
            notice(conn, notice_id, "doe", "eforms:eforms-de-1.2", Y2026, tender, 1).await;
            mention(conn, notice_id, "ORG-0004", org_id, SUEDBAYERN, &identifier.to_lowercase(), "Lot-ReviewOrg", tender, 1)
                .await;
        }

        // LEONHARD WEISS: the earlier single-use row, then the stable platform key across three tenders.
        org(conn, 8_000, "DE", "4D5E6F7A8B9C43A4B526D7E8F9A0B1C2", WEISS).await;
        notice(conn, 40_000_001, "doe", "eforms:eforms-de-1.2", Y2026, 995_000, 1).await;
        mention(conn, 40_000_001, "ORG-0002", 8_000, WEISS, "4d5e6f7a-8b9c-43a4-b526-d7e8f9a0b1c2", "Tenderer", 995_000, 1).await;
        org(conn, 8_378, "DE", "9A9357BC0D1E4F2A8B3C4D5E6F7A8B9C", WEISS).await;
        for (k, source) in ["ted", "ted", "ted"].into_iter().enumerate() {
            let notice_id = 45_000_001 + k as i64;
            let tender = 996_000 + k as i64;
            notice(conn, notice_id, source, "eforms:eforms-sdk-1.10", Y2026, tender, 1).await;
            mention(conn, notice_id, "ORG-0003", 8_378, WEISS, "9A9357BC0D1E4F2A8B3C4D5E6F7A8B9C", "Tenderer", tender, 1).await;
        }

        // The second role classes: party and bid-party rows only, on mentions that already stand.
        conn.execute(
            "INSERT INTO tender_version_parties (tender_id, seq, lot_id, role, organization_id, mention_notice_id, mention_section_id)
             VALUES (996000, 1, NULL, 'Lot-ReviewOrg', 8378, 45000001, 'ORG-0003')",
            (),
        )
        .await
        .unwrap();
        conn.execute(
            "INSERT INTO tender_version_bid_parties (tender_id, seq, bid_id, role, organization_id, mention_notice_id,
                                                     mention_section_id)
             VALUES (990000, 1, 1, 'tenderer', 30971684, 46692092, 'ORG-0004')",
            (),
        )
        .await
        .unwrap();

        // Near misses of the class.
        org(conn, 30_000_001, "DE", "0A9EA48008E44AB6CF12D722D0AD54B6", "Variant C GmbH").await;
        org(conn, 30_000_002, "DE", "0A9EA48008E41AB6BF12D722D0AD54B6", "No Version GmbH").await;
    }

    fn args<'a>(stop: &'a (dyn Fn() -> bool + Sync), progress: &'a (dyn Fn(u64, &str) + Sync)) -> GuidCensusArgs<'a> {
        GuidCensusArgs {
            is_guid: uuid_v4,
            role_class,
            stable_ids: &[254, 255, 8_378],
            stable_mentions: 100,
            samples: 30,
            // Narrow enough that the walk crosses several windows before its one batch.
            window: 5_000_000,
            batch: GUID_CENSUS_BATCH,
            canaries: &[8_378, 22_759_261, 22_759_265, 30_971_684, 254],
            stop,
            progress,
        }
    }

    /// Issue 530 unit 1: every count of the census on one fixture of each shape.
    #[tokio::test]
    async fn the_guid_census_counts_tenders_twins_and_cross_notice_names() {
        let (db, conn, path) = open("counts", true).await;
        seed(&conn).await;
        let stop = || false;
        let progress = |_: u64, _: &str| {};
        let r = db.guid_census(args(&stop, &progress)).await.unwrap();
        assert!(!r.stopped && !r.mention_ceiling_hit);
        assert_eq!(r.max_id, 30_971_757);
        // 8 GUID rows; the VAT row is no candidate, the two near misses are candidates outside the class.
        assert_eq!(r.candidates, 10);
        assert_eq!(r.nibble_only, 1, "the variant-C value: the Verify regex counts it, the class does not");
        assert_eq!(r.all.rows, 8);
        assert_eq!(r.windows, 7);
        assert_eq!(r.mentions_walked, 6 + 3 + 1 + 3, "Göttingen's two copies of three, Südbayern, WEISS");
        assert_eq!(r.all.mentions, 13);

        // (1) Single-use: Göttingen's three rows (two notices, ONE tender), Südbayern's three, WEISS 8000.
        assert_eq!(r.all.one_tender, 7);
        assert_eq!(r.all.multi_tender, 1, "8378 recurs across three tenders");
        assert_eq!((r.all.no_tender, r.all.no_mentions), (0, 0));
        // 312's basis counts the TED+DÖE copies as two notices: Göttingen's three rows and 8378.
        assert_eq!(r.all.multi_notice, 4);
        assert_eq!(r.by_country["DE"].rows, 8);
        assert_eq!(r.by_role[GUID_ROLE_BUYER].rows, 1);
        // The precedence is buyer, tenderer_winner, review_or_info, other: 8378 (Tenderer + Lot-ReviewOrg)
        // and 30971684 (Lot-ReviewOrg + a bid party) are tenderer_winner, and they are the mixed rows.
        assert_eq!(r.by_role[GUID_ROLE_REVIEW_INFO].rows, 3, "22759262's Lot-AddInfo, Südbayern's later two");
        assert_eq!(r.by_role[GUID_ROLE_TENDERER].rows, 4, "Rommel, 8000, 8378, 30971684");
        assert_eq!(r.by_role[GUID_ROLE_TENDERER].one_tender, 3, "Rommel, 8000, 30971684");
        assert_eq!(r.role_mixed, 2);
        assert_eq!(r.by_platform["doe+ted dashed-lower"].rows, 3, "Göttingen's award, both copies");
        assert_eq!(r.by_platform["doe plain-lower"].rows, 3, "Südbayern");
        assert_eq!(r.by_platform["doe dashed-lower"].rows, 1, "8000");
        assert_eq!(r.by_platform["ted plain-upper"].rows, 1, "8378");
        assert_eq!(r.by_profile["eforms:eforms-de-1.2+eforms:eforms-sdk-1.10"].rows, 3);
        assert_eq!(r.by_first_year["2024"].rows, 3);
        assert_eq!(r.by_first_year["2026"].rows, 5);

        // (2) The twin: both copies of the award name the buyer under two GUIDs ("Landkreis Göttingen "
        // folds to the same name_norm); Rommel and Südbayern have none.
        assert_eq!((r.twin.notices, r.twin.groups), (2, 2));
        assert_eq!((r.twin.rows, r.twin.one_tender_rows, r.twin.stable_rows), (2, 2, 0));
        assert_eq!(r.samples.twin_groups.len(), 2);
        assert_eq!(r.samples.twin_groups[0].notice_id, 24_262_502);
        assert_eq!(r.samples.twin_groups[0].org_ids, vec![22_759_261, 22_759_262]);
        assert_eq!(r.samples.twin_groups[0].tender_ids, vec![948_285]);
        assert_eq!(r.all.one_tender_twin, 2);
        // Notice-scoped, the twin notices also reach the winner Rommel, whose name each names once.
        assert_eq!((r.twin.notice_rows, r.twin.notice_one_tender_rows, r.twin.notice_stable_rows), (3, 3, 0));
        assert_eq!(r.all.one_tender_twin_notice, 3);
        assert_eq!(r.by_role[GUID_ROLE_TENDERER].one_tender_twin, 0, "a winner is named once per notice");
        assert_eq!(r.by_role[GUID_ROLE_TENDERER].one_tender_twin_notice, 1, "Rommel");

        // (3) The cross signal: 22759262 sits under 22759261 (2 mentions), Südbayern's second and third
        // under its first, and 8378 under 8000 (1 mention) — 4 rows under 3 names.
        assert_eq!(r.cross.rows, 4);
        assert_eq!(r.cross.one_tender_rows, 3);
        assert_eq!(r.cross.names, 3);
        let reached: Vec<(i64, Option<i64>)> = r.samples.cross.iter().map(|s| (s.org_id, s.cross_with)).collect();
        assert_eq!(
            reached,
            vec![(8_378, Some(8_000)), (22_759_262, Some(22_759_261)), (30_971_734, Some(30_971_684)), (30_971_757, Some(30_971_684))]
        );
        assert_eq!(r.all.one_tender_cross, 3);
        // Either signal: 22759261, 22759262, 30971734, 30971757; neither: Rommel, 30971684, 8000. With the
        // twin notice-scoped Rommel joins them.
        assert_eq!(r.all.one_tender_either, 4);
        assert_eq!(r.all.one_tender_either_notice, 5);

        // (4) The cross signal fires on the stable bidder's name and reaches the stable row itself.
        assert_eq!((r.cross.names_with_stable, r.cross.rows_in_stable_names), (1, 1));
        assert_eq!((r.cross.stable_rows, r.cross.one_tender_rows_in_stable_names), (1, 0));
        assert_eq!(r.samples.cross_stable_names[0].org_id, 8_378);

        // (5) 8378 is stable by id though it holds 3 mentions; it spans 3 notices and 3 tenders.
        assert_eq!((r.stable.rows, r.stable.multi_tender, r.stable.multi_notice), (1, 1, 1));
        assert_eq!((r.not_stable.rows, r.not_stable.multi_notice, r.not_stable.multi_tender), (7, 3, 0));
        assert_eq!(r.samples.stable[0].org_id, 8_378);
        assert!(r.samples.multi_tender.is_empty());

        // The samples and canaries.
        assert_eq!(r.samples.one_tender.len(), 7);
        let buyer = r.canaries.iter().find(|c| c.org_id == 22_759_261).expect("the Göttingen buyer");
        assert_eq!((buyer.mentions, buyer.notices, buyer.tenders), (2, 2, 1));
        assert_eq!(buyer.notice_ids, vec![24_262_502, 26_483_825]);
        assert_eq!(buyer.tender_ids, vec![948_285]);
        assert_eq!(buyer.role, GUID_ROLE_BUYER);
        assert!(buyer.twin && buyer.twin_notice && buyer.cross_with.is_none());
        assert!(!buyer.role_mixed);
        let rommel = r.canaries.iter().find(|c| c.org_id == 22_759_265).expect("the Göttingen winner");
        assert!(!rommel.twin && rommel.twin_notice, "reached by its notice, not by a pair");
        assert_eq!((rommel.role, rommel.role_mixed), (GUID_ROLE_TENDERER, false));
        let weiss = r.canaries.iter().find(|c| c.org_id == 8_378).expect("the stable bidder");
        assert_eq!((weiss.role, weiss.role_mixed), (GUID_ROLE_TENDERER, true));
        let chamber = r.canaries.iter().find(|c| c.org_id == 30_971_684).expect("Südbayern's first row");
        assert_eq!((chamber.role, chamber.role_mixed), (GUID_ROLE_TENDERER, true));
        assert_eq!(r.canaries.len(), 4);
        assert_eq!(r.canaries_absent, vec![254]);
        assert_eq!(r.notices_read, 9, "one batch reads each distinct notice once");

        // One row per batch: 22759261 and 22759262 land in different batches, so their twin pairs only
        // through the walk-wide row index. Every count but `notices_read` (summed per batch: Göttingen's
        // three rows each re-read its two notices) is the same.
        let one = db.guid_census(GuidCensusArgs { batch: 1, ..args(&stop, &progress) }).await.unwrap();
        assert_eq!(one.notices_read, 13);
        assert_eq!(one.twin.rows, 2, "the twin pairs across batches");
        let (mut a, mut b) = (r.clone(), one.clone());
        (a.notices_read, b.notices_read) = (0, 0);
        assert_eq!(a, b, "a batch of one reads what a batch of 512 does");
        drop(conn);
        cleanup(&path);
    }

    /// Issue 530 unit 1: item (4) in the prod shape — a stable key minted FIRST (254, 255 and 8378 sit at
    /// the bottom of the id space), then single-use rows of its name. The stable row has too many
    /// mentions to be `few`, so the first later row is, and only the second is reached: the name counts
    /// in `names_with_stable` while the stable row stays unreached. 255 is also named in one of
    /// Göttingen's twin notices: the notice-scoped twin reaches it (its cost), the pair-scoped one not.
    #[tokio::test]
    async fn a_stable_key_minted_first_is_not_reached_by_the_cross_signal() {
        const BUND: &str = "Vergabekammer des Bundes";
        let (db, conn, path) = open("stable-first", true).await;
        seed(&conn).await;
        org(&conn, 255, "DE", "0A9EA48008E44AB6BF12D722D0AD54B6", BUND).await;
        // 255: three notices of three tenders, the first Göttingen's TED copy (tender 948285, seq 1).
        mention(&conn, 24_262_502, "ORG-0005", 255, BUND, "0A9EA48008E44AB6BF12D722D0AD54B6", "Lot-ReviewOrg", 948_285, 1)
            .await;
        for (k, notice_id) in [47_000_001i64, 47_000_002].into_iter().enumerate() {
            let tender = 997_001 + k as i64;
            notice(&conn, notice_id, "ted", "eforms:eforms-sdk-1.10", Y2026, tender, 1).await;
            mention(&conn, notice_id, "ORG-0005", 255, BUND, "0A9EA48008E44AB6BF12D722D0AD54B6", "Lot-ReviewOrg", tender, 1)
                .await;
        }
        // Two later single-use rows of the same name, one notice and tender each.
        for (k, (org_id, identifier)) in
            [(31_000_001i64, "7E8F9A0B1C2D4E3F8A4B5C6D7E8F9A0B"), (31_000_002, "8F9A0B1C2D3E4F5A9B5C6D7E8F9A0B1C")]
                .into_iter()
                .enumerate()
        {
            let notice_id = 47_100_001 + k as i64;
            let tender = 997_101 + k as i64;
            org(&conn, org_id, "DE", identifier, BUND).await;
            notice(&conn, notice_id, "doe", "eforms:eforms-de-1.2", Y2026, tender, 1).await;
            mention(&conn, notice_id, "ORG-0004", org_id, BUND, &identifier.to_lowercase(), "Lot-ReviewOrg", tender, 1).await;
        }
        let stop = || false;
        let progress = |_: u64, _: &str| {};
        let r = db
            .guid_census(GuidCensusArgs { canaries: &[255, 31_000_001, 31_000_002], ..args(&stop, &progress) })
            .await
            .unwrap();
        assert_eq!(r.cross.names_with_stable, 2, "WEISS and the Bund chamber");
        assert_eq!(r.cross.rows_in_stable_names, 2, "8378 and 31000002 — not 31000001, the `few` row");
        assert_eq!(r.cross.one_tender_rows_in_stable_names, 1, "31000002");
        assert_eq!(r.cross.stable_rows, 1, "8378 only: 255 came first");
        let ids: Vec<i64> = r.samples.cross_stable_names.iter().map(|s| s.org_id).collect();
        assert_eq!(ids, vec![8_378, 31_000_002]);
        let canary = |id: i64| r.canaries.iter().find(|c| c.org_id == id).cloned().expect("canary");
        let bund = canary(255);
        assert!(bund.stable && bund.cross_with.is_none());
        assert_eq!((bund.mentions, bund.notices, bund.tenders), (3, 3, 3));
        assert_eq!(canary(31_000_001).cross_with, None, "the first later row is the `few` one");
        assert_eq!(canary(31_000_002).cross_with, Some(31_000_001));
        // 255 in a twin notice: the notice-scoped rule would refuse its key there, the pair-scoped not.
        assert!(!bund.twin && bund.twin_notice);
        assert_eq!((r.twin.rows, r.twin.stable_rows), (2, 0));
        assert_eq!((r.twin.notice_rows, r.twin.notice_one_tender_rows, r.twin.notice_stable_rows), (4, 3, 1));
        drop(conn);
        cleanup(&path);
    }

    /// Issue 530 unit 1: a stable row by mention count, without the id list.
    #[tokio::test]
    async fn the_mention_threshold_makes_a_row_stable_and_the_id_list_is_not_needed() {
        let (db, conn, path) = open("threshold", true).await;
        seed(&conn).await;
        let stop = || false;
        let progress = |_: u64, _: &str| {};
        let r = db
            .guid_census(GuidCensusArgs { stable_ids: &[], stable_mentions: 3, ..args(&stop, &progress) })
            .await
            .unwrap();
        assert_eq!(r.stable.rows, 1, "8378's three mentions meet the threshold");
        assert_eq!(r.cross.stable_rows, 1);
        drop(conn);
        cleanup(&path);
    }

    /// Issue 530 unit 1: without its indexes the census refuses before reading; a stop between windows,
    /// before a batch, or before any IN chunk inside one returns an empty report marked stopped.
    #[tokio::test]
    async fn the_guid_census_refuses_without_its_indexes_and_stops_between_windows() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let (db, conn, path) = open("refuse", false).await;
        seed(&conn).await;
        let stop = || false;
        let progress = |_: u64, _: &str| {};
        let err = db.guid_census(args(&stop, &progress)).await.expect_err("tender_version_parties_org_role is deferred");
        assert!(err.to_string().contains("tender_version_parties_org_role is absent"), "{err}");
        db.build_tender_indexes().await.unwrap();
        let calls = AtomicUsize::new(0);
        let stop_second = || calls.fetch_add(1, Ordering::Relaxed) >= 1;
        let r = db.guid_census(args(&stop_second, &progress)).await.unwrap();
        assert!(r.stopped);
        assert_eq!(r.all.rows, 0, "a stopped census reports nothing");

        // One window and one batch read the flag six times: the window (1), the batch (2), then its
        // mentions (3), notices (4), Tenders (5) and roles (6) chunks. Firing on any of them stops the walk
        // there; firing only on a 7th call is never reached and the census completes.
        for at in 1..=7usize {
            let calls = AtomicUsize::new(0);
            let stop = || calls.fetch_add(1, Ordering::Relaxed) + 1 >= at;
            let r = db.guid_census(GuidCensusArgs { window: 100_000_000, ..args(&stop, &progress) }).await.unwrap();
            assert_eq!(r.stopped, at <= 6, "a stop on call {at}");
            assert_eq!(calls.load(Ordering::Relaxed), at.min(6), "the flag is read up to the firing call");
            assert_eq!(r.all.rows, if at <= 6 { 0 } else { 8 }, "a stop on call {at}");
        }
        drop(conn);
        cleanup(&path);
    }

    /// Issue 530 unit 1: every read the census sends on prod, planned at the width it sends, seeks: the
    /// window by rowid, the mentions through `organization_mentions_org`, the notices by primary key, the
    /// Tenders through `tender_versions_notice`, the roles through `tender_version_parties_org_role`, the
    /// bid parties and winners through their `org` indexes. A `SCAN` is a whole-table read per chunk.
    #[tokio::test]
    async fn the_guid_census_reads_by_index_at_the_chunk_width() {
        let (db, conn, path) = open("eqp", true).await;
        drop(conn);
        let reader = db.reader().await.unwrap();
        let ids: Vec<Value> = (1..=IN_CHUNK as i64).map(Value::Integer).collect();
        for (what, sql, params, needle) in [
            (
                "window",
                GUID_CENSUS_ORGS_SQL.to_owned(),
                vec![Value::Integer(0), Value::Integer(GUID_CENSUS_WINDOW)],
                "SEARCH organizations",
            ),
            ("mentions", guid_census_mentions_sql(IN_CHUNK), ids.clone(), "organization_mentions_org"),
            ("notices", guid_census_notices_sql(IN_CHUNK), ids.clone(), "SEARCH notices"),
            ("tenders", guid_census_tenders_sql(IN_CHUNK), ids.clone(), "tender_versions_notice"),
            ("roles", party_role_counts_sql(IN_CHUNK), ids.clone(), "tender_version_parties_org_role"),
            ("bidders", guid_census_bidders_sql(IN_CHUNK), ids.clone(), "tender_version_bid_parties_org"),
            ("winners", guid_census_winners_sql(IN_CHUNK), ids.clone(), "tender_version_result_winners_org"),
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
        }
        // The window's plan reads `rowid=?` whatever its bounds, so the bytecode is what shows the walk
        // seeks to the window's lower id and stops at its upper one, instead of reading every later row
        // from each window's start (a quadratic walk the plan text would not show).
        let mut rows = reader
            .query(&format!("EXPLAIN {GUID_CENSUS_ORGS_SQL}"), vec![Value::Integer(0), Value::Integer(GUID_CENSUS_WINDOW)])
            .await
            .unwrap();
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
