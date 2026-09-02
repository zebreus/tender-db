# 337 — a restart orphans turso's per-connection temp database under `TMPDIR`

Status: **RESOLVED 2026-09-02** — trigger identified in turso's source and
reproduced in a test (`BEGIN IMMEDIATE`, once per connection), the framing corrected
(turso does clean up; our process never unwinds), the cheap-looking fix measured and
rejected, and the sweep shipped as its own daily unit. `/data/tmp` is at 0.
Was: DIAGNOSED 2026-09-02 (trigger correlated to the daily tick); before that, filed
2026-09-02 with the rate over-estimated at ~5/day.
Kind: resource leak (our exit path, not turso's) / operational hygiene
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

## The trigger, read from turso's source and then reproduced

The previous section left one thing open — *which* part of the tick creates the
directory — and proposed waiting a day for another tick with per-job timing. That
was the wrong instrument. The job stamps are second-granularity and the birth
(09:35:03.731) falls exactly on the probe→process boundary, so another tick would
have produced the same ambiguity. Reading turso 0.7.2's own source settled it in
minutes, and a test then confirmed every step:

| where | what it does |
| --- | --- |
| `translate/transaction.rs` | `BEGIN IMMEDIATE`/`EXCLUSIVE` emit an `Insn::Transaction` for `TEMP_DB_ID` — deliberately, "to keep the opcode sequence identical to SQLite". A **deferred** `BEGIN` emits no `Transaction` opcode at all. |
| `vdbe/execute.rs` | `op_transaction` sees `db == TEMP_DB_ID` and calls `Connection::ensure_temp_database()`. |
| `connection.rs` | that lazily calls `create_temp_database()` → `tempfile::tempdir()` under `TMPDIR`, holding `tursodb-temp.db`. Memoised per connection. |
| `connection.rs` | the `TempDir` is owned by the connection's `TempDatabase`, **so dropping the connection removes the directory**. |

Measured by `crates/store/tests/turso_temp_db_leak.rs`, which asserts each line:

```
after open + WAL + CREATE TABLE                → 0 dir(s)
after deferred BEGIN + INSERT + COMMIT         → 0 dir(s)
after BEGIN IMMEDIATE, before any write        → 1 dir(s)  .tmp2tFqVt/tursodb-temp.db
after its INSERT + COMMIT                      → 1 dir(s)
after a SECOND BEGIN IMMEDIATE, same conn      → 1 dir(s)
after a second connection, opened              → 1 dir(s)
after BEGIN IMMEDIATE on the SECOND connection → 2 dir(s)
after dropping the second connection           → 1 dir(s)
after dropping the first connection            → 0 dir(s)
```

### The framing in this issue's title was wrong

turso does not leak. It creates the directory at the first immediate transaction
on a connection and removes it when that connection closes. What leaks is **our
exit path**: the unit takes a default SIGTERM on every restart and the process
does not unwind, so `TempDir::drop` never runs and whatever the writer connection
held is orphaned. `crates/app` contains no signal handling of any kind — grepped,
nothing for SIGTERM, graceful or shutdown.

That finally explains every count that did not add up:

* **~1/day steady state** — one restart-with-a-write per day. The writer connection
  takes its first `BEGIN IMMEDIATE` during the daily tick's first write job, and
  the next deploy orphans it.
* **20 deploys → 4 directories** (2026-09-01) — a restart only orphans something if
  that process ever wrote. Most of a deploy day's restarts land minutes apart with
  no write in between.
* **0 in the 1h50m after a restart, with ~23 connections open** — the read pool
  never issues `BEGIN IMMEDIATE`, and no write job ran in that window.
* **Not per-job** — one per connection *ever*, not per transaction.

## The cheap fix that isn't: `PRAGMA temp_store = MEMORY`

One pragma in the store's `PRAGMAS` would skip the directory entirely
(`create_temp_database` returns a `MemoryIO` database when `temp_store` is
`Memory`), and our temp database is always empty — no `CREATE TEMP TABLE` exists
anywhere in `crates/*/src`. It is still the wrong move: `temp_store` is the same
switch the **sorter and hash table** read (`TempFile::with_temp_store` in
`vdbe/sorter.rs` and `vdbe/hash_table.rs`), so setting it to `Memory` would route
every external sort and hash spill into RAM. That is precisely the failure the
`TMPDIR=/data/tmp` drop-in exists to prevent (issue 83: the spill landed in the
`PrivateTmp` tmpfs and failed an index build with "no storage space"), on a
490 GiB database. Trading an OOM for 4 KB of directory entries is not a fix.

Worth knowing for the future: `Connection::create_tempdir` honours `TURSO_TMPDIR`
and `SQLITE_TMPDIR` ahead of `TMPDIR`, so the temp *database* could be pointed
somewhere separate from the spill if that ever becomes useful.

## Shipped: `tender-db-tmpsweep`, its own daily unit

Option 1, built as its own unit rather than smuggled into `diskwatch` (which is
documented detection-only, and the issue flagged that as the one caution):

* `ops/watchdogs/tender-db-tmpsweep.{sh,service,timer}`, daily at 23:41, installed
  by `ops/watchdogs/install.sh` like the other four.
* Deletes only `.tmp*` directories **older than the service's
  `ActiveEnterTimestamp`** — now exact rather than merely safe, because a live
  connection's directory is provably removed the moment the connection closes.
* And only when their contents are turso's own (`tursodb-temp.db*` or
  `tursodb_temp_file*`); anything else is reported and left in place.
