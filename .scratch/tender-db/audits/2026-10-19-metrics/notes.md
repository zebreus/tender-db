# Metrics audit (2026-10-19 23:5xZ, hourly AUDIT)

- `tender_db_legacy_adjacency_watermark` 47414824 == `SELECT max(id) FROM notices` 47414824 (one bounded rowid read, q1): the
  legacy OJS adjacency coverage is attested up to the newest notice.
- `tender_db_change_cursor` 958848784 == /health cursor; unchanged since project 2658 (no fold has written since; only dry jobs ran).
- `tender_db_dq_report_age_seconds` 156416 (43 h; data-quality is weekly, last 2026-10-18 04:22Z) — within its cadence.
- Writer: queue 0, wait 0 s; SQL: 0 timeouts / 0 busy in the recent window; deleted-open files 0. Nothing off.

# Changes-feed audit (2026-10-20 00:5xZ, hourly AUDIT)

- `/v1/changes?since=958848000&limit=60`: 60 events (26 tender, 34 lot), all `added` at 2026-10-19T07:35:55Z (the 10-19 daily
  fold). Spot-checked: tenders 8851942 / 8851943 serve version 1 (= the events' version), and lot 14691630 (event right after
  tender 8851942's) belongs to tender 8851942. The feed matches entity state. Not done yet: the 545 cohort (2651) and 548 v13
  (2654) persistence read-backs — no daily fold has run since those applies; re-run them after the 10-20 chain.
