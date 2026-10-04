//! Fetcher integration tests against a local HTTP fixture server — the
//! archive-immutability and idempotency invariants, no network involved.

use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use ingest::fetch::{fetch, probe_doe_daily, Outcome, Target};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Serves `/pkg` with swappable content; returns (base_url, content handle).
async fn fixture_server() -> (String, Arc<Mutex<Vec<u8>>>) {
    let content = Arc::new(Mutex::new(b"package-one".to_vec()));
    let served = content.clone();
    let app = axum::Router::new().route(
        "/pkg",
        get(move || {
            let body = served.lock().unwrap().clone();
            async move { body }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("http://{addr}"), content)
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tender-db-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn target(base: &str) -> Target {
    Target {
        source: "ted",
        kind: "daily",
        period: "2026-00137".into(),
        url: format!("{base}/pkg"),
        rel_path: "ted/daily/2026-00137.tar.gz".into(),
    }
}

#[tokio::test]
async fn fetch_is_idempotent_and_versions_changed_content() {
    let (base, content) = fixture_server().await;
    let archive = temp_dir("archive");
    let db_path = archive.join("test.db");
    let db = store::Db::open(db_path.to_str().unwrap()).await.unwrap();
    let client = reqwest::Client::new();
    let t = target(&base);

    // First fetch: file lands, registry row exists.
    assert_eq!(fetch(&db, &client, &archive, &t, false).await.unwrap(), Outcome::Fetched);
    let stored = archive.join("ted/daily/2026-00137.tar.gz");
    assert_eq!(std::fs::read(&stored).unwrap(), b"package-one");
    let row = db.latest_fetch("ted", "daily", "2026-00137").await.unwrap().unwrap();
    assert_eq!(row.bytes, 11);

    // Re-run without refetch: no download, no change.
    assert_eq!(fetch(&db, &client, &archive, &t, false).await.unwrap(), Outcome::Unchanged);

    // Refetch with identical upstream content: still unchanged, single row.
    assert_eq!(fetch(&db, &client, &archive, &t, true).await.unwrap(), Outcome::Unchanged);
    assert_eq!(db.latest_fetch("ted", "daily", "2026-00137").await.unwrap().unwrap(), row);

    // Upstream rewrites the package (pre-finality): new version, old file intact.
    *content.lock().unwrap() = b"package-two!".to_vec();
    assert_eq!(fetch(&db, &client, &archive, &t, true).await.unwrap(), Outcome::NewVersion);
    let row2 = db.latest_fetch("ted", "daily", "2026-00137").await.unwrap().unwrap();
    assert_eq!(row2.path, "ted/daily/2026-00137-v2.tar.gz");
    assert_eq!(std::fs::read(&stored).unwrap(), b"package-one");
    assert_eq!(std::fs::read(archive.join(&row2.path)).unwrap(), b"package-two!");

    let _ = std::fs::remove_dir_all(&archive);
}

/// Issue 451: `/pkg` behind a WAF shaped like TED's CloudFront since
/// 2026-09-30 — a 202 with an empty body and `x-amzn-waf-action: challenge`
/// for any User-Agent that is not Mozilla-compatible.
async fn waf_server() -> String {
    let app = axum::Router::new().route(
        "/pkg",
        get(|headers: axum::http::HeaderMap| async move {
            let ua = headers.get("user-agent").and_then(|v| v.to_str().ok()).unwrap_or("");
            if ua.starts_with("Mozilla/5.0") {
                (StatusCode::OK, b"package-one".to_vec()).into_response()
            } else {
                (StatusCode::ACCEPTED, [("x-amzn-waf-action", "challenge")], Vec::<u8>::new()).into_response()
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{addr}")
}

/// Issue 451: a client with no User-Agent (reqwest's default) is challenged,
/// and the job says so by name, registering nothing; the fetch client's
/// crawler-convention User-Agent gets the package.
#[tokio::test]
async fn a_waf_challenge_is_named_and_the_crawler_user_agent_passes_it() {
    let base = waf_server().await;
    let archive = temp_dir("waf");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    let t = target(&base);

    let err = fetch(&db, &reqwest::Client::new(), &archive, &t, false).await.expect_err("challenged");
    assert!(matches!(err, ingest::fetch::Error::Challenged { .. }), "{err}");
    assert!(err.to_string().contains("x-amzn-waf-action: challenge"), "{err}");
    assert!(db.latest_fetch("ted", "daily", "2026-00137").await.unwrap().is_none());

    assert!(ingest::fetch::USER_AGENT.starts_with("Mozilla/5.0 (compatible; tender-db/"));
    let client = reqwest::Client::builder().user_agent(ingest::fetch::USER_AGENT).build().unwrap();
    assert_eq!(fetch(&db, &client, &archive, &t, false).await.unwrap(), Outcome::Fetched);
    assert_eq!(std::fs::read(archive.join("ted/daily/2026-00137.tar.gz")).unwrap(), b"package-one");

    let _ = std::fs::remove_dir_all(&archive);
}

#[tokio::test]
async fn missing_package_reports_not_found() {
    let (base, _content) = fixture_server().await;
    let archive = temp_dir("archive-404");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    let client = reqwest::Client::new();

    let mut t = target(&base);
    t.url = format!("{base}/nope");
    assert_eq!(fetch(&db, &client, &archive, &t, false).await.unwrap(), Outcome::NotFound);
    assert!(db.latest_fetch("ted", "daily", "2026-00137").await.unwrap().is_none());

    let _ = std::fs::remove_dir_all(&archive);
}

/// Serves the DÖE day export endpoint: 200 with per-day content for every day in
/// `available`, 404 otherwise (mirrors the real `/api/notice-exports?pubDay=…`).
async fn doe_server(available: &[&str]) -> String {
    let days: Arc<HashSet<String>> = Arc::new(available.iter().map(|d| d.to_string()).collect());
    let app = axum::Router::new().route(
        "/api/notice-exports",
        get(move |Query(q): Query<HashMap<String, String>>| {
            let days = days.clone();
            async move {
                match q.get("pubDay") {
                    Some(day) if days.contains(day) => {
                        (StatusCode::OK, format!("doe-{day}")).into_response()
                    }
                    _ => StatusCode::NOT_FOUND.into_response(),
                }
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{addr}")
}

/// A skipped scheduler tick must not silently drop DÖE days: the walk-forward
/// catches up every missed day from the last watermark, then no-ops once current.
#[tokio::test]
async fn doe_walk_forward_catches_up_a_multi_day_gap() {
    let base = doe_server(&["2026-07-25", "2026-07-26", "2026-07-27", "2026-07-28"]).await;
    let archive = temp_dir("doe-walk");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    let client = reqwest::Client::new();

    // No DÖE daily on record yet: the walk fetches only `end` (a single day), never
    // a full-archive backfill. This seeds the watermark at 2026-07-25.
    let seeded = probe_doe_daily(&db, &client, &archive, &base, (2026, 7, 25), |_, _| {})
        .await
        .unwrap();
    assert_eq!(seeded, vec![("2026-07-25".into(), Outcome::Fetched)]);

    // Three ticks were then missed (26, 27, 28). One walk with end=2026-07-28 must
    // catch up all three — a normal run advances one day, a gap catches up the lot.
    let touched: Vec<String> = Vec::new();
    let touched = Arc::new(Mutex::new(touched));
    let t2 = touched.clone();
    let caught = probe_doe_daily(&db, &client, &archive, &base, (2026, 7, 28), |p, _| {
        t2.lock().unwrap().push(p.to_owned());
    })
    .await
    .unwrap();
    assert_eq!(
        caught,
        vec![
            ("2026-07-26".into(), Outcome::Fetched),
            ("2026-07-27".into(), Outcome::Fetched),
            ("2026-07-28".into(), Outcome::Fetched),
        ]
    );
    assert_eq!(*touched.lock().unwrap(), ["2026-07-26", "2026-07-27", "2026-07-28"]);
    // Every caught-up day is now durably registered.
    for day in ["2026-07-26", "2026-07-27", "2026-07-28"] {
        assert!(db.latest_fetch("doe", "daily", day).await.unwrap().is_some());
    }

    // Idempotent: re-running with the same end is a no-op (watermark is current).
    let again = probe_doe_daily(&db, &client, &archive, &base, (2026, 7, 28), |_, _| {})
        .await
        .unwrap();
    assert!(again.is_empty());

    let _ = std::fs::remove_dir_all(&archive);
}

/// The walk spans a month/year boundary (the civil-date round-trip handles the
/// rollover), fetching each consecutive calendar day exactly once.
#[tokio::test]
async fn doe_walk_forward_crosses_month_boundary() {
    let base = doe_server(&["2026-11-30", "2026-12-01", "2026-12-02"]).await;
    let archive = temp_dir("doe-walk-month");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    let client = reqwest::Client::new();

    probe_doe_daily(&db, &client, &archive, &base, (2026, 11, 30), |_, _| {})
        .await
        .unwrap();
    let crossed = probe_doe_daily(&db, &client, &archive, &base, (2026, 12, 2), |_, _| {})
        .await
        .unwrap();
    assert_eq!(
        crossed,
        vec![
            ("2026-12-01".into(), Outcome::Fetched),
            ("2026-12-02".into(), Outcome::Fetched),
        ]
    );

    let _ = std::fs::remove_dir_all(&archive);
}

// ------------------------------------------------------------------ FTS (issue 342)

use axum::http::header;
use ingest::fetch::{fetch_fts, probe_fts_daily, DenseTally};
use ingest::fts;
use serde_json::{json, Value};
use std::io::Read;
use std::sync::atomic::Ordering;
use std::time::Duration;

/// Trimmed real pages of 3 Sep 2026 (5 + 3 releases): a non-final page with a
/// `links.next`, and the day's final page without one.
const FTS_P1: &str = "tests/fixtures/fts/pages/2026-09-03-p001.json";
const FTS_P2: &str = "tests/fixtures/fts/pages/2026-09-03-p002.json";

fn fixture_page(path: &str) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

/// Requests seen per key (see each server for what a key is).
type Hits = Arc<Mutex<HashMap<String, usize>>>;

fn zip_members(path: &std::path::Path) -> Vec<(String, Vec<u8>)> {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
    (0..zip.len())
        .map(|i| {
            let mut entry = zip.by_index(i).unwrap();
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).unwrap();
            (entry.name().to_owned(), bytes)
        })
        .collect()
}

/// The ids in the two fixture pages, sorted.
const FTS_IDS: [&str; 8] = [
    "083253-2026", "083256-2026", "083257-2026", "083608-2026",
    "083655-2026", "083662-2026", "083664-2026", "083674-2026",
];

/// 3 September 2026 on the keyset server: the 8 real releases of the two
/// trimmed fixture pages, each at its own `date` (UK wall clock), plus 100
/// synthetic ones every ten minutes from 01:00 — 108 in all, so the day's first
/// page is full and the walk has to split it (into 63 before 11:00 and 45 after).
fn busy_2026_09_03() -> Vec<Keyed> {
    let mut rows = Vec::new();
    for page in [FTS_P1, FTS_P2] {
        for r in fixture_page(page)["releases"].as_array().unwrap() {
            let id = r["id"].as_str().unwrap().to_owned();
            let at = wall_secs(&r["date"].as_str().unwrap()[..19]).unwrap();
            rows.push(Keyed { key: 700_000 + id_order(&id).1 as i64, id, at, release: r.clone() });
        }
    }
    let one = ingest::fetch::days_from_civil(2026, 9, 3) * 86_400 + 3_600;
    rows.extend((1..=100).map(|n| keyed(&format!("{n:06}-2026"), one + (n - 1) * 600, 600_000 + n)));
    rows
}

/// One daily window, split once (issue 477): a throttled request honours the
/// server's Retry-After, the walk assembles ONE zip with one member per
/// release, registers it, and leaves no staging behind. A second call is
/// `Unchanged` without HTTP, and a `refetch` re-walk of unchanged pages hashes
/// equal — the assembled zip is byte-deterministic, so the registry never
/// versions a package that did not change.
#[tokio::test]
async fn a_daily_window_lands_one_zip_and_a_refetch_hashes_equal() {
    // The second request is answered 429 Retry-After: 1 once.
    let quirks = Quirks { throttle: Some((2, 1, 1)), ..Quirks::default() };
    let (base, log) = fts_keyset_server(busy_2026_09_03(), quirks).await;
    let archive = temp_dir("fts-window");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    let client = reqwest::Client::new();
    let t = fts::day(&base, (2026, 9, 3));
    assert!(t.url.starts_with(&format!("{base}/ocdsReleasePackages?")));

    // The plain fetcher refuses a paged source: no caller can archive a raw page
    // under the zip's name.
    assert!(matches!(
        fetch(&db, &client, &archive, &t, false).await,
        Err(ingest::fetch::Error::Unsupported(_))
    ));

    let mut progress = Vec::new();
    let started = std::time::Instant::now();
    let outcome = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, || false, |p| {
        assert_eq!(p.dense, DenseTally::default(), "no dense span here");
        progress.push((p.day.to_owned(), p.pages, p.releases));
    })
    .await
    .unwrap();
    let waited = started.elapsed();
    assert_eq!(outcome, Outcome::Fetched);
    // The day (full: no leaf releases yet), then its two short halves.
    assert_eq!(
        progress,
        [("2026-09-03".to_owned(), 1, 0), ("2026-09-03".to_owned(), 2, 63), ("2026-09-03".to_owned(), 3, 108)]
    );
    {
        let log = log.lock().unwrap();
        assert_eq!(log.len(), 4, "the day, the older half twice (the 429, then the page), the newer half");
        assert_eq!(log[1], log[2], "the throttled request is asked again as it was");
    }
    // The SERVER's Retry-After, not our own backoff. "It retried" is not the
    // property: dropping the header would leave the 15 s first-attempt fallback,
    // which is what this window discriminates (issue 342 review, lens "tests").
    assert!(
        waited >= Duration::from_secs(1) && waited < Duration::from_secs(10),
        "Retry-After: 1 honoured, not the 15 s fallback (waited {waited:?})"
    );

    // One zip, one member per release id, sorted, each a single-release package
    // under the page header minus the page-specific fields.
    let zip_path = archive.join("fts/daily/2026-09-03.zip");
    let members = zip_members(&zip_path);
    let names: Vec<&str> = members.iter().map(|(n, _)| n.as_str()).collect();
    let mut expected: Vec<String> = (1..=100).map(|n| format!("{n:06}-2026.json")).collect();
    expected.extend(FTS_IDS.iter().map(|id| format!("{id}.json")));
    assert_eq!(names, expected);
    for (name, bytes) in &members {
        let member: Value = serde_json::from_slice(bytes).unwrap();
        let releases = member["releases"].as_array().unwrap();
        assert_eq!(releases.len(), 1, "{name}");
        assert_eq!(format!("{}.json", releases[0]["id"].as_str().unwrap()), *name);
        assert_eq!(member["version"], "1.1");
        assert_eq!(member["publisher"]["name"], "Cabinet Office");
        assert!(member["license"].as_str().unwrap().contains("open-government-licence"));
        for dropped in ["uri", "links", "publishedDate"] {
            assert!(member.get(dropped).is_none(), "{name} carries page field {dropped}");
        }
    }
    // A real release's bytes are carried verbatim.
    let real = fixture_page(FTS_P1)["releases"][0].clone();
    let held: Value = serde_json::from_slice(&members.iter().find(|(n, _)| n == "083674-2026.json").unwrap().1).unwrap();
    assert_eq!(held["releases"][0], real);

    // Registered with the zip's own identity; staging gone.
    let row = db.latest_fetch("fts", "daily", "2026-09-03").await.unwrap().expect("registered");
    let data = std::fs::read(&zip_path).unwrap();
    assert_eq!(row.bytes, data.len() as i64);
    assert_eq!(row.sha256, ingest::sha256_hex(&data));
    assert_eq!(row.path, "fts/daily/2026-09-03.zip");
    assert_eq!(row.url, t.url);
    assert!(!archive.join("fts/daily/2026-09-03.pages").exists(), "staging removed after landing");
    assert!(!archive.join("fts/daily/2026-09-03.zip.part").exists());

    // Known period, no refetch: no HTTP at all.
    let again = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, || false, |_| {}).await.unwrap();
    assert_eq!(again, Outcome::Unchanged);
    assert_eq!(log.lock().unwrap().len(), 4);

    // Refetch of unchanged pages: re-walked, re-assembled, hashes equal — one row.
    let refetched = fetch_fts(&db, &client, &archive, &t, true, Duration::ZERO, || false, |_| {}).await.unwrap();
    assert_eq!(refetched, Outcome::Unchanged, "a deterministic zip never versions itself");
    assert_eq!(log.lock().unwrap().len(), 4 + 3, "the same three spans again");
    assert_eq!(db.latest_fetch("fts", "daily", "2026-09-03").await.unwrap().unwrap(), row);
    assert!(!archive.join("fts/daily/2026-09-03.pages").exists());
    assert!(!archive.join("fts/daily/2026-09-03-v2.zip").exists());
    assert_eq!(cursor_requests(&log), 0);

    let _ = std::fs::remove_dir_all(&archive);
}

/// The throttle EXHAUSTION arm (plan D8): five 429s and the walk gives up with
/// its staging intact, so the next tick resumes at the span that could not be
/// had rather than restarting the window. Pinned because the retry policy is now
/// shared with the plain fetcher — a change to the attempt count would otherwise
/// only show as a slower production walk (issue 342 review, lens "tests").
#[tokio::test]
async fn fts_gives_up_after_five_throttled_attempts_with_staging_intact() {
    // Retry-After: 0 — the policy under test is the attempt count, not the wait.
    let quirks = Quirks { throttle: Some((2, usize::MAX, 0)), ..Quirks::default() };
    let (base, log) = fts_keyset_server(busy_2026_09_03(), quirks).await;
    let archive = temp_dir("fts-throttled");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    let client = reqwest::Client::new();
    let t = fts::day(&base, (2026, 9, 3));

    let err = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, || false, |_| {}).await.unwrap_err();
    assert!(matches!(err, ingest::fetch::Error::Throttled { .. }), "{err}");
    let log_len = log.lock().unwrap().len();
    assert_eq!(log_len, 1 + 5, "the day, then five attempts at its older half, then it stops");

    let staging = archive.join("fts/daily/2026-09-03.pages");
    let staged: Vec<String> =
        std::fs::read_dir(&staging).unwrap().map(|e| e.unwrap().file_name().into_string().unwrap()).collect();
    assert_eq!(staged, ["20260902T220000-20260903T235959.json"], "the day's page stays staged for the resume");
    assert!(!archive.join("fts/daily/2026-09-03.zip").exists(), "nothing lands");
    assert!(db.latest_fetch("fts", "daily", "2026-09-03").await.unwrap().is_none(), "nothing registers");

    let _ = std::fs::remove_dir_all(&archive);
}

/// A release the publisher sent without a usable id is ARCHIVED under the
/// reserved `_noid/` prefix, not thrown: failing the package would be
/// deterministic, so that day would fail every tick and the watermark would
/// never advance past it (issue 342 review, lens "fetcher").
#[tokio::test]
async fn a_release_without_an_id_is_archived_rather_than_failing_the_day() {
    let nameless = json!({ "ocid": "ocds-h6vhtk-0aaaaa", "tender": { "title": "no id here" } });
    let days: HashMap<String, Vec<Value>> =
        HashMap::from([("2026-09-03".to_owned(), vec![release("083253-2026", "fine"), nameless])]);
    let (base, _hits) = fts_day_server(days).await;
    let archive = temp_dir("fts-noid");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    let client = reqwest::Client::new();
    let t = fts::day(&base, (2026, 9, 3));

    let outcome = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, || false, |_| {}).await.unwrap();
    assert_eq!(outcome, Outcome::Fetched, "the day lands despite the defective release");
    let names: Vec<String> =
        zip_members(&archive.join("fts/daily/2026-09-03.zip")).into_iter().map(|(n, _)| n).collect();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"083253-2026.json".to_owned()));
    assert!(
        names.iter().any(|n| n.starts_with("_noid/")),
        "the nameless release is archived under the reserved prefix, got {names:?}"
    );

    let _ = std::fs::remove_dir_all(&archive);
}

/// Staging left behind by an interrupted cleanup is DEBRIS, not a resume point:
/// a refetch must re-walk the spans its stale pages claim, or a package whose
/// content changed would be re-registered from the old pages (issue 342 review,
/// lens "fetcher"). A staged span page no newer than the registered landing is
/// debris.
#[tokio::test]
async fn a_refetch_discards_staging_older_than_the_registered_package() {
    let days: HashMap<String, Vec<Value>> =
        HashMap::from([("2026-09-03".to_owned(), vec![release("083253-2026", "fine")])]);
    let (base, hits) = fts_day_server(days).await;
    let archive = temp_dir("fts-debris");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    let client = reqwest::Client::new();
    let t = fts::day(&base, (2026, 9, 3));
    fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, || false, |_| {}).await.unwrap();
    let registered = db.latest_fetch("fts", "daily", "2026-09-03").await.unwrap().unwrap();

    // Debris: the day's span page holding a STALE release, stamped BEFORE the landing.
    let staging = archive.join("fts/daily/2026-09-03.pages");
    std::fs::create_dir_all(&staging).unwrap();
    let page = staging.join("20260902T220000-20260903T235959.json");
    let stale_page = json!({ "version": "1.1", "releases": [release("083253-2026", "stale")] });
    std::fs::write(&page, stale_page.to_string()).unwrap();
    let stale = std::time::UNIX_EPOCH + Duration::from_secs(registered.fetched_at as u64 - 3600);
    std::fs::File::options().write(true).open(&page).unwrap().set_modified(stale).unwrap();

    let before = hits.lock().unwrap().values().sum::<usize>();
    let outcome = fetch_fts(&db, &client, &archive, &t, true, Duration::ZERO, || false, |_| {}).await.unwrap();
    assert_eq!(hits.lock().unwrap().values().sum::<usize>(), before + 1, "the day was re-walked");
    assert_eq!(outcome, Outcome::Unchanged, "same bytes as the server's, so no new version");
    assert!(!staging.exists(), "staging cleaned up after the landing");

    let _ = std::fs::remove_dir_all(&archive);
}

/// The walk-forward is CAPPED per tick: a watermark left far behind cannot hold
/// the job runner for hours, and the remainder is the next tick's work
/// (issue 342 review, lens "ops").
#[tokio::test]
async fn the_walk_forward_is_capped_per_tick_and_resumes_next_tick() {
    let (base, _hits) = fts_day_server(HashMap::new()).await;
    let archive = temp_dir("fts-cap");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    let client = reqwest::Client::new();
    // One registered day sets the watermark; the end is 50 days later.
    fetch_fts(&db, &client, &archive, &fts::day(&base, (2026, 1, 1)), false, Duration::ZERO, || false, |_| {})
        .await
        .unwrap();

    let cap = ingest::fts::PROBE_DAY_CAP;
    assert!(cap < 50);
    let mut walked = 0;
    let mut ticks = 0;
    while ingest::fetch::latest_fts_day(&db).await.unwrap() != Some((2026, 2, 20)) {
        let tick = probe_fts_daily(&db, &client, &archive, &base, (2026, 2, 20), Duration::ZERO, || false, |_| {})
            .await
            .unwrap();
        if ticks == 0 {
            assert_eq!(tick.first().map(|(p, _)| p.as_str()), Some("2026-01-02"));
            assert_eq!(tick.len(), cap, "the cap, then it stops");
        }
        assert!(!tick.is_empty() && tick.len() <= cap, "{}", tick.len());
        walked += tick.len();
        ticks += 1;
    }
    assert_eq!(walked, 50, "every day once, over the ticks");
    assert_eq!(ticks, 50usize.div_ceil(cap), "the remainder is the next ticks' work");

    let _ = std::fs::remove_dir_all(&archive);
}

/// A page that is not a release package fails the walk with the staging intact
/// (nothing lands, nothing registers), so the next run resumes rather than
/// archiving garbage under the package's name. Here the day's page is full and
/// its halves answer garbage.
#[tokio::test]
async fn fts_malformed_page_fails_with_staging_intact() {
    let quirks = Quirks { malformed_under: Some(90_000), ..Quirks::default() };
    let (base, _log) = fts_keyset_server(busy_2026_09_03(), quirks).await;
    let archive = temp_dir("fts-malformed");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    let client = reqwest::Client::new();
    let t = fts::day(&base, (2026, 9, 3));

    let err = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, || false, |_| {}).await.unwrap_err();
    assert!(matches!(err, ingest::fetch::Error::Malformed(_)), "{err}");
    let staging = archive.join("fts/daily/2026-09-03.pages");
    let staged: Vec<String> =
        std::fs::read_dir(&staging).unwrap().map(|e| e.unwrap().file_name().into_string().unwrap()).collect();
    assert_eq!(staged, ["20260902T220000-20260903T235959.json"], "the day's page stays; the garbage is not staged");
    assert!(!archive.join("fts/daily/2026-09-03.zip").exists());
    assert!(db.latest_fetch("fts", "daily", "2026-09-03").await.unwrap().is_none());

    let _ = std::fs::remove_dir_all(&archive);
}

/// Serves one page per requested window, keyed by the `updatedTo` civil day:
/// the releases in `days` for that day (an absent day answers `releases: []`,
/// the real API's empty window). Records the `updatedFrom` seen per day.
/// Returns the API base (`http://…/api/1.0`), the shape `fts::BASE` has in
/// production.
async fn fts_day_server(days: HashMap<String, Vec<Value>>) -> (String, Hits) {
    let days = Arc::new(days);
    let hits: Hits = Arc::new(Mutex::new(HashMap::new()));
    let app = axum::Router::new().route("/api/1.0/ocdsReleasePackages", {
        let hits = hits.clone();
        get(move |Query(q): Query<HashMap<String, String>>| {
            let (days, hits) = (days.clone(), hits.clone());
            async move {
                let to = q.get("updatedTo").cloned().unwrap_or_default();
                let from = q.get("updatedFrom").cloned().unwrap_or_default();
                let day = to.get(..10).unwrap_or("").to_owned();
                *hits.lock().unwrap().entry(format!("{day} from {from}")).or_default() += 1;
                let releases = days.get(&day).cloned().unwrap_or_default();
                let page = json!({
                    "uri": format!("http://fts/api/1.0/ocdsReleasePackages?updatedFrom={from}&updatedTo={to}&limit=100"),
                    "version": "1.1",
                    "extensions": [],
                    "publishedDate": "",
                    "publisher": { "name": "Cabinet Office", "scheme": "GB-GOR", "uid": "D2" },
                    "license": "http://www.nationalarchives.gov.uk/doc/open-government-licence/version/3/",
                    "publicationPolicy": "https://www.gov.uk/government/publications/open-contracting",
                    "releases": releases,
                });
                (StatusCode::OK, page.to_string()).into_response()
            }
        })
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("http://{addr}/api/1.0"), hits)
}

fn release(id: &str, title: &str) -> Value {
    json!({ "id": id, "ocid": format!("ocds-h6vhtk-{id}"), "tag": ["tender"], "tender": { "title": title } })
}

/// The DÖE walk-forward shape for FTS: with nothing registered the probe fetches
/// `end`'s month up to `end`, never further back; a gap of missed ticks is
/// caught up day by day; an EMPTY window
/// lands a 0-member zip so the day is held; the daily window reaches
/// 2 h into the previous day; and within one package a re-served release is
/// one member.
#[tokio::test]
async fn fts_walk_forward_catches_up_a_multi_day_gap() {
    let days: HashMap<String, Vec<Value>> = HashMap::from([
        ("2026-09-03".to_owned(), vec![release("000003-2026", "third")]),
        // Day 4's window (from 2026-09-03T22:00:00) re-serves day 3's late notice,
        // and lists one release twice; a byte-identical repeat is one member.
        (
            "2026-09-04".to_owned(),
            vec![release("000004-2026", "fourth"), release("000004-2026", "fourth"), release("000003-2026", "third")],
        ),
        // 2026-09-05 is absent: a Saturday with no notices.
        ("2026-09-06".to_owned(), vec![release("000006-2026", "sixth")]),
    ]);
    let (base, hits) = fts_day_server(days).await;
    let archive = temp_dir("fts-walk");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    let client = reqwest::Client::new();

    // Nothing on record: day 1 of `end`'s month through `end` — the running
    // month is never left to a backfill that stops at the previous one (issue
    // 477's seam), and never a full-archive backfill through the probe.
    let seeded = probe_fts_daily(&db, &client, &archive, &base, (2026, 9, 3), Duration::ZERO, || false, |_| {})
        .await
        .unwrap();
    assert_eq!(
        seeded,
        vec![
            ("2026-09-01".into(), Outcome::Fetched),
            ("2026-09-02".into(), Outcome::Fetched),
            ("2026-09-03".into(), Outcome::Fetched),
        ]
    );

    // Three missed ticks (4, 5, 6): one walk catches up the lot, in order.
    let touched = Arc::new(Mutex::new(Vec::<String>::new()));
    let t2 = touched.clone();
    let caught = probe_fts_daily(&db, &client, &archive, &base, (2026, 9, 6), Duration::ZERO, || false, |p| {
        t2.lock().unwrap().push(p.day.to_owned());
    })
    .await
    .unwrap();
    assert_eq!(
        caught,
        vec![
            ("2026-09-04".into(), Outcome::Fetched),
            ("2026-09-05".into(), Outcome::Fetched),
            ("2026-09-06".into(), Outcome::Fetched),
        ]
    );
    assert_eq!(*touched.lock().unwrap(), ["2026-09-04", "2026-09-05", "2026-09-06"]);
    {
        // Each day asked once, from 22:00 the evening before (the 2 h overlap).
        let hits = hits.lock().unwrap();
        assert_eq!(hits.get("2026-09-01 from 2026-08-31T22:00:00"), Some(&1));
        assert_eq!(hits.get("2026-09-02 from 2026-09-01T22:00:00"), Some(&1));
        assert_eq!(hits.get("2026-09-03 from 2026-09-02T22:00:00"), Some(&1));
        assert_eq!(hits.get("2026-09-04 from 2026-09-03T22:00:00"), Some(&1));
        assert_eq!(hits.get("2026-09-05 from 2026-09-04T22:00:00"), Some(&1));
        assert_eq!(hits.get("2026-09-06 from 2026-09-05T22:00:00"), Some(&1));
        assert_eq!(hits.values().sum::<usize>(), 6, "one request per day, none repeated");
    }

    // Day 4: both ids, the overlap's re-served release included, the repeat once.
    let day4 = zip_members(&archive.join("fts/daily/2026-09-04.zip"));
    let names: Vec<&str> = day4.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["000003-2026.json", "000004-2026.json"]);
    let fourth: Value = serde_json::from_slice(&day4[1].1).unwrap();
    assert_eq!(fourth["releases"][0]["tender"]["title"], "fourth");
    // The re-served release is byte-identical to the day it was first archived:
    // that is what lets the overlap dedup on identity downstream.
    let day3 = zip_members(&archive.join("fts/daily/2026-09-03.zip"));
    assert_eq!(day3[0].1, day4[0].1, "same release, same bytes across days");

    // The empty Saturday: a valid 0-member zip, registered (so no longer a
    // gap), and walkable.
    let empty = archive.join("fts/daily/2026-09-05.zip");
    assert!(zip_members(&empty).is_empty());
    assert!(db.latest_fetch("fts", "daily", "2026-09-05").await.unwrap().is_some());
    let mut visited = 0;
    ingest::package::walk(&empty, |_| visited += 1).unwrap();
    assert_eq!(visited, 0);
    assert_eq!(ingest::fetch::latest_fts_day(&db).await.unwrap(), Some((2026, 9, 6)));
    for day in ["2026-09-03", "2026-09-04", "2026-09-05", "2026-09-06"] {
        assert!(!archive.join(format!("fts/daily/{day}.pages")).exists(), "{day} staging gone");
    }

    // Idempotent: every day through `end` is held, nothing to walk.
    let again = probe_fts_daily(&db, &client, &archive, &base, (2026, 9, 6), Duration::ZERO, || false, |_| {})
        .await
        .unwrap();
    assert!(again.is_empty());
    // The SUM, not the key count: a probe that re-requested the same six days
    // would leave six keys and twelve requests (issue 342 review, lens "tests").
    assert_eq!(
        hits.lock().unwrap().values().sum::<usize>(),
        6,
        "no HTTP when every day through `end` is held"
    );

    let _ = std::fs::remove_dir_all(&archive);
}

/// A monthly package is one contiguous 1-day window per civil day, no overlap,
/// assembled into one zip under the month's period.
#[tokio::test]
async fn fts_monthly_walks_every_civil_day_into_one_zip() {
    let days: HashMap<String, Vec<Value>> = HashMap::from([
        ("2021-02-01".to_owned(), vec![release("000101-2021", "first")]),
        ("2021-02-15".to_owned(), vec![release("000115-2021", "mid")]),
        ("2021-02-28".to_owned(), vec![release("000128-2021", "last")]),
    ]);
    let (base, hits) = fts_day_server(days).await;
    let archive = temp_dir("fts-monthly");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    let client = reqwest::Client::new();
    let t = fts::monthly(&base, (2021, 2));

    let mut days_seen = Vec::new();
    let outcome = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, || false, |p| {
        days_seen.push(p.day.to_owned());
    })
    .await
    .unwrap();
    assert_eq!(outcome, Outcome::Fetched);
    assert_eq!(days_seen.len(), 28);
    assert_eq!(days_seen.first().map(String::as_str), Some("2021-02-01"));
    assert_eq!(days_seen.last().map(String::as_str), Some("2021-02-28"));
    {
        let hits = hits.lock().unwrap();
        assert_eq!(hits.values().sum::<usize>(), 28, "one request per civil day, none repeated");
        assert_eq!(hits.get("2021-02-01 from 2021-02-01T00:00:00"), Some(&1), "no overlap in a monthly");
        assert_eq!(hits.get("2021-02-28 from 2021-02-28T00:00:00"), Some(&1));
    }
    let names: Vec<String> =
        zip_members(&archive.join("fts/monthly/2021-02.zip")).into_iter().map(|(n, _)| n).collect();
    assert_eq!(names, ["000101-2021.json", "000115-2021.json", "000128-2021.json"]);
    let row = db.latest_fetch("fts", "monthly", "2021-02").await.unwrap().expect("registered");
    assert_eq!(row.path, "fts/monthly/2021-02.zip");
    assert!(!archive.join("fts/monthly/2021-02.pages").exists());

    let _ = std::fs::remove_dir_all(&archive);
}

// ------------------------------------------------- FTS: the keyset server (issue 477)

/// One release as the measured FTS server holds it (issue 477,
/// `.scratch/tender-db/477-fts/`): its notice id, the HIDDEN publication instant
/// a window selects on (naive UK wall-clock seconds, the encoding
/// `fts::window_url` writes), and the HIDDEN per-release key its `links.next`
/// cursor continues on. The keys are not in id order — that is the defect.
#[derive(Clone)]
struct Keyed {
    id: String,
    at: i64,
    key: i64,
    release: Value,
}

fn keyed(id: &str, at: i64, key: i64) -> Keyed {
    Keyed { id: id.into(), at, key, release: release(id, &format!("release {id}")) }
}

/// What a keyset server does besides the measured paging.
#[derive(Clone, Default)]
struct Quirks {
    /// `(n, times, retry_after)`: from the `n`th request on (1-based), `times`
    /// requests are answered `429 Retry-After: <retry_after>`.
    throttle: Option<(usize, usize, u64)>,
    /// `(secs, serve)`: a span LONGER than `secs` answers only its newest
    /// `serve` rows and still names a `links.next` — a short page that is not
    /// the end of its window.
    truncate_over: Option<(i64, usize)>,
    /// A span SHORTER than this many seconds answers a body that is not a
    /// release package.
    malformed_under: Option<i64>,
    /// A FULL page names no `links.next`: only the row count says there is
    /// more (issue 477 review, lens "tests").
    full_without_next: bool,
    /// `(n, id)`: from the `n`th request on, release `id` is served with a
    /// changed title — a release re-published between two requests.
    mutate_from: Option<(usize, String)>,
}

/// `YYYY-MM-DDTHH:MM:SS` → naive wall-clock seconds, the inverse of the
/// encoding `fts::window_url` writes.
fn wall_secs(s: &str) -> Option<i64> {
    let (date, time) = s.split_once('T')?;
    let mut d = date.split('-').map(|p| p.parse::<i64>().ok());
    let mut t = time.split(':').map(|p| p.parse::<i64>().ok());
    let (y, m, day) = (d.next()??, d.next()??, d.next()??);
    let (h, mi, sec) = (t.next()??, t.next()??, t.next()??);
    let days = ingest::fetch::days_from_civil(y as u16, m as u8, day as u8);
    Some(days * 86_400 + h * 3_600 + mi * 60 + sec)
}

/// `NNNNNN-YYYY` → (year, number): the listing's sort key, newest first.
fn id_order(id: &str) -> (u32, u32) {
    let (n, y) = id.split_once('-').unwrap();
    (y.parse().unwrap(), n.parse().unwrap())
}

/// Every raw query string the server was asked, in order.
type Log = Arc<Mutex<Vec<String>>>;

/// A model of the measured FTS listing (issue 477's probes, every saved page
/// fits it): a window holds the rows whose hidden instant lies in
/// `updatedFrom..=updatedTo` (BOTH ends inclusive), sorted by notice id newest
/// first. A page is the first `limit` of them; with a `cursor`, only rows whose
/// key is at or below the cursor are eligible. `links.next` carries the key of
/// the first row NOT served. `updatedTo <= updatedFrom` is a 400, as the live
/// API answers a one-second window. Returns the API base and the request log.
async fn fts_keyset_server(rows: Vec<Keyed>, quirks: Quirks) -> (String, Log) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/api/1.0", listener.local_addr().unwrap());
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    let rows = Arc::new(rows);
    let throttled = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let app = axum::Router::new().route("/api/1.0/ocdsReleasePackages", {
        let (base, log) = (base.clone(), log.clone());
        get(move |axum::extract::RawQuery(raw): axum::extract::RawQuery, Query(q): Query<HashMap<String, String>>| {
            let (base, log, rows, quirks, throttled) =
                (base.clone(), log.clone(), rows.clone(), quirks.clone(), throttled.clone());
            async move {
                let n = {
                    let mut log = log.lock().unwrap();
                    log.push(raw.unwrap_or_default());
                    log.len()
                };
                if let Some((from_n, times, retry_after)) = quirks.throttle
                    && n >= from_n
                    && throttled.fetch_add(1, Ordering::SeqCst) < times
                {
                    return (
                        StatusCode::TOO_MANY_REQUESTS,
                        [(header::RETRY_AFTER, retry_after.to_string())],
                        "Rate limit of 12 exceeded. Please retry after 120 seconds.",
                    )
                        .into_response();
                }
                let from_s = q.get("updatedFrom").cloned().unwrap_or_default();
                let to_s = q.get("updatedTo").cloned().unwrap_or_default();
                let (Some(from), Some(to)) = (wall_secs(&from_s), wall_secs(&to_s)) else {
                    return (StatusCode::BAD_REQUEST, "bad window").into_response();
                };
                if to <= from {
                    return (StatusCode::BAD_REQUEST, "'updatedTo' must be later than 'updatedFrom'").into_response();
                }
                if quirks.malformed_under.is_some_and(|secs| to - from + 1 < secs) {
                    return (StatusCode::OK, json!({ "error": "not a package" }).to_string()).into_response();
                }
                let limit: usize = q.get("limit").and_then(|l| l.parse().ok()).unwrap_or(100);
                let mut window: Vec<&Keyed> = rows.iter().filter(|r| from <= r.at && r.at <= to).collect();
                window.sort_by(|a, b| id_order(&b.id).cmp(&id_order(&a.id)));
                if let Some(cursor) = q.get("cursor") {
                    let cursor: i64 = cursor.parse().unwrap();
                    window.retain(|r| r.key <= cursor);
                }
                let mut releases: Vec<Value> = window
                    .iter()
                    .take(limit)
                    .map(|r| match &quirks.mutate_from {
                        Some((from_n, id)) if n >= *from_n && r.id == *id => {
                            let mut changed = r.release.clone();
                            changed["tender"]["title"] = json!("changed");
                            changed
                        }
                        _ => r.release.clone(),
                    })
                    .collect();
                let mut next = window.get(limit).map(|r| r.key);
                if quirks.full_without_next && releases.len() == limit {
                    next = None;
                }
                if let Some((secs, serve)) = quirks.truncate_over
                    && to - from + 1 > secs
                {
                    releases.truncate(serve);
                    next = Some(-1); // a cursor no row's key is at or below
                }
                let mut page = json!({
                    "uri": format!("http://fts/api/1.0/ocdsReleasePackages?updatedFrom={from_s}&updatedTo={to_s}&limit={limit}"),
                    "version": "1.1",
                    "extensions": [],
                    "publishedDate": "",
                    "publisher": { "name": "Cabinet Office", "scheme": "GB-GOR", "uid": "D2" },
                    "license": "http://www.nationalarchives.gov.uk/doc/open-government-licence/version/3/",
                    "publicationPolicy": "https://www.gov.uk/government/publications/open-contracting",
                    "releases": releases,
                });
                if let Some(cursor) = next {
                    page["links"] = json!({ "next": format!(
                        "{base}/ocdsReleasePackages?limit={limit}&updatedFrom={from_s}&updatedTo={to_s}&cursor={cursor}"
                    ) });
                }
                (StatusCode::OK, [(header::CONTENT_TYPE, "application/json")], page.to_string()).into_response()
            }
        })
    });
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (base, log)
}

