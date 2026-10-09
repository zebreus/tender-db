# 504 — same-day publications are converted at the previous day's rate, and nothing re-derives them

Status: ready-for-agent — filed 2026-10-09 from issue 495 unit 5's prod validation (`rederive-eur` job 2099).
Kind: correctness / ADR-0014 money (`fetch-rates`, the daily pipeline, `rederive-eur`)
Relates to: 495 (unit 5 made `rederive-eur` announce, ADR-0017 D5), 306 / 375 / 490 (the walk, the stamp, the
re-queue), ADR-0014 D5 (missing days resolve to the nearest PREVIOUS business day)

## What was seen

Job 2099 was the first `rederive-eur` in at least the last 400 jobs. It took 598 s and moved **73,139 of
260,656,611 money rows (0.028%) on 845 Tenders**. It announced them with 4,985 correction rows and re-queued
5,241 notices. The trailing fold (job 2100, 52 s) corrected 828 of those Tenders with 4,867 more rows. Every
money table compared identical there: the walk derives what the fold derives. Only the elected lot values
(569 `tender_version_lots` tables) and heads moved. The moved rows sit on RECENT versions, including versions
of old Tender ids. (A mid-walk extrapolation of "about 6%" was wrong: the low Tender ids carry many recent
versions.)

Example, tender 2793:
- seq 2, published 2026-10-01: 845,557,750 RON cents. After the walk it reads 160,052,574 EUR cents, which is
  rate 5.2830, the 2026-10-01 fixing.
- seq 1, 2026-06-22: 161,402,945, the 2026-06-22 fixing (5.2388).

## Why (diagnosed from the schedule, not yet traced row by row)

- The daily pipeline runs `fetch-rates` and then `project` at about 07:35 UTC.
- The ECB publishes the day's reference rates at about 16:00 CET. So a notice published TODAY is folded
  before today's rate exists, and ADR-0014's lookup resolves it to the nearest previous day (yesterday).
- The next day's `fetch-rates` loads the real fixing, but nothing re-derives the rows already written.
  `rederive-eur` is manual, and before issue 495 unit 5 it was silent too.
- So a daily publication can carry the previous day's rate, off by the day's move, roughly 0.1–0.5%.
  The Tender's elected head value and the stored lot values inherit it.

## Fix options

1. **A daily scoped `rederive-eur`** after `fetch-rates`, over Tenders whose head was published in the last
   N days (N ≈ 8 covers weekends and holidays). Recent publications are almost always heads, so
   `tenders_current_published` bounds the window to a few tens of thousands of Tenders. It runs the same
   R2 window (ADR-0017 D5 announcements, stamp, re-queue), so the daily `project` re-elects the heads. It
   needs a `since` bound on the walk, and its own watermark or no resume (it is minutes, not hours).
2. **Run the daily pipeline after the ECB fixing.** No: TED's daily package lands in the morning, and the
   fold should not wait half a day.
3. **Leave it and document** "EUR at the nearest earlier fixing as of the fold". No: the stored value then
   depends on fold timing, which ADR-0014 and E6 rule out, and a refold would silently change it.

Lean: option 1.

## Units

1. Confirm the diagnosis on job 2099's moved rows. For a sample, check that each moved row's version was
   folded on its publication day (`notices.ingested_at` against `published_at`) and that the old value
   equals the previous fixing.
2. A `since` bound on `rederive_eur_window` (or a scoped variant over `tenders_current_published`), and a
   daily pipeline step after `fetch-rates`, with a test that a same-day publication folded before its rate
   existed is re-derived and announced the next day.
