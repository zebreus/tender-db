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
    /// A status the server itself says is temporary: 429 Too Many Requests, 408
    /// Request Timeout or 503 Service Unavailable, carrying its `Retry-After`
    /// seconds when it sent one.
    ///
    /// Split from [`Error::Status`] because these are the codes that mean "ask
    /// again later", and lumping them in with 400/404 is what made a TED rate
    /// limit fail a daily probe outright (2026-08-18, job 725). 503 joined on
    /// FTS's word (docs/research/uk-fts.md §1: handle it exactly like 429).
    Throttled { status: reqwest::StatusCode, retry_after: Option<u64> },
    Io(std::io::Error),
    Db(turso::Error),
    /// The target's source is not what this fetcher serves — FTS windows are
    /// paged and assembled by [`fetch_fts`], and [`fetch`] refuses them rather
    /// than archiving a raw first page under the package's name.
    Unsupported(&'static str),
    /// A 200 whose body is not the shape the source documents: an FTS page
    /// without a `releases` array, a release without a usable `id`. Not
    /// retried — the bytes are what they are — and the job fails with its
    /// staging intact for a person to look at.
    Malformed(String),
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
            Error::Unsupported(what) => write!(f, "unsupported: {what}"),
            Error::Malformed(what) => write!(f, "malformed response: {what}"),
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
    if target.source == "fts" {
        // FTS is paged and assembled by `fetch_fts`; streaming its first page to
        // disk under the zip's name would archive a page as if it were the package.
        return Err(Error::Unsupported("fts is a paged source: use fetch_fts"));
    }
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
    land(db, archive_root, target, existing.as_ref(), &final_path, bytes, sha256).await
}

