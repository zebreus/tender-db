# 243 — the data-quality pass went from 40 minutes to ~4 hours, and it holds the job queue the whole time

Status: ready-for-agent — **the result-section probe is REWRITTEN as two point seeks, gated (129/129) 2026-09-26** (see the last section): `kind IN (…) AND notice_id = ?` planned as a walk of every section of the notice by the primary key's `notice_id` prefix, and the eForms windows (tender ids ≤ 1.25M, 57 sections per version) were paying ten times the text era for it in `sections_can` (1,173 s) and `awards` (1,880 s) of the 6,057 s run; on prod slices the two-seek form is 4–6× faster with identical rows, and turso's plan is pinned by a test. **DEPLOYED 2026-09-26 09:29 UTC at `def770a`** (health green, no error lines, queue idle); **the Sunday 2026-09-27 01:10 UTC weekly run is the measurement** — read its `cost by query` line. Next candidates (covering indexes for `title`/`buyer`/`cpv`, a single code read for `awards`+`doc_types`) are sized at the foot, none built. Was: the award merge is DONE in code 2026-08-19 (one `awards` query replacing three); the
`sections_can`/`sections_with` pair is the remaining candidate. Runtime figure corrected to 92.8 min.
Kind: cost regression in a scheduled job (correct numbers, impractical runtime)
Blocked by: — (the fix wants the per-label cost breakdown this very run will print)
Relates to: 230 (windowed measurement), 235 (added three queries), 242 (added the fourth), 27 (the report)

## What

The full-corpus data-quality pass now projects to about **4 hours**, against a measured baseline of
**39.5 minutes** (job 732, 2026-08-18) and 21.0 minutes (job 731). From the current run's log:

    [data-quality] window 1/32 (0..250000]: 451.2s for 15 queries (451.2s elapsed)

32 windows × 451 s ≈ 4.0 h. Per query that is ~30 s now against ~6.7 s before (39.5 min / 32 windows
/ 11 queries). Jobs are serialized, so the queue is held for the duration — the daily
probe/process/project chain waits behind it. `/health/deep` stays green (ingest freshness threshold is
26 h and the box is otherwise idle), so this is a practicality problem, not an outage.

## Why

Issues 235 and 242 added four per-version probes into `notice_codes` / `notice_sections`:

| query | probes per version | population |
|---|---|---|
| `awards_can` | 1 (`notice_codes`) | every version in the window |
| `awards_with` | 1 + 1 (`lot_results`) | every version in the window |
| `awards_barren` | 1 + 1 (`notice_sections`) | every version in the window |
| `doc_types` | 2 (`notice_codes`) | every version in the window, no filter at all |

Eight correlated probes per version where the old pair did two, and none of them can be narrowed by a
cheap predicate first — award-hood IS the probe. Ad-hoc timings on a 40k-tender window were 1.3–2.0 s
each, which looked affordable; at 250k tenders per window and in-process cache behaviour the real cost
is several times the naive scaling.

## The candidate fix, and why it is not written yet

`awards_can`, `awards_with` and `awards_barren` evaluate the SAME award predicate over the SAME
population and differ only in one extra `EXISTS`. They can be one query:

    SELECT n.profile,
           COUNT(*)                                             AS award_notices,
           SUM(CASE WHEN EXISTS(lot_results …)   THEN 1 ELSE 0 END) AS with_results,
           SUM(CASE WHEN NOT EXISTS(sections …)  THEN 1 ELSE 0 END) AS no_award_content
      FROM tender_versions tv JOIN notices n ON n.id = tv.caused_by_notice_id
     WHERE {window} AND (award)
     GROUP BY n.profile

That is one `notice_codes` probe instead of three and one scan instead of three, and it makes the
three numbers atomically consistent — the `IMPOSSIBLE` state issue 235 guards against becomes
unrepresentable rather than merely tested. `doc_types` drives a different population (every version,
unfiltered) so it stays its own label; it is likely the single most expensive query and may want its
own treatment.

