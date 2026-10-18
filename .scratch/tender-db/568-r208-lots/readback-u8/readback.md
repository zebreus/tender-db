# Combined read-back: 568 unit 8 + 570 (refold-fields 2616 + project 2617). Judge, 2026-10-18

The drain finished at 15:19:48Z. 1,250,447 Tenders were written: 1,203,568 verified and 46,879 corrected. The changed
set is the 46,879 version-null tender events with 957854566 < cursor <= 958764334. Three stages paged it separately
and got the same sha256 (8062ccd1…).

There were four stages: sets, fresh568, fresh570 and band. Each had an adversarial verifier. Per-item results, evidence
and files are in `readback_result.json`. I made no GETs: every judge check below is offline, over the stage and
verifier caches and the decision texts.

**Update, 19:35Z.** The O1 extension ran and was adversarially verified. 568's verdict is now final, **PASS**: see
"O1 extension" below. The judge again made no GETs.

## Verdicts

- **568: PASS (final, after O1).** Every item passes over the conformant 30:
  - (a) D7;
  - (b) 8327340;
  - (c) the s150's 57 refused blocks;
  - (d) the 77, 692/692;
  - OOS 50;
  - (f) the band;
  - the draw: 30 held, 21 outside 2012–2015 (at least 10 required), and **8 G2** (at least 8 required);
  - the amended bar over the 30:
    - (i) parity: 417/417 blocks, 0 breaks;
    - (ii) 0 of 30 Tenders keep a judged-wrong block, against a limit of 2;
    - (iii) no kept wrong block and no reopen trigger.

  At 16:58Z this verdict was OPEN, because the draw held only 5 G2 Tenders. The cause was a defect in
  `draw_fresh30.py`, not in the build. The pre-registered 3-Tender extension (O1) closed the gap: 5048894, 4836560
  and 5226033. They add 66 568-scope blocks, all labelled single_right; 65 are served on their lot and 1 is falsely
  refused by G5. One label could be contested (4836560 17518149/RES-1). Judged wrong, it would make (ii) 1/30, still a
  pass.
- **570: PASS.** Items (a) to (g) all pass, and (e) is 30/30. The 6876932 question is settled by rule step 3's
  definition of a target, as set out under (e) below. Nothing reopens.

## Judge's own checks

- **568 (d), the 6 extra-strata Tenders missing from the baseline.** Live equals `readback/measured77.json` on 692 of
  692 blocks. That file is 568's first read-back: the lots served after drain 2593/2594 and before this drain. So
  "unchanged" is now observed on both sides for all 77. `u7-cache.jsonl.gz` cannot serve as the before-read, because
  its m77 reads predate 568's first drain.
- **The G2 quota.** 6324865, 6788159 and 6869520 have only `ted-export-r209` notices: 35, 174 and 11 of them. They have
  0 blocks in 568 scope.
  - The 22 non-G2 Tenders were held at walk indices 0–21. After that only a G2 Tender can be held. Dropping the three
    false G2 Tenders therefore leaves the other 27 unchanged.
  - Recall. The verifier listed 290 candidates. Their 15,707 lot-placed results are all r209. The other 322 have lot
    results only on notices ≥ 21,501,487, which is above the highest R2.0.8 id in any cache (21,409,453).
- **570 (e), the meaning of "target".**
  - decision.md rule step 3 defines Targets as "the carried lots of kind Lot with a non-blank carried number".
  - Error risk defines a wrong remap as "one printed number naming different lots in two notices of one procedure".
  - I adopt the verifier's reading on both counts, as explained under (e) below.
- **570 (e), the hold rule.** "Remap-touched" was fixed in the stage spec before any read and is computed from the port,
  not from what is served. It cannot favour a pass. It restricts the combined changed set to the Tenders that 570
  itself can correct.
- **The D3 wording.** ADR-0017 D3 rule L announces every head lot only when the head differs, plus every lot whose own
  rows differ. The build matches the ADR, and the 570 decision's "every lot of the head version" is looser wording.

## 568: items

