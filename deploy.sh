#!/usr/bin/env bash
# Deploy tender-db to the production VPS (ADR-0006: Ubuntu + nix-built bundle).
#
# Pushes the current main to the VPS bare repo, builds the flake package there
# (the box has the 1 Gb/s uplink and the warm nix store), atomically switches the
# /opt/tender-db/app symlink, restarts the service, and health-checks it.
#
# Usage: ./deploy.sh [git-ref]     (default: HEAD)
set -euo pipefail
# Everything below names repo paths relatively (ops/check.sh, the gate marker, the
# queue probe it pipes to the box), so stand at the repo root wherever it was run from.
cd "$(dirname "$0")"

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

REV="$(git rev-parse "$REF")"

# Refuse a commit the public repository does not have (issue 463). /_source and /v1's
# source_offer link its tree on GitHub — that link IS the AGPL §13 offer — and this script
# pushes only to the box, which has run commits GitHub did not have (2026-09-04). The
# decision is rev_published (ops/published.sh): it fetches the repository the app links,
# by URL, never `origin`, which in a local clone of the shared tree is that tree and
# contains every unpushed commit (review of 463). Its pin runs first, as the gate's does
# below: a rev_published that said `published` to everything would ship a dead link.
if [ "${FORCE_UNPUBLISHED:-0}" != "1" ]; then
    if ! selftest=$(bash ops/test-published.sh 2>&1); then
        printf '%s\n' "$selftest" | grep -v '^ok ' >&2 || true
        echo "refusing to deploy: ops/test-published.sh FAILED (above), so rev_published cannot be trusted (FORCE_UNPUBLISHED=1 overrides)" >&2
        exit 1
    fi
    # shellcheck source=ops/published.sh
    . ops/published.sh
    published=$(rev_published "$REV") || published=
    case "$published" in
        published) ;;
        unpublished)
            # `origin` when it is the public repository, so its tracking ref moves too.
            to=$PUBLIC_REPOSITORY
            [ "$(git remote get-url origin 2>/dev/null)" = "$PUBLIC_REPOSITORY" ] && to=origin
            push="git push $to $REV:main"
            [ "$REV" = "$(git rev-parse HEAD)" ] && push="git push $to HEAD:main"
            cat >&2 <<MSG

$REF ($(git rev-parse --short "$REV")) is on no branch of $PUBLIC_REPOSITORY.

/_source and /v1 link the running revision's tree there (the AGPL §13 source offer,
issue 463), so deploying a rev it does not have serves a dead link. Push it, then
deploy again:

    $push

(or to a branch of its own, if main is not ready for it). To deploy it anyway, with
/_source linking nothing until the push lands: FORCE_UNPUBLISHED=1 ./deploy.sh $REF
MSG
            exit 1
            ;;
        *)
            echo "refusing to deploy: could not fetch $PUBLIC_REPOSITORY, so nothing says it has $(git rev-parse --short "$REV") (FORCE_UNPUBLISHED=1 overrides)" >&2
            exit 1
            ;;
    esac
fi

# Refuse to deploy something the shared main has already moved past. This is the
# stale-ref case the default above fixes, caught for any explicit ref too: a
# commit that is a strict ANCESTOR of origin/main is older than what everyone
# else considers deployed, and pushing it would roll the box backwards. Anything
# else — main itself, a branch ahead of it, an unmerged topic branch — passes.
if git rev-parse --verify --quiet origin/main >/dev/null \
    && [ "$REV" != "$(git rev-parse origin/main)" ] \
    && git merge-base --is-ancestor "$REV" origin/main 2>/dev/null \
    && [ "${FORCE_BEHIND:-0}" != "1" ]; then
    cat >&2 <<MSG

$REF ($(git rev-parse --short "$REV")) is BEHIND origin/main
($(git rev-parse --short origin/main)) by $(git rev-list --count "$REV"..origin/main) commit(s).

Deploying it would roll production back to older code. If you meant the tip, bring
the checkout to it and deploy that:

    git fetch origin main && git merge --ff-only origin/main && ./deploy.sh

