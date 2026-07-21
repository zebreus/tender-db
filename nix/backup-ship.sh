#!/usr/bin/env bash
# Ship the newest local DB snapshot off-box and prune the remote retention ring
# (issue 23). Runs as a systemd timer on the VPS, NOT in the app — the app only
# takes the consistent snapshot (Supervisor `snapshot` job → /data/snapshots);
# getting it off the box is a separate, failure-isolated step.
#
# BLOCKED until an off-box destination is provisioned (docs/operations.md
# "Shipping off-box"): set DEST to a Hetzner Storage Box / Object Storage target
# and drop the SSH key on the box. Recommended: a Storage Box (rsync-native,
# off the prod host). This script is a template — wire DEST and enable the timer.
set -euo pipefail

# --- configuration (override via the systemd unit's Environment=) ------------
SNAP_DIR="${TENDER_SNAPSHOT_DIR:-/data/snapshots}"
# rsync destination, e.g. u123456@u123456.your-storagebox.de:snapshots/
# (Storage Box) — leave empty and the script refuses to run.
DEST="${BACKUP_DEST:-}"
# SSH identity + options for the Storage Box (port 23 on Hetzner Storage Boxes).
SSH_OPTS="${BACKUP_SSH_OPTS:--p 23 -i /root/.ssh/storagebox -o StrictHostKeyChecking=accept-new}"
# Retention ring, kept on the remote: N most-recent dailies + N weeklies.
KEEP_DAILY="${BACKUP_KEEP_DAILY:-7}"
KEEP_WEEKLY="${BACKUP_KEEP_WEEKLY:-4}"

if [[ -z "$DEST" ]]; then
  echo "backup-ship: BACKUP_DEST is unset — no off-box destination provisioned yet." >&2
  echo "backup-ship: see docs/operations.md 'Shipping off-box'. Refusing to run." >&2
  exit 1
fi

# --- pick the newest snapshot ------------------------------------------------
newest="$(ls -1t "$SNAP_DIR"/tender-db-*.db 2>/dev/null | head -n1 || true)"
if [[ -z "$newest" ]]; then
  echo "backup-ship: no snapshots in $SNAP_DIR — nothing to ship." >&2
  exit 0
fi

# --- ship it (resumable, checksummed) ----------------------------------------
# --partial + a .inprogress suffix so an interrupted transfer never leaves a
# short file that looks complete on the remote.
echo "backup-ship: shipping $(basename "$newest") → $DEST"
rsync -e "ssh $SSH_OPTS" --partial --inplace --checksum \
  "$newest" "${DEST%/}/$(basename "$newest").inprogress"
ssh $SSH_OPTS "${DEST%%:*}" \
  "mv '${DEST#*:}/$(basename "$newest").inprogress' '${DEST#*:}/$(basename "$newest")'"

# --- prune the remote ring ---------------------------------------------------
# Keep the KEEP_DAILY newest, plus one per ISO week for KEEP_WEEKLY weeks. The
# names are tender-db-<unix>.db, so lexical sort == chronological sort.
remote_ls() { ssh $SSH_OPTS "${DEST%%:*}" "ls -1 '${DEST#*:}'/tender-db-*.db 2>/dev/null || true"; }
mapfile -t all < <(remote_ls | sort)

declare -A keep=()
# newest KEEP_DAILY
for f in $(printf '%s\n' "${all[@]}" | tail -n "$KEEP_DAILY"); do keep["$f"]=1; done
# one per ISO week, newest KEEP_WEEKLY weeks
declare -A week_seen=()
for f in $(printf '%s\n' "${all[@]}" | tac); do
  unix="$(basename "$f" .db)"; unix="${unix#tender-db-}"
  wk="$(date -u -d "@$unix" +%G-%V 2>/dev/null || echo "")"
  [[ -z "$wk" ]] && continue
  if [[ -z "${week_seen[$wk]:-}" && "${#week_seen[@]}" -lt "$KEEP_WEEKLY" ]]; then
    week_seen[$wk]=1; keep["$f"]=1
  fi
done

for f in "${all[@]}"; do
  if [[ -z "${keep[$f]:-}" ]]; then
    echo "backup-ship: pruning remote $(basename "$f")"
    ssh $SSH_OPTS "${DEST%%:*}" "rm -f '$f'"
  fi
done
echo "backup-ship: done."
