# 53 — In-process /metrics endpoint (Prometheus text); defer the server

Status: ready-for-agent (post-backfill / launch-hardening — NOT now)

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
