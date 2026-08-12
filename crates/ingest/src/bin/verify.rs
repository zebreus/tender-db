//! `verify` — the standing acceptance harness for a deployed tender-db
//! (issue 15, spec §5 "verification"). It is deliberately **black-box**: it
//! talks only to the public API of a running instance (default
//! `https://tenders.zebreus.click`) and checks three things, each against an
//! *external* ground truth, never against the instance's own dashboard:
//!
//! 1. **Coverage** — per-year TED notice counts held by the instance vs the
//!    vendored research ground truth (`crates/app/data/ted-notice-counts.csv`),
//!    within a documented tolerance (upstream counts are approximate for the
//!    older eras — see the CSV header and docs/research/ted-access-channels.md).
//! 2. **Search-API cross-check** — for a few eForms-era sample days, take the
//!    notices the TED Search API v3 lists for that publication date and assert
//!    the instance actually holds them. This is a *set-membership* check, not a
//!    raw day-count: even now that `published_at` is the true OJEU publication
//!    date (issue 18: sourced from the notice's `efac:Publication` block, so a
//!    same-day comparison against the Search API's `publication-date` is in
//!    principle sound), membership stays the stronger assertion — it is immune
//!    to a handful of notices whose OJEU stamp differs from the Search API's
//!    view, and to the dispatch-vs-publication skew that `dispatched_at` now
//!    records separately. "Does our raw notice layer contain these exact
//!    publication ids" is the invariant we actually care about.
//! 3. **Era ladder** — one known real notice per format era must resolve
//!    through the API: the Notice exists, its Tender exists, and the eForms
//!    contract-award notice carries its results and a winner.
//!
//! Counting per (source, year/date) is not expressible through the REST
//! filters, so checks 1 and the instance side of 2/3 use the product's own
//! read-only `/v1/sql` endpoint and need an API token (`--token`, or the
//! `TENDER_API_TOKEN` env var); without one those checks are reported as
//! *skipped*, not silently passed. The Search API itself is anonymous.
//!
//! Human-readable report by default; `--json` for machines. Exit is non-zero
//! when any executed check fails or could not run — so a partially-backfilled
//! instance (most years missing today, pre-backfill) reports failure honestly
//! instead of crashing.

use serde_json::{Value, json};
use std::process::ExitCode;

/// The production instance, and the default target.
const DEFAULT_BASE_URL: &str = "https://tenders.zebreus.click";

/// Anonymous TED Search API v3 (docs/research/ted-access-channels.md §3).
const SEARCH_API: &str = "https://api.ted.europa.eu/v3/notices/search";

/// The vendored per-year ground truth, compiled in so the tool is a single
/// self-contained binary. Same file the dashboard's coverage metric uses.
const GROUND_TRUTH: &str = include_str!("../../../app/data/ted-notice-counts.csv");

/// Relative tolerance for the per-year comparison. The ground-truth counts are
/// approximate by construction (max-publication-number overcounts skipped
/// numbers; an API-vs-filename check for 2026 differed by 0.13 %), so an exact
/// match is neither expected nor required — 2 % absorbs that noise while still
/// catching a genuinely short year.
const DEFAULT_TOLERANCE: f64 = 0.02;

/// eForms-era publication days used for the Search-API cross-check, spread
/// across the era (the Search API reliably indexes 2016+; these are all OJ S
/// weekdays). `2026-07-17` is the fixtures' reference daily (`daily-202600136`).
const SAMPLE_DAYS: [&str; 5] =
    ["2024-01-03", "2024-07-02", "2025-01-02", "2025-07-01", "2026-07-17"];

/// The era ladder: one real TED publication per format era, from the parser
/// fixtures (crates/ingest/tests/fixtures/README.md). Each must resolve through
/// the API once its era is backfilled. `expect_results` is set only where the
/// fixtures document a result/winner (the eForms CAN).
struct EraNotice {
    label: &'static str,
    era: &'static str,
    publication_id: &'static str,
    expect_results: bool,
}

