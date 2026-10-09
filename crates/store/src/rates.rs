//! The EUR-pivot exchange-rate reference table (ADR-0014).
//!
//! One row per (currency, day): the ECU daily series 1993–1998 chained 1:1 to
//! the ECB reference rates 1999→, plus the irrevocable euro conversion rates as
//! `'irrevocable'` rows valid from each adoption date forever. `rate_to_eur` is
//! units of currency per 1 EUR (the ECB quoting convention), so
//! `eur = amount / rate`. This unit ships the table, the seed, and the lookup —
//! all DORMANT: nothing in the serving path calls [`Db::rate_to_eur`] until the
//! `eur_cents` derivation lands (ADR-0014 build order).

use crate::{Db, t, text};
use turso::Value;

/// How far back a DAILY rate may satisfy a lookup (weekends, holidays, and the
/// occasional source gap). Beyond it the answer is honest absence (ADR-0014 D4:
/// unconvertible → NULL), never a stale rate. `'irrevocable'` rows are exempt —
/// a frozen conversion rate has no staleness.
const DAILY_WINDOW_DAYS: i64 = 7;

/// The irrevocable euro conversion rates, per EU Council regulation — exact by
/// definition. `(currency, adoption_date, units_per_eur)`. Each seeds one
/// `'irrevocable'` row at its adoption date; the lookup treats such a row as
/// valid for every later date (the national currency is frozen from then on —
/// legacy notices kept quoting DEM/FRF/… for a while after 1999). Dates BEFORE
/// adoption are served by the daily ECU/ECB series, not by these.
pub const IRREVOCABLE_EURO_RATES: &[(&str, &str, f64)] = &[
    ("ATS", "1999-01-01", 13.7603),
    ("BEF", "1999-01-01", 40.3399),
    ("DEM", "1999-01-01", 1.95583),
    ("ESP", "1999-01-01", 166.386),
    ("FIM", "1999-01-01", 5.94573),
    ("FRF", "1999-01-01", 6.55957),
    ("IEP", "1999-01-01", 0.787564),
    ("ITL", "1999-01-01", 1936.27),
    ("LUF", "1999-01-01", 40.3399),
    ("NLG", "1999-01-01", 2.20371),
    ("PTE", "1999-01-01", 200.482),
    ("GRD", "2001-01-01", 340.750),
    ("SIT", "2007-01-01", 239.640),
    ("CYP", "2008-01-01", 0.585274),
    ("MTL", "2008-01-01", 0.429300),
    ("SKK", "2009-01-01", 30.1260),
    ("EEK", "2011-01-01", 15.6466),
    ("LVL", "2014-01-01", 0.702804),
    ("LTL", "2015-01-01", 3.45280),
    ("HRK", "2023-01-01", 7.53450),
    ("BGN", "2026-01-01", 1.95583),
];

/// A resolved rate: units of the requested currency per 1 EUR, plus the row's
/// date and source for provenance (`eur_rate_date` in the derivation).
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedRate {
    pub rate_to_eur: f64,
    pub rate_date: String,
    pub source: String,
}

impl Db {
    /// Upsert a batch of daily rate rows — the rates fetch job's write path.
    /// REPLACE, not IGNORE: a source correcting a published day must win.
    pub async fn upsert_currency_rates(
        &self,
        rows: &[(String, String, f64, String)],
    ) -> turso::Result<u64> {
        let conn = self.conn().await;
        Self::immediate(&conn, async {
            for (currency, date, rate, source) in rows {
                conn.execute(
                    "INSERT OR REPLACE INTO currency_rates(currency, rate_date, rate_to_eur, source)
                 VALUES (?, ?, ?, ?)",
                    (t(currency), t(date), Value::Real(*rate), t(source)),
                )
                .await?;
            }
            Ok(())
        })
        .await?;
        Ok(rows.len() as u64)
    }

    /// Seed the irrevocable conversion rates. Idempotent (REPLACE of constants).
    pub async fn seed_irrevocable_euro_rates(&self) -> turso::Result<u64> {
        let rows: Vec<(String, String, f64, String)> = IRREVOCABLE_EURO_RATES
            .iter()
            .map(|(c, d, r)| ((*c).to_owned(), (*d).to_owned(), *r, "irrevocable".to_owned()))
            .collect();
        self.upsert_currency_rates(&rows).await
    }

    /// The EUR-pivot rate for `currency` on `date` (`'YYYY-MM-DD'`): the nearest
    /// row at-or-before the date — within [`DAILY_WINDOW_DAYS`] for a daily
    /// source, unbounded for an `'irrevocable'` one. `EUR` is identity. `None`
    /// is the honest unconvertible answer (ADR-0014 D4).
    pub async fn rate_to_eur(
        &self,
        currency: &str,
        date: &str,
    ) -> turso::Result<Option<ResolvedRate>> {
        let currency = canonical_currency(currency);
        if currency == "EUR" {
            return Ok(Some(ResolvedRate {
                rate_to_eur: 1.0,
                rate_date: date.to_owned(),
                source: "identity".to_owned(),
            }));
        }
        let conn = self.reader().await?;
        let mut rows = conn
            .query(
                "SELECT rate_to_eur, rate_date, source FROM currency_rates
                  WHERE currency = ? AND rate_date <= ?
                  ORDER BY rate_date DESC LIMIT 1",
                (t(currency), t(date)),
            )
            .await?;
        let Some(row) = rows.next().await? else { return Ok(None) };
        let rate = match row.get_value(0)? {
            Value::Real(r) => r,
            Value::Integer(i) => i as f64,
            _ => return Ok(None),
        };
        let found = ResolvedRate { rate_to_eur: rate, rate_date: text(&row, 1), source: text(&row, 2) };
        if found.source == "irrevocable" || day_gap(&found.rate_date, date) <= DAILY_WINDOW_DAYS {
            Ok(Some(found))
        } else {
            Ok(None)
        }
    }
}

