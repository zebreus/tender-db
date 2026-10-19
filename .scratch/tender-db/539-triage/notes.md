# 539 unit 1 — DÖE `published_at` against the delivering export's window (measured 2026-10-19)

All numbers below come from bounded `/v1/sql` reads against the serving DB (29 of the 40-read budget) and 7 public
GETs (of 10). Every read returned `rows` + `columns`, none was truncated, none answered 408, no transport failure
(`requests.log`). Coverage is **all 135 DÖE fetches**: 132 carry notices (1,162,607 notices), 3 carry none.

## Method

- **Fetches** (`q01_fetches`): `v_fetches WHERE source = 'doe'`. The path-free view is on `/v1/sql`, so the
  per-export join that unit 1's text said could not run from the agent does run.
- **Window** of a fetch. `kind = 'monthly'` has `period = 'YYYY-MM'` (`pubMonth=YYYY-MM`), and its window is
  [1st of the month, 1st of the next month). `kind = 'daily'` has `period = 'YYYY-MM-DD'` (`pubDay=YYYY-MM-DD`), and
  its window is that one day. Days are Europe/Berlin, as in crates/ingest/src/doe.rs.
- **Civil day** of an instant: `(published_at + ifnull(published_offset,0)*60) / 86400` (integer epoch-day), and the
  same for `dispatched_*`.
  - The formula is exact for timed values.
  - It is also exact for date-only values: issue 418 anchors those at the civil day's 00:00Z, and the positive DÖE
    offsets keep them on the same day.
  - It still holds on pre-418 local-midnight anchors.
  - `published_offset IS NULL` occurs on 1 row of 1,162,607.
  - Unit-tested on a synthetic copy (`test.db`) with turso through `plan-probe run`: gap 1 / 29 / 30 / 364 / 365
    edges, timed 23:30 and 00:30 local, and the old anchoring all bucket correctly.
- **Gap** = window start − civil day of `published_at`, in days. Buckets: 1, 2–29 (together the issue's "1–29"),
  30–179, 180–364, 365+, plus `in` and `after` (past the window end).
  - "With IssueDate" means `dispatched_at IS NOT NULL`. `dispatched_at` resolves from `BT-05(a)` / `DE1-IssueDate` /
    `SDK01-IssueDate`.
- **One fetch per UNION ALL arm**, each arm bounded by `n.fetch_id = <id>` (equality) with `GROUP BY +n.profile`
  (the issue-323/329 traps). Fetch 444 was split into two reads by `+n.id` < / >= 1,000,000. No IN-list on an
  indexed column drives anything.
- **Ingest order and dedup.** Notices insert `OR IGNORE` on (source, publication_id, content_hash), so the first
  fetch to deliver a notice owns it.
  - The monthlies were ingested in ascending month order: 444 (2022-12) ended 2026-07-23T18:07, then 443 at 18:09,
    and so on through 402 (2026-07) at 19:46. Dailies were ingested one per day afterwards.
  - So a notice that sits in two exports belongs to the EARLIER one. A "stale" notice therefore first appeared in
    an export later than its date: the staleness is not inflated by dedup.
  - Dailies 3, 5 and 445 (2026-07-17, -18, -19) own 0 notices. All their members were already owned by monthly
    4 / 402.
- **Plans**: see the end of this file. Every read's EXPLAIN QUERY PLAN was read locally first (python sqlite3 and
  turso `plan-probe plan`) on `plan.db`, a copy of 545-triage/u2b/plan.db. It already had `notices_fetch_id`,
  `tender_versions_notice`, `fetches_period` and the PK/unique autoindexes, so nothing was added. `run.sh` refuses
  to send any read without a `.plan.txt`, or whose plan SCANs anything but `fetches` or a derived `(subquery-N)`.

## Step 1 — the DÖE fetches

There are 135 fetches.
- **44 monthly**, 2022-12 through 2026-07. Fetch ids: 444 = 2022-12, then descending to 403 = 2026-05, 4 = 2026-06,
  402 = 2026-07. All were fetched 2026-07-19/20. 402 was fetched 2026-07-20 00:24Z, so 2026-07 is partial: days
  1–19.
- **91 daily**, 2026-07-17 through 2026-10-18, fetched D+1 around 07:35Z.
- **Days never fetched: 2026-07-20, 2026-07-21 and 2026-07-24.** No daily exists for them, and the monthly 2026-07
  was fetched before they closed. This is incidental: a corpus gap unless those notices recur in a later export.
