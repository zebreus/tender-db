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

/// Requests seen per page key: `""` for a first page, `p2`/`p3`… for cursors.
type Hits = Arc<Mutex<HashMap<String, usize>>>;

/// Serves `/api/1.0/ocdsReleasePackages` keyed by the `cursor` query param: no
/// cursor → `pages[0]`, `cursor=p2` → `pages[1]`, … `links.next` is rewritten to
/// point back at this server; the final page carries none (the real API's
/// shape). `throttle` is `(how many requests for page 2 are answered
/// `429 Retry-After: <secs>`, that Retry-After value)` — the live limiter's
/// answer at test speed; `usize::MAX` never lets page 2 through, which is how
/// the exhaustion arm is driven. Returns the API base (`http://…/api/1.0`), the
/// shape `fts::BASE` has in production.
async fn fts_server(pages: Vec<Value>, throttle: (usize, u64)) -> (String, Hits) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/api/1.0", listener.local_addr().unwrap());
    let hits: Hits = Arc::new(Mutex::new(HashMap::new()));
    let pages = Arc::new(pages);
    let (throttle_times, retry_after) = throttle;
    let throttled = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let app = axum::Router::new().route("/api/1.0/ocdsReleasePackages", {
        let (base, hits, throttled) = (base.clone(), hits.clone(), throttled.clone());
        get(move |Query(q): Query<HashMap<String, String>>| {
            let (base, hits, pages, throttled) = (base.clone(), hits.clone(), pages.clone(), throttled.clone());
            async move {
                let cursor = q.get("cursor").cloned().unwrap_or_default();
                let idx = match cursor.strip_prefix('p') {
                    Some(n) => n.parse::<usize>().unwrap() - 1,
                    None => 0,
                };
                *hits.lock().unwrap().entry(cursor).or_default() += 1;
                if idx == 1 && throttled.fetch_add(1, Ordering::SeqCst) < throttle_times {
                    return (
                        StatusCode::TOO_MANY_REQUESTS,
                        [(header::RETRY_AFTER, retry_after.to_string())],
                        "Rate limit of 12 exceeded. Please retry after 120 seconds.",
                    )
                        .into_response();
                }
                let mut page = pages[idx].clone();
                if idx + 1 < pages.len() {
                    page["links"] = json!({ "next": format!(
                        "{base}/ocdsReleasePackages?limit=100&updatedFrom={}&updatedTo={}&cursor=p{}",
                        q.get("updatedFrom").cloned().unwrap_or_default(),
                        q.get("updatedTo").cloned().unwrap_or_default(),
                        idx + 2
                    ) });
                } else {
                    page.as_object_mut().unwrap().remove("links");
                }
                (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, "application/json")],
                    serde_json::to_string_pretty(&page).unwrap(),
                )
                    .into_response()
            }
        })
    });
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (base, hits)
}

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

/// The ids in the two fixture pages, sorted — the members the day's zip must hold.
const FTS_IDS: [&str; 8] = [
    "083253-2026", "083256-2026", "083257-2026", "083608-2026",
    "083655-2026", "083662-2026", "083664-2026", "083674-2026",
];

/// One daily window: the walk follows `links.next` through a 429 to the final
/// page, assembles ONE zip with one member per release id, registers it, and
/// leaves no staging behind. A second call is `Unchanged` without HTTP, and a
/// `refetch` re-walk of unchanged pages hashes equal — the assembled zip is
/// byte-deterministic, so the registry never versions a package that did not
/// change.
#[tokio::test]
async fn fts_window_follows_links_next_into_one_zip() {
    let (base, hits) = fts_server(vec![fixture_page(FTS_P1), fixture_page(FTS_P2)], (1, 1)).await;
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
    let outcome = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, |day, pages, releases| {
        progress.push((day.to_owned(), pages, releases));
    })
    .await
    .unwrap();
    let waited = started.elapsed();
    assert_eq!(outcome, Outcome::Fetched);
    assert_eq!(progress, [("2026-09-03".to_owned(), 1, 5), ("2026-09-03".to_owned(), 2, 8)]);
    {
        let hits = hits.lock().unwrap();
        assert_eq!(hits.get(""), Some(&1), "first page once");
        assert_eq!(hits.get("p2"), Some(&2), "page 2: the 429, then the page");
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
    let expected: Vec<String> = FTS_IDS.iter().map(|id| format!("{id}.json")).collect();
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
    let again = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, |_, _, _| {}).await.unwrap();
    assert_eq!(again, Outcome::Unchanged);
    assert_eq!(hits.lock().unwrap().get(""), Some(&1));

    // Refetch of unchanged pages: re-walked, re-assembled, hashes equal — one row.
    let refetched = fetch_fts(&db, &client, &archive, &t, true, Duration::ZERO, |_, _, _| {}).await.unwrap();
    assert_eq!(refetched, Outcome::Unchanged, "a deterministic zip never versions itself");
    assert_eq!(hits.lock().unwrap().get(""), Some(&2));
    assert_eq!(db.latest_fetch("fts", "daily", "2026-09-03").await.unwrap().unwrap(), row);
    assert!(!archive.join("fts/daily/2026-09-03.pages").exists());
    assert!(!archive.join("fts/daily/2026-09-03-v2.zip").exists());

    let _ = std::fs::remove_dir_all(&archive);
}

