# 171 — value-domain profile of the data itself

Status: two of three deliverables DONE (study 2026-08-09; public caveats doc 2026-08-23 — /docs
#caveats); remaining: corpus-total queries (snapshot-gated, §4 list) and wiring §3's rule seeds
into ingestion validation where they are not already tripwires
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

### Public "known data caveats" doc SHIPPED (2026-08-23, owner)

Deliverable (b) is now a `Data caveats` section on the served API reference (`/docs#caveats`,
crates/app/src/v1/docs.rs) — written from this issue's measured §2 facts plus what landed since:
era-varying coverage and the backfill-can-add-content note, publisher-silent winners (257's
inverted diagnosis, sdk-0.1's ~87 %), the sub-cent rounding rule (268/ADR-0010 amendment),
source-published negatives and the −1.00 sentinel, zero-as-no-value with its 2.7–8.9 % band,
`tax_basis` NULL semantics and era bias (251), the 26-currency inventory with no normalisation,
placeholder instants (~55/100k) and the 0.2–0.3 % deadline-before-publication background,
CPV-2003/2008 coexistence (feeds 172), NUTS pseudo-codes, provisional organizations (234), and
the strict-quarantine disclosure. Deploys with the next batch.

Remaining for this issue: the §4 corpus-total list (snapshot machine, team-lead-gated reads) and
promoting §3 rules 8–17 into ingestion-side validation where 267/230 don't already cover them.
