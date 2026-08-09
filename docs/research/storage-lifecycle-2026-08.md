# Storage lifecycle at half a terabyte — capacity model for production

Research date: 2026-08-09. Closes research gap #3 (research-gaps-2026-08.md)
and issue 169. This is the study pilot-sizing.md and turso-scale.md both
flagged as missing (">10 GB behaviour", "turso at 100–300 GB") — production
ran the experiment for us; this doc reads the instruments and does the
arithmetic.

**Method.** Everything marked **[verified]** was read from the production box
on 2026-08-09 under docs/agents/prod-box-reads.md: filesystem metadata
(`df -B1`, `ls`, `du` on the archive/scratch trees, `stat`, `xfs_info`, one
`xfs_bmap` extent-map walk of the single DB inode, output bounded) and nine
bounded `/v1/sql` reads — all single SELECTs on small tables (`v_fetches`,
480 rows) or indexed/LIMITed point reads (`changes` by PK cursor, `notices`
by id). Every query completed well inside the 10 s cap; **no 408 was incurred
and nothing was retried**. Figures citing other docs are marked
[measured → doc]. Everything else is **[estimated]**, with the basis stated.

---

## 1. Current state, 2026-08-09 [verified]

| Component | Logical size | Allocated on disk | Notes |
|---|---|---|---|
| `/data/db/tender-db.db` | **459,689,758,720 B (459.7 GB)** | **601,213,292,544 B (601.2 GB)** | mtime 2026-08-08 07:38 (daily run); 7,001,517 extents |
| `tender-db.db-wal` | 1.2 MB | 1.2 MB | auto-checkpointing keeps it trivial |
| `/data/archive` | — | 191.4 GB (`ted/` 176 GiB, `doe/` 3.3 GiB) | `SUM(bytes)` over all 480 `fetches` rows = 191,144,197,531 B — matches to 0.15 % |
| `/data/swapfile` | 17.2 GB | 17.2 GB | created 2026-07-23 |
| `/data/scratch-lots` | — | 12.3 GB | Aug-03 test DBs (rig.db 4.9 GB, probe.db 4.4 GB, tenders.db 2.1 GB, orgs.db 0.9 GB) — issue-115-era leftovers, deletable on owner ack |
| `/data/tmp`, snapshots dirs | ~0 | ~0 | both snapshot dirs empty (feature removed 2026-08-06) |
| **`/data` volume** | 1,073,479,680,000 B | **used 838,435,590,144 B (78.1 %), free 235,044,089,856 B (235.0 GB)** | XFS, reflink=1 |
| **`/` root disk** | 80.3 GB | used 55.3 GB (72 %), free 21.7 GB | app/nix-store only; not a data-growth surface |

Component sum (601.2 + 191.4 + 17.2 + 12.3) = 822.1 GB vs 838.4 GB df-used;
the ~16 GB residue is XFS metadata/log/per-AG reserves [estimated].

### 1.1 The DB file occupies 141.5 GB more than it contains [verified]

`stat` reports 1,174,244,712 × 512 B blocks = 601.2 GB against a 459.7 GB
logical size. The extent map shows the data fork ends **exactly at EOF**
(last extent closes at sector 897,831,560 = the file size), so the extra
141.5 GB is not beyond-EOF preallocation — by elimination it sits in the
inode's **COW fork**: copy-on-write staging left over from the reflink
snapshot era (snapshots were `cp --reflink` copies; checkpoint-time page
overwrites of shared extents went through COW staging with speculative
preallocation; the snapshots were deleted 2026-08-06 but the staging stays
pinned while the inode is busy — and this inode is held open by the service
permanently). Mechanism attribution is [estimated]; the arithmetic is
[verified]. Since no shared extents remain, **new** COW staging should no
longer accumulate — worth re-checking `stat` monthly.

This is potentially **+141.5 GB of free capacity (+13 % of the volume) for
the price of a quiet-window service restart** (inode eviction triggers XFS
cowblocks reclaim). Unproven — filed as the first open question.

### 1.2 Reconciling "87 % full" (08-07) with 78 % today

Both readings are honest: the handover's ~87 % (≈ 934 GB used) was taken
during the #39 reprocess window, when `/data/tmp` held fold sort-spill
(the 14 h fold spills tens of GB; `/data/tmp` is ~100 KB today) and COW
churn was at its peak. The ~96 GB delta is transient working set, not
reclaimed baseline [estimated]. Lesson for the model: **planning must
reserve ~50 GB of transient headroom for fold spills** — the operative
ceiling is below the nominal one. Likewise the handover's "DB ~455–558 GB"
range is now legible: ~455 GB was the apparent size, ~558 GB the allocated
size of the day.

---

## 2. Growth model

### 2.1 Archive: ~5.5–6 GB/yr [verified]

