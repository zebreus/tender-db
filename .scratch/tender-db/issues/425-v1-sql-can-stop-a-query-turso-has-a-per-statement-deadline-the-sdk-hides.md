# 425 — `/v1/sql` could stop a query: turso 0.7.2 has a per-statement deadline and `interrupt()`, and the SDK hides both

Status: ready-for-agent — filed 2026-09-26 21:xx UTC from the owner's review of how user SQL is isolated
(asked by Lennart). Read from the pinned 0.7.2 sources; nothing measured yet — the measurement is the
first step, and this project has been wrong about a turso timeout before (sql.rs module doc, issue 51).
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
   failure mode `isolate.rs` documents.
4. Upstream: ask turso to expose `set_query_timeout`/`interrupt` on `turso::Connection`, so the patch can go.

## Verify

    grep -c 'set_query_timeout' crates/app/src/v1/sql.rs

- **done**: 1 or more — the endpoint sets the engine's own deadline (and the measurement is recorded here)
- **open**: 0 (read 2026-09-26)

A source read, free.
