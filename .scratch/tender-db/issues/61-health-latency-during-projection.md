# 61 — /health (and API) latency spikes to ~4.5s during a full projection

Status: ready-for-agent
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
