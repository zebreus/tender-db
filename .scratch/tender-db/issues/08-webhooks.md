# 08 — Minimal webhooks

Status: ready-for-agent
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
