# Issue 477: challenge of the root-cause synthesis

**Verdict:** The cause is right. The FTS API's `links.next` cursor drops rows, and no claim I checked is refuted. A few labels and estimates are wrong. Three gaps matter:
- The fix asks the API for one-second windows, which it rejects.
- The top-up plan cannot find the day of an id that the audit flags.
- The expected leftover of unpublished ids is too low.

I sent 17 requests to the API: 15 returned 200, one returned 400 and two returned 404, with no 429s. I made no repo edits and ran no cargo. Everything I added is under `/root/477/challenge/`. That holds the scripts `pages.py`, `dump.py`, `verify.py`, `model.py`, `model2.py` and `probe*.sh`, plus the pages in `api/` with a `requests.log`.

The issue's Verify command still returns 14,093 today. The box clock is CEST, so `2021-05.zip` landed at 12:36 UTC, before the probes ran.

## The "proven" claims

| # | Verdict | What I found |
|---|---|---|
| 1 | Holds | 2021-05-07: the cursor walk served 103 ids, and they equal what the archive holds in 009911–010062. 2024-01-12: 124 rows, 111 distinct, 13 repeats (1102, 1116, 1132, 1139, 1148, 1161, 1164, 1171, 1172, 1179, 1180, 1181, 1186). The 111 equal the archive. Ids 009910, 010063, 001036 and 001193 are all held. |
| 2 | Holds | The hourly pages give 152 contiguous ids (9911–10062) and 156 contiguous ids (1037–1192). Every hour is one short page with no `next`. |
| 3 | Holds | Every missing id sorts below page 1's last row (9963 and 1093). |
| 4 | Holds, but the key description is wrong | I wrote my own constraint checker for the rule. It finds no contradiction across 42 pages (2021-05-07) or 39 pages (2024-01-12). Correction below the table. |
| 5 | Holds, now on more pages | My 13 new pages without a cursor all fit. The model says a gap on page 1 cannot be cursor loss, so it must be an unpublished id. Two such gaps came back 404 by id: 023840-2021 (rank 47) and 022039-2026 (rank 44). |
| 6 | Holds, now proven directly | `/ocdsReleasePackages/009921-2021?updatedFrom=2021-05-07T00:00:00&updatedTo=2021-05-07T23:59:59` returns the release. The same query for 2021-07-27, its `date`, returns `[]`. So the window selects on a hidden timestamp. 001060 (dated 2024-02-07) and 001169 (2024-02-14) also have re-touched dates. |
| 7 | Holds, and stronger | Page 2 of 2026-02-25 has four gaps: 016955, 016969, 016970 and 017009. The archive is missing exactly those four in 016900–017139. A window from 14:30 to 14:59:59 without a cursor returns 35 contiguous ids, 016944–016978, including all three others. So all four are cursor loss. A window from 12:45 to 13:14:59 on 2026-03-25 lists 027303-2026. A window from 16:15 to 16:29:59 on 2026-04-30 does not list 040111-2026, which fits it being absent. |
| 8 | Holds | August's last id is 082420, dated 2026-08-31T21:58. The first daily id is 084166, dated 2026-09-07T00:02. The code cited at `fetch.rs:489-492` and `supervisor.rs:2246-2251` says what the synthesis says. |
| 9 | Holds | `coverage.rs:624` and `:661-664` are as described. The page keys are only uri, version, extensions, publishedDate, publisher, license, publicationPolicy, releases and links, so there is no total count. |
| 10 | Wrong framing | Page 1 cannot lose rows under the synthesis's own model. Its "0.2–0.3 % page-1 loss" is really the rate of unpublished ids (both samples 404). By id rank, 276 pre-Act and 199 post-Act missing ids sit on page 1. |

The key bands for claim 4 are wider than the synthesis says. Only 009962 is pinned, at 261,858. 009955 and 009947 are only known to be at or below 261,858. The rest of the day sits in at least three bands:
- 605,317–605,324: 009961 and 009963–009969.
- 599,795–599,811: 009931, 009950, 009953, 009974 and 010052.
- Between 261,859 and 599,795: 010062, 010057 and 010035.

A cursor walk of 2021-05-07 with `limit=10` collected only 33 of 152 ids. The loss happens at every page boundary.

**The split table** reproduces exactly: 10,102 + 2,188 + 1,745 + 58 = 14,093. Two labels need correcting:
- **"Pre-Act = cursor loss"** is proven for 94 ids on two days. The bucket also holds unpublished ids (the 276 on page 1, plus 678 single-id runs).
- **"7 of 7 daily-era ids sampled are absent"** is off by one: 037802-2026 is from April. I have now shown 19 distinct daily-era ids absent and none present:
  - by id: 085345, 086495, 087452, 088394, 089221 and 092128;
  - missing from short pages without a cursor: 086496, 086498, 086512, 088391, 088392, 088395, 088396, 088398, 088399, 088402, 088403, 088405 and 088406.

