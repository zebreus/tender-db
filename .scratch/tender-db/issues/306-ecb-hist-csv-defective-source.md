# 306 — INCIDENT: ECB's bare eurofxref-hist.csv is a defective artifact; rates stop at 2010 + one garbage row loaded

Status: OPEN — fix in progress (found 2026-08-27 20:5x reading the post-refold
acceptance DQ: modern eras' eur-conv rates were impossibly low — sdk-1.8 17.4%,
r208 32.8%, r209 39.2% — while text-era converted at 96.9%)
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

## Fix (one unit)

- `fetch-rates` switches to the ZIP (reuse the ingest zip machinery), keeps
  the registry archive of the zip + extracted CSV.
- **Staleness tripwire**: the job hard-errors unless the newest parsed date is
  within 10 days of the fetch date — this exact failure becomes a red job at
  the first fetch instead of silent NULLs discovered via a gauge.
- New `rederive-eur` job: windowed walk over the four money loci recomputing
  `eur_cents` for EVERY row from (cents, currency, version published_at) via
  the in-memory lookup — idempotent, fixes class 2 and fills class 1, quiet on
  the change feed (derived-beside layer; the corpus content did not change).
  Then re-run `backfill-values`; gauges correct on the next DQ run.

## Lesson

A reference source can be wrong in-band (200 OK, valid CSV, plausible bytes).
Every acquisition job needs a freshness/shape assertion tied to what the data
CLAIMS, not just parse success — the eurostat-ecu loader gets the equivalent
guard (its series is closed: assert coverage ends 1998-12-31).
