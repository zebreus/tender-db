//! Fetcher integration tests against a local HTTP fixture server — the
//! archive-immutability and idempotency invariants, no network involved.

use axum::routing::get;
use ingest::fetch::{fetch, Outcome, Target};
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
