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
//! 2. **Table allow-list**, denied by default: a table can only be read by
//!    naming it in a table position, so we walk the parsed statement's table
//!    references (FROM/JOIN entries, `x IN table`, and every nested subquery)
//!    and reject the query unless *every* base table it reads is on the
//!    explicit public-surface allow-list ([`ALLOWED`]). Walking the AST — not
//!    the raw text — means a string literal like `'%sessions%'` or a column
//!    named `users` never false-trips, and a table-valued function call
//!    (`generate_series(...)`) is a function, not a base-table read. This is a
//!    positive allow-list on purpose (issue 45): a deny-list silently re-opened
//!    the moment a new private table was added, which is exactly how webhook
//!    signing secrets were exposed (issue 43). A private table added later is
//!    denied by default because it is simply not in [`ALLOWED`].
//! 3. **`query_only=1` connection** from a pool dedicated to this endpoint, so
//!    a long analytical scan never starves the REST readers.
//! 4. **Timeout, one layer that works** (10 s). The in-task
//!    `tokio::time::timeout` on the isolated runtime **does not bound anything**,
//!    and this comment used to say otherwise. It claimed turso yields at every row
//!    boundary, so a long *streaming* query is dropped between rows and its
//!    connection freed. **Measured 2026-08-03 on the deployed turso 0.7.0: it is
//!    not dropped — cold or warm.** A 0.5 s budget over a 3.5 s streaming read
//!    never fired, on a pass verified to reach disk (1.26 M filesystem inputs
//!    against zero warm); a 50 ms budget over a 0.95 s 4 M-row read never fired
//!    either. Likely mechanism, offered as hypothesis: `Statement::step` returns
//!    `Poll::Pending` only on `TursoStatusCode::Io`, and a synchronous VFS blocks
//!    *inside* the poll rather than returning `Io`, so there is no await point for
//!    the timer and cache state changes only how long the poll takes. The claim may
//!    have held on another engine version or VFS; it does not hold here. The
//!    non-yielding aggregate (`SELECT count(*) FROM generate_series(1, huge)`) was
//!    always known to slip past it — it used to run past 40 s with no 408 (issue
//!    51) — and it turns out the streaming case is no different.
//!    The handler adds a **backstop timeout on the main runtime**: it fires
//!    on time regardless (the heavy work is isolated), returns 408 at the cap,
//!    and drops the permit so the concurrency slot frees at once — held for the
//!    cap, disconnected or not, never 40 s. The aggregate itself still runs to
//!    completion on the isolated runtime (abandoned via [`AbortOnDrop`], which
//!    can only cancel at an await point), but issue 17's isolation means it pins
//!    at most that runtime's threads, never the main API/SSE runtime, and the
//!    per-token limits bound it further. `query_only` guarantees no write can be
//!    left half-done to poison the connection either way.
//!
//!    **So the endpoint's protection against an expensive query is the backstop
//!    plus the isolation, and nothing else.** The backstop bounds the RESPONSE and
//!    frees the concurrency slot; issue 17's isolation bounds the BLAST RADIUS. The
//!    work itself is never stopped — turso exposes no `interrupt()`, and no timeout
//!    at any layer can take its place. Nothing here should be designed as if a
//!    per-query time cap can halt turso work; it cannot.
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

/// Longest a single query may run before it is dropped (a streaming query,
/// between rows) or the handler stops waiting and answers 408 (a non-yielding
/// aggregate). The value the server runs; [`SqlState::with_timeout`] lets a test
/// watch the cap fire without a 10 s query.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// Extra margin the handler-side backstop waits beyond the in-task timeout, so a
/// streaming query always reports through the in-task path (a clean drop between
/// rows) and the backstop only ever fires for a non-yielding aggregate.
const TIMEOUT_GRACE: Duration = Duration::from_secs(1);

/// Result caps, applied while streaming rows out of the engine.
const MAX_ROWS: usize = 10_000;
const MAX_BYTES: usize = 10 * 1024 * 1024;

/// Request body cap: SQL only, so generous-but-bounded.
const MAX_BODY: usize = 64 * 1024;

/// Per-token quotas (CONTEXT.md's opening posture).
const MAX_CONCURRENT: usize = 2;
const PER_HOUR: u32 = 300;

