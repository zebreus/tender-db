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

## Fix design VALIDATED at prod scale (2026-08-24) — and a rejected first attempt

Applying the D5 lesson (validate query shapes on the real corpus before building), I tested
two candidate-window shapes against prod via /v1/sql before writing any code:

1. **REJECTED — window that keeps a version-predicate inside**: `SELECT … FROM (SELECT t.id …
   WHERE <status EXISTS> ORDER BY current_deadline LIMIT 500) … WHERE <country EXISTS>` still
   **408s at 10s**. Bounding the RETURNED candidates does not bound how many are SCANNED:
   the inner status-EXISTS + correlated MAX(seq) runs per scanned row until 500 pass. Had I
   built this "obvious" fix and shipped it, it would have been a second non-fix.
2. **VALIDATED — window with ONLY served head-predicates + ORDER BY**: `SELECT … FROM (SELECT
   t.id FROM tenders WHERE current_deadline > ? ORDER BY current_deadline LIMIT 500) c WHERE
   <country EXISTS>` returns in **0.13s**. The window is a pure index range (bounded scan);
   ALL version-predicate EXISTS move OUTSIDE it.

### The implementation (ready-for-agent, its own focused session + prod re-validation)

- Restructure `tenders_ordered_query` (read.rs): inner window = served head predicates
  (`current_deadline`/`current_published_at` range, source, kind) + cursor + `ORDER BY key
  LIMIT scan_budget`; outer applies `version_predicates` EXISTS + `LIMIT page`.
- **Cursor contract change** (client-visible, handle carefully): a page now returns 0..page
  matches AND must resume from the last WINDOW candidate's (key,id), not the last match. Add
  a boundary signal so the client keeps paging until it has `page` rows or the window comes
  back short of `scan_budget` (= stream truly exhausted). This is the real work; the SQL is
  the easy half.
- **Bonus narrow win**: `status=open` ≡ `current_deadline > now` is a HEAD column. Serving it
  as a range predicate (not the current deadline-EXISTS) fixes the common default-view mid-class
  outright, and makes it a natural served-window predicate. Consider doing this first — it is
  smaller and covers the most common walk.
- Same treatment for the org-named walk and SSE snapshots (they share the walk pool).
- Re-validate the FULL production query (not just the principle) on the box before deploy,
  D5-style.

Not rushed into this firing's deploy on purpose: the D5 regression an hour earlier is the
argument for landing this deliberately with prod re-validation.

## Refinement (2026-08-24): the common cases need only a SMALL, provably-correct change

Digging into the projection contract changes the plan. `status` is currently a per-row
version-predicate EXISTS (read.rs:767, over `tender_version_dates`). But `status=open` is
**provably equivalent** to the head-column range `t.current_deadline > now`:
- `head_deadline()` (canonical.rs:933) writes `MAX(head submission_deadline)` into
  `current_deadline` in the projection's head UPDATE.
- `MAX(deadlines) > now` ⟺ `EXISTS(some deadline > now)` — MAX is the largest, so it exceeds
  `now` iff at least one does. Exactly the Open predicate. Closed ⟺
  `current_deadline IS NULL OR current_deadline <= now`.

So converting status to a served head-range predicate on `current_deadline` (indexed by
`tenders_current_deadline`) is safe AND bounds the scan for the whole combination: the killer
`status=open&country=LU` stops being "country-EXISTS over all 7.9M" and becomes "country-EXISTS
over the 36,600 `current_deadline > now` rows" — the 0.13s shape I validated. **This single
change fixes the most common 273 cases (anything with status) and the mid-class status=open
cost, with no cursor-contract change.**

### Revised plan, in order
1. **(small, do first)** In the Tenders query builders, when `status` is set, emit the
   `current_deadline` head-range predicate and DON'T pass status to `version_predicates`; drop
   status from `walks()`'s version-predicate set for Tenders so status-only runs on the main
   pool. Update the 112/117 plan-pin tests to the new shape. Re-validate on prod. This alone
   retires the common DoS.
2. **(larger, later)** The general candidate-window + scan-cursor refactor (above) for the
   residual no-status heavy case (`cpv=…&min_value=…&sort=deadline` with a sparse result).
   Only needed once (1) lands and re-measurement shows what's left.

Scoped for a focused session with prod re-validation; not rushed into an unattended deploy.

### Final de-risking (2026-08-24) — change site pinned, one risk to clear

- **Confirmed minimal:** the DoS fix needs ONLY the `version_predicates` SQL change, NOT a
  `walks()` change. With `country` present the read already isolates (correctly); the
  head-range just makes it fast. So `walks()` and the isolation_routing tests stay untouched
  (status still isolates for Tenders — fine, it's fast now). Defer the status-only
  de-isolation optimisation.
- **Change site:** add a `deadline_col: Option<&str>` param to `version_predicates`. Tenders
  builders pass `Some("t.current_deadline")` → emit `t.current_deadline > ?` (Open) /
  `(t.current_deadline IS NULL OR t.current_deadline <= ?)` (Closed); the three Lots sites pass
  `None` → keep the existing EXISTS (no head column on lots).
- **No SQL plan-pin test pins the status EXISTS string** — status is only exercised through
  functional fixtures that check RESULTS. `api.rs:925` already asserts `current_deadline`
  equals the served submission_deadline, corroborating the equivalence.
- **The ONE risk to clear first:** do the tender status fixtures populate `current_deadline`
  (via the real projection's head_deadline) or hand-insert `tender_version_dates` only? If the
  latter, the head-range reads them as not-open and those fixtures break — audit + fix the
  fixtures as part of the change. This is the first thing to check in the focused session.

Estimate: 1–2 h with prod re-validation. Not started in the tail of a firing after today's
D5 hang; the module is the read-path safety core and deserves fresh context.
