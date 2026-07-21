# 20 — /admin/jobs latency spikes under heavy processing

Status: needs-verification

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

## Fix (2026-07-21)

Root cause: `GET /admin/jobs` → `Supervisor::ingestion()` read the recent
runs via `Db::recent_job_runs`, which used `self.conn()` — the **single
writer connection behind a `Mutex`**. An ingestion job holds that writer
for the length of each transaction (the projection commits in 512-tender
`BEGIN IMMEDIATE … COMMIT` batches — issue 19), so the dashboard read
queued behind the batch and returned only once it committed: the ~23s
stall. `/health` was unaffected because it never touches the DB.

It was the one read path still on the writer: the public API, SSE, and the
webhook sweeper each already read through the WAL reader pool "so reading …
never queues behind ingestion on the writer" (webhooks.rs). This just
brings `/admin/jobs` onto the same pattern.

Change (no behaviour change):
- `store::recent_job_runs` is now a free function over a borrowed
  `&Connection` (like `read::changes_since`), not a `Db` method on the
  writer. `record_job_run` stays on the writer — it is a write.
- `Supervisor` owns a small `Readers` pool (`db.readers(2)`, mirroring the
  webhook sweeper) and serves `ingestion()`'s recent runs through it. WAL
  readers see the last committed snapshot without waiting on the writer.

Tests (crates/store/src/jobs.rs):
- `recent_runs_read_while_the_writer_is_busy` — holds the writer in an open
  `BEGIN IMMEDIATE` transaction and asserts the reader-pool read still
  returns (< 1s). Under the old writer-routed read this deadlocks against
  the held guard — the acute form of the production stall.
- `job_log_round_trips_newest_first` updated to read via the reader pool.
- Full app suite green, incl. `admin_api_drives_ingestion_end_to_end`.

Needs production verification: confirm /admin/jobs p99 < 1s during the
historical backfill's heavy process/project phases.
