# Turso at tens-of-GB scale — measurements on the production VPS

Research date: 2026-07-19. Follow-up to `turso-capabilities.md`, resolving its
open questions: tens-of-GB behaviour, crash safety, and the 0.7.0-final bump.

**Hardware**: the Hetzner VPS that is also the production target
(`zebreus.click`, Ubuntu 26.04, 4 vCPU, 7.6 GiB RAM, 75 GB NVMe-backed disk,
observed ~490 MB/s cold sequential read). All binaries release-mode
(`opt-level=3`, thin LTO), rustc 1.97.1. sqlite3 CLI 3.46.1 for interop checks.

**Method**: a synthetic schema shaped like ours — three STRICT tables
(`sources`, `tenders`, `notices`) with real FKs, text-heavy ~2.4 KB
notice payloads, five secondary indexes — bulk-loaded to ~10 GB, then each
measurement run as its own process under `/usr/bin/time -v` with the page
cache dropped between phases (cold numbers unless marked warm). Scripts and
raw output live on the VPS under `/opt/tender-db/turso-bench/` (`bench/` crate,
`build.sh`, `run-suite.sh`, `crash-loop.sh`, `run-all.sh`, `post.sh`,
`extras.sh`, `results/*`). Everything below marked **[measured]** comes from
those runs; extrapolations are marked **[extrapolated]**.

---

## 0. The pin that wasn't: we already run 0.7.0 final

Before any benchmark: our `Cargo.toml` says `turso = "0.7.0-pre.10"`, but that
is a **caret requirement, not a pin** — and `turso 0.7.0-pre.10` itself
declares its `turso_core`/`turso_sdk_kit`/… dependencies as caret ranges
(`"0.7.0-pre.10"`), which cargo happily resolves to `0.7.0` final now that it
exists. **Our repo's `Cargo.lock` already resolves the entire turso crate
family to 0.7.0 final** [measured — inspect `Cargo.lock`]. Any fresh
`cargo build` of a crate depending on `turso = "=0.7.0-pre.10"` still links
the 0.7.0 engine unless every `turso_*` crate is force-pinned.

For the A/B comparison below, the pre.10 binary was built with explicit `=`
requirements on all eight `turso_*` crates (verified in the lockfile); the
0.7.0 binary is a clean resolve.

## 1. The 10 GB suite — both versions side by side [measured]

Dataset: ~597k tenders + ~2.388M notices = ~2.985M rows, main file
10.07 GB / 10.05 GB (pre.10 / 0.7.0). Loaded with `foreign_keys = ON`,
`synchronous = NORMAL`, prepared statements, 2 500 rows per transaction,
all five secondary indexes present during the load.

| Measurement | 0.7.0-pre.10 | 0.7.0 final | Max RSS (both) |
|---|---|---|---|
| Bulk load to 10 GB | 265.5 s, **11 241 rows/s** (~38 MB/s) | 262.3 s, 11 359 rows/s | 135–143 MB |
| WAL size during load | stayed ≤ ~36 MB (auto-checkpoints; no manual checkpoints needed) | same | — |
| Open (build+connect+first query, cold) | 34 / 34 / **47 ms** | 39 / 39 / 56 ms | ~21 MB |
| Point query, unique index, cold | avg 0.61 ms, p50 0.59, p99 1.30, max 2.35 | avg 0.61, p99 1.32 | ~28 MB |
| Point query, FK index (`tender_id`), cold | avg 0.83 ms, p99 1.30 | avg 0.86, p99 1.68 | ~28 MB |
| Point query, warm | 0.021 / 0.034 ms | 0.020 / 0.033 ms | ~28 MB |
| Full-scan aggregate over all notices (10 GB, cold) | **20.5 s** (~490 MB/s) | 20.6 s | 31–32 MB |
| Group-by over 597k tenders (cold) | 2.8 s | 2.7 s | — |
| `CREATE INDEX` on 2.39M-row table (cold) | **31.8 s** | 32.3 s | 209–224 MB |
| `ANALYZE` (cold) | **11.3 s** | 11.2 s | 46–53 MB |
| `VACUUM INTO` (cold) | **OOM-killed** at 7.49 GB RSS after 3:16 | **OOM-killed** at 7.49 GB RSS after ~3:17 | 7.49 GB → kernel kill |
| sqlite3 CLI `count(*)` on the live turso file | 1.7 s, correct | 1.9 s, correct | — |

