# 494 — /v1/sql's latency and outcomes are unmeasured

Status: ready-for-agent — UNIT 1 BUILT 2026-10-08 (gate green on `8e7c404`, the session branch; rides with issue 490's
deploy B). Filed the same day from the owner's "the sql endpoint is the main way to consume our data and it needs
to be blazingly fast". NEXT: deploy with 490 B, then unit 2 (read the gauges for a day and file whatever the
slow log names).
Kind: observability / the SQL surface
Relates to: 417 / 425 (the pinned-computation gauges, the only /v1/sql series today), 241 (writer
contention, the precedent for measuring what "felt fine")

## What is wrong

`/metrics` exports two `/v1/sql` series: `tender_db_sql_in_flight` and the pinned-computation pair. Nothing
records:

- how long queries take;
- how many answer 200, 400, 408, 429 or 503;
- which queries are slow.

So "is the main consumption path fast" can only be answered by timing queries by hand, as was done on
2026-10-08 (top-5 by value 3 ms; a 984k-row range count 56–66 ms). A regression would be invisible: a
planner change, a turso bump (issue 457), or a new hot shape that 408s for every analyst who tries it.

## Design

`metrics.rs` is deliberately **gauges only**: a level re-read from its source, never an in-process
counter that a restart resets mid-series. So the measurement is a **rolling window**, not a counter:

- `SqlState` keeps a ring of the last 1,000 finished requests, each `(duration, outcome)`. The outcome
  is read off the response's status:
  - `ok` (200), `bad_request` (400), `timeout` (408), `rate_limited` (429), `busy` (503), `error`
    (anything else).
- `/metrics` serves:
  - `tender_db_sql_recent_requests{outcome}`: counts over the window;
  - `tender_db_sql_recent_ok_seconds{quantile="0.5"|"0.95"|"0.99"|"1"}`: latency of the window's 200s;
  - `tender_db_sql_recent_window_seconds`: how much wall time the window spans, so a reader knows
    whether 1,000 queries is an hour or a week.
  - A fresh process serves an empty window. Absence is honest, and the series appear with the first
    query.
- **Slow-query log:** one stderr line per request that took ≥ 1 s or timed out. It carries the duration,
  the status, the user id and the first 300 characters of the SQL with whitespace collapsed (never a
  token: the SQL body carries none).

## Units

1. Build the ring, the gauges and the log line, with tests: the window caps at 1,000, outcomes are
   classified by status, and the quantiles read the 200s only.
2. After deploying, read the gauges for a day and file whatever the slow log names.
