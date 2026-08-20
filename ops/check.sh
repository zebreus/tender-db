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
PRUNE
}
prune_stale_test_binaries

# Leaked test scratch databases (issue 260 follow-up, found at 26,180 files / 10.2 GB).
# Every store/ingest test writes /tmp/tender-db-<name>-<pid>.db and removes it on the
# way out — but a test that PANICS never reaches its remove, and several remove only
# the bare path while turso leaves an 11 MB -wal beside it. Anything older than two
# hours cannot belong to a live run (the whole gate takes minutes), so it is leak.
find /tmp -maxdepth 1 -name 'tender-db-*' -type f -mmin +120 -delete 2>/dev/null || true

started=$(date +%s)
for args in "test -p model" "test -p store" "test -p ingest" "test-app"; do
    printf '\n\033[1m==> cargo %s\033[0m\n' "$args"
    # Unquoted on purpose: each entry is a small fixed argv, and `set -e` carries a
    # failure straight out of the loop.
    # shellcheck disable=SC2086
    cargo $args
done
elapsed=$(( $(date +%s) - started ))

# The marker the deploy gate reads. Only written for a CLEAN tree: a green run over a
# dirty tree says nothing about the commit, and a marker that can lie is worse than no
# marker. Lives under target/ (gitignored) — it is a local fact, not a repository one.
if [ -z "$(git status --porcelain)" ]; then
    mkdir -p target
    git rev-parse HEAD > target/.tests-green
    printf '\n\033[1m==> all suites green in %ss at %s\033[0m\n' "$elapsed" "$(git rev-parse --short HEAD)"
else
    printf '\n\033[1m==> all suites green in %ss (tree DIRTY — no marker written)\033[0m\n' "$elapsed"
fi
