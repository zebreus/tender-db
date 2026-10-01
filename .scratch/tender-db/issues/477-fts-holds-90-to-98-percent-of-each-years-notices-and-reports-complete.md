# 477 — FTS holds 90–98 % of each year's notices, the missing ones are on the API, and the dashboard reports the source complete

Status: ready-for-agent — UNIT 1 DEPLOYED 2026-10-01 16:4x UTC (`3d79f11`; built `e9e73bb`, review fixes `3d79f11`; gate GATE-EXIT=0 in 761 s). The FTS walk never follows `links.next`: full cursorless spans split, never below 2 s; the daily probe walks every day after the newest monthly that holds no daily, which closes the 09-01..06 seam; and a same-id second release is kept. TOP-UP RUNNING: refetch every monthly 2021-01 → 2026-08 with the new walker (`refetch:true`), chunked to end before each 07:35 UTC tick. Chunk 2021 = jobs 1815–1828. NEXT: read 2021-05 (1815) against its 172 missing ids, then the next chunks, then unit 3 (the per-year id invariant and audit).
Was status: ready-for-agent — ROOT CAUSE PROVEN 2026-10-01 (workflow `wf_4e12a01b-ca9`: three probes, a synthesis, and a challenger who confirmed the cause): the FTS API's `links.next` cursor continues on a hidden per-release key that is not in notice-id order, so page 2 and later silently drop rows, and the dropped page comes back short with no next link. The 2026-09-01..06 seam was never fetched (1,745 ids), and some post-Act ids were never published. NEXT: unit 1, the walk. Never follow `links.next`; split any window whose cursorless page is full, never into a one-second window (the API answers 400). Fix the seam start too. Design and evidence: `.scratch/tender-db/477-fts/`.
Was status: ready-for-agent — filed 2026-10-01 13:5x UTC from the 342 close-out audit. The first unit is the root cause:
why does the `updatedFrom`/`updatedTo` window walk skip notices that the API serves by id? Diff one day's API listing
against its archived members.
Kind: completeness (a source silently short, under a "complete" claim)
Relates to: 342 (the FTS backfill, which this audits), 449 (stuck paging cursor, re-walked hour by hour), 395 (TED
June 2025 never fetched while the pipeline reported complete: the same shape)

## What is wrong

FTS notice ids are a zero-padded per-year sequence (`NNNNNN-YYYY`, `docs/research/uk-fts.md` §7), so a year's
highest id is its count. Read 2026-10-01 13:5x UTC: the member names of every FTS package in `/data/archive/fts`
(68 monthly, 24 daily, 314,156 members, all matching the pattern), against each year's highest id:

| year | distinct ids held | highest id | missing ids | held |
|---|---|---|---|---|
| 2021 | 31,618 | 32,542 | 924 | 97.2 % |
| 2022 | 35,254 | 36,737 | 1,483 | 96.0 % |
| 2023 | 35,595 | 38,048 | 2,453 | 93.6 % |
| 2024 | 37,672 | 41,642 | 3,970 | 90.5 % |
| 2025 | 84,680 | 86,608 | 1,928 | 97.8 % |
| 2026 (to 09-30) | 89,277 | 92,612 | 3,335 | 96.4 % |

That is **14,093 missing notice ids, about 4.3 % of FTS**. They come in contiguous runs, not as scattered single ids:
2021 from 9911 (9911–9917, …), 2022 from 139 (139–146, …), and 2024 from 1037 (1037–1044, …).

**They are on the API.** `GET https://www.find-tender.service.gov.uk/api/1.0/ocdsReleasePackages/<id>`, read from
the box 2026-10-01:
- `009911-2021`: 200, 1 release, `date` 2021-05-07T06:35:29+01:00, tag `tender`;
- `001037-2024`: 200, 1 release, 2024-01-12T08:47:36Z, `tender`;
- `000139-2022`: 200, 1 release, 2022-01-05T07:59:35Z, `tenderUpdate`.

