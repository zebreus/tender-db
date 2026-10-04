//! The per-year id invariant and the by-id audit (issue 477 unit 3).
//!
//! FTS notice ids are a zero-padded per-year sequence, `NNNNNN-YYYY`
//! (docs/research/uk-fts.md §7), so a year's highest id held is the number of
//! notices issued that year so far, and every id below it is one of:
//! - **held** — a notice row carries it;
//! - **absent** — the API answers it by id with 404/410 or a package with no
//!   release: an id issued and never published (482's probes found such ids;
//!   19 of 19 checked in the 2026-09 dailies were). Only while the endpoint is
//!   PROVEN to answer for a known id ([`CONTROL_EVERY`]): a wrong base or an
//!   outage page served as 404 must not stamp every id absent;
//! - **quarantined** — not a notice row, but an archived member the processor
//!   quarantined carries it: the fetch did not lose it, so it is accounted for
//!   (and a refetch would not add a row);
//! - **present** — the API serves it by id, so a fetch lost it: the release's
//!   day (and its held neighbours' packages) is where a refetch recovers it;
//! - **unaccounted** — not held and not yet answered (no probe, or a probe that
//!   failed: `error`).
//!
//! The coverage denominator follows: `published = highest − absent`, and a
//! year is `fetch_complete` exactly when `held + absent == highest`. The
//! `audit-fts-ids` job (`crate::fetch::audit_fts_ids`) asks each missing id,
//! and these are the pure parts it and the dashboard share.
//!
//! **What this cannot see** (issue 477, "Still open"): an id ABOVE the highest
//! held, and a second release of a HELD id (the id is held, so it is never
//! missing). For the current year the ids above the highest are the newest
//! notices, the daily probe's job. For a CLOSED year they are ids a fetch lost
//! after the last one held (the year's final days): `complete` is bounded by
//! the highest id held, not the highest issued.

use super::{Page, release_id, release_string, uk_wall_of, ymd};
use crate::fetch::civil_date;
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// The fetch-registry and ledger source this audit covers.
pub const SOURCE: &str = "fts";

/// The stored report's kind (`/admin/reports/audit-fts-ids`).
pub const REPORT_KIND: &str = "audit-fts-ids";

/// Consecutive `error` verdicts after which a run halts: the API (or the path
/// to it) is down, and asking the remaining ids would only stamp each one
/// `error` at the 12 s pace. The answers so far stay in the ledger, so a re-run
/// resumes; the job fails so the halt is seen.
pub const ERROR_STREAK_CAP: usize = 5;

/// A run asks a HELD id (its year's highest) before its first by-id request and
/// again after every this many: a 404 counts as "no such notice" only while the
/// endpoint serves a notice we know exists. A control that does not answer
/// `present` halts the run and demotes the absents recorded since the last
/// passed control to `error`, so a wrong `fts_base`, an API path change or an
/// outage served as 404 costs at most this many requests and no false absent.
/// 2 % more requests (~24 on 1,200 ids).
pub const CONTROL_EVERY: usize = 50;

/// An absent id of the CURRENT year is re-asked by default once its answer is
/// this old (30 days): an id can be reserved now and published later, and a
/// walk that drops it then would otherwise leave it out of the denominator for
/// good. Closed years' absents are re-asked only on request
/// (`recheck_absent_days`).
pub const CURRENT_YEAR_ABSENT_RECHECK_SECS: i64 = 30 * 86_400;

/// `NNNNNN-YYYY` as (year, sequence). `None` for any other shape: the ledger and
/// the invariant only count ids of the sequence.
pub fn parse_id(id: &str) -> Option<(u16, u32)> {
    let (seq, year) = id.split_once('-')?;
    if seq.len() < 6 || year.len() != 4 || !seq.bytes().chain(year.bytes()).all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some((year.parse().ok()?, seq.parse().ok()?))
}

/// The id of (year, sequence), zero-padded to six digits.
pub fn format_id(year: u16, seq: u32) -> String {
    format!("{seq:06}-{year}")
}

/// The by-id URL: one notice's release package (`GET
/// /ocdsReleasePackages/009911-2021` answered 200 with its one release,
/// 2026-10-01; a never-published id answers 404 or an empty package).
pub fn id_url(base: &str, id: &str) -> String {
    format!("{base}/ocdsReleasePackages/{id}")
}

