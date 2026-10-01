# 477 — FTS holds 90–98 % of each year's notices, the missing ones are on the API, and the dashboard reports the source complete

Status: ready-for-agent — filed 2026-10-01 13:5x UTC from the 342 close-out audit. The first unit is the root cause:
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
