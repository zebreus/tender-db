# 417 — two capped `/v1/sql` queries pin both runtime workers, and the endpoint answers `503 saturated` until they finish (12+ minutes measured)

Status: **DONE 2026-09-18** — FIXED, gated (127/127) and DEPLOYED at `0005861` 17:08 UTC, `## Verify` read at 17:09: `tender_db_sql_pinned_computations 0`, `tender_db_sql_pinned_since_seconds 0`, `tender_db_sql_in_flight 0` on an idle box, and `SELECT 1` answers through the endpoint. Every `/v1/sql` computation now runs on its own blocking thread; an abandoned one is counted from the moment its request stops waiting until it returns, admission refuses at four with a 503 that says how many and since when, and the end-to-end test proves a cheap query answers at once behind two capped bombs. Was: found 2026-09-18 15:51 UTC by the owner's own census read (issue 397 unit 2): two join-bridged range reads each ran past the 10 s cap and pinned the two workers for **13.5 minutes** (15:51:50–16:05:23), during which `/v1/sql` answered `503 saturated` to everything, `SELECT 1` included; a third capped read at 16:33 pinned one again. The public API was untouched throughout (the SQL runtime is isolated, issue 17).
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

    curl -s --max-time 20 https://tenders.zebreus.click/metrics | grep -E '^tender_db_sql_(pinned_computations|in_flight) '

- **done**: two lines — the gauges are served; `pinned_computations` reads `0` on an idle box, and the endpoint stays answerable while it is under `4`
- **open**: no such lines (read 2026-09-18 before the deploy: none; at `0005861` 17:09 UTC: both lines, `0` and `0`)

## Done when

- A query past its cap no longer occupies one of the runtime's request workers: run each query on its own blocking thread (`spawn_blocking` or a dedicated thread per query) and on timeout ABANDON that thread rather than the pool — it finishes on its own, the pool serves the next request. Bound the abandoned set (a small cap, e.g. four; past it the endpoint says so with the same `503`, now honest about a real limit rather than two pinned threads), and expose the count as a `/metrics` gauge so the next such event is visible while it happens.
- Check turso for an interrupt hook first (`Connection::interrupt`, a progress handler, anything that makes a running statement return); if one exists in the pinned version, use it and the abandonment cap becomes the fallback for statements that do not honour it.
- The `503` body says how many computations are pinned and since when.
- A test: with a 50 ms cap, two runaway queries plus a third `SELECT 1` — the third answers, and the gauge reads 2.

## The duration, measured

Two queries, sent 10 s apart at 15:51:40 and 15:51:50 (each `408` after its 10 s), and every
`SELECT 1` through `/root/sq.sh` answered `503 saturated` until **16:05:23** — the first `200` after
13 minutes 33 seconds. Load average on the box read 1.13 for the period (two pinned computations
on a mostly idle machine); `/health`, the list pages and the seeded walks were unaffected.

## Second event, 16:06 — the same shape on 2,000 tenders also hit the cap: it is the plan, not the size