/// The ISO-4217 series a published currency code resolves against. The rate
/// SERIES are keyed by ISO codes, but pre-1997 TED notices publish the OJ's own
/// legacy codes — measured on the 2026-08-24 snapshot (issue 172 validation
/// pass, 2026-08-27): 1993–1996 amounts are almost entirely LIT/UKL/DKR/HFL/
/// SKR/NKR/BFR/PTA/LFR/FMK/ESC/FFR…, flipping to ISO mid-1997 with stragglers
/// after. Without this map the whole early corpus would derive NULL against a
/// series that covers every one of these currencies. The PUBLISHED code stays
/// verbatim in the row (ADR-0014: served as published) — only the lookup
/// translates. ECU/XEU resolve as EUR identity: the ECU converted 1:1 by law
/// (Council Regulation 1103/97), and the pre-1999 daily series is ECU-based
/// anyway. An unknown code (e.g. the one observed `GPB` typo) passes through
/// and honestly resolves to nothing.
pub fn canonical_currency(code: &str) -> &str {
    match code {
        "ECU" | "XEU" => "EUR",
        "LIT" => "ITL",
        "UKL" => "GBP",
        "DKR" => "DKK",
        "HFL" | "NFL" => "NLG",
        "SKR" => "SEK",
        "NKR" => "NOK",
        "BFR" => "BEF",
        "PTA" => "ESP",
        "LFR" => "LUF",
        "FMK" => "FIM",
        "ESC" => "PTE",
        "FFR" => "FRF",
        "IKR" => "ISK",
        "SFR" => "CHF",
        "YEN" => "JPY",
        "IRL" => "IEP",
        "CND" => "CAD",
        other => other,
    }
}

/// The whole rates table as an in-memory lookup — what the projection derives
/// `eur_cents` from (ADR-0014). Loaded once per projection run (and after a
/// `fetch-rates` load), so fold-time conversion is pure memory: ~85k rows,
/// a few MB. Mirrors [`Db::rate_to_eur`]'s semantics exactly — nearest row
/// at-or-before the date, [`DAILY_WINDOW_DAYS`] for daily sources, unbounded
/// for `'irrevocable'`, EUR identity, `None` for the unconvertible (D4) — and a
/// test pins the two against each other.
#[derive(Debug, Default)]
pub struct RatesLookup {
    /// currency → (rate_date, rate_to_eur, is_irrevocable), sorted by date.
    by_currency: std::collections::HashMap<String, Vec<(String, f64, bool)>>,
}

impl RatesLookup {
    /// Units of `currency` per 1 EUR on `date`, or `None` (unconvertible).
    pub fn rate(&self, currency: &str, date: &str) -> Option<f64> {
        let currency = canonical_currency(currency);
        if currency == "EUR" {
            return Some(1.0);
        }
        let series = self.by_currency.get(currency)?;
        let at = series.partition_point(|(d, _, _)| d.as_str() <= date);
        let (found_date, rate, irrevocable) = series.get(at.checked_sub(1)?)?;
        if *irrevocable || day_gap(found_date, date) <= DAILY_WINDOW_DAYS {
            Some(*rate)
        } else {
            None
        }
    }

    /// `cents` of `currency` on `date` as EUR cents, half-up-rounded, or `None`.
    pub fn eur_cents(&self, cents: i64, currency: &str, date: &str) -> Option<i64> {
        let rate = self.rate(currency, date)?;
        Some(((cents as f64) / rate).round() as i64)
    }

    /// Seed one rate, for tests that need a conversion without a database.
    ///
    /// The real lookup is built by [`Db::reload_rates_lookup`] from the rates
    /// table, which a unit test of the ELECTION has no business standing up —
    /// issue 378's rounding rule is about what the election does with a
    /// converted zero, and it needs exactly one non-EUR rate to exercise.
    #[cfg(test)]
    pub fn insert_for_test(&mut self, currency: &str, date: &str, per_eur: f64) {
        self.by_currency
            .entry(currency.to_owned())
            .or_default()
            .push((date.to_owned(), per_eur, false));
    }
}

