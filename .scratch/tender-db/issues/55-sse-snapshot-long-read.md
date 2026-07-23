# 55 — SSE initial snapshot holds a minutes-long read transaction

Status: ready-for-agent
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
