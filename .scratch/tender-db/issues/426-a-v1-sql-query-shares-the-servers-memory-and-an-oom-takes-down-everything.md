# 426 — a `/v1/sql` query shares the server's memory and process, so an out-of-memory query takes down everything

Status: ready-for-agent — filed 2026-09-26 21:xx UTC from the owner's review of how user SQL is isolated
(asked by Lennart). Not observed; a structural gap.
Kind: operations / safety
Relates to: 17 (isolated runtime — threads, not memory), 337 (turso's temp database), 425 (cancellation —
also bounds how long a big sort can grow)

## What

`/v1/sql` runs in the server process on its own threads and 4 reader connections, which bounds CPU (about
4 of the box's 32 cores) and keeps the REST runtime free. Nothing bounds **memory or temp space per query**:
a large `ORDER BY`, `GROUP BY` or hash join builds its state in the process, the result caps (10,000 rows /
10 MB) only bound what is RETURNED, and the box has 62 GB. An out-of-memory kill would take the whole
service down — API, SSE, webhooks, ingest — not just the query, and systemd would restart it mid-job.

## Options (decide when picked up; measure first)

1. **A worker process for `/v1/sql`** with its own memory limit (a systemd scope/slice with `MemoryMax`, or
   `RLIMIT_AS`), talking to the server over a pipe: a runaway kills the worker, the server answers 503 and
   respawns it. Strongest; most work.
2. **`MemoryMax`/`MemoryHigh` on the service unit** plus turso's `cache_size` for the SQL connections: cheap,
   but an OOM still takes the whole service.
3. After 425: an engine deadline bounds how long a sort can GROW, which bounds memory in practice for most
   shapes — measure peak RSS of the worst shapes under the deadline before building option 1.

Leaning: 425 first, then measure peak RSS for the worst shapes, then decide between 1 and 2.

## Verify

    ssh -o BatchMode=yes root@zebreus.click "systemctl show tender-db -p MemoryMax"

- **done**: a finite `MemoryMax` on the unit or on a dedicated SQL worker scope, and the peak-RSS
  measurement recorded here
- **open**: `MemoryMax=infinity` (read 2026-09-26)