| item | result | evidence (short) |
|---|---|---|
| (a) D7 | pass | 1,203,568 + 46,879 = 1,250,447; 0 epoch-0 rows in 7 windows |
| (b) 8327340 | pass | 7 multi-lot blocks serve null; RES-3 LOT-24, RES-4 LOT-32, 17840150 RES-1 LOT-2; read twice, identical |
| (c) s150 | pass | 57 refused serve null; 5051007 RES-1..3 and 5267810 RES-1 kept; 1190/1190 = expected; 59 moves = 57 + two 570 movers on 8313642 |
| (d) m77 | pass | 692/692 = expected and = the first read-back (judge check); 0 refusals |
| OOS 50 | pass | 100 refused serve null; 432/432 = expected |
| fresh 30: draw | pass | written 15:42:26Z, before the bar read; reproduced offline. The conformant 30 (27 + O1's 3; draw_ext.json written 18:07:28Z, before its bar read) has 21 outside 2012–2015 |
| fresh 30: ≥ 8 G2 | pass (after O1) | 8 of 8: the 5 true-G2 Tenders plus O1's 5048894, 4836560 and 5226033. It was 5 of 8 at 16:58Z |
| bar (i) parity | pass | over the 30: 417/417 blocks (351 + 66), 0 breaks. The 27 were read at 15:43Z and re-read at 18:07Z, identical |
| bar (ii) ≤ 2/30 kept-wrong | pass | over the 30: 0/30. 178 blocks labelled. 6 wrong blocks, all refused (G3 ×4 on 8291946; G5 on 8365595 and 5864590). Tenders with a wrong block: 3/30 before the guards, 0/30 after |
| bar (iii) | pass | over the 30: no kept wrong block; no trigger |
| (f) band / path-V | pass | 3,728 heads → 3,728; 0 movers; 17 path-V heads unchanged |

## O1 extension (2026-10-18, judged 19:35Z)

**Verdict: pass. 568 closes.** An extender ran the extension and an adversarial verifier tried to refute it. The
verifier could not. I checked both offline, over their caches.

### The draw

- **Kept.** The 27 Tenders that stand: walk indices 0–21, plus 61, 282, 366, 449 and 770. The three false-G2 Tenders
  (6324865, 6788159, 6869520) are dropped. Each kept id sits at the same index as in fresh568.
- **The walk.** It used the same `568u8-fresh30-` salted order over the same changed set (957854566 < c ≤ 958764334).
  - 46,879 events, minus the exclusions (s150 8, f30 1, oos50 2), leaves 46,868 candidates.
  - It resumed at index 771 and stopped at index 2134, on the third hold. That is 1,364 rows.
  - 220 population members were skipped unread. 1,144 Tenders were read, with 0 404s.
- **The corrected G2 test.** A Tender qualifies if it is outside population.json and has a lot-placed result from a
  notice that has no lot section of its own. That notice's profile must be `ted-export-r208` (from
  `/v1/notices?tender=`), and "no lot section" is confirmed from its `/v1/notices/{id}/content`.
  - The tool is a copy, `ext/draw_fresh30_g2fix.py`. `draw_fresh30.py` is unedited.
  - Run offline over fresh568's cache, the test reproduces my 16:58Z split exactly: 5 G2 Tenders and 3 false ones.
- **Held.**

  | Tender | walk index | year | qualifying notice | R2.0.8 notices with a lot-placed result | 568 blocks |
  |---|---|---|---|---|---|
  | 5048894 | 805 | 2015 | 18258276 | 18258276 | 24 |
  | 4836560 | 1891 | 2013 | 17517331 | 17517331, 17517659, 17518079, 17518149, 17657578 | 20 |
  | 5226033 | 2134 | 2015 | 18033024 | 18033024, 18242004, 18527820, 18527875 | 22 |

  Every notice in the 3 chains is `ted-export-r208` (R2.0.8.S02.E01), and every lot-placed notice has 0 lot sections.
- **Checks on the walk.**
  - The verifier re-walked it with its own code and fresh GETs, and matched the extender on 1,364 of 1,364 rows.
  - Recall: across the 1,144 read Tenders, exactly 10 lot-placed notices are R2.0.8, and all 10 are in the 3 held.
  - **Judge's check:** I re-derived the salted order offline from fresh568's changes pages. It reproduces fresh568's
    rows 0–770 and the extension's rows 771–2134. Of the 1,144 read Tenders outside population.json, exactly 3 have
    an R2.0.8 lot-placed notice, and they are the 3 held.
