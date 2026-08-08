# 162 — deploy.sh's regression guard misses diverged lines (silent feature rollback)

Status: open — fix designed, to land as a follow-up commit (do not edit deploy.sh while a deploy runs)
Role: ops

## What happened

Prod ran the read-path line (`1830d50`-era, deployed 2026-08-03/04: tender-scoped
lots reads, issue-115/117 fixes, deferred indexes). Later deploys shipped the
main line (`8c8182a`, `825dd64`), which had diverged and did NOT contain that
work. The deploy succeeded silently and prod regressed to the pre-`1830d50`
read path — measured 2026-08-08 ~23:05 Berlin: `GET /v1/tenders/1` (single-lot
tender) took **70.0s**; `/v1/lots?tender=1` 502'd. Nobody noticed for ~4 days
because the list endpoints stayed fast and nothing gated on detail-read latency.

## Root cause

deploy.sh's guard only refuses when the target rev is an **ancestor** of the
deployed rev:

    git -C $SRC merge-base --is-ancestor $REV "$DEPLOYED" → refuse

A diverged line is neither ancestor nor descendant, so it deploys without
comment. The guard answers "is this literally older?" but the question that
matters is "does this contain everything currently running?"

## Fix

Add the second arm: if the currently deployed rev is NOT an ancestor of the
target (`! git merge-base --is-ancestor $DEPLOYED $REV`), abort with a message
naming both revs and requiring an explicit `FORCE_DIVERGENT=1` override. A
fast-forward-only deploy policy makes silent rollback impossible while still
allowing a deliberate, named divergent deploy.

## Follow-ups

- The 2026-08-06 reclaim-all-then-rebuild recovery (`reset_tender_layer`)
  dropped the branch-built read indexes; the issue-111 startup builder (landed
  in `3485e3d`) is what heals that class. Verify it rebuilds them post-deploy.
- Consider a detail-read latency probe in /health/deep or the daily verify suite
  so a read-path regression trips something (issue 24/107 territory).