/// The public-surface allow-list: the only tables and views `/v1/sql` may read.
/// Compared lowercased. Denied by default — anything not enumerated here (a new
/// private table, an internal `__turso_*` table, a future credential store) is
/// simply not queryable, which is the durable fix for issue 43's deny-list that
/// re-opened whenever a private table was added. Each entry is public business
/// data (CONTEXT.md); the account/webhook/operator tables and the raw-fetch
/// registry are deliberately absent (see the note below the list).
const ALLOWED: [&str; 45] = [
    // Current-state views — the analyst entry points (docs/architecture.md).
    "v_tenders",         // current version of each Tender
    "v_lots",            // current Lots
    "v_organizations",   // canonical Organizations with mention counts
    "v_lot_results",     // current award decisions with their winner
    "v_tender_current",  // the (tender_id, seq) current-version pointer
    "notice_withheld_fields", // fields a notice marked withheld — public metadata
    // Analyst convenience views (issue 50) — the common questions, one view away.
    "v_tender_buyers",         // buyers of each current Tender
    "v_awards",                // current award decisions with winner + buyer
    "v_tender_classifications", // current CPV/NUTS codes
    "v_tender_amounts",        // current money amounts
    "v_tender_dates",          // current dates (epoch seconds + offset)
    "v_tender_notices",        // the notices that caused each Tender version
    "v_fetches",               // path-free fetch provenance (issue 45)
    // Canonical current tables (ADR-0001): the projected Tender/Lot/Org layer.
    "tenders",
    "tender_versions",
    "lots",
    "organizations",
    "organization_mentions",
    "lot_results",
    "bids",
    "contracts",
    // Canonical version satellites — one Tender version's facts, all public.
    "tender_version_texts",
    "tender_version_amounts",
    "tender_version_dates",
    "tender_version_classifications",
    "tender_version_lots",
    "tender_version_parties",
    "tender_version_bids",
    "tender_version_bid_parties",
    "tender_version_contracts",
    "tender_version_lot_results",
    "tender_version_result_winners",
    "tender_version_result_stats",
    // Notice-parsed layer — the relational reading of each raw notice payload.
    "notices",
    "notice_sections",
    "notice_texts",
    "notice_amounts",
    "notice_dates",
    "notice_classifications",
    "notice_codes",
    "notice_ids",
    "notice_integers",
    "notice_numbers",
    // Raw-ingest quality + the public change feed.
    "quarantine",        // whole raw notices that failed to map — public payloads
    "changes",           // the change cursor log, served verbatim by /v1/changes
    // Deliberately NOT allowed:
    //  * users, api_tokens, sessions — credentials.
    //  * webhook_endpoints, webhook_delivery_log — per-user signing secrets +
    //    private URLs + cross-account delivery history (issue 43).
    //  * job_queue, job_log — operator job params.
    //  * fetches — the raw-fetch registry's `path` column is server filesystem
    //    layout (ingestion/operator infra), not business data. Provenance (which
    //    package/period a notice came from) is legitimately public and will be
    //    surfaced through a path-free `v_fetches` view, deferred to the store lane
    //    with issue 50's analyst views.
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
    /// The per-query time limit — [`DEFAULT_TIMEOUT`] in production.
    timeout: Duration,
}

impl SqlState {
    pub fn new(readers: Arc<store::Readers>) -> SqlState {
        SqlState::with_timeout(readers, DEFAULT_TIMEOUT)
    }

