# Metrics audit (2026-10-19 23:5xZ, hourly AUDIT)

- `tender_db_legacy_adjacency_watermark` 47414824 == `SELECT max(id) FROM notices` 47414824 (one bounded rowid read, q1): the
  legacy OJS adjacency coverage is attested up to the newest notice.
- `tender_db_change_cursor` 958848784 == /health cursor; unchanged since project 2658 (no fold has written since; only dry jobs ran).
- `tender_db_dq_report_age_seconds` 156416 (43 h; data-quality is weekly, last 2026-10-18 04:22Z) — within its cadence.
- Writer: queue 0, wait 0 s; SQL: 0 timeouts / 0 busy in the recent window; deleted-open files 0. Nothing off.
