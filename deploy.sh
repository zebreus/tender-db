#!/usr/bin/env bash
# Deploy tender-db to the production VPS (ADR-0006: Ubuntu + nix-built bundle).
#
# Pushes the current main to the VPS bare repo, builds the flake package there
# (the box has the 1 Gb/s uplink and the warm nix store), atomically switches the
# /opt/tender-db/app symlink, restarts the service, and health-checks it.
#
# Usage: ./deploy.sh [git-ref]     (default: main)
set -euo pipefail

VPS="${VPS:-root@zebreus.click}"
# Keepalives: without them a dropped TCP connection leaves ssh hanging on a
# dead socket forever and the deploy looks stuck (2026-07-21 incident).
SSH="ssh -o BatchMode=yes -o ServerAliveInterval=15 -o ServerAliveCountMax=4 -o ConnectTimeout=10"
REF="${1:-main}"
REMOTE_REPO=/opt/tender-db/repo.git
SRC=/opt/tender-db/src
APP=/opt/tender-db/app
PUBLIC_URL="${PUBLIC_URL:-https://tenders.zebreus.click}"

say() { printf '\n\033[1m==> %s\033[0m\n' "$*"; }

say "Pushing $REF to $VPS:$REMOTE_REPO"
git push vps "$REF:main"
REV="$(git rev-parse "$REF")"

say "Building $REV on the VPS (this can take a while on a cold store)"
$SSH "$VPS" bash -euo pipefail -s <<EOF
export PATH=/nix/var/nix/profiles/default/bin:\$PATH

# One deploy at a time: two concurrent deploys can build different revs and
# the slower (older) one would win the symlink switch — a silent regression.
# The lock covers build → switch → restart on the VPS side.
exec 9>/opt/tender-db/deploy.lock
flock -n 9 || { echo "another deploy holds /opt/tender-db/deploy.lock — aborting" >&2; exit 1; }

# Refuse to move production backwards: if the target rev is an ancestor of the
# currently deployed rev, this deploy would regress.
if [ -f /opt/tender-db/deployed-rev ]; then
  DEPLOYED=\$(cat /opt/tender-db/deployed-rev)
  if git -C $SRC merge-base --is-ancestor $REV "\$DEPLOYED" 2>/dev/null && [ "$REV" != "\$DEPLOYED" ]; then
    echo "refusing regression: $REV is an ancestor of deployed \$DEPLOYED" >&2
    exit 1
  fi
  # And refuse a DIVERGED line: the target must CONTAIN what is running, or the
  # deploy silently rolls features back with no error anywhere (issue 162 — prod
  # lost the read-path work for four days this way). An unknown deployed rev also
  # lands here, and refusing is the safe reading. Deliberate divergent deploys
  # say so: FORCE_DIVERGENT=1 ./deploy.sh <ref>
  if [ "$REV" != "\$DEPLOYED" ] && ! git -C $SRC merge-base --is-ancestor "\$DEPLOYED" $REV 2>/dev/null && [ "${FORCE_DIVERGENT:-0}" != "1" ]; then
    echo "refusing divergent deploy: $REV does not contain deployed \$DEPLOYED (FORCE_DIVERGENT=1 overrides)" >&2
    exit 1
  fi
fi

cd $SRC
git fetch origin main
git checkout -f main
git reset --hard $REV
echo "source at: \$(git rev-parse HEAD)"

# Build to a generation-specific result link so the running app keeps its store
# path alive until we switch (and so a failed build never touches the symlink).
nix build "$SRC#tender-db" -o /opt/tender-db/app-result --print-build-logs
STORE_PATH="\$(readlink -f /opt/tender-db/app-result)"
echo "built: \$STORE_PATH"

# Atomic switch: ln -T to a temp name, then rename over the old symlink.
ln -sfnT "\$STORE_PATH" ${APP}.new
mv -T ${APP}.new $APP
echo "$APP -> \$(readlink $APP)"

# Record the deployed revision (regression guard + the deploy summary below).
echo "$REV" > /opt/tender-db/deployed-rev

# Feed the rev to the RUNNING service via a systemd drop-in, read at runtime as
# COMMIT_SHA (the app's v1::rev()). The rev lives in the environment, not the
# built artifact, so the nix build stays reproducible while /health, /v1 and the
# dashboard's System panel still report the exact deployed revision.
install -d /etc/systemd/system/tender-db.service.d
printf '[Service]\nEnvironment=COMMIT_SHA=%s\n' "$REV" > /etc/systemd/system/tender-db.service.d/rev.conf
systemctl daemon-reload

systemctl restart tender-db
systemctl --no-pager --lines=0 status tender-db | head -5
EOF

say "Health check"
# /health (issue 05) reports the process is up and the database answers. It is
# outside the rate limiter and stays responsive under load — ingestion runs
# in-process (issue 16) with readers serving over WAL, so this must return 200
# throughout a load, not just at idle.
# Patience: schema work at open (e.g. a first-boot index build over millions
# of rows) can hold /health past a minute; that is startup, not failure.
for i in $(seq 1 120); do
  code="$(curl -s -o /dev/null -w '%{http_code}' --max-time 10 "$PUBLIC_URL/health" || true)"
  [ "$code" = "200" ] && break
  sleep 1
done

if [ "${code:-}" != "200" ]; then
  echo "health check FAILED: $PUBLIC_URL/health returned ${code:-no response}" >&2
  $SSH "$VPS" 'journalctl -u tender-db -n 40 --no-pager' >&2 || true
  exit 1
fi

body="$(curl -s --max-time 10 "$PUBLIC_URL/health")"
case "$body" in
  *'"ok":true'*) ;;
  *) echo "health check FAILED: $PUBLIC_URL/health returned 200 but not ok: $body" >&2; exit 1 ;;
esac

echo "OK  $PUBLIC_URL/health -> 200, database ok"
echo "OK  deployed rev: $($SSH "$VPS" 'cat /opt/tender-db/deployed-rev')"
echo
echo "Ingestion is in-process via the /admin API (operator secret in"
echo "/root/tender-admin-secret on the VPS). See docs/operations.md → Ingestion."
