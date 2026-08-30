# 314 — Stage 4's 1.5M candidate edges have no consumer

Status: ready-for-agent (the stage's stated purpose, unconnected)
Kind: capability (organization layer)
Relates to: 300 (Stage 4 built it), 311 (was meant to consume it), 312

## The gap

`org_candidate_edges` holds **1,498,485** E3 edges, refreshed by the weekly
wet scan. Nothing reads them. Verified 2026-08-30: the only code touching
the table outside its own writer is a doc comment.

Stage 4 was justified by feeding the issue-311 per-case review loop — the
design's own words, and the reason the store is advisory-only (edges never
merge). Then the 311 campaign ran off the **Bietergemeinschaft cohort**
instead, because that cohort was already enumerable and the edges were not
yet built. The campaign succeeded (823 cases reviewed, 442 strips, 280 of
them since restored per 312) and the edge store quietly became a thing that
is maintained but not used.

## What consuming them means

1. **Cohort enumeration from edges.** A case = an edge (or a connected
   component of edges) whose two orgs are candidates for the same entity.
   The 311 machinery takes verdicts keyed by org id, so an edge-derived
   cohort needs a component→case mapping and a cohort name.
2. **Evidence assembly per case**: both orgs' rows, satellite names,
   mentions, collision counts, PLUS the edge's own `evidence` JSON (which
   already names the reaching key, the group size and each side's witness).
   The batch-shape-v2 enrichment pass generalizes: it is the same bulk SQL
   with a different seed set.
3. **Verdict vocabulary is different from the consortium cohort's.** These
   are same-name/cross-language pairs, so the verdicts are merge /
   distinct-entities / needs-more-evidence — NOT the consortium-vehicle
   classes. New rubric, new gold exemplars, its own pilot before any batch.
4. **Nothing auto-applies at first.** The 311 apply job's safe subset is an
   identifier strip; a merge verdict must execute through the merge arms
   (which re-check their own denial stacks), and that path does not exist
   for review-driven merges. Pilot verdicts land as records only.

## Sizing before building (measure first, per 312's lesson)

1,498,485 edges is far too many to review case by case at ~5.3k tokens per
case. Before any campaign, measure the shape: component-size distribution,
how many components are canonical×canonical (the only ones a merge verdict
could act on), how many are already merged-away duplicates, and how the
e3-xlang slice (57,808) differs from e3-name. A tractable first cohort is
probably "xlang pairs where both sides are canonical and the countries
differ" — small, high-value, and exactly the contamination class the
exemplar (org 23294544) represents.

## Do not do

Do not wire an automatic merge off these edges. They are E3 name-equality
only; §1's tier table puts name evidence in candidate generation, never in
the merge decision. The consumer is review, and the reviewer is the gate.
