# 58 — Incremental projection: re-project only the Tenders touched since last run

Status: ready-for-agent
Severity: MEDIUM (daily-wall-clock optimization, not an outage)
Blocked by: 57 must be validated in prod first — HOLD implementation until
team-lead gives the go. The bounded streaming fix (issue 57, commit e4ffb8b /
deployed 20767e8) ended the OOM emergency; this is the follow-up that makes the
*daily* projection fast. Reproduce-first before implementing.

## Problem

`project()` reads and re-folds the WHOLE corpus on EVERY run, not just once.
`crates/ingest/src/project.rs` Phase 1 loops `parsed_chunk(after_id, …)` from
`after_id = 0` unconditionally (project.rs:294); the `rebuild` flag only controls
whether `clear_canonical()` runs first (project.rs:278) — it does NOT scope the
read. The daily pipeline pushes `Spec::Project { rebuild: false }` after every
process job (supervisor.rs), so every daily projection re-reads + re-folds all
7.5M+ notices.

Issue 57 bounded the *memory* of that whole-corpus pass (peak = one batch, not
the corpus), but not its *time*: a daily run still does O(corpus) work — reads
every notice, resolves every mention, folds every Tender — to absorb a few
thousand new notices. It completes now (no OOM), just slowly, and the cost grows
with the corpus forever.

## Goal

Daily projection cost scales with **notices changed since the last projection**,
not with the corpus size. Re-project only the Tenders those notices touch,
re-deriving each touched Tender IN FULL. The `rebuild: true` full re-projection
(needed to build the canonical layer initially and after schema changes) keeps
the issue-57 bounded-streaming path and must always fit in RAM.

## Correctness trap (load-bearing — the whole reason this is subtle)

A new notice can attach to an **old** Tender: a correction / award / result
notice references an earlier contract notice, and grouping assembles a Tender
from all its notices across time (ADR-0001, procedure_key). So "incremental"
must NOT mean "project the new notices in isolation." It means:

1. Find the grouping keys **touched** by the new notices.
2. Re-project each touched Tender **in full**, loading its pre-existing notices
   too (so supersession/version chains and results accumulation stay correct).

Computing the touched-set is not just "the new notices' own keys":

- **Keyed (BT-04) / island:** the touched key is the notice's own
  procedure_key (or island id). A new notice under an existing BT-04 re-projects
  that whole Tender. A new BT-04 that a previously-island notice now shares
  upgrades the island into the keyed Tender (island upgrade — CONTEXT.md), so
  the previously-island notice's Tender is also touched.
- **Legacy OJS chains (the subtle one):** these group by transitive union-find
  over OJS edges. A new legacy notice can **bridge two previously-separate
  components** into one (the ADR-0003-style late-edge merge — see
  `project_equivalence`'s bridge case, and `a_late_edge_merges_two_legacy_tenders`
  in tests/project.rs). So the touched-set for a legacy notice is the union-find
  **component(s) its edges transitively reach**, not just its own OJS number —
  and the merge retires the absorbed key with `removed` change events
  (`retire_absorbed_legacy_tenders`). The touched-set computation must therefore
  see the existing edge graph, not only the new notices.

If any of that is gotten wrong the symptom is a split or stale Tender that a
full rebuild would have merged/updated — silent, not a crash.

## Design direction

The issue-57 streaming code is already structured for this:

- **Phase 1** already separates cheap grouping identity (`Ident`) from the heavy
  fold. Incremental scopes Phase 1's notice set to "changed since the last
  projection watermark," then expands to the touched grouping keys (with the
  legacy transitive-reach expansion above), then loads the *full* notice-id set
  of each touched Tender.
- **Phase 2** is unchanged: it already folds + applies whole Tenders a bounded
  batch at a time. Feed it only the touched Tenders' plans.

Open design points to settle:
- **Watermark.** project is currently a stateless full scan. Incremental needs a
  "last projected" marker — a projection watermark (max notice id projected, or a
  cursor). Decide where it lives and how `rebuild` resets it. Interaction with
  the change-feed cursor (the canonical version sequence) must stay coherent
  (see issue 46, rebuild change-feed coherence).
- **Mention resolution.** Mentions of unchanged notices are already resolved and
  recorded; incremental must resolve only the new notices' mentions but still bind
  touched Tenders' pre-existing mentions from the DB (mentions_by_ids already does
  this per batch).
- **Touched-key discovery query.** Needs an index-friendly way to go from "new
  notice ids" → their procedure_keys / ojs edges → touched tenders, and for legacy
  the transitive component. Likely a bounded graph walk over the existing ojs edge
  set for the touched components only.

## Reproduce first

Before implementing, build a timing curve (not a memory curve this time): a large
existing corpus + a small batch of new notices, and show the current whole-corpus
projection's wall-clock scales with the corpus while an incremental run scales
with notices-changed. This is the analogue of `project_memory` for the time axis.

## Acceptance

- Daily projection wall-clock scales with notices-changed, not corpus size
  (timing curve).
- Output identity: an incremental projection of `new notices` over an existing
  canonical layer produces the SAME canonical state as a full `rebuild` over the
  whole corpus — proven on a fixture that includes (a) a late notice attaching to
  an old keyed Tender across time, (b) a legacy bridge notice merging two existing
  components, (c) an island-upgrade where a later notice supplies a shared BT-04.
- `rebuild: true` still works within RAM via the issue-57 bounded streaming path.
- Change-feed / cursor coherence preserved (issue 46).

## Comments

2026-07-24 (proj-fix): Captured while fresh, right after the issue-57 bounded fix
landed (e4ffb8b, deployed 20767e8). The legacy transitive-reach point is the easy
thing to get wrong — the touched-set for a legacy notice is the union-find
component(s) its edges reach, not its own key. Implementation on HOLD per
team-lead until 57 is validated in prod.
