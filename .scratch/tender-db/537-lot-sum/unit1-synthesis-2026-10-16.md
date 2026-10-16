## Unit 1: measured (2026-10-16)

**Summary.** Of the 48 matched Tenders, 43 have the wrong head, 4 have the wrong lots and 1 is a coincidence. The final verdicts contain no both_plausible or unclear. The recommended rule would refuse 22 heads, all of them wrong heads, with no wrong refusal. That is 22 of the 43 wrong heads. All 22 drop below €1 bn. Two fallbacks the readers named are not what the election would serve (867491 and 114991, below).

**Sources:**
- Workflow output: first reader plus skeptic for each Tender.
- `crates/store/src/canonical.rs`: `head_value_eur_cents_with` (3284–3360), `ScalePartners` (3590–4110), the `IMPLAUSIBLE_EUR_CENTS` doc (3443–3458).
- 7 public GETs by the synthesiser on 2026-10-16: 8312215, 867491, 114991, 8400892, 8720567, 4726892, 7070770.

**Population:**
- 3,750 elected heads at ≥ €1 bn and below the €100 bn cap.
- 2,082 of them carry at least one lot-scoped figure in the head's currency. The other 1,668 cannot be tested.
- 48 Tenders match `|F − 10ᵏ·S| ≤ 10ᵏ·n` cents for some candidate lot sum S over n lots: k=1 17, k=2 10, k=3 20, k=4 1. The sweep reported no match at k ≥ 5. 43 are TED and 5 are FTS.
- Sanity population at ratio 1 (the head equals its lot sum): 776 lot-estimate sums and 550 award sums. Not adjudicated, and no rule here touches k = 0.

**Residuals:**
- Every match is exact except four:
  - 369057 awards −1000 cents. Its BT-709 sum is exact.
  - 165437 +600.
  - 6921980 +200 (k=2, n=15).
  - 801998 −100 (k=2, n=14).
- At the lot scale the largest gap is 2 cents (6921980), so a tolerance of `10ᵏ·n` covers all four.

**The kinds of S the sweep built** (unit 1 proposed only the first three; it also summed the last two):

| S kind | field | side |
|---|---|---|
| awards | lot_results.awarded, each lot once, from its latest award | award |
| BT-709 | result_framework_maximum (partner-only) | award |
| BT-660 | result_framework_reestimate (partner-only) | award |
| est | lot estimated_value (BT-27-Lot) | estimate |
| BT-271-Lot | lot framework_maximum | estimate |

### 1. Measurement

**Final verdicts:** head_wrong (HW) 43, lots_wrong (LW) 4, coincidence (CO) 1.
- Two first-reader calls were both_plausible, and both were refuted by the skeptic to head_wrong. Neither verdict holds on in-Tender evidence alone:
  - 130304 rests on an FTS twin, Tender 8640633.
  - 8717158 rests on a re-tender folded into Tender 7954684.
- 8596393's lots_wrong is moderate confidence (both readers).

| split | HW | LW | CO |
|---|---|---|---|
| k = 1 (17) | 15 | 1 (8596393) | 1 (1039053) |
| k = 2 (10) | 9 | 1 (8720567) | 0 |
| k = 3 (20) | 19 | 1 (8400892) | 0 |
| k = 4 (1) | 0 | 1 (8312215) | 0 |
| S = awards (28 Tenders) | 26 | 2 (8400892, 8596393) | 0 |
| S = est, BT-27-Lot (19) | 15 | 3 (8312215, 8720567, 8596393) | 1 (1039053) |
| S = BT-709 / BT-660 (3) | 3 | 0 | 0 |
| S = BT-271-Lot (5) | 5 | 0 | 0 |
| head field `result_value`, BT-161 or TED VAL_TOTAL (28) | **28** | 0 | 0 |
| head field `estimated_value` (18) | 14 | **4** | 0 |
| head field `framework_maximum` (2) | 1 (508666) | 0 | 1 (1039053) |
| n = 1 lot summed (10) | 9 | 1 (8400892) | 0 |
| n ≥ 2 (38) | 34 | 3 | 1 |
| TED (43) | 40 | 2 | 1 |
| FTS (5) | 3 | 2 | 0 |

- Eight Tenders match on two kinds:
  - awards + BT-709: 369057
  - awards + est: 1123701, 8596393
  - est + BT-271-Lot: 1011786, 1022481, 511132
  - BT-271-Lot + BT-709: 1024214
  - BT-709 + BT-660: 317736
