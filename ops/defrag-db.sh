#!/usr/bin/env bash
# Rewrite the live DB file contiguously (issue 488). Run ON THE BOX, as root, with the
# admin queue idle. Nothing here edits the original file: it is copied, the copy is
# compared byte-for-byte, and only then do two renames swap them. The original stays as
# tender-db.db.pre-defrag until the operator deletes it after the service is healthy.
#
#   ops/defrag-db.sh check   # read-only: sizes, free space, extents, queue — no stop
#   ops/defrag-db.sh run     # stop → copy → cmp → swap → start → health
#
# Any failure before the swap leaves the original in place and restarts the service on
# it (the trap). A failure after the swap leaves both files; the trap restarts on the
# copy only if both renames completed, else puts the original back.
set -euo pipefail

MODE=${1:-check}
DB=${TENDER_DB:-/data/db/tender-db.db}
DIR=$(dirname "$DB")
NEW="$DB.defrag-new"
OLD="$DB.pre-defrag"
UNIT=${TENDER_UNIT:-tender-db.service}
HEALTH=${TENDER_HEALTH_URL:-http://127.0.0.1:8080/health}
MARGIN_GIB=${DEFRAG_MARGIN_GIB:-20}

say() { printf '==> %s\n' "$*"; }
die() { printf 'ERROR defrag: %s\n' "$*" >&2; exit 1; }

extents() { xfs_io -r -c stat "$1" 2>/dev/null | awk '/fsxattr.nextents/ {print $3}'; }

queue_idle() {
    local j
    j=$(/root/aj.sh /admin/jobs) || return 1
    jq -e '.current == null and (.queued | length) == 0' <<<"$j" >/dev/null
}

[ "$(id -u)" -eq 0 ] || die "run as root"
[ -f "$DB" ] || die "$DB missing"
[ ! -e "$NEW" ] || die "$NEW exists — a previous run did not finish; inspect it first"
[ ! -e "$OLD" ] || die "$OLD exists — delete it (after checking) before another run"

db_bytes=$(stat -c %s "$DB")
wal_bytes=$(stat -c %s "$DB-wal" 2>/dev/null || echo 0)
need=$(( db_bytes + wal_bytes + MARGIN_GIB * 1024 * 1024 * 1024 ))
free=$(df -B1 --output=avail "$DIR" | tail -1 | tr -d ' ')
say "db $(( db_bytes >> 30 )) GiB, wal $(( wal_bytes >> 20 )) MiB, free $(( free >> 30 )) GiB, need $(( need >> 30 )) GiB (copy + ${MARGIN_GIB} GiB margin)"
say "extents now: $(extents "$DB")"
[ "$free" -ge "$need" ] || die "not enough free space on $DIR — nothing done"
queue_idle || die "admin queue not idle (or unreadable) — nothing done"
[ "$MODE" = run ] || { say "check passed (read-only; nothing done)"; exit 0; }

stopped=0; swapped=0
cleanup() {
    rc=$?
    if [ "$rc" -ne 0 ]; then
        echo "ERROR defrag: failed (rc=$rc) — recovering" >&2
        if [ "$swapped" -eq 0 ]; then
            # Original never moved, or the first rename happened without the second.
            if [ ! -e "$DB" ] && [ -e "$OLD" ]; then mv "$OLD" "$DB"; [ -e "$OLD-wal" ] && mv "$OLD-wal" "$DB-wal"; fi
            rm -f "$NEW" "$NEW-wal"
        fi
        if [ "$stopped" -eq 1 ]; then systemctl start "$UNIT" || true; fi
    fi
}
trap cleanup EXIT

queue_idle || die "queue became busy — nothing done"
say "stopping $UNIT"
systemctl stop "$UNIT"; stopped=1
systemctl is-active --quiet "$UNIT" && die "$UNIT still active"
sync

# Re-read the sizes now the writer is gone (the WAL may have been checkpointed on stop).
db_bytes=$(stat -c %s "$DB")
say "copying $(( db_bytes >> 30 )) GiB (no reflink)"
t0=$(date +%s)
cp --reflink=never --sparse=never --preserve=mode,ownership,timestamps "$DB" "$NEW"
if [ -e "$DB-wal" ]; then cp --reflink=never --preserve=mode,ownership,timestamps "$DB-wal" "$NEW-wal"; fi
sync
say "copied in $(( $(date +%s) - t0 )) s; extents of the copy: $(extents "$NEW")"

say "comparing byte for byte"
cmp "$DB" "$NEW" || die "copy differs from the original"
if [ -e "$DB-wal" ]; then cmp "$DB-wal" "$NEW-wal" || die "WAL copy differs"; fi
say "identical"

# The swap: two renames per file on one filesystem.
mv "$DB" "$OLD"
[ -e "$DB-wal" ] && mv "$DB-wal" "$OLD-wal"
mv "$NEW" "$DB"
[ -e "$NEW-wal" ] && mv "$NEW-wal" "$DB-wal"
swapped=1
say "swapped; original kept as $OLD"

systemctl start "$UNIT"; stopped=0
for _ in $(seq 1 60); do
    if curl -fsS --max-time 10 "$HEALTH" | jq -e '.ok == true' >/dev/null 2>&1; then
        say "healthy on the defragmented file; extents now: $(extents "$DB")"
        say "after checking the API, delete $OLD (and $OLD-wal) to free its space"
        trap - EXIT
        exit 0
    fi
    sleep 10
done
die "service did not come healthy within 10 min — both files kept; to roll back: stop, mv $OLD $DB (and -wal), start"
