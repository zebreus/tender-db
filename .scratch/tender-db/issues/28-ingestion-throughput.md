# 28 — Ingestion throughput: profile, then parallelize if it pays

Status: RE-MEASURED AT FULL SCALE 2026-08-21 (owner) — the cost model this issue was filed under
no longer holds; see the bottom. The original 70–120/s is now 285–360/s measured on real era-scale
re-parses, so a full reprocess is ~11–14 h, not 1–2 days. Parallel parse/map remains unbuilt and
undecided; the pressure that would justify it has halved twice since filing. Was: ready-for-agent
Blocked by: 15 (profile against real full-archive runs; don't disturb the
active backfill)

2026-08-09 note (orchestrator, parser audit — docs/research/parser-audit-2026-08.md):
the profile run needs real packages + a release `process` CLI, and the deployed
store path ships only `server`. Whoever picks this up: either add the CLI bins to
the nix output (small flake change) or run on a snapshot machine with archive
access. Steady-state dailies are comfortably inside the tick; this issue only
gates BULK reprocess latency (~1-2 days at 14.2M notices today).

Processing runs single-threaded at ~70–120 notices/s (era-dependent).
Fine for dailies; a full reprocess (quarantine-triage fixes, mapping
corrections) costs ~1–2 days at 12.9M notices. Before optimizing:
profile where the time goes — XML parse, profile mapping, zip walking,
or the store write path. The writer is a single connection by design, so
the likely win is parallel parse/map feeding the existing batched
writer, but evidence first (issue 19's lesson: profile showed the
bottleneck was NOT where assumed).

Acceptance: a documented profile of a representative monthly package per
era; either a parallelization design + implementation with measured
speedup (×2 or better on the mid-2000s text era), or a documented
decision that it doesn't pay.

---

## Full-scale measurement, free of charge (2026-08-21, owner)

Issue 251's step 4 ran two era-scale re-parses through the deployed pipeline — parse + map + store
write + per-notice clear, the exact loop this issue wants profiled — at production scale on the
production box:

- **ted-export-r208:** 2,699,213 notices / 161 monthly packages in ~2h05m ≈ **360 notices/s**
  sustained end-to-end (7,487,859 archive members walked).
- **ted-export-r209 (mid-run):** ~205 notices/s over the first 2.15M — the heavier era (bigger
  payloads, more values per notice).

Against the filed 70–120/s, that is a 2–3× improvement with NO parallelism — it came from removing
serial waste: the issue-247 mention-clear index (153 ms → µs per notice), the issue-243 merged
award statement, and issue 234's org-table collapse (30.5M → 12.3M rows shrinks every resolver
probe). The lesson the issue itself predicted (19's: profile first, the bottleneck is not where
assumed) held — the wins were all in the store write path, not the XML parse the parallelization
sketch targeted.

**Consequence for the decision this issue asks for:** a whole-corpus reprocess now costs a
weekend-night, not "1–2 days", and the campaign/dailies fit comfortably. Parallel parse/map would
still help (the parse IS CPU-bound between writes), but the ×2 acceptance bar now buys ~6 h off an
~12 h rare event — worth building only when full reprocesses become routine. Leaving open at low
priority; the next real full-archive reprocess should record its wall-clock here as the standing
benchmark.