## The hypotheses

- **H1 is now proven.** On 2025-12-10, page 1 returns 081728–081629 with `nextCursor=578232`, the same cursor issue 449 recorded. Page 2 with that cursor returns the same 100 ids, and its `next` equals its own URL. The archive has no gaps on 2025-12-10, because 449's hourly fallback walked it without a cursor.
- **H2 is unproven.**
- **H3 is still about half and half.** Of 13 post-Act gap ids checked, 7 are present and 6 are absent.
  - Present: 027303, 017009, 016955, 016969, 016970 (all -2026), 021810-2025 and 064593-2025.
  - Absent: 040111, 045844, 039194, 039467, 037802 and 022039 (all -2026).
- **H4 is unproven.** One clue: the by-id endpoint accepts `updatedFrom` and `updatedTo`, and its echoed `uri` adds `updatedTo=<now>`.
- **H5 is unproven but fits.** The 2026-02 package, fetched 2026-09-30, matches today's walk exactly.

## Problems with the fix

1. **One-second windows are rejected.** `?updatedFrom=2026-02-25T15:30:46&updatedTo=2026-02-25T15:30:46` returns 400: "'updatedTo' must be later than 'updatedFrom". The spec's `split` only returns `None` for a one-second span, so its last step asks for an invalid URL and fails as a 400 status error, not the designed Malformed error.
   - `split` must refuse any cut that leaves a one-second half. Alternatively, overlap the two halves on the cut second; deduplicating by id makes that free.
   - The mock should return 400 when `from == to`.
   - The real-world risk is tiny: even on 2025-05-23, with 1,778 releases, at most 2 share a `date` second.
2. **Second edges are fine.** Both window ends include the boundary second: 017009 is returned by both 15:30:45–46 and 15:30:46–47, and 017015 by both 15:35:27–28 and 15:35:28–29. So cuts of the form `[a,m]` / `[m+1,b]` leave no hole.
3. **The autumn DST guard may point the wrong way.** The error bodies have the shape of Spring Boot's default, so the server is probably Java. Java resolves an ambiguous local time to the earlier offset by default. In that case a cut at 02:00:00 leaves 01:00–01:59 GMT in no window, and a cut at 01:00:00 is the safe one. This is untested, and traffic at that hour is close to zero.
4. **The audit cannot find the day for an id it flags.** For an id that is present but not held, it says "re-walk that day", but the release's `date` can be wrong (009921 is dated 07-27 but sits in the 05-07 window).
   - This affects 47 runs whose neighbours are more than 4 days apart (108 ids). It also affects 5 runs (11 ids) whose most common nearby day is in no top-up list.
   - With the audit in the daily chain, those ids would fail the chain every day, and "missing_ids 0" can never be reached.
   - A fix is now possible: ask by id with a window, which finds the day by bisection in about 11 requests per id. Or archive these releases by id.
5. **`topup.py` silently skips runs whose next neighbour is dated before the previous one.** That is 43 runs (196 ids): their days list is empty and they are not reported as wide runs. Today, other runs happen to cover all 43 days.
6. **The leftover estimate is low.** Pre-Act unpublished ids exist: 0.2–0.3 % of about 156,000 is roughly 300–450. Expect a residue of about 1,400–1,700, not 1,100–1,300.
7. **The daily-era refetch (9 days, about 150 requests) will probably recover nothing.** All 19 daily-era ids checked are absent.
8. **Checking only `highest+1` for a closed year** cannot see a missing id that sits behind an unpublished one.
9. **The seam fix does not close the current seam.** A daily package is already on record (2026-09-30), so only the 6 top-up jobs recover 2026-09-01..06. The plan does include them.

## Would the fix have caught and fixed every missing run the probes found?

**Yes, for every present id the probes placed in a window:**
- 2021-05-07: 49 ids
- 2024-01-12: 45 ids
- 2026-02-25: 4 ids
- 027303-2026
- the September seam, through the 6 jobs

Each of those was served by a page without a cursor, and each day is in the top-up lists. 2025-12-10 needed nothing.

**Likely, but not shown,** for the present ids no probe placed in a window: 021810-2025, 064593-2025, 031479-2024, 041161-2024, 019609-2024 and 000302-2025. Their neighbour days are in the top-up lists.

**Not fetchable** for the absent ids; the audit only records them.

**Not covered by the plan as written** for the ids whose day is unknown (the wide and inverted runs). The plan needs the window-and-id locate step from problem 4. The one-second bug in problem 1 does not affect any day the probes found.