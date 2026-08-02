# 98b — pre-pass stripes balance notice COUNT, but per-notice read cost rises ~4.7× with id

Status: proposed
Kind: performance / follow-up refinement
Design owner: proj-fix
Relates to: 94 (balanced stripes — this is the second-order residual), 66 (the sharded pre-pass itself), 96 (apply-side variability), 76 (quarantine reprocess — the beneficiary)

> Numbered 98b to avoid colliding with the in-flight issue 98 (DE-1.x org refs).
> Renumber on triage if the tracker prefers.

## Context

Measured live on prod during the 2026-08-02 eForms-DE 1.1+1.2 re-fold (job 4, rev
`33dfba7`), the first at-scale exercise of issue 94's balanced striping.

**94's core fix works and is not in question here.** The pre-pass reported

```
[project] phase 2 pre-pass: 8 shard(s) over notice ids (177, 27297321] (99% of the id space)
```

— 8 real stripes, sized by notice count (visibly *not* by id width: shard 0 spans 2.23M
ids, shard 1 spans 1.77M, shard 2 spans 13.86M, all holding comparable notice counts).
All 8 workers ran concurrently for the whole phase; none idled at DONE; the catastrophic
1× collapse of the previous night did not recur.

## The residual: equal notice counts ≠ equal time

Per-shard heartbeats, ~12 min in:

| shard | id range | notices/s | spilled |
|---|---|---|---|
| 0 | 177 – 2,232,185 | **1,722** | 20 |
| 1 | 2,232,185 – 4,004,574 | **1,494** | 0 |
| 2 | 4,004,574 – 17,861,049 | **722** | 0 |
| 3 | 17,861,049 – 19,629,867 | **359** | 0 |

**A monotonic ~4.7× decline in throughput as notice id rises.**

Per-worker cumulative reads at the same moment spread only **2.9×**
(9,885 MB … 3,359 MB), while notices swept spread **≥4.4×** — i.e. the slow workers both
read fewer bytes per second *and* extract fewer notices per byte.

### The cause is the data, not the workers

Two observations rule out per-worker artefacts:

1. **Shard 2 slowed from 1,309 → 722 notices/s *within its own stripe*** as it advanced
   to higher ids. Same thread, same code, same device — only the ids changed. That is a
   **within-shard gradient**, so it cannot be scheduling, reader-pool contention, or
   stripe assignment.
2. **Shard 3 is 4.7× slower than shard 0 while having spilled ZERO.** It performs no
   resolve/encode/spill work at all, so plan density (the obvious first guess, and the
   one initially proposed) is falsified as the mechanism for the slow shards. Only
   shard 7, at the very top of the id space, spills materially.

The remaining explanation consistent with all of it: **per-notice read cost rises with
id.** The high-id range is recent eForms, whose records are substantially fatter and
span more pages/sections than the legacy TED records dominating the low-id range.

## Consequence

`parsed_id_stripes` equalises **notices per stripe**, so a stripe of cheap old notices
finishes long before a stripe of expensive recent ones. The `join` barrier waits for the
slowest, so the fast workers idle.

On this run shards 0 and 1 were ~56% through their allocation at 12 min (finishing in
~20 min) while shards 3-7 were under 14%. Effective parallelism decays toward the count
of slow shards — delivering roughly **5-6×** of the theoretical **8×**.

That is a real win versus the pre-94 effective 1× (6h42m → ~1.3h on this run), and this
issue is strictly a refinement on top of it — **not a regression, not blocking**.

## Proposal

Stripe by **estimated read cost**, not raw notice count. Options, cheapest first:

- **(a) Byte-weighted stripes.** Size stripes so each holds a comparable sum of
  `length(raw_xml)` (or whatever per-notice size column is cheapest to aggregate) rather
  than a comparable row count. A coarse histogram over id ranges is enough — the goal is
  removing a 4.7× skew, not perfection.
- **(b) Profile-weighted stripes.** Cheaper proxy: weight each notice by a per-profile
  constant (eForms ≈ N× legacy TED), calibrated once. Avoids scanning sizes at all.
- **(c) Work-stealing.** Leave striping alone; let a finished worker claim unswept id
  ranges from the slowest. Removes the barrier idle entirely and is robust to any future
  cost skew, at the cost of coordination between workers.

(c) is the most general and the only one that self-corrects for skews we haven't measured
yet — worth considering given the reprocess's cohort may have a different cost profile
again.

## Why this matters — the reprocess

The quarantine reprocess (issue 76) targets **2.42M notices**, concentrated in exactly
the id ranges that are most expensive per notice. A 4.7× cost gradient with count-based
striping means the slowest stripes dominate, so the run costs materially more than
`total_work / 8`. Sizing that work off this run's aggregate throughput would
under-estimate it.

## Acceptance

- Per-shard `notices/s` within a small factor of each other across the whole id space, or
- fast workers no longer idle at the barrier (under (c)), and
- the reprocess sized against a measured per-notice cost curve rather than a flat average.

## Comments

Filed 2026-08-02 from live prod measurement during the DE-1.x re-fold. The plan-density
hypothesis was proposed first and **falsified** by the zero-spill readings on shards 3-6
before this was written; the record recorded here is the surviving explanation, not the
first one.
