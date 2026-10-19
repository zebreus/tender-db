# 551 B5 investigation read: the confirm 30, the 9 new band heads and 22 never-measured fires

2026-10-19, workflow wf_eecbe1b7-bc7. Path W v2 was switched off and drained back before this read (36ca974; 2628/2629/2630), so every item is the candidate S as served at about 02:45Z (`read_items.json`, `served/{id}.json`), judged against its notices. "Head without path W" is the max election of the head's figures, which is what the Tender serves now (`post_switchoff_rows.json`, live GETs). Protocol (M2): a reader and an adversarial skeptic on every head; the final verdict is theirs when they agree, otherwise a third reader decides. 61 heads, 122 reads. The judge rebuilt S from each served snapshot's `lot_results` (latest awarded row per lot key): it equals the served S on 61 of 61.

## Result

- **59 right, 1 neutral (7749755), 1 wrong (6465510, class other).** Reader and skeptic agree on all 61 heads, verdict and class. 0 third readers were needed.
- **(A) Pre-registered confirm bar, at most 1 wrong of the 30 confirm30: PASS, with no margin.** 28 right, 1 neutral (7749755), 1 wrong (6465510). Exact 95 % interval for the wrong rate at 1/30: 0.1%–17.2%.
- **(B) Band bar, 0 wrong of any class among the 9 band rows outside the reference 20: PASS.** 9 of 9 right, 0 neutral, 0 wrong.
- **(C) Investigation measure, not pre-registered.** Outside30 (the 8 confirm30 ids outside project 2575's population plus the outside22): 29 right, 0 neutral, 1 wrong (6465510). That is at the ≤ 1-of-30 yardstick with no margin (exact 95 % 0.1%–17.2%). The 22 inside-2575 confirm30 ids: 21 right, 1 neutral (7749755), 0 wrong.
- 6465510 is in both A and C (a confirm30 draw that lies outside the 2575 population), so the single wrong spends the whole margin of both measures.
- The outside22 and the band9 are clean: 31 of 31 right. The one wrong and the one neutral both come from the confirm30.

## Wrong and neutral

### 6465510 — wrong / other (confirm30, outside the 2575 population)

- **S** = EUR 162,020.92 over n = 2: LOT-2 EUR 91,680.50 (21020093, printed LOT_NO 2 '2η ομάδα', contract 35/2020 modifying 75/2019; the 2019 award 20767563 gives the same figure) + LOT-6 EUR 70,340.42 (21044287, printed LOT_NO 6, contract 13/2020 extending 69/2019). The skeptic found that contract 69/2019 (Spectratech) covers two lots, lot 6 (20772906, 67,002.20) and lot 20 (20767549, 3,338.22), so the LOT-6 figure is that two-lot contract's total; lot 20 is not summed again, so nothing is counted twice.
- **Vouched by** TED-VAL_TOTAL EUR 161,290.32 on 20772908. That notice is the single-lot award of printed lot 4 (contract 66/2019, Katzagiannakis, one tender, LOW = HIGH 160,690.00), and its VAL_TOTAL is exactly lot 4's VAL_OBJECT in CN 20341803. Lot 4 is not in S; the figure lands 0.45 % from S by coincidence.
- **The procedure:** CN 20341803 (F02, 22 lots) states VAL_ESTIMATED_TOTAL EUR 967,741.94. Twenty lots were awarded, each in its own single-lot award notice; their awards add to about EUR 860k (skeptic 859,832.58, reader 863,051.74). No notice states an award total for the whole procedure.
- **Why it is wrong:** S covers 3 of 20 awarded lots (about 18.8 % of the money; the head without W, EUR 70,340.42, is about 8.2 %). S is nominally closer, but the rubric's explicit wrong criterion applies: the vouching total belongs to something else (lot 4's own figure). No total of this procedure vouches S.
- **Why R1–R10 let it through:** in the served snapshot, 19 of the 20 award notices of 2019 have their results stored with lot = null and awarded = null (the 20th, 20767555, is keyed LOT-1 but also carries no award), so their values never enter LotSums; LOT-2 and LOT-6 are summed only through the 2020 modification notices. R10 excludes 'a single-lot notice's own award' by comparing T with the stored award, which is null for 20772908, so the exclusion cannot see that T is that notice's own lot figure.

