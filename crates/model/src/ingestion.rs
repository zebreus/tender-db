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
    pub kind: String,
    pub params: String,
    pub started_at: i64,
    pub finished_at: i64,
    /// `ok` | `error`.
    pub outcome: String,
    /// A human one-liner of what the run did (counts, or the error).
    pub counts: String,
}
