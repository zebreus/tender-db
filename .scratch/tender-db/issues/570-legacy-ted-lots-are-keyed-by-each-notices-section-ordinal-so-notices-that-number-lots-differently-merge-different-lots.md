# 570 — legacy TED lots are keyed by each notice's section ordinal, so notices that number their lots differently merge different lots under one key

Status: needs-triage — filed 2026-10-17 from issue 567's read-back (workflow wf_8e33e7af-3be: readers and skeptics
flagged it independently on 5 of 45 tenders). Next: measure the reach (multi-notice legacy TED Tenders whose notices
print different LOT_NO → section-ordinal maps, and how many award blocks / lots sit under a key whose Tender title is
another lot's), then decide whether legacy lots may be keyed by their printed number across a chain.
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