/// 2021-05-07 (issue 477): the measured 152 releases, 009911–010062, in the
/// hours 06, 08–18, 21 and 22 with the real hourly counts. The KEYS are a model,
/// not the measured bands: 009947, 009955 and 009962 (one process, `0292a9`)
/// sit near 261,858 as measured, and every other key is a monotone 600,000 + n
/// (the probes measured bands, 599,795–605,324 and below). That reproduces the
/// measured limit-100 cursor walk — the cursor after page 1 is 009962's key and
/// page 2 holds those three rows only — and nothing finer (a limit-10 walk of
/// this model serves 103 ids, where the probes measured 33).
fn day_2021_05_07() -> Vec<Keyed> {
    const HOURS: [(i64, i64); 14] =
        [(6, 1), (8, 2), (9, 11), (10, 14), (11, 6), (12, 17), (13, 9), (14, 16), (15, 20), (16, 25), (17, 7), (18, 7), (21, 1), (22, 16)];
    let midnight = ingest::fetch::days_from_civil(2021, 5, 7) * 86_400;
    let mut n = 9_911;
    let mut out = Vec::new();
    for (hour, count) in HOURS {
        for k in 0..count {
            let at = midnight + hour * 3_600 + k * 3_600 / count;
            let key = match n {
                9_947 => 261_856,
                9_955 => 261_857,
                9_962 => 261_858,
                _ => 600_000 + n,
            };
            out.push(keyed(&format!("{n:06}-2021"), at, key));
            n += 1;
        }
    }
    assert_eq!(out.len(), 152);
    out
}

