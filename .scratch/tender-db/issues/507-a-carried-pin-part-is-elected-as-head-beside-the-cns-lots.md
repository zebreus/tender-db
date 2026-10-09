# 507 — a carried PIN Part's figure is elected as the head beside the CN's Lots

Status: needs-triage — filed 2026-10-09 from issue 505's adjudication (8818621).
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
