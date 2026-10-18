# Combined read-back: 568 unit 8 + 570 (refold-fields 2616 + project 2617). Judge, 2026-10-18

The drain finished at 15:19:48Z. 1,250,447 Tenders were written: 1,203,568 verified and 46,879 corrected. The changed
set is the 46,879 version-null tender events with 957854566 < cursor <= 958764334. Three stages paged it separately
and got the same sha256 (8062ccd1…).

There were four stages: sets, fresh568, fresh570 and band. Each had an adversarial verifier. Per-item results, evidence
and files are in `readback_result.json`. I made no GETs: every judge check below is offline, over the stage and
verifier caches and the decision texts.

## Verdicts

- **568: OPEN. Not failed, not yet passed.** Every item that was read passes:
  - (a) D7;
  - (b) 8327340;
  - (c) the s150's 57 refused blocks;
  - (d) the 77, 692/692;
  - OOS 50;
  - (f) the band;
  - the amended bar: (i) parity 878/878, (ii) 0/30, and (iii) no trigger.

  The bar's draw requires **at least 8 G2 Tenders**, and the draw holds 5. The cause is a defect in
  `draw_fresh30.py`, not in the build. 568 closes on the pre-registered 3-Tender extension (O1 below).
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
| fresh 30: draw | pass | written 15:42:26Z, before the bar read; reproduced offline; 24 of 30 outside 2012–2015 |
| fresh 30: ≥ 8 G2 | **open** | 5 of 8; see O1 |
| bar (i) parity | pass | 878/878, 0 breaks; verifier recomputed offline and re-read live |
| bar (ii) ≤ 2/30 kept-wrong | pass | 0/30. 6 wrong blocks, all refused (G3 ×4 on 8291946; G5 on 8365595 and 5864590). Tenders with a wrong block: 3/30 before the guards, 0/30 after |
| bar (iii) | pass | no kept wrong block; no trigger |
| (f) band / path-V | pass | 3,728 heads → 3,728; 0 movers; 17 path-V heads unchanged |

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

- **O1 (568): fix.** Run the G2 extension. This is the only thing between 568 and closing.
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
- **568, kept wrong blocks:** none in the 30. None of the five recorded residual shapes was met.
- **568, false refusal:** the copied-title-against-the-number shape (O2).
- **570:** the welded re-tender with interleaved numberings (O4, issue 364's class). Seen 0 times in 570's 211 chains;
  1 in the fresh 30.
- **Band caveat:** none of the 45 corrected band Tenders is in the baseline's lot snapshots, so their block
  attributions take the before-state from the replay. The value, derivation and version diff that the bar reads is
  observed on both sides.

## What to file

1. 568: the O1 extension as the next unit (read-back completion), with `Blocked by:` none.
2. 568 decision record: O2's false-refusal shape, and O3 as an unclear G5 refusal.
3. 364: 6876932 as weld evidence (O4).
4. 570 decision record: the identity caveat (O4) and the D3 wording (O5). 570 can be marked done on this read-back.
5. 570 Unit 7 follow-on (567 single-section placement): 8371908 RES-1 as an exhibit (O6).
