# 239 — counting 5,000 ids of `v_tenders` takes over 10 seconds

Status: needs-triage, RAISED — `v_tenders` cannot serve even a primary-key point read (measured
2026-08-18); the analyst surface's headline view is unusable for any filtered query
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

## MEASURED (2026-08-18): it is far worse than a slow aggregate, and my estimate was wrong

Three measurements settle it, and correct the hypothesis below:

    SELECT COUNT(*) FROM tenders    WHERE id < 5000   →  200, 0.043 s
    SELECT COUNT(*) FROM v_tenders  WHERE id < 5000   →  408, >10 s
    SELECT COUNT(*) FROM v_tenders  WHERE id < 100    →  408, >10 s   ← same cost, 50× smaller range
    SELECT id, title FROM v_tenders WHERE id = 93601  →  408, >10 s   ← a PRIMARY KEY point read

**The cost does not depend on the filter at all.** So the predicate is never pushed into the view's
driving table: `v_tenders` materialises essentially the whole corpus and filters afterwards, whether the
caller asked for 7.9M rows, 100, or one.

That makes my estimate below — "the count pays ~5,000 correlated subqueries" — **wrong by three orders of
magnitude**. It pays ~7.9M. I reasoned about the per-row cost and never questioned whether the row COUNT
would be what the caller asked for. The one measurement that exposed it (shrink the range and see if the
cost moves) took ten seconds and should have come first.

**And the headline finding is not the aggregate.** `WHERE id = <pk>` also takes >10 s, so `v_tenders`
cannot serve a point read by primary key. The analyst surface's flagship view is unusable for ANY
filtered query, not merely for aggregates — which is a much bigger claim than this issue's title, and
the title now undersells it.

### Why (best supported explanation)

A correlated subquery in a view's SELECT list is the classic blocker to view flattening: the planner
cannot merge the view into the outer query, so it cannot push `id = ?` down, so it evaluates the view
in full. The same shape appears in `v_lots` and `v_organizations` (a per-row `COUNT(*)` over
`organization_mentions`), which should be measured next — if they share the defect, this is a
surface-wide problem rather than one view's.

Not yet distinguished: whether the planner drives from `tenders` or from `tender_versions` (14.15M rows).
Either way it does not seek, so the fix is the same; the distinction only changes the size of the number.

### Note for whoever measures next

Each of these 11 s queries **keeps computing after the endpoint abandons it** (turso has no
`interrupt()`), so two probes in a row pin both SQL workers and the endpoint sheds everything for minutes
— issue 238. Measuring this view is itself an outage risk: leave a gap between probes, and check
`SELECT 1` first.

## Leading hypothesis, from the view definition (read locally, no prod access needed)

`v_tenders` computes its `title` column with a **correlated subquery that also sorts**, once per row
(`canonical.rs:532`):

    (SELECT x.value FROM tender_version_texts x
      WHERE x.tender_id = t.id AND x.seq = v.seq AND x.field = 'title'
      ORDER BY (x.lot_id IS NULL) DESC, (x.lang = 'ENG') DESC
      LIMIT 1) AS title

So every row the view yields costs a seek into `tender_version_texts` plus a sort over the matches on two
COMPUTED expressions (which no index can serve). For `COUNT(*)` the column is never read, and an
optimiser that proved the subquery unnecessary would drop it — evidently this one does not, so the count
pays ~5,000 correlated subqueries-with-sort. That is the right order of magnitude for >10 s.

**Test it in one step before believing it:** `SELECT COUNT(*) FROM tenders WHERE id < 5000` (a PK range)
against the view version. If the bare table is instant, the subquery is confirmed as the cost and the
`EXPLAIN QUERY PLAN` is confirmation rather than discovery.

## If confirmed, the fix has a precedent in this schema

`tenders` already carries fold-maintained denormalised head pointers for exactly this reason —
`current_seq`, `current_published_at`, `current_deadline` (issues 25 and 216), each added because a
per-row lookup was too expensive on a listing path. A `current_title` maintained the same way would
remove the subquery from the view entirely, and the fold already knows the title when it writes the
version.

That is a schema change plus a backfill, so it wants sizing — but it is a known pattern here, not a new
idea, and it would also speed every non-aggregate read of the view.

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
