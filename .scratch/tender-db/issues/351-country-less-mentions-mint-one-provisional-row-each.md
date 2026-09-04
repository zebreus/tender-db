# 351 — country-less mentions mint one provisional row each: 92 identical `Stadt Burghausen` rows, and 68% of prominent entities' mentions on echo rows

Status: UNITS 2–4 DEPLOYED 2026-09-04 08:20 (`b41008f`); the top-200 cohort recorded (200 verdicts, `top200-2026-09-04`); the fold's dry run is job 665 (walks the 8M-row class, ~40 min). Then: review the plan (`scratchpad/pef-review.py`, 30-sample), the wet run (may hit the classifier), and the next ingest's diag line for the resolver's reuse counters. Status: UNIT 4 BUILT 2026-09-04 08:0x (gate running) — `org_name_verdicts` + `POST /admin/name-verdicts`; the three-tier gate (`EchoTier`: verdict → echo-of-one → raw wall) now decides the fold, the resolver's country-less reuse and the census listing; 30 store tests green across the five affected files. Then: deploy units 2–4 when the box is idle, POST the top-200 cohort, dry fold, 30-sample, wet. Status: CENSUS DONE 2026-09-04 (job 647, 2,511 s) — **8,032,637 country-less identifier-less provisional rows under 1,714,583 names; 574,089 names hold more than one row (6,892,143 rows; a fold removes 6,318,054)** — and the wall gate as built cannot reach the big ones: all 200 largest names are over the wall BY THEIR OWN ECHO. Units 2+3 (committed, not deployed) get a revised gate before deploying: verdict table → echo-of-one-entity rule → raw wall. Was: ALL THREE UNITS BUILT 2026-09-04 07:4x — unit 1 (census) deployed `b7d0eba`, running as job 647; unit 2 (prevention) committed `08919c9`; unit 3 (`fold-provisional-echoes`: wall-gated fold to one row per name, ledger rule `p0`, dry/wet with parity, residual re-record; test `provisional_echo_fold.rs`) gate running. Deploy of 2+3 waits for job 647; then the dry plan, a 30-sample, and the wet run (which may hit the same classifier denial as 329's). Was: UNITS 1+2 BUILT 2026-09-04 07:4x — unit 1 (census) deployed `b7d0eba`, running as job 647 (~1,670 rows/s; report pending); unit 2 (prevention: country-less reuse of the standing `(name_norm, NULL)` row when the N2 key is under the wall, mint over it, counters in the resolver's diag line; test `country_less_mentions_reuse_under_the_wall_and_mint_over_it`) gate running, deploys once the queue is idle. Was: UNIT 1 BUILT 2026-09-04 07:0x (gate running) — `provisional-echo-census`: a keyset walk of the `(name_norm, id)` index over provisional NULL-country identifier-less rows, one group in memory at a time, top-200 by rows with mentions and the wall's verdict; tests `provisional_echo_census.rs`. Then: deploy, run, record; units 2 (prevention) and 3 (repair) follow. Was: ready-for-agent (filed 2026-09-04 from issue 350's measurement)
Kind: data quality / identity (organization layer) — prevention + repair, the 234 shape for the country-less half
Relates to: 234 (closed the `(name_norm, country)` half; left "nameless or country-less mentions mint fresh rows"), 350 (the measurement), 349 (why the wall reads these entities as generic), 300 Stage 3 (R3 rescues NULL-country rows WITH identifiers; these have none)

## Observed

