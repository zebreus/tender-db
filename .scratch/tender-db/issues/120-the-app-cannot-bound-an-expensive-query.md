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

### RESOLVED: abandoned walks terminate (task 22)

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

**Answered by measurement on a dedicated bed** (`probe.db`, 13.2M real lots, 100% `kind='lot'` — the
dense worst case): `/v1/lots?kind=Lot` **completed in 231.6 s**. Finite, O(table). So permits do
release, the endpoint recovers unaided, and `SLOTS` is a tuning knob rather than a countdown. **The
expensive branch is closed: no process-level kill, and no separate issue for one.**

The expectation was right; the reasoning behind it was not why to trust it. "A finite scan must
terminate" is plausible, but the place unboundedness could have hidden was the **top-level sorter over
the matched set**, not the scan — and prod's residue (~2.6 cores at 50 minutes) was a confounded
instrument that could not distinguish a slow-draining backlog from genuine non-termination. A
single-query bed could.

So the module claims **"confines a runaway, with a bounded recovery time"**. Two limits belong with that
claim rather than after it:

- **Bounded is not small.** Nothing cancels on disconnect, so every abandoned request costs its FULL
  runtime of executor capacity with nobody waiting for the answer. A retrying client ACCUMULATES load
  rather than replacing it. Size `SLOTS` against **arrival rate × full query runtime**, never against
  concurrent clients — an ingress rate limit bounds arrival and does nothing about in-flight
  accumulation.
- **231.6 s is a lower bound on prod's per-query cost, not an estimate.** The bed is the dense worst
  case at smaller scale than prod. The termination conclusion transfers; the number does not, and
  nothing should be sized off it.

Prod's residue is consistent with this model rather than with non-termination: ~2.6 cores × 50 min ≈
7,800 core-seconds ≈ 20–30 runs of a 230 s-class query, against more than a dozen heavy uncancelled
reads fired that day.

---

## Observed on prod, 2026-08-04: bounded duration is the claim that does not hold

> **A 14-minute burst produced at least 47 minutes of degradation — a blast radius
> more than 3x the traffic that caused it, and still open when this was written.**
> That ratio, not any single slow read, is why this issue is reopened: it is what
> turns "a slow endpoint" into "a self-sustaining degradation", and it is invisible
> on a request-rate graph, which shows the 14 minutes and nothing after.

The 47 minutes is a **lower bound, not a measurement**: the burst ended 09:33:44 UTC
and two `slow-read-exec` threads were still occupied at 10:20:33, when this was
recorded. The true figure is larger by however long it kept running. Stated as a
bound rather than rounded up, for the same reason `thread_cpu.sh` reports occupancy
as a lower bound — an honest floor beats a confident guess, and the floor is already
enough to carry the argument.

Status: REOPENED (task #31). The section above closed the expensive branch on the
grounds that a runaway *terminates*. It does. What was never bounded — and what
prod demonstrated — is **how long termination takes**, and that is the property the
`SLOTS` shed does not address.

**The observation.** Lennart's acceptance test ran against the live endpoint from
09:19:28 to 09:33:44 UTC: 239 requests, including `/v1/tenders?limit=2000`,
`/v1/tenders?include_data=true` and ten `/v1/sql` calls. Legitimate traffic from the
owner; nothing here is about the traffic being wrong.

**The finding is what happened after it stopped.** At 09:58 — **twenty-six minutes
after the last request** — the box was still doing real work, with every other
explanation eliminated:

    no new API traffic        nginx: only bot noise (GET / 400/404, robots.txt) since 09:33:44
    no supervisor job         /admin/jobs: {"current": null, "queued": []}
    not a sort spill          turso temp DB 4,096 bytes, static
    not an external reader    the reads are attributed to the `server` PID
    still 26 MB/s             sustained, from the block device (read_bytes, not rchar)
    sql-exec                  in D
    slow-read-exec            2 of 4 slots, D and R

So one of the `/v1/sql` calls issued around 09:31:37 was **still executing at 09:58**,
with the client long gone and nothing able to stop it.

**Load did decay — 4.49 -> ~3.05 — so this is not non-termination.** The prior
section's conclusion stands. The correction is narrower and sharper:

> **The shed bounds CONCURRENCY (4 slots). Nothing bounds DURATION.**
> Four unlucky queries can therefore pin the whole pool indefinitely, and the
> "bounded recovery time" the module claims has no bound anyone has measured.

This is why sizing `SLOTS` against *arrival rate x full query runtime* (already stated
above) is not merely conservative but load-bearing: the runtime term is unbounded, so
the product is unbounded, so no finite `SLOTS` makes the pool safe by itself.

**The operational consequence is the part worth carrying.** This was *self-sustaining
degradation from a burst that had ended half an hour earlier*. Every read arriving
afterwards competed with it. The incident's blast radius in **time** far exceeded the
traffic that caused it — a materially different risk from "the API was hammered for
fourteen minutes", and one that a request-rate graph would not show at all.

**A second, quieter defect: none of this was visible.** The app logged NOTHING between
08:03:21 and the time of writing. A request logs on completion, so an in-flight
expensive read appears nowhere — not in the journal, not in `/admin/jobs` (it is not a
job). The only instruments that identified it were thread names (`slow-read-exec` =
the issue-5 pool, `sql-exec` = the issue-17 pool), `wchan`, and `/proc/<pid>/io`. A
runaway read is currently diagnosable only by someone who already knows to look at
kernel-level process state. That is an observability gap in its own right and belongs
with this issue rather than in a separate one.

**And the measurement trap it implies, for issue 30b.** `?include_data=true` returned
63,500 bytes promptly while burning a Class B slot for over ten minutes. **The response
returning is not the work finishing.** Any sweep that clocks reads from the client side
will record this entire class as fast. 30b therefore needs a server-side completion
signal — slot occupancy or thread CPU — not a stopwatch on the response. (sdk-vendor's
observation; it changes 30b's method, not just its results.)

**Not fixed by a timeout.** turso cannot interrupt a running statement (`fb8c55c`), so
the remedy is issue 30: make the reads fast enough that none runs away. Until then the
honest module claim is *"confines a runaway to 4 slots, for an unbounded time"*.