- Every one of the 5 wrong rows has a round lot sum (at most 5 significant digits), and so does every k = 1 sum. The 17 matches whose sum has at least 6 significant digits are all head_wrong. This is noted, not adopted: the threshold is arbitrary, and it loses round-lot slips such as 7427765, 5779108 and 7829010.

**Third-figure guard T** (used in section 2). T holds when the Tender's chain, in any version, holds a figure X with all of these:
- X is in F's currency;
- X is recorded at procedure scope at least once (`ScalePartners.lot_only[(cur, X)] == false`, so a lot-null copy of an award does not count);
- X ≠ F;
- X is above `SCALE_PARTNER_FLOOR_CENTS`;
- S/10 ≤ X ≤ 10·S.

T was judged from the readers' notice reads plus the 7 GETs. Figures in earlier versions (for example 6921980 v1, 6036813 v1, 317736's CN versions) are taken as stored amounts of those versions, as the readers quote them.

**Exhibits.** "T: yes" names the qualifying X. A starred verdict was disputed between the readers.

| id | match (k:S:n) | head field | src | T | verdict | note |
|---|---|---|---|---|---|---|
| 8312215 | 4:est:2 | est | TED | no | LW | Value texts say "2 billion GBP" per lot; the lots print £200k. Head in one field (GET). |
| 369057 | 3:awards:3 (−1000), 3:709:3 (0) | result | TED | no | HW | CN estimate €79,142,248 sits in another Tender (BT-04 differs). |
| 165437 | 3:awards:3 (+600) | result | TED | yes: est 2020289170 | HW | Three-decimal shape. |
| 8636199 | 3:est:2 | est | FTS | no | HW | Plausibility only. |
| 508666 | 3:BT-271-Lot:4 | fwmax | TED | yes: est 588000000 | HW | Each lot's BT-271 = 1.5 × its BT-27. |
| 1024214 | 3:BT-271-Lot:3, 3:709:3 | result | TED | no | HW | BT-300 annual caps × 4 = the lot maxima. |
| 7427765 | 3:awards:6 | result | TED | no | HW | All figures round; decided on plausibility plus an SME flag. |
| 164283 | 3:est:5 | est | TED | no | HW | The next figure, result_value €2.486 bn, carries the same ×1000 slip. |
| 7044132 | 3:awards:7 | result | TED | no | HW | CN text €596,400 a year × 4 years. |
| 5779108 | 3:awards:2 | result | TED | no | HW | CN text annual maxima × 4. |
| 905123 | 3:est:6 | est | TED | no | HW | BT-300 annual maxima. |
| 317736 | 3:709:1, 3:660:1 | est | TED | yes: CN BT-27 201680672 | HW | 471-corroborated (est + fwmax). |
| 7829010 | 3:awards:2 | result | TED | no | HW | Plausibility only. |
| 114991 | 3:est:3 | est | TED | yes: BT-161 75683700 | HW | |
| 867491 | 3:awards:3 | est | TED | yes: BT-161 136080000 | HW | LOT-0001's BT-27-Lot carries the same ×1000 slip. |
| 380921 | 3:awards:4 | result | TED | no | HW | Tender security (wadium) is 1.97 % of the award sum. |
| 425446 | 3:awards:4 | result | TED | no | HW | Plausibility only. |
| 5943121 | 3:awards:10 | result | TED | no | HW | Plausibility plus the digit match. |
| 1140407 | 3:awards:2 | result | TED | yes: est 120753600 | HW | |
| 622872 | 3:est:1 | est | TED | yes: CN BT-271 100000000 | HW | 471-corroborated. |
| 8400892 | 3:awards:1 | est | TED | no | LW | 471-corroborated, kept today. PIN and CN (other Tenders) at the head's scale. |
| 392980 | 2:awards:2 | result | TED | no | HW | The head followed a lot correction, ×100 both times. |
| 6921980 | 2:awards:15 (+200) | result | TED | yes: v1 est 2500000000 | HW | |
| 251619 | 2:awards:4 | result | TED | no | HW | BT-202 modification amounts are 20 % of the lots. |
| 8514592 | 2:awards:2 | result | TED | no | HW | Plausibility plus separator shape. 2 of 3 lots awarded. |
| 706887 | 2:awards:10 | result | TED | no | HW | Plausibility plus digit shape. |
| 812132 | 2:est:5 | est | TED | no | HW | The 2021 CN sits in Tender 7153347. Head in 2 fields. |
| 801998 | 2:awards:14 (−100) | result | TED | yes: est 1271119000 | HW | 14 of 17 lots awarded. |
| 470107 | 2:awards:3 | result | TED | yes: fwmax 1495320000 | HW | |
| 6342034 | 2:awards:2 | result | TED | no | HW | Plausibility plus separator shape. |
| 8720567 | 2:est:2 | est | FTS | no | LW | Sibling DPV notices at £1.6 bn; size. |
| 7209792 | 1:awards:1 | result | TED | yes: CN est 4000000000000 | HW | |
| 1123701 | 1:awards:3, 1:est:3 | result | TED | yes: est 88500000000 | HW | |
| 1011786 | 1:est:4, 1:BT-271-Lot:4 | result | TED | yes | HW | Its award rows are 46 × the framework total (not a per-lot sum). |
| 1022481 | 1:est:1, 1:BT-271-Lot:1 | est | TED | no | HW | The lot (€350 m) is not vindicated either. Head in 2 fields. |
| 7425703 | 1:awards:4 | est | TED | yes: CAN VAL_TOTAL 32500000000 | HW | |
| 511132 | 1:est:1, 1:BT-271-Lot:1 | result | TED | yes | HW | |
| 130304 | 1:est:1 | est | TED | no | HW* | First reader: both_plausible. Head in 2 fields. |
| 8452581 | 1:awards:1 | est | TED | yes | HW | The head is a lot-null per-winner row (RES-9 of 33) with an extra zero. |
| 8770281 | 1:est:1 | est | FTS | no | HW | Contract value £136,422,265 is not stored as an amount. |
| 7070770 | 1:awards:1 | result | TED | no | HW | |
| 7422241 | 1:awards:3 | result | TED | no | HW | Plausibility only. |
| 8717158 | 1:est:4 | est | FTS | no | HW* | First reader: both_plausible. Procedure abandoned. |
| 676632 | 1:est:13 | result | TED | yes | HW | |
| 4726892 | 1:est:5 | result | TED | no | HW | The CN range £85–100 m is unmapped. Could be ingest-side. |
| 6036813 | 1:awards:7 | result | TED | yes: v1 est 10000000000 | HW | Double-count reading is a near miss (€1,003 m vs €1,000 m). |
| 8596393 | 1:awards:2, 1:est:2 | est | FTS | no | LW (moderate) | The buyer's UK1 notice (£650 m) sits in Tender 8694887. |
| 1039053 | 1:est:52 | fwmax | TED | yes | CO | Deliberate 10× ceiling (BT-300). The lot BT-271 sum equals the head at k = 0. |

