//! The public API: `/v1` REST + the change feed, as a plain axum sub-router
//! merged beside the dioxus application (docs/research/api-layer.md §1).
//!
//! Server functions stay the dashboard's private RPC; everything an external
//! client touches lives here, where we own the URL space, the error contract
//! and the middleware.

pub mod auth;
pub mod docs;
pub mod health;
pub mod json;
pub mod sql;
pub mod sse;
pub mod webhooks;

pub use auth::AuthUser;

use axum::Router;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use store::read::{self, Filter, Scope, Status};
use tokio::sync::watch;
use tower_governor::key_extractor::KeyExtractor;
use tower_governor::{GovernorError, GovernorLayer, governor::GovernorConfigBuilder};

/// The git revision the running server is deployed as. Read at RUNTIME from the
/// `COMMIT_SHA` environment variable — `deploy.sh` sets it per deploy from the
/// rev it already records in `/opt/tender-db/deployed-rev`, so the nix build
/// stays reproducible (it never bakes a rev into the artifact). Falls back to a
/// compile-time `COMMIT_SHA` if one was set, then to `dev` for a plain build.
pub fn rev() -> &'static str {
    static REV: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    REV.get_or_init(|| {
        std::env::var("COMMIT_SHA")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| option_env!("COMMIT_SHA").map(str::to_owned))
            .unwrap_or_else(|| "dev".to_owned())
    })
    .as_str()
}

/// AGPL §13: a network user must be offered the running version's source. The
/// `/_source` route answers with this revision and how to obtain it.
const SOURCE_OFFER: &str = "https://tenders.zebreus.click/_source";

/// Anonymous SSE is capped per client (CONTEXT.md: ~5 streams/IP).
const MAX_STREAMS_PER_CLIENT: usize = 5;

/// Opening rate posture, per CONTEXT.md: generous, ~10 rps sustained with room
/// for a burst of paging.
const RATE_PER_SECOND: u64 = 10;
const RATE_BURST: u32 = 50;

/// Default and maximum page sizes.
const DEFAULT_LIMIT: i64 = 100;

#[derive(Clone)]
pub struct AppState {
    /// The writer handle — reads go through `readers`; this is here for the
    /// account lookups token authentication does, which also *write* (the
    /// `last_used_at` touch).
    pub db: Arc<store::Db>,
    pub readers: Arc<store::Readers>,
    /// The writer's change-cursor doorbell — SSE's only wake-up signal.
    pub cursor: watch::Receiver<i64>,
    /// The read-only SQL endpoint's dedicated pool and per-token limiters.
    pub sql: Arc<sql::SqlState>,
    streams: Arc<Mutex<HashMap<String, usize>>>,
}

impl AppState {
    pub fn new(db: Arc<store::Db>, readers: Arc<store::Readers>) -> AppState {
        let cursor = db.cursor_watch();
        // A pool of readers dedicated to `/v1/sql`, kept apart from the REST
        // pool so a slow analytical query cannot starve the live API.
        let sql = Arc::new(sql::SqlState::new(db.readers(sql::SQL_READERS).expect("sql reader pool")));
        AppState { db, readers, cursor, sql, streams: Arc::new(Mutex::new(HashMap::new())) }
    }
}

/// The API sub-router. Rate limiting covers `/v1` only: `/health` must answer
/// for the deploy script under any load, and `/_source` is a licence
/// obligation, not a service.
pub fn router(state: AppState) -> Router {
    let limits = GovernorConfigBuilder::default()
        .per_second(RATE_PER_SECOND)
        .burst_size(RATE_BURST)
        .key_extractor(ClientKey)
        .finish()
        .expect("a positive rate and burst always build a config");

    Router::new()
        .route("/v1", get(root))
        .route("/v1/tenders", get(tenders))
        .route("/v1/tenders/{id}", get(tender))
        .route("/v1/lots", get(lots))
        .route("/v1/organizations", get(organizations))
        .route("/v1/organizations/{id}", get(organization))
        .route("/v1/notices", get(notices))
        .route("/v1/notices/{id}", get(notice))
        .route("/v1/changes", get(changes))
        .route("/v1/me", get(me))
        .merge(sql::routes())
        .merge(webhooks::routes())
        .layer(GovernorLayer::new(limits))
        .route("/health", get(health))
        // The deep operational probe an external pinger watches — liveness plus
        // ingest freshness, job failures and disk. Outside the rate limiter, like
        // `/health`: a pinger must never be throttled (issue 24).
        .route("/health/deep", get(health::deep))
        .route("/_source", get(source))
        // The human-readable API reference. Outside the rate limiter (like
        // `/_source`): reading the docs is not a service call and must not spend
        // a caller's API budget.
        .route("/docs", get(docs::page))
        .with_state(state)
}

