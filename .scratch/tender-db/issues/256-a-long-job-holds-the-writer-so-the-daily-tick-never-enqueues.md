# 256 — a long job holds the writer, so the daily tick silently never enqueues

Status: PART 1 DONE 2026-08-20 (the enqueue no longer parks on the writer — it gives up after 30 s
and queues the job in memory, loudly). PART 2 OPEN: the fold itself, which holds the writer for hours
and logs nothing for over an hour of it. Awaiting deploy — the very fold that proved the defect is
still running, and deploying would restart it from the top
Kind: operability defect, silent data loss window
Blocked by: —
Relates to: 245 (the missed-tick catch-up this defeats), 222 (the weekday catch-up window it also
eats), 21 (the durable queue), 92/95 (CPU-bound fold phases), 213 (health that claims what it does not
check)

## What happened

2026-08-20 was the first day in at least four with **no daily ingest at all**:

    v_fetches, newest rows
    499  doe  daily  2026-08-18   fetched 2026-08-19 07:36:06Z   (= 09:36 CEST)
    498  ted  daily  2026-00159   fetched 2026-08-19 07:35:02Z   (= 09:35 CEST)
    497  doe  daily  2026-08-17   fetched 2026-08-18 07:36:03Z
    496  ted  daily  2026-00158   fetched 2026-08-18 07:35:01Z

Aug 17, 18 and 19 all fetched within two seconds of the 09:35 Berlin tick. On Aug 20 the tick came
and went: `queued: []`, no `probe` or `process` job in the log, no scheduler line in the journal, and
**no panic** — the process had been up since 06:37:55 and a main-runtime periodic task (the presence
observer) ran at 09:30:17, five minutes before the tick.

## The mechanism, proven

