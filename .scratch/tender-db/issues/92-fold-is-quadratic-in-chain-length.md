# 92 — `fold()` is O(chain² × state): latent, harmless today, fatal on a long chain

Status: open — LATENT, INSTRUMENTED, DEFER CONFIRMED BY MEASUREMENT (step 2 answered
2026-08-28 from the epoch refold's real execution — see the bottom section). The worst real
chain (3,282 versions, tender 5785085) folded inside a 50k-version chunk that took 488s vs a
52s median — minutes, not the fatal zone; absolute worst chunk anywhere in the corpus was
999s. The `longest_chain` ≥ 4,000 weekly tripwire is the standing signal-producer; the
rewrite stays deferred until it flags. Was: LATENT ON THE CLOCK (2026-08-26 re-measure:
chain 3,282, 6,468 accumulating results); before that ~0.1s worst cohort (2026-08-02).
Kind: performance (latent) / robustness
Blocked by: —
Relates to: 91 (where this was investigated and ruled out), 85, ADR-0001 (byte-identity), ADR-0003

## Verify

    ssh -o BatchMode=yes root@zebreus.click "/root/aj.sh /admin/reports/data-quality" | python3 -c 'import sys,json; b=json.load(sys.stdin)["body"]; s=b[b.find("== 9."):]; print(s.split("\n")[1].strip())'

- **done** (deferral holds): `longest chain: N (flag threshold 4,000; …)` with N under 4,000 — read 2026-09-19: `longest chain: 1,876`
- **open** (reopen): N at or over 4,000 — the O(chain²) fold is within reach of a chain that would hurt

## The defect

`fold(chain)` in `crates/ingest/src/project.rs` rebuilds each version by **cloning the previous
version's entire accumulated state**:

```rust
let mut facts = previous.map(|p| p.facts.clone()).unwrap_or_default();
let mut lots  = previous.map(|p| p.lots.clone()).unwrap_or_default();
let mut rounds = previous.map(|p| p.rounds.clone()).unwrap_or_default();
…
rounds.push(round.clone());     // results ACCUMULATE — never superseded
versions.push(TenderVersion { … });   // and every version is retained
```

`facts` and `lots` are bounded by supersession (a republished field replaces the carried one), so those
clones are O(state) per step — already O(N × state) overall. **`rounds` are additive**: a framework/DPS
round or tranche CAN adds a round and never removes one (ted-empirical-checks.md §1). So version *i*
carries *i* rounds, and the chain materialises **N²/2 round copies** — quadratic in both time and memory.

## Measured (release build, synthetic DE-1.x-shaped states)

```
typical DE1 CAN (40 tender facts, 2 lots, 2 lot_results/bids/contracts)
  chain    50   0.003s        chain   400   0.076s        chain  1600   0.985s
  chain   100   0.007s        chain   800   0.252s
  per-doubling cost x~2.0  =>  clean O(N^2)

large DE1 CAN (120 facts, 20 lots, 20 of each result entity)
  chain  1600   9.412s        (0.14 ms/notice at N=50  ->  5.88 ms/notice at N=1600)
```

## Why it is harmless today

Direct scan of `/data/archive/doe` (read-only, no DB), 104,581 eForms-DE 1.x notices, extracting every
`cbc:ContractFolderID`:

- distinct folder uuids **81,646**; keyed notices 102,622
- **max notices per key = 42**; 66,698 keys are single-notice, 11,480 are 2, the tail decays to 42
- no folder id: 1,959 — **non-uuid: 0**

⇒ Σ N² over the whole cohort ≈ 3×10⁵ units ⇒ **total fold cost ≈ 0.1 s for all 218,635 notices.**

This is why the quadratic was *not* the cause of the 2026-08-01 stall, despite the extrapolation
matching the cohort size almost exactly (5h07m ⇒ a single chain of ~218,700). The memory footprint
independently forbids it: a 218k chain needs N²/2 round copies — terabytes — and the process peaked at
4.7 GB.

## When it WILL bite

Any grouping regime that can produce a chain in the thousands. The realistic candidate is a **legacy OJS
transitive component**: `build_plan_groups` unions the whole edge graph, and the last rebuild reported
**10,981,536 union-find nodes / 10,966,375 legacy notices**. A single large component becomes one
`ojs:` group whose whole notice set is one `fold()` chain — and `next_plan_batch` never splits a group,
so it arrives in one batch regardless of the notice budget. At N = 10,000 this is ~40 s and growing
quadratically; at N = 50,000, ~16 min and ~gigabytes of round copies.

Worth checking as part of the fix: the actual max `ojs:` component size in the current plan (it was never
measured — `plan_summary` counts distinct legacy keys, not their sizes).

## Fix sketch

Do not clone the accumulated state per step. Either:

1. Build the versions by carrying **one** mutable running state and snapshotting only what
   `apply_tenders` actually writes per version (it writes rows, not the Rust structs) — i.e. emit each
   version's rows as the chain is walked rather than materialising N full `TenderVersion`s; or
2. Keep `rounds` as an immutable persistent list / `Arc` slice shared across versions, so appending is
   O(1) and versions share the prefix.

(1) is the deeper fix and also removes the O(N × state) memory; (2) is smaller and kills the quadratic
term alone.

**Gate: byte-identity.** `fold()` output feeds `apply_tenders` in global fold order and every surrogate
id depends on it. `project_golden`, `project_equivalence`, `project_fold_source` and
`incremental_bucketed_fold_matches_parsed_fold_and_full` must all stay green, and the fix should add a
perf assertion in the shape of the table above (cost per doubling must stay ~1.0, not ~2.0).

## Owner re-measurement + decision (2026-08-26)

Re-measured the trigger against the LIVE corpus (bounded `/v1/sql`, prod
`2ec16db`) — the deferral rested on "~0.1s on today's worst real cohort", and
that number has moved:

* **Longest chain is now `current_seq` = 3,282** (tender 5785085, a TED DPS,
  "Junior Financieel Adviseur"), up from the sub-thousand cohort this issue was
  filed against. 3 tenders ≥ 2,000, 15 ≥ 1,000, 47 ≥ 500; mean 1.81.
* That tender's **head version carries 6,468 accumulated `lot_results`** (≈1.97
  per version, monotone) — this IS the `rounds`-accumulate quadratic shape, not
  the bounded facts/lots shape. A full fold of it clones ≈ Σ2i ≈ 3,282² ≈ **10.8M
  result-copies** for one tender.
