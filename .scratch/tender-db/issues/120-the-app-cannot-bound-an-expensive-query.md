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

> **A 14-minute burst produced ~85 minutes of degradation — a blast radius roughly
> 6x the traffic that caused it.** That ratio, not any single slow read, is why this
> issue is reopened: it is what turns "a slow endpoint" into "a self-sustaining
> degradation", and it is invisible on a request-rate graph, which shows the 14
> minutes and nothing after.

**The termination was OBSERVED, not interrupted** — which is the strongest form this
evidence can take. A controlled restart was authorised and armed for 11:10Z; the slot
released on its own at **10:59:08 UTC**, before the deadline, so the restart premise
expired and nothing was killed. Had we restarted, this would read "at least 57
minutes, censored by intervention" — a floor, not a runtime.

    last genuine API request   09:33:44 UTC   (all later traffic is scanner 404s)
    slot released              10:59:08 UTC
    abandoned, no client       ~85 minutes, run to completion

Two honesty notes on the figure:

* **It is still a lower bound on the QUERY's runtime**, though not on the abandonment.
  The clock starts at the last request of the burst, but this query may have begun
  earlier in it — the `/v1/sql` calls were at 09:31:37 — so the true runtime is ~85
  minutes *or more*. What is uncensored is the *termination*, not the start.
* **It is one observation.** The earlier instance in this same incident gives an
  independent ~26 minutes. Two points, both large, one run to completion.

An earlier draft of this section recorded 47 minutes as a still-open lower bound. That
was honest when written and is superseded rather than corrected: the phenomenon simply
kept going for another 38 minutes.

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

### Methodology: a decaying aggregate is not evidence of imminent completion

Recorded because it corrects a claim made twice in this investigation, by me.

Watching load average fall, I twice reported the box was "draining" and that the
walks "terminate, just slowly". sdk-vendor then measured the same drain per-thread
with `thread_cpu.sh`, and it was **not monotonic**:

    196%  ->  150%  ->  94%  ->  130%  ->  23%

It looked nearly finished several times before it was, and once went back up. Every
one of those dips is a point where "load is falling, it's almost done" would have
been asserted and been wrong — and my inference was drawn from an aggregate that
does exactly this. The conclusion happened to survive; the reasoning did not deserve
to. **This strengthens rather than weakens the unbounded-duration finding**: work
whose apparent progress reverses is not work with a predictable end.

The rule this yields, which is the same one this file's ANSWER/PATH/COST table is
about: **measure the artifact, not a correlate.** Load average is a correlate of
"are the Class B slots occupied". The artifact is the busy-thread count itself, and
it is directly readable. sdk-vendor's re-armed trigger — *zero busy `slow-read-exec`
threads for N consecutive samples* — is the right shape; a load threshold is not.

**But there are two grades of "the artifact", and the weaker one nearly misled the
restart decision.** Two independent watchers ran against this same slot:

    run-driver   sampled INSTANTANEOUS thread state (D|R from /proc), every 45s
    sdk-vendor   sampled CPU DELTA over an interval, every ~10s

They agreed on the outcome, and the transition times nearly agree — but not quite,
and the direction matters. run-driver's watcher read `busy=0` at **10:58:52**;
sdk-vendor's read `busy=1` at **10:58:57**, five seconds later, with the true release
at 10:59:08. An instantaneous `D|R` check sees a thread that is momentarily in `S`
between I/O waits and calls it idle. **So the weaker instrument can report a clear
while the query is still running**, and here it happened to do so within seconds of
the real clear rather than minutes before it. The restart decision was premise-bound
on exactly this reading; a momentary `S` sampled at 10:40 would have produced a
confident "CLEARED-EARLY" with an hour of work still to go.

CPU delta over an interval is the artifact; instantaneous run-state is one more
correlate, just a much better one than load average. The lesson is not "use /proc"
but that **"measure the artifact" has grades, and a check can be one level closer to
the truth and still not close enough.**

**One caveat on the instrument, pending a fix.** `thread_cpu.sh`'s per-sample
percentages are correct — calibrated against a C pthread spinner with `ps -L` as an
independent ground truth: 3 threads at 99.3% each, tool reported total=300%. Its
**occupancy summary** is not: `occupied` is keyed by thread NAME rather than tid, so
concurrently-busy threads sharing a name each increment the same counter per sample,
and the reported duration inflates by that factor (1 thread -> "2/2 samples (~6s)",
correct; 3 threads -> "6/2 samples (~18s)", true window 6s). `slow-read-exec` is
FOUR threads with one name, so any duration taken from the occupancy line for this
issue can be inflated up to 4x — **in the direction that supports this issue's
conclusion.** Until it is fixed, duration claims here should be derived from the
per-sample busy-thread counts and from request timestamps, not from that summary.
The ~50-minute figure is unaffected: it comes from the last genuine request
(09:33:44) against samples at 10:21-10:23, and its `12/12` implies a single busy
thread, where the defect is inert.

That is the third instrument in this investigation whose bug would have produced
evidence AGREEING with the hypothesis, and the second in this one tool. Agreement is
where nobody looks.
