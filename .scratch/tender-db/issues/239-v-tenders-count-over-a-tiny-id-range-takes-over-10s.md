# 239 — counting 5,000 ids of `v_tenders` takes over 10 seconds

Status: needs-triage — measured 2026-08-18 on prod, twice, from the analyst surface
Kind: read-path cost (the headline analyst view is not usable for aggregates)
Blocked by: —
Relates to: 50 (the analyst surface), 238 (found while verifying its fix; strengthens its
admission-check recommendation), 116 (v_tenders' lot join), 225 (a previous covering-index fix for a
different shape on the same tables)

## What

    SELECT COUNT(*) FROM v_tenders WHERE id < 5000     →  HTTP 408, >10 s, twice

Reached for as an obviously-cheap probe while verifying issue 238. It is not cheap. An id range of
5,000 out of ~7.9M Tenders — 0.06 % of the table — cannot be counted inside the endpoint's 10 s budget.

## Why this matters more than a slow query

`v_tenders` is the analyst surface's headline view: `/v1/sql`'s allow-list exposes the `v_*` views
precisely so analysts write against a stable, curated shape instead of raw satellites (issue 50). If a
narrow-range COUNT over it cannot finish, then essentially no aggregate over it can, and the surface is
advertised as something it does not do.

It also burned two worker threads for 11 s each in the very run that verified issue 238's shed, which is
the practical harm: an unremarkable-looking query is enough to saturate the SQL runtime. Any admission
check built for 238 has to catch this shape, and any capacity reasoning that assumes "cheap queries are
cheap" is wrong here.

## Where to look first

- **`EXPLAIN QUERY PLAN` for the statement** — the cheapest possible next step, and it names the culprit
  directly. Do this before theorising.
- The view's definition: how many joins/subqueries it materialises per row, and whether `id < ?` is
  pushed down to the driving table or applied after materialising. A predicate that cannot be pushed
  through the view is the classic cause of exactly this symptom.
- Whether COUNT is the problem or the view is: compare
  `SELECT COUNT(*) FROM tenders WHERE id < 5000` (should be an index range on the PK) with the view
  version. That one comparison splits "the view is expensive" from "counting is expensive".

## Acceptance

- The plan is recorded here, with the specific step that dominates.
- Either the view is fixed/indexed so a narrow-range aggregate completes well inside the budget, or the
  limitation is documented on the analyst surface — an advertised view that cannot be aggregated should
  say so rather than time out.
- A regression probe alongside the existing `crates/store/tests/*_probe.rs` cost tests, which exist for
  exactly this class.
