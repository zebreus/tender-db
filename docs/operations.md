# Operations runbook

Production is a single Hetzner VPS, `root@zebreus.click`, serving
<https://tenders.zebreus.click>. It runs Ubuntu, not NixOS — the app is built by
the flake and run under a hand-written systemd unit that mirrors
`nix/module.nix`'s hardening flags (ADR-0006). Design context: `CONTEXT.md`.

## Layout on the box

| Path | What |
| --- | --- |
| `/opt/tender-db/repo.git` | bare repo; `git push vps main` from the dev machine lands here |
| `/opt/tender-db/src` | working clone the build runs from |
| `/opt/tender-db/app` | symlink → the live nix store path (atomic switch on deploy) |
| `/opt/tender-db/app-result` | `nix build` result link (the most recently built bundle) |
| `/opt/tender-db/deployed-rev` | git rev of the running deploy |
| `/data/db/tender-db.db` | the Turso database (500 GB volume) |
| `/data/archive/<source>/…` | raw fetched packages, immutable |

`/opt/tender-db/` also holds research artifacts from the exploration phase
(sample packages, SDK checkouts, scan scripts). They are not part of the
deployment; leave them alone.

## Deploy

From a clean checkout on the dev machine:

```sh
./deploy.sh          # deploys main
./deploy.sh <ref>    # deploys any ref
```

