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

Progress before interruption: reached 1994-05 (pkg 18/401), ~115k notices,
peaks ~390n/s through the mid-90s text era, RSS ~150MB, /data steady 38%.

### 2026-07-20 — INTERRUPTED by a9b0883 view-fix deploy; re-enqueued, run-driver

The team lead deployed a9b0883 (store: drop-and-recreate VIEWs at open — the
public /v1/tenders + /v1/notices were 500ing on stale pre-issue-18 view
definitions) mid-backfill, deliberately: a 500ing main endpoint outweighs the
interruption. The service restart wiped the in-memory supervisor queue (job 1
mid-run at ~pkg 18/401 + queued jobs 2–5). Re-walk of already-processed
packages is idempotent (identity dedup) so the re-enqueued run fast-forwards
past the already-ingested notices.

Confirmed on the box: deployed-rev=a9b0883, service active, /v1/tenders 200
with items (incl. dispatched_at), the supervisor queue empty and IDLE (original
job 1 was killed mid-run before it could record to job_log — recent[0] reverted
to the pre-run 473/project). Re-enqueued the same five jobs (all HTTP 202); job
1 fast-forwarded pkg 10→19/401 in ~12s (all dups, notices counter 0) then
resumed real parsing past 1994-05. Monitor re-armed (tmux logger untouched
throughout). /data steady 38%.

### 2026-07-21 — CORRUPT-PACKAGE incident + walker-resilience fix, run-driver

