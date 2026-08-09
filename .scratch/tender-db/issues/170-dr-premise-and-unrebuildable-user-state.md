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

2026-08-09 (orchestrator): the study half is DONE —
docs/research/dr-premise-2026-08.md. Headlines: honest RTO is ~1-1.5d (S1
layer rebuild) to ~4-6d (S2/S3 DB/volume loss), correcting both the "one
day" decision record and issue 23's "weeks"; unrebuildable state is <1MB
(users, api_tokens, webhook_endpoints, cursor/epoch continuity) with ZERO
copies anywhere since the 2026-08-06 snapshot removal; NEW finding — the
fetches registry is DB-resident, so a lost DB forces a full ~180GB
re-download even with an intact archive (cheap fix: re-register-from-disk
path). Recommendation menu §7: (b) tiny off-box user-state copy is the
pre-launch minimum (<100KB today, ~€0-4/mo) + two no-regret items (archive
re-register path, schedule D4). Remaining: Lennart's re-decision (queued in
bulk questions).
