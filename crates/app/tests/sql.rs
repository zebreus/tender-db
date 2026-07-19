//! Issue 07 — the read-only SQL endpoint end to end.
//!
//! The real router over a real Turso file, driven over a real socket with a
//! real Bearer token. The allow-list's unit tests (crates/app/src/v1/sql.rs)
//! prove the *classification*; this proves the whole gate holds when a hostile
//! query is actually POSTed: writes are refused and change nothing, credential
//! tables are invisible, the caps and the timeout fire, the per-token
//! concurrency limit rejects, and a genuine analytical query answers.
#![cfg(feature = "server")]

use ingest::{eforms, profile, project};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use store::{Db, Notice, Parse};
use tender_db::{accounts, v1};

const SOURCE: &str = "ted";
const FIXTURES: &str = "../ingest/tests/fixtures";

/// The chain fixture: one real Maltese procedure — a contract notice, two
/// corrigenda, and an award notice — so the canonical layer has amounts,
/// parties and classifications to analyse.
const CHAIN: [&str; 4] = [
    "eforms-chain/1-cn-16-831374-2025.xml",
    "eforms-chain/2-change-16-6281-2026.xml",
    "eforms-chain/3-change-16-18902-2026.xml",
    "eforms-chain/4-can-29-380868-2026.xml",
];

struct Server {
    db: Arc<Db>,
    base: String,
    http: reqwest::Client,
    fetch_id: i64,
    token: String,
    path: String,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

impl Server {
    /// Boot the API over a scratch database with one account and one token
    /// already minted — the SQL endpoint is account-gated, so every test needs
    /// a credential in hand.
    async fn start(name: &str) -> Server {
        let path = format!("/tmp/tender-db-sql-{name}-{}.db", std::process::id());
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

        let account = accounts::register(&db, "analyst", "a long password")
            .await
            .expect("register")
            .0;
        let token = accounts::create_token(&db, account.id, "cli").await.expect("token").token;

        let state = v1::AppState::new(db.clone(), db.readers(4).expect("readers"));
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
            token,
            path,
        }
    }

    /// POST a query with the account's token.
    async fn sql(&self, query: &str) -> reqwest::Response {
        self.sql_as(Some(&self.token), query).await
    }

    async fn sql_as(&self, token: Option<&str>, query: &str) -> reqwest::Response {
        let mut request =
            self.http.post(format!("{}/v1/sql", self.base)).body(query.to_owned());
        if let Some(token) = token {
            request = request.header("authorization", format!("Bearer {token}"));
        }
        request.send().await.expect("request")
    }

    /// Run a fixture through the real dispatch + parse chain and project it —
    /// the same path the processor takes from an archive.
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
                },
                &parse,
            )
            .await
            .expect("record notice");
        assert!(matches!(parse, Parse::Parsed(_)), "{relative}: {parse:?}");
        project::project(&self.db, false).await.expect("project");
    }

    async fn ingest_chain(&self) {
        for fixture in CHAIN {
            self.ingest(fixture).await;
        }
    }
}

/// `rows` count a successful SQL response reports.
fn row_count(body: &Value) -> usize {
    body["rows"].as_array().expect("rows is an array").len()
}

// ------------------------------------------------------------- the gate

#[tokio::test(flavor = "multi_thread")]
async fn the_gate_refuses_everything_that_is_not_a_read() {
    let server = Server::start("adversarial").await;
    server.ingest_chain().await;

    // Every one of these must be refused with 400, and none may change the
    // database. The writes are blocked twice over — by the allow-list before
    // execution, and by query_only if they somehow reached a connection.
    let attacks = [
        "INSERT INTO fetches(source,kind,period,url,sha256,bytes,fetched_at,path) \
         VALUES('x','x','x','x','x',1,1,'x')",
        "UPDATE notices SET profile = 'x'",
        "DELETE FROM notices",
        "DROP TABLE notices",
        "CREATE TABLE evil (a)",
        // The documented query_only escape hatch, as a lone statement…
        "PRAGMA query_only = 0",
        // …and smuggled after a write in a multi-statement body.
        "PRAGMA query_only = 0; INSERT INTO fetches(source,kind,period,url,sha256,bytes,fetched_at,path) \
         VALUES('x','x','x','x','x',1,1,'x')",
        "ATTACH DATABASE '/etc/passwd' AS leak",
        "VACUUM",
        "VACUUM INTO '/tmp/exfil.db'",
        // A write hidden inside a CTE — parses as INSERT, not SELECT.
        "WITH x AS (SELECT 1) INSERT INTO notices SELECT * FROM x",
        // Two statements, the second hostile.
        "SELECT 1; DROP TABLE notices",
    ];
    for attack in attacks {
        let status = server.sql(attack).await.status();
        assert_eq!(status, 400, "attack should be 400: {attack:?}");
    }

    // The database is exactly as the chain left it — nothing was written.
    let notices: Value = server.sql("SELECT COUNT(*) AS n FROM notices").await.json().await.unwrap();
    assert_eq!(notices["rows"][0][0].as_i64(), Some(4), "the four chain notices, untouched");
    let fetches: Value = server.sql("SELECT COUNT(*) AS n FROM fetches").await.json().await.unwrap();
    assert_eq!(fetches["rows"][0][0].as_i64(), Some(1), "still only the one fetch");
    // The account the DROP/DELETE attempts could not touch is still there.
    assert!(server.db.user(1).await.expect("user").is_some(), "the account survived");
}

