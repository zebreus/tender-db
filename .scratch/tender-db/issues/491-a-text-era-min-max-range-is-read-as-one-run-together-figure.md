# 491 — a text-era minimum/maximum range is read as one run-together figure

Status: ready-for-agent — filed 2026-10-08 from issue 471 unit 5. In 471's stratified sample all 8
text-era rows in the €10 bn band were errors, and 5 of the 8 have this mechanism. NEXT: unit 1,
measure the shape corpus-wide by currency (a window read, not a full scan) before changing
`parse_money`.
Kind: data quality / text-era parser (`crates/ingest/src/text/parse.rs`)
Relates to: 471 (the band; unit 5 decision), 489 (F14 value corrections, the structured-era version of
(c)), ADR-0004

## What is wrong

### (a) A bons-de-commande minimum and maximum, run together

French award notices from 2005–2008 print a purchase-order framework's minimum and maximum as two
space-grouped numbers in one V.4 value:

```text
Value: 60 000 220 000 EUR          CAN 199890-2008, Ville du Robert (Martinique), ready-mix concrete
Value: 87 250 349 000 EUR          CAN 128539-2006, Conseil général de Corse-du-Sud, road materials
Value: 5 500 22 000. EUR           CAN 126383-2006, the same procedure's other award
```

French procurement law (CMP 2001/2004, bons de commande) caps the maximum at 4 × the minimum.
The first two read as 60 000 / 220 000 (ratio 3.67) and 87 250 / 349 000 (ratio 4.0).

`parse_money` requires every group after the first to be exactly three digits. `60 000 220 000`
satisfies that, so it is claimed as €60,000,220,000. `5 500 22 000` has a two-digit group and is
correctly refused. So the defect reaches only ranges whose maximum's leading group has three digits.
With Y ≤ 4X and a six-digit Y, X is at least 25,000, so every affected figure is at least
25,000,100,000 currency units. In EUR, GBP or FRF that is €3.8 bn and up. **Every case lands in or
near the band**, which is why it dominates the text stratum there.

Measured on section 16 of data-quality job 2954 (`471-values/section16-dq2954-2026-10-08.txt`):
**19 of the 27 text-era band rows** split at a group boundary into X < Y ≤ 4X:

3450659, 3664738, 3377764, 3303961, 3757597, 3916175, 3362364, 3424626, 3420171, 3664789, 3495825,
3432959, 3437324, 3482629, 3449983, 3391114, 3428706, 3365047, 3722841.

The other 8 do not split that way: 4179951 is (c) below, 3427333 is (b), 8161499 is a lone ×1000 typo,
and 3864512, 8083067, 8212935, 2694586 and 3551819 were not adjudicated. Same-buyer clusters show it is
systematic: OPAM (Nice) has seven awards of this shape.

### (b) V.4's figure is not stored when II.2.1 is present

CAN 143891-2008 (3427333, Cyprus Ministry of the Interior, waste plant):

- II.2.1 prints `Value: 35 076 200 000 EUR`.
- The same notice's V.4 prints `Initial estimated total value … 35 076 200 EUR` and
  `Total final value … 35 076 200 EUR`.

That is an exact 10³ partner, and issue 471's rule would refuse the big figure. But only the II.2.1
`result_value` is stored, so the rule never sees the partner.

### (c) A free-text corrigendum is not applied

4179951: the head notice 91132-2010 is a corrigendum. Its text reads "Instead of: … Total final value
of the contract: 15 000 000 000 GBP. Read: … 150 000 000 GBP." We still serve £15 bn. This is the
text-era form of issue 489, which maps the structured F14 `OLD_VALUE`/`NEW_VALUE` blocks.

## Direction

(a) is the high-value unit. The likely fix follows `parse_money`'s existing rule ("a wrong amount is
worse than a missing one", and `Minimum/maximum: …` is already "refused — a range"): refuse a value
whose tokens split into X then a two-group Y with X < Y ≤ 4X. Precision has to be measured first,
because a currency with small units (ITL, HUF, ESP, GRD, PTE, BEF) can carry a genuine
11-digit figure. A round genuine figure (`30 000 000 000`) cannot split, since Y = 000000 fails, but a
semi-round one might.

## Units

1. **Measure.** Count text-era `result_value` / `estimated_value` amounts of this token shape, by
   currency and buyer country, on a bounded window. Adjudicate a sample of the non-EUR/GBP/FRF hits.
   Decide the gate: X < Y ≤ 4X alone, or also a currency or country condition.
2. **Build (a)** in `parse_money` with tests (the three shapes above, plus a round and a semi-round
   genuine figure in ITL). Then re-parse the text era (the reparse-and-fold runbook) and re-read
   section 16. Expect the 19 rows out of the band.
3. **(b)** Read why V.4 is dropped when II.2.1 is present, and decide whether V.4's figure is stored
   as an amount. That would feed 471's exact-10ᵏ rule.
4. **(c)** Measure how often text-era corrigenda carry an "Instead of / Read" value pair before
   deciding whether to map them (issue 489's coordinate approach does not transfer as is: the text
   era has no structured blocks).
