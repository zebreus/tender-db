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

> **A deploy builds from a COMMITTED SHA in a CLEAN tree — never the shared working
> tree.** *"From a clean checkout"* above is the rule, not a stylistic preference, and
> this is what it prevents.
>
> Several agents share this worktree. On 2026-08-05 a deploy was authorised at
> `80e2365`; `HEAD` was `80e2365`, and the tree was **not** — two files carried
> uncommitted work, and they were the two files the deploy shipped
> (`crates/app/src/supervisor.rs`, `crates/store/src/lib.rs`). A teammate was mid-edit
> on the very constant the deploy's review had cleared. Building "what's here" would
> have put un-committed, un-reviewed code into production, and a deploy is the one
> action where that is unrecoverable.
>
> So, before staging:
>
> ```sh
> git status --short crates/     # MUST be empty
> git log --oneline -1           # the SHA you are deploying, read from output
> ```
>
> If the tree is dirty, deploy from a fresh checkout of the SHA instead — never
> `git stash` someone else's work to clear the path.
>
> **Two corollaries, both learned the same day.** A pre-deploy test run against a dirty
> tree certifies a tree nobody will deploy — the green describes the wrong artifact, so
> run the gate on the committed tip. And **a review clearance is bound to the artifact it
> was given against**: if the code changes after a review, the clearance does not
> automatically follow it, and the reviewer has to say so. See
> [`agents/instrument-discipline.md`](agents/instrument-discipline.md).

