# 63 — Adopt `plan_state` blob@Phase-1 for fast full rebuilds

Status: PARKED BY MEASUREMENT 2026-09-03 — the pre-pass is 8% of a full fold now (see the last section); was: scoped (proj-fix, 2026-07-29) — smaller than the design's ~1-2 day estimate; the
serialization machinery already exists. Concrete implementation plan below.

## Concrete implementation plan (proj-fix — grounded in the current code)

KEY FINDING: the blob machinery is ALREADY BUILT, so this is NOT a from-scratch
serialization job. `postcard` is an ingest dep; `BucketRow` (Serialize/Deserialize,
project.rs:1100), `NoticeState::read` + `bind_organizations`, and the pre-pass's
`NoticeState::read → bind → BucketRow::snapshot → postcard` are all in place
(`write_buckets_sharded`, project.rs:1000-1014). The pre-pass just re-reads the 254GB
parsed layer to build those blobs; this issue moves the build into Phase-1.

Steps:
1. **`plan_state(notice_id INTEGER PRIMARY KEY, blob BLOB)`** — plain rowid table.
   `reset_plan` must also clear it; `plan_is_complete`'s COUNT must cover it (so a
   complete plan ⇒ blobs exist — the resume-salvage invariant).
2. **Split `BucketRow` into `(group_key, StateRow)`** — the Phase-1 blob is the
   group_key-INDEPENDENT payload (`StateRow` = today's BucketRow minus `group_key`),
   because grouping runs AFTER Phase-1, so the group_key isn't known at write time.
   The pre-pass then joins `plan_state.blob` (StateRow) + `plan_notice.group_key`
   (both notice_id-PK-ordered) to reconstruct a BucketRow for bucketing.
3. **Phase-1 write** (`build_plan` chunk loop, project.rs:537-544): after
   `resolve_mentions` (whose returned ids are aligned to the input `&[Mention]`,
   each carrying notice_id+section_id — canonical.rs:1665), zip ids→`by_section`,
   `NoticeState::read(notice, parsed)` + `bind_organizations(by_section)`, serialize
   a StateRow, `insert` into `plan_state` in the SAME chunk txn as `insert_plan`.
4. **Pre-pass swap** (`write_buckets_sharded`): read `plan_state` (StateRow, ~25-50GB)
   sequentially instead of `parsed_chunk_on` (254GB), joined with `plan_group_keys_on`,
   → BucketRow → buckets. Phase-2 unchanged.
5. **Byte-identity gate:** project_golden + project_equivalence + project_resume +
   fold_source-invariance must stay byte-identical (the StateRow must round-trip to the
   exact NoticeState; org ids already resolve in the same id order in Phase-1).

Effort: ~half-to-full day of careful work — the risk is byte-identity of the StateRow
round-trip and the resume-invariant (plan_state completeness), not new machinery.

Refinement (lower churn): store the whole `BucketRow` in `plan_state` with `group_key=""`
(unknown at Phase-1 write time); the pre-pass overwrites `group_key` from `plan_notice`
before bucketing. `group_key` is only used for sort/bucket, never in `to_notice_state`,
so this reuses `BucketRow` AS-IS — no StateRow struct split, byte-identical.

### ⚠ REBUILD WAL-HARDENING (added 2026-07-30, from the finale-rebuild OOM) — in scope for this session

The 2026-07-30 `rebuild:true` over the full 14.2M corpus OOM-looped in Phase-1 (killed
before OOM; layer intact, uncommitted WAL discarded). Root cause: build_plan commits
per chunk and checkpoints TRUNCATE every 4 chunks, but on a LIVE box the coverage
refresher (60s scans) + /api + /health continuously hold WAL read-marks, so TRUNCATE
was perpetually busy → the WAL grew with every whole-corpus write (14.2M plan rows +
~28M org/mention rows) → 127GB, and turso's in-memory WAL-INDEX grew with it →
monotonic+accelerating swap → OOM trajectory. (org_of is bounded ~5-6GB and plateaus —
proven by the 300K incremental control which preloads the same cache and sat at ~3GB;
NOT the driver. The WAL-index was.) The 12.4M first build survived only because it ran
quiesced (swap band-aid + TENDER_DROP_JOBS, few live readers).

So a completing `rebuild:true` at 14.2M REQUIRES, in this session:
- **Run with `TENDER_DISABLE_COVERAGE` set** (removes the refresher's reader snapshots)
  AND ensure no other long reader snapshot is held during Phase-1.
- **A WAL-truncate strategy that reclaims under any incidental readers** — a FULL (not
  TRUNCATE) checkpoint reclaims WAL space without needing reader-exclusivity, or
  checkpoint more aggressively / gate the projection's readers. Without this the WAL
  grows unbounded regardless of the plan_state optimization.
- (OPTIONAL) bound the resolver (drop the all-orgs preload; DB-seek the dedup key via an
  index + bounded LRU) — only if a `COUNT(*) FROM organizations WHERE identifier IS NOT
  NULL` shows org_of is genuinely large (~tens of millions). Pending that measurement.
- **Re-emit a clean CDC baseline** — the feed is currently empty (the killed rebuild's
  clear_changes committed); the completing rebuild (clear_changes:true) re-emits it.

Note the plan_state piggyback ALONE does not fix this — it shrinks the pre-pass's 254GB
re-read but the WAL-under-readers blow-up is Phase-1's WRITE side. Both must be addressed
for a 14.2M rebuild to complete.

### ⚠ REVISED DIAGNOSIS (2026-07-30, after the coverage-OFF retry ALSO ballooned)

The coverage-off retry (`rebuild:true clear_changes`, `TENDER_DISABLE_COVERAGE` set) STILL
ballooned: WAL 1MB→3MB→647MB→2.16GB→3.5GB, ~+640MB/min, killed at 3.5GB. Coverage-off did
NOT hold it. That falsifies the "coverage refresher is the pin" root cause above, and forces
a revision. Two things are now established from the code + the retry data:

1. **The self-pin hypothesis (build_plan holds a read cursor across its writes) is FALSE.**
   `build_plan` reads each chunk via `parsed_chunk` on the READER POOL, fully drains it into
   a `Vec`, and drops the reader (returns it to the pool, autocommit → no snapshot per the
   `read.rs` Drop + the `an_idle_pooled_reader_does_not_pin_the_wal` test) BEFORE any write.
   `insert_plan`/`resolve_mentions`/`checkpoint` all run on the SINGLE writer conn. Read and
   write never overlap. Proof by control: `project_incremental_chunked` uses the IDENTICAL
   pattern (`parsed_by_ids`→drain→`insert_plan`+`resolve_mentions`) and its WAL stays sub-MB.
   The ONLY structural difference is cadence (rebuild TRUNCATEs every 4 chunks mid-loop; the
   incremental once at the end) and VOLUME (14.2M+28M rows vs a small delta).

2. **No continuous app reader exists in the code to pin the WAL with coverage off.** Audited
   every reader entry point: public API is memoized (never scans the store, `api.rs:22`),
   `/health` is DB-free (issue 61), the webhook sweeper drops its reader BEFORE the outbound
   POST and only ticks every 15s (`webhooks.rs:367`), the snapshot backup runs as a
   SERIALIZED supervisor job (one-job-at-a-time queue → cannot overlap the rebuild), and
   `heavy_write_in_progress` already gates coverage (its only consumer). So a reader-pin
   would have to be a RUNTIME/EXTERNAL reader not visible in code (a left-open admin SQL
   console, an SSE client, an external probe) — possible, but not the code's default state.

**Leading hypothesis now: checkpoint-THROUGHPUT divergence at 254GB, not a reader pin.**
The WAL's knee-shaped monotonic growth (small and controlled early, then 647MB→2.16GB→3.5GB
runaway) fits a positive-feedback loop: each TRUNCATE must backfill its WAL frames as random
writes into the 254GB main DB; as Phase-1 proceeds under IO/page-cache contention the
backfill slows, so more WAL accrues between the every-4-chunks checkpoints, so the next
TRUNCATE has more frames to backfill and is slower still → runaway. This elegantly explains
BOTH why coverage-off didn't help (the bottleneck is checkpoint IO throughput vs write rate,
not a reader) AND why the 12.4M build survived quiesced (no competing IO → backfill kept up)
AND why the incremental/process-loop stay bounded (small WAL → instant TRUNCATE). It is a
HYPOTHESIS, not yet confirmed.

