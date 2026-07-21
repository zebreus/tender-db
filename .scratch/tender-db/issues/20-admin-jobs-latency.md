# 20 — /admin/jobs latency spikes under heavy processing

Status: ready-for-agent

Observed during the historical backfill (run-driver, 2026-07-21): GET
/admin/jobs occasionally takes ~23s while a process job writes heavily
(turso read-vs-write contention around recent_job_runs), then returns
instant. /health stays <50ms throughout. Effect: the dashboard Ingestion
panel is intermittently slow during heavy ingestion — cosmetic, not
availability.

Likely fix directions: read job_log through the reader pool rather than
the writer connection if it isn't already; cache the supervisor's recent
runs in memory (it writes them — it can serve them without touching the
DB); or bound the query. Investigate first.

Acceptance: /admin/jobs p99 under heavy processing < 1s; no behavior
change otherwise.
