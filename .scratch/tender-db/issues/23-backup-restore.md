# 23 — Backup & restore for the production DB

Status: ready-for-agent

Everything lives on one Hetzner VPS volume: the raw archive (~200 GB,
re-fetchable from TED/DÖE so NOT worth backing up) and the canonical DB
(weeks of processing time to rebuild — worth backing up). Today a volume
failure or fat-fingered delete loses the DB with no recovery path except
a full re-backfill.

Scope:
- Periodic consistent DB snapshot (the store is SQLite/turso-family —
  use its online-backup mechanism, never a file copy of a live DB),
  shipped off-box (e.g. Hetzner storage box / object storage), retention
  of a small ring (e.g. 7 daily + 4 weekly).
- Per "no dev shortcuts in prod": the snapshot trigger lives in the app
  (supervisor scheduled job + /admin visibility), the shipping can be a
  systemd timer on the box.
- A documented, TESTED restore procedure in the operations runbook —
  restore one snapshot into a scratch dir and open it read-only as the
  test.
- Mind disk headroom during snapshot (DB may reach ~100+ GB; /data must
  hold archive + DB + one snapshot in flight).

Acceptance: snapshots appear off-box on schedule; a restore drill
documented in the runbook with measured duration; dashboard/admin shows
last-snapshot age.
