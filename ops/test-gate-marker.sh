#!/usr/bin/env bash
# Offline pin for ops/gate-marker.sh (issue 459): the predicates that bind the test gate
# to the commit a deploy ships. Builds a throwaway git repository in a tempdir and
# drives the real functions against it — no cargo, no box, and never the real marker.
#
# deploy.sh runs this BEFORE it reads target/.tests-green, the way install.sh runs
# test-watchdogs.sh before installing (issue 373): a predicate that silently answered
# "covered" would ship untested code with a green-looking skip line, and nothing
# downstream would catch it (nix/package.nix sets doCheck = false).
#
# Every "covered"/"marker written"/"skip" case has its "not covered"/"no marker"/
# "refuse" twin, so a predicate hard-wired either way fails here (instrument-discipline.md,
# "Self-test every predicate both ways"). And the axes a fixture tends to hold constant
# are varied on purpose (its "both-ways is per-axis"): REV ≠ HEAD, git config that hides
# files from `git status`, a `git replace`, an edit that is reverted before the end.
#
# The harness cannot pass by not running its cases (review of 459): every predicate
# runs in a subshell, so one that calls `exit 0` answers "success" for that case rather
# than ending the harness green; a fixture step that dies stops the run red; and an EXIT
# trap turns any exit before the last line, and any count other than $planned, into a
# failure. deploy.sh reads only this script's exit status.
#
# Run: bash ops/test-gate-marker.sh   (exits 0 on success, 1 on any failure)
set -uo pipefail

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
# An unchecked `work=$(mktemp -d)` that failed would make $work/repo the path /repo.
work=$(mktemp -d) && [ -n "$work" ] && [ -d "$work" ] || { echo "FAIL mktemp -d gave no directory"; exit 1; }

# Update when a case is added or removed. A mismatch is a FAIL, so a harness that
# silently skipped cases cannot read as green.
planned=62
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

# Isolate git from the operator's config, from every channel it has: a commit.gpgsign
# or hooksPath would make the fixture's commits fail (or run hooks), and settings like
# status.showUntrackedFiles or an excludes file are exactly what the predicates must not
# depend on — so the cases below set those deliberately, one at a time. GIT_CONFIG_COUNT
# and GIT_CONFIG_PARAMETERS carry config through the environment past GIT_CONFIG_GLOBAL
# (this container sets three keys that way).
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1
unset GIT_CONFIG_COUNT GIT_CONFIG_PARAMETERS GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_NO_REPLACE_OBJECTS
export GIT_AUTHOR_NAME=gate-test GIT_AUTHOR_EMAIL=gate-test@invalid
export GIT_COMMITTER_NAME=gate-test GIT_COMMITTER_EMAIL=gate-test@invalid
# The "outside a repository" cases must not find one above the tempdir, and the start
# stamps gate_begin makes land inside it and go with it.
GIT_CEILING_DIRECTORIES=$(dirname "$work")
export GIT_CEILING_DIRECTORIES TMPDIR="$work"

# shellcheck source=ops/gate-marker.sh
. "$here/gate-marker.sh"

pass() { ran=$((ran + 1)); echo "ok   $1"; }
fail() { ran=$((ran + 1)); echo "FAIL $1"; failures=$((failures + 1)); }
# expect <name> <0|1> <cmd…>: the command's success (0) or failure (non-zero) is the
# claim. In a subshell, so a predicate that exits cannot end the harness.
expect() {
    local name=$1 want=$2; shift 2
    local got=0
    ( "$@" ) >/dev/null 2>&1 || got=1
    if [ "$got" = "$want" ]; then pass "$name"; else fail "$name (wanted $( [ "$want" = 0 ] && echo success || echo failure ), got the other)"; fi
}
# step_is <name> <skip|run|refuse-head|refuse-dirty> <rev>: deploy.sh's gate decision.
step_is() {
    local name=$1 want=$2 rev=$3 got
    got=$(gate_deploy_step "$rev" 2>/dev/null) || got="(exit $?) $got"
    if [ "$got" = "$want" ]; then pass "$name"; else fail "$name (wanted $want, got '$got')"; fi
}
# verdict_says <name> <text> <start> <clean> <stamp>: no marker, and the reason names it.
verdict_says() {
    local name=$1 text=$2 out rc=0; shift 2
    out=$(gate_end_verdict "$@" 2>&1) || rc=$?
    if [ "$rc" -ne 0 ] && grep -qF -- "$text" <<<"$out"; then
        pass "$name (says: $out)"
    else
        fail "$name — wanted no marker and '$text', got rc $rc: $out"
    fi
}
# begin: gate_begin in a subshell (it may not exit the harness either), its globals
# carried back as start / start_clean / stamp.
begin() {
    local v
    v=$( gate_begin >/dev/null 2>&1 && printf '%s|%s|%s' "$GATE_START_HEAD" "$GATE_START_CLEAN" "$GATE_STAMP" ) || v=
    IFS='|' read -r start start_clean stamp <<<"$v"
}
# A fixture step that fails would leave every later case asserting about the wrong
# history, so it stops the run instead of being counted as a case.
fx() { ( "$@" ) >/dev/null 2>&1 || { echo "FAIL fixture step failed: $*"; exit 1; }; }
commit() { fx git add -A; fx git -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -qm "$1"; }