const ERA_LADDER: [EraNotice; 8] = [
    EraNotice { label: "eForms CN (sub 16, DE)", era: "eforms", publication_id: "00494343-2026", expect_results: false },
    EraNotice { label: "eForms CAN (sub 29, NO)", era: "eforms", publication_id: "00495054-2026", expect_results: true },
    EraNotice { label: "R2.0.9 F02 (contract notice)", era: "r209", publication_id: "000245-2019", expect_results: false },
    EraNotice { label: "R2.0.9 F03 (contract award)", era: "r209", publication_id: "000988-2019", expect_results: false },
    EraNotice { label: "R2.0.8 F02 (2014)", era: "r208", publication_id: "000333-2014", expect_results: false },
    EraNotice { label: "R2.0.7 F02 (2011)", era: "r207", publication_id: "001441-2011", expect_results: false },
    EraNotice { label: "text-era CAN (2005)", era: "text", publication_id: "154-2005", expect_results: false },
    EraNotice { label: "text-era CN (2008)", era: "text", publication_id: "723-2008", expect_results: false },
];

// ------------------------------------------------------------------ ground truth

/// One year of the vendored ground truth.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GroundYear {
    year: u16,
    expected: u64,
    partial: bool,
}

/// Parse the vendored CSV: `# …` comment lines and blank lines are skipped, the
/// rest are `year,notices,partial`.
fn parse_ground_truth(csv: &str) -> Vec<GroundYear> {
    csv.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let mut cols = line.split(',');
            let year = cols.next()?.trim().parse().ok()?;
            let expected = cols.next()?.trim().parse().ok()?;
            let partial = cols.next().is_some_and(|c| c.trim() == "1");
            Some(GroundYear { year, expected, partial })
        })
        .collect()
}

// --------------------------------------------------------------------- verdicts

/// Whether an instance's count for a year meets the ground truth.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Coverage {
    /// Within tolerance.
    Pass,
    /// The instance holds nothing for a year the ground truth expects.
    Missing,
    /// Present but below tolerance — an incomplete year.
    Short,
    /// Present but above tolerance — duplicate or spurious ingestion.
    Over,
}

impl Coverage {
    fn ok(self) -> bool {
        self == Coverage::Pass
    }

    fn label(self) -> &'static str {
        match self {
            Coverage::Pass => "PASS",
            Coverage::Missing => "MISSING",
            Coverage::Short => "SHORT",
            Coverage::Over => "OVER",
        }
    }
}

/// Compare one year's instance count against the ground truth.
///
/// A `partial` year (the current, calendar-incomplete year) uses a
/// snapshot count as its ground truth, so being *at or above* it is fine and
/// only a real shortfall below tolerance is flagged; a complete year is held to
/// a two-sided band.
fn classify(expected: u64, actual: u64, partial: bool, tolerance: f64) -> Coverage {
    if actual == 0 {
        return if expected == 0 { Coverage::Pass } else { Coverage::Missing };
    }
    let expected = expected as f64;
    let actual = actual as f64;
    let low = expected * (1.0 - tolerance);
    let high = expected * (1.0 + tolerance);
    if actual < low {
        Coverage::Short
    } else if !partial && actual > high {
        Coverage::Over
    } else {
        Coverage::Pass
    }
}

/// The outcome of one non-coverage check.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Check {
    Pass,
    Fail(String),
    /// Not run (e.g. no token) — reported, never counted as success.
    Skip(String),
    /// Could not be evaluated (network/parse) — counted as a failure.
    Error(String),
}

impl Check {
    fn label(&self) -> &'static str {
        match self {
            Check::Pass => "PASS",
            Check::Fail(_) => "FAIL",
            Check::Skip(_) => "SKIP",
            Check::Error(_) => "ERROR",
        }
    }

    fn detail(&self) -> &str {
        match self {
            Check::Pass => "",
            Check::Fail(m) | Check::Skip(m) | Check::Error(m) => m,
        }
    }

    /// A check counts against the exit code unless it passed or was skipped.
    fn failed(&self) -> bool {
        matches!(self, Check::Fail(_) | Check::Error(_))
    }
}

// --------------------------------------------------------------------- rows

struct CoverageRow {
    year: u16,
    expected: u64,
    actual: u64,
    partial: bool,
    verdict: Coverage,
}

struct SearchRow {
    date: String,
    /// The Search API's full notice count for the day.
    api_total: Option<u64>,
    /// How many of the sampled ids the instance holds, out of how many sampled.
    held: Option<u64>,
    sampled: Option<u64>,
    verdict: Check,
}

