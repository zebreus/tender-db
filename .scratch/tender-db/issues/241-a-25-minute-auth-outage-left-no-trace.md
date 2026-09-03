# 241 — a 25-minute outage of every authenticated endpoint left no trace in any probe, and nothing bounded the hang

Status: BOTH GAPS CLOSED IN CODE 2026-08-21 — gap 1 DONE and VERIFIED ON PROD 2026-08-19; gap 2's
sizing question answered and the bound built (below). Remaining: deploy (queue busy with the
issue-234 merge run) and the prod probe during a real long job per the acceptance
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

---

## Gap 2 closed in code (2026-08-21, owner) — the sizing question answered, then the bound

**The edge check the status line demanded, first:** the nginx vhost proxies everything through ONE
location block, and that block sets `proxy_read_timeout 24h` / `proxy_send_timeout 24h` — put there
for SSE ("never time out the stream"), but scoped to `/`. So the edge holds a silent upstream for a
DAY: during 240 nothing would have cut the hang at 60s, because the SSE exemption swallowed the
default. The first real bound must be in-process, and the SSE exemption must be surgical rather
than vhost-wide.

**The bound:** `request_deadline` — an axum middleware on the `/v1` set (layered after the
governor, so it wraps the governor's own wait; `/health`, `/metrics`, `/docs`, `/_source` are
outside it by registration order, as they are for the limiter). 30 s (`REQUEST_DEADLINE`), far
above `/v1/sql`'s 10 s in-handler cap, so it can only fire on a request that is already an outage.
On elapse: 503 in the JSON envelope naming the cause ("no response within the 30s service bound — a
stalled internal wait, not your request; safe to retry"), which lands in the nginx access log with
a status — the silent-hang class is gone. Extractors are covered because they run inside the
wrapped route service — exactly the layer 240 proved unbounded.

**SSE:** exempt by `Accept: text/event-stream` (the stream shares its path with the JSON
collection endpoints, so a path exemption cannot work). The vhost's 24h read timeout remains
correct for what actually streams.

**Visibility:** `tender_db_request_deadline_hits_total` on `/metrics`, never reset — the writer
queue gauges say a stall IS happening; this says one already turned into a 503.

Gates: `a_stalled_request_ends_in_a_503_not_silence` (503 + envelope cause + counter moved; a quick
request passes untouched), `an_event_stream_request_is_exempt_from_the_deadline` (still in flight
well past the deadline).

**Still owed for the acceptance:** the prod probe during a real long-running job — hold the writer
or ride a fold, confirm a stalled `/v1` request answers 503 within ~30 s and the counter moves.

## 2026-09-03 — the high-water mark had no timestamp; now a wait ≥ 10 s is journaled

Read during the 304 campaign (03:5x UTC, service up since 2026-09-02 13:26 UTC):

| | |
| --- | --- |
| `writer_acquisitions_total` | 7,195,090 |
| `writer_wait_seconds_total` | 763.2 |
| `writer_longest_wait_seconds` | **752.7** |
| `request_deadline_hits_total` | 0 |
| `writer_queue_depth` | 0 |

One wait held 752 of the 763 seconds ever waited — a single caller sat 12.5 minutes
behind the writer, once, and no request was cut, so it was an internal path (the
supervisor's own bookkeeping, most likely behind one of the two re-parses'
`stamp_stale_for_profiles` UPDATE of 3.5M tenders). Which one, and when, the box
cannot say: the gauge is a never-reset maximum by design, the journal has no line
for it, and `Db::conn` has no caller label. That is exactly the "no trace" this
issue is about, one level down from the request.

Landed directly (small): `Db::conn` now journals
`[store] writer acquired after a N s wait (K caller(s) still queued)` when a
single wait reaches `SLOW_WRITER_WAIT` (10 s). The timestamp is the attribution —
whichever job the journal shows holding the writer around it. The threshold is
far above the ordinary acquisition (the mean is microseconds) and, at this scale,
fires about once per campaign, so it is not noise. No test: a stderr line behind
a constant; the contention path itself is pinned by the existing test.

## 2026-09-03 09:3x — the depth gauge itself had a leak; fixed

After the 304 campaign's fold, `/metrics` read `writer_queue_depth 2` on an idle
box with no job running. The two were issue 256's queue-persist give-ups
("persist queued job 613 gave up after 30s — a long job is holding the writer"):
`Db::conn` did `depth += 1` before the lock and `depth -= 1` after it, and a
`tokio::time::timeout` that drops the waiting future skips the second half. Each
abandoned wait left the gauge one higher for the life of the process — the
"sustained non-zero depth" alarm this issue built, ringing for nobody.

Fixed: the count is a drop guard (`Queued`), released whether the wait resumes
or is dropped. Test `a_waiter_that_gives_up_leaves_no_ghost_in_the_queue_depth`
holds the writer, times a waiter out at 50 ms, and asserts depth 0 while still
holding. The service restart at the 09:03 deploy cleared the two ghosts; the fix
itself deploys with the next bundle. The ≥ 10 s wait line (above) is unaffected —
it runs only after an acquisition.
