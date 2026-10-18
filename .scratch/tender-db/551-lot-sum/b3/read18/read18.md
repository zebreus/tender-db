# 551 read18: M2-bar read of the 18 post-drain new ships

2026-10-18. Path W v2 is still off on the server, so every item is the CANDIDATE S from `b3/new_ships_18.json`. That is the B3 port's ship after the 567/568/570 lot-key drains; all 18 were refused by R2 before the drains. Protocol: a reader and an adversarial skeptic on every ship. A head counts as right only when both agree, or when a third reader decides.

## Result

- **18 of 18 right**: reader and skeptic agree on every head, all class `ok`. 0 neutral, 0 wrong, 0 disagreements, 0 third readers needed.
- **Band bar: PASS.** The 5 new band heads (S >= EUR 100 m) are 6569442, 6603588, 6679375, 7139761, 7542368: 5/5 right, 0 wrong of any class.
- **Other 13: 0 wrong.** M2's sample bar was <= 1 wrong of 30 on a uniform draw. These 13 are not a uniform draw (they are the ids the drains flipped), so they are not scored against that bar. With 0 wrongs they do not threaten B5's bars.
- **M2's 15 band heads are unaffected.** B3-movers.json shows drain_moved = remap_moved = false and the same S for all 15 (none moved), so their M2 reads (15/15) still stand. **The reference band set for B5 is therefore 20:** M2's 15 plus the 5 above.
- Both heads labelled double_count on their pre-drain sums, 5985466 and 6603588, read right on their post-drain sums. The drains removed the double count in each case.

## Verdicts

Columns: B = band head. S/T = S against the stated total T. S/truth and head/truth are measured against the whole procedure; truth differs from T only where noted.

