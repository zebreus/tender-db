# 37 — Dashboard first snapshot after boot takes ~9 min under load

Status: ready-for-agent

Observed after the b0a5cdb deploy (2026-07-21): the background refresher
(issue 20 part 3) computes the whole dashboard snapshot as one
measure() pass, so after a restart the data panels serve the empty
initial snapshot until the first full pass lands — ~8-9 min while the
re-walk churned IO. Requests are fast throughout (by design) but the
dashboard shows "never"/zeros, which reads as broken.

Fix directions (pick the minimal one that works):
- Fill sections independently: funnel/counts/lag are cheap (sub-second)
  and can land in the snapshot immediately; only the coverage GROUP BY
  is slow. Incremental section fills make the dashboard useful within
  seconds of boot.
- And/or persist the last snapshot (a small JSON blob in the DB, written
  by the refresher) and serve it stale-with-age on boot until the first
  fresh pass replaces it — the snapshot_age field already exists to be
  honest about it.

Acceptance: within ~10s of a restart under ingestion load, the
dashboard shows data (fresh cheap sections, or stale-labelled previous
snapshot) instead of zeros; no request-path scans (part-3 invariant
holds).