/// The ids `2021-05.zip` holds for 2021-05-07: exactly what the cursor walk served.
fn archived_2021_05_07() -> Vec<String> {
    [9_947, 9_955, 9_962].into_iter().chain(9_963..=10_062).map(|n| format!("{n:06}-2021")).collect()
}

fn ids_2021_05_07() -> Vec<String> {
    (9_911..=10_062).map(|n| format!("{n:06}-2021")).collect()
}

/// Member names of a zip, minus `.json`.
fn member_ids(path: &std::path::Path) -> Vec<String> {
    zip_members(path).into_iter().map(|(n, _)| n.trim_end_matches(".json").to_owned()).collect()
}

/// `(updatedFrom, updatedTo)` of every logged request that carried no cursor.
fn spans_asked(log: &Log) -> Vec<(String, String)> {
    log.lock()
        .unwrap()
        .iter()
        .map(|raw| {
            let q: HashMap<String, String> = url_pairs(raw);
            (q.get("updatedFrom").cloned().unwrap_or_default(), q.get("updatedTo").cloned().unwrap_or_default())
        })
        .collect()
}

fn url_pairs(raw: &str) -> HashMap<String, String> {
    raw.split('&').filter_map(|kv| kv.split_once('=')).map(|(k, v)| (k.to_owned(), v.to_owned())).collect()
}

fn cursor_requests(log: &Log) -> usize {
    log.lock().unwrap().iter().filter(|raw| raw.contains("cursor=")).count()
}

/// THE ROOT CAUSE, reproduced (issue 477). Following `links.next` through the
/// 2021-05-07 window yields exactly the 103 ids the archive holds — page 2 comes
/// back with 3 rows and no next, a perfectly normal-looking last page — while the
/// same day asked as 24 cursorless hourly windows returns all 152. So the model
/// reproduces the measured limit-100 walk, and a walker that follows the cursor
/// loses 49.
#[tokio::test]
async fn the_keyset_mock_reproduces_the_2021_05_07_cursor_loss() {
    let (base, _log) = fts_keyset_server(day_2021_05_07(), Quirks::default()).await;
    let client = reqwest::Client::new();

    let mut next = Some(format!(
        "{base}/ocdsReleasePackages?limit=100&updatedFrom=2021-05-07T00:00:00&updatedTo=2021-05-07T23:59:59"
    ));
    let (mut sizes, mut ids) = (Vec::new(), Vec::new());
    while let Some(url) = next {
        let page: Value = client.get(&url).send().await.unwrap().json().await.unwrap();
        let releases = page["releases"].as_array().unwrap();
        sizes.push(releases.len());
        ids.extend(releases.iter().map(|r| r["id"].as_str().unwrap().to_owned()));
        next = page["links"]["next"].as_str().map(str::to_owned);
    }
    assert_eq!(sizes, [100, 3], "page 2 is short with no next: it reads as the last page");
    ids.sort();
    assert_eq!(ids, archived_2021_05_07(), "the cursor walk serves exactly the 103 archived ids");

    let mut hourly = Vec::new();
    for h in 0..24 {
        let url = format!(
            "{base}/ocdsReleasePackages?limit=100&updatedFrom=2021-05-07T{h:02}:00:00&updatedTo=2021-05-07T{h:02}:59:59"
        );
        let page: Value = client.get(&url).send().await.unwrap().json().await.unwrap();
        assert!(page["links"]["next"].is_null(), "every hour fits one cursorless page");
        hourly.extend(page["releases"].as_array().unwrap().iter().map(|r| r["id"].as_str().unwrap().to_owned()));
    }
    hourly.sort();
    assert_eq!(hourly, ids_2021_05_07(), "without a cursor the day is whole: 152 contiguous ids");
}

/// The fix (issue 477): a full cursorless page is SPLIT, never followed. The
/// 2021-05 monthly lands all 152 of 2021-05-07's releases — the 49 the cursor
/// lost included — with no `cursor=` request at all, every span asked once, and
/// the month's first URL (its registry identity) unchanged.
#[tokio::test]
async fn a_full_first_page_is_split_never_followed() {
    let (base, log) = fts_keyset_server(day_2021_05_07(), Quirks::default()).await;
    let archive = temp_dir("fts-split");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    let client = reqwest::Client::new();
    let t = fts::monthly(&base, (2021, 5));

    let outcome = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, || false, |_| {}).await.unwrap();
    assert_eq!(outcome, Outcome::Fetched);
    assert_eq!(member_ids(&archive.join("fts/monthly/2021-05.zip")), ids_2021_05_07(), "all 152, none lost");
    assert_eq!(cursor_requests(&log), 0, "links.next is never followed");

    let asked = spans_asked(&log);
    let distinct: HashSet<&(String, String)> = asked.iter().collect();
    assert_eq!(distinct.len(), asked.len(), "each span asked once: {asked:?}");
    // 30 one-page days, and 2021-05-07: its day (152, full), its halves at
    // 12:00 (34, short; 118, full), and the afternoon's halves at 18:00 (94; 24).
    let day7: Vec<(&str, &str)> =
        asked.iter().filter(|(f, _)| f.starts_with("2021-05-07")).map(|(f, t)| (f.as_str(), t.as_str())).collect();
    assert_eq!(
        day7,
        [
            ("2021-05-07T00:00:00", "2021-05-07T23:59:59"),
            ("2021-05-07T00:00:00", "2021-05-07T11:59:59"),
            ("2021-05-07T12:00:00", "2021-05-07T23:59:59"),
            ("2021-05-07T12:00:00", "2021-05-07T17:59:59"),
            ("2021-05-07T18:00:00", "2021-05-07T23:59:59"),
        ]
    );
    assert_eq!(asked.len(), 31 + 4);
    // The first request is the month's first window URL byte for byte, and the
    // registry row keeps it: no registry URL changes under the new walk.
    assert_eq!(format!("{base}/ocdsReleasePackages?{}", log.lock().unwrap()[0]), t.url);
    assert_eq!(db.latest_fetch("fts", "monthly", "2021-05").await.unwrap().unwrap().url, t.url);

    let _ = std::fs::remove_dir_all(&archive);
}

/// 2025-12-10, issue 449's day: 150 releases whose 101st row carries the
/// highest key of all, so page 2 (cursor 578,232) is page 1 again and names
/// itself as next — the measured stuck cursor (issue 477's challenge replayed it).
fn day_2025_12_10() -> Vec<Keyed> {
    let midnight = ingest::fetch::days_from_civil(2025, 12, 10) * 86_400;
    (81_579..=81_728)
        .enumerate()
        .map(|(i, n)| {
            let i = i as i64;
            let at = midnight + 8 * 3_600 + (i / 15) * 3_600 + (i % 15) * 240;
            let key = if n == 81_628 { 578_232 } else { 578_000 + (n - 81_579) };
            keyed(&format!("{n:06}-2025"), at, key)
        })
        .collect()
}

/// Issue 449's stuck cursor is the same defect, and the split walk needs no
/// fallback for it: the full first page is split like any other, and the day
/// lands whole without one `cursor=` request. (The 449 hourly fallback is gone.)
#[tokio::test]
async fn the_449_stuck_shape_lands_without_a_fallback() {
    let (base, log) = fts_keyset_server(day_2025_12_10(), Quirks::default()).await;
    let client = reqwest::Client::new();

    // The premise: this server IS the stuck shape — page 2 repeats page 1 and
    // its next is its own URL.
    let first = format!("{base}/ocdsReleasePackages?limit=100&updatedFrom=2025-12-10T00:00:00&updatedTo=2025-12-10T23:59:59");
    let p1: Value = client.get(&first).send().await.unwrap().json().await.unwrap();
    let next = p1["links"]["next"].as_str().unwrap().to_owned();
    assert!(next.ends_with("cursor=578232"), "{next}");
    let p2: Value = client.get(&next).send().await.unwrap().json().await.unwrap();
    assert_eq!(p2["releases"], p1["releases"], "page 2 is page 1 again");
    assert_eq!(p2["links"]["next"].as_str(), Some(next.as_str()), "and names itself as next");
    log.lock().unwrap().clear();

    let archive = temp_dir("fts-449");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    let t = fts::day(&base, (2025, 12, 10));
    let outcome = tokio::time::timeout(
        Duration::from_secs(30),
        fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, || false, |_| {}),
    )
    .await
    .expect("never loops")
    .expect("the day lands");
    assert_eq!(outcome, Outcome::Fetched);
    let expected: Vec<String> = (81_579..=81_728).map(|n| format!("{n:06}-2025")).collect();
    assert_eq!(member_ids(&archive.join(&t.rel_path)), expected);
    assert_eq!(cursor_requests(&log), 0, "the stuck cursor is never asked");
    // The day (150, full), its halves at 11:00 (45, short; 105, full), and the
    // later half's halves at 17:30 (98; 7). No hour-by-hour re-walk.
    let asked = spans_asked(&log);
    let asked: Vec<(&str, &str)> = asked.iter().map(|(f, t)| (f.as_str(), t.as_str())).collect();
    assert_eq!(
        asked,
        [
            ("2025-12-09T22:00:00", "2025-12-10T23:59:59"),
            ("2025-12-09T22:00:00", "2025-12-10T10:59:59"),
            ("2025-12-10T11:00:00", "2025-12-10T23:59:59"),
            ("2025-12-10T11:00:00", "2025-12-10T17:29:59"),
            ("2025-12-10T17:30:00", "2025-12-10T23:59:59"),
        ]
    );

    let _ = std::fs::remove_dir_all(&archive);
}

