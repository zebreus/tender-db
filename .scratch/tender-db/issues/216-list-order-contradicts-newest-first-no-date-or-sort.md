# 216 — /v1/tenders documents "newest matching first" but sorts ascending id, and offers no date-range or sort controls

Status: PART A FIXED (docs reconciled) 2026-08-16 — PART B (date-range + sort capability) OPEN. Part A, the
doc-vs-behavior lie, is fixed: the three "newest matching first" surfaces (`docs.rs:122`, `docs.rs:407`,
`openapi.json:65`) now state the real order — **ascending id on every collection** (a stable keyset order for
pagination, not by date). The query is unchanged (`ORDER BY t.id`, read.rs:956); shipping the honest doc now
stops actively misleading clients, as the issue recommended, ahead of the sort capability. Test:
`pagination_walks_the_whole_collection_exactly_once` now asserts the list is served in ascending id order,
so a future descending change (Part B) must move the docs with it.

Part B — `published_after`/`published_before`, `deadline_before`/`deadline_after`, and `sort`/`order` with a
keyset cursor carrying the sort key — remains OPEN. It needs a date index on `tender_version_dates`
(coordinate with the deferred-index builder, issue 111) and isolation-classifier routing (issue 120) so a
bare date range does not become an unbounded walk — a real capability project, related to 217's
publication_id lookup (same date/index infrastructure theme).

Original: needs-triage — HIGH, CONFIRMED (code) 2026-08-15. Filed from the API completeness review (subagent).
Kind: correctness (doc-vs-behavior) + completeness (missing query capability)
Blocked by: —
Relates to: 117 (keyset pagination — the fixed `id` order this is built on), 215 (contract-drift cluster),
118 (accepted-but-ignored filters)

## Two problems, one root (the list's ordering vocabulary)

### A. The documented order is the opposite of the delivered order (correctness)

Three published surfaces promise newest-first:

- `crates/app/src/v1/docs.rs:122` — "Tenders (current version of each), **newest matching first**."
- `crates/app/src/v1/docs.rs:407` — "Sort order is fixed (**newest-first for tenders**, id for the rest)."
- `crates/app/data/openapi.json:65` — "The current version of each Tender, **newest matching first**."

But `tenders_query` paginates `… AND t.id > ? ORDER BY t.id LIMIT ?` (`crates/store/src/read.rs:908`) —
**ascending `t.id`**. Page 1 is the *lowest* ids; the newest tenders are on the last page of millions.
Ascending `t.id` is neither "newest" by publication nor by ingest. A client that trusts the documented
order and reads page 1 gets the oldest rows.

### B. No date-range filter and no sort/order control (completeness)

`Params` (`crates/app/src/v1/mod.rs:452-472`) is the whole query vocabulary, and it has `deny_unknown_fields`
(`:451`) — so anything not listed is a hard **400**, not a silent ignore. It offers no `published_after`,
`published_before`, `deadline_before`, `deadline_after`, `sort`, or `order`. The only date-adjacent control
is the binary `status=open|closed` (deadline vs. now, read.rs). Yet `published_at` and the
`submission_deadline` satellite are returned on every row (json.rs:66,71) — filterable/sortable by nothing.

## Why it matters

For a live procurement dataset the flagship queries are "what was published recently" and "what closes
soon." Both are unreachable:

- "Tenders with a submission deadline in the next 7 days" — impossible; `status=open` returns everything
  not-yet-closed, unbounded and unordered.
- "The 50 most recently published tenders" — impossible without paging the entire corpus from the lowest
  id upward.

## Fix direction

1. Add `published_after`/`published_before` and `deadline_before`/`deadline_after` range params, seeking the
   `tender_version_dates` satellite (indexed on `(tender_id, seq)`; a date-range filter wants its own index
   — coordinate with the deferred-index set and the isolation classifier so a bare date range doesn't become
   an unbounded walk).
2. Add `sort=published_at|deadline|id` + `order=asc|desc`; keep keyset pagination by making the cursor
   carry the sort key + id tiebreak (so `ORDER BY published_at DESC, id DESC` still paginates without OFFSET).
3. Until sort exists, at minimum reconcile A: either implement descending order for tenders or correct all
   three "newest-first" docs to state the real order. Shipping the doc fix now (cheap) stops actively
   misleading clients even before the capability lands.

## Verification

- `GET /v1/tenders?limit=5` first page matches whatever the docs now claim (newest, or explicitly "oldest
  by id").
- `GET /v1/tenders?deadline_before=<iso>&status=open` returns only tenders closing before that instant.
- `GET /v1/tenders?sort=published_at&order=desc` returns most-recent first and paginates cleanly to the end.
- An unknown param still 400s (deny_unknown_fields preserved).
