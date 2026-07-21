# 51 — Uniform JSON error envelope; 408 + cancel-on-disconnect for SQL

Status: ready-for-agent
Severity: MEDIUM (robustness; one part has a real resource-leak edge)

Found by usability audit (2026-07-21). Two related robustness gaps:

1. **Inconsistent error responses.** The documented `{"error":{...}}`
   JSON envelope is honored for validation (`status=maybe` → 400 JSON)
   but NOT for: query-string deserialize errors (plain text), path parse
   (`/v1/tenders/abc` → plain text leaking the Rust type `i64`), SQL 429
   (plain text "Too Many Requests! Wait for 1s"), and unknown `/v1/*`
   subpaths (fall through to the HTML dashboard router, leaking route
   names). Make every /v1 error a JSON envelope; never leak Rust types or
   internal routes.
2. **SQL timeout doesn't fire on non-streaming aggregates, and a
   disconnected query keeps running.** The 10s cap only fires at row
   boundaries (sql.rs comment admits it); a `COUNT(*)`/GROUP-BY over
   `notices` runs >40s with no 408, and — worse — a client that
   disconnects leaves the query running server-side, holding one of the
   2 concurrent SQL slots, so the next query blocks/429s (~15s). This
   partially defeats issue 17's isolation. Fix: return 408 at the cap for
   aggregates too (even if it means bounding the statement differently),
   and cancel/abandon the query when the client disconnects so the slot
   frees immediately.

Acceptance: every /v1 error is JSON with no internal leakage; an
expensive SQL query returns 408 at the cap; a disconnected client frees
its SQL slot promptly.
