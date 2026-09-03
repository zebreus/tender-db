# 339 — a bucketed fold shows the last pre-pass count until its first whole bucket lands

Status: DEPLOYED 2026-09-03 (`9f0bcea`), verification pending the next bucketed fold — was: DIAGNOSED 2026-09-02 (corrected the same day — the first filing blamed the
plan build; the journal's stage timings named the real stage). Fix in progress.
Kind: operability / instrument honesty
Relates to: 65 (Progress → phase record), 90 (the fold heartbeat), 262 (the same
gap on the plan build, fixed), 304 (the campaign whose fold surfaced it), 42/53
(why a growing WAL during a silent stage must be tellable from a runaway)

## Observed (job 608, 2026-09-02, CEST)

| time | journal | `/admin/jobs` phase |
| --- | --- | --- |
| 15:52:09 | `pre-pass shard 30 DONE` — the last shard | `pre-pass 8520548 / None` |
| 15:52 → 16:09 | nothing from `[project]` | still `pre-pass 8520548` (checked 16:04) |
| 16:09:24 | `incremental fold: 2779/43538 Tenders folded, 50001 versions written` | folding |
| 16:14:29 | fold done; `phase 2 fold + apply: 2213.8s` | |

Meanwhile the WAL went 66 KB → 26 GB and free disk 702 → 683 GB during the
silent 17 minutes. That was the fold writing its FIRST BUCKET — 2,779 tenders
whose 50,001 versions now carry up to 24 languages of text each — and the phase
record said "pre-pass" the whole time.

## Why

Two things compound:

1. `bucketed_fold` emits `Progress::Applying` **once per bucket**, after the
   bucket is applied. The first bucket is the biggest chains, so its first tick
   is the slowest to arrive — 17 minutes here; on the campaign's corpus fold
   (612: 4.4M re-parsed notices × 24 languages) it could be hours.
2. The grouping step's `Progress::Grouped` had set the phase to `folding 0/N`,
   but the pre-pass's `PrePass` ticks then overwrote it with `pre-pass <count>`,
   and nothing resets it when the pre-pass barrier is passed. So the record
   does not merely lag — it names the wrong stage.

The first filing of this issue blamed the plan build. It was wrong: 608's plan
build ran at the START (`planning 50000/160334`, visible), and `build_plan`
already ticks per chunk and reads the stop flag. The silent stage was the fold.

## Why it matters

An operator reading the job during that window sees a stalled pre-pass count and
a WAL growing by gigabytes — exactly the shape of the issue-42/53 reader-pinned
runaway. Telling the two apart took the journal and prior knowledge. And the
campaign's fold 612 will spend far longer in exactly this state.

## Fix

At the pre-pass barrier (after the shard join, before the first bucket),
`bucketed_fold` emits `Applying { tenders: 0, total, versions }` — the phase flips
to `folding 0/N` the moment the fold starts, and the record names the stage.
Nothing about what is folded changes.

Considered and rejected: ticking per apply batch INSIDE a bucket. A bucket is
one `apply_tenders` transaction by design (committed whole, then its notices
marked projected), so intra-bucket ticks would mean splitting that transaction —
a fold-shape change for an operability gain. The count still moves per bucket;
what the barrier tick fixes is the stage being NAMED wrongly for the whole first
bucket, which was the dangerous part.

Pinned in `the_prepass_reports_its_sweep_as_progress`: the first `Applying` tick
carries `tenders: 0`, and the pre-pass barrier guarantee (no `PrePass` tick after
the first `Applying`) still holds with the new tick placed after the join.

## 2026-09-03 — deployed (`9f0bcea`, 09:03 UTC); the control case read as predicted; close on the next bucketed fold

Job 612 (the 304 campaign's fold, pre-fix binary) is the CONTROL: its job row
read `pre-pass 6887156` with no total through the pre-pass and only switched to
`folding N / 7922692` once the first bucket's apply came round — the gap this
issue describes, seen live. The fix is on prod now but the daily `project`
takes the incremental `ParsedFold` path (4,391 changed notices in 163 s today),
which never enters `bucketed_fold`; the barrier tick shows only on the next
big fold (an epoch refold or a campaign's paired fold). Stays DEPLOYED-UNVERIFIED
until then; nothing to do in the meantime.
