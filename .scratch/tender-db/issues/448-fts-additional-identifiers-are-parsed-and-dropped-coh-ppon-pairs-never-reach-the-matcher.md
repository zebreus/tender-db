# 448 — FTS `additionalIdentifiers` are parsed and then dropped: the Companies House ↔ PPON pairing never reaches the matcher, and 161 suppliers stand as two organizations

Status: ready-for-agent — FULL WET RUN DONE 2026-10-01 07:09 UTC (job 1765): 4,652 pairs merged, every one reviewed or register-confirmed. Post-run plan 0; 4,702 pairs are one org. 247 stay split, each denied with a reason. NEXT (small): review the 26 gate-denied pairs that are new since the campaign (`448-campaign/unread-denied-1766.json`, minus consortium/conflict), then read the alias counter on the next fold.
Was status: ready-for-agent — UNIT 4 campaign DONE and verdicts POSTED (336, cohort `altid-2026-09-30`); capped wet run 1717 merged 50 pairs, verified. NEXT: the full wet run over the 4,604-pair residual, which waits for the owner's go-ahead (the session's permission classifier refused it as a bulk production write), then a fold and the alias counters. Follow-ups: issue 452 (wrong company numbers), a verdict override for legal-form re-registrations.
Was status (until 2026-09-30 09:0x): ready-for-agent — UNITS 1b, 2, 3 and 3b DEPLOYED 2026-09-30 (`60a0191`, `636475c`, `ba1b159`). Dry job 1691: plan 4,373 pairs (23 co-occurring), 101 witness-only, 326 uncorroborated, 49 conflicts; the full listing carries each side's witness-free names. The 1b Verify's Amentum expectation was WRONG: that pair is true (see 05:1x below). NEXT: unit 4, the review campaign over the full listing with Companies House register names, then a capped wet run.
Was status (until 2026-09-30 06:1x): ready-for-agent — UNITS 1b and 2 BUILT, reviewed and gated 2026-09-30 (`e4c39b3`, `7b6d52e`; unit 2 `2d55856` + review fixes `2998b0a`; all pushed, deploy pending the box's idle window ~05:30 UTC). NO WET RUN until unit 3 (the resolver alias) is deployed too. Next: deploy, dry re-run and the 1b Verify; then unit 3; then the unit-4 campaign (which first needs the full plan listing — see unit 2's notes).
Was status (until 2026-09-30 04:3x): ready-for-agent — UNIT 1b BUILT and gated 2026-09-30 (`e4c39b3` + review fixes `7b6d52e`, both pushed, not yet deployed). Corroboration now ignores the pair's own witness names; new gates `witness-only` and `form-conflict`; three recall folds. Next: deploy when the box queue is idle (backfill chunk 2 runs until ~05:00 UTC), re-run `{"kind":"match-org-identifiers","rule":"altid"}`, and read the Verify for 1b below. Then units 2–4.
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

## 2026-09-30 — unit 2 built (the wet path), reviewed, fixed

Built by a delegated agent (`2d55856`, gate 141 suites green). Four review lenses followed: merge correctness, the
stored-plan contract, transactions, edges and readers. Their fixes are in `2998b0a`, gated green.

**What a wet run does** (`{"kind":"match-org-identifiers","rule":"altid","dry_run":false}`):
- The supervisor reads the latest `altid-merge-plan` and refuses if there is none. It applies R3's `org_match_keys`
  refusals and refuses without the org FK indexes.
- The store re-plans under the writer. It aborts before any write when the live and stored sets differ by more than
  max(2%, 5), counted as a symmetric difference.
- It merges live ∩ stored in sorted key order, 50 pairs per `BEGIN IMMEDIATE` with foreign keys ON:
  `repoint_org_references` (keep = the company-number org), delete the PPON org, an `org_merge_log` row `e2-altid`
  with flat evidence, change events, the admitting verdict stamped, and the pair's `e2-altid` edge set to `merged`.
  Each transaction ends with COMMIT, a TRUNCATE checkpoint and a cursor publish.
