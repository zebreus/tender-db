# Refold diagnosis, 2026-10-08 (wf_805a60a7-d9b)

## Synthesis

**Why the refold is slow, and whether to defrag**

No, don't defrag. The fragmentation hasn't come back, and the filesystem now takes under 1 % of the fold's time. The refold is slow for two reasons. It rewrites the entire corpus through one writer thread. And it spends about 4.4 h re-planning before it starts writing.

## 1. Where the time goes

The full job is about 11 h. Times are for the last identical refold, job 2044, finished 2026-10-07:

| Stage | Time |
|---|---|
| Requeue and stamp | 48 min |
| Plan | 2 h 48 m (10,085 s) |
| Grouping | 25 min |
| Pre-pass | 27 min |
| **Folding** | **6 h 35 m (60 %)** |

**What the folding does.** The epoch bump marks every Tender stale, so all 8.78 M are deleted and re-inserted in full:
- 1.118 B leaf rows, each deleted and inserted again.
- About 1.75 index entries per row, about 0.75 of them at random positions. That comes to about 6 B B-tree changes, about 1.7 B of them random.
- 70.6 M change rows, because the whole history is re-emitted.
- About 340 M statements, roughly 40 per Tender, each re-parsed (not prepared once and reused).
- About 46 k commits, each with a WAL fsync. turso 0.7.2 checkpoints automatically at a hard-coded 1,000 frames, and one commit is about 18 k pages, so every commit also copies its pages into the DB file. Every page is written twice.

More than 95 % of what gets rewritten is byte-identical to what it replaces.

**The writer thread is the bottleneck.** The fold logic already runs on a separate thread (issue 175) and uses 0.025 cores. The writer thread is on the CPU 84.5 % of the time and blocked 15.4 %:
- **CPU:** 88 % is user time, 9–12 % kernel. perf puts 86.6 % of samples in the server binary, 8.7 % in the kernel and 4.7 % in libc.
- **Blocked:** about 1,670 disk reads a second at 0.09 ms each, which accounts for the 15 %. These are mostly first-time reads of the old rows it is deleting.
- **Disk:** 12–23 % busy, IO pressure about 7 %. fsync is cheap: the drives have power-loss protection and the device flush counter is 0.
- **Rate:** about 17.5 µs per leaf row plus about 0.41 ms per Tender, which is 55 k rows/s. The model predicts 22,850 s against 23,674 s actual.

**What we don't know yet.** The binary has no symbols, so I can't split the 17.5 µs between turso's B-tree and index upkeep and statement overhead. "40–60 % random index upkeep" is a guess.

Job 2067 is tracking job 2044 within about a minute at every checkpoint. Folding should end around 16:15–16:45 UTC (18:15–18:45 CEST).

## 2. Defrag now? No

- **Extent count:** 1,947,070 now against 1,946,893 right after the 10-06 defrag. That is 177 more while the file grew about 10 GiB. Snapshots are off, so nothing is re-fragmenting it.
- **Profile:** the extent lookup (`xfs_iext_lookup_extent`) is 0.12 %, down from 9.2 %. The whole kernel share is 8.7 %, down from 75.7 %. Even removing the entire kernel share would gain under 9 %, and the extent-related part is under 1 %.
- **Free space is healthy:** 773 GB free, largest free runs 64.6 GiB and 37.9 GiB.
- **History:** the defrag cut extents 47× but made the same fold only 4.6 % faster (414 → 395 min; the whole job 2.9 % faster).
  - Issue 488's "~10 %" rests on a misread end time. The journal shows job 2031 finished at 04:53:46 (17,988.9 s), not 04:13.
  - Issue 488's 75.7 % kernel sample doesn't fit the timings either: a 47× cut would have done far more than 4.6 %. The fold was never mostly limited by extent lookups.

## 3. What would make refolds faster, ranked

1. **Don't rewrite content that hasn't changed.** Store a digest per version (or per version and table) and skip unchanged rows. A variant that reads and compares before deleting needs no schema change.
   - Gain: for refolds like 484 or 490, folding drops from 6.6 h to about 0.5–1.5 h (about 2–3 h for the read-and-compare variant). That saves about 5 h of the 11.
   - Cost: several days of work. The digest version needs a new column, and the first refold after it ships still pays full price.
   - Risk: medium. A wrong "unchanged" leaves stale content without any error. The digest must cover the derived EUR columns and the reused ids.
   - It also stops the 70.6 M re-emitted change rows per refold. That changes documented feed behaviour (issue 179; operations.md:929–931), so it needs your decision.
