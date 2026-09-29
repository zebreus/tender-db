//! The weekly `ANALYZE` of the read-path tables (issue 429, from issue 428's
//! measurement on a snapshot copy).
//!
//! turso keeps no statistics unless asked, and without them its planner guesses
//! row counts. That guess is what made issue 421's plain JOIN drive from a version
//! table instead of `tenders`. `ANALYZE` fixes the guess for the tables listed
//! here; 428 measured the other tables and left them out for the reasons on
//! [`ANALYZE_TABLES`].

use crate::{Db, int};

/// The tables the `analyze` job gathers statistics for — and the only ones it
/// may touch. 428 measured what statistics do to each class, and three stay out:
///
/// * `organizations`: the resolver's `(name_norm, country)` lookup went from
///   0.001 s to 6.19 s cold on a skewed name once the table had statistics (the
///   planner judged the name index unselective and scanned). The job also deletes
///   any `sqlite_stat1` row for it, so a stray manual `ANALYZE` cannot bring that
///   regression back.
/// * the raw `notice_*` layer and `changes`: 1 h 44 min of `ANALYZE` for no
///   measured plan benefit.
/// * the small operational tables (jobs, reports, accounts, …): with statistics
///   their seeks became scans.
pub const ANALYZE_TABLES: [&str; 24] = [
    "tenders",
    "lots",
    "tender_versions",
    "organization_names",
    "organization_mentions",
    "tender_version_dates",
    "tender_version_classifications",
    "tender_version_parties",
    "tender_version_texts",
    "tender_version_amounts",
    "tender_version_lots",
    "tender_version_bids",
    "tender_version_bid_parties",
    "tender_version_contracts",
    "tender_version_lot_results",
    "tender_version_result_winners",
    "tender_version_result_stats",
    "tender_version_lot_group_members",
    "lot_results",
    "bids",
    "contracts",
    "currency_rates",
    "quarantine",
    "notices",
];

impl Db {
    /// `ANALYZE <table>` on the writer, as its own statement, returning the
    /// seconds it took. One table per call so the writer is released between
    /// tables — the longest, `tender_version_parties`, took 242 s on prod data —
    /// and a stopped job costs at most the table in flight. A table outside
    /// [`ANALYZE_TABLES`] is refused, never analyzed.
    pub async fn analyze_table(&self, table: &str) -> Result<f64, String> {
        if !ANALYZE_TABLES.contains(&table) {
            return Err(format!("{table} is not in ANALYZE_TABLES (issue 429) — refused"));
        }
        let conn = self.conn().await;
        let started = std::time::Instant::now();
        conn.execute(&format!("ANALYZE {table}"), ()).await.map_err(|e| format!("ANALYZE {table}: {e}"))?;
        Ok(started.elapsed().as_secs_f64())
    }

    /// The job's last step. Deletes any `sqlite_stat1` row for `organizations`,
    /// returning how many there were, then makes every pooled reader pick the new
    /// statistics up.
    ///
    /// The second half is not optional. A reader opened before the `ANALYZE`
    /// keeps planning with the statistics it loaded at open until the schema
    /// changes (measured: `tests/analyze_stats_pickup.rs`), so without it the
    /// server would run two plans for one query until its next restart. A
    /// throwaway `CREATE TABLE` + `DROP TABLE` is that schema change.
    pub async fn finish_analyze(&self) -> Result<i64, String> {
        let conn = self.conn().await;
        let mut rows = conn
            .query("SELECT COUNT(*) FROM sqlite_schema WHERE type = 'table' AND name = 'sqlite_stat1'", ())
            .await
            .map_err(|e| e.to_string())?;
        let has_stats = match rows.next().await.map_err(|e| e.to_string())? {
            Some(row) => int(&row, 0) > 0,
            None => false,
        };
        drop(rows);
        let removed = if has_stats {
            conn.execute("DELETE FROM sqlite_stat1 WHERE tbl = 'organizations'", ())
                .await
                .map_err(|e| format!("drop organizations statistics: {e}"))? as i64
        } else {
            0
        };
        conn.execute_batch("CREATE TABLE analyze_refresh_bump (x INTEGER); DROP TABLE analyze_refresh_bump;")
            .await
            .map_err(|e| format!("schema bump: {e}"))?;
        Ok(removed)
    }
}
