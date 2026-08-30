# 319 — The organization country column holds alpha-3 codes and free text

Status: ready-for-agent (measured 2026-08-30 on prod)
Kind: data quality / API correctness (organization layer)
Relates to: 300 (R2 keys on country), 314 (the edge census surfaced it), 230

Found by reading the issue-314 cohort sample, which paired
`GL:Nukissiorfiit` with `GRL:Nukissiorfiit` — the same Greenlandic utility
under two country codes, one of them not alpha-2.

## Measured (prod, /v1/sql, index-served GROUP BY)

- **396 distinct `organizations.country` values** over 4,241,997
  country-bearing rows.
- **244 are two characters** (4,240,649 rows) — the intended alpha-2.
- **151 are three characters** (1,347 rows) — ISO alpha-3 leaking into an
  alpha-2 column: MCO 61, ARE 54, TUN 40, ZAF 39, SGP 38, MAR 36, EGY 35,
  GEO 35, KEN 33, SEN 33, GRL 12, …
- **One is a country NAME**: `'LUXEMBOURG'`, 1 row.

**A trap worth naming, because I walked into it:** "the first two letters of
the alpha-3 are also a live code" is NOT evidence of a split country. SEN
(Senegal) truncates to SE (Sweden, 75,920 rows) and BEN (Benin) to BE
(Belgium, 91,596). Any fold must go through a real alpha-3 → alpha-2 table,
never a prefix.

## Why it matters, in order of severity

1. **The API lies to a filter.** `country` is a public field and a filter on
   `/v1/organizations`. A consumer asking for `GL` does not get the 12 GRL
   rows, and nothing tells them they are missing.
2. **R2 cannot merge across the split.** The merge arm keys on
   `(country, kind, value)`, so GL and GRL rows for one entity never meet —
   a permanent duplicate class the matcher is structurally unable to close.
3. **It contaminates the issue-314 cohort.** The 962 canonical-only
   cross-border components include pairs like GL–GRL that are not
   cross-border at all. Sizing a review campaign on that number without
   this fix means paying an agent to rediscover a normalization bug.

## A SECOND, different class: valid-shaped but wrong

`VU` (Vanuatu) holds 110 org rows and `GY` (Guyana) 33. Sampling VU returns
Bulgarian entities with Bulgarian EIK numbers (`„Метрополитен“ ЕАД`,
`Българска национална телевизия`), British ones (Oxfordshire County
Council, Lloyds Pharmacy, RSM McClure Watters) and a German one (Giesecke &
Devrient). These are well-formed alpha-2 codes carrying the wrong country,
so a shape check will never catch them — the value has to be wrong against
the entity, which is a per-case judgement (the issue-311 direction), or
traced to whatever parse produced it.

Do not fold this class into the alpha-3 fix. Different cause, different
evidence, different remedy.

## Shape of the work

1. **Normalize at ingest**: an alpha-3 → alpha-2 fold plus a name → code
   fold, in `crosswalk` beside the existing EL→GR handling, with the table
   as data rather than match arms.
2. **Backfill**: 1,348 rows is a bounded update job with a dry plan, and it
   must run BEFORE any campaign consumes the 314 cohort.
3. **Then re-run the census**: the cohort number will drop, and the drop is
   the measurement of how much of the "cross-border" signal was this bug.
4. **Trace the VU/GY class separately** — start from the notices those org
   rows were minted from and find the field the parser read.

## Cheap tripwire, worth landing with the fix

A weekly count of `organizations` rows whose country is not in the alpha-2
allow-list. It is one indexed GROUP BY (measured: it completes inside the
/v1/sql 10 s cap; the LENGTH() form does NOT — it cannot use the index and
times out), and a non-zero result means a new source started publishing a
shape nothing folds.
