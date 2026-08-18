//! Package downloader: streams a URL into the raw archive with hash-based
//! idempotency and a registry row per file version.
//!
//! Invariants (docs/architecture.md):
//! - Archived files are immutable. A changed upstream package becomes a NEW
//!   file (`…-v2.…`) and a new registry row; the newest row is current.
//! - Idempotency is decided by our own sha256 — sources send no ETags.

use std::io::Write;
use std::path::{Path, PathBuf};

/// One package to download, addressed by its registry identity.
pub struct Target {
    pub source: &'static str,
    /// `daily` | `monthly`
    pub kind: &'static str,
    /// Registry period key, e.g. `2026-00137` (daily) or `2026-06` (monthly).
    pub period: String,
    pub url: String,
    /// Archive-relative file name, e.g. `ted/daily/2026-00137.tar.gz`.
    pub rel_path: String,
}

#[derive(Debug, PartialEq)]
pub enum Outcome {
    /// Downloaded and registered (first fetch of this period).
    Fetched,
    /// Registry already has this period and the content is unchanged.
    Unchanged,
    /// Upstream content changed: stored as a new file version + row.
    NewVersion,
    /// Not on the server (404) — e.g. probing past the newest issue.
    NotFound,
    /// The server refused the period as out of range (400) — e.g. DÖE
    /// rejecting today/future days or months before its 2022-12 archive start.
    Rejected,
}

#[derive(Debug)]
pub enum Error {
    Http(reqwest::Error),
    Status(reqwest::StatusCode),
    /// A status the server itself says is temporary: 429 Too Many Requests or 408
    /// Request Timeout, carrying its `Retry-After` seconds when it sent one.
    ///
    /// Split from [`Error::Status`] because these are the two 4xx codes that mean
    /// "ask again later", and lumping them in with 400/404 is what made a TED rate
    /// limit fail a daily probe outright (2026-08-18, job 725).
    Throttled { status: reqwest::StatusCode, retry_after: Option<u64> },
    Io(std::io::Error),
    Db(turso::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Http(e) => write!(f, "http: {e}"),
            Error::Status(s) => write!(f, "unexpected status: {s}"),
            Error::Throttled { status, retry_after } => match retry_after {
                Some(secs) => write!(f, "throttled: {status} (Retry-After: {secs}s)"),
                None => write!(f, "throttled: {status} (no Retry-After)"),
            },
            Error::Io(e) => write!(f, "io: {e}"),
            Error::Db(e) => write!(f, "db: {e}"),
        }
    }
}
impl std::error::Error for Error {}
impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::Http(e)
    }
}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}
impl From<turso::Error> for Error {
    fn from(e: turso::Error) -> Self {
        Error::Db(e)
    }
}

/// Fetch one package into `archive_root`, honouring the registry.
///
/// `refetch`: when false and a registry row exists, the download is skipped
/// entirely (backfill mode). When true the package is re-downloaded and
/// compared by hash (finality window / forced checks).
pub async fn fetch(
    db: &store::Db,
    client: &reqwest::Client,
    archive_root: &Path,
    target: &Target,
    refetch: bool,
) -> Result<Outcome, Error> {
    let existing = db.latest_fetch(target.source, target.kind, &target.period).await?;
    if existing.is_some() && !refetch {
        return Ok(Outcome::Unchanged);
    }

    let final_path = archive_root.join(&target.rel_path);
    if let Some(dir) = final_path.parent() {
        std::fs::create_dir_all(dir)?;
    }

    let (bytes, sha256) = match download(client, &target.url, &final_path).await {
        Ok(v) => v,
        Err(Error::Status(reqwest::StatusCode::NOT_FOUND)) => return Ok(Outcome::NotFound),
        Err(Error::Status(reqwest::StatusCode::BAD_REQUEST)) => return Ok(Outcome::Rejected),
        Err(e) => return Err(e),
    };

    let (outcome, rel_path) = match &existing {
        None => (Outcome::Fetched, target.rel_path.clone()),
        Some(prev) if prev.sha256 == sha256 => {
            // Same content — drop the temp file, keep the registry as is.
            let _ = std::fs::remove_file(temp_path(&final_path));
            return Ok(Outcome::Unchanged);
        }
        Some(prev) => {
            // Content changed: never overwrite the archived file — version it.
            let version = prev.path.matches("-v").count() + 2;
            (Outcome::NewVersion, versioned(&target.rel_path, version))
        }
    };

    let dest = archive_root.join(&rel_path);
    std::fs::rename(temp_path(&final_path), &dest)?;

    db.record_fetch(&store::Fetch {
        source: target.source.into(),
        kind: target.kind.into(),
        period: target.period.clone(),
        url: target.url.clone(),
        sha256,
        bytes,
        fetched_at: store::now_unix(),
        path: rel_path,
    })
    .await?;
    Ok(outcome)
}

