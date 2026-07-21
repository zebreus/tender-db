# 21 — Durable supervisor job queue

Status: ready-for-agent
Blocked by: 20 (same code area; avoid parallel edits)

The supervisor's job queue is in-memory only. Every service restart wipes
queued and running jobs — this has already cost us twice during issue 15
(the a9b0883 deploy killed job 1 mid-run plus 4 queued jobs; the fa873ec
deploy required a full re-enqueue). Worse, it makes every deploy during a
long-running job a coordination problem: right now issue 20's fix sits
undeployed because a restart would wipe the backfill queue.

Fix: persist queue state (queued jobs + the running job's identity) in the
DB so that on startup the supervisor re-enqueues what was pending and
restarts the interrupted job from the top (re-walks are idempotent via
identity dedup, so restart-from-top is correct and cheap — the a9b0883
incident fast-forwarded 19 packages in ~12s). No exactly-once machinery;
at-least-once + idempotency is the design.

Acceptance: enqueue N jobs, kill -9 the service mid-job-1, restart →
queue holds the same jobs, job 1 re-runs and fast-forwards; deploys no
longer require queue-state coordination.