    /// As [`new`](SqlState::new), with an explicit per-query time limit — the
    /// seam a test uses to observe the 408 cap without running a 10 s query.
    pub fn with_timeout(readers: Arc<store::Readers>, timeout: Duration) -> SqlState {
        let quota = Quota::per_hour(NonZeroU32::new(PER_HOUR).expect("PER_HOUR is non-zero"));
        SqlState {
            readers,
            rate: RateLimiter::keyed(quota),
            concurrency: Mutex::new(HashMap::new()),
            runtime: spawn_sql_runtime(),
            timeout,
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
    // reader is borrowed and the in-task timeout applied *there*, so even a query
    // that pins its worker thread cannot touch the main API/SSE runtime.
    let readers = sql_state.readers.clone();
    let timeout = sql_state.timeout;
    let handle = sql_state.runtime.spawn(async move {
        let reader = readers.get().await.map_err(Executed::Db)?;
        // The in-task timeout drops a *streaming* query between rows, freeing its
        // reader promptly. A non-yielding aggregate computes in one poll and
        // slips past it — the handler-side backstop below is what bounds that.
        match tokio::time::timeout(timeout, execute(&reader, &sql)).await {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(e)) => Err(Executed::Db(e)),
            Err(_) => Err(Executed::Timeout),
        }
    });
    // Abandon the isolated task whenever this handler stops waiting — the backstop
    // firing, or the request future being dropped when the client disconnects —
    // so a query never outlives the request that asked for it (issue 51). On a
    // normal finish the abort is a no-op.
    let _abandon = AbortOnDrop(handle.abort_handle());

    // The backstop runs here, on the main runtime, so it fires on time even while
    // a non-yielding aggregate pins an isolated worker (a COUNT/GROUP-BY that used
    // to run past 40 s with no 408). It bounds how long the concurrency slot is
    // held — disconnected or not — to the cap: when it or a disconnect ends this
    // await, `permit` drops and the slot frees at once (issue 51).
    let outcome = tokio::time::timeout(timeout + TIMEOUT_GRACE, handle).await;
    drop(permit);

    match outcome {
        Ok(Ok(Ok(result))) => Ok(Json(json!({
            "columns": result.columns,
            "rows": result.rows,
            "row_count": result.rows.len(),
            "truncated": result.truncated,
        }))
        .into_response()),
        // A turso execution error is the user's SQL being wrong (unknown column,
        // type error, dialect gap) — a 400 with the engine's message.
        Ok(Ok(Err(Executed::Db(e)))) => Err(ApiError(StatusCode::BAD_REQUEST, e.to_string())),
        // A streaming query dropped between rows by the in-task timeout, or the
        // backstop firing on a non-yielding aggregate — both are the query
        // exceeding the time limit, and both are a 408.
        Ok(Ok(Err(Executed::Timeout))) | Err(_) => Err(timed_out(timeout)),
        // The task was aborted because we stopped waiting (backstop/disconnect):
        // report the same 408 rather than a spurious 500.
        Ok(Err(join)) if join.is_cancelled() => Err(timed_out(timeout)),
        // A genuine panic on the isolated runtime — our fault, not the caller's.
        Ok(Err(join)) => {
            Err(ApiError(StatusCode::INTERNAL_SERVER_ERROR, format!("sql runtime: {join}")))
        }
    }
}

/// The 408 a query past its time limit gets.
fn timed_out(timeout: Duration) -> ApiError {
    ApiError(
        StatusCode::REQUEST_TIMEOUT,
        format!("query exceeded the {}s time limit", timeout.as_secs()),
    )
}

/// Aborts the wrapped task on drop, so a query is abandoned the moment the
/// handler stops waiting for it — the backstop firing or the client
/// disconnecting — instead of running on and holding a reader (issue 51).
struct AbortOnDrop(tokio::task::AbortHandle);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// How a query finished on the isolated runtime — a DB error is the caller's,
/// a timeout is the watchdog's.
enum Executed {
    Db(store::turso::Error),
    Timeout,
}

/// Time columns are Unix epoch seconds in SQL while REST returns ISO — the trap
/// issue 50 calls out. Reused across every timestamp column.
const EPOCH_NOTE: &str = "Unix epoch seconds — NOT ISO (the REST API returns \
    ISO). Filter/format with strftime(col,'unixepoch'), e.g. \
    strftime(published_at,'unixepoch') LIKE '2012%'.";

/// One-line descriptions for the tables/views worth explaining in
/// `/v1/sql/schema` (issue 50); the rest are self-describing.
const TABLE_NOTES: &[(&str, &str)] = &[
    ("v_tenders", "Current version of each Tender — the usual entry point (one row per Tender)."),
    ("v_lots", "Current Lots — subdivisions of a Tender."),
    ("v_organizations", "Canonical Organizations (buyers, bidders, winners) with a mention count."),
    ("v_lot_results", "Current award decisions: one row per (result, winning organization); \
      winner_* is NULL for an unresolved or withheld award."),
    ("v_tender_current", "The (tender_id, seq) current-version pointer — join it to read any \
      version satellite at current state cheaply."),
    ("notices", "One row per raw publication event. The parsed payload is in the notice_* \
      tables; the canonical layer is projected from it (ADR-0001)."),
    ("quarantine", "Whole notices whose content could not be mapped — public raw payloads \
      kept for reprocessing."),
    ("changes", "The change-cursor log behind /v1/changes: ingestion order, never renumbered."),
    ("tender_version_parties", "Organizations linked to a Tender version by role (see role)."),
    ("tender_version_classifications", "CPV and NUTS codes of a Tender version (see scheme)."),
    // Analyst convenience views (issue 50).
    ("v_tender_buyers", "Buyers of each current Tender (one row per buyer party)."),
    ("v_awards", "Current award decisions with their winner and a representative buyer — \
      keeps v_lot_results' one-row-per-winner grain (does not multiply by buyer count)."),
    ("v_tender_classifications", "CPV and NUTS codes of each current Tender (see scheme)."),
    ("v_tender_amounts", "Money amounts of each current Tender (field, cents, currency)."),
    ("v_tender_dates", "Dates of each current Tender (utc_seconds epoch + offset_minutes)."),
    ("v_tender_notices", "The notices that caused each Tender version — the ADR-0001 chain, \
      across all versions."),
    ("v_fetches", "Path-free fetch provenance: which source package/period a notice came from."),
];

/// Column notes and small enum vocabularies. Table `"*"` matches a column of
/// that name in any table (the epoch columns recur widely). Open or
/// era-dependent vocabularies are described rather than exhaustively listed.
const COLUMN_NOTES: &[(&str, &str, &str)] = &[
    // The epoch-seconds columns — the time-format trap.
    ("*", "published_at", EPOCH_NOTE),
    ("*", "dispatched_at", EPOCH_NOTE),
    ("*", "ingested_at", EPOCH_NOTE),
    ("*", "fetched_at", EPOCH_NOTE),
    ("*", "changed_at", EPOCH_NOTE),
    ("*", "first_seen", EPOCH_NOTE),
    ("*", "reprocessed_at", EPOCH_NOTE),
    ("*", "utc_seconds", EPOCH_NOTE),
    // Enum / coded columns.
    ("notices", "parse_state", "One of: pending, parsed, quarantined (ADR-0004)."),
    ("tender_version_classifications", "scheme", "One of: cpv, nuts."),
    (
        "tender_version_parties",
        "role",
        "Buyer roles appear as 'buyer' or 'Procedure-Buyer' (era-dependent — match with \
         role LIKE '%uyer%'); results-layer roles are 'winner', 'tenderer', 'subcontractor'.",
    ),
    (
        "*",
        "decision",
        "eForms winner-selection-status code, e.g. 'selec-w' (a winner was selected), \
         'clos-nw' (closed, no award).",
    ),
    (
        "*",
        "notice_subtype",
        "eForms notice subtype id, e.g. '16' (contract notice), '29' (contract award).",
    ),
    (
        "*",
        "provisional",
        "1 = a single-mention profile with no official identifier, never merged (CONTEXT.md).",
    ),
];

/// The description for a table/view, if one is curated.
fn table_note(name: &str) -> Option<&'static str> {
    TABLE_NOTES.iter().find(|(t, _)| *t == name).map(|&(_, note)| note)
}

/// The note for a column — an exact `(table, column)` match wins over a `"*"`
/// (any-table) one, so a table can override the generic vocabulary.
fn column_note(table: &str, column: &str) -> Option<&'static str> {
    let matches = |t: &str| t == table || t == "*";
    COLUMN_NOTES
        .iter()
        .find(|(t, c, _)| *t == table && *c == column)
        .or_else(|| COLUMN_NOTES.iter().find(|(t, c, _)| matches(t) && *c == column))
        .map(|&(_, _, note)| note)
}

