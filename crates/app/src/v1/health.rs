//! Deep operational health (issue 24).
//!
//! [`super::health`] stays the fast liveness probe `deploy.sh` greps for
//! `ok:true` — process up and serving HTTP, nothing more (no DB access, issue 61),
//! so a deploy is never failed by a stale-ingest or full-disk signal that has
//! nothing to do with the new build being live. The database-answering check lives
//! HERE, not there. `/health/deep` is what an *external* pinger watches: it
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
use store::{LayerPresence, LayerState};
use serde_json::{Value, json};

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

/// A presence observation older than this is treated as no observation at all.
/// The observer is a supervisor job; if it stops, the stored verdicts freeze at
/// whatever they last said — and a frozen green is indistinguishable from a
/// real one. Reporting unhealthy on a stale observation is the honest answer:
/// we do not currently know whether the layer is intact. Sized well above the
/// observer's cadence so ordinary jitter never trips it.
const LAYER_STALE_SECS: i64 = 6 * 3_600;

/// How far back to scan the job log for the newest success. A failure run longer
/// than this would bury the last success — but that is itself caught by the
/// last-job check, which flips unhealthy the moment the newest run errors.
/// Shared with `/metrics`, whose per-kind last-run gauges read the same window.
pub(super) const JOB_SCAN: i64 = 100;

/// The job kinds whose success proves data is still ARRIVING. `ingest_freshness`
/// tracks the newest successful run among ONLY these, so a maintenance job that
/// happens to succeed — a `reindex`, `reprocess`, `refold` — cannot reset the clock
/// and report the box "fresh" while the daily ingest has actually stalled. Before
/// this, `last_success` took any-kind success, so a manual reindex marked ingestion
/// fresh even though nothing new was coming in.
///
/// `project` used to be in this list and had to come out. Every maintenance refold
/// enqueues a PAIRED `project` — `refold`, `refold-notices`, `refold-sections` and
/// `reprocess` all push one — so a busy maintenance day kept freshness green through
/// the pair, which is exactly the masking this constant exists to prevent. Found by
/// operating the box: after an hour of refolds the check read 20 minutes fresh while
/// nothing had been fetched from any source for 23 hours.
///
/// What is left says only "we asked a source for data, or processed a package, and it
/// worked". `enqueue_daily` pushes a DÖE `probe` + `process` every single day and a
/// TED pair on weekdays, so the heartbeat does not go quiet at weekends, and a
/// projection — which folds what is already stored — no longer speaks for the fetch.
const INGEST_KINDS: [&str; 2] = ["probe", "process"];

/// The deep probe: liveness + ingest freshness + last-job outcome + disk, folded
/// into one `ok` the external pinger alerts on.
pub async fn deep(State(state): State<AppState>) -> Response {
    // 1. Database answering — a REAL reader-pool read, not the in-memory cursor.
    //    `recent_job_runs` (step 2) is a reader-pool query (issue 20), so its
    //    success IS the "the database answered" signal, and an `Err` means the
    //    reader pool could not serve a read — the box is not ready. The reader pool
    //    serves over WAL and never queues behind the writer, so this honours issue
    //    61's "don't block behind a projection" rule while still being a genuine DB
    //    touch. The old `Some(current_cursor())` was an in-memory read that can
    //    never fail, which made this check vacuous (issue 213) — its `unhealthy`
    //    branch in `assess` was unreachable.
    let runs_result = state.db.recent_job_runs(JOB_SCAN).await;
    let db_answered = runs_result.is_ok();
    let runs = runs_result.unwrap_or_default();

    // The last-known cursor, reported only when the DB actually answered, so the
    // `database` verdict flips unhealthy (cursor `None`) exactly when the read failed.
    let cursor = db_answered.then(|| state.db.current_cursor());

    // 2. Ingest freshness and the last job's outcome, from that same reader-pooled
    //    log read (issue 20). Freshness counts only the daily-pipeline kinds
    //    (`INGEST_KINDS`), so a maintenance job cannot mask a stalled ingest; the
    //    last-job check below is any-kind on purpose (it reports the newest run).
    let last_success = ingest_last_success(&runs);
    let last_job = runs.into_iter().next();

    // 3. Disk on the volume holding the database file.
    let disk = disk_usage();

    // 4. The canonical layer's presence verdicts (issue 133). READ, never
    //    observe: observing takes the writer connection, and a probe that
    //    blocks behind a running projection would report the service unhealthy
    //    for being busy. The supervisor job does the observing.
    let layer = state.db.read_layer_presence().await.ok();

    let signals = Signals { now: store::now_unix(), cursor, last_success, last_job, disk, layer };
    let (ok, checks) = assess(&signals);

    let body = json!({ "ok": ok, "rev": rev(), "checks": checks });
    let status = if ok { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE };
    (status, Json(body)).into_response()
}

