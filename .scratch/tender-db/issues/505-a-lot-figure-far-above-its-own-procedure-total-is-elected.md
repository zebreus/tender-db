# 505 — a lot figure far above its own procedure total is elected

Status: ready-for-agent — UNIT 1 MEASURED 2026-10-09 (`505-lot-over-procedure/heads-2026-10-09.json`); adjudication
`wf_368978c8-198` running over the 23 Tenders at ≥ 10×.
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
