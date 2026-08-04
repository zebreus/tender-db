//! The in-app ingestion Supervisor (issue 16, ADR-0005 made real).
//!
//! Ingestion runs *inside* the server process: no external process ever opens
//! the production DB (turso is single-process), and readers keep serving over
//! WAL while a job writes, so a load has zero downtime. The Supervisor is a
//! background tokio task owning a small job queue executed **one job at a time**
//! (the store has a single writer anyway), plus a scheduler that enqueues the
//! daily TED/DÖE work.
//!
//! Jobs come from two places: the `/admin` API ([`crate::admin`]) and the
//! [`Scheduler`]. Live progress is a shared [`JobProgress`]; finished runs are
//! persisted to the store's `job_log`. The whole thing is a global singleton
//! ([`init`]/[`get`]) so both the admin router and the dashboard's server
//! function reach the same instance.
//!
//! It reuses ingest's **library** functions (`fetch`, `process`, `project`) —
//! never the CLIs, which are dev tools for scratch databases only.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use crate::snapshot;
use ingest::{doe, fetch, process, project, ted};
use model::ingestion::{Ingestion, JobProgress, QueuedJob};
use serde::{Deserialize, Serialize};
use store::turso;
use tokio::sync::{Notify, OnceCell};

/// How many recent runs the dashboard/admin log shows.
const RECENT_RUNS: i64 = 20;

/// Worker threads on the isolated job runtime (issue 61). A job's heavy body —
/// projection/process/fetch/snapshot — runs here, never on the main
/// API/SSE/dashboard runtime, so turso's *blocking* preads pin these threads and
/// leave the API responsive. Jobs still run one at a time (the store has a single
/// writer), so this is for isolation, not parallelism — a couple of threads,
/// mirroring the SQL runtime (`crate::v1::sql`).
const WORKER_RUNTIME_THREADS: usize = 2;

/// A tokio runtime dedicated to running the Supervisor's jobs, owned by a parked
/// thread so it lives for the whole process and is never dropped in an async
/// context (which tokio panics on). The exact pattern issue 17 proved for
/// `/v1/sql` (`crate::v1::sql::spawn_sql_runtime`).
///
/// One is created per [`Supervisor`] — once per server in production. The `Db`
/// writer (`tokio::sync::Mutex<Connection>`) and reader pool are tokio async
/// primitives, safe to use from this runtime and the main one alike, so a job's
/// blocking reads land here while the API keeps reading on the main runtime.
fn spawn_worker_runtime() -> tokio::runtime::Handle {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("job-runtime".into())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(WORKER_RUNTIME_THREADS)
                .thread_name("job-exec")
                .enable_all()
                .build()
                .expect("build the isolated job runtime");
            tx.send(runtime.handle().clone()).expect("hand back the runtime handle");
            // Park the owner thread on a future that never completes, so the
            // runtime stays alive without this thread busy-waiting.
            runtime.block_on(std::future::pending::<()>());
        })
        .expect("spawn the job runtime thread");
    rx.recv().expect("receive the job runtime handle")
}

/// The process-wide Supervisor. Set once at server startup.
static SUPERVISOR: OnceCell<Arc<Supervisor>> = OnceCell::const_new();

/// Start the Supervisor over the process database and spawn its worker +
/// scheduler. Idempotent: a second call (dev hot-reload re-runs the server
/// initializer) returns the already-running instance without spawning again.
///
/// Recovery runs *before* the worker or scheduler start, so the durable queue is
/// rebuilt before any job is popped or any tick fires (issue 21).
pub async fn init(db: Arc<store::Db>) -> Arc<Supervisor> {
    SUPERVISOR
        .get_or_init(|| async {
            let archive: PathBuf =
                std::env::var("TENDER_ARCHIVE").unwrap_or_else(|_| "archive".into()).into();
            let sup = Arc::new(Supervisor::new(db, archive, reqwest::Client::new()));
            sup.recover().await;
            sup.clone().spawn_worker();
            sup.clone().spawn_scheduler();
            sup
        })
        .await
        .clone()
}

/// The running Supervisor, if [`init`] has run — the dashboard server function
/// uses this. `None` before startup (or in a unit test that never called init).
pub fn get() -> Option<Arc<Supervisor>> {
    SUPERVISOR.get().cloned()
}

pub struct Supervisor {
    db: Arc<store::Db>,
    archive: PathBuf,
    http: reqwest::Client,
    ted_base: String,
    doe_base: String,
    queue: Mutex<VecDeque<Job>>,
    wake: Notify,
    next_id: AtomicU64,
    current: RwLock<Option<JobProgress>>,
    /// The isolated runtime a job's heavy body runs on (issue 61), kept off the
    /// main API/SSE/dashboard runtime so blocking turso preads never starve it.
    worker_runtime: tokio::runtime::Handle,
}

/// A queued unit of work: a display identity plus what to do.
#[derive(Clone)]
struct Job {
    id: u64,
    kind: String,
    params: String,
    spec: Spec,
    /// The resume cursor for a process job (issue 32): the last package a prior
    /// run fully completed, restored from the durable row on recovery. `None` for
    /// a freshly enqueued job, so a fresh enqueue always re-walks from the start.
    resume_after: Option<String>,
}

/// What a job does. Fetch/process/project map onto ingest's library entry
/// points; `ProbeTed`/`ProbeDoe` are the realtime daily walk-forwards (TED probes
/// the server for the next issue, DÖE walks calendar days from its last
/// watermark). `Serialize`/`Deserialize` so a job survives a restart in the
/// durable queue (issue 21).
#[derive(Clone, Serialize, Deserialize)]
enum Spec {
    Fetch { source: String, package_kind: String, period: String, refetch: bool },
    ProbeTed { refetch: bool },
    /// Walk DÖE daily exports forward from the last fetched day (issue 69), so a
    /// missed scheduler tick catches up instead of leaving a permanent hole.
    ProbeDoe,
    Process { source: String, package_kind: String, period: Option<String> },
    /// Re-attempt a held quarantine bucket from the archive, writing the parsed
    /// layer in place for members that now parse (issues 71/72/73). The bucket is
    /// `reason` + optional `detail LIKE` + optional exact `profile`.
    Reprocess { reason: String, detail_like: Option<String>, profile: Option<String> },
    /// Re-derive the canonical layer. `clear_changes` (rebuild only, issue 81)
    /// DROP+recreates the CDC feed first, so the rebuild re-emits ONE clean
    /// generation for the recovered baseline instead of appending. `#[serde(default)]`
    /// keeps pre-flag durable job rows deserializable.
    Project {
        rebuild: bool,
        #[serde(default)]
        clear_changes: bool,
    },
    /// A consistent online snapshot of the store, shipped off-box (issue 23).
    /// A unit variant, so it serialises into the durable job_queue as `"Snapshot"`
    /// and survives a restart like any other job.
    Snapshot,
    /// Rebuild any missing DEFERRED_TENDER_INDEXES / org indexes on the existing
    /// layer WITHOUT re-folding (issues 82/83): `build_tender_indexes` only runs at a
    /// rebuild's end, and a tmpfs-truncated run can leave the tender indexes partial,
    /// dropping the tenders list to a full scan. Idempotent (`CREATE INDEX IF NOT
    /// EXISTS` loops), so it builds only what's missing. A unit variant → durable.
    Reindex,
    /// Re-queue a profile cohort for the incremental fold (issue 85): clear its
    /// `projected` watermark so the trailing `project rebuild=false` re-derives just
    /// those Tenders. For a cohort the projection MIS-READ (unmapped field ids) —
    /// the parsed layer is already correct, so no re-parse. `expect` is a guard: the
    /// run aborts BEFORE writing if the cohort is not about that size, which is what
    /// a mistyped profile string looks like.
    Refold { profiles: Vec<String>, expect: Option<u64> },
    /// Clear a STALE `rebuild_in_progress` flag (the issue-85 interlock's escape
    /// hatch). The flag routes any `project` — including the 09:35 daily tick — into
    /// the salvage branch, which `reset_tender_layer()`s a good layer; a rebuild that
    /// completed cleanly clears it itself, so a flag left set over an intact layer is
    /// stale by definition. [`Db::clear_plan`] clears it and retires the plan in one
    /// transaction. Fire ONLY with the layer verified intact; jobs run sequentially,
    /// so this cannot race a projection. Reports whether it actually cleared anything.
    ClearRebuildFlag,
}

