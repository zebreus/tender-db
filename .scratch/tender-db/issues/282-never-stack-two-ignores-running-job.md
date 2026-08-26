# 282 — the scheduler's "never stack two" guards inspect only the queue, not the running job, so a tick during a run enqueues a duplicate

Status: FIXED in working tree (2026-08-26, owner), awaiting gate+deploy
Kind: operational (duplicate scheduled work)
Severity: LOW
Relates to: 280/241 (a wedged/slow running instance widens the duplication window), 274 (the daily reveal-recheck guard is one of the three)
Found by: the 2026-08-26 supervisor review.

## The bug

`spawn_report_scheduler` guarded weekly data-quality with
`if self.queued().iter().any(|j| j.kind == "data-quality")`, same shape for
`rehash-probe` and for `reveal-recheck` on the daily chain (`enqueue_daily`). The
comment states the invariant: "Never stack two: … a second one would double a
36-minute job for one report that gets overwritten anyway." But `queued()` maps
only the queue; a job that has been popped is RUNNING and lives in `current`,
invisible to `queued()`. So a tick firing during the ~40-minute window a prior
instance is executing (e.g. an operator-enqueued run, or one popped just before
the tick from behind a long rebuild) sees an empty queue and pushes a second — the
exact stacking the comment forbids. Under issue 280 (a wedged running instance)
the window widens and every subsequent tick stacks another.

## Fix (shipped)

Added `Supervisor::already_pending(kind)` = "queued OR currently running"
(`current_progress().kind == kind || queued().any(...)`), and routed all three
guards through it. Test `already_pending_sees_the_running_job_not_only_the_queue`
pins that a running instance counts as pending and a different running kind does
not.
