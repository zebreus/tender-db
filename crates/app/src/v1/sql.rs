//! `POST /v1/sql` — account holders run read-only SQL against the public
//! schema, plus `GET /v1/sql/schema` describing what they may query.
//!
//! Turso has no read-only open flag and no authorizer hook, and its
//! `PRAGMA query_only` is escapable from the same connection
//! (docs/research/turso-capabilities.md §2), so the wall is *layered* and the
//! statement allow-list is the primary one:
//!
//! 1. **Parse + classify** with `turso_parser` (turso's own parser, pinned to
//!    the engine version): accept exactly one statement, and only a bare
//!    `SELECT`. This one check rejects writes, `PRAGMA`, `ATTACH`, `VACUUM`,
//!    multi-statement bodies, and CTE-wrapped writes (`WITH … INSERT` parses as
//!    `Stmt::Insert`, never `Stmt::Select`).
//! 2. **Account-table deny**, complete by construction: a table can only be
//!    read by naming it, so we tokenise and reject any query whose *identifier*
//!    tokens include `users`, `api_tokens` or `sessions`. Working on the token
//!    stream (not the text) means a string literal like `'%sessions%'` does not
//!    false-trip it. Everything else in the database is public business data
//!    (CONTEXT.md), so there is no positive table allow-list to maintain.
//! 3. **`query_only=1` connection** from a pool dedicated to this endpoint, so
//!    a long analytical scan never starves the REST readers.
//! 4. **Timeout-by-drop** (10 s): the query runs under `tokio::time::timeout`,
//!    and turso yields to tokio at every row boundary, so a long *streaming*
//!    query is dropped between rows and its connection freed (verified, §2).
//!    Caveat measured here: a single non-yielding aggregate (`SELECT count(*)
//!    FROM generate_series(1, huge)`) computes inside one poll and cannot be
//!    interrupted — turso exposes no `interrupt()`. Issue 17 contains the blast
//!    radius: execution runs on an **isolated runtime** (see
//!    [`spawn_sql_runtime`]),
//!    so such a query can pin at most that runtime's threads and never the main
//!    API/SSE runtime; the per-token limits then bound it further. `query_only`
//!    guarantees no write can be left half-done to poison the connection either
//!    way.
//! 5. **Result caps** while reading: 10 000 rows / 10 MB, then `truncated:true`.
//! 6. **Per-token limits**: 2 concurrent (semaphore) + 300/h (governor).
//!
//! The body is capped at 64 KB and the SQL travels in it, never in the URL, so
//! queries stay out of access logs.

use crate::v1::{ApiError, AppState};
use axum::extract::{DefaultBodyLimit, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use governor::clock::{Clock, DefaultClock};
use governor::{DefaultKeyedRateLimiter, Quota, RateLimiter};
use serde_json::{Value as Json_, json};
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Semaphore;

/// Dedicated `query_only` reader connections for this endpoint. Separate from
/// the REST pool so a 10 s query here cannot queue behind the live API.
pub const SQL_READERS: usize = 4;

/// Longest a single query may run before it is dropped.
const TIMEOUT: Duration = Duration::from_secs(10);

/// Result caps, applied while streaming rows out of the engine.
const MAX_ROWS: usize = 10_000;
const MAX_BYTES: usize = 10 * 1024 * 1024;

/// Request body cap: SQL only, so generous-but-bounded.
const MAX_BODY: usize = 64 * 1024;

/// Per-token quotas (CONTEXT.md's opening posture).
const MAX_CONCURRENT: usize = 2;
const PER_HOUR: u32 = 300;

/// Tables that hold credentials or account-private data, never queryable via
/// the public SQL endpoint. Compared lowercased. `webhook_endpoints` stores
/// per-user signing secrets + private URLs and `webhook_delivery_log` their
/// cross-account history; `job_queue`/`job_log` carry operator job params —
/// none is "public business data". The durable fix is a positive allow-list
/// (issue 45): a deny-list silently re-opens the moment a new private table is
/// added, which is exactly how the secrets were exposed (issue 43).
const FORBIDDEN: [&str; 7] = [
    "users",
    "api_tokens",
    "sessions",
    "webhook_endpoints",
    "webhook_delivery_log",
    "job_queue",
    "job_log",
];

/// Worker threads on the isolated SQL runtime (issue 17). Query execution runs
/// here, never on the main API/SSE/dashboard runtime, so a non-yielding
/// aggregate (which no timeout can interrupt — turso has no `interrupt()`) can
/// pin at most this many threads and never starves the rest of the server. The
/// per-token concurrency cap and this thread count together bound SQL CPU.
const SQL_RUNTIME_THREADS: usize = 2;

/// A tokio runtime dedicated to SQL query execution, owned by a parked thread so
/// it lives for the whole process and is never dropped in an async context
/// (which would panic).
///
/// One is created per [`SqlState`] — that is once per server, so production runs
/// exactly one. It is deliberately *not* a process-global singleton: a server
/// owns its own isolation, which is what lets each server in the test suite run
/// on independent threads instead of contending for one shared pool.
fn spawn_sql_runtime() -> tokio::runtime::Handle {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("sql-runtime".into())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(SQL_RUNTIME_THREADS)
                .thread_name("sql-exec")
                .enable_all()
                .build()
                .expect("build the isolated SQL runtime");
            tx.send(runtime.handle().clone()).expect("hand back the runtime handle");
            // Park the owner thread on a future that never completes, so the
            // runtime stays alive without this thread busy-waiting.
            runtime.block_on(std::future::pending::<()>());
        })
        .expect("spawn the SQL runtime thread");
    rx.recv().expect("receive the SQL runtime handle")
}

