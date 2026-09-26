# 424 — a lot is returned as `open` on a SIBLING lot's deadline, and its row shows no deadline or a past one

Status: ready-for-agent — filed 2026-09-26 15:1x UTC from issue 423's post-deploy paging read. Measured on
prod; the mechanism is in the code; the decision (below) is mine to take when this is picked up.
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
