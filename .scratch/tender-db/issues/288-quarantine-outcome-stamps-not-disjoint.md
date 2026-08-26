# 288 — quarantine outcome stamps are not disjoint: a row can carry BOTH `reprocessed_at` and `skipped_at`, and `quarantine_resolution` double-counts it

Status: DIAGNOSED (2026-08-26, owner — adversarial reclaim review; write-side crux verified against the code by the owner)
Kind: correctness (ledger honesty — dashboard surfaces disagree)
Severity: LOW-MEDIUM (no data loss; the resolution card and the header can contradict each other)
Relates to: 87 (stale-state-after-partial-reclaim class), 190 (skip flags), 196/181 (whole-file rows this shape needs)
Found by: the 2026-08-26 adversarial reclaim/quarantine review.

## The gap (two halves, one invariant)

The schema documents three DISJOINT outcomes for a quarantine row (outstanding /
reclaimed / skipped), and `quarantine_counts_by_reason_split` (lib.rs ~2474)
enforces disjointness in its CASE arms ("a row that somehow carried both is
counted as RECLAIMED"). But:

1. **Write side (the root):** `flag_skipped_members` (lib.rs ~2651) guards only
   `q.skipped_at IS NULL` — NOT `reprocessed_at IS NULL` — so it can stamp
   `skipped_at` onto an already-RECLAIMED row. VERIFIED against the code
   (2026-08-26, owner): the UPDATE's WHERE carries `fetch_id`, `skipped_at IS
   NULL`, `member_path IN (...)` and no reprocessed guard. The reclaim stamps
   have the mirror gap (they filter `reprocessed_at IS NULL` but not
   `skipped_at IS NULL`). The bulk backfill `mark_skipped_siblings` is already
   safe (`SKIPPED_SIBLING_SCOPE` includes `reprocessed_at IS NULL`); only the
   reprocess-time path is unguarded.
2. **Read side (the symptom):** `quarantine_resolution` (lib.rs ~2949) computes
   `reclaimed = SUM(reprocessed_at IS NOT NULL)` and `skipped = SUM(skipped_at
   IS NOT NULL)` with no mutual exclusion — a both-stamped row is counted in
   BOTH buckets, so `reclaimed + skipped + outstanding` exceeds the bucket's
   row count, while the header total (the disjoint counter) counts it once.

## Reachable scenario (from the review)

A whole-file `not-utf8` row for file F (member_path=F, notice_id NULL). Reclaim
run 1: record F#1 parses → the by-file arm stamps `reprocessed_at` on the F row
(F#2 still held under its own row). Run 2 re-walks F (F#2 still held; the held
walk strips `#ordinal` back to F); a dispatch policy now declines the whole file
→ `flag_skipped_members` stamps `skipped_at` on the F row (its skipped_at is
NULL). Both stamps on one row; the resolution card and the header disagree.

## Fix direction

Make the write side enforce the invariant AND the read side tolerate history:
- `flag_skipped_members`: add `AND q.reprocessed_at IS NULL`.
- The reclaim stamp arms: add `AND skipped_at IS NULL`? NO — decide first:
  a skipped row that a later reclaim actually restores should arguably flip to
  reclaimed (the stronger, true outcome). If so, the reclaim stamps should
  CLEAR `skipped_at` when they stamp `reprocessed_at`, rather than skip the row.
  Decide which way, then pin it with a test.
- `quarantine_resolution`: make the CASE arms disjoint exactly like
  `quarantine_counts_by_reason_split` (reprocessed wins).

Pin with tests: a row stamped reclaimed then hit by flag_skipped_members stays
reclaimed-only; resolution == split counts on a mixed fixture.
