# 537 — a head figure 1,000× the sum of its own lots (to the cent) is served: 369057 at €62.4 bn, 165437 at €20.0 bn

Status: ready-for-agent — unit 1 MEASURED and unit 2 DECIDED 2026-10-16 (below): of 3,750 heads ≥ €1 bn, 48 match 10ᵏ × a lot sum; adjudicated 43 head_wrong, 4 lots_wrong, 1 coincidence. Rule: k 2..6, n ≥ 2, award-side S unconditional, estimate-side S only under a third-figure guard. It refuses 22 heads, all wrong, 0 wrong refusals. Next: unit 3 (build) after 533 and 524 v10 leave the tree; the lot-sum fallback is filed as 551.
Kind: data quality (amount plausibility: the head election)
Relates to:
- 471 (done 2026-10-08). Two of its decisions border this one, and neither covers it:
  - **4(c)** closed dropped decimals "caveat-only: there is no signal to build" (471:948–951). 369057 and 165437
    ARE dropped decimals, and they DO carry an in-notice signal: the sum of their own lots. They contradict the
    premise 4(c) was closed on, which is why this is filed despite a decision.
  - **Unit 5** serves "a single figure with no in-Tender partner" as published under a /docs caveat (471:880–927).
    These two have a partner (the lot sum), so they are outside that class.
  - 20905 (Unit 5's exhibit) and 355006 (4(c)'s exhibit) are decided there. Not re-filed here.
- 366 (done 2026-09-12): named "the tender-amount-versus-lot-sum ratio" as the signal for this band twice (366:627,
  366:828) and left it unbuilt — not rejected. The same note stands in the `IMPLAUSIBLE_EUR_CENTS` doc
  (crates/store/src/canonical.rs:3366–3374; 471:23 cites it at its older place, 1501–1506). 471:97 adds that "the
  lot-sum ratio needs a lot figure", which both tenders have.
- 492 (done 2026-10-09): the ×100 framework exemption reads the lot sum, but only to KEEP a figure.
- 505 (done 2026-10-09): a lot figure 10ᵏ above the procedure remainder — the converse direction. Its heads census
  (3,736 heads ≥ €1 bn on 2026-10-09) is the shape unit 1 reuses.
- 506 (done 2026-10-10): BT-709/BT-660 are partner-only rows, kept out of lot sums. 369057's BT-709 rows exist
  because of it. BT-118-NoticeResult stays unmapped (506 unit 2).
- 267 / section 16: the weekly band listing. Both rows sit there with no partner.
- 369 (done 2026-09-12): decided tender 2's placeholder-shaped key. Named here only because the same review raised
  it; see "What the external review got wrong".

## Verify

    for t in 369057 165437; do curl -s --max-time 20 https://tenders.zebreus.click/v1/tenders/$t | python3 -c "import json,sys; d=json.load(sys.stdin); s=sum(r['awarded']['cents'] for r in d['lot_results'] if r.get('awarded')); print(d['id'], d['value'], 'lot_awards', s)"; done

- **done**: neither head is at or above €1 bn (`cents` < 100000000000; what it falls to is the election's next
  figure — for 165437 its €20,202,891.70 procedure estimate). The `lot_awards` sums are unchanged. OR a
  `## Decision` section here keeps them, with the reason.
- **open (2026-10-14)**:
  `369057 {'cents': 6237873628000, 'currency': 'EUR'} lot_awards 6237873629` and
  `165437 {'cents': 1997527462600, 'currency': 'EUR'} lot_awards 1997527462`

## Repro

Public GETs, 2026-10-14 (`B=https://tenders.zebreus.click/v1`):
- `curl -s $B/tenders/369057 | jq -c .amounts`: `result_value` 6237873628000 (procedure) plus three
  `result_framework_maximum` (BT-709) lot rows, 158878238 + 3367786340 + 2711209050 = 6,237,873,628.
- `curl -s $B/tenders/165437 | jq -c .amounts`: procedure `estimated_value` 2020289170, `result_value`
  1997527462600, three lot estimates summing to 2,020,289,170 (the procedure estimate, exactly).
- The decided exhibits, unchanged: 20905 → `value.cents` 3326050000000 (one figure); 355006 → 3500009416600 (one
  `estimated_value`); 2 → `procedure_key` `00000001-2023-4000-a000-000000000001`, 2 versions.

## What was found

| tender | head (BT-161 `result_value`) | lot awards, summed | other figures | head ÷ 1000 against the lot sum |
|---|---|---|---|---|
| 369057 "servizi accoglienza migranti" (TED eForms sdk-1.12, 00522142-2025, 1 version) | €62,378,736,280.00 | 1,588,782.38 + 33,677,863.41 + 27,112,090.50 = €62,378,736.29 | BT-709 per lot sums to €62,378,736.28 | 1 cent off the awards; exact on BT-709 |
| 165437 noise barriers, restricted procedure (TED eForms sdk-1.12, 00667488-2025, 2 versions) | €19,975,274,626.00 | 6,352,317.58 + 6,765,347.10 + 6,857,609.94 = €19,975,274.62 | procedure estimate €20,202,891.70 (×988.7, not a partner) | head ÷ 1000 = €19,975,274.626, 0.6 cent off |

