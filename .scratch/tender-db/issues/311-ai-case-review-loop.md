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

## Notes

- Session-driven review batches first (the hourly owner sessions run
  them); productize an in-app review pipeline only if volume demands.
- Sizing: start with a 50-case pilot batch to calibrate cost/quality and
  the verdict schema before running cohorts.
- The wrong-data policy consolidation (one docs/ page: floor rules +
  corrigenda flow + review loop + tripwires) folds into this issue's
  completion.
