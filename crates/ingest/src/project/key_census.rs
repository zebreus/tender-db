//! Issue 482 unit 1: the procedure-key census.
//!
//! The fold groups notices by procedure key, and for a UUID key (BT-04) nothing else is
//! checked: issue 369's gate reads shaped keys only. A publisher reusing one UUID welds
//! unrelated procedures into one Tender (exhibit: Tender 1110706, a German DÖE/TED pair
//! and a Bulgarian award 15 months later under one BT-04). This census measures how
//! often that happens before a guard is chosen.
//!
//! It walks every UUID-keyed Tender with at least two notices, by Tender id in bounded
//! windows, and clusters each Tender's notices by buyer overlap. The overlap test is the
//! link guard's own (issue 481 units 2b/2c): each notice's tolerant token set
//! ([`GuardSide`]: resolved organization, raw identifier, signatory, agency principal,
//! whole-word prefixes and heads, acronyms), compared with
//! [`store::buyer_tokens_disjoint`]. Clustering is transitive: a notice joins a cluster
//! when it overlaps any member. A notice that names no buyer has an empty set, which is
//! unknown and never decisive, so it joins no cluster and splits nothing; it is counted
//! apart.
//!
//! Read-only: the job stores its report and writes nothing else.

use super::{GuardSide, add_org_tokens, is_de1_profile, is_sdk01_profile, is_uuid, normalise_de1, procedure_key};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Tender ids per window. The UUID-keyed Tenders sit densely at the bottom of the id
/// space (the 2026-08-20 full re-projection minted them first: 1,950 of every 2,000 ids
/// up to ~1.16M, read 2026-10-02), where a window holds ~3,200 Tenders with two or more
/// notices and ~10,000 notices to parse. Above that a window is a cheap empty range read.
pub const KEY_CENSUS_WINDOW: i64 = 5_000;

/// Samples kept per bucket, and Tenders kept in the hub list.
pub const KEY_CENSUS_SAMPLES: usize = 30;

/// Notices per parse batch ([`store::Db::parsed_by_ids`] reads every satellite of the
/// batch), as the link census's endpoint read.
const PARSE_BATCH: usize = 500;

/// Publications listed per cluster in a sample; the cluster's `notices` says how many
/// there are.
const SAMPLE_PUBLICATIONS: usize = 40;

/// Clusters listed per sample (the largest first; `clusters_total` says how many there
/// are), and buyerless notices listed per sample (`without_buyers_total`): a Tender of
/// tens of thousands of versions stays a bounded report row.
const SAMPLE_CLUSTERS: usize = 20;
const SAMPLE_WITHOUT_BUYERS: usize = 40;

/// The buckets, in report order: the clusters' jurisdictions, whether two clusters come
/// from disjoint Sources, the time gap between the minority clusters and the largest one
/// (the split rule's "more than N months away"), and the Tender's whole span.
pub const KEY_CENSUS_BUCKETS: [&str; 11] = [
    "one-jurisdiction",
    "several-jurisdictions",
    "unknown-jurisdiction",
    "one-source",
    "several-sources",
    "gap-le-90d",
    "gap-le-1y",
    "gap-gt-1y",
    "span-le-90d",
    "span-le-1y",
    "span-gt-1y",
];

/// One buyer cluster of a sampled Tender.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct KeyCensusCluster {
    /// How many notices the cluster holds.
    pub notices: usize,
    /// Its notices' `source:publication_id`, in publication order (at most
    /// [`SAMPLE_PUBLICATIONS`]).
    pub publications: Vec<String>,
    /// The first buyer name its earliest notice publishes.
    pub buyer: String,
    /// The register jurisdictions its notices' buyers name.
    pub jurisdictions: Vec<String>,
    /// The Sources of its notices.
    pub sources: Vec<String>,
    /// Its earliest and latest notice's publication day (`YYYY-MM-DD`).
    pub first_published: String,
    pub last_published: String,
    /// Days between its time range and the largest cluster's (`0` when they overlap, and
    /// for the largest cluster itself).
    pub gap_days: i64,
    /// Of its notices, the ones whose own procedure key is not the Tender's: joined by a
    /// link (issue 481), not by the shared BT-04.
    pub other_keys: usize,
}

