# 169 — storage lifecycle model at 0.5 TB and beyond

Status: open — URGENCY RESTORED 2026-09-01. The 2026-08-15 downgrade rested on
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
