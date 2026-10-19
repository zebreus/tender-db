# 574 — rule v13's consortium/family releases parked behind `head_current` ship unread as their heads age out

Status: ready-for-agent — filed 2026-10-19 from issue 548 U2's adversarial check and U3's GO read. Next: unit 1 — list the v13
plan's released rows now held `head_current` (consortium 50, family 7, gb_family 1) with the date each ages out, and read them
(all if ≤ 60) before the first one does; then decide whether the weekly upkeep needs a guard.
Kind: data quality (served organization names; rollout safety)
Relates to:
- 548 (U2's caveat: "34 of the 56 subset rows that land in head_current age out of it within 365 days and would then ship
  unread"; U3's plan 2653 confirmed it: 6 consortium rows moved from head_current to change:R1 in the 3 days since plan 2569);
- 524 (unit 2c's weekly upkeep, which ships whatever a fresh plan elects).

## Verify

    python3 -c "import json,gzip;b=json.load(gzip.open('.scratch/tender-db/548-triage/org-name-refresh-plan-2026-10-19-v13.json.gz'));b=b.get('body',b);b=json.loads(b) if isinstance(b,str) else b;print(sum(1 for e in b['held'] if e['reason']=='head_current'))"

- **done**: every released row behind `head_current` has a recorded dated verdict (in this issue's triage dir) before it ages
  out, and wrongs (if any) are guarded or recorded as accepted.
- **open (2026-10-19)**: prints 6390 (all head_current holds); the released subset (~58: 50 consortium + 7 family + 1 gb_family
  from plan 2569's bind-guard holds) is unread.

## What was found

v13 (issue 548 U2/U3) releases three bind guards. Released rows that still meet a later hold keep that hold: 50 consortium
subset rows, 7 family H ≤ 1 rows and 1 gb_family row now read `hold:head_current` on plan 2653, because their head was printed
within 1,095 days of `as_of`. `head_current` is a time-based hold, so each will ship as a plain `change:R1` on the first plan
after its head's newest item turns 1,095 days old — unread: U1/U1c recorded verdicts for the shippable rows only, and step 3's
v13 re-judge (wf_931e89e7-17a, 51/51 R) read only those already shipping. The consortium release is conservative (salted 30:
29R/1N/0W) and family H ≤ 1 read 22/22 R, so the expected wrong rate is low; the point is that the runbook's "dated read before
a wet run" is skipped for rows that cross the line between plans.

## Proposed units

1. **List and read (S).** From plan 2653, the released rows held `head_current` with `head_newest + 1,095 days`; read all (≤ 60)
   with dates, a skeptic on every wrong; record in `.scratch/tender-db/574-triage/`.
2. **Decide (S).** Accept, or have the weekly upkeep (524 unit 2c) hold a v13-released row until it carries a verdict.