`process ted monthly (all)` (job#474) ERRORED after 996s on a truncated inner
ZIP: `19960208_1996027.tar.gz/SV_19960208_1996027_ISO_ORG.zip: invalid Zip
archive: Could not find EOCD`. Because the walker propagated the ZIP-open
failure and `run_process` aborts the job on the first bad package, ALL TED from
1996-02→2026 was left unprocessed (the 2004–2010 text era, ~70% of the dataset,
is downstream of it). Diagnosed on the box: the inner zip is exactly 393216 B
(384 KB block boundary), no End-Of-Central-Directory — TED's upstream 1996
archive is baked-in corrupt (the outer .tar.gz decompresses cleanly and holds
this truncated member), so re-fetch cannot help and a 33-year archive will have
more such rot.

Fix (rev fa873ec, ADR-0004 generalized): corruption is never fatal —
- **member** (truncated/unreadable inner ZIP bundle or entry) → quarantine
  bucket with reason (visible metric, reprocessable), never a policy-skip;
- **package** (unreadable outer container/nested tar) → `corrupt-package`
  quarantine + continue to the next package (`process_package_resilient`, used
  by the supervisor and the process() loop) — the job never dies on one file;
- **systemic** (db errors) stay fatal.
Regression tests: the byte-exact SV_19960208 fixture is quarantined-not-fatal;
a bad ZIP entry is corruption-tagged; a whole-source run continues past an
unreadable package. Deployed via ./deploy.sh, then re-enqueued the five jobs.
EXPECT a bump in quarantine (reason `unreadable zip bundle …` / profile
`corrupt-package`) across old eras — recorded per plan, not fatal.

### 2026-07-21 — RESTART-RECOVERY recon (local machine reboot; VPS untouched), run-driver

Local dev machine rebooted; production (root@zebreus.click, rev fa873ec) kept
running throughout — the reboot only cost the local driver session, not the run.
Resumed babysitting. State verified ~09:17 UTC:

- **Job 1 healthy and past the danger zone.** `process ted monthly (all)` at pkg
  **217/401 (2011-02)**, ~3.54M notices, ~120 n/s, RSS ~766MB, /data 221G/500G
  (45%). Packages walk **oldest→newest**, so it has already sailed past
  **1996-02** (pkg ~37) — the exact truncated SV_19960208 inner zip that killed
  job#474 — without erroring. **The walker-resilience fix (fa873ec) is proven in
  production.** The heavy 2004–2010 text era (70% of the dataset) is also behind
  us: notices climbed 3.7k (1993) → 2.23M (2007-06) → 3.54M (2011-02). Remaining
  184 packages are the compact TED_EXPORT (2011–2022) + eForms (2023–2026) eras —
  smaller and faster than what's done.

- **`q=9` explained — NOT pre-restart duplicates.** Queue is jobs 2–10. Jobs 2–5
  are the original backfill remainder (ted daily, doe monthly, doe daily,
  project). Jobs **6–10 are the in-app daily scheduler's automatic tick**
  (`enqueue_daily`, supervisor.rs:478): probe → ted daily → fetch doe daily
  2026-07-20 → process doe daily → project. Proof: `q` jumped 4→9 at **07:36 UTC
  = 09:36 Berlin**, exactly the scheduler's 09:35 Europe/Berlin cron. Jobs 7/9/10
  are functionally idempotent duplicates of 2/4/5 (re-walk dedups; project
  rebuild=false is idempotent); 6 (probe) and 8 (doe daily fetch) are legit new
  scheduled work. **All harmless — left in place.** This is actually the
  continuous-mode scheduler demonstrating itself (an acceptance item), not a bug.

- **Raw-archive FETCH complete.** backfill-fetch.log: `DONE; df used=200GB`.
  Final on disk: **397 TED monthlies + 44 DÖE monthlies**, 200GB used — under the
  ~245GB fetch guard. Fetch driver's job is finished; DB growth is now the disk
  driver (221G/45% and climbing gently, well clear of the ~70% flag).

- **Quarantine so far:** not visible mid-run (`current.counts` is null until a
  job completes; dashboard is WASM-rendered so no server-side scrape). Per the
  "no dev shortcuts in prod" rule I did not query the prod DB directly. The only
  completed process job since the fix — #475 `ted daily (all)` — reported
  `0 quarantined`. Expect the 1996 SV member + old-era rot to surface as
  `corrupt-package` / `unreadable zip bundle …` in **job 1's final counts** on
  completion; will record then.

Monitor tmux (`bf15-monitor` → backfill-status.log, `bf-fetch-watch`) untouched
and still ticking. Standing by on a background wait for job-1 completion / disk
threshold; will verify each queued job starts and log milestones.

### 2026-07-21 — INTERRUPTED by da2ab87 deploy (issue 20 fix); lead re-enqueued, run-driver

The team lead deployed rev **da2ab87** (issue 20 — read-only Db accessors moved
to a WAL reader pool, to fix the dashboard/`/admin/jobs` stalls under ingestion)
at **09:56 UTC**, deliberately mid-backfill. The restart killed job 1 (was at
~pkg 218/401, 2011-03) and wiped the in-memory supervisor queue as expected.

**Watcher miss (my fault, fixed):** the deploy-green watcher I had armed never
fired, so the queue sat empty ~09:56→10:57 (1h) until the lead noticed and
recovered. Root cause: it was a single long-lived `ssh 'while true'` with **no
SSH keepalive** — when the service restart/network blip dropped the connection,
the ssh client hung half-open on a dead socket, producing no output and never
exiting, so the background task never completed and I was never re-woken. A
lone persistent tunnel is a silent single point of failure. Fix: watchers now
use `ServerAliveInterval=15 ServerAliveCountMax=4 ConnectTimeout=10` (dead peer
detected → ssh exits → I re-wake) **and** a bounded ~24-min heartbeat cap so the
watch always returns and re-arms — the max blind window is now minutes, not open-
ended. It also early-exits on a rev change, so a future redeploy re-wakes me fast.

**Recovery (done by the team lead at ~10:57 UTC, recorded honestly):** the lead
restarted the service again at 10:56 (to clear a separate da2ab87 CPU-spin — the
coverage query behind `/` pins a core for hours; issue 20 reopened) and
re-enqueued the standard five jobs (all 202, ids 1–5: process ted monthly → ted
daily → doe monthly → doe daily → project rebuild=false). Confirmed on the box:
health ok / rev da2ab87; job 1 running, fast-forwarding the ~217 already-ingested
packages (pkg 26/401 = 1995-03, notices counter 0 = dedup re-walk) before real
parsing resumes ~2011-02. The daily scheduler lost its queued tick (jobs 6–10)
in the restart too; it re-adds at the next 09:35 Berlin tick — no action needed.

**New monitoring constraint:** do NOT curl `/` or `/api/dashboard` on da2ab87 —
each hit starts the pathological coverage query and burns a core for hours.
Monitoring is now `/health` + authed `/admin/jobs` only (both fast: /admin/jobs
measured 0.7–4ms under ingestion load, see issue 20). Re-armed the robust watcher
on the new run (job 1 of 5, started ~10:57 UTC).
