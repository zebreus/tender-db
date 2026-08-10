# 179 — a legacy-era refold pays a full-corpus epoch rewrite (58-v1 fallback × epoch bump)

Status: ready-for-agent
Severity: MEDIUM (operational cost of every future legacy mapping fix; no
correctness impact — the output is right, it just costs half a day+)
Found: 2026-08-10 (orchestrator), watching issue 177's refold live
Relates to: 58 (the v1 fallback), 99 (epoch machinery), 174/177 (the refolds
that paid it), 175 (the fold-pipeline work this cost profile motivates next)

## What happened

Issue 177's deploy bumped PROJECTION_EPOCH 2→3 and enqueued
`refold ted-export-r208` (2,694,814 notices re-queued, ~50 min) + the
follow-up `project rebuild=false`. Two mechanisms then compounded:

1. **project.rs ~1101**: the incremental fold's pass-1 sees ANY legacy notice
   in the delta → `INCREMENTAL → FULL fallback: re-projecting the whole
   corpus` (issue 58 v1 kept legacy identity out of the incremental planner).
   An era refold of a legacy profile is BY DEFINITION all-legacy, so every
   legacy refold takes this branch.
2. **canonical.rs epoch check**: every stored tender is epoch-2 → stale → the
   fold's chain-compare keeps nothing and fully rewrites each tender it walks.

Net: the "incremental" fold walks all 7,925,880 tenders and rewrites every
one through the read-stored-chain-then-rewrite path. Observed on the box
(2026-08-10, pipelined fold from issue 175 deployed): ~4.5K tenders/min
through the legacy-heavy segment (~2.2 versions/tender written), CPU ~80% of
one core equivalent at the observe moment, memory flat (~7 GB RES), journal
clean. Projection: 13-29h wall depending on how much faster the single-version
eForms majority folds. For comparison the clean-slate `rebuild=true` path did
the same corpus in 6.5h SERIAL (pre-175, migration night): the chain-compare
rewrite is slower than a clean rebuild when ~100% of tenders rewrite anyway.

## Why it matters

Every future legacy mapping fix (the 174/177 class — there WILL be more; the
176 matrix exists because of them) pays this same full-corpus cost, and the
daily tick queues behind it (ingest freshness threshold 26h — a >24h fold
makes /health/deep go transiently red the next morning).

## Fix directions (pick one, red test first)

a. **Scoped legacy incremental** (best): teach pass-1 the legacy identity
   regime instead of falling back — the OJS chain closure of the delta is
   computable (the plan groups already encode it); fold only the touched
   plan groups. An era refold then folds ~its own era.
b. **Epoch-aware fallback choice**: when the fallback triggers AND every
   stored tender is epoch-stale anyway, run the clean-slate rebuild path
   (measured 1.8-4x faster for this shape) instead of chain-compare —
   with clear_changes now signalled by issue 46's generation.
c. **At minimum**: make the refold job's docs/notes say "legacy profiles →
   full-corpus fold; budget accordingly", so the cost is chosen, not found.

Acceptance: an r208-scoped refold folds a bounded cohort (a) or takes the
clean-slate path (b), measured against tonight's baseline; the daily tick is
not delayed by more than the fold of the actual cohort.

## Tonight's live numbers (for the baseline)

- refold pass (job 601): 2,694,814 re-queued in ~50 min.
- fold: full-corpus fallback, 674,798/7,925,880 tenders and 1,500,104
  versions written at t+170min. Final wall-clock: see issue 175/177 notes
  once complete.

## Comments

**2026-08-10 20:46 CEST (orchestrator) — final fold numbers.** The epoch 2→3 full-corpus
rewrite completed 18:46:46Z: `done: 14,246,456 notices → 7,925,880 tenders (700,425 islands),
14,246,456 versions, 63,284,607 change rows in 21,745.1s` (~6h02m; apply phase 16,304.9s);
`WAL after end-of-run index builds: 0 MB`. Caveat stands: this is the issue-58-v1 legacy-fallback
full-rewrite path (issue 179), NOT a clean issue-175 pipeline measurement.