/// The `POST /admin/jobs` body. `kind` selects the operation; the rest are its
/// parameters. Curl-friendly and forgiving (`serde(default)` everywhere).
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct JobRequest {
    /// `fetch` | `process` | `project` | `backfill` | `snapshot` | `daily`
    /// (issue 69 catch-up) | `reprocess` (re-attempt a held quarantine bucket).
    pub kind: String,
    /// `ted` | `doe`.
    pub source: Option<String>,
    /// `daily` | `monthly` (default `daily`).
    pub package_kind: Option<String>,
    /// A single period (`2026-00136` daily, `2026-06` monthly, `2026-07-18` DÖE day).
    pub period: Option<String>,
    /// Backfill range, inclusive: `["2024-01", "2024-12"]` monthly periods.
    pub range: Option<[String; 2]>,
    /// `project` only: drop and re-derive the whole canonical layer.
    pub rebuild: Option<bool>,
    /// `fetch` only: re-download and hash-compare a known period (finality).
    pub refetch: Option<bool>,
    /// `reprocess` only: the held quarantine bucket to re-attempt — `reason` is
    /// required, `detail_like` (a SQL LIKE pattern, e.g. `%: OC`) and `profile`
    /// narrow it further.
    pub reason: Option<String>,
    pub detail_like: Option<String>,
    pub profile: Option<String>,
    /// `reprocess` only: skip the trailing incremental fold, leaving reclaimed
    /// notices `projected=0` for one later `rebuild:true` to fold in bulk.
    pub reclaim_only: Option<bool>,
    /// `project` + `rebuild` only: DROP+recreate the CDC feed before folding, so the
    /// rebuild re-emits ONE clean generation for the recovered baseline (issue 81).
    pub clear_changes: Option<bool>,
    /// `refold` only: the notice profiles whose cohort to re-project, and the size it
    /// is expected to be. `expect` aborts the run before any write if the cohort is
    /// off by more than a quarter — the shape of a mistyped profile string.
    pub profiles: Option<Vec<String>>,
    pub expect: Option<u64>,
}

impl Supervisor {
    pub fn new(db: Arc<store::Db>, archive: PathBuf, http: reqwest::Client) -> Supervisor {
        Supervisor {
            db,
            archive,
            http,
            ted_base: ted::BASE.to_owned(),
            doe_base: doe::BASE.to_owned(),
            queue: Mutex::new(VecDeque::new()),
            wake: Notify::new(),
            next_id: AtomicU64::new(1),
            current: RwLock::new(None),
            worker_runtime: spawn_worker_runtime(),
        }
    }

    // ---------------------------------------------------------------- queueing

    async fn push(&self, kind: &'static str, params: String, spec: Spec) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        // Persist before enqueuing in memory: the durable row is what a restart
        // rebuilds the queue from, so it must exist first (issue 21). Best-effort
        // like the run log — a failed persist still runs this session, it just
        // won't survive a restart.
        let spec_json = serde_json::to_string(&spec).expect("job spec serializes");
        if let Err(e) = self.db.enqueue_job(id as i64, kind, &params, &spec_json).await {
            eprintln!("supervisor: persist queued job {id}: {e}");
        }
        self.queue.lock().expect("queue lock").push_back(Job {
            id,
            kind: kind.to_owned(),
            params,
            spec,
            resume_after: None,
        });
        self.wake.notify_one();
        id
    }

    /// Turn one admin request into one or more queued jobs, returning their ids.
    /// A backfill fans a period range out into individual fetch jobs plus a
    /// trailing process+project, so progress and cancellation stay per-package.
    pub async fn enqueue_request(&self, req: &JobRequest) -> Result<Vec<u64>, String> {
        match req.kind.as_str() {
            "fetch" => {
                let (source, package_kind, period) = self.fetch_parts(req)?;
                let refetch = req.refetch.unwrap_or(false);
                Ok(vec![
                    self.push(
                        "fetch",
                        format!("{source} {package_kind} {period}"),
                        Spec::Fetch { source: source.into(), package_kind: package_kind.into(), period, refetch },
                    )
                    .await,
                ])
            }
            "process" => {
                let source = req.source.clone().unwrap_or_else(|| "ted".into());
                let package_kind = req.package_kind.clone().unwrap_or_else(|| "daily".into());
                let period = req.period.clone();
                let params = match &period {
                    Some(p) => format!("{source} {package_kind} {p}"),
                    None => format!("{source} {package_kind} (all)"),
                };
                Ok(vec![self.push("process", params, Spec::Process { source, package_kind, period }).await])
            }
            "project" => {
                let rebuild = req.rebuild.unwrap_or(false);
                // `clear_changes` only pairs with a rebuild (it resets the CDC feed
                // for the rebuild to re-emit); ignored on an incremental project.
                let clear_changes = rebuild && req.clear_changes.unwrap_or(false);
                let params = format!("rebuild={rebuild}{}", if clear_changes { " clear_changes" } else { "" });
                Ok(vec![self.push("project", params, Spec::Project { rebuild, clear_changes }).await])
            }
            "backfill" => self.enqueue_backfill(req).await,
            "snapshot" => Ok(vec![self.push("snapshot", "snapshot".into(), Spec::Snapshot).await]),
            // Rebuild any missing deferred indexes on the existing layer, no re-fold
            // (issues 82/83). Safe to fire repeatedly (idempotent).
            "reindex" => Ok(vec![self.push("reindex", "reindex".into(), Spec::Reindex).await]),
            // Escape hatch for a stale rebuild watermark (issue 85 interlock). No-op
            // safe: it reports whether the flag was actually set.
            "clear-rebuild-flag" => {
                Ok(vec![self.push("clear-rebuild-flag", "clear-rebuild-flag".into(), Spec::ClearRebuildFlag).await])
            }
            // Re-project a profile cohort the projection mis-read (issue 85): clear its
            // watermark, then fold it incrementally. Two jobs like `reprocess`, so the
            // fold is a normal queued projection; if the guard aborts the mark, that
            // projection simply finds an empty change-set and returns.
            "refold" => {
                let profiles = req.profiles.clone().unwrap_or_default();
                if profiles.is_empty() {
                    return Err("refold needs at least one profile".into());
                }
                let params = format!("refold {}", profiles.join(","));
                Ok(vec![
                    self.push("refold", params, Spec::Refold { profiles, expect: req.expect }).await,
                    self.push("project", "rebuild=false".into(), Spec::Project { rebuild: false, clear_changes: false })
                        .await,
                ])
            }
            // Force the full daily reconciliation now (post-downtime catch-up,
            // issue 69). Always runs both source probes — the TED probe self-heals
            // a multi-day gap and is a cheap no-op walk on a non-publishing day.
            "daily" => Ok(self.enqueue_daily(true).await),
            // Re-attempt a held quarantine bucket now that a fix ships, then fold
            // what it reclaims (issues 71/72/73). The bucket is reason + optional
            // detail-LIKE + profile.
            "reprocess" => {
                let reason = req.reason.clone().ok_or("reprocess needs a reason")?;
                let detail_like = req.detail_like.clone();
                let profile = req.profile.clone();
                let params = format!(
                    "reprocess {reason}{}{}",
                    detail_like.as_deref().map(|d| format!(" LIKE {d}")).unwrap_or_default(),
                    profile.as_deref().map(|p| format!(" [{p}]")).unwrap_or_default(),
                );
                let mut ids =
                    vec![self.push("reprocess", params, Spec::Reprocess { reason, detail_like, profile }).await];
                // `reclaim_only` skips the trailing incremental fold: a bulk reclaim
                // leaves its members projected=0 for one later `rebuild:true` to fold
                // sequentially, instead of paying a random-seek incremental fold per
                // bucket (issue 81 note / bulk-recovery plan).
                if !req.reclaim_only.unwrap_or(false) {
                    ids.push(self.push("project", "rebuild=false".into(), Spec::Project { rebuild: false, clear_changes: false }).await);
                }
                Ok(ids)
            }
            other => Err(format!("unknown job kind {other:?}")),
        }
    }

    /// A source + period range fanned into per-package fetch jobs, then one
    /// process pass over the whole source and one projection.
    async fn enqueue_backfill(&self, req: &JobRequest) -> Result<Vec<u64>, String> {
        let source = req.source.as_deref().ok_or("backfill needs a source")?;
        let months = match source {
            // DÖE: the whole monthly archive by default, or the given range.
            "doe" => match &req.range {
                Some([a, b]) => months_between(a, b)?,
                None => {
                    let (y, m, _) = fetch::current_date_utc();
                    doe::months_through((y, m))
                        .into_iter()
                        .map(|(y, m)| format!("{y}-{m:02}"))
                        .collect()
                }
            },
            // TED: a monthly range (each month bundles that month's dailies).
            "ted" => {
                let [a, b] = req.range.as_ref().ok_or("ted backfill needs a monthly range")?;
                months_between(a, b)?
            }
            other => return Err(format!("unknown source {other:?}")),
        };
        if months.is_empty() {
            return Err("backfill range is empty".into());
        }

        let src: &'static str = if source == "doe" { "doe" } else { "ted" };
        let mut ids = Vec::new();
        for period in &months {
            ids.push(
                self.push(
                    "fetch",
                    format!("{src} monthly {period}"),
                    Spec::Fetch {
                        source: src.into(),
                        package_kind: "monthly".into(),
                        period: period.clone(),
                        refetch: false,
                    },
                )
                .await,
            );
        }
        ids.push(
            self.push(
                "process",
                format!("{src} monthly (all)"),
                Spec::Process { source: src.to_owned(), package_kind: "monthly".into(), period: None },
            )
            .await,
        );
        ids.push(self.push("project", "rebuild=false".into(), Spec::Project { rebuild: false, clear_changes: false }).await);
        Ok(ids)
    }

