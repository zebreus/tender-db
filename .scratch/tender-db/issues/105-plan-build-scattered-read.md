# 105 — the incremental PLAN BUILD reads scattered, and is what makes the quarantine reprocess infeasible

Status: CLOSED 2026-08-25 (owner review) — premise no longer reproduces; the PlanRead sweep is
not built, deliberately. Re-open trigger below.
Kind: performance (projection Phase 1) — the reprocess blocker
Renumbered: filed as 97, moved to 105 — 97 collided with the nginx request-timing issue, which keeps the number.
Blocked by: — (independent of 95; see "Not blocked on the ParsedFold unknown")
Relates to: 91 (the Phase-2 half of the same defect, fixed), 94 (the sweep machinery this reuses),
96 (apply-phase degradation), 76/77 (the quarantine reprocess this gates), 58, 81

## The measurement

First full stage breakdown of an incremental fold, from the eForms-DE 1.x re-fold (2026-08-02,
218,635 changed → 473,094 planned notices, **34,634s total**):

| stage | time | scales with |
|---|---|---|
| pass-1 identity | 1,415.8s (23.6m) | `changed` — **6.5 ms/notice** |
| touched expansion | 18.4s | — |
| **pass-2 plan build** | **5,012.0s (83.5m)** | `all_ids` — **10.6 ms/notice** |
| grouping | 8.0s | — |
| retire regrouped | 344.8s | orphans (fixed in 93) |
| phase-2 fold + apply | 27,833.8s | pre-pass ~6h51m (fixed in 94) + apply ~53m |

With Group 1 deployed the pre-pass collapses and the plan build becomes the floor: pass-1 + pass-2 =
**~107 minutes that nothing in Group 1 touches**.

Scaled to the quarantine reprocess (2.42M notices, the main remaining recovery):

```
pass-1  2.42M × 6.5 ms  ≈  4.4 h
pass-2  ~3M   × 10.6 ms ≈  8.8 h
                          -------
                          ~13 h of plan build alone
```

That dwarfs a fixed pre-pass and is why **Group 1 is necessary but not sufficient** for the reprocess.

## Root cause — the same defect as 91, one phase earlier

Both passes read the parsed layer through `Db::parsed_by_ids`, which chunks the wanted ids into
`IN (512)` lists and issues one query per satellite table per chunk — index seeks scattered across
ten notice-keyed tables totalling hundreds of GB. Issue 91 removed exactly this pattern from Phase 2
by routing at the sequential bucketed sweep; the plan build still pays it.

The measured gap is large and consistent:

| read | measured |
|---|---|
| `parsed_by_ids` (scattered, pass-2) | **10.6 ms/notice** |
| `parsed_chunk_on` sweep (Phase-2 pre-pass, single-threaded, unbalanced) | **1.74 ms/notice** |

~6× per notice **before** any parallelism, and the sweep is shardable where the scattered read is not.

## Proposed change

Swap the read mechanism; keep the data identical. For a plan whose size crosses the existing
`INCREMENTAL_BUCKET_THRESHOLD`, drive both passes from an ascending-id **ranged sweep**
(`Db::parsed_chunk_on` over `[min(wanted), max(wanted)]`) and skip notices absent from the wanted set
in Rust — precisely what `write_shard` already does with `plan_group_keys`. Below the threshold,
`parsed_by_ids` stays: for a few-hundred-notice daily a range sweep would be far worse, the same
asymmetry issue 91 settled for Phase 2.

Cost model for a 3M-notice reprocess: sequential sweep of 14.2M notices at 1.74 ms ≈ 6.9 h
single-threaded, ≈ **1 h at 8 balanced shards** (issue 94's `parsed_id_stripes`), against ~8.8 h
scattered. The win grows with delta size, which is the direction that matters.

## The constraint that shapes the design — pass-2 CANNOT be parallelised

Pass-2 resolves Organization mentions as it sweeps, and `resolve_mentions` **creates** Organizations
in encounter order. Their surrogate ids are part of the projection's byte-identity surface (they
appear in `organizations`, `organization_mentions`, and every `tender_version_parties` row). The
existing code is explicit that this ordering is load-bearing: mentions resolve "in global id order so
org ids match a whole-delta pass".

So:

- **pass-1 is order-independent** — it only detects legacy notices and collects `new_keyed_keys`,
  which is sorted and deduped afterwards. It can be sharded exactly like the pre-pass.
- **pass-2 must stay serial and strictly ascending.** Sharding it would renumber every Organization.

That asymmetry is the main design finding. Parallelising pass-2 is possible only behind a separate
change that makes Organization identity independent of encounter order — out of scope here, and it
would need its own byte-identity argument.

## A semantic divergence the swap must NOT introduce

`parsed_by_ids` selects `FROM notices WHERE id IN (…)` with **no `parse_state` filter**.
`parsed_chunk_on` selects `WHERE parse_state = 'parsed' AND id > ? AND id <= ?`.