| id | B | population | S | n | vouched_by | today's head | stated total T (source) | S/T | S/truth | head/truth | reader | skeptic | final |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 5862601 |  | walk-fallback | HUF 1,683,175,196.00 | 19 | TED-VAL_TOTAL | HUF 526,009,606.00 | HUF 1,683,175,196.00 (CAN 19576559 VAL_TOTAL) | +0.00% | +0.00% | -68.75% | right/ok | right/ok | **right** |
| 5888594 |  | walk | HUF 91,683,772.00 | 2 | TED-VAL_TOTAL | HUF 79,420,079.00 | HUF 91,683,772.00 (CAN 19383494 VAL_TOTAL) | +0.00% | +0.00% | -13.38% | right/ok | right/ok | **right** |
| 5985466 |  | labelled-mid | EUR 96,003,380.66 | 3 | TED-VAL_TOTAL | EUR 43,488,972.45 | EUR 96,003,380.00 (CAN 19749944 VAL_TOTAL) | +0.00% | +0.00% | -54.70% | right/ok | right/ok | **right** |
| 6317191 |  | walk-fallback | CHF 61,378,150.00 | 6 | TED-VAL_TOTAL | CHF 7,521,050.00 | CHF 61,678,075.00 (F03 20431961 VAL_TOTAL (truth = after lot-3 re-award 20515863)) | -0.49% | +0.00% | -87.75% | right/ok | right/ok | **right** |
| 6569442 | **B** | labelled-mid | CHF 244,681,010.00 | 29 | TED-VAL_TOTAL | CHF 15,400,000.00 | CHF 244,681,010.00 (F03 20743713 VAL_TOTAL) | +0.00% | +0.00% | -93.71% | right/ok | right/ok | **right** |
| 6595330 |  | labelled-sample | EUR 2,295,195.00 | 2 | TED-VAL_TOTAL | EUR 1,940,400.00 | EUR 2,295,195.00 (F03/F06 20747457 VAL_TOTAL) | +0.00% | +0.00% | -15.46% | right/ok | right/ok | **right** |
| 6603588 | **B** | labelled-mid | EUR 154,611,342.00 | 7 | TED-VAL_ESTIMATED_TOTAL | EUR 37,046,995.00 | EUR 154,611,342.00 (CN 20628343 VAL_ESTIMATED_TOTAL = F03 21740303 VAL_TOTAL) | +0.00% | +0.00% | -76.04% | right/ok | right/ok | **right** |
| 6679375 | **B** | labelled-mid | EUR 238,500,000.00 | 3 | TED-VAL_TOTAL | EUR 29,800,000.00 | EUR 238,500,000.00 (F03 20928012 VAL_TOTAL (lots 1,2,4); truth adds lot 3 from 20916402) | +0.00% | -11.11% | -88.89% | right/ok | right/ok | **right** |
| 7139761 | **B** | labelled-mid | EUR 166,731,929.69 | 13 | TED-VAL_ESTIMATED_TOTAL | EUR 39,917,220.26 | EUR 166,731,929.71 (CN 21841642 VAL_ESTIMATED_TOTAL) | -0.00% | -0.00% | -76.06% | right/ok | right/ok | **right** |
| 7153588 |  | walk-fallback | EUR 19,128.00 | 4 | TED-VAL_TOTAL | EUR 2,720.00 | EUR 19,128.00 (F03 22126132 VAL_TOTAL (lots 1,3,4,6); truth adds lots 5 and 2) | +0.00% | -15.54% | -87.99% | right/ok | right/ok | **right** |
| 7177672 |  | walk-fallback | HUF 315,979,996.00 | 4 | TED-VAL_TOTAL | HUF 135,574,700.00 | HUF 315,979,996.00 (F03 22174236 VAL_TOTAL) | +0.00% | +0.00% | -57.09% | right/ok | right/ok | **right** |
| 7298633 |  | walk | PLN 3,583,769.75 | 9 | TED-VAL_TOTAL | PLN 597.40 | PLN 3,583,172.35 (F03 22385634 VAL_TOTAL (8 lots); truth adds lot 4 from 22375511) | +0.02% | +0.00% | -99.98% | right/ok | right/ok | **right** |
| 7398935 |  | walk-fallback | EUR 14,446.00 | 2 | TED-VAL_TOTAL | EUR 11,133.00 | EUR 14,446.00 (CAN 22566107 VAL_TOTAL (lots 1,3); truth adds lot 2 from 22603453) | +0.00% | -43.52% | -56.48% | right/ok | right/ok | **right** |
| 7475504 |  | walk-fallback | BGN 35,531,694.00 | 4 | TED-VAL_ESTIMATED_TOTAL | BGN 12,114,130.00 | BGN 35,531,694.00 (CN 22586040 VAL_ESTIMATED_TOTAL) | +0.00% | +0.00% | -65.91% | right/ok | right/ok | **right** |
| 7542368 | **B** | labelled-mid | EUR 179,840,748.93 | 3 | TED-VAL_TOTAL | EUR 67,957,659.22 | EUR 179,840,748.93 (CAN 23357149 VAL_TOTAL) | +0.00% | +0.00% | -62.21% | right/ok | right/ok | **right** |
| 7722197 |  | walk-fallback | EUR 179,360.00 | 4 | TED-VAL_TOTAL | EUR 64,015.00 | EUR 179,360.00 (F03 23370588 VAL_TOTAL (lots 1,3,4,5); truth adds lot 2 from 23438408) | +0.00% | -26.30% | -73.70% | right/ok | right/ok | **right** |
| 7789396 |  | walk-fallback | BGN 28,400.60 | 6 | TED-VAL_ESTIMATED_TOTAL | BGN 11,320.00 | BGN 28,476.00 (CN 23334253 VAL_ESTIMATED_TOTAL (truth = sum of the two F03 VAL_TOTALs)) | -0.26% | +0.00% | -60.14% | right/ok | right/ok | **right** |
| 8371658 |  | walk-fallback | EUR 14,936,120.00 | 4 | TED-VAL_TOTAL | EUR 5,263,705.00 | EUR 14,936,120.00 (F03 20173160 VAL_TOTAL) | +0.00% | +0.00% | -64.76% | right/ok | right/ok | **right** |

## Band result

