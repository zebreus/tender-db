# Issue 477 synthesis: why FTS is short, the fix, and the top-up plan

The cause is mainly a defect in the FTS API's `links.next` paging cursor. The fetcher follows that cursor and stores exactly what the API serves, so whole runs of notices never reach the archive. Two smaller causes add to it: six days in September 2026 were never fetched, and some ids were never published at all. Nothing compares the ids held with the ids issued, so the dashboard reports FTS as complete.

I sent 15 new API requests of my own (all answered 200, no 429s), made no repo edits and ran no cargo. Everything I added on the box is under `/root/477/synth/`.

## The cursor defect

The listing sorts a window's releases by notice id, newest first. But `links.next` continues on a different, hidden key that each release carries:
- `nextCursor` is the key of the first row the page did not serve.
- The next page is the newest `limit` rows by id among the window's rows whose key is at or below that cursor.

Those keys do not rise with the notice id. So at every page boundary two things can happen:
- **Silent loss:** rows below the boundary whose key is higher than the boundary row's key are never served. The page that should hold them comes back short with no `links.next`, which looks exactly like a normal last page.
- **Repeats:** rows above the boundary with a lower key are served a second time.

`fetch_fts` follows `links.next` until it is absent (`crates/ingest/src/fetch.rs:388-415`). It only checks for a stuck cursor (`:405`), and it registers the package once the loop ends (`:453-465`).

## How the 14,093 missing ids split (the parts add up exactly)

| Part | Ids | Cause |
|---|---|---|
| 2021-01 to 2025-02-23, contiguous runs on days that needed a second page | 10,102 | Cursor loss, proven on two days |
| 2025-02-24 to 2026-08, mostly single ids | 2,188 | Mixed: cursor loss plus ids never published (sample below) |
| Daily packages 2026-09-07 to 09-30 | 58 | 7 of 7 sampled are absent from the API |
| 082421–084165-2026, 2026-09-01 to 09-06 | 1,745 | Never fetched |

## What is proven (by saved pages or archive reads)

1. **The fetcher stored everything it was served.**
   - 2021-05-07: the cursor walk served 103 ids, the same 103 the archive holds (`/root/477/day-2021-05-07/day-p00{1,2}.json`, `archive_0507.tsv`).
   - 2024-01-12: 124 rows, 111 distinct ids, the same 111 the archive holds (`/root/477/day-2024-01-12/api/whole/`, `diff.json`).
2. **The missing ids are inside the day's window.** Asked hour by hour without a cursor, every hour fits on one short page with no `links.next`. Together the hours return 009911–010062 (152 ids, no gaps) and 001037–001192 (156 ids, no gaps). That includes all 49 and all 45 missing ids.
3. **They are lost on the cursor page.**
   - Page 2 of 2021-05-07 (cursor 261858) holds 3 rows and no next.
   - Page 2 of 2024-01-12 (cursor 652327) holds 24 rows, 13 of them repeats from page 1, and no next.
   - Neither page holds the missing ids, all of which sort below page 1's last row.
4. **The continuation rule above fits every saved page** (probe 0: 20 pages over 4 walks; probe 1: 7 cursor and limit probes).
   - Out-of-step keys: 009962, 009955 and 009947 sit near 261,857 while their neighbours sit near 605,3xx; 001092 is 652,327 and 001130 is 652,328 among keys near 700,5xx.
   - A hand-built cursor `nextCursor=652328` returned page 2 plus 001130.
   - Control: a `limit=63` walk of 2024-01-12 got all 156 ids. So whether a day loses rows depends on which rows land on the page boundaries.
5. **A page asked for without a cursor was correct in every test.** That covers the first pages of the day windows, all 48 hourly first pages, and the new 2026-02-25 15:00 window below. If such a page holds fewer than `limit` rows, the window is complete.
6. **The window does not select on the release `date`, and it does not filter by notice kind.**
   - 009921 and 009932 are dated 2021-07-27 but listed in the 09:00 and 10:00 hours of 2021-05-07; 001040 is dated 2024-02-07 but listed in the 08:00 hour of 2024-01-12.
   - The missing ids cover every tag.
7. **New in this synthesis: single gaps after 2025-02-24 are partly cursor loss.**
   - I asked for 8 random single-id gaps from 2025-03 to 2026-08 by id. Four exist: 027303-2026, 021810-2025, 017009-2026, 064593-2025. Four come back 200 with `releases: []`: 040111, 045844, 039194, 039467 (all -2026).
   - The four that exist all sat at position 116 or later from the day's newest, so on page 2 or later.
   - I also sampled four pre-Act single gaps (031479, 041161, 019609-2024 and 000302-2025); all four exist.
   - **Replay of 2026-02-25:** page 1 is 017125..017026. Page 2 (`nextCursor=523941`) is 017025..016922: 100 rows, no repeats, and 017009-2026 missing. The 15:00–15:59 window without a cursor returns 67 rows, no next, and includes 017009-2026. Saved in `/root/477/synth/byid/` and `/root/477/synth/day-2026-02-25/`, each with `requests.log`.
