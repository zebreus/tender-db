//! The Supervisor's recent-run log (issue 16) and durable job queue (issue 21).
//!
//! `job_log` is a bounded history of *finished* ingestion jobs, persisted so the
//! dashboard shows what the importer has been doing across restarts. `job_queue`
//! is the *pending* work — one row per queued or currently-running job — so a
//! restart re-enqueues what was outstanding instead of losing it. Live progress
//! of the running job is in-memory in the app; only these two land here.

use crate::{Db, Value, int, opt_text_of, t, text};
use model::ingestion::JobRun;

pub const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS job_log (
        id          INTEGER PRIMARY KEY AUTOINCREMENT,
        kind        TEXT NOT NULL,
        params      TEXT NOT NULL,
        started_at  INTEGER NOT NULL, -- unix seconds
        finished_at INTEGER NOT NULL, -- unix seconds
        outcome     TEXT NOT NULL,    -- 'ok' | 'error'
        counts_json TEXT NOT NULL     -- human one-liner / counts summary
    ) STRICT;

    -- The durable job queue (issue 21). One row per outstanding job, keyed by the
    -- Supervisor's own monotonic id. `spec` is an opaque serialization the app
    -- owns — the store never interprets it — while `kind`/`params` mirror the
    -- display identity so the queue is legible in a plain SELECT. A row lives from
    -- enqueue until the job concludes (ok or error); the job that was running when
    -- the process died is simply the lowest surviving id, so ordering by id brings
    -- it back at the front to be re-run (re-walks are idempotent).
    CREATE TABLE IF NOT EXISTS job_queue (
        id       INTEGER PRIMARY KEY, -- the Supervisor's job id (app-assigned)
        kind     TEXT NOT NULL,
        params   TEXT NOT NULL,
        spec     TEXT NOT NULL,       -- opaque app payload (serialized job Spec)
        -- The resume cursor (issue 32): the last package a process job fully
        -- completed, updated as each finishes. On restart the job resumes just
        -- after it instead of re-walking from the start. NULL for a fresh job and
        -- for kinds that have no per-package progress.
        progress TEXT
    ) STRICT;
";

/// One outstanding job as persisted in `job_queue`. `spec` is the app's opaque
/// payload; the store round-trips it verbatim. `progress` is the resume cursor
/// (issue 32).
pub struct QueuedJobRow {
    pub id: i64,
    pub kind: String,
    pub params: String,
    pub spec: String,
    pub progress: Option<String>,
}

impl Db {
    /// Append one finished run. `counts` is a human summary (or the error text)
    /// — the panel shows it verbatim.
    pub async fn record_job_run(
        &self,
        kind: &str,
        params: &str,
        started_at: i64,
        finished_at: i64,
        outcome: &str,
        counts: &str,
    ) -> turso::Result<()> {
        let conn = self.conn().await;
        conn.execute(
            "INSERT INTO job_log(kind, params, started_at, finished_at, outcome, counts_json)
             VALUES(?, ?, ?, ?, ?, ?)",
            (
                t(kind),
                t(params),
                Value::Integer(started_at),
                Value::Integer(finished_at),
                t(outcome),
                t(counts),
            ),
        )
        .await?;
        Ok(())
    }

    /// The most recent finished runs, newest first. Read through the reader pool
    /// (`self.reader()`), never the writer, so the dashboard/admin log never
    /// queues behind an ingestion job holding the writer for the length of its
    /// transaction (issue 20).
    pub async fn recent_job_runs(&self, limit: i64) -> turso::Result<Vec<JobRun>> {
        let conn = self.reader().await?;
        let mut rows = conn
            .query(
                "SELECT id, kind, params, started_at, finished_at, outcome, counts_json
                 FROM job_log ORDER BY id DESC LIMIT ?",
                (Value::Integer(limit),),
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(JobRun {
                id: int(&row, 0),
                kind: text(&row, 1),
                params: text(&row, 2),
                started_at: int(&row, 3),
                finished_at: int(&row, 4),
                outcome: text(&row, 5),
                counts: text(&row, 6),
            });
        }
        Ok(out)
    }

    // ------------------------------------------------------------ durable queue

    /// Persist one outstanding job (issue 21). A write — the queue is authored
    /// on the writer like every other mutation. `spec` is the app's opaque
    /// payload, stored verbatim.
    pub async fn enqueue_job(&self, id: i64, kind: &str, params: &str, spec: &str) -> turso::Result<()> {
        let conn = self.conn().await;
        conn.execute(
            "INSERT INTO job_queue(id, kind, params, spec) VALUES(?, ?, ?, ?)",
            (Value::Integer(id), t(kind), t(params), t(spec)),
        )
        .await?;
        Ok(())
    }

    /// Drop a job from the durable queue — on completion or cancellation. A job
    /// killed mid-run never gets here, so its row survives for recovery.
    pub async fn remove_job(&self, id: i64) -> turso::Result<()> {
        let conn = self.conn().await;
        conn.execute("DELETE FROM job_queue WHERE id = ?", (Value::Integer(id),)).await?;
        Ok(())
    }

