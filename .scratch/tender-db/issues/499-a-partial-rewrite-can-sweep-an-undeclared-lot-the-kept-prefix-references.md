# 499 — a partial rewrite can sweep a lot that only the kept prefix's results or bids reference

Status: needs-triage — SUSPECTED, not reproduced. Filed 2026-10-08 from a code reading by the issue 495 unit 2
golden's author (the store fold-writer golden avoided the shape on purpose so as not to pin it). NEXT: unit 1,
reproduce it in a store test.
Kind: correctness / the orphan sweep (`crates/store/src/canonical.rs`)
Relates to: 103 (the orphan sweep), 279 (`extend_keep_with_kept_prefix`), 495 (unit 2's identity cache reads the
same sets)

## Suspicion

On a partial rewrite (`keep > 0`), `extend_keep_with_kept_prefix` decides which of the Tender's entities
survive before `sweep_orphaned_entities` deletes the rest and announces each deletion as `removed`.

- **lots:** "still valid" means only `SELECT DISTINCT lot_id FROM tender_version_lots WHERE tender_id = ?`.
- **lot_results, bids, contracts:** those under a kept notice.

A lot can be referenced WITHOUT a `tender_version_lots` row. `result_lot` mints one when a result or bid names
a lot that no section declares (an "undeclared lot", e.g. LOT-0099). Such a lot is then referenced only by
`tender_version_lot_results.lot_id` and `tender_version_bids.lot_id`, and possibly by a group-member pair. Two
conditions together trigger the bug:

1. the only versions referencing it are in the kept prefix;
2. the rewritten tail does not carry that round forward.

The sweep would then delete the `lots` row while kept versions still point at it. The fold runs with FKs off, so
the version rows are left dangling, and a spurious `removed` reaches the feed. A pure tail drop (a shrinking
chain, nothing rewritten) is the most direct path.

## Evidence so far

- Prod, 2026-10-08, after the epoch-4 refold:
  - Four 100k-tender windows (1.0M, 4.0M, 6.9M, 8.7M) have 0 dangling `lot_id`s in `tender_version_lot_results`
    or `tender_version_bids`. The 224k window answered 408 and was not retried.
  - Expected: the refold rewrote every Tender with keep = 0, so no partial sweep has run since. Any damage
    would come from partial rewrites from now on.
- No test pins the shape.

## Units

1. **Reproduce.** A store test with:
   - v1 carrying a result on an undeclared lot;
   - v2 not carrying it;
   - a later re-key or removal so the chain shrinks to v1 (or a mid-chain insert after v1 whose tail drops the
     round).
   - Assert: the `lots` row survives, and no `removed` change row is written for it.
   Also try the group-member variant.
2. **Fix, if it reproduces.** Seed `written.lots` from every lot reference the kept prefix holds: the lot ids in
   `tender_version_lot_results`, `tender_version_bids` and `tender_version_lot_group_members` for the Tender, not
   only `tender_version_lots`. Then re-read the dangling-reference windows after the next week of daily folds.
