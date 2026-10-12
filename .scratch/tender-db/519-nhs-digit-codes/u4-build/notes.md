# 519 unit 2, U4: build notes (2026-10-12)

Scope: decision §3.2-§3.7 and §6 (`../unit2-decision.md`), the U4 deploy. That covers the identity-loser fold,
the exact-triple alias arm, the cohort-scoped unwind with last-in-first-out refusal, S4's `identity` and
`identity-stray` labels, removing U2's `child-fold-unbuilt` hold, and the S5 canary over child-coded notices.

Not done here:
- nothing is committed;
- `ops/check.sh` was not run;
- prod was not touched;
- the issue file's Status is unedited (the parent does it after the gate).

Tests were written first (§6) and then the code. The new tests did not run red before the code existed. So a
mutation run (below) checks that they fail without it.

**Commit list.** Stage these named files. `git show --stat` must list each one and nothing else.
- `crates/store/src/canonical.rs`
- `crates/store/tests/nhs_body.rs`
- `crates/store/tests/altid_merge.rs`
- `crates/app/src/supervisor.rs`
- `crates/ingest/src/project.rs`
- `docs/operations.md`
- this file

## What changed (canonical.rs unless named)

- **Fold (§3.2-§3.4).**
  - `nhs_child_evidence` and `NhsChildEvidence`: a child loser's code, codes, rel and periods.
  - `nhs_body_ledger_evidence` writes `loser_kind: identity` for an identity loser, plus `loser_identifier_kind`.
    Its `loser_name_norm` is `""` and its `loser_name_norms` is `[]`, and the fold's key set is emptied too.
  - Child rows add `child_code`, `child_codes`, `child_rel` and `child_periods`. Every new row says
    `code_recorded`.
  - `repoint_org_references` already writes the identity loser's `GBNHS<C>` on the survivor (rule `nhs-body`,
    loser = the child). So `?identifier=GBNHS<C>` answers the survivor, and nothing new was needed there.
  - Zero-only children fold as name losers, and `GBNHS<00x>` is recorded on the survivor (ruling 9).
  - `nhs_body_survivor`: an identity child is never the anchor. With no other member the result is
    `Err("child-into-anchor")`.
  - `merge_nhs_bodies`: the `child-fold-unbuilt` hold is gone. A wet `child-into-anchor` hold backs up the
    census status.
- **Alias (§3.5).**
  - `NHS_BODY_ALIAS_LEDGER_SQL` gains `loser_kind`, `loser_id` and `loser_identifier_kind` after `at`.
  - `arm_nhs_body_alias_on` diverts identity rows into `NhsBodyAlias.triples` before the name-key check.
    Country-less triples are never armed.
    - A `keep` drops a triple's rows, as it drops name rows.
    - The newest row per (triple, code) is that code's arm. Two codes give two arms, routed by name.
    - Anchor chains are followed. A broken chain poisons the whole triple.
  - `AltIdAliasRules.nhs_child_check` is the gate. `project::altid_alias_rules` sets it to
    `nhs_ods::census_child_check`. When it is `None`, no triple is armed, and the arming logs that.
  - The target half of `nhs_body_alias_bind` is factored into `nhs_body_target_bind`, a shared helper that
    keeps 513's behaviour (513's alias tests pass unchanged).
  - `nhs_body_triple_bind` is the new arm. Its refusals:
    - `triple_refused_withheld`: the child triple is withheld;
    - `triple_refused_name`: no arm admits the name;
    - `triple_refused_ambiguous`: two or more arms admit it;
    - `triple_refused_shared`: the target's triple stands on two or more rows, or is withheld.

    Each refusal falls through to `org_of` and the mint unchanged. A bind records `GBNHS<code>` and `GBNHS<C>`
    once per run and is never cached.
  - Where it runs: `resolve_one_mention` asks it before `org_of`, behind `Box::pin`. It is reached only when the
    mention's literal is in `triple_literals`, a borrowed set probe, so an ordinary identified mention costs no
    clone and no box.
  - Counts on `NhsBodyAliasCounts`: `triple_rows`, `triples`, `triple_arms`, `triple_poisoned`, `triple_asked`,
    `triple_bound`, `triple_minted`, `triple_refused_{name,ambiguous,withheld,shared}` and `triple_recorded`.
  - Logging: an `[issue 519]` arming line, and a closing line at `finish_mention_resolver` (zeros included).
