# 488 — the DB file is 90 million extents, and the fold spends three quarters of its time in the kernel

Status: ready-for-agent — DEFRAG DONE 2026-10-06 (90.9M → 1.95M extents; see "Defrag run"); NEXT: time the next fold, and decide the snapshot strategy before 2026-10-11. Filed 2026-10-05 from a `perf` sample of fold 2002 (issue 479's refold). The first step needs
Lennart's word, because it removes the last snapshot and takes downtime. After the fold, decide the snapshot strategy,
then defragment the DB once (see the proposed fix below).
Kind: performance / ops
Relates to: 269 (the reflink snapshots), 479 (the refold that surfaced it), 475 (disk), 1616 (the last full fold, 7.4 h)

## What was measured (2026-10-05 ~22:00 UTC, fold 2002 phase 2)

- **Where the time goes.** `perf record -F 199 -g -p <server> -- sleep 30` on the fold thread (`job-exec`):
  - 75.7 % of samples are in the kernel and 24.3 % in `server`.
  - The top kernel symbols: `xfs_iext_lookup_extent` 9.2 %, `filemap_get_folios_tag` 5.5 %, `__submit_bio`,
    `xfs_buf_lookup`, `xas_find_marked`, and the iommu/kmalloc paths beneath them.
  - The fold thread runs at ~80 % of one core. IO pressure is ~5 %, so this is CPU spent walking filesystem metadata,
    not waiting on the disk.
- **The DB file is fragmented.** `xfs_io -r -c stat`:
  - `/data/db/tender-db.db` is 693.7 GB in 90,221,462 extents, about 7.7 KB per extent. In practice that is one extent
    per few pages.
  - The 2026-10-04 reflink snapshot has 87,807,676 extents.
  - `cowextsize` is 4096, so every copy-on-write after a reflink snapshot allocates 4 KB pieces.
- **Cause.** The weekly `cp --reflink=always` snapshots (issue 269) share every extent with the live DB. Each page the
  app writes afterwards is copied out of sharing as a new 4 KB extent. Weeks of daily folds plus this corpus-wide fold
  have cut the file into tens of millions of extents. Every write, writeback and fsync then searches a 90M-entry
  extent tree.
- **Fold rates.** This fold ran at 4.9k–18.6k tenders/min, against job 1616's whole fallback in 7.4 h. Only a quarter of
  the CPU is the fold's own code, so the user-space code is not the first lever.
- **Same mechanism, disk side.** The un-sharing also consumed `/data` (319 → 280 GB free during the fold, issue 479),
  until the 2026-09-27 snapshot was deleted (≈317 GB returned).

## Proposed fix (in order)

1. **Stop making it worse.** Raise the COW extent hint on the live DB, for example
   `xfs_io -c "cowextsize 16m" /data/db/tender-db.db`, so post-snapshot copies are allocated in large runs. Do this
   first and verify it on a scratch file.
2. **Replace reflink snapshots** with a copy that does not fragment the live file, or keep reflinks but defragment
   after each one (step 3 on a schedule). Options: SQLite's backup API or `VACUUM INTO` onto /data (costs a full copy of
   space and time, ~700 GB), or a snapshot on another volume. The snapshot is forensics, not DR (its script says so);
   the raw archive is the rebuild path.
3. **Defragment once.** With the service stopped, and with the last snapshot gone or moved off-volume, either
   `xfs_fsr /data/db/tender-db.db` or `cp --reflink=never` + rename. This needs ~700 GB of free space and roughly an hour
   of downtime. Measure `nextents` before and after, and re-time the next daily fold.
4. **Then profile user space.** The `server` binary is stripped (perf shows only addresses), so profile a build with
   symbols against a copy before touching the fold's code.

## Verify

`xfs_io -r -c stat /data/db/tender-db.db | grep nextents` reads a small number (thousands, not millions). A `perf`
sample of the next fold shows the kernel share below ~20 %, and the next corpus-wide fold beats job 1616's 7.4 h.

## Defrag run (2026-10-06, Lennart's go-ahead: "downtime and losing the one snapshot is fine")

- **Preconditions.** Fold 2002 had finished (`ok`, 8,776,591 tenders). The 2026-10-04 snapshot was deleted;
  `/data` free went 640 → 814 GiB once xfs had reclaimed it. The free space was still fragmented: about 210 GiB in
  runs of 1 GiB or more, the rest in small runs, average free extent 12 blocks.
- **Run.** `ops/defrag-db.sh run` as transient unit `tender-db-defrag`. Service stopped at 01:34 UTC. Copied 649 GiB
  with `cp --reflink=never` in 1,855 s (~350 MB/s). `cmp` reported the copy identical, the files were swapped by
  rename, and the service was healthy again at about 02:25 UTC. About 50 min of API downtime.
- **Result.** 90,930,280 extents became 1,946,893 (47× fewer). The original `.pre-defrag` was deleted after the API
  checked out: tender 8576017 serves `["open"]`, `?procedure_type=open` answers, `/v1/tenders?country=DE` returns
  in 0.6 s, and the queue is idle. `/health/deep` showed 503 only on `disk` (both copies present) until the
  deletion.
- **Hint.** `xfs_io -c "cowextsize 16m"` was set on the live file (was 4096).
- **Still open:**
  1. Time the next daily fold, and the next large one, against job 1616 and fold 2002. Take a `perf` sample to
     confirm that the kernel share fell.
  2. The weekly reflink snapshot timer (Sunday 05:23 CEST) is still enabled. With `cowextsize` at 16m it fragments
     far less, but it still un-shares, which costs disk. Decide before 2026-10-11 whether to keep reflink snapshots,
     move them off-volume, or switch to a backup-API copy.
  3. A second copy pass now has better free space (the 649 GiB original was freed whole), if 1.95M extents still
     show in the profile.
- **First daily after the defrag (2026-10-06).** Project 2010 folded 4,048 notices into 4,260 tenders in 299 s.
  Before the defrag, project 1991 folded 4,685 notices in 311 s. Daily folds are dominated by fixed per-run costs
  (closure, grouping, index upkeep), so they cannot show the gain. Extents held at 1,946,893.
- **Full-fold baseline.** On 2026-10-04 the full folds 1974 and 1978 (14.88M notices → 8.77M tenders) took 17,706 s
  and 16,638 s (~4.7 h). Fold 2002, 36 h later on a file un-shared further by that weekend's snapshot, took
  38,189 s (10.6 h). The next corpus-wide fold is the real measure; compare it with ~4.7 h, not 10.6 h.