8. **The September seam was never fetched.**
   - No package holds 2026-09-01..06. The monthly backfill ends at `2026-08.zip` and the first daily package is `2026-09-07.zip`.
   - With no daily package on record, `probe_fts_daily` fetches only `end` (`fetch.rs:489-492`). The comment at `supervisor.rs:2246-2251` wrongly assumes the daily probe covers everything after the backfill.
9. **"Complete" has no denominator.**
   - `fetch_complete` only checks that monthly periods are contiguous (`crates/app/src/coverage.rs:661-664`).
   - `published` is filled only for TED (`coverage.rs:624`).
   - The API pages carry no total count, so a per-window total is not available.
10. **Where the losses fall (probe 3):**
    - In 2021–2024, page 1 loses 0.2–0.3 % of rows, page 2 loses 12.7–24.8 %, and weekend days, which fit on one page, lose nothing.
    - **Estimated** busiest single hour: 217 releases (2025-05-23), so hourly windows alone are not enough.

## Hypotheses (not proven)

- **H1: Issue 449's stuck cursor is the same defect.** If the boundary row's key is at or above every key on page 1, page 2 equals page 1 and `next` points back to the same cursor. That fits 449's 59 identical pages with 100 distinct ids, but I did not replay it.
- **H2: The keys come from a re-keying migration.** The key values are not in time order (about 605k for 2021-05, 700k for 2024-01, 524k for 2026-02, 578k for 2025-12, 590k for 2026-09). Before 24 Feb 2025 keys are often out of step; after it they rarely are. A cluster of re-touched dates around 2025-02-13 to 02-21 also fits.
- **H3: About half of the 2,188 post-Act gaps are cursor loss and half were never published.** That rests on a sample of 8, plus 7 daily-era ids that were all absent. The audit below settles each id.
- **H4: The absent ids were allocated but never published (drafts or withdrawn).** The API answers them two ways, 404 ("'identifier' is not found") or 200 with an empty list; I don't know what separates the two.
- **H5: The window selects on the original publication time**, so a day that has passed is final.

## Fix

### 1. The walk: never follow `links.next`; split any window whose page is full

A detector cannot replace this. Page 2 of 2021-05-07 and page 2 of 2026-02-25 lost rows with zero repeats, and an id gap inside a page looks the same as an id that was never published.

**`crates/ingest/src/fts/mod.rs`**
- Delete `MAX_WINDOW_PAGES` (:112), `MAX_HOUR_PAGES` (:117) and `hour_urls` (:126-142), together with its test (:359).
- Rewrite the module doc and the "paged by an opaque `links.next`" claim.
- Add:
  - `pub struct Span { from: i64, to: i64 }`: inclusive wall-clock seconds, in the same naive encoding `window_url` uses.
  - `window_span(day, overlap_secs) -> Span`.
  - `span_url(base, Span) -> String`. `window_url` becomes `span_url(base, window_span(..))`, so a day's first URL is unchanged byte for byte and so are the registry URLs.
  - `span_key(Span) -> String`, e.g. `20210507T000000-20210507T235959`.
  - `split(Span) -> Option<(Span, Span)>`: halves `[a,m]` and `[m+1,b]`, `None` for a one-second span. The cut never falls inside 01:00–01:59 on the last Sunday of October; it snaps to 02:00:00.
  - `page_is_short(count, next) = count < PAGE_LIMIT && next.is_none()`.
- `Window` carries a `span`.
- Optional: cut at the `date` of the full page's 100th row when it lies inside the span. Correctness only needs the cuts to partition the window, so this changes cost, not correctness.

**`crates/ingest/src/fetch.rs`**
- In `fetch_fts`, replace the page loop and the 449 fallback (:364-458). For each window, run a stack of spans:
  - Use the staged file `<day>-s<span_key>.json` if it exists; otherwise pause, check stop, GET `span_url`, and write it atomically.
  - If the page is not short, split the span and push both halves.
  - If a one-second span is still full, fail with `Error::Malformed("… ≥100 releases within one second; no smaller window (issue 477)")` and leave staging intact.
- Remove `PageCursor`, `CURSOR_FILE`, `read_cursor` and `write_cursor` (:527-568). The staged span files are the resume state.
- `assemble_fts_zip` (:591-646) reads only `-s` pages. A staging dir holding old cursor-walk files (`-pNNN`, `-hNN-pNNN`, `cursor.json`) is deleted, not assembled. None exist on the box today: I checked and there are 0 `.pages` dirs.
- `fetch_fts` refuses a `monthly` target whose last UK day has not ended. It must, or a partly walked month would be registered and then frozen.