    fn fetch_parts(
        &self,
        req: &JobRequest,
    ) -> Result<(&'static str, &'static str, String), String> {
        let source: &'static str = match req.source.as_deref() {
            Some("ted") | None => "ted",
            Some("doe") => "doe",
            Some(o) => return Err(format!("unknown source {o:?}")),
        };
        let package_kind: &'static str = match req.package_kind.as_deref() {
            Some("daily") | None => "daily",
            Some("monthly") => "monthly",
            Some(o) => return Err(format!("unknown package kind {o:?}")),
        };
        let period = req.period.clone().ok_or("fetch needs a period")?;
        Ok((source, package_kind, period))
    }

    /// Remove a still-queued job. Returns false if it is not in the queue
    /// (already running or finished — the running job cannot be cancelled).
    /// Also drops the durable row so the cancellation survives a restart.
    pub async fn cancel(&self, id: u64) -> bool {
        let removed = {
            let mut queue = self.queue.lock().expect("queue lock");
            let before = queue.len();
            queue.retain(|job| job.id != id);
            queue.len() != before
        };
        if removed
            && let Err(e) = self.db.remove_job(id as i64).await
        {
            eprintln!("supervisor: remove cancelled job {id}: {e}");
        }
        removed
    }

    fn pop(&self) -> Option<Job> {
        self.queue.lock().expect("queue lock").pop_front()
    }

    /// Rebuild the in-memory queue from the durable one at startup, before the
    /// worker or scheduler run (issue 21). Rows come back oldest-id first, so the
    /// job that was running when the process died — its row never removed — lands
    /// at the front and re-runs from the top (re-walks are idempotent via
    /// identity dedup). `next_id` is advanced past every recovered id so a new
    /// enqueue cannot collide with a recovered one.
    async fn recover(&self) {
        let pending = match self.db.pending_jobs().await {
            Ok(rows) => rows,
            Err(e) => {
                eprintln!("supervisor: recover queue: {e}");
                return;
            }
        };
        // Operator escape hatch: `TENDER_DROP_JOBS=6,7` removes those durable job
        // rows at boot, before the worker runs. A job that was *running* when the
        // process died is recovered at the front (issue 21) and re-runs from the
        // top; for a long, regenerable job — e.g. a snapshot whose multi-hour
        // offline `integrity_check` verify is blocking a queued resume — dropping
        // its row is the only clean, turso-native way to skip it without editing
        // the live DB out-of-process.
        let drop_ids: std::collections::HashSet<i64> = std::env::var("TENDER_DROP_JOBS")
            .unwrap_or_default()
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        let mut jobs = Vec::with_capacity(pending.len());
        let mut max_id = 0u64;
        for row in pending {
            let id = row.id as u64;
            max_id = max_id.max(id);
            if drop_ids.contains(&row.id) {
                eprintln!("supervisor: dropping recovered job {id} per TENDER_DROP_JOBS");
                let _ = self.db.remove_job(row.id).await;
                continue;
            }
            match serde_json::from_str::<Spec>(&row.spec) {
                Ok(spec) => jobs.push(Job {
                    id,
                    kind: row.kind,
                    params: row.params,
                    spec,
                    resume_after: row.progress,
                }),
                Err(e) => {
                    // A row this build cannot parse is dropped, not fatal — it can
                    // never wedge the queue. `Spec` serializes as serde's
                    // externally-tagged enum (`{"Fetch":{…}}`, `"Snapshot"`), so a
                    // variant a *newer* rev enqueued is, on a rollback to this rev,
                    // an unknown-tag error here — a clean miss we drop, never a
                    // silent misparse into the wrong variant. Adding a `Spec`
                    // variant is therefore forward/backward safe: old revs shed
                    // what they don't understand (a dropped Snapshot just isn't
                    // taken; it is regenerable), and this is the only place the
                    // queue's on-disk format is decoded.
                    eprintln!("supervisor: dropping unreadable queued job {id}: {e}");
                    let _ = self.db.remove_job(row.id).await;
                }
            }
        }
        let recovered = jobs.len();
        self.queue.lock().expect("queue lock").extend(jobs);
        if max_id + 1 > self.next_id.load(Ordering::Relaxed) {
            self.next_id.store(max_id + 1, Ordering::Relaxed);
        }
        if recovered > 0 {
            eprintln!("supervisor: recovered {recovered} pending job(s) from the durable queue");
            self.wake.notify_one();
        }
    }

    // ---------------------------------------------------------------- progress

    fn set_current(&self, progress: Option<JobProgress>) {
        *self.current.write().expect("progress lock") = progress;
    }

    fn update<F: FnOnce(&mut JobProgress)>(&self, f: F) {
        if let Some(p) = self.current.write().expect("progress lock").as_mut() {
            f(p);
        }
    }

    /// True while a write-heavy job — a package walk (`process`) or a projection
    /// (`project`) — is running. These are the jobs whose per-package / per-batch
    /// TRUNCATE checkpoint (issue 42) needs reader-free windows to reclaim the
    /// WAL. The dashboard's coverage refresher consults this and skips its
    /// multi-minute full-`notices` scan while one runs: a live reader snapshot
    /// held across that scan pins the WAL, blocks the TRUNCATE, and the log
    /// balloons (70 GB in the field) — issue 53.
    pub fn heavy_write_in_progress(&self) -> bool {
        self.current
            .read()
            .expect("progress lock")
            .as_ref()
            .is_some_and(|p| {
                matches!(p.kind.as_str(), "process" | "project" | "reprocess" | "reindex" | "refold")
            })
    }

    fn queued(&self) -> Vec<QueuedJob> {
        self.queue
            .lock()
            .expect("queue lock")
            .iter()
            .map(|j| QueuedJob { id: j.id, kind: j.kind.to_owned(), params: j.params.clone() })
            .collect()
    }

    /// The full Supervisor snapshot the admin API and dashboard render: the
    /// running job, the queue, and the persisted recent-run log.
    pub async fn ingestion(&self) -> turso::Result<Ingestion> {
        // Snapshot the in-memory state into owned values FIRST: the std lock
        // guards must not be held across the await below, or the future stops
        // being `Send` and axum rejects the handler.
        let current = self.current.read().expect("progress lock").clone();
        let queued = self.queued();
        // `recent_job_runs` reads through the store's reader pool, not the writer
        // an ingestion job holds — so this never queues behind it (issue 20).
        let recent = self.db.recent_job_runs(RECENT_RUNS).await?;
        Ok(Ingestion { current, queued, recent, measured_at: store::now_unix() })
    }

    // ------------------------------------------------------------------ worker

    /// Spawn the worker loop: pop a job, run it, sleep on the doorbell when idle.
    ///
    /// The loop itself (pop + doorbell) stays on the main runtime, but each job's
    /// heavy body runs on the isolated `worker_runtime` (issue 61): the loop
    /// submits `execute` there via `Handle::spawn` and only awaits the
    /// `JoinHandle` — a cheap channel wait — so the job's blocking turso preads
    /// pin the job runtime's threads, never a main API/SSE/dashboard worker.
    /// Awaiting the single handle preserves the one-job-at-a-time serialisation.
    pub fn spawn_worker(self: Arc<Self>) {
        tokio::spawn(async move {
            loop {
                match self.pop() {
                    Some(job) => {
                        let sup = self.clone();
                        let handle = self.worker_runtime.spawn(async move { sup.execute(job).await });
                        if let Err(e) = handle.await {
                            // A panic inside a job must not take the worker loop
                            // down: log it and move on to the next job. The job's
                            // durable row survives (execute never reached its
                            // remove_job), so it recovers on the next restart.
                            eprintln!("supervisor: job runtime task failed: {e}");
                        }
                    }
                    None => self.wake.notified().await,
                }
            }
        });
    }

    async fn execute(&self, job: Job) {
        let started_at = store::now_unix();
        self.set_current(Some(JobProgress {
            id: job.id,
            kind: job.kind.to_owned(),
            params: job.params.clone(),
            started_at,
            package: None,
            packages_done: 0,
            packages_total: 0,
            members_done: 0,
            members_total: 0,
            notices: 0,
            duplicates: 0,
        }));

        let result = self.run_spec(&job).await;
        self.set_current(None);

        let (outcome, counts) = match result {
            Ok(summary) => ("ok", summary),
            Err(e) => ("error", e),
        };
        if let Err(e) = self
            .db
            .record_job_run(&job.kind, &job.params, started_at, store::now_unix(), outcome, &counts)
            .await
        {
            // The log is best-effort telemetry; a failure to persist it must not
            // take the worker down.
            eprintln!("supervisor: record job {} log: {e}", job.id);
        }
        // The job has concluded (ok or error) — drop its durable row. A job that
        // was killed mid-run never reaches here, so its row survives for recovery
        // and re-runs from the top on the next start (issue 21).
        if let Err(e) = self.db.remove_job(job.id as i64).await {
            eprintln!("supervisor: remove finished job {} from queue: {e}", job.id);
        }
    }

    async fn run_spec(&self, job: &Job) -> Result<String, String> {
        match &job.spec {
            Spec::Fetch { source, package_kind, period, refetch } => {
                let target = build_target(&self.ted_base, &self.doe_base, source, package_kind, period)?;
                self.update(|p| {
                    p.package = Some(period.clone());
                    p.packages_total = 1;
                });
                let outcome = fetch::fetch(&self.db, &self.http, &self.archive, &target, *refetch)
                    .await
                    .map_err(|e| e.to_string())?;
                self.update(|p| p.packages_done = 1);
                Ok(format!("{outcome:?}"))
            }
            Spec::ProbeTed { refetch } => {
                let results = fetch::probe_ted_daily(
                    &self.db,
                    &self.http,
                    &self.archive,
                    &self.ted_base,
                    *refetch,
                    |period, _| self.update(|p| p.package = Some(period.to_owned())),
                )
                .await
                .map_err(|e| e.to_string())?;
                let fetched = results
                    .iter()
                    .filter(|(_, o)| {
                        matches!(o, fetch::Outcome::Fetched | fetch::Outcome::NewVersion)
                    })
                    .count();
                Ok(format!("probed {} issue(s), {fetched} new", results.len()))
            }
            Spec::ProbeDoe => {
                // The last completed T+1 day (DÖE rejects today/future): yesterday UTC.
                let end = fetch::civil_date(store::now_unix() - 86_400);
                let results = fetch::probe_doe_daily(
                    &self.db,
                    &self.http,
                    &self.archive,
                    &self.doe_base,
                    end,
                    |period, _| self.update(|p| p.package = Some(period.to_owned())),
                )
                .await
                .map_err(|e| e.to_string())?;
                let fetched = results
                    .iter()
                    .filter(|(_, o)| {
                        matches!(o, fetch::Outcome::Fetched | fetch::Outcome::NewVersion)
                    })
                    .count();
                Ok(format!("probed {} day(s), {fetched} new", results.len()))
            }
            Spec::Process { source, package_kind, period } => {
                self.run_process(job.id, source, package_kind, period.as_deref(), job.resume_after.as_deref())
                    .await
            }
            Spec::Reprocess { reason, detail_like, profile } => {
                self.run_reprocess(
                    job.id,
                    reason,
                    detail_like.as_deref(),
                    profile.as_deref(),
                    job.resume_after.as_deref(),
                )
                .await
            }
            Spec::Project { rebuild, clear_changes } => {
                // Resume-from-plan salvage (issue 60) OUTRANKS the rebuild/
                // incremental routing: if a full rebuild's Phase-2 was interrupted,
                // finish it (re-run grouping + Phase-2 from the on-disk plan, SKIP
                // Phase-1) whatever THIS recovered job's rebuild flag says —
                // `project(_, true)` detects the complete plan and resumes. Without
                // it a recovered `rebuild:false` job would route to
                // `project_incremental`, see an all-unprojected corpus, and re-do the
                // whole Phase-1, discarding the salvage.
                //
                // The signal is the durable `rebuild_in_progress` flag, NOT
                // `plan_is_complete()`. A complete plan on disk is NOT proof of an
                // interrupted rebuild: it is also the resting state of a FINISHED
                // build (whose layer is fully applied) and of an interrupted
                // rebuild=false full-fallback (over an intact layer). Keying the
                // salvage on plan-completeness re-fired `reset_tender_layer` on every
                // restart, nuking a good 6.96M-tender layer into a ~15h re-fold each
                // time — a livelock. The flag is set only when a rebuild empties the
                // layer and cleared with the plan on clean completion, so it is true
                // exactly when there is an interrupted rebuild to finish.
                let salvage = self.db.rebuild_in_progress().await.map_err(|e| e.to_string())?;

                // One-time CDC baseline reset (issue 81): on a rebuild flagged
                // `clear_changes`, DROP+recreate the feed FIRST so the rebuild
                // re-emits ONE clean generation for the recovered baseline instead of
                // appending onto the accumulated feed. Only on a rebuild path (never
                // an incremental project). Idempotent on a salvage resume — the durable
                // flag re-clears any partial generation the interrupted rebuild wrote.
                if *clear_changes && (salvage || *rebuild) {
                    self.db.clear_changes().await.map_err(|e| e.to_string())?;
                }

                // Otherwise: the daily path is INCREMENTAL (issue 58) — re-derive
                // only the Tenders touched since the last run; a `rebuild` does the
                // full bounded-streaming projection (initial build / schema change)
                // and resets the `projected` watermark.
                let report = if salvage || *rebuild {
                    project::project(&self.db, true).await
                } else {
                    project::project_incremental(&self.db).await
                }
                .map_err(|e| e.to_string())?;
                self.update(|p| p.notices = report.notices);
                Ok(format!(
                    "{} notices → {} tenders ({} islands), {} versions",
                    report.notices,
                    report.tenders,
                    report.islands,
                    report.applied.versions_written
                ))
            }
            Spec::Snapshot => snapshot::run(&self.db, &snapshot::Config::from_env(), store::now_unix()).await,
            Spec::Reindex => {
                // Both builders are CREATE INDEX IF NOT EXISTS loops — idempotent, so
                // this rebuilds only the missing deferred indexes without touching the
                // fold. Pair the TRUNCATE checkpoint to reclaim the build's WAL tail,
                // exactly as the rebuild's end-of-fold index build does (project.rs).
                self.db.build_organization_indexes().await.map_err(|e| e.to_string())?;
                self.db.build_tender_indexes().await.map_err(|e| e.to_string())?;
                let _ = self.db.checkpoint(store::CheckpointMode::Truncate).await;
                Ok("deferred org + tender indexes rebuilt".into())
            }
            Spec::Refold { profiles, expect } => {
                let refs: Vec<&str> = profiles.iter().map(String::as_str).collect();
                // Count BEFORE writing: a mistyped profile string matching a far larger
                // set would otherwise re-queue that set silently, and the trailing
                // projection would fold it. Abort while nothing has been written yet.
                let found = self
                    .db
                    .projected_notice_count_for_profiles(&refs)
                    .await
                    .map_err(|e| e.to_string())?;
                if let Some(expect) = expect {
                    let slack = expect / 4;
                    if found.abs_diff(*expect) > slack {
                        return Err(format!(
                            "refold aborted: {} notices match {:?}, expected ~{expect} — \
                             check the profile strings (nothing was written)",
                            found, profiles
                        ));
                    }
                }
                let requeued =
                    self.db.unmark_projected_for_profiles(&refs).await.map_err(|e| e.to_string())?;
                Ok(format!("re-queued {requeued} notices for the incremental fold"))
            }
            Spec::ClearRebuildFlag => {
                let was_set = self.db.rebuild_in_progress().await.map_err(|e| e.to_string())?;
                if !was_set {
                    return Ok("rebuild_in_progress was already clear — nothing to do".to_owned());
                }
                // The flag is only STALE over an intact layer. An EMPTY tenders table
                // with the flag set is a rebuild genuinely mid-flight (reset_tender_layer
                // has run, the fold has not finished) — clearing there would discard a
                // real salvage and force a full re-fold. O(1): existence, not a count.
                let intact = self
                    .db
                    .scalar("SELECT 1 FROM tenders LIMIT 1")
                    .await
                    .map_err(|e| e.to_string())?
                    .is_some();
                if !intact {
                    return Err("refusing to clear rebuild_in_progress: the tenders table is EMPTY, \
                                so a rebuild is genuinely mid-flight and its salvage would be lost \
                                — let it finish (nothing was written)"
                        .to_owned());
                }
                self.db.clear_plan().await.map_err(|e| e.to_string())?;
                Ok("stale rebuild_in_progress CLEARED (plan retired) — projections route \
                    incremental again"
                    .to_owned())
            }
        }
    }

    /// Walk the source's current packages through the processor, updating live
    /// progress per package and per member. The store's single writer serialises
    /// the inserts; readers keep serving over WAL throughout (zero downtime).
    async fn run_process(
        &self,
        job_id: u64,
        source: &str,
        kind: &str,
        period: Option<&str>,
        resume_after: Option<&str>,
    ) -> Result<String, String> {
        let all = self.db.current_packages(source, kind, period).await.map_err(|e| e.to_string())?;
        // Resume (issue 32): `current_packages` is ordered by period, so on a
        // restart skip every package at or before the last one a prior run fully
        // completed. Correct because a package is recorded done only after its
        // last member committed; the partial one that was interrupted has a period
        // > the cursor, so it re-runs and dedups.
        let skipped = resume_skip(&all, resume_after);
        let packages = &all[skipped..];
        self.update(|p| p.packages_total = packages.len() as u64);
        if let Some(cursor) = resume_after {
            eprintln!("supervisor: job {job_id} resumes after {cursor} ({skipped} package(s) already done)");
        }
        if packages.is_empty() {
            return Ok("no packages to process".into());
        }

        let mut total = process::Report::default();
        for (i, pkg) in packages.iter().enumerate() {
            self.update(|p| {
                p.package = Some(pkg.period.clone());
                p.packages_done = i as u64;
                p.members_done = 0;
                p.members_total = 0;
            });
            let base_notices = total.notices;
            let base_duplicates = total.duplicates;
            // Resilient: a corrupt package is quarantined and skipped, so one
            // bad archived file never aborts a multi-year job; only a systemic
            // (database) failure is fatal (ADR-0004).
            let report = process::process_package_resilient(
                &self.db,
                &self.archive.join(&pkg.path),
                source,
                pkg.fetch_id,
                |done, members_total, r| {
                    // Throttle the shared write: every 64 members and at the end.
                    if done % 64 == 0 || done == members_total {
                        self.update(|p| {
                            p.members_done = done;
                            p.members_total = members_total;
                            p.notices = base_notices + r.notices;
                            // Surfaced so the dashboard can name a re-walk (issue 33).
                            p.duplicates = base_duplicates + r.duplicates;
                        });
                    }
                },
            )
            .await
            .map_err(|e| format!("db: {e}"))?;
            total.members += report.members;
            total.notices += report.notices;
            total.parsed += report.parsed;
            total.parse_quarantined += report.parse_quarantined;
            total.quarantined += report.quarantined;
            total.duplicates += report.duplicates;
            self.update(|p| {
                p.packages_done = (i + 1) as u64;
                p.notices = total.notices;
            });
            // Advance the durable resume cursor now the package is fully committed
            // (issue 32). Best-effort: a failed cursor write only costs a re-walk
            // of this package on the next restart, never correctness.
            if let Err(e) = self.db.record_job_progress(job_id as i64, &pkg.period).await {
                eprintln!("supervisor: job {job_id} record progress {}: {e}", pkg.period);
            }
            // Bound the WAL (issue 42): turso autocheckpoints PASSIVE at a size
            // threshold, but that reuses the -wal file in place (never shrinks it)
            // and is blocked whenever a long reader snapshot is held (the coverage
            // scan) — so in the field the WAL spiked to 13 GB. TRUNCATE here — a
            // writer-idle moment, right after the package committed — returns the
            // file space and forces reclaim; idle pooled readers do not pin it, and
            // a busy result (a reader mid-scan) simply reclaims on the next package
            // (verified in store::checkpoint tests). Best-effort: a failed
            // checkpoint only delays reclaim, never correctness.
            match self.db.checkpoint(store::CheckpointMode::Truncate).await {
                Ok(c) if c.busy => eprintln!(
                    "supervisor: job {job_id} checkpoint after {} busy (reader pinned), wal {} MB",
                    pkg.period,
                    self.db.wal_bytes().unwrap_or(0) / 1_048_576
                ),
                Ok(_) => {}
                Err(e) => eprintln!("supervisor: job {job_id} checkpoint after {}: {e}", pkg.period),
            }
        }

        Ok(format!(
            "{} members → {} notices ({} parsed, {} quarantined, {} unrecognised, {} dup)",
            total.members,
            total.notices,
            total.parsed,
            total.parse_quarantined,
            total.quarantined,
            total.duplicates
        ))
    }

    /// Re-attempt a held quarantine bucket, package by package, writing the parsed
    /// layer in place for members that now parse (issues 71/72/73). Resumable by
    /// `fetch_id` cursor (like `run_process`) and idempotent — a reclaimed member
    /// is `parsed` on the next pass, so a restart mid-bucket redoes nothing.
    async fn run_reprocess(
        &self,
        job_id: u64,
        reason: &str,
        detail_like: Option<&str>,
        profile: Option<&str>,
        resume_after: Option<&str>,
    ) -> Result<String, String> {
        // The work list is re-derived each run: reclaimed packages fall out (their
        // rows carry `reprocessed_at`), and `after` skips those a prior run drained.
        let after = resume_after.and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
        let packages = self
            .db
            .quarantine_reclaim_packages(reason, detail_like, profile, after)
            .await
            .map_err(|e| e.to_string())?;
        self.update(|p| p.packages_total = packages.len() as u64);
        if packages.is_empty() {
            return Ok("no held packages to reprocess".into());
        }

        let (mut reclaimed, mut still_held, mut already, mut skipped) = (0u64, 0u64, 0u64, 0u64);
        for (i, (fetch_id, source, path)) in packages.iter().enumerate() {
            self.update(|p| {
                p.package = Some(format!("fetch {fetch_id}"));
                p.packages_done = i as u64;
                p.members_done = 0;
                p.members_total = 0;
            });
            // Issue 77: parse only this package's held members, not all of them.
            let held = self
                .db
                .quarantine_held_member_files(*fetch_id, reason, detail_like, profile)
                .await
                .map_err(|e| e.to_string())?;
            let report = process::reclaim_package(
                &self.db,
                &self.archive.join(path),
                source,
                *fetch_id,
                held,
                |done, members_total, _| {
                    if done % 64 == 0 || done == members_total {
                        self.update(|p| {
                            p.members_done = done;
                            p.members_total = members_total;
                        });
                    }
                },
            )
            .await
            .map_err(|e| format!("db: {e}"))?;
            reclaimed += report.reclaimed;
            still_held += report.still_held;
            already += report.already;
            skipped += report.skipped_by_policy;
            self.update(|p| p.packages_done = (i + 1) as u64);
            // Advance the durable resume cursor once the package is fully drained
            // (issue 32 pattern). Best-effort: a failed write only re-walks this
            // package next restart — idempotent, never a correctness cost.
            if let Err(e) = self.db.record_job_progress(job_id as i64, &fetch_id.to_string()).await {
                eprintln!("supervisor: job {job_id} record reprocess progress {fetch_id}: {e}");
            }
            // Bound the WAL between packages, exactly as `run_process` does.
            if let Err(e) = self.db.checkpoint(store::CheckpointMode::Truncate).await {
                eprintln!("supervisor: job {job_id} checkpoint after fetch {fetch_id}: {e}");
            }
        }

        // Every held member ends in exactly one of the four outcomes, so the
        // summary sums to the bucket. `skipped` is the one that writes nothing:
        // a documented duplicate sibling no reclaim can ever move (issue 84).
        Ok(format!(
            "{} package(s): {reclaimed} reclaimed, {still_held} still held, \
             {already} already parsed, {skipped} skipped by dispatch policy",
            packages.len()
        ))
    }

    // --------------------------------------------------------------- scheduler

    /// Spawn the daily scheduler: at 09:35 Europe/Berlin it enqueues the TED
    /// probe (Mon–Fri) and DÖE completed-day fetch, then process + project.
    pub fn spawn_scheduler(self: Arc<Self>) {
        tokio::spawn(async move {
            loop {
                let now = store::now_unix();
                let (tick, weekday) = next_berlin_tick(now, 9, 35);
                let wait = (tick - now).max(0) as u64;
                tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
                self.enqueue_daily(weekday).await;
                // Step past this tick so the next computation lands on tomorrow.
                tokio::time::sleep(std::time::Duration::from_secs(61)).await;
            }
        });
    }

    /// The daily pipeline, in execution order (jobs run sequentially). Returns
    /// the enqueued job ids so the admin "run daily now" path can report them.
    async fn enqueue_daily(&self, weekday: bool) -> Vec<u64> {
        let mut ids = Vec::new();
        // TED publishes Mon–Fri; probe forward and re-fetch the current day for
        // the finality window (a daily may be rewritten until 09:30 CET).
        if weekday {
            ids.push(self.push("probe", "ted daily (probe)".into(), Spec::ProbeTed { refetch: true }).await);
            ids.push(
                self.push(
                    "process",
                    "ted daily (all)".into(),
                    Spec::Process { source: "ted".into(), package_kind: "daily".into(), period: None },
                )
                .await,
            );
        }
        // DÖE walks forward from its last fetched day up to yesterday (the freshest
        // completed T+1 day), so a missed tick catches up instead of leaving a hole.
        ids.push(self.push("probe", "doe daily (probe)".into(), Spec::ProbeDoe).await);
        ids.push(
            self.push(
                "process",
                "doe daily (all)".into(),
                Spec::Process { source: "doe".into(), package_kind: "daily".into(), period: None },
            )
            .await,
        );
        // One projection folds whatever the fetch+process just landed.
        ids.push(self.push("project", "rebuild=false".into(), Spec::Project { rebuild: false, clear_changes: false }).await);
        // Then a consistent snapshot of the day's result, shipped off-box by the
        // systemd timer (issue 23). Last in the sequence, so it captures the
        // freshly folded canonical layer.
        ids.push(self.push("snapshot", "snapshot".into(), Spec::Snapshot).await);
        ids
    }
}

