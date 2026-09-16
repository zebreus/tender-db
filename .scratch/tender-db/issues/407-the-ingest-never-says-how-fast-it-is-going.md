# 407 — the ingest logs nothing per package, so a 240× slowdown is invisible until someone counts rows by hand

Status: ready-for-agent — unit 1 (the per-package rate line) LANDED 2026-09-16, see the foot. The calibrated guard on top of it is open, deliberately.
Kind: defect (observability — `run_process` in `crates/app/src/supervisor.rs` logged nothing between "job started" and "job finished")
Relates to: 404 (the regression that made the point: an unindexed twin lookup took a TED daily from 30 notices/s to 0.12), 406 (the stop lever the same incident showed was missing), 405 (the same class one layer up — the dashboard refresher was mute on success, and that silence cost a read the same day), 230 (the data-quality job's per-window timing line, which is the style this follows and which made ITS 88-minute run legible the same morning)
Blocked by: nothing

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
