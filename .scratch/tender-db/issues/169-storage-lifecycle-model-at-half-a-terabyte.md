# 169 — storage lifecycle model at 0.5 TB and beyond

Status: open — research gap #3; urgency DOWNGRADED 2026-08-15 (measured: /data at 39%, 1.1T free — the "~87% full" premise no longer holds; lifecycle model still worth writing before the corpus doubles)
Role: run-driver

The disk arithmetic ended at "buy a 500 GB volume" (pilot-sizing); reality:
DB + archive share 1 TB at ~87%. Unstudied: measured steady-state +
rebuild-driven growth (change log is promised forever and rebuilds append
full-corpus change rows), turso behavior at 0.5-1 TB (the bench stopped at
10 GB; the 40-100 GB run was flagged as needed twice and never ran), and the
compaction dead end (VACUUM INTO OOMs — a turso DB can never shrink). Escape
hatch if the volume fills is dump-and-reload of half a terabyte with the
service down — wall-clock never measured. Deliverable: a dated capacity model
with a "volume full" date and a change-log retention trigger criterion (C17's
reserved pruning path has no trigger today). Growth curve is computable from
fetches/notices/changes timestamps — cheap, metadata-only.

2026-08-09 (orchestrator): the study is DONE —
docs/research/storage-lifecycle-2026-08.md. Headlines: /data is 78.1% (the
87% was transient fold-spill — reserve ~50GB transient headroom in all
planning); the DB file carries a verified 141.5GB COW-fork fossil from the
deleted reflink-snapshot era (459.7GB logical vs 601.2GB allocated) —
possibly reclaimable by one quiet-window restart (experiment pending);
growth ~50-75GB/yr steady state of which DB ~90%; a full-rebuild change
generation measured at ~87M rows (~6-8GB, 45% over ADR-0009's estimate) —
rebuild cadence is the controllable driver; earliest 90% breach 2027-Q4
(4 rebuilds/yr) / mid-2028 steady state; dump-and-reload escape hatch is
currently NOTIONAL (no dump tool + doesn't fit on-box — needs a temporary
second volume as standing policy). Deliverable delivered: C17 retention
trigger recommendation (prune superseded-epoch rows at 85% /data or ~175M
changes rows) — belongs next to the issue-46 epoch decision. Remaining:
Lennart decisions (C17 trigger, second-volume policy, housekeeping ack) +
the COW-reclaim experiment.