- **The draw file.** `ext/draw_ext.json` holds 30 ids: 27 + 3, complete=true, with the cursor range recorded. The 30
  hold 21 OUT and 8 G2. It was written at 18:07:28Z, before the bar read started at 18:07:56Z.

### Parity (bar (i))

- **The run.** `check_live.py --bar` read all 30 ids with a new empty cache, `--drain-job 2617 --drain-finished
  2026-10-18T15:19:48+00:00`.
  - It ran 18:07:56–18:13:17Z: 201 GETs, 0 fetch errors, 30 of 30 Tenders read, 0 incomplete.
  - Result: **417/417 blocks at parity.** 0 breaks, 0 computed-not-served, 0 served-outside-chain, 0 non-legacy
    mismatches, 0 legacy-test disagreements, 0 prefix fallbacks. `pass_bar_i=true`.
- **Where the blocks are.** 351 are on the 27 kept Tenders and 66 on the 3 new ones (24 + 20 + 22).
  - fresh568's 878 blocks included 527 on the three dropped Tenders: 6324865 45, 6788159 460, 6869520 22.
    878 − 527 = 351.
- **The verifier's fresh re-read** of the 3 new Tenders (18:25Z, check mode, committed tool): 66/66, and the Tender
  JSONs are byte-identical to the bar cache.
- **Stability of the 27.** Every served `lot_result` and every Tender version equals fresh568's 15:43–15:53Z read: 0
  differences, 0 version changes. I recomputed this from the raw caches.
- The changes feed still ends at 958764334 with more=false.

### Labels

Every 568 cross-notice block of the 3 new Tenders was labelled from the notices, by the unit-7 criteria.

- **The extender** labelled from `view.py`, which shows only the pre-guard replay lot.
- **The verifier** labelled from its own notice view, with no served lot and no guard.
- **They agree on 66 of 66 blocks**, on both label and tentative lot. I read 4836560's and 5048894's notices myself
  and concur.
- **Disclosure, accepted.** The walk's G2 evidence counted 19 lot-placed R2.0.8 results on 4836560, against 20
  blocks. That implied one block was served null, but not which. It cannot bias (ii): both independent labellings put
  all 66 at single_right.

| Tender | notice | printed → tentative lot | blocks (RES-) | label | served |
|---|---|---|---|---|---|
| 5048894 | 18258276 | 1 (Hardware) → LOT-2 | 1–6, 19 | single_right | LOT-2 |
| 5048894 | 18258276 | 2 (Special Seating) → LOT-1 | 7, 8, 12, 14, 15, 17, 20, 22, 23 | single_right | LOT-1 |
| 5048894 | 18258276 | 3 (Cushions) → LOT-3 | 9–11, 13, 16, 18, 21, 24 | single_right | LOT-3 |
| 4836560 | 17517331 | 3, 15, 16, 17 → same | 1–4 | single_right | own lot |
| 4836560 | 17517659 | 8, 10, 11, 12, 13, 18, 19 → same | 1–7 | single_right | own lot |
| 4836560 | 17518079 | 1, 2, 5, 20 → same | 1–4 | single_right | own lot |
| 4836560 | 17518149 | 4, 6 → same | 1, 2 | single_right | LOT-4, LOT-6 |
| 4836560 | 17518149 | 14 → LOT-14 | 3 | single_right | **null (G5)** |
| 4836560 | 17657578 | 7, 9 → same | 1, 2 | single_right | own lot |
| 5226033 | 18033024 | 1–13 → same | 1–13 | single_right | own lot |
| 5226033 | 18242004 | 3, 4, 5, 9, 12, 13 → same | 1–6 | single_right | own lot |
| 5226033 | 18527820 | 12 → LOT-12 | 1 | single_right | LOT-12 |
| 5226033 | 18527875 | 4, 9 → same | 1, 2 | single_right | LOT-4, LOT-9 |

