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
    AwardLinkage, Count, Coverage, Dashboard, Lag, PipelineStage, Quarantine, QuarantineClass,
    Quarantined, ResolvedCategory, System, quarantine_class,
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
    /// Issue 396 unit 1: for a PARTIAL year, the date this count was taken.
    /// Served beside the denominator, because a frozen mid-year number presented
    /// undated reads as a coverage percentage — and 2026 passed 117 % as the
    /// corpus grew past a 2026-07-17 snapshot the page never dated.
    as_of: Option<String>,
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
                // Optional 4th column; absent for every complete year.
                as_of: fields.next().map(str::trim).filter(|d| !d.is_empty()).map(str::to_owned),
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
static SNAPSHOT: OnceLock<RwLock<Dashboard>> = OnceLock::new();

fn cell() -> &'static RwLock<Dashboard> {
    SNAPSHOT.get_or_init(|| RwLock::new(Dashboard::default()))
}

/// Serve the newest memoized snapshot — a synchronous, store-free read, so a
/// request can never recompute or block. Its sections fill in independently
/// (issue 37): before a section's first measurement it is `None`, which the page
/// renders as "measuring since boot…" — never as zeros.
pub fn latest() -> Dashboard {
    cell().read().expect("coverage snapshot").clone()
}

/// Spawn the background refresher: measure on an interval, off the request path,
/// filling each section independently so a slow scan can't hold the cheap ones
/// hostage and a boot never shows zeros (issue 37). Idempotent — a second call
/// (dev hot-reload re-runs the server initializer) does not spawn a second loop.
pub fn init(db: Arc<Db>) {
    static STARTED: OnceLock<()> = OnceLock::new();
    if STARTED.set(()).is_err() {
        return;
    }
    // Ops safety valve: turn the dashboard measurement off entirely.
    if std::env::var_os("TENDER_DISABLE_COVERAGE").is_some() {
        eprintln!("coverage: refresher disabled via TENDER_DISABLE_COVERAGE");
        return;
    }
    // Run the refresher on its OWN dedicated thread + runtime, NOT the HTTP runtime
    // (issue 61 coverage regression). The heavy sections do turso full-table scans,
    // and turso does BLOCKING preads inline on the executing worker thread; at
    // projection scale a cold ~25GB scan pinned the HTTP runtime's workers until the
    // acceptor starved and EVERY endpoint hung — even the memoized root `/` (no DB),
    // because the runtime had no free thread to poll it. Isolated here, those
    // blocking preads pin this one thread; the API runtime stays free. Mirrors the
    // job-worker isolation (supervisor::spawn_worker_runtime).
    std::thread::Builder::new()
        .name("coverage".into())
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("build the coverage runtime");
            rt.block_on(async move {
                let mut last_heavy_key: Option<HeavyKey> = None;
                loop {
                    // Skip the heavy coverage scan while a write-heavy job holds the
                    // WAL (issue 53) — see `refresh_into`. The supervisor may not
                    // exist yet in a unit-test server, in which case nothing writes.
                    let sup = crate::supervisor::get();
                    let heavy_write = sup.as_ref().is_some_and(|s| s.heavy_write_in_progress());
                    // Concluded-job count in the watermark (issue 191): a job's
                    // in-place writes (reclaim stamps, skip flags, parse-state
                    // flips) move no cursor and add no rows, so without this a
                    // finished reprocess never triggered a heavy re-measure.
                    let jobs_completed = sup.map_or(0, |s| s.jobs_completed());
                    refresh_into(&db, cell(), heavy_write, jobs_completed, &mut last_heavy_key).await;
                    tokio::time::sleep(REFRESH).await;
                }
            });
        })
        .expect("spawn the coverage thread");
}

/// A cheap O(1) watermark of everything the heavy dashboard sections depend on:
/// the in-memory change cursor (canonical/projection writes) plus the newest fetch
/// and notice instants (ingestion). Equal across two refreshes ⇒ nothing was
/// written between them ⇒ the heavy full-table scans would recompute identical
/// numbers, so they can be skipped. This is what stops an IDLE server from re-running
/// the ~25GB scan every 60s (issue 61 coverage: even isolated, that cold-scan I/O
/// every minute is real disk load that competes with ingestion).
#[derive(Clone, PartialEq, Eq)]
struct HeavyKey {
    cursor: i64,
    newest_fetch_at: Option<i64>,
    newest_notice_at: Option<i64>,
    /// Concluded-job count (issue 191). A reprocess stamps quarantine rows and
    /// flips parse states IN PLACE — none of the three fields above move — so
    /// without this the heavy sections were never re-measured after a reclaim
    /// and the panel served hours-stale numbers under a fresh `measured_at`.
    jobs_completed: u64,
}

