# 170 — DR premise re-check + unrebuildable user state

Status: RESOLVED 2026-08-23 — Lennart re-decided with the real numbers on the table: NO backups
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

## RESOLVED (2026-08-23) — the re-decision: no backups, risk accepted

Lennart, directly, with the study's numbers presented (honest RTO 1–6 days; <1 MB of
unrebuildable user state with zero copies; ~180 GB re-download on DB loss): **no backups of any
kind — "we won't lose our db, so we do NOT need backups; it's fine if prod crashes and we lose
data."** The off-box user-state copy recommendation (§7b) is DECLINED. That is the owner's
record of an explicit, informed risk acceptance; the exposure enumeration in
dr-premise-2026-08.md stays as the statement of what that acceptance covers.

What stands regardless (already landed, not backups): the archive re-register-from-disk path
(`RegisterArchive`, issue 23) and the weekly D4 re-hash probe (issue 173). No further work.
