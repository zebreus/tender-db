# 269 — zero DB snapshots exist: no offline-read path, no point-in-time artifact

Status: CLOSED 2026-08-22, same day — `tender-db-snapshot` is installed and has taken the first
snapshot: /data/db/snapshots/tender-db-1787374320.db, 472 GB apparent at ZERO additional disk
(XFS reflink=1 confirmed on /dev/md3; `cp --reflink=always` is instant, blocks shared until the
live DB diverges). Weekly timer (Sun 05:23 UTC) beside the watchdogs; skips loudly while a job
runs; the snapshot's WAL is folded+truncated on the copy so the verify suite's snapshot mode
works against it; prune keeps the newest 2 and can never delete the last. Documented as a
verification/forensics artifact, NOT disaster recovery — same volume; the raw archive remains
the rebuild path. Was: needs-triage — filed 2026-08-22 (owner, noticed during issue 109's
backtest attempt).
Kind: operational gap (backup / offline reads)
Blocked by: —
Relates to: 109 (whose backtest died on this), 102/107 (the verify tooling built AROUND
snapshots), prod-box-reads policy (data-page reads "run against a snapshot, never the serving
DB" — currently impossible)

## The gap

`find /data -name "*.db" -size +100G` returns exactly one file: the serving DB. Every snapshot
(including the issue-85-window one the 109 backtest needed) was pruned during disk-pressure
cleanups and none has been taken since. Three capabilities silently lapsed:

1. **The offline read path** — de1x_verify.sh's snapshot mode and the prod-box-reads policy both
   assume a snapshot exists; today any data-page read has nowhere to go but the serving DB.
2. **Point-in-time recovery** — the WAL'd live file is the ONLY copy of a 441 GB database whose
   archive can rebuild it, but only at ~day scale (fetch + parse + fold from scratch).
3. **Forensics** — issue 109's lesson: an incident window's state is only investigable if a
   snapshot from it survives.

Disk is not the constraint anymore: /data has ~1 TB free (40 % used) after the org merge.

## Shape

A snapshot job or timer: checkpoint (TRUNCATE) then reflink/copy the DB to
/data/db/snapshots/tender-db-<unixtime>.db with a 0-byte -wal sibling (the verify suite's
expectation), keep the newest N (2?), take one weekly in a quiet window — and REFUSE (loudly)
when a job is mid-write. Decide: plain cp (441 GB, minutes of IO) vs XFS reflink if the fs
supports it (instant, shared blocks). `df` said /dev/md3 — check the fs first.

## Acceptance

A fresh snapshot exists and is listed; the weekly cadence keeps exactly N; de1x_verify.sh's
snapshot mode works against it (witness W1 passes with a real baseline); the pruning rule can
never delete the LAST snapshot.
