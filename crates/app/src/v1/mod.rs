//! The public API: `/v1` REST + the change feed, as a plain axum sub-router
//! merged beside the dioxus application (docs/research/api-layer.md §1).
//!
//! Server functions stay the dashboard's private RPC; everything an external
//! client touches lives here, where we own the URL space, the error contract
//! and the middleware.

pub mod isolate;
pub mod auth;
pub mod docs;
pub mod health;
pub mod json;
pub mod metrics;
pub mod openapi;
pub mod sql;
pub mod sse;
pub mod webhooks;

pub use auth::AuthUser;

use axum::Router;
use axum::extract::{FromRequestParts, Path, Query, State};
use axum::http::request::Parts;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
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
    /// Where reads whose filter shape CAN walk are executed, so they cannot starve
    /// `readers` (issue 120). Routing is [`store::read::walks`].
    pub isolated: Arc<isolate::IsolatedReads>,
    /// Rows per SSE snapshot page — the per-subscription memory bound and the
    /// spacing of its cancellation points (issue 55). Production keeps the
    /// default; tests shrink it to exercise multi-page snapshots.
    pub snapshot_page: i64,
    /// The job supervisor, when one runs beside this API (issue 65) — `/metrics`
    /// scrapes the running job's phase from it. `None` in tests and any embedding
    /// that serves the API without an ingestion worker; the gauges are then
    /// absent, not zero, same rule as every other unmeasured source there.
    pub jobs: Option<Arc<crate::supervisor::Supervisor>>,
    streams: Arc<Mutex<HashMap<String, usize>>>,
}

impl AppState {
    pub fn new(db: Arc<store::Db>, readers: Arc<store::Readers>) -> AppState {
        AppState::with_sql_timeout(db, readers, sql::DEFAULT_TIMEOUT)
    }

