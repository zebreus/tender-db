# shellcheck shell=bash
# The test gate's marker, as predicates both its writer and its reader source (issue
# 459). Sourced, never run: `ops/check.sh` decides whether to WRITE the marker,
# `deploy.sh` decides whether the marker COVERS the commit it ships, and
# `ops/test-gate-marker.sh` pins both against a throwaway repository.
#
# Why one file. The marker (`target/.tests-green`, gitignored, a local fact) used to be
# written as "HEAD at the END of the run, if the tree was clean then" and read as "equals
# HEAD and the tree is clean now". Both halves were bound to the working tree, and the
# deploy ships `$REV`, not the working tree:
#   * `./deploy.sh origin/main` skipped the suites on a green HEAD and shipped a
#     different, untested commit;
#   * a commit landing in this shared worktree during the ~12-minute run moved HEAD, the
#     tree was clean again by the end, and the marker named a commit cargo never compiled;
#   * any commit at all — 16 of 20 touch only `.scratch/` — moved HEAD off the marker, so
#     operators judged "only .scratch/ changed" by eye and deployed with SKIP_TESTS=1.
#     A judgement by eye in place of a gate is a permissive gate.
#
# So the marker is now a fact about a COMMIT ("the suites passed on this commit's tree,
# outside .scratch/"), and covering is a fact about two commits' TREES. Nothing here
# reads the working tree except the cleanliness and quiet-run checks the writer needs.
#
# The `.scratch/` exclusion is sound only while no build or test reads under `.scratch/`
# (2026-10-01: no include_str!/include_bytes! from it and no path literal into it under
# crates/). Widen the exclusion only with a stated reason of the same kind.
#
# Every predicate here answers "no" when it cannot answer: a git error (exit 128), an
# unreadable marker, a marker that is not a full hex SHA in the current format, an
# unknown rev, a missing start stamp. A permissive default is the bug class this file
# exists to remove (docs/agents/instrument-discipline.md ledger #6), so none of these
# functions has one.

# Outside `.scratch/`, from the top of the repository wherever the caller stands.
GATE_PATHSPEC=(':(top)' ':(top,exclude).scratch')

# The marker's first word. A pre-459 marker was a bare SHA written under the old rule
# ("HEAD at the END of the run") — the race this file fixes — so it must cover nothing:
# the first deploy after 459 re-gates once rather than trusting it (review of 459).
GATE_MARKER_FORMAT=gate-v2

# Every git call the predicates make goes through here.
#   --no-optional-locks: a plain `git status` takes .git/index.lock to refresh the index,
#     and in this shared worktree that makes another agent's concurrent `git add` or
#     `git commit` fail with "index.lock: File exists" — 13 of 150 adds beside a status
#     loop on 2026-10-01, 0 with the flag.
#   GIT_NO_REPLACE_OBJECTS=1: with `git replace <code> <base>` in the repository,
#     diff-tree reads the base's tree under the code commit's name, so a marker at base
#     "covers" code by construction.
#   core.fsmonitor=false: a stale fsmonitor daemon makes `git status` report what it last
#     heard, not what the disk holds.
gate_git() { GIT_NO_REPLACE_OBJECTS=1 git --no-optional-locks -c core.fsmonitor=false "$@"; }

# The marker's path, at the top of the CURRENT repository (so the self-test's throwaway
# repository has its own and never touches the real one).
gate_marker_path() {
    local top
    top=$(gate_git rev-parse --show-toplevel 2>/dev/null) && [ -n "$top" ] || return 1
    printf '%s/target/.tests-green\n' "$top"
}

# gate_trees_equal <a> <b>: 0 iff both name commits whose trees are identical outside
# `.scratch/`. `diff-tree` is plumbing: unlike `git diff`, no textconv or external diff
# driver from someone's config can make two different blobs compare equal. Its exit 1
# (they differ) and 128 (a bad rev) both come back non-zero, which is the point.
gate_trees_equal() {
    [ -n "${1:-}" ] && [ -n "${2:-}" ] || return 1
    gate_git diff-tree --quiet -r "$1^{commit}" "$2^{commit}" -- "${GATE_PATHSPEC[@]}" 2>/dev/null
}

