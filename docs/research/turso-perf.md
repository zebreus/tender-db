# Turso 0.7.0 performance model — source-grounded cheat sheet

Research date: 2026-07-25. Pin: **turso 0.7.0 final** (`Cargo.lock:5092`; the whole
`turso_*` family resolves to 0.7.0). Companion to `docs/research/turso-scale.md`
(empirical VPS measurements at 10 GB) and `docs/research/turso-capabilities.md`
(SQL/feature matrix). This doc is the *performance* model, read from the actual
crate source.

All `file:line` citations are into the pinned crate source at
`~/.cargo/registry/src/index.crates.io-*/turso_core-0.7.0/` (the `turso` SDK crate
and `turso_sdk_kit` are cited by crate name where used). turso is the pure-Rust
SQLite rewrite (`github.com/tursodatabase/turso`, formerly "limbo"); it reports
`sqlite_version() = 3.50.4`.

**Why this exists:** the projection's Phase-2 folds 6.96M tenders from 12.39M
notices' parsed layer stored in a ~254 GB turso DB on an 8 GB Hetzner VPS. Reading
each notice in fold (group_key) order = cold random rowid seeks → days. This
document is the source basis for making sequential reads and bulk writes fast.

---

## ★ Prioritized levers

### Reads (the multi-day problem)
1. **Read in ROWID / physical order, not group_key order.** A rowid seek costs
   *tree-depth* cold page reads with **no prefetch/readahead** (btree.rs:1615-1726;
   grep confirms zero readahead in `storage/`). Group-order reads = ~12.4M random
   descents across the satellite b-trees. Physical-order streaming is a sequential
   leaf walk at ~490 MB/s (bench: 20.5 s/10 GB full scan). Single biggest lever.
2. **Give turso a large page cache.** Default is only **2000 pages ≈ 8 MB**
   (page_cache.rs:14) — too small to hold the interior/index nodes of many big
   b-trees at 254 GB, so even index descents hit disk. This, not raw seek latency,
   is why prod is ~50 ms/notice while the 10 GB bench point-query was 0.6 ms cold.
   `PRAGMA cache_size` **is honored** and resizes the live cache
   (translate/pragma.rs:353-359, 1775-1827 → `pager.change_page_cache_size`,
   pager.rs:3432). RAM is lazy — allocated only as pages are read, capped at
   `capacity × page_size` (buffer_pool.rs:448-464, anon mmap + `MADV_HUGEPAGE`).
   Set e.g. `cache_size = -1500000` (~1.5 GB). **Coupling caveat:** the sorter's
   in-memory buffer equals `cache_size` (§4) — a huge cache also makes every sort
   buffer huge. Budget RAM = cache + sort buffer + HTTP server.
3. **Use covering indexes / covering reads.** A non-covering index costs **one
   extra random table b-tree descent per row** (`DeferredSeek` resolved lazily in
   `op_column`, execute.rs:1779-1831). Read the table directly in rowid order, or
   make the index cover.
4. **Batch with `WHERE k IN (…)` — turso already sorts the probe keys.** IN-lists
   compile to an ephemeral **unique** index that is built then scanned in key order
   (translate/main_loop/in_seek.rs:26-76), so the N point seeks come out sorted →
   good locality. Still N descents (not one range scan), so pair with a large cache
   and rowid-sorted batches.

### Writes (bulk backfill / projection output)
5. **Load PK-only, build secondary indexes afterward via `CREATE INDEX`.**
   CREATE INDEX / REINDEX use a sorter → sorted `SeekEnd` + `IdxInsert use_seek(false)`
   = ordered appends that hit the `balance_quick` fast path (translate/index.rs:448-596,
   btree.rs:2907-2989). Bench: 160 MB/s PK-only load vs 38 MB/s with-indexes;
   1.8× faster end-to-end.
6. **Never maintain a random-key (UUID / 128-bit) unique index during insert.**
   `balance_quick` (the cheap append path) is **TableLeaf-only** (btree.rs:2851-2883),
   so every random index insert falls to the expensive 3-sibling `balance_non_root`
   split (btree.rs:2896-2992), scatters cache misses, and makes each PASSIVE
   checkpoint rewrite many distinct hot leaves. Degrades super-linearly as the tree
   grows — this is the "org/tender unique-index steepening". Defer these indexes and
   build them post-load via CREATE INDEX.
7. **Keep `synchronous = NORMAL`.** Default is FULL = fsync per commit
   (pager.rs:4308-4320, lib.rs:2224). NORMAL amortizes durability to checkpoints.
   Big transactions (~2500 rows) keep the WAL small; auto-checkpoint is PASSIVE at
   **1000 frames** (wal.rs:4620, 3852-3855).

