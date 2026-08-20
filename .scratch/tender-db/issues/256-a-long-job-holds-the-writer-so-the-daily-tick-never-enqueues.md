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


## Part 2, narrowed by the re-run (2026-08-20 11:50)

Job 288 — the same fold, re-enqueued after the restart — is on the **incremental** path, not the full
rebuild one. The evidence is the durable phase record: the full path feeds it (`/admin/jobs` shows
planning / pre-pass / folding) and the incremental path deliberately sets none, and `admin.sh queue`
reads `CURRENT 288 project rebuild=false | - -/-`. So the `phase 1: n/14,289,308 notices planned`
heartbeats are `project_incremental`'s own, and the incremental delta really is corpus-sized.

Measured on 288's plan phase:

    11:00:20  chunk 1-2      10,000 notices/chunk, both inside one second
    11:21:44  7,500,000 / 14,289,308
    11:42:36  11,000,000 / 14,289,308
    11:48:38  11,420,000 — ~10 s per 10,000-notice chunk

So it starts instant and settles at ~2,800 notices/s, ~85 minutes for the corpus. Job 283's plan took
91 minutes for the same walk — the same order, which **kills the WAL-growth explanation I floated
before measuring it**: 288 started with a fresh WAL after a restart and still degraded the same way.
The degradation is within the run, as the plan tables fill.

What drives a corpus-sized delta: every `reparse` reports `stamped 2601443 tender(s) epoch-stale` —
the same 2,601,443 each time, whether it re-parsed one package or twenty. That stamp is what the next
fold must consider.

**The open question, stated precisely so the next unit does not start from a guess:** jobs 277, 279
and 281 were folds that FOLLOWED exactly such a stamp and finished in 80-100 s, folding ~30,000
notices each. Jobs 283 and 288 followed the same kind of stamp and walk all 14.3M. Whatever
distinguishes those two cases is the whole of part 2 — the candidates are the size of the re-parsed
set (32,920 notices vs 524,774), the incremental scoping's fallback rules (issue 58 v2's closure walk
logs "its own decisions… or a named fallback"), and the watermark's state after an interrupted fold.
The journal's own scoping line for 288 is the first thing to read when it appears.


## Part 2, ANSWERED by the system's own log (2026-08-20 11:52)

The open question above is answered, and not by me — job 288 printed it at start-up and I had not
looked in the right three minutes of journal:

    10:59:31  [project] incremental: 497672 changed notices
    11:00:03  [project] incremental stage pass-1 identity
              (3408 new keyed keys, 493745 legacy notices, 531036 seed keys): 31.8s
    11:00:08  [project] INCREMENTAL → FULL fallback: legacy closure exceeds cap
              (611530 notices > 500000) (issue 58 v2); re-projecting the whole corpus

So the discriminator is a **cap, by design**: issue 58 v2's incremental projection computes the legacy
closure of the changed set and falls back to a full-corpus re-projection when that closure exceeds
500,000 notices. One re-parsed package (32,920 notices) stays far under it and folds in 80–100 s.
Twenty (524,774 changed → 611,530 closure) crosses it, and the fold becomes a whole-corpus rebuild:
an 85–91 minute plan phase, then the group steps, then the ADR-0011 previous-notice pass where both
attempts stopped.

Two corrections to what I wrote earlier today, both from guessing ahead of the evidence:

- **"The fold's cost is dominated by corpus-wide steps that do not care how few packages changed"**
  (issue 244, an hour ago) — wrong. It cares a great deal: below the cap the fold is incremental and
  cheap, above it the whole corpus is re-projected. The number that matters is the closure size, not
  the package count.
- **"WAL growth explains the slow plan"** — already retracted above; the fresh-WAL re-run degraded
  identically.

### What this changes operationally

