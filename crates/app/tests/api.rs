//! Issue 05 — the public API end to end.
//!
//! The real router, over a real Turso file, ingested from the real fixture
//! notices, talked to over a real socket. Nothing here is a mock: the point is
//! that the REST shapes, the filters, the cursor and the SSE protocol hold
//! together against the canonical layer as issue 04 actually built it.
//!
//! The API only exists in the `server` build, so this whole file does too:
//! `cargo test --workspace --features tender-db/server` is the command that
//! runs it (the same feature `nix flake check`'s clippy gate uses).
#![cfg(feature = "server")]

use ingest::{eforms, profile, project};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use store::{Db, Notice, Parse};
use tender_db::v1;

const SOURCE: &str = "ted";
const FIXTURES: &str = "../ingest/tests/fixtures";

/// The chain fixture: one real Maltese procedure, four notices, one Tender.
const CHAIN: [&str; 4] = [
    "eforms-chain/1-cn-16-831374-2025.xml",
    "eforms-chain/2-change-16-6281-2026.xml",
    "eforms-chain/3-change-16-18902-2026.xml",
    "eforms-chain/4-can-29-380868-2026.xml",
];

/// A second, unrelated procedure — what "a new notice arrives mid-stream"
/// means for the live feed.
const LATE: &str = "eforms/cn-16-00494343-2026.xml";

// --------------------------------------------------------------- the harness

struct Server {
    db: Arc<Db>,
    base: String,
    http: reqwest::Client,
    fetch_id: i64,
    path: String,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

impl Server {
    /// Boot the API over a scratch database. The `Db` handle is kept so the
    /// test can ingest *while the server is serving it* — same file, same
    /// writer, same doorbell, which is exactly the production arrangement.
    async fn start(name: &str) -> Server {
        let path = format!("/tmp/tender-db-api-{name}-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Arc::new(Db::open(&path).await.expect("open scratch db"));
        db.record_fetch(&store::Fetch {
            source: SOURCE.into(),
            kind: "daily".into(),
            period: "2026-00136".into(),
            url: "https://example.invalid/pkg".into(),
            sha256: "aa".into(),
            bytes: 1,
            fetched_at: 0,
            path: "ted/daily/2026-00136.tar.gz".into(),
        })
        .await
        .expect("record fetch");
        let fetch_id = db.current_packages(SOURCE, "daily", None).await.expect("packages")[0].fetch_id;

        let state = v1::AppState::new(db.clone(), db.readers(4, "test").expect("readers"));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let port = listener.local_addr().expect("addr").port();
        tokio::spawn(async move {
            let _ = axum::serve(listener, v1::router(state)).await;
        });

        Server {
            db,
            base: format!("http://127.0.0.1:{port}"),
            http: reqwest::Client::new(),
            fetch_id,
            path,
        }
    }

    async fn get(&self, path: &str) -> Value {
        let response = self.http.get(format!("{}{path}", self.base)).send().await.expect("request");
        assert!(response.status().is_success(), "GET {path} → {}", response.status());
        response.json().await.expect("json body")
    }

    /// Like [`Self::get`] but does not assert success — for endpoints whose
    /// unhealthy answer is a non-2xx with a JSON body (e.g. `/health/deep`).
    async fn get_allow_error(&self, path: &str) -> Value {
        let response = self.http.get(format!("{}{path}", self.base)).send().await.expect("request");
        response.json().await.expect("json body")
    }

    async fn status(&self, path: &str) -> u16 {
        self.http
            .get(format!("{}{path}", self.base))
            .send()
            .await
            .expect("request")
            .status()
            .as_u16()
    }

    /// Run a fixture through the real dispatch + parse chain, store it, and
    /// project — the same path `process` + `project` take from an archive.
    async fn ingest(&self, relative: &str) {
        let path = format!("{FIXTURES}/{relative}");
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
        let profile::Disposition::Records(records) = profile::dispatch(relative, &bytes) else {
            panic!("{relative}: dispatch skipped a fixture");
        };
        let [profile::Record::Notice(n)] = &records[..] else {
            panic!("{relative}: expected one notice record");
        };
        let parse = eforms::parse_payload(&n.profile, &bytes);
        assert!(matches!(parse, Parse::Parsed(_)), "{relative}: {parse:?}");
        self.db
            .record_notice(
                &Notice {
                    source: SOURCE.into(),
                    publication_id: n.publication_id.clone(),
                    content_hash: n.content_hash.clone(),
                    profile: n.profile.clone(),
                    declared_version: n.declared_version.clone(),
                    fetch_id: self.fetch_id,
                    member_path: n.member_path.clone(),
                    ingested_at: 0,
                    published_at: None,
                    dispatched_at: None,
                },
                &parse,
            )
            .await
            .expect("record notice");
        project::project(&self.db, false).await.expect("project");
    }

    async fn ingest_chain(&self) {
        for fixture in CHAIN {
            self.ingest(fixture).await;
        }
    }
}

fn items(page: &Value) -> &Vec<Value> {
    page["items"].as_array().expect("items is an array")
}

// ------------------------------------------------------------------ SSE tape

/// One parsed `event:`/`id:`/`data:` block.
#[derive(Debug, Clone)]
struct SseEvent {
    name: String,
    id: Option<String>,
    data: Value,
}

/// Reads an SSE response incrementally, so a test can assert on the events that
/// have arrived *so far* and then trigger the next ones.
struct Tape {
    stream: std::pin::Pin<Box<dyn futures::Stream<Item = reqwest::Result<bytes::Bytes>> + Send>>,
    buffer: String,
}

impl Tape {
    async fn open(server: &Server, path: &str, resume: Option<&str>) -> Tape {
        let mut request = server
            .http
            .get(format!("{}{path}", server.base))
            .header("accept", "text/event-stream");
        if let Some(id) = resume {
            request = request.header("last-event-id", id);
        }
        let response = request.send().await.expect("sse request");
        assert!(response.status().is_success(), "SSE {path} → {}", response.status());
        assert_eq!(
            response.headers().get("x-accel-buffering").and_then(|v| v.to_str().ok()),
            Some("no"),
            "nginx must be told not to buffer the stream"
        );
        assert!(
            response
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .is_some_and(|t| t.starts_with("text/event-stream")),
        );
        Tape { stream: Box::pin(response.bytes_stream()), buffer: String::new() }
    }

