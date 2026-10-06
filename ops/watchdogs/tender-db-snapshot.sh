#!/usr/bin/env bash
# tender-db weekly DB snapshot (issue 269) — restore the offline-read path,
# point-in-time artifact, and forensics capability that lapsed when the last
# snapshot was pruned. NOT a watchdog: this one writes (a reflink copy).
#
# Mechanics, and their honesty:
# * /data is XFS with reflink=1, so `cp --reflink=always` is instant and
#   shares blocks — a snapshot costs ~nothing on the day it is taken. It is NOT
#   free afterwards (issue 488): every page the app rewrites in the live DB is
#   copied OUT of sharing, so the snapshot slowly becomes a full second copy.
#   - Disk: un-sharing allocates new blocks for each rewritten page, up to the
#     whole live file per snapshot alive (a corpus-wide refold un-shared ~40 GB/h
#     of /data). Hence the disk guard below, and KEEP=1.
#   - Fragmentation: each un-share is allocated in units of the file's COW extent
#     hint (`cowextsize`). At 4 KiB (issue 169's setting) that cut the live DB into
#     90M extents and the fold spent 76 % of its CPU walking the extent tree. The
#     DB was defragmented on 2026-10-06 (1.95M extents). A LARGER hint brings back
#     issue 169's leak (unused COW preallocation never reclaimed on the always-open
#     file: 220 GiB at 128 KiB), so the default stays 4096 until issue 488 decides
#     between dropping reflink snapshots and a larger hint plus a periodic
#     `xfs_spaceman prealloc -s` reclaim. Re-applied before every snapshot, because a
#     replaced inode (a restore, a defrag swap) silently drops it.
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
# * Prunes to the newest KEEP (default 1, was 2 until issue 488), AFTER the new
#   copy is written, and can NEVER delete the last snapshot nor the one just
#   written. Prune-after-copy is what keeps one snapshot present at every moment
#   with KEEP=1; the price is that the old snapshot's exclusive blocks are still
#   held while the new one is taken (the disk guard counts free space with them).
#
# The three knobs (issue 488), all environment variables:
#   TENDER_SNAP_KEEP=1          snapshots kept after a run (0 = never prune)
#   TENDER_SNAP_COWEXT=4096     cowextsize re-applied to the live DB before each
#                               snapshot (`xfs_io -c "cowextsize …"`); a failure is
#                               a WARN line, not fatal — a non-xfs box cannot set it
#   TENDER_SNAP_FREE_FACTOR=1.2 refuse (exit 0, `SKIP snapshot: …`) unless the DB's
#                               filesystem has at least live-DB-size × this free:
#                               one snapshot can cost up to the whole file in
#                               un-sharing before the next run prunes it. Free space
#                               that cannot be read (`df -B1 --output=avail`) is an
#                               ERROR exit 1 — fail closed, never a snapshot.
# * Same-volume only — this is a verification/forensics artifact, not
#   disaster recovery; the raw archive remains the rebuild-from-zero path.
set -euo pipefail

DB="${TENDER_SNAP_DB:-/data/db/tender-db.db}"
DIR="${TENDER_SNAP_DIR:-/data/db/snapshots}"
KEEP="${TENDER_SNAP_KEEP:-1}"
COWEXT="${TENDER_SNAP_COWEXT:-4096}"
FREE_FACTOR="${TENDER_SNAP_FREE_FACTOR:-1.2}"
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

# (Re)apply the COW extent hint before the copy makes the file shared again (issue
# 488): every page un-shared after this snapshot is allocated in runs of COWEXT, not
# page by page. Not fatal — a box (or test) without xfs cannot set it, and a snapshot
# beats none — but loud. Note the trade-off issue 169 met: a large hint leaves
# speculative COW preallocation behind on the always-open file; `xfs_spaceman -c
# "prealloc -s -m 100g" /data` reclaims it online (docs/operations.md).
if xfs_out=$(xfs_io -c "cowextsize $COWEXT" "$DB" 2>&1); then
    echo "cowextsize $COWEXT applied to $DB"
else
    echo "WARN snapshot: could not set cowextsize $COWEXT on $DB (${xfs_out:-xfs_io failed}) — snapshotting anyway; un-sharing may fragment the live DB (issue 488)"
fi

# Disk guard (issue 488). The reflink is instant, but from then on every page the
# app rewrites is copied out of sharing: one snapshot can cost up to the WHOLE live
# file in new blocks before the next run prunes it (a corpus-wide refold un-shared
# ~40 GB/h). So demand that much free, times a margin, before taking it. A skip is
# exit 0 (the old snapshot stays — nothing is pruned), an unreadable answer exit 1.
if ! [[ "$FREE_FACTOR" =~ ^[0-9]+([.][0-9]+)?$ ]]; then
    echo "ERROR snapshot: TENDER_SNAP_FREE_FACTOR='$FREE_FACTOR' is not a number — NOT snapshotted"
    exit 1
fi
if ! db_bytes=$(stat -c %s "$DB" 2>/dev/null) || ! [[ "$db_bytes" =~ ^[0-9]+$ ]]; then
    echo "ERROR snapshot: cannot read the size of $DB — NOT snapshotted"
    exit 1
fi
df_out=$(df -B1 --output=avail "$DB" 2>/dev/null) || df_out=""
# Exactly a header line and one all-digit line, or nothing is believed.
free_bytes=$(printf '%s\n' "$df_out" | awk 'NR == 2 { gsub(/^[ \t]+|[ \t]+$/, ""); v = $0 } END { if (NR == 2 && v ~ /^[0-9]+$/) print v }')
if [ -z "$free_bytes" ]; then
    echo "ERROR snapshot: cannot read free space on the filesystem of $DB (df -B1 --output=avail answered '$(printf '%s' "$df_out" | tr '\n' '|')') — NOT snapshotted"
    exit 1
fi
need_bytes=$(awk -v s="$db_bytes" -v f="$FREE_FACTOR" 'BEGIN { printf "%.0f", s * f }')
if [ "$free_bytes" -lt "$need_bytes" ]; then
    echo "SKIP snapshot: only $free_bytes bytes free on the filesystem of $DB, need $need_bytes (live DB $db_bytes bytes × $FREE_FACTOR: a snapshot can cost up to the whole file in un-sharing) — NOT snapshotted, nothing pruned"
    exit 0
fi
echo "disk guard: $free_bytes bytes free >= $need_bytes needed (live DB $db_bytes × $FREE_FACTOR)"

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

# Prune to KEEP newest — AFTER the copy, so with KEEP=1 a snapshot exists at every
# moment — and never the last one, nor the one just written, whatever KEEP says.
mapfile -t snaps < <(ls -1t "$DIR"/tender-db-*.db 2>/dev/null)
if [[ "$KEEP" =~ ^[0-9]+$ ]] && [ "$KEEP" -ge 1 ] && [ "${#snaps[@]}" -gt "$KEEP" ]; then
    for old in "${snaps[@]:$KEEP}"; do
        [ "$old" = "$snap" ] && continue
        rm -f "$old" "$old-wal" "$old-shm"
        echo "pruned $old"
    done
fi
echo "snapshots kept: ${#snaps[@]} -> $(ls -1 "$DIR"/tender-db-*.db | wc -l)"