(\`./deploy.sh origin/main\` from a checkout whose tree differs from origin/main is
refused by the test gate unless target/.tests-green already covers origin/main — the
gate can only test the checked-out tree, issue 459.) To deploy the older commit anyway
(a deliberate rollback): FORCE_BEHIND=1 ./deploy.sh $REF
MSG
    exit 1
fi

say() { printf '\n\033[1m==> %s\033[0m\n' "$*"; }

# The test gate (issue 254). Three green-looking zeros went past me in one session — a
# pipeline's exit code, a truncated doc-test section, and a feature-gated crate that ran
# no tests at all — so the deploy reads the verdict itself, from cargo's own exit code,
# via ops/check.sh. It runs BEFORE the queue probe so a red tree costs nothing on the VPS.
#
# The gate is bound to $REV, the commit pushed and built below — not to HEAD or the
# working tree (issue 459). It used to compare the marker with HEAD, then ship $REV:
# `./deploy.sh origin/main` on a green HEAD skipped the suites and shipped an untested
# commit, and a dirty tree was tested, then not shipped. Now there are exactly two ways
# past this step:
#   * target/.tests-green names a commit whose tree equals $REV's outside .scratch/
#     (marker_covers, ops/gate-marker.sh) — free, and a .scratch/-only commit after a
#     green gate no longer needs SKIP_TESTS=1 or a by-eye `git diff --stat`;
#   * SKIP_TESTS=1, for the deploy that cannot wait — a one-line fix while the box is
#     down is not the moment for a 15-minute suite.
# Otherwise, if the checked-out tree IS $REV's (outside .scratch/, and clean), ops/check.sh
# runs and must leave a marker covering $REV behind, or the deploy refuses: that catches a
# commit landing during the run, and an edit made and reverted during it. A HEAD that
# differs from $REV, or a dirty tree, is refused up front, because the gate could only
# test the wrong tree. Which of these applies is gate_deploy_step's one word.
if [ "${SKIP_TESTS:-0}" = "1" ]; then
    say "SKIP_TESTS=1: deploying WITHOUT running the suites"
else
    # The predicates are pinned before they are trusted, the way install.sh runs
    # test-watchdogs.sh before installing: a marker_covers that said yes to
    # everything would turn every deploy into an untested one with a green-looking line.
    # Its output is kept and shown on a failure — a refusal that hides its reason sends
    # the operator to SKIP_TESTS=1.
    if ! selftest=$(bash ops/test-gate-marker.sh 2>&1); then
        printf '%s\n' "$selftest" | grep -v '^ok ' >&2 || true
        echo "refusing to deploy: ops/test-gate-marker.sh FAILED (above), so target/.tests-green cannot be trusted" >&2
        echo "  (SKIP_TESTS=1 bypasses the whole gate)" >&2
        exit 1
    fi
    # shellcheck source=ops/gate-marker.sh
    . ops/gate-marker.sh
    # The whole decision is gate_deploy_step, pinned by ops/test-gate-marker.sh with
    # REV ≠ HEAD both ways; this only maps its word to an action. Anything else —
    # including no word at all — refuses.
    step=$(gate_deploy_step "$REV") || step=
    case "$step" in
        skip)
            say "Suites already green at $(git rev-parse --short "$(gate_marker_commit)"), whose tree equals $(git rev-parse --short "$REV")'s outside .scratch/ — skipping (ops/check.sh)"
            ;;
        run)
            say "Running the suites before deploying (issue 254; SKIP_TESTS=1 to override)"
            ./ops/check.sh
            if ! marker_covers "$REV"; then
                cat >&2 <<MSG

refusing to deploy: the gate ran but wrote no marker covering $(git rev-parse --short "$REV").
Its closing line says why — a tree dirty outside .scratch/, a commit outside .scratch/
landing during the run, or a file outside .scratch/ written during it. Either way the
green describes a tree this deploy does not ship.
MSG
                exit 1
            fi
            ;;
        refuse-head)
            # The gate tests the CHECKED-OUT tree. Twelve minutes of suite over HEAD would
            # certify nothing about $REV, so refuse before spending them.
            cat >&2 <<MSG

refusing to deploy: no marker covers $REF ($(git rev-parse --short "$REV")), and the
checked-out HEAD ($(git rev-parse --short HEAD 2>/dev/null || echo '?')) differs from it outside .scratch/ — so
ops/check.sh here would test a tree this deploy does not ship.

Check out $REF and run ./deploy.sh again, or deploy from a fresh checkout of
$(git rev-parse --short "$REV") (docs/operations.md, Deploy). SKIP_TESTS=1 skips the gate
altogether.
MSG
            exit 1
            ;;
        *)
            # refuse-dirty, or a word nobody foresaw. check.sh writes no marker over a tree
            # dirty outside .scratch/ (the green would describe uncommitted edits, not $REV),
            # so the run could only end in the refusal after it. Say so now, and what is
            # dirty, rather than after twelve minutes.
            report=$(gate_tree_report 2>/dev/null) || report="(git could not read the working tree)"
            cat >&2 <<MSG

refusing to deploy: no marker covers $(git rev-parse --short "$REV"), and the working tree
is not clean outside .scratch/ (gate step: ${step:-none}), so ops/check.sh would test
something this deploy does not ship and write no marker:
MSG
            sed -n '1,20s/^/    /p' <<<"${report:-(nothing listed)}" >&2
            echo "$(gate_dirt_hint "$report")" >&2
            exit 1
            ;;
    esac
fi

# A deploy RESTARTS the service, and a restart re-runs the job that was running from
# the top: its durable row survives by design (issue 21), so recovery puts it back at the
# front of the queue. That is cheap for a 90-second fold and expensive for a package
# re-parse — during issue 244's campaign it cost hours of redone work, twice, because
# nothing said the box was busy (issue 245).
#
# So ask first. This runs before the push and the build, so a busy box costs a second
# rather than five minutes, and the override is explicit: FORCE_BUSY=1 ./deploy.sh
#
# The probe is ops/watchdogs/tender-db-queue-probe.sh, piped from THIS checkout (issue
# 459), so the deploy never depends on what is installed on the box. It answers one
# named line, and queue_verdict (ops/watchdogs/tender-db-queue-verdict.sh, the same
# function the snapshot uses, table-tested by test-watchdogs.sh) judges that line
# together with ssh's exit status: only `idle` or `down` (the app's unit has no process,
# so nothing to re-run) proceeds. The probe it replaces printed '' for idle AND for
# every failure to measure — ssh down, no secret, a timeout, a 403 or 404 body — and
# this read '' as idle. Now an ssh failure, an empty answer, an `error` and any output
# nobody foresaw all come back as `error <what>` and land in the last arm, which
# refuses. FORCE_BUSY=1 overrides both refusals.
say "Checking the job queue on $VPS"
# shellcheck source=ops/watchdogs/tender-db-queue-verdict.sh
. ops/watchdogs/tender-db-queue-verdict.sh
probe_rc=0
QUEUE="$($SSH "$VPS" bash -s < ops/watchdogs/tender-db-queue-probe.sh)" || probe_rc=$?
VERDICT=$(queue_verdict "$probe_rc" "$QUEUE") || VERDICT=
case "$VERDICT" in
    idle)
        say "Queue idle on $VPS"
        ;;
    down)
        say "tender-db.service on $VPS has no process and accepts no connection (probe: down) — no job to re-run"
        ;;
    "busy "*)
        BUSY="${VERDICT#busy }"
        if [ "${FORCE_BUSY:-0}" != "1" ]; then
            cat >&2 <<MSG
refusing to deploy while a job is running: $BUSY

A restart re-runs it from the top — its durable row survives, so nothing is lost, but the
work is redone. Wait for the queue to drain (ops/admin.sh queue), stop the job
(DELETE /admin/jobs/<id>), or override with FORCE_BUSY=1 if the deploy is the urgent thing.
MSG
            exit 1
        fi
        say "FORCE_BUSY=1: deploying over the running job ($BUSY)"
        ;;
    *)
        WHAT="${VERDICT#error }"
        WHAT="${WHAT:-no verdict (ssh exit $probe_rc)}"
        if [ "${FORCE_BUSY:-0}" != "1" ]; then
            cat >&2 <<MSG
refusing to deploy: could not measure the queue: $WHAT

A deploy restarts the service, and a restart re-runs whatever job is running from the
top (issue 245), so it goes ahead only on a queue it has READ as idle. "HTTP 403" means
/root/tender-admin-secret no longer matches the secret the service started with (it reads
the file once, at start); "HTTP 404" means the service has no TENDER_ADMIN_SECRET at all;
"curl exit 7 … not stopped" means nothing answered where the probe asked while the
service has a process (a changed port?). Look with ssh $VPS tender-admin queue, or
override with FORCE_BUSY=1.
MSG
            exit 1
        fi
        say "FORCE_BUSY=1: deploying although the queue could not be measured ($WHAT)"
        ;;
esac

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

say "Pushing $REF ($(git rev-parse --short "$REV")) to $VPS:$REMOTE_REPO"
# `push.negotiate` off: the box's bare repo over the ssh tunnel speaks protocol
# v1, so the v2 negotiation git tries first fails ("--negotiate-only requires
# protocol v2 … the remote end hung up") and git proceeds anyway — a wasted
# round trip and three alarming lines in every deploy log for nothing.
# The PINNED sha, not "$REF": a ref like HEAD is re-read here, twenty minutes
# after REV was taken, so a commit made while the gate ran would reach the
# box's main while the build below still checks out REV (found reading it, 2026-09-29).
git -c push.negotiate=false push vps "$REV:refs/heads/main"

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
# Cap the service's memory (issue 426), so a runaway query or fold is stopped inside
# the service instead of starving sshd and the kernel on the 62 GiB box. The
# measured peak is 22.5 GB anon (a TED daily fold, job 1714); full rebuilds peak
# at ~20.8 GB RSS. MemoryHigh makes the kernel reclaim the service's own page cache
# first; MemoryMax is the hard stop. The box runs this hand-installed unit, not
# nix/module.nix, so the limit lives in a drop-in that every deploy rewrites.
printf '[Service]\nMemoryHigh=54G\nMemoryMax=58G\n' > /etc/systemd/system/tender-db.service.d/memory.conf
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
