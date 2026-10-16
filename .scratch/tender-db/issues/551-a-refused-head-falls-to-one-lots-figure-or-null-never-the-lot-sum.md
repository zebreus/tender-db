# 551 — when the election refuses a head, it falls to one lot's figure (or to null), never to the sum of the lots

Status: ready-for-agent — unit 1 MEASURED 2026-10-16 (wf_8519d344-f56; reading corrected by its skeptic, below). Next: unit 2, decide. A guarded lot-sum fallback is under a design panel against the 88 hand-classified members.
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
