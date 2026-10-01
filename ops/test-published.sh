#!/usr/bin/env bash
# Offline pin for ops/published.sh (issue 463): `rev_published`, the check that keeps
# /_source's link to the running revision on GitHub true. Builds a throwaway bare
# repository standing in for GitHub, a checkout whose `origin` it is, and a local clone of
# that checkout, and drives the real function against them — no network, no box, and never
# the real repository's refs.
#
# deploy.sh runs this before it trusts `rev_published`, the way it runs
# ops/test-gate-marker.sh before it reads the gate marker: a predicate that answered
# `published` to everything would ship a dead source link with no line saying so.
#
# Every `published` case has its `unpublished` twin, and the axis the first version held
# constant is varied on purpose (review of 463): a checkout whose `origin` is NOT the
# public repository, whose branches contain the unpushed rev.
#
# As in test-gate-marker.sh, the harness cannot pass by not running its cases: each check
# runs in a subshell, a fixture step that dies stops the run red, and an EXIT trap turns
# an early exit, or any count other than $planned, into a failure.
#
# Run: bash ops/test-published.sh   (exits 0 on success, 1 on any failure)
set -uo pipefail

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
work=$(mktemp -d) && [ -n "$work" ] && [ -d "$work" ] || { echo "FAIL mktemp -d gave no directory"; exit 1; }

# Update when a case is added or removed.
planned=14
ran=0
failures=0
finished=0
on_exit() {
    local rc=$?
    rm -rf "$work"
    if [ "$finished" != 1 ]; then
        echo "FAIL the harness ended before its last case (exit $rc after $ran of $planned cases) — a fixture step died, or something sourced called exit"
        exit 1
    fi
    exit "$rc"
}
trap on_exit EXIT

# Isolate git from the operator's config, from every channel it has (see
# test-gate-marker.sh: this container passes config through GIT_CONFIG_COUNT).
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1
unset GIT_CONFIG_COUNT GIT_CONFIG_PARAMETERS GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE
export GIT_AUTHOR_NAME=published-test GIT_AUTHOR_EMAIL=published-test@invalid
export GIT_COMMITTER_NAME=published-test GIT_COMMITTER_EMAIL=published-test@invalid
GIT_CEILING_DIRECTORIES=$(dirname "$work")
export GIT_CEILING_DIRECTORIES TMPDIR="$work"

# shellcheck source=ops/published.sh
. "$here/published.sh"
# Every case below asks the stand-in; the real URL is never fetched.
public="$work/public.git"
PUBLIC_REPOSITORY=$public

pass() { ran=$((ran + 1)); echo "ok   $1"; }
fail() { ran=$((ran + 1)); echo "FAIL $1"; failures=$((failures + 1)); }
# says <name> <published|unpublished|unreadable> <rev>: rev_published's word, run in a
# subshell so a function that exits cannot end the harness.
says() {
    local name=$1 want=$2 rev=$3 got
    got=$( rev_published "$rev" 2>/dev/null ) || got="(exit $?) $got"
    if [ "$got" = "$want" ]; then pass "$name"; else fail "$name (wanted $want, got '$got')"; fi
}
fx() { ( "$@" ) >/dev/null 2>&1 || { echo "FAIL fixture step failed: $*"; exit 1; }; }
commit() { echo "$1" >>file; fx git add file; fx git -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -qm "$1"; }

fx git init -q --bare "$public"
repo="$work/repo"
mkdir -p "$repo" && cd "$repo" || exit 1
fx git init -q -b main .
fx git remote add origin "$public"
commit base
base=$(git rev-parse HEAD)
fx git push -q origin HEAD:main
commit pushed
pushed=$(git rev-parse HEAD)
fx git push -q origin HEAD:main

says "the public main's tip is published" published "$pushed"
says "an ancestor of a public branch is published" published "$base"

commit 'not pushed'
local_only=$(git rev-parse HEAD)
says "a commit made here and never pushed is unpublished" unpublished "$local_only"

# The review's case: a checkout whose `origin` is not the public repository. A local clone
# of the shared tree has `origin` = that tree, whose branches contain the unpushed rev.
clone="$work/clone"
fx git clone -q "$repo" "$clone"
cd "$clone" || exit 1
if git for-each-ref --contains "$local_only" refs/remotes/origin | grep -q .; then
    pass "the clone's origin branches contain the unpushed rev (the fixture is the review's case)"
else
    fail "the clone's origin branches contain the unpushed rev (the fixture is the review's case)"
fi
says "…and it is still unpublished there: origin is not the public repository" unpublished "$local_only"
says "…while a pushed rev is published from the clone too" published "$pushed"
cd "$repo" || exit 1

# A tracking ref that says otherwise does not count either: only the fetch does.
fx git update-ref refs/remotes/origin/main "$local_only"
says "a tracking ref of origin pointing at the rev does not publish it" unpublished "$local_only"
fx git update-ref refs/remotes/origin/main "$pushed"

commit topic
topic=$(git rev-parse HEAD)
fx git push -q origin HEAD:refs/heads/topic
says "a rev on a public branch other than main is published" published "$topic"
fx git push -q origin :refs/heads/topic
says "…and unpublished again once that branch is deleted upstream (--prune)" unpublished "$topic"

fx git tag tagged "$topic"
fx git push -q origin refs/tags/tagged
says "a rev the public repository has only as a tag is unpublished (branches only)" unpublished "$topic"

# The fetch above left refs/published/heads/main containing $pushed. A fetch that fails
# must not fall back on it.
PUBLIC_REPOSITORY="$work/no-such-repository.git"
says "a failed fetch is unreadable, even with a previous fetch's refs in place" unreadable "$pushed"
PUBLIC_REPOSITORY=$public

says "an unknown rev is unreadable" unreadable 0000000000000000000000000000000000000000
says "an empty rev is unreadable" unreadable ""

cd "$work" || exit 1
says "outside a repository the answer is unreadable" unreadable "$pushed"

echo
if [ "$ran" -ne "$planned" ]; then
    echo "FAIL ran $ran cases but $planned are planned — a case was skipped, or one was added without updating \$planned"
    failures=$((failures + 1))
fi
finished=1
if [ "$failures" -eq 0 ]; then
    echo "all published tests passed ($ran cases)"
    exit 0
fi
echo "$failures published test(s) failed"
exit 1
