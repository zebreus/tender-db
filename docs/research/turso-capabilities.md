# Turso capabilities — what 0.7.0-pre.10 can and cannot do for tender-db

Research date: 2026-07-19. Subject: the `turso` crate **0.7.0-pre.10** (pure-Rust
SQLite rewrite by Turso, repo `tursodatabase/turso`), the version pinned in our
workspace. Method: every claim marked **[verified]** was exercised hands-on with a
throwaway probe crate against exactly this version (plus `sqlite3` CLI 3.53.1 for
interop tests); claims marked **[source]** come from the crate/`turso_core` source
in the cargo registry; claims marked **[upstream]** cite the GitHub repo/docs.
Turso reports `sqlite_version() = 3.50.4` [verified].

Do not confuse the `turso` crate with `libsql` (Turso's older C-SQLite fork used
by their legacy cloud platform). This document is only about the rewrite.

---

## 1. SQL feature matrix (focused on what we need)

| Feature | Verdict | Detail |
|---|---|---|
| STRICT tables | **works** [verified] | Wrong-type insert rejected (`cannot store TEXT value in INTEGER column`). On by default; the `experimental_strict()` builder flag is a no-op leftover [source]. |
| Foreign keys | **works, but OFF by default** [verified] | Same as C SQLite: `PRAGMA foreign_keys` starts at `0` and orphan inserts are silently accepted. With `= ON`: orphans rejected, `ON DELETE CASCADE` works, `DEFERRABLE INITIALLY DEFERRED` works. **Per-connection setting** — must be applied to every connection, not once per process. |
| CHECK constraints | **works** [verified] | Violations rejected with proper error. |
| Generated columns | **partial, experimental** [verified] | Needs `Builder::experimental_generated_columns(true)`, and only `VIRTUAL` works. `STORED` → parse error, and turso **cannot even open a db file that contains a STORED generated column** (verified with a sqlite3-created file: open fails). |
| Views | **works** [verified] | Create + query, default build. |
| Materialized views | **works, experimental** [verified] | `CREATE MATERIALIZED VIEW` behind `experimental_materialized_views(true)`; incrementally maintained (count updated after later insert). Turso-specific, not SQLite. |
| Triggers | **works** [verified] | `AFTER INSERT` with `NEW.` refs. Enabled by default (`experimental_triggers()` is a no-op) [source]. |
| CTEs | **plain yes, RECURSIVE NO** [verified] | `WITH x AS (...)` works. `WITH RECURSIVE` → `Parse error: Recursive CTEs are not yet supported`. |
| Window functions | **partial** [verified] | `row_number() OVER`, aggregates `OVER (PARTITION BY …)` work. `rank`, `dense_rank`, `ntile`, `lead`, `lag`, `first_value` → "no such function". Custom frames (`ROWS BETWEEN …`) → "not supported yet". |
| JSON functions | **rich, works** [verified] | `json_extract`, `->>`, `json_each`, `json_tree`, `json_group_array/object`, `json_set/remove/patch/type/quote/valid`, `json_array_length`, `jsonb()`. All verified. |
| FTS5 | **absent** [verified] | `CREATE VIRTUAL TABLE … USING fts5` → `no such module: fts5`. |
| Native FTS (turso-specific) | **works, experimental** [verified] | `CREATE INDEX d_fts ON docs USING fts (title, body) WITH (tokenizer='default', weights='title=2.0')` behind `experimental_index_method(true)`; query via `WHERE (title, body) MATCH '…'`. Tantivy-backed (pure Rust; `fts` cargo feature, on by default). Verified: multi-term match, `"quoted phrase"`, per-column match, updates/deletes reflected immediately, index stored **inside the single db file**. Tokenizers: default/raw/simple/whitespace/ngram [source]. **Caveat: makes the file unreadable by the sqlite3 CLI** (see §5). |
| RETURNING | **works** [verified] | On INSERT, UPDATE, DELETE. |
| UPSERT | **works** [verified] | `ON CONFLICT … DO UPDATE` (incl. `excluded.`) and `DO NOTHING`. |
| Partial indexes | **works** [verified] | `CREATE INDEX … WHERE …`. |
| Expression indexes | **works & used** [verified] | `CREATE INDEX ON t(lower(s))`; `EXPLAIN QUERY PLAN` confirms `SEARCH t USING INDEX lower_idx`. |
| Date/time functions | **works** [verified] | `date`, `datetime`, `strftime`, `unixepoch`, `julianday`, `timediff`, modifiers (`start of month`, `weekday N`, `localtime`, `now`, `-3 months`). |
| Collations | **NOCASE works; no custom collations** [verified] | Column-level `COLLATE NOCASE` equality and `ORDER BY … COLLATE NOCASE` work. `LIKE` is case-insensitive for ASCII only (`'ä' LIKE 'Ä'` = 0), `upper('straße')` = `STRAßE` — exactly C SQLite's ASCII-only behaviour, no ICU. The SDK exposes no `create_collation`. |
| Misc verified extras | | `GLOB`, `LIKE … ESCAPE`, **`REGEXP` built in** (bonus vs C SQLite), `group_concat`, `string_agg`, aggregate `FILTER (WHERE …)`, `count(DISTINCT)`, `HAVING`, `INTERSECT`/`EXCEPT`, `FULL OUTER JOIN`, correlated `EXISTS`, `IN (subquery)`, `VALUES`, math fns (`sqrt/pow/floor/ceil/ln/mod`), `printf/concat/instr/replace/substr/trim`, `generate_series`, savepoints, temp tables, `EXPLAIN QUERY PLAN`. |
| ALTER TABLE | **works** [verified] | ADD COLUMN, RENAME COLUMN, RENAME TO, DROP COLUMN. |
| WITHOUT ROWID | **experimental flag to create; reads fine** [verified] | Creating needs `experimental_without_rowid(true)`; reading a sqlite3-created WITHOUT ROWID table works even without the flag. |