The text-era sweep must batch by **changed notices**, not by packages: ~12–15 packages of ~26 k
notices keeps the closure under 500,000 and the fold incremental. Twenty was over. That is a campaign
rule, not a defect — the cap exists because a closure that large is genuinely cheaper to re-project
whole (issue 58 v2's own reasoning), assuming the whole-corpus path terminates, which is now the one
thing in doubt.

### What remains part 2

The full path's ADR-0011 previous-notice pass, which did not finish in 2 h 05 m on job 283 and is
about to be attempted again by 288 at ~12:25. If it stalls a second time, the reproduction is
confirmed and the unit is to measure `plan_prev_edge`'s size and the join's plan on a full-corpus
plan — with the fold's phase record extended to cover the group steps (issue 65), so that an operator
can see which step is running instead of inferring it from a missing log line.


## Reproduced, and instrumented (2026-08-20 12:52)

Job 288 took the full path (the cap fallback above), finished its plan in **5,342.5 s** for
14,289,308 notices and 47,233,426 mentions — within 2 % of job 283's 5,453.9 s — then:

    12:29:34  group step keyed/island: 23.2s
    12:29:41  group step union-load: 6.6s (11,007,709 nodes)
    12:31:28  group step legacy-update: 107.5s (11,003,671 legacy)
    12:51:57  …20 minutes of silence, 78.8 % CPU, RSS 1.18 GB and flat

Same step, same silence, second run. Two independent attempts is a reproduction.

One difference worth recording because it rules something out: 288 logged
`WAL after Phase-1 (build_plan): 0 MB` where 283 logged 797 MB. The WAL state going into the step is
therefore not what makes it slow.

**What landed now:** the step counts its input and reports progress. `SELECT COUNT(*) FROM
plan_prev_edge` before the join, printed as `group step previous-notice: N edge(s) in the plan`, and a
heartbeat every 50,000 rows read. That is the one number nobody could get from outside the process —
an empty edge set and a grinding join look identical when the only line prints on completion. A normal
run (563 resolvable references when ADR-0011 was measured) prints the count and nothing else.

It cannot help the run in flight: deploying restarts the service and this fold is 1 h 52 m old. So the
plan is a deadline — if 288 has not passed the step by ~13:50 (80 minutes in it, against 283's 2 h 05 m),
it goes the way of 283: drop, deploy the instrumentation, re-run, and read the edge count when the step
starts.


## The leading hypothesis, and the change that tests it

The step's join is:

    FROM plan_prev_edge e
    JOIN notices n     ON n.source = e.a_source AND n.publication_id = e.b_publication_id
    JOIN plan_notice a ON a.notice_id = e.a_notice_id
    JOIN plan_notice b ON b.notice_id = n.id

Every one of those is an index seek IF the planner drives from `plan_prev_edge`, which is tiny — 563
resolvable references corpus-wide when ADR-0011 landed. Driving from `plan_notice` instead is a walk
of 14.3 M rows with three probes each.

And the planner has nothing to choose with. turso keeps no row statistics unless asked, and this same
function already runs `ANALYZE plan_notice` for that exact reason — with the comment *"its young
planner has mis-planned at scale"* — but it runs it **after** this step, beside the fold index. So the
one join in the grouping phase that most needs to know which table is small is the one that runs
before any stats exist.

So: `ANALYZE plan_prev_edge` and `ANALYZE plan_notice` now run BEFORE the join, timed and non-fatal
like the later one. Both are 0.0 s on fixture-scale plans; their cost at 14.3 M rows is now logged
rather than assumed.

**This is a hypothesis, and it is labelled as one in the code.** The verification is whether the step
completes on the next full re-projection — the same run whose new heartbeats will finally say how many
edges there were. If it still stalls with stats in place, the next move is to stop guessing at the
planner and restructure the query (materialise the edge set into a temp table with its own index, or
resolve the `notices` lookup in a separate pass), not to add another hint.

## Part 2, first instrumented run — 2026-08-20, job 288

The instrumentation deployed as `715f454` reached the step at 15:23:18 box time and printed the
number the issue was waiting for:

    [project] group step keyed/island: 24.0s
    [project] group step union-load: 6.7s (11007709 nodes)
    [project] group step legacy-update: 107.8s (11003671 legacy)
    [project] group step analyze plan_prev_edge: 0.0s
    [project] group step analyze plan_notice: 0.3s
    [project] group step previous-notice: 245955 edge(s) in the plan

**245,955 edges.** Every step before it is minutes; this one has been running 10+ minutes with no
completion line, and `top` shows the server on **one core at ~91 %, load average 1.00** — single
threaded and CPU-bound, not blocked on I/O and not deadlocked. That is the shape of a nested-loop
join, not of work proportional to 245,955 lookups.

**The ANALYZE hypothesis is falsified, or at least did not fire.** `ANALYZE plan_prev_edge` returned
in **0.0s** on a 245,955-row table. Whatever it did, it did not collect statistics worth the name, and
the step it was meant to speed up is still slow. I recorded that pre-join ANALYZE as a hypothesis when
I added it; it should now be read as tried and not sufficient, and it stays only because it is free.

**Correction to my own instrumentation, before anyone reads too much into the silence.** The heartbeat
counts rows the query RETURNS, not edges it scans:

    while let Some(row) = rows.next().await? {
        read += 1;
        if read % PREV_EDGE_HEARTBEAT == 0 { ... }

and the query filters hard (`b.published_at < a.published_at AND a.group_key <> b.group_key`). If
fewer than 50,000 edges survive that filter, **no heartbeat ever fires even on a perfectly healthy
run**. So "no heartbeat in 10 minutes" is not evidence of a stall, and I nearly read it as such. What
IS evidence is the pinned core with no completion line. The heartbeat should be moved to a wall-clock
interval rather than a row count — filed as the next slice.

## The suspect, stated as a hypothesis with a test

The join's inner side is:

    JOIN notices n ON n.source = e.a_source AND n.publication_id = e.b_publication_id

`notices` carries `UNIQUE(source, publication_id, content_hash)` — so the predicate is a **prefix** of
an existing unique index, which any competent planner would use. There is no two-column index that
matches it exactly. turso's planner already has a documented history here (239: no predicate pushdown
into views; 248: DELETE ignoring a composite-PK index), and "does not use a prefix of a composite
UNIQUE" belongs to the same family. If it is instead scanning `notices` (14.3M rows) per edge, the
observed single-core hour is arithmetic, not mystery.

**This is a hypothesis, not a finding, and it must not be shipped as an explanation.** The two
retractions on this board — "the fold's cost is insensitive to batch size" and "WAL growth explains
the slow plan" — were both explanations that sounded right and were never tested. The test here:
reproduce the schema and both plan tables locally at representative row counts and read
`EXPLAIN QUERY PLAN` for this exact statement. It cannot be read against prod: the plan tables are
temp tables private to the fold's own connection, invisible to `/v1/sql`. If the plan confirms a scan,
the fix is a two-column `notices(source, publication_id)` index, which is cheap and independently
defensible.

The completion line for this step, when job 288 reaches it, gives the first real elapsed time for
245,955 edges. That number belongs here when it lands.

## The index hypothesis is WRONG — measured, not argued (2026-08-20)

I wrote the two-column-index suspicion above as a hypothesis with a test attached, and then ran the
test. It refutes the hypothesis. `EXPLAIN QUERY PLAN` on the exact statement the fold runs:

    SCAN plan_prev_edge AS e
    SEARCH a USING INTEGER PRIMARY KEY (rowid=?)
    SEARCH n USING INDEX sqlite_autoindex_notices_1 (source=? AND publication_id=?)
    SEARCH b USING INTEGER PRIMARY KEY (rowid=?)

The plan is optimal and `clear_plan_on`'s comment was right all along: the lookup **does** seek
`notices` by the leading columns of its `UNIQUE(source, publication_id, content_hash)` index, and both
`plan_notice` lookups are rowid seeks. The driving table is the small one. There is no scan to remove
and no index to add. (Caveat stated plainly: the plan is read on an empty DB, where this engine has no
row statistics — but nothing here depends on statistics. It is an equality predicate on the exact
prefix of a unique index, and the syntactic join order already puts the small table outermost, which is
what a statistics-free planner follows.)

That is three hypotheses on this board now retired by measurement rather than by argument — batch-size
sensitivity, WAL growth, and this. The pattern is consistent enough to be worth naming: on this engine
my guesses about *why* a step is slow have been wrong every time, and the thing that has never been
wrong is a timer.

The plan assertion is kept as `the_previous_notice_join_seeks_notices_rather_than_scanning_it`
(crates/store/src/lib.rs), reading `PREV_EDGE_JOIN_SQL` — the statement is now a named const used by
both the fold and the test, so the plan can never be checked against a copy that has drifted. It costs
nothing and it means the next person to suspect this join can see in one test run that it is not the
problem.

## So: timers, not theories

The step is one `eprintln!` at the start and one at the end, with four quite different things in
between — the streaming read, the union-find over the involved keys, the `plan_group_merge` writes, and
the relabel `UPDATE` batched across all ~11M `plan_notice` rows with a TRUNCATE checkpoint per batch.
Any of the four could be the hour and the log cannot distinguish them. Each now reports its own
elapsed time, and the two long-running ones (the read, the relabel) heartbeat on a 15-second clock:

    [project] group step previous-notice: 245955 edge(s) in the plan
    [project] group step previous-notice read: …s (… of 245955 edge(s) matched, … key(s) involved)
    [project] group step previous-notice union: …s (… key(s) to relabel)
    [project] group step previous-notice merge-write: …s
    [project] group step previous-notice relabel: batch N, notice_id lo..hi of max, …s
    [project] group step previous-notice relabel: …s (N batch(es))
    [project] group step previous-notice: …s (… edge(s) resolved, … key(s) merged)

`PREV_EDGE_HEARTBEAT` is now a `Duration`, not a row count, for the reason recorded above: a row-count
interval is silent on a run that matches fewer rows than the interval, and silence that means "fine"
is indistinguishable from silence that means "hung".

**Sequencing.** Job 288 is deliberately being left to finish rather than restarted onto this build:
its completion line is the first real elapsed time for 245,955 edges, which is the number this part of
the issue has been waiting for, and deploying would throw away ~1.5 h of re-projection to learn it a
different way. Deploy after it drains; the breakdown lands on the next full re-projection.

## Job 288, one hour into the step — facts only (2026-08-20 16:21)

    15:23:18  group step previous-notice: 245955 edge(s) in the plan
    16:21:00  (no completion line; 57m 42s elapsed in this step)
    top:      one core at 100.0 %, load 1.00, %wa 0.0
    RSS:      905 MB at 15:32  ->  2.1 GB at 16:21

CPU-bound, single-threaded, not waiting on I/O, and growing memory the whole time. Every neighbouring
step in this phase is between 0.3 s and 108 s.

The memory growth suggested a fan-out: `notices` is `UNIQUE(source, publication_id, content_hash)`,
so in principle one publication can hold SEVERAL notice rows, and the join would then emit one output
row per stored variant per edge — many more rows than the 245,955 the plan counts, which would
explain both the time and the RSS. **Refuted, in one bounded query:**

    SELECT COUNT(*), COUNT(DISTINCT source || '|' || publication_id)
      FROM notices WHERE id BETWEEN 19900000 AND 19950000
    -> 50001, 50001

One publication, one notice row. No fan-out. That is the fourth explanation for this step retired by
measurement (batch size, WAL growth, the missing index, now fan-out), and the first three each took
longer to retire than this one did, because this one was stated as a testable claim before it was
believed.

What remains unexplained: 245,955 index seeks over an hour is ~14 ms per edge, which is disk-seek
scale — but `%wa` is 0.0 and the core is pinned, so it is not disk. I do not know what it is, and I am
deliberately not proposing a fifth mechanism here. The four phase timers committed in `a92207d` split
this step into read / union / merge-write / relabel; whichever of them holds the hour will say so on
the next full re-projection, and the 15-second heartbeats mean it will say so while it is happening
rather than afterwards.

RSS growth is consistent with either the read accumulating `edges`/`rank` or turso's page cache
growing as the seeks walk a 14.3M-row index; the read timer distinguishes those too, since a slow
read prints its own elapsed time before the union-find ever starts.

## The step ran 3 h 27 m and never finished — and there was no way to stop it (2026-08-20)

Job 288's previous-notice step started at 15:23:18 and was still running at 18:50:26 — **3 h 27 m**,
one core pinned at 100 %, RSS flat at 2.2 GB the whole time (so it was not accumulating; it was
grinding). That is the third occurrence, and the first with a number attached. The step never printed
its completion line, so the elapsed time this part of the issue was waiting for is a lower bound, not
a measurement.

**The operator cannot stop it.** This is the part worth fixing. The documented route refuses:

    POST /admin/jobs/288/cancel
    {"error":{"message":"job 288 is running as kind \"project\", which has no stop checkpoint","status":409}}

`STOPPABLE_KINDS = ["reparse", "data-quality"]`, so a `project` job — the longest job the system runs,
and the one this whole issue is about holding the writer — is the one kind that cannot be interrupted.
The supervisor is honest about it (issue 252's message names the reason precisely), but honesty is not
a lever. What is left is a service restart, and a restart RE-RUNS the job from the top: its durable row
survives recovery, so the 3.5 h is repaid, not saved.

The only clean exit is the `TENDER_DROP_JOBS` escape hatch: set it, restart (the job is dropped at
recovery rather than resumed), clear it. That works — it is what unblocked tonight's deploy — but it
means the answer to "this fold is stuck and the daily tick is due in twelve hours" is *edit a systemd
drop-in and bounce the service*, which is not an operational procedure so much as a workaround with a
manual.

**Proposed, not yet built:** give the projection a stop checkpoint. It already checkpoints between
`GROUP_KEY_UPDATE_BATCH` ranges and between plan chunks — the loop boundaries where a stop flag would
be read exist and are frequent. `phase 1` reads the flag per chunk, the group step per batch, `phase 2`
per fold batch. That turns `project` into a stoppable kind and makes the documented cancel route true
for the job that most needs it. It does NOT need resumability to be useful: dropping out cleanly at a
checkpoint and leaving the plan for the next run is strictly better than a restart that redoes
everything.

### An ops trap worth knowing about, found the hard way

Setting `TENDER_DROP_JOBS=288` in a NEW drop-in (`drop-288.conf`) appeared to work — `systemctl show`
listed the variable, and `/proc/<pid>/environ` confirmed the process had it — but the value was
**empty**, and the job was recovered rather than dropped:

    TENDER_DROP_JOBS=$          # cat -A: the value is the empty string

`/etc/systemd/system/tender-db.service.d/` already held `dropjobs.conf` from an earlier use of the same
hatch, left in place with the value reset to empty. Drop-ins are applied in **alphanumeric order**, and
`drop-288.conf` sorts before `dropjobs.conf` (`-` = 0x2D < `j`), so the older file silently won. The
symptom is the worst kind: the variable is present, so every check says "it is set", and it is set to
nothing.

Fix applied: write the value into the file that wins and delete the redundant one. **Rule for next
time — there is exactly one `dropjobs.conf`; set its value, use it, reset it to empty. Never add a
second drop-in for the same variable.** A leftover that sets an empty value is indistinguishable from
an unset variable at every observation point except `cat -A` on the resolved environment.

## The phase timers' first run — and the hours are GONE (2026-08-20, job 289)

The same statement, the same 245,955-edge plan, the same 14.3M-notice corpus, on the same box:

    21:03:00  group step keyed/island: 28.5s
    21:03:06  group step union-load: 6.8s (11,007,709 nodes)
    21:04:54  group step legacy-update: 107.1s (11,003,671 legacy)
    21:04:54  group step analyze plan_prev_edge: 0.1s
    21:04:56  group step analyze plan_notice: 2.2s
    21:04:56  group step previous-notice: 245955 edge(s) in the plan
    21:05:04  group step previous-notice read: 8.0s (29,646 of 245,955 edge(s) matched, 42,907 key(s) involved)
    21:05:04  group step previous-notice union: 0.0s (24,528 key(s) to relabel)
    21:05:04  group step previous-notice merge-write: 0.2s

**The read took 8.0 seconds.** Yesterday the step containing the identical join pinned one core for
3 h 27 m without completing, twice before that for 2+ hours. The plan is the same size to the edge.

**What I can say and what I cannot.** The completion also answers what the filter keeps: 29,646
survivors of 245,955 references, 24,528 keys relabelled — the old row-count heartbeat (50,000) could
indeed never have fired, as recorded above.

The honest position on WHY it is fast now: unattributed. Three observable differences between the runs,
none of which I can promote to a cause without an experiment this is not worth:

1. **The ANALYZE did real work this time.** `analyze plan_notice` took 2.2 s against 0.3 s on the slow
   run — on an 11M-row table, 0.3 s cannot have collected meaningful statistics, 2.2 s plausibly did.
   If turso's stats made the difference, the "tried and not sufficient" verdict I recorded for the
   pre-join ANALYZE was wrong in the most instructive way: the mitigation was right and its FIRST
   EXECUTION was the thing that failed, silently.
2. **A process restart sits between them** (the TENDER_DROP_JOBS bounce) — a fresh page cache, fresh
   connection state, whatever the slow run's connection had accumulated over its preceding 90-minute
   phase 1, all gone.
3. **The build differs** (`715f454` → `45c0a14`), though the join text is byte-identical (now a
   shared const) and nothing else in the step's data path changed semantically.

**CORRECTED an hour later, by the relabel heartbeat this same run carried:** there was no shift and
no mystery. The READ was always seconds — its 8.0 s here is the first time it was measured alone.
The hours were always the RELABEL, one statement further down, which had no instrumentation and sat
after the only log line, so every grind was mis-attributed to the join. Batch 12's heartbeat put it
at ~256 s per 200 k batch × 143 batches ≈ 6 h; the filter `group_key IN (SELECT from_key FROM
plan_group_merge)` plans as a LIST SUBQUERY that re-SCANS the 24,528-row merge table inside the row
loop (~5 billion comparisons per batch, EXPLAIN-verified). Benched at matched scale: the IN form
~1,475 s per 200 k batch, the indexed-EXISTS form 26.3 s — 56×, and prod's release build extrapolates
to seconds. Fixed in `b416894`. The three refuted theories (batch-size, WAL, the join's index) were
all theories about the WRONG STATEMENT. The step is now fully
instrumented per phase, so the next slow occurrence will name its phase and its progress on a 15-second
clock instead of demanding an afternoon of forensics. That is the durable win; the mystery is recorded,
not solved, and the stop-checkpoint proposal above stands regardless — an operator still cannot cancel
a `project` job that DOES grind.

## The stop checkpoint is built (2026-08-20, same evening)

`project` is now a stoppable kind — the proposal above, implemented the same day the two
TENDER_DROP_JOBS restarts made its absence vivid:

- **Checkpoints:** between Phase-1 plan chunks, before grouping, and between Phase-2 fold batches
  (both the bucketed and the ParsedFold arm, full and incremental paths, with the stop forwarded
  into the incremental's whole-corpus fallback so a cancel reaches whichever path the delta routed
  to). The same clean points the WAL checkpoints already use.
- **Semantics: a stop costs a redo, never correctness.** Everything committed before the stop stays
  committed and marked `projected`; a stopped Phase-1 does NOT attest the legacy-adjacency watermark
  (an incomplete walk attesting completeness would be issue 105's marked-without-a-row lie); a
  stopped fold skips retirement, plan teardown and the index builds (retiring legacy keys against a
  partial fold would remove Tenders whose members simply had not folded yet); a stopped REBUILD keeps
  `rebuild_in_progress` + the plan, which is exactly the issue-60 salvage state.
- **The log row says so:** a cancelled run's counts line leads with `CANCELLED at a checkpoint —`,
  the same looks-complete-isn't rule the capped reparse follows (issue 244), and `Report::stopped`
  carries it programmatically.
- **Gates:** `a_stopped_projection_resumes_to_the_identical_layer` (ingest) — an immediate stop
  folds nothing, marks nothing, attests nothing; a later un-stopped run converges to the identical
  complete layer. The supervisor's stoppable-list tests now pin
  `["reparse", "data-quality", "project"]`, with `reindex` taking over as the refused example.

Deploys after the running fold drains. With this, the answer to "this fold is stuck" becomes
`POST /admin/jobs/<id>/cancel` — the documented route — and the TENDER_DROP_JOBS procedure above
demotes to the break-glass it should always have been.
