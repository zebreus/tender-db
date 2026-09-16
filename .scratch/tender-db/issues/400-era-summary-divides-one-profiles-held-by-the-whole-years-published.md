# 400 — the collapsed era summary divides ONE profile's held count by the WHOLE year's published count, so 14 of 28 eras read a coverage that is not theirs (eforms-sdk-1.5 summarises as 0.00 % over year rows that all read 100.00 %†)

Status: ready-for-agent — filed 2026-09-16 from issue 396 unit 1's carve-out ("Adjacent, recorded but NOT part of this unit … either fixed with this or filed as its own issue against 229's shape"); 396's own fix shipped without touching it, so this is the "filed as its own issue" branch, taken because the rendering question below is a real design choice that wants its own measurement, not a rider on a copy fix
Kind: bug (dashboard presentation, `crates/app/src/ui.rs` — `coverage_by_era` and the `summary` line). No data, no API surface and no per-year cell is wrong; the defect is confined to the one line a reader sees before expanding
Relates to: 229 (RESOLVED-VERIFIED 2026-08-18, rev `cc0ef20` — it fixed exactly this arithmetic at the PER-YEAR level and introduced `shared_year()`/`year_held`/`year_ratio` and the †; the era summary it left alone still computes the pre-229 number, and `ted · internal-ojs` reproduces 229's own cited "0.079" to the digit), 396 (RESOLVED 2026-09-16 — unit 1 dated the partial-year denominator and rewrote the `*` note; its Done-when required this line to be decided rather than left open, and this issue is that decision), 06 (resolved — the vendored ground truth `crates/app/data/ted-notice-counts.csv`, which is per YEAR and per source, never per profile: there is no per-profile denominator to divide by, and that is the root of both 229 and this), 189 (RESOLVED-VERIFIED 2026-08-12 — the other above-100 % reading, TED document-number reuse in 1993–1999; unrelated mechanism, but it is why `ted · text`'s year rows sit at 101 % and so why that era's summary is only mildly wrong rather than wildly)
Blocked by: nothing

Issue 229 established the rule: **a year served by more than one profile has no per-profile
denominator**, so the per-year cell shows the WHOLE year's coverage marked `†` rather than that
profile's share. The collapsed era summary was never brought under that rule. It sums `held` over the
era's own years and sums `published` over the same years — but `published` is the whole year's count,
shared with every other profile serving that year. For the 14 `ted` eras whose years are all shared,
the summary is therefore one profile's numerator over fourteen profiles' denominator.

The result is not a rounding difference. It is the number on the only line a reader sees before
expanding, and it contradicts every row underneath it.

## Observed (verified 2026-09-16 on prod, rev `a5db49e` via `/health`)

    curl https://tenders.zebreus.click/

| era summary (collapsed) | its per-year rows (expanded) |
| --- | --- |
| `ted · eforms:eforms-sdk-1.5` — **35 / 1 597 124 · 0.00 %** | `2024 \| 12 \| 801 444 \| 100.00 %†`<br>`2023 \| 23 \| 795 680 \| 100.00 %†` |
| `ted · eforms:eforms-sdk-1.6` — **37 669 / 1 597 124 · 2.36 %** | `2024 \| 32 559 \| 801 444 \| 100.00 %†`<br>`2023 \| 5 110 \| 795 680 \| 100.00 %†` |
| `ted · internal-ojs` — **26 955 / 339 534 · 7.94 %** | `2008 \| 26 955 \| 339 534 \| 100.14 %†` |
| `ted · eforms:eforms-sdk-1.14` — **104 355 / 497 791 · 20.96 % \*** | `2026 \| 104 355 \| 497 791 \| 118.74 % \*†` |
| `ted · eforms:eforms-sdk-1.13` — **809 883 / 1 368 940 · 59.16 % \*** | `2026 \| 383 521 \| 497 791 \| 118.74 % \*†`<br>`2025 \| 426 362 \| 871 149 \| 100.00 %†` |

Every single per-year row in that table is `†`-marked and reads ≥ 100 %. Every single summary above
them reads a shortfall, three of them catastrophic. **`eforms-sdk-1.5` is the clean case: a summary of
`0.00 %` over two rows that both read `100.00 %†`.**

`ted · internal-ojs` is the sharpest evidence that this is 229 recurring rather than a new bug:
26 955 / 339 534 = **0.0794**. Issue 229's own test doc-comment names the numbers it fixed — "which
read as 0.079 and 0.922 before this, a 313k-notice hole that did not exist" — and 0.079 is still on
the page today, one DOM level up from where 229 fixed it.

The full inventory from the live page: **14 of the 28 eras are `ted` with a denominator** (the 13
`doe`/`fts` eras have no ground truth and correctly read `—`; `ted · text`, `ted · internal-ojs`,
`ted · ted-export-r208`, `ted · ted-export-r209` and ten `ted · eforms:*` eras carry a figure). Of
those 14, **the 10 eforms eras and internal-ojs are wholly composed of shared years**, so their
summary ratio has no valid reading at all. `ted · text` (99.58 %) is only mildly affected — one of its
18 years (2008) is shared — and r208/r209 need checking against their own year rows.

## Mechanism, in code

`crates/app/src/ui.rs`, `coverage_by_era`:

    era.held += row.held;
    if let Some(p) = row.published {
        era.published = Some(era.published.unwrap_or(0) + p);
    }
    …
    era.ratio = era.published.map(|p| era.held as f64 / p as f64);

`row.published` is the year's count for the whole source. Nothing consults `row.shared_year()`, which
is the predicate 229 added for precisely this question and which the loop three lines further down
(the per-year `td`) does consult. The summary then prints it unconditionally:

    "{group(era.held)} / {published_cell(era.published)} · {coverage_pct(era.ratio, era.partial)}"

`published_cell` and `coverage_pct` are innocent — they render whatever they are handed. The `†`
footnote that would explain the mismatch is attached only to per-year cells, so the summary carries no
mark at all: nothing on the collapsed line says the figure is not what it appears to be.

## Why it matters

The whole point of the era rows is that they collapse: the panel's own copy says "One collapsible row
per source and profile era — expand for the per-year breakdown." The summary is what a reader scans;
expanding is the exception. So the 14-row scan reads as a pipeline with a dozen gaping holes —
0.00 %, 0.21 %, 2.36 %, 3.54 %, 5.12 %, 5.71 % — when the underlying years are complete. Coverage is
the panel that answers "do we hold the whole year", and this is the one line where it answers wrong.

The direction is the dangerous one: it under-reports. 396 unit 1's surplus was benign (nobody
re-fetches data they are told they already have twice over); this invites the opposite — re-fetching,
re-backfilling, or filing gap issues against eras that have no gap.

