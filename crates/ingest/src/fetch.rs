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
    Io(std::io::Error),
    Db(turso::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Http(e) => write!(f, "http: {e}"),
            Error::Status(s) => write!(f, "unexpected status: {s}"),
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
            Err(e @ Error::Status(s)) if s.is_client_error() => return Err(e),
            Err(e) if attempt >= 3 => return Err(e),
            Err(_) => tokio::time::sleep(std::time::Duration::from_secs(2 * attempt as u64)).await,
        }
    }
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

    let append = match resp.status() {
        reqwest::StatusCode::OK => false, // full body (server ignored/no Range)
        reqwest::StatusCode::PARTIAL_CONTENT => true,
        status => return Err(Error::Status(status)),
    };

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
    use super::{civil_date, days_from_civil, versioned};

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
}