In 5048894, the anchor CN 17545429 prints Hardware as 1 on section LOT-2, and Special Seating as 2 on section
LOT-1. The 2014 CN prints the same numbers, so blocks printed 1 belong on LOT-2. Per-block evidence is in
`readback_result.json` → `O1.labels` and in `ext-verify/labels_verify.json`.

**The contested label: 4836560 17518149/RES-1 → LOT-4. I rule it single_right.**
- The block prints 4: "Lot 04 : Etanchéité - zinguerie", contract 13003, MJP Bardage.
- The anchor CN 17101010's section LOT-4 prints 4, "Lot 04: étanchéité - zinguerie". LOT-4 is the only key lot 4
  ever had, so the block sits on its own lot.
- But LOT-4 serves lot 14's text. Relaunch CN 17244355 ("lots 4,6,7 et 14") prints LOT_NUMBER 1..4 by position. Its
  section LOT-4, printed 4, is "lot 14 : Equipements de cuisine". That section lands on LOT-4 whether it is joined by
  section id or by printed number, and no later notice re-titles LOT-4.
- So this is a lot-text carry defect. The block did not cause it, and neither 568 nor 570 can see it: 570 does not
  move that section. The unit-7 labels judge the block's lot identity, not the carried text.
- **Sensitivity.** Labelled wrong_other, (ii) would read 1/30, still a pass, and (iii) would record an inherited
  kept shape. The verdict does not depend on this ruling.

### The bar over the 30

| | the 27 (fresh568) | the 3 (O1) | the 30 |
|---|---|---|---|
| blocks at parity | 351/351 | 66/66 | **417/417, 0 breaks** |
| 568 blocks labelled | 112 | 66 | 178 |
| single_right / multi_lot_contract / wrong_other | 106 / 4 / 2 | 66 / 0 / 0 | 172 / 4 / 2 |
| kept: right / wrong | 105 / 0 | 65 / 0 | **170 / 0** |
| refused: right / wrong | 1 / 6 | 1 / 0 | 2 / 6 |
| Tenders with a wrong block, before guards → after | 3 → 0 | 0 → 0 | **3 → 0 of 30** |

- **(i)** 0 parity breaks. Pass.
- **(ii)** 0 of 30 Tenders keep a judged-wrong block, against a limit of 2. Under the contested reading it is 1/30.
  Pass.
- **(iii)** No kept wrong block, so no new kept wrong shape. Neither reopen trigger fired (a break, or 3 or more of
  30). Pass.
- **The quota.** 8 G2 against at least 8; 21 OUT against at least 10.

### The G2 stratum: what 570 newly matched

All 3 new Tenders exercise 570 through a G2 refusal at base, so the stratum is **8 of 8**. It was 5 of 8.

| Tender | blocks moved null → lot by 570 | labels | served |
|---|---|---|---|
| 5048894 | 24 | 24 single_right | 24 on their lot |
| 4836560 | 19 | 19 single_right | 19 on their lot |
| 5226033 | 22 | 22 single_right | 22 on their lot |

- 65 blocks moved, and all 65 are right. No wrong block was newly matched.
- A 66th block, 4836560 17518149/RES-3, is newly matched before the guards: tentative LOT-14, which is right. G5
  refuses it, so it serves null. That is a false refusal with cause "unchanged" (null at base by G2, null now).
- Across all 8 G2 Tenders: 114 blocks moved, all right. Two newly matched blocks were refused by G5: 5864590 RES-3,
  which is wrong, rightly refused; and 4836560 RES-3, which is right, falsely refused.

### Recorded, not a reopen

1. **A G5 false-refusal shape** (for the 568 decision record). A positional relaunch CN carries the true lot's title
   onto another lot's key. Here, lot 14's title sits on LOT-4, so the lot-14 award's title equals exactly one other
   lot's title, and G5 refuses it.
   - Exhibit: 4836560 17518149/RES-3.
   - It is not a regression: the block was null before the drain.
   - Pooled false refusals: 2 of 2,253 right blocks (0.09 %). That is the earlier 2,187 plus these 66.
2. **An inherited lot-text carry defect** (an exhibit for the lot-carry class).
   - The shape: a relaunch CN prints LOT_NUMBER 1..k by position for a subset of lots.
   - 4836560's LOT-4 serves lot 14's title and description, and LOT-14 keeps only the anchor's description.
   - The section id equals the printed number, so neither today's merge nor 570's remap can tell. No 568 or 570 fix.
