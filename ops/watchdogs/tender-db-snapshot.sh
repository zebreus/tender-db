#!/usr/bin/env bash
# tender-db weekly DB snapshot (issue 269) — restore the offline-read path,
# point-in-time artifact, and forensics capability that lapsed when the last
# snapshot was pruned. NOT a watchdog: this one writes (a reflink copy).
#
# Mechanics, and their honesty:
# * /data is XFS with reflink=1, so `cp --reflink=always` is instant and
#   shares blocks — a snapshot costs ~nothing until the live DB diverges.
# * The copy pair (db, then -wal) is CRASH-CONSISTENT, not transactional: the
#   serving app may write between the two reflinks. SQLite recovery treats the
#   snapshot like a power cut — a WAL whose salts mismatch is discarded — so
#   the worst case is losing sub-second bookkeeping writes IN THE SNAPSHOT.
#   The job-quiescence check below keeps real data writes out of that window.
# * After copying, the snapshot's OWN WAL is checkpointed (TRUNCATE) via
#   python3's sqlite3 and left as the 0-byte sibling de1x_verify.sh expects.
# * Waits for the admin queue to drain (up to 150 min, issue 420) and only then
#   refuses (exit 0, loud): a fold mid-write would reflink a mid-transaction
#   page soup worth nothing, and skipping outright lost the 2026-09-13 snapshot.
# * Snapshots ONLY on a named `idle` or `down` from tender-db-queue-probe.sh, as
#   judged by tender-db-queue-verdict.sh (issue 459). A queue it could not read —
#   wrong or missing secret, a 403/404, a timeout, an answer with the wrong exit
#   status — is polled like a busy one, and if the budget runs out on that it
#   exits 1 with an ERROR line: NOT snapshotted, and visible in systemctl --failed.
# * Prunes to the newest KEEP, and can NEVER delete the last snapshot.
# * Same-volume only — this is a verification/forensics artifact, not
#   disaster recovery; the raw archive remains the rebuild-from-zero path.
set -euo pipefail

DB="${TENDER_SNAP_DB:-/data/db/tender-db.db}"
DIR="${TENDER_SNAP_DIR:-/data/db/snapshots}"
KEEP="${TENDER_SNAP_KEEP:-2}"
SECRET_FILE="${TENDER_ADMIN_SECRET_FILE:-/root/tender-admin-secret}"
URL="${TENDER_ADMIN_URL:-http://127.0.0.1:8080}"

# Quiescence gate: WAIT for the queue to drain, then snapshot (issue 420). The
# first version SKIPPED while a job ran; the weekly data-quality run starts
# 03:10 Berlin and takes 90–190 min, so on 2026-09-13 it was still running at
# 05:23 and the snapshot skipped — the ring then held two fully diverged copies
# for a fortnight. Now: poll once a minute for up to TENDER_SNAP_WAIT_MIN minutes
# (default 150 — 05:23 + 150 min = 07:53 Berlin, still clear of the 09:35 daily
# tick), and only then skip, loudly. An app that accepts no connection means no
# admin API is up, which issue 420 judged the SAFEST time to snapshot — proceed,
# but only on the probe's named `down`, never on a failed read.
#
# The queue is read by tender-db-queue-probe.sh, the same bytes deploy.sh pipes to
# the box (issue 459). The version before it printed '' for idle AND for any curl
# failure or non-200 (a 403 has no ['current']), and skipped the whole gate when
# the secret file was missing — so a rotated secret or a removed admin.conf made
# every Sunday's gate read "idle" while a fold was writing. A failed read now polls
# like a busy queue (a timeout under load can clear) and, if it never clears,
# exits 1: a missing snapshot that says so beats a mid-fold one that does not.
WAIT_MIN="${TENDER_SNAP_WAIT_MIN:-150}"   # polls at the default 60 s poll = minutes
POLL_SECS="${TENDER_SNAP_POLL_SECS:-60}"
DRY="${TENDER_SNAP_DRY:-0}"               # 1 = stop after the gate (offline tests)
# The installed probe and its verdict sit beside the installed snapshot in
# /usr/local/bin; run from the repo, the repo copies sit beside this one. A missing
# probe is an `error`, not idle; a missing verdict is an ERROR before anything else.
HERE="$(dirname "${BASH_SOURCE[0]}")"
PROBE="$HERE/tender-db-queue-probe.sh"
# shellcheck source=ops/watchdogs/tender-db-queue-verdict.sh
if ! . "$HERE/tender-db-queue-verdict.sh" 2>/dev/null || ! declare -F queue_verdict >/dev/null; then
    echo "ERROR snapshot: cannot load $HERE/tender-db-queue-verdict.sh, so the queue cannot be judged — NOT snapshotted"
    exit 1
