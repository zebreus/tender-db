# 505 — a lot figure far above its own procedure total is elected

Status: ready-for-agent — UNIT 3 BUILT 2026-10-09 (the residual rule, `ScalePartners::residual_slip`). NEXT: gate,
review, deploy, drain (`refold-value-band`), re-read the 5 expected Tenders.
- **Unit 2, decided** from adjudication `wf_368978c8-198` (two readers per Tender, who agreed on every refuse/keep
  call; `505-lot-over-procedure/adjudication-*`). Of the 23 Tenders at ≥ 10×:
  - Verdicts: 16 LOT_SLIP and 2 BOTH_JUNK (refusing the lot is right), 3 PROCEDURE_JUNK and 2 LOT_GENUINE (refusing
    it is wrong).
  - Candidate rules:
    - (a) a plain ≥ 10× ratio: 5 wrong refusals in 23 (22%).
    - (c) an exact power of ten: 1 wrong in 3.
    - Ratio bands: the same welded genuine Tender (1155337) moves between bands.
  - Only the **exact sum residual** has no false positive: the version has ≥ 2 Lots, and the lot figure equals
    (procedure figure − the other Lots' figures) × 10ᵏ with k ≥ 2, in the same field and currency. It fires on 4 of
    the 23 (8784848 ×100; 553044, 5748163 and 1120720 ×1000).
  - Over every version holding a stored lot value ≥ €1 bn (1,815 versions, 856 Tenders), it fires on 5 Tenders:
    those 4 plus 915781 (a PLN 7.96 bn medicines lot under a PLN 29.6 m procedure, ×1000), all slips.
  - Decision: build it, behind the €1 bn gate, for the stored lot value and the head's lot candidates.
  - The other 14 slips stay as published. Follow-ups, none a ratio rule:
    - Store BT-709/BT-1118/BT-660 as amounts; that gives 395737 and 627219 an exact ×100 partner.
    - Elect a PIN Part as head only in a version with no Lot (8818621).
    - 1155337 is a weld, not a value problem.
    - 578884 and 8716838 still serve a wrong procedure figure after a refusal.
- Unit 1 MEASURED 2026-10-09 (`505-lot-over-procedure/heads-2026-10-09.json`).
- Of 3,736 heads at ≥ €1 bn (after 492's drain), 3,621 are elected from a procedure figure and 115 from a lot figure only.
  - 72 of the 115 have no smaller procedure figure in the head version.
  - 43 have one. By the ratio head / procedure figure: under 2×, 10; 2–10×, 10; 10–50×, 6; 50–100×, 1 (8784848);
    100–1000×, 9; ≥ 1000×, 7.
- So the population a rule could touch is small: 23 Tenders at ≥ 10×.
Was: ready-for-agent — filed 2026-10-09 from issue 492's drain re-read.
Kind: correctness / the head and lot value elections (`canonical.rs`, `ScalePartners`, `head_value_eur_cents_with`,
`elect_lot_value`)
Relates to: 492 (the ×100 rule; its unit 2 named this signal as an option and did not take it), 471 (k ≥ 3; "a plain ratio
is not a signal"), 366 (the €100 bn ceiling)

## What was seen

8784848 (Essex County Council care-homes framework, FTS 004793-2022) serves a head of **€72 bn**. That is lot 1's
£60 bn estimate. The procedure estimate is £1 bn. The other lots are £150 m and £250 m, so with lot 1 at £600 m the
three lots sum exactly to the procedure total. The buyer's later IRN notices state £600 m and £1.57 bn. Lot 1 is a
×100 slip of £600 m.

Neither scale rule reaches it:
- The exact-10ᵏ rule (471, k ≥ 3) and the ×100 rule (492, k = 2) both need a partner figure in the chain. £600 m is
  not a figure of the chain; it appears only in the IRN notices, which are not versions.
- The €100 bn ceiling (366) is not reached: £60 bn is about €72 bn.

The signal the Tender does carry is structural: one lot's figure is 60× its own version's procedure total, and the other
lots sum to the procedure total minus 1/100 of it.

## Why it is not simply "refuse a lot above its procedure total"

- 471's band adjudication found that a plain ratio is not a signal. Real frameworks publish a procedure figure below
  a lot ceiling (a per-year figure beside a whole-term lot maximum, or a stale procedure estimate).
- No sample measures how often a lot sits above its own procedure figure among genuine Tenders.

## Units

1. **Measure.** List the heads of €1 bn or more whose elected figure is a lot figure, together with the same version's
   procedure figure in that currency and the ratio between them. Use a bounded read off `tenders_current_value_eur` plus
   per-Tender seeks, as in 492's unit 1. Bucket by ratio, and adjudicate a sample of the high-ratio buckets.
2. **Decide.** Options: a refusal when the lot exceeds the procedure total by ≥ 10× AND the remaining lots sum to
   the procedure total (8784848's shape); a refusal at a measured ratio; or caveat only.
3. Build, gate and drain (`refold-value-band`), if step 2 says so.
