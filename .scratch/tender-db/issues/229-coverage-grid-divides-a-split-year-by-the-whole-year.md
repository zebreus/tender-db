# 229 — the coverage grid divides each profile's held count by the WHOLE year, so a year served by two profiles reads as two gaps

Status: needs-triage — found 2026-08-17 while verifying issue 41 against prod (rev `62f0e19`)
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
