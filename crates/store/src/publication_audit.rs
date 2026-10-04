//! The publication-id audit ledger (issue 477 unit 3).
//!
//! A source whose notice ids are a per-year sequence (FTS: `NNNNNN-YYYY`,
//! docs/research/uk-fts.md §7) says how many notices it issued: a year's
//! highest id. Every id below it that the corpus does not hold is either a
//! notice the fetch lost (it exists on the API) or one that was never published
//! (withdrawn before publication: the API answers 404 or an empty package). The
//! `audit-fts-ids` job asks the API for each such id BY ID and records the
//! answer here, one row per id, so the coverage denominator can be "highest id
//! minus the ids shown absent" instead of nothing (the dashboard read FTS as
//! complete at 95.7 %, issue 477).
//!
//! The issue's design named the table `absent_publications`; it holds every
//! verdict (`absent`, `present`, `error`) because the job's resume state IS the
//! ledger — a re-run re-asks only `error` rows and ids with no row — and a
//! `present` row carries the release date a refetch needs. The absent set is
//! `WHERE verdict = 'absent'`.
//!
//! Notice-layer bookkeeping, like `fetches`: never reset with the tender layer,
//! and never rebuilt from the archive (the archive does not hold the answers).

use crate::{Db, Value, int, opt_int, opt_int_of, opt_text, opt_text_of, t, text};

pub const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS publication_audit (
        source         TEXT    NOT NULL,           -- 'fts'
        publication_id TEXT    NOT NULL,           -- '009911-2021'
        year           INTEGER NOT NULL,           -- the id's own year, 2021
        seq            INTEGER NOT NULL,           -- the id's sequence number, 9911
        verdict        TEXT    NOT NULL,           -- 'absent' | 'present' | 'error' | 'quarantined'
        -- The by-id request's HTTP status; NULL when no response arrived (a
        -- transport error after the retries).
        http_status    INTEGER,
        -- For 'present': the release `date` as served (the earliest when the
        -- notice has several releases), its UK civil day, the first release's
        -- ocid and how many releases of the id the answer carried.
        published      TEXT,
        published_day  TEXT,
        ocid           TEXT,
        releases       INTEGER,
        -- What decided the verdict, in words: '404', 'empty package', the error.
        detail         TEXT,
        checked_at     INTEGER NOT NULL,           -- unix seconds, the latest probe
        attempts       INTEGER NOT NULL DEFAULT 1, -- probes of this id so far
        PRIMARY KEY (source, publication_id)
    ) STRICT;
";

/// One probed id, as the ledger holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicationAudit {
    pub source: String,
    pub publication_id: String,
    pub year: i64,
    pub seq: i64,
    pub verdict: String,
    pub http_status: Option<i64>,
    pub published: Option<String>,
    pub published_day: Option<String>,
    pub ocid: Option<String>,
    pub releases: Option<i64>,
    pub detail: Option<String>,
    pub checked_at: i64,
    pub attempts: i64,
}