/// When the newest SUCCESSFUL daily-pipeline run finished — the freshness clock.
/// Filters to [`INGEST_KINDS`] so a maintenance job's success cannot reset it and
/// hide a stalled ingest. `runs` is newest-first, so the first match is the newest.
pub(super) fn ingest_last_success(runs: &[JobRun]) -> Option<i64> {
    runs.iter()
        .find(|r| r.outcome == "ok" && INGEST_KINDS.contains(&r.kind.as_str()))
        .map(|r| r.finished_at)
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
    /// Per-table presence verdicts with the time each was observed, or `None`
    /// if they could not be read.
    layer: Option<Vec<(LayerPresence, i64)>>,
}

pub(crate) struct Disk {
    pub(crate) used_fraction: f64,
    pub(crate) free_bytes: u64,
    pub(crate) total_bytes: u64,
    /// Size of the `-wal` sidecar, surfaced so the alerting routine sees a
    /// runaway WAL during bulk loads (issue 42). Informational — a large WAL is
    /// expected mid-backfill, so it does not by itself flip the disk verdict.
    pub(crate) wal_bytes: Option<u64>,
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

    // The canonical layer. Two separate ways to be unhealthy, and conflating
    // them would hide the worse one:
    //   * a table that HELD rows and is now empty — the wipe (issue 133);
    //   * an observation too old to mean anything — we do not know.
    // An empty verdict set is NOT a failure: a fresh box has never observed,
    // and alarming over its own absence is the false positive that gets a
    // check switched off (same rule as every other signal here).
    let emptied: Vec<&LayerPresence> = s
        .layer
        .iter()
        .flatten()
        .filter(|(p, _)| matches!(p.state, LayerState::WentEmpty { .. }))
        .map(|(p, _)| p)
        .collect();
    let oldest = s.layer.iter().flatten().map(|(_, at)| *at).min();
    let layer_stale = oldest.is_some_and(|at| s.now - at > LAYER_STALE_SECS);
    let layer_ok = emptied.is_empty() && !layer_stale;

    let ok = db_ok && fresh_ok && last_ok && disk_ok && layer_ok;

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
        "canonical_layer": {
            "ok": layer_ok,
            // Named, not counted: "3 tables emptied" sends someone hunting for
            // which, and the names are the whole diagnosis.
            "emptied": emptied.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
            "observed_at": oldest,
            "stale": layer_stale,
            "stale_threshold_secs": LAYER_STALE_SECS,
            "measured": s.layer.as_ref().is_some_and(|l| !l.is_empty()),
        },
        "disk": match &s.disk {
            Some(d) => json!({
                "ok": disk_ok,
                // Rounded to a tenth of a percent — the raw f64 is noise.
                "used_fraction": (d.used_fraction * 1000.0).round() / 1000.0,
                "free_bytes": d.free_bytes,
                "total_bytes": d.total_bytes,
                "wal_bytes": d.wal_bytes,
                "threshold_fraction": DISK_FULL_FRACTION,
            }),
            None => json!({ "ok": true, "measured": false }),
        },
    });
    (ok, checks)
}

/// Files this process holds open after they were unlinked (issue 361): space
/// `df` counts, no path shows, and only a process exit releases. Read off
/// `/proc/self/fd` — one directory of ~120 entries, bounded — so the weekly
/// census and `/metrics` see the class the 2026-09-06 restart released 191 GB
/// of, instead of an operator stumbling on it. `None` off Linux or when
/// `/proc` is unreadable.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct DeletedOpen {
    pub(crate) files: u64,
    pub(crate) bytes: u64,
    /// Up to five `(path, bytes)` pairs, largest first, for the report.
    pub(crate) sample: Vec<(String, u64)>,
}

