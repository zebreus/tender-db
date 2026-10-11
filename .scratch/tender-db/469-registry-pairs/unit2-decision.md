# Issue 469 — unit 2 decision

Decided 2026-10-11 by a design panel (workflow wf_28c893dd-a43: three independent designs — parity, evidence-only, minimal — each adversarially judged against the code; scores: minimal safety 9 / cost 8 / correctness 8, parity 8/3/7, evidence 7/5/7 with one fatal flaw). Synthesis below, verbatim.

## Decision

The minimal design (C) wins. Its adversarial scores are the best of the three: safety 9, cost 8, correctness 8, with no fatal flaw. Parity (A) scored safety 8 but cost 3. Evidence (B) scored safety 7 and carries a fatal flaw: it claims a repair path for register identities that does not exist. C is changed in one way: "defer" becomes "build after a bounded read, with stop rules". So yes, the 448 arm is extended to GB-UKPRN, GB-CHC, GB-MPR, GB-SC and GB-NIC, as one generalisation over schemes rather than a copy, and in this order:
1. Write the decisions down.
2. Clear the company-number arm's own backlog, which needs no new code.
3. Read the sampled pairs.
4. Build a narrowly scoped arm whose wet run needs register confirmation or a HIGH verdict with a challenger.

Grafted from A:
- register confirmation from public sources (the UKRLP lookup, the Charity Commission and OSCR bulk extracts, FCA and CCNI read by hand);
- a register channel for the evidence wall and the loser-incoherent gate;
- parity held separately for company-number pairs and register pairs;
- wet path and alias shipped in one deploy.

Grafted from B:
- no notice-count (k) threshold and no co-occurrence gate;
- scheme-qualified verdict keys;
- a separate stored set of planned register pairs;
- a coherence flag that is listed, not gated.

Rejected:
- B's lane for merging pairs nobody has read, and B's claim that issue 452 verdicts repair wrong merges afterwards;
- A's triples admitted by a verdict.