**Not implemented yet on purpose.** The measurement loop already accumulates elapsed time PER LABEL
across the whole run and logs the breakdown at the end — exactly the evidence needed to decide what to
merge, and whether `doc_types` needs more than merging. Guessing from one window's total, when the
run itself is about to print per-query numbers, is the mistake this project keeps not making.

## Steps

1. Let the current run finish; read the per-label cost breakdown from the job log.
2. Merge the three award queries into one (above), keeping the assembler's column reads and every
   existing test green — the label set is asserted by
   `queries_are_labelled_in_execution_order`, and `windowed_sums_equal_the_unwindowed_result` already
   covers multi-column windowed sums.
3. Decide `doc_types` on the evidence: merge into the same pass, sample rather than sweep, or accept
   it as the one expensive diagnostic.
4. Re-run and record the new duration next to the 39.5 min / ~4 h pair, so the next person adding a
   query knows the budget they are spending.
5. Fix the stale runtime claims in the code while there: `supervisor.rs` calls it "a ~10 minute
   full-corpus pass" in one comment and "a 36-minute job" in another, and both are now wrong.

## Note for whoever adds the next query

The cost of a query in this report is not "how long does it take on a test window" — it is that,
times 32, plus its share of a queue that everything else waits behind. Two of the four queries added
this week would have been affordable; four were not.


---

## Corrected with the finished run (2026-08-19, owner)

**The ~4 h projection in the filing above was wrong.** It extrapolated from window 1, which was cold:
451 s. Warm windows ran 120–190 s and the dense eForms tail 320–416 s. The run finished in **5,566 s =
92.8 minutes** (job 751: 23 eras over 32 windows, 0 labels unmeasured), against the 39.5 min baseline.
So the regression is ~2.3×, not 6×. Filing a projection as if it were a measurement was the mistake;
the number below is the measurement.

### Cost by query, whole run

    awards_can      1346s   <- new (issue 235)
    sections_can     900s
    awards_barren     813s  <- new (issue 242)
    title             746s
    doc_types         614s  <- new (issue 235)
    awards_with       495s  <- new (issue 235)
    cpv               203s
    deadline           87s
    buyer              77s
    versions           65s
    merge              65s
    winner             53s
    value              37s
    linkage            36s
    sections_with      28s

The four new queries are 3,268 s of 5,566 s — **59 % of the run**, which matches the 2,370 s baseline
plus 3,268 s almost exactly.

### The one surprise, and it points at the fix

`awards_can` (1,346 s) costs nearly 3× `awards_with` (495 s) **despite doing strictly less work** —
`awards_with` is the same query plus an `EXISTS(lot_results …)`. The extra predicate makes it FASTER,
which means the planner evaluates the cheap `lot_results` seek first and the expensive `notice_codes`
document-type probe only for rows that survive it. So the document-type probe is the cost, and probe
ORDER is worth as much as probe count.

That makes the merge in the filing above more attractive than estimated: one pass, one document-type
probe, three counts. Expected saving 1,300–1,800 s of the 2,654 s the three award queries cost today —
call it 25–30 % of the whole run.

`sections_can` at 900 s is next, and it is not new: it is the invariant's denominator (section 3b),
which sweeps `notice_sections` for every version in the window. Worth folding into the same shape as a
second step, since 3b's two halves have the same relationship as section 3's three.

### Revised steps

1. Merge `awards_can` + `awards_with` + `awards_barren` into one CASE-aggregate pass, with the cheap
   `lot_results` / `notice_sections` predicates written FIRST so the planner keeps the ordering the
   timings just revealed. Expect ~70 min → ~65 min plus a much better worst case on the dense windows.
2. Then consider the same treatment for `sections_can` / `sections_with` (section 3b).
3. `doc_types` (614 s) is a coverage diagnostic over an unfiltered population; leave it alone until 1
   and 2 land, then re-measure before touching it.
4. Fix the stale runtime claims in `supervisor.rs` ("~10 minute full-corpus pass", "a 36-minute job")
   to the measured 93 min, and note that the number moves whenever a query is added.


