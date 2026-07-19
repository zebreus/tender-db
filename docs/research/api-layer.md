# API layer: implementation patterns and crate choices

Research notes, 2026-07-19. Desk research against docs.rs, crate source in the
local cargo registry (`~/.cargo/registry/src/…`), crates.io API metadata, and
vendor/spec documents. Version compatibility checked against this repo's
`Cargo.lock` (dioxus 0.7.9, axum 0.8.9, tokio 1.53.0, tower 0.5.3,
tower-http 0.6.11, hyper 1.10.1, turso 0.7.0). Confidence is marked per
section; **verified** means read from primary source (crate source or official
docs), **known** means from general knowledge with a citation but not
re-checked in depth.

---

## 1. Composing REST+SSE alongside Dioxus server functions

### Which axum, and how the router composes (verified, from crate source)

dioxus-server 0.7.9 depends on `axum ^0.8.4`; our lock resolves the single
in-tree axum to **0.8.9**, which is also the latest stable axum as of today —
no version skew to manage. tower is 0.5.3, tower-http 0.6.11 (dioxus-server
only enables tower-http's `fs` feature — notably it adds **no compression
layer**, so nothing in the dioxus stack will buffer or gzip an SSE response).

From `dioxus-server-0.7.9/src/launch.rs` and `server.rs`:

- `dioxus::server::router(app) -> axum::Router<()>` — builds
  `Router::new().serve_dioxus_application(ServeConfig::new(), app)`, which
  registers all server functions (collected via `inventory`), serves static
  assets, adds the SSR fallback, and **already applies its state**
  (`.with_state(FullstackState::new(..))`), returning a plain `Router<()>`.
- `dioxus::server::serve(|| async { Ok(router) })` binds to the address from
  `dioxus_cli_config` (IP/PORT env, default 127.0.0.1:8080) and serves it; in
  debug it wires hot-reload, in release it is just `axum::serve`.

Because the dioxus router is state-erased (`Router<()>`), composition is the
vanilla axum pattern — build our API as an independent `Router` with its own
`AppState`, then merge:

```rust
dioxus::server::serve(|| async {
    let state = AppState::new().await?;          // Turso handle, watch channel, limiter…
    let api = api_router()                        // Router<AppState>
        .layer(/* tower layers apply only to these routes */)
        .with_state(state);
    Ok(dioxus::server::router(App).merge(api))    // both are Router<()>
});
```

Layers attached to our sub-router do not leak onto dioxus's routes and vice
versa. One collision hazard: server functions register at their literal paths
(`#[get("/api/tenders")]` in `crates/app/src/api.rs` claims `/api/tenders` on
the merged router), so the public API namespace and server-fn namespace must
not overlap — either give server functions an internal prefix (`/_dash/…`) or
give the public API a versioned prefix (`/v1/…`). Recommendation below: both.

### Server functions vs plain axum for the public API (verified)

Dioxus 0.7 server functions are more "real HTTP" than their reputation:
`#[get]/#[post]` produce ordinary routes, the default encoding is JSON
(`JsonEncoding`, content-type `application/json`, per
`dioxus-fullstack-0.7.9/src/encoding.rs`), so non-Rust clients *can* call
them. But for an external-facing API they are the wrong tool:

- **Execution model**: each server-fn request is `spawn_pinned` onto a
  `LocalPoolHandle` (a separate local pool for !Send futures —
  `dioxus-server-0.7.9/src/serverfn.rs`). Fine for dashboard RPC; an extra
  scheduling hop and thread-pool bottleneck for a high-fanout public API, and
  SSE streams should not live on that pool.
- **Error contract**: errors serialize in dioxus's `ServerFnError` JSON shape
  — not a contract we want external users to depend on or that we control.
- **OpenAPI / docs**: no story. Plain axum handlers can be annotated with
  utoipa and served as an OpenAPI document.
- **Versioning & content negotiation**: server-fn paths/encodings are macro
  fixed; axum gives us `/v1` nesting, `Accept` handling, cache headers, ETags.