/// One sampled Tender with two or more buyer-disjoint clusters.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct KeyCensusSample {
    pub tender_id: i64,
    pub procedure_key: String,
    pub notices: usize,
    /// First to last notice over all of them, buyerless ones included.
    pub span_days: i64,
    /// The smallest [`KeyCensusCluster::gap_days`] of the minority clusters.
    pub gap_days: i64,
    /// How many clusters there are; `clusters` lists at most [`SAMPLE_CLUSTERS`].
    pub clusters_total: usize,
    /// Largest cluster first.
    pub clusters: Vec<KeyCensusCluster>,
    /// The notices that name no buyer (`source:publication_id`), never in a cluster: at
    /// most [`SAMPLE_WITHOUT_BUYERS`] of `without_buyers_total`.
    pub without_buyers_total: usize,
    pub without_buyers: Vec<String>,
}

/// One bucket: how many split Tenders fell in it, and a uniform sample of them.
#[derive(Clone, Debug, Default, Serialize)]
pub struct KeyCensusBucket {
    pub tenders: u64,
    pub samples: Vec<KeyCensusSample>,
    /// The sample's ranks ([`sample_rank`]), parallel to `samples`.
    #[serde(skip)]
    ranks: Vec<u64>,
}

impl KeyCensusBucket {
    /// Count `sample` and keep it when its rank is among the [`KEY_CENSUS_SAMPLES`]
    /// smallest: a bottom-k sample by a hash of the Tender id, so one corpus gives one
    /// sample whatever the window size, and the sample is not the oldest Tenders.
    fn offer(&mut self, rank: u64, sample: &KeyCensusSample) {
        self.tenders += 1;
        if self.samples.len() < KEY_CENSUS_SAMPLES {
            self.ranks.push(rank);
            self.samples.push(sample.clone());
        } else if let Some((at, &worst)) = self.ranks.iter().enumerate().max_by_key(|(_, r)| **r)
            && rank < worst
        {
            self.ranks[at] = rank;
            self.samples[at] = sample.clone();
        }
    }

    /// The samples in Tender id order, for a reader.
    fn sort(&mut self) {
        let mut both: Vec<(u64, KeyCensusSample)> = self.ranks.drain(..).zip(self.samples.drain(..)).collect();
        both.sort_by_key(|(_, s)| s.tender_id);
        (self.ranks, self.samples) = both.into_iter().unzip();
    }
}