/// The API answers 400 to a one-second window (`'updatedTo' must be later than
/// 'updatedFrom'`, measured), so the walk never asks one. A span still full at
/// two seconds whose page holds SEVERAL notices (here 100 ids) fails LOUD as
/// malformed, with its staging intact for a person to look at, and nothing
/// lands: a notice below its 100 rows is out of every request's reach. (One
/// notice's fan-out is completed from records: the `dense_` tests below.)
#[tokio::test]
async fn a_full_two_second_span_fails_loud_with_staging_intact() {
    let ten = ingest::fetch::days_from_civil(2026, 9, 3) * 86_400 + 10 * 3_600;
    let rows: Vec<Keyed> = (1..=120).map(|n| keyed(&format!("{n:06}-2026"), ten, 500_000 + n)).collect();
    let (base, log) = fts_keyset_server(rows, Quirks::default()).await;
    let archive = temp_dir("fts-two-seconds");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    let client = reqwest::Client::new();
    let t = fts::day(&base, (2026, 9, 3));

    let err = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, || false, |_| {}).await.unwrap_err();
    assert!(matches!(err, ingest::fetch::Error::Malformed(_)), "{err}");
    assert!(err.to_string().contains("issue 477"), "{err}");
    let widths: Vec<i64> = spans_asked(&log)
        .iter()
        .map(|(f, t)| wall_secs(t).unwrap() - wall_secs(f).unwrap() + 1)
        .collect();
    assert_eq!(widths.iter().min(), Some(&2), "down to two seconds and never one: {widths:?}");
    assert_eq!(cursor_requests(&log), 0);
    let staging = archive.join("fts/daily/2026-09-03.pages");
    assert!(std::fs::read_dir(&staging).unwrap().count() >= 2, "the walked spans stay staged");
    assert!(!archive.join("fts/daily/2026-09-03.zip").exists(), "nothing lands");
    assert!(db.latest_fetch("fts", "daily", "2026-09-03").await.unwrap().is_none(), "nothing registers");

    let _ = std::fs::remove_dir_all(&archive);
}

/// A short page is complete only when it ALSO names no next. One that is short
/// and still names a next is not trusted: the span is split, and its halves
/// land what the short page left out — without following the next.
#[tokio::test]
async fn a_short_page_that_still_names_a_next_is_split() {
    let midnight = ingest::fetch::days_from_civil(2026, 9, 3) * 86_400;
    let rows: Vec<Keyed> = [3, 9, 13, 15, 20]
        .into_iter()
        .enumerate()
        .map(|(i, h)| keyed(&format!("{:06}-2026", i + 1), midnight + h * 3_600, 900_000 + i as i64))
        .collect();
    // The daily window (26 h) answers its newest 3 and a next; a half does not.
    let quirks = Quirks { truncate_over: Some((86_400, 3)), ..Quirks::default() };
    let (base, log) = fts_keyset_server(rows, quirks).await;
    let archive = temp_dir("fts-short-next");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    let client = reqwest::Client::new();
    let t = fts::day(&base, (2026, 9, 3));

    let outcome = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, || false, |_| {}).await.unwrap();
    assert_eq!(outcome, Outcome::Fetched);
    assert_eq!(member_ids(&archive.join(&t.rel_path)), ["000001-2026", "000002-2026", "000003-2026", "000004-2026", "000005-2026"]);
    assert_eq!(cursor_requests(&log), 0);
    assert_eq!(spans_asked(&log).len(), 3, "the day, then its two halves");

    let _ = std::fs::remove_dir_all(&archive);
}

/// The staged span pages are the resume state (issue 450's stop, issue 477's
/// walk). A walk stopped after its `k`th request lands nothing; the next fetch
/// of the same target never asks a staged span again, walks the rest, and
/// lands everything. Every stop point of 2021-05-07's five-request walk, and a
/// monthly stopped mid-month: from the second request on, a SHORT leaf page is
/// among the staged ones (05-06 22:00 – 05-07 10:59:59, 28 rows), and a resume
/// that asked only for what it lacked but assembled only what it asked would
/// land 124 of 152 (issue 477 review, lens "tests").
#[tokio::test]
async fn a_stopped_split_walk_resumes_without_asking_a_staged_span_again() {
    let runs = [(1, 5, false), (2, 5, false), (3, 5, false), (4, 5, false), (8, 35, true), (20, 35, true)];
    for (stop_after, total, monthly) in runs {
        let (base, log) = fts_keyset_server(day_2021_05_07(), Quirks::default()).await;
        let archive = temp_dir(&format!("fts-split-resume-{stop_after}"));
        let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
        let client = reqwest::Client::new();
        let t = if monthly { fts::monthly(&base, (2021, 5)) } else { fts::day(&base, (2021, 5, 7)) };
        let case = format!("{} stopped after {stop_after}", t.period);

        let asked = std::sync::atomic::AtomicUsize::new(0);
        let stop = || asked.fetch_add(1, Ordering::SeqCst) >= stop_after;
        let outcome = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, stop, |_| {}).await.unwrap();
        assert_eq!(outcome, Outcome::Stopped, "{case}");
        assert_eq!(log.lock().unwrap().len(), stop_after, "{case}: the requests, then the stop");
        let staging = archive.join(t.rel_path.trim_end_matches(".zip").to_owned() + ".pages");
        assert_eq!(std::fs::read_dir(&staging).unwrap().count(), stop_after, "{case}: every page asked stays staged");
        assert!(!archive.join(&t.rel_path).exists(), "{case}: nothing lands");
        assert!(db.latest_fetch("fts", t.kind, &t.period).await.unwrap().is_none(), "{case}: nothing registers");

        let outcome = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, || false, |_| {}).await.unwrap();
        assert_eq!(outcome, Outcome::Fetched, "{case}");
        let asked = spans_asked(&log);
        assert_eq!(asked.iter().collect::<HashSet<_>>().len(), asked.len(), "{case}: no span asked twice: {asked:?}");
        assert_eq!(asked.len(), total, "{case}: the resume asked only the rest");
        assert_eq!(cursor_requests(&log), 0);
        assert_eq!(member_ids(&archive.join(&t.rel_path)), ids_2021_05_07(), "{case}: all 152");
        assert!(!staging.exists(), "{case}: staging removed after landing");

        let _ = std::fs::remove_dir_all(&archive);
    }
}

/// A staged page that no longer parses — a power loss after the rename leaves
/// it truncated or empty — is storage damage, since a page is staged only once
/// it has parsed. It is discarded and its span asked again, rather than
/// failing every retry at the same file without a request (issue 477 review).
#[tokio::test]
async fn a_damaged_staged_page_is_discarded_and_asked_again() {
    let (base, log) = fts_keyset_server(day_2021_05_07(), Quirks::default()).await;
    let archive = temp_dir("fts-damaged-staging");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    let client = reqwest::Client::new();
    let t = fts::day(&base, (2021, 5, 7));
    let day = fts::windows(&t)[0].span;
    let (older, _) = fts::split(day).unwrap();
    let staging = archive.join("fts/daily/2021-05-07.pages");
    std::fs::create_dir_all(&staging).unwrap();
    std::fs::write(staging.join(format!("{}.json", fts::span_key(day))), b"{\"version\":\"1.1\",\"releases\":[{\"id\":").unwrap();
    std::fs::write(staging.join(format!("{}.json", fts::span_key(older))), b"").unwrap();

    let outcome = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, || false, |_| {}).await.unwrap();
    assert_eq!(outcome, Outcome::Fetched);
    assert_eq!(member_ids(&archive.join(&t.rel_path)), ids_2021_05_07());
    let asked = spans_asked(&log);
    assert_eq!(asked.len(), 5, "both damaged spans asked again, the walk otherwise as fresh: {asked:?}");
    assert_eq!(asked.iter().collect::<HashSet<_>>().len(), asked.len());

    let _ = std::fs::remove_dir_all(&archive);
}

/// A full page that names no next is still split: the row count alone says
/// the span may hold more (`page_is_short` needs BOTH fewer than 100 rows and
/// no next). A walk that trusted a missing next — the cursor walker's own end
/// rule — would land 100 of 152 (issue 477 review, lens "tests").
#[tokio::test]
async fn a_full_page_without_a_next_is_split() {
    let quirks = Quirks { full_without_next: true, ..Quirks::default() };
    let (base, log) = fts_keyset_server(day_2021_05_07(), quirks).await;
    let archive = temp_dir("fts-full-no-next");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    let client = reqwest::Client::new();
    let t = fts::day(&base, (2021, 5, 7));

    let outcome = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, || false, |_| {}).await.unwrap();
    assert_eq!(outcome, Outcome::Fetched);
    assert_eq!(member_ids(&archive.join(&t.rel_path)), ids_2021_05_07(), "all 152");
    let asked = spans_asked(&log);
    let asked: Vec<(&str, &str)> = asked.iter().map(|(f, t)| (f.as_str(), t.as_str())).collect();
    assert_eq!(
        asked,
        [
            ("2021-05-06T22:00:00", "2021-05-07T23:59:59"),
            ("2021-05-06T22:00:00", "2021-05-07T10:59:59"),
            ("2021-05-07T11:00:00", "2021-05-07T23:59:59"),
            ("2021-05-07T11:00:00", "2021-05-07T17:29:59"),
            ("2021-05-07T17:30:00", "2021-05-07T23:59:59"),
        ],
        "split exactly as when the next is named"
    );

    let _ = std::fs::remove_dir_all(&archive);
}

/// Only LEAF pages are assembled, never a full page that was split. A release
/// re-published between the full page and its half's request (12 s later in
/// production) is archived once, as the leaf served it — not as a spurious
/// second release of its id (issue 477 review, lens "tests").
#[tokio::test]
async fn only_leaf_pages_are_assembled_never_a_split_full_page() {
    // 010000-2021 sits in 15:00–15:59, so in the full day page (request 1) and
    // the full 11:00–23:59:59 half (3), and lands from the 11:00–17:29:59 leaf (4).
    let quirks = Quirks { mutate_from: Some((2, "010000-2021".to_owned())), ..Quirks::default() };
    let (base, log) = fts_keyset_server(day_2021_05_07(), quirks).await;
    let archive = temp_dir("fts-leaves-only");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    let client = reqwest::Client::new();
    let t = fts::day(&base, (2021, 5, 7));

    fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, || false, |_| {}).await.unwrap();
    assert_eq!(log.lock().unwrap().len(), 5);
    let members = zip_members(&archive.join(&t.rel_path));
    let of_id: Vec<&(String, Vec<u8>)> = members.iter().filter(|(n, _)| n.starts_with("010000-2021")).collect();
    assert_eq!(of_id.len(), 1, "one member, no `~` copy from the split full page: {:?}", of_id.iter().map(|m| &m.0).collect::<Vec<_>>());
    let leaf: Value = serde_json::from_slice(&of_id[0].1).unwrap();
    assert_eq!(leaf["releases"][0]["tender"]["title"], "changed", "as the leaf served it");
    assert_eq!(members.len(), 152);

    let _ = std::fs::remove_dir_all(&archive);
}

/// Staging the OLD cursor walker left (`<day>-pNNN.json`, `<day>-hNN-pNNN.json`,
/// `cursor.json`) is discarded, not resumed and not assembled: those pages are
/// exactly what lost releases, and its cursor is never asked.
#[tokio::test]
async fn staging_from_the_cursor_walker_is_discarded_not_assembled() {
    let midnight = ingest::fetch::days_from_civil(2026, 9, 3) * 86_400;
    let rows = vec![keyed("000001-2026", midnight + 9 * 3_600, 1), keyed("000002-2026", midnight + 15 * 3_600, 2)];
    let (base, log) = fts_keyset_server(rows, Quirks::default()).await;
    let archive = temp_dir("fts-old-staging");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    let client = reqwest::Client::new();
    let t = fts::day(&base, (2026, 9, 3));

    let staging = archive.join("fts/daily/2026-09-03.pages");
    std::fs::create_dir_all(&staging).unwrap();
    let old_page = json!({ "version": "1.1", "releases": [release("099999-2026", "from the cursor walk")] });
    std::fs::write(staging.join("2026-09-03-p001.json"), old_page.to_string()).unwrap();
    std::fs::write(staging.join("2026-09-03-h03-p001.json"), old_page.to_string()).unwrap();
    let next = format!("{base}/ocdsReleasePackages?limit=100&updatedFrom=2026-09-02T22:00:00&updatedTo=2026-09-03T23:59:59&cursor=5");
    std::fs::write(staging.join("cursor.json"), json!({ "day": "2026-09-03", "page": 1, "next": next, "done": [] }).to_string())
        .unwrap();

    let outcome = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, || false, |_| {}).await.unwrap();
    assert_eq!(outcome, Outcome::Fetched);
    assert_eq!(member_ids(&archive.join(&t.rel_path)), ["000001-2026", "000002-2026"], "the old pages are not assembled");
    assert_eq!(cursor_requests(&log), 0, "the old cursor is not resumed");
    assert_eq!(spans_asked(&log).len(), 1, "the day walked afresh");
    assert!(!staging.exists());

    let _ = std::fs::remove_dir_all(&archive);
}

fn registry_row(source: &str, kind: &str, period: &str) -> store::Fetch {
    store::Fetch {
        source: source.into(),
        kind: kind.into(),
        period: period.into(),
        url: format!("archive://{source}/{kind}/{period}.zip"),
        sha256: "0".repeat(64),
        bytes: 22,
        fetched_at: 1,
        path: format!("{source}/{kind}/{period}.zip"),
    }
}

/// THE SEAM (issue 477): the monthly backfill ended at 2026-08 and the first
/// daily was 2026-09-07, because with no daily on record the walk-forward
/// fetched only `end`, and 2026-09-01..06 (1,745 ids) were never fetched. The
/// walk is now a GAP walk: every day after the newest monthly's last day that
/// holds no daily, whichever of the two landed first (issue 477 review: a
/// high-water mark reopened the seam when the daily landed before the
/// backfill, which is how 477 actually happened).
#[tokio::test]
async fn the_walk_forward_starts_after_the_newest_monthly_not_at_end() {
    let (base, _log) = fts_keyset_server(Vec::new(), Quirks::default()).await;
    let client = reqwest::Client::new();
    let days = |r: Vec<(String, Outcome)>| r.into_iter().map(|(p, _)| p).collect::<Vec<_>>();

    // A monthly through 2026-08 and no daily: September from its first day.
    let archive = temp_dir("fts-seam");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    db.record_fetch(&registry_row("fts", "monthly", "2026-08")).await.unwrap();
    let walked = probe_fts_daily(&db, &client, &archive, &base, (2026, 9, 6), Duration::ZERO, || false, |_| {}).await.unwrap();
    assert_eq!(days(walked), ["2026-09-01", "2026-09-02", "2026-09-03", "2026-09-04", "2026-09-05", "2026-09-06"]);
    let _ = std::fs::remove_dir_all(&archive);

    // A daily OLDER than the newest monthly: the monthly's end wins.
    let archive = temp_dir("fts-seam-older-daily");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    db.record_fetch(&registry_row("fts", "daily", "2026-07-15")).await.unwrap();
    db.record_fetch(&registry_row("fts", "monthly", "2026-08")).await.unwrap();
    let walked = probe_fts_daily(&db, &client, &archive, &base, (2026, 9, 2), Duration::ZERO, || false, |_| {}).await.unwrap();
    assert_eq!(days(walked), ["2026-09-01", "2026-09-02"]);
    let _ = std::fs::remove_dir_all(&archive);

    // Dailies held through 2026-09-10 after the monthly: the days after them.
    let archive = temp_dir("fts-seam-newer-daily");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    db.record_fetch(&registry_row("fts", "monthly", "2026-08")).await.unwrap();
    for d in 1..=10 {
        db.record_fetch(&registry_row("fts", "daily", &format!("2026-09-{d:02}"))).await.unwrap();
    }
    let walked = probe_fts_daily(&db, &client, &archive, &base, (2026, 9, 12), Duration::ZERO, || false, |_| {}).await.unwrap();
    assert_eq!(days(walked), ["2026-09-11", "2026-09-12"]);
    let _ = std::fs::remove_dir_all(&archive);

    // 477 AS IT HAPPENED: the dailies landed first (2026-09-07..09, the old
    // walk's `end`-only seeding), the backfill through 2026-08 weeks later.
    // The days below the newest daily are still walked, and only those.
    let archive = temp_dir("fts-seam-daily-first");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    for d in 7..=9 {
        db.record_fetch(&registry_row("fts", "daily", &format!("2026-09-{d:02}"))).await.unwrap();
    }
    db.record_fetch(&registry_row("fts", "monthly", "2026-08")).await.unwrap();
    let walked = probe_fts_daily(&db, &client, &archive, &base, (2026, 9, 10), Duration::ZERO, || false, |_| {}).await.unwrap();
    assert_eq!(
        days(walked),
        ["2026-09-01", "2026-09-02", "2026-09-03", "2026-09-04", "2026-09-05", "2026-09-06", "2026-09-10"]
    );
    let _ = std::fs::remove_dir_all(&archive);

    // The same order from an EMPTY registry, tick by tick (the review's
    // replay): the first tick walks `end`'s month, so no day is ever left out.
    let archive = temp_dir("fts-seam-empty-first");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    let tick1 = probe_fts_daily(&db, &client, &archive, &base, (2026, 9, 7), Duration::ZERO, || false, |_| {}).await.unwrap();
    assert_eq!(days(tick1).first().map(String::as_str), Some("2026-09-01"));
    db.record_fetch(&registry_row("fts", "monthly", "2026-08")).await.unwrap();
    let tick2 = probe_fts_daily(&db, &client, &archive, &base, (2026, 9, 8), Duration::ZERO, || false, |_| {}).await.unwrap();
    assert_eq!(days(tick2), ["2026-09-08"]);
    for d in 1..=8 {
        let period = format!("2026-09-{d:02}");
        assert!(db.latest_fetch("fts", "daily", &period).await.unwrap().is_some(), "{period} fetched");
    }
    let _ = std::fs::remove_dir_all(&archive);

    // A daily row for a day past `end` (a future day landed by hand before
    // `fetch_fts` refused one) moves nothing: the walk reads gaps, not MAX.
    for monthly in [true, false] {
        let archive = temp_dir(&format!("fts-seam-future-row-{monthly}"));
        let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
        if monthly {
            db.record_fetch(&registry_row("fts", "monthly", "2026-08")).await.unwrap();
        }
        db.record_fetch(&registry_row("fts", "daily", "2026-12-25")).await.unwrap();
        let walked = probe_fts_daily(&db, &client, &archive, &base, (2026, 9, 3), Duration::ZERO, || false, |_| {}).await.unwrap();
        assert_eq!(days(walked), ["2026-09-01", "2026-09-02", "2026-09-03"], "monthly on record: {monthly}");
        let _ = std::fs::remove_dir_all(&archive);
    }
}