Both are dropped decimals: the publisher printed the lot total with its decimal separator gone. 165437 shows it
plainly: its head is the lot sum written with three decimals and no separator.

Why nothing refuses them:
- The scale rules test ONE partner figure: 471's k ≥ 3, 492's ×100 and 505's residual (`refuses` /
  `refuses_by_partner`, canonical.rs:3986–4020). A total that is 10ᵏ × the SUM of the lots has no single partner.
- `framework_total` (canonical.rs:3856) reads the lot sum only to keep a ×100 figure whose lots add up to F/10..F,
  never to refuse one.
- Partner-only rows (BT-709/BT-660) never count in lot sums (canonical.rs:3517–3522).
- 369057's BT-118-NoticeResult carries the same figure as its BT-161, but BT-118 is unmapped (crates/ingest/src/
  project.rs:1278, 506 unit 2). It is the same slip in a second slot, not corroboration.
- Section 16 (dq2263, 2026-10-11, `.scratch/tender-db/471-values/section16-dq2263-2026-10-11.txt` lines 178 and
  206) lists both with "exact 10^k partner —" and an "x smallest" of x39,261 and x3,144. Neither appears in 471's
  unit-5 sample or in the 492/505 adjudication files.

Hand sample (verifier): 7 of the 41 eForms rows of section 16 (369057, 1080295, 281452, 761379, 165437, 594971,
655469). 2 match (369057, 165437). The other 5 have head-to-lot-award ratios of 118, 1.0, 2.13, 0.50 and 0.55.

Not measured: the FTS, r208/r209 and text rows of the band; the €1–10 bn band; the corpus.

### What the external review got wrong

- **It grouped three ids as one class.** 20905 (€33.26 bn, one figure in every slot, no sibling) is 471 Unit 5's
  exhibit. 355006 (€35,000,094,166.00, about €350,000,941.66 with the decimals dropped) is 471 4(c)'s exhibit. Both
  are served as published under the /docs caveat the review quotes (docs.rs:799–805, "roughly a third"; 471's
  sample gave about 111 of 305, n = 48). That is a recorded decision, not an oversight: buyer history caught 6 of
  20 sampled errors at zero false flags and fails for structural reasons. **Do not re-file 20905 or 355006.** Only
  369057, plus 165437 found while checking it, carries an in-notice signal.
- **Tender 2's key is not a defect.** `00000001-2023-4000-a000-000000000001` is placeholder-shaped, but it is the
  publisher's own BT-04. It groups one buyer (Landkreis Göttingen, org 22183653) across the DÖE notice and its TED
  twin `00710890-2023`. Issue 369 found it and admitted it on purpose (369:121–130); `is_placeholder_key` pins it
  (project.rs:13510). **Do not re-file it.**
- The first verification draft's errors, not to be repeated: "exactly 1,000×" (165437 is 0.6 cent off; say "to the
  cent"), and arguing only against Unit 5 (the decision these two contradict is 4(c)).

## Unit 1 — measured (2026-10-16)

Evidence pack: `.scratch/tender-db/537-lot-sum/`.
- `unit1-synthesis-2026-10-16.md`: the full record, every exhibit, and the precision matrix.
- `unit1-verdicts-2026-10-16.json`: first reader and skeptic for each Tender.
- `sweep_537.py` / `analyse_537.py`.

**Population.**
- 3,750 elected heads at ≥ €1 bn and under the €100 bn cap. These are one bounded seek on
  `tenders_current_value_eur`, plan-checked, and the public detail of each.
- 2,082 of them carry a lot figure in the head's currency.
- **48 match** `|F − 10ᵏ·S| ≤ 10ᵏ·n` cents for some lot sum S:
  - k=1: 17; k=2: 10; k=3: 20; k=4: 1;
  - 43 TED and 5 FTS.

  S is one of: each lot's latest award; lot BT-27; lot BT-271; or BT-709/BT-660.
- Sanity population: the head equals its lot sum for 776 estimate sums and 550 award sums.

**Adjudicated** (workflow wf_da0f812d-8bd: a reader per Tender, then a skeptic on every decided verdict):
**43 head_wrong, 4 lots_wrong, 1 coincidence**.
- lots_wrong:
  - 8400892 (k=3, n=1, the kept housing framework);
  - 8312215 (k=4, lots in thousands);
  - 8720567 (k=2, FTS);
  - 8596393 (k=1, FTS, moderate confidence).
- coincidence: 1039053 (k=1, a deliberate 10× framework ceiling).
- Every wrong row has an `estimated_value` or `framework_maximum` head, or n = 1.
- All 28 `result_value` heads are head_wrong.

**Precision on the 48** (right / wrong refusals):
- Award-side S (awards ∪ BT-709 ∪ BT-660) over n ≥ 2 lots at k ≥ 2: **20 / 0**.
- The same at k ≥ 1: 24 / 1.
- Estimate-side S without a guard at k ≥ 2: 7 / 2.
- Estimate-side S with guard T: 2 / 0. T holds when a third procedure-scope figure X ≠ F in the chain sits within
  [S/10, 10·S].
