# 51 — Uniform JSON error envelope; 408 + cancel-on-disconnect for SQL

Status: needs-verification
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

## Progress (api-polish, 2026-07-23)
Part 1 — uniform envelope (mod.rs):
- `ApiQuery`/`ApiPath` extractors map axum's plain-text rejections to the
  `{"error":{…}}` shape; the path message no longer leaks the Rust type `i64`.
- The tower_governor 429 is re-dressed as the envelope via `.error_handler`
  (with `Retry-After`); the SQL per-token 429 was already JSON.
- A `/v1/{*rest}` catch-all answers unknown subpaths with a JSON 404 instead of
  falling through to the dashboard's HTML router.
Tests: `unknown_v1_paths_are_json_404`, `malformed_inputs_return_the_json_error_envelope`,
`the_rate_limit_429_is_our_json_envelope`.

Part 2 — SQL cap + abandon (sql.rs):
- A handler-side backstop timeout runs on the MAIN runtime (the heavy work is on
  the isolated one), so it fires on time even for a non-yielding aggregate that
  the in-task timeout can't catch → 408 at the cap, and the permit drops so the
  concurrency slot frees at once. Bounds slot-hold to the cap (disconnected or
  not) instead of 40s+.
- `AbortOnDrop` abandons the isolated task the moment the handler stops waiting
  (backstop firing or the request future being dropped).
Test: `a_non_yielding_aggregate_is_capped` (via a short-timeout server seam,
`AppState::with_sql_timeout`).

Honest limitation: axum/hyper does not reliably cancel a *unary* handler future
mid-execution on client disconnect, so "frees promptly" is delivered as "freed
within the cap" (≤ ~11s), a large improvement over 40s+. If hyper *does*
propagate the disconnect (dropping the future), `AbortOnDrop` cancels the task
at once. turso still exposes no `interrupt()`, so a non-yielding aggregate runs
to completion abandoned on the isolated runtime — issue 17's isolation contains
that; the client already got its 408 and the slot is free.
