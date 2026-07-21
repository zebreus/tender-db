# 25 — /api/tenders cold read is ~3s (v_tenders MAX-seq view)

Status: ready-for-agent

Observed post-bad8dda deploy (2026-07-21): `/api/tenders` (list_tenders,
200 rows over the `v_tenders` MAX(seq)-per-tender view) took 2.9s on a
cold read at 3.5M notices; warm reads are fast. The dashboard hides it
behind SSR + caching today, but the cost is O(all tender versions) per
cold read and the dataset is about to quadruple with the backfill.

Investigate: what plan does turso pick for the view's MAX(seq) GROUP BY;
whether a current-version flag/table (maintained by the projection, which
already knows when it supersedes a version) or an index gets it to
O(page). Prefer the design that keeps ADR-0001's append-only change
model intact.

Acceptance: cold /api/tenders and /v1/tenders p99 < 500ms at full-archive
scale.