/// How many leading packages a resumed process job skips: the period-ordered
/// prefix at or before the cursor (issue 32). `None` (a fresh job) skips nothing.
fn resume_skip(packages: &[store::Package], resume_after: Option<&str>) -> usize {
    resume_after.map_or(0, |cursor| {
        packages.iter().take_while(|pkg| pkg.period.as_str() <= cursor).count()
    })
}

// --------------------------------------------------------------- period → URL

/// Build a fetch target from a source + package kind + period string.
fn build_target(
    ted_base: &str,
    doe_base: &str,
    source: &str,
    package_kind: &str,
    period: &str,
) -> Result<fetch::Target, String> {
    match (source, package_kind) {
        ("ted", "daily") => {
            let (year, issue) = parse_issue(period)?;
            Ok(ted::daily(ted_base, year, issue))
        }
        ("ted", "monthly") => {
            let (year, month) = parse_year_month(period)?;
            Ok(ted::monthly(ted_base, year, month))
        }
        ("doe", "daily") => Ok(doe::day(doe_base, parse_ymd(period)?)),
        ("doe", "monthly") => {
            let (year, month) = parse_year_month(period)?;
            Ok(doe::monthly(doe_base, year, month))
        }
        (s, k) => Err(format!("no fetch target for {s} {k}")),
    }
}