    /// Every outstanding job, oldest id first — the order the Supervisor rebuilds
    /// its in-memory queue in at startup. The interrupted running job is the
    /// lowest id, so it lands back at the front.
    pub async fn pending_jobs(&self) -> turso::Result<Vec<QueuedJobRow>> {
        let conn = self.reader().await?;
        let mut rows = conn
            .query("SELECT id, kind, params, spec, progress FROM job_queue ORDER BY id", ())
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(QueuedJobRow {
                id: int(&row, 0),
                kind: text(&row, 1),
                params: text(&row, 2),
                spec: text(&row, 3),
                progress: opt_text_of(&row, 4),
            });
        }
        Ok(out)
    }

    /// Advance a job's resume cursor to the package it just finished (issue 32).
    /// Written after the package's members have all committed, so a restart
    /// resumes at the next package and never skips one left half-done.
    pub async fn record_job_progress(&self, id: i64, package: &str) -> turso::Result<()> {
        let conn = self.conn().await;
        conn.execute(
            "UPDATE job_queue SET progress = ? WHERE id = ?",
            (t(package), Value::Integer(id)),
        )
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::Db;
    use std::time::Instant;

    #[tokio::test]
    async fn job_log_round_trips_newest_first() {
        let path = format!("/tmp/tender-db-joblog-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();

        assert!(db.recent_job_runs(10).await.unwrap().is_empty());

        db.record_job_run("process", "ted daily 2026-00136", 100, 160, "ok", "42 notices")
            .await
            .unwrap();
        db.record_job_run("project", "rebuild=false", 200, 205, "error", "db: locked")
            .await
            .unwrap();

        let runs = db.recent_job_runs(10).await.unwrap();
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].kind, "project", "newest first");
        assert_eq!(runs[0].outcome, "error");
        assert_eq!(runs[1].params, "ted daily 2026-00136");
        assert_eq!(runs[1].counts, "42 notices");

        // The limit is honoured.
        assert_eq!(db.recent_job_runs(1).await.unwrap().len(), 1);

        let _ = std::fs::remove_file(&path);
    }

    /// The durable queue (issue 21): rows persist in id order, the opaque spec
    /// round-trips verbatim, and removal takes exactly one job.
    #[tokio::test]
    async fn job_queue_persists_and_removes() {
        let path = format!("/tmp/tender-db-jobqueue-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();

        assert!(db.pending_jobs().await.unwrap().is_empty());

        db.enqueue_job(5, "process", "ted daily (all)", r#"{"Process":{"source":"ted"}}"#).await.unwrap();
        db.enqueue_job(6, "project", "rebuild=false", r#"{"Project":{"rebuild":false}}"#).await.unwrap();

        let pending = db.pending_jobs().await.unwrap();
        assert_eq!(pending.len(), 2);
        assert_eq!((pending[0].id, pending[0].kind.as_str()), (5, "process"), "oldest id first");
        assert_eq!(pending[0].spec, r#"{"Process":{"source":"ted"}}"#, "spec round-trips verbatim");
        assert_eq!(pending[0].progress, None, "a fresh job has no resume cursor");
        assert_eq!(pending[1].id, 6);

        // The resume cursor (issue 32) persists and comes back on the next read.
        db.record_job_progress(5, "2004-07").await.unwrap();
        let pending = db.pending_jobs().await.unwrap();
        assert_eq!(pending[0].progress.as_deref(), Some("2004-07"), "cursor round-trips");

        db.remove_job(5).await.unwrap();
        let pending = db.pending_jobs().await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, 6, "only the removed job is gone");

        let _ = std::fs::remove_file(&path);
    }

    /// The regression for issue 20: while the writer connection is held in an
    /// open transaction — exactly what a heavy process/project batch does
    /// (`BEGIN IMMEDIATE … COMMIT`) — a read-only accessor still returns
    /// promptly, because `Db::reader()` hands it a pooled reader connection over
    /// WAL rather than the writer mutex. Routed through the writer (the old code)
    /// this deadlocks against the guard held below — the acute form of the ~23s
    /// production stall that hit every dashboard/admin read.
    #[tokio::test]
    async fn reads_do_not_block_on_a_held_writer() {
        let path = format!("/tmp/tender-db-joblog-busy-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();

        db.record_job_run("process", "ted daily 2026-00136", 100, 160, "ok", "42 notices")
            .await
            .unwrap();

        // Take and hold the writer in a live transaction; nothing routed through
        // it could make progress until COMMIT.
        let writer = db.conn().await;
        writer.execute("BEGIN IMMEDIATE", ()).await.unwrap();

        let started = Instant::now();
        let runs = db.recent_job_runs(10).await.unwrap();
        assert!(started.elapsed().as_secs() < 1, "the read must not queue behind the writer");
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].counts, "42 notices");

        writer.execute("COMMIT", ()).await.unwrap();
        let _ = std::fs::remove_file(&path);
    }
}
