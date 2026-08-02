# 65 — surface projection/big-job progress on the dashboard + /admin/jobs

Status: open
Kind: observability / dashboard
Blocked by: —

## Motivation

During the 2026-07-25 full canonical rebuild there was NO way to see progress
from the outside. `/admin/jobs` reported the running `project` job but with all
its progress fields (`packages_done`, `members_done`, `notices`, …) at 0 — those
are wired for fetch/process, not projection. The only signal was the journal
`[project]` lines, and the multi-hour `write_buckets` pre-pass emits NOTHING at
all until the fold pass starts. Operating the rebuild meant sshing in and reading
`/proc/<pid>/io` read_bytes and `du` on the bucket dir to infer how far along it
was. A long, expensive, once-in-a-while job is exactly the one that needs a
progress bar.

## What's missing

- The projection's `on_progress` (Progress::Applying { tenders, total }) goes to
  the journal only; it is not persisted into the job's durable progress row that
  `/admin/jobs` (GET) and the dashboard read.
- No phase model: reset → grouping → **pre-pass (write_buckets)** → fold →
  index-build are invisible. The pre-pass and the end-of-fold index builds (each
  many minutes) show as dead air.
- `/admin/jobs` `current` has no generic "phase + fraction + detail" field for a
  project job.

## Proposal

1. Give a running job a small structured progress record the supervisor updates
   and `/admin/jobs` returns: `{ phase: string, done: i64, total: i64, detail:
   string, updated_at }`. Generic across job kinds (fetch/process/project).
2. Have the projection report each phase into it:
   - reset_tender_layer: "clearing previous layer"
   - build_plan_groups: "grouping" (or "reusing grouping")
   - **write_buckets: "pre-pass" with notices-processed / total** (it already
     loops `parsed_chunk(after_id, N)` — emit after_id-based progress every chunk;
     add a periodic `[project] pre-pass: N/12.39M notices bucketed` log line too,
     so the journal isn't silent for ~2h either).
   - fold pass: "folding" with tenders-applied / total (already computed).
   - index build: "rebuilding indexes" with which index.
3. Dashboard System/Jobs panel: render the running job's phase + a progress bar
   from done/total + detail. Poll the same way the panel already polls health.

## Notes / scope

- Keep it cheap: the progress write must not add per-row overhead — update at the
  existing chunk/batch boundaries only (every 10k-notice chunk / 50k-tender batch),
  not per notice.
- The pre-pass is the worst offender (multi-hour, zero output). Even just the
  periodic journal line in (2) would remove most of the pain; the dashboard bar is
  the fuller fix.
- Relates to issue 61 (projection starves the HTTP runtime): during the fold the
  dashboard/health can be unresponsive anyway, so the progress record should be
  written such that a later GET (once responsive) still reflects the latest phase.
