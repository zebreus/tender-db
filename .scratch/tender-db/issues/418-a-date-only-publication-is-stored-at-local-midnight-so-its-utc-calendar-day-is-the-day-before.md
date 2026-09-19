# 418 — a date-only publication is stored at its LOCAL midnight, so every UTC day boundary in the corpus (bounds, `/v1/sql` day and year grouping, the sort column) puts it on the day before

Status: ready-for-agent — **DRY PASSES RUN 2026-09-19 03:49–03:59 UTC (queue ids 1488/1489, see the foot): 2,949,915 `shifted` + 11,563,185 `unstamped` parsed notices, 0 planned, 8 min; the version pass counts all 14,513,100 versions as `notice_unstamped` in 34 s, as designed. The wet campaign stays gated on Lennart's go-ahead.** **Unit 1 BUILT, gated (127/127) and DEPLOYED 2026-09-19 01:18 UTC at `ced082d`** (see the foot): a date-only publication/dispatch instant anchors at its civil day's UTC midnight at the resolver, the renderer prints the UTC date, and `repair-notice-instants` streams the standing rows' shift as a mechanical `shifted` class (unit 2's notice side). **Unit 1 ACCEPTED LIVE 2026-09-19 07:48 UTC on the first tick's ingest** (see the foot): the DÖE rows ingested that morning serve bare dates and a bare-date window finds them on their civil day — 74 in `2026-09-18..19` (73 bare `2026-09-18`, 1 timed), 200 in `2026-09-17..18` (197 bare `2026-09-17`, 1 `Z`, 2 timed at `+02:00` whose UTC instant is 09-17), none across the boundary. **Unit 2b BUILT, gated (128/128) and DEPLOYED 2026-09-19 02:12 UTC at `507ca83`** (see the foot): `repair-version-instants` makes every version say what its repaired notice says and re-derives the head column, following only notices that carry the pair, so it cannot run ahead of the notice repair. **Open: the two gated wet runs** — `repair-notice-instants` wet, then `repair-version-instants` dry → wet, back to back (Lennart's go-ahead, dry first); until then the standing rows sit at local midnight and the Verify block reads its open state. **Unit 3 (date-only DEADLINES) is CLOSED as measured-moot 2026-09-19** — see the foot: submission deadlines are timed on 100 % of the eForms/DÖE rows in two 50k-notice windows, and the only date-only ones are legacy r208 rows at offset zero (1,249 of 21,227 in a 50k window), which already sit at their civil day's UTC midnight; no anchoring change is owed on that axis. Was: filed 2026-09-18 23:5x UTC by the hourly audit (step 3) from issue 367 unit 4's candidate, after a bounded measurement showed the class is not a corner: essentially EVERY publication date in the eForms/DÖE era is date-only with a positive offset (three windows below). The decision is TAKEN here — option (i), civil UTC midnight for the publication/dispatch axis — and the units are cut.
Kind: defect (instants — the publication/dispatch axis's stored instant; the bare-date bound, `/v1/sql` day/year grouping and `tenders.current_published_at` all read the UTC day, which is the civil day minus one for a positive offset)
Relates to: 367 (unit 3 rendered the civil DATE correctly by carrying the offset/precision pair; this is the instant beneath it, named there as the candidate unit 4 and re-scoped twice — this issue is that unit), 216 (`sort=published_at` and the published bounds ride `current_published_at`), 50 / 239 (`/v1/sql`, whose `strftime('%Y', published_at, 'unixepoch')` idiom is documented in `EPOCH_NOTE`), 386 (FTS: `uk_zone` supplies +00/+01 to date-only values, the same shape), ADR-0013 D3, CONTEXT.md:139 ("timestamps as UTC + original offset")
Blocked by: nothing

## Verify

    B=https://tenders.zebreus.click; for w in "2026-09-05&published_before=2026-09-06" "2026-09-04&published_before=2026-09-05"; do curl -s --max-time 30 "$B/v1/tenders?published_after=$w&source=doe&limit=200" | python3 -c "import json,sys; d=json.load(sys.stdin); print('$w'.split('&')[0], len(d['items']), 'items')"; done

- **done**: the first line (the civil day 2026-09-05) counts the DÖE tenders published that day and the second (2026-09-04) does not carry them — a bare-date window finds a publication by the date the API serves for it
- **open**: `2026-09-05 0 items` then `2026-09-04 N items` — the tenders the API serves as published on 09-05 sit in the 09-04 window, because their instant is 2026-09-04T22:00:00Z (read 2026-09-18 23:5x UTC at `5d245cb`: `2026-09-05 0 items` then `2026-09-04 45 items`)

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

## Unit 1 BUILT 2026-09-19 (`3deda36` + `ced082d`, gate 127/127, deployed 01:18 UTC on an idle queue) — and unit 2's notice side with it

**The rule, where it lives.** `ingest::project::notice_stamps` anchors a `has_time = false`
instant at `utc + offset × 60` — the civil day's 00:00Z — on the publication and dispatch axis
only (`civil_day`). The parse layer and `tender_version_dates` are untouched: the detail's
`dates` array still shows what the source said, and a date-only DEADLINE keeps its local-midnight
storage (unit 3, not decided here). Both the notice row and the version resolve through this one
function, so the invariant `notices_and_their_versions_carry_the_same_instants` holds unchanged,
and the fold's own order key (`published_at`) moves with it.

**What a consumer sees, from the next daily tick.** A date-only publication renders as its date
(367 unit 3's rendering, now off the UTC date — a negative offset would otherwise print the day
before); `published_after=D&published_before=D+1` finds it on `D`; `/v1/sql`'s
`strftime('%Y-%m-%d', published_at, 'unixepoch')` groups it under `D`; `sort=published_at` orders
civil days without an offset skew. The bare-date bound (unit 3's midnight-UTC rule) is right by
construction. Pinned end to end on the DÖE fixture: found by the 16th, not by the 15th.

**The standing rows — the repair's third class.** `repair-notice-instants` now classifies a
stored instant that is the local-midnight form of the resolver's civil one (`to = from + offset ×
60`, `has_time = false`) as `shifted`: mechanical like `unstamped` (the civil day is unchanged,
only its anchor moves), streamed with the pair as the walk goes, never planned, counted on its own
line. The streaming statement writes all six instant columns, guarded per row on the instants it
was read with. A dry run on the corpus will report ~13M `shifted` (the eForms/DÖE era; MEASURED 2,949,915 on 2026-09-19 — the prediction counted eras that publish no offset, see the dry passes at the foot) beside the
`unstamped` legacy rows and a plan of ~0. **Unit 2b, before any wet run:** the version side —
`tender_versions.published_at` / `dispatched_at` follow their notice by `caused_by_notice_id`,
then `tenders.current_published_at` re-derived from the head — is not built yet; a wet notice-side
run alone would break the notice/version agreement on the standing rows. The wet run is the
gated production write; it needs Lennart's go-ahead, dry first.

**The golden snapshot moved by exactly the rule.** Ten `tender_versions.published_at` values, each
by its offset (+3600 in winter, +7200 in summer), to a UTC midnight; `dispatched_at` (timed)
unchanged; no other section touched. Regenerated after reading that diff — the change IS the
semantic, not a drift.

**Tests.** `process.rs`: +02:00, +00:00 and −05:00 date-only values all anchor at the same
civil midnight, a timed instant is exact and untouched, the DE-1.x specimen's two dates. The
repair suite (10): `date()` is now a timed value so the plan classes keep their meaning,
`date_only()` carries the shift case — `shifted` 1 on the dry run, written on the wet, `agree` on
the next walk. `api.rs`: the DÖE date-only tender on every surface, the bare-date windows, the
unstamped fallback at `2026-07-16T00:00:00Z`. Docs, the OpenAPI `published_at` description and the
`/v1/sql` epoch note say the rule and the standing-row caveat.

**Read after the deploy (01:18 UTC):** the Verify block still reads `2026-09-05 0 items` / `2026-09-04 45 items` and 7954578 still serves `2026-09-04T22:00:00Z` — the standing rows are unchanged by design (no pair, local midnight) until the gated repair; the first rows under the rule arrive with the 07:35 UTC daily tick, when a DÖE tender published on the 19th must answer to `published_after=2026-09-19&published_before=2026-09-20` — the owed read.

## Unit 2b BUILT 2026-09-19 (`507ca83`, gate 128/128, deployed 02:12 UTC on an idle queue) — `repair-version-instants`

**What it does.** Walks `tender_versions` in bands of 25,000 tender ids, reads each version's
causing notice by primary-key seek (500 ids per statement), and makes the version's
`published_at` / `dispatched_at` say what the notice says, then re-derives
`tenders.current_published_at` from the head version for every tender it touched — in the same
transaction, sliced at 20k rows with a checkpoint between slices, each write guarded on the two
instants the row was read with.

**It follows only a notice that carries the pair** (`published_offset IS NOT NULL`), i.e. one the
notice repair has been through. A version behind an unrepaired notice is counted as
`notice_unstamped` — never as agreement — so the job cannot report "done" ahead of the notice
repair, and its summary says which to run. The order is not cosmetic: the list renderer combines
the VERSION's instant with the NOTICE's pair, so a stamped notice beside an unmoved version renders
a date-only publication a day early. **Campaign order, both gated (Lennart's go-ahead, dry first):
`repair-notice-instants` wet → `repair-version-instants` dry → wet, back to back.**

**No plan, two passes.** The moved set is the whole eForms/DÖE era (~13M versions — measured 2026-09-19: the shifted notices are 2,949,915, so expect that order of versions; the dry run
will say); a per-row plan would not fit and holds nothing for a reviewer — every write is "the
version says what its notice says". So the dry run counts; the wet run counts again, aborts if
the count is outside the notice repair's max(2 %, 5) tolerance of the reviewed dry figure, then
walks a second time writing. Stoppable between bands and between slices; the committed prefix
stands and a re-run finds the rest. No change events: the civil date a stamped row serves does
not change, only the instant beneath it (and `sort=published_at`'s order among civil days,
which is what the change is for).

**Tests** (`version_instant_repair.rs`): the campaign shape — a repaired notice's version is
moved and its tender's head column follows, an unrepaired notice's version is skipped and counted,
an agreeing version is left alone, a second run finds nothing; the wet gate's abort; a stop.
Supervisor: the `repair-version-instants` admin kind, dry by default, wet gated on the stored dry
report's `moved`; in the stoppable list (its test updated).

**Open on this issue after unit 2b:** the two gated wet runs (unit 2 proper), then the extent
re-measured with `published_at % 86400 = 0` as the civil-midnight signature, and the Verify block
turning; unit 3 (date-only deadlines) stays a named non-goal. Owed read after the 07:35 UTC tick:
a DÖE tender published on the 19th answering to `published_after=2026-09-19&published_before=2026-09-20`.

## Unit 3 — MEASURED 2026-09-19 02:5x UTC and closed as moot: date-only deadlines do not carry an offset

The question was whether a date-only `submission_deadline` (stored, like every date-only value,
at the publisher's local midnight in UTC) needed the same civil-midnight anchoring — or an
end-of-day one. Three bounded `notice_dates` windows, the deadline fields only,
`(field, has_time, sign(offset), rows)`:

| window | rows | timed | date-only |
| --- | --- | --- | --- |
| 26.20–26.25M (DÖE) | 19,023 | SDK01 …TenderSubmissionDeadlinePeriod-EndDate 18,923 + BT-131(d)-Lot 100, all `+` | **0** |
| 27.30–27.35M (TED eForms) | 6,515 | BT-131(d)-Lot 6,052 `+` / 168 `0` / 4 `−`, BT-1311(d)-Lot 283 `+` / 7 `0` / 1 `−` | **0** |
| 17.40–17.45M (legacy r208) | 21,227 | TED-RECEIPT_LIMIT_DATE 19,978, offset `0` | 1,249, offset `0` |

So the class this unit would have moved — a date-only deadline with a non-zero offset — is
**empty** in both eForms-era windows, and the legacy era's 1,249 date-only deadlines (5.9 % of
its window) carry no offset at all: their local midnight IS the civil day's UTC midnight, the
one instant every reading agrees on. There is nothing to anchor. The end-of-day question (is "by
2026-09-19" the start or the end of that day?) survives only for those legacy rows, whose
deadlines closed years ago and never meet `?status=open` or `deadline_after=now`; the row serves
the date the source published, which is the honest answer, and `stamp()` keeps rendering it as
a date. Closed as measured, not decided: the measurement says the decision has nobody to apply to.

What this leaves on 418 is exactly the campaign: the two gated wet runs, then the extent
re-measured and the Verify block turning.

## Dry passes RUN 2026-09-19 03:49–03:59 UTC — the campaign is sized, nothing written

Both dry runs at the foot's campaign order, queue idle, well before the 07:35 tick (cite the QUEUE
id — `job_id` on a `GET /admin/jobs` recent row; the log rows are 2397/2398).

**`repair-notice-instants` DRY, queue id 1488** — 03:49:25 → 03:57:38, **8 min 13 s** for the whole
corpus. Stored as report `notice-instant-repair`, `computed_at` 1789790258:

| class | notices | what the wet run does with them |
|---|---:|---|
| walked (`parse_state = parsed`) | 14,513,100 | |
| `unstamped` — instants agree, pair missing (367 unit 3) | 11,563,185 | writes the four pair columns, instants untouched |
| `shifted` — date-only instant at local midnight (418) | 2,949,915 | writes all six columns: civil UTC midnight + pair |
| `agree` — instants agree AND pair present | 0 | nothing (none is stamped yet) |
| planned (`epoch_published` / `null_published` / `resolver_silent`) | 0 | no plan, no gate to pass |

So the wet run is a **14.5M-row mechanical UPDATE campaign**, streamed as walked, guarded per row
(`WHERE id = ? AND published_at IS ? AND dispatched_at IS ?`), no plan and therefore no
`expect_rows` tolerance to review. Its wall time is not the dry run's: the dry pass reads the parse
layer and writes nothing, the wet pass adds one guarded UPDATE per row (367's job 799 wrote 226k rows
at a rate this record does not hold — it will be read off the wet run's own summary).

**The ~13M prediction was wrong by 4.4×, and the reason is worth keeping.** `shifted` requires
`has_time = false` AND a non-zero offset. Only the eForms era publishes a date with the publisher's
offset: the DQ report's era table (section 1, versions) has eforms-de 457,683 + eforms-sdk 2,359,403 =
2,817,086, which is the shifted class to within the notice/version difference. TED_EXPORT r2.0.9
(4,490,549), r2.0.8 (2,699,213), the text era (3,786,955) and the DÖE sdk-0.1 island (676,287, literal
`Z`) publish a bare date, offset zero, so their local midnight IS the civil UTC midnight and they are
`unstamped`, not `shifted`. The moved set is the eForms era, ~2.95M notices, and the version pass
should count `moved` of that order once the notices carry their pair.

**`repair-version-instants` DRY, queue id 1489** — 03:58:56 → 03:59:30, **34 s** (bands of 25,000
tender ids, notices by PK seek). Stored as report `version-instant-repair`, `computed_at` 1789790370:
`walked` 14,513,100, `notice_unstamped` 14,513,100, `agree` 0, `moved` 0, `heads_recomputed` 0. That
is the designed answer before the notice repair: the job follows only notices that carry the pair
(`published_offset IS NOT NULL`), so it cannot move a version ahead of its notice, and its summary
says so in words ("run repair-notice-instants (wet) first, then this again"). The timing is the useful
number: a full count pass is half a minute, so the wet run's first (count) pass and its gate on the
reviewed dry `moved` figure cost nothing to repeat.

**What the campaign now looks like, in order, all gated on Lennart's go-ahead:**

1. `repair-notice-instants` **wet** — 14.5M rows, every one mechanical; the per-row guard makes it
   restartable and the daily tick may run beside it (the tick's new notices resolve with the pair
   already, and a row the tick rewrites fails the guard and is simply skipped, `skipped_moved`).
2. `repair-version-instants` **dry** — expect `moved` ≈ the versions behind the 2,949,915 shifted
   notices, the rest `agree`; `notice_unstamped` 0.
3. `repair-version-instants` **wet** — counts again, gates on the stored dry `moved` within
   max(2 %, 5), writes in 20k slices, re-derives `current_published_at` per touched tender.

After 1 the Verify block above still reads its open state (the versions serve the window); after 3
it reads done. 7954578 (367's probe) stops inverting after step 1 alone — the tender's own
`published_at` is rendered from the notice pair.

## Unit 1 ACCEPTED LIVE 2026-09-19 07:48 UTC — the first tick's ingest, read through bare-date windows

The 07:35 tick ingested `doe daily 2026-09-18` (1,046 notices) on rev `110d527` (unit 1 + 2b in the
build). `GET /v1/tenders?published_after=<day>&published_before=<day+1>&source=doe&limit=200`:

| window | items | `published_at` shapes |
|---|---:|---|
| 2026-09-18 .. 19 | 74 | 73 × bare `2026-09-18`, 1 × timed (`2026-09-18T08:21:08+02:00`) |
| 2026-09-17 .. 18 | 200 | 197 × bare `2026-09-17`, 1 × `2026-09-17T12:38:57Z`, 2 × `2026-09-18T00:00:05+02:00` |

No bare 09-18 row sits in the 09-17 window and no bare 09-17 row in the 09-18 window; the two
`+02:00` rows at five seconds past local midnight are timed instants whose UTC instant IS 09-17
22:00:05, correctly inside the 09-17 window. So on a fresh ingest a date-only publication is served
as the date the source stated and found by that date — the issue's `## Done when` for new rows.
The standing rows (the Verify block above, 09-04/09-05) keep their local-midnight instants until
the gated wet campaign runs; nothing about today's tick moves them.

(The package `doe daily 2026-09-18` evidently carries notices published on 09-17 as well as 09-18 —
packages are by fetch day, publication dates by the source; 197 + 74 of its 1,046 notices sit in
these two windows.)

## 2026-09-19 09:5x — the wet run's rate could not be recovered then (superseded at 12:15, next section)

For the go/no-go: job 799 (367's wet, 226,293 rows, 2026-09-07) is neither in the persisted job log
(`GET /admin/jobs` keeps 200 rows, back to queue id 1297) nor in the journal (the repair prints no
line). So no measured UPDATE rate exists on this box. What IS known: the dry pass reads the whole
parse layer in 8 min 13 s; the wet pass adds one guarded UPDATE per row — 14.5M of them; the job
is stoppable between rows and every row is guarded by its pre-image, so a stopped run leaves a
consistent corpus and a re-run walks on (`agree` for what it already stamped). The operational
shape is therefore: start it in an evening window, read its `[repair]` progress after ten minutes,
and stop it before 07:00 UTC if it would cross the tick — nothing is lost by stopping.

## 2026-09-19 12:15 — job 799 recovered through the deeper job log; the campaign's wall time is bounded

`GET /admin/jobs?limit=` now serves 2,000 rows (`0e4436f`, deployed 12:13 at `44d57de`), and job
799 is in them: `repair-notice-instants (issue 367, WET)`, 2026-09-07 20:58:07 → 21:02:20, **233 s**
for 14,346,064 notices walked and **226,293 rows written** (the dry run before it, job 798, took
471 s for the same walk with no writes). So the whole wet pass — walk plus 226k guarded UPDATEs —
cost less than the dry walk alone; the per-row write cost is bounded above by 233 s / 226,293 ≈
1.0 ms and is certainly far lower, since most of the 233 s is the walk.

For the 418 campaign's 14,513,100 guarded UPDATEs that gives a **ceiling of ~4 h** (every row at the
1 ms upper bound, plus the 8-minute walk) and a plausible **~30–60 min** (writes at the rate the
226k rows suggest once the walk is subtracted). Either way it fits an evening window — start after
20:00 UTC, and it is stoppable between rows with every row guarded, so a run that threatens the
07:35 tick is stopped, not raced. The version pass after it re-counts in ~34 s and writes in 20k
slices. The go/no-go stands as stated at the head of this record; this section only prices it.