/// The endpoint's own state: a dedicated connection pool and the two per-user
/// limiters. Lives inside [`AppState`].
pub struct SqlState {
    readers: Arc<store::Readers>,
    /// 300/h per user id.
    rate: DefaultKeyedRateLimiter<i64>,
    /// 2 concurrent per user id — one semaphore per user, created on first use.
    concurrency: Mutex<HashMap<i64, Arc<Semaphore>>>,
    /// The isolated runtime queries execute on (issue 17).
    runtime: tokio::runtime::Handle,
}

impl SqlState {
    pub fn new(readers: Arc<store::Readers>) -> SqlState {
        let quota = Quota::per_hour(NonZeroU32::new(PER_HOUR).expect("PER_HOUR is non-zero"));
        SqlState {
            readers,
            rate: RateLimiter::keyed(quota),
            concurrency: Mutex::new(HashMap::new()),
            runtime: spawn_sql_runtime(),
        }
    }

    /// The per-user concurrency gate, shared across a user's in-flight queries.
    fn semaphore(&self, user_id: i64) -> Arc<Semaphore> {
        self.concurrency
            .lock()
            .expect("sql concurrency lock")
            .entry(user_id)
            .or_insert_with(|| Arc::new(Semaphore::new(MAX_CONCURRENT)))
            .clone()
    }
}

/// Register `/v1/sql` and `/v1/sql/schema` on the API router. Kept a one-liner
/// at the call site so the endpoint's surface lives entirely in this file.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/v1/sql", post(run).layer(DefaultBodyLimit::max(MAX_BODY)))
        .route("/v1/sql/schema", get(schema))
}

// ------------------------------------------------------------------ handler

async fn run(
    State(state): State<AppState>,
    user: crate::v1::AuthUser,
    sql: String,
) -> Result<Response, ApiError> {
    let sql_state = &state.sql;

    // Rate first — a rejected caller should not even take a connection slot.
    if let Err(not_until) = sql_state.rate.check_key(&user.id()) {
        let retry = not_until.wait_time_from(DefaultClock::default().now()).as_secs().max(1);
        return Ok(too_many("hourly query limit reached", retry));
    }

    // Then concurrency: try, never queue — a third live query is refused, not
    // stalled behind the other two.
    let permit = match sql_state.semaphore(user.id()).try_acquire_owned() {
        Ok(permit) => permit,
        Err(_) => return Ok(too_many("too many concurrent queries (max 2)", 1)),
    };

    classify(&sql)?;

    // Run the query on the isolated SQL runtime (issue 17), not this one. The
    // reader is borrowed and the timeout applied *there*, so even a query that
    // pins its worker thread cannot touch the main API/SSE runtime. This handler
    // only awaits the result over a cheap channel — it never blocks a main
    // worker. The permit is held (on this side) until the result comes back.
    let readers = sql_state.readers.clone();
    let outcome = sql_state
        .runtime
        .spawn(async move {
            let reader = readers.get().await.map_err(Executed::Db)?;
            match tokio::time::timeout(TIMEOUT, execute(&reader, &sql)).await {
                Ok(Ok(result)) => Ok(result),
                Ok(Err(e)) => Err(Executed::Db(e)),
                Err(_) => Err(Executed::Timeout),
            }
        })
        .await;
    drop(permit);

    match outcome {
        Ok(Ok(result)) => Ok(Json(json!({
            "columns": result.columns,
            "rows": result.rows,
            "row_count": result.rows.len(),
            "truncated": result.truncated,
        }))
        .into_response()),
        // A turso execution error is the user's SQL being wrong (unknown column,
        // type error, dialect gap) — a 400 with the engine's message.
        Ok(Err(Executed::Db(e))) => Err(ApiError(StatusCode::BAD_REQUEST, e.to_string())),
        // A streaming query past the limit was dropped between rows (§4).
        Ok(Err(Executed::Timeout)) => Err(ApiError(
            StatusCode::REQUEST_TIMEOUT,
            format!("query exceeded the {}s time limit", TIMEOUT.as_secs()),
        )),
        // The isolated task panicked or was cancelled — our fault, not the
        // caller's.
        Err(e) => Err(ApiError(StatusCode::INTERNAL_SERVER_ERROR, format!("sql runtime: {e}"))),
    }
}

