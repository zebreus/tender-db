# 551 — when the election refuses a head, it falls to one lot's figure (or to null), never to the sum of the lots

Status: ready-for-agent — unit 2 DECIDED 2026-10-16 (panel wf_3d72c491-993: 88 dated labels, designs A/B, adversarial judge). Adopted: path V (the lot sum a scale refusal vouched for) plus path W (an unvouched AWARDS-only lot sum under strict guards), a derivation marker on `tenders`, never a synthetic amount row. Next: unit 3 build (schema columns, election, API field, docs), then a drain by id cohort, then the post-drain bounded read.
unit1-synthesis-2026-10-16.md`, "What each refused head falls to"). Next: measure how often an elected head is a
single lot's figure while the Tender has two or more lot figures in the same currency, then decide whether the
election may serve a lot sum.
Kind: data quality (amount election: `head_value_eur_cents_with`)
Relates to:
- 537 (decided 2026-10-16): its scale rule refuses 22 heads ≥ €1 bn. 6 fall to null, and 10 fall to ONE lot's
  figure at the right scale, 30–98 % of the true total.
- 471, 492, 505: the other scale rules. A refusal under any of them meets the same fallback.
- 506: partner-only rows (BT-709/BT-660) are never elected and stay out of lot sums.

## Verify

    for t in 7427765 5943121 369057; do curl -s --max-time 20 https://tenders.zebreus.click/v1/tenders/$t | python3 -c "import json,sys; d=json.load(sys.stdin); print(d['id'], d['value'])"; done

- **done**: after 537's drain, 7427765 serves about €3,000,000 (its six lot awards summed), not one lot's €500,000;
  5943121 serves about €1,321,579.92; 369057 serves €62,378,736.29 or `null` with a recorded reason. OR a
  `## Decision` here keeps the max election, with the reason, and a /docs line says a head can be one lot's figure.
