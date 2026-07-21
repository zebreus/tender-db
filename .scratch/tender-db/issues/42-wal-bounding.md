# 42 — Bound the WAL during bulk loads (13G+ and growing)

Status: ready-for-agent
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
