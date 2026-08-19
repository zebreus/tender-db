# 241 — a 25-minute outage of every authenticated endpoint left no trace in any probe, and nothing bounded the hang

Status: needs-triage — split out of 240 (2026-08-19), which fixed the specific cause
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
