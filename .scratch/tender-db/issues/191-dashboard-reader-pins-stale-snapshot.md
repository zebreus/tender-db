# 191 — the dashboard serves a pinned read snapshot: fresh `measured_at`, hour-old data, and a WAL checkpoint it blocks

Status: needs-triage
Kind: correctness (observability surface) + WAL hygiene
Blocked by: —
Relates to: 139 (cost a morning: a completed reclaim read as a failed one), 46/179 (the fold whose checkpoints it blocked)

## What happened (measured, 2026-08-12)

Reprocess job 612 stamped 1,898 quarantine rows reclaimed between 08:56 and 09:05 CEST. A
dashboard read at 09:37 carried `measured_at = 09:37` yet showed the quarantine panel exactly
as the DB stood at 08:55: job 610's 154 sibling flags (08:23) visible, job 612's stamps —
committed half an hour before the read — invisible. The numbers were not partially stale:
they were a perfectly consistent snapshot of 08:55, the moment the service (re)started for the
145277a deploy. After the next restart (53d4b06 deploy, ~11:17) the same endpoint immediately
served the converged numbers. Meanwhile, for the whole 2h01m fold (job 613), every per-chunk
checkpoint logged `busy=true … WAL not fully reclaimed` — a long-lived reader snapshot pinned
the WAL the entire time. Same reader, in all likelihood.

Two harms:

1. **Operators act on fiction.** Issue 139 burned a diagnosis morning because a successful
   reclaim was indistinguishable from a failed one: `measured_at` said "fresh", the rows said
   08:55. Any reclaim/relabel verification against `/api/dashboard` inherits this.
2. **WAL growth during long jobs.** A pinned snapshot blocks checkpointing for the duration
   (624 MB WAL observed mid-fold). The fold's per-chunk truncate attempts were all no-ops.

## Likely mechanism

A dashboard/measure reader connection opens (or leaves open) a read transaction at service
start and never ends it, so its WAL snapshot never advances (turso 0.7: a read tx pins its
snapshot; there is no auto-refresh). `/health`'s cursor read advanced during the same window,
so this is per-connection, not global.

## What

1. Find the reader that holds the long-lived read transaction (dashboard measure pool is the
   prime suspect; the SSE snapshot pool is the other candidate) and make it end/refresh its
   read transaction per measure run — a measure must observe a snapshot no older than its own
   `measured_at`, which then means what it says.
2. Add the regression check: after a write commits, a subsequent dashboard measure must reflect
   it (test at store level with two connections: write, measure, assert).
3. Consider surfacing snapshot age next to `measured_at` (cheap: compare a max(rowid)/cursor
   read on the measure connection vs a fresh one) so a pinned reader is self-announcing rather
   than silent.
