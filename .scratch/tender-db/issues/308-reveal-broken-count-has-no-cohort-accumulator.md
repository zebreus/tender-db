# 308 — D5's BROKEN reveal count has no cohort-wide accumulator or gauge

Status: open (filed 2026-08-28 during the hourly audit slot)
Kind: observability gap (small)
Relates to: 274 (the sliced reveal recheck; its residue declared this number
"the campaign's acceptance metric, accrues nightly"), ADR-0013 D5.

## What

The nightly reveal-recheck reports `later_still_withholds` (BROKEN — a later
version exists past the promised availability date and STILL withholds the
BT-198 field) **per slice only**, in the job counts line and the
/admin/reports passthrough. The cohort walk wraps every ~3 nights, so the
cohort-wide BROKEN total exists nowhere: you must read 3 consecutive job
lines and add them by hand. /metrics carries job plumbing for the kind
(last_ok, duration, report timestamp) but no data gauge — a sudden jump in
BROKEN (an upstream regression in TED's reveal pipeline, or a fault in our
version chaining) would be invisible to anything that watches.

Measured while auditing (stable so far): same slice 25535052..26788048 was
371 BROKEN on 2026-08-25 and 374 on 2026-08-28 (+3 due, +3 broken — pure
accrual as due dates pass); slice 0..25535052 reported 1,133 on 2026-08-27.
Cohort ≈ 279,483 withheld fields, of which the due-and-broken tail is
~1,500-1,600 per wrap so far.

## Fix sketch

The slice report already carries the cursor and a `wrapped` flag. Keep
per-wrap running totals (due/revealed/awaiting/broken) in the persisted
report state, roll them into cohort totals when a slice wraps, and export
`tender_db_dq_reveal_broken_total` (+ due/revealed siblings) as gauges from
the last COMPLETED wrap — never a partial sum, so the gauge is comparable
day to day. Dashboard QualityPanel can then show it next to the withheld
total. Small unit; rides any deploy.

## Acceptance

- gauge appears after the first completed wrap post-deploy and matches the
  hand-summed three slice lines;
- a partial wrap never moves the gauge;
- 274's residue note updated to point here.
