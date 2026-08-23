# 94 — the bucketed pre-pass shards by id WIDTH, runs ~1× parallel, and reports nothing

Status: RESOLVED — all three defects fixed in `f4ce4e1` and running live; recorded 2026-08-23 by
the board-vs-code audit (the file had never been updated, same as 93/64)
Kind: performance (parallelism) + operability
Blocked by: —
Relates to: 66 (the sharded pre-pass this is about), 62 (the bucketed fold), 90 (the same blindness one
level up), 91, 93

## Observed

During the eForms-DE 1.x re-fold's Phase-2 pre-pass, ~80 minutes in, per-thread `/proc/<pid>/task/*/io`:

```
tid 3133829  job-exec  read=72.3GB  utime=31.7min  state=D   +16.2 MB/s   <- the ONLY sweeping worker
tid 3111033  job-exec  read=59.7GB  utime=20.0min  state=S   futex_do_wait <- plan build, parked at the join
everything else                     <20 MB
```

`k = 3` (three `shard{0,1,2}_bucket*.bin` sets exist), but two of the three workers had already exited.
`std::thread::scope` threads leave `/proc/.../task` when their closure returns, so their absence is
completion, not death.

The whole-process throughput curve that triggered the escalation — 51 → 40 → 14 → 7 MB/s — was
**workers finishing**, not a deepening stall. The survivor held a steady 15–16 MB/s in state D
throughout, with the box at 75% idle / 17.5% iowait.

## Defect 1 — equal-width id stripes are unbalanced to the point of serialising

`write_buckets_sharded` splits `(0, max_id]` into `k` **equal-width id ranges**:

```rust
let max_id = db.max_parsed_notice_id().await?;
let width = ((max_id + k as i64 - 1) / k as i64).max(1);
```

That assumes notices are uniformly dense across id space. They are not. `max_parsed_notice_id` sits far
above the dense region (issue 85 cites notice id 26,195,620 against 14.2M parsed notices), and bulk
reclaims append late, so the top stripes are largely empty id space. They complete almost immediately
having swept nothing, and **stripe 0 carries essentially the whole corpus single-threaded.**

So issue 66's parallel pre-pass delivers ~**1×**, not `cores − 1`×. It also means the 7h13m pre-pass
measured on the 2026-07-31 rebuild — the number used as the budget for this re-fold — was very likely
also effectively single-threaded, i.e. a realistic floor rather than an optimistic one.

**Fix:** stripe by equal *parsed-notice count*, not id width. The boundaries are one indexed query over
`notices(id)` (`parse_state='parsed'`, ordered, sampled at `n/k` offsets), computed once. Byte-identity
is unaffected — routing is by `group_key`, so which stripe produced a notice never changes which bucket
it lands in (the existing `sharded_prepass_matches_the_serial_prepass` test already pins this).

## Defect 2 — `worker_count` is sized for a CPU-bound sweep; this one is latency-bound

`worker_count` returns `cores − 1` (3 on the prod box). But the survivor sits in state **D** with the
CPU 75% idle: the sweep is bound by I/O *latency*, one outstanding read at a time per worker, not by
cores. Queue depth is what buys throughput here.

**Fix:** for the pre-pass, size `k` well above core count (device queue depth, not CPU parallelism) —
still clamped by the fd budget, which already accounts for `n_buckets × workers` open files. Worth
measuring 3 vs 8 vs 16 on prod-shaped data before picking a number.

## Defect 3 — the pre-pass reports NOTHING

`write_shard` prints no heartbeat. A multi-hour sweep emits zero output between
`[project] incremental phase 2: Buckets …` and the first fold heartbeat hours later. That is exactly
the blindness issue 90 removed one level up, and it is why an hour of diagnosis went into
reconstructing worker positions from `/proc` counters.

Worse, the obvious external proxy is **misleading**: the bucket files are wrapped in `BufWriter` and
flushed only at the end of the sweep, so on-disk bucket bytes stay near zero regardless of progress. A
reader who assumes bucket size tracks progress will conclude the sweep is dead when it is healthy.
(This nearly happened here.)

**Fix:** per-chunk heartbeat from each worker — shard index, notices swept, current `after_id`, rows
spilled — throttled like `PLAN_HEARTBEAT`. Cheap, and it makes the stripe imbalance above
self-evident the first time it runs.

## Not defects (measured, so they are not re-investigated)

- The sweep is **bounded and terminating**: `after_id` advances monotonically to each chunk's last id
  and every satellite read is a ranged scan over that chunk's window, so each notice is visited exactly
  once. Total work is fixed at the parsed layer's size; only the rate varies.
- The read amplification (sweeps ~14.2M notices to fold 473K) is deliberate — it is what buys the
  sequential read that issue 62 exists for. Scoping the sweep to the plan's ids is the obvious ~30×
  idea, but it is ParsedFold-shaped (a scattered read of a scoped id set), and ParsedFold's 5h07m
  CPU-bound zero-I/O stretch is **still unexplained** (issue 91). Do not take that path until it is.
  Page-level amplification also blunts it: at ~1-in-30 density a scoped read touches nearly a page per
  notice per table, so the real win is well under 30×.

## RESOLVED (recorded 2026-08-23, owner) — all three defects fixed in `f4ce4e1`, verified live tonight

- **Defect 1 (stripe imbalance):** stripes now hold equally many PARSED notices
  (`parsed_id_stripes`), not equal id widths — and the sweep is additionally bounded to the
  PLAN's id range, so a scoped re-fold skips the empty id space entirely
  (`write_buckets_sharded`, project.rs).
- **Defect 2 (worker sizing):** `PREPASS_MIN_WORKERS = 8` floors the count independent of cores
  (the sweep is latency-bound, queue depth is the lever), `TENDER_PREPASS_SHARDS` is the ops
  valve, and `PREPASS_CHUNK_BUDGET / k` keeps peak RAM flat as k rises.
- **Defect 3 (blindness):** per-shard stderr heartbeats plus an aggregate swept counter polled
  into the job's Progress record every 2 s (issue 65's surface), with a closing tick after the
  join.

Live witness, tonight's campaign fold #326 (2026-08-22 20:51 UTC): `phase 2 pre-pass: 31
shard(s) over notice ids (0, 28811137] (100% of the id space)` followed by heartbeats every ~10 s
sweeping ~25K notices/s aggregate — the exact observability and parallelism this issue asked for.

Still open elsewhere: the scoped-read variant this issue's fix list gated on issue 95's
ParsedFold unknown stays parked with 95.
