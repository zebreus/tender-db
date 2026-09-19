# 407 — the ingest logs nothing per package, so a 240× slowdown is invisible until someone counts rows by hand

Status: ready-for-agent — **the calibrated guard is BUILT, gated (128/128) and DEPLOYED 2026-09-19 05:07 UTC at `110d527`** (see the foot): the floor is the box's own history per (source, kind) — the median members/s of the previous writing walks over 10, the divisor calibrated against a week of real lines — and the `[process]` line and the job summary carry it. **LIVE 2026-09-19 07:35 UTC:** the tick's two writing walks printed `doe daily 2026-09-18: 1046 members → 1046 notices (0 dup) in 4.1s (252.9 members/s, floor pending 0/5)` and `fts daily 2026-09-18: 483 members → 482 notices (1 dup) in 0.7s (701.4 members/s, floor pending 0/5)`, the 71 pure-dedup re-walks the plain rate — the table, the judge and the clause work on prod. The floor itself appears after five writing walks per (source, kind), about a week for the dailies; the Verify block flips then. Unit 1 (the per-package rate line) LANDED 2026-09-16.
Kind: defect (observability — `run_process` in `crates/app/src/supervisor.rs` logged nothing between "job started" and "job finished")
Relates to: 404 (the regression that made the point: an unindexed twin lookup took a TED daily from 30 notices/s to 0.12), 406 (the stop lever the same incident showed was missing), 405 (the same class one layer up — the dashboard refresher was mute on success, and that silence cost a read the same day), 230 (the data-quality job's per-window timing line, which is the style this follows and which made ITS 88-minute run legible the same morning)
Blocked by: nothing

## Verify

    ssh -o BatchMode=yes root@zebreus.click "journalctl -u tender-db --since '-3 days' --no-pager | grep -F '[process]' | tail -1"

- **done**: the line carries the floor it was checked against beside the rate — `… (716.0 members/s, floor N)` — or an ALARM clause when it is under it: the calibrated per-(source, kind) guard exists
- **open**: `… (701.4 members/s, floor pending 0/5)` (read 2026-09-19 07:48, the first tick after the deploy) — the guard runs but has no floor until five writing walks of that (source, kind) exist; before the deploy the line carried the rate alone

## What happened

`process ted daily 2026-00135` ran for over two hours at 7.5 members/minute. The same job shape did
3,534 new notices in 119 s the morning before. Nothing in the journal said so — between the job's
start and its summary the ingest emits no line at all, so the only way to see it was to read
`members_done` out of `/admin/jobs`, sample it twice by hand to get a rate, and compare that against a
job summary from a previous run.

The data-quality job, by contrast, prints a line per window; that is how its 88-minute run the same
morning was legible at a glance.

## Done when

- Each package logs its members, notices, duplicates, wall time and members/s when it completes.
- A guard fires when the rate collapses — but only on a threshold CALIBRATED against real lines, not
  a guessed one. See below.

## Unit 1 landed 2026-09-16

    [process] ted daily 2026-00135: 3550 members → 3550 notices (0 dup) in 9.3s (382 members/s)

in the style of `[data-quality] window 14/35 …: 64.1s for 16 queries`.

**No threshold, and that is a decision rather than an omission.** The floor that would have caught
today is easy to pick — anything between 1 and 10 members/s — and easy to get wrong: a monthly package
with huge members is legitimately slower per member than a daily, and a cold archive read is slower
than a warm one. This codebase's habit is to calibrate a threshold against measurement and say so
(`PUBLICATION_GAP_MIN_DAYS`: "CALIBRATED, not guessed", with the fortnight it was measured on). These
lines are that measurement. The guard belongs on top of a few weeks of them, per source and package
kind, and filing it now with a number in it would be exactly the guess the rest of the board refuses.

## Still open

- The calibrated guard: once the daily/monthly/FTS lines have accumulated, set a per-(source, kind)
  floor and make the job summary carry an ALARM clause the way `org-merge-health` does.

## Comment — 2026-09-17: first ordinary fold with the line in place, and it earns itself

The weekday fold on rev `7726bcb`, read straight out of `journalctl`:

    [process] ted daily 2026-00179: 3534 members → 0 notices (3534 dup) in 2.1s (1719.2 members/s)
    [process] ted daily 2026-00180: 3424 members → 3424 notices (0 dup) in 25.1s (136.4 members/s)
    [process] doe daily 2026-09-15:  857 members → 0 notices (857 dup)  in 0.2s (4721.4 members/s)
    [process] doe daily 2026-09-16: 1184 members → 1184 notices (0 dup) in 4.2s (281.8 members/s)
    [process] fts daily 2026-09-15:  444 members → 0 notices (444 dup)  in 0.0s (15065.6 members/s)
    [process] fts daily 2026-09-16:  453 members → 446 notices (7 dup)  in 0.7s (680.0 members/s)

**The shape the line makes visible, which the job summary never could:** a package that is pure
dedup runs at 1,700–15,000 members/s, and a package that actually WRITES runs at 136–680. An
order of magnitude and more between "walking members we already hold" and "inserting notices", per
package, per source, every day — so a change in either regime is legible the morning it happens
rather than as an aggregate that moved.

It also priced issue 404's index regression without anybody setting up a measurement: `ted daily
2026-00180` at **136.4 members/s** against the 0.12 notices/s that regression produced. That is what
this line was built for — 404 was found by noticing a job was slow and having no per-package number
to confirm it with.

Worth noting the dedup rates differ by source by 3–8x (fts ~15k/s, doe ~4.7k/s, ted ~1.7k/s) and
that is not yet explained. Probably member size and archive layout; not investigated, recorded so the
next person reading these numbers knows the spread is expected rather than a finding.

## The guard BUILT, gated (128/128) and DEPLOYED 2026-09-19 05:07 UTC (`110d527`) — the floor is measured, not guessed

**The calibration came first, from the journal.** Eight days of `[process]` lines held eleven
writing TED daily walks (2026-00124 to 00134 — 402's backfill — and 00180): 155.7, 79.3, 34.0,
81.8, 156.3, 179.5, 136.2, 135.7, 87.7, 86.9, 75.2 members/s. Median 87.7, a **5.3× natural spread
within ONE (source, kind)** — a cold archive read and a package of large members are both
legitimately slower per member, exactly the objection the 09-16 comment raised against a picked
number. The pure-dedup walks of the same packages ran at 1,565–2,226. Issue 404's regression ran
at 0.12. So the shape of the guard follows from the data: compare a writing walk only with writing
walks of its own (source, kind), take the median of the recent ones, and alarm at a ratio under it
that clears the natural spread with room and still sits far above the defect. **Ten**: the slowest
real walk (34.0) judged against the other ten gets floor 11.2 and clears by 3×; 0.12 is 90× under.

**What landed (`110d527`).**

- `package_rates` (`jobs::SCHEMA`, notice layer, survives every rebuild): one row per walk —
  source, kind, period, members, notices, duplicates, seconds, walked_at — the journal line kept
  where the guard can read it. A few dozen rows a day.
- `store::jobs`: `PackageRate`, `record_package_rate`, `recent_writing_rates(source, kind, 30)`
  (notices > 0, members ≥ 100, newest first), and the pure `rate_verdict(walk, history)` →
  `NotJudged | Pending {have, need: 5} | Clear {floor, median, history} | Alarm {…}` with
  `floor = median / RATE_FLOOR_DIVISOR (10)`. The four constants carry their reasons and the
  calibration in their doc comments.
- `run_process`: judge BEFORE recording (a walk is never its own baseline); a stopped walk is
  neither judged nor recorded; a pure-dedup walk is recorded and never judged; a walk under 100
  members is neither judged nor in any history (fixed cost, not throughput — `doe daily 2026-07-27:
  93 members … in 0.0s`). The line reads `(136.4 members/s, floor 8.8)`, `(…, floor pending 3/5)`,
  or `(… — RATE ALARM (issue 407): under floor 8.8 = median 87.7 of the last 11 writing walk(s) /
  10)`; an alarm is repeated in the job summary as `; N RATE ALARM(S) (issue 407, ted daily): …`,
  because the journal scrolls and the summary is what `/admin/jobs` keeps (the
  `org-merge-health` shape).
- Tests (`crates/store/tests/package_rates.rs`, 4): the history filter and order; the calibration
  fixture — every one of the eleven real walks judged against the other ten clears, 404's 0.12
  alarms; pending under five, dedup and tiny walks never judged (a collapsed dedup walk included:
  the guard is for the write path); the even-count median and "at the floor is clear".
- `docs/operations.md`: "Reading a `process` job's `[process]` lines (issue 407)".

**What was NOT done, and why.** The table starts empty, so every (source, kind) prints
`floor pending n/5` until five writing walks have run — about a week for the three dailies, longer
for monthlies, which walk only on backfills. Seeding it from the journal's historical lines would
give TED daily a floor today, but that is a production write from outside the ingest path and the
operating session's classifier refuses that class (not routed around); a week of pending is the
honest alternative and costs nothing but a week. The guard also says nothing about a walk that is
SLOW IN ABSOLUTE TERMS but consistent with its history — that is by design: the 09-16 comment's
point was that "slow" has no meaning without the (source, kind) it is slow for.

**Acceptance, owed.** The 07:35 tick's three writing walks print `floor pending 0/5` (proof the
table and the path work on prod); the Verify block flips to `floor N` once five writing walks of
one (source, kind) exist, ~2026-09-26. The first `RATE ALARM` line, whenever it comes, closes the
issue's second `## Done when` item in the only way it can be closed.

## LIVE 2026-09-19 07:35 UTC — the first tick after the deploy, read at 07:48

    doe daily 2026-09-18: 1046 members → 1046 notices (0 dup) in 4.1s (252.9 members/s, floor pending 0/5)
    fts daily 2026-09-18: 483 members → 482 notices (1 dup) in 0.7s (701.4 members/s, floor pending 0/5)

Both writing walks were judged and found history-less, exactly the designed first reading; the 71
pure-dedup re-walks around them (DÖE walks every daily package since 07-17 each morning, 48,161
members for 1,046 new notices) printed the plain rate and were recorded, not judged. No TED walk
today: the scheduler skips TED on weekends by design (`enqueue_daily(weekday)`), so TED's five
writing walks accrue Monday to Friday. Saturday's job summaries carry no alarm clause. The
acceptance this record owed is met; what remains is the calendar — `floor N` on the dailies from
about 2026-09-26, and the first `RATE ALARM` whenever a walk earns one.
