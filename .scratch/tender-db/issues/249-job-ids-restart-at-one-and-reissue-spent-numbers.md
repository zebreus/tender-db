# 249 — the Supervisor's job ids restart at 1 on a drained restart and reissue numbers the log has spent

Status: FIXED 2026-08-19 (owner) — `job_log.job_id` recorded, recovery seeds from its high-water
mark; falsified against the unfixed code. Deployed in `08ca548`; the prod acceptance read (one id
series in the panel) is still to do.
Kind: operational legibility defect (job identity), found by observation during a routine check
Blocked by: —
Relates to: 16 (the run log), 21 (the durable queue and the id counter), 65 (the progress record
that made the queue readable)

## What

Routine `/admin/jobs` read during the 2026-08-19 morning check:

    CURRENT 7 reparse reparse text after fetch 312 (first 1 package(s))
    QUEUED 8:project, 9:reparse, …, 42:project
      921 reparse ok | re-parsed 12385 notices across 1 packages …
      920 project  ok | 14551 notices → 13945 tenders …
      919 reparse ok | re-parsed 14551 notices across 1 packages …

A job numbered **7** is running while the three runs that finished immediately before it are
numbered **919, 920, 921**. Both numbers are called "the job id" by the same endpoint.

## Why

They are two independent id namespaces, and nothing recorded the correspondence.

- `job_log.id` is `INTEGER PRIMARY KEY AUTOINCREMENT` — the log's own append counter. 921 rows in,
  921.
- `job_queue.id` is app-assigned from `Supervisor::next_id`, an `AtomicU64` **initialised to 1** in
  memory. `recover()` advances it past every id it finds in the *pending* queue.

The pending queue is emptied as jobs finish (`remove_job` on conclusion — issue 21's design, and
correct). So a restart that happens with **no work outstanding** recovers `max_id = 0`, leaves the
counter at 1, and hands 1, 2, 3 … to new jobs — numbers that already name entirely different runs
in the log. This build restarted on a drained queue (the `e50c3e5` deploy), which is why the
campaign's packages are numbered from 1 while the log is in the 900s.

Nothing is lost or mis-run: the queue is keyed consistently within itself, `cancel(id)` and
`DELETE /admin/jobs/{id}` both take the live queue id, and the catch-up scan and `/health`
freshness match log rows by `kind` and time, never by id. The damage is to identity —

- a finished run cannot be correlated with the queue row it ran under, in either direction;
- "job 7" is ambiguous across restarts, so operator notes, issue write-ups and ledger entries that
  name a job id (this board has many) can point at two different runs;
- the ids visibly go backwards, which reads as data loss to anyone who has not read `jobs.rs`.

## Fix

`job_log` gains a nullable `job_id` column carrying the Supervisor's id for the run, and
`recover()` takes `MAX(job_id)` as a **second floor** for the counter alongside the pending
queue's max. That makes ids unique and monotonic for the life of the database: every job that
concludes leaves its number in the log, so the floor survives a drained restart.

Pre-existing rows answer NULL and contribute no floor, which is the honest reading — they were
numbered under the old scheme. The migration is therefore additive with no id collision anywhere:
the counter never jumps backwards, and never over a number that is already spent.

## Acceptance

- `migration_adds_the_job_log_job_id_column` (store): a pre-column `job_log` opens, the old row
  reads `job_id = None`, a new run records its Supervisor id, and the two ids are shown to be
  different numbers.
- `a_drained_restart_does_not_reissue_a_spent_job_id` (app): a job runs, concludes, the queue
  drains, a fresh Supervisor recovers over the same database — and the next id is above the spent
  one. **Falsified**: with the log floor removed the assertion fails (it reissues id 1).
- On prod after deploy: the ids in the recent-runs lines and the ids in `current`/`queued` come
  from one series, and a restart on a drained queue does not send them back to 1.
