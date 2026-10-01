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
use ingest::fetch::{fetch_fts, probe_fts_daily};
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
    let outcome = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, || false, |day, pages, releases| {
        progress.push((day.to_owned(), pages, releases));
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
    let again = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, || false, |_, _, _| {}).await.unwrap();
    assert_eq!(again, Outcome::Unchanged);
    assert_eq!(log.lock().unwrap().len(), 4);

    // Refetch of unchanged pages: re-walked, re-assembled, hashes equal — one row.
    let refetched = fetch_fts(&db, &client, &archive, &t, true, Duration::ZERO, || false, |_, _, _| {}).await.unwrap();
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

    let err = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, || false, |_, _, _| {}).await.unwrap_err();
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

    let outcome = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, || false, |_, _, _| {}).await.unwrap();
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
    fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, || false, |_, _, _| {}).await.unwrap();
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
    let outcome = fetch_fts(&db, &client, &archive, &t, true, Duration::ZERO, || false, |_, _, _| {}).await.unwrap();
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
    fetch_fts(&db, &client, &archive, &fts::day(&base, (2026, 1, 1)), false, Duration::ZERO, || false, |_, _, _| {})
        .await
        .unwrap();

    let cap = ingest::fts::PROBE_DAY_CAP;
    assert!(cap < 50);
    let mut walked = 0;
    let mut ticks = 0;
    while ingest::fetch::latest_fts_day(&db).await.unwrap() != Some((2026, 2, 20)) {
        let tick = probe_fts_daily(&db, &client, &archive, &base, (2026, 2, 20), Duration::ZERO, |_, _| {})
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

    let err = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, || false, |_, _, _| {}).await.unwrap_err();
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
/// only `end`; a gap of missed ticks is caught up day by day; an EMPTY window
/// lands a 0-member zip so the watermark advances; the daily window reaches
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

    // No FTS daily on record: only `end`, never a full-archive backfill.
    let seeded = probe_fts_daily(&db, &client, &archive, &base, (2026, 9, 3), Duration::ZERO, |_, _| {})
        .await
        .unwrap();
    assert_eq!(seeded, vec![("2026-09-03".into(), Outcome::Fetched)]);

    // Three missed ticks (4, 5, 6): one walk catches up the lot, in order.
    let touched = Arc::new(Mutex::new(Vec::<String>::new()));
    let t2 = touched.clone();
    let caught = probe_fts_daily(&db, &client, &archive, &base, (2026, 9, 6), Duration::ZERO, |p, _| {
        t2.lock().unwrap().push(p.to_owned());
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
        assert_eq!(hits.get("2026-09-03 from 2026-09-02T22:00:00"), Some(&1));
        assert_eq!(hits.get("2026-09-04 from 2026-09-03T22:00:00"), Some(&1));
        assert_eq!(hits.get("2026-09-05 from 2026-09-04T22:00:00"), Some(&1));
        assert_eq!(hits.get("2026-09-06 from 2026-09-05T22:00:00"), Some(&1));
        assert_eq!(hits.values().sum::<usize>(), 4, "one request per day, none repeated");
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

    // The empty Saturday: a valid 0-member zip, registered, walkable, and the
    // watermark moved past it.
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

    // Idempotent: the watermark is current, nothing to walk.
    let again = probe_fts_daily(&db, &client, &archive, &base, (2026, 9, 6), Duration::ZERO, |_, _| {})
        .await
        .unwrap();
    assert!(again.is_empty());
    // The SUM, not the key count: a probe that re-requested the same four days
    // would leave four keys and eight requests (issue 342 review, lens "tests").
    assert_eq!(
        hits.lock().unwrap().values().sum::<usize>(),
        4,
        "no HTTP for a current watermark"
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
    let outcome = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, || false, |day, _, _| {
        days_seen.push(day.to_owned());
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
                let mut releases: Vec<Value> = window.iter().take(limit).map(|r| r.release.clone()).collect();
                let mut next = window.get(limit).map(|r| r.key);
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

/// 2021-05-07 as the probes measured it (issue 477): 152 releases,
/// 009911–010062, in the hours 06, 08–18, 21 and 22 with the real hourly counts.
/// The keys of 009947, 009955 and 009962 (one process, `0292a9`) sit near
/// 261,858 while every other key sits near 600,000 — so the cursor after page 1
/// is 009962's key and page 2 holds those three rows only.
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
/// below is the measured server, and a walker that follows the cursor loses 49.
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

    let outcome = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, || false, |_, _, _| {}).await.unwrap();
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
        fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, || false, |_, _, _| {}),
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
/// 'updatedFrom'`, measured), so the walk never asks one: a span that is still
/// full at two seconds fails LOUD as malformed, with its staging intact for a
/// person to look at, and nothing lands.
#[tokio::test]
async fn a_full_two_second_span_fails_loud_with_staging_intact() {
    let ten = ingest::fetch::days_from_civil(2026, 9, 3) * 86_400 + 10 * 3_600;
    let rows: Vec<Keyed> = (1..=120).map(|n| keyed(&format!("{n:06}-2026"), ten, 500_000 + n)).collect();
    let (base, log) = fts_keyset_server(rows, Quirks::default()).await;
    let archive = temp_dir("fts-two-seconds");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    let client = reqwest::Client::new();
    let t = fts::day(&base, (2026, 9, 3));

    let err = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, || false, |_, _, _| {}).await.unwrap_err();
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

    let outcome = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, || false, |_, _, _| {}).await.unwrap();
    assert_eq!(outcome, Outcome::Fetched);
    assert_eq!(member_ids(&archive.join(&t.rel_path)), ["000001-2026", "000002-2026", "000003-2026", "000004-2026", "000005-2026"]);
    assert_eq!(cursor_requests(&log), 0);
    assert_eq!(spans_asked(&log).len(), 3, "the day, then its two halves");

    let _ = std::fs::remove_dir_all(&archive);
}