**Decisive, zero-risk measurement to confirm the mechanism:** `build_plan`'s checkpoint
ALREADY computes `Checkpointed { busy, wal_frames, checkpointed }` and DISCARDS it
(`if let Err(e) = db.checkpoint(...)` at project.rs:550). Log it (+ any Err) at each
Phase-1 checkpoint. The next run reads the mechanism in the FIRST minute:
- `busy = true`, `checkpointed ≈ 0`, `wal_frames` climbing → a READER pins it → hunt the
  runtime reader (and/or add the checkpoint reader-gate below).
- `busy = false`, `checkpointed > 0` but WAL still grows → THROUGHPUT divergence → the
  checkpoint can't keep up; readers are irrelevant.
This is a 3-line change with zero failure risk; ship it before the next retry so the retry
is diagnostic even if it's also a fix attempt.

**Fix directions, keyed to the measurement:**
- THROUGHPUT branch (most likely): the real fix is to stop making Phase-1's checkpoint
  compete with the 254GB serving DB's IO. Options, in rough order of leverage: (a) build the
  rebuild into a SEPARATE DB file / its own WAL and atomically swap it in — the structural
  end-state this issue already points at; its WAL checkpoints without contending against the
  live 254GB main DB, and it has no concurrent readers; (b) 64K `page_size` on the rebuild DB
  (fewer WAL frames → smaller WAL-index in RAM → faster backfill — already a companion lever
  above); (c) cut Phase-1 write VOLUME — the ~28M org/mention re-inserts dominate; if a
  rebuild PRESERVED `organizations` (canonical org identity is deterministic) instead of
  clear+recreate, Phase-1 would write only ~14.2M plan rows, roughly halving the WAL (needs a
  correctness check that org identity is stable across a rebuild); (d) run quiesced (the
  proven 12.4M condition) — pause the webhook sweeper + any background IO under
  `heavy_write_in_progress`, and don't just rely on coverage-off.
