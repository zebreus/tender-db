# 477 — FTS holds 90–98 % of each year's notices, the missing ones are on the API, and the dashboard reports the source complete

Status: ready-for-agent — UNIT 3 LANDED 2026-10-04 (not deployed, not committed): the `audit-fts-ids` job, the `publication_audit` ledger and FTS's id-based coverage denominator — see the last section. Review fixes applied the same day (see "Unit 3 review fixes"). NEXT: deploy → run the audit (`{"kind":"audit-fts-ids","dry_run":true}` first, then `{"kind":"audit-fts-ids","max_ids":300}` repeated until `due` is 0, ~1 h each at 12 s per id, clear of the 07:35 UTC chain) → read absent/present from `/admin/reports/audit-fts-ids` → refetch the present ids' packages from its `enqueue` list, then process + project → Verify (and list the absent residue here).
Was status: ready-for-agent — TOP-UP COMPLETE 2026-10-03 (Verify 1,180, was 14,093; NEXT unit 3, the per-id audit of the residue). UNIT 1b BUILT 2026-10-02 10:5x UTC (the unit 1b commit; gate GATE-EXIT=0, NOT deployed): a span still full at two seconds keeps its page as a leaf, and a page of ONE notice is completed from the records of its ocid run — see the last section. NEXT: deploy it between top-up chunks, then re-enqueue `{"kind":"fetch","source":"fts","package_kind":"monthly","period":"2023-11","refetch":true}`, process and project, and check that the job row reads `NewVersion · 1 dense span(s) completed from 17 ocid record(s)` and that `033562-2023` is 15 notices on 15 Tenders; the remaining chunks (2024b → 2026-08) do not wait on it; then unit 3 (the per-year id invariant and audit).
Was status: ready-for-agent — UNIT 1b DECIDED 2026-10-02 04:5x UTC (dense-span walk via ocid records; 2023-11 waits on it — see the last section). UNIT 1 DEPLOYED 2026-10-01 16:4x UTC (`3d79f11`; built `e9e73bb`, review fixes `3d79f11`; gate GATE-EXIT=0 in 761 s). The FTS walk never follows `links.next`: full cursorless spans split, never below 2 s; the daily probe walks every day after the newest monthly that holds no daily, which closes the 09-01..06 seam; and a same-id second release is kept. TOP-UP RUNNING: refetch every monthly 2021-01 → 2026-08 with the new walker (`refetch:true`), chunked to end before each 07:35 UTC tick. Chunk 2021 = jobs 1815–1828. NEXT: read 2021-05 (1815) against its 172 missing ids, then the next chunks, then unit 3 (the per-year id invariant and audit).
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

### 2026-10-01 20:2x UTC — chunk 2021 landed; chunk 2022 enqueued

- Fetches 1815–1826: all `ok`. 2021-04 was `Unchanged`, because the old walk had been complete there, and the rest
  were `NewVersion`. Process 1827: `29837 members → 972 notices (972 parsed, 0 quarantined …, 28865 dup)`. Project
  1828: `989 tenders written`.
- Verify by year: **2021 from 924 to 20**. Total **13,189**.
- Chunk 2022: jobs 1832–1843 (refetch 2022-01..12), process 1844, project 1845, enqueued 20:2x UTC after the batch-2
  deploy (`b26cf3a`). It should end around 01:00 UTC, well before the 07:35 tick.

### 2026-10-02 00:5x UTC — chunk 2022 landed; chunk 2023 enqueued

- Fetches 1832–1843: all `ok`. Process 1844: `36788 members → 1781 notices (… 35007 dup)`. Project 1845: `1761
  tenders written`.
- Verify by year: **2022 from 1,483 to 8**, 2021 at 20. Total **11,714**.
- Chunk 2023: 12 refetches, then process 1859 and project 1860, enqueued after the batch-3 deploy (`08c3dd9`). It
  should end around 05:30 UTC.

### 2026-10-02 04:5x UTC — 2023-11 fails on a dense span: one notice fanned out by the API (unit 1b)

- Fetch 1857 (2023-11) failed loud as designed: `…updatedFrom=2023-11-14T10:05:14&updatedTo=2023-11-14T10:05:15: 100
  releases and a next page in a 2-second window`. Staging is intact. The rest of the chunk ran on; 2024-01..05 (jobs
  1861–1867) are queued behind it.
