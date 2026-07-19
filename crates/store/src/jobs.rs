//! The Supervisor's recent-run log (issue 16).
//!
//! A bounded history of finished ingestion jobs, persisted so the dashboard
//! shows what the importer has been doing across restarts. Live progress of the
//! *running* job is in-memory in the app; only finished runs land here.

use crate::{Db, Value, int, t, text};
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

    /// The most recent finished runs, newest first.
    pub async fn recent_job_runs(&self, limit: i64) -> turso::Result<Vec<JobRun>> {
        let conn = self.conn().await;
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
}

#[cfg(test)]
mod tests {
    use crate::Db;

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
}
