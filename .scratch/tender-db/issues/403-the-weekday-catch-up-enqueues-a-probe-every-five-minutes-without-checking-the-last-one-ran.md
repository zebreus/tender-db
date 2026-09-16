# 403 — the weekday catch-up pushes a probe every 5 minutes for 3 hours without checking whether the previous one has RUN, so a busy queue collects up to 36 identical no-op jobs

Status: ready-for-agent — found 2026-09-16 08:50Z by the hourly audit (step 3), on a live queue holding **4 identical `ted daily (catch-up)` probes**. The mechanism is in the code and does not depend on the observation; the observation is what prompted reading it.
Kind: defect (operations — `Supervisor::spawn_scheduler`'s weekday catch-up loop, `crates/app/src/supervisor.rs`). Nothing is lost and no data is wrong: the cost is queue time taken from whatever long job is running, and a catch-up that cannot do the thing it is retrying for.
Relates to: 222 (built this loop — "keep re-probing on a short interval until it does (or the morning window closes)", correct when the queue is free and unexamined when it is not), 245 (`catch_up_missed_tick`, the STARTUP catch-up, which already does the check this loop is missing: `self.queue.lock()…any(|j| j.kind == "probe")`), 247 (the serialized queue and `push_front`, whose doc is the precedent that a job which makes the queue slow is worth treating specially), 252 (which job kinds read the stop flag)
Blocked by: nothing

## The mechanism

`spawn_scheduler`, the weekday arm:

```rust
while self.latest_ted_issue_now().await <= ted_before && store::now_unix() < deadline {
    self.push("probe", "ted daily (catch-up)".into(), Spec::ProbeTed { refetch: true }).await;
    caught_up = true;
    tokio::time::sleep(Duration::from_secs(Self::CATCHUP_POLL_SECS)).await;   // 300
}
```

The loop's exit condition is the TED watermark advancing. **The watermark can only advance when a
queued probe actually EXECUTES.** The loop never checks whether the probe it pushed five minutes ago
has run — only whether its effect has appeared. So when the queue is occupied by a long job, the loop
pushes on a fixed 5-minute cadence for the whole `CATCHUP_WINDOW_SECS` (3 hours): **up to 36 identical
jobs**, none of which can have any effect until the long job finishes.

The comment above it says "the retries are cheap no-op probes". That is true when they run promptly
and false when they queue: a no-op probe is cheap to RUN and not free to QUEUE, because the queue is
serialized and every one of them takes a turn ahead of the next real job.

The startup catch-up ten lines up already knows this:

```rust
let probe_queued = self.queue.lock().expect("queue lock").iter().any(|j| j.kind == "probe");
if !tick_needs_catch_up(tick, now, &runs, probe_queued) { return; }
```

So the check exists, is tested, and was simply not applied to the polling arm.

## Observed (2026-09-16 08:50Z on prod)

    /admin/jobs -> queued, grouped:
      4x probe        | ted daily (catch-up)
      2x project      | rebuild=false
      1x reveal-recheck, 1x probe ted, 1x process ted, 1x probe doe, 1x process doe,
      1x probe fts, 1x process fts, 1x fetch-rates, 1x reparse

The morning window opened at 09:35 Berlin (07:35Z) and the queue has been held by a DÖE re-parse
campaign and its folds. Four had accumulated 75 minutes in — with some having already drained, since
each probe that runs removes one.

The `2x project | rebuild=false` beside them is a second, smaller instance of the same class: one
projection from the daily schedule and one from a re-parse's follow-on, queued back to back over the
same change-set. Not this issue's, but worth naming, because "the queue holds two jobs that will do
the same work" has the same cause — enqueue decisions made without looking at the queue.

## Why it matters

Three things, none of them data loss:

1. **It steals queue turns from the long job it is blocked behind.** A campaign that already takes
   hours gets interleaved with dozens of no-op probes, each a network round trip and a registry read.
2. **It defeats its own purpose.** The point of issue 222's loop is to land a late TED package
   PROMPTLY. When the queue is blocked, pushing more copies cannot make that happen sooner; only the
   first one can ever matter.
3. **It is unbounded within the window, and the window is the only bound.** Nothing caps the count at
   the queue's ability to drain, so the worse the backlog the more copies are added to it — the shape
   that turns a slow morning into a slower one.

## Why this is ours, not the publisher's

TED published on its own schedule and the probe is a read. This is a scheduler asserting progress it
cannot observe.

## Repro

1. Occupy the queue with a job longer than 5 minutes on a weekday morning (any `reparse` chunk).
2. `/admin/jobs` → the `queued` array grows by one `probe | ted daily (catch-up)` every ~5 minutes
   until the 3-hour window closes or the queue drains.
3. `sed -n '9848,9862p' crates/app/src/supervisor.rs` → the loop, with no already-queued check.
4. `sed -n '9800,9804p'` → `catch_up_missed_tick`, which does have one.

## Done when

- The polling arm does not push a catch-up probe while one is already queued — the same check
  `catch_up_missed_tick` makes, applied to the same decision. The loop keeps polling (the watermark
  read is the thing that must stay on its cadence); only the PUSH is gated.
- A test drives the loop's decision with a non-empty queue and asserts no second push. The queue is
  behind a mutex on `Supervisor`, so the testable unit is the predicate, not the `tokio::spawn`.
- The `caught_up` flag still means "this morning needed a catch-up", so the trailing
  `enqueue_daily` that folds a late package still fires. Gating the push must not gate the fold.
- Re-read after deploy: hold the queue with a long job on a weekday morning and confirm the queued
  catch-up count stays at 1 rather than climbing.
- Recorded either way: whether the duplicate `project | rebuild=false` pair is worth the same
  treatment, or is cheap enough that a second no-op projection is not worth a dedupe rule. It is a
  different cost — a projection with nothing to fold is fast but not free.

## FIXED 2026-09-16 — the push is gated, the poll and the fold are not

Status: fixed, gate green (`GATE-EXIT=0`), not yet deployed.

`catch_up_probe_pending()` asks the same question `catch_up_missed_tick` already asks, and the
polling arm now asks it before pushing. Three things were deliberately left alone:

- **The poll keeps its cadence.** `latest_ted_issue_now()` is still read every 5 minutes — that read
  is how the loop learns the package landed, and slowing it would make a late package land later.
  Only the PUSH is gated.
- **`caught_up` is still set every pass.** It means "this morning needed a catch-up" and drives the
  trailing `enqueue_daily` that folds a package which arrived during the window. Gating the push must
  not gate the fold, and a test would not have caught that — it is a one-line ordering the comment
  now states.
- **The match is on the params string, not on `kind`.** Suppressing on `kind == "probe"` alone would
  let the daily tick's own probe silence the catch-up, which is precisely the case issue 222 exists
  for: the tick's probe can run, succeed, and still miss a package that lands at 10:15. The test
  asserts this directly — it pushes `ted daily (probe)` first and requires the predicate to stay
  false.

The predicate reads the in-memory queue, which is what the worker pops. It cannot see a probe that
has already been popped and is RUNNING, and does not need to: that one is about to advance the
watermark, which ends the loop.

### The duplicate `project` pair — decided, not fixed

The `## Done when` required a decision either way. **Left alone, deliberately.** The two
`project | rebuild=false` jobs come from different places with different contracts: one from the
daily tick, one as a re-parse's follow-on. A dedupe rule would have to know that the second's
change-set is a superset of the first's, which is true today and is not a property either caller
states. And the cost is asymmetric with the probe's: a projection with nothing to fold exits after
its planning pass, where a probe is a network round trip — so the thing that made the probe worth
gating (a turn taken from a long job, 36 times) does not hold here at 2. Revisit if a queue is ever
seen holding more than two.

### Live acceptance still owed (after deploy)

Hold the queue with a long job on a weekday morning and confirm the queued
`probe | ted daily (catch-up)` count stays at 1 rather than climbing. Today's four were the
observation that prompted this; they will drain on their own.
