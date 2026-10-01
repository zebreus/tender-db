#!/usr/bin/env bash
# Install (or restore) the tender-db watchdog timers on the production box.
#
# Idempotent: copies the watch scripts and the operator CLI (ops/admin.sh as
# tender-admin) to /usr/local/bin, the admin helper ops/aj.sh to /root/aj.sh, and
# the unit files to /etc/systemd/system, reloads systemd, and enables + starts the
# timers.
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

# Gate the install on the offline tests (issue 373). They need no box and no
# secret, so there is no reason to skip them — and the bug they exist for was a
# watchdog that ran green against the real box while structurally unable to see
# the failures it was watching for. Catching that BEFORE the script reaches
# /usr/local/bin is the whole point. Set TENDER_SKIP_WATCHDOG_TESTS=1 only to
# recover a box when the harness itself is what is broken.
if [ "${TENDER_SKIP_WATCHDOG_TESTS:-0}" != "1" ]; then
    echo "--- watchdog offline tests ---"
    if ! bash "$here/test-watchdogs.sh"; then
        echo "install.sh: watchdog tests FAILED — refusing to install" >&2
        echo "  (override with TENDER_SKIP_WATCHDOG_TESTS=1 if the harness is the broken part)" >&2
        exit 1
    fi
fi

# tender-db-queue-probe.sh has no unit: the snapshot runs it from beside itself in
# $bin (issue 459), so it must land there too or every Sunday's gate reads `error`.
for s in tender-db-diskwatch.sh tender-db-jobwatch.sh tender-db-driftwatch.sh \
    tender-db-snapshot.sh tender-db-tmpsweep.sh tender-db-queue-probe.sh; do
    install -m 0755 -o root -g root "$here/$s" "$bin/$s"
    echo "installed $bin/$s"
done
# Sourced, not run: the snapshot loads queue_verdict from beside itself, and without it
# refuses every Sunday with an ERROR (issue 459).
install -m 0644 -o root -g root "$here/tender-db-queue-verdict.sh" "$bin/tender-db-queue-verdict.sh"
echo "installed $bin/tender-db-queue-verdict.sh"

# The operator CLI rides along: ops/admin.sh's own header says it is installed
# to /usr/local/bin under these conventions, but nothing did it — on 2026-09-05
# the box still ran the 2026-08-17 copy, without `queue` and `cancel`.
install -m 0755 -o root -g root "$here/../admin.sh" "$bin/tender-admin"
echo "installed $bin/tender-admin"
# And the handover's one-line helper (issue 464): `aj.sh <path>` GETs, `aj.sh <path>
# '<json>'|@file` POSTs. It lived only in /root, and the issue files and their Verify
# lines call it by that path, so it goes back there — root-only, like the secret it reads.
install -m 0700 -o root -g root "$here/../aj.sh" /root/aj.sh
echo "installed /root/aj.sh"

for u in \
    tender-db-diskwatch.service tender-db-diskwatch.timer \
    tender-db-jobwatch.service  tender-db-jobwatch.timer \
    tender-db-driftwatch.service tender-db-driftwatch.timer \
    tender-db-snapshot.service tender-db-snapshot.timer \
    tender-db-tmpsweep.service tender-db-tmpsweep.timer; do
    install -m 0644 -o root -g root "$here/$u" "$unitdir/$u"
    echo "installed $unitdir/$u"
done

systemctl daemon-reload
systemctl reset-failed tender-db-diskwatch.service tender-db-jobwatch.service \
    tender-db-driftwatch.service 2>/dev/null || true
systemctl enable --now tender-db-diskwatch.timer tender-db-jobwatch.timer \
    tender-db-driftwatch.timer tender-db-snapshot.timer tender-db-tmpsweep.timer
echo "timers enabled:"
systemctl list-timers 'tender-db-*.timer' --no-pager || true

# Prove the scripts run clean right now (does not wait for the next tick).
echo "--- diskwatch dry fire ---"; systemctl start tender-db-diskwatch.service && journalctl -u tender-db-diskwatch.service -n 5 --no-pager
echo "--- jobwatch dry fire ---";  systemctl start tender-db-jobwatch.service  && journalctl -u tender-db-jobwatch.service  -n 5 --no-pager
# Read-only; anything but idle / busy / down here means the snapshot's gate cannot read
# the queue either (issue 459).
echo "--- queue probe ---"; "$bin/tender-db-queue-probe.sh" || true
# The sweep deletes, so its dry fire is a DRY fire — it reports and removes nothing.
echo "--- tmpsweep dry fire (reports only) ---"; TENDER_TMPSWEEP_DRY=1 /usr/local/bin/tender-db-tmpsweep.sh
