# 55 — SSE initial snapshot holds a minutes-long read transaction

Status: ready-for-agent — SEVERITY UPGRADED to HIGH (launch-blocking): proven DoS in prod 2026-08-09 (see incident note below)
Severity: MEDIUM (scaling wart on the live-subscription request path)

De-link note (2026-07-23): this is NOT a cause of the multi-day WAL runaway.
The runaway was `import_lag`'s full scan (issue 42, fixed), and the mid-txn SSE
reader leak was already fixed separately (pool discards non-autocommit readers,
2d2e223). This issue stands on its own merits — a successful large-collection
snapshot still holds one api-pool reader for minutes, a real latency/robustness
concern — not as a WAL-pin suspect.

Found during the issue-53 diagnosis. The SSE `start` handler
(crates/app/src/v1/sse.rs) does BEGIN → collect_snapshot → COMMIT, and
collect_snapshot paginates the WHOLE subscribed collection. At full-
archive scale (millions of rows) that snapshot runs for MINUTES, holding
one read transaction the whole time. The issue-53 leak (a cancelled
snapshot pinning the WAL forever) is FIXED (the pool discards it on
drop), but even a SUCCESSFUL large-collection snapshot holds a genuine
live read snapshot for its multi-minute duration — which pins the WAL
for that window (a checkpoint during it returns busy) and ties up a
reader.

This is inherent to "stream the whole current collection then diff".
Options to investigate: keyset-paginate the snapshot in bounded chunks
that each drain+release (so no single multi-minute transaction), at the
cost of snapshot-consistency-across-chunks (acceptable? the diff stream
reconciles); or cap/stream the snapshot differently; or document the
subscribe-to-a-huge-collection cost. Not urgent (few/no live subscribers
pre-launch) but real before launch with large collections.

Acceptance: an SSE subscription to a large collection does not hold a
single multi-minute read transaction; snapshot + diff coherence intact.

## Incident 2026-08-09 ~00:30–00:50 Berlin (orchestrator)

This stopped being theoretical: ~5 SSE subscriptions to `/v1/tenders` (opened
as post-deploy verification probes, then aborted client-side) each started the
full-collection snapshot — 8.1M tenders on rev `3485e3d`. Client aborts did not
end the server-side work observably; combined they saturated the 4-core box
(load 6.1), drained the api reader pool, and starved EVERY read including
`/health` — full public outage, 000s across the board, ~15–20 min until a
service restart cleared it. Recovery: `systemctl restart tender-db`; post-boot
all endpoints sub-second, /health/deep green, no data impact (reads only).

Consequences for the fix, beyond the original options list:
- This is a **public, anonymous, ~5-streams-per-IP** endpoint: five requests
  from ONE client sufficed. Treat as launch-blocking availability hole, not a
  scaling wart.
- Client disconnect must actually cancel the snapshot work (verify axum drop
  propagation through the pager), AND the snapshot itself must be bounded
  (chunked keyset pages that release the reader between chunks, a hard row/time
  budget per subscription, or refuse unfiltered snapshots of huge collections).
- The `/v1/sql` half of tonight's load ran on the isolated runtime as designed
  (issue 17/120 landings) — the starvation driver was the SSE snapshots on the
  main api pool.

Reproduction is now precise: `curl -N -H "Accept: text/event-stream"
"…/v1/tenders?limit=1"` ×5, abort after a few seconds, watch load and /health.
