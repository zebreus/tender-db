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
        Server::start_with_sql_timeout(name, v1::sql::DEFAULT_TIMEOUT).await
    }

    /// Boot with an explicit `/v1/sql` time limit — the seam that lets the cap be
    /// observed firing on a non-yielding aggregate without a 10 s query.
    async fn start_with_sql_timeout(name: &str, sql_timeout: Duration) -> Server {
        let path = format!("/tmp/tender-db-sql-{name}-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        tender_db::plan_capture::install_from_env();
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

        let state =
            v1::AppState::with_sql_timeout(db.clone(), db.readers(v1::sql::SQL_READERS).expect("readers"), sql_timeout);
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
                    published_at: None,
                    dispatched_at: None,
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
    // fetches is NOT on the public allow-list — its `path` column is server
    // filesystem layout, operator infra not business data (issue 45) — so the
    // read is denied; confirm the write was blocked via the DB directly instead.
    assert_eq!(server.sql("SELECT COUNT(*) FROM fetches").await.status(), 400, "fetches is denied");
    let packages = server.db.current_packages(SOURCE, "daily", None).await.expect("packages");
    assert_eq!(packages.len(), 1, "still only the one fetch — no write landed");
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

/// Issue 204: a pragma table-valued function returns a table's SCHEMA — column
/// names, default values — without a base-table reference, so before the TVF
/// allow-list `SELECT * FROM pragma_table_info('users')` returned 200 and
/// disclosed the credential tables' columns. Every such probe must 400, and the
/// credential column names must never appear in a response body.
#[tokio::test(flavor = "multi_thread")]
async fn pragma_table_functions_cannot_disclose_credential_schema() {
    let server = Server::start("pragma-leak").await;
    server.ingest_chain().await;

    for query in [
        "SELECT * FROM pragma_table_info('users')",
        "SELECT * FROM pragma_table_xinfo('users')",
        "SELECT * FROM pragma_table_info('api_tokens')",
        "SELECT name FROM pragma_table_list",
        "WITH x AS (SELECT * FROM pragma_table_info('users')) SELECT * FROM x",
        "SELECT * FROM v_tenders WHERE 1 IN pragma_table_info('users')",
    ] {
        let resp = server.sql(query).await;
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        assert_eq!(status, 400, "must deny the pragma TVF: {query:?} (body {body})");
        // The credential column names must not surface even in the schema.
        for secret_col in ["password_hash", "token_hash", "id_hash"] {
            assert!(
                !body.contains(secret_col),
                "{query:?} leaked the {secret_col} column: {body}"
            );
        }
    }

    // The one allow-listed TVF still works — the fix is a scalpel, not a ban.
    assert_eq!(server.sql("SELECT * FROM generate_series(1, 3)").await.status(), 200);
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
/// module docs). A per-row cost keeps the total well past 15 s on any machine
/// while giving the watchdog a row boundary to fire on.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_long_query_is_dropped_at_the_time_limit() {
    let server = Server::start("timeout").await;
    // The subquery's range depends on the outer row, so it cannot be hoisted to
    // a constant — it is recomputed for each of the 2000 rows, and the whole
    // thing far outlasts 15 s while yielding a row boundary to fire on.
    let slow = "SELECT a.value, \
                  (SELECT COUNT(*) FROM generate_series(a.value, a.value + 20000000)) \
                FROM generate_series(1, 2000) a";
    let started = std::time::Instant::now();
    let status = server.sql(slow).await.status().as_u16();
    let elapsed = started.elapsed();
    assert_eq!(status, 408, "a query past the limit should be a timeout");
    assert!(elapsed < Duration::from_secs(30), "it should stop near the 15s limit, not run on: {elapsed:?}");
}

/// A single non-yielding aggregate — the `COUNT(*)`/`GROUP BY` shape that
/// computes in one poll with no row boundary — is capped at the time limit and
/// answered 408, rather than running past 40 s with no timeout (issue 51). Since
/// issue 425 it is an interrupt at the limit that stops it (a timer and turso's
/// `interrupt()` since issue 438); the handler-side backstop remains for what that
/// check does not reach. A short cap keeps the test quick and
/// the abandoned query brief; the aggregate far outlasts it on any machine.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_non_yielding_aggregate_is_capped() {
    let server = Server::start_with_sql_timeout("aggregate", Duration::from_millis(300)).await;
    // One aggregate over a large cross join: no row is ever emitted, so the
    // in-task timeout has no boundary to fire on — only the backstop can end it.
    // 49M iterations far outlast a 300ms cap on any machine.
    let bomb = "SELECT COUNT(*) FROM generate_series(1, 7000) a, generate_series(1, 7000) b";
    let started = std::time::Instant::now();
    let response = server.sql(bomb).await;
    let status = response.status().as_u16();
    let elapsed = started.elapsed();
    assert_eq!(status, 408, "a non-yielding aggregate past the cap should be a timeout");
    assert!(elapsed < Duration::from_secs(10), "the backstop should fire near the cap: {elapsed:?}");
    // The 408 is our JSON error envelope, not plain text (issue 51).
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["error"]["status"].as_i64(), Some(408));
}