/// D8 resume: a walk interrupted after page 1 (five throttled attempts, a
/// restart) left the page and `cursor.json` in the staging dir. The next run
/// continues at the cursor's `next` — page 1 is NOT requested again — and the
/// finished zip holds the releases of both pages.
#[tokio::test]
async fn fts_window_resumes_from_staged_pages() {
    let (base, hits) = fts_server(vec![fixture_page(FTS_P1), fixture_page(FTS_P2)], (0, 0)).await;
    let archive = temp_dir("fts-resume");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    let client = reqwest::Client::new();
    let t = fts::day(&base, (2026, 9, 3));

    // What the earlier run left: page 1 verbatim, and the cursor pointing at page 2.
    let staging = archive.join("fts/daily/2026-09-03.pages");
    std::fs::create_dir_all(&staging).unwrap();
    std::fs::copy(FTS_P1, staging.join("2026-09-03-p001.json")).unwrap();
    let next = format!(
        "{base}/ocdsReleasePackages?limit=100&updatedFrom=2026-09-02T22:00:00\
         &updatedTo=2026-09-03T23:59:59&cursor=p2"
    );
    std::fs::write(
        staging.join("cursor.json"),
        json!({ "day": "2026-09-03", "page": 1, "next": next, "done": [] }).to_string(),
    )
    .unwrap();

    let outcome = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, |_, _, _| {}).await.unwrap();
    assert_eq!(outcome, Outcome::Fetched);
    {
        let hits = hits.lock().unwrap();
        assert_eq!(hits.get(""), None, "page 1 was on disk: never asked for again");
        assert_eq!(hits.get("p2"), Some(&1));
    }
    let names: Vec<String> =
        zip_members(&archive.join("fts/daily/2026-09-03.zip")).into_iter().map(|(n, _)| n).collect();
    assert_eq!(names, FTS_IDS.iter().map(|id| format!("{id}.json")).collect::<Vec<_>>());
    assert!(!staging.exists());
    assert!(db.latest_fetch("fts", "daily", "2026-09-03").await.unwrap().is_some());

    let _ = std::fs::remove_dir_all(&archive);
}

