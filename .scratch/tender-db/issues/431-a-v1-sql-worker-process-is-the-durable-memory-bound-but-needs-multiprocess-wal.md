# 431 — the durable `/v1/sql` memory bound is an out-of-process worker, gated on turso's multi-process WAL

Status: ready-for-agent — filed 2026-09-27 from issue 426's decision (step 3). The cheap layer-1 gate (426's
in-process function refusal) ships first; this is the durable isolation, and it is a multi-day commitment with a
hard prerequisite, so it is its own item. DO NOT start before the WAL soak below passes.
Kind: operations / architecture
Relates to: 426 (the measurement and the decision — read it first), 17 (the isolated runtime this extends to a
process), 425 (the deadline, which bounds time not bytes), 337 (turso temp files), ADR-0005 ("one systemd unit" —
this amends it), ADR-0006 (the Ubuntu deploy)

## Why a worker, not a cgroup cap

Issue 426 measured that the dangerous `/v1/sql` shapes allocate through the engine's INFALLIBLE global allocator,
so a runaway ABORTS the whole process — and that a unit-level `MemoryMax` (426 step 2) only converts a kernel OOM
kill into a cgroup OOM kill of the same single `server` process: the same full-service outage (API, SSE, webhooks,
a running ingest job). The ONLY way an out-of-memory `/v1/sql` query takes down just the query is to run it in a
separate process with its own memory limit, so the server survives and answers 503.

## The prerequisite that makes this multi-day

Both the server and the worker must open the same 675 GB DB file. turso serves multiple processes over one WAL
only under `experimental_multiprocess_wal` (`crates/vendor/turso/src/lib.rs`), which is marked experimental/untested
and touches every write path — including the ingest supervisor's, on the production DB. Before ANY worker code:

1. **WAL soak on a scratch copy** (reflinked, never the serving DB): (a) `kill -9` a worker mid-read while the
   server writes and checkpoints — does the `-wal` grow without bound, is the reader slot reclaimed (OFD locks
   should drop on Linux)? (b) a transaction over the 262,144-frame shared-index cap — read latency after the
   overflow falls back to WAL scans? (c) schema/ANALYZE pickup across processes (issue 429's bump). 
2. **An ADR-0005 amendment** — it currently says "one systemd unit". A worker (child process or a second unit) is a
   deliberate departure; record it.

## The shape, once the prerequisite passes (426's box report favours 2a for footprint)

A `/v1/sql` worker as a CHILD of the server (not a second unit): `RLIMIT_DATA` ~4 GiB + `oom_score_adj=1000` +
`PR_SET_PDEATHSIG`, the service unit gains `OOMPolicy=continue` (today it is `stop`, so any in-cgroup OOM stops the
whole unit). A runaway hits `RLIMIT_DATA`, malloc fails, the worker aborts (SIGABRT) — no kernel OOM, server up; if
a global OOM fires anyway, the +1000 score makes the worker the victim. The server frames queries to it over a pipe
and answers 503 while it respawns. NOTE the box's `SystemCallFilter=~@resources` kills a raw `setrlimit` syscall
with SIGSYS, but glibc's `setrlimit` calls `prlimit64` (in `@default`), so `RLIMIT_DATA` via glibc is fine — verify.
Do NOT reach `systemd-run` from the server (box report option 2c: a root-equivalent polkit grant on an
internet-facing process).

## Verify

    ssh -o BatchMode=yes root@zebreus.click "systemctl show tender-db -p MemoryMax; ps --ppid \$(systemctl show -p MainPID --value tender-db) -o comm="

- **done**: a `/v1/sql` worker child (or unit) with a finite `RLIMIT_DATA`/`MemoryMax`, and a soak record showing a
  killed worker leaves the WAL and the server healthy
- **open**: no worker; `/v1/sql` runs in `server` (read 2026-09-27)
