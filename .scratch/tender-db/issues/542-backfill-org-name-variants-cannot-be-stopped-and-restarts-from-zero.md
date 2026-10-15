# 542 — `backfill-org-name-variants` cannot be stopped, and a restart re-walks it from notice 0

Status: ready-for-agent — filed 2026-10-14 23:5x UTC. Unit 1 gated green and on main at `66f1e63`. Unit 2 built
2026-10-15 (reviewed; gate, then main). Both deploy with 534 and 543 when the queue is idle. Unit 3 resolves itself
when 2491 finishes. Next: deploy, then the Verify.
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
- took 3.3 h: job 2491 walked 14,923,146 notices in 11,979 s (42.9M mentions, 44.8M variants written), ending at
  01:46 UTC. While it ran, 2492 (org-head-drift) and 2493 (name-phrase-census) queued behind it. Before that, the
  sizing read had answered 408 and the early rate projected past the 07:35 dailies;
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
2. **Scope the runbook step (built 2026-10-15).** Option (a) was taken: each dissolve window re-derives the
   `organization_names` rows of every org its mentions landed on, inside its own transaction.
   - `Db::rederive_org_name_variants` reads ALL of a target org's mentions with labelled name texts, through
     `organization_mentions_org` and the `notice_texts` primary key. It leaves the rows the issue-307 walk would leave:
     per mention, the first labelled text per language in key order; across mentions, the latest (notice, section).
     It writes with REPLACE.
   - `repair_placeholder_orgs_batch` takes the name-field list and language normaliser. The job passes ingest's.
   - `OrgDissolve` reports `variant_orgs` and `variants_written`, and the job's summary names both. A dry run gives an
     upper bound.
   - **The first build was withdrawn by its own review** (never on main). It collected re-bound notices in memory
     and rewrote each notice's variants after the run. Of 16 findings, all confirmed, two decided the redesign:
     - the per-notice rewrite also rewound co-mentioned orgs and reused targets to an older notice's text;
     - an interrupted or failed run lost the list, and with it the variants, while its re-run claimed "no walk
       needed".
     In-transaction, per-org re-derivation removes both.
   - **A second review** (11 findings, all confirmed, all low) added:
     - a forced-rollback test (a trigger refuses the window's change event after the re-derivation, and the satellite
       writes roll back with it);
     - a pinned plan: `org_name_rederive_sql` is a `CROSS JOIN` that seeks `organization_mentions_org` and then the
       `notice_texts` key;
     - fixtures inserted out of key order, and a language with no labelled text that keeps its row;
     - a real-dissolve supervisor test, the corrected `organization_names` schema comment, and summary wording that
       says passes, since the count sums per window.
     Two findings go to issue 544: the raw text (inherited from the 307 walk) and dry-run sizing for big reused
     targets.
   - Tests:
     - `the_dissolve_rederives_the_name_variants_of_the_orgs_it_rebinds`: a reused target keeps its later notice's
       text, a co-mentioned org is untouched, the first text in key order wins, the dissolved org's rows are gone,
       and the satellite writes roll back with their window;
     - `the_name_variant_rederivation_seeks_both_indexes`: the plan pin;
     - `the_placeholder_dissolve_names_its_rederived_name_variants`: the live v2 gate condemns the s.94 PPON, and
       ingest's fields and normaliser reach the store (`EN` → `ENG`).
   - Runbooks: 440 and 522 now say to drop the backfill step.
   - The readers of `organization_names` are the cross-language matcher and the resolver's name-agreement evidence
     (`resolve_one_mention`, `altid_alias_bind`). Both read the union of an org's names.
3. **Re-create the variants for 522's 7 mentions.** Job 2491 started after dissolve 2489, so its whole-corpus walk
   covers notices 46800936, 46806664, 46818757 and 46819715. Read 31966280's and 31966281's rows once it finishes.

## Notes

- 2026-10-14 23:50 UTC: 2491's cancel answered 409. A `max(id)` read of `notices` to size the remainder answered 408
  (not retried). 2491 was left to run, because a restart would only have re-walked it from 0 at the front of the
  queue.
- 2026-10-15 01:46 UTC: 2491 finished ok, 3.3 h for 14.9M notices. Notice ids are sparse (max ~47M), which is why
  the estimates above ran long. Unit 3 is read done: 31966280 `Individual`, 31966281 `DOC-Labor GmbH` and 2123088
  `HISCOX INSURANCE COMPANY LIMITED` each hold their ENG variant.
- 2026-10-15 00:20 UTC: unit 1 on main (`66f1e63`, gate green). A deploy over the running 2491 (`FORCE_BUSY=1`, so the
  restart would bring 2491 back on the stoppable build to be cancelled) was refused by the session's permission
  classifier. 2491 runs to the end, at ~1,070 notices/s with 8.56M walked, and the deploy waits for an idle queue.
  2492, 2493 and the 07:35 dailies queue behind it.
- The sibling audit (every other non-stoppable kind, adversarially verified) is filed as issue 543.