**"Plausibility only".** These verdicts rest on the contract's plausible size plus digit shape, with no independent printed figure: 8636199, 7427765, 7829010, 425446, 5943121, 8514592, 706887, 6342034, 7422241, 7070770 (whose V.2.3 is a copy) and 1022481 (sibling Tenders).

### 2. Precision of candidate refuse rules on the 48

**Rule template.** Refuse a head F (at least €1 bn, at most `IMPLAUSIBLE_EUR_CENTS`) when, for some k ≥ k_min, `|F − 10ᵏ·S| ≤ 10ᵏ·n` cents.
- (a) is k_min = 3, (b) is k_min = 2, (c) is k_min = 1.
- **"n ≥ 2"** means the matching S sums at least 2 lots that carry a figure of that kind.
- **"T"** is the guard defined in section 1.
- A Tender counts as refused when any one of its match rows passes.
- Each cell reads **right refusals (head_wrong) / wrong refusals (lots_wrong + coincidence)**, with the wrong ids in brackets.

Matched counts: (a) 21, (b) 31, (c) 48.

**Any S kind**

| rule | all n, no guard | n ≥ 2, no guard | all n, T | n ≥ 2, T |
|---|---|---|---|---|
| (a) k ≥ 3 | 19 / 2 [8400892, 8312215] | 17 / 1 [8312215] | 7 / 0 | 5 / 0 |
| (b) k ≥ 2 | 28 / 3 [+8720567] | 26 / 2 [8312215, 8720567] | 10 / 0 | 8 / 0 |
| (c) k ≥ 1 | 43 / 5 [+8596393, 1039053] | 34 / 4 [8312215, 8720567, 8596393, 1039053] | 18 / 1 [1039053] | 13 / 1 [1039053] |

**By S kind** (cells: all n, no guard · n ≥ 2, no guard · all n, T · n ≥ 2, T)