/// `YYYY-NNNNN` → (year, issue).
fn parse_issue(period: &str) -> Result<(u16, u32), String> {
    let (y, n) = period.split_once('-').ok_or_else(|| bad(period))?;
    Ok((y.parse().map_err(|_| bad(period))?, n.parse().map_err(|_| bad(period))?))
}

/// `YYYY-MM` → (year, month).
fn parse_year_month(period: &str) -> Result<(u16, u8), String> {
    let (y, m) = period.split_once('-').ok_or_else(|| bad(period))?;
    Ok((y.parse().map_err(|_| bad(period))?, m.parse().map_err(|_| bad(period))?))
}

/// `YYYY-MM-DD` → (year, month, day).
fn parse_ymd(period: &str) -> Result<(u16, u8, u8), String> {
    let mut parts = period.split('-');
    let mut next = || parts.next().ok_or_else(|| bad(period));
    let y = next()?.parse().map_err(|_| bad(period))?;
    let m = next()?.parse().map_err(|_| bad(period))?;
    let d = next()?.parse().map_err(|_| bad(period))?;
    Ok((y, m, d))
}

fn bad(period: &str) -> String {
    format!("malformed period {period:?}")
}

/// Inclusive list of `YYYY-MM` periods from `a` to `b`.
fn months_between(a: &str, b: &str) -> Result<Vec<String>, String> {
    let (mut y, mut m) = parse_year_month(a)?;
    let (ey, em) = parse_year_month(b)?;
    if (y, m) > (ey, em) {
        return Err(format!("range start {a} is after end {b}"));
    }
    let mut out = Vec::new();
    while (y, m) <= (ey, em) {
        out.push(format!("{y}-{m:02}"));
        (y, m) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    }
    Ok(out)
}