---

## The merge, A/B'd against prod before touching any plumbing (2026-08-19)

Rather than refactor the report and hope, the merged SQL was run beside the three separate queries on
the same window (`tv.tender_id` 1,840,000–2,100,000, text era, 122,274 award versions, single-marker
predicate so it could be written by hand):

| form | query time | numbers |
|------|-----------|---------|
| three separate | 1.08 + 1.14 + 1.32 = **3.53 s** | 122,274 / 0 / 122,274 |
| merged, one pass | **2.46 s** | 122,274 / 0 / 122,274 |

**Identical numbers**, which is the half that matters — the merge is an algebraic rewrite and the
window confirms it — and ~30 % less query time, plus three round trips collapsed to one.

**So the earlier 25–30 %-of-the-run estimate was too optimistic.** Scaling the measured 30 % onto the
2,654 s the three award queries actually cost gives ~1,850 s, a saving of ~800 s: **14 % of the
5,566 s run**, not 25–30 %. The reason is visible in the A/B: the merge pays the document-type probe
once instead of three times, but the `lot_results` EXISTS and the `notice_sections` NOT EXISTS still run
for every row that passes the predicate, and on this era every row passes.

Still worth doing — 800 s off a job that holds the serial queue, and one round trip instead of three —
but it is a 14 % fix, and the next thing after it (`sections_can`, 900 s) is worth more than the
remainder of this one.

### What the refactor has to touch (scoped, not yet done)

`sum_profile_counts` already sums every column after the label, so a 3-count row needs no new summing.
The rest: one `awards_template` replacing three; one catalog + one windowed entry replacing three;
`RawRows`' three fields becoming one; the assembler's three `count_by_profile` calls becoming one pass;
and the `UNMEASURED` narration, which loses per-query granularity — if the merged query fails, all
three numbers go unmeasured together. That last point is a real (small) loss and the honest note to put
in the narration: they share one scan, so in practice they always did fail together.


## The merge landed (2026-08-19)

One `awards` query replaced `awards_can` / `awards_with` / `awards_barren`: `COUNT(*)` plus two `CASE`
sums over the same rows, with the cheap predicates inside the sums and the document-type probe in the
`WHERE` — the order the timings argued for.

`sum_profile_counts` needed no change (it already summed every column after the label, which is what
lets a 3-count row fold across windows), and the assembler reads three columns from one row set. Two
consequences worth having on the record:

- **`IMPOSSIBLE` is now unreachable from the database.** The render exists because prod produced 0 award
  notices against 139,961 with results for sdk-0.1 — but that was two queries measuring two populations.
  With one pass the numerator cannot exceed the denominator. The render and its test stay deliberately:
  a defensive path nobody exercises rots, and splitting these counts again would need it.
- **Failure granularity is coarser by one label**, which is honest — they shared a scan, so they always
  failed together in practice.

Next, if it is worth it: `sections_can` (900 s) and `sections_with` (28 s) are the same shape for section
3b. That pair is worth more than the remainder of this one, and the same A/B-first discipline applies —
run both forms against prod on one window and compare row for row before touching the plumbing.


---

## 2026-09-26 — the remaining half, measured: the cost is one probe's shape, and it is in the eForms windows

A month of the run's own `cost by query` lines (08-20 … 09-20, 23 runs) keeps the same ranking:
`awards` 1,500–3,900 s, `sections_can` 700–1,600 s, `title` 600–980 s, `doc_types` 300–1,900 s
(cache-dependent), everything else under 250 s. The 2026-09-20 weekly run (job 1501): **6,057 s**;
`awards` 1,880, `sections_can` 1,173, `title` 963, `doc_types` 348, `cpv` 235 — the top three are
66 % of the run.

