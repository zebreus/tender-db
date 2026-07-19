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

Unit: `/etc/systemd/system/tender-db.service`. Runs as the `tenderdb` system
user (not `DynamicUser`, unlike the NixOS module — the data in `/data` must
outlive restarts). `ProtectSystem=strict` with `ReadWritePaths=/data/db
/data/archive`: the process can write nowhere else, so a new state directory
needs a unit edit, not just a `mkdir`. Env is set in the unit
(`IP=127.0.0.1`, `PORT=8080`, `TENDER_DB`, `TENDER_ARCHIVE`,
`TENDER_ADMIN_SECRET` — see [Ingestion](#ingestion)).

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

- **Scheduler** — every weekday at 09:35 Europe/Berlin it probes the newest TED
  daily forward (and re-fetches the current day for the 09:30 finality window),
  fetches the DÖE completed day (T+1), then processes and projects. No operator
  action needed for the daily cadence.
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

Generated once and stored on the box, never committed:

```sh
# On the VPS, first-time setup:
openssl rand -hex 32 > /root/tender-admin-secret
chmod 600 /root/tender-admin-secret
```

Wire it into the systemd unit so the service reads it at start
(`/etc/systemd/system/tender-db.service`, `[Service]` section) — either inline
or, to keep it out of `systemctl show`, via a credential file:

```ini
Environment=TENDER_ADMIN_SECRET=<paste the hex here>
# or: EnvironmentFile=/root/tender-admin-secret   (as KEY=VALUE)
```

```sh
systemctl daemon-reload && systemctl restart tender-db
journalctl -u tender-db -n5   # logs "admin: /admin API enabled"
```

Rotating it is an edit + `systemctl restart`; the secret lives only on the box.

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

Job payloads: `{kind: fetch|process|project|backfill, source?, package_kind?,
period?, range?, rebuild?, refetch?}`. `process`/`project` accept no period to
run the whole source. `refetch:true` re-downloads a known package (finality
re-check). Jobs run **one at a time** in enqueue order — the writer is single
anyway — so a fetch → process → project sequence lands in order.

## Logs

```sh
journalctl -u tender-db -f              # follow
journalctl -u tender-db -n 200          # recent
journalctl -u tender-db --since '1 hour ago'
journalctl -u tender-db -p err          # errors only
```

nginx: `/var/log/nginx/access.log`, `/var/log/nginx/error.log`.

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

The DB does not fit the 75 GB root disk — DB and archive both live on the
500 GB volume at `/data`. Watch it, especially during backfill:

```sh
df -h /data /
du -sh /data/db /data/archive
```

Root disk pressure is usually the nix store; reclaim with
`nix store gc` (this deletes unreferenced store paths, including old bundles you
might want for rollback — switch the symlink back first if unsure).

## Backups

There are none, by design. Per `CONTEXT.md`, off-box backups are an accepted
risk for now: everything is rebuildable — canonical data from the raw archive,
the archive from the upstream sources — at a cost of roughly a day. If that
tradeoff changes, the checkpoint+copy runbook (pause writer →
`wal_checkpoint(TRUNCATE)` → file copy, verified with `integrity_check` + row
counts) is in `docs/research/turso-scale.md`. `VACUUM INTO` is forbidden at
scale (OOM).