/// A monthly is registered once and never re-walked, so a monthly of a month
/// whose last UK day has not ended would freeze a partial month (issue 477).
/// It is refused before any request; its days come from the daily walk.
#[tokio::test]
async fn a_monthly_for_an_unfinished_month_is_refused() {
    let (base, log) = fts_keyset_server(Vec::new(), Quirks::default()).await;
    let archive = temp_dir("fts-unfinished");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    let client = reqwest::Client::new();
    let (y, m, _) = fts::uk_civil_date(store::now_unix());
    let t = fts::monthly(&base, (y, m));

    let err = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, || false, |_| {}).await.unwrap_err();
    assert!(matches!(err, ingest::fetch::Error::Unsupported(_)), "{err}");
    assert!(log.lock().unwrap().is_empty(), "refused before any request");
    assert!(!archive.join(format!("fts/monthly/{y:04}-{m:02}.pages")).exists(), "nothing staged");
    assert!(db.latest_fetch("fts", "monthly", &t.period).await.unwrap().is_none());

    let _ = std::fs::remove_dir_all(&archive);
}

/// A daily is registered once and the walk-forward never revisits a held day,
/// so a daily of the running UK day would freeze it part-walked (only its
/// 22:00–23:59 tail would come back, through the next day's overlap), and one
/// of a future day would land empty. Both are refused before any request, the
/// rule the monthly already had (issue 477 review); yesterday is accepted.
#[tokio::test]
async fn a_daily_whose_day_has_not_ended_is_refused() {
    let (base, log) = fts_keyset_server(Vec::new(), Quirks::default()).await;
    let archive = temp_dir("fts-unfinished-day");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    let client = reqwest::Client::new();
    let now = store::now_unix();
    let today = fts::uk_civil_date(now);

    for day in [today, (today.0 + 1, 1, 1)] {
        let t = fts::day(&base, day);
        let err = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, || false, |_| {}).await.unwrap_err();
        assert!(matches!(err, ingest::fetch::Error::Unsupported(_)), "{}: {err}", t.period);
        assert!(!archive.join(format!("fts/daily/{}.pages", t.period)).exists(), "{}: nothing staged", t.period);
        assert!(db.latest_fetch("fts", "daily", &t.period).await.unwrap().is_none(), "{}", t.period);
    }
    assert!(log.lock().unwrap().is_empty(), "refused before any request");

    let yesterday = fts::day(&base, fts::uk_civil_date(now - 86_400));
    let outcome = fetch_fts(&db, &client, &archive, &yesterday, false, Duration::ZERO, || false, |_| {}).await.unwrap();
    assert_eq!(outcome, Outcome::Fetched);

    let _ = std::fs::remove_dir_all(&archive);
}

/// The probe reads the stop flag before every request of every day (issue 477
/// review: a 14-day catch-up is about an hour on the single runner). A stop
/// ends the walk on the stopped day, which did not land; the days before it
/// did, and the next run starts at the stopped day.
#[tokio::test]
async fn a_stopped_probe_ends_on_the_stopped_day_and_the_next_resumes_it() {
    let (base, log) = fts_keyset_server(Vec::new(), Quirks::default()).await;
    let archive = temp_dir("fts-probe-stop");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    let client = reqwest::Client::new();
    db.record_fetch(&registry_row("fts", "monthly", "2026-08")).await.unwrap();
    let days = |r: &[(String, Outcome)]| r.iter().map(|(p, o)| format!("{p} {o:?}")).collect::<Vec<_>>();

    // An empty day is one request: stop before the third.
    let asked = std::sync::atomic::AtomicUsize::new(0);
    let stop = || asked.fetch_add(1, Ordering::SeqCst) >= 2;
    let walked = probe_fts_daily(&db, &client, &archive, &base, (2026, 9, 5), Duration::ZERO, stop, |_| {}).await.unwrap();
    assert_eq!(days(&walked), ["2026-09-01 Fetched", "2026-09-02 Fetched", "2026-09-03 Stopped"]);
    assert_eq!(log.lock().unwrap().len(), 2);
    assert!(db.latest_fetch("fts", "daily", "2026-09-03").await.unwrap().is_none(), "the stopped day did not land");

    let walked = probe_fts_daily(&db, &client, &archive, &base, (2026, 9, 5), Duration::ZERO, || false, |_| {}).await.unwrap();
    assert_eq!(days(&walked), ["2026-09-03 Fetched", "2026-09-04 Fetched", "2026-09-05 Fetched"]);
    assert_eq!(log.lock().unwrap().len(), 5, "no day asked twice");

    let _ = std::fs::remove_dir_all(&archive);
}

/// The pause holds between ANY two FTS requests of the process, not only
/// inside one fetch: the next job's (or the next probe day's) first request
/// used to follow the last one at once, and the limiter has answered 429 with
/// `Retry-After: 120` even at an 11 s cadence (issue 477 review, lens
/// "operability"). Two one-request fetches, so only the pause BETWEEN the
/// calls can make this take the pause.
#[tokio::test]
async fn the_request_pause_holds_across_fetches() {
    let (base, log) = fts_keyset_server(Vec::new(), Quirks::default()).await;
    let archive = temp_dir("fts-pause-across");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    let client = reqwest::Client::new();
    let pause = Duration::from_millis(400);

    let started = std::time::Instant::now();
    for day in [(2026, 9, 1), (2026, 9, 2)] {
        fetch_fts(&db, &client, &archive, &fts::day(&base, day), false, pause, || false, |_| {}).await.unwrap();
    }
    assert_eq!(log.lock().unwrap().len(), 2);
    assert!(started.elapsed() >= pause, "the second fetch's request waited out the pause: {:?}", started.elapsed());

    let _ = std::fs::remove_dir_all(&archive);
}

/// A crash between the registry row and the staging cleanup leaves the
/// landing's pages behind, and nothing re-walks a landed day. A non-refetch
/// fetch of the day removes that debris; the newer pages of an interrupted
/// REFETCH stay, for the refetch to resume (issue 477 review, lens "operability").
#[tokio::test]
async fn leftover_staging_of_a_landed_package_is_removed_without_a_refetch() {
    let days: HashMap<String, Vec<Value>> =
        HashMap::from([("2026-09-03".to_owned(), vec![release("083253-2026", "fine")])]);
    let (base, hits) = fts_day_server(days).await;
    let archive = temp_dir("fts-leftover");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    let client = reqwest::Client::new();
    let t = fts::day(&base, (2026, 9, 3));
    fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, || false, |_| {}).await.unwrap();
    let landed_at = db.latest_fetch("fts", "daily", "2026-09-03").await.unwrap().unwrap().fetched_at as u64;
    let staging = archive.join("fts/daily/2026-09-03.pages");
    let stage = |name: &str, at: u64| {
        std::fs::create_dir_all(&staging).unwrap();
        let page = staging.join(name);
        std::fs::write(&page, json!({ "version": "1.1", "releases": [] }).to_string()).unwrap();
        let at = std::time::UNIX_EPOCH + Duration::from_secs(at);
        std::fs::File::options().write(true).open(&page).unwrap().set_modified(at).unwrap();
    };

    // Debris (no newer than the landing): removed, and the dir with it.
    stage("20260902T220000-20260903T235959.json", landed_at - 60);
    stage("20260902T220000-20260903T105959.json", landed_at);
    let before = hits.lock().unwrap().values().sum::<usize>();
    let outcome = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, || false, |_| {}).await.unwrap();
    assert_eq!(outcome, Outcome::Unchanged);
    assert_eq!(hits.lock().unwrap().values().sum::<usize>(), before, "no HTTP");
    assert!(!staging.exists(), "the landing's leftover staging is gone");

    // An interrupted refetch's newer page stays.
    stage("20260902T220000-20260903T235959.json", landed_at + 3_600);
    let outcome = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, || false, |_| {}).await.unwrap();
    assert_eq!(outcome, Outcome::Unchanged);
    assert_eq!(std::fs::read_dir(&staging).unwrap().count(), 1, "kept for the refetch to resume");

    let _ = std::fs::remove_dir_all(&archive);
}

/// One notice id, two releases (issue 477): `038018-2025` is a `tenderUpdate`
/// on the old procurement and an `award,contract` on the new one, under two
/// ocids. Within ONE package the assembler used to keep the first and drop the
/// second without trace. Now each distinct release is a member — `<id>.json`
/// for the lowest member hash, `<id>~<hash8>.json` for the other — while a
/// byte-identical repeat still collapses; served in the other order, the same
/// releases make the same zip byte for byte (issue 477 review: by serve order,
/// a refetch of an unchanged day could land as a new version); and the
/// processor stores both as notices of the same publication id.
#[tokio::test]
async fn one_id_carried_by_two_releases_keeps_both_and_a_repeat_collapses() {
    let update = json!({ "id": "038018-2025", "ocid": "ocds-h6vhtk-04a001", "tag": ["tenderUpdate"], "tender": { "title": "old" } });
    let award = json!({ "id": "038018-2025", "ocid": "ocds-h6vhtk-04b002", "tag": ["award", "contract"], "awards": [] });
    let serve = |releases: Vec<Value>| HashMap::from([("2025-04-10".to_owned(), releases)]);
    let (base, _hits) =
        fts_day_server(serve(vec![release("038019-2025", "next"), update.clone(), award.clone(), update.clone()])).await;
    let archive = temp_dir("fts-one-id-two");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    let client = reqwest::Client::new();
    let t = fts::day(&base, (2025, 4, 10));
    fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, || false, |_| {}).await.unwrap();

    let members = zip_members(&archive.join(&t.rel_path));
    let names: Vec<&str> = members.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names.len(), 3, "two releases of one id, one other; the repeat collapsed: {names:?}");
    assert_eq!(names[0], "038018-2025.json");
    let second = &members[1];
    assert_eq!(second.0, format!("038018-2025~{}.json", &ingest::sha256_hex(&second.1)[..8]));
    assert_eq!(names[2], "038019-2025.json");
    let first: Value = serde_json::from_slice(&members[0].1).unwrap();
    let other: Value = serde_json::from_slice(&second.1).unwrap();
    let mut ocids = [first["releases"][0]["ocid"].as_str().unwrap(), other["releases"][0]["ocid"].as_str().unwrap()];
    ocids.sort();
    assert_eq!(ocids, ["ocds-h6vhtk-04a001", "ocds-h6vhtk-04b002"], "both releases, one member each");
    assert!(
        ingest::sha256_hex(&members[0].1) < ingest::sha256_hex(&second.1),
        "the plain name goes to the lower hash, not the first served"
    );

    // The other serve order: the same zip, byte for byte.
    let (reversed, _) = fts_day_server(serve(vec![award, update, release("038019-2025", "next")])).await;
    let archive2 = temp_dir("fts-one-id-two-reversed");
    let db2 = store::Db::open(archive2.join("test.db").to_str().unwrap()).await.unwrap();
    let t2 = fts::day(&reversed, (2025, 4, 10));
    fetch_fts(&db2, &client, &archive2, &t2, false, Duration::ZERO, || false, |_| {}).await.unwrap();
    assert_eq!(
        std::fs::read(archive2.join(&t2.rel_path)).unwrap(),
        std::fs::read(archive.join(&t.rel_path)).unwrap(),
        "the zip depends on which releases were served, not their order"
    );
    let _ = std::fs::remove_dir_all(&archive2);

    // The processor reads the publication id from the payload, so the `~`
    // member is the SAME publication — a second notice row, not a quarantine.
    ingest::process::process(&db, &archive, "fts", "daily", None, |_, _| {}, || false).await.unwrap();
    assert_eq!(count(&db, "SELECT COUNT(*) FROM notices WHERE publication_id = '038018-2025'").await, 2);
    assert_eq!(count(&db, "SELECT COUNT(DISTINCT content_hash) FROM notices WHERE publication_id = '038018-2025'").await, 2);
    assert_eq!(count(&db, "SELECT COUNT(*) FROM notices WHERE member_path LIKE '038018-2025~%.json'").await, 1);
    assert_eq!(
        count(&db, "SELECT COUNT(*) FROM quarantine WHERE notice_id IS NULL").await,
        0,
        "no member was refused an identity"
    );

    let _ = std::fs::remove_dir_all(&archive);
}

async fn count(db: &store::Db, sql: &str) -> i64 {
    match db.scalar(sql).await.unwrap() {
        Some(store::turso::Value::Integer(n)) => n,
        other => panic!("{sql}: {other:?}"),
    }
}

// ------------------------------------------ FTS: dense spans (issue 477 unit 1b)

use serde_json::value::RawValue;
use std::collections::{BTreeMap, BTreeSet};

/// The live shapes of 2023-11-14 10:05:14–15 (fetch 1857), gzipped as probed:
/// the span's cursorless page and its cursor page 2 (between them all 15 of
/// `033562-2023`'s ocids), and the records of `041970` (an ocid the page
/// serves), `041977` (one it does not) and `04197e` (the next notice's).
const DENSE: &str = "tests/fixtures/fts/dense";
const PAGE_1: &str = "2023-11-14T100514-cursorless.json.gz";
const PAGE_2: &str = "2023-11-14T100514-cursor-p2.json.gz";

fn gunzip(name: &str) -> String {
    let mut text = String::new();
    flate2::read::GzDecoder::new(std::fs::File::open(format!("{DENSE}/{name}")).unwrap())
        .read_to_string(&mut text)
        .unwrap();
    text
}

/// A served page's releases, raw, nested as the page nests them.
fn page_raw_releases(text: &str) -> Vec<String> {
    fts::Page::read(text.as_bytes()).unwrap().releases().unwrap().iter().map(|r| r.get().to_owned()).collect()
}

/// A served record package's releases, raw, nested as the record nests them.
fn record_raw_releases(text: &str) -> Vec<String> {
    let top: HashMap<&str, &RawValue> = serde_json::from_str(text).unwrap();
    let records: Vec<HashMap<&str, &RawValue>> = serde_json::from_str(top["records"].get()).unwrap();
    let releases: Vec<&RawValue> = serde_json::from_str(records[0]["releases"].get()).unwrap();
    releases.iter().map(|r| r.get().to_owned()).collect()
}

/// `033562-2023`'s listing bytes per ocid. Page 2 serves all 15 ocids, page 1
/// the first 8, and the two agree byte for byte.
fn listing_033562() -> BTreeMap<String, String> {
    let mut by_ocid = BTreeMap::new();
    for page in [PAGE_1, PAGE_2] {
        for raw in page_raw_releases(&gunzip(page)) {
            let ocid = fts::release_ocid(&RawValue::from_string(raw.clone()).unwrap()).unwrap();
            let held = by_ocid.entry(ocid).or_insert_with(|| raw.clone());
            assert_eq!(*held, raw, "one release per ocid, whichever page serves it");
        }
    }
    assert_eq!(by_ocid.len(), 15);
    by_ocid
}

fn ocids(range: std::ops::RangeInclusive<u32>) -> Vec<String> {
    range.map(|n| format!("ocds-h6vhtk-{n:06x}")).collect()
}