#[tokio::test(flavor = "multi_thread")]
async fn credential_tables_are_invisible() {
    let server = Server::start("credentials").await;

    for query in [
        "SELECT * FROM users",
        "SELECT password_hash FROM users",
        "SELECT token_hash FROM api_tokens",
        "SELECT id_hash FROM sessions",
        "SELECT * FROM v_tenders WHERE id IN (SELECT user_id FROM sessions)",
    ] {
        assert_eq!(server.sql(query).await.status(), 400, "must deny: {query:?}");
    }

    // A string literal that merely contains the word is fine — it is data, not
    // a table reference.
    let ok = server.sql("SELECT 'sessions expire' AS note").await;
    assert_eq!(ok.status(), 200);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_endpoint_is_account_gated() {
    let server = Server::start("gated").await;
    assert_eq!(server.sql_as(None, "SELECT 1").await.status(), 401);
    assert_eq!(server.sql_as(Some("tdb_not_a_real_token"), "SELECT 1").await.status(), 401);
    assert_eq!(server.sql("SELECT 1").await.status(), 200);
}

// ---------------------------------------------------------- caps & limits

#[tokio::test(flavor = "multi_thread")]
async fn results_are_capped_and_flagged() {
    let server = Server::start("caps").await;

    // 20 000 rows asked for, 10 000 returned, and the truncation is announced.
    let body: Value = server
        .sql("SELECT value FROM generate_series(1, 20000)")
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(row_count(&body), 10_000);
    assert_eq!(body["truncated"], Value::Bool(true));

    // A small result is not flagged.
    let small: Value =
        server.sql("SELECT value FROM generate_series(1, 5)").await.json().await.unwrap();
    assert_eq!(row_count(&small), 5);
    assert_eq!(small["truncated"], Value::Bool(false));
}

/// The 10 s watchdog interrupts a long query between the rows it streams.
///
/// The work is deliberately shaped as *many rows, each individually expensive*
/// (a per-row aggregate) rather than one giant aggregate: turso only yields to
/// tokio at row boundaries, so a query that never emits a row cannot be dropped
/// by `tokio::time::timeout` (a documented engine limit — see the endpoint's
/// module docs). A per-row cost keeps the total well past 10 s on any machine
/// while giving the watchdog a row boundary to fire on.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_long_query_is_dropped_at_the_time_limit() {
    let server = Server::start("timeout").await;
    // The subquery's range depends on the outer row, so it cannot be hoisted to
    // a constant — it is recomputed for each of the 2000 rows, and the whole
    // thing far outlasts 10 s while yielding a row boundary to fire on.
    let slow = "SELECT a.value, \
                  (SELECT COUNT(*) FROM generate_series(a.value, a.value + 20000000)) \
                FROM generate_series(1, 2000) a";
    let started = std::time::Instant::now();
    let status = server.sql(slow).await.status().as_u16();
    let elapsed = started.elapsed();
    assert_eq!(status, 408, "a query past the limit should be a timeout");
    assert!(elapsed < Duration::from_secs(30), "it should stop near the 10s limit, not run on: {elapsed:?}");
}

/// A third simultaneous query is refused rather than queued. The two running
/// queries hold both permits when the third arrives, so it is rejected
/// immediately — the permit check happens before any work starts, so this holds
/// regardless of how fast the queries themselves are.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_third_concurrent_query_is_rejected() {
    let server = Arc::new(Server::start("concurrency").await);

    // A streaming query with real per-row work (a correlated subquery keeps the
    // optimiser from shortcutting it) runs a few seconds — long enough for all
    // three requests to reach the gate together.
    let busy = "SELECT a.value, \
                  (SELECT COUNT(*) FROM generate_series(a.value * 1000000, a.value * 1000000 + 5000000)) \
                FROM generate_series(1, 30) a";
    let mut handles = Vec::new();
    for _ in 0..3 {
        let server = server.clone();
        handles.push(tokio::spawn(async move { server.sql(busy).await.status().as_u16() }));
    }
    let mut statuses = Vec::new();
    for handle in handles {
        statuses.push(handle.await.expect("join"));
    }
    statuses.sort_unstable();
    // Exactly one is turned away for concurrency; the other two are admitted —
    // whether they then finish (200) or hit the time limit (408) is not what
    // this test is about, only that the third could not get a permit.
    assert_eq!(
        statuses.iter().filter(|s| **s == 429).count(),
        1,
        "exactly one query should be rejected for concurrency: {statuses:?}"
    );
    assert!(
        statuses.iter().filter(|s| **s != 429).all(|s| *s == 200 || *s == 408),
        "the two admitted queries ran: {statuses:?}"
    );
}