/// Read the current [`HeavyKey`] — O(1): the cursor from the in-memory doorbell (no
/// DB), the fetch/notice instants from `import_lag` (small-table MAX + an id-PK
/// read). `None` only if the watermark read itself errors, in which case the caller
/// measures (never skips on an error).
async fn current_heavy_key(db: &Db, jobs_completed: u64) -> Option<HeavyKey> {
    let lag = db.import_lag().await.ok()?;
    Some(HeavyKey {
        cursor: db.current_cursor(),
        newest_fetch_at: lag.newest_fetch_at,
        newest_notice_at: lag.newest_notice_at,
        jobs_completed,
    })
}

/// One refresh pass: measure each section and publish it the moment it is ready,
/// cheapest first. Each section is independent — a section that errors keeps its
/// last good value (stale-while-revalidate) rather than blanking, and a slow or
/// poisoned section (e.g. the coverage scan under ingestion load) never delays
/// the sections ahead of it. `system` lands within a second of a restart; the
/// full-table-scan sections land as each completes.
async fn refresh_into(
    db: &Db,
    cell: &RwLock<Dashboard>,
    heavy_write_active: bool,
    jobs_completed: u64,
    last_heavy_key: &mut Option<HeavyKey>,
) {
    let now = store::now_unix();
    // `system` is the only measurement safe to run while a write-heavy job holds
    // the WAL: cursor (MAX over the changes PK), a `job_log` point read, and the
    // import lag — all O(1) point reads, no table-proportional scan, so none holds
    // a reader snapshot long enough to matter. This claim is load-bearing: the lag's
    // newest-notice read MUST stay O(1) (id-PK, not `MAX(ingested_at)` which
    // full-scans notices) or `measure_system` becomes the WAL-pinning reader it was
    // in the field (issue 42/53, store::Db::import_lag). It lands within a second of
    // a restart and keeps its per-60s cadence throughout ingestion.
    publish(cell, "system", measure_system(db, now).await, |d, v| d.system = Some(v));

    // EVERYTHING BELOW holds a live reader snapshot for the duration of a
    // table-proportional scan, which pins the WAL: while a write-heavy job runs,
    // the package/batch TRUNCATE (issue 42) needs reader-free windows to reclaim,
    // and a pinned snapshot blocks even turso's mid-package PASSIVE autocheckpoint
    // — the WAL grew to 70 GB and throughput collapsed to 5 n/s in the field
    // (issue 53). So skip ALL of them while such a job is active and keep the last
    // measured values (stale-while-revalidate); they barely move within one job,
    // and re-measure in the idle gap after it. Each is here because it scans:
    //  * `quarantine` — the reason `GROUP BY` and especially the resolution-ledger
    //    `detail LIKE` scans sweep 1.2M-row / 577k+ big buckets (issue 40); the
    //    reason index seeks the bucket but the ledger LIKE still scans it. This was
    //    the third pin the first two gate passes missed.
    //  * `award-linkage` — indexed `EXISTS` per era, trivial pre-projection but a
    //    heavy canonical scan once the projection grows those tables.
    //  * `counts` — `COUNT(*)` over every canonical table, same projection-scale trap.
    //  * `coverage`/`pipeline` — the one full `notices` `GROUP BY`, the heaviest read.
    // (Gated `quarantine`/`coverage` render as "measuring…" not a false `0` — the
    // sectioned `None` default, issue 37 — so the boot-zero guard still holds.)
    if heavy_write_active {
        return;
    }
    // Change-gate (issue 61 coverage): when nothing has been written since the last
    // heavy measure, keep the last values and DON'T re-scan — so an idle server never
    // re-runs the ~25GB full-table scan (it fired every 60s and, even isolated, its
    // cold-scan I/O competes with ingestion). The watermark is O(1). A fully-
    // quarantined package with no new notice is the only residual staleness, tolerable
    // on a dashboard; during an active job the `heavy_write` gate above already skips.
    let key = current_heavy_key(db, jobs_completed).await;
    if let (Some(k), Some(last)) = (key.as_ref(), last_heavy_key.as_ref())
        && k == last
    {
        return;
    }
    publish(cell, "quarantine", timed("quarantine", measure_quarantine(db)).await, |d, v| {
        d.quarantine = Some(v)
    });
    publish(cell, "award-linkage", timed("award-linkage", measure_award_linkage(db)).await, |d, v| {
        d.award_linkage = Some(v)
    });
    publish(cell, "counts", timed("counts", measure_counts(db)).await, |d, v| d.counts = Some(v));
    publish(
        cell,
        "coverage",
        timed("coverage", measure_coverage_pipeline(db, now)).await,
        |d, (coverage, pipeline)| {
            d.coverage = Some(coverage);
            d.pipeline = Some(pipeline);
        },
    );
    // Record the watermark the heavy sections were measured at, so the next idle pass
    // with an unchanged DB skips the scan above.
    if let Some(k) = key {
        *last_heavy_key = Some(k);
    }
}