// ------------------------------------------------------------ Europe/Berlin

/// The next unix instant at which Berlin local wall-clock reads `hour:minute`,
/// with that day's weekday (`true` = Mon–Fri). 09:35 is far from the 01:00–03:00
/// DST switch, so taking the day's offset at noon is unambiguous.
fn next_berlin_tick(now: i64, hour: i64, minute: i64) -> (i64, bool) {
    let day = now.div_euclid(86_400);
    for k in 0..8 {
        let midnight = (day + k) * 86_400;
        let offset = berlin_offset(midnight + 12 * 3_600);
        let tick = midnight + hour * 3_600 + minute * 60 - offset;
        if tick > now {
            let weekday = (day + k + 4).rem_euclid(7); // 0 = Sunday
            return (tick, weekday != 0 && weekday != 6);
        }
    }
    unreachable!("a matching tick exists within a week")
}

/// Berlin's UTC offset in seconds at `unix`: +1h CET, +2h CEST. EU rule: summer
/// runs from the last Sunday of March 01:00 UTC to the last Sunday of October
/// 01:00 UTC.
fn berlin_offset(unix: i64) -> i64 {
    let (year, _, _) = fetch::civil_date(unix);
    let start = last_sunday(year, 3) + 3_600; // 01:00 UTC, last Sunday March
    let end = last_sunday(year, 10) + 3_600; // 01:00 UTC, last Sunday October
    if unix >= start && unix < end { 7_200 } else { 3_600 }
}

