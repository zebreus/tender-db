# 409 — the data-quality run's cost breakdown covers only the WINDOWED queries, so eleven whole-corpus sweeps (~10 % of the run) are unattributed

Status: ready-for-agent — **units 1 and 2 DONE and gated 2026-09-17**, same day as filing: the
whole-corpus sweeps are timed into the same map, the line prints AFTER them (it was printed
before, so it structurally could not include them), whole-corpus labels are marked `*`, and the
line now states its total and NAMES any measured label it has no timing for. Awaiting only the
next `data-quality` run for the numbers. Was: filed 2026-09-17 from job 1446's output while
accepting issue 402 unit B. Measured, not inferred: the logged breakdown sums to 4,933 s against
a run the job itself reports as 5,507 s.
Kind: defect (instrumentation) — the instrument built to inform sizing and indexing decisions is
blind to a tenth of the work, including a full-table scan added the same day
Relates to: 402 (whose unit-B fix made one of the invisible sweeps a full `notices` pass — the
reason this stopped being tidy and became live), 246 (which added the whole-corpus queries this
does not time), 92 / 122 / 117 (the sizing and indexing decisions this breakdown exists to feed),
368 unit 4b (the same shape one report over: a diagnostic that ran and showed nobody anything)
Blocked by: nothing

## Observed

`crates/app/src/supervisor.rs` accumulates elapsed per label — and the comment beside it states
the purpose plainly:

> Elapsed per LABEL across every window, so the run says which query costs what. A per-window total
> cannot: the eleven queries differ by more than an order of magnitude in cost, so "this window took
> 291 s" identifies nothing to fix. **This breakdown is the input to the next sizing or indexing
> decision**, which is the whole reason the timings are logged at all.

`*cost.entry(query.label.clone()).or_default() += …` sits inside the **windowed** loop. The
whole-corpus statements run after it, in their own loop, and are never timed.

Job **1446** (2026-09-17, rev `92f12ec`), which reports `24 eras over 35 windows in 5507s`:

    [data-quality] cost by query: awards 1630s, sections_can 977s, title 971s, doc_types 330s,
      cpv 219s, buyer 188s, deadline 102s, versions 100s, merge 81s, amount_basis 80s,
      amount_plausibility 56s, winner 53s, linkage 40s, value 40s, factless 37s, sections_with 29s

Sixteen labels, **4,933 s**. The run measured 5,507 s. The missing **~574 s (~10 %)** is the eleven
whole-corpus sweeps plus loop overhead, and the line offers no way to split them.

## Why it matters now rather than in principle

It has always been a gap and it was never costly, because the whole-corpus sweeps were small
relative to 35 windows of sixteen probes. Issue 402 unit B changed that on 2026-09-17:
`publication_days` went from `id > MAX(id) - 2,000,000` — a primary-key range over 2M of ~14.4M
rows — to a **full pass over `notices`**, deliberately, because an ingest-ordered window could not
see the hole the section exists to find.

That was the right trade, and the run above is consistent with it being cheap. But the strongest
statement available is *"all eleven together are bounded by ~574 s"*, which is an upper bound, not
an attribution. The next person asking "should `notices.published_at` be indexed?" — the exact
question this breakdown was built to answer — cannot answer it from the instrument.

The uncomfortable shape: the measurement built to tell us what to index is the one that cannot see
the scan we just added.

## Units

**Unit 1 — time the whole-corpus sweeps into the same map.** They already run in a loop with a
label in hand; the same `cost.entry(label) += elapsed` applies. The only real decision is whether
they share the table with the windowed labels or print as a second line. **Share it, and mark
them** — a reader comparing `awards 1630s` against `publication_days 210s` wants one ordering, and
the distinction that matters (once vs once-per-window) is a property to annotate, not a reason to
split the list. Note the windowed figures are already sums across 35 windows, so the two are
directly comparable as *total run cost*, which is the quantity a sizing decision needs.

**Unit 2 — a test.** The natural one is not a timing assertion (flaky by construction) but a
coverage one: every label in `queries()` **and** `whole_corpus_queries()` appears in the cost map
of a completed run. That is the property that was violated, it is deterministic, and it fails if
someone adds a third query category later — which is how this gap arose in the first place.