/// The queryable schema: every allow-listed table and view with its columns,
/// per-table/column notes and enum vocabularies — so a client can discover the
/// surface without guessing (issue 50).
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
        if !ALLOWED.contains(&name.to_ascii_lowercase().as_str()) || !safe_identifier(&name) {
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
            let col_name = store_text(&row, 1);
            let mut col = json!({
                "name": col_name,
                "type": store_text(&row, 2),
                "notnull": store_int(&row, 3) != 0,
                "pk": store_int(&row, 5) != 0,
            });
            if let Some(note) = column_note(&name, &col_name) {
                col.as_object_mut().expect("column is an object").insert("note".into(), json!(note));
            }
            cols.push(col);
        }
        let mut table = json!({ "name": name, "type": kind, "columns": cols });
        if let Some(note) = table_note(&name) {
            table.as_object_mut().expect("table is an object").insert("note".into(), json!(note));
        }
        tables.push(table);
    }

    Ok(Json(json!({
        "tables": tables,
        "notes": [
            "Read-only: only a single SELECT is accepted.",
            "Queryable surface is a positive allow-list: only the tables and \
             views listed above are readable. Account, webhook and operator \
             tables (users, api_tokens, sessions, webhook_endpoints, job_queue, …) \
             and the raw-fetch registry (fetches, which holds server filesystem \
             paths) are not queryable.",
            "Time columns are Unix epoch seconds, NOT ISO — the REST API returns \
             ISO, so the two disagree. Filter/format with strftime(col,'unixepoch'); \
             each timestamp column's note flags this. WHERE published_at LIKE \
             '2012%' silently matches nothing.",
            "Backfill in progress: the canonical v_* layer currently reflects only \
             PROJECTED tenders (2026 forward, until the historical backfill is \
             projected), so a v_* query scoped to earlier years may return nothing \
             yet. The notice_* and quarantine layers already hold the full \
             imported history.",
            "Turso SQL dialect gaps: no WITH RECURSIVE; window functions are \
             partial (row_number and aggregate OVER work; rank/lead/lag and \
             custom frames do not).",
            format!("Results are capped at {MAX_ROWS} rows / {}MB; a capped \
                     response carries \"truncated\": true.", MAX_BYTES / 1024 / 1024),
            format!("Limits: {MAX_CONCURRENT} concurrent queries and {PER_HOUR} \
                     queries per hour per token; each query may run {}s (a query \
                     past the cap — including a slow aggregate — is 408).",
                    state.sql.timeout.as_secs()),
        ],
        "examples": [
            "SELECT source, count(*) FROM v_tenders GROUP BY source",
            "SELECT strftime(published_at,'unixepoch','start of year') AS year, \
             count(*) AS tenders FROM v_tenders GROUP BY year ORDER BY year",
            "SELECT o.name, count(*) AS lots_won FROM v_lot_results r \
             JOIN v_organizations o ON o.id = r.winner_organization_id \
             GROUP BY o.id ORDER BY lots_won DESC LIMIT 10",
        ],
    }))
    .into_response())
}

// --------------------------------------------------------------- allow-list

/// Parse the body and accept it only if it is exactly one bare `SELECT` whose
/// every base-table reference is on the public-surface allow-list. Every
/// rejection is a 400 with a reason.
fn classify(sql: &str) -> Result<(), ApiError> {
    use turso_parser::ast::{Cmd, Stmt};
    use turso_parser::parser::Parser;

    let mut parser = Parser::new(sql.as_bytes());
    let first = parser
        .next()
        .ok_or_else(|| bad("empty query"))?
        .map_err(|e| bad(format!("could not parse SQL: {e}")))?;

    let select = match first {
        Cmd::Stmt(Stmt::Select(select)) => select,
        Cmd::Stmt(_) => return Err(bad("only SELECT statements are allowed")),
        Cmd::Explain(_) | Cmd::ExplainQueryPlan(_) => {
            return Err(bad("EXPLAIN is not allowed; send the SELECT itself"));
        }
    };

    // A second statement means a multi-statement body — reject the whole thing.
    if parser
        .next()
        .transpose()
        .map_err(|e| bad(format!("could not parse SQL: {e}")))?
        .is_some()
    {
        return Err(bad("only a single statement is allowed"));
    }

    if let Some(name) = disallowed_table(&select) {
        return Err(bad(format!(
            "the {name} table is not in the queryable public surface"
        )));
    }
    Ok(())
}

/// Table-valued functions allowed as a `FROM` source. Deny-by-default, the same
/// posture as [`ALLOWED`]: a TVF is a callable that materialises rows, and some
/// of them (`pragma_table_info('users')`, `pragma_table_xinfo(…)`) reach a
/// table's schema WITHOUT a base-table reference — so they slip past the
/// allow-list walk and disclose the columns of the credential tables that are
/// deliberately absent from [`ALLOWED`] (issue 204). Only functions that read no
/// schema object are listed. `generate_series` is the one analysts actually use
/// (date spines, gap-filling); everything else — every `pragma_*`, any future
/// TVF — is denied until proven safe and added here.
const ALLOWED_TVF: [&str; 1] = ["generate_series"];

/// The base tables an executed SELECT would read, each tagged with whether a CTE
/// of that name was visible where it appeared, plus the table-valued functions.
///
/// A base-table reference covered by a *visible* CTE resolves to that derived
/// query, not a base table, so it is not checked against [`ALLOWED`]. Coverage is
/// decided at the reference's own lexical position (see [`Scope`]), never from a
/// global name set: CTE visibility is scoped, and a CTE buried in an inner
/// subquery must not launder an outer reference to a credential table (issue 210).
#[derive(Default)]
struct Tables {
    /// Base-table references (FROM/JOIN entries and `x IN table`), lowercased,
    /// each paired with `covered` — whether a CTE of that name was in scope at
    /// the reference. A `covered` ref is a derived-query read, not a base table.
    refs: Vec<(String, bool)>,
    /// Table-valued function names used as a FROM source, lowercased — checked
    /// against [`ALLOWED_TVF`]. A CTE cannot define a callable, so a TVF name can
    /// never resolve to one, and scope does not enter into it.
    tvfs: Vec<String>,
}

