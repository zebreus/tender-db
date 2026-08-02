# 93 — `retire_regrouped_tenders` retires in ONE unbounded transaction, with no heartbeat

Status: open — do NOT deploy while the issue-85 re-fold is in flight (deploying restarts a 7h sweep).
Kind: robustness (WAL/RAM bound) + operability
Blocked by: —
Relates to: 63 (bounded WAL — the pattern every other bulk writer in canonical.rs already follows),
90 (the stage timings that exposed this), 91, 85

## What happened

During the eForms-DE 1.x re-fold (2026-08-02) `retire_regrouped_tenders` retired **216,450** Tenders —
the cohort's island→keyed upgrade — in **344.8s (5m45s, 1.59 ms/orphan)**. It completed correctly.

But for those 5m45s it presented as a stall: one thread at high CPU, no stage output, and a static
WAL. It was escalated as a suspected second quadratic and came within four minutes of being killed.
It was neither quadratic nor unhealthy — just unbounded, unobservable, and firing at scale for the
first time (both prior prod runs reported `0 retired`, so this loop had literally never executed).

## Two real defects

**1. One transaction, no bound.** `retire_regrouped_tenders` (canonical.rs:3023) opens a single
`BEGIN IMMEDIATE`, retires every orphan, then `COMMIT`s. Each orphan costs ~23 statements in
`retire_tender_tx`: 1 `append_change`, 4 entity `SELECT id … WHERE tender_id = ?`, 17
`DELETE … WHERE tender_id = ?`, 1 `DELETE FROM tenders`. At 216,450 orphans that is **~5M statements
in one un-checkpointable transaction**.

This is the one bulk writer in canonical.rs that issue 63 never chunked. Every other one already
commits and TRUNCATE-checkpoints in batches: `insert_plan`, the `GROUP_KEY_UPDATE_BATCH` loop, the
legacy-update loop at `NODE_WRITE_BATCH`, the Phase-2 apply at `CHECKPOINT_EVERY_BATCHES`. Retirement
was written for the daily case — "a handful of regrouped Tenders" — and never revisited.

It committed fine at 216k. It is a WAL/in-RAM-WAL-index balloon waiting to happen at a larger orphan
count, and it is all-or-nothing: a kill or a crash rolls back the whole thing.

**2. No heartbeat.** Issue 90 added stage timings, but they print at stage *end*. A 5m45s stage with
no interior output is indistinguishable from a wedged one — which is exactly the failure mode 90 set
out to remove, just one level down.

## Fix

Chunk the retirement at `NODE_WRITE_BATCH` (20 000) orphans: `BEGIN IMMEDIATE` / retire the chunk /
`COMMIT` / TRUNCATE checkpoint, mirroring the legacy-update loop in `build_plan_groups`. Emit a
heartbeat per chunk (`retired N/M`).

A chunk boundary is safe: one retirement is a whole Tender's removal plus its `removed` change events,
and chunks never split one. `publish_cursor` stays a single call after the last chunk.

Worth doing at the same time, since the per-orphan constant is dominated by statement count:

- the 17 `DELETE FROM {table} WHERE tender_id = ?` are `format!`-built and re-parsed on every orphan —
  prepare once (the codebase already does this in `apply_tenders`, canonical.rs:3287) or batch them
  with `tender_id IN (…)` over the chunk;
- the 4 entity `SELECT id`s exist only to emit `removed` change events, and could be folded into the
  same chunked pass.

Measured baseline to beat: **1.59 ms/orphan**.

### Where the 344.8s actually went — batch the RETIRE loop, not just the scan loop

Worth being precise, because the two loops differ by ~7× in statement count and only one of them
matters:

| loop | statements | share of 344.8s |
|---|---|---|
| scan loop — 339,914 touched × 2 queries | 679,828 | ~41 s (12%) |
| **retire loop — 216,450 orphans × ~23 statements** | **~4.98M** | **~304 s (88%)** |
| total | ~5.66M | 344.8s ⇒ **~61 µs/statement** |