- Live pairs the stored set lacks are `deferred_unreviewed` and never merge.
- A PPON org that two live pairs claim merges in neither pair (`contradictory`).
- The stop is polled between transactions. The residual is re-recorded as the new plan (`residual_of_wet_run`).
- Denied two-org pairs then become open `e2-altid`/`E2` candidate edges. Edges are never deleted.
- `write_candidate_edges` is extracted from `scan_org_match_keys` with the tier as a parameter. The census and the xb
  packet read `e3-*` rules only, and e2 edges are counted apart.

**Review findings fixed in `2998b0a`:**
- **Contested PPON across the stored set** (medium; three lenses reported it independently). The claim count ran
  after the cut to reviewed pairs. A HIGH verdict posted after the dry run, on a deferred pair, therefore did not stop
  the reviewed pair from folding the shared PPON org. It is now counted over the whole live plan. Test:
  `a_later_admitting_verdict_on_a_deferred_pair_still_contests_the_ppon_org`.
- **Edge-phase failure after the merges committed** (low). It lost the residual record and locked the next wet run out
  on parity. The edge phase and the closing publish are now reported (`edges_error`), not raised.
- **Known-deferred pairs re-counted as drift** on a continuation, whose tolerance is smaller (low). The supervisor now
  passes a residual's `deferred_pairs` as `known_deferred`. Test:
  `a_continuation_does_not_recount_pairs_an_earlier_run_deferred`.

**Carried to unit 4, not defects:**
- The plan LISTING is capped at 500 (`R2_PLAN_LISTING_CAP`), while `pairs` is uncapped (1,575 in dry job 1681). A
  reviewer cannot read the pairs past the cap, but a wet run would merge them. **Before any wet run, the campaign
  needs every planned pair's members and witnesses.** Either a paged listing, or a cap raised for this report only.
  It is unit 4's first step.
- A stale plan recorded before unit 1b's gate changes still passes parity only to the extent it matches the live
  re-plan, and it merges only live ∩ stored. Re-run the dry plan after every deploy that changes the gates anyway.
- Only `docs/operations.md` enforces "no wet run before unit 3". An early run is not destructive, but the fold would
  re-mint the merged PPON orgs.

## 2026-09-30 05:1x–05:5x UTC — deployed (`60a0191`); the 1b Verify read on dry job 1690

**Counts** (dry job 1690, 8 s, 145,568 FTS notices after backfill chunks 1–2):
- 17,254 company-number/PPON pairs keyed (17,622 literal). Unpaired: 267 padded, 5 condemned, 96 malformed, 46 non-GB.
  0 ambiguous parties.
- Owners: 0 already one, 7 multi-target, 35 no company-number org, 12,328 no PPON org (the PPON never led a party),
  0 neither, **4,884 two distinct GB orgs**.
- Denied: 0 gate, 16 consortium, 17 legal-form, 0 evidence-wall, 0 loser-incoherent, 0 verdict-keep,
  254 uncorroborated-overlap, 72 uncorroborated-disjoint, **101 witness-only**, 0 form-conflict, 2 generic.
  49 conflicts.
- **Plan: 4,373 pairs** (1,575 at unit 1, before the backfill and 1b).

**The Verify's Amentum expectation was wrong, and planning the pair is right.** Unit 1 read PBDC-7744-BTPG as Altrad
Babcock's PPON because its org is HEADED "Altrad Babcock Limited". The backfill shows otherwise:
- UKAEA's notice 46740078 lists both suppliers, each with its own pair: Amentum Clean Energy = COH 01120437 + PPON
  PBDC-7744-BTPG, and Altrad Babcock = COH 00839354 + PPON **PBDJ-7746-PBLD**.
