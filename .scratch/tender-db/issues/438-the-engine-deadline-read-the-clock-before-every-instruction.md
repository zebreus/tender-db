# 438 — the engine deadline read the clock before every instruction: every bounded read ran 1.4–2× slower

Status: ready-for-agent — **BUILT 2026-09-27** (uncommitted, not deployed; see the foot): no serving connection sets
turso's `set_query_timeout` any more. `/v1/sql` and both REST pools stop a read with a timer that calls
`interrupt()` (`crates/app/src/v1/stop.rs`), one mechanism shared with the isolated pool's abandon path. Next: commit,
deploy, then read the Verify. Was: filed 2026-09-27 from a read of the vendored turso 0.7.2 source and a local A/B,
after issues 425 and 120 went live the same morning.
Kind: performance regression (read runtime — every `/v1/sql` query and every REST read)
Relates to: 425 (`/v1/sql`'s engine deadline, `063d9dd`), 120 (the same deadline on both REST pools, `cbacee1`),
430 (the `/metrics` series for stops and abandons, kept), 417 (the `/v1/sql` blocking thread and abandoned cap,
kept), 426 (`plan-probe mem`, the one caller of `set_query_timeout` left)

## Observed (2026-09-27)

**The source.** With a query timeout set, `turso_core` 0.7.2 arms `state.query_deadline` when a statement starts
(`statement.rs` `arm_query_timeout_if_needed`). Then `normal_step` calls `maybe_request_interrupt` before EVERY
VDBE instruction (`vdbe/mod.rs` ~1721), and that function evaluates
`io.current_time_monotonic() >= deadline` (~1515–1530). `current_time_monotonic` is a `clock_gettime` per call
(`io/clock.rs` ~13–17). So one clock read per instruction, for every statement on a connection that has a timeout,
whether or not the statement ever gets near it. The interrupt FLAG (`connection.is_interrupted()`, an atomic load)
is read in the same function unconditionally, so a stop through `interrupt()` adds nothing to the hot loop.

**The measurement.** A local A/B changed nothing but `set_query_timeout(0)` vs `set_query_timeout(600 s)`:

| shape | slowdown with a timeout set |
| --- | --- |
| covering-index GROUP BY | **2.9×** |
| scans | 1.7–2.7× |
| sorter GROUP BY | 1.7–2.1× |

Estimated ~1.4–2× on the box, where IO dilutes the per-instruction cost. It hit every read on the bounded pools, not
just the slow ones: `/v1/sql` (`execute` set the endpoint's 10 s limit on every query, issue 425) and both REST
pools (`Readers::bound_statements(STATEMENT_DEADLINE)`, 25 s, applied at every borrow, issue 120). One casualty:
`/v1/sql/schema`'s first example (`SELECT source, count(*) FROM tenders GROUP BY source`, 3.3 s on 2026-08-31) came
within reach of the 10 s limit.

## Fix

Delete the engine deadline and stop queries with a timer and `interrupt()`:

1. **One mechanism** (`v1/stop.rs`). A borrow registers a clone of its connection (a shared `Arc`, no `Drop` of
   its own) in a `Running` slot with its due instant. A timer task sleeps until then, calls `interrupt()` under the
   slot's lock, and repeats every 50 ms (`TICK`) while the read stays registered. turso ignores an interrupt when no
   statement is active, so the repeats catch a read that was between statements when its time ran out. The borrow's
   `Drop` (`Held`) unregisters under the same lock BEFORE the reader goes back to the pool, then aborts the timer.
2. **An interrupted connection is never pooled again** (`store::Reader::discard`). turso's `interrupt()` checks
   "a root statement is active" and then sets the flag as two separate steps. A statement ending between them clears
   the flag first and the store lands after it, leaving the flag set with nothing running, and the next borrower's
   first statement fails. The window is nanoseconds and nothing public clears the flag, so any borrow that sent an
   interrupt drops its connection. That costs a reconnect and a cold page cache per stop, and there were zero stops
   on prod through 2026-09-27.
3. **Where the timers run.** REST uses a dedicated current-thread runtime (`read-deadline`, one per `AppState`) that
   runs no query, so a timer fires even while every API and isolated thread is pinned in a non-yielding step.
   `/v1/sql` uses its own coordination runtime (issue 417 gave the computation a blocking thread, so those workers
   stay free). The `Held` moves into the blocking closure, so an abandoned computation is still stopped at the limit.
4. **What is bounded changes from a statement to a read.** The clock starts at the borrow and covers every statement
   in it, which is what the 503 always promised ("the most this service spends on one read"). The SSE diff restarts
   the clock per change (`Held::restart_clock`). One borrow classifies up to 500 changes, and each change is one read,
   as each statement was one under the engine deadline.
5. **Kept unchanged.** `Error::Interrupt` still maps to 408 on `/v1/sql` and to 503 plus `STATEMENT_STOPS` on REST.
   The isolated pool's `Abandon` interrupts through the same `Running` slot and still counts `abandoned_total`.
   `/metrics` series names are the same.

## Verify

    grep -rn --include='*.rs' 'set_query_timeout(' crates/app crates/store | wc -l

- **done**: `0`. No serving code sets turso's deadline. (The vendored SDK keeps the pass-through for the offline
  `plan-probe mem`, which lives under `crates/ingest`, outside this count.)
- **open**: `3` (read 2026-09-27 at `13520aa`: `sql.rs` `execute`, `read.rs` `Readers::get`, the store probe test)

A source read, free. It does not prove the deploy. After deploying, `/v1` `source` names the rev.

## Built (2026-09-27)

Uncommitted in the worktree when filed. The lead commits and deploys.

- `crates/app/src/v1/stop.rs` (new): `Running`, `Deadline` (limit + timer runtime), `Held` (Deref to the connection;
  `Drop` = unregister, discard if interrupted, abort the timer), `BoundedReaders`, `spawn_deadline_runtime`.
- `crates/store/src/read.rs`: `Readers::{bound_statements, statement_deadline}` and `deadline_ms` removed, and `get`
  no longer calls `set_query_timeout`. Added `Reader::discard`.
- `crates/app/src/v1/mod.rs`: `AppState::readers` is an `Arc<stop::BoundedReaders>` (the handlers' `readers.get()`
  call sites are unchanged). `AppState::bound_statements` became `set_read_deadline`. `STATEMENT_DEADLINE`'s doc now
  describes the mechanism.
- `crates/app/src/v1/isolate.rs`: the local `Running`/`Held` moved to `stop` and `Abandon` calls
  `Running::interrupt`. `IsolatedReads::new(db, timers)` holds a `Deadline`, and `bound_statements`/`statement_deadline`
  became `set_read_deadline`/`read_deadline`.
- `crates/app/src/v1/sql.rs`: `SqlState` holds a `Deadline` on the SQL runtime. `run` bounds the borrow and drops the
  `Held` in the blocking closure right after the statement. `execute` lost its `limit` parameter and the
  `set_query_timeout` call. The module doc and the example timing note are updated.
- `crates/app/src/v1/sse.rs`: the main-pool borrows go through `BoundedReaders`, and `diff` restarts the clock per
  change.
- `/docs`, `openapi.json` and the `/metrics` help say "interrupted" instead of "the engine's per-statement deadline".
- `crates/vendor/turso`: `set_query_timeout` stays (for `plan-probe mem`), with a doc warning. `VENDORED.md` records
  why, and the upstream ask now includes "read the clock every N steps".
- `crates/store/tests/query_timeout_probe.rs` was replaced by `interrupt_probe.rs`, which tests the engine facts the
  mechanism needs:
  - `interrupt()` from another thread stops the four offender shapes (0.301 / 0.301 / 0.301 / 0.317 s after a 300 ms
    interrupt, 200k rows, debug), and the connection works after.
  - An interrupt with nothing running is ignored, both idle and between statements, while a statement paused between
    rows is stopped at its next step.
  - `Reader::discard` keeps a connection out of the pool.

**Tests.** Focused runs, all green: store `interrupt_probe` 3/3, api 69/69, sql 16/16. New in `api.rs`:

- `a_read_finishing_just_before_its_deadline_leaves_its_connection_clean`: 1.5 s limit, read 1 ends at 0.75 s, read 2
  holds a statement open for 3 s on the same connection. **Mutation-checked**: with `Held::drop` emptied, read 2 fails
  with `Interrupt`.
- `a_main_pool_read_that_never_yields_is_stopped_at_its_deadline`: on a current-thread runtime, a 49M-iteration
  aggregate is stopped at a 300 ms limit. **Mutation-checked**: with the timer spawned on the calling runtime, the
  statement ran 13.1 s to completion.

The issue 120/425/430 tests pass with their meaning unchanged: `a_walk_past_the_statement_deadline_is_stopped_with_a_503`
(still counted on `/metrics`), `an_abandoned_walk_is_interrupted_and_its_slot_freed`, the `/v1/sql` 408 tests and
`a_capped_computation_is_stopped_not_abandoned` (pinned gauge 0).

**Not done here.**

- `crates/ingest/src/bin/plan-probe.rs` `mem` still sets `set_query_timeout` "the way a `/v1/sql` reader is set up".
  That no longer matches `/v1/sql`, and it slows the probed statement. It measures memory, not time, so its numbers
  stand, but its setup is stale. Out of this lane: another agent owns `crates/ingest`.
- clippy and the full `ops/check.sh` gate were not run (disk).
- No A/B was re-measured after the fix: the tests prove the stop path without timing.
