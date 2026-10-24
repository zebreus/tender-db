# 539 post-drain audit: skeptic pass (2026-10-24)

**Verdict: CONFIRMED.** I re-tabulated everything myself from the raw bodies: the pre-drain `../q03_agg … ../q09_day .out.json`
and the post-drain `./q03_agg … ./q09_day .out.json`, plus `../q01_fetches` for the fetch list and `../q20–q23_opta` for the
prediction check. I did not use `summ_post.py`, `table.txt` or `table.json`. Every total in the audit matches mine. I found no
wrong number and no material overclaim. There are three wording or framing nits and one missed corroboration, listed at the end.
Nothing was sent to the box, and git was not used.

My check scripts are in the session scratchpad (`sk1.py`–`sk5.py`), under
`/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/`.

## (a) Copies are byte-identical

All 14 files match their originals in `../` by sha256: 7 `.sql` and 7 `.plan.txt`. q03 `.sql` is `df277b26845bd633…` and q09
`.sql` is `5630a62a43f999d2…`. The other five `.sql` prefixes in result.md also match. `cmp` on q03 agrees.

Parsing the arms out of the `.sql` gives 11 + 8 + 12 + 13 + 30 + 30 + 31 = **135 fetch arms**. That is 135 distinct fetches with
no duplicate arm, exactly the 135 ids in q01, with nothing missing and nothing extra. In every arm, `SELECT <id> f` equals
`fetch_id = <id>`.

Plans: each `.plan.txt` holds a local sqlite plan and a turso plan-probe. Each has exactly 2 × arms
`SEARCH n USING INDEX notices_fetch_id (fetch_id=?)` and 0 SCAN.

## (b) Bodies are complete

Every one of the 14 bodies (7 pre, 7 post) has exactly these keys: `columns`, `row_count`, `rows`, `truncated`. None has an
`error` key. `truncated` is false, and `row_count` equals `len(rows)` in all of them. The columns are the same 14 in all.

| read | rows pre → post | bytes on disk = bytes in requests.log |
|---|---|---|
| q03_agg | 137 → 94 | 9654 |
| q04_agg | 159 → 108 | 11003 |
| q05_agg | 318 → 220 | 22210 |
| q06_agg | 313 → 203 | 20511 |
| q07_day | 266 → 188 | 19129 |
| q08_day | 311 → 215 | 21808 |
| q09_day | 313 → 232 | 23526 |

- The pre-drain body sizes also equal the sizes in `../requests.log`, so neither set was edited after it was logged.
- All 7 post-drain log lines show `rc=0`, and there is no `err` line.
- The gaps between log timestamps are 13, 7, 8, 7, 6 and 7 s, so "6–13 s" holds. The run times (1.16, 0.88, 1.30, 1.30, 0.48,
  0.48, 0.49 s) match.

## (c) My totals vs the auditor's

The pre-drain bodies reproduce `../fetch_table.txt` on all 135 rows, with 0 mismatches. That includes the 3 empty fetches
(3, 5 and 445).

| group | notices | inv | in | gap1 | gap2–29 | gap30–179 | gap180–364 | gap365+ | stale | stale % | ≥2 d | ≥30 d |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| all, pre | 1,162,607 | 16,344 | 953,695 | 69,326 | 37,450 | 72,799 | 28,896 | 89 | 208,560 | 17.94 | 139,234 | 101,784 |
| all, post | 1,162,607 | **1** | 964,866 | 69,339 | 31,894 | 67,773 | 28,382 | 1 | 197,389 | 16.98 | 128,050 | 96,156 |
| regular excl. 444, pre | 1,040,660 | 16,344 | 941,406 | 68,700 | 24,533 | 5,066 | 514 | 89 | 98,902 | 9.50 | 30,202 | 5,669 |
| regular excl. 444, post | 1,040,660 | 1 | 952,577 | 68,713 | 18,977 | 40 | 0 | 1 | 87,731 | 8.43 | 19,018 | 41 |
| monthly excl. 444, pre | 971,970 | 15,338 | 932,731 | 23,542 | 10,219 | 4,679 | 401 | 48 | 38,889 | 4.00 | | |
| monthly excl. 444, post | 971,970 | 1 | 943,302 | 23,371 | 4,906 | 40 | 0 | 1 | 28,318 | 2.91 | | |
| daily, pre | 68,690 | 1,006 | 8,675 | 45,158 | 14,314 | 387 | 113 | 41 | 60,013 | 87.37 | | |
| daily, post | 68,690 | 0 | 9,275 | 45,342 | 14,071 | 0 | 0 | 0 | 59,413 | 86.49 | | |
| 444, pre = post | 121,947 | 0 | 12,289 | 626 | 12,917 | 67,733 | 28,382 | 0 | 109,658 | 89.92 | | |

- `after` is 350 → 350, `null` 2 → 2, `fut` 0 → 0, `nooff` 2 → 2, `timed` 3,817 → 3,817, and IssueDate present 664,654 →
  664,654. IssueDate after the window is 0 → 0. All of these are unchanged per fetch as well, not only in total.
- Every figure above equals result.md's totals table and its bucket-shift bullets.
- Per profile, inv pre → post: sdk-0.1 10,890 → 0; de-2.1 2,487 → 0; de-1.1 1,360 → **1**; de-1.2 1,138 → 0; de-2.0 469 → 0.
  de-1.0 and sdk-1.0, 1.10, 1.12, 1.13 and 1.14 are 0 → 0. All match.