struct EraRow {
    label: String,
    era: String,
    publication_id: String,
    verdict: Check,
}

/// A whole run, ready to render as text or JSON.
struct Report {
    base_url: String,
    tolerance: f64,
    coverage: Vec<CoverageRow>,
    coverage_note: Option<String>,
    search: Vec<SearchRow>,
    era: Vec<EraRow>,
}

impl Report {
    /// Overall pass: every executed check passed, and the coverage check — the
    /// core acceptance metric — actually ran.
    fn ok(&self) -> bool {
        let coverage_ran = self.coverage_note.is_none();
        let coverage_ok = self.coverage.iter().all(|r| r.verdict.ok());
        let search_ok = self.search.iter().all(|r| !r.verdict.failed());
        let era_ok = self.era.iter().all(|r| !r.verdict.failed());
        coverage_ran && coverage_ok && search_ok && era_ok
    }
}

// ------------------------------------------------------------------- HTTP client

/// A thin client bound to one instance; carries the token if we have one.
struct Instance {
    http: reqwest::Client,
    base_url: String,
    token: Option<String>,
}

impl Instance {
    /// Run a read-only SELECT via `/v1/sql` and return its rows. The endpoint
    /// takes the SQL as the raw request body and authenticates a Bearer token.
    async fn sql(&self, query: &str) -> Result<Vec<Vec<Value>>, String> {
        let token = self.token.as_deref().ok_or("no API token")?;
        let response = self
            .http
            .post(format!("{}/v1/sql", self.base_url))
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "text/plain")
            .body(query.to_owned())
            .send()
            .await
            .map_err(|e| format!("request failed: {e}"))?;
        let status = response.status();
        let text = response.text().await.map_err(|e| format!("read body: {e}"))?;
        if !status.is_success() {
            return Err(format!("HTTP {}: {}", status.as_u16(), api_error(&text)));
        }
        let body: Value = serde_json::from_str(&text).map_err(|e| format!("bad JSON: {e}"))?;
        let rows = body
            .get("rows")
            .and_then(Value::as_array)
            .ok_or("response had no `rows`")?
            .iter()
            .filter_map(|r| r.as_array().cloned())
            .collect();
        Ok(rows)
    }

    /// Fetch a tender's full detail document.
    async fn tender(&self, id: i64) -> Result<Value, String> {
        let response = self
            .http
            .get(format!("{}/v1/tenders/{id}", self.base_url))
            .send()
            .await
            .map_err(|e| format!("request failed: {e}"))?;
        let status = response.status();
        let text = response.text().await.map_err(|e| format!("read body: {e}"))?;
        if !status.is_success() {
            return Err(format!("HTTP {}: {}", status.as_u16(), api_error(&text)));
        }
        serde_json::from_str(&text).map_err(|e| format!("bad JSON: {e}"))
    }
}

/// Pull `error.message` out of an API error body, or fall back to the raw text.
fn api_error(text: &str) -> String {
    serde_json::from_str::<Value>(text)
        .ok()
        .and_then(|v| v.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str()).map(str::to_owned))
        .unwrap_or_else(|| text.chars().take(200).collect())
}

/// Read a JSON cell as a count, tolerating integer or float encodings.
fn as_u64(cell: &Value) -> Option<u64> {
    cell.as_u64()
        .or_else(|| cell.as_i64().map(|v| v.max(0) as u64))
        .or_else(|| cell.as_f64().map(|v| v.max(0.0) as u64))
}

// ------------------------------------------------------------------- checks

