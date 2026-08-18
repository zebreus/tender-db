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

use ingest::{doe, fetch, process, project, ted};
use model::ingestion::{Ingestion, JobProgress, Phase, QueuedJob};
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
            // Issue 111: notice a missing deferred index and ask the existing Reindex
            // job to build it. After `recover`, so an already-queued Reindex is seen.
            sup.ensure_deferred_indexes().await;
            sup.clone().spawn_worker();
            sup.clone().spawn_scheduler();
            sup.clone().spawn_report_scheduler();
            sup.clone().spawn_presence_observer();
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
    /// Monotonic count of CONCLUDED jobs (ok or error). The dashboard's
    /// change-gate folds this into its watermark: a reprocess stamps quarantine
    /// rows and flips parse states IN PLACE — no new notice id, no fetch row, no
    /// change-log cursor movement — so the (cursor, newest-fetch, newest-notice)
    /// key alone reads "nothing happened" and the heavy sections are never
    /// re-measured. That served an hours-stale quarantine panel under a fresh
    /// `measured_at` and cost issue 139 a diagnosis morning (issue 191).
    jobs_completed: AtomicU64,
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
/// Rows marked per transaction by [`Spec::MarkSkippedSiblings`]. Small on
/// purpose: turso writes a WAL frame per row and cannot checkpoint mid-statement,
/// so one ~593k-row UPDATE is the shape that produced a 127 GB WAL and an OOM
/// during the recovery. The job checkpoints between batches.
const MARK_BATCH: i64 = 5_000;

/// Tenders per `backfill-deadlines` transaction (issue 216). Rows are small and
/// the per-row work is one indexed `(tender_id, seq)` seek into the dates
/// satellite, so the batch can be larger than [`MARK_BATCH`]'s wide quarantine
/// rows while keeping each WAL transaction bounded.
const BACKFILL_BATCH: i64 = 10_000;

/// Span of `tender_versions.tender_id` measured per data-quality window (issue
/// 230). The size is chosen against the only numbers anyone actually measured —
/// the field probes ran 0.37 s over 50k tenders, 1.25 s over 200k and 4.79 s over
/// 800k, all cache-hot — so 250k sits an order of magnitude inside the largest
/// timed window and leaves room for the cold case that made the two unwindowed
/// runs unkillable. It is a tuning knob, not a law: every window logs its own
/// elapsed, so resizing it is a decision the journal can support.
const DQ_WINDOW: i64 = 250_000;

/// Most notice ids `refold-notices` will accept (issue 58 v2, step 3).
///
/// The job exists to make a legacy fold observable at a size a person can reason
/// about, so its guard is a cap rather than the `expect`-with-slack the derived
/// refolds use: slack is a statistical guard on a cohort nobody enumerated, and it
/// says nothing about a list that was typed. 1,000 is well above any exerciser
/// (step 3's is a handful) and far below any cohort worth a real refold — a list
/// that big means someone reached for the wrong job, and being told so beats being
/// obeyed.
const REFOLD_NOTICES_CAP: usize = 1_000;

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
    /// Re-parse a profile cohort's already-parsed notices from the archive
    /// against the current parser (issue 100). Resume rides on the job row's
    /// `resume_after` like `process`/`reprocess`, not in the spec.
    Reparse { profiles: Vec<String> },
    /// Run the semantic data-quality measurement and store its report (issue 230).
    /// `confirmed` is the operator's explicit "yes, tie the queue up for this" —
    /// see the enqueue arm.
    DataQuality { confirmed: bool },
    /// Re-derive the canonical layer. `clear_changes` (rebuild only, issue 81)
    /// DROP+recreates the CDC feed first, so the rebuild re-emits ONE clean
    /// generation for the recovered baseline instead of appending. `#[serde(default)]`
    /// keeps pre-flag durable job rows deserializable.
    Project {
        rebuild: bool,
        #[serde(default)]
        clear_changes: bool,
    },
    /// Rebuild any missing DEFERRED_TENDER_INDEXES / org indexes on the existing
    /// layer WITHOUT re-folding (issues 82/83): `build_tender_indexes` only runs at a
    /// rebuild's end, and a tmpfs-truncated run can leave the tender indexes partial,
    /// dropping the tenders list to a full scan. Idempotent (`CREATE INDEX IF NOT
    /// EXISTS` loops), so it builds only what's missing. A unit variant → durable.
    Reindex,
    /// Rebuild the `fetches` registry from the on-disk archive (issue 23 / the DR
    /// premise finding): after a DB loss with the archive intact, this is what
    /// lets `process` run with zero re-downloads. Idempotent — known periods are
    /// skipped without hashing.
    RegisterArchive,
    /// Unicode-lowercase every org name into `name_norm` (issue 217-B) — the
    /// batched backfill behind the name-prefix search. Idempotent (stamped rows
    /// are skipped), durable like its siblings.
    BackfillOrgNames,
    /// Populate the durable legacy OJS adjacency for the standing corpus and
    /// establish its coverage watermark (issue 58 v2, step 2). Batched +
    /// checkpointed, idempotent (INSERT OR IGNORE), so a restart re-runs from
    /// zero at worst. A unit variant → durable across restarts.
    BackfillLegacyAdjacency,
    /// Stamp `tenders.current_deadline` from each head version's dates (issue 216,
    /// deadline half). Batched + checkpointed like the mark job (issue 42);
    /// idempotent, so a restart redoes the walk from zero at worst. A unit
    /// variant → durable across restarts like `Reindex`.
    BackfillDeadlines,
    /// Re-queue a profile cohort for the incremental fold (issue 85): clear its
    /// `projected` watermark so the trailing `project rebuild=false` re-derives just
    /// those Tenders. For a cohort the projection MIS-READ (unmapped field ids) —
    /// the parsed layer is already correct, so no re-parse. `expect` is a guard: the
    /// run aborts BEFORE writing if the cohort is not about that size, which is what
    /// a mistyped profile string looks like.
    Refold { profiles: Vec<String>, expect: Option<u64> },
    /// Re-fold an EXPLICIT, small notice-id list (issue 58 v2, step 3's exerciser).
    ///
    /// `refold` and `refold-fields` both derive their cohort, and both derive one
    /// that is far too large to use as a test: the smallest legacy profile is tens
    /// of thousands of notices, and step 3's whole question is what the incremental
    /// projection's legacy CLOSURE WALK does when a legacy delta arrives — a
    /// question that needs a delta of five notices, not fifty thousand. The daily
    /// fold cannot answer it either: its journal reads `legacy-update: 0.0s (0
    /// legacy)` on an ordinary day, because eForms-only days never touch a legacy
    /// chain at all.
    ///
    /// So the cohort here is named, not derived. That makes the size guard
    /// structural rather than statistical (`expect`-style slack is meaningless for
    /// a list you typed): the list is capped, and a list over the cap is refused.
    RefoldNotices { notices: Vec<i64> },
    /// Re-project the notices whose PARSE LAYER carries any of these field ids —
    /// the FIELD-scoped refold (issue 88 follow-up): a new mapping for a grafted
    /// id affects exactly its carriers, a set no profile names. Sweeps the value
    /// tables (bounded windows), then requeues the carriers and stamps their
    /// tenders epoch-stale (issue 179's scoped-staleness pair). `expect` guards
    /// like `Refold`'s: abort before any write if the cohort size surprises.
    RefoldFields { fields: Vec<String>, expect: Option<u64> },
    /// Issue 84: mark the 2008 per-language duplicate siblings as
    /// skipped-by-policy, so the outstanding count stops reporting ~593k rows of
    /// work that no reprocess can ever do. `dry_run` counts and writes nothing.
    ///
    /// `expect` is the abort-before-write guard, and the run is authorised against
    /// it: the population was verified independently (section B, 5000/5000), so a
    /// count that disagrees means the predicate and the verified set have diverged
    /// and the job must stop rather than write a set nobody checked. A SHORTFALL is
    /// specifically NOT a reason to widen the predicate — a held non-English row
    /// whose English original did not parse is HELD-BUT-UNEXTRACTED — we have the
    /// bytes and failed to read them — and marking it as a duplicate is the one
    /// outcome worse than the overstated count. Deliberately not "lost": that word
    /// mis-frames a parse defect as a data defect, and it is the reasoning shape
    /// that justifies keeping backups forever (team-lead's correction, issue 138
    /// criterion 3).
    MarkSkippedSiblings {
        #[serde(default)]
        dry_run: bool,
        expect: Option<u64>,
        /// How many in-scope rows the sibling guard is EXPECTED to reject.
        ///
        /// `None` means "none at all" — the original, strictest rule. Supplying a
        /// number does not loosen the guard, it re-aims it: the run still aborts
        /// unless the rejected set is exactly this size, so a population that has
        /// shifted by a single row since it was investigated stops the run.
        #[serde(default)]
        expect_gaps: Option<u64>,
    },
    /// Issue 190: restore to OUTSTANDING the sibling rows a guard-free flag pass
    /// swept into skipped-by-policy — rows in the 2008 sibling scope, marked
    /// skipped, whose English original is missing or unparsed. The inverse repair
    /// of [`Spec::MarkSkippedSiblings`]'s guard, and self-limiting: once the
    /// originals parse, the guard accepts these rows and the repair matches
    /// nothing. `dry_run` counts and writes nothing.
    RepairSweptSiblings {
        #[serde(default)]
        dry_run: bool,
    },
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
    /// `fetch` | `process` | `project` | `backfill` | `daily`
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
    /// `mark-skipped-siblings` only: count without writing. **Defaults to TRUE
    /// when omitted** — for a job that writes ~593k rows, a forgotten flag must
    /// mean the harmless thing, not the destructive one.
    pub dry_run: Option<bool>,
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
    /// `refold-notices` only: the explicit notice ids to re-fold (issue 58 v2's
    /// step-3 exerciser). Capped — see [`REFOLD_NOTICES_CAP`].
    pub notices: Option<Vec<i64>>,
    /// `mark-skipped-siblings` execute only: the number of in-scope rows the
    /// sibling guard is EXPECTED to reject (issue 84, the 154).
    ///
    /// Omitted, the guard's rejection set must be empty — the original, strictest
    /// rule. Supplying it does not loosen the guard, it re-aims it: the run still
    /// aborts unless the rejected set is exactly this size, so a population that
    /// has shifted by even one row since it was investigated stops the run.
    /// Naming the number is the whole point — an operator has to state what they
    /// already know is there, and cannot proceed past a set they have not looked at.
    pub expect_gaps: Option<u64>,
}