/// What the procedure-key census walked and found, accumulated window by window.
#[derive(Clone, Debug, Default, Serialize)]
pub struct ProcedureKeyCensus {
    /// UUID-keyed Tenders with two or more notices, and their notices.
    pub tenders: u64,
    pub notices: u64,
    /// Of those notices, the ones that name no buyer (never decisive).
    pub notices_without_buyers: u64,
    /// Tenders with fewer than two notices naming a buyer: nothing to compare.
    pub undecidable: u64,
    /// Tenders whose notices fall into two or more buyer-disjoint clusters.
    pub split: u64,
    /// Of `split`, the Tenders that also hold a notice naming no buyer.
    pub split_with_buyerless: u64,
    /// Of `split`, how many notices name no buyer.
    pub split_notices_without_buyers: u64,
    /// Of `split`, the Tenders whose every notice carries the Tender's own key: the
    /// BT-04 reuse itself, with no issue-481 link weld in the Tender.
    pub split_same_key: u64,
    /// Of `split`: some minority cluster's time range overlaps the largest cluster's
    /// (`interleaved`: one platform or authority running procedures side by side under
    /// one key), or none does (`sequential`: a later procedure reusing the key).
    pub interleaved: u64,
    pub sequential: u64,
    /// Of `split`, the Tenders whose every cluster but the largest is a single notice
    /// (the island shape a split would cut out), against balanced clusters (the hub shape).
    pub singleton_minorities: u64,
    /// The split Tenders per bucket ([`KEY_CENSUS_BUCKETS`]).
    pub buckets: BTreeMap<String, KeyCensusBucket>,
    /// The split Tenders by jurisdictions × Sources × gap
    /// (`several-jurisdictions/one-source/gap-gt-1y`).
    pub cross: BTreeMap<String, u64>,
    /// The split Tenders by cluster count: `2`, `3`, `4-5`, `6-10`, `11+`.
    pub cluster_counts: BTreeMap<String, u64>,
    /// The largest cluster count seen, and the [`KEY_CENSUS_SAMPLES`] Tenders with the
    /// most clusters (the hub shape: Tender 42726, 36 notices under 8 buyers).
    pub max_clusters: usize,
    pub hubs: Vec<KeyCensusSample>,
    /// The Tender id the walk reached, and the one it walks to (captured before it).
    pub cursor: i64,
    pub target: i64,
    /// A cancel ended the walk between windows.
    pub stopped: bool,
}

impl ProcedureKeyCensus {
    pub fn new() -> ProcedureKeyCensus {
        ProcedureKeyCensus {
            buckets: KEY_CENSUS_BUCKETS.iter().map(|b| ((*b).to_owned(), KeyCensusBucket::default())).collect(),
            ..Default::default()
        }
    }

    /// One split Tender's sample, with its bucket names.
    fn record_split(&mut self, sample: KeyCensusSample, jurisdictions: &str, sources: &str, gap: &str, span: &str) {
        self.split += 1;
        let clusters = sample.clusters_total;
        let rank = sample_rank(sample.tender_id);
        for bucket in [jurisdictions, sources, gap, span] {
            self.buckets.get_mut(bucket).expect("a census bucket").offer(rank, &sample);
        }
        *self.cross.entry(format!("{jurisdictions}/{sources}/{gap}")).or_default() += 1;
        let histogram = match clusters {
            0..=2 => "2",
            3 => "3",
            4..=5 => "4-5",
            6..=10 => "6-10",
            _ => "11+",
        };
        *self.cluster_counts.entry(histogram.to_owned()).or_default() += 1;
        self.max_clusters = self.max_clusters.max(clusters);
        // The hub list: the most clusters, then the lowest id.
        let key = |s: &KeyCensusSample| (std::cmp::Reverse(s.clusters_total), s.tender_id);
        if self.hubs.len() < KEY_CENSUS_SAMPLES {
            self.hubs.push(sample);
            self.hubs.sort_by_key(key);
        } else if self.hubs.last().is_some_and(|last| key(&sample) < key(last)) {
            self.hubs.pop();
            self.hubs.push(sample);
            self.hubs.sort_by_key(key);
        }
    }

