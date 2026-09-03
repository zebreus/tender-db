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

use ingest::{doe, fetch, package, process, project, ted};
use model::ingestion::{Ingestion, JobProgress, JobRun, Phase, QueuedJob};
use serde::{Deserialize, Serialize};
use store::turso;
use tokio::sync::{Notify, OnceCell};

/// How many recent runs the dashboard/admin log shows.
const RECENT_RUNS: i64 = 20;

/// The deepest job-log page `GET /admin/jobs?limit=` will serve.
///
/// [`RECENT_RUNS`] is the DEFAULT, not a ceiling — and for a while it was
/// silently both, which cost an hour on 2026-08-30: a census delta needed
/// "what touched the org layer yesterday?", every `?limit=` was ignored, 20
/// rows covered only back to 00:11, and the answer finally came from report
/// `computed_at` stamps by accident. The neighbouring [`CATCH_UP_SCAN`]
/// comment had already measured the same trap ("those 20 rows covered 18.9
/// hours"). Bounded, because this is a reader-pool query an operator can
/// aim at a 12.6M-row corpus.
const JOB_LOG_MAX: i64 = 200;

/// How far back the startup catch-up reads the job log for this morning's probe
/// (issue 245). Much deeper than [`RECENT_RUNS`] on purpose: a maintenance-heavy
/// morning fills 20 rows in under a day — on 2026-08-19 those 20 rows covered 18.9
/// hours — and a probe that falls out of the window reads as "never ran", which would
/// re-run a daily that already happened. Cheap either way: one reader-pool query, once,
/// at startup.
const CATCH_UP_SCAN: i64 = 200;

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
            let sup = Arc::new(Supervisor::new(db, archive, Supervisor::fetch_client()));
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
    /// The id of a RUNNING job an operator has asked to stop, or 0 (issue 247).
    ///
    /// `cancel` could only ever reach QUEUED jobs, and the one time that mattered — a
    /// re-parse crawling at 153 ms a notice, holding the queue against the index build
    /// that would have fixed it — the only ways to stop it were a service-environment
    /// escape hatch and a restart that re-ran it from the top. A long job that cannot be
    /// stopped is a long job nobody can afford to start.
    cancel_running: AtomicU64,
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

/// Org rows scanned per merge batch (issue 234). Each batch is one bounded
/// index-range read plus that range's repoints in one transaction, with a
/// TRUNCATE checkpoint between batches (issue 42's shape).
const ORG_MERGE_BATCH: i64 = 20_000;

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
    /// `packages` caps how many archive packages one run touches (issue 244): a
    /// 215-package, 3.8M-notice era is not a thing to launch unmeasured, and the
    /// resume cursor means a capped run is a PREFIX of the full one rather than a
    /// different job — run it again and it continues where the cap stopped.
    Reparse { profiles: Vec<String>, packages: Option<usize>, after: Option<i64> },
    /// Run the semantic data-quality measurement and store its report (issue 230).
    /// `confirmed` is the operator's explicit "yes, tie the queue up for this" —
    /// see the enqueue arm.
    DataQuality { confirmed: bool },
    /// The D4 immutability probe (issue 173 / dr-premise §6): re-download a
    /// SAMPLE of historical packages with `refetch:true` and let the fetch
    /// path's hash compare say whether upstream still serves what we ingested.
    /// A drifted package is versioned into the archive (never overwritten) and
    /// named in the stored report; a vanished one is named too. The sample
    /// cursor cycles the whole registry over successive runs, so re-fetchability
    /// drift — the re-ingest DR premise's unverified assumption, measurable only
    /// while the original hashes still exist — accumulates coverage weekly.
    RehashProbe { samples: usize },
    /// The D5 reveal recheck (issue 173): measure whether BT-198 "publish
    /// later" promises are kept — of the withheld fields whose reveal date has
    /// passed, how many were actually revealed by a later notice version, and
    /// how many are the source's standing reveal debt. Read-only; stores a
    /// report. A unit variant → durable.
    RevealRecheck,
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
    /// Stamp `tenders.current_title` from each head version's texts (issue 239).
    ///
    /// The `BackfillDeadlines` twin, one column over, and needed for the same reason:
    /// the fold maintains the column for Tenders it touches, so without a one-time
    /// walk every pre-239 row reads NULL and `v_tenders` shows no title.
    BackfillTitles,
    /// Stamp `tenders.current_value_eur_cents` from each head version's amounts
    /// (ADR-0014 D5). The `BackfillDeadlines` twin: the fold maintains the
    /// column for Tenders it touches, so without a one-time walk every
    /// pre-D5-fold row reads NULL and the value bounds match nothing. Run AFTER
    /// the eur_cents backfill refold — it aggregates the satellite's derived
    /// column, so stamping before the refold just writes NULLs.
    BackfillValues,
    /// ADR-0013 D3's third leg (2026-09-02): stamp `tender_versions.original_lang`
    /// on the standing corpus from the notice-level language codes already in
    /// `notice_codes` — a batched PK-seek walk, not a refold. New folds write the
    /// column directly; this is for the ~14.3M rows that predate it.
    BackfillOriginalLang,
    /// Re-derive `eur_cents` across all four money loci from the CURRENT rates
    /// table (issue 306): a rowid-windowed walk per locus recomputing each
    /// row's EUR sibling from (cents, currency, version publication date) via
    /// the in-memory lookup, updating only rows whose value changes. The
    /// repair for a poisoned/incomplete rates load — hours cheaper than a
    /// whole-corpus refold and quiet on the change feed, because the corpus
    /// content did not change, only the derived-beside layer. Run
    /// `backfill-values` after so the head column follows.
    RederiveEur,
    /// Repair the stale nested-org mention layer (issue 259 landing): the
    /// 2026-08-20 alias fix changed what `mentions()` emits, but the resolver's
    /// idempotency keeps recorded (notice, section) bindings, so refolds never
    /// re-route them. Walks nameless provisional orgs; where one is the outer
    /// wrapper of a nested Organization pair, repoints its references to the
    /// named inner org (merge machinery shape) and deletes the empty row.
    /// `dry_run` counts and writes nothing — the safe default, like the other
    /// org-mutating jobs.
    RepairNestedOrgs { dry_run: bool },
    /// Populate the `organization_names` satellite for the standing corpus
    /// (issue 307): the resolver writes variants only for NEWLY recorded
    /// mentions, so everything mentioned before the D4 deploy needs this one
    /// notice-windowed walk. Idempotent (REPLACE, stable row count).
    BackfillOrgNameVariants,
    /// Issue 300 Stage 0: the org-merge-health census — distinct N2 mention
    /// names per identifier-bearing org, whole corpus, read-only. Its report
    /// is the matcher's baseline (distribution + top-100 allowlist input) and,
    /// from Stage 1 on, the bad-merge tripwire's weekly input: a placeholder
    /// identifier merging strangers shows up as one org accreting distinct
    /// names (DE123456789 reached 144 before it was caught by hand).
    OrgMergeHealth,
    R2Census,
    R3Census,
    /// Issue 314: size the candidate-edge store — components, not edges,
    /// are review cases, so a campaign cannot be scoped without this.
    /// Read-only.
    OrgEdgeCensus,
    MatchOrgIdentifiersR2 { dry_run: bool, max_groups: Option<u64> },
    MatchOrgIdentifiersR3 { dry_run: bool, max_groups: Option<u64> },
    ApplyCaseReviews { dry_run: bool },
    /// Issue 312: the symmetric UNDO for `ApplyCaseReviews` — restore the
    /// identifiers whose pre-image value is a platform GUID, which the
    /// measurement showed were linking keys rather than false merge keys.
    /// Writes entity rows, so `dry_run` defaults TRUE like its twin.
    UnapplyCaseReviews { dry_run: bool },
    /// Issue 317 Units B/C: list the parked review verdicts nothing
    /// consumes, with the evidence needed to close them. Read-only.
    CaseReviewBacklog,
    /// Issue 319: fold non-canonical country codes on organization rows.
    FoldOrgCountries { dry_run: bool },
    /// Issue 317 Unit A: which reviewed vehicle rows hold mentions naming
    /// somebody else. Read-only.
    FusionCensus,
    /// Issue 317 Unit A: the reviewer's input — the off-name mentions still
    /// awaiting a verdict, with the standing rows each could move to.
    /// Read-only.
    RehomingPacket,
    /// Issue 321: name variants a re-homing left on the ORIGIN that no
    /// remaining mention supports. Read-only measurement.
    SatelliteOrphans,
    /// Issue 321: drop the orphaned variants that already stand on a row the
    /// origin re-homed to. Wet writes `organization_names`.
    DropOrphanSatellites { dry_run: bool },
    /// Issue 321: put back what a drop pass removed, from its pre-images.
    RestoreDroppedSatellites { dry_run: bool, only_job: Option<i64> },
    /// Issue 318: how far apart the resolver's anchor bind and the batch
    /// merge arm stand on the genericness wall. Read-only measurement.
    AnchorWallCensus,
    /// Issues 311 + 314: the reviewer's input for the same-name cross-border
    /// cohort — the first thing to consume org_candidate_edges. Read-only.
    XbPacket,
    /// Issue 326: same identifier, two country codes one letter apart —
    /// SK/SG, CZ/CR, BG/BF. Read-only measurement.
    CountryTypoCensus,
    /// Issue 326 step 1 re-cut: the same class grouped by IDENTIFIER, with the
    /// one-letter test demoted to a corruption filter. Read-only.
    CountryClusterCensus,
    /// Issue 328 follow-on: standing duplicate `(country, kind, identifier)`
    /// triples, split by whether `canonical_key` can see them and — where it
    /// cannot — by whether the member names agree. Read-only measurement.
    DuplicateIdentityCensus,
    /// Issue 330: organization names carrying a line break, and whether the
    /// notice published it that way. Read-only measurement.
    NamePollutionCensus,
    /// Issue 331: does duplication actually push a name key over the
    /// genericness wall? Read-only measurement.
    GenericWallCensus,
    /// Issue 332: are over-cap name keys widely-shared names, or single
    /// identities fragmented? Read-only measurement.
    GenericStatisticCensus,
    /// Issue 334: for the widest name keys, does the stored org name match what
    /// the notices said? Read-only probe.
    NameAttributionProbe,
    /// Issue 169: the storage figures, recorded so report history accumulates a
    /// TREND instead of leaving the next reader to infer one from two samples.
    /// Read-only, and instantaneous — statvfs plus one stat, no inode walk.
    DiskCensus,
    /// Issue 326 step 2: move the rows a decisive anchor says are mistyped.
    /// Wet writes `organizations`.
    RepairCountryTypos { dry_run: bool },
    /// Issue 328: re-parse the rows whose identifier carries a publisher label.
    /// Wet writes `organizations`.
    RepairLabelPrefixes { dry_run: bool },
    /// Issue 325 step 4: re-parse the standing `kind = 'vat'` rows and write
    /// what the identifier parser now says. Wet writes `organizations`.
    RepairMintedCountries { dry_run: bool },
    /// Issue 317 Unit A: move reviewed mentions to the row they describe.
    ApplyRehoming { dry_run: bool },
    BuildOrgMatchKeys { dry_run: bool },
    /// Issue 300 Stage 4: the candidate-edge scan over the key satellite —
    /// E3 name-equality edges into `org_candidate_edges`, advisory only
    /// (never merges; the consumer is the issue-311 review loop). Census
    /// dry-first with T4 parity; `max_edges` is the capped first prod run.
    ScanOrgMatchKeys { dry_run: bool, max_edges: Option<u64> },
    /// Issue 300 Stage 1, the repair half: dissolve organizations whose
    /// identifier the (now live) v2 gate condemns — placeholder-keyed
    /// stranger-mergers like DE123456789/NIMAT500 — re-resolving every
    /// recorded mention through the post-234 provisional path. `dry_run`
    /// counts and writes nothing (the default, like every org-mutating job).
    RepairPlaceholderOrgs { dry_run: bool },
    /// Re-queue a profile cohort for the incremental fold (issue 85): clear its
    /// `projected` watermark so the trailing `project rebuild=false` re-derives just
    /// those Tenders. For a cohort the projection MIS-READ (unmapped field ids) —
    /// the parsed layer is already correct, so no re-parse. `expect` is a guard: the
    /// run aborts BEFORE writing if the cohort is not about that size, which is what
    /// a mistyped profile string looks like.
    Refold { profiles: Vec<String>, expect: Option<u64> },
    /// Re-fold every notice carrying a section of these KINDS (issue 237).
    ///
    /// The `RefoldFields` twin for mappings whose trigger is a section kind rather than a
    /// field id — and much cheaper, because `notice_sections_kind` makes the cohort an
    /// index read where the field sweep must pass whole value tables.
    RefoldSections { kinds: Vec<String> },
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
    /// Issue 278: count the notices whose `caused_by` appears under 2+ Tenders —
    /// the ghost signature — in bounded slices of the notice-id space. READ-ONLY.
    ///
    /// It was the track-2 sweep, which stalled the queue for 40+ minutes on an
    /// unbounded `GROUP BY` and then sat as a no-op. The cleanup it existed for is
    /// done: prod measured zero ghosts across all 29.96M notice ids on 2026-09-02,
    /// the ~45k having drained away with the reparse backlog. What was missing was
    /// anything that would notice them coming back, which is what this now is.
    GhostCensus,
    /// Fetch the ECB daily reference-rate history and load `currency_rates`
    /// (ADR-0014 unit 2): archive the CSV under the ordinary fetch registry
    /// (source `ecb`, kind `rates`, period = fetch date), parse, seed the
    /// irrevocable euro conversion rates, and chunk-upsert the daily rows.
    FetchRates,
    /// Fetch the pre-1999 daily ECU series (ADR-0014 D2a) — the Commission's
    /// Official Journal rates as Eurostat carries them, split across TWO
    /// datasets (`ert_h_eur_d`: the former national currencies; `ert_bil_eur_d`:
    /// the rest) — and load 1993-1998 into `currency_rates` as `eurostat-ecu`
    /// rows. One-time historical load: the series is closed (ECU→EUR 1:1 on
    /// 1999-01-01, Council Regulation 1103/97), so a re-run is hash-idempotent
    /// through the fetch registry and REPLACE-idempotent in the table.
    FetchRatesEcu,
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
    /// Collapse the standing stock of duplicate identifier-less provisional
    /// Organizations into one row per `(name_norm, country)` and repoint every
    /// referencing row (issue 234's backfill half — the resolver merge only
    /// PREVENTS new duplicates; no fold can retro-collapse the existing ones,
    /// because the mention idempotency preload never re-resolves a recorded
    /// mention). Batched + checkpointed, idempotent (merged groups leave the
    /// scan's scope), stoppable between batches. `dry_run` counts and writes
    /// nothing.
    MergeProvisionalOrgs { dry_run: bool },
    /// Clear a STALE `rebuild_in_progress` flag (the issue-85 interlock's escape
    /// hatch). The flag routes any `project` — including the 09:35 daily tick — into
    /// the salvage branch, which `reset_tender_layer()`s a good layer; a rebuild that
    /// completed cleanly clears it itself, so a flag left set over an intact layer is
    /// stale by definition. [`Db::clear_plan`] clears it and retires the plan in one
    /// transaction. Fire ONLY with the layer verified intact; jobs run sequentially,
    /// so this cannot race a projection. Reports whether it actually cleared anything.
    ClearRebuildFlag,
}

/// Issue 325 step 5: which parser-vs-stock counters have moved far enough to
/// alarm, given the previous `org-merge-health` report's block as the baseline.
///
/// A free function because the comparison is the whole tripwire, and a
/// comparison buried inside a job body cannot be tested — which is how the
/// muted-probe bug (issue 300 Stage 4 Unit 5) got in: a tripwire whose only
/// proof of being wired was that the job ran.
///
/// **The baseline is the previous run of the same report**, so the floor tracks
/// the corpus instead of being a constant that goes stale as the residue is
/// worked down. Two counters get a tolerance — the org layer grows daily and a
/// few new ambiguous rows are normal traffic. `vat_refused` gets none: the
/// parser refusing a value it used to accept means the v2 gate moved under the
/// standing rows, and one such row is worth a look.
/// The counters the next run's comparison actually reads, and nothing else
/// (issue 325's tripwire; the nesting fixed 2026-09-02).
///
/// A report that quotes its predecessor whole quotes its predecessor's
/// predecessor with it. This keeps the human-readable "what it was compared
/// against" line without the recursion: flat, three numbers, same shape every
/// week however long the series runs.
///
/// **Three, not four.** `gln_shared_one_country` is deliberately absent:
/// [`parser_vs_stock_alarms`] treats it as a ZERO-FLOOR check — any value above
/// zero alarms, whatever last week said — so it keeps no baseline by design. The
/// first cut of this helper included it anyway and produced a field that was
/// structurally always null, which is an invitation for someone to later "fix"
/// the wiring of a comparison that does not exist.
fn trimmed_baseline(previous: Option<&serde_json::Value>) -> serde_json::Value {
    let Some(before) = previous.map(|v| &v["parser_vs_stock"]) else {
        return serde_json::Value::Null;
    };
    if !before.is_object() {
        return serde_json::Value::Null;
    }
    let pick = |k: &str| before.get(k).cloned().unwrap_or(serde_json::Value::Null);
    serde_json::json!({
        "no_longer_vat": pick("no_longer_vat"),
        "vat_country_differs": pick("vat_country_differs"),
        "vat_refused": pick("vat_refused"),
    })
}

fn parser_vs_stock_alarms(
    before: Option<&serde_json::Value>,
    no_longer_vat: u64,
    vat_country_differs: u64,
    vat_refused: u64,
    gln_shared_one_country: u64,
) -> Vec<String> {
    let mut alarms = Vec::new();
    let mut watch = |label: &str, current: u64, floor: Option<u64>| {
        // No baseline (the first run) is not an alarm. A census that shouted on
        // its own arrival would be muted by the second week.
        let Some(was) = floor else { return };
        let tolerance = std::cmp::max(was / 10, 25);
        if current > was + tolerance {
            alarms.push(format!("{label} {was} -> {current}"));
        }
    };
    watch("no_longer_vat", no_longer_vat, before.and_then(|b| b["no_longer_vat"].as_u64()));
    watch(
        "vat_country_differs",
        vat_country_differs,
        before.and_then(|b| b["vat_country_differs"].as_u64()),
    );
    if vat_refused > 0 {
        alarms.push(format!("vat_refused {vat_refused} (floor is 0)"));
    }
    // Issue 327, and it needs no baseline either. A 9110 GLN held by more than
    // one row is a class that is wrong about two thirds of the time; what keeps
    // those rows apart today is only that they stand under DIFFERENT countries,
    // since R2 keys on `(country, kind, identifier)`. One of them collapsing
    // onto a single country means that guard is gone — so the floor is zero and
    // there is no tolerance to spend.
    if gln_shared_one_country > 0 {
        alarms.push(format!(
            "gln_shared_one_country {gln_shared_one_country} (floor is 0 — a shared \
             Austrian GLN under ONE country is a merge path that has opened)"
        ));
    }
    alarms
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
    /// `restore-dropped-satellites` only: undo just this drop job's pass
    /// (issue 321). Omitted means every outstanding pre-image — which is the
    /// right default for one pass and the wrong one for two, so an operator
    /// unwinding a bad run names it.
    pub job: Option<i64>,
    pub profile: Option<String>,
    /// `reprocess` only: skip the trailing incremental fold, leaving reclaimed
    /// notices `projected=0` for one later `rebuild:true` to fold in bulk.
    pub reclaim_only: Option<bool>,
    /// `reparse` only: stop after this many archive packages (issue 244). The run is
    /// resumable per package, so a cap makes a staged first pass — measure the rate
    /// and check the output on real data — a prefix of the full era rather than a
    /// separate exercise. Omitted means every package the profile has.
    pub packages: Option<usize>,
    /// `reparse` only: start at the first package whose `fetch_id` exceeds this
    /// (issue 244). Package ids are ingestion order, NOT publication order — the text
    /// era's `fetch 186` is 2010-12 and `fetch 240` is 2006-06 — so a staged run that
    /// wants a particular vintage has to say which package, not just how many. Real
    /// resume progress overrides it once the job has advanced past it.
    pub after: Option<i64>,
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
    /// `match-org-identifiers` wet runs only: merge at most this many groups —
    /// the design's capped first prod run (issue 300 Stage 2). Omitted means
    /// the whole plan.
    pub max_groups: Option<u64>,
    /// `match-org-identifiers` only: which merge rule to run — `r2` (the
    /// Stage-2 same-country canonical-key merge, the default) or `r3` (the
    /// Stage-3 NULL-country checksum-anchor rescue). Anything else is
    /// rejected at enqueue.
    pub rule: Option<String>,
    /// `scan-org-match-keys` wet runs only: write at most this many edges —
    /// the design's capped first prod run (issue 300 Stage 4). The census
    /// still runs whole, so parity stays a whole-plan check. Omitted means
    /// everything the census found.
    pub max_edges: Option<u64>,
}

impl Supervisor {
    /// The store handle, for read-only operator surfaces that need it (the stored
    /// report reader, issue 230). An accessor rather than a public field on purpose:
    /// a caller can read, and the queue stays the only way to make the supervisor
    /// write.
    pub fn db(&self) -> &store::Db {
        &self.db
    }

    /// The production fetch client for every TED/DÖE/rehash download. Unlike the
    /// webhook client (a flat 10s total `.timeout()` — deliveries are tiny), a
    /// package download legitimately streams for many minutes, so a total timeout
    /// is wrong: it would abort a healthy large monthly. Instead bound the two
    /// failure modes a large steady download never hits — connection setup, and a
    /// stall between bytes (issue 280: `reqwest::Client::new()` had neither, so a
    /// half-open or slow-loris upstream on `send()`/`chunk()` in `download_once`
    /// hung the single serialized worker forever, with no watchdog and no cancel
    /// path for the un-stoppable probe/fetch/rehash kinds). `read_timeout` resets
    /// on each successful read, so it caps only idle gaps, never total transfer.
    fn fetch_client() -> reqwest::Client {
        reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(30))
            .read_timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("static fetch-client config cannot fail to build")
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
            cancel_running: AtomicU64::new(0),
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

    /// How long a queue persist may wait for the writer before the job is queued in
    /// memory only (issue 256).
    ///
    /// 30 s, not 5: an ordinary `process`/`project` chunk holds the writer for seconds at
    /// a time and a persist that lands mid-chunk must still get its durable row. Nothing
    /// legitimate holds it for half a minute — the case this exists for held it for hours.
    const PERSIST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

    /// Await one queue persist, giving up after [`Self::PERSIST_TIMEOUT`] (issue 256).
    /// Returns whether the durable row exists — the caller queues the job either way, which
    /// is the whole point: a job that cannot be written down still has to run.
    ///
    /// Free function over the future rather than a method on `self`, so the timeout branch
    /// is testable with a paused clock and a future that never completes. Verifying it
    /// against a real held writer would mean a public "hold the writer" hook in the store
    /// for one test; the prod evidence in issue 256 covers that end.
    async fn persist_queued<F>(id: u64, kind: &str, within: std::time::Duration, persist: F) -> bool
    where
        F: std::future::Future<Output = turso::Result<()>>,
    {
        match tokio::time::timeout(within, persist).await {
            Ok(Ok(())) => true,
            Ok(Err(e)) => {
                eprintln!("supervisor: persist queued job {id}: {e}");
                false
            }
            Err(_) => {
                eprintln!(
                    "supervisor: persist queued job {id} ({kind}) gave up after {}s — a long job \
                     is holding the writer (issue 256). The job IS queued in memory and will run, \
                     but a restart before it does will lose it.",
                    within.as_secs()
                );
                false
            }
        }
    }

    async fn push(&self, kind: &'static str, params: String, spec: Spec) -> u64 {
        self.enqueue(kind, params, spec, false).await
    }

    /// Enqueue at the FRONT — for work the rest of the queue depends on (issue 247).
    ///
    /// Only the deferred-index bootstrap uses it, and the reason is a measured deadlock:
    /// a missing index made a queued re-parse take 153 ms a notice instead of
    /// microseconds, and the Reindex that would have fixed it sat behind that same
    /// re-parse for hours. Ordinary work must never jump the queue — an operator's
    /// sequence is a sequence — but a job whose absence is what makes the queue slow is
    /// not ordinary work.
    ///
    /// In-memory only: a restart rebuilds the queue from the durable rows in id order,
    /// so the priority is lost across a restart and `ensure_deferred_indexes` re-applies
    /// it on the next boot (it runs before the worker, every time).
    async fn push_front(&self, kind: &'static str, params: String, spec: Spec) -> u64 {
        self.enqueue(kind, params, spec, true).await
    }

    async fn enqueue(&self, kind: &'static str, params: String, spec: Spec, front: bool) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        // Persist before enqueuing in memory: the durable row is what a restart
        // rebuilds the queue from, so it must exist first (issue 21). Best-effort
        // like the run log — a failed persist still runs this session, it just
        // won't survive a restart.
        let spec_json = serde_json::to_string(&spec).expect("job spec serializes");
        // …but never for longer than [`Self::PERSIST_TIMEOUT`]. The persist takes the
        // single writer connection, and a long job holds that writer for its whole run:
        // on 2026-08-20 a 2.75-hour fold parked the 09:35 daily tick inside this call, so
        // the day's ingest never happened and NOTHING said so — no log line, no queued
        // job, because the in-memory push below only happens after the persist returns
        // (issue 256). Honouring the best-effort contract stated above means giving up on
        // the row rather than on the job.
        Self::persist_queued(
            id,
            kind,
            Self::PERSIST_TIMEOUT,
            self.db.enqueue_job(id as i64, kind, &params, &spec_json),
        )
        .await;
        let job = Job { id, kind: kind.to_owned(), params, spec, resume_after: None };
        {
            let mut queue = self.queue.lock().expect("queue lock");
            if front {
                queue.push_front(job);
            } else {
                queue.push_back(job);
            }
        }
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
                let packages = req.packages;
                let after = req.after;
                let mut params = format!("reparse {}", profiles.join(","));
                if let Some(from) = after {
                    params.push_str(&format!(" after fetch {from}"));
                }
                if let Some(n) = packages {
                    params.push_str(&format!(" (first {n} package(s))"));
                }
                let mut ids = vec![
                    self.push("reparse", params, Spec::Reparse { profiles, packages, after }).await,
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
            // The D4 immutability probe (issue 173): `packages` caps the sample
            // (default 8 — a weekly 8 cycles the whole registry in about a year
            // at today's size, and the cursor makes any cadence a continuation).
            "rehash-probe" => {
                let samples = req.packages.unwrap_or(8).max(1);
                Ok(vec![
                    self.push(
                        "rehash-probe",
                        format!("rehash probe ({samples} package(s))"),
                        Spec::RehashProbe { samples },
                    )
                    .await,
                ])
            }
            "reveal-recheck" => Ok(vec![
                self.push("reveal-recheck", "reveal recheck".into(), Spec::RevealRecheck).await,
            ]),
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
            "backfill-titles" => Ok(vec![
                self.push("backfill-titles", "backfill-titles".into(), Spec::BackfillTitles).await,
            ]),
            "backfill-deadlines" => Ok(vec![
                self.push("backfill-deadlines", "backfill-deadlines".into(), Spec::BackfillDeadlines)
                    .await,
            ]),
            // ADR-0014 D5: stamp the head-value EUR column (run after the
            // eur_cents backfill refold).
            "backfill-original-lang" => Ok(vec![
                self.push(
                    "backfill-original-lang",
                    "backfill-original-lang".into(),
                    Spec::BackfillOriginalLang,
                )
                .await,
            ]),
            "backfill-values" => Ok(vec![
                self.push("backfill-values", "backfill-values".into(), Spec::BackfillValues).await,
            ]),
            // Issue 306: re-derive the four loci's eur_cents from the current
            // rates table, then follow with backfill-values for the head column.
            "rederive-eur" => Ok(vec![
                self.push("rederive-eur", "rederive-eur".into(), Spec::RederiveEur).await,
                self.push("backfill-values", "backfill-values".into(), Spec::BackfillValues).await,
            ]),
            // Issue 307: one-time satellite population for the standing corpus.
            "backfill-org-name-variants" => Ok(vec![
                self.push(
                    "backfill-org-name-variants",
                    "backfill-org-name-variants".into(),
                    Spec::BackfillOrgNameVariants,
                )
                .await,
            ]),
            // Issue 300 Stage 0: read-only distinct-name census per
            // identifier-bearing org — the matcher baseline + standing
            // bad-merge tripwire input.
            "org-merge-health" => Ok(vec![
                self.push("org-merge-health", "org-merge-health".into(), Spec::OrgMergeHealth)
                    .await,
            ]),
            // Issue 300 Stage 2 opening census: read-only canonical-key
            // grouping preview — what the R2 same-country merge WOULD find,
            // measured before any merge code exists.
            "r2-census" => {
                Ok(vec![self.push("r2-census", "r2-census".into(), Spec::R2Census).await])
            }
            // Issue 314: the candidate-edge store's own census. Read-only,
            // and the measure-first gate on any review campaign over edges.
            "org-edge-census" => {
                Ok(vec![self.push("org-edge-census", "org-edge-census".into(), Spec::OrgEdgeCensus).await])
            }
            // Issue 300 Stage 3 opening census: classify the NULL-country
            // rescue pool by checksum anchoring + name corroboration.
            // Read-only.
            "r3-census" => {
                Ok(vec![self.push("r3-census", "r3-census".into(), Spec::R3Census).await])
            }
            // Issue 300 Stage 2: the R2 same-country canonical-key merge.
            // Deletes org rows: dry_run defaults TRUE, and a wet run REQUIRES
            // a stored dry-run plan (the T4 parity input) — there is no way
            // to run it un-previewed.
            "match-org-identifiers" => {
                let dry_run = req.dry_run.unwrap_or(true);
                let max_groups = req.max_groups;
                let rule = req.rule.as_deref().unwrap_or("r2");
                let spec = match rule {
                    "r2" => Spec::MatchOrgIdentifiersR2 { dry_run, max_groups },
                    "r3" => Spec::MatchOrgIdentifiersR3 { dry_run, max_groups },
                    other => {
                        return Err(format!(
                            "match-org-identifiers: unknown rule '{other}' (r2 or r3)"
                        ));
                    }
                };
                let params = match (dry_run, max_groups) {
                    (true, _) => format!("match-org-identifiers {rule} dry-run"),
                    (false, Some(cap)) => format!("match-org-identifiers {rule} cap={cap}"),
                    (false, None) => format!("match-org-identifiers {rule}"),
                };
                Ok(vec![self.push("match-org-identifiers", params, spec).await])
            }
            // Issue 311: apply the safe subset of recorded per-case review
            // verdicts (identifier strips). Entity-writing: dry_run defaults
            // TRUE; verdicts land beforehand via POST /admin/case-reviews.
            "apply-case-reviews" => {
                let dry_run = req.dry_run.unwrap_or(true);
                let params = if dry_run {
                    "apply-case-reviews dry-run"
                } else {
                    "apply-case-reviews"
                }
                .to_owned();
                Ok(vec![
                    self.push("apply-case-reviews", params, Spec::ApplyCaseReviews { dry_run })
                        .await,
                ])
            }
            // Issue 312: undo the strips whose pre-image was a platform
            // GUID (a linking key, not a merge key — measured). Selection
            // is by value SHAPE, never by the reviewer's diagnosis prose.
            "unapply-case-reviews" => {
                let dry_run = req.dry_run.unwrap_or(true);
                let params = if dry_run {
                    "unapply-case-reviews dry-run (select=uuid-v4)"
                } else {
                    "unapply-case-reviews (select=uuid-v4)"
                }
                .to_owned();
                Ok(vec![
                    self.push(
                        "unapply-case-reviews",
                        params,
                        Spec::UnapplyCaseReviews { dry_run },
                    )
                    .await,
                ])
            }
            // Issue 317 Unit A: the re-homing apply. Moves entity
            // references, so dry_run defaults TRUE.
            "apply-rehoming" => {
                let dry_run = req.dry_run.unwrap_or(true);
                Ok(vec![
                    self.push(
                        "apply-rehoming",
                        if dry_run { "apply-rehoming dry-run" } else { "apply-rehoming" }
                            .to_owned(),
                        Spec::ApplyRehoming { dry_run },
                    )
                    .await,
                ])
            }
            // Issue 317 Unit A: the fusion census. Read-only.
            "fusion-census" => Ok(vec![
                self.push("fusion-census", "fusion-census".into(), Spec::FusionCensus).await,
            ]),
            // Issue 319: the country fold. Writes a published field on
            // entity rows, so dry_run defaults TRUE like every other writer.
            "fold-org-countries" => {
                let dry_run = req.dry_run.unwrap_or(true);
                Ok(vec![
                    self.push(
                        "fold-org-countries",
                        if dry_run { "fold-org-countries dry-run" } else { "fold-org-countries" }
                            .to_owned(),
                        Spec::FoldOrgCountries { dry_run },
                    )
                    .await,
                ])
            }
            // Issue 321: the repair, dry by default. The wet arm compares
            // itself against the stored dry plan as tuples before it writes.
            "drop-orphan-satellites" => {
                let dry_run = req.dry_run.unwrap_or(true);
                let params = if dry_run {
                    "drop-orphan-satellites dry-run"
                } else {
                    "drop-orphan-satellites"
                }
                .to_owned();
                Ok(vec![
                    self.push(
                        "drop-orphan-satellites",
                        params,
                        Spec::DropOrphanSatellites { dry_run },
                    )
                    .await,
                ])
            }
            "restore-dropped-satellites" => {
                let dry_run = req.dry_run.unwrap_or(true);
                let params = if dry_run {
                    "restore-dropped-satellites dry-run"
                } else {
                    "restore-dropped-satellites"
                }
                .to_owned();
                Ok(vec![
                    self.push(
                        "restore-dropped-satellites",
                        params,
                        Spec::RestoreDroppedSatellites { dry_run, only_job: req.job },
                    )
                    .await,
                ])
            }
            // Issue 321: the measurement that decides whether the leftover
            // name variants need machinery or a line in the review schema.
            // Issues 311 + 314: the same-name cohort's review packet.
            "xb-packet" => Ok(vec![
                self.push("xb-packet", "xb-packet".into(), Spec::XbPacket).await,
            ]),
            // Issue 325 step 4: the repair for the countries minted out of a
            // word. Dry by default; the wet arm reads the row count out of the
            // stored dry plan and aborts if the corpus has moved (T4 parity).
            "repair-minted-countries" => {
                let dry_run = req.dry_run.unwrap_or(true);
                Ok(vec![
                    self.push(
                        "repair-minted-countries",
                        if dry_run {
                            "repair-minted-countries dry-run"
                        } else {
                            "repair-minted-countries"
                        }
                        .to_owned(),
                        Spec::RepairMintedCountries { dry_run },
                    )
                    .await,
                ])
            }
            // Issue 326: size the country-typo class before building a repair
            // for it. 18 of 18 such cases the issue-314 campaign reviewed came
            // back wrong-country, so the census is the cheap half of a repair
            // that already has its evidence.
            // Issue 326 step 1 re-cut. The pair census stays: it is a valid
            // measurement of a different unit, and the two disagree in ways
            // worth keeping visible.
            // Issue 326 step 2, dry by default. The wet arm reads its row count
            // out of the stored dry plan and aborts if the corpus has moved.
            // Issue 328, dry by default. The wet arm reads its row count out of
            // the stored dry plan and aborts if the corpus has moved.
            "repair-label-prefixes" => {
                let dry_run = req.dry_run.unwrap_or(true);
                Ok(vec![
                    self.push(
                        "repair-label-prefixes",
                        if dry_run { "repair-label-prefixes dry-run" } else { "repair-label-prefixes" }
                            .to_owned(),
                        Spec::RepairLabelPrefixes { dry_run },
                    )
                    .await,
                ])
            }
            "repair-country-typos" => {
                let dry_run = req.dry_run.unwrap_or(true);
                Ok(vec![
                    self.push(
                        "repair-country-typos",
                        if dry_run { "repair-country-typos dry-run" } else { "repair-country-typos" }
                            .to_owned(),
                        Spec::RepairCountryTypos { dry_run },
                    )
                    .await,
                ])
            }
            "country-cluster-census" => Ok(vec![
                self.push(
                    "country-cluster-census",
                    "country-cluster-census".into(),
                    Spec::CountryClusterCensus,
                )
                .await,
            ]),
            "disk-census" => Ok(vec![
                self.push("disk-census", "disk-census".into(), Spec::DiskCensus).await,
            ]),
            "name-attribution-probe" => Ok(vec![
                self.push(
                    "name-attribution-probe",
                    "name-attribution-probe".into(),
                    Spec::NameAttributionProbe,
                )
                .await,
            ]),
            "generic-statistic-census" => Ok(vec![
                self.push(
                    "generic-statistic-census",
                    "generic-statistic-census".into(),
                    Spec::GenericStatisticCensus,
                )
                .await,
            ]),
            "generic-wall-census" => Ok(vec![
                self.push(
                    "generic-wall-census",
                    "generic-wall-census".into(),
                    Spec::GenericWallCensus,
                )
                .await,
            ]),
            "name-pollution-census" => Ok(vec![
                self.push(
                    "name-pollution-census",
                    "name-pollution-census".into(),
                    Spec::NamePollutionCensus,
                )
                .await,
            ]),
            "duplicate-identity-census" => Ok(vec![
                self.push(
                    "duplicate-identity-census",
                    "duplicate-identity-census".into(),
                    Spec::DuplicateIdentityCensus,
                )
                .await,
            ]),
            "country-typo-census" => Ok(vec![
                self.push(
                    "country-typo-census",
                    "country-typo-census".into(),
                    Spec::CountryTypoCensus,
                )
                .await,
            ]),
            // Issue 318: size the disagreement before deciding how hard to
            // close it — the issue's own step 3.
            "anchor-wall-census" => Ok(vec![
                self.push(
                    "anchor-wall-census",
                    "anchor-wall-census".into(),
                    Spec::AnchorWallCensus,
                )
                .await,
            ]),
            // Issue 321: the measurement that decides whether the leftover
            // name variants need machinery or a line in the review schema.
            "satellite-orphans" => Ok(vec![
                self.push("satellite-orphans", "satellite-orphans".into(), Spec::SatelliteOrphans)
                    .await,
            ]),
            // Issue 317 Unit A: the review packet. Read-only; the verdicts
            // it feeds arrive over POST /admin/rehoming, and `apply-rehoming`
            // is the only thing that writes.
            "rehoming-packet" => Ok(vec![
                self.push("rehoming-packet", "rehoming-packet".into(), Spec::RehomingPacket).await,
            ]),
            // Issue 317 Units B/C: the parked-verdict backlog. Read-only,
            // so no dry_run — there is nothing for a flag to protect.
            "case-review-backlog" => Ok(vec![
                self.push(
                    "case-review-backlog",
                    "case-review-backlog".into(),
                    Spec::CaseReviewBacklog,
                )
                .await,
            ]),
            // Issue 300 Stage 4: (re)build the org_match_keys scratch
            // satellite. Rebuildable, no entity writes; dry_run defaults
            // TRUE and measures the walk without writing.
            "build-org-match-keys" => {
                let dry_run = req.dry_run.unwrap_or(true);
                let params = if dry_run {
                    "build-org-match-keys dry-run"
                } else {
                    "build-org-match-keys"
                }
                .to_owned();
                Ok(vec![
                    self.push("build-org-match-keys", params, Spec::BuildOrgMatchKeys { dry_run })
                        .await,
                ])
            }
            // Issue 300 Stage 4: the candidate-edge scan. Advisory edges
            // only — it never merges — but dry_run still defaults TRUE:
            // the census IS the reviewed pre-estimate the T4 ladder gates
            // wet runs on. Params carry the keys-build epoch (audit line).
            "scan-org-match-keys" => {
                let dry_run = req.dry_run.unwrap_or(true);
                let max_edges = req.max_edges;
                let params = format!(
                    "scan-org-match-keys{}{} stoplist={} epoch={}",
                    if dry_run { " dry-run" } else { "" },
                    match max_edges {
                        Some(cap) => format!(" cap={cap}"),
                        None => String::new(),
                    },
                    SCAN_STOPLIST_CAP,
                    ingest::crosswalk::NAME_KEY_EPOCH
                );
                Ok(vec![
                    self.push(
                        "scan-org-match-keys",
                        params,
                        Spec::ScanOrgMatchKeys { dry_run, max_edges },
                    )
                    .await,
                ])
            }
            // Issue 300 Stage 1 repair: dissolve v2-gate-condemned orgs.
            // Deletes org rows and emits change events: dry_run defaults TRUE.
            "repair-placeholder-orgs" => {
                let dry_run = req.dry_run.unwrap_or(true);
                let params = if dry_run {
                    "repair-placeholder-orgs dry-run"
                } else {
                    "repair-placeholder-orgs"
                }
                .to_owned();
                Ok(vec![
                    self.push(
                        "repair-placeholder-orgs",
                        params,
                        Spec::RepairPlaceholderOrgs { dry_run },
                    )
                    .await,
                ])
            }
            // Issue 259 landing: repair the stale nested-org mention layer.
            // Deletes org rows and emits change events, so it asks to be meant:
            // `dry_run` defaults to TRUE (the data-quality convention — a
            // forgotten flag must mean the harmless thing).
            "repair-nested-orgs" => {
                let dry_run = req.dry_run.unwrap_or(true);
                let params = if dry_run {
                    "repair-nested-orgs dry-run"
                } else {
                    "repair-nested-orgs"
                }
                .to_owned();
                Ok(vec![
                    self.push("repair-nested-orgs", params, Spec::RepairNestedOrgs { dry_run })
                        .await,
                ])
            }
            // Escape hatch for a stale rebuild watermark (issue 85 interlock). No-op
            // safe: it reports whether the flag was actually set.
            "clear-rebuild-flag" => {
                Ok(vec![self.push("clear-rebuild-flag", "clear-rebuild-flag".into(), Spec::ClearRebuildFlag).await])
            }
            // Re-project a profile cohort the projection mis-read (issue 85): clear its
            // watermark, then fold it incrementally. Two jobs like `reprocess`, so the
            // fold is a normal queued projection; if the guard aborts the mark, that
            // projection simply finds an empty change-set and returns.
            // ADR-0014 unit 2: load/refresh the EUR-pivot exchange-rate table.
            "fetch-rates" => {
                Ok(vec![
                    self.push("fetch-rates", "ecb eurofxref-hist".into(), Spec::FetchRates).await,
                ])
            }
            // ADR-0014 D2a: load the closed 1993-1998 daily ECU series.
            "fetch-rates-ecu" => Ok(vec![
                self.push("fetch-rates-ecu", "eurostat ecu 1993-1998".into(), Spec::FetchRatesEcu)
                    .await,
            ]),
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
            // Reuses the request's `profiles` list as the KIND list, exactly as
            // `refold-fields` reuses it for field ids: one list-shaped parameter per
            // request, read according to the kind of job asked for.
            "refold-sections" => {
                let kinds = req.profiles.clone().unwrap_or_default();
                if kinds.is_empty() {
                    return Err("refold-sections needs at least one section kind (pass via profiles)".into());
                }
                let params = format!("refold-sections {}", kinds.join(","));
                Ok(vec![
                    self.push("refold-sections", params, Spec::RefoldSections { kinds }).await,
                    self.push("project", "rebuild=false".into(), Spec::Project { rebuild: false, clear_changes: false })
                        .await,
                ])
            }
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
            // Issue 278: count the ghost signature. NOT paired with a project — the
            // sweep it replaced marked notices unprojected and needed a fold behind
            // it to do the retiring, but a census writes nothing, so the pairing was
            // pure noise (verified on prod: the paired run reported "0 notices → 0
            // tenders"). `sweep-regrouped-ghosts` stays accepted as an alias, because
            // durable job rows carry the kind string and a recovered row from before
            // the rename must still resolve — and it now resolves to something safe.
            "ghost-census" | "sweep-regrouped-ghosts" => Ok(vec![
                self.push("ghost-census", "ghost-census".into(), Spec::GhostCensus).await,
            ]),
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
            // Issue 234's backfill half: collapse duplicate identifier-less
            // provisional Organizations. Deletes org rows and repoints references,
            // so the safe default applies: a missing flag means dry-run.
            "merge-provisional-orgs" => {
                let dry_run = req.dry_run.unwrap_or(true);
                let params = if dry_run { "dry-run".to_owned() } else { "execute".to_owned() };
                Ok(vec![
                    self.push(
                        "merge-provisional-orgs",
                        params,
                        Spec::MergeProvisionalOrgs { dry_run },
                    )
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

}

/// Job kinds whose loop actually READS the stop flag (issue 252).
///
/// `project` reads it at the projection's checkpoints — between Phase-1 plan chunks
/// and between Phase-2 fold batches (issue 256): the longest job the system runs was
/// the one kind that could not be cancelled, and the only exit from a grinding fold
/// was TENDER_DROP_JOBS plus a service restart, twice in one day.
///
/// The flag was built for `reparse` (issue 247) and for a while that was the only reader,
/// which made `cancel` on any other running job answer "asked it to stop" and then do
/// nothing — measured on prod: a cancelled `data-quality` run advanced ten more queries
/// over the following six minutes. Naming the readers here is what lets `cancel` refuse
/// instead of lie, and the next long job added is refused by default rather than
/// silently ignored.
/// Issue 318: what the genericness wall did this run, as the suffix that rides
/// the DURABLE job row. Extracted so its four cases are testable — this runtime's
/// stderr does not reach journald (issues 61/63), so the job row IS the surface,
/// and it printed a false alarm for as long as it existed (issue 338).
///
/// The counts ride here rather than a log line, and the suffix reports `enabled`
/// and `anchor_reached` alongside the asks, because otherwise a zero means two
/// things: the first full day's fold read "asked 0" over 3,889 notices and could
/// not say whether the anchor path never fired or the wall was switched off.
fn wall_suffix(w: &store::WallCounts) -> String {
    if w.errored > 0 {
        // An errored probe is never "fine": those binds went through at the
        // pre-318 bar.
        format!(
            "; issue-318 wall enabled={} reached {} asked {} refused {} \
             — {} PROBE(S) ERRORED, those binds took the pre-318 bar",
            w.enabled, w.anchor_reached, w.asked, w.denied, w.errored
        )
    } else if w.anchor_reached > 0 {
        format!(
            "; issue-318 wall enabled={} reached {} asked {} refused {}",
            w.enabled, w.anchor_reached, w.asked, w.denied
        )
    } else if w.resolved && !w.enabled {
        // A DISABLED wall on a quiet day would otherwise be invisible: nothing
        // reached the gate, so nothing is reported, so a prevention that is
        // switched off reads exactly like one with nothing to do. That is the
        // failure this whole instrument exists to avoid.
        //
        // `w.resolved` guards the INVERSE failure (issue 338): a fold with
        // nothing unprojected returns `Report::default()` without opening a
        // resolver, and a defaulted `enabled: false` read as a switched-off
        // wall — printing a cause ("a key build is in flight or was
        // interrupted") that was not true and binds ("took the pre-318 bar")
        // that never happened. An instrument that cries wolf on empty runs is
        // one an operator learns to skip.
        "; issue-318 wall DISABLED this run (key build in flight or \
         interrupted) — anchor binds took the pre-318 bar"
            .to_owned()
    } else {
        // Armed and nothing fired, or nothing ran at all. Genuinely nothing to
        // say either way.
        String::new()
    }
}

const STOPPABLE_KINDS: &[&str] = &[
    "reparse",
    "data-quality",
    "project",
    "merge-provisional-orgs",
    "org-merge-health",
    "r2-census",
    "r3-census",
    "match-org-identifiers",
    "build-org-match-keys",
    "scan-org-match-keys",
    "org-edge-census",
    "case-review-backlog",
    "fold-org-countries",
    "fusion-census",
    "rehoming-packet",
    "satellite-orphans",
    "drop-orphan-satellites",
    "anchor-wall-census",
    "xb-packet",
    "country-typo-census",
    "country-cluster-census",
    "duplicate-identity-census",
    "name-pollution-census",
    "generic-wall-census",
    "generic-statistic-census",
    "name-attribution-probe",
    // Issue 278: it reads the stop flag between slices, so this is a claim it
    // can keep. Its predecessor's whole incident was being un-stoppable —
    // one 40-minute statement with nothing to check a flag between.
    "ghost-census",
    "repair-country-typos",
    "repair-label-prefixes",
    "repair-minted-countries",
];

/// Issue 300 decision 5: a key shared by more organizations than this is a
/// generic name (the "Gymnázium" / "Centre hospitalier" class) — its O(n²)
/// pairs would be noise, so the scan counts and samples it instead of
/// emitting. Must stay far under [`store::SCAN_KEY_WINDOW`]: the walk's
/// fills-page guard stoplists on that ordering.
const SCAN_STOPLIST_CAP: usize = ingest::idgate::STOPLIST_CAP;

/// One page of the issue-332 key walk. Same shape and reasoning as the
/// anchor-wall census's window: big enough that a key run rarely spans two
/// pages, small enough that a page is tens of thousands of rows in RAM.
const GENERIC_KEY_WINDOW: usize = 20_000;

/// The Stage-0 exemplar sheet's contamination case: an org that carried a
/// foreign satellite name (the Dutch-MoD shape). Chased through
/// org_merge_log to its standing id and probed on EVERY scan run — the
/// acceptance gate wants it surfacing as an edge and nothing else.
const SCAN_EXEMPLAR_ORG: i64 = 23294544;

/// Tripwire 6 (issue 300 Stage 4 Unit 5): the monotone-growth check over
/// the edge store, evaluated at the end of every completed, uncapped wet
/// scan against the durable baseline. Edges are append-refresh-only in
/// v1, so a shrink is RED — and it is measured on `before_writes`, the
/// standing count BEFORE this run's upserts, because the run itself
/// re-covers any out-of-band deletion (the one legitimate wholesale reset
/// — the bare org rebuild — zeroes the baseline instead). Growth is
/// measured on the post-run total. NOTE the division of labor (panel
/// round 2): a census SPIKE from a new generic-name family or a broken
/// key fn is intercepted FIRST by the in-store T4 parity/bounds abort,
/// which lands on the alarm surface via [`Supervisor::edge_scan_refuse`];
/// the SPIKE arm here is the residual belt for growth that passes parity
/// (baseline corruption, anomalous capped interludes) — not the primary
/// spike detector.
fn edge_alarm(
    baseline: i64,
    before_writes: i64,
    total: i64,
    plan_expect: u64,
) -> Option<&'static str> {
    if before_writes < baseline {
        return Some("SHRUNK");
    }
    let growth = (total - baseline).max(0) as u64;
    if growth > store::EDGE_VOLUME_CEILING || growth > 2 * plan_expect.max(1) {
        return Some("SPIKE");
    }
    None
}

/// What [`Supervisor::cancel`] did.
#[derive(Debug, PartialEq, Eq)]
pub enum Cancelled {
    /// Dropped from the queue before it ever ran; the durable row is gone too.
    Queued,
    /// Running, and its kind checks the stop flag — it will end at its next checkpoint.
    Stopping,
    /// Running, but nothing in this kind's loop reads the flag, so the honest answer is
    /// no. Carries the kind so the caller can say which.
    Unstoppable(String),
    /// No such job.
    Unknown,
}

impl Supervisor {
    /// Remove a still-queued job, or ask the running one to stop if its kind can
    /// (issue 252 — and say so plainly when it cannot).
    /// Also drops the durable row so the cancellation survives a restart.
    pub async fn cancel(&self, id: u64) -> Cancelled {
        let removed = {
            let mut queue = self.queue.lock().expect("queue lock");
            let before = queue.len();
            queue.retain(|job| job.id != id);
            queue.len() != before
        };
        if removed {
            if let Err(e) = self.db.remove_job(id as i64).await {
                eprintln!("supervisor: remove cancelled job {id}: {e}");
            }
            return Cancelled::Queued;
        }
        // Not queued: it may be the one RUNNING. Flag it and let the job notice — a
        // cooperative stop, so the job ends its transaction, records a run log saying it
        // was cancelled, and drops its durable row like any concluded job. Killing it
        // mid-transaction would leave the row behind and re-run it on the next start.
        let Some(running) = self.current_progress().filter(|p| p.id == id) else {
            return Cancelled::Unknown;
        };
        if !STOPPABLE_KINDS.contains(&running.kind.as_str()) {
            eprintln!(
                "supervisor: job {id} is running as kind {} — nothing in its loop reads the \
                 stop flag, so it cannot be cancelled (issue 252)",
                running.kind
            );
            return Cancelled::Unstoppable(running.kind);
        }
        self.cancel_running.store(id, Ordering::Relaxed);
        eprintln!("supervisor: job {id} is running — asked it to stop at its next checkpoint");
        Cancelled::Stopping
    }

    /// Whether this running job has been asked to stop (issue 247). Checked at a job's
    /// own checkpoints — between packages, and between members inside one.
    fn cancelled(&self, id: u64) -> bool {
        self.cancel_running.load(Ordering::Relaxed) == id
    }

    /// A scan-org-match-keys refusal that CANNOT pass silently (issue 300
    /// Stage 4 Unit 5): the weekly wet scan IS tripwire 6's clock, and its
    /// three standing refusal paths (missing index after a bare rebuild,
    /// keys-epoch drift after a deploy, stale/out-of-bounds plan) would
    /// otherwise land only as a failed 03:xx Sunday job that nobody reads —
    /// the muted-probe failure. Every refusal writes the
    /// `org-edge-scan-alarm` report (an operator surface shows report
    /// stamps) and shouts to the journal; a completed wet run clears it.
    async fn edge_scan_refuse(&self, msg: String) -> Result<String, String> {
        eprintln!("[scan-org-match-keys] REFUSED: {msg}");
        let now = store::now_unix();
        let body = serde_json::json!({ "refused": msg, "at": now }).to_string();
        if let Err(e) = self.db.put_report("org-edge-scan-alarm", &body, now).await {
            eprintln!("[scan-org-match-keys] alarm report write failed: {e}");
        }
        Err(msg)
    }

    /// Tripwire 6's evaluate-and-anchor step (issue 300 Stage 4 Unit 5):
    /// the monotone check against the durable baseline, run ONLY for a
    /// completed, uncapped wet scan — capped and stopped runs return
    /// without touching the baseline or the alarm surface (extracted as a
    /// method precisely so a test can pin that guard). The verdict lands
    /// in the wet report's fields AND overwrites `org-edge-scan-alarm` —
    /// which a clean run clears, so the surface always shows the latest
    /// verdict, not the latest incident. Returns the job-message suffix.
    async fn apply_edge_tripwire(
        &self,
        r: &store::OrgEdgeScanReport,
        expect_edges: Option<u64>,
        wet: &mut serde_json::Value,
        now: i64,
    ) -> Result<String, String> {
        if r.capped || r.stopped {
            return Ok(String::new());
        }
        let baseline = self.db.org_edge_baseline().await.map_err(|e| e.to_string())?;
        let total = r.total_edges_after as i64;
        let alarm = edge_alarm(
            baseline,
            r.total_edges_before as i64,
            total,
            expect_edges.unwrap_or(r.would_emit),
        );
        wet["baseline_before"] = baseline.into();
        wet["alarm"] = match alarm {
            Some(a) => a.into(),
            None => serde_json::Value::Null,
        };
        let mut line = String::new();
        let alarm_body = match alarm {
            Some(a) => {
                eprintln!(
                    "[scan-org-match-keys] TRIPWIRE 6 {a}: edges {} before / {total} after \
                     vs baseline {baseline} (plan {expect_edges:?})",
                    r.total_edges_before
                );
                line = format!("; TRIPWIRE 6 {a} (baseline {baseline})");
                serde_json::json!({
                    "alarm": a, "baseline_before": baseline,
                    "edges_before_run": r.total_edges_before,
                    "total": total, "at": now,
                })
            }
            None => serde_json::json!({ "clear": true, "total": total, "at": now }),
        };
        self.db
            .put_report("org-edge-scan-alarm", &alarm_body.to_string(), now)
            .await
            .map_err(|e| e.to_string())?;
        self.db.set_org_edge_baseline(total).await.map_err(|e| e.to_string())?;
        Ok(line)
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
        // The pending queue is only half the floor. It is emptied as jobs finish,
        // so a restart that finds no work outstanding recovers max_id = 0 and the
        // counter stays at 1 — handing ids that already name finished runs in the
        // log to brand-new jobs. The log's own high-water mark closes that; rows
        // older than the `job_id` column answer NULL and contribute no floor,
        // which is correct (they were numbered under the old scheme anyway).
        match self.db.max_logged_job_id().await {
            Ok(Some(logged)) if logged > 0 => max_id = max_id.max(logged as u64),
            Ok(_) => {}
            Err(e) => eprintln!("supervisor: read job-id high-water mark: {e}"),
        }
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
            // Already queued is not the same as queued FIRST, and the difference is the
            // whole of issue 247: the reindex prod needed was in the queue for hours,
            // behind the job it would have made a hundred times faster. So move it, and
            // say whether the move was needed.
            let moved = {
                let mut queue = self.queue.lock().expect("queue lock");
                match queue.iter().position(|j| matches!(j.spec, Spec::Reindex)) {
                    Some(0) | None => false,
                    Some(at) => {
                        let job = queue.remove(at).expect("position just found");
                        queue.push_front(job);
                        true
                    }
                }
            };
            eprintln!(
                "supervisor: {} deferred index(es) missing ({}); a reindex was already queued{}",
                missing.len(),
                missing.join(", "),
                if moved { " — moved to the front" } else { " and is already first" }
            );
            self.wake.notify_one();
            return;
        }
        eprintln!(
            "supervisor: {} deferred index(es) missing ({}) — queueing a reindex AHEAD of \
             {} pending job(s)",
            missing.len(),
            missing.join(", "),
            self.queue.lock().expect("queue lock").len()
        );
        // Ahead of the queue, not behind it (issue 247). A missing index is not a
        // background chore when the queued work is what needs it: prod spent hours on a
        // re-parse costing 153 ms a notice while the Reindex that would have made it
        // microseconds waited its turn behind that very job.
        self.push_front("reindex", format!("auto: {}", missing.join(", ")), Spec::Reindex).await;
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

    /// Is a job of this kind already queued OR currently running? The scheduler's
    /// "never stack two" guards need this: `queued()` alone misses the running
    /// instance, because a popped job lives in `current`, not the queue (issue
    /// 282). Without the `current` leg a tick firing during the ~40-minute window
    /// a prior data-quality/rehash/reveal run is EXECUTING sees an empty queue and
    /// enqueues a duplicate — the exact double the guard exists to prevent.
    fn already_pending(&self, kind: &str) -> bool {
        self.current_progress().is_some_and(|p| p.kind == kind)
            || self.queued().iter().any(|j| j.kind == kind)
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
            // Its own phase name (issue 305): the identity scan used to borrow
            // "planning", so a long pass-1 plus the 58-v2 fallback's real plan
            // build read as one phase whose counter reset — a crash-restart
            // look-alike on a healthy run.
            Progress::Identity { notices, total } => {
                self.set_phase("identity", Some(notices), Some(total), "changed notices scanned".into());
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
            Progress::Applying { tenders, total, versions, leaf_rows } => {
                self.set_phase(
                    "folding",
                    Some(tenders),
                    Some(total),
                    format!("{versions} version rows written, {leaf_rows} leaf rows"),
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
            .is_some_and(|p| heavy_write_kind(p.kind.as_str()))
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
        self.ingestion_limited(RECENT_RUNS).await
    }

    /// [`Supervisor::ingestion`] with an operator-chosen job-log depth,
    /// clamped to `1..=JOB_LOG_MAX`. A caller asking for more than the cap
    /// gets the cap rather than an error — the point is that the depth is
    /// REACHABLE, not that the operator guessed the bound.
    pub async fn ingestion_limited(&self, limit: i64) -> turso::Result<Ingestion> {
        let limit = limit.clamp(1, JOB_LOG_MAX);
        // Snapshot the in-memory state into owned values FIRST: the std lock
        // guards must not be held across the await below, or the future stops
        // being `Send` and axum rejects the handler.
        let current = self.current.read().expect("progress lock").clone();
        let queued = self.queued();
        // `recent_job_runs` reads through the store's reader pool, not the writer
        // an ingestion job holds — so this never queues behind it (issue 20).
        let recent = self.db.recent_job_runs(limit).await?;
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
        // Clear any stop request with the job it named, so it cannot leak onto the next.
        let _ = self.cancel_running.compare_exchange(
            job.id,
            0,
            Ordering::Relaxed,
            Ordering::Relaxed,
        );
        self.jobs_completed.fetch_add(1, Ordering::Relaxed);

        let (outcome, counts) = match result {
            Ok(summary) => ("ok", summary),
            Err(e) => ("error", e),
        };
        if let Err(e) = self
            .db
            .record_job_run(
                job.id as i64,
                &job.kind,
                &job.params,
                started_at,
                store::now_unix(),
                outcome,
                &counts,
            )
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
            Spec::RehashProbe { samples } => {
                // Where the cursor stopped last run; a missing or garbled report
                // restarts the cycle from the oldest package, which only costs
                // re-probing rows that were probed before — idempotent by design.
                let after = match self.db.latest_report("rehash-cursor").await {
                    Ok(Some((body, _))) => serde_json::from_str::<serde_json::Value>(&body)
                        .ok()
                        .and_then(|v| v["after"].as_i64())
                        .unwrap_or(0),
                    _ => 0,
                };
                let mut page =
                    self.db.registry_page(after, *samples).await.map_err(|e| e.to_string())?;
                let mut wrapped = false;
                if page.len() < *samples {
                    // The cursor reached the registry's end: wrap to the oldest
                    // packages so the cycle never stalls at the tail.
                    wrapped = true;
                    let more = self
                        .db
                        .registry_page(0, *samples - page.len())
                        .await
                        .map_err(|e| e.to_string())?;
                    page.extend(more.into_iter().filter(|(id, ..)| *id <= after));
                }
                self.update(|p| p.packages_total = page.len() as u64);
                let (mut unchanged, mut drifted, mut gone, mut skipped) = (0, 0, 0, 0);
                let mut findings: Vec<serde_json::Value> = Vec::new();
                let mut cursor = after;
                for (id, source, kind, period) in &page {
                    self.update(|p| p.package = Some(format!("{source} {kind} {period}")));
                    let target = match build_target(&self.ted_base, &self.doe_base, source, kind, period)
                    {
                        Ok(t) => t,
                        Err(e) => {
                            // A registry row no target builder covers is a finding,
                            // not a crash — record it and keep cycling.
                            skipped += 1;
                            findings.push(serde_json::json!({
                                "package": format!("{source} {kind} {period}"),
                                "outcome": "unbuildable", "detail": e,
                            }));
                            cursor = cursor.max(*id);
                            self.update(|p| p.packages_done += 1);
                            continue;
                        }
                    };
                    match fetch::fetch(&self.db, &self.http, &self.archive, &target, true).await {
                        Ok(fetch::Outcome::Unchanged) => unchanged += 1,
                        Ok(fetch::Outcome::NewVersion) => {
                            // Upstream serves different bytes than we ingested. The
                            // fetch path has already archived the new version BESIDE
                            // the original — this is the drift D4 exists to see.
                            drifted += 1;
                            findings.push(serde_json::json!({
                                "package": format!("{source} {kind} {period}"),
                                "outcome": "drifted",
                            }));
                        }
                        Ok(fetch::Outcome::NotFound) => {
                            gone += 1;
                            findings.push(serde_json::json!({
                                "package": format!("{source} {kind} {period}"),
                                "outcome": "gone",
                            }));
                        }
                        Ok(other) => {
                            skipped += 1;
                            findings.push(serde_json::json!({
                                "package": format!("{source} {kind} {period}"),
                                "outcome": format!("{other:?}"),
                            }));
                        }
                        Err(e) => {
                            // One package's transient network failure must not void
                            // the rest of the sample; it is recorded, not retried.
                            skipped += 1;
                            findings.push(serde_json::json!({
                                "package": format!("{source} {kind} {period}"),
                                "outcome": "error", "detail": e.to_string(),
                            }));
                        }
                    }
                    cursor = cursor.max(*id);
                    self.update(|p| p.packages_done += 1);
                }
                let now = store::now_unix();
                let report = serde_json::json!({
                    "probed": page.len(), "unchanged": unchanged, "drifted": drifted,
                    "gone": gone, "skipped": skipped, "wrapped": wrapped,
                    "findings": findings,
                })
                .to_string();
                self.db.put_report("rehash-probe", &report, now).await.map_err(|e| e.to_string())?;
                // The cursor advances even when packages misbehaved: a drifted or
                // vanished package is REPORTED, and re-probing it every week would
                // stall the cycle on exactly the rows we already know about. Wrap
                // resets to the newest id consumed this run.
                let next = if wrapped { page.iter().map(|(id, ..)| *id).max().unwrap_or(0) } else { cursor };
                self.db
                    .put_report("rehash-cursor", &serde_json::json!({ "after": next }).to_string(), now)
                    .await
                    .map_err(|e| e.to_string())?;
                let alarm = if drifted + gone > 0 {
                    format!("; {} package(s) DRIFTED/GONE — read the report", drifted + gone)
                } else {
                    String::new()
                };
                Ok(format!(
                    "rehash probe: {} probed — {unchanged} unchanged, {drifted} drifted, \
                     {gone} gone, {skipped} skipped{}{alarm}",
                    page.len(),
                    if wrapped { " (registry cycle wrapped)" } else { "" },
                ))
            }
            Spec::RevealRecheck => {
                // One cursor slice per run (issue 274): the capped predecessor
                // bounded only the reveal-EXISTS pass while its population
                // aggregates joined the whole FieldsPrivacy cohort — 18+ min at
                // one saturated core on prod, uncancellable, ended by two
                // service restarts (2026-08-24). A slice is seconds, so the
                // job needs no stop-flag plumbing; consecutive nightly runs
                // walk the cohort and wrap, like D4's re-hash probe. A missing
                // or garbled cursor report restarts the walk from the oldest
                // notices — idempotent by design.
                let cursor_body = match self.db.latest_report("reveal-cursor").await {
                    Ok(Some((body, _))) => Some(body),
                    _ => None,
                };
                let after = cursor_body
                    .as_deref()
                    .and_then(|b| serde_json::from_str::<serde_json::Value>(b).ok())
                    .and_then(|v| v["after"].as_i64())
                    .unwrap_or(0);
                let now = store::now_unix();
                let sl = self
                    .db
                    .reveal_recheck(now, after, 100_000)
                    .await
                    .map_err(|e| e.to_string())?;
                let report = serde_json::json!({
                    "withheld_rows": sl.withheld_total,
                    "slice": {
                        "after": sl.after, "upto": sl.upto, "wrapped": sl.wrapped,
                        "sections": sl.sections, "with_reveal_date": sl.dated,
                        "due": sl.due, "checked": sl.checked,
                        "revealed_at_head": sl.revealed,
                        "still_withheld": sl.checked - sl.revealed,
                        "no_later_version": sl.no_later,
                        "later_still_withholds": sl.checked - sl.revealed - sl.no_later,
                        "due_by_field": sl.by_field
                            .iter()
                            .map(|(f, n)| serde_json::json!({ "field": f, "due": n }))
                            .collect::<Vec<_>>(),
                    },
                })
                .to_string();
                self.db
                    .put_report("reveal-recheck", &report, now)
                    .await
                    .map_err(|e| e.to_string())?;
                let (cursor_next, wrap_done) = roll_reveal_wrap(cursor_body.as_deref(), &sl, now);
                if let Some(wrap) = wrap_done {
                    self.db
                        .put_report("reveal-wrap", &wrap, now)
                        .await
                        .map_err(|e| e.to_string())?;
                }
                self.db
                    .put_report("reveal-cursor", &cursor_next, now)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(format!(
                    "reveal recheck: {} withheld field(s) in corpus; slice {}..{}: \
                     {} section(s), {} due, of {} checked {} revealed at head, \
                     {} awaiting a later version, {} BROKEN (later version still withholds){}",
                    sl.withheld_total,
                    sl.after,
                    sl.upto,
                    sl.sections,
                    sl.due,
                    sl.checked,
                    sl.revealed,
                    sl.no_later,
                    sl.checked - sl.revealed - sl.no_later,
                    if sl.wrapped { " (cohort cycle wrapped)" } else { "" },
                ))
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
            Spec::DataQuality { confirmed } => self.run_data_quality(job.id, *confirmed).await,
            Spec::Reparse { profiles, packages, after } => {
                self.run_reparse(job.id, profiles, *packages, *after, job.resume_after.as_deref())
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
                    // Observed (issue 65): the projection's Progress events feed
                    // the durable phase record, so /admin/jobs shows planning /
                    // pre-pass / folding instead of dead air for the multi-hour
                    // phases. The journal keeps its heartbeats — project_observed
                    // composes the stderr sink with this mapping, one stream.
                    project::project_observed_stoppable(
                        &self.db,
                        true,
                        |p| self.phase_from_progress(p),
                        &|| self.cancelled(job.id),
                    )
                    .await
                } else {
                    // The incremental path earns the same durable phase record as
                    // the full one (issue 262): a re-parse-scale delta spends tens
                    // of minutes in the plan build, and `phase: None` for all of it
                    // is how a cancel was watched grope for a checkpoint for 17
                    // minutes. A daily-scale delta flashes through `planning` in a
                    // heartbeat — harmless. Stop threads through per plan-build
                    // chunk now, not only between fold batches (issues 256 + 262).
                    project::project_incremental_observed_stoppable(
                        &self.db,
                        |p| self.phase_from_progress(p),
                        &|| self.cancelled(job.id),
                    )
                    .await
                }
                .map_err(|e| e.to_string())?;
                self.update(|p| p.notices = report.notices);
                // The written/unchanged split (issue 108) rides in the durable
                // counts line: a later G2 breach can then be read against what
                // each fold actually did — "written 0 / unchanged N" points at
                // the watermark over-claiming, a large `written` at the fold.
                // A cancelled run's log row must SAY so (issue 256): its tallies
                // are real, committed work — but a summary that looks complete is
                // how a stopped fold gets mistaken for a finished one (the same
                // rule the capped reparse follows, issue 244).
                let cancelled = if report.stopped { "CANCELLED at a checkpoint — " } else { "" };
                let wall = wall_suffix(&report.wall);
                Ok(format!(
                    "{cancelled}{} notices → {} tenders ({} islands), {} versions; {} tenders written, {} verified unchanged{wall}",
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
            Spec::BackfillTitles => {
                // The `BackfillDeadlines` walk exactly: bounded batch per transaction,
                // WAL checkpoint between batches (issue 42), progress as members_done.
                let mut stamped = 0i64;
                let mut watermark = 0i64;
                loop {
                    let (rows, next) = self
                        .db
                        .backfill_current_title(BACKFILL_BATCH, watermark)
                        .await
                        .map_err(|e| e.to_string())?;
                    if rows == 0 {
                        break;
                    }
                    stamped += rows;
                    watermark = next;
                    self.update(|p| p.members_done = stamped as u64);
                    if let Err(e) = self.db.checkpoint(store::CheckpointMode::Truncate).await {
                        eprintln!("supervisor: checkpoint after title batch: {e}");
                    }
                }
                Ok(format!(
                    "current_title stamped over {stamped} tenders (head-version title, \
                     Tender's own before a lot's; NULL where none is published)"
                ))
            }
            Spec::BackfillOriginalLang => {
                // The `BackfillValues` walk exactly: bounded batch per transaction,
                // WAL checkpoint between batches (issue 42), progress as
                // members_done. The 639-2/T map is the fold's own, injected as a
                // `fn` across the crate seam (`store` cannot depend on `ingest`).
                fn normalize(code: &str) -> Option<String> {
                    ingest::project::normalize_lang(Some(code))
                }
                let mut stamped = 0i64;
                let mut watermark = 0i64;
                loop {
                    let (rows, next) = self
                        .db
                        .backfill_original_lang(BACKFILL_BATCH, watermark, normalize)
                        .await
                        .map_err(|e| e.to_string())?;
                    if rows == 0 {
                        break;
                    }
                    stamped += rows;
                    watermark = next;
                    self.update(|p| p.members_done = stamped as u64);
                    if let Err(e) = self.db.checkpoint(store::CheckpointMode::Truncate).await {
                        eprintln!("supervisor: checkpoint after original_lang batch: {e}");
                    }
                }
                Ok(format!(
                    "original_lang backfill walked {stamped} tenders, every NULL version of \
                     theirs stamped from its notice's own language code (BT-702 / LG_ORIG / \
                     OL, in the fold's 639-2/T vocabulary — ADR-0013 D3's third leg). Left \
                     NULL where the era never said: the 1990s text notices before the OL line."
                ))
            }
            Spec::BackfillValues => {
                // The `BackfillDeadlines` walk exactly: bounded batch per transaction,
                // WAL checkpoint between batches (issue 42), progress as members_done.
                let mut stamped = 0i64;
                let mut watermark = 0i64;
                loop {
                    let (rows, next) = self
                        .db
                        .backfill_current_value_eur(BACKFILL_BATCH, watermark)
                        .await
                        .map_err(|e| e.to_string())?;
                    if rows == 0 {
                        break;
                    }
                    stamped += rows;
                    watermark = next;
                    self.update(|p| p.members_done = stamped as u64);
                    if let Err(e) = self.db.checkpoint(store::CheckpointMode::Truncate).await {
                        eprintln!("supervisor: checkpoint after value batch: {e}");
                    }
                }
                Ok(format!(
                    "current_value_eur_cents stamped over {stamped} tenders (head-version \
                     MAX derived-EUR amount; NULL where none converts — ADR-0014 D4)"
                ))
            }
            Spec::RederiveEur => {
                // Reload FIRST: the walk must see the rates table as repaired,
                // not whatever snapshot the last projection cached.
                let cached = self.db.reload_rates_lookup().await.map_err(|e| e.to_string())?;
                let rates = self.db.rates_lookup();
                let mut tenders = 0i64;
                let mut scanned = 0i64;
                let mut updated = 0i64;
                // Issue 306 resumability: a restarted process re-runs this
                // persisted job — pick the walk up at the last completed
                // window instead of redoing hours. Idempotent writes make the
                // at-most-one-window overlap harmless.
                let mut watermark = self.db.rederive_watermark().await.map_err(|e| e.to_string())?;
                let resumed = watermark;
                if resumed > 0 {
                    eprintln!("[rederive-eur] resuming past tender id {resumed}");
                }
                loop {
                    let (t, rows, changed, next) = self
                        .db
                        .rederive_eur_window(&rates, BACKFILL_BATCH, watermark)
                        .await
                        .map_err(|e| e.to_string())?;
                    if t == 0 {
                        break;
                    }
                    tenders += t;
                    scanned += rows;
                    updated += changed;
                    watermark = next;
                    if let Err(e) = self.db.set_rederive_watermark(watermark).await {
                        eprintln!("supervisor: rederive watermark write: {e}");
                    }
                    self.set_phase(
                        "walking",
                        Some(tenders as u64),
                        None,
                        format!(
                            "{scanned} money rows scanned, {updated} updated, at tender {watermark}"
                        ),
                    );
                    if let Err(e) = self.db.checkpoint(store::CheckpointMode::Truncate).await {
                        eprintln!("supervisor: checkpoint after rederive window: {e}");
                    }
                }
                self.db.set_rederive_watermark(0).await.map_err(|e| e.to_string())?;
                Ok(format!(
                    "eur_cents re-derived from {cached} cached rates over {tenders} tenders{}: \
                     {updated} of {scanned} money rows changed — follow with backfill-values \
                     (issue 306)",
                    if resumed > 0 {
                        format!(" (resumed past tender id {resumed})")
                    } else {
                        String::new()
                    }
                ))
            }
            Spec::BackfillOrgNameVariants => {
                let mut totals = store::OrgNameBackfill::default();
                let mut watermark = 0i64;
                loop {
                    let (batch, next) = self
                        .db
                        .backfill_org_name_variants_batch(
                            ingest::project::ORG_NAME_FIELD_IDS,
                            ingest::project::normalize_lang,
                            BACKFILL_BATCH,
                            watermark,
                        )
                        .await
                        .map_err(|e| e.to_string())?;
                    if batch.notices == 0 {
                        break;
                    }
                    totals.notices += batch.notices;
                    totals.mentions += batch.mentions;
                    totals.written += batch.written;
                    watermark = next;
                    self.set_phase(
                        "walking",
                        Some(totals.notices),
                        None,
                        format!("{} mentions, {} variants written", totals.mentions, totals.written),
                    );
                    if let Err(e) = self.db.checkpoint(store::CheckpointMode::Truncate).await {
                        eprintln!("supervisor: checkpoint after org-name window: {e}");
                    }
                }
                Ok(format!(
                    "organization_names backfilled over {} notices: {} mentions visited, \
                     {} labelled variants written (issue 307)",
                    totals.notices, totals.mentions, totals.written
                ))
            }
            Spec::RepairNestedOrgs { dry_run } => {
                let dry_run = *dry_run;
                let mut totals = store::NestedOrgRepair::default();
                let mut watermark = 0i64;
                loop {
                    let (batch, next) = self
                        .db
                        .repair_nested_org_mentions_batch(BACKFILL_BATCH, watermark, dry_run)
                        .await
                        .map_err(|e| e.to_string())?;
                    if batch.scanned == 0 {
                        break;
                    }
                    totals.scanned += batch.scanned;
                    totals.repaired += batch.repaired;
                    totals.skipped += batch.skipped;
                    totals.winner_dups += batch.winner_dups;
                    totals.tender_changes += batch.tender_changes;
                    watermark = next;
                    self.set_phase(
                        if dry_run { "previewing" } else { "repairing" },
                        Some(totals.scanned),
                        None,
                        format!("{} repaired, {} skipped", totals.repaired, totals.skipped),
                    );
                    if !dry_run {
                        if let Err(e) = self.db.checkpoint(store::CheckpointMode::Truncate).await {
                            eprintln!("supervisor: checkpoint after nested-org batch: {e}");
                        }
                    }
                }
                Ok(format!(
                    "nested-org mention repair (issue 259){}: {} nameless provisionals scanned, \
                     {} repaired onto their named inner org (empty row deleted), {} skipped by \
                     guards, {} duplicate winner rows removed, {} tenders touched",
                    if dry_run { " DRY RUN — nothing written" } else { "" },
                    totals.scanned,
                    totals.repaired,
                    totals.skipped,
                    totals.winner_dups,
                    totals.tender_changes
                ))
            }
            Spec::RepairPlaceholderOrgs { dry_run } => {
                let dry_run = *dry_run;
                let mut totals = store::OrgDissolve::default();
                let mut watermark = 0i64;
                loop {
                    let (batch, next) = self
                        .db
                        .repair_placeholder_orgs_batch(
                            ingest::idgate::condemns,
                            BACKFILL_BATCH,
                            watermark,
                            dry_run,
                        )
                        .await
                        .map_err(|e| e.to_string())?;
                    if batch.scanned == 0 {
                        break;
                    }
                    totals.scanned += batch.scanned;
                    totals.condemned += batch.condemned;
                    totals.dissolved += batch.dissolved;
                    totals.skipped += batch.skipped;
                    totals.mentions += batch.mentions;
                    totals.fresh += batch.fresh;
                    totals.reused += batch.reused;
                    totals.parties += batch.parties;
                    totals.bid_parties += batch.bid_parties;
                    totals.winners += batch.winners;
                    totals.winner_dups += batch.winner_dups;
                    totals.winners_deleted += batch.winners_deleted;
                    totals.refold_tenders += batch.refold_tenders;
                    totals.refold_notices += batch.refold_notices;
                    totals.tender_changes += batch.tender_changes;
                    watermark = next;
                    self.set_phase(
                        if dry_run { "previewing" } else { "dissolving" },
                        Some(totals.scanned),
                        None,
                        format!(
                            "{} condemned, {} dissolved, {} skipped, {} queued for refold",
                            totals.condemned,
                            totals.dissolved,
                            totals.skipped,
                            totals.refold_tenders
                        ),
                    );
                    if !dry_run {
                        if let Err(e) = self.db.checkpoint(store::CheckpointMode::Truncate).await {
                            eprintln!("supervisor: checkpoint after dissolve batch: {e}");
                        }
                    }
                }
                Ok(format!(
                    "placeholder dissolve (issue 300){}: {} identifier-bearing orgs scanned, \
                     {} condemned by the v2 gate, {} dissolved, {} skipped (unresolvable winner), \
                     {} mentions re-resolved ({} fresh provisionals, {} reused), \
                     {} party rows, {} bid-party rows, {} winner rows ({} duplicates removed), \
                     {} ambiguous winner rows deleted for refold ({} tenders stamped epoch-stale, \
                     {} notices re-queued — the next incremental fold re-derives their winner \
                     sets), {} tenders touched",
                    if dry_run { " DRY RUN — nothing written" } else { "" },
                    totals.scanned,
                    totals.condemned,
                    totals.dissolved,
                    totals.skipped,
                    totals.mentions,
                    totals.fresh,
                    totals.reused,
                    totals.parties,
                    totals.bid_parties,
                    totals.winners,
                    totals.winner_dups,
                    totals.winners_deleted,
                    totals.refold_tenders,
                    totals.refold_notices,
                    totals.tender_changes
                ))
            }
            Spec::OrgMergeHealth => {
                // Issue 300 Stage 0. Read-only: no transactions, no events, no
                // checkpoints — the walk is ~58 batches over the 1.16M
                // identifier-bearing orgs and their mentions. The report is
                // the baseline the matcher stages gate on and the standing
                // distinct-name tripwire input; a run is cheap enough to ride
                // any cadence later. Stop flag honoured between batches
                // (issue 252's bar) — a stopped run stores nothing, because a
                // partial census would undercount the tail and a later reader
                // would trust it (the 230 zero-lie class).
                let mut watermark = 0i64;
                let mut orgs = 0u64;
                let (mut ge2, mut ge6, mut ge20) = (0u64, 0u64, 0u64);
                let mut max: (u64, i64) = (0, 0);
                // Top-100 by distinct-name count: a min-heap of (count, org).
                let mut top: std::collections::BinaryHeap<std::cmp::Reverse<(u64, i64)>> =
                    std::collections::BinaryHeap::with_capacity(101);
                // The Stage-1 gate census, riding the same walk: per-scheme
                // populations + checksum pass/fail (the §2.1 enablement
                // input — a scheme may hard-reject only at a measured ≥97%
                // pass rate, which also adjudicates the validator
                // implementations themselves) and the placeholder-class
                // counters.
                #[derive(Default)]
                struct SchemeTally {
                    pop: u64,
                    pass: u64,
                    fail: u64,
                }
                let mut schemes: std::collections::HashMap<&'static str, SchemeTally> =
                    std::collections::HashMap::new();
                let (mut lexicon, mut sequence, mut letter_run, mut short_vat) =
                    (0u64, 0u64, 0u64, 0u64);
                let (mut hex_hash, mut compound) = (0u64, 0u64);
                // Issue 325 step 5: the parser-vs-stock tripwire.
                //
                // Not a SQL predicate. The class this watches is defined by
                // `normalise_identifier`, so the gauge CALLS it and compares
                // against what the row stores — the same reason the repair job
                // injects the classifier instead of restating the rule. A
                // predicate here would be a second spelling, and the quieter
                // spelling is the one that goes wrong (issues 318, 323, 326).
                //
                // THREE counters, because this arm has now been wrong in both
                // directions in one day: it minted countries out of words
                // (`CHARITYNO298028` filed as Swiss), and then the tightening
                // rejected 211 real VAT ids that carry a scheme label (`MVA`,
                // `MWST`, `USTID`). One number could not tell those apart.
                let (mut no_longer_vat, mut vat_country_differs, mut vat_refused) = (0u64, 0u64, 0u64);
                // Issue 327: the Austrian 9110 GLN class, watched rather than
                // trusted.
                //
                // 6,965 rows over 6,915 distinct values, so it is not a
                // false-merge source in aggregate — but of the ~50 values held
                // by MORE THAN ONE row, 31 of 46 sampled pair an Austrian public
                // body with an unrelated foreign supplier, because the publisher
                // puts the buyer's GLN in the supplier's organization block.
                //
                // No merge has occurred: those rows sit under different
                // countries, and that country difference is the only thing
                // keeping them apart. The damage is LATENT, so what this watches
                // is whether it stops being latent.
                let mut gln_rows = 0u64;
                let mut gln_by_value: std::collections::HashMap<String, Vec<String>> =
                    Default::default();
                loop {
                    if self.cancelled(job.id) {
                        return Ok(
                            "org-merge-health stopped by cancel — no report stored".to_owned()
                        );
                    }
                    let (rows, next) = self
                        .db
                        .org_merge_health_batch(ingest::project::match_norm, BACKFILL_BATCH, watermark)
                        .await
                        .map_err(|e| e.to_string())?;
                    if rows.is_empty() {
                        break;
                    }
                    for r in &rows {
                        let n = r.distinct_names;
                        orgs += 1;
                        if n >= 2 { ge2 += 1; }
                        if n >= 6 { ge6 += 1; }
                        if n >= 20 { ge20 += 1; }
                        if n > max.0 { max = (n, r.org_id); }
                        top.push(std::cmp::Reverse((n, r.org_id)));
                        if top.len() > 100 {
                            top.pop();
                        }
                        let c = ingest::idgate::census(
                            r.country.as_deref(),
                            r.kind.as_deref(),
                            &r.identifier,
                        );
                        let t = schemes.entry(c.scheme).or_default();
                        t.pop += 1;
                        // Lexicon/sequence-condemned ids are removed by the
                        // EARLIER gate stage regardless of checksum, so they
                        // must not depress the enablement rate the ≥97% bar
                        // reads (the run-1331 refinement: PL:nip and
                        // FR:siret sat just under the bar with junk
                        // included in the denominator).
                        if !c.lexicon && !c.sequence {
                            match c.checksum {
                                ingest::idgate::Checksum::Pass => t.pass += 1,
                                ingest::idgate::Checksum::Fail => t.fail += 1,
                                ingest::idgate::Checksum::Unknown => {}
                            }
                        }
                        if c.lexicon { lexicon += 1; }
                        if c.sequence { sequence += 1; }
                        if c.letter_run { letter_run += 1; }
                        if c.short_vat { short_vat += 1; }
                        if c.hex_hash { hex_hash += 1; }
                        if c.compound { compound += 1; }
                        // Issue 325 step 5. Free: the walk already holds
                        // everything the parser needs. Passing the row's OWN
                        // country is deliberate — the VAT arm ignores it when
                        // it mints from the prefix, so what comes back is the
                        // arm's verdict rather than an echo of the stock.
                        if r.kind.as_deref() == Some("vat") {
                            match ingest::project::normalise_identifier(
                                &r.identifier,
                                r.country.as_deref(),
                            ) {
                                None => vat_refused += 1,
                                Some(id) if id.kind != "vat" => no_longer_vat += 1,
                                Some(id) if id.country.as_deref() != r.country.as_deref() => {
                                    vat_country_differs += 1
                                }
                                Some(_) => {}
                            }
                        }
                        if r.identifier.len() == 13
                            && r.identifier.starts_with("9110")
                            && r.identifier.bytes().all(|b| b.is_ascii_digit())
                        {
                            gln_rows += 1;
                            let cc = r.country.clone().unwrap_or_default();
                            gln_by_value.entry(r.identifier.clone()).or_default().push(cc);
                        }
                    }
                    watermark = next;
                    self.set_phase(
                        "censusing",
                        Some(orgs),
                        None,
                        format!("{ge2} orgs >=2 names, {ge6} >=6, max {}", max.0),
                    );
                }
                let ranked: Vec<(u64, i64)> = {
                    let mut v: Vec<_> = top.into_iter().map(|r| r.0).collect();
                    v.sort_unstable_by(|a, b| b.cmp(a));
                    v
                };
                let ids: Vec<i64> = ranked.iter().map(|&(_, id)| id).collect();
                let meta = self.db.org_health_meta(&ids).await.map_err(|e| e.to_string())?;
                let by_id: std::collections::HashMap<i64, _> =
                    meta.into_iter().map(|m| (m.0, m)).collect();
                let now = store::now_unix();
                // Issue 325 step 5: the tripwire's baseline is the PREVIOUS run
                // of this same report, read before this one overwrites it. No
                // new schema and no hardcoded floor to go stale — the floor is
                // whatever the corpus last measured, which is the only number
                // that stays true as the residue is worked down.
                let previous = match self.db.latest_report("org-merge-health").await {
                    Ok(Some((body, _))) => serde_json::from_str::<serde_json::Value>(&body).ok(),
                    // A missing or unreadable baseline is not a reason to fail
                    // the census. The first run has nothing to compare against
                    // and says so rather than alarming on its own arrival.
                    _ => None,
                };
                // Issue 327's two numbers. `shared` is the size of the class
                // that is wrong two thirds of the time; `shared_one_country` is
                // the one that must stay ZERO, because a shared GLN whose rows
                // have collapsed onto a single country is a merge path that has
                // OPENED — R2 keys on (country, kind, identifier), so at that
                // point nothing stands between the Austrian ministry and the
                // Norwegian aviation firm.
                let mut gln_shared = 0u64;
                let mut gln_shared_one_country = 0u64;
                for codes in gln_by_value.values() {
                    if codes.len() < 2 {
                        continue;
                    }
                    gln_shared += 1;
                    let first = &codes[0];
                    if codes.iter().all(|c| c == first) {
                        gln_shared_one_country += 1;
                    }
                }
                let alarms = parser_vs_stock_alarms(
                    previous.as_ref().map(|v| &v["parser_vs_stock"]),
                    no_longer_vat,
                    vat_country_differs,
                    vat_refused,
                    gln_shared_one_country,
                );
                let mut scheme_rows: Vec<(&str, SchemeTally)> = schemes.into_iter().collect();
                scheme_rows.sort_by(|a, b| b.1.pop.cmp(&a.1.pop));
                let placeholder_total = lexicon.max(sequence);
                let report = serde_json::json!({
                    "identifier_bearing": orgs,
                    "ge2": ge2, "ge6": ge6, "ge20": ge20,
                    "max": max.0, "max_org": max.1,
                    "gate": {
                        "lexicon": lexicon, "sequence": sequence,
                        "letter_run": letter_run, "short_vat": short_vat,
                        "hex_hash": hex_hash, "compound": compound,
                        "schemes": scheme_rows.iter().map(|(k, t)| serde_json::json!({
                            "scheme": k, "pop": t.pop, "pass": t.pass, "fail": t.fail,
                        })).collect::<Vec<_>>(),
                    },
                    // Issue 325 step 5: rows the identifier parser no longer
                    // agrees with. Each is a DELIBERATE residue at the floor
                    // below, so a jump means either the parser moved or new
                    // contaminated stock arrived — and which counter jumps says
                    // which direction the arm went wrong in.
                    //
                    // The floors are MEASURED (prod, job 540, right after the
                    // step-4 repair) and much lower than the estimate written
                    // when this shipped. That estimate assumed the repair's 408
                    // "ambiguous" rows would each show up here; they do not,
                    // because the repair skips an ambiguous row BEFORE it
                    // reclassifies, so most were never rows the parser
                    // disagreed with at all. The true residue is eight rows.
                    "parser_vs_stock": {
                        // Stands as `vat`; the parser now calls it something
                        // else. Floor 7.
                        "no_longer_vat": no_longer_vat,
                        // Stands as `vat` and still is, under a DIFFERENT
                        // country code — the `EL`/`UK`/`XI` re-contamination
                        // channel issue 319's fold could not see. Floor 1.
                        "vat_country_differs": vat_country_differs,
                        // The v2 gate now refuses the value outright. Counted,
                        // never acted on: stripping a published identifier is
                        // what issue 312 had to undo. Floor 0.
                        "vat_refused": vat_refused,
                        // TRIMMED, not the previous block whole. Storing
                        // `previous["parser_vs_stock"]` verbatim embedded that
                        // block's OWN baseline, so every run nested one level
                        // deeper — the live report had reached three levels,
                        // each carrying a full per-scheme table, growing without
                        // bound and multiplied by the ten versions report history
                        // now keeps. Nothing read it either:
                        // `parser_vs_stock_alarms` compares against the previous
                        // block's TOP-LEVEL counters, never its baseline.
                        "baseline": trimmed_baseline(previous.as_ref()),
                        "alarms": alarms.clone(),
                    },
                    // Issue 327: the shared-GLN class, watched not trusted.
                    "gln_9110": {
                        "rows": gln_rows,
                        "distinct": gln_by_value.len(),
                        // ~50 today, of which about two thirds pair unrelated
                        // entities. Growth means the publisher-side error is
                        // spreading.
                        "shared": gln_shared,
                        // MUST STAY ZERO. A shared GLN under one country is a
                        // merge path that has opened.
                        "shared_one_country": gln_shared_one_country,
                    },
                    "top": ranked.iter().map(|&(n, id)| {
                        let m = by_id.get(&id);
                        serde_json::json!({
                            "org_id": id, "distinct_names": n,
                            "country": m.and_then(|m| m.1.clone()),
                            "identifier_kind": m.and_then(|m| m.2.clone()),
                            "identifier": m.and_then(|m| m.3.clone()),
                            "name": m.map(|m| m.4.clone()),
                        })
                    }).collect::<Vec<_>>(),
                })
                .to_string();
                self.db
                    .put_report("org-merge-health", &report, now)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(format!(
                    "org-merge-health census (issue 300): {orgs} identifier-bearing orgs, \
                     {ge2} with >=2 distinct mention names, {ge6} >=6, {ge20} >=20, \
                     max {} (org {}); gate census: {lexicon} lexicon, {sequence} sequence, \
                     {letter_run} letter-run, {short_vat} short-vat, {hex_hash} hex-hash, \
                     {compound} compound hits (~{placeholder_total}+ placeholder-keyed; \
                     checksum rates now exclude condemned ids). Parser-vs-stock \
                     (issue 325 step 5): {no_longer_vat} no longer vat, \
                     {vat_country_differs} vat under another country, \
                     {vat_refused} now refused. Issue 327: {} Austrian 9110 \
                     GLN row(s), {} value(s) held by more than one row (that \
                     class is wrong about two thirds of the time), {} of them \
                     under a SINGLE country{}",
                    max.0,
                    max.1,
                    gln_rows,
                    gln_shared,
                    gln_shared_one_country,
                    if alarms.is_empty() {
                        if previous.is_some() {
                            String::new()
                        } else {
                            " — first run, no baseline to compare against".to_owned()
                        }
                    } else {
                        format!("; PARSER-VS-STOCK ALARM(S): {}", alarms.join(", "))
                    }
                ))
            }
            Spec::R2Census => {
                // Issue 300 Stage 2, the opening census. Read-only preview of
                // the R2 same-country merge: canonical keys over the whole
                // identifier-bearing org layer, grouped in RAM (the B-ID
                // block, §4.1) — measured BEFORE any merge code exists, the
                // same discipline that sized Stage 1. Rides the org-merge-
                // health walk; a stopped run stores nothing (issue 230's
                // zero-lie bar).
                use std::collections::HashMap;
                #[derive(Default)]
                struct Group {
                    // E1 members as (org_id, is_vat_kind).
                    e1: Vec<(i64, bool)>,
                    // Pad-derived attachments (E2): counted, never merged.
                    e2: u64,
                }
                let mut watermark = 0i64;
                let mut scanned = 0u64;
                let (mut keyed_e1, mut keyed_e2, mut unkeyed) = (0u64, 0u64, 0u64);
                let (mut es_ute, mut cz699) = (0u64, 0u64);
                let (mut null_country_keyed, mut prefix_contradictions) = (0u64, 0u64);
                let mut groups: HashMap<(String, &'static str, String), Group> = HashMap::new();
                loop {
                    if self.cancelled(job.id) {
                        return Ok("r2-census stopped by cancel — no report stored".to_owned());
                    }
                    let (rows, next) = self
                        .db
                        .org_merge_health_batch(ingest::project::match_norm, BACKFILL_BATCH, watermark)
                        .await
                        .map_err(|e| e.to_string())?;
                    if rows.is_empty() {
                        break;
                    }
                    for r in &rows {
                        scanned += 1;
                        let kind = r.kind.clone().unwrap_or_else(|| "national".into());
                        let is_vat = kind == "vat";
                        // Denial-class counters (the key fn returns None for
                        // these; the census names them so the report shows
                        // the walls working, not silent gaps).
                        let norm: String = r
                            .identifier
                            .chars()
                            .filter(char::is_ascii_alphanumeric)
                            .map(|c| c.to_ascii_uppercase())
                            .collect();
                        if is_vat && norm.starts_with("CZ699") {
                            cz699 += 1;
                        }
                        let es_body = if is_vat {
                            norm.strip_prefix("ES")
                        } else if r.country.as_deref() == Some("ES") {
                            Some(norm.as_str())
                        } else {
                            None
                        };
                        if es_body.is_some_and(|b| b.starts_with('U')) {
                            es_ute += 1;
                        }
                        let Some(k) = ingest::crosswalk::canonical_key(
                            r.country.as_deref(),
                            &kind,
                            &r.identifier,
                        ) else {
                            unkeyed += 1;
                            continue;
                        };
                        // R2 is SAME-COUNTRY: a NULL-country row's key is
                        // Stage-3 (R3) material — counted, not grouped. A row
                        // whose country contradicts its VAT prefix is an
                        // anomaly — counted, not grouped (GR/EL fold applied).
                        let Some(country) = r.country.as_deref() else {
                            null_country_keyed += 1;
                            continue;
                        };
                        let country = if country == "EL" { "GR" } else { country };
                        if is_vat && !k.scheme.starts_with(country) {
                            prefix_contradictions += 1;
                            continue;
                        }
                        let g = groups
                            .entry((country.to_owned(), k.scheme, k.key))
                            .or_default();
                        match k.tier {
                            ingest::crosswalk::Tier::E1 => {
                                keyed_e1 += 1;
                                g.e1.push((r.org_id, is_vat));
                            }
                            ingest::crosswalk::Tier::E2 => {
                                keyed_e2 += 1;
                                g.e2 += 1;
                            }
                        }
                    }
                    watermark = next;
                    self.set_phase(
                        "censusing",
                        Some(scanned),
                        None,
                        format!("{keyed_e1} E1-keyed, {} key groups", groups.len()),
                    );
                }
                // Per-scheme tallies over the E1 groups.
                #[derive(Default)]
                struct SchemeStat {
                    groups_ge2: u64,
                    orgs_in_groups: u64,
                    mixed_kind: u64,
                    over_cap: u64,
                    max_group: u64,
                }
                const GROUP_CAP: usize = 8;
                let mut per_scheme: HashMap<&'static str, SchemeStat> = HashMap::new();
                let (mut e2_attached, mut e2_orphan_keys) = (0u64, 0u64);
                let mut sample: Vec<(&(String, &'static str, String), usize)> = Vec::new();
                for (key, g) in &groups {
                    if g.e2 > 0 {
                        if g.e1.is_empty() {
                            e2_orphan_keys += 1;
                        } else {
                            e2_attached += g.e2;
                        }
                    }
                    if g.e1.len() < 2 {
                        continue;
                    }
                    let s = per_scheme.entry(key.1).or_default();
                    s.groups_ge2 += 1;
                    s.orgs_in_groups += g.e1.len() as u64;
                    s.max_group = s.max_group.max(g.e1.len() as u64);
                    if g.e1.iter().any(|m| m.1) && g.e1.iter().any(|m| !m.1) {
                        s.mixed_kind += 1;
                    }
                    if g.e1.len() > GROUP_CAP {
                        s.over_cap += 1;
                    }
                    sample.push((key, g.e1.len()));
                }
                // The largest 30 groups become the report's inspection sample
                // (the precision review's raw material).
                sample.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
                sample.truncate(30);
                let sample_ids: Vec<i64> = sample
                    .iter()
                    .flat_map(|(k, _)| groups[*k].e1.iter().map(|m| m.0))
                    .collect();
                let meta =
                    self.db.org_health_meta(&sample_ids).await.map_err(|e| e.to_string())?;
                let by_id: HashMap<i64, _> = meta.into_iter().map(|m| (m.0, m)).collect();
                let mut scheme_rows: Vec<(&str, SchemeStat)> = per_scheme.into_iter().collect();
                scheme_rows.sort_by(|a, b| b.1.groups_ge2.cmp(&a.1.groups_ge2));
                let (total_groups, total_orgs, total_mixed, total_over_cap) =
                    scheme_rows.iter().fold((0u64, 0u64, 0u64, 0u64), |acc, (_, s)| {
                        (
                            acc.0 + s.groups_ge2,
                            acc.1 + s.orgs_in_groups,
                            acc.2 + s.mixed_kind,
                            acc.3 + s.over_cap,
                        )
                    });
                let now = store::now_unix();
                let report = serde_json::json!({
                    "scanned": scanned,
                    "keyed_e1": keyed_e1, "keyed_e2": keyed_e2, "unkeyed": unkeyed,
                    "groups_ge2": total_groups, "orgs_in_groups": total_orgs,
                    "mixed_kind_groups": total_mixed, "over_cap_groups": total_over_cap,
                    "e2_attached": e2_attached, "e2_orphan_keys": e2_orphan_keys,
                    "null_country_keyed": null_country_keyed,
                    "prefix_contradictions": prefix_contradictions,
                    "denials": { "es_ute": es_ute, "cz699": cz699 },
                    "group_cap": GROUP_CAP,
                    "schemes": scheme_rows.iter().map(|(k, s)| serde_json::json!({
                        "scheme": k, "groups_ge2": s.groups_ge2,
                        "orgs_in_groups": s.orgs_in_groups, "mixed_kind": s.mixed_kind,
                        "over_cap": s.over_cap, "max_group": s.max_group,
                    })).collect::<Vec<_>>(),
                    "sample": sample.iter().map(|(key, n)| serde_json::json!({
                        "country": key.0, "scheme": key.1, "key": key.2, "size": n,
                        "members": groups[*key].e1.iter().map(|(id, is_vat)| {
                            let m = by_id.get(id);
                            serde_json::json!({
                                "org_id": id, "vat_kind": is_vat,
                                "identifier": m.and_then(|m| m.3.clone()),
                                "name": m.map(|m| m.4.clone()),
                            })
                        }).collect::<Vec<_>>(),
                    })).collect::<Vec<_>>(),
                })
                .to_string();
                self.db.put_report("r2-census", &report, now).await.map_err(|e| e.to_string())?;
                Ok(format!(
                    "r2-census (issue 300 Stage 2): {scanned} identifier-bearing orgs, \
                     {keyed_e1} E1-keyed / {keyed_e2} E2 (pad) / {unkeyed} no key; \
                     {total_groups} same-country groups >=2 holding {total_orgs} orgs \
                     ({total_mixed} mixed vat+national, {total_over_cap} over cap {GROUP_CAP}); \
                     {e2_attached} pad rows attach to keyed groups, {e2_orphan_keys} pad-only \
                     keys; {null_country_keyed} NULL-country keyed (R3 pool), \
                     {prefix_contradictions} country/prefix contradictions; denials: \
                     {es_ute} ES-UTE, {cz699} CZ699"
                ))
            }
            Spec::R3Census => {
                // Issue 300 Stage 3 opening census, read-only: classify the
                // NULL-country identifier pool by checksum anchoring, standing
                // target existence, and cross-language name corroboration —
                // the exact conditions the R3 merge will demand, measured
                // before the merge exists (the campaign's standing pattern).
                self.set_phase("censusing", None, None, "preloading canonical keys".to_owned());
                // Standing (country-ful) orgs' E1 canonical keys — the
                // rescue's target map.
                let mut canon: std::collections::HashMap<(&'static str, String), Vec<i64>> =
                    std::collections::HashMap::new();
                let mut watermark = 0i64;
                loop {
                    if self.cancelled(job.id) {
                        return Ok("r3-census stopped by cancel — no report stored".to_owned());
                    }
                    let (rows, next) = self
                        .db
                        .org_merge_health_batch(ingest::project::match_norm, BACKFILL_BATCH, watermark)
                        .await
                        .map_err(|e| e.to_string())?;
                    if rows.is_empty() {
                        break;
                    }
                    for r in &rows {
                        let kind = r.kind.clone().unwrap_or_else(|| "national".into());
                        if r.country.is_none() {
                            continue;
                        }
                        if let Some((scheme, key, true)) = ingest::crosswalk::canonical_key_flat(
                            r.country.as_deref(),
                            &kind,
                            &r.identifier,
                        ) {
                            canon.entry((scheme, key)).or_default().push(r.org_id);
                        }
                    }
                    watermark = next;
                }
                let pool = self.db.null_country_ident_orgs().await.map_err(|e| e.to_string())?;
                let total = pool.len() as u64;
                let (mut anchored_corr, mut anchored_uncorr, mut anchored_no_target) =
                    (0u64, 0u64, 0u64);
                let (mut unanchored_none, mut unanchored_multi, mut register_prefixed) =
                    (0u64, 0u64, 0u64);
                let mut samples: Vec<serde_json::Value> = Vec::new();
                for (id, _kind, value, name) in &pool {
                    if self.cancelled(job.id) {
                        return Ok("r3-census stopped by cancel — no report stored".to_owned());
                    }
                    if value.bytes().any(|b| b.is_ascii_alphabetic()) {
                        register_prefixed += 1;
                        continue;
                    }
                    let anchors = ingest::idgate::checksum_anchors(value);
                    // The DK|SI marker is ambiguity-by-construction, never an
                    // anchor of its own.
                    let real: Vec<_> =
                        anchors.iter().filter(|(s, _)| !s.contains('|')).collect();
                    let ambiguous = anchors.len() > real.len() || real.len() > 1;
                    match (real.len(), ambiguous) {
                        (0, _) => unanchored_none += 1,
                        (_, true) => unanchored_multi += 1,
                        (1, false) => {
                            let (scheme, key) = real[0];
                            let Some(targets) = canon.get(&(scheme, key.clone())) else {
                                anchored_no_target += 1;
                                continue;
                            };
                            // Cross-language N2 corroboration against every
                            // target name (head + satellite).
                            let n2 = ingest::project::match_norm(name);
                            let mut corroborated = false;
                            'targets: for &t in targets {
                                for tn in
                                    self.db.org_all_names(t).await.map_err(|e| e.to_string())?
                                {
                                    if !n2.is_empty()
                                        && ingest::project::match_norm(&tn) == n2
                                    {
                                        corroborated = true;
                                        break 'targets;
                                    }
                                }
                            }
                            if corroborated {
                                anchored_corr += 1;
                                if samples.len() < 40 {
                                    samples.push(serde_json::json!({
                                        "org_id": id, "identifier": value, "name": name,
                                        "scheme": scheme, "key": key,
                                        "targets": targets,
                                    }));
                                }
                            } else {
                                anchored_uncorr += 1;
                            }
                        }
                        _ => unreachable!("covered above"),
                    }
                }
                let now = store::now_unix();
                let report = serde_json::json!({
                    "pool": total,
                    "anchored_corroborated": anchored_corr,
                    "anchored_uncorroborated": anchored_uncorr,
                    "anchored_no_target": anchored_no_target,
                    "unanchored_none": unanchored_none,
                    "unanchored_multi": unanchored_multi,
                    "register_prefixed": register_prefixed,
                    "sample": samples,
                })
                .to_string();
                self.db.put_report("r3-census", &report, now).await.map_err(|e| e.to_string())?;
                Ok(format!(
                    "r3-census (issue 300 Stage 3): {total} NULL-country identifier orgs — \
                     {anchored_corr} anchored+corroborated (R3 merge candidates), \
                     {anchored_uncorr} anchored without name corroboration (edges), \
                     {anchored_no_target} anchored with no standing target, \
                     {unanchored_multi} multi-scheme ambiguous (EBSCO class), \
                     {unanchored_none} unanchored, {register_prefixed} register-prefixed \
                     (the separate R3 alternative)"
                ))
            }
            Spec::MatchOrgIdentifiersR2 { dry_run, max_groups } => {
                let dry_run = *dry_run;
                // The ingest-side rules, handed across as plain fns (the
                // idgate/dissolve pattern); the flat crosswalk is the SAME fn
                // the resolver's prevention hook injects, so repair and
                // prevention cannot drift apart.
                // A wet run REQUIRES the recorded dry plan: the T4 parity
                // input, and the ladder's guarantee that nothing merges
                // un-previewed.
                let expect_groups = if dry_run {
                    None
                } else {
                    let (body, _) = self
                        .db
                        .latest_report("r2-merge-plan")
                        .await
                        .map_err(|e| e.to_string())?
                        .ok_or_else(|| {
                            "no stored r2-merge-plan — run the dry run first".to_owned()
                        })?;
                    let v: serde_json::Value =
                        serde_json::from_str(&body).map_err(|e| e.to_string())?;
                    Some(
                        v["plan_groups"]
                            .as_u64()
                            .ok_or_else(|| "r2-merge-plan lacks plan_groups".to_owned())?,
                    )
                };
                self.set_phase(
                    if dry_run { "planning" } else { "merging" },
                    None,
                    None,
                    "preloading the identifier-bearing org layer".to_owned(),
                );
                let job_id = job.id;
                let stop = || self.cancelled(job_id);
                let r = self
                    .db
                    .match_org_identifiers_r2(store::R2MergeArgs {
                        key: ingest::crosswalk::canonical_key_flat,
                        condemns: ingest::idgate::condemns,
                        consortium: ingest::crosswalk::consortium_name,
                        legal_form: ingest::crosswalk::legal_form_family,
                        dry_run,
                        max_groups: *max_groups,
                        expect_groups,
                        job_id: Some(job_id as i64),
                        stop: &stop,
                    })
                    .await
                    .map_err(|e| e.to_string())?;
                // Keep the recorded plan CURRENT (verifier catch: a capped or
                // stopped wet run leaves merged groups out of the next
                // recompute, so a continuation would parity-abort against the
                // stale figure until someone re-ran the dry run — discarding
                // the reviewed plan). A dry run records its full plan; a wet
                // run re-records the RESIDUAL, so the next capped slice
                // continues under parity without ceremony. EXCEPT a stop
                // during classification (stopped, empty plan): it computed
                // nothing, and recording plan_groups 0 would clobber a
                // reviewed plan (the R3 verification round's catch, mirrored
                // here).
                if !(r.stopped && r.plan_groups == 0) {
                    let now = store::now_unix();
                    let plan = serde_json::json!({
                        "plan_groups": r.plan_groups - r.merged_groups,
                        "scanned": r.scanned, "keyed": r.keyed, "groups": r.groups,
                        "denied_cap": r.denied_cap, "denied_gate": r.denied_gate,
                        "denied_consortium": r.denied_consortium,
                        "consortium_excluded": r.consortium_excluded,
                        "denied_legal_form": r.denied_legal_form,
                        "denied_group_vat": r.denied_group_vat,
                        "merged_this_run": r.merged_groups,
                        "residual_of_wet_run": !dry_run,
                        // Dry-run blast-radius preview (the Stage-1 lesson):
                        // what the plan's merges would move.
                        "mentions": r.mentions, "parties": r.parties,
                        "bid_parties": r.bid_parties, "winners": r.winners,
                        // The precision-review sample (dry runs only; empty
                        // on wet re-records).
                        // The plan as a LISTING (issue 326): complete when it
                        // fits the cap, which is what makes a 200-group merge
                        // reviewable at all. `sample` below stays as it was —
                        // a 1-in-199 content-stable draw, unbiased over large
                        // plans but exactly one row over a small one.
                        "plan_listing_truncated": r.plan_listing_truncated,
                        "plan": r.plan_listing.iter().map(|(country, scheme, key, members)| {
                            serde_json::json!({
                                "country": country, "scheme": scheme, "key": key,
                                "members": members.iter().map(|(id, kind, literal, name)| {
                                    serde_json::json!({
                                        "org_id": id, "kind": kind,
                                        "identifier": literal, "name": name,
                                    })
                                }).collect::<Vec<_>>(),
                            })
                        }).collect::<Vec<_>>(),
                        "sample": r.plan_sample.iter().map(|(country, scheme, key, members)| {
                            serde_json::json!({
                                "country": country, "scheme": scheme, "key": key,
                                "members": members.iter().map(|(id, kind, literal, name)| {
                                    serde_json::json!({
                                        "org_id": id, "kind": kind,
                                        "identifier": literal, "name": name,
                                    })
                                }).collect::<Vec<_>>(),
                            })
                        }).collect::<Vec<_>>(),
                    })
                    .to_string();
                    self.db
                        .put_report("r2-merge-plan", &plan, now)
                        .await
                        .map_err(|e| e.to_string())?;
                }
                if r.stopped {
                    return Ok(if r.plan_groups == 0 {
                        "match-org-identifiers r2 STOPPED during classification: nothing \
                         was planned or written, and the previously recorded plan was \
                         left untouched"
                            .to_owned()
                    } else {
                        format!(
                            "match-org-identifiers r2 STOPPED at a checkpoint: {} of {} plan \
                             groups merged before the stop; the residual plan was re-recorded, \
                             so a re-run continues under parity",
                            r.merged_groups, r.plan_groups
                        )
                    });
                }
                Ok(format!(
                    "match-org-identifiers r2 (issue 300 Stage 2){}: {} orgs scanned, \
                     {} E1-keyed, {} groups >=2; denied: {} cap, {} gate, {} consortium \
                     ({} members excluded member-scoped), \
                     {} legal-form, {} vat-group-wall; plan {} groups; merged {} groups \
                     ({} org rows removed, {} mentions, {} parties, {} bid-parties, \
                     {} winners repointed, {} winner dups deleted, {} tenders touched)",
                    if dry_run { " DRY RUN — plan recorded, nothing written" } else { "" },
                    r.scanned,
                    r.keyed,
                    r.groups,
                    r.denied_cap,
                    r.denied_gate,
                    r.denied_consortium,
                    r.consortium_excluded,
                    r.denied_legal_form,
                    r.denied_group_vat,
                    r.plan_groups,
                    r.merged_groups,
                    r.removed,
                    r.mentions,
                    r.parties,
                    r.bid_parties,
                    r.winners,
                    r.winner_dups,
                    r.tender_changes
                ))
            }
            Spec::MatchOrgIdentifiersR3 { dry_run, max_groups } => {
                // Issue 300 Stage 3: the NULL-country rescue merge — the
                // r3-census's ladder recomputed live, hardened with the R2
                // denial stack. Same T4 ladder as R2: a wet run REQUIRES the
                // recorded r3-merge-plan, and re-records the residual.
                let dry_run = *dry_run;
                // Issue 316: r3 corroboration consults the generic-name wall,
                // and an unbuilt or mid-build key satellite makes that wall
                // invisible — every key would read non-generic. A DRY run may
                // still plan (its plan is reviewed by a person), but a WET one
                // refuses rather than merging under a wall that is not there.
                // Issue 316: r3 corroboration consults the generic-name wall,
                // so the satellite has to be in a state the wall can be read
                // from. Which refusals apply to a DRY run and which only to a
                // WET one is the whole subtlety here (panel catch: the first
                // version guarded wet only, and the dry run — the DEFAULT for
                // this job — kept the probe that has no index to stand on).
                let (watermark, epoch) =
                    self.db.org_match_keys_state().await.map_err(|e| e.to_string())?;
                let keys = self.db.org_match_keys_count().await.map_err(|e| e.to_string())?;
                let rebuild = "run a WET build first: \
                               {\"kind\":\"build-org-match-keys\",\"dry_run\":false} \
                               — the default is DRY and stores nothing";
                // Mid-build: the covering index does not exist yet, so every
                // candidate's probe becomes a full scan of a multi-million-row
                // table, and the half-written keyspace answers wrongly anyway.
                // Both halves apply to a dry run as much as a wet one — a dry
                // run costs the same and records a plan a wet run trusts.
                if watermark != 0 {
                    return Err(format!(
                        "match-org-identifiers r3 REFUSED: a key build is in flight \
                         (watermark {watermark}); its covering index does not exist yet, \
                         so the generic-name probe would full-scan {keys} rows per \
                         candidate and read a half-built keyspace"
                    ));
                }
                // Epoch drift, same reasoning as the scan's own rung: keys
                // built under superseded semantics are a keyspace whose
                // membership no longer matches what `norm` produces today, so
                // the wall answers a DIFFERENT question and reads "not
                // generic" for every key that moved. Skipped when the
                // satellite is empty, so a fresh box reports emptiness rather
                // than an empty-string epoch.
                if keys > 0 && epoch != ingest::crosswalk::NAME_KEY_EPOCH {
                    return Err(format!(
                        "match-org-identifiers r3 REFUSED: org_match_keys carries keys epoch \
                         {epoch:?}, this binary's is {:?} — the generic-name wall would \
                         answer about a superseded keyspace; {rebuild}",
                        ingest::crosswalk::NAME_KEY_EPOCH
                    ));
                }
                // An EMPTY satellite is the one state only a wet run refuses:
                // the probe is cheap there (nothing to scan) and every key
                // reads non-generic, which is a fine state to PLAN in — a
                // person reads the plan — but not one to merge under, because
                // the wall would silently not be there.
                if !dry_run && keys == 0 {
                    return Err(format!(
                        "match-org-identifiers r3 REFUSED: org_match_keys is empty, so the \
                         issue-316 generic-name wall cannot see anything — {rebuild}"
                    ));
                }
                let expect_groups = if dry_run {
                    None
                } else {
                    let (body, _) = self
                        .db
                        .latest_report("r3-merge-plan")
                        .await
                        .map_err(|e| e.to_string())?
                        .ok_or_else(|| {
                            "no stored r3-merge-plan — run the dry run first".to_owned()
                        })?;
                    let v: serde_json::Value =
                        serde_json::from_str(&body).map_err(|e| e.to_string())?;
                    Some(
                        v["plan_groups"]
                            .as_u64()
                            .ok_or_else(|| "r3-merge-plan lacks plan_groups".to_owned())?,
                    )
                };
                self.set_phase(
                    if dry_run { "planning" } else { "merging" },
                    None,
                    None,
                    "preloading the canonical target map".to_owned(),
                );
                let job_id = job.id;
                let stop = || self.cancelled(job_id);
                let r = self
                    .db
                    .match_org_null_country_r3(store::R3MergeArgs {
                        key: ingest::crosswalk::canonical_key_flat,
                        anchors: ingest::idgate::checksum_anchors,
                        condemns: ingest::idgate::condemns,
                        consortium: ingest::crosswalk::consortium_name,
                        legal_form: ingest::crosswalk::legal_form_family,
                        norm: ingest::project::match_norm,
                        hard_scheme: ingest::idgate::hard_scheme,
                        stoplist_cap: SCAN_STOPLIST_CAP,
                        dry_run,
                        max_groups: *max_groups,
                        expect_groups,
                        job_id: Some(job_id as i64),
                        stop: &stop,
                    })
                    .await
                    .map_err(|e| e.to_string())?;
                // Keep the recorded plan current (the R2 lesson): a dry run
                // records the full plan, a wet run the residual, so a capped
                // continuation runs under parity without a fresh dry run.
                // EXCEPT a stop during classification (stopped with an empty
                // plan): it computed nothing, and recording plan_groups 0
                // would clobber a reviewed plan (verification-round catch).
                // No sample here — the r3-census's 40-candidate sample is
                // the precision-review material and stays put.
                let classify_stopped = r.stopped && r.plan_groups == 0;
                if !classify_stopped {
                    let now = store::now_unix();
                    let plan = serde_json::json!({
                        "plan_groups": r.plan_groups - r.merged_groups,
                        "pool": r.pool,
                        "register_prefixed": r.register_prefixed,
                        "unanchored": r.unanchored,
                        "no_target": r.no_target,
                        "multi_target": r.multi_target,
                        "uncorroborated": r.uncorroborated,
                        "denied_generic_name": r.denied_generic_name,
                        "generic_name_hard_anchor": r.generic_name_hard_anchor,
                        "denied_gate": r.denied_gate,
                        "denied_consortium": r.denied_consortium,
                        "denied_legal_form": r.denied_legal_form,
                        "denied_group_vat": r.denied_group_vat,
                        "denied_cap": r.denied_cap,
                        "denied_generic_name": r.denied_generic_name,
                        "generic_name_hard_anchor": r.generic_name_hard_anchor,
                        // Panel catch: a plan computed with a BLIND wall (an
                        // empty satellite, which only a wet run refuses) is
                        // otherwise indistinguishable from one computed with a
                        // readable one — both report denied_generic_name 0. The
                        // plan says which it was, so a reviewer reading it later
                        // cannot mistake "nothing was generic" for "nothing
                        // could be seen".
                        "generic_wall_readable": keys > 0,
                        "merged_this_run": r.merged_groups,
                        "residual_of_wet_run": !dry_run,
                        "mentions": r.mentions, "parties": r.parties,
                        "bid_parties": r.bid_parties, "winners": r.winners,
                    })
                    .to_string();
                    self.db
                        .put_report("r3-merge-plan", &plan, now)
                        .await
                        .map_err(|e| e.to_string())?;
                }
                if r.stopped {
                    return Ok(if classify_stopped {
                        "match-org-identifiers r3 STOPPED during classification: nothing \
                         was planned or written, and the previously recorded plan was \
                         left untouched"
                            .to_owned()
                    } else {
                        format!(
                            "match-org-identifiers r3 STOPPED at a checkpoint: {} of {} plan \
                             candidates merged before the stop; the residual plan was \
                             re-recorded, so a re-run continues under parity",
                            r.merged_groups, r.plan_groups
                        )
                    });
                }
                Ok(format!(
                    "match-org-identifiers r3 (issue 300 Stage 3){}: pool {}; skipped: \
                     {} register-prefixed, {} unanchored/ambiguous, {} no-target, \
                     {} multi-target, {} uncorroborated; denied: {} generic-name \
                     ({} generic but hard-anchored), {} gate, {} consortium, \
                     {} legal-form, {} vat-group-wall, {} co-anchor-cap; plan {} \
                     candidates; merged {} \
                     ({} org rows removed, {} mentions, {} parties, {} bid-parties, \
                     {} winners repointed, {} winner dups deleted, {} tenders touched){}",
                    if dry_run { " DRY RUN — plan recorded, nothing written" } else { "" },
                    r.pool,
                    r.register_prefixed,
                    r.unanchored,
                    r.no_target,
                    r.multi_target,
                    r.uncorroborated,
                    r.denied_generic_name,
                    r.generic_name_hard_anchor,
                    r.denied_gate,
                    r.denied_consortium,
                    r.denied_legal_form,
                    r.denied_group_vat,
                    r.denied_cap,
                    r.plan_groups,
                    r.merged_groups,
                    r.removed,
                    r.mentions,
                    r.parties,
                    r.bid_parties,
                    r.winners,
                    r.winner_dups,
                    r.tender_changes,
                    if keys == 0 {
                        " — NOTE: org_match_keys is EMPTY, so the generic-name wall saw \
                         nothing and denied nothing"
                    } else {
                        ""
                    }
                ))
            }
            Spec::FoldOrgCountries { dry_run } => {
                let dry_run = *dry_run;
                let job_id = job.id;
                let stop = || self.cancelled(job_id);
                self.set_phase(
                    if dry_run { "planning" } else { "folding" },
                    None,
                    None,
                    "reading the distinct country values".to_owned(),
                );
                // The T4 ladder: a wet run executes the plan a person read.
                let expect_rows = if dry_run {
                    None
                } else {
                    let (body, _) = self
                        .db
                        .latest_report("country-fold")
                        .await
                        .map_err(|e| e.to_string())?
                        .ok_or_else(|| "no stored country-fold plan — run the dry pass first")?;
                    let v: serde_json::Value =
                        serde_json::from_str(&body).map_err(|e| e.to_string())?;
                    if v["dry_run"] != serde_json::Value::Bool(true) {
                        return Err("the stored country-fold report is from a WET run, not a \
                                    reviewed plan — run the dry pass again"
                            .to_owned());
                    }
                    Some(
                        v["rows"].as_u64().ok_or_else(|| "country-fold plan lacks rows")?,
                    )
                };
                let now = store::now_unix();
                let r = self
                    .db
                    .fold_org_countries(
                        ingest::project::canonical_country,
                        ingest::countries::is_alpha2,
                        dry_run,
                        expect_rows,
                        now,
                        &stop,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                if r.stopped {
                    return Ok("fold-org-countries STOPPED by cancel — nothing planned or \
                               written"
                        .to_owned());
                }
                let body = serde_json::json!({
                    "dry_run": dry_run,
                    "values": r.values, "rows": r.rows, "collisions": r.collisions,
                    "vat_scope_skipped": r.vat_scope_skipped.iter().map(|(v, n)| {
                        serde_json::json!({ "value": v, "rows": n })
                    }).collect::<Vec<_>>(),
                    "plan": r.plan.iter().map(|(f, t, n)| {
                        serde_json::json!({ "from": f, "to": t, "rows": n })
                    }).collect::<Vec<_>>(),
                    "unmapped": r.unmapped.iter().map(|(v, n)| {
                        serde_json::json!({ "value": v, "rows": n })
                    }).collect::<Vec<_>>(),
                })
                .to_string();
                self.db
                    .put_report("country-fold", &body, now)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(format!(
                    "fold-org-countries (issue 319){}: {} distinct country values, {} to \
                     fold over {} rows; {} identifier-bearing rows land on an identity that \
                     already stands (R2's to merge, not this job's); {} values stay \
                     unmapped{}; {} VAT-scope values SKIPPED on purpose (the resolver \
                     binds on them)",
                    if dry_run { " DRY RUN — plan recorded, nothing written" } else { "" },
                    r.values,
                    r.plan.len(),
                    r.rows,
                    r.collisions,
                    r.unmapped.len(),
                    match r.unmapped.first() {
                        Some((v, n)) => format!(" (largest: {v:?}, {n} rows)"),
                        None => String::new(),
                    },
                    r.vat_scope_skipped.len()
                ))
            }
            Spec::ApplyRehoming { dry_run } => {
                let dry_run = *dry_run;
                self.set_phase(
                    if dry_run { "planning" } else { "rehoming" },
                    None,
                    None,
                    "walking unapplied re-homing verdicts".to_owned(),
                );
                // The T4 ladder: a wet run executes the move list a person
                // read, compared tuple-wise — the destination org is exactly
                // what a reviewer of this plan is checking, and a count
                // cannot show that it changed.
                let expect: Option<Vec<(i64, String, i64, i64)>> = if dry_run {
                    None
                } else {
                    let (body, _) = self
                        .db
                        .latest_report("rehoming-plan")
                        .await
                        .map_err(|e| e.to_string())?
                        .ok_or_else(|| "no stored rehoming-plan — run the dry pass first")?;
                    let v: serde_json::Value =
                        serde_json::from_str(&body).map_err(|e| e.to_string())?;
                    let moves = v["plan"].as_array().ok_or_else(|| "plan lacks moves")?;
                    Some(
                        moves
                            .iter()
                            .map(|m| {
                                (
                                    m["notice"].as_i64().unwrap_or(-1),
                                    m["section"].as_str().unwrap_or_default().to_owned(),
                                    m["from"].as_i64().unwrap_or(-1),
                                    m["to"].as_i64().unwrap_or(-1),
                                )
                            })
                            .collect(),
                    )
                };
                let r = self
                    .db
                    .apply_rehoming(
                        dry_run,
                        expect.as_deref(),
                        Some(job.id as i64),
                        store::now_unix(),
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                if dry_run {
                    // The CONCRETE move list, not counts: a destination org id
                    // is exactly the thing a counts-only preview cannot show
                    // to be wrong (the issue-311 panel's lesson).
                    let now = store::now_unix();
                    let body = serde_json::json!({
                        "pending": r.pending, "eligible": r.eligible, "moves": r.moved,
                        "noop": r.noop, "missing_target": r.missing_target,
                        "plan": r.plan.iter().map(|(n, s, from, to, name)| {
                            serde_json::json!({
                                "notice": n, "section": s, "from": from, "to": to,
                                "target_name": name,
                            })
                        }).collect::<Vec<_>>(),
                    })
                    .to_string();
                    self.db
                        .put_report("rehoming-plan", &body, now)
                        .await
                        .map_err(|e| e.to_string())?;
                }
                // `refold_notices` is the row count of the re-queue UPDATE, so
                // it exists only on the wet path. Printing it as "0 notices
                // are re-queued" beside 258 touched tenders is a dry-path zero
                // reading as a finding — the sentence said the fold had
                // nothing to do when the wet run then re-queued 1,710.
                let requeue = if dry_run {
                    ", whose causing notices the wet run will re-queue".to_owned()
                } else {
                    format!(", whose {} notices are re-queued", r.refold_notices)
                };
                Ok(format!(
                    "apply-rehoming (issue 317 Unit A){}: {} pending verdicts, {} eligible; \
                     {} mentions {} ({} party and {} bid-party rows follow them across \
                     {} tenders{} so the fold re-derives the winners); {} no-ops, {} name \
                     a target org that does not exist or no target at all (v1 never mints \
                     one)",
                    if dry_run { " DRY RUN — plan recorded, nothing written" } else { "" },
                    r.pending,
                    r.eligible,
                    r.moved,
                    if dry_run { "would move" } else { "moved" },
                    r.parties,
                    r.bid_parties,
                    r.tenders,
                    requeue,
                    r.noop,
                    r.missing_target
                ))
            }
            Spec::FusionCensus => {
                let job_id = job.id;
                let stop = || self.cancelled(job_id);
                self.set_phase("censusing", None, None, "reading reviewed rows' mentions".to_owned());
                let r = self
                    .db
                    .fusion_candidates(ingest::project::match_norm, 120, &stop)
                    .await
                    .map_err(|e| e.to_string())?;
                if r.stopped {
                    return Ok("fusion-census STOPPED by cancel — no report stored".to_owned());
                }
                let now = store::now_unix();
                let body = serde_json::json!({
                    "cases": r.cases, "with_mentions": r.with_mentions,
                    "fused": r.fused, "off_name_mentions": r.off_name_mentions,
                    "truncated": r.truncated,
                    "candidates": r.candidates.iter().map(|c| serde_json::json!({
                        "org": c.org, "cohort": c.cohort, "name": c.name,
                        "mentions": c.mentions, "off_name": c.off_name,
                        "groups": c.groups.iter().map(|(n, k)| {
                            serde_json::json!({ "name": n, "mentions": k })
                        }).collect::<Vec<_>>(),
                    })).collect::<Vec<_>>(),
                })
                .to_string();
                self.db
                    .put_report("fusion-candidates", &body, now)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(format!(
                    "fusion-census (issue 317 Unit A): {} applied case rows, {} with \
                     mentions; {} hold at least one mention naming somebody ELSE, over \
                     {} such mentions{}",
                    r.cases,
                    r.with_mentions,
                    r.fused,
                    r.off_name_mentions,
                    if r.truncated { " (candidate list TRUNCATED at the cap)" } else { "" }
                ))
            }
            Spec::RehomingPacket => {
                let job_id = job.id;
                let stop = || self.cancelled(job_id);
                // Preconditions, the `scan-org-match-keys` set: every
                // destination in this packet comes from the N2 satellite, so
                // an index-less, mid-walk or wrong-epoch satellite does not
                // make the packet WRONG in a way anyone can see — it makes
                // every group read "no destination anywhere", which is the
                // one answer a reviewer cannot tell from a real finding.
                // Refuse instead of publishing it.
                if !self.db.has_index("org_match_keys_kk").await.map_err(|e| e.to_string())? {
                    return Err("rehoming-packet refused: no org_match_keys_kk index — \
                                run build-org-match-keys (wet) first"
                        .to_owned());
                }
                let (wm, epoch) =
                    self.db.org_match_keys_state().await.map_err(|e| e.to_string())?;
                if wm != 0 {
                    return Err(format!(
                        "rehoming-packet refused: keys build in flight (watermark {wm}) — \
                         let it finish or rerun build-org-match-keys"
                    ));
                }
                if epoch != ingest::crosswalk::NAME_KEY_EPOCH {
                    return Err(format!(
                        "rehoming-packet refused: keys built under epoch {epoch:?}, this \
                         binary keys under {:?} — rerun build-org-match-keys (wet) first",
                        ingest::crosswalk::NAME_KEY_EPOCH
                    ));
                }
                // Preconditions cannot catch STALENESS, which is the same
                // silence arriving later: a satellite built three weeks ago
                // passes all three and still reports every org minted since
                // as having no destination. So the packet carries the build's
                // provenance, and a packet read a month on says so itself.
                let keys_built_at = match self
                    .db
                    .latest_report("org-match-keys-build")
                    .await
                    .map_err(|e| e.to_string())?
                {
                    Some((body, at)) => {
                        let rows = serde_json::from_str::<serde_json::Value>(&body)
                            .ok()
                            .and_then(|v| v.get("rows").and_then(|r| r.as_u64()));
                        Some((at, rows))
                    }
                    None => None,
                };
                self.set_phase(
                    "packing",
                    None,
                    None,
                    "reading undecided off-name mentions and their destinations".to_owned(),
                );
                // The census measured 102 fused rows over 585 off-name
                // mentions, so 150 cases and 80 mentions each hold the whole
                // workload with room — and the packet EXCLUDES what is
                // already decided, so it shrinks as the campaign runs.
                // SCAN_STOPLIST_CAP is threaded rather than duplicated: this
                // wall and the E3 scan's must be the same number or a name
                // the scan calls generic is a destination here.
                let r = self
                    .db
                    .rehoming_packet(
                        ingest::project::match_norm,
                        150,
                        80,
                        5,
                        SCAN_STOPLIST_CAP,
                        &stop,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                if r.stopped {
                    return Ok("rehoming-packet STOPPED by cancel — no report stored".to_owned());
                }
                let now = store::now_unix();
                // Built as named closures rather than one nested literal:
                // `json!` expands recursively, and four levels of it blows
                // the macro recursion limit.
                let target = |t: &store::RehomingTarget| {
                    serde_json::json!({
                        "org": t.org, "name": t.name, "country": t.country,
                        "identifier_kind": t.identifier_kind, "identifier": t.identifier,
                        "mentions": t.mentions, "saturated": t.saturated,
                        "via_alias": t.via_alias, "identifier_match": t.identifier_match,
                    })
                };
                let group = |g: &store::RehomingGroup| {
                    serde_json::json!({
                        "key": g.key, "name": g.name, "mentions": g.mentions,
                        "target_total": g.target_total, "generic_key": g.generic_key,
                        "targets": g.targets.iter().map(&target).collect::<Vec<_>>(),
                    })
                };
                let mention = |m: &store::RehomingMention| {
                    serde_json::json!({
                        "notice_id": m.notice_id, "section_id": m.section_id,
                        "name": m.name, "key": m.key, "country": m.country,
                        "raw_identifier": m.raw_identifier, "scheme": m.scheme,
                        "group_shown": m.group_shown,
                        "notice_orgs": m.notice_orgs,
                    })
                };
                let case = |c: &store::RehomingCase| {
                    serde_json::json!({
                        "org": c.org, "cohort": c.cohort, "name": c.name,
                        "country": c.country,
                        "identifier_kind": c.identifier_kind, "identifier": c.identifier,
                        "mentions": c.mentions, "off_name": c.off_name, "open": c.open,
                        "groups_elided": c.groups_elided,
                        "mentions_truncated": c.mentions_truncated,
                        "groups": c.groups.iter().map(&group).collect::<Vec<_>>(),
                        "off_mentions": c.off_mentions.iter().map(&mention).collect::<Vec<_>>(),
                    })
                };
                let parked = |p: &store::RehomingParked| {
                    serde_json::json!({
                        "case_org": p.case_org, "notice_id": p.notice_id,
                        "section_id": p.section_id, "action": p.action,
                        "confidence": p.confidence, "target_org_id": p.target_org_id,
                        "target_name": p.target_name, "reason": p.reason,
                    })
                };
                let body = serde_json::json!({
                    "cases": r.cases, "open": r.open,
                    "off_name_mentions": r.off_name_mentions,
                    "already_reviewed": r.already_reviewed,
                    "parked_total": r.parked_total,
                    "parked": r.parked.iter().map(&parked).collect::<Vec<_>>(),
                    "groups_total": r.groups_total,
                    "probed_groups": r.probed_groups,
                    "groups_elided": r.groups_elided,
                    "probed_groups_with_target": r.probed_groups_with_target,
                    "groups_generic": r.groups_generic,
                    "listed_cases": r.rows.len(),
                    "truncated": r.truncated,
                    "keys_epoch": epoch,
                    "keys_built_at": keys_built_at.map(|(at, _)| at),
                    "keys_rows": keys_built_at.and_then(|(_, rows)| rows),
                    "rows": r.rows.iter().map(&case).collect::<Vec<_>>(),
                })
                .to_string();
                self.db.put_report("rehoming-packet", &body, now).await.map_err(|e| e.to_string())?;
                // Every number in this sentence is scoped: the workload
                // counts cover every case, the destination counts cover the
                // groups actually probed, and they are named apart so the
                // sentence cannot read as one ratio over one population.
                Ok(format!(
                    "rehoming-packet (issue 317 Unit A): {} applied case orgs, {} still hold \
                     an UNDECIDED off-name mention, over {} such mentions in {} name groups; \
                     listed {} cases probing {} groups ({} have a standing destination, {} are \
                     a shared literal over the genericness wall); {} mentions already decided, \
                     {} verdicts PARKED (recorded but unappliable){}{}",
                    r.cases,
                    r.open,
                    r.off_name_mentions,
                    r.groups_total,
                    r.rows.len(),
                    r.probed_groups,
                    r.probed_groups_with_target,
                    r.groups_generic,
                    r.already_reviewed,
                    r.parked_total,
                    if r.groups_elided > 0 {
                        format!("; {} groups elided by the per-case cap", r.groups_elided)
                    } else {
                        String::new()
                    },
                    if r.truncated { "; CASE LIST TRUNCATED at the cap" } else { "" }
                ))
            }
            Spec::DropOrphanSatellites { dry_run } => {
                let job_id = job.id;
                let stop = || self.cancelled(job_id);
                let dry_run = *dry_run;
                self.set_phase(
                    if dry_run { "planning" } else { "dropping" },
                    None,
                    None,
                    "issue 321: orphaned name variants on re-homing origins".to_owned(),
                );
                // The wet arm reads the plan the dry arm recorded and compares
                // it as tuples. No plan on record is not a reason to guess:
                // a wet run without one is refused, because the parity check
                // is half of what makes this safe.
                let plan: Option<Vec<(i64, String, String, Option<i64>)>> = if dry_run {
                    None
                } else {
                    let stored = self
                        .db
                        .latest_report("drop-orphan-satellites")
                        .await
                        .map_err(|e| e.to_string())?;
                    // A refusal is a FAILED job, not a green one. Both of
                    // these mean "I would not do the thing you asked", and a
                    // run that renders green in /admin/jobs while having
                    // written nothing is how an operator concludes the
                    // campaign is done.
                    let Some((body, _)) = stored else {
                        eprintln!(
                            "[drop-orphan-satellites] REFUSED: no dry plan on record"
                        );
                        return Err("drop-orphan-satellites --wet REFUSED: no dry plan on \
                                    record. Run the dry pass first — the tuple parity between \
                                    plan and run is half of what makes this safe."
                            .to_owned());
                    };
                    let v: serde_json::Value =
                        serde_json::from_str(&body).map_err(|e| e.to_string())?;
                    Some(
                        v["rows"]
                            .as_array()
                            .map(|a| a.as_slice())
                            .unwrap_or_default()
                            .iter()
                            .filter_map(|r| {
                                Some((
                                    r["org"].as_i64()?,
                                    r["lang"].as_str()?.to_owned(),
                                    r["key"].as_str()?.to_owned(),
                                    // The destination the plan was reviewed
                                    // against. The dry report already carries
                                    // it; leaving it out of the tuple let a
                                    // plan pass parity while pointing
                                    // somewhere else entirely.
                                    r["target"].as_i64(),
                                ))
                            })
                            .collect(),
                    )
                };
                let now = store::now_unix();
                let r = self
                    .db
                    .drop_orphan_satellites(
                        ingest::project::match_norm,
                        dry_run,
                        plan.as_deref(),
                        Some(job_id as i64),
                        now,
                        &stop,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                if r.stopped {
                    return Ok(
                        "drop-orphan-satellites STOPPED by cancel — nothing written".to_owned()
                    );
                }
                if r.no_plan {
                    eprintln!("[drop-orphan-satellites] REFUSED: no plan reached the store");
                    return Err("drop-orphan-satellites --wet REFUSED by the store: no dry \
                                plan was supplied. The tuple parity between plan and run is \
                                half of what makes this safe."
                        .to_owned());
                }
                if r.drifted {
                    eprintln!(
                        "[drop-orphan-satellites] REFUSED: plan drift, {} added / {} gone",
                        r.plan_added.len(),
                        r.plan_removed.len()
                    );
                    return Err(format!(
                        "drop-orphan-satellites --wet REFUSED: the plan drifted. {} tuple(s) \
                         appeared since the dry run and {} went away — first added {:?}, first \
                         gone {:?}. Re-run the dry pass and read it before the wet one; a \
                         count would not have shown this.",
                        r.plan_added.len(),
                        r.plan_removed.len(),
                        r.plan_added.first(),
                        r.plan_removed.first(),
                    ));
                }
                if dry_run {
                    let body = serde_json::json!({
                        "candidates": r.candidates,
                        "rows": r.rows.iter().map(|o| serde_json::json!({
                            "org": o.org, "org_name": o.org_name, "lang": o.lang,
                            "name": o.name, "key": o.key,
                            "target": o.target, "target_name": o.target_name,
                        })).collect::<Vec<_>>(),
                    })
                    .to_string();
                    self.db
                        .put_report("drop-orphan-satellites", &body, now)
                        .await
                        .map_err(|e| e.to_string())?;
                    return Ok(format!(
                        "drop-orphan-satellites DRY (issue 321): {} variant(s) would be \
                         dropped — each unsupported by any mention on its org, not that \
                         org's own head name, and already standing on a row the org \
                         re-homed to. Plan recorded; the wet arm compares against it as \
                         (org, lang, key, destination) tuples — the destination is in the \
                         tuple because an origin's verdicts can name several, and a plan \
                         reviewed against one must not run against another.",
                        r.candidates
                    ));
                }
                if r.cancelled {
                    return Ok(format!(
                        "drop-orphan-satellites WET CANCELLED PART-WAY (issue 321): {} of {} \
                         candidate(s) were dropped and are COMMITTED, each with its pre-image \
                         in org_name_drops; {} skipped on the re-check; the rest were never \
                         attempted. This is a PARTIAL run — re-run the dry pass to see what \
                         is left, or restore-dropped-satellites with job {} to undo it.",
                        r.dropped, r.candidates, r.skipped_recheck, job_id
                    ));
                }
                Ok(format!(
                    "drop-orphan-satellites WET (issue 321): {} candidate(s) matched the \
                     recorded plan exactly; dropped {}, skipped {} on the in-transaction \
                     re-check. Pre-images in org_name_drops — restore-dropped-satellites \
                     puts every one back. {} org_match_keys row(s) still carry a dropped \
                     key; the next build-org-match-keys clears them.",
                    r.candidates, r.dropped, r.skipped_recheck, r.stale_keys
                ))
            }
            Spec::RestoreDroppedSatellites { dry_run, only_job } => {
                let (dry_run, only_job) = (*dry_run, *only_job);
                let now = store::now_unix();
                let r = self
                    .db
                    .restore_dropped_satellites(dry_run, only_job, now)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(format!(
                    "restore-dropped-satellites {} (issue 321{}): {} outstanding pre-image(s), \
                     {} {}; {} left alone because something has since written that (org, lang), \
                     {} superseded by a newer drop on the same slot, {} whose org no longer \
                     exists. A restore never clobbers newer truth and never lets one \
                     unrestorable row take the rest of the pass with it.",
                    if dry_run { "DRY" } else { "WET" },
                    match only_job {
                        Some(j) => format!(", bounded to drop job {j}"),
                        None => String::new(),
                    },
                    r.outstanding,
                    if dry_run { r.rows.len() as u64 } else { r.restored },
                    if dry_run { "would be restored" } else { "restored" },
                    r.occupied,
                    r.superseded,
                    r.orphaned
                ))
            }
            Spec::XbPacket => {
                let job_id = job.id;
                let stop = || self.cancelled(job_id);
                self.set_phase(
                    "assembling",
                    None,
                    None,
                    "issues 311+314: same-name cross-border review packet".to_owned(),
                );
                let p = self
                    .db
                    .xb_same_name_packet(
                        ingest::project::match_norm,
                        ingest::idgate::checksum_anchors,
                        ingest::idgate::anchor_vocabulary,
                        600,
                        3,
                        &stop,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                if p.stopped {
                    return Ok("xb-packet STOPPED by cancel — no packet stored".to_owned());
                }
                let now = store::now_unix();
                let body = serde_json::json!({
                    "cohort": p.cohort,
                    "truncated": p.truncated,
                    "cases": p.cases.iter().map(|c| serde_json::json!({
                        "root": c.root,
                        "size": c.size,
                        "key": c.key,
                        "countries": c.countries,
                        "members": c.members.iter().map(|m| serde_json::json!({
                            "org": m.org,
                            "country": m.country,
                            "identifier_kind": m.identifier_kind,
                            "identifier": m.identifier,
                            "name": m.name,
                            "variants": m.variants.iter()
                                .map(|(l, n)| serde_json::json!({"lang": l, "name": n}))
                                .collect::<Vec<_>>(),
                            "mentions": m.mentions,
                            "notices": m.notices,
                            "anchors": m.anchors,
                            "country_agrees": m.country_agrees,
                            "country_probed": m.country_probed,
                        })).collect::<Vec<_>>(),
                    })).collect::<Vec<_>>(),
                })
                .to_string();
                self.db
                    .put_report("xb-packet", &body, now)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(format!(
                    "xb-packet (issues 311+314): {} same-name cross-border component(s) in \
                     the cohort, {} carried in this packet{}. Each case lists every \
                     member's row, its language-labelled variants, its mention count and \
                     a few publication ids — the mention SPREAD is the first read: one \
                     heavy row beside a light one is a stray duplicate, two heavy rows are \
                     more likely two real registrations. ANCHOR KEY (issue 314): read \
                     country_agrees only WITH country_probed. probed+agrees = the \
                     arithmetic works where the row says it is; probed+disagrees = tested \
                     under the row's own register and refused, which is the contaminated \
                     country signal; NOT probed = no scheme of that country has this \
                     value's shape, so the silence carries no information.",
                    p.cohort,
                    p.cases.len(),
                    if p.truncated { " (CAPPED)" } else { "" }
                ))
            }
            Spec::RepairMintedCountries { dry_run } => {
                let dry_run = *dry_run;
                let job_id = job.id;
                let stop = || self.cancelled(job_id);
                self.set_phase(
                    if dry_run { "planning" } else { "repairing" },
                    None,
                    None,
                    "issue 325: re-parsing the standing vat rows".to_owned(),
                );
                // The T4 ladder: a wet run executes the plan a person read.
                let expect_rows = if dry_run {
                    None
                } else {
                    let (body, _) = self
                        .db
                        .latest_report("minted-country-repair")
                        .await
                        .map_err(|e| e.to_string())?
                        .ok_or_else(|| {
                            "no stored minted-country-repair plan — run the dry pass first"
                        })?;
                    let v: serde_json::Value =
                        serde_json::from_str(&body).map_err(|e| e.to_string())?;
                    if v["dry_run"] != serde_json::Value::Bool(true) {
                        return Err("the stored minted-country-repair report is from a WET \
                                    run, not a reviewed plan — run the dry pass again"
                            .to_owned());
                    }
                    Some(
                        v["rows"]
                            .as_u64()
                            .ok_or_else(|| "minted-country-repair plan lacks rows")?,
                    )
                };
                let r = self
                    .db
                    .repair_minted_countries(
                        |value, country| {
                            ingest::project::normalise_identifier(value, country)
                                .map(|id| (id.kind, id.country))
                        },
                        dry_run,
                        expect_rows,
                        &stop,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                if r.stopped && dry_run {
                    return Ok("repair-minted-countries STOPPED by cancel — nothing planned"
                        .to_owned());
                }
                let now = store::now_unix();
                // The plan is the review artifact AND the record of what a wet
                // run did, so it is stored either way — capped for the report,
                // counted in full above it.
                const PLAN_CAP: usize = 400;
                let body = serde_json::json!({
                    "dry_run": dry_run,
                    "walked": r.walked,
                    "rows": r.rows,
                    "ambiguous": r.ambiguous,
                    "no_mention_country": r.no_mention_country,
                    "now_refused": r.now_refused,
                    "collisions": r.collisions,
                    "applied": r.applied,
                    "skipped_moved": r.skipped_moved,
                    "stopped": r.stopped,
                    "plan_truncated": r.plan.len() > PLAN_CAP,
                    "plan": r.plan.iter().take(PLAN_CAP).map(|f| serde_json::json!({
                        "org": f.org,
                        "identifier": f.identifier,
                        "from": {"kind": f.from_kind, "country": f.from_country},
                        "to": {"kind": f.to_kind, "country": f.to_country},
                        "mention_country": f.mention_country,
                        "mentions": f.mentions,
                    })).collect::<Vec<_>>(),
                })
                .to_string();
                self.db
                    .put_report("minted-country-repair", &body, now)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(format!(
                    "repair-minted-countries (issue 325 step 4, {}): {} kind='vat' row(s) \
                     re-parsed. {} disagree with what the parser now says and are planned; \
                     {} left alone because their own mentions name more than one country, \
                     {} because no mention names an alpha-2 country at all, and {} because \
                     the value is now refused outright (counted, never stripped — issue \
                     312). {} planned target(s) would leave one (country, kind, identifier) \
                     held by more than one row: that is the R2 merge arm's work and reaching \
                     it is the point, not a blocker.{}",
                    if dry_run { "DRY" } else { "WET" },
                    r.walked,
                    r.rows,
                    r.ambiguous,
                    r.no_mention_country,
                    r.now_refused,
                    r.collisions,
                    if dry_run {
                        String::new()
                    } else {
                        format!(
                            " APPLIED {}, skipped {} whose row moved under the plan.{}",
                            r.applied,
                            r.skipped_moved,
                            if r.stopped { " STOPPED by cancel — the committed prefix stands." } else { "" }
                        )
                    }
                ))
            }
            Spec::RepairLabelPrefixes { dry_run } => {
                let dry_run = *dry_run;
                let job_id = job.id;
                let stop = || self.cancelled(job_id);
                self.set_phase(
                    if dry_run { "planning" } else { "repairing" },
                    None,
                    None,
                    "issue 328: stripping publisher labels off identifiers".to_owned(),
                );
                let expect_rows = if dry_run {
                    None
                } else {
                    let (body, _) = self
                        .db
                        .latest_report("label-prefix-repair")
                        .await
                        .map_err(|e| e.to_string())?
                        .ok_or_else(|| {
                            "no stored label-prefix-repair plan — run the dry pass first"
                        })?;
                    let v: serde_json::Value =
                        serde_json::from_str(&body).map_err(|e| e.to_string())?;
                    if v["dry_run"] != serde_json::Value::Bool(true) {
                        return Err("the stored label-prefix-repair report is from a WET run, \
                                    not a reviewed plan — run the dry pass again"
                            .to_owned());
                    }
                    Some(v["rows"].as_u64().ok_or_else(|| "plan lacks rows")?)
                };
                let r = self
                    .db
                    .repair_label_prefixes(
                        ingest::countries::label_prefix_stripped,
                        |value, country| {
                            ingest::project::normalise_identifier(value, country)
                                .map(|id| (id.kind, id.country, id.value))
                        },
                        dry_run,
                        expect_rows,
                        &stop,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                if r.stopped && dry_run {
                    return Ok("repair-label-prefixes STOPPED by cancel — nothing planned"
                        .to_owned());
                }
                let now = store::now_unix();
                const PLAN_CAP: usize = 400;
                let body = serde_json::json!({
                    "dry_run": dry_run,
                    "labelled": r.labelled,
                    "now_refused": r.now_refused,
                    "already_clean": r.already_clean,
                    "rows": r.rows,
                    "reunions": r.reunions,
                    "applied": r.applied,
                    "skipped_moved": r.skipped_moved,
                    "stopped": r.stopped,
                    "plan_truncated": r.plan.len() > PLAN_CAP,
                    "plan": r.plan.iter().take(PLAN_CAP).map(|f| serde_json::json!({
                        "org": f.org,
                        "from": {"identifier": f.from_identifier, "kind": f.from_kind,
                                 "country": f.from_country},
                        "to": {"identifier": f.to_identifier, "kind": f.to_kind,
                               "country": f.to_country},
                    })).collect::<Vec<_>>(),
                })
                .to_string();
                self.db
                    .put_report("label-prefix-repair", &body, now)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(format!(
                    "repair-label-prefixes (issue 328, {}): {} row(s) carry a publisher label. \
                     {} planned; {} left standing as published because the remainder \
                     classified as nothing (a bare field name, or a value the gate refuses), \
                     and {} already agree with the re-parse. {} planned row(s) land on an \
                     identity that ALREADY stands. NOTE: for the German class that is NOT a \
                     merge R2 will perform — `crosswalk::canonical_key` has no DE arm at all \
                     (\"court-scoped registers\", a pinned negative), so those rows become \
                     visible exact duplicates rather than folded ones.{} The published string \
                     is untouched either way: it stays in \
                     organization_mentions.raw_identifier.",
                    if dry_run { "DRY" } else { "WET" },
                    r.labelled,
                    r.rows,
                    r.now_refused,
                    r.already_clean,
                    r.reunions,
                    if dry_run {
                        String::new()
                    } else {
                        format!(
                            " APPLIED {}, skipped {} whose row moved under the plan.{}",
                            r.applied,
                            r.skipped_moved,
                            if r.stopped { " STOPPED by cancel — the committed prefix stands." } else { "" }
                        )
                    }
                ))
            }
            Spec::RepairCountryTypos { dry_run } => {
                let dry_run = *dry_run;
                let job_id = job.id;
                let stop = || self.cancelled(job_id);
                self.set_phase(
                    if dry_run { "planning" } else { "repairing" },
                    None,
                    None,
                    "issue 326: moving rows a decisive anchor names".to_owned(),
                );
                let expect_rows = if dry_run {
                    None
                } else {
                    let (body, _) = self
                        .db
                        .latest_report("country-typo-repair")
                        .await
                        .map_err(|e| e.to_string())?
                        .ok_or_else(|| {
                            "no stored country-typo-repair plan — run the dry pass first"
                        })?;
                    let v: serde_json::Value =
                        serde_json::from_str(&body).map_err(|e| e.to_string())?;
                    if v["dry_run"] != serde_json::Value::Bool(true) {
                        return Err("the stored country-typo-repair report is from a WET run, \
                                    not a reviewed plan — run the dry pass again"
                            .to_owned());
                    }
                    Some(v["rows"].as_u64().ok_or_else(|| "plan lacks rows")?)
                };
                let r = self
                    .db
                    .repair_country_typos(
                        ingest::idgate::census_anchors,
                        ingest::idgate::census_vocabulary,
                        ingest::countries::one_letter_apart,
                        ingest::countries::is_operational_footprint,
                        dry_run,
                        expect_rows,
                        &stop,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                if r.stopped && dry_run {
                    return Ok("repair-country-typos STOPPED by cancel — nothing planned"
                        .to_owned());
                }
                let now = store::now_unix();
                const PLAN_CAP: usize = 400;
                let body = serde_json::json!({
                    "dry_run": dry_run,
                    "clusters_considered": r.clusters_considered,
                    "decisive": r.decisive,
                    "left_unmoved": r.left_unmoved,
                    "rows": r.rows,
                    "from_asked_and_refused": r.from_asked_and_refused,
                    "from_never_asked": r.from_never_asked,
                    "refused_row_has_standing": r.refused_row_has_standing,
                    "refused_luhn_only": r.refused_luhn_only,
                    "applied": r.applied,
                    "skipped_moved": r.skipped_moved,
                    "stopped": r.stopped,
                    "plan_truncated": r.moves.len() > PLAN_CAP,
                    "plan": r.moves.iter().take(PLAN_CAP).map(|m| serde_json::json!({
                        "identifier": m.identifier,
                        "from": m.from, "to": m.to,
                        "mentions": m.mentions,
                        "codes": m.codes,
                        "names": m.names,
                        "from_asked_and_refused": m.from_asked_and_refused,
                        "by_scheme": m.by_scheme,
                    })).collect::<Vec<_>>(),
                })
                .to_string();
                self.db
                    .put_report("country-typo-repair", &body, now)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(format!(
                    "repair-country-typos (issue 326 step 2, {}): {} cluster(s) walked, {} \
                     decisive (the evidence names exactly one of the cluster's own codes). \
                     {} row(s) planned to move; {} left alone because their code is NOT one \
                     letter from the survivor — they share the identifier and nothing more, \
                     and a checksum elsewhere is no reason to rewrite a published country. \
                     Of the planned moves {} abandon a country that WAS tested and refused \
                     the value, and {} abandon one no scheme covers at this shape — the \
                     second half rests on the survivor's anchor alone and is where the dry \
                     run's false positives were found. REFUSED: {} row(s) carry {} mentions \
                     or more under their own country — real standing, so probably a real \
                     registration and not a slip — and {} cluster(s) are named only by a bare \
                     Luhn, which carries no country information at all.{} \
                     EVERY move lands on an identity the survivor already holds, ON PURPOSE: \
                     run match-org-identifiers --r2 afterwards to fold them.",
                    if dry_run { "DRY" } else { "WET" },
                    r.clusters_considered,
                    r.decisive,
                    r.rows,
                    r.left_unmoved,
                    r.from_asked_and_refused,
                    r.from_never_asked,
                    r.refused_row_has_standing,
                    store::TYPO_MOVE_MENTION_VETO,
                    r.refused_luhn_only,
                    if dry_run {
                        String::new()
                    } else {
                        format!(
                            " APPLIED {}, skipped {} whose row moved under the plan.{}",
                            r.applied,
                            r.skipped_moved,
                            if r.stopped { " STOPPED by cancel — the committed prefix stands." } else { "" }
                        )
                    }
                ))
            }
            Spec::CountryClusterCensus => {
                let job_id = job.id;
                let stop = || self.cancelled(job_id);
                self.set_phase(
                    "walking",
                    None,
                    None,
                    "issue 326: grouping identifiers by country set".to_owned(),
                );
                const CAP: usize = 400;
                let r = self
                    .db
                    .country_cluster_census(
                        // The EVIDENCE probe, not the decision probe: it carries
                        // the census-only BG/LT/SK arms that would have halved
                        // the 8-digit merge path's reach if added to the shared
                        // table (issue 326).
                        ingest::idgate::census_anchors,
                        ingest::idgate::census_vocabulary,
                        ingest::countries::one_letter_apart,
                        ingest::countries::is_operational_footprint,
                        CAP,
                        &stop,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                if r.stopped {
                    return Ok("country-cluster-census STOPPED by cancel — no report stored"
                        .to_owned());
                }
                let now = store::now_unix();
                // The share distribution is reported as quartiles rather than
                // 400 raw bytes: the point is to pick a threshold from it, and
                // a median plus tails is what that needs.
                let q = |p: usize| -> u8 {
                    if r.majority_share.is_empty() {
                        return 0;
                    }
                    let i = (r.majority_share.len() - 1) * p / 100;
                    r.majority_share[i]
                };
                let body = serde_json::json!({
                    "rows_walked": r.rows_walked,
                    "identifiers": r.identifiers,
                    "countries": r.countries,
                    "clusters": r.clusters,
                    "with_one_letter_pair": r.with_one_letter_pair,
                    "with_heavy_one_letter": r.with_heavy_one_letter,
                    "too_short": r.too_short,
                    "footprint": r.footprint,
                    "verdicts": r.verdicts,
                    "majority_share": {
                        "p25": q(25), "p50": q(50), "p75": q(75), "p90": q(90),
                        "n": r.majority_share.len(),
                    },
                    // Which checksum arms would actually pay: the countries
                    // appearing in clusters that fail ONLY for want of a scheme.
                    "nobody_asked_by_country": r.nobody_asked_by_country,
                    // The cut that actually chooses the arms: the heavy side,
                    // where the entity lives. The all-codes cut above mixes in
                    // countries that are only ever the typo TARGET.
                    "nobody_asked_heavy_country": r.nobody_asked_heavy_country,
                    "truncated": r.truncated,
                    "rows": r.rows.iter().map(|c| serde_json::json!({
                        "identifier": c.identifier,
                        "codes": c.codes,
                        "named": c.named,
                        "asked": c.asked,
                        "one_letter_pair": c.one_letter_pair,
                        "heavy_one_letter": c.heavy_one_letter,
                        "footprint": c.footprint,
                        "mentions": c.mentions,
                        "names": c.names,
                        "verdict": c.verdict,
                    })).collect::<Vec<_>>(),
                })
                .to_string();
                self.db
                    .put_report("country-cluster-census", &body, now)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(format!(
                    "country-cluster-census (issue 326 re-cut): {} rows walked, {} distinct \
                     identifiers under {} country codes. {} identifiers stand under MORE THAN \
                     ONE code; {} have some two codes one letter apart, and {} have the \
                     HEAVIEST code one letter from another (the sharper filter — spray one \
                     letter from spray says nothing). {} cluster(s) are an \
                     operational footprint (embassy or development agency: one entity, one \
                     register number, filed from everywhere it operates) and are excluded, \
                     never corrected. {} identifier(s) are under {} characters. Verdicts: {}. \
                     Heaviest code's mention share p25/p50/p75/p90 = {}/{}/{}/{}%.",
                    r.rows_walked,
                    r.identifiers,
                    r.countries,
                    r.clusters,
                    r.with_one_letter_pair,
                    r.with_heavy_one_letter,
                    r.footprint,
                    r.too_short,
                    store::MIN_CLUSTER_IDENTIFIER,
                    r.verdicts
                        .iter()
                        .map(|(k, v)| format!("{k}={v}"))
                        .collect::<Vec<_>>()
                        .join(", "),
                    q(25), q(50), q(75), q(90),
                ))
            }
            Spec::DiskCensus => Box::pin(async move {
                // NOT in STOPPABLE_KINDS on purpose: this is statvfs plus one
                // stat, so there is no loop to read a stop flag and claiming it
                // could be cancelled would be the lie issue 252 removed.
                let Some(disk) = crate::v1::health::disk_usage() else {
                    return Err("could not stat the database filesystem".to_owned());
                };
                let db_path = std::env::var("TENDER_DB").unwrap_or_else(|_| "tender-db.db".into());
                let db_bytes = std::fs::metadata(&db_path).ok().map(|m| m.len());
                let used_pct = disk.used_fraction * 100.0;
                let now = store::now_unix();

                // The previous sample, read through the history issue 335 added.
                // This is the whole point of the job: growth is a reading rather
                // than an inference.
                let previous = self.db.previous_report("disk-census").await.ok().flatten();
                // Three outcomes, kept apart deliberately. Collapsing "no earlier
                // sample" and "the earlier sample is too close" into one branch
                // made the second run report "no comparable earlier sample yet"
                // while holding two — which reads as history failing to
                // accumulate, the opposite of the truth.
                let interval = previous.as_ref().map(|(_, at)| now.saturating_sub(*at));
                let trend = previous.as_ref().and_then(|(body, at)| {
                    let older: serde_json::Value = serde_json::from_str(body).ok()?;
                    let free_then = older.get("free_bytes")?.as_u64()?;
                    let secs = now.saturating_sub(*at);
                    // A sample interval under an hour says nothing about daily
                    // growth; two runs a minute apart would produce a wild rate.
                    if secs < 3_600 {
                        return None;
                    }
                    let days = secs as f64 / 86_400.0;
                    let consumed = free_then as i64 - disk.free_bytes as i64;
                    let per_day = consumed as f64 / days;
                    // Only meaningful while it is actually shrinking; a freed
                    // volume has no days-to-full.
                    let to_full = (per_day > 0.0).then(|| disk.free_bytes as f64 / per_day);
                    Some((days, per_day, to_full))
                });

                let body = serde_json::json!({
                    "total_bytes": disk.total_bytes,
                    "free_bytes": disk.free_bytes,
                    "used_pct": (used_pct * 100.0).round() / 100.0,
                    "wal_bytes": disk.wal_bytes,
                    "db_bytes": db_bytes,
                    "sample_interval_days": trend.map(|(d, _, _)| (d * 100.0).round() / 100.0),
                    "bytes_per_day": trend.map(|(_, r, _)| r.round()),
                    "days_to_full": trend.and_then(|(_, _, f)| f).map(|f| f.round()),
                    // Said in the report, not only in the commit message: this is
                    // a two-point rate between consecutive samples, and issue 169
                    // exists because two points were mistaken for a trend once.
                    "rate_caveat": "two-point rate against the previous sample — not a trend; \
                                    read several versions before concluding",
                })
                .to_string();
                self.db.put_report("disk-census", &body, now).await.map_err(|e| e.to_string())?;

                let gib = |b: u64| format!("{:.1} GiB", b as f64 / 1_073_741_824.0);
                Ok(format!(
                    "disk-census (issue 169): {} of {} used ({:.1}%), {} free. Database file {}; \
                     WAL {}. {} NOTHING IS WRITTEN beyond the report; report history keeps up to \
                     {} versions, so the trend this job exists to provide arrives by accumulating \
                     samples rather than by projecting from one.",
                    gib(disk.total_bytes.saturating_sub(disk.free_bytes)),
                    gib(disk.total_bytes),
                    used_pct,
                    gib(disk.free_bytes),
                    db_bytes.map(gib).unwrap_or_else(|| "unreadable".into()),
                    disk.wal_bytes.map(gib).unwrap_or_else(|| "absent".into()),
                    match trend {
                        Some((days, per_day, to_full)) => format!(
                            "Against the previous sample {:.1} day(s) ago: {}/day, which at that \
                             two-point rate is {} to full — a rate between two samples, NOT a \
                             trend.",
                            days,
                            if per_day >= 0.0 {
                                gib(per_day as u64)
                            } else {
                                format!("-{}", gib((-per_day) as u64))
                            },
                            to_full
                                .map(|f| format!("{f:.0} day(s)"))
                                .unwrap_or_else(|| "no horizon (the volume gained space)".into()),
                        ),
                        // Says WHICH of the two no-rate cases this is, because
                        // "no earlier sample" and "one exists but is minutes old"
                        // call for opposite reactions from a reader.
                        None => match interval {
                            None => "No earlier sample yet — this run establishes the baseline."
                                .to_owned(),
                            Some(secs) => format!(
                                "An earlier sample exists but is only {secs}s old, too close to \
                                 give a daily rate; the weekly cadence is what produces one."
                            ),
                        },
                    },
                    store::REPORT_HISTORY_DEPTH,
                ))
            }).await,
            Spec::NameAttributionProbe => Box::pin(async move {
                // BOXED. `run_spec` is a 62-arm async match, so every arm's
                // locals live in ONE future — and adding this census's arm
                // overflowed the stack of a pre-existing supervisor test
                // (`an_execute_without_an_expected_count_is_refused`, SIGABRT)
                // without touching that test at all. Boxing puts this arm's
                // frame on the heap so the parent future stops growing with it.
                let job_id = job.id;
                let stop = || self.cancelled(job_id);
                self.set_phase(
                    "walking",
                    None,
                    None,
                    "issue 334: asking the notices what they called them".to_owned(),
                );
                let progress = |done: u64, detail: &str| {
                    self.set_phase("walking", Some(done), None, detail.to_owned());
                };
                // The widest 60 keys, 120 carriers apiece. A PROBE: every number
                // below is about that set, and the job line says so rather than
                // leaving a reader to infer a corpus tally.
                const KEYS: usize = 60;
                const PER_KEY: usize = 120;
                let r = self
                    .db
                    .name_attribution_probe(
                        ingest::project::match_norm,
                        SCAN_STOPLIST_CAP,
                        KEYS,
                        PER_KEY,
                        GENERIC_KEY_WINDOW,
                        &stop,
                        &progress,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                if r.stopped {
                    return Ok(
                        "name-attribution-probe STOPPED by cancel — no report stored".to_owned()
                    );
                }
                let now = store::now_unix();
                let body = serde_json::json!({
                    "population": "the widest over-cap n2 keys, sampled — NOT a corpus tally",
                    "keys_walked": r.keys_walked,
                    "keys_examined": r.keys_examined,
                    "per_key_sample": PER_KEY,
                    "sampled": r.sampled,
                    "agrees": r.agrees,
                    "differs": r.differs,
                    "silent": r.silent,
                    "published_keys": r.published_keys,
                    "replaced_keys": r.replaced_keys,
                    "mixed_keys": r.mixed_keys,
                    "no_evidence_keys": r.no_evidence_keys,
                    "rows": r.rows.iter().map(|k| serde_json::json!({
                        "key": k.key,
                        "carriers": k.carriers,
                        "sampled": k.sampled,
                        "agrees": k.agrees,
                        "differs": k.differs,
                        "silent": k.silent,
                        "verdict": k.verdict,
                        "examples": k.examples.iter()
                            .map(|(stored, said)| serde_json::json!({
                                "stored": stored, "a_notice_said": said,
                            }))
                            .collect::<Vec<_>>(),
                    })).collect::<Vec<_>>(),
                })
                .to_string();
                self.db
                    .put_report("name-attribution-probe", &body, now)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(format!(
                    "name-attribution-probe (issue 334): walked {} name key(s) to find the {} \
                     WIDEST over the cap of {}, then sampled up to {} carriers of each and \
                     asked their own notices. THIS IS A PROBE OVER THOSE KEYS, NOT A CORPUS \
                     TALLY — every number here is about that set. {} carrier(s) sampled: {} \
                     agree with at least one of their own mentions, {} have every mention \
                     naming something ELSE, {} have no mentions at all. Per key: {} published \
                     (stored name is what the notices said — no parser change would have \
                     prevented it), {} replaced (no sampled carrier's notices agree, so the \
                     name was put there downstream, which would be ours), {} mixed, {} with no \
                     evidence. Read the `examples` in the report before concluding anything: \
                     the counts say how big, only the values say what happened. NOTHING IS \
                     WRITTEN.",
                    r.keys_walked,
                    r.keys_examined,
                    SCAN_STOPLIST_CAP,
                    PER_KEY,
                    r.sampled,
                    r.agrees,
                    r.differs,
                    r.silent,
                    r.published_keys,
                    r.replaced_keys,
                    r.mixed_keys,
                    r.no_evidence_keys,
                ))
            }).await,
            Spec::GenericStatisticCensus => Box::pin(async move {
                // BOXED. `run_spec` is a 62-arm async match, so every arm's
                // locals live in ONE future — and adding this census's arm
                // overflowed the stack of a pre-existing supervisor test
                // (`an_execute_without_an_expected_count_is_refused`, SIGABRT)
                // without touching that test at all. Boxing puts this arm's
                // frame on the heap so the parent future stops growing with it.
                let job_id = job.id;
                let stop = || self.cancelled(job_id);
                self.set_phase(
                    "walking",
                    None,
                    None,
                    "issue 332: cutting over-cap keys by identity diversity".to_owned(),
                );
                let progress = |done: u64, detail: &str| {
                    self.set_phase("walking", Some(done), None, detail.to_owned());
                };
                const CAP: usize = 200;
                let r = self
                    .db
                    .genericness_statistic_census(
                        SCAN_STOPLIST_CAP,
                        CAP,
                        GENERIC_KEY_WINDOW,
                        &stop,
                        &progress,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                if r.stopped {
                    return Ok(
                        "generic-statistic-census STOPPED by cancel — no report stored".to_owned()
                    );
                }
                let now = store::now_unix();
                // Percentiles, because the question is whether the ratio is
                // BIMODAL — two populations that separate — or one smear with no
                // threshold worth having. A mean would hide exactly that.
                let q = |p: usize| -> u8 {
                    if r.identity_ratio.is_empty() {
                        return 0;
                    }
                    r.identity_ratio[(r.identity_ratio.len() - 1) * p / 100]
                };
                let body = serde_json::json!({
                    "keys_walked": r.keys_walked,
                    "stoplist_cap": SCAN_STOPLIST_CAP,
                    "keys_over_cap": r.keys_over_cap,
                    "over_cap_carriers": r.over_cap_carriers,
                    "single_identity": r.single_identity,
                    "mostly_fragmented": r.mostly_fragmented,
                    "mostly_distinct": r.mostly_distinct,
                    "no_identifiers": r.no_identifiers,
                    "too_little_evidence": r.too_little_evidence,
                    "identity_ratio": {
                        "n": r.identity_ratio.len(),
                        "p10": q(10), "p25": q(25), "p50": q(50),
                        "p75": q(75), "p90": q(90),
                    },
                    "truncated": r.truncated,
                    "rows": r.rows.iter().map(|k| serde_json::json!({
                        "key_kind": k.key_kind,
                        "key": k.key,
                        "carriers": k.carriers,
                        "with_identifier": k.with_identifier,
                        "distinct_identities": k.distinct_identities,
                        "verdict": k.verdict,
                    })).collect::<Vec<_>>(),
                })
                .to_string();
                self.db
                    .put_report("generic-statistic-census", &body, now)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(format!(
                    "generic-statistic-census (issue 332): {} name key(s) walked across n2 and \
                     n3; {} are over the stoplist cap of {}, covering {} carrier rows. Cut by \
                     how many DISTINCT (country, kind, identifier) triples those carriers hold: \
                     {} hold exactly ONE (a single identity fragmented — the wall refusing a \
                     name nobody else uses), {} are mostly-fragmented (distinct identities at \
                     most half the identifier-bearing carriers), {} are mostly-distinct (a \
                     genuinely shared name, the wall working as designed), and {} have no \
                     identifier-bearing carrier at all, and {} have exactly ONE (which makes \
                     `distinct == 1` arithmetically true and evidentially empty) — neither can \
                     be decided this way. \
                     Identity ratio p10/p25/p50/p75/p90 = {}/{}/{}/{}/{}%. THE QUESTION IS \
                     WHETHER THAT IS BIMODAL: two populations that separate would justify a \
                     better statistic, one smear means the carrier count is fine and issue \
                     332 closes on a negative. NOTHING IS WRITTEN.",
                    r.keys_walked,
                    r.keys_over_cap,
                    SCAN_STOPLIST_CAP,
                    r.over_cap_carriers,
                    r.single_identity,
                    r.mostly_fragmented,
                    r.mostly_distinct,
                    r.no_identifiers,
                    r.too_little_evidence,
                    q(10), q(25), q(50), q(75), q(90),
                ))
            }).await,
            Spec::GenericWallCensus => Box::pin(async move {
                // BOXED. `run_spec` is a 62-arm async match, so every arm's
                // locals live in ONE future — and adding this census's arm
                // overflowed the stack of a pre-existing supervisor test
                // (`an_execute_without_an_expected_count_is_refused`, SIGABRT)
                // without touching that test at all. Boxing puts this arm's
                // frame on the heap so the parent future stops growing with it.
                let job_id = job.id;
                let stop = || self.cancelled(job_id);
                self.set_phase(
                    "walking",
                    None,
                    None,
                    "issue 331: collapsing duplicate carriers".to_owned(),
                );
                let progress = |done: u64, detail: &str| {
                    self.set_phase("walking", Some(done), None, detail.to_owned());
                };
                const CAP: usize = 200;
                let r = self
                    .db
                    .generic_wall_inflation_census(
                        ingest::project::match_norm,
                        ingest::crosswalk::n3_key,
                        SCAN_STOPLIST_CAP,
                        CAP,
                        &stop,
                        &progress,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                if r.stopped {
                    return Ok("generic-wall-census STOPPED by cancel — no report stored".to_owned());
                }
                let now = store::now_unix();
                let body = serde_json::json!({
                    "duplicate_groups": r.duplicate_groups,
                    "duplicate_rows": r.duplicate_rows,
                    "stoplist_cap": SCAN_STOPLIST_CAP,
                    "keys_with_savings": r.keys_with_savings,
                    "keys_over_cap": r.keys_over_cap,
                    "keys_falsely_generic": r.keys_falsely_generic,
                    "orgs_affected_upper_bound": r.orgs_affected,
                    "nearest_miss": r.nearest_miss,
                    "truncated": r.truncated,
                    "rows": r.rows.iter().map(|k| serde_json::json!({
                        "key": k.key,
                        "carriers": k.carriers,
                        "savings": k.savings,
                        "collapsed": k.collapsed,
                        "verdict": k.verdict,
                    })).collect::<Vec<_>>(),
                })
                .to_string();
                self.db
                    .put_report("generic-wall-census", &body, now)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(format!(
                    "generic-wall-census (issue 331): {} duplicate identity group(s) covering \
                     {} org rows. {} name key(s) are carried more than once inside some group, \
                     so their carrier count is inflated at all; {} of those are over the \
                     stoplist cap of {} today. FALSELY GENERIC — over the cap now, at or under \
                     it once duplicates collapse: {}. Upper bound on orgs behind them: {}. \
                     Nearest miss among keys that stay generic: {} collapsed carriers against \
                     a cap of {}. A zero here means the mechanism is real and INERT, which is \
                     a complete answer and the same shape issue 327 settled on. NOTHING IS \
                     WRITTEN. Conservative by construction: duplication is proved by a shared \
                     (country, kind, identifier) triple, so fragmentation among rows with no \
                     identifier is counted as genuine commonality and never inflates this.",
                    r.duplicate_groups,
                    r.duplicate_rows,
                    r.keys_with_savings,
                    r.keys_over_cap,
                    SCAN_STOPLIST_CAP,
                    r.keys_falsely_generic,
                    r.orgs_affected,
                    r.nearest_miss,
                    SCAN_STOPLIST_CAP,
                ))
            }).await,
            Spec::NamePollutionCensus => Box::pin(async move {
                // BOXED. `run_spec` is a 62-arm async match, so every arm's
                // locals live in ONE future — and adding this census's arm
                // overflowed the stack of a pre-existing supervisor test
                // (`an_execute_without_an_expected_count_is_refused`, SIGABRT)
                // without touching that test at all. Boxing puts this arm's
                // frame on the heap so the parent future stops growing with it.
                let job_id = job.id;
                let stop = || self.cancelled(job_id);
                self.set_phase(
                    "walking",
                    None,
                    None,
                    "issue 330: reading organization names".to_owned(),
                );
                let progress = |done: u64, detail: &str| {
                    self.set_phase("walking", Some(done), None, detail.to_owned());
                };
                const CAP: usize = 300;
                let r = self
                    .db
                    .name_pollution_census(CAP, &stop, &progress)
                    .await
                    .map_err(|e| e.to_string())?;
                if r.stopped {
                    return Ok("name-pollution-census STOPPED by cancel — no report stored"
                        .to_owned());
                }
                let now = store::now_unix();
                // Percentiles, not 1.1M raw lengths: the point is to pick a
                // "suspiciously long" threshold from the distribution, and a
                // median plus tails is what that needs.
                let q = |p: usize| -> u32 {
                    if r.name_lengths.is_empty() {
                        return 0;
                    }
                    r.name_lengths[(r.name_lengths.len() - 1) * p / 100]
                };
                let body = serde_json::json!({
                    "rows_walked": r.rows_walked,
                    "polluted": r.polluted,
                    "published": r.published,
                    "derived_only": r.derived_only,
                    "no_mentions": r.no_mentions,
                    "polluted_mentions": r.polluted_mentions,
                    "by_country": r.by_country,
                    "name_length": {
                        "p50": q(50), "p90": q(90), "p99": q(99),
                        "max": r.name_lengths.last().copied().unwrap_or(0),
                        "n": r.name_lengths.len(),
                    },
                    "truncated": r.truncated,
                    "rows": r.rows.iter().map(|c| serde_json::json!({
                        "org_id": c.org_id,
                        "country": c.country,
                        "name": c.name,
                        "mentions": c.mentions,
                        "verdict": c.verdict,
                    })).collect::<Vec<_>>(),
                })
                .to_string();
                self.db
                    .put_report("name-pollution-census", &body, now)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(format!(
                    "name-pollution-census (issue 330): {} organization name(s) walked, {} \
                     carry a LINE BREAK, attached to {} mention(s). Of those, {} were \
                     PUBLISHED that way (at least one mention carries the break too — no \
                     parser change would have prevented it), {} are derived-only (every \
                     mention clean, so the break was introduced downstream of the notice, \
                     which would be ours), and {} have no mentions to compare. By country: \
                     {}. Name length p50/p90/p99/max = {}/{}/{}/{} characters. NOTHING IS \
                     WRITTEN: the split between published and derived-only is what decides \
                     whether this is a parser fix or a normalisation policy.",
                    r.rows_walked,
                    r.polluted,
                    r.polluted_mentions,
                    r.published,
                    r.derived_only,
                    r.no_mentions,
                    r.by_country
                        .iter()
                        .map(|(k, v)| format!("{k}={v}"))
                        .collect::<Vec<_>>()
                        .join(", "),
                    q(50), q(90), q(99),
                    r.name_lengths.last().copied().unwrap_or(0),
                ))
            }).await,
            Spec::DuplicateIdentityCensus => Box::pin(async move {
                // BOXED. `run_spec` is a 62-arm async match, so every arm's
                // locals live in ONE future — and adding this census's arm
                // overflowed the stack of a pre-existing supervisor test
                // (`an_execute_without_an_expected_count_is_refused`, SIGABRT)
                // without touching that test at all. Boxing puts this arm's
                // frame on the heap so the parent future stops growing with it.
                let job_id = job.id;
                let stop = || self.cancelled(job_id);
                self.set_phase(
                    "walking",
                    None,
                    None,
                    "issue 328 follow-on: grouping standing identities".to_owned(),
                );
                // The first run sat on a single unchanging phase line for over
                // half an hour, which is indistinguishable from a hang to
                // whoever is watching. The scan-org-match-keys feed, borrowed.
                let progress = |done: u64, detail: &str| {
                    self.set_phase("walking", Some(done), None, detail.to_owned());
                };
                const CAP: usize = 400;
                let r = self
                    .db
                    .duplicate_identity_census(
                        // The DECISION probe, deliberately: the question is what
                        // the merge arms can actually see, so it has to be the
                        // same function they key on — not the census-only
                        // evidence table, which would report groups as reachable
                        // that R2 will never plan.
                        ingest::crosswalk::canonical_key_flat,
                        ingest::crosswalk::n3_key,
                        SCAN_STOPLIST_CAP,
                        CAP,
                        &stop,
                        &progress,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                if r.stopped {
                    return Ok("duplicate-identity-census STOPPED by cancel — no report stored"
                        .to_owned());
                }
                let now = store::now_unix();
                let pct = |n: u64| -> u64 {
                    if r.unkeyed_groups == 0 {
                        0
                    } else {
                        n * 100 / r.unkeyed_groups
                    }
                };
                let disagree = *r.verdicts.get("disagree").unwrap_or(&0);
                let de_vat = |v: &str| -> u64 {
                    *r.verdicts_by_scope.get(&format!("DE:vat/{v}")).unwrap_or(&0)
                };
                let body = serde_json::json!({
                    "rows_walked": r.rows_walked,
                    "triples": r.triples,
                    "duplicate_groups": r.duplicate_groups,
                    "duplicate_rows": r.duplicate_rows,
                    "keyed_groups": r.keyed_groups,
                    "unkeyed_groups": r.unkeyed_groups,
                    // The output that chooses the next cross-walk arm.
                    "unkeyed_by_scope": r.unkeyed_by_scope,
                    "verdicts": r.verdicts,
                    "verdicts_by_scope": r.verdicts_by_scope,
                    // Read `agree-distinctive` with suspicion if this is not 0:
                    // the genericness probe reads org_match_keys, so a stale or
                    // unrun key-build makes every agreeing group look
                    // distinctive.
                    "name_keys_absent": r.name_keys_absent,
                    "group_sizes": {
                        "n": r.group_sizes.len(),
                        "max": r.group_sizes.last().copied().unwrap_or(0),
                        "p50": r.group_sizes
                            .get(r.group_sizes.len().saturating_sub(1) / 2)
                            .copied()
                            .unwrap_or(0),
                    },
                    "truncated": r.truncated,
                    "rows": r.rows.iter().map(|c| serde_json::json!({
                        "scope": c.scope,
                        "identifier": c.identifier,
                        "members": c.members,
                        "mentions": c.mentions,
                        "names": c.names,
                        "name_keys": c.name_keys,
                        "verdict": c.verdict,
                    })).collect::<Vec<_>>(),
                })
                .to_string();
                self.db
                    .put_report("duplicate-identity-census", &body, now)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(format!(
                    "duplicate-identity-census (issue 328 follow-on): {} rows walked, {} \
                     distinct (country, kind, identifier) triples. {} triple(s) are held by \
                     MORE THAN ONE org row, covering {} rows. Of those, {} are keyed by \
                     canonical_key and are already inside the merge arms' field of view; {} \
                     are NOT keyed and no arm can reach them. Unkeyed by scope: {}. Name \
                     verdicts over the unkeyed groups: {} ({}% disagree). DE:vat specifically \
                     — {} agree-distinctive, {} agree-generic, {} contained, {} disagree, {} \
                     unnamed. `contained` is UNDECIDED and NOT a safe bucket: an Organschaft \
                     subsidiary and a branch office are named the same way (parent name plus a \
                     qualifier), so nothing in a name separates them. {} name key(s) were \
                     absent from org_match_keys entirely, which is the number that says \
                     whether the agree/generic split can be trusted at all. NOTHING IS \
                     WRITTEN: this is the measurement the DE cross-walk decision was waiting \
                     on, not the decision.",
                    r.rows_walked,
                    r.triples,
                    r.duplicate_groups,
                    r.duplicate_rows,
                    r.keyed_groups,
                    r.unkeyed_groups,
                    r.unkeyed_by_scope
                        .iter()
                        .map(|(k, v)| format!("{k}={v}"))
                        .collect::<Vec<_>>()
                        .join(", "),
                    r.verdicts
                        .iter()
                        .map(|(k, v)| format!("{k}={v}"))
                        .collect::<Vec<_>>()
                        .join(", "),
                    pct(disagree),
                    de_vat("agree-distinctive"),
                    de_vat("agree-generic"),
                    de_vat("contained"),
                    de_vat("disagree"),
                    de_vat("unnamed"),
                    r.name_keys_absent,
                ))
            }).await,
            Spec::CountryTypoCensus => {
                let job_id = job.id;
                let stop = || self.cancelled(job_id);
                self.set_phase(
                    "measuring",
                    None,
                    None,
                    "issue 326: same identifier, country codes one letter apart".to_owned(),
                );
                let r = self
                    .db
                    .country_typo_census(
                        ingest::idgate::checksum_anchors,
                        ingest::idgate::anchor_vocabulary,
                        400,
                        &stop,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                if r.stopped {
                    return Ok(
                        "country-typo-census STOPPED by cancel — no report stored".to_owned()
                    );
                }
                let now = store::now_unix();
                let body = serde_json::json!({
                    "countries": r.countries,
                    "pairs_considered": r.pairs_considered,
                    "rows_walked": r.rows_walked,
                    "hits": r.hits,
                    "decided": r.decided,
                    "neither_probed": r.neither_probed,
                    "truncated": r.truncated,
                    "rows": r.rows.iter().map(|p| serde_json::json!({
                        "identifier_kind": p.identifier_kind,
                        "identifier": p.identifier,
                        "anchors": p.anchors,
                        "a": {
                            "org": p.org_a, "country": p.country_a, "name": p.name_a,
                            "mentions": p.mentions_a,
                            "agrees": p.a_agrees, "probed": p.a_probed,
                        },
                        "b": {
                            "org": p.org_b, "country": p.country_b, "name": p.name_b,
                            "mentions": p.mentions_b,
                            "agrees": p.b_agrees, "probed": p.b_probed,
                        },
                    })).collect::<Vec<_>>(),
                })
                .to_string();
                self.db
                    .put_report("country-typo-census", &body, now)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(format!(
                    "country-typo-census (issue 326): {} country code(s) hold \
                     identifier-bearing rows, {} pairing(s) are exactly one letter apart \
                     with both sides present, {} row(s) walked on the rarer sides. \
                     HITS: {} pair(s) carry the SAME identifier across a one-letter \
                     country pair. Of those the checksum names a survivor for {}, and \
                     for {} it never asked about EITHER country — so {} of {} need a \
                     discriminator the arithmetic cannot give, and a repair must abstain \
                     there rather than guess. {} row(s) carried in the report{}. \
                     Read-only: nothing was merged, folded or written.",
                    r.countries,
                    r.pairs_considered,
                    r.rows_walked,
                    r.hits,
                    r.decided,
                    r.neither_probed,
                    r.hits.saturating_sub(r.decided),
                    r.hits,
                    r.rows.len(),
                    if r.truncated { " (CAPPED)" } else { "" }
                ))
            }
            Spec::AnchorWallCensus => {
                let job_id = job.id;
                let stop = || self.cancelled(job_id);
                self.set_phase(
                    "measuring",
                    None,
                    None,
                    "issue 318: standing surface where ingest binds and batch refuses".to_owned(),
                );
                let r = self
                    .db
                    .anchor_wall_census(
                        ingest::idgate::checksum_anchors,
                        ingest::idgate::hard_scheme,
                        SCAN_STOPLIST_CAP,
                        200,
                        GENERIC_KEY_WINDOW,
                        &stop,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                if r.stopped {
                    return Ok("anchor-wall-census STOPPED by cancel — no report stored".to_owned());
                }
                let now = store::now_unix();
                let body = serde_json::json!({
                    "keys_walked": r.keys_walked,
                    "generic_keys": r.generic_keys,
                    "generic_orgs": r.generic_orgs,
                    "probed": r.probed,
                    "anchored": r.anchored,
                    "anchored_hard": r.anchored_hard,
                    "anchored_soft": r.anchored_soft,
                    "soft_slots": r.soft_slots,
                    "by_scheme": r.by_scheme.iter()
                        .map(|(s, n)| serde_json::json!({"scheme": s, "orgs": n}))
                        .collect::<Vec<_>>(),
                    "rows": r.rows.iter().map(|o| serde_json::json!({
                        "org": o.org, "name": o.name, "key": o.key,
                        "carriers": o.carriers, "scheme": o.scheme,
                    })).collect::<Vec<_>>(),
                })
                .to_string();
                self.db
                    .put_report("anchor-wall-census", &body, now)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(format!(
                    "anchor-wall-census (issue 318): {} n2 key group(s) walked, {} over the \
                     stoplist cap, holding {} carrier slot(s) of which {} were probed. \
                     DISTINCT orgs anchoring to exactly one scheme: {} — {} HARD (both paths \
                     agree, the design's exemption) and {} SOFT. That {} is the STANDING \
                     SURFACE where the resolver would bind and the batch arm would refuse: \
                     distinct rows reachable ({} (org, generic-name) slots), not binds \
                     observed.{}",
                    r.keys_walked,
                    r.generic_keys,
                    r.generic_orgs,
                    r.probed,
                    r.anchored,
                    r.anchored_hard,
                    r.anchored_soft,
                    r.anchored_soft,
                    r.soft_slots,
                    match r.by_scheme.first() {
                        Some((s, n)) => format!(" Widest scheme: {s} at {n}."),
                        None => String::new(),
                    }
                ))
            }
            Spec::SatelliteOrphans => {
                let job_id = job.id;
                let stop = || self.cancelled(job_id);
                self.set_phase(
                    "measuring",
                    None,
                    None,
                    "reading name variants left on re-homing origins".to_owned(),
                );
                let r = self
                    .db
                    .satellite_orphans(ingest::project::match_norm, 200, &stop)
                    .await
                    .map_err(|e| e.to_string())?;
                if r.stopped {
                    return Ok("satellite-orphans STOPPED by cancel — no report stored".to_owned());
                }
                let now = store::now_unix();
                let row = |o: &store::OrphanSatellite| {
                    serde_json::json!({
                        "org": o.org, "org_name": o.org_name, "lang": o.lang,
                        "name": o.name, "key": o.key,
                        "target": o.target, "target_name": o.target_name,
                    })
                };
                let body = serde_json::json!({
                    "origins": r.origins, "standing": r.standing,
                    "variants": r.variants, "orphans": r.orphans,
                    "orphans_at_target": r.orphans_at_target,
                    "origins_with_orphans": r.origins_with_orphans,
                    "truncated": r.truncated,
                    "rows": r.rows.iter().map(&row).collect::<Vec<_>>(),
                })
                .to_string();
                self.db
                    .put_report("satellite-orphans", &body, now)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(format!(
                    "satellite-orphans (issue 321): {} re-homing origins, {} still standing, \
                     holding {} name variants; {} are supported by NO remaining mention \
                     ({} of those already stand on a row this origin re-homed to), over {} \
                     origins{}",
                    r.origins,
                    r.standing,
                    r.variants,
                    r.orphans,
                    r.orphans_at_target,
                    r.origins_with_orphans,
                    if r.truncated { "; LIST TRUNCATED at the cap" } else { "" }
                ))
            }
            Spec::CaseReviewBacklog => {
                let job_id = job.id;
                let stop = || self.cancelled(job_id);
                self.set_phase(
                    "listing",
                    None,
                    None,
                    "walking the parked review verdicts".to_owned(),
                );
                // 200 per list. The first prod run measured the real backlog —
                // 20 escalations and 111 medium-band cases, not the 17/102 the
                // campaign log remembered — and clipped the mediums at 60. A
                // cap that truncates the thing the job exists to show is a cap
                // set to the wrong number.
                let r = self.db.case_review_backlog(200, &stop).await.map_err(|e| e.to_string())?;
                if r.stopped {
                    return Ok("case-review-backlog STOPPED by cancel — no report stored".to_owned());
                }
                let row = |c: &store::CaseBacklogRow| {
                    serde_json::json!({
                        "org": c.org, "cohort": c.cohort, "verdict": c.verdict,
                        "confidence": c.confidence, "diagnosis": c.diagnosis,
                        "handling": c.handling, "name": c.name,
                        "identifier_kind": c.identifier_kind, "identifier": c.identifier,
                        "gone": c.gone,
                        "peer_rows": c.peer_rows, "peer_example": c.peer_example,
                    })
                };
                let now = store::now_unix();
                let body = serde_json::json!({
                    "total": r.total, "applied": r.applied, "unapplied": r.unapplied,
                    "by_verdict": r.by_verdict.iter().map(|(v, c, n)| {
                        serde_json::json!({ "verdict": v, "confidence": c, "cases": n })
                    }).collect::<Vec<_>>(),
                    "escalations": r.escalations.iter().map(row).collect::<Vec<_>>(),
                    "medium_band": r.medium_band.iter().map(row).collect::<Vec<_>>(),
                    "truncated": r.truncated,
                })
                .to_string();
                self.db
                    .put_report("case-escalations", &body, now)
                    .await
                    .map_err(|e| e.to_string())?;
                // The corroborated count is the actionable half: a medium
                // verdict whose identifier ALSO stands on another org row has
                // the structural evidence it was missing (issue 317 Unit C).
                let corroborated =
                    r.medium_band.iter().filter(|c| c.peer_rows > 0).count();
                Ok(format!(
                    "case-review-backlog (issue 317): {} verdicts, {} applied, {} parked; \
                     {} escalations and {} medium-band cases listed ({} of the mediums \
                     have a peer row carrying the same identifier){}",
                    r.total,
                    r.applied,
                    r.unapplied,
                    r.escalations.len(),
                    r.medium_band.len(),
                    corroborated,
                    if r.truncated { "; LIST TRUNCATED at the cap" } else { "" }
                ))
            }
            Spec::OrgEdgeCensus => {
                let job_id = job.id;
                let stop = || self.cancelled(job_id);
                self.set_phase("censusing", None, None, "walking the candidate-edge store".to_owned());
                // Sample every 500th component, capped at 40 in-store.
                let r = self
                    .db
                    .census_org_candidate_edges(500, ingest::project::match_norm, &stop)
                    .await
                    .map_err(|e| e.to_string())?;
                if r.stopped {
                    return Ok("org-edge-census STOPPED by cancel — no report stored".to_owned());
                }
                let now = store::now_unix();
                let body = serde_json::json!({
                    "edges": r.edges, "e3_name": r.e3_name, "e3_xlang": r.e3_xlang,
                    "orgs_touched": r.orgs_touched, "dangling_orgs": r.dangling_orgs,
                    "components": r.components,
                    "size_buckets": r.size_buckets.iter().map(|(k, v)| {
                        serde_json::json!({ "size": k, "components": v })
                    }).collect::<Vec<_>>(),
                    "max_component": r.max_component,
                    "canonical_only_components": r.canonical_only_components,
                    "mixed_components": r.mixed_components,
                    "provisional_only_components": r.provisional_only_components,
                    "multi_country_components": r.multi_country_components,
                    "null_country_components": r.null_country_components,
                    "canonical_cross_border_components": r.canonical_cross_border_components,
                    "xb_same_name": r.xb_same_name,
                    "xb_diff_name": r.xb_diff_name,
                    "xb_with_intra": r.xb_with_intra,
                    "xb_class_sample": r.xb_class_sample.iter()
                        .map(|(c, root, size, ms)| serde_json::json!({
                            "class": c, "root": root, "size": size, "members": ms,
                        }))
                        .collect::<Vec<_>>(),
                    "canonical_cross_border_pairs": r.canonical_cross_border_pairs.iter().map(|(k, v)| {
                        serde_json::json!({ "pair": k, "components": v })
                    }).collect::<Vec<_>>(),
                    "canonical_cross_border_sample": r.canonical_cross_border_sample.iter()
                        .map(|(root, size, members)| {
                            serde_json::json!({ "root": root, "size": size, "members": members })
                        }).collect::<Vec<_>>(),
                    "country_pairs": r.country_pairs.iter().map(|(k, v)| {
                        serde_json::json!({ "pair": k, "components": v })
                    }).collect::<Vec<_>>(),
                    "sample": r.sample.iter().map(|(root, size, members)| {
                        serde_json::json!({ "root": root, "size": size, "members": members })
                    }).collect::<Vec<_>>(),
                })
                .to_string();
                self.db.put_report("org-edge-census", &body, now).await.map_err(|e| e.to_string())?;
                Ok(format!(
                    "org-edge-census (issue 314): {} edges ({} e3-name + {} e3-xlang) over \
                     {} orgs ({} dangling); {} components, max {}; {} canonical-only, \
                     {} mixed, {} provisional-only; {} span >1 KNOWN country, \
                     {} hold a country-less member; COHORT (canonical-only AND \
                     cross-border): {}",
                    r.edges,
                    r.e3_name,
                    r.e3_xlang,
                    r.orgs_touched,
                    r.dangling_orgs,
                    r.components,
                    r.max_component,
                    r.canonical_only_components,
                    r.mixed_components,
                    r.provisional_only_components,
                    r.multi_country_components,
                    r.null_country_components,
                    r.canonical_cross_border_components
                ))
            }
            Spec::ApplyCaseReviews { dry_run } => {
                let dry_run = *dry_run;
                self.set_phase(
                    if dry_run { "planning" } else { "applying" },
                    None,
                    None,
                    "walking unapplied case-review verdicts".to_owned(),
                );
                let r = self
                    .db
                    .apply_case_reviews(dry_run, Some(job.id as i64), store::now_unix())
                    .await
                    .map_err(|e| e.to_string())?;
                if dry_run {
                    // The CONCRETE strip list, recorded for review before
                    // any wet run (panel catch: counts alone cannot surface
                    // a hallucinated org id in a verdict batch).
                    let now = store::now_unix();
                    let plan = serde_json::json!({
                        "pending": r.pending, "eligible": r.eligible,
                        "would_strip": r.stripped, "noop": r.noop,
                        "strips": r.plan.iter().map(|(id, name, ident)| {
                            serde_json::json!({ "org_id": id, "name": name, "identifier": ident })
                        }).collect::<Vec<_>>(),
                    })
                    .to_string();
                    self.db
                        .put_report("case-apply-plan", &plan, now)
                        .await
                        .map_err(|e| e.to_string())?;
                }
                Ok(format!(
                    "apply-case-reviews (issue 311){}: {} pending verdicts, {} eligible \
                     (wrong-identifier + high confidence); {} identifiers stripped, {} no-ops",
                    if dry_run { " DRY RUN — plan recorded, nothing written" } else { "" },
                    r.pending,
                    r.eligible,
                    r.stripped,
                    r.noop
                ))
            }
            Spec::UnapplyCaseReviews { dry_run } => {
                let dry_run = *dry_run;
                self.set_phase(
                    if dry_run { "planning" } else { "restoring" },
                    None,
                    None,
                    "walking applied strips for platform-GUID pre-images".to_owned(),
                );
                let r = self
                    .db
                    .unapply_case_reviews(
                        ingest::idgate::uuid_v4,
                        dry_run,
                        Some(job.id as i64),
                        store::now_unix(),
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                if dry_run {
                    // The CONCRETE restore list, reviewable before the
                    // reversal runs — the same bar the strip plan had.
                    let now = store::now_unix();
                    let plan = serde_json::json!({
                        "applied": r.applied, "selected": r.selected,
                        "would_restore": r.restored, "noop": r.noop,
                        "restores": r.plan.iter().map(|(id, name, ident)| {
                            serde_json::json!({ "org_id": id, "name": name, "identifier": ident })
                        }).collect::<Vec<_>>(),
                    })
                    .to_string();
                    self.db
                        .put_report("case-unapply-plan", &plan, now)
                        .await
                        .map_err(|e| e.to_string())?;
                }
                Ok(format!(
                    "unapply-case-reviews (issue 312){}: {} applied verdicts examined, \
                     {} strips with a platform-GUID pre-image; {} identifiers restored, \
                     {} no-ops (org gone or an identifier written since)",
                    if dry_run { " DRY RUN — plan recorded, nothing written" } else { "" },
                    r.applied,
                    r.selected,
                    r.restored,
                    r.noop
                ))
            }
            Spec::BuildOrgMatchKeys { dry_run } => {
                let dry_run = *dry_run;
                // ~1,260 windows over 12.6M orgs; ~1.7 keys/org keeps a
                // window's rows inside the sorted bulk-load band, and the
                // stop flag reads every few seconds of work.
                const BATCH: i64 = 10_000;
                let started = std::time::Instant::now();
                let mut after = 0i64;
                if !dry_run {
                    let (wm, epoch) =
                        self.db.org_match_keys_state().await.map_err(|e| e.to_string())?;
                    if wm > 0 && epoch == ingest::crosswalk::NAME_KEY_EPOCH {
                        // Crash-resume: the committed windows stand; the walk
                        // continues past them. (A crashed FINISH self-heals
                        // too: the first batch finds nothing and the index
                        // re-create is IF NOT EXISTS.)
                        eprintln!("[build-org-match-keys] resuming past id {wm}");
                        after = wm;
                    } else {
                        if wm > 0 {
                            eprintln!(
                                "[build-org-match-keys] semantics epoch changed \
                                 ({epoch:?} -> {:?}): restarting from zero",
                                ingest::crosswalk::NAME_KEY_EPOCH
                            );
                        }
                        self.db
                            .reset_org_match_keys(ingest::crosswalk::NAME_KEY_EPOCH)
                            .await
                            .map_err(|e| e.to_string())?;
                    }
                }
                let mut totals = store::MatchKeyBuildWindow::default();
                let mut windows = 0u64;
                loop {
                    if self.cancelled(job.id) {
                        // A stopped WET build leaves the watermark standing —
                        // the next run resumes; no report is recorded (the
                        // honest state for an unfinished build).
                        return Ok(format!(
                            "build-org-match-keys stopped by cancel after {windows} windows \
                             (watermark stands at {after}; a re-run resumes)"
                        ));
                    }
                    let (w, next) = self
                        .db
                        .build_org_match_keys_batch(
                            ingest::project::match_norm,
                            ingest::crosswalk::n3_key,
                            BATCH,
                            after,
                            dry_run,
                        )
                        .await
                        .map_err(|e| e.to_string())?;
                    if w.orgs == 0 {
                        break;
                    }
                    windows += 1;
                    totals.orgs += w.orgs;
                    totals.names_read += w.names_read;
                    totals.n2_rows += w.n2_rows;
                    totals.n3_rows += w.n3_rows;
                    totals.empty_skipped += w.empty_skipped;
                    totals.rows_written += w.rows_written;
                    after = next;
                    self.set_phase(
                        if dry_run { "measuring" } else { "walking" },
                        Some(totals.orgs),
                        None,
                        format!("id ..= {after}, {} rows so far", totals.rows_written),
                    );
                }
                let now = store::now_unix();
                if dry_run {
                    let plan = serde_json::json!({
                        "orgs": totals.orgs, "names": totals.names_read,
                        "n2": totals.n2_rows, "n3": totals.n3_rows,
                        "empty_skipped": totals.empty_skipped,
                        "projected_rows": totals.n2_rows + totals.n3_rows,
                        "windows": windows,
                        "elapsed_seconds": started.elapsed().as_secs(),
                    })
                    .to_string();
                    self.db
                        .put_report("org-match-keys-plan", &plan, now)
                        .await
                        .map_err(|e| e.to_string())?;
                    return Ok(format!(
                        "build-org-match-keys DRY RUN — STORED NOTHING: {} orgs, {} names, \
                         {} n2 + {} n3 keys projected ({} empty skipped) over {windows} \
                         windows in {}s",
                        totals.orgs,
                        totals.names_read,
                        totals.n2_rows,
                        totals.n3_rows,
                        totals.empty_skipped,
                        started.elapsed().as_secs()
                    ));
                }
                self.set_phase("indexing", None, None, "covering index + analyze".to_owned());
                let rows = self.db.finish_org_match_keys().await.map_err(|e| e.to_string())?;
                let report = serde_json::json!({
                    "rows": rows, "n2": totals.n2_rows, "n3": totals.n3_rows,
                    "orgs": totals.orgs, "names": totals.names_read,
                    "empty_skipped": totals.empty_skipped, "windows": windows,
                    "index_built": true, "built_at": now,
                    "epoch": ingest::crosswalk::NAME_KEY_EPOCH,
                    "elapsed_seconds": started.elapsed().as_secs(),
                })
                .to_string();
                self.db
                    .put_report("org-match-keys-build", &report, now)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(format!(
                    "build-org-match-keys (issue 300 Stage 4): {rows} rows stand indexed \
                     ({} n2 + {} n3 from {} orgs / {} names, {} empty skipped) \
                     over {windows} windows in {}s",
                    totals.n2_rows,
                    totals.n3_rows,
                    totals.orgs,
                    totals.names_read,
                    totals.empty_skipped,
                    started.elapsed().as_secs()
                ))
            }
            Spec::ScanOrgMatchKeys { dry_run, max_edges } => {
                let dry_run = *dry_run;
                let started = std::time::Instant::now();
                // Preconditions, refuse-with-remedy (the has_index precheck
                // precedent): the covering index, a FINISHED build, and a
                // keys epoch matching this binary's key fns — scanning old-
                // epoch keys with new fns would mislabel rules silently.
                if !self.db.has_index("org_match_keys_kk").await.map_err(|e| e.to_string())? {
                    return self
                        .edge_scan_refuse(
                            "scan-org-match-keys refused: no org_match_keys_kk index — \
                             run build-org-match-keys (wet) first"
                                .to_owned(),
                        )
                        .await;
                }
                let (wm, epoch) =
                    self.db.org_match_keys_state().await.map_err(|e| e.to_string())?;
                if wm != 0 {
                    return self
                        .edge_scan_refuse(format!(
                            "scan-org-match-keys refused: keys build in flight \
                             (watermark {wm}) — let it finish or rerun build-org-match-keys"
                        ))
                        .await;
                }
                if epoch != ingest::crosswalk::NAME_KEY_EPOCH {
                    return self
                        .edge_scan_refuse(format!(
                            "scan-org-match-keys refused: keys built under epoch {epoch:?}, \
                             this binary keys under {:?} — rerun build-org-match-keys (wet) \
                             first",
                            ingest::crosswalk::NAME_KEY_EPOCH
                        ))
                        .await;
                }
                let Some((build_body, _)) = self
                    .db
                    .latest_report("org-match-keys-build")
                    .await
                    .map_err(|e| e.to_string())?
                else {
                    return self
                        .edge_scan_refuse(
                            "scan-org-match-keys refused: no org-match-keys-build report — \
                             run build-org-match-keys (wet) first"
                                .to_owned(),
                        )
                        .await;
                };
                let build: serde_json::Value =
                    serde_json::from_str(&build_body).map_err(|e| e.to_string())?;
                let Some(built_at) = build["built_at"].as_i64() else {
                    return self
                        .edge_scan_refuse("org-match-keys-build lacks built_at".to_owned())
                        .await;
                };
                // T4 ladder: a wet run REQUIRES the census recorded against
                // the SAME keys build, in bounds; parity itself is checked
                // whole-plan in-store, before any write.
                let expect_edges = if dry_run {
                    None
                } else {
                    let Some((body, _)) = self
                        .db
                        .latest_report("org-edge-scan-plan")
                        .await
                        .map_err(|e| e.to_string())?
                    else {
                        return self
                            .edge_scan_refuse(
                                "no stored org-edge-scan-plan — run the dry scan first"
                                    .to_owned(),
                            )
                            .await;
                    };
                    let v: serde_json::Value =
                        serde_json::from_str(&body).map_err(|e| e.to_string())?;
                    if v["keys_built_at"].as_i64() != Some(built_at) {
                        return self
                            .edge_scan_refuse(
                                "org-edge-scan-plan predates the current keys build — \
                                 rerun the dry scan and review it"
                                    .to_owned(),
                            )
                            .await;
                    }
                    if v["bounds_ok"].as_bool() != Some(true) {
                        return self
                            .edge_scan_refuse(
                                "org-edge-scan-plan bounds_ok is false — an out-of-bounds \
                                 census means the semantics are wrong; re-plan, there is \
                                 no override"
                                    .to_owned(),
                            )
                            .await;
                    }
                    match v["would_emit"].as_u64() {
                        Some(n) => Some(n),
                        None => {
                            return self
                                .edge_scan_refuse(
                                    "org-edge-scan-plan lacks would_emit".to_owned(),
                                )
                                .await;
                        }
                    }
                };
                let exemplar = self
                    .db
                    .chase_merged_org(SCAN_EXEMPLAR_ORG)
                    .await
                    .map_err(|e| e.to_string())?;
                let phase = if dry_run { "censusing" } else { "emitting" };
                self.set_phase(phase, None, None, "walking the key index".to_owned());
                let job_id = job.id;
                let stop = || self.cancelled(job_id);
                let progress = |done: u64, detail: &str| {
                    self.set_phase(phase, Some(done), None, detail.to_owned());
                };
                let r = match self
                    .db
                    .scan_org_match_keys(
                        store::OrgEdgeScanArgs {
                            n2: ingest::project::match_norm,
                            n3: ingest::crosswalk::n3_key,
                            stoplist_cap: SCAN_STOPLIST_CAP,
                            key_window: store::SCAN_KEY_WINDOW,
                            max_edges: *max_edges,
                            expect_edges,
                            exemplar_org: Some(exemplar),
                            dry_run,
                            job_id: Some(job.id as i64),
                            stop: &stop,
                            progress: &progress,
                        },
                        store::now_unix(),
                    )
                    .await
                {
                    Ok(r) => r,
                    // The in-store aborts — T4 parity, the bounds ceiling —
                    // and any plain store error land on the alarm surface
                    // too (panel round 2): a parity abort on the Sunday
                    // tick was the ops amendment's LEAD scenario for the
                    // silently-stopped clock, and it must never show a
                    // stale "clear" while the weekly run stands refused.
                    Err(e) => return self.edge_scan_refuse(e.to_string()).await,
                };
                if r.stopped {
                    // Honest cancel: NO report (the issue-230 zero-lie bar).
                    // Wet keeps any committed edge batches — an idempotent
                    // refresh a re-run heals.
                    return Ok(format!(
                        "scan-org-match-keys STOPPED by cancel: census had {} edges, \
                         {} written before the stop; NO report was recorded",
                        r.would_emit, r.edges_written
                    ));
                }
                let now = store::now_unix();
                let census = serde_json::json!({
                    "keys_walked": r.keys_walked,
                    "groups_ge2": r.groups_ge2,
                    "groups_emitting": r.groups_emitting,
                    "provisional_only_groups": r.provisional_only_groups,
                    "stoplist_skipped_n2": r.stoplist_skipped_n2,
                    "stoplist_skipped_n3": r.stoplist_skipped_n3,
                    "stoplist_top": r.stoplist_top,
                    "dangling_members": r.dangling_members,
                    "stale_pairs": r.stale_pairs,
                    "pairs_considered": r.pairs_considered,
                    "would_emit": r.would_emit,
                    "e3_name": r.e3_name,
                    "e3_xlang": r.e3_xlang,
                    "max_group": r.max_group,
                    "bounds_ok": r.bounds_ok,
                    "ceiling_truncated": r.ceiling_truncated,
                    "keys_built_at": built_at,
                    "keys_build_epoch": ingest::crosswalk::NAME_KEY_EPOCH,
                    "exemplar": {
                        "org": r.exemplar_org,
                        "edges": r.exemplar_edges,
                        "rules": r.exemplar_rules,
                        "peers": r.exemplar_peers,
                        "states": r.exemplar_states,
                    },
                    // The hand-review material: real edges with their
                    // evidence, from the report alone (the edge table is
                    // outside the public SQL surface).
                    "sample": r.sample.iter().map(|(a, b, rule, ev)| {
                        serde_json::json!({
                            "org_a": a, "org_b": b, "rule": rule,
                            "evidence": serde_json::from_str::<serde_json::Value>(ev)
                                .unwrap_or_else(|_| serde_json::Value::String(ev.clone())),
                        })
                    }).collect::<Vec<_>>(),
                    "capped": r.capped,
                    "elapsed_seconds": started.elapsed().as_secs(),
                });
                if dry_run {
                    self.db
                        .put_report("org-edge-scan-plan", &census.to_string(), now)
                        .await
                        .map_err(|e| e.to_string())?;
                    return Ok(format!(
                        "scan-org-match-keys DRY RUN — STORED NOTHING: {} keys walked, \
                         {} groups >=2, {} emitting; would emit {} edges ({} e3-name + \
                         {} e3-xlang); stoplist skipped {} n2 + {} n3 (max group {}); \
                         {} provisional-only groups, {} pairs, {} dangling, {} stale; \
                         exemplar org {} has {} edge(s); bounds_ok={} in {}s",
                        r.keys_walked,
                        r.groups_ge2,
                        r.groups_emitting,
                        r.would_emit,
                        r.e3_name,
                        r.e3_xlang,
                        r.stoplist_skipped_n2,
                        r.stoplist_skipped_n3,
                        r.max_group,
                        r.provisional_only_groups,
                        r.pairs_considered,
                        r.dangling_members,
                        r.stale_pairs,
                        exemplar,
                        r.exemplar_edges,
                        r.bounds_ok,
                        started.elapsed().as_secs()
                    ));
                }
                let mut wet = census.clone();
                wet["edges_written"] = r.edges_written.into();
                wet["edges_new"] = r.edges_new.into();
                wet["edges_refreshed"] = r.edges_refreshed.into();
                wet["total_edges_after"] = r.total_edges_after.into();
                let alarm_line = self.apply_edge_tripwire(&r, expect_edges, &mut wet, now).await?;
                self.db
                    .put_report("org-edge-scan", &wet.to_string(), now)
                    .await
                    .map_err(|e| e.to_string())?;
                // Re-anchor the plan after a COMPLETED, UNCAPPED wet run
                // (the R2 residual-plan lesson, mandatory amendment):
                // without this, the weekly wet drifts from one aging dry
                // plan until every scheduled run parity-aborts — a silently
                // stopped tripwire. A capped run leaves the reviewed plan
                // standing: the rollout ladder still points at it.
                if !r.capped {
                    self.db
                        .put_report("org-edge-scan-plan", &census.to_string(), now)
                        .await
                        .map_err(|e| e.to_string())?;
                }
                Ok(format!(
                    "scan-org-match-keys (issue 300 Stage 4){}: {} keys walked, \
                     {} groups >=2, {} emitting; {} edges written ({} new, \
                     {} refreshed) of a {} census ({} e3-name + {} e3-xlang); \
                     stoplist skipped {} n2 + {} n3; exemplar org {} has {} edge(s); \
                     {} edges stand, in {}s{alarm_line}",
                    if r.capped { " CAPPED" } else { "" },
                    r.keys_walked,
                    r.groups_ge2,
                    r.groups_emitting,
                    r.edges_written,
                    r.edges_new,
                    r.edges_refreshed,
                    r.would_emit,
                    r.e3_name,
                    r.e3_xlang,
                    r.stoplist_skipped_n2,
                    r.stoplist_skipped_n3,
                    exemplar,
                    r.exemplar_edges,
                    r.total_edges_after,
                    started.elapsed().as_secs()
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
            Spec::RefoldSections { kinds } => {
                let refs: Vec<&str> = kinds.iter().map(String::as_str).collect();
                let carriers =
                    self.db.notice_ids_with_section_kind(&refs).await.map_err(|e| e.to_string())?;
                if carriers.is_empty() {
                    // Not an error: a kind no notice carries is a legitimate answer, and
                    // saying so beats re-queueing nothing while reporting success.
                    return Ok(format!("no parsed notice carries a {:?} section", kinds));
                }
                self.set_phase("re-folding", None, None, format!("{} carriers", carriers.len()));
                let requeued =
                    self.db.unmark_projected_by_ids(&carriers).await.map_err(|e| e.to_string())?;
                // The issue-179 pair: the requeue alone leaves each chain identical and an
                // unchanged chain with a current epoch early-returns, so the mapping fix
                // would never land.
                let stamped =
                    self.db.stamp_stale_for_notices(&carriers).await.map_err(|e| e.to_string())?;
                Ok(format!(
                    "{} notice(s) carry a {:?} section: re-queued {requeued}, stamped {stamped} \
                     tender(s) epoch-stale for the incremental fold",
                    carriers.len(),
                    kinds
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
            Spec::FetchRates => {
                // The ZIP, not the bare CSV: the bare
                // `eurofxref-hist.csv` URL serves a frozen defective artifact
                // (2010-02-12 rates plus one garbage row the CDN has pinned for
                // sixteen years) while the zip at the same path carries the
                // real live series. Issue 306.
                const URL: &str = "https://www.ecb.europa.eu/stats/eurofxref/eurofxref-hist.zip";
                let period = store::rates::civil_date(store::now_unix());
                let target = fetch::Target {
                    source: "ecb",
                    kind: "rates",
                    period: period.clone(),
                    url: URL.to_owned(),
                    rel_path: format!("rates/eurofxref-hist-{period}.zip"),
                };
                // refetch=true: a same-day re-run re-downloads and lands as
                // Unchanged when the content hash matches — the registry and the
                // archived file are the durable record either way (ADR-0004).
                let outcome = fetch::fetch(&self.db, &self.http, &self.archive, &target, true)
                    .await
                    .map_err(|e| format!("rates fetch: {e:?}"))?;
                let row = self
                    .db
                    .latest_fetch("ecb", "rates", &period)
                    .await
                    .map_err(|e| e.to_string())?
                    .ok_or_else(|| format!("rates fetch {outcome:?} but no registry row"))?;
                let csv = package::zip_single_text(&self.archive.join(&row.path))
                    .map_err(|e| format!("read archived rates zip {}: {e}", row.path))?;
                let rows = store::rates::parse_ecb_history_csv(&csv);
                if rows.is_empty() {
                    return Err(format!(
                        "rates csv parsed to ZERO rows ({} bytes) — format drift? nothing written",
                        row.bytes
                    ));
                }
                // The issue-306 tripwire: a live daily series whose newest row
                // is stale means the SOURCE is defective — refuse before any
                // write, so this failure mode is a red job, not silent NULLs.
                store::rates::assert_fresh(&rows, &period, 10)?;
                let seeded =
                    self.db.seed_irrevocable_euro_rates().await.map_err(|e| e.to_string())?;
                let total = rows.len();
                let mut upserted = 0u64;
                for (i, chunk) in rows.chunks(50_000).enumerate() {
                    upserted +=
                        self.db.upsert_currency_rates(chunk).await.map_err(|e| e.to_string())?;
                    self.set_phase(
                        "loading",
                        Some(upserted),
                        Some(total as u64),
                        format!("chunk {} upserted", i + 1),
                    );
                    if let Err(e) = self.db.checkpoint(store::CheckpointMode::Truncate).await {
                        eprintln!("supervisor: checkpoint after rates chunk: {e}");
                    }
                }
                // REPLACE can only overwrite, never remove: a poisoned stored
                // row on a date the real file doesn't have (the garbage Sunday
                // 2010-02-14 row) would survive every re-fetch. The file is the
                // authority for its own source — delete what it disowns.
                let removed = self
                    .db
                    .reconcile_currency_dates("ecb", &rows)
                    .await
                    .map_err(|e| e.to_string())?;
                // The running process folds with an in-memory snapshot — refresh
                // it so the NEXT projection uses what was just loaded.
                let cached = self.db.reload_rates_lookup().await.map_err(|e| e.to_string())?;
                Ok(format!(
                    "rates: {upserted} daily rows upserted from {period} ({:?}, {} bytes) + \
                     {seeded} irrevocable conversion rates seeded; {removed} stale rows \
                     reconciled away; {cached} rows cached",
                    outcome, row.bytes
                ))
            }
            Spec::FetchRatesEcu => {
                // Eurostat splits the official daily ECU series in two: the
                // former euro-area national currencies (DEM/FRF/ITL/… — the
                // ones pre-1999 tenders actually publish in) live in
                // `ert_h_eur_d`, everything else (GBP/DKK/USD/SEK/…) in
                // `ert_bil_eur_d`. Both verified 2026-08-27: daily back to
                // 1974, OBS_VALUE = national units per 1 ECU (DEM closes
                // 1998-12-31 on the irrevocable 1.95583 exactly), CC BY 4.0,
                // no key. `endPeriod` caps at 1998-12-31 in the URL, and the
                // loader re-filters below, because from 1999 the ECB series is
                // the authority and the two must not overlap.
                const BASE: &str = "https://ec.europa.eu/eurostat/api/dissemination/sdmx/2.1/data";
                const RANGE: &str = "format=SDMX-CSV&startPeriod=1993-01-01&endPeriod=1998-12-31";
                let datasets =
                    [("ert_h_eur_d", "rates-ecu-h"), ("ert_bil_eur_d", "rates-ecu-bil")];
                let mut upserted = 0u64;
                let mut summary = Vec::new();
                let mut all_rows = Vec::new();
                for (dataset, kind) in datasets {
                    let target = fetch::Target {
                        source: "eurostat",
                        kind,
                        period: "1993-1998".to_owned(),
                        url: format!("{BASE}/{dataset}?{RANGE}"),
                        rel_path: format!("rates/ecu-{dataset}-1993-1998.csv"),
                    };
                    let outcome = fetch::fetch(&self.db, &self.http, &self.archive, &target, true)
                        .await
                        .map_err(|e| format!("{dataset} fetch: {e:?}"))?;
                    let row = self
                        .db
                        .latest_fetch("eurostat", kind, "1993-1998")
                        .await
                        .map_err(|e| e.to_string())?
                        .ok_or_else(|| format!("{dataset} fetch {outcome:?} but no registry row"))?;
                    let csv = std::fs::read_to_string(self.archive.join(&row.path))
                        .map_err(|e| format!("read archived {dataset} csv {}: {e}", row.path))?;
                    let rows: Vec<_> = store::rates::parse_eurostat_sdmx_csv(&csv)
                        .into_iter()
                        .filter(|(_, date, _, _)| date.as_str() < "1999-01-01")
                        .collect();
                    if rows.is_empty() {
                        return Err(format!(
                            "{dataset} parsed to ZERO rows ({} bytes) — format drift? nothing \
                             written",
                            row.bytes
                        ));
                    }
                    // The issue-306 guard, closed-series form: this series ENDS
                    // 1998-12-31 (the euro replaced the ECU), so freshness means
                    // coverage reaches December 1998, not today. A file stopping
                    // earlier is truncated/defective — refuse before writing.
                    let newest = store::rates::newest_date(&rows).unwrap_or("").to_owned();
                    if newest.as_str() < "1998-12-01" {
                        return Err(format!(
                            "{dataset} coverage ends {newest} — the closed ECU series must \
                             reach 1998-12; truncated or defective file, nothing written \
                             (issue 306)"
                        ));
                    }
                    let total = rows.len();
                    for (i, chunk) in rows.chunks(50_000).enumerate() {
                        upserted +=
                            self.db.upsert_currency_rates(chunk).await.map_err(|e| e.to_string())?;
                        self.set_phase(
                            "loading",
                            Some(upserted),
                            None,
                            format!("{dataset} chunk {} upserted", i + 1),
                        );
                        if let Err(e) = self.db.checkpoint(store::CheckpointMode::Truncate).await {
                            eprintln!("supervisor: checkpoint after ecu rates chunk: {e}");
                        }
                    }
                    summary.push(format!("{dataset}: {total} rows ({:?}, {} bytes)", outcome, row.bytes));
                    all_rows.extend(rows);
                }
                // Reconcile against the UNION of both datasets — they share the
                // 'eurostat-ecu' source tag, so either file alone would disown
                // the other's dates (issue 306's REPLACE-can't-delete lesson).
                let removed = self
                    .db
                    .reconcile_currency_dates("eurostat-ecu", &all_rows)
                    .await
                    .map_err(|e| e.to_string())?;
                let cached = self.db.reload_rates_lookup().await.map_err(|e| e.to_string())?;
                Ok(format!(
                    "ecu rates 1993-1998: {upserted} rows upserted — {}; {removed} stale rows \
                     reconciled away; {cached} rows cached",
                    summary.join("; ")
                ))
            }
            Spec::GhostCensus => Box::pin(async move {
                // BOXED. `run_spec` is a 62-arm async match, so every arm's locals
                // live in ONE future; a census arm added to it once overflowed the
                // stack of an unrelated supervisor test. Boxing keeps this arm's
                // frame on the heap.
                //
                // This job kind used to be the track-2 SWEEP, and it stalled prod
                // for 40+ minutes on 2026-08-26 running an unbounded
                // `GROUP BY … COUNT(DISTINCT)` over ~12.4M `tender_versions`. It was
                // then a deliberate no-op for a week. What replaces it is a
                // read-only census over the same signature, sliced on the GROUP BY
                // key itself — 0.30 s per million notice ids on prod, ~9 s for the
                // whole space. It is a DETECTOR, not a cleanup: the ~45k ghosts it
                // was built to remove are gone (measured 2026-09-02, zero across all
                // 29.96M ids), and nothing was watching for the signature returning.
                let job_id = job.id;
                let stop = || self.cancelled(job_id);
                self.set_phase("walking", None, None, "issue 278: ghost signature".to_owned());
                let progress = |done: u64, detail: &str| {
                    self.set_phase("walking", Some(done), None, detail.to_owned());
                };
                // A slice per million notice ids. Wide enough that the whole space
                // is ~30 statements, narrow enough that each is far under the
                // /v1/sql-class 10 s bound that the unbounded form blew past.
                const GHOST_WINDOW: i64 = 1_000_000;
                const CAP: usize = 200;
                let r = self
                    .db
                    .ghost_census(GHOST_WINDOW, CAP, &stop, &progress)
                    .await
                    .map_err(|e| e.to_string())?;
                if r.stopped {
                    return Ok("ghost-census STOPPED by cancel — no report stored".to_owned());
                }
                let now = store::now_unix();
                let surplus = r.ghost_tender_refs.saturating_sub(r.ghost_notices);
                let body = serde_json::json!({
                    "window": r.window,
                    "max_notice_id": r.max_notice_id,
                    "slices": r.slices,
                    "ghost_notices": r.ghost_notices,
                    "ghost_tender_refs": r.ghost_tender_refs,
                    "surplus_tenders": surplus,
                    "sample_truncated": r.truncated,
                    "sample": r.sample.iter().map(|g| serde_json::json!({
                        "notice_id": g.notice_id,
                        "tenders": g.tenders,
                    })).collect::<Vec<_>>(),
                })
                .to_string();
                self.db.put_report("ghost-census", &body, now).await.map_err(|e| e.to_string())?;
                Ok(format!(
                    "ghost-census (issue 278): {} notice(s) claimed by 2+ Tenders across \
                     {} slice(s) of {} notice ids, up to {}. Claims total {}, so the surplus \
                     (ghost) Tender count is {}. A notice keys to exactly ONE group — \
                     `plan_notice.notice_id` is a PK — so any non-zero here is a defect, not \
                     a tolerance. NOTHING IS WRITTEN: this replaced the sweep that stalled \
                     the queue for 40+ minutes, and it counts rather than retires because \
                     there has been nothing to retire since the reparse backlog drained. If \
                     it ever returns non-zero, the retirement path already exists on the \
                     incremental fold (`retire_regrouped_tenders`) — re-queue the named \
                     notices and let an ordinary project run drop the stale member.",
                    r.ghost_notices,
                    r.slices,
                    r.window,
                    r.max_notice_id,
                    r.ghost_tender_refs,
                    surplus,
                ))
            }).await,
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
            Spec::MergeProvisionalOrgs { dry_run } => {
                // The scan is one ordered pass over `organizations_name_country`;
                // without that index (deferred, issues 62/111 — `reindex` builds it)
                // every batch would sort the whole org table instead. Refuse with
                // the remedy rather than grind.
                let indexed = self
                    .db
                    .has_index("organizations_name_country")
                    .await
                    .map_err(|e| e.to_string())?;
                if !indexed {
                    return Err("merge-provisional-orgs needs the organizations_name_country \
                                index — run a `reindex` job first"
                        .into());
                }
                let total = self.db.org_merge_scope_count().await.map_err(|e| e.to_string())? as u64;
                let mut totals = store::OrgMergeBatch::default();
                let mut scanned = 0u64;
                let mut cursor = String::new();
                let mut stopped = false;
                loop {
                    // A stop is honoured between batches: each batch is its own
                    // committed transaction and merged groups leave the scan's
                    // scope, so a restart from `''` redoes nothing (issue 252's
                    // bar: the flag must be READ, and the log must say so).
                    if self.cancelled(job.id) {
                        stopped = true;
                        break;
                    }
                    let b = self
                        .db
                        .merge_provisional_organizations_batch(ORG_MERGE_BATCH, &cursor, *dry_run)
                        .await
                        .map_err(|e| e.to_string())?;
                    scanned += b.scanned;
                    totals.groups += b.groups;
                    totals.removed += b.removed;
                    totals.mentions += b.mentions;
                    totals.parties += b.parties;
                    totals.bid_parties += b.bid_parties;
                    totals.winners += b.winners;
                    totals.winner_dups += b.winner_dups;
                    totals.tender_changes += b.tender_changes;
                    cursor = b.cursor.clone();
                    self.update(|p| p.members_done = totals.removed);
                    self.set_phase(
                        "merging",
                        Some(scanned),
                        Some(total),
                        format!(
                            "cursor \"{}\"; {} group(s) collapsed, {} org(s) removed",
                            cursor, totals.groups, totals.removed
                        ),
                    );
                    if !dry_run {
                        if let Err(e) = self.db.checkpoint(store::CheckpointMode::Truncate).await {
                            eprintln!("supervisor: checkpoint after org-merge batch: {e}");
                        }
                    }
                    if b.done {
                        break;
                    }
                }
                let cancelled = if stopped { "CANCELLED at a checkpoint — " } else { "" };
                let mode = if *dry_run { "org-merge dry run: would collapse" } else { "org merge: collapsed" };
                Ok(format!(
                    "{cancelled}{mode} {} duplicate group(s): {} provisional org(s) removed; \
                     {} mention(s), {} party row(s), {} bid-party row(s), {} winner row(s) \
                     repointed, {} duplicate winner row(s) dropped, {} tender change event(s)",
                    totals.groups,
                    totals.removed,
                    totals.mentions,
                    totals.parties,
                    totals.bid_parties,
                    totals.winners,
                    totals.winner_dups,
                    totals.tender_changes
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
    async fn run_data_quality(&self, job_id: u64, confirmed: bool) -> Result<String, String> {
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
        let whole = data_quality::whole_corpus_queries();
        if !confirmed {
            let unwindowed = data_quality::unwindowed_labels();
            let caveat = if unwindowed.is_empty() {
                "; every label is windowed or whole-corpus".to_owned()
            } else {
                format!("; unmeasured, no windowing: {}", unwindowed.join(","))
            };
            // The whole-corpus labels are named, not just counted: a dry run exists to
            // show what WOULD run, and "3 more statements" says nothing about which.
            let once = if whole.is_empty() {
                String::new()
            } else {
                format!(
                    ", plus {} whole-corpus statement(s) run once ({})",
                    whole.len(),
                    whole.iter().map(|(l, _)| l.as_str()).collect::<Vec<_>>().join(","),
                )
            };
            // Screamed, not mentioned (issue 272): queued as an acceptance read,
            // this line sat in the recent-jobs list looking like a pass while
            // having stored nothing.
            return Ok(format!(
                "DRY RUN — STORED NOTHING (enqueue with {{\"dry_run\": false}} to measure): \
                 would measure {} window(s) of {DQ_WINDOW} ids up to \
                 tender_id {max_id}, {} queries each ({} statements){once}{caveat}",
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
        // The whole-corpus queries (issue 246) run once each after the windows, so the
        // progress denominator counts them too — a phase that reads 480/480 with work
        // still to do is the kind of small lie issue 65 added phases to avoid.
        let units = (windows.len() * queries.len() + whole.len()) as u64;
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
        // A stop request is honoured between queries (issue 252). This is the longest job
        // in the system and each query is an independent read holding no transaction, so
        // stopping here costs nothing — while before this the flag `cancel` set was read
        // by nobody and the operator was told otherwise.
        let mut stopped = false;
        for (wi, (lo, hi)) in windows.iter().enumerate() {
            let window_started = Instant::now();
            for query in &queries {
                if self.cancelled(job_id) {
                    stopped = true;
                    break;
                }
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
            if stopped {
                break;
            }
        }

        // A partial measurement is NOT stored. Half the windows would render as a report
        // whose numbers look like a whole corpus, and a stale-but-complete report beats a
        // fresh-looking wrong one — the same reasoning as the empty-layer guard below.
        if stopped {
            let elapsed = run_started.elapsed().as_secs_f64();
            eprintln!("[data-quality] cancelled after {done}/{units} units, {elapsed:.0}s");
            return Ok(format!(
                "data quality: CANCELLED after {done} of {units} measurement(s) in {elapsed:.0}s \
                 — nothing stored, the previous report stands"
            ));
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
        // Whole-corpus queries (issue 246): a population no `tender_id` window can
        // slice — the quarantine ledger has no tender — so they run ONCE here rather
        // than per window. A failure records the label as unmeasured, exactly as a
        // failed windowed query does: "the query did not run" and "nothing arrived"
        // must stay different claims (issue 230).
        for (label, sql) in whole {
            self.set_phase("measuring", Some(done), Some(units), format!("whole-corpus {label}"));
            match self.db.measure_rows(&sql).await {
                Ok(rows) => {
                    results.push((label, Some(rows.into_iter().map(json_row).collect())));
                }
                Err(e) => {
                    eprintln!("[data-quality] whole-corpus query {label} failed: {e}");
                    results.push((label, None));
                }
            }
            done += 1;
        }
        for label in data_quality::unwindowed_labels() {
            results.push((label, None));
        }

        let raw = Raw::from_labelled(results).map_err(|e| e.to_string())?;
        let report = data_quality::assemble("(in-process)", &raw);

        // Issue 109: the content-staleness alarm compares THIS run's per-era
        // factless rates to the PREVIOUS run's, stored as their own tiny report
        // kind — `reports` keeps one row per kind, and reading it before the
        // overwrite is exactly the one-run lookback the step change needs. A
        // fired alarm leads the report body (an operator reading anything reads
        // the top) and rides the job summary; the rates are stored only on the
        // path that stores the report, so a cancelled run compares against the
        // last COMPLETE one, never a partial.
        let previous: Vec<(String, u64, u64)> =
            match self.db.latest_report("data-quality-presence").await {
                Ok(Some((body, _))) => serde_json::from_str(&body).unwrap_or_default(),
                _ => Vec::new(),
            };
        let alarms = data_quality::presence_step_changes(&previous, &report.presence);
        let mut body = data_quality::render_text(&report);
        if !alarms.is_empty() {
            body = format!(
                "!! CONTENT-STALENESS STEP CHANGE (issue 109) !!\n{}\n\n{body}",
                alarms.join("\n")
            );
        }
        let now = store::now_unix();
        self.db.put_report("data-quality", &body, now).await.map_err(|e| e.to_string())?;
        let rates: Vec<(String, u64, u64)> = report
            .presence
            .iter()
            .map(|r| (r.profile.clone(), r.versions, r.factless))
            .collect();
        match serde_json::to_string(&rates) {
            Ok(json) => {
                if let Err(e) = self.db.put_report("data-quality-presence", &json, now).await {
                    eprintln!("[data-quality] store presence rates: {e}");
                }
            }
            Err(e) => eprintln!("[data-quality] encode presence rates: {e}"),
        }
        // The headline history (issue 265): every run appends its per-era
        // headline rates, bounded to the last HEADLINE_HISTORY_KEEP runs — one
        // reports row, read by the dashboard's delta table and the /metrics
        // gauges (issue 266). Best-effort like the presence rates: losing a
        // trend point must never fail the measurement that produced it.
        let existing = match self.db.latest_report("data-quality-headlines").await {
            Ok(Some((body, _))) => body,
            _ => "[]".to_owned(),
        };
        let history = data_quality::append_headline_history(
            &existing,
            data_quality::headline_history_entry(&report, now),
        );
        if let Err(e) = self.db.put_report("data-quality-headlines", &history, now).await {
            eprintln!("[data-quality] store headline history: {e}");
        }
        let alarm_note = if alarms.is_empty() {
            String::new()
        } else {
            format!("; {} CONTENT-STALENESS ALARM(S) — read the report", alarms.len())
        };

        // The summary is the digest; the body is in `reports` for whoever reads it.
        // Unmeasured labels are named in the summary, not just in the body — the job
        // log is what an operator sees first.
        let took = run_started.elapsed().as_secs_f64();
        if broken.is_empty() {
            Ok(format!(
                "data quality measured: {} eras over {} windows in {took:.0}s; {} label(s) \
                 unmeasured ({}){alarm_note}",
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
        cap: Option<usize>,
        start_after: Option<i64>,
        resume_after: Option<&str>,
    ) -> Result<String, String> {
        // Real progress wins over the requested floor: a job that has already walked
        // packages must not be sent back to its start by its own parameters.
        let after = resume_after
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or_else(|| start_after.unwrap_or(0));
        let refs: Vec<&str> = profiles.iter().map(String::as_str).collect();
        let mut packages =
            self.db.reparse_packages(&refs, after).await.map_err(|e| e.to_string())?;
        // The cap applies to THIS run, after the resume cursor: a capped run is a
        // prefix, and running the same request again continues from where it stopped
        // (issue 244). Truncating here rather than in the query keeps `packages_total`
        // honest about what this run will do.
        let held_back = cap.map(|n| packages.len().saturating_sub(n)).unwrap_or(0);
        if let Some(n) = cap {
            packages.truncate(n);
        }
        self.update(|p| p.packages_total = packages.len() as u64);
        if packages.is_empty() {
            return Ok(format!("no packages hold parsed notices of {}", refs.join(",")));
        }

        let (mut reparsed, mut unmatched, mut failing, mut members) = (0u64, 0u64, 0u64, 0u64);
        // Issue 247: how far the run got, and whether it stopped because it was asked to.
        let (mut stopped, mut packages_done) = (false, 0usize);
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
                // A stop request is honoured between notices (issue 247), so a long
                // package need not finish before an operator can reclaim the queue —
                // the gap that made an unstoppable job an operational problem.
                || self.cancelled(job_id),
            )
            .await
            .map_err(|e| format!("db: {e}"))?;
            reparsed += report.reparsed;
            unmatched += report.unmatched;
            failing += report.now_failing;
            members += report.members;
            if report.cancelled {
                stopped = true;
                packages_done = i;
                break;
            }
            packages_done = i + 1;
            self.update(|p| p.packages_done = (i + 1) as u64);
            if let Err(e) = self.db.record_job_progress(job_id as i64, &fetch_id.to_string()).await {
                eprintln!("supervisor: job {job_id} record reparse progress {fetch_id}: {e}");
            }
            if let Err(e) = self.db.checkpoint(store::CheckpointMode::Truncate).await {
                eprintln!("supervisor: job {job_id} checkpoint after fetch {fetch_id}: {e}");
            }
        }

        // Without this stamp the whole job reports success and changes nothing
        // (issue 100, point (b)). `reparse_notice` sets `projected = 0`, so the
        // notice does enter the incremental change-set — but the fold EARLY-RETURNS
        // on a Tender whose chain is unchanged and whose epoch is current, and a
        // re-parse changes neither: same notices, same order, same chain. The new
        // parse rows would sit there while every reader kept seeing the old fold.
        //
        // The alternative on the table was a global `PROJECTION_EPOCH` bump, which
        // issue 99 measures as "declares 7.9M tenders stale to fix one era" — hours,
        // to land a 31-notice cohort. Stamping just this cohort's tenders is the same
        // trick the `refold` jobs already use for the same reason (issues 85/99/179),
        // and it is why a targeted re-parse no longer implies a full rebuild.
        //
        // Scoped by PROFILE rather than by the ids actually re-parsed: a superset,
        // deliberately, because a stale stamp only forces a rewrite that recomputes
        // identical content, while a missed one silently loses the re-parse. Gated on
        // `reparsed > 0` so a no-op walk does not age a whole profile for nothing.
        let stamped = if reparsed > 0 {
            self.db.stamp_stale_for_profiles(&refs).await.map_err(|e| e.to_string())?
        } else {
            0
        };

        // `now_failing` is the line to read first on any future run: it counts
        // notices the CURRENT parser can no longer parse, whose stored layer was
        // therefore left alone. Non-zero means a parser regression, not progress.
        // A capped run must say what it did NOT do, in the job log where an operator
        // reads it: a summary that looks complete after touching 1 of 215 packages is
        // how a staged pass gets mistaken for the whole era (issue 244).
        // The continuation cursor an operator can actually USE: resume rides the
        // job ROW (deleted at completion), so "run again" was a lie for a fresh
        // enqueue — it starts at its requested floor and re-walks the prefix
        // (issue 272: job 358 redid 303k notices this way). The honest message
        // hands over the exact `after` for the next enqueue instead.
        let next_after = if packages_done == 0 {
            after
        } else {
            packages.get(packages_done - 1).map(|(id, ..)| *id).unwrap_or(after)
        };
        let remaining = if stopped {
            format!(
                "; STOPPED at an operator's request after {packages_done} of {} package(s) — \
                 continue with a fresh enqueue carrying {{\"after\": {next_after}}}",
                packages.len()
            )
        } else if held_back > 0 {
            format!(
                "; {held_back} package(s) held back by the cap — continue with a fresh enqueue \
                 carrying {{\"after\": {next_after}}} (re-enqueueing the original params \
                 restarts at their floor)"
            )
        } else {
            String::new()
        };
        Ok(format!(
            "re-parsed {reparsed} notices across {} packages ({members} members walked, \
             {unmatched} unmatched, {failing} now failing and left untouched); \
             stamped {stamped} tender(s) epoch-stale{remaining}",
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

    /// Run today's daily pipeline if its tick has already passed unserved (issue 245).
    ///
    /// Reads the job log through the reader pool and the queue that `recover` has
    /// already rebuilt, so it sees both "a probe succeeded this morning" and "the
    /// tick's probe is still pending after a restart" and stays quiet for either.
    async fn catch_up_missed_tick(&self) {
        let now = store::now_unix();
        let day = now.div_euclid(86_400);
        let tick = berlin_tick_on(day, 9, 35);
        let runs = match self.db.recent_job_runs(CATCH_UP_SCAN).await {
            Ok(runs) => runs,
            // No job log, no evidence either way. Enqueuing a whole daily pipeline on a
            // guess is worse than leaving the tick to the loop, so say so and move on.
            Err(e) => {
                eprintln!("supervisor: startup catch-up: read job log: {e}");
                return;
            }
        };
        let probe_queued =
            self.queue.lock().expect("queue lock").iter().any(|j| j.kind == "probe");
        if !tick_needs_catch_up(tick, now, &runs, probe_queued) {
            return;
        }
        let weekday = weekday_of(day);
        let weekday = weekday != 0 && weekday != 6;
        println!(
            "[scheduler] the 09:35 Berlin tick passed unserved {}s ago — running today's daily now (issue 245)",
            now - tick
        );
        self.enqueue_daily(weekday).await;
    }

    pub fn spawn_scheduler(self: Arc<Self>) {
        tokio::spawn(async move {
            // Startup catch-up (issue 245). The tick is an in-process timer, so it is
            // lost whenever the process is not running at 09:35 — a box that was down,
            // and far more often here, a deploy that restarted the service inside the
            // morning window. `enqueue_daily`'s jobs are durable rows and re-run on
            // their own, but issue 222's re-probe loop lives only in this task and dies
            // with it silently: no log line, no job record, and the day is missed until
            // tomorrow's walk-forward. Deploys land at whatever minute the work
            // finishes, so treat a missed tick as ordinary and serve it on startup
            // instead of waiting a day.
            self.catch_up_missed_tick().await;

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
    /// Measured end to end on prod, which is the only number worth writing down
    /// here: the eleven-query pass took **2366 s (39 min) over 32 windows with zero
    /// failed windows**, and the seven-query pass before it took 1258 s. A 03:10
    /// Berlin start therefore finishes around 03:50, some five hours clear of the
    /// 09:35 daily.
    ///
    /// The first two windows of that run cost 291 s and 229 s, and projecting from
    /// them gave "2–3 hours" — wrong by a factor of four, because the mid-corpus id
    /// ranges are sparse and run in 17–27 s. That is the third time on this job that
    /// a partial sample has mispredicted the whole, in both directions. The per-query
    /// cost breakdown each run logs is the thing to read instead.
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
                    self.run_report_tick().await;
                }

                // Step past this tick so the next computation lands on tomorrow.
                tokio::time::sleep(std::time::Duration::from_secs(61)).await;
            }
        });
    }

    /// Everything the weekly pre-dawn tick enqueues, as a callable unit.
    ///
    /// Extracted from the scheduler loop so a TEST can prove the branch
    /// enqueues what it claims (issue 313): the loop itself sleeps until a
    /// wall-clock Sunday, so for as long as this body lived inside it, the
    /// only proof that tripwire 6's weekly clock was wired at all would have
    /// been waiting a week and seeing whether it fired.
    async fn run_report_tick(&self) {
        {
            {
                    // Never stack two: if last week's run is still waiting behind
                    // something long, a second one would double a 36-minute job for
                    // one report that gets overwritten anyway.
                    if self.already_pending("data-quality") {
                        eprintln!("[schedule] data-quality already queued or running, skipping this week");
                    } else {
                        // Issue 169: cheapest possible, and FIRST — statvfs plus one stat,
                        // so it records the week's storage stamp without waiting behind an
                        // hour of key-building. Its value is the series, and a series with a
                        // missed week is worth less than one without.
                        self.push("disk-census", "disk-census (weekly)".into(), Spec::DiskCensus).await;
                        // Issue 278: the standing check that no notice has come to
                        // key to two Tenders again. ~9 s read-only at prod scale, so
                        // it rides beside the disk stamp ahead of the long jobs. The
                        // track-1 fix closes the one KNOWN mechanism; this is what
                        // would catch a second one, which is the part that was
                        // missing while the ~45k sat on prod unnoticed for weeks.
                        self.push("ghost-census", "ghost-census (weekly)".into(), Spec::GhostCensus).await;
                        self.push(
                            "data-quality",
                            "data-quality (weekly)".into(),
                            Spec::DataQuality { confirmed: true },
                        )
                        .await;
                    }
                    // The D4 immutability probe rides the same pre-dawn Sunday tick,
                    // queued BEHIND the measurement (the queue serializes them; the
                    // probe is network-bound and touches the DB only through the
                    // registry). Weekly ×8 cycles today's registry in about a year —
                    // re-fetchability drift is a slow question, and the original
                    // hashes it compares against die with the DB, so the cadence
                    // matters more than the batch size (issue 173 / dr-premise §6).
                    if self.already_pending("rehash-probe") {
                        eprintln!("[schedule] rehash-probe already queued or running, skipping this week");
                    } else {
                        self.push(
                            "rehash-probe",
                            "rehash probe (weekly, 8 package(s))".into(),
                            Spec::RehashProbe { samples: 8 },
                        )
                        .await;
                    }
                    // D5 (reveal recheck) used to ride this weekly tick too; since
                    // the issue-274 slicing it rides the DAILY chain instead — a
                    // weekly 100k-section slice stretched one cohort walk to ~3
                    // weeks (found mis-cadenced 2026-08-25; see enqueue_daily).

                    // Tripwire 6's clock (issue 300 Stage 4 Unit 5): the weekly
                    // WET edge scan refreshes last_seen, runs the monotone check,
                    // and re-anchors the parity plan. Queued BEHIND data-quality
                    // and the rehash probe (the queue serializes them); measured
                    // full wet on prod 2026-08-30: 64 s census+writes over a
                    // 1,498,485-edge census — noise against this chain's 39-min
                    // budget, so the 09:35 margin argument above is untouched.
                    // A refusal here (missing index after a bare rebuild, keys-
                    // epoch drift after a deploy, stale plan) writes
                    // org-edge-scan-alarm and shouts to the journal — a silently
                    // stopped tripwire is the muted-probe failure.
                    //
                    // Issue 315: the key satellite is built WHOLESALE and never
                    // maintained incrementally, so every org minted since the
                    // last build is invisible to the scan — no keys, no groups,
                    // no edges. A week of drift is order 1e3-1e4 orgs, and they
                    // are exactly the rows a matcher cares about most (this
                    // week's freshly minted provisionals). The wet build
                    // measured 85 s on prod against a 13M-row satellite, which
                    // is noise inside this chain's 39-minute budget, so it now
                    // rides AHEAD of the scan (the queue is FIFO, so pushing it
                    // first is the ordering) rather than waiting for an
                    // operator to notice staleness nothing reports.
                    //
                    // The failure mode is contained and LOUD, which is why this
                    // is affordable: a build that dies mid-walk leaves a
                    // non-zero watermark and no covering index, and both the
                    // scan (org-edge-scan-alarm) and a wet r3 (issue 316's
                    // guard) then refuse and say so, instead of running against
                    // a half-built keyspace.
                    if self.already_pending("build-org-match-keys") {
                        eprintln!(
                            "[schedule] build-org-match-keys already queued or running, \
                             skipping this week"
                        );
                    } else {
                        self.push(
                            "build-org-match-keys",
                            format!(
                                "build-org-match-keys epoch={} (weekly)",
                                ingest::crosswalk::NAME_KEY_EPOCH
                            ),
                            Spec::BuildOrgMatchKeys { dry_run: false },
                        )
                        .await;
                    }
                    // Issue 300 Stage 0 built org-merge-health as the BASELINE
                    // and nothing has run it since; its stored report on prod
                    // was from before the 317 re-homing campaign moved 416
                    // mentions and before 321 dropped 66 satellites. An
                    // operator surface shows report stamps, so it read as a
                    // gauge of the current org layer while describing a
                    // superseded one — the issue-161/191 shape, a confident
                    // plausible permanently-stale number.
                    //
                    // 43 s measured on prod (job history, 2026-08-29), which
                    // is noise inside this chain's 39-minute budget, and it is
                    // read-only. It rides LAST: the merge health of a layer is
                    // worth measuring after the week's build and scan have
                    // run, not before.
                    if self.already_pending("org-merge-health") {
                        eprintln!(
                            "[schedule] org-merge-health already queued or running, \
                             skipping this week"
                        );
                    } else {
                        self.push(
                            "org-merge-health",
                            "org-merge-health (weekly)".into(),
                            Spec::OrgMergeHealth,
                        )
                        .await;
                    }
                    if self.already_pending("scan-org-match-keys") {
                        eprintln!(
                            "[schedule] scan-org-match-keys already queued or running, \
                             skipping this week"
                        );
                    } else {
                        self.push(
                            "scan-org-match-keys",
                            format!(
                                "scan-org-match-keys stoplist={} epoch={} (weekly)",
                                SCAN_STOPLIST_CAP,
                                ingest::crosswalk::NAME_KEY_EPOCH
                            ),
                            Spec::ScanOrgMatchKeys { dry_run: false, max_edges: None },
                        )
                        .await;
                    }
            }
        }
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
        // Refresh the ECB reference rates BEFORE the fold (ADR-0014): the
        // derivation's daily window is 7 days, so without a standing refresh
        // every fold more than a week after the last manual fetch-rates run
        // would silently derive NULL eur_cents for non-EUR amounts (found
        // 2026-08-27 — the table had only ever been loaded by hand). Riding
        // the daily chain keeps the newest rate at most one business day
        // behind a version's publication date. Queued-guard like the reveal
        // recheck: never stack two, the table is REPLACE-idempotent anyway.
        if self.already_pending("fetch-rates") {
            eprintln!("[schedule] fetch-rates already queued or running, skipping today");
        } else {
            ids.push(self.push("fetch-rates", "ecb eurofxref-hist (daily)".into(), Spec::FetchRates).await);
        }
        // One projection folds whatever the fetch+process just landed.
        ids.push(self.push("project", "rebuild=false".into(), Spec::Project { rebuild: false, clear_changes: false }).await);
        // D5 reveal recheck rides the daily chain — issue 274's design intent
        // ("the nightly cadence walks the cohort and wraps", ~3 slices/cohort).
        // It sat on the weekly Sunday tick from when a run cost 18 minutes;
        // since the 274 slicing a run is 5–7s of read-only aggregates over one
        // 100k-section slice, and weekly stretched a full cohort walk to ~3
        // weeks (found mis-cadenced 2026-08-25). The queued-guard mirrors the
        // weekly jobs': never stack two, the report is overwritten anyway.
        if self.already_pending("reveal-recheck") {
            eprintln!("[schedule] reveal-recheck already queued or running, skipping today");
        } else {
            ids.push(self.push("reveal-recheck", "reveal recheck (daily slice)".into(), Spec::RevealRecheck).await);
        }
        ids
    }
}

/// Job kinds whose per-batch / per-package TRUNCATE checkpoint (issue 42) needs a
/// reader-free window to reclaim the WAL, so the coverage refresher's
/// multi-minute full-table scan must stand down while one runs (issue 53's 70 GB
/// balloon). This is the allowlist behind [`Supervisor::heavy_write_in_progress`].
///
/// Issue 281: the original list named only `process`/`project`/`reprocess`/
/// `reindex`/`refold`/`refold-fields` and silently omitted every OTHER batched
/// writer — `reparse` (the longest job), `merge-provisional-orgs` (~30M orgs),
/// `mark-skipped-siblings`, the `backfill-*` walks, `refold-notices`/
/// `refold-sections`, `repair-swept-siblings` — each of which TRUNCATEs per
/// batch just the same. Over-inclusion is cheap here (a read-only job listed by
/// mistake only makes coverage skip one refresh — a staleness, not a fault),
/// while under-inclusion is the actual WAL hazard, so the belt covers every job
/// that writes canonical/parsed rows in checkpointed batches. Read-only or
/// trivial-write kinds stay off it (`probe`, `data-quality`, `reveal-recheck`,
/// `register-archive`, `clear-rebuild-flag`) so coverage still refreshes during
/// them.
fn heavy_write_kind(kind: &str) -> bool {
    matches!(
        kind,
        "process"
            | "project"
            | "reprocess"
            | "reindex"
            | "refold"
            | "refold-fields"
            | "refold-notices"
            | "refold-sections"
            | "reparse"
            | "merge-provisional-orgs"
            | "mark-skipped-siblings"
            | "repair-swept-siblings"
            | "backfill-deadlines"
            | "backfill-titles"
            | "backfill-values"
            | "backfill-org-names"
            | "backfill-legacy-adjacency"
            | "rederive-eur"
            | "repair-nested-orgs"
            | "repair-placeholder-orgs"
            | "match-org-identifiers"
            | "build-org-match-keys"
            | "scan-org-match-keys"
            | "backfill-org-name-variants"
            | "fetch-rates"
            | "fetch-rates-ecu"
    )
}

/// Issue 308: the D5 cohort walk wraps every ~3 nightly slices, so per-slice
/// numbers cannot be watched — the BROKEN count (the campaign's acceptance
/// metric) existed only in job-counts lines. The `reveal-cursor` report
/// carries running per-wrap totals alongside the cursor; a slice that wraps
/// rolls them (final slice included) into a `reveal-wrap` report and resets.
/// `/metrics` reads ONLY `reveal-wrap`, so a partial wrap never moves the
/// gauges and consecutive values are comparable wrap to wrap. A missing or
/// legacy `{"after": N}` cursor body counts as zero running totals — the
/// walk restarts honestly rather than inventing history.
fn roll_reveal_wrap(prev_cursor: Option<&str>, sl: &store::RevealSlice, now: i64) -> (String, Option<String>) {
    let running = prev_cursor
        .and_then(|b| serde_json::from_str::<serde_json::Value>(b).ok())
        .map(|v| v["wrap"].clone())
        .unwrap_or(serde_json::Value::Null);
    let pick = |k: &str| running[k].as_i64().unwrap_or(0);
    let due = pick("due") + sl.due;
    let revealed = pick("revealed") + sl.revealed;
    let awaiting = pick("awaiting") + sl.no_later;
    let broken = pick("broken") + (sl.checked - sl.revealed - sl.no_later);
    let slices = pick("slices") + 1;
    if sl.wrapped {
        let wrap = serde_json::json!({
            "due": due, "revealed": revealed, "awaiting": awaiting, "broken": broken,
            "slices": slices, "withheld_rows": sl.withheld_total, "completed_at": now,
        })
        .to_string();
        (serde_json::json!({ "after": 0 }).to_string(), Some(wrap))
    } else {
        let cursor = serde_json::json!({
            "after": sl.upto,
            "wrap": { "due": due, "revealed": revealed, "awaiting": awaiting, "broken": broken, "slices": slices },
        })
        .to_string();
        (cursor, None)
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
        let tick = berlin_tick_on(day + k, hour, minute);
        if tick > now {
            let weekday = weekday_of(day + k);
            return (tick, weekday != 0 && weekday != 6);
        }
    }
    unreachable!("a matching tick exists within a week")
}

/// The unix instant at which Berlin wall-clock reads `hour:minute` on this UTC `day`.
/// Split out of [`next_berlin_tick`] so the startup catch-up (issue 245) can ask about
/// a tick in the PAST — today's — which the "next" form by construction cannot answer.
fn berlin_tick_on(day: i64, hour: i64, minute: i64) -> i64 {
    let midnight = day * 86_400;
    let offset = berlin_offset(midnight + 12 * 3_600);
    midnight + hour * 3_600 + minute * 60 - offset
}

/// Whether the day's tick passed without being served, so startup should run it now
/// (issue 245).
///
/// A tick counts as served by a SUCCESSFUL `probe` at or after it — the first job
/// `enqueue_daily` pushes — or by one still sitting in the queue, which the durable
/// job rows re-run on their own after a restart. Anything else means the tick fired
/// into a process that is no longer here, or never fired because the box was down.
///
/// Deliberately keyed on `probe` rather than on any daily job: `project` is pushed by
/// every maintenance refold too (the same conflation that made `ingest_freshness`
/// lie), and `process` succeeds trivially when there is nothing to process.
fn tick_needs_catch_up(tick: i64, now: i64, runs: &[JobRun], probe_queued: bool) -> bool {
    if tick > now || probe_queued {
        return false;
    }
    !runs
        .iter()
        .any(|r| r.kind == "probe" && r.outcome == "ok" && r.finished_at >= tick)
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

    /// Issue 308: the reveal-cursor report accumulates per-wrap totals and only a
    /// COMPLETED wrap rolls them into the `reveal-wrap` report the gauges read.
    #[test]
    fn reveal_wrap_totals_accumulate_and_roll_only_on_wrap() {
        let slice = |due, revealed, no_later, upto, wrapped| store::RevealSlice {
            withheld_total: 279_483,
            after: 0,
            upto,
            wrapped,
            sections: 100_000,
            dated: due + 10,
            due,
            checked: due,
            revealed,
            no_later,
            by_field: vec![],
        };

        // Fresh start (no cursor report): the first slice seeds the running totals.
        let (cursor, wrap) = roll_reveal_wrap(None, &slice(2606, 252, 1221, 25_535_052, false), 100);
        assert!(wrap.is_none(), "a partial wrap must not publish");
        let c: serde_json::Value = serde_json::from_str(&cursor).expect("cursor json");
        assert_eq!(c["after"].as_i64(), Some(25_535_052));
        assert_eq!(c["wrap"]["due"].as_i64(), Some(2606));
        assert_eq!(c["wrap"]["broken"].as_i64(), Some(2606 - 252 - 1221));
        assert_eq!(c["wrap"]["slices"].as_i64(), Some(1));

        // Second slice accumulates on top of the first.
        let (cursor, wrap) =
            roll_reveal_wrap(Some(&cursor), &slice(1029, 54, 601, 26_788_048, false), 200);
        assert!(wrap.is_none());
        let c: serde_json::Value = serde_json::from_str(&cursor).expect("cursor json");
        assert_eq!(c["wrap"]["due"].as_i64(), Some(2606 + 1029));
        assert_eq!(c["wrap"]["broken"].as_i64(), Some(1133 + 374));
        assert_eq!(c["wrap"]["slices"].as_i64(), Some(2));

        // The wrapping slice rolls the sum (itself included) into the wrap report
        // and resets the cursor to a bare restart — no stale totals carry over.
        let (cursor, wrap) = roll_reveal_wrap(Some(&cursor), &slice(40, 10, 20, 27_000_000, true), 300);
        let w: serde_json::Value =
            serde_json::from_str(&wrap.expect("a wrapped slice publishes")).expect("wrap json");
        assert_eq!(w["due"].as_i64(), Some(2606 + 1029 + 40));
        assert_eq!(w["revealed"].as_i64(), Some(252 + 54 + 10));
        assert_eq!(w["awaiting"].as_i64(), Some(1221 + 601 + 20));
        assert_eq!(w["broken"].as_i64(), Some(1133 + 374 + 10));
        assert_eq!(w["slices"].as_i64(), Some(3));
        assert_eq!(w["withheld_rows"].as_i64(), Some(279_483));
        assert_eq!(w["completed_at"].as_i64(), Some(300));
        let c: serde_json::Value = serde_json::from_str(&cursor).expect("cursor json");
        assert_eq!(c["after"].as_i64(), Some(0));
        assert!(c["wrap"].is_null(), "the running totals reset with the wrap");

        // A legacy pre-308 cursor body ({"after": N} only) reads as zero running
        // totals rather than failing or fabricating.
        let (cursor, wrap) = roll_reveal_wrap(
            Some(r#"{"after": 25535052}"#),
            &slice(100, 30, 50, 26_000_000, false),
            400,
        );
        assert!(wrap.is_none());
        let c: serde_json::Value = serde_json::from_str(&cursor).expect("cursor json");
        assert_eq!(c["wrap"]["due"].as_i64(), Some(100));
        assert_eq!(c["wrap"]["broken"].as_i64(), Some(20));
        assert_eq!(c["wrap"]["slices"].as_i64(), Some(1));
    }

    /// Issue 247: `cancel` must reach the RUNNING job, not only queued ones.
    #[tokio::test]
    async fn cancelling_the_running_job_asks_it_to_stop() {
        let db = scratch().await;
        let sup = Arc::new(Supervisor::new(db, "archive".into(), reqwest::Client::new()));

        // Nothing running, nothing queued: cancel finds no such job.
        assert_eq!(sup.cancel(1).await, Cancelled::Unknown, "no job to cancel");

        // A queued job still cancels the old way — removed from the queue outright.
        let queued = sup
            .push("project", "rebuild=false".into(), Spec::Project {
                rebuild: false,
                clear_changes: false,
            })
            .await;
        assert_eq!(sup.cancel(queued).await, Cancelled::Queued, "a queued job is cancellable");
        assert!(sup.queue.lock().expect("queue lock").is_empty());
        assert!(!sup.cancelled(queued), "a removed job needs no stop flag");

        // A RUNNING job is flagged instead: it ends its own transaction, logs why, and
        // drops its durable row like any concluded job. `execute` publishes the progress
        // record, so that is what identifies the running job here.
        sup.set_current(Some(JobProgress {
            id: 42,
            kind: "reparse".into(),
            params: "reparse text".into(),
            started_at: 0,
            package: None,
            packages_done: 0,
            packages_total: 1,
            members_done: 0,
            members_total: 0,
            notices: 0,
            duplicates: 0,
            phase: None,
        }));
        assert!(!sup.cancelled(42), "not cancelled until asked");
        assert_eq!(
            sup.cancel(42).await,
            Cancelled::Stopping,
            "reparse reads the flag, so the running job accepts a stop request"
        );
        assert!(sup.cancelled(42), "and the job sees it at its next checkpoint");
        // The flag names ONE job: a different id must not stop on someone else's request.
        assert!(!sup.cancelled(43));
    }

    /// Issue 252: a running job whose kind reads no stop flag must be REFUSED, not told it
    /// is stopping. Measured on prod: a cancelled `data-quality` run answered
    /// `{"cancelled": 202}` and then advanced ten more queries over six minutes, because
    /// `cancelled()` had exactly one caller and it was not that loop.
    #[tokio::test]
    async fn cancelling_a_kind_with_no_checkpoint_is_refused_rather_than_promised() {
        let db = scratch().await;
        let sup = Arc::new(Supervisor::new(db, "archive".into(), reqwest::Client::new()));

        let running = |kind: &str| {
            Some(JobProgress {
                id: 7,
                kind: kind.to_owned(),
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
            })
        };

        // `reindex` has no checkpoint: refused, and the refusal names the kind so the
        // caller can say which. (`project` used to be this test's example — it gained
        // its checkpoints in issue 256, after two TENDER_DROP_JOBS restarts in one
        // day were what "cancelling" a fold actually took.)
        sup.set_current(running("reindex"));
        assert_eq!(sup.cancel(7).await, Cancelled::Unstoppable("reindex".to_owned()));
        assert!(!sup.cancelled(7), "a refused cancel must not leave a flag set");

        // `data-quality` now checks between queries, so it is accepted.
        sup.set_current(running("data-quality"));
        assert_eq!(sup.cancel(7).await, Cancelled::Stopping);
        assert!(sup.cancelled(7));

        // And every kind named stoppable must actually be one — the list is the contract,
        // so a kind added to it without a checkpoint is the bug this test exists to catch.
        // org-merge-health and r2-census read the flag at the top of every census batch
        // and store nothing when stopped (issue 300 Stages 0 and 2);
        // match-org-identifiers polls it between preload windows and merge
        // transactions, and a stopped run reports its committed prefix.
        // build-org-match-keys reads it at the top of every 10k-org window,
        // and a stopped wet build leaves its watermark standing so the next
        // run resumes (issue 300 Stage 4). scan-org-match-keys reads it
        // between index pages, between classification groups, and between
        // edge-write batches; a stopped run records no report.
        // case-review-backlog reads the flag between verdict rows and again
        // between per-case evidence probes, and returns an EMPTY report
        // rather than a partial backlog (issue 317).
        // satellite-orphans reads it between origins and again between the
        // per-orphan destination probes, and a stopped run stores nothing —
        // an undercount of leftover keys would read as a clean campaign.
        // rehoming-packet reads it between cases and again between the
        // per-group destination probes, and a stopped run stores no packet
        // — a HALF packet is worse than none, because a reviewer cannot see
        // that the missing cases were dropped rather than clean (issue 317
        // Unit A).
        // ghost-census reads it between notice-id slices, and a stopped run
        // stores NO report — a partial ghost count reads as a clean corpus,
        // which is the one wrong answer this census must never give. Its
        // predecessor could not be stopped at all: one unbounded statement
        // with no checkpoint between anything (issue 278 INCIDENT).
        assert_eq!(
            STOPPABLE_KINDS,
            &[
                "reparse",
                "data-quality",
                "project",
                "merge-provisional-orgs",
                "org-merge-health",
                "r2-census",
                "r3-census",
                "match-org-identifiers",
                "build-org-match-keys",
                "scan-org-match-keys",
                "org-edge-census",
                "case-review-backlog",
                "fold-org-countries",
                "fusion-census",
                "rehoming-packet",
                "satellite-orphans",
                "drop-orphan-satellites",
                "anchor-wall-census",
                "xb-packet",
                "country-typo-census",
                "country-cluster-census",
                "duplicate-identity-census",
                "name-pollution-census",
                "generic-wall-census",
                "generic-statistic-census",
                "name-attribution-probe",
                "ghost-census",
                "repair-country-typos",
                "repair-label-prefixes",
                "repair-minted-countries"
            ]
        );
    }

    /// Issue 325's tripwire quoted its predecessor WHOLE, so every weekly run
    /// nested one level deeper — the live report had reached three levels, each
    /// carrying a full per-scheme table, and report history now keeps ten
    /// versions of it. Found 2026-09-02 by reading the stored report while
    /// checking that the tripwire was still being computed at all.
    #[test]
    fn the_stored_baseline_is_three_numbers_and_never_quotes_its_own_predecessor() {
        // A previous report in the shape the bug produced: a block that already
        // carries a baseline of its own, plus the bulky scheme table.
        let previous = serde_json::json!({
            "parser_vs_stock": {
                "no_longer_vat": 7,
                "vat_country_differs": 1,
                "vat_refused": 0,
                "gln_shared_one_country": 0,
                "schemes": [{"scheme": "FR:siret", "pop": 71036, "pass": 64059, "fail": 2840}],
                "alarms": [],
                "baseline": {"no_longer_vat": 9, "baseline": {"no_longer_vat": 11}},
            }
        });
        let trimmed = trimmed_baseline(Some(&previous));

        // The counters survive, because the point of keeping a baseline at all
        // is that a reader can see what the alarms compared against.
        assert_eq!(trimmed["no_longer_vat"], 7);
        assert_eq!(trimmed["vat_country_differs"], 1);
        assert_eq!(trimmed["vat_refused"], 0);
        // NOT gln_shared_one_country: it is a zero-floor check that keeps no
        // baseline, so storing it only ever produced a null field.
        assert!(trimmed.get("gln_shared_one_country").is_none());

        // THE REGRESSION: no recursion, and no bulk.
        assert!(trimmed.get("baseline").is_none(), "a baseline must not carry a baseline");
        assert!(trimmed.get("schemes").is_none(), "nor the per-scheme table");
        assert_eq!(
            trimmed.as_object().map(|o| o.len()),
            Some(3),
            "three counters, so the body is the same size on week one and week fifty"
        );

        // A first run and a malformed predecessor both give null rather than a
        // half-built object — the alarms function already reads a missing key as
        // "no baseline for that key" (see the test above), so null is the shape
        // it expects.
        assert!(trimmed_baseline(None).is_null());
        assert!(trimmed_baseline(Some(&serde_json::json!({"parser_vs_stock": 3}))).is_null());
        assert!(trimmed_baseline(Some(&serde_json::json!({"something_else": 1}))).is_null());
    }

    /// Issue 247: the deferred-index bootstrap must jump the queue, because the queue is
    /// what needs the index.
    #[tokio::test]
    async fn a_missing_index_reindex_is_queued_ahead_of_pending_work() {
        let db = scratch().await;
        let sup = Arc::new(Supervisor::new(db, "archive".into(), reqwest::Client::new()));

        // Two ordinary jobs first — the shape prod had: a long re-parse and its fold,
        // already queued when the box notices an index is missing.
        sup.push("reparse", "reparse text".into(), Spec::Reparse {
            profiles: vec!["text".to_owned()],
            packages: Some(1),
            after: None,
        })
        .await;
        sup.push("project", "rebuild=false".into(), Spec::Project {
            rebuild: false,
            clear_changes: false,
        })
        .await;

        sup.ensure_deferred_indexes().await;

        let kinds = |sup: &Arc<Supervisor>| -> Vec<String> {
            sup.queue.lock().expect("queue lock").iter().map(|j| j.kind.clone()).collect()
        };
        assert_eq!(
            kinds(&sup),
            vec!["reindex", "reparse", "project"],
            "the reindex runs FIRST: a re-parse behind a missing index costs 153 ms a notice \
             instead of microseconds, and prod paid that for hours"
        );

        // And a reindex that is already queued BEHIND work gets moved, which is the case
        // prod was actually in — the first boot queued one at the back, and every later
        // boot said "already queued" and left it there.
        {
            let mut queue = sup.queue.lock().expect("queue lock");
            let stale = queue.pop_front().expect("the reindex");
            queue.push_back(stale);
        }
        assert_eq!(kinds(&sup), vec!["reparse", "project", "reindex"], "moved to the back");
        sup.ensure_deferred_indexes().await;
        assert_eq!(
            kinds(&sup),
            vec!["reindex", "reparse", "project"],
            "an already-queued reindex is moved to the front, not left where it was"
        );
        // Idempotent: a third call with it already first changes nothing.
        sup.ensure_deferred_indexes().await;
        assert_eq!(kinds(&sup), vec!["reindex", "reparse", "project"]);
    }

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

    /// The startup catch-up's decision (issue 245), which is the whole of it: the
    /// enqueue afterwards is `enqueue_daily`, already covered.
    #[test]
    fn a_tick_that_passed_unserved_is_caught_up_on_startup() {
        let run = |kind: &str, outcome: &str, finished_at: i64| JobRun {
            id: 1,
            job_id: None,
            kind: kind.into(),
            params: String::new(),
            started_at: finished_at - 10,
            finished_at,
            outcome: outcome.into(),
            counts: String::new(),
        };
        let tick = fetch::days_from_civil(2026, 7, 15) * 86_400 + 7 * 3_600 + 35 * 60;
        let now = tick + 4 * 3_600; // startup at 13:35 Berlin, four hours late

        // Nothing since the tick — the deploy that restarted the box ate it.
        assert!(tick_needs_catch_up(tick, now, &[run("project", "ok", now - 60)], false));

        // A probe that succeeded after the tick means the daily ran: stay quiet.
        assert!(!tick_needs_catch_up(tick, now, &[run("probe", "ok", tick + 30)], false));

        // A probe that succeeded BEFORE the tick is yesterday's, and does not serve today.
        assert!(tick_needs_catch_up(tick, now, &[run("probe", "ok", tick - 100)], false));

        // A probe still in the queue after a restart will run on its own — the durable
        // rows survive; only the catch-up LOOP is lost. Do not double up.
        assert!(!tick_needs_catch_up(tick, now, &[], true));

        // Before the tick, there is nothing to catch up; the loop will serve it.
        assert!(!tick_needs_catch_up(tick, tick - 1, &[], false));

        // A FAILED probe is not a served tick — the pipeline did not get its data.
        assert!(tick_needs_catch_up(tick, now, &[run("probe", "error", tick + 30)], false));
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

    /// Issue 319: the fold job wired to the REAL `canonical_country`,
    /// `is_vat_scope_label` and `is_alpha2` — the store test injects
    /// miniatures, so without this nothing proves the production fns are
    /// the ones the job runs with (panel catch).
    /// Issue 317 Unit A: the packet job against the real tables and the real
    /// `match_norm`. The store test owns the grouping rules; what this pins
    /// is the wiring — that the job stores a report under the name the
    /// campaign will fetch it by, and that the report actually carries the
    /// two things a verdict cannot be written without: the mention's address
    /// and a destination org id.
    #[tokio::test]
    async fn the_rehoming_packet_job_stores_addresses_and_destinations() {
        let path = format!(
            "/tmp/tender-db-sup-packet-{}-{}.db",
            std::process::id(),
            store::now_unix()
        );
        let _ = std::fs::remove_file(&path);
        let db = Arc::new(store::Db::open(&path).await.unwrap());
        let raw = store::turso::Builder::new_local(&path).build().await.unwrap();
        let conn = raw.connect().unwrap();
        for (id, name) in
            [(1i64, "Bietergemeinschaft Dobler / Oberall"), (2, "Dobler GmbH & Co. KG")]
        {
            conn.execute(
                "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
                 VALUES (?, 'DE', NULL, NULL, ?, ?, 0, 0)",
                (
                    store::turso::Value::Integer(id),
                    store::turso::Value::Text(name.into()),
                    store::turso::Value::Text(name.to_lowercase()),
                ),
            )
            .await
            .unwrap();
            // The satellite key, written by the REAL normalizer — the same
            // function the packet groups mentions by. If those two ever drift
            // apart, every group loses its destinations and this fails.
            conn.execute(
                "INSERT INTO org_match_keys (org_id, key_kind, key) VALUES (?, 'n2', ?)",
                (
                    store::turso::Value::Integer(id),
                    store::turso::Value::Text(ingest::project::match_norm(name)),
                ),
            )
            .await
            .unwrap();
        }
        conn.execute(
            "INSERT INTO org_case_reviews
               (case_org_id, cohort, verdict, diagnosis, handling, rationale, confidence,
                reviewed_at, applied_at, applied_action, job_id)
             VALUES (1, 'biege-pilot', 'consortium-vehicle-wrong-identifier', 'd', 'h', 'r',
                     'high', 1, 1, 'a', 1)",
            (),
        )
        .await
        .unwrap();
        conn.execute(
            "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, country, raw_identifier)
             VALUES (7001, 'ORG-0002', 1, 'Dobler GmbH & Co. KG', 'DE', NULL)",
            (),
        )
        .await
        .unwrap();

        let sup = Supervisor::new(db, "archive".into(), reqwest::Client::new());
        let job = Job {
            id: 1,
            kind: "rehoming-packet".into(),
            params: String::new(),
            spec: Spec::RehomingPacket,
            resume_after: None,
        };
        // Every destination comes from the N2 satellite, so an unusable one
        // does not make the packet wrong visibly — it makes every group read
        // "no destination anywhere". The job refuses instead.
        let err = sup.run_spec(&job).await.expect_err("no covering index yet");
        assert!(err.contains("org_match_keys_kk"), "{err}");
        conn.execute(
            "CREATE INDEX IF NOT EXISTS org_match_keys_kk \
                 ON org_match_keys(key_kind, key, org_id)",
            (),
        )
        .await
        .unwrap();
        let err = sup.run_spec(&job).await.expect_err("epoch is still empty");
        assert!(err.contains("epoch"), "{err}");
        conn.execute(
            "UPDATE projection_state SET org_match_keys_watermark = 0, \
                 org_match_keys_epoch = ? WHERE id = 0",
            (store::turso::Value::Text(ingest::crosswalk::NAME_KEY_EPOCH.into()),),
        )
        .await
        .unwrap();

        let msg = sup.run_spec(&job).await.expect("the packet reads");
        assert!(msg.contains("1 still hold"), "{msg}");
        let (body, _) = sup.db().latest_report("rehoming-packet").await.unwrap().expect("packet");
        // The address, keyed exactly as org_mention_rehoming keys a verdict.
        assert!(body.contains("\"notice_id\":7001"), "{body}");
        assert!(body.contains("\"section_id\":\"ORG-0002\""), "{body}");
        // And the destination, which only the satellite join can supply.
        assert!(body.contains("\"target_total\":1"), "{body}");
        assert!(body.contains("\"org\":2"), "{body}");
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn the_country_fold_job_runs_with_the_real_tables() {
        let path = format!(
            "/tmp/tender-db-sup-fold-{}-{}.db",
            std::process::id(),
            store::now_unix()
        );
        let _ = std::fs::remove_file(&path);
        let db = Arc::new(store::Db::open(&path).await.unwrap());
        let raw = store::turso::Builder::new_local(&path).build().await.unwrap();
        let conn = raw.connect().unwrap();
        for (id, country, kind, ident) in [
            (1i64, "GRL", "national", "18440202"),   // folds to GL
            (2, "MCO", "national", "MC1"),           // folds to MC
            (3, "EL", "vat", "EL094019245"),         // VAT scope label: skipped
            (4, "DE", "national", "DE811111111"),    // already canonical
            (5, "1A0", "national", "X1"),            // residue
        ] {
            conn.execute(
                "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
                 VALUES (?, ?, ?, ?, 'n', 'n', 0, 0)",
                (
                    store::turso::Value::Integer(id),
                    store::turso::Value::Text(country.into()),
                    store::turso::Value::Text(kind.into()),
                    store::turso::Value::Text(ident.into()),
                ),
            )
            .await
            .unwrap();
        }
        let sup = Supervisor::new(db, "archive".into(), reqwest::Client::new());
        let job = |id: u64, dry: bool| Job {
            id,
            kind: "fold-org-countries".into(),
            params: String::new(),
            spec: Spec::FoldOrgCountries { dry_run: dry },
            resume_after: None,
        };
        // A wet run with no reviewed plan refuses — the T4 ladder.
        let err = sup.run_spec(&job(1, false)).await.expect_err("wet needs the plan");
        assert!(err.contains("country-fold"), "{err}");
        // Dry: the real ISO tables fold GRL and MCO, skip EL, and name '1A0'.
        let msg = sup.run_spec(&job(2, true)).await.expect("dry plans");
        assert!(msg.contains("DRY RUN"), "{msg}");
        let (plan, _) = sup.db().latest_report("country-fold").await.unwrap().expect("plan");
        assert!(plan.contains("\"rows\":2"), "{plan}");
        // serde_json orders object keys alphabetically, so the pair reads
        // from/rows/to — assert on that shape, not on the order I wrote.
        assert!(plan.contains("{\"from\":\"GRL\",\"rows\":1,\"to\":\"GL\"}"), "{plan}");
        assert!(plan.contains("{\"from\":\"MCO\",\"rows\":1,\"to\":\"MC\"}"), "{plan}");
        assert!(plan.contains("\"value\":\"1A0\""), "the residue is named: {plan}");
        assert!(plan.contains("\"value\":\"EL\""), "the VAT skip is reported: {plan}");
        assert!(!plan.contains("\"to\":\"GR\""), "EL must NOT be planned: {plan}");
        // Wet: runs under that plan.
        let msg = sup.run_spec(&job(3, false)).await.expect("wet folds");
        assert!(!msg.contains("DRY RUN"), "{msg}");
        let mut rows = conn
            .query("SELECT id, country FROM organizations ORDER BY id", ())
            .await
            .unwrap();
        let mut got: Vec<(i64, String)> = Vec::new();
        while let Some(row) = rows.next().await.unwrap() {
            let (store::turso::Value::Integer(id), store::turso::Value::Text(c)) =
                (row.get_value(0).unwrap(), row.get_value(1).unwrap())
            else {
                panic!("shape")
            };
            got.push((id, c));
        }
        assert_eq!(
            got,
            vec![
                (1, "GL".to_owned()),
                (2, "MC".to_owned()),
                (3, "EL".to_owned()),
                (4, "DE".to_owned()),
                (5, "1A0".to_owned()),
            ]
        );
    }

    /// Issue 316: r3's corroboration consults the generic-name wall, so the
    /// run refuses when that wall cannot be read. The split is the point: a
    /// build in flight and an epoch-stale keyspace refuse BOTH ways (the
    /// probe has no index to stand on, and a dry plan is what a wet run
    /// trusts), while an EMPTY satellite refuses only a WET run — planning
    /// against it is cheap and harmless, merging under it is not.
    #[tokio::test]
    async fn the_r3_run_refuses_without_a_usable_key_satellite() {
        let path = format!(
            "/tmp/tender-db-sup-r3guard-{}-{}.db",
            std::process::id(),
            store::now_unix()
        );
        let _ = std::fs::remove_file(&path);
        let db = Arc::new(store::Db::open(&path).await.unwrap());
        let raw = store::turso::Builder::new_local(&path).build().await.unwrap();
        let conn = raw.connect().unwrap();
        let sup = Supervisor::new(db, "archive".into(), reqwest::Client::new());
        let job = |id: u64, dry: bool| Job {
            id,
            kind: "match-org-identifiers".into(),
            params: String::new(),
            spec: Spec::MatchOrgIdentifiersR3 { dry_run: dry, max_groups: None },
            resume_after: None,
        };
        // 1. Empty satellite: the wall is not there, so a wet run refuses —
        //    and the refusal names the WET remedy, because the remedy job's
        //    own default is dry and a dry build stores nothing (panel catch:
        //    "run build-org-match-keys first" sends the operator in a loop).
        let err = sup.run_spec(&job(1, false)).await.expect_err("empty satellite");
        assert!(err.contains("org_match_keys is empty"), "{err}");
        assert!(err.contains("\"dry_run\":false"), "the remedy must be the WET one: {err}");
        // 2. Keys present but a build is mid-walk (non-zero watermark): the
        //    covering index does not exist yet.
        conn.execute(
            "INSERT INTO org_match_keys (org_id, key_kind, key) VALUES (1, 'n2', 'acme')",
            (),
        )
        .await
        .unwrap();
        conn.execute(
            "UPDATE projection_state SET org_match_keys_watermark = 77 WHERE id = 0",
            (),
        )
        .await
        .unwrap();
        let err = sup.run_spec(&job(2, false)).await.expect_err("build in flight");
        assert!(err.contains("build is in flight") && err.contains("77"), "{err}");
        // 3. Watermark clear but the keys were built under SUPERSEDED
        //    semantics: the wall would answer about a keyspace `norm` no
        //    longer produces, so it refuses too (panel catch — the first
        //    version of this guard read the epoch and threw it away, and
        //    this very test asserted the stale state was fine).
        conn.execute(
            "UPDATE projection_state SET org_match_keys_watermark = 0 WHERE id = 0",
            (),
        )
        .await
        .unwrap();
        let err = sup.run_spec(&job(3, false)).await.expect_err("stale keys epoch");
        assert!(err.contains("keys epoch"), "{err}");
        // 4. Epoch stamped to this binary's: the guard passes, and the run
        //    goes on to fail on the NEXT precondition — which is how we know
        //    it passed this one.
        conn.execute(
            "UPDATE projection_state SET org_match_keys_epoch = ? WHERE id = 0",
            (store::turso::Value::Text(ingest::crosswalk::NAME_KEY_EPOCH.to_owned()),),
        )
        .await
        .unwrap();
        let err = sup.run_spec(&job(4, false)).await.expect_err("no reviewed plan");
        assert!(err.contains("r3-merge-plan"), "{err}");
        // 5. A DRY run plans on an EMPTY satellite — the state every first
        //    run is in, where the probe is cheap and a person reads the plan
        //    before anything merges.
        conn.execute("DELETE FROM org_match_keys", ()).await.unwrap();
        let msg = sup.run_spec(&job(5, true)).await.expect("dry plans");
        assert!(msg.contains("DRY RUN"), "{msg}");
        //    …and it SAYS the wall was blind, so "denied 0 generic names"
        //    cannot be read as "nothing was generic" when it means "nothing
        //    could be seen" (panel catch).
        assert!(msg.contains("org_match_keys is EMPTY"), "{msg}");
        let (plan, _) = sup.db().latest_report("r3-merge-plan").await.unwrap().expect("plan");
        assert!(plan.contains("\"generic_wall_readable\":false"), "{plan}");
        // 6. But a DRY run does NOT get to skip the COST guard: mid-build,
        //    the probe has no covering index and would full-scan the whole
        //    satellite once per candidate. This is the panel's catch — the
        //    first guard was wet-only, and dry is this job's default.
        conn.execute(
            "INSERT INTO org_match_keys (org_id, key_kind, key) VALUES (1, 'n2', 'acme')",
            (),
        )
        .await
        .unwrap();
        conn.execute(
            "UPDATE projection_state SET org_match_keys_watermark = 5 WHERE id = 0",
            (),
        )
        .await
        .unwrap();
        let err = sup.run_spec(&job(6, true)).await.expect_err("dry refuses mid-build too");
        assert!(err.contains("build is in flight"), "{err}");
        // …and the same for a keyspace built under superseded semantics: a
        // dry plan computed against the wrong keyspace is a plan a later wet
        // run trusts.
        conn.execute(
            "UPDATE projection_state SET org_match_keys_watermark = 0, \
                 org_match_keys_epoch = 'n0v0' WHERE id = 0",
            (),
        )
        .await
        .unwrap();
        let err = sup.run_spec(&job(7, true)).await.expect_err("dry refuses stale semantics");
        assert!(err.contains("keys epoch"), "{err}");
    }

    /// Issue 173 (D4): the rehash probe enqueues with its sample cap, defaulting
    /// to the weekly 8, and the params string names the count an operator will
    /// see in the queue.
    #[tokio::test]
    async fn rehash_probe_enqueues_with_its_sample_cap() {
        let db = scratch().await;
        let sup = Arc::new(Supervisor::new(db, "archive".into(), reqwest::Client::new()));
        sup.enqueue_request(&req("rehash-probe")).await.expect("enqueue default");
        let mut capped = req("rehash-probe");
        capped.packages = Some(3);
        sup.enqueue_request(&capped).await.expect("enqueue capped");
        let queued = sup.queued();
        assert_eq!(queued.len(), 2);
        assert_eq!(queued[0].params, "rehash probe (8 package(s))");
        assert_eq!(queued[1].params, "rehash probe (3 package(s))");
        assert!(queued.iter().all(|j| j.kind == "rehash-probe"));
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
        let plan = sup.run_data_quality(1, false).await.expect("a dry run cannot fail on an empty db");
        // Issue 272: the line must be unmistakable in the recent-jobs list — a
        // dry run queued as an acceptance read sat there reading like a pass.
        assert!(plan.starts_with("DRY RUN — STORED NOTHING"), "{plan}");
        assert!(plan.contains("{\"dry_run\": false}"), "the line must carry the enqueue fix: {plan}");
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
        let out = sup.run_data_quality(1, true).await.expect("an empty confirmed run is not an error");
        assert_eq!(out, "data quality: no tender versions to measure");
        assert!(sup.db.latest_report("data-quality").await.unwrap().is_none());
    }

    /// Issue 237: `refold-sections` needs a kind, and pairs itself with a projection so
    /// the re-queued notices are actually folded.
    #[tokio::test]
    async fn refold_sections_needs_a_kind_and_pairs_with_a_projection() {
        let sup = Supervisor::new(scratch().await, "archive".into(), reqwest::Client::new());
        assert!(
            sup.enqueue_request(&req("refold-sections")).await.is_err(),
            "a kind list is the whole cohort — refuse an empty one rather than re-fold nothing"
        );
        let ids = sup
            .enqueue_request(&JobRequest {
                kind: "refold-sections".into(),
                profiles: Some(vec!["GroupComposition".into()]),
                ..Default::default()
            })
            .await
            .expect("a kind enqueues");
        assert_eq!(ids.len(), 2, "the refold and its trailing projection");
        let queued = sup.queued();
        assert_eq!(queued[0].kind, "refold-sections");
        assert_eq!(queued[0].params, "refold-sections GroupComposition");
        assert_eq!(queued[1].kind, "project", "without the fold the requeue changes nothing");
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

    /// Issue 300 Stage 4: the candidate-edge scan defaults to a DRY census —
    /// the T4 ladder's reviewable pre-estimate — and its params line names
    /// the cap, the stoplist and the keys epoch, so a job-log reader can
    /// audit WHICH key semantics a scan ran under.
    #[tokio::test]
    async fn scan_org_match_keys_defaults_dry_and_names_its_knobs() {
        let sup = Supervisor::new(scratch().await, "archive".into(), reqwest::Client::new());
        sup.enqueue_request(&req("scan-org-match-keys")).await.expect("enqueues");
        let queued = sup.queued();
        assert_eq!(queued[0].kind, "scan-org-match-keys");
        // The params line fully encodes the knobs (the arm builds Spec and
        // params from the same locals), so the enqueue contract is pinned
        // here without reaching into the private queue.
        assert_eq!(
            queued[0].params,
            format!(
                "scan-org-match-keys dry-run stoplist=20 epoch={}",
                ingest::crosswalk::NAME_KEY_EPOCH
            )
        );
        sup.enqueue_request(&JobRequest {
            kind: "scan-org-match-keys".into(),
            dry_run: Some(false),
            max_edges: Some(200_000),
            ..Default::default()
        })
        .await
        .expect("wet with a cap enqueues");
        let queued = sup.queued();
        assert!(
            queued[1].params.starts_with("scan-org-match-keys cap=200000"),
            "a wet run's params name the cap: {}",
            queued[1].params
        );
    }

    /// Issue 300 Stage 4 (panel round): the scan handler's precondition
    /// ladder and T4 gates through `run_spec` itself — each refusal names
    /// its remedy, a cancelled run records NO report, a dry run records the
    /// plan with the build's lineage, and a completed uncapped wet run
    /// records the scan report and re-anchors the plan.
    #[tokio::test]
    async fn scan_org_match_keys_run_path_refusals_and_report_lifecycle() {
        let sup = Supervisor::new(scratch().await, "archive".into(), reqwest::Client::new());
        let job = |id: u64, dry: bool| Job {
            id,
            kind: "scan-org-match-keys".into(),
            params: String::new(),
            spec: Spec::ScanOrgMatchKeys { dry_run: dry, max_edges: None },
            resume_after: None,
        };
        // 1. No covering index → the remedy is the wet build.
        let err = sup.run_spec(&job(1, true)).await.expect_err("no index");
        assert!(err.contains("org_match_keys_kk"), "{err}");
        // 2. Index present (finish is IF NOT EXISTS) but the stored keys
        //    epoch is not this binary's → refuse, or rules mislabel.
        sup.db().finish_org_match_keys().await.unwrap();
        let err = sup.run_spec(&job(2, true)).await.expect_err("epoch mismatch");
        assert!(err.contains("epoch"), "{err}");
        // 3. Epoch stamped (reset) + index rebuilt (finish), but no
        //    completed build report → no lineage to echo.
        sup.db().reset_org_match_keys(ingest::crosswalk::NAME_KEY_EPOCH).await.unwrap();
        sup.db().finish_org_match_keys().await.unwrap();
        let err = sup.run_spec(&job(3, true)).await.expect_err("no build report");
        assert!(err.contains("org-match-keys-build"), "{err}");
        // 4. Build report present: a WET run still refuses without the
        //    reviewed dry plan (the T4 ladder).
        sup.db().put_report("org-match-keys-build", "{\"built_at\":123}", 123).await.unwrap();
        let err = sup.run_spec(&job(4, false)).await.expect_err("wet needs the plan");
        assert!(err.contains("org-edge-scan-plan"), "{err}");
        // 5. Dry run: the census records the plan with the build's lineage.
        let msg = sup.run_spec(&job(5, true)).await.expect("dry census");
        assert!(msg.contains("DRY RUN — STORED NOTHING"), "{msg}");
        let (plan, _) =
            sup.db().latest_report("org-edge-scan-plan").await.unwrap().expect("plan recorded");
        assert!(plan.contains("\"keys_built_at\":123"), "{plan}");
        assert!(plan.contains("\"bounds_ok\":true"), "{plan}");
        // 6. A cancelled wet run records NO report (the zero-lie bar).
        sup.cancel_running.store(6, Ordering::Relaxed);
        let msg = sup.run_spec(&job(6, false)).await.expect("stopped, not failed");
        assert!(msg.contains("STOPPED by cancel"), "{msg}");
        assert!(
            sup.db().latest_report("org-edge-scan").await.unwrap().is_none(),
            "a stopped run must not record a scan report"
        );
        sup.cancel_running.store(0, Ordering::Relaxed);
        // 7. A completed uncapped wet run records the scan report and
        //    re-anchors the plan (the weekly-cadence deadlock fix).
        let msg = sup.run_spec(&job(7, false)).await.expect("wet");
        assert!(msg.contains("scan-org-match-keys (issue 300 Stage 4)"), "{msg}");
        assert!(sup.db().latest_report("org-edge-scan").await.unwrap().is_some());
        // 7b. Tripwire 6 wiring: the completed wet anchored the baseline
        //     and wrote a CLEAR verdict to the alarm surface.
        let (alarm, _) =
            sup.db().latest_report("org-edge-scan-alarm").await.unwrap().expect("alarm surface");
        assert!(alarm.contains("\"clear\":true"), "{alarm}");
        assert_eq!(sup.db().org_edge_baseline().await.unwrap(), 0, "anchored at the total");
        // 7c. Tripwire 6 through the REAL handler (panel round 2: the
        //     alarm block's wiring must fire in a wet run, not only in the
        //     pure-fn ladder): seed the baseline above the standing table —
        //     the pre-write count reads as an out-of-band shrink; the
        //     verdict reaches the job message, the wet report, the alarm
        //     surface, and the baseline re-anchors.
        sup.db().set_org_edge_baseline(10).await.unwrap();
        let msg = sup.run_spec(&job(9, false)).await.expect("alarmed, not failed");
        assert!(msg.contains("TRIPWIRE 6 SHRUNK"), "{msg}");
        let (scan, _) =
            sup.db().latest_report("org-edge-scan").await.unwrap().expect("wet report");
        assert!(scan.contains("\"alarm\":\"SHRUNK\""), "{scan}");
        let (alarm, _) =
            sup.db().latest_report("org-edge-scan-alarm").await.unwrap().expect("surface");
        assert!(alarm.contains("\"alarm\":\"SHRUNK\""), "{alarm}");
        assert_eq!(sup.db().org_edge_baseline().await.unwrap(), 0, "re-anchored 10 -> 0");
        // 8. A keys rebuild after the plan was recorded → wet refuses on
        //    lineage until a fresh dry census is reviewed — and the refusal
        //    lands on the alarm surface (the muted-tripwire fix), not just
        //    as a failed 03:xx job line.
        sup.db().put_report("org-match-keys-build", "{\"built_at\":456}", 456).await.unwrap();
        let err = sup.run_spec(&job(8, false)).await.expect_err("stale plan");
        assert!(err.contains("predates"), "{err}");
        let (alarm, _) =
            sup.db().latest_report("org-edge-scan-alarm").await.unwrap().expect("refusal alarm");
        assert!(alarm.contains("predates"), "{alarm}");
        // 9. The IN-STORE T4 parity abort reaches the alarm surface too
        //    (panel round 2's lead finding: a census spike parity-aborts
        //    BEFORE edge_alarm runs, and that path stopping silently was
        //    the ops amendment's exact scenario). Plant a plan whose
        //    would_emit is far from the live census → the wet aborts, and
        //    the abort — not a stale clear — is what the surface shows.
        sup.db()
            .put_report(
                "org-edge-scan-plan",
                "{\"keys_built_at\":456,\"bounds_ok\":true,\"would_emit\":5000}",
                457,
            )
            .await
            .unwrap();
        let err = sup.run_spec(&job(10, false)).await.expect_err("parity abort");
        assert!(err.contains("parity abort"), "{err}");
        let (alarm, _) =
            sup.db().latest_report("org-edge-scan-alarm").await.unwrap().expect("abort alarm");
        assert!(alarm.contains("parity abort"), "{alarm}");
    }

    /// Tripwire 6's decision ladder (issue 300 Stage 4 Unit 5): shrink is
    /// always RED, growth alarms only past the volume ceiling or twice the
    /// reviewed plan, and both the first anchor and weekly drift are quiet.
    #[test]
    fn edge_alarm_ladder() {
        assert_eq!(
            edge_alarm(0, 0, 1_498_485, 1_498_485),
            None,
            "the first anchor is not a spike"
        );
        assert_eq!(
            edge_alarm(1_000_000, 1_000_000, 1_010_000, 1_000_000),
            None,
            "weekly drift is quiet"
        );
        // SHRUNK reads the PRE-write count: the wet run's own upserts
        // re-cover an out-of-band deletion, so the post-run total alone
        // would mask it (panel round 2).
        assert_eq!(
            edge_alarm(100, 99, 200, 1_000),
            Some("SHRUNK"),
            "a deletion between runs shows in the before-count even though \
             the run re-covered it"
        );
        assert_eq!(
            edge_alarm(0, 0, store::EDGE_VOLUME_CEILING as i64 + 1, u64::MAX / 4),
            Some("SPIKE"),
            "past the order-of-magnitude ceiling"
        );
        assert_eq!(
            edge_alarm(1_000, 1_000, 3_001, 1_000),
            Some("SPIKE"),
            "more than twice the reviewed plan"
        );
    }

    /// Issue 325 step 5: the parser-vs-stock tripwire's comparison, which is the
    /// whole tripwire. Extracted from the job body precisely so it can be
    /// driven directly — the muted-probe bug (Stage 4 Unit 5) was a tripwire
    /// whose only evidence of being wired was that its job ran.
    #[test]
    fn parser_vs_stock_alarms_need_a_baseline_and_a_real_jump() {
        // The floors as MEASURED on prod (job 540) right after the step-4
        // repair, not as estimated: eight rows, not the ~427 first guessed.
        let base = serde_json::json!({
            "no_longer_vat": 7, "vat_country_differs": 1, "vat_refused": 0,
        });

        // Steady state: the residue the step-4 repair deliberately left.
        assert!(parser_vs_stock_alarms(Some(&base), 7, 1, 0, 0).is_empty());

        // A SMALL floor makes the flat tolerance the operative one, and that is
        // the point: at a floor of 7 the alarm trips at 33, so the class cannot
        // quietly regrow by an order of magnitude the way a 10%-of-422 band
        // would have allowed.
        assert!(parser_vs_stock_alarms(Some(&base), 32, 1, 0, 0).is_empty());
        assert_eq!(parser_vs_stock_alarms(Some(&base), 33, 1, 0, 0).len(), 1);

        // Daily growth inside the tolerance is not an alarm. The org layer
        // gains rows every ingest and a few new ambiguous ones are traffic.
        assert!(parser_vs_stock_alarms(Some(&base), 20, 12, 0, 0).is_empty());

        // THE REGRESSION I ACTUALLY SHIPPED, in the direction I shipped it: a
        // tightening that rejected 211 real VAT ids carrying a scheme label
        // (MVA, MWST, USTID) would have pushed `no_longer_vat` from 7 to 218.
        let a = parser_vs_stock_alarms(Some(&base), 218, 1, 0, 0);
        assert_eq!(a.len(), 1, "{a:?}");
        assert!(a[0].contains("no_longer_vat 7 -> 218"), "{a:?}");

        // And the ORIGINAL defect's direction: fresh stock arriving with a
        // country minted out of a word. The 4,206 that stood before the repair
        // would be unmissable.
        let b = parser_vs_stock_alarms(Some(&base), 7, 4206, 0, 0);
        assert_eq!(b.len(), 1, "{b:?}");
        assert!(b[0].contains("vat_country_differs 1 -> 4206"), "{b:?}");

        // `vat_refused` has a floor of ZERO by construction and no tolerance:
        // the gate moving under standing rows is worth one row's notice.
        let c = parser_vs_stock_alarms(Some(&base), 7, 1, 1, 0);
        assert_eq!(c.len(), 1, "{c:?}");
        assert!(c[0].contains("floor is 0"), "{c:?}");

        // No baseline is NOT an alarm. A census that shouted on its own first
        // run would be muted by the second week — except `vat_refused`, which
        // needs no baseline to be meaningful.
        assert!(parser_vs_stock_alarms(None, 999_999, 999_999, 0, 0).is_empty());
        assert_eq!(parser_vs_stock_alarms(None, 0, 0, 3, 0).len(), 1);

        // A baseline missing the keys (an older report shape) behaves like no
        // baseline for those keys rather than reading them as zero — otherwise
        // the first run after this ships would alarm on all 422.
        let old = serde_json::json!({"something_else": 1});
        assert!(parser_vs_stock_alarms(Some(&old), 7, 1, 0, 0).is_empty());

        // Issue 327 rides the same function and needs no baseline either: a
        // shared Austrian GLN under ONE country means the country difference
        // that was keeping an Austrian ministry apart from a Norwegian aviation
        // firm is gone, and R2 keys on `(country, kind, identifier)`.
        let g = parser_vs_stock_alarms(Some(&base), 7, 1, 0, 1);
        assert_eq!(g.len(), 1, "{g:?}");
        assert!(g[0].contains("gln_shared_one_country"), "{g:?}");
        assert!(g[0].contains("merge path that has opened"), "{g:?}");
        // Shared-but-multi-country is the STEADY state, not an alarm — about
        // fifty values sit there and always have.
        assert!(parser_vs_stock_alarms(Some(&base), 7, 1, 0, 0).is_empty());
        assert!(parser_vs_stock_alarms(None, 0, 0, 0, 2).len() == 1, "no baseline needed");

        // A count that FALLS is never an alarm — that is the residue being
        // worked down, which is the outcome this tripwire wants.
        assert!(parser_vs_stock_alarms(Some(&base), 0, 0, 0, 0).is_empty());
    }

    /// Tripwire 6's guard rails (panel round 2): capped and stopped runs
    /// never touch the baseline or the alarm surface; a healthy completed
    /// run anchors the baseline and writes the CLEAR verdict.
    #[tokio::test]
    async fn capped_and_stopped_runs_never_touch_tripwire_6() {
        let sup = Supervisor::new(scratch().await, "archive".into(), reqwest::Client::new());
        sup.db().set_org_edge_baseline(42).await.unwrap();
        let mut wet = serde_json::json!({});
        for (capped, stopped) in [(true, false), (false, true)] {
            let r = store::OrgEdgeScanReport {
                capped,
                stopped,
                total_edges_after: 7,
                ..Default::default()
            };
            let line = sup.apply_edge_tripwire(&r, Some(7), &mut wet, 123).await.unwrap();
            assert!(line.is_empty());
        }
        assert_eq!(sup.db().org_edge_baseline().await.unwrap(), 42, "baseline untouched");
        assert!(
            sup.db().latest_report("org-edge-scan-alarm").await.unwrap().is_none(),
            "alarm surface untouched"
        );
        let r = store::OrgEdgeScanReport {
            total_edges_before: 42,
            total_edges_after: 45,
            would_emit: 45,
            ..Default::default()
        };
        let line = sup.apply_edge_tripwire(&r, Some(45), &mut wet, 124).await.unwrap();
        assert!(line.is_empty(), "healthy run: no message suffix");
        assert_eq!(sup.db().org_edge_baseline().await.unwrap(), 45, "anchored");
        let (alarm, _) =
            sup.db().latest_report("org-edge-scan-alarm").await.unwrap().expect("clear written");
        assert!(alarm.contains("\"clear\":true"), "{alarm}");
    }

    /// Issue 338: the wall suffix must not read a DEFAULT as a disabled wall.
    ///
    /// `project_incremental` returns `Report::default()` without opening a
    /// resolver when nothing is unprojected, and `WallCounts::enabled` is a
    /// `bool` — so the no-op run printed "wall DISABLED this run (key build in
    /// flight or interrupted) — anchor binds took the pre-318 bar" over a run
    /// that opened no resolver, made no bind, and had no build in flight. Two
    /// of its three claims were false.
    ///
    /// The pinned pair is the point: the alarm must stay loud when the wall is
    /// genuinely off, because silencing it is the OTHER error and it is the
    /// worse one.
    #[test]
    fn the_wall_suffix_tells_a_no_op_run_apart_from_a_disabled_wall() {
        let disabled = "wall DISABLED this run";

        // A run that never opened a resolver: every field defaulted.
        let quiet = store::WallCounts::default();
        assert_eq!(
            wall_suffix(&quiet),
            "",
            "a fold with nothing to project must say nothing about a wall it never consulted"
        );

        // A run that DID open one and found the wall unavailable. Still loud.
        let off = store::WallCounts { resolved: true, enabled: false, ..Default::default() };
        assert!(
            wall_suffix(&off).contains(disabled),
            "a genuinely switched-off wall must stay loud: {}",
            wall_suffix(&off)
        );

        // Armed, nothing reached the gate — silent, and NOT for the same reason
        // as the quiet run above, which is why `resolved` has to exist.
        let armed = store::WallCounts { resolved: true, enabled: true, ..Default::default() };
        assert_eq!(wall_suffix(&armed), "", "armed and idle has nothing to report");

        // Anything that actually reached the gate reports its counts.
        let fired = store::WallCounts {
            resolved: true,
            enabled: true,
            anchor_reached: 7,
            asked: 3,
            denied: 1,
            errored: 0,
        };
        let s = wall_suffix(&fired);
        assert!(s.contains("reached 7") && s.contains("asked 3") && s.contains("refused 1"), "{s}");
        assert!(!s.contains(disabled), "{s}");

        // An errored probe is never silent and never "fine" — those binds went
        // through at the pre-318 bar.
        let errored = store::WallCounts { resolved: true, enabled: true, errored: 2, ..Default::default() };
        assert!(wall_suffix(&errored).contains("PROBE(S) ERRORED"), "{}", wall_suffix(&errored));
    }

    /// Issue 313: the weekly pre-dawn tick must actually enqueue all three
    /// of its jobs, and must not stack a second copy of any of them. Until
    /// this test existed the body lived inside a loop that sleeps until a
    /// wall-clock Sunday, so tripwire 6's weekly clock had only ever been
    /// exercised by hand — a wiring slip would have surfaced as silence.
    #[tokio::test]
    async fn the_weekly_report_tick_enqueues_its_seven_jobs_once_each() {
        let sup = Supervisor::new(scratch().await, "archive".into(), reqwest::Client::new());
        sup.run_report_tick().await;
        let kinds: Vec<String> = sup.queued().into_iter().map(|j| j.kind).collect();
        assert_eq!(
            kinds,
            vec![
                // Issue 169: first, because it is statvfs plus one stat and its
                // whole value is an unbroken weekly series.
                "disk-census",
                // Issue 278: read-only, ~9 s, rides with the cheap stamps.
                "ghost-census",
                "data-quality",
                "rehash-probe",
                "build-org-match-keys",
                "org-merge-health",
                "scan-org-match-keys",
            ],
            "issue 315: the key rebuild rides AHEAD of the scan — the queue is FIFO, \
             so this order IS the dependency"
        );
        // The scan rides as a WET run — it IS the tripwire's clock, and a dry
        // one would refresh nothing.
        let scan = sup.queued().into_iter().find(|j| j.kind == "scan-org-match-keys").unwrap();
        assert!(scan.params.contains("(weekly)"), "{}", scan.params);
        assert!(!scan.params.contains("dry-run"), "the weekly scan must be wet: {}", scan.params);
        // So does the build: a dry build measures and stores nothing, which
        // would leave the keyspace exactly as stale as before.
        let build = sup.queued().into_iter().find(|j| j.kind == "build-org-match-keys").unwrap();
        assert!(build.params.contains("(weekly)"), "{}", build.params);
        assert!(!build.params.contains("dry-run"), "the weekly build must be wet: {}", build.params);
        assert!(
            build.params.contains(ingest::crosswalk::NAME_KEY_EPOCH),
            "the build carries its keys epoch as an audit line: {}",
            build.params
        );

        // A second tick with last week's work still queued stacks nothing
        // (the issue-282 already_pending guard).
        sup.run_report_tick().await;
        assert_eq!(sup.queued().len(), 7, "already_pending must stop the double enqueue");
    }

    /// Issue 324: the dry arm of `drop-orphan-satellites` writes its plan as
    /// JSON into `reports`, and the wet arm parses it back into
    /// `(org, lang, key, target)` tuples. Nothing tested that round trip, so a
    /// field rename on either side would have produced a wet run seeing an
    /// EMPTY plan.
    ///
    /// That direction happens to be safe — an empty plan differs from the
    /// fresh candidate set, so the pass refuses rather than over-drops — but
    /// "safe by luck" is not "checked", and the next rename may not be so
    /// lucky. This pins the two halves against each other without a database.
    #[test]
    fn the_drop_plan_json_round_trips_into_the_wet_arms_tuples() {
        // Exactly what the dry arm stores (supervisor.rs, Spec::DropOrphanSatellites).
        let body = serde_json::json!({
            "candidates": 2,
            "rows": [
                {
                    "org": 9610149, "org_name": "Bietergemeinschaft Dobler / Oberall",
                    "lang": "DEU", "name": "Dobler GmbH & Co.KG Bauunternehmung",
                    "key": "dobler gmbh co kg bauunternehmung",
                    "target": 1711879, "target_name": "Dobler GmbH & Co. KG Bauunternehmung",
                },
                // A row whose destination is absent: `target` must survive as
                // None rather than dropping the tuple entirely, or the parity
                // set silently shrinks.
                {
                    "org": 22197923, "org_name": "Bietergemeinschaft IG WEPAE",
                    "lang": "FRA", "name": "AMBERG ENGINEERING",
                    "key": "amberg engineering",
                    "target": null, "target_name": null,
                },
            ],
        })
        .to_string();

        // Exactly what the wet arm parses.
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        let plan: Vec<(i64, String, String, Option<i64>)> = v["rows"]
            .as_array()
            .map(|a| a.as_slice())
            .unwrap_or_default()
            .iter()
            .filter_map(|r| {
                Some((
                    r["org"].as_i64()?,
                    r["lang"].as_str()?.to_owned(),
                    r["key"].as_str()?.to_owned(),
                    r["target"].as_i64(),
                ))
            })
            .collect();

        assert_eq!(plan.len(), 2, "a null target must not drop its whole tuple");
        assert_eq!(
            plan[0],
            (
                9610149,
                "DEU".to_owned(),
                "dobler gmbh co kg bauunternehmung".to_owned(),
                Some(1711879)
            )
        );
        assert_eq!(
            plan[1],
            (22197923, "FRA".to_owned(), "amberg engineering".to_owned(), None)
        );
    }

    /// Issue 313: the job-log depth is an operator-reachable parameter now,
    /// clamped rather than rejected.
    #[tokio::test]
    async fn the_job_log_depth_is_reachable_and_clamped() {
        let sup = Supervisor::new(scratch().await, "archive".into(), reqwest::Client::new());
        for ask in [1i64, 50, JOB_LOG_MAX, JOB_LOG_MAX * 10, 0, -7] {
            sup.ingestion_limited(ask).await.expect("any depth answers");
        }
        assert_eq!(JOB_LOG_MAX.clamp(1, JOB_LOG_MAX), JOB_LOG_MAX);
        assert!(JOB_LOG_MAX > RECENT_RUNS, "the cap must exceed the default to be worth asking for");
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
        // Issue 281: every other batched writer must pin the WAL too, not just the
        // original six — a reparse/merge/backfill checkpoints per batch the same way.
        for kind in [
            "reparse",
            "merge-provisional-orgs",
            "mark-skipped-siblings",
            "repair-swept-siblings",
            "backfill-deadlines",
            "backfill-titles",
            "backfill-org-names",
            "backfill-legacy-adjacency",
            "refold-notices",
            "refold-sections",
            "build-org-match-keys",
            "scan-org-match-keys",
        ] {
            sup.set_current(Some(progress(kind)));
            assert!(sup.heavy_write_in_progress(), "{kind} checkpoints per batch — coverage must stand down");
        }
        // Read-only / trivial-write kinds stay off the belt so coverage still refreshes.
        for kind in ["fetch", "probe", "data-quality", "reveal-recheck", "register-archive", "clear-rebuild-flag"] {
            sup.set_current(Some(progress(kind)));
            assert!(!sup.heavy_write_in_progress(), "{kind} does not pin the WAL — coverage may scan");
        }
        sup.set_current(None);
        assert!(!sup.heavy_write_in_progress(), "idle again");
    }

    /// Issue 282: the scheduler's "never stack two" guard must see a RUNNING
    /// instance, not only a queued one — a popped job lives in `current`.
    #[tokio::test]
    async fn already_pending_sees_the_running_job_not_only_the_queue() {
        let sup = Supervisor::new(scratch().await, "archive".into(), reqwest::Client::new());
        assert!(!sup.already_pending("data-quality"), "idle: nothing pending");
        sup.set_current(Some(progress("data-quality")));
        assert!(
            sup.already_pending("data-quality"),
            "a running instance counts as pending — else the tick stacks a duplicate"
        );
        assert!(!sup.already_pending("rehash-probe"), "a different running kind is not pending");
        sup.set_current(None);
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

        assert_eq!(sup.cancel(b[0]).await, Cancelled::Queued, "a queued job cancels");
        assert_eq!(sup.queued().len(), 1);
        assert_eq!(sup.cancel(b[0]).await, Cancelled::Unknown, "cancelling twice is a no-op");
        assert_eq!(sup.cancel(9_999).await, Cancelled::Unknown, "an unknown id cancels nothing");
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

    /// Issue 256: a queue persist that cannot get the writer must not take the job with
    /// it. On 2026-08-20 a 2.75-hour fold held the writer and the 09:35 daily tick parked
    /// inside this call — no log line, no queued job, no ingest for the day, because the
    /// in-memory push happens only after the persist returns.
    ///
    /// Paused clock and a persist that never completes, so the timeout branch is exercised
    /// in milliseconds and deterministically.
    #[tokio::test]
    async fn a_persist_that_cannot_get_the_writer_gives_up_rather_than_parking() {
        let long = std::time::Duration::from_secs(30);
        let brief = std::time::Duration::from_millis(20);

        // The happy path is unchanged: a persist that lands says so, and the timeout does
        // not make it wait.
        assert!(
            Supervisor::persist_queued(1, "probe", long, std::future::ready(Ok(()))).await,
            "a stored row reports durable"
        );
        // A persist that FAILS is already tolerated by the caller — it just is not durable.
        let failed = std::future::ready(Err(turso::Error::Corrupt("boom".into())));
        assert!(!Supervisor::persist_queued(2, "probe", long, failed).await);

        // And one that never returns gives up rather than parking forever. The timeout is
        // a parameter precisely so this case costs 20 ms instead of the production 30 s —
        // without it the test would hang, which is the failure it pins.
        let never = std::future::pending::<turso::Result<()>>();
        assert!(
            !Supervisor::persist_queued(3, "probe", brief, never).await,
            "a writer held forever must not hold the queue with it"
        );
    }

    /// A restart that finds an EMPTY queue must not restart the id counter, or new
    /// jobs get numbers that already name finished runs in the log. Observed on
    /// prod 2026-08-19: `/admin/jobs` reported a running `id 7` while the log's
    /// newest rows were in the 900s, because `next_id` is in-memory, starts at 1,
    /// and was seeded from the *pending* queue only — which a drained restart
    /// finds empty. The log's high-water mark is the missing floor.
    #[tokio::test]
    async fn a_drained_restart_does_not_reissue_a_spent_job_id() {
        let db = scratch().await;
        let sup = Supervisor::new(db.clone(), "archive".into(), reqwest::Client::new());
        let ids = sup.enqueue_request(&req("project")).await.unwrap();
        let id = ids[0];

        // The job runs and concludes: its log row records the id, its queue row goes.
        db.record_job_run(id as i64, "project", "rebuild=false", 10, 20, "ok", "0 tenders")
            .await
            .unwrap();
        db.remove_job(id as i64).await.unwrap();
        assert!(db.pending_jobs().await.unwrap().is_empty(), "the queue is drained");

        // "Restart" over the same database, with nothing outstanding to recover.
        let restarted = Supervisor::new(db.clone(), "archive".into(), reqwest::Client::new());
        restarted.recover().await;
        let fresh = restarted.enqueue_request(&req("project")).await.unwrap();
        assert!(
            fresh[0] > id,
            "a drained restart reissued id {} (spent by the logged run {id})",
            fresh[0]
        );
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
        assert_eq!(sup.cancel(ids[0]).await, Cancelled::Queued);

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