- The PPON org PBDC-7744-BTPG carries 14 mentions. 11 name Amentum Clean Energy; 3 name Altrad Babcock. The 3 come from
  UK Industrial Fusion Solutions notices (46761121, 46848741, 46848786), which list Amentum by its company number AND
  "Altrad Babcock Limited" under Amentum's PPON as a second party: a publisher's mislabel. The first-seen election
  made that stray the head.
- Altrad Babcock's real PPON org (PBDJ-7746-PBLD) carries 8 mentions, all "Altrad Babcock Limited".
- So 01120437~PBDC7744BTPG corroborates on witness-free names (Amentum ↔ Amentum), and 1b plans it correctly. The 3
  mislabelled mentions move with the merge; they are the publisher's fault on either org.

Doosan → Altrad Babcock (00839354~PBDJ7746PBLD) lists **`witness-only`**, as the Verify allowed: its company-number org's
only witness-free name is "Doosan Babcock Ltd" (a TED mention, the pre-rename name). It is a true rename and goes to
review.

The circularity 1b fixes is still real. The synthetic test keeps the shape, with its doc corrected.

**The 50-pair read.** Of the 500 listed plan pairs (the deployed listing cap), 406 have equal head names under a rough
norm, and 94 differ. Every one of the six most suspicious-looking heads is one entity by its witness-free mention names:

| pair | heads | mention names |
|---|---|---|
| 00968498~PHXX6931VYBP | Northgate Public Services ↔ Gravitas Recruitment Group | both mostly NEC Software Solutions UK (Northgate's rename); the PPON org also holds 1 Gravitas and 1 Cadcorp stray |
| 00986729~PPJL2485DZGH | University of Greenwich ↔ London and South East University Group | Greenwich 22 + LSEUG 2 / LSEUG 12 + Greenwich 2: the consortium publishes under Greenwich's numbers |
| 01007314~PWYW9582JTRT | Busch (UK) ↔ Vacuum Furnace Engineering | "Busch (UK) Ltd trading as Vacuum Furnace Engineering" |
| 00115834~PXRT4831MJVW | Communicare247 ↔ Legrand Electric | Legrand Electric 22 + Communicare247 (its brand) 6 / Legrand Electric |
| 00062537~PYXP9254JYBJ | Bunzl Retail & Healthcare t/a Care Shop ↔ Mediq Healthcare UK | Mediq Healthcare UK on both sides (5 of 6, 17 of 17) |
| 00454264~PPLG5515HRRP | MWUK ↔ Mi Hub | MI HUB LIMITED 7 + MWUK 2 / Mi Hub 2 (rename) |

The rest of the 94: renames (Actavis → Accord UK, Johnston Sweepers → Bucher Municipal, Engie → Equans, Hanson →
Heidelberg Materials, Interserve Construction → Tilbury Douglas, MORI → Ipsos, Atkins → AtkinsRéalis), trading names,
typos (Envionmental, Infrastrucutre, Metler-Toledo), dotted initials and bracketed numbers.

**What that says about the listing.** A head name is a first-seen election, and a stray mention can set it. The listing
showed only heads, so each of these pairs cost a /v1/sql read to judge. Unit 3b puts the reviewer's evidence in the
listing:
- each side's witness-free names;
- the name pair that cleared the wall;
- the notices where both orgs appear as distinct parties (the UKIFS shape), counted in `plan_cooccurring`. This is
  measured before any gate reads it: a true pair beside a mislabelling publisher shows it too.

## 2026-09-30 — unit 3 built, reviewed, DEPLOYED (`5b52135` + review fixes `636475c`, live 05:43 UTC)

**The resolver alias.** `Db::arm_altid_alias` arms the fold's mention resolver at both fold call sites; every other
caller is unarmed and byte-identical to before.
- The preload reads the `e2-altid` ledger rows as PPON key → company-number key. It maps identity to identity, so the
  alias survives a rebuild's renumbering.
- A pair under a `keep` verdict is dropped. A PPON merged into two company numbers is poisoned.
- A GB mention whose raw identifier keys to an aliased PPON is aliased only after the triple and the canonical key
  both miss. It then binds to the org that owns the company number, at the arm's own bar: a standing owner, the
  consortium veto, the legal forms head against head, the shared name predicate (`altid_corroborates`) and the
  generic wall.
- A bind is never cached (issue 318). A refused mention mints exactly as it would unarmed.
- Counters go to the diag log (`[issue 448]`) and to the `project` counts line (when anything was asked).

**The full plan listing.** `ALTID_PLAN_LISTING_CAP` = 20,000, so a reviewer sees every pair a wet run would merge.
The denied and conflict listings keep 500.

**Review, two findings, both fixed in `636475c`:**
- The preload scanned `org_merge_log` on every fold. The ledger is NOT small: p0 alone wrote 5.76M rows (issue 351).
  It now seeks the partial index `org_merge_log_e2_altid` (built once at the first open, which health confirmed at the
  deploy). The harvest plan guard pins it (`ALTID_ALIAS_LEDGER_SQL`).
- `bound_unwalled` leaked across corroborating pairs: a probe failure on a pair that did not clear marked the pair
  that did. The flag is now per pair.

Verify for unit 3: the next fold's `project` counts line and the diag log show the alias ARMED with 0 aliases (no
`e2-altid` merge exists yet), and asked/bound/refused all 0. The first non-zero read comes after the first wet run.