/// The immutability rules, shared by every fetcher: the bytes waiting in
/// `<final_path>.part` either land as the period's first file, are dropped
/// because the registry's newest row already carries this hash, or land as a
/// NEW `-vN` file beside the original — and the registry row is written only
/// once the bytes are in place under their final name.
async fn land(
    db: &store::Db,
    archive_root: &Path,
    target: &Target,
    existing: Option<&store::Fetch>,
    final_path: &Path,
    bytes: i64,
    sha256: String,
) -> Result<Outcome, Error> {
    let (outcome, rel_path) = match existing {
        None => (Outcome::Fetched, target.rel_path.clone()),
        Some(prev) if prev.sha256 == sha256 => {
            // Same content — drop the temp file, keep the registry as is.
            let _ = std::fs::remove_file(temp_path(final_path));
            return Ok(Outcome::Unchanged);
        }
        Some(prev) => {
            // Content changed: never overwrite the archived file — version it.
            let version = prev.path.matches("-v").count() + 2;
            (Outcome::NewVersion, versioned(&target.rel_path, version))
        }
    };

    let dest = archive_root.join(&rel_path);
    std::fs::rename(temp_path(final_path), &dest)?;

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

/// Fetch one FTS package (docs/research/uk-fts.md §2, plan D8): walk every
/// request window of `target` page by page, staging each page verbatim under
/// `<archive>/<rel_path minus .zip>.pages/`, then assemble ONE zip — one member
/// `<release id>.json` per release ([`crate::fts::member_bytes`]), sorted by
/// id, the first occurrence winning on a duplicate — and land it under the same
/// immutability rules as [`fetch`].
///
/// Resumable: `cursor.json` in the staging dir records the window being walked,
/// how many of its pages are on disk, the `links.next` to ask for, and the
/// windows already complete. A page is written BEFORE the cursor advances, so
/// an interruption at any point leaves a state the next run continues from — a
/// month that hit the limiter resumes at its page, not its start — and page 1 is
/// never asked for twice. The staging dir is removed only after the zip has
/// landed; an `Err` (five throttled attempts, a malformed page) leaves it
/// intact, and the `fetches` row is written only for a complete window.
///
/// `refetch` is as [`fetch`]: false skips a registered period without HTTP.
/// `page_pause` separates consecutive requests — `fts::PAGE_PAUSE_SECS` in
/// production, zero in tests. `on_progress(day, pages, releases)` fires after
/// every page with the window's running totals.
///
/// An empty window (a Sunday, December 2020) still lands a 0-member zip, so
/// `MAX(period)` advances and the daily walk never re-asks for the day.
pub async fn fetch_fts(
    db: &store::Db,
    client: &reqwest::Client,
    archive_root: &Path,
    target: &Target,
    refetch: bool,
    page_pause: std::time::Duration,
    mut on_progress: impl FnMut(&str, usize, usize),
) -> Result<Outcome, Error> {
    let base = match crate::fts::base_of(&target.url) {
        Some(base) if target.source == "fts" => base,
        _ => return Err(Error::Unsupported("fetch_fts serves fts window targets only")),
    };
    let windows = crate::fts::windows(base, target);
    if windows.is_empty() {
        return Err(Error::Malformed(format!(
            "no request windows for {} {} {:?}",
            target.source, target.kind, target.period
        )));
    }
    let existing = db.latest_fetch(target.source, target.kind, &target.period).await?;
    if existing.is_some() && !refetch {
        return Ok(Outcome::Unchanged);
    }

    let staging = staging_dir(archive_root, &target.rel_path);
    // Debris, not a resume point (issue 342 review, lens "fetcher"): a staging
    // dir whose cursor predates the registered landing is what an interrupted
    // `remove_dir_all` (or a crash between the row and the cleanup) left behind,
    // and its `done` list would make this refetch skip windows it must re-walk.
    // A cursor NEWER than the row is an interrupted refetch and is resumed.
    if let Some(existing) = &existing
        && let Ok(meta) = std::fs::metadata(staging.join(CURSOR_FILE))
        && let Ok(modified) = meta.modified()
        && let Ok(age) = modified.duration_since(std::time::UNIX_EPOCH)
        && (age.as_secs() as i64) < existing.fetched_at
    {
        std::fs::remove_dir_all(&staging)?;
    }
    std::fs::create_dir_all(&staging)?;
    let cursor_path = staging.join(CURSOR_FILE);
    let mut cursor = read_cursor(&cursor_path);
    let mut requests = 0usize;
    for window in &windows {
        let day = crate::fts::ymd(window.day);
        if cursor.done.iter().any(|done| *done == day) {
            continue; // its pages are on disk from an earlier run
        }
        // Resume mid-window at the recorded `next`; otherwise the window's first page.
        let (mut page, mut next) = if cursor.day == day && cursor.page > 0 {
            (cursor.page, cursor.next.clone())
        } else {
            (0, Some(window.url.clone()))
        };
        let mut releases = 0usize;
        while let Some(url) = next {
            if requests > 0 {
                tokio::time::sleep(page_pause).await;
            }
            requests += 1;
            let bytes = get_bytes(client, &url).await?;
            let (count, links_next) =
                page_summary(&bytes).map_err(|what| Error::Malformed(format!("{url}: {what}")))?;
            page += 1;
            releases += count;
            write_atomic(&staging.join(format!("{day}-p{page:03}.json")), &bytes)?;
            next = links_next;
            cursor.day.clone_from(&day);
            cursor.page = page;
            cursor.next.clone_from(&next);
            write_cursor(&cursor_path, &cursor)?;
            on_progress(&day, page as usize, releases);
        }
        cursor.done.push(day);
        cursor.day.clear();
        cursor.page = 0;
        cursor.next = None;
        write_cursor(&cursor_path, &cursor)?;
    }

    let final_path = archive_root.join(&target.rel_path);
    if let Some(dir) = final_path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let (bytes, sha256) = assemble_fts_zip(&staging, &temp_path(&final_path))?;
    let outcome = land(db, archive_root, target, existing.as_ref(), &final_path, bytes, sha256).await?;
    std::fs::remove_dir_all(&staging)?;
    Ok(outcome)
}

/// Walk FTS daily windows forward from the newest day already registered up to
/// and including `end` (yesterday in UK civil time), fetching each — the DÖE
/// walk-forward shape ([`probe_doe_daily`]): a normal run advances one day, a
/// gap catches up every missed day, and with no FTS daily on record it fetches
/// only `end`, leaving the monthly backfill to seed history.
///
/// It never refetches: the API filters on a release's last-update instant,
/// which cannot fall into a UK day that is over, so a passed day is final; a
/// tick that ran late is covered by the next day's 2 h overlap. Days are paced
/// by `page_pause` like pages, so a multi-day catch-up never bursts the limiter.
pub async fn probe_fts_daily(
    db: &store::Db,
    client: &reqwest::Client,
    archive_root: &Path,
    base: &str,
    end: (u16, u8, u8),
    page_pause: std::time::Duration,
    mut on_day: impl FnMut(&str, &Outcome),
) -> Result<Vec<(String, Outcome)>, Error> {
    let mut day = match latest_fts_day(db).await? {
        Some(prev) => next_civil_day(prev),
        None => end,
    };
    let mut out = Vec::new();
    while day <= end {
        // CAPPED PER TICK (issue 342 review, lens "ops"). An FTS day is 4–5
        // paced requests, up to eight minutes of back-off if the limiter is
        // unhappy — unlike a DÖE day, which is one download. Uncapped, a
        // watermark left far in the past (a month of failed ticks, a restored
        // registry) would hold the single job runner for hours and park
        // `fetch-rates`, `project` and the fold behind it. The watermark
        // advances per landed day, so the remainder is simply the next tick's
        // work; a real gap closes in days, and `fetch fts --day` or a monthly
        // backfill closes it at once.
        if out.len() >= crate::fts::PROBE_DAY_CAP {
            break;
        }
        if !out.is_empty() {
            tokio::time::sleep(page_pause).await;
        }
        let target = crate::fts::day(base, day);
        let outcome = fetch_fts(db, client, archive_root, &target, false, page_pause, |_, _, _| {}).await?;
        on_day(&target.period, &outcome);
        out.push((target.period.clone(), outcome));
        day = next_civil_day(day);
    }
    Ok(out)
}

/// Newest FTS daily day already registered. Periods are zero-padded
/// `YYYY-MM-DD`, so `MAX(period)` is the newest across every year.
pub async fn latest_fts_day(db: &store::Db) -> turso::Result<Option<(u16, u8, u8)>> {
    let latest = db.latest_fetch_period_max("fts", "daily", "").await?;
    Ok(latest.as_deref().and_then(parse_ymd))
}

/// The walk's progress file inside the staging dir.
const CURSOR_FILE: &str = "cursor.json";

/// Where a paged package is staged while its windows are walked:
/// `<archive>/<rel_path minus .zip>.pages/`, beside the package it becomes.
/// A directory, so [`register_archive`] steps over it as a non-file.
fn staging_dir(archive_root: &Path, rel_path: &str) -> PathBuf {
    let stem = rel_path.strip_suffix(".zip").unwrap_or(rel_path);
    archive_root.join(format!("{stem}.pages"))
}

/// `cursor.json`: the window being walked (`day`, its `page` count on disk, the
/// `next` URL to ask for) and the windows already `done`. Every field defaults
/// so a hand-edited or older cursor still reads.
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct PageCursor {
    #[serde(default)]
    day: String,
    #[serde(default)]
    page: u32,
    #[serde(default)]
    next: Option<String>,
    #[serde(default)]
    done: Vec<String>,
}

fn read_cursor(path: &Path) -> PageCursor {
    let Ok(bytes) = std::fs::read(path) else { return PageCursor::default() };
    match serde_json::from_slice(&bytes) {
        Ok(cursor) => cursor,
        Err(e) => {
            // Restarting the walk re-fetches pages that are on disk; it never
            // loses anything, and a cursor nobody can read is not worth trusting.
            eprintln!("[fetch] {}: unreadable cursor ({e}), restarting the walk", path.display());
            PageCursor::default()
        }
    }
}

fn write_cursor(path: &Path, cursor: &PageCursor) -> Result<(), Error> {
    write_atomic(path, &serde_json::to_vec(cursor).expect("a cursor always serialises"))
}

/// Write via `<path>.part` + rename, so a page or cursor that was interrupted
/// mid-write never reads as a saved one.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    let tmp = temp_path(path);
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// What a page says about the walk: how many releases it holds and where the
/// next page is. Anything without a `releases` array is not a release package.
fn page_summary(bytes: &[u8]) -> Result<(usize, Option<String>), String> {
    let page = crate::fts::Page::read(bytes)?;
    Ok((page.releases()?.len(), page.next()))
}

/// Assemble the staged pages into `part`: members `<release id>.json` built by
/// [`crate::fts::member_bytes`], sorted by id, the first occurrence winning on
/// a duplicate id (the 2 h overlap re-serves the previous day's tail, and pages
/// are newest-first). Returns the written zip's (bytes, sha256-hex). A staging
/// dir with no releases yields a valid 0-member zip.
fn assemble_fts_zip(staging: &Path, part: &Path) -> Result<(i64, String), Error> {
    let mut pages: Vec<PathBuf> = std::fs::read_dir(staging)?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| {
            path.is_file()
                && path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.ends_with(".json") && n != CURSOR_FILE)
        })
        .collect();
    pages.sort(); // `<day>-p<NNN>.json`: window order, then page order
    let mut members: std::collections::BTreeMap<String, Vec<u8>> = std::collections::BTreeMap::new();
    for path in &pages {
        let malformed = |what: String| Error::Malformed(format!("{}: {what}", path.display()));
        let bytes = std::fs::read(path)?;
        let page = crate::fts::Page::read(&bytes).map_err(malformed)?;
        let releases = page.releases().map_err(malformed)?;
        for (index, release) in releases.iter().enumerate() {
            let id = match crate::fts::release_id(release) {
                // The id becomes a member name; a separator in it would name a directory.
                Some(id) if !id.is_empty() && !id.contains(['/', '\\']) => id,
                // A release the publisher sent without a usable id is ARCHIVED,
                // not thrown (issue 342 review, lens "fetcher"). Failing the
                // package here would be deterministic: the day would fail every
                // tick, the watermark would never advance, and one malformed
                // release would stop the whole walk-forward. Under a reserved
                // `_noid/` prefix it reaches the profile layer, which quarantines
                // it as a missing publication id with the bytes intact — the
                // publisher's defect, recorded where defects are recorded.
                _ => {
                    let stem = path.file_stem().and_then(|n| n.to_str()).unwrap_or("page");
                    format!("_noid/{stem}-{index:03}")
                }
            };
            members.entry(id).or_insert_with(|| page.member_bytes(release));
        }
    }

    let mut zip = zip::ZipWriter::new(std::fs::File::create(part)?);
    // A fixed entry timestamp: the zip's bytes are its registry identity, and a
    // window re-walked with `refetch` must hash equal when nothing changed.
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .last_modified_time(zip::DateTime::default());
    for (id, bytes) in &members {
        zip.start_file(format!("{id}.json"), opts).map_err(zip_error)?;
        zip.write_all(bytes)?;
    }
    let mut file = zip.finish().map_err(zip_error)?;
    file.flush()?;
    drop(file);

    let data = std::fs::read(part)?;
    Ok((data.len() as i64, crate::sha256_hex(&data)))
}

