# 192 — the incremental fold's plan phase is corpus-proportional, not change-proportional

Status: needs-triage
Kind: performance observation (projection)
Blocked by: —
Relates to: 46/ADR-0009 (the plan/fold design this measures), 179 (epoch cost accounting), 61 (cold-scan I/O competes with ingestion)

## Observation (measured, 2026-08-12)

A `project rebuild=false` with ANY non-empty change set walks the ENTIRE notices table in its
plan phase — 14.25M rows — regardless of change-set size:

- job 613: 1,898 reclaimed notices → plan walk 14.25M, 2h01m total (cold cache post-restart)
- job 2/624: 3,352 reclaimed notices → same 14.25M walk, ~50 min (warmer)
- job 618 (daily, 4,212 notices): 132s total — warm cache makes the same walk cheap
- jobs 611/620 (0 notices): <1s — the walk only runs when something is unprojected

So the fold's cost is `O(corpus)` I/O per non-empty run, dominated by cache temperature: warm
it hides inside two minutes, cold (after every deploy restart) it is one to two HOURS during
which the job queue is blocked and the dashboard's heavy sections are gated. Three deploys
today each paid it.

## Why it matters (and why it may be fine)

The design (issue 46) plans islands over the full corpus because a reclaimed notice can extend
version chains anywhere; the plan phase is where that global reasoning lives. The daily cadence
(one fold per day, warm) pays ~2 minutes and does not care. It bites only when deploys and
reclaim campaigns interleave — an operations pattern, not a user-facing one.

## What (when someone owns it)

1. Measure what fraction of the plan walk is avoidable: an indexed seek over `projected = 0`
   notices plus island-local expansion may bound planning to the change set's neighbourhoods.
2. If the global walk is load-bearing (cross-corpus island merges), document that in ADR-0009
   and close this as by-design with the cost stated.
3. Cheap interim: after a deploy restart, a page-cache warmup read of the notices table before
   the first fold would turn the 2h cold case back into the 2-minute warm case.

**2026-08-13 data point (orchestrator):** job 630 — 2,266 reclaimed notices, fold running ~3h
(plan walk cold after the b97decb deploy restart, write phase reached ~06:5x CEST). Every
reclaim round in a campaign now pays a multi-hour fold; with issue 194's two-pass drain that is
two fold-hours for ~2.5k notices. Raises the priority of measuring option 1 (change-set-bounded
planning) or at least option 3 (post-deploy cache warmup).

**2026-08-14 (orchestrator) — data point.** The 108-notice issue-200 reclaim (2010-era
records) triggered the full Buckets sweep again: plan-checkpoint chunks ~75 min, then the
sharded pre-pass at ~250 notices/s/shard (100% spill — by design for the full strategy),
daily queued behind it, ~2h+ total. Same shape as the 4-notice 1999 COR wave (job 655,
2h03). Old-era reclaims reliably fall back to the full sweep while recent-era reclaims fold
in seconds — when this issue is picked up, that watermark/fallback is the lever. Frequency
argument for leaving it: the old-era waves are one-offs and now largely done.