Re-sized per the new rule: the same join over `t.id BETWEEN 7960000 AND 7961999` (2,000 tenders,
50× smaller) — `408` after 10 s all the same, and one worker is pinned again as this is written. So
the range bound never bounded anything: turso is not driving from `tenders` by primary key and
seeking `notice_codes` per version; it is driving from `notice_codes` by `field_id` (the equality on
a low-cardinality column, the traps table's known shape) and walking the corpus. The census moves
off `/v1/sql` — the codelist was already settled from the committed fixtures — and the plan is to be
read LOCALLY on a scratch database, which is the rule the traps table already states and this
caller skipped twice. Two lessons for the rule, both now in `docs/agents/prod-box-reads.md`: a
join-bridged read is sized by its plan, not its range; and after ONE 408 the next read of any
shape waits until the runtime answers `SELECT 1` again.

## The plan, read locally 16:20 — `SCAN notice_codes`, and the `+` that fixes it

`EXPLAIN QUERY PLAN` on a scratch database with the real schema (a throwaway probe, run once and
deleted):

| statement | turso's plan |
| --- | --- |
| as sent (`… AND c.field_id = 'TXT-NC' … GROUP BY c.code`) | **`SCAN notice_codes AS c`**, `SEARCH v USING INDEX tender_versions_notice (caused_by_notice_id=?)`, `SEARCH t USING INTEGER PRIMARY KEY`, sorters for GROUP BY and ORDER BY |
| the same with `+c.field_id = 'TXT-NC'` | `SEARCH t USING INTEGER PRIMARY KEY (rowid=?)`, `SEARCH v USING INDEX sqlite_autoindex_tender_versions_2 (tender_id=?)`, `SEARCH c USING INDEX sqlite_autoindex_notice_codes_1 (notice_id=?)` |
| the join without the GROUP BY | still `SCAN notice_codes` — the equality on `field_id` alone steers it |

So the range on `t.id` was never the driver: the planner took the equality on the unindexed
`field_id` as its entry point and walked the largest table in the corpus, seeking versions per
row — the fourth instance of the traps table's "steered by the last clause" shape, and the same
cure (`+` on the column that must not drive). The two queries that pinned the runtime for 13.5
minutes were each a full pass over `notice_codes`. The docs row now says exactly that, and this
issue stays open on its own clause: a capped computation should not hold a request worker.

## Third event, 16:33 — the caller broke the rule it had just written

With the runtime free again and the `+` cure in hand, a notices-driven shape (`notices n JOIN
notice_codes c ON c.notice_id = n.id AND +c.field_id = 'TXT-NC' WHERE n.id BETWEEN 2000000 AND
2001999`) was sent WITHOUT reading its plan locally first — on the assumption that a primary-key
range on `notices` would drive it. `408` after 10 s; one worker pinned again for the length of
whatever turso chose to do. Three capped computations in 45 minutes, all from one caller, all the
same class: a join into `notice_codes` whose plan was assumed rather than read. The rule in
`docs/agents/prod-box-reads.md` is therefore absolute now, not advisory: **no join-bridged read
reaches `/v1/sql` without its `EXPLAIN QUERY PLAN` read on a scratch database in the same
session, and the plan pasted into the write-up beside the numbers.** The census itself is closed —
the codelist was settled from the committed fixtures before any of this — and the plan for the
notices-driven shape will be read locally before it is ever sent again.

## FIXED 2026-09-18 — the computation gets its own thread; an abandoned one is counted and capped

**The mechanism (`crates/app/src/v1/sql.rs`).** The isolated runtime's two workers now do only
what can always finish — take the per-user permits, borrow the reader, coordinate — and the
computation itself runs inside `tokio::task::spawn_blocking` on its own thread, under a
per-query current-thread runtime so the in-task timeout still drops a streaming query between
rows and frees its reader promptly. turso still cannot be interrupted, and the non-yielding
aggregate still runs to completion; what changed is WHERE: on a blocking thread of its own, so
the workers are free, the next query gets a fresh thread, and the runtime never saturates on
computations nobody is waiting for. `max_blocking_threads(16)` sizes the pool; admission keeps
the live count under it, so the pool's own queue is never reached.

**The accounting.** Each computation carries a three-state record — RUNNING, FINISHED,
ABANDONED — resolved by one atomic exchange from each side: the handler's `Watch` guard marks it
ABANDONED on every way out that is not the computation finishing first (the backstop, a
disconnect, an early return), and the `Running` guard moved into the blocking closure marks it
FINISHED when the computation actually returns. Pinned = abandoned-and-still-running, counted
exactly from the abandonment to the return whichever transition lands first (the unit test
drives both orders), with the unix second of the oldest current abandonment beside it.
Admission refuses at `ABANDONED_CAP = 4` with a 503 that says so — *"4 abandoned computation(s)
are still running on their own threads and cannot be interrupted (cap 4), the oldest since unix
second N; they finish on their own"* — instead of the wordless `saturated`, which stays only for
the never-polled backstop case that the change makes near-impossible. `/metrics` serves
`tender_db_sql_pinned_computations`, `tender_db_sql_pinned_since_seconds` and
`tender_db_sql_in_flight`.

**Tests.** Unit: `an_abandoned_computation_is_counted_until_it_returns_and_a_finished_one_never`
(finished-then-left counts nothing; left-then-finished counts exactly in between, a second
abandon is a no-op, the since clears) and the updated in-flight test. End to end
(`crates/app/tests/sql.rs`, `an_abandoned_computation_keeps_no_worker_and_is_counted`): at a
300 ms cap, two non-yielding bombs each 408, `/metrics` then reads
`tender_db_sql_pinned_computations 2` with a since, and `SELECT 1` answers `200` in well under
two seconds — the request that was a `503` for 13.5 minutes this afternoon. The suite's 15 pass.

**Not built, deliberately.** turso's `Connection` in the pinned version has no interrupt or
progress hook to check for (the module's own comment, unchanged); the cap-of-four refusal is the
whole bound, and an operator who sees the gauge at 4 for minutes has the number and the time to
act on. Read on prod after the deploy (`0005861`, 17:09 UTC): the three gauges served at `0`, `SELECT 1`
answering — the `## Verify` block. No runaway query was sent to the box to prove the rest; the
end-to-end test did that, and the next real one will show on the gauge instead of as a silent
`503`.