# gate_tree_report: prints one line per thing that makes the tree outside `.scratch/`
# differ from HEAD, and nothing when it is clean; non-zero when git could not read it (a
# failed `git status` prints nothing, which a bare `[ -z "$(git status …)" ]` would read
# as clean). Three sources, because each hides something from the others:
#   modified  …   `git status`, tracked files only: modified, staged, deleted.
#   unwatched …   a file flagged assume-unchanged (lower-case tag in `ls-files -v`) or
#                 skip-worktree (`S`): git status does not look at it at all, so its
#                 edits are invisible there and the tree cannot be called clean.
#   untracked …   `ls-files --others` against the COMMITTED .gitignore files only. cargo
#                 compiles an untracked tests/*.rs it finds, so untracked counts; and
#                 `.git/info/exclude`, `core.excludesFile` and `status.showUntrackedFiles`
#                 are personal settings that must not be able to hide one.
gate_tree_report() {
    local tracked flags untracked line
    tracked=$(gate_git status --porcelain --untracked-files=no -- "${GATE_PATHSPEC[@]}" 2>/dev/null) || return 1
    flags=$(gate_git ls-files -v -- "${GATE_PATHSPEC[@]}" 2>/dev/null) || return 1
    untracked=$(gate_git ls-files --others --exclude-per-directory=.gitignore -- "${GATE_PATHSPEC[@]}" 2>/dev/null) || return 1
    if [ -n "$tracked" ]; then
        while IFS= read -r line; do printf 'modified  %s\n' "$line"; done <<<"$tracked"
    fi
    if [ -n "$flags" ]; then
        while IFS= read -r line; do
            case "$line" in
                [a-z]\ *|S\ *) printf 'unwatched %s (assume-unchanged or skip-worktree: git status cannot see its edits)\n' "${line#? }" ;;
            esac
        done <<<"$flags"
    fi
    if [ -n "$untracked" ]; then
        while IFS= read -r line; do printf 'untracked %s\n' "$line"; done <<<"$untracked"
    fi
    return 0
}

# gate_tree_clean: 0 iff gate_tree_report READ the tree and found nothing.
gate_tree_clean() {
    local report
    report=$(gate_tree_report) || return 1
    [ -z "$report" ]
}

# gate_dirt_hint <report>: one line of advice for a refusal, chosen by what the report
# holds. A log redirected into the checkout (`nohup ./deploy.sh … > deploy.log`, or
# `ops/check.sh > gate.out`) is the usual untracked file, and "deploy from a fresh
# checkout" — a cold 35-minute gate — is the wrong advice for it (review of 459).
gate_dirt_hint() {
    if [ -n "${1:-}" ] && ! grep -qv '^untracked ' <<<"$1"; then
        echo "Only untracked files — a log or an output redirected into the checkout? Move it outside (e.g. under ${TMPDIR:-/tmp}); never delete or stash someone else's work to clear the path."
    else
        echo "Commit the change, or deploy from a fresh checkout of the commit (docs/operations.md, Deploy) — never stash someone else's work to clear the path."
    fi
}

# gate_marker_commit: prints the commit the marker names, or fails. The marker must be
# exactly one line `gate-v2 <full hex SHA>`: a marker that says `HEAD` would cover
# whatever is checked out, an abbreviated SHA can become ambiguous, and a bare SHA is a
# pre-459 marker written under the old rule.
gate_marker_commit() {
    local path m
    path=$(gate_marker_path) || return 1
    [ -f "$path" ] || return 1
    m=$(cat "$path" 2>/dev/null) || return 1
    [[ "$m" =~ ^gate-v2\ ([0-9a-f]{40}([0-9a-f]{24})?)$ ]] || return 1
    printf '%s\n' "${BASH_REMATCH[1]}"
}