**Cost, from a simulation in `/root/477/synth/sim*.py` and `.out`**
- The whole history would cost about 11,800 requests split this way, against 4,550 with cursors.
- The daily poll would cost about 12.8 requests a day (at most 25), against 4.0, which is about 2.6 minutes at 12 s per request.
- Lower `PROBE_DAY_CAP` (:73, now 31) to about 14 and fix its "4–5 paced requests" comment.

### 2. The seam
- **`fetch.rs`:** `probe_fts_daily` starts on the day after the later of (newest daily day, last day of the newest monthly period). `None => end` remains only when neither exists.
- **`crates/app/src/supervisor.rs:2246-2261`:** a default FTS backfill runs through the previous UK month, and an explicit range that reaches into an unfinished month is refused.

### 3. The invariants that would have refused "complete"
- **Per window, built into the walk:** a package cannot land unless every leaf window's page was short.
- **Per year, ids held against ids issued:**
  - Store schema in `crates/store/src/lib.rs`:
    `CREATE TABLE IF NOT EXISTS absent_publications (source TEXT NOT NULL, publication_id TEXT NOT NULL, checked_at INTEGER NOT NULL, status INTEGER NOT NULL, PRIMARY KEY(source, publication_id)) STRICT;`
  - New `Db::fts_id_census() -> Vec<IdCensus{year, highest, held, absent_unheld}>`. It reads `notices.source='fts'` (`publication_id` matching `NNNNNN-YYYY`, served by the existing UNIQUE index) and subtracts absent ids that are now held.
- **`coverage.rs` (`measure_coverage_pipeline`, :600-670):**
  - For FTS, `published = highest − absent` per year.
  - `fetch_complete &&= every year has highest − held − absent_unheld == 0`.
  - A new `PipelineStage.missing_ids: Vec<{year, highest, held, absent, unexplained}>` names the gaps.
- **Closed years:** check `<highest+1>-<year>` once by id, about 5 requests, so a year's newest ids cannot be missing unseen.

### 4. The audit job
- `Spec::AuditFtsIds { limit }`, kind `audit-fts-ids`. Run it out of `run_spec`'s frame with `Box::pin`, as the CLAUDE.md stack trap requires.
- It calls a new `fetch::audit_fts_ids(db, client, base, ids, page_pause, stop)`, which paces each GET `{base}/ocdsReleasePackages/{id}`:
  - 404, or 200 with `releases: []`: record the id in `absent_publications`.
  - 200 with releases: record a "present but not held" finding and end the job in error. The walk missed it, so re-walk that day; the release is not archived by id.
- Add `limit: Option<usize>` to `JobRequest`.
- Daily chain (`supervisor.rs:12104-12113`): add `audit-fts-ids` with limit 50 after `fts daily (all)`. That covers the roughly 6 unpublished ids a day after the Procurement Act.

## Tests

The existing fake paged APIs live in `crates/ingest/tests/fetch.rs` (`fts_server` :269, `fts_day_server` :745, the stuck mock :676). `fts/mod.rs` has only pure unit tests.

**New mock in `tests/fetch.rs`:** `fts_keyset_server(Vec<KeyedRelease{id, at, key}>)`, a model of the measured server. It filters `from ≤ at ≤ to`, sorts by id descending, applies `key ≤ cursor` when a cursor is present, and sets `next` to the key of row `limit+1`. Fixture `day_2021_05_07()`: 152 releases 009911–010062 with the real hourly counts 1,2,11,14,6,17,9,16,20,25,7,7,1,16 (hours 06, 08–18, 21, 22), and low keys on 009962, 009955 and 009947.
- `the_keyset_mock_reproduces_the_2021_05_07_cursor_loss`: following `next` by hand yields exactly the 103 archived ids.
- `a_full_first_page_is_split_never_followed`: the 2021-05 monthly lands all 152, no request carries `cursor=`, and each span is asked once.
- `the_449_stuck_shape_lands_without_a_fallback`
- `a_window_full_within_one_second_fails_with_staging_intact`
- `a_short_page_that_still_names_a_next_is_split`
- `a_stopped_split_walk_resumes_without_asking_a_staged_span_again`: replaces the tests at :433 and :477.
- `staging_from_the_cursor_walker_is_discarded_not_assembled`
- `the_walk_forward_starts_after_the_newest_monthly_not_at_end`: a 2026-08 monthly is registered and `end` is 2026-09-06, so it fetches 09-01..09-06.
- `a_monthly_for_an_unfinished_month_is_refused`
- `the_id_audit_records_absent_ids_and_flags_present_ones`: covers 404, 200 with an empty list, and 200 with one release.
- Rework onto the new mock: `fts_window_follows_links_next_into_one_zip` (rename to `a_daily_window_lands_one_zip_and_a_refetch_hashes_equal`), `fts_gives_up_after_five_throttled_attempts_with_staging_intact` (throttle the second request) and `fts_malformed_page_fails_with_staging_intact`. Delete the stuck test at :676.

