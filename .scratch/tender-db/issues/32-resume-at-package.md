# 32 — Restarted process jobs resume at the last incomplete package

Status: needs-verification

A restarted `process (all)` job re-walks every package from 1993 to find
where it left off. Identity-dedup makes that correct but expensive: the
2026-07-21 bad8dda restart spent ~90 min skip-scanning the 2004-2010 era
(huge bundles) at 0 notices written — user-visible as "stuck at 0.0/s",
and every deploy during a long backfill pays it again.

Fix: the durable queue (issue 21) already persists the job row across
restarts — extend it with a progress cursor (last fully-completed
package period, updated as each package finishes, same write
transaction as the package's job-progress bump). recover() re-enqueues
the job with the cursor; the walker starts from the first package AFTER
it. Correctness: a package is only recorded complete after its last
member committed, so resuming after it never skips anything; the
partially-done package re-runs and dedups (seconds, not hours).
`rebuild`-style full re-walks stay available by enqueuing without a
cursor (a fresh enqueue never inherits one).

Acceptance: kill -9 mid-backfill at package N, restart → job resumes at
N (not package 1), first status line shows the resumed position;
existing recovery tests still green.

## Fix (2026-07-21)

The durable queue row (issue 21) gains a `progress` column — the period of
the last package a process job fully completed. `run_process` updates it after
each `process_package_resilient` returns (all that package's members
committed), so it always trails the running package by at most the one being
worked. On recovery the cursor rides back into the in-memory `Job`
(`resume_after`), and `run_process` skips the period-ordered prefix at or
before it, resuming at the next package.

- store/jobs.rs: `job_queue.progress TEXT` (NULL for a fresh job / non-process
  kinds); `record_job_progress(id, package)`; `pending_jobs` returns it;
  `QueuedJobRow.progress`.
- supervisor.rs: `Job.resume_after` (set from the row on `recover()`, `None` on
  a fresh `push()` — so a rebuild/fresh enqueue never inherits a cursor and
  re-walks fully). `run_process` computes `resume_skip` (the prefix ≤ cursor),
  processes the rest, and records progress per package. Logs the resumed
  position on start.

Correctness: a package is recorded complete only after its members commit, so
resuming after the cursor never skips anything; the interrupted package (period
> cursor) re-runs and dedups (seconds, not the ~90 min re-walk from 1993).
Best-effort cursor write — a failed update costs one re-walk of that package,
never correctness.

Tests: store `job_queue_persists_and_removes` extended (cursor round-trips);
supervisor `resume_skip_skips_the_completed_prefix` (the skip is the ordered
prefix, inclusive of the cursor) and `a_process_job_recovers_its_resume_cursor`
(recovery restores it; a fresh enqueue has none). Existing recovery tests green.

Deploy value: every remaining backfill deploy resumes cheaply instead of
re-walking the 2004–2010 bundles.
