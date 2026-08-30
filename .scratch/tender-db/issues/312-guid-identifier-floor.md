# 312 — Platform GUIDs in the identifier slot: measured, NOT a deny-floor class

Status: RESOLVED 2026-08-30 — step 0 restored the strips, step 1a shipped
(GUIDs excluded from the merge keyspace), steps 1b/2 DECLINED on measurementnt
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

## STEP 0 DONE (2026-08-30 09:5x): the GUID strips are restored

`unapply-case-reviews` built, gated (78 suites), panel-reviewed and deployed
(9524f64). Adversarial panel: **0 confirmed / 8 rejected** — every candidate
finding was verified as a non-defect, including the sharpest one (the single
shared-GUID pair among the 442: restoring both returns the exact status quo
ante, and the duplicate is inert because `canonical_key` returns None for
DE/AT, so R2 never groups it).

Prod: job 487 DRY — 456 applied verdicts examined, 280 with a platform-GUID
pre-image, 280 restorable, 0 no-ops; plan hand-verified (every planned value
a v4 GUID, every org a campaign case, no non-GUID strip touched). Job 488 WET
— **280 identifiers restored**, plan-exact, journal clean.

Verified after: org 22149631 carries its GUID + kind again, while org
21862089 (a lead member's Swiss UID) and 22634951 (two fused German member
VATs) remain stripped — the split the measurement called for. The 162
non-GUID strips of the 311 campaign stand untouched.

The machinery is general: `unapply_case_reviews(select, …)` takes the
predicate over PRE-IMAGE VALUES, so any future misjudged apply class is
reversible the same way, dry-first, with the same two guards (never clobber
a newer value; `applied_at` stays set so nothing falls back into the pending
set and gets re-applied forever).

## STEP 1a SHIPPED, STEPS 1b/2 DECLINED (2026-08-30, rev 65c1854)

**Shipped.** `crosswalk::canonical_key` returns `None` for a v4 UUID under
any country or kind. The GUID keeps linking (the resolver binds on the raw
`(country, kind, value)` triple — canonical.rs:5215 — which this does not
touch) and stops being eligible as a merge key.

**Correction, made by the post-deploy census (this matters more than the
change).** I justified this guard with "roughly 8 rows corpus-wide were
riding the FR:siret arm into an E1 key", extrapolated from 1 FR specimen in
400 carrying exactly 14 digit characters. The census after the deploy reads
`keyed_e1` = 366,766 — **identical** to before it. The true number was
ZERO, and the reason is three lines below the guard: every national arm
gates on `digits_only`, and a 32-char hex UUID always carries letters. The
digit COUNT was necessary and nowhere near sufficient; I checked the half
that confirmed my expectation and not the half that would have refuted it.

Distribution by keyable country, for the record: FR 3,092, BE 325, IT 74,
CZ 23, PL 21, FI 9, SE/PT/NO/HR 5 each, GR 2 (DE 46,240 never keys).

The guard stays, on the honest justification rather than the flattering
one: today "no platform key ever merges" holds only as an emergent
consequence of every arm requiring all-digit bodies. One alphanumeric
register scheme — HRB/FN shapes exist — and that property dies silently.
The line states the invariant so it survives that change.

**Declined: the `platform-guid` identifier kind and the 75k-row
reclassification** (this issue's original steps 1-2). The measurement
removes the operational case for it:

- The merge exposure it was meant to fix is ~8 rows, and step 1a closed it
  outright.
- What remains is truthfulness of an API field — real, but not worth the
  risk it carries: the resolver's binding key INCLUDES `kind`, so changing
  stored rows to a new kind splits them from incoming mentions until a
  backfill completes, and the reverse order fragments just as badly. That is
  a self-inflicted fragmentation window across 75,555 orgs to correct a
  label.
- Deciding this way is the same discipline that saved the first pass: a
  class earns a change by its measured effect, not by looking untidy.

**What would reopen it**: a consumer that must distinguish register
identifiers from platform keys (none today — the field is advisory in the
API and no internal path reads `kind` except the binding triple and
`canonical_key`, which now ignores GUIDs anyway); or a platform that starts
REUSING one key across genuinely different bidders, which would make the
class a false-merge source and thus floor-worthy after all. The weekly
r2-census is where that would first show.
