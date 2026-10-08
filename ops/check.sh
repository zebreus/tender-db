#!/usr/bin/env bash
# Run every crate's tests the way that crate has to be run, and record the commit they
# passed at (issue 254).
#
# Why a script rather than "just run cargo test": on 2026-08-20 three different green
# LOOKING zeros went past me in one session —
#
#   * `cargo test … | grep … | head` reports the PIPELINE's exit code, not cargo's, and
#     `head` truncated the doc-test section that was failing;
#   * nine doctests had been failing for several commits under that truncation;
#   * `cargo test -p tender-db --lib` prints "ok. 0 passed" because the server modules are
#     behind `#[cfg(feature = "server")]` — .cargo/config.toml documents that trap and
#     provides `cargo test-app`, and I had read it and still ran the wrong command.
#
# So: cargo's own exit code, unpiped, one command per crate including the alias.
set -euo pipefail
cd "$(dirname "$0")/.."

export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0

# Prune superseded test binaries before building (issue 260). cargo names each test
# executable `<target>-<16 hex of the build hash>` and NEVER removes the old one, so a
# day of iterating leaves a graveyard: 12 test targets x every rebuild, at 200-350 MB
# each because these link the whole dependency tree. On 2026-08-20 that filled the disk
# three times in one session — twice as `No space left on device` and once, far less
# obviously, as `ld terminated with signal 7 [Bus error]` with an LLVM "please file a
# bug" banner, which is what a failing mmap looks like from inside the linker.
#
# Keep only the NEWEST hash per target: that is the one this build will reuse, and any
# other is either stale or cheap to relink. Deliberately conservative — it never touches
# rlibs, build scripts, or anything outside deps/.
prune_stale_test_binaries() {
    local deps=target/debug/deps
    [ -d "$deps" ] || return 0
    python3 - "$deps" <<'PRUNE'
import os, re, sys, collections
deps = sys.argv[1]
pat = re.compile(r"^(.+)-[0-9a-f]{16}$")
groups = collections.defaultdict(list)
for name in os.listdir(deps):
    path = os.path.join(deps, name)
    if not os.path.isfile(path) or not os.access(path, os.X_OK):
        continue
    m = pat.match(name)
    # Only the big linked executables; an .rlib or .so does not match the pattern
    # anyway, and the size floor keeps small helper binaries out of it.
    if m and os.path.getsize(path) > 20_000_000:
        groups[m.group(1)].append((os.path.getmtime(path), path))
freed = 0
for _, entries in groups.items():
    if len(entries) < 2:
        continue
    entries.sort()
    for _, path in entries[:-1]:          # keep the newest
        freed += os.path.getsize(path)
        os.remove(path)
if freed:
    print(f"==> pruned {freed / 2**30:.1f} GB of superseded test binaries")

# Issue 260, the second thing that grows: superseded HASH VARIANTS of the largest
# dependency archives. A profile or feature change gives every rlib a new hash and
# leaves the old one behind — nine variants of the two turso archives (200–300 MB
# each, 4.5 GB together) sat in deps/ on 2026-09-18 with 3.5 GB of allowance left.
# Keep the newest of each stem above 100 MB; a stale one cargo still wanted costs a
# rebuild of that crate, never a wrong build.
#
# The turso kits' cdylibs (`.so`, 58-116 MB) matched no pattern and stayed forever: ten
# variants on 2026-09-29. They are pruned the same way from 50 MB up. The key is
# (stem, extension), because one build of a kit writes `.a`, `.so` and `.rlib` under ONE
# hash: keyed by stem alone, the newest of that live pair would delete the other.
lib = re.compile(r"^(lib.+)-[0-9a-f]{16}\.(a|rlib|so)$")
archives = collections.defaultdict(list)
for name in os.listdir(deps):
    path = os.path.join(deps, name)
    m = lib.match(name)
    floor = 50_000_000 if m and m.group(2) == "so" else 100_000_000
    if m and os.path.isfile(path) and os.path.getsize(path) > floor:
        archives[(m.group(1), m.group(2))].append((os.path.getmtime(path), path))
freed = 0
for _, entries in archives.items():
    if len(entries) < 2:
        continue
    entries.sort()
    for _, path in entries[:-1]:
        freed += os.path.getsize(path)
        os.remove(path)
if freed:
    print(f"==> pruned {freed / 2**30:.1f} GB of superseded dependency archives")
PRUNE
}
prune_stale_test_binaries

