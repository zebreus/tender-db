# 169 — storage lifecycle model at 0.5 TB and beyond

Status: open — research gap #3 (docs/research/research-gaps-2026-08.md), NOW (/data ~87% full)
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