/// One id's verdict.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    /// 404 / 410, or a package with no release: never published.
    Absent,
    /// The API serves a release of the id: a fetch lost it.
    Present,
    /// No answer that decides it: a transport failure, a throttle that outlived
    /// the retries, a 5xx, another 4xx, a body that is not a release package,
    /// releases of other ids only, or a 200 with an EMPTY body (FTS sends those
    /// for records that exist: `04196f`, issue 477 unit 1b). Re-asked next run.
    Error,
    /// Not asked: an archived member the processor quarantined carries the id,
    /// so the fetch did not lose it. Never re-asked; a reclaim makes it held.
    Quarantined,
}

impl Verdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Verdict::Absent => "absent",
            Verdict::Present => "present",
            Verdict::Error => "error",
            Verdict::Quarantined => "quarantined",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "absent" => Some(Verdict::Absent),
            "present" => Some(Verdict::Present),
            "error" => Some(Verdict::Error),
            "quarantined" => Some(Verdict::Quarantined),
            _ => None,
        }
    }
}

/// What one by-id request came back with.
#[derive(Debug, Clone, Copy)]
pub enum Answer<'a> {
    /// A 200 and its body.
    Body(&'a [u8]),
    /// A non-200 status the server answered with (after the retry policy).
    Status(u16),
    /// No decisive answer: the request failed, with the status if one arrived
    /// (a throttle that outlived its retries).
    Failed { status: Option<u16>, what: &'a str },
}

/// One classified probe — what the ledger row records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Probe {
    pub verdict: Verdict,
    pub http_status: Option<u16>,
    /// For `Present`: the earliest release `date` of the id, as served.
    pub published: Option<String>,
    /// That date's UK civil day, `YYYY-MM-DD` (the day the API's windows count in).
    pub published_day: Option<String>,
    /// The ocid of that release.
    pub ocid: Option<String>,
    /// Releases of the id in the answer.
    pub releases: usize,
    /// What decided the verdict, in words.
    pub detail: String,
}

impl Probe {
    fn bare(verdict: Verdict, http_status: Option<u16>, detail: impl Into<String>) -> Self {
        Probe { verdict, http_status, published: None, published_day: None, ocid: None, releases: 0, detail: detail.into() }
    }
}

/// Classify the by-id answer for `id`.
///
/// - 404 or 410, or a package whose `releases` is empty: **absent**. These are
///   the API's two ways of saying "no such notice" (482's probes, 2026-10-02/03).
/// - A 200 with an EMPTY body: **error**, never absent. FTS answered `04196f`'s
///   record, whose release was on the listing page, with three empty 200s
///   (issue 477 unit 1b), and a truncating proxy looks the same.
/// - A 200 whose releases include one with `id`: **present**, with the earliest
///   `date` among them (a fanned-out notice serves several, issue 477 unit 1b).
/// - Anything else: **error**, re-asked next run. That includes a package
///   holding only OTHER ids' releases: it does not say the id is absent, and
///   calling it absent would take the id out of the denominator on a server
///   quirk.
pub fn classify(id: &str, answer: Answer) -> Probe {
    match answer {
        Answer::Status(status @ (404 | 410)) => Probe::bare(Verdict::Absent, Some(status), status.to_string()),
        Answer::Status(status) => Probe::bare(Verdict::Error, Some(status), format!("HTTP {status}")),
        Answer::Failed { status, what } => Probe::bare(Verdict::Error, status, what),
        Answer::Body(bytes) if bytes.iter().all(u8::is_ascii_whitespace) => {
            Probe::bare(Verdict::Error, Some(200), "empty body")
        }
        Answer::Body(bytes) => {
            let page = match Page::read(bytes) {
                Ok(page) => page,
                Err(e) => return Probe::bare(Verdict::Error, Some(200), format!("not a release package: {e}")),
            };
            let releases = match page.releases() {
                Ok(releases) => releases,
                Err(e) => return Probe::bare(Verdict::Error, Some(200), e),
            };
            if releases.is_empty() {
                return Probe::bare(Verdict::Absent, Some(200), "empty package");
            }
            let own: Vec<_> = releases.iter().filter(|r| release_id(r).as_deref() == Some(id)).collect();
            if own.is_empty() {
                let others: BTreeSet<String> = releases.iter().filter_map(|r| release_id(r)).collect();
                return Probe::bare(
                    Verdict::Error,
                    Some(200),
                    format!(
                        "{} release(s) of other ids only ({})",
                        releases.len(),
                        others.into_iter().take(3).collect::<Vec<_>>().join(", ")
                    ),
                );
            }
            // The earliest by UK wall clock where the dates parse, else by text.
            let earliest = own
                .iter()
                .map(|r| (release_string(r, "date"), release_string(r, "ocid")))
                .min_by_key(|(date, _)| {
                    let wall = date.as_deref().and_then(uk_wall_of);
                    (wall.is_none(), wall, date.clone())
                })
                .expect("own is not empty");
            let published_day = earliest.0.as_deref().and_then(uk_wall_of).map(|wall| ymd(civil_date(wall)));
            Probe {
                verdict: Verdict::Present,
                http_status: Some(200),
                published: earliest.0,
                published_day,
                ocid: earliest.1,
                releases: own.len(),
                detail: format!("{} release(s) of the id", own.len()),
            }
        }
    }
}

