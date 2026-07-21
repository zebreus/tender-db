//! The Supervisor's recent-run log (issue 16).
//!
//! A bounded history of finished ingestion jobs, persisted so the dashboard
//! shows what the importer has been doing across restarts. Live progress of the
//! *running* job is in-memory in the app; only finished runs land here.

use crate::{Db, Value, int, t, text};
use model::ingestion::JobRun;
use turso::Connection;

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
";

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
}

/// The most recent finished runs, newest first. A free function over a borrowed
/// connection (like [`crate::read`]'s queries) rather than a `Db` method, so the
/// dashboard reads it through the reader pool — never queuing behind ingestion
/// on the writer mutex, which a heavy process/project batch holds for the length
/// of its transaction (issue 20).
pub async fn recent_job_runs(conn: &Connection, limit: i64) -> turso::Result<Vec<JobRun>> {
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

#[cfg(test)]
mod tests {
    use super::recent_job_runs;
    use crate::Db;
    use std::time::Instant;

    #[tokio::test]
    async fn job_log_round_trips_newest_first() {
        let path = format!("/tmp/tender-db-joblog-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        let readers = db.readers(1).unwrap();
        let read = || async { recent_job_runs(&readers.get().await.unwrap(), 10).await.unwrap() };

        assert!(read().await.is_empty());

        db.record_job_run("process", "ted daily 2026-00136", 100, 160, "ok", "42 notices")
            .await
            .unwrap();
        db.record_job_run("project", "rebuild=false", 200, 205, "error", "db: locked")
            .await
            .unwrap();

        let runs = read().await;
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].kind, "project", "newest first");
        assert_eq!(runs[0].outcome, "error");
        assert_eq!(runs[1].params, "ted daily 2026-00136");
        assert_eq!(runs[1].counts, "42 notices");

        // The limit is honoured.
        assert_eq!(recent_job_runs(&readers.get().await.unwrap(), 1).await.unwrap().len(), 1);

        let _ = std::fs::remove_file(&path);
    }

    /// The regression for issue 20: while the writer connection is held in an
    /// open transaction — exactly what a heavy process/project batch does
    /// (`BEGIN IMMEDIATE … COMMIT`) — the dashboard's recent-runs read still
    /// returns promptly, because it goes through a reader connection over WAL
    /// rather than queuing on the writer mutex. Routed through the writer (the
    /// old code) this would block for the whole batch, which is the ~23s stall.
    #[tokio::test]
    async fn recent_runs_read_while_the_writer_is_busy() {
        let path = format!("/tmp/tender-db-joblog-busy-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        let readers = db.readers(1).unwrap();

        db.record_job_run("process", "ted daily 2026-00136", 100, 160, "ok", "42 notices")
            .await
            .unwrap();

        // Take and hold the writer in a live transaction; nothing routed through
        // it could make progress until COMMIT.
        let writer = db.conn().await;
        writer.execute("BEGIN IMMEDIATE", ()).await.unwrap();

        // Reading through a separate reader connection cannot touch the writer
        // mutex, so this returns while the transaction is still open. Routed
        // through the writer (the old code) it would deadlock against the guard
        // held above — the acute form of the ~23s production stall.
        let reader = readers.get().await.unwrap();
        let started = Instant::now();
        let runs = recent_job_runs(&reader, 10).await.unwrap();
        assert!(started.elapsed().as_secs() < 1, "the read must not queue behind the writer");
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].counts, "42 notices");

        writer.execute("COMMIT", ()).await.unwrap();
        let _ = std::fs::remove_file(&path);
    }
}