impl Db {
    /// Reload the in-memory rates lookup from the table. Called at the start of
    /// every projection run and after a `fetch-rates` load; until first called,
    /// the lookup is empty and every derivation is honestly `None`.
    pub async fn reload_rates_lookup(&self) -> turso::Result<usize> {
        let conn = self.reader().await?;
        let mut by_currency: std::collections::HashMap<String, Vec<(String, f64, bool)>> =
            Default::default();
        let mut rows = conn
            .query(
                "SELECT currency, rate_date, rate_to_eur, source FROM currency_rates
                  ORDER BY currency, rate_date",
                (),
            )
            .await?;
        let mut n = 0usize;
        while let Some(row) = rows.next().await? {
            let rate = match row.get_value(2)? {
                Value::Real(r) => r,
                Value::Integer(i) => i as f64,
                _ => continue,
            };
            by_currency.entry(text(&row, 0)).or_default().push((
                text(&row, 1),
                rate,
                text(&row, 3) == "irrevocable",
            ));
            n += 1;
        }
        self.set_rates_lookup(RatesLookup { by_currency });
        Ok(n)
    }
}

/// Parse the ECB `eurofxref-hist.csv` (header `Date,USD,JPY,…`; one row per
/// business day, values = units per EUR, missing cells `N/A`; rows carry a
/// trailing comma) into upsert rows tagged `'ecb'`. Unparseable or non-positive
/// cells are skipped — absence over a guess, per ADR-0014 D4.
pub fn parse_ecb_history_csv(csv: &str) -> Vec<(String, String, f64, String)> {
    let mut lines = csv.lines();
    let Some(header) = lines.next() else { return Vec::new() };
    let currencies: Vec<&str> = header.split(',').map(str::trim).collect();
    let mut out = Vec::new();
    for line in lines {
        let mut cells = line.split(',');
        let Some(date) = cells.next().map(str::trim) else { continue };
        if day_number(date).is_none() {
            continue; // not a data row
        }
        for (i, cell) in cells.enumerate() {
            let Some(currency) = currencies.get(i + 1).filter(|c| !c.is_empty()) else { continue };
            if let Ok(rate) = cell.trim().parse::<f64>()
                && rate > 0.0
                && rate.is_finite()
            {
                out.push(((*currency).to_owned(), date.to_owned(), rate, "ecb".to_owned()));
            }
        }
    }
    out
}

/// Parse a Eurostat SDMX-CSV rates file (`ert_h_eur_d` / `ert_bil_eur_d` — the
/// Commission's official daily ECU series, ADR-0014 D2a) into upsert rows
/// tagged `'eurostat-ecu'`. Header-driven: the `currency`, `TIME_PERIOD` and
/// `OBS_VALUE` columns are located by name, so a re-ordered export keeps
/// parsing. Lines are CRLF-terminated in the wild; `\r` is trimmed before
/// splitting. A row with an empty `OBS_VALUE` (confidential cells carry
/// `CONF_STATUS=C` and no value) or a non-positive/non-finite rate is skipped —
/// absence over a guess, per ADR-0014 D4. `OBS_VALUE` is national units per
/// 1 ECU, the same direction as the ECB series' units-per-EUR (ECU→EUR was 1:1
/// by Council Regulation 1103/97), so it IS `rate_to_eur` verbatim.
pub fn parse_eurostat_sdmx_csv(csv: &str) -> Vec<(String, String, f64, String)> {
    let mut lines = csv.lines().map(|l| l.trim_end_matches('\r'));
    let Some(header) = lines.next() else { return Vec::new() };
    let columns: Vec<&str> = header.split(',').map(str::trim).collect();
    let col = |name: &str| columns.iter().position(|c| c.eq_ignore_ascii_case(name));
    let (Some(currency), Some(date), Some(value)) =
        (col("currency"), col("TIME_PERIOD"), col("OBS_VALUE"))
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for line in lines {
        let cells: Vec<&str> = line.split(',').map(str::trim).collect();
        let (Some(cur), Some(day)) = (cells.get(currency), cells.get(date)) else { continue };
        if cur.is_empty() || day_number(day).is_none() {
            continue;
        }
        if let Some(cell) = cells.get(value)
            && let Ok(rate) = cell.parse::<f64>()
            && rate > 0.0
            && rate.is_finite()
        {
            out.push(((*cur).to_owned(), (*day).to_owned(), rate, "eurostat-ecu".to_owned()));
        }
    }
    out
}

/// The newest `rate_date` among parsed upsert rows — the input to the
/// freshness assertion below.
pub fn newest_date(rows: &[(String, String, f64, String)]) -> Option<&str> {
    rows.iter().map(|(_, d, _, _)| d.as_str()).max()
}

/// The incident-306 tripwire: a rates file that parses cleanly can still be
/// WRONG — the ECB's bare hist.csv served a 2010-frozen artifact with one
/// garbage row for months of silence-shaped failure. A live daily series must
/// end near today; a file whose newest row is older than `max_age_days` is
/// refused as defective/stale BEFORE anything is written.
pub fn assert_fresh(
    rows: &[(String, String, f64, String)],
    today: &str,
    max_age_days: i64,
) -> Result<(), String> {
    let newest = newest_date(rows).ok_or("no rows parsed")?;
    let age = day_gap(newest, today);
    if age > max_age_days {
        return Err(format!(
            "rates file is STALE or defective: newest row {newest} is {age} day(s) old \
             (limit {max_age_days}) — refusing to load (issue 306)"
        ));
    }
    Ok(())
}