### 7749755 — neutral / other (confirm30, inside the 2575 population)

- **S** = EUR 4,124.60 over n = 3, all from award notice 23353167: LOT_NO 1 'GAISMIŅA' 990.00 (NV Stils), LOT_NO 6 'ZĪĻUKS' 1,452.00 and LOT_NO 9 'KAMOLĪTIS' 1,682.60 (RG Group). Three distinct real lots; no double count, slip or VAT mix.
- **Vouched by** 23353167's own TED-VAL_TOTAL EUR 4,124.60. That is the total of the same three awards: R10 holds by arithmetic identity, not because a total for the procedure matches.
- **Left out:** lot 2 (23429800, Lazurīts S, EUR 3,229.40) and lot 5 (23354748, RG Group, EUR 920.00). Each was awarded in a single-lot F03 whose LotResult prints no LOT_NO, so both sit on lot = null rows. Lots 3, 4, 7 and 8 were discontinued (23444244, 23279468). CN 23230760 states no value.
- **Why neutral:** the true awarded total is EUR 8,274.00 (the sum of the three disjoint notice totals). S covers 49.85 % of it and the head without W (lot 2's EUR 3,229.40) covers 39.0 %. S is not farther, and its vouch is a real buyer-stated total of these very awards, so it is not wrong; but it is not clearly better, and the 'lots: 3, vouched' label overstates it as a procedure total. Both readers flagged a possible buyer-side scale entry (EUR 8,274 for 9 furniture lots on TED); it would move S and the head alike.

## What the wrong and the neutral have in common

Both leave awarded lots outside S on results the store did not key to a lot (lot = null), and both are vouched by one notice's TED-VAL_TOTAL whose coverage R10 never compares with the summed lots. In 7749755 that notice covers exactly the summed lots (a vouch by identity); in 6465510 it covers a different lot altogether (a vouch by coincidence). Three rights share the first half of the shape (the read18 note-2 partial sum): they are right only because the head without W holds the single left-out award.

| id | final | S / truth | head without W / truth | left out of S (lot = null) | what the vouching notice awards |
|---|---|---|---|---|---|
| 6465510 | **wrong** | ~18.8% | ~8.2% | 17 single-lot award notices, results stored lot = null and awarded = null | lot 4 only (not in S) |
| 7749755 | **neutral** | 49.9% | 39.0% | lot 2 (23429800, 3,229.40), lot 5 (23354748, 920.00) | lots 1, 6, 9 = exactly S |
| 6779279 | right | 95.4% | 4.6% | lot 8 (21279524, 496.40) | lots 1, 4, 6, 7 = exactly S |
| 6850401 | right | 90.1% | 9.9% | lot 4 (21559314, 130,504.00) | lots 1, 2, 5 = exactly S |
| 7698260 | right | 81.2% | 18.8% | lot 3 relaunch (23343796, 123,300.00) | lots 1, 2 = exactly S |

**Guard-shaped patterns R1–R10 miss** (measured on the 61 served snapshots; `served_signals` in read.json):

1. **G-cover: an award-type vouching total must come from a notice that awards at least one summed key.** Award-type here means TED-VAL_TOTAL, BT-161, BT-118, BT-1118 and TED-LOW/HIGH; a CN estimate is not an award and is left to the existing rules. On the 61 it refuses **exactly 6465510 and no right**: 20772908 awards neither LOT-2 nor LOT-6, and every other award-type vouch comes from a notice that awards summed keys. An equivalent fix: judge R10's 'single-lot notice's own award' exclusion by the notice's own structure (one Lot section, one LotResult), not by whether its award row was stored with a value. The proxy above reads the served `lot_results`; the Rust `stated_totals.lots_awarded` must be checked to give the same answer before anyone relies on it.
2. **Unplaced awards (a measurement, not proposed as a refusal).** A lot = null result whose award restates no summed award marks exactly the partial-sum family: 6779279, 6850401, 7698260 and 7749755, plus 6465510 by its 19 lot = null winner rows without a value. Refusing on it would cost 3 rights to remove 1 neutral (6465510 is already caught by G-cover). The fix for this family is placement plus a broader vouch: key a single-Lot-section notice's LotResult by its sole Lot section's LOT_NO (the 567/570 follow-on), and let R10 accept the sum of disjoint notice totals (read18 note 2). With both, all four become full sums vouched by their own notices: 8,274.00, 10,857.40, 1,313,410.00 and 656,900.00.
3. Neither pattern is sized beyond these 61. The share of the 3,502 fires whose award-type vouching notice awards no summed key can be measured with the B3 port on public-API facts, by the same bounded-read route as M1.

## Judge notes (rights with caveats; none changes a verdict)

1. **Partial sums that read right:** 6779279, 6850401 and 7698260 (table above). They are fragile in the way read18 note 2 describes. A lot-key drain that places their lot = null rows makes S the full figure, which no single stated total vouches within 1 % unless R10 accepts disjoint notice totals.
2. **6908248 depends on lot-null rows.** 22266591's RES-2..RES-10 print LOT_NO '2'..'10' as supplier ordinals and each repeats lot 1's EUR 840,000 ceiling. They are stored lot = null today. If a lot-key drain keyed RES-2 to LOT-2, LOT-2's latest award would become 840,000 and S EUR 1.68 m, a double count. R3 should then refuse it (22266591's own VAL_TOTAL of 840,000 would sit below the 1.68 m it awards). Worth a pin before any such drain.
3. **S differs from its vouching total but stays within 1 %:** 7561428 +0.82 % and 7319895 +0.66 % (each vouching total predates one later award: lot 4's relaunch, lot 21), 5795986 +0.44 % (LOT-1's V.2.4 figure is the post-2022-modification value), 6736263 +0.33 % (the CAN restates the VEAT's round 345,000), 7214560 −0.41 % (unexplained), 7256152 +0.00002 % (rounding). Of the in-band gaps, only 6465510's 0.45 % is a vouch from the wrong object.
4. **As-awarded against modified.** Many heads carry F20 modifications (for example 6435162, 8407303, 6548237, 7046008, 6937980, 6736263). On every head S is the as-awarded sum, except that 5795986's LOT-1 picks up a modified V.2.4 figure. Where a head was modified, the reads also checked that S is closer than the head without W under the post-modification total as well.
5. **Ceilings are counted once per lot.** On the multi-supplier and cascade frameworks (6435162, 6718092, 8407303, 5795986, 6908248), each lot's ceiling is repeated on every parallel contract in that lot, and path W takes it once per lot; the buyer's own procedure total confirms the sum. The non-additive repeats, BT-709 carrying the whole notice total on each lot (609878, 766623), are partner-only rows and are not summed.
6. **Facts the skeptics added (no verdict overturned):** 6465510 (LOT-6 is the two-lot contract 69/2019); 6811355 (each F20's VAL_TOTAL_BEFORE carries the 4,763,512.46 base); 5795986 (lots 2 and 3 were modified too); 6736263 (Covid extension, post-modification total 432,662.48); 6937980 (the F20s absorb the discontinued lot 4's 1 M masks, giving 3,773,000); 7266931 (the F20 is a 12-month renewal rate, not a raise); 8479463 (contract numbers are not all distinct; a MAIN_SITE copy slip); 8460630 (the award/estimate ratio is not a steady 2/3 on lots 12–15).

## Verdicts

Pop = population tag. 2575 = in project 2575's population. head/S = the head without path W over S, both in EUR (post-switch-off row against the served S_eur).

| id | pop | 2575 | S | n | vouched_by | head without W | head/S | reader | skeptic | final |
|---|---|---|---|---|---|---|---|---|---|---|
| 7456417 | confirm30 | no | EUR 729,337.00 | 5 | TED-VAL_TOTAL 22933392 | EUR 70,000.00 | 9.6% | right/ok | right/ok | **right** |
| 609878 | confirm30 | yes | EUR 21,901,314.96 | 4 | BT-118-NoticeResult 25659799 | null | - | right/ok | right/ok | **right** |
| 6779279 | confirm30 | yes | EUR 10,361.00 | 4 | TED-VAL_TOTAL 21253579 | EUR 496.40 | 4.8% | right/ok | right/ok | **right** |
| 6465510 | confirm30 | no | EUR 162,020.92 | 2 | TED-VAL_TOTAL 20772908 | EUR 70,340.42 | 43.4% | wrong/other | wrong/other | **wrong** / other |
| 6811355 | confirm30 | yes | EUR 4,763,512.46 | 2 | TED-VAL_TOTAL 21225827 | EUR 3,404,601.06 | 71.5% | right/ok | right/ok | **right** |
| 7319895 | confirm30 | yes | PLN 5,816,958.21 | 40 | TED-VAL_TOTAL 22580039 | PLN 37,960.00 | 0.7% | right/ok | right/ok | **right** |
| 5948260 | confirm30 | no | GBP 1,133,880.71 | 2 | TED-VAL_TOTAL 19682959 | GBP 708,283.50 | 62.5% | right/ok | right/ok | **right** |
| 482122 | confirm30 | yes | EUR 640,000.00 | 2 | BT-1118-NoticeResult 24521069 | null | - | right/ok | right/ok | **right** |
| 193717 | confirm30 | yes | EUR 392,000.00 | 3 | BT-118-NoticeResult 24264397 | null | - | right/ok | right/ok | **right** |
| 6420294 | confirm30 | yes | EUR 32,053,000.00 | 2 | TED-VAL_TOTAL 20784598 | EUR 12,920,000.00 | 40.3% | right/ok | right/ok | **right** |
| 766623 | confirm30 | yes | EUR 37,048,841.36 | 2 | BT-118-NoticeResult 27863637 | null | - | right/ok | right/ok | **right** |
| 6979655 | confirm30 | no | HUF 71,663,914.00 | 3 | TED-VAL_TOTAL 21639328 | HUF 23,919,275 | 33.4% | right/ok | right/ok | **right** |
| 7561428 | confirm30 | yes | EUR 2,146,039.08 | 4 | TED-VAL_TOTAL 23116675 | EUR 17,414.00 | 0.8% | right/ok | right/ok | **right** |
| 6850557 | confirm30 | yes | HUF 328,154,560.00 | 2 | TED-VAL_TOTAL 21539204 | HUF 140,584,560.00 | 42.8% | right/ok | right/ok | **right** |
| 6850401 | confirm30 | yes | EUR 1,182,906.00 | 3 | TED-VAL_TOTAL 21613977 | EUR 130,504.00 | 11.0% | right/ok | right/ok | **right** |
| 7157983 | confirm30 | yes | EUR 544,000.00 | 2 | TED-VAL_TOTAL 22119193 | EUR 440,000.00 | 80.9% | right/ok | right/ok | **right** |
| 305060 | confirm30 | yes | EUR 297,520.00 | 2 | BT-118-NoticeResult 23859386 | null | - | right/ok | right/ok | **right** |
| 5936963 | confirm30 | yes | EUR 196,510.00 | 2 | TED-VAL_TOTAL 19299125 | null | - | right/ok | right/ok | **right** |
| 6647432 | confirm30 | yes | HUF 479,114,000.00 | 2 | TED-VAL_TOTAL 21086519 | HUF 315,114,000.00 | 65.8% | right/ok | right/ok | **right** |
| 7620308 | confirm30 | no | BGN 70,000.00 | 2 | TED-VAL_ESTIMATED_TOTAL 22957072 | BGN 40,000.00 | 57.1% | right/ok | right/ok | **right** |
| 7749755 | confirm30 | yes | EUR 4,124.60 | 3 | TED-VAL_TOTAL 23353167 | EUR 3,229.40 | 78.3% | neutral/other | neutral/other | **neutral** / other |
| 7036030 | confirm30 | yes | EUR 323,893.36 | 2 | TED-VAL_ESTIMATED_TOTAL 21590983 | EUR 70,158.08 | 21.7% | right/ok | right/ok | **right** |
| 7214560 | confirm30 | no | PLN 2,378,000.00 | 3 | TED-VAL_TOTAL 22183927 | PLN 1,394,000.00 | 58.6% | right/ok | right/ok | **right** |
| 42392 | confirm30 | yes | EUR 321,949.70 | 8 | BT-118-NoticeResult 24611909 | null | - | right/ok | right/ok | **right** |
| 7698260 | confirm30 | yes | EUR 533,600.00 | 2 | TED-VAL_TOTAL 23343875 | EUR 123,300.00 | 23.1% | right/ok | right/ok | **right** |
| 7402471 | confirm30 | yes | EUR 677,900.00 | 2 | TED-LOW 22597758 | EUR 420,600.00 | 62.0% | right/ok | right/ok | **right** |
| 5883834 | confirm30 | yes | EUR 2,539,000.00 | 2 | TED-VAL_ESTIMATED_TOTAL 19198403 | EUR 1,335,000.00 | 52.6% | right/ok | right/ok | **right** |
| 6658447 | confirm30 | no | BGN 12,500.00 | 2 | TED-VAL_ESTIMATED_TOTAL 20735584 | BGN 10,000.00 | 80.0% | right/ok | right/ok | **right** |
| 7046008 | confirm30 | no | HUF 521,500,000.00 | 6 | TED-VAL_TOTAL 21840821 | HUF 26,500,000 | 5.1% | right/ok | right/ok | **right** |
| 6817321 | confirm30 | yes | HUF 400,000,000.00 | 2 | TED-VAL_ESTIMATED_TOTAL 21089390 | HUF 220,000,000 | 55.0% | right/ok | right/ok | **right** |
| 5769515 | band9 | no | EUR 131,674,590.00 | 4 | TED-VAL_ESTIMATED_TOTAL 18974369 | EUR 61,235,420.00 | 46.5% | right/ok | right/ok | **right** |
| 6435162 | band9 | no | EUR 108,666,493.00 | 5 | TED-VAL_TOTAL 20680957 | EUR 21,850,924.00 | 20.1% | right/ok | right/ok | **right** |
| 6548237 | band9 | no | HUF 82,500,000,000.00 | 15 | TED-VAL_ESTIMATED_TOTAL 20508422 | HUF 14,000,000,000.00 | 17.0% | right/ok | right/ok | **right** |
| 6718092 | band9 | no | EUR 990,000,000.00 | 4 | TED-VAL_ESTIMATED_TOTAL 20884376 | EUR 180,000,000.00 | 18.2% | right/ok | right/ok | **right** |
| 7167278 | band9 | no | HUF 344,998,891,444.00 | 2 | TED-VAL_TOTAL 22108220 | HUF 220,000,000,002.00 | 63.8% | right/ok | right/ok | **right** |
| 7196993 | band9 | no | HUF 43,410,887,886.00 | 2 | TED-VAL_TOTAL 22331647 | HUF 29,211,998,888.00 | 67.3% | right/ok | right/ok | **right** |
| 7461921 | band9 | no | EUR 122,610,000.00 | 2 | TED-VAL_ESTIMATED_TOTAL 22581643 | EUR 11,910,000.00 | 9.7% | right/ok | right/ok | **right** |
| 8407303 | band9 | no | EUR 1,134,251,967.00 | 4 | TED-VAL_TOTAL 19931200 | EUR 125,111,580.00 | 11.0% | right/ok | right/ok | **right** |
| 8460630 | band9 | no | EUR 404,568,741.62 | 21 | TED-VAL_TOTAL 21675186 | EUR 66,221,677.29 | 16.4% | right/ok | right/ok | **right** |
| 5795986 | outside22 | no | EUR 50,221,600.00 | 3 | TED-VAL_ESTIMATED_TOTAL 19000574 | EUR 24,500,000.00 | 48.8% | right/ok | right/ok | **right** |
| 6736263 | outside22 | no | GBP 346,131.77 | 3 | TED-VAL_TOTAL 20876879 | GBP 107,913.98 | 31.2% | right/ok | right/ok | **right** |
| 6034467 | outside22 | no | BGN 261,575.00 | 2 | TED-VAL_ESTIMATED_TOTAL 19501229 | BGN 221,000.00 | 84.5% | right/ok | right/ok | **right** |
| 7007140 | outside22 | no | HUF 145,526,590.00 | 19 | TED-VAL_TOTAL 22746439 | HUF 9,415,000.00 | 6.5% | right/ok | right/ok | **right** |
| 7259365 | outside22 | no | HUF 90,000,000.00 | 2 | TED-VAL_TOTAL 22266703 | HUF 70,000,000.00 | 77.8% | right/ok | right/ok | **right** |
| 6029648 | outside22 | no | EUR 1,622,000.00 | 3 | TED-VAL_ESTIMATED_TOTAL 19478189 | EUR 116,000.00 | 7.1% | right/ok | right/ok | **right** |
| 7256152 | outside22 | no | EUR 1,468,683.34 | 9 | TED-VAL_TOTAL 22083308 | EUR 197,254.95 | 13.4% | right/ok | right/ok | **right** |
| 7407838 | outside22 | no | BGN 490,154.00 | 2 | TED-VAL_ESTIMATED_TOTAL 22458240 | BGN 428,218.00 | 87.4% | right/ok | right/ok | **right** |
| 6908248 | outside22 | no | EUR 1,020,000.00 | 2 | TED-VAL_ESTIMATED_TOTAL 21299997 | EUR 840,000.00 | 82.3% | right/ok | right/ok | **right** |
| 6748628 | outside22 | no | HUF 1,004,578,214.00 | 22 | TED-VAL_TOTAL 21438723 | HUF 16,450,350.00 | 1.6% | right/ok | right/ok | **right** |
| 7210026 | outside22 | no | HUF 69,214,920.00 | 5 | TED-VAL_TOTAL 22480294 | HUF 6,512,800.00 | 9.4% | right/ok | right/ok | **right** |
| 7266931 | outside22 | no | HUF 93,465,720.00 | 11 | TED-VAL_TOTAL 22315515 | HUF 26,546,600.00 | 28.4% | right/ok | right/ok | **right** |
| 7128709 | outside22 | no | HUF 125,100,000.00 | 3 | TED-VAL_TOTAL 22151972 | HUF 56,900,000.00 | 45.5% | right/ok | right/ok | **right** |
| 7143232 | outside22 | no | BGN 400,000.00 | 2 | TED-VAL_ESTIMATED_TOTAL 21821950 | BGN 100,000.00 | 25.0% | right/ok | right/ok | **right** |
| 5689244 | outside22 | no | BGN 451,817.79 | 8 | TED-VAL_TOTAL 18951101 | BGN 124,422.75 | 27.5% | right/ok | right/ok | **right** |
| 6969482 | outside22 | no | EUR 350,000.00 | 2 | TED-VAL_TOTAL 21768241 | EUR 300,000.00 | 85.7% | right/ok | right/ok | **right** |
| 8479463 | outside22 | no | EUR 620,064.15 | 3 | TED-VAL_TOTAL 21920282 | EUR 233,071.38 | 37.6% | right/ok | right/ok | **right** |
| 6260726 | outside22 | no | HUF 285,040,208.00 | 2 | TED-VAL_TOTAL 20624483 | HUF 205,193,216.00 | 72.0% | right/ok | right/ok | **right** |
| 6937980 | outside22 | no | EUR 3,260,000.00 | 3 | TED-VAL_TOTAL 21517498 | EUR 780,000.00 | 23.9% | right/ok | right/ok | **right** |
| 6993571 | outside22 | no | BGN 591,400.00 | 2 | TED-VAL_ESTIMATED_TOTAL 21506205 | BGN 324,400.00 | 54.9% | right/ok | right/ok | **right** |
| 7190148 | outside22 | no | HUF 79,700,000.00 | 2 | TED-VAL_TOTAL 22090031 | HUF 57,600,000.00 | 72.3% | right/ok | right/ok | **right** |
| 6267826 | outside22 | no | BGN 116,666.67 | 2 | TED-VAL_ESTIMATED_TOTAL 19970041 | BGN 91,666.67 | 78.6% | right/ok | right/ok | **right** |

## Files

- `read.json`: `summary` (tallies A/B/C, wrong and neutral, pattern and guard signals), `heads` (per id: final verdict, class, decided_by, `served_signals`, and the full reader, skeptic and third reads), `groups_raw` (the panel output as returned).
- Raw GETs by the readers and skeptics sit beside this file in `read/` (rdr_*, sk*, skp_*, b5r3_*, g3_*).
