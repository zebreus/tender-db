# 171 — value-domain profile of the data itself

Status: open — research gap #6 (docs/research/research-gaps-2026-08.md), before launch (cheap)
Role: run-driver

The research profiled structure and identifiers exhaustively, content values
not at all; the data-quality tool measures presence, not validity. Every
value pathology so far (issues 48, 18, 131-136, 144, 160) was found by
accident and cost a forensic loop. Study: one systematic bounded-SQL
profiling pass per era over amounts (negatives, sub-cent, magnitudes),
currencies (incl. pre-euro), dates (impossible deadlines, tz artifacts) and
coded values, yielding (a) a validation-rule catalog for ingestion, (b) a
public "known data caveats" doc. ~2 days; the 10s/single-SELECT surface
already proved sufficient for exactly this shape (issues 136/137).
