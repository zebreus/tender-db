# 426 — a `/v1/sql` query shares the server's memory and process, so an out-of-memory query takes down everything

Status: DIAGNOSED-DECIDED 2026-09-27 — measured (below) and cross-checked by a 4-agent read-only workflow
(turso-memory / reach / box / adversarial critic). Decision taken (owner): a layered fix; the first buildable
piece is the in-process AST gate on the measured ABORT class. The worker-process isolation is real but gated on
turso's experimental multi-process WAL + an ADR-0005 amendment, so it is a separate multi-day item (filed as 431).
Filed 2026-09-26 21:xx UTC from the owner's review of how user SQL is isolated (asked by Lennart). Not observed; a
structural gap.
Kind: operations / safety
Relates to: 17 (isolated runtime — threads, not memory), 337 (turso's temp database), 425 (cancellation —
also bounds how long a big sort can grow)

## What

`/v1/sql` runs in the server process on its own threads and 4 reader connections, which bounds CPU (about
4 of the box's 32 cores) and keeps the REST runtime free. Nothing bounds **memory or temp space per query**:
a large `ORDER BY`, `GROUP BY` or hash join builds its state in the process, the result caps (10,000 rows /
10 MB) only bound what is RETURNED, and the box has 62 GB. An out-of-memory kill would take the whole
service down — API, SSE, webhooks, ingest — not just the query, and systemd would restart it mid-job.

## Options (decide when picked up; measure first)

1. **A worker process for `/v1/sql`** with its own memory limit (a systemd scope/slice with `MemoryMax`, or
   `RLIMIT_AS`), talking to the server over a pipe: a runaway kills the worker, the server answers 503 and
   respawns it. Strongest; most work.
2. **`MemoryMax`/`MemoryHigh` on the service unit** plus turso's `cache_size` for the SQL connections: cheap,
   but an OOM still takes the whole service.
3. After 425: an engine deadline bounds how long a sort can GROW, which bounds memory in practice for most
   shapes — measure peak RSS of the worst shapes under the deadline before building option 1.

Leaning: 425 first, then measure peak RSS for the worst shapes, then decide between 1 and 2.

## Verify

    ssh -o BatchMode=yes root@zebreus.click "systemctl show tender-db -p MemoryMax"

- **done**: a finite `MemoryMax` on the unit or on a dedicated SQL worker scope, and the peak-RSS
  measurement recorded here
- **open**: `MemoryMax=infinity` (read 2026-09-26)

## Measured (2026-09-27 06:05 UTC) — the deadline bounds TIME, not BYTES

Instrument: `plan-probe mem <db> <deadline_ms> <sql>` (new; `crates/ingest/src/bin/plan-probe.rs`) opens the file the
way a `/v1/sql` reader is set up — `query_only`, a 128 MiB page cache, the per-statement deadline (issue 425) — drains
the way the endpoint does (stops reading at 10,000 rows), and reports the process's peak RSS, an RSS timeline, and bytes
held in temp files. One statement per process. Bed: a synthetic 8M-row table (842 MB) on tmpfs, release build, 10 s
deadline, each run under an 8 GiB address-space cap. A warm table lets the engine run as fast as it can, so these are
UPPER bounds on growth rate. Exact reproducing statements are deliberately NOT recorded here while the endpoint is
unfixed; they are in the session scratchpad, and the class is enough to design against.

| class | peak RSS (10 s deadline) | spills? |
| --- | --- | --- |
| baseline (`SELECT 1`) | 21 MiB | — |
| sorts (ORDER BY, with or without LIMIT) and GROUP BY | 0.7–0.9 GiB, one integer sort 1.2 GiB and 2.6 s past the deadline | yes (0.4–0.7 GiB temp) except that one |
| joins, IN-subqueries, DISTINCT | ≤ 0.2 GiB | partly |
| accumulating aggregates over one text column | 1.4–2.7 GiB, finishing INSIDE the deadline | never |
| byte-amplifying scalar expressions | 1.9–6.3 GiB from a single row | never |
| accumulating aggregates over amplified rows | **process abort** — one allocation of ~5 GiB failed | never |

**What this settles:**

1. **The deadline does not bound memory.** One statement reached 6.3 GiB inside 10 s, and expression shapes ran up to
   2.6 s past the deadline inside one instruction.
2. **The failure is an ABORT, not only an OOM kill.** Rust aborts the whole process on a failed allocation. In the
   server that is every endpoint, the SSE streams, webhooks and a running ingest job at once — so a unit-level
   `MemoryMax` (option 2) turns "kernel OOM kill" into "cgroup OOM kill or allocation-failure abort": the same outage.
   Option 2 alone does not address it.
3. **Spilling structures are not the problem.** The dangerous classes are accumulating aggregates and byte-amplifying
   scalar functions — both are FUNCTIONS, which the endpoint's allow-list (tables and table-valued functions, on the
   parsed AST) never inspects today.

