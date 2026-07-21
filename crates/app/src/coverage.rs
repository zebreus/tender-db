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
    tokio::spawn(async move {
        loop {
            refresh_into(&db, cell()).await;
            tokio::time::sleep(REFRESH).await;
        }
    });
}

/// One refresh pass: measure each section and publish it the moment it is ready,
/// cheapest first. Each section is independent — a section that errors keeps its
/// last good value (stale-while-revalidate) rather than blanking, and a slow or
/// poisoned section (e.g. the coverage scan under ingestion load) never delays
/// the sections ahead of it. `system` lands within a second of a restart; the
/// full-table-scan sections land as each completes.
async fn refresh_into(db: &Db, cell: &RwLock<Dashboard>) {
    let now = store::now_unix();
    publish(cell, "system", measure_system(db, now).await, |d, v| d.system = Some(v));
    // Quarantine next: it is the panel a boot must never show as a false `0`
    // (actionable = 0 reads as "the guarantee holds"), and its reason-keyed
    // queries are indexed (issue 37), so it lands early and cheap.
    publish(cell, "quarantine", measure_quarantine(db).await, |d, v| d.quarantine = Some(v));
    publish(cell, "counts", measure_counts(db).await, |d, v| d.counts = Some(v));
    publish(cell, "award-linkage", measure_award_linkage(db).await, |d, v| d.award_linkage = Some(v));
    // Coverage grid and import funnel share the one notices scan (the heaviest
    // read), so they are measured together and land last.
    publish(cell, "coverage", measure_coverage_pipeline(db, now).await, |d, (coverage, pipeline)| {
        d.coverage = Some(coverage);
        d.pipeline = Some(pipeline);
    });
}

/// Measure every section once and return the assembled snapshot — the one-shot
/// form (tests, a future "measure now"). The running server never calls this: it
/// uses the background refresher ([`init`]) and serves [`latest`].
pub async fn measure(db: &Db) -> Dashboard {
    let cell = RwLock::new(Dashboard::default());
    refresh_into(db, &cell).await;
    cell.into_inner().expect("snapshot")
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
        snapshot_age: db.last_snapshot_at().await?.map(|at| now - at),
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
    let by_reason: Vec<Count> = db
        .quarantine_counts_by_reason()
        .await?
        .into_iter()
        .map(|(label, value)| Count { label, value })
        .collect();
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
    Ok(Quarantine {
        total: by_reason.iter().map(|c| c.value).sum(),
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

    // The import pipeline per source: fetch registry + the notice counts already
    // gathered + projected tenders, so the operator sees which stage the backfill
    // is in without ssh.
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

    Ok((coverage, pipeline))
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
        let snapshot = Dashboard {
            system: Some(System {
                measured_at: 0,
                cursor: 4242,
                service_rev: "test".into(),
                snapshot_age: None,
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
        refresh_into(&db, &cell).await;
        let d = cell.read().unwrap();
        assert!(d.system.is_some(), "system");
        assert!(d.counts.is_some(), "counts");
        assert!(d.quarantine.is_some(), "quarantine");
        assert!(d.award_linkage.is_some(), "award_linkage");
        assert!(d.coverage.is_some(), "coverage");
        assert!(d.pipeline.is_some(), "pipeline");
        let _ = std::fs::remove_file(&path);
    }
}
