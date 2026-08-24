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