They therefore disagree about a notice that is in the wanted set but is no longer `parsed` — which
`notice_ids_for_tenders` can produce, because it returns `tender_versions.caused_by_notice_id` for
notices that were parsed when they were folded but may have been re-quarantined since (issue 87
territory). Today such a notice yields an empty `Parsed` and still gets a plan row; under a sweep it
would be skipped entirely, and its Tender would regroup or retire.

This is rare but it is a real behaviour change, and it must be settled deliberately rather than
discovered. Recommendation: **preserve today's behaviour** — sweep for the bulk, then fetch any
wanted id the sweep did not return via `parsed_by_ids`, so the plan is identical either way. The
reconciliation set is tiny by construction, and it keeps the change purely about read mechanics.

## Byte-identity gate strategy

The output surface is the plan rows, the Organizations and their ids, and the mentions — everything
downstream is a pure function of those.

1. **A `PlanRead` selector**, mirroring `Phase2`: `ByIds` (today) and `Sweep`, with
   `project_incremental_chunked_phase2`-style forcing so tests can drive both. The routing default
   picks by plan size.
2. **The load-bearing test**: the same delta absorbed three ways — full non-rebuild projection,
   incremental via `ByIds`, incremental via `Sweep` — asserting a byte-identical canonical layer,
   surrogate ids included. `project_incremental.rs::snapshot` already digests `organizations` (with
   ids) and `organization_mentions`, so Organization renumbering — the specific hazard here — is
   caught. This mirrors `incremental_bucketed_fold_matches_parsed_fold_and_full`.
3. **Sparse-delta test**: a delta scattered thinly across a wide id range, so the sweep reads many
   notices it must discard. Correctness under maximum wasted reads.
4. **Mixed-parse-state test**: a wanted notice that is NOT `parse_state='parsed'`, pinning the
   divergence above — with the reconciliation fetch in, `Sweep` must equal `ByIds`.
5. **Fail-on-old-behaviour check** for each new gate, as with 93/94: a test that cannot fail is not a
   gate. In particular test 2 must be shown to fail if pass-2 is sharded (the Organization-order
   hazard), so the constraint is enforced by the suite and not just by a comment.

## Not blocked on the ParsedFold unknown (issue 95)

The unexplained 5h07m CPU-bound zero-I/O stall was on the **group_key-ordered** Phase-2 read.
Pass-1 and pass-2 read in **ascending id order** and have never stalled — they completed at 6.5 and
10.6 ms/notice in the very run that stalled elsewhere. This change moves them further from that
shape, not closer: an ascending ranged sweep is the pattern the pre-pass already runs at scale.

## Sequencing

After Group 1 deploys and is proven, before Group 2. Group 2's fold is corpus-wide and would
otherwise pay this ~13h plan build in full.

## Deliberately out of scope

`Ident::read` needs only a handful of fields (BT-04 / folder id, OJS refs, subtype, the instants,
source, profile), yet pass-1 materialises each notice's **entire** parsed form to compute it. A
targeted projection of just those field ids would likely beat both read mechanisms for pass-1 by a
wide margin. That is a larger change with its own correctness surface; noted here so it is not lost,
but it should not ride along with a mechanical read swap.

## Owner review (2026-08-25): the measured premise no longer reproduces — close without building

The design's floor was the 2026-08-02 stage breakdown: pass-2 at 10.6 ms/notice,
~107 min of plan build per large fold, ~13 h scaled to the quarantine reprocess.
That world is gone. The most recent large incremental fold — closing fold 334
(2026-08-23), 458,572 changed notices → 352,196 Tenders — ran **end to end in
1,172.8 s ≈ 2.6 ms/notice**, plan build included, on a delta 2.1× the size of
the run that produced the 10.6 ms figure. The Group-1 line (91/93/94), 243's
merged reads, and the general read-path work have already collapsed the cost
this design existed to remove; building the PlanRead selector now would add a
second read mechanism, a routing threshold, and four byte-identity gates to
recover time the pipeline no longer spends.

The motivating workload is also mostly behind us: the 2.42M quarantine
reprocess happened piecewise through the 2026-08 campaigns (100/139/196/244),
each absorbed at post-Group-1 speeds.

**Re-open trigger, so this closure cannot rot silently:** issue 65's job
phase/progress record now exposes per-stage timings on every fold. If a future
large fold's record shows the plan-build phase re-dominating (≳ half of wall
time on a ≥100k-notice delta), re-open with that breakdown — the design here,
including the pass-2 ordering constraint and the parse-state divergence remedy,
remains correct and ready; only its economics failed review.

The out-of-scope note stands on its own merits and is worth keeping visible:
pass-1 materialises the entire parsed form to read a handful of Ident fields.
If pass-1 ever dominates a stage breakdown, a targeted field projection is the
first thing to try — smaller than this design, no byte-identity surface.
