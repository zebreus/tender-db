# 14 — Production deployment on the VPS

Status: resolved
Blocked by: 05, 06

Goal: tenders.zebreus.click serves the app from the VPS (ADR-0006).

Scope:
- Install nix on the Ubuntu VPS; `nix build .#tender-db` there (repo synced
  via git); install bundle into /opt/tender-db/app (nix profile or direct
  store path + symlink for atomic switch).
- systemd unit `tender-db.service`: hardened (mirror nix/module.nix flags),
  DynamicUser=false → dedicated `tenderdb` user owning /data/db +
  /data/archive; env: IP=127.0.0.1, PORT=8080, TENDER_DB=/data/db/tender-db.db,
  TENDER_ARCHIVE=/data/archive.
- nginx + certbot: TLS vhost tenders.zebreus.click → 127.0.0.1:8080,
  proxy_buffering off for SSE, HTTP/2.
- A `deploy.sh` in the repo (build, install, restart, health-check) run via
  ssh; importer scheduling via the app's internal scheduler (verify).
- Runbook: docs/operations.md (deploy, logs, restart, disk watch).

Acceptance: https://tenders.zebreus.click serves the dashboard and
/v1/tenders publicly; systemd survives reboot; deploy.sh redeploys cleanly.

## Comments

2026-07-19 — Infrastructure groundwork done; issue stays `ready-for-agent`
because full acceptance still depends on 05 and 06 shipping real endpoints.

Live now:
- Determinate Nix (daemon, flakes on) installed on the VPS.
- `nix build /opt/tender-db/src#tender-db` builds the bundle on the box.
  Cold build ≈ 25 min on 4 cores; warm rebuilds ≈ 6 min (deps derivation
  cached, only the `dx bundle` step re-runs).
- `tenderdb` system user (uid 999) owns /data/db + /data/archive.
- `/etc/systemd/system/tender-db.service`: mirrors nix/module.nix hardening
  with `User=tenderdb` instead of DynamicUser, plus
  `ReadWritePaths=/data/db /data/archive` (the module relies on
  StateDirectory). `systemd-analyze security` → 3.0 OK. Enabled, so it comes
  back on reboot.
- nginx + certbot vhost tenders.zebreus.click → 127.0.0.1:8080, HTTP/2,
  `proxy_buffering off` + 24 h read timeout for SSE, HTTP→HTTPS redirect.
  Cert expires 2026-10-17, certbot timer handles renewal.
- `deploy.sh` (repo root) and `docs/operations.md` added; deploy.sh tested
  end-to-end, deployed rev 7808a32.

Verified: `https://tenders.zebreus.click/` → 200, HTTP/2, serves the Dioxus
scaffold page.

Still open for full acceptance:
- The dashboard and `/v1/tenders` are not implemented yet (05/06) — the app
  currently serves only the scaffold with a `GET /api/tenders` server fn.
- deploy.sh's health check asserts `/` returns HTML; swap it for the real
  `/health` endpoint when 05 lands.
- Importer scheduling via the app's internal scheduler is unverified — the
  scheduler doesn't exist yet.
- Reboot survival is enabled but not exercised with an actual reboot (the
  box was busy with other agents' fetch/scan jobs).

Gotcha for other agents: tmux session names are global on the VPS. A session
named `build` collided with another agent's and killed this build once — use
a task-specific name (`tenderdb-deploy` here). Also, anything writing to
/data as root leaves files the service can't touch; re-run
`chown -R tenderdb:tenderdb /data/db /data/archive` afterwards.

2026-07-20 — Redeployed and verified live end-to-end; acceptance met. Setting
`resolved`.

- Deployed rev **7199300** (issues 06/07/08/17) via `./deploy.sh`: pushed main,
  built the flake on the box, atomic symlink switch, service restart, `/health`
  check green. Production DB survived the restart (cursor unchanged across it).
- **Dashboard live**: `https://tenders.zebreus.click/` → 200 HTTP/2, serving the
  real dashboard (Coverage / Ingestion / Quarantine panels), not the scaffold.
  `/_source` (AGPL §13) → 200.
- **`/v1/tenders` live**: serves real data (7163 Tenders after a verification
  ingest). `/v1` advertises the full endpoint set incl. `/v1/sql`,
  `/v1/sql/schema`, `/v1/webhooks`.
- **Accounts** (dashboard server fns): register → 200 + HttpOnly/Secure session
  cookie; token mint → 200; `/v1/me` with the bearer token → 200.
- **SQL endpoint** over live data: `top buyers by tender count` returned real
  buyers (Południowy Koncern Węglowy, ŘSD, ČEZ, …). Gate holds live: write →
  400, `SELECT * FROM users` → 400 "not queryable", no token → 401.
- **Webhooks**: registered a public https receiver (SSRF guard passed it), drove
  fresh changes via `/admin` (TED daily 2026-00137: +3717 notices, +3702
  versions, cursor 24553 → 47224), and the sweeper delivered — **50 signed
  batches, every one independently HMAC-SHA256-verified** against the secret.
  (Subsequent `consecutive_failures` were the free-tier receiver throttling,
  which incidentally exercised the backoff path.) Test webhook + account cleaned
  up afterward.
- **SSE through nginx**: `Accept: text/event-stream` on `/v1/tenders` streams the
  snapshot (`event: change` / `op:added`, full tender data) then a `live` marker
  (`id: 47224`); an empty-filter subscription returns the `live` marker
  immediately — confirming nginx is not buffering. `content-type:
  text/event-stream`, `cache-control: no-store`.
- **In-process importer** verified via `/admin` (secret from the systemd
  EnvironmentFile): enqueue → run → job log, readers serving throughout.
- `deploy.sh` redeploys cleanly and health-checks `/health` (already swapped
  from the old HTML check).

Rough edges (not blockers, noted for follow-up):
- **DÖE ingestion is broken on the box**: `fetch doe daily/monthly` downloads,
  but `process` fails with `package: io: invalid gzip header` (the DÖE endpoint
  is not returning the expected gzip). TED works fine. Belongs to the DÖE-source
  owner (issue 12) — flagging, not fixing here.
- `/root/tender-admin-secret` is a systemd **EnvironmentFile**
  (`TENDER_ADMIN_SECRET=<64-hex>`), not a bare secret — pass the value after the
  `=`, not the whole line, to `X-Admin-Secret`.
- `rev` still reports `dev` because `deploy.sh` doesn't set `COMMIT_SHA`;
  cosmetic. `deployed-rev` on the box is authoritative.
- Reboot survival still enabled-but-not-exercised (didn't reboot prod).
