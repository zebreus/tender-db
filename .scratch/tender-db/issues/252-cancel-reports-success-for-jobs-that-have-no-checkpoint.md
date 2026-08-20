# 252 — cancelling a running job reports success even when that kind of job cannot stop

Status: FIXED 2026-08-20 (owner), same firing it was found — `data-quality` now checks between
queries and stores nothing when stopped; `cancel` refuses with 409 for a kind that reads no flag.
AWAITING DEPLOY, and the prod acceptance is to cancel a real run and watch it end
Kind: operational surface tells an untruth (the worst kind of rough edge)
Blocked by: —
Relates to: 247 (where the running-job stop flag was built, for the reparse path), 250 (the POST verb
that made cancel reachable), 230/243 (the data-quality job this was aimed at), 21 (the durable queue)

## What

`ops/admin.sh cancel 202` against the running `data-quality` job:

    {"cancelled": 202}

and the log line `supervisor: job 202 is running — asked it to stop at its next checkpoint`. Six
minutes later the job had advanced from query 24 to query 34. It never stops.

The flag is real; the checkpoint is not. `Supervisor::cancelled()` has **exactly one caller** — the
`reparse` path, which checks it between notices (issue 247, where it was built for exactly the job that
needed it then). Every other long job — `data-quality`, `project`, `process`, `fetch`, `refold`,
`reprocess`, the snapshot — has no checkpoint at all, so `cancel` sets a flag nobody reads and the
caller is told it worked.

Reporting success for something that will not happen is worse than refusing: an operator who believes
the queue is about to free waits for it, and the flag then gets cleared silently when the job finally
concludes on its own.

## Why it bit now

The `data-quality` run is the longest job in the system (5,566 s measured, and issue 253 has it far
slower now). It is precisely the job an operator most wants to be able to stop, and the only one of the
long jobs whose loop has an obvious cheap checkpoint — it runs 417 independent queries across 32
windows and could check between any two of them.

## Fix, in two halves

1. **Give the long jobs checkpoints.** `data-quality` first, between queries: it holds no long
   transaction there, so stopping is free. Then `project` between phases and `process` between package
   members, both of which already have loops in the right shape. A stopped job records its run log
   saying it was cancelled and drops its durable row, exactly as the reparse path already does.
2. **Make the answer honest until then.** `cancel` should only claim a running job when that job's kind
   honours the flag; otherwise it should say so — `409` with `running job of kind 'data-quality' has no
   stop checkpoint` beats a `200` that is not true. This half is worth landing even after half 1,
   because the next long job added will not have a checkpoint on day one either.

## Acceptance

- Cancelling a running `data-quality` job actually ends it, within one query, with a run-log line
  saying it was cancelled.
- A test that asserts the flag is READ, not just set: spawn the job, cancel, assert it concluded early.
- Cancelling a running job of a kind with no checkpoint returns a refusal naming the kind, and a test
  pins that the two answers are different.


---

## Fixed (2026-08-20), both halves

**Half 1 — the checkpoint.** `run_data_quality` takes the job id and checks the flag **between
queries**. That is the right place: each query is an independent read holding no transaction, so
stopping there costs nothing and the worst case is one query's wait rather than a whole window's.

A stopped run **stores nothing**:

    data quality: CANCELLED after 36 of 417 measurement(s) in 742s — nothing stored,
    the previous report stands

Storing the partial would be worse than not running at all — half the windows render as a report whose
numbers look like a whole corpus. Same reasoning as the empty-layer guard already in that function.

**Half 2 — the honest refusal.** `cancel` now returns four distinct answers instead of a bool, and the
admin route maps them:

    200 {"state":"dropped"}    queued, and gone
    200 {"state":"stopping"}   running, and its kind reads the flag
    409                        running as a kind with NO stop checkpoint — names the kind
    404                        no such job

`STOPPABLE_KINDS` is the contract: `reparse` (issue 247) and `data-quality`. Anything else is refused
**by default**, so the next long job added is refused rather than silently ignored — which is the
failure mode this issue is, one job later.

A refused cancel leaves no flag set, which the test pins: a stale flag would stop the NEXT job to take
that id.

## Tests

`cancelling_a_kind_with_no_checkpoint_is_refused_rather_than_promised` — a running `project` is refused
and leaves no flag, a running `data-quality` is accepted and sets one, and `STOPPABLE_KINDS` is asserted
literally so adding a kind to it without a checkpoint fails here.

`docs/operations.md` lists all four answers beside the cancel examples, including that a cancelled
data-quality run stores nothing.

## Still to do

The prod acceptance: cancel a real `data-quality` run after the deploy and watch it end within one
query. The run that exposed this could not be stopped and had to be left going.
