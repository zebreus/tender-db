# Upstream requests for turso — ready to paste

Status: READY TO POST, not posted. The session's GitHub identity has no write access to `tursodatabase/turso`
(`add_repo` refused 2026-09-29), so a person files these from their own account through the GitHub web UI:
<https://github.com/tursodatabase/turso/issues/new>. Anyone can open an issue on the public repo. Check that each
ask still stands on the current turso release first (see "Still true?" under each). Then record the issue URL on the
tender-db issue named in each heading.

Engine version measured: `turso_core` / `turso` **0.7.2** (vendored SDK patch: `crates/vendor/turso/VENDORED.md`).

---

## 1. `turso::Connection`: expose `set_query_timeout` and `interrupt` (tender-db issue 425 step 4, with 438)

Still true? `turso`'s `src/connection.rs` exposes `busy_timeout` and nothing else of the kind (checked on
0.8.0-pre.13, 2026-09-27).

> **`turso::Connection`: expose `set_query_timeout` and `interrupt`**
>
> `turso_core::Connection` has a per-statement deadline (`set_query_timeout`, checked before every VDBE
> instruction in `normal_step`) and a thread-safe `interrupt()`, and `turso_sdk_kit::rsapi::TursoConnection`
> passes both through — but the `turso` crate's `Connection` keeps its inner connection private and exposes
> only `busy_timeout`, so a Rust application cannot stop a running statement. A `tokio` timeout cannot either:
> `Statement::step` does not yield while the pages are cached, so the timer never gets polled.
>
> We serve a public read API over turso and needed both: a user-facing SQL endpoint with a 10 s limit and a
> REST surface whose filtered reads can walk millions of rows. With the two methods patched into a vendored
> 0.7.2 SDK (two three-line pass-throughs to `get_inner_connection()`), a 300 ms deadline stopped a
> `generate_series` aggregate, a nested-loop join, a full `ORDER BY` and a `GROUP BY` within 0–71 ms, every
> time, with the connection usable afterwards; before, an abandoned query once ran for ~85 minutes with nobody
> waiting. Would you accept a PR adding
> `Connection::set_query_timeout(&self, Duration) -> Result<()>` and `Connection::interrupt(&self) -> Result<()>`?
>
> One engine-side note on the deadline itself: while a query timeout is set, `maybe_request_interrupt` evaluates
> `io.current_time_monotonic() >= deadline` before EVERY VDBE instruction (a `clock_gettime` per call). Changing
> nothing but the timeout (0 vs 600 s) made a covering-index GROUP BY 2.9× slower, scans 1.7–2.7× and sorter
> GROUP BYs 1.7–2.1×, so we could not keep it on and stop queries with a timer and `interrupt()` instead. Reading
> the clock every N steps (the way the progress handler counts `vm_steps`) would keep the deadline's precision to
> within N instructions at a fraction of the cost.

---

## 2. FK parent-key probe ignores a child index that leads with the FK columns (tender-db issue 442 step 4)

Still true? `translate/fkeys.rs` `emit_fk_parent_key_probe` requires `ix.columns.len() == child_cols.len()`.

> **FK parent-key probe ignores a child index that leads with the FK columns**
>
> `translate/fkeys.rs` `emit_fk_parent_key_probe` (turso_core 0.7.2) looks for a child index with
> `ix.columns.len() == child_cols.len()` and every column equal in order. An index that merely LEADS with the child
> columns fails that test, including a composite PRIMARY KEY, and the probe falls back to `table_scan_match_any`: a
> full scan of the child table per deleted (or re-keyed) parent row. SQLite uses any index whose leftmost columns
> are the child key.
>
>     CREATE TABLE p (id INTEGER PRIMARY KEY);
>     CREATE TABLE c (pid INTEGER NOT NULL REFERENCES p(id), lang TEXT NOT NULL, PRIMARY KEY (pid, lang));
>     PRAGMA foreign_keys = ON;
>     EXPLAIN DELETE FROM p WHERE id = 1;   -- OpenRead c + Rewind: the whole table, per row
>
> With `c` at 78M rows a single-row delete takes ~2 s. The function already has the pieces for a prefix match:
> `index_scan_match_any` iterates "the index entries whose leading columns equal `probe_start`". Selecting an index
> with `ix.columns.len() >= child_cols.len()` and a matching prefix, and taking the `index_scan_match_any` path
> whenever the index is longer than the key, would serve these without a scan. Workaround today: a redundant index
> of exactly the FK's columns.

Evidence to attach: prod timings of 2.2 s per mention delete before the exact-shape index, 16 µs after (issue 441).

---

## 3. `multiprocess_wal`: a transaction past the shared frame index jams its own process (tender-db issue 431)

Still true? `storage/shared_wal_coordination.rs` `MAX_FRAME_INDEX_CAPACITY` = 4096 × 64.

> **multiprocess_wal: one transaction past `MAX_FRAME_INDEX_CAPACITY` makes the writer's own process return Busy
> until it reopens**
>
> With `experimental_multiprocess_wal(true)` on 0.7.2, a single write transaction that appends more than 262,144
> WAL frames (1 GiB at 4 KiB pages) fails with `Busy("database is locked")`, even when no other process has the
> database open. With the flag off, the same transaction commits. Repro (1 KiB pages, 700-byte rows):
> `BEGIN; INSERT … × 300,000; COMMIT` fails at row 260,805 with the WAL at ~263k frames. Without the flag it commits
> (634 MB WAL, 7.3 s).
>
> From reading the code: once the shared index is full, `record_frame` sets the overflow flag
> (`shared_wal_coordination.rs` ~2287). After that, every lookup whose snapshot the process's private coverage does
> not include returns Busy (`wal.rs` ~1851-1876, "would require blocking WAL scan I/O"). That covers page-cache
> misses (including spilled pages re-read at commit), read-transaction begins (~10 s of retries, then Busy) and the
> TRUNCATE checkpoint that would clear it (`wal.rs` ~4764). The only way out is closing and reopening.
>
> Any workload with large transactions (index builds, bulk loads) therefore cannot run under the feature. Would you
> consider (a) a fallback to a WAL scan (slow but correct) or a growable shared index instead of Busy, and (b) a
> busy timeout on the checkpoint's try-locks (`wal.rs` ~5025 "TOOD: implement proper BUSY handling")? Separately,
> it looks like a reader that begins while the WAL is fully checkpointed takes no shared reader slot
> (`wal.rs` ~2017-2026), so the writer's next checkpoint cannot see it. We did not reproduce that one.

The soak program (a two-process repro) is in tender-db issue 431, "The soak program".
