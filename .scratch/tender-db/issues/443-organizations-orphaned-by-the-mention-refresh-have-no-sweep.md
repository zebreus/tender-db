# 443 — organizations left with no mention by 434's refresh have no sweep: they stay listed, searchable and counted

Status: ready-for-agent — filed 2026-09-28 from the first fold with 434's refresh (job 1610, stopped in planning
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
