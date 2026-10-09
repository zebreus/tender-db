**Issue 505 unit 1: adjudication of the 23 EUR 1 bn+ heads elected from a lot figure >= 10x the version's procedure figure (2026-10-09)**

The adjudication of the 23 Tenders at 10× or more is done. 18 of them are slips, so refusing the lot figure would be right. 5 are genuine values, so refusing them would be wrong. I recommend building only rule (b): it catches 4 slips and wrongly refuses none of the 23. All other Tenders stay as published, under a caveat.

The two readers disagreed only on 578884 and 8716838. Both readers already agreed that refusing the lot is right on each, and I re-read both:
- **578884: BOTH_JUNK.** The award is €382,486.23, stated three times in notice 44511057 (BT-161, BT-710, BT-711). BT-720 gives €38.2m, which is the award ×100. The procedure estimate of €220m is 575× the award. Refusing the lot still leaves a wrong head of €220m.
- **8716838: BOTH_JUNK.** The contract is £5,225,037, which is 0.997× £5.24m. So the procedure figure of £52.4m has one extra zero, and the lot figure of £5.42bn is about 1000× with digits 524 swapped to 542. Refusing the lot leaves a wrong head of about €61m.

I also re-read 8682609 and keep it as PROCEDURE_JUNK (medium confidence): all 32 contract rows in the two award notices are £1.359bn.

## Verdicts
"Right" means refusing the lot figure would be right; "wrong" means it would be wrong.

| Tender | Ratio | Lots | Verdict | Refuse | Shape | True value | Confidence |
|---|---|---|---|---|---|---|---|
| 8698658 | 9.3m | 1 | PROCEDURE_JUNK | wrong | procedure is a £100 token | £933.65m estimate; IBM award £710.9m | high |
| 153108 | 10000.0003 | 1 | LOT_SLIP | right | decimal point dropped; the seq 2 correction fixed only the procedure figure | €327,582.51 | high |
| 162784 | 6139 | 1 | LOT_SLIP | right | round BGN 4bn framework maximum, 7005× the estimate | BGN 571k estimate / 651.5k award | high |
| 4478154 | 3963 | 1 | LOT_SLIP | right | lot FMTVAL is 8501350000 | €2.145m | high |
| 1155337 | 2116 | 39 | LOT_GENUINE | wrong | framework total copied onto 38 lots; procedure figure is a call-off welded into the Tender | about €4.01bn | high |
| 304460 | 1190 | 1 | LOT_SLIP | right | ratio is 1000 × 1.19 (VAT); procedure figure is the net award | €3.7128m gross / €3.12m net | high |
| 666474 | 1109 | 1 | PROCEDURE_JUNK | wrong | the award was entered in thousands of euros | about €2.25bn award; €2.5bn estimate | high |
| 8818621 | 942 | 6 | LOT_SLIP | right | elected figure is a carried PIN Part with garbled digits | €1,187,620 | high |
| 553044 | 534 | 3 | LOT_SLIP | right | **sum residual, ×1000, against the framework maximum** | €26.958m | high |
| 8821978 | 526 | 5 | LOT_SLIP | right | both lots ×1000; their sum is €20 off the procedure figure | €14.11m | high |
| 627219 | 498 | 1 | LOT_SLIP | right | ×100 of BT-1118/BT-660, which are not stored as amounts | €33.23m maximum | high |
| 1120720 | 413 | 4 | LOT_SLIP | right | **sum residual, ×1000** | €2.85m | high |
| 395737 | 408 | 1 | LOT_SLIP | right | ×100 of BT-709-LotResult, which is not stored | BGN 29.99m maximum / 7.35m estimate | high |
| 5748163 | 233 | 16 | LOT_SLIP | right | **sum residual, ×1000** | €8,416,216 | high |
| 8716838 | 103 | 1 | BOTH_JUNK | right | lot about ×1000; procedure about ×10 | about £5.2m | high |
| 578884 | 100.9 | 1 | BOTH_JUNK | right | lot cents read as units; procedure 575× the award | about €0.38m award | medium |
| 8784848 | 60 | 3 | LOT_SLIP | right | **sum residual, ×100** | £1.0bn (lot 1 £600m) | high |
| 514188 | 29.5 | 94 | LOT_SLIP | right | 88 lots ×1000; lots sum to 97% of the procedure figure, not exact | €140.9m | high |
| 435764 | 23.4 | 5 | LOT_SLIP | right | ×100 of the lot's own description text; procedure figure is the award | €61.59m estimate / €52.76m award | high |
| 1016150 | 21.7 | 1 | LOT_GENUINE | wrong | framework maximum against one call-off's result | CZK 40bn | high |
| 8682609 | 10.0 exact | 1 | PROCEDURE_JUNK | wrong | exact ×10; procedure lost a zero | £1.359bn | medium |
| 8676548 | 10.0 exact | 1 | LOT_SLIP | right | exact ×10 extra zero | £260m | high |
| 174969 | 10.0 exact | 1 | LOT_SLIP | right | exact ×10; the lot and procedure estimates are equal (€277m) | €554m maximum | high |

**Counts by verdict:** 16 LOT_SLIP, 2 BOTH_JUNK, 3 PROCEDURE_JUNK, 2 LOT_GENUINE, 0 UNCLEAR. That makes 18 right refusals and 5 wrong ones. The 18 slips serve €162.9bn of head value; the 5 genuine Tenders serve €10.8bn.