- Spelling of the periods: `monthly` 'YYYY-MM' and `daily` 'YYYY-MM-DD'. No period repeats.
- The full per-fetch table (notices, in, after, null, gap buckets, stale %, inversions) is in `fetch_table.txt`.

## Step 2 — `published_at` against the window, per profile

The profiles are read from the data (`eforms:eforms-…`):
- sdk-0.1: 686,394 notices
- de-2.1: 164,049
- de-1.1: 145,859
- de-2.0: 89,338
- de-1.2: 72,986
- sdk-1.0: 3,584
- sdk-1.13: 164
- sdk-1.10: 102
- sdk-1.12: 87
- de-1.0: 31
- sdk-1.14: 13

### The three eras behave differently

| era | fetches | notices | in window | after window end | stale (gap ≥ 1) |
|---|---|---|---|---|---|
| 2022-12 export (444) | 1 | 121,947 | 12,289 | 0 | **109,658 (89.9 %)** |
| monthly 2023-01 … 2026-07 | 43 | 971,970 | 932,731 | 349 | 38,889 (4.0 %) |
| daily 2026-07-22 … 10-18 | 88 | 68,690 | 8,675 | 1 | 60,013 (87.4 %) |

- **The 2022-12 export is a backlog load, not stale dates.** It holds sdk-0.1 (121,660) and sdk-1.0 (287).
  - Of its 109,467 stale sdk-0.1 notices, 23,037 carry an IssueDate, and **every one of those IssueDates is also
    before 2022-12**.
  - None of its notices has an IssueDate later than its requested date.
  - The gaps run to 364 days, so requested dates go back to 2021-12.
  - Both dates agree. These are older notices the first export carried.
- **The daily era shows the portal's publishing cadence.**
  - Export D holds mostly notices whose requested date is D−1: 45,158 of 68,690 (65.7 %).
  - A further 14,314 (20.8 %) are 2–29 days before D. These are mostly Friday or holiday requests that appear 2–3
    days later; de-2.1 Sunday exports hold about 500 gap-2 notices each.
  - Monday exports are tiny, about 80 notices, mostly in window.
  - So a DÖE requested date systematically precedes its pubDay by one publishing day. In the monthly era this shows
    only at month boundaries: 23,542 gap-1 notices.
  - Months whose last day is a holiday or weekend show almost none: 2023-01 has 2, 2024-01 has 2 and 2024-04 has 0.

### Regular exports (excluding 2022-12), by gap and IssueDate (`prof_regular.txt`, `prof_monthly.txt`, `prof_daily.txt`)

| profile | notices | with IssueDate | gap 1 (with / without) | 2–29 | 30–179 | 180–364 | 365+ | stale ≥ 2 d |
|---|---|---|---|---|---|---|---|---|
| sdk-0.1 | 564,734 | 162,320 (28.7 %) | 8,590 / 25,997 | 4,490 / 0 | 3,596 / 0 | 165 / 0 | 45 / 0 | **8,296 (5.1 % of dispatch-bearing)** |
| de-2.1 | 164,049 | 164,047 | 25,542 | 15,933 | 652 | 288 | 43 | 16,916 |
| de-1.1 | 145,859 | all | 4,042 | 1,436 | 395 | 46 | 1 | 1,878 |
| de-2.0 | 89,338 | all | 2,420 | 1,108 | 50 | 0 | 0 | 1,158 |
| de-1.2 | 72,986 | all | 2,078 | 1,314 | 336 | 15 | 0 | 1,665 |
| sdk-1.0 | 3,297 | all | 19 | 243 | 27 | 0 | 0 | 270 |
| sdk-1.10 / 1.12 / 1.13 / 1.14, de-1.0 | 397 | all | 12 | 9 | 10 | 0 | 0 | 19 |

- **A dispatch-less sdk-0.1 notice is never more than 1 day stale in any regular export.** All 25,997 of the
  dispatch-less stale ones have gap = 1, out of 402,414 dispatch-less notices.
  - Every multi-day sdk-0.1 row carries an IssueDate.
  - The issue's "280 of the 586 April-dated notices in the May export carry no IssueDate" does not hold up (Step 4):
    158 are gap-1 (2023-04-30) and 122 belong to the April export.