- **6569442**: right / ok (reader+skeptic agree). S = CHF 244,681,010.00 over n = 29; T = CHF 244,681,010.00; today's head CHF 15,400,000.00. 30 lot sections in string-sorted ITEM order; skeptic checked all 31 lot_results against printed LOT_NO: 0 mismatches.
- **6603588**: right / ok (reader+skeptic agree). S = EUR 154,611,342.00 over n = 7; T = EUR 154,611,342.00; today's head EUR 37,046,995.00. S is on the ceiling basis (CN VAL_OBJECT = F20 VAL_TOTAL = F03 procedure VAL_TOTAL). Skeptic corrected the reader: F03 21740303 discounted lot awards sum to 134,390,083.30 (not 117.2 m); pre-drain S was the LOT-1 mis-key. S +15% vs head -72% on the discounted basis. Labelled double_count on its pre-drain sum.
- **6679375**: right / ok (reader+skeptic agree). S = EUR 238,500,000.00 over n = 3; T = EUR 238,500,000.00; today's head EUR 29,800,000.00. 
- **7139761**: right / ok (reader+skeptic agree). S = EUR 166,731,929.69 over n = 13; T = EUR 166,731,929.71; today's head EUR 39,917,220.26. Multi-supplier acuerdo marco, but one ceiling per lot, never per supplier; buyer itself sums them (2 cents off).
- **7542368**: right / ok (reader+skeptic agree). S = EUR 179,840,748.93 over n = 3; T = EUR 179,840,748.93; today's head EUR 67,957,659.22. CN 22769993 lot values are ~10x low; buyer fixed them in corrigendum 22756883 (found by the skeptic, missed by the reader). S carries none of the slip.

Bar: 0 wrong of any class among the band heads. Found: 0 wrong. **PASS.**

## Wrongs and neutrals

None. No head drew a wrong or neutral from either reader, and no reader disagreed.

## Judge notes (not wrongs)

1. **Partial sums: 4 of 18, one of them a band head.** 6679375: leaves out 20916402 (lot 3, EUR 29,800,000); 7153588: leaves out 22109924 (lot 5, EUR 799.00), 22110418 (lot 2, EUR 2,720.00); 7398935: leaves out 22603453 (lot 2, EUR 11,133.00); 7722197: leaves out 23438408 (lot 2, EUR 64,015.00). In each case the left-out award is the only award on a single-Lot-section notice whose LotResult prints no LOT_NO, so it stays lot = null. S is still right on each: it equals a buyer-stated VAL_TOTAL exactly, its lots are distinct real awards, and it is much closer to the whole-procedure figure than today's head (6679375: S -11% against head -89%; 7153588: -16% against -88%; 7398935: -44% against -56%; 7722197: -26% against -74%). 7398935 is the weakest right of the 18: S covers 2 of 3 lots and today's head covers the third.
2. **Fragility for B5's band stop rule.** These 4 heads are vouched only because the summed lots exactly match one notice's coverage. A later drain that keys such LotResults by the sole Lot section's LOT_NO would make S the full total, for example 6679375 to about EUR 268.3 m. No stated total lies within R10's 1% of that full figure, so the rule would refuse. 6679375 would then leave the band and B5's "band set different from the reference" stop would trip. So no lot-key drain should land between this read and B5's band seek. If one does, re-run the B3 port diff and read any band change under this bar before B5. Separately, R10 could accept the sum of disjoint notice totals; that would serve the full figure in these cases. That is a possible follow-up, not filed by this read.
3. **Lot-null rows that only restate a keyed award** (they cannot move S): 5862601: 20229342 (LOT_NO 5.), 20563776 (LOT_NO 6.); 5888594: 20505389 (LOT_NO 2.); 7475504: 23595485 (LOT_NO 4), 23595634 (LOT_NO 2), 23592536 (LOT_NO 3). These are residue for the lot-key drains.
4. **Reader misses the skeptics caught.** On 7542368 the reader missed corrigendum 22756883, the buyer's own fix of the CN's ~10x-low lot values; the verdict is the same. On 6603588 the reader mis-stated the F03 discounted lot sum (134,390,083.30, not 117.2 m); the verdict is the same either way. Every skeptic re-fetched the notices and none overturned a verdict.
5. **Basis.** 6603588, 7139761, 5862601 and 5888594 sum per-lot single-operator (or per-lot) framework ceilings. Each ceiling belongs to one lot only, and the buyer's own procedure total is their sum, so no not_additive case arises.

## Recommendation

**Proceed to B4 with the band set of 20** (M2's 15 plus 6569442, 6603588, 6679375, 7139761 and 7542368), and record these 20 as B5's reference band set. B4's operations.md text should name 20 band heads. Freeze the lot-key drains (568/570 residue) until after B5's band seek, or re-run this check if one lands. B5's confirmation read is unchanged: a new salt of 30, plus any band lot-sum-awards row outside the 20.

## Files

- `read18.json`: the summary plus, for each head, the final verdict, the figures and both reads verbatim.
- `sk<id>/`, `skep<id>/`, `g1raw/`, `g2/`..`g6/`: the raw notice and tender reads the readers saved.