Reading of the non-vacuum rows: **a 10 GB turso database behaves fine.**
Open is instant, point queries through indexes are sub-millisecond even cold,
full scans run at disk speed with a ~30 MB memory footprint, `CREATE INDEX`
at ~13 s/M rows (release build; the 31 s/M in `turso-capabilities.md` was a
debug-opt artifact), `ANALYZE` is cheap. The two engine versions are
performance-identical within noise.

### VACUUM INTO is unusable at this scale — two separate defects

1. **Unbounded memory**: on both versions, `VACUUM INTO` of the 10 GB file
   grew to ~7.49 GB anon RSS and was killed by the kernel OOM killer
   (verified in `dmesg`; `/usr/bin/time` recorded "Command terminated by
   signal 9"). Memory appears to scale with database size, so this box
   cannot vacuum a database bigger than roughly its RAM. [measured]
2. **The destination is written through a destination-side WAL**: the
   interrupted run left a 4 KB main file plus a ~10.1 GB `dest-wal`. On small
   databases the destination does get checkpointed into the main file at the
   end (verified: 20 KB main, empty `-wal`), so this is the *interrupted*
   state — but it means an aborted/killed `VACUUM INTO` leaves a plausible-
   looking file pair behind, and the stray 10 GB `dest-wal` is not cleaned up.

Treacherously, `sqlite3 <dest> "PRAGMA integrity_check"` on the OOM-killed
backup pair reported `ok` (in 17 s — versus 7m19s for a genuine full check of
a complete 10 GB copy, i.e. it clearly did not validate the real content) —
an "integrity-checked" backup that was nevertheless produced by a killed
process. Backup verification must compare row counts against the source, not
just run integrity_check.

Also corrected from `turso-capabilities.md`: `VACUUM INTO` now works
**without** `Builder::experimental_vacuum(true)` on both pre.10 and 0.7.0
[measured — probe `vacuum_into_no_flag`].

### The workable backup path at 10–40 GB [measured]

`PRAGMA wal_checkpoint(TRUNCATE)` + `cp` of the 10 GB file: **17.6 s**
(~570 MB/s, cold cache, 2.5 MB RSS). Offline verification of the copy with
the sqlite3 CLI: full `PRAGMA integrity_check` **7m19s** (cold, IO-bound;
user 19 s / sys 49 s), `count(*)` of the 2.4M-row table 1.9 s.

## 2. Bulk-load strategy: indexes-during vs load-then-index [measured]

The 5-index load sustained ~11.3k rows/s at 74 % CPU but with heavy block-IO
write amplification (≈1.5–1.9 G blocks reported by `time -v` — every
auto-checkpoint rewrites hot index pages; the unique `ref` index is hit at
random positions the whole run).

Load-then-index run (0.7.0, same 10 GB target, tables only, then all five
indexes + ANALYZE):

| Phase | Time | Rate | Max RSS |
|---|---|---|---|
| Load 3.03M rows / 10 GB, PK only, FK ON | **62.4 s** | **48 562 rows/s** (~160 MB/s) | 45 MB |
| `CREATE UNIQUE INDEX notices(ref)` | 26.5 s | | |
| `CREATE INDEX notices(tender_id)` | 23.9 s | | |
| `CREATE INDEX notices(published)` | 27.3 s | | |
| `CREATE INDEX tenders(cpv)` | 3.4 s | | |
| `CREATE INDEX tenders(country)` | 2.9 s | | |
| All five indexes | **84.0 s** | ~13 s per M rows per index | 166 MB |
| ANALYZE | 2.6 s | | |
| **Total to fully-indexed 10 GB** | **~149 s** | vs **265 s** with indexes-during-load | |

Load-then-index is **1.8× faster end-to-end and 4.3× faster on the insert
path**, and the block-IO written drops from ~790 GB (indexes during load —
every auto-checkpoint rewrites the hot, randomly-hit index pages) to ~30 GB
(≈3× the file size: WAL + checkpoint) for the load itself. On this VPS the
disk absorbed the amplification either way; on slower storage the difference
would widen further.

## 3. Crash safety — kill -9 torture loop [measured]

Protocol: a writer inserts 100-row transactions into a STRICT table as fast
as it can, appending the committed max seq to an fsync'd side log after each
commit; it is SIGKILLed at a random point (0.1–1.2 s), then a checker reopens
the db and asserts `PRAGMA integrity_check = ok`, no gaps (`count == max
seq`), and `max seq >= last logged commit` (no committed transaction lost).
Run with production pragmas (`synchronous = NORMAL`; for kill -9 — as opposed
to power loss — NORMAL vs FULL makes no difference since the OS page cache
survives).

Results, 120 iterations per engine version:

| | 0.7.0-pre.10 | 0.7.0 |
|---|---|---|
| Iterations | 120 | 120 |
| integrity_check failures | **0** | **0** |
| Lost committed transactions | **0** | **0** |
| Sequence gaps / partial transactions visible | **0** | **0** |
| Rows accumulated across kills | 568 400 | 558 400 |
| Final file verified by sqlite3 CLI | `ok`, counts match | `ok`, counts match |

In several iterations the reopened database contained one batch *more* than
the fsync'd log (db ahead of log) — the expected benign direction (killed
between COMMIT and log write). The dangerous direction (log ahead of db,
i.e. a committed transaction rolled back) never occurred. WAL recovery after
kill -9 is solid at this (small-file, high-frequency-commit) scale on both
versions. This does not exercise power-loss/fsync-lying scenarios — only
process death.

