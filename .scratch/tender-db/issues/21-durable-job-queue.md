# 21 — Durable supervisor job queue

Status: resolved

Resolution (2026-07-21, team lead): production-verified three times in
one afternoon. The b978c96 restart recovered all 6 queued jobs
automatically ("supervisor: recovered 6 pending job(s) from the durable
queue"), job order and the running job intact, zero manual re-enqueue —
retiring the manual re-enqueue dance that every earlier restart
required. Two further restarts (deploys #4/#5) recovered equally
cleanly, combined with the issue-32 cursor for cheap resumes. ADR-0007
records the design incl. the Spec serde compat contract.

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

## Fix (2026-07-21)

A durable `job_queue` table (store/jobs.rs) mirrors the in-memory queue:
one row per outstanding job — `id` (the supervisor's monotonic id), `kind`,
`params`, and an opaque serialized `spec` the store round-trips verbatim.
Store methods `enqueue_job`/`remove_job` (writes, on the writer connection)
and `pending_jobs` (reader pool, ordered by id). `QueuedJobRow` re-exported.

Supervisor (app/supervisor.rs):
- `Spec` gained `Serialize`/`Deserialize` (and its two `&'static str` Fetch
  fields became `String`) so a job round-trips through the DB. `Job.kind` is
  owned for the same reason.
- `push` persists the job (`enqueue_job`) *before* enqueuing it in memory, so
  a crash between the two is recovered from the DB. Best-effort like the run
  log — a failed persist still runs this session, it just won't survive a
  restart. `push`/`enqueue_request`/`enqueue_backfill`/`enqueue_daily`/`cancel`
  are now async.
- `execute` drops the durable row only *after* the job concludes (ok or
  error). A job killed mid-run never reaches that line, so its row survives.
- `cancel` drops the durable row too, so a cancellation survives a restart.
- New `recover()` runs in `init` **before** the worker or scheduler start
  (`init` is now async over a tokio `OnceCell`). It rebuilds the in-memory
  queue from `pending_jobs` (oldest id first, so the interrupted running job —
  the lowest surviving id — lands at the front and re-runs from the top),
  advances `next_id` past every recovered id, and drops any row whose spec
  this build can't parse. main.rs awaits `init`; admin.rs awaits enqueue/cancel.

At-least-once + idempotency, no exactly-once machinery: a re-run job
fast-forwards via the processor's identity dedup. The daily scheduler runs
after recovery and only sleeps-then-enqueues the next tick, so recovery never
races it; the sole overlap (a crash at exactly 09:35 with today's daily
already persisted) re-enqueues an already-idempotent batch — harmless.

Tests (all green):
- store::jobs::job_queue_persists_and_removes — table round-trip + ordered +
  single-row removal (`cargo test -p store`).
- supervisor::recovers_the_queue_across_a_restart — a fresh Supervisor over the
  same DB rebuilds the same pending jobs in order; next_id advances.
- supervisor::an_interrupted_running_job_is_recovered_at_the_front — the
  kill-mid-job-1 acceptance in miniature (pop without completing → row survives
  → comes back at the front ahead of the queued job).
- supervisor::a_cancelled_job_does_not_come_back.
- Full `tender-db --features server` suite (30 lib + admin end-to-end +
  accounts/api/sql/webhooks) + `store` green; clippy clean.

Needs verification: the real kill -9 mid-job on the box (restart re-enqueues +
fast-forwards), which unblocks deploying during a long job.

## Note — multi-agent file collision (2026-07-21)

Issue 23 (backup) was concurrently editing the same three files
(supervisor.rs, store/jobs.rs, store/lib.rs) with an uncommitted `Spec::Snapshot`
queue-job + `store::backup` module. Per the lead, 21 lands first (its durable
queue is the foundation the snapshot job rides in); 23 backed its hunks out of
those files and rebases its Snapshot integration onto this commit. Its own new
modules (snapshot.rs, backup.rs) were untouched. Verified this commit is
snapshot/backup-free and builds green in isolation (a throwaway worktree at
HEAD + this change only), since 23's other work is still uncommitted in the
shared tree.

## 2026-07-21 15:11 — PRODUCTION ACCEPTANCE: queue self-recovered on b0a5cdb (run-driver)

First real prod restart with the durable queue. Deploy b0a5cdb restarted the
service at 15:09:37 UTC; **no manual re-enqueue was done**. On startup the
persisted jobs recovered by themselves, intact and in order:

- current: job 1 `process ted monthly (all)`, at front, running
- queued: 2 `ted daily`, 3 `doe monthly`, 4 `doe daily`, 5 `project`,
  6 `snapshot`

All six survived the restart (incl. the on-demand snapshot at position 6).
Job 1's row predates the resume-cursor column so it re-walks from 1993 once more
(expected, one-time). **Durable-queue production acceptance MET** — restarts now
self-recover; the manual re-enqueue dance (three times today) is retired.
