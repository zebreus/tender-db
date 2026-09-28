# 443 — organizations left with no mention by 434's refresh have no sweep: they stay listed, searchable and counted

Status: ready-for-agent — **SWEPT AND VERIFIED 2026-09-28** (see the foot): 1,768,353 orphans deleted, and a fresh dry run reads 0. Left: step 3 (wire it after folds that re-bind), and reading step 4 (see the foot). Was: BUILT 2026-09-28, deployed `d5bf157`. Was: filed 2026-09-28 from the first fold with 434's refresh (job 1610, stopped in planning
after re-binding 4,649,867 mentions, and job 1616, the full fold now running). Measure-first: size the orphaned
cohort after 1616 lands, then build.
mine to take.
Kind: data correctness (org layer), with an API-visible symptom
Relates to: 434 (the refresh that moves mentions off a row; its "Mention-less org sweep: None exists" finding), 435
(~1.05M nameless R2.0.7 provisionals whose mentions now resolve to named rows), 393 (text-era `TXT-CY` country:
country-less stock rows whose mentions re-bind to country-bearing ones), 351 (the p0 echo fold, which merges
country-less duplicates but never deletes an unreferenced row), 365 (`dissolve_condemned`), 300 (merge ledger:
every org delete so far is merge-shaped)

## What is wrong

A recorded mention whose published facts changed is re-resolved and re-bound in place (434). The organization it
used to point at is not touched. When that was its last mention, the row stays in `organizations` with zero mentions:
listed by `/v1/organizations`, found by name-prefix search, counted by the dashboard and the org-merge-health report,
and reachable by id with no tenders. Every org delete in the code is merge-shaped (`merge-provisional-orgs`,
R2/R3/E0, `fold_provisional_plan`, `dissolve_condemned`, `repair-nested-orgs`), so nothing ever removes an
unreferenced row.

Expected size: large. Job 1610 re-bound 4,649,867 mentions in its first 4.13M planned notices, mostly text-era
mentions gaining a country and R2.0.7 mentions gaining a name, so their country-less or nameless stock rows lose
mentions en masse. 434 estimated ~1.05M nameless R2.0.7 provisionals alone.

## What to build (after measuring)

1. **Measure** after the full fold (1616): the provisional rows with no `organization_mentions` row (an anti-join on
   `organization_mentions_org`, walked in id windows so no read exceeds `/v1/sql`'s cap; or a dry job that walks
   it). Split by nameless / country-less / named, and check the non-provisional ones separately. Those should be ~0,
   since an identifier-bearing row's mentions re-resolve to itself.
