# 443 — organizations left with no mention by 434's refresh have no sweep: they stay listed, searchable and counted

Status: ready-for-agent — **SWEPT AND VERIFIED 2026-09-28** (see the foot): 1,768,353 orphans deleted, and a fresh dry run reads 0. Left: step 3 (wire it after folds that re-bind), reading the 30 non-provisional orphans, and one small follow-up commit (see the foot). Was: BUILT 2026-09-28, deployed `d5bf157`. It
394 refold fold) IS step 1's measurement. Was: filed 2026-09-28 from the first fold with 434's refresh (job 1610, stopped in planning
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
- Still open:
  - **Step 3.** Nothing runs the sweep by itself yet. Each fold that re-binds (`mentions_rebound > 0`) makes more
    orphans. Wire it after the incremental fold or on the weekly tick.
  - **The 30 non-provisional orphans.** Read them: identifier-bearing rows whose mentions all moved, or rows a case
    review stripped (the `org_case_reviews` cohort). The sweep never touches them.
  - **A small follow-up is built locally, not committed.** The job summary and phase detail say "non-provisional"
    instead of "identifier-bearing". `the_orphan_sweep_counts_plans_and_sweeps_real_orphans` runs the dry→wet cycle
    on 30 seeded rows in the supervisor. Its gate ran, but its result could not be read this session, so it waits
    for the next gate run.

