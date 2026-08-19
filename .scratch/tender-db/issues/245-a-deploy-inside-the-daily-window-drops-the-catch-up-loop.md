# 245 — a deploy inside the morning window silently drops the daily catch-up

Status: needs-triage — noticed 2026-08-19 while timing a deploy around the 09:35 tick
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