/// Walk TED daily issues forward from the newest one already registered for the
/// current UTC year, fetching each until the server 404s past the newest
/// published issue. Returns every `(period, outcome)` it touched.
///
/// This is the realtime probe (docs/research/ted-access-channels.md §5), lifted
/// out of the fetch CLI so the in-app Supervisor and the CLI share one
/// implementation. Starting at the newest *known* issue (not the next one) means
/// `refetch = true` re-downloads it first — the finality re-check, since a daily
/// package may be rewritten until 09:30 CET on its publication day. With
/// `refetch = false` that first issue is a cheap registry hit (`Unchanged`, no
/// download) and the walk still advances to the genuinely new issues.
pub async fn probe_ted_daily(
    db: &store::Db,
    client: &reqwest::Client,
    archive_root: &Path,
    base: &str,
    refetch: bool,
    mut on_issue: impl FnMut(&str, &Outcome),
) -> Result<Vec<(String, Outcome)>, Error> {
    let year = current_date_utc().0;
    let mut issue = latest_ted_issue(db, year).await?.unwrap_or(1);
    let mut out = Vec::new();
    loop {
        let target = crate::ted::daily(base, year, issue);
        let outcome = fetch(db, client, archive_root, &target, refetch).await?;
        on_issue(&target.period, &outcome);
        let stop = matches!(outcome, Outcome::NotFound);
        out.push((target.period.clone(), outcome));
        if stop {
            break;
        }
        issue += 1;
    }
    Ok(out)
}

/// Newest daily issue number already registered for `year`. Periods sort
/// lexicographically (`YYYY-NNNNN`), so `MAX(period)` is the newest.
pub async fn latest_ted_issue(db: &store::Db, year: u16) -> turso::Result<Option<u32>> {
    let latest = db.latest_fetch_period_max("ted", "daily", &format!("{year}-")).await?;
    Ok(latest.and_then(|p| p.split_once('-').and_then(|(_, n)| n.parse().ok())))
}

/// Walk DÖE daily exports forward from the newest day already registered up to
/// and including `end` (the last completed T+1 day), fetching each. A normal run
/// advances a single day; a gap since the last successful fetch catches up every
/// missed day. DÖE has no server-side "next issue" probe like TED, so this
/// last-watermark→forward walk is what stops a skipped scheduler tick from
/// silently dropping a day (issue 69 / ADR-0004 completeness).
///
/// The walk starts the day *after* the watermark, not at it: a DÖE completed day
/// is final once fetchable (strictly T+1), so there is no TED-style finality
/// re-check to do. And it never triggers a full-archive backfill — with no DÖE
/// daily on record it fetches only `end` (a single day), leaving the monthly
/// backfill job to seed history.
pub async fn probe_doe_daily(
    db: &store::Db,
    client: &reqwest::Client,
    archive_root: &Path,
    base: &str,
    end: (u16, u8, u8),
    mut on_day: impl FnMut(&str, &Outcome),
) -> Result<Vec<(String, Outcome)>, Error> {
    let mut day = match latest_doe_day(db).await? {
        Some(prev) => next_civil_day(prev),
        None => end,
    };
    let mut out = Vec::new();
    while day <= end {
        let target = crate::doe::day(base, day);
        let outcome = fetch(db, client, archive_root, &target, false).await?;
        on_day(&target.period, &outcome);
        out.push((target.period.clone(), outcome));
        day = next_civil_day(day);
    }
    Ok(out)
}

