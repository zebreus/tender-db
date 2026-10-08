# 491 — a text-era minimum/maximum range is read as one run-together figure

Status: ready-for-agent — UNIT 2 DEPLOYED + MOSTLY DRAINED 2026-10-08 (`f71802e`, then `ab879bc`; see "Drain (2026-10-08)"). The 12-fetch reparse (jobs 2046–2057, all with 0 unmatched, re-keyed or failing) and project 2058 cleared 22 of the 25 notices. All 5 exhibits now serve `value: null`. The other 3 restate the pair UNSPACED in V.4, so the rule now tests the number rather than its groups (`ab879bc`). NEXT: reparse 2059–2061 (fetches 223, 229, 232) and project 2062 are queued. Then confirm 0 run-together `VAL_TOTAL` rows on the 25 notices, re-run data-quality, and re-read section 16 (expect the 19 text rows out).
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

## Unit 1 — measurement (2026-10-08)

Read off the parse layer with `491-values/shape-window.sh <from> <to>`. It is a bounded window over
`notice_amounts ⋈ notices` (`profile = 'text'`, prefix range on the PK, about 5 s per 100k notice ids)
that counts amounts whose units split as X = units / 10⁶, Y = units mod 10⁶ with Y ≥ 100,000 and
X < Y ≤ 4X, by currency.

- **Where.** The 13 windows covering notice ids 2,400,000–3,699,999 (2005–2008, about 260k text-era
  amounts) hold 29 hits: 26 EUR and 3 HUF (`491-values/windows-2400000-3699999-2026-10-08.txt`). Sampled
  windows at 1.5M, 2.0M, 2.2M, 3.7M and 3.9M hold 0. The 1.0M window (1997) holds 2 ITL hits. Every hit
  is a `PROCEDURE` `TED-VAL_TOTAL`, the V.4 value.
- **Which.** Each hit is listed in `491-values/hits-2026-10-08.tsv`. 25 of the 26 EUR hits are round
  (X and Y both whole thousands, mostly whole ten-thousands) or exactly Y = 4X (62709/250836,
  46250/185000, 87250/349000), which is the CMP cap. That is the French bons-de-commande print. The 5
  non-EUR hits are non-round, e.g. HUF 266,854,708,381 (≈ €1.06 bn) and ITL 76,995,264,650 (≈ €40 m).
  They read like ordinary published figures, not min/max pairs. EUR 247,110,402,110 (X = 247110,
  Y = 402110) is non-round too. It sits above the €100 bn ceiling, so it is never elected, and it is
  left alone.
- **Gate.** X < Y ≤ 4X, AND (Y = 4X, OR X ≡ Y ≡ 0 mod 1000), AND Y's first group is not 0-led (a
  min/max print has no leading zero). It matches all 19 text band rows, 25 of the 26 EUR hits (all but
  the 247 bn one), and none of the HUF or ITL hits. Every match is ≥ 25,000,100,000 units, so the rule
  cannot touch an ordinary-sized figure. 6 of the 25 (≥ €100 bn) are already above the ceiling and
  never elected; refusing them still removes them from `amounts`.
- **Blast radius.** About 25 notices corpus-wide. Expected effect: the 19 band rows (and the ≥ €25 bn
  EUR rows outside section 16's head list) lose the run-together figure, and each head falls to its
  next figure or to none.


**Whole-profile completion (same day).** The remaining windows were scanned the same way (`491-values/windows-rest-2026-10-08.txt`):
0–2.4M, 3.7M–4.4M, and 27.0M–28.1M, which together with the windows above cover all 3,786,955 text notices. They add only 9 hits, all
lira: LIT ×5 (1993–1996) and ITL ×4 (1997–2000). All are non-round (e.g. LIT 57,157,228,327) and **none matches the tight gate**.
So the gate's corpus-wide match set is exactly the 25 EUR notices in the 2005–2008 windows (12 fetches: 214, 223, 226, 229, 232,
235, 239, 240, 241, 245, 246, 248).

**Drain sizing.** Those 12 packages hold about 310k notices, under `reparse`'s 500k full-fallback line. So a
`reparse text` of just those packages (each `packages:1, after:<fetch−1>, reclaim_only:true`) and then one `project`
stays incremental. Unlike issue 484, this does not need a from-the-floor run, because no other package carries the
shape.

## Unit 2 — landed (2026-10-08)

`crates/ingest/src/text/parse.rs`: `parse_money` keeps its digit groups as written, and the new
`run_together_range(groups, fraction)` refuses the value as a range under the unit-1 gate. The gate is
X = the groups before the last two, Y = the last two, Y not 0-led, a zero or absent fraction,
X < Y ≤ 4X, and (Y = 4X or X ≡ Y ≡ 0 mod 1000). The test
`a_minimum_and_maximum_run_together_are_refused_as_a_range` pins three prod shapes as refused
(60 000 220 000, 87 250 349 000, 62 709 250 836). It pins as still claimed: the non-round
LIT/HUF figures, round figures (Y = 0), Y > 4X, a 0-led Y, a plain two-group figure, and a non-zero
fraction. Gate on the tree: `GATE-EXIT=0`, 145 suites.

**Before** (served `value` on 2026-10-08, from `/v1/tenders/{id}`). Expect each to fall to its next
admitted figure or to none:

| Tender | value now |
|---|---|
| 3450659 | 87,250,349,000.00 EUR |
| 3916175 | 60,000,220,000.00 EUR |
| 3303961 | 62,709,250,836.00 EUR |
| 3420171 | 50,000,110,000.00 EUR |
| 3365047 | 25,000,100,000.00 EUR |

**Drain.** Fetches 214, 223, 226, 229, 232, 235, 239, 240, 241, 245, 246 and 248 each need
`{"kind":"reparse","profiles":["text"],"packages":1,"after":<fetch−1>,"reclaim_only":true}`, then ONE
`project`. `reparse` stamps every text Tender epoch-stale by profile, so expect the project to rewrite
the text era's Tenders (incremental path, under the 500k-notice line).

## Drain (2026-10-08)

- **Deploy `f71802e`** (unit 2, group-based rule). **Probe** reparse 2955 (`text after 240`, 1 package): 25,624
  notices, 0 unmatched, 0 re-keyed, 0 now failing. Fetch 241's 8 target notices lost their `TED-VAL_TOTAL`.
- **The other 11 fetches**, jobs 2047–2057, one package each: about 275k notices, all with 0 unmatched, re-keyed
  or failing. **Project 2058**: 283,134 notices → 298,411 Tenders written, incremental path, 2,161 s.
- **Exhibits after 2058.** 3450659, 3916175, 3303961, 3420171 and 3365047 all serve `value: null`. The
  run-together figure was each one's only figure, so nothing else is elected. That is correct: the notice
  states a range, not a total.
- **Residual: 3 of the 25 kept the figure.** 3006685 (26854-2007), 3093522 (113691-2007) and 3238462
  (258631-2007). Their body prints the pair spaced at II.2.1 (`Valeur: 40 000 120 000 EUR`) and UNSPACED at
  V.4 (`Value: 40000120000 EUR.`). Once the aggregate was refused, the parser claimed the single-token V.4.
  The rule now tests the integer (X = units / 10⁶, Y = units mod 10⁶, Y ≥ 100,000) instead of its groups
  (`ab879bc`, gate green, deployed). Unit 1's measurement already read the number, spaced or not, so the
  measured match set is unchanged at the same 25 notices. Reparse 2059–2061 and project 2062 follow.