* If the start time cannot be read it **abstains** — a missing bound is a reason
  not to act, not a reason to fall back to an age guess.
* `TENDER_TMPSWEEP_DRY=1` reports without deleting; `install.sh` dry-fires it.

Proven on the box against fabricated cases before being pointed at `/data/tmp`:

```
before        : .tmpFOREIGN .tmpNEWlive .tmpOLDorphan .tmpSPILL
after dry run : .tmpFOREIGN .tmpNEWlive .tmpOLDorphan .tmpSPILL
after wet run : .tmpFOREIGN .tmpNEWlive
```

then live: `1 orphaned dir(s) removed, 4 KiB freed; 0 newer than the service start
left held` — `/data/tmp` back to 0.

### A second measurement error, same shape as the first

The first run of that fixture check printed a correct summary ("1 newer than the
service start left held") beside an `ls` that showed an empty directory. Both
could not be true. The cause: every fixture name begins with `.tmp`, and plain
`ls` hides dotfiles. `ls -1A` showed the truth. The first error in this issue was
broken quoting, this one was a hidden-file default — different mechanisms, same
lesson, and the same rule caught both: **two numbers that cannot both be true are
a signal to re-measure, not to pick one.**

### A live prediction, checked the same firing

The model above says a restart orphans a directory only if that process ever
took an immediate transaction. The deploy of `d552033` gave a free test of it:
the service ran 10:02:53 → 11:21:35 with no job in that window, was restarted,
and `/data/tmp` stayed at **0**. The naive "one per restart" reading, which the
first diagnosis nearly settled on, predicts 1. The sweep unit is installed and
enabled (`tender-db-tmpsweep.timer`, next fire 23:41 CEST), and its dry fire at
install reported the same 0.

## Correction: TWO artefacts leak, and the size claim was too small

Observed the same day the sweep shipped. `/data/tmp` held one orphan again after
the afternoon's restarts, and it was not the artefact this issue was written
about:

```
/data/tmp/.tmpzGLa6Y/tursodb_temp_file    21,573,552 bytes
```

`tursodb_temp_file` is a **sorter or hash-table spill** (`TempFile::new` in
turso's `io/mod.rs`), not the per-connection temp **database**
(`tursodb-temp.db`) that `BEGIN IMMEDIATE` creates. Different trigger — a query
whose plan spills — but the same `tempfile::TempDir`, held by the statement's
sorter, and the same escape: the process dies without unwinding, so the directory
outlives it.

Two things follow:

* **The sweep already covers it**, by luck rather than foresight in the sizing
  but deliberately in the code: its guard admits `tursodb-temp.db*` *and*
  `tursodb_temp_file*`, and the start-time rule is the same either way. The dry
  run correctly named this directory as removable and left nothing else.
* **"4 KB per orphan" was wrong.** That is the size of an empty temp database. A
  spill orphan is as big as the sort that made it — 21.5 MB here. The rate stays
  ~1/day; the volume is not bounded by anything but the largest query the process
  ran before it died. Still not urgent against a 1.7 TB volume, and still
  unbounded, which was always the actual argument.

Not established: which query produced this one. The directory is stamped 11:56:42
and the file was still being written at 12:21, which spans a restart and several
hand-run jobs; second-hand timestamps are not enough to name it and it is not
worth a campaign to find out. The sweep does not care which query it was.

## What is left

Nothing on the leak itself. Two optional follow-ons, neither urgent:

* **Graceful shutdown.** A SIGTERM handler that drops the `Db` would make the
  sweep unnecessary and would be the honest fix, but it is a real change to the
  server's exit path (in-flight jobs, the writer's transaction) for a 4 KB/day
  problem. Not worth it on this evidence; file it if a second reason appears.
* **Upstream.** There is nothing to report as a bug — turso's cleanup is correct.
  The only arguable improvement is creating the temp database lazily on first
  *use* rather than on `BEGIN IMMEDIATE`, which would help nobody here.
