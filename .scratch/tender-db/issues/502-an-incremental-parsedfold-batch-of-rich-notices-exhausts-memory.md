# 502 — an incremental ParsedFold batch of 50k rich eForms notices exhausts the server's memory

Status: ready-for-agent. INCIDENT 2026-10-09 ~04:00–04:36 UTC, mitigated. Unit 1 (the bounded batch) is built.
NEXT: gate, deploy, then fold the left-over cohort under a memory watch before the 07:35 UTC daily.
Kind: operations / the incremental fold (`crates/ingest/src/project.rs`)
Relates to: 495 (its unit-3 shadow measurement ran the re-fold that hit this), 91 / 62 (ParsedFold vs buckets),
57 (`APPLY_NOTICE_BATCH`), 175 (the 512 MiB page cache per connection, `cache.conf`)

## What happened

Issue 495 unit 3's second shadow measurement queued a `refold` of `eforms:eforms-sdk-1.6`. That re-queued 37,669
notices and stamped 19,877 Tenders stale, and the trailing incremental `project` (job 2085) planned 73,419
notices. The plan was under `INCREMENTAL_BUCKET_THRESHOLD` (100k), so Phase 2 took `ParsedFold`, whose batch is
`APPLY_NOTICE_BATCH` = 50,000 notices.

1. Inside that first batch the server's RSS reached 54.5 GB, which is the unit's `MemoryHigh=54G`.
2. The cgroup reclaimed file pages, the server binary's text included. `md2` (the root disk) then read
   1.36 GB/s at 98.6% utilisation, and `/data` (md3) sat idle.
3. `/health` and `/v1/sql SELECT 1` stopped answering within 15 s, from about 04:00 UTC.
4. Restarted at 04:20 UTC (the shadow drop-in removed in the same step). The supervisor resumed job 2085 with
   the compare OFF, and RSS climbed to 42 GB within minutes, so it is not the shadow compare.
5. Dropped job 2085 at boot with `TENDER_DROP_JOBS=2085`. Healthy again at 04:35 UTC.
   - Gotcha: the box's `/etc/systemd/system/tender-db.service.d/dropjobs.conf` sets `TENDER_DROP_JOBS=`
     (empty) and sorts after `drop-*.conf`, so a runtime drop-in must sort after it (`zz-…`).
   - A plain `POST /admin/jobs/{id}/cancel` does not survive the restart: the job resumes.

State left behind: the batch never finished, so none of its notices was marked projected. The committed write
batches did rewrite some of the cohort's Tenders (the change cursor moved 920,542,090 → 921,637,189); the rest
are still epoch-stale. The next incremental fold (the 07:35 UTC daily) would plan the same set again and take
the same `ParsedFold` batch.

## Why

`apply_plan_batch` holds the batch's whole parsed layer (`parsed_by_ids`), every `NoticeState` and every
folded `TenderProjection` in RAM at once, and the folds carry each version's accumulated state.
`APPLY_NOTICE_BATCH` was sized on the 8 GB VPS and its legacy-heavy corpus. A rich eForms batch is many times
heavier per notice. The first shadow cohort (job 2083: 75,403 planned notices, mostly legacy `internal-ojs`)
went through the same path untroubled.

The bucketed path folds the same 50k-notice buckets but spills the parsed layer to disk first, and has run the
whole corpus (job 2044). It is no escape hatch for a scattered cohort, though: its pre-pass sweeps the id
range, which for this cohort is hours.

## Units

1. **Bound the batch** (BUILT): `INCREMENTAL_PARSED_BATCH` = 5,000 for the incremental ParsedFold loop.
   - Byte-identical, because groups are applied in `group_key` order whatever the batch boundaries.
   - A daily delta stays one batch.
   - A cancel now lands within one small batch.
2. **Fold the left-over cohort**, watching RSS, in a queue gap before the daily. Then confirm the daily's
   counts.
3. **Measure, then decide** whether the full-project ParsedFold path (`project_with_batch`, `rebuild=true`)
   and the bucketed path's `APPLY_NOTICE_BATCH` buckets need the same bound on today's corpus. Bound by
   estimated bytes rather than notices if they do.
