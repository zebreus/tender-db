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
    isolated: Arc<v1::isolate::IsolatedReads>,
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
        Server::boot(name, None).await
    }

    /// As [`start`](Server::start), with the SSE snapshot page size shrunk so a
    /// handful of fixture rows spans several pages (issue 55's paged snapshot).
    async fn boot(name: &str, snapshot_page: Option<i64>) -> Server {
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

        let mut state = v1::AppState::new(db.clone(), db.readers(4).expect("readers"));
        if let Some(page) = snapshot_page {
            state.snapshot_page = page;
        }
        let isolated = state.isolated.clone();
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
            isolated,
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

    /// Status and body from ONE request. `/health/deep` folds a live measurement
    /// (disk) into its verdict, so asserting a relationship between its status and
    /// its body across two requests would be asserting across two different
    /// measurements.
    async fn get_with_status(&self, path: &str) -> (u16, Value) {
        let response = self.http.get(format!("{}{path}", self.base)).send().await.expect("request");
        let status = response.status().as_u16();
        (status, response.json().await.expect("json body"))
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
    // Liveness only: `/health` does NOT query the database (issue 61/213), so it
    // must NOT report a `database` verdict — that claim now lives on `/health/deep`.
    assert!(
        health.get("database").is_none(),
        "/health is a liveness probe and must not report a DB check it never runs"
    );
    assert!(health["rev"].is_string(), "health names the revision deploy.sh checks");
    assert_eq!(health["cursor"], "0", "a fresh database sits at cursor zero");

    let root = server.get("/v1").await;
    assert_eq!(root["service"], "tender-db");
    assert_eq!(root["license"], "AGPL-3.0-or-later");
    // AGPL §13: a network user must be offered the running version's source.
    assert!(root["source_offer"].as_str().is_some_and(|s| s.starts_with("https://")));
    assert_eq!(server.status("/_source").await, 200);
}

/// `/health/deep`'s verdict is exactly the conjunction of its per-check verdicts,
/// and the status code is `200` iff that verdict is healthy — the contract the
/// external pinger relies on, since it alerts on the status alone.
///
/// This holds on ANY host, which is the point: it is the strongest statement about
/// the probe that does not depend on the machine running it. It catches the wiring
/// defect that matters — a check computed and reported but not folded into `ok`,
/// which would leave the pinger silent through a real outage.
fn assert_verdict_is_the_conjunction(status: u16, body: &Value, when: &str) {
    let checks = body["checks"].as_object().expect("the probe names its checks");
    // Named explicitly rather than derived from the body: a check that vanished
    // from the JSON must fail this test, and iterating whatever is present would
    // silently accept its absence.
    for name in ["database", "ingest_freshness", "last_job", "disk"] {
        assert!(checks.contains_key(name), "{when}: /health/deep dropped the {name} check");
    }
    let all_green = checks.values().all(|c| c["ok"] == Value::Bool(true));
    assert_eq!(
        body["ok"],
        Value::Bool(all_green),
        "{when}: the overall verdict must be the conjunction of the checks, got {body}"
    );
    let expected = if all_green { 200 } else { 503 };
    assert_eq!(status, expected, "{when}: the pinger judges by status alone, got {body}");
}

/// The deep health probe (issue 24): a fresh, live box — database answering, no
/// ingest run yet — is healthy on every signal the test controls, and the body
/// carries every operational check the external pinger judges production by.
///
/// **What this test may and may not assert.** One of the four checks — `disk` — is
/// a property of the HOST rather than of the code: this machine's free space
/// decides it. So the three checks the test controls are asserted by value, and
/// the fourth only through [`assert_verdict_is_the_conjunction`], which holds
/// everywhere.
///
/// It used to assert a bare `200`, which made the verdict a function of the
/// developer's free disk space: **red in the gate at 90% used, green at 89%** —
/// and this box sits at 89%. That is not a flake to retry, it is a test reporting
/// on the wrong subject, and a standing expected-red is how a team learns to scroll
/// past a real one. The thresholds themselves are unit-tested in `v1::health`,
/// where the disk figure is an input rather than a measurement — which is why
/// nothing is lost by refusing to re-measure them here.
#[tokio::test]
async fn the_deep_health_probe_reports_operational_health() {
    let server = Server::start("deep-health").await;

    let (status, deep) = server.get_with_status("/health/deep").await;
    assert_verdict_is_the_conjunction(status, &deep, "fresh box");
    assert_eq!(deep["checks"]["database"]["ok"], Value::Bool(true));
    // No scheduled run has fired yet — absence is not an alarm.
    assert_eq!(deep["checks"]["ingest_freshness"]["ok"], Value::Bool(true));
    assert_eq!(deep["checks"]["ingest_freshness"]["last_success_at"], Value::Null);
    assert_eq!(deep["checks"]["last_job"]["outcome"], Value::Null);

    // A successful run refreshes the freshness clock; a later failure trips the
    // last-job check and flips the whole probe to 503 for the pinger.
    let now = store::now_unix();
    server.db.record_job_run("process", "ted daily (all)", now - 20, now - 10, "ok", "42 notices").await.unwrap();
    let (status, ok_run) = server.get_with_status("/health/deep").await;
    assert_verdict_is_the_conjunction(status, &ok_run, "after a successful run");
    assert_eq!(ok_run["checks"]["ingest_freshness"]["last_success_at"], Value::from(now - 10));

    // The unhealthy direction IS asserted absolutely: one failing check must force
    // 503 whatever the disk says, because failure is monotone in the conjunction.
    server.db.record_job_run("project", "rebuild=false", now - 5, now, "error", "db: locked").await.unwrap();
    let (status, errored) = server.get_with_status("/health/deep").await;
    assert_verdict_is_the_conjunction(status, &errored, "after a failed job");
    assert_eq!(status, 503, "the last job errored — unhealthy regardless of the host");
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

/// Issue 211: the poll feed must carry only the public entity kinds SSE emits —
/// tender/lot/organization. The projection also writes lot_result/bid/contract
/// change rows for the canonical layer, but those are undocumented, never emitted
/// by SSE, and carry ids that resolve to no REST endpoint; a strict generated
/// client crashes on them. They must not appear on `/v1/changes`, and the filter
/// param must reject them with a 400.
#[tokio::test]
async fn the_change_feed_carries_only_the_public_entity_kinds() {
    let server = Server::start("changes-kinds").await;
    server.ingest_chain().await;

    // Precondition, so the assertions are not vacuous: the award chain really does
    // write result-graph change rows. Read the raw log through the store.
    let reader = server.db.readers(1).expect("readers").get().await.expect("reader");
    let raw = store::read::changes_since(&reader, 0, 10_000, None).await.expect("raw changes");
    drop(reader);
    let raw_kinds: std::collections::BTreeSet<&str> =
        raw.iter().map(|c| c.entity_kind.as_str()).collect();
    assert!(
        raw_kinds.iter().any(|k| matches!(*k, "lot_result" | "bid" | "contract")),
        "fixture precondition: the award chain must write result-graph change rows, got {raw_kinds:?}"
    );

    // The public feed excludes them.
    let feed = server.get("/v1/changes?since=0").await;
    let events = feed["events"].as_array().expect("events");
    assert!(!events.is_empty(), "the chain produced public changes to deliver");
    for e in events {
        let kind = e["entity"].as_str().expect("each event names its entity kind");
        assert!(
            matches!(kind, "tender" | "lot" | "organization"),
            "the poll feed leaked the undocumented entity kind {kind:?} (issue 211)"
        );
    }

    // The filter param rejects a non-public value with a 400, never undocumented rows.
    assert_eq!(server.status("/v1/changes?since=0&entity=lot_result").await, 400);
    assert_eq!(server.status("/v1/changes?since=0&entity=bid").await, 400);
    // A documented value still narrows to that kind.
    assert_eq!(server.status("/v1/changes?since=0&entity=tender").await, 200);
    let tenders_only = server.get("/v1/changes?since=0&entity=tender").await;
    for e in tenders_only["events"].as_array().expect("events") {
        assert_eq!(e["entity"], "tender", "the tender filter must return only tender events");
    }
}

/// Issue 215-C: `/v1/changes` must report `more:true` only when a next page really
/// exists — an exactly-`limit` FINAL page reports `more:false` (a `limit+1`
/// look-ahead), so a caller is not sent one extra empty poll. `more`/the cursor
/// operate over the raw change rows (all kinds, issue 211), so the total is read
/// through the store.
#[tokio::test]
async fn the_change_feed_reports_more_only_when_a_next_page_exists() {
    let server = Server::start("changes-more").await;
    server.ingest_chain().await;

    let reader = server.db.readers(1).expect("readers").get().await.expect("reader");
    let total = store::read::changes_since(&reader, 0, 100_000, None).await.expect("changes").len() as i64;
    drop(reader);
    assert!(total >= 2, "the chain produced change rows");

    // A page of EXACTLY the total is the final page.
    let full = server.get(&format!("/v1/changes?since=0&limit={total}")).await;
    assert_eq!(full["more"], false, "an exactly-full final page must report more:false");
    // One short of the end → a next row exists.
    let short = server.get(&format!("/v1/changes?since=0&limit={}", total - 1)).await;
    assert_eq!(short["more"], true, "a page one short of the end must report more:true");
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

/// Issue 49: the ids a tender detail hands out — `caused_by_notice_id` and
/// `parties[].organization_id` — must be fetchable, so the ADR-0001 chain does
/// not dead-end at the API.
#[tokio::test]
async fn advertised_entities_are_fetchable_by_id() {
    let server = Server::start("by_id").await;
    server.ingest_chain().await;

    let tender_id = items(&server.get("/v1/tenders").await)[0]["id"].as_i64().expect("tender id");
    let detail = server.get(&format!("/v1/tenders/{tender_id}")).await;

    // A notice id from the version chain resolves to that notice.
    let notice_id = detail["versions"].as_array().expect("versions")[0]["caused_by_notice_id"]
        .as_i64()
        .expect("caused_by_notice_id");
    let notice = server.get(&format!("/v1/notices/{notice_id}")).await;
    assert_eq!(notice["id"].as_i64(), Some(notice_id));
    assert!(notice["profile"].is_string(), "the per-id notice carries the list shape");

    // An organization id from parties resolves to that organization.
    let org_id = detail["parties"]
        .as_array()
        .expect("parties")
        .iter()
        .find_map(|p| p["organization_id"].as_i64())
        .expect("a party organization");
    let org = server.get(&format!("/v1/organizations/{org_id}")).await;
    assert_eq!(org["id"].as_i64(), Some(org_id));
    assert!(org["name"].is_string());

    // A missing id is a JSON 404 envelope, never the HTML dashboard.
    assert_eq!(server.status("/v1/notices/99999999").await, 404);
    assert_eq!(server.status("/v1/organizations/99999999").await, 404);
    let missing = server.get_allow_error("/v1/organizations/99999999").await;
    assert_eq!(missing["error"]["status"].as_u64(), Some(404));
}

/// Issue 49 part 4: a tender list row echoes the `cpv` and `country` codes it
/// carries, so a client can see why the row matched a filter (before, you could
/// filter on them but not see them).
#[tokio::test]
async fn tender_list_rows_echo_cpv_and_country() {
    let server = Server::start("echo_fields").await;
    server.ingest_chain().await;

    let tender = items(&server.get("/v1/tenders").await)[0].clone();
    let echoed: Vec<&str> =
        tender["cpv"].as_array().expect("cpv is an array").iter().filter_map(|c| c.as_str()).collect();
    assert!(!echoed.is_empty(), "the chain classifies by CPV, so the row must show it");
    assert!(tender["country"].is_array(), "country is echoed as an array");

    // The echoed codes are exactly the version's CPV classifications (the detail
    // reads them the same way), so the list agrees with the detail.
    let id = tender["id"].as_i64().expect("tender id");
    let detail = server.get(&format!("/v1/tenders/{id}")).await;
    let detail_cpv: Vec<&str> = detail["classifications"]
        .as_array()
        .expect("classifications")
        .iter()
        .filter(|c| c["scheme"] == "cpv")
        .filter_map(|c| c["code"].as_str())
        .collect();
    assert!(
        echoed.iter().all(|code| detail_cpv.contains(code)),
        "echoed cpv {echoed:?} should all appear in the detail {detail_cpv:?}"
    );
}

/// Issue 49: `?tender=` lists exactly a tender's notices rather than silently
/// ignoring the filter and dumping unrelated ones.
#[tokio::test]
async fn notices_can_be_scoped_to_a_tender() {
    let server = Server::start("notices_of_tender").await;
    server.ingest_chain().await;
    let tender_id = items(&server.get("/v1/tenders").await)[0]["id"].as_i64().expect("tender id");

    let scoped = server.get(&format!("/v1/notices?tender={tender_id}")).await;
    let scoped_ids: Vec<i64> =
        items(&scoped).iter().map(|n| n["id"].as_i64().expect("notice id")).collect();
    assert_eq!(scoped_ids.len(), 4, "the four notices that built this tender");
    // Every scoped notice is a real notice of the collection.
    let all_ids: Vec<i64> = items(&server.get("/v1/notices").await)
        .iter()
        .map(|n| n["id"].as_i64().unwrap())
        .collect();
    assert!(scoped_ids.iter().all(|id| all_ids.contains(id)));
    // An unknown tender is a 404, not an unfiltered dump.
    assert_eq!(server.status("/v1/notices?tender=99999999").await, 404);
}

/// Issue 217-A: a Notice is reachable by its official publication number, the
/// real-world external key printed on every notice — an exact-match `publication_id`
/// filter on `/v1/notices`. On collections that do not honour it, it is NAMED ignored
/// (issue 118), never silently applied.
#[tokio::test]
async fn notices_can_be_looked_up_by_publication_id() {
    let server = Server::start("pubid").await;
    server.ingest_chain().await;

    let notices = items(&server.get("/v1/notices").await).clone();
    let pubid = notices[0]["publication_id"].as_str().expect("publication_id").to_owned();
    let id = notices[0]["id"].as_i64().expect("notice id");

    // Exact lookup (source-paired: the indexed path) returns that notice, only it.
    let hit = server.get(&format!("/v1/notices?source=ted&publication_id={pubid}")).await;
    let hit_ids: Vec<i64> = items(&hit).iter().map(|n| n["id"].as_i64().unwrap()).collect();
    assert!(hit_ids.contains(&id), "publication_id lookup returns the matching notice");
    assert!(
        items(&hit).iter().all(|n| n["publication_id"] == pubid),
        "the page contains only that publication_id"
    );

    // Unknown value → empty page, not a 400 and not an unfiltered dump.
    let miss = server.get("/v1/notices?publication_id=nonesuch-9999").await;
    assert!(items(&miss).is_empty(), "an unknown publication_id returns an empty page");

    // On a collection that does not apply it, it is named ignored, not silently dropped.
    let on_tenders = server.get(&format!("/v1/tenders?publication_id={pubid}")).await;
    let ignored: Vec<String> = on_tenders["ignored_filters"]
        .as_array()
        .expect("ignored_filters is always present")
        .iter()
        .map(|v| v.as_str().expect("a filter name").to_owned())
        .collect();
    assert!(
        ignored.iter().any(|f| f == "publication_id"),
        "publication_id must be named ignored on /v1/tenders, got {ignored:?}"
    );
}

/// Issue 217: an Organization is reachable by its official identifier VALUE (e.g. a
/// VAT number), not just its internal id — the canonical lookup that turns "I have
/// this company's VAT" into its profile, and the front door to the winner/buyer/
/// bidder reverse-lookups. Paired with `kind` it pins the scheme; unknown → empty
/// page; named ignored on collections that do not apply it (issue 118).
#[tokio::test]
async fn organizations_can_be_looked_up_by_identifier() {
    let server = Server::start("org_identifier").await;
    server.ingest_chain().await;

    // A real org that carries an official identifier, from the fixture.
    let orgs = items(&server.get("/v1/organizations").await).clone();
    let identified = orgs.iter().find(|o| o["identifier"].is_string());

    if let Some(org) = identified {
        let value = org["identifier"].as_str().expect("identifier").to_owned();
        let id = org["id"].as_i64().expect("org id");
        let kind = org["identifier_kind"].as_str().map(str::to_owned);

        let hit = server.get(&format!("/v1/organizations?identifier={value}")).await;
        let hit_ids: Vec<i64> = items(&hit).iter().map(|o| o["id"].as_i64().unwrap()).collect();
        assert!(hit_ids.contains(&id), "the identifier lookup returns the matching org");
        assert!(
            items(&hit).iter().all(|o| o["identifier"].as_str() == Some(value.as_str())),
            "the page contains only that identifier value"
        );

        // Paired with its scheme, still a hit (kind narrows, does not exclude).
        if let Some(kind) = kind {
            let paired =
                server.get(&format!("/v1/organizations?identifier={value}&kind={kind}")).await;
            assert!(
                items(&paired).iter().any(|o| o["id"].as_i64() == Some(id)),
                "identifier + its own kind still returns the org"
            );
        }
    }

    // Unknown value → empty page (a seek that finds nothing), not a 400 or a dump.
    let miss = server.get("/v1/organizations?identifier=ZZ-nonesuch-9999").await;
    assert!(items(&miss).is_empty(), "an unknown identifier returns an empty page");

    // Named ignored where it has no meaning (issue 118), never silently dropped.
    let ig: Vec<String> = server.get("/v1/tenders?identifier=DE811907980").await["ignored_filters"]
        .as_array()
        .expect("ignored_filters present")
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect();
    assert!(ig.iter().any(|f| f == "identifier"), "identifier is named ignored on /v1/tenders");
}

/// Issue 216 (deadline half, store side): the fold's head update maintains
/// `tenders.current_deadline` — for every ingested tender it must equal the
/// submission deadline the API serves (the read's own MAX-over-the-version pick),
/// so the materialised column can never drift from what a client sees.
#[tokio::test]
async fn the_fold_maintains_the_current_deadline_column() {
    let server = Server::start("current_deadline").await;
    server.ingest_chain().await;

    let page = server.get("/v1/tenders?limit=100").await;
    let reader = server.db.readers(1).expect("readers").get().await.expect("reader");
    let mut checked = 0;
    for t in items(&page) {
        let id = t["id"].as_i64().unwrap();
        let mut rows = reader
            .query(
                "SELECT current_deadline FROM tenders WHERE id = ?",
                [store::turso::Value::Integer(id)],
            )
            .await
            .expect("query");
        let column = rows
            .next()
            .await
            .expect("row")
            .and_then(|r| r.get_value(0).unwrap().as_integer().copied());
        // The API's submission_deadline is the same instant rendered in the
        // published offset; compare on the unix value via a re-parse.
        let api = t["submission_deadline"].as_str().map(|s| {
            chrono::DateTime::parse_from_rfc3339(s).expect("valid ISO 8601").timestamp()
        });
        assert_eq!(
            column, api,
            "tender {id}: current_deadline must equal the served submission_deadline"
        );
        checked += 1;
    }
    assert!(checked > 0, "the fixture serves tenders");
    // At least one fixture tender actually has a deadline, or this test is vacuous.
    assert!(
        items(&page).iter().any(|t| t["submission_deadline"].is_string()),
        "the fixture must include a tender with a submission deadline"
    );
}

/// Issue 216: the published-ordered Tender list — `sort=published_at` serves the
/// flagship "most recently published" query, newest first by default, paginating
/// by a composite (published_at, id) keyset cursor; a published range implies the
/// order; invalid sort vocabulary and unsupported combinations are hard 400s so a
/// client can never mistake an unsorted page for a sorted one.
#[tokio::test]
async fn tenders_list_sorts_by_publication_date() {
    let server = Server::start("published_sort").await;
    server.ingest_chain().await;

    let published = |t: &Value| t["published_at"].as_str().expect("published_at").to_owned();

    // Newest first by default under sort=published_at.
    let page = server.get("/v1/tenders?sort=published_at").await;
    let desc: Vec<String> = items(&page).iter().map(published).collect();
    assert!(!desc.is_empty(), "the fixture has published tenders");
    let mut sorted = desc.clone();
    sorted.sort_by(|a, b| b.cmp(a)); // ISO 8601 sorts lexicographically
    assert_eq!(desc, sorted, "sort=published_at defaults to newest first");

    // order=asc reverses.
    let asc_page = server.get("/v1/tenders?sort=published_at&order=asc").await;
    let asc: Vec<String> = items(&asc_page).iter().map(published).collect();
    let mut fwd = asc.clone();
    fwd.sort();
    assert_eq!(asc, fwd, "order=asc is oldest first");

    // One-row pages reassemble the full list via the composite cursor — no dup, no gap.
    let mut paged: Vec<i64> = Vec::new();
    let mut cursor: Option<String> = None;
    loop {
        let q = match &cursor {
            Some(c) => format!("/v1/tenders?sort=published_at&limit=1&cursor={c}"),
            None => "/v1/tenders?sort=published_at&limit=1".into(),
        };
        let p = server.get(&q).await;
        paged.extend(items(&p).iter().map(|t| t["id"].as_i64().unwrap()));
        match p["next_cursor"].as_str() {
            Some(next) => cursor = Some(next.to_owned()),
            None => break,
        }
    }
    let whole: Vec<i64> = items(&page).iter().map(|t| t["id"].as_i64().unwrap()).collect();
    assert_eq!(paged, whole, "1-row pages must reassemble the whole ordered list");

    // A published range narrows (and implies the published order without ?sort=).
    let newest = &desc[0];
    let ranged = server.get(&format!("/v1/tenders?published_after={newest}")).await;
    assert!(
        items(&ranged).iter().all(|t| published(t).as_str() >= newest.as_str()),
        "published_after keeps only rows at or after the bound"
    );

    // The strictness contract: bad vocabulary and unsupported shapes are 400s.
    assert_eq!(server.status("/v1/tenders?sort=newest").await, 400);
    assert_eq!(server.status("/v1/tenders?sort=published_at&order=sideways").await, 400);
    assert_eq!(server.status("/v1/tenders?order=desc").await, 400, "desc id order unsupported");
    assert_eq!(server.status("/v1/lots?sort=published_at").await, 400, "only /v1/tenders sorts");
    assert_eq!(
        server.status("/v1/tenders?sort=published_at&cursor=12345").await,
        400,
        "an id-shaped cursor does not match the published sort"
    );
    assert_eq!(server.status("/v1/tenders?published_after=not-a-date").await, 400);

    // Honesty: published_after is honoured on tenders, named ignored on notices.
    let ig = |page: &Value| -> Vec<String> {
        page["ignored_filters"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().into()).collect()
    };
    assert!(!ig(&ranged).iter().any(|f| f == "published_after"));
    assert!(
        ig(&server.get("/v1/notices?published_after=2020-01-01T00:00:00Z").await)
            .iter()
            .any(|f| f == "published_after"),
        "published_after is named ignored on /v1/notices"
    );
}

/// Issue 216 (deadline half): `sort=deadline` serves "what closes soon" —
/// soonest-closing first by default, deadline-less tenders omitted, the composite
/// cursor paginating cleanly; a deadline bound implies the ordering; giving both
/// date bounds without an explicit sort is ambiguous and must 400.
#[tokio::test]
async fn tenders_list_sorts_by_deadline() {
    let server = Server::start("deadline_sort").await;
    server.ingest_chain().await;

    let deadline = |t: &Value| t["submission_deadline"].as_str().map(str::to_owned);

    // Soonest-closing first by default; every listed row HAS a deadline.
    let page = server.get("/v1/tenders?sort=deadline").await;
    let ds: Vec<String> = items(&page).iter().map(|t| deadline(t).expect("listed ⇒ has one")).collect();
    assert!(!ds.is_empty(), "the fixture has deadline-carrying tenders");
    let mut asc = ds.clone();
    asc.sort();
    assert_eq!(ds, asc, "sort=deadline defaults to soonest first");

    // A tender without a deadline exists in the corpus but not in this ordering.
    let all = server.get("/v1/tenders?limit=100").await;
    let deadline_less = items(&all).iter().filter(|t| deadline(t).is_none()).count();
    if deadline_less > 0 {
        assert!(
            items(&page).len() < items(&all).len(),
            "deadline-less tenders are omitted from the deadline ordering"
        );
    }

    // One-row pages reassemble the ordered list exactly (composite cursor).
    let mut paged: Vec<i64> = Vec::new();
    let mut cursor: Option<String> = None;
    loop {
        let q = match &cursor {
            Some(c) => format!("/v1/tenders?sort=deadline&limit=1&cursor={c}"),
            None => "/v1/tenders?sort=deadline&limit=1".into(),
        };
        let p = server.get(&q).await;
        paged.extend(items(&p).iter().map(|t| t["id"].as_i64().unwrap()));
        match p["next_cursor"].as_str() {
            Some(next) => cursor = Some(next.to_owned()),
            None => break,
        }
    }
    let whole: Vec<i64> = items(&page).iter().map(|t| t["id"].as_i64().unwrap()).collect();
    assert_eq!(paged, whole, "1-row pages must reassemble the deadline ordering");

    // A deadline bound implies the ordering and narrows. Deliberately UNencoded:
    // the timestamp's `+` decodes to a space, which parse_instant restores — a
    // client pasting a served timestamp back must not 400.
    let first = &ds[0];
    let bounded = server.get(&format!("/v1/tenders?deadline_after={first}")).await;
    assert!(
        items(&bounded)
            .iter()
            .all(|t| deadline(t).expect("bounded ⇒ has one").as_str() >= first.as_str()),
        "deadline_after keeps only rows at or after the bound"
    );

    // Ambiguity and vocabulary contracts.
    assert_eq!(
        server
            .status("/v1/tenders?published_after=2020-01-01T00:00:00Z&deadline_after=2020-01-01T00:00:00Z")
            .await,
        400,
        "both date bounds without sort is ambiguous"
    );
    assert_eq!(server.status("/v1/tenders?sort=deadline&order=sideways").await, 400);
    assert_eq!(
        server.status("/v1/notices?deadline_after=2020-01-01T00:00:00Z").await,
        200,
        "deadline_after is accepted and named ignored off-tenders"
    );
    let ig: Vec<String> = server.get("/v1/notices?deadline_after=2020-01-01T00:00:00Z").await
        ["ignored_filters"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().into())
        .collect();
    assert!(ig.iter().any(|f| f == "deadline_after"));
}

/// Issue 225: the buyer seed narrows by role in SQL (`role LIKE '%Buyer%'`),
/// mirroring the EXISTS's own match — a vocabulary drift between the two would
/// silently drop legitimate buyers, so this pins the round trip on a REAL buyer
/// party from the fixture: the org must be reachable through `?buyer=` exactly as
/// recorded, and a non-buyer party org must NOT be (the seed's whole point).
#[tokio::test]
async fn tenders_reverse_lookup_by_buyer_matches_the_recorded_role() {
    let server = Server::start("buyer_role_seed").await;
    server.ingest_chain().await;

    let reader = server.db.readers(1).expect("readers").get().await.expect("reader");
    // A buyer-role party on a current version, straight from the canonical layer.
    let mut rows = reader
        .query(
            "SELECT p.organization_id, p.tender_id FROM tender_version_parties p
              WHERE p.role LIKE '%Buyer%'
                AND p.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = p.tender_id)
              LIMIT 1",
            (),
        )
        .await
        .expect("query buyer parties");
    let buyer = rows.next().await.expect("row").map(|r| {
        (
            r.get_value(0).unwrap().as_integer().copied().unwrap(),
            r.get_value(1).unwrap().as_integer().copied().unwrap(),
        )
    });
    drop(rows);
    // An org that appears ONLY in non-buyer roles, if the fixture has one.
    let mut rows = reader
        .query(
            "SELECT p.organization_id FROM tender_version_parties p
              WHERE p.organization_id NOT IN
                    (SELECT organization_id FROM tender_version_parties WHERE role LIKE '%Buyer%')
              LIMIT 1",
            (),
        )
        .await
        .expect("query non-buyer parties");
    let non_buyer =
        rows.next().await.expect("row").map(|r| r.get_value(0).unwrap().as_integer().copied().unwrap());
    drop(rows);
    drop(reader);

    if let Some((org, tender_id)) = buyer {
        let page = server.get(&format!("/v1/tenders?buyer={org}")).await;
        assert!(
            items(&page).iter().any(|t| t["id"].as_i64() == Some(tender_id)),
            "a recorded buyer role must be reachable through ?buyer="
        );
    }
    if let Some(org) = non_buyer {
        let page = server.get(&format!("/v1/tenders?buyer={org}")).await;
        assert!(
            items(&page).is_empty(),
            "an org with only non-buyer roles must not match ?buyer= (org {org})"
        );
    }

    // Absent org: the reachable() short-circuit still answers fast and empty.
    assert!(items(&server.get("/v1/tenders?buyer=999999999").await).is_empty());
}

/// Issue 217-B: `/v1/organizations?name_prefix=` — the case-insensitive name
/// search, in name order with a keyset cursor. The vocabulary contracts: empty
/// prefix 400s, a bad cursor 400s, other collections name it ignored.
#[tokio::test]
async fn organizations_can_be_searched_by_name_prefix() {
    let server = Server::start("org_name_prefix").await;
    server.ingest_chain().await;

    let orgs = items(&server.get("/v1/organizations").await).clone();
    let name = orgs[0]["name"].as_str().expect("org name").to_owned();
    let id = orgs[0]["id"].as_i64().expect("org id");
    // Search by the UPPERCASED first grapheme-ish chunk of a real name — the
    // round trip must be case-insensitive in both directions.
    let prefix: String = name.chars().take(4).collect::<String>().to_uppercase();

    let hit = server.get(&format!("/v1/organizations?name_prefix={}", urlenc(&prefix))).await;
    assert!(
        items(&hit).iter().any(|o| o["id"].as_i64() == Some(id)),
        "an uppercased prefix of a real name finds the org (case-insensitive)"
    );
    assert!(
        items(&hit).iter().all(|o| {
            o["name"].as_str().unwrap().to_lowercase().starts_with(&prefix.to_lowercase())
        }),
        "every hit actually carries the prefix"
    );

    // Absent prefix → empty page; empty prefix → 400; bad cursor → 400.
    assert!(items(&server.get("/v1/organizations?name_prefix=zzzzzzz").await).is_empty());
    assert_eq!(server.status("/v1/organizations?name_prefix=").await, 400);
    assert_eq!(
        server.status("/v1/organizations?name_prefix=a&cursor=not-a-name-cursor").await,
        400
    );

    // Named ignored on collections that do not apply it (issue 118).
    let ig: Vec<String> = server.get("/v1/tenders?name_prefix=a").await["ignored_filters"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().into())
        .collect();
    assert!(ig.iter().any(|f| f == "name_prefix"));
}

fn urlenc(s: &str) -> String {
    // Percent-encode per UTF-8 BYTE (an umlaut is two %XX escapes, not one).
    s.bytes()
        .flat_map(|b| {
            if b.is_ascii_alphanumeric() {
                vec![b as char]
            } else {
                format!("%{b:02X}").chars().collect()
            }
        })
        .collect()
}

/// Issue 218-B: `/v1/notices/{id}/content` serves the notice's WHOLE parsed
/// payload — the section tree and every typed value in the source's own field
/// vocabulary — so no business term is unreachable via REST even before it earns
/// a canonical projection (the parser-audit's per-layer contract).
#[tokio::test]
async fn notice_content_serves_the_whole_parsed_layer() {
    let server = Server::start("notice_content").await;
    server.ingest_chain().await;

    let id = items(&server.get("/v1/notices").await)[0]["id"].as_i64().expect("notice id");
    let content = server.get(&format!("/v1/notices/{id}/content")).await;
    assert_eq!(content["notice_id"].as_i64(), Some(id));

    let sections = content["sections"].as_array().expect("sections array");
    assert!(!sections.is_empty(), "a parsed notice has sections");
    assert!(
        sections.iter().any(|s| s["section_id"] == "PROCEDURE"),
        "the notice root is the PROCEDURE section"
    );
    // Every value carries the envelope fields + a type tag; at least one text
    // value exists somewhere (every real notice titles something).
    let values: Vec<&Value> =
        sections.iter().flat_map(|s| s["values"].as_array().unwrap()).collect();
    assert!(!values.is_empty(), "a parsed notice has values");
    for v in &values {
        assert!(v["field_id"].is_string(), "every value names its source field");
        assert!(v["ordinal"].is_i64(), "every value carries its repeat ordinal");
        assert!(v["type"].is_string(), "every value is type-tagged");
    }
    assert!(
        values.iter().any(|v| v["type"] == "text" && v["value"].is_string()),
        "at least one text value"
    );

    // An unknown notice is a 404 — distinct from a held notice's empty content.
    assert_eq!(server.status("/v1/notices/999999999/content").await, 404);
}

/// Issue 218: `/v1/notices/{id}` carries a `quarantine` field so a held notice
/// explains why it is absent from the canonical layer instead of returning a bare
/// `parse_state` stub. A cleanly-parsed notice reports `null`; the list rows stay
/// lean (no per-item quarantine lookup). The positive, held-notice case is covered
/// at the store layer (`tests/notice_quarantine.rs`), because the fixtures ingest
/// only clean notices and a held row cannot be minted through the public API.
#[tokio::test]
async fn notice_detail_carries_the_quarantine_field() {
    let server = Server::start("notice_quarantine").await;
    server.ingest_chain().await;

    let notices = items(&server.get("/v1/notices").await).clone();
    let id = notices[0]["id"].as_i64().expect("notice id");

    let detail = server.get(&format!("/v1/notices/{id}")).await;
    // The identity the list row carries is still present on the detail...
    assert_eq!(detail["id"].as_i64(), Some(id));
    assert!(detail["publication_id"].is_string(), "detail keeps the notice identity");
    // ...plus the quarantine field, which is null for a cleanly-parsed notice — the
    // field is ALWAYS present, so absent (parsed) is never confused with absent (bug).
    assert!(detail.get("quarantine").is_some(), "the quarantine field is always present");
    assert!(detail["quarantine"].is_null(), "a parsed notice is not held → quarantine null");

    // The lean list shape does NOT carry the field (it pays no per-item lookup).
    assert!(
        notices[0].get("quarantine").is_none(),
        "the list row stays lean; quarantine is a by-id detail concern only"
    );
}

/// Issue 217-C: a Tender is reachable by an org that SUBMITTED a bid on it (a
/// tenderer), won or not — the competitor-history reverse-lookup, a superset of
/// `winner`. Honoured on tenders/lots, named ignored elsewhere (issue 118), and an
/// org that bid on nothing short-circuits to an empty page (issue 219 / reachable()).
#[tokio::test]
async fn tenders_can_be_filtered_by_bidder() {
    let server = Server::start("bidder").await;
    server.ingest_chain().await;

    // A real tenderer + its tender from the fixture, so the positive case is not vacuous.
    let reader = server.db.readers(1).expect("readers").get().await.expect("reader");
    let mut rows = reader
        .query(
            "SELECT organization_id, tender_id FROM tender_version_bid_parties WHERE role = 'tenderer' LIMIT 1",
            (),
        )
        .await
        .expect("query bid parties");
    let bidder = rows.next().await.expect("row").map(|r| {
        (
            r.get_value(0).unwrap().as_integer().copied().unwrap(),
            r.get_value(1).unwrap().as_integer().copied().unwrap(),
        )
    });
    drop(rows);
    drop(reader);

    if let Some((org, tender_id)) = bidder {
        let page = server.get(&format!("/v1/tenders?bidder={org}")).await;
        assert!(
            items(&page).iter().any(|t| t["id"].as_i64() == Some(tender_id)),
            "the bidder filter returns a tender the org submitted a bid on"
        );
    }

    // An org that bid on nothing → empty page (reachable() short-circuit, not a walk).
    assert!(
        items(&server.get("/v1/tenders?bidder=999999999").await).is_empty(),
        "an unknown bidder returns an empty page"
    );

    // Honoured on tenders; named ignored on notices (which has no bid predicate).
    let ig = |page: &Value| -> Vec<String> {
        page["ignored_filters"]
            .as_array()
            .expect("ignored_filters present")
            .iter()
            .map(|v| v.as_str().unwrap().to_owned())
            .collect()
    };
    assert!(!ig(&server.get("/v1/tenders?bidder=7").await).iter().any(|f| f == "bidder"),
        "bidder is honoured on /v1/tenders");
    assert!(ig(&server.get("/v1/notices?bidder=7").await).iter().any(|f| f == "bidder"),
        "bidder is named ignored on /v1/notices");
}

/// Issue 223: an org reverse-lookup (winner/buyer/bidder) drives from the
/// participation table's `organization_id` index instead of walking the corpus. The
/// rewrite must not change WHAT the filter returns — only how fast — so this pins the
/// semantics the driving `hits` join has to preserve: the matching tender is present,
/// appears exactly once (the `DISTINCT` join must not duplicate a tender the org won
/// on several lots), and a companion filter still narrows correctly.
#[tokio::test]
async fn tenders_reverse_lookup_by_winner_preserves_semantics() {
    let server = Server::start("winner_reverse_lookup").await;
    server.ingest_chain().await;

    // A real current-version winner + its tender + that tender's source, so the
    // positive and companion cases are grounded in the fixture, not invented ids.
    let reader = server.db.readers(1).expect("readers").get().await.expect("reader");
    let mut rows = reader
        .query(
            "SELECT w.organization_id, w.tender_id, t.source
               FROM tender_version_result_winners w
               JOIN tenders t ON t.id = w.tender_id
               JOIN tender_versions v ON v.tender_id = t.id AND v.seq = w.seq
              WHERE v.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id)
              LIMIT 1",
            (),
        )
        .await
        .expect("query winners");
    let winner = rows.next().await.expect("row").map(|r| {
        (
            r.get_value(0).unwrap().as_integer().copied().unwrap(),
            r.get_value(1).unwrap().as_integer().copied().unwrap(),
            r.get_value(2).unwrap().as_text().cloned().unwrap(),
        )
    });
    drop(rows);
    drop(reader);

    if let Some((org, tender_id, source)) = winner {
        let page = server.get(&format!("/v1/tenders?winner={org}")).await;
        let hits: Vec<i64> = items(&page).iter().filter_map(|t| t["id"].as_i64()).collect();
        assert!(hits.contains(&tender_id), "the winner filter returns the won tender");
        assert_eq!(
            hits.iter().filter(|&&id| id == tender_id).count(),
            1,
            "the DISTINCT driving join must not duplicate a tender the org won more than once"
        );

        // Companion filter: the tender's own source keeps it; a different source drops it.
        let same = server.get(&format!("/v1/tenders?winner={org}&source={source}")).await;
        assert!(
            items(&same).iter().any(|t| t["id"].as_i64() == Some(tender_id)),
            "winner + the tender's own source still returns it"
        );
        let other = if source == "ted" { "doe" } else { "ted" };
        let filtered = server.get(&format!("/v1/tenders?winner={org}&source={other}")).await;
        assert!(
            !items(&filtered).iter().any(|t| t["id"].as_i64() == Some(tender_id)),
            "winner + a different source excludes the tender"
        );
    }

    // An org that won nothing → empty page via the reachable() short-circuit.
    assert!(
        items(&server.get("/v1/tenders?winner=999999999").await).is_empty(),
        "an unknown winner returns an empty page"
    );
}

/// Issue 49: an unknown or mistyped query param is a 400, so an analyst never
/// mistakes "everything matched" for "my typo'd filter matched".
#[tokio::test]
async fn unknown_query_params_are_rejected() {
    let server = Server::start("unknown_params").await;
    server.ingest_chain().await;

    // `cvp` is a typo for `cpv`: it must 400, not return every tender.
    assert_eq!(server.status("/v1/tenders?cvp=72").await, 400);
    assert_eq!(server.status("/v1/tenders?nonsense=1").await, 400);
    // The correctly-spelled filter still works.
    assert_eq!(server.status("/v1/tenders?cpv=45").await, 200);
}

/// Issue 118: a filter a collection accepts but does not apply is named in the
/// envelope's `ignored_filters`, so an unfiltered page can never be mistaken for a
/// filtered one. The field is always present; empty means every filter applied.
#[tokio::test]
async fn list_endpoints_name_the_filters_they_ignore() {
    let server = Server::start("ignored_filters").await;
    server.ingest_chain().await;

    // The dropped-filter names, in the fixed vocabulary order the envelope lists them.
    fn ignored(page: &Value) -> Vec<String> {
        page["ignored_filters"]
            .as_array()
            .expect("ignored_filters is always present")
            .iter()
            .map(|v| v.as_str().expect("a filter name").to_owned())
            .collect()
    }

    // No filters: the field is present and empty, never absent.
    assert!(ignored(&server.get("/v1/tenders").await).is_empty());
    // A filter the collection applies is never named.
    assert!(ignored(&server.get("/v1/tenders?source=ted").await).is_empty());
    // `tender` is meaningless on the Tenders collection — accepted, ignored, named.
    assert_eq!(ignored(&server.get("/v1/tenders?tender=5").await), ["tender"]);

    // Organizations carry no CPV or status, so those are named — and, the actual
    // failure mode, the page is NOT narrowed: it is the whole unfiltered set.
    let all_orgs = items(&server.get("/v1/organizations").await).len();
    let cpv_page = server.get("/v1/organizations?cpv=45&status=open").await;
    assert_eq!(ignored(&cpv_page), ["cpv", "status"], "named in vocabulary order");
    assert_eq!(items(&cpv_page).len(), all_orgs, "cpv did not narrow — it was ignored");
    // Organizations DO apply country and kind, so neither is named.
    assert!(ignored(&server.get("/v1/organizations?country=DE").await).is_empty());

    // Notices apply only source/kind through the collection read path.
    assert_eq!(ignored(&server.get("/v1/notices?country=DE").await), ["country"]);
    assert!(ignored(&server.get("/v1/notices?source=ted").await).is_empty());

    // `?tender=` on notices IS honoured — by the app-layer dispatch to the notices
    // behind a tender's versions — so it is not named; but any other filter sent
    // alongside it is still dropped, and still named.
    let tender_id = items(&server.get("/v1/tenders").await)[0]["id"].as_i64().expect("tender id");
    assert!(
        ignored(&server.get(&format!("/v1/notices?tender={tender_id}")).await).is_empty(),
        "the tender dispatch applies `tender`"
    );
    assert_eq!(
        ignored(&server.get(&format!("/v1/notices?tender={tender_id}&country=DE")).await),
        ["country"],
        "the dispatch applies `tender` only; the rest is dropped and named",
    );

    // Lots apply the whole vocabulary, so nothing is ever ignored there.
    assert!(ignored(&server.get("/v1/lots?source=ted&status=open").await).is_empty());
}

/// Issue 51: an unknown `/v1/*` path is our JSON 404, never a fall-through to
/// the dashboard's HTML router that would leak its route names.
#[tokio::test]
async fn unknown_v1_paths_are_json_404() {
    let server = Server::start("unknown_path").await;
    assert_eq!(server.status("/v1/bogus").await, 404);
    // A JSON body (get_allow_error parses it) with our envelope and no HTML.
    let body = server.get_allow_error("/v1/nope/deeper").await;
    assert_eq!(body["error"]["status"].as_u64(), Some(404));
    assert!(
        body["error"]["message"].as_str().is_some_and(|m| !m.contains('<')),
        "no HTML or route name should leak: {body}"
    );
}

/// Issue 51: every malformed input is the one documented `{"error":{…}}`
/// envelope — a bad path never leaks the Rust type `i64`, a bad query string is
/// not plain text.
#[tokio::test]
async fn malformed_inputs_return_the_json_error_envelope() {
    let server = Server::start("malformed").await;

    // A non-integer id: JSON 400 that never names `i64`.
    assert_eq!(server.status("/v1/tenders/abc").await, 400);
    let path_err = server.get_allow_error("/v1/tenders/abc").await;
    assert_eq!(path_err["error"]["status"].as_u64(), Some(400));
    assert!(
        path_err["error"]["message"].as_str().is_some_and(|m| !m.contains("i64")),
        "the Rust type must not leak: {path_err}"
    );

    // A bad query string: the same envelope, not axum's plain-text rejection.
    let query_err = server.get_allow_error("/v1/tenders?cvp=72").await;
    assert_eq!(query_err["error"]["status"].as_u64(), Some(400));
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
    // The documented order (issue 216): every collection is served in ASCENDING id
    // order, a stable keyset order — not "newest first". If a future change makes it
    // descending (real published-date sort, issue 216-B), the docs must move with it.
    assert!(all.windows(2).all(|w| w[0] < w[1]), "the list is served in ascending id order");

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
    assert!(
        snapshot[0].id.is_none(),
        "snapshot events must not be resume points: a stream that dies mid-snapshot \
         has to re-snapshot, not resume past its own missing remainder"
    );
    let boundary = live.data["cursor"].as_str().expect("live carries the cursor").to_owned();
    // The marker's id is the generation-qualified boundary (issue 46).
    let generation = live.data["generation"].as_i64().expect("live names its generation");
    assert_eq!(live.id.as_deref(), Some(format!("{generation}:{boundary}").as_str()));

    // A live stream with nothing happening stays quiet (keep-alives aside).
    assert!(tape.quiet().await, "no diffs before anything changes");

    // Step 3: a notice lands while the stream is open. The writer rings the
    // doorbell; the diff loop reads past the boundary and classifies it.
    server.ingest(LATE).await;
    let diff = tape.next().await.expect("the new Tender must reach the stream");
    assert_eq!(diff.name, "change");
    assert_eq!(diff.data["op"], "added");
    assert_eq!(diff.data["entity"], "tender");
    // The diff decorates its payload only when the client asked for it and the entity
    // matches on the new side (issue 221) — so an `added` under include_data still
    // carries the new state, not just its identity.
    assert!(diff.data["data"]["title"].is_string(), "an include_data diff embeds the new state");
    let diff_cursor: i64 = diff.data["cursor"].as_str().expect("cursor").parse().expect("number");
    assert!(diff_cursor > boundary.parse::<i64>().expect("number"), "diffs are past the boundary");
    assert_eq!(
        diff.id.as_deref(),
        Some(format!("{generation}:{diff_cursor}").as_str()),
        "diff ids are generation-qualified resume tokens"
    );
}

#[tokio::test]
async fn a_snapshot_larger_than_one_page_arrives_page_by_page_exactly_once() {
    // Page size 1 forces the snapshot through the keyset-paging path (issue
    // 55): each page takes its own pooled reader and yields before the next,
    // so nothing here may be lost, duplicated, or reordered by the paging.
    let server = Server::boot("sse-paged", Some(1)).await;
    server.ingest_chain().await;
    server.ingest(LATE).await;

    let mut tape = Tape::open(&server, "/v1/tenders", None).await;
    let (snapshot, live) = tape.until_live().await;
    assert_eq!(snapshot.len(), 2, "both Tenders arrive even though each page holds one");
    assert!(
        snapshot.iter().all(|e| e.id.is_none()),
        "no snapshot event is a resume point, on any page"
    );
    let ids: Vec<i64> =
        snapshot.iter().map(|e| e.data["id"].as_i64().expect("snapshot events carry ids")).collect();
    assert!(ids.windows(2).all(|w| w[0] < w[1]), "keyset pages walk ids strictly upward: {ids:?}");
    let boundary: i64 =
        live.data["cursor"].as_str().expect("live carries the cursor").parse().expect("number");
    assert!(boundary > 0, "the boundary is the log position captured before page one");
    assert!(tape.quiet().await, "a paged snapshot ends at live, not with re-emissions");
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
    // The boundary is a `generation:cursor` token (issue 46); compare cursors.
    let boundary_cursor: i64 =
        boundary.split(':').next_back().expect("token").parse().expect("number");
    assert!(
        event.data["cursor"].as_str().expect("cursor").parse::<i64>().expect("number")
            > boundary_cursor
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

/// Issue 46: a rebuild wipes the canonical layer (and, with `clear_changes`,
/// the log); entity ids are reissued, so state from before the wipe composes
/// with nothing after it — and before this fix the feed never said so. The
/// generation is the signal: poll clients read it in every envelope, SSE
/// clients hold generation-qualified resume tokens and get a `reset` on any
/// cross-generation resume.
#[tokio::test]
async fn a_rebuild_moves_the_generation_and_resets_stale_resumes() {
    let server = Server::start("generation").await;
    server.ingest_chain().await;

    let root = server.get("/v1").await;
    assert_eq!(root["generation"].as_i64(), Some(1), "a fresh database is generation 1");
    let poll = server.get("/v1/changes?since=0&limit=1000").await;
    assert_eq!(poll["generation"].as_i64(), Some(1));
    assert!(poll["events"].as_array().is_some_and(|e| !e.is_empty()));

    // A live client establishes a resume position under generation 1.
    let token = {
        let mut tape = Tape::open(&server, "/v1/tenders", None).await;
        let (_, live) = tape.until_live().await;
        assert_eq!(live.data["generation"].as_i64(), Some(1));
        let id = live.id.expect("live is a resume point");
        assert!(id.starts_with("1:"), "resume tokens are generation-qualified: {id}");
        id
    };

    // The operator rebuilds from scratch: layer wiped, feed dropped — the
    // exact sequence `project rebuild=true clear_changes=true` runs.
    server.db.clear_canonical().await.expect("clear canonical");
    server.db.clear_changes().await.expect("clear changes");

    let root = server.get("/v1").await;
    assert_eq!(root["generation"].as_i64(), Some(3), "each wipe moves the generation");
    let poll = server.get("/v1/changes?since=0&limit=1000").await;
    assert_eq!(poll["generation"].as_i64(), Some(3), "poll clients see the move in the envelope");

    // The stale token resumes nothing: an explicit reset, not a silent merge
    // of two worlds.
    let mut resumed = Tape::open(&server, "/v1/tenders", Some(&token)).await;
    let first = resumed.next().await.expect("an answer, not silence");
    assert_eq!(first.name, "reset");
    assert_eq!(first.data["reason"], "feed_rebuilt");
    assert_eq!(first.data["generation"].as_i64(), Some(3));

    // A bare pre-rebuild cursor (no generation to check) that points past the
    // dropped feed's head is caught by the ahead-of-head guard.
    let mut bare = Tape::open(&server, "/v1/tenders?cursor=999999", None).await;
    let first = bare.next().await.expect("an answer, not silence");
    assert_eq!(first.name, "reset");
    assert_eq!(first.data["reason"], "cursor_ahead");
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

/// Issue 164: retirement hard-deletes a Tender's versions before the diff loop
/// can probe them, so its `removed` change row used to fall into the
/// (None, None) arm and vanish — a subscriber that snapshotted the Tender kept
/// a ghost forever. The log row's own op is the only remaining witness, and
/// the feed must relay it.
#[tokio::test]
async fn a_retirement_reaches_the_stream_as_removed() {
    let server = Server::start("retire").await;
    server.ingest_chain().await;

    let mut tape = Tape::open(&server, "/v1/tenders", None).await;
    let (snapshot, _) = tape.until_live().await;
    assert_eq!(snapshot.len(), 1, "one Tender snapshotted");
    let id = snapshot[0].data["id"].as_i64().expect("entity id");

    // A second subscriber whose filter never matched the Tender. Its versions
    // are gone by diff time, so the filter cannot be evaluated against what
    // this client saw; the feed deliberately over-delivers the removal
    // (clients treat `removed` as an idempotent delete).
    let mut unmatched = Tape::open(&server, "/v1/tenders?source=doe", None).await;
    let (none, _) = unmatched.until_live().await;
    assert!(none.is_empty(), "the doe filter matches nothing here");

    // Retire through the real path: an empty plan reproduces no touched
    // Tender's key → orphaned → retired (the issue-93 test's recipe).
    server.db.reset_plan().await.expect("reset plan");
    let ids: Vec<i64> = (1..=20).collect();
    let retired = server.db.retire_regrouped_tenders(&ids, 1_000_000).await.expect("retire");
    assert!(retired >= 1, "the retirement must actually happen");

    let ev = tape.next().await.expect("the removal must reach the stream, not silence");
    assert_eq!(ev.name, "change");
    assert_eq!(ev.data["op"], "removed", "a retired Tender is a removal to its subscriber");
    assert_eq!(ev.data["id"].as_i64(), Some(id));
    assert!(ev.data["version"].is_null(), "retirement rows carry no version");
    assert!(ev.id.is_some(), "diff events are resume points");

    let ev = unmatched.next().await.expect("over-delivery is the documented contract");
    assert_eq!(ev.data["op"], "removed");
}

/// Issue 163: snapshot pages carry the same unauthenticated filters as the
/// list endpoint, so a walk-shaped filter must page on the isolated pool
/// (issue 120's routing) instead of pinning a main-pool reader page after
/// page for as long as the client stays connected. Saturating the isolated
/// pool proves where each shape runs: the walk-shaped subscription is shed,
/// the unfiltered one — main pool, untouched — still snapshots.
#[tokio::test]
async fn a_walk_shaped_snapshot_pages_on_the_isolated_pool() {
    let server = Server::start("walk-routing").await;
    server.ingest_chain().await;

    let held = server.isolated.hold_slots_for_test(server.isolated.available());

    // `country=` is a version predicate — an EXISTS probed per row, no index
    // serves it (store::read::walks). With the pool saturated the page read
    // is refused, and the stream says so rather than touching main readers.
    let mut walker = Tape::open(&server, "/v1/tenders?country=DE", None).await;
    let ev = walker.next().await.expect("the stream answers before ending");
    assert_eq!(ev.name, "error", "a saturated isolated pool sheds the walk-shaped snapshot");
    assert!(
        ev.data["message"].as_str().unwrap_or_default().contains("expensive"),
        "the shed names its reason: {}",
        ev.data
    );

    // The main pool is unaffected: an unfiltered snapshot completes.
    let mut plain = Tape::open(&server, "/v1/tenders", None).await;
    let (snapshot, _) = plain.until_live().await;
    assert_eq!(snapshot.len(), 1, "non-walk snapshots still run on the main pool");

    // Freed slots admit the same subscription again — shed is load, not a ban.
    drop(held);
    let mut walker = Tape::open(&server, "/v1/tenders?country=DE", None).await;
    let (_, live) = walker.until_live().await;
    assert!(live.data["cursor"].is_string(), "released slots admit the walk-shaped snapshot");
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

// ------------------------------------------------------------------- openapi

/// The vendored OpenAPI document and the router describe the same surface.
///
/// One direction is enforced live: every path+method the spec declares is
/// fired at the real router, and any answer of `no such endpoint` (the
/// `/v1/{*rest}` catch-all) or a 405 fails the test — the spec may not
/// describe routes nobody serves. The reverse direction rides the service
/// info: every endpoint `GET /v1` advertises must appear in the spec, so the
/// two public inventories cannot drift apart silently.
#[tokio::test]
async fn the_openapi_spec_matches_the_served_surface() {
    let server = Server::start("openapi").await;

    // The document itself: JSON, CORS-enabled for browser-based viewers.
    let response = server
        .http
        .get(format!("{}/v1/openapi.json", server.base))
        .send()
        .await
        .expect("request");
    assert!(response.status().is_success());
    assert_eq!(
        response
            .headers()
            .get("access-control-allow-origin")
            .and_then(|v| v.to_str().ok()),
        Some("*"),
        "hosted Swagger UI / Redoc need CORS to load the spec"
    );
    let spec: Value = response.json().await.expect("openapi.json parses");

    // Spec → router: every declared operation reaches a real handler.
    for (path, item) in spec["paths"].as_object().expect("paths") {
        let concrete = path.replace("{id}", "1");
        for method in item.as_object().expect("path item").keys() {
            let request = match method.as_str() {
                "get" => server.http.get(format!("{}{concrete}", server.base)),
                "post" => server.http.post(format!("{}{concrete}", server.base)),
                "delete" => server.http.delete(format!("{}{concrete}", server.base)),
                other => panic!("{path}: unexpected method {other} in the spec"),
            };
            let response = request.send().await.expect("request");
            let status = response.status().as_u16();
            assert_ne!(status, 405, "{method} {path}: served path, undeclared method");
            // 401/400/404-entity are fine (no token, no body, empty db) — only
            // the catch-all's `no such endpoint` proves the route missing.
            if status == 404 {
                let body: Value = response.json().await.expect("a JSON 404");
                assert_ne!(
                    body["error"]["message"].as_str().unwrap_or_default(),
                    "no such endpoint",
                    "{method} {path} is in the spec but not in the router"
                );
            }
        }
    }

    // Router → spec: the service info's advertised endpoints all appear.
    let root = server.get("/v1").await;
    let spec_paths = spec["paths"].as_object().expect("paths");
    for endpoint in root["endpoints"].as_array().expect("endpoints") {
        let endpoint = endpoint.as_str().expect("endpoint strings");
        assert!(
            spec_paths.contains_key(endpoint),
            "{endpoint} is advertised by GET /v1 but missing from the OpenAPI spec"
        );
    }
    assert_eq!(root["openapi"], "/v1/openapi.json", "the service info links the spec");
}

/// The unauthenticated surface is CORS-open to any origin; the token-gated
/// surface is not. Also the SSE-resume preflight: a reconnecting EventSource
/// sends Last-Event-ID, which is not CORS-safelisted, so OPTIONS must answer
/// with it allowed or browser resume dies on the second connection.
#[tokio::test]
async fn the_unauthenticated_surface_is_cors_open() {
    let server = Server::start("cors").await;

    for path in
        ["/v1", "/v1/tenders", "/v1/changes?since=0", "/v1/sql/schema", "/v1/openapi.json", "/health"]
    {
        let response = server
            .http
            .get(format!("{}{path}", server.base))
            .header("origin", "https://example.com")
            .send()
            .await
            .expect("request");
        assert_eq!(
            response.headers().get("access-control-allow-origin").and_then(|v| v.to_str().ok()),
            Some("*"),
            "{path} is CORS-open"
        );
    }

    // The SSE resume preflight.
    let preflight = server
        .http
        .request(reqwest::Method::OPTIONS, format!("{}/v1/tenders", server.base))
        .header("origin", "https://example.com")
        .header("access-control-request-method", "GET")
        .header("access-control-request-headers", "last-event-id")
        .send()
        .await
        .expect("preflight");
    assert_eq!(preflight.status().as_u16(), 204);
    let allowed = preflight
        .headers()
        .get("access-control-allow-headers")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();
    assert!(allowed.contains("last-event-id"), "SSE resume needs Last-Event-ID allowed: {allowed}");

    // Token-gated endpoints carry no CORS grant — call them server-side.
    for (method, path) in [
        (reqwest::Method::GET, "/v1/me"),
        (reqwest::Method::GET, "/v1/webhooks"),
        (reqwest::Method::POST, "/v1/sql"),
    ] {
        let response = server
            .http
            .request(method.clone(), format!("{}{path}", server.base))
            .header("origin", "https://example.com")
            .send()
            .await
            .expect("request");
        assert_eq!(
            response.headers().get("access-control-allow-origin"),
            None,
            "{method} {path} stays CORS-closed"
        );
    }
}