**Counts by ratio band:**

| Band | Tenders | Right | Wrong | Wrong ones |
|---|---|---|---|---|
| 10–50× | 6 | 4 | 2 | 1016150, 8682609 |
| 50–100× | 1 | 1 | 0 | |
| 100–1000× | 9 | 9 | 0 | |
| ≥1000× | 7 | 4 | 3 | 8698658, 1155337, 666474 |

**Counts by lot count:**
- **One lot:** 14 Tenders, 10 right and 4 wrong.
- **Two or more lots:** 9 Tenders, 8 right and 1 wrong (1155337).

## What separates the slips from the genuine values
In all 5 wrong refusals, the procedure figure is the odd one out, not the lot. No single ratio, lot count or field type separates them:
- **The procedure figure is an award:** 666474 (entered in thousands of euros) and 1016150 (one call-off). But the slips 435764 and 304460 also have only an award as their procedure figure.
- **The procedure figure is a token:** 8698658 (£100).
- **The procedure figure comes from a welded call-off:** 1155337. An earlier version (seq 1) has a procedure figure equal to the lot figure, €4.01bn. The slip 153108's seq 1 also had procedure equal to lot, so "an earlier version vouches for the lot" would wrongly exempt it.
- **The direction of an exact ×10 is ambiguous:** 8682609.

The only condition in the sample with no false positive is the exact sum residual (the shape of 8784848). The version has two or more lots. The procedure figure minus the other lots, in the same field and currency, times 10^k exactly with k of 2 or more, equals the lot figure. Four Tenders have it:
- 8784848: lot 1 is £600m ×100.
- 553044: lot 2 is €14.4m ×1000, against the procedure framework maximum. It does not match against the procedure estimate, so the comparison has to be field to field.
- 5748163: lot 1 is €1.96m ×1000.
- 1120720: lot 4 is €1.178m ×1000.

## The candidate rules on the 23 Tenders

| Rule | Refuses | Right | Wrong | Notes |
|---|---|---|---|---|
| (a) lot at 10× or more of the procedure figure | 23 | 18 | 5 (21.7%) | compare 1 wrong in 56 for 492's head rule |
| (a2) same, but the procedure figure must be a non-token estimate or maximum | 18 | 16 | 2 (8682609, 1155337) | misses 435764 and 304460 |
| (b) exact sum residual, two or more lots, k of 2 or more | 4 | 4 | 0 | 55% (€89.6bn) of the slip value served |
| (b) extended to single lots (exact ×10) | 7 | 6 | 1 (8682609) | |
| (c) exact power of ten | 3 | 2 | 1 (33%) | all at k=1: 174969, 8676548, 8682609 |
| Ratio band 50–1000× | 10 | 10 | 0 | not principled; see below |
| Two or more lots and 10× or more | 9 | 8 | 1 (1155337) | |

Notes on these rules:
- **(c):** only k=1 is left here, because 471 and 492 already refuse exact matches at k of 2 or more. k=1 is the level most open to round-number coincidence. 153108 misses an exact match by 97 cents.
- **The ratio band:** 1155337's own 32 call-off versions would give ratios from 841× (seq 13, €4.77m) to 18,147× (seq 29). So the same welded genuine Tender falls inside the band depending on which call-off is the head version.

## Recommendation
1. **Build rule (b).** Refuse a lot figure when all of these hold:
   - the version has two or more Lots (kind Lot only, not Parts or LotsGroups);
   - the lot, its sibling Lots and an admitted procedure figure share one field and one currency;
   - the residual (procedure figure minus the other Lots) is above zero;
   - the lot figure is exactly 10^k times the residual, with k of 2 or more;
   - the lot is worth €1bn or more (the gate in `refuses_amount`).

   It fixes the 4 Tenders above: 8784848 falls from €72bn to £1bn (€1.2bn), 553044 to €26.958m, 5748163 to €8.42m and 1120720 to €2.85m. The 10× clause adds nothing on the sample, so lot above procedure is enough.

   **Before building,** run the predicate over the 862 Tenders that hold 4,974 lot rows at €1bn or more, and adjudicate any hit outside these 23. In 492, the rule measured on heads went 2 wrong in 6 on lot rows.
2. **Do not build** (a), (a2), (c) or a ratio band.
3. **Leave the other 14 slips caveat-only**, with these follow-ups that are not ratio rules:
   - **Ingest the result-level figures as amounts.** Storing BT-709-LotResult, BT-1118 and BT-660 would give 395737 and 627219 an exact ×100 partner, so 492's rule would refuse them.
   - **Elect a PIN Part as head only when the version has no Lot.** That fixes 8818621 and mirrors 492's lot-sum fix.
   - **1155337's problem is the weld, not a value rule.** A framework award and 32 call-off notices are folded into one Tender.
   - **Two fallbacks stay wrong after a refusal.** 578884 would serve €220m and 8716838 about €61m; record this in the caveat.
   - **Left as published:** 174969, 8676548, 153108, 162784, 304460, 435764, 514188, 8821978 and 4478154. Each has a one-Tender signal (cross-field equality, a digit prefix, VAT ×1000, description text, a 97% or ±€20 near-sum, a round sentinel), too thin to build a rule on.

The unit 1 measurement is in `/home/user/tender-db/.scratch/tender-db/505-lot-over-procedure/heads-2026-10-09.json`.
