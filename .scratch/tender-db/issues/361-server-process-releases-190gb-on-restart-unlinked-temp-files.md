# 361 — the running server held ~190 GB the filesystem showed as used; a restart released it

Status: DETECTION BUILT 2026-09-06 16:xx UTC (owner) — the recipe now runs inside the process: `/metrics` gauges `tender_db_deleted_open_{files,bytes}` and the weekly `disk-census` fields `deleted_open_*` with a 1 GiB alarm, so next Sunday's walk measures the class without anyone at the keyboard; the creator hunt (step 2) still waits for a non-zero reading. Was: OBSERVED 2026-09-06 03:3x UTC — measured once, mechanism inferred, detection recipe in hand. Filed from the 359 fold night.
Kind: storage / operations (relates to 169's model and 337's leaked temp database)
Relates to: 169 (storage lifecycle), 337 (turso temp database leak), 83 (service tmpdir), 269 (snapshot ring — ruled out as the cause)

## Observed

`/data` free space, bounded `df` reads through the night of 2026-09-05/06:

| time (UTC) | free | what happened |
|---|---|---|
| 01:10 | 432 GiB | weekly disk-census (job 749) |
| 03:19 | 430 GB | after the 359 label repair + the first R2 fold (12k rows removed) |
| 03:25 | 437 GB | weekly snapshot: new reflink copy, Aug 30 pruned (+7 GB) |
| 03:32 | — | deploy 895acf6 restarted `tender-db.service` |
| 03:35 | **628 GB** | after the restart and a 4k-group fold (+191 GB) |