/// Check 1 — per-year TED coverage. Counts distinct publication ids per year
/// from the instance and compares each against the ground truth.
async fn run_coverage(instance: &Instance, ground: &[GroundYear], tolerance: f64) -> (Vec<CoverageRow>, Option<String>) {
    if instance.token.is_none() {
        return (Vec::new(), Some("skipped: per-year counting needs --token (the /v1/sql endpoint)".into()));
    }
    // The year is the four-digit suffix of a TED publication id (`494343-2026`).
    let query = "SELECT CAST(substr(publication_id, instr(publication_id, '-') + 1) AS INTEGER) AS year, \
                 COUNT(DISTINCT publication_id) AS notices \
                 FROM notices WHERE source = 'ted' GROUP BY year";
    let rows = match instance.sql(query).await {
        Ok(rows) => rows,
        Err(e) => return (Vec::new(), Some(format!("skipped: {e}"))),
    };
    let mut counts = std::collections::HashMap::new();
    for row in &rows {
        if let (Some(year), Some(count)) = (row.first().and_then(as_u64), row.get(1).and_then(as_u64)) {
            counts.insert(year as u16, count);
        }
    }
    let coverage = ground
        .iter()
        .map(|g| {
            let actual = counts.get(&g.year).copied().unwrap_or(0);
            CoverageRow {
                year: g.year,
                expected: g.expected,
                actual,
                partial: g.partial,
                verdict: classify(g.expected, actual, g.partial, tolerance),
            }
        })
        .collect();
    (coverage, None)
}

/// How many publication ids to sample per day (the Search API's max page).
const SEARCH_SAMPLE: usize = 250;

/// Check 2 — Search-API cross-check for the sample days. Sequential and small
/// (Search-API politeness). For each day we take up to [`SEARCH_SAMPLE`] of the
/// notices TED lists for that publication date and assert the instance holds
/// every one — a set-membership check that is immune to the dispatch-vs-
/// publication date skew a raw count comparison would trip on.
async fn run_search(instance: &Instance, days: &[&str]) -> Vec<SearchRow> {
    let mut rows = Vec::new();
    for &day in days {
        let page = search_api_page(&instance.http, day).await;
        let (verdict, api_total, held, sampled) = match page {
            Err(e) => (Check::Error(format!("Search API: {e}")), None, None, None),
            Ok((total, ids)) if ids.is_empty() => (
                Check::Skip("Search API lists no notices for this day".into()),
                Some(total),
                None,
                Some(0),
            ),
            Ok((total, ids)) => {
                let sampled = ids.len() as u64;
                match instance_holds(instance, &ids).await {
                    Err(e) if instance.token.is_none() => {
                        (Check::Skip(format!("instance side: {e}")), Some(total), None, Some(sampled))
                    }
                    Err(e) => (Check::Error(format!("instance side: {e}")), Some(total), None, Some(sampled)),
                    Ok(held) if held == sampled => (Check::Pass, Some(total), Some(held), Some(sampled)),
                    Ok(held) => (
                        Check::Fail(format!("instance holds {held}/{sampled} sampled (API total {total})")),
                        Some(total),
                        Some(held),
                        Some(sampled),
                    ),
                }
            }
        };
        rows.push(SearchRow { date: day.to_owned(), api_total, held, sampled, verdict });
    }
    rows
}