- **What the span holds** (probed from the container, ~40 requests; evidence in `.scratch/tender-db/477-fts/dense-2023-11-14/`):
  - The cursorless page has 100 rows, all with notice id `033562-2023`, tag `planning`, dated 2023-11-14T10:05:15Z. It
    is The Procurement Assist Consortium's pipeline notice.
  - Those 100 rows are **8 distinct releases**, one per ocid (`ocds-h6vhtk-04196f` … `-041976`), each repeated about 14
    times. The API fans the notice out: each ocid's release is served once per ocid of the notice.
  - The notice has **15 ocids**, `04196f` … `04197d`, a contiguous hex run. The record lookup brackets it: `04196e` is
    404, `04197e` is notice `033564-2023` (10:07:02) and `04197f` is `033567-2023`.
  - The DB holds **1 of the 15** (`notices` 46920564, from the old cursor walk). 14 planned procurements are missing.
- **The cursor cannot be the fallback.** A limit=100 walk happened to reach all 15 ocids on 2 pages. A limit=10 walk
  served the same 10 rows (all `04196f`) on 29 pages under one `nextCursor=695701`. That is issue 449's stuck cursor:
  when the ids tie, the cursor does not advance.
- **The record endpoint can.** `GET /ocdsRecordPackages/{ocid}` returns the ocid's releases. For this notice that is 14
  copies of one release, parsed-equal to the listing's. Dedented by 8 spaces (record nesting minus listing nesting),
  the record's raw release is **byte-identical** to the listing's (checked on `041970`). So a member built from a
  record hashes the same as one built from a listing, and archive dedup holds. One server defect: `04196f`'s record
  is a 200 with an **empty body** (twice). Its release is on the listing page.

**Decision (owner): unit 1b, the dense-span walk.** For a span still full at the minimum width:
1. Keep its cursorless page, which is staged as now, as a leaf.
2. Seed with the distinct ocids on that page. Extend the run of consecutive hex ocids in both directions. A
   neighbour joins when its record carries a release whose id is one of the page's notice ids. The first 404, or
   record of another notice, ends that side. Cap the run at a stated bound (e.g. 500 ocids) and fail loud above it.
3. For each ocid in the run that is not already on the page, fetch its record and stage it as
   `<span key>-r<ocid>.json`. Take the releases whose id is one of the page's notice ids, dedent them to the
   listing's nesting, and verify each by re-parsing before splicing. An empty-body record fails loud, unless that
   ocid's release is already on the page.
4. Fail loud, exactly as today, when the page's notice ids are not all accounted for by records. That covers a
   page holding several notices, where a hidden notice could sit below the top 100 by id. The rule stays: no package
   lands unless every leaf is complete.
5. Count dense spans in the fetch outcome, so they are visible rather than silent.
6. Tests use a mock built from the saved page and record shapes: a fanned-out notice lands all 15 ocids; the seam
   ocids end the run; an empty-body record of an ocid on the page passes, one off the page fails; a record release
   dedented equals the listing bytes; a span with two notice ids where one is hidden fails loud.

Sequenced after 481 unit 2 lands, because one gate at a time fits the container's disk. Then re-enqueue
`{"kind":"fetch","source":"fts","package_kind":"monthly","period":"2023-11","refetch":true}`, process and project, and
check that the 15 ocids of `033562-2023` are on 15 Tenders.

### 2026-10-02 05:5x UTC — chunk 2023 landed except 2023-11; chunk 2024a running

- Fetches 1847–1858: all `ok` except 1857 (2023-11, the dense span above). Process 1859: `34884 members → 3236
  notices (… 31648 dup)`. Project 1860: `3158 tenders written`.
- Verify by year: 2021 20, 2022 8, **2023 134** (2023-11 has not been refetched), 2024 3,673 (01 and 02 refetched
  so far), 2025 1,928, 2026 3,335. Total **9,098**.
- `tender-db-jobwatch` reads failed: `WARN jobwatch: failed run in last 26h: fetch #2766 [fts monthly 2023-11] →
  error`. That is the watch working as designed, on a failure this issue already tracks. It clears 26 h after the
  run, or sooner if unit 1b's refetch lands.
