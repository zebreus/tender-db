//! Issue 16 — the `/admin` API drives ingestion end to end.
//!
//! One test, run sequentially, because the operator secret lives in a
//! process-global env var: it walks the whole surface (404 when unset, 403 on a
//! bad secret, 202 on enqueue), then enqueues a real process job over a
//! fixture-built package through the running Supervisor, polls until it lands,
//! and checks the canonical counts — the same in-process path production uses.
//!
//! Server-only, like the API test: `cargo test --features tender-db/server`.
#![cfg(feature = "server")]

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};
use store::Db;
use tender_db::admin;
use tender_db::supervisor::Supervisor;

const SECRET: &str = "correct horse battery staple";
const FIXTURE: &str = "../ingest/tests/fixtures/eforms/cn-16-00494343-2026.xml";

/// A .tar.gz shaped like a TED daily: one eForms member the processor parses.
fn write_package(path: &Path, member: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let gz =
        flate2::write::GzEncoder::new(std::fs::File::create(path).unwrap(), flate2::Compression::fast());
    let mut tar = tar::Builder::new(gz);
    let mut header = tar::Header::new_gnu();
    header.set_size(member.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    tar.append_data(&mut header, "20260101_1/notice.xml", member).unwrap();
    tar.into_inner().unwrap().finish().unwrap();
}

struct Harness {
    http: reqwest::Client,
    base: String,
    db: Arc<Db>,
    dir: std::path::PathBuf,
}

impl Drop for Harness {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

impl Harness {
    async fn start() -> Harness {
        let dir = std::env::temp_dir().join(format!("tender-db-admin-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let member = std::fs::read(FIXTURE).unwrap_or_else(|e| panic!("read {FIXTURE}: {e}"));
        write_package(&dir.join("ted/daily/2026-00136.tar.gz"), &member);

        let db = Arc::new(Db::open(dir.join("test.db").to_str().unwrap()).await.unwrap());
        db.record_fetch(&store::Fetch {
            source: "ted".into(),
            kind: "daily".into(),
            period: "2026-00136".into(),
            url: "https://example.invalid/pkg".into(),
            sha256: "aa".into(),
            bytes: 1,
            fetched_at: 0,
            path: "ted/daily/2026-00136.tar.gz".into(),
        })
        .await
        .unwrap();

        let sup = Arc::new(Supervisor::new(db.clone(), dir.clone(), reqwest::Client::new()));
        // Worker only — no scheduler, so the test enqueues everything itself.
        sup.clone().spawn_worker();

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let _ = axum::serve(listener, admin::router(sup)).await;
        });

        Harness { http: reqwest::Client::new(), base: format!("http://127.0.0.1:{port}"), db, dir }
    }

    async fn get(&self, secret: Option<&str>) -> reqwest::Response {
        let mut r = self.http.get(format!("{}/admin/jobs", self.base));
        if let Some(s) = secret {
            r = r.header("x-admin-secret", s);
        }
        r.send().await.unwrap()
    }

    async fn enqueue(&self, secret: Option<&str>, body: Value) -> reqwest::Response {
        let mut r = self.http.post(format!("{}/admin/jobs", self.base)).json(&body);
        if let Some(s) = secret {
            r = r.header("x-admin-secret", s);
        }
        r.send().await.unwrap()
    }

    /// Poll the queue until it is idle and at least `min_runs` runs are logged.
    async fn drain(&self, min_runs: usize) -> Value {
        for _ in 0..200 {
            let state: Value = self.get(Some(SECRET)).await.json().await.unwrap();
            let idle = state["current"].is_null();
            let runs = state["recent"].as_array().map(Vec::len).unwrap_or(0);
            if idle && runs >= min_runs {
                return state;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("supervisor did not reach {min_runs} finished run(s)");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn admin_api_drives_ingestion_end_to_end() {
    let h = Harness::start().await;

    // --- auth: unset ⇒ 404 (the feature is simply not there) ----------------
    unsafe { std::env::remove_var("TENDER_ADMIN_SECRET") };
    assert_eq!(h.get(None).await.status().as_u16(), 404, "unset secret hides the surface");

    // --- auth: set, but wrong/missing ⇒ 403 ---------------------------------
    unsafe { std::env::set_var("TENDER_ADMIN_SECRET", SECRET) };
    assert_eq!(h.get(None).await.status().as_u16(), 403, "no header is forbidden");
    assert_eq!(h.get(Some("nope")).await.status().as_u16(), 403, "wrong secret is forbidden");
    assert_eq!(h.enqueue(Some("nope"), json!({"kind":"project"})).await.status().as_u16(), 403);

    // A correct secret sees an idle supervisor.
    let state: Value = h.get(Some(SECRET)).await.json().await.unwrap();
    assert!(state["current"].is_null());
    assert!(state["queued"].as_array().unwrap().is_empty());

    // --- enqueue a process job over the fixture package ----------------------
    let accepted =
        h.enqueue(Some(SECRET), json!({"kind":"process","source":"ted","package_kind":"daily"})).await;
    assert_eq!(accepted.status().as_u16(), 202, "the job is accepted");
    let body: Value = accepted.json().await.unwrap();
    assert_eq!(body["enqueued"].as_array().unwrap().len(), 1);

    // --- poll until it lands, and check the counts arrived -------------------
    let state = h.drain(1).await;
    let run = &state["recent"][0];
    assert_eq!(run["kind"], "process");
    assert_eq!(run["outcome"], "ok", "the process run succeeded: {run}");
    assert!(run["counts"].as_str().unwrap().contains("notices"));

    let notices: i64 =
        h.db.notice_counts_by_profile().await.unwrap().iter().map(|(_, n)| n).sum();
    assert!(notices > 0, "the process job wrote notices into the DB");

    // --- project, then confirm a Tender exists ------------------------------
    assert_eq!(h.enqueue(Some(SECRET), json!({"kind":"project"})).await.status().as_u16(), 202);
    h.drain(2).await;
    assert!(!h.db.list_tenders(10).await.unwrap().is_empty(), "projection built a Tender");

    // --- DELETE an unknown queued job ⇒ 404 (and the route is gated) --------
    let unknown =
        h.http.delete(format!("{}/admin/jobs/999999", h.base)).header("x-admin-secret", SECRET);
    assert_eq!(unknown.send().await.unwrap().status().as_u16(), 404);
    let ungated = h.http.delete(format!("{}/admin/jobs/1", h.base)).send().await.unwrap();
    assert_eq!(ungated.status().as_u16(), 403, "DELETE is gated too");

    unsafe { std::env::remove_var("TENDER_ADMIN_SECRET") };
}