- **IssueDate position, sdk-0.1 regular stale**: 8,508 in the window, 8,378 before it (mostly the gap-1 rows), 0
  after it.
  - **No IssueDate falls after its export window in any profile**: 0 of 664,654 dispatch-bearing notices.
- **`published_at` after the window end**: 350 (sdk-0.1 190, de-1.1 86, de-2.1 33, de-1.2 21, de-2.0 20).
  **`published_at` after the fetch's own day**: 0.
- **Multi-day inversion** (IssueDate civil day > `published_at` civil day): **16,344 DÖE notices**, about 1.4 % of
  the corpus.
  - By profile: sdk-0.1 10,890, de-2.1 2,487, de-1.1 1,360, de-1.2 1,138, de-2.0 469, all others 0.
  - The 2022-12 export has 0.
  - **The OpenAPI `Tender.dispatched_at` sentence "earlier than published_at" is false today for these 16,344
    rows**, whatever unit 2 decides. The same-day rows are 367's rendering question on top.
- **sdk-1.x** carries neither a requested nor a stamped publication date, so `published_at` falls back to the
  IssueDate. Its staleness is the IssueDate itself predating the export, and no option touches it.

### Where de-* deep staleness comes from (`q24`–`q29`, `src_table.txt`) — this corrects the issue's premise

`PUBLICATION_DATE_FIELDS` ranks `OPP-012-notice` and `DE1-Publication-PublicationDate` (a *stamped* publication
date) above `BT-738-notice` / `DE1-RequestedPublicationDate`.
- The served date is that stamped date on 1,824 of the 1,826 de-* notices that are ≥ 30 days stale.
- On those rows the **requested date sits inside the window for 1,540**, 1 day before it for 154, and 2–29 days
  before it for 130.
- In a stratified sample of every notice except sdk-0.1 (fetches 425, 413, 4, 491, 562, 693 and 846; 46,304
  notices), a stamped date exists on only 0.4–2.2 % of de-* notices.
  - It is **never later** than the requested date.
  - It equals the requested date on 194 of the 702 stamped notices.
  - It is earlier on the other 508, of which 180 are ≥ 30 days earlier.
  - Every ≥ 30-day stale de-* row in the sample is stamped (161 of 161).
- Shallow de-* staleness (1–29 days) is the requested date itself, the cadence above. Over all stale rows except
  sdk-0.1, the requested date is served (hp0) on 33,851 of 34,113 gap-1 rows and 18,590 of 20,043 gap 2–29 rows.
- So **for de-*, the deep tail is a stale OPP-012 / DE1-Publication-PublicationDate, not the requested date**, and
  "eForms-DE 1.x takes DE1-RequestedPublicationDate" holds only for unstamped notices.
- Sample 26411995 shows it:
  - `DE1-Publication-PublicationDate` 2023-12-19 is served.
  - `DE1-RequestedPublicationDate` is 2024-07-25.
  - The IssueDate is 2024-07-25T14:01+02:00.
  - The export is 2024-07.

## Step 3 — supersession exposure (`q10`–`q17`, `sup_table.txt`)

- **Every stale notice causes exactly one version.** `with_v` = `vers` = stale, 208,560 of 208,560.
- Multi-version tender = the tender has another version.
- "Reorder" (`r_doe`) = some later-seq version's notice was delivered by a DÖE export whose window ENDS at or before
  the stale notice's window START.
  - The predicate per fetch is checked against `window()` for all 133 fetches that hold notices. 3 and 5 hold none.
  - The fold orders seq by `published_at`, then source rank, publication_id and notice_id
    (canonical.rs `next_plan_batch`). So a later seq means the stale date placed this version first.
- Two broader counts:
  - `r_ted`: a later-ordered non-DÖE version whose `published_at` is before the window start.
  - `r_pub`: a later-ordered version of any source published (as served) before the window start, i.e. a version
    that a re-date to the window would jump past.