impl Db {
    /// Reconcile one source's daily rows against the freshly parsed file: any
    /// stored row of `source` whose `rate_date` does not appear in the file at
    /// all is deleted. REPLACE-only loading can never REMOVE a poisoned row
    /// (the issue-306 garbage 2010-02-14 Sunday row survives every re-fetch
    /// otherwise); the fetched file is the authority for its own source, so
    /// reconciliation deletes by date, chunked per year to keep statements
    /// bounded. Same-date same-currency corrections are REPLACE's job.
    pub async fn reconcile_currency_dates(
        &self,
        source: &str,
        rows: &[(String, String, f64, String)],
    ) -> turso::Result<u64> {
        let dates: std::collections::BTreeSet<&str> =
            rows.iter().map(|(_, d, _, _)| d.as_str()).collect();
        let conn = self.conn().await;
        // Walk the STORED years, not the file's: a poisoned row in a year the
        // file doesn't cover at all must still be swept (its year then has an
        // empty keep-list and the whole year goes).
        let mut years = Vec::new();
        let mut year_rows = conn
            .query(
                "SELECT DISTINCT substr(rate_date, 1, 4) FROM currency_rates \
                  WHERE source = ?",
                [t(source)],
            )
            .await?;
        while let Some(row) = year_rows.next().await? {
            years.push(crate::text(&row, 0));
        }
        drop(year_rows);
        let mut removed = 0u64;
        for year in years {
            let in_year: Vec<&str> =
                dates.iter().copied().filter(|d| d.starts_with(&year)).collect();
            let sql = if in_year.is_empty() {
                "DELETE FROM currency_rates WHERE source = ? AND rate_date LIKE ?".to_owned()
            } else {
                let placeholders = vec!["?"; in_year.len()].join(",");
                format!(
                    "DELETE FROM currency_rates WHERE source = ? AND rate_date LIKE ? \
                     AND rate_date NOT IN ({placeholders})"
                )
            };
            let mut params: Vec<Value> = vec![t(source), t(format!("{year}-%"))];
            params.extend(in_year.iter().map(|d| t(*d)));
            removed += conn.execute(&sql, params).await?;
        }
        Ok(removed)
    }

    /// The rederive walk's persisted resume point (issue 306): the last
    /// COMPLETED window's tender-id watermark, 0 when no walk is in flight.
    pub async fn rederive_watermark(&self) -> turso::Result<i64> {
        let conn = self.reader().await?;
        let mut rows = conn
            .query("SELECT rederive_eur_watermark FROM projection_state WHERE id = 0", ())
            .await?;
        Ok(rows.next().await?.map_or(0, |row| crate::int(&row, 0)))
    }

    /// Persist the walk's resume point; write 0 on completion. Each window
    /// writes its own watermark inside its transaction (issue 495 unit 5), so
    /// this is for the completion and for tests.
    pub async fn set_rederive_watermark(&self, watermark: i64) -> turso::Result<()> {
        let conn = self.conn().await;
        conn.execute(
            "UPDATE projection_state SET rederive_eur_watermark = ? WHERE id = 0",
            (Value::Integer(watermark),),
        )
        .await?;
        Ok(())
    }

    /// One window of the issue-306 repair walk, the worked example of ADR-0017's
    /// R2 template (`crate::inplace`). It re-derives ALL FOUR money loci's EUR
    /// siblings for the next `batch` tenders past the id watermark, from
    /// (cents, currency, version publication date) via the in-memory lookup.
    /// That is the exact `EurContext` derivation the fold itself uses, so a
    /// re-run after repair writes nothing. Only rows whose derived value
    /// CHANGES are updated: garbage-rate values corrected, newly convertible
    /// NULLs filled, no-longer-derivable values honestly NULLed.
    ///
    /// The window drives on the `tenders` PK and each locus filters
    /// `a.tender_id > ? AND a.tender_id <= ?` — a PK/`*_version`-index prefix
    /// range, the walk shape every backfill already proved on turso. The first
    /// cut windowed on `a.rowid > ? ORDER BY a.rowid LIMIT ?` instead, and on
    /// prod that shape COLLAPSED ~55M rows in: from ~30k rows/s to a 100%-CPU
    /// zero-IO spin (the issue-274 seek lesson resurfacing on a bare rowid
    /// range). Updates stay rowid-addressed; only the WINDOWING changed.
    ///
    /// **The window is one `BEGIN IMMEDIATE`** (issue 495 unit 5). Inside it, after the
    /// updates:
    /// - **ADR-0017 D5's correction rows.** A moved `eur_cents` is served, so the walk
    ///   is no longer quiet: each moved Tender gets a seq-less `tender changed`, and
    ///   rule L's lots a seq-less `lot changed` ([`crate::inplace::Moved::announce`]).
    /// - **The stamp and the re-queue** of the moved Tenders (E4). Moving `eur_cents`
    ///   invalidates the head value column (issue 375): `current_value_eur_cents` is
    ///   elected FROM these values, and since issue 490 so are the stored lot values.
    ///   The FOLD is the one implementation allowed to elect them, so the moved Tenders
    ///   are stamped epoch-stale and their causing notices re-queued (a stamp alone
    ///   re-folds nothing: the fold's rewrite set is `projected = 0`). The fold that
    ///   follows compares (ADR-0017 D1) and announces what its election moved.
    /// - **The watermark.**
    ///
    /// Before, these ran as separate statements after the updates had autocommitted
    /// one by one. A crash in between kept the moved values and lost the re-queue,
    /// and the re-run could not find it again: the values it would have to compare
    /// against were already right.
    ///
    /// Only Tenders whose stored value actually moved are announced, stamped and
    /// re-queued. A rate correction usually touches one currency-day, so this is a
    /// handful of rows rather than the corpus, which is the whole reason it beats
    /// an epoch bump. A window that moved nothing writes nothing but its watermark.
    pub async fn rederive_eur_window(
        &self,
        rates: &RatesLookup,
        batch: i64,
        after: i64,
    ) -> turso::Result<RederiveWindow> {
        let conn = self.conn().await;
        let now = crate::now_unix();
        let window = Db::immediate(&conn, async {
            let (tenders, watermark) = crate::inplace::tender_window(&conn, after, batch).await?;
            if tenders == 0 {
                return Ok(RederiveWindow { watermark: after, ..RederiveWindow::default() });
            }
            let mut window = rederive_scope(&conn, rates, &Scope::Range { after, watermark }, now).await?;
            conn.execute(
                "UPDATE projection_state SET rederive_eur_watermark = ? WHERE id = 0",
                (Value::Integer(watermark),),
            )
            .await?;
            window.tenders = tenders;
            window.watermark = watermark;
            Ok(window)
        })
        .await?;
        self.ring_for_rederive(&conn, &window).await;
        Ok(window)
    }