## 2. Read-only enforcement for the public SQL endpoint

- **`PRAGMA query_only` exists and is real** [verified]: with `PRAGMA query_only = 1`
  (accepts `1/0/true/false`; **`ON`/`OFF` keywords are rejected** — parse error),
  INSERT/UPDATE/DELETE/CREATE/DROP/ANALYZE/VACUUM/`VACUUM INTO` all fail at
  translate time with `Cannot execute write statement in query_only mode`. Reads
  keep working. The setting is **per-connection** and does not affect other
  connections on the same file [verified].
- **It is NOT sufficient alone**: the same connection can simply run
  `PRAGMA query_only = 0` and then write — escape hatch confirmed hands-on
  [verified]. So arbitrary user SQL must never reach a connection where PRAGMA
  statements are allowed.
- Side channels under `query_only` [verified]: `ATTACH` is still **accepted** when
  the db was built with `experimental_attach(true)` (would let users read
  arbitrary files, and `VACUUM INTO`-style exfil paths open up) — the default
  build rejects ATTACH entirely, so **never enable attach on the endpoint's
  database handle**. `PRAGMA wal_checkpoint` also still runs (harmless but a
  side effect). `BEGIN/COMMIT` allowed (fine).
- **No read-only open flag** in the SDK: `Builder::new_local` exposes no
  OpenFlags; `turso_core` has `OpenFlags` but the SDK doesn't surface it [source].
- **No authorizer hook** (sqlite3_set_authorizer equivalent) and **no statement
  classification API** (`Statement::readonly()` etc.) in the SDK [source].
- **No `interrupt()`** on the SDK `Connection` (core has
  `Connection::interrupt()` and a progress handler, but the wrapper doesn't
  expose them) [source]. However, **cancellation-by-drop works on file-backed
  databases** [verified]: turso statements only advance while polled, and file IO
  yields to tokio constantly (a watchdog task ticked 206/211 expected times
  during a 10.5 s query), so `tokio::time::timeout(dur, fut)` fired at 500 ms and
  the connection was immediately reusable. Caveat: on `:memory:` databases the
  poll never yields — a 300 ms timeout did NOT fire and the query ran 304 s to
  completion inside one poll (the `io_memory_yield` cargo feature exists for
  that). Production is file-backed, so timeouts work.

**Workable enforcement recipe** (all pieces verified individually):
dedicated connection(s) for the endpoint with `PRAGMA query_only = 1` set
server-side, plus an app-level statement gate that parses incoming SQL and
allows only `SELECT` (and optionally `EXPLAIN QUERY PLAN`) — rejecting PRAGMA,
ATTACH, and multi-statement input. The `turso_parser` crate (same version,
already in our tree as a transitive dep) parses SQL to an AST and makes this a
few lines. On top: `tokio::time::timeout` per statement, row/byte caps while
streaming rows, and never enabling `experimental_attach` on that handle.