repo="$work/repo"
mkdir -p "$repo/crates/store/src" "$repo/.scratch/issues"
cd "$repo" || exit 1
fx git init -q -b main .
printf '/target\n' >.gitignore
echo 'pub fn a() {}' >crates/store/src/lib.rs
echo '# 1' >.scratch/issues/001.md
commit base
base=$(git rev-parse HEAD)

marker=$(gate_marker_path) || { echo "FAIL gate_marker_path answered nothing inside a repository"; exit 1; }
case "$marker" in
    "$repo"/target/.tests-green) pass "the marker lives in the repository under test, not the real one" ;;
    *) fail "the marker path is $marker, outside the throwaway repository"; exit 1 ;;
esac

# --- marker_covers: the deploy's half ---------------------------------------------------
expect "no marker file covers nothing" 1 marker_covers "$base"

fx gate_write_marker "$base"
expect "a marker at REV covers it" 0 marker_covers "$base"
if [ "$(cat "$marker")" = "gate-v2 $base" ]; then pass "the marker is written as 'gate-v2 <sha>'"; else fail "the marker is written as 'gate-v2 <sha>' — got: $(cat "$marker")"; fi
expect "writing the marker leaves the tree clean (target/ is ignored)" 0 gate_tree_clean
expect "the marker writer refuses anything but a full SHA" 1 gate_write_marker HEAD

echo '# 1, triaged' >.scratch/issues/001.md
echo '# 2' >.scratch/issues/002.md
commit 'scratch only'
scratch=$(git rev-parse HEAD)
expect "a marker plus a .scratch/-only commit covers it" 0 marker_covers "$scratch"

echo 'pub fn b() {}' >>crates/store/src/lib.rs
commit code
code=$(git rev-parse HEAD)
expect "a marker plus a crates/ change does not cover it" 1 marker_covers "$code"

# REV ≠ HEAD, both ways: `./deploy.sh origin/main` from a checkout elsewhere. A
# marker_covers that read HEAD instead of its argument passes every case above, because
# there HEAD always IS the rev.
expect "a covered REV is covered while HEAD is an uncovered code commit" 0 marker_covers "$scratch"
fx git checkout -q "$scratch"
expect "a marker covering HEAD does not cover a different REV with a code change" 1 marker_covers "$code"
fx git checkout -q main

# `git replace <code> <base>` makes git read base's tree under code's name.
fx git replace "$code" "$base"
expect "a git replace of the code commit by the marker's does not make it covered" 1 marker_covers "$code"
fx git replace -d "$code"

# A code change that is later reverted has the marker's tree again: covered, because the
# box builds trees, not histories.
fx git -c commit.gpgsign=false -c core.hooksPath=/dev/null revert --no-edit HEAD
reverted=$(git rev-parse HEAD)
expect "a commit whose tree equals the marker's outside .scratch/ is covered" 0 marker_covers "$reverted"

# A change outside both crates/ and .scratch/ is a change: the exclusion is .scratch/
# alone, not "everything but crates/".
mkdir -p .claude && echo '{}' >.claude/settings.json
commit settings
settings=$(git rev-parse HEAD)
expect "a change outside .scratch/ and crates/ is not covered either" 1 marker_covers "$settings"

expect "an unknown rev is not covered" 1 marker_covers "0000000000000000000000000000000000000000"
expect "an empty rev is not covered" 1 marker_covers ""