- **Unwind (§3.6).**
  - `NhsBodyUnwindArgs.cohort` filters rows by `$.cohort`.
  - Last in, first out: the unwind refuses, with nothing written, when a later un-retired row stands on the code
    in another cohort, or on the survivor from another code. "Later" is compared by `(at, rowid)`.
  - `NhsBodyUnwindReport` gains `cohort` and `code_kept`.
  - `GBNHS<P>` stays when another cohort of the code stands, or when the cohort's rows say
    `code_recorded: false`.
  - The child literals it returns are dropped off the survivor whatever wrote them: the fold (loser = the child),
    or the arm or a zero-only fold (loser = org).
- **S4 (§3.7).** `nhs_body_ledger_listing` takes `child_check`. When a child literal stands on a row other than
  the live survivor:
  - the label is `identity` if the row's head name, or any of its newest 4,096 mention names, passes
    `child_check(code)`;
  - otherwise it is `identity-stray`.

  The helper is `nhs_regrown_named_as_body`.
- **Supervisor.**
  - `Spec::MatchOrgIdentifiersNhsBodyUnwind` gains `cohort`, marked `#[serde(default)]` so pre-U4 job rows still
    deserialize. `JobRequest` gains `cohort`.
  - `nhs_body_unwind_cohort` refuses an empty `cohort`. `cohort` is refused on every other rule.
  - Params read `code=QMF cohort=…`.
  - The unwind summary names the cohort and says when the code was kept. The canary summary shows the triple
    counts.
