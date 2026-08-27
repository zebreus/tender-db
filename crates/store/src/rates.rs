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
        conn.execute("BEGIN IMMEDIATE", ()).await?;
        for (currency, date, rate, source) in rows {
            conn.execute(
                "INSERT OR REPLACE INTO currency_rates(currency, rate_date, rate_to_eur, source)
                 VALUES (?, ?, ?, ?)",
                (t(currency), t(date), Value::Real(*rate), t(source)),
            )
            .await?;
        }
        conn.execute("COMMIT", ()).await?;
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
    fn day_gap_counts_calendar_days() {
        assert_eq!(day_gap("1999-01-01", "1999-01-04"), 3);
        assert_eq!(day_gap("1998-12-31", "1999-01-01"), 1, "across a year boundary");
        assert_eq!(day_gap("2024-02-28", "2024-03-01"), 2, "leap year");
        assert_eq!(day_gap("2023-02-28", "2023-03-01"), 1, "non-leap year");
        assert_eq!(day_gap("bogus", "2023-03-01"), i64::MAX, "malformed fails the window");
    }
}