## 2026-09-30 — unit 3b built, reviewed, DEPLOYED (`ba1b159`, live 06:0x UTC): the reviewer's evidence in the listing

Every two-org listing (plan, denied, conflicts) now carries:
- `coh_names` / `ppon_names`: the witness-free names each side's corroboration read, the first 8 in key order, with
  `coh_name_keys` / `ppon_name_keys` as the totals;
- `corroborated_by`: the name pair that cleared the wall (planned pairs);
- `cooccurring` / `cooccur_publications`: notices OTHER than the pair's witnesses where both orgs are distinct parties.

`plan_cooccurring` counts the planned pairs that co-occur; it gates nothing.

The review ran two lenses (equivalence of the moved corroboration code; honesty of the evidence). Its four findings are
fixed in the same commit:
- the stored `plan_cooccurring` counts the STORED plan (the residual after a wet run);
- a witness notice never counts as a co-occurrence (the same supplier listed twice);
- name totals ride beside the capped lists;
- co-occurrence publication ids are read in `push_listing`, only for listed pairs.

**Dry job 1691** (on `ba1b159`, 4 s): the same counts as 1690, and plan 4,373 with **23 co-occurring**. The full
listing is 4,373 entries, not truncated (4.4 MB). The Amentum pair now reads:
- `coh_names` 5: Amec Foster Wheeler Nuclear UK, AMEC Nuclear UK, Amentum Clean Energy, Jacobs Clean Energy, Wood
  Nuclear;
- `ppon_names` 2: Altrad Babcock Ltd (the stray), AMENTUM CLEAN ENERGY LIMITED;
- `corroborated_by`: (Amentum Clean Energy Ltd, AMENTUM CLEAN ENERGY LIMITED);
- `cooccurring` 2: 072223-2025 and 074827-2026, the UKIFS mislabel.

Next, unit 4: the campaign over the full listing, with Companies House register names joined in (fetched from the
public company pages for all 4,872 company numbers in the plan, denied and conflict sets).

## 2026-09-30 06:4x–09:0x UTC — unit 4: the review campaign, the verdicts, and a capped wet run

**Register first.** Companies House's public page was fetched for every one of the 4,872 company numbers in dry job
1691's plan, denied and conflict sets: 4,868 answered, 4 do not exist. A planned pair is REGISTER-CONFIRMED when every
witness-free PPON-side name is a register name (current or previous, by core), a company-number-side name is one too,
and the two orgs never co-occur outside the witnesses.
- 4,032 of 4,373 planned pairs confirmed.
- A blind sample of 40 of them, read by a reviewer: 40 merge (38 high). The split holds.