/// Measure every section once and return the assembled snapshot — the one-shot
/// form (tests, a future "measure now"). The running server never calls this: it
/// uses the background refresher ([`init`]) and serves [`latest`].
pub async fn measure(db: &Db) -> Dashboard {
    let cell = RwLock::new(Dashboard::default());
    // One-shot: measure every section, including the heavy coverage scan. A fresh
    // `None` watermark forces the measure (never skips).
    refresh_into(db, &cell, false, 0, &mut None).await;
    cell.into_inner().expect("snapshot")
}

/// Say how long a heavy section took, whichever way it went (issue 405).
///
/// Until this existed the refresher was mute on success — `publish` logs the
/// `Err` arm only — so a section that had NEVER completed and one completing
/// every minute produced byte-identical logs: none. That made "Measuring…" on the
/// dashboard unreadable. On 2026-09-16 the `coverage` panel was blank for the
/// quarter-hour after a deploy while `counts`, `quarantine` and `award-linkage`
/// were populated, and nothing on the box could say whether the scan was running,
/// wedged on a pinned reader, or disabled. It was running — but the only way to
/// learn that was to poll the endpoint from outside for a quarter of an hour.
///
/// `coverage` is published LAST of the four, which is why it is reliably the one
/// a reader sees missing.
///
/// The first thing this instrument said, on the pass after its own deploy
/// (2026-09-16 14:12–14:15Z): quarantine 18.6 s, award-linkage 37.2 s, **counts
/// 182.6 s**, coverage **7.4 s** — about four minutes in total, with `counts`
/// dominating and the section this module calls "the heaviest read" in the gate
/// comment above being the cheapest of the four. Read that with its caveat: the
/// four run in sequence, so `coverage` scans `notices` immediately after `counts`
/// has already walked it, and a cold first pass may divide differently. Which is
/// the point — that question is now answerable from the log across restarts
/// instead of being settled by a comment nobody could check.
async fn timed<T, E>(
    section: &str,
    measure: impl std::future::Future<Output = Result<T, E>>,
) -> Result<T, E> {
    let t = std::time::Instant::now();
    let out = measure.await;
    // Logged on the error path too: how long it took to fail is the difference
    // between a timeout and a refusal, and `publish` says only that it failed.
    eprintln!(
        "coverage: {section} measured in {:.1}s{}",
        t.elapsed().as_secs_f64(),
        if out.is_err() { " (failed)" } else { "" }
    );
    out
}

/// Publish one measured section into the snapshot, or keep the last good value
/// and log if the measurement failed. The lock is held only for the swap.
fn publish<T, E: std::fmt::Display>(
    cell: &RwLock<Dashboard>,
    section: &str,
    measured: Result<T, E>,
    set: impl FnOnce(&mut Dashboard, T),
) {
    match measured {
        Ok(v) => set(&mut cell.write().expect("coverage snapshot"), v),
        Err(e) => eprintln!("coverage: {section} refresh failed, keeping last value: {e}"),
    }
}

/// The cheap status section — cursor, revision, staleness. No full-table scan, so
/// it is the first to land after a restart.
async fn measure_system(db: &Db, now: i64) -> store::turso::Result<System> {
    let lag = db.import_lag().await?;
    Ok(System {
        measured_at: now,
        cursor: db.latest_cursor().await?,
        service_rev: crate::v1::rev().to_owned(),
        // An age, not an instant — a browser with a wrong clock must not be able
        // to report the import as healthy.
        lag: Lag {
            fetch_age: lag.newest_fetch_at.map(|at| now - at),
            notice_age: lag.newest_notice_at.map(|at| now - at),
        },
    })
}

/// The `Contents` counts of the canonical layer.
async fn measure_counts(db: &Db) -> store::turso::Result<Vec<Count>> {
    Ok(db.canonical_counts().await?.into_iter().map(|(label, value)| Count { label, value }).collect())
}

/// Award-chaining health per era (indexed-`EXISTS`, issue 38).
async fn measure_award_linkage(db: &Db) -> store::turso::Result<Vec<AwardLinkage>> {
    Ok(db
        .award_linkage()
        .await?
        .into_iter()
        .map(|(era, awards, unchained)| AwardLinkage {
            era,
            awards,
            unchained,
            ratio: if awards > 0 { unchained as f64 / awards as f64 } else { 0.0 },
        })
        .collect())
}

