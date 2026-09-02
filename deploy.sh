#!/usr/bin/env bash
# Deploy tender-db to the production VPS (ADR-0006: Ubuntu + nix-built bundle).
#
# Pushes the current main to the VPS bare repo, builds the flake package there
# (the box has the 1 Gb/s uplink and the warm nix store), atomically switches the
# /opt/tender-db/app symlink, restarts the service, and health-checks it.
#
# Usage: ./deploy.sh [git-ref]     (default: HEAD)
set -euo pipefail

VPS="${VPS:-root@zebreus.click}"
# Keepalives: without them a dropped TCP connection leaves ssh hanging on a
# dead socket forever and the deploy looks stuck (2026-07-21 incident).
SSH="ssh -o BatchMode=yes -o ServerAliveInterval=15 -o ServerAliveCountMax=4 -o ConnectTimeout=10"
# HEAD, not `main`: work happens on a branch here, and the local `main` is not
# what keeps it current — nothing updates it, so it silently rots behind
# origin/main. On 2026-09-02 that cost a deploy at the last step, 278 seconds of
# green suite in, with "! [rejected] main -> main (non-fast-forward)" — the local
# `main` was 29 commits behind the commit that had just been pushed and tested.
# Deploying what is checked out is what the operator means every time; the guard
# below catches the other direction (a ref that is genuinely older than main).
REF="${1:-HEAD}"
REMOTE_REPO=/opt/tender-db/repo.git
SRC=/opt/tender-db/src
APP=/opt/tender-db/app
PUBLIC_URL="${PUBLIC_URL:-https://tenders.zebreus.click}"

# Refuse to deploy something the shared main has already moved past. This is the
# stale-ref case the default above fixes, caught for any explicit ref too: a
# commit that is a strict ANCESTOR of origin/main is older than what everyone
# else considers deployed, and pushing it would roll the box backwards. Anything
# else — main itself, a branch ahead of it, an unmerged topic branch — passes.
REV="$(git rev-parse "$REF")"
if git rev-parse --verify --quiet origin/main >/dev/null \
    && [ "$REV" != "$(git rev-parse origin/main)" ] \
    && git merge-base --is-ancestor "$REV" origin/main 2>/dev/null \
    && [ "${FORCE_BEHIND:-0}" != "1" ]; then
    cat >&2 <<MSG

$REF ($(git rev-parse --short "$REV")) is BEHIND origin/main
($(git rev-parse --short origin/main)) by $(git rev-list --count "$REV"..origin/main) commit(s).

Deploying it would roll production back to older code. If you meant the tip:

    git fetch origin main && ./deploy.sh origin/main

or fast-forward your local branch first. To deploy the older commit anyway
(a deliberate rollback): FORCE_BEHIND=1 ./deploy.sh $REF
MSG
    exit 1
fi

say() { printf '\n\033[1m==> %s\033[0m\n' "$*"; }

# A deploy RESTARTS the service, and a restart re-runs the job that was running from
# the top: its durable row survives by design (issue 21), so recovery puts it back at the
# front of the queue. That is cheap for a 90-second fold and expensive for a package
# re-parse — during issue 244's campaign it cost hours of redone work, twice, because
# nothing said the box was busy (issue 245).
#
# So ask first. This runs before the push and the build, so a busy box costs a second
# rather than five minutes, and the override is explicit: FORCE_BUSY=1 ./deploy.sh
# The test gate (issue 254). Three green-looking zeros went past me in one session — a
# pipeline's exit code, a truncated doc-test section, and a feature-gated crate that ran
# no tests at all — so the deploy reads the verdict itself, from cargo's own exit code,
# via ops/check.sh.
#
# Free when `ops/check.sh` already passed at this exact commit with a clean tree: the
# marker it writes is what this skips on. SKIP_TESTS=1 overrides, for the deploy that
# cannot wait — a one-line fix while the box is down is not the moment for a 15-minute
# suite. It runs BEFORE the queue probe so a red tree costs nothing on the VPS.
HEAD_SHA="$(git rev-parse HEAD)"
if [ "${SKIP_TESTS:-0}" = "1" ]; then
    say "SKIP_TESTS=1: deploying WITHOUT running the suites"
elif [ -f target/.tests-green ] && [ "$(cat target/.tests-green)" = "$HEAD_SHA" ] \
    && [ -z "$(git status --porcelain)" ]; then
    say "Suites already green at $(git rev-parse --short HEAD) — skipping (ops/check.sh)"