**The campaign** (`.scratch/tender-db/448-campaign/`: `rubric.md`, `cases-1691.json`, `results-2026-09-30.json`) ran
43 agents: 21 reviewer batches of 40, a challenger on every verdict that would act, and the blind sample. Reviewers
checked the register (and officers/PSC pages) where a case was unclear. It read 836 cases.
- **Planned pairs (341):** 312 merge. 29 are HELD with a `keep` verdict: 16 high (challenger agreed), 13 low (disputed,
  or needs more evidence; re-post as `merge` in the same cohort to release one). The held are wrong company numbers
  (nonexistent, transposed digits, a dissolved or dormant shell, a council under a marketing company's number; filed as
  **issue 452**), parent vs subsidiary (SGN, Group 1/Barons, Concur NL), and PPON orgs whose names split between the
  company and an unrelated one (D3 office group, CRS Communications, Big Yellow).
- **Denied and conflict pairs (495):** 307 ADMITTED by a HIGH `merge` verdict with the challenger agreeing:
  143 uncorroborated-overlap, 96 witness-only, 30 disjoint, 27 conflicts, 10 legal-form, 1 generic. They are renames
  the register's previous names prove, acronyms, spacing, and trading names.
- POSTed as cohort `altid-2026-09-30`: 336 verdicts (307 merge/high, 16 keep/high, 13 keep/low)
  (`verdicts-altid-2026-09-30.json`).

