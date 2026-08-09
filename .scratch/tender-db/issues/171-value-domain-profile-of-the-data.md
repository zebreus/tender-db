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

2026-08-09 (orchestrator): first-pass study DONE —
docs/research/data-profile-2026-08.md §2. Shape catalogue with verified
specimens: -100-cents publisher sentinel (all 99 negatives in the 2024
window are exactly -1.00 — answers issue 132 Q2 for that stratum);
zero-as-no-value 2.7-8.9% of EUR amounts per era (previously unnamed);
placeholder instants (year-0000, 1899-12-31, 2100) at ~55/100k; deadlines
before publication a stable 0.2-0.3% source background across 20 years;
currency inventory 26 incl. OP_DATPRO codelist leak, retired currencies,
USN funds code; CPV essentially clean but CPV-2003 divisions coexist with
2008 (feeds 172); NUTS carries pseudo-codes. Validation-rule catalog seed
§3 rules 8-17. Corpus totals need the snapshot machine (§4 list).
