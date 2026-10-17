# 551 — when the election refuses a head, it falls to one lot's figure (or to null), never to the sum of the lots

Status: ready-for-agent — path V LIVE (16 heads); path W WITHDRAWN and refolded back (unit 4a done). Unit 4b DECIDED 2026-10-17 (wf_efa005fa-6a8; adversarial check found it unsound as written, amendments ADOPTED): path W v2 = v1's guards + refusals R1–R10 (R10: a stated total vouches every sum ≥ EUR 10 m per amendment (a)), measured BEFORE any Rust by a salted shadow walk; bar: ≤ 1 wrong in a fresh uniform 30 (skeptic on every head, two agreeing reads) and 0 wrong of ANY class among band heads. NEXT: M1 (rule_v2.py port + walk). Fallback fixed: 'vouched everywhere'; if both fail, path W closes for good (unit X).
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
4. Need n >= 2 and 0 < S_eur <= EUR 100 bn (366), with S_eur = S converted by RatesLookup at the head's date. (V0, added at unit 3's review: and S_eur above today's figure; see the guards.)
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
  - 6885102 (G0);
  - 4930240, 8822313, 4925871, 175096 and 5625591 (estimates, so path W cannot fire; 5625591 was labelled
    "(G3)" here until unit 3's review: that was design A's estimate-path G3, and path W never reaches a guard for
    it).