/// The CTE names visible at one point in the walk — a stack of `WITH` frames,
/// innermost first, held as a cons-list so a descent borrows its parent instead
/// of cloning the whole stack.
///
/// This is the heart of the issue-210 fix. SQL binds a `FROM x` to a CTE only
/// when a CTE named `x` is lexically in scope there; otherwise `x` is the base
/// table. Resolving each reference against the CTE names visible *from its own
/// position* is what stops an inner-scope CTE from whitelisting an outer read of
/// a table deliberately absent from [`ALLOWED`] (`SELECT * FROM api_tokens WHERE
/// 1 = (WITH api_tokens AS (SELECT 1) SELECT 1)` — the outer `api_tokens` cannot
/// see the subquery's CTE, so it is the credential table and must be denied).
struct Scope<'a> {
    names: &'a std::collections::HashSet<String>,
    parent: Option<&'a Scope<'a>>,
}

impl Scope<'_> {
    /// Is `name` bound to a CTE visible here? Walks outward through the enclosing
    /// `WITH` frames.
    fn covers(&self, name: &str) -> bool {
        self.names.contains(name) || self.parent.is_some_and(|p| p.covers(name))
    }
}

/// The first referenced table or table-valued function the query is not allowed
/// to read, if any — the reason it is denied. A base table must be covered by a
/// visible CTE or be in [`ALLOWED`]; a TVF must be in [`ALLOWED_TVF`].
fn disallowed_table(select: &turso_parser::ast::Select) -> Option<String> {
    let mut tables = Tables::default();
    let empty = std::collections::HashSet::new();
    walk_select(select, &Scope { names: &empty, parent: None }, &mut tables);
    tables
        .refs
        .iter()
        .find(|(name, covered)| !covered && !ALLOWED.contains(&name.as_str()))
        .map(|(name, _)| name.clone())
        .or_else(|| tables.tvfs.iter().find(|name| !ALLOWED_TVF.contains(&name.as_str())).cloned())
}

fn norm(name: &turso_parser::ast::Name) -> String {
    name.as_str().to_ascii_lowercase()
}

/// Walk a SELECT, resolving CTE scope. Each CTE body sees the enclosing scope
/// plus the siblings declared *before* it (plus itself, when the `WITH` is
/// `RECURSIVE`); the primary query, its compound arms, ORDER BY and LIMIT see the
/// enclosing scope plus *all* siblings — exactly SQL's ordered CTE visibility.
/// Anything this under-approximates (mutual recursion, forward references) is
/// rejected, which is the safe direction for an allow-list.
fn walk_select(s: &turso_parser::ast::Select, scope: &Scope, t: &mut Tables) {
    let Some(with) = &s.with else {
        walk_select_parts(s, scope, t);
        return;
    };
    let mut siblings: std::collections::HashSet<String> = std::collections::HashSet::new();
    for cte in &with.ctes {
        let name = norm(&cte.tbl_name);
        // The body sees earlier siblings, and itself only if the WITH is RECURSIVE.
        let mut visible = siblings.clone();
        if with.recursive {
            visible.insert(name.clone());
        }
        walk_select(&cte.select, &Scope { names: &visible, parent: Some(scope) }, t);
        siblings.insert(name);
    }
    // The primary query and its tails see every sibling.
    walk_select_parts(s, &Scope { names: &siblings, parent: Some(scope) }, t);
}

/// The parts of a SELECT other than its `WITH`: the body, its compound arms, and
/// the ORDER BY / LIMIT expressions — all walked under `scope`.
fn walk_select_parts(s: &turso_parser::ast::Select, scope: &Scope, t: &mut Tables) {
    walk_one_select(&s.body.select, scope, t);
    for compound in &s.body.compounds {
        walk_one_select(&compound.select, scope, t);
    }
    for sc in &s.order_by {
        walk_expr(&sc.expr, scope, t);
    }
    if let Some(limit) = &s.limit {
        walk_expr(&limit.expr, scope, t);
        if let Some(offset) = &limit.offset {
            walk_expr(offset, scope, t);
        }
    }
}

fn walk_one_select(o: &turso_parser::ast::OneSelect, scope: &Scope, t: &mut Tables) {
    use turso_parser::ast::{OneSelect, ResultColumn};
    match o {
        OneSelect::Select { columns, from, where_clause, group_by, window_clause, .. } => {
            for col in columns {
                match col {
                    ResultColumn::Expr(e, _) => walk_expr(e, scope, t),
                    ResultColumn::Star | ResultColumn::TableStar(_) => {}
                }
            }
            if let Some(from) = from {
                walk_from(from, scope, t);
            }
            if let Some(w) = where_clause {
                walk_expr(w, scope, t);
            }
            if let Some(group) = group_by {
                for e in &group.exprs {
                    walk_expr(e, scope, t);
                }
                if let Some(having) = &group.having {
                    walk_expr(having, scope, t);
                }
            }
            for def in window_clause {
                walk_window(&def.window, scope, t);
            }
        }
        OneSelect::Values(rows) => {
            for row in rows {
                for e in row {
                    walk_expr(e, scope, t);
                }
            }
        }
    }
}

