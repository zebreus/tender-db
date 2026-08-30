# 312 — Platform GUIDs in the identifier slot: measured, NOT a deny-floor class

Status: DIAGNOSED — the obvious fix is wrong; the narrow fix is specified below
Kind: data quality / identity semantics
Relates to: 311 (found by the review campaign), 300 (canonical keys), 234

## What the review found, and what the measurement said back

The issue-311 campaign read 306 of 823 identifier-bearing consortium rows
carrying a 32-hex **v4 UUID** (undashed, RFC-4122 nibbles) in the national-
identifier field — a submission platform's own record keys. The obvious
conclusion was "rule-shaped class, graduate it to `idgate::condemns`".

**Measured before building it (2026-08-30), and the conclusion reversed:**

- Corpus: **75,555** org rows carry a v4-GUID identifier (DE 45,974 /
  CH 25,301 / FR 3,091 / BE 325 / EE 316) across **75,548 DISTINCT** values.
  Seven rows share a GUID, in pairs. So the class produces **≈0 false
  merges** — and preventing false merges is the ONLY thing `condemns` does.
- Campaign sample (306 GUID orgs, local evidence, no prod scan): **93% span
  more than one notice**, mean 15.3 mentions, max 348; 4,678 mentions in the
  sample alone are held together BY that key.

So the platform GUID is not a placeholder — it is a **stable per-bidder
platform key doing real linking work**. Condemning it at the gate would
fragment ~75k orgs (every future mention minting its own provisional row) to
prevent essentially no bad merges. A prototype condemn was written and
REVERTED unbuilt on this measurement.

This is the deny-floor's own rule applied honestly (issue 234 lesson): a
class earns the floor by its measured false-merge rate, not by looking
untidy. "Not a real registration" and "not a useful identity key" are
different claims, and only the first one is true here.

## The tension this exposes in the 442 applied strips (issue 311)

The campaign's strips were individually reviewed and each removed a FALSE
CLAIM (`identifier_kind='national'` asserting a register entry that does not
exist). But for the GUID subset they also removed a WORKING LINK: those rows
keep their recorded mentions, yet future mentions carrying the same GUID
will no longer find the org and will mint provisional rows instead.

The right shape was never "strip or keep" but **reclassify**: keep the value,
stop it claiming register status.

## The narrow fix (specified, not built)

1. An `identifier_kind` value for platform keys (e.g. `platform-guid`):
   the value keeps linking mentions to their org, `canonical_key`/R2/R3 treat
   it as NON-register evidence (never a cross-country merge key, never
   checksum-anchored), and no consumer reads it as a national id.
2. Resolver + backfill: classify v4-GUID values into that kind at ingest
   (the `uuid_v4` predicate is trivial and was already test-drafted).
3. Then, and only then, revisit the 311 GUID strips: `org_case_reviews`
   holds every pre-image in `applied_action`, so a restore-as-platform-kind
   pass is mechanical — but it needs an unapply path that does not exist yet.
4. Re-measure the false-merge rate per platform after step 1; if a platform
   ever reuses one key across genuinely different bidders, THAT is the
   floor-worthy finding.

## Still open from the campaign (unchanged)

- Phone numbers / postal codes in the identifier slot: same measure-first
  discipline before any floor treatment.
- Register-format impossibility (FN/HRB/HRA on GbR-shaped names) stays a
  REVIEW calibration, not a rule.
