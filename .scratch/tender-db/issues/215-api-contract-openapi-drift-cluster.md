# 215 — API contract / OpenAPI drift cluster (limit ceiling, include_data on notices, changes `more`, lots absent-value routing)

Status: needs-triage — LOW, CONFIRMED (code) 2026-08-15. Filed from the API review (subagent).
Four small, independent drifts between documented and actual behavior. Grouped because each is a
one-line-ish fix and none alone warrants its own ticket; split out if one grows.
Kind: correctness / API contract + docs drift
Blocked by: —
Relates to: 117 (cursor/pagination), 118 (list filters), 47 (documented-vs-observed), 120 (isolation
routing the last item touches)

## A. `limit` ceiling: docs say max 500, server clamps to 1000

- `crates/store/src/read.rs:1613` `pub const MAX_PAGE: i64 = 1000;`, applied at
  `crates/app/src/v1/mod.rs:509` (`.clamp(1, read::MAX_PAGE)`).
- Docs say 500: `crates/app/data/openapi.json:701` (`"maximum": 500`), `crates/app/src/v1/docs.rs:153`
  ("default 100, max 500").
- Effect: a spec-generated validator rejects `limit=800` the server accepts; page-cost assumptions
  (which scale isolated-pool walk cost) understated 2×.
- Fix: make one authoritative — lower `MAX_PAGE` to 500, or set the spec/docs `maximum` to 1000.

## B. `include_data` undocumented on `/v1/notices`

- Declared for tenders/lots/organizations (`openapi.json:79,140,174`) but not notices
  (`openapi.json:220-226`), though the notices SSE snapshot honors it (`sse.rs:213-221`).
- Fix: add the `include_data` query param to the `/v1/notices` spec entry (and `/docs`), or confirm and
  document that notices deliberately omit it.

## C. `/v1/changes` reports `more:true` on an exactly-full final page

- `crates/app/src/v1/mod.rs:793` sets `more = rows.len() == limit` — no `limit+1` look-ahead probe,
  unlike the list path (`mod.rs:657`/`675`). A final page of exactly `limit` rows reports `more:true`,
  costing the client one extra empty poll. No data loss; the cursor round-trips correctly.
- Fix: fetch `limit+1`, set `more` from the overflow, truncate to `limit` — mirror the list handler.

## D. Absent-value short-circuit is inconsistent across collections → SUPERSEDED BY 219

> **Folded into issue 219** (2026-08-15). The performance review found the same absent-value-guard
> inconsistency has a second, worse hole — `/v1/tenders?kind=<absent>` also skips the short-circuit and
> is an *unauthenticated* isolated-pool saturation vector — so the whole guard-consistency problem is now
> tracked canonically in 219 (which absorbs this lots half). Left here as a pointer; fix in 219. The
> `/docs` performance-table pool-column drift noted below stays a 215 doc nit.

- `reachable()` short-circuits an absent filter value to an empty page in `tenders()`
  (`read.rs:808`) but is not applied in `lots()` (which only guards absent `kind`,
  `read.rs:1238-1248`). So `/v1/lots?buyer=<absent>` / `?country=<absent>` run a full isolated walk.
- Docs claim the short-circuit generically ("an absent filter value short-circuits to an empty page",
  `docs.rs:395,401`) and label absent-value filters as "main" pool — but on tenders, country/buyer route
  to the **isolated** pool (`walks` is true), so the docs' pool column is also off for that row. *(This
  doc-column drift remains a 215 item; the routing/guard fix moves to 219.)*

## Verification

- A: `GET /v1/tenders?limit=1000` behavior matches whatever the spec now says (accepted or clamped to
  the documented max).
- B: notices spec entry lists `include_data`; a spec-driven client can request it.
- C: a `/v1/changes` page of exactly `limit` rows at the end of the feed returns `more:false`.
- D: `GET /v1/lots?buyer=<absent>` returns an empty page without a full walk; the `/docs` table's pool
  labels match `walks()`.