# The marker must NAME a commit, in the current format. `HEAD` or a branch name would
# cover whatever is checked out; an abbreviated SHA can become ambiguous; a bare SHA is a
# pre-459 marker, written under the old end-of-run rule; garbage and an empty file are
# just broken.
printf 'gate-v2 HEAD\n' >"$marker"
expect "a marker that says HEAD covers nothing (not even HEAD)" 1 marker_covers "$settings"
printf 'gate-v2 %s\n' "${base:0:12}" >"$marker"
expect "an abbreviated SHA in the marker covers nothing" 1 marker_covers "$base"
printf '%s\n' "$base" >"$marker"
expect "a pre-459 marker (a bare SHA) covers nothing, not even its own commit" 1 marker_covers "$base"
: >"$marker"
expect "an empty marker covers nothing" 1 marker_covers "$base"
printf 'gate-v2 %s\ngate-v2 %s\n' "$base" "$base" >"$marker"
expect "a two-line marker covers nothing" 1 marker_covers "$base"
printf 'gate-v2 %s\n' "$(printf 'f%.0s' $(seq 1 40))" >"$marker"
expect "a well-formed SHA that is no commit here covers nothing" 1 marker_covers "$base"

# --- gate_deploy_step: deploy.sh's whole decision ----------------------------------------
# HEAD is $settings; the marker goes back to $base, which covers $scratch and $reverted.
fx gate_write_marker "$base"
step_is "a covered REV skips the suites while HEAD is an uncovered commit elsewhere" skip "$scratch"
step_is "an uncovered REV that IS HEAD's tree, on a clean tree, runs the gate" run "$settings"
step_is "an uncovered REV whose tree differs from HEAD's is refused up front" refuse-head "$code"
echo 'pub fn edit() {}' >>crates/store/src/lib.rs
step_is "an uncovered REV over a dirty tree is refused up front" refuse-dirty "$settings"
step_is "a covered REV skips even over a dirty tree (the marker is about commits)" skip "$scratch"
fx git checkout -q -- crates/store/src/lib.rs
echo 'log' >deploy.log
step_is "an untracked log in the checkout is a dirty tree" refuse-dirty "$settings"
if gate_dirt_hint "$(gate_tree_report)" | grep -q "Only untracked files"; then
    pass "…and the advice names an untracked file, not a fresh checkout"
else
    fail "…and the advice names an untracked file, not a fresh checkout — got: $(gate_dirt_hint "$(gate_tree_report)")"
fi
fx rm deploy.log
echo 'pub fn edit() {}' >>crates/store/src/lib.rs
if gate_dirt_hint "$(gate_tree_report)" | grep -q "fresh checkout"; then
    pass "…while a modified file gets the fresh-checkout advice"
else
    fail "…while a modified file gets the fresh-checkout advice — got: $(gate_dirt_hint "$(gate_tree_report)")"
fi
fx git checkout -q -- crates/store/src/lib.rs
fx rm "$marker"
step_is "with no marker, HEAD's own clean tree runs the gate" run HEAD

# --- gate_tree_clean: what counts as dirty ------------------------------------------------
expect "a clean tree reads clean" 0 gate_tree_clean
echo 'pub fn d() {}' >>crates/store/src/lib.rs
expect "an uncommitted crates/ edit reads dirty" 1 gate_tree_clean
fx git add crates/store/src/lib.rs
expect "a staged crates/ edit reads dirty" 1 gate_tree_clean
fx git reset -q --hard HEAD

echo 'mod new;' >crates/store/src/new.rs
expect "an untracked file outside .scratch/ reads dirty (cargo finds tests/*.rs by itself)" 1 gate_tree_clean
fx git config status.showUntrackedFiles no
expect "…even under status.showUntrackedFiles=no" 1 gate_tree_clean
fx git config --unset status.showUntrackedFiles
echo 'crates/store/src/new.rs' >>.git/info/exclude
expect "…even when .git/info/exclude ignores it" 1 gate_tree_clean
: >.git/info/exclude
echo 'new.rs' >"$work/excludes"
fx git config core.excludesFile "$work/excludes"
expect "…even when core.excludesFile ignores it" 1 gate_tree_clean
fx git config --unset core.excludesFile
fx rm crates/store/src/new.rs
mkdir -p target && echo 'build output' >target/out.bin
expect "a file the COMMITTED .gitignore ignores leaves the tree clean" 0 gate_tree_clean

fx git update-index --skip-worktree crates/store/src/lib.rs
echo 'pub fn hidden() {}' >>crates/store/src/lib.rs
expect "an edit to a skip-worktree file reads dirty" 1 gate_tree_clean
fx git update-index --no-skip-worktree crates/store/src/lib.rs
fx git checkout -q -- crates/store/src/lib.rs
fx git update-index --assume-unchanged crates/store/src/lib.rs
echo 'pub fn hidden() {}' >>crates/store/src/lib.rs
expect "an edit to an assume-unchanged file reads dirty" 1 gate_tree_clean
fx git update-index --no-assume-unchanged crates/store/src/lib.rs
fx git checkout -q -- crates/store/src/lib.rs
expect "…and with the flags cleared the tree reads clean again" 0 gate_tree_clean

