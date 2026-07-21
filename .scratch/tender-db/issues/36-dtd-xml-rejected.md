# 36 — DTD-bearing TED XML is rejected wholesale (~622k members)

Status: ready-for-agent

Split out of issue 30's quarantine triage (2026-07-21). The `unparsable-xml`
bucket is 628,204 members and — grouped by `detail` via `/v1/sql` on the prod
`quarantine` table — **621,863 are exactly `XML with DTD detected`**. That is a
deliberate rejection in the XML dispatch/parse path, not random malformed junk:
the parser refuses any notice XML carrying a DTD (`<!DOCTYPE …>`), so an entire
DTD-era of TED export XML is dropped whole.

This is suspected-real (a coverage gap), not benign: DTD-bearing files are how
a TED XML era shipped real notices. Not yet byte-confirmed to a specific notice
— that is the first step.

Investigate-then-fix:
1. Extract a few `XML with DTD detected` members from the archive (member paths
   are in the quarantine rows) and confirm they are real notices (TED_EXPORT /
   eForms roots behind the DTD), and which era/root they use.
2. Decide the safe handling — strip/skip the internal DTD subset before
   `roxmltree` (which rejects DTDs by policy), or route these through the
   appropriate profile — WITHOUT enabling XXE/entity-expansion risks (the DTD is
   being refused for a reason; handle it, do not blindly trust it).
3. Fixture + regression test; reprocess the held bucket after deploy
   (`reason='unparsable-xml' AND detail='XML with DTD detected'`).

Open question (same as issue 35): reconcile the ~622k against the 96.7% quoted
text coverage once the issue-15 backfill settles — the current numbers are a
mid-re-walk snapshot. The fix is warranted regardless if step 1 confirms real
notices.

Acceptance: DTD-bearing real notices parse (safely, no XXE); the bucket no
longer produced on those inputs; held entries reprocessed post-deploy.
