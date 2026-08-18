# 235 — results-materialisation density measures the projection against itself, so it cannot see an award notice with no results

Status: needs-triage — found 2026-08-18 in the first complete eleven-query report (job 732)
Kind: metric definition defect (a green that cannot go red)
Blocked by: —
Relates to: 27 (the report), 230 (the measurement that made this visible), 13 (results layer), 100
(DE-1.x winners — a real materialisation gap this metric would not have caught), 226/228 (the same
lesson: a signal that cannot distinguish two states is not a signal)

## What

The first complete run of the data-quality report puts section 3 at **exactly 100.0 % for all twenty
eras that measure at all**, numerator identical to denominator digit for digit, including eras of
1.07M and 2.10M versions:

    eforms-sdk-1.7                    148,697        148,697   100.0%
    TED_EXPORT r2.0.8               1,073,760      1,073,760   100.0%
    TED_EXPORT r2.0.9               2,098,634      2,098,634   100.0%

A metric that reads 100.0 % on every era at that scale is not a clean bill of health, it is a metric
that cannot fail. And the reason is in its definition, not in the data.

## Why it cannot fail

Both halves are keyed on the SAME fact:

- **Denominator** (`DENSITY_CAN_SQL`): versions whose notice carries a result SECTION in the parsed
  layer (`notice_sections.kind IN ('LotResult','TenderResult')`).
- **Numerator** (`DENSITY_WITH_SQL`): versions whose `(tender_id, notice_id)` carries a `lot_results`
  ROW in the canonical layer.

The projection writes a `lot_results` row *from* those sections. So section 3 asks "did the projection
write what it parsed" — a genuine invariant, and worth having — but it is NOT the question the report
was built to answer, which was **"do contract-award notices actually materialise their results?"**

An award notice that parsed with **zero result sections** — the actual failure mode — is excluded from
both sides and is therefore invisible. The metric's denominator is "notices that already have results
in the parse layer", so by construction it can never count a notice that has none.

## The evidence that this is a definition problem, not good news

Three things line up:

1. Uniform 100.0 % across twenty eras of wildly different vintage and quality. Nothing else in this
   report is uniform — field completeness ranges from 0.5 % to 100 %, linkage from 0.3 % to 100 %.
2. The module's own docstring still cites the finding that motivated it: *"eForms CANs materialise
   results at 0.3 %"*. Either that has been fixed (plausible) or the old measurement was different in
   kind (it was: the old numerator counted DISTINCT notices from `lot_results` while the denominator
   counted versions — mismatched units, fixed 2026-08-18). Either way the docstring now states a number
   the report contradicts, and that line needs correcting.
3. Issue 100 is an open, confirmed materialisation gap — DE-1.x award winners unresolved — and section 3
   reports `eforms-de-1.1: 60,037 / 60,037 = 100.0%`. A metric that reads perfect on an era with a
   known open results defect is demonstrating the blindness described above.

## The fix direction

The denominator has to come from what the notice IS, not from what it already carries:

- `notices.notice_subtype` for eForms (the CAN subtypes), and the legacy `TD` type code for the text /
  r208 / r209 / internal-ojs eras (`TD: 7 - Contract award` in the text era's vocabulary). The eras
  name award-hood differently, so this needs a per-era vocabulary — and per the lesson one query above
  it, that vocabulary must be a named list checked by a test, not a literal buried in SQL.
- Keep the current pair as a SEPARATE row or section under an honest name: "sections → rows", the
  projection-writes-what-it-parses invariant. It is cheap and it did catch a real defect the same day
  (sdk-0.1's `TenderResult` sections falling out of the denominator), so it should not be deleted —
  just stop calling it density.

Cost note: both new denominators are `notices`-driven with an indexed profile/subtype read, so they
should window on `tender_id` exactly like the current pair (issue 230).

## Acceptance

- Section 3's denominator counts award NOTICES by their own type, per era, from a tested vocabulary list.
- The sections→rows invariant survives under its own name.
- The module docstring's "0.3 %" claim is either re-verified or removed.
- Issue 100's DE-1.x cohort no longer reads 100.0 % while its winners are unresolved — or, if it
  legitimately does, the report explains which question that number answers.
