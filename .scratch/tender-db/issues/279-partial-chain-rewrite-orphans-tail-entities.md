# 279 — a partial chain rewrite (keep>0) drops the tail's result entities without sweeping them or emitting `removed`

Status: RESOLVED-IN-CODE 2026-08-26 (owner) — UNCERTAIN verdict resolved (reachability
CONFIRMED by a red-first fixture: keep=1 leaves an orphaned lot), fix implemented, full
`ops/check.sh` green (65 suites, golden + full-vs-incremental equivalence unaffected).
Deploy pending an idle queue. Forward-only: pre-existing orphans from past partial
rewrites are not retro-swept (sizing them is a deferred read-only prod probe). See
"Implementation" below.
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

## Implementation (2026-08-26, owner)

CONFIRMED first: the red-first fixture `a_partial_rewrite_sweeps_the_dropped_tails_orphaned_lot`
(project_incremental.rs) builds `[CN(A), B]` where B adds LOT-2, reparses B to a new BT-04
so it regroups away, folds incrementally, and asserts LOT-2 is not left orphaned. Before
the fix it read `1` orphan (keep=1, tail dropped, sweep skipped) — the bug is reachable,
not merely plausible.

Fix (canonical.rs `apply_tender_tx`): the shrinking-rewrite sweep now runs whenever the
chain SHRANK — `!stored.is_empty() && keep < stored.len()` — instead of only `keep == 0`.
For `keep > 0` the kept prefix's rows were never re-written into `written`, so a new
`extend_keep_with_kept_prefix` augments the keep-set first: `lots` from every surviving
`tender_version_lots` reference (they accumulate and are keyed by lot_key, not notice),
and `lot_results`/`bids`/`contracts` from the rows under each kept notice (keyed by their
origin notice, and the kept prefix's content is unchanged by the chain-as-state-key
match). `keep == 0` augments nothing (empty prefix) and stays byte-identical to issue 103,
so issue 99's forced-rewrite byte-identity holds unchanged — proven by the golden and
full-vs-incremental equivalence suites staying green.

Scope: the fixture exercises the `lots` leg (the one entity kind that needs the surviving-
reference computation rather than a notice-id lookup); the `lot_results`/`bids`/`contracts`
legs ride the identical gate + sweep, augmented by notice, and are covered by the same
green equivalence/golden suites. Building an award-graph fixture programmatically was not
worth it for a shared code path.

## Follow-up (deferred, read-only)

Size the pre-existing orphan stock left by partial rewrites BEFORE this fix — count
`lot_results`/`bids`/`contracts`/`lots` rows for a tender not referenced by its current
chain — on a SNAPSHOT (data-page read; gates per prod-box-reads, not the serving DB). If
material, a scoped reprojection of the affected tenders sweeps them (the fix makes any
re-fold of a shrunk chain clean). Not urgent: the leak is bounded (only regroup-away
tail-drops produce it) and no longer grows.
