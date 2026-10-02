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
    /// A cancel stopped a paged walk between two requests (issue 450). Nothing
    /// landed; the staged span pages stay exactly as a crash would leave them,
    /// so fetching the same target again resumes where it stopped.
    Stopped,
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
    /// without a `releases` array — or (issue 477) an FTS span still full at
    /// two seconds, the shortest window the API answers, whose notices the
    /// dense-span walk cannot account for by records (several notice ids on
    /// the page, a record that contradicts the listing, a hole in a run,
    /// another notice dated inside the span, a run past
    /// [`crate::fts::DENSE_RUN_CAP`]). Not retried — the bytes are what they
    /// are — and the job fails with its staging intact for a person to look at.
    Malformed(String),
    /// Issue 451: an edge WAF answered instead of the origin. TED's CloudFront
    /// returns 202 with an empty body and `x-amzn-waf-action: challenge` to a
    /// client it takes for an unidentified bot (since 2026-09-30, for any
    /// request without a Mozilla-compatible User-Agent). Not retried: the
    /// same request gets the same challenge.
    Challenged { status: reqwest::StatusCode, action: String },
}

/// Issue 451: the User-Agent every source fetch sends, in the crawler
/// convention (`Mozilla/5.0 (compatible; <bot>/<version>; +<url>)`). It names
/// the tool and where to find it, so a source can still tell these requests
/// apart and refuse them; reqwest sends no User-Agent at all by default, which
/// TED's WAF answers with a challenge. FTS, DÖE and the ECB accept it too
/// (checked 2026-09-30).
pub const USER_AGENT: &str =
    concat!("Mozilla/5.0 (compatible; tender-db/", env!("CARGO_PKG_VERSION"), "; +https://tenders.zebreus.click)");

/// Issue 451: the refusal an edge WAF put in the origin's place, if this
/// response is one. A WAF header on a 200/206 is not a refusal.
fn waf_refusal(status: reqwest::StatusCode, headers: &reqwest::header::HeaderMap) -> Option<Error> {
    let action = headers.get("x-amzn-waf-action")?;
    (status != reqwest::StatusCode::OK && status != reqwest::StatusCode::PARTIAL_CONTENT).then(|| {
        Error::Challenged { status, action: action.to_str().unwrap_or("?").to_owned() }
    })
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
            Error::Challenged { status, action } => write!(
                f,
                "refused by the source's bot challenge ({status}, x-amzn-waf-action: {action}): \
                 the request's User-Agent was not accepted (issue 451)"
            ),
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

/// What [`fetch_fts`] reports after every span page, and after every record
/// or release package a dense span's walk reads (asked, or a record staged by
/// an earlier run).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FtsProgress<'a> {
    /// The window's UK civil day, `YYYY-MM-DD`.
    pub day: &'a str,
    /// Span pages read in this window.
    pub pages: usize,
    /// Releases on this window's leaf pages.
    pub releases: usize,
    /// The dense spans of the whole fetch so far (issue 477 unit 1b).
    pub dense: DenseTally,
}

/// What one fetch's dense-span walks did (issue 477 unit 1b): rare, and worth
/// seeing on the job row — of the backfill's fetch and of the daily probe.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DenseTally {
    /// Dense spans met, the one being walked included.
    pub spans: usize,
    /// The ocids their notices were completed over — each run's length,
    /// counted when its walk ends (`033562-2023`: 15).
    pub ocids: usize,
    /// Record and release-package requests ASKED for them: 404s and the
    /// ocids past a run's ends included, a record read back from staging not.
    pub requests: usize,
}

impl std::ops::Add for DenseTally {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            spans: self.spans + other.spans,
            ocids: self.ocids + other.ocids,
            requests: self.requests + other.requests,
        }
    }
}

impl DenseTally {
    /// The job row's suffix: nothing without a dense span, else
    /// ` · 1 dense span(s) completed: 15 ocid(s), 21 record request(s)`.
    pub fn row_suffix(&self) -> String {
        if self.spans == 0 {
            return String::new();
        }
        format!(
            " · {} dense span(s) completed: {} ocid(s), {} record request(s)",
            self.spans, self.ocids, self.requests
        )
    }

    /// The progress line's: nothing without a dense span, else
    /// `, 1 dense span(s), 9 record request(s)`.
    pub fn progress_suffix(&self) -> String {
        if self.spans == 0 {
            return String::new();
        }
        format!(", {} dense span(s), {} record request(s)", self.spans, self.requests)
    }
}

