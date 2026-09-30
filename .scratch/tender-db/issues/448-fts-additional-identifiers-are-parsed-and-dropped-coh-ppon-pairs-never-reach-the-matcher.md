# 448 — FTS `additionalIdentifiers` are parsed and then dropped: the Companies House ↔ PPON pairing never reaches the matcher, and 161 suppliers stand as two organizations

Status: ready-for-agent — UNIT 1b BUILT and gated 2026-09-30 (`e4c39b3` + review fixes `7b6d52e`, both pushed, not yet deployed). Corroboration now ignores the pair's own witness names; new gates `witness-only` and `form-conflict`; three recall folds. Next: deploy when the box queue is idle (backfill chunk 2 runs until ~05:00 UTC), re-run `{"kind":"match-org-identifiers","rule":"altid"}`, and read the Verify for 1b below. Then units 2–4.
Was status (until 2026-09-30 03:0x): ready-for-agent — UNIT 1 DEPLOYED and RUN 2026-09-30 (rev `7f24c30`, dry job 1681, 3 s): 1,992 split pairs, plan 1,575. The dry run found a precision hole: corroboration through satellite names is CIRCULAR, because a witness notice's own party name is recorded as a satellite of the org its first identifier binds. It plans at least one false merge, Amentum Clean Energy (COH 01120437) ← an `Altrad Babcock Limited` PPON org. Next: unit 1b corroborates on names from mentions OUTSIDE the pair's witness notices, then re-run the dry plan. Units 2–4 as designed after that.
Was status (until 2026-09-30 02:5x): ready-for-agent — UNIT 1 BUILT 2026-09-30 (the dry-only planner: `match-org-identifiers` rule `altid`, report `altid-merge-plan`; wet refused until unit 2). Gated green, 20 store tests + 4 crosswalk + 2 supervisor. Next: deploy when the queue is idle, run `{"kind":"match-org-identifiers","rule":"altid"}`, and read the plan against the 161. Design: `.scratch/tender-db/448-altid-design.md`. Filed 2026-09-30 00:0x UTC, as the follow-up `342-fts-plan.md` §5 risk 3 promised and never filed.

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

## 2026-09-30 02:3x UTC — unit 1 on prod: dry job 1681

Deployed at `7f24c30` (with 450), queue idle after backfill chunk 1. `{"kind":"match-org-identifiers","rule":"altid"}`
ran in **3 s** over **82,647** FTS notices (14,647 before the backfill chunk):

    10470 company-number/PPON pairs keyed (10725 literal; unpaired: 186 padded, 3 condemned, 66 malformed,
    25 non-GB; 0 ambiguous parties); owners: 0 already one, 4 multi-target, 29 no company-number org,
    8445 no PPON org, 0 neither, 1992 two distinct GB orgs; denied: 0 gate, 7 consortium, 12 legal-form,
    0 evidence-wall, 0 loser-incoherent, 0 verdict-keep, 341 uncorroborated-overlap, 38 uncorroborated-disjoint,
    0 generic; 19 conflicts; plan 1575 pairs

Report `altid-merge-plan` (a copy is at `/root/altid-plan-1681.json` on the box). The plan listing is capped at 500 of
1,575. The denied (398) and conflict (19) listings are complete.

**both_distinct 1,992 against the issue's 161.** The corpus grew 5.6× (82,647 against 14,647 notices), and the
planner keys owners canonically. In the listings, 45% (plan) to 67% (denied) of the company-number orgs carry the
FTS spelling `GBCOH…`; the rest carry the bare TED number. The hand count matched literals and so could see only one
of the two spellings. Both effects point the same way, and no count is off in a way that suggests a bug.

**0 ambiguous parties, checked.** The pre-backfill "COH→COH 89" were repeats. Bounded `/v1/sql` over notice ids
46.70M–46.79M found 156 party sections listing `GB-COH` more than once, and 0 listing two DIFFERENT numbers.

**Recall: the strict name key costs real matches.** Of the 341 `uncorroborated-overlap` pairs, 13 differ only by dotted
initials or punctuation ("Cardinal Health U.K. 432 Limited" / "Cardinal Health UK 432 Limited"). About 105 more differ
only by a legal-form suffix, mostly one-sided ("Carnall Farrar Ltd" / "Carnall Farrar"). These are approximate counts
from a Python re-normalisation, not the arm's own key. The other 223 differ in words: typos, renames, `t/a` trading
names, and sister or parent companies ("Northumbrian Water Group Limited" / "Northumbrian Water Ltd").

**Precision: the satellite leg of corroboration is circular.** Of the 500 listed plan pairs, 390 have equal head
names (under a rough normalisation), 21 differ only by legal form, and **89 are corroborated only through a satellite
name**. That 89 mixes three kinds:
- true renames, where the company number is one legal entity: Aggregate Industries UK → Holcim UK, Hanson Quarry
  Products Europe → Heidelberg Materials UK, Atkins → AtkinsRéalis UK, Engie Services → Equans Services,
  Magnox → Nuclear Restoration Services, Doosan Babcock → Altrad Babcock;
