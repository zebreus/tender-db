# 506 — result-level value fields (BT-709, BT-1118, BT-660) are not stored as amounts

Status: needs-triage — filed 2026-10-09 from issue 505's adjudication (`.scratch/tender-db/505-lot-over-procedure/`).
Read 2026-10-09:
- The fold's `AMOUNTS` table (`project.rs`) maps only BT-27, BT-271 and BT-161 among the eForms money BTs, plus the
  legacy and DÖE spellings. BT-709, BT-660, BT-118/BT-1118 and BT-156/157 reach the notice layer (the all-BT claim)
  and go no further.
- Mapping any of them adds head and partner candidates to every eForms award notice. So unit 1 needs a per-field
  census of `notice_amounts`. That table has no `field_id` index, so the census is a windowed job (the
  `procedure-key-census` shape), not a bounded `/v1/sql` read.
Kind: data model (eForms result-level money that the fold drops)
Relates to: 505 (the residual rule), 492 / 471 (the partner rules, which can only see figures stored in `tender_version_amounts`
or lot awards)

## What was seen

Two adjudicated slips have their true value in a result-level field that the fold stores nowhere as an amount:
- **627219.** The lot figure is exactly 100× BT-1118 / BT-660. Neither is stored, so no partner exists and 492's ×100 rule
  cannot see the slip.
- **395737.** The lot figure is exactly 100× BT-709-LotResult (the framework maximum on the lot result). It is not stored
  either.

If these figures were amounts, both slips would have an exact ×100 partner and the existing rule would refuse them.

## Units

1. **Measure.** For each eForms SDK, which result-level money BTs the parser reads, which reach the notice layer, and
   which the fold maps to `tender_version_amounts` (or `lot_results.awarded`). Start from the AMOUNTS table in `project.rs`
   and from the 505 adjudication rows.
2. **Decide** per field: map it into an amount field (which one), or leave it with a reason. Mind 492's decision (c):
   a lot-null amount equal to a lot award counts as a lot figure. A new result-level field must not change which
   figures are lot figures without a measurement.
3. Build, refold the affected profiles, and re-read 395737 and 627219.
