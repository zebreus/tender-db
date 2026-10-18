# 570 — legacy TED lots are keyed by each notice's section ordinal, so notices that number their lots differently merge different lots under one key

Status: done — 2026-10-18. Built (wf_1da4edb1-7be), gated cd26e41, deployed 03bdf4d, DRAINED in one refold with 568
unit 8 (refold-fields 2616 + project 2617: 1,250,447 written = 1,203,568 verified + 46,879 corrected; D7 holds), READ BACK
(wf_f6e14299-ff8, `568-r208-lots/readback-u8/readback.md`): (a)–(g) all pass — 8375276 serves LOT-1 "Lahve…" / LOT-2
"Kojenecka voda" with all 124 LOT_NO-1 blocks on LOT-1 and 30 LOT_NO-2 on LOT-2 (147 moved, = port); the 8 + 5 changed
Tenders 239/239 on their B lot, key sets unchanged; the other 196 equal the baseline; fresh 30 (salt "570-fresh30-")
30/30; band / path-V 0 movers; 0 lot removed among 862,889 lot events. Recorded: the K1 re-keying caveat (6876932, a
weld — evidence on 364), the D3 announcement wording, 8371908 as a unit-7 exhibit. Follow-on: unit 7 (567's
single-section no-number placement) — NOT before 551 B5's band seek (551 read18 note 2).
Decision: `.scratch/tender-db/570-lot-keys/decision.md`.
Kind: correctness (legacy lot identity: awards and lots merged across different lots)
Relates to:
- 567 (same-notice mapping by printed LOT_NO; correct within a notice — this is the cross-notice keying it rests on);
- 568 (cross-notice match for R2.0.8 awards against the inherited lots' printed numbers);
- 551 (path W's S3a–d shapes "TED lots keyed by ordinal LOT-k instead of LOT_NO"; unit 4b's R2 guard exists because of
  this; unit X would have filed it);
- 364 (chain welding).

## Verify

    curl -s https://tenders.zebreus.click/v1/tenders/8375276 | python3 -c "import json,sys; d=json.load(sys.stdin); print([(l['key'], l.get('title')) for l in d['lots']]); from collections import Counter; print(Counter(r.get('lot') for r in d['lot_results']))"

- **done**: an award whose notice printed `LOT_NO n` serves on the Tender lot that the chain identifies as lot n, in
  every notice of the chain (or the issue records why a cross-notice key is refused).
- **open (2026-10-17)**: 8375276's lots[] (from notice 21337435) say LOT-1 = "Lahve a savicky", LOT-2 = "Kojenecka
  voda"; eight 2018–2020 award notices print LOT-1 = LOT_NO 2 ("Kojenecka voda") and LOT-2 = LOT_NO 1 ("Lahve…"), so
  their LOT_NO-1 awards serve on Tender LOT-2 under the other title.

## What was found

TED R2.0.9 lot sections are keyed `LOT-k` by their position in the notice (the r209 walker's section id / ITEM), and
the fold merges a Tender's lots — and puts each notice's award blocks — by that key across notices. Issue 567 now maps
each block to the right section of ITS OWN notice; where two notices of one Tender put different lots at the same
position, that section's key still names a different Tender lot.

From 567's read-back (all within the rule; the misplacement is at Tender level):
- **8375276** (12 award notices, 155 blocks): eight 2018–2020 notices swap lots 1/2 against the Tender's lots[] (from
  notice 21337435). 147 blocks are affected; by Tender lot title 567 made 120 worse (they followed their own notice's
  LOT_NO onto its section LOT-2, which is the Tender's "Kojenecka voda" lot) — skeptic-confirmed counts: 120 LOT_NO-1
  blocks on LOT-2 (110 titled "Lahve"), 27 LOT_NO-2 blocks on LOT-1 (25 titled "Kojenecka").
- **8387180**: notice 19043870 keys its 7 lots (printing 19, 26, 27, 28, 29, 16, 17) LOT-1..7; the Tender's LOT-1..7
  titles come from notice 19152735 (23 lots), and the contract notice 18927620 has those lots at LOT-19, LOT-26 … .
  Unchanged by 567.
- **8480304**: notice 21927890's 7 lots (printing 3, 5, 8, 9, 10, 16, 30) are keyed LOT-1..7, so the "Zad.3" award sits
  on the Tender's LOT-1 titled "Zad.1"; the contract notice 21788395 has LOT-k = Zad.k. Unchanged by 567.
- **7697322**: award notice 23276289's single lot section (printing `02`, terrassement) is keyed LOT-1, the Tender's
  "v.r.d" lot (lot 01 of notice 23413021).
- **7415397**: award notice 22583013's single section LOT-1 prints LOT_NO 2 (the Tender's LOT-2); its no-number block
  goes on none under 567, so no harm there, but the same keying.
- 7025548: the contract notice 21593860 numbers 10 sections differently from the award notice (CN LOT-12 = LOT_NO 9,
  award LOT-12 = 19); the Tender's lots follow the award notice, so its awards line up — the CN's lot facts may not.

A related cost of 567's decided rule (recorded there, depends on this issue): a block that prints no number on a notice
whose only lot section prints a number other than 1 now goes on none (6120045: the only lot of a one-lot Tender, block
title = lot title; 7025760: the only award of a one-lot Tender). Placing it on that section would be right within the
notice but, with ordinal keys, can land on another Tender lot (7415397's shape) — so any refinement waits for this.

## Proposed first unit (measure)

A bounded sample of multi-notice legacy TED Tenders (R2.0.9, 2+ notices with lot sections): per Tender, each notice's
section-ordinal → printed-number map; whether the maps disagree; per disagreeing Tender, how many award blocks and lot
facts sit under a key whose Tender lot (title / printed number) is another lot. Then decide: key legacy lots by printed
number (normalised) across the chain, with guards for unnumbered sections and duplicate numbers — an identity change
for lot keys (ADR-0017 implications: lot ids, change rows, subscriptions) — or a cross-notice remap at fold time, or
refuse and record.

## Unit 1 — measured and decided (2026-10-18, workflow wf_0271cb46-6c5)

Sample: issue 567's part-1/2 draws (one multi-lot legacy CAN per random OJ S day, 2016–2023): 204 Tenders (182 R2.0.9,
22 R2.0.8); every version notice read; measurer + adversarial skeptic per group of 17; judge with a port replay
(`remap_port.py`, `port_result.json`: today's rule equals production on all 1,316 placed blocks).

| class | Tenders | misattributed results | lot facts on another lot |
|---|---|---|---|
| single_lot_notice | 47 | 0 | 0 |
| consistent | 143 | 0 | 0 |
| disagree_harmless | 6 | 0 | 0 |
| disagree_harmful | 7 | 9 | 22 |
| unnumbered | 1 | 0 | 0 |

Decision (full rule, tests, drain and read-back: `decision.md`): **BUILD B** — not A (re-keying by printed number would
change keys in 13 % of Tenders and mint/sweep ids in 5 %, 19 of 27 for no gain), not C (leaves the measured harm). Raw
API reads of the measurement and skeptics: `raw-cache.tar.gz` (unit 3's tests are built from these parsed values —
extract it into this folder first). Interactions: 567 (B rests on its same-notice resolution), 568 (shared matcher; two
568 test assertions are re-specified on purpose because a swap is now resolved), 551 (path W's R2 inputs change; the port
must apply the remap; B5's flip waits for this drain and B3's re-run), 364 (two welded Tenders are recorded there).

## Follow-on exhibit (2026-10-18, read-back O6)

For decision.md Unit 7 (567's single-section no-number placement, beside 6120045 and 7025760): **8371908** CAN 21510251
RES-1, a EUR 2.97 bn award, prints "Lot No. 3 - Lot 1C …", which 567's same-notice normalisation does not read, so it
serves `lot: null` before and after the combined drain. The notice's only lot section, LOT_NO 3, is now remapped from
its own LOT-1 to LOT-3. Read-back: `568-r208-lots/readback-u8/readback.md`.