/// One page of the TED Search API for a publication date: the total count and
/// the (validated) publication numbers on the first page.
async fn search_api_page(http: &reqwest::Client, day: &str) -> Result<(u64, Vec<String>), String> {
    let compact: String = day.chars().filter(char::is_ascii_digit).collect();
    let body = json!({
        "query": format!("publication-date={compact}"),
        "fields": ["publication-number"],
        "page": 1,
        "limit": SEARCH_SAMPLE,
        "scope": "ALL",
        "paginationMode": "PAGE_NUMBER",
    });
    let response = http
        .post(SEARCH_API)
        .header("content-type", "application/json")
        .body(body.to_string())
        .send()
        .await
        .map_err(|e| format!("request failed: {e}"))?;
    let status = response.status();
    let text = response.text().await.map_err(|e| format!("read body: {e}"))?;
    if !status.is_success() {
        return Err(format!("HTTP {}: {}", status.as_u16(), text.chars().take(200).collect::<String>()));
    }
    let value: Value = serde_json::from_str(&text).map_err(|e| format!("bad JSON: {e}"))?;
    let total = value
        .get("totalNoticeCount")
        .and_then(as_u64)
        .ok_or("response had no totalNoticeCount")?;
    let ids = value
        .get("notices")
        .and_then(Value::as_array)
        .map(|notices| {
            notices
                .iter()
                .filter_map(|n| n.get("publication-number").and_then(Value::as_str))
                .filter(|s| valid_publication_number(s))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    Ok((total, ids))
}

/// A TED publication number is `<up to 8 digits>-<4-digit year>` (the pattern
/// the Search API itself enforces). Validated before it is ever interpolated
/// into a SQL `IN` list.
fn valid_publication_number(s: &str) -> bool {
    let Some((number, year)) = s.split_once('-') else { return false };
    (1..=8).contains(&number.len())
        && number.bytes().all(|b| b.is_ascii_digit())
        && year.len() == 4
        && year.bytes().all(|b| b.is_ascii_digit())
}

/// How many of the given TED publication numbers the instance's raw notice
/// layer holds. Storage zero-pads the number (`00494343-2026`) while the Search
/// API strips it (`494343-2026`), so both sides are normalised to the stripped
/// `<int>-<year>` form for the comparison.
async fn instance_holds(instance: &Instance, numbers: &[String]) -> Result<u64, String> {
    let list = numbers.iter().map(|n| format!("'{n}'")).collect::<Vec<_>>().join(",");
    let query = format!(
        "SELECT COUNT(*) FROM (SELECT DISTINCT publication_id FROM notices WHERE source = 'ted' AND \
         (CAST(substr(publication_id, 1, instr(publication_id, '-') - 1) AS INTEGER) || '-' || \
         substr(publication_id, instr(publication_id, '-') + 1)) IN ({list}))"
    );
    let rows = instance.sql(&query).await?;
    rows.first()
        .and_then(|r| r.first())
        .and_then(as_u64)
        .ok_or_else(|| "empty count".to_owned())
}

/// Check 3 — the era ladder. Each known notice must resolve: the Notice row
/// exists, a Tender version references its publication id, and where a result is
/// expected the tender detail carries at least one winner.
async fn run_era_ladder(instance: &Instance, ladder: &[EraNotice]) -> Vec<EraRow> {
    let mut rows = Vec::new();
    for n in ladder {
        let verdict = if instance.token.is_none() {
            Check::Skip("resolution by publication id needs --token".into())
        } else {
            resolve_era(instance, n).await
        };
        rows.push(EraRow {
            label: n.label.to_owned(),
            era: n.era.to_owned(),
            publication_id: n.publication_id.to_owned(),
            verdict,
        });
    }
    rows
}

async fn resolve_era(instance: &Instance, n: &EraNotice) -> Check {
    // 1. the Notice exists.
    let notice_sql = format!(
        "SELECT COUNT(*) FROM notices WHERE source = 'ted' AND publication_id = '{}'",
        n.publication_id
    );
    match instance.sql(&notice_sql).await {
        Err(e) => return Check::Error(e),
        Ok(rows) => {
            let found = rows.first().and_then(|r| r.first()).and_then(as_u64).unwrap_or(0);
            if found == 0 {
                return Check::Fail("notice not found".into());
            }
        }
    }
    // 2. a Tender version references it.
    let tender_sql = format!(
        "SELECT tender_id FROM tender_versions WHERE publication_id = '{}' LIMIT 1",
        n.publication_id
    );
    let tender_id = match instance.sql(&tender_sql).await {
        Err(e) => return Check::Error(e),
        Ok(rows) => match rows.first().and_then(|r| r.first()).and_then(as_u64) {
            Some(id) => id as i64,
            None => return Check::Fail("no tender projected from this notice".into()),
        },
    };
    // 3. where expected, the tender detail carries a winner.
    if n.expect_results {
        match instance.tender(tender_id).await {
            Err(e) => return Check::Error(format!("tender {tender_id}: {e}")),
            Ok(detail) => {
                if !has_winner(&detail) {
                    return Check::Fail(format!("tender {tender_id} has no lot result with a winner"));
                }
            }
        }
    }
    Check::Pass
}

/// Whether a tender detail document carries at least one lot result naming a
/// winner (a non-empty `winners` array on some `lot_results` entry).
fn has_winner(detail: &Value) -> bool {
    detail
        .get("lot_results")
        .and_then(Value::as_array)
        .is_some_and(|results| {
            results.iter().any(|r| {
                r.get("winners").and_then(Value::as_array).is_some_and(|w| !w.is_empty())
            })
        })
}

// ------------------------------------------------------------------- rendering

fn render_text(report: &Report) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "tender-db verification — {}", report.base_url);
    let _ = writeln!(out, "tolerance: ±{:.1}%\n", report.tolerance * 100.0);

    // Coverage.
    let _ = writeln!(out, "== 1. Coverage vs ground truth (TED, per year) ==");
    if let Some(note) = &report.coverage_note {
        let _ = writeln!(out, "  {note}");
    } else {
        let _ = writeln!(out, "  {:<6} {:>10} {:>10} {:>8}  status", "year", "expected", "instance", "delta");
        for row in &report.coverage {
            let delta = row.actual as i64 - row.expected as i64;
            let mark = if row.partial { " (partial)" } else { "" };
            let _ = writeln!(
                out,
                "  {:<6} {:>10} {:>10} {:>+8} {:>8}{}",
                row.year, row.expected, row.actual, delta, row.verdict.label(), mark
            );
        }
        let passes = report.coverage.iter().filter(|r| r.verdict.ok()).count();
        let _ = writeln!(out, "  {passes}/{} years within tolerance", report.coverage.len());
    }

    // Search API.
    let _ = writeln!(out, "\n== 2. Search-API cross-check (sample days, set membership) ==");
    let _ = writeln!(out, "  {:<12} {:>10} {:>12}  status", "date", "api-total", "held/sampled");
    for row in &report.search {
        let api = row.api_total.map(|v| v.to_string()).unwrap_or_else(|| "-".into());
        let held = match (row.held, row.sampled) {
            (Some(h), Some(s)) => format!("{h}/{s}"),
            (None, Some(s)) => format!("-/{s}"),
            _ => "-".into(),
        };
        let _ = write!(out, "  {:<12} {api:>10} {held:>12}  {}", row.date, row.verdict.label());
        if !row.verdict.detail().is_empty() {
            let _ = write!(out, " — {}", row.verdict.detail());
        }
        let _ = writeln!(out);
    }

    // Era ladder.
    let _ = writeln!(out, "\n== 3. Era-ladder spot checks ==");
    for row in &report.era {
        let _ = write!(out, "  [{:<7}] {:<28} {:<14} {}", row.era, row.label, row.publication_id, row.verdict.label());
        if !row.verdict.detail().is_empty() {
            let _ = write!(out, " — {}", row.verdict.detail());
        }
        let _ = writeln!(out);
    }

    let _ = writeln!(out, "\nRESULT: {}", if report.ok() { "PASS" } else { "FAIL" });
    out
}