## Decision (2026-09-27, owner) — layered, in this order

The critic's cross-check (verified against turso_core source, file:line) settled two things the options section left
open: (1) the dangerous class allocates through the INFALLIBLE global allocator, so it ABORTS the process rather than
failing the query — the sorter, hash join, DISTINCT and ephemeral-index structures all allocate FALLIBLY
(`try_reserve` → a graceful query error) and spill to `/data/tmp` (xfs), so they cap at ~512 MiB / 64 MiB and are NOT
the threat. (2) The deadline cannot reach a single-instruction allocation, so it never makes `/v1/sql` OOM-safe.

1. **The in-process gate — BUILT 2026-09-27, gated (133/133); deploy + live pending.** `classify` now refuses the
   abort class via `BANNED_FUNCTIONS` (19 functions) on the same exhaustive AST walk the table/TVF allow-lists use,
   so a call nested in a subquery, CASE arm, HAVING, ORDER BY or an aggregate argument is caught (coverage verified:
   `walk_select_parts` walks the body, every compound arm, the outer ORDER BY and LIMIT). 400 names the function.
   `/docs` lists the refused set. Test `memory_amplifying_functions_are_refused` (crates/app/tests/sql.rs) pins each
   class incl. hidden calls, and that count/sum/GROUP BY/`json_object`/`length` stay 200. Original plan follows:
   Extend `classify`'s existing expression walk (`sql.rs`, the
   `walk_expr` recursion that already covers subqueries for the table allow-list) to REFUSE a SELECT that calls any
   abort-class function: the unbounded accumulating aggregates `group_concat` / `string_agg` / `json_group_array` /
   `json_group_object` (+ `jsonb_*` twins) / `array_agg` / `mode` / any `percentile*`, and the byte-amplifiers
   `zeroblob` / `randomblob` / `char` / `unhex` / `replace` / `printf`/`format` / `hex` / `quote` — with a 400 that
   names the function and says why. This is the ONLY control that stops the class with no spill path and infallible
   allocation; the deadline and every turso budget (there is none for these — `DEFAULT_MEM_BUDGET` is hash-only, a
   `hash_table.rs` TODO) cannot. It refuses some legitimate small `group_concat`, but anything useful there is already
   under the 10 MB result cap, so the loss is small. NOT a table deny-list — it refuses specific FUNCTIONS that have no
   bounded use on an unauthenticated endpoint, so issue 45's positive-list principle is not weakened (the table/TVF
   allow-lists stay positive). Pair with a smaller cache_size on the 4 SQL reader connections.
2. **A unit `MemoryHigh`/`MemoryMax` drop-in — HOST protection only, not `/v1/sql` isolation.** The box has no swap,
   `overcommit_memory=0`, `OOMPolicy=stop`, one process in the cgroup: a cgroup limit turns the Rust abort into a
   cgroup-OOM SIGKILL of the same `server` — the same full-service outage. Worth having so a runaway cannot take the
   HOST down (evicting the page cache, starving neighbours), NOT as the fix for 426. Gated on measuring the service's
   ANON share during a heavy job first (the historical 53–57 GiB instance peaks are mostly page cache; a `MemoryMax`
   below a fold's real anon working set would kill legitimate jobs). Deferred until that measurement.
3. **The `/v1/sql` worker process (issue 431).** A child (or sibling unit) under `RLIMIT_DATA` + `oom_score_adj=1000`
   + `OOMPolicy=continue`, so the abort/kill takes only the worker and the server answers 503 and respawns. The only
   option that truly isolates the class in process terms — but it requires BOTH the server and the worker to open the
   DB, which means turso's `experimental_multiprocess_wal` (marked experimental/untested; it touches every write path
   on the 675 GB prod DB), a soak on a scratch copy (kill a worker mid-read while the server writes+checkpoints; a
   >262,144-frame transaction; schema/ANALYZE pickup across processes), and an ADR-0005 amendment ("one systemd
   unit"). Multi-day; filed as issue 431. Do NOT reach `systemd-run` from the server (a root-equivalent polkit grant).

## Verify

    ssh -o BatchMode=yes root@zebreus.click "systemctl show tender-db -p MemoryMax"

- **done**: step 1 LIVE on prod 2026-09-27 07:0x UTC (`9ae5ddd`) — `group_concat(id) FROM v_tenders`, `hex(zeroblob(1e9))`
  and nested `replace(hex(zeroblob…))` each 400 naming the function; `count`/`max`/`min` over a base table,
  `json_object` and `length` stay 200. (The two `v_tenders` aggregate 408s seen while checking are the view's own
  7.9M-row materialisation cost, issue 239 — they parse and execute, so the gate admits them.) A finite `MemoryMax`
  is step 2, still to come.
- **open**: `MemoryMax=infinity` (read 2026-09-26; still infinity 2026-09-27 — step 2 deferred on the anon measurement)