* It is a **DPS/framework**: a new mini-competition notice arrives periodically
  and grows the chain without bound. When one lands, the daily incremental
  re-folds this tender's *entire* chain at O(N²) (the Buckets path calls
  `fold()` on the full touched chain — 90/91). So the cost is real, per-arrival,
  and strictly increasing.

**Decision (owner): STILL DEFER the fold rewrite, but the posture changes from
"latent, forget it" to "latent, on the clock, instrumented."** Rationale:

1. The rewrite is byte-identity-critical (ADR-0001; four golden/equivalence gates)
   and touches the hot apply path — not a 03:00 speculative change, and a
   3,282-fold is still seconds, not the minutes that would threaten the daily
   chain today. Building it now trades a real regression risk against a latent
   one. No.
2. BUT "wait for a signal" had no signal-producer. The missing number is the
   REAL fold time for the 3,282 tender (everything above is modelled from the
   synthetic table). **Next concrete step:** measure `fold()` wall-time for
   tender 5785085 against the `/data/db/snapshots` copy in a dedicated window
   (not the serving DB) — that single number decides build-vs-defer with
   confidence and replaces the extrapolation.
3. **Cheap tripwire to add:** a "longest chain" line in the weekly data-quality
   report (it already full-scans; one `MAX(current_seq)` aggregate is ~free
   there) that flags at ≥ 4,000, so the approach to the fatal zone is visible
   weekly instead of discovered inside a slow fold. Filed as the follow-up
   rather than built here so the DQ-report change lands as one reviewed unit.

   → **BUILT 2026-08-27** (rode the ADR-0014 D5 read-surface unit): a
   `longest_chain` whole-corpus query in the DQ run (streaming MAX over
   `tenders`, no hash state), report section 9 with the ≥ 4,000 FLAG line,
   `longest_chain` in the headline history, and a `tender_db_dq_longest_chain`
   gauge on /metrics (absent for pre-field stored runs, never a fake 0). First
   number lands with the first post-deploy weekly run. Step 2 (the real fold
   wall-time for tender 5785085 on a snapshot) remains open and is still the
   build-vs-defer decider.

