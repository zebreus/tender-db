# 339 — the full projection path's plan build shows the last pre-pass count for its whole duration

Status: needs-triage (filed 2026-09-02 from a live observation; code not yet read)
Kind: operability / instrument honesty
Relates to: 262 (the same gap on the INCREMENTAL path, fixed there), 304 (the
campaign whose fold surfaced it), 42/53 (why a growing WAL during a silent
stage needs to be tellable from a runaway)

## Observed

Job 608 (the full-path fold behind the issue-304 stage-1 re-parse), 2026-09-02:

- 15:52:09 CEST the journal prints `pre-pass shard 30 DONE` — the last of the
  30 shards. Nothing further from `[project]` for the next ten-plus minutes.
- `/admin/jobs` keeps reporting `phase: pre-pass 8520548 / None, "notices swept
  into buckets"` — a number that stopped moving when the shards finished.
- Meanwhile the WAL went 66 KB → 9.6 GB and free disk 702 → 683 GB.

That WAL is the plan build's single write transaction (14.3M plan rows; the
2026-08-27 epoch refold's runbook noted the free-disk dip "was the plan build"),
so it is expected — but nothing on the job row says so. An operator reading the
job sees a stalled pre-pass count and a WAL growing by gigabytes, which is
exactly the shape of the issue-42/53 reader-pinned runaway. Telling the two
apart today takes the journal and prior knowledge; the job row should do it.

## What issue 262 did for the incremental path

Gave the plan-build stage its own phase record and stop checks. The full path
(the `project rebuild=false` fallback over ≥100k notices, and `rebuild=true`)
evidently goes from the pre-pass straight into its plan build without a
`set_phase`. Same fix, other path: a `planning` phase with a moving count, and
the stop flag read between plan batches so a cancel does not wait for the whole
14.3M-row transaction.

## Not

Not a correctness problem and not urgent — the stage completes. It is the
difference between "hours of a number that does not move" and a phase an
operator can read, on the one job kind that legitimately runs for a day.