**`fts/mod.rs`:**
- `split_partitions_a_span_into_two_nonempty_halves`, including that a one-second span gives `None`.
- `the_day_span_url_is_the_old_window_url`
- `a_split_never_cuts_inside_the_repeated_autumn_hour`
- `page_is_short_needs_fewer_than_limit_and_no_next`

**Store:** `fts_id_census_counts_held_absent_and_unexplained_per_year`. It skips `_noid/`, counts the cross-year `004469-2026` under 2026, and counts an absent id that is now held once.

**`coverage.rs`:** `an_fts_year_with_unexplained_ids_denies_fetch_complete`, beside the test at :853. Ids 1–10 minus 4 give false; recording 4 as absent gives true; a control source is unaffected.

**`supervisor.rs`:** update `fts_backfill_fans_months_under_the_fts_source` (through the previous month) and add `an_fts_backfill_range_into_the_unfinished_month_is_refused`.

## Top-up plan

Deploy the fixed walker first; a top-up run through the old walker loses the same rows again. At the 12 s pace, about 5 requests a minute:

| Step | Jobs | Requests (estimated) | Time |
|---|---|---|---|
| Seam 2026-09-01..06 | 6 daily fetches | about 45 | — |
| Pre-Act affected days (449, 2021-03-29 to 2025-02-21) | daily fetches | about 2,500 | about 8 h |
| Post-Act affected days (326, 2025-02-28 to 2026-08-28) | daily fetches | about 4,800 | — |
| Daily-era affected days (9, 2026-09-09 to 09-30) | daily fetches with refetch | about 150 | — |
| Audit by id | about 5 requests per 1,000 gaps checked plus year ends | about 1,100–1,300 | about 4 h |
| **Total** | 790 fetch jobs plus process and audit | **about 8,800** | **about 29 h plus 429 back-offs** |

- The affected days are both neighbour days of every run, taken from `runs.tsv`.
- 47 runs (about 110 ids) whose neighbours are more than 4 days apart, because of re-touched dates, are left to the audit.
- The audit covers the expected unpublished ids (about half of 2,188, plus 58, plus about 110) and the 5 year ends.

All 790 jobs are generated in `/root/477/synth/topup-jobs.jsonl`, with the day lists in `topup-days-{pre,post,daily}.txt`. Nothing is enqueued. The job shapes are:
- `{"kind":"fetch","source":"fts","package_kind":"daily","period":"2021-05-07"}`
- For the 9 daily-era days, add `"refetch":true`.
- Enqueue them in chunks of about 80–100 jobs, roughly 1,000 requests and 3.5 h each, outside the 07:35 UTC daily chain.
- After each chunk: `{"kind":"process","source":"fts","package_kind":"daily"}`.
- At the end: `{"kind":"project"}`, then `{"kind":"audit-fts-ids","limit":5000}`, then 448's altid re-plan.

The cheaper paths I priced and why I didn't pick them:
- Refetch the 65 affected months whole: about 11,356 requests (about 38 h).
- Fetch every gap by id: 14,093 requests (about 47 h), and it needs a new "by id" package kind.
- Fetch post-Act gaps by id and re-walk only pre-Act days: about 4,750 requests (about 16 h). It saves runner time but adds a second way of acquiring releases, a bolt-on rather than the fix.

**Done:** the issue's Verify count equals the rows in `absent_publications` (the expected residue is about 1,100–1,300 ids, each checked by id), and `missing_ids` reads 0 for every year.

## Side findings (not the cause of 477)

- **Same id, two releases.** `assemble_fts_zip` keeps the first release per id (`fetch.rs:626`). Probe 3 found 11 ids carried by two releases under different ocids, so when both fall in one package the second is dropped. This needs its own issue.
- **Docs to correct:** `docs/research/uk-fts.md` §2 ("Paging is by `links.next`") and §8's recommended strategy, plus a cross-reference on issue 449.
- **Negligible:** the DST and whole-second window edges. The split's autumn-hour guard covers DST.

Files on the box under `/root/477/synth/`:
- `sim.py`, `sim.out`, `sim2.py`, `sim2.out`: the request-cost simulation
- `topup.py`, `topup-jobs.jsonl`, `topup-days-{pre,post,daily}.txt`: the top-up job and day lists
- `byid/` and `day-2026-02-25/`: the saved API pages, each with a `requests.log`