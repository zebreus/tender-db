# 170 — DR premise re-check + unrebuildable user state

Status: open — research gap #4 (docs/research/research-gaps-2026-08.md), before launch; needs a Lennart re-decision
Role: run-driver

The "no off-box backups, everything rebuildable in roughly a day" decision
(2026-07-19) traces to a measurement that excluded projection; issue 23 now
says weeks, and snapshots were removed 2026-08-06 (DR = re-ingest). Also:
accounts, API tokens, webhook registrations, and cursor/epoch continuity are
NOT derivable from sources — that state did not exist when the decision was
made. Study: honest end-to-end recovery estimate from measured stage rates,
enumeration of non-derivable tables, and a design note for a tiny off-box
copy of just user-state tables (MBs — not issue 23's 500 GB problem). Then
re-decide with the real numbers on the table.