/// Newest DÖE daily day already registered. Periods are zero-padded `YYYY-MM-DD`,
/// so `MAX(period)` is the newest across every year.
pub async fn latest_doe_day(db: &store::Db) -> turso::Result<Option<(u16, u8, u8)>> {
    let latest = db.latest_fetch_period_max("doe", "daily", "").await?;
    Ok(latest.as_deref().and_then(parse_ymd))
}

/// Parse a zero-padded `YYYY-MM-DD` period into a civil date.
fn parse_ymd(period: &str) -> Option<(u16, u8, u8)> {
    let (y, rest) = period.split_once('-')?;
    let (m, d) = rest.split_once('-')?;
    Some((y.parse().ok()?, m.parse().ok()?, d.parse().ok()?))
}

/// The calendar day after `date` (proleptic Gregorian), via the civil-date
/// round-trip so month/year rollovers fall out for free.
fn next_civil_day(date: (u16, u8, u8)) -> (u16, u8, u8) {
    let (y, m, d) = date;
    civil_date((days_from_civil(y, m, d) + 1) * 86_400)
}

/// Today as (year, month, day) UTC.
pub fn current_date_utc() -> (u16, u8, u8) {
    civil_date(store::now_unix())
}

/// The (year, month, day) UTC of a unix instant — Howard Hinnant's days→civil
/// algorithm (no date-library dependency).
pub fn civil_date(unix_seconds: i64) -> (u16, u8, u8) {
    let days = unix_seconds.div_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    ((y + i64::from(m <= 2)) as u16, m as u8, d as u8)
}