fn zip_error(e: zip::result::ZipError) -> Error {
    Error::Io(std::io::Error::other(e))
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
    retrying(url, || download_once(client, url, &part)).await
}

/// GET a URL into memory — one FTS page — under the same retry policy as
/// [`download`], without a file: a page is at most ~1 MB and is staged by the
/// caller only once it has parsed.
pub(crate) async fn get_bytes(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, Error> {
    retrying(url, || get_once(client, url)).await
}

async fn get_once(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, Error> {
    let resp = client.get(url).send().await?;
    classify_status(resp.status(), || retry_after_secs(resp.headers()))?;
    Ok(resp.bytes().await?.to_vec())
}

/// The retry policy every request shares — [`download`] and [`get_bytes`]
/// differ only in what one attempt does, so the policy lives once.
async fn retrying<T, Fut>(url: &str, mut attempt_once: impl FnMut() -> Fut) -> Result<T, Error>
where
    Fut: std::future::Future<Output = Result<T, Error>>,
{
    let mut attempt = 0u32;
    loop {
        attempt += 1;
        match attempt_once().await {
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
/// (job 725, 2026-08-18). 503 is in the same class on FTS's documented word
/// (uk-fts.md §1: "no further requests until after Retry-After", 503 handled the
/// same) — its limiter sends both. `retry_after` is a closure so the header is
/// only read when the status is one that carries it.
fn classify_status(
    status: reqwest::StatusCode,
    retry_after: impl FnOnce() -> Option<u64>,
) -> Result<bool, Error> {
    match status {
        reqwest::StatusCode::OK => Ok(false), // full body (server ignored/no Range)
        reqwest::StatusCode::PARTIAL_CONTENT => Ok(true),
        reqwest::StatusCode::TOO_MANY_REQUESTS
        | reqwest::StatusCode::REQUEST_TIMEOUT
        | reqwest::StatusCode::SERVICE_UNAVAILABLE => {
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
    for source in ["ted", "doe", "fts"] {
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
        civil_date, classify_status, days_from_civil, retry_after_secs, staging_dir, versioned,
        Error, THROTTLE_ATTEMPTS, THROTTLE_BACKOFF_SECS, THROTTLE_WAIT_CAP_SECS,
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

    /// FTS documents 503 as "handle exactly like 429: wait `Retry-After`"
    /// (docs/research/uk-fts.md §1), and its limiter is the one that actually
    /// sends both — so 503 joins the throttled class with the server's delay
    /// carried through, while the other 5xx keep the generic short backoff.
    #[test]
    fn a_503_is_throttled_like_a_429() {
        use reqwest::StatusCode;
        match classify_status(StatusCode::SERVICE_UNAVAILABLE, || Some(120)) {
            Err(Error::Throttled { status, retry_after: Some(120) }) => {
                assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
            }
            other => panic!("503 must be throttled with its delay, got {other:?}"),
        }
        assert!(matches!(
            classify_status(StatusCode::SERVICE_UNAVAILABLE, || None),
            Err(Error::Throttled { retry_after: None, .. })
        ));
        for status in [StatusCode::INTERNAL_SERVER_ERROR, StatusCode::BAD_GATEWAY, StatusCode::GATEWAY_TIMEOUT] {
            assert!(
                matches!(classify_status(status, || None), Err(Error::Status(s)) if s == status),
                "{status} takes the generic backoff, not the throttle wait"
            );
        }
    }

    /// The staging dir sits beside the package it becomes, named so a human
    /// reading the archive sees which zip it belongs to, and so it is a
    /// DIRECTORY `register_archive` steps over rather than a file it misreads.
    #[test]
    fn staging_dir_sits_beside_its_package() {
        let root = std::path::Path::new("/archive");
        assert_eq!(
            staging_dir(root, "fts/daily/2026-09-03.zip"),
            std::path::PathBuf::from("/archive/fts/daily/2026-09-03.pages")
        );
        assert_eq!(
            staging_dir(root, "fts/monthly/2025-06.zip"),
            std::path::PathBuf::from("/archive/fts/monthly/2025-06.pages")
        );
        assert_eq!(staging_dir(root, "odd/name"), std::path::PathBuf::from("/archive/odd/name.pages"));
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
