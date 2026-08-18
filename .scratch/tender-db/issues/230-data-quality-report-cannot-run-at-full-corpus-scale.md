# 230 — the data-quality report times out on every query at full-corpus scale, and reports empty instead of saying so loudly

Status: FIXED and measured on prod 2026-08-18 — the report runs at full-corpus scale; two
follow-ups remain (cost-per-query index decision, bin/data-quality read path)
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

## Windowing, step 2 landed — the refusal is lifted (2026-08-18)

`run_data_quality` now drives the windows, so the job is bounded by construction rather than by hope:

- Range bounds come from **two separate** indexed aggregates. `MIN(x)` alone and `MAX(x)` alone are
  index probes; asking for both in one statement makes SQLite scan the index, and a full index scan of
  14.15M versions is exactly the unbounded statement this job exists to stop running.
- The floor is *derived* (`MIN(tender_id) - 1`), not assumed to be 0. Windows are half-open `(lo, hi]`,
  so an assumed floor that was wrong would silently drop one tender — and a dropped version renders as
  a plausible percentage, not as an error.
- `dq_windows()` is a pure function, tested for exact tiling: first window opens at the floor, last
  closes at the max, consecutive windows share a boundary, none empty, none wider than the bound, and
  an empty range yields NO windows (so an empty corpus cannot store a report full of zeros). Overlap
  double-counts, a gap under-counts, and both look believable in the output — that is why the
  arithmetic is asserted instead of read.
- Windows OUTER, queries INNER: the seven probes over one slice touch the same version and notice
  pages, so a slice is paged in once rather than seven times.
- `DQ_WINDOW = 250_000` ids — an order of magnitude inside the largest window anyone actually timed
  (4.79 s over 800k, cache-hot), leaving headroom for the cold case that made the two unwindowed runs
  unkillable. Every window logs its own elapsed and the running total, so re-sizing it will be a
  decision the journal supports.
- A label with ANY failed window is reported **unmeasured**, never summed. A sum missing one window is
  a wrong number wearing a right number's clothes — the precise failure `Raw::from_labelled`'s `None`
  path exists to prevent.
- `confirmed` recovers its meaning: a dry run reports the plan (window count, statement count, and the
  four labels that will come back unmeasured) off the two aggregates alone, having touched no data page.
  The summary now names the unmeasured labels too — the job log is what an operator reads first.

Tests: `dq_windows` tiling + a dry run that plans and stores nothing, and a confirmed run over an empty
corpus that returns "no tender versions to measure" and stores nothing rather than overwriting a real
earlier measurement. 77/77 app lib, 4/4 ingest data-quality green.

**Not yet run on prod.** Item 2 below is the next thing and it is a measurement, not a formality.

## MEASURED on prod: the report runs at full-corpus scale (2026-08-18)

The thing this issue exists for now happens. Job 731, rev `a79540e`, queue idle, outside the daily
window:

    data quality measured: 23 eras over 32 windows in 1258s; 4 label(s) unmeasured

**Zero failed windows.** 14.15M tender-versions measured across 23 eras — the same queries that
returned 11-of-11 HTTP 408 when this issue was opened. Per-window timings from the journal (the real
distribution, not a line through two points): 65.8 s, 70.3 s, 65.2 s, 68.5 s, 54.9 s, then a stretch
of 12–25 s through the sparse mid-corpus id ranges, rising back to 36–65 s in the dense recent eras,
24.4 s for the short tail window. The variance is the reason a single sample extrapolates so badly:
the cheapest window in this run was 5× faster than the dearest.

The stored report is readable at `GET /admin/reports/data-quality` (body + `computed_at` +
`age_seconds`), and `/metrics` now carries
`tender_db_report_computed_timestamp_seconds{kind="data-quality"}`, so "nobody has run it lately" is
finally an alertable condition rather than a silence.

### What the first report found

Four board outcomes in the first hour of having the measurement, none of them previously visible:

- **Issue 29 VERIFIED CLOSED at scale.** It had sat at needs-verification since 2026-07-21 with a note
  saying this exact measurement was what it needed. Over 666,671 sdk-0.1 versions: title 100 %, buyer
  100 %, deadline 77.9 % — off the floor, era-wide, not fixture-wide. Its "0 % on every field" premise
  is retired.
- **Issue 231 filed** — the narrowed sdk-0.1 residual: `value` 0 % (AMOUNTS never gained an `SDK01-*`
  entry, visible in the code), `cpv` 0 % (a research question BEFORE a mapping question).
- **Issue 232 filed and then DIAGNOSED the same day** — the text era, 3,786,955 versions and ~27 % of
  the corpus, carries a buyer on 0.5 % of them. Cause found in the code: the text parser emits no
  organization role reference at all (`text/parse.rs` hardcodes every ref to `scheme: "ojs"`, which the
  projection filters as a chain edge before `role_name` is reached; `role_name` has no `TXT-` branch;
  `AU` is declared `Prose`, not a ref). Mentions get a name and nothing ever attaches a role.
- **Issue 233 filed** — INTERNAL_OJS 2008 carries a title on 43.7 % of versions where every other era
  manages 96 %+, while the same notices resolve a buyer 91.5 % and a CPV 100 % of the time.

