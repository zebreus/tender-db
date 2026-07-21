# 42 — Bound the WAL during bulk loads (13G+ and growing)

Status: needs-verification
Priority: high — disk-risk during the running backfill

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