Nothing on disk accounts for it: `/data/tmp` holds four `.tmp*` dirs with sub-MiB
`tursodb-temp.db` files, the archive is untouched, the live file's allocation did not move
(843,548 MiB allocated against 618,473 apparent, before AND after), the snapshots are
reflink-shared. Space that `df` counts and no path shows, released by a process exit, is
the signature of **files unlinked while still open** — temp databases (issue 337's class)
or sorter spills created by the day's jobs and deleted while the process still held the
descriptor. The fresh process (pid 2951540) holds 0 `(deleted)` descriptors and 2 open
files under `/data/tmp`, so the recipe is:

```sh
pid=$(systemctl show tender-db.service -p MainPID --value)
ls -la /proc/$pid/fd | grep -c '(deleted)'          # count
ls -la /proc/$pid/fd | grep '(deleted)' | head        # which
```

(bounded: one directory of ~120 entries). Run it after the next long job (a fold, the
weekly data-quality walk, a census) and before the next restart; the sizes come from
`stat -L /proc/$pid/fd/<n>`.

## Why it matters

The daily deploy cadence has been masking it. On a week without a restart the held space
grows until the disk-census's `days_to_full` is real, and no on-disk listing can find it.
It also means the census's "Database file" number understates what the service occupies.

## Not this

- The snapshot ring: the Sunday prune freed 7 GB (169's note).
- XFS speculative preallocation on the live file: the 225 GiB allocated-over-apparent gap
  survived the restart, so it is not a lifetime-bound prealloc. It is still unexplained —
  `fsxattr.nextents = 30,742,982` on the live file (copy-on-write fragmentation), CoW-fork
  staging unverifiable here (`xfs_io bmap -c` refuses on this kernel). Stays with 169.

## Next

1. Measure: `(deleted)` descriptors and their sizes after the next heavy job (any firing).
2. If confirmed, find the creator: turso temp databases (337) are the prime suspect — the
   `.tmp*` dirs are theirs — and the fix is upstream lifecycle or an explicit close; a
   scheduled restart is the fallback, not the fix.

## Probe 1 (2026-09-06 07:50 UTC): a 30 s census does not reproduce it

Baselines on the fresh process: 0 `(deleted)` descriptors after the deploy restart (03:3x),
after the two R2 folds (05:3x) and after the daily tick (07:49). Sampling `/proc/<pid>/fd`
every 10 s through `org-merge-health` (job 772, 31 s): turso opened sorter spills as
`/data/tmp/.tmp<rand>/tursodb_temp_file` in fresh directories — two seen, each gone within
one sample — never in the `(deleted)` state, and the only files held afterwards are the
process's own `tursodb-temp.db` (4 KB) and its WAL (32 B). The `.tmp*` directories stay
behind empty (six now; tmpsweep's business). So a short read-only census neither leaks nor
holds; the 190 GB needs the long walks — the weekly data-quality job (5,560 s, 32 windows)
is the candidate, and the next Sunday batch (2026-09-13 01:10 UTC) is the measurement
window: sample the descriptors and their `stat -L` sizes every minute through it.

**2026-09-06 15:5x UTC — a quiet-day restart released nothing.** The deploy of `a6cf472`
restarted the service on a box that had run only censuses since its previous restart
(13:4x, the 239 deploy): `df` avail 866,405 MiB before and after. So the 191 GB release at
03:32 is tied to the heavy-job night that preceded it, as the recipe assumes; the scheduled
measurement on the next weekly walk (2026-09-13 01:40Z) stands. Note for the model: the
live file's 220 GiB of allocated-over-apparent was a different thing entirely (issue 169
item 3, copy-on-write leftovers, reclaimed 15:0x), not part of this issue's class.

## Detection built (2026-09-06 16:xx UTC)

Step 1 of "Next" no longer needs a firing at the right moment. `health::deleted_open()`
walks `/proc/self/fd` (bounded, ~120 entries), keeps every link ending in ` (deleted)`
with the size read through the descriptor, and reports `{files, bytes, sample}` — the
five largest by path. It feeds `/metrics` (`tender_db_deleted_open_files`,
`tender_db_deleted_open_bytes`) continuously and the weekly `disk-census` report
(`deleted_open_files`, `deleted_open_bytes`, `deleted_open_sample`, and
`deleted_open_alarm` at 1 GiB, the alarm text carrying the recipe). Unit-tested by
unlinking a 1 MiB file the test holds open and watching it appear with its size and
vanish on close. Baseline after the deploy restart: 0 files, 0 bytes. The weekly walk on
2026-09-13 (01:10 UTC start) is the first measurement that matters; the census runs in it,
and the gauges can be read at any point during it. Step 2 (the creator) starts from the
sample's paths.

## Sunday 2026-09-13 sample during the weekly data-quality job

The scheduled measurement window, run as the recipe says: pid 3635482, one reading a minute for 15
minutes (03:43–03:58 CEST) while job 1341 (`data-quality`) was in its `measuring` phase (units 42 →
58 of 522 — the windowed queries, before the whole-corpus statements). Bounded reads only.

| reading | `(deleted)` descriptors | files under `/data/tmp` | sizes |
| --- | --- | --- | --- |
| every one of 15 | **0** | `tursodb-temp.db`, `tursodb-temp.db-wal` | 4,096 B, 32 B |

`df /data`: 1006 G used, 654 G free, 61 % at start and at end — unchanged. The in-process gauge
`tender_db_deleted_open_files` read 0 at the end of the window, and this Sunday's `disk-census`
(job 1339, 03:10) read 0 unlinked-but-open files (0.0 GiB) before the job started.

**So the class did not reproduce in this window.** The two `/data/tmp` files are the fresh
process's standing turso temp database (the 2026-09-06 baseline saw the same pair), never in the
`(deleted)` state and 4 KiB in size. What this window did NOT cover: the job's whole-corpus
statements (they run after the 480 windowed units) and the org-scan jobs queued behind it — the
2026-09-06 observation was tied to the heavy-job night as a whole, not to one phase. The detection
built on 2026-09-06 watches continuously (gauge + weekly census with the 1 GiB alarm), so a
non-zero reading will land on the report without a keyboard; a hand sample at a chosen minute is
the wrong instrument for a class that may appear for seconds, and this one is the last of its kind
unless the gauge fires.
