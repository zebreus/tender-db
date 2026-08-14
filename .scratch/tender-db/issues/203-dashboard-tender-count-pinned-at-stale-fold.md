# 203 — dashboard shows two tender counts; one pinned at a five-folds-old value

Status: needs-triage
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
