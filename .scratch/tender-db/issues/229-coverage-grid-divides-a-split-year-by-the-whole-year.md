# 229 — the coverage grid divides each profile's held count by the WHOLE year, so a year served by two profiles reads as two gaps

Status: REOPENED 2026-09-15 — incomplete fix: the fix landed on the per-year grid rows only, and the
collapsed era summary line still divides one profile's held count by the WHOLE-year denominators, so
the 0.079 this issue was filed to remove is served on prod (rev `9e082fd`) as "7.94 %". See Comments.

Previous status, kept as history:
Status: RESOLVED-VERIFIED (2026-08-18, owner — deployed rev `cc0ef20`). Fixed the way the sketch's
second option describes, taken to its logical end: a shared year offers NO per-profile ratio, because
no ground truth exists for one profile's share of a year. `Coverage` gained `year_held`/`year_ratio`
(the year summed across its profiles), `ratio` is `None` when the year has more than one profile, and
the grid renders the year figure daggered, with the year's held count in the cell title and a footnote
explaining the era boundary. A year one profile serves alone is unchanged.

Verified on prod after deploy — both 2008 rows now read `year_held` 340,014, `year_ratio` 1.0014,
`ratio` null, exactly the honest shape:

    profile internal-ojs  held  26,955  year_held 340,014  ratio null  year_ratio 1.0014
    profile text          held 313,059  year_held 340,014  ratio null  year_ratio 1.0014

The test uses these measured numbers as its fixture and pins the complement (a sole profile keeps its
own ratio, unmarked). Not done, and deliberately: the issue suggested checking other transition years
for the same shape — the fix is general (it triggers on any year with >1 profile, whatever the era),
so no year-by-year sweep is needed; the r208→r209 and r209→eForms boundaries now self-report.
Kind: observability / dashboard (misleading-as-read, not wrong)
Blocked by: —
Relates to: 33 (the funnel + grid this lives in), 41 (the 2008 era whose verification surfaced it),
30 (the same family: a headline number that overstates a problem), 108 (the recurring shape — a
record accurate about what it measured and misleading about what it is read as)

## What

`model::dashboard::Coverage` is "notices held for one (source, profile, year) against what that year
is known to have published" — the numerator is per (profile, year), the denominator is the YEAR's
total. When one year is served by TWO profiles, every row divides by the full year and each looks
deficient, even when the year is complete.

Measured on prod today:

    year 2008  profile internal-ojs   held  26,955   published 339,534   ratio 0.079
    year 2008  profile text          held 313,059   published 339,534   ratio 0.922

Read row-by-row that is a catastrophic gap in the 2008 OPOCE export and a 7.8% shortfall in text. In
fact 26,955 + 313,059 = **340,014 against 339,534 — the year is complete (ratio 1.0014)**, comfortably
inside the ±2% tolerance. 2008 is a transition year carrying both formats, which is exactly what issue
41 built the INTERNAL_OJS profile for.

## Why it matters