| S kind | (a) k ≥ 3 | (b) k ≥ 2 | (c) k ≥ 1 |
|---|---|---|---|
| awards | 11/1 [8400892] · 11/0 · 3/0 · 3/0 | 19/1 [8400892] · **19/0** · 6/0 · 6/0 | 26/2 [8400892, 8596393] · 23/1 [8596393] · 11/0 · 9/0 |
| est (BT-27-Lot) | 5/1 [8312215] · 4/1 · 2/0 · 1/0 | 6/2 [8312215, 8720567] · 5/2 · 2/0 · 1/0 | 15/4 · 10/4 · 6/1 [1039053] · 4/1 |
| BT-709 / BT-660 (all at k = 3) | 3/0 · 2/0 · 1/0 · 0/0 | same | same |
| BT-271-Lot | 2/0 · 2/0 · 1/0 · 1/0 | 2/0 · 2/0 · 1/0 · 1/0 | 5/0 · 3/0 · 3/0 · 2/0 |
| award side (awards ∪ BT-709 ∪ BT-660) | 13/1 · 12/0 · 4/0 · 3/0 | 21/1 [8400892] · **20/0** · 7/0 · 6/0 | 28/2 · 24/1 [8596393] · 12/0 · 9/0 |
| estimate side (BT-27-Lot ∪ BT-271-Lot) | 7/1 · 6/1 · 3/0 · 2/0 | 8/2 · 7/2 · 3/0 · **2/0** | 17/4 · 12/4 · 7/1 · 5/1 |

**Head-field slice.**
- `result_value` heads (28, all head_wrong), any kind of S: (a) 11, (b) 19, (c) 28 (25 with n ≥ 2). No wrong refusal in any cell.
- All 4 lots_wrong rows have `estimated_value` heads. The coincidence has a `framework_maximum` head.

**What the measurement says:**
- Every wrong refusal comes through an estimate-side S (8312215, 8720567, 8596393, 1039053) or through n = 1 (8400892).
- Award-side sums over at least 2 lots are clean at k ≥ 2: 20 / 0.
- On the estimate side, T separates the two k ≥ 2 lots_wrong rows (both T: no) from 2 of the 6 head_wrong rows (T: yes). That is n = 2 against 2, so it is thin.
- k = 1 is 15 / 2. Its sums are all round, so a match carries no digit information.
- Without the 10ᵏ·n tolerance (exact matches only), 165437 (a done exhibit), 6921980 and 801998 are lost. 369057 survives through its exact BT-709 sum.