So collapsing the scan loop into a single set-based query (one join of the touched Tenders against
`plan_notice`'s group keys) is correct and cheap — but on its own it recovers only ~12%. The fix has
to cut the **retire** loop's 23 statements per orphan: prepare-once or batch the 17 DELETEs with
`tender_id IN (…)` over the chunk, and fold the 4 entity `SELECT id`s into the same pass.

Note the per-statement cost is ~61 µs, not the ~0.5–1 ms that a prepare-per-call model suggests —
turso is evidently caching some of the parse. The lever is statement *count*, not per-statement
overhead.

### WAL behaviour, measured

During the retire the WAL grew **4KB → 38MB** while RSS stayed ~2–3GB. So turso spills an open
transaction's dirty pages to the WAL incrementally rather than buffering all ~5M statements' worth in
RAM — which is why this committed comfortably and why the "static WAL" reading was taken before the
retire loop began.

That *weakens* but does not remove the unbounded-transaction risk: 38MB of WAL per 216K orphans is
linear, so ~2M orphans would mean ~350MB of WAL that cannot be checkpointed mid-statement, plus the
in-RAM WAL-index entry per un-checkpointed frame that issue 63 identified as the actual OOM ceiling.
Chunking is still the right fix; the urgency is lower than first assumed.

## Not a defect (measured, recorded so it is not re-investigated)

`lots` / `lot_results` / `bids` / `contracts` have no explicit `tender_id` index, but each carries
`UNIQUE(tender_id, …)`, and **turso 0.7 does use that auto-index for a leading-column prefix match**:

```
PREFIX  SELECT id FROM lots WHERE tender_id = ?               0.013 ms   (2M-row table)
FULLKEY SELECT id FROM lots WHERE tender_id = ? AND lot_key=? 0.013 ms
PREFIX  DELETE FROM lots WHERE tender_id = ? (no match)       0.091 ms
```

So there is no full-scan-per-tender here and no index to add. The cost is purely statement count ×
orphan count.

## Decision — partial-retirement visibility is ACCEPTED (team-lead, 2026-08-02)

Chunking retirement changes one piece of observable API behaviour, so it was escalated for an explicit
decision rather than folded in silently.

**What changes.** A poller on `/v1/changes` can now observe a **partial** retirement mid-run — some
Tenders removed, others not yet — where previously the whole retirement appeared atomically at one
COMMIT. Note that `publish_cursor` does **not** gate this: it is the in-memory SSE doorbell (issue 61),
and committed rows are readable through `/v1/changes` before it fires. The initial assumption that it
preserved a visibility boundary was checked and found false.

**Accepted, for these reasons:**

1. The feed's contract is an append-only, cursor-ordered stream of independent per-entity events
   consumed via `cursor > last`. It has never promised cross-entity transactional atomicity — this
   exercises the contract as designed rather than violating it.
2. Each `removed` event is independent. No cross-Tender invariant breaks if a consumer sees X removed
   before Y; they are separate business notices, not rows of one record.
3. It manifests only during a fold — a background mutation window in which the tender layer is already
   in flux, which is what the honesty gate exists for. It is not a steady-state anomaly.
4. On the daily incremental the window is a handful of Tenders over sub-second. It is observable at all
   only on a large regrouping like the eForms-DE 1.x re-fold.
5. The alternative is the defect this issue exists for: one unbounded transaction, i.e. a WAL/RAM
   balloon, no heartbeat, and a crash losing *every* retirement to the rollback. The chunked version's
   crash behaviour — earlier chunks committed, the next run re-derives the orphan set and finishes — is
   strictly better and self-healing.

The trade is a slightly longer observable partial-retirement window in exchange for bounded memory,
crash resilience and observability. Worth it.

**Also settled:** cursor gaps are possible when a chunk rolls back (AUTOINCREMENT advances regardless).
Harmless — consumers seek with `cursor > last`, so a gap is indistinguishable from a quiet period.
