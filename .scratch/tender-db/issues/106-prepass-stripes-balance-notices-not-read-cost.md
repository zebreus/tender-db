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

### Byte cost dominates; plan density is a minor additive term

**Byte cost rising with id is the primary mechanism and explains essentially the whole
gradient.** The decisive evidence is a **within-shard** measurement: shard 2 fell
**1,309 → 722 → 544 notices/s while advancing through its own stripe** — same thread, same
code, same device, only the ids rising. That rules out scheduling, pool contention and
stripe assignment, and pins the cause on the data: recent eForms records are fatter than
the legacy TED records at low ids.

**Plan density does not drive cost — it is anti-correlated with it.** The decisive
measurement is *within a single worker*, holding thread, stripe, code and device constant
so that only the region changes:

| shard 7 segment | spill added | notices/s |
|---|---|---|
| 753,750 → 1,005,000 swept | **+1,612** | **186** |
| 1,005,000 → 1,256,250 swept | **+57,254** | **372** |

**The same worker did 35× more producer work in the second segment and ran 2× FASTER.**
No model in which resolve/encode/spill is what slows a shard survives that.

(A cross-shard comparison pointed the same way — shard 6 carried ~9× more producer work
per notice swept than shard 7, at 12.1% vs 1.3% cohort share, and was 1.7× faster — but it
has the confound of different workers at different progress points. The within-worker
result above does not.)

**So the sole surviving explanation is per-notice read cost varying by region**, and the
cohort's own region is *cheaper* per notice than the surrounding sparse high-id notices —
plausibly because the reclaimed cohort was written recently and contiguously.

### The long pole changed mid-run

Shard 7 holds the highest ids and was, on every model considered during the run, the
stripe that "should" dominate. It finished **before** shard 6.

**Which stripe is the bottleneck was not fixed** — it changed as the workers crossed
regions. A static weighting scheme would have to predict not merely the cost gradient but
*which stripe wins the race*, and there was no correct answer to predict.

> **Method note.** The causal account went through three revisions, each forced by a new
> measurement rather than by re-reasoning old data:
> 1. "Slow shards are slow because they're producing" — **falsified** by shards 3-6 being
>    slow at *zero* spill.
> 2. "Both mechanisms stack on shard 7" — **falsified** by the normalised comparison above
>    (shard 6: 9× the producer load, 1.7× faster).
> 3. A per-thread attribution mapping tids to shards to derive KB/notice was
>    **retracted** — it failed an arithmetic check (cumulative read implied a swept count
>    that would have fired heartbeats which never fired).
>
> Only **self-labelled shard heartbeats** are used in this issue. The headline conclusion
> was unchanged by all three revisions, which is the reason to trust it: it does not
> depend on which mechanism dominates.

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

- ❌ **Byte-weighted stripes.** Two independent reasons it fails.

  **(i) Concentration.** A cohort concentrated in one id range still lands in one stripe
  however the boundaries are weighted — you cannot split a stripe's *plan membership* by
  choosing where to cut the id axis, because the members are contiguous in that axis.

  **(ii) The cost is non-stationary WITHIN a stripe.** Shard 7's own segment rates were
  **81 → 122 → 186 notices/s** — a **2.3× variation for the same worker on the same
  stripe**, as it crossed regions of differing record size and cohort density. Its spill
  over those segments went 3,771 → 6,737 → 8,349, i.e. it moved from a dense stretch into
  a sparse one mid-stripe.

  So a static weight is not merely *hard to calibrate ahead of time* — **there is no
  correct constant to calibrate to.** Any weight derived from a point-in-time rate, or
  from an average, is wrong by up to 2.3× for that very stripe at some point during its
  run. Perfect foreknowledge of stripe boundaries would not help; the cost varies inside
  the boundary.
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