fn render_json(report: &Report) -> String {
    let coverage = report
        .coverage
        .iter()
        .map(|r| json!({
            "year": r.year,
            "expected": r.expected,
            "instance": r.actual,
            "partial": r.partial,
            "status": r.verdict.label(),
        }))
        .collect::<Vec<_>>();
    let search = report
        .search
        .iter()
        .map(|r| json!({
            "date": r.date,
            "api_total": r.api_total,
            "held": r.held,
            "sampled": r.sampled,
            "status": r.verdict.label(),
            "detail": r.verdict.detail(),
        }))
        .collect::<Vec<_>>();
    let era = report
        .era
        .iter()
        .map(|r| json!({
            "era": r.era,
            "label": r.label,
            "publication_id": r.publication_id,
            "status": r.verdict.label(),
            "detail": r.verdict.detail(),
        }))
        .collect::<Vec<_>>();
    let value = json!({
        "base_url": report.base_url,
        "tolerance": report.tolerance,
        "ok": report.ok(),
        "coverage": { "note": report.coverage_note, "years": coverage },
        "search": search,
        "era_ladder": era,
    });
    serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".into())
}

// ------------------------------------------------------------------- CLI

struct Args {
    base_url: String,
    token: Option<String>,
    sample_days: usize,
    tolerance: f64,
    json: bool,
}

fn usage() -> ! {
    eprintln!(
        "usage: verify [--base-url URL] [--token TDB…] [--sample-days N] \
         [--tolerance F] [--json]\n\
         \n\
         Checks a deployed tender-db against external ground truth.\n\
         --base-url    instance to verify (default {DEFAULT_BASE_URL})\n\
         --token       API token for /v1/sql (or env TENDER_API_TOKEN); coverage\n\
         \x20             and era-ladder checks are skipped without one\n\
         --sample-days number of Search-API sample days, max {} (default {})\n\
         --tolerance   per-year relative tolerance (default {DEFAULT_TOLERANCE})\n\
         --json        emit the report as JSON",
        SAMPLE_DAYS.len(),
        SAMPLE_DAYS.len(),
    );
    std::process::exit(2);
}