/// The staged span pages are the resume state (issue 450's stop, issue 477's
/// walk). A walk stopped after its first request lands nothing; the next fetch
/// of the same target never asks that span again, walks the rest, and lands
/// the whole day.
#[tokio::test]
async fn a_stopped_split_walk_resumes_without_asking_a_staged_span_again() {
    let (base, log) = fts_keyset_server(day_2021_05_07(), Quirks::default()).await;
    let archive = temp_dir("fts-split-resume");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    let client = reqwest::Client::new();
    let t = fts::day(&base, (2021, 5, 7));

    let asked = std::sync::atomic::AtomicUsize::new(0);
    let stop = || asked.fetch_add(1, Ordering::SeqCst) >= 1;
    let outcome = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, stop, |_, _, _| {}).await.unwrap();
    assert_eq!(outcome, Outcome::Stopped);
    assert_eq!(log.lock().unwrap().len(), 1, "one request, then the stop");
    let staging = archive.join("fts/daily/2021-05-07.pages");
    assert_eq!(std::fs::read_dir(&staging).unwrap().count(), 1, "the one page stays staged");
    assert!(!archive.join(&t.rel_path).exists(), "nothing lands");
    assert!(db.latest_fetch("fts", "daily", "2021-05-07").await.unwrap().is_none(), "nothing registers");

    let outcome = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, || false, |_, _, _| {}).await.unwrap();
    assert_eq!(outcome, Outcome::Fetched);
    let asked = spans_asked(&log);
    assert_eq!(asked.iter().collect::<HashSet<_>>().len(), asked.len(), "no span asked twice: {asked:?}");
    assert_eq!(asked[0], ("2021-05-06T22:00:00".to_owned(), "2021-05-07T23:59:59".to_owned()));
    assert_eq!(cursor_requests(&log), 0);
    assert_eq!(member_ids(&archive.join(&t.rel_path)), ids_2021_05_07());
    assert!(!staging.exists(), "staging removed after landing");

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

    let outcome = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, || false, |_, _, _| {}).await.unwrap();
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
/// walk now starts the day after the later of the newest daily and the last
/// day of the newest monthly.
#[tokio::test]
async fn the_walk_forward_starts_after_the_newest_monthly_not_at_end() {
    let (base, _log) = fts_keyset_server(Vec::new(), Quirks::default()).await;
    let client = reqwest::Client::new();
    let days = |r: Vec<(String, Outcome)>| r.into_iter().map(|(p, _)| p).collect::<Vec<_>>();

    // A monthly through 2026-08 and no daily: September from its first day.
    let archive = temp_dir("fts-seam");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    db.record_fetch(&registry_row("fts", "monthly", "2026-08")).await.unwrap();
    let walked = probe_fts_daily(&db, &client, &archive, &base, (2026, 9, 6), Duration::ZERO, |_, _| {}).await.unwrap();
    assert_eq!(days(walked), ["2026-09-01", "2026-09-02", "2026-09-03", "2026-09-04", "2026-09-05", "2026-09-06"]);
    let _ = std::fs::remove_dir_all(&archive);

    // A daily OLDER than the newest monthly: the monthly's end wins.
    let archive = temp_dir("fts-seam-older-daily");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    db.record_fetch(&registry_row("fts", "daily", "2026-07-15")).await.unwrap();
    db.record_fetch(&registry_row("fts", "monthly", "2026-08")).await.unwrap();
    let walked = probe_fts_daily(&db, &client, &archive, &base, (2026, 9, 2), Duration::ZERO, |_, _| {}).await.unwrap();
    assert_eq!(days(walked), ["2026-09-01", "2026-09-02"]);
    let _ = std::fs::remove_dir_all(&archive);

    // A daily NEWER than the newest monthly: the daily's next day, as before.
    let archive = temp_dir("fts-seam-newer-daily");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    db.record_fetch(&registry_row("fts", "monthly", "2026-08")).await.unwrap();
    db.record_fetch(&registry_row("fts", "daily", "2026-09-10")).await.unwrap();
    let walked = probe_fts_daily(&db, &client, &archive, &base, (2026, 9, 12), Duration::ZERO, |_, _| {}).await.unwrap();
    assert_eq!(days(walked), ["2026-09-11", "2026-09-12"]);
    let _ = std::fs::remove_dir_all(&archive);
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

    let err = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, || false, |_, _, _| {}).await.unwrap_err();
    assert!(matches!(err, ingest::fetch::Error::Unsupported(_)), "{err}");
    assert!(log.lock().unwrap().is_empty(), "refused before any request");
    assert!(!archive.join(format!("fts/monthly/{y:04}-{m:02}.pages")).exists(), "nothing staged");
    assert!(db.latest_fetch("fts", "monthly", &t.period).await.unwrap().is_none());

    let _ = std::fs::remove_dir_all(&archive);
}

