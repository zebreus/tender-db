//! UK Find a Tender Service (FTS) addressing and packaging
//! (docs/research/uk-fts.md, issue 342 unit 2; the walk is issue 477's).
//!
//! FTS serves OCDS 1.1 release packages from one unauthenticated endpoint,
//! `GET {BASE}/ocdsReleasePackages?updatedFrom=&updatedTo=`, at most 100
//! releases a page. There is no bulk package to download, so the fetcher
//! ([`crate::fetch::fetch_fts`]) walks each window and ASSEMBLES the archive
//! package itself: one zip per (kind, period), one member per release, each
//! member a single-release OCDS package built by [`Page::member_bytes`].
//!
//! **The walk never follows `links.next`** (issue 477). The API sorts a window
//! newest notice id first, but its paging cursor continues on a hidden
//! per-release key that is not in id order. So at every page boundary rows can
//! be silently dropped (and others repeated), and the page that dropped them
//! comes back short with no next, exactly like a real last page. That lost
//! 14,093 notices (4.3 % of FTS) on days that needed a second page, and issue
//! 449's stuck cursor (page 2 is page 1 again, its next its own URL) is the same
//! defect. A page asked for WITHOUT a cursor was correct in every measurement.
//! So a window is a [`Span`] of wall-clock seconds, and only cursorless pages
//! are ever asked:
//! - a page that is short ([`page_is_short`]: fewer than [`PAGE_LIMIT`] rows
//!   and no next) is the whole span;
//! - a full page means the span holds more than a page, so it is [`split`] in
//!   two and each half is asked again.
//!
//! The API answers 400 to a window whose ends are equal, so no split ever
//! makes a one-second span; a span still full at [`MIN_SPAN_SECS`] fails the
//! fetch loudly (the fetcher's job, not a silent truncation).
//!
//! Two kinds, mirroring TED/DÖE so `Process{daily, None}` re-walks only live
//! days (plan D2):
//! - `daily` — the live poll: ONE window over a UK civil day with a 2 h overlap
//!   on `updatedFrom` ([`OVERLAP_SECS`]), so a release updated around midnight
//!   is never lost between two ticks. The overlap re-yields releases already
//!   archived the day before; they dedup on identity (D3), because
//!   [`Page::member_bytes`] is byte-deterministic.
//! - `monthly` — the backfill package: one contiguous 1-day window per civil
//!   day of the month, no overlap (§2 of the research: wide windows were
//!   rate-limited before answering).
//!
//! `updatedFrom`/`updatedTo` are interpreted by the server in UK local time
//! (GMT/BST), select on a hidden publication instant (not the release `date`),
//! include both end seconds, and are sent as bare wall-clock strings; nothing
//! here converts a time zone. [`uk_offset`] exists so the supervisor can name
//! "yesterday" in UK civil time, and [`split`] only steers its cuts off the
//! two seconds where the repeated autumn hour could open a gap.
//!
//! **Member bytes are a re-serialisation, not bytes-as-served** — the first
//! deviation from docs/architecture.md's "archive the bytes the source sent".
//! A page's composition shifts under the overlap and the splits, so the page is
//! the wrong unit of identity; the release is the unit, and the only way to
//! store a release as its own self-describing, OGL-attributed package is to
//! re-serialise it under the page's header. The staged raw pages are deleted
//! once the zip lands (plan risk 5).
//!
//! Contains public sector information licensed under the Open Government
//! Licence v3.0.

use crate::fetch::{civil_date, days_from_civil, Target};
use serde_json::value::RawValue;
use std::collections::HashMap;

pub mod checklist;
pub mod parse;
pub use parse::parse_payload;

pub const BASE: &str = "https://www.find-tender.service.gov.uk/api/1.0";

/// Earliest month with data: the API holds nothing before 2021-01-02 (the
/// December 2020 window answers `releases: []`, recorded 2026-09-07).
pub const FIRST_MONTH: (u16, u8) = (2021, 1);

/// How far a daily window reaches back into the previous day: 2 h, so a
/// release whose `date` straddles midnight (or a tick that ran late) is caught
/// by the next day's window too. The overlap dedups on identity downstream.
pub const OVERLAP_SECS: i64 = 7_200;

/// Pause between two page requests. The limiter is variable (§1: 6 s and 11 s
/// cadences both drew 429s, a 15 s cadence drew none), so 12 s is the measured
/// safe side of it, not a documented figure.
pub const PAGE_PAUSE_SECS: u64 = 12;

/// Releases per page — the API's maximum (`limit` 1–100, default 100).
pub const PAGE_LIMIT: u32 = 100;

/// How many days one walk-forward tick may fetch ([`crate::fetch::probe_fts_daily`]).
///
/// Two weeks, so a normal gap (a few failed ticks, a weekend of downtime)
/// closes in one run while a gap left far behind cannot hold the job
/// runner for hours. Under the split walk (issue 477) a 2026 day costs about
/// 12 paced requests on average — 17 on a weekday, 23 at the 95th percentile,
/// 35 at most (simulated on the archive's 2026 days with this module's
/// [`split`]) — so 2.5–3.5 minutes at the 12 s pace plus any back-off, where a DÖE
/// day costs one download: 14 days is under an hour typically and under two
/// at worst. The remainder is the next tick's work — a landed day is no longer
/// a gap — and a real backfill is the monthly path, not this.
pub const PROBE_DAY_CAP: usize = 14;

/// The header fields carried into every member package. Everything else on a
/// page header (`uri`, `links`, `publishedDate`) describes THAT PAGE, not the
/// release, and would make the same release hash differently on every fetch.
pub const MEMBER_HEADER_FIELDS: [&str; 5] =
    ["version", "extensions", "publisher", "license", "publicationPolicy"];

