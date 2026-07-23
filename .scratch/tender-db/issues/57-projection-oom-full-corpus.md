# 57 — Full-corpus projection OOMs the 8GB VPS (crash loop)

Status: ready-for-agent
Severity: HIGH (was crash-looping prod ~every 10 min; mitigated with swap, not fixed)

Found 2026-07-23 ~21:00 UTC: after the WAL fix (4d023bb) let the backfill
progress to the projection phase, the service began OOM-crash-looping on the
8GB VPS — kernel OOM kills at 20:37, 20:48, 20:58, 21:08, 21:19 (~10 min
apart, restart counter 9). Each cycle: restart → recover ~14 durable jobs →
RSS climbs ~10 MB/s silently → OOM at ~7 GB → repeat, making ZERO forward
progress (the projection is not checkpointed mid-run, so it restarts from
scratch every time and never completes).

Mechanism: crates/ingest/src/project.rs Phase 1 (the parsed_chunk read loop)
accumulates the WHOLE corpus's per-notice `states` + `all_mentions` in RAM
before it logs `[project] read:` — at 7.5M+ notices that exceeds 8GB. The
read is chunked but the ACCUMULATION is unbounded. Same class of failure the
process pass already solved with a bounded channel; the projection didn't.

MITIGATION IN PLACE (2026-07-23): added 16 GB swap on /data (swapfile,
persisted in /etc/fstab) → ~23 GB usable, breaks the loop so the projection
can complete (slowly, thrashing). This is a band-aid, NOT the fix.

Real fix: bound the projection's peak memory so it fits in RAM regardless of
corpus size. Options to weigh:
  - stream/spill: process the projection in bounded period/id windows and
    persist intermediate states rather than holding all in RAM at once;
  - or group incrementally so `states`/`all_mentions` never hold the whole
    corpus.
Must preserve correctness (grouping spans a whole Tender's notices across
time — the window boundaries must not split a Tender's version chain).

Acceptance: full-corpus projection completes on the 8GB box with peak RSS
well under 8 GB (swap removable); daily incremental projection unaffected;
version chains / islands identical to a whole-RAM projection on a fixture.
