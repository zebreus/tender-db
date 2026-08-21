# 267 — amount plausibility is a one-off analysis, not a standing measure

Status: needs-triage — filed 2026-08-21 (owner, requested by Lennart: investigate data quality +
more monitors).
Kind: data-quality measurement (new section for the weekly run)
Blocked by: —
Relates to: 131/132 (source-published negative amounts, row-by-row diagnosed), 134 (the ratio-form
invariant this operationalizes), 136 (the 407 residue measurement), 251 (VAT basis — the adjacent
"is this amount usable" work)

## The gap

The negative-amount investigations (131/132/134/136) established two things and then STOPPED
measuring: (a) negative and absurd amounts exist and are overwhelmingly source-published, and
(b) the honest invariant is a RATIO per era (issue 134's form), not a magnitude cap. Nothing
watches that ratio today. A parser regression that started fabricating negatives (the 407-class
mechanical error), or a source that started shipping garbage at scale, would be invisible until a
human repeats the one-off analysis.

## What to build

One windowed query in the weekly run (the 109/251 pattern — one more label, auto-covered by the
equivalence gate), per era over `tender_version_amounts`:

    amounts, negative, zero, over_1e12   (counts; the render shows rates)

as section 8 of the report, with the 131/136 verdict as its standing caveat ("negatives are
overwhelmingly source-published; the RATE moving is the signal, not their existence") — and the
per-era rates joining 265's headline history so a step change shows as a delta like every other
headline. The absolute threshold (1e12) is deliberately crude: it is not a correctness claim, it
is a tripwire for the "unrepresentable-value" class escaping quarantine into the layer.

## Acceptance

- The section renders per era with the caveat; rates land in the headline history.
- Backtest: the recorded 131-era numbers (51 negative estimated/framework amounts as of
  2026-08-05, r2.0.8-heavy) are reproduced by the query within the same order of magnitude.
- A fabricated-negative regression in a test fixture moves the rate and (with 265) the delta.
