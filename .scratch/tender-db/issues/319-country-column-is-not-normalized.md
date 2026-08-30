# 319 — The organization country column holds alpha-3 codes and free text

Status: org-layer fix BUILT and gated (ebbfaff), awaiting panel + deploy;
mention layer measured and NOT yet fixed; the VU/GY class untouched
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


## What ebbfaff actually fixed, and what it did not

**Diagnosis corrected first.** The fold was not missing — `canonical_country`
has existed since issue 48. Its alpha-3 table was HAND-PICKED (51 entries:
the EU-27, near-Europe, and "common third countries seen in TED supplier
data"), so the other 151 alpha-3 codes fell through the `None => up` arm
that preserves unknown values. The bug was a partial allow-list wearing the
shape of a complete one.

The table is now generated from `/usr/share/iso-codes/json/iso_3166-1.json`
(249 assignments) plus a 425-entry name table. Checked before replacing:
the old table disagreed with ISO on NOTHING and carried exactly one entry
ISO does not have — `XKX → XK`, Kosovo, user-assigned — which stays a hand
arm beside `UK → GB` and `EL → GR`.

`fold-org-countries` backfills the standing rows: one index-served GROUP BY,
then an indexed range update per value. It rewrites a LABEL and never
identity, so rows that land on an identity that already stands are counted
and left to R2.

## The mention layer carries the same values (measured, NOT fixed)

`organization_mentions.country` holds the unfolded strings too, and for one
class it is far bigger than the org layer:

    LUXEMBOURG  151 mentions   (vs 1 org row)
    MCO          67
    SEN          38
    GRL          14

That is 270 mention rows for four codes alone. Two reasons it is a separate
unit rather than a wider `WHERE` clause here:

1. **Reachability.** Mentions of an AFFECTED org are indexed
   (`organization_mentions_org`), so folding those is cheap. A mention whose
   own country is stale while its org's is fine is NOT reachable that way,
   and finding it needs a scan of the whole mention table.
2. **Blast radius.** Mentions are the immutable evidence layer; the R2/R3
   walls key their evidence by mention country. Rewriting them deserves its
   own dry plan and its own panel round, not a rider on a job that was
   reviewed for the org table.

## Still untouched: the VU/GY class — and it is UPSTREAM, not ours

143 rows whose country code is well-formed but wrong for the entity
(Bulgarian and British organizations under Vanuatu and Guyana).

**Traced 2026-08-30, and the trace exonerates the parser.** Joining the
mentions to the notices they came from, `notice_codes.TED-COUNTRY` for those
exact sections reads `VU` (232 mentions) and `GY` (74) — the SOURCE
publishes the wrong country, and the pipeline stores it faithfully. On one
notice (18856147) sections ORG-1..ORG-3 carry `BG` and ORG-4 carries `VU`,
for `Комисия за защита на конкуренцията`, the Bulgarian competition
authority. The same body appears under VU across several notices, which
looks like a form default somebody set once and never corrected.

(Recorded because I got this wrong first: reading a section id off by one, I
briefly had the mention disagreeing with its own notice, which would have
meant a stale mention layer. The join says otherwise.)

So this class cannot be fixed by normalization, and correcting it means
OVERRIDING published data on evidence the publisher did not give us — the
entity's name language, its identifier's shape, its other notices. That is
the issue-311 per-case direction, not a rule. Nothing here is urgent: 143
rows, each mislabelled exactly as its source mislabelled it.