    /// Issue 504: one window of the same walk over an explicit set of Tenders, the
    /// daily `rederive-eur-recent`'s unit, reading each Tender's versions from the first
    /// one published at or after `since`. The same transaction holds the moved
    /// `eur_cents`, D5's correction rows, the stamp and the re-queue. There is no
    /// watermark: the set is recomputed from scratch on every run. The caller keeps
    /// `ids` to a few hundred, because the version-date read carries them as an `IN`
    /// list.
    pub async fn rederive_eur_tenders(
        &self,
        rates: &RatesLookup,
        ids: &[i64],
        since: i64,
    ) -> turso::Result<RederiveWindow> {
        if ids.is_empty() {
            return Ok(RederiveWindow::default());
        }
        let conn = self.conn().await;
        let now = crate::now_unix();
        let window = Db::immediate(&conn, async {
            let mut window = rederive_scope(&conn, rates, &Scope::Recent { ids, since }, now).await?;
            window.tenders = ids.len() as i64;
            window.watermark = ids.iter().copied().max().unwrap_or(0);
            Ok(window)
        })
        .await?;
        self.ring_for_rederive(&conn, &window).await;
        Ok(window)
    }

    /// Issue 504: the Tenders whose head version was published at or after `since`
    /// (epoch seconds), ascending, off `tenders_current_published`. These are the ones
    /// a same-day rate can have missed: the daily fold converts a version published
    /// today before the ECB publishes today's rate.
    pub async fn recent_head_tenders(&self, since: i64) -> turso::Result<Vec<i64>> {
        let conn = self.reader().await?;
        let mut rows =
            conn.query("SELECT id FROM tenders WHERE current_published_at >= ?", (Value::Integer(since),)).await?;
        let mut ids = Vec::new();
        while let Some(row) = rows.next().await? {
            ids.push(crate::int(&row, 0));
        }
        ids.sort_unstable();
        Ok(ids)
    }

    /// Ring the doorbell for a committed rederive window's correction rows. The
    /// window is durable by now, so a doorbell that fails to ring is logged, not
    /// returned: failing the job would report applied work as not applied. The next
    /// change anyone appends rings for these rows too (it publishes the newest cursor).
    async fn ring_for_rederive(&self, conn: &turso::Connection, window: &RederiveWindow) {
        if window.corrections > 0
            && let Err(e) = self.publish_cursor(conn).await
        {
            eprintln!("[rederive-eur] window up to tender {} committed; doorbell: {e}", window.watermark);
        }
    }
}

/// Which Tenders a rederive window covers: an id range of the full walk, or an explicit
/// set (issue 504's recent walk).
enum Scope<'a> {
    Range { after: i64, watermark: i64 },
    /// Issue 504's recent walk: these Tenders' versions from the first one published
    /// at or after `since`. Only a version published within days of its fold can have
    /// been converted at a rate that was not fixed yet; reading a long chain's whole
    /// history every day (a framework of thousands of versions) cost the first prod
    /// run 250M rows for nothing.
    Recent { ids: &'a [i64], since: i64 },
}