/// The shortest span the walk asks for. The API answers a window whose two
/// ends are equal with 400 `'updatedTo' must be later than 'updatedFrom'`
/// (measured 2026-10-01, issue 477's challenge), so a one-second span cannot be
/// asked at all, and [`split`] never makes one.
pub const MIN_SPAN_SECS: i64 = 2;

/// An inclusive span of UK wall-clock seconds, `from..=to`, in the naive
/// encoding [`window_url`] writes: `days_from_civil(day) * 86 400` plus the
/// seconds into that day, with NO time-zone conversion — the server reads its
/// parameters as UK local time (§2). Both ends are inclusive, as the server's
/// are: a release on a boundary second is listed by both windows that share
/// that second (measured, issue 477), so `[a, m]` and `[m + 1, b]` leave no hole.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub from: i64,
    pub to: i64,
}

impl Span {
    /// Seconds covered, both ends counted.
    pub fn secs(self) -> i64 {
        self.to - self.from + 1
    }
}

/// One request window: the civil day it covers and its [`Span`] — the span
/// the walk starts from, and splits when its page is full.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    pub day: (u16, u8, u8),
    pub span: Span,
}

/// The span of a window ending at `day` 23:59:59 and starting `overlap_secs`
/// before `day` 00:00:00.
pub fn window_span(day: (u16, u8, u8), overlap_secs: i64) -> Span {
    let (y, m, d) = day;
    let midnight = days_from_civil(y, m, d) * 86_400;
    Span { from: midnight - overlap_secs, to: midnight + 86_399 }
}

/// `YYYY-MM-DDTHH:MM:SS` of a naive wall-clock second.
fn wall(secs: i64) -> String {
    let (y, m, d) = civil_date(secs);
    let tod = secs.rem_euclid(86_400);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}", tod / 3_600, tod % 3_600 / 60, tod % 60)
}

/// The cursorless URL of a span — UK wall-clock strings exactly as the server
/// interprets them (§2); NO time-zone conversion here.
pub fn span_url(base: &str, span: Span) -> String {
    format!(
        "{base}/ocdsReleasePackages?limit={PAGE_LIMIT}&updatedFrom={}&updatedTo={}",
        wall(span.from),
        wall(span.to)
    )
}

/// The URL of a window ending at `day` 23:59:59 and starting `overlap_secs`
/// before `day` 00:00:00: the first request of the day's walk, and the
/// registry URL of a target. Byte-identical to the URL the cursor walker
/// asked first, so no registry URL changed with issue 477.
pub fn window_url(base: &str, day: (u16, u8, u8), overlap_secs: i64) -> String {
    span_url(base, window_span(day, overlap_secs))
}

/// A span's name in the staging dir: `20210507T000000-20210507T235959`. The
/// staged page of every span asked is the walk's resume state.
pub fn span_key(span: Span) -> String {
    let compact = |secs: i64| wall(secs).replace(['-', ':'], "");
    format!("{}-{}", compact(span.from), compact(span.to))
}

/// The inverse of [`span_key`], strictly: anything that does not render back
/// to itself (a page the cursor walker staged, a hand-made name) is `None`.
pub fn parse_span_key(key: &str) -> Option<Span> {
    let (from, to) = key.split_once('-')?;
    let secs = |s: &str| -> Option<i64> {
        if s.len() != 15 || s.as_bytes()[8] != b'T' {
            return None;
        }
        let num = |range: std::ops::Range<usize>| -> Option<i64> {
            let part = s.get(range)?;
            part.bytes().all(|b| b.is_ascii_digit()).then(|| part.parse().ok())?
        };
        let (y, m, d) = (num(0..4)?, num(4..6)?, num(6..8)?);
        let (h, mi, sec) = (num(9..11)?, num(11..13)?, num(13..15)?);
        Some(days_from_civil(y as u16, m as u8, d as u8) * 86_400 + h * 3_600 + mi * 60 + sec)
    };
    let span = Span { from: secs(from)?, to: secs(to)? };
    (span.to > span.from && span_key(span) == key).then_some(span)
}

/// A span cut in two: `[from, m]` and `[m + 1, to]`, the older half first.
/// `None` when a cut would leave a half shorter than [`MIN_SPAN_SECS`] — the
/// API refuses a one-second window — which the fetcher turns into a loud
/// failure for a span that is still full.
///
/// The older half takes an even number of seconds, so an even span (every
/// window is: 86 400 s, or 93 600 s with the daily overlap) halves into even
/// halves all the way down to two seconds.
///
/// **The repeated autumn hour.** The server turns each END of a window into an
/// instant; on the last Sunday of October UK time runs 01:00–01:59 twice, and
/// how it resolves that ambiguous wall clock is not documented. Probably
/// Java (its error bodies have Spring Boot's shape), whose default is the
/// earlier offset: then wall 01:59:59 is 00:59:59 UTC but 02:00:00 is 02:00:00
/// UTC, and two spans meeting at 02:00:00 leave the second 01:xx hour in
/// neither. Under the later offset the same gap opens at 01:00:00 instead. A
/// cut at any other second joins two wall seconds under one offset, so the
/// halves meet end to end under either rule — so the cut is moved off BOTH
/// seconds rather than betting on one. (The spring hour that does not exist
/// can only make two spans overlap, which the id dedup absorbs; a window's own
/// ends are never at 01:00 or 02:00.) Pinned by
/// `a_split_never_cuts_at_either_edge_of_the_repeated_autumn_hour`.
pub fn split(span: Span) -> Option<(Span, Span)> {
    let n = span.secs();
    if n < 2 * MIN_SPAN_SECS {
        return None;
    }
    let fits = |cut: i64| cut - span.from >= MIN_SPAN_SECS && span.to - cut + 1 >= MIN_SPAN_SECS;
    // `cut` is the first second of the newer half.
    let mut cut = span.from + n / 2 / 2 * 2;
    if autumn_edge(cut) {
        cut = [cut + 2, cut - 2].into_iter().find(|&c| fits(c) && !autumn_edge(c))?;
    }
    Some((Span { from: span.from, to: cut - 1 }, Span { from: cut, to: span.to }))
}

