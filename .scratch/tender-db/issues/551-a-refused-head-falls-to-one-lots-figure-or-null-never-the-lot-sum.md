# 551 — when the election refuses a head, it falls to one lot's figure (or to null), never to the sum of the lots

Status: in-progress — **path W v2+G ON since 2026-10-21 10:24Z (e718684)**: B6 U6 froze P (3,467, a2619fd9…) after the 10-21 chain; U7 drained it by tender_ids (stage1 + stage2: 3,467 fired exactly as predicted, Rg identical, band = B_P 29) and the L_changed runs (729 admitted, 8 new fires); U8 verify V1–V5 PASS and the delta read 8 right / 0 wrong. Switch-off twin d569107 gated green (covered deploy). NEXT: U9 14-day watch after each 07:35Z chain (2026-10-22 → 11-04; band, count vs 1.0974 %, day-7 salted 30), then 546 U3's held patch, 539's drain, 561 unit 4. Was: ready-for-agent — path W v2 OFF since 2026-10-19 03:33Z (B5's stop rule; drained back, verified). Investigation read done (confirm 28R/1N/1W PASS no margin, band 9/9, never-measured 29/30). **B6 DECIDED 2026-10-19** (wf_834aaac1-932, panel unanimous + critic; `551-lot-sum/b6/decision-b6.md`): v2 + G (G-single / G-cover, each adopted by a fixed read rule), flipped from an exact predicted set P by tender_ids, no earlier than the 10-21 post-chain gap. NEXT: B6 U1 (port rule_v2_g.py + fetcher) and U2 (fresh public-API facts for the 3,502, ~8 h). Was: ready-for-agent — path W v2 OFF since 2026-10-19 03:33Z (B5's stop rule; drained back, verified). **Investigation read DONE** (wf_eecbe1b7-bc7, 61 heads, reader + skeptic agreed on all): pre-registered confirm bar PASS at its limit (28 right / 1 neutral / 1 wrong of 30), band bar PASS (9/9 new band heads right), never-measured population 29/30 right. The wrong (6465510) is a guard gap: an award-type vouching total from a notice that awards none of the summed lots (proposed G-cover). NEXT: the re-enable decision (panel) — as-is with a re-based window, or G-cover first. Was: ready-for-agent — **path W v2 SWITCHED OFF again 2026-10-19** (B5's stop rule: 3,504 corrections vs window 835–3,338): 36ca974 gated (1,297 s) and deployed 03:33Z, drained back by `refold-lot-sum` tender_ids 2628/2629 + project 2630 (3,502 written, 3,502 corrected), verified (0 of the 3,503 ids and 0 of 35,121 band rows serve `lot-sum-awards`; pins back to pre-551; path V unchanged). Investigation running (wf_eecbe1b7-bc7: the pre-registered confirm 30 + the 9 new band heads + 22 never-measured fires, M2 protocol). Next: the read's result → re-base the projection → decide whether v2 goes back on (and with what window). Was: ready-for-agent — **B5 STOP RULE TRIPPED 2026-10-19 02:35Z**: corpus drain project 2627 corrected 3,429 (+ the cohort drains' 75 = 3,504; window 835–3,338), every correction a `lot-sum-awards` fire; the band holds the 20 plus 9 new heads. Per runbook step 7: path W switched OFF again (code wf_91677ed3-a17 → gate → deploy → `refold-lot-sum` by tender_ids over the 3,502 served heads), then the investigation (pre-registered confirm read of 30 + the 9 band rows, plus 30 from the never-measured population). Pins 31/31 as expected. Was: ready-for-agent — path V LIVE; path W withdrawn (4a done). Unit 4b M1 DONE; M2 FAILED (27 right / 3 wrong); fallback "vouched everywhere" PASSED 2026-10-17 (29 right / 1 wrong of 30, band 15/15, projected fires ≈ 1,669). **B1 DONE** (3c81386) and **B2 DONE** (d4b8d76, wf_6743eeff-da0, gate green 1366 s; path W still off; DEPLOYED 86f2745 2026-10-17 23:00Z — nothing served changes). **B3 PASSED 2026-10-18** (wf_643ad3a2-3ba, after 567/568/570 drained: Rust = remap-aware port on 5,307 rows, projection parity clean, v2 ⊆ v1; `551-lot-sum/b3/B3-result.md`). The drains moved the port's own outcomes: 18 NEW ships, 0 lost; the ≥ EUR 100 m band is now 20 (M2's 15 + 6569442, 6603588, 6679375, 7139761, 7542368). **The 18 READ 2026-10-18** (wf_24a65e6a-e63, reader + skeptic on every head): 18/18 right, band 5/5 right, 0 wrong → reference band set for B5 = **20**. NEXT: B4 (served text) + B5 (flip, cohort drain, corpus drain, confirm; stop window 835–3,338) — and **no lot-key drain may land before B5's band seek** (read18 note 2: 4 partial-sum heads, 6679375 a band head, would leave R10's vouch if their lot-null rows were keyed). Port of record: `b3/rule_v2_b3.py`.
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

## Unit 4b M1 — done (2026-10-17, workflow wf_66e388ca-ef6)

Deliverables in `.scratch/tender-db/551-lot-sum/unit4b/`: `rule_v2.py` (v1's T'/K/G0/G1'/G2/G3/G4/G5/G5'/G6/G7/G8/G8b
ported to public-API facts plus R1–R10 with the amendments: R10 from EUR 10 m, `published_lot_no` never TED-ITEM, G8b
unchanged, v2 ⊆ v1), `M1-port-check.md/.json`, `walk.py`, `walk.jsonl`, `M1-walk.md`, `M1-walk-30.json`,
`M1-fidelity.md` (+ `fidelity/`).

- **Port check (in-sample, a fidelity check of the port, not of v2):** wrongs refused 105/105; the band ships exactly the
  judge's 15; the uniform 50 ships 32, all labelled right; out of sample ships {294968, 467176, 585917, 5995030,
  8426318}; 0 decision differences against the judge's j7 and the amended strict table. v1 fires on all 285 ids and the
  served head equals the port's max election on all 285; 3,106 version notices read. EUR conversion rebuilt from the
  public series (ECB, Eurostat ECU, fixed rates), checked against `/v1/lots` value filters on 21 non-EUR heads (21/21).
- **Shadow walk:** salt 0x551B4 (348,596), order `((id XOR salt) × 2654435761) mod 2³², id` over the failed drain's
  15,352 corrected ids, skipping the 279. 54 walked (238 GETs), 54 v1 fires, 30 v2 ships, 24 refused (first refusing
  check: R3 9, R1 4, R2c 4, R2d 2, R5.1, R5.13, R6, R7, R8 1 each). Projected v2 fires 15,352 × 30/54 ≈ **8,529** (95 %
  6,356–10,605); B5's stop window 0.5×–2× = 4,264–17,058. No walked ship ≥ EUR 10 m (largest 307725, EUR 8.89 m), so the
  band read stays the judge's 15 and the EUR 10–100 m stratum is empty in this draw.
- **Fidelity:** 6 ships + 4 refusals derived by hand with independent code (550 field comparisons, 7 seeded errors all
  caught): 0 mismatches, 0 flips.
- **Recorded deviations:** (1) the salt and the port's sha256 were fixed in the workflow's script (persisted before the
  run) and written into `M1-walk.md` at 15:02Z before the first draw, but NOT into this issue file first; the issue's
  M1 text had named a `sha256('551-4b-fresh30' + id)` order — the salt was chosen without sight of any outcome, so the
  draw stays uniform over v2's fires. (2) The amendment "prefer ids corrected in both 2575 and 2586" was not applied;
  v1 fired on 54/54 walked ids, so no non-fire entered. (3) Notes for B2/B3 from the fidelity read: v1's "latest award
  by notice id" is not always the latest published; eForms "published lot numbers" are the LOT-000N keys themselves (R2
  cannot see a buyer's own part numbers); R3's "keys that notice awards" must be pinned (closed/no-award rows included
  or not); unkeyed modification awards (6094289) must be keyed the same way in B1. (4) 567's fix changes legacy TED award
  lot keys: B3's parity check re-runs the port after 567's refold.


## Unit 4b M2 — FAILED (2026-10-17, workflow wf_88da8542-c46)

Result file: `.scratch/tender-db/551-lot-sum/unit4b/M2-result.json` (per head: reader A, skeptic B, tiebreak C).

- **Fresh 30 (M1's walk):** 27 right, **3 wrong** (bar ≤ 1). Path W v2 fails.
  - **1091419** (other): three contract-modification notices only (subtype 38) for procedure 2022/4741, whose 9-lot
    award sits on twin tender 7405521. S = EUR 499,497.55 covers 3 of 9 lots (procedure award EUR 1,044,065.41) and mixes
    pre-modification values (lots 1, 2) with a post-modification one (lot 8). 2 wrong + 1 neutral; the tiebreak read
    wrong.
  - **6942121** (not_additive / farther): a Swedish ramavtal with three parallel suppliers per lot; S sums the
    "latest" supplier's offer per lot (SEK 951,900) while the notice's PROCUREMENT_TOTAL range is SEK 427,260–538,200.
    The served SEK 492,000 sits inside it.
  - **6759797** (not_additive): a Rahmen-Werkliefervertrag for e-buses; each lot's figure is one vehicle's price
    (CHF 849,767 / 711,683), no quantity or total stated, so S adds two unit rates.
  - None of the three carried a vouching stated total: R10 at every size refuses all three.
- **Band (15 heads, re-read adversarially):** 15 right, 0 wrong of any class. The band half of the bar holds.
- **Per the decision:** the fallback "vouched everywhere" is tested once from the same walk (pre-draw record
  `unit4b/M2-fallback-walk.md`, written before any new id was read). The 5 vouched ships among M1's 30 (7399356,
  6094289, 6527544, 307725, 1042997) were all read right and are reused; 25 new vouched ships are read. Projected
  fallback fires on M1's draw: 15,352 × 5/54 ≈ 1,420 (the decision expected about 2,450).

## Unit 4b M2 fallback "vouched everywhere" — PASSED (2026-10-17)

Pre-draw record, walk and read: `.scratch/tender-db/551-lot-sum/unit4b/M2-fallback-walk.md` (written 16:10Z before
any new id was read); items `M2-fallback-items.json`; read `M2-fallback-result.json`.

- **Walk:** the same salted order continued from position 54 to 276 (277 walked, 1,349 GETs): 134 v2 ships, 142
  refused, 1 dropped. 30 vouched ships (5 reused from M1, all read right in M2; 25 new). Projected fires: 15,352 ×
  30/276 ≈ **1,669** (decision: about 2,450). No vouched ship reached EUR 100 m (largest EUR 22.54 m), so M2's band
  read stands.
- **Read (M2's amended protocol, wf_03469a70-c1b):** new 25 → 24 right, 1 wrong; **29 right, 1 wrong of 30** (bar ≤ 1).
  Band 15/15 right.
- **The wrong, 5816379 (farther):** the vouching total (2017 award notice VAL_TOTAL 472,520 = S) is superseded by a
  2018 re-award of lot 2's scope (head notice 19891093, LOT_NO 0, stored lot-less, VAL_TOTAL 322,400, served today).
  Shape for B2 to weigh: a vouch from an earlier award notice while a later award notice in the chain states its own
  different total. Recorded only; the bar is not re-scored.
- **Per the decision:** B1 to B5 run with R10 applied at every size. B2's `a_band_sum_needs_a_buyer_stated_total`
  becomes "every awards sum needs a buyer-stated total" (EUR 99,999,999.99 no longer exempt); B5's stop window is
  0.5×–2× of 1,669 = 835–3,338; B5's confirmation read keeps the same bar.
- **Order:** 567's refold (legacy TED award lot keys, deployed b957661) drains first; B3's projection-parity check
  re-runs the port after it, as M1's note (4) says.

## Unit 4b B1 — built (2026-10-17, wf_443c0ebc-b4b)

Projection only; nothing stored moves (fold_writer and project_refold snapshots byte-identical; head election and
amount rows unchanged; BT-118 / BT-1118 never elected). Fields: `LotResultState.published_lot_no` / `group`,
`LotState.published_lot_no`, `TenderVersion.stated_totals: Vec<StatedTotal>` (`notice_id`, `logical_notice_id`,
`field`, `kind: Award | Estimate | Range`, `cents`, `currency`, `lots_awarded`). Seven B1 tests plus the bucket codec
round trip; the review's three test findings (currency unpinned, the correction gate untested, 6094289's F20 shape
misdescribed) were fixed, each checked by a deliberate break.

Notes for B2 and B3 (the implementer's judgement calls, accepted):
- `StatedTotal` carries two fields beyond the spec: `logical_notice_id` (the bucket fold path rebuilds a notice with
  `logical_id: None`, so the correction rule travels on the total) and `kind` (whether `TED-VAL_TOTAL` is an award or an
  estimate depends on the notice's `TED-FORM`, which B2 cannot see).
- eForms-DE 1.x notices state NO totals: the projection has already renamed `DE1-NoticeResult-TotalAmount` to BT-161,
  and rule_v2.py reads the published `DE1-*` ids and never sees them, so the dialect is excluded for parity.
- **R2's published lot number:** B2 reads a summed award's number from the `LotState.published_lot_no` of the version
  its round's notice caused, falling back to `LotResultState.published_lot_no`. That reproduces the port's precedence
  (lot section first, award block second); without it, a block placed by its ordinal in a notice whose every section's
  number is its ordinal gets `None` from B1 but the section's number from the port.
- 6094289's F20s: the blocks stay unkeyed because the section prints `3.` and the block `3` (567's
  `normalise_lot_number` strips separators only after a Lot/Los/Lote/Lotto word); B1 keeps `"3"` on the result and
  `"3."` on the lot and re-keys nothing.
- Values are kept raw (no trim, a blank stays a value); field presence decides, as in the port.

## Unit 4b B2 — done (2026-10-17, wf_6743eeff-da0, d4b8d76)

Guards R1–R10 (R10 at every size) in `awards_lot_sum`, path W still off; `lot_sum_decision` names the refusing check in
the port's terms (`V`, `V0`, `V1`, `W/off`, `W/T'`, `W/K`, `W/G0`…`W/G8b`, `R1`…`R10`); `vouched_by` in the
lot-sum-awards detail only. 28 real-row fixtures (`canonical_551_v2_fixtures.rs`) from `unit4b/gen_fixtures.py` over a
committed, sha256-pinned input bundle (`fixtures_data/inputs.tar.gz`, byte-identical regeneration; `--all-port-check`
re-decides all 285 port-check ids: 26 former ships now refuse at R10, 25 stay ships, every refusal keeps its first
check). A temporary harness ran the Rust against rule_v2.py on all 562 cached ids: 0 disagreements on ship/refuse, S,
currency, n, vouched field and first refusing check. Review (parity / invariants / tests): 5 findings fixed — path V's
refusal names (`W/T'` where a refused figure names no sum, `V0`, `V1`), R5 thresholds pinned at-edge and past-edge, R7's
currency filter and R3's procedure-estimate source tested, generator inputs committed; 22 deliberate mutations each
now caught.

Known B3 name-level differences (decisions agree): `vouched_by` names the first vouching total in B1's order where the
port groups by field; `lot_number_tokens` takes ASCII digits only; equal-rank lot titles tie to the first in Fact
order. R10's "equals a single summed award" clause can never be the first refusal (R8 fires first); R5.9 is first only
with a zero estimate.

## B3 — port parity PASSED (2026-10-18, workflow wf_643ad3a2-3ba)

Run after 567 (2590), 568 (2593/2594 + unit 8) and 570 drained (combined 2616/2617). Evidence: `551-lot-sum/b3/`
(`B3-result.md`, `B3-result.json`, `results/B3-parity.json`, `results/B3-movers.json`, the harness sources under
`harness/`). The harness was temporary: `crates/` is byte-identical to HEAD (verified twice, by the implementer and an
independent re-run that reproduced every output byte for byte).

| population | compared | agree | ships port / Rust | v1 fires port / Rust |
|---|---|---|---|---|
| labelled 279 | 279 | 279 | 30 / 30 | 276 / 276 |
| out-of-sample 6 | 6 | 6 | 2 / 2 | 6 / 6 |
| walked 277 | 277 | 277 | 41 / 41 | 273 / 273 |
| records 4,745 | 4,745 | 4,745 | 0 / 0 | 6 / 6 |

- 0 disagreements on ship or refuse, S, currency, n, the first refusing check, `vouched_by`. Every v2 fire is a v1
  fire (also against the pre-drain v1).
- Projection parity (B1's real `NoticeState::read` → `fold` on parsed contents of the 562 Tenders vs the port's
  facts): `LotState.published_lot_no` 26,764/26,764, lot key sets 4,416/4,416, `LotResultState.lot_key` /
  `published_lot_no` / `group` 6,699/6,699 each, `stated_totals` 1,737/1,737. Only `LotState.kind` differs on 122 DE1
  rows (the fold's `normalise_de1` rename; not a B1 field).
- **Port of record from now on: `b3/rule_v2_b3.py` (sha256 5400bc555f15…) and `b3/gen_b3.py` (2e9635bae0a4…)**,
  written by `b3/make_port_copy.py` from `unit4b/rule_v2.py` (39be6d1e…): `pub_no` reads the lot section whose 570
  remap target is the served key (`combined_port.remap_chain`, pinned); R10 at every size; FTS `is_legacy` ignores
  `TED-VAL_TOTAL_TAX_BASIS`. Without the remap edit the port moves 20 decisions (6 of them Rust ships). B5's cohort
  check uses these copies.
- 7511890 (named in operations.md's 570 block, not in B3's populations), checked separately: port and Rust agree,
  22980479 RES-1 on LOT-3 with number "3" (its own LOT-1 moved onto LOT-3), W/T' (not a v1 fire), S PLN 339,546.48,
  n = 4. Add it to B5's id cohort.
- Coverage limits: projection parity does not cover R1/R9's winners and bids (both sides read the served ones); the
  4,745 records mostly test the v1 guards (0 ships, 6 v1 fires).
- **The drains moved the port's own outcomes (both sides agree on the new state):** 18 new ships, 0 lost. Labelled:
  5985466 and 6603588 (labelled double_count on their OLD sums: 124.8 m over 4 lots → EUR 96.0 m over 3; 117.2 m →
  EUR 154.6 m), 6569442 (neutral; CHF 244.7 m), 6595330, 6679375, 7139761, 7542368 (labelled right; 7139761's S
  changed). Walked: 5862601, 5888594, 6317191, 7153588, 7177672, 7298633, 7398935, 7475504, 7722197, 7789396, 8371658.
  No longer v1 fires: 5571139, 5669744, 6852738, 6969157, 7372694, 7495109. **Band (≥ EUR 100 m) = 20**: M2's 15 plus
  6569442, 6603588, 6679375, 7139761, 7542368 — B5's stop rule "a band set different from M2's" would trip, so these
  are read under M2's bars BEFORE B4 (the 5 band heads: 0 wrong of any class; all 18 read, every ship with a skeptic).

## The 18 post-drain new ships — read (2026-10-18, workflow wf_24a65e6a-e63)

`551-lot-sum/b3/read18/read18.md` / `.json`. Reader and adversarial skeptic on every head; a head counts right only when
both agree (no third reader was needed).
- **18 of 18 right / ok**; 0 neutral, 0 wrong. Band bar (0 wrong of any class): the 5 new band heads 6569442,
  6603588, 6679375, 7139761, 7542368 are 5/5 right → **PASS**. M2's 15 band heads did not move (B3-movers: same S), so
  their 15/15 stand. **B5's reference band set is 20**: 683951, 707424, 726354, 1102571, 1147190, 1148297, 5278856,
  5842187, 6017838, 6057925, 6195903, 6370901, 7001924, 7011414, 8426318 + the 5 above.
- 5985466 and 6603588 (labelled double_count on their pre-drain sums) read right on the post-drain sums: the drains
  removed the double count.
- **Partial sums (right, but fragile):** 6679375 (band; leaves out lot 3's EUR 29.8 m in 20916402), 7153588, 7398935,
  7722197 each leave out an award sitting on a lot-null row of a single-Lot-section notice whose LotResult prints no
  LOT_NO; S equals one notice's VAL_TOTAL exactly, so R10 vouches. A later drain keying those rows (570 unit 7's
  follow-on, 567's single-section no-number placement) would make S the full figure, which no stated total vouches
  within 1 %: 6679375 would leave the band and B5's band stop rule would trip. **So no lot-key drain before B5's band
  seek**; if one lands, re-run the B3 port diff and read any band change. Follow-up idea (not filed): R10 accepting the
  sum of disjoint notice totals.

## B5 tooling and pre-registered draw (2026-10-18, before any B5 read)

- `551-lot-sum/b5/cohort_ids.json`: the cohort-first drain's 563 ids (279 labelled + 277 walked + the 6 px2 ids +
  7511890; sha256 da47ec52…d648). `b5/cohort_parity.py fetch` then `compare` implements operations.md step 3 (a head
  served lot-sum-awards counts as the port's ship when v1 reason is W or W/G0-served, no v2 refusals, served
  cents/currency/lots = S/currency/n); exit non-zero on any ship/refuse, S, currency or n disagreement — and, stricter
  than the runbook, on a served `vouched_by` naming a total the port does not count (B3 found 0). Self-tests 18/18.
- `b5/confirm_draw.py`: the confirmation read's draw. **Salt fixed now: `sha256("551-b5-confirm-" + id)` ascending**,
  over the corpus drain's changed set (cursor range recorded at run time), excluding the 279 labelled and 277 walked ids,
  the first 30 served lot-sum-awards; plus every band row outside the reference 20. Self-tests 17/17.

## B5 — deployed, cohort drained, parity FOUND ONE DISAGREEMENT (2026-10-18 ~20:50Z)

- B4 + B5 code gated (54b9068, 2,380 green) and deployed 4b6a4bd: **path W v2 is ON**.
- Cohort-first drain (`refold-lot-sum` with the 563 `tender_ids`): dry 2620 → 562 of 563 (7511890 not reachable,
  as B3 found); wet 2621 + project 2622: 562 written, 488 verified + 74 corrected.
- Corpus dry 2623: **251,526** Tenders, 759,642 notices (not yet run wet).
- Cohort parity (`b5/cohort_parity.py`, fresh reads; `b5/cohort_parity_result.json`): **562/563 agree, 1 disagree**:
  **179732** is served EUR 173,099.00 `lot-sum-awards` (n = 4, vouched_by BT-161 46689383); the port refuses at v1
  `W/T'`. Its head (notice 46693986) has a WITHHELD tender-scope `result_value` and its four lot awards are the -1
  withheld marker; earlier rounds published the awards. Root cause: canonical.rs files a tender-scope `result_value`
  as an award copy (`FigureScope::Lot(None)`), whose branch drops a Withheld judgement, so T' passes vacuously and G7
  ("a withheld procedure figure — a lot sum would publish it") never sees it. B3 missed it because `gen_b3.py`
  encoded the row so the harness read it as a procedure figure (an input-encoding gap in B3's fixtures). **The corpus
  drain is HELD**; fix in progress (a withheld tender-scope copy refuses path W, as the port does), then gate, deploy,
  re-drain the cohort, re-run parity, then the corpus drain.
- **Fixed and re-checked (2026-10-18 21:44Z):** 5d3fc52 (a withheld tender-scope award copy refuses at W/T', as the
  port does; gate 2,382 green) deployed; cohort re-drain 2624/2625: 562 written, 561 verified + 1 corrected (179732
  back to its null head); **cohort parity 563/563, 73 ships each side** (`b5/cohort_parity_result2.json`).
- **Corpus drain started:** refold-lot-sum 2626 (`expect` 251,526): 759,642 notices re-queued, 251,526 stamped
  epoch-stale; project 2627 running. Stop window for the two drains' corrections together: 835–3,338 (cohort so far
  74 + 1).
- **Confirmation draw's range, fixed now (2026-10-19 01:10Z, before project 2627 finished and before any read):**
  `--range 958764958 <cursor after 2627>` — 958764958 is `/health`'s cursor read right after 2626/2627 were enqueued
  (and still the cursor at 23:47Z, the project in its plan phase). The cohort drains' range is left out: its ids are
  the excluded 279 + 277 bar 7 (the 6 px2 ids and 7511890).

## B5 — corpus drain: STOP RULE TRIPPED (2026-10-19 02:35Z)

- **Counts.** refold-lot-sum 2626 stamped 251,526 (= dry 2623); project 2627 (21:47Z → 02:35Z, 4 h 48 m: the plan phase
  walked all 14,935,382 notices) **248,097 verified + 3,429 corrected = 251,526** (D7 complete). With the cohort drains'
  74 + 1: **3,504 > 3,338**, so the pre-registered stop rule trips (the corpus drain alone, 3,429, is above it too).
- **What the corrections are.** `/v1/changes` 958764958 → 958787061: 3,429 `tender changed` events, 3,429 distinct ids.
  Bounded `/v1/sql` PK reads of all 3,503 corrected ids (corpus + cohort): **3,502 serve `lot-sum-awards`**, 1 null
  (179732, reverted by 2625). So every correction is a path-W v2 fire, and the excess is fires, not other corrections.
  Shape: all `source` ted; years 2016–2026 (2023: 727, 2022: 593, 2026: 507, 2025: 466); vouched by TED-VAL_TOTAL 1,633,
  BT-118 876, TED-VAL_ESTIMATED_TOTAL 604, BT-1118 210, TED-HIGH 79, TED-LOW 53, BT-161 47; S equal to its vouching total
  exactly on 3,118 of 3,502; EUR bands <10k 29, <100k 316, <1m 1,716, <10m 1,122, <100m 290, <1bn 27, ≥1bn 2; currencies
  EUR 2,116, HUF 677, BGN 385, PLN 247.
- **Why 2.1×, first decomposition.** The 1,669 projection was M1's walk rate (30 vouched ships of 276 walked) on project
  2575's 15,352 v1 fires. (1) **Inside that population** (`/v1/changes` 955146761 → 955254530: 15,300 distinct corrected
  ids) v2 fires **2,348**. The rate moved before B5: after the 567/568/570 lot-key drains the port ships **41** of the 277
  walked (B3; operations.md step 2's "41 walked"), not 30, and 15,300 × 41/277 ≈ 2,265 ≈ 2,348. The projection and the
  window were never re-based on B3's post-drain rate. (2) **Outside it**: **1,154 fires** on Tenders that were not 2575 v1
  fires (the cohort grew 194,404 → 251,526 with the lot-key drains and new notices). That population was never sampled
  by M1/M2; only the confirmation read reaches it.
- **Pins (step 5)**: 31/31 as expected (`b5/confirm/pins_post2627.json`; the first check reported the 16 path-V pins off
  only because the served `rule` is the string "537" — pins.py fixed, re-evaluated offline on the same reads).
- **Band (step 5)**: the 20 all present; **9 rows outside the 20** (each a stop-and-read): 8407303 EUR 1,134,251,967,
  6718092 EUR 990,000,000, 7167278 HUF 344,998,891,444 (≈ EUR 866 m), 8460630 EUR 404,568,742, 6548237 HUF 82.5 bn
  (≈ EUR 220 m), 5769515 EUR 131,674,590, 7461921 EUR 122,610,000, 7196993 HUF 43.4 bn (≈ EUR 116 m), 6435162
  EUR 108,666,493.
- **Confirmation draw (step 6, pre-registered salt `551-b5-confirm-`)** on the corpus changed set (958764958 →
  958787061; 3,429 eligible, 0 excluded): held 30 of the first 30 walked (`b5/confirm/confirm_result.json`, predraw sha256
  d8b7d352…): 7456417, 609878, 6779279, 6465510, 6811355, 7319895, 5948260, 482122, 193717, 6420294, 766623, 6979655,
  7561428, 6850557, 6850401, 7157983, 305060, 5936963, 6647432, 7620308, 7749755, 7036030, … (8 of the 30 outside the 2575
  population).
- **Decision: switch off (step 7), as pre-registered**, then investigate. Route: `LOT_SUM_AWARDS_PATH = false`, gate,
  deploy, and `refold-lot-sum` with `tender_ids` = the 3,502 served `lot-sum-awards` heads (under the 5,000 cap) instead
  of the corpus walk: every served `lot-sum-awards` head was written by 2622/2625/2627 (path W was off from 4a's deploy
  until B5's, and no other fold ran since B5's deploy), and each of those changes is a correction in the feed, so the id
  list is complete; the corpus route would plan 14.9 M notices again (~5 h) into the 07:35Z daily chain. Verified after
  by the band seek (no `lot-sum-awards` row) and a PK read of the 3,502.
- **Investigation evidence, snapshotted before the switch-off** (`b5/investigate/`): `served_lsa_rows.json` (the 3,503
  PK rows), `corpus_changes.json`, `p2575_ids.json`, `read_items.json` (61 items: the 30 drawn, the 9 band rows, and 22
  more from the 1,146 outside-population fires not drawn, salt `551-b5-outside-`, so 30 outside reads with the drawn 8),
  `served/*.json` (each item's served Tender).

## B5 — switched off and drained back (2026-10-19 03:31–03:40Z)

- **Code:** 36ca974 (`LOT_SUM_AWARDS_PATH = false`; tests re-pinned to the off state with every v2 expectation kept
  through the test-only on twin; /docs, /v1/sql notes, OpenAPI, CHANGELOG, operations.md; built by wf_91677ed3-a17, two
  review lenses, 7 findings fixed). Gate green 1,297 s, marker `gate-v2 36ca974`; pushed with 541 unit 2 (923172d) and
  deployed 03:33Z (`/health` rev 9529345).
- **Drain back by id** (the 3,502 served `lot-sum-awards` heads, `b5/investigate/served_lsa_rows.json`): dry 2628 "3502
  of 3502 named Tender(s) … 15537 notice(s); every named Tender is in the cohort"; wet 2629 (`expect` 3502) stamped 3,502,
  re-queued 15,537; project 2630 (03:34–03:36Z): 4,146 Tenders folded, **3,502 written / 3,502 corrected**, 0 verified
  (D7 complete; the other 644 were the touched expansion, unchanged).
- **Verified:** PK reads of all 3,503 corrected ids: **0 serve `lot-sum-awards`**, each EUR value null or below the S it
  served (`b5/investigate/post_switchoff_rows.json`). The public band (`min_value` EUR 100 m, 40 pages, 35,121 rows):
  **0 `lot-sum-awards`**. Pins: 585917 null, 5995030 PLN 2,721.60, 8426318 GBP 2,000,000,000.00 (all unmarked, as after
  unit 4a); the Lyon-Turin trio EUR 1,660,661,682.94 unmarked; the 16 path-V pins and the 9 must-stay heads identical to
  the pre-2627 baseline (`b5/confirm/pins_post_switchoff.json`).
- No daily chain ran between B5's deploy (2026-10-18 ~20:30Z) and the switch-off deploy, so the id list was complete; the
  07:35Z chain folds with path W off.

## B5 investigation read — result (2026-10-19, workflow wf_eecbe1b7-bc7)

`b5/investigate/read/read.md` / `read.json`. 61 heads (the pre-registered confirm30, the 9 band rows outside the 20, 22 more
from the 1,154 never-measured fires, salt `551-b5-outside-`), each read against its notices from the served snapshot,
reader + adversarial skeptic; they agreed on all 61 (no third reader).
- **59 right, 1 neutral (7749755), 1 wrong (6465510, class other).**
- **(A) confirm bar (≤ 1 wrong of 30): PASS, no margin** — 28 right, 1 neutral, 1 wrong (exact 95 % 0.1–17.2 %).
- **(B) band bar (0 wrong of any class): PASS** — 9 of 9 right (8407303, 6718092, 7167278, 8460630, 6548237, 5769515,
  7461921, 7196993, 6435162). With read18's 20, all 29 band heads have read right.
- **(C) never-measured population (not pre-registered)**: 29 right, 1 wrong (6465510) of 30. Inside-2575 confirm30
  items: 21 right, 1 neutral, 0 wrong. outside22 + band9: 31/31 right.
- **The wrong, 6465510**: S EUR 162,020.92 (LOT-2 + LOT-6, ~19 % of ~EUR 860 k awarded over 20 single-lot award notices)
  vouched by TED-VAL_TOTAL EUR 161,290.32 on 20772908 — lot 4's own single-lot award, lot 4 not in S, 0.45 % from S by
  coincidence. R10's "not a single-lot notice's own award" exclusion compares T with the stored award row, and 19 of the 20
  2019 award results are stored lot = null / awarded = null, so it could not see it.
- **The neutral, 7749755**: S vouched by the same notice that awards exactly the three summed lots (an identity vouch);
  two more awards sit on lot = null rows; S covers 49.9 % of the awarded total, the head without W 39.0 %.
- **Guard-shaped pattern (judge, measured on the 61):** G-cover — an award-type vouching total (TED-VAL_TOTAL, BT-161,
  BT-118, BT-1118, TED-LOW/HIGH) must come from a notice that awards at least one summed key: refuses exactly 6465510 and no
  right. Not yet sized on the 3,502 or checked against the Rust `stated_totals` facts. The partial-sum family (lot = null
  awards outside S: 6779279, 6850401, 7698260 right, 7749755 neutral) is better fixed by placement + disjoint-total vouching
  than by a refusal (a refusal costs 3 rights for 1 neutral). 6908248 is right only while 22266591's RES-2..10 stay lot =
  null (a future lot-key drain would make it a double count R3 should catch: pin it before any such drain).

## B6 — decision (2026-10-19, workflow wf_834aaac1-932; recorded BEFORE any new fetch or read)

Full record: `551-lot-sum/b6/decision-b6.md` (three judges — risk-first, value-first, process-integrity — all chose
**g-cover-first**; the synthesis; the critic's amendments, all adopted). Binding points:

1. **Why the stop rule tripped: our process error.** B3 had recorded 41 ships of 277 walked after the lot-key drains (M1:
   30), and the window was never re-based (15,300 × 41/277 ≈ 2,265 vs 2,348 seen inside 2575's population). From now on any
   B3-type rate change re-registers the window. **confirm30 stands as recorded (28/1/1, PASS no margin)** and is never
   re-scored under a new guard; outside30 is an investigation measure only; G gets no credit on the 61.
2. **Two candidate guards, wording fixed now, no rewording after seeing their sizes:**
   - **G-single** (the repair of R10's existing "not a single-lot notice's own award" clause): a stated total from a notice
     whose round in this version has exactly ONE LotResult (keyed or not) does not vouch.
   - **G-cover**: a total of kind Award (BT-161, BT-118, BT-1118, TED-VAL_TOTAL on a CAN-type form) or Range (TED-LOW,
     TED-HIGH) vouches only if its notice names at least one summed key — on R3's own accessor (`view.on_lot(key)` holds a
     row of that notice; port: R3's `results_by_lot`), keyless results excluded. Estimate totals unchanged. Port kinds:
     Range = `field in ('TED-LOW','TED-HIGH')`, Award = `r3_kind == 'award'`.
   - Both only filter R10's candidates (refusal name stays R10; harness-only diagnostic `g_removed`). v2+G ⊆ v2 ⊆ v1.
3. **Adoption rule (monotone, one rule for any size).** R_single = the ships G-single refuses; R_excess = the ships G-cover
   refuses beyond G-single. For each set: read min(|R|, 30) under M2's protocol (salt `sha256('551-b6-gcost-'+id)`
   ascending; ids already read keep their recorded verdicts); adopt the guard iff **≥ 10 % of its read set is non-right**
   (wrong or neutral; ≥ 3 of 30, ≈ 3× the 2/61 background). Otherwise reject it and flip without it.
4. **Measurement, both sides with parity.** U1 `b6/rule_v2_g.py` (from `b3/rule_v2_b3.py`) + a fetcher (from
   `b5/cohort_parity.py`); tenders read ONLY from a fresh U2 layer, notices from U2 then the M1/B3/bundle layers; rates
   refreshed and their sha256 recorded. U2 fetches the 3,502. **v2 parity:** the port's v2 must reproduce
   `served_lsa_rows.json` (ship, S, currency, n; `vouched_by` by membership in the port's vouch list) on every id outside the
   provisional L_changed (ids with a change row from 958787061 to that id's own fetch time, or a notice folded since
   2026-10-18 20:30Z — fixed per id at fetch time, before comparing). U3 Rust v2+G vs the port on B3's 5,307 rows, the 563
   cohort, the 61 and fixtures of the 3,502: 0 disagreements, plus projection parity; an end-to-end test that real contents
   of 6465510 refuse at R10 through `NoticeState::read` → fold → `head_value_with_awards_path`.
5. **Code drift.** Every commit touching crates/ingest or crates/store between the deployed 9529345 and the flip rev is
   listed in this issue. **571 unit 2 (21b2537) changes result winners (R1/R5.1/R5.12/R6 inputs): its drain lands before U6
   and its whole refolded id set joins L_changed.** A projection-only change (stated_totals, published_lot_no, group,
   unserved rounds) writes no change row: if one lands, the id list is incomplete and the corpus walk is required.
6. **Zero-drift probe.** After U3's W=false deploy: `refold-lot-sum` with the 3,502 tender_ids, wet, expect corrected = 0;
   any correction is drift, explained before U6.
7. **P and the fresh read (B-fresh, sequential).** P = the port's v2+G ships among the 3,502 (exact ids with S, currency, n,
   vouched_by; re-predicted on refetched facts for L_changed ∩ 3,502 at U6). 30 from P minus every read id (salt
   `551-b6-fresh-`): 0 wrong → go; exactly 1 → 30 more in the same order with 0 wrong; ≥ 2 → keep off, re-decide. Readers
   get the port-predicted S, summed keys and notice ids, n and the G-filtered vouched_by, plus the live head. Estimate vouches
   from a single-lot-section CN whose lot is not in S are recorded in U2 and flagged to readers (the estimate twin of
   6465510).
8. **B-band.** Re-read any of the 29 band heads whose predicted S, n or vouched_by changed, and every band row from L_changed
   or the delta (0 wrong of any class). D4: an id within 2 % of EUR 100 m that differs from B_P is stop-and-explain, not a
   switch-off.
9. **Drain, staged, by id list (no corpus walk).** Stage 1: the 61, the pins, 7511890, every id in Rg, a salted 300 of P;
   check D1–D4; stage 2: the rest; then L_changed runs. D1/D2 are measured on a PK-snapshot diff of the named ids (the job's
   `corrected` = |P| + probe drift); a fire in the touched expansion is judged under D5/D6. D5 = PK-read every id in the
   project's change-row range. Stop rules D1–D7 as in the record (exact set, tolerance 0; pins; band; scope; L_changed
   rate max(2 × proj, proj + 10); time).
10. **Timing.** Flip no earlier than the 10-21 post-chain gap (10-20 only if 560 records U-d PASS and closed on the 10-20
    morning and 562 unit 5 has its cursor ranges). Never clock-driven; a missed bar waits for the next gap.
11. **Switch-off route** = a pre-gated W=false twin on the currently deployed base (re-gated before each gap) + `refold-lot-sum`
    by tender_ids over drain ids ∪ every id with a change row since the flip cursor that serves lot-sum-awards.
12. **14-day watch** after the flip (band rows read; daily count vs the L_changed-observed rate on the chain's folded Tenders;
    day-7 salted read `551-b6-daily-`) and the standing rule: no drain or deploy that changes path-W inputs (lot keys, winners,
    org merges, rounds, stated totals, published lot numbers) while W is on without re-running the port diff on the served set
    first (pins: 6908248, the partial sums 6679375, 7153588, 7398935, 7722197, 6779279, 6850401, 7698260).
13. **Accepted residuals:** 5816379's superseded-vouch shape (~1 in 60, ~1.5× errors) stays unguarded and is filed; partial
    sums stay; subscribers see the |P| heads' third correction (on, off, on) — noted in CHANGELOG/operations.md at the flip.
14. **Salts:** `551-b6-gcost-` (guard cost reads), `551-b6-fresh-` (pre-flip read), `551-b6-delta-` (new fires after the
    drain), `551-b6-daily-` (day-7 read of daily-derived heads).
- **B6 code-drift ledger (point 5), as of 2026-10-19 05:55Z:** deployed 9529345 → 3f32b59 adds **21b2537** (571 unit 2,
  result winners: served data). Its drain 2635/2636 is done (66 corrected, ids in `b6/l_changed_571_ids.json`, cursor
  958809905 → 958810037); it changes served winners only, so its corrected set is its whole fact-changed set. 541 unit 2
  (923172d) shipped before the switch-off deploy; its 2632 corrections (cursor 958809778 → 958809905) also join L_changed.

## B6 U1 — port of record built (2026-10-19, workflow wf_284cb847-3f6; recorded BEFORE any U2 sizing run)

- `b6/rule_v2_g.py` **sha256 23cbd8508a3afa35b195f8a2ee91a75bb3b8f08aa2e2f3f9abbd0ea80454452f** (generated by
  `b6/make_rule_g.py` 7d154afc… from `b3/rule_v2_b3.py` 5400bc55…, exact single-occurrence replacements, `--check` MATCH);
  `b6/fetch_b6.py` 33977d77…; `b6/selftest_g.py` ef6c6fe6…; rates files (m551/rates, 10-17 set): eurofxref-hist.zip
  8e925cef…, ecu-ert_h 47e94127…, ecu-ert_bil 48e2c9f1… (to be refreshed before U6, critic A5).
- The rule reads tenders ONLY from the fresh layer (`scratchpad/b6/tenders`, gzip + sidecar with the millisecond fetch time of
  the successful attempt), notices from `b6/notices` then the M1/B3/B5/rb8 layers; a miss raises; no network code.
  `decide(id, guards)` over frozenset() / {single} / {cover} / {single, cover}; guards are pure functions (`guard_kind`:
  Range by field TED-LOW/HIGH, Award by r3_kind; `cand_flags`); both only filter R10's candidates (refusal name R10;
  diagnostics `g_single_removed`, `g_cover_removed`).
- **Self-tests PASS** (T0 18/18 mechanics + 2 guard-disagreement cases; T1 6465510 v2 ship → R10 under single/cover/both;
  T2 4/4 identity vouches keep S; T3 60/60 others unchanged; T4 port v2 = served_lsa_rows on 61/61, vouched_by by
  membership; T5 pins 3/3 keep S, 8426318's candidates are Range; T6 the 4 partial sums keep S; T7 6595330). Fault
  injection (Range by r3_kind, g_single = not names, cover on award only) makes the suite fail.
- **First recorded cost (not a bar):** G-single refuses **6595330** (one of read18's 18 rights: its only candidate,
  TED-VAL_TOTAL on 20747457, comes from a notice with one LotResult — which names a summed key, LOT-2); G-cover keeps it.
  Among the 70 self-test ids: single refuses {6465510, 6595330}, cover refuses {6465510}.
- The small fetch (70 tenders, 372 notices) completed: 0 misses, 0 404s.

## B6 U3 (build part) — Rust guards built (2026-10-19, workflow wf_a98740e3-e1d; commit below, gate pending)

- canonical.rs: `r10_g_single_refuses` (`round_results(head, notice) == 1`, where `round_results` counts
  `AwardsView::results(head)` of that notice: every LotResultState of the version's rounds, keyless included — what
  `write_round` stores one `tender_version_lot_results` row for, so it equals the port's served-row count) and
  `r10_g_cover_refuses` (Award or Range kind and `names_a_summed_key` false, on R3's `view.on_lot`), applied as filters in
  `r10_vouching_total`; with none left R10 refuses under its own name; a guard can shift `vouched_by` to the next total
  (amendment A4). Switches `R10_G_SINGLE` / `R10_G_COVER` = true in code; **path W stays off**, so nothing served moves; the
  flip commit sets them from U4's adoption result. Test-only twins `head_value_with_awards_path_guards` /
  `awards_path_check_guards` evaluate NONE / SINGLE / COVER / BOTH. No projection facts or schema added.
- Tests: 6465510 hand-built from its served rows (v2 ships EUR 162,020.92; single/cover/both refuse at R10, today's EUR
  70,340.42 kept); 6595330's shape (single refuses, cover keeps; plus a variant proving G-single reads the total's own
  notice); identity vouch kept; Range kept only from a notice naming a summed lot; the pins unchanged. v2's own check tests run
  with guards off (`v2_check` → `R10Guards::NONE`); each guard has its own assertions. Focused `canonical::` 73/73.
- **Deviation from point 4, recorded before any U2 result:** the end-to-end unit test from real contents (`NoticeState::read`
  → fold → twin) is not built — there is no content-JSON → `Parsed` loader in crates and the twin is store-private. Its risk
  (the real fold projecting 6465510's keyless rows differently from the served rows the fixture copies) is covered on the
  real store instead: **6465510 is in drain stage 1 and must not fire (D1, tolerance 0)**, and the U3 harness compares Rust
  with the port on fixtures built from the served rows of all 3,502. If stage 1 shows a divergence, the switch-off route
  applies.
- **U3 gated and deployed (2026-10-19):** f41c809 + bcc501a (the second reads `clos-nw` through `DECISION_NO_AWARD` in the new
  fixtures: the first gate's issue-533 tripwire caught three literals). Gate green 1,427 s at bcc501a; deployed 07:39Z after
  the daily chain (rev bcc501a, path W off). Code-drift ledger: 9529345 → bcc501a adds 21b2537 (571, drained) and the U3
  guards (inert while W is off).
- **Zero-drift probe (amendment C2) PASSED:** `refold-lot-sum` tender_ids = the 3,502, dry 2647 / wet 2648 (stamped 3,502,
  15,537 notices) / project 2649: **3,502 verified, 0 corrected**, 0 correction rows, cursor unchanged (958848408). The store
  is at the deployed code's fixed point for the whole set, so the drain's D1/D2 will measure path W and nothing else.
- Today's daily chain (07:35Z: ted 4,029 / doe 601 / fts 14 notices, project 2645: 4,070 written, 102 relabelled, rederive
  0 changed) ran before the probe; its folded Tenders join L_changed by the change-row range 958810037 → 958848408.
- **Code-drift ledger, continued (2026-10-19, after the probe):** bcc501a → dc86e8e adds c09165d (548 rule v13: org heads only,
  no Tender fold input; 127 renames by job 2654 are `organization changed` rows, not tender rows) → 47def3f adds 9ce8649 (539: a
  DÖE notice's published_at is the latest publication candidate raised to its IssueDate's day; the fold re-derives it, so any
  re-fold of a DÖE Tender can re-date it — 539's review measured 0 changed rows on this cohort: 54 DÖE Tenders, 104 DÖE
  versions). 546 unit 3 is held OFF main (`546-triage/u3-42cc601.patch`). Issue 545's rehoming (2651/2652) corrected 47 Tenders,
  listed in `b6/l_changed_545_ids.json` with 571's in `l_changed_571_ids.json`. **Before U6, re-run the zero-drift probe** on
  the deployed rev (refold-lot-sum on the 3,502, expect 0 corrected) so the flip's D1/D2 still measure path W alone.


## B6 U2 — fetch complete (2026-10-19 ~14:00Z)

- `b6/fetch_b6.py` over six windows (exit 124 ×5 at the 2 h cap, then 0): **3,502 tenders, 15,246 notices, 0 × 404, 0 other
  errors, 0 tenders with a missing notice** (the summary's `notices_missing_now: 0` was computed after the rb8 scratch layer was
  deleted for disk, so the rule needs no rb8 notice). Layer: `scratchpad/b6/tenders` (gzip + fetch-time sidecars),
  `scratchpad/b6/notices`. Log: `scratchpad/b6_fetch_u2.log`.
- Next, running: U2 parity (port v2 vs served_lsa_rows outside the provisional L_changed; P, R_single, R_excess for U4) as
  wf_6fc422ac-117 with an adversarial re-derivation; U3's Rust harness waits for the 530 gate to leave the tree.

## B6 U2 — parity measured (2026-10-19, wf_6fc422ac-117; adversarial re-run byte-identical)

- B6 U2 — measured (2026-10-19, wf_6fc422ac-117; the adversarial re-run reproduces all 9 outputs byte for byte). Port b6/rule_v2_g.py sha256 23cbd8508a3a…, matching SHA256SUMS; rates are the m551 10-17 set, not refreshed (8e925cef1572… / 47e941270ac4… / 48e2c9f17ae7…). Fresh layer: 3,502 tenders fetched 05:47:56Z–07:41:30Z (460 of them inside the 07:35Z chain window, a deviation with no effect, since no chain row touches the 3,502). 15,537 notices: 15,246 from b6 and 291 from the M1 cache; none needed rb8; 0 port errors. Provisional L_changed ∩ 3,502 = 0 under the operative reading, which excludes switch-off drain-back 2630's 3,502 version-null rows stamped 03:34:07Z, cursor 958787062–958809773; the literal reading would exempt all 3,502 and compare nothing. The same 2630-excluded reading must be fixed for U6. Feed 958787061 → 958848784 (8,573 tender rows): no other row touches the 3,502. Newest named notice 47406725 was ingested 2026-10-16T07:35:45Z. None of the 3,502's notices is a 541 (TED-COST_PARAMETERS/T02) or 571 (TED-ADDRESS_PARTY) refold carrier, and none of 545's 48 is among them. v2 parity: 3,502 compared, 3,502 agree, 0 disagree (ship/S/currency/n; the served vouched_by is the port's first vouch on all). Guard sets, all ⊆ v2 (3,502): v2+single P = 3,468 (R_single = 34, including 6465510 and 6595330; every removed vouch is an award total from a one-result notice); v2+cover P = 3,489 (13 refused, 12 of them in R_single); v2+both P = 3,467; R_excess = {7202436}; no surviving ship's vouched_by moved. B_P = the 29 read band heads under every set; within 2 % of EUR 100 m: 899272, 5842187. Estimate-twin flags: 0. 5816379 shape (U2's own test, record only): 286 under both guards (144 lot-less). Final P is fixed by U4's adoption. Next: U4 reads 30 of R_single's 34 and R_excess's 1 in sha256('551-b6-gcost-'+id) order under M2's protocol; U3 Rust harness; before U6, add 0787596 to the code-drift ledger and use the whole refolded sets (541: 753/820, 571: 67, 545: 48).
- **Checker corrections, adopted:** (1) the provisional L_changed ∩ 3,502 is **0** under the operative reading (2630's own
  3,502 head-revert rows excluded, as the decision's complement wording does), not "239" (that was the three hand sets outside
  the population); (2) **the 2630-excluded reading binds U6 too** (read literally, every id has a 2630 row and U6 would refetch all
  3,502); the notice criterion means "first ingested since 2026-10-18 20:30Z" (every notice was re-folded by 2627/2630/2648);
  (3) the 541/571/545 sets in the ledger are corrected-only; U6 uses the whole refolded sets (541: 2631 stamped 753 / folded 820;
  571: 2636 wrote 67; 545: 2651/2652 48) — for U2 none of the 3,502's notices is a 541/571 carrier and none of 545's 48 is among
  them; (4) 460 of the 3,502 were fetched inside the 10-19 07:35Z chain window (no pause flag): a recorded deviation with no
  effect (no chain row touches the 3,502); (5) P stays per guard set until U4: v2 3,502 / +single 3,468 / +cover 3,489 / +both
  3,467; (6) 0787596 (530 U1: guid-census, a read-only census; crates/store gains `guid_census.rs` and two `pub(crate)`s, no fold
  input) joins the code-drift ledger.
- **Adoption-rule gap, closed BEFORE any U4 read:** G-cover refuses 13, 12 of them in R_single. If U4 REJECTS G-single and the
  read would adopt G-cover, G-cover is judged on ALL its refusals (the 12 overlap keep their R_single verdicts, plus 7202436): adopt
  iff ≥ 10 % of that 13-read set is non-right. If both are adopted, P = v2+both (3,467); single only, P = v2+single (3,468);
  cover only, P = v2+cover (3,489); neither, P = v2 (3,502).

## B6 U3 — Rust v2+G vs the port: parity PASSED (2026-10-19, wf_2a97436e-983; adversarially re-verified)

- B6 U3 parity PASSED, adversarially re-verified 2026-10-19 15:32Z. Rust v2+G (awards_path_check_guards / head_value_with_awards_path_guards under NONE/SINGLE/COVER/BOTH) equals the port rule_v2_g (sha256 23cbd850…, unchanged) on 37,732 (row, guard set) pairs: 9,433 population rows covering 8,731 distinct Tenders, times 4 guard sets. 0 disagreements and 0 invariant failures. Re-running compare_u3.py on the saved files reproduces u3-compare.json and compare.log exactly, and an independent strict recount agrees. Ships (port = Rust) under v2 / +single / +cover / +both: b3api 73/72/73/72, b3rec 0, c563 73/72/73/72, s61 61/60/60/60, f3502 3,502/3,468/3,489/3,467. The port equals U2's decisions on 3,502/3,502. R_single is 34 (= U2's file), R_cover 13, R_excess {7202436}. 6465510, 6595330 and 7202436 each differ between NONE and BOTH on both sides. Every ship's vouched_by is the port's first kept vouch (14,747/14,747). Fixtures match the served rows in their own layer on all 4,688 API Tenders (lot results, bids, amounts, lots, version chain, stated-total notices and keys). Projection parity is clean: the only difference is LotState.kind (71 on the 3,502, 122 on the 562 and the 563, 0 on the 61), and per-notice result counts are equal. Two harness-side fixes are justified and changed no semantics: the records' withheld encoding now matches the port's _entry, and the projection's chain order was a comparison-input bug. crates/ equals HEAD 8455018 (whose crates/ is identical to ab9aa22), verified twice: git diff --quiet exit 0, empty porcelain status, no u3_harness anywhere. Two record defects remain: export_meta.json mis-records the tender/notice layers for b3api, c563 and s61 (the actual reads are verified correct), and COMPARE-EXIT does not cover projection parity.
- Evidence: `551-lot-sum/b6/u3/` (u3-result.md/json, run.sh, export_u3.py, compare_u3.py, harness/*.rs + insert.py,
  inputs/outputs gzipped with their sha256). The harness was temporary (inserted by `harness/insert.py`, removed); crates/ is
  byte-identical to HEAD (re-checked by the orchestrator: `git diff --quiet HEAD -- crates/` clean).
- Record defects noted, no data effect: `export_meta.json` names the f3502 layers for every population (a shared-list mutation;
  the reads themselves verify per population), and `compare_u3.py`'s exit code covers decisions only — projection parity is read
  from its printed `by_what` (only LotState.kind, the DE1 ProcurementProjectLot→Lot rename B3 recorded).
- The end-to-end test stays the recorded deviation; D1 on 6465510 in drain stage 1 remains its guard.
- Next: U4's adoption reads (running, wf_e231858e-cd0) fix the guard set and P; then U5's fresh read of 30 from P.


## B6 U4 — guard adoption read (2026-10-19, wf_e231858e-cd0; tally `551-lot-sum/b6/u4/u4-result.md` / `.json`)

| guard | read set | right | neutral | wrong | non-right share | bar | decision |
|---|---|---|---|---|---|---|---|
| G-single | R_single, first 30 by `sha256('551-b6-gcost-'+id)` | 27 | 0 | 3 | 10.0 % | ≥ 3 of 30 | **ADOPT** |
| G-cover | R_excess {7202436} | 0 | 0 | 1 | 100 % | ≥ 1 of 1 | **ADOPT** |
| G-cover, gap case (record only, not triggered) | all 13 refusals | 9 | 0 | 4 | 30.8 % | — | (would adopt) |

- Non-rights, each upheld by a reader + skeptic pair: 6465510 (recorded B5 verdict), 6582856 and 7179291 (`skeptic_2.md`,
  `skeptic_4.md`), all wrong/other in the 6465510 shape (a single-lot notice's own award vouching a sum it is not part of);
  7202436 (`skeptic_5.md`). Priors kept: 6595330 right (b3/read18), 6465510 wrong. 7600895 (a G-cover overlap id outside the 30)
  read right, recorded separately.
- **P = v2+both, 3,467 ships** (`b6/u2/P.json`, ids_sha256 a2619fd9e380d3d5…): v2's 3,502 minus R_single's 34 and 7202436.
  The flip sets `R10_G_SINGLE = true`, `R10_G_COVER = true` (both already true in code since f41c809; inert while W is off).
- Sensitivity, recorded: G-single sits exactly on its bar — overturning one of its three non-rights would reject it (2/30), and
  the gap rule would still adopt G-cover (3/13), giving P = v2+cover (3,489). The adoption rule is monotone and was fixed before
  the reads (decision point 3); the cost it accepts is ~27 right ships refused per 30 for the 3 wrongs it stops.
- Next: U5, the fresh pre-flip read of 30 from P minus every read id (salt `551-b6-fresh-`), then the B-band re-read (point 8).

## B6 U5 — fresh pre-flip read: GO (2026-10-19, wf_3e938530-8df; `551-lot-sum/b6/u5/u5-result.md`)

- Read order: P (v2+both, 3,467, ids_sha256 a2619fd9…) minus the 132 P ids carrying any recorded 551 verdict (530 read ids over
  12 sources, `u5/excluded_sources.json`), ordered by `sha256('551-b6-fresh-'+id)` (`u5/order.json`, `build_order.py` asserts the
  P hash and source counts).
- **Round a: 30 right, 0 neutral, 0 wrong → GO** (round b not needed). Live heads (path W off): 7 null, 23 one lot's figure at
  4.7–87.3 % of S. Weak shapes flagged without verdict effect: 7429588 a partial sum not on the pin list (fragile under a future
  single-section placement drain), 6826783 / 6611306 / 7085538 / 6763031 / 7442464 summed at pre-modification values, 5663715
  vouched by Range HIGH, 64472 +0.56 % over its BT-118.
- **B-band (point 8): PASS, re-read set empty** — none of the 29 B_P heads changed S, currency, n or vouched_by against its last
  (right) read, none is in L_changed, no delta before the flip. D4 watch: 899272 (≈ EUR 98.95 m, outside B_P, unread) within 2 %
  of EUR 100 m — a drain-time difference on it is stop-and-explain.
- 134 public GETs, all 200, min gap 2.02 s.

## Zero-drift probe re-run on rev 0787596 (2026-10-19 ~16:35Z)

- After the 548 (c09165d), 539 (9ce8649) and 530 (b81d9ce) deploys: `refold-lot-sum` tender_ids = the 3,502 (identical id set to
  2647/2648), dry 2656 → wet 2657 (re-queued 15,537, stamped 3,502) → project 2658: **3,502 verified, 0 corrected, 0 correction
  rows, cursor unchanged (958848784)**. The deployed code's fixed point still holds for the whole set; the flip's D1/D2 will
  measure path W alone (539's DÖE re-date: 0 rows on this cohort, as its review measured).
- Remaining before U6: the flip commit (LOT_SUM_AWARDS_PATH = true, guards true, CHANGELOG/operations notes per point 13) built
  and gated but held off main until the gap; rates refreshed (A5) and their sha256 recorded; L_changed re-fixed at U6 with the
  2630-excluded reading; the gap = 10-21 after the 07:35Z chain (10-20 only if 560 records U-d PASS and closed on the 10-20
  morning and 562 unit 5 has its cursor ranges).

## B6 flip commit — built and held (2026-10-19, wf_f58c9b34-9a9; patch `551-lot-sum/b6/flip-f0e58cf.patch`)

- Built on base 3df8748 (implement, three lenses — fidelity / docs / regression — a skeptic per finding, fix): 12 findings
  survived, 11 fixed, 1 deferred (a Rust pin on the eight newly served pins 6908248, 6679375, 7153588, 7398935, 7722197,
  6779279, 6850401, 7698260 — a U10 follow-up, the f3502 harness being the measured evidence; generate with
  `unit4b/gen_fixtures.py` into `canonical_551_v2_fixtures.rs` and add to `the_r10_guards_move_no_pin`'s served table).
- Code: `LOT_SUM_AWARDS_PATH = true`, `R10_G_SINGLE` / `R10_G_COVER` stay true; a new `the_production_guard_set_is_both`
  pin and a 7202436-shape G-cover test (`g_cover_refuses_a_total_from_a_notice_naming_no_summed_lot_that_g_single_keeps`);
  tests re-pinned from "path W off" to production == the switch-on twin under BOTH (585917 → EUR 250,000.00, 5995030 → PLN
  1,549,511.60, 8426318 → GBP 3,000,000,000.00; the switch-off twin keeps the max election); head_election_agreement's
  drain-back test now plants a head production refuses (G-cover, a total on a no-round notice). /docs, OpenAPI, /v1/sql
  notes, CHANGELOG (third value correction since B5; 2,334 of the 3,467 also moved by the first drain and 4a's drain-back,
  so their fifth) and operations.md flip to "served again since issue 551 B6"; G-cover is worded as "a result on at least
  one summed lot (one that closes the lot or awards nothing counts)", matching the code.
- Full package suite (the gate's command, unfiltered) green on base 3df8748: 2,456 passed, 0 failed, 34 ignored.
  ops/check.sh NOT run: the commit is re-gated on the final base when it ships (530 Build 1b lands on main first).
- Local branch `hold/551-b6-flip` (f0e58cf); `git apply --check` clean on 3df8748. The held 546 patch (`546-triage/
  u3-42cc601.patch`) applies on top of the flip with a 3-way merge; only CHANGELOG.md conflicts (top-of-file entry order).

## Pre-U6 state (2026-10-20 08:2xZ): code drift, 530's drain, timing

- **Code-drift ledger (point 5 / C1), continued:** 530 Build 2 — bf645ed + c27c8bf (crates/ingest, crates/store), deployed
  d617a78 2026-10-20 08:1xZ. It wires rule 530-T into `mentions_and_dropped` (new twin notices bind by name) and refactors 440's
  `dissolve_condemned` onto a shared `Db::dissolve_org_mentions` (effects unchanged). It changes no fold output for stored
  mentions (`RecordedMention::publishes`; pinned by a test) — the zero-drift probe re-run below measures that on the 3,502.
  Its stock drain (the wet `dissolve-guid-twins`, ~10,995 orgs, org identity = R1/R6 input) runs TODAY, before U6, and its whole
  touched Tender set (`tender changed` rows + the manifest's `refold_tenders`) joins L_changed (C1/C9).
- 560 U-d PASSED and 560 closed on the 10-20 morning; 562 unit 5 PASSED with its cursor ranges — point 10's 10-20 condition
  holds, but the 530 drain uses today's gap, so the flip stays at the **10-21 post-chain gap** (U6 after that chain's
  rederive; U7 with ≥ 2 h before the next 07:35Z).
- Before U6: zero-drift probe re-run on d617a78 after 530's drain + project (expect 0); the flip patch re-gated on d617a78;
  the switch-off twin = the deployed d617a78 (gated, marker c27c8bf); rates refreshed (A5) with sha256.
- **Flip re-gated on the deployed base (2026-10-20):** the held f0e58cf re-applied on d617a78 (CHANGELOG merged newest-first) as
  9f01a5e on local `hold/551-b6-flip-v2`, **gate green at 9f01a5e (1,572 s)**; patch `551-lot-sum/b6/flip-9f01a5e.patch`
  (`git apply --check` clean on main d617a78). The switch-off twin is the deployed d617a78 (its marker was c27c8bf; a switch-off
  redeploy re-gates it).
- **530's wet run is blocked on a permission decision** (its enqueue was refused by the session's permission classifier). Until it
  is allowed and lands, the C1/C9 order is open: either the flip waits for it, or the flip goes first and 530's drain later needs
  the port-diff re-run on the served set (U9's standing rule). The zero-drift probe re-run waits for that choice.

- **530's stock drain landed before U6 (2026-10-21 01:2x–01:3xZ):** wet jobs 2672 + 2673 (10,995 GUID orgs dissolved, 22,056
  mentions moved, 52 minted; change cursor 958904203 → 958919213; 0 Tenders stamped stale), project 2674 (0). Every Tender of
  those `tender changed` rows joins L_changed (C1/C9); 530 is DONE.
- **Zero-drift probe re-run on d617a78 after 530's drain (2026-10-21 ~01:5xZ): PASS.** refold-lot-sum tender_ids = the 3,502
  (`served_lsa_rows.json` minus 179732; same set as 2656/2657): dry 2677 (3,502 in cohort, 15,537 notices) → wet 2678 (re-queued
  15,537, stamped 3,502) → project 2679: **3,502 verified, 0 corrected, 0 correction rows, cursor unchanged (958919213)**. The
  store is at the deployed code's fixed point for the whole set after 530's org moves; the flip's D1/D2 measure path W alone.

## B6 U6 — freeze (2026-10-21 10:13–10:20Z, kit `551-lot-sum/b6/u6/`, runbook `u6/RUNBOOK.md`)

- **Preconditions held.** The 10-21 chain finished 07:49:16Z: probes/process 2680–2685 ok (DÖE "probed 2 day(s), 2 new" — the
  10-20 504 on 2663 caught up), fetch-rates 2686, rederive-eur-recent 2687 (822 stamped), project 2688, reveal-recheck 2689; queue
  empty. `git diff d617a78 fa96ee3 -- crates/` empty; /health rev d617a78; drift 0 (2677–2679); box_health OK; start rule GO.
  Kit fix before use: `u6/health.py` read the cursor from a nested shape the public /health does not serve (it is flat:
  `{"cursor":"…","ok":true,"rev":"…"}`) and printed `cursor=None`; now reads the top-level cursor (nested kept as fallback).
- **C0 = 959004090.**
- **Rates (A5):** `$SP/m551/rates-20261021T101342Z` — eurofxref-hist.zip 4599576d28c5f1ab…, newest fixing 2026-10-20 (7,118
  rows); ecu-ert_bil 48e2c9f17ae7… and ecu-ert_h 47e941270ac4… unchanged. The old set is kept; `u6/rates.json` names both.
- **L_changed = 15,374** (ids_sha256 c0d42241f8f0e7c1…; file 5e609807…), over change cursor (958787061, 959004090] = 217,029 rows
  in 44 PK-range chunks, 2630-excluded. Per source (overlaps allowed): S1 tender rows 15,373; S2 lot rows 12,447; S3 541 126;
  S4 571 66; S5 545 48 (whole); S6 rederive 1,720 (10-19 0 `[START, END)`, 10-20 913 and 10-21 822 `[START, END]`, all
  confirmed); S7 530 2,742; S8 ingest 12,376 (N0 = 47410179 → NMAX 47424229, 14,045 notices, 0 unprojected). Rows by kind/op:
  tender +6,584 / ~18,937 / −37, lot +15,043 / ~63,983 / −97, lot_result +18,263 / −70, bid +44,821 / −9,902, contract
  +23,856 / −51, organization +3,672 / ~718 / −10,995 (530). **37 Tenders retired in the range** (taken out of L_changed, none
  of the 3,502): 1166545 … 8852015 (`u6/l_changed.json` `retired`).
- **L_changed ∩ 3,502 = {8534577}, a 2630 artefact.** Its only rows are 2630's own: the tender row at 958809773 (the recorded
  range end) and its five lot rows at 958809774–958809778 with the same 03:34:07Z stamp and null version — the 2630 range
  really ends at 958809778. The kit's anomaly rule includes them (conservative), so 8534577 was refetched (T_REF 10:18:15Z) and
  re-predicted: unchanged.
- **Deviations from binding text, recorded before the flip deploy (U6.6):**
  (a) **C1/C3 "whole refolded sets" of 541 and 571** — only Tenders with a change row are enumerable: 541's 126 corrected
  (found in (958809778, 958809905]) and 571's 66. 541's 627 verified + 67 touched and 571's 1 verified have no row and no
  bounded enumeration (their carriers are a full notice_texts / notice_ids walk). Accepted because a verified Tender's served
  output compared identical, 541/571 change served result rows / winners only, none of the 3,502's notices is a 541/571
  carrier (U2), and a missed fire only loses a fix. D5 treats them as unchanged: a lot-sum-awards fire on one is a SWITCH OFF.
  (b) **C3 "folded since" read as "first ingested since 2026-10-18 20:30Z"** (U2 correction (2)): S8 = tender_versions of
  notices with id ≥ N0; a notice ingested earlier and first parsed later is invisible per notice, but any fold of one that moved
  served facts wrote a change row (S1/S2).
- **P (port of record 23cbd850…, v2+both, new rates) = 3,467, ids_sha256 a2619fd9e380d3d5…** (P_final.json 9f207e73…):
  replaced 0, same as U2 3,501 (+ 8534577 re-decided identically), S_eur-only moves 0, errors 0. Rg 35 (= U2). **B_P = the 29
  (= U2)**, band re-read set empty; near EUR 100 m within 2 %: 899272 (outside P, watch) and 5842187. Pins: as `u6/freeze.json`
  `pins_expected` (585917 / 5995030 / 8426318 path-W with membership vouches, the Lyon-Turin trio, 16 path-V, 9 must-stay,
  6465510 not lot-sum-awards); all 8 U9 partial-sum/double-count pins are in P. freeze.json 05f45870….
- **L = 3,502 ∪ L_changed = 18,875** (L_ids.json b03cd5c3…). **PK snapshot (pre)**: 18,904 of 18,904 ids (L ∪ pins ∪ 7511890 ∪
  the 61; snapshot_ids 50317688…), 10 statements, missing 0, **serving lot-sum-awards 0**; manifest 290976fc….
- Box reads used so far: 61 (budget 300, ≤ 280/h).

## B6 U7 — plan (2026-10-21 10:20Z, before the deploy)

- **Stages** (`u7/stages.json` sha256 476cd4bc…; order stage1, stage2, lc01–lc04): stage1 431 ids (P 367, Rg 35, B_P 11, pins
  40, the 61, 7511890; 29 outside the 3,502) with the salted 300 drawn from P minus the fixed ids (population 3,400, ids_sha256
  c563c3c9…) by `sha256('551-b6-stage1-'+id)`, drawn-list sha256 35e9bd5a… (first: 6260678, 6663541, 30546, 7109997, 889869);
  stage2 3,100 (all P, B_P 18); lc01–lc03 5,000 each, lc04 373 (none in the 3,502). Start rule at 10:20Z for 6 runs: GO, estimate
  3 h 05 m → 13:25Z (switch-off gate budgeted), latest switch-off start 2026-10-22T06:30Z.
- **Switch-off rev (§0.6):** the marker covers 84c5118 (the flip) only; `gate_deploy_step d617a78` = refuse-head. Plan: deploy the
  flip, then gate the W=false twin in this checkout — a revert commit of the flip, whose tree outside `.scratch` equals
  pre-flip main (fa96ee3 / 9d1b4ca: d617a78 + the user's `.claude/settings.local.json`) — so a switch-off is a push of that
  revert + a covered deploy (no gate inside the switch-off). Until that gate is green the start rule's 35-min gate budget stands.

## B6 U7 — flip deployed and staged drain (2026-10-21)

- **Flip deployed 10:24:45Z**: e718684 (84c5118 cherry-picked onto main d209375; `gate_deploy_step` = skip, tree equal to the
  gated 84c5118). The first deploy attempt refused fail-closed (the queue probe's ssh hop dropped, "could not measure the queue");
  the retry deployed. **C1 = 959004090 = C0**: no rows between U6 and the deploy (classify not needed).
- The ssh hop (wstunnel) drops intermittently today ("Connection closed by UNKNOWN port 65535", no request reaches the box). Job
  POSTs go through an idempotent helper (re-reads the job list and re-posts only when no newer job exists; an unreadable list
  stops it); kit reads that drop are flagged and re-run only after box_health, never retried blind.
- Switch-off twin: d569107 (revert of e718684, tree = d209375 outside `.scratch`) on local `hold/551-switchoff`, gating since
  10:25Z in this checkout.
- **stage1 — PASS** (10:25–10:32Z). Dry 2690: 426 of 431 admitted; not admitted 7511890 (expected) and four pins outside the
  3,502 that no lot sum can reach (4925871, 4930240, 5625591, 8822313). Wet 2691 (expect 426: stamped 426, re-queued 1,916
  notices) → project 2692: 59 verified, 367 corrected, 2,430 correction rows; cursor 959004090 → 959006520 (lot ~2,063, tender
  ~367). D1 367/367 fired as predicted (vouched_by = the port's first on 367/367), Rg 35 identical; D2 corrected 367 = expected;
  D3 pins PASS; D4 band (public-API route: the /v1/sql seek's ssh hop dropped) 11 = B_P ∩ stage1; D5 0 outside the named list;
  D7 GO.
- **stage2 — PASS** (10:34–10:39Z). Dry 2693: 3,100 of 3,100 admitted. Wet 2694 (expect 3,100, stamped 3,100) → project
  2695: 0 verified, 3,100 corrected (0 tables rewritten: the head value columns only), 20,081 correction rows; cursor
  959006520 → 959026601. D1 3,100/3,100 as predicted; D2 3,100 = expected; D3 PASS; **D4 band (sql) 29 = B_P**; D5 0 outside.
  The read's first attempts lost statements to the ssh drops; with ssh connection multiplexing (one persistent tunnel,
  ControlMaster) every later statement answered. **The 3,502 run is done: 3,467 = |P| fired exactly as predicted, Rg (35)
  identical, nothing else fired.**
- **L_changed runs — all PASS** (10:40–10:53Z; D6 cap max(2·proj, proj+10), proj = admitted × 3,502/251,526):
  lc01 dry 2696 admitted 489/5,000 → wet 2697 → project 2698: 481 verified, 8 corrected; new fires 5 (214797, 235514, 511223,
  598677, 792396; proj 6.81, cap 16.8). lc02 2699 → 2700/2701: 206 admitted, 206 verified, 0 fires. lc03 2702 → 2703/2704:
  33 admitted, 30 verified, 3 corrected = 3 new fires (8851505, 8854032, 8854105; proj 0.46, cap 10.5). lc04 2705 →
  2706/2707: 1 admitted, verified. D5 0 outside P ∪ L_changed on every run; D4 band 29 throughout.
- **Three non-W corrections in lc01 (530 residue, not path W):** 497154, 675459, 780251 were corrected with no head-value
  change (their snapshot columns identical; 8 tables / 376 rows rewritten at the head, so rule L announced every head lot:
  34 / 6 / 8 lot rows). Their only change rows since 10-18 are 530's wet drain (01:29–01:32Z `tender changed`), and 530 did
  not refold them (2673 stamped 0). So a refold of a 530-touched Tender rewrites head tables that 530's in-place rehome left
  differently from a fresh projection — a 530 rebuild-parity gap, filed as issue 579. No value moved, so no 551 rule reads it.
- **U7 done 10:53Z** (start rule GO throughout). **Switch-off twin gated green**: d569107, GATE-EXIT=0, marker
  `gate-v2 d569107…`; its tree equals pre-flip d209375 outside `.scratch`, so a switch-off is now a push of a revert with
  that tree + a covered deploy (no gate inside it).

## B6 U8 — verify and delta read (2026-10-21)

- **Verify (10:53–10:54Z): PASS.** `u8/verify.py read`: 18,904 ids by PK, expected-absent 0, band 29 (sql route). `check`:
  V1 3,467/3,467 fired as predicted, vouched_by = the port's first on 3,467/3,467, Rg 35 identical; V2 nothing outside
  P ∪ L_changed serves lot-sum-awards; V3 pins PASS; V4 band = B_P (29); V5 every run PASS. verify_result.json 79d9722c….
- **U9 count baseline (C8): 8 new fires / 729 dry-admitted over the lc runs = 1.0974 %** (corpus figure 1.3923 %). U9 counts
  each chain's new lot-sum-awards heads against this rate; > 3× means stop and read.
- **Delta read — predraw recorded before the draw** (delta_predraw.json 4433a6fc…): population **8** (sha256 ed24b402c1e72fc8…),
  band rows none; bar case **30 or fewer: every head read, 0 wrong → PASS, 1 or more wrong → FAIL → switch off** (no
  continuation exists); neutral allowed and counted; protocol: reader + adversarial skeptic per head, a third reader on
  disagreement, wf_dc85fa7e-c00's rubric (M2). Draw (mode all): 214797 (EUR 840,000.00, 2 lots), 235514 (EUR 460,000.00, 3),
  511223 (EUR 6,279,778.00, 2), 598677 (EUR 508,977.00, 3), 792396 (EUR 70,673.78, 10; vouched BT-1118 70,674.00), 8851505
  (EUR 4,901,198.00, 5), 8854032 (EUR 134,103.46, 2), 8854105 (EUR 8,000,000.00, 2); all vouched by a BT-118/BT-1118 notice
  total. Bodies fetched (fetch_b6, 8 tenders + 16 notices). delta_read_set.json 825e5f61….
- **Delta read (wf_60e69470-890, 10:58–11:17Z): 8 right / 0 neutral / 0 wrong → PASS** (bar: 30 or fewer, 0 wrong). Reader
  and adversarial skeptic agreed on all 8 at high confidence; no third reader needed; no band row. Every head had a null
  pre-flip value (the max election found nothing to elect), and every vouching total is the BT-118/BT-1118 of the same award
  notice that awards exactly the summed lots: 214797 (2 lots = EUR 840,000.00), 235514 (3 = 460,000.00), 511223 (2 single-
  operator framework lots = 6,279,778.00; the BT-709 maxima are per-lot frameworks, not one envelope), 598677 (3 accord-cadre
  lots = 508,977.00), 792396 (10 lots = 70,673.78 vs BT-1118 70,674.00, the buyer's rounding; the Tender holds two copies of
  one CAN and S sums one copy), 8851505 (5 = 4,901,198.00), 8854032 (2 = 134,103.46), 8854105 (2 = 8,000,000.00 summed from
  BT-720 — the per-lot BT-709 copies of the 8 m envelope are not summed). Record `b6/u8/delta_read_result.json`.
- **U8 PASS — path W v2+G stays ON.** Next: **U9, the 14-day watch** after each 07:35Z chain from 2026-10-22 through
  2026-11-04: band seek for new lot-sum-awards rows ≥ EUR 100 m (each read at 0 wrong of any class, else switch off); the
  chain's new lot-sum-awards heads against **1.0974 %** of its admitted Tenders (> 3× = stop and read); day 7 (2026-10-28) a
  salted 30 (`551-b6-daily-`) of daily-derived heads, U5's sequential bar; standing rule — no lot-key or W-input drain
  (org merges included: 579's refold, 578, 519 cohorts) until the port diff is re-run on the served lot-sum-awards set; the
  U9 pins (6908248, and the partial sums 6679375, 7153588, 7398935, 7722197, 6779279, 6850401, 7698260). Switch-off = push a
  revert whose tree equals d209375 outside `.scratch` + a covered deploy (marker d569107), then `u7/switchoff.py`.

## B6 U9 — 14-day watch

- **Days 1–2 (chains 2026-10-22 2708–2717 and 2026-10-23 2718–2727, both ok), read 2026-10-23 18:48–18:53Z: PASS.** Every
  live Tender with a change row since the flip cursor (959004090 → 959180852: 13,453 Tenders) plus the 3,502 ∪ L_changed ∪
  every fire ∪ the pins, by PK (`u7/switchoff.py build`, read-only; outputs moved to `b6/u9/20261023/`): **3,478 serve
  lot-sum-awards = P's 3,467 (all at their predicted S and currency, 0 off) + the 8 delta heads + 3 new daily fires**:
  97901 (HUF 23,916,180.00 ≈ EUR 65,268, 4 lots, vouched BT-161 HUF 23,868,000.00 on notice 47415952; an L_changed Tender
  whose new notice arrived on 10-22), 8859070 (EUR 640,000.00, 2 lots, BT-118 on 47430483, new Tender 10-23) and 8859315
  (EUR 400,000.00, 2 lots, BT-118 on 47430942, new 10-23). **Band (sql seek): 29 rows = B_P exactly**, no new band row.
  Count: 3 new fires over two chains (~11,500 Tenders written) — far under 3× the 1.0974 % baseline on any cohort share.
  The three join the day-7 read population (10-28: fewer than 30 so far → read all).
- The 546 gate (1730d45, green) overwrote the gate marker, so the W=false twin d569107 is NOT covered now: a switch-off
  today re-gates (start-rule budget). The twin is re-gated on top of 546 right after 546 deploys.
- **2026-10-23 ~19:00Z: 546 unit 3 deployed (512b297 = 1730d45's tree, gate green) on top of the flip; the W=false twin
  re-built on it and gated green: adcb7b0** (local `hold/551-switchoff-2`; revert of e718684 on 512b297, CHANGELOG kept as on
  main; `LOT_SUM_AWARDS_PATH = false`, 546 kept). Marker `gate-v2 adcb7b0…` — a switch-off is again a covered deploy: commit a
  revert whose tree equals adcb7b0 outside `.scratch` on top of main, push, `./deploy.sh` (`gate_deploy_step` = skip), then
  `u7/switchoff.py build/verify` with C1 = 959004090. Every later code deploy must re-gate the twin the same way (C7).
- **2026-10-23 ~21Z: W-on drain protocol DECIDED** (`b6/decision-w-on-drains.md`; three judges, a synthesis, critic W1–W15
  adopted, O1–O3 optional, recorder's check R1). Each writer is a single-writer segment, closed by gd5 against registry R
  before the next writer starts. R0 = `u9/20261023/switchoff.json` (3,478 ids, 8845f0d0…), K0 = 959180852. From 10-24 the
  U9 check is gd5 class chain over (K, chain end], with A_d (the 3× rule on dry-admitted). It starts when /admin/jobs shows
  the chain done, never by the clock. The tick is 09:35 Berlin, which is 08:35Z from 10-25. The weekly tick (Sundays 02:10Z,
  10-25 and 11-01) is a writer window. The day-7 population stays as C8 defined it; chain-MOVED R heads are read separately.
  Order: kit E1–E6 and the start_rule fix → P0 (512b297) → 539 step 1 → 539 step 2 (3 runs) → day 7 → batch (576/578 U2 +
  579 U4; 577 U2 only if W7 is empty) + P1 → 576 U3 → 546 U4 (if E1 is proven) → 577 U3. After 11-04, under C9': the
  class-I writers (358, 440/473, 519 M6), 575 U3/U4, 561 U4 and any lot-key drain. 579 is not a drain. Amendment: for
  post-U8 drains, a fire on a 541/571 residual (U6 deviation (a)) is read as an enumeration miss, not a switch-off.
- **579's 6 order-only Tenders (pre-registered, 579 U2/U3):** 408007, 413501, 454861, 179248, 340956, 823340 will read as
  corrected with no value move whenever a refold reaches them (539's step 2 does: all hold eforms-de versions). Not W drift.
  179248's v_awards representative buyer flips 23052423 → 13025902 at that refold (expected).
- **2026-10-23 ~23:40Z: 578 U2 + 579 U4 + 577 unit 2 gated green at 406529a (on main, not deployed).** That gate moved the
  marker to 406529a, so the deployed twin adcb7b0 is uncovered until it is re-gated (tonight, after the W-on kit commit;
  before the 10-24 07:35Z chain). No writer runs meanwhile (no segment open). Main now holds undeployed gated code: a
  switch-off commit must carry adcb7b0's tree outside `.scratch` on top of main (commit-tree with that tree + main's
  `.scratch`), not a plain revert of e718684 on main.
- **2026-10-23 ~23:35Z: the twin adcb7b0 re-gated green (GATE-EXIT=0); marker `gate-v2 adcb7b0…`, `gate_deploy_step
  adcb7b0` = skip — a switch-off is covered again.** The W-on drain kit is committed (2c757e2; `u9/RUNBOOK-W.md`). Next,
  10-24 after the 07:35Z chain: U9 chain segment (gd5 class chain), `registry.py init` + W12(b) replay, probe P0 on
  512b297 (class D), then 539 step 1 (class Z) and step 2 (class W, runs a/b/c) — decision-w-on-drains order.

## W-on drains — records before first use (decision-w-on-drains; 2026-10-23 23:4xZ)

- **Kit:** `b6/u9/KIT_W_SHA256SUMS` (sha256 c90615bfdac90923…; 20 files incl. the gitignored `u9/plan/plan.db`) checked
  live: `sha256sum -c` exit 0, 20 OK.
- **R0 / K0:** `u9/registry.py init` — 3,478 entries {P 3,467, delta-U8 8, chain-2026-10-22 1, chain-2026-10-23 2};
  ids_sha256 8845f0d03f91defccf46a7275a5ead4d13dc84a44a2501155202562365f3ae20; entries_sha256
  269abd4ee4c682b89012511264acbe06f1ec87390b8a22d5a7795f5187973ddb; K0 959180852; B_R 29; watch 63.
- **W12 (a)** (kit build, 20:30Z): E1 with `INDEXED BY changes_entity_cursor` accepted on prod (sqlite_master is not on
  /v1/sql's allow-list) — CONFIRMED. **W12 (b) replay (23:44Z, 4 statements): PASS** — E1's tender rows over
  (959004090, 959180852] = the archive's 18,216 exactly; E1's lot-sum-awards set = R0's 3,478 (lost 0). E1 is on. Calendar
  note (runbook §K.5): W12 ran before the first chain read, overriding the decision's 10-24 order (W12: "before E1 is
  used anywhere").
- **Code-drift ledger (C1) — 512b297** (deployed 10-23 ~19:00Z): 546 unit 3 (legacy TED `country` = place of performance
  only; no path-W input). Its W-on probe is P0.
- **P0 pre-registration (class D, B4):** `refold-lot-sum` tender_ids = R ∪ Rg ∪ pins (3,541 at R0), one run; bar: 0 moves in
  R's W columns (derivation, cents, currency, eur, detail.lots, vouched_by membership); NOTEs for seq/published_at moves and
  for place/country on R's legacy r209 heads; NEW expected 0; an eur-only move on a re-dated head with its tender row is a
  NOTE; path-W pin / partial sum off, or 6465510 serving lot-sum-awards = SWITCH OFF; 6908248 moving = stop and read. Twin
  adcb7b0 covered (switch-off gate 0 min). Writer lock: "Writer: 551 p0" from its open until its advance.
- **Writer: 551 p0 until 2026-10-24T01:01Z** (segment p0 opened 2026-10-23 23:46Z at CB = K = 959180852; class D; dry 2728 admitted 3,537 of 3,541).
- **Writer: 551 z539-1 until 2026-10-24T01:30Z** (539 step 1, repair-notice-instants, class Z; opened 2026-10-23 23:50Z at CB = K = 959192253).
- **P0 PASS (2026-10-23 23:46–23:49Z):** dry 2728 (3,537 of 3,541 admitted; 4 unreachable pins) → wet 2729 (stamped 3,537)
  → project 2730 (1,658 verified, 1,879 corrected, 8,039 tables rewritten — 546's place/country) over (959180852,
  959192253]. gd5 class D, route both: E1 = all-kind on the 1,879 Tenders; NEW 0; R MOVED/LOST/SILENT 0; pins PASS; band
  29 = B_R. Advanced: K → 959192253.
- **539 step 1 PASS (z539-1, 23:50–00:04Z):** dry 2731 planned 16,343 (sdk-0.1 10,890, de-2.1 2,487, de-1.1 1,359, de-1.2
  1,138, de-2.0 469; 0 epoch/NULL/removed; vs 16,344 measured in 539 unit 1) → wet 2732 APPLIED 16,343, skipped 0, no
  project. Class Z shortcut: K = CA = 959192253, own jobs only, corrected 0 → PASS, advanced.
- **539 step 2 pre-run record (C9' discharge):** A6 + P0 show R at 539's fixed point (539 deployed 10-19; zero-drift
  2656-2658/2677-2679 and P0 0 R moves), so the prediction is **0 R moves**; seq/published_at moves are NOTEs; an eur-only
  move on a re-dated head WITH its tender row is explained (899272 and 5842187 under the 2 % rule). Cap: max(2·A·1.0974 %,
  A·1.0974 % + 10) with A the run's dry-admitted count. Salt `551-b6-drain-539-`. Bar: U5's sequential bar on the NEW
  heads (all if ≤ 30); band rows 0 wrong of any class. 579's 6 order-only Tenders expected as non-W corrections. Runs:
  a = sdk-0.1 (expect ~565k), b = de-1.0/1.1/1.2 (~219k), c = de-2.0/2.1 (~253k); each its own segment, STOP point
  between runs; all close before the 10-25 02:10Z weekly tick, else c moves to 10-25. Started in the 10-23/24 night gap
  (after the 10-23 chain's check in R0, P0 and step 1; no tick before 07:35Z).
- **Writer: 551 w539-a until 2026-10-24T02:30Z** (539 step 2 run a, refold eforms-sdk-0.1, class W; opened 00:05Z at CB = K = 959192253).
- **Writer: 551 w539-b until 2026-10-24T03:00Z** (539 step 2 run b, refold eforms-de-1.0/1.1/1.2, class W; run a PASS: NEW 0, 0 R moves, A 6).
- **Writer: 551 w539-c until 2026-10-24T03:45Z** (539 step 2 run c, refold eforms-de-2.0/2.1, class W; run b PASS: NEW 0, 0 R moves, A 5, 579's 6 refolded).
- **539 step 2 run a PASS (w539-a, 00:05–00:41Z):** refold 2733 (687,796 notices) → project 2734 (9,117 corrected) over
  (959192253, 959211183]. gd5 class W, E1 all-kind. A 6 (dry refold-lot-sum over the 9,117), cap 10.07, NEW 0, R
  MOVED/LOST/SILENT 0, pins PASS, band 29. K → 959211183.
- **539 step 2 run b PASS (w539-b, 00:43–01:37Z):** refold 2737 (218,876) → project 2738 (2,211 corrected) over
  (959211183, 959215522]. A 5 (of 2,212), cap 10.05, NEW 0, R moves 0, band 29. 579's six were refolded, order only.
  K → 959215522.
- **539 step 2 run c PASS (w539-c, 01:38–02:35Z):** refold 2740 (255,165) → project 2741 (2,196 corrected) over
  (959215522, 959223268]. A 12 (of 2,289), cap 10.13, NEW 0, R moves 0, band 29. 93 non-R Tenders were retired
  (`tender/removed`). K → **959223268**, open_segment none. D7 advisory: GO, all runs closed before the 07:35Z chain
  and the 10-25 02:10Z tick. Writer released.
- **539 drain complete** (the 539 issue has the table). R stayed at 539's fixed point, as predicted by C9': 0 R moves
  across 4 class-W/D segments. Next W-on users: the U9 class-chain check after the 10-24 07:35Z chain (RUNBOOK-W §2); the
  10-25 02:10Z weekly tick (§6); 546 U4.
- **A for days 1–2 (H), measured 2026-10-24 02:51–03:0xZ: A_1-2 = 4,105** admitted of the decision's literal 13,453
  (`u9/ajobs/days12/`, `--after-cursor 959004090`, archived all-kind enumeration; dry refold-lot-sum 2743 / 2744 / 2745
  = 2,704 + 1,378 + 23; A.json ids_sha256 2bb77578…). **3 fires vs 3 × A_1-2 × 1.0974 % = 135.1: far under.** Order
  deviation, recorded: the decision asked for this before the first drain wrote, and it ran after P0 and 539's four
  segments. A dry admission reads today's heads. Across those segments, gd5 found NEW 0 and 0 R moves, and the retired
  Tenders are reported as not admitted. So the drift is at most a few re-headed non-R Tenders against 4,105, and the
  verdict cannot flip.
- **W7 for 577 U2: NOT EMPTY (2026-10-24, wf_52627394-803, analyst + adversarial skeptic CONFIRMED; `b6/w7/W7.md`).**
  Three code moves on eforms-de-1.x land on W-read ids: the lot estimate (its old id normalises to BT-27-Procedure),
  the framework maximum and the lot name. 14 more W hits come from the overlap list only. By the rule, 577 U2's
  deploy and 577 U3 wait until after 11-04. 406529a was on main undeployed, so it is reverted there (b3effc2) and the
  batch no longer ships it.
  - **The batch is now:** 576 U2 (cb19854, the one fold-changing commit), 578 U2, 579 U4, and 539's caveat retirement
    (ce20b28).
  - The 576 U2 + 539 + revert tree is gating at b3effc2. A green gate moves the marker off the twin adcb7b0, which is
    re-gated right after. The deploy itself still waits for the 10-28 day-7 fetch (decision calendar).
  - 576 U2 touches supervisor.rs, a flip file, so the twin rebuilt on the deployed base carries its hunk.
- **2026-10-24 04:06–04:29Z: gate green at b3effc2** (539 caveat retirement ce20b28 + 576 U2 cb19854 + the 577 U2 revert;
  GATE-EXIT=0, 1,384 s); main = handover = 44e0f00, which holds undeployed gated code.
  **04:30–04:54Z: twin adcb7b0 re-gated green** (GATE-EXIT=0, 1,441 s). The marker reads `gate-v2 adcb7b0…`;
  `marker_covers adcb7b0` = COVERS and `gate_deploy_step adcb7b0` = skip, so a switch-off is again a covered deploy
  (0 min). A switch-off commit carries adcb7b0's tree outside `.scratch` on top of main, as before. The twin is
  unchanged: the deployed code is still 512b297.
- **2026-10-24 06:00–06:23Z: 580 U2 gated green at fe3eca5** (served docs: a Tender id can be retired within a
  generation; fold summary prints `Report.absorbed`; 1,457 s). main = handover = fe3eca5. **06:24–06:50Z: twin
  adcb7b0 re-gated green** (1,562 s): COVERS, `gate_deploy_step` = skip, so a switch-off costs 0 min.
  - The 10-28 batch is now 576 U2 (cb19854, the only fold-changing commit), 578 U2, 579 U4, 539's caveat retirement
    (ce20b28) and 580 U2 (fe3eca5: docs and a summary line; no fold change).
  - The twin rebuilt on the deployed base must carry the supervisor.rs hunks of 576 and 580 and the docs/openapi
    hunks of 539, 576 and 580 (C7).