/// The body of a rederive window, inside the caller's transaction: re-derive the four
/// loci's EUR siblings for `scope`, rewrite the rows that moved, and hand the moved
/// Tenders to D5's correction rows, the stale stamp and the re-queue. Returns the
/// counts; the caller fills in `tenders` and `watermark`.
async fn rederive_scope(
    conn: &turso::Connection,
    rates: &RatesLookup,
    scope: &Scope<'_>,
    now: i64,
) -> turso::Result<RederiveWindow> {
    // The versions' publication dates, joined IN RUST: the SQL join
    // (`JOIN tender_versions ON (tender_id, seq)`) is what wedged BOTH
    // prod repair runs at the same window — a legacy mega-chain
    // (2,983 versions) whose additive result rounds put 8.9M
    // lot_result rows in one 10k-tender window, and turso's evaluation
    // of the join at that volume spun at 100% CPU indefinitely, while
    // the bare PK-range scan of the same rows returns in seconds. Two
    // indexed range scans + an O(1) map lookup replace it.
    let (pred, params) = match scope {
        Scope::Range { after, watermark } => {
            ("tender_id > ? AND tender_id <= ?".to_owned(), vec![Value::Integer(*after), Value::Integer(*watermark)])
        }
        Scope::Recent { ids, .. } => (
            format!("tender_id IN ({})", vec!["?"; ids.len()].join(", ")),
            ids.iter().map(|&id| Value::Integer(id)).collect(),
        ),
    };
    let mut date_of: std::collections::HashMap<(i64, i64), String> = std::collections::HashMap::new();
    // Recent only: each Tender's first version published at or after `since`.
    let mut first_recent: std::collections::BTreeMap<i64, i64> = std::collections::BTreeMap::new();
    {
        let mut rows =
            conn.query(&format!("SELECT tender_id, seq, published_at FROM tender_versions WHERE {pred}"), params).await?;
        while let Some(row) = rows.next().await? {
            let (tender_id, seq, published_at) = (crate::int(&row, 0), crate::int(&row, 1), crate::int(&row, 2));
            date_of.insert((tender_id, seq), civil_date(published_at));
            if let Scope::Recent { since, .. } = scope
                && published_at >= *since
            {
                first_recent.entry(tender_id).and_modify(|s| *s = (*s).min(seq)).or_insert(seq);
            }
        }
    }
    let mut scanned = 0i64;
    let mut moved = crate::inplace::Moved::default();
    let mut pending_by_locus: Vec<Vec<(i64, Option<i64>)>> = Vec::new();
    for (table, cents_col, currency_col, eur_col, lot_col) in EUR_LOCI {
        let lot = lot_col.map_or_else(|| "NULL".to_owned(), |c| format!("a.{c}"));
        let select = format!(
            "SELECT a.rowid, a.{cents_col}, a.{currency_col}, a.{eur_col}, a.tender_id, a.seq, {lot} FROM {table} a"
        );
        let mut pending: Vec<(i64, Option<i64>)> = Vec::new();
        match scope {
            Scope::Range { after, watermark } => {
                let mut rows = conn
                    .query(
                        &format!("{select} WHERE a.tender_id > ? AND a.tender_id <= ?"),
                        (Value::Integer(*after), Value::Integer(*watermark)),
                    )
                    .await?;
                while let Some(row) = rows.next().await? {
                    scanned += 1;
                    consider(&row, rates, &date_of, &mut pending, &mut moved);
                }
            }
            Scope::Recent { .. } => {
                // A seek on the `(tender_id, seq)` prefix per Tender: its recent versions only.
                let mut stmt = conn.prepare(&format!("{select} WHERE a.tender_id = ? AND a.seq >= ?")).await?;
                for (&tender_id, &seq) in &first_recent {
                    let mut rows = stmt.query((Value::Integer(tender_id), Value::Integer(seq))).await?;
                    while let Some(row) = rows.next().await? {
                        scanned += 1;
                        consider(&row, rates, &date_of, &mut pending, &mut moved);
                    }
                }
            }
        }
        pending_by_locus.push(pending);
    }
    let mut updated = 0i64;
    for ((table, _, _, eur_col, _), pending) in EUR_LOCI.iter().zip(pending_by_locus) {
        updated += pending.len() as i64;
        if !pending.is_empty() {
            let mut stmt = conn.prepare(&format!("UPDATE {table} SET {eur_col} = ? WHERE rowid = ?")).await?;
            for (rowid, value) in pending {
                stmt.execute((crate::opt_int(value), Value::Integer(rowid))).await?;
            }
        }
    }
    let changed_tenders = moved.tender_ids();
    let (mut corrections, mut restamped, mut requeued) = (0, 0, 0);
    if !changed_tenders.is_empty() {
        corrections = moved.announce(conn, now).await?;
        restamped = Db::stamp_tenders_stale(conn, changed_tenders.clone()).await?;
        let notices = Db::notice_ids_of_tenders(conn, &changed_tenders).await?;
        requeued = Db::requeue_notice_ids(conn, &notices, false).await?;
    }
    Ok(RederiveWindow { scanned, updated, changed_tenders, corrections, restamped, requeued, ..RederiveWindow::default() })
}

