# 424 — a lot is returned as `open` on a SIBLING lot's deadline, and its row shows no deadline or a past one

Status: ready-for-agent — **BUILT 2026-09-26 16:1x UTC** (see the foot): decision taken as leaned — the lots
`status` EXISTS is per lot, by the row's own rule; mutation-checked tests on both query shapes; gate,
deploy and the before/after latency read follow. Was: filed 2026-09-26 15:1x UTC from issue 423's
post-deploy paging read.
Kind: public API contract (the docs promise the opposite of what a row shows)
Relates to: 389 (fixed the PROCEDURE-scoped half: a lot inherits the procedure's deadline), 275 (pins the
lots `status` EXISTS with no `lot_id` term, on 273's tender-level equivalence), 370 (the
`submission_deadline_scope` marker), 422/423 (the same predicate, most recently touched)

## What

`/docs#caveats` → Dates: "a lot returned as open carries the deadline that opened it". It does not when
the deadline that opened it belongs to a DIFFERENT lot of the same tender:

    /v1/lots?status=open&cpv=45  (2026-09-26 15:1x UTC, three pages of 100)
      lot 219239 (LOT-0001)   submission_deadline: null                       -> returned as OPEN
        its sibling 13415907 (LOT-0000) has a lot-scoped 2026-09-28T10:00 deadline
      lots 187173 / 187174    submission_deadline: 2026-05-29 (scope: lot)    -> returned as OPEN

3 of the 300 rows read. The row is right by 389's rule (the lot's own deadline if it published one,
otherwise the procedure's); the FILTER is what is wide: `version_predicates`' lots arm is
`EXISTS (… tender_version_dates d WHERE d.tender_id = {tid} AND d.seq = {seq} AND d.field =
'submission_deadline' AND d.utc_seconds > ? …)` — any row of the version, any lot.

## The decision this needs

Make the lots `status` EXISTS per LOT, matching the row: open iff the lot's EFFECTIVE deadline is in the
future — its own lot-scoped deadline if it published any, otherwise the procedure-scoped one:

    EXISTS (own lot row, > now, within the horizon)
    OR (NOT EXISTS (own lot row) AND EXISTS (procedure row, > now, within the horizon))

That is a SUBSET of today's per-tender answer, so the open-head seed (`t.current_deadline > now`) stays a
valid candidate superset and 423's speed is untouched; 275's pin moves from "no lot_id term" to "the
row's rule". Default leaning: do it — the docs sentence is the contract, and the row is already right.
Cost to measure: the `status=open` lots shapes in 423's table, before/after.

## Verify

    curl -s 'https://tenders.zebreus.click/v1/lots?status=open&tender=81134&limit=50' | grep -o '"id":[0-9]*' | head

- **done**: lot 219239 is absent (its own row has no deadline and it published none, and the procedure
  published none) — only 13415907 is listed
- **open**: both 219239 and 13415907 are listed (read 2026-09-26)

A public read, free. (Tender 81134 holds both lots.)

## BUILT 2026-09-26 — `status` on a lot reads the date the lot row serves

- **`version_predicates` takes a `StatusBy`** instead of `deadline_col: Option<&str>`: `Head(col)` for the
  four tenders callers (unchanged range predicate on `t.current_deadline`), `Lot(lot_id_expr)` for the two
  lots callers (the tender-containment read and the stream, both `l.id`). The `None`-means-lots convention
  is gone, so a lots caller cannot forget to name the lot.
- **The lots EXISTS** is now: a deadline row of the version, `> now`, inside the horizon, AND (`d.lot_id =
  l.id` OR (`d.lot_id IS NULL` AND the lot has no ADMITTED own deadline — floor and horizon, any date)).
  That is `summarise`'s rule exactly: own admitted date if any, else the procedure's. A lot's own
  pre-1990 typo is not "its own" (171), so the lot falls back to the procedure's, as the row does.
- **Subset of the old per-tender answer**, so the open-head seed (issues 275/423) stays a candidate
  superset and needs no change.
- **Tests** (`lot_deadline_scope.rs::a_lot_is_open_by_its_own_deadline_never_a_siblings`), both query
  shapes (`served` = containment, `streamed` = stream): tender 81134's shape (a sibling's future date opens
  nothing; the undated lot is closed), 187173's shape (a lot's own PAST date beside a later procedure date
  → closed; its undated sibling inherits and is open), and a refused own date (falls back to the
  procedure's). **Mutation-checked**: collapsing the lot term to `1 = 1` (the old per-tender form) fails the
  test. The existing 389/422 pair tests pass unchanged.
- `/docs#caveats` Dates: `status` reads the same date as the row; no lot is opened by a sibling's.

**Baseline before deploy** (2026-09-26 16:0x UTC, rev `69f2a0e`, two reads each): `status=open&cpv=45&limit=100`
1.20 / 1.14 s, `status=open&limit=100` 1.69 / 1.67 s, `status=open&country=DE&limit=100` 0.96 / 0.93 s,
`status=closed&limit=100` 0.57 / 0.55 s, `status=closed&cpv=45&limit=100` 0.61 / 0.69 s,
`status=open&tender=81134` 0.45 / 0.41 s.

**Deployed `83187a5` 16:03 UTC** (gate 130/130, health 200, 0 error lines). Verify reads **done**: tender
81134's `status=open` lots list only 13415907. After-deploy latency (two reads each): open+cpv45 1.23 /
1.08 s, open+DE 1.02 / 0.93 s, closed 0.55 / 0.55 s, closed+cpv45 0.64 / 0.63 s, by tender 0.46 / 0.40 s —
all at baseline — but **bare `status=open&limit=100` rose 1.69 → 2.45 s**: the dense unseeded walk now pays
the per-lot term on every lot it crosses.

**Follow-up, same issue: seed bare `status=open` from the open head too.** Measured through `/v1/sql`
(one band, 101 rows, the new per-lot predicate, alternating twice): unseeded 3.56 / 3.53 s incl. ssh,
open-head seeded (39,871 tenders) 2.21 / 1.92 s — the same rows 322..33,222, ~1.5 s faster. The per-lot
term changed the arithmetic 423 recorded ("a seed would enumerate every open tender's lots, no better for
a value this dense"): it is better now. The arm's condition drops its prefix requirement; the tests that
pinned bare status as unseeded now pin it as seeded (`lots_open_head_seed.rs`, `lots_country_seed.rs`).
The first gate for this follow-up went **red** (101): `lots_filter_fixture.rs` inserts `tenders` rows without
`current_deadline`, so the open-head seed saw no open tender and `status=Open` selected 0 of 240 lots. The
fixture now stamps the column through its real writer (`backfill_current_deadline`, which transcribes the
election) — prod's fold writes it on every head, and `/v1/tenders?status=` has depended on it since 273.
Second gate green, 130/130.
