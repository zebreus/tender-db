# 230 — the data-quality report times out on every query at full-corpus scale, and reports empty instead of saying so loudly

Status: needs-triage — measured 2026-08-18 against prod (rev `cc0ef20`)
Kind: observability / tooling rot (a green that stopped being green without anyone noticing)
Blocked by: —
Relates to: 27 (the report this is, whose acceptance no longer holds), 120 (the app cannot bound an
expensive query — the same wall from the other side), 167 (capacity/abuse model), 226 + 228 (the same
lesson: a signal that cannot distinguish two states is not a signal)

## What

`crates/ingest/src/bin/data-quality` — the only *semantic* completeness measurement (per-era field
completeness, award linkage, results materialisation, TED↔DÖE merge) — cannot complete a single query
against prod. Run today with a valid token, on-box, against `localhost:8080`, queue idle, 01:55 UTC:

    query `versions`      failed (HTTP 408: query exceeded the 10s time limit)
    query `title`         failed (HTTP 408) … `buyer`, `value`, `cpv`, `deadline`, `winner`
    query `linkage`       failed (HTTP 408)
    query `density_can`   failed (HTTP 408) … `density_with`
    query `merge`         failed (HTTP 408)

**11 of 11.** Every section then printed with no rows, and section 4 printed the actively wrong-looking
"DÖE procedure Tenders: 0; merged with TED: 0" — a real zero and an unmeasured zero rendering
identically.

## Why it happened, and why it went unnoticed

Nothing regressed in the code. The queries were bounded *for the corpus that existed when issue 27
first ran them* (mid-backfill, documented in docs/research/data-quality.md). The corpus has since
reached 14.27M notices → 7.9M tenders → ~14.15M tender_versions, with 40.9M organization_mentions, and
a satellite-driven `GROUP BY` over the version layer is now a multi-minute scan. The 10s `/v1/sql` cap
— which exists precisely to protect the serving DB (issue 17/120) — refuses them all.

**CORRECTION (2026-08-18, same day, owner).** The first version of this issue said the tool "exits
successfully while reporting nothing" and made that the reason the rot hid. That is WRONG, and I
inferred it from the module docstring ("no pass/fail — it *measures*") without reading the exit path.
`bin/data-quality` in fact handles degradation properly: every failed query is named on stderr, sets
`degraded = true`, and the process returns `ExitCode::FAILURE`. A scheduled run would NOT have looked
fine.

The real reason it went unnoticed is duller and more actionable: **nothing runs it.** It is a manual
tool, invoked by a person when someone thinks to, and nobody had since the corpus outgrew the queries.
So the project has had no semantic-quality measurement for however long that is, while believing issue
27 delivered one — not because the signal lied, but because no one was listening for it.

One real conflation does survive, in the RENDERED output rather than the exit code: section 4 printed
"DÖE procedure Tenders: 0; merged with TED: 0 (—)" for a query that never ran. stdout shows zeros
where it should show "unmeasured", so a reader who sees the report pasted into a comment — without the
stderr lines or the exit code — is misled. That is worth fixing on its own.

## What NOT to do

Do not raise the cap or retry. `docs/agents/prod-box-reads.md` is explicit: a 408 bounds the wait, not
the work, and turso cannot interrupt a statement, so each retry stacks another uninterruptible scan.
Today's run alone spent ~11 × 10s of uninterruptible scan on the serving DB (deliberately, once, in a
low-traffic window, as issue 27's acceptance check) — it should not be repeated as-is.

## Fix direction

The dashboard already computes comparable aggregates at this scale, and it does so *in-process*: the
background refresher runs them on the isolated reader pool with no request deadline, gated by
`heavy_write_in_progress` and a change-gate so an idle server does not re-scan (issues 61/191). The
data-quality report wants the same treatment rather than a bigger hammer:

1. move the measurement server-side, onto the refresher's cadence, and expose the computed result
   (a panel section, or a `/v1/…` read of the stored numbers) — the binary then formats instead of
   scanning; or
2. keep it external but make each query bounded in the way the era queries already are — per-era, per
   satellite, with an id window, so each stays inside 10s; or
3. accept a sampled measurement (documented as such) — a stratified sample per era answers "what share
   carries a title" to within a known error at a fraction of the cost.

(1) matches how every other expensive number in this codebase is produced and is the recommendation.

**Two smaller pieces, independent of which option above wins:**

