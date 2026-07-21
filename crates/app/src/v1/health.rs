//! Deep operational health (issue 24).
//!
//! [`super::health`] stays the fast liveness probe `deploy.sh` greps for
//! `ok:true` — process up, database answers, nothing more, so a deploy is never
//! failed by a stale-ingest or full-disk signal that has nothing to do with the
//! new build being live. `/health/deep` is what an *external* pinger watches: it
//! folds that liveness check together with the operational signals that tell an
//! unattended operator production has quietly broken — the daily ingest stopped
//! landing, the last job errored, or the disk is filling. One external check on
//! this one URL therefore covers uptime, freshness, job failures and disk, with
//! nobody reading a dashboard.
//!
//! The endpoint answers `200` when every check passes and `503` when any fails,
//! so a plain hosted pinger can judge it by HTTP status alone; the JSON body
//! names which check tripped for whoever reads the alert.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use model::ingestion::JobRun;
use serde_json::{Value, json};
use store::read;

use super::{AppState, rev};

/// Ingest is stale when no job has succeeded in this long. TED publishes its
/// daily package by 09:30 CET on weekdays and the DÖE + projection legs of the
/// scheduler run every day, so a healthy box records a successful run at least
/// daily; 26h gives the 09:35 run its window plus slack before we alarm, and
/// carries a Friday success across the weekend without a false alert.
const INGEST_STALE_SECS: i64 = 26 * 3_600;

/// Disk is unhealthy once this fraction of the DB volume is in use. The parsed
/// DB and raw archive share the 500 GB Hetzner volume; crossing 90% is the cue
/// to grow it before a write fails mid-ingest.
const DISK_FULL_FRACTION: f64 = 0.90;

/// How far back to scan the job log for the newest success. A failure run longer
/// than this would bury the last success — but that is itself caught by the
/// last-job check, which flips unhealthy the moment the newest run errors.
const JOB_SCAN: i64 = 100;

/// The deep probe: liveness + ingest freshness + last-job outcome + disk, folded
/// into one `ok` the external pinger alerts on.
pub async fn deep(State(state): State<AppState>) -> Response {
    // 1. Liveness + DB — the same cursor read `/health` runs, through the reader
    //    pool so it never queues behind an ingest job holding the writer.
    let cursor = match state.readers.get().await {
        Ok(reader) => read::latest_cursor(&reader).await.ok(),
        Err(_) => None,
    };

    // 2. Ingest freshness and the last job's outcome, from the persisted log
    //    (also reader-pooled — see [`store::Db::recent_job_runs`], issue 20).
    let runs = state.db.recent_job_runs(JOB_SCAN).await.unwrap_or_default();
    let last_success = runs.iter().find(|r| r.outcome == "ok").map(|r| r.finished_at);
    let last_job = runs.into_iter().next();

    // 3. Disk on the volume holding the database file.
    let disk = disk_usage();

    let signals = Signals { now: store::now_unix(), cursor, last_success, last_job, disk };
    let (ok, checks) = assess(&signals);

    let body = json!({ "ok": ok, "rev": rev(), "checks": checks });
    let status = if ok { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE };
    (status, Json(body)).into_response()
}

/// The raw inputs the verdict is computed from — gathered by [`deep`] (all the
/// I/O), judged by [`assess`] (pure, so the thresholds are unit-tested without a
/// database or a clock).
struct Signals {
    now: i64,
    /// The latest change cursor, or `None` if the database did not answer.
    cursor: Option<i64>,
    /// When the newest successful run finished, or `None` if none ever has.
    last_success: Option<i64>,
    /// The newest finished run, whatever its outcome — `None` on a fresh box.
    last_job: Option<JobRun>,
    /// Disk stats for the DB volume, or `None` if they could not be measured.
    disk: Option<Disk>,
}

struct Disk {
    used_fraction: f64,
    free_bytes: u64,
    total_bytes: u64,
}

/// Turn the raw signals into an overall verdict plus the per-check JSON. A
/// missing signal never alarms: a fresh box with no runs yet, or a filesystem
/// whose stats could not be read, is reported healthy-but-unmeasured rather than
/// paging the operator over its own absence.
fn assess(s: &Signals) -> (bool, Value) {
    let db_ok = s.cursor.is_some();

    let age = s.last_success.map(|t| s.now - t);
    let fresh_ok = age.is_none_or(|age| age <= INGEST_STALE_SECS);

    let last_ok = s.last_job.as_ref().is_none_or(|r| r.outcome != "error");

    let disk_ok = s.disk.as_ref().is_none_or(|d| d.used_fraction <= DISK_FULL_FRACTION);

    let ok = db_ok && fresh_ok && last_ok && disk_ok;

    let checks = json!({
        "database": { "ok": db_ok, "cursor": s.cursor.map(|c| c.to_string()) },
        "ingest_freshness": {
            "ok": fresh_ok,
            "last_success_at": s.last_success,
            "age_secs": age,
            "threshold_secs": INGEST_STALE_SECS,
        },
        "last_job": match &s.last_job {
            Some(r) => json!({
                "ok": last_ok,
                "kind": r.kind,
                "params": r.params,
                "outcome": r.outcome,
                "finished_at": r.finished_at,
            }),
            None => json!({ "ok": true, "outcome": Value::Null }),
        },
        "disk": match &s.disk {
            Some(d) => json!({
                "ok": disk_ok,
                // Rounded to a tenth of a percent — the raw f64 is noise.
                "used_fraction": (d.used_fraction * 1000.0).round() / 1000.0,
                "free_bytes": d.free_bytes,
                "total_bytes": d.total_bytes,
                "threshold_fraction": DISK_FULL_FRACTION,
            }),
            None => json!({ "ok": true, "measured": false }),
        },
    });
    (ok, checks)
}

