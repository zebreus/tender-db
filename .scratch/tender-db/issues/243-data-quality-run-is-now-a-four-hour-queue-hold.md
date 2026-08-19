# 243 — the data-quality pass went from 40 minutes to ~4 hours, and it holds the job queue the whole time

Status: needs-triage — measured 2026-08-19 on prod, rev `e2ad213` (the run is still going as this is filed)
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
