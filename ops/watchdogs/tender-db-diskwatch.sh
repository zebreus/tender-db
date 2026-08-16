#!/usr/bin/env bash
# tender-db free-space watch — warn (to the journal) when / or /data crosses a
# fill threshold. Detection only; runs hourly via tender-db-diskwatch.timer.
#
# Deliberately app-independent (plain `df`, no call into the server), so it still
# fires a warning if the tender-db process is down — the case a disk-full is most
# likely to have caused. A clean run exits 0 and logs one OK line per mount; a
# breach exits non-zero so the run also shows up in `systemctl --failed`.
#
# This script is versioned in the repo (ops/watchdogs/) and installed to
# /usr/local/bin by ops/watchdogs/install.sh, which is the path the unit
# references. The originals lived ONLY on the box and vanished 2026-08-09,
# leaving the timer failing hourly for a week before anyone noticed (issue 224);
# keeping them in git, restorable by re-running install.sh, is the durable fix.
set -euo pipefail

threshold=${TENDER_DISK_WARN_PCT:-90}   # warn at or above this used-percent
status=0

for mount in / /data; do
    # `df -Ph`: POSIX single data row; field 5 is Capacity ("16%"), field 4 the
    # human-readable Available ("101G"). Strip the % to compare as an integer.
    read -r used avail <<<"$(df -Ph "$mount" | awk 'NR==2 {gsub(/%/,"",$5); print $5, $4}')"
    if [ "${used:-100}" -ge "$threshold" ]; then
        echo "WARN disk: $mount at ${used}% used, ${avail} free (threshold ${threshold}%)"
        status=1
    else
        echo "ok disk: $mount at ${used}% used, ${avail} free"
    fi
done

exit "$status"