// --------------------------------------------------------------- client keys

/// Who a request counts against for rate limiting and the SSE cap.
///
/// `dioxus::server::serve` calls `into_make_service()` — no `ConnectInfo` — so
/// the peer address is simply not available to us, and production runs behind
/// nginx anyway, where the peer address would be the proxy's for every request.
/// The forwarded headers are therefore the key, with one shared bucket for
/// direct (unproxied) callers.
pub fn client_key(headers: &HeaderMap) -> String {
    // nginx sets `X-Real-IP $remote_addr` — the true peer, a single trusted
    // value — so prefer it. `X-Forwarded-For` is `$proxy_add_x_forwarded_for`:
    // any client-supplied value is preserved and the real peer APPENDED, so the
    // only trustworthy entry is the LAST one. Keying on the leftmost (issue 44)
    // let a caller spoof `X-Forwarded-For: <random>` per request and evade the
    // rate limiter and SSE per-IP cap — the sole DoS controls on the
    // unauthenticated surface.
    if let Some(ip) = headers.get("x-real-ip").and_then(|v| v.to_str().ok())
        && !ip.trim().is_empty()
    {
        return ip.trim().to_owned();
    }
    if let Some(last) = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|value| value.split(',').next_back())
        && !last.trim().is_empty()
    {
        return last.trim().to_owned();
    }
    "direct".to_owned()
}

#[cfg(test)]
mod client_key_tests {
    use super::client_key;
    use axum::http::HeaderMap;

    #[test]
    fn prefers_trusted_real_ip_over_spoofable_forwarded_for() {
        // X-Real-IP is set by our proxy to the true peer; XFF's leftmost is
        // client-controlled. The key must be the trusted peer, never the spoof.
        let mut h = HeaderMap::new();
        h.insert("x-real-ip", "203.0.113.7".parse().unwrap());
        h.insert("x-forwarded-for", "1.2.3.4, 203.0.113.7".parse().unwrap());
        assert_eq!(client_key(&h), "203.0.113.7");
    }

    #[test]
    fn without_real_ip_uses_rightmost_forwarded_for_not_the_client_value() {
        // Only XFF present: the appended (rightmost) entry is our proxy's view
        // of the peer; the leftmost is attacker-supplied and must be ignored.
        let mut h = HeaderMap::new();
        h.insert("x-forwarded-for", "1.2.3.4, 203.0.113.7".parse().unwrap());
        assert_eq!(client_key(&h), "203.0.113.7");
    }

    #[test]
    fn falls_back_to_a_shared_bucket_when_unproxied() {
        assert_eq!(client_key(&HeaderMap::new()), "direct");
    }
}

#[derive(Clone, Copy)]
struct ClientKey;

impl KeyExtractor for ClientKey {
    type Key = String;

    fn extract<T>(&self, req: &axum::http::Request<T>) -> Result<String, GovernorError> {
        Ok(client_key(req.headers()))
    }
}

// ------------------------------------------------------------------- errors

/// The API's error contract — one shape, ours, for every failure.
pub struct ApiError(StatusCode, String);

impl ApiError {
    fn not_found(what: &str) -> ApiError {
        ApiError(StatusCode::NOT_FOUND, format!("no such {what}"))
    }

    fn bad_request(message: impl Into<String>) -> ApiError {
        ApiError(StatusCode::BAD_REQUEST, message.into())
    }
}

impl From<store::turso::Error> for ApiError {
    fn from(e: store::turso::Error) -> ApiError {
        ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = json!({ "error": { "status": self.0.as_u16(), "message": self.1 } });
        (self.0, axum::Json(body)).into_response()
    }
}

type ApiResult = Result<Response, ApiError>;

// ------------------------------------------------------------------- params

