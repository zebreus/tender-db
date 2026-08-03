# 104 — a DE-1.x golden fixture: close the epoch-discipline hole and the fold-order gap at once

Status: open — REQUIRED follow-up (team-lead). Post-landing; sdk-vendor to implement on their fixture
infra, epoch context from issue 99.
Kind: test coverage / process guard
Blocked by: —
Relates to: 99 (the discipline this guards), 98 (the change it would have caught), 94 (the allowlist
gate that covers the other half), 85

## Two holes, one fixture

**1. The epoch bump is human discipline, and its only guard has a gap.**

Issue 99's `PROJECTION_EPOCH` must be bumped by hand on any projection-logic change. The mitigation on
record is that `project_golden` pins fold output, so a logic change turns it red and forces a
deliberate regeneration — putting the bump on the road the change already travels.

That guard does not cover the case that actually happened. **`project_golden`'s corpus contains no
eForms-DE 1.x notice** (`grep -c "eforms-de-1" project_golden.rs` → 0), and `normalise_de1` is
profile-gated, so a DE-only change — issue 98 exactly — **cannot** turn golden red. The coupling would
not have prompted the very bump it exists to prompt.

The same is true of `project_equivalence` (its corpus is `eforms:eforms-sdk-1.13` and `ted-export-r209`
only). So "golden and equivalence stayed green" proves DE-path changes are safe only in the sense that
they are *invisible* to those suites.

**2. The DE path has no version-count or fold-order coverage.**

Multi-notice DE *grouping* is covered in both directions (`a_uuid_de1_folder_id_still_merges_the_procedure`,
`a_non_uuid_de1_folder_id_does_not_merge_notices`). Neither asserts version **count** or **order**, so
the `published_at` lever — which orders versions within a Tender via `plan_notice_fold` — has no
behavioural test. Issue 94's alias allowlist gate *prevents* an alias from reaching that lever, which is
stronger than detection, but it does not cover a fold-order change arriving by any other route.

## The fixture

A DE-1.x notice **and its TED twin** sharing a BT-04 / folder uuid, in the golden corpus, with distinct
`published_at` so their fold order is meaningful. That single addition:

- makes any DE-1.x projection-logic change turn `project_golden` red, so the epoch bump is prompted on
  the path the change already travels;
- gives the DE path multi-notice **version-count and fold-order** coverage;
- exercises the cross-source merge that is the cohort's dominant real shape (216,450 of the 218,635
  merged onto TED twins), which no current fixture does.

## Acceptance

- `project_golden` fails if a DE-1.x mapping changes without the golden being regenerated.
- The fixture asserts one Tender, two versions, ordered by `published_at`, with the DE notice's facts
  and the TED twin's both present.
- Regenerating the golden surfaces `PROJECTION_EPOCH` (include it in the fixture) so the bump is not
  silently skipped.

## Note

Not blocking the issue-98/99 re-fold: the production `EXPECT_NO_REGROUPING` backstop is the full-scale,
real-chain check for that run. This is load-bearing for the **next** profile-specific change, when
nobody is watching the fold as closely.