The script pushes the ref to the VPS bare repo, builds `#tender-db` **on the
VPS** (it has the 1 Gb/s uplink and the warm nix store — never build the bundle
over the dev machine's ~100 kB/s link), atomically switches `/opt/tender-db/app`,
restarts the service, and health-checks the public URL. A failed build never
touches the symlink, so the old bundle keeps serving. The ssh connection carries
keepalives (`ServerAliveInterval=15`, `ServerAliveCountMax=4`) so a dropped TCP
link fails the deploy cleanly instead of hanging forever on a dead socket
(2026-07-21 incident).

Build times, warm store (the normal case): a **server-code-only** change
rebuilds in **~4.6 min**; a change that touches the **dependency graph**
(`Cargo.lock`, a new crate, a toolchain bump) is **~10.5 min**. A first build on
a *cold* store compiles the whole Rust + wasm toolchain graph and takes far
longer (tens of minutes on the box's 4 cores). If a deploy might outlive your
connection, run it inside tmux on the box.

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
readers keep serving over WAL during a load. The script polls for up to **120 s**
(one probe/second) before it declares the deploy failed — that grace exists
because the **first open after a schema change migrates**: additive `ALTER`s plus
`CREATE INDEX IF NOT EXISTS` build once over the whole table (tens of seconds on
the multi-million-row `notices`), which can hold `/health` past a minute. That is
startup work, not failure — the migrating-open pause clears itself once the index
is built and every subsequent open is instant.

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
  yesterday's date) + `process`; then one `project` that folds whatever landed,
  and finally one `snapshot` of the freshly folded result. No operator action
  needed. Confirm a run fired by looking for a `probe` job (and the trailing
  `fetch`/`process`/`project`/`snapshot`) with that morning's `started_at` in
  `GET /admin/jobs` → `recent[]`, or on the dashboard's Ingestion panel. The
  scheduler is a plain in-process timer (no cron/systemd timer), so it only runs
  while the service is up — a box that was down at 09:35 simply misses that tick;
  re-drive it by hand via `/admin` if needed.
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
fetch|process|project|backfill|snapshot, source?, package_kind?, period?, range?,
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

### Restarts and recovery (no manual re-enqueue)

The job queue is **durable** (ADR-0007): every outstanding job is a row in
`job_queue`, written before it enters the in-memory queue and deleted only when
the job concludes or is cancelled. So a deploy, crash, or restart **does not lose
the queue** — the Supervisor rebuilds it from the durable rows at startup, before
the worker or scheduler run, and the job that was mid-run when the process died
comes back at the front and re-runs. **You never re-enqueue by hand after a
restart.** Re-runs are safe because ingestion is idempotent (notice identity is
`(source, publication_id, content_hash)`; the projection is a pure function of
the parsed layer), so a repeated walk inserts nothing new.

A long `process` job also carries a **resume cursor** (issue 32): the last
package it fully committed, advanced only after every member of that package
landed. On restart it resumes at the *next* package instead of re-walking years
of archive from the start — the interrupted (partial) package re-runs and dedups.
A *freshly enqueued* `process` never inherits a cursor, so a deliberate whole-
source reprocess still walks everything. The only operator-visible trace is a log
line: `job N resumes after <period> (K package(s) already done)`.

The daily scheduler is a plain in-process timer, so a box that was **down** at
09:35 misses that tick entirely (not a queued job to recover — it never fired);
re-drive it by hand via `/admin` if needed.

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

The goal (issue 24) is that the operator learns when production breaks without
watching dashboards. Two health endpoints, and one external watcher that must
live **off the box** (so it still fires when the app — or the whole VPS — is
down). The watcher today is a Claude scheduled routine polling every ~4 h — see
[The off-box watcher](#the-off-box-watcher-an-external-claude-scheduled-routine).

### The two health endpoints

| Endpoint | Cost | Answers | Used by |
| --- | --- | --- | --- |
| `GET /health` | DB-cheap, always fast | process is up + the database answers (`{"ok":true,…}`) | `deploy.sh`'s post-deploy check |
| `GET /health/deep` | one cursor read + one job-log scan + one `statvfs` | the above **plus** ingest freshness, last-job outcome and disk usage | the external watcher routine |

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
  the archive) is in use. The check also reports `wal_bytes`, the size of the
  `-wal` sidecar (issue 42): **informational only** — a large WAL is expected
  mid-backfill and never flips the verdict — but the alerting routine watches it
  for a runaway (see [Disk watch](#disk-watch)).

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
    "disk":             { "ok": true, "used_fraction": 0.041, "free_bytes": 479000000000, "total_bytes": 500000000000, "wal_bytes": 3500000000, "threshold_fraction": 0.9 }
  }
}
```

### The off-box watcher (an external Claude scheduled routine)

The watcher must live **off the box** so it still fires when the app — or the
whole VPS — is down, and it must need no third-party account. The chosen shape is
an **external Claude scheduled routine** that polls `https://tenders.zebreus.click/health/deep`
**every 4 h** and alerts on any non-200 (or no response). Because `/health/deep`
folds uptime, ingest freshness, last-job outcome and disk into one 503-or-200
verdict, that single GET covers everything; the routine reads the JSON body to
name which check tripped, and can watch `disk.wal_bytes` for a mid-backfill WAL
runaway. This replaces the earlier plan of a hosted uptime monitor
(UptimeRobot/Better Stack) or a GitHub Actions cron — both were rejected together
with the storage box and a public GitHub repo (no external resources, 2026-07-21).

The 4 h cadence is a deliberate trade: cheap and account-free, at the cost of up
to ~4 h to notice an outage rather than the "within minutes" issue 24 first
aimed at. Acceptable for a single-operator, rebuildable dataset; tighten the
interval if that ever stops being true. The routine is configured outside this
repo, so nothing in the deploy ships it — it is an operator-owned schedule.

### Test procedure

- **Freshness / job / disk logic** is unit-tested against crafted signals
  (`crates/app/src/v1/health.rs` `#[cfg(test)]`) and end-to-end
  (`crates/app/tests/api.rs::the_deep_health_probe_reports_operational_health`:
  a fresh box is 200, a recorded `error` run flips it to 503).
- **The live alert path** (the acceptance drill, run *after* a deploy, in a
  quiet window with no backfill in flight): `systemctl stop tender-db` on the
  box, confirm the watcher routine alerts on its next poll (within ~4 h), then
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
  - `/data/db/tender-db.db` (+ `-wal`) — the Turso database. The raw archive is
    the large static tenant (~178 GB and barely moving between backfills); the DB
    plus its WAL is what grows during a load.
- **`/` — the 75 GB root disk.** Pressure here is almost always the nix store
  (build artifacts + old bundles), not application data.

```sh
df -h /data /
du -sh /data/archive/* /data/db/*        # where the volume budget is going
ls -lh /data/db/tender-db.db-wal         # WAL size during a bulk load (issue 42)
```

A full backfill is the thing to plan for: the TED archive is the big one, and a
complete DÖE backfill is ~3 GB of ZIPs plus the projected rows. Watch `/data`
against a **~70 % operational guard** while a backfill runs — a self-imposed
ceiling well under the `/health/deep` 90 % alarm, leaving room to grow the volume
or pause the load before a write can fail mid-ingest. (The guard is a run-driver
convention, not a code constant; the only threshold in code is the 90 % deep-
health disk check.)

**WAL growth during bulk loads (issue 42).** Turso never auto-checkpoints fresh
frames, so a long `process`/`project` run would otherwise pile the whole run's
writes into `tender-db.db-wal` unbounded — it reached **13 GB and climbing**
during the first backfill. The processor now folds the WAL back at each package
boundary (a `wal_checkpoint(TRUNCATE)` at the writer-idle moment right after a
package commits), so the `-wal` stays bounded (single-digit GB) instead of
tracking the whole run. Idle pooled readers do not pin it; a reader mid-scan only
delays reclaim to the next package. The size is surfaced as `disk.wal_bytes` on
`/health/deep` — a large WAL mid-backfill is expected and never alarms, but a
*monotonically climbing* one across many packages is the signal that the
checkpoint is not reclaiming (investigate before `/data` fills).

Root-disk reclaim: `nix store gc` deletes unreferenced store paths — **including
old bundles you might want for a rollback**, so switch the symlink to the bundle
you want to keep before running it, or verify the current one is safe.

Note that nothing outside the service writes to `/data/db`: turso is
single-process, so the running server holds the database open exclusively (see
the Ingestion rule above). A scratch `*.db` may appear here from earlier
dev/verification work — harmless, but never point a CLI at the production file.

## Backups

The DB is worth backing up (weeks of processing to rebuild); the raw archive is
**not** (re-fetchable from TED/DÖE, ~178 GB — never back it up). Issue 23 wires
a consistent online snapshot of the DB, staged in a small **local ring** on the
volume. There is **no off-box destination for now** (a deliberate decision — see
[No off-box destination](#no-off-box-destination-local-ring-only) below).

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
Its log line carries the real numbers (size, notice count, seconds the writer was
frozen, seconds spent verifying), so per-run timings are recorded automatically.
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
archive at ~178 GB and *D* heading toward ~100 GB, keep `KEEP` small (2). `df -h
/data` before enabling the daily snapshot; a snapshot job that runs out of space
fails cleanly (the copy errors, no partial file is kept) without touching the
live DB.

### No off-box destination (local ring only)

Snapshots live **only in the local ring** on `/data` — there is deliberately no
off-box copy. The earlier plan (a Hetzner Storage Box, an rsync `backup-ship.sh`
timer, a 7-daily-+-4-weekly remote ring) was **dropped** together with the
external pinger and a public GitHub repo (no external resources, 2026-07-21). The
accepted risk is explicit and matches CONTEXT.md: **everything is rebuildable** —
the canonical layer re-derives from the archive, the archive re-fetches from
TED/DÖE — at a cost of roughly a day of processing, so a lost `/data` volume is
recoverable without an off-box backup. The local ring exists to make the *common*
recovery (a bad projection, an accidental drop) a fast file-copy rather than a
full rebuild; it is **not** disaster recovery, because it shares the volume's
failure domain. Revisit if the dataset ever stops being cheaply rebuildable.

### Restore procedure (TESTED)

Restoring is a plain file copy — **no turso tooling, no app** — because a
snapshot is an ordinary SQLite file. Drill it into a scratch dir and open it;
never overwrite the live DB in place.

```sh
# 1. Pick a snapshot from the local ring.
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
restore copy is the same order as the snapshot copy. Each snapshot job already
records its own **frozen** and **verify** seconds in its `job_log` line
(`GET /admin/jobs` → `recent[]`), so the real per-run numbers accrue there. *A
full end-to-end restore drill on production-scale data is still pending (issue 23
verification step) — record the measured restore timings here once run.*

`VACUUM INTO` is forbidden at scale (OOM — turso-scale.md §1).

## Verification

Two black-box binaries check a *running* instance from the outside — they talk
only to its public API (default `https://tenders.zebreus.click`), never to the DB
or the box, so they run from the dev machine. Both use the account-gated
read-only `/v1/sql` endpoint for the counting queries and need an API token
(`--token`, or `TENDER_API_TOKEN`); without one the token-gated checks are
reported *skipped*, never silently passed.

- **`verify`** (`crates/ingest/src/bin/verify.rs`) — the standing **acceptance
  harness**: pass/fail against *external* ground truth. It checks per-year TED
  coverage against the vendored counts, cross-checks a few eForms days against the
  live TED Search API (set membership), and walks one real notice per format era
  through the API (Notice → Tender → award → winner). Exits non-zero on any
  executed check that fails or could not run. **Run it after a deploy and after a
  backfill** to confirm the instance still meets ground truth. A partially
  backfilled instance honestly reports failure for the years it does not yet hold.
- **`data-quality`** (`crates/ingest/src/bin/data-quality.rs`) — the descriptive
  sibling: **no pass/fail**, it *measures* how complete the imported data is
  (per-era field completeness, award linkage, results materialisation, TED↔DÖE
  merge) with bounded `GROUP BY` aggregates, safe against the live rate-limited
  SQL endpoint. **Run it to read the numbers** after a parser change or backfill,
  when you want more depth than the dashboard's data-quality panel.

```sh
# From a dev checkout (nix provides the toolchain); --json for machine output.
TENDER_API_TOKEN=<token> cargo run -p ingest --bin verify
TENDER_API_TOKEN=<token> cargo run -p ingest --bin data-quality
```

## Open items

Known gaps in the production setup, tracked here so they aren't rediscovered:

- **Reboot survival is unexercised.** The unit is `enabled` (survives reboot by
  configuration), but no actual reboot has been done to confirm the service, the
  `/data` mount, and nginx all come back clean. Needs a deliberate quiet window —
  do it when no ingestion/backfill is in flight, then verify `systemctl status
  tender-db` and `curl https://tenders.zebreus.click/health`.
- **The off-box watcher is a per-4h Claude routine, not sub-minute.** `/health/deep`
  ships and covers uptime, freshness, disk and job failures, and the external
  Claude scheduled routine polls it every ~4 h (see
  [Monitoring and alerting](#monitoring-and-alerting)). That means an outage can
  go unnoticed for up to ~4 h — accepted for now given the rebuildable dataset
  and single operator; tighten the interval if that changes.
- **AGPL source offer is a written offer, not a public repo.** `/_source`
  currently tells a network user to request the Corresponding Source from the
  operator (AGPL §13 permits this). A public GitHub repo was declined for now (no
  external resources, 2026-07-21), so the written offer stands.