**Sensitivity:**
- If 130304 and 8717158 were scored both_plausible (the first readers' calls), only k ≥ 1 cells without the guard change: any (c) all n 43/5 → 41/7; any (c) n ≥ 2 34/4 → 33/5; est (c) 15/4 → 13/6 and 10/4 → 9/5. Both are k = 1, T: no, so no k ≥ 2 cell and no guarded cell moves.
- 8596393 is k = 1, so no k ≥ 2 cell depends on its moderate-confidence verdict.

### 3. Recommended unit-2 rule

**Rule.** Applied in `ScalePartners`, at election time (492's placement). Refuse every procedure-scope fact of the head version that is not a lot-award copy, whose cents equal F, worth at least `SCALE_ERROR_MIN_EUR_CENTS` and at most `IMPLAUSIBLE_EUR_CENTS`, when for some **k in 2..=6** and some S built from the head version over **n ≥ 2** lots:

`|F − 10ᵏ·S| ≤ 10ᵏ·n` cents

where S is one of:
- **(A) award side, no further condition:**
  - lot awards, each Lot once, taken from its **latest** award;
  - BT-709, each lot once;
  - BT-660, each lot once.

  The partner-only rows serve as S but stay never-elected and out of `head_lot_sums` (506).
- **(B) estimate side, only with guard T:**
  - BT-27-Lot, each Lot's largest figure in the field;
  - BT-271-Lot, each Lot's largest figure in the field.

Details:
- Lots of kind Lot only. Parts count only in a head with no Lot (`set_head`'s rule). Withheld figures and sentinels are skipped.
- No corroboration exemption (492's reason). In the 48, six heads sit in two electable fields: 8400892 (lots_wrong) and 317736, 622872, 1022481, 130304, 812132 (all head_wrong). None of them reaches this rule's refusal set, so the choice measures the same either way.
- When 492's `framework_total` keeps F, this rule does not refuse it. That costs nothing here: none of the 22 has a ×100 single partner, so 492 is silent on all of them.

**Effect on the band.**
- Refuses **22, all head_wrong, no wrong refusal**: 20 through (A), and 508666 and 114991 through (B). With 0 wrong in 22, the rule-of-three 95 % upper bound on the error rate is about 13.6 %. For comparison, 471 measured 1 in 65 and 492 measured 1 in 56.
- All 22 fall below €1 bn, so the band goes from 3,750 to 3,728, provided unit 3 builds the same S as unit 1 measured.
- Of the 31 matches at k ≥ 2 it keeps 9:
  - 3 lots_wrong, correctly: 8400892 (n = 1), 8312215 and 8720567 (T: no).
  - 6 head_wrong, missed: 8636199, 164283, 905123 and 812132 (estimate side, T: no); 317736 and 622872 (n = 1, both survivors of 471's corroboration exemption).
- All 17 k = 1 rows are untouched. That includes 15 head_wrong left as published.

**What each refused head falls to.** This is the election's maximum over the remaining procedure and head-lot candidates, checked against `head_value_eur_cents_with`. The readers' named figure is given where it differs.

| id | head | falls to | judgement |
|---|---|---|---|
| 369057 | €62,378,736,280.00 | **none** (null) | No candidate is left. The plausible total is the award sum, €62,378,736.29. |
| 380921 | PLN 7,247,040,000.00 | none | Plausible total: PLN 7,247,040.00. |
| 425446 | €1,549,007,190.00 | none | Plausible total: €1,549,007.19. |
| 392980 | €3,574,004,272.00 | none | Plausible total: €35,740,042.72. |
| 251619 | €2,433,875,808.00 | none | Plausible total: €24,338,758.08. |
| 706887 | PLN 7,056,079,275.00 | none | Plausible total: PLN 70,560,792.75. |
| 165437 | €19,975,274,626.00 | estimated_value €20,202,891.70 | Plausible (the done line). |
| 1140407 | €1,152,119,880.00 | estimated_value €1,207,536.00 | Plausible. |
| 801998 | €1,290,405,282.00 | estimated_value €12,711,190.00 | Plausible. |
| 470107 | €1,285,254,740.00 | framework_maximum €14,953,200.00 | Plausible. |
| 508666 | €8,820,000,000.00 | estimated_value €5,880,000.00 | Plausible. The true framework maximum is €8.82 m. |
| 7829010 | €2,000,000,000.00 | €1,965,000.00 (RES-2, lot-null copy) | 98.25 % of €2,000,000. |
| 8514592 | DKK 17,924,893,700.00 | DKK 158,639,473.00 (LOT-1) | 88.5 % of DKK 179,248,937.00. |
| 6342034 | €1,083,675,445.00 | €7,686,754.45 (lot 1) | 70.9 % of €10,836,754.45. |
| 5779108 | €2,120,000,000.00 | €1,400,000.00 (LOT-1) | 66.0 % of €2,120,000. |
| 6921980 | €2,493,033,613.00 | €10,369,747.89 (LOT-8) | 41.6 % of €24,930,336.11. |
| 1024214 | €3,400,000,000.00 | €1,200,000.00 (one lot's BT-271) | 35.3 % of €3,400,000. |
| 7044132 | €2,385,992,000.00 | €804,268.00 (LOT-3) | 33.7 % of €2,385,992.00. |
| 5943121 | €1,321,579,920.00 | €416,053.05 (one lot) | 31.5 % of €1,321,579.92. |
| 7427765 | €3,000,000,000.00 | €500,000.00 (one lot) | 1/6 of €3,000,000. |
| 114991 | €1,765,000,000.00 | **€1,000,000.00 (LOT-0001's BT-27-Lot)** | Readers named the BT-161, €756,837.00, but a lot candidate is larger. Right scale. |
| 867491 | €1,701,000,000.00 | **€491,400,000.00 (LOT-0001's BT-27-Lot)** | Wrong figure. That estimate is itself 1000 × its award and sits under the €1 bn gate, so no rule reaches it. Readers named €1,360,800. True total €1,701,000.00. |

Tally: 6 fall to none, 5 to a plausible procedure figure, 10 to one lot's figure at the right scale but understated, and 1 to a wrong figure. A lot-sum fallback would serve the readers' plausible total for 16 of the 22. The max election never serves a sum, so that is a separate decision (file it). It is not part of 537 unit 3.

**Test exhibits.**

Refuse:
- `a_total_ten_to_the_k_times_its_lot_award_sum_is_not_elected`:
  - 369057 → value null. Its awards match at −1000, within 3000; its BT-709 match is exact.
  - 165437 → 2020289170 (+600, the three-decimal shape).
- Tolerance at k = 2: 6921980 (+200, n = 15) and 801998 (−100, n = 14).
- Latest award per lot: 392980. A per-lot maximum (what `head_lot_sums` keeps) gives 3,760,912,819, and ×100 misses the head.
- Partner-only rows as S: 1024214. Its BT-271-Lot match fails T, so only BT-709 refuses it.
- Estimate side through T: 508666 (S from BT-271-Lot, X = BT-27-Procedure) and 114991 (S from BT-27-Lot, X = BT-161).

Keep:
- 8400892: awards k = 3, n = 1, also kept by 471.
- 8312215: est k = 4, n = 2, T: no. This is the "k€ lots keep the head" case.
- 8720567: est k = 2, FTS, T: no.
- 8596393 and 1039053: k = 1.
- 7955574: ratio 1.
- Regression controls not measured here: 492's nine genuine frameworks (8713680, 8423121, 8804016, 6572976, 8730895, 6904931, 7956096, 8800131, 8638612) plus 8595426 and 8618327, and 505's five (8784848, 553044, 5748163, 1120720, 915781).

**Recorded cost.** Arm (A) cannot keep a correct total over award lots published in thousands. That shape occurs 0 times in the 20 award-side refusals. A guard that would protect it also loses 369057, which has no in-chain third figure.

**Interactions:**
- **471 (single partner, k ≥ 3, corroboration exemption).** Requiring n ≥ 2 means a single-figure S stays 471's alone. All three k ≥ 3 rows with n = 1 already have a 471 partner, and 471's corroboration exemption keeps each of them: 8400892 is right, 317736 and 622872 are slips (BT-27 and BT-271 copies of one notice). That is recorded here and not changed by 537.
- **492 (×100 with framework exemption).** 492's window is a lot sum in [F/10, F]. 537 fires at S ≈ F/10ᵏ with k ≥ 2. The two windows are disjoint, and they read different sums (492 takes the per-lot maximum across fields; 537 takes per-kind sums). Precedence goes to 492's exemption, as stated in the rule. A 537-refused procedure figure no longer bounds 492's sibling-lot exemption, which errs toward refusing.
- **505.** Converse direction, a lot over its procedure total. It does not overlap this set. A refused P no longer serves as 505's admitted bound.
- **506.** BT-709 and BT-660 act as S only. 1024214 depends on that.
- **`IMPLAUSIBLE_EUR_CENTS` doc.** Its "needs the tender-versus-lot-sum ratio" sentence should name this rule.
- **471 4(c).** 537 narrows its "no signal" premise to Tenders without a lot figure. 355006 and 20905 stay caveat-only.

### 4. What is not measured

- **The < €1 bn band.** The rule sits behind the €1 bn gate. 867491's €491.4 m lot slip shows that slips of this kind exist below it.
- **The 1,668 heads at ≥ €1 bn with no lot figure in the head's currency.** No lot sum can be built for them.
- **The 2,034 heads that have lot figures but no 10ᵏ match.** Recall over slips of other shapes is unknown.
- **The ratio-1 population** (776 estimate sums, 550 award sums). Not adjudicated.
- **The guard T itself.** It was measured on 2 estimate-side refusals against 2 kept lots_wrong. For the guarded cells, T was read from the readers' notice reads. 4726892's T is "no" only because its CN range is reported unmapped. If its v1 TED-VALUE GLOBAL is stored, the k ≥ 1 guarded cells gain one right refusal.
- **Award rows that are not per-lot figures.** 1011786's rows are 46 × the framework total. 164283's contract total is repeated on every lot. 8452581 has a lot-null 10× row. These matched nothing here, and they show that an award-side S can be garbage.
- **Siblings in other Tenders.** These hold the deciding figure for 369057, 130304, 8717158, 812132 and 8596393, and no in-Tender rule can see them.
- **k = 1.** 15 head_wrong rows stay as published. With a `result_value` head, k = 1 is 9 / 0; with an `estimated_value` head it is 6 / 1 [8596393]. Not adopted: k = 1 is below every scale rule's floor, and its sums are all round.
- **The lot-sum fallback, and lot-null award copies elected as head.** 10 of the 22 refusals fall to one lot's figure. Neither is measured or built.
- **The drain.** Re-read section 16 and the Verify after `refold-value-band` → `project`. The expected result is that 22 heads leave the band, 369057 shows `None` and 165437 shows 2020289170.

Scratch files: /tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/s537/m.py computes the matrix; /tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/api537/ holds the 7 GETs.