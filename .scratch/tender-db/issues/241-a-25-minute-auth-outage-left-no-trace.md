# 241 — a 25-minute outage of every authenticated endpoint left no trace in any probe, and nothing bounded the hang

Status: gap 1 DONE and VERIFIED ON PROD 2026-08-19; gap 2 still open — needs the edge's timeout
behaviour checked first
Kind: observability gap + missing bound on the request path
Blocked by: —
Relates to: 240 (the outage that exposed both), 61 (`/health` reads the in-memory cursor by design),
238 (`/v1/sql`'s in-handler backstop, which cannot cover an extractor), 53 (`/metrics`), 90
(unobservable incremental projection — the same "it was happening and nothing said so" class)

## What issue 240 showed, beyond its own bug

For 25 minutes every token-bearing request to every `/v1` endpoint hung with no response and no error.
During that window:

    /health          200 in 0.4 ms     (in-memory cursor by design — issue 61)
    /health/deep     200 in 0.6 ms
    /metrics         200 in 2 ms
    /admin/jobs      answered normally, job progressing normally
    /v1/tenders      200 in 5 ms       (reader pool, unaffected)

Nothing was red. Nothing was slow. The job record showed a healthy run. The outage was found only
because a query happened to be run by hand while the fold was in flight — and the first two symptoms
were misread as a slow query and then as SQL-runtime saturation before the invalid-token probe localised
it.

Two separate gaps, both worth closing on their own terms.

## Gap 1: no probe exercises a writer-dependent path with a bound

`/health` is instant by construction, which is right for a liveness probe (issue 61 made it so
deliberately, and that decision should stand). But it means the entire class of "requests are waiting on
the writer" is invisible to every signal we have. The measurement that would have caught this is cheap:
attempt something that needs the writer mutex, with a short deadline, and report the wait.

Sketch, not a design: a `writer_wait_seconds` gauge on `/metrics`, sampled by trying `try_lock` on a
timer and recording how long the writer has been continuously held. A held-for-25-minutes writer is
normal during a fold — so the alertable signal is not "held" but "held while requests are queued behind
it", which is the pair worth exporting.

## Gap 2: nothing bounds an extractor, so a stall becomes a hang

`/v1/sql` has a backstop that returns 408 at its cap and sheds with 503 when the runtime is saturated
(issue 238). Both live inside the handler, so neither can fire for a request that never reaches it. Any
work an extractor does — auth today, anything else tomorrow — is unbounded by construction.

A bound belongs at the layer that covers everything: a tower timeout layer on the `/v1` router (or on
the auth extractor specifically) returning 503 with a cause. The value is not that it fixes a cause —
240's cause is fixed properly — it is that the NEXT unbounded wait becomes a legible error instead of a
silent hang, and shows up in the access log with a status.

Sizing note before building: nginx already has its own upstream timeouts, so part of this may be
present at the edge and merely invisible in-process. Check what the edge does with a 25-minute upstream
silence before adding a second timeout with different semantics (issue 97 — the access log has no
request timing — is in the way of answering that from logs alone).

## Acceptance

- Some signal, checkable without running a query by hand, distinguishes "a job holds the writer" (normal)
  from "requests are queued behind the writer" (an outage).
- A request that waits on anything unbounded ends in a status code rather than in silence.
- Both verified the way 240 was: by probing prod during a real long-running job, not only in a test.


---

## Gap 1 closed in code (2026-08-19, owner) — the queue is now measured

`Db::conn` is the single choke point for every writer acquisition in the store crate (83 call sites
route through it), so the counters live there and cover all of them:

    tender_db_writer_queue_depth           callers blocked waiting for the writer right now
    tender_db_writer_acquisitions_total    the denominator for a mean wait
    tender_db_writer_wait_seconds_total    cumulative seconds spent waiting
    tender_db_writer_longest_wait_seconds  longest single wait since open (never reset)

Design points, both from this issue's own framing:

- **Depth, not held.** A held writer is normal — a fold holds it for its whole transaction — so
  "held" is not alertable and "held while callers are queued behind it" is. Sustained non-zero
  `queue_depth` is exactly that state, whoever holds the lock, and it would have read non-zero for the
  full 25 minutes of issue 240.
- **A high-water mark that never resets**, so a stall stays visible after it ends. During 240 the
  outage was over before anyone could look; a gauge that only shows "now" would have been green again
  by then.
- **No cost when uncontended.** `try_lock` fast path first: an ordinary write takes two relaxed
  atomics and records no wait, so the instrument cannot invent contention.

The test (`a_held_writer_with_callers_behind_it_is_visible_while_it_happens`) parks three callers
behind a held writer and asserts the depth gauge reads 3 **while they wait** — a gauge that only moved
after the fact would be useless for the alert it exists to raise — and that the high-water mark is one
wait rather than the sum.

### Gap 1's acceptance, met on prod (2026-08-19)

Verified during a real writer-heavy job — the staged text-era re-parse (issue 244), which acquires the
writer about **390 times a second** while it rewrites a package's parse layer. Scraping `/metrics`
through the run:

    tender_db_writer_acquisitions_total   12,125 → 17,509 → 22,112 → 26,094   (~390/s)
    tender_db_writer_queue_depth          0
    tender_db_writer_wait_seconds_total   0

Depth and waits at zero while a job hammers the writer is the CORRECT reading, and worth stating: this
job takes the writer briefly and often, so nothing queues. It also means the run alone could not
demonstrate the alerting half, so contention was then created deliberately — three job enqueues (each a
writer-side row) fired into that stream:

    tender_db_writer_acquisitions_total   33,206
    tender_db_writer_wait_seconds_total   0.016472639
    tender_db_writer_longest_wait_seconds 0.003961235

Both wait counters moved off zero, and the high-water mark **persisted after the contention ended** —
the property issue 240 needed and did not have. The magnitude is the point of comparison: 4 ms here
against the ~1,500 s a 25-minute stall would record, so the gauge distinguishes ordinary contention
from an outage by three orders of magnitude rather than by a threshold anyone has to tune.

One thing this run also settles: a fold is not the only writer shape. Issue 240's outage came from a job
HOLDING the writer for a whole transaction; a re-parse instead takes it hundreds of times a second.
`queue_depth` catches the first, the wait counters catch the second, which is why both are exported.

### Gap 2 is deliberately still open

Bounding the request path is a separate change with a real prerequisite this issue already names:
find out what nginx does with a 25-minute upstream silence before adding a second timeout with
different semantics. Doing that from logs is blocked on issue 97 (no request timing in the access log),
so the honest next step is a deliberate probe against the edge, not a tower layer added on assumption.
