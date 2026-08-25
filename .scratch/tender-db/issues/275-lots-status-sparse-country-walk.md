# 275 — lots: status + sparse country still walks to the 30s shed (273's shape, on the endpoint 273 didn't fix)

Status: RESOLVED — deployed `0fe1d64` and VERIFIED on prod (2026-08-25 ~23:59
UTC): `status=open&country=LU` 30.7s/503 → **1.10s cold / 0.93s warm** with
correct rows (LU lots, future deadlines); `source=<absent>` 33.4s/503 →
**0.46s**; `country=CY` 1.0s warm. Health green on `0fe1d64`. Scope grew one finding during the audit: `?source=<absent>` on
lots is a SECOND live 503 (fixed round 1, verified 33.4s → 0.61s on prod).

Decline-path regression check (2026-08-25 22:50 UTC firing): the dense shapes
where the seed probe must DECLINE at the cap and keep the pre-275 plan are
unregressed — `country=DE` 0.68s, `status=open&country=DE` 1.41s,
`source=ted` 0.52s; the seeded sparse path holds (`status=open&country=CY`
1.13s). Verification complete on both sides of the cap.

## Round 2 — the first deploy did NOT fix the headline shape (kept honest by
## the acceptance probe)

Deployed `4ac4f4b` and `status=open&country=LU` was STILL 503/30.5s. Two causes,
both found by measuring exact statements in the on-box SQL sandbox:

1. **LU is over the seed cap.** `country_seed_viable` bails at 60,000
   classification entries; LU has 67,136. The flag never armed. On tenders the
   over-cap countries are saved by the `current_deadline` head-range (273 step
   1); lots have no head column, so an over-cap country got NOTHING.
2. **The JOIN seed form inverts on lots.** Even where the seed arms (CY),
   `hits JOIN lots l … ORDER BY l.id` makes turso drive from `lots` to serve
   the ORDER BY and probe `hits` per row. Measured, same rows each pair:
   CY seed as JOIN 2.1s vs as `l.tender_id IN (…)` **0.32s**; open-head seed
   as JOIN 7.8s vs as IN **0.48s**. (The tenders JOIN seed survives because it
   joins on the OUTER table's PK — `t.id = hits.tender_id` — which turso
   serves hits-first; `l.tender_id` is not the lots PK.)

Round-2 shape (`lot_seed_predicates`, both arms as IN semi-joins, mutually
exclusive, participation seed still outranks):
* viable sparse country → the case-variant UNION ALL enumeration, as
  `l.tender_id IN (SELECT DISTINCT tender_id FROM (…))` — 0.32s CY;
* over-cap country + `status=open` → `l.tender_id IN (SELECT t.id FROM
  tenders t WHERE t.current_deadline > ? AND EXISTS(country at
  t.current_seq))` — 273's proven status≡head-range equivalence bounds the
  candidates to ~38k open tenders and the tender-level country test keeps the
  lot explosion to matching tenders only — 0.48s LU.

The per-lot EXISTS/version predicates stay untouched and still decide
membership in both arms.
Kind: performance / availability (issue-61 class; unauthenticated, trivially reachable)
Relates to: 273 (the tenders fix this ports), 70 F3 (the audit that predicted the class), 223 (the participation-seed shape being reused)

## Measured

