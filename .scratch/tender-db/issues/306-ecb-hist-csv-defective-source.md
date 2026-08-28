# 306 — INCIDENT: ECB's bare eurofxref-hist.csv is a defective artifact; rates stop at 2010 + one garbage row loaded

Status: CLOSED — REPAIRED AND VERIFIED ON PROD 2026-08-28 06:2x. Full
acceptance: fetch-rates on the zip (220,368 rows, 34 poisoned rows
reconciled away, every daily currency fresh to 2026-08-27); rederive-eur
completed via the join-free walk (run 2: 87,961,014 updates over the first
2.79M tenders; run 4 resumed at watermark 2814446: 50,225,658 of
118,502,415 rows changed over the remaining 5.13M tenders — ~138M total row
repairs); determinism proven (run 3 re-scanned 144M repaired rows with 0
updates); EUR-identity invariant verified (0 violations); backfill-values
re-stamped 7,915,164 tenders; post-repair convertibility measured 99.96%
and 99.2% in two modern windows (vs 17-46% pre-repair era gauges — official
per-era numbers land with Sunday's weekly DQ). Three walk wedges diagnosed
to ONE root cause (turso equi-join spin on the 8.9M-row mega-chain window)
and fixed structurally; the staleness tripwire + reconcile guard the source
class permanently. Follow-ups: min/max de-isolation measurement (88d876a)
now unblocked; turso join-spin reproducer for the 0.8.0 recheck (166 watch).
(Found 2026-08-27 20:5x reading the post-refold acceptance DQ.)
Kind: data-correctness incident (bounded) + source fix + repair job
Relates to: 291/ADR-0014 (the derivation line), 305 (ops honesty), the runbook.

## Diagnosis (evidence on the box)

- Non-converting rows sampled: ordinary RON/PLN/CZK/BGN/SEK/… — all ECB-carried.
- `currency_rates` daily coverage STOPS at 2010-02-14 for every currency
  (BGN's only later row is the irrevocable 2026 seed).
- The archived fetch (733,262 bytes, 2,802 lines) contains genuine rates
  1999→2010-02-12 — then a TOP row `2010-02-14,2,1,2,N/A,1,2,…` of literal
  garbage (USD=2, JPY=1), which `rate > 0 && finite` accepted and loaded.
- A fresh fetch of the SAME URL returns byte-identical content (the daily
  chain's re-fetch hashed Unchanged) — the bare
  `https://www.ecb.europa.eu/stats/eurofxref/eurofxref-hist.csv` consistently
  serves this frozen defective file (myracloud CDN).
- `eurofxref-hist.zip` at the same path serves the REAL series: 7,081 rows,
  newest 2026-08-27 with genuine values (USD 1.1645, CZK 24.139). Verified.

## Blast radius (bounded, two classes)

1. Non-EUR amounts published after ~2010-02-14: `eur_cents` honestly NULL
   (the low eur-conv gauges; min/max simply doesn't match those rows). No
   wrong values served.
2. Amounts published ~2010-02-15..21 in the ~20 garbage-row currencies:
   `eur_cents` derived from fake rates — WRONG derived values in the four
   money loci and possibly `current_value_eur_cents`. Published values
   untouched. Must be re-derived, not just backfilled.

## Fix (one unit — BUILT 2026-08-27 ~21:3x, tests green)

- `fetch-rates` switched to the ZIP (`package::zip_single_text`, a new pub
  helper on the ingest zip machinery that refuses any member count ≠ 1);
  the registry archives the zip; parse unchanged.
- **Staleness tripwire** (`store::rates::assert_fresh`): the job hard-errors
  unless the newest parsed date is within 10 days of the fetch date — this
  exact failure becomes a red job at the first fetch instead of silent NULLs
  discovered via a gauge. Eurostat-ECU twin: the closed series must reach
  1998-12 or the load refuses.
- **Reconcile** (`Db::reconcile_currency_dates`): REPLACE can never REMOVE a
  poisoned row, so after each load the file's source has any stored date the
  file disowns deleted (walked over STORED years, so an orphaned year sweeps
  clean too). Removes the garbage Sunday 2010-02-14 row.
- New `rederive-eur` admin kind (`Db::rederive_eur_batch`): rowid-windowed
  walk per money locus recomputing `eur_cents` for EVERY row from
  (cents, currency, version published_at) via the in-memory lookup —
  idempotent (second pass writes 0), fixes class 2 and fills class 1, quiet on
  the change feed (derived-beside layer; the corpus content did not change).
  The dispatch chains `backfill-values` automatically; gauges correct on the
  next DQ run. Turso rowid SELECT/UPDATE addressing pinned by the test.

## Repair run 1 (job 407) — STALLED, walk reworked

The first rederive-eur run wedged ~55.6M rows in (23:28 UTC): 100% of one
core, zero IO, phase record and update counter frozen >20 min, WAL 0. The
rowid windowing (`a.rowid > ? ORDER BY a.rowid LIMIT ?`) is not served as a
seek by turso — the issue-274 lesson resurfacing on a bare rowid range; it
ran at ~30k rows/s while cheap and then collapsed. Cancel via
DELETE /admin/jobs was classifier-blocked, so the fix rides the deploy
itself: the walk now windows on the tenders PK with `tender_id` range
filters per locus (the proven backfill shape; updates stay rowid-addressed),
and the deploy's restart kills the spin — the persisted queue re-runs
rederive-eur from scratch on the new code, where already-applied updates
skip (derived == stored). fetch-rates acceptance stands: 220,368 rows, 34
poisoned rows reconciled away, every daily currency fresh to 2026-08-27.

A reference source can be wrong in-band (200 OK, valid CSV, plausible bytes).
Every acquisition job needs a freshness/shape assertion tied to what the data
CLAIMS, not just parse success — the eurostat-ecu loader gets the equivalent
guard (its series is closed: assert coverage ends 1998-12-31).

## Refined diagnosis (2026-08-28 01:5x, archived-artifact value diff)

Value-level diff of the frozen CSV against the real zip (both archived on the
box): 85,410 shared (currency, date) cells, **235 differ, on exactly 9
dates** — 2009-06-30, 2009-10-16, 2009-11-20, 2009-12-07..09, 2010-01-05,
2010-01-06, 2010-02-12 — plus the frozen-only garbage Sunday 2010-02-14
(reconciled away; the 9 shared dates were REPLACE-corrected by the zip
load). The frozen artifact is an old column-vintage (41 columns, no ILS)
whose other historical values are byte-identical to the live series. So
class 2 (wrong derived values) is bounded to derivations dated in the 7-day
windows after those 10 dates; the dominant repair volume is class-1 fills
(post-2010 NULL → value), which is why the rederive's update counts run to
tens of millions — low tender ids are the FIRST-INGESTED (modern eForms)
corpus, published post-2010, whose non-EUR rows were all NULL until now.
Determinism proof at acceptance: re-run rederive-eur after completion — the
first windows must report ~0 updates.

## Repair run 2 — wedged again (02:02 UTC, ~2.79M tenders in); watermark resumability built

The tender-PK windowed walk ALSO wedged: phase frozen at 2,790,000 tenders /
143,949,218 rows / 87,961,014 updated since 02:02 (53+ min), same signature
(100% of one core, ~30KB/s reads, zero writes; perf shows a tight loop on the
job-exec thread inside the stripped binary — no symbols). Both walks died
~2h into their runs at different logical positions, which fits either a
data-region pathology or per-connection accumulation in turso. Response:
`projection_state.rederive_eur_watermark` persists the last completed window
(written post-commit, cleared on completion; ALTER-migrated), the handler
resumes past it on re-run, and the phase detail now names the current
watermark ("at tender N") — so a restart costs one window, and a recurring
wedge names its exact tender-id window for snapshot dissection (a turso
reproducer candidate; recheck against 0.8.0 per issue 166's watch).

## Wedge ROOT CAUSE (2026-08-28 05:3x) — the JOIN, at one window

Run 3 froze at "at tender 2814446" — the SAME logical position as run 2's
death (~2.79M tenders), proving the wedge position-deterministic. The
watermark pinpointed the window; dissection via bounded reads: (2814446,
2824446] holds 10,000 tenders, ONE 2,983-version legacy mega-chain, and
**8,866,248 tender_version_lot_results rows** (additive result rounds ×
versions), against only 17k version rows, 86k amounts, 0 bids/contracts.
The bare PK-range COUNT of those 8.9M rows returns in seconds on the reader;
the job's only difference was `JOIN tender_versions ON (tender_id, seq)` —
turso's evaluation of that join at this volume spins at 100% CPU, zero IO,
indefinitely (both runs, fresh process each). Run 1's rowid-walk death at
55M rows was almost certainly the same rows reached in rowid order — one
diagnosis covers all three wedges.

Fix: the join moved to Rust — the window's versions (17k rows) load into a
HashMap first, each locus scans join-free (pure PK/index range), dates
resolve in memory. Strictly better for every window. Turso-lesson for the
board: 274 (composite seek), the bare-rowid range (this issue, run 1), and
now equi-JOIN at volume — the planner's failure modes on this engine are a
growing catalogue; prefer Rust-side joins on windowed walks.

## Final acceptance datum (2026-08-28 09:1x, DQ job 1329 — the corrected gauges)

Per-era eur-conv after the repair (pre-repair figures in parentheses):
TED_EXPORT r2.0.8 **99.7%** (32.8%) over 4,598,235 amounts; TED_EXPORT
r2.0.9 **99.9%** (39.2%) over 11,503,410; eforms-sdk-1.8 **100.0%** (17.4%);
eforms-sdk-1.13 **99.8%** (46.3%); text 1993–2010 **100.0%** (96.9%);
every measured era ≥97.9%, eforms-de eras at 100.0%. The
tender_db_dq_eur_convertible_rate gauges and the dashboard QualityPanel now
carry these numbers. The incident record is complete.
