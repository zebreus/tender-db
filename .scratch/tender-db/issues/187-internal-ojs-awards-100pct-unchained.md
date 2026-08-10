# 187 — internal-ojs awards are 100% unchained (9,701/9,701): the issue-27 rule fires

Status: needs-triage
Kind: reference-resolution defect (pre-registered trigger)
Blocked by: —
Relates to: 27 (the rule: ">90% unchained at full data ⇒ reference-resolution defect, file it"), 41 (the profile — its fixtures include a REF_NOTICE chain edge), 04 (canonical projection / chaining)

## Why

Issue 27 pre-registered a watch rule for the award-linkage panel when it read 96–99% unchained on
a pre-backfill canonical layer: re-check at full data, and above 90% file a reference-resolution
defect. The backfill is done, and the panel reads (public `/api/dashboard`, 2026-08-10):

    internal-ojs   awards=9,701   unchained=9,701   ratio=1.000

Every single 2008 OPOCE award is a lone-notice Tender. The issue-41 profile demonstrably parses
REF_NOTICE chain edges (its golden fixture asserts one), so the break is downstream or
cross-boundary. Two candidate causes, both checkable:

1. **Cross-profile chaining**: internal-ojs covers exactly one year (2008). A 2008 award's
   REF_NOTICE points at a contract notice held under the `text` profile (2007 and earlier) — if
   the chaining key embeds the profile/era, the link can never resolve by construction.
2. **The edge never reaches the fold**: parsed but not projected into the grouping key
   (the issue-99 class — projection-logic change without a refold, or the fold predates the
   profile's chain edges and no refold touched the era).

## What

Attribute first (one fixture-level trace of a known REF_NOTICE pair through grouping), then fix,
then the era needs a refold for links to materialize — note issue 179's cost finding: a legacy
refold currently pays the full-corpus price, so batch this with other pending era work if
possible. Success: ratio drops from 1.000 to the era's honest residual, and the number is quoted
next to r209's research-predicted ~17% baseline.
