//! The Supervisor's recent-run log (issue 16) and durable job queue (issue 21).
//!
//! `job_log` is a bounded history of *finished* ingestion jobs, persisted so the
//! dashboard shows what the importer has been doing across restarts. `job_queue`
//! is the *pending* work — one row per queued or currently-running job — so a
//! restart re-enqueues what was outstanding instead of losing it. Live progress
//! of the running job is in-memory in the app; only these two land here.

use crate::{Db, Value, int, opt_int_of, opt_text_of, t, text};
use model::ingestion::JobRun;

pub const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS job_log (
        id          INTEGER PRIMARY KEY AUTOINCREMENT,
        -- The Supervisor's own job id for this run — the same number the queue
        -- and `/admin/jobs` use while the job is live. `id` is the log's own
        -- append counter and is NOT that number: they are two independent
        -- namespaces, and before this column existed a finished run could not be
        -- correlated with the queue row it came from. Worse, the Supervisor's
        -- counter is in-memory and seeded from the *pending* queue, so a restart
        -- with an empty queue restarted it at 1 and re-used numbers that already
        -- named different runs in this log. Recording it here gives recovery a
        -- durable floor (see `max_logged_job_id`), which makes job ids unique and
        -- monotonic for the life of the database. NULL for rows written before
        -- the column existed.
        job_id      INTEGER,
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

    -- One row per package walk (issue 407). The `[process]` journal line is the
    -- measurement; this is the same line kept where the guard on top of it can
    -- read it, so the floor a walk is judged against is THIS box's own history
    -- for the same (source, kind) — calibrated against real walks, never a
    -- guessed number (see `RATE_FLOOR_DIVISOR`). A few dozen rows a day, never
    -- read on the serving path, and never reset with the tender layer.
    CREATE TABLE IF NOT EXISTS package_rates (
        source     TEXT    NOT NULL,
        kind       TEXT    NOT NULL,   -- 'daily' | 'monthly' | …
        period     TEXT    NOT NULL,   -- the package (e.g. '2026-00180')
        members    INTEGER NOT NULL,
        notices    INTEGER NOT NULL,   -- 0 = a pure-dedup walk, a different regime
        duplicates INTEGER NOT NULL,
        seconds    REAL    NOT NULL,   -- wall time of the walk
        walked_at  INTEGER NOT NULL    -- unix seconds, when the walk finished
    ) STRICT;
    CREATE INDEX IF NOT EXISTS package_rates_walks ON package_rates(source, kind, walked_at);
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
        job_id: i64,
        kind: &str,
        params: &str,
        started_at: i64,
        finished_at: i64,
        outcome: &str,
        counts: &str,
    ) -> turso::Result<()> {
        let conn = self.conn().await;
        conn.execute(
            "INSERT INTO job_log(job_id, kind, params, started_at, finished_at, outcome, counts_json)
             VALUES(?, ?, ?, ?, ?, ?, ?)",
            (
                Value::Integer(job_id),
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
                "SELECT id, job_id, kind, params, started_at, finished_at, outcome, counts_json
                 FROM job_log ORDER BY id DESC LIMIT ?",
                (Value::Integer(limit),),
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(JobRun {
                id: int(&row, 0),
                job_id: opt_int_of(&row, 1),
                kind: text(&row, 2),
                params: text(&row, 3),
                started_at: int(&row, 4),
                finished_at: int(&row, 5),
                outcome: text(&row, 6),
                counts: text(&row, 7),
            });
        }
        Ok(out)
    }

    /// The highest Supervisor job id this log has ever recorded, or `None` if the
    /// log is empty or predates the `job_id` column.
    ///
    /// Recovery needs this as a floor. The pending queue is emptied as jobs
    /// finish, so seeding the Supervisor's counter from it alone lets the counter
    /// fall back to 1 on any restart that happens with no work outstanding — and
    /// then hands the numbers of long-finished runs to new ones. Reading the log's
    /// high-water mark closes that, without ever re-using a number: old rows
    /// answer NULL and are simply not a floor.
    pub async fn max_logged_job_id(&self) -> turso::Result<Option<i64>> {
        let conn = self.reader().await?;
        let mut rows = conn.query("SELECT MAX(job_id) FROM job_log", ()).await?;
        let Some(row) = rows.next().await? else { return Ok(None) };
        Ok(opt_int_of(&row, 0))
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


// ------------------------------------------------------------ package rates (407)

/// One package walk as the `[process]` line reports it (issue 407).
#[derive(Debug, Clone, PartialEq)]
pub struct PackageRate {
    pub source: String,
    pub kind: String,
    pub period: String,
    pub members: u64,
    pub notices: u64,
    pub duplicates: u64,
    /// Wall time of the walk.
    pub seconds: f64,
    /// Unix seconds, when the walk finished.
    pub walked_at: i64,
}

impl PackageRate {
    /// members/s. A walk that took no measurable time reads as 0.0 — such a walk
    /// is never judged (it is below `RATE_MIN_MEMBERS` in practice).
    pub fn members_per_second(&self) -> f64 {
        if self.seconds > 0.0 { self.members as f64 / self.seconds } else { 0.0 }
    }

    /// A walk that inserted notices. A pure-dedup walk (every member already
    /// held) runs an order of magnitude faster — 1,700–15,000 members/s against
    /// 34–680 on 2026-09-17 — and says nothing about the write path issue 404's
    /// regression sat in, so the two regimes are never compared.
    pub fn is_writing(&self) -> bool {
        self.notices > 0
    }
}

/// How many previous writing walks of the same (source, kind) the floor is taken
/// over. Thirty is about six weeks of dailies: long enough to hold a cold-archive
/// week without tilting the median, short enough that a genuine step change in
/// the ingest re-calibrates the floor within a couple of months.
pub const RATE_HISTORY_WINDOW: usize = 30;

/// Below this many previous writing walks there is no floor: the line says
/// `floor pending n/5` and nothing alarms. Five is the smallest history whose
/// median is not one walk.
pub const RATE_FLOOR_MIN_HISTORY: usize = 5;

/// The floor is the history's median divided by this. CALIBRATED 2026-09-19
/// against the eleven writing TED daily walks in the journal (2026-00124 to
/// 00134 and 00180, rev 7726bcb–507ca83): 34.0 to 179.5 members/s, median 87.7,
/// a 5.3× natural spread within ONE (source, kind) — a cold archive read and a
/// package of large members are both legitimately slower per member. Ten clears
/// that spread twice over (the slowest real walk, 34.0, sits 3.9× above the floor
/// the other ten give it) and sits 100× above the 0.12 members/s of issue 404's
/// regression, the defect this guard exists to name the morning it happens.
pub const RATE_FLOOR_DIVISOR: f64 = 10.0;

/// A walk smaller than this is not judged and not part of any history: under a
/// hundred members the wall time is fixed cost (opening the archive, the first
/// statement), not throughput — `doe daily 2026-07-27: 93 members … in 0.0s`.
pub const RATE_MIN_MEMBERS: u64 = 100;

/// What the guard says about one walk (issue 407).
#[derive(Debug, Clone, PartialEq)]
pub enum RateVerdict {
    /// A pure-dedup walk, or one under `RATE_MIN_MEMBERS`: no floor applies.
    NotJudged,
    /// A writing walk with too little history to have a floor yet.
    Pending { have: usize, need: usize },
    /// At or above the floor.
    Clear { floor: f64, median: f64, history: usize },
    /// Under the floor — the rate collapsed relative to this box's own history.
    Alarm { floor: f64, median: f64, history: usize },
}

/// Median of a non-empty slice (the mean of the middle two for an even count).
fn median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).expect("finite rates"));
    let n = sorted.len();
    if n % 2 == 1 { sorted[n / 2] } else { (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0 }
}