/// The held ids, per year: every `NNNNNN-YYYY` of `ids`, deduplicated.
pub fn held_ids<'a>(ids: impl IntoIterator<Item = &'a str>) -> BTreeMap<u16, BTreeSet<u32>> {
    let mut held: BTreeMap<u16, BTreeSet<u32>> = BTreeMap::new();
    for (year, seq) in ids.into_iter().filter_map(parse_id) {
        held.entry(year).or_default().insert(seq);
    }
    held
}

/// Every id from 1 to each year's highest held that is not held, by (year, seq).
pub fn missing_ids(held: &BTreeMap<u16, BTreeSet<u32>>) -> Vec<(u16, u32)> {
    let mut out = Vec::new();
    for (&year, seqs) in held {
        let Some(&highest) = seqs.last() else { continue };
        out.extend((1..=highest).filter(|seq| !seqs.contains(seq)).map(|seq| (year, seq)));
    }
    out
}

/// One year of the invariant.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct YearIds {
    pub year: u16,
    /// The highest id held: the ids the year has issued, as far as we know.
    pub highest: u32,
    /// Distinct ids held in `1..=highest`.
    pub held: u32,
    /// Missing ids the ledger shows absent from the API.
    pub absent: u32,
    /// Missing ids the API serves (a refetch recovers them).
    pub present: u32,
    /// Missing ids whose latest probe failed.
    pub errors: u32,
    /// Missing ids an archived, quarantined member carries (accounted for: the
    /// fetch did not lose them; they are still published, so in the denominator).
    pub quarantined: u32,
    /// Missing ids never asked.
    pub unchecked: u32,
}

impl YearIds {
    /// `highest − held`: what the issue's Verify counts.
    pub fn missing(&self) -> u32 {
        self.highest - self.held
    }

    /// The denominator: the ids issued less those never published.
    pub fn published(&self) -> u32 {
        self.highest - self.absent
    }

    /// Ids neither held, quarantined nor shown absent: `present + errors + unchecked`.
    pub fn unaccounted(&self) -> u32 {
        self.highest - self.held - self.absent - self.quarantined
    }

    /// `held + quarantined + absent == highest`: every id is archived or shown
    /// never published.
    pub fn complete(&self) -> bool {
        self.unaccounted() == 0
    }

    /// `held / published`, the coverage ratio on the id basis.
    pub fn ratio(&self) -> Option<f64> {
        (self.published() > 0).then(|| f64::from(self.held) / f64::from(self.published()))
    }
}

/// The invariant per year: the held ids against the ledger's verdicts on the
/// missing ones. A ledger row of an id that is now held (a refetch recovered
/// it) or above the year's highest is not a missing id and is ignored.
pub fn census(held: &BTreeMap<u16, BTreeSet<u32>>, ledger: &HashMap<String, Verdict>) -> Vec<YearIds> {
    let mut years: BTreeMap<u16, YearIds> = BTreeMap::new();
    for (&year, seqs) in held {
        let Some(&highest) = seqs.last() else { continue };
        years.insert(year, YearIds { year, highest, held: seqs.range(1..=highest).count() as u32, ..Default::default() });
    }
    for (year, seq) in missing_ids(held) {
        let entry = years.get_mut(&year).expect("a missing id's year is held");
        match ledger.get(&format_id(year, seq)) {
            Some(Verdict::Absent) => entry.absent += 1,
            Some(Verdict::Present) => entry.present += 1,
            Some(Verdict::Error) => entry.errors += 1,
            Some(Verdict::Quarantined) => entry.quarantined += 1,
            None => entry.unchecked += 1,
        }
    }
    years.into_values().collect()
}

