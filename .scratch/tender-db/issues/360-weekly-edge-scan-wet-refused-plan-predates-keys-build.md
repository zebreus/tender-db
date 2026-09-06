# 360 — the weekly `scan-org-match-keys` wet run refuses itself every week: the batch rebuilds the keys first

Status: FIXED, DEPLOYED 2026-09-06 03:3x UTC (895acf6) — the weekly tick now enqueues a dry scan ahead of the wet one (eight jobs; `the_weekly_report_tick_enqueues_its_eight_jobs_once_each`). Verify on Sunday 2026-09-13: the wet scan must run `ok` after the dry. Was: DIAGNOSED 2026-09-06 03:0x UTC — fix is one line of scheduling plus a test update; lands with the next deploy. Filed from the Sunday batch's job 755 (`error`).
Kind: operations / scheduling defect (organization layer, Stage 4 edge store)
Relates to: 300 Stage 4 (the E3 edge scan and tripwire 6), 314 (the edge store's consumer), 346 (the weekly `build-org-match-keys` that joined the batch)

## Observed

Sunday 2026-09-06, the weekly tick's chain ran data-quality (751) → rehash-probe (752) →
build-org-match-keys (753, 97 s, 6.57 M rows) → org-merge-health (754) → **scan-org-match-keys
(755): `error` in 0 s — "org-edge-scan-plan predates the current keys build — rerun the dry
scan and review it".** The stored `org-edge-scan-plan` dates from 2026-08-30 03:36 UTC.

## Cause

The weekly scan is enqueued WET (`Spec::ScanOrgMatchKeys { dry_run: false }`, the tick's
seventh job, pinned by `the_weekly_report_tick_enqueues_its_seven_jobs_once_each`), and its
T4 parity check refuses a wet run whose recorded dry plan carries a `keys_built_at` other
than the current build's. Since the weekly `build-org-match-keys` was added ahead of it in
the same chain, the plan is stale by construction on every tick: the wet scan can never run
from the schedule, and tripwire 6's weekly clock (design: "the weekly wet scan IS tripwire
6's clock") has silently stopped. Last week's plan exists only because an operator ran the
dry scan by hand at 03:36.

## Fix

Enqueue a DRY scan between the build and the wet scan in the weekly chain (eight jobs): the
dry recomputes the plan against the fresh build and records `keys_built_at`; the wet that
follows passes parity. The edges are advisory (never a merge) and the tripwire compares
edge counts before/after, so the automatic dry→wet pair loses no safety that the refusal
was providing — the refusal was protecting against a plan computed on OTHER keys, and the
dry run in the chain is exactly the remedy its own message asks for. Update the tick test
to eight jobs, dry before wet. Until it lands: `tender-admin enqueue scan-org-match-keys`
(dry) then the wet, by hand, after the build.