## 3. Operational behaviour

### PRAGMAs — which ones actually do something [verified]

| Pragma | Behaviour |
|---|---|
| `journal_mode` | **WAL is the default and the only mode**: fresh db reports `wal`; `PRAGMA journal_mode = DELETE` is accepted but stays `wal`. Our `PRAGMA journal_mode = WAL` is a no-op (harmless). |
| `synchronous` | Real: default `2` (FULL); `= NORMAL` → readback `1`; `= FULL` → `2`. Our NORMAL setting is meaningful. |
| `busy_timeout` | Real: `= 5000` reads back `5000`; a blocked second writer with a 3 s timeout waited ~528 ms for the first tx to commit, then succeeded. Also settable via `Connection::busy_timeout(Duration)`. Default is 0 → immediate `database is locked`. |
| `foreign_keys` | Real, default **OFF**, per connection (see §1). |
| `query_only` | Real (see §2). |
| `integrity_check` / `quick_check` | Work, return `ok`. |
| `wal_checkpoint(PASSIVE\|TRUNCATE)` | Works; TRUNCATE truncated the -wal file to 0 bytes. |
| Accepted but inert/empty | `wal_autocheckpoint`, `mmap_size`, `optimize` return nothing; `temp_store`=0, `auto_vacuum`="", `cache_size`=-2000, `page_size`=4096 report values. |

Other pragmas in this build [source: `turso_parser` PragmaName]: `table_info/list`,
`index_list/info`, `database_list`, `function_list`, `page_count`, `freelist_count`,
`schema_version`, `require_where`, `max_page_count`, encryption keys, and an
**unstable CDC pragma** (`capture_data_changes` → `turso_cdc` table) that could
matter for our change cursor someday but is explicitly unstable.

### ANALYZE, VACUUM, backup

- `ANALYZE` works and populates `sqlite_stat1` (`t|bidx|10000 1000`) [verified].
- `VACUUM` and **`VACUUM INTO '<path>'` work** behind
  `Builder::experimental_vacuum(true)`; without the flag both are parse errors.
  `VACUUM INTO` produced a valid backup file [verified].
- No online-backup API in the SDK. Realistic backup story: (a) `VACUUM INTO`
  from the app, or (b) `wal_checkpoint(TRUNCATE)` + file copy, or (c) `sqlite3
  file.db ".backup"` from outside (works because the format is compatible — see
  §5 — as long as no native-FTS index is in the file).

### WAL, multi-connection, concurrency [verified]

- Real `-wal` sibling file; WAL-only engine; single process assumed by default
  (an `experimental_multiprocess_wal(true)` flag exists for multi-process
  access; untested, explicitly experimental).
- Multiple `Database::connect()` handles in one process behave like SQLite WAL:
  a reader on conn2 sees a consistent snapshot while conn1 holds an open write
  tx (count stayed at 1, saw 2 after commit).
- **Concurrent readers actually run in parallel**: two 2M-row `sum()` scans on
  two connections in two tasks took 248 ms wall vs 488 ms sequentially. Our
  current global-mutex-around-one-connection store design gives up real
  parallelism that turso supports.
- Writers: one at a time. Second writer errors `database is locked` immediately
  unless `busy_timeout` is set (then it waits, verified). MVCC / concurrent
  writers exist in `turso_core` (mvcc module) but are **not exposed** through
  the 0.7.0-pre.10 SDK [source].

## 4. Scale signals (dev-machine numbers, NVMe, debug-opt build)

- Bulk insert, one tx, prepared statement: **100k rows in 370 ms (~270k rows/s)**
  [verified]. Autocommit inserts: ~69k/s. Transaction batching is still the way
  to go for the TED backfill, but throughput is not a concern.
- `INSERT INTO … SELECT … FROM generate_series(1, 1_000_000)`: 5.3 s; resulting
  db 34 MB.
- **`CREATE INDEX` on the populated 1M-row table: 31.2 s** — an order of
  magnitude slower than C SQLite (typically 2–4 s). For a many-GB backfill,
  "load then index" could take hours; measure before relying on it, or create
  indexes upfront (inserts at 270k/s were with the PK only).
- Full scans: 100k-row aggregate 34 ms; 2M-row sum ~245 ms; 10M-row
  `generate_series` count+sum 722 ms; 1M-row ORDER BY (computed key) 627 ms.
  Indexed point lookup on 1M rows: sub-ms.