### Runtime (HTTP starvation — root cause)
8. **Run projection queries under `spawn_blocking` / a dedicated DB runtime,
   isolated from the HTTP runtime.** turso's async SDK drives blocking `pread`
   inline on the calling tokio worker thread (§6). A long cold query pins a worker
   in back-to-back blocking syscalls and starves everything else on it. This is a
   defect in how we *call* turso, not in turso.

---

## 1. Storage engine (on-disk format)

- **Page size** default 4096 (sqlite3_ondisk.rs:103; `MIN=512`, `MAX=65536`).
  Usable space = `page_size − reserved` (reserved normally 0, sqlite3_ondisk.rs:369-371).
- **Table-leaf cell** = `[payload_size varint][rowid varint][payload][opt 4B
  overflow ptr]` (sqlite3_ondisk.rs:901-927). The rowid IS the clustered key
  (INTEGER PRIMARY KEY aliases rowid). Table *interior* cells carry only
  `{left_child_page, rowid}` (sqlite3_ondisk.rs:782-785) — interior descent never
  touches row data.
- **Overflow threshold**: a row spills to overflow above `usable − 35 ≈ 4061 B`
  for **table** leaves; for **index** leaves (and WITHOUT ROWID clustered tables)
  the threshold is far lower: `((usable−12)*64/255) − 23 ≈ 1002 B`
  (btree.rs:9027-9036). Overflow pages chain one-at-a-time, each holding
  `usable − 4 ≈ 4092 B` (btree.rs:1180), walked page-by-page
  (btree.rs:1133-1244), with an extra full-payload alloc+copy per overflow row.
  **→ Keep rows/blobs under ~4 KB (table) to stay inline.**
- **Seek** = one root→leaf descent, **one page read per level, no sibling
  prefetch, no readahead** (btree.rs:1615-1726; grep confirms none in `storage/`).
  Page reads per point seek = tree depth.
- **Hot read path is allocation-free**: zero-copy cell view (a `&'static [u8]`
  into the page buffer, sqlite3_ondisk.rs:816) + one memcpy into a *reused* record
  buffer (btree.rs:5718-5769, cleared-not-freed between rows). Overflow rows cost
  an extra full-payload allocation.
- **turso does NOT mmap the database file.** The only mmaps are anonymous
  buffer-pool memory (buffer_pool.rs:448-464, `MAP_ANONYMOUS`, fd=−1, `MADV_HUGEPAGE`)
  and the experimental multiprocess-WAL coordination region (io/unix.rs:225-262).
  The DB is read page-by-page via IO into those anonymous buffers.
- **Page allocation**: `allocate_page` (pager.rs:2716) pulls from the freelist
  trunk first and only appends a new page at end-of-file when the freelist is empty.
  **→ A fresh append-only bulk load (empty freelist) lays leaf and overflow pages
  down sequentially in insert order → a subsequent scan is true sequential IO.**
  Deletes/updates/VACUUM populate the freelist, so later allocations scatter.

## 2. Page cache

- SIEVE/GClock eviction over an intrusive linked list (page_cache.rs:90-113).
  Default **2000 pages ≈ 8 MB** (page_cache.rs:14), minimum 200 (page_cache.rs:24).
- `cache_size` honored and **live-resizable** (`resize` just sets capacity;
  growing needs no eviction — page_cache.rs:429-443). RAM scales with pages
  actually read, capped at capacity. Soft limit — may exceed capacity if all pages
  are pinned (pager.rs:3351-3356).
- **No readahead / prefetch anywhere.** A big cache is the main defense for random
  reads: it keeps b-tree interior + index pages resident so descents stop hitting
  disk.

## 3. Query execution

- **Streams** row-by-row (pull VDBE). `op_result_row` stores a raw pointer + count
  into the register file and returns; nothing accumulates engine-side
  (execute.rs:2810). One `step()` = one row, valid only until the next step
  (vdbe/mod.rs:1704-1839, `result_row.take()` at 1756). No result-set
  materialization; memory is O(1) in rows except explicit Sorter / GROUP BY /
  DISTINCT operators.
- **No vectorization, no batched page I/O** — strictly scalar, one row at a time,
  cooperative single-completion I/O (vdbe/mod.rs:1806, 1817-1835).
- `WHERE k IN (list)` = build sorted unique ephemeral index once, then **N
  key-ordered point seeks** (translate/main_loop/in_seek.rs:26-76,
  translate/main_loop/open.rs:422-501). One root-to-leaf descent per value; good
  locality vs unsorted, but not a single range scan.
