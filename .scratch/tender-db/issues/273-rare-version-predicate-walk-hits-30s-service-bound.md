# 273 — a rare-but-nonzero version-predicate combo walks to the 30s bound → 503 (cheap DoS)

Status: RESOLVED pending final deploy+measure — step 2 (sparse-side country seed) landed 2026-08-24 late; prod-validated 0.05–0.11s for the worst shapes
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

## Step 1 landed (2026-08-24, evening firing)

`version_predicates` gained `deadline_col: Option<&str>`; the two Tenders
builders pass `Some("t.current_deadline")`, both Lots builders pass `None`.
The fixture risk cleared: the Tenders status fixtures (api.rs) run the real
projection, so `current_deadline` is populated; the store-level status
fixtures are all Lots and keep the EXISTS. Pin test `status_head_range.rs`
asserts the emitted shapes (range for Tenders, EXISTS for Lots) and prints
the production SQL.

Prod re-validation of the FULL emitted FROM/WHERE via /v1/sql:

| shape                      | before          | after (cold) | after (warm) |
|----------------------------|-----------------|--------------|--------------|
| status=open&country=LU     | 30.00s → 503, 0 rows | 2.69s, 25 rows | 0.14–0.21s |
| status=open&country=CY     | 30.00s → 503, 0 rows | 0.85s, 25 rows | — |
| status=open&country=DE     | 0.25s           | 0.02s        | — |

The killer combos now FILL their pages — the old walk 503'd before reaching
the matches. Acceptance ("well under a second or an honest answer") met warm;
cold worst-case 2.7s is 10× under the service bound.

Residual (step 2): the no-status sparse combinations (`cpv+min_value` etc.)
still walk; re-measure once this ships and decide whether the general
candidate-window refactor is still warranted.

## Step 1 live measurement (2026-08-24, post-deploy 75f3e40)

Live endpoint, warm: `status=open&country=LU` **3.6s / 200 with rows** (was
30s → 503 with zero rows), CY 5.8s, DE 0.8s. Immediately after restart the
cold first hits still reached the 30s bound once — cold page cache, not the
old walk (the same query warm is seconds). The DoS as filed is defanged: a
walk-pool slot is now held for seconds, not 30s+overrun.

Residual for step 2: the published_at-ordered default plan evaluates the
satellite SELECT list into the sorter for every WHERE-passing row (slim probe
0.53s vs full 2.9–3.6s), so the sub-second acceptance bar needs the
candidate-window shape (ids-only inner query, satellites outside). Re-measure
CY/LU after that lands; sort=deadline pages already ride the index directly.

## Step 1b (2026-08-24, late firing): satellites out of the sorter

The 3s residual decomposed exactly as predicted: the ordered list's satellite
SELECT list was materialised into the sorter for every WHERE-passing row.
`tenders_ordered_query` now builds an ids-only inner window (id, seq, key —
three integers per sorter row) and joins `tender_select_head` back onto the
LIMITed page by primary key, so WHAT a row is still comes from the one shared
string and only WHICH rows changed hands. Cursor contract unchanged.

Validated with the exact emitted statement on prod before landing:
`status=open&country=LU`, published order, 100 rows — **0.35–0.49s** (was
2.9–3.6s wrapped-less, 30s→503 pre-273). Pin + all 45 api fixtures pass
unchanged.

Remaining (original step 2 scope, only if re-measurement demands): the
id-ordered `tenders_query`/lots shapes still evaluate satellites inline, and a
sparse no-status combination there still walks; those pages are PK-ordered
(no sorter), so the satellite cost only bites rows that pass — re-measure
before building anything.

## Step 1b deployed + the residual, sharpened (2026-08-24 ~21:20 UTC, rev 350f98b)

Live after deploy: LU `sort=published_at` 1.4s (was 3.6s), DE default 0.6s;
worst measured shape CY ordered **12.6s live / 7.3s as the bare statement**
(and the no-sort default path, which rides the id-ordered `tenders_query`,
is unwrapped: LU 3.7s). No 503s anywhere, no 30s slot-pinning.

The CY number isolates the true residual: CY's open∩country matches are **83
— UNDER the page limit** — so the window cannot stop early and exhausts the
entire open head (~36k candidates), paying the country-EXISTS for every one.
Dense-or-abundant filters exit at the limit (LU: 100 rows, 0.35s); sparse
ones pay the whole range. Wrapping cannot fix this class.

Step 2 design candidates, in preference order:
1. **Drive from the sparse side**: `tender_version_classifications (scheme,
   code)` holds only thousands of rows for a sparse country — seed tender ids
   from there, then check deadline-range + head-seq per id. Needs a
   cheap cardinality probe (or the 117 guard's index) to pick the drive side.
2. Scan budget + partial-page cursor (the original step-2 shape) — honest
   sub-second at the cost of the client contract.
Also: wrap the id-ordered `tenders_query` the same way as 1b (its no-sort
default is what most clients hit; LU 3.7s there vs 1.4s ordered).

## Step 1c deployed (2026-08-24 ~22:05 UTC, rev 0cb9d95)

The id-ordered page (the no-sort default and SSE snapshot shape) wraps like
1b. Live default-path after deploy: LU 1.1–1.2s (was 3.7s), CY 1.8–1.9s (was
5.7s), DE 0.7–0.9s. Day's arc for `status=open&country=LU`: 30.0s→503 with
zero rows → 1.1s with a full page.

Still open for the strict sub-second bar: the under-limit sparse case pays
the open-head exhaustion (~36k candidates × country-EXISTS). The
drive-from-the-sparse-side design (seed ids from `tender_version_
classifications (scheme, code)`, thousands of rows for a sparse country,
then deadline+head checks per id) is the next unit, with the drive-side
choice needing a cheap cardinality probe. The ordered (`sort=published_at`)
sparse case (CY 12.6s live) gains the most from it.

## Step 2 landed (2026-08-24, night firing): the sparse-side country seed

`tender_from` gained a fourth seed arm (precedence after publication and
participation): with `Filter.country_seed` set, the read drives from
`(SELECT DISTINCT tender_id FROM tender_version_classifications WHERE scheme
= 'nuts' AND code >= ? AND code < ?)` — the issue-223 pattern verbatim. The
seed is a superset (any version matched); the untouched head-version EXISTS
still decides membership, pinned by a fixture where a tender's old version
was CY but its head moved away (it must not leak — and does not).

The flag is set only by the async entries (`tenders`, `tenders_ordered`) via
`country_seed_viable`: a COUNT capped at 60k over the (scheme, code) index —
~10 ms warm; sparse prefixes (CY 52,525 rows / 13,735 tenders; LU 18,694;
MT 13,032 tenders) come in under the cap, DE saturates it and keeps the
range shape, which is already the right drive side there. Isolation routing
untouched — a seeded read still runs isolated, it is just fast there.

Prod-validated before landing: seeded CY, both orders, 83 rows in
**0.05–0.11s** (was 1.8s default / 7.3–12.6s ordered). Deploy + live
re-measure close the issue.
