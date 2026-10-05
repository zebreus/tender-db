# 488 — the DB file is 90 million extents, and the fold spends three quarters of its time in the kernel

Status: ready-for-agent — filed 2026-10-05 from a `perf` sample of fold 2002 (issue 479's refold). The first step needs
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
