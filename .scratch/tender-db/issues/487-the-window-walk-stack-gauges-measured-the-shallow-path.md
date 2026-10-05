# 487 — the window-walk stack gauges were budgeted from the shallow path; the whole `--lib` run aborts

Status: done — 2026-10-05: the measured budgets (456 / 264 / 360 KiB) landed in `bb87cdc` with 486 unit 1; every gate since (`bb87cdc`, `f6f8e5d`, `9f9db13`, `9c83fb3`) GATE-EXIT=0.
`run_spec_futures_stay_inside_their_size_budgets`). Gate it with 486 (`ops/check.sh`) and commit it as its own
commit or with 486. Done when a gate passes with it.
Kind: test infrastructure (the issue-467 tripwire)
Relates to: 467 (the stack-budget gauges), 486 (whose review found it red), the 2026-10-04 note at the budgets

## What is wrong

`cargo test -p model -p store -p ingest -p tender-db --features tender-db/server --lib` aborts (GATE-EXIT=101,
SIGABRT) with `thread 'issue 467: run_backfill_tender_links's poll frame is over its 196608-byte stack budget' has
overflowed its stack`. The gauge test alone passes. The cause is not 486: on 2026-10-05 a detached worktree of
unmodified `3f66efe`, built in the same target dir, aborted the same way. The binary hash was the same, so it really
ran HEAD's code: 348 ingest lib tests, where 486 has 349.

## Why

The first poll of each window walk (`run_backfill_tender_links`, `run_procedure_key_census`, `run_buyer_role_census`,
`run_requeue_uuid_hubs`, and `run_audit_fts_ids` too) goes one of two depths into the store/turso chain. Which one
depends on timing (whether turso's read completes inline). The 192 KiB budgets came from runs on the shallow path.

Measured 2026-10-05 with a temporary paint gauge. It filled the gauge thread's stack with a pattern, polled once, and
read back the deepest word written. With a 4 MiB stack the poll took the deep path every time, isolated and in the
whole `--lib` run alike:

| poll | deep path | old budget | new budget |
|---|---|---|---|
| `run_backfill_tender_links` | 429 KiB | 192 | 456 |
| `run_procedure_key_census` | 424 KiB | 192 | 456 |
| `run_buyer_role_census` | 422 KiB | 192 | 456 |
| `run_requeue_uuid_hubs` | 237 KiB | 192 | 264 |
| `run_audit_fts_ids` | 336 KiB | 330 | 360 |

Painted inside its own 192 KiB budget, the same poll read the shallow path (it saturated the paint without
overflowing). The deep figure matches the seeded `run_buyer_role_census over a parsed notice` gauge: 422 KiB used,
budget 456. That gauge already budgets the same store chain on the deep path. Every other gauge measured inside its
budget.

## Fix

Use the deep-path figure plus about 24 KiB for each budget (table above). The paint instrumentation is not kept. In
production every walk runs through `off_frame` on its own large stack, so this changes the tripwire's margin only. It
does not change any production frame.

Verified 2026-10-05: the whole `--lib` run (4 packages, server feature) passed twice in a row with the new budgets,
GATE-EXIT=0 both times.
