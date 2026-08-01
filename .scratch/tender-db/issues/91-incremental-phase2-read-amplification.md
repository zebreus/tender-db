# 91 — the incremental Phase-2 fold re-reads scattered notices: 8h21m for a 7,211-notice daily

Status: FIXED 2026-08-02 (proj-fix) — incremental Phase 2 now routes at `bucketed_fold` above a
planned-notice threshold. Awaiting team-lead diff review + deploy.
Kind: performance / throughput (projection Phase 2)
Blocked by: —
Relates to: 58 (incremental projection), 62 (the bucketed fold this reuses), 67 (fold-apply statement
reduction), 85 (the DE-1.x re-fold this unblocks), 90 (the observability that proves it)

## Symptom — two prod measurements

**The ordinary daily, which COMPLETED** (2026-08-01 03:01 → 11:22):

```
[project] incremental: 7211 changed → 6384 touched Tenders (0 retired) in 30059.0s
```

**8h21m for a 7,211-notice delta** — and grouping finished at 04:09, so **7h13m of it was
post-grouping**, i.e. inside the Phase-2 fold+apply loop. That is ~4.1 s per Tender. The daily
projection was effectively saturating the box every single day.

**The eForms-DE 1.x re-fold, which did not** (2026-08-01, issue 85): ~218,635 changed notices, killed
after 5h07m of silent single-core CPU in the same post-grouping region with nothing committed.
Extrapolating the daily's 4.1 s/Tender, the re-fold needed **≥10 days**.

## Root cause

`project_incremental` used `Phase2::ParsedFold` unconditionally. Its own doc comment already records
the cost:

> Read each fold batch's parsed layer by an `IN(…)` over its scattered notice ids — random rowid seeks
> into the cold notice tables in group_key order, ~50ms/notice at scale (the ~7-day Phase-2, issue 62).
> **Still used by `project_incremental` (its delta is small and already scoped)**

That parenthetical is the bug. It is a correct trade for a few-hundred-notice daily, where sweeping the
whole corpus would be absurd. It is the wrong trade the moment the "delta" is a re-fold: `parsed_by_ids`
re-reads a six-figure scattered id set through ~5,000 `IN (512)` queries per 50k-notice batch, against
notice tables totalling hundreds of GB — after the plan build has *already* read the same rows once.

Issue 62 built `bucketed_fold` for exactly this: read the parsed layer ONCE sequentially, spill each
resolved `NoticeState` to an order-preserving on-disk bucket, fold each bucket sorted in RAM. Measured
on prod during the 8.1M rebuild: the sharded pre-pass resolved the **whole 14.1M-notice parsed layer in
7h13m**, and the fold+apply of 8,104,032 Tenders took 11h51m.

## What the diagnosis eliminated first

Recorded because the wrong answers were expensive and are worth not re-deriving. Every one of these was
measured against turso 0.7 with the exact prod DDL and **eliminated**:

| candidate | measured | verdict |
|---|---|---|
| `fold()` O(chain²) in accumulated state (issue 92) | real, but max DE-1.x chain is **42** notices ⇒ ~0.1 s for the whole cohort | not it |
| `read_results` O(values × result-entities) | a real DE-1.x notice averages **332 elements**, worst ~458 entities | not it |
| `retire_regrouped_tenders`' `tenders WHERE id = ?` ×220k | **0.012 ms, flat** at 200k/1M/4M rows, with and without the deferred indexes, after ANALYZE ⇒ ≈3 s total | not it |
| `plan_notice WHERE group_key = ? LIMIT 1` | 0.010 ms, hits and misses alike | not it |
| `plan_counts()` (2× `COUNT(DISTINCT)`, 400k plan rows) | 0.50 s | not it |
| `next_plan_batch` | 0.05 s per 50k batch, flat over 8 batches | not it |

By elimination the cost is the Phase-2 **read** — which is what this issue fixes, and which the
issue-90 stage timings will now confirm directly on the next run.

## Fix (landed)

`project_incremental_chunked` picks its Phase-2 fold by plan size:

```rust
const INCREMENTAL_BUCKET_THRESHOLD: usize = 100_000;   // planned notices
```

Below it, `ParsedFold` as before — a 200-notice daily stays sub-minute and never sweeps the corpus.
At or above it, `Phase2::Buckets`, the sequential sweep proven at 14.1M notices.

The bucketed path already honours a **scoped** plan with no change: `write_shard` skips notices absent
from the plan ("a notice absent from the plan … is never folded and never marked projected"),
`bucket_boundaries` derives from whatever plan is on disk, and `fold_bucket` already takes `rebuild`
and already calls `mark_projected`. So this is a routing change, not new fold machinery.

`project_incremental_chunked_phase2(db, chunk, Option<Phase2>)` forces a fold for the invariance test.

## Byte-identity proof

`Phase2`'s contract is that both folds are byte-identical — they change only HOW the parsed layer is
read. New test `incremental_bucketed_fold_matches_parsed_fold_and_full` (project_incremental.rs) drives
the SAME delta three ways — full non-rebuild projection, incremental via `ParsedFold`, incremental via
`Buckets` — and asserts all three canonical layers are byte-identical (surrogate ids included), that
the untouched Tender's fingerprint is unchanged (the bucketed sweep reads its notices and must still
leave it alone — the scoped-plan assertion that actually matters), that a two-notice chain folds into
one Tender, and that the change-set drains.

## Expected effect on the DE-1.x re-fold (issue 85)

| | measured/estimated |
|---|---|
| scoped plan build (pass 1+2 + grouping) | ~2h17m (measured, 2026-08-01) |
| bucketed pre-pass sweep | ~7h13m (measured on the 8.1M rebuild; fixed cost, sweeps 14.1M) |
| fold + apply ~250k Tenders | ~0.5–2h (pro-rata from 11h51m/8.1M, slower now that the tender indexes are present) |
| **total** | **~10–12h, zero downtime, no layer wipe, no changes-feed reset** |

versus ~30h with the tender layer empty throughout for a `rebuild=true`.

## Follow-up not done here

The ParsedFold apply loop still has **no WAL checkpoint** (the full path checkpoints every
`CHECKPOINT_EVERY_BATCHES`). A long sub-threshold incremental therefore grows the WAL monotonically —
the issue-42/63 failure shape. Deliberately out of scope: it changes prod behaviour on the daily path
and was not part of this fix. Worth its own ticket.