/// The live page's header, up to its `releases` array: the dense server lays
/// every page out under it, the API's 4-space layout.
fn page_head() -> String {
    let page = gunzip(PAGE_1);
    page[..page.find("\"releases\": [").unwrap()].to_owned()
}

/// A listing row as the dense server holds it: notice id, hidden instant, and
/// raw bytes at a page's depth (8 spaces: `releases` at 4, its items at 8).
#[derive(Clone)]
struct RawRow {
    id: String,
    at: i64,
    raw: String,
}

/// What `/ocdsRecordPackages/{ocid}` answers, keyed by the ocid; an ocid
/// without one is a 404. A key `package <ocid>` is what
/// `/ocdsReleasePackages/{ocid}` answers instead of its default: the listing's
/// rows of that ocid under the page's header, a 404 when it has none.
#[derive(Clone)]
enum Reply {
    Body(String),
    /// A 200 with no body: the live answer for `04196f`, asked twice.
    Empty,
}

/// Every request the dense server saw, in order, with when it arrived:
/// `list <query>`, `record <ocid>` or `package <ocid>`.
type Timed = Arc<Mutex<Vec<(std::time::Instant, String)>>>;

fn records_asked(log: &Timed) -> Vec<String> {
    log.lock().unwrap().iter().filter_map(|(_, l)| l.strip_prefix("record ").map(str::to_owned)).collect()
}

fn packages_asked(log: &Timed) -> Vec<String> {
    log.lock().unwrap().iter().filter_map(|(_, l)| l.strip_prefix("package ").map(str::to_owned)).collect()
}

/// A page of `releases` (raw, at a page's depth) under `head`, as the live API
/// lays one out.
fn lay_out(head: &str, releases: &[&str], links: &str) -> String {
    let mut page = format!("{head}\"releases\": [");
    if !releases.is_empty() {
        page.push_str(&format!("\n        {}\n    ", releases.join(",\n        ")));
    }
    page.push(']');
    page.push_str(links);
    page.push_str("\n}");
    page
}

/// The keyset server's listing model (a window, newest notice id first, 100
/// a page, ties in row order) over RAW rows, laid out as the live API lays a
/// page out — the real header, 4-space indent — so the walk's re-nesting
/// meets the real depths; and the record endpoint.
async fn fts_dense_server(head: String, rows: Vec<RawRow>, records: HashMap<String, Reply>) -> (String, Timed) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/api/1.0", listener.local_addr().unwrap());
    let log: Timed = Arc::new(Mutex::new(Vec::new()));
    let (head, rows, records) = (Arc::new(head), Arc::new(rows), Arc::new(records));
    let app = axum::Router::new()
        .route("/api/1.0/ocdsReleasePackages", {
            let (base, log, head, rows) = (base.clone(), log.clone(), head.clone(), rows.clone());
            get(move |axum::extract::RawQuery(raw): axum::extract::RawQuery, Query(q): Query<HashMap<String, String>>| {
                let (base, log, head, rows) = (base.clone(), log.clone(), head.clone(), rows.clone());
                async move {
                    log.lock().unwrap().push((std::time::Instant::now(), format!("list {}", raw.unwrap_or_default())));
                    let from_s = q.get("updatedFrom").cloned().unwrap_or_default();
                    let to_s = q.get("updatedTo").cloned().unwrap_or_default();
                    let (Some(from), Some(to)) = (wall_secs(&from_s), wall_secs(&to_s)) else {
                        return (StatusCode::BAD_REQUEST, "bad window").into_response();
                    };
                    if to <= from {
                        return (StatusCode::BAD_REQUEST, "'updatedTo' must be later than 'updatedFrom'").into_response();
                    }
                    let mut window: Vec<&RawRow> = rows.iter().filter(|r| from <= r.at && r.at <= to).collect();
                    window.sort_by(|a, b| id_order(&b.id).cmp(&id_order(&a.id)));
                    let served: Vec<&str> = window.iter().take(100).map(|r| r.raw.as_str()).collect();
                    let links = if window.len() > 100 {
                        format!(
                            ",\n    \"links\": {{\n        \"next\": \"{base}/ocdsReleasePackages?limit=100&updatedFrom={from_s}&updatedTo={to_s}&cursor=695588\"\n    }}"
                        )
                    } else {
                        String::new()
                    };
                    let page = lay_out(&head, &served, &links);
                    (StatusCode::OK, [(header::CONTENT_TYPE, "application/json")], page).into_response()
                }
            })
        })
        .route("/api/1.0/ocdsReleasePackages/{ocid}", {
            let (log, head, rows, records) = (log.clone(), head.clone(), rows.clone(), records.clone());
            get(move |axum::extract::Path(ocid): axum::extract::Path<String>| {
                let (log, head, rows, records) = (log.clone(), head.clone(), rows.clone(), records.clone());
                async move {
                    log.lock().unwrap().push((std::time::Instant::now(), format!("package {ocid}")));
                    let json = [(header::CONTENT_TYPE, "application/json")];
                    match records.get(&format!("package {ocid}")) {
                        Some(Reply::Body(body)) => return (StatusCode::OK, json, body.clone()).into_response(),
                        Some(Reply::Empty) => return (StatusCode::OK, json, String::new()).into_response(),
                        None => {}
                    }
                    let own = format!("\"ocid\": \"{ocid}\"");
                    let served: Vec<&str> = rows.iter().filter(|r| r.raw.contains(&own)).map(|r| r.raw.as_str()).collect();
                    if served.is_empty() {
                        return StatusCode::NOT_FOUND.into_response();
                    }
                    (StatusCode::OK, json, lay_out(&head, &served, "")).into_response()
                }
            })
        })
        .route("/api/1.0/ocdsRecordPackages/{ocid}", {
            let (log, records) = (log.clone(), records.clone());
            get(move |axum::extract::Path(ocid): axum::extract::Path<String>| {
                let (log, records) = (log.clone(), records.clone());
                async move {
                    log.lock().unwrap().push((std::time::Instant::now(), format!("record {ocid}")));
                    match records.get(&ocid) {
                        Some(Reply::Body(body)) => {
                            (StatusCode::OK, [(header::CONTENT_TYPE, "application/json")], body.clone()).into_response()
                        }
                        Some(Reply::Empty) => (StatusCode::OK, [(header::CONTENT_TYPE, "application/json")], String::new()).into_response(),
                        None => StatusCode::NOT_FOUND.into_response(),
                    }
                }
            })
        });
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (base, log)
}

/// 2023-11-14 as the live API serves it (fetch 1857). At 10:05:15 the
/// pipeline notice `033562-2023` fans out: each of its 15 ocids' real listing
/// release 14 times, ocid by ocid, so the two-second span's first page is the
/// live page's 7 × 14 + 2. At 10:07:02 comes the next notice, `033564-2023`
/// (ocid `04197e`). Records as probed: `04196c`..`04196e` 404, `04196f` an
/// empty 200, `041970`, `041977` and `04197e` the real ones; every other
/// ocid's is the real `041977` with its release swapped for theirs, 8 spaces
/// deeper. Release packages: `04196f`'s the real one, every other ocid's the
/// listing's rows of it (`04196e`'s a 404, as probed).
fn dense_2023_11_14() -> (Vec<RawRow>, HashMap<String, Reply>) {
    let listing = listing_033562();
    let at = wall_secs("2023-11-14T10:05:15").unwrap();
    let mut rows = Vec::new();
    for raw in listing.values() {
        rows.extend((0..14).map(|_| RawRow { id: "033562-2023".into(), at, raw: raw.clone() }));
    }
    let neighbour = gunzip("record-04197e.json.gz");
    let next = record_raw_releases(&neighbour).into_iter().find(|r| r.contains("\"id\": \"033564-2023\"")).unwrap();
    let at = wall_secs("2023-11-14T10:07:02").unwrap();
    rows.push(RawRow { id: "033564-2023".into(), at, raw: next.replace("\n        ", "\n") });

    let template = gunzip("record-041977.json.gz");
    let deeper = |raw: &str| raw.replace('\n', "\n        ");
    let own = deeper(&listing["ocds-h6vhtk-041977"]);
    assert_eq!(template.matches(&own).count(), 14, "the live record holds the listing's release 14 times, 8 deeper");
    let mut records = HashMap::from([
        ("ocds-h6vhtk-04196f".to_owned(), Reply::Empty),
        ("ocds-h6vhtk-041970".to_owned(), Reply::Body(gunzip("record-041970.json.gz"))),
        ("ocds-h6vhtk-041977".to_owned(), Reply::Body(template.clone())),
        ("ocds-h6vhtk-04197e".to_owned(), Reply::Body(neighbour)),
        ("package ocds-h6vhtk-04196f".to_owned(), Reply::Body(gunzip("release-package-04196f.json.gz"))),
    ]);
    for (ocid, raw) in &listing {
        records.entry(ocid.clone()).or_insert_with(|| {
            Reply::Body(template.replace(&own, &deeper(raw)).replace("ocds-h6vhtk-041977", ocid))
        });
    }
    (rows, records)
}

/// The members of a notice id in a zip: `<id>.json` and its `~` variants.
fn members_of(path: &std::path::Path, id: &str) -> Vec<(String, Vec<u8>)> {
    zip_members(path).into_iter().filter(|(n, _)| n == &format!("{id}.json") || n.starts_with(&format!("{id}~"))).collect()
}

/// THE FAN-OUT, COMPLETED (issue 477 unit 1b): 2023-11 as fetch 1857 met it.
/// The two-second span 10:05:14–15 is full of one notice, so its page stays a
/// leaf and `033562-2023`'s ocid run is read by record: the page's 8 ocids
/// (`04196f`'s empty record vouched for by its release package), then outward
/// until a 404 below (`04196e`, its release package a 404 too, and the two
/// ocids past it as well) and another notice above (`04197e`) end it, and
/// never past them. The month lands all 15 ocids' releases as 15 members of
/// the one id, each byte-identical to the member a listing-served release
/// makes, and every record request keeps the pace.
#[tokio::test]
async fn dense_a_fanned_out_notice_lands_every_ocid_of_its_run() {
    let (rows, records) = dense_2023_11_14();
    let (base, log) = fts_dense_server(page_head(), rows, records).await;
    let archive = temp_dir("fts-dense");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    let client = reqwest::Client::new();
    let t = fts::monthly(&base, (2023, 11));
    let pause = Duration::from_millis(15);

    let mut last = DenseTally::default();
    let outcome = fetch_fts(&db, &client, &archive, &t, false, pause, || false, |p| last = p.dense).await.unwrap();
    assert_eq!(outcome, Outcome::Fetched);
    assert_eq!(
        last,
        DenseTally { spans: 1, ocids: 15, requests: 21 },
        "one dense span of 15 ocids: 18 records and 3 release packages asked"
    );
    assert_eq!(
        last.row_suffix(),
        " · 1 dense span(s) completed: 15 ocid(s), 21 record request(s)",
        "what the job row reads"
    );

    let mut expected = ocids(0x04196f..=0x041976); // the page's, first to last
    expected.extend(ocids(0x04196c..=0x04196e).into_iter().rev()); // below: a 404, and the two past it
    expected.extend(ocids(0x041977..=0x04197e)); // above: 7 join, then 033564-2023's ends it
    assert_eq!(records_asked(&log), expected);
    assert_eq!(
        packages_asked(&log),
        ["ocds-h6vhtk-04196f", "ocds-h6vhtk-04196e"],
        "the empty seed's, and the 404's that ends the side below"
    );
    {
        let log = log.lock().unwrap();
        assert!(!log.iter().any(|(_, l)| l.contains("cursor=")), "never the cursor");
        let dense = "updatedFrom=2023-11-14T10:05:14&updatedTo=2023-11-14T10:05:15";
        assert_eq!(log.iter().filter(|(_, l)| l.contains(dense)).count(), 1, "the dense span asked once");
        for pair in log.windows(2) {
            assert!(pair[1].0 - pair[0].0 >= pause, "{} followed {} within the pause", pair[1].1, pair[0].1);
        }
    }

    // 15 members of the one id: `<id>.json` and 14 `~<hash8>`, each the member
    // the listing's own bytes for its ocid make under the page's header.
    let zip = archive.join(&t.rel_path);
    let held = members_of(&zip, "033562-2023");
    assert_eq!(held.len(), 15, "{:?}", held.iter().map(|m| &m.0).collect::<Vec<_>>());
    assert_eq!(held[0].0, "033562-2023.json");
    for (name, bytes) in &held[1..] {
        assert_eq!(*name, format!("033562-2023~{}.json", &ingest::sha256_hex(bytes)[..8]));
    }
    let page_1 = gunzip(PAGE_1);
    let page = fts::Page::read(page_1.as_bytes()).unwrap();
    let mut built: Vec<Vec<u8>> =
        listing_033562().values().map(|raw| page.member_bytes(&RawValue::from_string(raw.clone()).unwrap())).collect();
    let mut bytes: Vec<Vec<u8>> = held.into_iter().map(|(_, b)| b).collect();
    built.sort();
    bytes.sort();
    assert!(bytes == built, "every ocid's release, byte for byte as the listing serves it");
    // The next notice lands from its own listing leaf; its record's 2024
    // award (another notice of `04197e`) is not taken.
    let names: Vec<String> = zip_members(&zip).into_iter().map(|(n, _)| n).collect();
    assert_eq!(names.len(), 16, "{names:?}");
    assert!(names.contains(&"033564-2023.json".to_owned()));
    assert!(!archive.join("fts/monthly/2023-11.pages").exists(), "staging removed after landing");

    // And the processor reads them as 15 notices of one publication.
    ingest::process::process(&db, &archive, "fts", "monthly", None, |_, _| {}, || false).await.unwrap();
    assert_eq!(count(&db, "SELECT COUNT(*) FROM notices WHERE publication_id = '033562-2023'").await, 15);
    assert_eq!(count(&db, "SELECT COUNT(DISTINCT content_hash) FROM notices WHERE publication_id = '033562-2023'").await, 15);

    let _ = std::fs::remove_dir_all(&archive);
}

/// The re-nesting on the live bytes (issue 477 unit 1b): the records of
/// `041970` (an ocid the page serves) and `041977` (one only page 2 serves)
/// nest the release 16 deep where a page nests it 8; moved up by that measured
/// difference, it is the listing's own bytes, so a member built from a record
/// is the member a listing-served release makes. `04197e`'s record carries
/// only other notices.
#[test]
fn dense_a_renested_record_release_is_the_listing_bytes() {
    let listing = listing_033562();
    let page_1 = gunzip(PAGE_1);
    let page = fts::Page::read(page_1.as_bytes()).unwrap();
    let nesting = fts::listing_nesting(&page.releases().unwrap()).unwrap();
    assert_eq!(nesting, Some(8));
    let ids = BTreeSet::from(["033562-2023".to_owned()]);
    for (file, ocid) in [("record-041970.json.gz", "ocds-h6vhtk-041970"), ("record-041977.json.gz", "ocds-h6vhtk-041977")] {
        let record = gunzip(file);
        let served = record_raw_releases(&record);
        assert_eq!(served.len(), 14, "{ocid}");
        assert_eq!(fts::nesting(&RawValue::from_string(served[0].clone()).unwrap()), Some(16));
        assert_ne!(served[0], listing[ocid], "{ocid}: as served, the record's bytes are not the listing's");
        let fts::RecordSays::Carries(releases) =
            fts::record_releases(record.as_bytes(), &fts::Ocid::parse(ocid).unwrap(), &ids, nesting).unwrap()
        else {
            panic!("{ocid} carries the notice");
        };
        assert_eq!(releases.len(), 1, "{ocid}: 14 copies of one release");
        assert_eq!(releases[0].get(), listing[ocid], "{ocid}: re-nested, the listing's bytes");
        let from_listing = page.member_bytes(&RawValue::from_string(listing[ocid].clone()).unwrap());
        assert_eq!(page.member_bytes(&releases[0]), from_listing, "{ocid}: the same member, the same hash");
    }
    let neighbour = gunzip("record-04197e.json.gz");
    let next = fts::Ocid::parse("ocds-h6vhtk-04197e").unwrap();
    let fts::RecordSays::Other(elsewhere) = fts::record_releases(neighbour.as_bytes(), &next, &ids, nesting).unwrap() else {
        panic!("04197e is another notice's");
    };
    let named: Vec<(&str, Option<&str>)> = elsewhere.iter().map(|e| (e.id.as_str(), e.date.as_deref())).collect();
    assert_eq!(named, [("033564-2023", Some("2023-11-14T10:07:02Z")), ("017735-2024", Some("2024-06-07T14:17:43+01:00"))]);

    // `04196f`'s RECORD is an empty 200; its release package serves the page's
    // release at the page's own depth, 14 times, beside 9 later notices.
    let package = gunzip("release-package-04196f.json.gz");
    let first = fts::Ocid::parse("ocds-h6vhtk-04196f").unwrap();
    let (says, whole) = fts::package_releases(package.as_bytes(), &first, &ids, nesting).unwrap();
    assert!(whole, "23 releases, no next");
    let fts::RecordSays::Carries(releases) = says else { panic!("04196f carries the notice") };
    assert_eq!(releases.len(), 1);
    assert_eq!(releases[0].get(), listing["ocds-h6vhtk-04196f"], "the listing's bytes, as served");
}