/// The quarantine section (issues 30/40). The reason breakdown, field-code gaps
/// and the resolution ledger all filter/group by `reason`, now indexed
/// (`quarantine_reason`) so each seeks its bucket instead of scanning 1.2M rows.
async fn measure_quarantine(db: &Db) -> store::turso::Result<Quarantine> {
    // Split three ways (issue 137): `by_reason` carries what is STILL HELD, so
    // every downstream classification describes the present rather than the
    // union of the present and everything already fixed. A reason whose rows
    // were all reclaimed now shows 0 instead of its historical size.
    let split = db.quarantine_counts_by_reason_split().await?;
    let by_reason: Vec<Count> = split
        .iter()
        .filter(|(_, outstanding, _, _)| *outstanding > 0)
        .map(|(label, outstanding, _, _)| Count { label: label.clone(), value: *outstanding })
        .collect();
    let outstanding_total: i64 = split.iter().map(|(_, o, _, _)| *o).sum();
    let reclaimed_total: i64 = split.iter().map(|(_, _, r, _)| *r).sum();
    let skipped_total: i64 = split.iter().map(|(_, _, _, s)| *s).sum();
    // Split the headline three ways (issue 30): confirmed real-notice loss is the
    // number that matters; the ~1.2M suspected parser gaps are flagged distinctly;
    // the small benign remainder is neither.
    let class_total = |class| {
        by_reason.iter().filter(|c| quarantine_class(&c.label) == class).map(|c| c.value).sum()
    };
    let field_code_gaps: Vec<Count> = db
        .quarantine_field_code_gaps(FIELD_CODE_GAPS)
        .await?
        .into_iter()
        .map(|(label, value)| Count { label, value })
        .collect();
    let recent: Vec<Quarantined> = db
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
        .collect();
    // The resolution ledger (issue 40): each curated entry joined with its live
    // reclaimed/outstanding counts, so a fixed-and-reprocessed category keeps
    // telling its story after its count reaches zero. The ledger is a handful of
    // entries, each an indexed reason-seek.
    let mut resolved_categories = Vec::new();
    for entry in resolution_ledger() {
        let (reclaimed, skipped, outstanding) = db
            .quarantine_resolution(
                &entry.reason,
                entry.profile.as_deref(),
                entry.detail_like.as_deref(),
                entry.member_path_like.as_deref(),
                entry.member_path_unlike.as_deref(),
            )
            .await?;
        resolved_categories.push(ResolvedCategory {
            category: entry.category,
            diagnosis: entry.diagnosis,
            fix: entry.fix,
            resolved: entry.resolved,
            reclaimed,
            skipped,
            outstanding,
        });
    }
    Ok(Quarantine {
        total: outstanding_total + reclaimed_total + skipped_total,
        outstanding: outstanding_total,
        reclaimed: reclaimed_total,
        skipped: skipped_total,
        actionable: class_total(QuarantineClass::Actionable),
        suspected: class_total(QuarantineClass::SuspectedGap),
        by_reason,
        field_code_gaps,
        recent,
        resolved_categories,
    })
}