From `fetches` (480 rows — the cheap, metadata-only read the issue promised):

- Backfill (done, one-time): 401 TED monthly packages = 187.35 GB + 44 DÖE
  monthly = 3.43 GB. The 2004–2010 packages are anomalously fat
  (9.9–25.8 GB/yr — the per-language duplicate era); 2011+ runs
  1.6–2.2 GB/yr rising to 3.8 GB (2024).
- Steady state (dailies since 2026-07-19): TED ≈ 17.3–20.1 MB/issue
  (~17.5 MB recent, 254 issues/yr → **~4.4 GB/yr**), DÖE ≈ 0.3–9.3 MB/day
  (~3.5 MB avg → **~1.2 GB/yr**).
- Cross-check by data-year: TED 2025 (11 pkgs) → ~4.2 GB/yr rate, 2026
  (6 pkgs) → ~4.5 GB/yr; DÖE 2025 = 1.12 GB, 2026 → ~1.1 GB/yr. Converges.

Archive growth ≈ **6 GB/yr** and is not the problem.

### 2.2 Notice volume: ~1.15–1.25 M/yr [verified, two instruments]

Recent daily runs inserted ~3,000–5,000 live notice rows each (probed via
bounded `ORDER BY id DESC LIMIT 1 OFFSET n` reads: the 08-08 run ≈ 3,000
rows, 08-07 ≈ 5,000). Package arithmetic agrees: TED ~17.5 MB × 10.5×
compression / 53.1 KB/notice ≈ 3,460/issue [measured → pilot-sizing];
DÖE ≈ 1,100/day [measured → upstream-drift]. Yearly: TED ~0.88–0.94 M +
DÖE ~0.27–0.3 M.

### 2.3 DB file: no size history exists — bounded honestly

The only dated anchors: 2026-07-22 /data at 34 % mid-backfill (issue 23);
parsed layer ~254 GB at recovery time [measured → ADR-0009]; "~455–558 GB"
on 08-07 (handover); **459.7 GB logical today** [verified]. Too thin for a
regression, and the 08-01→08-08 window is polluted by recovery folds. So
the forward rate is built per-notice instead:

- Corpus-average all-in cost today: 459.7 GB / ~14.2 M parsed notices =
  **32.4 KB/notice** — every layer included (parsed satellites, canonical,
  changes, quarantine, indexes, internal free pages) [verified arithmetic,
  count from handover].
- pilot-sizing's notice-layer prediction for the same mix is ~165 GB →
  the observed all-layers multiplier is 459.7/165 ≈ **2.8× the notice
  layer** (pilot assumed 1.7–2.0; reality is fatter — canonical layer +
  change log + extra indexes + B-tree slack) [estimated].
- New intake is eForms-era: 22.3 KB/notice notice-layer [measured →
  pilot-sizing] × 2.8 ≈ **~62 KB/notice all-in**; DÖE ≈ 5.3 × 2.8 ≈ 15 KB
  [estimated].

Yearly DB growth ≈ TED 0.88–0.94 M × 62 KB + DÖE 0.27–0.3 M × 15 KB ≈
**~60 GB/yr, band 45–70 GB/yr** [estimated]. Sanity check: ~4.5 k
notices/weekday × ~45–60 KB ≈ 200–270 MB/weekday ≈ 1–1.4 GB/wk — the same
band from the other end.

### 2.4 Combined steady state

**/data grows ~50–75 GB/yr (midpoint ~65), of which the DB is ~90 %.**
Excluded and unbudgeted: user-state tables at launch (MBs), a native-FTS
decision under C14 (could be tens of GB — decide with this model in hand),
and long-run B-tree slack beyond today's observed multiplier.

---

## 3. The change-log arithmetic (C17: "kept forever")

Measured from the live `changes` table by PK point-reads (no scans):

- Max cursor **95,770,362** (2026-08-08 07:35); min cursor **1** stamped
  2026-07-30 20:33:59 — confirming the recovery rebuild ran with
  `clear_changes` and this is one clean generation [verified].
- **The full-corpus rebuild generation is ~87 M rows** (every cursor from 1
  to ~87.0–87.5 M carries the same 07-30 20:33:59 stamp) — **10.7 change
  rows per tender** across 8,140,205 tenders (max id, [verified]). Note:
  ADR-0009 predicted "~50–60 M" — actual is ~45 % higher.
- Bulk reclaim folds 08-01→08-06 added ~8 M rows; the clean dailies
  (08-07, 08-08) emitted **~20–50 k rows/weekday** [verified, 2-day window
  — treat as ±2×].
- Row cost ≈ 65–95 B including the `changes_entity` index [estimated from
  the STRICT schema]: a rebuild generation ≈ **6–8 GB**; steady state ≈
  **0.6–1.2 GB/yr**.

The arithmetic of the promise:

