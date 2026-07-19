# 14 — Production deployment on the VPS

Status: ready-for-agent
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