- ~~The RENDER must distinguish unmeasured from zero~~ **DONE** (`1f9a538`, 2026-08-18): failure is now
  explicit in the type — `from_labelled` takes `Option<Rows>`, `None` meaning the query did not run —
  and `Raw`/`Report` carry the unmeasured labels. The report opens with an `INCOMPLETE: N of 11 queries
  did not run (…)` line and section 4 prints `UNMEASURED — the \`merge\` query did not run` instead of a
  zero. Test drives the exact prod shape (all 11 failed) plus the complement (a query that ran and
  matched nothing still reports its real zero, no banner).
- Something must RUN it. A report nobody invokes cannot rot loudly however good its exit code is —
  which is precisely what happened here. If the measurement moves server-side (option 1) this solves
  itself, since the refresher runs on a cadence; if it stays external, it needs a schedule and somewhere
  for a non-zero exit to land.


## Scale half: MEASUREMENT MOVED TO A JOB (2026-08-18, owner — deployed rev `d09e08b`)

Implemented, with a refinement on option 1 above: **a job, not the dashboard refresher.** The
refresher's cadence is 60 s and assumes seconds-long sections; measured on prod tonight each field
query takes roughly **five minutes**, so eleven of them on that cadence would add close to an hour of
full scans after every ingest. As a job they are queue-serialized against ingestion (so no fold or
process runs beside them), execute on the reader pool (a long scan cannot block the writer), carry an
issue-65 phase record naming the query in flight, and land in the job log with a digest.

- `Db::measure_rows` — one read-only aggregate on the reader pool, no sandbox, no deadline.
  Deliberately documented as unreachable from a request path and must stay that way; the caller is a
  supervisor job, not a handler.
- `reports` table, one row per kind, replaced per run. A history of multi-minute scans is not worth
  storing; a stale report that states when it was measured beats none.
- `POST /admin/jobs {"kind":"data-quality"}`. Same `data_quality::queries()` as the binary — one
  definition of what semantic completeness means — so `bin/data-quality` still works against a small
  instance, and only the transport differs.
- The failed-vs-empty distinction carries end to end: an erroring query becomes `None`, the stored
  body marks those sections UNMEASURED, and the summary reads "PARTIAL: n/11 queries failed (…)".

**First prod run, 2026-08-18 — and it had to be killed.** The `versions` query, which 408'd through
`/v1/sql`, DID complete on the reader pool, which confirmed the deadline was the obstacle for it. But
the run then sat on the SECOND query (`title`) for **over 50 minutes at 100% CPU**, single-threaded, no
I/O wait, with no sign of finishing. My "~5 min per query" figure earlier in this issue was a guess
from two early observations and is **wrong** — at least one of these aggregates is not merely expensive
but algorithmically wrong for this corpus (nested loops over the version×satellite join, most likely).

Because jobs are queue-serialized it would have held the daily ingest behind it for hours, so the
measurement is now **gated behind `TENDER_DATA_QUALITY=1`** (`37a8c4c`) and deploying that gate cleared
the stuck run: the restart killed it, the recovered job declined immediately, the queue drained, and
`/health/deep` stayed green throughout (declining reports SUCCESS on purpose — an `error` outcome would
have turned the last-job check red and traded a blocked queue for a false alarm).

**The mistake worth recording:** the 408s told me these queries exceeded 10 s and I read that as
"a bit slower than the cap" instead of measuring the real cost before running eleven of them on the
serving box. The `/v1/sql` deadline was doing its job; I removed it without first asking what it was
protecting against.

A per-query timeout is NOT the fix: turso cannot interrupt a statement
(`docs/agents/prod-box-reads.md`), so a deadline abandons the future while the scan keeps burning a
pooled reader.

### Still open

1. **Nothing schedules it.** This is the gap that hid the rot in the first place, so it is the one that
   matters most. An hour-long measurement does not belong on the daily's critical path — a weekly
   timer, or a monthly one, queued outside the 09:35 window.
2. **No read surface** for the stored body beyond the `reports` table itself. `bin/data-quality`
   should learn to fetch the stored report instead of re-running eleven scans, and/or an operator
   endpoint should serve it with its `computed_at`.
