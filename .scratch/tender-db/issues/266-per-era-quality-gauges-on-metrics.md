# 266 — /metrics carries no data-quality gauges: external alerting cannot watch the corpus

Status: needs-triage — filed 2026-08-21 (owner, requested by Lennart: more data-quality monitors).
Kind: observability / metrics
Blocked by: — (pairs naturally with 265's headline storage)
Relates to: 53 (the /metrics endpoint), 230 (the report), 109 (the factless rate this would export)

## The gap

`/metrics` exports operational health (writer queue, jobs, deadline cuts) but nothing about DATA
quality — an external Prometheus/alerting setup watching the box today can page on "a job failed"
but never on "an era went content-stale" or "winner completeness halved". The weekly report holds
those numbers; a scraper cannot read a text report.

## What to build

From the latest stored headline rates (265's `data-quality-headlines`, or directly from
`data-quality-presence` for the factless slice) — a point lookup, no measurement on the scrape
path, per the /metrics rule established in issue 53:

    tender_db_dq_report_age_seconds                      how stale the quality picture is
    tender_db_dq_factless_rate{era="..."}                issue 109's rate, scrapeable
    tender_db_dq_winner_named_rate{era="..."}
    tender_db_dq_award_linkage_rate{era="..."}
    tender_db_dq_value_completeness{era="..."}

Era label from the report's own era names. Gauges absent until the first report exists (a gauge
that appears is honest; a gauge that reads 0 before measuring is issue-230's zero-lie).

## Acceptance

A scrape after a weekly run shows the per-era gauges matching the report's numbers; before any
report exists the gauges are absent, not zero; `dq_report_age_seconds` makes a silently-stopped
weekly run alertable (the issue-161 class: a monitor whose observer died reads permanently green).
