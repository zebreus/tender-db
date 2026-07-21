# 28 — Ingestion throughput: profile, then parallelize if it pays

Status: ready-for-agent
Blocked by: 15 (profile against real full-archive runs; don't disturb the
active backfill)

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
