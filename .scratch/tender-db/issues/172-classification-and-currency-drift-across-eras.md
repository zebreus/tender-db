# 172 — classification vocabularies and currency drift across 30 years

Status: CURRENCY HALF CLOSED 2026-08-27 (C10 decided as ADR-0014; the pre-refold
validation pass ran — see below). CLASSIFICATION half (NUTS/CPV drift) still open,
conditional: before promoting cross-era analytics.
Role: run-driver

NUTS revisions change region-code meanings mid-corpus; text-era notices
predate CPV-2008; pre-1999 amounts are national currencies; C10 (EUR-at-date
derived column + rate source) was never researched OR decided. A user summing
a NUTS region or CPV prefix across 2011-2026 gets silently wrong answers at
every vocabulary boundary. Study: enumerate codelist versions actually
present per era (from our own DB), map deltas, decide C10. Interim honest
fix: a caveats paragraph wherever analytics is marketed.

## 2026-08-27 — the currency validation pass (owner; ADR-0014's refold gate)

Ran against the 2026-08-24 snapshot (bounded census + samples, 90s):

- **Census finding (the pass's payoff): 1993–1996 amounts publish TED-legacy
  currency codes, not ISO** — LIT/UKL/DKR/HFL/SKR/NKR/BFR/PTA/LFR/FMK/ESC/FFR/
  IKR/SFR/YEN/IRL/NFL/CND, plus ECU itself (~130 rows) — flipping to ISO codes
  mid-1997 with stragglers after. Without aliasing, the whole early corpus
  would have derived NULL against a series that covers every one of these.
  **Fixed**: `canonical_currency()` legacy→ISO alias map applied at lookup in
  both resolution paths; ECU/XEU resolve as EUR identity (1:1, Council Reg
  1103/97); the published code stays verbatim in the row. The one observed
  typo (`GPB`, 1 row, 1998) deliberately stays unconvertible.
- **Magnitude spot-validation: PASS.** Nine real 1993 rows across
  SEK/USD/FMK/CND/DKR/LIT/UKL/LFR converted through the fetched series to
  5.5k–15M EUR-equivalents — plausible procurement magnitudes end to end.
  DEM converges onto 1.95583 at 1998-12-31 in the source itself; ITL 1936.27
  likewise (the two irrevocable constants cross-confirm the series).
- **Cutover semantics** are pinned by tests (daily-before-adoption /
  irrevocable-after; pre-adoption dates never resolve irrevocable) and the
  pre-adoption years are carried by the daily series itself.

With this, ADR-0014's "validate before the backfill refold" gate is OPEN: the
epoch refold can carry the eur_cents backfill once the alias fix is deployed.
