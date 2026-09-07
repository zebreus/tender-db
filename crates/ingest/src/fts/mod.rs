//! UK Find a Tender Service (FTS) addressing and packaging
//! (docs/research/uk-fts.md, issue 342 unit 2).
//!
//! FTS serves OCDS 1.1 release packages from one unauthenticated endpoint,
//! `GET {BASE}/ocdsReleasePackages?updatedFrom=&updatedTo=`, ≤100 releases a
//! page, paged by an opaque `links.next` URL. There is no bulk package to
//! download, so the fetcher ([`crate::fetch::fetch_fts`]) walks the pages of a
//! window and ASSEMBLES the archive package itself: one zip per (kind, period),
//! one member `<release id>.json` per release, each member a single-release
//! OCDS package built by [`member_bytes`].
//!
//! Two kinds, mirroring TED/DÖE so `Process{daily, None}` re-walks only live
//! days (plan D2):
//! - `daily` — the live poll: ONE window over a UK civil day with a 2 h overlap
//!   on `updatedFrom` ([`OVERLAP_SECS`]), so a release updated around midnight
//!   is never lost between two ticks. The overlap re-yields releases already
//!   archived the day before; they dedup on identity (D3), because
//!   [`member_bytes`] is byte-deterministic.
//! - `monthly` — the backfill package: one contiguous 1-day window per civil
//!   day of the month, no overlap (§2 of the research: wide windows were
//!   rate-limited before answering).
//!
//! `updatedFrom`/`updatedTo` are interpreted by the server in UK local time
//! (GMT/BST) and are sent as bare wall-clock strings; nothing here converts a
//! time zone — [`uk_offset`] exists only so the supervisor can name "yesterday"
//! in UK civil time.
//!
//! **Member bytes are a re-serialisation, not bytes-as-served** — the first
//! deviation from docs/architecture.md's "archive the bytes the source sent".
//! A page's composition shifts under the overlap and the cursor, so the page is
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
/// A month, so a normal gap (a few failed ticks, a weekend of downtime) closes
/// in one run while a watermark left far behind cannot hold the job runner for
/// hours: an FTS day costs 4–5 paced requests plus any back-off, where a DÖE day
/// costs one download. The remainder is the next tick's work — the watermark
/// advances per landed day — and a real backfill is the monthly path, not this.
pub const PROBE_DAY_CAP: usize = 31;

/// The header fields carried into every member package. Everything else on a
/// page header (`uri`, `links`, `publishedDate`) describes THAT PAGE, not the
/// release, and would make the same release hash differently on every fetch.
pub const MEMBER_HEADER_FIELDS: [&str; 5] =
    ["version", "extensions", "publisher", "license", "publicationPolicy"];

/// One `updatedFrom..updatedTo` request window: the civil day it covers and
/// the URL of its first page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Window {
    pub day: (u16, u8, u8),
    pub url: String,
}

/// The first-page URL of a window ending at `day` 23:59:59 and starting
/// `overlap_secs` before `day` 00:00:00 — UK wall-clock strings exactly as the
/// server interprets them (§2); NO time-zone conversion here.
pub fn window_url(base: &str, day: (u16, u8, u8), overlap_secs: i64) -> String {
    let (y, m, d) = day;
    let from = days_from_civil(y, m, d) * 86_400 - overlap_secs;
    let (fy, fm, fd) = civil_date(from);
    let tod = from.rem_euclid(86_400);
    format!(
        "{base}/ocdsReleasePackages?limit={PAGE_LIMIT}\
         &updatedFrom={fy:04}-{fm:02}-{fd:02}T{:02}:{:02}:{:02}\
         &updatedTo={y:04}-{m:02}-{d:02}T23:59:59",
        tod / 3_600,
        tod % 3_600 / 60,
        tod % 60
    )
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
pub fn windows(base: &str, target: &Target) -> Vec<Window> {
    if target.source != "fts" {
        return Vec::new();
    }
    match target.kind {
        "daily" => parse_day(&target.period)
            .map(|d| vec![Window { day: d, url: window_url(base, d, OVERLAP_SECS) }])
            .unwrap_or_default(),
        "monthly" => parse_month(&target.period)
            .map(|(y, m)| {
                (1..=days_in_month(y, m))
                    .map(|d| Window { day: (y, m, d), url: window_url(base, (y, m, d), 0) })
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

    /// `links.next` — the cursor URL of the following page, absent on the last.
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
        let dw = windows(BASE, &d);
        assert_eq!(dw.len(), 1);
        assert_eq!(dw[0], Window { day: (2026, 9, 3), url: d.url.clone() });
        let mw = windows(BASE, &m);
        assert_eq!(mw.len(), 30);
        assert_eq!(mw[0].url, m.url);
        assert_eq!(mw[29].day, (2025, 6, 30));
        assert!(mw[29].url.contains("updatedFrom=2025-06-30T00:00:00&updatedTo=2025-06-30T23:59:59"));
        assert_eq!(windows(BASE, &monthly(BASE, (2024, 2))).len(), 29, "leap February");
        assert_eq!(windows(BASE, &monthly(BASE, (2025, 12))).len(), 31, "year rollover");

        // Not an FTS shape: nothing to walk (the fetcher refuses rather than landing nothing).
        let ted = Target {
            source: "ted",
            kind: "daily",
            period: "2026-00137".into(),
            url: "https://ted/x".into(),
            rel_path: "ted/daily/2026-00137.tar.gz".into(),
        };
        assert!(windows(BASE, &ted).is_empty());
        let mut garbled = day(BASE, (2026, 9, 3));
        garbled.period = "nope".into();
        assert!(windows(BASE, &garbled).is_empty());
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
