# ADR-0015 — The /v1/sql surface promise (C15): what analysts may build on

Status: ACCEPTED 2026-08-27 (owner, closing issue 299 — C15 "appears in no
decision list, no ADR, no design doc" per research-gaps-2026-08.md; decided
under Lennart's standing direction to make the ADR-sized decisions with
maintainability, scalability and flexibility in mind).

## Context

The public SQL endpoint serves turso's SQLite dialect over a 46-name
allow-list, with per-table notes, a 10s/10k-row/10MB envelope, and one
`SELECT` per request (issues 43/50/210). What was never decided: which of
that is CONTRACT and which is implementation. One turso upgrade — 0.8.0 is
already on the recheck list — could silently break every saved analyst query,
and nothing today would even tell us it happened.

## Decision

**D1 — The contract is names and shapes, not the engine.** The promised
surface is: (a) the allow-listed table and view NAMES; (b) their existing
columns' names and meanings; (c) the request/response envelope (one bare
SELECT, the JSON `{columns, rows, row_count, truncated}` shape, epoch-second
time columns); (d) the documented result and rate caps. Additive change —
new columns, new tables/views, new notes — may happen at any time without
notice. Renaming or removing an allow-listed name or an existing column is a
BREAKING change: it requires a CHANGELOG.md entry and, where feasible, a
deprecation period with both names live.

**D2 — The dialect is described, not promised.** Queries get "SQLite as the
current embedded engine implements it" — today turso 0.7.x, with its
documented gaps (no `WITH RECURSIVE`, partial window functions; the /docs#sql
list is the honest description). A turso upgrade may change dialect corners
without a version bump of ours. The protection is a **dialect canary**: a
gate-time test suite of representative analyst query shapes (joins, GROUP BY,
CTE, strftime, CASE, correlated subqueries, LIKE, aggregates over views) that
runs against the exact engine we ship. An upgrade that breaks a shape turns
the gate red BEFORE deploy; the resolution — carry the break with a CHANGELOG
entry, or hold the upgrade — becomes a deliberate decision instead of an
analyst's surprise.

**D3 — The `v_*` views are the stable analyst API; base tables are the power
surface.** Views keep working across internal re-modelling (the fold may
re-shape how a view is DEFINED, never what it means); base tables expose the
real schema and inherit only D1's name/column stability. This is the existing
de-facto split, now stated.

**D4 — `currency_rates` joins the allow-list** (reference data, not notice
data): the EUR-pivot daily series (ECB reference rates 1999→, the
Commission's daily ECU series 1993–1998 via Eurostat, and the irrevocable
euro conversion rates), keyed `(currency, rate_date)` with `rate_to_eur` =
units per EUR and a `source` tag. It is public, freely reusable data (ECB
terms; Eurostat CC BY 4.0 — "Source: Eurostat") and it is exactly what an
analyst needs to convert published non-EUR amounts their own way instead of
trusting our derived `eur_cents` blindly — the flexibility half of ADR-0014's
honesty stance. The lookup POLICY (nearest at-or-before date, 7-day daily
window, unbounded irrevocable, legacy-code aliases) lives in the table note;
the table itself is just the series.

## Consequences

- CHANGELOG.md (created with the ADR-0014 D5 flip) is the surface ledger:
  breaking /v1/sql changes land there or they do not land.
- The dialect canary lives in `crates/app/tests/sql.rs` and runs in every
  `ops/check.sh`; the turso-0.8.0 recheck now has a concrete acceptance test.
- /docs#sql states the promise in one paragraph so analysts know what to
  build on without reading this ADR.
- Issue 299 closes; the currency_rates exposure question from issue 291
  closes with D4.
