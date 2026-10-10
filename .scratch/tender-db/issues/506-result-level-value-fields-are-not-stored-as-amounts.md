# 506 — result-level value fields (BT-709, BT-1118, BT-660) are not stored as amounts

Status: ready-for-agent — TRIAGED 2026-10-10: unit 1's code side is done (`../506-result-amounts/unit1-code-census.md`, a full parser → fold → elections walk with file:line cites, owner-spot-checked). The decisive fact: a LotResult has no Lot ancestor, so any LotResult BT mapped through plain `AMOUNTS` lands LOT-NULL and, by 492 decision (c), poisons `lot_only` / `lot_keys` for every genuine framework. BT-709 and BT-660 must go to the result's lot through BT-13713 (the award-role `lot_of` precedent, `project.rs:4660-4675`); BT-118 / BT-1118 are notice totals and lot-null is right for them, but each adds a Procedure head candidate on every framework award notice (BT-161 is forbidden there). Leave BT-710/711 (a losing bid; BT-711 would become heads). CENSUS READ 2026-10-10 (sampled, below). NEXT: unit 2's decision — per field, destination and scope, taken through an adversarial design panel against the census and the code walk; it must say what a BT-709/BT-660 lot figure does on the ~17 % of carrying notices with no mapped figure at all (it becomes their only head/lot candidate).
Was: needs-triage — filed 2026-10-09 from issue 505's adjudication (`.scratch/tender-db/505-lot-over-procedure/`).
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

## Unit 1 census (2026-10-10, sampled)

`../506-result-amounts/census.py` (bounded: per profile, 40 start ids through the `notices_profile` index, the
next 150 notices each, their `notice_amounts` and BT-13713 refs by primary key; paced under the `/v1/sql` rate
limit). 38,057 eForms notices over SDK 1.3–1.14 and eForms-DE 1.1/1.2; raw output
`../506-result-amounts/census-2026-10-10.json`. **Caveat: ids cluster, so the sample is lumpy** — sdk-1.6
contributes 11,975 of the 15,323 BT-709 rows from a few large multi-lot notices. Read the shape, not the totals.

"head proxy" = the notice's largest mapped figure (BT-27*/271*/161) in the same currency; "pow10" = the value is
10^k (k = 2…6) times a mapped figure or one 10^k-th of it.

| field | rows | = a BT-720 tender value | = its own lot's BT-271/27 | = a procedure figure | equals nothing | notice has no mapped figure | exceeds head proxy | pow10 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| BT-709-LotResult | 15,323 | 14,109 | 497 | 342 | 772 | 2,671 | 234 | 11 |
| BT-660-LotResult | 14,981 | 2,825 | 845 | 406 | 11,254 | 2,261 | 138 | 16 |
| BT-118-NoticeResult | 879 | 264 | — | 344 | 365 | 238 | 97 | 0 |
| BT-1118-NoticeResult | 406 | 47 | — | 225 | 152 | 37 | 37 | 1 |
| BT-710-LotResult | 18,108 | 16,508 | 171 | 788 | 1,521 | 2,748 | 104 | 19 |
| BT-711-LotResult | 18,001 | 11,165 | 181 | 546 | 6,759 | 2,748 | 311 | 16 |
| BT-156 / BT-157 | 1 / 4 | | | | | | | |

Read:
- **BT-709 is mostly the winning tender value**, not the lot's BT-271 (92 % equal a BT-720; 3 % equal their own
  lot's BT-271/27). At the result's lot it would mostly restate the award — the decision (c) shape, harmless — but on
  the notices with no mapped figure (17 % of carrying rows) it becomes the only head/lot candidate.
- **BT-660 is mostly a new number** (75 % equal nothing mapped) — the re-estimate genuinely differs from the
  estimate and the award.
- **BT-118 / BT-1118 raise the head proxy on ~10 %** of the notices carrying them (23670345: BT-118 €12.68 m against
  a €0.60 m largest mapped figure — the notice's framework total over one lot's estimate).
- **New 10^k partners are rare** (≤ 0.1 % of rows): mapping does not flood the scale rules.
- **eForms-DE 1.x carries all four** under `DE1-NoticeResult-…` spellings (`LotResult-FrameworkAgreementValues-
  MaximumValueAmount` / `-ReestimatedValueAmount`, `OverallMaximumFrameworkContractsAmount`,
  `OverallApproximateFrameworkContractsAmount`), plus `DE1-…RequestedTenderTotal-FrameworkMaximumAmount` (287 + 423
  rows in the sample), which is already the BT-271-Procedure alias (`project.rs:1151`); a bare `DE1-FrameworkMaximumAmount` (4 rows, de-1.1) is the root-level notice framework maximum (issue 195, `eforms/index.rs:967-986`): the EU minors gap-fill it as `UBL-FrameworkMaximumAmount` → `framework_maximum`, but eForms-DE 1.x keeps its DE1 id and the alias table (`project.rs:1146-1156`) has no entry, so it is DROPPED there — alias it to `BT-271-Procedure` in unit 2's build (~100 notices by the sample's rate).
- BT-710 / BT-711 stay unmapped (losing-bid figures; BT-711 exceeds the proxy on 311 rows).