fn parse_args() -> Args {
    let mut args = Args {
        base_url: DEFAULT_BASE_URL.to_owned(),
        token: std::env::var("TENDER_API_TOKEN").ok(),
        sample_days: SAMPLE_DAYS.len(),
        tolerance: DEFAULT_TOLERANCE,
        json: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let mut value = || it.next().unwrap_or_else(|| usage());
        match arg.as_str() {
            "--base-url" | "--base" => args.base_url = value().trim_end_matches('/').to_owned(),
            "--token" => args.token = Some(value()),
            "--sample-days" => args.sample_days = value().parse().unwrap_or_else(|_| usage()),
            "--tolerance" => args.tolerance = value().parse().unwrap_or_else(|_| usage()),
            "--json" => args.json = true,
            "-h" | "--help" => usage(),
            _ => usage(),
        }
    }
    args
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let args = parse_args();
    let instance = Instance {
        http: reqwest::Client::new(),
        base_url: args.base_url.clone(),
        token: args.token.filter(|t| !t.is_empty()),
    };
    let ground = parse_ground_truth(GROUND_TRUTH);
    let days: Vec<&str> = SAMPLE_DAYS.iter().take(args.sample_days.min(SAMPLE_DAYS.len())).copied().collect();

    let (coverage, coverage_note) = run_coverage(&instance, &ground, args.tolerance).await;
    let search = run_search(&instance, &days).await;
    let era = run_era_ladder(&instance, &ERA_LADDER).await;

    let report = Report {
        base_url: args.base_url,
        tolerance: args.tolerance,
        coverage,
        coverage_note,
        search,
        era,
    };

    let output = if args.json { render_json(&report) } else { render_text(&report) };
    println!("{output}");

    if report.ok() { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}

// ------------------------------------------------------------------- tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ground_truth_parses_and_skips_comments() {
        let ground = parse_ground_truth(GROUND_TRUTH);
        // 1993 through 2026 inclusive.
        assert_eq!(ground.len(), 2026 - 1993 + 1);
        // 1993 carries the distributed distinct-ND count (issue 189), not the
        // Office's assigned-number counter the table originally quoted.
        assert_eq!(ground.first().copied(), Some(GroundYear { year: 1993, expected: 66521, partial: false }));
        let last = ground.last().copied().unwrap();
        assert_eq!(last, GroundYear { year: 2026, expected: 497791, partial: true });
        // Exactly one partial year (the current one).
        assert_eq!(ground.iter().filter(|g| g.partial).count(), 1);
    }

    #[test]
    fn parse_ignores_blank_and_malformed_lines() {
        let csv = "# a comment\n\n1999,209009,0\nnot,a,number\n2000,161228,0\n";
        let ground = parse_ground_truth(csv);
        assert_eq!(ground.len(), 2);
        assert_eq!(ground[0].year, 1999);
        assert_eq!(ground[1].expected, 161228);
    }

    #[test]
    fn classify_exact_and_within_tolerance() {
        assert_eq!(classify(1000, 1000, false, 0.02), Coverage::Pass);
        assert_eq!(classify(1000, 990, false, 0.02), Coverage::Pass); // -1%
        assert_eq!(classify(1000, 1010, false, 0.02), Coverage::Pass); // +1%
    }

    #[test]
    fn classify_missing_short_and_over() {
        assert_eq!(classify(1000, 0, false, 0.02), Coverage::Missing);
        assert_eq!(classify(1000, 950, false, 0.02), Coverage::Short); // -5%
        assert_eq!(classify(1000, 1050, false, 0.02), Coverage::Over); // +5%
    }

    #[test]
    fn classify_zero_expected_is_pass() {
        // Not present in our CSV, but the logic must not divide-by-zero or crash.
        assert_eq!(classify(0, 0, false, 0.02), Coverage::Pass);
    }

    #[test]
    fn classify_partial_year_allows_surplus_but_flags_shortfall() {
        // A partial year's ground truth is a snapshot floor: at or above it is
        // fine, even well above (the calendar has moved on since the snapshot).
        assert_eq!(classify(497791, 497791, true, 0.02), Coverage::Pass);
        assert_eq!(classify(497791, 520000, true, 0.02), Coverage::Pass); // +4.5%, still Pass
        // A genuine shortfall below tolerance is still Short.
        assert_eq!(classify(497791, 400000, true, 0.02), Coverage::Short);
    }

    #[test]
    fn as_u64_reads_int_and_float_cells() {
        assert_eq!(as_u64(&json!(42)), Some(42));
        assert_eq!(as_u64(&json!(42.0)), Some(42));
        assert_eq!(as_u64(&json!(-1)), Some(0));
        assert_eq!(as_u64(&json!(null)), None);
        assert_eq!(as_u64(&json!("x")), None);
    }

    #[test]
    fn api_error_prefers_structured_message() {
        assert_eq!(api_error(r#"{"error":{"status":400,"message":"bad"}}"#), "bad");
        assert_eq!(api_error("plain text failure"), "plain text failure");
    }

    #[test]
    fn publication_number_validation_guards_the_in_list() {
        assert!(valid_publication_number("494343-2026"));
        assert!(valid_publication_number("1-2011"));
        assert!(valid_publication_number("00494343-2026")); // padded form is valid too
        assert!(!valid_publication_number("494343")); // no year
        assert!(!valid_publication_number("494343-26")); // short year
        assert!(!valid_publication_number("abc-2026")); // non-digit
        assert!(!valid_publication_number("1'); DROP--2026")); // injection attempt
        assert!(!valid_publication_number("123456789-2026")); // >8 digits
    }

    #[test]
    fn has_winner_detects_a_named_winner() {
        let with = json!({ "lot_results": [ { "winners": [ { "organization_name": "ACME" } ] } ] });
        assert!(has_winner(&with));
        let no_winner = json!({ "lot_results": [ { "winners": [] } ] });
        assert!(!has_winner(&no_winner));
        let no_results = json!({ "lot_results": [] });
        assert!(!has_winner(&no_results));
        assert!(!has_winner(&json!({})));
    }

    #[test]
    fn report_ok_requires_coverage_to_have_run() {
        // Coverage skipped (no token) → not ok, even with nothing failing.
        let report = Report {
            base_url: "x".into(),
            tolerance: 0.02,
            coverage: Vec::new(),
            coverage_note: Some("skipped".into()),
            search: Vec::new(),
            era: Vec::new(),
        };
        assert!(!report.ok());
    }

    #[test]
    fn report_ok_when_all_pass() {
        let report = Report {
            base_url: "x".into(),
            tolerance: 0.02,
            coverage: vec![CoverageRow { year: 2025, expected: 100, actual: 100, partial: false, verdict: Coverage::Pass }],
            coverage_note: None,
            search: vec![SearchRow { date: "2025-01-02".into(), api_total: Some(5), held: Some(5), sampled: Some(5), verdict: Check::Pass }],
            era: vec![EraRow { label: "x".into(), era: "eforms".into(), publication_id: "1-2025".into(), verdict: Check::Pass }],
        };
        assert!(report.ok());
    }

    #[test]
    fn report_fails_on_any_failed_check() {
        let report = Report {
            base_url: "x".into(),
            tolerance: 0.02,
            coverage: vec![CoverageRow { year: 2025, expected: 100, actual: 0, partial: false, verdict: Coverage::Missing }],
            coverage_note: None,
            search: Vec::new(),
            era: Vec::new(),
        };
        assert!(!report.ok());
        // A skipped search row alone does not fail the run.
        let skipped = Report {
            base_url: "x".into(),
            tolerance: 0.02,
            coverage: vec![CoverageRow { year: 2025, expected: 100, actual: 100, partial: false, verdict: Coverage::Pass }],
            coverage_note: None,
            search: vec![SearchRow { date: "d".into(), api_total: Some(1), held: None, sampled: Some(1), verdict: Check::Skip("no token".into()) }],
            era: Vec::new(),
        };
        assert!(skipped.ok());
    }
}
