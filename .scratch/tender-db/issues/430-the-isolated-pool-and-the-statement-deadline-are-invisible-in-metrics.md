# 430 — the isolated read pool and the new statement deadline are invisible in `/metrics`

Status: ready-for-agent — filed 2026-09-27 05:15 UTC from the hourly audit, while issue 120's engine deadline
was gating. Mine; next unit after 120 deploys.
Kind: observability
Relates to: 120 (its reopen trigger (a) needs this instrument; its 2026-08-04 section: "none of this was
visible"), 425, 241 (`tender_db_request_deadline_hits_total`, the pattern to copy), 417 (the `/v1/sql` gauges)

## What is missing

`IsolatedReads::available()` says it exists "for the readiness/debug surface, so the shed rate is observable
rather than inferred from 503s in a log". **Nothing in production reads it**: the only caller is a test
(`tests/api.rs`, `hold_slots_for_test`). Prod's `/metrics` (read 2026-09-27 05:12 UTC) carries
`tender_db_request_deadline_hits_total 0` and `tender_db_sql_pinned_computations 0` and nothing about the
REST isolated pool at all:

- how many of its 4 slots are busy right now,
- how many walk-capable requests it has SHED (503 "too many expensive filtered reads"),
- and — new with issue 120 — how many REST statements the engine's 25 s deadline STOPPED (503 "was stopped"),
  and how many abandoned reads were interrupted.

The last two are the instruments issue 120's owner position names as its reopen trigger: "deadline cuts
recurring in normal operation — real users hitting stalls". Since 120 a slow single statement is answered by
the ENGINE at 25 s, before the whole-request layer's 30 s, so `request_deadline_hits_total` no longer sees the
common case at all. Without a stop counter the trigger reads 0 whether or not users hit the limit.

## What to build

1. `tender_db_isolated_slots_busy` gauge (`SLOTS - available()`), `tender_db_isolated_shed_total` counter.
2. `tender_db_statement_deadline_stops_total{pool="main"|"isolated"}`: counted where `Error::Interrupt` becomes
   the 503 (`ApiError::from`) — label by pool needs the call sites, or one unlabelled counter is enough to start.
3. `tender_db_isolated_interrupts_total`: `Abandon::drop` found a registered connection and interrupted it.
4. `/metrics` test asserting the four series are present (absent-vs-zero rule: they are always measured, so zero).
5. Point issue 120's `## Verify` at the stop counter.

## Verify

    curl -s --max-time 10 https://tenders.zebreus.click/metrics | grep -cE '^tender_db_(isolated_slots_busy|isolated_shed_total|statement_deadline_stops_total|isolated_interrupts_total)'

- **done**: 4
- **open**: 0 (read 2026-09-27)