- **open (2026-10-16, before 537's build)**: the three serve their ×1000 heads; 537's unit 3 has not run.

## What was found

`head_value_eur_cents_with` elects the MAXIMUM over the electable procedure-scope and head-lot figures. It never
serves a sum. When a scale rule refuses the procedure figure, the next candidate is whatever single figure is
largest:
- **null** when nothing else is electable. This happens to 369057, 380921, 425446, 392980, 251619 and 706887: their
  only other figures are award rows or partner-only rows.
- **one lot's figure** when the lots carry them. This happens to 7829010 (98 % of the true total), 8514592 (88 %),
  6342034 (71 %), 5779108 (66 %), 6921980 (42 %), 1024214 (35 %), 7044132 (34 %), 5943121 (31 %), 7427765 (1/6)
  and 114991.
- **a wrong figure** when a lot carries its own slip: 867491 falls to LOT-0001's €491.4 m, itself 1000 × its award.

Serving the sum of the lots would give the readers' plausible total for 16 of 537's 22. Serving a single lot
understates every multi-lot Tender. That is better than a ×1000 head, but it is still wrong in every value filter
and sort.

## Proposed units

1. **Measure (S).**
   - In section 16's band and in a bounded sample below it, count the heads that are one lot's figure while the
     head version holds ≥ 2 lot figures in the same currency and no procedure figure.
   - Separately, count the Tenders where every procedure figure is refused or absent.
   - Record how far the lot sum is from the elected figure.
2. **Decide (S).**
   - (a) Serve the per-kind lot sum (awards first, then estimates) when no procedure figure survives, marking the
     head as derived.
   - (b) Keep the max election, and document that a head can be one lot's figure.
   - Mind 506 (partner-only rows never summed), 492's `head_lot_sums` (per-lot maximum across fields), and the SSE /
     change-row surface: a derived head is a new kind of value.
3. **Build (M)** if (a): `head_value_eur_cents_with` plus `refold-value-band` → `project`, pinned on 7427765,
   5943121 and 369057.

## Unit 1 — measured (2026-10-16, wf_8519d344-f56)

Evidence: `.scratch/tender-db/551-lot-sum/`.
- `scripts/elect.py` is a Python port of `head_value_eur_cents_with` and its scale rules.
  - `scripts/test_elect.py`: 238 assertions ported from canonical.rs's tests. All pass, and 17 of 20 mutations are
    caught.
  - `scripts/prodcheck.py`: the port against production.
- `results-slim.jsonl`: one line per Tender read, 4,745 lines.
- `skeptic/classify2.json`: the reading skeptic's hand classification of all 88 class members.

**Reads.** 48 bounded `/v1/sql` queries, all keyed by tender id. Each plan was read locally first, and every access
is a SEARCH. There were no 408s.
- **Band (B):** 3,751 Tenders, the ≥ €1 bn heads plus 537's 70.
- **Random sample (R):** 994 Tenders drawn uniformly from ids 1 to 8,848,695.

**Port fidelity:** 0 mismatches. `elect()` equals the stored `current_value_eur_cents` for all 4,745 Tenders.

**The measurer's class, over-counted.** The class is: ≥ 2 lots carry a figure in one currency, and no procedure figure
survives. The measurer found B 84 and R 4.

**The skeptic corrected B.** It read every member's head notice on the public API.
- **Repeated procedure totals (30 tenders).** In 33 of the 84, the head version holds a real procedure total (TED
  VAL_TOTAL/VAL_ESTIMATED_TOTAL, FTS BT-27-Procedure, a modification's VAL_TOTAL). It is stored lot-null, and its cents
  equal a lot award, so issue 492 (c) scores it as an "award copy" (`Lot(None)`). In 30 of those 33, the elected head IS
  that total, and the lot sum repeats it n times. Examples: 8440393, £8 bn on 18 lots, where the sum is £144 bn;
  7955558, £3.68 bn on 19 lots; G-Cloud 15 (8596072), £14 bn, not £70 bn.
- **Double count (1 tender).** 5316712: a modification restates the whole framework on LOT-1.
- **Slips (3 tenders).** 8488015, 5855551 and 8422921 have an admitted procedure figure, beaten by a ×10 slip in one
  award. They are the adjacent class.
- **Lots where summing is wrong.** Among the rest:
  - 274271: placeholders, 230 × PLN 6 bn;
  - 1136952 and 1142891: BT-271 caps per lot;
  - 543944: one figure repeated on 32 lots;
  - 5344896: one lot equals the sum of the others;
  - 5781391: slip-mixed awards.
- **Shifted head (1 tender).** 6885102: a one-lot CAN superseded the procedure total.

So B's class is **at most 51**: the 15 refused by 537, plus at most 36 in band. At least 9 of the 36 still have a wrong
sum, and in the skeptic's hand sample none of the 6 in-band members has the lot sum as its plausible total.

**Where the lot sum IS the plausible total** (confirmed by hand):
- **The 537-refused shape (15).**
  - 6 serve null today: 251619, 369057, 380921, 392980, 425446 and 706887, which would get €24.3 m, €62.4 m,
    PLN 7.2 m, €35.7 m, €1.5 m and PLN 70.6 m.
  - 9 serve one figure, ratio 1.02–6: 7427765 €500 k against €3 m; 5943121 €416 k against €1.32 m; 1024214 €1.2 m
    against €3.4 m (the BT-271 sum, matching the BT-300 text); …
- **R, 4 of 994** (0.40 %, CI 0.16–1.03 %; about 35 k Tenders corpus-wide, 14 k–91 k):
  - 5995030 serves PLN 2,721.60, one lot's award; its 35 awarded lots sum to PLN 1,549,511.60, and the v2 VAL_TOTAL
    reads PLN 1,546,390;
  - 585917 serves null; its awards sum to €250 k, which matches its unmapped BT-118;
  - 691343 €2.85 m against €8.55 m, which matches BT-118;
  - 667498 €1.68 m against €3.36 m, likely right.

**Other counts:**
- **Procedure figures all refused, vs absent:** B 16 all refused (15 by 537, 1 by 492: 6360443, which is not
  multi-lot), against 638 absent. R 0 refused, against 712 absent.
- **Adjacent class:** a larger lot figure beats a smaller admitted procedure figure in 31 B Tenders (114991, 867491, …)
  and in 0 multi-lot R Tenders.

**Exhibits** (the API still serves what the port elects, so the done line is not met): 7427765 €500,000.00, 5943121
€416,053.05, 369057 null.

**What this means for unit 2.** A lot-sum fallback fits the 537-refused shape and the R shape. It is wrong for
frameworks that print one total on every lot or award, for placeholders, for caps, and for one lot equal to the sum of
the others. So any (a) needs guards:
- do not sum when every lot carries one figure that equals a lot-null or procedure figure anywhere in the chain;
- do not sum when one lot equals the sum of the others;
- respect the €100 bn cap.

BT-118 (unmapped, issue 506) confirmed the total wherever it was present.

## Unit 2 — decided (2026-10-16, wf_3d72c491-993)

Evidence: `.scratch/tender-db/551-lot-sum/unit2/`:
- `panel-result.json`: the 88 labels, both designs with their scores, and the judge's verdict;
- `rule_A.py` and `rule_B.py`.

**Labels.** All 88 class members were read with dates: B 84 and R 4. (The task said 92; the list held 88, which is
unit 1's class exactly.)

| best served value | count |
| --- | --- |
| lot sum | 36 |
| current head | 36 |
| something else | 15 |
| unknown | 1 |

**Designs, scored on the labels** (changed-right / closer / farther / unchanged-right / unchanged-wrong):

| design | changed-right | closer | farther | unchanged-right | unchanged-wrong | right after |
| --- | --- | --- | --- | --- | --- | --- |
| A, guarded lot sum over all kinds | 36 | 0 | 2 | 36 | 14 | 72 / 88 |
| B, minimal: only a refusal-vouched sum | 15 | 0 | 0 | 37 | 36 | 52 / 88 |

A's 2 farther are 4925871 (sums insured: €1.5 bn becomes €3.45 bn, against €2.5 m) and 8822313 (×1000 on every lot).
On 1,493 fresh holdout ids, A's estimate path also moved 4930240 farther: a TED object-section pseudo-lot at 3×.

**Decision: a merge (the judge's verdict).**

ADOPT A MERGE: B core (the "vouched" sum, which equals A's path A, with B's 471/492 extension) plus A's path B only when the summed kind is lot AWARDS, under stricter guards. A's path B over BT-27-Lot estimates and BT-271-Lot framework maxima is REJECTED. For those heads the max election stays, and a /docs line says the head can be one lot's figure.

How I checked:
- Both rule files re-run on all 4,745 records with my own scorer and my own copy of the labels (they match both designers' copies exactly, 88 entries, not 92).
- Scores reproduce exactly:
  - A: 36 changed-right / 0 closer / 2 farther / 36 unchanged-right / 14 unchanged-wrong.
  - B core: 15 / 0 / 0 / 37 / 36. B r_shape2: 34 / 0 / 2 / 36 / 16. B r_split: 16 / 0 / 0 / 37 / 35.
  - Changes in A: B 34, R 4. Changes in B core: B 15, R 0. Neither changes anything outside the labelled set.
- A's path A and B core serve the same value on all 15 of 537's refused heads (8514592 differs by 1 cent, from the rate).

Fresh data A never tested:
- I ran an API approximation of A (T + path B + G0-G6) on B's 1,493 fresh holdout tenders. It fires 7 times:
  - right (4): 467176, 294968, 7256284, and 423618 (new, which I read: BT-118 = EUR 14.75 m = the lot sum);
  - closer (2): 5977060, 8771589;
  - FARTHER (1): 4930240. Its LOT-1 is the old TED object section, exactly 3.000 x the 8 area lots (EUR 799,022.40 x 3); EUR 2.40 m, which is right, becomes EUR 3.20 m.
- A's guards also blocked correctly out of sample: G1 stopped 7487018 (42 awards x EUR 450 k, the repeated total) and G3 stopped 5625591 (one lot = the sum of the others).

Split by summed kind, across labelled and fresh fires:
- AWARDS: 10 fires, 9 right, 1 closer, 0 farther. The 9 right: 6450467/69/73, 8426318, 585917, 5995030, 467176, 294968, 7256284. The closer one: 5977060.
- ESTIMATES / FRAMEWORK MAXIMA: 20 fires, 16 right, 1 closer, 3 farther (15 %, 95 % CI 5-36 %). The farther ones are 4925871 (sums insured), 8822313 (x1000 on every lot) and 4930240 (a pseudo-lot at 3x). No stored field can detect any of them. Two of them land in the EUR 1 bn band, and 4930240 was right before.

Weighing it: wrong sums cost more than understated heads, and 537's S is already computed in ScalePartners. The vouched path is buyer-confirmed and costs almost nothing to build. On awards the evidence supports serving the sum (0 farther, a principled reason, every guard cheap on LotSums). On estimates it does not.

Live reads (2026-10-16, public API, at most 2 req/s):
- 15 changed heads: 1024214, 369057, 4671850, 4715842, 4894521, 585917, 59219, 5995030, 6450467, 667498, 691343, 7427765, 8426318, 8743051, 917292. All 15 still serve today's figure, and their API lot rows sum exactly to the rule's S.
- Also read: 8822313 plus its predecessor 8819836 (lots EUR 1.38 m / 1.5 m / 2.04 m, so x1000 is confirmed), 4925871 (sums insured, confirmed), 423618 with CAN 24470145, 4930240, 8771589, 928196, 667498's CN 25729092 (both lots carry the identical BT-300 sentence, so it stays ambiguous), and fresh re-reads of 294968, 7256284, 5977060 and 467176.

Work files: /tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/m551/u2/judge/ (run_both.py, both.pkl, score_J.py, probe_A.py, merge.py, labels_J.py, fetch.py, show.py, nshow.py, api/).

**The adopted rule**, precise enough to build:

Change `head_value_eur_cents_with` so it returns a HeadValue: `eur_cents` plus `Option<Derived>`. `Derived` holds:
- path: Vouched{rule: 537|471|492, f_field, f_cents, f_currency, k} or Awards;
- kind: LotSumKind;
- cents and currency (S in native money);
- lots (n).

The max election stays everywhere except on the two paths below, tried in order.

PATH V: the vouched lot sum. B core, exactly.
1. Precondition: no procedure-scope figure of the head is elected or admitted. Award copies (Lot(None), 492 (c)) and partner-only rows (506) are not procedure figures.
2. For each procedure-scope figure F refused by a scale rule, find S:
   - Refused by 537: take `partners.lot_sum_match(currency, F)`, i.e. its LotSumMatch {kind, exponent k, sum, lots}.
     - If that kind is electable (Awards, Estimate, FrameworkMaximum), S is that sum.
     - If it is partner-only (BT-709/BT-660), S is the electable LotSums kind with `lot_sum_exponent(F, S', n') == k`: closest |F - 10^k*S'| first, ties broken Awards, then Estimate, then FrameworkMaximum. The partner-only sum is never served (1024214: BT-709 matches, so the BT-271 sum EUR 3.4 m is served).
   - Refused by 471 or 492: S is the electable LotSums kind with `lot_sum_exponent(F, S', n') == the refusal's k` (471's own k; 2 for 492), chosen the same way (928196 gets EUR 46,797,460.35).
3. Every refused F that yields an S must yield the same (currency, S). Otherwise keep the max election.
4. Need n >= 2 and 0 < S_eur <= EUR 100 bn (366), with S_eur = S converted by RatesLookup at the head's date.
5. V1: S is not one figure M repeated on every summed lot with F within 1 % of 10^j*M (j >= 0).
6. No other repeat guard: the buyer's own figure at exactly 10^k vouches for the sum (7427765: 6 x EUR 500 k gives EUR 3,000,000.00).

PATH W: unvouched AWARDS lot sum (A's path B, restricted). Taken only when path V does not fire.
- T': every procedure-scope figure of the head is absent, or not-electable as a sentinel/placeholder (366/379). Any procedure figure that is refused, withheld or over the cap keeps today's head.
- K: today's head is an award copy (Lot(None)) or null. The summed kind is LotSumKind::Awards (each Lot's latest award by notice id, as LotSums builds it) in the elected figure's currency. For a null head, use the currency with the most award lots.
- Serve S = the awards sum unless a guard fires (see guards). Path W never sums Estimate or FrameworkMaximum, and never a partner-only kind.

MARKER: never a tender_version_amounts row. A synthetic row would re-enter the fold's ScalePartners figures and 492's award-copy test, and could confirm itself on the next fold.
- Add nullable columns on `tenders`:
  - `current_value_derivation` ('lot-sum-vouched' | 'lot-sum-awards'),
  - `current_value_cents`, `current_value_currency` (S in native money),
  - `current_value_derivation_detail` (JSON: kind, lots, rule, k, F field/cents/currency).
- `current_value_eur_cents` = S_eur, so value sorts and filters work unchanged.
- Code that must branch on the marker:
  - read.rs's elected-row pick (around lines 2525-2540, which matches `s.eur_cents = t.current_value_eur_cents` and would find no row);
  - data_quality.rs section 16 (around line 1631);
  - SSE and change rows (a new kind of value);
  - the API `value` gains a `value_derivation` object.
- /docs: one line that a head may be a derived lot sum (with its reason), and one line that a multi-lot Tender with only lot estimates and no procedure figure serves one lot's figure.

Pins for unit 3:
- Positive:
  - 7427765 EUR 3,000,000.00;
  - 5943121 EUR 1,321,579.92;
  - 369057 EUR 62,378,736.29;
  - 1024214 EUR 3,400,000;
  - 928196 EUR 46,797,460.35;
  - 6450467 EUR 3,574,764,979.28;
  - 585917 null -> EUR 250,000 (the BT-709 EUR 250 k x2 is never summed);
  - 5995030 PLN 1,549,511.60 (its small repeated pair passes G1').
- Negative (stay at today's figure):
  - 8440393 (G1');
  - 7487018 (G1');
  - 5316712 (G3);
  - 5625591 (G3);
  - 6885102 (G0);
  - 4930240, 8822313, 4925871 and 175096 (estimates, so path W cannot fire).

**Guards:**
- PATH V precondition: no procedure-scope figure elected or admitted (award copies and partner-only rows do not count); n >= 2 lots in S's kind.
- PATH V S-selection: 537's LotSumMatch S when its kind is electable; for a partner-only (BT-709/BT-660) match, the electable sum at the SAME k (closest, then Awards > Estimate > FrameworkMaximum); for 471/492, the electable sum with lot_sum_exponent(F,S',n') == the refusal's k; none -> keep.
- PATH V agreement: every refused F that yields an S yields the same (currency, S); else keep.
- PATH V cap: 0 < S_eur <= EUR 100 bn (366), converted by RatesLookup at the head's date.
- PATH V V1: refuse if every summed lot carries one figure M and F is within 1 % of 10^j*M (j >= 0). Then the lots ARE the slipped total, e.g. n = 10 lots each repeating T with F = 1000*T; 7427765 passes (F/M = 6000).
- PATH W T': every procedure-scope figure absent or sentinel-not-electable (366/379); any refused (471/492/537 without a vouched match), withheld (372) or over-cap procedure figure keeps today's head.
- PATH W K: today's head is an award copy (Lot(None)) or null; sum LotSumKind::Awards only (latest award per Lot by notice id), in the elected currency (null head: the currency with the most award lots). Never Estimate, FrameworkMaximum or partner-only kinds.
- PATH W G0: n >= 2 award lots and S_eur > today's eur_cents (equal or below keeps today's head).
- PATH W G5: S_eur <= EUR 100 bn, and no procedure or lot-null figure of the head is not-electable as over EUR 100 bn.
- PATH W G6: no lot award of the head is in another currency.
- PATH W G7: no withheld (372) procedure figure.
- PATH W G8: every summed award names a lot of the head's counted unit (Lot, or Part in a Lot-less head). Stops a re-keyed chain (TED LOT-1 vs eForms LOT-0001) being counted twice; costs 0 of 10 rights.
- PATH W G1' (repeat, confirmed or not): no figure on >= 2 summed lots that is >= 1/10 of the largest summed lot. Stricter than A's G1 (no lot-null confirmation needed) and covers 2-3 equal awards with no stored total; costs 0 rights (5995030's repeated PLN 8,000.00 is under 1/10 of its largest lot).
- PATH W G2: no figure carried by >= 4 summed lots AND by >= half of them.
- PATH W G3: with n >= 3, the largest lot is not within 1 % of 10^k x the sum of the others, for k = 0..3 (5316712, 5625591, 4787274 shapes).
- PATH W G4: no Estimate or FrameworkMaximum LotSums kind on >= 2 of the same lots whose sum over those lots is < 1/10 or > 10x the awards over them.
- Both paths: set the derivation marker columns; never write a synthetic amount row; partner-only rows (506) are never summed or served.

**Expected effect:**

LABELLED 88:
- 21 changed-right, 0 closer, 0 farther, 37 unchanged-right, 30 unchanged-wrong.
- Right after: 58/88. Right today: 37/88. A as designed: 72 right with 2 farther. B core: 52 right.
- The 21:
  - Path V (15): 251619, 369057, 380921, 392980, 425446, 706887, 1024214, 5779108, 5943121, 6342034, 6921980, 7044132, 7427765, 7829010, 8514592.
  - Path W (6): 6450467, 6450469, 6450473 (EUR 1.66 bn -> EUR 3,574,764,979.28), 8426318 (GBP 2 bn -> GBP 3 bn), 585917 (null -> EUR 250 k), 5995030 (PLN 2,721.60 -> PLN 1,549,511.60).
- Left understated on purpose (14 lot_sum labels):
  - 12 in-band estimate / framework-maximum sums: 59219, 242212, 411001, 741063, 917292, 979673, 1122853, 4534889, 4671850, 4894521, 8699396, 8743051;
  - R's 667498 and 691343 (uniform estimates; BT-118 would confirm them).
- The 36 current_head labels all stay right. The 15 other and 1 unknown stay as today, with no new damage: 4925871 and 8822313 are NOT made worse.

B (band census, 3,751 heads): 19 change.
- 15 by path V. All end below EUR 1 bn; 6 null heads gain a value.
- 4 by path W, raised inside the band: the Lyon-Turin trio and 8426318.
- 0 changes outside the labelled class.

R (994): 2 change, both right (585917, 5995030).

Holdout (1,493 fresh, API approximation): path V 0; path W 4.
- right: 467176 CHF 771,192 -> CHF 3,161,818; 294968 CHF 528,747 -> CHF 4,440,992; 7256284 EUR 109,100 -> EUR 574,873;
- closer: 5977060 EUR 33,243 -> EUR 4.14 m, against a plausible EUR 5.39 m (a re-run lot reuses the key LOT-1).

CORPUS (about 8,795,603 heads):
- Path V: about 16 heads today (537's 15 plus 928196 from the 471/492 probe), all right on reads, plus future refusals at fold time.
- Path W: 6/2,487 fresh = 0.24 %, about 21,200 heads (95 % Wilson 9,700-46,200).
  - Fresh outcome 5 right, 1 closer, 0 farther. Over all 10 awards fires, 0 farther, with a 95 % upper bound of 28 %.
- For comparison, A's full path B fires on about 38,900 (21,700-69,500), with 1 in 11 fresh fires farther and 3 in 27 overall.
- Each path-W change fixes an understatement of 1.5x to 569x. The guards that matter (G1', G2, G3, G4) are pure functions of LotSums and ScalePartners.lot_only.

Build: change `head_value_eur_cents_with`'s return type; add per-lot accessors on LotSums; add the marker columns; branch read.rs, section 16, SSE / change rows and the API. Because refold-value-band's cohort (stored value >= EUR 1 bn) reaches neither path, the drain needs a corpus refold (as 2563/2564 did) or an id cohort.

Done line: met through path V (7427765 EUR 3,000,000.00; 5943121 EUR 1,321,579.92; 369057 EUR 62,378,736.29, marked derived).

**Rejected:**

1. A AS DESIGNED (estimate / framework-maximum path B): rejected.
- Its estimate and framework-maximum sums move 3 of 20 the wrong way (15 %, 95 % CI 5-36 %):
  - 4925871: sums insured, EUR 1.5 bn -> EUR 3.45 bn against EUR 2.5 m;
  - 8822313: x1000 on every lot, EUR 2 bn -> EUR 5.1 bn against EUR 5.1 m (its predecessor 8819836 carries EUR 1.38 m / 1.5 m / 2.04 m);
  - 4930240: object-section pseudo-lot at 3.000x, a right EUR 2.40 m becomes EUR 3.20 m.
- No stored field can detect any of these shapes. An integer-multiple G3 that caught 4930240 would block 8426318 (GBP 2 bn = 2.000 x the others).
- Two of the three inflate heads in the EUR 1 bn band, which pollutes the most visible sort.
- The in-band gains are modest (x1.08-2.3), and several rest on labeller judgement: a PIN of separate contracts (4894521), an upper bound (8743051).
- A ran no fresh holdout. Its approximation on B's 1,493 fresh ids fires 7: 4 right, 2 closer, 1 farther.

2. A's G1 as written (lot-null confirmation required): replaced by G1', which costs no right and closes the gap of 2-3 equal awards with no stored total.

3. A's path A matching at any k: replaced by B core's same-k matching, which is stricter and gives identical results on all 15.

4. A's path B after a 537 miss, or after any refusal (refused F -> elected-kind sum): rejected. The buyer's scale is suspect, and there is no evidence either way (0 cases).

5. B CORE ALONE: correct (15/15 plus 928196, 0 wrong) but leaves about 21 k lot-award understatements (x1.5-569) in place. The evidence shows these sum safely: 10 fires, 9 right, 1 closer, 0 farther, and the guards held out of sample (7487018, 5625591).

6. B r_shape2: tuned to the labelled set (R3', R4' and R5' each fix one member), includes estimates (the same 2 farther), and moves 1 in 6 fresh fires the wrong way.

7. B r_split: subsumed by path W. Path W also covers single-CAN multi-result awards (Lyon-Turin, 8426318) and null heads (585917), with no wrong move.

8. (b) KEEP THE MAX ELECTION EVERYWHERE: fails the done line for 3 exhibits, and leaves 6 null heads whose total the buyer itself vouches for.

9. Adjacent class (a x10 lot slip beating an admitted or award-copy total; repeat-slip follow-up "serve M"): out of scope. It fires on 4 unlabelled heads with admitted procedure figures (8755619, 143300, 878305, 8630332) and needs its own issue and reads.

10. Serving null for plausible-null labels (274271, 8811130, 5781391): not part of this decision.

**Open questions for unit 3:**
- Marker shape: confirm columns on `tenders` (derivation, native cents, currency, detail JSON) over a synthetic amount row. A synthetic row would re-enter ScalePartners.lot_only and 492's award-copy test on the next fold. Which API field name carries it (`value_derivation`)?
- Drain route: refold-value-band (cohort: stored value >= EUR 1 bn, supervisor.rs value_band_tender_ids) reaches neither the 15 path-V heads (all below EUR 1 bn or null since 537's drain) nor most path-W heads. Unit 3 needs a corpus refold (2563/2564 scale) or an id cohort: heads with >= 2 lot awards and no admitted procedure figure, plus heads with 471/492/537 refusals.
- Path W's wrong-direction rate rests on 10 awards fires (0 farther, 95 % upper bound 28 %). Propose a post-drain bounded read: the derived awards heads >= EUR 100 m, plus a uniform sample of about 50 path-W changes, read before the change is called done.
- Path V's 471/492 branch rests on one case (928196). The V1 guard (lots repeat M with F ~ 10^j*M) is theoretical, with 0 observed cases; keep it, but it is untested.
- Re-keyed chains where BOTH key sets are typed lots (a TED CAN on LOT-1..n and an eForms modification on LOT-0001..n) would pass G8 and double count. Unmeasured; a test fixture should pin the expected behaviour.
- Latest-award-per-lot undercounts when a re-run procedure reuses a lot key (5977060: the 2019 painting re-run keyed LOT-1 replaces Sogea's EUR 1.242 m). Accept as closer, or key the latest award by result key or winner?
- Estimate-only multi-lot heads stay one lot's figure. Mapping BT-118-NoticeResult (issue 506) would confirm uniform per-lot sums (691343, 423618: BT-118 = the lot sum) and could later admit a BT-118-vouched estimate sum as a third path. Separate issue?
- The pseudo-lot (TED object section stored as LOT-1, here at 3x) matters only if the estimate path is ever revisited. Possible signals: a lot title equal to the Tender title, or exact integer multiples (which blocks 8426318 at m = 2).
- Thresholds inherited from A and fitted on the 88 labels: G1' share 1/10, G2 >= 4 lots and >= half, G3 1 % with k = 0..3, G4 10x. They are only weakly tested below the band (R: 2 awards members; holdout: 4).
- EUR conversion: Rust should use RatesLookup at the head's date. The labels differ by 1-4 cents where the designers used a row rate (8514592).
- The task named 92 labels; its JSON carries 88 (unit 1's class exactly, B 84 + R 4). Were 4 labels dropped upstream?
