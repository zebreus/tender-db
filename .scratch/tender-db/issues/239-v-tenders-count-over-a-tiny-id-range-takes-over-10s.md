# 239 — counting 5,000 ids of `v_tenders` takes over 10 seconds

Status: CAUSE FOUND 2026-08-18 — turso pushes no predicate into ANY view, so the whole `v_*` analyst
surface is unusable for filtered queries (a single-table view is 1000x slower than its table). The
`current_title` denormalisation shipped and helps unfiltered reads, but is NOT the fix
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

## THE REAL CAUSE (2026-08-18, measured after deploying the wrong fix): turso does not push predicates into views AT ALL

The `current_title` work below was built on a hypothesis I never tested, and it did **not** fix the
symptom. After deploying it and backfilling 7,924,659 rows, the original query was still >10 s. The
measurements that should have come first:

    SELECT current_seq FROM tenders          WHERE id = 93601         0.001 s   ← raw table, warm
    SELECT seq FROM v_tender_current         WHERE tender_id = 93601  1.41 s    ← SINGLE-TABLE view
    (repeated, warm)                                                  1.41 s, 1.51 s — not a cache effect
    raw 2-table join, same PK filter                                  0.017 s   ← the join is fine
    SELECT id, title FROM v_tenders          WHERE id = 93601         >10 s     ← same join, in a view
    SELECT id FROM v_lots                    WHERE tender_id = 93601  >10 s     ← and v_lots too

**A single-table view over `tenders` filtered by primary key is ~1000× slower than the same filter on
the table.** So the predicate is not pushed into a view, period — not for joins, not for subqueries, not
for a trivial `SELECT … FROM tenders WHERE current_seq IS NOT NULL`. Every `v_*` view scans its whole
source and filters afterwards, and the only reason the numbers differ is how expensive each view's full
scan is (a narrow scan of `tenders` costs 1.4 s; the joined one blows the budget).

### Consequence: the whole `v_*` analyst surface is unusable for filtered queries

Not one view, and not aggregates only. `/v1/sql` allow-lists both the views and the base tables, and the
base tables plan correctly — the raw two-table join with a PK filter is 17 ms. So the curated surface is
the slow path and the raw tables are the fast one, which is precisely backwards from what the views were
introduced to do (issue 50).

### Two wrong hypotheses, and the cheap test that would have killed both

1. "The count pays ~5,000 correlated subqueries" — wrong by 1000×; the filter did nothing at all.
2. "The correlated subquery blocks view flattening, so denormalise the title" — plausible, precedented,
   tested by nobody. The subquery was real per-row cost, but it was never why the filter failed.

The discriminating experiment was **two queries**: run the view's own join raw, then in the view. 17 ms
vs >10 s answers it immediately, needs no code, and I ran it only after writing a schema migration, a
fold change, a backfill job, 150 lines of tests, a deploy, and a 7.9M-row write. The lesson is not
"hypothesise better" — it is that when a cheap discriminating measurement exists, it comes before the
code, not after it.

### What the `current_title` work is actually worth

Not wasted, and not a fix. It removed a genuine per-row cost from the view (a subquery with a sort on two
computed expressions, evaluated for every row of every full scan), so unfiltered reads improved sharply —
`SELECT title FROM v_tenders LIMIT 1` is 2 ms and returns a correct title ("Ladekabel eAutos"), and the
GROUP BY examples may now be feasible where before they could not be. It is a correct, tested
denormalisation that this schema wanted anyway. It just does not address filtering, and the commit
message claiming the view now "flattens" is wrong — corrected here rather than rewritten.

### Fix directions, now that the cause is known

- **Point analysts at the base tables** and say plainly that the `v_*` views cannot be filtered. Cheapest
  honest step, deployable today: the endpoint's own schema descriptions (`sql.rs:548`) currently
  advertise `v_tenders` as "the usual entry point", which is the opposite of true.
- **Materialise** the views as real fold-maintained tables. Removes the problem entirely, costs write
  amplification and schema surface; the `current_*` columns are already halfway there.
- **Upstream**: predicate pushdown into views is ordinary SQLite behaviour, so this is a turso gap worth
  reporting and pinning with a version note.
- Do NOT reach for "make the views single-table" — measured above, it does not help.

## Superseded: FIXED in code and deployed (2026-08-18, rev `f985136`) — backfill running

`v_tenders.title` now reads `tenders.current_title`, a fold-maintained column, instead of computing a
correlated subquery per row. The view is a plain two-table join, so it flattens, so a caller's filter
pushes down.

- `head_title` (canonical.rs) applies the precedence when the fold writes a head pointer — the same
  place `current_seq` / `current_published_at` / `current_deadline` are written (issues 25, 216). The
  pattern already existed; this view was just not using it.
- `backfill_current_title` + the `backfill-titles` job stamp the 7.9M existing rows, batched and
  checkpointed like `backfill-deadlines`, restartable from a watermark, idempotent. **Running now.**
- Both writers are pinned by tests, on the cases where precedence decides: Tender-title over
  lot-title, ENG over another language, a title on a NON-head version losing, and no title staying
  NULL. The backfill's SQL is deliberately the old subquery verbatim, so "same answer, computed once"
  can be checked by reading it.

Deploy order was deliberate: the column is NULL until the backfill finishes, and only `/v1/sql` reads
this view (the REST endpoints do not), so the window costs absent titles on a surface that could not
answer a filtered query at all. Acceptance checks to run once the backfill lands are at the bottom of
this issue.

**One more thing the investigation turned up:** `/v1/sql`'s own documented example queries
(`sql.rs:712`) are `GROUP BY` aggregates over `v_tenders`. On the old view those could not have
completed inside the 10 s budget either — the endpoint was shipping examples it could not run.

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

- ~~The plan is recorded here~~ — superseded by the measurement above, which localised the cause without
  needing a plan: cost independent of the filter is conclusive on its own.
- ~~Either the view is fixed…~~ **DONE in code** (`f985136`); verify on prod once the backfill lands:
  - `SELECT id, title FROM v_tenders WHERE id = 93601` returns in milliseconds, with a non-NULL title.
  - `SELECT COUNT(*) FROM v_tenders WHERE id < 5000` completes well inside the budget.
  - `SELECT source, count(*) FROM v_tenders GROUP BY source` — the endpoint's own documented example —
    completes at all.
  - Spot-check a handful of titles against `tender_version_texts` for the same `(tender_id, seq)`, since
    the whole change rests on the two writers agreeing.
- **Still open: `v_lots` and `v_organizations` carry the same correlated-subquery shape** and were never
  measured. `v_organizations`' subquery is a `COUNT(*)` over `organization_mentions` (40.9M rows) per
  row, which is a worse shape than the one just fixed. Measure both before assuming this issue is closed
  — and mind the outage risk noted above when doing it.
- A regression probe alongside the existing `crates/store/tests/*_probe.rs` cost tests, which exist for
  exactly this class.
