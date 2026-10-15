# 542 — `backfill-org-name-variants` cannot be stopped, and a restart re-walks it from notice 0

Status: ready-for-agent — filed 2026-10-14 23:5x UTC. Unit 1 gated green (1239 s) and on main at `66f1e63`; next:
deploy it with 534 when the queue is idle, then unit 2.
Kind: operations (job control)
Relates to:
- 307 (the backfill itself);
- 440 and 522 (the dissolve runbook enqueues it after a dissolve);
- 252 (cancel refuses a kind with no checkpoint);
- 32 (the durable `process` resume cursor).

## Verify

    /root/aj.sh /admin/jobs '{"kind":"backfill-org-name-variants"}'   # only in a quiet window
    /root/aj.sh /admin/jobs/<id>/cancel '{}'

- **done**: the cancel answers `stopping`. The job ends at its next window with
  `backfill-org-name-variants STOPPED by cancel after notice id N`. While it runs, `/admin/jobs` shows
  `notice id N` in the phase detail.
- **open (2026-10-14)**: job 2491's cancel answered 409: `job 2491 is running as kind "backfill-org-name-variants",
  which has no stop checkpoint`.

## What was found

522's runbook (440's sequence) ends with a whole-corpus `backfill-org-name-variants`, to re-create name variants for
the mentions a dissolve re-bound. On 2026-10-14 that was 7 mentions. The walk:
- runs at ~1,336 notices/s (6.73M notices in 84 min, job 2491). The whole corpus is many hours: it was still walking
  when this was filed, and it held the queue ahead of 2492 (org-head-drift), 2493 (name-phrase-census) and the 07:35
  dailies;
- had **no stop checkpoint**, so `cancel` answered 409;
- had **no resume cursor**. A service restart (a deploy, or `TENDER_DROP_JOBS`'s escape) recovers the job at the
  FRONT of the queue (`an_interrupted_running_job_is_recovered_at_the_front`) and re-walks from notice 0. So a restart
  cost the hours already walked and did not free the queue. Only `TENDER_DROP_JOBS=<id>` plus a restart ended it.

## Units

1. **Stop and resume (built 2026-10-14).** In `supervisor.rs`'s `Spec::BackfillOrgNameVariants` arm:
   - read `self.cancelled(job.id)` before every notice window, and add the kind to `STOPPABLE_KINDS` and its contract
     test. Each window is its own REPLACE commit, so a stop leaves nothing half-written.
   - after every window, write the notice watermark with `record_job_progress`. A recovered job resumes after
     `job.resume_after`; a fresh enqueue still starts at 0.
   - put the watermark in the phase detail and in the stop and finish lines.
   - Test: `the_org_name_variant_backfill_stops_before_its_next_window` covers the stop, another job's flag, and the
     resume. `docs/operations.md`'s cancel block names the kind and the `TENDER_DROP_JOBS` escape.
   - `Spec::BackfillOrgNameVariants` stays a unit variant, so durable rows from the old build still deserialize.
2. **Scope the runbook step.** A dissolve re-binds a handful of mentions; re-walking every notice for them is the
   wrong size. Either:
   - (a) the dissolve writes the re-bound mentions' variants itself, in the transaction that re-binds them. The
     resolver already writes variants for newly recorded mentions (307);
   - (b) or the job takes `{"notice_ids":[…]}` (capped like `refold-notices`), and the dissolve's summary names the
     notices it touched.

   Prefer (a): it removes the step from 440's and 522's runbooks. Then fix both runbooks. Until then, run the full
   walk only in a quiet window, and stop it with `cancel` if the dailies are close.
3. **Re-create the variants for 522's 7 mentions** once unit 2 lands, or with a full walk in a quiet window. Only the
   cross-language matcher reads `organization_names`, so nothing served is wrong meanwhile. The rows are the fresh
   provisionals 31966280 (`Individual`) and 31966281 (DOC-Labor GmbH), and 2123088 (Hiscox, reused).

## Notes

- 2026-10-14 23:50 UTC: 2491's cancel answered 409. A `max(id)` read of `notices` to size the remainder answered 408
  (not retried). 2491 was left to run, because a restart would only have re-walked it from 0 at the front of the
  queue.
- 2026-10-15 00:20 UTC: unit 1 on main (`66f1e63`, gate green). A deploy over the running 2491 (`FORCE_BUSY=1`, so the
  restart would bring 2491 back on the stoppable build to be cancelled) was refused by the session's permission
  classifier. 2491 runs to the end, at ~1,070 notices/s with 8.56M walked, and the deploy waits for an idle queue.
  2492, 2493 and the 07:35 dailies queue behind it.
- The sibling audit (every other non-stoppable kind, adversarially verified) is filed as issue 543.