So the notices were published and are retrievable, and the backfill's window walk did not bring them in.

**The dashboard says FTS is complete.** `/api/dashboard` `pipeline` for `fts` reads `fetch_complete: true`,
`missing_periods: []`. Its coverage cells carry no denominator (`published: null`), so nothing compares held with
published. This is issue 395's failure shape in a new source: the pipeline's "complete" means every period was
fetched, not that every notice arrived.

The 342 plan's acceptance check for step 11 ("releases ≈ 319,742 + 2026 YTD", `.scratch/tender-db/342-fts-plan.md`)
would have caught it. 319,742 is exactly the sum of the highest ids 2021 → 2026-09-06 above. The archive holds
314,156 members through 2026-09-30, about 5 % fewer than that sum plus September's ~10,000.

## Hypotheses (unverified)

1. **Paging.** The walk follows `links.next` cursors. A cursor that skips a page, or a page that answers fewer than
   `limit` while more exist, would drop a contiguous run of ids. Issue 449 found cursors that stick; one that jumps
   would look like this.
2. **Window semantics.** `updatedFrom`/`updatedTo` select on the last-updated time, interpreted in UK local time
   (`fts/mod.rs:23`). A release whose update time falls into no fetched window, for example one written into a day
   after that day's window was walked, is never listed. The id runs date from early in each year, so this needs
   checking against `date` and update timestamps.
3. **Server-side filtering.** The listing may omit some release kinds that the by-id endpoint serves.

## First unit: find the cause

- For one day that holds a missing run (2021-05-07 for `009911-2021`, 2024-01-12 for `001037-2024`): walk the API
  listing for that day by hand, saving every page with its `links.next`. Diff the ids against the day's members in
  the monthly package, and see where the missing ids sit: on a skipped page, outside every window, or absent from the
  listing entirely.
- From the diff, fix the walk at its root (a paging-completeness check, a window that covers the update timestamp,
  or a per-id top-up). Add a fetch-side invariant that would have refused "complete": per year, the held ids against
  the highest id seen. The dashboard can show that as FTS's denominator, the way TED's is shown.
- Then a top-up fetch of the missing ids, a process and a fold, and the 448 altid re-plan (2024–2026 ids carry
  PPONs).

## Verify

    ssh -o BatchMode=yes root@zebreus.click 'python3 -c "import zipfile,glob,re,collections; s=collections.defaultdict(set); [s[int(m.group(2))].add(int(m.group(1))) for z in glob.glob(\"/data/archive/fts/*/*.zip\") for n in zipfile.ZipFile(z).namelist() for m in [re.search(r\"(\d{6})-(\d{4})\",n)] if m]; print(sum(max(v)-len(v) for v in s.values()))"'

- **open** (2026-10-01 13:5x UTC): `14093`, the missing ids across 2021–2026.
- **done:** a number near 0. A residue is fine only where each remaining id is shown absent from the API by id (a
  withdrawn notice), with the list recorded here.

## 2026-10-01 14:5x UTC — root cause proven (workflow `wf_4e12a01b-ca9`)

Full reports: `.scratch/tender-db/477-fts/probes-2026-10-01.md`, `root-cause-and-fix-2026-10-01.md` and
`challenge-2026-10-01.md`. The API pages they rest on are saved on the box under `/root/477/`, each directory with a
`requests.log`.

**The cause: the API's paging cursor.** A window's listing is sorted by notice id, newest first. `links.next`
continues on a hidden per-release key instead. The next page is the newest `limit` rows among those whose key is at
or below the cursor, and the cursor is the key of the first row not served. The keys are not in id order, so at every
page boundary rows are silently lost, and some are repeated. A page that lost rows comes back short with no
`links.next`, exactly like a real last page. The fetcher stored every row it was served, but it was not served
these.
- **2021-05-07:** the cursor walk served 103 ids, and the archive holds those 103. The same day asked as 24 hourly
  windows without a cursor returns 152 contiguous ids, 009911–010062, so 49 were lost on page 2 (cursor 261858: 3
  rows, no next).