/// Run a 2023-11-14 daily against `records` and expect it to fail LOUD
/// (issue 477 unit 1b): `Malformed`, naming `says`, its dense page and every
/// parsed record that names a notice still staged, nothing landed or
/// registered. Returns the requests seen and the records staged.
async fn dense_refused(case: &str, records: HashMap<String, Reply>, says: &[&str]) -> (Timed, Vec<String>) {
    let (rows, _) = dense_2023_11_14();
    let (base, log) = fts_dense_server(page_head(), rows, records).await;
    let archive = temp_dir(&format!("fts-dense-refused-{}", case.replace(' ', "-")));
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    let t = fts::day(&base, (2023, 11, 14));
    let err = fetch_fts(&db, &reqwest::Client::new(), &archive, &t, false, Duration::ZERO, || false, |_| {})
        .await
        .unwrap_err();
    assert!(matches!(err, ingest::fetch::Error::Malformed(_)), "{case}: {err}");
    let message = err.to_string();
    for part in says.iter().chain(&["issue 477"]) {
        assert!(message.contains(part), "{case}: no {part:?} in {message}");
    }
    let staging = archive.join("fts/daily/2023-11-14.pages");
    let span = "20231114T100514-20231114T100515";
    let staged: BTreeSet<String> =
        std::fs::read_dir(&staging).unwrap().map(|e| e.unwrap().file_name().into_string().unwrap()).collect();
    assert!(staged.contains(&format!("{span}.json")), "{case}: the dense span's page stays");
    let records: Vec<String> =
        staged.iter().filter_map(|n| n.strip_prefix(&format!("{span}-r"))?.strip_suffix(".json")).map(str::to_owned).collect();
    assert!(!archive.join(&t.rel_path).exists(), "{case}: nothing lands");
    assert!(db.latest_fetch("fts", "daily", "2023-11-14").await.unwrap().is_none(), "{case}: nothing registers");
    let _ = std::fs::remove_dir_all(&archive);
    (log, records)
}