The fix sketch below is unchanged and correct; option (2) (persistent/`Arc`
`rounds` list) is the smaller byte-identity-safe lever and remains the
recommended first cut when the measured number says build.

## Step 2 ANSWERED (2026-08-28) — measured from the epoch refold's real execution

Instead of a dedicated snapshot fold of tender 5785085, the number came free from
production: the 2026-08-27 epoch refold's journal prints a phase-2 heartbeat every
50,000 versions written, so inter-heartbeat wall-time gaps ARE per-chunk fold+write
cost on the real corpus, real hardware, real chains. Analysis over 286 intervals:

```
median gap: 52s
worst gaps (sec, tender-id range covered):
  999s   tenders   928,821 -> 951,038
  488s   tenders 5,756,578 -> 5,779,585   <- the chunk whose fold reaches tender 5785085
  411s   tenders 2,785,371 -> 2,820,075   <- the 8.9M-lot_results mega-chain window (306)
  301s   tenders   241,525 -> 263,507
  285s / 254s / 239s ...
```

Reading:

* The chunk containing the 3,282-version exemplar (5785085) cost **488s ≈ 9× median**
  for its 50k versions — the worst chunk anywhere was **999s ≈ 19× median**. Even the
  heaviest chain-cohorts are **minutes per 50k-version chunk**, comfortably inside the
  issue's own bar ("seconds-to-minutes, not minutes-threatening-the-daily-chain").
* These gaps CONFLATE fold CPU with satellite write volume — the 411s window writes
  8.9M lot_results rows (see 306) — so attributing the whole gap to fold() OVERSTATES
  the quadratic's cost. The true fold share is smaller than these numbers.
* The daily incremental re-folds one touched chain, not a 50k chunk; its worst case is
  a fraction of 488s. No observed daily-chain latency incident has implicated fold.

**Verdict: DEFER stands, now on a measured production number rather than synthetic
extrapolation.** The build trigger is unchanged and instrumented: the weekly DQ
`longest_chain` line flags at ≥ 4,000 (current 3,282). When it flags, option (2)
(`Arc`/persistent `rounds`) is the first cut, behind the four byte-identity gates.
Step 2 is closed; nothing on this issue is actionable until the tripwire fires.

## Instrument re-read 2026-09-02 — the deferral holds, and now with a series

This issue defers the rewrite until the `longest_chain ≥ 4,000` tripwire flags.
Three of this session's parked issues (48, 68, 169) turned out to rest on stale
premises, so the same check was applied here — by reading the instrument rather
than assuming it.

`data-quality-headlines` carries its own run series:

```
2026-08-22 00:24   longest_chain = (absent)
2026-08-23 04:50   longest_chain = (absent)
2026-08-24 02:09   longest_chain = (absent)
2026-08-27 05:19   longest_chain = (absent)
2026-08-27 19:57   longest_chain = 3282
2026-08-28 08:58   longest_chain = 3282
2026-08-30 02:30   longest_chain = 3282
```

**Flat at 3,282 across every reading it has, against a threshold of 4,000.** The
gauge appears to have been added around 2026-08-27, which is why the four earlier
runs are absent rather than zero — worth knowing so nobody reads those as a jump
from nothing.

### Conclusion: nothing to do, and this time that is measured

Unlike issues 48, 68 and 169, **this issue's premise has not drifted.** The
deferral was conditioned on a tripwire, the tripwire exists, it is being computed,
and its value is stable. That is the deferral working as designed rather than
being forgotten.

Two honest limits on the above: three readings over three days is a short series,
and `data-quality` runs weekly (last 2026-08-30 02:30, next Sunday 2026-09-06), so
this is checked-and-stable rather than proven-flat. No action, no re-triage.

The wider note this session earned: **a deferral is only as good as whether anyone
ever re-reads its condition.** Three of four parked issues had gone stale without
anyone noticing. This one had not — because it was deferred against a *computed
signal* instead of against a remembered number. That is the difference worth
copying.
