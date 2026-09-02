# 337 — turso leaks a temp-database directory under `TMPDIR`, ~5/day, unbounded

Status: DIAGNOSED 2026-09-02 — **trigger found: the daily tick, one directory per
tick.** Accumulated 143 swept safely; the leak itself is NOT fixed (it is in the
turso library) but it is now characterised, timestamped, and much smaller than the
first estimate. See "The trigger, caught on the first clean tick".
Kind: resource leak (third-party) / operational hygiene
Relates to: 169 (where it surfaced, and where it was correctly ruled OUT as the
storage cause), 166 (the turso 0.7.2 bump)
Blocked by: nothing

## What leaks

`TMPDIR=/data/tmp` is set in the unit. Under it, directories named `.tmpXXXXXX`
accumulate, each containing exactly:

```
tursodb-temp.db       4096 bytes
tursodb-temp.db-wal     32 bytes
```

**These are turso's, not ours.** There is no `tempfile`/`TempDir` use anywhere in
`crates/*/src`; the name `tursodb-temp.db` is the library's own.

## Measured 2026-09-02

| | |
| --- | --- |
| directories accumulated | **143** (oldest 2026-08-09) |
| total size | 128 MB |
| creation rate | ~4–9 per day, every day, through 2026-09-01 |
| created since the last service restart (~1h50m) | **0** |
| directories predating the current process start | **143 — all of them** |

Two readings follow from the last two rows, and the second is the useful one:

1. **It is not per-connection or per-hour.** The process opens ~23 connections and
   had been up nearly two hours with none created.
2. **Every accumulated directory is provably orphaned.** Nothing the running
   process holds is older than the process itself, so anything older than
   `ActiveEnterTimestamp` is garbage by construction. That is a precise, safe
   deletion rule and it needs no heuristic about age.

The rate does *not* match restarts either — 2026-09-01 saw roughly twenty deploys
and produced four directories — so what actually triggers it is **not yet known**.

## Done: a one-off sweep

143 directories older than the service's `ActiveEnterTimestamp` were removed;
`/data/tmp` went 128 MB → 0, and `/health` stayed green through it. Safe by the
rule above rather than by judgement: none could have been held by the live
process.

**This is not a fix.** It will accumulate again at ~5/day.

## Why it is not urgent, stated so it is not over-prioritised

128 MB over three weeks against a 1.7T volume is nothing, and issue 169 explicitly
measured it and ruled it out as the storage-growth cause. The real concern is that
it is **unbounded**: inodes and directory entries accumulate with no ceiling, on a
box whose database is already 490 GiB.

## Options, in increasing order of effort

1. **A recurring sweep with the rule above** — delete `.tmp*` directories older
   than the service's start time. Correct by construction, no age heuristic, and
   it can ride the existing hourly `tender-db-diskwatch.timer`. The one caution:
   that script is documented "detection only, journal-visible", so adding
   deletion changes its character — better as its own tiny unit than smuggled in.
2. **`systemd-tmpfiles`** with an age rule. Simpler, but an age heuristic can in
   principle delete a directory a very long-lived connection still holds, which
   the start-time rule cannot.
3. **Find and fix the trigger, or report upstream.** Needs knowing what actually
   creates them — the missing fact above. Worth doing before assuming a sweep is
   the permanent answer.

## Suggested first step

Watch for the next few to appear and correlate with what the box was doing. Now
that the directory is empty, the *next* `.tmp*` to show up is a clean signal with
a timestamp, which the previous 143 were not.

## The trigger, caught on the first clean tick

The previous section predicted that with `/data/tmp` emptied, "the next `.tmp*` to
appear is a clean timestamped signal the previous 143 were not". It was.

The daily tick fires 09:35 Berlin / 07:35 UTC. After it ran:

```
2026-09-02 09:35:03  /data/tmp/.tmp7EzLBf      (tursodb-temp.db + -wal, 4 KB)
count: 1
```

**One directory, stamped at the tick minute itself** — 09:35:03, before any of the
jobs it enqueued had finished (probe, process, fetch-rates, project and
reveal-recheck all completed 09:36–09:39, all `ok`).

### What that changes

* **The rate is ~1/day, not ~5/day.** The 4–9/day measured over 2026-08-09→09-01
  was inflated by campaign activity — that window held the text-era work, the
  325/326/328 repairs and dozens of manual census runs. In steady state this is
  one directory a day: ~365/year, still unbounded, but an order of magnitude
  slower than the first reading suggested.
* **It is not per-job and not per-restart.** Yesterday saw roughly twenty deploys
  and thirty manual jobs and produced four directories; today's tick produced
  exactly one. Whatever creates it happens once per tick.
* **The sweep design is unchanged and still correct** — anything older than the
  service's `ActiveEnterTimestamp` is provably orphaned.

### Still not known

*Which* part of the tick creates it. The stamp is the tick minute rather than any
job's completion, so the candidates are the tick's own bookkeeping or the first
job's opening work. One more tick with per-job timing would separate them, and
that costs nothing but a day's patience.

### A measurement error worth recording

The first pass at this reported a directory in one line and `count: 0` in the
next — a flat contradiction, caused by my own broken shell quoting inside a
command substitution (the pattern became a literal `\".tmp*\"` and matched
nothing). Had I read only the count, this issue would now say "nothing appeared at
the tick" and the trigger would still be unknown. **Two numbers that cannot both
be true are a signal to re-measure, not to pick one** — the same lesson issue 332
recorded when a ratio of 100% sat beside thousands of one-over-many keys.