## Why this is ours, not the publisher's

The ground truth is per (source, year) and always was — `ted-notice-counts.csv` has no profile column
and cannot have one, because TED does not publish "how many 2024 notices were SDK 1.5". Issue 229
already concluded that a per-profile share of a shared year has no ground truth and must not be
rendered as a ratio. This system then went on rendering exactly that, one level up.

## Repro

1. `curl https://tenders.zebreus.click/` → Coverage panel, collapsed summaries:
   `ted · eforms:eforms-sdk-1.5 | 35 / 1 597 124 · 0.00 %`.
2. Expand the same era → `2024 | 12 | 801 444 | 100.00 %†` and `2023 | 23 | 795 680 | 100.00 %†`.
   Both rows are `†`; the summary is not.
3. `ted · internal-ojs` → summary `7.94 %`, its only year row `100.14 %†`; 26 955 / 339 534 = 0.0794,
   the figure issue 229's test doc-comment names as the bug it fixed.
4. `sed -n '644,676p' crates/app/src/ui.rs` (`coverage_by_era`: sums `row.published` with no
   `shared_year()` consultation) against `sed -n '590,600p'` (the per-year `td`, which does consult it).

## Done when

- A decision is written here, with its reason, among at least these:
  (a) the era ratio is computed over the era's SOLE years only (numerator and denominator both
      restricted), and marked when years were excluded;
  (b) the era ratio is suppressed (`—`) once any of its years is shared, since no per-profile ground
      truth exists — the 229 conclusion applied unchanged, at the cost of losing a figure for 10 of
      14 `ted` eras;
  (c) the summary shows the whole-year coverage of the years the era spans, `†`-marked like the cells,
      so it means "these years are N % covered, across all profiles serving them";
  (d) the summary drops the ratio and shows held alone, with the per-year table carrying all coverage.
  Whichever is chosen, the collapsed line must not state a percentage that every row under it
  contradicts.
