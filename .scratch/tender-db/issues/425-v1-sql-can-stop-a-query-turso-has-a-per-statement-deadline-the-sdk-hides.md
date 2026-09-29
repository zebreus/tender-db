# 425 — `/v1/sql` could stop a query: turso 0.7.2 has a per-statement deadline and `interrupt()`, and the SDK hides both

Status: ready-for-agent — **the /v1/sql half is DONE and LIVE 2026-09-27 04:37 UTC** (`063d9dd`): on prod,
`SELECT COUNT(*) FROM generate_series(1, 100000000000)` answered **408 at 10.02 s** and two seconds later
`tender_db_sql_pinned_computations 0`, `tender_db_sql_in_flight 0`, `SELECT 1` → 200 — the work stopped with the
answer (before, that query would have computed for hours as an abandoned computation). Measured first: every offender
stops within 0–71 ms of the deadline. Step 3 (REST walks) BUILT and LIVE 2026-09-27 05:39 UTC (`cbacee1`) under issue 120 (both REST pools
carry a 25 s engine deadline; an abandoned isolated walk is interrupted). **2026-09-27 regression, issue 438: the engine
deadline read the clock before every instruction (every bounded read 1.4–2× slower); REPLACED by a timer +
`interrupt()` (`d987aea`, deployed 2026-09-27 14:40 UTC at `9dedf49`, VERIFIED 2026-09-28 under issue 438) — see the last section.** Open: step 4 (upstream ask) — DRAFTED
below; 0.8.0-pre.13 (read 2026-09-27) still exposes only `busy_timeout`. Re-check at 0.8.0 stable, where the
vendored patch must be re-applied (or dropped) anyway. Was: filed
2026-09-26 21:xx UTC from the owner's review of how user SQL is isolated (asked by Lennart).
Kind: operations / safety — the largest gap in `/v1/sql`'s isolation
Relates to: 17 (the isolated runtime), 51 (the in-task timeout that bounds nothing), 120 (REST walks —
"a backstop is only worth having once cancellation exists"), 238, 417 (abandoned computations counted and
capped at 4), 423

## What we believed

Every doc in `crates/app/src/v1/sql.rs` and `v1/isolate.rs` says turso has **no `interrupt()`**, so a
query cannot be stopped once it starts: a `/v1/sql` query that hits the 10 s cap answers 408 but keeps
computing to completion, holding one of the endpoint's 4 reader connections for minutes (2026-09-18: two
held the whole endpoint for 13.5 min). Issue 417 made that visible and capped (`ABANDONED_CAP = 4`), not
shorter.

## What the pinned sources say

- `turso_core-0.7.2/connection.rs`: `Connection::set_query_timeout(Duration)` (line ~4482),
  `set_progress_handler(ops, handler)` (~4499) and `interrupt()` (~4510).
- `turso_core-0.7.2/vdbe/mod.rs` `normal_step` (~1705): **before every VDBE instruction** it calls
  `maybe_request_interrupt`, which fires on the connection's interrupt flag, the statement's
  `query_deadline` (set from the query timeout when the statement starts, `statement.rs` ~487), or the
  progress handler — then `abort`s and returns `StepResult::Interrupt`. That check is inside the step loop,
  so it does not depend on the statement ever yielding to tokio — the reason our tokio-side timeouts never
  fired (`Statement::step` blocks inside one poll).
- `turso_sdk_kit-0.7.2/src/rsapi.rs` (~940): `TursoConnection::interrupt()` ("mirrors
  `sqlite3_interrupt` … safe to call from another thread") and `set_query_timeout(Duration)`.
- **The `turso` SDK we depend on (`=0.7.2`) exposes neither**: `turso::Connection` keeps its
  `inner: Option<Arc<turso_sdk_kit::rsapi::TursoConnection>>` private and exports only `busy_timeout`.
  0.8.0-pre.8 is the same.

## Plan

1. **Measure first**, on a scratch copy of prod-shaped data: reach `set_query_timeout` (a `[patch.crates-io]`
   vendored `turso` with two pass-through methods is the smallest route; a `turso_sdk_kit` reader pool is
   the other) and run the known offenders — `SELECT count(*) FROM generate_series(1, 1e12)` (the
   non-yielding aggregate), a full streaming scan, a big `ORDER BY` (a sorter: ONE instruction can do a lot
   of work, so granularity is per instruction and must be measured, not assumed), a `GROUP BY` over a
   satellite. Record: does it stop, how far past the deadline, is the connection reusable after.
2. If it stops them: set the per-statement timeout on the `/v1/sql` reader connections (10 s), answer 408
   from the engine's own interrupt, and the abandoned-computation class — `ABANDONED_CAP`, the pinned gauge,
   the 13.5-minute outage shape — goes away for everything the check reaches. Keep the backstop for what it
   does not.
3. Then revisit 120: with real cancellation a REST walk can have a deadline without the accumulation
   failure mode `isolate.rs` documents. **BUILT 2026-09-27 — recorded on issue 120 ("Adopted").**
4. Upstream: ask turso to expose `set_query_timeout`/`interrupt` on `turso::Connection`, so the patch can go.

## Verify

    grep -c 'deadline.hold(' crates/app/src/v1/sql.rs

- **done**: 1 — the endpoint bounds each query's borrow with the timer that interrupts it (issue 438's mechanism,
  which replaced `set_query_timeout`)
- **open**: 0 (read 2026-09-27 at `13520aa`, where `execute` still set the engine deadline — the check was
  `grep -c 'set_query_timeout'` until issue 438; it read 1 = done from 2026-09-27 04:37 UTC)

