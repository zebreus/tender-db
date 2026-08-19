//! What the ingestion Supervisor exposes — the live job it is running, what is
//! queued behind it, and the recent-run log.
//!
//! Report types, like [`crate::dashboard`]: the server resolves everything
//! (progress counters, outcomes) and the wasm client renders it. The admin API
//! (`GET /admin/jobs`) and the dashboard's Ingestion panel serialise the same
//! [`Ingestion`] shape, so the two never disagree about what the importer is
//! doing.

use serde::{Deserialize, Serialize};

/// One snapshot of the Supervisor: the job in flight, the queue, and history.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Ingestion {
    /// The job currently executing, with its live progress — `None` when idle.
    pub current: Option<JobProgress>,
    /// Jobs waiting to run, in execution order (one job at a time — the writer
    /// is single anyway).
    pub queued: Vec<QueuedJob>,
    /// The most recent finished runs, newest first (persisted in `job_log`).
    pub recent: Vec<JobRun>,
    /// The server's clock when this snapshot was taken, unix seconds. The client
    /// derives the running job's elapsed time and throughput from it rather than
    /// reading its own clock (see [`crate::dashboard`] for the same discipline).
    pub measured_at: i64,
}

/// The live progress of the running job. Counters advance as the job walks its
/// packages, so a progress bar is `packages_done / packages_total` with the
/// current package's members underneath, and `notices / (now - started_at)` is
/// the throughput.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JobProgress {
    pub id: u64,
    /// `fetch` | `process` | `project` | `probe`.
    pub kind: String,
    /// Human summary of the job's parameters, e.g. `ted daily 2026-00136`.
    pub params: String,
    /// When execution started, unix seconds.
    pub started_at: i64,
    /// The package (period) being worked right now, if the job walks packages.
    pub package: Option<String>,
    pub packages_done: u64,
    pub packages_total: u64,
    /// Members processed in the current package, and how many it holds.
    pub members_done: u64,
    pub members_total: u64,
    /// Notices written so far across the whole job.
    pub notices: u64,
    /// Members that were already ingested and deduped away (issue 33). During a
    /// backfill re-walk this climbs while `notices` stays flat — the signal the
    /// dashboard uses to say "re-walking" instead of a bare 0.0 notices/s.
    pub duplicates: u64,
    /// What the job is doing right now, for kinds whose work is not a package
    /// walk (issue 65). `None` for jobs that only move the counters above.
    ///
    /// This exists because the counters above are the WRONG SHAPE for several
    /// real jobs, and overloading them lies. The projection walks phases, not
    /// packages, and its multi-hour pre-pass moved nothing at all; a chunked
    /// backfill sweeps an id range, and issue 228 declined to put that cursor in
    /// `members_done` precisely because an id in a field every other job fills
    /// with a count is a dishonest signal. So: a separate, honestly-typed field
    /// rather than a reinterpretation of an existing one.
    pub phase: Option<Phase>,
}

/// A named stage of a running job, with optional progress through it (issue 65).
///
/// `done`/`total` are deliberately optional and deliberately NOT a fraction: a
/// phase that cannot cheaply know its total (a scan whose end is only provable
/// by reaching it) reports `done` alone and still shows movement, which is the
/// whole point — the failure this fixes is a reader unable to tell a working job
/// from a wedged one. A phase that knows neither still names itself, which beats
/// dead air.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Phase {
    /// Short stable name — `pre-pass`, `folding`, `sweeping`, `index-build`.
    /// Stable because an operator learns these and `/metrics` labels by them.
    pub name: String,
    /// Units completed in this phase, in whatever unit `detail` names.
    pub done: Option<u64>,
    /// The phase's end, when it is known up front without extra work.
    pub total: Option<u64>,
    /// One human line: what the unit is, and any position that is not a count
    /// (e.g. `id 11,400,000 of 28,251,412` for an id-windowed sweep).
    pub detail: String,
    /// When this phase record was last written — so a reader can tell a phase
    /// that is progressing slowly from one whose reporter has itself stopped.
    /// Without it, a stale phase and a slow phase look identical, which is the
    /// same ambiguity in a new place.
    pub updated_at: i64,
}

/// A job still in the queue — identity plus what it will do, so the operator can
/// cancel it by id (`DELETE /admin/jobs/{id}`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QueuedJob {
    pub id: u64,
    pub kind: String,
    pub params: String,
}

/// One finished run from the persisted `job_log` — outcome and counts kept so
/// the dashboard shows history across restarts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JobRun {
    pub id: i64,
    /// The Supervisor's job id for this run — the number `/admin/jobs` showed
    /// while it was live. `None` for runs logged before the column existed;
    /// `id` above is the log's own append counter, a different namespace.
    pub job_id: Option<i64>,
    pub kind: String,
    pub params: String,
    pub started_at: i64,
    pub finished_at: i64,
    /// `ok` | `error`.
    pub outcome: String,
    /// A human one-liner of what the run did (counts, or the error).
    pub counts: String,
}
