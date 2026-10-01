# 476 — the TED 2026 denominator has no year-end step, and the era summary line serves it without its ‡ date

Status: ready-for-agent — filed 2026-10-01 from the owner's board survey (workflow wf_4eac8781-4d0, verified by an adversarial pass). The first unit is the ‡ and dated hover on the six era summary lines that span 2026 (`ui.rs:673`), ready now; unit 2, the year-end re-vendor, is needs-info until TED's first 2027 daily (`2027-00001`) is in the fetch registry, in the first days of January 2027.
Kind: dashboard presentation (unit 1) and a calendar-driven ground-truth refresh (unit 2). No API field and no stored data is wrong: `/api/dashboard` already serves `published_as_of` on every 2026 row.
Relates to: 396 (DONE 2026-09-18 — decision (b): date the 2026 snapshot instead of refreshing it; its Done-when said "every January repeats this" and scheduled nothing), 400 (DONE 2026-09-18 — the era summary line; its Status line keeps the missing ‡ "recorded there as NOT done, one glyph beyond the unit"), 402 (DONE 2026-09-18 — a 44,600-notice 2026 hole hid under the over-100 % ratio), 06 (the vendored ground truth and its transcription chain from `docs/research/ted-access-channels.md` §6), 189 (the last re-vendor, 1993–1999)

## What is wrong

### Unit 1: the era summary serves the partial-year denominator undated

The per-year Published cell renders the denominator with its mark and its dated hover
(`crates/app/src/ui.rs:690-692`: `published_cell(row.published)` + `published_mark(row.published_as_of)`,
title `published_note(row.published_as_of)`). The collapsed era summary is a second call site for the same
denominator and calls neither helper (`ui.rs:671-673`): its title is `era_cover_note(era.shared)` and its text
is `{published_cell(era.published)} published{era_mark(era.shared)} · {coverage_pct(era.ratio, era.partial)}`.
It cannot, because `CoverageEra` (`ui.rs:737`) has no as-of field: `coverage_by_era` sums `row.published` and
ORs `row.partial`, and drops `row.published_as_of`.

Live, 2026-10-01 13:11 UTC, rev `9b44528` (`/health`), `GET /`. The six TED eras that span 2026 all carry 2026's
497,791 inside their denominator, all carry the `*`, and none carries a ‡:

| era summary (collapsed) | years in the denominator |
| --- | --- |
| `ted · eforms:eforms-sdk-1.14` — `128 632 held · 497 791 published † · 136.17 % *` | 2026 |
| `ted · eforms:eforms-sdk-1.13` — `862 248 held · 1 368 940 published † · 113.15 % *` | 2025–2026 |
| `ted · eforms:eforms-sdk-1.12` — `411 445 held · 2 170 384 published † · 108.30 % *` | 2024–2026 |
| `ted · eforms:eforms-sdk-1.11` — `111 018 held · 2 170 384 published † · 108.30 % *` | 2024–2026 |
| `ted · eforms:eforms-sdk-1.10` — `262 103 held · 2 966 064 published † · 106.07 % *` | 2023–2026 |
| `ted · eforms:eforms-sdk-1.7` — `344 518 held · 2 966 064 published † · 106.07 % *` | 2023–2026 |

The same page shows the 2026 per-year cell as `497 791 ‡` six times, each with the hover "Counted through
2026-07-17 and frozen there — the year is still publishing…". The `*` footnote (`ui.rs:718`) says "its published
count is a snapshot (‡, hover for its date)". On a summary line it points at a ‡ that is not there.