A source read, free.

## Measured (2026-09-27, `crates/store/tests/query_timeout_probe.rs`)

The SDK route: `crates/vendor/turso` is the crates.io `turso` 0.7.2 with two pass-through methods
(`Connection::set_query_timeout`, `Connection::interrupt`) onto `turso_sdk_kit`'s, wired by
`[patch.crates-io]` (see its `VENDORED.md`); `turso_core` and the SDK kit stay the registry 0.7.2. A 300 ms
deadline, each shape then `SELECT … WHERE id <= 10` on the same connection:

| shape | 200k rows, debug | 3M rows, release | error | connection after |
| --- | --- | --- | --- | --- |
| `count(*) FROM generate_series(1, 1e11)` (issue 51's non-yielding aggregate) | 0.301 s | 0.300 s | `Interrupt` | usable |
| nested-loop join `big a, big b` | 0.301 s | 0.300 s | `Interrupt` | usable |
| full sort `ORDER BY expr LIMIT 1` | 0.301 s | 0.300 s | `Interrupt` | usable |
| `GROUP BY expr` over every row | 0.301 s | **0.371 s** (the worst overshoot, 71 ms) | `Interrupt` | usable |
| `WITH RECURSIVE` unbounded | — | — | refused at parse: turso 0.7.2 has no recursive CTEs | — |

So the per-instruction granularity costs at most tens of milliseconds on these shapes — the "one sorter
instruction does a lot of work" worry did not materialise at 3M rows.

## Built (2026-09-27)

- `v1/sql.rs` `execute` sets `conn.set_query_timeout(limit)` on every query (the endpoint's limit, 10 s in
  production), beside `query_only`; `Error::Interrupt` answers **408**, not 400.
- The backstop, the per-query blocking thread and `ABANDONED_CAP` all stay — for the time spent waiting on a reader
  and any single instruction the check does not reach.
- `tests/sql.rs`: `an_abandoned_computation_keeps_no_worker_and_is_counted` (issue 417: gauge = 2 after two bombs)
  became `a_capped_computation_is_stopped_not_abandoned`: two 49M-iteration bombs each answer 408 at the cap, the
  pinned gauge reads **0**, and `SELECT 1` answers at once.
- `/docs` and `openapi.json`'s 408 no longer say "the engine offers no interrupt"; the docs test pins that on both
  surfaces. `isolate.rs` / `mod.rs` comments now say the REST walks set no engine deadline (issue 120).

## Step 4 — the upstream request, drafted (2026-09-27)

Checked first: `turso` 0.8.0-pre.13's `src/connection.rs` has `pub fn busy_timeout` and nothing else of the kind,
so the ask stands. Posting it is an outward-facing act on a third-party tracker, outside this session's GitHub
scope (`zebreus/tender-db` only); it goes out with the 0.8.0-stable upgrade, whoever does that upgrade, and the
patch's `VENDORED.md` points here. Text:

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

(Added 2026-09-27 by issue 438: the last paragraph. `interrupt()` alone now covers our use; `set_query_timeout`
stays in the ask because a cheap engine deadline would be simpler than our timer.)

## 2026-09-27 — regression: the engine deadline read the clock per instruction

Filed and fixed as **issue 438**. `set_query_timeout` makes turso 0.7.2 evaluate `io.current_time_monotonic() >=
deadline` before EVERY VDBE instruction (`turso_core` `vdbe/mod.rs` ~1515–1530 `maybe_request_interrupt`, called from
`normal_step` ~1721; `io/clock.rs` ~13–17 is a `clock_gettime` per call). A local A/B changing nothing but
`set_query_timeout(0)` vs 600 s: covering-index GROUP BY **2.9×** slower, scans **1.7–2.7×**, sorter group-by
**1.7–2.1×**. Estimated ~1.4–2× on the box. This endpoint set it on every query (`execute`), so every `/v1/sql` query
since `063d9dd` paid it, fast ones included. The 0–71 ms stopping precision measured above was real. The cost was
never measured, because the probe only timed stops.

**What replaced it** (BUILT 2026-09-27, not yet deployed; issue 438 carries the detail):

- `execute` no longer sets a deadline. `run` bounds the reader's borrow with `v1::stop::Deadline::hold`: a timer
  task on this endpoint's own runtime (issue 417 left its workers free) interrupts the connection at the limit,
  counted from the borrow, and again every 50 ms while the query still holds the connection.
- The `Held` guard moves into the blocking closure, so an abandoned computation is still stopped at the limit, and it
  is dropped the moment the statement returns. It disarms under the lock the interrupts are sent under, BEFORE the
  reader goes back to the pool.
- A connection that was sent an interrupt is discarded, not pooled (`store::Reader::discard`). turso's `interrupt()`
  is check-then-set, so an interrupt racing the statement's end could leave the flag set for the next query.
- `Error::Interrupt` still answers 408. `a_capped_computation_is_stopped_not_abandoned` still reads a pinned gauge of 0.
- The measurement file is now `crates/store/tests/interrupt_probe.rs`. `interrupt()` from another thread stops the
  same four shapes 0.301 / 0.301 / 0.301 / 0.317 s after a 300 ms interrupt, and the connection works after.
- The vendored `set_query_timeout` pass-through stays, with a warning, for its one remaining caller (`plan-probe mem`,
  offline). Step 4's draft above gained the clock-read paragraph, and `crates/vendor/turso/VENDORED.md` says the same.
- The Verify above now checks for the timer (`deadline.hold(`) rather than `set_query_timeout`.