- Non-covering index = **+1 random table descent per emitted row**
  (`DeferredSeek` → `op_column` does `seek(TableRowId, GE{eq_only})`,
  execute.rs:5180-5193, 1808-1831). Covering index avoids it entirely.
- Full table scan = `Rewind` + `Next` sequential leaf walk (execute.rs:1674,
  2826; btree.rs:1259-1292 advances within a leaf with no I/O, only reads the next
  leaf when the current is exhausted).

## 4. Sorter / temp_store (the old "unconfigured temp store" caution is OUTDATED)

- The Sorter is a proper **external merge sort**: in-memory buffer → sorted chunks
  on a temp file → k-way heap merge (sorter.rs:51-97, 267-296, 408-453).
  **Bounded memory.** Records are arena-allocated; sorting moves only 8-byte
  pointers (sorter.rs:52-58).
- **`temp_store` IS wired** in 0.7.0 (translate/pragma.rs:728-766). With the `fs`
  feature (our server), `Default`/`File` spill to a real temp file via
  `tempfile::tempdir()`; only `Memory` keeps chunks in RAM (io/mod.rs:298-333).
  So large `CREATE INDEX` / `ORDER BY` spill to disk and are safe at 12M scale.
- **⚠ Two gotchas:**
  - The sort in-memory buffer size **= the connection's `cache_size`**
    (execute.rs:6943-6950). Raising `cache_size` for reads also enlarges every
    sort/index-build buffer → OOM risk. Use a modest `cache_size` on the connection
    doing big sorts, or budget RAM accordingly.
  - `tempfile::tempdir()` uses `$TMPDIR` / `/tmp`. **If `/tmp` is tmpfs
    (RAM-backed), "spill to disk" is spill to RAM → OOM.** Set `TMPDIR` to the NVMe
    for any sort/index-build process, or set `temp_store = FILE` and verify the temp
    dir is on real disk.

## 5. Write path

- **WAL frame** = 24 B header + full page image (sqlite3_ondisk.rs:404-405). A
  1-row change rewrites the whole page. Commit marker = the last frame of a txn
  (`db_size > 0`, sqlite3_ondisk.rs:499-501).
- **fsync**: FULL (default) fsyncs the WAL on every dirty commit
  (pager.rs:4308-4320); NORMAL skips per-commit fsync and pays it at
  checkpoint / WAL-restart; OFF never fsyncs.
- **Auto-checkpoint** = PASSIVE at **1000 un-backfilled frames** (wal.rs:4620,
  should_checkpoint wal.rs:3852-3855; triggered post-commit pager.rs:4363-4367).
  PASSIVE is non-blocking, does **not** shrink the WAL, and can be starved by a
  long-lived reader pinning old frames. Cost ∝ number of *distinct* dirty pages
  (dedup to latest frame, sorted by frame id, batched writes — wal.rs:4774-4778,
  `CKPT_BATCH_PAGES=512`).
- **Random-key unique-index degradation** — see lever 6. Structural, three causes:
  (a) `balance_quick` append path is TableLeaf-only (btree.rs:2851-2883), so index
  inserts always hit `balance_non_root`; (b) random keys land at random leaves →
  cache-miss descents + half-full split pages; (c) checkpoint rewrites the many
  distinct hot leaves touched between checkpoints → write amplification grows with
  tree height.
- **Bulk fast paths**: sequential rowid append → `balance_quick`
  (btree.rs:2907-2989). CREATE INDEX/REINDEX → sorter → sorted appends
  (translate/index.rs:448-596). No fast path exists for steady-state random
  single-row inserts.
- **`wal_checkpoint(TRUNCATE)`** = full backfill + DB fsync + WAL zero/fsync under
  the exclusive writer lock (wal.rs:170, pager.rs:4629-4732). Cost scales with WAL
  size; blocks writers (not readers). Heavyweight — use sparingly.

## 6. Async / runtime — why the HTTP server starves

- **Default Linux backend is blocking `pread`, NOT io_uring** (io/mod.rs:17-51;
  `io_uring` is a non-default cargo feature, not in the default set). `UnixIO::pread`
  blocks the calling thread in the kernel for the read (io/unix.rs:317-338);
  `UnixIO::step()` is a no-op.
- **The async SDK is cooperative-yield-shaped but synchronous underneath.**
  `Rows::next().await` → `Statement::step` → on IO calls `run_io()` → `io.step()`
  **inline on the current tokio worker thread** (turso SDK `lib.rs:394-399`;
  `turso_sdk_kit` `rsapi.rs:1444-1451`). The `extra_io` offload hook is **`None`**
  for a plain `db.connect()` (turso SDK `lib.rs:335-337`) — only the sync engine
  installs one.
