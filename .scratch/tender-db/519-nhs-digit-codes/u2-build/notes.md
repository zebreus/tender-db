# 519 unit 2, phase A: build notes for U1 and U2 (2026-10-12)

Scope: the decision's §7 U1 (generator, children TSV, `nhs_ods.rs`, store types) and U2 (the census,
supervisor wiring, census tests). U3 (adjudication and cohort) and U4 (fold, alias, unwind) are not built
here. Nothing is committed. `ops/check.sh` was not run, and nothing touched prod.

**This change is U1+U2 ONLY: a DRY deploy.** The brief named the alias, unwind, survivor and ledger code
to extend; none of it is touched (no hunk in `arm_nhs_body_alias_on`, `nhs_body_alias_bind`,
`unwind_nhs_body`, `nhs_body_survivor`, `NhsBodyUnwindArgs`, `NHS_BODY_ALIAS_LEDGER_SQL`,
`NhsBodyAliasCounts`, `AltIdAliasRules`, `nhs_body_ledger_listing`; no Spec `cohort`). U4 — decision
§3.2-§3.7 (identity-loser fold and evidence, the exact-triple alias arm, the cohort-scoped LIFO unwind,
S4's `identity`/`identity-stray` labels) and its §6 alias, unwind, S4, wet and canary tests — is a
SEPARATE deploy that must land before §5 run-order step 6. Until then the `child-fold-unbuilt` hold
(deviation 2) holds every phase-A body on every wet run once U3 posts `nhs-body-children-<date>`: the S5
canary (`triple_bound` > 0) cannot pass and the S6 dry scoped unwind cannot run (no `cohort` argument).
§5 steps 1-5 (measure, this dry deploy and its census, rehome, re-census and post, dry run) can. U4 removes
the hold and its test (`a_child_member_is_held_out_of_the_wet_run_until_the_identity_fold_exists`), never before. 513's alias, fold, unwind and S4
paths are byte-for-byte untouched; the hold runs before `nhs_body_survivor` and any write.

**Commit list** (named files; `git show --stat` must list each): `crates/ingest/sdk/nhs-ods-children.tsv`
(UNTRACKED and `include_str!`-ed by `nhs_ods.rs`: left out, the flake's git-fetched source has no such
file and `nix build` fails on main), `crates/ingest/src/nhs_ods.rs`, `crates/store/src/canonical.rs`,
`crates/store/src/lib.rs`, `crates/store/tests/nhs_body.rs`, `crates/app/src/supervisor.rs`,
`docs/operations.md`, and the scratch: `.scratch/tender-db/513-nhs/u1-table/gen_nhs_ods_table.py`,
`.scratch/tender-db/519-nhs-digit-codes/u2-build/`, `…/measure/child_check_sim.py`,
`…/measure/m1_score.py`, `…/unit2-decision.md` (§1.3 amended), and the issue file.

## Commands and EXIT values (gate flags and package set, filters after `--`)

| run | EXIT |
|---|---|
| U1: `… -- nhs_ods` (ingest lib 36 passed, 2 ignored) | 0 |
| U1: the ignored harnesses `--lib -- nhs_ods_ --ignored` with `NHS_ODS_M1` and `NHS_ODS_ADJUDICATION_ROWS` set: the M1 child_check tally has 227 asks and 0 differences from the Python port; the 513 tally is 233/235 SAME registry, 116/116 SAME PPON, 0 DIFFERENT/NOT_ORG admitted | 0 |
| U2: `--no-run` (every test binary compiles; only pre-existing warnings) | 0 |
| U2: the 60 nhs_body test names + `nhs_ods` + `nhs_body` + `an_execute_without_an_expected_count_is_refused` (nhs_body 60/60, ingest 36, store lib 1, tender-db lib 5) | 0 (run twice, the second after the line wraps) |
| Neighbours: `--exact` over the test names of altid_merge (81), identifier_verdicts (9), merged_org (7) and rekey (9) | 0 |
| Review fixes: the 61 nhs_body test names + `nhs_ods` + `an_execute_without_an_expected_count_is_refused` (nhs_body 61/61, ingest lib 37 passed + 2 ignored, tender-db lib 1) | 0 |
| Review fixes: `--lib -- nhs_ods_ --ignored` with `NHS_ODS_M1` and `NHS_ODS_ADJUDICATION_ROWS`: M1 227 asks, 0 differ; 513 tally 233 SAME registry / 116 SAME PPON admitted, unchanged | 0 |

## Generator

- `gen_nhs_ods_table.py --offline`: `nhs-ods.tsv` is byte-identical (sha256 348fbad5…e994). It writes
  `nhs-ods-children.tsv` with 410 rows: 223 RE5, 162 `rel -`, 25 RE6. That is row-for-row equal to
  `measure/child-map.json` (code, parent, from, to, rel). There were 0 network requests.
- 369 child records were seeded into `513-nhs/u1-table/raw/` (untracked) from `measure/ods-ro98/` and `ods/`.

## Deviations (and why)

1. **A sibling file, not a parents column in `nhs-ods.tsv`.** The brief said to "emit the dated parents
   column … bump the version". The adopted decision rejects that (§0 flaw 1, §2.1): a 10th column changes
   513's parse, its VERSION stamp and the rival set. So `nhs-ods.tsv` and `VERSION` stay; the new file
   carries `CHILDREN_VERSION = ods-children-2026-10-12`. The generator exits if `nhs-ods.tsv` would change
   (`--rewrite-main` lets a deliberate re-snapshot through).
2. **A wet hold, not in the decision:** `merge_nhs_bodies` holds any body whose verdict names a child
   member (`child-fold-unbuilt`; nothing written, left in the residual). U2 is a dry deploy, and the fold
   without U4 would arm the child's head name in 513's name alias. U4 removes the hold.
3. **E5 is per distinct RAW name**, not per name_norm: `child_check` reads the raw name. The new cap is
   `NhsBodyCaps::child_names` = 512 (M1's maximum is 12).
4. **`keep_over_applied`, `child_unanchored` and `child_no_parent` are listings** (their length is the
   count). `child_unanchored` sees only three-character tails. A five-character child whose parents are all
   off the allowlist or unlisted is invisible (no harvest tail).
5. **Extra args:** `NhsBodyArgs.child` (for off-allowlist tails' parents) and `children_version`.
6. **`child-triple-shape` is listed only when 513's `minted-identity` does not already explain the
   holder.** A foreign identity reads `minted-identity`; a PPON child reads `child-triple-shape` +
   `child-carries-ppon`.

## Predicted child listing on prod (`predict_child_census.py` → `predicted-child-census.json`)

These are offline predictions from m1/M2/M3 and the closing 513 plan's 174 listed codes; they are not
read from prod.
- **Listed:** 61 child-holder entries, 60 orgs (D4U1Y under QNQ and QRL), 1,812 mentions, under 42 codes,
  57 (code, child) pairs. QHM also lists `00N` (5 parties), whose only holder is the survivor 31556600, an
  own holder.
- **Denied:** 11 `child-name-mixed` and 9 `child-name-refused`, with no E3/E4 denial (per M3). That leaves
  41 eligible entries.
- **PPON-survivor parents (phase A):** 42 entries, 30 eligible. That is 19 clean, 5 zero-only (the two
  "Place" orgs to be left out by the reviewer) and 6 sites. Denied: the 6 mixed holders (13T, 26A, 15E,
  15C, 15N, 93C) and 6 refused (31615636 Cumbira, 03Q, D4U1Y under QRL, X7V4Q, RNZ91, RTR54).
- **Anchor parents:** 19 entries. A verdict naming one of these is `child-into-anchor`.
- **`child_unanchored`:** 00X, 01A, 03N, 03W, 06K, 06N, 06Q, 10Q, 15A, 15M.
- **`child_no_parent`:** 14J, 14N.
- **Invisible:** 12 five-character sites under off-allowlist or unlisted trusts.
- **After U3's rehomes:** each rehome target becomes a child holder of the code it was published under
  (30929531 and Y63 31541949 under QHM; RM1/RGP under QMM; …). Each is denied, and each must be in
  `reviewed_holders`.

## Review fixes (2026-10-12)

1. **`child_check` dropped every Active RO98 row from its rivals.** Confirmed: 58 of the 165 Active RO98
   rows are not "<ICB> - <code>" relabels (NHS England's commissioning hubs 13Q-14T, 15L, 27T, 32T, 76A,
   85J, 97T; the legacy CCGs 00W, 12A, 13P; 32 London "<borough> - SUB-ICB REPORTING ENTITY" rows), and
   `child_check("QHM", "Cumbria and North East Commissioning Hub")` admitted the 13X hub as NENC. Now
   only a `relabel` row (Active RO98 whose name ends " - <own code>": 107 rows) is dropped. Pinned:
   the hub reads `better{13X}`, 13P's CrossCity CCG `better{13P}` under QHL, B2M3M's names still admitted
   (`child_check_drops_only_the_relabel_rows_from_its_rivals`, `child_check_answers_the_decisions_table`).
   Mirrored in `measure/child_check_sim.py` (`RELABELS`) and `m1_score.py`'s `best_body`: M1's 227 asks
   change 0 answers; `best_body` moves only the two NHS England names (RCC → 85J), both `nhse` and called
   before it runs, so no M1 classification moves and `m1.json` stands. `predict_child_census.py` re-run
   under the narrowed port: `predicted-child-census.json` byte-identical. Decision §1.3 amended.
2. **U4 not built.** Confirmed; recorded above, in the issue's Status and in `docs/operations.md`. The hold
   and its test stay.
3. **`keep_over_applied` missed a keep re-posted in place.** Confirmed: `record_merge_verdicts` upserts
   `action` and never clears `applied_at`, so the probe's `action = 'merge'` found 0 rows while the alias
   (`action = 'keep'` on any row) disarmed the code. The probe now reads the applied stamp alone (only the
   fold stamps an nhs-body row; still a PK seek). Test: `a_keep_reposted_over_the_applied_cohort_is_counted`.
4. **The children TSV is untracked.** Confirmed (`git status`: `??`). It is first in the commit list above.