/// A record that holds no release is the server's defect (`04196f`'s empty
/// body, three asks), and it says nothing about whose the ocid is — whether
/// the body is empty or the package lists no record (issue 477 unit 1b
/// review: `{"records": []}` used to read as "another notice's", ending the
/// run silently and staged for good). For an ocid the page serves, the seed's
/// release package vouches for the page's release (the fan-out test). Off the
/// page it is the ocid's release package that decides: here it carries the
/// notice, so the walk fails LOUD at `041979` — ending the run there would
/// drop it and every ocid past it — and stages nothing for it.
#[tokio::test]
async fn dense_a_record_without_a_release_fails_off_the_page_when_the_notice_is_there() {
    for (case, reply) in [("empty body", Reply::Empty), ("no records", Reply::Body(r#"{"records": []}"#.into()))] {
        let (_, mut records) = dense_2023_11_14();
        records.insert("ocds-h6vhtk-041979".into(), reply);
        let (log, staged) =
            dense_refused(case, records, &["ocds-h6vhtk-041979", "holds no release", "its release package carries 033562-2023"])
                .await;
        let asked = records_asked(&log);
        assert_eq!(asked.first().map(String::as_str), Some("ocds-h6vhtk-04196f"), "{case}: the page's empty record was read, and passed");
        assert_eq!(asked.last().map(String::as_str), Some("ocds-h6vhtk-041979"), "{case}: the walk stopped there");
        assert_eq!(packages_asked(&log).last().map(String::as_str), Some("ocds-h6vhtk-041979"), "{case}: its release package decided");
        assert_eq!(staged, ocids(0x041970..=0x041978), "{case}: every record naming the notice, none of the 404s or empty ones");
    }
}

/// The same `{"records": []}` at the ocid that ENDS the run (`04197e`): its
/// release package names another notice, `033564-2023`, dated outside the
/// span, so the side ends there and the notice lands whole — and nothing is
/// staged for the record that said nothing.
#[tokio::test]
async fn dense_a_release_less_record_at_the_end_is_decided_by_its_release_package() {
    let (rows, mut records) = dense_2023_11_14();
    records.insert("ocds-h6vhtk-04197e".into(), Reply::Body(r#"{"records": []}"#.into()));
    let (base, log) = fts_dense_server(page_head(), rows, records).await;
    let archive = temp_dir("fts-dense-release-less-end");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    let t = fts::day(&base, (2023, 11, 14));
    let stop_at_landing = || std::fs::read_dir(archive.join("fts/daily/2023-11-14.pages")).is_ok_and(|mut d| {
        d.any(|e| e.unwrap().file_name().to_string_lossy().ends_with("-rocds-h6vhtk-04197e.json"))
    });
    let outcome = fetch_fts(&db, &reqwest::Client::new(), &archive, &t, false, Duration::ZERO, stop_at_landing, |_| {})
        .await
        .unwrap();
    assert_eq!(outcome, Outcome::Fetched, "04197e's record was never staged, so the stop never fired");
    assert_eq!(packages_asked(&log), ["ocds-h6vhtk-04196f", "ocds-h6vhtk-04196e", "ocds-h6vhtk-04197e"]);
    assert_eq!(records_asked(&log).last().map(String::as_str), Some("ocds-h6vhtk-04197e"));
    assert_eq!(members_of(&archive.join(&t.rel_path), "033562-2023").len(), 15);
    let _ = std::fs::remove_dir_all(&archive);
}

/// A 404 is no proof that a notice's ocids stopped (issue 477 unit 1b
/// review): the series has holes (`04196c`..`04196e` are all 404), and the
/// record endpoint has failed real ocids before. Each fails LOUD where the
/// walk used to end the side silently and land a notice short of ocids:
/// - a hole inside the run: `04196e` is 404, but `04196d` past it carries the
///   notice;
/// - a 404 record for an ocid whose release package carries the notice
///   (`041979`, with `041979`..`04197d` past it);
/// - the seed `04196f`, whose record is empty, with a release package that
///   shows a second release of the notice the page hides, or none of it.
#[tokio::test]
async fn dense_an_end_that_is_not_proven_fails_loud() {
    let (_, base_records) = dense_2023_11_14();
    let template = gunzip("record-041977.json.gz");
    let mut hole = base_records.clone();
    hole.insert("ocds-h6vhtk-04196d".into(), Reply::Body(template.replace("ocds-h6vhtk-041977", "ocds-h6vhtk-04196d")));
    let (log, staged) = dense_refused("a hole", hole, &["ocds-h6vhtk-04196e is absent", "ocds-h6vhtk-04196d, 1 past it", "a hole"]).await;
    assert_eq!(packages_asked(&log), ["ocds-h6vhtk-04196f", "ocds-h6vhtk-04196e"], "the 404 confirmed before the look-ahead");
    assert!(staged.contains(&"ocds-h6vhtk-04196d".to_owned()), "the record past the hole stays staged for a person");

    let mut gone = base_records.clone();
    gone.remove("ocds-h6vhtk-041979");
    dense_refused("a 404 record of the notice", gone, &["the record of ocds-h6vhtk-041979 is a 404", "its release package carries 033562-2023"]).await;

    let (rows, _) = dense_2023_11_14();
    let page_bytes = rows.iter().find(|r| r.raw.contains("\"ocid\": \"ocds-h6vhtk-04196f\"")).unwrap().raw.clone();
    let mut second = base_records.clone();
    let other = page_bytes.replacen("\"tag\": [", "\"tag\": [\"planningUpdate\", ", 1);
    assert_ne!(other, page_bytes);
    second.insert("package ocds-h6vhtk-04196f".into(), Reply::Body(lay_out(&page_head(), &[&page_bytes, &other], "")));
    dense_refused("a hidden second release", second, &["the release package of ocds-h6vhtk-04196f holds a release of 033562-2023 the page does not show"]).await;

    let mut not_listed = base_records;
    not_listed.insert("package ocds-h6vhtk-04196f".into(), Reply::Body(lay_out(&page_head(), &[], "")));
    dense_refused("an empty seed package", not_listed, &["the record of ocds-h6vhtk-04196f, an ocid the page serves, holds no release", "does not carry 033562-2023"]).await;
}

/// The record that ends a side names another notice — but one dated INSIDE
/// the span, which only a notice the page's 100 rows hide could be (issue 477
/// unit 1b review): it fails LOUD rather than landing the span without it.
#[tokio::test]
async fn dense_another_notice_dated_inside_the_span_at_the_end_fails_loud() {
    let (_, mut records) = dense_2023_11_14();
    let neighbour = gunzip("record-04197e.json.gz").replace("\"date\": \"2023-11-14T10:07:02Z\"", "\"date\": \"2023-11-14T10:05:14Z\"");
    assert!(neighbour.contains("2023-11-14T10:05:14Z"));
    records.insert("ocds-h6vhtk-04197e".into(), Reply::Body(neighbour));
    dense_refused("in the span", records, &["ocds-h6vhtk-04197e holds another notice, 033564-2023, dated 2023-11-14T10:05:14Z"]).await;
}

/// The daily probe meets dense spans too, and is the only FTS fetch path once
/// the backfill is done (issue 477 unit 1b review): it forwards every day's
/// progress, so the walk shows while it runs and its tally reaches the row.
#[tokio::test]
async fn dense_the_daily_probe_reports_its_dense_spans() {
    let (rows, records) = dense_2023_11_14();
    let (base, _) = fts_dense_server(page_head(), rows, records).await;
    let archive = temp_dir("fts-dense-probe");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    for day in 1..=12 {
        db.record_fetch(&registry_row("fts", "daily", &format!("2023-11-{day:02}"))).await.unwrap();
    }
    let mut by_day: BTreeMap<String, DenseTally> = BTreeMap::new();
    let walked = probe_fts_daily(&db, &reqwest::Client::new(), &archive, &base, (2023, 11, 14), Duration::ZERO, || false, |p| {
        by_day.insert(p.day.to_owned(), p.dense);
    })
    .await
    .unwrap();
    assert_eq!(walked.iter().map(|(d, _)| d.as_str()).collect::<Vec<_>>(), ["2023-11-13", "2023-11-14"]);
    assert_eq!(by_day["2023-11-13"], DenseTally::default());
    assert_eq!(by_day["2023-11-14"], DenseTally { spans: 1, ocids: 15, requests: 21 });
    let _ = std::fs::remove_dir_all(&archive);
}

/// What records cannot complete fails LOUD before any record is asked, its
/// page staged and nothing landed (issue 477 unit 1b): a full two-second page
/// of TWO notice ids, where a notice below its 100 rows could hide whole and
/// no record would name it; a page whose ocids are further apart than
/// `DENSE_RUN_CAP`; and ocids of two series.
#[tokio::test]
async fn dense_a_span_records_cannot_complete_fails_loud_before_asking_one() {
    let at = wall_secs("2023-11-14T10:05:15").unwrap();
    let x = listing_033562()["ocds-h6vhtk-04196f"].clone();
    let rows = |parts: Vec<(&str, String, usize)>| -> Vec<RawRow> {
        parts
            .into_iter()
            .flat_map(|(id, raw, n)| (0..n).map(move |_| RawRow { id: id.to_owned(), at, raw: raw.clone() }))
            .collect()
    };
    let far = format!("ocds-h6vhtk-{:06x}", 0x04196f + fts::DENSE_RUN_CAP);
    let cases = [
        (
            "two notice ids",
            rows(vec![
                ("033563-2023", x.replacen("\"id\": \"033562-2023\"", "\"id\": \"033563-2023\"", 1), 3),
                ("033562-2023", x.clone(), 210),
            ]),
            "2 notice ids (033562-2023, 033563-2023)",
        ),
        (
            "over the cap",
            rows(vec![("033562-2023", x.clone(), 50), ("033562-2023", x.replace("ocds-h6vhtk-04196f", &far), 60)]),
            "past the cap of 500",
        ),
        (
            "two series",
            rows(vec![("033562-2023", x.clone(), 60), ("033562-2023", x.replace("ocds-h6vhtk-04196f", "ocds-b5fd17-04196f"), 60)]),
            "two series",
        ),
    ];
    for (case, rows, says) in cases {
        let (base, log) = fts_dense_server(page_head(), rows, HashMap::new()).await;
        let archive = temp_dir(&format!("fts-dense-refused-{}", case.replace(' ', "-")));
        let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
        let t = fts::day(&base, (2023, 11, 14));
        let err = fetch_fts(&db, &reqwest::Client::new(), &archive, &t, false, Duration::ZERO, || false, |_| {})
            .await
            .unwrap_err();
        assert!(matches!(err, ingest::fetch::Error::Malformed(_)), "{case}: {err}");
        assert!(err.to_string().contains(says) && err.to_string().contains("issue 477"), "{case}: {err}");
        assert!(records_asked(&log).is_empty(), "{case}: no record asked");
        let page = archive.join("fts/daily/2023-11-14.pages/20231114T100514-20231114T100515.json");
        assert!(page.exists(), "{case}: the dense span's page stays staged");
        assert!(!archive.join(&t.rel_path).exists(), "{case}: nothing lands");
        let _ = std::fs::remove_dir_all(&archive);
    }
}

/// A stopped dense walk resumes where it stopped: the stop checkpoint is read
/// before every record request, as before every span page (issue 450), and a
/// record staged by the stopped run is read from staging, never asked again.
/// Only the replies that stage nothing — `04196f`'s empty body, the 404s
/// below the run, the release packages — are asked again, and the resumed
/// walk lands all 15 ocids. A stop before each kind of request: the dense
/// span's page, the empty seed record, its release package, a seed mid-run,
/// the 404 below, the look-ahead past it, the first record above, the other
/// notice's that ends the run, and the first span page after it — where one
/// staged record is damaged first, and only it is asked again. (Every stop
/// point passes; nine keep the test around ten seconds in a debug build.)
#[tokio::test]
async fn dense_a_stopped_walk_resumes_without_asking_a_staged_record_again() {
    let (rows, records) = dense_2023_11_14();
    let client = reqwest::Client::new();
    let walk = |log: &Timed| log.lock().unwrap().iter().map(|(_, l)| l.clone()).collect::<Vec<_>>();
    // A clean run, to count the requests.
    let (base, log) = fts_dense_server(page_head(), rows.clone(), records.clone()).await;
    let archive = temp_dir("fts-dense-count");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    fetch_fts(&db, &client, &archive, &fts::day(&base, (2023, 11, 14)), false, Duration::ZERO, || false, |_| {}).await.unwrap();
    let clean = walk(&log);
    let at = |line: &str| clean.iter().position(|l| l == line).unwrap_or_else(|| panic!("{line} asked"));
    let after_records = clean.iter().rposition(|l| l.starts_with("record ")).unwrap() + 1;
    assert!(after_records < clean.len(), "the day's later spans follow the dense walk");
    let _ = std::fs::remove_dir_all(&archive);

    let stops = [
        at("record ocds-h6vhtk-04196f") - 1,
        at("record ocds-h6vhtk-04196f"),
        at("package ocds-h6vhtk-04196f"),
        at("record ocds-h6vhtk-041973"),
        at("record ocds-h6vhtk-04196e"),
        at("record ocds-h6vhtk-04196d"),
        at("record ocds-h6vhtk-041977"),
        at("record ocds-h6vhtk-04197e"),
        after_records,
    ];
    for stop_after in stops {
        let (base, log) = fts_dense_server(page_head(), rows.clone(), records.clone()).await;
        let archive = temp_dir(&format!("fts-dense-resume-{stop_after}"));
        let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
        let t = fts::day(&base, (2023, 11, 14));
        let asked = std::sync::atomic::AtomicUsize::new(0);
        let stop = || asked.fetch_add(1, Ordering::SeqCst) >= stop_after;
        let outcome = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, stop, |_| {}).await.unwrap();
        assert_eq!(outcome, Outcome::Stopped, "stopped after {stop_after}");
        assert_eq!(walk(&log), clean[..stop_after], "stopped after {stop_after}: the clean run's requests, then the stop");
        assert!(!archive.join(&t.rel_path).exists());
        let damaged = (stop_after == after_records).then(|| {
            let path = archive.join("fts/daily/2023-11-14.pages/20231114T100514-20231114T100515-rocds-h6vhtk-041972.json");
            std::fs::write(&path, b"{\"records\": [").unwrap();
            "record ocds-h6vhtk-041972".to_owned()
        });

        let outcome = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, || false, |_| {}).await.unwrap();
        assert_eq!(outcome, Outcome::Fetched, "resumed after {stop_after}");
        let mut seen: HashMap<String, usize> = HashMap::new();
        for line in walk(&log) {
            *seen.entry(line).or_default() += 1;
        }
        for (line, times) in &seen {
            let unstaged = line.starts_with("package ")
                || ["record ocds-h6vhtk-04196f", "record ocds-h6vhtk-04196e", "record ocds-h6vhtk-04196d", "record ocds-h6vhtk-04196c"]
                    .contains(&line.as_str())
                || damaged.as_deref() == Some(line.as_str());
            assert!(*times == 1 || (unstaged && *times == 2), "resumed after {stop_after}: {line} asked {times} times");
        }
        assert_eq!(members_of(&archive.join(&t.rel_path), "033562-2023").len(), 15, "resumed after {stop_after}");
        let _ = std::fs::remove_dir_all(&archive);
    }
}

// ---------------------------------------------------------------------------
// Issue 477 unit 3: the by-id audit of the ids below each year's highest.

use ingest::fetch::{audit_fts_ids, fts_id_census, AuditOptions};
use ingest::fts::audit::Verdict;

/// The by-id endpoint, `GET /api/1.0/ocdsReleasePackages/{id}`: each id answers
/// what `answers` says (`404`, `400`, `empty`, `nopkg` = `{"releases": []}`,
/// `once` = present on the first ask and 404 after, or a release date for a
/// present id); an id it does not name is a 404. Logs every id asked, in order,
/// control requests included ([`asked`] filters those out).
async fn fts_id_server(answers: HashMap<&'static str, &'static str>) -> (String, Log) {
    let answers = Arc::new(answers);
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    let app = axum::Router::new().route("/api/1.0/ocdsReleasePackages/{id}", {
        let log = log.clone();
        get(move |axum::extract::Path(id): axum::extract::Path<String>| {
            let (answers, log) = (answers.clone(), log.clone());
            async move {
                let seen = {
                    let mut log = log.lock().unwrap();
                    let seen = log.iter().filter(|asked| **asked == id).count();
                    log.push(id.clone());
                    seen
                };
                let page = |releases: Value| {
                    json!({
                        "uri": format!("http://fts/api/1.0/ocdsReleasePackages/{id}"),
                        "version": "1.1",
                        "publishedDate": "2026-10-04T00:00:00Z",
                        "publisher": { "name": "Cabinet Office" },
                        "releases": releases,
                    })
                    .to_string()
                };
                let present = |date: &str| {
                    page(json!([{ "id": id, "ocid": format!("ocds-h6vhtk-{id}"), "date": date, "tag": ["tender"] }]))
                };
                match answers.get(id.as_str()).copied().unwrap_or("404") {
                    "404" => (StatusCode::NOT_FOUND, String::new()).into_response(),
                    "400" => (StatusCode::BAD_REQUEST, String::new()).into_response(),
                    "empty" => (StatusCode::OK, String::new()).into_response(),
                    "nopkg" => (StatusCode::OK, page(json!([]))).into_response(),
                    "once" if seen == 0 => (StatusCode::OK, present("2021-01-04T09:00:00Z")).into_response(),
                    "once" => (StatusCode::NOT_FOUND, String::new()).into_response(),
                    date => (StatusCode::OK, present(date)).into_response(),
                }
            }
        })
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("http://{addr}/api/1.0"), log)
}

/// The ids the log shows asked, without the control requests of `controls`.
fn asked(log: &Log, controls: &[&str]) -> Vec<String> {
    log.lock().unwrap().iter().filter(|id| !controls.contains(&id.as_str())).cloned().collect()
}

/// How many control requests the log shows.
fn controls_asked(log: &Log, controls: &[&str]) -> usize {
    log.lock().unwrap().iter().filter(|id| controls.contains(&id.as_str())).count()
}

/// A held id's by-id answer, for the control requests.
const HELD: &str = "2021-01-04T09:00:00Z";

/// Hold `ids` as FTS notices of the package `(kind, period)`; the fetch id.
async fn hold_fts(db: &store::Db, kind: &str, period: &str, ids: &[&str]) -> i64 {
    db.record_fetch(&store::Fetch {
        source: "fts".into(),
        kind: kind.into(),
        period: period.into(),
        url: "u".into(),
        sha256: format!("{kind}{period}"),
        bytes: 1,
        fetched_at: 0,
        path: format!("fts/{kind}/{period}.zip"),
    })
    .await
    .unwrap();
    let fetch_id = db.current_packages("fts", kind, Some(period)).await.unwrap()[0].fetch_id;
    for id in ids {
        db.record_notice(
            &store::Notice {
                source: "fts".into(),
                publication_id: (*id).into(),
                content_hash: format!("h{id}"),
                profile: "fts:ocds-1.1".into(),
                declared_version: Some("1.1".into()),
                fetch_id,
                member_path: format!("{id}.json"),
                ingested_at: 0,
                published_at: None,
                dispatched_at: None,
            },
            &store::Parse::Pending,
        )
        .await
        .unwrap();
    }
    fetch_id
}

/// Issue 477 unit 3, end to end against a mock of the by-id endpoint: the job
/// asks exactly the ids missing below each year's highest (after a control
/// request of a held id), records absent / present / error, names the present
/// id's neighbours' package and its day's, stops at a checkpoint, resumes
/// asking only what is not decided, and the denominator closes once every id
/// is held or absent.
#[tokio::test]
async fn the_id_audit_asks_each_missing_id_once_and_resumes_from_its_ledger() {
    let archive = temp_dir("fts-id-audit");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    // 2021: 1..=8 issued; 3, 5, 6 missing. 2022: 1..=3, 2 missing. A non-sequence
    // id is ignored. The neighbours of 000005-2021 (4 and 7) sit in 2021-05.
    hold_fts(&db, "monthly", "2021-04", &["000001-2021", "000002-2021", "000004-2021"]).await;
    hold_fts(&db, "monthly", "2021-05", &["000007-2021", "000008-2021", "000008-2021-junk"]).await;
    hold_fts(&db, "monthly", "2022-01", &["000001-2022", "000003-2022"]).await;
    let controls = ["000008-2021", "000003-2022"];
    let (base, log) = fts_id_server(
        [
            ("000003-2021", "404"),
            ("000005-2021", "2021-07-27T09:00:00+01:00"),
            ("000006-2021", "400"),
            ("000002-2022", "nopkg"),
            ("000008-2021", HELD),
            ("000003-2022", HELD),
        ]
        .into(),
    )
    .await;
    let client = reqwest::Client::new();
    let pause = Duration::ZERO;

    // A dry run asks nothing and writes nothing, and counts what is due.
    let dry = audit_fts_ids(&db, &client, &base, pause, AuditOptions { dry_run: true, ..Default::default() }, || false, |_| {})
        .await
        .unwrap();
    assert_eq!((dry.missing, dry.due, dry.probed, dry.controls), (4, 4, 0, 0));
    assert!(log.lock().unwrap().is_empty(), "a dry run makes no request");
    assert!(db.publication_audits("fts").await.unwrap().is_empty());

    // Stopped after two answers: the two land in the ledger. The control is
    // asked first, and once more at the end to confirm the absent.
    let n = std::sync::atomic::AtomicUsize::new(0);
    let stopped = audit_fts_ids(
        &db,
        &client,
        &base,
        pause,
        AuditOptions::default(),
        || n.load(Ordering::Relaxed) >= 2,
        |_| {
            n.fetch_add(1, Ordering::Relaxed);
        },
    )
    .await
    .unwrap();
    assert!(stopped.stopped);
    assert_eq!(stopped.halted, None);
    assert_eq!((stopped.probed, stopped.controls), (2, 2));
    assert_eq!(
        *log.lock().unwrap(),
        ["000008-2021", "000003-2021", "000005-2021", "000008-2021"],
        "control, then (year, seq) order, then the closing control"
    );

    // The resume asks only the two never asked.
    let mut seen = Vec::new();
    let run = audit_fts_ids(&db, &client, &base, pause, AuditOptions::default(), || false, |p| {
        seen.push((p.id.to_owned(), p.verdict))
    })
    .await
    .unwrap();
    assert_eq!(seen, [("000006-2021".to_owned(), Verdict::Error), ("000002-2022".to_owned(), Verdict::Absent)]);
    assert_eq!(asked(&log, &controls).len(), 4, "no id asked twice");
    assert_eq!(run.halted, None);
    assert_eq!(run.absent_ids, ["000003-2021", "000002-2022"]);
    assert_eq!(run.error_ids, ["000006-2021"]);
    assert_eq!(run.present_ids.len(), 1);
    let present = &run.present_ids[0];
    assert_eq!(present.id, "000005-2021");
    assert_eq!(present.published_day.as_deref(), Some("2021-07-27"));
    assert_eq!(
        present.packages,
        ["monthly 2021-04", "monthly 2021-05", "monthly 2021-07"],
        "both neighbours' packages and its release month's (monthlies reach 2022-01)"
    );
    assert_eq!(run.enqueue.len(), 5, "three fetches, one process, one project: {:?}", run.enqueue);
    assert!(run.enqueue[0].contains(r#""period":"2021-04""#) && run.enqueue[0].contains(r#""refetch":true"#));
    let y21 = &run.years[0];
    assert_eq!((y21.highest, y21.held, y21.absent, y21.present, y21.errors, y21.published()), (8, 5, 1, 1, 1, 7));
    assert!(run.years[1].complete(), "2022: 000002 shown absent");
    assert_eq!(run.unaccounted, 2);
    assert!(!run.complete);
    assert!(run.summary().contains("4 missing id(s), 2 due, 2 probed"), "{}", run.summary());

    // The next run re-asks only the error; once it answers 404 and the present
    // id is held (a refetch recovered it), every year is complete, and the
    // dashboard's census agrees with the report.
    let row = db.publication_audits("fts").await.unwrap();
    assert_eq!(row.iter().find(|r| r.publication_id == "000006-2021").map(|r| r.http_status), Some(Some(400)));
    let (base, log) = fts_id_server([("000008-2021", HELD)].into()).await;
    hold_fts(&db, "monthly", "2021-07", &["000005-2021"]).await;
    let last = audit_fts_ids(&db, &client, &base, pause, AuditOptions::default(), || false, |_| {}).await.unwrap();
    assert_eq!(asked(&log, &controls), ["000006-2021"]);
    assert!(last.complete, "{}", last.summary());
    assert!(last.enqueue.is_empty() && last.present_ids.is_empty());
    let census = fts_id_census(&db).await.unwrap();
    assert_eq!(census, last.years);
    assert_eq!(census[0].published(), 6, "8 issued, 3 and 6 never published");
    let attempts = db.publication_audits("fts").await.unwrap();
    assert_eq!(attempts.iter().find(|r| r.publication_id == "000006-2021").map(|r| r.attempts), Some(2));

    // Closed years' stale absents are re-asked only on request.
    let again = audit_fts_ids(
        &db,
        &client,
        &base,
        pause,
        AuditOptions { recheck_absent_after_secs: Some(0), max_ids: Some(1), ..Default::default() },
        || false,
        |_| {},
    )
    .await
    .unwrap();
    assert_eq!((again.due, again.probed), (3, 1), "three absents due, capped to one ask");
    let _ = std::fs::remove_dir_all(&archive);
}

/// Issue 477 unit 3: five consecutive errors halt the run with the reason, and
/// what was answered stays recorded for the resume.
#[tokio::test]
async fn the_id_audit_halts_on_an_error_streak_and_keeps_what_it_learned() {
    let archive = temp_dir("fts-id-audit-halt");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    hold_fts(&db, "monthly", "2021-01", &["000001-2021", "000009-2021"]).await;
    // 2..=8 missing: 2 absent, then six 400s.
    let (base, log) = fts_id_server(
        [
            ("000002-2021", "404"),
            ("000003-2021", "400"),
            ("000004-2021", "400"),
            ("000005-2021", "400"),
            ("000006-2021", "400"),
            ("000007-2021", "400"),
            ("000008-2021", "400"),
            ("000009-2021", HELD),
        ]
        .into(),
    )
    .await;
    let run = audit_fts_ids(&db, &reqwest::Client::new(), &base, Duration::ZERO, AuditOptions::default(), || false, |_| {})
        .await
        .unwrap();
    let halted = run.halted.as_deref().expect("halted");
    assert!(halted.starts_with("5 consecutive errors, the last on 000007-2021: HTTP 400"), "{halted}");
    assert_eq!(asked(&log, &["000009-2021"]).len(), 6, "000008 is never asked");
    assert_eq!(db.publication_audits("fts").await.unwrap().len(), 6);
    assert_eq!(run.absent_ids, ["000002-2021"], "the closing control confirmed it");
    let _ = std::fs::remove_dir_all(&archive);
}

/// Issue 477 review: a non-error answer resets the error streak — four errors,
/// an absent, four more errors never halt, and every id is asked.
#[tokio::test]
async fn the_id_audit_error_streak_resets_on_a_decisive_answer() {
    let archive = temp_dir("fts-id-audit-streak");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    hold_fts(&db, "monthly", "2021-01", &["000001-2021", "000011-2021"]).await;
    let mut answers: HashMap<&'static str, &'static str> = [("000011-2021", HELD), ("000006-2021", "404")].into();
    for id in ["000002-2021", "000003-2021", "000004-2021", "000005-2021", "000007-2021", "000008-2021", "000009-2021", "000010-2021"] {
        answers.insert(id, "400");
    }
    let (base, log) = fts_id_server(answers).await;
    let run = audit_fts_ids(&db, &reqwest::Client::new(), &base, Duration::ZERO, AuditOptions::default(), || false, |_| {})
        .await
        .unwrap();
    assert_eq!(run.halted, None, "{}", run.summary());
    assert_eq!(asked(&log, &["000011-2021"]).len(), 9, "every missing id asked");
    assert_eq!((run.probed, run.absent, run.errors), (9, 1, 8));
    let _ = std::fs::remove_dir_all(&archive);
}

/// Issue 477 review: a 404 counts only while the endpoint serves a known id.
/// A wrong base (every request 404) halts on the first control and records
/// nothing; an endpoint that stops serving known ids mid-run has the absents
/// since the last passed control demoted to `error`, re-asked next run.
#[tokio::test]
async fn the_id_audit_records_no_absent_unless_a_held_id_answers() {
    let archive = temp_dir("fts-id-audit-control");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    hold_fts(&db, "monthly", "2021-01", &["000001-2021", "000005-2021"]).await;
    let client = reqwest::Client::new();

    // A wrong base: the route is not there, every request is a 404.
    let (base, log) = fts_id_server([("000005-2021", HELD)].into()).await;
    let run = audit_fts_ids(&db, &client, &format!("{base}/wrong"), Duration::ZERO, AuditOptions::default(), || false, |_| {})
        .await
        .unwrap();
    let halted = run.halted.as_deref().expect("halted");
    assert!(halted.starts_with("control: held id 000005-2021 answered absent (404)"), "{halted}");
    assert_eq!((run.probed, run.controls), (0, 1));
    assert!(log.lock().unwrap().is_empty());
    assert!(db.publication_audits("fts").await.unwrap().is_empty(), "nothing recorded absent");

    // The held id answers once, then the endpoint 404s everything: the three
    // absents are demoted by the closing control.
    let (base, log) = fts_id_server([("000005-2021", "once")].into()).await;
    let run = audit_fts_ids(&db, &client, &base, Duration::ZERO, AuditOptions::default(), || false, |_| {})
        .await
        .unwrap();
    let halted = run.halted.as_deref().expect("halted");
    assert!(halted.contains("3 absent answer(s) since the last passed control demoted to error"), "{halted}");
    assert_eq!(controls_asked(&log, &["000005-2021"]), 2);
    assert_eq!((run.probed, run.absent, run.errors), (3, 0, 3));
    assert!(run.absent_ids.is_empty());
    assert_eq!(run.error_ids, ["000002-2021", "000003-2021", "000004-2021"]);
    let rows = db.publication_audits("fts").await.unwrap();
    assert!(
        rows.iter().all(|r| r.verdict == "error" && r.detail.as_deref().is_some_and(|d| d.starts_with("absent, not confirmed: control"))),
        "{rows:?}"
    );
    assert!(!run.complete);
    let _ = std::fs::remove_dir_all(&archive);
}

/// Issue 477 review: a missing id an archived, quarantined member carries is
/// recorded `quarantined` without a request and counts as accounted for, so the
/// denominator can close (a `present` verdict never would: the refetch dedups).
#[tokio::test]
async fn the_id_audit_accounts_for_quarantined_members_without_asking() {
    let archive = temp_dir("fts-id-audit-quarantine");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    let fetch_id = hold_fts(&db, "monthly", "2021-01", &["000001-2021", "000004-2021"]).await;
    db.insert_quarantine(&store::Quarantined {
        fetch_id,
        member_path: "000003-2021~0a1b2c3d.json".into(),
        content_hash: "q".into(),
        profile: Some("fts:ocds-1.1".into()),
        reason: "parse-error".into(),
        detail: None,
        first_seen: 0,
    })
    .await
    .unwrap();
    let (base, log) = fts_id_server([("000004-2021", HELD)].into()).await;
    let client = reqwest::Client::new();

    let dry = audit_fts_ids(&db, &client, &base, Duration::ZERO, AuditOptions { dry_run: true, ..Default::default() }, || false, |_| {})
        .await
        .unwrap();
    assert_eq!((dry.missing, dry.quarantined, dry.due), (2, 1, 1));
    assert_eq!(dry.quarantined_ids, ["000003-2021"]);
    assert!(db.publication_audits("fts").await.unwrap().is_empty(), "a dry run writes nothing");

    let run = audit_fts_ids(&db, &client, &base, Duration::ZERO, AuditOptions::default(), || false, |_| {}).await.unwrap();
    assert_eq!(asked(&log, &["000004-2021"]), ["000002-2021"], "the quarantined id is never asked");
    assert_eq!(run.quarantined_ids, ["000003-2021"]);
    let y21 = &run.years[0];
    assert_eq!((y21.held, y21.absent, y21.quarantined, y21.published()), (2, 1, 1, 3), "4 issued, 1 never published");
    assert!(run.complete, "held + quarantined + absent == highest: {}", run.summary());
    let rows = db.publication_audits("fts").await.unwrap();
    assert_eq!(rows.iter().find(|r| r.publication_id == "000003-2021").map(|r| r.verdict.as_str()), Some("quarantined"));
    assert_eq!(fts_id_census(&db).await.unwrap(), run.years, "the dashboard reads the same");
    let _ = std::fs::remove_dir_all(&archive);
}