| stale class | stale | in multi-version tender | r_doe | r_ted | r_pub |
|---|---|---|---|---|---|
| all DÖE, gap ≥ 1 | 208,560 | 61,297 | **495** | 1,555 | 3,568 |
| gap ≥ 2 d | 139,234 | 27,022 | 480 | 790 | 2,380 |
| gap ≥ 30 d | 101,784 (76,025 are dispatch-less 2022-12 sdk-0.1) | 3,644 | 259 | 325 | 772 |
| sdk-0.1 with IssueDate, regular exports | 16,886 | 5,451 | 101 | 0 | 1,147 |
| sdk-0.1 without IssueDate | 112,427 | **0** | 0 | 0 | 0 |
| de-1.1 / 1.2 / 2.0 / 2.1 | 5,920 / 3,743 / 3,578 / 42,458 | ~100 % (42,335 of 42,458 for 2.1) | 38 / 22 / 22 / 202 | 185 / 88 / 100 / 1,165 | 305 / 176 / 129 / 1,648 |
| sdk-1.0 (IssueDate fallback) | 480 | 239 | 110 | 0 | 146 |

- The 2022-12 export's 109,658 stale notices sit in multi-version tenders 23 times (sdk-1.0 only). `r_doe` is 0 there
  (no earlier export exists) and `r_pub` is 15.
- Dispatch-less sdk-0.1 notices are always single-version. Every de-* version sits in a multi-version tender
  (consistent with TED twins).
- So the stale date reorders the chain against an earlier-delivered DÖE version **495 times**, 0.8 % of the 61,297
  multi-version stale versions. Counting any version published before the window, it is 3,568.

## Step 4 — the issue's unreconciled counts, ids 26,100,000–26,110,000 (`q18_idrange`)

All 10,001 ids exist. They belong to two fetches:
- **fetch 440** (the 2023-04 export): 269 ids, 26,100,000–26,100,268 (sdk-0.1 268, sdk-1.0 1). All are dated
  2023-04, which is in window for 440.
- **fetch 439** (the 2023-05 export): 9,732 ids (sdk-0.1 9,666, sdk-1.0 66). By date:
  - 2023-05: 9,296
  - 2023-04: 316 (sdk-0.1 312 = 158 without IssueDate, all 2023-04-30, + 154 with; sdk-1.0 4)
  - 2022-10 … 2023-03: 120 (2 + 1 + 7 + 25 + 83 + 2)

How the issue's numbers reconcile:
- **9,934 is not fetch 439.** It is the sdk-0.1 count across the range: 9,666 + 268 = 9,934. Equivalently
  10,001 − 67 sdk-1.0.
- **120** matches exactly.
- **586 April-dated** = 316 (fetch 439) + 269 (fetch 440, in window) + 1. **280 without IssueDate** = 158 (439) +
  122 (440) exactly.
- **The +1, and 9,295 against 9,296:** notice 26100269 (sdk-1.0, the first id of 439) is timed 2023-04-30T22:00:05Z,
  which is **2023-05-01** 00:00:05+02:00. The refute's per-month count evidently read it in UTC (April); this note
  reads the civil day in its offset (May).
- So 9,295 + 120 + 586 = 10,001 is the whole id range across two fetches and both profiles. Only **436** of those
  ids are stale against their own export (120 + 316), and 158 of the 436 are gap-1 dispatch-less rows.
- The span "IssueDates 2023-03-25 … 2023-05-16" also reconciles. The 2023-03-25 minimum is sdk-1.0, whose
  `published_at` = IssueDate (2 notices).
- The whole fetch 439 holds 18,484 notices (ids 26,100,269–26,118,752), not 9,934. Its stale count is 574:
  - gap 1: 179
  - 2–29: 222
  - 30–179: 171
  - 180–364: 2

## Step 5 — samples (`q19_samples`, `bodies/`, 7 GETs)