- **Steady state, "forever" is affordable**: ~1 GB/yr; thirty years ≈ 30 GB.
- **Across ADR-0009 rebuilds it is not**: post-launch, `clear_changes` is
  off the table (subscribers hold cursors — issues 46/163/164), so every
  rebuild appends a full ~87 M-row generation that supersedes everything
  before it. At 2 rebuilds/yr that is +14–24 GB/yr — the single largest
  *controllable* growth component (~25–35 % of total) — and after three
  rebuilds >95 % of the log is dead generations that a `since=0` replay
  must wade through (issue 46's non-convergence, in bytes).

---

## 4. The compaction dead end and the escape hatch

**The DB file can never shrink in place.** `VACUUM INTO` OOM-kills the
7.6 GiB box at 10 GB database size, memory scaling with DB size
[measured → turso-scale §1]; at 459.7 GB it is hopeless by ~60×, and the
upstream fix has not landed as of the 0.7.2 audit [verified →
upstream-drift §2]. `DELETE` (e.g. C17 pruning) frees pages only into the
internal freelist: future growth reuses them, the file's high-water mark
stands. Compaction and any schema rewrite at this scale mean
**dump-and-reload**, never measured. Bounding it from measured rates:

| Phase | Basis | Estimate |
|---|---|---|
| Dump | never measured; sequential read floor 490 MB/s [measured → turso-scale], realistic B-tree walk + SQL text, single-threaded | 2–6 h [estimated] |
| Reload, indexes-on | 11.3 k rows/s ≈ 38 MB/s [measured → turso-scale] over 459.7 GB | ~3.4 h floor |
| Reload, load-then-index | ~160 MB/s [measured] → ~0.8 h + index builds. Bench says 13 s/M rows/index at 10 GB; production reality was **~360 s/M rows** for one 8.1 M-row index at current scale (issue 82's 49-min boot build, cold + sort spill) — 27× worse | index phase 4 h–30 h [estimated] |
| Verify | integrity_check 7m19s at 10 GB [measured] → ~5.6 h at 460 GB, + row-count compare | ~6 h |

**Total: roughly 0.5–2 days; plan for 1–2 days** [estimated]. Two harder
facts than the wall-clock:

1. **On-box, it doesn't fit.** A reload target needs ~460 GB+; `/data` has
   235 GB free. The only on-box shape is: dump compressed (~50–90 GB
   [estimated, zstd on SQL text]) → **delete the live DB** → reload — a
   1–2-day full outage during which the dump is the only copy of five
   weeks of processing (DR = re-ingest, "weeks" per issue 23). That is an
   ops crisis, not a procedure.
2. **With a temporary second volume it degrades gracefully**: pause writes,
   dump (2–6 h, readers still served), reload+index+verify on the new
   volume off the serving path, cutover on a restart. Read downtime
   ~minutes; one or two missed daily ingests.

Also honest: no dump tool exists today. The file is sqlite3-CLI-readable
[measured → turso-scale], but the turso-only rule forbids that path, so the
escape hatch requires either an in-app dump job or an owner waiver —
neither exists yet. The hatch is currently **notional**.

Turso behaviour at 0.5 TB, incidentally established by production in place
of the never-run 40–100 GB bench: indexed point reads stay fast (ms);
full scans take 40 s+ and are uninterruptible (issues 82/120); `CREATE
INDEX` is ~27× slower than the 10 GB bench; a 59 k-tender fold ran 14 h at
~10 MB/s cold reads (handover §4). Serving works; every maintenance
operation is an order of magnitude past its bench figure.

---

## 5. The capacity model — dated

Baseline 2026-08-09: used 838.4 GB, free 235.0 GB, deep-health degrades
(503) at 90 % = 966.1 GB used → **127.7 GB to the operative ceiling,
235.0 GB to hard-full** — minus ~50 GB that must stay free for fold spill
(§1.2), so the *comfortable* runway is ~78 GB to the point where rebuilds
stop being safe to run.

| Scenario | Growth rate | Hits 90 % (service degrades) | Hits 100 % |
|---|---|---|---|
| (a) steady state | 50–75 GB/yr | **2028-04 → 2029-02** (mid 2028-08) | 2029-09 → 2031-04 (mid 2030-03) |
| (b) + 2 rebuilds/yr | 64–99 GB/yr | **2027-11 → 2028-08** | 2028-12 → 2030-04 |
| (b) + 4 rebuilds/yr | 78–123 GB/yr | **2027-09 → 2028-04** | 2028-07 → 2029-08 |
| any + COW fossil reclaimed (§1.1) | same − 141.5 GB used | all dates shift **+1.2 to +2.8 yr** | likewise |

All [estimated] on §2's rates; endpoints are the honest band, not
precision. Headline: **nothing is on fire — the earliest credible 90 %
breach is late 2027 and only under a routine-rebuild cadence with no
pruning; steady state alone reaches the ceiling around mid/late 2028.**
But the two structural facts (a file that can never shrink; an escape
hatch that needs space the volume doesn't have) mean the response has to
be decided *before* the watermark, not at it.

### Recommendations

1. **C17 retention trigger (the deliverable the issue asked for).** Adopt
   epoch semantics for rebuilds (the issue-46/163/164 protocol): each
   rebuild opens a new epoch; change rows of superseded epochs become
   prunable once no registered cursor/webhook watermark points into them,
   or after a documented grace window (e.g. 90 days, then the existing
   410/reset path). **Trigger: prune superseded-epoch change rows when
   /data crosses 85 % (912 GB used) OR `changes` exceeds ~175 M rows
   (≈ two stacked generations), whichever first.** Expected yield: 6–8 GB
   per superseded generation, reused in place. Keep "forever" for the
   current epoch only — at ~1 GB/yr that promise is cheap; it is only
   forever-across-rebuilds that was never affordable.
2. **Volume growth: no order needed today; wire the trigger now.** The
   1 TB volume was grown online once already (2026-07-22,
   `xfs_growfs`, zero downtime), so lead time is a console click, not
   procurement. **Grow to 2 TB when /data crosses 85 %** (projected
   2027-Q3 at the earliest, mid-2028 steady state) — cost order of
   +€26–52/mo at Hetzner's ~€0.05/GB/mo (pricing unverified since July).
   Diskwatch already exists; add the 85 % line to it.
3. **Run the COW-reclaim experiment** at the next quiet window: record
   `stat` blocks, restart the service, re-record. Potential +141.5 GB
   (+13 % capacity) for free; result also decides whether the §5 table's
   last row applies.
4. **Make the escape hatch real before it is needed**: an in-app dump job
   (turso-only-compliant), and the standing decision that dump-and-reload
   happens onto a *temporary second volume* (§4.2 shape) — never the
   delete-after-dump gamble. A paper drill of the §4 numbers against a
   40–100 GB scratch DB would also finally close turso-scale's un-run
   bench at the same time.
5. **Housekeeping** (owner ack, ~30 GB): delete `/data/scratch-lots`
   (12.3 GB of Aug-03 rig DBs); reconsider the 17.2 GB swapfile's
   residence on the data volume.

---

## Implications for tender-db

1. The disk-arithmetic story arc closes: pilot-sizing said "buy 500 GB",
   reality bought 1 TB, and at measured rates 1 TB carries steady state
   to ~2028 and hard-full to ~2030 — **the volume is not the risk; the
   rebuild cadence and the un-shrinkable file are.**
2. C17's "kept forever" splits cleanly: forever-within-epoch is ~1 GB/yr
   and fine; forever-across-rebuilds costs ~7 GB per rebuild and poisons
   `since=0` replays — the retention trigger above resolves both with one
   mechanism, and it is the same epoch protocol issue 46 already wants
   for correctness.
3. Every maintenance operation at 460 GB runs an order of magnitude past
   its 10 GB bench figure (index builds 27×, folds at 10 MB/s). Any plan
   whose step reads "then we rebuild/reindex/dump" must use §4's numbers,
   not turso-scale's.
4. The physical file is 30 % bigger than its logical content (§1.1) —
   capacity monitoring must watch **allocated blocks** (`stat`/df), not
   the file size the app reports.

## Open questions

**Needs research**
- Does a service restart reclaim the 141.5 GB COW fossil? (One quiet-window
  restart + `stat`; also re-check monthly that no new staging accumulates.)
- Actual `changes` bytes/row incl. index — needs `dbstat` or an off-box
  copy; the 65–95 B band brackets the C17 arithmetic but was not measured.
- The 62 KB/notice all-in eForms cost is inferred from one corpus-level
  ratio (2.8×); a per-table decomposition (off-box) would pin the growth
  band to ±10 %.
- The diskwatch journal may hold a df time series that would replace §2.3's
  per-notice model with a measured curve — someone with journal access
  should pull it (bounded `--since`), one hour of work.
- Dump wall-clock has no measured component at all; the §4 table's dump and
  index phases dominate its uncertainty. A 40–100 GB scratch drill closes
  both this and turso-scale's open bench item.
- Do monthly packages continue to be fetched as months close (dailies then
  double-stored)? Trivial bytes (~0.4 GB/mo) but the fetch policy should be
  stated somewhere.

**Needs user decision**
- The C17 trigger criterion and grace window (recommendation 1) — this is
  the reserved pruning path getting its trigger; it changes the public feed
  contract and belongs next to the issue-46 epoch decision.
- Second-volume-at-dump-time as standing DR/migration policy, and whether
  an in-app dump job gets built (or the turso-only rule waived for DR).
- Housekeeping deletions (scratch-lots, swapfile residence).