- **2024-01-12:** 124 rows, 111 distinct, and the archive holds those 111. The hourly walk returns 156 contiguous
  ids, so 45 were lost (page 2, cursor 652327: 24 rows, 13 of them repeats of page 1).
- **2026-02-25:** page 2 (cursor 523941) has 100 rows and no repeats, but four ids are missing. A cursorless 14:30–14:59
  window returns all four.
- **Issue 449's stuck cursor is the same defect** (2025-12-10 replayed: page 2 repeats page 1, and its next is its
  own URL).
- **A page asked for without a cursor was correct in every test** (all 48 hourly first pages, the day first pages
  and the new windows). So a cursorless page holding fewer than `limit` rows is a complete window.
- **The window selects on a hidden publication timestamp, not the release `date`.** 009921, dated 2021-07-27, is
  listed in 2021-05-07 09:00. Both window ends include their boundary second. A window with `from == to` answers
  400 `'updatedTo' must be later than 'updatedFrom'`.

**The 14,093, split** (the parts add up exactly):

| part | ids | cause |
|---|---|---|
| 2021-01 → 2025-02-23, contiguous runs on days that needed a page 2 | 10,102 | cursor loss (proven on two days) |
| 2025-02-24 → 2026-08 | 2,188 | mixed: cursor loss, plus ids never published (of 13 checked by id, 7 present and 6 absent) |
| 082421–084165-2026, 2026-09-01..06 | 1,745 | **never fetched**: the monthly backfill ends at 2026-08 and the first daily is 09-07. With no daily on record, `probe_fts_daily` fetches only `end` (`fetch.rs:489-492`) |
| daily packages 2026-09-07..30 | 58 | 19 of 19 checked are absent from the API (never published) |

**Why "complete".** `fetch_complete` checks only that the monthly periods are contiguous (`coverage.rs:661-664`), and
`published` is filled for TED only (`:624`). The API pages carry no total.

## Decision (owner, 2026-10-01): the fix

1. **The walk** (`fts/mod.rs`, `fetch.rs`). Never follow `links.next`, and delete the cursor and its machinery:
   `PageCursor`, `CURSOR_FILE`, the hourly fallback `hour_urls`, `MAX_*_PAGES`. A window is a span of wall-clock
   seconds:
   - A span whose cursorless page is short (`< limit`, no next) is complete.
   - A full page splits the span in two, and each half is fetched again.
   - A split never produces a one-second span, because the API answers 400. If a two-second span is still full, it
     fails loud with its staging intact (`Error::Malformed`).
   - The staged span pages are the resume state.
   - The day's first URL stays byte-identical, so registry URLs do not change.
   - The autumn DST hour is left to a test that pins the behaviour rather than to a guess, given the challenger's
     point that the server is probably Java and resolves the ambiguous hour to the earlier offset.
   - A monthly target whose last UK day has not ended is refused.
2. **The seam.** `probe_fts_daily` starts the day after the later of the newest daily and the end of the newest
   monthly period. A default FTS backfill runs through the previous UK month.
3. **The invariant.** The per-window part is built into the walk: no leaf window lands without a short page. The
   per-year part compares the ids held with the highest id issued, minus the ids recorded as absent
   (`absent_publications`, filled by an `audit-fts-ids` job that asks the API by id). It feeds FTS's coverage
   `published` and `fetch_complete`, so the dashboard has a denominator. Unit 3 is designed after unit 1 lands; the
   challenger's point that an id's day cannot always be located from its neighbours is open there.