/// Fetch one FTS package (docs/research/uk-fts.md §2, plan D8, issue 477):
/// walk every request window of `target` as cursorless SPAN pages, staging each
/// page verbatim as `<span key>.json` under `<archive>/<rel_path minus
/// .zip>.pages/`, then assemble ONE zip from the walk's leaf pages — a member
/// per distinct release (`assemble_fts_zip`) — and land it under the same
/// immutability rules as [`fetch`].
///
/// **The walk never follows `links.next`** (issue 477; [`crate::fts`] has the
/// measurements: the cursor drops rows at page boundaries and its short page
/// reads as a last page). It replaces both the cursor walk and issue 449's
/// hour-by-hour fallback for a stuck cursor, which is the same defect. Each
/// window starts as ONE span — its first URL byte-identical to the one the
/// registry records — and runs as a stack:
/// - a page that is short ([`crate::fts::page_is_short`]) is a leaf: the span
///   is complete;
/// - a full page, or a short one that still names a next, is split
///   ([`crate::fts::split`]) and both halves are asked, the older first;
/// - a span still full at two seconds cannot be split, because the API
///   refuses a one-second window. It is a DENSE span (issue 477 unit 1b): its
///   page stays a leaf, and when the page holds ONE notice, the rows it could
///   not show are that notice's fan-out, completed from process records over
///   the notice's ocid run ([`walk_dense_span`]). Never the cursor: where ids
///   tie it sticks — `limit=10` served that span's same 10 rows on 29 pages
///   under one cursor. What the records cannot account for fails the
///   fetch as [`Error::Malformed`] with its staging intact. So no package
///   lands unless every leaf is complete — a short page, or a dense page with
///   its run — the per-window half of issue 477's completeness invariant.
///
/// Resumable: the staged span pages and records ARE the walk's state. A span
/// whose page is on disk is never asked again, and its page decides — exactly
/// as it did the first time — whether it is a leaf, is split or is dense; a
/// staged record is never asked again either. A page or record is staged only
/// once it has parsed, and atomically; a staged one that no longer parses is
/// storage damage, discarded and asked again. The staging dir is removed only
/// after the zip has landed; an `Err` (five throttled attempts, a malformed
/// page, a dense span the records cannot complete) leaves it intact, and the
/// `fetches` row is written only for a complete walk. What a walk cannot
/// resume from is discarded first (`discard_unresumable_staging`): the old
/// cursor walker's pages, and debris older than the registered landing.
///
/// A target whose last UK civil day has not ended is refused before any
/// request ([`Error::Unsupported`]): registered once and never re-walked, a
/// `monthly` of the running month would freeze it part-walked (its days are
/// the daily walk's), and so would a `daily` of the running day — or, of a
/// future day, register it empty (issue 477 review).
///
/// `refetch` is as [`fetch`]: false skips a registered period without HTTP
/// (removing what a crashed cleanup left of its staging).
/// `page_pause` separates ANY two FTS requests of the process — consecutive
/// calls and jobs included ([`pace_fts`]), records as well as pages —
/// `fts::PAGE_PAUSE_SECS` in production, zero in tests. `on_progress` fires
/// after every span page and every dense span's record with the running
/// totals ([`FtsProgress`]).
///
/// An empty window (a Sunday, December 2020) still lands a 0-member zip, so
/// the day is registered and the daily walk never re-asks for it.
pub async fn fetch_fts(
    db: &store::Db,
    client: &reqwest::Client,
    archive_root: &Path,
    target: &Target,
    refetch: bool,
    page_pause: std::time::Duration,
    stop: impl Fn() -> bool,
    mut on_progress: impl FnMut(&FtsProgress),
) -> Result<Outcome, Error> {
    let base = match crate::fts::base_of(&target.url) {
        Some(base) if target.source == "fts" => base,
        _ => return Err(Error::Unsupported("fetch_fts serves fts window targets only")),
    };
    let windows = crate::fts::windows(target);
    if windows.is_empty() {
        return Err(Error::Malformed(format!(
            "no request windows for {} {} {:?}",
            target.source, target.kind, target.period
        )));
    }
    if target.kind == "monthly"
        && let Some(month) = crate::fts::parse_month(&target.period)
        && !crate::fts::month_has_ended(month, store::now_unix())
    {
        return Err(Error::Unsupported(
            "an fts monthly whose last UK civil day has not ended: it would be registered \
             part-walked and never re-walked (issue 477); its days come from the daily walk",
        ));
    }
    if target.kind == "daily"
        && let Some(day) = crate::fts::parse_day(&target.period)
        && !crate::fts::day_has_ended(day, store::now_unix())
    {
        return Err(Error::Unsupported(
            "an fts daily whose UK civil day has not ended (the running day, or a future one): \
             it would be registered part-walked or empty and never re-walked (issue 477)",
        ));
    }
    let existing = db.latest_fetch(target.source, target.kind, &target.period).await?;
    let staging = staging_dir(archive_root, &target.rel_path);
    if existing.is_some() && !refetch {
        // A crash between the registry row and the staging cleanup leaves the
        // landing's pages behind; nothing else would ever remove them. Debris
        // only — an interrupted REFETCH's newer pages stay, and so does the dir
        // holding them. Best effort: the package is registered either way.
        if let Err(e) = discard_unresumable_staging(&staging, existing.as_ref()) {
            eprintln!("[fetch] {}: leftover staging not cleaned: {e}", staging.display());
        }
        let _ = std::fs::remove_dir(&staging);
        return Ok(Outcome::Unchanged);
    }

    discard_unresumable_staging(&staging, existing.as_ref())?;
    std::fs::create_dir_all(&staging)?;
    let mut leaves: Vec<PathBuf> = Vec::new();
    let mut dense = DenseTally::default();
    for window in &windows {
        let day = crate::fts::ymd(window.day);
        let (mut pages, mut releases) = (0usize, 0usize);
        let mut spans = vec![window.span];
        while let Some(span) = spans.pop() {
            let path = staging.join(format!("{}.json", crate::fts::span_key(span)));
            let url = crate::fts::span_url(base, span);
            let staged = match std::fs::read(&path) {
                // Staged by an earlier run: decided by its page, never asked again.
                Ok(bytes) => match page_summary(&bytes) {
                    Ok(summary) => Some(summary),
                    // A page is staged only once it has parsed, so one that no
                    // longer parses is storage damage (a power loss after the
                    // rename): kept, it would fail every retry at this file
                    // without a request. Discarded, the span is asked again.
                    Err(what) => {
                        eprintln!("[fetch] {}: staged page unreadable ({what}); asking its span again", path.display());
                        std::fs::remove_file(&path)?;
                        None
                    }
                },
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                Err(e) => return Err(e.into()),
            };
            let (count, next) = match staged {
                Some(summary) => summary,
                None => {
                    pace_fts(page_pause).await;
                    // The stop checkpoint (issue 450), read right before every
                    // request, after the pause, so a cancel that lands during the
                    // pause costs no request. Every span page asked so far is
                    // staged, so a re-run continues from exactly here.
                    if stop() {
                        return Ok(Outcome::Stopped);
                    }
                    let asked = get_bytes(client, &url).await;
                    mark_fts_request();
                    let bytes = asked?;
                    let summary = page_summary(&bytes).map_err(|what| Error::Malformed(format!("{url}: {what}")))?;
                    write_atomic(&path, &bytes)?;
                    summary
                }
            };
            pages += 1;
            if crate::fts::page_is_short(count, next.as_deref()) {
                releases += count;
                leaves.push(path);
            } else if let Some((older, newer)) = crate::fts::split(span) {
                // Never `next`: split, and ask both halves without a cursor.
                spans.push(newer);
                spans.push(older); // popped first, so the leaves stay in time order
            } else {
                // Dense: nothing narrower can be asked. Boxed, so the run's
                // walk sits on the heap only for the rare span that needs it.
                let progress = &mut |asked: usize| {
                    let walking = DenseTally { spans: dense.spans + 1, requests: dense.requests + asked, ..dense };
                    on_progress(&FtsProgress { day: &day, pages, releases, dense: walking })
                };
                let walk = Box::pin(walk_dense_span(client, base, &staging, span, &path, page_pause, &stop, progress));
                let Some(run) = walk.await? else { return Ok(Outcome::Stopped) };
                let ocids = run.first.run_len(&run.last) as usize;
                eprintln!(
                    "[fetch] {url}: dense span ({count} rows of {}): its page and {} record(s) of its {ocids} \
                     ocids {}..={} are its leaves, {} request(s) asked (issue 477)",
                    run.notice,
                    run.leaves.len(),
                    run.first,
                    run.last,
                    run.requests
                );
                dense = dense + DenseTally { spans: 1, ocids, requests: run.requests };
                releases += count;
                leaves.push(path);
                leaves.extend(run.leaves);
            }
            on_progress(&FtsProgress { day: &day, pages, releases, dense });
        }
    }

    let final_path = archive_root.join(&target.rel_path);
    if let Some(dir) = final_path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let (bytes, sha256) = assemble_fts_zip(&leaves, &temp_path(&final_path))?;
    let outcome = land(db, archive_root, target, existing.as_ref(), &final_path, bytes, sha256).await?;
    std::fs::remove_dir_all(&staging)?;
    Ok(outcome)
}