    /// As [`new`](AppState::new), with an explicit `/v1/sql` time limit.
    /// Production uses the default (10 s); a test passes a short one to watch the
    /// 408 cap fire without running a 10 s query.
    pub fn with_sql_timeout(
        db: Arc<store::Db>,
        readers: Arc<store::Readers>,
        sql_timeout: Duration,
    ) -> AppState {
        let cursor = db.cursor_watch();
        // A pool of readers dedicated to `/v1/sql`, kept apart from the REST
        // pool so a slow analytical query cannot starve the live API.
        let sql = Arc::new(sql::SqlState::with_timeout(
            db.readers(sql::SQL_READERS).expect("sql reader pool"),
            sql_timeout,
        ));
        // The isolated runtime and pool for reads whose filter shape can walk, so a
        // walk nobody can cancel never holds one of `readers` (issue 120).
        let isolated =
            Arc::new(isolate::IsolatedReads::new(&db).expect("isolated read pool"));
        AppState {
            db,
            readers,
            cursor,
            sql,
            isolated,
            snapshot_page: sse::SNAPSHOT_PAGE,
            jobs: None,
            streams: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

/// The API sub-router. Rate limiting covers `/v1` only: `/health` must answer
/// for the deploy script under any load, and `/_source` is a licence
/// obligation, not a service.
pub fn router(state: AppState) -> Router {
    let limits = GovernorConfigBuilder::default()
        // tower_governor's `per_second(n)` sets the REPLENISH PERIOD — n seconds per
        // cell — so `per_second(10)` is one request every 10 s (0.1 rps), NOT 10 rps.
        // That silently throttled the public surface ~100x below the "~10 rps
        // sustained" posture (CONTEXT.md): a client paging results got the 50-cell
        // burst and then one request per 10 s (measured 2026-08-15). Express the rate
        // as a period instead: 1000/10 = 100 ms per cell = 10 rps.
        .per_millisecond(1000 / RATE_PER_SECOND)
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
        .route("/v1/notices/{id}/content", get(notice_content))
        .route("/v1/changes", get(changes))
        .route("/v1/me", get(me))
        .merge(sql::routes())
        .merge(webhooks::routes())
        // Any other `/v1/*` path is an unknown endpoint, answered with our JSON
        // 404 rather than falling through to the dashboard's HTML router and
        // leaking its route names (issue 51). Static routes above are more
        // specific, so this only catches the genuinely unmatched.
        .route("/v1/{*rest}", any(unknown_endpoint))
        // The rate limiter's own 429 is emitted as our JSON envelope, not
        // tower_governor's plain-text default (issue 51).
        .layer(GovernorLayer::new(limits).error_handler(governor_json_error))
        .route("/health", get(health))
        // The deep operational probe an external pinger watches — liveness plus
        // ingest freshness, job failures and disk. Outside the rate limiter, like
        // `/health`: a pinger must never be throttled (issue 24).
        .route("/health/deep", get(health::deep))
        // The Prometheus scrape (issue 53). Outside the rate limiter for the
        // same reason as the health probes — a scraper on a fixed cadence must
        // not spend, or be refused by, the public request budget. Deliberately
        // NOT in `is_public_surface`: it is an operator surface, so browser
        // JavaScript on other origins has no business reading it, and every
        // gauge it exposes is an operational level rather than corpus data.
        .route("/metrics", get(metrics::metrics))
        .route("/_source", get(source))
        // The human-readable API reference. Outside the rate limiter (like
        // `/_source`): reading the docs is not a service call and must not spend
        // a caller's API budget.
        .route("/docs", get(docs::page))
        // Its machine-readable twin — also documentation, also outside the
        // limiter. (Registered after the governor layer like `/docs`, but the
        // static path still wins over the `/v1/{*rest}` catch-all: axum routes
        // by specificity, not registration order.)
        .route("/v1/openapi.json", get(openapi::spec))
        // Outermost, so every response — the governor's 429s included — passes
        // through it: the unauthenticated surface is CORS-open to any origin.
        .layer(axum::middleware::from_fn(public_cors))
        .with_state(state)
}

// --------------------------------------------------------------------- cors

/// The unauthenticated read surface — what browser JavaScript on ANY origin
/// may call. Method + path, mirroring the router's credential-free routes;
/// the completeness test below and the e2e CORS test in `tests/api.rs` keep
/// the mirror honest. Token-gated endpoints (`/v1/me`, `/v1/sql`,
/// `/v1/webhooks…`) are deliberately absent: nothing here is
/// cookie-credentialed, so opening them would not be unsafe — but the public
/// grant is scoped to what needs no credential at all.
fn is_public_surface(method: &axum::http::Method, path: &str) -> bool {
    // HEAD rides along: axum's `get()` routes serve it, so the grant matches.
    if method != axum::http::Method::GET && method != axum::http::Method::HEAD {
        return false;
    }
    match path {
        "/v1" | "/v1/tenders" | "/v1/lots" | "/v1/organizations" | "/v1/notices"
        | "/v1/changes" | "/v1/sql/schema" | "/v1/openapi.json" | "/health" | "/health/deep"
        | "/_source" | "/docs" => true,
        // The id-detail forms: exactly one extra segment, nothing deeper.
        _ => ["/v1/tenders/", "/v1/organizations/", "/v1/notices/"].iter().any(|prefix| {
            path.strip_prefix(prefix).is_some_and(|rest| !rest.is_empty() && !rest.contains('/'))
        }),
    }
}

/// CORS for [`is_public_surface`]: `Access-Control-Allow-Origin: *` on every
/// response, and the OPTIONS preflight answered here — a reconnecting
/// `EventSource` sends `Last-Event-ID`, which is not CORS-safelisted, so SSE
/// resume from a browser needs the preflight to succeed.
async fn public_cors(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    use axum::http::{HeaderValue, Method};
    if req.method() == Method::OPTIONS && is_public_surface(&Method::GET, req.uri().path()) {
        return (
            StatusCode::NO_CONTENT,
            [
                (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"),
                (header::ACCESS_CONTROL_ALLOW_METHODS, "GET, OPTIONS"),
                (header::ACCESS_CONTROL_ALLOW_HEADERS, "Accept, Content-Type, Last-Event-ID"),
                (header::ACCESS_CONTROL_MAX_AGE, "86400"),
            ],
        )
            .into_response();
    }
    let public = is_public_surface(req.method(), req.uri().path());
    let mut response = next.run(req).await;
    if public {
        let headers = response.headers_mut();
        headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));
        // Retry-After is not a CORS-safelisted response header; a browser
        // client backing off from a 429 must be able to read it.
        headers.insert(
            header::ACCESS_CONTROL_EXPOSE_HEADERS,
            HeaderValue::from_static("Retry-After"),
        );
    }
    response
}

#[cfg(test)]
mod cors_tests {
    use super::is_public_surface;
    use axum::http::Method;

    #[test]
    fn the_public_surface_is_exactly_the_credential_free_routes() {
        for path in [
            "/v1", "/v1/tenders", "/v1/tenders/14327", "/v1/lots", "/v1/organizations",
            "/v1/organizations/9", "/v1/notices", "/v1/notices/12", "/v1/changes",
            "/v1/sql/schema", "/v1/openapi.json", "/health", "/health/deep", "/_source", "/docs",
        ] {
            assert!(is_public_surface(&Method::GET, path), "{path} is public");
        }
        // Token-gated, unknown, and deeper paths are not in the grant.
        for path in [
            "/v1/me", "/v1/sql", "/v1/webhooks", "/v1/webhooks/3", "/v1/tenders/1/x", "/v1/x", "/",
        ] {
            assert!(!is_public_surface(&Method::GET, path), "{path} is not public");
        }
        assert!(!is_public_surface(&Method::POST, "/v1/tenders"), "only GET/HEAD are granted");
        assert!(is_public_surface(&Method::HEAD, "/v1/tenders"), "HEAD rides along with GET");
    }
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

/// The JSON 404 for any `/v1/*` path we do not serve — so an unknown endpoint
/// never falls through to the dashboard's HTML router or leaks a route name.
async fn unknown_endpoint() -> ApiError {
    ApiError::not_found("endpoint")
}

/// tower_governor's 429, re-dressed as our error envelope with a `Retry-After`.
fn governor_json_error(error: GovernorError) -> Response {
    match error {
        GovernorError::TooManyRequests { wait_time, .. } => (
            StatusCode::TOO_MANY_REQUESTS,
            [(header::RETRY_AFTER, wait_time.to_string())],
            axum::Json(json!({ "error": {
                "status": 429,
                "message": format!("rate limit exceeded; retry after {wait_time}s"),
            } })),
        )
            .into_response(),
        GovernorError::UnableToExtractKey => {
            ApiError(StatusCode::INTERNAL_SERVER_ERROR, "could not identify the client".to_owned())
                .into_response()
        }
        GovernorError::Other { code, msg, .. } => {
            ApiError(code, msg.unwrap_or_else(|| "rate limiter error".to_owned())).into_response()
        }
    }
}

// -------------------------------------------------------------- extractors

/// `Query`/`Path` with our JSON error envelope in place of axum's plain-text
/// rejection, which for a path leaked the Rust type (`… to a i64`). Every `/v1`
/// input error is then the one documented `{"error":{…}}` shape (issue 51).
struct ApiQuery<T>(T);

impl<T, S> FromRequestParts<S> for ApiQuery<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, ApiError> {
        match Query::<T>::from_request_parts(parts, state).await {
            Ok(Query(value)) => Ok(ApiQuery(value)),
            // The message names the offending query field ("unknown field `cvp`"),
            // which is public API vocabulary — safe and useful to surface.
            Err(rejection) => Err(ApiError::bad_request(rejection.body_text())),
        }
    }
}

struct ApiPath<T>(T);

impl<T, S> FromRequestParts<S> for ApiPath<T>
where
    T: DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, ApiError> {
        match Path::<T>::from_request_parts(parts, state).await {
            Ok(Path(value)) => Ok(ApiPath(value)),
            // A fixed message, never the extractor's — it named the Rust type.
            Err(_) => Err(ApiError::bad_request("invalid path parameter: expected an integer id")),
        }
    }
}

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
    /// Tenders this Organization submitted a bid on (a `tenderer`), won or not.
    bidder: Option<i64>,
    status: Option<String>,
    min_value: Option<i64>,
    max_value: Option<i64>,
    kind: Option<String>,
    tender: Option<i64>,
    /// Official notice number (`publication_id`); exact-match on `/v1/notices` (issue 217).
    publication_id: Option<String>,
    /// Official organization identifier value (e.g. a VAT number); exact-match on
    /// `/v1/organizations`, pair with `kind` for the scheme (issue 217).
    identifier: Option<String>,
    /// Publication-date bounds on the current version (issue 216): unix seconds or
    /// RFC 3339. `after` inclusive, `before` exclusive. Tenders-only.
    published_after: Option<String>,
    published_before: Option<String>,
    /// Submission-deadline bounds on the current version (issue 216, deadline
    /// half): same format and contract as the published pair. Tenders-only.
    deadline_after: Option<String>,
    deadline_before: Option<String>,
    /// Case-insensitive organization name prefix (issue 217-B);
    /// `/v1/organizations` only.
    name_prefix: Option<String>,
    /// `/v1/tenders` REST only (issue 216): `id` (default), `published_at` or
    /// `deadline`. A bound on one date column implies sorting by it unless an
    /// explicit `sort` says otherwise.
    sort: Option<String>,
    /// `asc` | `desc`. Defaults: `asc` for `sort=id`, `desc` for `sort=published_at`.
    order: Option<String>,
    /// Pagination position: the last id of the previous page (or, under
    /// `sort=published_at`, the previous page's opaque `next_cursor` verbatim).
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
            bidder: self.bidder,
            status,
            min_value: self.min_value,
            max_value: self.max_value,
            kind: self.kind.clone(),
            tender: self.tender,
            publication_id: self.publication_id.clone(),
            identifier: self.identifier.clone(),
            published_after: parse_instant(self.published_after.as_deref(), "published_after")?,
            published_before: parse_instant(self.published_before.as_deref(), "published_before")?,
            deadline_after: parse_instant(self.deadline_after.as_deref(), "deadline_after")?,
            deadline_before: parse_instant(self.deadline_before.as_deref(), "deadline_before")?,
            // Unicode-lowercased HERE, once, so the store layer always sees the
            // normalised form the name_norm column stores. An empty prefix would
            // be an unbounded name-ordered dump of 24.6M orgs — refuse it.
            name_prefix: match self.name_prefix.as_deref().map(str::trim) {
                None => None,
                Some("") => {
                    return Err(ApiError::bad_request(
                        "name_prefix must not be empty; pass at least one character",
                    ));
                }
                Some(p) => Some(p.to_lowercase()),
            },
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

