//! The dashboard's measurement half: one read of the store turned into the
//! numbers `/` renders.
//!
//! Coverage is the interesting one. "How complete is this database?" needs a
//! denominator, and the only honest one is what the source is *known* to have
//! published — so the per-year TED notice counts established in
//! docs/research/ted-access-channels.md §6 are vendored beside this file and
//! compared against what we hold. Before any backfill the ratios are near zero,
//! which is the correct answer, not a bug to hide.

use model::dashboard::{AwardLinkage, Count, Coverage, Dashboard, Lag, Quarantined};
use store::Db;

/// Notice counts per TED publication year — the coverage denominator. Vendored
/// as data (not code) so refreshing it is an edit to a table, not a patch.
const GROUND_TRUTH: &str = include_str!("../data/ted-notice-counts.csv");

/// The source the ground-truth table describes. Other sources show holdings
/// with no ratio until their own volumes are measured.
const GROUND_TRUTH_SOURCE: &str = "ted";

/// How many quarantined payloads the drill-down lists.
const QUARANTINE_SAMPLE: i64 = 50;

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

/// Measure everything the dashboard shows, in one pass, so the panels are one
/// consistent snapshot rather than four independently-timed ones.
pub async fn measure(db: &Db, now: i64) -> store::turso::Result<Dashboard> {
    let truth = ground_truth();
    let coverage = db
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
        quarantine_total: quarantine_by_reason.iter().map(|c| c.value).sum(),
        quarantine_by_reason,
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
}
