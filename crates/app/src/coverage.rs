//! The dashboard's measurement half: one read of the store turned into the
//! numbers `/` renders.
//!
//! Coverage is the interesting one. "How complete is this database?" needs a
//! denominator, and the only honest one is what the source is *known* to have
//! published — so the per-year TED notice counts established in
//! docs/research/ted-access-channels.md §6 are vendored beside this file and
//! compared against what we hold. Before any backfill the ratios are near zero,
//! which is the correct answer, not a bug to hide.

use crate::ledger::resolution_ledger;
use model::dashboard::{
    AwardLinkage, Count, Coverage, Dashboard, Lag, PipelineStage, QuarantineClass, Quarantined,
    ResolvedCategory, quarantine_class,
};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};
use std::time::Duration;
use store::Db;

/// Notice counts per TED publication year — the coverage denominator. Vendored
/// as data (not code) so refreshing it is an edit to a table, not a patch.
const GROUND_TRUTH: &str = include_str!("../data/ted-notice-counts.csv");

/// The source the ground-truth table describes. Other sources show holdings
/// with no ratio until their own volumes are measured.
const GROUND_TRUTH_SOURCE: &str = "ted";

/// How many quarantined payloads the drill-down lists.
const QUARANTINE_SAMPLE: i64 = 50;

/// How many top field codes to surface behind the `unknown-field-code` bucket.
const FIELD_CODE_GAPS: i64 = 5;

/// One vendored row: what the year published, and whether the year is over.
struct Published {
    year: String,
    notices: i64,
    partial: bool,
}

fn ground_truth() -> Vec<Published> {
    GROUND_TRUTH
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let mut fields = line.split(',');
            Some(Published {
                year: fields.next()?.to_owned(),
                notices: fields.next()?.parse().ok()?,
                partial: fields.next()? == "1",
            })
        })
        .collect()
}

/// How often the background refresher re-measures. Generous on purpose: the
/// dashboard already shows live job progress from the supervisor (which never
/// scans), so the coverage numbers moving on a minute boundary is invisible —
/// and a background scan keeps the millions-of-rows read entirely off the
/// request path (issue 20 part 3).
const REFRESH: Duration = Duration::from_secs(60);

/// The last measured dashboard, recomputed by the background refresher and read
/// by requests. This is the availability property a request-path TTL cache could
/// not give: under sustained write churn its scan was always cold (page-cache
/// thrash) so the TTL never protected, and concurrent cold misses each scanned
/// and pinned a core. Off the request path, no public traffic can ever trigger a
/// scan — the page is served from memory, stale-while-revalidate.
static SNAPSHOT: OnceLock<RwLock<Option<Dashboard>>> = OnceLock::new();

fn cell() -> &'static RwLock<Option<Dashboard>> {
    SNAPSHOT.get_or_init(|| RwLock::new(None))
}

/// Serve the newest memoized snapshot — a synchronous, store-free read, so a
/// request can never recompute or block. Before the first refresh completes (a
/// fresh boot) this is the empty default, which the page renders as "no data
/// yet"; its `measured_at` tells the client how fresh it is.
pub fn latest() -> Dashboard {
    cell().read().expect("coverage snapshot").clone().unwrap_or_default()
}

/// Spawn the background refresher: measure on an interval, off the request path,
/// keeping the last good snapshot when a measurement errors. Idempotent — a
/// second call (dev hot-reload re-runs the server initializer) does not spawn a
/// second loop. Runs one measurement immediately so the first fill is prompt.
pub fn init(db: Arc<Db>) {
    static STARTED: OnceLock<()> = OnceLock::new();
    if STARTED.set(()).is_err() {
        return;
    }
    tokio::spawn(async move {
        loop {
            let now = store::now_unix();
            match measure(&db, now).await {
                Ok(dash) => *cell().write().expect("coverage snapshot") = Some(dash),
                Err(e) => eprintln!("coverage: refresh failed, keeping last snapshot: {e}"),
            }
            tokio::time::sleep(REFRESH).await;
        }
    });
}

