# 431 — the durable `/v1/sql` memory bound is an out-of-process worker, gated on turso's multi-process WAL

Status: done — **DECIDED 2026-09-29: NOT VIABLE on turso 0.7.2.** The soak fails: under `multiprocess_wal`, a write transaction that reaches the shared frame index's 262,144 frames (1 GiB of changed pages at 4 KiB) gets `Busy`, even with no other process open, and the process stays jammed until it reopens. The server's deferred `CREATE INDEX` builds and big fold batches cross that. 426's in-process gate remains the protection. **Revisit trigger:** a turso release whose multiprocess WAL lifts the cap; re-run the soak at the foot first. Was: ready-for-agent, soak step 1 engine half done (see below).
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

## Soak step 1, the transaction-size half — 2026-09-29 01:5x UTC: FAILED (local, scratch files only)

A two-process program against the vendored turso 0.7.2 (source below). Every process opens with
`experimental_multiprocess_wal(true)`, as the server and the worker would. The database used 1 KiB pages, so
frames, not bytes, hit the cap cheaply: a 50,000-row baseline, then one transaction inserting 300,000 rows of 700
bytes each.

| run | backend | result |
|---|---|---|
| A | multiprocess, no other process | **`Busy("database is locked")` on INSERT row 260,805**, WAL 275,800,096 B (~263k frames); rolled back |
| B | multiprocess, a long-lived reader in a second process | identical: row 260,805, same WAL size |
| C | default in-process backend (flag off) | **commits**: 300k rows, 634 MB WAL, commit 7.3 s; a reader sees `max_id` 350,000; TRUNCATE 0.73 s |

It is deterministic and needs no second process: the backend caps ONE transaction at the shared frame index.

**Why (a read-only code trace of turso_core 0.7.2, cited):**
- `MAX_FRAME_INDEX_CAPACITY` = 4096 × 64 = **262,144** entries (`storage/shared_wal_coordination.rs:59-70`). That is one
  per appended frame, across the whole un-truncated WAL generation, not per distinct page.