/// Every query parameter the API understands, in one struct: the filters are
/// shared across collections by design (docs/architecture.md — a subscription
/// *is* a collection query plus its filters).
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Params {
    source: Option<String>,
    country: Option<String>,
    cpv: Option<String>,
    buyer: Option<i64>,
    /// Tenders where this Organization won at least one Lot.
    winner: Option<i64>,
    status: Option<String>,
    min_value: Option<i64>,
    max_value: Option<i64>,
    kind: Option<String>,
    tender: Option<i64>,
    /// Pagination position: the last id of the previous page.
    cursor: Option<String>,
    limit: Option<i64>,
    /// `/v1/changes` only.
    since: Option<String>,
    entity: Option<String>,
    /// SSE only: embed the entity's current state in each event.
    include_data: Option<bool>,
}

impl Params {
    fn filter(&self, now: i64) -> Result<Filter, ApiError> {
        let status = match self.status.as_deref() {
            None => None,
            Some("open") => Some(Status::Open),
            Some("closed") => Some(Status::Closed),
            Some(other) => {
                return Err(ApiError::bad_request(format!(
                    "status must be 'open' or 'closed', not {other:?}"
                )));
            }
        };
        Ok(Filter {
            source: self.source.clone(),
            country: self.country.clone(),
            cpv: self.cpv.clone(),
            buyer: self.buyer,
            winner: self.winner,
            status,
            min_value: self.min_value,
            max_value: self.max_value,
            kind: self.kind.clone(),
            tender: self.tender,
            now,
        })
    }

    /// The pagination cursor is opaque to clients but is the last row id;
    /// anything unparseable starts from the beginning rather than erroring, so
    /// a truncated cursor never strands a client.
    fn after(&self) -> i64 {
        self.cursor.as_deref().and_then(|c| c.parse().ok()).unwrap_or(0)
    }

    fn limit(&self) -> i64 {
        self.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, read::MAX_PAGE)
    }

    fn since(&self) -> i64 {
        self.since.as_deref().and_then(|c| c.parse().ok()).unwrap_or(0)
    }
}

// -------------------------------------------------------------- collections

/// The four collections the API serves. Each one is a list endpoint, an SSE
/// subscription, and — for the three that appear in the change log — a slice of
/// the change feed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Collection {
    Tenders,
    Lots,
    Organizations,
    Notices,
}

impl Collection {
    /// The `changes.entity_kind` this collection's diffs come from. Notices are
    /// the raw import layer and have no canonical change rows, so an SSE
    /// subscription to them is a snapshot and then silence.
    pub fn entity_kind(self) -> Option<&'static str> {
        match self {
            Collection::Tenders => Some("tender"),
            Collection::Lots => Some("lot"),
            Collection::Organizations => Some("organization"),
            Collection::Notices => None,
        }
    }
}

/// One row as the API returns it, with the id pagination and diffing key on.
pub struct Item {
    pub id: i64,
    pub json: Value,
}

/// Read a collection in a scope. This is the single evaluation point for the
/// filter predicates: the list endpoint, the SSE snapshot and the SSE diff
/// probe all come through here, so a filtered stream can never disagree with
/// the filtered list it started from.
pub async fn read_items(
    collection: Collection,
    conn: &store::turso::Connection,
    filter: &Filter,
    scope: Scope,
) -> store::turso::Result<Vec<Item>> {
    Ok(match collection {
        Collection::Tenders => read::tenders(conn, filter, scope)
            .await?
            .iter()
            .map(|r| Item { id: r.id, json: json::tender(r) })
            .collect(),
        Collection::Lots => read::lots(conn, filter, scope)
            .await?
            .iter()
            .map(|r| Item { id: r.id, json: json::lot(r) })
            .collect(),
        Collection::Organizations => read::organizations(conn, filter, scope)
            .await?
            .iter()
            .map(|r| Item { id: r.id, json: json::organization(r) })
            .collect(),
        Collection::Notices => read::notices(conn, filter, scope)
            .await?
            .iter()
            .map(|r| Item { id: r.id, json: json::notice(r) })
            .collect(),
    })
}

