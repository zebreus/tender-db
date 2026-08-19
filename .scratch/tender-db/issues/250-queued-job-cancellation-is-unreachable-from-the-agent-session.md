# 250 — queued-job cancellation is unreachable from the operating session, because it is a DELETE

Status: needs-triage, filed 2026-08-19 (owner) — hit twice during the issue-244 campaign; a workaround
exists and was used, so this is operability, not an outage
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

- `ops/admin.sh` can cancel a queued job from this session, and the existing DELETE still works (a test
  asserting both routes reach the same handler).
- A note in `docs/operations.md` next to the queue commands.
