# 136 — the ≥€10k negative-money residue: source-published, NOT a delta fault (#37 unblocked)

Status: **measured 2026-08-05**, 5 of 7 queries returned. Issue 134's hypothesis **falsified**.
Owner: sdk-vendor (measurement) → proj-fix (#37 re-spec)
Relates to: 134 (the delta hypothesis, falsified here), 131 (its account DOES apply, contrary to
my own prior reasoning), 36 (the ~72k parent population), 37 (unblocked), 121 (the probe)

## Provenance, stated because a result is only as good as its knowledge of its input

* snapshot `/data/db/snapshots/tender-db-1785916688.db`, **pinned by name**, never resolved as
  "newest"; header-complete (455,205,724,160 B declared = actual), 0h old at run time
* query file `canonical-verify/negative_money_407_triage.sql` at `a85ec24`, whose **reading key was
  committed before any of these numbers existed**, so the interpretation is not fitted to them
* run confined: `MemoryMax=512M IOWeight=10 CPUWeight=10 Nice=19`, unit `tdb-407-triage`
* **INCOMPLETE**: killed by `TimeoutStartSec=2400` after 39m59s. **Q5 and Q6 did not run.**
* raw journal captured to `canonical-verify/407-triage-2026-08-05.journal.txt`

## Q0 — the population reconciles, and the remembered number was ambiguous

| surface | rows ≥ €10k | most negative |
|---|---|---|
| `tender_version_bids.cents` | **224** | −32,309,312,012 |
| `tender_version_lot_results.awarded_cents` | **183** | −32,309,312,012 |

224 + 183 = **407**. The remembered figure was right *as a sum across both surfaces* — and it was
carried without saying that, which is why Q0 recomputed rather than assumed. Had it been one
surface's count, every downstream conclusion would have been about a different set.

**Magnitude: −32,309,312,012 cents = −€323,093,120.12.** That is not a plausible award value.

## Q1 + Q2 — issue 134's delta hypothesis is FALSIFIED, for every row

| | |
|---|---|
| negatives whose lot_result has **no earlier value at all** | **92 / 183** |
| of the 91 that do have one: **plausible deltas** (`prior + neg ≥ 0`) | **0** |
| of the 91: negative **exceeds** its prior value | **91** |
| exact sign-flips of the prior | 0 |

**Not one of the 183 is consistent with a delta mapped as an absolute.** Half have nothing to apply
a delta *to* — the story is impossible, not merely unsupported. The other half have a predecessor
and the negative is *larger than what was there*, and you cannot revise away more than existed.

This is the finding that **unblocks #37**: the re-spec is no longer at risk of calibrating a
threshold to admit a fold defect, because there is no fold defect here.

## Q3 + Q4 — the source published it, and it is ONE root cause seen twice

| | |
|---|---|
| ≥€10k awarded negatives | 183 |
| with an **exact magnitude match** in the chain's parse layer | **175** (95.6%) |
| with **no** negative anywhere in the chain | **0** |
| with a negative **bid on the same version** | **183 (100%)** |

**I was wrong about which arm produces these, and the data says so plainly.** I argued that #131's
account ("the source publishes negatives, the fold copies them faithfully") *could not* apply to
`awarded_cents` because it is either `direct_cents` or `single_currency_total(winning bids)` — a
computed sum, and "the source said so" cannot explain a sum. That reasoning was sound but its
premise was untested: 175 of 183 have an exact published counterpart, so for this population the
**published arm dominates**, and #131's account applies directly.

100% co-occurrence with a negative bid on the same `(tender, seq)` means bids and awarded are **one
finding, not two**. Fix the source-handling for bids and awarded follows.

## What this does NOT establish

* **"Source-published" is not "correct".** −€323M is implausible on its face. What the data shows is
  that the value is *faithfully carried*, i.e. **not our defect** — the same distinction insisted on
  for issue 84's 154 (held-but-unextracted ≠ lost). It is a data-quality finding about the source.
* **8 rows (183 − 175) have no exact published match** and are unaccounted for. Small, and
  deliberately left flagged rather than folded into the 175.
* **Q5 never ran** — profile/era clustering, i.e. issue 134's *second* question, "is the residue a
  distinct population or the tail of the benign mass?" Unanswered. A re-run needs a longer bound;
  the queries are I/O-bound (2m52s CPU over 40m wall).
* Q6 (specimens) never ran, so no rows have been read individually.

## Recommendation to #37

Proceed — the blocking risk is gone. But **the threshold should not be derived to admit these 407**.
They are implausible values that happen to be faithfully copied; a magnitude threshold tuned to let
them through would be calibrated to accept published garbage, which is a different error from the
one issue 134 feared but still an error. Keep them flagged as a source-quality finding.

## The probe result, recorded because it is the first clean one

`aborted=no`, gate ran **2385 s** — the eviction guard stayed **silent for 40 minutes on a real,
heavy payload**, and the live service was measurably unharmed:

| window | health p95 | hot read p95 | cache_file_delta |
|---|---|---|---|
| baseline | 0.6 ms | 21.7 ms | +616,947,712 |
| during | 0.6 ms | 21.6 ms | **+874,110,976** |
| after | 0.6 ms | 21.0 ms | +328,978,432 |

Live `MemoryCurrent` **grew** 2,480,537,600 → 3,455,791,104 during the scan. Gate cgroup peaked at
exactly 536,870,912 — pinned at its cap. A 455 GB-scanning workload ran 40 minutes inside 512 MiB
while the live service's cache *grew* by 874 MB and its latency did not move.

That is **half the split standard** the guard owes: proven to stay silent on a payload that does no
harm. It still needs the other half — firing on one that does.
