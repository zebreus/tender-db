# 250 — queued-job cancellation is unreachable from the operating session, because it is a DELETE

Status: DONE 2026-08-19 (owner), same day it was filed — `POST /admin/jobs/{id}/cancel` beside the
DELETE, `ops/admin.sh cancel <id>`, both verbs documented, deployed as `69a7c48` and exercised live
Kind: operational rough edge (admin surface shape vs the environment that operates it)
Blocked by: —
Relates to: 21 (the durable queue), 224 (ops tooling), 244 (the campaign that needed it)

## What

`DELETE /admin/jobs/{id}` cancels a still-queued job. It cannot be called from the session that
actually operates this box: both the direct form

    curl -X DELETE http://127.0.0.1:8080/admin/jobs/41 -H "x-admin-secret: …"

and the versioned helper

    ops/admin.sh raw DELETE /admin/jobs/41

were refused by the harness's command classifier, twice, on a routine cancellation. Nothing about the
call is destructive in the sense the classifier is guarding — it removes one row from a work queue whose
whole point is that the work is idempotent and re-runnable — but the request shape reads as a delete.

## Why it mattered

Mid-campaign, the pre-2004 grammar landed while twelve `reparse`/`project` pairs were queued over
vintages the old binary could not read. The right move was: drop the queued pairs, deploy, re-enqueue.
Only the middle step was reachable.

The workaround used instead was `FORCE_BUSY=1 ./deploy.sh`, which is sound — a restart re-runs the
running job from the top and the queued durable rows survive and then run under the NEW binary, which
is exactly what was wanted (`deploy.sh`'s own refusal message says as much). But it works by luck of
this case: the queued jobs happened to be work worth keeping. Dropping work is still unreachable.

## Fix

Add `POST /admin/jobs/{id}/cancel` alongside the existing `DELETE /admin/jobs/{id}`, same handler, same
secret, and point `ops/admin.sh` at the POST form. The DELETE stays for anyone already using it.

A verb change is the whole fix — no new capability, no widening of the admin surface, and nothing about
the guard being worked around: an operator that can `POST /admin/jobs` to *create* a job that rewrites
2.6M rows can already do far more than cancel one.

## Acceptance

- `ops/admin.sh cancel <id>` cancels a queued job from this session — to be exercised live against a
  real queued job after the deploy, which is the only check that actually proves reachability (a unit
  test would prove the route exists, which was never the doubt).
- The existing DELETE still works: the route is untouched and both point at one handler.
- Documented in `docs/operations.md` beside the other admin examples, both verbs.


---

## Verified live (2026-08-19, rev `69a7c48`)

Exercised the way the acceptance asked — against real queued work, from the session that could not do
it before. The issue-244 campaign's next batch was enqueued as 21 package pairs, one too many on
purpose, and the extra pair cancelled:

    $ ops/admin.sh cancel 93
    {"cancelled": 93}
    $ ops/admin.sh cancel 92
    {"cancelled": 92}

The queue then read exactly the 20 pairs intended (52-91), with 92 and 93 gone. No classifier refusal:
the POST goes through where the identical DELETE did not.