# marker_covers <rev>: 0 iff the marker names a commit whose tree equals <rev>'s outside
# `.scratch/`. This is the ONLY way past deploy.sh's gate step short of SKIP_TESTS=1.
marker_covers() {
    local rev=${1:-} m
    [ -n "$rev" ] || return 1
    m=$(gate_marker_commit) || return 1
    gate_trees_equal "$m" "$rev"
}

# gate_deploy_step <rev>: deploy.sh's whole gate decision, as one word, so it is pinned
# here rather than in deploy.sh's call sites (review of 459: `marker_covers HEAD` in
# deploy.sh would have brought the first bug back with every case here still green).
#   skip          the marker covers <rev> — whatever HEAD or the working tree are
#   refuse-head   no marker covers <rev>, and HEAD's tree differs from it outside
#                 .scratch/, so ops/check.sh here would test a tree the deploy does not ship
#   refuse-dirty  no marker, HEAD is <rev>'s tree, but the working tree is not clean
#                 (or could not be read), so check.sh would write no marker
#   run           no marker, HEAD is <rev>'s tree and the tree is clean: run check.sh,
#                 and then the marker must cover <rev>
# Every failure to read lands in a refusal, never in `skip`.
gate_deploy_step() {
    local rev=${1:-}
    if marker_covers "$rev"; then
        echo skip
    elif ! gate_trees_equal HEAD "$rev"; then
        echo refuse-head
    elif ! gate_tree_clean; then
        echo refuse-dirty
    else
        echo run
    fi
}

# gate_begin: record what this run will certify, BEFORE cargo starts. Sets
#   GATE_STAMP        a fresh file whose mtime is the start of the run,
#   GATE_START_HEAD   HEAD now (empty if unreadable, which gate_end_verdict refuses),
#   GATE_START_CLEAN  1 if gate_tree_clean, else 0,
#   GATE_START_DIRT   gate_tree_report's lines when not clean, for check.sh's message.
# The stamp comes first, then a pause longer than the kernel's timestamp tick (a jiffy,
# 4–10 ms): every write after the cleanliness read below is then strictly newer than the
# stamp, which gate_quiet_since relies on.
gate_begin() {
    GATE_STAMP=$(mktemp "${TMPDIR:-/tmp}/gate-stamp.XXXXXX") && [ -f "$GATE_STAMP" ] || { GATE_STAMP=; return 1; }
    sleep 0.05
    GATE_START_HEAD=$(gate_git rev-parse --verify --quiet HEAD 2>/dev/null) || GATE_START_HEAD=
    if GATE_START_DIRT=$(gate_tree_report) && [ -z "$GATE_START_DIRT" ]; then
        GATE_START_CLEAN=1
    else
        GATE_START_CLEAN=0
        [ -n "$GATE_START_DIRT" ] || GATE_START_DIRT="(git could not read the tree)"
    fi
    return 0
}

