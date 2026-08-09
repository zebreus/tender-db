# 172 — classification vocabularies and currency drift across 30 years

Status: open — research gap #7 (docs/research/research-gaps-2026-08.md), conditional: before promoting cross-era analytics
Role: run-driver

NUTS revisions change region-code meanings mid-corpus; text-era notices
predate CPV-2008; pre-1999 amounts are national currencies; C10 (EUR-at-date
derived column + rate source) was never researched OR decided. A user summing
a NUTS region or CPV prefix across 2011-2026 gets silently wrong answers at
every vocabulary boundary. Study: enumerate codelist versions actually
present per era (from our own DB), map deltas, decide C10. Interim honest
fix: a caveats paragraph wherever analytics is marketed.
