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

## Groundwork (2026-08-18, owner): where award-hood actually lives, per era

Read from the code and the vendored inventories, so the next session starts from facts rather than
repeating this. The short version: **the signal exists in every era, and in four of six it is not
projected**, which changes what the fix should be.

1. **`notices` has no subtype column.** The fix cannot be a one-line predicate on `notices`.
2. **`tender_versions.notice_subtype` DOES exist** and is written by the fold from
   `first_code(parsed, SUBTYPE_FIELD)` where `SUBTYPE_FIELD = "OPP-070-notice"`. DE-1.x reaches it
   through the alias `DE1-NoticeSubType-SubTypeCode` → `SUBTYPE_FIELD`. So for eForms EU and
   eForms-DE the denominator is already available on a version-driven, tender_id-windowable table —
   no `notice_sections` probe at all, which would also make it CHEAPER than today's query.
3. **The legacy eras leave `notice_subtype` NULL.** Nothing aliases their document-type code to
   `SUBTYPE_FIELD` (grepped: no `TXT-TD`, no `TED-TD_DOCUMENT`, no form-root mapping). Their award-hood
   lives in the parse layer only:
   - **text**: `TXT-TD`, the inventory's "Type of document (code)" — `TD: 7 - Contract award` in the
     2005 CAN fixture.
   - **r208 / r209 / internal-ojs**: the FORM ROOT element. The vendored `ted-export-inventory.json`
     carries 97 form-shaped elements, roots running `F01_2014` … `F25_2014`; F03 and F06 are the
     contract-award forms, with more award-bearing roots among the concession/social/modification
     forms. The projection is already form-aware at fold time (issue 177's `VALUE_COST` routing) but
     does not persist the form.

## This suggests a better fix than the one proposed above

The section above proposes a per-era vocabulary spread across the report's SQL. That would put six
vocabularies inside a query — and the defect this issue exists to fix was *a hardcoded vocabulary in a
query claiming to span eras* (`kind = 'LotResult'`, which missed sdk-0.1's `TenderResult`). Repeating
that shape one layer up would be a poor trade.

**Project the era's document type into `tender_versions.notice_subtype` for the legacy eras too**, the
way DE-1.x already reaches it through an alias. Then:
- Section 3's denominator is ONE version-driven predicate over `notice_subtype`, with no era knowledge
  in the report at all.
- The per-era vocabulary lives in the projection's alias/mapping tables, where every other era
  difference already lives, and where the existing tests can gate it.
- `notice_subtype` stops being silently eForms-only — which is itself a latent trap for anything else
  that reads it (a "notices by type" facet would today show 11.9M legacy versions as NULL).

Cost: it needs a re-projection of the legacy eras to populate, which is the same rebuild window issues
100 and 232 are already queued behind — so it is close to free if bundled, and expensive if not.

**Still to be answered from the corpus** (needs bounded reads; not doable while a job holds the queue):
the actual distinct values of `notice_subtype` per era, and which `TXT-TD` codes and form roots really
appear. Both are `GROUP BY` over indexed columns and belong in one bounded probe before any mapping is
written — the sdk-0.1 CPV lesson from issue 231: check what the era publishes before mapping it.