3. **Corrections to the extender's notes** (the verifier's, checked by me).
   - 5048894 is a relaunch linked by the buyer's own references, not a weld of another procedure. 17923471
     discontinues CN 2013/S 161-280705, and the T034 CN 18025780 cites it.
   - 5226033's no-award blocks (clos-nw) serve no winners, and no party named "infructueux" is served.

## 570: items

| item | result | evidence (short) |
|---|---|---|
| (a) D7 | pass | same run as 568 |
| (b) 8375276 | pass | 124 blocks print 1 and serve LOT-1; 30 print 2 and serve LOT-2; 120 + 27 moved, equal to port_result |
| (c) 8 + 5 changed | pass | 239/239 on the B lot; 13 key sets unchanged; the 9 named Tenders equal the baseline |
| (d) other 196 | pass | 196/196 equal the baseline. The only difference, 5602216 RES-128 → null, is 568-u8's G5, not 570's |
| (e) E1 | pass | 142/142 moved blocks on the key whose head number is their own |
| (e) E2 | pass | 182/182 sections agree with their target as step 3 defines it |
| (e) E3 / E4 | pass | key sets equal with the remap on and off; 313/313 blocks at parity |
| (e) bar | pass | 30/30 |
| (f) | pass | 0 movers |
| (g) | pass | 862,889 lot events, all changed/null; 0 removed, 0 added |

**(e) 6876932.** This chain welds yearly re-tenders by one buyer from 2020 to 2023, with two interleaved lot
numberings.
- Every one of its 34 moved sections joins the carried lot that prints the same number and title. "Produits surgelés"
  prints 3 in every notice. So no number names two lots, and nothing here is a wrong remap.
- The mismatch with the anchor CN's titles comes from K1-refused notices: 21923843, 22231117 and 22219592 re-keyed the
  carry by today's positional merge, at v4, v6 and v7. Rule step 5 keeps refused notices on today's fold by design.
- With the remap, 0 keys are renumbered at v8. Without it, 17 are, and two numbers would each sit on two keys.
- No award moved, and the served head is the same as without the remap.

**6298189/21804358** agrees. Its section carries the procedure title, but its MAIN_SITE and framework number are
LOT-11's.

## Open items and dispositions

- **O1 (568): fix. Done 2026-10-18.** The extension ran and was verified, and the bar over the 30 passes. 568 PASS;
  see "O1 extension" above. The pre-registered instructions follow.
  - Keep the 27: walk indices 0–21, plus 61, 282, 366, 449 and 770.
  - Continue the same `568u8-fresh30-` walk from index 771, with the same exclusions, skipping members of
    population.json unread.
  - Hold the next 3 Tenders that pass the corrected G2 test: outside population.json, and with a lot-placed result
    from a notice that has no lot section of its own (`ted-export-r208`, confirmed from `/v1/notices?tender=`).
  - Read them with `check_live.py --bar`, and label every 568 cross-notice block from the notices before looking at
    guard outcomes. Then run an adversarial verify.
  - 568 passes on 0 parity breaks and at most 2 of 30 Tenders with a kept wrong block. It reopens on a break, or on 3
    or more.
- **O2 (568): accept and record.** A new false-refusal shape. 5457789 18461129/RES-8 has a copied CONTRACT_TITLE that
  names pakiet 34, while LN, CONTRACT_NUMBER, an estimate equal to the cent, and the winner all name lot 37. G4
  refuses it at block level. It was LOT-37 before the drain and is null now.
  - Pooled false refusals are 1 of 2,187 (0.05 %). A refusal is pre-568 behaviour, never a wrong lot.
  - Record the shape in decision-u7.md. Change no guard without a measured control.
- **O3 (568): accept.** 5602216/19023719 RES-128 was refused by G5. The CN prints identical text for lots 172 and 173,
  so whether the block was right cannot be decided. Record it as an unclear refusal.
- **O4 (570): file on 364 and record.** File 6876932 as weld evidence on issue 364. Add a caveat to 570's decision:
  "each moved id returns to the identity its first notice gave it" fails where K1-refused notices re-keyed the carry
  first. No 570 fix is needed.
