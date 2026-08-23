# 272 — two enqueue-arm operator traps: reparse "run again to continue" and data-quality's dry-run default

Status: needs-triage — filed 2026-08-23 (owner), both tripped live during the slice-9 sweep close-out
Kind: operability (the queue does what you said, not what you meant)
Relates to: 244 (the sweep it bit), 230 (data-quality's confirmed flag), 247 (job cancel/resume work)

## What happened, concretely

1. **A capped reparse's completion message promises "run again to continue", but a NEW
   enqueue does not continue.** Resume rides on the durable JOB ROW's `resume_after`
   (supervisor.rs, `run_reparse`: "real progress wins over the requested floor"), and
   `reparse_packages` is a stateless floor query. Re-running the SAME row resumes;
   enqueueing a fresh row with identical params starts at the floor again. Tonight job
   358 (fresh row, `after:228, packages:12`) re-walked the era's first 12 packages
   (303k notices, "161 held back") instead of the 17-package tail the chain's last
   tranche reported. Recovered by replacing the capped tail tranches with one UNCAPPED
   `after:228` run (363) — idempotent brute force over floor archaeology.

2. **`data-quality`'s enqueue arm defaults to dry-run.** The arm maps
   `confirmed = !req.dry_run.unwrap_or(true)` — an enqueue without an explicit
   `{"dry_run":false}` prints the plan and stores nothing. Tonight job 357, queued as
   the post-fold acceptance read, ran as a dry run; the real read had to be re-queued
   (365). The weekly scheduler is immune (it passes `confirmed: true` in the Spec);
   only manual enqueues trip. The default is DELIBERATE (issue 230: "the operator's
   explicit yes") — the trap is that the dry-run result reads like a pass in the
   recent-jobs list unless you notice the word.

## Proposed fixes (small, either/both)

- Make the reparse completion message honest about the mechanism: "…held back by the
  cap — RE-RUN THIS JOB (admin.sh rerun <id>) to continue; a new enqueue starts from
  the floor" — or better, let a fresh reparse row adopt the highest `resume_after` of
  any COMPLETED row with an identical spec (one indexed job_log read at start), which
  makes the message true as written.
- Prefix the dry-run data-quality summary loudly: "DRY RUN (stored nothing) — enqueue
  with {\"dry_run\":false} to measure", so the recent-jobs line cannot read as a pass.

## Acceptance

- A capped reparse chain can be continued by a fresh enqueue (or the message stops
  claiming it can).
- A dry-run data-quality line is unmistakable in `admin.sh queue` output.