echo 'draft' >.scratch/issues/004-draft.md
echo 'edit' >>.scratch/issues/001.md
expect "uncommitted and untracked .scratch/ files leave the tree clean" 0 gate_tree_clean
fx git checkout -q -- .scratch
fx rm -f .scratch/issues/004-draft.md

# --- gate_begin + gate_end_verdict: check.sh's half ---------------------------------------
fx git reset -q --hard "$scratch"
begin
if [ "$start" = "$scratch" ] && [ "$start_clean" = 1 ] && [ -f "$stamp" ]; then
    pass "gate_begin on a clean tree records HEAD, a clean start and a stamp"
else
    fail "gate_begin on a clean tree records HEAD, a clean start and a stamp — got head='$start' clean='$start_clean' stamp='$stamp'"
fi

expect "a run with nothing landing writes a marker" 0 gate_end_verdict "$start" "$start_clean" "$stamp"

echo '# 3' >.scratch/issues/003.md
commit 'scratch lands mid-run'
expect "a .scratch/-only commit landing mid-run still writes a marker" 0 gate_end_verdict "$start" 1 "$stamp"
echo 'more output' >target/out.bin
expect "an ignored file (target/) written mid-run still writes a marker" 0 gate_end_verdict "$start" 1 "$stamp"
echo 'draft' >.scratch/issues/005-draft.md
echo 'edit' >>.scratch/issues/001.md
expect "uncommitted and untracked .scratch/ files mid-run still write a marker" 0 gate_end_verdict "$start" 1 "$stamp"
fx git checkout -q -- .scratch
fx rm -f .scratch/issues/005-draft.md

echo 'pub fn c() {}' >>crates/store/src/lib.rs
commit 'code lands mid-run'
verdict_says "a code commit landing between start and end means no marker" "HEAD moved" "$start" 1 "$stamp"

# Start and end identical, the middle not: cargo compiled the edit.
fx git reset -q --hard "$start"
begin
echo 'pub fn short_circuited() {}' >>crates/store/src/lib.rs
fx git checkout -q -- crates/store/src/lib.rs
expect "an edit reverted during the run leaves a clean tree and the start HEAD" 0 gate_tree_clean
verdict_says "…and still means no marker" "changed during the run (first: crates/store/src/lib.rs)" "$start" "$start_clean" "$stamp"

fx git reset -q --hard "$start"
begin
echo 'pub fn e() {}' >>crates/store/src/lib.rs
commit 'code, then reverted, mid-run'
fx git -c commit.gpgsign=false -c core.hooksPath=/dev/null revert --no-edit HEAD
expect "a commit and its revert during the run leave the start tree" 0 gate_trees_equal "$start" HEAD
verdict_says "…and still mean no marker" "changed during the run" "$start" "$start_clean" "$stamp"

verdict_says "no start stamp means no marker" "no start stamp" "$start" 1 ""
verdict_says "a start stamp that is gone means no marker" "no start stamp" "$start" 1 "$work/no-such-stamp"

fx git reset -q --hard "$start"
echo 'pub fn dirty() {}' >>crates/store/src/lib.rs
begin
if [ "$start_clean" = 0 ]; then pass "gate_begin on a dirty tree records a dirty start"; else fail "gate_begin on a dirty tree records a dirty start — got clean='$start_clean'"; fi
fx git checkout -q -- crates/store/src/lib.rs
verdict_says "a tree dirty at the start means no marker, even if clean at the end" "at the start" "$start" "$start_clean" "$stamp"

begin
echo 'pub fn d() {}' >>crates/store/src/lib.rs
verdict_says "an uncommitted crates/ edit at the end means no marker" "at the end" "$start" "$start_clean" "$stamp"
fx git checkout -q -- crates/store/src/lib.rs

expect "an empty start HEAD means no marker" 1 gate_end_verdict "" 1 "$stamp"

# Outside any repository, every predicate says no rather than defaulting to yes.
cd "$work" || exit 1
expect "outside a repository the tree does not read clean" 1 gate_tree_clean
expect "outside a repository nothing is covered" 1 marker_covers "$base"
expect "outside a repository no marker is written" 1 gate_end_verdict "$start" 1 "$stamp"
step_is "outside a repository the deploy does not skip or run" refuse-head "$base"

echo
if [ "$ran" -ne "$planned" ]; then
    echo "FAIL ran $ran cases but $planned are planned — a case was skipped, or one was added without updating \$planned"
    failures=$((failures + 1))
fi
finished=1
if [ "$failures" -eq 0 ]; then
    echo "all gate-marker tests passed ($ran cases)"
    exit 0
fi
echo "$failures gate-marker test(s) failed"
exit 1
