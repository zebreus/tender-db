# 495 — a stale refold deletes and re-inserts the whole corpus, though more than 95 % of it is byte-identical

Status: ready-for-agent — filed 2026-10-08 from the owner's "why is the refold so slow? do we need to defrag?"
(diagnosis `wf_805a60a7-d9b`, saved with its raw data in `../495-refold/`). NEXT: unit 1, the design. The
change-feed meaning is the decision inside it.
Kind: performance / projection (`apply_tender_tx`, `crates/store/src/canonical.rs`)
Relates to: 179 (scoped staleness; it rejected rebuild=true), 488 (the defrag, which was not the cause),
496 (writer per-row cost), 497 (planning and grouping regressions), 340 (the in-place backfill precedent),
490 (the epoch-4 refold that prompted this)

## What was measured

The last identical all-profile refold, job 2044 (2026-10-07, epoch 3), took 11 h 06 m end to end:

| Stage | Time |
|---|---|
| Requeue and stamp | 48 m |
| Plan | 2 h 48 m |
| Grouping | 25 m |
| Pre-pass | 29 m |
| **Fold** | **6 h 35 m** |

The fold is the largest share. An epoch bump makes every Tender stale (keep=0), so all 8.78M are deleted
and re-inserted in full:

- 1,117,908,878 leaf rows deleted and inserted again;
- about 6 B B-tree changes, about 1.7 B of them at random positions;
- 70,643,598 change rows, because `append_version_changes` re-emits the whole history.

For a fix like 484 or 490, more than 95 % of those rows come back byte-identical.

The single writer thread is on the CPU 84.5 % of the time (88 % of that in user space) and blocked
15.4 %. Cost scales with leaf rows: from 17.5 µs per row on text-heavy eForms buckets up to 30–46 µs on
legacy buckets heavy in parties and classifications. Job 2067 (epoch 4, today) tracks job 2044 within
seconds, bucket by bucket.

The filesystem is not the cause:

- The extent count is 1,947,070, against 1,946,893 right after 488's defrag.
- `xfs_iext_lookup_extent` is 0.12 % of samples.
- The defrag made the same fold at most 4.6 % faster. That 4.6 % also includes removing the reflink
  snapshot and the 484 deploy.

## Direction

Skip the rows that have not changed, at **version × table** grain. A per-version digest does not pay
off: a 490-type change touches `tender_version_lots` on most versions.

Two shapes:

- **(a) Stored digest.** Store a digest per (version, table) and compare before deleting. It needs a
  schema column, and the first refold after it ships still pays full price.
- **(b) Read and compare.** Read the stored rows and compare them with the new ones before deleting. No
  schema change. It still reads about 1.1 B rows through the same writer, so its gain is unmeasured.

Either digest must cover the derived columns (EUR, the stored lot value from 490) and the reused ids.
A wrong "unchanged" leaves stale content in place and raises no error. That is the main risk.

**The change feed is the decision inside this issue.** Today the only way a feed consumer learns what a
refold corrected is that the whole history is re-emitted. `append_version_changes` compares new versions
with each other, never with what was stored. So:

- Skipping a Tender whose rows did not change is safe.
- A Tender whose rows did change needs a new "changed versus stored" emission. Without it, a 484-type
  fix (`is_buyer`) never reaches `/v1/changes` or SSE subscribers, and nothing reports an error.
- The documented feed behaviour (issue 179; `docs/operations.md` around lines 929–931) changes either
  way.

**Cheaper route for fixes that only add or derive a column:** update rows in place with a backfill, as
`backfill-original-lang` did (issue 340: 7.9M Tenders in 70 min, at one commit per row).

- 490's lot value could have been derived from stored rows alone, because REST already computed it at
  read time.
- That would have avoided an 11 h refold and 70M re-emitted change rows.
- The epoch-4 refold was chosen on purpose, because the epoch doubles as the completeness marker.
- Even so, the runbook should offer this route first, before reaching for an epoch bump.

## Units

1. **Design.**
   - Choose (a) or (b).
   - Define the grain.
   - Define the "changed versus stored" feed semantics, with an ADR, since this changes what
     `/v1/changes` means after a refold.
   - Add the backfill-first rule to the runbook (`docs/operations.md`, the refold section).
2. **Build, behind a fold test.** v1 is folded; refold with no logic change; expect 0 rows rewritten and
   0 change rows. Then refold with a change to one table; expect only that table's rows rewritten, and
   one "changed versus stored" row.
3. **Measure** on the next all-profile refold against job 2044's 6 h 35 m fold.
