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

## Comments

### 2026-07-20 — FETCH phase started (via the issue-16 supervisor /admin), core-16

The raw-archive fetch half of this issue is being driven in production through
the in-app supervisor's `/admin` API (no CLIs), newest→oldest, monthly
packages. Processing is deliberately NOT run yet (walker fixes for nested
monthlies + DÖE zips land later).

Measured real monthly package sizes per era (HTTP Content-Length, June sample):

| Era | June sample | Note |
| --- | --- | --- |
| 1993–2003 (text) | 80 MB (1995) → 233 MB (2003) | grows with EU languages |
| **2004–2010 (text)** | **0.9–2.4 GB** (2007-06 = 2.43 GB) | ~20-language bundles — 70% of total |
| 2011–2022 (TED_EXPORT) | ~130–180 MB | compact XML |
| 2023–2026 (eForms) | 296 MB (2024) → 395 MB (2026) | |
| DÖE monthly | 92 MB (2024-06), 168 MB (2022-12) | German eForms-DE |

Projected full archive: **TED ≈ 196 GB + DÖE ≈ 5 GB ≈ 200 GB**, under the
~250 GB guard (500 GB volume must also hold the DB). The 2004–2010 text era is
~142 GB (70%); dailies would NOT shrink it (text dailies bundle all languages
too), so deferral is the only lever — the fetch driver has a `df /data` guard
that aborts before 245 GB used and defers the oldest years if disk tightens.

Fetch driver: `/opt/tender-db/backfill-fetch.sh` (tmux session
`core16-backfill`), log at `/opt/tender-db/backfill-fetch.log`. Final measured
sizes / registry counts / disk state recorded on completion.