## Not in scope

Whether `publication_days` is *worth* its cost, or whether `notices.published_at` should be
indexed. Unit 1 is what makes that question answerable; answering it belongs with 402 or its own
issue, and answering it before the instrument can measure it would be guessing.


## Comment — 2026-09-17: units 1 and 2 shipped

Three changes in `crates/app/src/supervisor.rs`:

- the whole-corpus loop times each query into the **same** `cost` map;
- the breakdown is printed **after** that loop. It was printed before it, which is why no amount of
  care with the map alone would have fixed this — the line ran before the data existed;
- one table, both kinds, whole-corpus marked `*` with a legend, as unit 1 decided.

And one thing unit 1 did not ask for, which is the part that actually matters. The line now states
its **total** and **names any measured label it has no timing for**:

    [data-quality] cost by query (2811s total, * = whole-corpus, run once; the rest are sums over
      35 windows): awards 1630s, title 971s, publication_days* 210s — UNTIMED (1): weld_candidates

A wrong number announces itself; a missing row does not. The original defect was invisible precisely
because the omission left no trace — it took reconciling the printed sum against the run total by
hand to see it. A future third query category would have been wrong the same way, and now it fails
loudly on its first run instead of quietly shrinking the denominator.

`cost_line` is extracted as a pure function so this is testable without a 90-minute job.
`the_cost_line_covers_every_measured_label_and_names_any_it_missed` pins the ranking, the marker,
the stated total and the UNTIMED report.

**What this does NOT do** is answer whether `publication_days` is worth its cost, or whether
`notices.published_at` should be indexed. It makes that answerable: the next run gives that query
its own number instead of hiding it inside a ~574 s upper bound shared with ten other sweeps. The
answer belongs to 402 or its own issue, and reaching it before the instrument could measure it would
have been guessing — which is the thing this issue exists to stop.

## Comment — 2026-09-17: ACCEPTED on prod — all 27 labels, and `publication_days` finally has a number

Job **1462**, the first run on `301ee34`:

    [data-quality] cost by query (5371s total, * = whole-corpus, run once; the rest are sums over 35
    windows): awards 1516s, title 975s, sections_can 857s, doc_types 325s, cpv 236s,
    withheld_markers* 204s, weld_candidates* 197s, buyer 194s, deadline 104s, publication_days* 83s,
    amount_basis 80s, merge 79s, sentinel_dates* 68s, amount_plausibility 56s, winner 53s,
    weld_bands* 50s, value 39s, linkage 38s, unmapped_fields* 36s, factless 36s, sentinel_amounts* 33s,
    sentinel_low_rate* 32s, sections_with 27s, versions 27s, longest_chain* 12s, fresh_holds* 11s,
    sentinel_amounts_low* 2s

**27 labels — 16 windowed and 11 whole-corpus** — against the 16 the old line carried. The total is
stated, the whole-corpus sweeps are marked, they sort into the same ordering, and there is no
`UNTIMED` clause because nothing was missing. Both units accepted.

### The number this was built to get

**`publication_days* 83s`** — the full pass over `notices` that issue 402 unit B introduced costs
**83 seconds, 1.5 % of a 5,371 s run.** That closes the question 402 had to leave as an upper bound,
and it closes it in favour of the change: replacing a 2M-id primary-key range with a full-table scan
was not expensive, and the alternative was a measurement that could not see its own subject.

It also means **`notices.published_at` does not need an index** on this evidence — the reason the
index was declined can now be a number rather than a judgement.

### And it corrects a derived figure of mine

402's comment said the whole-corpus sweeps were *"bounded by ~574 s"*, derived as (run total −
windowed sum) on the previous run. Measured directly here they are **728 s** (13.5 %). The old figure
was a subtraction that silently absorbed loop overhead into neither column, which is exactly the kind
of inference this issue exists to replace. The conclusion it supported — that the window change was
cheap — survives, and now rests on `83s` rather than on arithmetic.