**Where the time goes is a window shape, not a query shape.** The per-window log line says windows
1–5 (`tender_id` ≤ 1.25M) take 360–520 s each, windows 6–17 take 40–65 s, 18–34 take 80–230 s:
the first five windows are 43 % of the windowed time. A bounded count per window says it is not
row count — windows 1–5 hold ~580k versions each, window 34 holds 498k and runs in 228 s, window
18 holds 399k and runs in 80 s. It is the ERA: tender ids 1..1.25M are the daily-ingested eForms
corpus (minted first, before the legacy backfills took the higher ids), and an eForms version
carries **75 `notice_codes` rows and 57 `notice_sections` rows** against 5 / 4 for sdk-0.1 and
11 / 2 for the text era (5k-tender slices, bounded reads). Every per-notice probe that walks a
prefix pays ten times there. (The 08-19 A/B above called window 1,840,000–2,100,000 "text era" —
right for that window, and the reason the merge's 30 % was measured on the cheap shape.)

**turso's plans, from a scratch DB with the real schema** (the prod-box-reads pattern; `/v1/sql`
refuses `EXPLAIN`): the codes probe already seeks all three key columns —
`notice_codes_1 (notice_id=? AND section_id=? AND field_id=?)` — so the award predicate is tight.
The result-section probe does NOT: `kind IN ('LotResult', 'TenderResult') AND notice_id = ?` plans
as `SEARCH s USING INDEX sqlite_autoindex_notice_sections_1 (notice_id=?)` — the primary key's
prefix, walking every section of the notice and filtering on `kind`; `notice_sections_kind_notice
(kind, notice_id)` is never used. That probe sits in `sections_can` (every version) and in
`awards`' `no_award_content` column (every award version). The field probes (`title`, `cpv`,
`buyer`) seek `(tender_id, seq)` and then read each satellite row to test `field`/`scheme`/`role`
— 13 rows per eForms version for `title`.

**A/B on prod, bounded slices, rows compared for equality:**

| slice | query | as written | two point seeks on `(kind, notice_id)` |
|---|---|---|---|
| 0..25,001 (eForms, 58k versions) | `sections_can` | **>10 s (capped)** | 2.40 s |
| 0..10,001 (eForms, 23k versions) | `awards` | 9.93 s | **2.34 s** |
| 0..10,001 | `sections_can` | 2.64 s | 1.79 s |
| 0..8,001 | `sections_can` | 0.81 s | 0.13 s |
| 4,250,001..4,275,001 (text, 38k versions) | `sections_can` | 0.96 s | 0.23 s |
| 0..10,001 | `doc_types` (unchanged) | 2.35 s | — |
| 0..25,001 | `title` / `buyer` / `cpv` / `value` as written | 4.23 / 1.54 / 1.10 / 0.28 s | — |
| 0..25,001 | bare versions→notices scan (the floor) | 0.17 s | — |

Identical rows in every pair. The floor scan is 0.17 s on the same slice, so the probes are the
whole cost and the scan-sharing merge this record once planned would buy nothing by itself.

