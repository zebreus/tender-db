# 191 — the dashboard change-gate is blind to in-place job writes: fresh `measured_at`, hours-old heavy sections

Status: RESOLVED (2026-08-12, orchestrator) — concluded-job count folded into the change-gate watermark; regression test in the gate suite
Kind: correctness (observability surface)
Relates to: 139 (cost a morning: a completed reclaim read as a failed one), 61 (the change-gate this extends), 53 (the heavy-write gate, still correct)

## What happened (measured, 2026-08-12)

Reprocess job 612 stamped 1,898 quarantine rows reclaimed between 08:56 and 09:05 CEST. A
dashboard read at 09:37 carried `measured_at = 09:37` yet showed the quarantine panel exactly
as the DB stood at 08:55 — job 612's stamps, committed half an hour earlier, invisible. The
next service restart made the same endpoint serve the converged numbers immediately. During
issue 139's diagnosis this staleness was indistinguishable from "the stamps never landed", and
it sent the investigation chasing a phantom third defect for a morning.

## Root cause (revised from the initial filing)

Not a pinned turso read snapshot. The issue-61 change-gate skips the heavy dashboard sections
(quarantine, coverage, counts, award-linkage) whenever its watermark — change-log cursor,
newest fetch instant, newest notice instant — is unchanged since the last heavy measure. A
reprocess/reclaim job writes IN PLACE: it stamps `reprocessed_at`/`skipped_at`, rewrites
reasons, and flips `parse_state` — no new notice id, no fetch row, no change-log cursor
movement until the *fold* later emits versions. So the watermark read "nothing happened", the
heavy sections were never re-measured, and only the cheap `system` section (measured every
pass) kept refreshing `measured_at` — fresh timestamp, stale data. The initial "pinned reader"
theory also mispredicted the WAL `busy=true` fold logs; those showed `wal_frames=0` (nothing
to reclaim) and were noise.

## Fix (landed with this issue)

`Supervisor` counts concluded jobs (ok or error) in an atomic; the coverage refresher folds
that count into the `HeavyKey` watermark. Any finished job now forces one heavy re-measure on
the next 60s tick, while the idle-server property stands (no jobs, no writes → no rescans) and
the issue-53 heavy-write gate is untouched. Regression case added to
`the_change_gate_skips_when_unchanged_and_runs_after_a_write`: an unchanged DB with a bumped
job count must re-measure.

Residual, deliberately not pursued: mid-job the panel stays gated (issue 53) and job progress
is visible live from the supervisor section — nobody needs mid-reprocess quarantine counts.