- **Docs.** `docs/operations.md` covers:
  - the `[issue 519]` diag lines;
  - the wet row (identity loser, `child-into-anchor`, `code_recorded`, the exact-triple arm, S4's kinds);
  - the unwind row (`cohort`, LIFO, `code_kept`);
  - the canary row (`triple_*`);
  - the child-codes section, with the hold removed and §5 steps 6-8 added.

## Tests (crates/store/tests/nhs_body.rs, §6)

- **Removed:** `a_child_member_is_held_out_of_the_wet_run_until_the_identity_fold_exists`.
- **Wet:**
  - `a_reviewed_child_folds_into_the_ppon_survivor_and_its_code_answers_it`
  - `a_second_cohort_on_an_applied_code_folds_only_the_children`
  - `a_rehomed_stray_leaves_its_holder_foldable` (this runs the real 317 `record_rehoming` and `apply_rehoming`)
  - `a_zero_only_child_folds_by_name_and_records_its_code`
- **Alias:**
  - `the_next_child_coded_mention_named_as_the_body_binds_the_survivor`
  - `a_refused_name_stands_on_its_own_row_and_a_later_admitted_one_still_binds_the_survivor`
  - `the_child_arm_survives_a_rebuild_in_notice_order`
  - `two_arms_route_by_name` (covers name routing, `refused_name` and `refused_ambiguous`)
  - `a_broken_anchor_chain_poisons_the_child_triple`
  - `a_withheld_child_triple_binds_nobody`
  - `a_country_less_child_mention_is_untouched`
  - `no_child_check_arms_no_triple`
  - `a_keep_on_the_code_drops_its_triples`
  - `an_unarmed_resolver_is_byte_identical`, extended with a child identity bound by E0 armed and unarmed
- **Unwind:**
  - `a_cohort_scoped_unwind_returns_the_children_through_e0_and_keeps_the_513_fold`
  - `a_scoped_unwind_under_a_later_cohort_is_refused`
  - `a_code_wide_unwind_still_unwinds_every_cohort`
- **S4:** `a_regrown_child_literal_named_as_the_body_halts_and_a_stray_is_listed`.
- **Canary:** `the_canary_re_resolves_a_folded_child_through_the_triple_arm`.
- **Supervisor:** `an_nhs_body_unwind_takes_a_cohort_and_cohort_is_the_unwinds_only` covers the parse, the empty
  refusal, the other-rule refusal, the params, and a pre-U4 row deserializing.

## Commands and EXIT values

Every run used the gate's flags and package set:
`CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p model -p store -p ingest -p tender-db --features tender-db/server -- …`.
Output was redirected to a file and `EXIT=$?` was echoed.

| run | result | EXIT |
|---|---|---|
| `--no-run` | every test binary compiles; only the existing warnings (process.rs, project.rs, api.rs, status_head_range.rs, turso_temp_db_leak.rs) | 0 |
| the 77 nhs_body names, `--exact` | 77 passed | 0 |
| **Mutation:** hook disabled, LIFO disabled, `code_kept` forced false | 8 U4 tests FAILED, 69 passed (see below); then `canonical.rs` restored and `cmp` confirmed it byte-identical | 101 |
| supervisor (7 named tests) + `nhs_ods` | 7 passed; 37 passed, 2 ignored | 0 |
| `run_spec_futures_stay_inside_their_size_budgets --nocapture` | `run_match_identifiers_spec` 2,720 B of 3,840; `run_nhs_body_canary` 352 of 512; `run_spec` 56 of 128 | 0 |
| neighbours, `--exact` on every test name | altid_merge 81, identifier_verdicts 9, mention_refresh 4, merged_org 7, rekey 9 | 0 |
| after the broken-chain fix and the docs: 78 nhs_body names + neighbours + 7 supervisor + `nhs_ods` | all passed | 0 |
| after the `triple_literals` pre-check: the same set | all passed | 0 |
| after rewording the unwind summary: the 7 supervisor tests | 7 passed | 0 |

The 8 tests that failed under the mutation:
- `a_cohort_scoped_unwind_…`
- `a_refused_name_stands_…`
- `a_scoped_unwind_under_a_later_cohort_…`
- `a_withheld_child_triple_…`
- `the_canary_…_child_…`
- `the_child_arm_survives_…`
- `the_next_child_coded_…`
- `two_arms_route_by_name`

`an_execute_without_an_expected_count_is_refused` (CLAUDE.md's stack messenger) passed in both supervisor runs.

## Deviations (and why)

1. **`code_recorded` on every new ledger row** (extra field). The scoped unwind uses it to decide whether
   `GBNHS<P>` stays.
   - Decision §3.6 item 2 says to keep it "when other rows of the code remain". That is not enough for a 513
     ONE-MEMBER body, which has no ledger row but whose fold recorded `GBNHS<P>`: the scoped child unwind would
     drop the code. The `qmf_body` fixture has this shape, and so may prod's QWU.
   - So the code is kept when another cohort's rows remain, or when the scoped rows say the cohort did not record
     it. A missing field (a pre-U4 row) reads as recorded.
2. **`child_codes` (a list) beside `child_code`.** A zero-only cluster can hold several zero-only tails, and the
   unwind drops each one's `GBNHS<00x>`.
3. **Extra counters:**
   - `triple_rows`: identity rows, which are also counted in `ledger_rows`, so an identity-only ledger still arms;
   - `triple_poisoned`;
   - `triple_refused_shared`: the arm's PPON target is shared or withheld. It mirrors 513's `refused_shared`.
4. **A broken anchor chain poisons the whole child triple.** Dropping just the broken arm could let a sibling arm
   bind a name both would admit, which is looser than the ambiguity refusal. In phase A, identity children fold
   only into PPON survivors, so this is theoretical.
5. **The gate memo is keyed per (code, RAW name)**, not per name_norm. `child_check` reads the raw name (U2
   deviation 3), so this is exact.
6. **LIFO's notion of "later":**
   - It compares `(at, rowid)`, because two folds in one second share `at`.
   - It reads un-retired rows of the same code in other cohorts, plus other codes' rows whose `keep` is a keep of
     the scoped rows (the survivor absorbed another body since).
   - A named cohort with no row of the code, while other cohorts' rows stand, is refused (a one-member fold under
     a later one).
   - A named cohort with no row and nothing else standing takes 513's one-member path.
7. **The `nhs_body.rs` `altid_rules()` sets `nhs_child_check: Some(child_check)`.** The decision said test
   literals set `None`, but the alias tests need the gate. `altid_merge.rs` sets `None`, and 513's alias
   outcomes are unchanged.
8. **Canary:** a new test rather than an extension of 513's, which stays byte-for-byte as the regression.
9. **No new supervisor fn for the unwind arm.** The decision's "about 40 bytes under budget" is stale (the family
   measures 2,720 of 3,840). The arm was already `Box::pin`'d, and it stays in place.
