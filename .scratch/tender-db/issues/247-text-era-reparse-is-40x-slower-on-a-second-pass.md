# 247 — the same text-era package re-parses 40× slower on a later run, pread-bound inside the DB

Status: needs-triage — observed on prod 2026-08-19 while running the issue-244 campaign; the decisive
comparison (re-run a package that was previously fast) is queued
Kind: performance regression, re-parse path
Blocked by: —
Relates to: 244 (the campaign that hit it), 100 (the re-parse mechanism), 92 (fold quadratic in chain
length — a different quadratic), 179 (legacy refold pays full corpus), 80 (WAL bound in the reclaim walk)

## The observation

`fetch 186` (the 2010-12 TED monthly: a 2.1 GB tar, 1,012 members, 35,830 text-era notices) re-parsed
**twice today with the same code path** and the same selection:

| run | wall clock | rate |
|-----|-----------|------|
| job 760, ~11:50 | **148 s** | 410 members/min |
| the current run, from 13:08 | 27 min for 256 of 1,012 members | **9.5 members/min** |

That is a factor of ~40. Nothing about the package changed, and the second run is the same binary plus
three parser commits that only touch award-name extraction (the notices in this package produce **no**
award sections at all — verified: 0 `LotResult` across 2,000 sampled notices of the package, so the
extractor returns from its gate).

## What it is not

Ruled out by measurement rather than by reasoning:

- **Not runaway extraction.** 0 `LotResult` sections on this package's notices, so `Emit::award` never
  runs and the parse layer written per notice is the same as before.
- **Not the tar.** `reparse_package` streams through `spawn_record_producer` — one pass, one walker
  thread.
- **Not checkpoint thrash.** `CHECKPOINT_EVERY` is 5,000 notices and the WAL is 39 MB.
- **Not disk pressure.** `/data` is 40 % used, 1 TB free.
- **Not my parser's allocations** (two commits fixed real problems there — a 7× flatten/uppercase cost
  and a quadratic name search — and neither changed this package's rate, which is the clue that led
  here).

## What the box says it is

`perf record` on the hot thread, and `/proc/<pid>/io` sampled over 30 s:

    rchar        +8.1 GB / 30 s   (270 MB/s of read() traffic)
    read_bytes   +2.8 GB / 30 s   (94 MB/s actually from disk)
    write_bytes  +188 KB / 30 s   (almost nothing is being written)
    rchar total   182 GB in ~6 minutes of runtime

The stack under the samples is `pread64 → xfs_file_read_iter → filemap_read → _copy_to_iter` with
`xas_load` alongside — page-cache lookups for a file being pread in small pieces. User-space symbols are
stripped in the release build, but the hot addresses cluster in one region, consistent with the DB
engine's page reader. Memory peaked at 19.3 GB in an 11-minute lifetime.

So: per notice, the run is reading hundreds of megabytes out of the 491 GB database and writing almost
nothing. Something in the re-parse's per-notice work is scanning where it used to seek.

## The suspects, in order

1. **`notice_state`'s lookup** — `WHERE source = ? AND publication_id = ? AND content_hash = ?`. There
   IS a `UNIQUE(source, publication_id, content_hash)` for it to seek, but a plan that stopped using it
   (stale statistics after the morning's 12 package re-parses and their folds, which stamped 2.6 M
   tenders epoch-stale and rewrote ~250 k notices) would look exactly like this.
2. **`clear_parsed`'s deletes** across the six parse-layer tables — cheap only if each has its
   `notice_id` prefix available. Issues 111/112 exist because deferred indexes have no guaranteed
   builder; an index that is missing or unusable here turns every delete into a scan.
3. **Page-cache eviction**: the first run may have had its working set warm from the preceding
   `process`/`project` jobs, and a 491 GB database on a box with far less RAM will re-read what it needs.
   This would make the first run the anomaly rather than the second — worth knowing, because it changes
   the campaign's arithmetic from 11 hours to days.

## The decisive experiment (queued)

Re-parse `fetch 240` — the 2006-06 package that took **72 s** for 20,755 notices earlier today, right
after `fetch 186`'s fast run. If it is now slow too, the cause is global (plan/statistics/index or cache),
not the package. If it is still fast, `fetch 186` has a property the others do not, and the 2.1 GB tar
and its 1,012 members are the place to look.

Deliberately NOT done: killing the running job. `cancel` covers queued jobs only, and the
`TENDER_DROP_JOBS` hatch needs a service-environment change this session could not make. The run is
idempotent and will finish; the queue behind it was cancelled so nothing else is blocked.

## Why it matters beyond this campaign

The text era is 215 packages. At the first run's rate the campaign is ~11 hours of queue time; at this
one's it is over a week, and it holds the queue against the daily ingest tick the whole time. Either
number is worth knowing before committing to the remaining 200 packages — and if suspect 3 is the
answer, then no re-parse campaign of this corpus can be planned from a single warm measurement.