    /// The next event, or None if none arrives within the timeout — which is
    /// how the test asserts that a stream is *quiet*.
    async fn next(&mut self) -> Option<SseEvent> {
        use futures::StreamExt;
        loop {
            if let Some(event) = self.take_buffered() {
                return Some(event);
            }
            let chunk =
                tokio::time::timeout(Duration::from_secs(10), self.stream.next()).await.ok()??;
            self.buffer.push_str(&String::from_utf8_lossy(&chunk.expect("chunk")));
        }
    }

    async fn quiet(&mut self) -> bool {
        use futures::StreamExt;
        if self.take_buffered().is_some() {
            return false;
        }
        match tokio::time::timeout(Duration::from_millis(750), self.stream.next()).await {
            Err(_) => true,
            Ok(None) => true,
            Ok(Some(chunk)) => {
                self.buffer.push_str(&String::from_utf8_lossy(&chunk.expect("chunk")));
                // Keep-alive comments are not events.
                self.take_buffered().is_none()
            }
        }
    }

    fn take_buffered(&mut self) -> Option<SseEvent> {
        while let Some(end) = self.buffer.find("\n\n") {
            let block: String = self.buffer.drain(..end + 2).collect();
            let mut name = "message".to_owned();
            let mut id = None;
            let mut data = String::new();
            for line in block.lines() {
                if let Some(rest) = line.strip_prefix("event:") {
                    name = rest.trim().to_owned();
                } else if let Some(rest) = line.strip_prefix("id:") {
                    id = Some(rest.trim().to_owned());
                } else if let Some(rest) = line.strip_prefix("data:") {
                    data.push_str(rest.trim());
                }
            }
            if data.is_empty() {
                continue; // a keep-alive comment block
            }
            return Some(SseEvent {
                name,
                id,
                data: serde_json::from_str(&data).expect("event data is json"),
            });
        }
        None
    }