10. **An empty `cohort` is refused at enqueue.** Omitting the field is how every cohort is requested.
11. **Not tested: the wet `child-into-anchor` hold.** The census never admits such a verdict (status
    `child-into-anchor`, from U2's test), so the wet branch cannot be reached without bypassing the census. It
    is a backstop.

## Left for the parent

- Run the gate (`ops/check.sh`).
- Commit the named files.
- Update issue 519's Status.
- Deploy U4.
- Then U3 (cohort `nhs-body-children-<date>`) and §5 steps 6-8, which are now documented in `docs/operations.md`.
  `s3check_children.py` is U5 tooling and is not built here.

## Review fixes (2026-10-12, after the review panel)

Five findings were reported. Three of them (1, 3, 5) are one defect. All were checked before fixing: a mutation
run that put back the old behaviour reproduced each finding's numbers (see below).

- **Findings 1, 3 and 5 were real.** A scoped unwind whose cohort has no standing row on the code fell into
  513's code-wide one-member path, which drops `GBNHS<code>` (rule `nhs-body`) wherever it stands.
  - It happened on a 513 one-member body, which is prod's shape: no ledger row, and only `GBNHS<P>` recorded.
    A retried scoped unwind or a mistyped `cohort` dropped 513's code.
  - On a multi-member body the same request was refused, but with a LIFO message that named 513's EARLIER
    cohort as "later".
  - Deviation 6's last two bullets are withdrawn.
  - **Fix.** In `unwind_nhs_body`, an empty scoped ledger is handled before LIFO and before the one-member
    path. The helper `nhs_body_applied_cohorts` does one primary-key-prefix seek on `org_merge_verdicts`
    (GB, `GB:nhs-body`, code, `applied_at IS NOT NULL`). The outcomes:
    - no applied verdict of the cohort: refused, "never folded <code>";
    - an applied merge with no standing row: refused, "unwound already";
    - the cohort's own one-member fold (`applied_action` starts `recorded GBNHS`): refused when a later
      cohort's rows stand (LIFO by `applied_at`, where a tie reads as later), when an earlier cohort's rows still
      claim the code, or when another one-member cohort also recorded it (use the code-wide unwind). Otherwise
      it takes 513's one-member path.
  - `code_kept` is now computed only on the non-empty branch.
  - `cohort: None` is untouched.
- **Finding 2 was real.** The moves pick mentions by literal and by 234 key, never by cohort.
  - **Fix.** A scoped unwind refuses, nothing written, when any of its rows' literals or keys is also on a
    standing row of the code in another cohort. Because LIFO has already passed, that row is an earlier one.
    The message names each shared literal or key with its cohort, and says to omit `cohort`.
  - Not extended: other codes' rows folded into the same survivor (`elsewhere`). 513's code-wide unwind has the
    same property, and this scope leaves it alone.
- **Finding 4 was real.** The census line counted `identity-stray` as an S4 halt.
  - **Fix.** `nhs_body_summary` now reads `… N keys regrown (S4), M identity-stray listed (no halt)`, where N
    counts only the `name`, `ppon` and `identity` kinds.
  - `docs/operations.md` is reworded in four places: the `[issue 513]` S4 paragraph, the `nhs-body` table row,
    campaign step 4, and child step 8. Each now says "no `name`/`ppon`/`identity` entry" in place of "`[]`".
- **Docs.** The `nhs-body-unwind` row gains both new refusals.

New tests:
- `nhs_body.rs`:
  - fixture `qmf_one_member_folded`: prod's 513 one-member shape, folded for real, so `GBNHSQMF` is recorded
    and the action reads `recorded GBNHSQMF on 40`;
  - `a_retried_or_mistyped_scoped_unwind_keeps_a_one_member_bodys_code`: the typo and the never-folded cohort,
    dry and wet; LIFO under the child cohort; the first scoped unwind; the retry (dry and wet); and then 513's
    own one-member scoped unwind;
  - `a_scoped_unwind_naming_a_cohort_with_no_rows_says_so`: the multi-member message;
  - `a_scoped_unwind_is_refused_when_an_earlier_cohort_folded_the_same_child_literal`: refused dry and wet,
    notice 410 stays on 40, and the code-wide unwind still dissolves both cohorts.
- `unwind_state` now wraps each `group_concat` in `COALESCE`. An empty `org_merge_log` read NULL and panicked.
- `supervisor.rs`: `the_nhs_body_line_counts_identity_strays_apart_from_the_s4_halt`.

| run (gate flags and package set, redirected, `EXIT=$?`) | result | EXIT |
|---|---|---|
| `--no-run` | compiles; only the existing warnings | 0 |
| the 81 nhs_body names, `--test nhs_body --exact` | 81 passed | 0 |
| **Mutation:** the empty-scope branch off, the shared check off, the old `code_kept`; the 3 new tests | 3 FAILED, with the findings' own numbers: `merged_identifiers_dropped: 1, code_kept: false`; "later cohort … 513-u2-test (QMF)"; `moved: 2, by_identity: 2`. `canonical.rs` restored, `cmp` identical | 101 |
| `nhs_body nhs_ods an_execute_without_an_expected_count_is_refused run_spec_futures_stay_inside_their_size_budgets an_interrupted_running_job_is_recovered_at_the_front` | nhs_ods 37 passed (2 ignored); 9 supervisor passed; plus 3 nhs_body-named store tests and the census date test | 0 |
| `--exact` the budget test and `an_execute_without_an_expected_count_is_refused`, `--nocapture` | 2 passed; `run_match_identifiers_spec` 2,720 of 3,840; `run_nhs_body_canary` 360 of 512; `run_spec` 56 of 128 | 0 |

## Second review (fix delta fe9edff..46affde; wf_63c23e7e-315, 3 lenses, each finding verified)

Gate on 46affde was green (GATE-EXIT=0, 1174 s) before these; they were fixed on top and re-gated.

- **Code-wide unwind named by a CHILD code** (verified real, should-fix). A child row carries its parent's
  `$.code`, so `{code: "13T"}` found no row, took 513's one-member path and dropped `GBNHS13T` (or a zero-only
  `GBNHS00P`) off the survivor while the parent's child-cohort row stood. Now `unwind_nhs_body` collects the standing
  rows whose `child_codes` hold `args.code` and refuses, scoped or not, naming `<parent> (<cohort>)`. Test
  `an_unwind_named_by_a_folded_child_code_is_refused` (dry/wet × scoped/unscoped).
