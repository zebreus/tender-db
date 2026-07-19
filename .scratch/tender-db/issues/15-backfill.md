# 15 — Full backfill + continuous operation + verification

Status: ready-for-agent
Blocked by: 09, 10, 11, 12, 13, 14

Goal: the full dataset per spec: TED 1993→ + DÖE 2022-12→, live updates on
schedule, verified.

Scope:
- Backfill runs on the VPS (newest→oldest; monthly packages; text era EN
  files): fetch → process → project, PK-only-then-index strategy for the
  initial load (turso-scale.md), ANALYZE after; disk headroom watch.
- Continuous mode: TED daily 09:35 CET, DÖE daily T+1; finality re-fetch;
  scheduler in-app.
- Verification (spec §5): per-year notice counts vs research ground truth;
  Search-API count assertion for sample days; era-ladder spot checks (one
  known notice per era through the API); quarantine review — every
  quarantined notice triaged (bug → fix + reprocess, or documented).
- Dashboard shows full coverage; record final DB/archive sizes in the
  operations runbook.

Acceptance: dashboard coverage matches ground truth within documented
tolerance; live updates observed for 3 consecutive days; quarantine
contains only triaged entries.
