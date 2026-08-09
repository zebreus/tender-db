# The projection's serial stages idle a big box (phase-1 plan build, fold+apply)

Status: open — filed by orchestrator, 2026-08-09, from live measurement on the new 32-core prod box.
Blocked by: nothing. Grabbable after the issue-174 refold verification closes.

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

## Explicitly NOT in scope

- Parallelizing the WRITER — one writer is a Turso/store invariant.
- `TENDER_PREPASS_SHARDS` tuning — it's an env valve already; measure per
  device when a sweep is on the critical path (the 8-floor comment says why).
- The API `READERS: usize = 8` const (app/main.rs) — separate, minor: worth an
  env override on a 32-core box, but it does not touch projection throughput.

## Why it matters

The plan build runs at FULL corpus scale on every projection, incremental or
not — it is a fixed ~1¾ h serial tax per fold on hardware that could plausibly
do it in ~20–30 min. Era refolds (174's follow-ups: r207, r2.0.8 …) each pay it
twice over the queue+fold cycle.
