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

**Critical sub-question — answered empirically (5 store tests):**
- **Idle pooled readers do NOT pin the WAL.** A reader that ran a query and was
  returned to the pool holds no snapshot, so a checkpoint reclaims straight past
  it (`an_idle_pooled_reader_does_not_pin_the_wal`). So the root cause is simply
  the *absence of any checkpoint* — turso never auto-checkpoints — **not** reader
  snapshots. No reader-side recycling is needed.
- Only a reader with a *live open transaction* pins the WAL, and then a TRUNCATE
  returns `busy=1` **promptly** (<1s, no `busy_timeout` stall — asserted) and
  reclaims on the next boundary once the snapshot ends
  (`a_live_reader_snapshot_blocks_reclaim_until_it_ends`).
- **PASSIVE folds frames but does NOT shrink the `-wal` file on disk; only
  TRUNCATE returns the space** (`passive_folds_but_only_truncate_shrinks_the_file`).
  Since the live incident is an already-large (13 GB) file, PASSIVE alone would
  not reclaim it.

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