- READER branch (if the measurement shows busy): add a WAL checkpoint reader-gate — an async
  RwLock every reader borrow takes `.read()` on and the periodic checkpoint takes `.write()`
  on, forcing a guaranteed reader-free instant for TRUNCATE to wrap, with a bounded-wait
  fallback to PASSIVE (so a genuinely long reader degrades safely, not deadlocks). Broad
  change (routes every reader entry point) — only worth it if a reader is actually the pin.

Either way, the plan_state piggyback (this issue's main body) is orthogonal — it speeds the
pre-pass, not Phase-1's write side. Do the measurement FIRST; do not blind-retry (every
rebuild pays the upfront `strip_tender_indexes` cost = ~30min synchronous index rebuild on
the next boot, so a retry is expensive to abort).

### STATUS (2026-07-30, proj-fix) — measurement + both fixes landed/staged

- **6b816bd** (LANDED): build_plan Phase-1 TRUNCATEs every chunk (`PLAN_CHECKPOINT_EVERY=1`),
  bounding the WAL + its in-RAM index (the OOM driver) regardless of write volume; the
  checkpoint result (busy/wal_frames/checkpointed), previously discarded, is now logged when
  it fails to fully reclaim → the next run self-diagnoses the mechanism in the first minute.
  This is the cheap throughput fix + the measurement, in one. Byte-identity gates green.
