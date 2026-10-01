# shellcheck shell=bash
# Whether the public repository has a commit — the question deploy.sh asks before it ships
# one (issue 463). Sourced, never run; `ops/test-published.sh` pins it against throwaway
# repositories, and deploy.sh runs that pin first.
#
# /_source and /v1's source_offer link `$PUBLIC_REPOSITORY/tree/<rev>` — that link IS the
# AGPL §13 offer — and deploy.sh pushes only to the box, which has run commits GitHub did
# not have (2026-09-04). So a rev has to be on GitHub before it ships.
#
# The repository is named by its URL, never by a remote. The first version asked whether
# a branch of `origin` contained the rev, and `origin` is whatever the checkout was cloned
# from: in a local clone of the shared tree — the "fresh checkout" deploy.sh's refusals
# prescribe — that is the shared tree, whose branches carry every commit made there, so an
# unpushed rev passed and /_source linked a 404 (review of 463).

# The repository the app links: `v1::REPOSITORY` in crates/app/src/v1/mod.rs, whose
# source_tests read this line, so the two cannot drift apart.
PUBLIC_REPOSITORY=https://github.com/zebreus/tender-db

# Where the fetch puts the public repository's branches. A namespace of its own, so no
# remote's tracking refs can answer for it.
PUBLISHED_REFS=refs/published/heads

# rev_published <rev>: one word —
#   published    a branch of $PUBLIC_REPOSITORY, fetched just now, contains <rev>
#   unpublished  the fetch succeeded and no branch contains <rev> (a tag does not count)
#   unreadable   <rev> names no commit here, the fetch failed, or git could not answer
# Fetched every time, not read from refs a previous fetch left: those are only as current
# as that fetch, so a failed fetch is `unreadable`, never a verdict from them. `--prune`
# drops a branch deleted upstream, so a rev only that branch carried does not pass on its
# stale ref. GIT_TERMINAL_PROMPT=0: a repository that asks for credentials fails the fetch
# rather than hanging the deploy at a prompt.
rev_published() {
    local sha refs
    sha=$(git rev-parse --verify --quiet "${1:-}^{commit}" 2>/dev/null) && [ -n "$sha" ] || { echo unreadable; return 0; }
    GIT_TERMINAL_PROMPT=0 git fetch --quiet --prune --no-tags "$PUBLIC_REPOSITORY" "+refs/heads/*:$PUBLISHED_REFS/*" >/dev/null 2>&1 \
        || { echo unreadable; return 0; }
    refs=$(git for-each-ref --count=1 --contains "$sha" "$PUBLISHED_REFS" 2>/dev/null) || { echo unreadable; return 0; }
    if [ -n "$refs" ]; then echo published; else echo unpublished; fi
}
