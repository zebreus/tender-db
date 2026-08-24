# 273 — a rare-but-nonzero version-predicate combo walks to the 30s bound → 503 (cheap DoS)

Status: DIAGNOSED — found 2026-08-24 in the issue-167 capacity campaign (E1/E2), on the live box, idle
Kind: availability + abuse surface (correctness-adjacent: a valid query returns 503, not results)
Relates to: 117 (Class B version-predicate walks), 120 (walks are uncancellable), 167 (the campaign), 55/163 (SSE snapshot is the same walk)

## The finding, reproducible on an idle box

```
GET /v1/tenders?status=open&country=DE   → 200 in 0.25s
GET /v1/tenders?country=LU               → 200 in 0.69s
GET /v1/tenders?status=open              → 200 in 0.06s
GET /v1/tenders?status=open&country=LU   → 503 in 30.00s   ("no response within the 30s service bound")
GET /v1/tenders?status=open&country=CY   → 503 in 30.00s
```

Each filter is fine alone or with a high-volume country; the COMBINATION on a
low-volume country stalls to the 30s service bound and 503s. The SSE form of the
same filter (`Accept: text/event-stream`) returns an empty snapshot (3 bytes) —
so the answer is "≈0 rows", and producing that answer is what costs 30s.

## Mechanism (read from the code, not guessed)

`status` and `country` are both **version predicates** (`store::read::walks`,
crates/store/src/read.rs:613) — per-row `EXISTS` subqueries over the satellites,
Class B of issue 117, no index on the driven table. Combined, the walk scans the
`tenders_current_deadline`-ordered stream applying both EXISTS per row. For DE the
page of `limit` fills near the top (many open DE rows); for LU/CY almost nothing
matches, so the walk scans the WHOLE ordered stream (~7.9M rows) to fill a page it
never fills, and hits the 30s bound.

## Why it is worse than "a slow query"

- **Uncancellable (issue 120):** the client 503s at 30s but the walk keeps running
  to completion on its reader.
- **The isolated walk pool is 4 slots** (`isolate.rs`: `SLOTS = 4`, pinned equal to
  the runtime threads by a compile-time assert). So **four** trivial requests —
  `?status=open&country=CY&limit=5`, costing the caller nothing — pin all four slots
  for 30s each and brown out EVERY walk-capable read (any country/cpv/value/buyer/
  winner filter, and SSE snapshots) for that window. The isolation that protects the
  main `readers` pool (issue 120's whole point) does NOT protect walk traffic from
  other walk traffic.
- The 10 rps/IP + 5 SSE/IP posture does not defend it: 4 requests is trivially under
  budget, and they can come from 4 IPs.

## Fix directions (pick in the capacity write-up; not yet decided)

1. **Cap the walk, don't let it run to the service bound.** A walk that has scanned
   N rows without filling its page returns what it has with a "partial/again" cursor,
   or a fast 422 "filter too sparse to walk — narrow it". Turns 30s into milliseconds.
2. **A composite/covering index** for the common `status`+`country` shape so it seeks
   instead of walking. Bounded: `status=open` is `current_deadline > now` on the head,
   already a column (`tenders_current_deadline`); country is the version-place EXISTS.
   A denormalized `country` on `tenders` (like `current_deadline`) would collapse both
   to an index range. Costs a projection column + backfill.
3. **Per-shape admission** (the campaign's larger conclusion): price walk slots and
   shed early with 429, rather than letting each hold a slot to the 30s bound.

Acceptance: `?status=open&country=<any valid low-volume country>` returns in well
under a second (a real result or an honest "narrow your filter"), and four of them
in parallel do not brown out unrelated walk traffic.
