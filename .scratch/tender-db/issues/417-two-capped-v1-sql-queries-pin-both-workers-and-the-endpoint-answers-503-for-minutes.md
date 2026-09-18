# 417 — two capped `/v1/sql` queries pin both runtime workers, and the endpoint answers `503 saturated` until they finish (12+ minutes measured)

Status: ready-for-agent — found 2026-09-18 15:51 UTC by the owner's own census read (issue 397 unit 2): two join-bridged range reads each ran past the 10 s cap, the caller got its two `408`s in 20 s, and `/v1/sql` then answered `503 sql backend busy: the SQL runtime is saturated` to EVERY query — `SELECT 1` included — from 15:51 until the two computations finished (still pinned at 16:02; the duration is recorded at the foot when it clears). The public API was untouched throughout (`/health` 0.7 s, list pages normal): the SQL runtime is isolated (issue 17), which is exactly what confined it.
Kind: defect (availability of `/v1/sql` — a capped query is not cancelled, so the cap bounds the caller's wait but not the worker's)
Relates to: 17 (the isolated SQL runtime), 239 (the time limit and its 408), `docs/agents/prod-box-reads.md` (the traps table gained this shape today), 397 (the census that hit it)
Blocked by: nothing

## Observed

`crates/app/src/v1/sql.rs`: `SQL_RUNTIME_THREADS` workers (two), and `in_flight` is "counted rather than gated by a semaphore, and the difference is the point: a turso aggregate that never yields cannot be interrupted (there is no `interrupt()`), so its future keeps computing after the client has gone and after `AbortOnDrop` has fired". So the design already knows: a capped query pins its worker to completion. What was not priced is how long "completion" can be for a query that looked bounded — 100k tenders joined through `tender_versions` to `notice_codes` by notice id is ~300k seeks plus whatever plan turso chose, and it ran for many minutes — and that two of them (the second sent 10 s after the first's 408, before anyone knew the first was still running) take the whole endpoint down for that long.

## Why it matters

`/v1/sql` is the bounded-read surface every audit, census and `## Verify` block leans on (`docs/agents/prod-box-reads.md`). One caller's two mistakes made it unavailable to everyone for the length of two runaway computations, and the 503 gives no estimate of when it comes back. The rule side is fixed in the docs today (size a join-bridged read on a 1–2k slice first; never fire the second before the first answers); the mechanism side is this issue.

## Repro

1. `grep -n "SQL_RUNTIME_THREADS\|cannot be interrupted\|SATURATED" crates/app/src/v1/sql.rs`.
2. Two concurrent queries that each exceed the cap (a local `SqlState::with_timeout` at 50 ms and a `SELECT COUNT(*)` over a big synthetic table do it in a test — `both workers occupied` is already asserted at `sql.rs:1919`); a third query is `503` until both return.

## Verify

    grep -c "spawn_blocking\|abandoned\|ABANDONED_CAP" crates/app/src/v1/sql.rs

- **done**: `1` or more — a capped query is moved off the shared workers so the endpoint stays answerable
- **open**: `0` (read 2026-09-18: `0`)

## Done when

- A query past its cap no longer occupies one of the runtime's request workers: run each query on its own blocking thread (`spawn_blocking` or a dedicated thread per query) and on timeout ABANDON that thread rather than the pool — it finishes on its own, the pool serves the next request. Bound the abandoned set (a small cap, e.g. four; past it the endpoint says so with the same `503`, now honest about a real limit rather than two pinned threads), and expose the count as a `/metrics` gauge so the next such event is visible while it happens.
- Check turso for an interrupt hook first (`Connection::interrupt`, a progress handler, anything that makes a running statement return); if one exists in the pinned version, use it and the abandonment cap becomes the fallback for statements that do not honour it.
- The `503` body says how many computations are pinned and since when.
- A test: with a 50 ms cap, two runaway queries plus a third `SELECT 1` — the third answers, and the gauge reads 2.
