# 120 — the app has no defence against expensive-query saturation

Status: open — architectural gap, established 2026-08-03 while costing issue 117 Class B.
Kind: availability / architecture
Blocked by: —
Blocks: —
Priority: medium — no live incident, but it is the reason every DoS-shaped read defect so far has had to
be fixed one query at a time.

## The gap

**The public API has 8 reader connections, and nothing can stop a query once it starts.** Those two
facts together mean any single expensive query saturates the API, and the app cannot intervene.

Measured, both on the deployed turso 0.7.0:

- **`READERS = 8`** (`crates/app/src/main.rs`) — "the API's real concurrency".
- **A walk cannot be aborted.** 4M rows, filter matching nothing, a 50 ms `tokio::time::timeout`
  wrapped around the read exactly as an app-layer guard would be written:

  | runtime | walk | budget | returned after |
  |---|---|---|---|
  | current_thread | 0.4408 s | 50 ms | 0.4426 s |
  | multi_thread (production's flavour) | 0.4485 s | 50 ms | 0.4533 s |

  The timeout never fires. `Statement::step` yields only on IO, so a warm walk never returns to the
  executor, and `timeout` polls the inner future and the sleep from the *same task* — so more worker
  threads do not help. Probe: `crates/store/tests/scan_budget_probe.rs`.
- **An abandoned request keeps running** — ~65 CPU ticks per 10 s window for 60 s+ after the client was
  killed, against a 0-tick idle baseline (run-driver). A disconnect frees the socket, never the work.

## Why this is bigger than any one query

Issue 117's `?country=` matches-late is *one instance*. The class is "any query whose cost the planner
and the app both fail to predict", and every member has had to be found and fixed individually — the
2.2 s lots walk, the 248.8 s `tender_detail`, the 22 s / 99 s / 226 s / >380 s paginated filters.

The reason they had to be fixed one at a time is precisely this gap: **there is no backstop.** A system
that could bound per-request work would have degraded gracefully on all of them instead of serving
minutes-long responses, and the fixes would have been performance work rather than availability work.

An app-layer budget cannot supply that backstop:

- it cannot **abort** — measured above;
- it can only **refuse before starting**, which needs a cost oracle predicting *this query will be
  expensive*. We have none. There are no selectivity statistics, and the one probe we do have (issue
  117's existence short-circuit) answers only "does any row carry this value", which is the
  matches-*nothing* case. For matches-*late* the value exists, so nothing pre-flight distinguishes it
  from a cheap query.

## The only true fix, and its cost

**Cancellability.** `turso::Connection` exposes no `interrupt()`; the capability exists one layer down
in `turso_sdk_kit::rsapi` (established by run-driver while assessing issue 117). Reaching it means
vendoring or forking a pinned dependency.

That is a permanent obligation on the engine we deploy, taken on to fix a class rather than an
instance. It is the right shape and possibly not the right trade — which is why this is filed as its
own item rather than folded into 117, where it would have been decided as a side effect of a
ten-second defect on a rare filter.

## What is being done instead, and its limit

Issue 117's residual (~10 s on a rare-but-real code) is accepted and bounded **at the ingress** with
`limit_req`, not in the app. That control is proportionate for that defect, and its own limitation must
be recorded with it: **`limit_req` sees arrival rate and cannot see that every admitted request holds a
reader to completion.** So the limit is derived, not chosen —

```
admitted_rate x walk_duration < pool_size
```

— which at 8 readers and a ~10 s walk means well under 0.8 req/s per key. A limit picked as a
plausible-looking requests/sec would look protective and not be.

**Note the ingress control does not close this issue.** It bounds one known defect's amplification. It
does nothing for the next expensive query nobody has found yet, because rate-limiting cannot
distinguish a cheap request from an expensive one either.

## Verification note

`READERS = 8` is read from source. Before any limit is derived from it, **confirm the value in the
deployed configuration** — an environment override would silently invalidate the arithmetic. Know the
inputs of the derivation, not just its form.

## The isolation backstop, built (`f67aa90`) — and what it does not yet claim

Walk-capable reads now run on a dedicated runtime with their own reader pool, behind a semaphore that
**sheds rather than queues**. Routing is `store::read::walks`, derived from the read layer's own
predicates rather than a maintained list — which is how it caught `/v1/tenders?kind=`, a member issue
117's audit had missed. **The backstop caught an unaudited expensive read before shipping, which is
what a backstop is for.**

Slots, runtime threads and pool connections all equal **4**, and the first two are tied by a
compile-time assertion. That is correctness, not tuning: a walk cannot be cancelled, so an admitted
request occupies a thread until the query ends naturally, and a surplus slot would queue behind it —
the exact starvation `try_acquire`-to-shed exists to prevent, relocated inside the sandbox where it is
harder to see. It was 2 threads against 4 slots until review caught it.

The permit is moved **into** the spawned task, so it tracks the QUERY and not the caller. `AbortOnDrop`
fires when the handler stops waiting, but `abort()` only lands at an await point the task cannot reach
until the query returns — so the future cannot drop mid-query and the permit cannot release mid-query.
**The non-cancellability that makes this problem hard is what makes the permit honest.** Were it
otherwise, a new request would be admitted while an abandoned one still burned a thread: ~2.77 cores
were measured still burning from clients that had exited minutes earlier.

### What it explicitly does not do

It does not reduce the work, does not bound any single request's duration, and sheds the (N+1)th
concurrent walk-capable request with a 503. The semaphore is **global** — these endpoints are
unauthenticated, so there is no token to key on and per-IP is spoofable and belongs at the ingress. A
heavy client can take all four slots and shed a second legitimate one; that degrades other rare-filter
requests and never the browsing path, and keying is the lever if an observed 503 rate shows it.

### The guarantee is UNVERIFIED, deliberately stated as such

Nobody has confirmed — for `/v1/sql` either, where it was assumed from the construction — that a
runaway's burn actually lands on the isolated threads rather than the main API workers. The worker
threads are named `slow-read-exec` precisely so `/proc/<pid>/task/*/stat` deltas can settle it, and the
ship gate is that measurement on this endpoint, not the code compiling. A doc that says *"this is the
claim and here is how to falsify it"* is worth more than one asserting a property nobody checked.

### Open: does an abandoned walk terminate? (task 22)

This decides the WORDING, not the design — shed is right either way — but the difference is not
cosmetic:

- **If walks terminate**, `SLOTS` is a concurrency limit and "confines a runaway" is accurate.
- **If they do not**, permits never return and `SLOTS` is a **countdown**: four requests and the
  filtered endpoints are permanently unavailable. That is still a strict improvement — the main API
  survives, which today it does not — but the honest description becomes *"trades the filtered reads to
  save the rest"*, and containment would need a process-level kill (respawning the executor to reclaim
  stuck permits), which is a materially different piece of work.

The module currently claims the first. If the measurement says the second, the doc gets corrected rather
than left asserting what the evidence does not support — the same correction the `/v1/sql` timeout claim
required.
