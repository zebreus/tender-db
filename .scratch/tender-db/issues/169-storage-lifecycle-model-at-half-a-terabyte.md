# 169 — storage lifecycle model at 0.5 TB and beyond

Status: open — research gap #3; urgency DOWNGRADED 2026-08-15 (measured: /data at 39%, 1.1T free — the "~87% full" premise no longer holds; lifecycle model still worth writing before the corpus doubles)

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