/// How a query finished on the isolated runtime — a DB error is the caller's,
/// a timeout is the watchdog's.
enum Executed {
    Db(store::turso::Error),
    Timeout,
}

/// The queryable schema: every table and view except the credential tables,
/// with its columns — so a client can discover the surface without guessing.
async fn schema(State(state): State<AppState>) -> Result<Response, ApiError> {
    let reader = state.sql.readers.get().await?;
    let mut objects = Vec::new();
    let mut rows = reader
        .query(
            "SELECT name, type FROM sqlite_schema
              WHERE type IN ('table','view') AND name NOT LIKE 'sqlite_%'
              ORDER BY type DESC, name",
            (),
        )
        .await?;
    while let Some(row) = rows.next().await? {
        let name = store_text(&row, 0);
        let kind = store_text(&row, 1);
        if FORBIDDEN.contains(&name.to_ascii_lowercase().as_str()) || !safe_identifier(&name) {
            continue;
        }
        objects.push((name, kind));
    }
    drop(rows);

    let mut tables = Vec::with_capacity(objects.len());
    for (name, kind) in objects {
        let mut cols = Vec::new();
        // `name` came from sqlite_schema and passed `safe_identifier`, so the
        // interpolation into this PRAGMA is safe (PRAGMA takes no bind params).
        let mut info = reader.query(&format!("PRAGMA table_info(\"{name}\")"), ()).await?;
        while let Some(row) = info.next().await? {
            cols.push(json!({
                "name": store_text(&row, 1),
                "type": store_text(&row, 2),
                "notnull": store_int(&row, 3) != 0,
                "pk": store_int(&row, 5) != 0,
            }));
        }
        tables.push(json!({ "name": name, "type": kind, "columns": cols }));
    }

    Ok(Json(json!({
        "tables": tables,
        "notes": [
            "Read-only: only a single SELECT is accepted.",
            "The users, api_tokens and sessions tables are not queryable.",
            "Turso SQL dialect gaps: no WITH RECURSIVE; window functions are \
             partial (row_number and aggregate OVER work; rank/lead/lag and \
             custom frames do not).",
            format!("Results are capped at {MAX_ROWS} rows / {}MB; a capped \
                     response carries \"truncated\": true.", MAX_BYTES / 1024 / 1024),
            format!("Limits: {MAX_CONCURRENT} concurrent queries and {PER_HOUR} \
                     queries per hour per token; each query may run {}s.", TIMEOUT.as_secs()),
        ],
    }))
    .into_response())
}

// --------------------------------------------------------------- allow-list

/// Parse the body and accept it only if it is exactly one bare `SELECT` that
/// names no credential table. Every rejection is a 400 with a reason.
fn classify(sql: &str) -> Result<(), ApiError> {
    use turso_parser::ast::{Cmd, Stmt};
    use turso_parser::parser::Parser;

    let mut parser = Parser::new(sql.as_bytes());
    let first = parser
        .next()
        .ok_or_else(|| bad("empty query"))?
        .map_err(|e| bad(format!("could not parse SQL: {e}")))?;

    match first {
        Cmd::Stmt(Stmt::Select(_)) => {}
        Cmd::Stmt(_) => return Err(bad("only SELECT statements are allowed")),
        Cmd::Explain(_) | Cmd::ExplainQueryPlan(_) => {
            return Err(bad("EXPLAIN is not allowed; send the SELECT itself"));
        }
    }

    // A second statement means a multi-statement body — reject the whole thing.
    if parser
        .next()
        .transpose()
        .map_err(|e| bad(format!("could not parse SQL: {e}")))?
        .is_some()
    {
        return Err(bad("only a single statement is allowed"));
    }

    if let Some(name) = forbidden_identifier(sql) {
        return Err(bad(format!("the {name} table is not queryable")));
    }
    Ok(())
}

