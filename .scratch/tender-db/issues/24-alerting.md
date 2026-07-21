# 24 — Alerting: know when production breaks without looking

Status: ready-for-agent

Current monitoring is tmux loggers writing files on the box — nobody is
notified if the service dies, /health goes red, disk fills, or the daily
continuous-mode jobs stop landing. The /v1 view-staleness 500s went
unnoticed until manually observed; continuous operation (issue 15
acceptance: 3 days of live updates) needs eyes that don't sleep.

Scope (keep minimal — this is a one-operator project, not an SRE stack):
- External uptime check on https://tenders.zebreus.click/health with
  email/push notification (a free hosted pinger is fine and is the one
  piece that must NOT live in the app, since it must fire when the app
  is down).
- In-app health surface for the pinger to judge: /health already exists;
  extend it (or a /health/deep) to go unhealthy when the last successful
  scheduled ingest is older than expected (TED > ~26h) or disk usage
  crosses a threshold — then the single external check covers freshness
  and disk too.
- Alert on job failures: supervisor marks a job ERRORED → surfaced in
  the same health signal.

Acceptance: kill the service → notification arrives within minutes;
simulate stale ingest (clock the threshold) → health flips unhealthy;
documented in the runbook.