- **Middleware**: auth extractors, per-route rate limits, and body-size caps
  are ordinary tower/axum idioms on our own router; threading them through
  `FullstackContext` is possible but awkward.

**Recommendation**: server functions only for the dashboard's own
client↔server needs (auth'd session RPC); the public API (REST, SSE, poll,
SQL, webhook registration) is plain axum handlers on the merged sub-router.
Confidence: high.

Sources: crate source at `~/.cargo/registry/src/index.crates.io-…/dioxus-server-0.7.9/src/{launch,server,serverfn}.rs`, `dioxus-fullstack-0.7.9/src/encoding.rs`; <https://docs.rs/dioxus-server/0.7.9> (docs build failed upstream; source consulted directly).

---

## 2. SSE mechanics in axum 0.8

### The responder (verified)

`axum::response::sse::{Sse, Event, KeepAlive}`
(<https://docs.rs/axum/0.8.9/axum/response/sse/>): handlers return
`Sse<impl Stream<Item = Result<Event, E>>>`; `Event` has `.data()`, `.id()`,
`.event()`, `.retry()`, `.comment()`, and `.json_data(&T)`.
`Sse::keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))` emits
comment lines (`:`) on idle — required so proxies and NATs don't kill quiet
connections; it also lets the server detect dead clients on write failure.

### Last-Event-ID (verified: no typed header exists)

Per the WHATWG spec, a reconnecting `EventSource` sends the last seen `id:`
field back as the `Last-Event-ID` request header
(<https://html.spec.whatwg.org/multipage/server-sent-events.html>). The
`headers` crate (0.4) has **no** `LastEventId` type — read it manually:

```rust
let resume = headers.get("last-event-id")
    .and_then(|v| v.to_str().ok()).and_then(|s| s.parse::<u64>().ok())
    .or(query.cursor);   // also accept ?cursor= for curl/non-EventSource clients
```

Set `id: <cursor>` on every diff event so any drop resumes exactly.

### Proxy / deployment notes (known, standard)

- nginx buffers proxied responses by default (`proxy_buffering on`), which
  delays SSE indefinitely. Either `proxy_buffering off` in the location, or —
  better, self-serve — send `X-Accel-Buffering: no` on the SSE response, which
  nginx honors per-response
  (<https://nginx.org/en/docs/http/ngx_http_proxy_module.html#proxy_buffering>).
  Also set `Cache-Control: no-store` explicitly.
- Long `proxy_read_timeout` (or rely on keep-alive comments arriving inside
  the default 60 s window — 15 s keep-alive is safely inside it).
- HTTP/1.1 browsers cap ~6 connections per host; SSE eats one each. Served
  behind an HTTP/2-terminating proxy this is a non-issue. Server side, cap
  subscriptions per token/IP with a counter in `AppState` (reject with 429).

### Fan-out: DB-as-log + `watch` doorbell, not payload broadcast (analysis)

Two standard shapes:

1. `tokio::sync::broadcast::channel<ChangeEvent>` — every subscriber gets each
   payload; bounded ring buffer; a slow client gets
   `RecvError::Lagged(n)` and has lost data
   (<https://docs.rs/tokio/latest/tokio/sync/broadcast/>), so you must build a
   DB-backed catch-up path *anyway*.
2. `tokio::sync::watch::Sender<u64>` carrying only the newest cursor
   (<https://docs.rs/tokio/latest/tokio/sync/watch/>): the change log lives in
   SQLite (it already does, per ADR-0001 — the canonical version sequence);
   `watch` is just the doorbell. Each SSE task loops: wait for
   `rx.changed()`, then `SELECT … FROM changes WHERE cursor > ?last_sent ORDER
   BY cursor LIMIT ?batch`, emit, repeat.

Option 2 is strictly better here: the live path, the resume path
(Last-Event-ID), and the poll endpoint (`/v1/changes?since=`) become the
*same* query; slow clients cannot lose data (they just read bigger batches
later — natural backpressure, since hyper stops polling the stream when the
socket is full); `watch` never lags (it only keeps the latest value); and a
single-process monolith with local SQLite makes the extra reads cheap. Drop a
client only if it stays more than a configured distance behind or on write
error. Confidence: high — this is the standard "database as the log" pattern
(same shape as Postgres logical replication consumers, §3).

### Snapshot-then-diff without a gap (analysis)

The classic race (miss events between snapshot read and subscription) is
avoided by ordering + idempotence, no locks needed:

1. **Subscribe first**: `let rx = state.cursor_watch.subscribe();`
2. **Snapshot**: one read transaction; capture `N = MAX(cursor)` *inside that
   transaction* (SQLite WAL read transactions see a stable snapshot); stream
   the matching set as `added` events, then emit a `live` marker with `id: N`.
3. **Diffs**: loop on `rx`, always querying `cursor > last_sent` starting from
   `last_sent = N`.

Because the cursor is strictly monotonic and the diff query is
`> last_sent`, any notification that fired during the snapshot is picked up by
the first diff query; nothing can be emitted twice or skipped. Resuming
clients skip step 2 entirely (diffs from their cursor), unless their cursor
is below the retention horizon — then send an explicit `reset` event (§3).

---

## 3. Change-feed precedents → cursor + event schema

### What the precedents teach (verified/known, cited)

- **CouchDB `_changes`**
  (<https://docs.couchdb.org/en/stable/api/database/changes.html>): resume via
  `since=<seq>`, responses carry `last_seq`; seq is *opaque* to clients;
  feed modes normal/longpoll/continuous/**eventsource** map 1:1 onto our
  poll/SSE endpoints; `include_docs` (payload embedding) is a client option;
  deletions appear in-band as changes flagged deleted. Lessons: expose the
  cursor as an opaque string, return `last_cursor` on every poll, make
  payload embedding optional, represent removals as events.
- **Postgres logical replication slots**
  (<https://www.postgresql.org/docs/current/logicaldecoding-explanation.html>):
  a slot is a named consumer position (LSN) stored server-side; delivery is
  at-least-once (position advances only on confirmation); WAL is retained
  until confirmed. Lessons: our *webhook endpoints are exactly slots* — a
  per-endpoint `last_delivered_cursor` in SQLite, advanced only on 2xx, gives
  at-least-once with no delivery queue table; and retention is a real design
  axis (a slot too far behind must be reset).
- **Firestore snapshot listeners**
  (<https://firebase.google.com/docs/firestore/query-data/listen>): first
  callback delivers the full matching set as `added` doc-changes, then
  incremental `added`/`modified`/`removed`; resume tokens can expire, forcing
  a fresh snapshot. This is precisely the product behavior CONTEXT.md asks
  for ("Firestore-like"), and it validates the `added/changed/removed` +
  explicit-reset vocabulary.
- **Turso/libsql sync**: pull-based, monotonically numbered WAL frames —
  same single-writer/monotonic-integer shape; details are the DB-side agent's
  scope, not load-bearing here.

### What our cursor should be (recommendation)

**One global integer**: the rowid of an append-only `changes` table (or
equivalently the canonical-version sequence id of ADR-0001), written **in the
same transaction** as the canonical-layer write it describes. Rationale:

- SQLite has exactly one writer at a time; a plain
  `INTEGER PRIMARY KEY AUTOINCREMENT` is already a total order — Lamport
  clocks / per-table cursors solve coordination problems we structurally do
  not have (ADR-0005: one process, one file). `AUTOINCREMENT` (not bare
  rowid) guarantees monotonicity across deletes/vacuum if the log is pruned.
- Per-table cursors would force clients to track N positions and us to answer
  "changed since" with N queries; a global cursor keeps SSE, poll, and
  webhooks on one number. Entity-filtered feeds are `WHERE` clauses over the
  same log, cursor unchanged.
- Same-transaction append means the feed can never announce a version that
  isn't durable, and every canonical version is announced exactly once.

Externally the cursor is serialized as a **string** (CouchDB lesson: clients
must not do arithmetic on it), internally it is `u64` (fits the SSE `id:`
field and `Last-Event-ID` round-trip).

Change-log row: `cursor, entity_kind, entity_id, canonical_version_id, op
(added|changed|removed), changed_at`. Payloads are *not* duplicated into the
log — the versioned canonical layer already stores every version; the event
payload is joined at read time (`include_data`-style option).

**Retention**: keep the log forever initially (it's thin — no payloads). If
pruned later, a resume below the horizon returns HTTP 410 / SSE `reset` event
→ client resnapshots (Firestore token-expiry semantics).

### Event schema (one JSON object for SSE `data:`, poll items, and webhooks)

```
// SSE:                                  // one change
event: change
id: 184467
data: {"cursor":"184467","op":"changed","entity":"tender",
       "id":"tender_01HZX…","version":"v_01HZY…",
       "changed_at":"2026-07-19T12:00:00Z",
       "data":{ …full canonical state, null when op=removed… }}

// SSE stream shape:
//   on fresh connect:  N × {op:"added"} snapshot events (id = snapshot cursor)
//                      → event: live  data: {"cursor":"184000"}
//                      → change events as they happen
//   on resume:         change events with cursor > Last-Event-ID
//   cursor expired:    event: reset   data: {"reason":"cursor_expired"}

// Poll:  GET /v1/changes?since=184000&limit=500[&entity=tender]
{"events":[ <same objects> ], "last_cursor":"184467", "more":false}

// Webhook POST body (batched):
{"delivery_id":"whd_01…","events":[ <same objects> ],"last_cursor":"184467"}
```

`version` ties every event to the ADR-0001 canonical version (and through it
to the causing Notice) — the traceability story reaches the API surface for
free. Confidence: high on cursor design; the exact field names are a
proposal.

---

## 4. Webhooks: minimal but correct

### Delivery semantics (recommendation, precedent-backed)

Model each registered endpoint as a **consumer slot** over the same change
log (§3): `last_delivered_cursor` advances only after a 2xx response ⇒
at-least-once, ordered, batched delivery with **no outbox/queue table** —
the change log is the queue. One tokio task per endpoint (or one sweeper task
scanning due endpoints — simpler, fine at v1 scale) driven by the same
`watch` doorbell as SSE.

- **Success** = 2xx within timeout; anything else (including redirects, per
  Standard Webhooks) is failure. Timeout ~10 s (GitHub's documented limit is
  10 s; Stripe uses longer — 10 s is the safe convention).
- **Retry**: exponential backoff with jitter; a practical schedule:
  30 s, 2 m, 10 m, 1 h, 4 h, 12 h, then daily — spanning multiple days, per
  the Standard Webhooks recommendation and Stripe's ~3-day retry window
  (<https://docs.stripe.com/webhooks>). Because retries just mean "don't
  advance the cursor", a recovering endpoint automatically receives
  everything it missed in the next batch — retrying delivers the *backlog*,
  not a stale payload.
- **Disable**: after N days of continuous failure (Stripe: disables after
  retries exhaust) set `disabled_at`; surface on the dashboard; re-enabling
  resumes from the stored cursor or "now" (user choice).

### Signatures (verified against specs)

Adopt **Standard Webhooks** (<https://github.com/standard-webhooks/standard-webhooks/blob/main/spec/standard-webhooks.md>) rather than inventing headers:

- Headers: `webhook-id`, `webhook-timestamp` (unix seconds),
  `webhook-signature: v1,<base64 HMAC-SHA256>`.
- Signed content: `{id}.{timestamp}.{payload}` — the timestamp in the MAC
  gives replay protection (GitHub's `X-Hub-Signature-256: sha256=<hex>` over
  the bare body does not;
  <https://docs.github.com/en/webhooks/using-webhooks/validating-webhook-deliveries>).
- Secret: 24–64 random bytes, shown to the user base64 with `whsec_` prefix.
- Consumers verify with constant-time comparison and a timestamp tolerance
  (~5 min, Stripe's default).

Pure-Rust implementation is trivial with `hmac` + `sha2` (RustCrypto; `sha2`
and `subtle` are already in the lock). The `standardwebhooks` reference crate
exists but is small enough to not be worth the dependency — implement the ~20
lines directly.

### HTTP client (verified in-tree)

`reqwest` is **already in our dependency tree** (pulled by dioxus-fullstack),
as are `rustls` and `ring`. So webhook delivery via
`reqwest` + rustls adds no new native-code exposure — though note `ring`
contains C/asm built via `cc` (no *system* libraries, builds fine in the Nix
flake, but it is not strictly pure Rust; it's already a fait accompli in the
tree). SSRF: webhook URLs are user-supplied — resolve and reject loopback /
RFC1918 / link-local targets before connecting, require https (or allow http
only for explicitly flagged endpoints).

### SQLite state (complete list)

```sql
webhook_endpoints(id, user_id, url, secret,            -- whsec_, stored encrypted-at-rest? see open questions
                  created_at, disabled_at,
                  last_delivered_cursor,               -- the "slot position"
                  failing_since, next_attempt_at, consecutive_failures)
```

Optionally `webhook_delivery_log(endpoint_id, delivered_at, cursor_from,
cursor_to, status, duration_ms, error)` ring-buffer for dashboard debugging
(prunable). Nothing else is needed. Confidence: high.

---

## 5. Accounts & auth, pure Rust

### Password hashing (verified)

**`argon2` 0.5.3** (RustCrypto: pure Rust, actively maintained — 0.6.0-rc.2
exists, stay on 0.5.x until 0.6 finalizes;
<https://docs.rs/argon2>). Defaults already match OWASP's recommended
Argon2id parameters (m=19 MiB, t=2, p=1;
<https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html>)
— use `Argon2::default()` + `PasswordHasher`/`PasswordVerifier` from
`password-hash`, store the PHC string (`$argon2id$v=19$m=19456,t=2,p=1$…`).
Run hash/verify inside `tokio::task::spawn_blocking` (deliberately ~50–100 ms
of CPU). Username+password only, per CONTEXT.md — no email machinery.

### API tokens (recommendation, precedent-backed)

**Random, not signed.** JWT/PASETO buy statelessness we don't need (single
process, and revocation would need the DB anyway):

- Generate 32 bytes from the OS RNG; encode base62/base64url with a
  recognizable prefix, GitHub-style: `tdb_<random>` (+ optional CRC32
  checksum for client-side typo detection) — the prefix enables secret
  scanning and support-ticket recognition
  (<https://github.blog/engineering/behind-githubs-new-authentication-token-formats/>).
- **Store only `SHA-256(token)`**; look up by hash. High-entropy input ⇒ fast
  hash is safe (this is what GitHub does); a DB leak leaks no usable tokens.
  Show the token once at creation. Comparing by exact-match on the hash also
  neutralizes timing concerns.
- Table: `api_tokens(id, user_id, token_hash UNIQUE, prefix_hint, name,
  created_at, last_used_at, revoked_at)`.

### Axum pattern for optional auth (recommendation)

Most endpoints are public, a few are gated — so use **extractors, not
route-blanket middleware**:

```rust
struct AuthUser(UserId);                 // FromRequestParts: Bearer token → hash → lookup; 401 on failure
// gated handler:    async fn register_webhook(user: AuthUser, …)
// optional-auth:    async fn query(user: Option<AuthUser>, …)   // Option<T> extractor never rejects
// dashboard:        session cookie checked in the same extractor (cookie OR bearer)
```

`FromRequestParts` composes with rate limiting (§6: key the limiter by the
extracted identity) and needs no `route_layer` bookkeeping about which routes
are public.

### Dashboard sessions vs API tokens (recommendation)

Separate the two credentials: browser dashboard uses a **session cookie**,
programmatic API uses **Bearer tokens**. Sessions: own table
(`sessions(id_hash, user_id, created_at, expires_at)`, id = 128-bit random,
hashed at rest like tokens) + cookie handling via **axum-extra 0.12.6**
(`cookie` feature; requires exactly `axum ^0.8.9` — matches our lock;
<https://docs.rs/axum-extra>). Cookie flags: `HttpOnly; Secure; SameSite=Lax;
Path=/`. A DB session table (vs signed stateless cookie) gives instant
logout/delete-account revocation and is trivial when you own the SQLite file.
Avoid `tower-sessions`' SQLite stores — they ride sqlx/libsqlite3-sys (C),
clashing with the turso choice. Confidence: high.

---

## 6. Rate limiting in-process

**`governor` 0.10.4** (<https://docs.rs/governor>) is the standard pure-Rust
GCRA limiter — actively maintained, no_std-capable, in-memory keyed state via
`DashMap` (`RateLimiter::keyed(quota)`), exactly right for a single-process
monolith. No redis/persistence: limits resetting on restart is acceptable.

Two tiers:

1. **Per-IP, unauthenticated endpoints**: **`tower_governor` 0.8.0**
   (axum ^0.8, governor ^0.10, tower ^0.5 — all match our lock;
   <https://docs.rs/tower_governor>) as a layer on the public sub-router,
   `PeerIpKeyExtractor` (or `SmartIpKeyExtractor` reading
   `X-Forwarded-For` **only if** deployed behind our own nginx). Maintenance
   note: single-maintainer, last release 2025-08 — but it's a thin wrapper;
   acceptable risk, and replaceable by ~50 lines over `governor` if it
   staleness ever bites.
2. **Per-token (SQL endpoint, webhooks registration)**: don't fight
   tower_governor's KeyExtractor (it runs before auth). Hold
   `RateLimiter<UserId, DashMapStateStore<…>>` in `AppState` and check it
   *inside* the `AuthUser`-extracting handlers → 429 with `Retry-After`.
   Add a **concurrency** cap for the SQL endpoint (per-user
   `tokio::sync::Semaphore`, e.g. 2 concurrent queries) — long-running
   queries are the real resource risk, not request rate.

Confidence: high; versions verified against crates.io dependency metadata.

---

## 7. Read-only SQL endpoint hardening (HTTP layer)

Defense-in-depth, complementing DB-level enforcement (read-only connection —
other agent's scope):

1. **Statement classification via `sqlparser` 0.62** (apache/datafusion-
   sqlparser-rs — very active, Apache-governed, pure Rust;
   <https://github.com/apache/datafusion-sqlparser-rs>): parse with
   `SQLiteDialect`, then require **exactly one** statement matching
   `Statement::Query(_)` (this single check rejects multi-statement
   payloads, DML/DDL, `PRAGMA`, `ATTACH`, `VACUUM`). Reject CTE-wrapped
   writes automatically (`WITH … INSERT` parses as non-Query). Caveat:
   sqlparser is a *generic* SQL parser — some valid SQLite `SELECT` syntax
   may fail to parse (false rejections, safe direction) and it must be the
   allow-list, never the only wall: never rely on it to *sanitize*, only to
   *reject*, with the read-only connection as the real enforcement.
2. **Timeout**: wrap execution in `tokio::time::timeout` (e.g. 5–10 s) and
   interrupt/drop the turso statement on expiry; the per-user semaphore (§6)
   bounds total exposure.
3. **Result caps, streaming**: stream rows out via `Body::from_stream`
   (serialize incrementally: `{"columns":[…],"rows":[[…],` …), counting rows
   and bytes; stop at caps (e.g. 10 000 rows / 10 MB) and close with
   `"truncated":true`. Cap-on-read beats rewriting the query to inject
   `LIMIT` (fragile against `UNION`/CTE shapes). Note: once streaming has
   started the HTTP status is already sent — mid-stream errors must be
   signaled in-band (trailer field in the JSON), which is another reason to
   keep the row cap modest.
4. **Request caps**: body size limit on the endpoint
   (`DefaultBodyLimit::max(64 * 1024)`), require `POST` with the SQL in the
   body (keeps queries out of access logs/URLs).

Confidence: high on approach; medium on sqlparser's SQLite-SELECT coverage —
worth a quick corpus test against the real schema's intended queries.

---

## Implications for tender-db

### Recommended stack (all verified compatible with the 0.7.9/0.8.9 lock)

| Crate | Version | Role | Notes |
|---|---|---|---|
| axum | 0.8.9 (in tree) | public API: REST, SSE, poll, SQL | latest stable; via dioxus-server |
| axum-extra | 0.12.6 | cookies (`cookie` feature) | requires axum ^0.8.9 ✓ |
| tokio `sync::watch` | 1.53 (in tree) | change-cursor doorbell for SSE + webhook tasks | |
| governor | 0.10.4 | keyed per-user limits + semaphores | pure Rust |
| tower_governor | 0.8.0 | per-IP layer on public routes | thin, replaceable |
| argon2 (+password-hash) | 0.5.3 | password hashing, OWASP defaults | RustCrypto, pure Rust |
| hmac + sha2 + subtle | current (sha2/subtle in tree) | webhook signatures, token hashing | RustCrypto |
| rand/getrandom | in tree | token/secret generation | |
| sqlparser | 0.62.0 | SQL endpoint allow-list | SQLiteDialect |
| reqwest (rustls) | in tree via dioxus-fullstack | webhook delivery | ring is already in tree |
| utoipa + utoipa-axum | 5.5.0 / 0.2.0 | OpenAPI (optional, v1-nice-to-have) | axum ^0.8 ✓ |

Architecture decisions this research supports:

- Public API = plain axum `Router<AppState>` merged into
  `dioxus::server::router(App)` inside `dioxus::server::serve`; server
  functions reserved for the dashboard; namespaces: `/v1/…` public,
  dashboard server-fns moved off `/api/…` to `/_dash/…`.
- **Cursor**: one global `u64` = rowid of an append-only `changes` table
  written transactionally with canonical-version writes; opaque string in
  the API; it is the SSE `id:`, the `since=` poll param, and each webhook
  endpoint's stored position.
- **SSE**: subscribe-watch → snapshot-in-one-read-txn (capture N) →
  `added…` + `live` marker → diff loop `cursor > last_sent`; keep-alive 15 s;
  `X-Accel-Buffering: no`; `reset` event when a resume cursor is below
  retention.
- **Webhooks** = server-driven consumers of the same log (Postgres-slot
  model): per-endpoint cursor, batch POST, Standard-Webhooks signature
  headers, 10 s timeout, backoff 30 s→daily, disable after ~N days failing.
- **Auth**: argon2id PHC strings; `tdb_`-prefixed random tokens stored as
  SHA-256; `AuthUser`/`Option<AuthUser>` extractors; DB-backed session
  cookie for the dashboard.
- **Event schema** as in §3 — one JSON object shared by SSE, poll, webhooks;
  `version` field links every event to its canonical version → Notice.

### Open questions

Needs more research:
1. ~~**turso statement interruption**~~ **Resolved** by
   docs/research/turso-capabilities.md: timeout-by-drop
   (`tokio::time::timeout` + dropping the future) is verified effective on
   file-backed databases; no `interrupt()` needed.
2. ~~**sqlparser vs SQLite SELECT dialect coverage**~~ **Superseded**: the
   allow-list should use `turso_parser` (turso's own parser — exactly the
   engine's dialect, no false rejections by construction), per
   docs/research/turso-capabilities.md. sqlparser remains a fallback option
   only.
3. **Snapshot semantics for filtered SSE feeds** — full-collection snapshots
   on `/v1/tenders/live` could be large; probably snapshot only the
   query-matching set (Firestore-style) — needs the query-parameter design
   first.
4. **utoipa in v1 or later** — costs annotation discipline on every handler;
   defer until the resource shapes stabilize?

Needs a user decision:
1. **Webhook secret storage**: secrets must be *usable* (we sign with them),
   so they can't be hashed like passwords — store plaintext in SQLite
   (simplest; DB-file compromise = re-issue secrets) or encrypted with a
   process-level key (key management for one box buys little)?
2. **SSRF policy** for webhook URLs: block private ranges + require https —
   strictness vs "test against localhost during development" convenience.
3. **Anonymous SSE**: are live streams available without an account (matches
   "basic query endpoints are unauthenticated"), and with what per-IP
   connection cap?
4. **Retention promise** for the change log: "forever" is cheap now but
   becomes an API contract; commit or explicitly reserve the right to prune
   (410/`reset` path exists either way).
5. **Rate-limit numbers** (per-IP rps, SQL per-user quota, concurrency cap)
   — need product judgment, not research.