- Past it, `record_frame` sets an overflow flag (`swc:2287-2303`). Every lookup then checks whether this process's
  private coverage includes the snapshot, and returns `Busy` if not (`wal.rs:1851-1876`, "would require blocking WAL
  scan I/O"). Coverage is set only by an open-time disk scan.
- So after overflow the writer process gets `Busy` on every page-cache miss (the INSERT above), including spill
  re-reads at commit (`pager.rs:4129`). A read-transaction begin retries ~10 s, then `Busy`. The TRUNCATE checkpoint
  that would clear it is `Busy` too (`wal.rs:4764-4770`). The only way out is closing everything and reopening (one
  full WAL disk scan).
- Other constraints, read in code:
  - Checkpoint locks are try-locks with no busy handler (`wal.rs:5025` "TOOD: implement proper BUSY handling").
    A long worker query holds a reader slot and makes TRUNCATE `Busy` for its whole run (`wal.rs:2220-2256`), which
    drives the WAL toward the cap even with small batches.
  - Several I/O and lock failures `panic!`/`expect` (`swc:2258, 2310, 2315, 2423`).
  - No test kills a WRITER mid-transaction (only a reader, `mpt:1554`).
- Inferred, NOT verified: a reader that starts right after a TRUNCATE (WAL empty) registers no shared slot
  (`wal.rs:2017-2026`), so the writer's next checkpoint could copy pages into the DB file under it: a torn read in
  exactly the worker pattern.

**Where our workload crosses 262,144 frames (1 GiB at 4 KiB pages):**
- the deferred `CREATE INDEX` builds over the 78M-row party tables (`DEFERRED_TENDER_INDEXES`, one statement each,
  several GiB of index pages);
- large fold batches;
- any run of batches whose TRUNCATE a worker query held off.

A server on this backend would jam on its first Reindex.

## Decision (2026-09-29)

**431 is closed as not viable on turso 0.7.2.** The alternatives considered:
- **A worker over a reflinked snapshot** (xfs reflink is cheap). This isolates memory with no shared WAL, but
  `/v1/sql` answers go stale by up to the refresh interval, which is a product change to the public SQL surface.
  Not taken now. If memory incidents recur, file it as its own decision with the staleness contract stated.
- **Chunking every write under 1 GiB.** Impossible for `CREATE INDEX`.
- **What stays:** 426's in-process layer-1 gate (function refusal) and the statement deadline (425/438).

**Upstream (post from the owner's account, with 425/442 step 4):** under `multiprocess_wal`, a transaction beyond
`MAX_FRAME_INDEX_CAPACITY` jams its own process with `Busy` instead of falling back to a (slower) WAL scan. Ask for
(1) a lookup fallback, or a growable shared index, (2) a busy timeout on checkpoint locks, and (3) a look at the
empty-WAL reader-slot gap. Attach this table and the program below.

## The soak program (to re-run on a future turso)

A scratch crate: `turso = "0.7.2"` patched to `crates/vendor/turso`, plus `tokio` (`rt`, `macros`, `time`). Copy the
workspace `Cargo.lock`, `cargo build --offline`, `[profile.dev] debug = 0, opt-level = 1`. Then:
`wal-soak init db 50000`, `wal-soak bigtxn db 300000` (a pass prints `bigtxn … commit_s=`, a fail prints
`bigtxn_FAILED row=…`), `wal-soak readloop db 120 &` for run B, and `MPWAL=0` for the control run.

```rust
//! Issue 431 soak, local: how does turso 0.7.2's multiprocess WAL behave when one
//! transaction overflows the shared frame index? Every process opens the file with
//! `experimental_multiprocess_wal(std::env::var("MPWAL").map(|v| v != "0").unwrap_or(true))`, as the server and a worker would.
use std::time::{Duration, Instant};

fn now_s() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64()
}

async fn open(path: &str) -> (turso::Database, turso::Connection, f64) {
    let t = Instant::now();
    let db = turso::Builder::new_local(path).experimental_multiprocess_wal(std::env::var("MPWAL").map(|v| v != "0").unwrap_or(true)).build().await.unwrap();
    let conn = db.connect().unwrap();
    // One read forces the WAL/index to be loaded.
    let mut r = conn.query("SELECT COUNT(*) FROM sqlite_schema", ()).await.unwrap();
    while r.next().await.unwrap().is_some() {}
    (db, conn, t.elapsed().as_secs_f64() * 1e3)
}

async fn scalar(conn: &turso::Connection, sql: &str) -> String {
    let mut r = conn.query(sql, ()).await.unwrap();
    match r.next().await.unwrap() {
        Some(row) => format!("{:?}", row.get_value(0).unwrap()),
        None => "none".into(),
    }
}

fn wal_bytes(path: &str) -> u64 {
    std::fs::metadata(format!("{path}-wal")).map(|m| m.len()).unwrap_or(0)
}

/// `n` point lookups over ids 1..=max in one autocommit statement each; mean µs.
async fn lookups(conn: &turso::Connection, max: i64, n: usize, seed: &mut u64) -> f64 {
    let t = Instant::now();
    for _ in 0..n {
        *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let id = 1 + ((*seed >> 33) as i64 % max);
        let mut r = conn.query("SELECT length(b) FROM t WHERE id = ?", (id,)).await.unwrap();
        while r.next().await.unwrap().is_some() {}
    }
    t.elapsed().as_secs_f64() * 1e6 / n as f64
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (mode, path) = (a[1].as_str(), a[2].as_str());
    match mode {
        "init" => {
            let rows: i64 = a[3].parse().unwrap();
            let db = turso::Builder::new_local(path).experimental_multiprocess_wal(std::env::var("MPWAL").map(|v| v != "0").unwrap_or(true)).build().await.unwrap();
            let conn = db.connect().unwrap();
            let _ = conn.execute("PRAGMA page_size = 1024", ()).await;
            conn.execute("CREATE TABLE t(id INTEGER PRIMARY KEY, b BLOB)", ()).await.unwrap();
            conn.execute("BEGIN", ()).await.unwrap();
            for _ in 0..rows {
                conn.execute("INSERT INTO t(b) VALUES (randomblob(700))", ()).await.unwrap();
            }
            conn.execute("COMMIT", ()).await.unwrap();
            println!("init\tpage_size={}\trows={rows}\twal_bytes={}", scalar(&conn, "PRAGMA page_size").await, wal_bytes(path));
            println!("checkpoint\t{}", scalar(&conn, "PRAGMA wal_checkpoint(TRUNCATE)").await);
            println!("after_ckpt\twal_bytes={}", wal_bytes(path));
        }
        "bigtxn" => {
            let rows: i64 = a[3].parse().unwrap();
            let (_db, conn, open_ms) = open(path).await;
            let t = Instant::now();
            conn.execute("BEGIN", ()).await.unwrap();
            for i in 0..rows {
                if let Err(e) = conn.execute("INSERT INTO t(b) VALUES (randomblob(700))", ()).await {
                    println!("{:.1}\tbigtxn_FAILED\trow={i}\terr={e}\twal_bytes={}\twal_frames~={}", now_s(), wal_bytes(path), wal_bytes(path) / 1048);
                    return;
                }
            }
            let ins = t.elapsed().as_secs_f64();
            conn.execute("COMMIT", ()).await.unwrap();
            println!(
                "{:.1}\tbigtxn\topen_ms={open_ms:.1}\trows={rows}\tinsert_s={ins:.1}\tcommit_s={:.2}\twal_bytes={}",
                now_s(),
                t.elapsed().as_secs_f64() - ins,
                wal_bytes(path)
            );
            let mut seed = 7u64;
            let max: i64 = scalar(&conn, "SELECT MAX(id) FROM t").await.trim_start_matches("Integer(").trim_end_matches(')').parse().unwrap();
            println!("{:.1}\twriter_lookup_us={:.1}", now_s(), lookups(&conn, max, 2000, &mut seed).await);
        }
        "read" => {
            let (_db, conn, open_ms) = open(path).await;
            let max: i64 = scalar(&conn, "SELECT MAX(id) FROM t").await.trim_start_matches("Integer(").trim_end_matches(')').parse().unwrap();
            let mut seed = 11u64;
            let us = lookups(&conn, max, 2000, &mut seed).await;
            println!("{:.1}\tread\topen_ms={open_ms:.1}\tmax_id={max}\tlookup_us={us:.1}\twal_bytes={}", now_s(), wal_bytes(path));
        }
        "readloop" => {
            let secs: f64 = a[3].parse().unwrap();
            let (_db, conn, open_ms) = open(path).await;
            println!("{:.1}\treadloop_open_ms={open_ms:.1}", now_s());
            let start = Instant::now();
            let mut seed = 13u64;
            while start.elapsed().as_secs_f64() < secs {
                let max: i64 = scalar(&conn, "SELECT MAX(id) FROM t").await.trim_start_matches("Integer(").trim_end_matches(')').parse().unwrap();
                let us = lookups(&conn, max.min(50_000), 200, &mut seed).await;
                println!("{:.1}\tloop\tmax_id={max}\tlookup_us={us:.1}\twal_bytes={}", now_s(), wal_bytes(path));
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
        "checkpoint" => {
            let (_db, conn, open_ms) = open(path).await;
            let t = Instant::now();
            let r = scalar(&conn, "PRAGMA wal_checkpoint(TRUNCATE)").await;
            println!("{:.1}\tcheckpoint\topen_ms={open_ms:.1}\tresult={r}\tckpt_s={:.2}\twal_bytes={}", now_s(), t.elapsed().as_secs_f64(), wal_bytes(path));
        }
        other => panic!("unknown mode {other}"),
    }
}
```