**Built, gated (129/129) and deployed 2026-09-26 09:29 UTC at `def770a` (this firing):** `result_section_probe()` — one `EXISTS` per kind in
`RESULT_SECTION_KINDS`, each an equality on `kind` and `notice_id` — shared by `sections_can_sql()`
(now a template like `awards`, windowed by the same builder) and `awards_template`'s
`no_award_content`. The planner's choice is pinned by
`the_result_section_probe_seeks_kind_then_notice` (`tests/data_quality.rs`, turso's `EXPLAIN
QUERY PLAN` on a scratch DB with prod's schema) and the vocabulary test now asserts the equality
form. The stale "~10 minute" / "36-minute" comments in `supervisor.rs` (step 5 above) say ~100
minutes and point at the cost line. Expected: `sections_can` ~1,173 → ~300 s, `awards` ~1,880 →
~600–900 s (its sections walk ran only on award rows, but on eForms rows they are most rows);
the run's total from ~6,050 s to ~4,000 s. **The Sunday 2026-09-27 01:10 UTC run is the
measurement** — read its `cost by query` line against the table at the top of this section.

**Next, in order of value, none built yet:** (1) `title` 963 s, `buyer` 194 s, `cpv` 235 s — a
covering index per satellite, `(tender_id, seq, field)` on `tender_version_texts` and the like,
turns the 13-row walk into a first-match seek; that is a migration on the largest satellites
(a long writer hold, a production write — gated with the other jobs), so it is a decision for
Lennart with the 09-27 cost line in hand. (2) `doc_types` 348 s + the award predicate probe the
same `(notice_id, 'PROCEDURE', field_id)` key up to three times per version (`award`, `unknown`,
`any`); one scalar read of the code per marker could serve all three, but that changes how the
section-3 denominator is evaluated and wants its own parity A/B. (3) `sections_can` and `awards`
still compute the (now cheap) sections probe twice for award rows; folding `sections_can` into the
awards pass is the 08-19 merge's shape and is worth ~100 s at most after this unit.

## 2026-09-26 10:xx — the covering indexes for `title`/`cpv`: built, measured, NOT shipped

Built and tested locally, then reverted before commit; the evidence is recorded so the decision can be
taken on numbers.

**turso takes the seek.** On a scratch DB with prod's schema plus
`tender_version_texts(tender_id, seq, field)` and `tender_version_classifications(tender_id, seq,
scheme)`, the plans move from `…_version (tender_id=? AND seq=?)` to `…_version_field (tender_id=? AND
seq=? AND field=?)` and `…_version_scheme (… AND scheme=?)`. A test pinning that passed (9/9 in the
suite). It does NOT help `buyer` or `deadline`: their predicates are `role LIKE '%uyer%'` and `field
LIKE '%deadline%'`, which no index column can seek, and turso kept the two-column prefix there.

**Why it is not shipped: the classifications index could never be built by the machinery that would
own it.** Sizing by rowid density (seven 1M-rowid bounded samples per table):

| table | MAX(rowid) | sampled density | est. rows | vs the 240M auto-build cap |
|---|---|---|---|---|
| `tender_version_texts` | 893,774,602 | 0.239 | ~213M | under — would build, ~10 GB sort RSS at 48 B/row, writer held for the build |
| `tender_version_classifications` | 800,887,679 | 0.347 | ~278M | **over — `build_tender_indexes` refuses it** |

A deferred index the builder refuses stays missing, so every boot's `ensure_deferred_indexes` would
queue a reindex, and each reindex runs `too_large_to_build`'s `COUNT(*)` over an 800M-rowid table and
refuses again. Putting it in the schema batch instead is the multi-hour blocking boot issues 82/83/111
removed. So `cpv` stays on the two-column seek. `title` alone would save at most ~900 s a week against
a ~10 GB, writer-holding one-time build, a permanent extra index on the largest satellite that every
fold maintains, and a rebuild path that drops and re-sorts it. For a Sunday-night diagnostic that is
not worth it; it stays unbuilt unless the 09-27 line shows `title` is now the dominant cost and a
reason appears to want the run shorter.

**Cheaper next step if the 09-27 line still reads high:** the `doc_types` / award-predicate code probe
(item 2 in the list above) needs no schema change. Measured in the same pass: in two 200k-notice
ranges (eForms + text, eForms + DÖE) no notice carries more than one code under any marker field, so
a single scalar read of the code per marker can serve the award, unknown and any-marker tests at once.

## Verify

    ssh -o BatchMode=yes root@zebreus.click "journalctl -u tender-db --since '-8 days' --no-pager | grep -F '[data-quality] cost by query' | tail -1 | cut -c1-400"

- **done**: a line newer than 2026-09-20 with `sections_can` under ~400 s and the total under ~4,500 s — the two-seek probe reached the weekly run (first chance: Sunday 2026-09-27 01:10 UTC, on `def770a` or later)
- **open**: `Sep 20 04:51:33 … (6057s total …): awards 1880s, sections_can 1173s, title 963s …` — the last run before the rewrite (read 2026-09-26); a newer line with `sections_can` still over ~1,000 s means the planner did not take the seek on prod and the pinned plan is wrong about the box

A journal read, free per `prod-box-reads.md`.