- **3ba2e36** (STAGED, default-OFF): opt-in WAL read-gate (`TENDER_WAL_READ_GATE`). Every
  pooled reader holds the shared side of a process-wide RwLock for its borrow; the Phase-1
  checkpoint takes the exclusive side (5s-timeout fallback to plain, deadlock-free) → a
  reader-free instant for TRUNCATE. Engaged ONLY if 6b816bd's log shows `busy=true` (reader
  pin). `None` unless the env is set, so the measurement run is uncontaminated.

### ⚠ 2nd BALLOON (2026-07-30) — turso writes per-row WAL for bulk DML (MEASURED)

6b816bd did NOT hold: WAL 4MB→3.8GB, +670MB/min, cursor 0, and — the key clue —
NO checkpoint self-diagnosis log. That means the ballooning write is NOT in
build_plan's per-chunk loop; it's a single whole-corpus statement elsewhere.

ROOT CAUSE, measured on a scratch DB: turso has no truncate optimisation — a
`DELETE FROM t` writes a WAL frame PER ROW (~240 B/row: DELETE of 100k rows = 24MB
WAL; `DROP TABLE` of the same = 36KB, 650x less). A single statement can't be
checkpointed mid-way, so EVERY whole-corpus DELETE/UPDATE/CREATE INDEX in the
rebuild balloons the in-RAM WAL-index to OOM, invisibly to the per-chunk log.

Structural (separate rebuild DB) does NOT help: the mechanism is per-row WAL +
WAL-index RAM, independent of readers/serving-IO, so an isolated DB OOMs identically.

FIXES LANDED (all byte-identity-gated: golden/equivalence/resume/fold_source):
- dbf57a5: clear_canonical stops DELETEing organizations/organization_mentions —
  strip_organization_indexes DROPs them right after (O(1)). + stage-boundary WAL
  probes (wal_bytes after teardown / Phase-1 / grouping).
- 2004eac: build_plan_groups batches its 3 whole-corpus writes — the keyed/island
  UPDATE (by notice_id range + TRUNCATE between), a TRUNCATE per legacy chunk, and
  a TRUNCATE after the fold-index build.
- 062761c: clear_canonical batches the `UPDATE notices SET projected=0` watermark
  reset by id range + TRUNCATE.
- 781cea9: clear_plan_on DROP+recreates the plan tables (single DDL home; reset_plan
  is now just clear_plan_on) instead of `DELETE FROM plan_notice` (~14M rows). This
  fires at BOTH reset_plan (Phase-1 start, over a partial plan a kill left — a strong
  candidate for the observed "3min, notices=0, no log" balloon) and clear_plan (end).
- 2d9aa63: TRUNCATE after ensure_changes_entity_cursor_index — on rebuild+clear_changes
  the changes table is fully re-emitted (~50-60M rows) so that index build is big, not
  the "still-small" one its comment assumed; its WAL was left as a tail.

Every whole-corpus DELETE/UPDATE that is non-empty in the current wiped-state run is
now bounded. Full sweep confirmed only two `DELETE FROM {table}` sites remain
unbounded (below), both empty in the wiped state.

RESIDUAL (not batchable):
- End-of-fold CREATE INDEXes (build_organization_indexes / build_tender_indexes):
  single statements over the full org/tender tables — a bounded-but-large WAL spike,
  reclaimed by the existing checkpoint-after (project.rs). Can't be batched; relies
  on the 16GB swap (the 12.4M build survived this). CONFIRM swap is on prod.
- reset_tender_layer's tender_version_* DELETEs: cheap NOW (tables empty in the
  wiped state) but would balloon on a rebuild over a POPULATED layer — future
  follow-up (convert to DROP+recreate), not needed for the current wiped restore.

### ⚠ THE DEADLOCK (2026-07-30) — the real balloon, found via the .diag.log

