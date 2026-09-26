# 428 — the planner sees every public table as a million rows: measure `ANALYZE` on a snapshot before deciding

Status: ready-for-agent — filed 2026-09-26 21:xx UTC from Lennart's question "don't we need table statistics
on all user tables, so the engine can choose the best plan for any arbitrary user query?". Mine to decide;
the decision is gated on the measurement below, not on anyone's word.
Kind: query planning (the `/v1/sql` surface first, the API's own read paths second)
Relates to: 421 (a plain JOIN planned backwards — the kind of trap stats may fix), 112 / 114 / 122 (plan gates
that are valid ONLY while `sqlite_stat1` is empty, and say so), 256 (`ANALYZE plan_*`), 425 / 426

## What is true today

- turso 0.7.2's optimizer is cost-based (`translate/optimizer/cost.rs`) and DOES use `sqlite_stat1`: the table
  row count (`RowCountEstimate::AnalyzeStats`) and per-index `avg_rows_per_distinct_prefix` for seek
  selectivity (`constraints.rs`, `cost.rs` ~286). Without stats it assumes **1,000,000 rows for every table**
  (`cost_params.rs`, `rows_per_table_fallback`).
- We run `ANALYZE` only on three internal tables (`org_match_keys`, `plan_prev_edge`, `plan_notice`). Every
  public table — `tenders` 8.6M, `lots` 14.0M, `tender_versions` 14.5M, `changes` 625M, the satellites in the
  tens to hundreds of millions — looks the same size to the planner.
- **That is deliberate, and recorded** (issue 122): every hand-tuned read path of the API and every plan gate
  (112/114/122) was designed and verified against the stats-free planner; a bare `ANALYZE` "would populate
  every read-path table and silently change the planner underneath every measurement". The gates check for
  it and report VOID if stats appear.

## Why it is worth revisiting

`/v1/sql` runs arbitrary queries the app never tuned, and the documented user-facing traps (421's backwards
JOIN, the `kind IN (…)` prefix walk) are exactly the mistakes a planner makes when it cannot tell a 14M-row
table from a 14k-row one. Stats could fix those for every analyst at once — or could move the app's own
tuned plans somewhere worse. Neither is known.

## The experiment (on a SNAPSHOT, never the serving DB)

1. Copy of the newest weekly snapshot (`/data/db/snapshots/tender-db-*.db`, ~672 GB) — run `ANALYZE`, timed
   (the 2026-08 research measured 11.3 s at 10 GB; at 630 GB it is unmeasured — expect tens of minutes).
2. Before/after on that copy: (a) the app's plan gates and hot read plans (`hot_read_plans.sh`, the 112/114/122
   gates — they must be pointed at the copy and will report VOID-with-stats by design; read the plans
   themselves); (b) the known `/v1/sql` traps (421's plain JOIN, the `kind IN` shape, 423's lots shapes); (c)
   timings of the list endpoints' statements through the engine.
3. Decide from the diff: all tables, a subset (only those whose plans improve and none regress), or none. If
   yes, it becomes a scheduled weekly job (it holds the writer for its duration, so it runs in the quiet
   window like the data-quality run), and the stats-absence preconditions in the plan gates change to
   "stats as of <date>" rather than "no stats".

## Verify

    ssh -o BatchMode=yes root@zebreus.click "echo 'SELECT COUNT(*) FROM sqlite_stat1' | /root/sq.sh"

- **done**: the experiment's before/after is recorded here and the decision taken (the count then reads
  whatever the decision implies)
- **open**: the query is refused — `sqlite_stat1` is not in the public surface, and nothing is recorded here
  yet (2026-09-26)