/// The first credential-table name that appears as an *identifier* token, if
/// any. Scanning identifier tokens (not raw text) means a value like
/// `WHERE title LIKE '%sessions%'` — a string literal — is not mistaken for a
/// table reference.
fn forbidden_identifier(sql: &str) -> Option<&'static str> {
    use turso_parser::lexer::Lexer;
    use turso_parser::token::TokenType;

    for token in Lexer::new(sql.as_bytes()).flatten() {
        if token.token_type == TokenType::TK_ID {
            let ident = String::from_utf8_lossy(token.value)
                .trim_matches(|c| c == '"' || c == '`' || c == '[' || c == ']')
                .to_ascii_lowercase();
            if let Some(hit) = FORBIDDEN.into_iter().find(|f| *f == ident) {
                return Some(hit);
            }
        }
    }
    None
}

// ---------------------------------------------------------------- execution

struct QueryResult {
    columns: Vec<String>,
    rows: Vec<Json_>,
    truncated: bool,
}

/// Run the (already-classified) SELECT, applying `query_only` defensively and
/// stopping at the row/byte caps.
async fn execute(conn: &store::turso::Connection, sql: &str) -> store::turso::Result<QueryResult> {
    // Set defensively every time: the pool is dedicated, but this keeps the
    // guarantee local to the one place user SQL is run.
    let mut pragma = conn.query("PRAGMA query_only = 1", ()).await?;
    while pragma.next().await?.is_some() {}
    drop(pragma);

    let mut rows = conn.query(sql, ()).await?;
    let columns = rows.column_names();
    let width = columns.len();

    let mut out = Vec::new();
    let mut bytes = 0usize;
    let mut truncated = false;
    while let Some(row) = rows.next().await? {
        // Cooperative yield between rows. Turso resolves CPU-bound and cached
        // work synchronously — its futures return Ready without ever pending —
        // so without this an expensive query would neither let the 10 s
        // `timeout` fire nor release its worker thread. Yielding at each row
        // boundary gives both a chance. (A single non-streaming aggregate has no
        // row boundary and still cannot be interrupted; that is the residual
        // limit the module docs call out.)
        tokio::task::yield_now().await;
        if out.len() >= MAX_ROWS || bytes >= MAX_BYTES {
            // We already pulled one more row than fits — that is the evidence
            // there was more, so the result is genuinely truncated.
            truncated = true;
            break;
        }
        let mut cells = Vec::with_capacity(width);
        for i in 0..width {
            let (value, size) = cell(row.get_value(i)?);
            bytes += size;
            cells.push(value);
        }
        out.push(Json_::Array(cells));
    }
    // Dropping `rows` here (or on the truncation break) ends the statement; the
    // connection returns to the pool immediately reusable.
    Ok(QueryResult { columns, rows: out, truncated })
}

/// One cell as JSON, plus a rough serialized-byte estimate for the size cap.
fn cell(value: store::turso::Value) -> (Json_, usize) {
    use store::turso::Value;
    match value {
        Value::Null => (Json_::Null, 4),
        Value::Integer(i) => (json!(i), 20),
        Value::Real(f) => (
            serde_json::Number::from_f64(f).map_or(Json_::Null, Json_::Number),
            24,
        ),
        Value::Text(s) => {
            let size = s.len() + 2;
            (Json_::String(s), size)
        }
        // The public schema has no BLOB columns; if one ever appears, hex keeps
        // the response valid JSON.
        Value::Blob(b) => {
            let hex: String = b.iter().map(|x| format!("{x:02x}")).collect();
            let size = hex.len() + 2;
            (Json_::String(hex), size)
        }
    }
}

// ------------------------------------------------------------------ helpers

fn bad(message: impl Into<String>) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, message.into())
}

/// A 429 with a `Retry-After`, built directly because it carries a header the
/// shared `ApiError` shape does not.
fn too_many(message: &str, retry_after: u64) -> Response {
    (
        StatusCode::TOO_MANY_REQUESTS,
        [(header::RETRY_AFTER, retry_after.to_string())],
        Json(json!({ "error": { "status": 429, "message": message } })),
    )
        .into_response()
}

/// Whether a name is a plain identifier safe to interpolate into a PRAGMA.
fn safe_identifier(name: &str) -> bool {
    !name.is_empty()
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        && !name.as_bytes()[0].is_ascii_digit()
}

fn store_text(row: &store::turso::Row, idx: usize) -> String {
    match row.get_value(idx) {
        Ok(store::turso::Value::Text(s)) => s,
        _ => String::new(),
    }
}

