# 382 — a job in STOPPABLE_KINDS cannot actually be stopped while it sits inside one long whole-corpus query

Status: ready-for-agent (filed 2026-09-12 by the owner, from cancelling job 1313)
Kind: defect (operability) — the stop contract promises more than it can deliver, and the gap only
shows on the runs where stopping matters most
Blocked by: nothing

## What happened

Job 1313 (`data-quality`) reached its last whole-corpus query and stayed there. `POST
/admin/jobs/1313/cancel` answered **`{"cancelled":1313,"state":"stopping"}`** — the honest answer for
a stoppable kind — and the job kept running for **another 20+ minutes** on one pegged core, still
inside the same query.

`data-quality` IS in `STOPPABLE_KINDS`, and a contract test pins that list. Nothing lied. But the
stop flag is checked **between** work items, and a whole-corpus query is one work item that can run
for an hour. So the promise a reader takes from `state: "stopping"` — "it will stop shortly" — does
not hold, and the operator has no way to tell the two cases apart from the API.

## Why it matters more than it looks

`stopping` is the answer an operator gets precisely when something has gone wrong and they want the
box back. The remedy that actually works is a service restart, which is a much bigger hammer and is
not what the API's answer suggests is necessary. On 2026-09-12 that restart had to be dressed up as
a deploy to avoid being an unexplained bounce.

The three-answer design (issue 252) was built exactly so an operator could tell a queued drop from a
running stop from an honest refusal. This is a fourth state hiding inside `stopping`: **accepted,
but not reachable for an unbounded time.**

## What to do — smallest thing first

1. **Say which it is.** `stopping` could carry the phase, or a second field naming the checkpoint it
   is waiting for. An operator who reads "stopping — will check the flag after `whole-corpus
   unmapped_fields_per_profile` completes" knows immediately whether to wait or restart. This is a
   message change, not a mechanism change, and it is most of the value.
2. **A checkpoint between whole-corpus queries.** The phase already advances per query, so the flag
   can be read there. That bounds the wait to one query rather than the whole phase — better, and
   still unbounded if one query is the problem.
3. **Not a query timeout.** Tempting and wrong as a default: these queries are legitimately long, and
   a timeout that fires on a healthy slow run would make the weekly report silently partial, which is
   the failure mode issue 230 spent effort removing. If a bound is wanted it belongs on the ADMIN
   path (cancel with a deadline, then escalate), not on the query.

## Corroborating detail worth keeping

The runaway query was the per-profile unmapped-field arm, reverted in the same firing. Its cost is
issue 368's problem, not this one — but it is a good example of the shape that triggers this: a
single statement whose cost is unbounded in a way the job's own progress reporting cannot show,
because `done/total` counts QUERIES and one query was 30× the rest of the phase combined.