/// The throttle EXHAUSTION arm (plan D8): five 429s and the walk gives up with
/// its staging intact, so the next tick resumes at the page that could not be
/// had rather than restarting the window. Pinned because the retry policy is now
/// shared with the plain fetcher — a change to the attempt count would otherwise
/// only show as a slower production walk (issue 342 review, lens "tests").
#[tokio::test]
async fn fts_gives_up_after_five_throttled_attempts_with_staging_intact() {
    // Retry-After: 0 — the policy under test is the attempt count, not the wait.
    let (base, hits) =
        fts_server(vec![fixture_page(FTS_P1), fixture_page(FTS_P2)], (usize::MAX, 0)).await;
    let archive = temp_dir("fts-throttled");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    let client = reqwest::Client::new();
    let t = fts::day(&base, (2026, 9, 3));

    let err = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, |_, _, _| {}).await.unwrap_err();
    assert!(matches!(err, ingest::fetch::Error::Throttled { .. }), "{err}");
    assert_eq!(hits.lock().unwrap().get("p2"), Some(&5), "five attempts, then it stops");

    let staging = archive.join("fts/daily/2026-09-03.pages");
    assert!(staging.join("2026-09-03-p001.json").exists(), "page 1 stays staged for the resume");
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

    let outcome = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, |_, _, _| {}).await.unwrap();
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
/// a refetch must re-walk the windows its stale `done` list claims, or a package
/// whose content changed would be re-registered from the old pages (issue 342
/// review, lens "fetcher").
#[tokio::test]
async fn a_refetch_discards_staging_older_than_the_registered_package() {
    let days: HashMap<String, Vec<Value>> =
        HashMap::from([("2026-09-03".to_owned(), vec![release("083253-2026", "fine")])]);
    let (base, hits) = fts_day_server(days).await;
    let archive = temp_dir("fts-debris");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    let client = reqwest::Client::new();
    let t = fts::day(&base, (2026, 9, 3));
    fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, |_, _, _| {}).await.unwrap();
    let registered = db.latest_fetch("fts", "daily", "2026-09-03").await.unwrap().unwrap();

    // Debris: a cursor claiming the day is done, stamped BEFORE the landing.
    let staging = archive.join("fts/daily/2026-09-03.pages");
    std::fs::create_dir_all(&staging).unwrap();
    std::fs::write(
        staging.join("cursor.json"),
        r#"{"day":"","page":0,"next":null,"done":["2026-09-03"]}"#,
    )
    .unwrap();
    let stale = std::time::UNIX_EPOCH + Duration::from_secs(registered.fetched_at as u64 - 3600);
    std::fs::File::options().write(true).open(staging.join("cursor.json")).unwrap()
        .set_modified(stale).unwrap();

    let before = hits.lock().unwrap().values().sum::<usize>();
    let outcome = fetch_fts(&db, &client, &archive, &t, true, Duration::ZERO, |_, _, _| {}).await.unwrap();
    assert_eq!(hits.lock().unwrap().values().sum::<usize>(), before + 1, "the day was re-walked");
    assert_eq!(outcome, Outcome::Unchanged, "same bytes, so no new version");
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
    fetch_fts(&db, &client, &archive, &fts::day(&base, (2026, 1, 1)), false, Duration::ZERO, |_, _, _| {})
        .await
        .unwrap();

    let first = probe_fts_daily(&db, &client, &archive, &base, (2026, 2, 20), Duration::ZERO, |_, _| {})
        .await
        .unwrap();
    assert_eq!(first.len(), ingest::fts::PROBE_DAY_CAP, "one month of days, then it stops");
    assert_eq!(first.first().map(|(p, _)| p.as_str()), Some("2026-01-02"));
    assert_eq!(ingest::fetch::latest_fts_day(&db).await.unwrap(), Some((2026, 2, 1)));

    let second = probe_fts_daily(&db, &client, &archive, &base, (2026, 2, 20), Duration::ZERO, |_, _| {})
        .await
        .unwrap();
    assert_eq!(second.len(), 19, "the remainder, next tick");
    assert_eq!(ingest::fetch::latest_fts_day(&db).await.unwrap(), Some((2026, 2, 20)));

    let _ = std::fs::remove_dir_all(&archive);
}

/// A page that is not a release package fails the walk with the staging intact
/// (nothing lands, nothing registers), so the next run resumes rather than
/// archiving garbage under the package's name.
#[tokio::test]
async fn fts_malformed_page_fails_with_staging_intact() {
    let (base, _hits) = fts_server(vec![fixture_page(FTS_P1), json!({ "error": "not a package" })], (0, 0)).await;
    let archive = temp_dir("fts-malformed");
    let db = store::Db::open(archive.join("test.db").to_str().unwrap()).await.unwrap();
    let client = reqwest::Client::new();
    let t = fts::day(&base, (2026, 9, 3));

    let err = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, |_, _, _| {}).await.unwrap_err();
    assert!(matches!(err, ingest::fetch::Error::Malformed(_)), "{err}");
    let staging = archive.join("fts/daily/2026-09-03.pages");
    assert!(staging.join("2026-09-03-p001.json").exists(), "page 1 stays staged");
    let cursor: Value = serde_json::from_slice(&std::fs::read(staging.join("cursor.json")).unwrap()).unwrap();
    assert_eq!(cursor["page"], 1);
    assert!(cursor["next"].as_str().unwrap().contains("cursor=p2"));
    assert!(!archive.join("fts/daily/2026-09-03.zip").exists());
    assert!(db.latest_fetch("fts", "daily", "2026-09-03").await.unwrap().is_none());

    let _ = std::fs::remove_dir_all(&archive);
}

/// Serves one page per requested window, keyed by the `updatedTo` civil day:
/// the releases in `days` for that day (an absent day answers `releases: []`,
/// the real API's empty window). Records the `updatedFrom` seen per day.
/// Returns the API base, as [`fts_server`] does.
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
/// 2 h into the previous day; and within one package the first occurrence of a
/// re-served id wins.
#[tokio::test]
async fn fts_walk_forward_catches_up_a_multi_day_gap() {
    let days: HashMap<String, Vec<Value>> = HashMap::from([
        ("2026-09-03".to_owned(), vec![release("000003-2026", "third")]),
        // Day 4's window (from 2026-09-03T22:00:00) re-serves day 3's late notice,
        // and — as pages newest-first do — lists an id twice; the first wins.
        (
            "2026-09-04".to_owned(),
            vec![release("000004-2026", "fourth"), release("000004-2026", "fourth, stale copy"), release("000003-2026", "third")],
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

    // Day 4: both ids, the overlap's re-served release included; the duplicated
    // id holds its FIRST occurrence.
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
    let outcome = fetch_fts(&db, &client, &archive, &t, false, Duration::ZERO, |day, _, _| {
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
