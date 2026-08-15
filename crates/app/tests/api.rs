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
