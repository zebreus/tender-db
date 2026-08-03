# 66 — parallelize the write_buckets pre-pass (the Phase-2 bottleneck)

Status: MERGED into issue 94 (2026-08-02) — 94 is the same pre-pass parallelization, observed live
during the issue-85 re-fold with the concrete defect (equal-width id stripes → ~1× not 3×). proj-fix
is implementing stripe-by-parsed-notice-count + higher worker_count under 94's Group-1 branch; do NOT
build this separately. This design note stands as the "why parallelize" background; 94 carries the fix.
Kind: performance
Blocked by: —
Relates to: 62 (bucketed fold), 63 (Phase-1 plan_state piggyback — the deeper,
composable fix), 64/task #3 (prepared-stmt writes, fold side)
Design owner: blob-schema

## Observation (2026-07-25 full rebuild)

With the bucketed fold, the dominant cost is the `write_buckets` PRE-PASS: read the
whole parsed layer (~200GB) once and, per notice, decode 10 tables → `NoticeState`
→ `bind_organizations` → postcard-encode → append to a bucket. **CPU-BOUND on the
per-notice decode/fold/encode, disk ~idle** (turso-bench: single-thread scan is
CPU-bound at ~159 MB/s raw, but the pre-pass does far more per notice so its
EFFECTIVE read rate is only ~16 MB/s ≈ ~980 notices/s single-thread → ~3.5h for
12.39M). The fold pass and index builds are smaller. So a rebuild's wall-clock now
lives in the pre-pass.

## Why it parallelizes cleanly (the core insight)

`write_buckets` routes each notice to `bucket[partition_point(boundaries,
group_key)]` — a bucket is a fold-order **group_key RANGE**, and routing is by
**group_key (content)**, NOT by notice_id. So a group's notices ALL route to the
same logical bucket regardless of where in the id space they sit. That means the
READ can be sharded by notice_id with zero effect on which bucket a notice lands
in. And the pre-pass is **pure-read against the DB** (it writes to bucket FILES,
not the DB) with **no shared mutable state** and **no cross-notice ordering
dependency** — crucially, it reads ALREADY-RESOLVED `organization_mentions`
(Phase-1 fixed the org ids), so unlike Phase-1's streaming org resolution it has no
sequential dependency. Embarrassingly parallel.

## Design — sharded pre-pass

### 1. Sharding scheme
- Compute `boundaries = bucket_boundaries(db, notice_batch)` **once, unchanged**
  (the fold-order group_key cut points from `next_plan_batch`). Shared read-only
  across all workers. Sharding does NOT touch the boundaries — only who writes.
- Partition the notice-id range `[1, MAX(notice_id)]` into **K contiguous stripes**
  `[lo_s, hi_s)`. (By id range; gaps are harmless — a worker just reads fewer
  notices. Optionally balance by plan_notice count per stripe, but equal id-width
  is fine and simplest.)
- Spawn **K workers**, each with its **own turso read connection** (from
  `db.readers(K)` — CONTEXT.md's N-parallel-readers model; no writer conn needed,
  so no writer lock the whole pre-pass). Each worker runs the EXISTING per-chunk
  body over its stripe:
  `parsed_chunk(after_id, 10k)` (bounded to `hi_s`) → `plan_group_keys(lo,hi)` →
  `mentions_by_ids(ids)` → per notice `NoticeState::read` + `bind_organizations` +
  `BucketRow::snapshot` + postcard → route via `partition_point(boundaries, gk)`.
- Each worker writes to its **own shard** of bucket files:
  `dir/shard{s}_bucket{b}.bin` (append-only, plain framed `[u32 len][bytes]`, same
  format as today). No two workers ever write the same file → **zero write
  contention**, no locks.