/// A dense span's notice and the records that complete it ([`walk_dense_span`]).
struct DenseRun {
    notice: String,
    first: crate::fts::Ocid,
    last: crate::fts::Ocid,
    /// The staged records that carry the notice: leaves beside the span's
    /// own page.
    leaves: Vec<PathBuf>,
    /// Record and release-package requests asked (a staged record read back
    /// is not one).
    requests: usize,
}

/// One ocid's record as a dense span's walk read it.
struct RecordRead {
    /// `None`: the server answered 404.
    says: Option<crate::fts::RecordSays>,
    /// Where it is (or would be) staged.
    path: PathBuf,
}

/// What an ocid's release package says ([`RecordAsker::package`]).
enum PackageSays {
    /// A 404.
    Absent,
    /// Its releases sorted as a record's are, and whether the page is the
    /// ocid's whole package.
    Holds(crate::fts::RecordSays, bool),
}

/// Whose an ocid OUTSIDE the page's run is, as far as the walk can tell
/// ([`RecordAsker::whose`]).
enum Whose {
    /// The notice's: its record carries it, staged here.
    Notice(PathBuf),
    /// Another notice's, never the span's.
    Other(Vec<crate::fts::Elsewhere>),
    /// Nobody's: a 404 (confirmed by its release package when asked to be).
    Absent,
}

/// What every record request of one dense span shares.
struct RecordAsker<'a> {
    client: &'a reqwest::Client,
    base: &'a str,
    staging: &'a Path,
    span: crate::fts::Span,
    notice: &'a str,
    ids: &'a std::collections::BTreeSet<String>,
    nesting: Option<usize>,
    page_pause: std::time::Duration,
    /// Requests asked so far, records and release packages.
    requests: usize,
}