# Leaked test scratch databases (issue 260 follow-up, found at 26,180 files / 10.2 GB).
# Every store/ingest test writes /tmp/tender-db-<name>-<pid>.db and removes it on the
# way out — but a test that PANICS never reaches its remove, and several remove only
# the bare path while turso leaves an 11 MB -wal beside it. Anything older than two
# hours cannot belong to a live run (the whole gate takes minutes), so it is leak.
find /tmp -maxdepth 1 -name 'tender-db-*' -type f -mmin +120 -delete 2>/dev/null || true

# Would one more build family fit (issue 475)? Asked AFTER the prune and the /tmp sweep
# above (they free what is already superseded) and BEFORE cargo: a run that re-hashes
# writes a whole new family beside the old one, and a run that cannot fit dies mid-link
# as GATE-EXIT=101 with no FAILED line, or as `ld … signal 7 [Bus error]`. The refusal
# prints `==> GATE REFUSED: disk`, the free space, the need, `du -sh target` and the
# remedy, and exits 3 before cargo starts. The threshold (13 GiB with no reusable family,
# ~3 GiB when target/.gate-inputs says target/ holds the family these inputs build;
# override GATE_DISK_NEED_BYTES) and its fail-closed rules live in ops/gate-disk.sh,
# pinned by ops/test-gate-disk.sh — which runs first, so a preflight that answered "fits"
# wrongly cannot wave a doomed build through. The paths are cargo's and the tests' own:
# CARGO_TARGET_DIR when set, and TMPDIR (std::env::temp_dir) when set.
if ! disk_selftest=$(bash ops/test-gate-disk.sh 2>&1); then
    printf '%s\n' "$disk_selftest" >&2
    echo "==> GATE REFUSED: disk — ops/test-gate-disk.sh FAILED (above), so the disk preflight cannot be trusted; cargo was NOT started" >&2
    exit 3
fi
# shellcheck source=ops/gate-disk.sh
. ops/gate-disk.sh
# Issue 475 unit 2: a re-hash (lockfile, [patch], profile, features, toolchain) makes
# target/'s family superseded; clean it BEFORE the preflight reads free space, so the
# run needs one family's room rather than two.
gate_disk_clean_on_rehash "${CARGO_TARGET_DIR:-target}" || {
    echo "==> GATE REFUSED: disk — cargo clean failed after the build inputs changed; cargo test was NOT started" >&2
    exit 3
}
gate_disk_preflight "${CARGO_TARGET_DIR:-target}" "${TMPDIR:-/tmp}" || exit 3

# The two-hour rule alone let a SESSION's runs pile up: on 2026-09-29 /tmp held 6,573
# scratch files (4.2 GB) from seven gates, ~0.6 GB per run — about 230 tests leave their
# .db and its 2.6 MB -wal every time, not only the panicking ones. So the gate also sweeps
# its own run on the way out, pass or fail: every tender-db-* file written since it
# started that no process holds open (read from /proc). A test still running elsewhere
# holds its database open and is left alone.
sweep_run_scratch() {
    python3 - "$1" <<'SWEEP' || true
import glob, os, sys
since = float(sys.argv[1])
held = set()
for fd in glob.glob('/proc/[0-9]*/fd/*'):
    try:
        held.add(os.readlink(fd))
    except OSError:
        pass
freed = count = 0
for path in glob.glob('/tmp/tender-db-*'):
    try:
        st = os.stat(path)
    except OSError:
        continue
    if not os.path.isfile(path) or st.st_mtime < since:
        continue
    base = path[:-4] if path.endswith(('-wal', '-shm')) else path
    if {base, base + '-wal', base + '-shm'} & held:
        continue
    try:
        os.remove(path)
    except OSError:
        continue
    freed += st.st_size
    count += 1
if count:
    print(f"==> swept {count} test scratch files ({freed / 2**30:.2f} GB) this run left in /tmp")
SWEEP
}

