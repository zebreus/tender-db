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

REMAINING: (1) solo-mention re-homing to member rows (the fusion
repair — dissolve-adjacent machinery, own panel round); (2) batch the
remaining ~772 identifier-bearing Bietergemeinschaft cases through
review (cost calibrated: ~55k tokens/case incl. audit share); (3) the
escalations queue + medium-confidence re-review policy; (4) widening
the case sources beyond the org layer if Lennart wants (offered).

## Notes

- Session-driven review batches first (the hourly owner sessions run
  them); productize an in-app review pipeline only if volume demands.
- Sizing: start with a 50-case pilot batch to calibrate cost/quality and
  the verdict schema before running cohorts.
- The wrong-data policy consolidation (one docs/ page: floor rules +
  corrigenda flow + review loop + tripwires) folds into this issue's
  completion.
