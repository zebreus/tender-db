# 431 — the durable `/v1/sql` memory bound is an out-of-process worker, gated on turso's multi-process WAL

Status: ready-for-agent — **soak step 1, engine half, DONE 2026-09-29** (see the foot): upstream's own crash, overflow and schema tests pass on 0.7.2, and our store write paths pass under the backend. Left: the box-scale half of step 1 (a reflinked copy, our real transaction sizes), then step 2 (ADR-0005). Was: filed 2026-09-27 from issue 426's decision (step 3). The cheap layer-1 gate (426's
in-process function refusal) ships first; this is the durable isolation, and it is a multi-day commitment with a
hard prerequisite, so it is its own item. DO NOT start before the WAL soak below passes.
Kind: operations / architecture
Relates to: 426 (the measurement and the decision — read it first), 17 (the isolated runtime this extends to a
process), 425 (the deadline, which bounds time not bytes), 337 (turso temp files), ADR-0005 ("one systemd unit" —
this amends it), ADR-0006 (the Ubuntu deploy)

## Why a worker, not a cgroup cap

Issue 426 measured that the dangerous `/v1/sql` shapes allocate through the engine's INFALLIBLE global allocator,
so a runaway ABORTS the whole process — and that a unit-level `MemoryMax` (426 step 2) only converts a kernel OOM
kill into a cgroup OOM kill of the same single `server` process: the same full-service outage (API, SSE, webhooks,
a running ingest job). The ONLY way an out-of-memory `/v1/sql` query takes down just the query is to run it in a
separate process with its own memory limit, so the server survives and answers 503.

## The prerequisite that makes this multi-day

Both the server and the worker must open the same 675 GB DB file. turso serves multiple processes over one WAL
only under `experimental_multiprocess_wal` (`crates/vendor/turso/src/lib.rs`), which is marked experimental/untested
and touches every write path — including the ingest supervisor's, on the production DB. Before ANY worker code:

1. **WAL soak on a scratch copy** (reflinked, never the serving DB): (a) `kill -9` a worker mid-read while the
   server writes and checkpoints — does the `-wal` grow without bound, is the reader slot reclaimed (OFD locks
   should drop on Linux)? (b) a transaction over the 262,144-frame shared-index cap — read latency after the
   overflow falls back to WAL scans? (c) schema/ANALYZE pickup across processes (issue 429's bump). 
2. **An ADR-0005 amendment** — it currently says "one systemd unit". A worker (child process or a second unit) is a
   deliberate departure; record it.

## The shape, once the prerequisite passes (426's box report favours 2a for footprint)

A `/v1/sql` worker as a CHILD of the server (not a second unit): `RLIMIT_DATA` ~4 GiB + `oom_score_adj=1000` +
`PR_SET_PDEATHSIG`, the service unit gains `OOMPolicy=continue` (today it is `stop`, so any in-cgroup OOM stops the
whole unit). A runaway hits `RLIMIT_DATA`, malloc fails, the worker aborts (SIGABRT) — no kernel OOM, server up; if
a global OOM fires anyway, the +1000 score makes the worker the victim. The server frames queries to it over a pipe
and answers 503 while it respawns. NOTE the box's `SystemCallFilter=~@resources` kills a raw `setrlimit` syscall
with SIGSYS, but glibc's `setrlimit` calls `prlimit64` (in `@default`), so `RLIMIT_DATA` via glibc is fine — verify.
Do NOT reach `systemd-run` from the server (box report option 2c: a root-equivalent polkit grant on an
internet-facing process).

## Verify

    ssh -o BatchMode=yes root@zebreus.click "systemctl show tender-db -p MemoryMax; ps --ppid \$(systemctl show -p MainPID --value tender-db) -o comm="

- **done**: a `/v1/sql` worker child (or unit) with a finite `RLIMIT_DATA`/`MemoryMax`, and a soak record showing a
  killed worker leaves the WAL and the server healthy
- **open**: no worker; `/v1/sql` runs in `server` (read 2026-09-27)

## Soak step 1, engine half — 2026-09-29 00:5x UTC (local, scratch files only; nothing on the box)

**Finding: upstream already tests all three questions, and they pass on the exact engine we ship.** `turso_core`
0.7.2 carries `multiprocess_tests.rs` (2,656 lines, real child processes). I built its lib tests in scratch (`cargo test
--locked --lib`, the crate's own lockfile) and ran `multiprocess_tests::`: **48/48 pass**, plus 16 child-process
invocations, in 0.55 s. The three this issue asked about:

- **(a) a SIGKILLed reader:** `subprocess_database_truncate_checkpoint_reclaims_dead_child_reader_slot`. A child
  holds a read transaction and pins a frame. A TRUNCATE checkpoint returns `Busy`, the child is `kill -9`ed, and the
  next TRUNCATE succeeds: the dead slot is reclaimed, so the WAL cannot grow without bound behind a dead worker.
  `subprocess_readonly_child_reader_blocks_restart_and_truncate_checkpoints` pins the live-reader half.
- **(b) the 262,144-frame shared index:** `database_open_rebuilds_from_disk_scan_when_shared_frame_index_overflowed`.
  Past the cap, an open falls back to a disk scan of the WAL. Correct, but the COST at our transaction sizes is the
  open question: a full fold writes far more than 262k frames in one transaction.
- **(c) schema pickup:** `subprocess_database_open_peer_refreshes_remote_schema_without_reopen`. A peer sees a
  schema change (so issue 429's ANALYZE bump) without reopening.

**Our write paths under the backend.** I set `Db::open` to `.experimental_multiprocess_wal(true)` behind a temporary
env switch (reverted, never committed) and ran the write-heavy store suites: `--lib` (136: record/reparse/project/
reclaim), `orphan_org_sweep`, `provisional_echo_fold`, `r2_merge`, `mention_fk_probe` and `provisional_name_norm`.
**All green.** 118 `-tshm` coordination files were written, so the backend really carried them. That includes the
fixtures' second in-process `Builder::new_local` handles on the same file.

**Hard constraints read along the way (they shape the design):**
- `database_open_without_experimental_multiprocess_wal_rejects_second_process`: a server opened WITHOUT the flag
  locks the worker out. The SERVER must open with the flag too, so every prod write path, ingest and folds included,
  runs on the experimental backend. This is the real risk, not the worker.
- `multiprocess_wal_rejects_journal_mode_mvcc_pragma` / `plain_vacuum_rejects_multiprocess_wal_database`: no MVCC
  and no plain VACUUM under the backend. We use neither today. Record it in the ADR amendment.
- `shared_wal_coordination_rejects_remote_filesystem_magic_values`: the box's `/data` is local **xfs**
  (`rw,relatime,inode64`), so it is accepted.
- `.gitignore` gains `*.db-tshm`: the soak left an untracked `crates/store/test-r2-merge.db-tshm`.

**Left of step 1 (box, on a reflinked copy, never the serving DB):**
1. Open the 675 GB copy with the flag, and time the open (does the first open disk-scan the WAL?).
2. Run one real fold-sized transaction past 262,144 frames. Measure reader latency and the next open's disk-scan
   time after the overflow.
3. A 10-minute loop: a writer doing our TRUNCATE checkpoints, and a reader killed at random mid-query. The WAL stays
   bounded and `PRAGMA integrity_check` passes on a small table afterwards.

Then step 2, the ADR-0005 amendment.
