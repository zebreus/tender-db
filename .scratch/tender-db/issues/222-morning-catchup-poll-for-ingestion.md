# 222 — ingest fires once at 09:35; a bounded morning catch-up poll would never miss a late TED day and fetch the moment it's final

Status: IMPLEMENTED & DEPLOYED 2026-08-16 (serving rev `e9cf997`) — awaiting first live weekday morning.
Lennart said "it's your project, you decide", so I built it. Chose the lowest-risk shape (design choice #1
below → the read-only watermark variant): the 09:35 tick and `enqueue_daily` are UNCHANGED, and a
purely-additive weekday catch-up runs after the tick — give the tick's probe time to land TED's package,
and if it has not, re-probe every 5 min (`CATCHUP_POLL_SECS`) until it does or a ~3h window closes
(`CATCHUP_WINDOW_SECS`), then process+project the late package the same morning. On a normal day the
watermark has already advanced so no catch-up runs; a late/holiday morning is a handful of cheap no-op
probes. Detection is a read-only `latest_ted_issue` check, so nothing races the queue. Commit `e9cf997`;
`latest_ted_issue_now_reflects_the_newest_registered_daily` + the 22-test supervisor suite green; clean
boot verified in prod.

**Owed:** first live observation on a weekday morning (next: Mon 2026-08-17 09:35 CEST) — confirm a normal
on-time TED day still runs exactly once (no spurious catch-up), and, ideally, watch a slipped day get
caught. Today (Sun) the weekday-gated catch-up does not run.

Was: PROPOSED — awaiting owner (Lennart) go-ahead on the approach 2026-08-16. Prompted by Lennart's
question "can we check more often and start the big thing in reaction?" (2026-08-16). The answer to the
literal question is no (see Non-goal); this issue is the useful version of the idea.
Kind: operability / ingestion responsiveness + resilience
Blocked by: — (design decision only)
Relates to: 16 (the in-process supervisor/scheduler), 69 (walk-forward probe so a missed tick catches up),
docs/research/ted-access-channels.md (the upstream cadence this is bounded by)

## Today

`supervisor.rs::spawn_scheduler` sleeps to the next **09:35 Berlin** tick and calls `enqueue_daily`, which
enqueues `probe → process → project` (TED Mon–Fri, DÖE every day). 09:35 sits just after TED's daily
package is "finalized by 09:30 CET". It fires **exactly once per day**; there is no retry.

## Non-goal (why "every few seconds" buys nothing)

The upstream publishes **once a day**. TED bundles a whole day's notices into a single daily package (one
OJ S issue), uploaded 00:01–09:00 CET and final by 09:30, Mon–Fri; per
`docs/research/ted-access-channels.md`: *"there is no intra-day channel, so 'near-realtime' means 'same
morning'."* DÖE is one export/day at T+1. So a seconds-cadence poll would issue ~17k requests/day to find
the one package that appears each morning — it cannot make data fresher, because nothing new exists between
daily drops. Do NOT build a high-frequency all-day poller.

## The real win — a bounded morning catch-up

Replace "one shot at 09:35, hope TED is on time" with "poll through the morning window until today's issue
lands, then idle till tomorrow." Concretely, on a weekday from ~09:30:

- run the cheap probe (a HEAD/registry hit that 404s when nothing's new) every few minutes;
- the first time it registers a **new** TED issue, run `process → project` and stop for the day;
- close the window at a cap (e.g. 12:00) so a genuinely absent day (holiday, outage) doesn't poll forever —
  fall back to the existing next-day behavior (the walk-forward probe already catches a missed day, issue 69).

Value: (1) **never lose a day** when TED finalizes late (past 09:35) — today that day's notices wait a full
24h; (2) **fetch the instant it's final** instead of at a fixed time; (3) auto-recover from a morning
network blip. Cost is negligible — the probe is one cheap request that 404s until there's data.

## Design choices (need a decision before building)

1. **How the scheduler learns the probe found something.** The probe currently runs as a *queued* job
   (durability + serialization with process/project). Options: (a) keep it queued and have the scheduler
   poll `latest_ted_issue` before/after to detect an advance; (b) run a lightweight probe INLINE in the
   scheduler for the detection only, then enqueue process/project as jobs (careful: an inline probe that
   writes must not race a queued job — keep detection read-only, or gate on `heavy_write_in_progress`).
   **Recommended: (a)** — no new inline/queue race, the queue stays the one writer path.
2. **Window + interval params.** Recommend start 09:30, cap 12:00, interval 3–5 min (all consts, easy to
   tune). Weekday-only for TED; DÖE stays on the daily tick (T+1, nothing to gain from morning polling).
3. **Idempotence / no double-run.** Once today's issue is processed, the day is done — guard on "already
   caught up today" so a late scheduler wakeup can't re-enqueue.

## Verification

- Unit: the tick/window computation (next 09:35, window open/close, "already done today") is pure and
  table-tested like `next_berlin_tick`.
- Integration/manual: with a stubbed probe returning NotFound then Fetched, the catch-up loop enqueues
  process+project exactly once and stops; an all-NotFound window closes at the cap without enqueuing.
- Prod: on a normal morning, the daily cycle starts within minutes of TED finalizing rather than at a fixed
  09:35; a deliberately-late test day is still picked up same-morning.

## Owner note

Filed because I asked Lennart "want me to implement?" and this captures the design for his yes/adjust. It
touches the scheduler (high blast radius), so I'm holding implementation for his nod on choice #1 rather
than picking unilaterally. Ready to build with tests the moment it's approved.

## Pre-observation note (2026-08-17 ~02:00 UTC, owner)

Scheduler audited ahead of the first live weekday morning (today, 07:35 UTC): armed (in-process
loop, re-computed at every boot — tonight's deploy restarts are harmless, the next tick always
lands on the coming 09:35 Berlin). Expected evidence of a NORMAL day: probe/process/project jobs
at ~07:35 UTC, no "(catch-up)" params anywhere, exactly one enqueue_daily.

**Operational rule found while auditing:** `ted_before` — the watermark the catch-up compares
against — is PROCESS-LOCAL state captured at the tick. A deploy restart during the catch-up
window (07:35–10:35 UTC) loses it and silently skips that day's catch-up (next tick computes to
tomorrow). So: **do not deploy between 07:30 and ~08:00 UTC on weekdays**, and after the daily
jobs finish check the watermark advanced before any morning deploy. If a restart in that window
is ever unavoidable, re-run the day by hand (`probe refetch=true` + process + project).
