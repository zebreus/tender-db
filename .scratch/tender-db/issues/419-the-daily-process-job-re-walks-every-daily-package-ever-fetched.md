# 419 — the daily process job re-walks every daily package ever fetched, and nothing retires one

Status: ready-for-agent — filed 2026-09-19 09:5x UTC by the hourly audit (step 3) from the 07:35 tick's `[process]` lines (issue 407's line made it visible: 71 DÖE/FTS re-walks around two writing walks). Measured, mechanism read, fix cut below; the build follows in the same session.
Kind: cost (operability — a linearly growing, unbounded walk inside the morning window, every day)
Relates to: 407 (the line that showed it, and the `package_rates` ledger the fix extends), 222/403 (the morning window this walk sits in), 32 (per-package resume — a different question: a walk interrupted mid-job), 77/79 (reprocess efficiency — the quarantine reclaim, not the daily walk), 404 (the accidental re-key a daily re-walk performed mid-campaign; under the fix that path closes and `reparse` is the deliberate one), 342 (the FTS comment that states the property the fix must keep).
Blocked by: nothing

## Verify

    ssh -o BatchMode=yes root@zebreus.click "/root/aj.sh '/admin/jobs?limit=40'" | python3 -c "import sys,json; r=[j for j in json.load(sys.stdin)['recent'] if j['kind']=='process' and j['params']=='ted daily (all)']; j=r[0]; print(j['finished_at']-j['started_at'], 's;', j['counts'][:70])"

- **done**: the newest weekday TED process walks only packages that can still yield something — a few seconds and `N members → N notices (… 0 dup)` plus a `skipped M clean package(s)` clause, where M is nearly every held daily
- **open**: `127 s; 197146 members → 3474 notices (3474 parsed, 0 quarantined, 0 unrecognised, 193672 dup)` — 58 daily packages re-walked to write one (read 2026-09-19, job 1480 of 09-18)

## Observed (2026-09-19, the 07:35 tick and the one before)

| job | walk | wall | members → notices (dup) | packages walked |
|---|---|---:|---|---:|
| 1480 (09-18) | `ted daily (all)` | 127 s | 197,146 → 3,474 (193,672 dup) | 58 (2026-00124..00181) |
| 1491 (09-19) | `doe daily (all)` | 15 s | 48,161 → 1,046 (47,115 dup) | 64 (07-17..09-18) |
| 1493 (09-19) | `fts daily (all)` | 2 s | 4,755 → 482 (4,273 dup) | 12 |

Every line but the newest package's reads `… → 0 notices (N dup) in ~2s (~1,850 members/s)` for TED
and `(~4,500 members/s)` for DÖE: the whole archive of dailies is re-read and every member's identity
re-checked, to find the one package that is new. 98.2 % of TED's morning walk is that.

## The mechanism

`enqueue_daily` pushes `Spec::Process { period: None }` for each source (`supervisor.rs`, the
`… daily (all)` jobs), and `Db::current_packages(source, kind, None)` returns the newest fetch row of
EVERY period ever fetched for that (source, kind). Nothing retires a daily: the archive holds 58 TED,
61 DÖE and 12 FTS daily packages, the monthly ranges stop before the dailies begin (TED monthly to
2026-06, dailies from 00124 = July), and no roll-up exists in the code. So the walk grows by one
package per publishing day, forever.

**The re-walk is a property, not an accident.** The FTS comment in `enqueue_daily` says why:
`current_packages(…, None)` "re-walks EVERY registered FTS daily on the first tick after this ships,
so the days the probe landed dark are processed then" — a member the dispatcher declined by policy
(`Disposition::Skipped`) yesterday is retried today, and a new dispatch arm claims it with no
operator step. That self-healing must survive the fix. What a re-walk can change, from
`process.rs`: a **skipped** member (retried), a **quarantined** member (re-inserted; reclaim is
`reprocess`'s job), and nothing else — a duplicate identity is counted and dropped, and a held notice
is never re-parsed by the walk (that is `reparse`).

## Why it matters

- **Growth.** TED adds a package per weekday at ~2 s of dedup each: +2 s/day per day. In a year the
  weekday walk is ~8–9 minutes; in two, ~17 — inside the 07:35–10:35 finality window that 222's
  catch-up and 403's gate protect, every morning, and every deploy waits behind it.
- **I/O.** 58 daily tarballs re-read each morning (~1.7 GB today, growing) for no row.
- **The 404 shape.** The re-walk is also the path that re-keyed 281 notices mid-campaign when a
  derivation changed — a daily job performing a corpus-wide identity change nobody enqueued. Closing
  the re-walk closes that path; `reparse`/`reprocess` remain the deliberate ones.

## Fix — the clean-walk watermark on the `package_rates` ledger (407)

A walk is **clean** when it recorded or de-duplicated every member: `skipped = 0` and `quarantined =
0`. A clean walk of a package at fetch id F cannot yield anything on a later walk at the same F: the
members are all held, nothing was declined, nothing is waiting on reclaim. So:

1. `package_rates` gains `fetch_id INTEGER`, `skipped INTEGER`, `quarantined INTEGER` (jobs::SCHEMA
   for fresh databases + `ALTER TABLE … ADD COLUMN` in MIGRATIONS for the prod table created on
   09-19; the migration runner tolerates "duplicate column"). Yesterday's rows read NULL and are
   never treated as clean.
2. `Db::clean_walks(source, kind) → {period: fetch_id}`: for each period, the newest recorded walk,
   if clean.
3. `run_process` with `period: None` (the tick's `(all)` walks and the monthly path) drops every
   package whose current fetch id equals its clean-walk fetch id, before the loop; the summary says
   `…; skipped N clean package(s) at their current fetch` so the shrink is visible. An explicit
   `period` walks regardless (the operator's single-package lever). A re-fetched package (TED's
   finality-window `refetch: true` lands a new fetch row → new id) is walked again, as today.
4. A package with skipped members keeps being walked every tick until an arm claims them — the FTS
   property — and one with quarantined members keeps being walked until `reprocess` empties it (or
   the walk could ignore quarantine for cleanliness once 77/79's reclaim is the only path; not
   decided here, the conservative rule is chosen).

Expected after deploy (the Verify block): TED's weekday walk drops from 127 s to the one new
package's ~25 s; DÖE from 15 s to ~4 s; the summary names the skipped count. Nothing about what is
held changes — a clean package yields nothing today either; the fix stops paying to prove it.

## Done when

- The three tick walks skip their clean packages and say how many; the newest package still walks.
- A package with skipped members is walked again (test: a declined member keeps its package unclean);
  a package with quarantined members is walked again; a re-fetched package is walked again; a package
  whose only walks predate the columns (NULL) is walked again.
- `docs/operations.md`'s process paragraph (407's) says what "clean" means and that `period=` forces
  a walk.
- The Verify block flips on the first weekday tick after the deploy.