else
    say "Running the suites before deploying (issue 254; SKIP_TESTS=1 to override)"
    ./ops/check.sh
fi

say "Checking the job queue on $VPS"
BUSY="$($SSH "$VPS" bash -euo pipefail -s <<'PROBE' || true
S=$(sed -n 's/^TENDER_ADMIN_SECRET=//p' /root/tender-admin-secret 2>/dev/null || true)
[ -n "$S" ] || exit 0
curl -s --max-time 10 -H "x-admin-secret: $S" http://127.0.0.1:8080/admin/jobs 2>/dev/null \
  | jq -r 'if .current then "\(.current.id) \(.current.kind) \(.current.params)" else "" end' 2>/dev/null || true
PROBE
)"
if [ -n "$BUSY" ] && [ "${FORCE_BUSY:-0}" != "1" ]; then
    cat >&2 <<MSG
refusing to deploy while a job is running: $BUSY

A restart re-runs it from the top — its durable row survives, so nothing is lost, but the
work is redone. Wait for the queue to drain (ops/admin.sh queue), stop the job
(DELETE /admin/jobs/<id>), or override with FORCE_BUSY=1 if the deploy is the urgent thing.
MSG
    exit 1
fi
[ -n "$BUSY" ] && say "FORCE_BUSY=1: deploying over the running job ($BUSY)"

# The `vps` remote is derived, not assumed. It lives only in the local git
# config, so a fresh clone or a reset working copy simply does not have it, and
# `git push vps` then fails with "'vps' does not appear to be a git repository"
# — AFTER the full test suite has run (2026-09-01: a deploy died there, 281
# seconds in, on a green tree). The address is already known from $VPS and
# $REMOTE_REPO, so there is no reason to require it to have been set up by hand.
want_remote="$VPS:$REMOTE_REPO"
have_remote="$(git remote get-url vps 2>/dev/null || true)"
if [ -z "$have_remote" ]; then
    say "Adding the 'vps' git remote ($want_remote)"
    git remote add vps "$want_remote"
elif [ "$have_remote" != "$want_remote" ]; then
    # Someone pointed it elsewhere, or VPS= was overridden for this run. Say so
    # rather than pushing a deploy at whatever address happens to be configured.
    say "'vps' points at $have_remote, not $want_remote — repointing it"
    git remote set-url vps "$want_remote"
fi

say "Pushing $REF to $VPS:$REMOTE_REPO"
git push vps "$REF:main"

say "Building $REV on the VPS (this can take a while on a cold store)"
$SSH "$VPS" bash -euo pipefail -s <<EOF
export PATH=/nix/var/nix/profiles/default/bin:\$PATH

# One deploy at a time: two concurrent deploys can build different revs and
# the slower (older) one would win the symlink switch — a silent regression.
# The lock covers build → switch → restart on the VPS side.
exec 9>/opt/tender-db/deploy.lock
flock -n 9 || { echo "another deploy holds /opt/tender-db/deploy.lock — aborting" >&2; exit 1; }

# Refuse to move production backwards: if the target rev is an ancestor of the
# currently deployed rev, this deploy would regress. Both ancestry checks run
# against the bare repo — it received the push already, whereas \$SRC has not
# fetched yet at this point, and an unknown rev would make the (negated)
# divergence check refuse a perfectly linear deploy.
if [ -f /opt/tender-db/deployed-rev ]; then
  DEPLOYED=\$(cat /opt/tender-db/deployed-rev)
  if git -C $REMOTE_REPO merge-base --is-ancestor $REV "\$DEPLOYED" 2>/dev/null && [ "$REV" != "\$DEPLOYED" ]; then
    echo "refusing regression: $REV is an ancestor of deployed \$DEPLOYED" >&2
    exit 1
  fi
  # And refuse a DIVERGED line: the target must CONTAIN what is running, or the
  # deploy silently rolls features back with no error anywhere (issue 162 — prod
  # lost the read-path work for four days this way). An unknown deployed rev also
  # lands here, and refusing is the safe reading. Deliberate divergent deploys
  # say so: FORCE_DIVERGENT=1 ./deploy.sh <ref>
  if [ "$REV" != "\$DEPLOYED" ] && ! git -C $REMOTE_REPO merge-base --is-ancestor "\$DEPLOYED" $REV 2>/dev/null && [ "${FORCE_DIVERGENT:-0}" != "1" ]; then
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
