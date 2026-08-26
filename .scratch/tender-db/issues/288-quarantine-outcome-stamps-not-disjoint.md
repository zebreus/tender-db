# 288 — quarantine outcome stamps are not disjoint: a row can carry BOTH `reprocessed_at` and `skipped_at`, and `quarantine_resolution` double-counts it

Status: CLOSED 2026-08-26 (owner) — all three edges fixed, red-first proven (0/3 tests pass without the fixes, 3/3 with), full gate green (66 suites). DECISION: reclaimed WINS — see Resolution below. DEPLOYED to prod (rev 89f6e4e, /health green, queue idle). Unblocks 303.
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

## Resolution (2026-08-26, owner — ultracode loop)

**Decision on the open question: reclaimed WINS.** A skip is a policy statement
("we chose not to process this"); a reclaim is the fact that the content now lives
in the parsed layer. So a later genuine reclaim of a skipped row flips it to
reclaimed, clearing the stale skip — a missing skip marker is honest, a skip marker
on restored content is not.

Three edges landed (crates/store/src/lib.rs):
1. `flag_skipped_members`: `AND q.reprocessed_at IS NULL` — the skip path never
   stamps a reclaimed row.
2. All SEVEN reclaim stamps (stamp_reclaimed's four addresses + the None-arm's
   three) now `SET reprocessed_at = ?, skipped_at = NULL, skipped_reason = NULL` —
   the flip, in the same statement.
3. `quarantine_resolution`: disjoint CASE arms, reprocessed wins — identical rule to
   `quarantine_counts_by_reason_split`, so the resolution card and the dashboard
   header can never disagree, INCLUDING about historical both-stamped rows (which
   makes a data backfill unnecessary — display is consistent without one).

Red-first tests (`crates/store/tests/quarantine_outcome_disjoint.rs`): the skip
guard, the reclaim flip (driven through the real `reclaim_notice` None-arm with a
real fetch row), and resolution counting a both-stamped row once — all three fail
without the fixes, pass with them.

**Completeness sweep (adversarial agent, verified conclusions):** every other write
to `reprocessed_at`/`skipped_at` in the codebase is safe — `mark_skipped_siblings`
guards both columns (SKIPPED_SIBLING_SCOPE), the two quarantine INSERTs never
pre-set stamps, `stamp_still_held`/`record_reclaim_attempt` write neither column,
`repair_swept_siblings`/`unmark_skipped_siblings` are clear-only, and no
delete/re-insert/upsert path exists that could resurrect a cleared stamp. With this
unit, every writer preserves the disjointness invariant. (One nugget for the
record: `repair_swept_siblings` skips both-stamped rows — irrelevant now that reads
are disjoint.)

**Prod audit (2026-08-26, /v1/sql bounded read): 0 both-stamped rows exist.** The
reachable scenario had not yet occurred in production — the fix landed preventive,
and the two dashboard surfaces never actually contradicted. CLOSED.