/// 00:00 UTC of the last Sunday of `(year, month)`. March and October both have
/// 31 days, which is all this is called for.
fn last_sunday(year: u16, month: u8) -> i64 {
    let z = fetch::days_from_civil(year, month, 31);
    let weekday = (z + 4).rem_euclid(7); // 0 = Sunday
    (z - weekday) * 86_400
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn months_between_is_inclusive_and_crosses_years() {
        assert_eq!(months_between("2024-11", "2025-02").unwrap(), ["2024-11", "2024-12", "2025-01", "2025-02"]);
        assert_eq!(months_between("2024-06", "2024-06").unwrap(), ["2024-06"]);
        assert!(months_between("2025-02", "2024-11").is_err());
    }

    #[test]
    fn targets_are_built_per_source_and_kind() {
        let t = build_target("https://ted", "https://doe", "ted", "daily", "2026-00137").unwrap();
        assert_eq!(t.url, "https://ted/packages/daily/202600137");
        let t = build_target("https://ted", "https://doe", "ted", "monthly", "2026-06").unwrap();
        assert_eq!(t.rel_path, "ted/monthly/2026-06.tar");
        let t = build_target("https://ted", "https://doe", "doe", "daily", "2026-07-18").unwrap();
        assert_eq!(t.period, "2026-07-18");
        assert!(build_target("https://ted", "https://doe", "ted", "weekly", "x").is_err());
        assert!(build_target("https://ted", "https://doe", "ted", "daily", "nope").is_err());
    }

    /// The EU DST rule: CET in winter, CEST in summer, switching on the last
    /// Sundays of March and October at 01:00 UTC.
    #[test]
    fn berlin_offset_follows_the_eu_dst_rule() {
        // 2026: last Sunday of March is the 29th; October is the 25th.
        let mar29_0030 = fetch::days_from_civil(2026, 3, 29) * 86_400 + 30 * 60; // 00:30 UTC → still CET
        let mar29_0130 = fetch::days_from_civil(2026, 3, 29) * 86_400 + 3_600 + 30 * 60; // 01:30 UTC → CEST
        assert_eq!(berlin_offset(mar29_0030), 3_600);
        assert_eq!(berlin_offset(mar29_0130), 7_200);

        let oct25_0030 = fetch::days_from_civil(2026, 10, 25) * 86_400 + 30 * 60; // still CEST
        let oct25_0130 = fetch::days_from_civil(2026, 10, 25) * 86_400 + 3_600 + 30 * 60; // back to CET
        assert_eq!(berlin_offset(oct25_0030), 7_200);
        assert_eq!(berlin_offset(oct25_0130), 3_600);

        // Deep winter and deep summer.
        assert_eq!(berlin_offset(fetch::days_from_civil(2026, 1, 15) * 86_400), 3_600);
        assert_eq!(berlin_offset(fetch::days_from_civil(2026, 7, 15) * 86_400), 7_200);
    }

    /// 09:35 Berlin on a known summer day is 07:35 UTC; the weekday flag is right.
    #[test]
    fn next_tick_lands_on_0935_berlin() {
        // 2026-07-15 is a Wednesday. 00:00 UTC that day.
        let midnight = fetch::days_from_civil(2026, 7, 15) * 86_400;
        let (tick, weekday) = next_berlin_tick(midnight, 9, 35);
        // CEST (+2h): 09:35 local = 07:35 UTC.
        assert_eq!(tick, midnight + 7 * 3_600 + 35 * 60);
        assert!(weekday, "Wednesday is a weekday");

        // From just after the tick, the next one is the following day.
        let (next, _) = next_berlin_tick(tick + 1, 9, 35);
        assert_eq!(next, tick + 86_400);

        // 2026-07-18 is a Saturday.
        let sat = fetch::days_from_civil(2026, 7, 18) * 86_400;
        let (_, weekend) = next_berlin_tick(sat, 9, 35);
        assert!(!weekend, "Saturday is not a weekday");
    }

    async fn scratch() -> Arc<store::Db> {
        // A per-call counter, not just the wall clock: tests run in parallel and
        // now write to the durable queue, so two sharing a second must not share
        // a database file.
        static N: AtomicU64 = AtomicU64::new(0);
        let path = format!(
            "/tmp/tender-db-sup-{}-{}-{}.db",
            std::process::id(),
            store::now_unix(),
            N.fetch_add(1, Ordering::Relaxed)
        );
        let _ = std::fs::remove_file(&path);
        Arc::new(store::Db::open(&path).await.unwrap())
    }

    fn req(kind: &str) -> JobRequest {
        JobRequest { kind: kind.into(), ..Default::default() }
    }

    fn progress(kind: &str) -> JobProgress {
        JobProgress {
            id: 1,
            kind: kind.into(),
            params: String::new(),
            started_at: 0,
            package: None,
            packages_done: 0,
            packages_total: 0,
            members_done: 0,
            members_total: 0,
            notices: 0,
            duplicates: 0,
        }
    }

    /// Issue 53: the coverage refresher gates its WAL-pinning scan on this. Only
    /// the jobs that write the store heavily and checkpoint it — `process` and
    /// `project` — count; a light `fetch`/`probe` or an idle supervisor does not,
    /// so coverage keeps measuring in those gaps.
    #[tokio::test]
    async fn heavy_write_in_progress_tracks_the_running_job_kind() {
        let sup = Supervisor::new(scratch().await, "archive".into(), reqwest::Client::new());
        assert!(!sup.heavy_write_in_progress(), "idle: nothing pins the WAL");
        sup.set_current(Some(progress("process")));
        assert!(sup.heavy_write_in_progress(), "a package walk holds the WAL");
        sup.set_current(Some(progress("project")));
        assert!(sup.heavy_write_in_progress(), "a projection holds the WAL");
        sup.set_current(Some(progress("fetch")));
        assert!(!sup.heavy_write_in_progress(), "a fetch is light — coverage may scan");
        sup.set_current(None);
        assert!(!sup.heavy_write_in_progress(), "idle again");
    }

    /// Enqueue, inspect the queue, and cancel — all without a running worker, so
    /// jobs stay put and the transitions are deterministic.
    #[tokio::test]
    async fn queue_enqueues_and_cancels() {
        let sup = Supervisor::new(scratch().await, "archive".into(), reqwest::Client::new());

        let a = sup.enqueue_request(&req("project")).await.unwrap();
        let b = sup
            .enqueue_request(&JobRequest {
                kind: "process".into(),
                source: Some("ted".into()),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(sup.queued().len(), 2);
        assert_eq!(sup.queued()[0].id, a[0], "FIFO order");

        assert!(sup.cancel(b[0]).await, "a queued job cancels");
        assert_eq!(sup.queued().len(), 1);
        assert!(!sup.cancel(b[0]).await, "cancelling twice is a no-op");
        assert!(!sup.cancel(9_999).await, "an unknown id cancels nothing");
    }

    /// Backfill fans a period range into one fetch job per package, then a
    /// process pass and a projection.
    #[tokio::test]
    async fn backfill_fans_a_range_into_per_package_jobs() {
        let sup = Supervisor::new(scratch().await, "archive".into(), reqwest::Client::new());
        let ids = sup
            .enqueue_request(&JobRequest {
                kind: "backfill".into(),
                source: Some("doe".into()),
                range: Some(["2024-01".into(), "2024-03".into()]),
                ..Default::default()
            })
            .await
            .unwrap();
        // 3 monthly fetches + 1 process + 1 project.
        assert_eq!(ids.len(), 5);
        let queued = sup.queued();
        assert_eq!(queued.iter().filter(|j| j.kind == "fetch").count(), 3);
        assert_eq!(queued.last().unwrap().kind, "project");

        // A bad request is rejected, not enqueued.
        assert!(sup.enqueue_request(&req("nonsense")).await.is_err());
        assert!(sup.enqueue_request(&req("fetch")).await.is_err(), "fetch needs a period");
    }

    /// Issue 21: the queue is durable. A fresh Supervisor over the same DB, once
    /// recovered, rebuilds the same pending jobs in the same order — the restart
    /// path, without a real kill.
    #[tokio::test]
    async fn recovers_the_queue_across_a_restart() {
        let db = scratch().await;
        let sup = Supervisor::new(db.clone(), "archive".into(), reqwest::Client::new());
        sup.enqueue_request(&JobRequest {
            kind: "backfill".into(),
            source: Some("doe".into()),
            range: Some(["2024-01".into(), "2024-02".into()]),
            ..Default::default()
        })
        .await
        .unwrap();
        let before = sup.queued();
        assert_eq!(before.len(), 4, "2 fetches + process + project");

        // "Restart": a new Supervisor over the same database, recovered.
        let restarted = Supervisor::new(db.clone(), "archive".into(), reqwest::Client::new());
        assert!(restarted.queued().is_empty(), "a fresh in-memory queue starts empty");
        restarted.recover().await;

        let after = restarted.queued();
        assert_eq!(after.len(), before.len(), "every pending job is restored");
        for (a, b) in after.iter().zip(&before) {
            assert_eq!((a.id, &a.kind, &a.params), (b.id, &b.kind, &b.params), "id, kind, order preserved");
        }
        // A newly enqueued job gets an id above every recovered one — no collision.
        let fresh = restarted.enqueue_request(&req("project")).await.unwrap();
        assert!(fresh[0] > after.last().unwrap().id, "next_id advanced past recovered ids");
    }

    /// Issue 21 acceptance in miniature: a job that was *running* when the process
    /// died (popped from memory, never completed → its durable row is still there)
    /// comes back at the front on restart, ahead of the jobs that were still
    /// queued behind it.
    #[tokio::test]
    async fn an_interrupted_running_job_is_recovered_at_the_front() {
        let db = scratch().await;
        let sup = Supervisor::new(db.clone(), "archive".into(), reqwest::Client::new());
        let running = sup.enqueue_request(&req("project")).await.unwrap()[0]; // job 1 — "runs"
        sup.enqueue_request(&JobRequest { kind: "process".into(), source: Some("ted".into()), ..Default::default() })
            .await
            .unwrap();

        // The worker takes job 1 and is then killed mid-run: pop it from memory
        // but never call execute()/remove_job, so its durable row survives.
        let taken = sup.pop().expect("a job to run");
        assert_eq!(taken.id, running);

        let restarted = Supervisor::new(db.clone(), "archive".into(), reqwest::Client::new());
        restarted.recover().await;
        let q = restarted.queued();
        assert_eq!(q.len(), 2, "the interrupted job and the one queued behind it both return");
        assert_eq!(q[0].id, running, "the interrupted job is back at the front");
        assert_eq!(q[0].kind, "project");
        assert_eq!(q[1].kind, "process");
    }

    /// A cancelled job stays gone across a restart — cancel drops the durable row.
    #[tokio::test]
    async fn a_cancelled_job_does_not_come_back() {
        let db = scratch().await;
        let sup = Supervisor::new(db.clone(), "archive".into(), reqwest::Client::new());
        let ids = sup.enqueue_request(&req("project")).await.unwrap();
        assert!(sup.cancel(ids[0]).await);

        let restarted = Supervisor::new(db.clone(), "archive".into(), reqwest::Client::new());
        restarted.recover().await;
        assert!(restarted.queued().is_empty(), "a cancelled job is gone from the durable queue too");
    }

    /// Issue 23: a queued `snapshot` job round-trips through the durable queue —
    /// `Spec::Snapshot` serialises into `job_queue` and deserialises back on
    /// recovery. This is exactly the daily-pipeline case (the projection is
    /// followed by a snapshot), so a restart mid-pipeline must bring it back.
    #[tokio::test]
    async fn a_snapshot_job_survives_a_restart() {
        let db = scratch().await;
        let sup = Supervisor::new(db.clone(), "archive".into(), reqwest::Client::new());
        let id = sup.enqueue_request(&req("snapshot")).await.unwrap();
        assert_eq!(id.len(), 1);
        assert_eq!(sup.queued()[0].kind, "snapshot");

        let restarted = Supervisor::new(db.clone(), "archive".into(), reqwest::Client::new());
        restarted.recover().await;
        let q = restarted.queued();
        assert_eq!(q.len(), 1, "the snapshot job is restored");
        assert_eq!((q[0].id, q[0].kind.as_str()), (id[0], "snapshot"), "Spec::Snapshot round-trips");
    }

    /// Issue 32: the resume skip is the period-ordered prefix at or before the
    /// cursor — nothing for a fresh job, everything through the cursor otherwise.
    #[test]
    fn resume_skip_skips_the_completed_prefix() {
        let pkgs: Vec<store::Package> = ["1993-01", "2004-07", "2010-12"]
            .iter()
            .map(|p| store::Package { fetch_id: 1, period: (*p).to_owned(), path: "x".into() })
            .collect();
        assert_eq!(resume_skip(&pkgs, None), 0, "a fresh job walks all");
        assert_eq!(resume_skip(&pkgs, Some("2004-07")), 2, "skip through the cursor (inclusive)");
        assert_eq!(resume_skip(&pkgs, Some("1992-99")), 0, "cursor before the first: skip none");
        assert_eq!(resume_skip(&pkgs, Some("2099-01")), 3, "cursor past the last: skip all");
    }

    /// Issue 32: a process job restored from the durable queue carries its resume
    /// cursor, so a restart continues where it left off; a fresh enqueue never
    /// inherits one, keeping `rebuild`-style full re-walks available.
    #[tokio::test]
    async fn a_process_job_recovers_its_resume_cursor() {
        let db = scratch().await;
        let sup = Supervisor::new(db.clone(), "archive".into(), reqwest::Client::new());
        let ids = sup
            .enqueue_request(&JobRequest {
                kind: "process".into(),
                source: Some("ted".into()),
                ..Default::default()
            })
            .await
            .unwrap();
        // A prior run fully completed packages through 2004-07.
        db.record_job_progress(ids[0] as i64, "2004-07").await.unwrap();

        let restarted = Supervisor::new(db.clone(), "archive".into(), reqwest::Client::new());
        restarted.recover().await;
        {
            let queue = restarted.queue.lock().expect("queue lock");
            assert_eq!(queue.len(), 1);
            assert_eq!(
                queue[0].resume_after.as_deref(),
                Some("2004-07"),
                "the recovered job resumes after the last completed package"
            );
        }

        // A freshly enqueued job has no cursor — it walks from the start.
        let fresh = restarted
            .enqueue_request(&JobRequest {
                kind: "process".into(),
                source: Some("ted".into()),
                ..Default::default()
            })
            .await
            .unwrap();
        let queue = restarted.queue.lock().expect("queue lock");
        let fresh_job = queue.iter().find(|j| j.id == fresh[0]).expect("the fresh job");
        assert!(fresh_job.resume_after.is_none(), "a fresh enqueue never inherits a cursor");
    }

    async fn seed_fetch(db: &store::Db) -> i64 {
        db.record_fetch(&store::Fetch {
            source: "ted".into(),
            kind: "daily".into(),
            period: "p".into(),
            url: "u".into(),
            sha256: "aa".into(),
            bytes: 1,
            fetched_at: 0,
            path: "p".into(),
        })
        .await
        .unwrap();
        db.current_packages("ted", "daily", None).await.unwrap()[0].fetch_id
    }

    /// One minimal single-notice keyed Tender.
    async fn record_keyed(db: &store::Db, fetch_id: i64, n: i64) {
        let parsed = store::Parsed {
            sections: vec![store::Section { id: "PROC".into(), kind: "Procedure".into(), parent: None }],
            values: vec![store::ValueRow {
                section_id: "PROC".into(),
                field_id: "BT-04-notice".into(),
                ordinal: 0,
                value: store::NoticeValue::Id { scheme: None, value: format!("bt04-{n}"), is_ref: false },
            }],
        };
        db.record_notice(
            &store::Notice {
                source: "ted".into(),
                publication_id: format!("pub-{n}"),
                content_hash: format!("h-{n}"),
                profile: "eforms:eforms-sdk-1.13".into(),
                declared_version: None,
                fetch_id,
                member_path: "m".into(),
                ingested_at: 0,
                published_at: Some(0),
                dispatched_at: None,
            },
            &store::Parse::Parsed(parsed),
        )
        .await
        .unwrap();
    }

    /// Issue 58: the daily project job (`rebuild:false`) runs the INCREMENTAL
    /// projection — its summary reports only the delta, not the whole corpus —
    /// while `rebuild:true` still does the full projection.
    #[tokio::test]
    async fn daily_project_job_is_incremental_while_rebuild_is_full() {
        let db = scratch().await;
        let fetch_id = seed_fetch(&db).await;
        for i in 0..3 {
            record_keyed(&db, fetch_id, i).await;
        }
        let sup = Supervisor::new(db.clone(), "archive".into(), reqwest::Client::new());

        // A rebuild folds the whole corpus (3 notices) and resets the watermark.
        let rebuild = Job {
            id: 1,
            kind: "project".into(),
            params: "rebuild=true".into(),
            spec: Spec::Project { rebuild: true, clear_changes: false },
            resume_after: None,
        };
        let full = sup.run_spec(&rebuild).await.unwrap();
        assert!(full.starts_with("3 notices"), "rebuild folds the whole corpus: {full}");

        // A daily delta of one notice — the daily job re-derives only its Tender.
        record_keyed(&db, fetch_id, 3).await;
        let daily = Job {
            id: 2,
            kind: "project".into(),
            params: "rebuild=false".into(),
            spec: Spec::Project { rebuild: false, clear_changes: false },
            resume_after: None,
        };
        let incr = sup.run_spec(&daily).await.unwrap();
        assert!(incr.starts_with("1 notices"), "daily job is incremental (delta only): {incr}");
        assert_eq!(db.unprojected_parsed_notice_ids().await.unwrap().len(), 0, "change-set drained");
    }

    async fn index_exists(db: &store::Db, name: &str) -> bool {
        matches!(
            db.scalar(&format!("SELECT 1 FROM sqlite_master WHERE type='index' AND name='{name}'"))
                .await
                .unwrap(),
            Some(store::turso::Value::Integer(1))
        )
    }

    /// Issue 60 salvage: a recovered project job — even `rebuild:false` — RESUMES
    /// an interrupted rebuild (skips Phase-1) rather than routing to the incremental
    /// path and re-scanning the whole corpus. The salvage signal is the durable
    /// `rebuild_in_progress` flag (a real rebuild sets it before Phase-1), NOT merely
    /// a complete plan on disk. Proven by the org indexes: `project_plan_only` strips
    /// them; only the resume path (project(_, true)) rebuilds them at the end —
    /// `project_incremental` never does.
    #[tokio::test]
    async fn a_recovered_job_resumes_an_interrupted_rebuild_even_when_not_a_rebuild() {
        let db = scratch().await;
        let fetch_id = seed_fetch(&db).await;
        for i in 0..4 {
            record_keyed(&db, fetch_id, i).await;
        }
        // Interrupt an in-flight REBUILD after Phase-1: a real rebuild sets the
        // in-progress flag before Phase-1 (with reset_tender_layer), then builds the
        // plan; here project_plan_only leaves the complete plan + stripped org indexes
        // and we set the flag to represent that the interrupted run was a rebuild.
        ingest::project::project_plan_only(&db).await.unwrap();
        db.set_rebuild_in_progress().await.unwrap();
        assert!(db.plan_is_complete().await.unwrap(), "plan complete after plan-only");
        assert!(!index_exists(&db, "organizations_identity").await, "plan-only strips the org index");

        let sup = Supervisor::new(db.clone(), "archive".into(), reqwest::Client::new());
        // The recovered daily job is rebuild:FALSE — but the in-progress flag must
        // make it resume, not re-scan.
        let daily = Job {
            id: 1,
            kind: "project".into(),
            params: "rebuild=false".into(),
            spec: Spec::Project { rebuild: false, clear_changes: false },
            resume_after: None,
        };
        let summary = sup.run_spec(&daily).await.unwrap();

        assert!(index_exists(&db, "organizations_identity").await, "resume rebuilds the org index (Phase-2 ran via project(true), not incremental)");
        assert!(!db.plan_is_complete().await.unwrap(), "the resume cleared the plan when done");
        assert!(!db.rebuild_in_progress().await.unwrap(), "the resume cleared the in-progress flag");
        assert!(summary.starts_with("4 notices"), "resume folded the whole plan: {summary}");
    }

    /// Salvage-loop regression (issue 61 incident): a COMPLETE plan on disk with the
    /// `rebuild_in_progress` flag UNSET — the resting state of a FINISHED build whose
    /// plan lingered, or of an interrupted rebuild=false full-fallback over an intact
    /// layer — must NOT trigger the layer-nuking resume. The recovered rebuild:false
    /// job routes to the incremental path instead (org index NOT rebuilt), and the
    /// flag stays clear. Before the fix, salvage keyed on `plan_is_complete()` and
    /// re-fired reset_tender_layer on every restart forever.
    #[tokio::test]
    async fn a_complete_plan_without_the_flag_does_not_re_salvage() {
        let db = scratch().await;
        let fetch_id = seed_fetch(&db).await;
        for i in 0..4 {
            record_keyed(&db, fetch_id, i).await;
        }
        // A complete plan on disk, but the flag is UNSET (no interrupted rebuild).
        ingest::project::project_plan_only(&db).await.unwrap();
        assert!(db.plan_is_complete().await.unwrap(), "plan complete after plan-only");
        assert!(!db.rebuild_in_progress().await.unwrap(), "no rebuild is in progress");

        let sup = Supervisor::new(db.clone(), "archive".into(), reqwest::Client::new());
        let daily = Job {
            id: 1,
            kind: "project".into(),
            params: "rebuild=false".into(),
            spec: Spec::Project { rebuild: false, clear_changes: false },
            resume_after: None,
        };
        sup.run_spec(&daily).await.unwrap();

        // Incremental was taken, NOT the project(true) salvage: the org identity index
        // (only ever rebuilt by the full resume path) stays absent, and the flag never
        // flips. No layer-nuking resume fired.
        assert!(
            !index_exists(&db, "organizations_identity").await,
            "a flagless complete plan must route to incremental, not the org-index-rebuilding resume"
        );
        assert!(!db.rebuild_in_progress().await.unwrap(), "the flag stays clear — no spurious salvage");
    }

    /// Issue 61: a job's body runs on the Supervisor's isolated runtime, off the
    /// main API/SSE/dashboard runtime — the whole point of the fix. A task
    /// submitted to `worker_runtime` executes on a `job-exec` thread, not the
    /// test's own runtime threads, so a job's blocking turso preads can never pin
    /// a main worker.
    #[tokio::test]
    async fn a_job_runs_on_the_isolated_runtime() {
        let sup = Supervisor::new(scratch().await, "archive".into(), reqwest::Client::new());
        let thread_name = sup
            .worker_runtime
            .spawn(async { std::thread::current().name().unwrap_or_default().to_owned() })
            .await
            .unwrap();
        assert!(
            thread_name.starts_with("job-exec"),
            "a job runs on the isolated runtime's threads, got {thread_name:?}"
        );
    }

    /// Issue 61: the worker loop still executes a queued job correctly through the
    /// isolated runtime — it folds the corpus, records an `ok` run, and clears the
    /// durable row (the recovery contract is unchanged by the runtime hop).
    #[tokio::test]
    async fn spawn_worker_runs_a_job_through_the_isolated_runtime() {
        let db = scratch().await;
        let fetch_id = seed_fetch(&db).await;
        for i in 0..3 {
            record_keyed(&db, fetch_id, i).await;
        }
        let sup = Arc::new(Supervisor::new(db.clone(), "archive".into(), reqwest::Client::new()));
        sup.enqueue_request(&req("project")).await.unwrap();
        sup.clone().spawn_worker();

        // Wait for the durable queue to drain: execute() ran on the isolated
        // runtime and removed the finished job's row.
        for _ in 0..200 {
            if db.pending_jobs().await.unwrap().is_empty() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        assert!(
            db.pending_jobs().await.unwrap().is_empty(),
            "the worker executed the job on the isolated runtime and cleared its durable row"
        );
        let runs = db.recent_job_runs(5).await.unwrap();
        assert!(
            runs.iter().any(|r| r.kind == "project" && r.outcome == "ok"),
            "the project job logged an ok run: {runs:?}"
        );
    }

    /// A default reprocess enqueues the reclaim + a trailing incremental fold;
    /// `reclaim_only` (bulk-recovery mode) enqueues only the reclaim, leaving its
    /// members `projected=0` for one later `rebuild:true`.
    #[tokio::test]
    async fn reprocess_reclaim_only_omits_the_trailing_project() {
        let db = scratch().await;
        let sup = Supervisor::new(db, "archive".into(), reqwest::Client::new());

        let full = sup
            .enqueue_request(&JobRequest {
                kind: "reprocess".into(),
                reason: Some("unknown-field-code".into()),
                detail_like: Some("%: OC".into()),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(full.len(), 2, "reprocess enqueues the reclaim + a trailing project");

        let bulk = sup
            .enqueue_request(&JobRequest {
                kind: "reprocess".into(),
                reason: Some("unknown-field-code".into()),
                reclaim_only: Some(true),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(bulk.len(), 1, "reclaim_only enqueues only the reclaim, no fold");
    }
}
