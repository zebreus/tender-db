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
# * Refuses (exit 0, loud) while an admin job runs: a fold mid-write would
#   reflink a mid-transaction page soup worth nothing.
# * Prunes to the newest KEEP, and can NEVER delete the last snapshot.
# * Same-volume only — this is a verification/forensics artifact, not
#   disaster recovery; the raw archive remains the rebuild-from-zero path.
set -euo pipefail

DB="${TENDER_SNAP_DB:-/data/db/tender-db.db}"
DIR="${TENDER_SNAP_DIR:-/data/db/snapshots}"
KEEP="${TENDER_SNAP_KEEP:-2}"
SECRET_FILE="${TENDER_ADMIN_SECRET_FILE:-/root/tender-admin-secret}"
URL="${TENDER_ADMIN_URL:-http://127.0.0.1:8080}"

# Quiescence gate: skip while a job runs. An unreachable app means no writer
# is alive, which is the SAFEST time to snapshot — proceed.
if secret=$(sed -n 's/^TENDER_ADMIN_SECRET=//p' "$SECRET_FILE" 2>/dev/null) && [ -n "$secret" ]; then
    current=$(curl -s --max-time 10 -H "x-admin-secret: $secret" "$URL/admin/jobs" 2>/dev/null \
        | python3 -c "import json,sys
try: print(json.load(sys.stdin)['current']['id'])
except Exception: print('')" || true)
    if [ -n "$current" ]; then
        echo "skipped snapshot: job $current is running — next timer firing retries"
        exit 0
    fi
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