2. **Skip re-planning when a fix doesn't change grouping.** This is issue 58 v2, the open planning half of issue 179.
   - Gain: most of the 4.4 h before folding, roughly 3 h.
   - Cost: large. Risk: medium to high, because grouping must stay correct.
   - Cheaper first step: look into two regressions that hit every full fallback.
     - The grouping step `group step keyed/island` went from 72 s to 1,374 s between 09-28 and 10-04, and no issue covers it.
     - Planning went from 5,927 s to 10,085 s since August, while notices grew only 4.6 %.
   - Gain about 20–90 min, a few hours of work, low risk.
3. **Prepare statements once.** That covers the 14 DELETEs, the change INSERT and the identity, chain and sweep SELECTs. Also write change rows as multi-row INSERTs, and move values instead of cloning them in `flush_rows`.
   - Gain 5–15 % of folding (20–60 min). About a day of work.
   - Low risk: output stays byte-identical and the existing suites check it.
4. **Profile with symbols** (issue 488's open step 4). This speeds nothing up, but it shows where the 17.5 µs per row goes and so how much lever 3 is worth.
   - Build the deployed rev 9e2e80d with symbols kept and read a fresh perf sample of the running binary against it. The addresses should match if the build is otherwise identical; that needs checking.
   - No restart and no disturbance to the fold.
5. **Give the writer a bigger page cache.** Gain is at most the 15 % blocked time, and probably much less, since the misses look like first-time reads. It needs a small code change: raising `TENDER_CACHE_KIB` raises it for every connection, including the 31 pre-pass readers, which risks running out of memory. Unmeasured.
6. **Smaller items:**
   - turso 0.8.1 (issue 457): write-speed gain unknown.
   - Bigger batches: at most 2–3 %, and they bring back the issue-63 WAL growth risk.
7. **Not worth doing:**
   - Defrag.
   - `synchronous=NORMAL`: issue 458 rejected it for torn-database risk, and fsync is cheap here anyway.
   - Changing the checkpoint threshold: turso 0.7.2 hard-codes it.
   - Changing `page_size`: needs a VACUUM, which runs out of memory at this size.
   - Parallel fold: already done; the writer is the bottleneck.

**Without code changes:**
- The only knob is `rebuild=true`, and you shouldn't use it.
  - It reissues tender ids, so every mirror resyncs, and the API serves a partial layer for hours.
  - `tender_version_classifications` has about 278 M rows, over the 240 M cap for building its index automatically. The rebuild would refuse that index, and it would then be built at the next boot with the API down (the issue-205 pattern).
  - Issue 179 rejected it permanently.
- The only free gain is running all-profile refolds less often. There have been three this week, about 31 h of fold time and about 210 M change rows.

**Recommendation:** make lever 1 the real fix and start designing it now; the change-feed question is part of it. Do lever 4 alongside, because it costs nothing and tells us whether lever 3 is worth a day.

## 4. Unexpected findings and risks

- **Other writes wait up to 11 minutes.** The fold holds the writer lock for a whole 50 k-notice batch (canonical.rs:15217). Another writer logged waits of 23–669 s. Anything with a timeout under about 11 min will fail during a refold. I don't know which writer that is.
- **Checkpoints during planning do nothing.** All 1,489 per-chunk TRUNCATE checkpoints in job 2067's planning returned busy and reclaimed nothing. The WAL was 2.2 GB after planning, the same in job 2044. That is harmless at this size, but the checkpoints are achieving nothing.
- **Planning time matched to the decimal:** 10,085.0 s in both job 2044 and job 2067. The timer is a fresh clock each run (project.rs:2133), so it is probably a coincidence.
- **Errors in the records:**
  - Job 2044 finished at 18:56 UTC, not 18:16. That is 10 h 18 m for the project job, 11 h 06 m including the refold job.
  - The job-log row number is about the job id + 909, and the issues mix the two. For example, "2953" is the log row of job 2044.
- **Probes wrote to the box.** One investigator briefly wrote and then deleted six small temp files in `/tmp`. Another wrote a perf data file to `/tmp` and deleted it. Nothing touched the DB file, the service or the job queue.

Raw data (heartbeats, journal phase lines, fit scripts) is in `/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/`; the main files are `hb.txt`, `fit.py`, `seg.py`, `p2_*.txt` and `jobs2000.json`.

## Adversarial check (corrections take precedence over the synthesis)

**Verdict:** the defrag answer ("no") holds, and the evidence for it is stronger than the draft says. The top lever (skip rewriting unchanged content) is the right direction, but the draft overstates its gain and understates what it does to the change feed. The page-cache lever rests on a wrong mechanism, and the draft misses a cheaper alternative that the codebase has already used. Everything below was checked against the raw journal files in the scratchpad, `jobs2000.json`, the repo at `fa1e958`, and the turso_core 0.7.2 source.

## Corrections

**1. Lever 5 (bigger page cache): the mechanism is wrong, and there is a real finding behind it.**
- Every explicit checkpoint in turso 0.7.2 throws away the writer's whole page cache.
  - `checkpoint_on` issues `PRAGMA wal_checkpoint(TRUNCATE)` (`crates/store/src/checkpoint.rs:79`).
  - turso runs that through `pager.checkpoint(mode, sync, true)` (`vdbe/execute.rs:659`). The third argument is `clear_page_cache`. The `Finalize` step in `storage/pager.rs` then clears the cache, and its comment says "explicit checkpoints do this, auto-checkpoint does not".
- `apply_tenders` calls it every 32 batches (`canonical.rs` ~15278), which is every 16,384 Tenders: roughly every 140 s on heavy buckets and every ~11 s on light ones. `project.rs` calls it again between buckets.
- So "misses look like first-time reads" is not supported; some of them are re-reads after the cache was wiped.
- "Gain is at most the 15 % blocked time" is also wrong. Of the ~11k page reads a second, only ~1.67k reach the disk. The rest are served from the OS page cache, and they cost CPU (syscall, copy, turso loading the page), not blocked time.
- A larger `TENDER_CACHE_KIB` would be thrown away every 32 batches unless the in-fold TRUNCATE is dropped or made rarer. The automatic checkpoint already copies each commit's pages into the DB file, so testing that comes first.
- Memory: 2002 and 2044 already peaked at 30.2 GB and 31.0 GB RSS (journal "peak RSS 30990 MB") on the 62 GB box.

**2. Blocked time: "1,670 reads/s × 0.09 ms accounts for the 15 %; mostly first-time reads of old rows" is not supported.**
- The arithmetic matches only by coincidence. Most of the thread's disk-wait samples had no wait-site label. The labelled ones were on the write side: 4 in writeback throttling (`rq_qos_wait`) and 2 waiting on a page.
- So fsync and writeback are part of the 15 %. Which pages are being read is unknown; code-cost's own guess is random-key index pages, which is the opposite claim.
- Correct statement: at most 15.4 % of wall time is reads and fsync combined.

**3. "Removing the entire kernel share would gain under 9 %" understates the ceiling.**
- perf's 8.7 % counts only on-CPU samples. `/proc` shows 12 % system time, and the thread is also blocked 15.4 % of wall time.
- The ceiling for any storage or kernel change is therefore about 25 % of wall time.
- The extent lookup is still 0.12 %, so the defrag verdict stands.

**4. "fsync is cheap / 1–2 %" is not established.**
- A device flush counter of 0 only means no cache-flush commands are sent. Each fsync still waits for the commit's ~80–115 MB of WAL writes and the checkpoint's DB-file writes to complete.
- Issue 458's measurement only shows the cost is within ±15 % noise. The real bound is "at most 15 %".
- Keeping `synchronous=FULL` is still correct.

**5. Lever 1 (skip unchanged content): needs a finer grain and a change-feed design.**
- **Grain.** A digest per version does not pay off for a 490-type change. 490 writes the new value columns on `tender_version_lots`, which sit on most versions (13.2M lots over 8.78M Tenders; about half the lots carry a figure). A large share of versions would still be rewritten in full. The 0.5–1.5 h estimate needs a digest per version and table.
- **Change feed.** Today, re-emitting the whole history is the only way feed consumers learn what a refold corrected. `append_version_changes` (`canonical.rs:29479`) compares new versions with each other, never with what was stored.
  - Skipping Tenders that did not change is safe.
  - Tenders that did change need a new "changed versus stored" emission. Without it, a 484-type fix (`is_buyer`) never reaches `/v1/changes` or SSE subscribers, and nobody sees an error.
  - This is a design decision about feed meaning, not just "70.6M fewer rows".
- **Read-and-compare variant.** It still reads ~1.1B rows through the same single writer thread, so the 2–3 h figure is unmeasured.

**6. The draft misses a cheaper route for fixes that only add or derive a column: a backfill that updates rows in place.**
- Precedent: `backfill-original-lang` (issue 340) walked 7,924,745 Tenders in 4,187 s (70 min), and that was with one commit per row.
- 490's lot value can be computed from stored rows alone: REST already computed it at read time through `summarise` and `elect_lot_value` (`seq <= ?`).
- A backfill in that style would have avoided the ~11 h refold: no delete and re-insert, no re-planning, and no 70.6M re-emitted change rows.
- The epoch-4 refold was a deliberate ops-review choice, because epoch 4 doubles as the completeness marker. But "the only free gain is refolding less often" should name this option.
- Scoped stamping (issue 179) already limits profile-scoped fixes to their own Tenders. This week's three all-profile refolds (479, 484/489, 490) were each chosen deliberately.

**7. "Parallel fold: already done" is only half right.**
- The prepare thread runs only reading, sorting and `fold_rows` (`project.rs` ~3490).
- These all run on the writer thread inside `apply_tenders`:
  - building rows in `write_version`;
  - `elect_lot_value` and `ScalePartners`;
  - diffing in `append_version_changes`;
  - formatting SQL strings with `format!`;
  - cloning values for `Pending`.
- Their share of the 86.6 % is unknown without symbols.
- The 0.025-core sample only shows that the prepare thread had finished its bucket and was parked on the zero-capacity channel (`sync_channel(0)`).

**8. ETA: drop "16:15".**
- At 1,013,368 Tenders, 2067 was 2h11m03s into folding and 2044 was 2h11m11s, 8 s apart.
- 2044 folded from 12:20:47 to 18:55:21 (6h34m34s).
- So 2067's folding should end about 16:44 UTC and the job about 16:45 UTC (18:45 CEST).

**9. Small wording and number errors.**
- **"31 h of fold time":** that is project-job time. Folding alone is about 20.5 h, and with the three ~48-min requeue jobs it is about 33.5 h. 2067 is still running, so its 70.6M change rows are a projection.
- **"~340M statements, each re-parsed":** overstated. The bulk INSERTs (`TenderInserts`) and the head UPDATE are prepared once. These are re-parsed:
  - the 14 `format!` DELETEs per version (`canonical.rs:15603–15627`);
  - `append_change` (`canonical.rs:31097`);
  - the sweep, identity and chain SELECTs;
  - the `mark_projected` UPDATEs.
- **Cost model:** "17.5 µs/row + 0.41 ms/Tender predicts 22,850 s" mixes two fits; 22,850 s matches 17.2 µs. The split is also fragile:
  - Fitting all 298 per-bucket deltas of 2044 (`fit.py` on `hb.txt`) gives 23–26 µs/row and a negative per-Tender term (R² 0.80).
  - Per-row cost ranges from 17.5 µs (text-heavy eForms) to 30–46 µs (legacy segments heavy in parties and classifications).
  - So the "≤15 % per-Tender share" that caps lever 3 is soft.
- **Pre-pass:** 29.4 min (2044) and 29.7 min (2067), not 27.
- **Pages per commit:** the WAL showed 20–28k frames per commit, not 18k.

**10. Lever 2 (skip re-planning).**
- **Grouping:** only the total time can be checked from the saved journals: 312.9 s (job 1616, 09-28) → 1,613.7 s (job 1974, 10-04) → 1,482 s / 1,468 s now. The "keyed/island 72 → 1,374 s" breakdown is not in the saved extracts.
- **Planning:** August's typical run was 5,213–5,357 s; only the first one was 5,927 s. That makes the regression about +80 min (+90 %), not +69 min.
- **"A few hours of work":** a guess, since the cause is unknown.
- **Missing argument:** small refolds also pay the planning tax. Job 2031 (refold-fields, 191k notices) took 5h05m because it fell back to the full corpus ("legacy closure exceeds cap (622004 > 500000)").

**11. rebuild=true: the draft's points are correct, with these additions.**
- It also drops and recreates `organizations` and `organization_mentions` (`strip_organization_indexes`), so organization ids are reissued as well.
- `operations.md:2278`: it "serves an empty/partial tender layer the whole time (ADR-0009)".
- `tender_version_classifications_code` is in the schema batch (`canonical.rs:364`), so it would be rebuilt at boot with the API down. At ~278M rows × 48 B that is about 13 GB, above the 12 GB `AUTO_INDEX_MEMORY_BUDGET`.
- The 278M is an estimate from issue 243's rowid-density sampling, not a count.

**12. "Other writes wait up to 11 minutes" is not a new risk.**
- It is the designed single-writer hold: `apply_tenders` takes `self.conn()` once for the whole ~50k-notice bucket (`canonical.rs` ~15217). This is the issue 241/256 class.
- `/v1` requests that need the writer are cut by the 30 s `request_deadline` (issue 241, gap 2) with a 503, not left hanging for 11 min.
- Today's daily jobs 2068–2070 are queued behind 2067, so today's ingest waits until about 16:45 UTC.

**13. Defrag evidence is stronger than the draft states.**
- Issue 488's sample (~22:00 UTC on 10-05) falls in fold 2002's buckets ~100–140. Over exactly that stretch, 2002 ran within 1–2 % of the post-defrag 2044 (segment ratio 1.01–1.02; whole run 0.99–1.13).
- The 75.7 % kernel time was not costing wall time.
- The 4.6 % overall gain also includes removing the reflink snapshot's copy-on-write (2002 ran with the 10-04 snapshot present) and the 484 deploy. So the defrag's own effect is at most 4.6 %.
- Worth telling the owner: a filesystem defrag cannot fix where B-tree pages sit inside the DB file. Only VACUUM or a rebuild would. The VACUUM-runs-out-of-memory evidence (turso-scale.md §1, issues 23/169) comes from the older, smaller box and has not been re-measured.

**14. Minor.**
- **Checkpoint threshold:** there is no pragma, but turso_core 0.7.2 has `Connection::wal_auto_actions_disable` (`connection.rs:2304`). The SDK is already vendored (issue 425), so it is reachable. It is low value while the disk is ~20 % busy.
- **Profiling with symbols:** sound. Check that the unstripped thin-LTO build of `9e2e80d` has identical `.text` and build-id before resolving perf addresses against it.

## Claims confirmed
- **No defrag.** 1,947,070 extents now against 1,946,893 after the defrag; `xfs_iext_lookup_extent` is 0.12 %.
- **Today matches 2044.** 2067 tracks 2044 bucket for bucket over the same 14,896,923 notices.
- **Planning time.** Both runs logged 10,085.0 s. The journal confirms each took 2h48m06s (06:27:19→09:15:25 and 08:38:36→11:26:42), so it is a coincidence, not a logging error.
- **2044's real timings.** Refold 07:50:22→08:38:24; project 08:38:24→18:56:23 (37,066.9 s). The "18:16" in issues 484 and 488 is a mid-fold heartbeat (7,861,955 Tenders folded at 18:16:27).
- **2031's real end.** Done at 04:53:46 (17,988.9 s), not 04:13:46.
- **Job numbering.** Log row = job id + 909 (2953↔2044, 2940↔2031, 2975↔2066).
- **Every page written twice.** The 1000-frame auto-checkpoint is hard-coded (`wal.rs:4620`, `3852–3855`), and the commit path triggers it.
- **Code shape.** 14 re-parsed DELETEs per version; `mark_projected` commits on its own, 512 ids at a time; the writer lock is held per bucket; a stale Tender is rewritten in full (`canonical.rs` ~15320).
- **2044 totals.** 1,117,908,878 leaf rows, 14,896,923 versions, 70,643,598 change rows. Cost scales with leaf rows, not Tenders.
- **Keep `synchronous=FULL`.** Issue 458's torn-database risk stands.
- **rebuild=true.** Issue 179's permanent rejection, the generation bump and mirror resync, and the 240M cap at `canonical.rs:10360` all check out.
- **Grouping regression.** Real (313 s → 1,614 s), and no issue covers it.

Files referenced:
- /home/user/tender-db/crates/store/src/checkpoint.rs
- /home/user/tender-db/crates/store/src/canonical.rs
- /home/user/tender-db/crates/ingest/src/project.rs
- /root/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turso_core-0.7.2/vdbe/execute.rs
- /root/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turso_core-0.7.2/storage/pager.rs
- /tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/p2_2002.txt, p2_2044.txt, p2_2067.txt, hb.txt, fit.py, seg.py, jobs2000.json
- /home/user/tender-db/.scratch/tender-db/issues/340-original-language-leg-of-the-lang-fallback.md