# gate_quiet_since <stamp>: 0 iff no file outside `.scratch/` that git tracks or could
# track (untracked and not ignored by the committed .gitignore files) was written after
# the stamp. On failure prints why, in a form check.sh's closing line can carry: the
# first such path, or what stopped it from looking.
#
# Why: start and end can be identical while the middle was not. An edit reverted during
# the run — a red-first short-circuit, a stash and pop, a commit and its revert — leaves
# the same HEAD and a clean tree at both ends, while cargo compiled the edit. Only the
# files' mtimes still say it happened. Ignored files (target/, *.db, __pycache__/) are not
# part of any commit, so writing them is not a change to the tree. mtime, not ctime:
# this container's git runs with core.trustctime=false, i.e. ctime is not to be trusted
# here, and a false "changed" would cost every gate its marker.
gate_quiet_since() {
    local stamp=${1:-} top out rc=0
    if [ -z "$stamp" ] || [ ! -f "$stamp" ]; then
        echo "no start stamp${stamp:+ at $stamp}, so a change during the run cannot be ruled out"; return 1
    fi
    top=$(gate_git rev-parse --show-toplevel 2>/dev/null) && [ -n "$top" ] || {
        echo "not inside a repository, so a change during the run cannot be ruled out"; return 1; }
    out=$(python3 - "$top" "$stamp" "${GATE_PATHSPEC[@]}" 2>/dev/null <<'PY'
import os, subprocess, sys
try:
    top, stamp, pathspec = sys.argv[1], sys.argv[2], sys.argv[3:]
    since = os.stat(stamp).st_mtime_ns
    env = dict(os.environ, GIT_NO_REPLACE_OBJECTS="1")
    listing = subprocess.run(
        ["git", "--no-optional-locks", "-C", top, "ls-files", "-z", "--cached", "--others",
         "--exclude-per-directory=.gitignore", "--", *pathspec],
        capture_output=True, env=env, check=True, stdin=subprocess.DEVNULL).stdout
    for rel in listing.split(b"\0"):
        if not rel:
            continue
        name = rel.decode(errors="replace")
        try:
            st = os.lstat(os.path.join(top.encode(), rel))
        except FileNotFoundError:
            print("files outside .scratch/ changed during the run (first: %s, gone)" % name)
            sys.exit(1)
        if st.st_mtime_ns > since:
            print("files outside .scratch/ changed during the run (first: %s)" % name)
            sys.exit(1)
except Exception as e:  # noqa: BLE001 — any failure to look is a "no", with its reason
    print("could not tell whether files changed during the run: %s" % str(e).replace("\n", " ")[:200])
    sys.exit(2)
PY
) || rc=$?
    if [ "$rc" -eq 0 ] && [ -z "$out" ]; then
        return 0
    fi
    printf '%s\n' "${out:-could not tell whether files changed during the run (exit $rc)}"
    return 1
}

# gate_end_verdict <start_head> <start_clean: 1|0> <stamp>: the condition check.sh writes
# the marker on, at the END of a green run. 0 iff
#   1. the tree outside `.scratch/` was clean at the start (start_clean=1) AND is now,
#   2. HEAD has not moved outside `.scratch/` since the start, AND
#   3. no file outside `.scratch/` was written since <stamp> (gate_quiet_since).
# Then cargo compiled exactly <start_head>'s tree, so the marker is <start_head> — not
# HEAD now, which another agent's commit can have moved during the run. On failure it
# prints which condition failed, for check.sh's closing line.
gate_end_verdict() {
    local start_head=${1:-} start_clean=${2:-0} stamp=${3:-} now why
    if [ -z "$start_head" ]; then
        echo "no start HEAD was recorded"; return 1
    fi
    if [ "$start_clean" != "1" ]; then
        echo "tree DIRTY outside .scratch/ at the start"; return 1
    fi
    if ! gate_tree_clean; then
        echo "tree DIRTY outside .scratch/ at the end"; return 1
    fi
    if ! now=$(gate_git rev-parse --verify --quiet HEAD 2>/dev/null) || [ -z "$now" ]; then
        echo "HEAD unreadable at the end"; return 1
    fi
    if ! gate_trees_equal "$start_head" "$now"; then
        echo "HEAD moved ${start_head:0:7} → ${now:0:7} outside .scratch/ during the run"; return 1
    fi
    if ! why=$(gate_quiet_since "$stamp"); then
        echo "$why"; return 1
    fi
    return 0
}

# gate_write_marker <sha>: atomically, so a concurrent reader never sees half a line.
gate_write_marker() {
    local path
    [[ "${1:-}" =~ ^[0-9a-f]{40}([0-9a-f]{24})?$ ]] || return 1
    path=$(gate_marker_path) || return 1
    mkdir -p "$(dirname "$path")" || return 1
    printf '%s %s\n' "$GATE_MARKER_FORMAT" "$1" >"$path.tmp.$$" && mv -f "$path.tmp.$$" "$path"
}