NHS is split in two. Letter-only ODS codes get their own identity issue and are never merged before it lands. Identity-minting NHS codes (at most 64 pairs, including the issue's Leeds `GBNHSRR8` exhibit) join the arm as a later unit.

## Why

**The numbers (issue 469:3).**
- 260 splits in all. 155 have a register side that mints an identity: UKPRN 71, CHC 48, MPR 32, NIC 2, SC 2.
- NHS has 105 splits out of 195 pairs. 131 of the 195 mint no identity and 73 are multi-target. Since `both_distinct` and `multi_target` are separate classes (canonical.rs:22018-22038), at most 64 NHS splits mint an identity and at least 41 are letter-only codes.
- `no_target_ppon` is CHC 134, UKPRN 79, MPR 26. `already_one` is 0, so no register↔PPON merge has ever run.

**Size against the company-number arm.** The 155 is about 3% of the roughly 4,700 pairs the 448 arm merged. Meanwhile that arm's own two-org count went from 231 at job 1788 (448:503-505) to 747 at job 2163. That growth is reviewable with tools that already exist, so it comes first. Value is also capped below 155: `ppon_beside_coh`, the triple shape, is measured (canonical.rs:21713) but not reported in the Status line, and every triple is denied below.

**Why the register check is required.** 448's precision came from it. Companies House confirmed 4,032 of 4,373 planned pairs. The 29 held keeps were wrong numbers and parent/subsidiary pairs (448:357-376).
- B's own recomputation shows that repeating a pair across notices adds nothing. Keep rates by witness count are flat: 0.83%, 0.53%, 0.46%, 0.66%. Its scorer reproduced this.
- B's name coherence leaves a residue of about 0.2%: the same names standing on someone else's number.
- That residue has no repair path for register identities:
  - Withheld orgs only drop out of the planner's owner walk (canonical.rs:21839, 21901).
  - The resolver guards canonical keys only (canonical.rs:14785-14836), and a register literal keys nothing (crosswalk.rs:429-433). The exact triple still binds.
  - A keep verdict posted after a merge only disarms the alias (canonical.rs:15039-15041).
  - The one undo on record is deleting the ledger row and refolding (ADR 0003:59-61).

  So confirmation has to happen before the merge.

**What stops a plain extension (verified).**
- `AltIdListing` is typed to Companies House (canonical.rs:5599).
- The verdict key is `{coh}~{ppon}` (canonical.rs:22367).
- The alias preload reads only `$.coh` and `$.ppon` (canonical.rs:5910) and skips a row with an empty coh (canonical.rs:15063-15067). A register merge without the new alias would therefore regrow silently.
- The alias finds its owner through `canon_of` (canonical.rs:15186).
- The survivor is always the company-number org (canonical.rs:21465).
- The evidence wall and loser-incoherent read keys from `mention_key` → `canonical_key_flat` (crosswalk.rs:1153-1156). That function keys no register, so two different UKPRNs pass the wall (canonical.rs:22322-22346).
- The evidence field name `"scheme"` is taken: it is `"GB:altid"` on every `e2-altid` row (canonical.rs:22727).

**Hazards found while verifying.**
- *Survivor reversal.* Merged identifiers make a 448 survivor the PPON-side holder (canonical.rs:21857-21894). A "register org keeps" rule would then fold a company-number org into a charity org.
- *`org_of` is too loose for the alias.* It is first-row-wins and includes withheld orgs (canonical.rs:14836).
- *A Scottish charity number without its `SC` prefix mints a VAT.* `GB-SC-012345` becomes `GBSC012345`, whose letter run is under 3, so it mints `kind = vat` (project.rs:8829-8832). This is why the shape gate requires `national`. A's check C, which feared this value would key as a company number, is dropped: the GB arm skips VAT kinds (crosswalk.rs:391).
- *Every FTS literal already carries its scheme token* (fts/parse.rs:235).

## What is built

Each unit ships on its own. Every commit goes through `ops/check.sh`.

- **U0 — decision record and reads (no code).**
  - Write decisions 1-4 on issue 469: pairs are E2 statements, never a key; the register org survives; triples are a hard deny; spellings get their own issue.
  - Read job 2163's stored `altid-merge-plan` through bounded reads. Per scheme: `ppon_beside_coh`, `ppon_beside_two_values`, `multi_target`, the 124-pair `registry_sample`, and literal shapes (label pollution, linked-charity suffixes, MPR and NIC formats).
  - Apply stop rule S1.
- **U1 — the company-number backlog (no build; 448's standing NEXT).**
  - Read job 2163's `plan_pairs` and the 747 split; register-check the delta with `ch_fetch.py`; post HIGH verdicts; run wet.
  - Explain why `already_one` fell from 4,733 to 4,564 while keyed pairs rose to 18,390. A gate regression found here is fixed before U4, because every gate is shared.
- **U2 — read the register sample (scratch tooling only).**
  - Scope: the 94 non-NHS sampled pairs (30 UKPRN, 30 CHC, 30 MPR, 2 NIC, 2 SC), plus the 30 NHS pairs, which are checked against the ODS API for U7.
  - Fetchers: one UKRLP GET per UKPRN; the Charity Commission bulk extract (name, status, other names, `linked_charity_number`, `charity_company_registration_number`); the OSCR bulk zip; FCA and CCNI by hand. All were probed from the agent container on 2026-10-11 (A, re-run by its scorer).
  - Record per pair: mention mass on each org, buyer or supplier role, whether the PPON holder is a 448 survivor, the register verdict, and the rubric verdict.
  - Apply stop rules S2 and S3. The other 61 pairs are read in U6, from U4's listing; no deploy is needed to lift the sample cap at canonical.rs:5713.
- **U3 — pure keys (crosswalk.rs, no behaviour change).**
  - `registry_pair_key(scheme, value, country)` covers exactly the five schemes. It runs `normalise_identifier` and requires `kind == national`, the scheme token as the identity prefix (`GBUKPRN…`, `GBCHC…`), and a per-scheme shape frozen from U0.
  - `registry_mention_key` gives the wall a register channel.
  - Left untouched: `altid_pair_key` (1174-1185), the GB arm, `gb_other_registers_and_vats_key_nothing` (747-751), and the `None` asserts (1355-1358).
  - Tests: `registry_pair_key_requires_the_scheme_token_and_a_national_kind`, `a_scottish_charity_without_its_sc_prefix_mints_a_vat_and_is_refused`, `a_glued_linked_charity_suffix_fails_the_chc_shape`, `label_spellings_are_refused_not_stripped` (`GBUKPRNUKPRN10007798`, `GBCHCCHARITYNUMBER216250`).
- **U4 — dry planner (canonical.rs and supervisor listing).**
  - Turn unit 1's `reg_pairs` (21673-21731) into a gated graph keyed (scheme, identity, PPON), kept apart from the company-number graph so its conflict counts never move.
  - Verdict key `<scheme>:<identity>~<ppon>` under (`GB`, `GB:altid`).
  - A separate stored `registry_pairs_planned`, which the current wet path never reads.
  - Listing fields: `scheme`, `registry`, `registry_literal`, k, p (distinct buyer orgs, read only for witness notices), self, coherent.
  - Tests: `an_ukprn_ppon_pair_is_planned_and_keeps_the_ukprn_org`, `a_ppon_beside_a_company_number_and_a_charity_is_a_triple` (both the party form and the merged-holder form), `a_ppon_beside_two_registers_is_cross_register`, `a_ppon_beside_two_ukprns_is_a_conflict`, `a_register_side_held_only_by_the_name_bound_org_never_plans`, `a_withheld_register_owner_never_plans`, `the_register_wall_denies_two_different_ukprns`, `registry_pairs_never_change_the_coh_plan` (unit 1's contract), the updated `an_nhs_ppon_pair_is_counted_and_never_planned` (altid_merge.rs:1883), and a round trip of the supervisor listing (supervisor.rs:9080-9111).
- **U5 — wet path and alias, in one deploy.**
  - `altid_wet` gains a register branch: keep = register org, loser = PPON org. The ledger evidence is `e2-altid` with `registry_scheme`, `registry_kind`, `registry`, `registry_literal`, `ppon`, `ppon_literal`, `keep_id`, `loser_id`, `name_key`, `witnesses`, `publishers`, `witness_notices`, `verdict` and `confirmed` (register or verdict). There is no `coh`, so an old preload ignores the row.
  - The 460 backfill reads only `$.loser_id` (canonical.rs:4931-4933) and needs no change.
  - Reviewed-set parity of max(2%, 5) is held **per family**. The contradictory-loser guard (canonical.rs:22657-22673) counts **across** families.
  - `ALTID_ALIAS_LEDGER_SQL` also reads the register fields, still through the partial index at canonical.rs:490.
  - `alias_of` becomes PPON → `Coh(key) | Registry(kind, identity)`. A register target binds through the exact triple only when one non-withheld row holds it. The resolver's open scan (canonical.rs:14826-14836) records the triples standing on two or more rows and the triples of withheld orgs, and the alias refuses either. A PPON aimed at two targets of any kind is poisoned. The name bar is 448's (canonical.rs:15130-15150).
  - Tests: `a_ukprn_ppon_pair_merges_into_the_ukprn_org_and_the_alias_binds_the_next_ppon_mention`, `a_registry_ledger_row_never_reaches_the_coh_alias`, `a_ppon_aliased_to_a_company_number_and_a_register_is_poisoned`, `a_register_target_on_two_rows_or_withheld_binds_nobody`, `registry_drift_aborts_even_when_coh_parity_holds`, `a_ppon_org_two_families_would_fold_merges_neither`. The plan guard `the_harvest_seeks_notices_profile_and_the_notice_ids_pk` and the stack canary `an_execute_without_an_expected_count_is_refused` must stay green; the arm is already boxed at supervisor.rs:8938.
  - Docs: operations.md:2066 and the DDL comment at canonical.rs:741-749.
- **U6 — campaign and runs (from the agent container).**
  - Register-first confirmation using 448's rule (altid_cases.py:69-75). For CHC it also requires a match on `linked_charity_number` and a registered status.
  - Everything not confirmed goes to reviewer plus challenger, HIGH only, cohort `altid-registry-<date>`.
  - Then: dry run, capped wet run (`max_groups` 20), a ledger read, full wet run, project, and the alias counters.
  - Verify: 30928147 / 30969231 read `200 308`.
- **U7 — identity-minting NHS.** Add GB-NHS to the scheme list behind an ODS shape gate, after U6 runs clean and the U2 NHS read passes. Test: `an_nhs_ppon_pair_merges_into_the_nhs_org_and_the_alias_binds_the_next_ppon_mention`. `a_register_value_with_no_identity_is_held_by_the_org_its_mention_bound` (altid_merge.rs:1937) must still never plan. Verify: 31540413 / 30913623 read `200 308`.
- **Filed separately:**
  - N: NHS letter-only identity.
  - S: spellings within one register.
  - T: the company-number↔charity cross-walk for charitable companies.
  - Prevention for the 239 `no_target_ppon` pairs, deferred as 448 deferred its company-number-only pairs.

## Gates and the wrong-merge guard

**Structural (no verdict overrides):**
1. The v2 gate on both literals (canonical.rs:22296-22300).
2. **registry-shape** (U3).
3. Consortium.
4. Legal form, head against head (22301-22317).
5. The evidence wall, with the register channel.
6. Loser-incoherent: the PPON org's evidence names any GB key, register keys included (22342).
7. **triple**: a party showed the PPON beside a company number (`reg_beside_coh`, 21713), or the PPON-side holder carries a `GB:coh` key of its own or through merged identifiers (21857-21894).
8. **cross-register**: the PPON is partnered with two identities of one scheme (21710) or with identities of two schemes.
9. **single-owner**: exactly one non-withheld org holds the identity. A side held only through `first_bound` never plans (21686-21698).

**Judgment (a HIGH verdict overrides):** the conflict flags; `altid_corroborates` on witness-free names (canonical.rs:6042); the generic wall.

**Listed, never gated:** k, p, self, coherent, co-occurrence. In 448, co-occurrence flagged 0 of 29 wrong pairs but 23 of 312 true ones (B, reproduced by its scorer).

**The guard against a wrong merge.** No pair merges unless it is register-confirmed or carries a HIGH verdict with the challenger agreeing. There is no unread lane. The first wet run is capped and its ledger is read. Merge and alias go live together. Because no automated undo exists (see Why), these checks happen before the merge.

## NHS

- **Letter-only codes: no merge until issue N.** They mint no identity because of the general digit rule (project.rs:8690), not an NHS policy. Merging them is unsafe and also regrows:
  - The alias maps identity to identity (canonical.rs:6150), so there is no target.
  - Provisional reuse needs `identifier IS NULL` (canonical.rs:16326-16329). With the PPON org surviving, the next register-first mention mints a new provisional org. With the provisional org surviving, the PPON-first mentions have nothing to alias to.
  - The 300 ladder puts a provisional↔canonical capture at E3: "Never merges" (300-design:712).
- **What issue N measures and decides:**
  - Measure: the distinct letter-only GB-NHS values, placeholders such as `TBC` and `NA`, and the 73 multi-target codes (name variants of one body, or different bodies under one code), with codes checked against the ODS ORD API.
  - Decide: an exemption to the digit rule scoped to `GBNHS` plus an ODS shape, still under `idgate::condemns`.
  - Ship with a 345-style re-key repair (project.rs:8640-8647). `publishes()` compares only the raw string (canonical.rs:9434-9439), so standing mentions would not re-bind otherwise.
  - Collapsing the 73 is itself a merge decision and needs its own gates.
- **Identity-minting codes: U7.** These do not depend on issue N: minting letter codes later leaves `GBNHSRR8` unchanged. NHS parties are mostly buyers stating their own codes (69 of 71 parties, uk-fts.md:157), which lowers typo risk. U7 waits for the U2 NHS read because site-versus-trust granularity (a 5-character site code beside a trust's PPON) is unmeasured. Treating one register two ways for a while costs coverage, not safety.

## Risks and the stop rules

**Risks:**
- *Yield shrinks.* Triples, cross-register partners and register-side holders that a 448 survivor already holds are all denied.
- *Name corroboration may miss.* `altid_name_key` and `gb_legal_family` were tuned on company names (crosswalk.rs:1212, 1312), so trusts, universities and charities may be denied rather than wrongly merged.
- *Linked charities.* Registered number 200027 belongs to two charities in the extract (A's probe).
- *Silent regrowth* if U5's two halves ship apart (canonical.rs:15063-15067).
- *The 239 `no_target_ppon` pairs stay unprotected.* The alias replays ledger merges only.
- *Stack.* A new arm adds stack inside `run_spec`.
- *Gate time.* 4-6 runs of `ops/check.sh`, about 11 minutes each, about 35 minutes from clean.

**Stop rules:**
- **S1, after U0.** If the plannable non-NHS splits (`both_distinct` minus triples and cross-register pairs) number fewer than 50, stop before U3. The cost, about 448 units 1b-3, is not repaid. Keep the pairs measured and re-check at each dry altid run.
- **S2, after U2.** If one wrong class appears at least 3 times (charity vs its trading subsidiary, university vs its company, housing group vs member, linked charity vs main), turn it into a structural deny before U4. If it cannot be expressed as one, stop.
- **S3, after U2.** If the read pairs' keep rate is above 448's planned-pair keep rate among pairs it could not register-confirm (29 of 341, 8.5%), or fewer than half the pairs can be register-confirmed, nothing merges except register-confirmed pairs.
- **S4, after the capped wet run.** One wrong merge in the ledger read halts the full run: post a keep and unwind by ADR 0003:59-61.
- **S5, after the fold.** If a merged register pair reappears as `both_distinct`, or alias refusals exceed binds, halt further wet runs; that is an alias defect.
- **S6, ordering.** U4 and U5 do not start until U1's verdicts are posted.

## What would change this decision

- If `ppon_beside_coh` dominates CHC and MPR, ship UKPRN alone and settle issue T first.
- If the 239-pair `no_target_ppon` pool turns into splits over repeated dry runs, or the losers carry heavy mention mass (Leeds has 398 mentions on its PPON org against 4 on its NHS org), build sooner and file prevention.
- If a register fetcher becomes unavailable, its scheme drops to verdict-only under S3, or out of scope.
- If U1 shows the company-number arm's 747 is a gate regression, fix 448 first.
- For NHS: if the 73 multi-target codes are different bodies, letter-only codes are not minted. If the ODS API confirms they are org-level codes, issue N goes ahead and NHS joins the arm.