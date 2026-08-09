# The projection's serial stages idle a big box (phase-1 plan build, fold+apply)

Status: DEPLOYED 2026-08-09 ~23:00 Berlin — Lennart gave explicit deploy permission (no git
push; rev traveled as a git bundle over ssh), nix build + atomic swap + restart executed,
post-checks green (MainPID 29599->122983, NRestarts 0, /health 200, /health/deep green at
rev b56ce13, TENDER_CACHE_KIB=524288 live in the service environment, SSE snapshot 200 with
first byte in 181ms). Remaining: measure the next era refold against the numbers below —
that closes the issue.
Blocked by: nothing; waiting on the next refold occasion for the measurement.

## Implemented (evening of 2026-08-09)

Both stages use the same shape: a prepare thread does everything that never
touches the writer, a rendezvous channel (`sync_channel(0)`) hands work to the
writer side in order, RAM stays bounded at two units in flight, producer
panics resume_unwind on the consumer so they can't read as a short run.

- **Stage A (`build_plan`)**: the parsed-layer sweep + JSON decode +
  `Ident::read`/mention extraction moved to the prepare thread (own
  current-thread runtime + own reader connection, the pre-pass workers'
  pattern). The id-order invariant stays on the writer side: one producer, an
  order-preserving channel, `resolve_mentions` + `insert_plan` sequential as
  before. Reading a chunk ahead of the writes is safe — sweep reads
  notices/parsed, writer writes organizations/mentions/plan (disjoint tables).
- **Stage B (`bucketed_fold`)**: shard-file read + sort + the pure-CPU fold
  (`fold_rows`, extracted from the old `fold_bucket`) runs one bucket ahead of
  `apply_tenders`/`mark_projected`. Apply order is untouched, so surrogate-id
  byte-identity holds structurally — and every existing gate now runs THROUGH
  the pipeline (project_equivalence, fold_source incl. 1-vs-3-vs-8-shard
  identity, golden, incremental, resume: 46 tests green).

Expected effect: phase 1 ≈ max(decode CPU, writer) instead of their sum;
fold+apply ≈ max(fold CPU, writer I/O) instead of their sum. Measure on the
next refold; if the writer is then the wall on both stages, the remaining
lever is the TENDER_CACHE_KIB bump (already staged) + issue-68's targeted
cache split.

## The measurement (2026-08-09, first fold on the new machine)

Job 2 (`project rebuild=false`, the issue-174 incremental) during phase 1 on the
new box (32 cores, 62 GB RAM, NVMe RAID):

- `server` pinned at ~100% of ONE core (top), 31 cores idle
- md3 at ~15% util — the disks are bored too
- plan rate ~94–143k notices/min → the 14.24M-notice plan build takes ~1¾ h
  wall-clock, all of it serial

Phase 2's PRE-PASS is already parallel (issue 66/94: `cores − 1` workers, fd
self-raise, `TENDER_PREPASS_SHARDS` ops valve) — on this box it gets 31 workers
and is not the problem. The serial stages bracketing it are:

## Stage A: phase-1 plan build (`build_plan`, project.rs)

One thread interleaves, per 10k chunk: read parsed chunk (I/O) → JSON decode of
`Parsed` + `Ident::read` + mention extraction (CPU — the pinned core) →
`resolve_mentions` + `insert_plan` (writer I/O) → checkpoint.

The ORDER INVARIANT that forbids naive sharding: mentions must resolve in
notice-id order so canonical Organization identity matches the whole-RAM
projection (the resume-from-plan salvage also relies on plan completeness
implying mention completeness). But the invariant binds the RESOLVE step, not
the decode. Two safe moves:

1. **Decode pool**: fan the chunk's decode/Ident/mention-extraction across a
   thread pool, deliver results back IN ID ORDER to the single resolve+plan
   writer thread. The order-sensitive tail is cheap relative to the decode.
2. **Read-ahead**: double-buffer — prefetch chunk N+1 on a reader connection
   while N decodes/resolves.

Gate: a serial-vs-pipelined invariance test asserting byte-identical plan rows
AND organizations tables (the same discipline as the pre-pass's 1-vs-3-shard
byte-identity gate).

## Stage B: fold+apply (`bucketed_fold` tail)

After the parallel pre-pass, buckets are processed one at a time:
`read_bucket_shards` (file I/O) → sort → fold (CPU decode+fold) → apply (the
single writer — inherently serial, keep). A two-stage pipeline (fold bucket
N+1 while applying N) overlaps the CPU with the writer without touching the
single-writer invariant or fold order. Same byte-identity gate applies.

## Memory (SHIPPED 2026-08-09): the page-cache valve

Lennart pointed out the new box has 64 GB vs the old 8. The store's
`cache_size = -131072` (128 MiB/connection) was the issue-61 swap-incident
remedy for the 8 GB box, and its own comment (issue-68 caveat) names Phase-1's
~11 indexed range scans per chunk as the pass that measurably benefited from a
bigger cache — exactly the pass measured pinned here. OS page cache does NOT
substitute: the pinned core is partly Turso re-pulling+re-decoding pages
through a 128 MiB window while sweeping a 460 GB file.

Shipped as `TENDER_CACHE_KIB` (positive KiB, clamp [1 MiB, 4 GiB], default
unchanged 128 MiB — the same ops-valve pattern as TENDER_PREPASS_SHARDS):
applied to the writer open and every pooled reader. Prod's unit carries a
drop-in (`tender-db.service.d/cache.conf`) with 524288 (512 MiB): fold-active
set ≈ writer + 31 pre-pass readers ≈ 16 GiB, ceiling ≈ 27 GiB — sized for 62 GB
with OS cache headroom. Effective from the next deploy restart; the currently
running fold still runs the old binary+default. Unit test: the valve parses,
clamps, and degrades to default on garbage. The turso-honors-cache_size proof
already existed (`store/tests/cache_size.rs`, issue 60).

## Explicitly NOT in scope

- Parallelizing the WRITER — one writer is a Turso/store invariant.
- `TENDER_PREPASS_SHARDS` tuning — it's an env valve already; measure per
  device when a sweep is on the critical path (the 8-floor comment says why).
- Raising `TENDER_CACHE_KIB` further (1 GiB+) — measure the 512 MiB fold first;
  the active-set arithmetic above shows where 62 GB starts to bind.
- The API `READERS: usize = 8` const (app/main.rs) — separate, minor: worth an
  env override on a 32-core box, but it does not touch projection throughput.

## Why it matters

The plan build runs at FULL corpus scale on every projection, incremental or
not — it is a fixed ~1¾ h serial tax per fold on hardware that could plausibly
do it in ~20–30 min. Era refolds (174's follow-ups: r207, r2.0.8 …) each pay it
twice over the queue+fold cycle.
