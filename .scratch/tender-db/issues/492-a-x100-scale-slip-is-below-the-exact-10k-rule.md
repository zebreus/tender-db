# 492 — a ×100 scale slip (k = 2) is below the exact-10ᵏ rule

Status: ready-for-agent — filed 2026-10-08 from issue 471 unit 5. NEXT: unit 1, measure k = 2's
precision; extend nothing until that is done.
Kind: data quality / head election (`ScalePartners`, `crates/store/src/canonical.rs`)
Relates to: 471 (unit 4(a), which set `SCALE_ERROR_MIN_EXPONENT` = 3; unit 5's sample)

## What is wrong

Issue 471 unit 4(a) refuses a figure worth ≥ €1 bn when it is exactly 10ᵏ times a same-currency
partner of the same Tender, but only for k ≥ 3. Two of the 20 errors in unit 5's sample are ×100
slips, and the rule therefore keeps them:

- **6640498** (TED 337801-2020, €14 bn). Section 16 shows a ×100 partner. The adjudication found ×100,
  either a typo or a dropped decimal.
- **8784848** (Essex County Council care-homes framework, FTS 004793-2022, lot 1 at £60 bn). The
  procedure estimate is £1 bn and the other lots are £150 m and £250 m. With lot 1 at £600 m the three
  lots sum exactly to the procedure total. The buyer's later IRN notices are £600 m and £1.57 bn.

## Why k = 3 was the floor

Round hundreds are common between genuine figures of one Tender: a €1 m lot inside a €100 m framework,
or a €10 m estimate beside a €1 bn ceiling. Unit 4's adjudication measured k ≥ 3 only. The k = 2
false-positive rate was never measured, and the €1 bn gate alone does not bound it.

## Units

1. **Measure.** List the Tenders whose head (≥ €1 bn) has an exact ×100 same-currency partner, using
   the data-quality band machinery or a bounded read off `tenders_current_value_eur`. Adjudicate a
   sample.
2. **Decide.** Choose one: extend the rule to k = 2 (perhaps only when the partner is a sibling LOT or
   the sum check of 8784848 holds), add a narrower structural signal ("a lot estimate above its own
   procedure total"), or caveat-only.
3. Build, gate, drain and re-read section 16, if step 2 says so.
