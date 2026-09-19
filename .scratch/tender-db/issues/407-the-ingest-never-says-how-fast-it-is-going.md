# 407 — the ingest logs nothing per package, so a 240× slowdown is invisible until someone counts rows by hand

Status: ready-for-agent — unit 1 (the per-package rate line) LANDED 2026-09-16, see the foot. The calibrated guard on top of it is open, deliberately.
Kind: defect (observability — `run_process` in `crates/app/src/supervisor.rs` logged nothing between "job started" and "job finished")
Relates to: 404 (the regression that made the point: an unindexed twin lookup took a TED daily from 30 notices/s to 0.12), 406 (the stop lever the same incident showed was missing), 405 (the same class one layer up — the dashboard refresher was mute on success, and that silence cost a read the same day), 230 (the data-quality job's per-window timing line, which is the style this follows and which made ITS 88-minute run legible the same morning)
Blocked by: nothing

## Verify

    ssh -o BatchMode=yes root@zebreus.click "journalctl -u tender-db --since '-3 days' --no-pager | grep -F '[process]' | tail -1"

- **done**: the line carries the floor it was checked against beside the rate — `… (716.0 members/s, floor N)` — or an ALARM clause when it is under it: the calibrated per-(source, kind) guard exists
- **open**: `[process] fts daily 2026-09-17: 441 members → 439 notices (2 dup) in 0.6s (716.0 members/s)` — the rate alone; the guard waits for a few weeks of lines by design (read 2026-09-19)

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