- `ted · eforms:eforms-sdk-1.5` no longer summarises as `0.00 %` above two `100.00 %†` rows, and
  `ted · internal-ojs` no longer reproduces 229's `0.079`.
- Any mark the summary gains is explained by a footnote in the same panel (the `†` note today speaks
  only of "the year is served by more than one profile"; a summary-level mark needs its own sentence
  or that note extended).
- A test pins the invariant, not the numbers: **for an era in which any year is shared, the summary
  figure is never `era.held / era.published`.** 229's existing fixture (internal-ojs 26 955 and text
  313 059 against 339 534 published in 2008) is the natural case — it is already in
  `ui::tests::a_shared_year_shows_the_years_coverage_not_a_profiles_share`, which asserts the per-year
  behaviour and can be extended to the era it belongs to.
- Re-read after the fix: the 13 `doe`/`fts` eras still read `—` (no ground truth, unchanged); the
  per-year cells are untouched, so 2008 still reads `100.14 %†` on both its profiles and 2026 still
  reads `118.74 % *†`; `ted · text`, whose years are nearly all sole, still reads ≈ 99.58 % or the
  chosen equivalent.
- Checked as part of the same unit, since the measurement above did not cover them: whether
  `ted · ted-export-r208` (32.05 %) and `ted · ted-export-r209` (71.13 %) are understated for the same
  reason — both eras overlap the `text` era's years — or whether those figures are real backfill
  shortfalls. The answer goes in this issue either way, because a reader cannot currently tell.

## Also on this line: the `‡` stops at the per-year cell (noticed 2026-09-16, rev `94d8f61`)

Issue 396 unit 1 dated the partial-year denominator, but only in the per-year `td`: the era summary
calls `published_cell(era.published)` directly and gains no mark. Live, one era now reads

    ted · eforms:eforms-sdk-1.14        104 355 / 497 791 · 20.96 % *
      2026 | 104 355 | 497 791 ‡ | 118.74 % *†

— the same 497 791, dated one line down and undated on the line a reader scans. So the summary is
now the ONLY place on the page where a frozen mid-year count is served without its date.