After the batching fixes, the rebuild STILL ballooned — but the .diag.log (c5694c5,
the channel that survives the worker-runtime→journald blindness) showed only
`projection start: rebuild=true resume=TRUE` and nothing else: the balloon fired
BEFORE the first stage probe, on the RESUME path.

Root cause: `reset_tender_layer` DROPs `tenders` (O(1) → the table 404s, which read as
"tender layer wiped") but DELETEd the tender_version_* / lots / bids / contracts /
lot_results tables — STILL FULL (~tens of millions of rows) from the original 12.4M
build. A DEADLOCK: every rebuild attempt's per-row DELETE balloons the WAL, is killed,
and the kill ROLLS BACK the DELETE → the satellite tables never clear → the next
attempt hits the identical balloon. `tenders` empty + satellites full is why it looked
"wiped" but kept ballooning. NOT resume-specific — the fresh path's `clear_canonical`
has the same un-batched DELETEs over the same full tables. (My earlier "empty in the
wiped state" residual note was WRONG on exactly this point.)

Fixes (byte-identity gated; all on branch, HEAD 3b983ea):
- c5694c5: the .diag.log channel (Db::log_diag) — WITHOUT it we'd still be blind, since
  the worker-runtime job's eprintln! never reached journald.
- 1f6ab90: generic Db::drop_and_recreate(table) — capture table+index DDL from
  sqlite_master, DROP (O(1) WAL), recreate. Used for the tender-content clears in BOTH
  reset_tender_layer and clear_canonical. + teardown sub-step probes.
- 3b983ea: TENDER_FORCE_FRESH_PLAN=1 env valve — drop the on-disk plan so resume=false
  and Phase-1 rebuilds from the current corpus (guards against a stale plan; resume is
  otherwise correct via plan_is_complete + faster).

NOT reader-pin: the gate-ON run (covers all HTTP + sweeper) still ballooned → confirmed
throughput/single-statement, never a reader. The gate (3ba2e36) stays default-off.

Decision tree for the instrumented restore run:
- silence + tiny WAL → throughput fixed by cadence (6b816bd) → prod restored in ~hours.
- `busy=true` → reader pin → set `TENDER_WAL_READ_GATE=1`, re-run with 3ba2e36.
- large `wal_frames` residual (not busy) → cadence didn't beat the backfill → structural
  separate-rebuild-DB escalation (below), NOT org-preservation (see next note).

**Org-preservation does NOT apply to the current wiped-layer restore:** `clear_canonical`
DELETEs organizations + organization_mentions (canonical.rs:1086-7), and the killed rebuild
left a PARTIAL/garbage org table (partial Phase-1, WAL replayed on boot) — so orgs MUST be
rebuilt from the clean parse layer this run; the ~28M org/mention writes are unavoidable.
6b816bd bounds the RAM despite that volume (per-chunk WAL, not cumulative). Org-preservation
stays a FUTURE optimization for a rebuild that starts from a CLEAN org table, and it's
correctness-sensitive (breaks byte-identity-to-from-scratch on org id VALUES → needs a
reframed "identical modulo a stable org-id remap" invariant + sign-off).

**Cheap-kill note (index strip):** deferring `strip_tender_indexes` does NOT make Phase-1
kills cheap — tenders are wiped to 0 rows so their index rebuild on boot is instant. The
~49min boot cost is `organization_mentions_org` (a schema-batch index) rebuilt over the
PARTIAL organization_mentions, and `strip_organization_indexes` must clear that table before
Phase-1 (index-free bulk load) so it can't be deferred. 6b816bd's first-minute diagnosis
mitigates this for free: kill on the FIRST bad checkpoint line → small partial → single-digit
minute boot. The real cheap-kill fix (boot skips rebuild-managed index creation while
`rebuild_in_progress` is set) is issue-64-adjacent, moderate, deferred past this run.

### ⚠ CRITICAL COUPLING (surfaced 2026-07-29) — build it as ONE atomic change

`plan_is_complete` (canonical.rs:1518) is the resume-from-plan salvage invariant: a
complete `plan_notice` ⇒ mentions complete ⇒ safe to resume Phase-2. For this issue,
"a complete plan ⇒ blobs exist" must ALSO hold, so `plan_is_complete` must additionally
require `COUNT(plan_state) == COUNT(plan_notice)`. That change is COUPLED to the Phase-1
write: it CANNOT land before the write is live, or a rebuild resumes with a complete
plan but NO blobs and the pre-pass (reading plan_state) folds nothing. So plan_state
schema + Phase-1 write + `plan_is_complete` + pre-pass swap must ship as ONE change and
deploy together — no partial stage. Deploying a `plan_is_complete`-only or plan_state-only
increment would break resume for the ACTIVE recovery salvage.

⇒ **Build AFTER the recovery lands**, when the salvage/resume path is no longer live, in a
focused uninterrupted session with the full byte-identity gate. Plan + approach are banked.

---

Severity: MEDIUM (turns every future full rebuild's Phase-2 from ~days into
~minutes; removes a whole 254GB sequential re-read from the fast-Phase-2 pipeline)
Relates to: Phase-2 blaze effort (bucketed range-fold, Task-1/Task-2), 59, 60, 62
Design: `.scratch/tender-db/design/phase2-blob-storage.md` (converged design +
"Follow-up" + "Read-path guardrail" sections)

## Context

The Phase-2 "blazingly fast" effort ships a **bucketed sequential-fold**: a
post-grouping pre-pass materialises each notice's fully-resolved fold-input
(`NoticeState`) as a serialized blob, routes blobs into fold-ordered range
buckets, and Phase-2 reads each bucket sequentially. For the CURRENT salvage the
pre-pass reads the existing parsed layer sequentially (Phase-1 already complete on
disk), which is a full ~254GB sequential re-read.

This issue captures the optimization that makes that re-read **free on the next
from-scratch rebuild**: write the blob during Phase-1's existing sequential pass.
Deferred out of the salvage cutover on purpose (Phase-1 is already done there);
adopt on the next `rebuild:true`.

## The optimization

`build_plan` (Phase-1, project.rs ~445-489) already streams the entire parsed
layer sequentially via `parsed_chunk` AND resolves each chunk's org mentions in id
order — then discards the heavy `Parsed` after computing the light `Ident`. Instead,
in the same chunk loop, also materialise the fold-input blob:

1. **Schema:** a separate `plan_state(notice_id INTEGER PRIMARY KEY, blob BLOB)`
   **plain ROWID** table.
   - Separate table, NOT a column on `plan_notice`: `build_plan_groups` does a
     full-table `UPDATE plan_notice SET group_key=…`, which would rewrite every
     blob (~25-50GB) for nothing.
   - Plain rowid, NOT WITHOUT ROWID: a table leaf's inline-blob threshold is
     ~4061B vs ~1002B for a WITHOUT-ROWID index leaf, so the common 1-3KB blobs
     stay inline (fewer overflow hops); rowid = insertion order gives
     clustering-by-notice-id for free (turso-internals, btree.rs:9027-9036).
2. **Write point:** in the chunk loop, after `resolve_mentions` (which returns org
   ids aligned index-for-index to the input `&[Mention]`, each carrying
   notice_id+section_id — canonical.rs:1665), zip the ids back to build each
   notice's `HashMap<section_id, org_id>`, run `NoticeState::read` +
   `bind_organizations(by_section)`, postcard-serialize, and append the
   `plan_state` row **in the SAME transaction as `insert_plan`**. Then
   `plan_is_complete`'s COUNT check (canonical.rs:1279) atomically covers blobs
   too — the resume-salvage invariant ("a complete plan implies its blobs exist")
   holds for free.
3. **Cost:** the 254GB read already happens in Phase-1; added work is CPU
   (`read`+`bind`+postcard on the `Parsed` already in RAM, Phase-1 is
   I/O/resolve-bound so ~free) + a sequential ~25-50GB blob append. Saves a whole
   254GB sweep vs a separate materialisation pass.
4. **Downstream unchanged:** the post-grouping range-bucket routing pass reads
   `plan_state` (~25-50GB) sequentially instead of the 254GB parsed layer; Phase-2
   folds per bucket.

## Prerequisites

- serde derives on the store fold types (`Fact`, `LotState`, `Round`, and the
  results structs) + make `NoticeState` (ingest, project.rs) serializable.
- Add `postcard` to the ingest crate (compact varint codec; smaller than bincode).

## Companion next-rebuild levers (turso-perf)

Fold in while touching the rebuild write path — each independently shrinks
rebuild wall-time and composes with the blob write:

- **64K `page_size`** on the rebuild DB: fewer/larger IO ops for the big
  sequential blob writes + reads; fewer overflow pages per large blob (raises the
  per-page inline capacity). Set at create time (companion to the plan_state
  layout). Validate against the VPS page-cache budget.
- **Prepared statements** for the per-row `plan_state`/`plan_notice` inserts:
  `conn.execute` re-parses SQL each call (memory: ~10M point writes ≈ 18min of
  pure re-parse at 12.4M). A prepared insert removes that.
- **Read-path guardrail:** any sequential materialisation (this piggyback OR the
  salvage pre-pass) MUST read via `parsed_chunk`'s [lo,hi]-per-table bursts, never
  a per-notice or lockstep-10-cursor read — turso has no readahead; the ~490MB/s
  comes from the kernel's per-fd readahead, which a naive 10-cursor-on-one-fd merge
  thrashes down to tens of MB/s. `parsed_chunk` already does the safe chunked-merge
  (10k-notice per-table runs; every parsed table PK-clustered by notice_id). The
  pre-pass's side-reads (group_key from plan_notice, mentions from
  organization_mentions) must ALSO use `WHERE notice_id BETWEEN lo AND hi`, not
  `IN(chunk_ids)`, to stay on the burst discipline (proj-fix). Window W is a
  RAM-vs-run-length knob, not read-throughput (size off cache_size). Optional
  upside: N independent read-only `Database` opens = N fds = N kernel readahead
  windows (turso-internals; off happy-path, A/B it, read-only/no-writer only).

