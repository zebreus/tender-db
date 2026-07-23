# 42 — Bound the WAL during bulk loads (13G+ and growing)

Status: resolved
Priority: high — disk-risk during the running backfill

## Resolution (2026-07-21, team lead)

Production-verified (rev 62f7255, run-driver). The per-package
wal_checkpoint(TRUNCATE) holds the WAL at MEGABYTES across boundaries
(2.9M at 2013-04→05; <1G mid-package) versus the pre-fix ~13G peak;
disk steady 48-50% with steady forward progress (pkg 1→3 in ~15 min).
/health/deep.wal_bytes exposes the live signal. Premise corrected
(c8c659a): turso DOES autocheckpoint PASSIVE but never shrinks the -wal
file and stalls behind a long reader snapshot (the coverage refresher's
GROUP BY scan); TRUNCATE at package/projection boundaries forces reclaim
AND returns the space. Remaining watch: the project (job 5) write burst
— run-driver keeps the 30G alarm armed across it; if the refresher
snapshot still spikes the WAL there, the interval lever is the source-
side follow-up (noted in the issue body).

Note (2026-07-23, issue 53): the refresher snapshot WAS the spike source —
its 60s coverage GROUP-BY pinned the WAL nearly continuously at 7.5M rows
(70G in the field). Fixed source-side (25607d4 + counts gate) by skipping the
scanning sections while a process/project job runs, so reader-free windows
exist for the boundary TRUNCATE. Intra-package checkpointing (checkpoint every
N members, not just at package boundaries) was CONSIDERED and DEFERRED: once
the reader pin is gone, turso's own PASSIVE autocheckpoint bounds the frame
count mid-package (throughput is governed by frame count, not file size), and
disk is not pressured — so it buys only on-disk shrink within a single
multi-hour eForms package, for real added complexity (the process progress
callback is a sync `FnMut` and cannot await a checkpoint). Revisit only if a
dense package's on-disk WAL high-water becomes a disk-headroom problem.

## Confirmed root cause & final fix (2026-07-23) — the SECOND pin

Gating the refresher scans (above) closed the coverage-GROUP-BY pin but the WAL
still ran away. A deployable pool-borrowed instrument (temporary; since stripped
in 0eb1c69) pinned it to a single reader held persistently by the **store** pool
while the WAL climbed. That reader was `store::Db::import_lag()`, called every
60s by the dashboard's `measure_system` — the ONE section deliberately left
UNGATED during heavy writes because it was believed to be cheap point-reads.

The mislabel: `import_lag` ran `SELECT MAX(ingested_at) FROM notices`. There is
no index on `ingested_at`, so `MAX()` over it is a **full table scan** of notices
(7.5M rows in prod), not the "point read" the `measure_system` design comment
claimed. A scan holds a live WAL read snapshot for its whole multi-second
duration (~80ms at 40k rows → ~15s at prod scale), which pins the WAL and
defeats the per-package TRUNCATE — the same spike mechanism as the refresher,
from a different query. This is why the issue-53 full-gate alone didn't stop it:
the pin had moved to the one reader the gate exempted.

Fix (commit 4d023bb, deployed; landed clean on main as 0eb1c69): read the newest
notice via the id PK — `SELECT ingested_at FROM notices ORDER BY id DESC LIMIT 1`.
`ingested_at` is assigned at insert (`now_unix`) in id order, so it is monotonic
with the autoincrement id: the id-newest row's `ingested_at` IS `MAX(ingested_at)`,
but this reads exactly one row via the PK (O(1), ~300us vs ~80ms; measured ~260x
cheaper). No scan, no long snapshot, no new index/migration, lag stays live.
Regression test `import_lag_reads_the_newest_notice_in_o1_not_a_full_scan` (store
lib) asserts both correctness (id-newest == max ingested_at) and that the read is
dramatically cheaper than the scan it replaced (self-calibrating, not a brittle
absolute threshold). The `measure_system` comment is corrected to state the O(1)
invariant is load-bearing.

Prod acceptance PASSED: WAL folds to 0 at every package boundary, store-pool
borrowed drops, WAL sawtooths instead of climbing.

**Lesson:** a "cheap point read" is an assumption about the query PLAN, not the
SQL text. `MAX(col)` is O(1) only with an index on `col`; without one it is a
full scan regardless of how it reads. Validate any "safe to run ungated / holds
no long snapshot" claim against `EXPLAIN QUERY PLAN`, not against a comment — the
`measure_system` comment asserted "no table-proportional scan" for a query that
was exactly that, and the wrong comment cost multiple days of WAL runaway.

