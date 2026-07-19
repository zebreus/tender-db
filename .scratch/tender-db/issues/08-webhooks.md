# 08 — Minimal webhooks

Status: resolved
Blocked by: 06

Goal: account holders register URLs that receive change events with
at-least-once delivery.

Scope:
- `store`: webhook_endpoints (user, url, secret plaintext, cursor position,
  failure state) (+ small delivery log ring for the dashboard).
- `app`: consumer-slot delivery task (batch POST of the shared event JSON,
  Standard-Webhooks signature headers, 10s timeout, backoff 30s→daily,
  disable after ~3 days failing); registration/management on the dashboard
  and `/v1/webhooks` (token-auth); SSRF guard: https-only, resolved address
  must be public (config flag relaxes for dev).
- Tests: delivery + retry + signature verification against a local receiver;
  SSRF guard unit tests.

Acceptance: a locally registered endpoint receives signed batches for new
changes; failure→retry→disable path exercised in tests.

## Answer

Delivered: webhooks end to end — store tables + accessors
(`crates/store/src/webhooks.rs`), the delivery engine
(`crates/app/src/webhooks.rs`: signing + SSRF guard + background sweeper +
CRUD), the `/v1/webhooks` REST surface (`crates/app/src/v1/webhooks.rs`), and a
dashboard section (`api.rs` server fns + `ui.rs` panel). Model types in
`crates/model/src/webhook.rs`.

**Consumer-slot model (no outbox).** Each endpoint's `last_delivered_cursor`
advances only after a 2xx, so the change log itself is the queue: a recovered
endpoint receives the whole backlog in its next batch, never a stale single
payload. One background **sweeper** task, woken by the same cursor doorbell the
SSE uses plus a 15 s timer (so a backed-off endpoint gets retried even with no
new changes), walks the due endpoints and delivers in batches of 500 (up to 20
batches/endpoint/sweep so one big backlog can't stall the others). It reads the
log through a dedicated 2-connection reader pool, off the writer path.

**Signatures — Standard Webhooks.** `webhook-id`, `webhook-timestamp`,
`webhook-signature: v1,<base64 HMAC-SHA256>` over `{id}.{timestamp}.{payload}`
(hmac + the in-tree sha2). Secret is `whsec_<base64(24 OS-random bytes)>`, shown
once at creation, stored plaintext by decision (the server must present it on
every delivery to compute the MAC, so a hash is not an option — one-box threat
model). The signing implementation is checked against an independent HMAC
computation, and the integration test verifies delivered signatures the way a
consumer would (recomputing the MAC, not via the producing code).

**Retry/disable.** 2xx (not redirects — the delivery client has redirect
following off) within a 10 s timeout is success. Failure holds the cursor and
schedules backoff 30 s → 2 m → 10 m → 1 h → 4 h → 12 h → daily; after a
continuous-failure streak exceeding ~3 days the endpoint auto-disables. A
disabled endpoint drops off the sweeper's list; re-enabling clears the streak
and either keeps the slot (deliver the backlog) or jumps to the head (skip it).

**SSRF guard.** `vet_url` requires `https` and resolves the host, rejecting any
result that is loopback / RFC1918 / link-local / CGNAT-shared / v4-mapped-private
/ IPv6 ULA / otherwise non-global. `TENDER_WEBHOOK_ALLOW_INSECURE=1` relaxes
scheme + address for local dev. Documented residual: a DNS-rebinding TOCTOU
window between the check and reqwest's own resolution — closing it fully needs
connection-pinning to the vetted IP; the guard already rejects the overwhelming
majority and the one-box holds nothing an internal request could usefully reach.

**Auth & scoping.** `/v1/webhooks` CRUD is gated by issue 06's `AuthUser`
extractor; every query is scoped by user id, so one account can never see or
touch another's endpoints (asserted in the store tests). Dashboard management
goes through session-cookie server functions.

**Tests (7 store + 6 app unit + 3 integration, all green):** store round-trips
(CRUD + owner scoping, success advances / failure holds the slot, disable/enable,
bounded delivery-log ring); unit (the Standard-Webhooks signature vector,
secret round-trip, the private/loopback/ULA/mapped IP matrix, `vet_url` scheme +
literal-loopback rejection, backoff schedule); integration against a real local
receiver over a socket (a signed batch delivered + slot advanced + no
re-delivery of an acked batch; a 5xx holds the cursor, backs off, then a
recovery delivers the whole backlog; a >3-day streak disables the endpoint).

**Files:** new `crates/store/src/webhooks.rs`, `crates/model/src/webhook.rs`,
`crates/app/src/webhooks.rs`, `crates/app/src/v1/webhooks.rs`,
`crates/app/tests/webhooks.rs`; edits to `store/src/lib.rs` (+schema, re-export)
& `accounts.rs` (a shared `random_bytes` helper), `model/src/lib.rs`,
`app/src/{lib,main,api,ui}.rs`, `app/src/v1/mod.rs`, and `Cargo.toml` +
`app/Cargo.toml` (hmac, base64, url). Nothing in `crates/ingest`.