// -------------------------------------------------------- it really queries

#[tokio::test(flavor = "multi_thread")]
async fn a_real_analytical_query_answers() {
    let server = Server::start("analytics").await;
    server.ingest_chain().await;

    // Proof that data actually flows: the four chain notices, grouped.
    let counts: Value = server
        .sql("SELECT profile, COUNT(*) AS n FROM notices GROUP BY profile ORDER BY n DESC")
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(counts["columns"], serde_json::json!(["profile", "n"]));
    let total: i64 = counts["rows"].as_array().unwrap().iter().map(|r| r[1].as_i64().unwrap()).sum();
    assert_eq!(total, 4);

    // The acceptance query: top buyers by awarded cents, scoped to a CPV range,
    // over the canonical layer exactly as a client would write it.
    let response = server
        .sql(
            "SELECT o.name, SUM(a.cents) AS total_cents
               FROM v_tender_current c
               JOIN tender_version_parties p
                 ON p.tender_id = c.tender_id AND p.seq = c.seq AND p.role = 'buyer'
               JOIN organizations o ON o.id = p.organization_id
               JOIN tender_version_amounts a
                 ON a.tender_id = c.tender_id AND a.seq = c.seq
               JOIN tender_version_classifications x
                 ON x.tender_id = c.tender_id AND x.seq = c.seq AND x.scheme = 'cpv'
              WHERE x.code LIKE '7%'
              GROUP BY o.id
              ORDER BY total_cents DESC
              LIMIT 10",
        )
        .await;
    assert_eq!(response.status(), 200, "the analytical query should run");
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["columns"], serde_json::json!(["name", "total_cents"]));
    assert_eq!(body["truncated"], Value::Bool(false));
}

#[tokio::test(flavor = "multi_thread")]
async fn the_schema_endpoint_documents_the_public_surface() {
    let server = Server::start("schema").await;
    let body: Value = server
        .http
        .get(format!("{}/v1/sql/schema", server.base))
        .send()
        .await
        .expect("request")
        .json()
        .await
        .unwrap();

    let names: Vec<&str> =
        body["tables"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"v_tenders"), "views are documented: {names:?}");
    assert!(names.contains(&"notices"), "base tables are documented: {names:?}");
    // The credential tables are never advertised.
    for hidden in ["users", "api_tokens", "sessions"] {
        assert!(!names.contains(&hidden), "{hidden} must not appear in the schema");
    }
    assert!(!body["notes"].as_array().unwrap().is_empty(), "dialect notes are present");

    // A named object carries its columns.
    let tenders = body["tables"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "v_tenders")
        .expect("v_tenders present");
    assert!(!tenders["columns"].as_array().unwrap().is_empty());
}

// -------------------------------------------------------- runtime isolation

/// Issue 17: SQL execution runs on its own runtime, so a pathological
/// non-yielding aggregate — which no timeout can interrupt — cannot starve the
/// main API/SSE runtime.
///
/// The whole server runs on a two-thread runtime here. Two heavy aggregates
/// (each computes in a single non-yielding poll) would, without isolation, pin
/// both of those threads and freeze the API. With execution on the isolated
/// runtime, the two main threads stay free and `/health` keeps answering
/// promptly — which is what this asserts.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sql_execution_does_not_starve_the_api() {
    let server = Arc::new(Server::start("isolation").await);

    // A finite but heavy cross-join count: ~tens of millions of rows aggregated
    // in one poll, no row boundary to yield on — it pins a worker thread for a
    // few seconds. Two of them take both of this user's concurrency permits.
    let bomb = "SELECT COUNT(*) FROM generate_series(1, 7000) a, generate_series(1, 7000) b";
    let mut running = Vec::new();
    for _ in 0..2 {
        let server = server.clone();
        running.push(tokio::spawn(async move { server.sql(bomb).await.status().as_u16() }));
    }
    // Let them reach the isolated runtime and start pinning its threads.
    tokio::time::sleep(Duration::from_millis(300)).await;

    // The main runtime must still answer a cheap, non-SQL request quickly. Were
    // execution on this runtime, both worker threads would be pinned and these
    // would block for the whole several-second query.
    let started = std::time::Instant::now();
    for _ in 0..5 {
        let health = server
            .http
            .get(format!("{}/health", server.base))
            .send()
            .await
            .expect("health request");
        assert_eq!(health.status(), 200);
    }
    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_secs(2),
        "the API stayed responsive while SQL was pinned: {elapsed:?}"
    );

    // Drain the aggregates. Their own status is not the point — under parallel
    // test load a cross join can exceed the 10 s wall-clock limit and come back
    // 408, which is fine; what mattered is that they never blocked `/health`.
    for handle in running {
        let status = handle.await.expect("join");
        assert!(status == 200 || status == 408, "the query ran to a normal result: {status}");
    }
}