impl RecordAsker<'_> {
    /// `ocid`'s record: read from staging, or asked — paced, behind the stop
    /// checkpoint, exactly like a span page — and staged once it has parsed.
    /// A 404 and a record holding no release stage nothing, so a re-run asks
    /// them again. `Ok(None)`: stopped before the request.
    async fn read(&mut self, ocid: &crate::fts::Ocid, stop: &impl Fn() -> bool) -> Result<Option<RecordRead>, Error> {
        use crate::fts::RecordSays;
        let path = self.staging.join(format!("{}.json", crate::fts::record_key(self.span, ocid)));
        let url = crate::fts::record_url(self.base, ocid);
        let (ids, nesting) = (self.ids, self.nesting);
        let says_of = |bytes: &[u8]| {
            crate::fts::record_releases(bytes, ocid, ids, nesting)
                .map_err(|what| Error::Malformed(format!("{url}: a dense span's record (issue 477): {what}")))
        };
        match std::fs::read(&path) {
            Ok(bytes) => match says_of(&bytes) {
                Ok(says @ (RecordSays::Carries(_) | RecordSays::Other(_))) => {
                    return Ok(Some(RecordRead { says: Some(says), path }));
                }
                // Only a record that names a notice is staged: anything else
                // on disk is storage damage (or a release-less record an
                // earlier build staged), discarded and asked again.
                Ok(RecordSays::Empty) | Err(_) => {
                    eprintln!("[fetch] {}: staged record unreadable; asking it again", path.display());
                    std::fs::remove_file(&path)?;
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        let Some(asked) = self.ask(&url, stop).await else { return Ok(None) };
        let bytes = match asked {
            Ok(bytes) => bytes,
            Err(Error::Status(reqwest::StatusCode::NOT_FOUND)) => return Ok(Some(RecordRead { says: None, path })),
            Err(e) => return Err(e),
        };
        let says = says_of(&bytes)?;
        if !matches!(says, RecordSays::Empty) {
            write_atomic(&path, &bytes)?;
        }
        Ok(Some(RecordRead { says: Some(says), path }))
    }

    /// `ocid`'s release package ([`crate::fts::release_package_url`]), asked
    /// like a record and never staged: it only decides what a record left
    /// open, and no member is built from it. `Ok(None)`: stopped.
    async fn package(&mut self, ocid: &crate::fts::Ocid, stop: &impl Fn() -> bool) -> Result<Option<PackageSays>, Error> {
        let url = crate::fts::release_package_url(self.base, ocid);
        let Some(asked) = self.ask(&url, stop).await else { return Ok(None) };
        let bytes = match asked {
            Ok(bytes) => bytes,
            Err(Error::Status(reqwest::StatusCode::NOT_FOUND)) => return Ok(Some(PackageSays::Absent)),
            Err(e) => return Err(e),
        };
        let (says, whole) = crate::fts::package_releases(&bytes, ocid, self.ids, self.nesting)
            .map_err(|what| Error::Malformed(format!("{url}: a dense span's release package (issue 477): {what}")))?;
        Ok(Some(PackageSays::Holds(says, whole)))
    }

    /// One paced request behind the stop checkpoint (issue 450), counted.
    async fn ask(&mut self, url: &str, stop: &impl Fn() -> bool) -> Option<Result<Vec<u8>, Error>> {
        pace_fts(self.page_pause).await;
        if stop() {
            return None;
        }
        let asked = get_bytes(self.client, url).await;
        mark_fts_request();
        self.requests += 1;
        Some(asked)
    }

    /// Whose `ocid`, outside the page's run, is. Its record decides when it
    /// names a notice. A record that holds no release says nothing, so its
    /// release package decides; and with `confirm`, so it does for a 404 —
    /// the record endpoint has served nothing for a real ocid (`04196f`), so
    /// a 404 ends a side only when the release package is a 404 too. A
    /// release package that carries the notice fails loud: the notice has an
    /// ocid whose record cannot supply its release.
    async fn whose(
        &mut self,
        ocid: &crate::fts::Ocid,
        stop: &impl Fn() -> bool,
        confirm: bool,
    ) -> Result<Option<Whose>, Error> {
        use crate::fts::RecordSays;
        let Some(read) = self.read(ocid, stop).await? else { return Ok(None) };
        let said = match read.says {
            Some(RecordSays::Carries(_)) => return Ok(Some(Whose::Notice(read.path))),
            Some(RecordSays::Other(elsewhere)) => return Ok(Some(Whose::Other(elsewhere))),
            None if !confirm => return Ok(Some(Whose::Absent)),
            None => "is a 404",
            Some(RecordSays::Empty) => "holds no release",
        };
        let url = crate::fts::release_package_url(self.base, ocid);
        let malformed = |what: String| {
            Error::Malformed(format!("{url}: a dense span (issue 477): the record of {ocid} {said}, but {what}"))
        };
        Ok(Some(match self.package(ocid, stop).await? {
            None => return Ok(None),
            Some(PackageSays::Absent | PackageSays::Holds(RecordSays::Empty, true)) => Whose::Absent,
            Some(PackageSays::Holds(RecordSays::Other(elsewhere), true)) => Whose::Other(elsewhere),
            Some(PackageSays::Holds(RecordSays::Carries(_), _)) => {
                return Err(malformed(format!(
                    "its release package carries {}: one of the notice's ocids has no record to complete it from",
                    self.notice
                )));
            }
            Some(PackageSays::Holds(_, false)) => {
                return Err(malformed("its release package is not whole (it names a next page)".into()));
            }
        }))
    }
}

/// Complete a DENSE span — still full at two seconds, so nothing narrower can
/// be asked — from process records (issue 477 unit 1b). `page` is the span's
/// staged cursorless page, a leaf already. `Ok(None)` when a stop landed
/// between two requests. `on_request` hears the running count of requests.
///
/// **Why records, and never the cursor.** 2023-11-14 10:05:14–15 lists 210
/// rows of ONE notice, `033562-2023`: its 15 ocids' release, 14 times each.
/// A page shows 100. The cursor is the defect itself: where ids tie it does
/// not advance — a limit-10 walk of the span served the same 10 rows on 29
/// pages under one `nextCursor` (issue 449's stuck shape), and the limit-100
/// walk reached all 15 ocids only by luck. A process's record holds all its
/// releases, so the notice is complete once every ocid of its run is read:
/// 1. **One notice.** The page's releases must all carry the same notice id.
///    With several, the rows below its 100 could hold another notice whole,
///    and no record names it: that fails as [`Error::Malformed`], as an
///    unsplittable span did before this walk.
/// 2. **The run.** Its seeds are the page's distinct ocids, one series
///    ([`crate::fts::Ocid`]). Every ocid from the first seed to the last is
///    asked, and must carry the notice. A seed's record that holds no
///    release (`04196f`'s is a 200 with no body) passes only when the seed's
///    release package holds the page's release of it byte for byte and no
///    other release of the notice; an ocid off the page fails on it.
/// 3. **The ends.** The run then extends one ocid at a time below and above
///    ([`RecordAsker::whose`]): an ocid whose record carries the notice joins
///    it. One of another notice ends that side (`04197e` is `033564-2023`) —
///    unless that notice is dated inside the span, which only a notice the
///    page's rows hide could be: that fails. A 404 ends it only once the
///    ocid's release package is a 404 too (`04196e`), and only when the
///    [`crate::fts::DENSE_LOOKAHEAD`] ocids past it do not carry the notice:
///    the series has holes (`04196c`..`04196e`), and a hole inside a run
///    fails rather than dropping every ocid past it. A record holding no
///    release is decided by its release package the same way. Past
///    [`crate::fts::DENSE_RUN_CAP`] ocids the run fails.
/// 4. **The leaves.** Each record carrying the notice is staged as
///    `<span key>-r<ocid>.json` ([`crate::fts::record_key`]) and is a leaf:
///    the assembler takes its releases of the notice, re-nested to the
///    page's depth and verified ([`crate::fts::record_releases`]), so a
///    member from a record is byte-identical to one from a listing. A seed's
///    record must hold the page's own release of that ocid, byte for byte
///    after re-nesting: the equality is checked live on every dense span,
///    not trusted from the two ocids it was measured on.
///
/// The seeds' records go beyond the owner's decision, which asks only for the
/// ocids the page does not show: without them neither the empty-body rule nor
/// the live byte check could happen, and a second release of the notice under
/// a seed ocid, hidden below the page, would be lost. The cost is one request
/// per seed (8 on 2023-11-14), plus a release package per seed whose record is
/// empty, per 404 that ends a side, and the look-ahead past it: 21 requests on
/// 2023-11-14 where the decision's walk asked 10.
///
/// **What the walk cannot see** (issue 477's "Still open"):
/// - a DIFFERENT notice, with a lower id, published in the same two seconds,
///   wholly below the page's 100 rows and on no ocid next to the run (dated
///   in the span, next to it, it fails). The per-year id invariant (issue 477
///   unit 3) sees it, as an id that is missing. A window offset by one second
///   (`[from − 1, from]`, `[to, to + 1]`) is askable and would narrow this to
///   the fan-out's own second; the walk does not ask it yet.
/// - a second release of the SAME notice on an ocid that is not next to the
///   run (a far process, as `038018-2025`'s two releases are), in the page's
///   hidden rows. Unit 3 does NOT see it — the id is held — and no request
///   lists a notice's ocids: `GET /ocdsReleasePackages/{notice id}` serves the
///   same capped, stuck page as the span.
#[allow(clippy::too_many_arguments)]
async fn walk_dense_span(
    client: &reqwest::Client,
    base: &str,
    staging: &Path,
    span: crate::fts::Span,
    page: &Path,
    page_pause: std::time::Duration,
    stop: impl Fn() -> bool,
    mut on_request: impl FnMut(usize),
) -> Result<Option<DenseRun>, Error> {
    use crate::fts::{Ocid, RecordSays};
    let url = crate::fts::span_url(base, span);
    let malformed = |what: String| Error::Malformed(format!("{url}: a dense span (issue 477): {what}"));
    let bytes = std::fs::read(page)?;
    let listing = crate::fts::Page::read(&bytes).map_err(malformed)?;
    let releases = listing.releases().map_err(malformed)?;

    // 1. One notice.
    let ids: std::collections::BTreeSet<String> = releases.iter().filter_map(|r| crate::fts::release_id(r)).collect();
    let notice = match ids.first() {
        Some(id) if ids.len() == 1 && releases.iter().all(|r| crate::fts::release_id(r).is_some()) => id.clone(),
        _ => {
            return Err(malformed(format!(
                "{} releases of {} notice ids ({}{}), and the API refuses a one-second window: a \
                 notice below the page's rows is named by no record, so the span cannot be \
                 completed",
                releases.len(),
                ids.len(),
                ids.iter().take(5).cloned().collect::<Vec<_>>().join(", "),
                if ids.len() > 5 { ", …" } else { "" },
            )));
        }
    };
    let nesting = crate::fts::listing_nesting(&releases).map_err(malformed)?;

    // 2. The run's seeds: the page's ocids, each with the page's releases of it.
    let mut seeds: std::collections::BTreeMap<Ocid, Vec<&str>> = std::collections::BTreeMap::new();
    for release in &releases {
        let ocid = crate::fts::release_ocid(release)
            .and_then(|o| Ocid::parse(&o))
            .ok_or_else(|| malformed(format!("a release of {notice} whose ocid is not of a hex series")))?;
        let held = seeds.entry(ocid).or_default();
        if !held.contains(&release.get()) {
            held.push(release.get());
        }
    }
    let (Some(first), Some(last)) = (seeds.keys().next().cloned(), seeds.keys().next_back().cloned()) else {
        return Err(malformed("a full page without releases".into()));
    };
    if let Some(odd) = seeds.keys().find(|o| !o.same_series(&first)) {
        return Err(malformed(format!("ocids of two series on one page: {first} and {odd}")));
    }
    if first.run_len(&last) > crate::fts::DENSE_RUN_CAP as u64 {
        return Err(malformed(format!(
            "its ocids {first}..={last} are {} apart, past the cap of {}",
            first.run_len(&last),
            crate::fts::DENSE_RUN_CAP
        )));
    }
    // A staged record that contradicts the page is never asked again: name it,
    // so a person can remove it and re-enqueue (docs/operations.md).
    let staged = |path: &Path| format!("staged as {}; remove it to have it asked again", path.display());
    // Another notice dated inside the span can only be one the page hides.
    let in_span = |at: &Ocid, elsewhere: &[crate::fts::Elsewhere]| -> Result<(), Error> {
        let inside = |date: &str| crate::fts::uk_wall_of(date).is_some_and(|w| span.from <= w && w <= span.to);
        match elsewhere.iter().find(|e| e.date.as_deref().is_some_and(inside)) {
            Some(hidden) => Err(malformed(format!(
                "{at} holds another notice, {}, dated {} — inside the span: a notice the page's rows hide, \
                 which no record of {notice}'s run accounts for",
                hidden.id,
                hidden.date.as_deref().unwrap_or_default()
            ))),
            None => Ok(()),
        }
    };

    let mut asker = RecordAsker {
        client,
        base,
        staging,
        span,
        notice: &notice,
        ids: &ids,
        nesting,
        page_pause,
        requests: 0,
    };
    let mut leaves: Vec<PathBuf> = Vec::new();
    // Every ocid from the first seed to the last carries the notice.
    let mut next = Some(first.clone());
    while let Some(at) = next.filter(|o| *o <= last) {
        let Some(read) = asker.read(&at, &stop).await? else { return Ok(None) };
        on_request(asker.requests);
        let on_page = seeds.get(&at);
        match (read.says, on_page) {
            (Some(RecordSays::Carries(carried)), Some(held)) => {
                // The live byte check: the record re-nested IS the listing.
                if let Some(missing) = held.iter().find(|raw| !carried.iter().any(|c| c.get() == **raw)) {
                    return Err(malformed(format!(
                        "the record of {at} does not hold the page's release of it byte for byte after \
                         re-nesting ({} bytes on the page), so a member built from a record would not \
                         match one built from the listing ({})",
                        missing.len(),
                        staged(&read.path)
                    )));
                }
                leaves.push(read.path);
            }
            (Some(RecordSays::Carries(_)), None) => leaves.push(read.path),
            (Some(RecordSays::Empty), Some(held)) => {
                // The page holds this seed's release, but the record cannot
                // say whether the notice has another under it: the seed's
                // release package does (`04196f`'s: 23 releases, 14 copies of
                // the page's one, 9 later notices).
                let Some(package) = asker.package(&at, &stop).await? else { return Ok(None) };
                on_request(asker.requests);
                let carried = match package {
                    PackageSays::Holds(RecordSays::Carries(carried), true) => carried,
                    PackageSays::Holds(_, false) => {
                        return Err(malformed(format!(
                            "the record of {at}, an ocid the page serves, holds no release, and its release \
                             package is not whole (it names a next page), so it cannot vouch for the page"
                        )));
                    }
                    _ => {
                        return Err(malformed(format!(
                            "the record of {at}, an ocid the page serves, holds no release, and its release \
                             package does not carry {notice}"
                        )));
                    }
                };
                if let Some(missing) = held.iter().find(|raw| !carried.iter().any(|c| c.get() == **raw)) {
                    return Err(malformed(format!(
                        "the release package of {at} does not hold the page's release of it byte for byte \
                         ({} bytes on the page)",
                        missing.len()
                    )));
                }
                if carried.iter().any(|c| !held.contains(&c.get())) {
                    return Err(malformed(format!(
                        "the release package of {at} holds a release of {notice} the page does not show, \
                         and its record (which holds no release) cannot supply it"
                    )));
                }
            }
            (Some(RecordSays::Empty), None) => {
                return Err(malformed(format!(
                    "the record of {at}, inside the run and off the page, holds no release"
                )));
            }
            (Some(RecordSays::Other(_)), _) => {
                let whose = if on_page.is_some() { "an ocid the page serves" } else { "an ocid inside the run" };
                return Err(malformed(format!(
                    "the record of {at}, {whose}, does not carry {notice} ({})",
                    staged(&read.path)
                )));
            }
            (None, _) => {
                let whose = if on_page.is_some() { "an ocid the page serves" } else { "an ocid inside the run" };
                return Err(malformed(format!("the record of {at}, {whose}, is a 404")));
            }
        }
        next = at.step(1);
    }
    // 3. Then outward, one ocid at a time, until each side ends.
    let (mut run_first, mut run_last) = (first.clone(), last.clone());
    for dir in [-1i64, 1] {
        let mut next = if dir < 0 { first.step(-1) } else { last.step(1) };
        while let Some(at) = next {
            let Some(whose) = asker.whose(&at, &stop, true).await? else { return Ok(None) };
            on_request(asker.requests);
            match whose {
                Whose::Notice(path) => leaves.push(path),
                Whose::Other(elsewhere) => {
                    in_span(&at, &elsewhere)?;
                    break;
                }
                Whose::Absent => {
                    // A hole is no end: the ocids just past it must not carry
                    // the notice either.
                    for k in 1..=crate::fts::DENSE_LOOKAHEAD {
                        let Some(beyond) = at.step(dir * k) else { break };
                        let Some(whose) = asker.whose(&beyond, &stop, false).await? else { return Ok(None) };
                        on_request(asker.requests);
                        match whose {
                            Whose::Notice(path) => {
                                return Err(malformed(format!(
                                    "{at} is absent (a 404), but {beyond}, {k} past it, carries {notice}: a \
                                     hole in its run, which the walk does not bridge ({})",
                                    staged(&path)
                                )));
                            }
                            Whose::Other(elsewhere) => {
                                in_span(&beyond, &elsewhere)?;
                                break;
                            }
                            Whose::Absent => {}
                        }
                    }
                    break;
                }
            }
            if dir < 0 { run_first = at.clone() } else { run_last = at.clone() }
            if run_first.run_len(&run_last) > crate::fts::DENSE_RUN_CAP as u64 {
                return Err(malformed(format!(
                    "{notice}'s run {run_first}..={run_last} grew past the cap of {} ocids",
                    crate::fts::DENSE_RUN_CAP
                )));
            }
            next = at.step(dir);
        }
    }
    leaves.sort();
    let requests = asker.requests;
    Ok(Some(DenseRun { notice, first: run_first, last: run_last, leaves, requests }))
}

/// Walk FTS daily windows forward over every day from [`fts_probe_floor`]
/// through `end` (yesterday in UK civil time) that has NO daily row, fetching
/// each in order — the DÖE walk-forward shape ([`probe_doe_daily`]), as a gap
/// walk: a normal run fetches one day, and missed ticks are caught up.
///
/// A gap walk, not a high-water mark (issue 477 and its review). The seam
/// opened because the walk started after the NEWEST covered day: the daily
/// probe shipped on an empty registry and fetched only 2026-09-07, the monthly
/// backfill landed through 2026-08 weeks later, and 2026-09-01..06 (1,745
/// notices) sat below a watermark already past them. Every day after the
/// newest monthly that holds no daily is now walked whichever landed first,
/// and a daily row past `end` (a future day landed by hand) moves nothing.
///
/// It never refetches: a window selects on a hidden publication instant (not
/// the release `date`, issue 477), and a UK day that is over is final; a tick
/// that ran late is covered by the next day's 2 h overlap. Requests are paced
/// by `page_pause` across days as within one ([`pace_fts`]).
///
/// `stop` is read before every request of every day (issue 450, via
/// [`fetch_fts`]): a stopped day ends the walk as its last entry,
/// [`Outcome::Stopped`], with its staged pages kept; the days before it have
/// landed, and the next run resumes the stopped day where it stopped.
///
/// `on_progress` hears every day's [`FtsProgress`] as [`fetch_fts`] reports
/// it — its `day` is the period walked — so a dense span the probe meets
/// (issue 477 unit 1b, up to [`crate::fts::DENSE_RUN_CAP`] paced record
/// requests) is visible while it runs and countable on the job row. Its
/// [`DenseTally`] counts that day's fetch only.
#[allow(clippy::too_many_arguments)]
pub async fn probe_fts_daily(
    db: &store::Db,
    client: &reqwest::Client,
    archive_root: &Path,
    base: &str,
    end: (u16, u8, u8),
    page_pause: std::time::Duration,
    stop: impl Fn() -> bool,
    mut on_progress: impl FnMut(&FtsProgress),
) -> Result<Vec<(String, Outcome)>, Error> {
    let floor = fts_probe_floor(db, end).await?;
    let held: std::collections::HashSet<String> = if floor <= end {
        db.fetch_periods_between("fts", "daily", &crate::fts::ymd(floor), &crate::fts::ymd(end))
            .await?
            .into_iter()
            .collect()
    } else {
        Default::default()
    };
    let mut out = Vec::new();
    let mut day = floor;
    while day <= end {
        if held.contains(&crate::fts::ymd(day)) {
            day = next_civil_day(day);
            continue;
        }
        // CAPPED PER TICK (issue 342 review, lens "ops"). An FTS day is about
        // 17 paced requests on a 2026 weekday under the split walk and 35 at
        // most ([`crate::fts::PROBE_DAY_CAP`]), plus up to eight minutes of
        // back-off if the limiter is unhappy — unlike a DÖE day, which is one
        // download. Uncapped, a gap left far in the past (a month of failed
        // ticks, a restored registry) would hold the single job runner for
        // hours and park `fetch-rates`, `project` and the fold behind it. A
        // landed day is no longer a gap, so the remainder is simply the next
        // tick's work; a real gap closes in days, and `fetch fts --day` or a
        // monthly backfill closes it at once.
        if out.len() >= crate::fts::PROBE_DAY_CAP {
            break;
        }
        let target = crate::fts::day(base, day);
        let outcome = fetch_fts(db, client, archive_root, &target, false, page_pause, &stop, &mut on_progress).await?;
        let stopped = outcome == Outcome::Stopped;
        out.push((target.period.clone(), outcome));
        if stopped {
            break;
        }
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

/// The first day [`probe_fts_daily`] may have to fetch for a walk ending at
/// `end`; it fetches every day from here through `end` that holds no daily.
///
/// - **A monthly on record:** the day after the newest monthly's last day.
///   The monthlies are the backfill's (through the previous UK month); every
///   day after them is the probe's, whichever of the two landed first.
/// - **No monthly:** the earlier of the day after the newest daily (a gap that
///   reaches back past `end`'s month is still caught up) and day 1 of `end`'s
///   month (the days of the running month are never left to a backfill that
///   stops at the previous one). With nothing on record, day 1 of `end`'s
///   month: never a full-archive backfill through the probe — that is the
///   monthly path's job.
pub async fn fts_probe_floor(db: &store::Db, end: (u16, u8, u8)) -> turso::Result<(u16, u8, u8)> {
    let monthly = db.latest_fetch_period_max("fts", "monthly", "").await?;
    if let Some((y, m)) = monthly.as_deref().and_then(crate::fts::parse_month) {
        return Ok(next_civil_day((y, m, crate::fts::days_in_month(y, m))));
    }
    let month_start = (end.0, end.1, 1);
    Ok(match latest_fts_day(db).await? {
        Some(newest) => next_civil_day(newest).min(month_start),
        None => month_start,
    })
}

/// When this process last finished an FTS request (issue 477 review, lens
/// "operability"). The pause between requests used to be kept only inside one
/// [`fetch_fts`] call, so the next job's — or the next probe day's — first
/// request followed the last one with no pause at all, and the limiter has
/// answered 429 with `Retry-After: 120` even at an 11 s cadence. ~790
/// back-to-back top-up jobs would have been ~790 such risks.
static LAST_FTS_REQUEST: std::sync::Mutex<Option<std::time::Instant>> = std::sync::Mutex::new(None);

/// Wait until `pause` has passed since the process's last FTS request.
async fn pace_fts(pause: std::time::Duration) {
    let last = *LAST_FTS_REQUEST.lock().unwrap_or_else(|e| e.into_inner());
    let wait = last.map_or(std::time::Duration::ZERO, |at| pause.saturating_sub(at.elapsed()));
    if !wait.is_zero() {
        tokio::time::sleep(wait).await;
    }
}

/// Record that an FTS request (and any retries [`get_bytes`] made) just ended.
fn mark_fts_request() {
    *LAST_FTS_REQUEST.lock().unwrap_or_else(|e| e.into_inner()) = Some(std::time::Instant::now());
}

/// Where a paged package is staged while its windows are walked:
/// `<archive>/<rel_path minus .zip>.pages/`, beside the package it becomes.
/// A directory, so [`register_archive`] steps over it as a non-file.
fn staging_dir(archive_root: &Path, rel_path: &str) -> PathBuf {
    let stem = rel_path.strip_suffix(".zip").unwrap_or(rel_path);
    archive_root.join(format!("{stem}.pages"))
}

/// Remove what a walk must not resume from, before it starts:
/// - **anything that is not a span page or a dense span's record**
///   (`<span key>.json`, `<span key>-r<ocid>.json`, see [`crate::fts::span_key`]
///   and [`crate::fts::record_key`]) — above all the cursor walker's
///   `<day>-pNNN.json`, `<day>-hNN-pNNN.json` and `cursor.json` (issue 477:
///   those pages are exactly what lost the releases, so they are discarded,
///   never assembled or resumed), and an interrupted write's `.part`;
/// - **debris of a landing** (issue 342 review, lens "fetcher"): a page staged
///   no later than the registered landing is what an interrupted cleanup (or a
///   crash between the row and the cleanup) left behind, and resuming from it
///   would make a refetch reuse old pages instead of re-walking. A page NEWER
///   than the row is an interrupted refetch, and is resumed.
fn discard_unresumable_staging(staging: &Path, landed: Option<&store::Fetch>) -> Result<(), Error> {
    let Ok(entries) = std::fs::read_dir(staging) else { return Ok(()) };
    let mut discarded = 0usize;
    for entry in entries {
        let path = entry?.path();
        let key = path.file_name().and_then(|n| n.to_str()).and_then(|n| n.strip_suffix(".json"));
        let span_page = path.is_file()
            && key.is_some_and(|k| crate::fts::parse_span_key(k).is_some() || crate::fts::parse_record_key(k).is_some());
        let staged_at = std::fs::metadata(&path)
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|age| age.as_secs() as i64);
        let debris = landed.is_some_and(|row| staged_at.is_none_or(|at| at <= row.fetched_at));
        if span_page && !debris {
            continue;
        }
        if path.is_dir() { std::fs::remove_dir_all(&path)? } else { std::fs::remove_file(&path)? }
        discarded += 1;
    }
    if discarded > 0 {
        eprintln!("[fetch] {}: discarded {discarded} staged file(s) the walk cannot resume from", staging.display());
    }
    Ok(())
}

/// Write via `<path>.part` + rename, so a page that was interrupted mid-write
/// never reads as a staged one.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    let tmp = temp_path(path);
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// What a page says about the walk: how many releases it holds and whether it
/// names a next page. Anything without a `releases` array is not a release
/// package.
fn page_summary(bytes: &[u8]) -> Result<(usize, Option<String>), String> {
    let page = crate::fts::Page::read(bytes)?;
    Ok((page.releases()?.len(), page.next()))
}

/// Assemble the walk's leaf pages, in walk order, into `part`: one member per
/// DISTINCT release, built by [`crate::fts::Page::member_bytes`] and named by
/// its notice id. One release of an id is `<id>.json`; when an id carries
/// several releases with different bytes, the one whose member sha256 (the
/// notice row's `content_hash`) is lowest keeps `<id>.json` and each other is
/// `<id>~<hash8>.json`, the first 8 hex digits of its own. By hash, not by
/// serve order: the server's order between two releases of one id is not
/// known to be stable, and a refetch of the same releases must hash equal
/// (issue 477 review). A byte-identical repeat collapses into one member.
///
/// Issue 477: one id can carry two releases — `038018-2025` is a
/// `tenderUpdate` on the old procurement and an `award,contract` on the new
/// one, under two ocids. Keyed by the id alone, the second was dropped without
/// trace whenever both fell in one package. The processor reads a member's
/// publication id from its payload, not its name, so the `~` member is the
/// same publication's second notice row.
///
/// A dense span's record (`<span key>-r<ocid>.json`, issue 477 unit 1b) is a
/// leaf of its span: read against the span's page, it gives the releases of
/// the page's notice id, re-nested to the page's depth and verified
/// ([`crate::fts::record_releases`]), each built under the PAGE's header — so
/// the member is the bytes a listing-served release of it makes. `033562-2023`
/// is the case this exists for: its 15 releases (one per ocid) are 15 members,
/// `<id>.json` and 14 `~` variants, and its 14-fold repeats collapse.
///
/// Members are sorted by name (`<id>.json` before its `~` variants); the zip
/// is byte-deterministic, so a refetch of unchanged pages hashes equal. A
/// walk with no releases yields a valid 0-member zip. Returns the written
/// zip's (bytes, sha256-hex).
fn assemble_fts_zip(leaves: &[PathBuf], part: &Path) -> Result<(i64, String), Error> {
    // id → its distinct member bytes.
    let mut by_id: std::collections::BTreeMap<String, Vec<Vec<u8>>> = std::collections::BTreeMap::new();
    for path in leaves {
        let malformed = |what: String| Error::Malformed(format!("{}: {what}", path.display()));
        let record = path.file_stem().and_then(|n| n.to_str()).and_then(crate::fts::parse_record_key);
        let page_path = match &record {
            Some((span, _)) => path.with_file_name(format!("{}.json", crate::fts::span_key(*span))),
            None => path.clone(),
        };
        let bytes = std::fs::read(&page_path)?;
        let page = crate::fts::Page::read(&bytes).map_err(malformed)?;
        let on_page = page.releases().map_err(malformed)?;
        let renested;
        let releases: Vec<&serde_json::value::RawValue> = match &record {
            None => on_page,
            Some((_, ocid)) => {
                let ids = on_page.iter().filter_map(|r| crate::fts::release_id(r)).collect();
                let nesting = crate::fts::listing_nesting(&on_page).map_err(malformed)?;
                match crate::fts::record_releases(&std::fs::read(path)?, ocid, &ids, nesting).map_err(malformed)? {
                    crate::fts::RecordSays::Carries(moved) => {
                        renested = moved;
                        renested.iter().map(|r| &**r).collect()
                    }
                    _ => return Err(malformed("a leaf record that does not carry its span's notice".into())),
                }
            }
        };
        for (index, release) in releases.iter().enumerate() {
            let id = match crate::fts::release_id(release) {
                // The id becomes a member name: a separator in it would name a
                // directory, and a `~` could pass for another id's variant.
                Some(id) if !id.is_empty() && !id.contains(['/', '\\', '~']) => id,
                // A release the publisher sent without a usable id is ARCHIVED,
                // not thrown (issue 342 review, lens "fetcher"). Failing the
                // package here would be deterministic: the day would fail every
                // tick, the walk-forward would never pass it, and one malformed
                // release would stop the whole walk-forward. Under a reserved
                // `_noid/` prefix it reaches the profile layer, which quarantines
                // it as a missing publication id with the bytes intact — the
                // publisher's defect, recorded where defects are recorded.
                _ => {
                    let stem = path.file_stem().and_then(|n| n.to_str()).unwrap_or("page");
                    format!("_noid/{stem}-{index:03}")
                }
            };
            let member = page.member_bytes(release);
            let held = by_id.entry(id).or_default();
            if !held.contains(&member) {
                held.push(member);
            }
        }
    }
    let mut members: std::collections::BTreeMap<String, Vec<u8>> = std::collections::BTreeMap::new();
    for (id, releases) in by_id {
        // Lowest hash first (bytes break a full-hash tie), so the names depend
        // only on WHICH releases were served, never on their order.
        let mut releases: Vec<(String, Vec<u8>)> =
            releases.into_iter().map(|bytes| (crate::sha256_hex(&bytes), bytes)).collect();
        releases.sort();
        for (n, (hash, bytes)) in releases.into_iter().enumerate() {
            let name = if n == 0 {
                format!("{id}.json")
            } else {
                let short = format!("{id}~{}.json", &hash[..8]);
                // Two different releases of one id agreeing on 32 bits of
                // hash: the full hash tells them apart.
                if members.contains_key(&short) { format!("{id}~{hash}.json") } else { short }
            };
            members.insert(name, bytes);
        }
    }

    let mut zip = zip::ZipWriter::new(std::fs::File::create(part)?);
    // A fixed entry timestamp: the zip's bytes are its registry identity, and a
    // window re-walked with `refetch` must hash equal when nothing changed.
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .last_modified_time(zip::DateTime::default());
    for (name, bytes) in &members {
        zip.start_file(name.as_str(), opts).map_err(zip_error)?;
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
    if let Some(refused) = waf_refusal(resp.status(), resp.headers()) {
        return Err(refused);
    }
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
    if let Some(refused) = waf_refusal(resp.status(), resp.headers()) {
        return Err(refused);
    }

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

/// Every publication source the fetch registry archives. The served data's reuse
/// terms are owed per source, and `tender_db::v1::DATA_SOURCES` carries one
/// attribution for each of these; its test fails when a source is added here
/// without one (issue 446).
pub const SOURCES: [&str; 3] = ["ted", "doe", "fts"];

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
    for source in SOURCES {
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