- Chunk 2024a (2024-01..05, process 1866, project 1867) should end around 07:00 UTC, before the tick.

### 2026-10-02 10:5x UTC — unit 1b built: the dense-span walk (not deployed)

Gate: GATE-EXIT=0 at 10:44 UTC, the 481-2b agent's `ops/check.sh` on the shared tree with these files in it, unchanged
since 10:12; `run_spec_futures_stay_inside_their_size_budgets` passed (`run_fetch_fts` 96 of 128 bytes,
`run_ingest_spec` 3,544 of 3,904). A focused re-run at 10:5x compiled 0 crates and passed the `dense_*` tests, the
`fts` unit tests and that budget test.

What landed (`fetch::walk_dense_span`, `fts::record_releases`; tests `dense_*` in `crates/ingest/tests/fetch.rs`):
- **A span still full at two seconds keeps its page as a leaf.** If every release on the page carries one notice
  id, the walk reads the records of that notice's ocid run. With several ids it fails loud as before (rule 4): a
  notice below the page's 100 rows cannot be reached by any request.
- **The run.** It is seeded with the page's distinct ocids, which must all be in one series (`ocds-h6vhtk-` plus
  fixed-width lowercase hex). Every ocid from the first seed to the last is asked and must carry the notice. It then
  extends one ocid at a time below and above: a record carrying the notice joins it, and a 404 or a record of other
  notices ends that side. The cap is **`DENSE_RUN_CAP` = 500 ocids** (100 min at the 12 s pace), and a run past it
  fails loud. On 2023-11-14 the walk reads 17 records: the 8 seeds, then `04196e` (404) below, then `041977`…`04197d`
  (join) and `04197e` (`033564-2023`) above.
- **Empty bodies.** An empty body passes for a seed, whose release is on the page (`04196f`). Anywhere else it fails
  loud, because ending the run there could drop the notice's later ocids.
- **The leaves.** Every record carrying the notice is staged atomically as `<span key>-r<ocid>.json` and is a leaf. A
  staged record is never asked again. A 404 or an empty body stages nothing and is asked again on a resume.
- **Assembly.** The assembler reads each record leaf against its span's page. It takes the releases of the page's
  notice id and re-nests them by the measured difference of depths (the spaces before each release's closing line:
  16 in a record, 8 on a page). It refuses a ragged or mixed layout, re-parses the result, and compares it with the
  record's own text with insignificant whitespace removed. Each member is built under the PAGE's header, so it is the
  bytes a listing-served release makes. `033562-2023` lands as 15 members (`<id>.json` and 14 `~<hash8>`), and its
  14-fold repeats collapse.