/// The coverage grid and the import funnel (issue 33), which share the one
/// notices scan — the heaviest read on the dashboard, so they land last.
async fn measure_coverage_pipeline(
    db: &Db,
    now: i64,
) -> store::turso::Result<(Vec<Coverage>, Vec<PipelineStage>)> {
    let truth = ground_truth();
    let cells = db.notice_counts_by_profile_year().await?;
    // Notices each (source, year) holds across ALL its profiles, and how many
    // profiles serve it (issue 229). The denominator is always the whole year, so
    // a year at an era boundary — 2008 carries internal-ojs AND text — has no
    // per-profile ratio to report: dividing one profile's share by the year's
    // total read 0.079 and 0.922 for a year that was in fact complete.
    let mut per_year: std::collections::HashMap<(String, String), (i64, usize)> = Default::default();
    for cell in &cells {
        let e = per_year.entry((cell.source.clone(), cell.year.clone())).or_insert((0, 0));
        e.0 += cell.notices;
        e.1 += 1;
    }
    let coverage: Vec<Coverage> = cells
        .into_iter()
        .map(|cell| {
            let published = (cell.source == GROUND_TRUTH_SOURCE)
                .then(|| truth.iter().find(|p| p.year == cell.year))
                .flatten();
            let (year_held, profiles) = per_year
                .get(&(cell.source.clone(), cell.year.clone()))
                .copied()
                .unwrap_or((cell.notices, 1));
            let shared = profiles > 1;
            Coverage {
                source: cell.source,
                profile: cell.profile,
                year: cell.year,
                held: cell.notices,
                published: published.map(|p| p.notices),
                // Suppressed for a shared year: there is no per-profile ground
                // truth, so any number here would be a gap report about nothing.
                ratio: (!shared)
                    .then(|| published.map(|p| cell.notices as f64 / p.notices as f64))
                    .flatten(),
                partial: published.is_some_and(|p| p.partial),
                published_as_of: published.and_then(|p| p.as_of.clone()),
                year_held,
                year_ratio: published.map(|p| year_held as f64 / p.notices as f64),
            }
        })
        .collect();

    // The import pipeline per source: fetch registry + the notice counts already
    // gathered + projected tenders, so the operator sees which stage the backfill
    // is in without ssh.
    let mut processed: HashMap<String, i64> = HashMap::new();
    for c in &coverage {
        *processed.entry(c.source.clone()).or_default() += c.held;
    }
    let published_ted: i64 = truth.iter().map(|p| p.notices).sum();
    let projected: HashMap<String, i64> = db.tenders_by_source().await?.into_iter().collect();
    // "Fetch complete" needs BOTH halves (issue 395). The old test was only the
    // first — "the latest fetched period is in the current year", i.e. downloading
    // has caught up to the present — and it greenlit a missing TED 2025-06 for
    // months, because a hole in the middle leaves the newest period exactly where
    // it was. The second half is a contiguity test over the monthly sequence, and
    // the missing months are NAMED so the funnel says what to enqueue.
    let current_year = (1970 + now / 31_557_600).to_string();
    let mut monthly: HashMap<String, Vec<(String, i64)>> = HashMap::new();
    for (source, period, rows) in db.monthly_fetch_periods().await? {
        monthly.entry(source).or_default().push((period, rows));
    }
    // Issue 401: one index extremum, read once for the whole funnel rather than
    // per row — both rate feeds load the same table.
    let rates_through = db.newest_rate_date().await?;
    let pipeline: Vec<PipelineStage> = db
        .fetch_registry_summary()
        .await?
        .into_iter()
        .map(|row| {
            let store::FetchRegistryRow { source, packages, from, to, reference_only } = row;
            let gaps = store::monthly_period_gaps(monthly.get(&source).map_or(&[][..], |v| v));
            PipelineStage {
                published: (source == GROUND_TRUTH_SOURCE).then_some(published_ted),
                fetched_packages: packages,
                reference_feed: reference_only,
                rates_through: reference_only.then(|| rates_through.clone()).flatten(),
                // A source with an unparsed monthly period cannot be called
                // complete either: the sequence test did not cover it, and
                // "we could not check" must never render as "we checked".
                //
                // Issue 401: a REFERENCE FEED is never "fetch complete", because
                // the question does not apply to it — its completeness is whether
                // today's rates arrived, which `rates_through` answers. `ecb`
                // passed the year test by accident (its period IS a civil date)
                // and `eurostat` failed it by accident (`1993-1998`), so both
                // verdicts were noise.
                fetch_complete: !reference_only
                    && to.starts_with(&current_year)
                    && gaps.missing.is_empty()
                    && gaps.unparsed.is_empty(),
                missing_periods: gaps.missing,
                duplicate_periods: gaps.duplicated,
                fetched_from: Some(from),
                fetched_to: Some(to),
                processed_notices: processed.get(&source).copied().unwrap_or(0),
                projected_tenders: projected.get(&source).copied().unwrap_or(0),
                source,
            }
        })
        .collect();

    Ok((coverage, pipeline))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Issue 395: the funnel must not call a source complete when its monthly
    /// sequence has a hole, even though the newest period is in the current year.
    ///
    /// That combination is not hypothetical — it is precisely what prod looked
    /// like: TED's 2025-06 package was never fetched, the range read
    /// `1993-01 … 2026-06`, and the dashboard showed `fetch complete ✓` over a
    /// ~72,000-notice gap. The old test was `to.starts_with(&current_year)`
    /// alone, which this fixture passes.
    ///
    /// Two sources in one registry, so the healthy arm is a CONTROL rather than a
    /// separate run: a contiguity check that fails everything is as useless as
    /// one that fails nothing.
    #[tokio::test]
    async fn an_interior_hole_denies_fetch_complete_even_when_the_newest_period_is_current() {
        let path = format!("/tmp/tender-db-funnel-gap-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = store::Db::open(&path).await.expect("open");

        // The clock the funnel reads, and the year it derives from it.
        let now = store::now_unix();
        let year = 1970 + now / 31_557_600;

        // `holed`: January and March of the current year, no February.
        // `whole`: January, February and March — the same shape, no hole.
        for (source, month) in [("holed", 1), ("holed", 3), ("whole", 1), ("whole", 2), ("whole", 3)]
        {
            db.record_fetch(&store::Fetch {
                source: source.to_owned(),
                kind: "monthly".to_owned(),
                period: format!("{year}-{month:02}"),
                url: "u".to_owned(),
                sha256: format!("{source}{month}"),
                bytes: 1,
                fetched_at: 0,
                path: "p".to_owned(),
            })
            .await
            .expect("register the package");
        }

        let (_, pipeline) = measure_coverage_pipeline(&db, now).await.expect("measure");
        let stage = |name: &str| {
            pipeline.iter().find(|s| s.source == name).unwrap_or_else(|| panic!("{name} missing"))
        };

        let holed = stage("holed");
        assert!(
            holed.fetched_to.as_deref().is_some_and(|to| to.starts_with(&year.to_string())),
            "the fixture's newest period IS in the current year — the old check passed here"
        );
        assert!(!holed.fetch_complete, "a hole in the middle is not 'fetch complete'");
        assert_eq!(
            holed.missing_periods,
            [format!("{year}-02")],
            "and the funnel names what to enqueue"
        );
        assert!(holed.duplicate_periods.is_empty());

        let whole = stage("whole");
        assert!(whole.fetch_complete, "the contiguous control must still read complete");
        assert!(whole.missing_periods.is_empty());

        let _ = std::fs::remove_file(&path);
    }

    /// Issue 401: a source whose packages are ALL rates downloads is a reference
    /// feed, and the funnel must not render notice counts for it.
    ///
    /// On prod the `ecb` row read `20 pkgs … fetch complete ✓ | 0 | 0` and
    /// `eurostat` read `1 pkgs (1993-1998 … 1993-1998) | 0 | 0`, while
    /// `currency_rates` held 277,445 rows over 54 currencies continuous from
    /// 1993-01-04 to 2026-09-14. Two of five rows permanently showed the shape of
    /// a stalled import, which is what teaches a reader to ignore zeros in that
    /// column.
    ///
    /// The classification is by fetch KIND, never by source name, and this test
    /// says so by using names that are nothing like `ecb` or `eurostat`: a match
    /// arm on the names would pass a test written with the names in it, and then
    /// miss the next reference feed.
    #[tokio::test]
    async fn a_source_that_fetches_only_rates_is_never_counted_as_an_import_source() {
        let path = format!("/tmp/tender-db-funnel-rates-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = store::Db::open(&path).await.expect("open");
        let now = store::now_unix();
        let year = 1970 + now / 31_557_600;

        // `ratesonly` fetches nothing but rates, under two different rates kinds —
        // the real shape, where one feed uses `rates` and another `rates-ecu-h`.
        // `mixed` fetches a rates package AND a notice package, so it is an import
        // source that happens to also carry reference data: the `MIN(...)`
        // aggregate has to refuse it, or a single rates row would blank a real
        // source's counts.
        for (source, kind, period) in [
            ("ratesonly", "rates", format!("{year}-09-15")),
            ("ratesonly", "rates-ecu-h", "1993-1998".to_owned()),
            ("mixed", "rates", format!("{year}-09-15")),
            ("mixed", "monthly", format!("{year}-01")),
            ("notices", "monthly", format!("{year}-01")),
        ] {
            db.record_fetch(&store::Fetch {
                source: source.to_owned(),
                kind: kind.to_owned(),
                period,
                url: "u".to_owned(),
                sha256: format!("{source}{kind}"),
                bytes: 1,
                fetched_at: 0,
                path: "p".to_owned(),
            })
            .await
            .expect("register the package");
        }

        let (_, pipeline) = measure_coverage_pipeline(&db, now).await.expect("measure");
        let stage = |name: &str| {
            pipeline.iter().find(|s| s.source == name).unwrap_or_else(|| panic!("{name} missing"))
        };

        let rates = stage("ratesonly");
        assert!(rates.reference_feed, "every package is a rates download");
        assert!(
            !rates.fetch_complete,
            "a reference feed is never 'fetch complete' — its completeness is whether \
             today's rates arrived, which is a different question and a different cell"
        );

        let mixed = stage("mixed");
        assert!(
            !mixed.reference_feed,
            "ONE rates package does not make an import source a reference feed"
        );
        assert!(!stage("notices").reference_feed);

        // The invariant this issue is actually about, stated over the whole
        // pipeline rather than over the two names: no reference feed is rendered
        // with a notice count. The UI dashes those cells; here we pin that the
        // stage never claims a nonzero one, so a future renderer cannot regress by
        // reading the raw field.
        for s in &pipeline {
            if s.reference_feed {
                assert_eq!(s.processed_notices, 0, "{} is a reference feed", s.source);
                assert_eq!(s.projected_tenders, 0, "{} is a reference feed", s.source);
            }
        }

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn the_vendored_ground_truth_parses_and_covers_the_ted_era() {
        let truth = ground_truth();
        assert_eq!(truth.first().map(|p| p.year.as_str()), Some("1993"));
        assert_eq!(truth.len(), 2026 - 1993 + 1, "one row per year 1993–2026");
        // Spot-check the transcription against the research table. 1993–1999
        // carry distributed distinct-ND counts (issue 189), not the Office's
        // assigned-number counter the table originally quoted.
        assert_eq!(truth.iter().find(|p| p.year == "1993").map(|p| p.notices), Some(66_521));
        assert_eq!(truth.iter().find(|p| p.year == "1999").map(|p| p.notices), Some(162_861));
        assert_eq!(truth.iter().find(|p| p.year == "2011").map(|p| p.notices), Some(411_850));
        assert_eq!(truth.iter().find(|p| p.year == "2025").map(|p| p.notices), Some(871_149));
        let total: i64 = truth.iter().map(|p| p.notices).sum();
        assert_eq!(total, 13_201_520);
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
        let snapshot = Dashboard {
            system: Some(System {
                measured_at: 0,
                cursor: 4242,
                service_rev: "test".into(),
                lag: Lag::default(),
            }),
            ..Dashboard::default()
        };
        *cell().write().unwrap() = snapshot;
        assert_eq!(
            latest().system.unwrap().cursor,
            4242,
            "the request path returns the memoized snapshot",
        );
    }

    /// Issue 37: sections fill independently. A section whose measurement fails
    /// keeps its last good value (stale-while-revalidate) and an unmeasured one
    /// stays `None` — the UI's "measuring since boot…" — so a slow or poisoned
    /// scan never blanks or, worse, zeroes the panels beside it, while a healthy
    /// section still updates in the same pass.
    #[test]
    fn a_failing_section_never_blanks_or_zeroes_the_others() {
        let cell = RwLock::new(Dashboard::default());
        // First pass: two sections land.
        publish::<_, String>(&cell, "counts", Ok(vec![Count { label: "tenders".into(), value: 5 }]), |d, v| d.counts = Some(v));
        publish::<_, String>(&cell, "coverage", Ok(vec![]), |d, v| d.coverage = Some(v));
        // Second pass: coverage's scan fails; counts refreshes fine.
        publish::<_, String>(&cell, "coverage", Err("scan timed out".to_owned()), |d, v| d.coverage = Some(v));
        publish::<_, String>(&cell, "counts", Ok(vec![Count { label: "tenders".into(), value: 6 }]), |d, v| d.counts = Some(v));

        let d = cell.read().unwrap();
        assert_eq!(d.counts.as_ref().unwrap()[0].value, 6, "a healthy section still updates");
        assert_eq!(d.coverage.as_ref().unwrap().len(), 0, "the poisoned section keeps its last good value");
        assert!(d.system.is_none(), "an unmeasured section stays 'measuring…', never zeroed");
    }

    /// Issue 37: one refresh pass fills every section, so nothing is stuck at
    /// "measuring…" once a healthy pass completes. Run against an empty store —
    /// every section measures to an empty-but-present value, never an error.
    #[tokio::test]
    async fn refresh_fills_every_section() {
        let path = format!("/tmp/tender-db-refresh-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        let cell = RwLock::new(Dashboard::default());
        refresh_into(&db, &cell, false, 0, &mut None).await;
        let d = cell.read().unwrap();
        assert!(d.system.is_some(), "system");
        assert!(d.counts.is_some(), "counts");
        assert!(d.quarantine.is_some(), "quarantine");
        assert!(d.award_linkage.is_some(), "award_linkage");
        assert!(d.coverage.is_some(), "coverage");
        assert!(d.pipeline.is_some(), "pipeline");
        let _ = std::fs::remove_file(&path);
    }

    /// Issue 53: while a write-heavy job holds the WAL, the refresher must NOT run
    /// its table-proportional scans — `coverage` (a full `notices` GROUP BY) and
    /// `counts` (a full `COUNT(*)` per canonical table, heavy once the projection
    /// grows them). A live reader snapshot held across such a scan pins the WAL
    /// and blocks the package/batch TRUNCATE (store::checkpoint proves a pinned
    /// snapshot defeats reclaim), which grew the log to 70 GB. The cheap indexed
    /// sections still refresh, and a previously measured value is kept
    /// (stale-while-revalidate), never blanked.
    #[tokio::test]
    async fn a_heavy_write_job_skips_the_scanning_sections_but_keeps_the_indexed_ones() {
        let path = format!("/tmp/tender-db-refresh-heavy-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();

        // A prior idle pass measured coverage.
        let cell = RwLock::new(Dashboard::default());
        refresh_into(&db, &cell, false, 0, &mut None).await;
        assert!(cell.read().unwrap().coverage.is_some(), "an idle pass measures coverage");

        // Now a write-heavy job is active: only the cheap `system` point-read
        // refreshes; EVERY table-proportional scan (quarantine, award-linkage,
        // counts, coverage) is skipped and its last value preserved — no new
        // WAL-pinning reader snapshot is opened. (Each guard is scoped so it is
        // never held across the next await.)
        refresh_into(&db, &cell, true, 0, &mut None).await;
        {
            let d = cell.read().unwrap();
            assert!(d.system.is_some(), "system (cheap point read) still refreshes while a job runs");
            // All four scanning sections keep their last value (stale-while-revalidate).
            assert!(d.quarantine.is_some(), "quarantine keeps its last value (gated)");
            assert!(d.award_linkage.is_some(), "award-linkage keeps its last value (gated)");
            assert!(d.counts.is_some(), "counts keeps its last value (gated)");
            assert!(d.coverage.is_some(), "the last coverage grid is kept, not blanked");
        }

        // And on a fresh snapshot (nothing measured yet) a heavy pass runs ONLY
        // `system`, leaving every scanning section unmeasured — none opens a
        // WAL-pinning snapshot. This is the airtight property: during ingestion
        // the refresher holds no long-lived reader at all (issue 53).
        let fresh = RwLock::new(Dashboard::default());
        refresh_into(&db, &fresh, true, 0, &mut None).await;
        {
            let f = fresh.read().unwrap();
            assert!(f.system.is_some(), "the cheap point-read section lands");
            assert!(f.quarantine.is_none(), "the quarantine bucket scans never ran while a job is active");
            assert!(f.award_linkage.is_none(), "award-linkage never ran while a job is active");
            assert!(f.counts.is_none(), "the heavy counts scan never ran while a job is active");
            assert!(f.coverage.is_none(), "the heavy coverage scan never ran while a job is active");
        }

        let _ = std::fs::remove_file(&path);
    }

    /// Issue 61 coverage change-gate — BOTH directions, so we neither freeze
    /// coverage (always-skip) nor leave it always-scanning:
    ///  * UNCHANGED DB → the heavy scan is SKIPPED (last values kept), so an idle
    ///    box never re-scans ~25GB every 60s.
    ///  * A WRITE that advances the watermark (here a new fetch) → the heavy scan
    ///    RE-RUNS, so coverage stays accurate the moment data changes.
    /// Observed by poisoning `counts`: it survives the skip, and is overwritten by
    /// the re-measure. `system` refreshes every pass regardless.
    #[tokio::test]
    async fn the_change_gate_skips_when_unchanged_and_runs_after_a_write() {
        let path = format!("/tmp/tender-db-refresh-gate-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        let cell = RwLock::new(Dashboard::default());
        let mut key = None;

        // First pass measures the heavy sections and records the watermark.
        refresh_into(&db, &cell, false, 0, &mut key).await;
        assert!(key.is_some(), "the first pass records the heavy-measure watermark");
        assert!(cell.read().unwrap().counts.is_some(), "the first pass measures counts");

        // (A) UNCHANGED → SKIP. Poison `counts`; an unchanged pass must leave it.
        cell.write().unwrap().counts = Some(vec![Count { label: "SENTINEL".into(), value: -1 }]);
        refresh_into(&db, &cell, false, 0, &mut key).await;
        assert_eq!(
            cell.read().unwrap().counts.as_ref().unwrap()[0].label,
            "SENTINEL",
            "an unchanged DB skips the heavy re-scan (the poisoned value survives)"
        );

        // (B) A WRITE advances the watermark (newest fetch instant) → RE-MEASURE.
        db.record_fetch(&store::Fetch {
            source: "ted".into(),
            kind: "daily".into(),
            period: "2026-07".into(),
            url: "u".into(),
            sha256: "h".into(),
            bytes: 1,
            fetched_at: 999_999,
            path: "p".into(),
        })
        .await
        .unwrap();
        refresh_into(&db, &cell, false, 0, &mut key).await;
        let d = cell.read().unwrap();
        assert_ne!(
            d.counts.as_ref().unwrap().first().map(|c| c.label.as_str()),
            Some("SENTINEL"),
            "a write that advances the watermark forces a re-measure (poison overwritten)"
        );
        assert!(d.system.is_some(), "system refreshes every pass regardless of the gate");
        drop(d);

        // (C) A CONCLUDED JOB forces a re-measure even when nothing else moved
        // (issue 191): a reprocess stamps quarantine rows in place — no cursor
        // movement, no new fetch or notice — and the panel must still refresh.
        cell.write().unwrap().counts = Some(vec![Count { label: "SENTINEL".into(), value: -1 }]);
        refresh_into(&db, &cell, false, 0, &mut key).await;
        assert_eq!(
            cell.read().unwrap().counts.as_ref().unwrap()[0].label,
            "SENTINEL",
            "still-unchanged DB and job count: the gate skips"
        );
        refresh_into(&db, &cell, false, 1, &mut key).await;
        assert_ne!(
            cell.read().unwrap().counts.as_ref().unwrap().first().map(|c| c.label.as_str()),
            Some("SENTINEL"),
            "a concluded job alone forces the heavy re-measure (issue 191)"
        );

        let _ = std::fs::remove_file(&path);
    }
}