4. **Top-up.** It runs after unit 1 is deployed; the old walker would lose the same rows again. Re-walk the
   affected days (both neighbour days of each run): ~790 daily fetch jobs, ~8,800 requests, ~29 h at the 12 s
   pace, in chunks of 80–100 outside the 07:35 UTC chain. Then process, project, the id audit and 448's altid
   re-plan. The job list is generated at `/root/477/synth/topup-jobs.jsonl`. The challenger found that
   `topup.py` skips 43 inverted runs (196 ids, whose days are covered by other runs today) and that the daily-era
   refetch will likely recover nothing. Regenerate the list after unit 1, with both fixed.

**In unit 1, not its own issue: one id, two releases.** `assemble_fts_zip` keys members by the notice id and keeps
the first (`fetch.rs:626`). 11 ids carry two different releases under different ocids, for example `038018-2025`
(a `tenderUpdate` on the old procurement and an `award,contract` on the new one, same date). Across packages both
are kept. Read 2026-10-01 via `/v1/sql`: `038018-2025` is notices 46727125 (fetch 645) and 46878074 (fetch 666), and
`086149-2026` is 31499804 and 46804978. Inside ONE package the second is dropped without trace. The rewritten
assembler keys a member by id plus a short hash of the release (`<id>.json`, then `<id>~<hash8>.json` for a second
distinct release), so a byte-identical repeat still collapses, and a different release is kept.

## 2026-10-01 16:4x UTC — unit 1 deployed; top-up started

- Built by `wf_f7bb0a6f-28e`: an implementer, three adversarial lenses (completeness, operability, tests-both-ways)
  and a fixer. Every defect they found was reproduced and fixed, and 10 mutants each turned a test red.
  - The walk splits a full cursorless span in two. Only a span of 4 s or more splits, so the API's 400 for
    `from == to` is unreachable. A 2- or 3-second span that is still full fails loud with its staging intact.
  - Cuts move 2 s off 01:00:00 and 02:00:00 on the last Sunday of October, so they hold whichever way the server
    resolves the repeated hour. A test pins it.
  - Staged span pages are the resume state, and a damaged one is re-asked.
  - The assembler keeps `<id>~<hash8>.json` for a distinct second release, and the name it picks does not depend on
    serve order.
  - The probe walks gaps: every day after the newest monthly that holds no daily, capped at 14 per run. It is
    cancellable, and every FTS request in the process is paced on one global clock.
  - A day or month that has not ended in UK time is refused.
  - The cursor machinery and 449's hourly fallback are deleted.
- **Cost, simulated on the archive's 314k timestamps:**
  - a daily poll costs ~12–17 requests (~3 min) against 4–6 before;
  - a month costs 82 requests in 2021 and ~390 in 2026;
  - the whole history is ~11,900 requests, ~40 h at the 12 s pace.
- **Decision: top up by refetching every monthly package.** The alternative was day re-walks chosen from each
  gap's neighbours. The challenger showed that choice misses inverted and wide runs, and depends on locating each id's
  day. A whole-month refetch with the new walker needs no locating and leaves every package complete. Each refetch
  lands as a new fetch, `process` walks it again, and held notices dedup by content hash. The chunks are sized to
  finish before 07:35 UTC, because the queue is FIFO and the tick queues behind them.
- **Chunk 2021:** jobs 1815 (2021-05 first, a month proven short), 1816–1826 (the other months), process 1827, project
  1828.

### 2026-10-01 17:5x UTC — the new walker recovers 2021-05 exactly

- Jobs 1815 (2021-05, 1,025 s), 1816 (2021-01, 694 s), 1817 (2021-02, 682 s) and 1818 (2021-03, 1,083 s) ran ok.
  Each answered `NewVersion`, because the assembled bytes changed.
- The Verify count, by year: 2021 went from **924 to 752**. The difference, 172, is exactly 2021-05's missing ids,
  and 2021-01..03 added nothing; the investigation found those months complete. The total is now **13,921** (from
  14,093). The count reads the union of every archived package, old and new.
- About 12–18 min per 2021 month, in line with the ~82-request estimate at the 12 s pace.