/// Issue 425 (superseding what issue 417 pinned here): a capped computation no
/// longer runs on after its 408. Until turso could be told to stop a statement, the
/// two bombs below each kept computing on their own blocking thread — abandoned,
/// counted, and the gauge read 2 (and on prod on 2026-09-18 two such computations
/// denied the endpoint for 13.5 minutes). Now each one is interrupted at the limit
/// (issue 438: a timer and `interrupt()`, no longer turso's per-statement deadline),
/// so by the time its 408 is answered it has already ended: the gauge reads 0,
/// nothing is left burning, and a cheap query answers at once.
///
/// Sequential, not concurrent, so the per-token concurrency cap never
/// enters: each bomb's permit is released when its 408 is answered.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_capped_computation_is_stopped_not_abandoned() {
    let server = Server::start_with_sql_timeout("abandoned", Duration::from_millis(300)).await;
    let bomb = "SELECT COUNT(*) FROM generate_series(1, 7000) a, generate_series(1, 7000) b";
    for n in 1..=2 {
        let started = std::time::Instant::now();
        let response = server.sql(bomb).await;
        let status = response.status().as_u16();
        assert_eq!(status, 408, "bomb {n} runs past the cap");
        assert!(started.elapsed() < Duration::from_secs(5), "bomb {n} answers at the cap: {:?}", started.elapsed());
        let body: Value = response.json().await.unwrap();
        let message = body["error"]["message"].as_str().unwrap_or_default().to_owned();
        assert!(message.contains("time limit"), "the engine's interrupt reads as the time limit, not a SQL error: {message}");
    }
    // Nothing was left running for nobody.
    let metrics = server
        .http
        .get(format!("{}/metrics", server.base))
        .send()
        .await
        .expect("metrics")
        .text()
        .await
        .expect("metrics body");
    let gauge = metrics
        .lines()
        .find(|l| l.starts_with("tender_db_sql_pinned_computations "))
        .expect("the pinned gauge is served");
    assert_eq!(gauge, "tender_db_sql_pinned_computations 0", "the engine stopped both — none is abandoned");

    // …and the endpoint answers a cheap query at once, right after two interrupts
    // (on a fresh connection: since issue 438 an interrupted one is never lent again).
    let started = std::time::Instant::now();
    let response = server.sql("SELECT 1 AS ok").await;
    let status = response.status().as_u16();
    let elapsed = started.elapsed();
    assert_eq!(status, 200, "a query after two stopped computations runs");
    assert!(elapsed < Duration::from_secs(2), "and runs promptly: {elapsed:?}");
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["rows"][0][0].as_i64(), Some(1));
}

