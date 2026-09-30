# 449 — the FTS API's paging cursor can stick; the fetcher followed it forever and blocked the job runner

Status: in-progress — found and built 2026-09-30 01:0x UTC (hourly check-in). Deploying: the deploy's restart is also
the only way to stop the looping job, because fetch jobs have no stop checkpoint (`POST /admin/jobs/1676/cancel`
answered 409 "no stop checkpoint").
Kind: operations / ingestion correctness (FTS), urgent: it blocks the single job runner in front of the 07:35 UTC tick
Relates to: 342 (the FTS fetcher, the backfill in chunks), 252/250 (the cancel checkpoints fetch lacks)

## What happened

Backfill chunk 2025-07 → 2026-02 (jobs 1671–1680). Job 1676, `fetch fts monthly 2025-12`, reached day 2025-12-10.
There the API answered the day window's first page with `links.next` = a cursor URL (`…&cursor=…nextCursor=578232`).
Every request to that URL returned **the same 100 releases and the same `links.next`**. On the box the staging dir
read, at page 59: 5,900 releases, 100 distinct ids, 1 distinct `next`; and the job was still climbing (page 63).
`fetch_fts` followed `links.next` until absent, so it would never end. A fetch job cannot be cancelled, so it held the
runner, with `fts monthly 2026-01`, `2026-02`, the whole-source process and the project queued behind it, and would
have held the 07:35 UTC daily chain too.

## Fix (built 2026-09-30)

`fetch::fetch_fts` marks a window stuck when `links.next` names the URL just fetched, or when the window passes
`fts::MAX_WINDOW_PAGES` (50; a real FTS day is 4–6 pages). The second test also catches a cursor that moves while its
content does not. A stuck window is re-walked with `fts::hour_urls`: one request window per hour over exactly the span
the day window covers, overlap included. An hour holds far fewer than a page's 100 releases, so it needs no cursor.
Hourly pages are staged as `<day>-h<NN>-p<NNN>.json`, and they sort before the stuck `-p` pages, so the hourly copy of
a release wins in `assemble_fts_zip` (first occurrence wins). If a cursor sticks inside one hour, or an hour passes
`MAX_HOUR_PAGES`, the job FAILS with staging intact. A loud failed month is re-enqueued by hand; a silently
truncated month is not noticed. The cursor file keeps the stuck URL as `next`, so a restart mid-fallback redoes the
hourly walk idempotently. That is exactly what job 1676 does after this deploy's restart: it resumes day 10 at the
stuck URL, sees it stuck, and walks the day hour by hour.

Tests: `fts::tests::hour_urls_cover_exactly_the_window_span`, including the daily window's 2 h overlap across a year
boundary. `a_stuck_paging_cursor_falls_back_to_hourly_windows_instead_of_looping` in `tests/fetch.rs`: a mock whose
day window points back at itself, the prod shape. The day lands every release once, the stuck URL is asked once, and
each of the 26 hours once.

## Not fixed here

Fetch still has no stop checkpoint (`/cancel` → 409). The loop was the reason a checkpoint was needed; with the guard,
a fetch job is bounded (a month is at most 31 × (50 + 26 × 20) pages). A checkpoint between pages is a small follow-up
if a stuck API ever needs a manual stop again.

## Verify

After deploy: job 1676 ends `ok`, then `fetch fts monthly 2026-01` starts. `unzip -l /data/archive/fts/monthly/2025-12.zip`
has a plausible December count, compared with the neighbouring months' zips and the research §7 daily counts.