started=$(date +%s)
GATE_STAMP=
trap 'sweep_run_scratch "$started"; [ -z "$GATE_STAMP" ] || rm -f "$GATE_STAMP"' EXIT

# What this run will certify, recorded BEFORE cargo starts (issue 459). The marker used
# to be `git rev-parse HEAD` taken AFTER cargo finished: in this shared worktree another
# agent can commit during the ~12-minute run, the tree is clean again by the end, and the
# marker then named a commit cargo may never have compiled. Now gate_begin takes a start
# stamp, HEAD and the tree's state here, and the marker is that start HEAD, written only
# if the tree was clean (outside .scratch/) at the start and the end, HEAD has not moved
# outside .scratch/, and no file outside .scratch/ was written in between (an edit
# reverted mid-run leaves both ends identical while cargo compiled it). The predicates
# live in ops/gate-marker.sh, which deploy.sh sources too, and ops/test-gate-marker.sh
# pins them.
# shellcheck source=ops/gate-marker.sh
. ops/gate-marker.sh
gate_begin || { echo "check.sh: could not create the gate's start stamp — no marker will be written" >&2; GATE_START_CLEAN=0; }
start_head=${GATE_START_HEAD:-}
start_clean=${GATE_START_CLEAN:-0}
if [ "$start_clean" != 1 ]; then
    printf '\n\033[1m==> tree DIRTY outside .scratch/ at the start — the suites run, but no marker will be written:\033[0m\n'
    sed -n '1,20s/^/    /p' <<<"${GATE_START_DIRT:-(unknown)}"
    printf '    %s\n' "$(gate_dirt_hint "${GATE_START_DIRT:-}")"
fi

# ONE cargo invocation over all four packages (issue 260, 2026-09-29). It used to be four
# (`test -p model`, `-p store`, `-p ingest`, `test-app-all`), and cargo resolves features
# per invocation: 41 of the 291 crates under turso_sdk_kit resolved differently for
# `-p store`, `-p ingest` and `-p tender-db --features server` (futures-core/-util,
# getrandom's `wasm_js`, …). Each step therefore built its own family of the turso kits,
# store and ingest, and the keep-newest prune above deleted the others at every start:
# 0.8 GB pruned and the kits recompiled on every gate. One invocation resolves once.
# The app's integration suites still run: plain `cargo test -p tender-db` runs lib, bins,
# tests/ and doctests, which is what issue 414's `test-app-all` (`--tests`) was for
# (`--lib` ran NOTHING in crates/app/tests, and tests/sql.rs sat red for twelve days).
printf '\n\033[1m==> cargo test -p model -p store -p ingest -p tender-db --features tender-db/server\033[0m\n'
cargo test -p model -p store -p ingest -p tender-db --features tender-db/server
# Green: target/ now holds the family these inputs build, so the next gate's preflight
# can credit it (issue 475). A failure to record costs the next gate the credit, never
# this one its verdict.
gate_disk_record_inputs "${CARGO_TARGET_DIR:-target}" \
    || echo "check.sh: could not record target/.gate-inputs — the next gate's disk preflight asks for a whole family" >&2
elapsed=$(( $(date +%s) - started ))

# The marker the deploy gate reads: the START head, and only when cargo provably compiled
# that commit's tree (gate_end_verdict, ops/gate-marker.sh). A green run over a dirty or
# moving tree says nothing about any commit, and a marker that can lie is worse than no
# marker. Lives under target/ (gitignored) — it is a local fact, not a repository one.
if why_not=$(gate_end_verdict "$start_head" "$start_clean" "$GATE_STAMP"); then
    gate_write_marker "$start_head" || { echo "check.sh: suites green but the marker could not be written" >&2; exit 1; }
    printf '\n\033[1m==> all suites green in %ss at %s (marker written)\033[0m\n' "$elapsed" "${start_head:0:7}"
else
    printf '\n\033[1m==> all suites green in %ss (%s — no marker written)\033[0m\n' "$elapsed" "$why_not"
fi
