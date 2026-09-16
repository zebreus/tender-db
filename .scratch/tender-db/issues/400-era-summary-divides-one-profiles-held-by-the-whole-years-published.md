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
