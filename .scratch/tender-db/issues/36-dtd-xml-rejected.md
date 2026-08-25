# 36 — DTD-bearing TED XML is rejected wholesale (~620k members, ~27k real loss)

Status: resolved-as-diagnosed (interim landed; the parser is issue 41)

## Resolution (2026-07-21)

Investigated with byte-exact prod payloads + `/v1/sql`. The original framing
below (a duplicate era to skip) was **overturned**; the finding:

- The 621,863 `XML with DTD detected` members are an `opoce-input/` per-language
  subtree the first sampling missed — member paths
  `20080502_2008085.tar.gz/<num>/opoce-input/<num>_2008.<lg>`, ~22 languages, so
  ≈**28k distinct notices**, all outstanding, first_seen during today's backfill
  (NOT historical).
- Each is `<!DOCTYPE INTERNAL_OJS PUBLIC "…INTERNAL_OJS XML R2.0.5//EN"
  "Internal_Ojs.dtd" [<!ENTITY % TYPE '…'>]>` + `<INTERNAL_OJS>` — a **distinct
  vocabulary** (not TED_EXPORT, text, or eForms), mainstream S-series headings
  (3310/3340/3540/45xx…), real `NO_DOC_OJS 2008/S 85-114238` refs.
- **Completeness check (mandatory, both directions): they are NOT duplicates.**
  0 of 350 opoce notices sampled across two package regions have a `ted` text
  twin in `notices` (format validated: the text notice `723-2008` does exist).
  ≈28k opoce-only notices ≈ the entire ~27k 2008 shortfall — **these notices
  ARE the 2008 gap**. (Lead's "duplicate delivery ÷22 languages" hypothesis is
  overturned by this check.)

Therefore neither proposed fix applies: a policy-skip would **drop 28k real
notices**; "route to r208" is wrong (different vocabulary). Closing the gap needs
a full **INTERNAL_OJS R2.0.5 parser** — filed as **issue 41**.

**Interim landed here (issue 36's closure):** an XXE-safe DOCTYPE strip
(`profile::strip_doctype` — never processes a DTD; a hostile internal general
entity referenced in the body is left undefined and refused, tested both ways),
and `INTERNAL_OJS` roots now route to an honest `unmapped-era` quarantine
(profile `internal-ojs`, detail points at issue 41, classes as SuspectedGap) —
instead of the misleading `unparsable-xml: XML with DTD detected`. Ledger entry
added. The ~28k notices are reclaimed by issue 41, not this.

---

## Original framing (overturned — kept for the record)

Status: ready-for-agent (historical, superseded by the header Status — kept only so board greps don't misread this file as open)

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
