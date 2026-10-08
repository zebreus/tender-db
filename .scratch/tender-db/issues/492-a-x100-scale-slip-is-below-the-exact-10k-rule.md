# 492 — a ×100 scale slip (k = 2) is below the exact-10ᵏ rule

Status: ready-for-agent — filed 2026-10-08 from issue 471 unit 5. Band count done (5 of 306 rows at ×100; see
"Unit 1 — first count"). NEXT: the €1–10 bn range, as a windowed read AFTER issue 490's backfill project
(a direct partner probe 408s while the fold runs), then adjudicate.
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

## Unit 1 — first count (2026-10-08)

Read off section 16 of dq 2954 (`471-values/section16-dq2954-2026-10-08.txt`). Its "x smallest" column
gives the head figure's ratio to the Tender's smallest figure. **5 of the 306 band rows sit at exactly
×100:**

- 8595426 (FTS, £38.3 bn)
- 4871119 (TED, £25 bn)
- 8618327 (FTS, £10 bn)
- 8810872 (FTS, £10 bn)
- 6640498 (TED, €14 bn)

The other band ratios: ×10 on 6 rows and ×1,000 on 2. "×smallest" is not the same as an exact partner:
the smallest figure can sit in a different field or version, and the rule also wants a single-field head.

4871119 is issue 489's flip-flop case: an F14 corrects £25 bn to £250 m, then a second F14 corrects it back
to £25 bn. A ×100 rule would refuse a figure the publisher restated twice. That is one reason k = 2 needs
its own adjudication before it is extended.

Two attempts at a direct partner probe over heads of €1 bn or more, and then €10 bn or more, both answered
408 during project 2067 and were not retried. The €1–10 bn range waits for a quiet box: a windowed
`tenders_current_value_eur` read with the partner test per window, sized like section 16's.