- **Runbook step 7 could not pass** (verified real). `max_groups` folds the first admitted codes in code order
  (QGH, QHL, QHM, QJK, QMJ on the predicted census), so QWU is not reached and its child verdict is unapplied.
  Step 6/7 now say so and name "a code in step 6's `folded[]`" (QHM). The empty-scope refusal reads every
  recorded verdict (`nhs_body_cohort_verdicts`, `applied_at` optional) and says `recorded but not folded yet`
  instead of the typo hint. Test `a_scoped_unwind_of_a_code_the_capped_run_has_not_reached_says_so`.
- **The one-member `earlier` and `twins` refusals were untested** (verified real). Test
  `a_scoped_one_member_unwind_refuses_while_another_cohort_claims_the_code` (both fixtures; 513's fold is
  backdated a day, since folds in one second tie and a tie reads as later — LIFO would refuse first).
- **CHANGELOG overstated 513** (verified real): 513 binds later mentions by reviewed NAME, never by the code;
  "each body … is consolidated" became "once a reviewed verdict for it is applied"; a child code "also" answers the
  parent (a refused mention's own row is returned beside it).
- Mutation: each new test fails with its guard disabled (EXIT=101), `canonical.rs` restored and `cmp`-identical.
- **Recorded, not fixed (nits, latent):**
  - No scope covers "this cohort and every later one": two child cohorts folding one literal can only be rolled
    back code-wide, which also dissolves 513's fold. Only an `identity`-kind regrowth can produce the shape.
  - When two body codes share one survivor, `claimed` and the shared-literal check read only the same code's rows;
    another code's fold of the same child triple would lose its mentions. Not reachable today (171 codes → 171
    distinct keeps); extend both over `elsewhere` if a fold ever gives two codes one survivor.