**Guards:**
- PATH V precondition: no procedure-scope figure elected or admitted (award copies and partner-only rows do not count); n >= 2 lots in S's kind.
- PATH V S-selection: 537's LotSumMatch S when its kind is electable; for a partner-only (BT-709/BT-660) match, the electable sum at the SAME k (closest, then Awards > Estimate > FrameworkMaximum); for 471/492, the electable sum with lot_sum_exponent(F,S',n') == the refusal's k; none -> keep.
- PATH V agreement: every refused F that yields an S yields the same (currency, S); else keep.
- PATH V cap: 0 < S_eur <= EUR 100 bn (366), converted by RatesLookup at the head's date.
- PATH V V0 (added at unit 3's review, 2026-10-16): when today's head is not null, S_eur > today's eur_cents (path W's G0 test); a vouched sum at or below the max election's figure keeps it. Without it path V could LOWER a head: 537's recorded-cost shape (F = EUR 1.2 bn = the two lot estimates' sum, awards 2 x EUR 6 m, so 537 refuses F at k = 2) served the awards' EUR 12 m marked vouched, x100 under the true total, where the max election serves one lot's EUR 600 m. Costs 0 of the 16 path-V pins (each lifts its head) and 0 of the 4,745 records.
- PATH V V1: refuse if every summed lot carries one figure M and F is within 1 % of 10^j*M (j >= 0). Then the lots ARE the slipped total, e.g. n = 10 lots each repeating T with F = 1000*T; 7427765 passes (F/M = 6000).
- PATH W T': every procedure-scope figure absent or sentinel-not-electable (366/379); any refused (471/492/537 without a vouched match), withheld (372) or over-cap procedure figure keeps today's head.
- PATH W K: today's head is an award copy (Lot(None)) or null; sum LotSumKind::Awards only (latest award per Lot by notice id), in the elected currency (null head: the currency with the most award lots). Never Estimate, FrameworkMaximum or partner-only kinds.
- PATH W G0: n >= 2 award lots and S_eur > today's eur_cents (equal or below keeps today's head).
- PATH W G5: 0 < S_eur <= EUR 100 bn, and no procedure or lot-null figure of the head is not-electable as over EUR 100 bn. (The lower bound came in unit 3's third review round, 2026-10-17, as path V's cap has it: on a null head, two awards above the 10.00 floor in a currency the rates table converts at more than ~4,000 per EUR, IDR or the pre-2005 ROL/TRL, sum to a figure that rounds to EUR 0, the value issue 378 reserves for "the election found nothing". 0 of the 4,745 records hold such a currency.)
- PATH W G5' (added at unit 3's review, 2026-10-16; widened in its third round, 2026-10-17): no lot-null figure (an award's tender-scope copy) that a scale rule refused (471/492 at Lot(None)), AND no summed award that a scale rule refuses at its own lot (`refuses_amount("result_value", currency, c, eur, Lot(Some(key)))`), copy or none. A x1000 award's copy below EUR 100 bn is refused, not over the cap, so G5 missed it and the slip itself was summed: LOT-1 awarded EUR 1.2 bn beside LOT-2's EUR 1.2 m, copies of both, served EUR 1.2012 bn where today's head is EUR 1.2 m. Round 1 judged only the copy, so the same slip with NO copy turned a null head into EUR 1.2012 bn, and with LOT-2's copy alone raised EUR 1.2 m to it: the max election never reads lot results, so nothing else judges a copy-less award, but 471 refuses it when asked at its lot (partner: LOT-2's award). A refused figure says the buyer's scale is broken, so today's head stays. Every rule's EUR 1 bn gate makes the check free below it. Costs 0 of the 21 changes and 0 of the 4,745 records (summed awards refused at their lot occur in 4 records, 288191, 319527, 614457 and 666053, and T' already stops all 4).
- PATH W G6: no lot award of the head is in another currency.
- PATH W G7: no withheld (372) procedure figure.
- PATH W G8: every summed award names a lot of the head's counted unit (Lot, or Part in a Lot-less head). Stops a re-keyed chain (TED LOT-1 vs eForms LOT-0001) being counted twice; costs 0 of 10 rights.
- PATH W G8b (added in unit 3's second review round, 2026-10-16): no two DISTINCT summed lot keys that spell one lot number. A key is read as a number only where it is recognisably a numbered lot key: ASCII digits alone (an OCDS id, `1`), or `LOT` in any case then digits, directly or after one `-`, space or `_` (TED's synthetic `LOT-1..n`, eForms' `LOT-0001`, `Lot 2`, `LOT2`), leading zeros dropped; any other key (`1a`, `1.2`, `2/E`, `L1`, `PAR-0001`) never trips it. The number is the key's SPELLING, not the published lot number: TED's `LOT-k` is the notice's k-th lot section in document order (`r209::parse`; the published LOT_NO survives only as the `TED-LOT_NO` value, and nothing re-keys a TED lot by it), so `LOT-k` is lot k only where the notice describes every lot from 1 in order. Where it does not, G8b misses a misaligned re-keying (a TED CAN describing only lots 3 and 5 as `LOT-1`/`LOT-2` beside an eForms restatement on `LOT-0003`/`LOT-0005` reads {1, 2, 3, 5} and sums each lot twice) and can refuse distinct lots (TED's `LOT-1` for lot 3 beside eForms' `LOT-0001`), which is harmless because today's head stays. The stored key carries no LOT_NO, so there is no cheap fix; recorded at unit 3's third review round, and the post-drain read compares each summed TED key against its notice's `TED-LOT_NO`. Why: a re-keyed chain whose BOTH key families are typed Lots (TED LOT-1..n beside eForms LOT-0001..n) passes G8 and summed each lot twice; round 1 accepted that as a recorded cost because 0 of the 4,745 records showed it, but the drain folds ~195,000 Tenders, and a refused path W keeps today's head, so failing closed cannot regress a head. A pure function of the summed set, like G1'..G4. Costs 0 of the 21 changes and 0 of the 4,745 records.
- PATH W G1' (repeat, confirmed or not): no figure on >= 2 summed lots that is >= 1/10 of the largest summed lot. Stricter than A's G1 (no lot-null confirmation needed) and covers 2-3 equal awards with no stored total; costs 0 rights (5995030's repeated PLN 8,000.00 is under 1/10 of its largest lot).
- PATH W G2: no figure carried by >= 4 summed lots AND by >= half of them.
- PATH W G3: with n >= 3, the largest lot is not within 1 % of 10^k x the sum of the others, for k = 0..3 (5316712 and 4787274 shapes; 5625591 is the same shape in lot estimates, which path W never reaches).
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
- Re-keyed chains where BOTH key sets are typed lots (a TED CAN on LOT-1..n and an eForms modification on LOT-0001..n) would pass G8 and double count. Unmeasured; a test fixture should pin the expected behaviour. DECIDED at unit 3's review: accepted as a recorded cost and pinned; REVISED in its second round (2026-10-16): G8b refuses it (see Unit 3, "Review fixes").
- Latest-award-per-lot undercounts when a re-run procedure reuses a lot key (5977060: the 2019 painting re-run keyed LOT-1 replaces Sogea's EUR 1.242 m). Accept as closer, or key the latest award by result key or winner?
- Estimate-only multi-lot heads stay one lot's figure. Mapping BT-118-NoticeResult (issue 506) would confirm uniform per-lot sums (691343, 423618: BT-118 = the lot sum) and could later admit a BT-118-vouched estimate sum as a third path. Separate issue?
- The pseudo-lot (TED object section stored as LOT-1, here at 3x) matters only if the estimate path is ever revisited. Possible signals: a lot title equal to the Tender title, or exact integer multiples (which blocks 8426318 at m = 2).
- Thresholds inherited from A and fitted on the 88 labels: G1' share 1/10, G2 >= 4 lots and >= half, G3 1 % with k = 0..3, G4 10x. They are only weakly tested below the band (R: 2 awards members; holdout: 4).
- EUR conversion: Rust should use RatesLookup at the head's date. The labels differ by 1-4 cents where the designers used a row rate (8514592).
- The task named 92 labels; its JSON carries 88 (unit 1's class exactly, B 84 + R 4). Were 4 labels dropped upstream?

## Unit 3 — built (2026-10-16, wf_69ed7f6c-050)

Evidence and scripts: `.scratch/tender-db/551-lot-sum/unit3/` (`merge_rule.py` is the adopted rule written from
the "Unit 2 — decided" text alone, over unit 1's `elect.py`; `gen_fixtures.py` writes the Rust fixtures;
`export_lines.py` + `port_check_harness.rs` + `compare.py` are the port check).

**The election** (`crates/store/src/canonical.rs`).
- `head_value_eur_cents_with` returns `HeadValue { eur_cents, derived: Option<DerivedValue> }`. `DerivedValue` is
  `{ path, kind: LotSumKind, cents, currency, lots }`; `path` is `DerivationPath::Vouched { rule: ScaleRule
  (Partner 471 | Hundred 492 | LotSum 537), field, cents, currency, exponent }` or `DerivationPath::Awards`.
  `head_value_eur_cents(chain)` still returns the EUR cents (every existing caller); `head_value(chain)` returns
  the whole `HeadValue`.
- The max election is unchanged, split out as `elect_head_figure` (the last of equal maxima, with its currency and
  whether it is an award copy) over one judgement per candidate (`judge_head_amount`: partner-only, withheld,
  sentinel, no rate, refused, over cap, zero, admitted — the election's order). `derive_lot_sum` then tries path V,
  else path W, exactly as decided: V's precondition, S-selection (537's own electable S; a partner-only match →
  the electable sum at the same k; 471/492 → `lot_sum_exponent(F, S', n') == k`; closest, then Awards > Estimate >
  FrameworkMaximum), agreement on (currency, S), the cap, V1; W's T', K, G0, G5, G6, G7, G8, G1', G2, G3, G4 (V0, G5', G8b and G5's lower bound came with the review fixes).
  Neither path ever sums a partner-only kind; W never sums Estimate or FrameworkMaximum.
- New machinery: `ScalePartners::procedure_refusal` (which rule refused a procedure figure, branch for branch with
  `refuses_amount`, debug-asserted equal), `ScaleRefusal`, `LotSums::by_currency` (the one construction 537's
  cached sums and 551 read; `lot_sums_of` now delegates to it), `LotSums::per_lot` / `award_lots` / `counts`.
- S's EUR value is `RatesLookup::eur_cents(S, currency, head date)`.

**The marker.** Nullable `tenders` columns `current_value_derivation` ('lot-sum-vouched' | 'lot-sum-awards'),
`current_value_cents`, `current_value_currency`, `current_value_derivation_detail` (JSON: `summed`, `lots`, and for
path V `rule`, `exponent`, `refused {field, value {cents, currency}}`), added in canonical's SCHEMA,
`reset_tender_layer`'s CREATE and `migrate()` (metadata-only ALTERs). `current_value_eur_cents` = S's EUR value. The
fold's `head_update` writes all four (NULL for an elected head) with every head write; `StoredHead` carries them,
so the shadow compare announces a head that becomes or stops being derived at an unchanged EUR figure. Never an
amount row (pinned: the fold test counts the amount rows). One writer, pinned
(`the_derived_head_marker_has_one_writer`).

**The read path.**
- `read.rs` `tender_select_head`: at the head version a derived head's `cents` / `currency` are the stored money
  (`CASE WHEN t.current_value_derivation IS NOT NULL AND v.seq = t.current_seq THEN … ELSE <the elected-row pick>
  END`), and two new columns carry the method and detail (head version only). Plan-probed on a copy of
  `planchk/schema.db` with the four columns added: the list statement's plan is IDENTICAL to the pre-551 shape (diffed),
  no new SCAN.
- API: every Tender row gains `value_derivation` (null, or `{method, …detail}`); `value` stays Money. OpenAPI
  declares it (`components.schemas.Tender.properties.value_derivation`).
- Section 16 (`data_quality.rs`): a derived head is listed under its own currency with its sum (COALESCE of the
  marker money and the elected row; the elected-row join is skipped for a derived head), its lot-sum column reads
  `derived: <method>`, and it is never "elected row NOT FOUND". Plan-probed: all SEARCH, first line the value-index
  seek, as before.
- SSE / change rows: no code of their own. A derived head is a head-column change like any other: an appended
  version announces it by its transition rows, a refold's compare by its correction rows (the marker is in
  `StoredHead`). (The At probe's earlier-version superset did need code: see "Review fixes".)

**Decisions beyond the spec.**
- API field name: `value_derivation`, a top-level sibling of `value` (not inside it), so `value` keeps the Money
  shape every client already parses. `method` names the column's value; the detail's keys are merged in.
- Detail JSON keys: `summed` (the `LotSumKind` label: `awards`, `BT-27-Lot`, `BT-271-Lot`), `lots`, `rule` (the
  deciding issue as a string), `exponent`, `refused`. Written by hand in the store (no JSON crate there, and adding
  one would re-hash `Cargo.toml`); every string is JSON-escaped.
- A derivation is served on the head version only; an older version's row keeps the elected-row pick.
- Path V's S-selection uses `LotSums`' three electable kinds (as the issue specifies), not rule_B's extra
  lot-scope `result_value` kind; the port check shows no record where that differs.
- 537's tests (`a_total_ten_to_the_k_times_its_lot_award_sum_is_not_elected`) now pin the MAX election
  (`max_election_eur_cents`, test-only): what a refusal falls to. The derivation on top is pinned separately on
  real rows; `cost` and `bounded` also pin what is SERVED (`head_value`), which V0 keeps at the max election.
- No epoch bump (recorded on `PROJECTION_EPOCH`): R3 by tender id.
- No ADR: value-election decisions live in the issues and `canonical.rs` (366/471/492/505/537 have none either).

**Tests** (focused command; logs in the scratchpad `b551/`).
- `canonical_551_fixtures.rs` (generated): the stored rows of the 15 path-V heads, the 6 labelled path-W heads,
  8440393, 5316712, 4925871, 8822313, 274271, 6885102, 175096 (unit 1's pull), and from the public API 4930240,
  928196 (its x100 partner version restated from its refusal), 7487018, 5625591 and the holdout 467176, 294968,
  7256284, 5977060.
- `a_refused_total_serves_the_lot_sum_it_vouches_for`: all 16 path-V pins (rule, k, field, F, kind, S, currency,
  n, EUR). `an_unvouched_awards_sum_is_served_under_its_guards`: the 10 path-W pins. `the_rejected_shapes_keep_the_max_election`:
  8440393, 7487018, 5316712, 6885102, 4925871, 8822313, 175096, 274271, 4930240, 5625591.
- `the_vouched_path_takes_only_the_sum_its_refusal_names` and `the_awards_path_refuses_every_shape_its_guards_name`:
  one synthetic shape per guard (precondition, award copy not a procedure figure, 471 naming k, partner-only with
  and without an electable same-k sum, agreement, V1 — a near-exact F so 537 rather than 471 refuses —, the cap;
  T' admitted / sentinel / withheld / over cap, K, estimates never summed, G0 one lot and below today, G5 copy over
  cap and S over cap, G6, G8, G8b (both typed families, one shared number, `1` beside `Lot 1`; each family alone and sub-lot keys still sum), G1' with the small-pair pass, G2, G3 k = 0 and k = 2, G4 off and near, the null-head
  currency, a non-EUR sum, no rate).
- Store: `a_derived_lot_sum_head_is_stored_and_served_as_the_fold_derived_it` (fold → columns → read row → a later
  admitted figure clears the marker), `the_derived_head_marker_has_one_writer`. Ingest: section 16 lists a derived
  head. App: `a_derived_head_says_how_its_value_was_derived` (json), `refold_lot_sum_requeues_the_tenders_a_derived_head_can_reach`.
- The two fold goldens (`project_refold.snapshot`, `fold_writer.snapshot`) were recaptured for the schema change
  only: the diff is the four new columns, NULL on every row, no value moved (533's precedent for its column).

**Port check.** The Rust election (a temporary ignored harness, `unit3/port_check_harness.rs`, since removed) run on
all 4,745 unit-1 records against `merge_rule.py` with the same rate table (each currency's rate from the head's
largest stored conversion): **0 disagreements** — the max election equals Python's on 4,745/4,745 (and equals
the stored production head on 4,745/4,745), and the 21 derived heads agree on path, rule, k, F, kind, S, currency
and n. One harness artifact was fixed on the way (a legacy code, `SKR`, seeded under its spelling rather than
`canonical_currency`'s). `merge_rule.py` itself reproduces unit 2's expected effect exactly: 21 changes, B 19
(15 V + the Lyon-Turin trio + 8426318), R 2 (585917, 5995030), nothing outside the labelled class.

**Drain design** (docs/operations.md, "The derived lot-sum head"; nothing run). A new job kind `refold-lot-sum`
(R3 by tender id), dry by default; the wet run needs `expect` (the dry count), aborts off it by more than a quarter
before any write, stamps the cohort stale, re-queues its notices and queues `project` behind itself; stoppable
(the walk reads the stop flag per window; listed in `STOPPABLE_KINDS`, heavy-write, Box::pin'd arm).
- Cohort (`canonical::lot_sum_cohort_sql`, 100k-id windows on the reader pool): a head version with a tender-scope
  figure of €1 bn or more (path V needs a scale refusal, every rule gated there), or awards above 10.00 on ≥ 2
  distinct lots with no tender-scope figure that is neither an award copy, nor partner-only, nor in the sentinels'
  range (≤ 1, = 100, ≥ 111,111,111). A superset (it over-selects big real figures rather than copy the digit
  walk); all 21 unit-2 changes are inside it. Plan-probed: PK range, then index seeks only.
- Size: R's 22/994 → about 195,000 Tenders (95 % 129,000–293,000); the looser "≥ 2 awarded lots" cohort is
  ~460,000. Fold ≈ 6–10 min at 2564's rate, against a corpus refold's ~5 h. Expected moves: path V ~16, path W
  ~21,200 (9,700–46,200).
- Why not refold-fields over the award value field ids: every award carrier of every era, a corpus-scale sweep
  that still misses path V's estimate sums and re-folds millions of single-lot awards.

**Review fixes (2026-10-16, on the uncommitted build).** Eleven confirmed findings, each fixed minimally:
- **G5' (blocking).** Path W summed a lot award whose tender-scope copy 471/492 refused: `derive_lot_sum` counted
  a `Lot(None)` copy only when it was over the cap. A refused copy now keeps today's head as an over-cap one does
  (`copy_suspect`). Pinned in `the_awards_path_refuses_every_shape_its_guards_name` (shape C: EUR 1.2 bn + EUR
  1.2 m with copies of both stays EUR 1.2 m, and the test asserts the copy is refused at `Lot(None)`). Recorded in
  Unit 2's guards.
- **V0.** Path V had no raise-only check, so 537's recorded-cost shape fell to a x100 understatement marked vouched
  and `bounded` was lowered from EUR 10 m to EUR 3 m. `vouched_lot_sum` now keeps today's figure unless S_eur is
  above it. Pinned on 537's own `cost` and `bounded` shapes (the test asserts `head_value` there) and on a minimal
  shape in `the_vouched_path_takes_only_the_sum_its_refusal_names`. Recorded in Unit 2's guards.
- **The SSE At probe (blocking).** `ValueBy::Version` read an earlier version as "an amount row inside the bound",
  which a derived head never is. A later version moving a derived head out of a bound probed `(false, false)`,
  the change was dropped, and the subscriber kept a ghost. Its ELSE leg now also accepts the version's lot figures:
  under `>=` their total (`SUM(eur) + COUNT(*)` over the version's positive lot amount rows, or over its lot
  results; the `COUNT` is one cent a row of conversion rounding), and under `<=` any one of them. Each leg seeks
  `(tender_id, seq)` and carries the head guard. Pinned in store's
  `an_earlier_derived_head_is_read_inside_the_bound_it_was_in` (path W under `min_value`, path V under `max_value`
  and a band). The At plan test now also checks the `vr` (lot results) leg, and the statements were plan-probed on a
  copy of `planchk/schema.db`: no SCAN.
- **G8b (second round, 2026-10-16; replaces round 1's accepted double count).** A re-keyed chain whose BOTH key
  families are typed Lots passed G8, and path W summed each lot twice. Round 1 accepted that as a recorded cost;
  the drain folds ~195,000 Tenders, 40x the sample that saw none, so path W now fails closed: two distinct summed
  keys that spell one lot number keep today's head (`canonical::lot_number` / `names_one_lot_twice`, read off the
  spellings the walkers write: `r209::parse`'s synthetic `LOT-1..n`, eForms' `LOT-nnnn`, Find a Tender's OCDS
  ids, plus unit 1's `Lot 2` / `LOT2`; leading zeros dropped; a key that is no numbered lot key never trips it).
  Pinned in `the_awards_path_refuses_every_shape_its_guards_name`: LOT-1/2 at EUR 1.0/2.0 m beside LOT-0001/2 at
  EUR 1.1/2.2 m now keeps today's EUR 2.2 m (was EUR 6.3 m over 4 lots), as do LOT-2 beside LOT-0002 alone and
  `1` beside `Lot 1`; the positive controls (LOT-0001/2 alone, LOT-1/2 alone, numbers 1/2/3 across families,
  `1`/`1a`/`1.2`) still sum. `the_awards_path_reads_a_lot_number_only_from_a_numbered_lot_key` pins the reader.
  Mirrored in `merge_rule.py`: `run_py.py` gives output identical to round 1 on all 4,745 records (0 values, 0
  reasons changed; the 21 changes stand). 27 records reach G8b (the 6 path-W changes, 20 G1' keeps, 1 G3 keep)
  and none trips it. 12 heads type two keys of one number (`1` beside `Lot 1`, e.g. 8604792, 8713827), and 0
  award both. Correction to round 1: 6978910 and 7775229 do not type both families. Their four-digit keys are
  the TED walker's own numbering past 999 (`LOT-1000`..`LOT-2604`), which G8b reads as 1000.., apart from `LOT-1`.
  The post-drain read still treats a double count as a finding: G8b cannot see one lot under keys it does not
  read as numbers (`Lot A` re-keyed to `LOT-0001`), nor under numbers that do not line up. TED's `LOT-k` is the
  notice's k-th lot section, not its published LOT_NO (corrected in round 3, below), so a TED CAN describing only
  lots 3 and 5 (`LOT-1`/`LOT-2`) beside an eForms `LOT-0003`/`LOT-0005` passes G8b and double counts, and TED's
  `LOT-1` for lot 3 beside eForms' `LOT-0001` is refused though the lots differ (harmless: today's head stays).
  The `/docs` Amounts caveat (its phrase pinned in
  `the_docs_say_a_head_can_be_a_derived_lot_sum`), CHANGELOG and docs/operations.md name the guard.
- **Section 16 JSON.** `head_value_band.rows[]` gains `derivation`, asserted on the band test's derived row 11.
- **Docs.** The `/docs` min_value/max_value bullet now says the bounds compare the head's EUR value (the lot sum's
  where `value_derivation` is set), and that a Tender with only skipped-class amounts has no known value unless
  its awards are summed. The Amounts caveat names V0 and G5'. A new
  `the_docs_say_a_head_can_be_a_derived_lot_sum` pins one phrase per sentence across `/docs` (both caveat lines
  Unit 2 required), OpenAPI `Tender.value` / `value_derivation` and the five `/v1/sql` `tenders` notes.
- **CHANGELOG.** Webhooks dropped from the surfaces that carry `value_derivation`: a webhook body carries change
  events only. V0 and G5' added to the two paths' descriptions.
- **operations.md / pins.** 5625591 moved to the estimate-only group (it never reaches a path-W guard), and
  5316712 is named as the only real-row G3 case, here and in canonical.rs's G3 comment.
- **Scripts.** `unit3/` no longer reads the session scratchpad. `merge_rule.py` imports `../scripts/elect.py`, and
  the inputs are committed under `unit3/data/`: `ctx.json.gz`, which is unit 1's 4,745-record pull as JSON, 1.3 MB,
  and the 8 API reads. `run_py.py` reads `../results-slim.jsonl`, which matches the old results.jsonl on
  population and stored head for all 4,745. From those inputs, `gen_fixtures.py` regenerates
  `canonical_551_fixtures.rs` byte for byte. `run_py.py` with V0 and G5' added gives output identical to the
  pre-review rule on all 4,745 records, so neither new guard fires there. `port_in.tsv` is generated
  (`export_lines.py [path]`), not committed.

**Review fixes, round 3 (2026-10-17, on the uncommitted build).** Six confirmed findings, each fixed minimally:
- **G5' widened (blocking).** G5' judged only an award's tender-scope copy, at `Lot(None)`; a summed award with no
  copy was never asked. LOT-1 awarded EUR 1.2 bn beside LOT-2's EUR 1.2 m, no copies: a null head became
  EUR 1.2012 bn, and with LOT-2's copy alone EUR 1.2 m was raised to it, in the band 471/537 protect. Round 2's
  carried note said such an award "is never judged by a scale rule"; true of the code, but `ScalePartners` refuses
  it when asked at `Lot(Some(key))` (471, partner LOT-2's award). `awards_lot_sum` now keeps today's head when any
  summed award converts and `refuses_amount("result_value", currency, c, eur, Lot(Some(lot)))` holds. Pinned in
  `the_awards_path_refuses_every_shape_its_guards_name` (no copies: null stays null, with the refusal asserted at
  LOT-1 and not at LOT-2; LOT-2's copy alone: EUR 1.2 m stays). Mirrored in `merge_rule.py` (each summed award judged
  at `E.LOT(lot)`). Recorded in Unit 2's G5' line.
- **G5's lower bound.** Path W tested only the cap and `eur <= today`, so on a null head an S that rounds to EUR 0
  passed and the fold stored `current_value_eur_cents = 0`, which issue 378 reserves for "nothing elected" and the
  `/docs` says never appears. Now `eur <= 0` keeps the head, as path V's `(0, cap]` does. Pinned with IDR 17,000/EUR
  (awards IDR 20.01 + 25.00, with and without copies judged `Zero`). Mirrored in `merge_rule.py` (`s_eur <= 0`).
  Recorded in Unit 2's G5 line.
- **The At probe's sums (read.rs).** The `>=` legs' `SUM(eur) + COUNT(*)` could overflow i64 on an earlier version
  whose lot rows or lot results sum past EUR 9.2e16; turso raises that as an error, the SSE diff loop sends a read
  error and returns, and a resumed stream re-reads the same change. Every lot leg (both `SUM`s and both `<=`
  EXISTS) now counts only rows `BETWEEN 1 AND` `IMPLAUSIBLE_EUR_CENTS` (from 0 since round 4, below): still a superset, since each summand of a
  derived S converts to at most S, and S to at most EUR 100 bn. Pinned in store's
  `an_earlier_version_whose_lot_figures_overflow_a_sum_still_probes` (two lot estimates and two lot awards of
  EUR 4.97e16 at v1, tenders and lots At `min_value` and a band); plan-probed on a copy of `planchk/schema.db`.
- **Value-bound docs.** The `/docs` filter row and OpenAPI `min_value` still said a Tender with no convertible amount
  never matches a value bound; path W breaks that (585917's only amounts are partner-only rows). Both now say the
  bound compares the head's EUR value, the lot sum's where `value_derivation` is set, and that such a Tender matches
  when its lot awards are summed; one phrase of each is pinned in `the_docs_say_a_head_can_be_a_derived_lot_sum`.
  The "nothing this API returns is converted" bullet names the lot sum, and read.rs's value-bound comment is
  current.
- **G8b's ordinal caveat (docs only).** TED's `LOT-k` is the notice's k-th lot section, not its LOT_NO, so G8b
  misses misaligned re-keyings and can refuse distinct lots (harmless). No cheap code fix (the stored key carries no
  LOT_NO). Stated in the `lot_number` and `derive_lot_sum` doc comments, Unit 2's G8b line, round 2's bullet above,
  `/docs`, CHANGELOG and operations.md; the post-drain read now compares each summed TED key against its notice's
  `TED-LOT_NO`.
- **Status line.** Rewritten against main at afdc1dc (552's v12 and 524's v11 are live), and the orphaned fragment of
  an older status under it removed.

`run_py.py`'s loop with round 3's guards (scratchpad `b551r2/fix/run_fix.py`) gives output identical to round 2 on
all 4,745 records: 0 values, 0 derivations and 0 reasons changed; the 21 changes stand. The Rust election, re-run with round 1's temporary
harness (`port_check_551`, inserted and removed again, canonical.rs restored byte for byte), agrees with it on
all 4,745: 0 disagreements, the max election equal on 4,745 and the 21 derived heads equal on path, rule, k, F,
kind, S, currency and n.

**Review fixes, round 4 (2026-10-17, on the uncommitted build).** Two confirmed findings, neither blocking, each
fixed minimally:
- **The At probe counts EUR 0 summands (read.rs).** Round 3's `BETWEEN 1 AND` the cap dropped a lot row stored at
  EUR 0, but each summand of a derived S can round to EUR 0 while S rounds to a cent (G5's floor is on S alone):
  IDR 80.00 + 84.00 at 17,000/EUR derive EUR 0.01 from two EUR 0 awards. After a later version moved the head out
  of a `max_value` bound, the earlier version probed outside the bound its served head was inside, the pair read
  `(false, false)` and the subscriber kept a ghost; under `min_value` the `>=` leg (`> 0` since round 2) served
  `added` where `changed` was due. Every lot leg now counts rows `BETWEEN 0 AND` the cap: under `>=`,
  `SUM(round(x_i)) + n >= sum(x_i) + n/2 >= round(S)` for n >= 1 (each summand and S convert at the same date and
  rate); under `<=` any summand is at most S. Overflow safety and the plans are unchanged (a residual after the
  `(tender_id, seq)` seek; a zero adds nothing to `SUM`). Pinned in store's
  `an_earlier_derived_head_of_zero_euro_summands_is_read_inside_its_bound` (tenders and lots At v1 inside
  `max_value` EUR 1.00 and `min_value` EUR 0.01 after v2 admits EUR 1.5 m). Reach is nil today: only path W in a
  currency above ~2,000 per EUR (IDR, the pre-2005 ROL/TRL), none in the 4,745 records; path V's S is at least
  F/10^6 with F >= EUR 1 bn. read.rs's leg comment and the `ValueBy::Version` doc no longer say "positive".
- **The carried residual-risk sentence (docs only).** It named the per-award check's blind spots wrongly. 537's
  lot-sum rule never judges a lot award: at `Lot(Some(key))` it is reached only through `lot_sum_group_copy`,
  which needs a LotsGroup key and a procedure figure of the head at the award's cents, and path W's head has none
  (case LS: LOT-1's EUR 1,200,000,010.00 award, exactly 10^3 x its two siblings' BT-709 sum, has no 10^k or x100
  partner and is served as EUR 1,200,600,010.00). And a k >= 3 slip WITH an exact partner is still summed when
  another field of the head repeats its cents, 471's corroboration (case S3: v1 estimates LOT-1 at EUR 1.5 m; the
  head estimates and awards LOT-1 at EUR 1.5 bn beside LOT-2's EUR 0.8 m award, no tender-scope copies; the award
  is not refused at `Lot(Some("LOT-1"))` while the same estimate is refused at `HeadLot(Some("LOT-1"))`, and a null
  head becomes EUR 1.5008 bn; with LOT-1's copy the max election itself admits EUR 1.5 bn). Both follow decided
  rules (471's corroboration, 537 judging procedure figures), so no guard changed and `merge_rule.py` is untouched.
  Rewritten in "Open (carried)" below; operations.md's post-drain read now flags a path-W head whose summed award
  a lot figure of the head repeats.

**Open (carried).** The post-drain bounded read (operations.md) before "done"; path V's 471/492 branch rests on
928196 alone; V1 has no observed case (pinned synthetically); the latest award per lot key undercounts a re-run that
reuses the key (5977060, pinned as the accepted "closer"); a BT-118-vouched estimate path (issue 506's unmapped
BT-118) stays a separate question. A slipped lot award, copy or none, is now judged at its own lot by the scale rules
(G5', round 3); what they cannot see is a slip below their EUR 1 bn gate, one with no exact 10^k partner (471's
k >= 3, 492's x100; 537's lot-sum rule never judges a lot award, round 4's case LS), or a k >= 3 one whose figure
another field of the head repeats (e.g. its lot's own estimate, which the head election refuses at `HeadLot`): 471
counts that as corroboration (round 4's case S3). Path W sums such an award unless G1'/G2/G3/G4 or the cap stop it;
the post-drain read of awards heads of EUR 100 m or more is where one would show, and it flags each path-W head whose
summed award a lot figure of the head repeats at the same cents. G8b reads TED's ordinal `LOT-k` as a lot number (round 3): the
post-drain read checks each summed TED key against `TED-LOT_NO`.

## Unit 3 — deployed and drained (2026-10-17)

- Deployed `4e5eaf7` 02:36 UTC (gate marker `gate-v2 4e5eaf7`, 1242 s; the first gate found the stoppable-kinds pin
  missing `refold-lot-sum`, fixed in the same commit).
- `refold-lot-sum` dry 2573: 194,404 Tenders, 598,710 notices, walk 170 s on the reader pool (the size estimate was
  ~195,000, 95 % 129,000–293,000; notices ran 3.1 per Tender against the 1.7 assumed).
- wet 2574 (`expect` 194404): stamped 194,404 epoch-stale, re-queued 598,710 notices, 2 s.
- `project` 2575: 2 h 18 m, NOT the runbook's 6–10 minutes — the incremental pass-1 identity stage alone took 2,513 s
  (598,710 changed notices, 325,955 of them legacy), the sharded pre-pass ~1,630 s, the touched expansion grew the fold
  to 219,278 Tenders / 640,228 notices, and the fold ran ~3,000 Tenders a minute (multi-lot heads). The compare: 179,052
  verified, 15,352 corrected (sums to the stamped 194,404: D7 complete), 108,018 correction rows. The runbook's cost line
  should say ~2–2.5 h for a cohort refold of this shape (to fold into docs/operations.md with the next code change).
- Expected moves were path W ~21,200 (95 % 9,700–46,200) and path V ~16: 15,352 corrected, and the first 50 corrected
  Tenders in salted order are all `lot-sum-awards`.
- Pins (docs/operations.md's Verify list): all 16 path-V heads serve their sums marked `lot-sum-vouched` (rule 537,
  928196 rule 492), the 6 path-W pins `lot-sum-awards`, and the 9 must-stay heads serve no derivation.
- The bounded read's population: 229 derived awards heads of EUR 100 m or more (1 at EUR 10 bn+: 8831876, GBP
  25,999,999,986.00; 20 at EUR 1–10 bn; 208 at EUR 100 m–1 bn), read by a seek on `tenders_current_value_eur`
  (plan-probed), and 50 salted-uniform path-W corrections from `/v1/changes` (cursor 955146761 → 955254530).

## The post-drain bounded read — FAILED (2026-10-17)

Workflow wf_dc85fa7e-c00 (40 readers, a skeptic on every wrong: 105 wrongs upheld of 108 first-read wrongs; evidence
`.scratch/tender-db/551-lot-sum/postdrain/read_result.json`). Each head was read against its notices: the procedure's
stated total, each lot's latest award, distinct lots (TED `LOT-k` is an ordinal: checked against the notice's LOT_NO),
slips, and the figure it replaced.

| population | read | right | neutral | wrong |
|---|---|---|---|---|
| derived awards heads ≥ EUR 1 bn | 21 | 12 | 0 | 9 |
| derived awards heads EUR 100 m – 1 bn | 208 | 108 | 13 | 87 |
| uniform 50 of the 15,352 path-W changes | 50 | 40 | 1 | 9 (18 %; 95 % ~9–31 %) |

Wrongs by class (all 105): `not_additive` 41 (framework ceilings or per-supplier maxima summed across lots that share
one envelope, e.g. 176280, 244952, 524007), `double_count` 28 (one lot under two keys or a re-issued award summed
with its original, e.g. 793898, 794186, 831972), `farther` 21 (the sum is farther from the notice's stated total
than the one figure it replaced, e.g. 6330626, 6421819, 6545090), `slip` 11 (a ×10ᵏ award summed, e.g. 993254,
51385), `other` 4. The sample's wrongs: 249185, 324582, 831972, 5779166, 5976305, 6330626, 6421819, 6545090, 7056700.
The ≥ EUR 1 bn wrongs: 176280, 244952, 524007, 793898, 794186, 993254, 1160515, 5316736, 8638894.

Unit 2's evidence for path W was 10 fires and 0 farther (95 % upper bound 28 %); the drain's 15,352 fires show the
guards (G1'–G8b) do not separate the non-additive and double-counted shapes, which are common at scale and dominate
the large heads, where a wrong sum is most visible (value filters, sorts, the data-quality band).

**Decided (2026-10-17, the owner): switch path W off.** House rule: fail closed — a refusal keeps today's head, and no
sampled re-judge in this project ships at 18 % wrong (524's bar is 1 in 30). Path V (`lot-sum-vouched`: a figure the
buyer published at exactly 10ᵏ the lot sum vouches for it) stays: all 16 of its pins read right, and its cohort is
small. Units:
- **4a (S, build): the switch.** Path W behind a constant that is off in production (`derive_lot_sum` returns no
  awards sum); its guards and tests stay, exercised through the inner function, so the redesign keeps its pins.
  Head-value tests of the path-W pins flip to "keeps the max election, no derivation". Served text (/docs Amounts
  caveat, openapi `value_derivation`, CHANGELOG, operations.md) says path W is withdrawn pending unit 4b. Gate, deploy,
  then `refold-lot-sum` dry → wet (`expect`) → its `project` (expect ~2–2.5 h, and ~15,352 corrections back),
  in a queue gap outside the daily ingest. Verify: the 6 path-W pins serve no derivation; a bounded read of the
  value band finds no `lot-sum-awards` row; the 16 path-V pins unchanged.
- **4b (M, decide + build): a path-W redesign** measured on the 279 labels above (and unit 1's 4,745 records):
  candidates — sum only where a notice states no total AND each summed award is a distinct contractor on a distinct
  published lot number (LOT_NO / BT-137 read, not the key), refuse framework or DPS procedures and multi-supplier
  lots outright, refuse when the sum exceeds every estimate on the head by 10×; adopt only at ≤ 1 wrong in a fresh
  uniform 30 and 0 farther in the ≥ EUR 100 m band.

## Unit 4a — done (2026-10-17)

Deployed `900efeb` (with 560 and 562; gate 1441 s) at 10:50 UTC. `refold-lot-sum` dry 2584 (194,411 Tenders, 598,751
notices), wet 2585 (`expect` 194411), `project` 2586 (2 h 12 m: 179,074 verified + 15,337 corrected = 194,411; 0
relabelled). Verify read: the 16 path-V pins unchanged (`lot-sum-vouched`, rule 537 / 492 for 928196); the 6 path-W pins
back to their max election with no derivation (6450467/69/73 EUR 1,660,661,682.94, 8426318 GBP 2,000,000,000.00, 585917
null, 5995030 PLN 2,721.60); the 9 must-stay heads unchanged; all 50 salted-uniform path-W heads of the failed read serve
no derivation; a bounded seek of the value band (≥ EUR 100 m) finds 0 `lot-sum-awards` rows.

## Unit 4b — decided (2026-10-17, workflow wf_efa005fa-6a8)

Failure-shape pass, per-tender features for all 279 labelled heads (174 cached + 105 fetched; the rebuilt sum equals the
served `lot-sum-awards` value on all 279), an offline scorer over candidate guard sets, a judge, and an adversarial check
(sound: False; its amendments are ADOPTED and override the judge where they differ). Evidence:
`.scratch/tender-db/551-lot-sum/unit4b/` (`shapes.md`, `result.json`, `evidence.tar.gz` with the scorer, extractors
and feature tables; `SHA256SUMS`). Every count below is IN-SAMPLE (thresholds were chosen on these 279).

Shapes (from `shapes.md`): S1a ceiling copied per supplier (26), S1b parallel competing offers summed (10), S2 one
multi-lot contract counted per lot (7), S3a–d TED ordinal `LOT-k` re-key / restated frameworks / merged call-offs /
ITEM collisions (≈25), S4 eForms modification re-keyed (5), S5 procedure total booked as one lot's award (13), S6 nested
lots (2), S7 pseudo-lots (2), S8 ×10ᵏ slip (10), S9 closed lot summed (2), S10 unit rates summed (3), S11 latest-by-id
picks an older notice. The single largest fix would be storing a notice's stated total (II.1.7 / BT-161 / BT-27-Procedure
/ BT-118) as a procedure figure instead of collapsing it into an award copy.

Candidate scoring (uniform 50 / band 229, wrong among what ships): none 9/50, 96/229; best simple union A+B+C+D+E ships
9/50 (0 wrong) and 28/229 (9 wrong); W13 ships 32/50 (0 wrong) but still ships a double count and a slip in the band.

### The decision (judge)

REDESIGN path W as "v2", but measure it before any Rust is built. The Rust switch LOT_SUM_AWARDS_PATH stays false until a fresh sample passes the bar.

v2 is today's T', K and G0–G8b plus ten refusal families, R1–R10. One of them, R10, requires a total the buyer stated whenever the sum is EUR 100 m or more.

How it is tested: a salted shadow walk over the failed drain's 15,352 path-W corrections, scored with the Python port of v2, gives the fresh uniform 30. The band heads v2 still ships (15 in-sample) are re-read in full. Only on a pass do the build units B1–B5 run.

A fallback is fixed in advance. "Vouched everywhere" is v2 with R10 applied at every size. It is tested once, from the same walk, only if v2 fails. If it fails too, path W stays withdrawn for good and 551 closes on path V.

Neither the shapes pass's W13 nor W12 is adopted as scored. On these same labels W13 ships 27915 (a double count) and 51385 (two x1000 slips; its reader calls the sum farther) in the EUR 100 m band. The band set is fixed, so a fresh band read would see both again and fail. W12 still ships 51385.

### The rule (judge, before amendments)

PATH W v2 (`lot-sum-awards`). It runs only where path V does not fire, as today.

KEPT UNCHANGED (canonical.rs awards_lot_sum, store already has the inputs): T', K, G0, G5, G5', G6, G7, G8, G8b, G1', G2, G3, G4.

NEW REFUSALS. Any one of them keeps today's head. "Summed award" means a lot's latest award by notice id, as LotSums builds it today.

R1, a lot won by two or more separate tenders (Bm_g).
- Refuse when a summed lot's result names 2 or more winners and they are not all tenderers of ONE LotTender for that lot in the same round.
- A consortium on one LotTender counts as one tender.
- A TED award block with AWARDED_TO_GROUP also counts as one tender, even with no LotTender rows.
- No LotTender rows for the lot, 2 or more winners and no group flag: refuse.
- Fields: LotResultState.winners and Round.bids (lot_key, parties with role tenderer) are already there. ADD LotResultState.group from TED-AWARDED_TO_GROUP inside the award block. r209 already parses it as a Rule::Marker, so no re-parse is needed.

R2, published lot numbers (C_plus). Read each summed award's published lot number as a set of tokens: leading zeros dropped, ranges up to 50 expanded, lists split. Refuse when:
- (a) two summed keys share a token;
- (b) one award's lot number is a list of 2 or more;
- (c) one key's award rounds name different lot numbers;
- (d) the summed awards come from 2 or more notices and any of them has no lot number.
- Fields to ADD: LotResultState.published_lot_no and LotState.published_lot_no.
  - TED: the LOT_NO / LOT_NUMBER / ITEM value. project.rs's legacy results reader already reads it to resolve lot_key, and then drops it.
  - eForms: BT-13713 as published.
  - FTS: the OCDS lot id.
- Today the stored key is TED's ordinal LOT-k. R2 closes G8b's ordinal blind spot (the S3a–d shapes).

R3, a notice's total below the lots it awards (D_keys).
- Take each round notice that awards a summed key and has an award-type total in S's currency: BT-161-NoticeResult, TED VAL_TOTAL at notice or procedure scope, BT-118-NoticeResult or BT-1118-NoticeResult. A single-lot notice's total equal to its own award does not count.
- Refuse when the summed awards of the keys that notice awards exceed 1.01 × its largest such total.
- With no such total anywhere: refuse when S > 10 × the largest estimate in the chain (BT-27 at Lot or Procedure, VAL_ESTIMATED_TOTAL, framework maximum).
- Fields to ADD: TenderVersion.stated_totals, a list of {notice_id, field, cents, currency, lots the notice awards}, cumulative over the chain like rounds (or read from ScalePartners' chain-wide figures).
  - BT-118 and BT-1118 are parsed but unmapped. They are READ here only, never written as an amount row and never elected (506 stands).
  - Estimates are already there.

R4 (E): a summed award at least 10 × its own lot's largest BT-27-Lot.
- Already there for eForms and FTS lots.
- TED lot estimates are stored with no lot, so R4 never fires on TED. That gap is recorded, not fixed.

R5: the 15 shape checks from the shapes pass. All inputs are already there: rounds, LotState amount facts including the partner-only BT-709 / BT-660 rows (read, never summed), and bids.
1. The summed result's decision starts with "clos", or it has no winner.
2. A later result on the same lot is closed, or has neither a winner nor a value.
3. An award is within 0.5 % of the sum of all lot estimates (at least 2 lots carry one).
4. An award is within 0.5 % of the sum of every lot's figure in one other round, or in one kind (BT-27-Lot, BT-709, BT-660, BT-271-Lot), over at least 2 lots.
5. An award is above 1.01 × the sum of its own valued bids in that round.
6. An award is at least 1.9 × its lot's BT-709.
7. S is above 1.5 × the sum of BT-709 over the summed lots that carry one.
8. An award is at least 1.5 × its lot's smallest estimate.
9. An award is at least 3 × its lot's estimate.
10. S is above 3 × the sum of the estimates of the summed lots that carry one.
11. Two or more equal valued bids equal the lot's BT-709, BT-660, BT-27 or BT-271 (within 0.1 %).
12. An award equals n_winners × one of the lot's BT-709, BT-660, BT-27 or BT-271 (within 0.1 %), with n_winners at least 2.
13. S is below 1,000.00 in its own currency.
14. A procedure estimate equals every lot's estimate (within 0.1 %).
15. All lot estimates are equal (at least 2 lots carry one).

R6 (C_title): two summed keys, awarded in different notices, have the same non-empty winner set and the same normalised lot title. Already there (lot "title" facts).

R7 (P10): a summed award is at least 10 × or at most 1/10 of an earlier valued award on the same key, in the same currency. Already there (rounds).

R8 (DOM10): the largest summed award is at least 10 × the second largest. Already there.

R9 (new; it refuses 27915 and 8597433 and 0 rights): the summed result's round publishes LotTenders, but none for the summed lot. This is the shape where an award is resolved through another lot's contract or tender. Already there (Round.bids lot_key).

R10, the band vouch (new):
- When S_eur is EUR 100,000,000.00 or more, refuse unless some stated total T vouches for S:
  - T is in S's currency and comes from any notice in the chain;
  - |T − S| ≤ 1 % of S;
  - T equals no single summed award, and is not a single-lot notice's own award.
- Kinds accepted: BT-161, TED VAL_TOTAL, BT-118, BT-1118, BT-27-Procedure, TED VAL_ESTIMATED_TOTAL, TED VAL_RANGE_TOTAL LOW/HIGH. Modification VAL_TOTAL_BEFORE/AFTER do not count.
- Fields: the same stated_totals as R3 (ADD).
- The detail JSON gains `vouched_by {field, notice_id, cents}` whenever a vouching total exists, at any size.

NOT adopted:
- A, the framework/DPS flag: 33 more lost fixes, including 8426318 and 585917, to catch one band wrong that R9 already catches.
- B, "one contractor holds 2 or more lots": it hits 101 rights.
- Z / Z_nototal, "sum only where no total is stated": the evidence runs the other way (see the rationale).

PARSE: nothing new. Every fact above is already in the parse layer. The change is in the projection only (ingest project.rs feeding store's canonical types), kept in memory, with no new columns and no schema change.

### Expected

ON THE 279 LABELS (in-sample; every check was chosen on them)
- Wrongs refused: 105 of 105. Neutrals refused: 13 of 14. Rights kept: 46 of 160 (114 lost fixes, 106 of them in the band).
- Uniform 50: ships 32, all right, 0 wrong (exact 95 %: 0–10.9 %).
- EUR 100 m band: ships 15 of 229 with 0 wrong of any class. They are 14 rights (683951, 707424, 726354, 1102571, 1147190, 1148297, 5842187, 6017838, 6057925, 6195903, 6370901, 7001924, 7011414, 8426318) and the neutral 5278856. That is down from 96 wrong under v1.

FIRE COUNT
- v2: about 9,800 heads, i.e. 15,352 × 32/50; 95 % 7,550–11,830. Because the count is in-sample, plan for 6,000–10,000.
- Of those, about 15 sit in the EUR 100 m band; the rest are below it. The uniform 50 held no head at or above EUR 100 m.
- Each fire fixes an understatement of 1.5x to 569x.
- Fallback "vouched everywhere": about 2,450 heads (8/50; 95 % 1,100–4,470), the same 15 in the band.
- Path V is unchanged: about 16 heads.

PINS UNDER v2
- Served:
  - 585917: EUR 250,000.00 (vouched by BT-118);
  - 5995030: PLN 1,549,511.60 (v2's VAL_TOTAL PLN 1,546,390, 0.2 % off);
  - 8426318: GBP 3,000,000,000.00 (vouched by TED VAL_RANGE HIGH);
  - holdout 467176 CHF 3,161,818.00 and 294968 CHF 4,440,992.42.
- Now refused:
  - the Lyon-Turin trio 6450467/69/73 (in the band with no stated total; they keep EUR 1,660,661,682.94);
  - 7256284 and 5977060 (R2/R7);
  - 8831876 (its GBP 26 bn total sits on a separate tender, 8831875).
- Out of sample, v2 ships 5 of the 10 earlier fires, all right.

CHANCE OF PASSING THE FRESH 30
- If 2 % of v2's fires are wrong: 0.88. At 3 %: 0.77. At 5 %: 0.55.
- At the in-sample upper bound of 11 %: 0.15.
- Each of the sample's 9 wrongs is caught by a check with a stated reason, not one added after reading the leftovers: Bm_g, the closed-lot and below-1,000.00 shape checks, D_keys, or C_plus.

MEASURING FIRST
- Expect about 47 ids walked and about 190 GETs (about 5 min at 1 GET per 1.5 s) to collect 30 shipped heads.
- Expect about 36 % of walked ids to be refused.

### Go bar

The bar is fixed and is not relaxed. Path W is switched on only when both of these hold:

(1) At most 1 wrong in a FRESH uniform 30 of v2's fires.
- None of the 30 may be among the 279 labelled ids.
- The salt is recorded in the issue before the draw.
- The draw is the shadow walk in M1, so nothing ships to make it. Walk the failed drain's 15,352 path-W corrections (project 2575, /v1/changes cursor 955146761 to 955254530) in salted order, score each id with the Python port of v2 on public-API facts, and keep the first 30 that v2 ships. v2 only adds refusals to v1, so this is a uniform draw of its fires.

(2) 0 heads labelled farther or double_count when every v2 head of EUR 100 m or more is read in full.
- That is the 15 band heads, re-read blind, plus any band head the walk finds that is not among them.

Both reads use wf_dc85fa7e-c00's rubric and put a skeptic on every wrong.

Order of tests, fixed in advance:
- v2 first.
- If v2 fails, "vouched everywhere" (R10 at every size) once, from the same walk.
- If that also fails, path W stays withdrawn (unit X).

Nothing in canonical.rs flips before (1) and (2) pass on the Python port. After B3, the Rust and the port must agree with 0 disagreements on every id walked or labelled.

After the drain, B5 repeats both checks on the real population: 30 drawn by a new salt, plus the band in full. A failure there switches path W off and refolds back by unit 4a's route.

Stop rules during the drain:
- Corrections outside 0.5x–2x of M1's projection: stop.
- A pin off: stop.
- A band set different from M2's: stop.

### Units

- M1 (S, measure, no shipped code). Port v2 and run the salted shadow walk. Write .scratch/tender-db/551-lot-sum/unit4b/rule_v2.py: the scorer's facts() from d551w/score.py, plus R9 and R10 with the thresholds in the rule. Assert on the 279: 105/105 wrongs refused; the band ships exactly the 15 listed; the sample ships 32; out of sample it ships {294968, 467176, 585917, 5995030, 8426318}. Record the salt in the issue BEFORE drawing: sha256('551-4b-fresh30' + id). List project 2575's path-W corrections from /v1/changes (cursor 955146761 to 955254530, bounded pages) and drop the 279. For each id in salted order: GET /v1/tenders/{id} and each version notice's /v1/notices/{id}/content, at most 1 GET per 1.5 s, a 408 retried once, no ssh. Rebuild S (each lot's latest award, at least 2 lots, one currency). Check G0 against today's served head. Apply v2. Write walk.jsonl (id, S, ship or refuse, first refusing check, vouched). Stop at 30 shipped. Why the draw is fair: v2 only adds refusals to v1, so on the same data its fires are a subset of v1's fires, and taking the first 30 shipped in salted order is a uniform draw of them. The walk misses only heads folded after 2026-10-17; B5's confirmation read covers those.
- M2 (S, read; blocked by M1). Fresh-30 and band read. Run a reader panel as wf_dc85fa7e-c00 did: each head read against its notices, verdict right/neutral/wrong with a class, and a skeptic on every wrong. Read the walk's 30, and re-read the 15 band heads blind to their old labels. Bar: at most 1 wrong of 30, and 0 farther or double_count among the band heads. Record the result in issue 551. PASS: B1 to B5 run with v2. FAIL: test the fallback once. Continue the same salted walk until 30 VOUCHED heads have shipped (vouched heads already read are reused), read only the new ones, and apply the same bar. If it passes, B1 to B5 run with R10 at every size. If it fails, run unit X.
- X (S; only if both variants fail). Close out path W. Change the issue status, the /docs Amounts caveat, the openapi value_derivation text, CHANGELOG and operations.md from 'withdrawn pending unit 4b' to 'withdrawn'. Update the phrases pinned in the_docs_say_a_head_can_be_a_derived_lot_sum. LOT_SUM_AWARDS_PATH and its tests stay. File three issues with shapes.md as evidence: (1) TED lots keyed by ordinal LOT-k instead of LOT_NO (shapes S3a–d); (2) stated procedure totals collapsed into award copies by 492(c) (shape S5); (3) x10^k lot-award slips below the EUR 1 bn gate (shape S8). 551 closes on path V. Expected fires: 0.
- B1 (M; blocked by M2 PASS). Carry the facts v2 reads, in the projection only: no parse change, no schema change, no stored rows. ADD to LotResultState: published_lot_no: Option<String> (TED LOT_NO/LOT_NUMBER/ITEM in the award block; eForms BT-13713; FTS the OCDS lot id) and group: bool (TED-AWARDED_TO_GROUP marker). ADD LotState.published_lot_no (TED lot section LOT_NO). ADD TenderVersion.stated_totals: Vec<StatedTotal{notice_id, field, cents, currency, lots_awarded}>, cumulative over the chain like rounds: BT-161-NoticeResult, TED-VAL_TOTAL at notice or procedure scope, BT-118 and BT-1118-NoticeResult (read only), BT-27-Procedure, TED-VAL_ESTIMATED_TOTAL, TED VAL_RANGE_TOTAL LOW/HIGH. Use serde(default) only: postcard spill rule. Tests in ingest/project.rs: a_ted_award_block_keeps_its_published_lot_number; a_ted_award_to_a_group_is_flagged; an_eforms_result_keeps_its_published_lot_id; a_single_section_ted_modification_keys_lot_1_but_keeps_its_lot_no; a_version_carries_its_notices_stated_totals; bt118_is_read_as_a_stated_total_and_never_elected (amount-row count and head election unchanged, per 506). Store: project_refold.snapshot and fold_writer.snapshot stay byte-identical (nothing stored moves). Run ops/check.sh and read GATE-EXIT.
- B2 (M; blocked by B1). Build the v2 guards in canonical.rs awards_lot_sum, with path W still off. Add R1 to R10 after G4, each as a named pure function. Add vouched_by to the detail JSON. LOT_SUM_AWARDS_PATH stays false. New tests, all run through head_value_with_awards_path: a_lot_won_by_two_tenders_is_not_summed (R1; a consortium on one LotTender and TED AWARDED_TO_GROUP pass); published_lot_numbers_that_collide_or_list_refuse_the_sum (R2 a–d; TED LOT-1 for lot 3 beside eForms LOT-0001 now sums); a_notice_total_below_the_lots_it_awards_refuses_the_sum (R3, including the 10x-estimate branch); an_award_ten_times_its_lot_estimate_refuses_the_sum (R4); the_shape_checks_refuse_their_shapes (R5, one synthetic shape per check, 15); a_lot_restated_under_a_second_key_refuses_the_sum (R6); an_award_ten_times_its_own_earlier_award_refuses_the_sum (R7); a_dominant_award_refuses_the_sum (R8); an_award_resolved_through_another_lots_tender_refuses_the_sum (R9, the 27915 shape); a_band_sum_needs_a_buyer_stated_total (R10: the Lyon-Turin trio keeps EUR 1,660,661,682.94; 8426318 serves GBP 3,000,000,000.00; EUR 99,999,999.99 needs no vouch). Also add the_redesigned_awards_path_keeps_its_labelled_outcomes, on real-row fixtures written by a new unit4b/gen_fixtures.py with the B1 facts: one wrong per shape from S1a to S10 refused, and 585917, 5995030, 8426318, 467176, 294968 plus 3 sample rights served. Update an_unvouched_awards_sum_is_served_under_its_guards and the_awards_path_refuses_every_shape_its_guards_name: 6450467/69/73, 7256284 and 5977060 now refused. Verify with the focused command from CLAUDE.md, then ops/check.sh.
- B3 (S; blocked by B2). Port check, as unit 3 did. Add a temporary ignored harness that runs the Rust v2 against rule_v2.py on the 279, the walk's ids and the 4,745 records. Expect 0 disagreements on ship or refuse, S, currency, n and the first refusing check. Remove the harness and leave canonical.rs byte for byte as before.
- B4 (S; blocked by B3). Served text. Update the /docs Amounts caveat (path W re-enabled with its guards, and the EUR 100 m vouch), OpenAPI Tender.value_derivation (vouched_by), CHANGELOG, and docs/operations.md (the guard list, the shadow walk, the bar). Update the phrases pinned in the_docs_say_a_head_can_be_a_derived_lot_sum.
- B5 (S, ops; blocked by B4). Flip, drain and confirm. Set LOT_SUM_AWARDS_PATH = true, run ops/check.sh and deploy. Run refold-lot-sum dry, then wet with expect, then its project (about 2–2.5 h, in a queue gap outside the daily ingest). Stop rule: corrections outside 0.5x–2x of M1's projection (about 9,800) mean switch off and investigate. Verify: 585917, 5995030 and 8426318 serve their pins marked lot-sum-awards; the Lyon-Turin trio has no derivation; the 16 path-V pins are unchanged; a bounded seek of the value band finds exactly M2's band set. Confirmation read: 30 drained path-W corrections drawn by a NEW salt, plus every band lot-sum-awards row outside M2's set, against the same bar. On failure, take unit 4a's route: switch off and refold back.

### Adversarial check — amendments (ADOPTED)

- R2/B1: published_lot_no takes TED LOT_NO / LOT_NUMBER (award block, and LotState from the lot section), eForms BT-13713 / BT-137, and the FTS OCDS lot id. It NEVER takes TED-ITEM, the ordinal attribute on every r209 section. With no LOT_NO the value is None, so R2(d) fires. Add a B1 test: a_ted_award_blocks_item_ordinal_is_not_a_lot_number (single-section CAN: ITEM="1", no LOT_NO → published_lot_no None). Add 6421819 to B2's labelled-outcomes fixtures as a must-refuse.
- Strike 'TED LOT-1 for lot 3 beside eForms LOT-0001 now sums' from B2. G8b stays unchanged and that pair stays refused. State as an invariant, checked in B3: every v2 fire is a v1 fire, so no guard is relaxed and nothing is re-keyed.
- Freeze one explicit field-to-kind table before the walk, shared by rule_v2.py and B1:
- R3 award totals: BT-161-NoticeResult, BT-118-NoticeResult, BT-1118-NoticeResult, TED-VAL_TOTAL at notice scope of a CAN-type form, and, if kept, TED-VALUE / TED-VALUE_COST on CAN-type forms only.
- Old-vintage TED contract-notice forms (numeric '2', '5', …) are estimates.
- R10 vouches: the rule's list exactly. No TED-VALUE / VALUE_COST / estimate-range kinds (in-sample those vouched 0 right / 2 wrong).
- R4 and R5 estimates: API-keyed lot estimates only.
- Published lot number: no title fallback.
- stated_totals: every version notice, with the same correction semantics in both implementations.
- R9: count only bids with a resolved lot_key.
Re-run the 279 asserts with the table: they still hold (adv/a2.py, 0 flips). Commit rule_v2.py and record its sha256 in the issue next to the salt. Any edit after the first walked id voids the draw and needs a new salt.
- Walk population and preconditions:
- rule_v2.py also ports v1's T', K, G0, G1', G2, G3, G4, G5, G5', G6, G7, G8 and G8b (from merge_rule.py) to API facts.
- An id where v1 does not fire today is dropped and logged, not counted.
- Prefer the ids corrected in BOTH 2575 and 2586 (moved to lot-sum-awards and back).
- Read every version notice of a walked id; no bounded-chain shortcut like the scorer used for 8 chains.
- Compute S_eur from the stored rate at the head date (a bounded /v1/sql read), and log ids within 2 % of EUR 100m (1062828 is one).
- Parity protocol, fixed in advance:
- B3 also runs a projection-parity check: B1's real projection fed the parsed /v1/notices/{id}/content of the 279 and the walked ids, compared field by field against the port's facts that feed R1, R2, R3 and R10.
- Any id where the Rust ships but the port refused is read under the M2 bar before B4.
- Any id where the Rust refuses but the port shipped is dropped from the 30; the salted walk continues until 30 Rust-and-port ships are read.
- B5 then drains an id cohort first (the 279, the walked ids and the px2 fires) and confirms 0 ship/refuse disagreements with rule_v2.py before the corpus refold.
- Reader protocol for M2 and B5: every shipped head gets a skeptic, not only the wrongs. A head counts as right only when both reads agree; a disagreement goes to a third reader. The 15 band heads are re-read adversarially (look for double counts, shared envelopes, x10^k slips), not just blind.
- Band bar (2) becomes: 0 wrong of any class (farther, double_count, not_additive, slip, other) among band heads; neutral allowed. The same applies to B5's band confirmation.
- Cover EUR 10m–100m before the walk, with one of these two:
- (a) apply R10 from EUR 10m. In-sample cost is 0: the sample's only v2 ship at or above EUR 10m, 7647263 (EUR 12.04m), is vouched by TED VAL_TOTAL, and no pin is in that range;
- (b) continue the salted walk until 10 v2 ships in [EUR 10m, 100m) are read, under bar (2).
- Before M1, copy score.py, score_work/pxscripts (extract.py, common.py), shapes_work/features.py, judge/common.py, j6.py, j7.py, and px and px2 features_all.json plus shapes features.json into .scratch/tender-db/551-lot-sum/unit4b/ with their sha256. M1's asserts are run from those copies.
- vouched_by goes only into lot-sum-awards detail. Path V's stored detail stays byte-identical, so the 16 path-V pins produce no corrections in B5.
- Unit X, or a filed issue either way: project.rs read_legacy_results builds lot_by_no from both TED-ITEM ordinals and LOT_NO, with or_insert keeping the first. When a lot section's ITEM equals another section's LOT_NO, an award block resolves to the wrong lot key, for example lot sections published in an order other than 1..n. Record it beside the S3a–d ordinal-keying issue.

### Adversarial check — objections (record)

- B1 and the rule's R2 field text count TED-ITEM as a published lot number ('TED LOT_NO/LOT_NUMBER/ITEM in the award block'). That breaks the rule's own in-sample result. The r209 parser emits TED-ITEM from the ITEM attribute that every Section element carries (crates/ingest/src/r209/parse.rs:344): it is the block's position in document order, not a published lot number. So a single-section TED notice's award block reads as lot '1'. The Python scorer never used ITEM (extract.py files it as 'item'). Re-scoring v2 with ITEM as the fallback (adv/a5.py) ships 6421819, a sample wrong labelled farther. Only R2(d) refused it: LOT-1's notice 21205884 has ITEM 1 and no LOT_NO. The sample goes from 0/32 to 1 wrong in 33 ships, and M1's assert '105/105 wrongs refused' fails. 12 of the 279 hold a summed TED award with ITEM but no LOT_NO, 7 of them wrongs. This is the TED re-keying shape R2 exists to close.
- B2 contradicts the rule. The rule keeps G8b unchanged, yet B2's R2 test says 'TED LOT-1 for lot 3 beside eForms LOT-0001 now sums'. G8b (canonical.rs names_one_lot_twice) refuses exactly that pair, because both keys spell 1. Making the test pass means weakening G8b. Then v2 is no longer v1 plus refusals: it fires on heads v1 never fired on. The 2575 correction set cannot sample those heads, so the walk's 'uniform draw of v2's fires' argument fails, and fails for the TED ordinal shapes (S3a–d) where double counts concentrate.
- The walk cannot tell which ids are path-W fires, and the port as specified does not check. Unit 4a removed every lot-sum-awards row, and /v1/changes rows carry no derivation. So nothing confirms an id in cursor 955146761–955254530 was a v1 fire: the range may also hold path-V heads, incidental corrections or interleaved ingest. M1 builds rule_v2.py as 'the scorer's facts() plus R9 and R10' and checks only G0. facts() has no T', K, G1', G2, G3, G4, G5, G5', G6, G7, G8 or G8b, because all 279 were already served lot-sum-awards. A walked id with an admitted procedure figure (T' fails) or a repeated total (G1') could be 'shipped' by the port, read in M2 and counted as a v2 fire.
- Port-to-Rust parity is not set out fact by fact, and B3 cannot catch the gap. B3 feeds the Rust from API-derived fixtures, so it compares two copies of the decision logic on the same inputs. It never tests whether B1's projection derives the same facts from the parsed notice. Known mismatches between score.py/extract.py and the rule text:
- published lot number: titles are a fallback in the port (title_lot_no) and not in B1; ITEM is in B1 and not in the port;
- lot estimates: R4 and R3's estimate branch use notice-text TED lot estimates (estimate_by_published_lot_no, TED-VAL_OBJECT in lot sections), which the rule says the store does not have;
- stated totals: the port maps TED-VALUE and TED-VALUE_COST at notice scope to result_total and BT-271-Procedure to an award total, and its TED_CN_FORMS has only F-codes, so old-vintage contract notices (forms '2' and '5', e.g. 5561539's notice 18561389) are read as award totals; the rule text lists none of these;
- correction-replacement: stated_totals are 'cumulative like rounds' (rounds have it) while the port has none;
- bids: R9 in the port ignores bids with an unresolved lot.
In-sample none of this changes an outcome (adv/a2.py: 0 flips), but out of sample each one can make the measured ship set differ from the built one. Several of them fail open: R3 missing an old-form CAN total, R2(a) collisions the port found only through titles.
- The bar leans toward passing. Both reads put a skeptic on every wrong but never on a right. The 279 labels were built the same way (108 first-read wrongs, 105 upheld; rights never challenged). A false 'right' is exactly the error that lets a bad rule pass '≤1 wrong of 30'. The band bar re-reads 15 heads already labelled right/neutral with the same rubric, which tests reader consistency, not correctness.
- Band bar (2) bars only farther and double_count. not_additive (41 of 105 wrongs, the largest class and dominant among large heads), slip and other could all ship in the band and still pass. The judge's own case against W13 treats 51385, a slip, as disqualifying. And v2 ships 0 band wrongs of any class in-sample, so the narrower bar buys nothing in expectation.
- Heads of EUR 10m–100m are effectively untested. The uniform 50 held 2 heads there, so a fresh 30 will hold about 1. v1's wrong rate rose with size: 18 % in the sample, 42 % in the band. R10 starts at EUR 100m, so a few hundred visible heads between EUR 10m and 100m ride on R1–R9 alone, with no read aimed at them.
- Overfitting, quantified. R10 is the only refuser of exactly 1 wrong (51385) and costs 60 band rights (W14 ships 74 band rights; v2 ships 14). The R5 checks were picked because each hit 0 rights on these labels. C_title, P10, DOM10, R9 and Bm_g's consortium exception were added after reading the leftover wrongs. 3 of the sample's 9 wrongs are sums under 1,000.00 (324582, 5779166, 6330626). Below the band, the in-sample 0/32 says little; only the fresh 30 measures it. The band is a fixed, fully read population, so it is not overfit in the usual sense, but its 'right' labels never had a skeptic.
- Band membership needs the exact stored rate. The scorer used the label group as the band, while rule_v2.py and the Rust use S_eur at the head's date. 1062828 (CHF 93,401,086.97, band right) is in the band only if CHF/EUR at its head date is at most 0.934, so it sits within about 1 % of the threshold. A port using different rates fails M1's 'band ships exactly the 15' assert, or misclassifies walk ids.
- The evidence the decision names lives in this session's /tmp scratchpad: d551w/score.py, px and px2 features_all.json, shapes_work/features.json, judge/common.py, j6.py, j7.py. That scratchpad is session-scoped. M1's asserts and B3's parity check cannot be reproduced once it is gone.
- Smaller points:
- R5.4 says 'one other round' but the scorer includes the award's own round (no in-sample difference, checked).
- R10's estimate-kind vouches rest on 1 right (7001924). The one in-sample estimate-style match outside the rule's list (5561539, an old contract notice's VALUE) was farther.
- TED-VALUE / TED-VALUE_COST-only vouches were 0 right / 2 wrong (adv/a10.py), so the rule's narrower list is better than the port's, which is one more reason the port must be aligned to it.

