# 279 — a partial chain rewrite (keep>0) drops the tail's result entities without sweeping them or emitting `removed`

Status: DIAGNOSED (2026-08-26, owner — exploratory review; verifier verdict UNCERTAIN, plausibility high)
Kind: correctness (storage leak + change-feed honesty)
Severity: MEDIUM
Relates to: 103 (fixed the keep==0 full-rewrite orphan case; this is the distinct keep>0 case), 164 (missing `removed` events), 236 (BT-04 instability is the trigger)
Found by: the 2026-08-26 fresh-eyes projection review.

## The gap

In `apply_tender_tx`, when a chain changes but a prefix still matches (`keep > 0`),
the tail is dropped by
`for seq in (keep+1..=stored.len()).rev() { self.delete_version(...) }`
(canonical.rs ~3907). `delete_version` (canonical.rs ~4116) removes only the
version-scoped satellites and the `tender_versions` row — it does NOT delete the
entity-table rows `lot_results` / `bids` / `contracts` / `lots` (keyed by
`(tender_id, notice_id)`), and emits no change events.

The one place that both deletes those entity rows and emits `removed` events —
`sweep_orphaned_entities` (canonical.rs ~4152/4186) — is gated
`if keep == 0 && !stored.is_empty()` (canonical.rs ~3942). Issue 103 fixed the
keep==0 full-rewrite path; the **partial-rewrite (keep>0) path never sweeps and
never announces the departed notice's entities.**

## Failure scenario

Tender T (keyed K) has chain `[CN(A), CAN(B)]`; award B produced
bids/contracts/lot_results under `(tender_id=T, notice_id=B)`. B is reparsed and
its BT-04 changes (eForms instability, issue 236) so it regroups to K2. On the
incremental fold of T the plan chain is `[A]`: `keep = 1`; `delete_version` drops
seq-2 junction rows, but B's `lot_results`/`bids`/`contracts` under T remain,
referenced by nothing. `keep != 0`, so the sweep is skipped: T keeps orphaned
result rows forever (storage leak) and any `/v1/changes` subscriber tracking those
ids is never sent a `removed` event, while the same entities are re-minted under
K2.

## Why UNCERTAIN (what to confirm before fixing)

The verifier could not fully confirm that `keep > 0` with a shrunk tail is
actually reachable for a *keyed* tender under the incremental fold (vs always
falling into keep==0 when the chain is rewritten). Confirm with a fixture: build
`[CN(A), CAN(B)]`, reparse B to regroup away, fold, and assert whether the code
takes keep=1 (bug reachable) or keep=0 (already swept). If reachable, extend
`sweep_orphaned_entities` to run whenever `keep < stored.len()` (the chain shrank),
tracking the complete new-chain reference set in `written`, and emit `removed`
events for the dropped-tail entities.

## Acceptance

Fixture reproduces the keep>0 shrink; after the fix the orphaned entity rows are
gone and `removed` change rows are emitted; projection golden/equivalence green;
a bounded prod probe sizes any existing orphans (count of `lot_results`/`bids`/
`contracts` rows whose `(tender_id, notice_id)` is not in that tender's current
chain).
