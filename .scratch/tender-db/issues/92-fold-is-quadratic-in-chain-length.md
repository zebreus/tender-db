# 92 — `fold()` is O(chain² × state): latent, harmless today, fatal on a long chain

Status: open — LATENT. Confirmed quadratic by measurement 2026-08-02 (proj-fix), but costs ~0.1 s on
today's worst real cohort. Deliberately NOT fixed during the issue-85 re-fold: zero measured gain, and
`fold()` is byte-identity-critical (ADR-0001). Fix before a genuine 5k+ chain can reach it.
Kind: performance (latent) / robustness
Blocked by: —
Relates to: 91 (where this was investigated and ruled out), 85, ADR-0001 (byte-identity), ADR-0003

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