## 4. 0.7.0 final vs pre.10 — capability re-probes and changelog

### Probe rerun: identical behaviour on every capability we depend on

Both binaries, same probe suite (`results/probes-*.out`):

| Probe | pre.10 | 0.7.0 |
|---|---|---|
| `sqlite_version()` | 3.50.4 | 3.50.4 |
| `foreign_keys` default | 0 (off), orphan accepted | same |
| FK enforcement with `= ON` | orphan rejected | same |
| RETURNING (INSERT) | works | works |
| UPSERT `DO UPDATE` + `excluded`, `DO NOTHING` | works | works |
| `WITH RECURSIVE` | "Recursive CTEs are not yet supported" | same |
| `row_number() OVER` | works | works |
| `rank()`, `lag()` | "no such function" | same (despite enum entries existing in 0.7.0 core — plumbing not wired) |
| Custom frames (`ROWS BETWEEN`) | "not supported yet" | same |
| `query_only = 1` blocks writes | yes | yes |
| `query_only` reset escape hatch | still open | still open |
| `VACUUM INTO` without experimental flag | **works** (doc correction) | works |

### What actually changed pre.10 → 0.7.0 (source diff + release notes)

The interval spans pre.11, pre.13, pre.20–22 (no release notes of their own)
and 0.7.0. ~19k changed lines in `turso_core`. Relevant to us:

- **"Drop the beta warning"** — upstream now considers it production-grade.
- **Crash/corruption fixes**: "Fix various corruption issues", "fix
  nonblocking read_page race and wal frame cache slot reuse corruption",
  "Fix WAL spill frame reuse during page spills", "prevent stale checkpoint
  backfill publish", "Only mark WAL initialized after a successful header
  sync". Several of these are exactly the class of bug our crash loop hunts.
- **Transaction semantics change**: "disallow multiple write stmts in single
  connection and poison tx if half-done write stmt is abandoned" — dropping
  a half-executed **write** statement now poisons the open transaction (COMMIT
  becomes rollback). Harmless for our read-only endpoint (timeout-by-drop on
  SELECTs), but the importer must never drop a mid-flight write future and
  then COMMIT; it must ROLLBACK/reset after any abandoned write.
- **FK fixes**: NULL parent-key semantics, composite-key rowid-alias probes,
  recursive cascade stack overflow, violation-counter bugs.
