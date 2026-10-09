# 507 — a carried PIN Part's figure is elected as the head beside the CN's Lots

Status: done — DEPLOYED AND DRAINED 2026-10-09 (`5b7ef55`, gate green).
- **Cohort.** A scan of the whole id space (884 windows of 10k ids, all answered) found 179 Tenders whose head equals
  a Part's figure in a version with a Lot. Their head notices went to `refold-notices` (jobs 2109 and 2110).
- **Fold.** 179 written: 134 verified, **45 corrected**, 415 correction rows.
- **Re-read** (`507-carried-part/drain-2026-10-09.json`):
  - All 45 moved down, none to no value. The median move is 1.1× (a PIN estimate slightly above the CN's own); the
    largest is 1,078×.
  - 8818621 → €1.19 m; 8819392 → €649 k; 8819552 → €44.6 m (RON 238.6 m); 8821187 → €3.31 m (RON 17.3 m);
    8813624 → €107 m (DKK 800 m).
  - 8819870 kept its only figure; 8819939 stayed at €12 m (the review's case).
History: UNIT 1 MEASURED, UNIT 2 BUILT 2026-10-09 (`5b94f9c`, review fix `5b7ef55`).
- **Measured.**
  - Heads ≥ €1 bn: 3 elected from a Part, 1 of them beside Lots (8818621).
  - Heads €100 m–1 bn (31,094): 19 from a Part, 5 beside Lots (8819392, 8819552, 8819870, 8821187, 8813624).
  - Id-window samples: Part+Lot head versions sit almost only in ids 8.75–8.837 M. Those are the Tenders that issue
    481's PIN→CN links have joined since 2026-10-05. In the 8.81 M and 8.82 M windows, 29 and 19 heads come from a Part.
- **Read (the 6 at ≥ €100 m).** 5 of 6 are wrong:
  - 8818621 is garbled; 8819392 and 8821187 are ×1000.
  - 8819552 is a stale PIN figure far above the CN's total.
  - 8813624 is an EUR PIN part outranking the CN's DKK total.
  - The sixth, 8819870, has the Part as the version's only figure, and it should stay.
- **Built (option 1, refined).** In a version with Lots, the head election runs first without the Parts. It falls
  back to including them only when nothing else is admitted. The Part's own stored lot value is untouched. Pinned by
  `a_carried_part_is_no_head_candidate_beside_lots_that_state_a_figure`.
- **Review** `wf_290409c2-315`: 2 confirmed.
  - (major) The first build (`5b94f9c`) skipped Parts whenever another figure merely existed, even one the election
    refuses. 8819939 (CN €1.2 bn ×2, refused as ×100 its PIN part's €12 m) would have dropped from €12 m to no value.
    Fixed: "admitted" is now the election's own test, so it elects without Parts first and falls back to them.
    Pinned: the 8819939 shape, and an unconvertible CN figure.
  - (minor) /docs and the CHANGELOG had no entry; both now do.
Was: needs-triage — filed 2026-10-09 from issue 505's adjudication (8818621).
Kind: correctness / the head election (`head_value_eur_cents_with`)
Relates to: 492 (since its review, the framework exemption counts Parts as lots only in a head with no Lot, because the
fold never drops a lot and a PIN's PAR-* parts stay in the head beside the CN's LOT-* lots), 505

## What was seen

8818621's head (€1.12 bn) is the figure of a PIN Part (PAR-*) carried into the CN's version, with garbled digits. The
true value is €1,187,620. The CN's own Lots and its procedure figure are far smaller.

The head election takes the max over every lot of the head version, Parts included. A Part carried from a PIN describes
a planning-stage figure, and the CN usually restates it under its own LOT-* key. 492 already stopped counting such Parts in
its lot sum and as sibling partners. The head election still elects them.

## Options

1. Elect a Part's figure as head only in a version with no Lot (a PIN head). A CN head ignores carried Parts, while
   the stored lot value of the Part row stays as it is.
2. Leave it, and caveat it in /docs.

## Units

1. **Measure.** Count heads ≥ €1 bn (and a sample below) whose elected figure is a Part's in a version that also has
   Lots. Adjudicate a sample.
2. Decide, build, drain (`refold-value-band` reaches only ≥ €1 bn; a broader change needs its own cohort).