/// The inverse of [`civil_date`]: days since 1970-01-01 for a calendar date —
/// Howard Hinnant's `days_from_civil` (proleptic Gregorian, no date dependency).
/// The one civil-date helper the ingest parsers and the server's DST math share
/// (issue 38, unifying two identical copies).
pub fn days_from_civil(year: u16, month: u8, day: u8) -> i64 {
    let y = i64::from(year) - i64::from(month <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m = i64::from(month);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Stream the URL to `<final_path>.part`, resuming a previous partial
/// download via a Range request. Returns (bytes, sha256-hex).
async fn download(
    client: &reqwest::Client,
    url: &str,
    final_path: &Path,
) -> Result<(i64, String), Error> {
    let part = temp_path(final_path);

    let mut attempt = 0u32;
    loop {
        attempt += 1;
        match download_once(client, url, &part).await {
            Ok(v) => return Ok(v),
            // Client errors are permanent — retrying a 400/404 just wastes time.
            // 429/408 are NOT in this class: they arrive as `Throttled`, below.
            Err(e @ Error::Status(s)) if s.is_client_error() => return Err(e),
            // A rate limit is the one 4xx that asks to be retried, and TED does
            // send them: a 429 on the 2026-08-18 daily probe (job 725) failed the
            // job outright because `is_client_error()` swallowed the retry. Honour
            // the server's own `Retry-After` when it sends one — capped, so a wild
            // or hostile value cannot park the queue — and fall back to a longer
            // backoff than the generic one, since a limit that just tripped will
            // still be tripped two seconds later.
            Err(e @ Error::Throttled { .. }) if attempt >= THROTTLE_ATTEMPTS => return Err(e),
            Err(Error::Throttled { retry_after, .. }) => {
                let wait = retry_after
                    .unwrap_or(THROTTLE_BACKOFF_SECS * attempt as u64)
                    .min(THROTTLE_WAIT_CAP_SECS);
                eprintln!("[fetch] throttled on {url}, waiting {wait}s (attempt {attempt})");
                tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
            }
            Err(e) if attempt >= 3 => return Err(e),
            Err(_) => tokio::time::sleep(std::time::Duration::from_secs(2 * attempt as u64)).await,
        }
    }
}

/// How many times a throttled request is re-attempted before the job fails.
///
/// More than the generic 3 because the wait is the point: a rate limit clears on
/// the server's schedule, not ours, and a daily probe that gives up after six
/// seconds loses the day's notices for the sake of finishing early.
const THROTTLE_ATTEMPTS: u32 = 5;
/// Base backoff when the server sends no `Retry-After` (multiplied by attempt).
const THROTTLE_BACKOFF_SECS: u64 = 15;
/// Longest single wait honoured, whatever `Retry-After` claims. Jobs are
/// serialized, so an unbounded sleep here is an unbounded queue stall.
const THROTTLE_WAIT_CAP_SECS: u64 = 120;

/// What a response status means for the download: append to the partial file,
/// overwrite it, or fail — and if fail, whether it is worth asking again.
///
/// Its own function so the classification is testable without a server, because
/// the distinction it draws is the whole point: 429 and 408 are the two 4xx codes
/// that mean "later", and treating them like 404 cost a daily probe its day
/// (job 725, 2026-08-18). `retry_after` is a closure so the header is only read
/// when the status is one that carries it.
fn classify_status(
    status: reqwest::StatusCode,
    retry_after: impl FnOnce() -> Option<u64>,
) -> Result<bool, Error> {
    match status {
        reqwest::StatusCode::OK => Ok(false), // full body (server ignored/no Range)
        reqwest::StatusCode::PARTIAL_CONTENT => Ok(true),
        reqwest::StatusCode::TOO_MANY_REQUESTS | reqwest::StatusCode::REQUEST_TIMEOUT => {
            Err(Error::Throttled { status, retry_after: retry_after() })
        }
        status => Err(Error::Status(status)),
    }
}

/// `Retry-After` in seconds, when the server sent it as a delay.
///
/// The header also permits an HTTP-date, which is deliberately NOT parsed: a date
/// requires trusting the server's clock against ours, and the fallback backoff is
/// a fine answer when the format is one we do not read.
fn retry_after_secs(headers: &reqwest::header::HeaderMap) -> Option<u64> {
    headers
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()
}

async fn download_once(
    client: &reqwest::Client,
    url: &str,
    part: &Path,
) -> Result<(i64, String), Error> {
    let resume_from = std::fs::metadata(part).map(|m| m.len()).unwrap_or(0);
    let mut req = client.get(url);
    if resume_from > 0 {
        req = req.header(reqwest::header::RANGE, format!("bytes={resume_from}-"));
    }
    let mut resp = req.send().await?;

    let append = classify_status(resp.status(), || retry_after_secs(resp.headers()))?;

    let mut file = if append && resume_from > 0 {
        std::fs::OpenOptions::new().append(true).open(part)?
    } else {
        std::fs::File::create(part)?
    };
    while let Some(chunk) = resp.chunk().await? {
        file.write_all(&chunk)?;
    }
    file.flush()?;
    drop(file);

    // Hash the complete file (covers the resumed case uniformly).
    let data = std::fs::read(part)?;
    Ok((data.len() as i64, crate::sha256_hex(&data)))
}

/// What [`register_archive`] did, for the job log.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Registered {
    /// Files hashed and recorded — periods the registry did not know.
    pub registered: i64,
    /// Periods the registry already had — skipped WITHOUT hashing (cheap re-run).
    pub existing: i64,
    /// Entries that do not match the `<source>/<kind>/<period>[.ext]` layout.
    pub unrecognised: i64,
}

/// Rebuild the `fetches` registry from the on-disk archive (issue 23 / the DR
/// premise's load-bearing finding): a lost DB forced a full ~180 GB re-download
/// even with `/data/archive` intact, because `fetch()` decides idempotency from
/// `latest_fetch(...)` and `process` walks packages via `fetches` rows — the
/// archive could not function as the source of truth ADR-0001 calls it. This
/// walks `<archive_root>/<source>/<kind>/`, hashes each package whose period the
/// registry lacks, and `record_fetch`s it, after which a fresh DB can `process`
/// the whole archive with zero downloads.
///
/// Provenance honesty: the original URL is gone, so the row records
/// `archive://<rel_path>`; `fetched_at` is the file's mtime (when the bytes
/// arrived, as the filesystem remembers it), never "now". Version files
/// (`-v<N>`, finality rewrites) register in version order under their one
/// period, so `latest_fetch` resolves to the newest content exactly as the live
/// history would have left it. A period the registry already knows is skipped
/// without hashing — re-runs are cheap and never overwrite real provenance.
pub async fn register_archive(db: &store::Db, archive_root: &Path) -> Result<Registered, Error> {
    let mut summary = Registered::default();
    for source in ["ted", "doe"] {
        for kind in ["daily", "monthly"] {
            let dir = archive_root.join(source).join(kind);
            let entries = match std::fs::read_dir(&dir) {
                Ok(e) => e,
                Err(_) => continue, // a source without this kind is normal
            };
            // Group by period so version files land in order under one identity.
            let mut by_period: std::collections::BTreeMap<String, Vec<(usize, PathBuf)>> =
                std::collections::BTreeMap::new();
            for entry in entries {
                let path = entry?.path();
                let Some(name) = path.file_name().and_then(|n| n.to_str()) else { continue };
                if !path.is_file() {
                    continue; // nested dirs are not archive packages
                }
                if name.ends_with(".part") {
                    // Interrupted-download debris — not content, not an anomaly.
                    continue;
                }
                let stem = name.split_once('.').map_or(name, |(s, _)| s);
                let (period, version) = match stem.rsplit_once("-v") {
                    Some((p, v)) if v.chars().all(|c| c.is_ascii_digit()) && !v.is_empty() => {
                        (p.to_owned(), v.parse().unwrap_or(1))
                    }
                    _ => (stem.to_owned(), 1),
                };
                if period.is_empty() {
                    summary.unrecognised += 1;
                    continue;
                }
                by_period.entry(period).or_default().push((version, path));
            }
            for (period, mut files) in by_period {
                if db.latest_fetch(source, kind, &period).await?.is_some() {
                    summary.existing += 1;
                    continue;
                }
                files.sort_by_key(|(version, _)| *version);
                for (_, path) in files {
                    let data = std::fs::read(&path)?;
                    let rel_path = format!(
                        "{source}/{kind}/{}",
                        path.file_name().and_then(|n| n.to_str()).unwrap_or_default()
                    );
                    let fetched_at = std::fs::metadata(&path)?
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or_else(store::now_unix);
                    db.record_fetch(&store::Fetch {
                        source: source.into(),
                        kind: kind.into(),
                        period: period.clone(),
                        url: format!("archive://{rel_path}"),
                        sha256: crate::sha256_hex(&data),
                        bytes: data.len() as i64,
                        fetched_at,
                        path: rel_path,
                    })
                    .await?;
                    summary.registered += 1;
                }
            }
        }
    }
    Ok(summary)
}

fn temp_path(final_path: &Path) -> PathBuf {
    let mut p = final_path.as_os_str().to_owned();
    p.push(".part");
    PathBuf::from(p)
}

/// `ted/daily/2026-00137.tar.gz` → `ted/daily/2026-00137-v2.tar.gz`
fn versioned(rel_path: &str, version: usize) -> String {
    match rel_path.split_once('.') {
        Some((stem, ext)) => format!("{stem}-v{version}.{ext}"),
        None => format!("{rel_path}-v{version}"),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        civil_date, classify_status, days_from_civil, retry_after_secs, versioned, Error,
        THROTTLE_ATTEMPTS, THROTTLE_BACKOFF_SECS, THROTTLE_WAIT_CAP_SECS,
    };

    #[test]
    fn versioned_filenames() {
        assert_eq!(versioned("ted/daily/2026-00137.tar.gz", 2), "ted/daily/2026-00137-v2.tar.gz");
        assert_eq!(versioned("plain", 3), "plain-v3");
    }

    #[test]
    fn days_from_civil_is_the_inverse_of_civil_date() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2000, 3, 1), 11017);
        for &(y, m, d) in &[(1970u16, 1u8, 1u8), (2000, 2, 29), (2026, 7, 19), (1993, 1, 1)] {
            assert_eq!(civil_date(days_from_civil(y, m, d) * 86_400), (y, m, d));
        }
    }

    /// The rule a TED rate limit taught on 2026-08-18 (job 725, `ted daily (probe)`
    /// → `unexpected status: 429`): 429 and 408 are the two 4xx codes that mean
    /// "ask again later", and `download`'s "client errors are permanent" shortcut
    /// was swallowing the retry for exactly them. A 429 that arrives before the
    /// fetch loses the day's notices and leaves only a red probe row to say so.
    #[test]
    fn a_rate_limit_is_retryable_where_a_404_is_not() {
        use reqwest::StatusCode;

        // The success shapes, unchanged: 206 appends to the partial file, 200 does not.
        assert_eq!(classify_status(StatusCode::OK, || None).unwrap(), false);
        assert_eq!(classify_status(StatusCode::PARTIAL_CONTENT, || None).unwrap(), true);

        // Throttled, and the server's own delay is carried through.
        for status in [StatusCode::TOO_MANY_REQUESTS, StatusCode::REQUEST_TIMEOUT] {
            match classify_status(status, || Some(30)) {
                Err(Error::Throttled { status: got, retry_after: Some(30) }) => {
                    assert_eq!(got, status);
                }
                other => panic!("{status} must be throttled with its delay, got {other:?}"),
            }
        }
        // No `Retry-After` is fine — the backoff covers it.
        assert!(matches!(
            classify_status(StatusCode::TOO_MANY_REQUESTS, || None),
            Err(Error::Throttled { retry_after: None, .. })
        ));

        // The genuinely permanent ones stay permanent, which is the other half of
        // the rule: retrying a 404 forever is how a probe hangs on a day that will
        // never exist.
        for status in [StatusCode::NOT_FOUND, StatusCode::BAD_REQUEST, StatusCode::FORBIDDEN] {
            assert!(
                matches!(classify_status(status, || None), Err(Error::Status(s)) if s == status),
                "{status} is permanent"
            );
        }

        // A server error is neither: it takes `download`'s generic short backoff.
        assert!(matches!(
            classify_status(StatusCode::INTERNAL_SERVER_ERROR, || None),
            Err(Error::Status(s)) if s.is_server_error()
        ));

        // The waits are bounded, because jobs are serialized and a sleep here is a
        // queue stall.
        assert!(THROTTLE_WAIT_CAP_SECS >= THROTTLE_BACKOFF_SECS);
        assert!(THROTTLE_ATTEMPTS * (THROTTLE_WAIT_CAP_SECS as u32) < 900, "worst case under 15min");
    }

    /// Seconds only. The header also permits an HTTP-date, and reading one would
    /// mean trusting the server's clock against ours for a value the fallback
    /// backoff already covers.
    #[test]
    fn retry_after_reads_a_delay_and_ignores_a_date() {
        let headers = |v: &str| {
            let mut h = reqwest::header::HeaderMap::new();
            h.insert(reqwest::header::RETRY_AFTER, v.parse().expect("header value"));
            h
        };
        assert_eq!(retry_after_secs(&headers("30")), Some(30));
        assert_eq!(retry_after_secs(&headers("  7 ")), Some(7));
        assert_eq!(retry_after_secs(&headers("Wed, 21 Oct 2026 07:28:00 GMT")), None);
        assert_eq!(retry_after_secs(&headers("-1")), None);
        assert_eq!(retry_after_secs(&reqwest::header::HeaderMap::new()), None);
    }
}
