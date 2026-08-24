# Measured capacity — the on-box campaign log (issue 167)

Rig per the 2026-08-23 decision (issue 167): the LIVE box, quiet windows, localhost
curl so network noise is out of the numbers. Server rev `d50df2e`, 4 vCPU / 8 GB,
corpus 7.9M tenders / 14.3M notices. Every row records the real HTTP code — a
sub-millisecond "fast" probe is a 400, not a result (that trap produced the first,
discarded, run of E1).

## E1 — per-endpoint worst-case map (first pass, 2026-08-24 ~04:00 UTC, idle box)

5 sequential requests each, `%{time_total}`, first is cold:

| shape | result |
|---|---|
| `/v1/tenders` default | 31ms cold, 24–32ms warm |
| `/v1/tenders?limit=100&cursor=1000000` (deep cursor) | 61ms cold, 17–22ms warm |
| `/v1/organizations?name_prefix=stadt` | 28ms cold, 2–3ms warm |
| `/v1/changes?since=0&limit=1000` | 3–6ms |
| `/v1/sql` `SELECT country, COUNT(*) FROM tenders GROUP BY country` | 1–5ms (index-only) |
| **`/v1/tenders?cpv=45&min_value=100000&sort=deadline`** | **17.8s cold, 4.67–4.72s warm, every time** |

The last row is the un-walkable class capacity-model-v0.md §A predicted: a filter
combination no index drives, re-scanned per request, no caching between identical
requests. With issue 120's measured no-cancellation, an abandoned request still
burns the full 4.7s of a reader.

Concurrency 4 on that shape, `/health` probed each second alongside:
q1 4.97s, q2 5.51s, q3 6.38s, q4 5.51s — near-linear absorption (≤35% dilation),
and `/health` stayed 0.4–0.6ms throughout (it bypasses the reader pool; the SLO
evidence we wanted).