Whichever rendering this issue picks, the `‡` (or whatever the summary's denominator becomes) goes
with it: `era.published` sums years that may be partial, so if it survives at all it needs the same
qualification the cell got. If the denominator is dropped — options (b) and (d) — the question
disappears with it, which is one more argument for those.

## BUILT 2026-09-16 — decision: (c), the years' coverage, marked; and r208/r209 answered

Status: built, gate green (`GATE-EXIT=0`), not yet deployed.

### The decision

Four renderings were offered. **Taken: (c) — the summary shows the coverage of the years the era
SPANS, `†`-marked, with the era's own held count beside it.** The line becomes

    ted · eforms:eforms-sdk-1.5      35 held · 1 597 124 published † · 100.00 %

and its hover reads "Some of these years are served by more than one profile, so the percentage is
how much of those YEARS is held across all of them … The held count is this era's own."

Why not the others:

- **(b) suppress the ratio** is 229's conclusion applied unchanged, and it is the tempting one — but
  it costs a figure for **10 of the 14** `ted` eras with a denominator, including every eForms one.
  The question "are the years this era touches covered" HAS a ground-truth answer; refusing to print
  it because the era's own share does not is throwing away the half that is knowable.
- **(d) held alone** is (b) with less information.
- **(a) the sole years only** looked principled and is worse in practice: an era whose years are ALL
  shared (10 of 14) gets nothing, and an era with a mix would silently report on a subset of its
  years without saying which — a new version of the same "the number is not what it looks like".

(c) degenerates to today's arithmetic exactly where nothing is shared: `year_held == held` for a sole
year, so an unshared era's percentage is unchanged and unmarked. The `†` is doing the whole job of
saying which of the two questions the number answers, which is why it is the same mark the per-year
cells use — one idea, one glyph.

### r208 and r209, which the `## Done when` required answering either way

They are **understated for exactly this reason. Neither is a backfill shortfall.**

| era | summary today | per-year rows |
| --- | --- | --- |
| `ted · ted-export-r208` | `2 699 213 / 8 421 040 · 32.05 %` | 15 years, **11 shared**; every row reads `100.00 %†` (2010 `100.03 %†`), and the four SOLE years read ~100 % too (2012 `414 836 / 414 837`, 2011 `411 850 / 411 850`) |
| `ted · ted-export-r209` | `4 490 549 / 6 313 458 · 71.13 %` | 10 years, **10 shared**; every row reads `100.00 %†` |

So two eras that today advertise 68 % and 29 % missing have, by the page's own per-year rows, no hole
at all. After the fix both read ≈ 100 %†. That is the sharpest before/after on the panel and the
reason this was worth separating from 396 rather than riding it.

### Tests

`an_era_summary_never_divides_one_profiles_held_by_a_shared_years_published` uses 229's own fixture
one level up — 2008 shared between `internal-ojs` (26,955) and `text` (313,059) against 339,534
published — and asserts both eras are `shared`, carry the `†`, summarise at the YEAR's 1.0014, and
explicitly that neither reproduces 0.079. It then asserts the held counts are still the eras' own
(26,955 and 313,059, not the year's), that an era with no shared year is UNMARKED and
arithmetically unchanged (100 / 200 = 0.5, as before), and that the hover distinguishes the two.

Run red first against the old arithmetic: it failed with
`internal-ojs summarises the YEAR's coverage, not its own share: 0.07938822032550495` — the live
figure, to the digit, and issue 229's own cited number.

### Live acceptance still owed (after deploy)

- `ted · eforms:eforms-sdk-1.5` reads ≈ `35 held · 1 597 124 published † · 100.00 %`, not `0.00 %`.
- `ted · internal-ojs` no longer reads `7.94 %`.
- `ted · ted-export-r208` and `r209` read ≈ 100 %†, not 32.05 % / 71.13 %.
- The 13 `doe`/`fts` eras still read `—` (no ground truth, untouched).
- The per-year cells are byte-identical: 2008 still `100.14 %†` on both profiles, 2026 still
  `118.74 % *†` with its `‡`.
- `ted · text`, whose years are nearly all its own, gains the `†` (2008 is shared) and its percentage
  moves from 99.58 % to ≈ 100.3 % — correct, and worth reading as the check that the mark is doing
  its job rather than as a change in what is held.

### Still open on this issue after the fix

The `‡` addendum above: `published_cell(era.published)` sums denominators that may include a PARTIAL
year, and the summary shows no `‡`. Under (c) the denominator survives, so it still needs the date
qualification. Not done in this unit — the summary now says `… published †` and the partial-year
date is one glyph further than this unit went.

## RESOLVED-VERIFIED 2026-09-16 — read live off `/api/dashboard` at rev `c40cccc`

Every cell the `## Done when` names, measured rather than assumed. `year_held` is the era's numerator
now, so the summary divides the years' WHOLE held by the years' published:

| source · profile | held | published | year_held | coverage | † |
| --- | ---: | ---: | ---: | ---: | :-: |
| ted · eforms:eforms-sdk-1.5 | 35 | 1,597,124 | 1,597,124 | **100.00 %** | yes |
| ted · internal-ojs | 26,955 | 339,534 | 340,014 | **100.14 %** | yes |
| ted · ted-export-r208 | 2,699,213 | 8,421,040 | 8,421,133 | **100.00 %** | yes |
| ted · ted-export-r209 | 4,490,549 | 6,313,458 | 6,313,456 | **100.00 %** | yes |
| ted · text | 3,786,955 | 3,802,937 | 3,815,808 | **100.34 %** | yes |

- `eforms-sdk-1.5` was the headline defect at **0.00 %** (35 ÷ 1,597,124 rounded away); it reads 100.00 %.
- `internal-ojs` was **7.94 %** (26,955 ÷ 339,534); it reads 100.14 %.
- r208 and r209 both read 100.00 %† — the 400-era question "are they understated by the defect or
  backfilled short" is answered: understated.
- `ted · text` gains its † and lands at 100.34 %, as predicted.
- **`doe` and `fts` still render `—`**: every one of their 12 profile rows carries `published: 0`, so
  no denominator is invented for a source with no published-count feed.
- The one row WITHOUT † is `fts · fts:ocds-1.1` (held 10,600 = year_held 10,600) — correct, it is the
  only profile serving its years, so nothing is shared and the mark would be noise.

Closing.