async fn collection(
    collection: Collection,
    state: AppState,
    headers: HeaderMap,
    params: Params,
) -> ApiResult {
    let filter = params.filter(store::now_unix())?;
    if wants_events(&headers) {
        return sse::subscribe(collection, state, headers, params, filter).await;
    }
    let limit = params.limit();
    let reader = state.readers.get().await?;
    // One extra row answers "is there another page?" without a second query.
    let scope = Scope::Page { after: params.after(), limit: limit + 1 };
    let mut items = read_items(collection, &reader, &filter, scope).await?;
    let next = (items.len() as i64 > limit).then(|| items[limit as usize - 1].id);
    items.truncate(limit as usize);
    Ok(axum::Json(json::page(items.into_iter().map(|i| i.json).collect(), next)).into_response())
}

fn wants_events(headers: &HeaderMap) -> bool {
    headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|accept| accept.contains("text/event-stream"))
}

// ------------------------------------------------------------------ handlers

async fn tenders(State(s): State<AppState>, h: HeaderMap, Query(p): Query<Params>) -> ApiResult {
    collection(Collection::Tenders, s, h, p).await
}

async fn lots(State(s): State<AppState>, h: HeaderMap, Query(p): Query<Params>) -> ApiResult {
    collection(Collection::Lots, s, h, p).await
}

async fn organizations(
    State(s): State<AppState>,
    h: HeaderMap,
    Query(p): Query<Params>,
) -> ApiResult {
    collection(Collection::Organizations, s, h, p).await
}

async fn notices(State(s): State<AppState>, h: HeaderMap, Query(p): Query<Params>) -> ApiResult {
    // `?tender=` lists the Notices that caused a Tender's versions — the
    // ADR-0001 chain, walkable from a detail's `caused_by_notice_id`. The store
    // has no notice→tender predicate, so this is answered in the app from the
    // tender detail rather than silently ignored (issue 49). A lookup, not a
    // subscription, so it is JSON regardless of Accept.
    if let Some(tender_id) = p.tender {
        return tender_notices(&s, tender_id).await;
    }
    collection(Collection::Notices, s, h, p).await
}

async fn tender(State(state): State<AppState>, Path(id): Path<i64>) -> ApiResult {
    let reader = state.readers.get().await?;
    match read::tender_detail(&reader, id).await? {
        Some(detail) => Ok(axum::Json(json::detail(&detail)).into_response()),
        None => Err(ApiError::not_found("tender")),
    }
}

/// `GET /v1/notices/{id}` — one Notice by id, the counterpart of the
/// `caused_by_notice_id` a tender detail hands out (issue 49).
async fn notice(State(state): State<AppState>, Path(id): Path<i64>) -> ApiResult {
    let reader = state.readers.get().await?;
    match read::notices(&reader, &Filter::default(), Scope::At { id, seq: 0 }).await?.into_iter().next()
    {
        Some(row) => Ok(axum::Json(json::notice(&row)).into_response()),
        None => Err(ApiError::not_found("notice")),
    }
}

/// `GET /v1/organizations/{id}` — one Organization by id, the counterpart of a
/// tender detail's `parties[].organization_id` (issue 49).
async fn organization(State(state): State<AppState>, Path(id): Path<i64>) -> ApiResult {
    let reader = state.readers.get().await?;
    match read::organizations(&reader, &Filter::default(), Scope::At { id, seq: 0 })
        .await?
        .into_iter()
        .next()
    {
        Some(row) => Ok(axum::Json(json::organization(&row)).into_response()),
        None => Err(ApiError::not_found("organization")),
    }
}

/// The Notices behind a Tender's version chain, in id order. `404` if the
/// Tender itself is unknown, so `?tender=` never masks a bad id as "no notices".
async fn tender_notices(state: &AppState, tender_id: i64) -> ApiResult {
    let reader = state.readers.get().await?;
    let Some(detail) = read::tender_detail(&reader, tender_id).await? else {
        return Err(ApiError::not_found("tender"));
    };
    let mut ids: Vec<i64> = detail.versions.iter().map(|v| v.caused_by_notice_id).collect();
    ids.sort_unstable();
    ids.dedup();
    let mut items = Vec::new();
    for id in ids {
        if let Some(row) =
            read::notices(&reader, &Filter::default(), Scope::At { id, seq: 0 }).await?.into_iter().next()
        {
            items.push(json::notice(&row));
        }
    }
    Ok(axum::Json(json::page(items, None)).into_response())
}

