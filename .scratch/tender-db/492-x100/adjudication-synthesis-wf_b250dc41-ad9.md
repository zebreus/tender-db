**Issue 492, unit 2: whether ×100 heads in the €1–10 bn range should be refused**

**Recommendation:** extend the rule to k = 2, but exempt one shape: a procedure total over its lots. On the 56 adjudicated Tenders this refuses all 45 slips (before the 471 corroboration exemption) and wrongly refuses one genuine head (8819939, 1.8%). A plain k = 2 rule wrongly refuses 10 (17.9%). The rule as issue 492 first framed it, "refuse only when the partner is a sibling lot or the sum check holds", has it backwards. The lot relation and the lots-sum check are what the genuine heads look like, so they belong in an exemption, not in the condition that refuses.

To check the lot structure I ran bounded `tender_version_lot_results` reads by `tender_id` on 20 Tenders (one row per lot award, so I can count and sum lots per version). The verdicts themselves come from the per-Tender adjudication.

**1. Counts**

| Verdict | Count | Tenders |
|---|---|---|
| SLIP_HEAD | 45 | |
| GENUINE_DISTINCT | 9 | 8713680, 8423121, 8804016, 6572976, 8730895, 6904931, 7956096, 8800131, 8638612 |
| SLIP_PARTNER | 1 | 8819939 |
| PLACEHOLDER | 1 | 6988280 |
| UNCLEAR | 0 | |

| Partner relation | Count | Verdicts |
|---|---|---|
| same field, earlier version | 17 | 16 SLIP_HEAD, 1 SLIP_PARTNER (8819939) |
| same version, other field | 12 | all SLIP_HEAD |
| lot vs procedure | 20 | 11 SLIP_HEAD, 9 GENUINE (all nine genuine heads are here) |
| other (cross-version, cross-field) | 6 | all SLIP_HEAD |
| sibling lot | 1 | PLACEHOLDER (6988280) |

The 11 lot-vs-procedure slips:
- **10 are single-lot procedures**, where the lot and the procedure should be equal: 1177887, 8396822, 524394, 6181847, 7683806, 1003919, 8715174, 292242, 627800, 474292.
  - Five of them have the lot estimate at 100× its own procedure total (BT-27-Lot): 1003919, 8715174, 292242, 627800, 474292.
- **1 is a three-lot procedure:** 8413585, whose total is 27.7× the sum of its lots.

The nine genuine heads are all national or regional frameworks or DPSs with 4 to 22 lots. In each, the head is the procedure total and the partner is one small lot.

**2. False-positive rate of a plain k = 2 rule** (`SCALE_ERROR_MIN_EXPONENT` lowered to 2, nothing else changed)

- **Without the 471 corroboration exemption:** all 56 heads are refused.
  - 10 of them wrongly: the 9 genuine plus 8819939. That is 10/56 = 17.9%, or a precision of 45/55 = 81.8%.
  - 6988280 is neutral: it drops from one placeholder (€1 bn) to another (€10 m).
  - For comparison, 471's k ≥ 3 rule had 1 false positive in 65 (8287294).
- **With the 471 corroboration exemption:** it is not a separator at k = 2.
  - **Genuine heads still wrongly refused:** at least 4 (8713680, 8423121, 8804016, 8800131 are confirmed unprotected) and up to 9, plus 8819939.
  - **Slips it keeps:** 5 confirmed (5579054, 7393311, 7393312, 7393320, 928196) and probably 3 more (5803269, 7393314, 7393321).
  - In every one of those, a single notice copies one figure into two slots (RES VAL_ESTIMATED_TOTAL and VAL_TOTAL, or BT-27 and BT-271).
  - Net result: 37–40 of 45 slips caught, with 5–10 wrong refusals.

**3. Whether a narrower condition separates the slips**

Conditions tried as the trigger for refusal:

| Condition | Slips refused (of 45) | Wrong refusals |
|---|---|---|
| Partner is a sibling lot | 0 | 0 (only 6988280, a placeholder) |
| Lot estimate above its own procedure total | 5 | 0 |
| Partner is the same field in an earlier version | 16 | 1 (8819939) |