- k = 1 sums are all round numbers (no digit information) and include the coincidence.

## Decision (2026-10-16, owner, from unit 1)

Adopt the synthesis's recommended rule.
- **Where:** in `ScalePartners` at election time (492's placement).
- **What it refuses:** every procedure-scope fact of the head version whose cents equal F, worth at least
  `SCALE_ERROR_MIN_EUR_CENTS` (€1 bn) and at most `IMPLAUSIBLE_EUR_CENTS`. Lot-award copies are not refused.
- **When:** for some k in 2..=6 and some S over **n ≥ 2** lots, `|F − 10ᵏ·S| ≤ 10ᵏ·n` cents, where S is:
  - **(A) award side, unconditional:** lot awards, each Lot once from its latest award; BT-709 per lot; BT-660 per
    lot. Partner-only rows serve as S only, never elected and never in `head_lot_sums` (506).
  - **(B) estimate side, only under guard T:** lot BT-27 (each Lot's largest figure); lot BT-271 (each Lot's largest
    figure).
- Lots of kind Lot only; Parts count only in a head with no Lot. Withheld figures and sentinels are skipped.
- 492's `framework_total` keep takes precedence. No corroboration exemption (492's reason).
- **k = 1 is not adopted.** It is below every scale rule's floor, its sums are round, and it holds the coincidence.
  Its 15 head_wrong rows stay as published, and the caveat stands.
- **Expected effect:** 22 refusals, all head_wrong, 0 wrong. The rule-of-three 95 % bound on the error rate is
  about 13.6 %.
  - All 22 drop below €1 bn: the band goes 3,750 → 3,728.
  - They fall to null 6×, to a plausible procedure figure 5×, to one lot's figure at the right scale but understated
    10×, and to a wrong figure 1× (867491's own ×1000 lot slip, under the gate).
  - Serving the lot sum instead of one lot's figure is a separate decision: filed as 551.
- **Recorded cost:** arm (A) cannot keep a correct total over award lots published in thousands. That shape occurs
  0 times in the 20 award-side refusals, and a guard against it would also lose 369057.
- **Interactions:**
  - 471: n ≥ 2 leaves single-partner cases to 471. Its corroboration exemption keeps 317736 and 622872, two slips;
    recorded, unchanged.
  - 492: disjoint windows, and 492's exemption wins.
  - 505: the converse direction.
  - The `IMPLAUSIBLE_EUR_CENTS` doc's "tender-versus-lot-sum ratio" sentence names this rule.

**Unit 3 is the build, as below, with these additions:**
- the S definitions above;
- the section-16 flag column;
- the tests and controls the synthesis names:
  - refuse: 369057 → null, 165437 → 2020289170, 6921980 / 801998 (tolerance), 392980 (latest award per lot), 1024214
    (BT-709 as S), 508666 / 114991 (arm B through T);
  - keep: 8400892, 8312215, 8720567, 8596393, 1039053, 7955574, and the 492/505 control sets.

## Proposed units

1. **Measure (S).** An in-app read (a section-16 column or a census job), not a corpus scan from the agent.
   - Range-read the heads ≥ €1 bn on the `tenders_current_value_eur` index (505's heads shape).
   - For each head figure F, in its currency, build candidate lot sums S from the head version: (i) lot awards,
     each lot once; (ii) the BT-709 and the BT-660 partner-only rows per lot. Leave out sentinels and withheld
     figures.
   - Flag |F − 10ᵏ·S| ≤ 10ᵏ·n cents for k = 3…6, where n is the number of lots summed (each lot is rounded to the
     cent).
   - Report every flagged row with k, which S matched, n, and whether it already has a single 10ᵏ partner (overlap
     with 471/492/505). Add the flag as a section-16 column so the weekly report keeps it.
2. **Decide (S).** Adjudicate every flagged row from the notice.
   - The false positive to rule on: a correct total over lots published in thousands (k€ lots). There the lots are
     the slip, and the head must stay.
   - Candidate corroboration that the lots are right: the lot sum agrees with an independent figure — a procedure
     estimate within 10×, contract values, BT-720 tender values.
   - A BT-161/BT-118 echo of the head is not corroboration. One notice copying a slip into two slots is 492's
     reason for having no corroboration exemption.
3. **Build (M), if (2) adopts the rule.**
   - Refuse a procedure-scope figure ≥ €1 bn that matches the rule, at election time in `ScalePartners` (492's
     placement), with S computed as in unit 1. Partner-only rows may serve as S, but stay never-elected and out of
     lot sums (506).
   - Controls that must not regress: 8400892 (the kept £10.8 bn housing framework); 492's nine genuine frameworks
     among its 56 adjudicated €1–10 bn heads; 505's adjudicated set; 7955574 (PSSV, nine lots summing exactly,
     ratio 1).
   - Pin it with `a_total_ten_to_the_k_times_its_lot_award_sum_is_not_elected` (369057's figures and 165437's
     three-decimal shape), plus a k€-lots case that keeps its head.
   - Drain with `refold-value-band` → `project` (505's route). Re-read section 16 and the Verify.