- publisher errors: Hinduja Global Solutions ↔ Student Loans Company, Fujitsu Services ↔ FCA,
  University of Greenwich ↔ London and South East University Group, Pertemps ↔ Stepping Up Leadership CIC;
- the proven case, checked on Companies House. COH 01120437 is **Amentum Clean Energy Limited** (formerly Amec
  Foster Wheeler Nuclear UK Limited), and COH 00839354 is **Altrad Babcock Limited** (formerly Doosan Babcock). The
  plan pairs 01120437 with PPON PBDC-7744-BTPG, whose org is named "Altrad Babcock Limited". It has 4 witnesses, all
  listing the company number first.

The mechanism: when a party lists the COH first, the fold binds its mention to the COH org and records the party's
name as that org's satellite. That name is the witness's own statement, and it then "corroborates" the pair it came
with. A wrong company number beside the right name and PPON (the Amentum case) passes every gate.

### Unit 1b — decided: corroborate on names from outside the pair's witness notices

For each side, read the org's mentions `(notice_id, name)` through `organization_mentions_org`. That is the same
per-org read the evidence wall already does, now with the name column. Drop mentions whose notice is one of the
pair's witnesses.
- If both sides keep at least one name, corroboration needs an altid-name-key match between the two NON-witness name
  sets.
- A side left with no non-witness mention is an org made only of those witness mentions. For that side only, its
  witness names stand in. Merging such an org moves only the mentions the witnesses themselves put there, which is
  no worse than today.
- A pair that the old rule corroborated and the new rule does not is listed as `witness-only`. It is denied and
  verdict-overridable like the other judgment gates, so true renames reach review with the Companies House history
  as their evidence.

Recall folds, in `altid_name_key` (the design's open question 3, answered by the counts above):
- (a) dotted initials fold (`U.K.` → `uk`, `E.P.` → `ep`) before `n3_key`;
- (b) a trailing `t/a …` or `trading as …` clause is dropped;
- (c) a name with NO legal-form token matches the same name with one. Both-sided different forms stay apart, because
  `gb_legal_family` already vetoes `plc`/`ltd`. `uk`, `group` and `holdings` are still kept.

Verify for 1b: re-run the dry plan. Amentum ← Altrad (01120437~PBDCBTPG) must list as `witness-only`, and the
Doosan → Altrad Babcock pair (00839354) must still plan or list as `witness-only`, never as a merge through a false
name. Record the class counts, then read a fresh 50-pair sample of the new plan.

### Unit 1b — built 2026-09-30 (`e4c39b3`, then `7b6d52e` after the review)

- `mention_rows` reads each org's mentions once, returning the keyed raws (the evidence wall) and every
  `(notice_id, name)` row, nameless rows included.
- `side(org)` picks the names for corroboration:
  - the names of mention ROWS outside the pair's witness notices;
  - if the org has no such row, its witness names;
  - if it has no mention at all, its designated names.
- `names_agree` (`crosswalk::altid_keys_agree`) returns true when two keys are equal, or equal but for a GB legal form
  (`§ltd §plc §llp §lp §cic`) that only one side carries.
- Denied classes, both verdict-overridable, both listed:
  - `witness-only`: the designated names agree and the witness-free ones do not;
  - `form-conflict`: the names on each side carry GB forms, and no form is shared. A formless name can no longer
    bridge a plc and a Ltd.
- The generic wall reads `match_norm(altid_trim(name))`, the same words the agreement read.
- `altid_name_key`:
  - a run of single letters is one initialism, and `&`/`and` ends the run;
  - `co` is keyed as `company`;
  - `altid_trim` cuts a trading-as clause and a parenthetical after the legal form.
- Tests:
  - store (25): `a_name_only_the_witnesses_recorded_never_corroborates` (the Amentum shape, then its HIGH-verdict
    admission), `an_org_made_only_of_witness_mentions_corroborates_with_its_own_names`,
    `a_nameless_outside_mention_still_keeps_the_witness_name_out`, `a_formless_name_never_bridges_a_plc_and_a_ltd`,
    `a_trading_as_clause_does_not_carry_a_generic_name_past_the_wall`;
  - crosswalk (7), including `an_ampersand_ends_an_initialism` and the GB-only strip.

**Review, three lenses (circularity, name key, accounting).** Six findings; five are fixed in `7b6d52e`:
- n3's `§` families were stripped as legal forms (`Siemens Healthineers AG` agreed with `Siemens Healthineers`);
- the generic wall read the untrimmed name;
- `&` did not end an initialism;
- the fallback was decided on keyed names rather than rows;
- the org-level plc/Ltd bridge.

One is **kept by decision**: a company-number org made only of witness mentions still corroborates with its witness
names. Both orgs are then the one party the names agree on. If that number is wrong, it is a fault the witnesses
already put on that org, and the merge neither creates nor hides it. The reasoning is in the code comment. The
residual stays the unit-4 campaign's to watch: `first_coh`-only pairs whose company-number org has no other history.

Verify for 1b, after the deploy: the dry plan lists 01120437~PBDC7744BTPG as `witness-only` or `form-conflict`, never
`plan`. Record every class count, then read a fresh sample of 50 plan pairs.