The condition that separates works as an exemption instead: keep a ×100 head F only when all three of these hold.
- **(b)** The head version has two or more lots, and F is a procedure figure, not a lot figure.
- **(c)** Every ×100 partner is a lot figure: a lot-scope amount, a lot award, or a lot-null `result_value` that equals a lot award of the same version.
- **(d)** The head version's lot figures sum to between F/10 and F.

Results:
- **Genuine heads:** all 9 kept.
  - The lots sum to F exactly in 6 of them (8713680, 8804016, 8730895, 7956096, 8800131, 8638612).
  - The others are 6572976 (F is 1.85× the lot sum), 6904931 (2.7×) and 8423121 (6.1×).
- **Single-lot slips:** 35, all refused by (b).
- **Multi-lot slips:** 10, all refused. Two of the conditions are each needed for some of them:
  - (c) alone would let 8413585, 5923251 and 8332921 through. The sum check refuses them at 27.7×, 91.7× and 50×.
  - (d) alone would let 5864294 (1.01×) and 7240718 (1.82×) through, because their lot awards carry the same ×100 slip. Condition (c) refuses them because the partner is the earlier version's procedure estimate.
  - 704313 (11.1×), 6449525 and 7428665 (lots summing to more than F), 6728980 (202×) and 928196 (100×) fail both.
- **The threshold of 10** sits between the largest genuine ratio (6.1, 8423121) and the smallest ratio that only the sum check catches (27.7, 8413585).
- **Overall:** 45 of 45 refused before corroboration, 1 wrong refusal (8819939, 1.8%), which matches the k ≥ 3 rate.
- **8819939 stays refused, and that is the cost.** It is a single-lot Amsterdam concession at €1.2 bn; the earlier preliminary notice's €12 m understates it. Refusing it drops the head to €12 m. Record it the way 8287294 is recorded for 471.

**4. Recommendation**

1. **Extend the rule to k = 2 with the exemption (b) + (c) + (d)**, and leave k ≥ 3 unchanged.
2. **Data-model hazard for unit 3:** `ScalePartners.figures` holds only currency and cents, so the change needs scope per figure, the lot count of the head version, and the lot sums. In TED r2.0.9 and in pre-eForms FTS award notices, lot awards land in `tender_version_amounts` with `lot_id` set to NULL; only `tender_version_lot_results` carries the lot. Examples: 6904931, 8423121, 6572976, 8413585, 6181847.
   - If partner scope is read from `amounts.lot_id` alone, 3 genuine heads become wrong refusals: 8423121, 6572976 and 6904931.
3. **Corroboration at k = 2:** in this sample it protects no genuine head and keeps 5–8 slips. My recommendation is not to apply it at k = 2. If unit 3 prefers to keep a single shared check, keep it and name those 5–8 as the known residue.
4. **Refused is not corrected.** At the drain, re-read these, which will probably fall to another wrong figure:
   - 5864294 to its lot-2 award of €287 m (itself ×100);
   - 7240718 to a ×100 lot result of DKK 1.96 bn;
   - 6449525 to a garbled €399.8 m;
   - 6988280 to the €10 m placeholder.
5. **Pin tests:**
   - 8800131 kept (lots sum exactly);
   - 8423121 kept (r2.0.9 award notice, lot partners stored at lot NULL, 6.1×);
   - 8413585 refused (27.7×);
   - 5864294 refused (sum is about F, but the partner is a procedure figure);
   - 474292 refused (single lot, lot 100× procedure);
   - 8819939 as the recorded wrong refusal.
6. **Before the drain,** adjudicate the ≥ €10 bn rows that sit at ×100, since the rule would reach them too:
   - 8595426, 8618327, 8810872 are unadjudicated;
   - 6640498 and 8784848 are known slips; 8784848 is a lot head, so it is refused;
   - 4871119 is issue 489's flip-flop case, and the rule would refuse it, so decide it with 489.
7. The €1 bn floor stays; nothing below it was measured.

The hit list is `/home/user/tender-db/.scratch/tender-db/492-x100/x100-partner-hits-2026-10-09.json`, and the rule is in `/home/user/tender-db/crates/store/src/canonical.rs` around lines 3299–3500.