The script pushes the ref to the VPS bare repo, builds `#tender-db` **on the
VPS** (it has the 1 Gb/s uplink and the warm nix store — never build the bundle
over the dev machine's ~100 kB/s link), atomically switches `/opt/tender-db/app`,
restarts the service, and health-checks the public URL. A failed build never
touches the symlink, so the old bundle keeps serving.

First build on a cold nix store compiles the whole Rust + wasm toolchain graph
and takes 20–40 minutes on the box's 4 cores. Subsequent deploys reuse the
cached dependency artifacts and take a couple of minutes. If a deploy might
outlive your connection, run it inside tmux on the box.

**Deploys are one at a time and never move production backwards** (f8bed0b —
two concurrent deploys raced once and the slower one would have regressed the
live rev). The VPS-side critical section (build → symlink switch → restart)
takes a `flock` on `/opt/tender-db/deploy.lock`; a second deploy while one holds
it aborts immediately with

```
another deploy holds /opt/tender-db/deploy.lock — aborting
```

Inside the lock it also refuses a regression: if the target rev is an ancestor
of the currently deployed rev (recorded in `/opt/tender-db/deployed-rev`) it
exits with `refusing regression: <rev> is an ancestor of deployed <rev>`. So
redeploying an older commit is a deliberate act — check out or revert to a
descendant rather than pointing `deploy.sh` at the old one. (For an emergency
revert to a *known-built* older bundle, use the symlink rollback below, which
bypasses the build and the guard.)

The health check at the end probes `GET /health` (see [Ingestion](#ingestion));
it must return `"ok":true` throughout, since ingestion runs in-process and the
readers keep serving over WAL during a load.

The deploy also writes the rev into a systemd drop-in
(`/etc/systemd/system/tender-db.service.d/rev.conf`,
`Environment=COMMIT_SHA=<rev>`) and reloads before the restart. The app reads
`COMMIT_SHA` at runtime (`crates/app/src/v1/mod.rs`, `rev()`), so `/health`,
`/v1`, `/_source`, and the dashboard's System panel report the actual deployed
revision — while the `nix build` never sees the rev and stays reproducible. A
plain local build (no `COMMIT_SHA` in the environment) reports `dev`.

Rollback: point the symlink at a previous store path and restart.

```sh
ssh root@zebreus.click 'ln -sfnT /nix/store/<old-path> /opt/tender-db/app.new \
  && mv -T /opt/tender-db/app.new /opt/tender-db/app && systemctl restart tender-db'
```

Old store paths survive until `nix store gc` runs; `nix profile`-style
generations are deliberately not used — the symlink is the whole mechanism.

## Service management

```sh
systemctl status tender-db
systemctl restart tender-db
systemctl stop tender-db
systemctl is-enabled tender-db      # enabled → survives reboot
```

Unit: `/etc/systemd/system/tender-db.service`, plus drop-ins under
`/etc/systemd/system/tender-db.service.d/` (`systemctl cat tender-db` shows the
merged result). Runs as the `tenderdb` system user (not `DynamicUser`, unlike
the NixOS module — the data in `/data` must outlive restarts).
`ProtectSystem=strict` with `ReadWritePaths=/data/db /data/archive`: the process
can write nowhere else, so a new state directory needs a unit edit, not just a
`mkdir`. The base unit sets `IP=127.0.0.1`, `PORT=8080`, `TENDER_DB`,
`TENDER_ARCHIVE`; the operator secret comes from the `admin.conf` drop-in — see
[Ingestion](#ingestion).

Anything writing to `/data` outside the service (a manual fetch run, say) must
leave the files owned by `tenderdb`, or the service loses access:

```sh
chown -R tenderdb:tenderdb /data/db /data/archive
```

## Ingestion

Ingestion runs **inside the server process** (ADR-0005, issue 16): the
Supervisor is a background task that owns the writer for its jobs while the
readers keep serving over WAL, so a production load has **zero downtime** and no
external process ever opens the DB (turso is single-process). You never stop the
service to load data.

Two ways jobs start:

- **Scheduler** — at 09:35 Europe/Berlin it enqueues the daily pipeline: on
  Mon–Fri a TED probe forward + re-fetch of the current day (the 09:30 CET
  finality window) and a `process`; every day a DÖE completed-day fetch (T+1,
  yesterday's date) + `process`; then one `project` that folds whatever landed.
  No operator action needed. Confirm a run fired by looking for a `probe` job
  (and the trailing `fetch`/`process`/`project`) with that morning's
  `started_at` in `GET /admin/jobs` → `recent[]`, or on the dashboard's
  Ingestion panel. The scheduler is a plain in-process timer (no cron/systemd
  timer), so it only runs while the service is up — a box that was down at 09:35
  simply misses that tick; re-drive it by hand via `/admin` if needed.
- **`/admin` API** — for manual loads, backfills and reprocessing. Gated by a
  preshared operator secret in `TENDER_ADMIN_SECRET`, sent as the
  `X-Admin-Secret` header and compared in constant time. **Unset ⇒ the whole
  `/admin` surface answers 404** (the feature is simply absent); a wrong secret
  is 403.

> ⚠️ **The `fetch` / `process` / `project` CLIs are dev tools for scratch
> databases only.** Never run them against the production DB: turso is
> single-process, so a CLI cannot open the file while the service is running, and
> stopping the service to run one is exactly the downtime this design removes.
> Everything below goes through `/admin` instead.

### The operator secret

Live setup on the box (never committed): the secret is an `EnvironmentFile`
holding one `KEY=VALUE` line, kept out of the main unit so it never appears in
`git` or `systemctl cat` of the checked-in unit.

```
/root/tender-admin-secret                          # mode 600, root-owned:
    TENDER_ADMIN_SECRET=<64 hex chars>

/etc/systemd/system/tender-db.service.d/admin.conf # the drop-in that wires it:
    [Service]
    EnvironmentFile=/root/tender-admin-secret
```

systemd reads the `EnvironmentFile` as root at start, before dropping to the
`tenderdb` user, so the `0600 root` file stays unreadable to the service user
and everyone else. First-time setup:

```sh
# On the VPS:
printf 'TENDER_ADMIN_SECRET=%s\n' "$(openssl rand -hex 32)" > /root/tender-admin-secret
chmod 600 /root/tender-admin-secret
mkdir -p /etc/systemd/system/tender-db.service.d
printf '[Service]\nEnvironmentFile=/root/tender-admin-secret\n' \
  > /etc/systemd/system/tender-db.service.d/admin.conf
systemctl daemon-reload && systemctl restart tender-db
journalctl -u tender-db -n5   # logs "admin: /admin API enabled"
```

Rotating it is an edit of the file + `systemctl restart tender-db`; the secret
lives only on the box. Unset it (remove the drop-in) and the entire `/admin`
surface goes back to answering 404.

### Driving it (examples)

`GET /admin/jobs` returns the running job's live progress, the queue, and the
recent-run log — the same shape the dashboard's Ingestion panel renders.

```sh
SECRET=$(cat /root/tender-admin-secret)
BASE=https://tenders.zebreus.click

# What is the importer doing right now?
curl -s -H "X-Admin-Secret: $SECRET" $BASE/admin/jobs | jq

# Fetch one TED daily, then process + project it (three sequential jobs).
curl -s -XPOST -H "X-Admin-Secret: $SECRET" -H 'content-type: application/json' \
  -d '{"kind":"fetch","source":"ted","package_kind":"daily","period":"2026-00136"}' $BASE/admin/jobs
curl -s -XPOST -H "X-Admin-Secret: $SECRET" -H 'content-type: application/json' \
  -d '{"kind":"process","source":"ted","package_kind":"daily","period":"2026-00136"}' $BASE/admin/jobs
curl -s -XPOST -H "X-Admin-Secret: $SECRET" -H 'content-type: application/json' \
  -d '{"kind":"project"}' $BASE/admin/jobs

# Backfill a DÖE monthly range (fans into one fetch per month + process + project).
curl -s -XPOST -H "X-Admin-Secret: $SECRET" -H 'content-type: application/json' \
  -d '{"kind":"backfill","source":"doe","range":["2024-01","2024-12"]}' $BASE/admin/jobs

# Cancel a still-queued job (the running one cannot be cancelled).
curl -s -XDELETE -H "X-Admin-Secret: $SECRET" $BASE/admin/jobs/42
```

Job payloads (`crates/app/src/supervisor.rs`, `JobRequest`): `{kind:
fetch|process|project|backfill, source?, package_kind?, period?, range?,
rebuild?, refetch?}`. Defaults: `source` `ted`, `package_kind` `daily`.
`process`/`project` accept no period to run the whole source (`process` with no
`period` re-parses every archived package of that source). `refetch:true`
re-downloads a known package (finality re-check). `project` with `rebuild:true`
drops and re-derives the whole canonical layer. `backfill` needs a `source`; for
`ted` it also needs a monthly `range` (`["2024-01","2024-12"]`), for `doe` the
range is optional (defaults to the whole 2022-12→now archive). A backfill fans
into one `fetch` per month, then one whole-source `process`, then one `project`,
so progress and cancellation stay per-package. Jobs run **one at a time** in
enqueue order — the writer is single anyway — so a fetch → process → project
sequence lands in order.

### Quarantine triage

A notice whose file matches no mapping profile (or fails a completeness check)
is **quarantined** rather than dropped (ADR-0004): the raw payload and a reason
are kept, and the rest of the package still ingests. Quarantine is the parser's
backlog, not data loss — the archive is intact, so a fixed parser recovers every
quarantined notice by reprocessing.

Triage loop:

1. **See it.** The dashboard (`/`, public) Data-quality panel shows the
   quarantine total, a breakdown by reason, and a recent sample (50). The same
   numbers ride each `process` job's `counts` line in `GET /admin/jobs` →
   `recent[]` (e.g. `… 3702 parsed, 15 quarantined …`).
2. **Diagnose.** Read the reason and the sampled payloads to find the unmapped
   element / customization ID / era the profile doesn't yet handle.
3. **Fix the parser** in `crates/ingest` (a new profile mapping, an ignore rule,
   or an inventory extension), with a fixture test, and **deploy** it
   (`./deploy.sh`).
4. **Reprocess.** Enqueue a `process` for the affected source (no `period` =
   the whole source) then a `project`. Processing re-parses from the archive and
   never re-downloads, so this is cheap and idempotent; quarantined notices that
   the new parser understands become canonical, and the quarantine count drops.

```sh
curl -s -XPOST -H "X-Admin-Secret: $SECRET" -H 'content-type: application/json' \
  -d '{"kind":"process","source":"ted"}' $BASE/admin/jobs
curl -s -XPOST -H "X-Admin-Secret: $SECRET" -H 'content-type: application/json' \
  -d '{"kind":"project"}' $BASE/admin/jobs
```

## Logs

```sh
journalctl -u tender-db -f              # follow
journalctl -u tender-db -n 200          # recent
journalctl -u tender-db --since '1 hour ago'
journalctl -u tender-db -p err          # errors only
```

nginx: `/var/log/nginx/access.log`, `/var/log/nginx/error.log`.

## Monitoring and alerting

The goal (issue 24) is that the operator learns within minutes when production
breaks, without watching dashboards. Two health endpoints, and one external
pinger that must live **off the box** (so it still fires when the app — or the
whole VPS — is down).

### The two health endpoints

| Endpoint | Cost | Answers | Used by |
| --- | --- | --- | --- |
| `GET /health` | DB-cheap, always fast | process is up + the database answers (`{"ok":true,…}`) | `deploy.sh`'s post-deploy check |
| `GET /health/deep` | one cursor read + one job-log scan + one `statvfs` | the above **plus** ingest freshness, last-job outcome and disk usage | the external pinger |

`/health` is deliberately narrow: its `ok` reflects only liveness, so a deploy
is never failed by a stale-ingest or full-disk condition unrelated to the new
build. **Do not widen it** — `deploy.sh` greps its `ok:true`.

`/health/deep` (`crates/app/src/v1/health.rs`) folds four signals into one
verdict, so a single external check covers uptime, freshness, job failures and
disk:

- **database** — the same cursor read `/health` does.
- **ingest_freshness** — unhealthy when no job has succeeded in **26 h**
  (`INGEST_STALE_SECS`). The scheduler lands a successful run at least daily
  (TED Mon–Fri, DÖE + projection every day), and 26 h carries a Friday success
  across the weekend. A box that has *never* run a job (fresh deploy, scheduler
  not yet fired) is reported healthy-but-unmeasured, not alarmed.
- **last_job** — unhealthy when the newest finished run in `job_log` has
  `outcome = "error"` (a Supervisor job ERRORED). Clears itself on the next
  success.
- **disk** — unhealthy once **90 %** (`DISK_FULL_FRACTION`) of the volume
  holding `TENDER_DB` (`/data`, the 500 GB Hetzner volume — same filesystem as
  the archive) is in use.

It answers **200** when every check passes and **503** when any fails, and the
JSON body names which check tripped:

```sh
curl -s https://tenders.zebreus.click/health/deep | jq
```

```json
{
  "ok": true,
  "rev": "…",
  "checks": {
    "database":         { "ok": true, "cursor": "12345" },
    "ingest_freshness": { "ok": true, "last_success_at": 1753000000, "age_secs": 3600, "threshold_secs": 93600 },
    "last_job":         { "ok": true, "kind": "project", "params": "rebuild=false", "outcome": "ok", "finished_at": 1753000000 },
    "disk":             { "ok": true, "used_fraction": 0.041, "free_bytes": 479000000000, "total_bytes": 500000000000, "threshold_fraction": 0.9 }
  }
}
```

### The external pinger (user action — no repo change ships this)

This repo's only git remote is the VPS bare repo (`git remote -v` → `vps
root@…:/opt/tender-db/repo.git`), **not GitHub**, so there is no place in the
codebase to run a scheduled check from — the pinger has to be an account the
operator creates on a third-party service. Pick one:

**Option A — a hosted uptime monitor (recommended, works today).** Free tiers
poll every 3–5 min and email/push on failure. Because `/health/deep` returns a
non-2xx (503) when anything is wrong, a plain HTTP monitor covers uptime,
freshness, disk and job failures with no extra configuration.

- **UptimeRobot** (<https://uptimerobot.com>, free: 50 monitors, 5-min interval):
  Add New Monitor → type **HTTP(s)** → URL `https://tenders.zebreus.click/health/deep`
  → interval 5 min → alert contact = `lennarteichhorn@gmail.com`. Its default
  rule (2xx/3xx = up, 4xx/5xx = down) makes the 503 an alert. Optionally add a
  **Keyword** monitor on the same URL, alert when `"ok": true` is *not present*,
  as a second signal.
- **Better Stack / Better Uptime** (<https://betterstack.com>, free: 10 monitors,
  3-min checks, mobile push app): Create Monitor → URL as above → "expect status
  code 200" → email + push. Push to a phone is the fastest path to "within
  minutes".

**Option B — a scheduled GitHub Actions check (credential-free, but needs the
repo on GitHub first).** This ties into the AGPL open item below (publishing the
repo). Once the repo has a GitHub remote, a scheduled workflow that curls the
endpoint is free and needs *no* secrets — a failed workflow run emails the repo
owner automatically:

```yaml
# .github/workflows/health-ping.yml
name: health-ping
on:
  schedule: [{ cron: "*/10 * * * *" }]   # every 10 min (GitHub's floor for cron)
  workflow_dispatch:
jobs:
  ping:
    runs-on: ubuntu-latest
    steps:
      - run: curl --fail --max-time 15 https://tenders.zebreus.click/health/deep
```

GitHub's cron floor is ~10 min and schedules can lag under load, so Option A
reacts faster; use B only if a GitHub-native, account-free check is preferred.
Either way this is **a decision + setup step for Lennart** — nothing in the
deploy ships it.

### Test procedure

- **Freshness / job / disk logic** is unit-tested against crafted signals
  (`crates/app/src/v1/health.rs` `#[cfg(test)]`) and end-to-end
  (`crates/app/tests/api.rs::the_deep_health_probe_reports_operational_health`:
  a fresh box is 200, a recorded `error` run flips it to 503).
- **The live alert path** (the acceptance drill, run *after* a deploy, in a
  quiet window with no backfill in flight): `systemctl stop tender-db` on the
  box, confirm the pinger's notification arrives within its interval, then
  `systemctl start tender-db`. To exercise the freshness signal without waiting
  26 h, temporarily lower `INGEST_STALE_SECS`, deploy, and confirm
  `/health/deep` reports `ingest_freshness.ok = false` — then revert.

## TLS and nginx

- vhost: `/etc/nginx/sites-available/tenders.zebreus.click` (symlinked into
  `sites-enabled`; the stock `default` site is removed).
- cert: `/etc/letsencrypt/live/tenders.zebreus.click/{fullchain,privkey}.pem`,
  issued by `certbot --nginx`. Renewal is automatic via certbot's systemd timer
  (`systemctl list-timers 'certbot*'`); dry-run it with
  `certbot renew --dry-run`.
- The vhost sets `proxy_buffering off`, `proxy_cache off` and a 24 h read
  timeout so SSE streams flow unbuffered, and enables HTTP/2. Certbot manages
  the `listen 443 ssl` / cert lines; the `http2 on;` line is ours — re-check it
  after any certbot config rewrite.

```sh
nginx -t && systemctl reload nginx
```

## Disk watch

Two filesystems, watched separately:

- **`/data` — the 500 GB Hetzner volume**, the one that grows with ingestion.
  It carries both the raw archive and the database, because the parsed DB alone
  will not fit the 75 GB root disk (text satellites dominate — pilot-sizing.md).
  - `/data/archive/<source>/…` — raw fetched packages, immutable, append-only.
    TED under `ted/{daily,monthly}/`, DÖE under `doe/{daily,monthly}/`.
  - `/data/db/tender-db.db` (+ `-wal`) — the Turso database.
- **`/` — the 75 GB root disk.** Pressure here is almost always the nix store
  (build artifacts + old bundles), not application data.

```sh
df -h /data /
du -sh /data/archive/* /data/db/*        # where the volume budget is going
```

A full backfill is the thing to plan for: the TED archive is the big one, and a
complete DÖE backfill is ~3 GB of ZIPs plus the projected rows. At a few percent
of 500 GB today there is ample headroom, but backfills are where it moves — keep
an eye on `df /data` while one runs.

Root-disk reclaim: `nix store gc` deletes unreferenced store paths — **including
old bundles you might want for a rollback**, so switch the symlink to the bundle
you want to keep before running it, or verify the current one is safe.

Note that nothing outside the service writes to `/data/db`: turso is
single-process, so the running server holds the database open exclusively (see
the Ingestion rule above). A scratch `*.db` may appear here from earlier
dev/verification work — harmless, but never point a CLI at the production file.

## Backups

The DB is worth backing up (weeks of processing to rebuild); the raw archive is
**not** (re-fetchable from TED/DÖE, ~200 GB — never back it up). Issue 23 wires
a consistent online snapshot of the DB, staged on the volume and shipped off-box.

### How a snapshot is taken (the mechanism)

Turso has no online-backup API, and `VACUUM INTO` OOM-kills the box at this
scale (`docs/research/turso-scale.md` §1). The only workable mechanism is
`wal_checkpoint(TRUNCATE)` + a file copy — and it is made **consistent under
concurrent writes** by taking the store's single writer for the length of the
copy (`crates/store/src/backup.rs`, `Db::snapshot`):

1. Acquire the single writer connection (all writes funnel through it; turso is
   single-process, so this is every writer there is).
2. `PRAGMA wal_checkpoint(TRUNCATE)` — fold the WAL into the main file and
   truncate the `-wal` to zero. The main `.db` is now self-contained.
3. Copy the `.db` to the staging dir on a blocking thread, **with the writer
   still held**, so no write and no auto-checkpoint can touch the bytes
   mid-copy. This is why it is *not* "a file copy of a live DB": the DB is
   momentarily frozen and checkpointed. Readers keep serving over WAL
   throughout — a snapshot has zero read downtime.
4. Release the writer, then **verify the copy offline**: open it independently,
   `PRAGMA integrity_check`, and compare `COUNT(*)` of `notices` against the
   source. A copy that does not match is deleted and the run fails (a killed
   copy can pass integrity_check alone — turso-scale.md §1 — so the row-count
   comparison is load-bearing).

The snapshot is a **Supervisor job** (`kind: snapshot`), so it serialises with
ingestion — it never runs concurrently with a fetch/process/project — and lands
in `job_log`, visible in `GET /admin/jobs` → `recent[]` and on the dashboard.
The dashboard's **System** panel shows *last DB snapshot* age.

Triggers:

- **Scheduled** — the daily pipeline (09:35 Europe/Berlin) ends with a snapshot,
  right after the projection folds the day's data.
- **On demand** — `POST /admin/jobs {"kind":"snapshot"}` (for the restore drill
  or an ad-hoc backup):

  ```sh
  curl -s -XPOST -H "X-Admin-Secret: $SECRET" -H 'content-type: application/json' \
    -d '{"kind":"snapshot"}' $BASE/admin/jobs
  ```

Config (env, set in the systemd unit): `TENDER_SNAPSHOT_DIR` (default
`/data/snapshots` in prod), `TENDER_SNAPSHOT_KEEP` (local ring size, default 2).
Snapshots are named `tender-db-<unix>.db`.

> ⚠️ **Unit change required before deploy.** `ProtectSystem=strict` with
> `ReadWritePaths=/data/db /data/archive` means the service cannot write
> `/data/snapshots` until it is added. On the box:
>
> ```sh
> mkdir -p /data/snapshots && chown tenderdb:tenderdb /data/snapshots
> # add /data/snapshots to ReadWritePaths= in the unit (or a drop-in), then:
> systemctl daemon-reload && systemctl restart tender-db
> ```

### Disk headroom

`/data` (500 GB) must hold **archive + DB + one snapshot in flight**. At DB
size *D*, staging one snapshot needs another *D* free; the local ring keeps
`TENDER_SNAPSHOT_KEEP` of them, so budget `archive + (KEEP+1)·D`. With the
archive at ~200 GB and *D* heading toward ~100 GB, keep `KEEP` small locally
(2) and rely on off-box for the long retention ring. `df -h /data` before
enabling the daily snapshot; a snapshot job that runs out of space fails
cleanly (the copy errors, no partial file is kept) without touching the live DB.

### Shipping off-box (retention ring)

The local staging dir is a short ring; the durable **7 daily + 4 weekly** ring
lives off-box. Shipping is a systemd timer on the box that pushes the newest
staged snapshot to the destination and prunes the remote ring — a template is
in `nix/backup-ship.sh` (rsync-based). **This step is blocked on a destination
decision** (see below); until it is wired, snapshots accumulate only in the
small local ring, so off-box durability is not yet in place.

Destination options (report to Lennart):

| Option | Rough monthly cost | Notes |
| --- | --- | --- |
| Hetzner **Storage Box** BX11 (1 TB) | ~€3.8 | Same DC, rsync/BorgBackup/SFTP; simplest; must be **ordered** (new credentials). |
| Hetzner Storage Box BX21 (5 TB) | ~€12 | Headroom for many weeklies + the archive if ever wanted. |
| Hetzner **Object Storage** (S3) | ~€6/TB | S3 API; needs an `rclone`/`aws` client + access keys. |
| A second Hetzner **Volume** | ~€0.044/GB (100 GB ≈ €4.4) | Same failure domain as prod (both are Hetzner block storage) — weaker than a Storage Box for disaster recovery. |

Recommendation: a **Storage Box BX11** (cheapest, off the prod host, rsync-native).
Blocked because it must be ordered and its SSH credentials provisioned — I
cannot invent those. Once ordered, drop the key on the box, point
`nix/backup-ship.sh` at it, and enable the timer.

### Restore procedure (TESTED)

Restoring is a plain file copy — **no turso tooling, no app** — because a
snapshot is an ordinary SQLite file. Drill it into a scratch dir and open it;
never overwrite the live DB in place.

```sh
# 1. Pick a snapshot (local ring, or pull one back from off-box first).
ls -lh /data/snapshots/                      # newest is the freshest
SNAP=/data/snapshots/tender-db-<unix>.db

# 2. Restore into a scratch dir (NOT /data/db).
mkdir -p /data/restore && cp "$SNAP" /data/restore/tender-db.db

# 3. Verify it offline — integrity + a row count sanity check.
sqlite3 /data/restore/tender-db.db 'PRAGMA integrity_check;'      # expect: ok
sqlite3 /data/restore/tender-db.db 'SELECT COUNT(*) FROM notices;'

# 4. (Full recovery) stop the service, swap the file in, restart.
systemctl stop tender-db
mv /data/db/tender-db.db /data/db/tender-db.db.bak    # keep the old one aside
cp /data/restore/tender-db.db /data/db/tender-db.db
rm -f /data/db/tender-db.db-wal /data/db/tender-db.db-shm   # snapshot is self-contained
chown tenderdb:tenderdb /data/db/tender-db.db
systemctl start tender-db
curl -s https://tenders.zebreus.click/health   # expect {"ok":true,…}
```

Steps 1–3 are the **read-only drill** (the issue's acceptance test) and are safe
to run anytime against production snapshots; step 4 is the real recovery.

**Measured durations** (from `docs/research/turso-scale.md`, 10 GB DB on this
VPS; scale ~linearly): checkpoint+copy **~18 s / 10 GB**; offline
`integrity_check` **~7 min / 10 GB** (cold, IO-bound); `COUNT(*)` ~2 s. The
restore copy is the same order as the snapshot copy. *A production drill with
real end-to-end timings is pending the first post-deploy snapshot (issue 23
verification step) — record the measured numbers here once run.*

`VACUUM INTO` is forbidden at scale (OOM — turso-scale.md §1).

## Open items

Known gaps in the production setup, tracked here so they aren't rediscovered:

- **Reboot survival is unexercised.** The unit is `enabled` (survives reboot by
  configuration), but no actual reboot has been done to confirm the service, the
  `/data` mount, and nginx all come back clean. Needs a deliberate quiet window —
  do it when no ingestion/backfill is in flight, then verify `systemctl status
  tender-db` and `curl https://tenders.zebreus.click/health`.
- **The external pinger is not set up yet.** `/health/deep` ships and covers
  uptime, freshness, disk and job failures, but nothing off-box watches it until
  Lennart creates a hosted-monitor account (or publishes the repo and enables
  the GitHub Actions check) — see [Monitoring and alerting](#monitoring-and-alerting).
  Until then a production outage is still silent.
- **AGPL source offer is a written offer, not a public repo.** `/_source`
  currently tells a network user to request the Corresponding Source from the
  operator (AGPL §13 permits this). Publishing the repo at a stable public URL
  and pointing `/_source` at it is cleaner — a Lennart decision, pending.