- Tens-of-GB databases: no hands-on data; nothing in the format prevents it
  (SQLite file format), but pre-1.0 maturity means we should synthesise a
  ~10 GB db on the VPS before trusting it (open question).

## 5. File-format compatibility with real SQLite (escape hatches)

All verified hands-on with sqlite3 CLI 3.53.1:

- **sqlite3 reads turso-written files**: integrity_check `ok`, tables, indexes,
  views, triggers, STRICT tables all visible and queryable (100k-row db).
- **The WAL format is compatible**: sqlite3 read rows that existed only in
  turso's un-checkpointed `-wal` file, integrity `ok`.
- **sqlite3 can write into a turso-written db** (insert + integrity `ok`), and
  **turso reads and writes sqlite3-written dbs** (tables, indexes, views,
  data; insert from turso works; integrity `ok`). Bidirectional for the common
  schema subset.
- **Exceptions found**:
  - A **native turso FTS index poisons the file for sqlite3**: `malformed
    database schema (__turso_internal_fts_dir_dfts_key) - near "USING"`; the
    sqlite3 CLI refuses to run any statement against the db. Using turso FTS in
    the main db forfeits the sqlite3 escape hatch for that file.
  - turso cannot open a db containing a **STORED generated column** (open
    fails outright).
  - turso opens a db containing an **FTS5 table** but the table itself is
    unusable (`no such table` on query).

## 6. Project maturity signals [upstream]

- Pre-1.0, self-described as being in **beta**; the project (formerly "Limbo")
  is developed in the open by Turso with a very high release cadence (multiple
  tagged releases per month through 2025–2026; our pin 0.7.0-pre.10 is one of a
  long pre-release series, and a final `0.7.0` has since shipped on crates.io —
  visible in the local cargo registry cache).
- The README historically carried an explicit warning that the software is under
  heavy development and not yet suitable for production data you cannot afford
  to lose; the compatibility document (COMPAT.md) tracks SQLite parity and its
  "not yet supported" entries match what we measured (recursive CTEs, parts of
  window functions, FTS5).
- The amount of `experimental_*` gating in the official SDK (attach, vacuum,
  generated columns, index-method/FTS, materialized views, without-rowid,
  multiprocess WAL, encryption) is itself the clearest maturity signal: the
  team does not yet consider these stable.
- Risk register for us: (1) pre-1.0 file-touching bugs — mitigated by the
  append-only Notice archive being rebuildable and by frequent `VACUUM INTO`
  backups plus sqlite3-verifiable files; (2) feature regressions between
  pre-releases — mitigated by exact version pinning (`=0.7.0-pre.10`) and
  rerunning the probe suite on every bump; (3) experimental flags changing
  semantics — keep the set of enabled flags minimal per handle.

## 7. Fallback map

| Needed capability | Turso 0.7.0-pre.10 status | Realistic fallback |
|---|---|---|
| Read-only SQL endpoint | `query_only` per-connection, but PRAGMA-escapable; no authorizer | **App-level statement gate** (parse with `turso_parser`, allow only SELECT) + `query_only=1` as belt-and-braces + timeout-by-drop + result caps. No C needed. |
| Statement timeout | No `interrupt()` in SDK | `tokio::time::timeout` + drop — verified effective on file-backed dbs. If ever insufficient: ask upstream to expose `interrupt()` (exists in core). |
| Full-text search | No FTS5; native tantivy FTS behind experimental flag | Options: (a) native FTS in the **main** file — best UX, loses sqlite3 escape hatch; (b) native FTS in a **separate** turso db file for search only (app-level join on rowids) — keeps main file clean; (c) v1 ships `LIKE`/`instr` scans + expression indexes; (d) rusqlite+FTS5 sidecar — violates no-C-deps, last resort. |
| Recursive CTEs | Missing | Iterate in Rust (bounded loops over queries) or precompute closure tables. Public-SQL users simply lack the feature until upstream lands it. |
| Window fns (rank/lag/lead/frames) | Missing (row_number + plain aggregate OVER exist) | Rewrite queries with self-joins/subqueries; app-side for API endpoints; document the gap for SQL-endpoint users. |
| Backup | `VACUUM INTO` (experimental flag), checkpoint+copy, sqlite3 `.backup` | All three work today; keep at least one non-turso-dependent path (checkpoint+copy) in the runbook. |
| ANALYZE after bulk import | Works | — |
| STORED generated columns | Unsupported (and files containing them unopenable) | Use VIRTUAL generated columns (flag) or plain columns maintained by triggers/app code. |
| Case-/locale-insensitive search | ASCII NOCASE only, no custom collations | Store normalised shadow columns (e.g. lower-cased, umlaut-folded) with expression or plain indexes; do folding in Rust at write time. |
| Concurrent writers / MVCC | Core-only, not in SDK | Single-writer discipline (we already serialise writes); revisit when SDK exposes MVCC. |
| Multi-process access | `experimental_multiprocess_wal` | Not needed under ADR-0005 (single process). Don't enable. |

