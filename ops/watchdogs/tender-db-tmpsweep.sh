#!/usr/bin/env bash
# tender-db temp-directory sweep (issue 337) — remove the turso temp databases
# orphaned by the last service restart. Runs daily via tender-db-tmpsweep.timer.
#
# WHAT ACCUMULATES, and why a sweep is the right answer rather than a code fix:
# `BEGIN IMMEDIATE` makes turso 0.7.2 lazily create a per-connection temp
# database — a `tempfile::tempdir()` under $TMPDIR holding `tursodb-temp.db`
# (translate/transaction.rs emits a Transaction opcode for TEMP_DB_ID; op_transaction
# calls ensure_temp_database). The `TempDir` is owned by the connection, so a
# graceful drop removes it. Ours never drop: the unit takes a default SIGTERM on
# every restart and the process does not unwind, so whatever the writer
# connection held is orphaned. ~1/day in steady state, unbounded, never reused.
# The whole mechanism is pinned by crates/store/tests/turso_temp_db_leak.rs,
# including why `PRAGMA temp_store = MEMORY` is NOT the fix (same switch routes
# sorter and hash spills, which must stay on disk at 490 GiB — issue 83).
#
# THE DELETION RULE is exact, not a heuristic: a directory older than the
# service's current ActiveEnterTimestamp cannot be held by the running process,
# because a live connection's directory is removed the moment that connection
# closes. Directories newer than the start time are left strictly alone, so a
# long-running index build's spill is never touched. If the start time cannot be
# read (service never started, systemd unavailable) the sweep does nothing —
# a missing bound is a reason to abstain, not to fall back to an age guess.
#
# Versioned in ops/watchdogs/ and installed to /usr/local/bin by install.sh
# (issue 224: the originals once lived only on the box and were lost).
#
#   TENDER_TMPSWEEP_DRY=1   report what would go, delete nothing
#   TENDER_TMPDIR=<dir>     sweep somewhere else (default /data/tmp)
set -euo pipefail

tmpdir=${TENDER_TMPDIR:-/data/tmp}
dry=${TENDER_TMPSWEEP_DRY:-}

[ -d "$tmpdir" ] || { echo "ok tmpsweep: $tmpdir does not exist, nothing to sweep"; exit 0; }

stamp=$(systemctl show tender-db -p ActiveEnterTimestamp --value 2>/dev/null || true)
if [ -z "$stamp" ]; then
    echo "ok tmpsweep: tender-db has no ActiveEnterTimestamp (never started?) — abstaining"
    exit 0
fi
if ! started=$(date -d "$stamp" +%s 2>/dev/null) || [ -z "$started" ]; then
    echo "WARN tmpsweep: cannot parse ActiveEnterTimestamp '$stamp' — abstaining"
    exit 1
fi

swept=0 freed=0 foreign=0
while IFS= read -r -d '' dir; do
    # Only turso's own temp artefacts: `tursodb-temp.db*` (the per-connection
    # temp database, what this issue is about) or `tursodb_temp_file*` (a sorter
    # or hash spill, orphaned by the same non-unwinding exit). A `.tmp*`
    # directory holding anything else belongs to someone else — report it and
    # leave it, because guessing is how a sweep turns into an incident.
    unexpected=$(find "$dir" -mindepth 1 -not -name 'tursodb-temp.db*' -not -name 'tursodb_temp_file*' -print -quit)
    if [ -n "$unexpected" ]; then
        echo "WARN tmpsweep: $dir holds unexpected content ($unexpected) — left in place"
        foreign=$((foreign + 1))
        continue
    fi
    bytes=$(du -sb "$dir" | cut -f1)
    if [ -n "$dry" ]; then
        echo "dry tmpsweep: would remove $dir (${bytes} B)"
    else
        rm -rf -- "$dir"
    fi
    swept=$((swept + 1))
    freed=$((freed + bytes))
done < <(find "$tmpdir" -maxdepth 1 -mindepth 1 -type d -name '.tmp*' -not -newermt "@$started" -print0)

kept=$(find "$tmpdir" -maxdepth 1 -mindepth 1 -type d -name '.tmp*' -newermt "@$started" -print | wc -l)

echo "ok tmpsweep: ${swept} orphaned dir(s)${dry:+ (dry run, none actually)} removed, $(( freed / 1024 )) KiB freed; \
${kept} newer than the service start ($stamp) left held; ${foreign} with unexpected content left alone"
exit 0
