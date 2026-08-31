# 323 — The re-homing re-queue is a 20-minute write transaction

Status: FIXED and panelled 2026-08-31 (12 of 19 findings confirmed); one
residue open — see "Still open" at the bottom
Kind: performance / operability
Relates to: 317 Unit A (apply-rehoming), 58 (incremental projection), 179

## The measurement

The issue-317 Unit A wet run moved **416 mentions** and took **1,185 seconds**
end to end. The identical computation, DRY, took **under one second** — the
walk, the party/bid-party accounting and the plan build are all cheap. So the
whole 20 minutes is the write transaction, and the write transaction is 416
tiny PK-addressed `UPDATE`s plus three bulk statements:

```sql
UPDATE notices SET projected = 0
 WHERE parse_state = 'parsed' AND projected <> 0 AND id IN (
   SELECT caused_by_notice_id FROM tender_versions WHERE tender_id IN (…258 ids…))
```

plus `stamp_tenders_stale` over the same 258 ids and ~424 `append_change`
rows. The re-queue matched **1,710 notices**.

`notices` is a multi-million-row table. 258 bound parameters in an `IN` list
feeding a correlated `IN (SELECT …)` is exactly the shape a planner declines
to turn into a seek, and the suspicion is a full `notices` scan — possibly one
per outer row. Nothing here is inherently 20 minutes' work.

## Why it matters

It is a single `BEGIN IMMEDIATE` transaction, so for those 20 minutes the
database has one writer and it is this job. The daily fetch/parse/fold chain
cannot start; a webhook delivery sweep cannot write. Reads were unaffected
(`/health` stayed green throughout), so this is an availability-of-writes
problem, not an outage — but it is 20 minutes of one, from a repair that
touched 416 rows, and the next campaign over a bigger cohort scales with it.

## What to do — measure before changing anything

1. `EXPLAIN QUERY PLAN` the re-queue statement against a snapshot with a
   realistic id list. If it is scanning `notices`, that is the finding.
2. The obvious shapes to compare: resolve the notice ids in a SEPARATE read
   first (the `tender_versions` half is cheap and indexed) and then re-queue
   by an explicit id list in chunks — the codebase already chunks `IN` lists
   (`IN_CHUNK`); or drive it from `tender_versions` with a join rather than a
   subquery.
3. Whatever the fix, the transaction boundary is the other half of the
   question: the mention moves must be atomic with their `applied_at` stamps
   (the pre-image is the only way back), but the re-queue and the stale stamps
   are idempotent and could be a second, chunked transaction that a crash
   simply redoes. That would cap the exclusive-write window at the moves.

## Do not

Do not simply raise a timeout or move the job to a quiet hour. The write is
slow for a reason that a query plan will name in a minute, and the same
statement shape sits in every repair path that re-queues notices.


## DIAGNOSED AND FIXED (2026-08-31)

`EXPLAIN QUERY PLAN`, turso 0.7.2, against the real schema:

    UPDATE notices SET projected = 0
     WHERE parse_state = 'parsed' AND projected <> 0 AND id IN (…)
    → SEARCH notices USING INDEX notices_parse_state (parse_state=?)

    UPDATE notices SET projected = 0 WHERE id IN (…)
    → SEARCH notices USING INTEGER PRIMARY KEY (rowid=?)

`notices` carries `notices_parse_state`, and turso PREFERS it to the rowid. So
the statement walks every parsed notice in the corpus — 3.4M rows — and a
chunked caller pays that once per chunk. Nothing about the id list matters;
the `parse_state` term alone decides the plan. That is the whole of the 1,185
seconds.

**The fix is one character**, and the codebase already had the idiom: a unary
`+` makes the term non-indexable (`SIBLING_HEAD` in lib.rs uses it for the
same reason, measured 2026-08-05). The statement is otherwise unchanged, so
the semantics are unchanged — which matters, because the first attempt at this
fix moved the predicates into Rust and a panel proved that version
double-counted a duplicate id across a chunk boundary.

Three sites carried the shape and now share two builders
(`requeue_update_sql` / `requeue_count_sql`) through `Db::requeue_notice_ids`:

- `Db::unmark_projected_by_ids` (issue 88's field carriers, and the
  `refold-notices` job) — chunked, so it paid the walk PER CHUNK;
- `Db::apply_rehoming` (issue 317 Unit A) — the measured one;
- the Tier-5 / issue-309 dissolve path, in both its dry and wet branches.

A fourth, found by the panel, was not a re-queue at all:
`Db::projected_parsed_above` — the fold's own coverage gate — had the same
mis-plan AND ran it on the **writer** connection, so a 3.4M-entry walk could
block the fetch/parse/fold chain from inside the gate meant to be "cheap". It
now takes the reader and the `+`.

### What the panel changed beyond the plan

- **The count.** `refold_notices` is printed beside the ids the job was asked
  for, and the difference is how a typo'd id is caught. The Rust-filter draft
  reported 1,000 for 500 distinct notices. Keeping `projected <> 0` on the
  statement makes a repeat a no-op; entry dedup saves the pointless second
  statement. Both pinned by a test whose duplicate straddles a chunk boundary.
- **`checkpoint_on` inside a transaction.** `stamp_tenders_stale` and the
  re-queue both call it from inside `BEGIN IMMEDIATE`, where turso answers
  `wal_checkpoint` with "database table is locked" — and both swallowed it with
  `let _ =`. Swallowing a guaranteed failure is how a real one stops being
  noticed, so `checkpoint_on` now answers the impossible case itself
  (`is_autocommit()`), once, for every present and future caller.
- **The probe.** The first version pinned hand-copied SQL literals; a verifier
  showed a REORDERED spelling of the poison that such a copy would miss. The
  plan test now lives in lib.rs's unit tests and asserts against the
  `pub(crate)` builders themselves — the `PREV_EDGE_JOIN_SQL` discipline — and
  pins all three poisoned spellings, so the day turso stops preferring that
  index the test says the `+` can go.

## Still open

The panel flagged one neighbouring RECORD as wrong: the doc block above
`Db::parsed_id_stripes` (canonical.rs, "the planner picks the rowid range
seek, not `notices_parse_state`"). A verifier's fixture EQP disagrees with it.
Left alone on purpose — that paragraph records a measurement with timings
beside it, and correcting a measured record deserves its own measurement, not
a fixture plan quoted second-hand. Worth half an hour with a snapshot.