fi

# One answer per call: idle | down | busy <id> <kind> <params> | error <what>, judged by
# queue_verdict — the same function deploy.sh uses, so the probe's exit status is half
# the answer here too: `idle` with exit 1, two lines, or nothing at all is an `error`.
read_queue() {
    local out rc=0
    out=$(TENDER_ADMIN_URL="$URL" TENDER_ADMIN_SECRET_FILE="$SECRET_FILE" bash "$PROBE" 2>/dev/null) || rc=$?
    queue_verdict "$rc" "$out"
}

waited=0
answer=$(read_queue)
while :; do
    case "$answer" in
        idle) break ;;
        down)
            echo "the app accepts no connection at $URL (probe: down) — no writer, snapshotting"
            break
            ;;
        "busy "*)
            read -r _ job_id job_kind job_params <<<"$answer"
            what="job $job_id is running ($job_kind${job_params:+ $job_params})"
            ;;
        *)
            what="could not read the queue (${answer#error })"
            ;;
    esac
    if [ "$waited" -ge "$WAIT_MIN" ]; then
        case "$answer" in
            "busy "*)
                echo "skipped snapshot: job $job_id still running ($job_kind) after $WAIT_MIN poll(s) of ${POLL_SECS}s — next timer firing retries"
                exit 0
                ;;
            *)
                echo "ERROR snapshot: could not read the queue (${answer#error }) after $WAIT_MIN poll(s) of ${POLL_SECS}s — NOT snapshotted"
                exit 1
                ;;
        esac
    fi
    if [ "$waited" -eq 0 ]; then
        echo "waiting: $what — polling every ${POLL_SECS}s for up to $WAIT_MIN poll(s)"
    fi
    sleep "$POLL_SECS"
    waited=$((waited + 1))
    answer=$(read_queue)
done
if [ "$waited" -gt 0 ]; then
    echo "queue $answer after $waited poll(s) — snapshotting"
fi
if [ "$DRY" = "1" ]; then
    echo "dry run: would snapshot $DB into $DIR (keep $KEEP)"
    exit 0
fi

mkdir -p "$DIR"
stamp=$(date +%s)
snap="$DIR/tender-db-$stamp.db"
cp --reflink=always "$DB" "$snap"
if [ -f "$DB-wal" ]; then cp --reflink=always "$DB-wal" "$snap-wal"; fi

# Fold the snapshot's WAL into it and truncate — the copy is now a clean
# standalone DB with the 0-byte -wal sibling the verify suite expects.
python3 - "$snap" <<'PYEOF'
import sqlite3, sys
conn = sqlite3.connect(sys.argv[1])
conn.execute("PRAGMA wal_checkpoint(TRUNCATE)")
conn.close()
PYEOF

size=$(du -h "$snap" | cut -f1)
echo "snapshot written: $snap ($size apparent; reflink-shared)"

# Prune to KEEP newest — and never the last one, whatever KEEP says.
mapfile -t snaps < <(ls -1t "$DIR"/tender-db-*.db 2>/dev/null)
if [ "${#snaps[@]}" -gt "$KEEP" ] && [ "$KEEP" -ge 1 ]; then
    for old in "${snaps[@]:$KEEP}"; do
        rm -f "$old" "$old-wal" "$old-shm"
        echo "pruned $old"
    done
fi
echo "snapshots kept: ${#snaps[@]} -> $(ls -1 "$DIR"/tender-db-*.db | wc -l)"
