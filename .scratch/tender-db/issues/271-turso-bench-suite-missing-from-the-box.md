# 271 — the D1 turso bench/probe suite is gone from the box

Status: needs-triage — filed 2026-08-23 (owner), found preparing issue 166's reprobe
Kind: operability (a documented gate cannot run)
Blocked by: —
Relates to: 166 (whose D1 reprobe step this blocks), 224 (the box-rebuild loss class — same
failure: on-box-only artifacts do not survive), 269 (same discovery family)

## What

`docs/research/turso-scale.md` §D1 pins turso bumps to a reprobe: "re-run
`/opt/tender-db/turso-bench/` (probes + crash loop at minimum)". That path does not exist on
the box (`/opt/tender-db/` holds only app/app-result/deploy.lock/deployed-rev/repo.git/src),
and the repo has no `bench/` crate — the doc says the bench + raw outputs "live on the VPS",
i.e. they were on-box-only and were lost, presumably in the same 2026-08-09 box rebuild that
took the watchdog scripts (issue 224's lesson: anything living only on the box dies with it).

So the 166 bump's step-1 gate is currently unrunnable as written. 166 is not blocked on it
substantively — the exposure audit found none of 0.7.1/0.7.2's fixes load-bearing, all local
suites + EQP plan gates are green, and the view-planning probe pins the planner — but the
protocol now references a suite that does not exist, which is exactly how gates rot.

## Fix

Either restore or retire, explicitly:

1. **Restore**: recreate the bench crate IN THE REPO (issue 224's rule) — the probe suite and
   kill-9 crash loop per turso-scale.md's description, built by `ops/check.sh`-style script,
   deployed to the box like the watchdogs (versioned + installed). Then 166 runs it and the D1
   doc points at the repo path.
2. **Retire**: amend turso-scale.md §D1 to the gates that actually exist now (local suites,
   EQP plan-pin tests, view-pushdown probe, kill-9 covered by the store's crash tests if it
   is), and record why the on-box bench is not being rebuilt.

Either is fine; the doc and reality must agree. Owner's lean: (1) is a day of work re-deriving
the suite from turso-scale.md's measurement descriptions; (2) is honest if the EQP gates are
judged to cover the planner-regression risk the bench existed for. Decide when 166 deploys.
