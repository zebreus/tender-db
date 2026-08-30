# 311 — Per-case AI review loop for rule-undetectable data errors

Status: ready-for-agent (directed by Lennart, 2026-08-29: "nearly all
errors can not be detected by simple rules... they need manual ai agent
review for each individual case and how to handle it")
Kind: capability (data quality / organization layer)
Relates to: 300 (the edge store this consumes — Stage 4), 310, 230 (DQ)

## The principle (Lennart's steer)

Deterministic rules are the DENY-DIRECTION FLOOR, not the detector:
checksums catch arithmetic errors, the consortium lexicon keeps KNOWN
shapes out of auto-merge — but most wrong-data classes (unlabelled
consortiums like "X GmbH / Y GmbH", garbled names, mis-filed rows,
semantic errors) have no rule-shaped signature. Those get INDIVIDUAL
AI-agent review with full evidence, one case at a time, each producing a
recorded verdict AND a handling decision.

## Architecture

1. **Case queue** — the Stage-4 `org_candidate_edges` store plus the
   standing suspect classes (denied merge families from r2/r3 reports,
   consortium suspects, uncorroborated anchors, the Bietergemeinschaft
   cohort). No new queue table needed to start; cases are enumerable from
   what exists.
2. **Review** — agent fan-outs (Workflow), ONE agent per case, given the
   complete evidence: org row(s), satellite names, mentions with raw
   identifiers, the notices involved, tender context. Verdict schema:
   merge / split / consortium-vehicle (poison + edge) / leave-standing /
   escalate, plus rationale. Adversarial spot-checks on a sample of
   verdicts before any batch is applied (the campaign's standing bar).
3. **Record** — `org_case_reviews` table (case ref, verdict, rationale,
   evidence snapshot, reviewed_at, applied_at, job_id) so every decision
   is auditable and re-checkable.
4. **Apply** — verdicts are executed ONLY through the verified machinery:
   merges via the merge arms (which re-check their own denial stacks),
   splits via dissolve, consortium verdicts via key poisoning + edges.
   The reviewer never writes entity tables directly.

## First cohort

The Bietergemeinschaft class (measured 2026-08-29: 620 BIEGE-prefixed +
8,658 spelled-out rows, 822 identifier-bearing). The lexicon floor
(landed same day) keeps them out of auto-merge; the review decides each
case's actual handling (vehicle vs member attribution, whether the
grouping deserves standing identity, member extraction).

## PILOT COMPLETE (2026-08-29 ~20:4x, 50 cases, 60 agents ~2.8M tokens)

Verdicts (full record: `311-pilot-verdicts.json`): **23
consortium-vehicle-wrong-identifier** (22 lead-member's number, 2
concatenated member VATs — "DE232901575DE231871970" ingested as one id —
1 street address as identifier), **20 consortium-vehicle-sound**, **4
member-row-mislabelled** (single company with a "Bietergemeinschaft
mit…" annotation), **3 unclear-escalate**. Confidence 17 high / 33
medium. **Audit: 10/10 independent skeptic re-derivations AGREED — zero
disagreements**, the calibration signal the pilot existed for.

Findings the rules could never have made:
- **The fusion shape**: a vehicle row carrying the lead member's number
  ALSO captures the member's SOLO mentions via the shared literal (R&K
  Ingenieure: 7 of 8 mentions are the member alone; Dobler: 7 solo
  mentions). The wrong identifier doesn't just mislabel the vehicle —
  it fuses two entities' records. Recommended handlings are concrete:
  strip the identifier to NULL, re-home the solo mentions to a member
  row.
- **The 9110 GLN class**: Austrian notices widely publish 13-digit GS1
  Austria GLNs (ERsB/USP-issued, 9110-prefix, mod-10-checkable) as
  organization identifiers — a real register class idgate/crosswalk do
  not model. Several reviewers verified check digits by hand. Worth a
  census + possible idgate scheme (separate slice).
- German VAT shape violations diagnose mechanically (DE + exactly 9
  digits): "D1633830016"-style manglings are detectable, whose number
  they are is not.

## APPLY STAGE LIVE, FIRST WET RUN DONE (2026-08-29 ~22:0x, rev 679369c)

Built + panel-hardened + deployed in one evening: `org_case_reviews`
(verdicts via POST /admin/case-reviews, one standing verdict per
org+cohort, re-record NEVER touches an applied stamp or its pre-image —
panel catch), `apply-case-reviews` job (dry default; dry records the
CONCRETE strip list as a case-apply-plan report — panel catch: counts
alone can't surface a hallucinated org id). First cohort executed:
50 verdicts recorded, dry plan matched the expected 14 strips EXACTLY
(zero strangers), wet stripped all 14 wrong identifiers (11 lead-member
numbers incl. the fused R&K/Dobler rows, 2 concatenated member-VAT
pairs, 1 address) with pre-images in applied_action and raw values
untouched in mentions. Journal clean.

## BATCH CAMPAIGN COMPLETE (2026-08-30 ~04:0x-06:3x, batch shape v2)

**All 823 remaining identifier-bearing Bietergemeinschaft cases reviewed
individually; 442 wrong identifiers stripped (job 481, plan-exact).**
Full verdicts: `311-batch-verdicts.json`; cohort
`biege-batch-2026-08-30` in org_case_reviews with pre-images.

Execution: one bulk enrichment pass (823 cases, 6,683 mentions,
collision counts), stratified by identifier shape (overlong 335 /
national-other 281 / vat-shaped 176 / gln9110 26 / concat-vat 5), 32
shared-preamble batches over two sonnet waves + two 4-agent Fable
skeptic audits (10%/stratum) + one 49-case correction round + one
owner-reviewed straggler. ~4.4M tokens total ≈ 5.3k/case — a 10×
improvement on the pilot's 55k/case, per-case bar intact.

Audit gates: wave 1 32/38, all 6 disagreements ONE class (v4 platform
GUIDs inconsistently spared) → the whole 48-case lenient slice
re-reviewed under the audit-confirmed calibration; wave 2 40/45, all 5
disagreements medium-band (no apply effect), adjudicated per the
register-impossibility / evidence-absence calibrations. **Across both
audits: ZERO downgrades of auto-applied highs — no false strip found
in 83 audited cases.**

Final verdict census (823): 442 wrong-identifier/high (APPLIED),
102 wrong-identifier/medium + 16 low (parked), 170 sound (mostly
medium), 58 member-row-mislabelled, 17 unclear-escalate.
Headline classes: 306 v4-GUID platform keys (→ issue 312: graduate to
the deny floor), lead-member VATs/FNs, concatenated member VATs,
phone numbers and postal codes in the id slot, e.V./eGbR standing
bodies correctly left sound.

Acceptance: strips verified on prod (identifier+kind NULL, names
standing, raw mention values untouched), journal clean, dry plan ==
wet result == my verdict ledger exactly (442).

PARTIALLY REVERSED (2026-08-30, issue 312): 280 of the 442 strips held a
platform's v4-GUID record key. The follow-up measurement showed that class
LINKS rather than false-merges (75,548 distinct values over 75,555 rows; 93%
of sampled GUID orgs span several notices), so those strips were restored
from their pre-images via the new `unapply-case-reviews` path — panel-clean,
plan-exact, journal clean. The other 162 strips (lead-member VATs, fused
member VATs, phone numbers, addresses) stand: those values were actively
wrong. The per-case reviews themselves were not wrong about the SEMANTICS
("this is not a register entry"); the apply ACTION was too blunt for one
class, and the fix is a platform-guid identifier kind (issue 312 steps 1-2).

REMAINING: (1) solo-mention re-homing to member rows (the fusion
repair — dissolve-adjacent machinery, own panel round; the campaign's
handling notes name every re-homing candidate); (2) the escalations
queue (17 unclear-escalate) + medium-confidence policy (102 parked
wrong-id/medium — candidates for evidence enrichment, e.g. register
lookups); (3) widening the case sources beyond the org layer if
Lennart wants (offered); (4) issue 312 (GUID deny-floor) and the
wrong-data docs consolidation.

## Notes

- Session-driven review batches first (the hourly owner sessions run
  them); productize an in-app review pipeline only if volume demands.
- Sizing: start with a 50-case pilot batch to calibrate cost/quality and
  the verdict schema before running cohorts.
- The wrong-data policy consolidation (one docs/ page: floor rules +
  corrigenda flow + review loop + tripwires) folds into this issue's
  completion.

## Batch shape v2 (designed 2026-08-30, for the ~772-case remainder)

The pilot's ~55k tokens/case was dominated by per-case context setup and
per-case enrichment round trips, not by the judgment itself. Four levers,
none of which weakens the per-case bar (every case still gets an
INDIVIDUAL agent verdict — Lennart's direction; deterministic features
group cases, they never decide them):

1. **Stratified shared-context batches**: one reviewer agent takes a
   STRATUM of 20-30 cases sharing a shape (the pilot's classes:
   lead-member number, concatenated member VATs, address-as-identifier,
   9110-GLN, other). The preamble — task, verdict schema, veto rules, the
   pilot's gold exemplars — amortizes over the batch. Est. 55k → 12-18k
   tokens/case.
2. **Bulk enrichment staging**: one bounded SQL pass stages all cases'
   evidence (identifier, head+satellite names, mention raws, co-party
   rows, tender refs) into a single JSON before any agent runs — the
   r3-sample-enriched.json pattern — instead of per-case fetches.
3. **Cheap-model first pass, big-model audit**: reviews run on the small
   model with the calibrated prompt; the big model audits a 10% sample
   per stratum against the pilot's 10/10 agreement bar. A stratum whose
   sample disagrees re-runs whole on the big model. Verdict confidence
   maps unchanged (only 'high' is apply-eligible).
4. **Apply cadence unchanged**: verdicts land per stratum via
   POST /admin/case-reviews; dry apply-case-reviews → plan review → wet.
   The safe subset stays verdict='consortium-vehicle-wrong-identifier' +
   confidence='high'.

Projected cost: ~25-30 stratum batches ≈ 3-5M tokens + audit, vs ~42M at
the pilot rate. Execute after Stage 4 Unit 5 lands (the review loop then
also consumes candidate edges, so one enrichment pass serves both).
