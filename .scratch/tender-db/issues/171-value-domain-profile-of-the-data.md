# 171 — value-domain profile of the data itself

Status: two of three deliverables DONE (study 2026-08-09; public caveats doc 2026-08-23 — /docs
#caveats); the third is mapped rule by rule (2026-09-26, foot) and rule 12's deadline half is
BUILT; remaining: rules 11 (currency validity) and 17 (NUTS pseudo-codes) at ingestion, and the
corpus-total queries (snapshot-gated, §4 list)
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

### 2026-09-26 — §3 rules 8–17 mapped against what is enforced; rule 12's deadline half built

| rule | what enforces it now |
|---|---|
| 8 −1.00 sentinel | DONE — `sentinel_amount` (366) refuses it from every head column; `quality='withheld'` (372) marks declared withholdings |
| 9 amounts ≥ 0 + ceiling | DONE — negatives, zero, one-unit and digit-run shapes in `sentinel_amount`; `IMPLAUSIBLE_EUR_CENTS` €100 bn; DQ §8 reports rates |
| 10 zero rate per field × era | REPORTED — DQ §8 and §10 (bottom-of-range listing); `/docs#caveats` states the 2.7–8.9 % band |
| 11 currency ISO 4217 | PARTIAL — the API refuses a non-ISO filter value; `OP_DATPRO`/retired codes convert to no `eur_cents` (ADR-0014), so no head value; ingestion stores them unmarked — **open** |
| 12 placeholder instants | deadline half **BUILT** (below); the other date fields stay published values, measured by DQ §10's date listing |
| 13 deadline ≥ publication rate | REPORTED as a caveat (the 0.2–0.3 % within-notice background, 37.6 % at row level — issue 370) |
| 14 plausibility window | DONE for the head deadline (366's ten-year horizon); DQ §10 lists the far-future clusters (all `duration_end`/`participation_deadline`, none in `submission_deadline`'s top 40) |
| 15 dispatched ≤ published | not measured since the profile; 418's instant repair made both comparable — **open, low** |
| 16 CPV `^\d{8}$` | DONE at the fold boundary — `normalize_cpv` (394); a shape it cannot read passes through visibly |
| 17 NUTS pattern / pseudo-codes | stored as published; `/docs#caveats` names pseudo-codes — **open, low**: measured 2026-09-26 by walking the `(scheme, code)` index ends, the whole below-`A` band is `00` ×31,411 rows and `1A` ×451, and nothing sorts above `ZZZZZ`. They reach only `?country=00` and the echoed `places`; no aggregate the API serves groups by region |

**Rule 12, deadline half — measured then built.** Prod on 2026-09-26 (bounded index-range read of
`tenders_current_deadline`): **5** tenders held a head deadline before 1990 — 8210860 year 0007
(2008 notice), 5671586 year 0016 (a two-digit year), 1542367 year 0025, 5653434 year 0206 (for
2016), 1466977 `1970-01-01` (2024 notice) — and all five sorted FIRST on
`sort=deadline&order=asc`. Every one was its tender's only deadline, so MAX had nothing better to
take. Their lots served the same dates (lot 6556410 `0016-06-09`, 3509049 `1970-01-01`). The band
just above 1990 is genuine (1991–92 deadlines restated by 1993 award notices).

The floor is **absolute** (`DEADLINE_FLOOR_SECS` = 1990-01-01), not "ten years before
publication" like the horizon, and deliberately: a head version is a union of its notices, so a
genuine 2008 deadline can sit on a head a 2023 modification notice published, and a relative rule
would delete it. Applied at every transcription of the election — `head_deadline`, the read pick
(`tender_select_head`), `backfill_current_deadline` — plus the lot row (which does not touch
`status`: a pre-1990 date is never `> now`). The weekly DQ sentinel-date floor now reads the same
constant. Tests: `head_election_agreement.rs` (fold and row agree; typo beside a real date; typo
alone; inclusive boundary; SQL interpolates the constant), `deadline_backfill.rs` (the backfill
refuses it; the suite's toy instants rebased onto a real 2005 base because 5000 s after the epoch
is now junk by definition), `lot_deadline_scope.rs` (a refused own deadline inherits the
procedure's; an epoch-zero-only procedure opens and serves nothing). `/docs#caveats` Dates says it.

**Standing rows**: the served VALUE fixes on deploy (the read pick recomputes); the five
`current_deadline` columns (sort order) fix on their next refold or a `backfill-deadline` run —
a production write, run when the environment permits admin jobs.

**Filed**: 422 — the lots path applies the floor but not the horizon, and its `status` EXISTS
neither, so a lot can be open on a year-3005 typo its tender refuses.

## Verify

    curl -s 'https://tenders.zebreus.click/v1/lots?tender=5671586&limit=1' | grep -o '"submission_deadline":[^,]*'; curl -s https://tenders.zebreus.click/v1/tenders/5671586 | grep -o '"submission_deadline":[^,]*'

- **done** (for the rule-12 unit): every line prints `"submission_deadline":null` — the tender's
  only deadline is the year-0016 typo (procedure-scoped, which lot 6556410 inherits)
- **open**: every line prints `"submission_deadline":"0016-06-09T15:00:00+00:00"` (five on 2026-09-26,
  pre-deploy)

A public read, free.