/// One query past the per-token cap is refused rather than queued. The running
/// queries hold every permit when the extra one arrives, so it is rejected
/// immediately — the permit check happens before any work starts, so this holds
/// regardless of how fast the queries themselves are. Reads the cap
/// ([`v1::sql::MAX_CONCURRENT`], 4 since 2026-09-30) rather than restating it.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_query_past_the_concurrency_cap_is_rejected() {
    let server = Arc::new(Server::start("concurrency").await);

    // A streaming query with real per-row work (a correlated subquery keeps the
    // optimiser from shortcutting it) runs a few seconds — long enough for all
    // three requests to reach the gate together.
    let busy = "SELECT a.value, \
                  (SELECT COUNT(*) FROM generate_series(a.value * 1000000, a.value * 1000000 + 5000000)) \
                FROM generate_series(1, 30) a";
    let mut handles = Vec::new();
    for _ in 0..=v1::sql::MAX_CONCURRENT {
        let server = server.clone();
        handles.push(tokio::spawn(async move { server.sql(busy).await.status().as_u16() }));
    }
    let mut statuses = Vec::new();
    for handle in handles {
        statuses.push(handle.await.expect("join"));
    }
    statuses.sort_unstable();
    // Exactly one is turned away for concurrency; the others are admitted —
    // whether they then finish (200) or hit the time limit (408) is not what
    // this test is about, only that the third could not get a permit.
    assert_eq!(
        statuses.iter().filter(|s| **s == 429).count(),
        1,
        "exactly one query should be rejected for concurrency: {statuses:?}"
    );
    assert!(
        statuses.iter().filter(|s| **s != 429).all(|s| *s == 200 || *s == 408),
        "the admitted queries ran: {statuses:?}"
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
    // over the canonical layer exactly as a client would write it — in the form the
    // surface documents since issue 239: the head version read off
    // `tenders.current_seq`, never through the refused `v_tender_current` pointer
    // view (this test carried the pre-239 shape and was red, unrun, for twelve days —
    // issue 414).
    let response = server
        .sql(
            "SELECT o.name, SUM(a.cents) AS total_cents
               FROM tenders t
               JOIN tender_version_parties p
                 ON p.tender_id = t.id AND p.seq = t.current_seq AND p.role = 'buyer'
               JOIN organizations o ON o.id = p.organization_id
               JOIN tender_version_amounts a
                 ON a.tender_id = t.id AND a.seq = t.current_seq
               JOIN tender_version_classifications x
                 ON x.tender_id = t.id AND x.seq = t.current_seq AND x.scheme = 'cpv'
              WHERE x.code LIKE '7%'
              GROUP BY o.id
              ORDER BY total_cents DESC
              LIMIT 10",
        )
        .await;
    let status = response.status();
    let text = response.text().await.unwrap();
    assert_eq!(status, 200, "the analytical query should run: {text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["columns"], serde_json::json!(["name", "total_cents"]));
    assert_eq!(body["truncated"], Value::Bool(false));
}

/// Issue 50: the analyst convenience views answer the common questions and are
/// queryable through the allow-list; v_fetches surfaces provenance without the
/// filesystem path.
#[tokio::test(flavor = "multi_thread")]
async fn the_analyst_views_answer() {
    let server = Server::start("analyst_views").await;
    server.ingest_chain().await;

    for view in [
        "v_tender_buyers",
        "v_awards",
        "v_tender_classifications",
        "v_tender_amounts",
        "v_tender_dates",
        "v_tender_notices",
        "v_fetches",
    ] {
        let status = server.sql(&format!("SELECT * FROM {view} LIMIT 5")).await.status();
        assert_eq!(status, 200, "{view} must be queryable through the allow-list");
    }

    // The chain publishes a buyer, CPV codes and causing notices — the views see them.
    let count = |v: &Value| v["rows"][0][0].as_i64().unwrap_or_else(|| panic!("no count in {v}"));
    let buyers: Value = server.sql("SELECT count(*) FROM v_tender_buyers").await.json().await.unwrap();
    assert!(count(&buyers) >= 1, "the chain has a buyer");
    // A FILTERED read of a `v_*` view is refused since issue 239 (the view would be
    // scanned whole); the documented form joins the satellite on the head version.
    let cpv: Value = server
        .sql(
            "SELECT count(*) FROM tenders t
               JOIN tender_version_classifications x
                 ON x.tender_id = t.id AND x.seq = t.current_seq
              WHERE x.scheme = 'cpv'",
        )
        .await
        .json()
        .await
        .unwrap();
    assert!(count(&cpv) >= 1, "the chain classifies by CPV");
    let notices: Value = server.sql("SELECT count(*) FROM v_tender_notices").await.json().await.unwrap();
    assert!(count(&notices) >= 1, "the tender's versions name their causing notices");

    // v_fetches exposes provenance but never the server filesystem path (issue 45).
    let fetches: Value = server.sql("SELECT * FROM v_fetches LIMIT 1").await.json().await.unwrap();
    let cols: Vec<&str> = fetches["columns"].as_array().unwrap().iter().filter_map(|c| c.as_str()).collect();
    assert!(!cols.contains(&"path"), "v_fetches must not expose the filesystem path: {cols:?}");
    assert!(cols.contains(&"period"), "v_fetches surfaces source provenance: {cols:?}");
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

/// Issue 50: the schema flags the epoch-seconds time format, lists enum
/// vocabularies, describes tables, ships worked examples, and hides internal
/// turso bookkeeping tables.
#[tokio::test(flavor = "multi_thread")]
async fn the_schema_documents_time_format_and_enums() {
    let server = Server::start("schema_notes").await;
    let body: Value = server
        .http
        .get(format!("{}/v1/sql/schema", server.base))
        .send()
        .await
        .expect("request")
        .json()
        .await
        .unwrap();

    let tables = body["tables"].as_array().unwrap();
    let names: Vec<&str> = tables.iter().map(|t| t["name"].as_str().unwrap()).collect();
    // Internal turso bookkeeping tables never surface.
    assert!(!names.iter().any(|n| n.starts_with("__turso")), "internal tables hidden: {names:?}");

    // v_tenders has a description, and its published_at column flags the epoch
    // trap with a strftime hint.
    let v_tenders = tables.iter().find(|t| t["name"] == "v_tenders").expect("v_tenders");
    assert!(v_tenders["note"].as_str().is_some_and(|n| !n.is_empty()), "table note present");
    let published = v_tenders["columns"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "published_at")
        .expect("published_at column");
    assert!(
        published["note"].as_str().is_some_and(|n| n.contains("strftime")),
        "the time-format trap is flagged on the column: {published}"
    );

    // Issue 50, the clause its closure dropped: a VIEW's columns carry their base
    // column's declared type, not the `TEXT` the PRAGMA reports for every view column.
    let col_type = |table: &str, col: &str| -> Value {
        tables
            .iter()
            .find(|t| t["name"] == table)
            .unwrap_or_else(|| panic!("{table}"))["columns"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == col)
            .unwrap_or_else(|| panic!("{table}.{col}"))["type"]
            .clone()
    };
    assert_eq!(col_type("v_tenders", "id"), "INTEGER", "the base column's type, through the view");
    assert_eq!(col_type("v_tenders", "published_at"), "INTEGER");
    assert_eq!(col_type("v_tenders", "title"), "TEXT");
    assert_eq!(col_type("v_fetches", "bytes"), "INTEGER", "a bare-column view over one source");
    assert_eq!(col_type("v_organizations", "mentions"), "INTEGER", "COUNT(*) is an integer");
    assert_eq!(col_type("v_lots", "title"), "TEXT", "a scalar subquery follows the column it projects");
    // Issue 490: the stored lot value, through the view.
    assert_eq!(col_type("v_lots", "value_cents"), "INTEGER");
    assert_eq!(col_type("v_lots", "value_currency"), "TEXT");
    assert_eq!(col_type("v_lots", "value_eur_cents"), "INTEGER");
    assert_eq!(col_type("v_awards", "winner_name"), "TEXT", "a view over a view over a view resolves through");
    assert_eq!(col_type("v_awards", "awarded_cents"), "INTEGER");
    // Every integer key of every view resolves to INTEGER — no view column that is a
    // row id or a sequence may read TEXT any more, whatever the PRAGMA says.
    // (`section_id` is a genuine TEXT key, `"RES-1"`, and is not in this set.)
    const INTEGER_KEYS: &[&str] = &["id", "tender_id", "notice_id", "organization_id", "lot_id", "lot_result_id", "seq"];
    for t in tables.iter().filter(|t| t["type"] == "view") {
        for c in t["columns"].as_array().unwrap() {
            let name = c["name"].as_str().unwrap();
            if INTEGER_KEYS.contains(&name) || name.ends_with("_organization_id") {
                assert_eq!(c["type"], "INTEGER", "{}.{name} must resolve to INTEGER: {c}", t["name"]);
            }
        }
    }

    // parse_state's vocabulary is listed on its column.
    let parse_state = tables
        .iter()
        .find(|t| t["name"] == "notices")
        .expect("notices")["columns"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "parse_state")
        .expect("parse_state column")
        .clone();
    assert!(parse_state["note"].as_str().is_some_and(|n| n.contains("quarantined")), "enum listed");

    // Top-level notes call out epoch time and the mid-backfill scope; examples ship.
    let notes = body["notes"].as_array().unwrap().iter().filter_map(|n| n.as_str()).collect::<String>();
    assert!(notes.contains("epoch") && notes.contains("strftime"), "epoch note present");
    assert!(notes.to_lowercase().contains("backfill"), "mid-backfill scope noted");
    assert!(!body["examples"].as_array().unwrap().is_empty(), "worked examples present");
}

/// Issue 484 unit 3: the buyer-equal winner flag is visible on the table and on both
/// award views, each with the note that tells a supplier statistic to filter it.
#[tokio::test(flavor = "multi_thread")]
async fn the_winner_is_buyer_columns_exist_and_say_how_to_filter() {
    let server = Server::start("winner_is_buyer_notes").await;
    let body: Value = server
        .http
        .get(format!("{}/v1/sql/schema", server.base))
        .send()
        .await
        .expect("request")
        .json()
        .await
        .unwrap();
    let tables = body["tables"].as_array().unwrap();
    let column = |table: &str, col: &str| -> Value {
        tables
            .iter()
            .find(|t| t["name"] == table)
            .unwrap_or_else(|| panic!("{table}"))["columns"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == col)
            .unwrap_or_else(|| panic!("{table}.{col} missing"))
            .clone()
    };
    for (table, col, filter) in [
        ("tender_version_result_winners", "is_buyer", "is_buyer IS NULL"),
        ("v_lot_results", "winner_is_buyer", "winner_is_buyer IS NULL"),
        ("v_awards", "winner_is_buyer", "winner_is_buyer IS NULL"),
    ] {
        let c = column(table, col);
        assert_eq!(c["type"], "INTEGER", "{table}.{col}: {c}");
        assert!(c["note"].as_str().is_some_and(|n| n.contains(filter) && n.contains("never 0")), "{table}.{col}: {c}");
    }
    for view in ["v_lot_results", "v_awards"] {
        let note = tables.iter().find(|t| t["name"] == view).unwrap()["note"].as_str().unwrap().to_owned();
        assert!(note.contains("winner_is_buyer IS NULL"), "{view}: {note}");
    }
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
    // few seconds. Two of them take two of this user's concurrency permits.
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

/// The dialect canary (ADR-0015 D2): representative analyst query shapes that
/// must keep running on the exact engine we ship. The /v1/sql contract is
/// names and shapes, not the engine — so when a turso upgrade breaks one of
/// these, THIS test turns the gate red before the deploy does it to an
/// analyst's saved query, and the resolution (carry the break with a
/// CHANGELOG entry, or hold the upgrade) becomes a decision instead of a
/// surprise. Shapes the docs DOCUMENT as unsupported (WITH RECURSIVE,
/// rank/lead/lag) are deliberately absent; `row_number() OVER ()` is present
/// because the docs promise it works.
#[tokio::test(flavor = "multi_thread")]
async fn the_dialect_canary_shapes_all_run() {
    let server = Server::start("dialect_canary").await;
    let shapes: [(&str, &str); 12] = [
        ("join", "SELECT t.id FROM tenders t JOIN tender_versions v ON v.tender_id = t.id AND v.seq = t.current_seq LIMIT 5"),
        ("group-by-having", "SELECT source, COUNT(*) AS n FROM notices GROUP BY source HAVING COUNT(*) >= 0 ORDER BY n DESC LIMIT 5"),
        ("cte", "WITH recent AS (SELECT id FROM notices ORDER BY id DESC LIMIT 5) SELECT COUNT(*) FROM recent"),
        ("strftime", "SELECT strftime('%Y', published_at, 'unixepoch') AS y, COUNT(*) FROM tender_versions GROUP BY y LIMIT 5"),
        ("case", "SELECT CASE WHEN cents < 0 THEN 'neg' WHEN cents = 0 THEN 'zero' ELSE 'pos' END AS b, COUNT(*) FROM tender_version_amounts GROUP BY b"),
        ("correlated-subquery", "SELECT id, (SELECT COUNT(*) FROM tender_versions v WHERE v.tender_id = t.id) FROM tenders t LIMIT 5"),
        ("like", "SELECT COUNT(*) FROM notices WHERE profile LIKE 'eforms%'"),
        ("in-subquery", "SELECT COUNT(*) FROM tenders WHERE id IN (SELECT tender_id FROM tender_versions LIMIT 10)"),
        ("union", "SELECT 'a' AS k UNION ALL SELECT 'b' ORDER BY k"),
        ("cast-coalesce", "SELECT COALESCE(CAST(NULL AS INTEGER), 42)"),
        ("row-number-over", "SELECT id, row_number() OVER () FROM tenders LIMIT 3"),
        ("rates-lookup", "SELECT rate_to_eur, source FROM currency_rates WHERE currency = 'DEM' AND rate_date <= '1999-06-01' ORDER BY rate_date DESC LIMIT 1"),
    ];
    for (label, query) in shapes {
        let response = server.sql(query).await;
        assert_eq!(
            response.status(),
            200,
            "dialect canary `{label}` no longer runs — a turso upgrade changed the dialect; \
             carry it with a CHANGELOG entry or hold the upgrade (ADR-0015 D2): {}",
            response.text().await.unwrap_or_default()
        );
    }
}

/// Issue 457 unit 3, end to end: recursion (keyword or self-reference) and EXCLUDE
/// frames are refused with a 400 that names the reason, on any engine.
#[tokio::test(flavor = "multi_thread")]
async fn recursion_and_exclude_frames_are_refused() {
    let server = Server::start("unbounded-constructs").await;
    for query in [
        "WITH RECURSIVE c(n) AS (SELECT 1 UNION ALL SELECT n + 1 FROM c WHERE n < 5) SELECT n FROM c",
        "WITH tenders AS (SELECT id FROM tenders UNION ALL SELECT id + 1 FROM tenders) SELECT COUNT(*) FROM tenders",
        "SELECT id, count(*) OVER (ORDER BY id ROWS BETWEEN 1 PRECEDING AND 1 FOLLOWING EXCLUDE CURRENT ROW) FROM lots",
    ] {
        let resp = server.sql(query).await;
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        assert_eq!(status, 400, "must refuse: {query:?} (body {body})");
        assert!(body.contains("issue 457"), "the 400 must name the reason: {query:?} → {body}");
    }
}

/// Issue 457 unit 3, the ENGINE canary: functions turso 0.7.2 does not have and
/// 0.8.1 does. Each is refused today by the engine, not by `classify`. DECIDED
/// (2026-10-08): on the bump they are ALLOWED -- the window functions keep
/// fixed-size state per row (nothing beyond the `OVER ()` buffering 0.7.2 already
/// does) and the three scalars do not amplify output (the 457 evaluation). So when
/// this turns red on the bump, move each shape into `the_dialect_canary_shapes_all_run`
/// and update the "dialect gaps" schema note -- do not add them to BANNED_FUNCTIONS.
#[tokio::test(flavor = "multi_thread")]
async fn engine_canary_functions_new_in_turso_0_8_are_not_yet_available() {
    let server = Server::start("engine-canary-0-8").await;
    for query in [
        "SELECT rank() OVER (ORDER BY id) FROM tenders LIMIT 1",
        "SELECT dense_rank() OVER (ORDER BY id) FROM tenders LIMIT 1",
        "SELECT percent_rank() OVER (ORDER BY id) FROM tenders LIMIT 1",
        "SELECT cume_dist() OVER (ORDER BY id) FROM tenders LIMIT 1",
        "SELECT ntile(4) OVER (ORDER BY id) FROM tenders LIMIT 1",
        "SELECT lag(id) OVER (ORDER BY id) FROM tenders LIMIT 1",
        "SELECT lead(id) OVER (ORDER BY id) FROM tenders LIMIT 1",
        "SELECT first_value(id) OVER (ORDER BY id) FROM tenders LIMIT 1",
        "SELECT last_value(id) OVER (ORDER BY id) FROM tenders LIMIT 1",
        "SELECT nth_value(id, 1) OVER (ORDER BY id) FROM tenders LIMIT 1",
        "SELECT get_byte(x'0102', 0)",
        "SELECT set_byte(x'0102', 0, 9)",
        "SELECT subtype(1)",
    ] {
        let resp = server.sql(query).await;
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        assert_ne!(
            status, 200,
            "ENGINE CANARY (issue 457): {query:?} now runs -- the engine widened the /v1/sql \
             surface. Decided: allow; move it into the dialect canary and update the schema note."
        );
        assert!(!body.contains("issue 457"), "refused by the engine, not by classify: {query:?} → {body}");
    }
}

/// Issue 426: two classes of function let a single `/v1/sql` query grow the engine's
/// memory without bound and ABORT the whole server (measured: 2.7 GiB from
/// `json_group_array` over one text column inside the 10 s deadline, 6.3 GiB from a
/// single row through nested `replace`/`hex`/`zeroblob`). The deadline bounds time,
/// not bytes; these structures never spill. So `classify` refuses them up front. This
/// pins the refusal end to end, including when the call hides in a subquery, a CASE
/// arm, or an aggregate's argument.
#[tokio::test(flavor = "multi_thread")]
async fn memory_amplifying_functions_are_refused() {
    let server = Server::start("mem-amplifiers").await;
    server.ingest_chain().await;

    for query in [
        // Accumulating aggregates — one unbounded value, no spill.
        "SELECT group_concat(title) FROM v_tenders",
        "SELECT json_group_array(title) FROM v_tenders",
        "SELECT json_group_object(id, title) FROM v_tenders",
        "SELECT string_agg(title, ',') FROM v_tenders",
        // Byte-amplifying scalars, including the composed forms the measurement used.
        "SELECT hex(zeroblob(1000000000))",
        "SELECT replace(hex(zeroblob(1000000)), '0', hex(zeroblob(1000)))",
        "SELECT randomblob(1000000000)",
        "SELECT repeat('x', 1000000000)",
        "SELECT printf('%.*c', 1000000000, 'x')",
        "SELECT quote(zeroblob(1000000000))",
        // Hidden: nested in a subquery, a CASE arm, and inside a count() argument.
        "SELECT (SELECT group_concat(title) FROM v_tenders)",
        "SELECT CASE WHEN 1 THEN hex(zeroblob(1000000000)) END",
        "SELECT count(*) FROM v_tenders WHERE title = replace(title, 'a', hex(zeroblob(1000000)))",
    ] {
        let resp = server.sql(query).await;
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        assert_eq!(status, 400, "must refuse the memory amplifier: {query:?} (body {body})");
        assert!(body.contains("not allowed here"), "the 400 must name the reason: {query:?} → {body}");
    }

    // The bounded aggregates and scalar json builders the docs teach stay allowed —
    // this is a refusal of specific unbounded functions, not of aggregation.
    for ok in [
        "SELECT count(*), max(title), min(id) FROM v_tenders",
        "SELECT source, count(*) FROM v_tenders GROUP BY source",
        "SELECT json_object('id', id, 'title', title) FROM v_tenders LIMIT 5",
        "SELECT length(title) FROM v_tenders LIMIT 5",
    ] {
        assert_eq!(server.sql(ok).await.status(), 200, "must still allow: {ok:?}");
    }
}
