# 448 — FTS `additionalIdentifiers` are parsed and then dropped: the Companies House ↔ PPON pairing never reaches the matcher, and 161 suppliers stand as two organizations

Status: ready-for-agent — UNIT 1 BUILT 2026-09-30 (the dry-only planner: `match-org-identifiers` rule `altid`, report `altid-merge-plan`; wet refused until unit 2). Gated green, 20 store tests + 4 crosswalk + 2 supervisor. Next: deploy when the queue is idle, run `{"kind":"match-org-identifiers","rule":"altid"}`, and read the plan against the 161. Design: `.scratch/tender-db/448-altid-design.md`. Filed 2026-09-30 00:0x UTC, as the follow-up `342-fts-plan.md` §5 risk 3 promised and never filed.

## What is wrong

An FTS party publishes its primary `identifier` and, since the Procurement Act, `additionalIdentifiers`. The dominant
shape is a Companies House number with the supplier's PPON beside it. The parser keeps all of them:
`fts/checklist.rs` maps `parties[].additionalIdentifiers` to further `BT-501-Organization-Company` rows at ordinal ≥ 1.
The fold then reads only the first. `project.rs` mention capture takes an id only while
`mention.raw_identifier.is_none()`. So the one statement that ties a supplier's company number to its PPON is dropped.
Wherever the same supplier is published PPON-first (as the primary identifier), it mints or keys a second organization.

Measured on prod 2026-09-30 00:0x UTC, bounded reads over the 14,647 FTS notices (2025-06 and 2026-09 only, before
the backfill):

| | |
| --- | --- |
| ordinal ≥ 1 `BT-501` rows (dropped at the fold) | **5,897**: GB-PPON 5,681, GB-COH 106, GB-CHC 58, GB-UKPRN 16, GB-SC 15, GB-MPR 11, GB-NHS 10 |
| primary → additional scheme pairs | GB-COH→GB-PPON **4,935**, GB-NHS→PPON 381, GB-UKPRN→PPON 179, GB-CHC→PPON 108, COH→COH 89, MPR→PPON 71, COH→CHC 58, PPON→COH 17 … |
| distinct GB-COH→GB-PPON pairs | **3,295** |
| … whose COH and PPON each stand as a GB national org, as two DIFFERENT orgs | **161**: one supplier served as two organizations |
| … COH org only / PPON org only / neither found | 2,454 / 190 / 490 |

The PPON only exists since February 2025, so the backfill (2021-01 → now, issue 342) mostly adds pre-Act releases with
few identifiers. But the whole 2025-02 → now stretch will multiply the 161.

The "neither found" 490 are worth a look while building. They are pairs whose normalised values match no GB national
org. Candidates: a different normalisation of `RC…`/`SC…` company numbers, identifiers the idgate condemns, or
mentions under a non-GB country.

## Design question (decide first)

1. **Tier.** Is "one party object lists both ids" E1 (merge key) or E2 (candidate edge needing corroboration)? The
   GB arm's comment says E2, and PPON is a platform registration, the class `platform_guids_are_never_a_merge_key`
   refuses to key. But this is not a platform id standing alone: the publisher states, on one party, that the two
   ids are one entity. Default: **E2 edges with the pair as evidence, auto-merged only under the name gate** (the
   R2/R3 machinery that already guards the 447 reunions). Publisher errors do happen (447's Energinet pair), and the
   verdict store is the escape hatch.
2. **Where it is recorded.** Options:
   - (a) a new `org_candidate_edges` rule `e2-altid`, tier `E2`, written by a job that walks ordinal ≥ 1 `BT-501`
     rows of FTS notices;
   - (b) mention-level: carry `additional_identifiers` on `organization_mentions`, so the resolver can bind a
     PPON-first mention to the org its COH already keys.
   (a) fits the existing candidate-edge flow (`org-edge-scan`/`org-edge-census`, issue 360) without touching the hot
   fold path. (b) prevents the split at ingest. Default: **(a) first** (it measures and repairs the standing 161), then
   decide (b) against the edge counts after the backfill.
3. **Consumer.** Which merge arm executes an `e2-altid` edge: R3 (the E2 corroboration arm) or a small dedicated arm
   with dry/wet parity like 447's R2 run.

## Verify

Re-run the split count above (the COH→PPON pairs whose two ids stand as different GB orgs). Today: **161**. Expect 0,
apart from pairs a reviewer keeps apart by verdict, which are listed.

## 2026-09-30 — unit 1 built (dry planner)

Built by a delegated build agent against the settled design, then reviewed and gated here (GATE-EXIT=0, 670 s).

Pieces:
- `crosswalk::{mention_key, altid_pair_key, altid_name_key, gb_legal_family}`;
- `Db::match_org_altid_pairs`: harvest, bipartite graph and conflicts, owners, five structural gates, verdict
  consult, judgment gates, listings. A wet call errors before reading anything, and the planner reads through the
  store's internal pool, not the writer;
- a boxed supervisor arm with R3's `org_match_keys` refusals;
- one ops row.

Deviations the agent made, each argued from code or EXPLAIN evidence:
1. The harvest collects FTS notice ids in one range read per profile. turso seeks `notices_profile` on `profile` only,
   so a keyset walk would re-read the cohort every window. It then does a per-notice PK-range read of `notice_ids`.
2. `already_one` is reachable only as "one side has no org, and the other org's mentions carry its key".
3. The evidence wall follows R2/R3 semantics: disjoint same-scheme key sets on both sides.
4. The store tests use miniature injected rules, per the r3_merge.rs convention. The supervisor test plans a real FTS
   pair through the production functions.

Open doubts, carried to the dry run's reading:
- the wall does not deny a COH org carrying a stray second company number when the PPON org carries none;
- harvest cost after the full backfill is unmeasured;
- a party section with no mention is counted as `unfolded_sections`;
- a HIGH merge verdict overriding a conflict flag has no test yet.