impl Db {
    /// Record one probe of an id: insert, or replace the previous verdict and
    /// count the attempt. Idempotent per answer — the row is the latest probe.
    pub async fn record_publication_audit(&self, row: &PublicationAudit) -> turso::Result<()> {
        let conn = self.conn().await;
        conn.execute(
            "INSERT INTO publication_audit(source, publication_id, year, seq, verdict, http_status,
                 published, published_day, ocid, releases, detail, checked_at, attempts)
             VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1)
             ON CONFLICT(source, publication_id) DO UPDATE SET
                 year = excluded.year, seq = excluded.seq, verdict = excluded.verdict,
                 http_status = excluded.http_status, published = excluded.published,
                 published_day = excluded.published_day, ocid = excluded.ocid,
                 releases = excluded.releases, detail = excluded.detail,
                 checked_at = excluded.checked_at, attempts = publication_audit.attempts + 1",
            (
                t(&row.source),
                t(&row.publication_id),
                Value::Integer(row.year),
                Value::Integer(row.seq),
                t(&row.verdict),
                opt_int(row.http_status),
                opt_text(row.published.as_deref()),
                opt_text(row.published_day.as_deref()),
                opt_text(row.ocid.as_deref()),
                opt_int(row.releases),
                opt_text(row.detail.as_deref()),
                Value::Integer(row.checked_at),
            ),
        )
        .await?;
        Ok(())
    }

    /// Every ledger row of `source`, by (year, seq). A few thousand rows at most
    /// (the ids below each year's highest that the corpus does not hold).
    pub async fn publication_audits(&self, source: &str) -> turso::Result<Vec<PublicationAudit>> {
        let conn = self.reader().await?;
        let mut rows = conn
            .query(
                "SELECT source, publication_id, year, seq, verdict, http_status, published, published_day,
                        ocid, releases, detail, checked_at, attempts
                   FROM publication_audit WHERE source = ? ORDER BY year, seq",
                (t(source),),
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(PublicationAudit {
                source: text(&row, 0),
                publication_id: text(&row, 1),
                year: int(&row, 2),
                seq: int(&row, 3),
                verdict: text(&row, 4),
                http_status: opt_int_of(&row, 5),
                published: opt_text_of(&row, 6),
                published_day: opt_text_of(&row, 7),
                ocid: opt_text_of(&row, 8),
                releases: opt_int_of(&row, 9),
                detail: opt_text_of(&row, 10),
                checked_at: int(&row, 11),
                attempts: int(&row, 12),
            });
        }
        Ok(out)
    }

    /// The `publication_id` of every notice row of `source`, one per row (an id
    /// with several releases repeats; the caller dedups). A range of the
    /// `UNIQUE(source, publication_id, content_hash)` index, so it reads index
    /// entries only: ~330k short keys for FTS, never a notice payload.
    ///
    /// Issue 477 unit 3 reads the held ids from here rather than from the
    /// archived zips' member names (the issue's Verify): the zips are ~90 files
    /// and gigabytes to open, while this is what the dashboard counts and what
    /// a refetch's `process` adds to. A member the processor quarantined without
    /// a notice row has no row here; the audit finds those through
    /// [`Db::quarantined_member_paths`] and records them `quarantined` without a
    /// request (a `present` verdict would never close: the refetch dedups by hash
    /// and adds no row).
    pub async fn publication_ids(&self, source: &str) -> turso::Result<Vec<String>> {
        let conn = self.reader().await?;
        let mut rows = conn.query("SELECT publication_id FROM notices WHERE source = ?", (t(source),)).await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(text(&row, 0));
        }
        Ok(out)
    }

    /// The `member_path` of every quarantine row whose package is of `source`
    /// (issue 477 unit 3: an archived member the processor quarantined, which the
    /// audit counts as accounted for). A scan of `quarantine` (no `fetch_id`
    /// index), so the audit job reads it once per run and the dashboard never does.
    pub async fn quarantined_member_paths(&self, source: &str) -> turso::Result<Vec<String>> {
        let conn = self.reader().await?;
        let mut rows = conn
            .query(
                "SELECT member_path FROM quarantine WHERE fetch_id IN (SELECT id FROM fetches WHERE source = ?)",
                (t(source),),
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(text(&row, 0));
        }
        Ok(out)
    }

    /// Turn the `absent` rows of `ids` into `error` rows with `detail`, keeping
    /// their attempt count: a control request showed the endpoint was not
    /// answering for a known id, so those 404s decided nothing (issue 477 unit 3).
    pub async fn demote_publication_audits(&self, source: &str, ids: &[String], detail: &str) -> turso::Result<()> {
        let conn = self.conn().await;
        for id in ids {
            conn.execute(
                "UPDATE publication_audit SET verdict = 'error', detail = ?
                  WHERE source = ? AND publication_id = ? AND verdict = 'absent'",
                (t(detail), t(source), t(id)),
            )
            .await?;
        }
        Ok(())
    }

    /// The packages `(kind, period)` that brought in the notice rows of
    /// `(source, publication_id)` — where a held id was LISTED, which is what
    /// locates a missing neighbour's day (issue 477 unit 3: the window selects
    /// on a hidden publication instant, not the release `date`). Two point reads
    /// per row, never a join the planner could drive from the wrong side
    /// (`notice_counts_by_profile_year`'s lesson).
    pub async fn publication_packages(
        &self,
        source: &str,
        publication_id: &str,
    ) -> turso::Result<Vec<(String, String)>> {
        let conn = self.reader().await?;
        let mut fetch_ids = Vec::new();
        let mut rows = conn
            .query(
                "SELECT fetch_id FROM notices WHERE source = ? AND publication_id = ?",
                (t(source), t(publication_id)),
            )
            .await?;
        while let Some(row) = rows.next().await? {
            fetch_ids.push(int(&row, 0));
        }
        drop(rows);
        let mut out = Vec::new();
        for id in fetch_ids {
            let mut rows = conn.query("SELECT kind, period FROM fetches WHERE id = ?", (Value::Integer(id),)).await?;
            if let Some(row) = rows.next().await? {
                let package = (text(&row, 0), text(&row, 1));
                if !out.contains(&package) {
                    out.push(package);
                }
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &str, verdict: &str, at: i64) -> PublicationAudit {
        PublicationAudit {
            source: "fts".into(),
            publication_id: id.into(),
            year: 2021,
            seq: id[..6].parse().unwrap(),
            verdict: verdict.into(),
            http_status: Some(404),
            published: None,
            published_day: None,
            ocid: None,
            releases: None,
            detail: Some("404".into()),
            checked_at: at,
            attempts: 1,
        }
    }

    /// Issue 477 unit 3: a re-probe replaces the verdict and counts the attempt,
    /// so the ledger is the latest answer per id, never a second row.
    #[tokio::test]
    async fn a_reprobe_replaces_the_verdict_and_counts_the_attempt() {
        let path = format!("/tmp/tender-db-publication-audit-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        db.record_publication_audit(&row("000003-2021", "error", 10)).await.unwrap();
        db.record_publication_audit(&row("000001-2021", "absent", 11)).await.unwrap();
        let mut present = row("000003-2021", "present", 20);
        present.http_status = Some(200);
        present.published = Some("2021-01-04T09:00:00Z".into());
        present.published_day = Some("2021-01-04".into());
        db.record_publication_audit(&present).await.unwrap();
        let rows = db.publication_audits("fts").await.unwrap();
        assert_eq!(rows.len(), 2, "one row per id");
        assert_eq!(rows[0].publication_id, "000001-2021", "ordered by (year, seq)");
        assert_eq!(rows[1].verdict, "present");
        assert_eq!(rows[1].published_day.as_deref(), Some("2021-01-04"));
        assert_eq!(rows[1].checked_at, 20);
        assert_eq!(rows[1].attempts, 2, "the error probe and the present one");
        assert!(db.publication_audits("ted").await.unwrap().is_empty());

        // A demotion turns an absent into an error and leaves other verdicts alone.
        db.demote_publication_audits("fts", &["000001-2021".into(), "000003-2021".into()], "control failed")
            .await
            .unwrap();
        let rows = db.publication_audits("fts").await.unwrap();
        assert_eq!((rows[0].verdict.as_str(), rows[0].detail.as_deref(), rows[0].attempts), ("error", Some("control failed"), 1));
        assert_eq!(rows[1].verdict, "present", "only an absent is demoted");
        let _ = std::fs::remove_file(&path);
    }
}
