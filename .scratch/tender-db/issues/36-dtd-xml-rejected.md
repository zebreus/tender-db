# 36 — DTD-bearing TED XML is rejected wholesale (~620k members, ~27k real loss)

Status: ready-for-agent

Split out of issue 30's quarantine triage (2026-07-21). The `unparsable-xml`
bucket is 628,204 members and — grouped by `detail` via `/v1/sql` — **621,863
are exactly `XML with DTD detected`**. That is a deliberate rejection in the XML
dispatch/parse path (`roxmltree` refuses DTDs), so any TED export XML carrying a
`<!DOCTYPE …>` is dropped whole.

**Concentrated in one era and mostly duplicates** (measured 2026-07-21, `/v1/sql`
quarantine⋈fetches on `fetch_id`): the DTD bucket is **619,965 in 2008** + 1,898
in 2010. Bounding it the same way as issue 35 — 2008 held-vs-ground-truth:
- held (notices⋈fetches, period `2008%`) = **312,567**
- TED ground truth 2008 = **339,534**
- ⇒ **92.1 %** coverage, shortfall **≈27k**.

620k DTD members against 339k published notices ⇒ multiple members per notice;
at 92% held they are overwhelmingly **duplicate representations** already held
via a non-DTD member. Real loss is bounded ≈27k in 2008, not 620k.

Still goal-critical: 92.1% **fails the verify ±2% tolerance**. Handling
DTD-bearing XML reclaims the missing ~27k (dedup absorbs the duplicate
majority).

Investigate-then-fix:
1. Extract a few `XML with DTD detected` 2008 members from the archive and
   confirm the root behind the DTD (TED_EXPORT R2.0.x?) and why 2008 shipped
   DTDs.
2. Strip/skip the internal DTD subset before `roxmltree`, or route through the
   right profile — **without** enabling XXE/entity-expansion (the DTD is refused
   for a reason; handle it, do not blindly trust it).
3. Fixture + regression test; reprocess the held bucket after deploy
   (`reason='unparsable-xml' AND detail='XML with DTD detected'`).

Acceptance: DTD-bearing real notices parse safely (no XXE); the bucket no longer
produced on those inputs; held entries reprocessed; 2008 coverage rises toward
the ±2% tolerance.