pub(crate) fn deleted_open() -> Option<DeletedOpen> {
    let entries = std::fs::read_dir("/proc/self/fd").ok()?;
    let mut held: Vec<(String, u64)> = Vec::new();
    for entry in entries.flatten() {
        let Ok(target) = std::fs::read_link(entry.path()) else { continue };
        let target = target.to_string_lossy();
        let Some(path) = target.strip_suffix(" (deleted)") else { continue };
        // The descriptor still reaches the inode, so its size is readable
        // through the fd link even though the path is gone.
        let bytes = std::fs::metadata(entry.path()).map(|m| m.len()).unwrap_or(0);
        held.push((path.to_owned(), bytes));
    }
    held.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let files = held.len() as u64;
    let bytes = held.iter().map(|(_, b)| b).sum();
    held.truncate(5);
    Some(DeletedOpen { files, bytes, sample: held })
}

/// Usage of the filesystem holding the database file (`TENDER_DB`, same volume
/// as the archive in production). `None` if the path cannot be stat'd — a
/// portability quirk must not masquerade as a full disk.
pub(crate) fn disk_usage() -> Option<Disk> {
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
    // The WAL sidecar sits beside the DB file; its size is the issue-42 runaway
    // signal. Absent (freshly checkpointed) reads as no WAL, which is healthy.
    let wal_bytes = std::fs::metadata(format!("{db}-wal")).ok().map(|m| m.len());
    Some(Disk { used_fraction, free_bytes: available, total_bytes: total, wal_bytes })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Issue 361: an unlinked file this process still holds is counted with
    /// its size, and stops being counted once the descriptor closes — the
    /// only thing that releases the space, which is the point of reading it.
    #[test]
    fn an_unlinked_open_file_is_counted_until_its_descriptor_closes() {
        use std::io::Write;
        let dir = std::env::temp_dir();
        let path = dir.join(format!("tender-db-deleted-open-{}.bin", std::process::id()));
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(&vec![7u8; 1 << 20]).unwrap();
        file.sync_all().unwrap();
        std::fs::remove_file(&path).unwrap();
        let Some(held) = deleted_open() else {
            eprintln!("no /proc/self/fd here; nothing to pin");
            return;
        };
        let name = path.to_string_lossy().into_owned();
        let mine = held.sample.iter().find(|(p, _)| *p == name).expect("the unlinked file is listed");
        assert_eq!(mine.1, 1 << 20);
        assert!(held.files >= 1);
        assert!(held.bytes >= 1 << 20);
        drop(file);
        let after = deleted_open().unwrap();
        assert!(after.sample.iter().all(|(p, _)| *p != name), "closed, so no longer held");
        assert_eq!(after.files, held.files - 1);
    }

    fn run(outcome: &str, finished_at: i64) -> JobRun {
        JobRun {
            id: 1,
            job_id: None,
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
            disk: Some(Disk {
                used_fraction: 0.42,
                free_bytes: 100,
                total_bytes: 200,
                wal_bytes: Some(3_500_000_000),
            }),
            layer: Some(vec![(present("tenders", LayerState::Populated), 1_000_000 - 600)]),
        }
    }

    fn present(name: &str, state: LayerState) -> LayerPresence {
        LayerPresence { name: name.to_string(), state }
    }

    /// Ingest freshness must track the last DAILY-PIPELINE success, not any job —
    /// else a maintenance job (a manual `reindex`, as on 2026-08-16) resets the
    /// clock and reports the box fresh while the ingest has actually stalled.
    #[test]
    fn a_maintenance_success_does_not_reset_ingest_freshness() {
        let job = |kind: &str, outcome: &str, finished_at: i64| JobRun {
            id: 1,
            job_id: None,
            kind: kind.into(),
            params: String::new(),
            started_at: finished_at - 10,
            finished_at,
            outcome: outcome.into(),
            counts: String::new(),
        };
        // Newest-first: a reindex just succeeded, but the last real ingest was earlier.
        let runs = vec![
            job("reindex", "ok", 2_000),
            job("reprocess", "ok", 1_800),
            job("process", "ok", 1_000),
            job("probe", "ok", 900),
        ];
        assert_eq!(
            ingest_last_success(&runs),
            Some(1_000),
            "freshness is the last ingest run, not the newer maintenance one"
        );

        // Only maintenance in the window → None: unmeasured, never a false green.
        assert_eq!(
            ingest_last_success(&[job("reindex", "ok", 2_000), job("reprocess", "ok", 1_800)]),
            None
        );

        // A failed ingest does not count; the prior successful ingest does.
        assert_eq!(
            ingest_last_success(&[job("process", "error", 2_000), job("probe", "ok", 1_500)]),
            Some(1_500)
        );

        // The refold pair must NOT count. Every maintenance refold enqueues a
        // `project` alongside it, so counting `project` handed the masking back:
        // an hour of refolds reported freshness while nothing had been fetched for
        // a day. A projection folds what is already stored — it says nothing about
        // whether data is still arriving.
        assert_eq!(
            ingest_last_success(&[
                job("project", "ok", 2_000),
                job("refold", "ok", 1_990),
                job("probe", "ok", 1_000),
            ]),
            Some(1_000),
            "a refold's paired project is maintenance, not evidence of arrival"
        );
        assert_eq!(
            ingest_last_success(&[job("project", "ok", 2_000), job("refold", "ok", 1_990)]),
            None,
            "a window holding only the refold pair is unmeasured, never fresh"
        );
    }

    /// A table that held rows and is now empty must flip the probe unhealthy —
    /// this is the 2026-07-30 wipe, and the whole point of issue 133.
    #[test]
    fn an_emptied_canonical_table_makes_the_probe_unhealthy() {
        let mut s = healthy();
        s.layer = Some(vec![
            (present("tenders", LayerState::WentEmpty { at: 999_000 }), 1_000_000 - 600),
            (present("changes", LayerState::Populated), 1_000_000 - 600),
        ]);
        let (ok, checks) = assess(&s);
        assert!(!ok, "an emptied canonical layer must not report healthy");
        assert_eq!(checks["canonical_layer"]["emptied"][0], "tenders");
    }

    /// A never-populated table is NOT damage — a fresh box has an empty layer
    /// legitimately, and alarming there is the false positive that gets the
    /// check disabled.
    #[test]
    fn a_never_populated_layer_is_healthy() {
        let mut s = healthy();
        s.layer = Some(vec![(present("tenders", LayerState::NeverPopulated), 1_000_000 - 600)]);
        let (ok, _) = assess(&s);
        assert!(ok, "a fresh, never-projected layer must not alarm");
    }

    /// A stale observation must read unhealthy, NOT green. If the observer job
    /// dies the stored verdicts freeze, and a frozen green is indistinguishable
    /// from a real one — a detector that cannot tell you it stopped looking is
    /// worse than none, because it is trusted.
    #[test]
    fn a_stale_presence_observation_is_not_treated_as_green() {
        let mut s = healthy();
        s.layer = Some(vec![(
            present("tenders", LayerState::Populated),
            1_000_000 - LAYER_STALE_SECS - 1,
        )]);
        let (ok, checks) = assess(&s);
        assert!(!ok, "a presence observation older than the threshold must not read as healthy");
        assert_eq!(checks["canonical_layer"]["stale"], true);
    }

    /// But never having observed at all is not staleness — a box that has not
    /// run the observer yet reports healthy-but-unmeasured, like every other
    /// missing signal in this probe.
    #[test]
    fn no_presence_observations_yet_is_healthy_but_unmeasured() {
        let mut s = healthy();
        s.layer = Some(vec![]);
        let (ok, checks) = assess(&s);
        assert!(ok, "a box that has never observed must not alarm over its own absence");
        assert_eq!(checks["canonical_layer"]["measured"], false);
    }

    #[test]
    fn all_green_is_ok() {
        let (ok, checks) = assess(&healthy());
        assert!(ok);
        assert_eq!(checks["database"]["ok"], true);
        assert_eq!(checks["ingest_freshness"]["ok"], true);
        assert_eq!(checks["last_job"]["ok"], true);
        assert_eq!(checks["disk"]["ok"], true);
        // WAL size is surfaced for the alerting routine, but a large WAL alone
        // does not flip the verdict (issue 42) — it is expected mid-backfill.
        assert_eq!(checks["disk"]["wal_bytes"], 3_500_000_000_i64);
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
            disk: Some(Disk {
                used_fraction: 0.95,
                free_bytes: 10,
                total_bytes: 200,
                wal_bytes: Some(10),
            }),
            ..healthy()
        };
        assert!(!assess(&full).0);

        let unmeasured = Signals { disk: None, ..healthy() };
        let (ok, checks) = assess(&unmeasured);
        assert!(ok);
        assert_eq!(checks["disk"]["measured"], false);
    }
}
