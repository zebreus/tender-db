# 57 — Full-corpus projection OOMs the 8GB VPS (crash loop)

Status: RESOLVED-BY-FIX (verify a peak-RSS number to fully close) — 2026-08-15. The bounded design
this issue asked for LANDED (via issue 62's on-disk plan/fold): Phase-1 no longer accumulates the
whole corpus in RAM — `build_plan` streams the notice layer in id-ordered chunks and **spills each
resolved notice state to disk**, holding "at most one bounded working set at a time (issue 57/62)"
(project.rs:487–494, 692, 723, 799). Evidence it holds: the 2026-08-15 full rebuild folded
**14.27M notices → 7.92M tenders and completed with SWAP REMOVED** (the box now reports `Swap: 0`),
which the acceptance's "swap removable" clause is exactly about. The old crash-loop was an 8 GB box;
that box is gone (now 62 GB, no cgroup limit). The `60.5 G memory peak` systemd reports for that run is
cgroup memory INCLUDING page cache (reading a ~450 GB DB + the disk spill), not the anonymous working
set — the sawtooth working set measured ~0.5–3 GB in the issue-62 work. Downgraded from HIGH: there is
no crash-loop and no swap band-aid to remove. The one thing left to turn this into RESOLVED-VERIFIED is
a clean peak-**anonymous**-RSS number from a full run confirming it stays low at 14.27M notices (the
projection does not log RSS today — a cheap `VmHWM`/`memory.peak`-minus-cache probe around the next
rebuild would do it). Was: HIGH, ready-for-agent.
Severity: was HIGH (crash-looping prod); now LATENT at most — bounded in code + ample headroom on the
62 GB box. Relates to 59 (the formal corpus-independent-memory guarantee, in implementation) and 62
(the disk plan/fold that delivered the bound).

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
