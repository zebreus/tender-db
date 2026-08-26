# 281 — heavy_write_in_progress() omits the long batched-write jobs, so the coverage refresher's WAL-pinning scan runs during them

Status: RESOLVED-DEPLOYED (0bdebd2, live on prod rev ae31cbd; board-hygiene sweep 301, 2026-08-27). heavy_write_kind() single source of truth covers every batched writer.
Kind: operational / WAL-growth risk (extends issue 53)
Severity: MEDIUM (bounded to ~one scan per transition by the change-gate)
Relates to: 53 (the 70 GB WAL / 5 n/s collapse this belt exists to prevent), 191 (the change-gate that partly masks the hole), 247/234/228 (the omitted job kinds)
Found by: the 2026-08-26 supervisor review.

## The bug

`heavy_write_in_progress()` (supervisor.rs) matched only
`process | project | reprocess | reindex | refold | refold-fields`. Its consumer,
the coverage refresher, skips its multi-minute table-proportional scans only while
that returns true — the belt whose comment names issue 53's WAL balloon (a live
reader snapshot held across the scan pins the WAL and blocks a job's per-batch
TRUNCATE checkpoint).

But `reparse` (TRUNCATE per package — the longest job, issue 247),
`merge-provisional-orgs` (TRUNCATE per 20k-org batch over ~30M orgs),
`mark-skipped-siblings` (~593k rows), the `backfill-*` walks, and
`refold-notices`/`refold-sections`/`repair-swept-siblings` all do the same
per-batch TRUNCATE yet were absent. At the transition into such a job (the
preceding fold left a fresh watermark), one coverage scan runs concurrently with
the job's early batches and pins the WAL for its full duration, blocking those
TRUNCATEs — WAL grows unchecked for that window.

## Fix (shipped)

Extracted the allowlist to a module-level `heavy_write_kind(&str)` (single source
of truth) covering every batched writer:
`process, project, reprocess, reindex, refold, refold-fields, refold-notices,
refold-sections, reparse, merge-provisional-orgs, mark-skipped-siblings,
repair-swept-siblings, backfill-deadlines, backfill-titles, backfill-org-names,
backfill-legacy-adjacency`. Read-only/trivial kinds (probe, data-quality,
reveal-recheck, register-archive, clear-rebuild-flag) stay off so coverage still
refreshes during them. Rationale recorded in the doc comment: over-inclusion only
costs a stale coverage refresh; under-inclusion is the WAL hazard, so the belt
covers the superset. `heavy_write_in_progress_tracks_the_running_job_kind`
extended to assert every new kind pins and every read-only kind does not.
