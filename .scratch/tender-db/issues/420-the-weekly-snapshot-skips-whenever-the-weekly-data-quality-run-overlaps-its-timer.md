# 420 — the weekly snapshot skips whenever the weekly data-quality run overlaps its timer, and the ring silently ages

Status: **DONE 2026-09-26** — the first scheduled Sunday firing under the gate (2026-09-20 05:23 CEST) wrote `tender-db-1789874587.db` at 05:28:54 (5 min 47 s) and pruned the 09-06 copy; the queue was already idle (the weekly data-quality run, job 1501, ran 01:10–02:51 UTC = 6068 s, and the org-merge chain behind it was done by 02:55 UTC), so no `waiting:` line was needed. `/data` went from 517 GiB free (09-19) to **831 GiB free** on 2026-09-26 — the 09-06 copy held ~300 GiB of private extents, not the ~600 the filing expected (see the foot). The `## Verify` block reads done. Was: ready-for-agent — filed 2026-09-19 12:0x UTC by the hourly audit (step 3) chasing 146 GB of free space that vanished in six days; the fix (wait for the queue, then snapshot) is BUILT with four offline harness cases, INSTALLED on the box by `install.sh`, and a fresh snapshot was fired by hand on the idle Saturday queue: written 2026-09-19 14:07 CEST (`tender-db-1789819325.db`, 628 GB apparent, reflink-shared, 5 min 6 s wall), the 09-03 copy pruned, ring = 09-06 + 09-19. Sunday 2026-09-20 05:23 CEST is the first scheduled firing under the new gate; the Verify block reads it.
Kind: operability (a scheduling collision between two weekly units; the visible symptom is disk)
Relates to: 269 (the snapshot unit), 169 (the storage lifecycle model — divergence is the cost this issue prices), 230 (the weekly data-quality tick at 03:10 Berlin), 224 (the watchdogs live in git and install idempotently — the path this fix took), 373 (the offline harness the new cases join).
Blocked by: nothing

## Verify

    ssh -o BatchMode=yes root@zebreus.click "journalctl -u tender-db-snapshot.service --since '-8 days' --no-pager | grep -E 'snapshot written|skipped snapshot|queue idle after' | tail -3"

- **done**: the latest Sunday firing prints `snapshot written: …` (with or without a `queue idle after N poll(s)` line before it) — the ring rotated this week
- **open**: `skipped snapshot: job N …` as the latest line, i.e. the week's snapshot was lost — the 09-13 reading; on 2026-09-19 the latest line is the hand-fired `snapshot written: …tender-db-1789819325.db`, and the first SCHEDULED reading under the new gate is Sunday 09-20's

## Observed (2026-09-19)

- `/data/db/snapshots` held two reflink snapshots, 2026-09-03 and 2026-09-06, each 648.5 GB
  apparent and **649 GB allocated** — both fully diverged from the live database (the whole-corpus
  folds of 09-12..16 rewrote every page) and largely from each other. `xfs_io fiemap` on the live
  file: 0 shared extents.
- The 09-13 firing: `skipped snapshot: job 1341 is running — next timer firing retries`. Job 1341
  was `data-quality (weekly)`, 01:10 → 03:57 UTC (166 min); the timer fired 03:23 UTC.
- Free space: 701 GB at the 09-13 disk census → 555 GB (df, 09-19), with the database itself +13 GB.
  The rest is snapshot divergence pinned by a ring that did not rotate.
- The weekly data-quality run's last eight durations: 88, 89, 91, 117, 122, 166, 190, 193 min. From
  a 01:10 UTC start it overruns the 03:23 UTC timer whenever it exceeds ~133 min — three of eight.

## The mechanism

`tender-db-snapshot.sh`'s quiescence gate was a single check: if `GET /admin/jobs` shows a running
job, print a line and exit 0. Correct in intent (a fold mid-write would reflink a mid-transaction
page soup), wrong in shape: the weekly data-quality job is the one job that is reliably running at
05:23 Berlin on a Sunday, because the scheduler put it at 03:10 Berlin for the same reason the
snapshot sits at 05:23 — the emptiest slot of the week. Two units chose the same quiet hour and one
of them lasts two to three of it. "Next timer firing retries" meant next WEEK.

## Fix — wait for the queue, then snapshot (built 2026-09-19, `4d74a8f`)

