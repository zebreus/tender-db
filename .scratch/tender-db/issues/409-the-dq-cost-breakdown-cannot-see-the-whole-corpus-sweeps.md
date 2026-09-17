# 409 — the data-quality run's cost breakdown covers only the WINDOWED queries, so eleven whole-corpus sweeps (~10 % of the run) are unattributed

Status: needs-triage — filed 2026-09-17 from job 1446's output while accepting issue 402 unit B.
Measured, not inferred: the logged breakdown sums to 4,933 s against a run the job itself reports
as 5,507 s.
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