    /// Classify one Tender's notices (its versions, in seq order) by their guard facts.
    fn classify(&mut self, versions: &[store::KeyedTenderVersion], facts: &HashMap<i64, NoticeFacts>) {
        let Some(first) = versions.first() else { return };
        self.tenders += 1;
        self.notices += versions.len() as u64;
        let empty = NoticeFacts::default();
        let notices: Vec<(&store::KeyedTenderVersion, &NoticeFacts)> =
            versions.iter().map(|v| (v, facts.get(&v.notice_id).unwrap_or(&empty))).collect();
        let buyerless = notices.iter().filter(|(_, f)| f.tokens.is_empty()).count();
        self.notices_without_buyers += buyerless as u64;
        let known: Vec<usize> = (0..notices.len()).filter(|&i| !notices[i].1.tokens.is_empty()).collect();
        if known.len() < 2 {
            self.undecidable += 1;
            return;
        }
        let tokens: Vec<&[u32]> = known.iter().map(|&i| notices[i].1.tokens.as_slice()).collect();
        let clusters = buyer_clusters(&tokens);
        if clusters.len() < 2 {
            return;
        }
        if buyerless > 0 {
            self.split_with_buyerless += 1;
            self.split_notices_without_buyers += buyerless as u64;
        }
        let publication = |(v, f): &(&store::KeyedTenderVersion, &NoticeFacts)| format!("{}:{}", f.source, v.publication_id);
        let day = |t: i64| crate::data_quality::day_string(t.div_euclid(86_400)).unwrap_or_default();
        let mut out: Vec<KeyCensusCluster> = Vec::with_capacity(clusters.len());
        // Per cluster: its jurisdictions, its Sources and its time range, for the axes.
        let mut facets: Vec<(BTreeSet<&str>, BTreeSet<&str>, (i64, i64))> = Vec::with_capacity(clusters.len());
        for members in &clusters {
            let mut members: Vec<&(&store::KeyedTenderVersion, &NoticeFacts)> =
                members.iter().map(|&m| &notices[known[m]]).collect();
            members.sort_by_key(|(v, _)| (v.published_at, v.notice_id));
            let jurisdictions: BTreeSet<&str> =
                members.iter().flat_map(|(_, f)| f.jurisdictions.iter().map(String::as_str)).collect();
            let sources: BTreeSet<&str> = members.iter().map(|(_, f)| f.source.as_str()).collect();
            let range = (
                members.first().map_or(0, |(v, _)| v.published_at),
                members.iter().map(|(v, _)| v.published_at).max().unwrap_or(0),
            );
            let other_keys = members.iter().filter(|(_, f)| f.key.as_deref() != Some(first.procedure_key.as_str())).count();
            out.push(KeyCensusCluster {
                other_keys,
                notices: members.len(),
                publications: members.iter().take(SAMPLE_PUBLICATIONS).map(|m| publication(m)).collect(),
                buyer: members.iter().find_map(|(_, f)| f.buyer.clone()).unwrap_or_default(),
                jurisdictions: jurisdictions.iter().map(|j| (*j).to_owned()).collect(),
                sources: sources.iter().map(|s| (*s).to_owned()).collect(),
                first_published: day(range.0),
                last_published: day(range.1),
                gap_days: 0,
            });
            facets.push((jurisdictions, sources, range));
        }
        let mut order: Vec<usize> = (0..out.len()).collect();
        order.sort_by_key(|&i| (std::cmp::Reverse(out[i].notices), i));
        let largest = order[0];
        // The gap axis: each minority cluster's distance from the largest cluster's time
        // range, the cut the split rule reads ("more than N months away").
        let (r0, r1) = facets[largest].2;
        let mut interleaved = false;
        for &i in &order[1..] {
            let (a0, a1) = facets[i].2;
            let gap = (a0.max(r0) - a1.min(r1)).max(0);
            interleaved |= a0.max(r0) <= a1.min(r1);
            out[i].gap_days = gap / 86_400;
        }
        let gap_days = order[1..].iter().map(|&i| out[i].gap_days).min().unwrap_or(0);
        if interleaved {
            self.interleaved += 1;
        } else {
            self.sequential += 1;
        }
        if order[1..].iter().all(|&i| out[i].notices == 1) {
            self.singleton_minorities += 1;
        }
        // Jurisdictions: several when two clusters name known jurisdictions and share none
        // (a cross-border joint procurement in one cluster is not the collision this asks
        // about; a DE cluster against a BG cluster is); unknown when no pair says so and
        // some pair has a side whose buyers name no country; one when every pair shares a
        // known jurisdiction.
        let n = facets.len();
        let pairs = || (0..n).flat_map(move |i| (i + 1..n).map(move |j| (i, j)));
        let known_pair = |(i, j): (usize, usize)| !facets[i].0.is_empty() && !facets[j].0.is_empty();
        let jurisdictions = if pairs().any(|p| known_pair(p) && facets[p.0].0.is_disjoint(&facets[p.1].0)) {
            "several-jurisdictions"
        } else if pairs().all(known_pair) {
            "one-jurisdiction"
        } else {
            "unknown-jurisdiction"
        };
        // Sources: several when two clusters come from disjoint Sources (a DÖE cluster
        // welded to a TED one); a TED notice colliding with a DÖE/TED pair is one Source.
        let sources = if pairs().any(|(i, j)| facets[i].1.is_disjoint(&facets[j].1)) {
            "several-sources"
        } else {
            "one-source"
        };
        if notices.iter().all(|(_, f)| f.key.as_deref() == Some(first.procedure_key.as_str())) {
            self.split_same_key += 1;
        }
        let (lo, hi) = notices
            .iter()
            .fold((i64::MAX, i64::MIN), |(lo, hi), (v, _)| (lo.min(v.published_at), hi.max(v.published_at)));
        let span_days = (hi - lo) / 86_400;
        let span = match span_days {
            ..=90 => "span-le-90d",
            91..=365 => "span-le-1y",
            _ => "span-gt-1y",
        };
        let gap = match gap_days {
            ..=90 => "gap-le-90d",
            91..=365 => "gap-le-1y",
            _ => "gap-gt-1y",
        };
        let clusters_total = out.len();
        let sample = KeyCensusSample {
            tender_id: first.tender_id,
            procedure_key: first.procedure_key.clone(),
            notices: versions.len(),
            span_days,
            gap_days,
            clusters_total,
            clusters: order.into_iter().take(SAMPLE_CLUSTERS).map(|i| out[i].clone()).collect(),
            without_buyers_total: buyerless,
            without_buyers: notices
                .iter()
                .filter(|(_, f)| f.tokens.is_empty())
                .take(SAMPLE_WITHOUT_BUYERS)
                .map(publication)
                .collect(),
        };
        self.record_split(sample, jurisdictions, sources, gap, span);
    }

