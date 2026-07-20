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

### 2026-07-20 — VERIFICATION harness landed (spec §5), verify-agent

`crates/ingest/src/bin/verify.rs` — the standing acceptance tool. Black-box
against a deployed instance (`--base-url`, default `https://tenders.zebreus.click`),
report + non-zero exit on failure, `--json` for machines. Three checks, each
vs an *external* ground truth (never the dashboard):

1. **Coverage** — per-year TED `COUNT(DISTINCT publication_id)` vs the vendored
   `crates/app/data/ted-notice-counts.csv`, ±2 % tolerance (upstream counts are
   approximate; partial years are a floor, not a ceiling).
2. **Search-API cross-check** — for eForms-era sample days, a *set-membership*
   check: does the instance hold the publication ids TED lists for that day.
   Deliberately **not** a raw day-count: the projection stores TED's
   **dispatch-date** as `published_at` (issue 136 = dispatch 07-16 / publication
   07-17), so a same-date count is unsound while id-membership is exact.
3. **Era ladder** — one real notice per format era (from the fixtures README)
   resolves through `/v1`: Notice exists → Tender exists → eForms CAN carries a
   winner.

Coverage + era-ladder need an API token (`--token` / `TENDER_API_TOKEN`) for
the read-only `/v1/sql` endpoint (per-year/per-notice counting isn't in the REST
filters); without one they report *skipped*, not passed. Unit tests cover the
CSV parse, the tolerance/verdict logic, id validation, `has_winner`, and the
overall pass/fail gate. Integration test intentionally skipped (would need a
circular ingest→app dep; the tool's real target is production).

**Real run against production today (rev 7199300, partial data):** RESULT FAIL,
correctly — the instance holds only 2026-07-16 (3715) + 2026-07-19 (3702) ≈
**7 439 TED notices**, so 1993–2025 are all `MISSING` and 2026 is `SHORT`
(partial). Set-membership: **2026-07-17 → 250/250 PASS**, other sample days
0/250 (not yet backfilled). Two findings worth flagging for the backfill/results
work, not tool bugs:
- **`published_at` = dispatch-date, not publication-date.** Semantic mismatch
  vs TED's OJS publication date (drove the set-membership design). Worth a
  deliberate decision — the field name implies publication date.
- **Results layer not materialised in prod:** 2 529 subtype-29 CANs present but
  **zero `lot_results` rows** — so the eForms-CAN era-ladder check fails ("no
  winner"). Issue 13's projection is not populating results on the live deploy.

The tool is the final acceptance gate: run `verify --token …` against
production once the backfill + results projection complete; green = spec §5 met.
(A production `acceptance-verify` account exists for this; mint a fresh token
from the dashboard — the one used today was revoked.)

### 2026-07-20 — FULL PROCESS+PROJECT RUN enqueued (rev 2945e9e), run-driver

Deploy of the store-migration fix (rev 2945e9e — additive `published_at`/
`dispatched_at` column migrations) landed 22:10 UTC; /health ok on public+local,
`deployed-rev`=2945e9e. The prior enqueue (jobs 469–472) had died on
`table notices has no column named published_at`; that column now exists.

Enqueued via /admin (all 202), running one-at-a-time in order:
1. process ted monthly (all) — 401 packages, 1993→2026
2. process ted daily (all)
3. process doe monthly (all)
4. process doe daily (all)
5. project rebuild=false

Job 1 confirmed progressing: 1993-02→1993-07, notices 3.7k→30k, no column
error. Monitor: VPS tmux `bf15-monitor` → `/opt/tender-db/backfill-status.log`,
one line / 120s (job progress, notices/s, RSS, df /data, health).
/data at 38% (189G/500G) at start.