/// The missing ids a run asks, in (year, seq) order: those never asked, those
/// whose last probe failed, and absent ones whose answer is stale — an id can
/// be published late. An absent id of `current_year` is stale after
/// [`CURRENT_YEAR_ABSENT_RECHECK_SECS`] by default; `recheck_absent_after`
/// makes every year's absents stale after that many seconds (the smaller
/// window wins for the current year). A `present` id is never re-asked: it
/// exists, and what recovers it is a refetch of its day, not another probe. A
/// `quarantined` one is archived and never asked.
pub fn due(
    missing: &[(u16, u32)],
    ledger: &HashMap<String, (Verdict, i64)>,
    now: i64,
    current_year: u16,
    recheck_absent_after: Option<i64>,
) -> Vec<String> {
    missing
        .iter()
        .filter_map(|&(year, seq)| {
            let id = format_id(year, seq);
            let asked = match ledger.get(&id) {
                None | Some((Verdict::Error, _)) => true,
                Some((Verdict::Present | Verdict::Quarantined, _)) => false,
                Some((Verdict::Absent, checked_at)) => {
                    let default = (year == current_year).then_some(CURRENT_YEAR_ABSENT_RECHECK_SECS);
                    let after = match (default, recheck_absent_after) {
                        (Some(a), Some(b)) => Some(a.min(b)),
                        (a, b) => a.or(b),
                    };
                    after.is_some_and(|after| now - checked_at >= after)
                }
            };
            asked.then_some(id)
        })
        .collect()
}

/// The publication id an archived FTS member's name carries: `NNNNNN-YYYY.json`
/// or `NNNNNN-YYYY~<hash>.json` (a second release of the id). `None` for any
/// other shape.
pub fn member_id(member_path: &str) -> Option<&str> {
    let name = member_path.rsplit('/').next()?.strip_suffix(".json")?;
    let id = name.split_once('~').map_or(name, |(id, _)| id);
    parse_id(id).map(|_| id)
}

/// The packages `(kind, period)` whose refetch can recover a present id:
/// - the packages that hold its nearest held neighbours (`neighbours`): ids are
///   issued in publication order and the API's windows select on that hidden
///   publication instant, so a neighbour's package is where the id was LISTED;
/// - the package of its release date's UK day (`published_day`): the monthly of
///   that month while monthlies reach it (`newest_monthly`), else the day's
///   daily. A hint only — the release `date` is not the window's instant
///   (009921-2021, dated 2021-07-27, is listed on 2021-05-07).
///
/// Sorted and distinct; only `daily`/`monthly` packages.
pub fn refetch_packages(
    published_day: Option<(u16, u8, u8)>,
    neighbours: &[(String, String)],
    newest_monthly: Option<(u16, u8)>,
) -> Vec<(String, String)> {
    let mut out: BTreeSet<(String, String)> =
        neighbours.iter().filter(|(kind, _)| kind == "daily" || kind == "monthly").cloned().collect();
    if let Some((y, m, d)) = published_day {
        if newest_monthly.is_some_and(|newest| (y, m) <= newest) {
            out.insert(("monthly".into(), format!("{y:04}-{m:02}")));
        } else {
            out.insert(("daily".into(), format!("{y:04}-{m:02}-{d:02}")));
        }
    }
    out.into_iter().collect()
}