### 2. Fold-pass consumption
`bucketed_fold` iterates buckets `b = 0..N` in fold order (unchanged — this is what
keeps surrogate ids in global fold order). Only the per-bucket READ changes: instead
of `read_bucket(dir/bucket_b.bin)`, do
`read_bucket_shards(dir, b, K)` = concat `read_bucket(dir/shard{s}_bucket{b}.bin)`
for `s in 0..K`, then the EXISTING `rows.sort_by(sort_key)` and `fold_bucket`.
Everything downstream (`fold_bucket`, `apply_tenders`, `mark_projected`) is
UNCHANGED. Per-bucket fold RAM is unchanged: logical bucket `b` still holds ~50k
notices total (the same group_key range), now spread across ≤K shard-files that
concatenate back to the same ~50k rows (~100MB) before the sort.

**No pipeline overlap:** a group's notices span stripes, so bucket `b` is only
complete once ALL workers finish (any stripe may still emit into `b`). So the fold
pass starts after a barrier (join all workers). Total = parallel-pre-pass +
serial-fold. (The fold stays serial: `apply_tenders` assigns surrogate ids in
order and writes via the single writer — parallelizing it would break byte-identity.)

## 3. Byte-identity argument (rigorous)

The bar (ADR-0001): the canonical layer must be byte-identical to the single-writer
path, surrogate ids included. It holds under sharding because **arrival order into a
bucket is irrelevant** — `fold_bucket` re-establishes the total order by sorting
each (merged) bucket by the fold key `(group_key, published_at, source_rank,
publication_id, notice_id)` before folding, and that key is a TOTAL order (notice_id
is unique and terminal → no ties). Concretely:

- **A group's notices land in exactly one logical bucket.** Every worker routes with
  the SAME `boundaries` array via `partition_point(boundaries, group_key)`, a pure
  function of group_key. So all notices of group `g` → the same bucket index `b`,
  regardless of which stripe/worker produced them. (This is the invariant that must
  hold; it does, because routing is content-keyed, not shard-keyed.)
- **Edge case — a group split across shards.** Groups DO split across stripes (a
  legacy OJS chain spans the whole id range; even a keyed CN and its award can fall
  in different id stripes). Harmless: the split notices go to different *physical*
  files (`shard0_bucket_b` vs `shard3_bucket_b`) but the SAME *logical* bucket `b`.
  The fold pass reads ALL K shard-files for `b`, concatenates, and sorts — so the
  per-group run in `fold_bucket` (which walks adjacent equal-group_key rows after
  the sort) sees the COMPLETE chain in exact fold order. A group is never split
  across two *logical* buckets (that would require its group_key straddling a
  boundary, impossible — a group has ONE group_key).
- **Cross-bucket order** is preserved: buckets are folded `b = 0..N` ascending, and
  boundaries are ascending contiguous group_key ranges (last = global max), so the
  concatenation of per-bucket fold orders = the global fold order = identical
  surrogate id assignment.
- **Org binding is deterministic under parallelism:** `bind_organizations` reads the
  already-recorded `organization_mentions.organization_id` (fixed by Phase-1), a
  pure read — same input for every worker, no ordering effect. (Contrast Phase-1,
  which MUST stay sequential; the pre-pass is the safe place to parallelize.)

Validation: `project_fold_source` (ParsedFold vs Buckets, all-tables byte-equal) +
`project_equivalence` stay the gate. Add a K>1 variant of the bucketed run to the
fold-source test so parallel production is exercised against the serial baseline.

## 4. Memory bounds

