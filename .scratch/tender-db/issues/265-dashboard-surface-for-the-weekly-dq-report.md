# 265 — the weekly data-quality run has no dashboard surface: headline rates + week-over-week deltas

Status: needs-triage — filed 2026-08-21 (owner, requested by Lennart: more data-quality monitors
on the dashboard).
Kind: observability / dashboard
Blocked by: —
Relates to: 230 (the report + `/admin/reports/data-quality`), 109 (the presence step-change alarm —
the pattern this generalizes), 191 (dashboard staleness lessons)

## The gap

The weekly measurement (issue 230) produces the richest quality picture the project has — per-era
completeness, linkage, winner rates, VAT basis, content presence — and its ONLY surfaces are a
text body behind an operator secret and the job summary line. The dashboard (`ui.rs`), the thing a
human actually looks at, shows coverage and quarantine but nothing from the report. And the report
is a POINT IN TIME: issue 109 stores previous-run rates for exactly one measure (factless) because
the alarm needed it; every other rate loses its history on overwrite, so "did winner completeness
move this week" is unanswerable without a saved copy.

## What to build

1. **Rate history, generalized from 109's mechanism**: alongside `data-quality-presence`, store a
   compact `data-quality-headlines` JSON per run — per era: versions, factless, winner-named rate,
   award linkage, value completeness, VAT stated rate. One row per kind is fine for the ALARM
   (one-run lookback), but the dashboard wants a short trend, so keep the last ~12 runs (a small
   JSON array, appended and truncated — still one reports row, still bounded).
2. **A dashboard section** rendering the latest headlines as a per-era table with a delta column
   against the previous run (▲/▼/— with the point change), and the report's `computed_at` shown
   prominently — a quality table without its measurement date is issue-191's staleness trap.
3. **Alarm surfacing**: when the 109 step-change alarm (or a future generalized one) fired on the
   latest run, the dashboard section leads with it — an alarm that lives only in a job summary is
   an alarm for whoever reads job summaries.

## Non-goals

No new measurements — this issue only SURFACES what the weekly run already computes. New measures
are their own issues (263/264/266/267 as of this filing).

## Acceptance

The dashboard answers, at a glance and with dates: what are the per-era headline rates, which
moved this week, and did anything alarm. A stale report (>8 days) is visibly flagged, not silently
rendered as current.
