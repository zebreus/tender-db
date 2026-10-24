# 496 — the fold writer's 17–46 µs per leaf row is unattributed, and its between-batch TRUNCATE wipes its page cache

Status: ready-for-agent — filed 2026-10-08 from the refold diagnosis (`wf_805a60a7-d9b`, `../495-refold/`).
NEXT: unit 1, a profile with symbols. It disturbs nothing, and it prices units 2 and 3.
Kind: performance / projection writer
Relates to: 495 (skip unchanged content, the big lever), 488 (step 4 there was never done), 42 (why the
in-fold TRUNCATE exists), 175 (the prepare/write split), 457 (turso 0.8.1)

## Verify

    grep -nE 'const CHECKPOINT_EVERY_BATCHES|&& let Err\(e\) = checkpoint_on\(&conn, CheckpointMode::' crates/store/src/canonical.rs

This reads unit 2 (the in-fold checkpoint cadence), the last unit whose done state shows in code. Unit 1 is a
measurement recorded in this file, and unit 3 waits on unit 1's pricing; most of it already landed with 495 unit 2
(see the 2026-10-24 comment at the bottom).
- **done**: the `apply_tenders` checkpoint line reads `CheckpointMode::Passive`, or it is gone (auto-checkpoint only),
  or `CHECKPOINT_EVERY_BATCHES` is raised. In each case unit 2's bounded-WAL test and the fold-bucket timing are
  recorded here.
- **open** (2026-10-24): two lines, `const CHECKPOINT_EVERY_BATCHES: usize = 32;` and `&& let Err(e) =
  checkpoint_on(&conn, CheckpointMode::Truncate).await` (canonical.rs 1849 and 24063 today). The fold writer still
  runs a TRUNCATE every 32 batches, and each one clears its own page cache.

## What is known

During a fold, the single writer thread (`apply_tenders`) uses 84.5 % of a core: 86.6 % of perf samples
are in the server binary, 8.7 % in the kernel and 4.7 % in libc. The rest of its time it is blocked
(15.4 %, reads plus fsync together).

The release binary is stripped, so the 17–46 µs per leaf row cannot be split between:

- turso's B-tree work and index upkeep;
- statement parsing;
- our own row building on the writer thread: `write_version`, `elect_lot_value`/`ScalePartners`, the
  `append_version_changes` diff, `format!`-built SQL, and cloning values into `Pending`.

The prepare thread runs only reading, sorting and `fold_rows`. The 0.025-core sample of it only shows
it parked on the zero-capacity channel.

**Finding: every explicit checkpoint clears the page cache of the connection that runs it.**

- In the fold, that is the writer's own cache.
- `apply_tenders` runs `checkpoint_on(&conn, Truncate)` every `CHECKPOINT_EVERY_BATCHES` (32) batches,
  at `canonical.rs` ~15278. That is roughly every 140 s on heavy buckets and every 11 s on light ones.
  `project.rs` checkpoints again between buckets.
- turso 0.7.2 handles `PRAGMA wal_checkpoint` through `pager.checkpoint(mode, sync, clear_page_cache:
  true)`. `storage/pager.rs:4905` says: "Clear page cache only if requested (explicit checkpoints do this,
  auto-checkpoint does not)".
- So the writer re-reads its hot index pages from the OS cache after every such checkpoint. That costs
  CPU (syscall, copy, page load), not blocked time.
- Raising `TENDER_CACHE_KIB` cannot help while this happens. It would also raise the cache of every
  connection, including the 31 pre-pass readers, and RSS already peaks at about 31 GB of 62.
- The auto-checkpoint (hard-coded at 1,000 frames, `wal.rs:4620`) already copies every commit's pages
  into the DB file. The TRUNCATE only shrinks the `-wal` file (issue 42).

**Statements re-parsed per version:**

- the 14 `format!` DELETEs (`canonical.rs` ~15603–15627);
- `append_change` (~31097);
- the sweep, identity and chain SELECTs;
- the `mark_projected` UPDATEs.

The bulk INSERTs and the head UPDATE are already prepared once.

## Units

1. **Profile with symbols.**
   - Build the deployed rev with symbols kept.
   - Confirm the `.text` section and build-id match the running binary.
   - Resolve a fresh `perf record -g` of the writer thread against it, during a fold.
   - Record the split across turso B-tree work, parse, our row building and checkpoint re-reads.
2. **Checkpoint cadence.** Measure a fold bucket with the in-fold TRUNCATE made rarer, or replaced by
   PASSIVE (which keeps the cache), while still bounding the `-wal` file as issue 42 requires. Test
   first, on a copy-sized fixture: does a large WAL stay bounded with only the auto-checkpoint?
3. **Prepare once**, if unit 1 prices it.
   - Covers the 14 DELETEs, `append_change`, and the identity, chain and sweep SELECTs.
   - Write change rows as multi-row INSERTs.
   - Move values instead of cloning them in `flush_rows`.
   - The output must stay byte-identical; the golden test (`project_golden.rs`) and the store suites
     check it.

## 2026-10-08 — the statement capture is still on, and it sits on the fold's writer

`/etc/systemd/system/tender-db.service.d/plancapture.conf` (issue 429 step 0) was meant to run "for one
week" from 2026-09-27. It is still live: `/data/tmp/plan-capture-429.sql` was 70 MB / 298,586 lines at
14:07 UTC today and still being appended to.

Its layer (`crates/app/src/plan_capture.rs`) runs this on EVERY turso `Preparing:` event, in this order:

1. a `format!("{value:?}")` of the whole SQL message;
2. a mutex;
3. a `HashSet` lookup.

The format runs BEFORE the `full` check, so the 20k-statement cap does not stop the cost. In phase 2 the
writer re-prepares about 40 statements per Tender (the 14 `format!` DELETEs per version among them), so a
corpus refold pays it about 340M times, on the bottleneck thread. That is an estimated 1–5 µs each, so
minutes per refold. Unmeasured, small next to the fold, but pure waste.

**Action:** remove the drop-in at issue 490's deploy-B restart (`systemctl daemon-reload` before the
restart). The capture file stays in place for 429's diff.

**Done 2026-10-08 20:21 UTC:** the drop-in is off (moved to `/root/plancapture.conf.removed-2026-10-08`); deploy B restarted without it.

## Comments

**2026-10-24, board hygiene: most of unit 3 landed with 495 unit 2.** Unit 3 is still open.
- 495 unit 2 was deployed on 2026-10-08 (~22:20 UTC, `79b1212`). It covers most of unit 3's list:
  - B2 `71a6c0c`: `Pending` is leaf-indexed and `flush_rows` moves values.
  - B3 `fc1b3dc`: the 14 per-version DELETEs, the stored-chain SELECT and the change-row INSERT are prepared once in
    `TenderInserts::prepare` (canonical.rs ~44878).
  - B4 `7e6da04`: a per-Tender identity cache. The lot, result, bid and contract lookups are prepared statements.
- What remains of unit 3:
  - the sweep SELECTs;
  - the `mark_projected` UPDATEs;
  - writing change rows as multi-row INSERTs. `APPEND_CHANGE_SQL` is still one row per statement.
- That remainder still waits on unit 1's pricing.
- Units 1 and 2 have no record of being done: no symbol profile is recorded, and the in-fold TRUNCATE is unchanged
  (see `## Verify`).