Live probes 2026-08-25 ~21:30–21:45 UTC, prod rev `e3a1f9a`, box otherwise
idle, one request at a time (the 70-F3 audit's predicted worst cases):

```
GET /v1/lots?status=open&country=LU&limit=20        → 503 in 30.7 s   ← DoS, fixed here (country seed)
GET /v1/lots?source=nonexistent-source&limit=100    → 503 in 33.4 s   ← DoS, fixed here (reachable leg)
GET /v1/lots?country=LU&limit=20                    → 200 in  1.9 s
GET /v1/lots?status=open&limit=100                  → 200 in  1.1 s   (predicted-high, did NOT reproduce)
GET /v1/tenders?source=nonexistent-source&limit=100 → 200 in  0.7 s   (tenders' in-row compare is cheap — why the guard is lots-only)
GET /v1/organizations?country=DE&kind=no-such-scheme → 200 in 2.0 s   (bounded; org combos are mediocre, not DoS)
GET /v1/organizations?kind=vat&country=AQ           → 200 in  1.7 s
GET /v1/notices?kind=no-such-profile                → 200 in  0.7 s   (reachable's kind leg covers notices, contra prediction)
GET /v1/notices?kind=eforms&cursor=9999999999       → 200 in  1.2 s
```

The `status=open&country=LU` shape is exactly what issue 273 killed on
`/v1/tenders` (30s → 1.1s there), alive on `/v1/lots`. Found while executing
issue 70 F3's "measure which filters actually scan at prod scale" step.

## Mechanism

`lots_query`'s stream shape drives from `lots l` (~10M rows, id order) with
per-row correlated predicates: the `MAX(seq)` head subquery, the
`tender_version_lots` EXISTS, and `version_predicates(..., deadline_col: None)`
— so `status` is the per-row submission-deadline EXISTS (lots have no
`current_deadline` head column; 273's own pin test records that split as
intended) and `country` is a per-row classifications EXISTS. A sparse country
means almost no row matches, so the walk tests candidates until the 30s bound
sheds it. The only seed wired into `lots_query` today is issue 223's
participation seed; 273 step 2's sparse-country seed was wired into
`tender_from` only.

## Fix (this issue)

Port 273 step 2 to the lots stream, reusing its machinery unchanged:

* `lots_identity` (both public entries route through it) runs
  `with_country_seed` after the `reachable()` guard, Page-scoped only — the
  At/containment paths ignore the flag, so probing there is pure cost on the
  SSE diff.
* `lot_from(filter)` mirrors `tender_from`: participation seed first, else the
  sparse-country `hits` set (UNION ALL case-variant ranges over
  `tender_version_classifications (scheme, code)` — never an OR'd WHERE, per
  273's live regression) joined `JOIN lots l ON l.tender_id = hits.tender_id`
  (served by `UNIQUE(tender_id, lot_key)`). The shared `hits` construction is
  factored into `country_seed_hits` so the two builders cannot drift.
* Membership still decided by the untouched EXISTS/version predicates — the
  seed is a candidate superset (a tender whose OLD version matched the prefix
  but whose head moved away must stay excluded; pinned by the behavioral test,
  the lots twin of `the_country_seed_stays_a_candidate_set_not_an_answer`).
* Publication numbers are a tenders-only filter; `tender_from`'s exact-seed arm
  has no lots mirror.

Second fix in the same change: a lots-only `source` leg in `reachable()` — a
bare `WHERE source = ? LIMIT 1` probe over `tenders` (~0.7s absent, instant
present) replacing the 33.4s correlated walk; present sources are dense
(ted/doe) so the guard is the whole fix for this class.

Tests: `crates/store/tests/lots_country_seed.rs` — statement pin (seeded FROM
with the flag, unseeded without) + the behavioral superset-trap twin of 273's
(head-moved-away tender seeded-in-filtered-out; lowercase `cy` equal to `CY`;
source guard both ways: absent → correct empty, present drops nothing).

Acceptance: the two 503 probes above sub-second on prod after deploy;
existing `lots_status_keeps_the_exists_form` and the lots equivalence oracle
stay green.

## Residuals (recorded, NOT fixed here — all isolated-pool, deadline-bound)

* `min_value`/`max_value`: per-row `MAX(cents)` correlated subquery, no
  reachable leg possible (any threshold is "present"); a high floor walks to
  the 30s shed on an isolated slot. Candidate fix if it ever matters: the
  F3 scan-cap idea. Predicted by the audit, not probed (nothing new to learn:
  the mechanism is the measured source shape with a different subquery).
* Sparse `kind` (Part/LotsGroup below page size): the recorded 132.1s latent
  case in `reachable()`'s own comment — unchanged.
* Org combo shapes (`country+kind` residual, `name_prefix`+companion): measured
  1.7–2.0s — mediocre, bounded, main-pool. Watch, don't build.
