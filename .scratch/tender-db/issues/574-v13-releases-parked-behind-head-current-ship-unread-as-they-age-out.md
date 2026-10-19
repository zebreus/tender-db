# 574 — rule v13's consortium/family releases parked behind `head_current` ship unread as their heads age out

Status: DONE 2026-10-19 — unit 1 read all 81 v13-released rows parked behind a time hold (wf_7d4abf7a-699: 9 dated judges + skeptics, `.scratch/tender-db/574-triage/read_result.json`): 78 R, 1 N, 2 W, 0 reverts. The 58 behind head_current (the hold that ages out) read 57 R / 1 N (22857451: UTE name → its members' names on the UTE's own NIF, neutral) / 0 W, so they may ship as they lapse. Both wrongs sit behind top_stale, which tightens with time (it holds while the elected spelling's newest print is over a year old) and releases only if that spelling is printed again: 6769473 (Capita Group Plc → Care and Custody (Health) Ltd on Capita's VAT group number 618184140, gb_family H=1) and 17915448 (ECO CENTRO SARDEGNA SRL → an RTI consortium sentence, family H=1). Unit 2 decided: accept, no upkeep guard; the two wrongs are recorded here and stay held by top_stale. Was: ready-for-agent — filed 2026-10-19 from issue 548 U2's adversarial check and U3's GO read. Next: unit 1 — list the v13
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


## Unit 1 — read (2026-10-19, wf_7d4abf7a-699)

- Population: plan 2653 `held` entries reading `head_current` or `top_stale` whose plan-2569 hold was a v13-released bind guard: 81
  (head_current: consortium 50, family 7, gb_family 1; top_stale: consortium 18, family 4, gb_family 1). `574-triage/rows.json`.
- Read with dates by 9 judges (9 rows each), a skeptic on every W/REVERT: **78 R, 1 N, 2 W, 0 REVERT**.
- head_current (58): 57 R, 1 N (22857451, `UTE ZENER SOFT CONSULTING SL, TECNICOS ASOCIADOS INFORMATICA SAU` → the members' names
  without `UTE`, on the UTE's own U-NIF; same entity, neither better). These ship as their heads age past 1,095 days (dates per
  row in `read_*.md`); nothing to guard.
- top_stale (23): 21 R, 2 W, both kept by the skeptic:
  - 6769473: `Capita Group Plc` → `Care and Custody (Health) Ltd` on GB VAT 618184140, which two Capita companies print in 2015
    (a VAT group number); Care and Custody already has its own org 11504910. Member for the group, or another entity.
  - 17915448: `ECO CENTRO SARDEGNA SRL` → `RTI COSTITUENDO MASNATA CHIMICI S.P.A. + CHIMICA SARDA S.R.L. (DATI MANDATARIA)`, a
    consortium sentence from one 2022 procedure on a single company's P.IVA.
  Both elected spellings were last printed in 2018 and 2022, so `top_stale` holds them and keeps holding them; a fresh print of
  the elected spelling is the only release, and would itself be evidence for it. Both are H = 1 rows the v13 family/gb_family
  release freed: the release's wrong rate on its parked rows is 2 of 81 (≈ 2.5%), all held.
- **Decision (unit 2):** accept. The weekly upkeep (524 unit 2c) needs no extra guard for v13 releases.
