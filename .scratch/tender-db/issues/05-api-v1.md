# 05 — Public API v1: REST + changes + SSE

Status: ready-for-agent
Blocked by: 04

Goal: the read side goes live in-process: `/v1` REST over the canonical
layer, the changes poll endpoint, and SSE snapshot+diff with resume.

Scope:
- `app`: axum sub-router merged beside dioxus (`/v1`), reader-connection
  pool from `store`.
- `GET /v1/tenders` (+`/{id}`), `/v1/lots`, `/v1/organizations`,
  `/v1/notices` — cursor-paginated JSON, filters: source, country, CPV
  prefix, buyer, status, value range, kind. `GET /v1/changes?since=`.
- SSE on collection endpoints via `Accept: text/event-stream`: watch
  doorbell, snapshot-in-read-tx (capture N) → added events → live marker →
  diff loop with per-subscription (old,new) predicate evaluation
  (docs/architecture.md); Last-Event-ID resume; reset event; keep-alive
  15s; X-Accel-Buffering: no; 5 streams/IP cap.
- Per-IP rate limiting (tower_governor) on `/v1`.
- Integration test: in-process server over a fixture-ingested DB; REST
  shapes, SSE snapshot/diff/resume sequences asserted.
- Update the VM smoke test to assert `/v1/tenders` answers.

Acceptance: integration tests green; manual `curl` against a locally
ingested day shows tenders, changes, and a live SSE diff when a new notice
is ingested mid-stream.