impl Supervisor {
    /// The store handle, for read-only operator surfaces that need it (the stored
    /// report reader, issue 230). An accessor rather than a public field on purpose:
    /// a caller can read, and the queue stays the only way to make the supervisor
    /// write.
    pub fn db(&self) -> &store::Db {
        &self.db
    }

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
            jobs_completed: AtomicU64::new(0),
            worker_runtime: spawn_worker_runtime(),
        }
    }

    /// How many jobs have concluded since boot — the dashboard change-gate's
    /// signal that job-shaped writes (which move no cursor and add no rows) may
    /// have changed what the heavy sections display. See the field doc.
    pub fn jobs_completed(&self) -> u64 {
        self.jobs_completed.load(Ordering::Relaxed)
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
            // Rebuild any missing deferred indexes on the existing layer, no re-fold
            // (issues 82/83). Safe to fire repeatedly (idempotent).
            "reindex" => Ok(vec![self.push("reindex", "reindex".into(), Spec::Reindex).await]),
            // Rebuild the fetches registry from the on-disk archive (issue 23):
            // idempotent, hash-only-what's-missing, so safe to fire any time.
            "register-archive" => Ok(vec![
                self.push("register-archive", "register-archive".into(), Spec::RegisterArchive)
                    .await,
            ]),
            "backfill-org-names" => Ok(vec![
                self.push("backfill-org-names", "backfill-org-names".into(), Spec::BackfillOrgNames)
                    .await,
            ]),
            // Sweep the standing legacy corpus into `legacy_ojs_keys` and establish
            // the coverage watermark (issue 58 v2, step 2) — batched, checkpointed,
            // idempotent. One-off after the table ships; the plan builds maintain
            // it from then on.
            // Re-parse a PROFILE cohort's already-parsed notices from the archive
            // against the current parser, then fold what changed (issue 100). The
            // `reprocess` twin for the case where the notices are fine and the
            // PARSER changed — reprocess cannot serve it (it walks quarantine rows)
            // and `refold` cannot either (it re-folds the existing parse rows).
            // Profiles ride in the request's `profiles` list, like `refold`.
            "reparse" => {
                let profiles = req.profiles.clone().unwrap_or_default();
                if profiles.is_empty() {
                    return Err("reparse needs at least one profile".into());
                }
                let params = format!("reparse {}", profiles.join(","));
                let mut ids = vec![
                    self.push("reparse", params, Spec::Reparse { profiles })
                        .await,
                ];
                // Re-parsed notices land `projected = 0`, so an ordinary incremental
                // projection folds them — no `refold` needed. `reclaim_only` skips
                // it for a bulk run that would rather pay one sequential rebuild.
                if !req.reclaim_only.unwrap_or(false) {
                    ids.push(
                        self.push(
                            "project",
                            "rebuild=false".into(),
                            Spec::Project { rebuild: false, clear_changes: false },
                        )
                        .await,
                    );
                }
                Ok(ids)
            }
            // Measure semantic data quality and store the rendered report (issue
            // 230). A JOB, not a refresher section and not an external tool: the
            // eleven aggregates each blow through the /v1/sql 10s cap at
            // full-corpus scale, and they are far too expensive for the
            // dashboard's 60s cadence. As a job they are queue-serialized, run on
            // the reader pool, carry a phase record, and land in the job log.
            // The measurement is a ~10 minute full-corpus pass that holds the
            // queue (jobs are serialized), so it asks to be meant: `dry_run`
            // defaults to TRUE and a dry run only reports what it would do. Same
            // safe-default convention as `mark-skipped-siblings` — a forgotten
            // flag must mean the harmless thing.
            "data-quality" => {
                let confirmed = !req.dry_run.unwrap_or(true);
                let params =
                    if confirmed { "data-quality" } else { "data-quality dry-run" }.to_owned();
                Ok(vec![self.push("data-quality", params, Spec::DataQuality { confirmed }).await])
            }
            "backfill-legacy-adjacency" => Ok(vec![
                self.push(
                    "backfill-legacy-adjacency",
                    "backfill-legacy-adjacency".into(),
                    Spec::BackfillLegacyAdjacency,
                )
                .await,
            ]),
            // Stamp every tender's current_deadline from its head version's dates
            // (issue 216, deadline half) — batched, checkpointed, idempotent. One-off
            // after the column ships; the fold maintains it from then on.
            "backfill-deadlines" => Ok(vec![
                self.push("backfill-deadlines", "backfill-deadlines".into(), Spec::BackfillDeadlines)
                    .await,
            ]),
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
            // The FIELD-scoped twin (issue 88 follow-up): re-project the notices
            // whose parse layer carries these field ids. Reuses the request's
            // `profiles` list as the field-id list — one list-shaped parameter per
            // request, keyed by kind. Paired with a projection like `refold`.
            "refold-fields" => {
                let fields = req.profiles.clone().unwrap_or_default();
                if fields.is_empty() {
                    return Err("refold-fields needs at least one field id (pass via profiles)".into());
                }
                let params = format!("refold-fields {}", fields.join(","));
                Ok(vec![
                    self.push("refold-fields", params, Spec::RefoldFields { fields, expect: req.expect })
                        .await,
                    self.push("project", "rebuild=false".into(), Spec::Project { rebuild: false, clear_changes: false })
                        .await,
                ])
            }
            // The NAMED-ids twin (issue 58 v2, step 3's exerciser): re-fold exactly
            // these notices. Paired with a projection like the other two refolds, so
            // the fold that follows is an ordinary incremental one — which is the
            // point, since the behaviour under test is what that fold does with a
            // legacy delta.
            "refold-notices" => {
                let notices = req.notices.clone().unwrap_or_default();
                if notices.is_empty() {
                    return Err("refold-notices needs at least one notice id".into());
                }
                if notices.len() > REFOLD_NOTICES_CAP {
                    return Err(format!(
                        "refold-notices refuses {} ids (cap {REFOLD_NOTICES_CAP}) — a list this \
                         long is a cohort, and a cohort wants `refold` or `refold-fields`",
                        notices.len()
                    ));
                }
                // The ids go in the params line, not just the spec, so the job log
                // records WHICH notices a run touched. A run whose effect cannot be
                // attributed afterwards is not much of an experiment.
                let shown: Vec<String> = notices.iter().take(8).map(i64::to_string).collect();
                let params = format!(
                    "refold-notices {}{}",
                    shown.join(","),
                    if notices.len() > shown.len() {
                        format!(" (+{} more)", notices.len() - shown.len())
                    } else {
                        String::new()
                    }
                );
                Ok(vec![
                    self.push("refold-notices", params, Spec::RefoldNotices { notices }).await,
                    self.push("project", "rebuild=false".into(), Spec::Project { rebuild: false, clear_changes: false })
                        .await,
                ])
            }
            // Issue 84: mark the 2008 language siblings skipped-by-policy. NOT
            // paired with a projection — this touches only quarantine bookkeeping,
            // no notice enters or leaves the corpus, so there is nothing to fold.
            // `dry_run` is the default when the caller omits it: the destructive
            // reading of a missing flag must be the safe one.
            "mark-skipped-siblings" => {
                let dry_run = req.dry_run.unwrap_or(true);
                let params = if dry_run { "dry-run".to_owned() } else { "execute".to_owned() };
                Ok(vec![
                    self.push(
                        "mark-skipped-siblings",
                        params,
                        Spec::MarkSkippedSiblings {
                            dry_run,
                            expect: req.expect,
                            expect_gaps: req.expect_gaps,
                        },
                    )
                    .await,
                ])
            }
            // Issue 190: clear the skipped marks a guard-free flag pass swept onto
            // protected siblings. Quarantine bookkeeping only — nothing to fold.
            // Same safe default as the marker: a missing flag means dry-run.
            "repair-swept-siblings" => {
                let dry_run = req.dry_run.unwrap_or(true);
                let params = if dry_run { "dry-run".to_owned() } else { "execute".to_owned() };
                Ok(vec![
                    self.push("repair-swept-siblings", params, Spec::RepairSweptSiblings { dry_run })
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

    /// Issue 111: notice at startup that a deferred index is missing, and ask the
    /// existing `Reindex` job to build it.
    ///
    /// The deferred indexes are created at a rebuild's end or when an operator fires
    /// `Reindex`, and at no other time — so a deploy that ADDS one leaves it
    /// uncreated, and the read it exists for stays slow until somebody notices. That
    /// is how issue 117's DoS fix would have shipped without taking effect.
    ///
    /// Detection is one `sqlite_master` scan (one row per object, not per row of
    /// data), and the BUILD does not happen here: it is enqueued and runs on the
    /// worker after the service is up. Building at boot is the multi-hour start-up
    /// issues 82/83 removed, and `notices(source, id)` alone would be ~7 minutes.
    ///
    /// Skipped when a `Reindex` is already queued, so a restart loop cannot stack
    /// them. The job itself is idempotent — `CREATE INDEX IF NOT EXISTS` loops — so a
    /// redundant one is harmless, just wasteful.
    /// Called by [`init`] AFTER [`recover`], never from inside it.
    ///
    /// `recover` restores the durable queue and does nothing else; this ADDS a job.
    /// Folding an enqueue into a restore made recovery's own tests fail — they assert
    /// the queue contains exactly what was persisted, and on a fresh database every
    /// deferred index is missing, so recovery silently gained a fifth job. That was a
    /// real design smell caught by a real test, and the separation is the fix rather
    /// than an accommodation: a function named for restoring state should not create
    /// any.
    pub(crate) async fn ensure_deferred_indexes(&self) {
        let reindex_already_queued = self
            .queue
            .lock()
            .expect("queue lock")
            .iter()
            .any(|j| matches!(j.spec, Spec::Reindex));
        let missing = match self.db.missing_deferred_indexes().await {
            Ok(m) => m,
            Err(e) => {
                eprintln!("supervisor: check deferred indexes: {e}");
                return;
            }
        };
        if missing.is_empty() {
            return;
        }
        if reindex_already_queued {
            eprintln!(
                "supervisor: {} deferred index(es) missing ({}); a reindex is already queued",
                missing.len(),
                missing.join(", ")
            );
            return;
        }
        eprintln!(
            "supervisor: {} deferred index(es) missing ({}) — queueing a background reindex",
            missing.len(),
            missing.join(", ")
        );
        self.push("reindex", format!("auto: {}", missing.join(", ")), Spec::Reindex).await;
    }

    // ---------------------------------------------------------------- progress

    fn set_current(&self, progress: Option<JobProgress>) {
        *self.current.write().expect("progress lock") = progress;
    }

    /// The running job's live progress, cloned under the read lock — the cheap
    /// synchronous view `/metrics` scrapes (issue 65). `ingestion()` is the full
    /// snapshot (queue + persisted runs, a DB read); a scrape must not pay that.
    pub fn current_progress(&self) -> Option<JobProgress> {
        self.current.read().expect("progress lock").clone()
    }

    fn update<F: FnOnce(&mut JobProgress)>(&self, f: F) {
        if let Some(p) = self.current.write().expect("progress lock").as_mut() {
            f(p);
        }
    }

    /// The projection's [`Progress`] events → the durable phase record (issue
    /// 65). One arm per variant, each keeping the event's own counts and units —
    /// no unit ever borrows another's field, the issue-228 rule.
    fn phase_from_progress(&self, p: ingest::project::Progress) {
        use ingest::project::Progress;
        match p {
            Progress::Planning { notices, total } => {
                self.set_phase("planning", Some(notices), Some(total), "notices planned".into());
            }
            // No total, honestly: the sweep is bounded by an id RANGE, and
            // counting its rows up front would pay the very scan the pre-pass
            // exists to do once. Movement alone is the signal (issue 228).
            Progress::PrePass { notices } => {
                self.set_phase("pre-pass", Some(notices), None, "notices swept into buckets".into());
            }
            Progress::Grouped { tenders, islands } => {
                self.set_phase(
                    "folding",
                    Some(0),
                    Some(tenders),
                    format!("plan grouped: {tenders} tenders, {islands} single-notice islands"),
                );
            }
            Progress::Applying { tenders, total, versions } => {
                self.set_phase(
                    "folding",
                    Some(tenders),
                    Some(total),
                    format!("{versions} version rows written"),
                );
            }
        }
    }

    /// Record what the running job is doing now (issue 65). `done`/`total` are
    /// optional because a phase that cannot cheaply know its end still shows
    /// movement from `done` alone — which is what separates a working job from a
    /// wedged one, the distinction issue 228 cost 40 minutes of doubt over.
    ///
    /// Stamps `updated_at` here rather than at the call site so every phase
    /// carries a truthful "last heard from" instant: a reporter that stops
    /// leaves a stale stamp, which reads differently from a slow phase.
    fn set_phase(&self, name: &str, done: Option<u64>, total: Option<u64>, detail: String) {
        let phase =
            Phase { name: name.to_owned(), done, total, detail, updated_at: store::now_unix() };
        self.update(|p| p.phase = Some(phase));
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
                matches!(
                    p.kind.as_str(),
                    "process" | "project" | "reprocess" | "reindex" | "refold" | "refold-fields"
                )
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
            phase: None,
        }));

        let result = self.run_spec(&job).await;
        self.set_current(None);
        self.jobs_completed.fetch_add(1, Ordering::Relaxed);

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
            // Bounded at last (issue 230): the two killed runs are recorded on
            // `run_data_quality`, which now measures over id windows instead of
            // over the whole corpus in one statement.
            Spec::DataQuality { confirmed } => self.run_data_quality(*confirmed).await,
            Spec::Reparse { profiles } => {
                self.run_reparse(job.id, profiles, job.resume_after.as_deref()).await
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
                    // Observed (issue 65): the projection's Progress events feed
                    // the durable phase record, so /admin/jobs shows planning /
                    // pre-pass / folding instead of dead air for the multi-hour
                    // phases. The journal keeps its heartbeats — project_observed
                    // composes the stderr sink with this mapping, one stream.
                    project::project_observed(&self.db, true, |p| self.phase_from_progress(p))
                        .await
                } else {
                    // The incremental daily is seconds-to-minutes; its scoping
                    // already logs its own decisions (the issue-58 closure line
                    // or a named fallback). No phase record until one is earned.
                    project::project_incremental(&self.db).await
                }
                .map_err(|e| e.to_string())?;
                self.update(|p| p.notices = report.notices);
                // The written/unchanged split (issue 108) rides in the durable
                // counts line: a later G2 breach can then be read against what
                // each fold actually did — "written 0 / unchanged N" points at
                // the watermark over-claiming, a large `written` at the fold.
                Ok(format!(
                    "{} notices → {} tenders ({} islands), {} versions; {} tenders written, {} verified unchanged",
                    report.notices,
                    report.tenders,
                    report.islands,
                    report.applied.versions_written,
                    report.applied.tenders_written,
                    report.applied.tenders_unchanged
                ))
            }
            Spec::Reindex => {
                // Both builders are CREATE INDEX IF NOT EXISTS loops — idempotent, so
                // this rebuilds only the missing deferred indexes without touching the
                // fold. Pair the TRUNCATE checkpoint to reclaim the build's WAL tail,
                // exactly as the rebuild's end-of-fold index build does (project.rs).
                // SEQUENTIALLY, and that is a hard constraint rather than style:
                // run-driver measured turso's CREATE INDEX peak RSS as LINEAR in row
                // count (~45 B/row — 366 MiB at 8.13M rows, 1.07 GB at 25.3M, no spill
                // threshold between them). Concurrent builds add their peaks, so
                // organizations + notices together would be ~2.3 GB against a ~4 GB
                // bounded-memory ceiling on a box still carrying issue 57's swap
                // band-aid. One at a time.
                self.db.build_organization_indexes().await.map_err(|e| e.to_string())?;
                self.db.build_tender_indexes().await.map_err(|e| e.to_string())?;
                self.db.build_notice_indexes().await.map_err(|e| e.to_string())?;
                let _ = self.db.checkpoint(store::CheckpointMode::Truncate).await;
                Ok("deferred org + tender + notice indexes rebuilt".into())
            }
            Spec::RegisterArchive => {
                let done = ingest::fetch::register_archive(&self.db, &self.archive)
                    .await
                    .map_err(|e| e.to_string())?;
                let _ = self.db.checkpoint(store::CheckpointMode::Truncate).await;
                Ok(format!(
                    "archive re-registered: {} package(s) hashed+recorded, {} period(s) already \
                     known (skipped unhashed), {} unrecognised entr(ies)",
                    done.registered, done.existing, done.unrecognised
                ))
            }
            Spec::BackfillOrgNames => {
                // The deadline backfill's shape (issue 42: bounded batches,
                // TRUNCATE checkpoints), over organizations.
                let mut stamped = 0i64;
                let mut watermark = 0i64;
                loop {
                    let (rows, next) = self
                        .db
                        .backfill_org_name_norm(BACKFILL_BATCH, watermark)
                        .await
                        .map_err(|e| e.to_string())?;
                    if rows == 0 {
                        break;
                    }
                    stamped += rows;
                    watermark = next;
                    self.update(|p| p.members_done = stamped as u64);
                    if let Err(e) = self.db.checkpoint(store::CheckpointMode::Truncate).await {
                        eprintln!("supervisor: checkpoint after org-name batch: {e}");
                    }
                }
                Ok(format!("name_norm stamped over {stamped} organizations"))
            }
            Spec::BackfillLegacyAdjacency => {
                // The org-names shape: the sweep batches and checkpoints itself
                // (issue 42). `members_done` keeps its established meaning — a
                // count, here of legacy notices found — and the id-window
                // position rides in the phase record instead (issues 228 + 65).
                // That split is the whole point: through the eForms tail the
                // count is motionless while the cursor climbs, so "working" and
                // "wedged" stop looking the same from the outside.
                let done = ingest::project::backfill_legacy_adjacency(&self.db, |t| {
                    self.update(|p| p.members_done = t.swept);
                    self.set_phase(
                        "sweeping",
                        Some(t.cursor.max(0) as u64),
                        Some(t.target.max(0) as u64),
                        format!(
                            "notice id {} of {} scanned; {} legacy notices found",
                            t.cursor, t.target, t.swept
                        ),
                    );
                })
                .await
                .map_err(|e| e.to_string())?;
                let _ = self.db.checkpoint(store::CheckpointMode::Truncate).await;
                Ok(format!(
                    "legacy adjacency backfilled: {} legacy notices swept, {} key rows offered, \
                     watermark established at {}",
                    done.swept, done.keys, done.watermark
                ))
            }
            Spec::BackfillDeadlines => {
                // Walk the whole tenders table in id order, one bounded batch per
                // transaction, WAL-checkpointing between batches (issue 42) — the
                // mark job's shape. Progress surfaces as members_done so the
                // dashboard shows the walk moving.
                let mut stamped = 0i64;
                let mut watermark = 0i64;
                loop {
                    let (rows, next) = self
                        .db
                        .backfill_current_deadline(BACKFILL_BATCH, watermark)
                        .await
                        .map_err(|e| e.to_string())?;
                    if rows == 0 {
                        break;
                    }
                    stamped += rows;
                    watermark = next;
                    self.update(|p| p.members_done = stamped as u64);
                    if let Err(e) = self.db.checkpoint(store::CheckpointMode::Truncate).await {
                        eprintln!("supervisor: checkpoint after deadline batch: {e}");
                    }
                }
                Ok(format!(
                    "current_deadline stamped over {stamped} tenders (head-version \
                     submission_deadline; NULL where none is published)"
                ))
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
                // issue 179: the requeue alone leaves each Tender's chain identical,
                // and an unchanged chain with a current epoch early-returns — the
                // mapping fix would never land. Stamp the cohort's tenders
                // epoch-stale so exactly THEY rewrite; the global PROJECTION_EPOCH
                // stays put, so nobody else does. Unconditional (not gated on
                // requeued > 0) so a job re-run after a crash heals both halves.
                let stamped =
                    self.db.stamp_stale_for_profiles(&refs).await.map_err(|e| e.to_string())?;
                Ok(format!(
                    "re-queued {requeued} notices, stamped {stamped} tenders epoch-stale \
                     for the incremental fold"
                ))
            }
            Spec::RefoldNotices { notices } => {
                let requeued =
                    self.db.unmark_projected_by_ids(notices).await.map_err(|e| e.to_string())?;
                let stamped =
                    self.db.stamp_stale_for_notices(notices).await.map_err(|e| e.to_string())?;
                // Report all three numbers, because their DIFFERENCES are the
                // finding. Fewer re-queued than asked means some ids were unparsed,
                // already re-queued, or simply do not exist — a typo'd id would
                // otherwise vanish into a job that says "ok". Fewer stamped than
                // re-queued means notices that never reached a Tender.
                Ok(format!(
                    "{} notice(s) named: re-queued {requeued}, stamped {stamped} tender(s)                      epoch-stale for the incremental fold",
                    notices.len()
                ))
            }
            Spec::RefoldFields { fields, expect } => {
                let refs: Vec<&str> = fields.iter().map(String::as_str).collect();
                // Enumerate BEFORE writing (the sweep is the expensive step and is
                // read-only), then gate on `expect` exactly like `refold`: a
                // mistyped field id matching a far larger carrier set must abort
                // while nothing has been written.
                let carriers =
                    self.db.notice_ids_carrying_fields(&refs).await.map_err(|e| e.to_string())?;
                let found = carriers.len() as u64;
                if let Some(expect) = expect {
                    let slack = expect / 4;
                    if found.abs_diff(*expect) > slack {
                        return Err(format!(
                            "refold-fields aborted: {found} notices carry {fields:?}, expected \
                             ~{expect} — check the field ids (nothing was written)"
                        ));
                    }
                }
                let requeued =
                    self.db.unmark_projected_by_ids(&carriers).await.map_err(|e| e.to_string())?;
                // Same issue-179 pair as `refold`: requeue + scoped stale-stamp,
                // both idempotent so a crashed job heals on re-run.
                let stamped =
                    self.db.stamp_stale_for_notices(&carriers).await.map_err(|e| e.to_string())?;
                Ok(format!(
                    "{found} carriers of {} field id(s): re-queued {requeued} notices, \
                     stamped {stamped} tenders epoch-stale for the incremental fold",
                    refs.len()
                ))
            }
            Spec::MarkSkippedSiblings { dry_run, expect, expect_gaps } => {
                // Count first, always — in dry-run it IS the answer, and in a real
                // run it is the gate that must agree before anything is written.
                let found = self.db.count_skipped_siblings().await.map_err(|e| e.to_string())? as u64;
                if let Some(expect) = expect {
                    if found != *expect {
                        return Err(format!(
                            "mark-skipped-siblings aborted: {found} rows match, expected exactly \
                             {expect} (nothing was written). A SHORTFALL is a finding, not a \
                             predicate to widen: the missing rows are held siblings whose English \
                             original did not parse — held-but-unextracted, and they must stay \
                             outstanding until something reads them."
                        ));
                    }
                }
                // The dry-run reports BOTH numbers, because the first alone cannot
                // explain itself. `gaps` is the set the guard declines — held
                // siblings whose English original is missing or unparsed. Expected
                // 0; a non-zero answer is a data-loss finding to investigate, and
                // the run should stop rather than proceed on a population that no
                // longer matches what was verified.
                let (no_original, unparsed) =
                    self.db.count_skipped_sibling_gaps().await.map_err(|e| e.to_string())?;
                let gaps = no_original + unparsed;
                if *dry_run {
                    return Ok(format!(
                        "dry run: {found} rows would be marked skipped-by-policy; \
                         {gaps} in scope REJECTED by the sibling guard \
                         ({no_original} with NO English original at all — a fetch/ingest \
                         gap; {unparsed} whose original is held but did not parse — a \
                         parse failure). These are held-but-unextracted, not lost: we \
                         have the bytes and failed to read them. Two separate \
                         investigations, and never a predicate to widen — an execute \
                         must name this count in `expect_gaps` rather than pass it"
                    ));
                }
                // BOTH halves of the go criterion are enforced HERE, not only in the
                // process that reads the dry-run. `found == expect` and the gap
                // check are independent: the scope can hold exactly the expected
                // number of markable rows AND a rejected set beside them, so a
                // matching count is not evidence about the gaps. Leaving this to the
                // operator would make half the criterion a promise rather than a
                // guarantee — and the promise would be kept by whoever remembered to
                // read the second number.
                //
                // An execute with no `expect` at all is refused outright. `None`
                // used to mean "skip the count check", which made the strictest
                // reading of a missing argument the most destructive one — the same
                // inversion `dry_run` already defends against. A run that writes
                // ~593k rows must state what it expects to write.
                if !dry_run && expect.is_none() {
                    return Err(
                        "mark-skipped-siblings aborted: an execute requires an explicit `expect` \
                         (nothing was written). A run that writes hundreds of thousands of rows \
                         must name the population it believes it is writing, so the count can \
                         disagree with it."
                            .to_owned(),
                    );
                }
                // The gap criterion. `expect_gaps` is NOT an override: it re-aims the
                // guard rather than disarming it. Omitted, the rejected set must be
                // empty — the original rule, unchanged. Supplied, the set must be
                // EXACTLY that size, so this still refuses a population that has
                // shifted by one row since it was investigated.
                //
                // The distinction matters because the guard's whole value is that it
                // rejected 154 and passed 592,856 — discrimination, not mere firing.
                // A flag that let the run proceed regardless of the gap count would
                // make the reject arm unreachable, and a guard that cannot fail is
                // not a guard; the fastest way to turn a red gate green is to move
                // the bar rather than the data. This keeps the bar, and requires an
                // operator to state the number they have already looked at.
                let allowed_gaps = expect_gaps.unwrap_or(0);
                if gaps != allowed_gaps as i64 {
                    return Err(format!(
                        "mark-skipped-siblings aborted: the sibling guard rejects {gaps} in-scope \
                         rows, expected exactly {allowed_gaps} (nothing was written) — \
                         {no_original} have NO English original at all (a fetch/ingest gap) and \
                         {unparsed} have one that is held but did not parse (a parse failure). \
                         Two different investigations. Both must stay outstanding: they are \
                         held-but-unextracted, not lost, and this is never a predicate to widen."
                    ));
                }
                let mut marked = 0i64;
                loop {
                    let batch = self
                        .db
                        .mark_skipped_siblings(MARK_BATCH, store::now_unix(), "internal-ojs-non-english")
                        .await
                        .map_err(|e| e.to_string())?;
                    if batch == 0 {
                        break;
                    }
                    marked += batch;
                    self.update(|p| p.members_done = marked as u64);
                    // Bound the WAL between batches, exactly as run_process and the
                    // reclaim do: turso writes a frame per row and cannot checkpoint
                    // mid-statement.
                    if let Err(e) = self.db.checkpoint(store::CheckpointMode::Truncate).await {
                        eprintln!("supervisor: checkpoint after mark batch: {e}");
                    }
                }
                Ok(format!(
                    "marked {marked} rows skipped-by-policy (reversible: skipped_reason = \
                     'internal-ojs-non-english')"
                ))
            }
            Spec::RepairSweptSiblings { dry_run } => {
                let swept = self.db.count_swept_siblings().await.map_err(|e| e.to_string())?;
                if *dry_run {
                    return Ok(format!(
                        "dry run: {swept} skipped sibling row(s) are REJECTED by the \
                         parsed-original guard and would be restored to outstanding"
                    ));
                }
                let restored = self.db.repair_swept_siblings().await.map_err(|e| e.to_string())?;
                Ok(format!(
                    "restored {restored} guard-rejected sibling row(s) to outstanding \
                     (found {swept} before the write)"
                ))
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
    /// Run the data-quality measurement (issue 230) and store the rendered report.
    ///
    /// The queries are the SAME ones `bin/data-quality` sends — one source of
    /// truth for what "semantic completeness" means — but they run here on the
    /// reader pool with no request deadline, because at full-corpus scale every
    /// one of them exceeds the endpoint's 10 s cap. A failed query stays
    /// distinguishable from an empty one all the way to the rendered text (the
    /// `None` that `from_labelled` reads as unmeasured), so a partial report says
    /// which sections it could not measure instead of printing zeros.
     /// One integer out of a one-row, one-column measurement query, on the reader
    /// pool. Absent rows and non-integers read as 0 rather than as an error: every
    /// caller wraps its SQL in `COALESCE`, so a missing number means an empty
    /// table, not a broken query.
    async fn measure_i64(&self, sql: &str) -> Result<i64, String> {
        Ok(self
            .db
            .measure_rows(sql)
            .await
            .map_err(|e| e.to_string())?
            .into_iter()
            .next()
            .map(json_row)
            .and_then(|row| row.first().and_then(serde_json::Value::as_i64))
            .unwrap_or(0))
    }

    /// Measure semantic data quality over bounded id windows, store the rendered
    /// report, and say what could not be measured (issue 230).
    ///
    /// Two earlier attempts ran these aggregates across the whole corpus in one
    /// statement each and both had to be killed. The EXISTS rewrite did fix the
    /// pathological query *shape*, but shape was never the whole problem: one
    /// statement over 7.9M versions is unbounded work, turso cannot interrupt a
    /// running statement, and so "slower than I guessed" and "will never finish"
    /// look identical from outside. Both times the guess came from cache-hot
    /// samples (0.37 s at 50k tenders extrapolated to ~47 s at full scale) and both
    /// times the real pass was still running minutes later.
    ///
    /// So the measurement is no longer *hopefully* fast, it is *inherently*
    /// bounded — issue 228's shape. Each statement covers one [`DQ_WINDOW`]-wide
    /// slice of `tender_versions.tender_id`; per-profile counts accumulate across
    /// the slices; every version falls in exactly one slice, so the sum is exactly
    /// the unwindowed answer. Two things follow that the unwindowed pass could not
    /// offer: the job reports honest `window k/N` progress instead of sitting mute,
    /// and each window's elapsed goes to the journal — so the next sizing decision
    /// is a measurement rather than a third extrapolation.
    ///
    /// A label with a FAILED window is reported unmeasured, never summed. A sum
    /// missing one window is a wrong number wearing a right number's clothes, which
    /// is the precise failure `Raw::from_labelled`'s `None` path exists to prevent.
    /// The four queries with no windowing yet (`linkage`, both densities, `merge`)
    /// are unmeasured for the same reason rather than quietly dropped.
    ///
    /// `confirmed` still means what it meant — the operator accepting that the job
    /// holds the serialized queue for the duration — and a dry run now has
    /// something worth reporting: the window count the run would take, from one
    /// indexed `MAX`, having touched no data page.
    async fn run_data_quality(&self, confirmed: bool) -> Result<String, String> {
        use ingest::data_quality::{self, Raw, Rows};
        use std::collections::BTreeMap;
        use std::time::Instant;

        // Two separate indexed aggregates, not one `MIN(), MAX()` query: SQLite
        // turns a LONE min or max over an indexed column into a probe, but has to
        // scan the index when asked for both at once — and a full index scan of
        // 7.9M versions is precisely the unbounded statement this job exists to
        // stop running.
        let max_id =
            self.measure_i64("SELECT COALESCE(MAX(tender_id), 0) FROM tender_versions").await?;
        // Windows are half-open `(lo, hi]`, so the first must start BELOW the
        // smallest id or it drops that tender silently. Deriving the floor beats
        // assuming ids begin at 1: the assumption would be invisible if it broke.
        let floor =
            self.measure_i64("SELECT COALESCE(MIN(tender_id), 1) FROM tender_versions").await? - 1;
        let windows = dq_windows(floor, max_id);

        let queries = data_quality::windowed_queries();
        if !confirmed {
            let unwindowed = data_quality::unwindowed_labels();
            let caveat = if unwindowed.is_empty() {
                "; every label is windowed".to_owned()
            } else {
                format!("; unmeasured, no windowing: {}", unwindowed.join(","))
            };
            return Ok(format!(
                "data-quality dry run: would measure {} window(s) of {DQ_WINDOW} ids up to \
                 tender_id {max_id}, {} queries each ({} statements){caveat}",
                windows.len(),
                queries.len(),
                windows.len() * queries.len(),
            ));
        }
        if windows.is_empty() {
            // Nothing to measure is not a report worth storing — storing one would
            // overwrite a real earlier measurement with a row of zeros.
            return Ok("data quality: no tender versions to measure".to_owned());
        }

        // Windows outer, queries inner: the seven probes over one slice hit the same
        // version and notice pages, so the slice stays warm for all of them instead
        // of being paged in seven times.
        let units = (windows.len() * queries.len()) as u64;
        let mut per_label: BTreeMap<String, Vec<Rows>> = BTreeMap::new();
        // Elapsed per LABEL across every window, so the run says which query costs
        // what. A per-window total cannot: the eleven queries differ by more than an
        // order of magnitude in cost, so "this window took 291 s" identifies nothing
        // to fix. This breakdown is the input to the next sizing or indexing
        // decision, which is the whole reason the timings are logged at all.
        let mut cost: BTreeMap<String, f64> = BTreeMap::new();
        let mut broken: Vec<String> = Vec::new();
        let mut done = 0u64;
        let run_started = Instant::now();
        for (wi, (lo, hi)) in windows.iter().enumerate() {
            let window_started = Instant::now();
            for query in &queries {
                self.set_phase(
                    "measuring",
                    Some(done),
                    Some(units),
                    format!("window {}/{} query {}", wi + 1, windows.len(), query.label),
                );
                let query_started = Instant::now();
                let measured = self.db.measure_rows(&query.sql(*lo, *hi)).await;
                *cost.entry(query.label.clone()).or_default() +=
                    query_started.elapsed().as_secs_f64();
                match measured {
                    Ok(rows) => per_label
                        .entry(query.label.clone())
                        .or_default()
                        .push(rows.into_iter().map(json_row).collect()),
                    Err(e) => {
                        eprintln!(
                            "[data-quality] window {}/{} ({lo}..{hi}] query {} failed: {e}",
                            wi + 1,
                            windows.len(),
                            query.label
                        );
                        if !broken.contains(&query.label) {
                            broken.push(query.label.clone());
                        }
                    }
                }
                done += 1;
            }
            eprintln!(
                "[data-quality] window {}/{} ({lo}..{hi}]: {:.1}s for {} queries ({:.1}s elapsed)",
                wi + 1,
                windows.len(),
                window_started.elapsed().as_secs_f64(),
                queries.len(),
                run_started.elapsed().as_secs_f64(),
            );
        }

        // Costliest first: the line an operator reads to decide what to index or
        // resize next.
        let mut ranked: Vec<(&String, &f64)> = cost.iter().collect();
        ranked.sort_by(|a, b| b.1.total_cmp(a.1));
        eprintln!(
            "[data-quality] cost by query: {}",
            ranked
                .iter()
                .map(|(label, secs)| format!("{label} {secs:.0}s"))
                .collect::<Vec<_>>()
                .join(", ")
        );

        let mut results: Vec<(String, Option<Rows>)> = Vec::new();
        for query in &queries {
            let rows = if broken.contains(&query.label) {
                None
            } else {
                Some(data_quality::sum_profile_counts(
                    per_label.get(&query.label).map(Vec::as_slice).unwrap_or_default(),
                ))
            };
            results.push((query.label.clone(), rows));
        }
        for label in data_quality::unwindowed_labels() {
            results.push((label, None));
        }

        let raw = Raw::from_labelled(results).map_err(|e| e.to_string())?;
        let report = data_quality::assemble("(in-process)", &raw);
        let body = data_quality::render_text(&report);
        let now = store::now_unix();
        self.db.put_report("data-quality", &body, now).await.map_err(|e| e.to_string())?;

        // The summary is the digest; the body is in `reports` for whoever reads it.
        // Unmeasured labels are named in the summary, not just in the body — the job
        // log is what an operator sees first.
        let took = run_started.elapsed().as_secs_f64();
        if broken.is_empty() {
            Ok(format!(
                "data quality measured: {} eras over {} windows in {took:.0}s; {} label(s) \
                 unmeasured ({})",
                report.completeness.len(),
                windows.len(),
                report.unmeasured.len(),
                report.unmeasured.join(","),
            ))
        } else {
            Ok(format!(
                "data quality PARTIAL: {} window query label(s) failed ({}) in {took:.0}s; {} \
                 eras measured, {} label(s) unmeasured ({})",
                broken.len(),
                broken.join(","),
                report.completeness.len(),
                report.unmeasured.len(),
                report.unmeasured.join(","),
            ))
        }
    }

    /// Re-parse a profile cohort from the archive (issue 100), one package at a
    /// time, resumable on the same `resume_after` cursor `process`/`reprocess` use.
    ///
    /// One thing differs from `run_reprocess` and it decides termination: a
    /// reclaim's work list SHRINKS as rows get `reprocessed_at`, so a re-query
    /// converges. A re-parsed notice is still a parsed notice of the same profile,
    /// so this work list is IDEMPOTENT — the cursor is the only progress there is,
    /// which is why it is advanced per package and read back on resume.
    async fn run_reparse(
        &self,
        job_id: u64,
        profiles: &[String],
        resume_after: Option<&str>,
    ) -> Result<String, String> {
        let after = resume_after.and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
        let refs: Vec<&str> = profiles.iter().map(String::as_str).collect();
        let packages =
            self.db.reparse_packages(&refs, after).await.map_err(|e| e.to_string())?;
        self.update(|p| p.packages_total = packages.len() as u64);
        if packages.is_empty() {
            return Ok(format!("no packages hold parsed notices of {}", refs.join(",")));
        }

        let (mut reparsed, mut unmatched, mut failing, mut members) = (0u64, 0u64, 0u64, 0u64);
        for (i, (fetch_id, source, path)) in packages.iter().enumerate() {
            self.update(|p| {
                p.package = Some(format!("fetch {fetch_id}"));
                p.packages_done = i as u64;
                p.members_done = 0;
                p.members_total = 0;
            });
            self.set_phase(
                "re-parsing",
                Some(i as u64),
                Some(packages.len() as u64),
                format!("fetch {fetch_id}: {reparsed} notices re-parsed so far"),
            );
            let selected = self
                .db
                .parsed_member_files(*fetch_id, &refs)
                .await
                .map_err(|e| e.to_string())?;
            let report = ingest::process::reparse_package(
                &self.db,
                &self.archive.join(path),
                source,
                *fetch_id,
                selected,
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
            reparsed += report.reparsed;
            unmatched += report.unmatched;
            failing += report.now_failing;
            members += report.members;
            self.update(|p| p.packages_done = (i + 1) as u64);
            if let Err(e) = self.db.record_job_progress(job_id as i64, &fetch_id.to_string()).await {
                eprintln!("supervisor: job {job_id} record reparse progress {fetch_id}: {e}");
            }
            if let Err(e) = self.db.checkpoint(store::CheckpointMode::Truncate).await {
                eprintln!("supervisor: job {job_id} checkpoint after fetch {fetch_id}: {e}");
            }
        }

        // `now_failing` is the line to read first on any future run: it counts
        // notices the CURRENT parser can no longer parse, whose stored layer was
        // therefore left alone. Non-zero means a parser regression, not progress.
        Ok(format!(
            "re-parsed {reparsed} notices across {} packages ({members} members walked, \
             {unmatched} unmatched, {failing} now failing and left untouched)",
            packages.len()
        ))
    }

}

/// One measured row as JSON (issue 230): `store::measure_rows` returns turso
/// values because the store carries no JSON dependency, and `data_quality` speaks
/// `serde_json`. Mirrors the `/v1/sql` cell mapping, minus the byte accounting
/// that endpoint needs for its response cap.
/// The half-open `(lo, hi]` windows of `tender_versions.tender_id` that together
/// cover every id in `floor+1 ..= max_id`, each at most [`DQ_WINDOW`] wide (issue
/// 230).
///
/// Kept separate from the job that walks them because the arithmetic is where a
/// bounded measurement silently becomes a wrong one: an overlap double-counts a
/// version, a gap drops one, and either way the report still renders a plausible
/// percentage. Exact coverage is a property worth asserting, so it lives somewhere
/// a test can reach without a corpus.
fn dq_windows(floor: i64, max_id: i64) -> Vec<(i64, i64)> {
    let mut out = Vec::new();
    let mut lo = floor;
    while lo < max_id {
        let hi = lo.saturating_add(DQ_WINDOW).min(max_id);
        out.push((lo, hi));
        lo = hi;
    }
    out
}

fn json_row(cells: Vec<store::turso::Value>) -> Vec<serde_json::Value> {
    use store::turso::Value;
    cells
        .into_iter()
        .map(|v| match v {
            Value::Null => serde_json::Value::Null,
            Value::Integer(n) => serde_json::json!(n),
            Value::Real(f) => serde_json::Number::from_f64(f)
                .map_or(serde_json::Value::Null, serde_json::Value::Number),
            Value::Text(s) => serde_json::Value::String(s),
            Value::Blob(b) => serde_json::Value::String(b.iter().map(|x| format!("{x:02x}")).collect()),
        })
        .collect()
}

impl Supervisor {
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
        let mut reasons: std::collections::BTreeMap<String, u64> = Default::default();
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
            for (reason, count) in &report.still_held_reasons {
                *reasons.entry(reason.clone()).or_insert(0) += count;
            }
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
        //
        // The residual's shape rides along (issue 87): the reasons are the
        // CURRENT re-parse failures, so a bucket that failed for a NEW cause is
        // visible right here — a reason-bucket delta check cannot see it, because
        // a StillHeld member used to keep its stale first-ingest reason. Bounded
        // to the top entries so a pathological residual cannot bloat the row.
        let residual = {
            let mut by_count: Vec<(&String, &u64)> = reasons.iter().collect();
            by_count.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
            let (head, tail) = by_count.split_at(by_count.len().min(8));
            let mut sample: Vec<String> =
                head.iter().map(|(reason, count)| format!("{reason} {count}")).collect();
            let rest: u64 = tail.iter().map(|(_, count)| **count).sum();
            if rest > 0 {
                sample.push(format!("(other) {rest}"));
            }
            if sample.is_empty() {
                String::new()
            } else {
                format!("; still held by current reason: {}", sample.join(", "))
            }
        };
        Ok(format!(
            "{} package(s): {reclaimed} reclaimed, {still_held} still held, \
             {already} already parsed, {skipped} skipped by dispatch policy{residual}",
            packages.len()
        ))
    }

    // --------------------------------------------------------------- scheduler

    /// Spawn the daily scheduler: at 09:35 Europe/Berlin it enqueues the TED
    /// probe (Mon–Fri) and DÖE completed-day fetch, then process + project.
    /// The morning window the weekday catch-up polls over after the 09:35 tick
    /// (issue 222): 09:35 → ~12:35. TED's daily package is "final by 09:30 CET" and
    /// always up well before noon, so three hours covers a slipped publication
    /// without ever polling into the afternoon.
    const CATCHUP_WINDOW_SECS: i64 = 3 * 3_600;

    /// How often the weekday catch-up re-checks for TED's package. A few minutes:
    /// fine enough to fetch a late package promptly, coarse enough that a late
    /// morning is a handful of cheap no-op probes, not a busy loop.
    const CATCHUP_POLL_SECS: u64 = 300;

    pub fn spawn_scheduler(self: Arc<Self>) {
        tokio::spawn(async move {
            loop {
                let now = store::now_unix();
                let (tick, weekday) = next_berlin_tick(now, 9, 35);
                tokio::time::sleep(std::time::Duration::from_secs((tick - now).max(0) as u64)).await;

                // TED's watermark before today's run, so the weekday catch-up can tell
                // whether today's issue actually landed at the tick.
                let ted_before = if weekday { self.latest_ted_issue_now().await } else { None };

                self.enqueue_daily(weekday).await;

                // Weekday morning catch-up (issue 222). TED's daily package is "final by
                // 09:30 CET" but the exact moment slips; the lone 09:35 tick would then
                // miss the day until tomorrow's walk-forward. Give the tick's probe time
                // to land the package on a normal day, and if it has NOT, keep re-probing
                // on a short interval until it does (or the morning window closes), then
                // process+project the late package the SAME morning. On a normal day the
                // watermark has already advanced, so no catch-up runs at all; the retries
                // are cheap no-op probes, and DÖE (T+1) gains nothing here so it is left
                // on the tick.
                if weekday {
                    let deadline = tick + Self::CATCHUP_WINDOW_SECS;
                    tokio::time::sleep(std::time::Duration::from_secs(Self::CATCHUP_POLL_SECS)).await;
                    let mut caught_up = false;
                    while self.latest_ted_issue_now().await <= ted_before
                        && store::now_unix() < deadline
                    {
                        self.push("probe", "ted daily (catch-up)".into(), Spec::ProbeTed { refetch: true })
                            .await;
                        caught_up = true;
                        tokio::time::sleep(std::time::Duration::from_secs(Self::CATCHUP_POLL_SECS)).await;
                    }
                    // A package that landed DURING catch-up was fetched by the probes
                    // above but not folded (the tick's project ran before it existed).
                    if caught_up && self.latest_ted_issue_now().await > ted_before {
                        self.enqueue_daily(weekday).await;
                    }
                }

                // Step past this tick so the next computation lands on tomorrow.
                tokio::time::sleep(std::time::Duration::from_secs(61)).await;
            }
        });
    }

    /// TED's newest registered daily issue for the current UTC year, or `None`
    /// (including on a read error — the catch-up then simply keeps polling, which is
    /// harmless). The weekday catch-up watches this for the day's package landing.
    async fn latest_ted_issue_now(&self) -> Option<u32> {
        let (year, _, _) = fetch::civil_date(store::now_unix());
        fetch::latest_ted_issue(&self.db, year).await.unwrap_or(None)
    }

    /// When the data-quality measurement is queued: Berlin wall-clock `(weekday,
    /// hour, minute)`, `0` = Sunday (issue 230).
    ///
    /// Weekly, not daily: the run is hours of reader-pool work over 32 id windows
    /// and it holds the serialized queue for all of it, while the numbers it
    /// produces (per-era field completeness, award linkage, results density) move on
    /// the scale of a parser change, not of a day's ingest.
    ///
    /// Measured on prod rather than guessed, because guessing is what went wrong
    /// twice before: the SEVEN-query pass (before linkage/density/merge were
    /// windowed) took **1258 s over 32 windows** with zero failed windows. Adding
    /// the last four queries roughly quadrupled a window — 291 s and 229 s for the
    /// first two — which puts the eleven-query pass in the 2–3 hour range. Even at
    /// the top of that, a 03:10 Berlin start finishes hours before the 09:35 daily.
    /// The per-query cost breakdown each run now logs is what will say whether that
    /// price is an index away from being much lower.
    ///
    /// 03:10 Sunday, not the 09:35 daily tick: the daily is the busiest moment the
    /// box has — probe, process and fold, back to back — and a measurement queued
    /// behind it would either delay the fold or measure a corpus mid-write. Sunday
    /// pre-dawn is the emptiest slot in the week and 6+ hours clear of that day's
    /// tick in either direction.
    const REPORT_TICK: (i64, i64, i64) = (0, 3, 10);

    /// Queue the data-quality measurement once a week (issue 230).
    ///
    /// Its own loop rather than a branch in the daily scheduler, which carries the
    /// TED catch-up retry window: a weekly job hanging off that loop would inherit
    /// timing that exists for a completely different reason and break the next time
    /// the catch-up is tuned.
    ///
    /// And scheduling it at all is the point of the issue, not a nicety. The report
    /// rotted for months precisely because nothing ran it: every query had been
    /// timing out, `bin/data-quality` was correctly returning FAILURE about it, and
    /// no scheduled run existed to see that failure. A measurement nobody runs is
    /// indistinguishable from a measurement that passes.
    pub fn spawn_report_scheduler(self: Arc<Self>) {
        tokio::spawn(async move {
            let (weekday, hour, minute) = Self::REPORT_TICK;
            loop {
                let now = store::now_unix();
                let (tick, _) = next_berlin_tick(now, hour, minute);
                tokio::time::sleep(std::time::Duration::from_secs((tick - now).max(0) as u64)).await;

                // A pre-dawn Berlin tick falls on the same UTC calendar day at either
                // DST offset, so the UTC day number names the Berlin weekday.
                if weekday_of(tick.div_euclid(86_400)) == weekday {
                    // Never stack two: if last week's run is still waiting behind
                    // something long, a second one would double a 36-minute job for
                    // one report that gets overwritten anyway.
                    if self.queued().iter().any(|j| j.kind == "data-quality") {
                        eprintln!("[schedule] data-quality already queued, skipping this week");
                    } else {
                        self.push(
                            "data-quality",
                            "data-quality (weekly)".into(),
                            Spec::DataQuality { confirmed: true },
                        )
                        .await;
                    }
                }

                // Step past this tick so the next computation lands on tomorrow.
                tokio::time::sleep(std::time::Duration::from_secs(61)).await;
            }
        });
    }

    /// How often the canonical layer's presence is observed (issue 133 / #38).
    ///
    /// Deliberately NOT the daily scheduler and NOT the job queue. The daily
    /// tick is the cadence that CAUSED the problem this detector exists for:
    /// a layer emptied at noon stayed invisible until the next day's snapshot,
    /// so detection latency was bounded by the observation cadence rather than
    /// by anything about the damage. And the queue runs jobs sequentially, so
    /// an observer behind a multi-hour projection would not run for those hours
    /// — the exact window it is supposed to be watching.
    const PRESENCE_INTERVAL_SECS: u64 = 300;

    /// Observe the canonical layer's presence on a short fixed interval, out of
    /// band from the job queue (issue 133 / task #38).
    ///
    /// Skips while a heavy write is in progress, and this is the load-bearing
    /// decision in the whole detector. A rebuild empties the layer at its start
    /// by design (`reset_tender_layer`), so observing during one would record
    /// `WentEmpty` on every single rebuild and flip `/health/deep` to 503 for
    /// hours of entirely correct operation. A probe that cries wolf on the
    /// normal path is a probe someone mutes, and a muted probe is worse than no
    /// probe because it is still trusted.
    ///
    /// `observed_at` is still refreshed while skipping, so a long rebuild does
    /// not age the observation into the staleness alarm — which would be the
    /// same false positive wearing a different hat.
    ///
    /// What this costs, stated plainly rather than glossed: the detector is
    /// blind for as long as a projection is running, including a rebuild that
    /// was killed and re-run at boot (issue 21). It does NOT catch "the layer
    /// is empty while a rebuild refills it" — that state is expected and
    /// already visible as a running job. It catches the case that actually hurt
    /// us: the layer left empty with nothing running to fix it.
    pub fn spawn_presence_observer(self: Arc<Self>) {
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(Self::PRESENCE_INTERVAL_SECS)).await;
                let now = store::now_unix();
                let result = if self.heavy_write_in_progress() {
                    self.db.touch_layer_presence(now).await.map(|()| Vec::new())
                } else {
                    self.db.observe_layer_presence(now).await
                };
                match result {
                    Ok(verdicts) => {
                        // Emptied tables are logged individually and by name:
                        // the journal is what an operator reads after the 503,
                        // and "which table" is the diagnosis.
                        for v in verdicts.iter().filter(|v| {
                            matches!(v.state, store::LayerState::WentEmpty { .. })
                        }) {
                            eprintln!(
                                "supervisor: CANONICAL LAYER EMPTY — `{}` held rows and no longer does ({:?})",
                                v.name, v.state
                            );
                        }
                    }
                    // Never fatal: a failed observation must not take down the
                    // process it is watching. It ages `observed_at`, which the
                    // staleness clause turns into an unhealthy probe on its own.
                    Err(e) => eprintln!("supervisor: layer presence observation failed: {e}"),
                }
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

/// Day of week for a unix *day* number, `0` = Sunday. Day 0 (1970-01-01) was a
/// Thursday, hence the `+4`.
fn weekday_of(day: i64) -> i64 {
    (day + 4).rem_euclid(7)
}

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
            let weekday = weekday_of(day + k);
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
            phase: None,
        }
    }

    /// Issue 230: `data-quality` is a plain single job — no parameters to get
    /// wrong, and deliberately NOT paired with anything, since it only measures.
    #[tokio::test]
    async fn data_quality_enqueues_as_a_single_measuring_job() {
        let sup = Supervisor::new(scratch().await, "archive".into(), reqwest::Client::new());
        let ids = sup.enqueue_request(&req("data-quality")).await.expect("enqueues");
        assert_eq!(ids.len(), 1, "a measurement changes nothing, so nothing follows it");
        assert_eq!(sup.queued()[0].kind, "data-quality");
        // A forgotten flag means the harmless thing: the default is a dry run, and
        // the params say so, so the job log never hides which one ran.
        assert_eq!(sup.queued()[0].params, "data-quality dry-run");

        let ids = sup
            .enqueue_request(&JobRequest {
                kind: "data-quality".into(),
                dry_run: Some(false),
                ..Default::default()
            })
            .await
            .expect("enqueues");
        assert_eq!(ids.len(), 1);
        assert_eq!(sup.queued()[1].params, "data-quality", "a confirmed run is named plainly");
    }

    /// Issue 230: the weekly measurement's slot is a claim about the calendar, so
    /// the calendar arithmetic is checked. A tick that silently landed on the wrong
    /// weekday would queue a 36-minute job into a busy morning.
    #[test]
    fn the_weekly_report_tick_lands_on_its_named_weekday() {
        // 2026-08-16 is a Sunday, the 18th a Tuesday.
        assert_eq!(weekday_of(fetch::days_from_civil(2026, 8, 16)), 0);
        assert_eq!(weekday_of(fetch::days_from_civil(2026, 8, 18)), 2);
        assert_eq!(weekday_of(fetch::days_from_civil(2026, 8, 22)), 6);

        // `next_berlin_tick`'s own weekday flag and `weekday_of` must agree, or one
        // of the two schedulers is reading a different calendar.
        for d in 0..14 {
            let noon = (fetch::days_from_civil(2026, 8, 10) + d) * 86_400 + 12 * 3_600;
            let (tick, weekday) = next_berlin_tick(noon, 3, 10);
            let wd = weekday_of(tick.div_euclid(86_400));
            assert_eq!(weekday, wd != 0 && wd != 6, "weekday flag disagrees at day {d}");
        }

        // The configured slot is the Sunday pre-dawn one this schedule argues for,
        // and it is nowhere near the 09:35 daily tick.
        let (weekday, hour, minute) = Supervisor::REPORT_TICK;
        assert_eq!((weekday, hour, minute), (0, 3, 10));
        assert!(hour < 9, "the measurement must not collide with the daily fold");
    }

    /// Issue 230: the windows must tile the id range EXACTLY — no gap (a dropped
    /// version understates a completeness rate) and no overlap (a double-counted
    /// one can push it over 100%). Both render as a believable percentage, so the
    /// tiling is checked rather than trusted.
    #[test]
    fn data_quality_windows_tile_the_id_range_exactly() {
        // The realistic shape: a floor of 0 (ids start at 1) and a span that is not
        // a whole number of windows.
        let w = dq_windows(0, DQ_WINDOW * 2 + 7);
        assert_eq!(w.len(), 3);
        assert_eq!(w[0], (0, DQ_WINDOW));
        assert_eq!(w[2], (DQ_WINDOW * 2, DQ_WINDOW * 2 + 7), "the tail window is short, not skipped");

        // Contiguity and coverage, for several spans including exact multiples and
        // spans smaller than one window.
        for (floor, max_id) in
            [(0, 1), (0, DQ_WINDOW), (0, DQ_WINDOW * 3), (5, 5 + DQ_WINDOW + 1), (-1, 4)]
        {
            let w = dq_windows(floor, max_id);
            assert_eq!(w.first().unwrap().0, floor, "the first window opens at the floor");
            assert_eq!(w.last().unwrap().1, max_id, "the last window closes at the max");
            for pair in w.windows(2) {
                assert_eq!(pair[0].1, pair[1].0, "consecutive windows share a boundary exactly");
            }
            for (lo, hi) in &w {
                assert!(hi > lo, "no empty window");
                assert!(hi - lo <= DQ_WINDOW, "no window wider than the bound");
            }
        }

        // Nothing to measure yields nothing to run — not one window over an empty
        // table, which would store a report full of zeros.
        assert!(dq_windows(0, 0).is_empty());
        assert!(dq_windows(9, 9).is_empty());
        assert!(dq_windows(9, 4).is_empty());
    }

    /// Issue 230: a dry run reports the plan and measures nothing. It is cheap by
    /// construction — two indexed aggregates — so an operator can see the shape of
    /// the run before accepting that it holds the serialized queue.
    #[tokio::test]
    async fn a_data_quality_dry_run_reports_the_plan_and_measures_nothing() {
        let sup = Supervisor::new(scratch().await, "archive".into(), reqwest::Client::new());
        let plan = sup.run_data_quality(false).await.expect("a dry run cannot fail on an empty db");
        assert!(plan.starts_with("data-quality dry run:"), "{plan}");
        assert!(plan.contains("0 window(s)"), "an empty corpus plans no windows: {plan}");
        // Whatever cannot be windowed is named in the plan, so a hole in the report
        // is known BEFORE the run rather than discovered in the body. Nothing is
        // unwindowed today, and the plan says that rather than saying nothing.
        let unwindowed = ingest::data_quality::unwindowed_labels();
        if unwindowed.is_empty() {
            assert!(plan.contains("every label is windowed"), "{plan}");
        } else {
            for label in unwindowed {
                assert!(plan.contains(&label), "the plan names {label} as unmeasured: {plan}");
            }
        }
        assert!(sup.db.latest_report("data-quality").await.unwrap().is_none(), "a dry run stores nothing");

        // And a confirmed run over an empty corpus still stores nothing, rather than
        // overwriting a real earlier measurement with zeros.
        let out = sup.run_data_quality(true).await.expect("an empty confirmed run is not an error");
        assert_eq!(out, "data quality: no tender versions to measure");
        assert!(sup.db.latest_report("data-quality").await.unwrap().is_none());
    }

    /// Issue 58 v2, step 3: `refold-notices` takes a NAMED list, and its guard is a
    /// cap rather than `expect`-with-slack — slack guards a cohort nobody
    /// enumerated, and says nothing about a list somebody typed.
    #[tokio::test]
    async fn refold_notices_needs_ids_caps_the_list_and_logs_which_ones() {
        let sup = Supervisor::new(scratch().await, "archive".into(), reqwest::Client::new());

        assert!(
            sup.enqueue_request(&req("refold-notices")).await.is_err(),
            "an empty list has nothing to fold — refuse it rather than run a no-op"
        );

        let too_many: Vec<i64> = (1..=(REFOLD_NOTICES_CAP as i64 + 1)).collect();
        let err = sup
            .enqueue_request(&JobRequest {
                kind: "refold-notices".into(),
                notices: Some(too_many),
                ..Default::default()
            })
            .await
            .expect_err("a list past the cap is a cohort, not an exerciser");
        assert!(err.contains("cap"), "the refusal says what the limit is: {err}");

        let ids = vec![11i64, 22, 33];
        let queued_ids = sup
            .enqueue_request(&JobRequest {
                kind: "refold-notices".into(),
                notices: Some(ids.clone()),
                ..Default::default()
            })
            .await
            .expect("a small named list enqueues");
        assert_eq!(queued_ids.len(), 2, "the refold and its trailing projection");
        let queued = sup.queued();
        assert_eq!(queued[0].kind, "refold-notices");
        // The ids are in the params, so the job log records which notices a run
        // touched — a run whose effect cannot be attributed later is not much of an
        // experiment.
        assert_eq!(queued[0].params, "refold-notices 11,22,33");
        assert_eq!(queued[1].kind, "project");

        // A long-but-legal list is summarised rather than dumped whole: the params
        // line is read by a human in a job log.
        let many: Vec<i64> = (1..=12).collect();
        sup.enqueue_request(&JobRequest {
            kind: "refold-notices".into(),
            notices: Some(many),
            ..Default::default()
        })
        .await
        .expect("twelve ids is well inside the cap");
        assert_eq!(sup.queued()[2].params, "refold-notices 1,2,3,4,5,6,7,8 (+4 more)");
    }

    /// Issue 100: `reparse` needs at least one profile, and pairs itself with a
    /// projection — re-parsed notices land `projected = 0`, so the ordinary
    /// incremental fold carries them into the canonical layer.
    #[tokio::test]
    async fn reparse_needs_profiles_and_pairs_with_a_projection() {
        let sup = Supervisor::new(scratch().await, "archive".into(), reqwest::Client::new());

        assert!(
            sup.enqueue_request(&req("reparse")).await.is_err(),
            "a profile-less reparse would walk the whole corpus — refuse it"
        );

        let ids = sup
            .enqueue_request(&JobRequest {
                kind: "reparse".into(),
                profiles: Some(vec!["eforms:eforms-de-1.1".into(), "eforms:eforms-de-1.2".into()]),
                ..Default::default()
            })
            .await
            .expect("a reparse with profiles enqueues");
        assert_eq!(ids.len(), 2, "the reparse and its trailing projection");
        let queued = sup.queued();
        assert_eq!(queued[0].kind, "reparse");
        assert_eq!(
            queued[0].params, "reparse eforms:eforms-de-1.1,eforms:eforms-de-1.2",
            "the cohort is legible in the job log, not hidden in the spec"
        );
        assert_eq!(queued[1].kind, "project");

        // `reclaim_only` drops the fold, for a bulk run that would rather pay one
        // sequential rebuild afterwards (the ADR-0009 shape).
        let bulk = sup
            .enqueue_request(&JobRequest {
                kind: "reparse".into(),
                profiles: Some(vec!["eforms:eforms-de-1.1".into()]),
                reclaim_only: Some(true),
                ..Default::default()
            })
            .await
            .expect("bulk reparse enqueues");
        assert_eq!(bulk.len(), 1, "reclaim_only leaves the fold to a later rebuild");
    }

    /// Issue 65: a phase record names what a non-package-walking job is doing,
    /// and `set_phase` stamps its own `updated_at` so the stamp cannot lie about
    /// when the reporter was last heard from.
    #[tokio::test]
    async fn set_phase_records_what_the_running_job_is_doing() {
        let sup = Supervisor::new(scratch().await, "archive".into(), reqwest::Client::new());

        let snapshot = || sup.current.read().expect("progress lock").clone();

        // No running job: setting a phase is a no-op, not a panic. The worker and
        // the API race by construction — a job can finish between a reporter's
        // last tick and the write.
        sup.set_phase("sweeping", Some(1), Some(2), "detail".into());
        assert!(snapshot().is_none(), "idle stays idle");

        sup.set_current(Some(progress("backfill-legacy-adjacency")));
        assert!(snapshot().expect("running").phase.is_none(), "a job starts with no phase");

        let before = store::now_unix();
        sup.set_phase("sweeping", Some(11_400_000), Some(28_251_412), "notice id 11.4M".into());
        let phase = snapshot().expect("running").phase.expect("phase set");
        assert_eq!(phase.name, "sweeping");
        assert_eq!((phase.done, phase.total), (Some(11_400_000), Some(28_251_412)));
        assert_eq!(phase.detail, "notice id 11.4M");
        assert!(phase.updated_at >= before, "the stamp is taken when the phase is written");

        // A phase that cannot know its end reports position alone and still shows
        // movement — the case the count-only signal could not express.
        sup.set_phase("pre-pass", Some(7), None, "notices bucketed".into());
        let phase = snapshot().expect("running").phase.expect("phase set");
        assert_eq!((phase.name.as_str(), phase.done, phase.total), ("pre-pass", Some(7), None));
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
    /// Assert the persisted jobs all came back, by IDENTITY and in ORDER — without
    /// asserting how many jobs the queue holds in total.
    ///
    /// The count was always a proxy for "the right jobs came back in the right order",
    /// and issue 111's boot-time reindex broke the proxy rather than the property: a
    /// legitimate extra job made `after.len() == before.len()` fail while every
    /// recovered job was correct. Measuring the property directly means a future
    /// legitimate addition cannot break these again, and — the part that matters — a
    /// job coming back WRONG still fails, which a looser count never caught either.
    ///
    /// Order is checked as a SUBSEQUENCE: the persisted jobs must appear in their
    /// original relative order, with anything else free to sit around them.
    #[cfg(test)]
    fn assert_recovered(after: &[QueuedJob], before: &[QueuedJob]) {
        let mut remaining = after.iter();
        for want in before {
            let found = remaining
                .find(|got| got.id == want.id)
                .unwrap_or_else(|| panic!(
                    "job {} ({}) did not come back, or came back out of order. \
                     Recovered: {:?}",
                    want.id,
                    want.kind,
                    after.iter().map(|j| (j.id, &j.kind)).collect::<Vec<_>>()
                ));
            assert_eq!(
                (&found.kind, &found.params),
                (&want.kind, &want.params),
                "job {} came back with different content",
                want.id
            );
        }
    }

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
        assert_recovered(&after, &before);
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
        // Identity and relative order, not the total: an unrelated job in the queue
        // must not be able to fail this, and a job coming back wrong still must.
        let interrupted = q.iter().position(|j| j.id == running).expect("the interrupted job is back");
        assert_eq!(q[interrupted].kind, "project");
        let behind = q
            .iter()
            .position(|j| j.kind == "process")
            .expect("the job queued behind it is back");
        assert!(
            interrupted < behind,
            "the interrupted job must come back AHEAD of the one queued behind it — \
             recovered order was {:?}",
            q.iter().map(|j| (j.id, &j.kind)).collect::<Vec<_>>()
        );
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
        // The property is that the CANCELLED job is gone, not that the queue is empty —
        // an unrelated job being present says nothing about cancellation.
        assert!(
            !restarted.queued().iter().any(|j| j.id == ids[0]),
            "a cancelled job is gone from the durable queue too"
        );
    }


    /// Wrap a bare Spec in the Job envelope `run_spec` takes. The id/kind/params
    /// are irrelevant to these refusals — the Spec is what is under test.
    fn job(spec: Spec) -> Job {
        Job { id: 1, kind: "mark-skipped-siblings".into(), params: String::new(), spec, resume_after: None }
    }

    /// An execute with no `expect` is refused outright (issue 138 criterion 1).
    ///
    /// `None` used to mean "skip the count check", so the least-specified request
    /// was also the most destructive — the inversion `dry_run` already defends
    /// against, reintroduced by a different argument. A dry run still needs no
    /// `expect`, because it writes nothing.
    #[tokio::test]
    async fn an_execute_without_an_expected_count_is_refused() {
        let db = scratch().await;
        let sup = Supervisor::new(db.clone(), "archive".into(), reqwest::Client::new());

        let err = sup
            .run_spec(&job(Spec::MarkSkippedSiblings { dry_run: false, expect: None, expect_gaps: Some(0) }))
            .await
            .expect_err("an execute with no expected count must be refused");
        assert!(
            err.contains("requires an explicit `expect`"),
            "the refusal must say what is missing, got: {err}"
        );

        // The dry run is unaffected: it writes nothing, so it has nothing to assert.
        sup.run_spec(&job(Spec::MarkSkippedSiblings { dry_run: true, expect: None, expect_gaps: None }))
            .await
            .expect("a dry run needs no expectation");
    }

    /// `expect_gaps` re-aims the guard; it does not disarm it (issue 138 criterion 5).
    ///
    /// The guard's value is that it rejected 154 and passed 592,856 — discrimination,
    /// not mere firing. So naming a gap count must still refuse a set of any OTHER
    /// size, in both directions. On this empty scratch database the guard rejects 0,
    /// so expecting any non-zero count must abort: a guard that cannot fail is not a
    /// guard, and this proves the reject arm is still reachable after the re-spec.
    #[tokio::test]
    async fn naming_an_expected_gap_count_still_refuses_a_different_one() {
        let db = scratch().await;
        let sup = Supervisor::new(db.clone(), "archive".into(), reqwest::Client::new());

        let err = sup
            .run_spec(&job(Spec::MarkSkippedSiblings {
                dry_run: false,
                expect: Some(0),
                expect_gaps: Some(154),
            }))
            .await
            .expect_err("a gap count that disagrees with reality must abort the run");
        assert!(
            err.contains("expected exactly 154"),
            "the refusal must name both the found and expected gap counts, got: {err}"
        );
        // And it must not have been reworded into a claim of data loss.
        assert!(
            !err.contains("data loss") && err.contains("held-but-unextracted"),
            "the rejected rows are held-but-unextracted, not lost: {err}"
        );
    }

    /// Issue 84: the mark job defaults to DRY RUN when the caller omits the flag.
    ///
    /// This is a job that writes ~593k user-facing rows, so the destructive reading
    /// of a missing field must be the safe one. A `serde` default of `false` would
    /// have made a forgotten flag mean "execute" — the wrong way round for an
    /// operation whose whole approval process is built on running the count first.
    #[tokio::test]
    async fn the_mark_job_defaults_to_a_dry_run() {
        let db = scratch().await;
        let sup = Supervisor::new(db.clone(), "archive".into(), reqwest::Client::new());

        sup.enqueue_request(&req("mark-skipped-siblings")).await.unwrap();
        assert_eq!(sup.queued()[0].params, "dry-run", "omitted dry_run must mean dry run");

        let explicit = JobRequest {
            kind: "mark-skipped-siblings".into(),
            dry_run: Some(false),
            ..Default::default()
        };
        sup.enqueue_request(&explicit).await.unwrap();
        assert_eq!(
            sup.queued()[1].params, "execute",
            "writing requires saying so explicitly"
        );

        // And it round-trips the durable queue, so a restart mid-run restores the
        // same mode rather than silently re-reading the default.
        let restarted = Supervisor::new(db.clone(), "archive".into(), reqwest::Client::new());
        restarted.recover().await;
        let q = restarted.queued();
        assert_eq!(q.len(), 2);
        assert_eq!(q[0].params, "dry-run");
        assert_eq!(q[1].params, "execute", "the execute flag survives a restart");
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
            let recovered = queue
                .iter()
                .find(|j| j.kind == "process")
                .expect("the process job is restored");
            assert_eq!(
                recovered.resume_after.as_deref(),
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

    /// Issue 222: the weekday catch-up watches `latest_ted_issue_now` for the day's
    /// package landing, so it must reflect the newest registered TED daily. `None`
    /// on a fresh box (so a first-issue `Some(_) > None` reads as "landed").
    #[tokio::test]
    async fn latest_ted_issue_now_reflects_the_newest_registered_daily() {
        let db = scratch().await;
        let sup = Supervisor::new(db.clone(), "archive".into(), reqwest::Client::new());
        assert_eq!(sup.latest_ted_issue_now().await, None, "no TED daily registered yet");

        let (year, _, _) = fetch::civil_date(store::now_unix());
        for issue in ["00137", "00138"] {
            db.record_fetch(&store::Fetch {
                source: "ted".into(),
                kind: "daily".into(),
                period: format!("{year}-{issue}"),
                url: "u".into(),
                sha256: "aa".into(),
                bytes: 1,
                fetched_at: 0,
                path: "p".into(),
            })
            .await
            .unwrap();
        }
        assert_eq!(
            sup.latest_ted_issue_now().await,
            Some(138),
            "the catch-up must see the newest TED daily issue"
        );
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