fn walk_from(f: &turso_parser::ast::FromClause, scope: &Scope, t: &mut Tables) {
    use turso_parser::ast::JoinConstraint;
    walk_table(&f.select, scope, t);
    for join in &f.joins {
        walk_table(&join.table, scope, t);
        if let Some(JoinConstraint::On(e)) = &join.constraint {
            walk_expr(e, scope, t);
        }
        // `USING (col, …)` names columns only, never a table.
    }
}

fn walk_table(st: &turso_parser::ast::SelectTable, scope: &Scope, t: &mut Tables) {
    use turso_parser::ast::SelectTable;
    match st {
        // A bare table name — the one place a base table is read. Tag it with
        // whether a CTE of that name is visible *here*: if so it resolves to the
        // CTE, not the base table (issue 210).
        SelectTable::Table(name, _, _) => {
            let name = norm(&name.name);
            let covered = scope.covers(&name);
            t.refs.push((name, covered));
        }
        // A table-valued function (`generate_series(…)`, `pragma_table_info(…)`):
        // a callable FROM source. Its name IS checked (against ALLOWED_TVF, not
        // ALLOWED) — a `pragma_*` TVF reaches a denied table's schema without a
        // base-table reference, so leaving it unchecked disclosed the credential
        // tables' columns (issue 204). Arguments are still walked for hidden
        // subqueries.
        SelectTable::TableCall(name, args, _) => {
            t.tvfs.push(norm(&name.name));
            for a in args {
                walk_expr(a, scope, t);
            }
        }
        SelectTable::Select(s, _) => walk_select(s, scope, t),
        SelectTable::Sub(f, _) => walk_from(f, scope, t),
    }
}

fn walk_window(w: &turso_parser::ast::Window, scope: &Scope, t: &mut Tables) {
    use turso_parser::ast::FrameBound;
    for e in &w.partition_by {
        walk_expr(e, scope, t);
    }
    for sc in &w.order_by {
        walk_expr(&sc.expr, scope, t);
    }
    if let Some(frame) = &w.frame_clause {
        for bound in [Some(&frame.start), frame.end.as_ref()].into_iter().flatten() {
            match bound {
                FrameBound::Following(e) | FrameBound::Preceding(e) => walk_expr(e, scope, t),
                FrameBound::CurrentRow
                | FrameBound::UnboundedFollowing
                | FrameBound::UnboundedPreceding => {}
            }
        }
    }
}

fn walk_function_tail(ft: &turso_parser::ast::FunctionTail, scope: &Scope, t: &mut Tables) {
    use turso_parser::ast::Over;
    if let Some(e) = &ft.filter_clause {
        walk_expr(e, scope, t);
    }
    match &ft.over_clause {
        Some(Over::Window(w)) => walk_window(w, scope, t),
        Some(Over::Name(_)) | None => {}
    }
}

