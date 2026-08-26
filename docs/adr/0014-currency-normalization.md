# ADR-0014 — Currency normalization: EUR-at-publication-date, layered beside published values

Status: ACCEPTED 2026-08-26 (owner, on Lennart's direction: "make the ADR-sized
decisions, with maintainability, scalability and flexibility in mind"). Decides
C10 — the decision research-gaps-2026-08.md records as having "silently fallen
off the register" — and the currency half of issue 291. One validation step
remains open by design (see D2 verification).

## Context

Every canonical amount stores raw integer cents + the published currency code +
tax_basis, verbatim (2026-07-19 representation decision; issue 251). Nothing
converts: `min_value`/`max_value` compare raw cents across currencies, list
headline values are raw-cents MAX over mixed currencies, and /docs#caveats
honestly says "No currency normalisation is applied". The corpus carries ≥26
currency codes including pre-euro nationals live in their eras (DEM, ESP verified
in real rows), retired codes in wrong eras (MTL in 2014), the USN funds code, and
one codelist leak (OP_DATPRO). Cross-era, cross-country analytics — the product's
stated purpose — need comparability.

## Decisions

### D1 — Published values are never touched; the normalized value is a derived sibling

`eur_cents` (nullable INTEGER) is added BESIDE the published `cents, currency`
in all four money loci (`tender_version_amounts`, `lot_results`, `bids`,
`contracts`), plus `eur_rate_date` provenance where row width allows. The
/docs#caveats "served as published" contract is load-bearing and stays: the
published value is the fact, the EUR value is our derivation, and the API serves
both (`{cents, currency, eur_cents?}` — additive, no consumer breaks).

*Maintainability*: derivations can be recomputed (rate corrections, better
sources) by refold without touching facts. *Flexibility*: a second target
currency later is another derived column or a read-time join over the same rates
table — the model does not privilege EUR structurally, only operationally.

### D2 — Rate source: ECB euro foreign-exchange reference rates + the fixed irrevocable euro conversion rates

- Floating era: the ECB daily reference rates (published since 1999-01-04, the
  standard citable source, free, stable URLs, full history).
- Pre-euro nationals (DEM, ESP, FRF, …): the fixed irrevocable conversion rates
  set by EU Council regulation — exact by definition, no time series needed.
- Pre-1999 amounts in currencies with no ECB series (and ECU-era rows):
  convert only where a defensible official series exists; otherwise
  `eur_cents = NULL` (D4). No third-party commercial rate feeds — citability and
  reproducibility beat coverage.

The rates land in a `currency_rates(currency, date, rate_to_eur, source)`
reference table — the repo's FIRST non-notice ingestion source, deliberately
minimal: a fetch job with the same hash-idempotent, raw-kept discipline as
notice packages (ADR-0004 spirit), a few MB total.

**Verification step (open)**: before the backfill refold, the 172 research pass
samples the pre-1999/accession-era rows against the chosen series and fixes the
per-currency cutover dates. The ADR decides the source; the research validates
the edges.

### D3 — Date policy: publication date, uniformly

The conversion date for every amount is its version's publication date — the one
instant every amount row has by construction. The buyer's decision date (issue
255) exists only on award blocks; using it selectively would make `eur_cents`
mean different things on different rows. A future award-subset refinement
(`eur_cents_at_decision`) is possible as ANOTHER derived column if analytics
ever need it — flexibility through addition, not through overloading.

*Scalability*: publication-date lookup is one indexed join per amount at fold
time; rates are ~7k days × ~40 currencies — trivially cached in memory for the
refold.

### D4 — Unconvertible rows are NULL with a reason, and NULL is a first-class analytic answer

No rate (OP_DATPRO, USN, retired-code-in-wrong-era, gaps): `eur_cents = NULL`.
Aggregations over `eur_cents` must surface their coverage (the DQ machinery
gains a `eur_convertible_rate` per era, riding the 265/266 gauge pattern) so
"the sum of converted values" is never mistaken for "the sum of values". The
data-quality tripwires (>1e12 etc.) STAY on published cents — they guard the
extraction, not the conversion.

### D5 — Read surface: currency filter + normalized comparisons, honestly labeled

- `?currency=` filters by PUBLISHED currency (exact match — cheap, honest).
- `min_value`/`max_value` switch to comparing `eur_cents` (documented: rows with
  NULL eur_cents do not match a value bound — the same one-sided honesty as
  every filter guard), with a materialized index so the filter leaves the
  isolation-pool cost class.
- The raw-cents comparison behavior is retired from the public surface, with a
  CHANGELOG entry (it was documented as a caveat, not a contract).

## Consequences

- Build order: rates table + fetch job → `eur_cents` migration (4 loci) →
  projection derivation → ONE whole-corpus refold (~3h precedent; rides 278's
  ghost cleanup for free) → filters + gauges + docs.
- The 172 research pass becomes scoped: validate cutovers and the pre-1999
  edges; its classification-drift half splits into its own item.
- New-portal onboarding gains one line: "your currencies must resolve in
  `currency_rates` or your values are NULL-normalized — check coverage."
- Storage cost: negligible (one INTEGER + one date per money row; rates ~MB).

## Amendment 2026-08-26 — full-history rates + arbitrary-target conversion (Lennart)

On Lennart's question ("real multi/original currency support — tracking all the
currencies, historical daily rates since the first tender — too much?"): not too
much, and mostly a sharpening of D2/D5. Three additions:

### D2a — The rate series covers the WHOLE corpus via the ECU chain

Pre-1999 (corpus start 1993 → 1998-12-31): the official daily **ECU** rates (the
Commission's Official Journal series, carried by Eurostat) — ECU→EUR converted
1:1 by law on 1999-01-01, so `rate_to_eur` is a single continuous, citable pivot
series from the first tender to today. Cross-check source: the Bundesbank daily
fixings (via the fixed 1.95583 DEM/EUR conversion). The 172 validation pass
verifies the fetched ECU series against a sample of real pre-1999 canonical rows
before the backfill refold (this was already the ADR's open gate; it now
explicitly includes the ECU dataset).

Size, for the record: ~280k rows (ECB era) + ~25k (ECU era) ≈ a few MB — the
cheapest reference table in the DB. Daily granularity is kept (the sources are
daily; monthly would just add an averaging policy to argue about). Missing days
(weekends/holidays) resolve to the nearest PREVIOUS business day.

### D6 — Arbitrary target currencies via pivot arithmetic, read-time only

`rate(A→B, date) = rate(A→EUR, date) / rate(B→EUR, date)`. No cross-currency
matrix is ever materialized; `eur_cents` remains the ONLY stored derived value
(it is the filter/index/comparison column). Any other target (`?convert=USD`)
is computed at read time from the pivot — two lookups per row, cacheable, and
additive to the API (`{cents, currency, eur_cents?, converted?}`). Value-bound
filters in a non-EUR currency convert the BOUND once per query at the latest
rate while rows carry EUR-at-publication — documented explicitly in /docs
(the user thinks in today's units; the corpus is valued at publication).

### Noted out of scope, by design slot-in-able

Inflation adjustment (CPI-deflated purchasing-power columns) is the real
cross-era comparability elephant and is explicitly NOT built here — but the
derived-beside pattern means a `eur_cents_real` column with its own reference
series slots in later with zero model change. That, not any speculative schema,
is this ADR's flexibility guarantee.
