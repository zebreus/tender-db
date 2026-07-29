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