## Implications for tender-db

1. **The public read-only SQL endpoint is buildable today**, but the mechanism
   is layered, not a single switch: parse-and-allowlist (SELECT only, single
   statement, no PRAGMA/ATTACH) → run on a `query_only=1` connection (defence
   in depth) → `tokio::time::timeout` per query → cap rows/bytes while
   streaming. Never enable `experimental_attach` on the serving database
   handle. This should be recorded as an ADR when the endpoint is designed.
2. **Search is the one genuine architecture decision**: turso's native FTS is
   real and good (phrases, tokenizers incl. ngram — useful for German compound
   words — weights, single-file), but putting it in the main db forfeits
   sqlite3-CLI readability of our primary data file while pre-1.0 risk is at
   its highest. A separate search-index db file (option b) buys both; plain
   LIKE is an acceptable v1 (needs user decision).
3. **The store crate should stop serialising reads.** Concurrent readers on
   separate connections genuinely parallelise. Shape: one writer connection
   behind a mutex (or a write-task with a channel), N reader connections (pool
   or per-task), all with `foreign_keys=ON` and `busy_timeout` applied **per
   connection** — remember foreign_keys defaults to OFF and our schema's FK web
   silently accepts orphans otherwise.
4. **Current pragma list**: `journal_mode=WAL` is redundant (WAL is the only
   mode), `synchronous=NORMAL`, `busy_timeout`, `foreign_keys=ON` are all real
   and worth keeping.
5. **Bulk backfill**: insert inside big transactions (~270k rows/s observed);
   prefer creating secondary indexes before the bulk load or budget for slow
   `CREATE INDEX` (31 s per 1M rows observed); run `ANALYZE` afterwards (works).
6. **Backups**: nightly `VACUUM INTO` (enable `experimental_vacuum` only on the
   maintenance path) or `wal_checkpoint(TRUNCATE)` + file copy; verify backups
   with the sqlite3 CLI (`PRAGMA integrity_check`) — which stays possible only
   if the main file has no native FTS index (reinforces implication 2).
7. **Schema design guardrails from the gaps**: no STORED generated columns; no
   reliance on recursive CTEs (e.g. for organization-merge chains — model those
   with explicit closure/edge tables); no rank/lag in canonical queries; German
   text search needs normalised shadow columns unless/until FTS is adopted.
8. **Version discipline**: stay pinned exactly; a final `0.7.0` exists and a bump
   should be a deliberate change that reruns the probe suite (the probe crate
   from this research is throwaway, but its SQL list is reproducible from this
   document).

## Open questions

- **[user decision] FTS strategy**: native turso FTS in the main file (lose
  sqlite3 escape hatch) vs a separate search db file vs LIKE-only v1. This
  gates dashboard/API search design.
- **[user decision] Endpoint SQL dialect promise**: do we document the SQL
  endpoint as "turso SQL" (no recursive CTEs, partial window functions) or
  restrict/translate further? Affects public API docs and expectations.
- **[needs research] Tens-of-GB behaviour**: synthesise a ~10 GB db on the VPS
  (real hardware, release build) and measure open time, query latency, index
  build, `VACUUM INTO` duration, memory ceiling. Nothing structural blocks it,
  but it is unproven here.
- **[needs research] Crash-safety confidence**: run a kill -9 torture loop
  during writes (turso's own testing is simulation-heavy, but our own check is
  cheap) before first production data.
- **[needs research] Upgrade to 0.7.0 final** (already in local cache): diff the
  changelog, rerun probes — did query_only/FTS/vacuum flags change?
- **[watch upstream] SDK exposure of `interrupt()`, MVCC/concurrent writes,
  recursive CTEs, remaining window functions, FTS stabilisation, CDC pragma
  (`turso_cdc`) stabilisation** — the CDC table could eventually replace parts
  of our hand-rolled change cursor, but it is explicitly unstable today.