## Validation (byte-identical gate)

Moving `NoticeState::read`+`bind` into Phase-1 must be output-identical:
- `BTreeSet<Fact>` round-trips through postcard order-preservingly (fold relies on
  `Fact::key()` order).
- Org ids identical (Phase-1 already resolves mentions in id order today).
- Gate: `project_equivalence` + `project_resume` green + the fold-source-invariance
  test (sequential/blob fold vs parsed fold → all canonical tables equal
  ORDER BY pk on a fresh scratch DB).

## 2026-09-03 — measured on the 304 campaign's fold (job 612): the pre-pass is 8% of a full fold

| phase of 612 (14.33M notices → 7.92M tenders, 20,960 s) | wall | share |
| --- | --- | --- |
| plan build (phase 1) | 8,580 s (2 h 23 min) | 41% |
| grouping | 237 s | 1% |
| pre-pass (31 stripes, the 254 GB re-read this issue targets) | **1,614 s (27 min)** | **8%** |
| apply (bucketed fold) | 12,352 s (3 h 26 min) | 59% |

This issue moves the pre-pass's read into phase 1. With issue 94's stripes the
pre-pass is 27 minutes of a 5.8-hour fold, and the blob write would make phase 1
— already the second-largest phase — heavier. The half-to-full day plus the
byte-identity risk does not buy a meaningful fraction of the wall any more.
**Parked by measurement**; the wall is the apply (issue 67, whose own gate this
run now meets) and the plan build (192). Revisit only if a pre-pass shape returns
to the 08-01 numbers (402 min, pre-94).