Observed (run-driver, 2026-07-21 ~17:50): tender-db.db-wal at 13G and
growing ~10G/47min during job 1's bulk parsing (47G main db, /data at
51%, 98G headroom to the 70% guard). Nothing in the codebase
checkpoints outside the snapshot path (backup.rs wal_checkpoint
TRUNCATE) — turso does not auto-checkpoint fresh frames (per
backup.rs's own comment), so a 166-package process run accumulates WAL
without bound. Projection (job 5) will be an even heavier write burst.

Fix:
- Checkpoint at package boundaries: after each process_package_resilient
  completes (a natural writer-idle moment, same place the issue-32
  cursor is recorded), run a checkpoint (PASSIVE or RESTART — choose
  and justify; TRUNCATE holds the writer hardest, likely overkill
  per-package). Same for projection batch boundaries (issue-19's
  512-tender commits) at a sensible cadence.
- Verify reclaim actually happens with the reader pools open: WAL
  checkpointing cannot pass the oldest live reader snapshot — check
  whether turso's pooled reader connections hold snapshots between
  queries (if they do, that's the REAL root cause and needs
  reader-side handling, e.g. resetting idle connections).
- Surface wal-size: add it to the /health/deep disk check's measured
  values (or a supervisor gauge) so alerting sees WAL runaway.

Acceptance: WAL stays bounded (single-digit GB) across a multi-package
process run under load; project run bounded likewise; wal size visible
to monitoring; no read/write regressions (reads never queue behind the
writer stays true — the checkpoint runs on the writer at idle points).

## Design & findings

New store module `crates/store/src/checkpoint.rs`: `CheckpointMode`
(Passive/Restart/Truncate), `Db::checkpoint(mode)` (acquires the writer),
`checkpoint_on(conn, mode)` (for callers already holding it), `Db::wal_bytes()`.

**Corrected root-cause model (live prod evidence, deploy #5 546189d — which has
NONE of this code):** turso *does* autocheckpoint PASSIVE at a WAL size
threshold; it is not "never". The 13 GB balloon then reclaim-to-near-zero then
regrow seen in prod is a **spike-and-reclaim**: a long-lived reader **snapshot**
pins the WAL so turso's PASSIVE autocheckpoint cannot fold past it, the WAL
grows for the life of that snapshot, and it reclaims once the snapshot ends. The
prime suspect is the dashboard background refresher's coverage scan — a
multi-minute read over ~3.5M rows every 60s (`crates/app/src/coverage.rs`
`measure`). (My earlier "turso never auto-checkpoints" was an over-generalization
of `backup.rs`'s narrower comment, and an artifact of the scratch tests: their
small WAL stayed *under* turso's autocheckpoint threshold, so they observed
manual-checkpoint behaviour, not the autocheckpoint prod shows.)

**What the 6 store tests still prove (and they confirm, not contradict, the
model above):**
- A **live open reader transaction pins the WAL** and blocks reclaim until it
  ends — TRUNCATE then returns `busy=1` **promptly** (<1s, no `busy_timeout`
  stall — asserted) and reclaims on the next boundary
  (`a_live_reader_snapshot_blocks_reclaim_until_it_ends`). This *is* the prod
  spike mechanism, reproduced.
- **Idle pooled readers do NOT pin the WAL** (`an_idle_pooled_reader_...`): a
  connection returned to the pool holds no snapshot, so no reader-side recycling
  is needed — only a legitimately-running scan pins it, and that can't be
  recycled away (it must hold its snapshot while it runs).
- **PASSIVE folds frames but does NOT shrink the `-wal` file on disk; only
  TRUNCATE returns the space** (`passive_folds_but_only_truncate_shrinks_the_file`).
  This is why the fix uses TRUNCATE: turso's own PASSIVE autocheckpoint reuses
  the file in place, so the 13 GB high-water mark persists; TRUNCATE returns it.

**Mode choice:**
- **Process loop → TRUNCATE per package** (`run_process`, right after the issue-32
  resume cursor is recorded — a writer-idle moment). TRUNCATE because it is the
  only mode that reclaims the file; safe per-package because idle readers don't
  pin it and a busy result returns promptly and self-heals next package.
- **Projection → TRUNCATE every 32 batches** (`apply_tenders`, between committed
  512-tender batches; the loop holds the writer throughout so `checkpoint_on` is
  used directly). Bounds and reclaims the projection burst.
- Both best-effort: a checkpoint failure only delays reclaim, never correctness.

**Monitoring:** `/health/deep` disk check now carries `wal_bytes` (the `-wal`
sidecar size). Informational — a large WAL is expected mid-backfill, so it does
not flip the verdict; the alerting routine watches the number for runaway.

Verification = confirm on the running box after deploy that the WAL holds
single-digit GB across a multi-package process run and the projection burst.