/// The poll half of the change feed. Same events, same cursor and same
/// filtering as SSE — a client that cannot hold a connection open loses
/// nothing but latency.
async fn changes(State(state): State<AppState>, Query(params): Query<Params>) -> ApiResult {
    let limit = params.limit();
    let reader = state.readers.get().await?;
    let rows =
        read::changes_since(&reader, params.since(), limit, params.entity.as_deref()).await?;
    let last = rows.last().map(|c| c.cursor).unwrap_or_else(|| params.since());
    let more = rows.len() as i64 == limit;
    let events: Vec<Value> = rows.iter().map(sse::change_event).collect();
    Ok(axum::Json(json!({
        "events": events,
        "last_cursor": json::cursor(last),
        "more": more,
    }))
    .into_response())
}

/// Who the presented API token belongs to — the token-gated endpoint every
/// client can call to check its credentials before relying on them, and the
/// probe the account round-trip test asserts against.
async fn me(AuthUser(user): AuthUser) -> ApiResult {
    Ok(axum::Json(json!({
        "user": { "id": user.id, "username": user.username, "created_at": user.created_at },
    }))
    .into_response())
}

/// The API root, which is also where AGPL §13's source offer lives.
async fn root(State(state): State<AppState>) -> ApiResult {
    let reader = state.readers.get().await?;
    Ok(axum::Json(json!({
        "service": "tender-db",
        "version": env!("CARGO_PKG_VERSION"),
        "source": rev(),
        "source_offer": SOURCE_OFFER,
        "license": "AGPL-3.0-or-later",
        "docs": "/docs",
        "cursor": json::cursor(read::latest_cursor(&reader).await?),
        "endpoints": [
            "/v1/tenders", "/v1/tenders/{id}", "/v1/lots", "/v1/organizations",
            "/v1/organizations/{id}", "/v1/notices", "/v1/notices/{id}",
            "/v1/changes", "/v1/me", "/v1/sql", "/v1/sql/schema", "/v1/webhooks",
        ],
        "live": "send Accept: text/event-stream to any collection endpoint",
    }))
    .into_response())
}

/// AGPL §13's "Corresponding Source" offer for the running version.
async fn source() -> Response {
    (
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        format!(
            "tender-db {version}, revision {rev}\n\
             Licensed AGPL-3.0-or-later; the full licence text ships with the source.\n\n\
             This service offers the Corresponding Source of the exact version it is\n\
             running, as AGPL section 13 requires. Request it, naming the revision\n\
             above, from the operator at {SOURCE_OFFER}.\n",
            version = env!("CARGO_PKG_VERSION"),
            rev = rev(),
        ),
    )
        .into_response()
}

/// The deploy script's readiness probe: the process is up, the database
/// answers, and this is what it was built from.
async fn health(State(state): State<AppState>) -> Response {
    let cursor = match state.readers.get().await {
        Ok(reader) => read::latest_cursor(&reader).await.ok(),
        Err(_) => None,
    };
    let body = json!({
        "ok": cursor.is_some(),
        "rev": rev(),
        "database": if cursor.is_some() { "ok" } else { "unavailable" },
        "cursor": cursor.map(|c| c.to_string()),
    });
    let status =
        if cursor.is_some() { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE };
    (status, axum::Json(body)).into_response()
}

// ------------------------------------------------------------- stream budget

/// One live subscription's slot in the per-client budget, released on drop —
/// which for an SSE stream is when the client disconnects.
pub struct StreamSlot {
    streams: Arc<Mutex<HashMap<String, usize>>>,
    key: String,
}

impl AppState {
    /// Claim a stream slot, or refuse when the client already holds its five
    /// (CONTEXT.md).
    fn claim_stream(&self, key: &str) -> Option<StreamSlot> {
        let mut streams = self.streams.lock().expect("stream budget lock");
        let count = streams.entry(key.to_owned()).or_insert(0);
        if *count >= MAX_STREAMS_PER_CLIENT {
            return None;
        }
        *count += 1;
        Some(StreamSlot { streams: self.streams.clone(), key: key.to_owned() })
    }
}

impl Drop for StreamSlot {
    fn drop(&mut self) {
        let mut streams = self.streams.lock().expect("stream budget lock");
        if let Some(count) = streams.get_mut(&self.key) {
            *count -= 1;
            if *count == 0 {
                streams.remove(&self.key);
            }
        }
    }
}
