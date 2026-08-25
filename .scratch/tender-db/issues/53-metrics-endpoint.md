# 53 — In-process /metrics endpoint (Prometheus text); defer the server

Status: CLOSED-VERIFIED (2026-08-25, owner) — the remaining "deploy + one live
scrape" is done: deployed since the post-08-17 releases, and today's live scrape of
`https://tenders.zebreus.click/metrics` serves well-formed Prometheus text (HELP/TYPE
pairs, change cursor 434M, RSS 2.27GB, sse_streams 0, deadline_hits_total 0,
writer_queue_depth 0 — all plausible against the dashboard). Originally: BUILT
2026-08-17 (owner) — endpoint landed, awaiting deploy. `GET /metrics`
(`crates/app/src/v1/metrics.rs`) serves Prometheus text: change cursor, RSS, live SSE
streams, disk + `wal_bytes`, per-kind last-run duration/finish/outcome from the bounded
job-log window, the freshness clock, import lag, canonical row counts and the quarantine
totals + per-reason breakdown. Cost discipline held: every gauge is O(1) or a read of the
dashboard's existing 60s cache — no table-proportional scan on the scrape path, so a
frequent scrape can't become the load it observes. A section not yet measured is ABSENT,
never zero (e2e-pinned: a fake `0` for `quarantine_outstanding` would read as a drained
backlog). Gauges only — nothing accumulates in-process, so a restart can't reset a series
mid-flight. Hand-rolled exposition (~40 lines) rather than a metrics crate, same
weight-without-payoff call that rejected OTel; Prometheus+Grafana server stays deferred as
decided. Outside the rate limiter, NOT in the public CORS grant. 2 e2e + 4 unit tests,
44/44 api suite; runbook section written. Remaining: deploy + one live scrape.

Motivation (validated by the 2026-07-22/23 WAL incident): the WAL-growth
+ throughput collapse, RSS creep (issue 52), and per-era throughput
decline (issue 28) are all "watch a number trend over time" problems,
caught only by eyeballing the tmux backfill-status.log + manual
reasoning. A real time-series would have surfaced the WAL non-reclaim as
an obvious trend hours earlier.

DECISION (team lead, owner):
- **OTel: rejected.** Its value is distributed tracing + vendor-neutral
  export across services; this is a deliberate single-process monolith
  (ADR-0005), no cross-service calls to trace. Weight without payoff.
- **/metrics endpoint: do it.** A small in-process Prometheus-text
  endpoint (a metrics crate — counters/gauges), monolith-friendly, no
  external account (fits [[no-external-resources]]). Expose the numbers
  we hand-watch: ingest rate, wal_bytes, RSS, disk, per-package/per-job
  durations, quarantine counts by class; post-launch add API request
  rate/latency/errors, SQL-query durations, SSE stream count. Reuse the
  signals /health/deep already computes where possible.
- **Prometheus+Grafana server: deferred + optional.** That's the real
  operational weight (extra process(es) to run/secure/resource-budget on
  the 8GB box, against the monolith ethos). The /metrics endpoint makes
  it a later reversible choice — stand it up ON-BOX (no external account)
  if history+dashboards+alerting are wanted. Launch-hardening, not now.

Scope when picked up: the /metrics endpoint only. Keep it off the hot
path (reader pool for any DB-derived gauge, like /health/deep). Document
in the runbook. Leave the server as a documented option.

Acceptance: GET /metrics returns Prometheus-format text with the
ingestion + system gauges; no new external dependency/account; documented.