- **Visibility.** The job row reads `<Outcome> · N dense span(s) completed from M ocid record(s)` (and nothing extra
  when there are none). The progress line adds `N dense span(s), M records`, and the journal gets one
  `[fetch] … dense span` line per span (the daily probe's summary does not count them).
- **Pacing.** Every record request goes through `pace_fts` and the stop checkpoint, exactly like a span page.
  429/Retry-After is `get_bytes`'s.

Measured from the container 2026-10-02 09:2x UTC (3 requests, 8 s apart; saved beside the earlier evidence):
- `041977`'s record, an ocid only the cursor's page 2 showed, re-nests by 8 to page 2's listing bytes exactly. That is
  the second ocid checked, after `041970`, and the first one off the page.
- `04196f` is still a 200 with an empty body (the third ask).
- `GET /ocdsReleasePackages/ocds-h6vhtk-041977` serves the same release at the listing's own depth: 90 KB against
  the record's 139 KB, no re-nesting needed. The walk does not use it. It is noted as the lighter alternative if
  record packages ever prove too heavy (§1 measured one at 21.6 MB).

Where this deviates from the decision:
- **The seeds' records are asked too**, not only the ocids the page does not show. The decision's own rules need
  them. Its empty-body exception ("unless that ocid's release is already on the page") can only fire for a seed's
  record. Rule 4's "accounted for by records" needs the seeds to carry the notice. Each seed's record must hold the
  page's own release of that ocid byte for byte after re-nesting, so the byte equality is checked live on every dense
  span instead of trusted from two ocids. It also catches a second release of the notice under a seed ocid, hidden
  below the page. The cost is 8 requests on 2023-11-14.
- **An ocid inside the seed range that does not carry the notice fails loud**, rather than ending anything. The
  decision only describes the two outward sides.

Still open (as amended by the review below):
- A page of one notice cannot rule out a DIFFERENT notice, with a lower id, published in the same two seconds, wholly
  below the page's rows and on no ocid next to the run (dated in the span next to it, the walk now fails). Unit 3's
  per-year id invariant would show it as a missing id. It is not out of every request's reach: a window offset by one
  second (`[from − 1, from]`, `[to, to + 1]`) is askable and isolates each second, which would narrow this to the
  fan-out's own second (for 2023-11-14, `10:05:13–14` is empty, so all 210 rows are at `:15`). Not built; an option for
  unit 3 or a later unit.
- A second release of the SAME notice on an ocid that is not next to the run (a far process, the shape of
  `038018-2025`'s two releases), in the page's hidden rows, is lost silently. Unit 3 does NOT see it, because the id is
  held, and no request lists a notice's ocids: `GET /ocdsReleasePackages/033562-2023` serves the same capped page as
  the span (100 rows, the same stuck `nextCursor`).

### 2026-10-02 11:4x UTC — unit 1b review fixes (not deployed)

Ten review findings (two major), each checked against the code and, where it needed the API, against 4 live
requests from the container (11:15–11:16 UTC, 11 s apart; saved beside the earlier evidence): the records of
`04196d` and `04196c` are 404, the release package of `04196e` is 404, and `04196f`'s release package is 200,
412,639 bytes, 23 releases (14 copies of `033562-2023`'s one release, byte-identical to the span page's `04196f`
release, and 9 later notices of the process, 2023-12 … 2025-06), with no next.

Fixed, each with a test that fails without it. Six mutants each turned a `dense_` test red: `Empty` read as `Other`,
a 404 left unconfirmed, no look-ahead, no in-span date check, an empty seed's release package left unchecked, and
the probe dropping progress.
- **A record with no release is `Empty`, not "another notice's"** (major). `{"records": []}`, a record without
  `releases`, or releases without an id used to read as `Other`: on the outward walk that ended the side silently,
  and it was staged, so no resume asked again. `Other` now needs a release of another notice id. `Empty` is never
  staged and never ends a run, and an earlier build's staged one is discarded and asked again.
- **A 404 is proven before it ends a side** (major). The series has holes right next to the run (`04196c`..`04196e`).
  A record 404 now ends a side only if the ocid's release package is a 404 too, and only if the
  `DENSE_LOOKAHEAD` = 2 ocids past it do not carry the notice. A carrier past a hole fails loud ("a hole in its run,
  which the walk does not bridge"). A record 404 whose release package carries the notice fails loud. Bridging a
  hole is the option if this ever fires on a real run.
- **An empty record is decided by the ocid's release package** (minor). On a seed (`04196f`), the package must hold
  the page's release byte for byte and no other release of the notice, so the hidden-second-release check now runs
  for an empty seed too. Off the page, a 404 or an empty package means absent (and the look-ahead applies), another
  notice ends the side, and a package carrying the notice fails loud (no member is built from a release package).
- **Another notice dated inside the span fails loud** (minor). This applies to the boundary record that ends a side,
  and to a look-ahead record. Dates are read as UK wall-clock seconds (`fts::uk_wall_of`).
- **The daily probe reports dense spans** (minor). `probe_fts_daily` forwards every day's `FtsProgress`. Its job row
  ends like a fetch's, and its progress line shows the record requests while a walk runs.
- **The counts mean what they say** (minor). `fetch::DenseTally`: spans; ocids, the run lengths (15 for
  `033562-2023`); requests, record and release-package requests actually asked (a staged record read back is not
  one). The row reads `Fetched · 1 dense span(s) completed: 15 ocid(s), 21 record request(s)` where it read 17
  "ocid records".
- **Docs** (minor): operations.md's FTS runbook (dense spans, the new loud failures, and the recovery: delete the
  `<span>-r<ocid>.json` the error names as "staged as …" and re-enqueue), the `fts::split` doc, and the "no request
  reaches it" wording in the fts module doc, `walk_dense_span`'s doc, the several-notice error and uk-fts.md §2.
  Both gaps are named in "Still open" above.

The 2023-11-14 walk under these rules: 21 requests (was 17). Seeds `04196f` (empty record, then its release package)
and `041970`…`041976`. Below, `04196e` (record 404, release package 404) and `04196d`, `04196c` (404). Above,
`041977`…`04197d` join, and `04197e` (`033564-2023`, 10:07:02, outside the span) ends the run. Every one of these
answers was measured live, so the refetch should land 15 members.

Deferred:
- **A cursor audit** (follow the span's `links.next` once, never as a source, and fail on any ocid of the notice it
  names outside the run). On 2023-11-14 page 2 named exactly the 15 ocids, and it would also catch the common case
  of the same-notice far-ocid gap. It is deferred because the decision says "Never follow `links.next`", and an
  audit-only use of the cursor is the owner's call.
- **The offset-window walk** (`[from − 1, from]`, `[to, to + 1]`) is recorded under "Still open" above.

Refuted:
- **"Fail loud on a staged carrying record outside the walked run".** A carrier is staged and never asked again, so
  only the re-asked 404s and empty records can move a run's ends between runs, and one of them that now carries
  only extends the run. The one way a staged carrier sits outside a run, past a hole, already fails loud.


### 2026-10-02 14:3x UTC — unit 1b deployed (`a5c7666`) and verified on both real dense spans

- Refetch 1894 (2023-11): `NewVersion · 1 dense span(s) completed: 15 ocid(s), 21 record request(s)`.
- Refetch 1895 (2025-02, the second dense span at 2025-02-14T16:03:36): `NewVersion · 1 dense span(s) completed: 11
  ocid(s), 16 record request(s)`.
- Process 1896: 1,145 notices. Project 1897: 1,141 Tenders.
- **`033562-2023` now holds 15 FTS notices on 15 distinct Tenders**: 46920564 (the old one, Tender 7954684) plus
  47189064…47189077 (Tenders 8810418…8810431). Before, it held 1.
- Verify total before these two refetches: 3,686 (2021 20, 2022 8, 2023 134, 2024 0, 2025 1,928, 2026 1,596).

### 2026-10-03 22:xx UTC — the top-up is complete: every monthly 2021-01 … 2026-08 refetched

- Every refetch landed `ok`, including the two dense spans (2023-11, 2025-02). The last process/project pair is
  1940/1941.
- **Verify: 1,180** (was 14,093). By year: 2021 20, 2022 8, 2023 6, 2024 0, 2025 73, **2026 1,073**.
- The 2026 residue sits in the daily-walked months (2026-09 onward has no monthly) and in ids that were never published
  (482's probes found 404s and empty releases).
- NEXT: unit 3, the per-year id invariant. Probe every remaining id by id (`/ocdsReleasePackages/{id}`), list the
  404 / empty ones as absent, re-walk the days of any that exist, and give the dashboard's coverage the id-based
  denominator. Then close.

### 2026-10-04 — Unit 3 — landed (not deployed)

The per-year id invariant, as decided in "The invariant" (item 3). Gate not run (focused tests only); not committed.

**What landed.**
- **`publication_audit` ledger** (`crates/store/src/publication_audit.rs`, created at open by `CREATE TABLE IF NOT
  EXISTS`): one row per (source, publication_id) with year, seq, verdict (`absent` | `present` | `error`),
  http_status, published (release `date` as served), published_day (its UK civil day), ocid, releases, detail,
  checked_at, attempts. Named for what it holds rather than `absent_publications`: the ledger is also the job's
  resume state and carries the present ids' dates; the absent set is `verdict = 'absent'`. A re-probe replaces the
  row and counts the attempt.
- **`audit-fts-ids` job** (`Spec::AuditFtsIds`, `run_audit_fts_ids` off `run_spec`'s frame, budget 888/1,024 bytes;
  the walk is `ingest::fetch::audit_fts_ids`, the pure parts `ingest::fts::audit`):
  - held ids are read from the DB (`notices.publication_id` of `fts`, an index range of the identity key), not
    from the archived zips: ~330k short keys against ~90 zips and gigabytes, and it is what the dashboard counts
    and what `process` adds to. A member quarantined before it had an identity reads as missing; the audit then
    finds it `present` (one request, a no-op refetch), never a false `absent`;
  - missing = 1..highest held per year minus held; due = no ledger row, or `error`, plus absents older than
    `recheck_absent_days` when given. `present` is never re-asked (a refetch recovers it, not a probe);
  - `GET {BASE}/ocdsReleasePackages/<id>` through `get_bytes` (the shared retry policy; 429/503/408 honour
    Retry-After) and the process-wide `pace_fts` clock (12 s, shared with every FTS fetch);
  - classification: 404/410, an empty body or `releases: []` → **absent**; a release of the id → **present** with
    the earliest `date` (by instant) and its UK day; anything else (5xx/other 4xx after retries, a throttle that
    outlived them, a transport error, a non-package body, releases of other ids only) → **error**, asked again;
  - wet by default (it writes only its own ledger and report); `dry_run: true` counts missing and due with no
    request; `max_ids` caps a run; stoppable before every request (`STOPPABLE_KINDS`); each answer is written as
    it arrives, so a stop, crash or restart resumes from the ledger; 5 consecutive errors halt the run, store the
    report and FAIL the job;
  - the report (`/admin/reports/audit-fts-ids`) is re-derived from the WHOLE ledger after the run: per-year
    `highest/held/absent/present/errors/unchecked`, `unaccounted`, `complete`, every absent id (the Verify's
    residue list), every present id with its packages, error ids, and `enqueue`, the exact `/admin/jobs` bodies.
- **Present ids are reported, not enqueued** (the simpler correct choice): a month is 82–390 paced requests in a FIFO
  queue, and the right package is a judgement — the open point stands, an id's day cannot always be located (the
  window selects on a hidden instant; `009921-2021`, dated 07-27, is listed on 05-07). So each present id names
  the packages of its nearest held neighbours (where it was listed, read via `notices.fetch_id → fetches`) plus
  its release day's package (the monthly while monthlies reach that month, else the daily). If a refetch of those
  does not recover an id, the next step is a by-id package kind (the by-id answer's member bytes equal a listing's,
  because `member_bytes` drops `uri`/`publishedDate`/`links`) — not built.
- **Coverage** (`coverage.rs`, `model::dashboard`, `ui.rs`): FTS cells carry `published` = the year's highest id held
  less its absent ids, `ratio` = the year's distinct ids held / that (an id with several releases counts once). The
  funnel's FTS row carries `published` (the sum), a new `unaccounted_ids`, renders ` · N ids unaccounted`, and
  `fetch_complete` now also needs `unaccounted_ids == 0`. Expect the ✓ to go away on deploy until the audit has run,
  and to flicker as new never-published ids appear daily (a re-run asks only the new ones; a weekly schedule is the
  option if that proves noisy).
- Docs: `docs/operations.md`, "The FTS id audit and `audit-fts-ids` (issue 477 unit 3)".

**Tests** (focused, gate flags and package set, each GATE-EXIT=0): `fts::audit::tests::*` (5: id shape,
classification, the denominator, due selection, refetch packages + exact enqueue bodies),
`publication_audit::tests::a_reprobe_replaces_the_verdict_and_counts_the_attempt`,
`the_id_audit_asks_each_missing_id_once_and_resumes_from_its_ledger` and
`the_id_audit_halts_on_an_error_streak_and_keeps_what_it_learned` (axum mock of the by-id endpoint, `ingest/tests/fetch.rs`),
`coverage::tests::fts_coverage_is_the_highest_id_less_the_absent_ones_and_gates_fetch_complete`,
`supervisor::tests::the_fts_id_audit_job_enqueues_wet_by_default_and_a_dry_run_asks_nothing`,
`run_spec_futures_stay_inside_their_size_budgets`, `cancelling_a_kind_with_no_checkpoint_is_refused_rather_than_promised`;
regression filters `coverage::` (12) and `fts` (83) green.

**Unit 3 review fixes (2026-10-04, not committed).** Twelve findings checked against the code:
- **Empty 200 → `error`, not `absent`** (major): FTS sent three empty 200s for `04196f`'s existing record (unit 1b),
  so `classify` now answers `error` ("empty body") and the next run asks again; only 404/410 and `releases: []`
  are absent.
- **Control request** (major): before the first by-id request, after every 50 (`CONTROL_EVERY`), and once more at
  the end when absents were recorded since, the run asks the year's highest HELD id. Unless it answers `present`,
  the run halts (the job fails) and the absents since the last passed control are demoted to `error`
  (`Db::demote_publication_audits`, attempts kept). A wrong `fts_base`, path change or outage-as-404 now costs at
  most 50 requests and no false absent. Tested: wrong base → halt, nothing recorded; endpoint that stops serving
  known ids mid-run → 3 absents demoted.
- **Current-year absents re-asked by default** (minor): an absent of the current UK year is due again once its
  answer is 30 days old (`CURRENT_YEAR_ABSENT_RECHECK_SECS`); `recheck_absent_days` still re-asks every year's.
- **Highest held ≠ highest issued for closed years** (minor): documented, not built — module doc, docs/operations.md
  and the funnel tooltip say `complete` is bounded by the highest id held (a closed year's lost final days are
  invisible). Probing above the highest needs the census to grow `highest` from ledger rows; left for a later unit
  if the residue suggests year-end loss.
- **Quarantined members** (minor): the job reads FTS quarantine member paths once per run
  (`Db::quarantined_member_paths`; `NNNNNN-YYYY[~hash].json` → id) and records a missing id they carry as a new
  verdict `quarantined`, with no request: accounted for (`unaccounted` excludes it), still in `published`, never
  asked. A `present` verdict would never have closed (the refetch dedups by hash). The dashboard needs no scan: it
  reads the ledger.
- **Coverage bases** (major + minor, one fix): an FTS cell's `held` and `year_held` are now the distinct ids of the
  id-year (`ids.held`), so the Held column, `ratio` and the era header (`Σ year_held / Σ published`) all divide ids
  by ids — the test's data read 4/3 = 133 % in the era before, 3/4 now. The test asserts `held`/`year_held` and the
  era quotient before and after the absent row.
- **Queue blocking** (minor): the documented runs are capped (`max_ids: 300` ≈ 1 h, repeat until `due` is 0), the
  docs say a cancel can wait up to ~10 min behind a throttled request.
- **Supervisor halt path** (minor): new `a_halted_fts_id_audit_stores_its_report_and_fails_the_job` (axum mock
  answering 400) asserts the `audit-fts-ids HALTED` Err and the stored report.
- **Error-streak reset** (minor): new `the_id_audit_error_streak_resets_on_a_decisive_answer` (4 errors, absent,
  4 errors → no halt, all 9 asked).
- **Recovery one-liner** (minor): the docs list `.enqueue[]` first; the loop is a separate opt-in step with
  `|| break`, so a refused fetch stops before its process/project.
- **Skipped: the test's year derivation** — `1970 + now / 31_557_600` is exactly what `measure_coverage_pipeline`
  uses for `current_year` (coverage.rs), so test and production cannot disagree; switching only the test to a
  civil date would create the mismatch the finding warns of.

Focused tests after the fixes (gate flags, gate package set), GATE-EXIT=0: the audit unit tests (6), the ledger
test, the five ingest mock tests, the coverage test, both supervisor audit tests, `run_spec_futures_stay_inside_their_size_budgets`,
`an_execute_without_an_expected_count_is_refused`, filters `era`, `coverage::`, `fts`.

**Run on prod, after the deploy:**

    /root/aj.sh /admin/jobs '{"kind":"audit-fts-ids","dry_run":true}'      # expect ~1,180 missing, quarantined, due
    /root/aj.sh /admin/jobs '{"kind":"audit-fts-ids","max_ids":300}'       # ~1 h; repeat until due is 0; cancellable, resumable
    /root/aj.sh /admin/reports/audit-fts-ids | jq -r .body | jq '{missing, due, probed, absent, present, errors, quarantined, controls, unaccounted, complete, halted}'
    /root/aj.sh /admin/reports/audit-fts-ids | jq -r .body | jq -r '.present_ids[] | "\(.id)  \(.published_day)  \(.packages | join(", "))"'
    /root/aj.sh /admin/reports/audit-fts-ids | jq -r .body | jq -r '.enqueue[]'   # read first; then the opt-in loop in docs/operations.md

Done when the report reads `complete: true` (every id held, quarantined or absent), the absent list is recorded here, and the
Verify's residue equals the absent count.