**First budget line:** the worst REST class sustains **< 1 request/second for the
whole box** (4 cores × ~4.7s each). The 10 rps/IP posture is an order of magnitude
too generous for this class; per-shape admission (the walk-router's classes) is the
lever, not a global rate number. Cheap-class endpoints (lists, cursor walks,
prefix lookups) are 3 orders of magnitude cheaper and the posture is fine for them.

Redo next pass (my probe shapes were 400s, so unmeasured): `winner=` and `status=`
filter vocabularies; then E2 (SSE fan-out curve) and E3 (hostile-SQL swarm at the
2-concurrent cap), E4 (mixed soak during the 09:35 daily).

## E1 redo + E3 — filter vocabularies and the hostile-SQL swarm (2026-08-24 ~05:00 UTC)

E1 redo (the two shapes that 400'd; vocabularies: `status` ∈ open|closed, `winner` = org id):

| shape | result |
|---|---|
| `status=open` (the default user view) | 1.73s cold, **0.55s warm** — a real mid-class |
| `status=closed&country=DE` | 62ms |
| `winner=<org id>` | ≤3ms (indexed) |

E3 — hostile-SQL swarm at the cap. Two concurrent full-scan `COUNT(*)` over
`tender_version_amounts`, plus probes during and after:

- Both hostile queries: **HTTP 408 at 11.0s** — the sandbox's 10s budget FIRES.
- A third SQL during the swarm: **429 in 1ms** — the 2-reader gate rejects instead of
  queueing. No pile-up surface.
- REST during the swarm: 18–51ms (idle baseline 17–32ms) — the SQL pool is isolated
  from the REST readers; hostile SQL cannot starve the API.
- After the swarm: REST normal, CPU 93% idle — **the 408 frees the reader.** Issue
  120's cannot-cancel finding applies to REST walks, NOT to the sandbox; the abuse
  surface the model feared (zombie readers burning after timeout) does not exist for
  /v1/sql.

Correction to the model's rig assumptions: the box reports 64 GB RAM (buff/cache
51 GB), not 8 GB. CPU count re-checked next pass.

Remaining: E2 (SSE fan-out driver), E4 (mixed soak — observe the 09:35 daily live),
and the budget write-up deriving the rate limits (status=open at 0.55s warm needs a
line of its own: ~7 rps/box saturates on it).

## E2 — SSE snapshot + the walk pool, and a real bug (2026-08-24 ~06:00 UTC)

SSE snapshot streams rows as `data:` lines (no `event:` prefix), one per matching
row, then a `live` marker, then silence/keepalive. Measured snapshot rate for
`status=open`: ~8,000 rows/120s (~66 rows/s, ~15 KB/s) — the snapshot is itself a
walk and paces at the walk's row rate, so a large filter's snapshot is a minutes-long
reader hold. Small filters complete instantly (LU/CY: empty, 3 bytes).

**Bug found and filed as issue 273** (the campaign's highest-value output so far): the
combination `status=open` + a low-volume `country` walks the whole ordered stream and
hits the **30s service bound → 503**, on an idle box, while either filter alone is
fast. It is uncancellable (issue 120) and the isolated walk pool is only **4 slots**,
so **four trivial requests brown out all walk-capable reads for 30s** — the concrete
abuse budget this campaign existed to find. The rate posture (10 rps/IP) does not
defend it. Fix directions in 273; the capacity write-up will fold in per-shape
admission as the structural answer.

Revised budget picture: the binding constraint is not a global rps number, it is the
**4 walk slots × their hold time**. A walk that can hit 30s makes the whole
walk surface a 4-request DoS. Bounding walk time (issue 273 fix 1) is therefore a
prerequisite for any rps budget to mean anything.

Still to run: E4 (mixed soak observing the 09:35 daily), CPU-count recheck, and the
budget write-up once 273's fix direction is chosen.

## Hardware correction (2026-08-24)

The box is **32 vCPU / 62 GB RAM**, not the 4/8 the model assumed (capacity-model-v0.md
§B rig line). This MATTERS for interpretation: the earlier "4 cores × 4.7s" framing of
the un-walkable class is wrong — there are 32 cores. The walk pool is 4 slots by
**policy** (`isolate.rs` ISOLATED_READERS = SLOTS = 4), not because the box has 4 cores.
So the binding constraint on walk traffic is a deliberately small pool in front of an
otherwise-idle 32-core machine — which makes both levers (widen the pool / bound the
walk) cheap in hardware terms.

## The capacity budget (deliverable, capacity-model-v0.md §5) — derived 2026-08-24

Rate limiting must be **per-shape**, because the measured cost per request spans five
orders of magnitude. Three classes:

| class | example | cost/req | box capacity | today's posture | verdict |
|---|---|---|---|---|---|
| **cheap** (indexed seek / bounded range / cursor page) | default list, `winner=`, `country=DE`, `changes`, cursor walk, prefix org lookup | 2–60 ms | thousands/s (main reader pool, not the walk pool) | 10 rps/IP, burst 50 | **fine** — leave as is |
| **mid** (bounded-range walk) | `status=open` | 0.55 s warm | 4 walk slots ⇒ ~7 concurrent-req/s steady before the pool saturates | 10 rps/IP | **too generous per-IP**; the pool, not the IP limit, is the real bound |
| **heavy** (unindexed version-predicate walk) | `cpv=45&min_value=…&sort=deadline`; any 2 version-predicates on a sparse result | 4.7 s → **30 s → 503** (issue 273) | 4 walk slots total for the WHOLE box | 10 rps/IP | **undefended** — 4 requests brown out all walk traffic |

Derived limits (proposal, feeds CONTEXT.md's posture once 273 lands):

- **SQL**: keep. Measured safe — 10 s budget fires (408, reader freed), 2-concurrent
  gate sheds with 429, fully isolated from REST. 2 concurrent / 300 per hour / 10 s
  per query needs no change.
- **Cheap REST**: keep 10 rps/IP + burst 50. The main reader pool absorbs it.
- **Walk REST is the whole game.** The scarce resource is 4 walk slots × hold time,
  not any per-IP number. Two things, in order:
  1. **Bound walk time** (issue 273 fix): no walk may run to the 30 s service bound.
     Cap scanned rows; return a short result or a fast 422 "narrow your filter." Until
     this lands, a per-IP rate limit is theatre — 4 requests from 4 IPs still brown out
     the surface.
  2. **Meter walk slots, not just IPs**: an admission counter on the isolated pool
     (e.g. ≤2 of the 4 slots per IP) so one client cannot occupy the whole pool. With
     32 idle cores the pool can also simply be **widened** (ISOLATED_READERS 4 → e.g.
     12) — cheap here, and it raises the mid/heavy ceiling proportionally. Widening is
     not a substitute for (1): an unbounded walk just browns out a bigger pool slower.
- **SSE**: the snapshot is a walk (≈66 rows/s) and holds a walk-class reader for its
  whole duration; a large-filter subscription is a minutes-long hold. The 5-streams/IP
  cap bounds fan-out count but not snapshot cost — so SSE belongs under the SAME walk
  budget as heavy REST, and (1) applies to snapshots too (issue 55 already paginates
  them for memory; this is the time dimension).

**Bottom line for launch:** the rate *numbers* are mostly fine; the missing piece is
that walk-class reads (heavy REST filters + SSE snapshots) share one small uncancellable
pool with no per-client metering and no time bound. Issue 273 fix 1 is the prerequisite;
pool metering + optional widening is the follow-up. This closes 167's research half —
the remaining E4 soak is confirmatory, not blocking.
