# 106 — count-balanced stripes cannot parallelise a CONCENTRATED cohort: 94 is necessary but insufficient for the reprocess

Status: proposed
Kind: performance / **blocker for the quarantine reprocess at scale**
Design owner: proj-fix
Relates to: 94 (balanced stripes — necessary, and working; this is what it does *not* solve), 66 (the sharded pre-pass), 96 (apply-side variability), 76 (quarantine reprocess — the work this blocks)

## Headline

On the 2026-08-02 eForms-DE 1.1+1.2 re-fold, **issue 94's balanced striping worked exactly
as designed and delivered ~10% — not the 5-8× it appears to promise** — because the cohort
being folded is concentrated in one stripe.

| | pre-pass wall-clock |
|---|---|
| 2026-08-01, pre-94 (effective 1 worker) | **402 min** |
| 2026-08-02, with 94, 8 balanced stripes | **~366 min** (projected from the slowest shard) |

Seven workers finished in the first quarter of the phase and idled at the join barrier;
the eighth did the real work alone. **That is the same single-threaded shape as the night
before, reproduced despite the fix functioning perfectly.**

The stripes are balanced by **notice count**. The work is not divisible that way.

## Evidence

Measured live on prod (job 4, rev `33dfba7`), first at-scale exercise of 94.

**94 itself is not in question.** The pre-pass reported

```
[project] phase 2 pre-pass: 8 shard(s) over notice ids (177, 27297321] (99% of the id space)
```

8 real stripes, sized by notice **count** — visibly not by id width (shard 0 spans 2.23M
ids, shard 1 spans 1.77M, shard 2 spans 13.86M). Counts are balanced *exactly*: shards 0
and 1 both reported `1768800 swept` on completion. All 8 workers ran concurrently; none
started idle; the pre-94 collapse to one effective worker did not recur.

### Throughput collapses ~21× across the id space

Self-labelled per-shard heartbeats:

| shard | id range | notices/s | spilled |
|---|---|---|---|
| 0 | 177 – 2,232,185 | **1,712** | 20 |
| 1 | 2,232,185 – 4,004,574 | **1,427** | 0 |
| 2 | 4,004,574 – 17,861,049 | **722 → 544** | 0 |
| 3 | 17,861,049 – 19,629,867 | **416** | 0 |
| 4 | 19,629,867 – 21,398,728 | **349** | 0 |
| 5 | 21,398,728 – 23,167,755 | **268** | 0 |
| 6 | 23,167,755 – 25,043,650 | **200** | **20,408** |
| 7 | 25,043,650 – 27,297,321 | **81** | **3,771 and climbing** |

**21× between the fastest and slowest stripe**, and the slowest is the one holding the
cohort.

### Two distinct mechanisms, and they stack

1. **Byte cost rises with id** — explains shards 2-5, which are slow while spilling
   *nothing*. The decisive evidence is a **within-shard** gradient: shard 2 fell
   **1,309 → 722 → 544 notices/s while advancing through its own stripe** — same thread,
   same code, same device, only the ids rising. That rules out scheduling, pool
   contention and stripe assignment, and pins the cause on the data: recent eForms records
   are fatter than the legacy TED records at low ids.

2. **Plan density** — explains shards 6 and 7, which additionally pay resolve + encode +
   spill per plan member. Shard 6 went from 0 to 20,408 spilled as it entered the cohort;
   shard 7 is the only shard that has been producing throughout.

Shard 7 carries **both** — high byte cost *and* the bulk of the producer work — which is
why it is 21× slower rather than the ~5× the byte gradient alone would give.

> A note on method: "plan density" was proposed first and **falsified** for shards 3-6 by
> their zero-spill readings, then found to be genuinely operating on shards 6-7 once their
> spill climbed. Both mechanisms are real; neither alone explains the spread. An earlier
> per-thread attribution (mapping tids to shards to derive KB/notice) was **retracted** —
> it failed an arithmetic check. Only self-labelled shard heartbeats are used above.

## Why this blocks the reprocess

Effective parallelism on this run is **~2.4× of a theoretical 8×** — the sum of per-shard
runtimes over the wall-clock set by the slowest.

The quarantine reprocess (issue 76) targets **2.42M notices**, and like this cohort they
are **concentrated in recent ids**. It therefore hits exactly the same wall: one stripe
inherits nearly all the work, the other seven idle, and adding shards does not help
because the bottleneck is a single stripe's serial workload.

**Sizing the reprocess off aggregate throughput would badly under-estimate it.** The
correct estimate is *the slowest stripe's* runtime, not total work ÷ shards.

## What will and will not fix it

- ❌ **Byte-weighted stripes.** Would flatten mechanism (1), not (2). A cohort concentrated
  in one id range still lands in one stripe however the boundaries are weighted — you
  cannot split a stripe's *plan membership* by choosing where to cut the id axis, because
  the members are contiguous in that axis.
- ❌ **Profile-weighted stripes.** Same limitation, cheaper to compute.
- ❌ **More shards (`TENDER_PREPASS_SHARDS`).** Finer slicing of a concentrated cohort
  still puts the dense region in one slice unless the slicing is *driven* by density —
  and if it were, that is byte/plan weighting, which fails for the reason above.
- ✅ **Work-stealing.** A worker that finishes its stripe claims unswept ranges from the
  slowest. This is the only scheme that parallelises **concentrated** work, because it
  subdivides at runtime according to what is actually left, requiring **no prediction of
  the cost distribution at all** — and this issue is precisely the evidence that the
  distribution is neither uniform nor predictable in advance.

On this run, work-stealing would have had seven idle workers absorb shard 7's remaining
range instead of waiting ~5 hours at the barrier.

## Acceptance

- A concentrated cohort's pre-pass wall-clock approaches `total_work / workers` rather
  than `slowest_stripe_work`.
- No worker idles at the join barrier while another still has unswept range.
- The 2.42M reprocess sized against the slowest-stripe model, or the model made obsolete
  by work-stealing.

## Comments

Filed 2026-08-02 from live prod measurement during the DE-1.x re-fold; rewritten the same
evening when shard 7's self-labelled 81 notices/s revealed the effect was an order of
magnitude larger than first assessed. The original framing ("second-order refinement,
~5-6× of 8×") was **wrong** — it was extrapolated before the slowest shard had reported.
The corrected finding is that 94 is necessary but insufficient, and that work-stealing is
the actual enabler for the reprocess.
