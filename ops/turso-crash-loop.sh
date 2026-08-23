#!/usr/bin/env bash
# The kill-9 crash loop (issue 271) — the D1 durability gate, in-repo.
#
#   ops/turso-crash-loop.sh [iterations] [db-path]
#
# Each iteration starts `crash_probe write` against ONE persistent database,
# lets it commit for a random 0.3-1.5 s, kill -9s it mid-flight, and reopens
# with `crash_probe verify`, which holds turso to the WAL contract: every
# acked commit present, no torn parent/satellite batch, no FK orphans. The
# database deliberately survives across iterations — restart-over-existing is
# the recovery path prod actually takes, so WAL recovery is exercised every
# round, not just round one.
#
# Runs anywhere cargo does (dev container, CI, the box). ~2 min for the
# default 32 rounds. The lost on-box bench's other leg — throughput at scale —
# is deliberately NOT here; see docs/research/turso-scale.md §D1.
set -euo pipefail

iters=${1:-32}
db=${2:-/tmp/tender-db-crash-probe.db}

# Debug profile on purpose: the probe measures durability, not speed, and the
# release tree would double the target dir on a disk that has been tight.
cargo build -p store --example crash_probe
probe=$(dirname "$(cargo locate-project --workspace --message-format plain)")/target/debug/examples/crash_probe

rm -f "$db" "$db-wal" "$db-shm"
last=0
for i in $(seq 1 "$iters"); do
    log=$(mktemp)
    "$probe" write "$db" > "$log" 2>&1 &
    pid=$!
    # Wait for the FIRST ack before arming the kill: on a fresh file the schema
    # install alone outlasts any fixed pre-kill sleep, and killing a writer that
    # never committed proves nothing (the zero-ack check below would fire).
    for _ in $(seq 1 300); do
        grep -q '^committed ' "$log" && break
        kill -0 "$pid" 2>/dev/null || break
        sleep 0.1
    done
    # …then 0.3-1.5 s of committing before the kill, varied so the WAL is
    # caught at different phases (mid-batch, mid-checkpoint, just-after-commit).
    sleep "0.$((RANDOM % 12 + 3))"
    kill -9 "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
    acked=$(grep -c '^committed ' "$log" || true)
    # A round in which the writer never committed proves NOTHING — a writer
    # that crashes at startup would sail through every verify against an empty
    # database. Zero progress is a failure of the probe itself, not a pass.
    if [ "$acked" -eq 0 ]; then
        echo "FAILED at iteration $i: the writer acked no commits before the kill — probe broken?"
        sed -n '1,8p' "$log" >&2 || true
        exit 1
    fi
    last_line=$(tail -n 1 "$log" | grep -oE '[0-9]+$' || echo "$last")
    [ -n "$last_line" ] && last=$last_line
    rm -f "$log"
    "$probe" verify "$db" "$last" || { echo "FAILED at iteration $i (acked $acked this round)"; exit 1; }
    echo "round $i/$iters ok (through commit $last, +$acked this round)"
done
echo "crash loop clean: $iters kill -9 rounds, no torn batch, no lost ack"
