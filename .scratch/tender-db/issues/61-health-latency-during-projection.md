# 61 — /health (and API) latency spikes to ~4.5s during a full projection

Status: RESOLVED (2026-08-17, owner triage sweep) — fixed by design, long shipped: `/health` no
longer queries the database AT ALL (mod.rs `async fn health`, whose doc cites this issue) — it reads
the in-memory cursor doorbell and answers instantly regardless of reader-pool saturation; DB-backed
readiness moved to `/health/deep`. Prod-corroborated: /health stayed fast/200 through the 2026-08-15
full rebuild and every multi-hour backfill since (no pinger incidents in the journals).
Was: ready-for-agent
Severity: LOW-MEDIUM (service stays 200, but slow; may trip the cloud pinger)

Found 2026-07-24 right after deploying 8f0dc74 (issue 59 disk-backed plan +
60 cache_size 512MiB). During the full-corpus projection's Phase 1, /health
went from ~40ms (old build) to ~4.5s (occasionally >10s → curl 000 / pinger
timeout). Service stayed 200; WAL bounded; not a hang.

/health does only `read::latest_cursor` (MAX over changes PK) via an api-pool
reader — cheap. The latency is contention, from two new-build changes:
  1. issue 59 now WRITES ~12.4M plan_notice rows to the main DB during Phase 1
     (old build built the plan in RAM, no writes) → more disk I/O.
  2. issue 60 cache_size = -524288 (512 MiB) PER CONNECTION reduces the shared
     OS page cache, so the health reader's changes-leaf fetch misses cache and
     queues behind the projection's saturating disk I/O.
So the health reader's tiny read waits on a saturated disk.

Expected to be TRANSIENT: recovers when the projection's heavy I/O ends (now
much faster with the cache win), and the daily incremental projection (issue
58) touches few notices → no saturation. So it mainly bites the rare full
rebuild.

Fixes to weigh (only if it recurs / matters):
  - Tune cache_size down (256 MiB still 128× default) to leave more OS cache;
    OR scope the large cache to the projection's connection only, keep API
    readers lean.
  - Give /health a short timeout + cached/last-known cursor so a pinger never
    sees a slow health during a heavy job (like issue 56's freshness gating).
  - Throttle/yield the projection's plan-write I/O.

Acceptance: /health stays sub-second (or a fast cached answer) even during a
full projection; cache win on the projection preserved.

## UPDATE 2026-07-28 — it's full HTTP starvation at prod scale, not 4.5s

At full prod scale (6.96M-tender layer built; daily incremental + snapshot jobs),
/health and /admin/jobs FULLY TIME OUT (>8-30s) for the entire duration of ANY
heavy job — not the mild 4.5s originally diagnosed. Confirmed: the daily
`project` (incremental fold) and `process` jobs each take the API/dashboard dark
for their whole run (~tens of min), and the daily `snapshot`'s integrity_check
took the API down for ~26.7h (that specific starve is now fixed by af4c2b2 —
`TENDER_SNAPSHOT_INTEGRITY` gate — but the projection/process starvation remains).

Root cause is broader than disk contention: the supervisor's job worker runs
`execute(job).await` on the MAIN tokio runtime (supervisor.rs:421 `tokio::spawn`),
and turso does BLOCKING preads on the worker threads — a long job saturates the
runtime so the API/SSE/dashboard tasks can't be scheduled.

THE FIX (proven pattern already in this codebase): isolate the job worker on its
own dedicated tokio runtime, exactly as issue 17 did for `/v1/sql`
(`crates/app/src/v1/sql.rs` → `spawn_sql_runtime`: a parked OS thread owns a
small multi-thread runtime; work is submitted via `Handle::spawn` and only the
`JoinHandle` is awaited on the main runtime). Move the supervisor worker's job
execution onto such a dedicated runtime so projections/snapshots pin the job
runtime's threads, never the API runtime. The Db writer (`Mutex<Connection>`) +
`read_pool` are already shared across runtimes (the SQL endpoint proves turso
connections work off the main runtime), so the API's read-pool queries stay
responsive.

Severity is now HIGH (was LOW-MEDIUM): the API/dashboard being unavailable during
every daily job is the biggest gap vs "operating" + "easy to inspect". Also
consider the issue-61 palliatives (cached /health cursor; tune per-conn
cache_size) as belt-and-suspenders. Acceptance: /health + /v1 stay sub-second
while a full projection AND a snapshot run.