The grid is the most prominent thing on the dashboard and this is its headline number. A reader who
trusts a row concludes there is a 313k-notice hole where there is none — and the failure direction is
the dangerous one for a data project: it invites a "recovery" of data already held. It also devalues
the grid generally, because once one row is known to be misleading, a genuinely deficient row earns
the same shrug (issue 226's lesson: a signal that cannot distinguish two states is not a signal).

The type's doc comment does say the denominator is the year's total, so nothing here is *wrong*. The
defect is that the presentation invites a per-row reading the data does not support.

## Fix sketch (not costed)

The ratio is only meaningful per YEAR, so let the year own it:

- group the grid by (source, year) with the year's summed `held` and its single ratio, and keep the
  per-profile numbers as a breakdown carrying counts only — no per-profile ratio to misread; or
- keep per-profile rows but suppress the ratio whenever the year has more than one profile, with the
  year subtotal alongside.

Either way the invariant to test is the one that failed here: for a year served by N profiles, the
displayed ratio must be computed from the summed held count. Check the other transition years for the
same shape while fixing it (any year where two eras overlap — the r208→r209 and r209→eForms
boundaries are the obvious candidates).


## Comments

### 2026-09-15 — API/data-quality review fan-out: the collapsed era summary still divides one profile's held count by the whole year (this issue's defect, one click above the row it fixed)

**This is an incomplete fix, not new territory.** 229's fix and its prod verification cover the per-year
rows (`Coverage.ratio = None`, `year_ratio` rendered daggered — `crates/app/src/ui.rs:573-581`). The
collapsible era row that folds those rows up — `held`/`published`/`ratio` in each `<details class="era">`
summary — predates the fix (documented as designed in `06-dashboard-accounts`, ~2026-07-20) and was never
brought under the invariant this issue states: *for a year served by N profiles, the displayed ratio must
be computed from the summed held count.* The summary computes exactly the per-profile share the † footnote
says has no ground truth. `<details class="era">` (`ui.rs:547`) carries no `open` attribute, so that
summary line — not the corrected rows — is what a reader sees by default.

Evidence (literal):

    curl -sS https://tenders.zebreus.click/        # 2026-09-14, 512 KB, tags stripped

Era summary line vs. its own year rows:

| era summary line (default view)                                  | its year rows (expanded)                              |
|------------------------------------------------------------------|-------------------------------------------------------|
| `ted · internal-ojs 26 955 / 339 534 · 7.94 %`                   | `2008 \| 26 955 \| 339 534 \| 100.14 %†` (its only row)  |
| `ted · eforms:eforms-sdk-1.2 3 / 735 067 · 0.00 %`               | `2022 \| 3 \| 735 067 \| 100.00 %†` (its only row)      |
| `ted · ted-export-r208 2 699 213 / 8 421 040 · 32.05 %`           | all 15 rows read `100.00 %` (11 daggered, 4 unshared)  |
| `ted · ted-export-r209 4 490 549 / 6 313 458 · 71.13 %`           | every row daggered                                     |
| `ted · eforms:eforms-sdk-1.10 255 324 / 2 966 064 · 8.61 % *`     | rows read `91.75 %†` / `100.00 %†` / `117.38 % *†`    |
| `ted · eforms:eforms-sdk-1.5 35 / 1 597 124 · 0.00 %`             | every row daggered                                     |

Arithmetic checks out, which is the point — the numbers are computed exactly as the code says:
26 955 / 339 534 = 0.0794; 8 421 040 is precisely the sum of r208's 15 whole-year published counts
(11 of them shared years) and 2 699 213 / 8 421 040 = 0.3205; 4 490 549 / 6 313 458 = 0.7113;
255 324 / 2 966 064 = 0.0861.

Scope: 15 of the 16 TED eras — every one except `text` — have EVERY year row daggered and a summary
ratio no row supports: internal-ojs 7.94 %, ted-export-r208 32.05 %, ted-export-r209 71.13 %,
sdk-1.2 0.00 %, sdk-1.3 0.21 %, sdk-1.5 0.00 %, sdk-1.6 2.36 %, sdk-1.8 5.71 %, sdk-1.9 3.54 %, and the
six starred eras sdk-1.10 8.61 %, sdk-1.7 11.62 %, sdk-1.11 4.97 %, sdk-1.12 17.33 %, sdk-1.13 56.11 %,
sdk-1.14 20.59 %. The 16th, `text`, is mildly distorted (99.58 % shown; ≈100.40 % over its 16 sole-profile
years) because 2008 and 2010 are folded into its denominator. Non-TED eras (doe, fts) print "—" and are
unaffected.

Code — `coverage_by_era`, `crates/app/src/ui.rs:635-643`:

    era.held += row.held;
    if let Some(p) = row.published { era.published = Some(era.published.unwrap_or(0) + p); }
    …
    era.ratio = era.published.map(|p| era.held as f64 / p as f64);

No `shared_year()` check anywhere in the fold.

Judge's reasoning for why this is ours: the collapsed era summary is a dashboard computation, not a
source-published figure — `coverage_by_era` divides one profile's held count by the sum of the WHOLE-year
TED denominators of every year that profile touches, with no shared-year suppression, and TED's year counts
are an honest denominator only for the year, never for one profile's slice of it. It is not resolved by
this issue's fix: 229 is RESOLVED-VERIFIED (2026-08-18) but the fix and the prod check reach only the
per-year rows, while the era summary line predates them, so this is a coverage gap in the fix rather than
a regression in it — the literal number 229 quoted as the defect (2008 internal-ojs 0.079) is live today
as "7.94 %" one click above the `100.14 %†` row 229 installed, and since the `details` element has no
`open` attribute that summary is the headline reading. Severity medium: same Kind as this issue
(misleading-as-read, not wrong data), on the most prominent panel's default view, failing in the dangerous
direction 229 named — it invites "recovering" notices already held. No API or data impact.

To close: in `coverage_by_era` either set `era.ratio = None` (render "—") when any `row.shared_year()`, or
compute the era figure from `year_held` for shared years plus `held` for sole years over the summed
denominators; then extend `a_shared_year_shows_the_years_coverage_not_a_profiles_share` to the era fold —
the existing `coverage_folds_into_eras_newest_year_first` builds only sole-profile cells
(`year_held: held`), which is why the era level was never exercised.