/// Walk one expression for the base tables its subqueries read. The match is
/// exhaustive with no wildcard on purpose: a `turso_parser` upgrade that adds an
/// `Expr` variant carrying a table reference then fails to compile here rather
/// than silently opening a hole (issue 45's whole point).
fn walk_expr(e: &turso_parser::ast::Expr, scope: &Scope, t: &mut Tables) {
    use turso_parser::ast::Expr::*;
    match e {
        Between { lhs, start, end, .. } => {
            walk_expr(lhs, scope, t);
            walk_expr(start, scope, t);
            walk_expr(end, scope, t);
        }
        Binary(a, _, b) => {
            walk_expr(a, scope, t);
            walk_expr(b, scope, t);
        }
        Case { base, when_then_pairs, else_expr } => {
            if let Some(b) = base {
                walk_expr(b, scope, t);
            }
            for (when, then) in when_then_pairs {
                walk_expr(when, scope, t);
                walk_expr(then, scope, t);
            }
            if let Some(el) = else_expr {
                walk_expr(el, scope, t);
            }
        }
        Cast { expr, .. } => walk_expr(expr, scope, t),
        Collate(x, _) => walk_expr(x, scope, t),
        Exists(s) => walk_select(s, scope, t),
        FieldAccess { base, .. } => walk_expr(base, scope, t),
        FunctionCall { args, order_by, within_group, filter_over, .. } => {
            for a in args {
                walk_expr(a, scope, t);
            }
            for sc in order_by.iter().chain(within_group) {
                walk_expr(&sc.expr, scope, t);
            }
            walk_function_tail(filter_over, scope, t);
        }
        FunctionCallStar { filter_over, .. } => walk_function_tail(filter_over, scope, t),
        InList { lhs, rhs, .. } => {
            walk_expr(lhs, scope, t);
            for e in rhs {
                walk_expr(e, scope, t);
            }
        }
        InSelect { lhs, rhs, .. } => {
            walk_expr(lhs, scope, t);
            walk_select(rhs, scope, t);
        }
        InTable { lhs, rhs, args, .. } => {
            walk_expr(lhs, scope, t);
            if args.is_empty() {
                // `x IN some_table` reads some_table's first column — tagged with
                // CTE coverage at this position exactly like a FROM ref (issue 210).
                let name = norm(&rhs.name);
                let covered = scope.covers(&name);
                t.refs.push((name, covered));
            } else {
                // `x IN tvf(args)` — a table-valued function; its name is checked
                // against ALLOWED_TVF exactly as in `walk_table` (a pragma TVF
                // here would leak a denied table's schema just the same, issue
                // 204).
                t.tvfs.push(norm(&rhs.name));
                for a in args {
                    walk_expr(a, scope, t);
                }
            }
        }
        IsNull(x) | NotNull(x) => walk_expr(x, scope, t),
        Like { lhs, rhs, escape, .. } => {
            walk_expr(lhs, scope, t);
            walk_expr(rhs, scope, t);
            if let Some(e) = escape {
                walk_expr(e, scope, t);
            }
        }
        Parenthesized(xs) => {
            for x in xs {
                walk_expr(x, scope, t);
            }
        }
        Raise(_, x) => {
            if let Some(x) = x {
                walk_expr(x, scope, t);
            }
        }
        Subquery(s) => walk_select(s, scope, t),
        Unary(_, x) => walk_expr(x, scope, t),
        Subscript { base, index } => {
            walk_expr(base, scope, t);
            walk_expr(index, scope, t);
        }
        Array { elements } => {
            for e in elements {
                walk_expr(e, scope, t);
            }
        }
        SubqueryResult { lhs, .. } => {
            if let Some(x) = lhs {
                walk_expr(x, scope, t);
            }
        }
        // Leaves: identifiers, literals, columns and parameters — no nested
        // expression and no base-table reference.
        Register(_) | DoublyQualified(..) | Id(_) | Column { .. } | RowId { .. }
        | Literal(_) | Name(_) | Qualified(..) | Variable(_) | Default => {}
    }
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

    /// The base table `classify` would reject, for a bare SELECT — the test-only
    /// window onto the allow-list walk.
    fn denied_table(sql: &str) -> Option<String> {
        use turso_parser::ast::{Cmd, Stmt};
        use turso_parser::parser::Parser;
        match Parser::new(sql.as_bytes()).next().unwrap().unwrap() {
            Cmd::Stmt(Stmt::Select(select)) => disallowed_table(&select),
            other => panic!("not a bare SELECT: {other:?}"),
        }
    }

    #[test]
    fn identifies_which_table_was_denied() {
        assert_eq!(denied_table("SELECT * FROM api_tokens").as_deref(), Some("api_tokens"));
        assert_eq!(denied_table("SELECT * FROM v_tenders"), None);
    }

    #[test]
    fn account_private_tables_are_denied() {
        // Regression (issue 43): every private table stays unreachable.
        // `webhook_endpoints.secret` is a per-user signing key — reachable here
        // would let any account forge signed webhooks for every other account.
        for sql in [
            "SELECT user_id, url, secret FROM webhook_endpoints",
            "SELECT * FROM webhook_delivery_log",
            "SELECT * FROM job_queue",
            "SELECT * FROM job_log",
            "SELECT * FROM users",
            "SELECT * FROM api_tokens",
            "SELECT * FROM sessions",
        ] {
            assert!(classify(sql).is_err(), "must deny private table: {sql:?}");
        }
        assert_eq!(
            denied_table("SELECT secret FROM webhook_endpoints").as_deref(),
            Some("webhook_endpoints"),
        );
    }

    #[test]
    fn a_new_private_table_is_denied_by_default() {
        // The durable property (issue 45): a table nobody added to ALLOWED — a
        // future credential store, an internal `__turso_*` table — is denied
        // without touching this gate. If this test ever fails, a non-public
        // table has become reachable.
        for sql in [
            "SELECT * FROM password_resets",
            "SELECT * FROM billing_accounts",
            // fetches is a real table, deliberately excluded: its `path` column is
            // server filesystem layout, not public business data (issue 45).
            "SELECT * FROM fetches",
            "SELECT * FROM __turso_internal_seq_notices",
            // Hidden in a subquery, a comma-join, an IN-table and a CTE body —
            // every table position the walk must reach.
            "SELECT * FROM v_tenders WHERE id IN (SELECT id FROM password_resets)",
            "SELECT * FROM v_tenders, password_resets",
            "SELECT 1 WHERE 1 IN password_resets",
            "WITH x AS (SELECT * FROM password_resets) SELECT * FROM x",
            "SELECT * FROM v_tenders ORDER BY (SELECT max(id) FROM password_resets)",
        ] {
            assert!(classify(sql).is_err(), "a non-allowlisted table must be denied: {sql:?}");
        }
    }

    #[test]
    fn cte_scope_does_not_launder_a_private_table_read() {
        // Issue 210: a CTE only covers a base-table reference where the CTE is
        // lexically in scope. A same-named CTE in an inner or later scope must NOT
        // whitelist a reference that turso resolves to the real credential table.
        for sql in [
            // The confirmed exploit: the outer `api_tokens` is an ANCESTOR of the
            // subquery's `WITH api_tokens`, cannot see it, and is the base table.
            "SELECT * FROM api_tokens WHERE 1 = (WITH api_tokens AS (SELECT 1) SELECT 1)",
            "SELECT COUNT(id) FROM job_queue WHERE 1 = (WITH job_queue AS (SELECT 1) SELECT 1)",
            // A LATER sibling cannot be seen by an earlier CTE's body, so the body's
            // `api_tokens` is the base table.
            "WITH t AS (SELECT * FROM api_tokens), api_tokens AS (SELECT 1) SELECT * FROM t",
            // The shadow CTE sits in a sibling subquery in the FROM, not enclosing
            // the outer reference.
            "SELECT * FROM users, (WITH users AS (SELECT 1) SELECT 1) AS shadow",
            // Shadow defined only in an ORDER BY subquery.
            "SELECT * FROM sessions ORDER BY (WITH sessions AS (SELECT 1) SELECT 1)",
        ] {
            assert!(classify(sql).is_err(), "CTE scope must not launder: {sql:?}");
        }
        // The escape names the real table it resolves to.
        assert_eq!(
            denied_table("SELECT * FROM api_tokens WHERE 1 = (WITH api_tokens AS (SELECT 1) SELECT 1)")
                .as_deref(),
            Some("api_tokens"),
        );

        // The legitimate CTE shapes the fix must keep working.
        for sql in [
            // Same-scope: the reference is in the WITH's own primary query.
            "WITH x AS (SELECT 1 AS n) SELECT n FROM x",
            // Earlier sibling: `b`'s body may see `a`.
            "WITH a AS (SELECT id FROM v_tenders), b AS (SELECT id FROM a) SELECT id FROM b",
            // An enclosing CTE is visible inside a nested WITH's body and primary.
            "WITH a AS (SELECT id FROM v_lots) \
             SELECT id FROM a WHERE id IN (WITH b AS (SELECT id FROM a) SELECT id FROM b)",
            // Recursive self-reference is legal and reads no base table.
            "WITH RECURSIVE c(n) AS (SELECT 1 UNION ALL SELECT n + 1 FROM c WHERE n < 5) SELECT n FROM c",
            // A CTE named after a credential table is fine when it fully shadows it
            // in every position the name is used (turso reads the CTE, not the table).
            "WITH api_tokens AS (SELECT 1 AS n) SELECT n FROM api_tokens",
        ] {
            assert!(classify(sql).is_ok(), "a legitimate CTE query must classify OK: {sql:?}");
        }
    }

    #[test]
    fn every_allowlisted_table_is_queryable() {
        // The positive half: each advertised public table/view classifies OK,
        // so the allow-list never accidentally denies its own surface.
        for name in ALLOWED {
            let sql = format!("SELECT * FROM {name}");
            assert!(classify(&sql).is_ok(), "allow-listed table must be queryable: {name}");
        }
    }

    #[test]
    fn table_valued_functions_are_deny_by_default() {
        // The allow-listed TVF works, including a private table hidden in its
        // arguments still being caught.
        assert!(classify("SELECT * FROM generate_series(1, 5)").is_ok());
        assert!(classify("SELECT value FROM generate_series(1, 5) WHERE value > 2").is_ok());
        assert!(
            classify("SELECT * FROM generate_series(1, (SELECT count(*) FROM sessions))").is_err()
        );
    }

    #[test]
    fn pragma_table_valued_functions_cannot_disclose_a_denied_table(
    ) {
        // Issue 204: `pragma_table_info('users')` is a table-valued function that
        // returns the SCHEMA of its argument — column names (`password_hash`) and
        // default values — WITHOUT a base-table reference, so before the TVF
        // allow-list it slipped past the walk and disclosed the credential
        // tables' shape. Every pragma TVF, in every FROM/IN position, is denied.
        for sql in [
            "SELECT * FROM pragma_table_info('users')",
            "SELECT * FROM pragma_table_xinfo('api_tokens')",
            "SELECT name FROM pragma_table_list",
            "SELECT dflt_value FROM pragma_table_info('sessions')",
            // Laundered through a CTE, a subquery, an IN-table, a join.
            "WITH x AS (SELECT * FROM pragma_table_info('users')) SELECT * FROM x",
            "SELECT * FROM v_tenders WHERE id IN (SELECT cid FROM pragma_table_info('users'))",
            "SELECT * FROM v_tenders WHERE 1 IN pragma_table_info('users')",
            "SELECT * FROM v_tenders, pragma_table_info('users')",
            "SELECT (SELECT count(*) FROM pragma_table_info('users'))",
            // A future/unknown TVF is denied too — deny by default.
            "SELECT * FROM some_new_tvf('users')",
        ] {
            assert!(classify(sql).is_err(), "pragma/unknown TVF must be denied: {sql:?}");
        }
        // The denied name is reported, so the 400 says which TVF was refused.
        assert_eq!(
            denied_table("SELECT * FROM pragma_table_info('users')").as_deref(),
            Some("pragma_table_info"),
        );
    }

    #[test]
    fn denied_tables_stay_denied_through_every_reference_shape() {
        // Issue 204 pass 2: the walk must reach a credential table in every
        // position an attacker can put one — a compound (UNION/EXCEPT/INTERSECT)
        // arm, a case-varied name, a schema qualifier, or a system catalog under
        // any schema. Each of these was probed live and held; this pins it.
        for sql in [
            // Set-op arms — the compound walk must visit every branch.
            "SELECT id FROM v_tenders UNION SELECT id FROM users",
            "SELECT id FROM v_tenders EXCEPT SELECT id FROM sessions",
            "SELECT id FROM v_tenders INTERSECT SELECT rowid FROM api_tokens",
            // Case is folded before the allow-list check.
            "SELECT * FROM USERS",
            "SELECT * FROM UsErS",
            // A schema qualifier does not launder the object name.
            "SELECT * FROM main.users",
            "SELECT * FROM temp.sqlite_master",
            // The SQLite system catalogs, bare and temp-qualified, are not public.
            "SELECT * FROM sqlite_master",
            "SELECT * FROM sqlite_temp_master",
            "SELECT * FROM sqlite_temp_schema",
            // json_each/json_tree are table-valued functions — deny-by-default
            // now (not in ALLOWED_TVF), so a subquery hidden in one cannot even
            // reach execution.
            "SELECT * FROM json_each('[1]')",
            "SELECT value FROM json_each((SELECT password_hash FROM users))",
        ] {
            assert!(classify(sql).is_err(), "must stay denied: {sql:?}");
        }
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