2. **Sweep job** `sweep-orphan-orgs` (dry default, expected-count parity like the other repairs): delete provisional
   rows with no mention AND no referencing party / bid-party / winner / `organization_names` / verdict row. It must
   be FK-safe without a bracket: issue 442's `organization_names_org` makes the org delete's proof a seek. Record a
   ledger row per deleted id (300 §6's auditability), and never touch a non-provisional row.
3. Wire it after the weekly edge scan, or after any fold whose report shows `mentions_rebound > 0`.

## Verify

After the sweep: the provisional orphan count reads 0 (from step 1's measurement, re-run), and
`/v1/organizations?name=` no longer returns a mention-less provisional row for a sampled rebound name.

- **open**: unmeasured (filed 2026-09-28; the cohort is still being created by job 1616)

## Sampled 2026-09-28 ~17:10 UTC (after 1616, during 1617; three bounded `/v1/sql` windows)

| org id window | rows | provisional | no mention |
| --- | --- | --- | --- |
| 1,000,001–1,020,000 | 20,000 | 20,000 | **19,785** |
| 12,000,001–12,020,000 | 2,464 | 1,974 | 4 |
| 25,000,001–25,020,000 | 6,439 | 6,439 | 2,358 |

The first 2,000 rows of the first window: 1,969 orphans, all nameless, 84 of them country-less, all minted the same
second (`created_at` 1786785861, 2026-08-15), and none named by a party, bid-party or winner row. These are the R2.0.7
nameless provisionals (435) whose mentions 434/439 re-bound to named rows, as predicted. No identifier-bearing orphan
was seen in these windows.

## Built (2026-09-28): `sweep-orphan-orgs`, dry by default

- `Db::sweep_orphan_orgs_batch(batch, after, dry_run, job_id)` (`canonical.rs`) walks `organizations` in id
  windows. One range read of `organization_mentions_org` per window says which ids are still mentioned. A
  provisional orphan then takes three seeks (party, bid-party and winner `(organization_id)` indexes); if any hits,
  it is `referenced` and kept for the fold to move. An orphan named by a review table is `protected` and kept, since
  those tables carry no FK. The review tables are case reviews, re-homing case and target, country verdicts,
  merge-verdict members and name drops, and the keep-set is read on the writer inside every window, so a verdict
  POSTed mid-run is honoured from the next window on. Non-provisional orphans are counted under `identified` and
  never touched.
- Wet, in one transaction per window with foreign keys ON: the row's pre-image goes into the new `org_sweep_log`
  first (name, name_norm, country, created_at, and every variant as JSON `[[lang, name, name_norm], …]`,
  job_id). Then its `organization_names` rows go, then the row, and `organization removed` is published. This
  keeps step 2's ledger rule in its own table: `org_merge_log` needs a keeper, and an orphan has none. 442's
  `organization_names_org` makes the delete's proof a seek. A row something still references fails the window
  rather than being orphaned, and so does a row that is no longer provisional (the ROLLBACK restores its variants).
- It refuses to start while any of its five indexes (`organization_mentions_org`, `organization_names_org` and the
  three party-table `_org`) is missing. All five are deferred, so an unfinished rebuild would otherwise turn every
  window into table walks.
- Parity (432's shape): every run counts first. Dry stores `orphan-org-sweep-plan`. Wet refuses without that plan
  and aborts before writing if its count is outside max(2%, 50) of the plan's `swept`. It then re-records the
  residual after every window that deleted, so a wet run that is stopped, fails or is cut by a restart resumes
  under the same parity. A stop before the first delete leaves the reviewed plan untouched. It is stoppable between
  windows and is a heavy-write kind.
- Tests:
  - `crates/store/tests/orphan_org_sweep.rs`: every keep arm (all six review sources), dry writing nothing,
    windows of 3 across every boundary, FK on before and after, the pre-image rows exact, a second run finding
    nothing, and a verdict recorded between windows protecting its org.
  - `the_orphan_sweep_reads_by_index`: no `Rewind` in any read, and the mention range ends at the window.
  - `the_orphan_sweep_deletes_only_under_a_matching_dry_plan`: a missing index → refused; no plan → refused; a moved
    count → aborted with the plan untouched; a matching plan → runs and re-records.
- Reviewed before commit by a four-lens adversarial workflow (correctness, transactions, performance, contract).
  Its surviving points are all in the list above. Not taken: prepared statements for the per-candidate seeks (a nit;
  the reviewer's estimate is minutes over the whole walk).
- Step 3 (wiring it after folds) waits for the first wet run's timing.

## Ran 2026-09-28 (rev `d5bf157`)

- **Dry, job 1624 (79 s):** 7,367,032 organizations walked; **1,768,353** provisional rows with no recorded
  mention: 1,119,072 nameless, 638,629 country-less and 10,652 named. 30 non-provisional; 0 referenced; 0 under
  review; 588 name variants. So 24 % of the table was orphaned. The 434/439 re-binds emptied the R2.0.7 nameless
  class (435's ~1.05M estimate) and the text-era country-less class (393's `TXT-CY`).
- **Wet, job 1628 (348 s for both walks):** swept **1,768,353 of the 1,768,353 counted**, with 588 name variants,
  each published as `organization removed` and each leaving a pre-image in `org_sweep_log`. Health was green through
  it and the journal clean.
- `SELECT COUNT(*) FROM organizations`: 7,367,032 before, **5,598,679** after — exactly the swept count.
- **Verify, dry job 1629 (9 s):** 5,598,679 walked, **0** provisional orphans. **done.**
- **Verify, second line (read ~19:00 UTC):** `/v1/organizations?name_prefix=` for three names whose mentions were
  re-bound (`sarner`, `monaco digital`, `qatar airways`) serves 15 rows. Every provisional row among them has
  `mentions` ≥ 1, and the only `mentions: 0` rows are the identifier-bearing ones the sweep leaves on purpose
  (23375495, 23506939, 23386361 — see the 30 below). **done.**
- Still open:
  - **Step 3 — BUILT 2026-09-28.** Evidence: the fold after the 1999 re-parse (1632) re-bound 164 mentions and
    left 129 orphans. Now `run_project` queues `Spec::SweepOrphanOrgsAuto { cap: 10_000 }` whenever
    `sweep_after_fold(stopped, mentions_rebound)` holds (not stopped, re-bound > 0), once (`already_pending`). That
    run counts; its own count is the plan. At or under the cap it sweeps in the same job, with the same pre-image
    log, keep-set and residual. Above the cap, an era-scale fold like 1616's 943,695 re-binds, it records the plan,
    writes nothing and says so, leaving the wet run to a person. Tests:
    `a_fold_queues_the_sweep_only_after_it_re_bound_a_mention` and
    `the_fold_queued_sweep_sweeps_under_its_cap_and_only_plans_above_it` (30 seeded rows, cap 10 → plan only, cap
    24 → 24 swept with no plan on file).
  - **The 30 non-provisional orphans.** Read them: identifier-bearing rows whose mentions all moved, or rows a case
    review stripped (the `org_case_reviews` cohort). The sweep never touches them.
  - **The small follow-up is COMMITTED 2026-09-28 (gate GATE-EXIT=0, 784 s)**; it deploys with the next bundle. The
    job summary and phase detail say "non-provisional" instead of "identifier-bearing", and
    `the_orphan_sweep_counts_plans_and_sweeps_real_orphans` runs the dry→wet cycle on 30 seeded rows in the
    supervisor.

## The 30 non-provisional orphans, read 2026-09-28 ~19:00 UTC (8 bounded `/v1/sql` windows of 4M ids, ≤ 0.2 s each)

All 30 were minted at the same second (`created_at` 1786785861, the 2026-08-15 rebuild). None is a review-table
row. They are identity rows whose mentions re-resolved to another row under the live key rules:

- 5 BG rows with 13-digit BULSTAT branch codes (`1310631880291`, `1752013040134`, …).
- 5 ES rows (`B18298679`, `74725508A`, `B21917828`, …).
- 20 rows from outside the EU or with odd keys. Examples: `UK`/vat `UKCOMPANYREGISTER02231841`, `SE`/vat
  `SECPCUIN0023763` (Pakistan), `LI`/vat `LIBRO3137FOLIO0435994NO109809`, the Danish embassies in BF and ML sharing
  `43271911`, and QA, EG, MC, SM, LB, PG, PF, HK, AZ, GE, TZ, ZA, CI, AE.

Two traced through the public API:

- **Monaco Digital SAM** (23506939, `MC`/national `RCI77S01656`, 0 mentions). Row 18370494 has the IDENTICAL
  identity and holds the 4 mentions. The orphan is a duplicate the merge arms never folded (name gate), now empty.
- **Sarner International Ltd** (23375495, `UK`/vat `UKCOMPANYREGISTER02231841`, 0 mentions). Its mentions now sit
  on 31516324: the same raw identifier under country **`CD`** (national, 2 mentions), beside an `NL` identity row
  and a `GB` provisional one. A UK company keyed under the Democratic Republic of the Congo — **traced: source reality, not a parse defect.**
  The notices publish it. Notice 25253580 (00735989-2025, eForms) has `BT-514-Organization-Company` `COD` on
  ORG-0004, and every other party there is `DEU`. The two BG rows under `CD` (31502459, 31503822) come from r209
  notices whose `TED-COUNTRY` on that one ORG section is `CD` beside `BG` everywhere else, e.g. 19581268
  (496753-2017) ORG-2. A publisher's country slip is the wrong-country class issues 355/357's verdict-gated
  country moves handle. No new issue.

**Step 4 (decide):** extend the sweep to non-provisional orphans. They have no evidence and no party rows, like the
provisional ones. The pre-image log makes each deletion restorable, and 30 rows is a trivial blast radius. It is a
small code change (drop the `provisional` arm of the classification, keep the review keep-set) and waits for the
follow-up above to land, since both touch the same test and summary.

