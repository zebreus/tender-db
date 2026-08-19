# 245 — a deploy inside the morning window silently drops the daily catch-up

Status: fix (3) DONE in code 2026-08-19 — startup catch-up landed; (1) deploy-time refusal still open
Kind: operational hazard, in-process scheduler
Blocked by: —
Relates to: 222 (the weekday catch-up this would drop), 240 (the outage that showed how invisible a
missed pipeline is), the `ingest_freshness` fix of 2026-08-19 (`probe`/`process` only), 97

## What

The daily pipeline is an in-process `tokio` task (`Supervisor::spawn_scheduler`): it sleeps until 09:35
Berlin, calls `enqueue_daily`, then — on weekdays — enters issue 222's catch-up loop, which re-probes
TED every `CATCHUP_POLL_SECS` until the day's package lands or the window closes.

The queued jobs are durable — they are rows in `job_queue`, so a restart re-runs them. **The catch-up
loop is not.** It lives only in that task, so a `systemctl restart` (i.e. every deploy) inside the
morning window ends it silently: no log line, no job record, no failed run. If TED's package was late
that morning — exactly the case issue 222 exists for — the day is simply missed until tomorrow's
walk-forward, and nothing says so.

I hit the near-miss version today: a deploy at 09:00 CEST, 35 minutes before the tick. Had it been
09:40 the catch-up would have been discarded while looking healthy.

## Why it is worth fixing rather than remembering

"Do not deploy between 09:35 and 10:35 Berlin" is a rule that has to be held in an operator's head,
and the operator here is usually an agent starting a fresh session with no memory of the clock. The
board is the memory, so the fix belongs in the code.

## Candidate fixes, cheapest first

1. **Make the deploy refuse (or warn loudly) inside the window.** `deploy.sh` already checks
   `/health`; it can ask `/admin/jobs` whether the morning catch-up is live and print a one-line
   refusal with an override flag. Cheap, no runtime change, and it puts the knowledge where the action
   is instead of in a doc.
2. **Persist the catch-up as a job rather than a loop.** A `probe (catch-up)` job that re-queues itself
   until the watermark advances or the window closes is durable across restarts by construction, and it
   shows up in the job log like everything else. More faithful, slightly more machinery.
3. **On startup, notice a missed tick.** If the scheduler starts and today's 09:35 has already passed
   with no successful `probe` since, run `enqueue_daily` immediately. This also covers a box that was
   down over the tick for any other reason, which today is silent.

(3) is the one that makes the system self-correcting rather than requiring care, and it composes with
(1). Worth doing (3) plus (1).

## Acceptance

- A restart inside the morning window either preserves the catch-up or re-establishes it on startup.
- A tick missed entirely (box down, deploy at the wrong minute) is visibly caught up rather than
  waiting for the next day, and the catch-up is legible in the job log.
- Verified by restarting the service inside a simulated window, not only in a test.


---

## Fix (3) landed 2026-08-19 — the startup catch-up

`Supervisor::catch_up_missed_tick` runs before the scheduler's loop. It asks whether today's 09:35
Berlin tick has been served, where served means either

- a **successful `probe` at or after the tick** — the first job `enqueue_daily` pushes, so its success
  is the tick's own footprint; or
- **a `probe` still in the queue** — after a restart the durable rows re-run on their own, and
  double-enqueuing a daily on top of that would just duplicate the work.

Anything else means the tick fired into a process that is gone, or never fired at all, so it runs
`enqueue_daily` immediately and says so in the log.

Keyed on `probe` deliberately, not on "any daily job": `project` is pushed by every maintenance refold
(the conflation that had `ingest_freshness` reporting fresh while nothing was arriving — fixed the same
morning), and `process` succeeds trivially when there is nothing to process. Only a probe means
"a source was actually asked".

The scan depth is 200 job runs rather than the dashboard's 20, because today's own log filled 20 rows
in 18.9 hours; a probe that falls out of the window would read as "never ran" and re-run a daily that
had already happened.

`berlin_tick_on` is factored out of `next_berlin_tick` — the "next tick" form cannot answer a question
about a tick in the past, which is the only question this asks.

### Still open

- **(1) a deploy-time guard.** The startup catch-up makes a badly-timed restart recoverable, but it
  does not make it free: a restart mid-fold still discards in-flight work and the catch-up re-does it.
  A `deploy.sh` check that refuses (with an override) while a fold is running or the morning window is
  live is still worth having.
- **Verification against a real restart.** The decision is unit-tested at every branch, and the quiet
  path was observed on prod (a deploy after a served tick logged nothing). The interesting case — a
  restart INSIDE the window, tick unserved, catch-up firing on prod — has not been exercised yet, and
  the honest way to see it is a deliberate restart while the tick's probe has not yet run.