| notice | profile, fetch (window) | served `published_at` | source field it came from | requested date | IssueDate = served `dispatched_at` | IssueDate in window? | tender, seq / versions |
|---|---|---|---|---|---|---|---|
| 26044864 | sdk-0.1, 443 (2023-01) | 2022-10-19 | SDK01-RequestedPublicationDate | 2022-10-19 | 2023-01-31T14:02:48+01:00 ✓ | yes | 65023, 1 / 1 |
| 26100484 | sdk-0.1, 439 (2023-05) | 2023-03-17 (gap 45) | SDK01-RequestedPublicationDate | 2023-03-17 | 2023-05-02T08:11:00+02:00 ✓ | yes | 416736, 1 / 1 |
| 26044079 | sdk-0.1, 443 (2023-01) | 2021-06-23 (gap 557; 586 d to the IssueDate = the issue's max) | SDK01-RequestedPublicationDate | 2021-06-23 | 2023-01-30T10:42:32+01:00 ✓ | yes | 588923, 1 / 1 |
| 26411995 | de-1.1, 425 (2024-07) | 2023-12-19 (gap 195) | **DE1-Publication-PublicationDate** | **2024-07-25** | 2024-07-25T14:01:02+02:00 ✓ | yes | 175451, 1 / 2 |
| 27077227 | de-2.1, 456 (pubDay 2026-07-23) | 2025-07-14 (gap 374) | **OPP-012-notice** | **2026-07-22** (BT-738) | 2026-07-22T09:00:00+02:00 ✓ | the day before, which is the cadence | 8822368, 2 / 4 |

- **RequestedPublicationDate = served `published_at`**: true for the three sdk-0.1 samples. False for the de-1.1 and
  de-2.1 samples, which serve the stamped publication date.
- **IssueDate = served `dispatched_at`**: true for all five, in both the DB row and the parsed content. The API
  detail for 26044864 and 27077227 serves exactly these strings.
- **The IssueDate's civil day lies inside the delivering monthly window on all 4 monthly samples.** On the daily
  sample it is pubDay − 1.
- At scale: of the dispatch-bearing ≥ 2-day stale sdk-0.1 rows in regular exports, (a) moves 8,292 of 8,296 into
  the window, so the IssueDate lies in the window on all of those.

## Step 6 — the unit-2 decision

Measured shares over the regular exports. The 2022-12 export (109,658) is excluded and is discussed under (b).
- ≥ 30-day stale: 5,669
- 2–29 days: 24,533
- 1 day: 68,700
- The issue's own class (sdk-0.1, ≥ 2 days): 8,296

The (a) column comes from `q20`–`q23` (`opta_table.txt`). Their totals cross-check identical to step 2's
(1,162,607).

| option | ≥ 30 d fixed | 2–29 d fixed | 1 d fixed | sdk-0.1 ≥ 2 d fixed | collateral |
|---|---|---|---|---|---|
| **(a)** `published_at` = max(served, IssueDate civil day) | **5,343 into window (94.3 %)**, +154 to 1 day, 131 at 2–29, 41 left (37 are sdk-1.x fallback) | 5,499 (22.4 %; 4,489 of 4,490 sdk-0.1) | 329 | **8,292 of 8,296 (99.95 %)** | Re-dates 16,344 notices, exactly the multi-day inversions. Leaves the 2022-12 backlog and the 1–3-day cadence alone. Pushes nothing past its window (0 IssueDates after a window). |
| **(b)** clamp/floor to the delivering export's window | 100 % by construction | 100 % | 100 % | 100 % | Re-dates 98,902 regular notices: 68,700 by exactly 1 day, 24,533 by 2–29 days (18,846 of them cadence rows that (a) leaves) and 5,669 by 30+ days, plus 109,658 coherent 2022-12 backlog dates onto 2022-12-01 unless exempted. A monthly floor gives the 1st of the month, a bound and not a date, where (a) gives the exact IssueDate day. It needs a re-fetch-stable notice → fetch link. |
| **(c)** keep and qualify the docs | 0 | 0 | 0 | 0 | Keeps 586-day errors on 1,000s of versions. |

- **Recommendation: (a)**, generalised for `source = 'doe'` to `published_at` = max(every publication-axis candidate
  present, the IssueDate's civil day), keeping the 367/418 rendering.
  - Since a stamped date is never later than the requested date (sample), this equals max(requested, IssueDate day)
    whenever the requested date exists.
  - On the de-* deep tail it gives the requested date or the IssueDate day, e.g. 2024-07-25 for 26411995. It also
    fixes the stale OPP-012 / DE1-Publication-PublicationDate class that the issue did not know about.
  - The issue's objection to (a), "fixes only the 29 % with an IssueDate", does not bite. No dispatch-less sdk-0.1
    notice in any regular export is more than 1 day stale, so the multi-day class is entirely dispatch-bearing.
  - Verify for 65023: `published_at` becomes 2023-01-31.
- **Do not take (b).** It is right only in the daily era, where pubDay is a real day. There it would redefine
  `published_at` as the export day for 87 % of notices, a 1–3-day cadence shift. That is a separate product
  decision, worth a follow-up issue ("DÖE `published_at` precedes the portal's export day by one publishing day")
  rather than part of 539.
- **Also do the (c) docs part either way:**
  - Qualify that DÖE `published_at` is the buyer's requested date and usually precedes the portal's export day by
    1–3 days.
  - The OpenAPI `Tender.dispatched_at` sentence "earlier than published_at" (openapi.json:971) is **false today for
    the 16,344 multi-day rows**. Correct it in the same unit until the drain lands.
- **Reorder exposure** (the D5 corrections the 495 compare engine will announce):
  - 61,297 stale versions sit in multi-version tenders.
  - The stale date orders 495 of them before a version delivered by an earlier DÖE export.
  - 3,568 sit before a version (any source) published before their window. That is a lower bound on what (a)
    moves past: (a) lands them inside the window, past every such version.
  - Re-dates inside a single month (the 4,563 in-window monthly inversions plus partial moves, about 5,173 notices)
    can add same-month reorders. These were not measured.
  - The (a) drain touches 16,344 notices, so at most 16,344 versions, and re-elects only in their tenders.

## Plans (all read locally before sending; every `.plan.txt` is kept)

- `q01_fetches`: SEARCH fetches USING INDEX fetches_period (sqlite3). Turso shows SCAN fetches under the view,
  which is fine: 135 rows.
- `q02`–`q09`, `q20`–`q23`: per arm, **SEARCH n USING INDEX notices_fetch_id (fetch_id=?)**, then the GROUP BY
  sorter.
  - `q20`–`q29` read through a derived table, which turso prints as `SCAN (subquery-0)` over that same seek.
  - `q02` is the first 439 read with the old 1–29 bucket. It is superseded by `q03` and not used.
- `q10`–`q17`:
  - SEARCH n USING INDEX notices_fetch_id
  - SEARCH v USING INDEX tender_versions_notice (caused_by_notice_id=?) LEFT-JOIN
  - per subquery: SEARCH x USING INDEX sqlite_autoindex_tender_versions_1/2 (tender_id=? [AND seq>?]) and SEARCH m
    USING INTEGER PRIMARY KEY
- **Caught locally**: for 444, the constant-false `r_doe` EXISTS planned as `SCAN tender_versions` / `SCAN notices`.
  It was replaced by a literal 0 before sending (`q16`/`q17`).
- `q18_idrange`: SEARCH n USING INTEGER PRIMARY KEY (rowid>? AND rowid<?). Turso prints `(rowid=?)` for a range;
  a 600k-row synthetic copy confirmed the seek (11 groups in 0.005 s against 31 s for the full range).
- `q19_samples`: rowid seeks plus tender_versions_notice plus a tender_versions autoindex seek.
- `q24`–`q29`: notices_fetch_id, plus `SEARCH d USING INDEX sqlite_autoindex_notice_dates_1 (notice_id=?)` per
  subquery.
- Timings on prod: 1.6–3.3 s wall each including about 1.5 s of ssh. The largest was `q11_sup` (20 fetches, about
  500k notices) at 3.28 s.

## Files

- **SQL, results and plans:** `q01`…`q29` `.sql` / `.out.json` / `.plan.txt`, and `requests.log`.
- **Scripts:**
  - `run.sh`: the /v1/sql runner, with a 40-read cap, plan-required and SCAN guards, and no retry.
  - `tplan.sh`: the dual plan check.
  - `get.sh`: GETs, with a 10-call cap and 2 s spacing.
  - `gen.py`: the arm generators and the `check` for `earlier()`.
  - `summ.py`, `prof.py`, `supsum.py`, `optsum.py`, `srcsum.py`, `dates.py`.
- **Tables:** `fetch_table.txt`, `prof_regular.txt`, `prof_monthly.txt`, `prof_daily.txt`, `prof_444.txt`,
  `sup_table.txt`, `opta_table.txt`, `src_table.txt`, `counts.json`.
- **Bodies:** `bodies/c<id>.json` (content) and `bodies/n<id>.json` (detail).
- **Databases:** `plan.db` (the plan copy, schema only) and `test.db` (the plan copy plus synthetic rows for the
  functional checks).

## Caveats

- **pubDay/pubMonth semantics are inferred from the data, not confirmed on the portal.** That the export day is the
  portal's publication day is an assumption; the API is T+1, so pubDay = D is fetchable on D+1.
- **"Earlier export" is window-based.** Two versions inside the same monthly window cannot be ordered by export, so
  `r_doe` excludes same-window pairs.
- **The stamped-vs-requested relation is sampled.** It comes from 7 fetches (46,304 notices), not all fetches. The
  ≥ 30-day hp census (q24–q27) covers all regular fetches.