/// 01:00:00 or 02:00:00 on the last Sunday of October: the two wall seconds
/// where, depending on how the server resolves the repeated hour, a cut could
/// open a gap ([`split`]).
fn autumn_edge(wall: i64) -> bool {
    let (year, month, _) = civil_date(wall);
    let sunday = last_sunday(year, 10);
    month == 10 && (wall == sunday + 3_600 || wall == sunday + 7_200)
}

/// A page is the whole of its span when it holds fewer than [`PAGE_LIMIT`]
/// releases AND names no next page. A full page might have more behind it; a
/// short page that still names a next is not what the measured server does,
/// and is not trusted either — both are split, never followed.
pub fn page_is_short(count: usize, next: Option<&str>) -> bool {
    count < PAGE_LIMIT as usize && next.is_none()
}

/// Whether the LAST UK civil day of `month` is over at `now_unix`. A monthly
/// package is registered once and never re-walked, so a monthly of a month
/// that is still running would freeze it part-walked: the fetcher refuses it
/// (issue 477), and its days are the daily walk's.
pub fn month_has_ended(month: (u16, u8), now_unix: i64) -> bool {
    let (y, m, _) = uk_civil_date(now_unix);
    month < (y, m)
}

/// Whether the UK civil `day` is over at `now_unix`. A daily package is
/// registered once and the walk-forward never revisits a registered day, so a
/// daily of the running day would freeze it part-walked, and one of a future
/// day would land empty: the fetcher refuses both (issue 477 review).
pub fn day_has_ended(day: (u16, u8, u8), now_unix: i64) -> bool {
    day < uk_civil_date(now_unix)
}

/// The newest month whose last UK day is over at `now_unix`: where a default
/// FTS backfill ends (issue 477).
pub fn last_ended_month(now_unix: i64) -> (u16, u8) {
    let (y, m, _) = uk_civil_date(now_unix);
    if m == 1 { (y - 1, 12) } else { (y, m - 1) }
}

/// The base URL a window URL was built on — the inverse of [`window_url`], so
/// the fetcher can re-derive a target's windows from its registry identity.
pub fn base_of(url: &str) -> Option<&str> {
    url.split_once("/ocdsReleasePackages").map(|(base, _)| base)
}

/// `YYYY-MM-DD`, the daily period key (zero-padded, so `MAX(period)` sorts).
pub fn ymd(day: (u16, u8, u8)) -> String {
    let (y, m, d) = day;
    format!("{y:04}-{m:02}-{d:02}")
}

/// `YYYY-MM-DD` → (year, month, day); `None` for anything else.
pub fn parse_day(period: &str) -> Option<(u16, u8, u8)> {
    let (y, rest) = period.split_once('-')?;
    let (m, d) = rest.split_once('-')?;
    let day = (y.parse().ok()?, m.parse().ok()?, d.parse().ok()?);
    ((1..=12).contains(&day.1) && (1..=days_in_month(day.0, day.1)).contains(&day.2)).then_some(day)
}

/// `YYYY-MM` → (year, month); `None` for anything else.
pub fn parse_month(period: &str) -> Option<(u16, u8)> {
    let (y, m) = period.split_once('-')?;
    let month = (y.parse().ok()?, m.parse().ok()?);
    ((1..=12).contains(&month.1) && !y.is_empty()).then_some(month)
}

/// Days in a civil month, via the civil-date round-trip (leap years fall out).
pub fn days_in_month(year: u16, month: u8) -> u8 {
    let next = if month == 12 { days_from_civil(year + 1, 1, 1) } else { days_from_civil(year, month + 1, 1) };
    (next - days_from_civil(year, month, 1)) as u8
}

/// The live-poll package for one UK civil day: kind `daily`, period
/// `YYYY-MM-DD`, one window with the 2 h overlap.
pub fn day(base: &str, day: (u16, u8, u8)) -> Target {
    Target {
        source: "fts",
        kind: "daily",
        period: ymd(day),
        url: window_url(base, day, OVERLAP_SECS),
        rel_path: format!("fts/daily/{}.zip", ymd(day)),
    }
}

/// The backfill package for one month: kind `monthly`, period `YYYY-MM`,
/// assembled from one contiguous 1-day window per civil day ([`windows`]).
/// `url` is the first window's first page.
pub fn monthly(base: &str, month: (u16, u8)) -> Target {
    let (y, m) = month;
    Target {
        source: "fts",
        kind: "monthly",
        period: format!("{y:04}-{m:02}"),
        url: window_url(base, (y, m, 1), 0),
        rel_path: format!("fts/monthly/{y:04}-{m:02}.zip"),
    }
}

