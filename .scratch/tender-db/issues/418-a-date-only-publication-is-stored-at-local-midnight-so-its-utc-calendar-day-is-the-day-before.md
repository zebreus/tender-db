# 418 — a date-only publication is stored at its LOCAL midnight, so every UTC day boundary in the corpus (bounds, `/v1/sql` day and year grouping, the sort column) puts it on the day before

Status: ready-for-agent — filed 2026-09-18 23:5x UTC by the hourly audit (step 3) from issue 367 unit 4's candidate, after a bounded measurement showed the class is not a corner: essentially EVERY publication date in the eForms/DÖE era is date-only with a positive offset (three windows below). The decision is TAKEN here — option (i), civil UTC midnight for the publication/dispatch axis — and the units are cut; unit 1 (the resolver, renderer, tests) is buildable now, unit 2 (the standing rows) rides the gated repair. Unit 3 (date-only DEADLINES) is a named non-goal with its own question.
Kind: defect (instants — the publication/dispatch axis's stored instant; the bare-date bound, `/v1/sql` day/year grouping and `tenders.current_published_at` all read the UTC day, which is the civil day minus one for a positive offset)
Relates to: 367 (unit 3 rendered the civil DATE correctly by carrying the offset/precision pair; this is the instant beneath it, named there as the candidate unit 4 and re-scoped twice — this issue is that unit), 216 (`sort=published_at` and the published bounds ride `current_published_at`), 50 / 239 (`/v1/sql`, whose `strftime('%Y', published_at, 'unixepoch')` idiom is documented in `EPOCH_NOTE`), 386 (FTS: `uk_zone` supplies +00/+01 to date-only values, the same shape), ADR-0013 D3, CONTEXT.md:139 ("timestamps as UTC + original offset")
Blocked by: nothing

## Verify

    B=https://tenders.zebreus.click; for w in "2026-09-05&published_before=2026-09-06" "2026-09-04&published_before=2026-09-05"; do curl -s --max-time 30 "$B/v1/tenders?published_after=$w&source=doe&limit=200" | python3 -c "import json,sys; d=json.load(sys.stdin); print('$w'.split('&')[0], len(d['items']), 'items')"; done

- **done**: the first line (the civil day 2026-09-05) counts the DÖE tenders published that day and the second (2026-09-04) does not carry them — a bare-date window finds a publication by the date the API serves for it
- **open**: `2026-09-05 0 items` then `2026-09-04 N items` — the tenders the API serves as published on 09-05 sit in the 09-04 window, because their instant is 2026-09-04T22:00:00Z (read 2026-09-18 23:5x UTC at `5d245cb`: 0, then 200 with `more`)

## Observed (2026-09-18, prod)

A date-only publication `2026-09-05+02:00` is stored as `2026-09-04T22:00:00Z` — its civil day's
LOCAL midnight re-expressed in UTC (the parse layer's `timestamp()` = `days_from_civil × 86400 −
offset`). Issue 367 unit 3 made the API RENDER it as `2026-09-05` by carrying the offset; the
instant beneath is unchanged, and every UTC day boundary reads it as the 4th:

- `/v1/tenders?published_after=2026-09-05&published_before=2026-09-06&source=doe` → **0 items**;
  the same tenders answer to `published_after=2026-09-04&published_before=2026-09-05` (200 with
  `more`, e.g. 7954578, served `2026-09-04T22:00:00Z` today and `2026-09-05` once its pair is
  stamped). A consumer asking for "published on the 5th" by the date the API shows gets nothing.
- `/v1/sql`: `strftime('%Y-%m-%d', published_at, 'unixepoch')` — the documented idiom — groups the
  same rows under `2026-09-04`; `strftime('%Y', …)` puts every 1 January publication with a positive
  offset in the previous YEAR. Any dashboard or client grouping by UTC day is one day early for
  this whole class.
- `tenders.current_published_at` (the `sort=published_at` column) carries the shifted instant;
  the ORDER is right within one offset, the day boundary is not.

**Extent — three bounded `notice_dates` windows (50k notice ids each, the publication fields
only, `(field, has_time, sign(offset), rows)`):**

| window | rows | date-only | positive offset | zero offset |
| --- | --- | --- | --- | --- |
| 26.20–26.25M (DÖE) | 49,966 | **all** | DE1-RequestedPublicationDate 23,494 + SDK01-RequestedPublicationDate 22,076 + DE1-Publication 24 | 2,396 + 1,879 + 97 |
| 27.30–27.35M (TED eForms) | 4,954 | **all** | OPP-012 3,752 + BT-738 974 | BT-738 228 |
| 29.50–29.55M (recent mix) | 557 | **all** | SDK01 335 + BT-738 220 + OPP-012 1 | 1 |

Every publication date is a DATE (`has_time = 0` on 100 % of rows in all three windows), and ~92 %
of them carry a positive offset — including TED's own `OPP-012-notice` (3,752 of 3,752 in the
TED window; the sdk-0.1 "literal Z" relaxation covers only the minority that publish no offset).
So the shift is the RULE for the eForms and DÖE eras, not a dialect corner. The legacy eras
(r209/r208/text) publish no offsets and are stored at UTC midnight already — the day boundary is
right there, which is why the year-grouping idiom never looked wrong on the old corpus.

## Why, exactly

A publication date is a CALENDAR DAY, not an instant. The parse layer represents a date-only value
as "local midnight in the publisher's zone" and then subtracts the offset — a faithful UTC instant
for that midnight, and the wrong anchor for a day: the day the publisher named starts at 00:00 in
THEIR calendar, and the only instant that puts it on that day under every UTC-based reading is the
civil day's UTC midnight. 367 unit 3 fixed the rendering (the pair recovers the civil date); the
bound parser (`YYYY-MM-DD` = UTC midnight), `/v1/sql`'s `strftime`, and the sort column have no
pair to consult and read the shifted instant as-is.

## Decision — option (i), on the publication/dispatch axis, at the resolver

Store a date-only publication or dispatch instant at the UTC midnight of its civil date, keeping
the published offset beside it (the pair from 367 unit 3). Concretely, in `notice_stamps`: for a
`Date { has_time: false, offset_minutes: o }`, `utc_seconds += o × 60` — local midnight plus the
offset IS the civil day's UTC midnight. The parse layer and `tender_version_dates` are untouched;
the change is where the notice row's and the version's instants are RESOLVED, which is the one
function both layers share (the invariant `notices_and_their_versions_carry_the_same_instants`).
The renderer for these two fields then prints the UTC calendar date for `has_time = false`
(`json::published`), not the local one — a negative-offset date-only value (rare, non-EU) would
otherwise print the day before.

Why not the parse layer: `timestamp()` also serves deadlines, award dates and periods, and a
date-only DEADLINE is not the same question (unit 3). Why not the bound parser: a bare-date bound
"minus 14 h" would over-include, and `/v1/sql` and the sort column would stay wrong. Why not
"document it": the measurement says it is the whole era.

## Units

1. **The resolver and the renderer.** `notice_stamps` applies the civil-midnight rule for
   `has_time = false`; `json::published` renders `has_time = false` as the UTC date; the bare-date
   bound stays UTC midnight (now correct by construction); the `/v1/sql` `EPOCH_NOTE` says the day
   grouping is the civil day for date-only publications. Tests: the resolver on `+02:00`, `+00:00`
   and `-05:00` date-only values (UTC midnight of the civil day in all three); the API test from
   367 unit 3 (`a_date_only_publication_is_served_as_a_date…`) extended with the bare-date window
   finding the tender on its own day and NOT the day before; the sort test's lexicographic
   comparison stays valid. New ingests are right from the next tick after the deploy.
2. **The standing rows — the gated repair, extended.** `repair-notice-instants` gains a third
   class beside `planned` and `unstamped`: a row whose stored instant differs from the resolver's
   by EXACTLY the date-only civil shift (`to == from + offset × 60`, `has_time = false`) is a
   mechanical rewrite, streamed like `unstamped` (no plan — nothing for a reviewer to weigh) and
   counted (`shifted`). The version side needs the same move: `tender_versions.published_at` /
   `dispatched_at` and `tenders.current_published_at` follow their notice (a streaming UPDATE from
   the notice row by `caused_by_notice_id`, in slices, then `current_published_at` re-derived from
   the head version) — either as a second phase of the same job or as its own job; `tender_version_dates`
   is left alone (it stores the parse layer's instants, and the fold's fold-order key is
   `published_at`, which the same repair re-derives). This is a production write of the
   `Modify Shared Resources` class the operating session's classifier refuses; it needs Lennart's
   go-ahead, dry first.
3. **Non-goal, on record: date-only deadlines.** `submission_deadline` from a date-only value is
   stored the same way (local midnight − offset), and `?status=open` / `deadline_after=now` read it
   as an instant. The honest meaning of "by 2026-07-16" is the END of that day, not its start —
   neither storage answers that. A separate decision (end-of-civil-day for `has_time = false`
   deadlines? and what `stamp()` prints), not this issue's.

## Done when

- `/v1/tenders?published_after=D&published_before=D+1` finds every tender the API serves as
  published on `D`, for a DÖE date-only day (the Verify block), on new ingests after unit 1 and on
  the standing corpus after unit 2.
- `/v1/notices/<id>` and `/v1/tenders/<id>` keep serving the civil date (367 unit 3's rendering)
  under the new instant, including for a negative-offset date-only value (test).
- `SELECT strftime('%Y-%m-%d', published_at, 'unixepoch') …` over a DÖE day window groups the rows
  under the day the API serves.
- The invariant test still holds: the notice row and its version carry the same instants.
- The extent table above is re-measured after unit 2 with `published_at % 86400 = 0` as the
  signature of a civil-midnight instant.
