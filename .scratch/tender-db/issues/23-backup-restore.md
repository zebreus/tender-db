# 23 — Backup & restore for the production DB

Status: needs-verification

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

## Design & implementation

**Mechanism — `wal_checkpoint(TRUNCATE)` + file copy under the single writer**
(`crates/store/src/backup.rs`, `Db::snapshot`). Turso exposes no online-backup
API and `VACUUM INTO` OOM-kills the box at this scale
(docs/research/turso-scale.md §1), so checkpoint+copy is the only workable path.
It is made **consistent under concurrent writes** by holding the store's single
writer connection for the checkpoint + copy: turso is single-process and every
write funnels through that one connection, so with it held and the WAL
truncated into the main file, the `.db` is momentarily frozen — the copied bytes
are a self-contained, crash-clean SQLite file, *not* "a copy of a live DB".
Readers keep serving over WAL throughout (zero read downtime). The copy runs on
a blocking thread (a 100 GB copy must not stall the async runtime) with the
writer guard held across the await. The writer is released before verification.

**Verification (offline, on the copy):** open it independently, `PRAGMA
integrity_check`, and compare `COUNT(*)` of `notices` against the source
captured at snapshot time. A killed/torn copy can pass integrity_check alone
(turso-scale.md §1), so the row-count comparison is load-bearing. A copy that
fails is deleted, so a bad snapshot never lingers.

**Trigger lives in the app (no dev shortcuts in prod).** The snapshot is a
Supervisor **job** (`Spec::Snapshot`, `kind: "snapshot"`), which buys: it
serialises with ingestion (never concurrent with a write job — the single
writer), it lands in `job_log` (so `/admin/jobs` and the dashboard show it), and
queue/cancel work like any job. Triggers: (a) the daily 09:35 pipeline ends with
a snapshot, right after the projection folds the day's data; (b) on-demand
`POST /admin/jobs {"kind":"snapshot"}`. App orchestration (staging dir, local
ring, one-line summary) is isolated in `crates/app/src/snapshot.rs`; supervisor.rs
got only 4 minimal sibling-mirroring hooks.

**Visibility.** `job_log` gives the recent-runs history; a dedicated
`Db::last_snapshot_at()` feeds a new `Dashboard.snapshot_age`, rendered as
"last DB snapshot" age in the dashboard **System** panel.

**Ships where.** Snapshots stage on the data volume
(`TENDER_SNAPSHOT_DIR`, default `/data/snapshots`; local ring
`TENDER_SNAPSHOT_KEEP`, default 2). Off-box shipping + the durable 7-daily +
4-weekly ring is a systemd timer running `nix/backup-ship.sh` (rsync template).
Disk budget: `archive + (KEEP+1)·D` on the 500 GB volume — keep the local ring
small, rely on off-box for retention.

**Runbook.** docs/operations.md "Backups" rewritten: mechanism, on-demand
trigger, the required `ReadWritePaths += /data/snapshots` unit edit, disk
headroom, shipping options with costs, and the TESTED restore procedure
(restore into a scratch dir → integrity_check + row count → optional swap-in),
with measured durations from turso-scale.md pending a real post-deploy drill.

### Blocked decision (report to Lennart)

Off-box **destination** must be provisioned before shipping can be wired — I
cannot invent credentials. Recommended: a **Hetzner Storage Box BX11 (1 TB,
~€3.8/mo)** — off the prod host, rsync-native. Alternatives (Object Storage
~€6/TB, BX21 5 TB ~€12, a second Volume — same failure domain, weaker) are in the
runbook. Everything up to the shipping step is implemented; `backup-ship.sh`
refuses to run until `BACKUP_DEST` is set.

### Verification status

Code is **fully verified against HEAD in isolation** (store: clippy + 16 tests
green incl. a real snapshot+restore-open drill; app: clippy + 60 tests green,
incl. the ring-prune test). Final acceptance = a real snapshot+restore drill in
prod, which happens after deploy (record measured durations in the runbook then).

### Commit coordination note (resolved)

Sequenced behind issue-21's durable job queue (0d6adf2): the Snapshot job now
rides that queue. The supervisor hooks were re-applied against the new async
`push`/`enqueue_request`, and `Spec::Snapshot` (a unit variant) serialises into
`job_queue` and recovers across a restart — covered by
`supervisor::tests::a_snapshot_job_survives_a_restart` (the daily-pipeline
projection→snapshot case). Full store + app suites and clippy green.

### 2026-07-21 — Off-box destination deferred (team lead)

Lennart: no new external resources for now (no Storage Box). Decision:
run the LOCAL snapshot ring only (daily via the scheduler pipeline +
on-demand, KEEP=2 on /data/snapshots) — covers corruption and
fat-finger deletes; volume loss consciously accepted until a destination
exists. backup-ship.sh stays dormant (BACKUP_DEST unset by design).
Revisit when a destination is provided.

### 2026-07-22 — Volume resized to 1TB (Lennart + team lead)

Disk pressure during the backfill (67% at pkg 103/158, projected ~75-80%
post-project) would have left too little free space for a snapshot (a
full DB-file copy needs DB-sized headroom) — breaking the local ring at
full scale. Decision (Lennart): resize the Hetzner /data volume 500G→1TB.
Lennart grew the volume in the Hetzner console; team lead grew the XFS
filesystem online (`xfs_growfs /data`, metadata-only, no backfill
interruption). /data now 1000G, 34% used (340G/1000G, 660G free). The
archive (178G) stays on-box for reprocessing; DB + KEEP=2 snapshots now
fit with margin. Off-box shipping still deferred (no external
destination) — this is headroom for the LOCAL ring only; volume-loss
risk still consciously accepted.