/// The exact `/admin/jobs` bodies that refetch `packages` and fold what lands:
/// one `fetch` with `refetch:true` per package, then one whole-kind `process`
/// per kind touched (it walks the new versions; held members dedup by hash),
/// then one `project`. Empty for no package.
pub fn enqueue_commands(packages: &BTreeSet<(String, String)>) -> Vec<String> {
    let mut out: Vec<String> = packages
        .iter()
        .map(|(kind, period)| {
            serde_json::json!({
                "kind": "fetch", "source": SOURCE, "package_kind": kind, "period": period, "refetch": true,
            })
            .to_string()
        })
        .collect();
    let kinds: BTreeSet<&str> = packages.iter().map(|(kind, _)| kind.as_str()).collect();
    for kind in kinds {
        out.push(serde_json::json!({ "kind": "process", "source": SOURCE, "package_kind": kind }).to_string());
    }
    if !packages.is_empty() {
        out.push(serde_json::json!({ "kind": "project" }).to_string());
    }
    out
}

/// A present id in the report: where it is, and what to refetch for it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct PresentId {
    pub id: String,
    pub published: Option<String>,
    pub published_day: Option<String>,
    pub ocid: Option<String>,
    /// `kind period`, e.g. `monthly 2021-05` ([`refetch_packages`]).
    pub packages: Vec<String>,
}

/// The `audit-fts-ids` report: the run's tally, the invariant per year after
/// it, every absent id (the issue's Verify wants the residue listed), every
/// present id with its packages, and the enqueue bodies that recover them.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct AuditReport {
    pub dry_run: bool,
    /// Missing ids in total, and how many of them this run was due to ask.
    pub missing: u64,
    pub due: u64,
    /// What this run asked and what came back.
    pub probed: u64,
    pub absent: u64,
    pub present: u64,
    pub errors: u64,
    /// Missing ids found quarantined (an archived member carries them), and
    /// the control requests asked of held ids ([`CONTROL_EVERY`]).
    pub quarantined: u64,
    pub controls: u64,
    /// A cancel stopped the run between two requests.
    pub stopped: bool,
    /// Why the run halted early on consecutive errors ([`ERROR_STREAK_CAP`]).
    pub halted: Option<String>,
    /// The invariant per year, after the run, from the whole ledger.
    pub years: Vec<YearIds>,
    /// Over all years: `held + absent == highest` everywhere.
    pub complete: bool,
    pub unaccounted: u64,
    pub absent_ids: Vec<String>,
    pub present_ids: Vec<PresentId>,
    pub error_ids: Vec<String>,
    pub quarantined_ids: Vec<String>,
    /// The `/admin/jobs` bodies that refetch the present ids' packages.
    pub enqueue: Vec<String>,
}