- **Net effect**: a long cold query = thousands of blocking preads back-to-back on
  one tokio worker; it returns `Poll::Pending` but reschedules instantly and never
  yields to the scheduler → HTTP handlers on that worker starve. Enough concurrent
  cold queries starve all workers.
- **No built-in IO-thread offload** for local file DBs. Mitigations are
  caller-side: (a) run projection via `spawn_blocking` or a dedicated DB
  runtime/thread isolated from the HTTP runtime — you can use the synchronous
  turso_core runners (`run_collect_rows`, statement.rs:687) there and skip the
  async wrapper; (b) enabling `io_uring` shortens but does not remove the blocking
  window (the leader thread still blocks in `submit_and_wait`, io/io_uring.rs:540-546);
  (c) keep the working set cached (lever 2) so cold reads — the only thing that
  blocks — are rare.
- `io/memory_yield.rs` (the `io_memory_yield` feature) is a **test/bench backend**
  that defers completions to force the yield path; not a production knob.

## HARD LIMITS / facts

- **Fast sequential full-table scan: YES, ~490 MB/s cold** (bench, 10 GB), ~30 MB
  RAM — turso streams scans at disk speed. A blob-per-row table reads fast
  sequentially **as long as rows stay under the ~4 KB (table) overflow threshold**;
  wider rows add overflow-page chains that slow the scan (still sequential in a
  fresh bulk load; random after freelist reuse).
- **Random-order reads of 12.4M rows across many satellites is the pathology** —
  the fix is ordering + cache, not a turso knob.
- **Sorts / index builds spill safely (bounded RAM)** — but mind the `cache_size`
  coupling and the TMPDIR-on-tmpfs trap (§4).
- **`VACUUM INTO` OOMs at this scale** (memory scales with DB size; see
  turso-scale.md §1). Don't use it. Backup path is `wal_checkpoint(TRUNCATE)` +
  file copy.
- **No online-backup API, no `interrupt()` on the SDK Connection, no MVCC/concurrent
  writers exposed** through the 0.7.0 SDK (turso-capabilities.md).

## Operational notes

### Large `DELETE FROM changes` (append-only table, salvage-cutover clear)

- `DELETE` is planned as a **table scan + per-row delete** (translate/delete.rs:155,
  `Operation::Scan(Scan::BTreeTable)`); there is **no** unqualified-DELETE
  truncate/`OP_Clear` fast path like C SQLite's. So `DELETE FROM changes` walks
  every row, issues a per-row b-tree delete, and frees each page (incl. overflow
  pages, btree.rs:4864) to the freelist. It is streaming/bounded-memory and needs
  no VACUUM for correctness, but it is **O(rows)** and generates WAL churn
  (auto-checkpointed PASSIVE every 1000 frames). The file does **not** shrink;
  freed pages are reused by later inserts (which then scatter — see §1).
- **Recommendation for a full clear at millions of rows: `DROP TABLE changes;
  CREATE TABLE changes …` instead of `DELETE FROM`.** DROP TABLE emits `Insn::Destroy`
  (translate/schema.rs:2066,2084 → `op_destroy`, execute.rs:11515), which frees the
  whole b-tree in one operation — far cheaper than millions of per-row deletes, and
  it resets the table to a fresh (empty-freelist-relative) state so subsequent
  appends lay down sequentially. Recreate indexes afterward (post-load, lever 5).

### `PRAGMA page_size` is creation-time only

- `PRAGMA page_size = N` is honored **only before the database is initialized**
  (before page 1 / any content is written): `reset_page_size` returns early as a
  no-op once `db.initialized()` is true (connection.rs:2599-2621), mirroring C
  SQLite. **Prod's 4096 cannot be changed in place — it requires a rebuild**: create
  a fresh DB, set `page_size` before writing any table, then bulk-load. A larger
  page_size (e.g. 8192/16384) is viable *only* via such a rebuild; it would raise
  the inline-blob threshold (`usable − 35`) and cut tree depth, at the cost of more
  read/write amplification per page — worth a bench before committing.

---

## Cross-references

- `docs/research/turso-scale.md` — empirical 10 GB measurements on the prod VPS
  (load rates, scan MB/s, CREATE INDEX, VACUUM OOM, crash-safety, backup path).
- `docs/research/turso-capabilities.md` — SQL/feature matrix, `query_only`
  enforcement, PRAGMA behaviour, file-format compatibility, fallback map.
- Bench artifact + scripts on the VPS: `/opt/tender-db/turso-bench/`.
