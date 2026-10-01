# 458 — `synchronous = NORMAL` under turso 0.7.2 can tear the database at a checkpoint: the WAL is never fsynced before backfill

Status: ready-for-agent — DEPLOYED 2026-10-01 11:50 UTC (`9b44528`, gate green: GATE-EXIT=0, all suites in 823 s), and the Verify reads done (`9b44528`, `1`). NEXT: read the cost. Chunk 9's project 1809 against chunk 8's 1798 (97 s for 21,258 notices) and chunk 7's 1785 (123 s for 24,570), and the 2026-10-02 07:35 UTC daily fold against 2026-10-01's. Close when the cost is recorded.
Was status: ready-for-agent — filed 2026-10-01 11:xx UTC from 457's evaluation, and BUILT the same hour: `PRAGMAS` now sets
`synchronous = FULL`, pinned by `every_store_connection_runs_synchronous_full`. NEXT: gate, deploy on an idle queue,
run the Verify, and compare the next chunk's and the next daily fold's wall time against the ones before.
Kind: durability (the database is the only copy: no backups, by decision)
Relates to: 457 (turso 0.8.1, which fixes it upstream), 170 (DR posture: no backups), 23/269 (snapshots)

## What is wrong

`crates/store/src/lib.rs` `PRAGMAS` set `synchronous = NORMAL`, reasoned in `docs/research/turso-perf.md` §7 as
"NORMAL amortizes durability to checkpoints". That holds for SQLite: in WAL mode under NORMAL, SQLite fsyncs the WAL
before every checkpoint, so whatever the checkpoint copies into the database file is already durable in the WAL.

turso 0.7.2 does not. Read 2026-10-01 in the published `turso_core-0.7.2` source:
- `storage/wal.rs` `CheckpointState` is `Start → Processing → DetermineResult → Finalize`. `Start` collects the
  frames and goes straight to `Processing`, which writes them into the database file (~:4785). No WAL fsync comes
  first.
- `storage/pager.rs` (~:4620–4645) runs `wal.checkpoint(...)` and only then `SyncDbFile`.
- The commit path (~:4310–4316) fsyncs the WAL only when `sync_mode == SyncMode::Full`. Its comment says NORMAL
  "still fsyncs on checkpoint", but the checkpoint path above does not.

`turso_core-0.8.1` adds the missing barrier. Its own doc comment on `CheckpointState::SyncWal` describes the bug:

> Fsync the WAL before backfilling any frame into the database file. Under `synchronous=NORMAL` commits do not fsync
> the WAL, so without this durability barrier a crash mid-backfill could persist some backfilled DB pages while
> recovery drops the unsynced WAL tail, leaving a torn database that matches no committed prefix.

**Exposure on prod.** The window opens at every checkpoint: each 1,000-frame auto-PASSIVE checkpoint inside a
writer's commit, the TRUNCATE checkpoints the bulk paths and the shutdown run, and the period until the WAL tail is
written back. The trigger is a power loss or kernel crash. A process kill or an OOM kill does not count, because the
kernel page cache survives it. The box has been up 52 days, so the probability is low. The impact is total: a 684 GB
file with no backup and no committed prefix to recover to. That means a rebuild from `/data/archive` (~1–6 days,
`docs/research/dr-premise-2026-08.md`) and the loss of user state (accounts, tokens, webhooks), which the archive
cannot rebuild.

## Decision (owner, 2026-10-01)

**Set `synchronous = FULL` on 0.7.2 now.** Under FULL, every commit fsyncs a dirty WAL, so every frame a checkpoint
copies is already durable, and the torn window closes without touching turso.

- **Cost.** One fsync per write transaction. The data volume is md raid1 over two Micron 7450 NVMe drives
  (`lsblk`, 2026-10-01). These are datacenter drives with power-loss protection, so an fsync is acknowledged from
  protected cache. The bulk paths write ~2,500 rows per transaction (turso-scale.md), so a whole-corpus fold commits
  thousands of times, not millions. The expected cost is seconds per fold. It is measured after the deploy (Verify).
- **Rejected:** backporting `SyncWal` into a patched `turso_core`. It is a second vendored crate for a fix that a
  pragma gives us, and it disappears at the bump anyway.
- **Revisit at the turso bump (457).** 0.8.1 makes NORMAL safe again. Whether to return to NORMAL then is a
  measurement, not a default.

## Built (2026-10-01)

- `crates/store/src/lib.rs` `PRAGMAS`: `PRAGMA synchronous = FULL`, with the reasoning in the doc comment. Every
  connection the store opens takes `PRAGMAS`: the writer at `Db::open`, the store's read pool, and the public
  `readers` pools (`read.rs`).
- Test `every_store_connection_runs_synchronous_full` (lib.rs tests) reads `PRAGMA synchronous` back as `2` on the
  writer, the store read pool and a public readers pool.
- `docs/research/turso-perf.md` §7 is corrected: its premise holds for SQLite, not for turso 0.7.2.

## Verify

    ssh -o BatchMode=yes root@zebreus.click "git -C /opt/tender-db/src rev-parse --short HEAD; grep -c '\"PRAGMA synchronous = FULL\"' /opt/tender-db/src/crates/store/src/lib.rs"

- **open** (2026-10-01): `f40d5e5` and `0`. The deployed source sets `synchronous = NORMAL`.
- **done:** `1`, with the deployed rev at or after the 458 commit. Then read the cost: the next FTS chunk's project
  and the next 07:35 UTC daily project, against chunk 7's project 1785 (123 s for 24,570 notices) and the
  2026-10-01 daily fold.

## 2026-10-01 11:50 UTC — deployed

- Gate: `ops/check.sh` GATE-EXIT=0, all suites green in 823 s at `a90ea7b`. No marker was written because another
  agent's new issue file left the tree dirty. `git diff --stat a90ea7b 9b44528 -- . ':!.scratch'` is empty, so
  `SKIP_TESTS=1 ./deploy.sh` shipped exactly the gated code.
- Deployed `9b44528` (health green, journal `-p err` empty since the restart). The queue was idle by hand-read
  beforehand, because deploy.sh's own busy probe fails open (issue 459).
- Verify: `9b44528` / `1`.
- The cost reading rides the last FTS backfill chunk (jobs 1800–1809), enqueued right after, and tomorrow's daily.
