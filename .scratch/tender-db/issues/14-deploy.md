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
