# 63 — Adopt `plan_state` blob@Phase-1 for fast full rebuilds

Status: scoped (proj-fix, 2026-07-29) — smaller than the design's ~1-2 day estimate; the
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