/// Usage of the filesystem holding the database file (`TENDER_DB`, same volume
/// as the archive in production). `None` if the path cannot be stat'd — a
/// portability quirk must not masquerade as a full disk.
fn disk_usage() -> Option<Disk> {
    let db = std::env::var("TENDER_DB").unwrap_or_else(|_| "tender-db.db".into());
    let path = std::path::Path::new(&db);
    // statvfs needs an existing path; fall back to the DB's directory when the
    // file itself is not there yet (fresh box, before the first open).
    let dir = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(std::path::Path::new("."));
    let stats = fs4::statvfs(path).or_else(|_| fs4::statvfs(dir)).ok()?;
    let total = stats.total_space();
    let available = stats.available_space();
    // df's Use% counts reserved blocks as used; matching it errs toward alarming
    // slightly early, which is the right bias for a "grow the disk" cue.
    let used_fraction = if total > 0 { (total - available) as f64 / total as f64 } else { 0.0 };
    Some(Disk { used_fraction, free_bytes: available, total_bytes: total })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(outcome: &str, finished_at: i64) -> JobRun {
        JobRun {
            id: 1,
            kind: "process".into(),
            params: "ted daily (all)".into(),
            started_at: finished_at - 10,
            finished_at,
            outcome: outcome.into(),
            counts: "42 notices".into(),
        }
    }

    fn healthy() -> Signals {
        Signals {
            now: 1_000_000,
            cursor: Some(7),
            last_success: Some(1_000_000 - 3_600),
            last_job: Some(run("ok", 1_000_000 - 3_600)),
            disk: Some(Disk { used_fraction: 0.42, free_bytes: 100, total_bytes: 200 }),
        }
    }

    #[test]
    fn all_green_is_ok() {
        let (ok, checks) = assess(&healthy());
        assert!(ok);
        assert_eq!(checks["database"]["ok"], true);
        assert_eq!(checks["ingest_freshness"]["ok"], true);
        assert_eq!(checks["last_job"]["ok"], true);
        assert_eq!(checks["disk"]["ok"], true);
    }

    #[test]
    fn a_database_that_does_not_answer_is_unhealthy() {
        let (ok, checks) = assess(&Signals { cursor: None, ..healthy() });
        assert!(!ok);
        assert_eq!(checks["database"]["ok"], false);
    }

    #[test]
    fn ingest_older_than_the_window_flips_unhealthy() {
        // 27h since the last success — past the 26h threshold.
        let stale = Signals { last_success: Some(1_000_000 - 27 * 3_600), ..healthy() };
        let (ok, checks) = assess(&stale);
        assert!(!ok);
        assert_eq!(checks["ingest_freshness"]["ok"], false);

        // Exactly at the threshold is still healthy (boundary is inclusive).
        let edge = Signals { last_success: Some(1_000_000 - INGEST_STALE_SECS), ..healthy() };
        assert!(assess(&edge).0);
    }

    #[test]
    fn a_box_with_no_runs_yet_does_not_alarm() {
        let fresh = Signals { last_success: None, last_job: None, ..healthy() };
        let (ok, checks) = assess(&fresh);
        assert!(ok, "no scheduled run has fired yet — not a failure");
        assert_eq!(checks["ingest_freshness"]["last_success_at"], Value::Null);
        assert_eq!(checks["last_job"]["outcome"], Value::Null);
    }

    #[test]
    fn a_failed_last_job_is_surfaced() {
        let errored = Signals { last_job: Some(run("error", 1_000_000 - 60)), ..healthy() };
        let (ok, checks) = assess(&errored);
        assert!(!ok);
        assert_eq!(checks["last_job"]["ok"], false);
        assert_eq!(checks["last_job"]["outcome"], "error");
    }

    #[test]
    fn a_full_disk_is_unhealthy_but_an_unmeasured_one_is_not() {
        let full = Signals {
            disk: Some(Disk { used_fraction: 0.95, free_bytes: 10, total_bytes: 200 }),
            ..healthy()
        };
        assert!(!assess(&full).0);

        let unmeasured = Signals { disk: None, ..healthy() };
        let (ok, checks) = assess(&unmeasured);
        assert!(ok);
        assert_eq!(checks["disk"]["measured"], false);
    }
}