This was recorded as owed and then left. 396 found it on its live read and handed it to 400 ("the era summary is
now the only place a partial-year denominator appears undated — recorded on 400"). 400 shipped without it: its
"Still open on this issue after the fix" section says "Not done in this unit", and 400 closed DONE on 2026-09-18.
No open issue tracks it.

### Unit 2: nothing moves the denominator across the year boundary

`crates/app/data/ted-notice-counts.csv:68` is `2026,497791,1,2026-07-17`. The deployed source on the box is the
same (`9b44528`, read 2026-10-01). It is compiled in with `include_str!` (`crates/app/src/coverage.rs:22`,
`crates/ingest/src/bin/verify.rs:48`), and nothing refreshes it at runtime. The 2026 ratio climbs with every
daily:

| read | 2026 held (all profiles) | ratio against 497,791 |
| --- | --- | --- |
| 2026-09-14 (396) | 584,293 | 117.38 % |
| 2026-09-16 (400) | — | 118.74 % |
| 2026-10-01 (`/api/dashboard`, `year_held` / `year_ratio`) | 677,855 | 136.17 % |

That is correct under the ‡ in-year, and 396 chose it on purpose: option (a), re-vendoring, "re-breaks in eight
weeks". This issue does not reopen that decision. What 396 did not do is the January step its own Done-when
named. A grep of `.scratch/`, `docs/` and `ops/` for the CSV's name, `497791`/`497,791`, "re-vendor" and "year-end"
finds the closed issues 06, 15, 189, 396, 400 and 402, the research table itself, and unrelated hits (425's and 457's
turso SDK re-vendor, 342's and `docs/research/uk-fts.md`'s FTS "year-end" ids and pages). There is no open issue and no
procedure. If nothing happens, from 2027-01-01:

- **2026 still reads as an open year.** Its row stays `partial` with `as_of` 2026-07-17. The hover says "the year
  is still publishing" (`ui.rs:864`) and the `*` footnote says "the year is not over" (`ui.rs:718`). Both are false.
- **2027 has no denominator.** `coverage.rs:566-568` finds no row, so every 2027 cell reads `—`. The panel copy
  explains a dash as "any source but TED, or a year outside the reference counts" (`ui.rs:643-644`).
- **`verify` cannot see a 2026 hole, and does not see 2027 at all.** `run_coverage` builds rows only from the CSV's
  years (`verify.rs:340-351`), so 2027 is skipped. `classify` (`verify.rs:147-163`) treats a partial year as a floor
  (`Over` only `if !partial`), so 2026 passes at any held count above 0.98 × 497,791 = 487,835. It holds 677,855
  today (the dashboard's `year_held`; `verify` counts distinct publication ids by id suffix, close to it), so a 2026 hole smaller than ~190,000 notices passes `verify` and reads above 100 % on the page. 402 is that
  shape: 44,600 notices missing under 118.74 %.
- **The tests pin the year by hand.** `coverage.rs:993` (`2026 - 1993 + 1`), `coverage.rs:1002` (total 13,201,520),
  `verify.rs:742` (`2026 - 1993 + 1`), `verify.rs:747` (`year: 2026, expected: 497791, partial: true`). A re-vendor
  has to edit four assertions in two crates. The one-partial-row checks (`coverage.rs:1004`, `verify.rs:749`) are
  already shape assertions and survive a re-vendor that adds a partial 2027.

## Proposed fix

### Unit 1 (ready now): one renderer for a denominator, used at both call sites

- Give `CoverageEra` a `published_as_of: Option<String>`, set in `coverage_by_era` from the row that has one. The
  CSV holds at most one partial year, and `coverage.rs:1004` pins that.
- Move "a denominator with its mark and its hover" into one helper, and use it in the per-year cell and in the
  summary span. The summary then reads `1 368 940 ‡ published † · 113.15 % *`, and its `title` is
  `era_cover_note(..)` followed by `published_note(..)`, so the hover carries the date. With one helper, a third
  call site cannot drop the date the way this one did.
- Test: `an_era_summary_spanning_a_partial_year_carries_its_dated_mark` in the `ui.rs` tests. Fold a 2026 partial
  row (`published_as_of: Some("2026-07-17")`) with a 2025 complete row, and assert that the era carries the date, its
  mark is ` ‡` and its hover names `2026-07-17`. Assert that an era of complete years only has `None` and no mark.

### Unit 2 (needs-info until `2027-00001` is fetched): re-vendor at year-end, and make the next one a data edit

The signal is a value the system produces. `/api/dashboard` → `.pipeline[] | select(.source=="ted") | .fetched_ranges`
reads `[["daily","2026-00124","2026-00190"],["monthly","1993-01","2026-06"]]` on 2026-10-01. When its daily range reaches `2027-00001`, 2026 has stopped
publishing, so its count can be final.

1. Re-take 2026's count with the method its row already uses, the Search API v3 `totalNoticeCount`
   (`publication-date>=20260101 AND publication-date<=20261231`). Record it in `docs/research/ted-access-channels.md`
   §6, which is the chain 06 set up. Then vendor `2026,<n>,0`, with no `as_of`, and a dated `2027,<n>,1,<date>`.
   2026 then gets a two-sided check in `verify`, which is the first check that can find a 2026 hole.
2. Make the re-vendor a data edit. Replace the four year-literal assertions with shape assertions: rows run
   contiguous from 1993; exactly one row is partial, and it is the last; every partial row has an `as_of` and no
   complete row has one. Keep the literal spot checks for closed years only.
3. Make the expiry announce itself, so the January after next does not depend on someone reading this issue. When a
   partial row's year is earlier than the current UTC year, the page says the year has closed and its final count
   is not vendored yet, instead of "still publishing". A TED year with held notices and no CSV row reads "not
   vendored yet" instead of the generic dash. Test: `a_partial_year_past_its_calendar_says_its_count_is_stale`,
   fed a fixed date (e.g. 2027-01-10). Step 3 does not depend on the signal and could be built before January.

## Verify

    curl -s https://tenders.zebreus.click/ | grep -o '[‡ ]*published[ †‡]*·\|Counted through [0-9-]*' | sort | uniq -c

One read covers both units: the marks on the summary lines (unit 1) and the snapshot date the page serves
(unit 2, the last open unit).

- **open** (2026-10-01 13:11 UTC, rev `9b44528`):

        12  published ·
        16  published † ·
         6 Counted through 2026-07-17

  The 12 unmarked lines are the other sources' eras. The 16 `†` lines are the TED eras. The six "Counted through"
  hovers are the per-year 2026 cells only. No summary line carries a ‡.
- **done** (unit 1): a ‡ line with count 6 (` ‡ published † ·`, or ` published ‡ † ·` if the fix puts the mark after
  the word). `published † ·` drops to 10, and `Counted through 2026-07-17` rises to 12, because the six summary
  hovers now carry the date.
- **done** (unit 2, from January 2027): no `Counted through 2026-07-17` line. Any date shown is the 2027 snapshot's,
  and the ‡ lines count the eras that span 2027.
