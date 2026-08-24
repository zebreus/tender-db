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
