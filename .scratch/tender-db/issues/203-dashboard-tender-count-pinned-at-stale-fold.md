# 203 — dashboard shows two tender counts; one pinned at a five-folds-old value

Status: RESOLVED (2026-08-14 — diagnosed not-a-bug, see bottom)
Kind: dashboard staleness
Relates to: 191 (change-gate), 53 (measure job gate)

## What (2026-08-14, observed across several hours)

The dashboard page carries 7,938,839 (job 678's fold output, later grown further) AND
7,936,786 — exactly job 669's output — simultaneously, including at queue-idle and after
service restarts. One panel's number has not moved through five subsequent folds. Either
a second, separately-sourced stat (updated only on some event that stopped firing) or a
cell the issue-191 change-gate misses. Find which panel renders the stale number
(crates/app/src/coverage.rs measures), whether its refresh condition can fire at all
after incremental folds, and fix or label it ("as of last full fold") honestly.

## Diagnosis (2026-08-14, orchestrator) — not a bug

The "pinned" number never was a gauge. The dashboard renders tender totals in exactly
three places, and all three checked out:

1. **Contents panel** (`canonical_counts`) — live: showed 7 938 868, which equals
   `SELECT COUNT(*) FROM tenders` at the same instant. Not stale.
2. **Pipeline panel** (`tenders_by_source`) — live: doe 669 019 + ted 7 269 849 =
   7 938 868. Same measure pass, same watermark gate. Not stale.
3. **Ingestion → Recent runs** — the job history table renders each run's counts string
   VERBATIM (`RunRow`, ui.rs). Full-sweep folds report cumulative totals
   ("14272042 notices → 7938839 tenders"), so job 669's row keeps quoting its as-of-then
   total (7,936,786) forever — through later folds, queue-idle, and restarts (job_log is
   durable). That is history behaving as history, not a stale panel.

The "two counts simultaneously" observation was the Contents gauge beside old fold rows
in the history table. No refresh condition is broken; journalctl shows zero
"refresh failed, keeping last value" lines since Aug 10. The issue-191 gate works.
No code change: the history rows already sit under a "Recent runs" heading with
job id + params per row.
