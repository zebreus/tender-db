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