The gate polls `GET /admin/jobs` once a minute for up to `TENDER_SNAP_WAIT_MIN` minutes (default
150: 05:23 + 150 min = 07:53 Berlin, still clear of the 09:35 daily tick) and only then skips,
loudly. `TENDER_SNAP_DRY=1` stops right after the gate so `test-watchdogs.sh` can pin it without XFS:
an idle queue snapshots at once and never prints `waiting:`; a running job past the budget prints
`skipped snapshot: job N still running after …` and never snapshots; a job that ends mid-wait
prints `queue idle after N poll(s) — snapshotting` and proceeds. The unit gains `TimeoutStartSec=4h`.
Installed by `install.sh` (which runs the harness first) — the same path 224 built.

## Done when

- The harness's four gate cases pass (they do) and `install.sh` installs the new script and unit.
- A hand-fired run on the idle queue writes a snapshot and prunes the ring to KEEP=2 (foot).
- The 2026-09-20 05:23 CEST firing writes a snapshot even if the data-quality run is still going —
  its journal shows `waiting:` … `queue idle after N poll(s)` … `snapshot written`.
- 169's record carries the divergence price: a held reflink snapshot costs FULL size across any
  whole-corpus fold, and a ring that fails to rotate keeps paying it.

## 2026-09-19 12:0x–12:08 UTC — fix installed, ring rotated by hand

- `install.sh` on the box (the ops tree copied with tar over ssh; rsync is absent in the operating
  container): the harness's 12 cases passed there, the script and unit were installed, timers
  re-enabled, the three watchdog dry fires read ok. `/usr/local/bin/tender-db-snapshot.sh` carries
  the gate, the unit `TimeoutStartSec=4h`, next firing `Sun 2026-09-20 05:23:00 CEST`.
- Before the install, on the idle queue, `systemctl start tender-db-snapshot.service` (the old
  script — an idle queue passes its gate): 14:02:05 → 14:07:11 CEST, `snapshot written:
  /data/db/snapshots/tender-db-1789819325.db (628G apparent; reflink-shared)`, `pruned …
  tender-db-1788425421.db` (the 09-03 copy), `snapshots kept: 3 -> 2`. Ring: 09-06 (648.5 GB, fully
  diverged) + 09-19 (shares everything with the live file today). `df` unchanged at 517 GB free —
  the pruned 09-03 shared its blocks with 09-06, so nothing was released; the 09-06 copy is the one
  holding ~600 GB of pre-fold pages, and it goes at Sunday's rotation (KEEP=2) if the gate holds.
  The prod-read snapshot is now 13 minutes old instead of 13 days.

Expected Sunday: the data-quality run starts 03:10 CEST; the snapshot timer fires 05:23 and
either snapshots at once or prints `waiting: job N is running` and `queue idle after M poll(s)`
before `snapshot written`; the 09-06 copy is pruned and ~600 GB return to `/data`. If instead the
journal shows `skipped snapshot: … still running after 150 poll(s)`, the data-quality run took
more than 4 h 13 min and the budget needs raising — the 169 disk census on the same morning
(01:10) will not yet show the release either way.

## 2026-09-26 08:0x — Sunday rotated under the gate; what came back

Journal of the 2026-09-20 firing: `Starting … 05:23:07`, `snapshot written: /data/db/snapshots/tender-db-1789874587.db
(628G apparent; reflink-shared) 05:28:54`, `pruned /data/db/snapshots/tender-db-1788664999.db`, `Finished`. No
`waiting: job N` line — the data-quality run had finished at 02:51 UTC (6068 s, the longest of its last three:
5382 s on 09-17) and the timer fires at 03:23 UTC, so the gate found the queue idle on its first poll. Ring today:
09-19 (`1789819325`) + 09-20 (`1789874587`), each 627 GiB apparent.

**Disk (df 2026-09-26 08:0x): 828 GiB used, 831 GiB free of 1.7 TiB** — up from 531 GiB free at the 09-20 01:10
disk census (before the rotation) and 517 GiB on 09-19. That is ~300 GiB released, not the ~600 the filing
predicted: the 09-19 note's "649 GB ALLOCATED, fully diverged" for the 09-06 copy came from `du`/allocated size,
which counts extents a reflink copy still SHARES with the live file. The private cost of a held snapshot is only
what `df` gives back when it is pruned. Today's arithmetic says the same: `/data/archive` 180 GiB + live DB 629 GiB
(674,860,470,272 bytes) = 809 GiB, so the two-snapshot ring costs ~19 GiB of private extents six days after the
09-20 copy — the daily ticks diverge a few GiB a day. The number to watch is `df`, and the weekly disk census
(169) reads it; the 09-27 census is the first that will show the rotation.