**Dry job 1716** (after the verdicts and today's daily): plan **4,654**, with 29 verdict-keep and 297 verdict-admitted.
The 10 legal-form admits did NOT apply: the head-against-head legal-form veto is structural and no verdict overrides it
(by design). They are mostly re-registrations the register proves (3M UK plc → Ltd, Axis Europe plc → Ltd, SCC plc);
whether a HIGH verdict should override that veto for a re-registration is an open question for a later unit. The plan
held 14 pairs the campaign never saw (today's FTS daily). All 14 are register-confirmed by the same rule
(`delta-new-1716.json`). So every pair of the stored plan was reviewed or register-confirmed.

**Capped wet job 1717** (`max_groups` 50, 4 s):
- merged 50 pairs: 50 org rows removed; 198 mentions, 253 parties, 9,046 bid-parties and 9,076 winners repointed;
  101 tenders touched;
- 0 deferred, 0 contradictory;
- 247 `e2-altid` edges written for the denied pairs;
- residual 4,604 re-recorded.

Verified with /v1/sql: the 50 PPON orgs are gone, the 50 company-number orgs stand, 0 mentions remain on the losers,
and the keepers now hold the 198 PPON-keyed mentions.

**The full wet run (the 4,604-pair residual) is NOT run.** The session's permission classifier refused it as a bulk
production write, and it waits for the owner's go-ahead. Nothing drifts meanwhile:
- the residual is recorded and the verdicts are stored;
- `{"kind":"match-org-identifiers","rule":"altid","dry_run":false}` resumes it under parity;
- if new FTS days land first, re-run the dry plan and register-check the delta the way 1716's 14 were checked.

**Unit 3 Verify, read:** the diag log shows the alias ARMED on today's folds (1699, 1714) with 0 aliases, 0 asked. After
job 1717 the ledger holds 50 `e2-altid` rows. The next fold should read "50 PPON key(s) aliased".

**Audit, co-occurrence (23 → 25 planned pairs):** nearly all are one supplier listed twice in a notice (a tenderer
section by company number, a supplier section by PPON). The suspect ones (Southern Water's PPON carrying a fire
authority's name) went through the campaign. Co-occurrence stays a displayed signal, never a gate.

**Legal-form admits, analysed (08:5x UTC): no override yet.** A HIGH verdict could be made to skip the head-against-head
legal-form veto, but that alone would not hold:
- after the merge the verdict is stamped applied, so the next plan judges the pair by the gates again;
- the resolver alias vetoes a later PPON-first mention the same way. The survivor's stale `plc` head sits beside the
  supplier's new `Ltd` name, so the mention would mint a new PPON org, and the pair would split and be re-denied.

The root is the head name: a first-seen election that goes stale after a re-registration (3M UK plc → Ltd, Axis
Europe plc → Ltd). The fix belongs in head election (or in a legal-form check that reads the register's current form),
not in a verdict carve-out. The 10 pairs stay denied, with open `e2-altid` edges, until then.

## 2026-10-01 07:0x–07:2x UTC — the full wet run

- **Fresh dry plan, job 1764** (after FTS chunks 5–6 and 453's re-keys): **4,652 pairs**, 294 verdict-admitted,
  21 verdict-keep, 0 verdicts stale. The stored plan dated from before chunks 5–6, so it was replaced, not resumed.
- **Delta review.** 4,598 keys were already reviewed: the 4,032 register-confirmed in 1691, the 836 campaign cases
  and the 14 of 1716. **54 are new.** The register was fetched for their 82 new numbers (all 200), and the
  campaign's own rule confirmed 53 (`delta-new-1764.json`, `delta-1764-split.json`). The one it could not confirm
  was read: `03341254~PHYC4884JJWW`, CRYOPDP. 03341254 is PDP COURIER SERVICES LIMITED, and the PPON side names
  itself "PDP Courier Services Ltd. (CRYOPDP)"; CryoPDP is that company's trading name, so it is one entity and
  the merge is right.
- **Wet, job 1765** (26 s): merged 4,652 of 4,652. 4,652 org rows were removed; 19,577 mentions, 26,490 parties,
  324,888 bid-parties and 332,248 winners were repointed; 7 winner duplicates were deleted; 8,303 tenders were
  touched. 0 were deferred, 0 no longer planned, 0 contradictory. 247 `e2-altid` edges were written for the denied
  pairs.
- **Checked.** In five sampled pairs across the plan (indices 0, 1000, 2500, 4000 and 4651), each keeper serves
  200 on `/v1/organizations/{id}` and each loser 404.
- **Post-run dry, job 1766: plan 0.** 4,702 pairs are already one org. 247 stand as two GB orgs, and each is
  denied with a reason: 21 verdict-keep, 16 consortium, 17 legal-form (the head-election root above), 110
  uncorroborated-overlap, 46 uncorroborated-disjoint, 21 witness-only, 1 generic and 15 conflicts.
- **Verify (the split count): 161 → 247.** It reads higher than at filing because the backfill multiplied the
  pairs (17,445 keyed today against the 161 counted then). Every remaining pair is now held apart by a gate or a
  verdict, and none is a missed merge. **44 of the 247 were never read by the campaign**
  (`448-campaign/unread-denied-1766.json`):
  - 16 consortium. The veto skips these by design. 13 share Birmingham's consortium PPON PJXZ4423NDBT with
    housing associations. Golley Slater, BearingPoint, Consortium Trust and SEC Procurement look like single
    entities caught by the word or the shape; they stay held.
  - 2 Essity conflicts (coh-multi-ppon), correctly held.
  - **26 gate-denials that are new since the campaign:** 16 witness-only, 6 uncorroborated-overlap, 4
    uncorroborated-disjoint. Some are one entity (RelyOn Nutec, 1st Coverall, Pentagon Solutions NI, Doctors
    Training); others are clearly different (Synectics vs Ocular Integration, CP Media vs Outdo Media). This is
    the next small review: a HIGH `merge` verdict admits a true pair on the next wet run.
- **Alias.** Jobs 1717 and 1765 between them merged 50 + 4,652 pairs. The next fold, the 07:35 UTC daily project,
  should report PPON keys aliased in the `[store]` diag line. That is unit 3's live Verify.