A maintenance `project` job (283, the 20-package text-era sweep's fold) started 07:10:41 and was still
running at 09:55. While it ran:

    GET  /health          →  4 ms
    POST /v1/sql SELECT 1 →  8 ms
    POST /admin/jobs {"kind":"daily"}  →  TIMED OUT after 15 s

Reads are instant; **enqueuing is what blocks**. `Supervisor::enqueue` persists the durable row first
(`db.enqueue_job`), which takes the single writer connection — and a long job holds that writer for
its whole run. So:

1. the 09:35 scheduler tick woke, called `enqueue_daily`, and parked on the writer inside its first
   `push`, before writing any log line;
2. the queue therefore shows nothing — the in-memory push happens AFTER the persist, so a blocked
   persist means no visible job either;
3. an operator hitting the admin API to force it gets a timeout with no explanation.

The parked enqueues are not lost: when the fold ends, the writer frees, the tick's `push` completes
and today's daily runs — hours late. What IS lost is the timing guarantee the tick exists for, and on
a weekday the whole issue-222 catch-up window (`tick + CATCHUP_WINDOW_SECS`) can elapse while parked,
which is the case where a late TED package is missed until tomorrow.

## Why nothing noticed

`/health/deep` reported, at the same moment:

    "ingest_freshness": {"age_secs": null, "last_success_at": null, "ok": true, ...}

`ok: true` with `last_success_at: null`. "Never ingested" reads as healthy — the same
absent-is-not-zero trap as issue 213's health check and the report's UNMEASURED banners. A day with no
ingest is exactly what this check is for, and it is the check that says fine.

## The fix, in two parts

**Part 1, the enqueue — DONE.** `enqueue`'s own comment says the
persist is *"best-effort like the run log — a failed persist still runs this session, it just won't
survive a restart"*. It does not honour that under contention: it awaits the writer forever instead of
failing. Wrapping the persist in a timeout makes the stated tolerance real — the tick enqueues in
memory immediately, the jobs run when the worker frees, and only a process death in between loses
them. Loud log line when it happens, because a queue that diverges from its durable rows must not be
quiet about it.

**Part 2, the fold (bigger, deferred).** A job that holds the writer for 2.75 h and logs nothing for
65 minutes of it is the real problem; part 1 only stops it from taking the day's ingest with it. The
same run's phases, measured from the journal:

    plan (14,285,381 notices, 47,219,642 mentions)   5,453.9 s   91 min
    group step keyed/island                              17.7 s
    group step union-load (11,007,709 nodes)              6.4 s
    group step legacy-update (11,003,671 legacy)        104.6 s
    then: no log line for 65+ minutes, 86 % CPU, RSS 2.55 GB

Every `plan checkpoint` line in that run says `busy=true wal_frames=0 checkpointed=0 — WAL not fully
reclaimed`, and the WAL reached 797 MB — consistent with one transaction held open across the whole
job. Releasing the writer between chunks (or between group steps) would let the queue breathe;
issue 65's phase/progress record would let an operator tell working from stuck without resorting to
`ps -o pcpu`.

## Also worth recording: what a 20-package fold costs

For the issue-244 campaign's planning, since this is where the number came from: 20 packages
(524,774 notices re-parsed) cost a **91-minute plan phase alone**, against ~80 seconds for a
single-package fold. The plan phase walks the whole 14.3M-notice corpus regardless of how few packages
changed, so batching packages is dramatically cheaper per package than doing them one at a time — but
the batch blocks the writer for the whole time, which is what part 1 is for. The remaining 161
text-era packages at 20 per fold are ~8 more folds.


## Part 1 as landed

`Supervisor::persist_queued(id, kind, within, persist)` wraps the durable write in a
`tokio::time::timeout` and returns whether the row exists; `enqueue` queues the job in memory either
way, which is what the pre-existing comment already promised and did not deliver.

`PERSIST_TIMEOUT` is **30 s, not 5**: an ordinary `process`/`project` chunk holds the writer for
seconds at a time and a persist landing mid-chunk must still get its durable row. Nothing legitimate
holds it for half a minute — the case this exists for held it for hours.

The timeout is a PARAMETER rather than a constant read inside the function, for one reason: it makes
the give-up branch testable in 20 ms with a future that never completes
(`a_persist_that_cannot_get_the_writer_gives_up_rather_than_parking`). Without the timeout that test
hangs, which is exactly the production failure it pins. Verifying it against a genuinely held writer
would need a public "hold the writer" hook in the store for one test; the prod observation above — a
`POST /admin/jobs` timing out at 15 s while `/health` answered in 4 ms — is the evidence for that end,
and the next long job is the natural place to re-check it with `ops/admin.sh enqueue daily`.

What part 1 does NOT do: it does not make the day's ingest timely. The jobs still wait for the worker,
which is busy with the long job. It removes the SILENCE and the loss — the tick's jobs appear in the
queue, the log says the row was skipped, and the work runs the moment the worker frees.


## The step it was stuck in, and how today ended (2026-08-20, 10:53)

The fold never logged again after `group step legacy-update` at 08:44:32. The next step in
`canonical.rs` is the **ADR-0011 previous-notice pass** (issue 236), and its own timing line —
`group step previous-notice: …s (… edge(s) resolved, … key(s) merged)` — prints when that step
FINISHES. It never printed. So the job spent **2 h 05 m inside one step** and was still there when I
stopped it, at 89 % CPU, RSS flat at 2.55 GB for the last hour (so not accumulating rows), cursor
static since 08:40 (so not writing).

The step's driver is a four-way join built per run:

    FROM plan_prev_edge e
    JOIN notices n      ON n.source = e.a_source AND n.publication_id = e.b_publication_id
    JOIN plan_notice a  ON a.notice_id = e.a_notice_id
    JOIN plan_notice b  ON b.notice_id = n.id
    WHERE …

`notices` carries `UNIQUE(source, publication_id, content_hash)`, whose leading columns are exactly
that join's keys, so it CAN seek — on a plan holding 14.3 M notices it evidently did not, or the edge
set was far larger than the 563 resolvable references ADR-0011 measured. **Unproven either way**, and
worth proving before anyone optimises it: the honest statement today is that the step did not complete
in two hours on a full-corpus plan, not that I know why.

### What I did about it

The daily ingest had not run in 5 hours and the fold's own work was re-runnable, so the fold lost:

1. `TENDER_DROP_JOBS=283` into the systemd drop-in, `systemctl restart tender-db`;
2. recovery dropped the job (`supervisor: dropping recovered job 283 per TENDER_DROP_JOBS`) instead of
   re-running it;
3. issue 245's startup catch-up then did exactly its job — it saw the 09:35 tick had passed unserved
   and enqueued the daily, which began fetching within four seconds (`284 probe ok | probed 3
   issue(s), 1 new`);
4. the drop hatch was cleared again immediately, so the next restart drops nothing.

So today's notices are landing. The 524,774 re-parsed notices from job 282 are still stamped
epoch-stale, and the daily's own `project` (job 288) now inherits that fold — the same plan, the same
step. If it stalls the same way, that is the reproduction this issue needs and the next unit is to
measure the edge set and the join's plan rather than to guess at them.
