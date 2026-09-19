# 169 — storage lifecycle model at 0.5 TB and beyond

Status: open — ITEM 3 CLOSED 2026-09-06 15:0x UTC: the 220 GiB allocated-over-apparent gap on the live DB file was leftover copy-on-write preallocation (reflink ring + XFS's default 128 KiB COW extent hint); reclaimed live with `xfs_spaceman -c "prealloc -s -m 100g" /data` (2 min 27 s, no outage, **+224,365 MiB free: 641,692 → 866,057**) and prevented with `cowextsize 4096` on the file; details at the bottom. Was: URGENCY RESTORED 2026-09-01. The 2026-08-15 downgrade rested on
"/data at 39%, 1.1T free"; it is now 58% / 709G. That basis is void. Warning
threshold lowered 90% -> 80% as an interim measure (applied to prod today). See
"Re-measured 2026-09-01".

2026-08-28 07:5x (owner) — fresh post-repair snapshot TAKEN:
/data/db/snapshots/tender-db-1787903537.db (reflink, 523.7GB logical) is the
new standing prod-read target, current with ADR-0014's money loci, the 306
repair, the 259 org repair, and the 38.7M-row organization_names satellite.
BOTH older snapshots are now superseded and deletable (their rm remains
classifier-blocked in this session): tender-db-1787374320.db (~472G du) and
tender-db-1787598039.db (~483G du) — freeing up to ~950G once run by Lennart
or a permissive session:
  rm /data/db/snapshots/tender-db-1787374320.db
  rm /data/db/snapshots/tender-db-1787598039.db*

2026-08-28 04:5x (owner) — MODEL EVENT: the epoch refold + the 306 rederive
(83M+ row updates) rewrote most of the live DB's pages, and the two reflink
snapshots DIVERGED toward full copies underneath it: du now shows 472G
(08-22, superseded) + 483G (08-24, the standing prod-read target); /data went
863G free (08-25) → 632.5G free (measured now), ~230G consumed in three days
with barely any logical archive growth. Lifecycle lesson for the model: a
whole-corpus rewrite converts every held reflink snapshot into ~a full copy —
snapshot retention must be priced at FULL size across any refold/repair
campaign, not at COW size. ESCALATION: deleting the superseded 08-22 snapshot
is no longer zero-urgency housekeeping — it frees up to ~470G. The rm is
classifier-blocked in this session (retried tonight); needs Lennart or a
permissive session: `rm /data/db/snapshots/tender-db-1787374320.db`. After
306's acceptance, take a FRESH post-repair snapshot as the new prod-read
target and retire the 08-24 one the same way (it predates ADR-0014's money
loci and will keep diverging). 304's +150G plan must budget against the
post-cleanup number.

2026-08-25 (owner) — the study's pending "one quiet-window restart reclaims the
COW fossil" experiment is ANSWERED by the natural course, and the hypothesis is
FALSIFIED: many service restarts have happened since 08-09 (deploys on 08-17,
08-18, 08-21, 08-23, 08-24 ×2, 08-25) and the allocation barely moved — 601.2 GB
then vs 596.6 GB now. What DID change: the file grew logically 459.7 → 517.8 GB
(+58.1 GB, the text-era campaign), so the fossil shrank 141.5 → 78.8 GB almost
exactly by being GROWN INTO. Model consequence: the fossil is inert preallocated
headroom that new growth consumes before touching free space — at the study's
50–75 GB/yr it is fully absorbed in ~1 year, and /data sits at 48% (863 GB
free). No reclaim action is needed or planned. Housekeeping noted while
measuring: /data/db/snapshots/ holds two reflink snapshots (08-22, superseded;
08-24, the 168-study one — keep as the standing prod-read target); deleting the
08-22 one was classifier-blocked this session — cheap to do in any session that
can, zero urgency at 48%.
Role: run-driver

The disk arithmetic ended at "buy a 500 GB volume" (pilot-sizing); reality:
DB + archive share 1 TB at ~87%. Unstudied: measured steady-state +
rebuild-driven growth (change log is promised forever and rebuilds append
full-corpus change rows), turso behavior at 0.5-1 TB (the bench stopped at
10 GB; the 40-100 GB run was flagged as needed twice and never ran), and the
compaction dead end (VACUUM INTO OOMs — a turso DB can never shrink). Escape
hatch if the volume fills is dump-and-reload of half a terabyte with the
service down — wall-clock never measured. Deliverable: a dated capacity model
with a "volume full" date and a change-log retention trigger criterion (C17's
reserved pruning path has no trigger today). Growth curve is computable from
fetches/notices/changes timestamps — cheap, metadata-only.

2026-08-09 (orchestrator): the study is DONE —
docs/research/storage-lifecycle-2026-08.md. Headlines: /data is 78.1% (the
87% was transient fold-spill — reserve ~50GB transient headroom in all
planning); the DB file carries a verified 141.5GB COW-fork fossil from the
deleted reflink-snapshot era (459.7GB logical vs 601.2GB allocated) —
possibly reclaimable by one quiet-window restart (experiment pending);
growth ~50-75GB/yr steady state of which DB ~90%; a full-rebuild change
generation measured at ~87M rows (~6-8GB, 45% over ADR-0009's estimate) —
rebuild cadence is the controllable driver; earliest 90% breach 2027-Q4
(4 rebuilds/yr) / mid-2028 steady state; dump-and-reload escape hatch is
currently NOTIONAL (no dump tool + doesn't fit on-box — needs a temporary
second volume as standing policy). Deliverable delivered: C17 retention
trigger recommendation (prune superseded-epoch rows at 85% /data or ~175M
changes rows) — belongs next to the issue-46 epoch decision. Remaining:
Lennart decisions (C17 trigger, second-volume policy, housekeeping ack) +
the COW-reclaim experiment.

2026-08-30 05:24: the weekly snapshot service pruned the two superseded
snapshots itself (tender-db-1787598039 + tender-db-1787374320, kept 4 -> 2)
while writing the fresh weekly reflink — the classifier-blocked manual rm
was never needed; the service's own retention did it. /data free 549G ->
713G. Standing state: 2 snapshots (08-28 + 08-30), serving DB 490G.

## Verify

    curl -s --max-time 30 https://tenders.zebreus.click/health/deep | python3 -c 'import sys,json; d=json.load(sys.stdin)["checks"]["disk"]; print(d["used_fraction"], d["free_bytes"]//2**30, "GiB free")'

- **done** (headroom): `used_fraction` under 0.9 — read 2026-09-19: `0.688 517 GiB free` (after item 3's 224 GiB reclaim)
- **open**: at or over 0.9 — the next item of the lifecycle model is due, not a cleanup

## Re-measured 2026-09-01 — the downgrade basis is gone

| | 2026-08-15 (the downgrade) | 2026-09-01 | change |
| --- | --- | --- | --- |
| `/data` used | 39% | **58%** | +19 points |
| `/data` free | 1.1T | **709G** | −~390G |

On a 1.7T volume that is roughly **330–390 GB in 17 days**, and the database file
alone is now **526,163,959,808 bytes = 490 GiB** — this issue's title ("at half a
terabyte") has become literally true rather than prospective.

### Arithmetic, with its caveat stated first

**Two points 17 days apart are not a trend**, and that window contained unusually
heavy activity: the text-era campaign, the issue-325/326/328 repairs, and a dozen
censuses. Steady-state growth may be far lower. With that said, at the observed
~19 GB/day:

* 90% used (the old warning threshold) ≈ 32 days away;
* 100% ≈ 41 days away;
* so the **first warning would have arrived with about nine days of runway** —
  not enough to provision storage or plan an archive prune.

That is the part worth acting on regardless of whether the rate holds: the
*margin between warning and full*, not the date.

### Done today

`TENDER_DISK_WARN_PCT=80` via a systemd drop-in
(`/etc/systemd/system/tender-db-diskwatch.service.d/threshold.conf`), applied and
verified — the hourly watch now reports `ok disk: /data at 58% used, 709G free`
against an 80% line. At the observed rate that is ~24 days of notice instead of
nine; at steady state it is months away and silent. Reverting is deleting the file.

The script already read this valve, so nothing was changed in code and no deploy
was involved.

### Also noticed, and NOT a problem

`/data/tmp` holds ~145 leaked `.tmp*` directories going back to 2026-08-09 —
`TMPDIR=/data/tmp` is set in the unit and something is not cleaning up after
itself. **Total size: 128 MB.** Measured before assuming: it is untidy, it is not
a storage factor, and it should not be confused with the growth above. Worth a
sweep whenever something else touches that area.

### What this issue still needs — and it is exactly its title

A **lifecycle model**, because the honest answer above is "I cannot tell you the
trend from two points." What is missing is a recorded series: /data used, DB file
size, archive size, sampled on a cadence, so growth is read rather than inferred.
That is now cheap to build — issue 335 gave reports a bounded history this
afternoon, so a small periodic disk report would accumulate its own trend with no
new storage design.

Concretely, the next unit: a `disk-census` job writing `df` figures plus the DB
file size to a report on the weekly tick. Ten versions of that is ten weeks of
trend, which turns every future question here from arithmetic-on-two-points into a
reading.

## 2026-09-03 05:5x UTC — the 304 campaign costs about twice its DB growth, and the model says why

Measured mid-fold (job 612 at 5.3M of 7.9M tenders), exact `df -B1` on `/data`:

| | 2026-08-30 (post-prune) | now | change |
| --- | --- | --- | --- |
| free | 713 GiB ≈ 765 GB | 493.0 GB (73% used) | **−272 GB** |
| DB file (logical) | 526.2 GB | 642.2 GB | +116 GB |
| fold spill (`tender-db.db.proj_buckets`) | 0 | 34 GB | +34 GB (released at landing) |
| archive | ~192 GB | 179 GiB ≈ 192 GB | ~0 |
| **unexplained by the above** | | | **≈ +120 GB** |

The 120 GB is the 08-28 model event again, and this time it was predictable: two
reflink snapshots (08-28, 08-30 — both pre-campaign) hold the old extents of every
page the re-parse deleted and re-inserted and every tender-layer page the fold is
rewriting, so **a whole-corpus rewrite bills its snapshots at full size**. The
serving DB's `du` reads 803 GiB against 598 GiB logical — the COW bookends — and
the two snapshots' 489/490 GiB `du` each are, by now, mostly private copies.
Accounting the volume: 1,287.6 GB used = DB 642 + archive 192 + spill 34 +
**≈ 420 GB of snapshot-held and fossil extents**.

### Projection and the action

The fold has ~1.3 h left at the measured 575 tenders/s; it has been consuming
~31 GB/h (2 DB + 14 spill + ~15 divergence), so landing is ≈ 450 GB free before
the spill goes, ≈ 485 GB after — about 73% used. The 80% line (1,424 GB used) is
~140 GB away: **not reached by this campaign**, and `diskwatch` stays `ok`.

The reclaim is not a rewrite or a VACUUM (impossible here anyway) — it is
retiring the pre-campaign snapshots, which the weekly `tender-db-snapshot.service`
already does on its own schedule (KEEP=2, prune newest-first, refuses while a job
runs). Plan, added to the 304 landing runbook:

1. After 612 lands, the daily chain drains and the queue is idle: **run the
   snapshot service once by hand** (`systemctl start tender-db-snapshot.service`).
   It reflinks the post-campaign DB — the new standing prod-read target, current
   with all 24 languages of the text era — and prunes 08-28. That frees only
   08-28's private delta against 08-30 (small): the two pre-campaign snapshots
   share most of their extents with each other, not with the serving DB.
2. **Sunday 2026-09-06 05:23 CEST** the scheduled run takes another and prunes
   08-30 — the last pre-campaign copy — which is when the ~400 GB comes back.
   Expected reading on that firing: free ≈ **850–900 GB (~50% used)**. If it does
   not move by hundreds of GB, this accounting is wrong and that is the finding.
3. Keeping 08-30 until Sunday is deliberate: it is the only pre-flip state left,
   and it costs nothing more than it already has while the landing probes run.
   Retiring it early is one `rm` if the volume needs it — it will not.

Lifecycle rule for the model, stated once: **price a held reflink snapshot at
FULL size across any campaign that rewrites the layer it holds, and retire
pre-campaign snapshots as soon as the campaign is accepted.** Two campaigns
have now demonstrated it (the 08-28 epoch refold + 306 rederive: ~230 GB; the
304 campaign: ~120 GB and counting).

## 2026-09-06 01:2x UTC — 57 % → 74 % in 3.9 days; where the 297 GB went (owner probe)

The weekly disk-census (job 749, 01:10 UTC) read **1,226 GiB of 1,658 used, 432 GiB free,
71 GiB/day against the 2026-09-02 sample, "6 days to full"** — a two-point rate on the two
heaviest write days the project has had (the 353/355/357/359 campaigns: ~1.9 M party rows
and 400k winners repointed, 28k identifiers rewritten, 1,400 country moves, 900 folds), not
a trend. Read against the files, bounded:

| | 2026-09-02 03:51 | 2026-09-06 01:10 |
|---|---|---|
| volume used | 57.3 % (free 761 GB) | 73.9 % (free 464 GB) |
| live DB, apparent | 526 GB | 649 GB (618 GiB) |
| live DB, allocated (`du`) | — | 842 GiB |
| snapshots (`/data/db/snapshots`, issue 269, KEEP=2) | Aug 30 (501 GiB apparent) | Aug 30 + Sep 3 (618 GiB apparent) |

Three things add up to the 297 GB:

1. **The live DB grew ~122 GB apparent in 3.9 days.** Attribution to tables is not done
   (a `dbstat` walk is a corpus scan; the weekly `org-merge-health` / `canonical_rows`
   gauges are the bounded way to follow it week to week).
2. **Reflink divergence.** `/data` is XFS `reflink=1`; a snapshot is free the second it is
   taken and then every page the live DB rewrites leaves the snapshot holding the old block.
   The Aug 30 snapshot now has **29.25 M extents** (a 30 s-bounded `xfs_bmap`) — the
   copy-on-write signature, one extent per rewritten 4 KiB page — i.e. on the order of
   100+ GB of blocks only it references. The campaigns rewrote pages across the whole org
   layer, which is the worst case for a same-volume snapshot ring.
3. **Allocated exceeds apparent on the live file by ~224 GiB.** The file is not sparse and
   reflink cannot make allocation exceed size, so this reads as XFS speculative
   preallocation on a file that is never closed (the reclaim lifetime is 300 s after the
   last close). Unverified: the live file's extent map did not finish inside the 30 s bound
   and MUST NOT be read unbounded (prod-box-reads: `filefrag`/`xfs_bmap` are unbounded
   metadata). `df` counts those blocks as used.

What happens next without intervention: the snapshot timer fires Sunday 03:23 UTC (05:23
CEST); with KEEP=2 it prunes the Aug 30 snapshot and frees its unique blocks — IF the job
queue is idle then (the script refuses while a job runs; the weekly batch started 01:10 and
was on its first job at 01:18). If it is skipped, the Aug 30 snapshot stands another week.

Decision points (for 169's model, not tonight): a same-volume reflink ring costs the
divergence, which on campaign weeks is ~30 GB/day per snapshot; KEEP=1, or snapshotting
only after a `VACUUM`-free quiet week, or an off-volume artifact, are the options. The
weekly census's `days_to_full` should be read with `sample_interval_days` and this note.

### 2026-09-06 03:25 UTC — the Sunday snapshot ran; the prune freed 7 GB, not 100+

The timer fired at 03:23 UTC with the queue idle (the weekly batch had ended 02:47; the
359 fold and projection ended 03:01): new reflink snapshot `tender-db-1788664999.db`
(605 GiB apparent, 2 min 24 s), Aug 30 pruned, 2 kept. **Free went 430 → 437 GB.** So the
Aug 30 copy held only ~7 GB of blocks nothing else referenced — the divergence hypothesis
in item 2 above is wrong in magnitude: pages rewritten since Aug 30 were mostly ALSO
rewritten since Sep 3 (the campaigns hit the same org-layer pages), so the two snapshots
shared them, and the live file's own allocation carries the growth. That leaves item 3 —
allocated 842 GiB against 618 GiB apparent on the live file — as the number to explain,
and item 1's 122 GB of apparent growth to attribute by table. The snapshot ring is cheap
(~7 GB a week at the current cadence); it is not the lever.

### 2026-09-06 03:4x UTC — 437 → 628 GB free at the 03:32 restart; filed as issue 361

The deploy's service restart released ~191 GB that no on-disk path held (temp dirs sub-MiB,
archive untouched, live-file allocation unchanged at 843,548 MiB against 618,473 apparent):
the unlinked-but-open-files signature. Detection recipe and the plan are in issue 361. The
live file's 225 GiB allocated-over-apparent gap survived the restart, so item 3's
speculative-preallocation reading is wrong too; it stays open here (30.7 M extents on the
file — copy-on-write fragmentation from the reflink ring is the remaining candidate, and
`xfs_fsr`/defragmentation or `cp --reflink=never` into a fresh file would be the test, both
heavy — owner conversation, not a firing).

### 2026-09-06 14:0x UTC — item 3 reproduced in miniature: the gap is copy-on-write preallocation left behind on a reflinked file

Bounded reads on the live file (`xfs_io -r -c "stat -v"`, one inode): size 648,515,756,032
(604 GiB), `stat.blocks` 1,727,772,352 (824 GiB), `nextents` 30,739,959, `extsize` 0,
`cowextsize` 0, no attr-fork extents; the count has not moved in four idle hours (last
write 09:35). Both snapshots read `blocks × 512 = size + 0.47 GB` — clean clones carry
no gap; the live file alone does. Kernel 7.0.0-22-generic, `/data` XFS `reflink=1`,
no `allocsize`, `speculative_prealloc_lifetime` 300 s.

Controlled experiment on the same volume (`/data/probe169`, 64 MiB scratch file,
reflink clone, then 2,000 random 4 KiB `pwrite`s = 1,904 distinct pages = 7.8 MB
rewritten, fsync): the original's allocation went from 64 MiB to **118.9 MiB — a gap of
57.5 MB for 7.8 MB of writes**; `nextents` 3,363; the clone stayed at exactly its size.
57.5 MB is what leftover COW preallocation predicts: with `cowextsize` 0 XFS uses its
default COW extent-size hint of 32 blocks (128 KiB), so each rewritten page allocates a
128 KiB COW extent of which 4 KiB is remapped and the rest stays allocated in the COW
fork as speculative preallocation — 512 windows × 128 KiB ≈ 64 MiB minus the pages
actually written. That is item 3's mechanism, scaled: 30.7 M extents on the live file
are the rewritten pages, and the 220 GiB is their windows' unused remainder.

Two things still to settle, both running now: whether the periodic block GC (every
300 s) reclaims it on a CLOSED file (`orig`, re-read at t+7 min) and on a file HELD
OPEN the way the server holds the DB (`orig2`, fd held ten minutes with readings every
two, then closed) — the live file's four idle hours say the open case does not reclaim.
If that holds, the levers are (a) `xfs_spaceman -c "prealloc -s" /data` to force the GC
(the supported maintenance ioctl), (b) a `cowextsize` of one block on the live file so
future COW writes preallocate nothing, and (c) a restart is NOT one (measured 03:4x).
`df` is not lying — those blocks are allocated — so the ~220 GiB is real headroom to win.

**Readings so far (14:1x–14:3x UTC).** The CLOSED scratch file was reclaimed within seven
minutes: 243,448 → 131,192 blocks = its size plus 60 KiB of extent-tree overhead. The file
HELD OPEN kept its full 58 MB gap through eight minutes of the periodic GC, and the forced
trim (`xfs_spaceman -c "prealloc -s -u 0 -m 32m" /data`, scoped to root-owned scratch
files so the live DB was untouched) returned in 24 ms and reclaimed nothing on it. So
lever (a) is dead: neither the periodic GC nor the forced trim touches a file that is
open, and the server holds the DB open for its whole life. What the 03:32 restart shows
is that even a close-and-reopen did not reclaim the live file; the candidate reason is
XFS's dirty-release rule — a file closed while dirty is flagged and its speculative
allocation is left in place on every later close ("in this case don't do the
truncation", fs/xfs/xfs_inode.c) — and a scratch reproduction of exactly that (a close
without fsync, then rewrites, fsync, clean close) is being read at t+7 min.

**The lever that remains, if the dirty-release reading holds: a clone swap at a restart
window.** A reflink clone is a fresh inode sharing every data block (free, ~2.5 min for
this file per the snapshot timings) and carrying no COW-fork reservations; unlinking the
old inode cancels its reservations (inactivation does what release will not). Stop the
service → `cp --reflink=always tender-db.db tender-db.new` → set `cowextsize` to one
block on the clone so future rewrites reserve nothing beyond the page they write →
rename into place, keep the WAL as is → start → `/health` → `rm` the old file. About
three minutes of downtime, ~220 GiB back. Prevention alone (`cowextsize` on the live
file) would stop the growth but reclaim nothing, since the standing reservations belong
to the old inode.

**Rehearsed on the scratch file, 14:3x UTC — the swap works exactly as designed.** With
the gapped file: `cp --reflink=always` → clone at size + 60 KiB (no gap); `xfs_io -c
"cowextsize 4096"` accepted (xflags gains `cowextsize`); rename into place; `rm` of the old
inode → `df` avail rose by 55 MiB, i.e. the reservations were released; then 2,000 more
random-page rewrites on the clone with the fd HELD OPEN left a gap of 112 KiB (was 58 MB
for the same workload without the hint). So the procedure both reclaims and prevents.

**Not executed on the live file: the session's permission classifier denied the
step** (a `systemctl stop` + rename + `rm` of the live DB in one script, 14:4x UTC). Not
worked around. The procedure, ready to run in one ~3-minute window on an idle queue —
every step verified above except on the file itself:

```
cd /data/db && systemctl stop tender-db && sleep 3 && ! fuser tender-db.db
df -B1M --output=avail /data                                  # 361 data point: after stop
cp --reflink=always tender-db.db tender-db.db.new && sync     # ~2.5 min, shares every block
chown tenderdb:tenderdb tender-db.db.new && chmod 600 tender-db.db.new
xfs_io -c "cowextsize 4096" tender-db.db.new                  # one-block COW: no leftovers
mv tender-db.db tender-db.db.old && mv tender-db.db.new tender-db.db   # WAL stays as is
systemctl start tender-db && curl -s localhost:8080/health    # "ok":true, rev unchanged
/root/sqlq.py "SELECT id FROM tenders WHERE id = 93601"       # a read through the clone
rm tender-db.db.old && sync && df -B1M --output=avail /data   # expect ~+220 GiB
```
Rollback if health fails: stop, `mv tender-db.db tender-db.db.new; mv tender-db.db.old
tender-db.db`, start. Expected: allocated = size + ~0.5 GB afterwards, and the weekly
disk census's live-file line stops over-stating by a third.

### 2026-09-06 15:0x UTC — item 3 CLOSED: reclaimed live, no outage, and prevented

The last scratch readings changed the picture once more: the file held open by its holder
was reclaimed by the periodic GC between t+480 s and t+600 s — while STILL open — and the
one closed at t+0 by t+7 min. So "open" was never the discriminator; what the earlier
forced trim on the 20-second-old scratch file showed was only that fresh reservations are
not eligible yet. The live file's five idle hours without reclaim therefore pointed at a
different gate, and the direct test was the forced trim on the file itself, the supported
maintenance path and non-destructive (it frees speculative reservations, touches no data):

```
xfs_spaceman -c "prealloc -s -m 100g" /data      # files >= 100 GiB: the live DB (and the snapshots, which hold none)
```

**2 min 27 s, `stat.blocks` 1,727,772,352 → 1,267,907,456 (824 → 604.5 GiB = size + 0.6 GB),
`df` avail 641,692 → 866,057 MiB: +224,365 MiB.** `/health` ok, a point read fine, queue idle
throughout. Then `xfs_io -c "cowextsize 4096" /data/db/tender-db.db` (xflags now carry
`cowextsize`; rehearsed on scratch: the same 2,000-page workload leaves 112 KiB instead of
58 MB) so the reservations do not regrow — every future COW write on a snapshot-shared
page allocates exactly the page. Why the periodic GC never got to the live file is left
open (dirty-release flag or a busy-inode skip; the 03:32 restart's non-reclaim fits either);
the trim command above is the operator's answer if the disk census ever again shows the
live file's allocation a third above its size. Probe directory removed. Item 1 (122 GB of
apparent growth by table) stays the open question in this issue; the reflink ring's real
cost is now ~7 GB/week of divergence plus nothing.

**Tripwire built and baselined (deployed `a6cf472` 2026-09-06 15:5x UTC, job 775).** The
weekly `disk-census` now reads `st_blocks` beside `len()` and reports
`db_allocated_bytes`, the over-allocation in bytes and percent, and
`db_overallocation_alarm` above 5 % (and 1 GiB) with the reclaim command in the text.
First reading, right after the trim: file 604.0 GiB, allocated 604.6 GiB, **0.10 % over**
(the extent-tree overhead of 30.7 M extents), alarm null; volume 49.0 % used, 846.1 GiB
free — it read 73.9 % at 01:10 this morning. A restart (the deploy) left `df` unchanged at
866,405 MiB and the file's `cowextsize 4096` in place.

*2026-09-13:* the issue-361 hand sample during the weekly data-quality job read 0 `(deleted)`
descriptors in 15 of 15 readings and `df` unchanged at 61 %; the disk-census the same morning read
1005.2 GiB used (60.6 %), file 614.3 GiB allocated 0.15 % over, 0 unlinked-but-open, and a
two-point rate of −31.5 GiB/day against last Sunday (the r208 refold and the reflink trim between).

## 2026-09-19 12:0x — where 146 GB went in six days, and why the ring did not rotate (issue 420)

Free space 701 GB at the 09-13 census → 555 GB today (df), the database +13 GB (659.6 → 672.7 GB
apparent). The rest is the snapshot ring: `/data/db/snapshots` holds 09-03 and 09-06, each 648.5 GB
apparent and 649 GB ALLOCATED — the whole-corpus folds of 09-12..16 diverged both into full copies
(live file: 0 shared extents). The 09-13 weekly snapshot was skipped because the weekly data-quality
run (03:10 Berlin, 166 min that day) was still running at 05:23 — a scheduling collision, filed and
fixed as issue 420 (the gate now waits for the queue). The price this record already stated on
08-28 holds and is now measured twice: a held reflink snapshot costs full size across any
whole-corpus rewrite, and a ring that fails to rotate keeps paying it. The disk census's
`bytes_per_day` (-33.9 GB/day on 09-13) is the reclaim's artefact, not a trend, as its own caveat says.