- Per worker: one `parsed_chunk` (10k notices' `Parsed` in RAM, ~tens of MB) + its
  N `BufWriter`s (one per bucket) + the per-chunk `group_keys`/`mentions` maps
  (~10k entries). Peak ≈ **K × (chunk + N buffers)** ≈ 4 × (~50MB + N×8KB). With
  N≈248 buckets: ~4 × (~50MB + ~2MB) ≈ **~210MB**. Well under the box.
- **File-descriptor ceiling:** N×K open bucket files (248×4 ≈ ~1000) approaches the
  common `ulimit -n` 1024. Mitigations: raise the soft limit for the process, OR
  cap N (fewer/larger buckets — bucket fold-RAM is ~50k notices × ~2KB ≈ 100MB, so
  N could halve safely), OR have each worker flush+close buckets it's finished with.
  **Note this explicitly in the impl.**
- **Page-cache interaction — does NOT worsen.** K workers each sweep a DIFFERENT
  id-stripe, so the TOTAL bytes read = the same ~200GB, each page still read exactly
  once (a forward sweep per stripe; no re-read). Cache pressure is dominated by the
  same total bytes, not multiplied. The disk has ample headroom: the pre-pass reads
  at only ~16 MB/s effective (CPU-bound), so K=4 → ~64 MB/s aggregate ≪ the
  ~315 MB/s disk wall (turso-bench). Parallelism is CPU-bound and the disk never
  becomes the bottleneck until K ≈ 16+ workers — far beyond the core count.

## 5. Concurrency primitive + minimal restructuring

- **Primitive:** turso multi-connection reads (`db.readers(K)`), which turso-bench
  measured scaling near-linearly on the box's cores with the disk idle. K tokio
  tasks (or threads), each owning a stripe + its own reader + its own `Vec<BufWriter>`;
  `join!` all, then run the fold pass.
- **Minimal restructuring** (the whole point — sharding touches TWO seams):
  - `write_buckets(db, boundaries, dir)` → `write_buckets_sharded(db, boundaries,
    dir, K)`: computes K stripe ranges and spawns K copies of the EXISTING per-chunk
    body, parameterized by `(stripe_lo, stripe_hi, shard_index)`, writing to
    `shard{s}_bucket{b}.bin`. The per-chunk body is lifted verbatim.
  - `bucketed_fold`'s per-bucket read: `read_bucket(path)` →
    `read_bucket_shards(dir, b, K)` (concat then the existing sort). ~5 lines.
  - `bucket_boundaries`, `fold_bucket`, `BucketRow`/`sort_key`/`snapshot`,
    `apply_tenders`, `mark_projected`, the codec — ALL UNCHANGED.
  - One small helper: bound `parsed_chunk` to a stripe's `hi` (either a
    `parsed_chunk_range(after, hi, limit)` variant, or the worker breaks when
    `last.id > hi_s`).

## 6. Expected speedup + choosing K

Pre-pass is CPU-bound, so speedup ≈ K until CPU saturates (K = cores) — the disk
wall (K ≈ 16) is never reached. **K = cores − 1** (leave one for OS/IO):
- 4-core box: K=3 → ~3× → **~70 min** (from ~3.5h).
- 8-core box: K=7 → ~7× → **~30 min**.
Report the box's real core count at implementation time and set K accordingly. Beyond
cores there's no gain (CPU-bound); below the disk wall so no I/O ceiling. If ~70min
on a 4-core box isn't "tens of minutes" enough, the deeper lever is issue 63 (§7),
which removes the decode entirely.

## 7. Relationship to issue 63 (the deeper, composable fix)

66 parallelizes the decode-heavy pre-pass. **63 (Phase-1 `plan_state` piggyback)
ELIMINATES the decode** — Phase-1 already reads the 254GB parsed layer and resolves
orgs, so it can serialize the `BucketRow` blob then (org-resolution-bound, inherently
sequential — NOT parallelizable). Then the "pre-pass" becomes a light **routing pass**
that reads the already-serialized `plan_state` blobs (~25-50GB, no re-decode) and
shards them into buckets — I/O-light AND parallelizable by exactly this §1 scheme.
So **63 + 66 compose**: 63 removes the 254GB re-read + the per-notice decode, 66
parallelizes the residual routing. Best-case future rebuild = Phase-1 (unchanged
cost) + parallel light routing + serial fold. For THIS salvage (63 not yet in), 66
parallelizes the current decode-heavy pre-pass directly.

## 8. Non-goals / scope

- Not needed for correctness, nor for the incremental daily path (tiny — single
  chunk). Purely to make a FULL rebuild fast.
- The fold pass stays serial (byte-identical surrogate ids). Its speed is task #3
  (prepared-stmt writes) + the index-free apply, not this issue.
- Implement AFTER this build lands + the write-side speedups (task #3 / issue 64),
  so the parallel pre-pass is measured against a known-good serial baseline via
  project_fold_source.
