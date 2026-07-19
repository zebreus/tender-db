# 17 — Isolate SQL-endpoint execution on its own runtime

Status: resolved
Blocked by: 07

Goal: close the residual resource gap in the SQL endpoint: a single
non-streaming aggregate (no row boundary to yield on, no turso interrupt())
can hog a worker thread bounded only by pool + per-token caps.

Scope: run /v1/sql query execution on a dedicated small tokio runtime (1-2
threads) or spawn_blocking pool so a pathological query can never starve the
API/SSE/dashboard runtime; keep the cooperative per-row yield; watch
upstream for `interrupt()` exposure on turso::Connection (the engine has it
in sdk-kit; docs/research/turso-scale.md tracks it) and adopt it when
available.

Acceptance: a deliberately CPU-pathological aggregate leaves API latency
unaffected (measured in a test or documented manual probe); SQL tests stay
green.

## Answer

Delivered: `/v1/sql` query execution now runs on a dedicated tokio runtime, off
the main API/SSE/dashboard runtime (`crates/app/src/v1/sql.rs`).

**The isolation.** `SqlState` owns a two-worker-thread runtime built by
`spawn_sql_runtime`. The runtime is owned by a parked OS thread that blocks on a
never-completing future, so it lives for the whole process and is never dropped
in an async context (which tokio panics on). The `/v1/sql` handler still does
rate + concurrency + parse checks on the main runtime, then submits the actual
`execute` (reader acquisition + `tokio::time::timeout` + row streaming) to the
isolated runtime via `Handle::spawn`, and only `.await`s the `JoinHandle` — a
cheap channel wait that never blocks a main worker. So a non-yielding aggregate
(which no timeout can interrupt — turso has no `interrupt()`) can pin at most the
two isolated threads; the main runtime keeps serving. The per-token concurrency
cap (2) and this thread count together bound SQL CPU. The cooperative per-row
`yield_now` from issue 07 stays, so streaming queries remain interruptible.

**Not a process-global singleton — one runtime per `SqlState`.** In production
`AppState::new` runs once, so this is one runtime. Making it per-instance rather
than a global `OnceLock` means each server in the test suite gets its own
isolated threads instead of all test servers contending for one shared pool —
which was a real source of cross-test starvation (a cooperative query's
wall-clock timeout firing while it was starved of a shared thread). A server
owning its own isolation is also the more honest model.

**Acceptance test** (`sql_execution_does_not_starve_the_api`): the whole server
runs on a two-thread runtime; two heavy non-yielding cross-join aggregates are
fired (taking both of the user's concurrency permits), and while they run
`/health` is hit five times and must answer in under two seconds. Without
isolation the two aggregates would pin both main threads and freeze the API;
with it, `/health` stays prompt. The aggregates' own status is not asserted
(under parallel-test CPU load a cross join can legitimately exceed the 10 s
wall-clock limit and return 408) — the point is only that they never blocked the
API. All prior SQL tests stay green through the new execution path, including the
timeout test (streaming queries are still dropped at the limit on the isolated
runtime).

**Upstream watch (unchanged scope):** turso's sdk-kit has `Connection::interrupt`
in the engine but the SDK does not surface it; when it is exposed we can
interrupt even a single non-yielding aggregate instead of merely containing it.
Tracked in docs/research/turso-scale.md.

Files: `crates/app/src/v1/sql.rs`, `crates/app/tests/sql.rs`.
