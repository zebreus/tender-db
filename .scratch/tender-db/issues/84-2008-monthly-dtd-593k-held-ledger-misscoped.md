# 84 — ~593K held 2008 monthly-TED DTD notices; ledger reports the category "resolved" (mis-scoped to the ~28K opoce subset)

Status: open — DISCOVERED 2026-08-01 (proj-fix, from immutable snapshot 531). Largest single remaining reclaim opportunity (~4.8% of corpus). Needs (1) format investigation + reclaim path, (2) ledger scope correction.
Kind: completeness / data-quality + ledger correctness
Blocked by: —
Relates to: 36 (XXE-safe DTD strip), 41 (internal-ojs parser, the opoce subset), 73 (unparsable-xml sizing — this REVISES it), ADR-0004, ADR-0009, [[resolved-categories-ledger]]

## Finding (snapshot 531, 2026-08-01)

Post-recovery, `unparsable-xml` still holds **601,256** notices. By `detail`:

| detail | held |
|---|---|
| `XML with DTD detected` | **594,915** |
| `unknown token at 1:1` | 4,441 |
| `unknown token at 1:3` | 1,900 |

Joined to `fetches`, the DTD rows are **ted/monthly/2008 = 593,017 held** vs **26,948 reclaimed**
(same source/kind/year), plus 1,898 from 2010. So the 2008 DTD bucket is really **~620K**, not the
~28K that issue 41 scoped.

## Why it's still held (NOT a regression)

Issue 36 makes DTD-bearing XML parseable (strip the DOCTYPE, resolve no external entities). Issue 41's
`internal-ojs` parser then reclaimed the **opoce-only S-series subset** — "the whole 2008 coverage gap"
as it was understood — which is exactly the **26,948** now reclaimed. The other **~593K** 2008 DTD notices
are a different vintage that was never in the internal-ojs parser's scope and remain quarantined. My earlier
"~116K genuine unparseable remainder" estimate was wrong: the genuinely-unparseable-looking residual is only
**6,341** (the `unknown token` rows). Everything else here is a tractable, large reclaim target.

## Two actions

1. **Investigate + reclaim (the big lever):** determine what the ~593K 2008 (+~1.9K 2010) DTD notices ARE —
   likely the older standard TED_EXPORT XML (R2.0.x) that legitimately declares a DTD, distinct from the
   opoce INTERNAL_OJS backbone. Confirm whether, after the issue-36 DTD strip, they parse under an existing
   TED-XML profile or need a mapping profile. If reclaimable, run the reprocess (ADR-0009,
   `reason=unparsable-xml detail_like='XML with DTD detected'` scoped to the non-opoce set) + fold. This is
   the single largest remaining reclaim (~4.8% of the corpus), bigger than everything left in the SDK/DE buckets.

2. **Correct the ledger scope (correctness):** the dashboard "Resolved categories" entry
   "2008 OPOCE INTERNAL_OJS (DTD)" (issue 36) presents the 2008-DTD category as resolved while 593K notices
   of that exact `XML with DTD detected` signature are still held. `coverage.rs` does show its live
   outstanding count, but the "resolved" framing is misleading given the bucket's true size. Re-scope the
   ledger entry to the opoce subset it actually covers, and track the broader 2008-DTD reclaim as this open
   issue — so the user-facing record matches reality (a core project goal).

## Note

This corrects issue 73's sizing (which sized unparsable-xml at 628K and assumed issues 36+41 covered the
recoverable share). The recoverable share is far larger than the 28K reclaimed.