impl AuditReport {
    /// The job row's one line.
    pub fn summary(&self) -> String {
        let years = self
            .years
            .iter()
            .filter(|y| y.missing() > 0)
            .map(|y| {
                format!(
                    "{} {}/{} held, {} absent, {} present, {} unaccounted",
                    y.year,
                    y.held,
                    y.published(),
                    y.absent,
                    y.present,
                    y.unaccounted()
                )
            })
            .collect::<Vec<_>>();
        format!(
            "{} missing id(s), {} due, {} probed ({} absent, {} present, {} error){}; after: {} unaccounted{}; \
             {} package(s) to refetch{}",
            self.missing,
            self.due,
            self.probed,
            self.absent,
            self.present,
            self.errors,
            if self.quarantined > 0 { format!(", {} quarantined", self.quarantined) } else { String::new() },
            self.unaccounted,
            if self.complete { " — every year complete" } else { "" },
            self.enqueue.iter().filter(|c| c.contains("\"fetch\"")).count(),
            if years.is_empty() { String::new() } else { format!(" [{}]", years.join("; ")) },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "009911-2021";

    fn package(releases: &str) -> String {
        format!(
            r#"{{"version":"1.1","uri":"u","publishedDate":"2026-10-04T00:00:00Z","publisher":{{"name":"FTS"}},"releases":[{releases}]}}"#
        )
    }

    fn release(id: &str, date: &str, ocid: &str) -> String {
        format!(r#"{{"id":"{id}","date":"{date}","ocid":"{ocid}","tag":["tender"]}}"#)
    }

    #[test]
    fn ids_parse_and_format_round_trip_and_only_the_sequence_shape_counts() {
        assert_eq!(parse_id(ID), Some((2021, 9911)));
        assert_eq!(format_id(2021, 9911), ID);
        assert_eq!(format_id(2026, 7), "000007-2026");
        assert_eq!(parse_id("1234567-2026"), Some((2026, 1_234_567)), "a seventh digit is still the sequence");
        for bad in ["9911-2021", "009911-21", "00991a-2021", "ocds-h6vhtk-04196f", "009911_2021", ""] {
            assert_eq!(parse_id(bad), None, "{bad}");
        }
        assert_eq!(id_url("https://x/api/1.0", ID), "https://x/api/1.0/ocdsReleasePackages/009911-2021");
    }

    /// Issue 477 unit 3: the API's two "no such notice" answers are absent, a
    /// release of the id is present with its date, and everything else is an
    /// error that the next run asks again — never a silent absent.
    #[test]
    fn the_by_id_answer_classifies_into_absent_present_or_error() {
        // Absent: 404, 410, an empty package.
        for (answer, detail) in [(Answer::Status(404), "404"), (Answer::Status(410), "410")] {
            let probe = classify(ID, answer);
            assert_eq!(probe.verdict, Verdict::Absent, "{detail}");
            assert_eq!(probe.detail, detail);
        }
        // An empty 200 is an ERROR, re-asked next run: FTS sent three for
        // `04196f`'s existing record (issue 477 unit 1b), so it never means absent.
        for body in [&b""[..], b" \n"] {
            let probe = classify(ID, Answer::Body(body));
            assert_eq!((probe.verdict, probe.http_status, probe.detail.as_str()), (Verdict::Error, Some(200), "empty body"));
        }
        let empty = package("");
        let probe = classify(ID, Answer::Body(empty.as_bytes()));
        assert_eq!((probe.verdict, probe.http_status, probe.detail.as_str()), (Verdict::Absent, Some(200), "empty package"));

        // Present: the measured 2026-10-01 answer for 009911-2021 (one release,
        // BST offset), and its UK day.
        let one = package(&release(ID, "2021-05-07T06:35:29+01:00", "ocds-h6vhtk-02a1b2"));
        let probe = classify(ID, Answer::Body(one.as_bytes()));
        assert_eq!(probe.verdict, Verdict::Present);
        assert_eq!(probe.published.as_deref(), Some("2021-05-07T06:35:29+01:00"));
        assert_eq!(probe.published_day.as_deref(), Some("2021-05-07"));
        assert_eq!(probe.ocid.as_deref(), Some("ocds-h6vhtk-02a1b2"));
        assert_eq!(probe.releases, 1);

        // A fanned-out notice: the earliest date wins by instant, not by text
        // (00:30Z on the 8th is 01:30 BST on the 8th; 23:59+01:00 on the 7th is earlier).
        let fanned = package(&format!(
            "{},{},{}",
            release(ID, "2021-05-08T00:30:00Z", "ocds-h6vhtk-000002"),
            release(ID, "2021-05-07T23:59:00+01:00", "ocds-h6vhtk-000001"),
            release("009912-2021", "2021-05-01T00:00:00Z", "ocds-h6vhtk-000009"),
        ));
        let probe = classify(ID, Answer::Body(fanned.as_bytes()));
        assert_eq!(probe.verdict, Verdict::Present);
        assert_eq!(probe.releases, 2, "only the id's own releases count");
        assert_eq!(probe.ocid.as_deref(), Some("ocds-h6vhtk-000001"));
        assert_eq!(probe.published_day.as_deref(), Some("2021-05-07"));

        // A date that does not parse still makes the id present, with no day.
        let odd = package(&release(ID, "sometime", "ocds-h6vhtk-000001"));
        let probe = classify(ID, Answer::Body(odd.as_bytes()));
        assert_eq!((probe.verdict, probe.published_day), (Verdict::Present, None));

        // Error: releases of other ids only, a body that is not a package, a
        // package without `releases`, a 5xx, a 400, a throttle, a transport error.
        let others = package(&release("009912-2021", "2021-05-07T06:35:29Z", "ocds-h6vhtk-000009"));
        let probe = classify(ID, Answer::Body(others.as_bytes()));
        assert_eq!(probe.verdict, Verdict::Error);
        assert!(probe.detail.contains("other ids only (009912-2021)"), "{}", probe.detail);
        for answer in [
            Answer::Body(b"<html>"),
            Answer::Body(br#"{"version":"1.1"}"#),
            Answer::Status(500),
            Answer::Status(400),
            Answer::Failed { status: Some(429), what: "throttled: 429 (Retry-After: 120s)" },
            Answer::Failed { status: None, what: "http: connection reset" },
        ] {
            assert_eq!(classify(ID, answer).verdict, Verdict::Error, "{answer:?}");
        }
        assert_eq!(classify(ID, Answer::Status(503)).http_status, Some(503));
        assert_eq!(classify(ID, Answer::Failed { status: None, what: "x" }).http_status, None);
        for v in [Verdict::Absent, Verdict::Present, Verdict::Error, Verdict::Quarantined] {
            assert_eq!(Verdict::parse(v.as_str()), Some(v));
        }
    }

    /// Issue 477 unit 3: the denominator. `published = highest − absent`, a year
    /// is complete exactly when held + absent reaches the highest id, and the
    /// ledger rows of ids that are now held do not count twice.
    #[test]
    fn the_denominator_is_the_highest_id_less_the_absent_ones() {
        // 2021: 1..=10 issued, 3, 5, 7 and 8 missing. 2022: complete, 1..=4.
        let held = held_ids(
            ["000001-2021", "000002-2021", "000004-2021", "000006-2021", "000009-2021", "000010-2021", "000010-2021"]
                .into_iter()
                .chain(["000001-2022", "000002-2022", "000003-2022", "000004-2022", "junk", "ocds-x"]),
        );
        assert_eq!(missing_ids(&held), vec![(2021, 3), (2021, 5), (2021, 7), (2021, 8)]);
        let ledger: HashMap<String, Verdict> = [
            ("000003-2021", Verdict::Absent),
            ("000005-2021", Verdict::Present),
            ("000007-2021", Verdict::Error),
            // Held now (a refetch recovered it): ignored, never counted twice.
            ("000002-2021", Verdict::Present),
            // Above the year's highest: not a missing id.
            ("000011-2021", Verdict::Absent),
            // An archived, quarantined member: accounted for, still published.
            ("000008-2021", Verdict::Quarantined),
        ]
        .into_iter()
        .map(|(id, v)| (id.to_owned(), v))
        .collect();
        let years = census(&held, &ledger);
        assert_eq!(years.len(), 2);
        let y21 = &years[0];
        assert_eq!(
            (y21.year, y21.highest, y21.held, y21.absent, y21.present, y21.errors, y21.quarantined, y21.unchecked),
            (2021, 10, 6, 1, 1, 1, 1, 0)
        );
        assert_eq!(y21.missing(), 4, "the Verify count: highest − held");
        assert_eq!(y21.published(), 9, "highest − absent; a quarantined id was published");
        assert_eq!(y21.unaccounted(), 2, "present + error + unchecked, not the quarantined one");
        assert!(!y21.complete());
        assert_eq!(y21.ratio(), Some(6.0 / 9.0));
        let y22 = &years[1];
        assert_eq!((y22.highest, y22.held, y22.published()), (4, 4, 4));
        assert!(y22.complete(), "nothing missing is complete with no ledger row at all");

        // Every missing id shown absent: complete, and the denominator is what is held.
        let all_absent: HashMap<String, Verdict> =
            ["000003-2021", "000005-2021", "000007-2021", "000008-2021"].map(|id| (id.to_owned(), Verdict::Absent)).into();
        let y21 = &census(&held, &all_absent)[0];
        assert!(y21.complete());
        assert_eq!((y21.published(), y21.ratio()), (6, Some(1.0)));
    }

    /// Issue 477 unit 3: a re-run asks only what is not decided — never asked,
    /// or an error — and a closed year's absent id only when the caller asks
    /// for stale ones. The current year's absents go stale on their own after
    /// 30 days (an id can be published late).
    #[test]
    fn a_rerun_asks_only_unchecked_and_error_ids_and_stale_absents_on_request() {
        let missing = [(2021, 1), (2021, 2), (2021, 3), (2021, 4), (2021, 5), (2021, 6)];
        let ledger: HashMap<String, (Verdict, i64)> = [
            ("000002-2021", (Verdict::Absent, 100)),
            ("000003-2021", (Verdict::Present, 100)),
            ("000004-2021", (Verdict::Error, 100)),
            ("000005-2021", (Verdict::Absent, 900)),
            ("000006-2021", (Verdict::Quarantined, 100)),
        ]
        .into_iter()
        .map(|(id, v)| (id.to_owned(), v))
        .collect();
        assert_eq!(due(&missing, &ledger, 1_000, 2026, None), ["000001-2021", "000004-2021"]);
        assert_eq!(
            due(&missing, &ledger, 1_000, 2026, Some(500)),
            ["000001-2021", "000002-2021", "000004-2021"],
            "the absent asked 900 s ago is stale at 500, the one asked 100 s ago is not"
        );
        assert_eq!(
            due(&missing, &ledger, 1_000, 2026, Some(0)).len(),
            4,
            "a present or quarantined id is never re-asked"
        );

        // The current year: absents re-asked by default once 30 days old.
        let month = CURRENT_YEAR_ABSENT_RECHECK_SECS;
        let now = 500 + month;
        assert_eq!(
            due(&missing, &ledger, now, 2021, None),
            ["000001-2021", "000002-2021", "000004-2021"],
            "now is 30 days after 500: the absent asked at 100 is stale, the one asked at 900 is not yet"
        );
        assert_eq!(due(&missing, &ledger, now, 2021, Some(0)).len(), 4, "the explicit window wins when smaller");
        assert_eq!(due(&missing, &ledger, now, 2026, None), ["000001-2021", "000004-2021"], "a closed year: on request only");
    }

    #[test]
    fn an_archived_members_name_carries_its_publication_id() {
        assert_eq!(member_id("009911-2021.json"), Some("009911-2021"));
        assert_eq!(member_id("009911-2021~0a1b2c3d.json"), Some("009911-2021"));
        assert_eq!(member_id("x/009911-2021~0a1b2c3d.json"), Some("009911-2021"));
        for bad in ["009911-2021", "ocds-x.json", "9911-2021.json", ""] {
            assert_eq!(member_id(bad), None, "{bad}");
        }
    }

    /// Issue 477 unit 3: the refetch for a present id is its neighbours'
    /// packages plus its release day's package — the monthly while monthlies
    /// reach that month, else the daily — and the enqueue bodies are exact.
    #[test]
    fn a_present_id_names_its_neighbours_packages_and_its_days_and_the_exact_enqueue_bodies() {
        let neighbours = [
            ("monthly".to_owned(), "2021-05".to_owned()),
            ("monthly".to_owned(), "2021-05".to_owned()),
            ("rates".to_owned(), "x".to_owned()),
        ];
        assert_eq!(
            refetch_packages(Some((2021, 7, 27)), &neighbours, Some((2026, 8))),
            [("monthly".to_owned(), "2021-05".to_owned()), ("monthly".to_owned(), "2021-07".to_owned())],
            "009921-2021's shape: dated in July, listed among May's neighbours"
        );
        assert_eq!(
            refetch_packages(Some((2026, 9, 3)), &[("daily".into(), "2026-09-02".into())], Some((2026, 8))),
            [("daily".to_owned(), "2026-09-02".to_owned()), ("daily".to_owned(), "2026-09-03".to_owned())],
            "past the newest monthly the day's daily"
        );
        assert_eq!(refetch_packages(Some((2026, 9, 3)), &[], None), [("daily".to_owned(), "2026-09-03".to_owned())]);
        assert!(refetch_packages(None, &[], Some((2026, 8))).is_empty());

        let packages: BTreeSet<(String, String)> = [
            ("monthly".to_owned(), "2021-05".to_owned()),
            ("daily".to_owned(), "2026-09-03".to_owned()),
            ("monthly".to_owned(), "2021-07".to_owned()),
        ]
        .into();
        assert_eq!(
            enqueue_commands(&packages),
            [
                r#"{"kind":"fetch","package_kind":"daily","period":"2026-09-03","refetch":true,"source":"fts"}"#,
                r#"{"kind":"fetch","package_kind":"monthly","period":"2021-05","refetch":true,"source":"fts"}"#,
                r#"{"kind":"fetch","package_kind":"monthly","period":"2021-07","refetch":true,"source":"fts"}"#,
                r#"{"kind":"process","package_kind":"daily","source":"fts"}"#,
                r#"{"kind":"process","package_kind":"monthly","source":"fts"}"#,
                r#"{"kind":"project"}"#,
            ]
        );
        assert!(enqueue_commands(&BTreeSet::new()).is_empty());
    }
}