/// Measure everything the dashboard shows, in one pass, so the panels are one
/// consistent snapshot rather than four independently-timed ones. One store
/// scan — the background refresher ([`init`]) runs it off the request path;
/// requests read [`latest`] instead.
pub async fn measure(db: &Db, now: i64) -> store::turso::Result<Dashboard> {
    let truth = ground_truth();
    let coverage: Vec<Coverage> = db
        .notice_counts_by_profile_year()
        .await?
        .into_iter()
        .map(|cell| {
            let published = (cell.source == GROUND_TRUTH_SOURCE)
                .then(|| truth.iter().find(|p| p.year == cell.year))
                .flatten();
            Coverage {
                source: cell.source,
                profile: cell.profile,
                year: cell.year,
                held: cell.notices,
                published: published.map(|p| p.notices),
                ratio: published.map(|p| cell.notices as f64 / p.notices as f64),
                partial: published.is_some_and(|p| p.partial),
            }
        })
        .collect();

    let quarantine_by_reason: Vec<Count> = db
        .quarantine_counts_by_reason()
        .await?
        .into_iter()
        .map(|(label, value)| Count { label, value })
        .collect();
    // Split the headline three ways (issue 30): confirmed real-notice loss is the
    // number that matters; the ~1.2M suspected parser gaps are flagged distinctly;
    // the small benign remainder is neither.
    let class_total = |class| {
        quarantine_by_reason
            .iter()
            .filter(|c| quarantine_class(&c.label) == class)
            .map(|c| c.value)
            .sum()
    };
    let quarantine_actionable = class_total(QuarantineClass::Actionable);
    let quarantine_suspected = class_total(QuarantineClass::SuspectedGap);
    let quarantine_field_code_gaps: Vec<Count> = db
        .quarantine_field_code_gaps(FIELD_CODE_GAPS)
        .await?
        .into_iter()
        .map(|(label, value)| Count { label, value })
        .collect();

    // The resolution ledger (issue 40): each curated entry joined with its live
    // reclaimed/outstanding counts, so a fixed-and-reprocessed category keeps
    // telling its story after its count reaches zero. Off the request path with
    // the rest of measure; the ledger is a handful of entries.
    let mut resolved_categories = Vec::new();
    for entry in resolution_ledger() {
        let (reclaimed, outstanding) = db
            .quarantine_resolution(&entry.reason, entry.profile.as_deref(), entry.detail_like.as_deref())
            .await?;
        resolved_categories.push(ResolvedCategory {
            category: entry.category,
            diagnosis: entry.diagnosis,
            fix: entry.fix,
            resolved: entry.resolved,
            reclaimed,
            outstanding,
        });
    }

    // The import pipeline per source (issue 33): fetch registry + the notice
    // counts already gathered + projected tenders, so the operator sees which
    // stage the backfill is in without ssh. All cheap, all off the request path.
    let mut processed: HashMap<String, i64> = HashMap::new();
    for c in &coverage {
        *processed.entry(c.source.clone()).or_default() += c.held;
    }
    let published_ted: i64 = truth.iter().map(|p| p.notices).sum();
    let projected: HashMap<String, i64> = db.tenders_by_source().await?.into_iter().collect();
    // "Fetch complete" = the latest fetched period is in the current year, i.e.
    // downloading has caught up to the present (periods are YYYY-prefixed).
    let current_year = (1970 + now / 31_557_600).to_string();
    let pipeline: Vec<PipelineStage> = db
        .fetch_registry_summary()
        .await?
        .into_iter()
        .map(|(source, fetched_packages, from, to)| PipelineStage {
            published: (source == GROUND_TRUTH_SOURCE).then_some(published_ted),
            fetched_packages,
            fetch_complete: to.starts_with(&current_year),
            fetched_from: Some(from),
            fetched_to: Some(to),
            processed_notices: processed.get(&source).copied().unwrap_or(0),
            projected_tenders: projected.get(&source).copied().unwrap_or(0),
            source,
        })
        .collect();

    let lag = db.import_lag().await?;
    let counts: Vec<Count> = db
        .canonical_counts()
        .await?
        .into_iter()
        .map(|(label, value)| Count { label, value })
        .collect();

    let award_linkage: Vec<AwardLinkage> = db
        .award_linkage()
        .await?
        .into_iter()
        .map(|(era, awards, unchained)| AwardLinkage {
            era,
            awards,
            unchained,
            ratio: if awards > 0 { unchained as f64 / awards as f64 } else { 0.0 },
        })
        .collect();

    Ok(Dashboard {
        measured_at: now,
        coverage,
        pipeline,
        quarantine_total: quarantine_by_reason.iter().map(|c| c.value).sum(),
        quarantine_actionable,
        quarantine_suspected,
        quarantine_by_reason,
        quarantine_field_code_gaps,
        quarantine_recent: db
            .recent_quarantine(QUARANTINE_SAMPLE)
            .await?
            .into_iter()
            .map(|q| Quarantined {
                reason: q.reason,
                profile: q.profile,
                member_path: q.member_path,
                detail: q.detail,
                first_seen: q.first_seen,
            })
            .collect(),
        resolved_categories,
        // Ages, not instants: the client has its own clock, and a browser whose
        // clock is wrong should not be able to report the import as healthy.
        lag: Lag {
            fetch_age: lag.newest_fetch_at.map(|at| now - at),
            notice_age: lag.newest_notice_at.map(|at| now - at),
        },
        counts,
        award_linkage,
        cursor: db.latest_cursor().await?,
        service_rev: crate::v1::rev().to_owned(),
        // An age, not an instant — same discipline as the import lag above.
        snapshot_age: db.last_snapshot_at().await?.map(|at| now - at),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_vendored_ground_truth_parses_and_covers_the_ted_era() {
        let truth = ground_truth();
        assert_eq!(truth.first().map(|p| p.year.as_str()), Some("1993"));
        assert_eq!(truth.len(), 2026 - 1993 + 1, "one row per year 1993–2026");
        // Spot-check the transcription against the research table.
        assert_eq!(truth.iter().find(|p| p.year == "1993").map(|p| p.notices), Some(74_433));
        assert_eq!(truth.iter().find(|p| p.year == "2011").map(|p| p.notices), Some(411_850));
        assert_eq!(truth.iter().find(|p| p.year == "2025").map(|p| p.notices), Some(871_149));
        // The rows sum to 13.31 M. Note that ted-access-channels.md §6 quotes
        // "≈12.9 M" beneath the same table — its headline is a stale rounding of
        // its own rows, and the rows are the numbers we transcribed.
        let total: i64 = truth.iter().map(|p| p.notices).sum();
        assert_eq!(total, 13_312_103);
        // Only the current year is partial.
        assert_eq!(truth.iter().filter(|p| p.partial).count(), 1);
    }

    /// Issue 20 part 3: the request path (`latest`) is a synchronous, store-free
    /// read of the memoized snapshot — it can never scan, no matter how slow a
    /// measurement would be. `latest` takes no `Db` and is not `async`, so "a
    /// request recomputes" is a compile error, not just a runtime assertion; here
    /// we confirm it returns exactly what the refresher last stored.
    #[test]
    fn latest_serves_the_memoized_snapshot_without_measuring() {
        // A marker the empty default never carries.
        let snapshot = Dashboard { cursor: 4242, ..Dashboard::default() };
        *cell().write().unwrap() = Some(snapshot);
        assert_eq!(latest().cursor, 4242, "the request path returns the memoized snapshot");
    }
}
