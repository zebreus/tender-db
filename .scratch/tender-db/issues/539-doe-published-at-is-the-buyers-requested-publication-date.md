# 539 — DÖE `published_at` is the buyer's requested publication date; on a share of sdk-0.1 notices it predates the export that published them by weeks to 586 days, so the IssueDate lands after it

Status: ready-for-agent — filed 2026-10-14 from an external data review (verified: verifier + adversarial refute). Next: unit 1, measure `published_at` against the delivering export's period per DÖE profile (S, in-process read).
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
     `repair-notice-instants`, then 418's `repair-version-instants`, then `project`.
   - Under (c): edit crates/app/data/openapi.json:971 and docs.rs, and gate the served sentence (the 370 class).
   - Either way, the OpenAPI sentence is false today for the multi-day rows. Correct it in the same unit, even
     under (a)/(b), until the drain lands.