Over 168 prominent entities (issue 350's echo keys): 7,856 of 9,003 carrier
rows are provisional and 7,384 of those are NULL-country; 44,340 mentions sit
on them against 21,064 on the identified rows. They are not spelling variants —
`Stadt Burghausen` × 92, `IBM Deutschland GmbH` × 83, `STADT MANNHEIM` × 59 —
one row per mention, because the post-234 provisional path reuses
`(name_norm, country)` only, and a country-less mention has no country to
match on.

## Units

1. **Census** (read-only, one PK walk): provisional NULL-country rows by
   `name_norm` → groups ≥ 2, rows, mentions, top 200 groups, per-name-key
   genericness (the 348 probe's breakdown) so unit 2's gate can be sized.
2. **Prevention**: `resolve_one_mention`'s provisional path looks up
   `(name_norm, NULL)` for a country-less mention and reuses the row when the
   N2 key is under the wall (`name_key_is_generic_on`, the fold's own
   connection, lenient on error as the corroboration probe is). Over the wall
   → mint as today. Test: two country-less "Stadt Burghausen" mentions share a
   row; two country-less "Gemeinde Taufkirchen" mentions do not once the key is
   over the cap.
3. **Repair**: fold standing identical-`name_norm` NULL-country provisional
   rows into the lowest id per name, repointing mentions/parties/bid-parties
   through `repoint_org_references`, ledger rule `p0`, dry/wet with parity;
   gated by the same wall so the Taufkirchen shape is left standing.

## Done when

- the census is on this issue with corpus-wide numbers;
- prevention deployed and the next census shows the class not growing;
- the repair's dry plan reviewed (30-sample), wet run, and the E0 dry plan's
  `admitted_echo` drops as the keys fall back under the wall.

## Census (job 647, 2026-09-04 07:16–07:58 UTC, 2,511 s at ~3,200 rows/s)

| | |
| --- | --- |
| provisional, NULL-country, identifier-less, named rows | **8,032,637** (of 12.6M org rows) |
| distinct names among them | 1,714,583 |
| names held by more than one row | **574,089** |
| rows in those groups | **6,892,143** — a fold to one row per name removes **6,318,054** |
| group sizes | 2: 224,989 · 3–5: 180,987 · 6–20: 119,261 · 21–100: 39,564 · **101+: 9,288** |
| the 200 largest names | 698,934 rows, 698,843 mentions — every one over the wall |

Half the organization table is this class. The largest names are old-era
buyers and review bodies published without identifiers, one row per notice:
`European Commission` × 28,219, `Tribunal administratif de Paris` × 23,400,
`Vermögens- und Hochbauverwaltung Baden-Württemberg` × 14,385, `PGF Urtica Sp.
z o.o.` × 10,167, `SNCF` × 7,530, `Landeshauptstadt Dresden` × 5,110, `Ville de
Paris` × 4,604 — and among them the shapes the 234 exclusion was written for:
`TendSign` × 16,597 (a platform), `AMMINISTRAZIONE COMUNALE` × 13,863, `Centre
hospitalier` × 6,537, `Ministry of Defence` × 5,537 (generic, many entities),
`Infructueux` × 6,027, `Sans suite`, `Lots 1)`, `Siehe VI.4.1)` (not names at all).

**The gate as built is circular for exactly the rows that matter.** Every
provisional row carries the name's N2 key, so a name in 92 rows has ≥ 92
carriers and reads "generic" — the echo blocks the fold of the echo. The
2 / 3–5 / 6–20 buckets (525,237 groups, ~1.7M removable rows) are mostly under
the wall and fold; the 21–100 and 101+ buckets (48,852 groups, ~4.6M rows) do
not, and they hold the Burghausen/Dresden/SNCF shape this issue exists for.

## Revised gate (units 2 and 3, before deploying)

Three tiers, first match wins:

1. **A recorded name verdict** (`org_name_verdicts`, unit 4): `single` folds
   and reuses whatever the wall says; `generic`, `platform`, `non-name` never
   fold and never reuse. Verdicts are recorded through `POST
   /admin/name-verdicts` — the 311 pattern: a person or an agent decides, the
   verified machinery applies. The first cohort is the top 200 above, read by
   hand this morning (`351-top200-verdicts.json`): ~170 single entities, ~20
   generic names, 1 platform, ~6 non-names, and a handful I could not decide
   (`unclear`, treated as no verdict).
2. **Echo of one identified entity**: over the raw wall, but the identified
   carriers number 1..cap and all sit in ONE country — the Burghausen shape;
   `TendSign` (0 identified) and `Ministry of Defence` (many countries) fail it.
   `Gemeinde Taufkirchen` (5 identified, all DE) passes, which conflates the
   three Taufkirchens' country-less mentions into one provisional row — the
   same policy issue 234 already applies to their DE-country mentions.
3. **The raw wall**: under the cap → fold/reuse; over → stand/mint.

Gate 2 is `GENERIC_KEY_BREAKDOWN_SQL` with a distinct-country column; the
census's listing gets the same verdict so the next report says which tier
each large name falls in.