- **O5 (570): record the wording.** Correct the 570 decision's D3 sentence to match ADR-0017 rule L, which is what is
  built. No code change.
- **O6 (follow-on): file.** 8371908 CAN 21510251 RES-1 is a EUR 2.97 bn award. It prints "Lot No. 3 - Lot 1C …" and
  serves null both before and after the drain. It is an exhibit for 570 Unit 7, 567's single-section no-number
  placement. The notice's only section, LOT_NO 3, is now remapped to LOT-3.
- **O7 (tooling): fix in O1, and record.**
  - draw_fresh30.py's G2 signal fails its precision claim ("0 of 2,951") on multi-CAN R2.0.9 chains: a Tender-scope
    winner from another result fires it. O1 replaces the signal.
  - check570.py's anchor-title reference goes stale after K1-refused notices. It is a proxy for the carried lot.
  - **Recorded (2026-10-18).**
    - draw_fresh30.py's G2 signal is superseded by the corrected test used in O1: outside population.json, with a
      lot-placed result from a notice that has no lot section of its own (`ted-export-r208`, confirmed from
      `/v1/notices?tender=`). No `ext/` copy of the tool had been written when this was recorded. A later draw uses
      O1's test, not the one-read signal.
    - **O1 used it (19:35Z).** The test lives in `ext/draw_fresh30_g2fix.py`, a copy; `draw_fresh30.py` is unedited.
      In the extension's walk, the old signal would have fired falsely on 15 skipped Tenders, the first 6193033 at
      index 986.
    - `fresh570/check570.py` E2's anchor-title reference (`ref_title`, `use_anchor=True`) holds only where no
      K1-refused notice precedes the remapped one, because a refused notice re-keys the carry. After one, the
      carried-lot reference (`use_anchor=False`) is the valid one, as (e)'s reading of 6876932 used.

## Residual shapes recorded

- **568, caught and not kept, fresh 30.**
  - (a) One block per contract covering several lots, printing the first. The Bulgarian plural "обособени позиции №
    …", 8291946 ×4, refused by G3. This is unit-7 shape (1).
  - (b) CAN-only renumbering in a one-block CAN, where the title and value are another lot's: 8365595, G5.
  - (c) A CAN that repeats a printed number on another lot's award: 5864590 RES-3, G5. It is newly matched because 570
    narrows G2.
- **568, kept wrong blocks:** none in the conformant 30, O1's 3 included. None of the five recorded residual shapes
  was met.
- **568, false refusals:**
  - the copied-title-against-the-number shape (O2);
  - O1: a positional relaunch CN carries the true lot's title onto another key, so G5 refuses the true lot's award
    (4836560 17518149/RES-3; null before the drain too).
- **Inherited, outside 568 and 570:** the positional-relaunch lot-text carry (4836560 LOT-4 serves lot 14's text).
  It was met once, in O1.
- **570:** the welded re-tender with interleaved numberings (O4, issue 364's class). Seen 0 times in 570's 211 chains;
  1 in the fresh 30.
- **Band caveat:** none of the 45 corrected band Tenders is in the baseline's lot snapshots, so their block
  attributions take the before-state from the replay. The value, derivation and version diff that the bar reads is
  observed on both sides.

## What to file

1. 568: the O1 extension. **Done**: the read-back is complete and 568 passes, so 568 can be marked done on this
   read-back.
2. 568 decision record:
   - O2's false-refusal shape;
   - O3 as an unclear G5 refusal;
   - O1's G5 false-refusal shape (4836560 17518149/RES-3), with pooled false refusals now 2 of 2,253 (0.09 %).
3. Lot-carry follow-on: file 4836560 (relaunch CN 17244355, LOT-4 carrying lot 14's text) as an exhibit of a
   positional relaunch numbering. Neither 568 nor 570 can fix it.
4. 364: 6876932 as weld evidence (O4).
5. 570 decision record: the identity caveat (O4) and the D3 wording (O5). 570 can be marked done on this read-back.
6. 570 Unit 7 follow-on (567 single-section placement): 8371908 RES-1 as an exhibit (O6).
