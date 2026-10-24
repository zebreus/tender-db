# 539 — DÖE `published_at` is the buyer's requested publication date; on a share of sdk-0.1 notices it predates the export that published them by weeks to 586 days, so the IssueDate lands after it

Status: ready-for-agent — DRAIN DONE and VERIFY DONE 2026-10-24 (551 W-on segments z539-1, w539-a/b/c, all PASS; see "Drain — done"): step 1 re-stamped 16,343 notice rows; step 2 refolded 1,161,837 DÖE notices in three class-W runs (9,117 + 2,211 + 2,196 Tenders corrected; NEW lot-sum heads 0; 0 R moves); tender 65023 now serves published_at 2023-01-31. NEXT (last step): retire the 'issue-539 drain' caveats in /docs and the CHANGELOG pre-drain note — served text, so it rides the next gated deploy (the 551 W-on batch: 576 U2, 578 U2, 579 U4, 577 U2 if W7 is empty). The retirement is WRITTEN 2026-10-24 03:0xZ (ungated WIP 5bd158f on claude/cool-sagan-5bk0rc: /docs Dates sentence, the three OpenAPI descriptions, a CHANGELOG "drain ran" bullet, RETIRED token `issue-539 drain re-dates`; focused docs tests 2 passed, EXIT=0). It gates with 576 U2, then done. Was: ready-for-agent — unit 3 BUILT (wf_42503274-9d9: implement, three lenses, a skeptic per finding, fixes), GATED (9ce8649, 1,415 s) and DEPLOYED 2026-10-19 (47def3f). From now on new DÖE notices, and every re-fold of a DÖE Tender, take the new date (each move announced as a correction); stored notice rows keep the old stamp until the drain. NEXT: the drain, AFTER issue 551 B6's flip and U8 and off the 10-20/10-21 D5 mornings: (1) repair-notice-instants dry → reviewed expect_rows → wet; (2) an R3 refold of the DÖE profiles that re-date (sdk-0.1, de-1.0 … de-2.1; ~1.16M notices, verify-dominated) with a reviewed expect, whose trailing project re-sorts the chains and writes the D3 rows; NOT repair-version-instants (see Unit 3's drain bullet). Then Verify on 65023 and retire the 'issue-539 drain' caveats. Was: ready-for-agent — unit 1 MEASURED and unit 2 DECIDED 2026-10-19: (a) for source doe, published_at = the latest of every publication-date candidate present and the IssueDate's civil day (= max(requested, IssueDate day) whenever a requested date exists), date-only rendering kept; the OpenAPI/docs dispatched_at sentence corrected in the same unit. Measured across all 135 DÖE fetches (.scratch/tender-db/539-triage/, adversarially re-derived): 2+-day staleness always carries an IssueDate (sdk-0.1: mostly change-notice versions inheriting version 1's requested date; de-*: a stale stamped OPP-012 / DE1-Publication-PublicationDate); (a) re-dates 16,344 notices and fixes 8,292 of 8,296 sdk-0.1 rows 2+ days stale and 5,343 of 5,669 regular rows 30+ days stale; about 23.5% of re-dated versions change order (~3,800, extrapolated). NEXT: unit 3 build (resolver + test on 26044864 / 26411995 / 27077227, docs), then the drain (repair-notice-instants → an ADR-0017 R3 `refold` of the DÖE profiles that re-date, whose trailing project re-sorts the chains and writes the D3 rows; NOT repair-version-instants, see Unit 3's drain bullet) AFTER issue 551 B6's flip and U8 and off the 10-20/10-21 D5 mornings. Was: ready-for-agent — filed 2026-10-14 from an external data review (verified: verifier + adversarial refute). Next: unit 1, measure `published_at` against the delivering export's period per DÖE profile (S, in-process read).
Kind: data quality (dates) / API contract (`published_at`, `dispatched_at`)
Relates to:
- 18 (resolved 2026-07-20): chose BT-738 / `SDK01-RequestedPublicationDate` as the DÖE publication date ("the DÖE
  portal date", crates/ingest/src/project.rs:930), because DÖE notices carry no OPP-012. sdk-0.1 carries only
  RequestedPublicationDate (100%) and IssueDate (~29%). It rejected the package period as a date source: TED's daily
  periods are OJS issue numbers, and monthly packages span many publication days. This issue questions the BT-738
  premise for DÖE. The TED half of 18 stands.
- 367 (done; unit 3 2026-09-18): measured ALL `dispatched_at > published_at` inversions, multi-day included (96.5%
  of dispatch-bearing sdk-0.1 notices, ids 26.20–26.25M), and attributed them all to the rendering of a date-only
  publication. It never split out the multi-day share, which rendering cannot explain and unit 3 could not fix.
- 418 (done 2026-09-27): date-only instants anchored at civil-day UTC midnight. Same-day only.
- CONTEXT.md:251: the supersession fold orders shared eForms fields by publication date (dispatch as fallback). A
  stale `published_at` can order a later notice first.

## Verify

    curl -s --max-time 30 https://tenders.zebreus.click/v1/tenders/65023 https://tenders.zebreus.click/v1/openapi.json | jq -c 'if .components then .components.schemas.Tender.properties.dispatched_at.description else [.id, .published_at, .dispatched_at] end'

It prints two lines: tender 65023's dates, then the served `Tender.dispatched_at` description.
- **done**: either line 1's `published_at` is no longer `2022-10-19` (unit 2 option a or b), or line 2 no longer
  says "earlier than published_at" unconditionally and names DÖE's requested date (option c).
- **open (2026-10-14)**: `[65023,"2022-10-19","2023-01-31T14:02:48+01:00"]` then
  `"When the buyer dispatched the notice for publication — earlier than published_at. Same rendering rule as published_at."`

## Repro

Bounded reads on `notices` by id range, `GROUP BY +profile`, 2026-10-14 (verifier; third row from the refute).
These count rows where `dispatched_at` is 1 day or more after `published_at`:

| ids | profile | with a dispatch | ≥ 1 day after | ≥ 30 days | max days |
|---|---|---|---|---|---|
| 26,040,000–26,050,000 | sdk-0.1 | 3,171 | 292 (9.2%) | 117 | 586 |
| 26,200,000–26,210,000 | sdk-0.1 | 1,453 | 104 (7.2%) | 56 | 491 |
| 26,100,000–26,110,000 | sdk-0.1 | 2,854 | 271 (9.5%) | 144 | 195 |
| 26,200,000–26,210,000 | de-1.1 | 4,682 | 45 (1.0%) | 4 | 113 |
| both verifier windows | sdk-1.0, de-1.0 | — | 0 | 0 | — |

Export-window reads (refute, notice id ranges against the fetch that delivered them):
- **26,040,000–26,050,000** (fetches 442/443, the 2023-01 and 2023-02 exports):
  - 9,871 of 10,001 notices have `published_at` in 2023-01/02.
  - The other 130 carry requested dates from 2021-06 to 2022-12, yet their IssueDates fall between 2023-01-09 and
    2023-02-08, inside the export window.
  - 26 of the 130, 65023 among them, predate `FIRST_MONTH` 2022-12 (crates/ingest/src/doe.rs:17), the first month
    the DÖE API serves.
- **26,100,000–26,110,000** (fetch 439, the 2023-05 export):
  - 9,295 of 9,934 notices are dated 2023-05.
  - 120 are dated 2022-10..2023-03 and 586 are dated 2023-04. All their IssueDates fall between 2023-03-25 and
    2023-05-16.
  - 280 of the 586 carry no IssueDate.
  - These counts do not reconcile to one total (9,295 + 120 + 586 = 10,001, the id-range width, against 9,934 in
    fetch 439). Unit 1 re-takes them.
- **Sample of the ≥ 30-day rows** (verifier, ids 26.04–26.05M):
  - 26042057, 26042102, 26042106 and 26042151 are published 2022-11-09, with IssueDates 2023-01-06..09;
  - 26042110 is 2022-10-27 → 2023-01-09;
  - 26042146 is 2022-12-01 → 2023-01-10.

## What was found

**65023** is one version, notice 26044864 (DÖE, `eforms:eforms-sdk-0.1`, publication_id `4288d10e-…-1`). That one
notice states:
- `SDK01-RequestedPublicationDate` 2022-10-19, which is served as `published_at`;
- `SDK01-IssueDate` 2023-01-31T14:02:48+01:00, which is served as `dispatched_at`, 104 days later;
- `SDK01-TenderResult-AwardDate` 2000-01-01, a sentinel.

**The stale date is `published_at`, not the IssueDate.**
- The notice reached us in an export one or more months after its "publication" date.
- The IssueDates sit inside the export window.
- 26 requested dates predate the first month the API can serve.

A requested date is what the buyer asked for, not when the portal published. An IssueDate after `published_at` is
only the visible symptom. The class is wider: 71% of sdk-0.1 notices carry no IssueDate and cannot invert, yet they
can carry a stale requested date (280 of the 586 April-dated notices in the May export). Size it by the gap between
the export period and `published_at`, not by the inversion. eForms-DE 1.x takes `DE1-RequestedPublicationDate`, the
same semantics, and shows the same multi-day shape at 1.0%.

**What it touches:**
- `published_at` drives `published_after`/`published_before`, `sort=published_at` and the supersession order.
- The OpenAPI `Tender.dispatched_at` promises "earlier than published_at" unconditionally
  (crates/app/data/openapi.json:971). `Notice.dispatched_at` makes no such claim.
- `notice_stamps` (project.rs:10081) resolves each axis from its own list (`PUBLICATION_DATE_FIELDS` /
  `DISPATCH_DATE_FIELDS`, project.rs:925–947), with no cross-check.

### What the external review got wrong

- "Apparently mixed from different notice versions" is wrong. 65023 has one version and one notice, and that notice
  states both dates. It is a source-side gap in DÖE sdk-0.1, systematic at 7–10% of dispatch-bearing notices.
- Do not re-file the verifier's first draft:
  - it blamed `SDK01-IssueDate` ("lands weeks after publication");
  - its option "drop IssueDate as a dispatch source" would delete the plausible date and keep the wrong one;
  - it said 367 "measured only the same-day inversion" (367 measured all of them and attributed them to rendering);
  - it sized the class by the inversion alone.

## Proposed units

1. **Measure (S).** An in-process read (a census job): `fetches` is not on `/v1/sql`, so the per-export join cannot
   be run from the agent.
   - Per DÖE profile (sdk-0.1, de-1.0, de-1.1, sdk-1.x / de-2.x), count notices whose `published_at` falls before
     the start of the export period that delivered them (pubMonth start, or the pubDay), bucketed by gap (1–29,
     30–179, 180–364, 365+ days). Split by whether an IssueDate exists.
   - Count how many sit in multi-version tenders, and in how many of those the stale date reorders the versions
     (the supersession exposure).
   - Confirm on the portal what pubMonth/pubDay mean on 5 sampled notices (26044864 plus 4 from the sample), e.g.
     whether the notice is in the pubDay export of its IssueDate's day.
2. **Decide (S).** Record it here. Options:
   - (a) `published_at` = max(requested date, the IssueDate's civil day). It fixes only notices with an IssueDate
     (about 29% of sdk-0.1).
   - (b) Clamp to the delivering export's pubDay, or floor at its pubMonth start for a monthly fetch. 18 rejected
     the package period as a date because TED's periods are OJS issue numbers and monthly packages span many days. A
     DÖE pubDay is a real date, and a pubMonth floor is a bound, not a date. Say so in the decision. It needs the
     notice → fetch link, and a later re-fetch of the notice by another export must not move the date.
   - (c) Keep the date as published. Qualify `Tender.dispatched_at` in the OpenAPI and the /docs dates list (near
     docs.rs:838): DÖE `published_at` is the buyer's requested date.
   - **Not:** dropping the IssueDate. It removes the plausible date.
   - Under (a) or (b), versions of multi-version DÖE tenders can reorder, so the refold re-elects fields. The 495
     compare engine announces each change as a D5 correction; size that from unit 1.
3. **Build (S/M).**
   - Under (a)/(b): change the resolver (`notice_instants` / `notice_stamps`) and keep the 367/418 rendering (a
     date-only value stays date-only). Pin it with a test on 26044864's three dates. Drain with
     `repair-notice-instants`, then 418's `repair-version-instants`, then `project`. (Corrected in unit 3's review: that
     route announces nothing and re-sorts nothing; see Unit 3's drain bullet.)
   - Under (c): edit crates/app/data/openapi.json:971 and docs.rs, and gate the served sentence (the 370 class).
   - Either way, the OpenAPI sentence is false today for the multi-day rows. Correct it in the same unit, even
     under (a)/(b), until the drain lands.

## Unit 1 — measured (2026-10-19, workflow wf_1e9fae7b-5bd: a measurer and an adversarial checker)

The unit as proposed said `fetches` is not on /v1/sql. It is: `v_fetches` (issue 45) is served, and `notices.fetch_id` is indexed
(`notices_fetch_id`), so the measurement ran as bounded per-fetch reads (29 + 4 /v1/sql, 7 + 1 GETs, all plans SEARCH-only, no 408s).
Files: `.scratch/tender-db/539-triage/` (notes.md, q*.sql/.out.json/.plan.txt, v539_check_*.py, v539_r*).

**Coverage.** I covered all 135 DÖE fetches through /v1/sql, using 29 of the 40 allowed reads plus 7 of the 10 allowed GETs. Every read returned rows and columns; there were no 408s, no truncated results and no transport failures (see requests.log).
- Fetches 3, 5 and 445 (daily, 2026-07-17 to 19) own no notices. The other 132 hold 1,162,607 notices.
- Steps 2, 3 and the option-(a) residuals each covered all 132. Fetch 444 was split into two reads by +n.id.
- The option-(a) totals match step 2 exactly (1,162,607).
- The stamped-date relation was checked on a stratified sample of 7 fetches (425, 413, 4, 491, 562, 693, 846; 46,304 notices).

Period spelling:
- monthly: 'YYYY-MM' (pubMonth). 44 fetches, 2022-12 to 2026-07. Fetch 444 is 2022-12; ids descend to 403 = 2026-05; 4 = 2026-06; 402 = 2026-07, which is partial (days 1 to 19, fetched 2026-07-20 00:24Z).
- daily: 'YYYY-MM-DD' (pubDay). 91 fetches, 2026-07-17 to 2026-10-18.
- Incidental gap: 2026-07-20, 07-21 and 07-24 were never fetched.

Method:
- Each UNION ALL arm is bounded by n.fetch_id equality, with GROUP BY +n.profile.
- Every plan was checked locally with python sqlite3 and turso plan-probe on 539-triage/plan.db, a copy of 545-triage/u2b/plan.db. It already had notices_fetch_id, tender_versions_notice and the autoindexes, so nothing was added.
- Every plan showed only SEARCH steps (notices_fetch_id, tender_versions_notice, tender_versions autoindex, notices rowid, notice_dates PK). The only SCANs are of fetches (135 rows) or a derived (subquery-N).
- One SCAN was caught locally and removed before sending: a constant-false EXISTS for 444.
- Ingest order: monthlies were ingested oldest month first (444 at 18:07 through 402 at 19:46 on 07-23), and the store keeps the first fetch that delivered a notice. So a "stale" notice genuinely first appeared in a later export; deduplication does not inflate the counts.

**By profile.** Gap = days from published_at's civil day (in its own offset) back to the start of the delivering export's window.

There are three different behaviours:

**(1) The 2022-12 export (fetch 444) is a backlog load.** 109,658 of its 121,947 notices (89.9%) are dated before the window, by up to 364 days.
- sdk-0.1 makes up 109,467 of these: 23,037 have an IssueDate and 86,430 do not.
- Every one of those IssueDates is also before 2022-12, and none is later than its requested date. The two dates agree, so these are genuinely old notices, not stale dates.

**(2) There is a publishing cadence: the requested date usually precedes the export day.**
- Daily exports, 68,690 notices:
  - 8,675 are in window.
  - 45,158 (65.7%) have gap = 1: export D mostly holds requests for D−1.
  - 14,314 (20.8%) are 2–29 days early, mostly weekend lag; de-2.1 Sunday exports each hold about 500 gap-2 notices.
  - 541 are 30+ days early.
- In monthly exports this only shows at month boundaries: 23,542 notices have gap = 1.

**(3) The real stale tail, regular exports only (2023-01 onward, excluding 2022-12):**

| Profile | Notices | Gap 1 | 2–29 | 30–179 | 180–364 | 365+ |
|---|---|---|---|---|---|---|
| sdk-0.1 | 564,734 (162,320 with IssueDate, 28.7%) | 8,590 with / 25,997 without IssueDate | 4,490 | 3,596 | 165 | 45 |
| de-2.1 | 164,049 | 25,542 | 15,933 | 652 | 288 | 43 |
| de-1.1 | 145,859 | 4,042 | 1,436 | 395 | 46 | 1 |
| de-2.0 | 89,338 | 2,420 | 1,108 | 50 | 0 | 0 |
| de-1.2 | 72,986 | 2,078 | 1,314 | 336 | 15 | 0 |
| sdk-1.0 | 3,297 | 19 | 243 | 27 | 0 | 0 |
| Other (sdk-1.10/1.12/1.13/1.14, de-1.0) | 397 | 12 | 9 | 10 | 0 | 0 |

- For sdk-0.1, every row at 2 days or more has an IssueDate: 8,296 rows, 5.1% of dispatch-bearing notices.
- Every de-* notice and every sdk-1.x notice carries an IssueDate.
- **No dispatch-less sdk-0.1 notice in any regular export is more than 1 day stale.**
- 350 notices are dated after the window end (sdk-0.1 190, de-1.1 86, de-2.1 33, de-1.2 21, de-2.0 20). No IssueDate falls after its window (0 of 664,654).
- 16,344 DÖE notices have an IssueDate civil day after their published_at civil day: sdk-0.1 10,890, de-2.1 2,487, de-1.1 1,360, de-1.2 1,138, de-2.0 469.

**New finding that corrects the issue's premise.** The resolver ranks OPP-012-notice and DE1-Publication-PublicationDate above BT-738 / DE1-RequestedPublicationDate.
- For 1,824 of the 1,826 de-* notices that are 30+ days stale, the served date is that stamped date, not the requested date.
- On those rows the requested date is inside the window for 1,540, 1 day before for 154, and 2–29 days before for 130.
- In the sample, a stamped date appears on only 0.4–2.2% of de-* notices. It is never later than the requested date: equal on 194 of 702, earlier on 508 (180 of them by 30+ days).
- sdk-1.x has neither field, so published_at falls back to the IssueDate.

**Supersession.** Every stale DÖE notice causes exactly one version: 208,560 of 208,560.

"Reorder" (r_doe) means a later version in the chain came from a DÖE export whose window ended before the stale notice's window began. The fold orders versions by published_at, so this is a version the stale date pushed ahead of one the portal published earlier.

| Stale class | Stale | In multi-version tenders | r_doe | r_ted | r_pub |
|---|---|---|---|---|---|
| Gap ≥ 1 day | 208,560 | 61,297 | 495 (0.8% of multi-version) | 1,555 | 3,568 |
| Gap ≥ 2 days | 139,234 | 27,022 | 480 | 790 | 2,380 |
| Gap ≥ 30 days | 101,784 | 3,644 | 259 | 325 | 772 |

- r_ted: a later version from another source (TED) was published before the DÖE window.
- r_pub: a later version from any source was published before the window. These are the versions a re-date to the window would jump past.

By profile:
- sdk-0.1 with an IssueDate (regular exports): 16,886 stale, 5,451 multi-version, r_doe 101, r_pub 1,147.
- sdk-0.1 without an IssueDate: 112,427 stale, 0 multi-version, so no exposure.
- de-1.1 / 1.2 / 2.0 / 2.1: almost all multi-version (42,335 of 42,458 for 2.1, consistent with TED twins). r_doe 38 / 22 / 22 / 202; r_pub 305 / 176 / 129 / 1,648.
- sdk-1.0: r_doe 110.
- 2022-12 export: 23 multi-version (sdk-1.0 only), r_doe 0 (no earlier export exists), r_pub 15.

Under option (a), the drain re-dates 16,344 notices. r_pub is a lower bound on the versions (a) moves past. About 5,173 of the re-dates stay within one month; those could add same-month reorders, which I did not measure.

**The issue's fetch-439 counts.** The issue's numbers come from an id range that spans two fetches.

IDs 26,100,000 to 26,110,000 are all present (10,001):
- 269 belong to fetch 440, the 2023-04 export (ids 26,100,000 to 26,100,268). All are dated 2023-04, which is inside that export's window.
- 9,732 belong to fetch 439, the 2023-05 export. Of these:
  - 9,296 are dated 2023-05.
  - 316 are dated 2023-04: 312 sdk-0.1 (158 without an IssueDate, all dated 2023-04-30, and 154 with one) and 4 sdk-1.0.
  - 120 are dated 2022-10 to 2023-03.

How each of the issue's figures reconciles:
- **9,934** is not fetch 439. It is the sdk-0.1 count across the whole id range (9,666 + 268), or equivalently 10,001 minus the 67 sdk-1.0 notices.
- **120** matches exactly.
- **586 April-dated** = 316 from fetch 439 + 269 from fetch 440 (in window for April) + 1.
- **280 without an IssueDate** = 158 from 439 + 122 from 440, exactly.
- **The +1, and 9,295 versus my 9,296:** notice 26100269 (sdk-1.0) is 2023-04-30T22:00:05Z, which is 2023-05-01 at +02:00. The refute counted it in UTC (April); I count the civil day in its own offset (May).

So 9,295 + 120 + 586 = 10,001 covers the whole range across both fetches and both profiles. Only 436 of those ids are stale against their own export (120 + 316), and 158 of the 436 are 1-day-gap notices without an IssueDate.

The whole of fetch 439 has 18,484 notices (ids 26,100,269 to 26,118,752). 574 are stale: 179 by 1 day, 222 by 2–29 days, 171 by 30–179 days, 2 by 180–364 days.

The issue's IssueDate span of 2023-03-25 to 2023-05-16 also reconciles. The 2023-03-25 minimum comes from sdk-1.0, whose published_at is the IssueDate itself.

**Samples.** Notice 26044864 (tender 65023) came from fetch 443, the 2023-01 export.
- Served published_at: 2022-10-19, which equals SDK01-RequestedPublicationDate.
- Served dispatched_at: 2023-01-31T14:02:48+01:00, which equals SDK01-IssueDate and falls inside the window.

The other four samples:

| Notice | Profile | Fetch / window | Served published_at | Gap | Field used | IssueDate | IssueDate vs window |
|---|---|---|---|---|---|---|---|
| 26100484 | sdk-0.1 | 439 / 2023-05 | 2023-03-17 | 45 days | RequestedPublicationDate | 2023-05-02T08:11+02:00 | inside |
| 26044079 | sdk-0.1 | 443 / 2023-01 | 2021-06-23 | 557 days (586 to the IssueDate, the issue's maximum) | RequestedPublicationDate | 2023-01-30T10:42:32+01:00 | inside |
| 26411995 | de-1.1 | 425 / 2024-07 | 2023-12-19 | 195 days | DE1-Publication-PublicationDate (DE1-RequestedPublicationDate is 2024-07-25) | 2024-07-25T14:01:02+02:00 | inside |
| 27077227 | de-2.1 | 456 / pubDay 2026-07-23 | 2025-07-14 | 374 days | OPP-012-notice (BT-738 is 2026-07-22) | 2026-07-22T09:00+02:00 | pubDay − 1 (the cadence) |

- In all five, the IssueDate equals the served dispatched_at, checked in both the database row and the parsed content. The API detail endpoint serves exactly these strings for 26044864 and 27077227.
- The requested date equals the served published_at only for the three sdk-0.1 notices.

**Recommendation (measurer).** Choose (a), generalised for source = 'doe': published_at = the latest of every publication-date candidate present and the IssueDate's civil day, keeping the 367/418 rendering. Because a stamped date is never later than the requested date, this equals max(requested date, IssueDate day) whenever a requested date exists.

What each option fixes, over regular exports (98,902 stale notices: 5,669 at 30+ days, 24,533 at 2–29, 68,700 at 1 day):

**(a)**
- 30+ days: 5,343 moved into the window (94.3%), 154 more to within 1 day, 131 left at 2–29, 41 left at 30+ (37 of those are sdk-1.x, whose date already is the IssueDate).
- 2–29 days: 5,499 fixed (22.4%).
- 1 day: 329 fixed.
- The issue's own class (sdk-0.1, 2+ days): 8,292 of 8,296 fixed (99.95%).
- It also fixes the de-* stale-stamped-date tail; for example 26411995 becomes 2024-07-25.
- Total re-dated: 16,344 notices, all of them inversions. No notice is pushed past its export window.
- It leaves the 2022-12 backlog (109,658 notices with consistent old dates) and the 1–3-day cadence untouched.
- The objection "it only fixes the 29% with an IssueDate" does not apply: the whole multi-day class carries an IssueDate.

**(b)** Fixes 100% by construction, but:
- Re-dates all 98,902: 68,700 by exactly 1 day, 24,533 by 2–29 days (18,846 of them are cadence rows that (a) leaves alone), and 5,669 by 30+ days.
- Would collapse the 109,658 backlog notices onto 2022-12-01 unless that export is exempted.
- A month floor gives the 1st of the month, which is a bound, not a date. (a) gives the exact IssueDate day.
- Needs a notice-to-fetch link that stays stable across re-fetches.
- A daily-only "published_at = pubDay" rule would redefine published_at for 87% of daily-era notices. That deserves its own follow-up issue rather than being folded into 539.

**(c)** Fixes 0%.

Do the docs part of (c) either way:
- State that DÖE published_at is the requested date, or a stamped OPP-012 / DE1-Publication-PublicationDate, and usually runs 1–3 days before the portal's export day.
- The OpenAPI Tender.dispatched_at sentence "earlier than published_at" (openapi.json:971) is false today for the 16,344 notices whose IssueDate falls on a later day than their published_at. Correct it in the same unit until the drain lands.

**Adversarial check** (sound: True):

- Verdict: the measurement holds up. Every reported number re-derives exactly from the saved .out.json files, and the windows are right. The problems below are in how some numbers were read and in small counts; none of them reverses the recommendation. Re-derived: 1,162,607 notices and the per-profile totals (exact 'eforms:eforms-…' profile strings); the three-era table; the regular-export table; 16,344 inversions; 350 dated after the window; 0 of 664,654 IssueDates after the window; the (a) residuals 5,343/154/131/41, 5,499, 329 and 8,292 of 8,296; supersession 208,560 / 61,297 / r_doe 495 / r_ted 1,555 / r_pub 3,568 and the per-profile rows; the step-4 id-range reconciliation; sample gaps 557/586/45/195/374. Check scripts: .scratch/tender-db/539-triage/v539_check_*.py.
- Windows, calendar and coverage are correct. All 539 UNION ALL arms carry the same constants as an independent calendar computation: monthly is [1st, 1st of next month), daily is [pubDay, +1). The CASE structure matches on all 267 aggregation and option-(a) arms. The civil-day formula is right for date-only values (stored at civil 00:00Z with offset 0/60/120 per 418) and for timed ones (26100269 lands on 2023-05-01). Steps 2, (a) and supersession each have rows for 132 fetches. Fetches 3, 5 and 445 were queried in q07 and returned nothing. The stamped-date read (q24–q27) covers the 131 regular fetches; 444 is excluded on purpose. The r_doe 'earlier export' predicate matches the independent windows for all 133 arms against all 135 fetches, with 0 mismatches. Version seq is publication order (fold ORDER BY published_at, source_rank, publication_id). Every plan file predates its request, shows only SEARCH steps on notices, tender_versions and notice_dates, and plan.db's indexes match the store DDL.
- Wrong claim about the reorder count. The notes call r_pub (3,568) 'a lower bound on the versions (a) moves past'. It is not. r_pub counts stale versions whether or not (a) touches them, and (a) re-dates only the 16,344 inversions. In fetch 491, r_pub is 17 but only 2 of (a)'s re-dates move past another version. Meanwhile (a) also re-dates 4,563 in-window inversions that r_pub never counts. Measured directly (v539_r3_aswap: fetches 439, 425, 411 and 491; 1,368 re-dates): 322 (23.5%) would move past at least one version currently ordered after them, 71 of those from in-window re-dates. By profile: sdk-0.1 214 of 1,017, de-2.1 93 of 170, de-1.1 15 of 140. That extrapolates to roughly 3,800–3,900 re-dated versions changing order across the drain, so the size of the D5 corrections is still an estimate, not a measurement. Also, 'reorders 495 chains' is wrong wording: r_doe counts stale versions, not tenders.
- The mechanism behind the stale dates is different from what the issue and notes describe (v539_r1_sibling, v539_r4_examples, plus 1 GET). In fetch 439, 294 of the 385 sdk-0.1 notices that are 2+ days stale (76%) are later versions (UUID suffix -2…-n) of a notice already in the database. For 279 of them a sibling version came in an earlier export, and 275 have a sibling with the identical published_at. The in-window rate is 504 of 5,018 (10%). Exhibit: 26100556 (…-5, IssueDate 2023-05-02, fetch 439) states RequestedPublicationDate 2023-02-13, the same date as versions -2 to -8, whose IssueDates run from 02-20 to 06-26. So the stale date is mostly inherited from the first version of a change-notice chain; the notice did not reach us months late. No exact publication_id duplicates exist (0), so deduplication does not inflate the counts, as the notes say. This strengthens (a): each version gets its own IssueDate day, and the fold no longer has to break tied dates on the publication_id string. 65023 (a lone -1 whose date predates FIRST_MONTH) is the minority case.
- Generalised (a) checked (v539_r2_general: 7 sample fetches, 46,304 non-sdk-0.1 notices, 702 with a stamped date). On every one of the 702, max(stamped, requested, IssueDate day) equals the measured max(served, IssueDate day), and the same 508 notices are re-dated. So the 16,344 count carries over to the generalised rule, but only as a sample check, not a corpus-wide one.
- Small count errors: (1) published_offset is NULL on 2 rows, not 1. Both are dateless de-2.1 notices with no published_at (26983293 in fetch 4, 46856314 in fetch 659). (2) The era table leaves out those 2 null rows: monthly 971,970 = 932,731 + 349 + 38,889 + 1, and daily 68,690 = 8,675 + 1 + 60,013 + 1. (3) 'Every de-* notice carries an IssueDate' is true except for the same 2 notices. (4) Fetch 444 was ingested in two passes: 2026-07-21 00:46 (ids from 468,244) and 07-23 18:07 (ids around 26.0M), not only at 18:07. This does not affect the deduplication argument. (5) That fetches 3, 5 and 445 own nothing because monthlies 4 and 402 already owned their notices is an inference, not a measurement.
- Process deviation, no safety impact: several reads covered 20–44 fetches each (q11 about 20 fetches and ~500k notices; q22, q23, q26 and q27 44 fetches each). The rule asks for one fetch per read or a small IN-list of fetch ids. Every arm was still bounded by fetch_id equality, every plan only SEARCHed, the slowest took 3.3 s, and there were no 408s.
- Caveat that does not change any number: a pubDay-D export mostly carries day D−1 (requested dates and IssueDates both; Monday exports hold about 80 notices). So daily-era 'gap 1' and much of the 'gap 2–29' bucket (14,314 notices, e.g. Sunday gap-2 rows) is how the export is cut, not staleness. The notes already treat it as cadence, and (a) leaves it alone. The recommendation holds: (a) does better than (b), which re-dates 98,902 regular notices and would need 444 exempted, and better than (c), which fixes nothing.
- Budget used: 4 /v1/sql reads (v539_r1–r4; each plan checked locally first, about 2 s each, no 408s) and 1 GET (c26100556). requests.log now shows 33 sql and 8 get lines.

## Unit 2 — decided (2026-10-19)

**(a), generalised for `source = doe`.** `published_at` = the latest of every publication-date candidate the notice carries
(OPP-012 / DE1-Publication-PublicationDate, BT-738 / DE1- / SDK01-RequestedPublicationDate) and the IssueDate's civil day in its own
offset. A date-only result stays date-only (367/418 rendering). No other source changes.
- Why (a): the whole multi-day class carries an IssueDate (0 dispatch-less sdk-0.1 rows more than 1 day stale in any regular export), so
  the issue's objection that (a) fixes only ~29% does not hold; (a) fixes 99.95% of the issue's own class and 94% of the 30+-day tail,
  re-dates only real inversions (16,344), pushes nothing past its export window, and leaves the 2022-12 backlog (109,658 consistent old
  dates) and the 1–3-day export cadence alone. In sdk-0.1 it also gives each change-notice version its own day instead of version 1's.
- Not (b): it would re-date all 98,902 regular-export stale rows (68,700 by exactly the cadence day), collapse the 2022-12 backlog onto
  2022-12-01 unless exempted, and a month floor is a bound, not a date. A daily-era `published_at = pubDay` redefinition is a separate
  question, not 539's.
- Not (c) alone: it fixes nothing. Its docs half is done in unit 3 regardless: DÖE `published_at` is the requested (or a stamped) date and
  usually runs 1–3 days before the export day; `Tender.dispatched_at` "earlier than published_at" is false today for 16,344 DÖE notices
  (until the drain) and stays qualified for DÖE afterwards only where a candidate is later than the IssueDate (none measured).
- Exposure: about 23.5% of re-dated versions change order within their tender (~3,800 extrapolated from 4 fetches, 322 of 1,368 measured).
  The 495 compare engine announces each (a D3 row) when the drain's R3 refold re-folds its Tender (Unit 3's drain bullet); the
  dry repair counts the re-dated notices exactly, and the refold's own corrections count the reorders.

## Unit 3 — proposed build

- Resolver (`notice_stamps` / `notice_instants`, crates/ingest/src/project.rs ~925–947, ~10081): for DÖE, take the max over the
  publication candidates present and the IssueDate civil day; keep the rendering flags right (a date-only winner stays date-only; an
  IssueDate-day winner is date-only too). Tests: 26044864 (sdk-0.1: requested 2022-10-19, IssueDate 2023-01-31 → 2023-01-31), 26411995
  (de-1.1: stamped 2023-12-19, requested 2024-07-25, IssueDate 2024-07-25 → 2024-07-25), 27077227 (de-2.1: OPP-012 2025-07-14, BT-738
  2026-07-22, IssueDate 2026-07-22 → 2026-07-22), an sdk-1.x notice (unchanged), a TED notice (unchanged).
- Docs: openapi.json `Tender.dispatched_at` (~971), /docs dates list (docs.rs ~838), a CHANGELOG entry; gate the served sentence.
- Drain after 551 B6's flip and U8, not on a D5 morning that 560 U-d reads (route corrected in unit 3's review, 2026-10-19):
  1. `repair-notice-instants` (doe): dry first, review the by-profile counts (expect ~16,344 plus the inversions ingested
     before the deploy), then wet with the reviewed `expect_rows`. It re-stamps notice rows only: no change rows, and it
     leaves `projected` alone.
  2. An ADR-0017 R3 refold through the existing `refold` kind, with the profiles that re-date and a reviewed `expect`:
     eforms:eforms-sdk-0.1, -de-1.0, -de-1.1, -de-1.2, -de-2.0, -de-2.1 (sdk-1.x already serves the IssueDate). It
     re-queues, stamps the Tenders epoch-stale and queues the trailing `project rebuild=false`, so the compare engine
     re-sorts each chain and writes a D3 row for every Tender with a re-dated version (up to ~16,344 versions, ~3,800
     of them reordered). Cost: a verify-dominated refold over ~1.16M DÖE notices. A tighter scope needs a new
     Tender-id or notice-id stale-stamp step: the stored dry plan keeps only 400 ids (PLAN_CAP) and `refold-notices`
     caps at 1,000 (REFOLD_NOTICES_CAP).
  - **Not `repair-version-instants`.** It rewrites `tender_versions.published_at` / `tenders.current_published_at` in
    place with no change row and never re-sorts `seq`: for 539 that is a silent move of a served date (ADR-0017 D5). Run
    BEFORE the refold it also hides the date moves from the compare (stored == re-derived), leaving only the reorders
    announced. At most run it after the refold, dry, as a check that should report 0 moved.
  - **The pre-drain window.** The fold re-derives every instant from the parse and never reads `notices.published_at`,
    so from the deploy on any fold of a DÖE Tender (a daily incremental, or a refold for another issue) re-dates its
    versions and announces them, while `/v1/notices` keeps the old stamp until step 1 (CHANGELOG and /docs say so).
    The Tender renderers pair the VERSION's instant with the causing NOTICE's stored offset / has_time (store
    read.rs list and detail rows), so in that window a refolded version whose old winner was TIMED renders the new
    day's 00:00Z in the old offset as a time (e.g. `2026-07-22T02:00:00+02:00`); where the old winner was date-only
    (the sdk-0.1 request) it renders the right day. Step 1 clears it, so it runs first.
  - **551.** No deploy hold: the B6 cohort carries 54 Tenders with 104 DÖE versions and the rule changes 0 of them
    (measured on the B6 U2 layer). If 539 deploys before the B6 flip, list it in B6's code-drift ledger (point 5) and
    re-run the zero-drift probe (refold-lot-sum on the 3,502; expect 0 corrected) before U6.
- **Served-docs audit (2026-10-19 15:5xZ, hourly AUDIT):** `/docs` on rev 0787596 serves the 539 Dates caveat ("raised to the day of
  the notice's IssueDate when that is later (issue 539)") and the pre-drain caveat ("Until the issue-539 drain re-dates …").

**Verify re-read (2026-10-19 20:5xZ, hourly AUDIT, rev 8e9222c):** line 1 still `[65023,"2022-10-19","2023-01-31T14:02:48+01:00"]`
(expected: the stored date moves only with the pending drain — repair-notice-instants dry/wet, then the DÖE R3 refold, after
551 U8); line 2 now reads "earlier than published_at, except where a DÖE notice caused the current version … raised to the day
of the notice's IssueDate when that is later (issue 539)" — the served description half of `done` holds.

**551 W-on drain protocol (2026-10-23, `551-lot-sum/b6/decision-w-on-drains.md`).** 539 drains first, from the 10-24 gap,
after that morning's U9 check and the probe P0. Step 1 is class Z. Run it dry, review expect_rows, then run it wet. It passes
when the cursor is unchanged (or one E1 statement shows no tender row) and /admin/jobs lists only its own job. Step 2 is class
W, in three runs with a STOP point between them: (a) sdk-0.1, (b) de-1.0–1.2, (c) de-2.0–2.1. start_rule must say GO for each
run. All three close before the 10-25 02:10Z weekly tick; otherwise run c moves to 10-25. Never run between the 10-28 chain
and the day-7 fetch. Before the runs, record the C9' discharge: R is at 539's fixed point (probes 2656–2658 and 2677–2679,
then P0), so 0 served moves are predicted. Also record the cap max(2·A·1.0974 %, A·1.0974 % + 10), the salt
`551-b6-drain-539-` and a writer-lock line. Per run: gd5 class W, plus A from a dry refold-lot-sum. NEW heads are read in full
if ≤ 30, otherwise a salted 30 under U5's sequential bar; over the cap is a STOP. MOVED, LOST or SILENT is a STOP, followed by
the port on refetched facts; wrong, or unexplained by the end of the gap, means switch off. seq/published_at moves are NOTEs;
EUR-only re-date moves are explained (899272 and 5842187 under the 2 % rule). Cost: about 13–21 statements plus 4–12 dry jobs,
about 3–4 h. **Next step:** besides 551 U8, the drain now waits for 551's kit (E1–E6 and the start_rule DST/weekly-tick fix),
that morning's U9 check and P0. 579 needs nothing from it.
- **579's pre-registration applies to step 2:** 408007, 413501, 454861, 179248, 340956, 823340 (all hold eforms-de versions)
  are expected to read corrected with order-only changes and no value move; 179248's v_awards buyer flips 23052423 →
  13025902 (579 U3). Not a fire, not a STOP.
- **Writer: 551 z539-1 until 2026-10-24T01:30Z** (539 step 1, repair-notice-instants, class Z; opened 2026-10-23 23:50Z at CB = K = 959192253).
- **Writer: 551 w539-a until 2026-10-24T02:30Z** (539 step 2 run a, refold eforms-sdk-0.1, class W; opened 00:05Z at CB = K = 959192253).
- **Writer: 551 w539-b until 2026-10-24T03:00Z** (539 step 2 run b, refold eforms-de-1.0/1.1/1.2, class W; run a PASS: NEW 0, 0 R moves, A 6).
- **Writer: 551 w539-c until 2026-10-24T03:45Z** (539 step 2 run c, refold eforms-de-2.0/2.1, class W; run b PASS: NEW 0, 0 R moves, A 5, 579's 6 refolded).

## Drain — done (2026-10-23 23:50Z – 2026-10-24 02:35Z, 551 W-on protocol)

Run as 551 W-on segments (`551-lot-sum/b6/u9/segments/`). Each segment was single-writer and advanced the registry only
on PASS.

- **Step 1, z539-1 (class Z):** `repair-notice-instants` source doe. Dry 2731 planned 16,343 (sdk-0.1 10,890; de-2.1 2,487;
  de-1.1 1,359; de-1.2 1,138; de-2.0 469; 16,344 measured in unit 1). Wet 2732 APPLIED 16,343 with expect_rows reviewed
  and 0 skipped. Cursor unchanged, own jobs only: PASS.
- **Step 2, three ADR-0017 R3 `refold` runs (class W, each its own segment, STOP point between):**

  | run | profiles | re-queued notices | Tenders stale-stamped | refold / project | corrected | correction rows | A (dry lot-sum) | NEW | R moves | band |
  |---|---|---|---|---|---|---|---|---|---|---|
  | a (w539-a) | sdk-0.1 | 687,796 | 676,936 | 2733 / 2734 | 9,117 | 18,071 | 6 | 0 | 0 | 29 |
  | b (w539-b) | de-1.0/1.1/1.2 | 218,876 | 126,812 | 2737 / 2738 | 2,211 | 2,372 | 5 | 0 | 0 | 29 |
  | c (w539-c) | de-2.0/2.1 | 255,165 | 147,825 | 2740 / 2741 | 2,196 | 2,565 | 12 | 0 | 0 | 29 |

  Total: 1,161,837 notices, which matches unit 3's ~1.16M. Run c's change rows include 93 `tender/removed` (Tenders merged
  away by re-dated links). None of them is an R member; they entered the registry's retired set. Registry K → 959223268,
  status READY.
- **579's pre-registered six** (408007, 413501, 454861, 179248, 340956, 823340) were refolded in run b as expected: order
  only, no value move.
- **`repair-version-instants` was not run** (see Unit 3's drain bullet).

**Verify (2026-10-24 02:3xZ, rev 512b297):** `GET /v1/tenders/65023` → `[65023, "2023-01-31", "2023-01-31T14:02:48+01:00"]`
(was `2022-10-19`). The served openapi `Tender.dispatched_at` description carries the DÖE IssueDate exception. Both halves
of `done` hold. The only thing left is retiring the pre-drain caveats, which waits on a gated deploy.

**Post-drain audit (2026-10-24 02:55–03:10Z, wf_ee1f8a9b-9b6: an auditor and an adversarial re-tabulator, CONFIRMED;
`539-triage/post-drain/`).** The seven unit-1 per-fetch reads (q03–q09) were re-sent byte-identical (sha256-checked,
plans reused, 7 statements, queue idle, no 408). They cover all 135 baseline fetches and all 128 that had inversions.
- Inversions went from 16,344 to 1. Notices per fetch were unchanged (1,162,607).
- The stale share of regular exports fell from 9.50 % to 8.43 %; notices 30+ days stale fell from 5,669 to 41.
- The 444 backlog is identical row for row.
- The post-drain buckets equal option (a)'s pre-drain prediction (q20–q23) in every cell.
- The one residual is notice 26250418 (de-1.1): date-only 2024-01-17 with a −05:00 offset, which the read's civil-day
  formula shifts to 01-16. It is served as "2024-01-17", the same day as its dispatch, so it is a measurement artifact
  and not an inversion. It also explains the dry run's 1,359 against the 1,360 measured.
- **0 real inversions remain.**

**Chain-order audit (2026-10-24 03:45–04:20Z, wf_41e00ab3-eb4: an auditor and an adversarial skeptic, CONFIRMED;
`539-triage/post-drain-chains/`).**
- **Invariant, from code.** The fold orders versions by (published_at, source rank with TED 1 and others 0,
  publication_id, notice id), ascending: `BucketRow::sort_key`, matching `next_plan_batch`'s ORDER BY. seq = i + 1.
  The served detail lists versions `ORDER BY seq`. dispatched_at is never a tie-breaker.
- **Statements.** 7 bounded statements, each a PK range of ≤ 5,000 tender ids over 5 windows. They pinned
  `INDEXED BY sqlite_autoindex_tender_versions_1`; plans are SEARCH only; queue idle; no retry.
- **Result.**
  - 23,574 live Tenders, 6,203 of them with a DÖE version; 26,639 adjacent pairs, 12,798 of them DÖE.
  - 0 violations, 0 seq gaps, 0 head mismatches.
  - Stored instant vs notice instant: 0 mismatches over 22,629 versions.
  - 3 API spot checks are in order.
- **Skeptic's correction.** The 93 Tenders of window w1 (8,860,949–8,861,041) are NEW: run c created them when it
  retired 93 Tenders whose notices regrouped (issue 93's retire-and-mint; 8811451 now answers 404). So the evidence
  for re-sorts below the head is 21 Tenders, 103 pairs (97 DÖE), all in order.
- **Not measured.** The ~3,800 reorder estimate was not verified: version-level change rows came on only 470
  Tenders across the three runs, 93 of them new. Folds between the 10-19 deploy and the drain had already applied the
  new dates to many chains.
- A served-docs gap may exist: no sentence says a Tender id can be retired on regroup. It is being checked
  adversarially before filing.