/// The request windows a target is walked as: a daily is ONE window with the
/// overlap; a monthly is one 1-day window per civil day, contiguous and
/// without overlap. Empty for a target that is not an FTS shape (the fetcher
/// refuses it rather than landing an empty zip).
pub fn windows(target: &Target) -> Vec<Window> {
    if target.source != "fts" {
        return Vec::new();
    }
    match target.kind {
        "daily" => parse_day(&target.period)
            .map(|d| vec![Window { day: d, span: window_span(d, OVERLAP_SECS) }])
            .unwrap_or_default(),
        "monthly" => parse_month(&target.period)
            .map(|(y, m)| {
                (1..=days_in_month(y, m))
                    .map(|d| Window { day: (y, m, d), span: window_span((y, m, d), 0) })
                    .collect()
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// Months from [`FIRST_MONTH`] through `end` inclusive — the backfill walk
/// (the DÖE shape, `doe::months_through`).
pub fn months_through(end: (u16, u8)) -> Vec<(u16, u8)> {
    let mut out = Vec::new();
    let (mut year, mut month) = FIRST_MONTH;
    while (year, month) <= end {
        out.push((year, month));
        (year, month) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
    }
    out
}

/// UK offset from UTC at a unix instant: 0 (GMT) or 3 600 (BST). The UK
/// follows the EU rule — BST from 01:00 UTC on the last Sunday of March to
/// 01:00 UTC on the last Sunday of October — so the switch instants are the
/// supervisor's Berlin ones, one hour less of offset.
pub fn uk_offset(unix: i64) -> i64 {
    let (year, _, _) = civil_date(unix);
    let start = last_sunday(year, 3) + 3_600;
    let end = last_sunday(year, 10) + 3_600;
    if unix >= start && unix < end { 3_600 } else { 0 }
}

/// The UK civil date of a unix instant — what "yesterday" means to the daily
/// probe, since the API's windows are UK-local.
pub fn uk_civil_date(unix: i64) -> (u16, u8, u8) {
    civil_date(unix + uk_offset(unix))
}

/// 00:00 UTC of the last Sunday of `(year, month)`; March and October both
/// have 31 days, which is all this is called for.
fn last_sunday(year: u16, month: u8) -> i64 {
    let z = days_from_civil(year, month, 31);
    let weekday = (z + 4).rem_euclid(7); // 0 = Sunday
    (z - weekday) * 86_400
}

/// The release's `id` — the notice id (`083685-2026`), FTS's publication
/// identity and the member name inside the package.
pub fn release_id(release: &RawValue) -> Option<String> {
    let fields: HashMap<&str, &RawValue> = serde_json::from_str(release.get()).ok()?;
    serde_json::from_str::<String>(fields.get("id")?.get()).ok()
}

/// One page, read WITHOUT building a document over it.
///
/// **A generic JSON document is the wrong tool for publisher payloads, and one
/// live release proves it.** Release `083529-2026` of 3 September 2026 carries
/// `"maximumLotsBidPerSupplier": 1e9999` — the publisher's way of writing "no
/// limit". That is valid JSON syntax and `serde_json::Value` REFUSES it, because
/// a `Value` number must fit an `f64`: "number out of range". Parsed as a
/// document, that one field failed the whole page, which failed the whole day,
/// deterministically, on every tick — the watermark would never have advanced
/// past 3 September 2026. Python's parser accepts it as infinity, which is why
/// the acquisition research never saw it.
///
/// So nothing here parses a number at all. The releases stay RAW: their bytes
/// are carried into the member verbatim, so the archived member is what the
/// publisher served rather than our re-rendering of it, and a value we cannot
/// represent is simply a value we never looked at. Only the fields the fetcher
/// actually reads — the header's five, `links.next`, and a release's `id` —
/// are decoded, and each of those is a string.
pub struct Page<'a> {
    fields: HashMap<&'a str, &'a RawValue>,
}

#[derive(serde::Deserialize)]
struct Links {
    next: Option<String>,
}

impl<'a> Page<'a> {
    /// Read a page's top level. Fails only when the bytes are not a JSON
    /// object; what a missing or odd `releases` means is the CALLER's to say —
    /// the fetcher refuses to archive such a page, while the profile layer
    /// records it as a packaging defect under a profile it can still read.
    pub fn read(bytes: &'a [u8]) -> Result<Self, String> {
        let fields: HashMap<&str, &RawValue> =
            serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        Ok(Self { fields })
    }

    /// The page's releases, still unparsed.
    pub fn releases(&self) -> Result<Vec<&'a RawValue>, String> {
        let raw = self.fields.get("releases").ok_or("not a release package: no `releases` array")?;
        serde_json::from_str(raw.get()).map_err(|e| format!("`releases` is not an array: {e}"))
    }

    /// The package's declared OCDS `version` — the profile the parser gates on.
    pub fn version(&self) -> Option<String> {
        serde_json::from_str(self.fields.get("version")?.get()).ok()
    }

    /// `links.next` — the cursor URL the server offers for a following page.
    /// The walk NEVER follows it (issue 477: the cursor drops rows); it only
    /// reads whether one is named, which is half of [`page_is_short`].
    pub fn next(&self) -> Option<String> {
        let links = self.fields.get("links")?;
        serde_json::from_str::<Links>(links.get()).ok()?.next
    }

    /// One release as its own single-release OCDS package: the header's
    /// [`MEMBER_HEADER_FIELDS`] in that order, then `releases: [release]`, all
    /// spliced as raw bytes. `uri`, `links` and `publishedDate` are DROPPED —
    /// they describe the page, and keeping them would give the same release a
    /// different hash on every fetch.
    pub fn member_bytes(&self, release: &RawValue) -> Vec<u8> {
        let mut out = Vec::with_capacity(release.get().len() + 512);
        out.push(b'{');
        for key in MEMBER_HEADER_FIELDS {
            if let Some(value) = self.fields.get(key) {
                out.extend_from_slice(format!("{}:", serde_json::Value::from(key)).as_bytes());
                out.extend_from_slice(value.get().as_bytes());
                out.push(b',');
            }
        }
        out.extend_from_slice(b"\"releases\":[");
        out.extend_from_slice(release.get().as_bytes());
        out.extend_from_slice(b"]}");
        out
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    /// Issue 477: a split partitions its span — the halves meet end to end
    /// and cover it exactly — and never makes a span the API would refuse.
    #[test]
    fn split_partitions_a_span_into_two_halves_of_at_least_two_seconds() {
        for n in 1..=400 {
            let span = Span { from: 1_000_000, to: 1_000_000 + n - 1 };
            match split(span) {
                Some((older, newer)) => {
                    assert_eq!(older.from, span.from, "{n}");
                    assert_eq!(older.to + 1, newer.from, "{n}: the halves meet with no hole and no overlap");
                    assert_eq!(newer.to, span.to, "{n}");
                    assert!(older.secs() >= MIN_SPAN_SECS && newer.secs() >= MIN_SPAN_SECS, "{n}: {older:?} {newer:?}");
                    assert_eq!(older.secs() % 2, 0, "{n}: the older half is even");
                    assert!(n >= 4);
                }
                None => assert!(n < 4, "{n} seconds can split"),
            }
        }
        // A two-second span is the floor: the API refuses one second
        // (`'updatedTo' must be later than 'updatedFrom'`), so it cannot split.
        assert_eq!(split(Span { from: 10, to: 11 }), None);
        assert_eq!(split(Span { from: 10, to: 12 }), None, "three seconds would leave a one-second half");
        // A day halves at noon, and a day with the daily overlap at 11:00; an
        // even span halves into even spans all the way down to two seconds.
        let day = window_span((2021, 5, 7), 0);
        let (am, pm) = split(day).unwrap();
        assert!(span_url(BASE, am).ends_with("updatedFrom=2021-05-07T00:00:00&updatedTo=2021-05-07T11:59:59"));
        assert!(span_url(BASE, pm).ends_with("updatedFrom=2021-05-07T12:00:00&updatedTo=2021-05-07T23:59:59"));
        let (early, _) = split(window_span((2021, 5, 7), OVERLAP_SECS)).unwrap();
        assert!(span_url(BASE, early).ends_with("updatedFrom=2021-05-06T22:00:00&updatedTo=2021-05-07T10:59:59"));
        let mut span = window_span((2025, 12, 10), OVERLAP_SECS);
        let mut halvings = 0;
        while let Some((older, _)) = split(span) {
            span = older;
            halvings += 1;
        }
        assert_eq!(span.secs(), MIN_SPAN_SECS, "the walk can always reach two seconds");
        assert!(halvings <= 16, "{halvings}");
    }

    /// The day's FIRST request — and with it every registry URL — is the URL
    /// the cursor walker asked, byte for byte (issue 477 changes the walk, not
    /// the package identity).
    #[test]
    fn the_day_span_url_is_the_old_window_url() {
        let cases = [
            ((2026, 9, 3), OVERLAP_SECS, "updatedFrom=2026-09-02T22:00:00&updatedTo=2026-09-03T23:59:59"),
            ((2026, 1, 1), OVERLAP_SECS, "updatedFrom=2025-12-31T22:00:00&updatedTo=2026-01-01T23:59:59"),
            ((2024, 3, 1), OVERLAP_SECS, "updatedFrom=2024-02-29T22:00:00&updatedTo=2024-03-01T23:59:59"),
            ((2021, 5, 7), 0, "updatedFrom=2021-05-07T00:00:00&updatedTo=2021-05-07T23:59:59"),
        ];
        for (day, overlap, window) in cases {
            let expected = format!("{BASE}/ocdsReleasePackages?limit=100&{window}");
            assert_eq!(window_url(BASE, day, overlap), expected);
            assert_eq!(span_url(BASE, window_span(day, overlap)), expected);
        }
        let d = day(BASE, (2026, 9, 3));
        assert_eq!(windows(&d)[0].span, window_span((2026, 9, 3), OVERLAP_SECS));
        assert_eq!(span_url(BASE, windows(&d)[0].span), d.url);
    }

    /// Issue 477's challenge: how the server resolves the repeated autumn hour
    /// is undocumented, and the two plausible rules open a gap at different
    /// seconds — 02:00:00 (earlier offset, Java's default) or 01:00:00 (later
    /// offset). This pins what the walk DOES: no split of any span lets a half
    /// start at either second on the last Sunday of October, so the halves meet
    /// end to end under either rule; every other day, and every other second
    /// of that day, splits exactly as the plain rule says.
    #[test]
    fn a_split_never_cuts_at_either_edge_of_the_repeated_autumn_hour() {
        let sunday = days_from_civil(2026, 10, 25) * 86_400;
        assert_eq!(uk_offset(sunday + 3_600 - 1), 3_600, "2026-10-25 is the autumn switch");
        let (one, two) = (sunday + 3_600, sunday + 7_200);
        // Spans whose plain midpoint is exactly 01:00:00 or 02:00:00.
        for (from, to) in [(sunday, two - 1), (one, sunday + 3 * 3_600 - 1), (one - 4, one + 3), (two - 4, two + 3)] {
            let (older, newer) = split(Span { from, to }).expect("a long enough span still splits");
            assert!(newer.from != one && newer.from != two, "{from}..{to} cut at {}", newer.from);
            assert_eq!(older.to + 1, newer.from);
        }
        // Nowhere on that day does a half start at either edge — walk every
        // span the halving of the day (and of the daily window) can reach.
        for root in [window_span((2026, 10, 25), 0), window_span((2026, 10, 25), OVERLAP_SECS)] {
            let mut stack = vec![root];
            let mut cuts = 0usize;
            while let Some(span) = stack.pop() {
                // Only the spans around the switch matter; the rest halve as any day.
                if span.to < sunday || span.from > sunday + 4 * 3_600 {
                    continue;
                }
                if let Some((older, newer)) = split(span) {
                    assert!(newer.from != one && newer.from != two, "{span:?}");
                    cuts += 1;
                    stack.push(older);
                    stack.push(newer);
                }
            }
            assert!(cuts > 7_000, "the reachable spans were walked: {cuts}");
        }
        // A four-second span centred on an edge has nowhere else to cut.
        assert_eq!(split(Span { from: two - 2, to: two + 1 }), None);
        // The repeated hour itself is asked as plain wall clock, once.
        let hour = Span { from: one, to: two - 1 };
        assert!(span_url(BASE, hour).ends_with("updatedFrom=2026-10-25T01:00:00&updatedTo=2026-10-25T01:59:59"));
        // A year later the same rule applies to that year's last Sunday, and
        // the same seconds a week earlier are ordinary.
        let next_year = days_from_civil(2027, 10, 31) * 86_400;
        let (_, newer) = split(Span { from: next_year, to: next_year + 7_200 - 1 }).unwrap();
        assert_ne!(newer.from, next_year + 3_600);
        let ordinary = days_from_civil(2026, 10, 18) * 86_400;
        let (_, newer) = split(Span { from: ordinary, to: ordinary + 7_200 - 1 }).unwrap();
        assert_eq!(newer.from, ordinary + 3_600);
    }

    #[test]
    fn page_is_short_needs_fewer_than_limit_and_no_next() {
        assert!(page_is_short(0, None), "an empty window is complete");
        assert!(page_is_short(99, None));
        assert!(!page_is_short(100, None), "a full page might have more behind it");
        assert!(!page_is_short(101, None));
        assert!(!page_is_short(3, Some("https://x/ocdsReleasePackages?cursor=1")), "short but naming a next");
        assert!(!page_is_short(100, Some("https://x")));
    }

    /// The staging dir names a page by its span, and only a name that renders
    /// back to itself is one — the cursor walker's pages are not.
    #[test]
    fn a_span_key_round_trips_and_nothing_else_parses() {
        let span = window_span((2021, 5, 7), OVERLAP_SECS);
        assert_eq!(span_key(span), "20210506T220000-20210507T235959");
        assert_eq!(parse_span_key(&span_key(span)), Some(span));
        let (_, newer) = split(span).unwrap();
        assert_eq!(parse_span_key(&span_key(newer)), Some(newer));
        for not_a_span in [
            "2026-09-03-p001",
            "2026-09-03-h03-p001",
            "cursor",
            "20210507T235959-20210507T000000",
            "20210507T000000-20210507T000000",
            "20211307T000000-20211307T235959",
            "20210507T240000-20210508T000000",
            "20210507T00000-20210507T235959",
            "2021O507T000000-20210507T235959",
        ] {
            assert_eq!(parse_span_key(not_a_span), None, "{not_a_span}");
        }
    }

    /// A monthly is refused until its last UK civil day is over — UK, not
    /// UTC: 23:30 UTC on 30 September 2026 is already 1 October in BST.
    #[test]
    fn a_month_ends_with_its_last_uk_civil_day() {
        let sep30 = days_from_civil(2026, 9, 30) * 86_400;
        assert!(!month_has_ended((2026, 9), sep30 + 22 * 3_600 + 59 * 60), "23:59 BST on the 30th");
        assert!(month_has_ended((2026, 9), sep30 + 23 * 3_600), "00:00 BST on 1 October");
        assert!(month_has_ended((2026, 8), sep30));
        assert!(!month_has_ended((2026, 10), sep30 + 23 * 3_600));
        assert_eq!(last_ended_month(sep30 + 22 * 3_600), (2026, 8));
        assert_eq!(last_ended_month(sep30 + 23 * 3_600), (2026, 9));
        // In winter UK time is UTC, and January's previous month is December.
        let jan1 = days_from_civil(2027, 1, 1) * 86_400;
        assert_eq!(last_ended_month(jan1 - 1), (2026, 11));
        assert_eq!(last_ended_month(jan1), (2026, 12));
    }

    /// A daily is refused until its UK civil day is over, by the same clock.
    #[test]
    fn a_day_ends_with_its_uk_civil_day() {
        let sep30 = days_from_civil(2026, 9, 30) * 86_400;
        assert!(!day_has_ended((2026, 9, 30), sep30 + 22 * 3_600 + 59 * 60), "23:59 BST on the 30th");
        assert!(day_has_ended((2026, 9, 30), sep30 + 23 * 3_600), "00:00 BST on 1 October");
        assert!(!day_has_ended((2026, 10, 1), sep30 + 23 * 3_600), "the running day");
        assert!(!day_has_ended((2026, 12, 25), sep30), "a future day");
        assert!(day_has_ended((2026, 9, 29), sep30));
        let jan1 = days_from_civil(2027, 1, 1) * 86_400;
        assert!(!day_has_ended((2026, 12, 31), jan1 - 1), "GMT: 23:59:59 UTC is still the 31st");
        assert!(day_has_ended((2026, 12, 31), jan1));
    }

    #[test]
    fn urls_match_documented_patterns() {
        let d = day(BASE, (2026, 9, 3));
        assert_eq!(d.source, "fts");
        assert_eq!(d.kind, "daily");
        assert_eq!(d.period, "2026-09-03");
        assert_eq!(d.rel_path, "fts/daily/2026-09-03.zip");
        // The 2 h overlap reaches into the previous day, as a bare wall-clock string.
        assert_eq!(
            d.url,
            "https://www.find-tender.service.gov.uk/api/1.0/ocdsReleasePackages?limit=100\
             &updatedFrom=2026-09-02T22:00:00&updatedTo=2026-09-03T23:59:59"
        );
        // The overlap crosses a month AND a year boundary without help.
        assert!(window_url(BASE, (2026, 1, 1), OVERLAP_SECS).contains("updatedFrom=2025-12-31T22:00:00"));
        assert!(window_url(BASE, (2026, 3, 1), OVERLAP_SECS).contains("updatedFrom=2026-02-28T22:00:00"));
        // No overlap: the window is exactly the civil day.
        assert_eq!(
            window_url("http://x", (2021, 1, 2), 0),
            "http://x/ocdsReleasePackages?limit=100&updatedFrom=2021-01-02T00:00:00&updatedTo=2021-01-02T23:59:59"
        );
        assert_eq!(base_of(&d.url), Some(BASE));

        let m = monthly(BASE, (2025, 6));
        assert_eq!(m.kind, "monthly");
        assert_eq!(m.period, "2025-06");
        assert_eq!(m.rel_path, "fts/monthly/2025-06.zip");
        assert!(m.url.contains("updatedFrom=2025-06-01T00:00:00&updatedTo=2025-06-01T23:59:59"));

        // A daily is one overlapping window; a monthly is one window per civil day.
        let dw = windows(&d);
        assert_eq!(dw.len(), 1);
        assert_eq!(dw[0], Window { day: (2026, 9, 3), span: window_span((2026, 9, 3), OVERLAP_SECS) });
        assert_eq!(span_url(BASE, dw[0].span), d.url);
        let mw = windows(&m);
        assert_eq!(mw.len(), 30);
        assert_eq!(span_url(BASE, mw[0].span), m.url);
        assert_eq!(mw[29].day, (2025, 6, 30));
        assert!(span_url(BASE, mw[29].span).contains("updatedFrom=2025-06-30T00:00:00&updatedTo=2025-06-30T23:59:59"));
        assert!(mw.windows(2).all(|w| w[0].span.to + 1 == w[1].span.from), "contiguous, no overlap");
        assert_eq!(windows(&monthly(BASE, (2024, 2))).len(), 29, "leap February");
        assert_eq!(windows(&monthly(BASE, (2025, 12))).len(), 31, "year rollover");

        // Not an FTS shape: nothing to walk (the fetcher refuses rather than landing nothing).
        let ted = Target {
            source: "ted",
            kind: "daily",
            period: "2026-00137".into(),
            url: "https://ted/x".into(),
            rel_path: "ted/daily/2026-00137.tar.gz".into(),
        };
        assert!(windows(&ted).is_empty());
        let mut garbled = day(BASE, (2026, 9, 3));
        garbled.period = "nope".into();
        assert!(windows(&garbled).is_empty());
    }

    #[test]
    fn periods_parse_and_reject_garbage() {
        assert_eq!(parse_day("2026-09-03"), Some((2026, 9, 3)));
        assert_eq!(parse_day("2024-02-29"), Some((2024, 2, 29)));
        assert_eq!(parse_day("2023-02-29"), None, "not a leap year");
        assert_eq!(parse_day("2026-13-01"), None);
        assert_eq!(parse_day("2026-09"), None);
        assert_eq!(parse_month("2026-09"), Some((2026, 9)));
        assert_eq!(parse_month("2026-00"), None);
        assert_eq!(parse_month("2026"), None);
        assert_eq!(days_in_month(2026, 2), 28);
        assert_eq!(days_in_month(2000, 2), 29);
        assert_eq!(days_in_month(1900, 2), 28);
    }

    #[test]
    fn backfill_walk_spans_first_month_through_end() {
        assert_eq!(months_through((2021, 3)), vec![(2021, 1), (2021, 2), (2021, 3)]);
        let long = months_through((2026, 9));
        assert_eq!(long.first(), Some(&FIRST_MONTH));
        assert_eq!(long.last(), Some(&(2026, 9)));
        assert_eq!(long.len(), 12 * 5 + 9, "2021–2025 whole, 2026 through September");
        assert!(months_through((2020, 12)).is_empty(), "nothing before the service existed");
    }

    /// The UK rule: GMT in winter, BST in summer, switching at 01:00 UTC on the
    /// last Sundays of March and October — the same instants as the EU switch.
    #[test]
    fn uk_offset_follows_the_dst_rule_at_both_edges() {
        // 2026: last Sunday of March is the 29th; October is the 25th.
        let mar29 = days_from_civil(2026, 3, 29) * 86_400;
        assert_eq!(uk_offset(mar29 + 30 * 60), 0, "00:30 UTC: still GMT");
        assert_eq!(uk_offset(mar29 + 3_600 - 1), 0, "00:59:59 UTC: still GMT");
        assert_eq!(uk_offset(mar29 + 3_600), 3_600, "01:00 UTC: BST begins");
        assert_eq!(uk_offset(mar29 + 3_600 + 30 * 60), 3_600);

        let oct25 = days_from_civil(2026, 10, 25) * 86_400;
        assert_eq!(uk_offset(oct25 + 30 * 60), 3_600, "00:30 UTC: still BST");
        assert_eq!(uk_offset(oct25 + 3_600 - 1), 3_600);
        assert_eq!(uk_offset(oct25 + 3_600), 0, "01:00 UTC: back to GMT");

        // Deep winter and deep summer, and the civil date that follows from it.
        assert_eq!(uk_offset(days_from_civil(2026, 1, 15) * 86_400), 0);
        assert_eq!(uk_offset(days_from_civil(2026, 7, 15) * 86_400), 3_600);
        // 23:30 UTC on a summer day is already the next UK civil day.
        assert_eq!(uk_civil_date(days_from_civil(2026, 7, 15) * 86_400 + 23 * 3_600 + 30 * 60), (2026, 7, 16));
        // ...and in winter it is not.
        assert_eq!(uk_civil_date(days_from_civil(2026, 1, 15) * 86_400 + 23 * 3_600 + 30 * 60), (2026, 1, 15));
    }

    /// A member is the header's five fields, in that order, then the release's
    /// OWN BYTES — so the archive holds what the publisher served rather than
    /// our re-rendering of it, and nothing in a release is ever converted.
    #[test]
    fn a_member_is_the_headers_fields_then_the_releases_own_bytes() {
        // A page as the API serves one: pretty-printed, keys in the server's
        // order, page-specific fields present.
        let served = br#"{
            "uri": "https://x/ocdsReleasePackages?updatedFrom=A&cursor=one",
            "version": "1.1",
            "extensions": ["https://ext/one.json"],
            "publisher": {"name": "Cabinet Office", "scheme": "GB-GOR", "uid": "D2"},
            "license": "http://www.nationalarchives.gov.uk/doc/open-government-licence/version/3/",
            "publicationPolicy": "https://www.gov.uk/government/publications/open-contracting",
            "publishedDate": "2026-09-03T23:31:32+01:00",
            "releases": [
                {"id": "083685-2026", "ocid": "ocds-h6vhtk-06f1bc", "tender": {"title": "T"}},
                {"ocid": "ocds-h6vhtk-000000"}
            ],
            "links": {"next": "https://x/ocdsReleasePackages?cursor=two"}
        }"#;
        let page = Page::read(served).unwrap();
        assert_eq!(page.version().as_deref(), Some("1.1"));
        assert_eq!(page.next().as_deref(), Some("https://x/ocdsReleasePackages?cursor=two"));
        let releases = page.releases().unwrap();
        assert_eq!(releases.len(), 2);
        assert_eq!(release_id(releases[0]).as_deref(), Some("083685-2026"));
        assert_eq!(release_id(releases[1]), None, "no id field");

        let member = page.member_bytes(releases[0]);
        assert_eq!(member, page.member_bytes(releases[0]), "stable across calls");
        // The release's bytes are IN there, untouched.
        let raw = releases[0].get();
        assert!(
            String::from_utf8_lossy(&member).contains(raw),
            "the release is spliced verbatim, not re-rendered"
        );
        // The header's fields in MEMBER_HEADER_FIELDS order, and nothing of the page.
        let text = String::from_utf8(member.clone()).unwrap();
        let order: Vec<usize> = MEMBER_HEADER_FIELDS
            .iter()
            .map(|k| text.find(&format!("\"{k}\"")).unwrap_or_else(|| panic!("{k} is carried into every member")))
            .collect();
        assert!(order.windows(2).all(|w| w[0] < w[1]), "header fields keep their declared order");
        for dropped in ["uri", "links", "publishedDate"] {
            assert!(!text.contains(dropped), "{dropped} is page-specific and must not be in a member");
        }
        // And it is a package a reader can read back.
        let back = Page::read(&member).unwrap();
        assert_eq!(back.version().as_deref(), Some("1.1"));
        assert_eq!(back.next(), None);
        assert_eq!(back.releases().unwrap().len(), 1);

        let other = page.member_bytes(releases[1]);
        assert_ne!(other, member, "a different release is different bytes");
    }

    /// THE RELEASE THAT FORCED THE RAW READER (live, 3 September 2026, release
    /// `083529-2026`): `1e9999` is valid JSON and no `f64` holds it. A document
    /// parser refuses the whole page, which refused the whole day, for ever.
    #[test]
    fn a_number_no_f64_can_hold_passes_through_untouched() {
        let served = br#"{"version":"1.1","license":"OGL",
            "releases":[{"id":"083529-2026","tender":{"lotDetails":{"maximumLotsBidPerSupplier":1e9999}}}]}"#;
        // The document parser is where this used to die.
        assert!(
            serde_json::from_slice::<serde_json::Value>(served).is_err(),
            "the premise: a Value cannot hold this page"
        );

        let page = Page::read(served).expect("the raw reader reads it");
        let releases = page.releases().unwrap();
        assert_eq!(release_id(releases[0]).as_deref(), Some("083529-2026"));
        let member = page.member_bytes(releases[0]);
        assert!(
            String::from_utf8_lossy(&member).contains("1e9999"),
            "the publisher's value survives into the archive"
        );
        // And the member reads back as a package, so the profile layer can
        // dispatch it into a notice instead of a quarantine row.
        let back = Page::read(&member).unwrap();
        assert_eq!(back.version().as_deref(), Some("1.1"));
        assert_eq!(release_id(back.releases().unwrap()[0]).as_deref(), Some("083529-2026"));
    }

    /// What is NOT a page: the reader must refuse a shape we would otherwise
    /// archive as one, and refuse it as a message rather than a panic.
    #[test]
    fn a_shape_that_is_not_a_release_package_is_refused() {
        // `read` answers "is this a JSON object"; `releases` answers "is it a
        // release package", because the two callers disagree about what to do
        // with the second answer.
        assert!(Page::read(br#"{"error": "not a package"}"#).unwrap().releases().is_err(), "no releases array");
        assert!(Page::read(br#"{"releases": {"a": 1}}"#).unwrap().releases().is_err(), "not an array");
        assert!(Page::read(b"[]").is_err(), "not an object");
        assert!(Page::read(b"{").is_err(), "truncated");
        // An empty page IS a page: a Sunday, or December 2020.
        let empty = Page::read(br#"{"version":"1.1","releases":[]}"#).unwrap();
        assert!(empty.releases().unwrap().is_empty());
        assert_eq!(empty.next(), None);
    }
}