Each of the three new issues states explicitly what NOT to conclude from the `winner` column, which is
measured over all versions and therefore dominated by the CN/CAN mix.

## Windowing step 3: all eleven queries windowed (`b50184b`, 2026-08-18)

The 4 unmeasured labels are gone. Three took the same treatment as the seven; the fourth turned up a
bug.

- `linkage`, `density_can` and `merge` each drive from `tender_versions`, so each takes a `tender_id`
  range predicate spliced into its existing WHERE. `merge`'s inner `SELECT DISTINCT v.tender_id` is
  the windowed part, and that `DISTINCT` sums exactly **only because the distinct key IS the window
  key** — a DISTINCT over anything else would need proof the key cannot straddle two windows, and
  `tender_versions` declares only `UNIQUE (tender_id, caused_by_notice_id)`, which does not give it.
- **`density_with` was measuring the wrong unit.** It drove from `lot_results` counting
  `COUNT(DISTINCT lr.notice_id)` — distinct on NOTICES while its own denominator (`density_can`)
  counts VERSIONS. A notice causing two versions contributed 2 below the line and 1 above it, quietly
  depressing results density in exactly the eras where corrigenda are common. Now version-driven like
  its denominator, probing `lot_results` on `(tender_id, notice_id)` (the prefix of its UNIQUE index,
  so still an indexed seek). Windowing it was the *occasion*; the ratio being a ratio is the fix.
- `sum_profile_counts` now sums every count column after the label, `merge` carries a constant `'all'`
  scope label to share the `[label, counts…]` shape, and `WindowedQuery` carries its own `tender_id`
  column — the eleven statements alias `tender_versions` as `v`, `v1` and `tv`, and a hardcoded alias
  becomes "no such table", which the equivalence test caught rather than a reviewer.
- New test: every catalog query must be either windowed or declared unmeasured, and every template
  must carry exactly one filled window predicate. A query in neither list is one the report silently
  does not measure — this issue's own failure mode, reintroduced by a future addition.

### The eleven-query pass costs 4× a window, and that is the next decision

First two windows of the eleven-query pass: **291.3 s** and **228.6 s**, against 65.8 s and 70.3 s for
the same windows with seven queries. So the four added queries roughly triple-to-quadruple a window,
putting the full pass in the **2–3 hour** range. The schedule survives it (a 03:10 Berlin start
finishes hours before the 09:35 daily), so this is a cost question, not a safety one.

It is deliberately NOT being answered by inspection. A per-window total cannot say which of eleven
queries spent the time, so the run now logs **cost per query, costliest first** at the end
(`f3d1bbf`). The suspicion on the table — to be confirmed or killed by that line, not by argument —
is `density_can`'s `EXISTS(… notice_sections WHERE notice_id = ? AND kind = 'LotResult')`:
`notice_sections` is indexed `PRIMARY KEY (notice_id, section_id)` plus a lone `(kind)` index, so this
predicate has no exact-seek path and must range-scan every section of each notice. If the breakdown
confirms it, a `(notice_id, kind)` index is the fix — and building one on a table that size is itself
a job, so it wants the deferred-index machinery, not an inline DDL.

The breakdown costs nothing extra to obtain: it lands with the next scheduled Sunday run.

### Remaining, in order

1. ~~Teach `run_data_quality` to drive `windowed_queries()` — walk `MAX(tender_id)` in fixed windows,
   accumulate, report `window k/N` in the phase record, and mark the four unwindowed labels as
   unmeasured. Then lift the refusal.~~ **DONE** — see "Windowing, step 2" above.
2. ~~Re-measure on prod **outside** the 09:35 daily window, and record the real per-window timing — a
   measurement, not an extrapolation. Pick the window size from that.~~ **DONE** — 1258 s over 32
   windows, zero failed, per-window distribution recorded above. `DQ_WINDOW = 250_000` stays: the
   spread (12 s to 70 s per window) shows the size is not the binding constraint, query cost is.
3. ~~Only then: schedule it, and add the read surface for the stored body.~~ **DONE** (`766bb0b`) —
   `spawn_report_scheduler` queues it Sunday 03:10 Berlin in its own loop (never stacking two), and
   `GET /admin/reports/{kind}` serves body + `computed_at` + `age_seconds`. Plus
   `tender_db_report_computed_timestamp_seconds{kind}` on `/metrics` (`c731a05`), which is what makes
   the original rot alertable rather than merely fixed.
4. ~~Separately decide windowing for the four remaining queries, or accept them as permanently
   unmeasured and say so in the report's own text.~~ **DONE** (`b50184b`) — all four windowed, and
   `density_with`'s unit bug fixed on the way.

### Remaining after all four

- **Read the cost-per-query line** from the first run that carries it and decide whether
  `density_can` (or another) wants an index. Do not add an index to `notice_sections` before that line
  says so.
- **Confirm sections 2–4 are populated** in the first eleven-query report, and judge the era winner
  questions (issues 231/232) from section 3's award-notice denominator rather than from the `winner`
  column's all-versions one.
- **`bin/data-quality` still re-runs all eleven queries live**, which no longer works against prod.
  Point it at `GET /admin/reports/data-quality` for the stored body, keeping its live path for small
  instances.
