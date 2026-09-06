# 361 — the running server held ~190 GB the filesystem showed as used; a restart released it

Status: OBSERVED 2026-09-06 03:3x UTC (owner) — measured once, mechanism inferred, detection recipe in hand; measure on the next heavy job day before building anything. Filed from the 359 fold night.
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
