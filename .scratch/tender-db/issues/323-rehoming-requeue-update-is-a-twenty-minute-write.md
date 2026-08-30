# 323 — The re-homing re-queue is a 20-minute write transaction

Status: ready-for-agent (measured on prod, 2026-08-30, job 505)
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