- Where the pre-drain inversions sat, by served bucket: in window 4,563 (all monthly), gap1 329, gap2–29 5,824, gap30–179 5,026,
  gap180–364 514, gap365+ 88. These match.
- Option (a) prediction (q20–q23, 132 arms) against the post-drain cells (fetch, profile, bucket, IssueDate present): 1,149 cells
  on each side, **0 cells differ, 0 notices off**. The flows into gap1 also match: 329 out to the window, 188 in from gap 2–29,
  and 70 + 56 + 28 = 154 in from gap ≥ 30.
- Lower bound on notices that changed cell: 11,504. Net stale → in window: 11,171. Both match.
- 444 is identical in all 14 columns, row for row, across 15 groups. Only 444, 493, 505 and 543 are fully identical. The audit
  claims something narrower for the 8 fetches with "no bucket change": 444, 459, 493, 505, 543, 612, 800 and 852 have identical
  (profile, bucket, IssueDate) cells. That holds. Inversions in 459, 612, 800 and 852 were 3 + 2 + 2 + 2 = 9, which matches.
- NULL `inv`: 147 groups pre and 147 post, all in the compact daily arms, all `db = none`, with the same keys before and after.
  No `db = none` group has a non-zero inv. Counting NULL as 0 hides nothing.

## (d) Coverage

128 fetches had inv > 0 before the drain, and all 128 are covered: none is missing. All 135 fetches are read: 132 with
notices, plus 3, 5 and 445, which return 0 rows both before and after.

The sup reads q10–q17 have columns `stale, with_v, vers, multi, r_doe, r_ted, r_pub`. They have no `n` total and no `inv`, so
leaving them out costs nothing. q02 is the old-bucket 439 read, superseded by q03 as `../notes.md` line 277 says. The coverage
claim is honest.

## (e) Non-zero post-drain inv

Exactly one notice is still inverted: fetch 431 (2024-01), profile de-1.1, in group `pb = in`, `db = in`. That group has
n = 11,224 and ids 26,239,408–26,260,202. Before the drain the same group had 11,207 notices with inv 78. The 17 inversions in
`b002/in` moved into it (11,207 + 17 = 11,224).

Every sdk-1.x profile is 0 before and 0 after, as expected. So is de-1.0. The audit reports the leftover correctly.

Its explanation ("fits the one row the drain didn't pick up") is supported more strongly than the audit shows. The issue file
(`issues/539-…md`, "Drain — done") records **dry 2731 planned 16,343: sdk-0.1 10,890; de-2.1 2,487; de-1.1 1,359; de-1.2 1,138;
de-2.0 469**. That is the measured per-profile count in every profile except de-1.1, which is short by exactly 1. So the planner's
own rule, max(candidates, IssueDate civil day), did not treat this row as an inversion before the wet run.

That points to a disagreement between this read and the resolver about the IssueDate civil day of one row, for example how
the offset is handled. The audit's other idea, "a re-stamp that still landed before the IssueDate day", does not fit the
per-profile count. The cause is still open. Finding the row needs an id-level read, which is correctly deferred.

## (f) Notice counts

- The count changed for 0 of 135 fetches and 0 (fetch, profile) pairs. 11 profiles keep identical totals.
- min and max id and min and max `ingested_at` per fetch are unchanged.
- IssueDate present and IssueDate after the window are unchanged per fetch.
- Confirmed.

## Discrepancies and nits (none changes a total or the verdict)

1. **Wrong attribution in result.md.** It says "Fetch 431 (monthly 2024-01): 1 notice, profile de-1.1 **(142 before the
   drain)**". 142 is fetch 431's inv across all profiles: de-1.1 95 (17 + 78) and sdk-0.1 47 (5 + 33 + 2 + 1 + 6). Before the drain,
   de-1.1 in fetch 431 had **95**, not 142.
2. **Missed corroboration.** For the 16,343 explanation, the audit cites only the total. The dry plan's per-profile split
   (de-1.1 1,359 against 1,360 measured) pins the gap to the same profile as the leftover and argues against the audit's second
   hypothesis (see (e)).
3. **"Job queue idle before each statement" is not in any saved file.** `run.sh` prints the `/admin/jobs` result to stdout
   only and does not write it to `requests.log`. The claim rests on `run.sh`'s control flow: it exits 7 before sending, and each
   logged `sql` line means the check passed. Also, those 7 `/admin/jobs` reads went to the box through ssh. They are metadata
   reads, outside the 20-statement cap, and are not logged anywhere.
4. **Framing.** "Every plan is a notices_fetch_id index search" refers to the copied pre-drain plan files (a local sqlite
   plan plus a turso plan-probe), not a plan taken on 10-24. Since the SQL is byte-identical this is reasonable, and the faster
   timings (0.48–1.30 s against 1.8–2.8 s before) fit it. `run.sh`'s comment says ≥ 3 s spacing while the code waits 4 s after
   the last logged statement. That is harmless.

**Scope, which the audit discloses.** The headline "16,344 → 1" covers the 135 baseline fetches through fetch 856 (daily
2026-10-18, fetched 10-19 07:35Z). Daily fetches ingested after 10-19 are not measured. The refold or version-order effects are
not measured either.
