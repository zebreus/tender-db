# 17 — Isolate SQL-endpoint execution on its own runtime

Status: resolved, and the isolation is now **MEASURED** (run-driver, task #25,
2026-08-03) — it **HOLDS at 2x oversubscription**: 4 concurrent pathological queries
against the 2-thread `sql-exec` pool, main API unaffected. Scope is not arbitrary
concurrency; see "Measured result" at the end for what the number does and does not
license. It protects `/v1/sql` only. (An earlier version of this line called it
load-bearing for the Class B confinement in `223330a`. That was wrong — separate pools;
see the correction below.)
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


## What would falsify this — a design, because the claim has never been tested (sdk-vendor, 2026-08-03)

This issue is marked resolved on the strength of the code being written. **The property it
claims — that a pathological `/v1/sql` query cannot starve the API/SSE/dashboard runtime —
has not been measured.** `sql.rs`'s own header leans on it: *"the backstop bounds the
RESPONSE and frees the concurrency slot; issue 17's isolation bounds the BLAST RADIUS."*

**CORRECTION (sdk-vendor, same day): I first wrote that the Class B confinement deployed
in `223330a` rests on this. It does not, and the distinction matters enough to state
plainly — otherwise "load-bearing for the deploy" reads as "the deploy is unverified".**
They are two separate runtimes, verified in the code:

| mechanism | thread pool | status |
|---|---|---|
| Class B confinement (task #5) | `slow-read-exec` — `v1/isolate.rs:113` | **measured directly** (99.9% on the rig; threads verified present on prod) |
| `/v1/sql` isolation (this issue) | `sql-exec` — `v1/sql.rs:200` | **never measured** |

So this is a pre-existing gap **beside** what shipped, not a foundation **under** it, and
the `sql.rs` header cites it for `/v1/sql`'s own protection. My error was the day's usual
shape: I saw the header citing issue 17, and generalised "load-bearing for `/v1/sql`" into
"load-bearing for the deploy" without checking which pool the confinement actually uses —
a subset claim carried onto the whole by the sentence it landed in. One `git grep` for the
two thread names settles it, and I ran it only after being corrected.

That makes it the same shape as the `?kind=` density assumption and `notices_fetch_id`'s
"tens of seconds": a claim true-by-construction when written, load-bearing later, never
checked against the thing it describes.

### The measurement, and the one way it is easy to get wrong

**Falsification target:** with the isolated runtime saturated, latency on an ordinary
endpoint is unchanged.

1. **Saturate the isolated pool, do not merely occupy it.** The runtime is *1–2 threads*.
   **One pathological query cannot falsify starvation** — it leaves a thread free, so a
   clean result proves only that one query fits. Run **N ≥ threads + 1** concurrent
   pathological `/v1/sql` queries. Getting this wrong yields a confident green that tested
   nothing, which is this project's most repeated failure.
2. **Verify the load is actually pathological** — each query must still be running when the
   latency samples are taken. A query that finished early tests an idle system.
3. **Measure an endpoint that shares the runtime under test**, not `/health` if health is
   served from somewhere else. Check which runtime serves the probe before trusting it.
4. **Take a control in the same run**: identical latency samples with the isolated pool
   idle, same box, same minute. Without it, degradation is indistinguishable from load
   elsewhere — and this box is shared by four sessions.
5. **State the concurrency the conclusion is licensed for** (rule 7). "N=3 did not degrade
   it" says nothing about N=50. If the isolated pool has a queue, the interesting question
   is what happens when the queue is deep, not when it is short.

### What a pass and a fail each mean

* **Pass** — ordinary-endpoint latency is flat while the isolated pool is saturated:
  isolation holds *at that concurrency*, and the record should say which.
* **Fail** — latency degrades: the isolation is nominal, and every downstream claim that
  cites it (including the Class B confinement) inherits the gap.

**A third outcome is likely and must not be read as a pass:** the pathological queries are
killed by the 10 s cap before the isolated pool is ever saturated. That measures the
*backstop*, not the isolation, and the two are explicitly different mechanisms in
`sql.rs`'s own account. If the cap fires first, the isolation is untestable through that
path and the record should say so rather than record a green.

## Measured result — the assumption HELD (run-driver, task #25, 2026-08-03)

Ran against the falsification design above, all three traps executed:

* **Provable saturation, not one query leaving a thread free.** A *second* rig token was
  needed to fire **4 concurrent** queries at the **2-thread** pool, because the per-token
  limit caps a single credential at 2. Without that, the run would have tested a pool with
  spare capacity and reported a green that meant nothing.
* **The cap was distinguished from the isolation — and it mattered.** The 10 s cap fires
  **but does not stop the work.** "Still running" therefore could not be measured by
  whether requests were outstanding; run-driver measured it by **thread CPU**. That made
  the *post-cap* window the strongest evidence: **21 s of `sql-exec` burn while the main
  API stayed at 0.01 s and every request had already returned.**
* **Verdict:** `/v1/sql` isolation is real. `sql.rs`'s "the backstop bounds the RESPONSE
  and frees the concurrency slot; issue 17's isolation bounds the BLAST RADIUS" is now a
  measured statement rather than a design intention — and the two halves were observed
  doing *different* jobs in the same run, which is what the header claims.

### What the number licenses, and what it does not

**Measured: 2x oversubscription (4 against 2).** It does **not** license arbitrary
concurrency — a deep queue is a different question, and rule 7 applies: the conclusion is
licensed for the concurrency actually varied.

The bound is less arbitrary than it looks, though: the **per-token limit caps a single
credential at 2**, which is why a second token was needed to reach 4. So 2x is roughly the
worst a single credential can impose, and exceeding it requires multiple credentials —
worth re-measuring if that limit ever changes, since the licensed scope moves with it.

### Worth recording that it held

Most of today was assumptions failing under measurement. This one held, and that is the
other half of why the discipline is worth the effort: a check that can only ever confirm
what you feared teaches you to stop running checks. The value was never in the answer
being bad — it was in the answer being *known*.