3. **The query cost is the BLOCKER now, not a footnote** — it gates items 1 and 2, since there is no
   point scheduling or serving a measurement that cannot finish. Next step is a plan check on the
   completeness queries (`EXPLAIN QUERY PLAN` via the hot-read-plans gate, which is metadata-only and
   free per the prod-box-reads rule), starting with `title`, then whichever satellite joins share its
   shape. Expect a missing index on the satellite's `(notice_id)`/`(tender_id, seq)` side, or a
   `COUNT(DISTINCT …)` that forces a sort over tens of millions of rows. Re-cost, then re-open the
   gate.


## Second attempt: root cause FOUND and fixed, cost estimate wrong AGAIN (2026-08-18)

**The pathological shape was documented in this file the whole time.** The linkage query carries an
explicit warning that it does NOT wrap its satellite in an inline `(SELECT DISTINCT …) AS a JOIN`,
because *turso re-evaluates such a derived table per outer row* and times out "even at a few thousand
rows". `field_sql` built exactly that shape. So last night's >50-minute single query was a known trap
applied to the wrong query and never revisited — not a surprise about the data.

Rewritten to the prescribed shape (`b04c897`): drive from `tender_versions`, probe the satellite with
an indexed `EXISTS` on `(tender_id, seq)`, the by-version index texts/amounts/dates/classifications all
carry. A test now pins it — no `SELECT DISTINCT` in any field query, an `EXISTS` in every one — so the
trap cannot return.

**And then I got the cost wrong a second time.** Measured with bounded windows instead of another
uncapped run: 0.37 s at 50k tenders, 1.25 s at 200k, 4.79 s at 800k — clean and linear, ~6 µs/tender,
extrapolating to ~47 s over 7.9M. Ran it confirmed; five minutes later it was still on the same query.
The windows were **cache-hot**: at 800k tenders the satellite index pages fit in RAM, and per-row cost
climbs once the working set does not. A linear fit across three points inside the cache says nothing
about the point outside it.

Two cost estimates, two errors, one night. The lesson is not "estimate better" — it is that this
measurement must be **inherently bounded rather than hopefully fast**.

`a79540e` therefore makes the job DECLINE unconditionally, which also stopped the second run (the
deploy's restart killed it; the recovered job returned the refusal instead of starting a third scan).
`/health/deep` green throughout; `bin/data-quality` still works against a small instance.

### The design to build (and it already exists elsewhere)

Issue 228 solved this exact class for the adjacency sweep: **window the id range, bound every query on
both axes, report progress per window.** Applied here — accumulate per-profile counts over
`tender_versions.tender_id` windows, summing in Rust — each query is measured-fast at its window size
(0.37 s per 50k is a measurement, not an extrapolation), the pass is interruptible between windows, and
the phase record shows real movement. ~158 windows at 50k for the full corpus.

Structural note for whoever builds it: `queries()` currently hands out opaque SQL strings, so nothing
can inject a range predicate. The catalogue needs to declare, per query, whether it is windowable and
on which column — which is also the honest place to record that `merge` and the density queries may not
be windowable the same way.


## Windowing, step 1 landed (`5d64015`, 2026-08-18)

The primitives, with the correctness property tested rather than asserted in prose:

- `windowed_queries()` — the seven version-driven queries (denominator + six field probes) as templates
  with a `{window}` placeholder, each running over a half-open `(lo, hi]` slice of
  `tender_versions.tender_id`.
- `sum_profile_counts()` — folds per-window results into the one per-profile result set the assembler
  already expects.
- `unwindowed_labels()` — names the four that are NOT windowed (`linkage`, both densities, `merge`),
  so a caller reports them as unmeasured through the `None` path instead of dropping them. Each drives
  from a different table and wants its own decision; a partial report that says what is missing beats a
  complete one that cannot finish.
- **Test:** over the real fixture corpus with a window size of ONE — every `tender_id` its own window,
  every seam exercised — the summed result equals the whole-range result for all seven. If that
  property fails, a bounded measurement is a wrong measurement, so it is checked rather than argued.

`queries()` is untouched, so `bin/data-quality` against a small instance is unchanged.

### Remaining, in order

1. Teach `run_data_quality` to drive `windowed_queries()` — walk `MAX(tender_id)` in fixed windows,
   accumulate, report `window k/N` in the phase record, and mark the four unwindowed labels as
   unmeasured. Then lift the refusal.
2. Re-measure on prod **outside** the 09:35 daily window, and record the real per-window timing — a
   measurement, not an extrapolation. Pick the window size from that.
3. Only then: schedule it, and add the read surface for the stored body.
4. Separately decide windowing for the four remaining queries, or accept them as permanently
   unmeasured and say so in the report's own text.