/// One money row of a rederive window: derive its EUR sibling from (cents, currency, the
/// version's publication date) and, if that differs from what is stored, queue the
/// rowid's rewrite and record the move for D5's correction rows.
fn consider(
    row: &turso::Row,
    rates: &RatesLookup,
    date_of: &std::collections::HashMap<(i64, i64), String>,
    pending: &mut Vec<(i64, Option<i64>)>,
    moved: &mut crate::inplace::Moved,
) {
    let stored = crate::opt_int_of(row, 3);
    let (tender_id, seq) = (crate::int(row, 4), crate::int(row, 5));
    let derived = match (crate::opt_int_of(row, 1), crate::opt_text_of(row, 2), date_of.get(&(tender_id, seq))) {
        (Some(cents), Some(currency), Some(date)) => rates.eur_cents(cents, &currency, date),
        _ => None,
    };
    if derived != stored {
        pending.push((crate::int(row, 0), derived));
        moved.row(tender_id, seq, crate::opt_int_of(row, 6));
    }
}

/// One window of [`Db::rederive_eur_window`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RederiveWindow {
    /// Tenders in the window; 0 ends the walk.
    pub tenders: i64,
    /// Money rows read.
    pub scanned: i64,
    /// Money rows whose derived EUR value moved, and was rewritten.
    pub updated: i64,
    /// The Tenders a moved row belongs to, ascending.
    pub changed_tenders: Vec<i64>,
    /// ADR-0017 D3's seq-less correction rows written for them (D5).
    pub corrections: u64,
    /// How many of them were stamped epoch-stale.
    pub restamped: u64,
    /// How many of their causing notices were re-queued for the fold.
    pub requeued: u64,
    /// The window's last Tender id, the walk's resume point.
    pub watermark: i64,
}

/// The four ADR-0014 money loci: (table, cents column, currency column,
/// derived-EUR column, the lot column rule L reads, if the table has one). The order is
/// the rederive walk's locus index.
pub const EUR_LOCI: [(&str, &str, &str, &str, Option<&str>); 4] = [
    ("tender_version_amounts", "cents", "currency", "eur_cents", Some("lot_id")),
    ("tender_version_lot_results", "awarded_cents", "awarded_currency", "awarded_eur_cents", Some("lot_id")),
    ("tender_version_bids", "cents", "currency", "eur_cents", Some("lot_id")),
    ("tender_version_contracts", "cents", "currency", "eur_cents", None),
];