fn store_int(row: &store::turso::Row, idx: usize) -> i64 {
    match row.get_value(idx) {
        Ok(store::turso::Value::Integer(i)) => i,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_plain_select() {
        assert!(classify("SELECT 1").is_ok());
        assert!(classify("SELECT id, title FROM v_tenders WHERE id > 0 LIMIT 5").is_ok());
        assert!(classify("  select * from v_lots  ").is_ok());
        // A trailing semicolon is one statement, not two.
        assert!(classify("SELECT 1;").is_ok());
        // Plain CTEs are fine.
        assert!(classify("WITH x AS (SELECT 1 AS n) SELECT n FROM x").is_ok());
        // Subqueries against public tables are fine.
        assert!(classify("SELECT * FROM v_tenders WHERE id IN (SELECT tender_id FROM v_lots)").is_ok());
    }

    #[test]
    fn rejects_everything_that_is_not_one_select() {
        for sql in [
            "",
            "INSERT INTO v_tenders VALUES (1)",
            "UPDATE tenders SET title = 'x'",
            "DELETE FROM tenders",
            "DROP TABLE tenders",
            "CREATE TABLE t (a)",
            "PRAGMA query_only = 0",
            "ATTACH DATABASE 'x.db' AS x",
            "VACUUM",
            "SELECT 1; SELECT 2",
            "SELECT 1; DROP TABLE tenders",
            // CTE-wrapped write parses as an INSERT, not a SELECT.
            "WITH x AS (SELECT 1) INSERT INTO tenders SELECT * FROM x",
            "EXPLAIN SELECT 1",
            "EXPLAIN QUERY PLAN SELECT 1",
            "not sql at all ~~~",
        ] {
            assert!(classify(sql).is_err(), "should have rejected: {sql:?}");
        }
    }

    #[test]
    fn rejects_reads_of_credential_tables() {
        for sql in [
            "SELECT * FROM users",
            "SELECT password_hash FROM users",
            "SELECT * FROM api_tokens",
            "SELECT token_hash FROM api_tokens",
            "SELECT * FROM sessions",
            "SELECT * FROM main.users",
            "SELECT u.username FROM users u",
            "SELECT * FROM v_tenders WHERE id IN (SELECT user_id FROM sessions)",
            "SELECT * FROM \"users\"",
        ] {
            assert!(classify(sql).is_err(), "should have denied: {sql:?}");
        }
        // A string literal that merely contains a forbidden word is fine — it
        // is not an identifier token.
        assert!(classify("SELECT * FROM v_tenders WHERE title LIKE '%sessions%'").is_ok());
        assert!(classify("SELECT 'users' AS label").is_ok());
    }

    #[test]
    fn identifies_which_table_was_denied() {
        assert_eq!(forbidden_identifier("SELECT * FROM api_tokens"), Some("api_tokens"));
        assert_eq!(forbidden_identifier("SELECT * FROM v_tenders"), None);
    }

    #[test]
    fn account_private_tables_are_denied() {
        // Regression (issue 43): the deny-list must cover every private table,
        // not just the auth trio. `webhook_endpoints.secret` is a per-user
        // signing key — reachable here would let any account forge signed
        // webhooks for every other account.
        for sql in [
            "SELECT user_id, url, secret FROM webhook_endpoints",
            "SELECT * FROM webhook_delivery_log",
            "SELECT * FROM job_queue",
            "SELECT * FROM job_log",
        ] {
            assert!(classify(sql).is_err(), "must deny private table: {sql:?}");
        }
        assert_eq!(
            forbidden_identifier("SELECT secret FROM webhook_endpoints"),
            Some("webhook_endpoints"),
        );
    }

    #[test]
    fn cells_map_to_json() {
        use store::turso::Value;
        assert_eq!(cell(Value::Null).0, Json_::Null);
        assert_eq!(cell(Value::Integer(42)).0, json!(42));
        assert_eq!(cell(Value::Text("hi".into())).0, json!("hi"));
        // NaN cannot be JSON, so it degrades to null rather than erroring.
        assert_eq!(cell(Value::Real(f64::NAN)).0, Json_::Null);
        assert_eq!(cell(Value::Blob(vec![0xde, 0xad])).0, json!("dead"));
    }

    #[test]
    fn safe_identifier_guards_pragma_interpolation() {
        assert!(safe_identifier("v_tenders"));
        assert!(safe_identifier("notice_texts"));
        assert!(!safe_identifier("no-dashes"));
        assert!(!safe_identifier("1leading"));
        assert!(!safe_identifier(""));
        assert!(!safe_identifier("a\"; DROP"));
    }
}