    fn finish(&mut self) {
        for bucket in self.buckets.values_mut() {
            bucket.sort();
        }
    }
}

/// What the census needs of one notice: its Source, the link guard's token set, the
/// first buyer name and the buyers' register jurisdictions.
#[derive(Clone, Debug, Default)]
struct NoticeFacts {
    source: String,
    /// The notice's own procedure key, as the plan reads it.
    key: Option<String>,
    tokens: Vec<u32>,
    buyer: Option<String>,
    jurisdictions: BTreeSet<String>,
}

/// The buyer clusters of one Tender's notices, as indices into `tokens` (each set
/// non-empty): the connected components of "the guard calls them overlapping"
/// ([`store::buyer_tokens_disjoint`]), so a notice joins a cluster when it overlaps any
/// member. Pairwise, skipping pairs already joined, so a Tender whose notices all
/// overlap costs about one comparison a notice.
fn buyer_clusters(tokens: &[&[u32]]) -> Vec<Vec<usize>> {
    let mut parent: Vec<usize> = (0..tokens.len()).collect();
    fn root(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    for i in 0..tokens.len() {
        for j in i + 1..tokens.len() {
            let (a, b) = (root(&mut parent, i), root(&mut parent, j));
            if a != b && !store::buyer_tokens_disjoint(tokens[i], tokens[j]) {
                parent[b.max(a)] = a.min(b);
            }
        }
    }
    let mut clusters: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..tokens.len() {
        let r = root(&mut parent, i);
        clusters.entry(r).or_default().push(i);
    }
    clusters.into_values().collect()
}

/// A stable pseudo-random rank of a Tender id (splitmix64), for the bottom-k samples.
fn sample_rank(tender_id: i64) -> u64 {
    let mut z = (tender_id as u64).wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// The census facts of `ids`, from each notice's full parse through the link guard's own
/// derivation ([`GuardSide`], then the resolved organizations from
/// `organization_mentions`, as the link census's endpoint read takes them).
async fn notice_facts(db: &store::Db, ids: &[i64]) -> turso::Result<HashMap<i64, NoticeFacts>> {
    let mut out = HashMap::with_capacity(ids.len());
    for batch in ids.chunks(PARSE_BATCH) {
        let mut notices = db.parsed_by_ids(batch).await?;
        normalise_de1(&mut notices);
        let orgs = Box::pin(db.mentions_by_ids(batch)).await?;
        for (notice, parsed) in notices {
            let sdk01 = is_sdk01_profile(&notice.profile);
            let key = procedure_key(&parsed, sdk01, is_de1_profile(&notice.profile));
            let guard = GuardSide::read(sdk01, notice.id, &parsed);
            let buyer = guard.buyers().iter().map(|m| m.name.trim()).find(|n| !n.is_empty()).map(str::to_owned);
            let jurisdictions = guard
                .buyers()
                .iter()
                .filter_map(|m| m.country.as_deref().map(str::trim).filter(|c| !c.is_empty()))
                .map(|c| store::register_jurisdiction(c).to_owned())
                .collect();
            let mut tokens = guard.tokens();
            add_org_tokens(&mut tokens, &guard.sections(), orgs.get(&notice.id));
            out.insert(notice.id, NoticeFacts { source: notice.source, key, tokens, buyer, jurisdictions });
        }
    }
    Ok(out)
}

/// Issue 482 unit 1: the procedure-key census over the whole Tender layer, in
/// [`KEY_CENSUS_WINDOW`]-id windows. Read-only. `stop` is read before every window, and a
/// stopped census is a prefix (`stopped`), which the caller does not store.
pub async fn procedure_key_census(
    db: &store::Db,
    stop: &(dyn Fn() -> bool + Sync),
    progress: impl FnMut(&ProcedureKeyCensus),
) -> turso::Result<ProcedureKeyCensus> {
    procedure_key_census_windowed(db, KEY_CENSUS_WINDOW, stop, progress).await
}

/// [`procedure_key_census`] with an explicit window, for the tests.
pub async fn procedure_key_census_windowed(
    db: &store::Db,
    window: i64,
    stop: &(dyn Fn() -> bool + Sync),
    mut progress: impl FnMut(&ProcedureKeyCensus),
) -> turso::Result<ProcedureKeyCensus> {
    debug_assert!(window > 0, "a non-positive window could not advance the cursor");
    let mut report = ProcedureKeyCensus::new();
    report.target = db.max_tender_id().await?;
    while report.cursor < report.target {
        if stop() {
            report.stopped = true;
            break;
        }
        let hi = report.cursor.saturating_add(window).min(report.target);
        let versions = db.uuid_keyed_tender_versions(report.cursor, hi).await?;
        // The Tenders of the window, each with two or more notices and a genuine UUID key.
        let mut tenders: Vec<&[store::KeyedTenderVersion]> = Vec::new();
        for group in versions.chunk_by(|a, b| a.tender_id == b.tender_id) {
            if group.len() >= 2 && is_uuid(&group[0].procedure_key) {
                tenders.push(group);
            }
        }
        let ids: Vec<i64> = tenders.iter().flat_map(|t| t.iter().map(|v| v.notice_id)).collect();
        // Boxed (issue 467's stack budgets): the parse batches stay off the walk's frame.
        let facts = Box::pin(notice_facts(db, &ids)).await?;
        for tender in tenders {
            report.classify(tender, &facts);
        }
        report.cursor = hi;
        progress(&report);
    }
    report.finish();
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(name: &str) -> Vec<u32> {
        let mut t = vec![store::buyer_token(name)];
        t.sort_unstable();
        t
    }

    #[test]
    fn clusters_are_transitive_and_disjoint_sets_stay_apart() {
        let (a, b, c) = (digest("a"), digest("b"), digest("c"));
        let mut ab = [a.clone(), b.clone()].concat();
        ab.sort_unstable();
        // a — ab — b chain into one cluster; c stands alone.
        let sets: Vec<&[u32]> = vec![&a, &c, &b, &ab];
        let clusters = buyer_clusters(&sets);
        assert_eq!(clusters, vec![vec![0, 2, 3], vec![1]]);
        // Without the bridging joint notice, a and b are apart.
        let sets: Vec<&[u32]> = vec![&a, &b];
        assert_eq!(buyer_clusters(&sets).len(), 2);
    }

    /// A Tender of many buyer-disjoint notices and many buyerless ones stays a bounded
    /// sample: the largest clusters and the first buyerless notices, with their totals,
    /// and the hub list ranks it by the total, not by the capped list.
    #[test]
    fn a_huge_tender_is_a_bounded_sample_ranked_by_its_cluster_total() {
        let key = "f3943baf-54ae-441a-aee9-3e802998024a";
        let (disjoint, buyerless) = (25_i64, 45_i64);
        let versions: Vec<store::KeyedTenderVersion> = (0..disjoint + buyerless)
            .map(|i| store::KeyedTenderVersion {
                tender_id: 7,
                procedure_key: key.to_owned(),
                notice_id: i,
                publication_id: format!("{i:08}-2024"),
                published_at: 1_700_000_000 + i * 86_400,
            })
            .collect();
        let facts: HashMap<i64, NoticeFacts> = (0..disjoint + buyerless)
            .map(|i| {
                let named = i < disjoint;
                let facts = NoticeFacts {
                    source: "ted".to_owned(),
                    key: Some(key.to_owned()),
                    tokens: if named { digest(&format!("buyer {i}")) } else { Vec::new() },
                    buyer: named.then(|| format!("buyer {i}")),
                    jurisdictions: BTreeSet::new(),
                };
                (i, facts)
            })
            .collect();
        let mut report = ProcedureKeyCensus::new();
        report.classify(&versions, &facts);
        assert_eq!(report.split, 1);
        assert_eq!(report.max_clusters, 25);
        assert_eq!(report.cluster_counts.get("11+"), Some(&1));
        let hub = &report.hubs[0];
        assert_eq!((hub.clusters_total, hub.clusters.len()), (25, SAMPLE_CLUSTERS));
        assert_eq!((hub.without_buyers_total, hub.without_buyers.len()), (45, SAMPLE_WITHOUT_BUYERS));
        assert_eq!(report.buckets["unknown-jurisdiction"].tenders, 1, "no buyer names a country");
        assert_eq!((report.sequential, report.singleton_minorities), (1, 1));
        assert_eq!(hub.gap_days, 1, "the nearest minority cluster is a day from the largest");
    }

    #[test]
    fn a_bucket_keeps_the_smallest_ranks_whatever_the_order() {
        let sample = |id: i64| KeyCensusSample {
            tender_id: id,
            procedure_key: String::new(),
            notices: 2,
            span_days: 0,
            gap_days: 0,
            clusters_total: 0,
            clusters: Vec::new(),
            without_buyers_total: 0,
            without_buyers: Vec::new(),
        };
        let mut forward = KeyCensusBucket::default();
        let mut backward = KeyCensusBucket::default();
        for id in 0..200 {
            forward.offer(sample_rank(id), &sample(id));
            backward.offer(sample_rank(199 - id), &sample(199 - id));
        }
        forward.sort();
        backward.sort();
        assert_eq!(forward.tenders, 200);
        assert_eq!(forward.samples.len(), KEY_CENSUS_SAMPLES);
        assert_eq!(forward.samples, backward.samples);
    }
}