- **ANALYZE fix**: spurious NULL-idx `sqlite_stat1` rows.
- **`VACUUM INTO` panic fix** (nested statement yields, #7237) — but not the
  OOM (§1).
- **PRAGMA behaviour**: unknown pragmas are now silently ignored
  (SQLite-compatible) instead of erroring.
- **New SQL surface**: REINDEX; SQL-standard scalar functions + PG aliases;
  `WITHIN GROUP` ordered-set aggregates; PostgreSQL-style sequences and
  MVCC-safe AUTOINCREMENT; window-function plumbing (stubs only — see probes).
- **Collations**: core grew custom/locale-backed collation support
  (`register_external_collation` at sdk-kit level) — **still not exposed** on
  the Rust SDK's `Connection`, so the shadow-column plan stands. Same for
  `interrupt()`: now on `TursoConnection` (sdk-kit), still absent from the
  public `turso::Connection`.
- Lots of MVCC work (still not exposed through the SDK builder we use).

### Bump recommendation

**Bump the manifest to `turso = "=0.7.0"` (with `=`) now.** Reasons:

1. We already ship the 0.7.0 engine — the lockfile resolved to it long ago
   (§0). The "pre.10 pin" only misleads readers; there is no behaviour change
   left to adopt.
2. Measured behaviour and capabilities at 10 GB are identical, so nothing
   regresses.
3. 0.7.0 contains the corruption/crash fixes listed above and dropped the
   beta label.
4. Using `=` makes the pin real for the next bump, and future upgrades should
   re-run `/opt/tender-db/turso-bench/` (probes + crash loop at minimum).

## 5. Practical guidance for tender-db

### Bulk backfill (TED history)

- **Expected throughput on this VPS**: ~11k text-heavy rows/s ≈ **38 MB/s of
  database growth** with all indexes on and FK enforcement enabled, ~135 MB
  RSS. A 10 GB backfill ≈ 4.5 min of pure insert time; 40 GB ≈ ~18 min
  [measured at 10 GB; 40 GB extrapolated]. Import throughput will be
  parser-bound, not storage-bound.
- **Transaction size**: 2 500 rows/tx (~6 MB) worked well; WAL stays small on
  its own (auto-checkpointing), no manual `wal_checkpoint` needed during the
  load. Don't autocommit per row.
- **Index strategy**: for the historical backfill, load with primary keys
  only and create secondary indexes afterwards — measured 1.8× faster
  end-to-end (149 s vs 265 s per 10 GB) with ~27× less write traffic during
  the insert phase (§2). `CREATE INDEX` in release mode is cheap (~13 s per
  million rows per index at 10 GB), overturning the debug-build caution in
  `turso-capabilities.md`. Continuous ingestion afterwards keeps indexes on,
  obviously.
- Run `ANALYZE` after the backfill — 11 s at 10 GB is nothing [measured].
- Keep `foreign_keys = ON` during import; its cost is included in the 11k
  rows/s figure.

### Backup cadence at 10–40 GB

- **Do not use `VACUUM INTO` for backups at this scale** — it OOMs the 8 GB
  box at 10 GB database size, on both engine versions [measured]. Revisit
  when upstream fixes vacuum memory usage; even then, remember the
  destination-WAL gotcha (§1.2).
- Use **`wal_checkpoint(TRUNCATE)` + file copy**: 17.6 s for 10 GB
  (~570 MB/s) [measured]; ~1–1.5 min at 40 GB [extrapolated linearly]. The
  checkpoint itself is sub-second when the WAL is small (it stayed ≤ 150 MB
  in all runs).
- **Maintenance window**: only the writer needs pausing (readers keep
  working on their WAL snapshots): pause writes → `wal_checkpoint(TRUNCATE)`
  → `cp` → resume. **≈ 20 s at 10 GB, ≈ 1–2 min at 40 GB** [copy measured at
  10 GB; 40 GB extrapolated]. A nightly cadence is trivially affordable; even
  hourly would be.
- Verify backups **offline on the copy**: sqlite3 `integrity_check` (7m19s at
  10 GB, cold — does not extend the maintenance window since it runs on the
  copy) **and** compare `count(*)` of the biggest tables against the source;
  integrity_check alone passed on a known-bad vacuum output [measured].

### Memory ceilings observed (10 GB db)

Steady-state serving needs tens of MB: open ~21 MB, point queries ~28 MB,
full scan ~32 MB, ANALYZE ~53 MB, CREATE INDEX ~220 MB, bulk load ~140 MB.
Only VACUUM INTO explodes. An 8 GB box is comfortably sized for a 40 GB
database on every path except vacuum. [measured]

## Implications for tender-db

1. **10 GB is a solved size; 40 GB is credible.** Every serving-path
   measurement (open, indexed point queries, scans, index builds, ANALYZE)
   is fast with tens-of-MB memory ceilings on the production VPS, and the
   two engine versions are indistinguishable. Nothing here blocks the
   TED-scale archive. The only path that fails is `VACUUM INTO` (next
   point). Behaviour beyond ~10 GB is extrapolated, not measured.
2. **Backup design must not use `VACUUM INTO` at archive scale.** It
   OOM-kills an 8 GB box at 10 GB database size (memory scales with db
   size), leaves a stray destination WAL when interrupted, and its failure
   mode passes a naive integrity check. The runbook path is:
   pause writer → `wal_checkpoint(TRUNCATE)` → `cp` (≈ 20 s / 10 GB) →
   resume → verify the copy offline (integrity_check + row-count
   comparison). This also keeps the "no turso involved in restore" property.
   Update the `turso-capabilities.md` §3 backup guidance accordingly.
3. **Backfill recipe** (measured): single writer, big transactions
   (~2 500 rows), `foreign_keys = ON`, PK-only schema during the load,
   secondary indexes + ANALYZE afterwards. Storage sustains ~48k
   notice-sized rows/s on this box — the eForms XML parsing will be the
   bottleneck, not turso.
4. **Bump the pin to `turso = "=0.7.0"`** — we already run the 0.7.0 engine
   via lockfile resolution (the Cargo.toml "pin" is a caret requirement and
   turso's own dependency ranges leak forward), capabilities and performance
   are identical, and 0.7.0 carries the corruption fixes. Use `=` so the pin
   is real next time, and keep the bench + probe suite on the VPS as the
   pre-bump regression gate.
5. **Kill-safety is good**: 240/240 clean recoveries across both versions
   with zero lost committed transactions, and the surviving files stay
   sqlite3-readable. Process crashes of the monolith should not corrupt the
   archive. Power-loss safety remains untested (needs fsync-fault hardware
   or dm-flakey, not worth it now given the archive is rebuildable).
6. **Write-statement drop discipline** (new in 0.7.0): an abandoned
   half-executed write statement poisons the open transaction. The store
   crate's writer must never apply timeout-by-drop to writes mid-statement
   and then COMMIT; on any abandoned write, ROLLBACK and retry. Timeout-by-
   drop for the read-only SQL endpoint is unaffected.

## Open questions

- **[watch upstream] `VACUUM INTO` memory** — file an issue / watch for a
  fix (the 0.7.0 "page cache soft limit" work did not cover it). Until
  fixed, periodic compaction of a heavily-deleted database would need
  dump-and-reload; our append-mostly workload makes this a non-issue near
  term.
- **[needs research, later] True 40 GB run** — all 40 GB figures here are
  linear extrapolations from 10 GB. Before the full multi-decade backfill,
  rerun `run-suite.sh` with `TARGET_GB=40` (needs ~45 min and 45 GB free
  disk; skip the vacuum phase or expect the OOM).
- **[needs research, later] Restore drill** — actually restore a checkpoint
  copy and serve from it once the app exists; the file-level pieces are
  verified, the operational drill is not.
- **[watch upstream] rank/lag/lead/frames** — 0.7.0 core contains the enum
  plumbing but the functions still error; recheck on the next bump.
- **[watch upstream] SDK exposure of `interrupt()` and custom collations** —
  both now exist at sdk-kit level but are still unreachable from
  `turso::Connection`; either would simplify the SQL endpoint (real statement
  interrupt) and German-text search (locale collations).
- **[minor] `experimental_vacuum` flag** — `VACUUM INTO` now runs without
  the builder flag on both versions; if we keep a vacuum path anywhere,
  don't rely on the flag as an off-switch.