/// Judge one walk against the members/s of the PREVIOUS writing walks of its
/// (source, kind) — `history` as `Db::recent_writing_rates` returns it, which
/// must not include the walk itself (a walk is never its own baseline: judge,
/// then record).
pub fn rate_verdict(walk: &PackageRate, history: &[f64]) -> RateVerdict {
    if !walk.is_writing() || walk.members < RATE_MIN_MEMBERS {
        return RateVerdict::NotJudged;
    }
    if history.len() < RATE_FLOOR_MIN_HISTORY {
        return RateVerdict::Pending { have: history.len(), need: RATE_FLOOR_MIN_HISTORY };
    }
    let median = median(history);
    let floor = median / RATE_FLOOR_DIVISOR;
    if walk.members_per_second() < floor {
        RateVerdict::Alarm { floor, median, history: history.len() }
    } else {
        RateVerdict::Clear { floor, median, history: history.len() }
    }
}

impl Db {
    /// Keep one walk's line (issue 407). A write, on the writer like every other
    /// mutation; best-effort at the call site — a lost row costs one walk of
    /// history, never correctness.
    pub async fn record_package_rate(&self, walk: &PackageRate) -> turso::Result<()> {
        let conn = self.conn().await;
        conn.execute(
            "INSERT INTO package_rates(source, kind, period, members, notices, duplicates, seconds, walked_at) \
             VALUES(?, ?, ?, ?, ?, ?, ?, ?)",
            (
                t(&walk.source),
                t(&walk.kind),
                t(&walk.period),
                Value::Integer(walk.members as i64),
                Value::Integer(walk.notices as i64),
                Value::Integer(walk.duplicates as i64),
                Value::Real(walk.seconds),
                Value::Integer(walk.walked_at),
            ),
        )
        .await?;
        Ok(())
    }

    /// members/s of the last `limit` WRITING walks of (source, kind) that were
    /// large enough to judge (`RATE_MIN_MEMBERS`), newest first — the history
    /// `rate_verdict` takes. Pure-dedup walks are a different regime and are
    /// left out (see `PackageRate::is_writing`).
    pub async fn recent_writing_rates(&self, source: &str, kind: &str, limit: usize) -> turso::Result<Vec<f64>> {
        let conn = self.reader().await?;
        let mut rows = conn
            .query(
                "SELECT members, seconds FROM package_rates \
                 WHERE source = ? AND kind = ? AND notices > 0 AND members >= ? AND seconds > 0 \
                 ORDER BY walked_at DESC LIMIT ?",
                (t(source), t(kind), Value::Integer(RATE_MIN_MEMBERS as i64), Value::Integer(limit as i64)),
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            let members = opt_int_of(&row, 0).unwrap_or(0) as f64;
            let seconds = match row.get_value(1) {
                Ok(Value::Real(r)) => r,
                Ok(Value::Integer(i)) => i as f64,
                _ => 0.0,
            };
            if seconds > 0.0 {
                out.push(members / seconds);
            }
        }
        Ok(out)
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

        db.record_job_run(7, "process", "ted daily 2026-00136", 100, 160, "ok", "42 notices")
            .await
            .unwrap();
        db.record_job_run(8, "project", "rebuild=false", 200, 205, "error", "db: locked")
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

        db.record_job_run(7, "process", "ted daily 2026-00136", 100, 160, "ok", "42 notices")
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