/// `'YYYY-MM-DD'` (UTC) for an epoch-seconds instant — the fetch job's period
/// key. Hinnant's civil-from-days, the inverse of [`day_number`].
pub fn civil_date(epoch_seconds: i64) -> String {
    let z = epoch_seconds.div_euclid(86_400) + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

/// Whole days between two `'YYYY-MM-DD'` strings (`later - earlier`), computed
/// with a civil-date to day-number conversion — no clock, no timezone (the
/// table's dates are calendar days by construction). A malformed date yields
/// `i64::MAX`, which fails the window check — honest absence over a guess.
fn day_gap(earlier: &str, later: &str) -> i64 {
    match (day_number(earlier), day_number(later)) {
        (Some(a), Some(b)) => b - a,
        _ => i64::MAX,
    }
}

/// Days since the civil epoch for `'YYYY-MM-DD'` (Howard Hinnant's
/// days-from-civil algorithm — exact over the table's whole range).
fn day_number(date: &str) -> Option<i64> {
    let mut parts = date.split('-');
    let y: i64 = parts.next()?.parse().ok()?;
    let m: i64 = parts.next()?.parse().ok()?;
    let d: i64 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146097 + doe - 719468)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rule L reads a moved money row's lot through `EUR_LOCI`'s lot column (issue 495
    /// unit 5). It must say what the fold's compare says about the same table, or a
    /// walk would announce a different lot than a refold for the same move.
    #[test]
    fn eur_loci_name_the_lot_column_the_fold_attributes_rows_by() {
        use crate::canonical::{LEAF_TABLES, LotScope};
        for (table, cents, currency, eur, lot) in EUR_LOCI {
            let leaf = LEAF_TABLES.iter().find(|t| t.name == table).unwrap_or_else(|| panic!("{table} is a leaf"));
            for col in [cents, currency, eur] {
                assert!(leaf.cols.contains(&col), "{table}.{col}");
            }
            let fold = match leaf.lot {
                LotScope::Column(c) => Some(c),
                LotScope::Tender => None,
                other => panic!("{table}: the walk reads no {other:?}"),
            };
            assert_eq!(lot, fold, "{table}'s lot column");
        }
    }

    #[test]
    fn eurostat_sdmx_csv_parses_both_dataset_flavours_and_skips_confidential_cells() {
        // Verbatim shapes from the verified 2026-08-27 fetches: CRLF line ends,
        // `NAT` (ert_h_eur_d) and `NAC` (ert_bil_eur_d) unit codes, an
        // empty-OBS_VALUE confidential row (RSD 1995-96 carries CONF_STATUS=C
        // and no value), and DEM closing 1998 on the irrevocable 1.95583.
        let csv = "DATAFLOW,LAST UPDATE,freq,statinfo,unit,currency,TIME_PERIOD,OBS_VALUE,OBS_FLAG,CONF_STATUS\r\n\
                   ESTAT:ERT_H_EUR_D(1.0),08/01/26 11:00:00,D,AVG,NAT,DEM,1993-01-04,1.95268,,\r\n\
                   ESTAT:ERT_H_EUR_D(1.0),08/01/26 11:00:00,D,AVG,NAT,DEM,1998-12-31,1.95583,,\r\n\
                   ESTAT:ERT_BIL_EUR_D(1.0),26/08/26 11:00:00,D,AVG,NAC,USD,1993-01-04,1.1932,,\r\n\
                   ESTAT:ERT_BIL_EUR_D(1.0),26/08/26 11:00:00,D,AVG,NAC,RSD,1995-01-31,,,C\r\n\
                   not,a,data,row,at,all,nope,,,\r\n";
        let rows = parse_eurostat_sdmx_csv(csv);
        assert_eq!(
            rows,
            vec![
                ("DEM".to_owned(), "1993-01-04".to_owned(), 1.95268, "eurostat-ecu".to_owned()),
                ("DEM".to_owned(), "1998-12-31".to_owned(), 1.95583, "eurostat-ecu".to_owned()),
                ("USD".to_owned(), "1993-01-04".to_owned(), 1.1932, "eurostat-ecu".to_owned()),
            ],
            "confidential empty-value rows and non-date rows are skipped; \
             both unit codes parse; the CR never leaks into a cell"
        );
        // A reordered export still parses — the columns are found by name.
        let reordered = "TIME_PERIOD,OBS_VALUE,currency\r\n1997-06-02,6.57,FRF\r\n";
        assert_eq!(
            parse_eurostat_sdmx_csv(reordered),
            vec![("FRF".to_owned(), "1997-06-02".to_owned(), 6.57, "eurostat-ecu".to_owned())]
        );
        // A header without the needed columns parses to nothing, loudly zero —
        // the job turns that into a hard error rather than an empty success.
        assert!(parse_eurostat_sdmx_csv("a,b,c\r\n1,2,3\r\n").is_empty());
    }

    #[test]
    fn ecb_history_csv_parses_with_gaps_and_trailing_commas() {
        let csv = "Date, USD, JPY, HRK,\n\
                   2026-08-26, 1.1623, 171.83, N/A,\n\
                   2026-08-25, 1.1608, , 7.5345,\n\
                   not-a-date, 9.9, 9.9, 9.9,\n";
        let rows = parse_ecb_history_csv(csv);
        assert_eq!(
            rows,
            vec![
                ("USD".to_owned(), "2026-08-26".to_owned(), 1.1623, "ecb".to_owned()),
                ("JPY".to_owned(), "2026-08-26".to_owned(), 171.83, "ecb".to_owned()),
                ("USD".to_owned(), "2026-08-25".to_owned(), 1.1608, "ecb".to_owned()),
                ("HRK".to_owned(), "2026-08-25".to_owned(), 7.5345, "ecb".to_owned()),
            ],
            "N/A and empty cells skipped, the trailing comma's phantom column ignored, \
             a non-date row dropped"
        );
    }

    #[test]
    fn civil_date_inverts_day_number() {
        for d in ["1993-01-04", "1998-12-31", "1999-01-01", "2024-02-29", "2026-08-27"] {
            let n = day_number(d).expect("valid");
            assert_eq!(civil_date(n * 86_400), d, "round-trip {d}");
            assert_eq!(civil_date(n * 86_400 + 86_399), d, "last second of {d}");
        }
    }

    #[test]
    fn the_stale_rates_tripwire_refuses_a_frozen_file() {
        // The issue-306 shape exactly: a file whose newest row is years old
        // parses cleanly and must still be refused. Ten days tolerates
        // weekends and holiday runs of the real business-day series.
        let frozen = vec![
            ("USD".to_owned(), "2010-02-12".to_owned(), 1.3572, "ecb".to_owned()),
            ("USD".to_owned(), "2010-02-14".to_owned(), 2.0, "ecb".to_owned()),
        ];
        assert_eq!(newest_date(&frozen), Some("2010-02-14"));
        let err = assert_fresh(&frozen, "2026-08-27", 10).unwrap_err();
        assert!(err.contains("STALE or defective"), "says what it refused: {err}");
        assert!(err.contains("2010-02-14"), "names the newest row: {err}");

        let live = vec![("USD".to_owned(), "2026-08-25".to_owned(), 1.1645, "ecb".to_owned())];
        assert!(assert_fresh(&live, "2026-08-27", 10).is_ok(), "a fresh file passes");
        assert!(
            assert_fresh(&[], "2026-08-27", 10).is_err(),
            "an empty parse is refused, not silently fresh"
        );
    }

    #[test]
    fn day_gap_counts_calendar_days() {
        assert_eq!(day_gap("1999-01-01", "1999-01-04"), 3);
        assert_eq!(day_gap("1998-12-31", "1999-01-01"), 1, "across a year boundary");
        assert_eq!(day_gap("2024-02-28", "2024-03-01"), 2, "leap year");
        assert_eq!(day_gap("2023-02-28", "2023-03-01"), 1, "non-leap year");
        assert_eq!(day_gap("bogus", "2023-03-01"), i64::MAX, "malformed fails the window");
    }
}