    /// Everything up to and including the `live` marker.
    async fn until_live(&mut self) -> (Vec<SseEvent>, SseEvent) {
        let mut snapshot = Vec::new();
        loop {
            let event = self.next().await.expect("stream ended before the live marker");
            if event.name == "live" {
                return (snapshot, event);
            }
            snapshot.push(event);
        }
    }
}

// ----------------------------------------------------------------- REST shape

#[tokio::test]
async fn the_service_root_and_health_answer() {
    let server = Server::start("root").await;

    let health = server.get("/health").await;
    assert_eq!(health["ok"], Value::Bool(true));
    assert_eq!(health["database"], "ok");
    assert!(health["rev"].is_string(), "health names the revision deploy.sh checks");
    assert_eq!(health["cursor"], "0", "a fresh database sits at cursor zero");

    let root = server.get("/v1").await;
    assert_eq!(root["service"], "tender-db");
    assert_eq!(root["license"], "AGPL-3.0-or-later");
    // AGPL §13: a network user must be offered the running version's source.
    assert!(root["source_offer"].as_str().is_some_and(|s| s.starts_with("https://")));
    assert_eq!(server.status("/_source").await, 200);
}

/// The deep health probe (issue 24): a fresh, live box — database answering, no
/// ingest run yet — reports healthy, and the body carries every operational
/// check the external pinger judges production by.
#[tokio::test]
async fn the_deep_health_probe_reports_operational_health() {
    let server = Server::start("deep-health").await;

    assert_eq!(server.status("/health/deep").await, 200, "a live box is healthy");
    let deep = server.get("/health/deep").await;
    assert_eq!(deep["ok"], Value::Bool(true));
    assert_eq!(deep["checks"]["database"]["ok"], Value::Bool(true));
    // No scheduled run has fired yet — absence is not an alarm.
    assert_eq!(deep["checks"]["ingest_freshness"]["ok"], Value::Bool(true));
    assert_eq!(deep["checks"]["ingest_freshness"]["last_success_at"], Value::Null);
    assert_eq!(deep["checks"]["last_job"]["outcome"], Value::Null);

    // A successful run refreshes the freshness clock; a later failure trips the
    // last-job check and flips the whole probe to 503 for the pinger.
    let now = store::now_unix();
    server.db.record_job_run("process", "ted daily (all)", now - 20, now - 10, "ok", "42 notices").await.unwrap();
    let ok_run = server.get("/health/deep").await;
    assert_eq!(ok_run["checks"]["ingest_freshness"]["last_success_at"], Value::from(now - 10));

    server.db.record_job_run("project", "rebuild=false", now - 5, now, "error", "db: locked").await.unwrap();
    assert_eq!(server.status("/health/deep").await, 503, "the last job errored");
    let errored = server.get_allow_error("/health/deep").await;
    assert_eq!(errored["ok"], Value::Bool(false));
    assert_eq!(errored["checks"]["last_job"]["ok"], Value::Bool(false));
}

#[tokio::test]
async fn a_fresh_database_answers_every_collection_with_an_empty_page() {
    let server = Server::start("empty").await;
    for path in ["/v1/tenders", "/v1/lots", "/v1/organizations", "/v1/notices"] {
        let page = server.get(path).await;
        assert!(items(&page).is_empty(), "{path} should be empty");
        assert_eq!(page["more"], Value::Bool(false));
        assert_eq!(page["next_cursor"], Value::Null);
    }
    let changes = server.get("/v1/changes?since=0").await;
    assert!(changes["events"].as_array().expect("events").is_empty());
    assert_eq!(changes["last_cursor"], "0");
}

#[tokio::test]
async fn the_collections_serve_the_canonical_layer() {
    let server = Server::start("collections").await;
    server.ingest_chain().await;

    let tenders = server.get("/v1/tenders").await;
    assert_eq!(items(&tenders).len(), 1, "the four notices are one Tender");
    let tender = &items(&tenders)[0];
    assert_eq!(tender["source"], SOURCE);
    assert_eq!(tender["kind"], "procedure");
    assert_eq!(tender["version"], 4, "the list shows the newest version");
    assert!(tender["title"].is_string());
    assert!(
        tender["published_at"].as_str().is_some_and(|s| s.ends_with('Z')),
        "timestamps are ISO 8601"
    );
    assert!(tender["lots"].as_i64().is_some_and(|n| n > 0));

    assert!(!items(&server.get("/v1/lots").await).is_empty());
    assert!(!items(&server.get("/v1/organizations").await).is_empty());
    assert_eq!(items(&server.get("/v1/notices").await).len(), 4, "four Notices, one Tender");

    // Money is {cents, currency} — never a float, never a bare number.
    let valued = items(&server.get("/v1/lots").await)
        .iter()
        .find(|l| l["value"].is_object())
        .cloned()
        .expect("the chain publishes lot values");
    assert!(valued["value"]["cents"].is_i64());
    assert!(valued["value"]["currency"].is_string());
}

#[tokio::test]
async fn the_filters_narrow_the_same_way_on_every_collection() {
    let server = Server::start("filters").await;
    server.ingest_chain().await;

    let cpv = items(&server.get("/v1/tenders").await)[0].clone();
    let id = cpv["id"].as_i64().expect("tender id");

    // Source and kind come off the Tender identity.
    assert_eq!(items(&server.get("/v1/tenders?source=ted").await).len(), 1);
    assert!(items(&server.get("/v1/tenders?source=doe").await).is_empty());
    assert!(items(&server.get("/v1/tenders?kind=registration").await).is_empty());

    // The classification predicates read the version's satellites.
    let detail = server.get(&format!("/v1/tenders/{id}")).await;
    let classifications = detail["classifications"].as_array().expect("classifications");
    let cpv_code = classifications
        .iter()
        .find(|c| c["scheme"] == "cpv")
        .and_then(|c| c["code"].as_str())
        .expect("the chain classifies by CPV")
        .to_owned();
    assert_eq!(items(&server.get(&format!("/v1/tenders?cpv={}", &cpv_code[..4])).await).len(), 1);
    assert!(items(&server.get("/v1/tenders?cpv=9999").await).is_empty());

    // The buyer filter resolves through the canonical Organization.
    let buyer = detail["parties"]
        .as_array()
        .expect("parties")
        .iter()
        .find(|p| p["role"].as_str().is_some_and(|r| r.contains("Buyer")))
        .and_then(|p| p["organization_id"].as_i64())
        .expect("the chain names a buyer");
    assert_eq!(items(&server.get(&format!("/v1/tenders?buyer={buyer}")).await).len(), 1);
    assert!(items(&server.get("/v1/tenders?buyer=999999").await).is_empty());

    // Value range, in cents.
    let value = detail["value"]["cents"].as_i64();
    if let Some(cents) = value {
        assert_eq!(items(&server.get(&format!("/v1/tenders?min_value={cents}")).await).len(), 1);
        assert!(items(&server.get(&format!("/v1/tenders?max_value={}", cents - 1)).await).is_empty());
    }

    // Status is evaluated against the submission deadline: this procedure's
    // deadline is long past, so it is closed and not open.
    assert_eq!(items(&server.get("/v1/tenders?status=closed").await).len(), 1);
    assert!(items(&server.get("/v1/tenders?status=open").await).is_empty());
    assert_eq!(server.status("/v1/tenders?status=sideways").await, 400);

    // Lots take the tender-level predicates through their parent.
    assert!(!items(&server.get("/v1/lots?source=ted").await).is_empty());
    assert!(items(&server.get("/v1/lots?source=doe").await).is_empty());
    assert!(!items(&server.get(&format!("/v1/lots?tender={id}")).await).is_empty());
}

#[tokio::test]
async fn pagination_walks_the_whole_collection_exactly_once() {
    let server = Server::start("pages").await;
    server.ingest_chain().await;

    // Notices are the deterministic collection here: the chain is exactly four.
    let all: Vec<i64> = items(&server.get("/v1/notices").await)
        .iter()
        .map(|n| n["id"].as_i64().expect("notice id"))
        .collect();
    assert_eq!(all.len(), 4);

    let mut seen = Vec::new();
    let mut cursor = None;
    loop {
        let path = match &cursor {
            None => "/v1/notices?limit=2".to_owned(),
            Some(c) => format!("/v1/notices?limit=2&cursor={c}"),
        };
        let page = server.get(&path).await;
        seen.extend(items(&page).iter().map(|n| n["id"].as_i64().expect("notice id")));
        assert!(items(&page).len() <= 2, "a page never exceeds its limit");
        match page["next_cursor"].as_str() {
            Some(next) => cursor = Some(next.to_owned()),
            None => break,
        }
    }
    assert_eq!(seen, all, "paging yields every row, in order, without repeats");
}

#[tokio::test]
async fn the_tender_detail_carries_its_whole_evidence_trail() {
    let server = Server::start("detail").await;
    server.ingest_chain().await;

    let id = items(&server.get("/v1/tenders").await)[0]["id"].as_i64().expect("tender id");
    let detail = server.get(&format!("/v1/tenders/{id}")).await;

    assert_eq!(detail["id"], id);
    assert_eq!(detail["version"], 4);
    let versions = detail["versions"].as_array().expect("versions");
    assert_eq!(versions.len(), 4, "one version per Notice of the procedure");
    assert_eq!(versions[0]["seq"], 1);
    // ADR-0001's traceability promise, reaching the API surface.
    assert!(versions.iter().all(|v| v["caused_by_notice_id"].as_i64().is_some()));
    assert!(versions.iter().all(|v| v["published_at"].as_str().is_some()));

    assert!(!detail["texts"].as_array().expect("texts").is_empty());
    assert!(!detail["lot_details"].as_array().expect("lots").is_empty());
    assert!(!detail["parties"].as_array().expect("parties").is_empty());
    // A date-only field stays a date; a deadline with a time keeps its offset.
    for date in detail["dates"].as_array().expect("dates") {
        assert!(date["value"].as_str().is_some(), "every date renders");
    }

    assert_eq!(server.status("/v1/tenders/999999").await, 404);
}

#[tokio::test]
async fn the_change_feed_replays_the_cursor_spine() {
    let server = Server::start("changes").await;
    server.ingest_chain().await;

    let all = server.get("/v1/changes?since=0&limit=1000").await;
    let events = all["events"].as_array().expect("events");
    assert!(!events.is_empty());
    assert!(events.iter().all(|e| e["cursor"].is_string()), "cursors are opaque strings");
    assert!(events.iter().any(|e| e["entity"] == "tender"));
    assert!(events.iter().any(|e| e["entity"] == "organization"));
    assert!(events.iter().all(|e| e["changed_at"].as_str().is_some()));

    // `since` is exclusive and the entity filter is a slice of the same log.
    let first = events[0]["cursor"].as_str().expect("cursor").to_owned();
    let rest = server.get(&format!("/v1/changes?since={first}&limit=1000")).await;
    assert_eq!(rest["events"].as_array().expect("events").len(), events.len() - 1);
    let tenders = server.get("/v1/changes?since=0&limit=1000&entity=tender").await;
    assert!(tenders["events"].as_array().expect("events").iter().all(|e| e["entity"] == "tender"));

    // Reading to the end lands on the same cursor the root reports.
    assert_eq!(all["last_cursor"], server.get("/v1").await["cursor"]);
}

// ----------------------------------------------------------------------- SSE

#[tokio::test]
async fn a_subscription_snapshots_then_streams_diffs() {
    let server = Server::start("sse").await;
    server.ingest_chain().await;

    let mut tape = Tape::open(&server, "/v1/tenders?include_data=true", None).await;
    let (snapshot, live) = tape.until_live().await;

    // Step 2: the matching set arrives as `added`, then the boundary marker.
    assert_eq!(snapshot.len(), 1, "one Tender matches, so one snapshot event");
    assert_eq!(snapshot[0].name, "change");
    assert_eq!(snapshot[0].data["op"], "added");
    assert_eq!(snapshot[0].data["entity"], "tender");
    assert!(snapshot[0].data["data"]["title"].is_string(), "include_data embeds the state");
    let boundary = live.data["cursor"].as_str().expect("live carries the cursor").to_owned();
    assert_eq!(live.id.as_deref(), Some(boundary.as_str()), "the marker's id is the boundary");

    // A live stream with nothing happening stays quiet (keep-alives aside).
    assert!(tape.quiet().await, "no diffs before anything changes");

    // Step 3: a notice lands while the stream is open. The writer rings the
    // doorbell; the diff loop reads past the boundary and classifies it.
    server.ingest(LATE).await;
    let diff = tape.next().await.expect("the new Tender must reach the stream");
    assert_eq!(diff.name, "change");
    assert_eq!(diff.data["op"], "added");
    assert_eq!(diff.data["entity"], "tender");
    let diff_cursor: i64 = diff.data["cursor"].as_str().expect("cursor").parse().expect("number");
    assert!(diff_cursor > boundary.parse::<i64>().expect("number"), "diffs are past the boundary");
    assert_eq!(diff.id.as_deref(), Some(diff.data["cursor"].as_str().expect("cursor")));
}

#[tokio::test]
async fn a_resumed_subscription_gets_exactly_what_it_missed() {
    let server = Server::start("resume").await;
    server.ingest_chain().await;

    // Establish a position, then disconnect.
    let boundary = {
        let mut tape = Tape::open(&server, "/v1/tenders", None).await;
        let (_, live) = tape.until_live().await;
        live.id.expect("the live marker carries the resume position")
    };

    // Everything below happens while nobody is listening.
    server.ingest(LATE).await;

    let mut resumed = Tape::open(&server, "/v1/tenders", Some(&boundary)).await;
    let event = resumed.next().await.expect("the missed change must be replayed");
    assert_eq!(event.name, "change", "a resume skips the snapshot entirely");
    assert_eq!(event.data["op"], "added");
    assert!(
        event.data["cursor"].as_str().expect("cursor").parse::<i64>().expect("number")
            > boundary.parse::<i64>().expect("number")
    );
    assert!(resumed.quiet().await, "and nothing else — the resume is exact, not approximate");
}

#[tokio::test]
async fn resuming_from_the_start_of_the_log_replays_it_rather_than_resetting() {
    let server = Server::start("reset").await;
    server.ingest_chain().await;

    // `?cursor=` is the resume position for clients that are not EventSource.
    // Cursor 0 is *at* the horizon while nothing has been pruned, so the honest
    // answer is the whole log, not a reset. The reset path is reachable only
    // once the log is pruned, which it never is today (docs/architecture.md).
    let mut tape = Tape::open(&server, "/v1/tenders?cursor=0", None).await;
    let first = tape.next().await.expect("the replay must start");
    assert_eq!(first.name, "change", "a resume replays changes, it does not resnapshot");
    assert_ne!(first.name, "reset");
}

#[tokio::test]
async fn a_filtered_subscription_only_snapshots_its_own_matches() {
    let server = Server::start("filtered").await;
    server.ingest_chain().await;

    let mut matching = Tape::open(&server, "/v1/tenders?source=ted", None).await;
    let (snapshot, _) = matching.until_live().await;
    assert_eq!(snapshot.len(), 1);

    let mut empty = Tape::open(&server, "/v1/tenders?source=doe", None).await;
    let (snapshot, live) = empty.until_live().await;
    assert!(snapshot.is_empty(), "a filter that matches nothing snapshots nothing");
    assert!(live.data["cursor"].is_string(), "but still names the boundary it starts from");

    // The same predicate governs the diff: a Tender that does not match never
    // reaches this stream, even though the change log records it.
    server.ingest(LATE).await;
    assert!(empty.quiet().await, "a non-matching change is not this subscription's business");
}

#[tokio::test]
async fn a_client_may_hold_five_streams_and_no_more() {
    let server = Server::start("cap").await;
    server.ingest_chain().await;

    // The cap is per client key; the forwarded header is that key.
    let open = async |n: usize| {
        server
            .http
            .get(format!("{}/v1/tenders", server.base))
            .header("accept", "text/event-stream")
            .header("x-forwarded-for", "203.0.113.7")
            .send()
            .await
            .unwrap_or_else(|e| panic!("stream {n}: {e}"))
    };

    let mut held = Vec::new();
    for n in 0..5 {
        let response = open(n).await;
        assert!(response.status().is_success(), "stream {n} is within the budget");
        held.push(response);
    }
    assert_eq!(open(5).await.status().as_u16(), 429, "the sixth stream is refused");

    // Dropping a stream returns its slot.
    held.pop();
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(open(6).await.status().is_success(), "a released slot is reusable");
}
