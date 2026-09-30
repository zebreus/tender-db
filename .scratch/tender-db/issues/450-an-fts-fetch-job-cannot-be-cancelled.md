# 450 — an FTS fetch job cannot be cancelled, so a misbehaving API holds the single job runner until a deploy

Status: needs-info — DEPLOYED 2026-09-30 ~02:27 UTC (rev `7f24c30`, health green, 0 error lines). The first half of the Verify reads done. The live half, a cancel on a running FTS fetch, needs a month that is re-fetched anyway; the next such month is any throttled month of the backfill (342).
Was status: ready-for-agent — BUILT 2026-09-30 (`94fcbae`, gated green; two review tweaks after it: the stop is read after the page pause, and one comment's edge case is corrected). A two-lens adversarial review (resume correctness; the cancel contract) found no defect. Deploys with 448 unit 1 when the box queue is idle; the Verify's live half waits for a month that is re-fetched anyway.
Was status: ready-for-agent — filed 2026-09-30 02:0x UTC from issue 449's "Not fixed here", which named the follow-up
and did not file it.
Kind: operations (job control)
Relates to: 449 (the stuck cursor that needed a `FORCE_BUSY=1` deploy to stop), 252 (the honest-cancel contract:
`STOPPABLE_KINDS`), 406 (the same gap on `process`, closed the same way), 342 (the FTS backfill in chunks)

## What is wrong

`POST /admin/jobs/{id}/cancel` on a running `fetch` answers 409 "no stop checkpoint". For TED and DÖE that is
honest and harmless: their fetch is one download. An FTS fetch is not. It is a paced walk of the paged API, one
request per page, and a month is ~150 requests in the normal case and up to 31 × (50 + 26 × 20) ≈ 17,700 under
449's hourly fallback, at the API's pace. On 2026-09-30 job 1676 looped on a stuck cursor in front of the daily
tick. The only way to end it was to build and deploy a code fix with `FORCE_BUSY=1`. A restart alone does not stop
it either: recovery re-runs the in-flight job from the front of the durable queue.

The backfill (342) is now months of these jobs, fetched in chunks through idle windows. Any new API anomaly that
the 449 guard does not catch (throttling that never clears, a slow endpoint, a cursor that advances through junk)
has the same single exit: a deploy.

## What to build

1. `ingest::fetch::fetch_fts` takes a `stop` closure and reads it before every request, in the day walk and in
   the hourly fallback. A stopped walk returns `Outcome::Stopped`. Nothing lands, and the staging directory and
   `cursor.json` are kept exactly as a crash would leave them, so re-enqueueing the same fetch resumes where it
   stopped. The cursor is already written after every page.
2. The supervisor passes `|| self.cancelled(job.id)`. The `Fetch` arm reports a stopped walk as
   `CANCELLED at a checkpoint — …`, the prefix a cancelled `project` uses.
3. `cancel` decides stoppability from the kind AND its params: a `fetch` whose params start `fts ` is stoppable.
   A TED/DÖE fetch still answers 409, because nothing in a single download reads the flag.
4. Tests:
   - a stopped walk lands nothing, keeps the cursor, and a second call resumes without re-asking the pages it holds;
   - `stoppable` holds for the params both enqueue paths produce, and not for a TED fetch.
5. Docs: `docs/operations.md`'s cancel table and the FTS backfill paragraph.

## Verify

    grep -n 'kind == "fetch"' crates/app/src/supervisor.rs

- **done**: the line exists, and on prod, a `cancel` on a running FTS fetch answers `{"state":"stopping"}`. The job
  row then ends `ok` with `CANCELLED at a checkpoint`, and re-enqueueing the month resumes from its staged pages.
  (Run this only on a month that would be re-fetched anyway.)
- **open**: `cancel` on a running FTS fetch answers 409.

## Built 2026-09-30

- `fetch_fts(…, stop, on_progress)` reads `stop()` right before every request, after the page pause, in both loops.
  `Outcome::Stopped` returns before anything lands. Staging and the cursor are exactly as a crash would leave them.
- `Supervisor::run_fetch_fts` passes `|| self.cancelled(job_id)`. The `Fetch` arm answers
  `CANCELLED at a checkpoint — fts monthly <M>: nothing landed; …`. That is an `ok` row, and the durable row is
  dropped like any concluded job's, so a restart does not re-run it.
- `stoppable(kind, params)` answers for `cancel`. A TED/DÖE fetch and the FTS daily probe (kind `probe`) still get 409.
- Tests:
  - `a_stopped_fts_walk_lands_nothing_and_the_next_fetch_resumes_it` (ingest): one page, then the stop. Nothing
    registered, the cursor names page 2, and the re-run asks page 2 only and lands every release;
  - `a_running_fts_fetch_can_be_cancelled_and_a_single_download_cannot` (supervisor): the params of both enqueue paths,
    plus cancel on ted/doe/fts fetches.
- Review notes, not defects:
  - re-enqueueing a stopped *refetch* of a registered month as a plain fetch answers `Unchanged` and leaves the
    staging in place, the same as after a crash or a throttle error; re-enqueue it with `refetch: true`;
  - `TENDER_DROP_JOBS` plus a restart was the other existing exit, besides a deploy.