/// One notice id, two releases (issue 477): `038018-2025` is a `tenderUpdate`
/// on the old procurement and an `award,contract` on the new one, under two
/// ocids. Within ONE package the assembler used to keep the first and drop the
/// second without trace. Now each distinct release is a member — `<id>.json`,
/// then `<id>~<hash8>.json` — while a byte-identical repeat still collapses;
/// and the processor stores both as notices of the same publication id.
#[tokio::test]
async fn one_id_carried_by_two_releases_keeps_both_and_a_repeat_collapses() {
    let update = json!({ "id": "038018-2025", "ocid": "ocds-h6vhtk-04a001", "tag": ["tenderUpdate"], "tender": { "title": "old" } });
    let award = json!({ "id": "038018-2025", "ocid": "ocds-h6vhtk-04b002", "tag": ["award", "contract"], "awards": [] });
    let days: HashMap<String, Vec<Value>> = HashMap::from([(
        "2025-04-10".to_owned(),
        vec![release("038019-2025", "next"), update.clone(), award, update],
    )]);
    let (base, _hits) = fts_day_server(days).await;
    let archive = temp_dir("fts-one-id-two");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    let client = reqwest::Client::new();
    let t = fts::day(&base, (2025, 4, 10));
    fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, || false, |_, _, _| {}).await.unwrap();

    let members = zip_members(&archive.join(&t.rel_path));
    let names: Vec<&str> = members.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names.len(), 3, "two releases of one id, one other; the repeat collapsed: {names:?}");
    assert_eq!(names[0], "038018-2025.json");
    let second = &members[1];
    assert_eq!(second.0, format!("038018-2025~{}.json", &ingest::sha256_hex(&second.1)[..8]));
    assert_eq!(names[2], "038019-2025.json");
    let first: Value = serde_json::from_slice(&members[0].1).unwrap();
    let other: Value = serde_json::from_slice(&second.1).unwrap();
    assert_eq!(first["releases"][0]["ocid"], "ocds-h6vhtk-04a001", "the first served keeps the plain name");
    assert_eq!(other["releases"][0]["ocid"], "ocds-h6vhtk-04b002");

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