    /// The filter parameters the client actually set, by their client-facing name.
    ///
    /// The list handler diffs this against the collection's
    /// [`store::read::Collection::honoured_params`] to name the filters that were
    /// accepted but changed nothing (issue 118). The order here is fixed — the filter
    /// vocabulary in declaration order — so the echoed `ignored_filters` is
    /// deterministic regardless of query-string order. Pagination and streaming
    /// controls (`cursor`, `limit`, `since`, `entity`, `include_data`) are not filters
    /// and never appear here; the names must match `honoured_params`' spelling exactly,
    /// since a mismatch would silently report a honoured filter as ignored.
    fn provided_filters(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        if self.source.is_some() {
            out.push("source");
        }
        if self.country.is_some() {
            out.push("country");
        }
        if self.cpv.is_some() {
            out.push("cpv");
        }
        if self.buyer.is_some() {
            out.push("buyer");
        }
        if self.winner.is_some() {
            out.push("winner");
        }
        if self.bidder.is_some() {
            out.push("bidder");
        }
        if self.status.is_some() {
            out.push("status");
        }
        if self.min_value.is_some() {
            out.push("min_value");
        }
        if self.max_value.is_some() {
            out.push("max_value");
        }
        if self.kind.is_some() {
            out.push("kind");
        }
        if self.tender.is_some() {
            out.push("tender");
        }
        if self.publication_id.is_some() {
            out.push("publication_id");
        }
        if self.identifier.is_some() {
            out.push("identifier");
        }
        if self.published_after.is_some() {
            out.push("published_after");
        }
        if self.published_before.is_some() {
            out.push("published_before");
        }
        if self.deadline_after.is_some() {
            out.push("deadline_after");
        }
        if self.deadline_before.is_some() {
            out.push("deadline_before");
        }
        if self.name_prefix.is_some() {
            out.push("name_prefix");
        }
        out
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

impl From<Collection> for store::read::Collection {
    /// The read layer has its own `Collection` because the isolation routing lives
    /// next to the predicates it classifies (`store::read`), and `store` cannot depend
    /// on `app`. Two enums that must agree is a drift hazard, so this match is
    /// exhaustive: adding a collection here fails to compile until it is classified
    /// there.
    fn from(c: Collection) -> Self {
        match c {
            Collection::Tenders => store::read::Collection::Tenders,
            Collection::Lots => store::read::Collection::Lots,
            Collection::Organizations => store::read::Collection::Organizations,
            Collection::Notices => store::read::Collection::Notices,
        }
    }
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

/// Whether a matching entity exists at `scope` — the SSE diff's classification need
/// (added/changed/removed is decided by presence on each side of a change), WITHOUT
/// the display decoration the diff throws away unless `?include_data=true` (issue
/// 221). Only `Lots` has a separable, whole-slice `summarise`, so only it takes the
/// identity-only path; every other collection's identity query already IS its
/// cheapest read, so an existence check over `read_items` is as cheap as it gets.
pub async fn read_matches(
    collection: Collection,
    conn: &store::turso::Connection,
    filter: &Filter,
    scope: Scope,
) -> store::turso::Result<bool> {
    Ok(match collection {
        Collection::Lots => !read::lots_identity(conn, filter, scope).await?.is_empty(),
        _ => !read_items(collection, conn, filter, scope).await?.is_empty(),
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
    // One extra row answers "is there another page?" without a second query.
    let scope = Scope::Page { after: params.after(), limit: limit + 1 };
    // A filter shape that CAN walk runs on the isolated runtime and pool, so a walk
    // nobody can cancel cannot hold one of the API's own readers (issue 120). The
    // routing is derived from the read layer's own predicates, never a list here.
    let mut items = if store::read::walks(collection.into(), &filter) {
        match state.isolated.read(collection, filter.clone(), scope).await {
            Ok(result) => result?,
            Err(isolate::Shed) => {
                return Err(ApiError(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "too many expensive filtered reads in flight; retry shortly".into(),
                ));
            }
        }
    } else {
        let reader = state.readers.get().await?;
        read_items(collection, &reader, &filter, scope).await?
    };
    let next = (items.len() as i64 > limit).then(|| items[limit as usize - 1].id.to_string());
    items.truncate(limit as usize);
    // Name any filter the client sent that this collection does not apply, so an
    // unfiltered page never masquerades as a filtered one (issue 118). The honoured
    // set lives in the read layer next to the builders it describes.
    let honoured = store::read::Collection::from(collection).honoured_params();
    let ignored: Vec<&str> =
        params.provided_filters().into_iter().filter(|p| !honoured.contains(p)).collect();
    Ok(axum::Json(json::page(items.into_iter().map(|i| i.json).collect(), next, &ignored))
        .into_response())
}

/// A client-supplied instant: unix seconds, or RFC 3339 (the format every
/// timestamp in the responses uses). Anything else is a 400 naming the parameter.
fn parse_instant(value: Option<&str>, name: &str) -> Result<Option<i64>, ApiError> {
    let Some(raw) = value else { return Ok(None) };
    if let Ok(unix) = raw.parse::<i64>() {
        return Ok(Some(unix));
    }
    // A `+` in a query string URL-decodes to a space, so a pasted
    // `…T09:30:00+01:00` arrives as `…T09:30:00 01:00`. RFC 3339 has no bare
    // space at that position, so restoring the `+` is unambiguous — without this,
    // every timestamp the API itself serves would 400 when pasted back unencoded.
    let restored;
    let candidate = if raw.contains(' ') {
        restored = raw.replace(' ', "+");
        restored.as_str()
    } else {
        raw
    };
    match chrono::DateTime::parse_from_rfc3339(candidate) {
        Ok(dt) => Ok(Some(dt.timestamp())),
        Err(_) => Err(ApiError::bad_request(format!(
            "{name} must be unix seconds or RFC 3339, not {raw:?}"
        ))),
    }
}

fn wants_events(headers: &HeaderMap) -> bool {
    headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|accept| accept.contains("text/event-stream"))
}

// ------------------------------------------------------------------ handlers

async fn tenders(State(s): State<AppState>, h: HeaderMap, ApiQuery(p): ApiQuery<Params>) -> ApiResult {
    // Sort resolution (issue 216). Validated here because only /v1/tenders sorts;
    // a published bound implies published order (the id-ordered application of a
    // narrow range is walk-shaped — it exists for SSE snapshots and explicit
    // sort=id, both isolated), and explicit sort=id keeps today's ascending list.
    // Copy verdicts, not borrows: `p` moves into the dispatched handler below.
    // `sort` names the ordering; a bound on one date column implies sorting by it.
    #[derive(Clone, Copy, PartialEq)]
    enum SortChoice {
        Unset,
        Id,
        Published,
        Deadline,
    }
    let sort = match p.sort.as_deref() {
        None => SortChoice::Unset,
        Some("id") => SortChoice::Id,
        Some("published_at") => SortChoice::Published,
        Some("deadline") => SortChoice::Deadline,
        Some(other) => {
            return Err(ApiError::bad_request(format!(
                "sort must be 'id', 'published_at' or 'deadline', not {other:?}"
            )));
        }
    };
    let (order_asc, order_desc) = match p.order.as_deref() {
        None => (false, false),
        Some("asc") => (true, false),
        Some("desc") => (false, true),
        Some(other) => {
            return Err(ApiError::bad_request(format!(
                "order must be 'asc' or 'desc', not {other:?}"
            )));
        }
    };
    let filter = p.filter(store::now_unix())?;
    let published_range = filter.published_after.is_some() || filter.published_before.is_some();
    let deadline_range = filter.deadline_after.is_some() || filter.deadline_before.is_some();
    let sort = match sort {
        SortChoice::Unset => match (published_range, deadline_range) {
            (true, true) => {
                return Err(ApiError::bad_request(
                    "both published and deadline bounds given; pass sort=published_at or \
                     sort=deadline to choose the ordering",
                ));
            }
            (true, false) => SortChoice::Published,
            (false, true) => SortChoice::Deadline,
            (false, false) => SortChoice::Id,
        },
        explicit => explicit,
    };
    let head_order = match sort {
        SortChoice::Published => Some(read::HeadOrder::PublishedAt),
        SortChoice::Deadline => Some(read::HeadOrder::Deadline),
        _ => None,
    };
    let Some(head_order) = head_order else {
        if order_desc {
            return Err(ApiError::bad_request(
                "descending id order is not supported; use sort=published_at for newest-first",
            ));
        }
        return collection(Collection::Tenders, s, h, p).await;
    };
    if wants_events(&h) {
        return Err(ApiError::bad_request(
            "sort does not apply to event streams: a subscription snapshots in id order \
             and then follows the change log; subscribe without sort/order",
        ));
    }
    // Direction defaults follow the question each ordering answers: newest-first
    // for publication ("what just came out"), soonest-first for deadlines ("what
    // closes soon").
    let desc = match head_order {
        read::HeadOrder::PublishedAt => !order_asc,
        read::HeadOrder::Deadline => order_desc,
    };
    tenders_ordered(s, p, filter, head_order, desc).await
}

/// The ordered Tender list (issue 216): the REST half that rides the ordering
/// column\'s `(column, id)` index. The cursor is `<key>.<id>` of the last row,
/// opaque to clients (echo `next_cursor` verbatim).
async fn tenders_ordered(
    state: AppState,
    params: Params,
    filter: Filter,
    order: read::HeadOrder,
    desc: bool,
) -> ApiResult {
    // The cursor is tagged with its sort column. Both orderings carry an
    // `<epoch>.<id>` position, so without the tag a published_at cursor pasted
    // into sort=deadline PARSES — and silently returns a page keyed off the
    // wrong column (found by probing the documented "a cursor is specific to
    // its sort" claim against prod). The tag makes the documented 400 real.
    let tag = match order {
        read::HeadOrder::PublishedAt => 'p',
        read::HeadOrder::Deadline => 'd',
    };
    let cursor = match params.cursor.as_deref() {
        None => None,
        Some(raw) => match raw.strip_prefix(tag).and_then(|rest| {
            let (k, i) = rest.split_once('.')?;
            Some((k.parse::<i64>().ok()?, i.parse::<i64>().ok()?))
        }) {
            Some(pair) => Some(pair),
            None => {
                return Err(ApiError::bad_request(
                    "cursor does not match this sort; pass the previous page's \
                     next_cursor verbatim, or drop it to restart",
                ));
            }
        },
    };
    let limit = params.limit();
    // The published bounds are SERVED by this read's index ride; the REMAINING
    // filters decide isolation exactly as on the id-ordered list (issue 120) — a
    // sparse version-predicate walks the ordered stream the same way it walks the
    // PK, so it must not hold a main-pool reader.
    let stripped = order.strip_served(&filter);
    let mut rows = if store::read::walks(store::read::Collection::Tenders, &stripped) {
        match state.isolated.read_ordered(filter, order, desc, cursor, limit + 1).await {
            Ok(result) => result?,
            Err(isolate::Shed) => {
                return Err(ApiError(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "too many expensive filtered reads in flight; retry shortly".into(),
                ));
            }
        }
    } else {
        let reader = state.readers.get().await?;
        read::tenders_ordered(&reader, &filter, order, desc, cursor, limit + 1).await?
    };
    let next = (rows.len() as i64 > limit).then(|| {
        let last = &rows[limit as usize - 1];
        let key = match order {
            read::HeadOrder::PublishedAt => last.published_at,
            // The deadline the row was ordered by; the row always HAS one here
            // (NULL-deadline rows are excluded from this ordering), and the list
            // row\'s deadline is the same MAX the column materialises.
            read::HeadOrder::Deadline => last.deadline.map(|d| d.utc_seconds).unwrap_or(0),
        };
        format!("{tag}{}.{}", key, last.id)
    });
    rows.truncate(limit as usize);
    let honoured = store::read::Collection::Tenders.honoured_params();
    let ignored: Vec<&str> =
        params.provided_filters().into_iter().filter(|f| !honoured.contains(f)).collect();
    let items: Vec<serde_json::Value> = rows.iter().map(json::tender).collect();
    Ok(axum::Json(json::page(items, next, &ignored)).into_response())
}

async fn lots(State(s): State<AppState>, h: HeaderMap, ApiQuery(p): ApiQuery<Params>) -> ApiResult {
    reject_sort(&p, "/v1/lots")?;
    collection(Collection::Lots, s, h, p).await
}

async fn organizations(
    State(s): State<AppState>,
    h: HeaderMap,
    ApiQuery(p): ApiQuery<Params>,
) -> ApiResult {
    reject_sort(&p, "/v1/organizations")?;
    let filter = p.filter(store::now_unix())?;
    // issue 217-B: a REST name-prefix search rides organizations_name_norm_id in
    // name order (the probe-settled fast path). SSE keeps the id-ordered
    // collection path, where walks() isolates the prefix application.
    if let (Some(prefix), false) = (filter.name_prefix.clone(), wants_events(&h)) {
        return organizations_by_name(s, p, filter, prefix).await;
    }
    collection(Collection::Organizations, s, h, p).await
}

/// The name-ordered organization search (issue 217-B). Cursor: `<id>~<name_norm>`
/// of the last row — id first (digits, so the FIRST `~` always ends it; a name
/// may contain anything, including `~`).
async fn organizations_by_name(
    state: AppState,
    params: Params,
    filter: Filter,
    prefix: String,
) -> ApiResult {
    let cursor = match params.cursor.as_deref() {
        None => None,
        Some(raw) => match raw
            .split_once('~')
            .and_then(|(id, norm)| Some((norm.to_owned(), id.parse::<i64>().ok()?)))
        {
            Some(pair) => Some(pair),
            None => {
                return Err(ApiError::bad_request(
                    "cursor does not match a name_prefix search; pass the previous page's \
                     next_cursor verbatim, or drop it to restart",
                ));
            }
        },
    };
    let limit = params.limit();
    // A companion filter beside the name range flips the planner onto the
    // companion's index and scans its whole slice (country=DE: 4.9 s over 3.85M
    // rows, measured on prod) — correct but walk-shaped, so it runs isolated;
    // the bare prefix seeks in ~2 ms and stays on the main pool. A
    // (country, name_norm, id) composite would make the pairing seek too —
    // tracked on issue 217.
    let companioned = filter.country.is_some() || filter.kind.is_some();
    let mut rows = if companioned {
        match state.isolated.read_org_named(filter, prefix, cursor, limit + 1).await {
            Ok(result) => result?,
            Err(isolate::Shed) => {
                return Err(ApiError(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "too many expensive filtered reads in flight; retry shortly".into(),
                ));
            }
        }
    } else {
        let reader = state.readers.get().await?;
        read::organizations_by_name(&reader, &filter, &prefix, cursor, limit + 1).await?
    };
    let next = (rows.len() as i64 > limit).then(|| {
        let last = &rows[limit as usize - 1];
        format!("{}~{}", last.id, last.name.to_lowercase())
    });
    rows.truncate(limit as usize);
    let honoured = store::read::Collection::Organizations.honoured_params();
    let ignored: Vec<&str> =
        params.provided_filters().into_iter().filter(|f| !honoured.contains(f)).collect();
    let items: Vec<serde_json::Value> = rows.iter().map(json::organization).collect();
    Ok(axum::Json(json::page(items, next, &ignored)).into_response())
}

/// `sort`/`order` are a /v1/tenders capability (issue 216). The other collections
/// REJECT rather than ignore them: a silently-unsorted page masquerading as a
/// sorted one is exactly the lie `ignored_filters` (issue 118) exists to prevent,
/// and these are not filters, so that channel cannot carry the honesty.
fn reject_sort(params: &Params, path: &str) -> Result<(), ApiError> {
    if params.sort.is_some() || params.order.is_some() {
        return Err(ApiError::bad_request(format!(
            "sort/order are not available on {path}; only /v1/tenders sorts"
        )));
    }
    Ok(())
}

async fn notices(State(s): State<AppState>, h: HeaderMap, ApiQuery(p): ApiQuery<Params>) -> ApiResult {
    reject_sort(&p, "/v1/notices")?;
    // `?tender=` lists the Notices that caused a Tender's versions — the
    // ADR-0001 chain, walkable from a detail's `caused_by_notice_id`. The store
    // has no notice→tender predicate, so this is answered in the app from the
    // tender detail rather than silently ignored (issue 49). A lookup, not a
    // subscription, so it is JSON regardless of Accept.
    if let Some(tender_id) = p.tender {
        // This path applies `tender` and nothing else, so any other filter the
        // client sent is dropped — name it, exactly as the collection path does.
        let ignored: Vec<&str> =
            p.provided_filters().into_iter().filter(|f| *f != "tender").collect();
        return tender_notices(&s, tender_id, &ignored).await;
    }
    collection(Collection::Notices, s, h, p).await
}

async fn tender(State(state): State<AppState>, ApiPath(id): ApiPath<i64>) -> ApiResult {
    let reader = state.readers.get().await?;
    match read::tender_detail(&reader, id).await? {
        Some(detail) => Ok(axum::Json(json::detail(&detail)).into_response()),
        None => Err(ApiError::not_found("tender")),
    }
}

/// `GET /v1/notices/{id}` — one Notice by id, the counterpart of the
/// `caused_by_notice_id` a tender detail hands out (issue 49).
async fn notice(State(state): State<AppState>, ApiPath(id): ApiPath<i64>) -> ApiResult {
    let reader = state.readers.get().await?;
    match read::notices(&reader, &Filter::default(), Scope::At { id, seq: 0 }).await?.into_iter().next()
    {
        // issue 218: a held notice has no parsed satellites and no tender, so the
        // quarantine row is its only content — surface it here instead of the bare
        // metadata stub. One bounded `(notice_id)` lookup, only on the by-id path.
        Some(row) => {
            let quarantine = read::notice_quarantine(&reader, id).await?;
            Ok(axum::Json(json::notice_detail(&row, quarantine.as_ref())).into_response())
        }
        None => Err(ApiError::not_found("notice")),
    }
}

/// `GET /v1/notices/{id}/content` — the notice's whole parsed payload (issue
/// 218-B): the section tree and every typed field value, verbatim from the parse
/// layer (source field ids, not canonical projections). Bounded: one notice's
/// satellite slices, read by the same `parsed_by_ids` seek set the projection
/// folds from. A quarantined notice legitimately has zero sections — held whole,
/// never partially imported (ADR-0004) — and the detail's `quarantine` field says
/// why; an unknown id is a 404, so the two "nothing here" cases stay distinct.
async fn notice_content(State(state): State<AppState>, ApiPath(id): ApiPath<i64>) -> ApiResult {
    let reader = state.readers.get().await?;
    if read::notices(&reader, &Filter::default(), Scope::At { id, seq: 0 }).await?.is_empty() {
        return Err(ApiError::not_found("notice"));
    }
    drop(reader);
    let parsed = state
        .db
        .parsed_by_ids(&[id])
        .await?
        .into_iter()
        .next()
        .map(|(_, p)| p)
        .unwrap_or_default();
    Ok(axum::Json(json::notice_content(id, &parsed)).into_response())
}

/// `GET /v1/organizations/{id}` — one Organization by id, the counterpart of a
/// tender detail's `parties[].organization_id` (issue 49).
async fn organization(State(state): State<AppState>, ApiPath(id): ApiPath<i64>) -> ApiResult {
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
async fn tender_notices(state: &AppState, tender_id: i64, ignored: &[&str]) -> ApiResult {
    let reader = state.readers.get().await?;
    // Only the version→notice mapping is needed. A full `tender_detail` (issue 220)
    // ran ~17 satellite queries — lots + `summarise`, lot_results/bids/contracts with
    // their org joins, parties, amounts, dates — on the main reader pool and then
    // discarded all but this, thousands of rows fetched for a handful of notice ids on
    // a high-lot tender. A tender with no versions is an unknown id (every real Tender
    // has ≥1), so emptiness is the 404 — the same 404 `tender_detail` gave.
    let mut ids = read::tender_version_notice_ids(&reader, tender_id).await?;
    if ids.is_empty() {
        return Err(ApiError::not_found("tender"));
    }
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
    // A fixed sub-resource — the notices this tender's versions cite. It applies
    // `tender` only; any other filter the caller sent is named as ignored.
    Ok(axum::Json(json::page(items, None, ignored)).into_response())
}

/// The poll half of the change feed. Same events, same cursor and same
/// filtering as SSE — a client that cannot hold a connection open loses
/// nothing but latency.
async fn changes(State(state): State<AppState>, ApiQuery(params): ApiQuery<Params>) -> ApiResult {
    // The `entity` filter is the public-feed enum only (issue 211): the projection
    // also writes lot_result/bid/contract change rows, but those are not on the
    // public feed (SSE never emits them, they are absent from the schema, and their
    // ids resolve to no endpoint). Reject an out-of-enum value with a 400 rather
    // than returning undocumented rows.
    if let Some(entity) = params.entity.as_deref()
        && !sse::is_public_change_kind(entity)
    {
        return Err(ApiError::bad_request(format!(
            "unknown entity {entity:?}; expected one of tender, lot, organization"
        )));
    }
    let limit = params.limit();
    let reader = state.readers.get().await?;
    // Fetch one past the page (issue 215-C): `more` comes from the overflow row, not
    // from a full page, so an exactly-`limit` final page reports `more:false` instead
    // of costing the client one extra empty poll — mirrors the list handler.
    let mut rows =
        read::changes_since(&reader, params.since(), limit + 1, params.entity.as_deref()).await?;
    let more = rows.len() as i64 > limit;
    rows.truncate(limit as usize);
    let last = rows.last().map(|c| c.cursor).unwrap_or_else(|| params.since());
    // Serialize only the public-feed kinds (issue 211). The cursor still advances
    // over the FULL fetch (`last`/`more` above), so a window dominated by result-
    // graph rows carries the client past them without redelivering — keeping the
    // poll feed identical to what SSE emits.
    let events: Vec<Value> = rows
        .iter()
        .filter(|c| sse::is_public_change_kind(&c.entity_kind))
        .map(sse::change_event)
        .collect();
    Ok(axum::Json(json!({
        "events": events,
        "last_cursor": json::cursor(last),
        "more": more,
        // The feed's generation (issue 46). A poll client must store this with
        // its cursor: when it moves, the stored cursor and every entity id it
        // has are from a world that no longer exists — drop state, re-snapshot
        // the collections, and continue from this response's last_cursor.
        "generation": read::feed_generation(&reader).await?,
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
        "openapi": "/v1/openapi.json",
        "cursor": json::cursor(read::latest_cursor(&reader).await?),
        // Bumped by every rebuild that wipes the canonical layer or the change
        // log (issue 46): cursors and entity ids from different generations do
        // not compose — on a change, drop state and re-snapshot.
        "generation": read::feed_generation(&reader).await?,
        "endpoints": [
            "/v1/tenders", "/v1/tenders/{id}", "/v1/lots", "/v1/organizations",
            "/v1/organizations/{id}", "/v1/notices", "/v1/notices/{id}",
            "/v1/changes", "/v1/me", "/v1/sql", "/v1/sql/schema", "/v1/webhooks",
            "/v1/openapi.json",
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

/// The deploy script's post-restart gate: a LIVENESS probe. It confirms the
/// process is up and serving HTTP, and reports the revision it was built from. It
/// deliberately does NOT query the database (issue 61), so it stays instant and
/// answers `200` even while a projection saturates the reader pool — which is
/// exactly what a "did the new build come back up" gate needs. It has no unhealthy
/// path and must not imply one: a hung or crashed process fails this by not
/// answering at all, not by a `503`. For a database-backed readiness signal — DB
/// answering, ingest fresh, disk, canonical layer — see [`health::deep`]
/// (`/health/deep`).
async fn health(State(state): State<AppState>) -> Response {
    // In-memory doorbell read — no DB access (issue 61). The last-known ingestion
    // cursor is reported as a liveness DETAIL, not as a claim the database was
    // queried; the DB-answering check lives in `/health/deep`. This probe has no
    // failing path, so it does not pretend to (the old `is_some()`/`unavailable`/
    // `503` scaffolding was dead code over an infallible read — issue 213).
    let body = json!({
        "ok": true,
        "rev": rev(),
        "cursor": state.db.current_cursor().to_string(),
    });
    (StatusCode::OK, axum::Json(body)).into_response()
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

    /// Live subscriptions across all clients — the `/metrics` stream gauge.
    pub(super) fn live_streams(&self) -> usize {
        self.streams.lock().expect("stream budget lock").values().sum()
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

#[cfg(test)]
mod error_envelope_tests {
    use super::*;
    use tower_governor::GovernorError;

    #[tokio::test]
    async fn the_rate_limit_429_is_our_json_envelope() {
        // tower_governor's default 429 is plain text ("Too Many Requests! Wait
        // for Ns"); the handler must re-dress it as our envelope with a
        // Retry-After (issue 51).
        let response =
            governor_json_error(GovernorError::TooManyRequests { wait_time: 3, headers: None });
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response.headers().get(header::RETRY_AFTER).unwrap(), "3");
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"]["status"].as_i64(), Some(429));
        assert!(json["error"]["message"].as_str().is_some_and(|m| m.contains("retry after 3s")));
    }
}
