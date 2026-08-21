#!/usr/bin/env bash
# Install (or restore) the tender-db watchdog timers on the production box.
#
# Idempotent: copies the two watch scripts to /usr/local/bin and the four unit
# files to /etc/systemd/system, reloads systemd, and enables + starts the timers.
# Safe to re-run at any time — this is the recovery path after a box rebuild or an
# accidental deletion of the scripts (which is exactly what happened 2026-08-09,
# issue 224: the scripts existed only on the box and were lost, so the timers
# failed hourly for a week).
#
# Run as root, from this directory:  sudo ./install.sh
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
bin=/usr/local/bin
unitdir=/etc/systemd/system

if [ "$(id -u)" -ne 0 ]; then
    echo "install.sh must run as root (writes $bin and $unitdir)" >&2
    exit 1
fi

for s in tender-db-diskwatch.sh tender-db-jobwatch.sh tender-db-driftwatch.sh; do
    install -m 0755 -o root -g root "$here/$s" "$bin/$s"
    echo "installed $bin/$s"
done

for u in \
    tender-db-diskwatch.service tender-db-diskwatch.timer \
    tender-db-jobwatch.service  tender-db-jobwatch.timer \
    tender-db-driftwatch.service tender-db-driftwatch.timer; do
    install -m 0644 -o root -g root "$here/$u" "$unitdir/$u"
    echo "installed $unitdir/$u"
done

systemctl daemon-reload
systemctl reset-failed tender-db-diskwatch.service tender-db-jobwatch.service \
    tender-db-driftwatch.service 2>/dev/null || true
systemctl enable --now tender-db-diskwatch.timer tender-db-jobwatch.timer \
    tender-db-driftwatch.timer
echo "timers enabled:"
systemctl list-timers 'tender-db-*watch.timer' --no-pager || true

# Prove the scripts run clean right now (does not wait for the next tick).
echo "--- diskwatch dry fire ---"; systemctl start tender-db-diskwatch.service && journalctl -u tender-db-diskwatch.service -n 5 --no-pager
echo "--- jobwatch dry fire ---";  systemctl start tender-db-jobwatch.service  && journalctl -u tender-db-jobwatch.service  -n 5 --no-pager
