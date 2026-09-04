# 351 — country-less mentions mint one provisional row each: 92 identical `Stadt Burghausen` rows, and 68% of prominent entities' mentions on echo rows

Status: UNIT 5 IN PROGRESS 2026-09-04 13:5x — post-fold census (job 669): 4,625,100 rows / 57,692 multi-row names / 2,968,199 rows in them; cohort 2 (the next 400 over-wall names by rows: 328 single / 15 generic / 18 non-name / 39 unclear; 273,509 rows admitted) recorded; dry re-plan job 670 running, wet next. Status: STOCK FOLD COMPLETE 2026-09-04 13:0x — **the whole plan folded: 516,400 groups, 3,407,962 rows removed** over three runs (jobs 666/667/668; the uncapped run 484,400 groups / 3,254,420 rows in 4,885 s, ~1,400 rows/s with foreign keys off), parity held throughout, residual plan 0; verified by bounded reads (`ville de paris` 4,604 → 1, `tribunal administratif de paris` 23,400 → 1; `tendsign`, `amministrazione comunale`, `infructueux` untouched). Remaining: the 57,669 over-wall names (~2.9M rows) as verdict cohorts — unit 5, census job 669 running. Status: SECOND WET SLICE DONE 2026-09-04 11:33 (job 667, `fbed95b`: 30,000 groups / 147,671 rows in ~3 min of folding after the 42-min plan walk — **~800 rows/s, ~300× the first slice**; parity held at 514,400; `aena…` 3,767 → 1 row, `aeroports de paris` 3,577 → 1); residual 484,400 groups / ~3.25M rows; next: the uncapped run (~1h45) once 352's deploy lands. Status: FIRST WET SLICE DONE 2026-09-04 10:37 (job 666: 2,000 groups, 5,871 rows removed, 5,871 mentions + 6,292 parties + 20,659 winners repointed, 3,785 tenders touched; ~0.37 s per row); two throughput fixes committed (`9ef1226` light repoint, `fbed95b` foreign keys off in the wet loop) and deploying; next: a 30,000-group slice to measure the new pace, then uncapped; issue 352 filed for the R2/E0/R3 loops. Status: DRY PLAN REVIEWED, FIRST WET SLICE RUNNING 2026-09-04 09:3x — job 665 planned **516,400 groups / 3,407,962 rows** (tiers: under-wall 493,882 · echo-of-one 22,361 · verdict-single 157; standing: over-wall 57,669 · verdict-refused 23); 30/30 on the listing, a 40-row tail read below; wet slice job 666 (`max_groups` 2,000) enqueued — the classifier allowed it this time. Then: verify, larger slices, and the over-wall backlog as the next verdict cohort. Status: UNITS 2–4 DEPLOYED 2026-09-04 08:20 (`b41008f`); the top-200 cohort recorded (200 verdicts, `top200-2026-09-04`); the fold's dry run is job 665 (walks the 8M-row class, ~40 min). Then: review the plan (`scratchpad/pef-review.py`, 30-sample), the wet run (may hit the classifier), and the next ingest's diag line for the resolver's reuse counters. Status: UNIT 4 BUILT 2026-09-04 08:0x (gate running) — `org_name_verdicts` + `POST /admin/name-verdicts`; the three-tier gate (`EchoTier`: verdict → echo-of-one → raw wall) now decides the fold, the resolver's country-less reuse and the census listing; 30 store tests green across the five affected files. Then: deploy units 2–4 when the box is idle, POST the top-200 cohort, dry fold, 30-sample, wet. Status: CENSUS DONE 2026-09-04 (job 647, 2,511 s) — **8,032,637 country-less identifier-less provisional rows under 1,714,583 names; 574,089 names hold more than one row (6,892,143 rows; a fold removes 6,318,054)** — and the wall gate as built cannot reach the big ones: all 200 largest names are over the wall BY THEIR OWN ECHO. Units 2+3 (committed, not deployed) get a revised gate before deploying: verdict table → echo-of-one-entity rule → raw wall. Was: ALL THREE UNITS BUILT 2026-09-04 07:4x — unit 1 (census) deployed `b7d0eba`, running as job 647; unit 2 (prevention) committed `08919c9`; unit 3 (`fold-provisional-echoes`: wall-gated fold to one row per name, ledger rule `p0`, dry/wet with parity, residual re-record; test `provisional_echo_fold.rs`) gate running. Deploy of 2+3 waits for job 647; then the dry plan, a 30-sample, and the wet run (which may hit the same classifier denial as 329's). Was: UNITS 1+2 BUILT 2026-09-04 07:4x — unit 1 (census) deployed `b7d0eba`, running as job 647 (~1,670 rows/s; report pending); unit 2 (prevention: country-less reuse of the standing `(name_norm, NULL)` row when the N2 key is under the wall, mint over it, counters in the resolver's diag line; test `country_less_mentions_reuse_under_the_wall_and_mint_over_it`) gate running, deploys once the queue is idle. Was: UNIT 1 BUILT 2026-09-04 07:0x (gate running) — `provisional-echo-census`: a keyset walk of the `(name_norm, id)` index over provisional NULL-country identifier-less rows, one group in memory at a time, top-200 by rows with mentions and the wall's verdict; tests `provisional_echo_census.rs`. Then: deploy, run, record; units 2 (prevention) and 3 (repair) follow. Was: ready-for-agent (filed 2026-09-04 from issue 350's measurement)
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

## Dry plan (job 665, 2,596 s) and the two reads

| tier | groups | what it is |
| --- | --- | --- |
| under-wall | 493,882 | small groups (2–20 rows) under the raw cap |
| echo-of-one | 22,361 | over the raw wall, 1..20 identified carriers all in one country |
| verdict-single | 157 | the top-200 cohort's single entities |
| **plan** | **516,400 groups, 3,407,962 rows removed** | |
| over-wall | 57,669 | over the wall, no other tier — the 21+ buckets without a verdict, ~2.9M rows |
| verdict-refused | 23 | generic / platform / non-name |

**Listing read (30-sample of the 200 largest plan groups): 30/30 one entity** —
DB Netz AG, Statsbygg, Magistrat der Stadt Wien, Communauté urbaine de Lille,
Tribunal Administrativo Central de Recursos Contractuales, Centre hospitalier de
Valenciennes, Kompania Węglowa, Bialmed, … (the verdict-single and largest
echo-of-one names).

**Tail read (40 random rows of the class, row-weighted):** mostly single
entities again (Bundeskartellamt, Universität Siegen, Philips Ibérica, INPI,
Flintshire County Council, Conseil général du Doubs, the tribunals); but the
tail also holds **non-names** (`1`, `S. o.`, `Siehe VI.3.1`, `Procédure déclarée
sans suite`, `Vous pouvez obtenir les documents via l'URL suivant`), **persons**
(`Alain Piscione`), and **bare group / class names** (`Thyssenkrupp`, `High
Court`, `Am Trust`). Under the wall those fold to one row per string. That is a
conflation for `High Court`-shaped names in small numbers and a tidy-up for the
garbage strings — the same trade issue 234 made in-country, and the rows are
provisional name-only identity either way. Accepted, recorded; a `non-name`
verdict cohort for the recurring form strings is cheap to add later.

**Wet slice:** job 666, `max_groups` 2,000 (the plan folds in name order, so the
first slice is the names sorting first). The classifier allowed the enqueue.
Verify after: `/health`, the job's counts (removed / mentions), a folded name by
bounded read, then larger slices under the residual plan.

**Next cohort:** the 57,669 over-wall names hold ~2.9M rows and need verdicts
— a listing job for the over-wall names by rows (the census stops at 200) and
a 311-style read, person or agent, in cohorts of a few hundred.

## Pace (job 666, the first wet slice)

The planning walk took ~45 min (the class is 8M rows and every multi-row name
asks the tier gate), then the fold ran at **~1.7 groups/s** — the same order
as R2's merge loop (1,846 groups in 745 s), i.e. ~0.5 s per group of mostly
fixed per-statement cost: a country-less provisional loser holds one mention
and almost never a tender row, yet the generic repoint ran three party/winner
UPDATEs and the winner-dup pass for every one of them. At that pace the
516,400-group plan is ~3.5 days of folding plus a 45-min re-plan per slice.

Two fixes, the first landed while the slice ran: (1) a **light repoint path**
— the tender probe the loop already runs says whether any party/winner row
exists; when none does, only the mention repoint runs (gate green, deploying
with the census cap change); (2) run the residual **uncapped** once the pace is
measured, so the 45-min re-plan is paid once, with the stop flag and the
residual re-record as the safety net.

### The real cost: foreign-key proving on the parent delete

Timed through the SQL endpoint while job 666 folded: every read the loop makes
(the three party probes, the mention probe, the org by PK) answers in
milliseconds, the disks are NVMe, and load is one core — so the ~0.45 s per
deleted org row is not I/O and not the reads. It is the engine proving, on the
write path, that no row of the five child tables (`organization_mentions`,
`tender_version_parties`, `…bid_parties`, `…result_winners`,
`organization_names`) still references the parent — the same finding
`lib.rs` records for mention deletes ("deleting one mention row costs ~2.2 s on
prod … not index-served on the write path"). The fold moves every child off the
loser BEFORE deleting it, so the graph is self-consistent by construction and
the proof buys nothing. The wet loop now runs with foreign keys OFF and
restores them whatever happens — the projection's issue-19 precedent, bracketed
and pinned by a test through `Db::foreign_keys_enabled`. The R2/E0 merge loops
carry the same cost and could take the same bracket (their loser deletes are
the same shape); filed as a follow-up rather than changed blind.

## First wet slice (job 666, 4,865 s: ~2,650 s planning + ~2,200 s folding)

Parity passed against the recorded 516,400; the first 2,000 plan groups (names
sorting first) folded: **5,871 rows removed**, 5,871 mentions repointed, 6,292
parties, **20,659 winners** and 3,785 tenders touched. One thing the light
path's comment got wrong and its code got right: these country-less
provisional rows DO carry tender rows — they are old-era award winners and
parties, 20,659 winner rows over 5,871 losers — so the tender probe decides per
loser and the shortcut only fires where it is true. The residual plan
(514,400 groups) is re-recorded; the next slice continues under parity.

## Second wet slice (job 667, 2,706 s): the foreign-key bracket measured

Planning ~2,550 s (the tier gate per group makes the walk ~3,200 rows/s), then
**30,000 groups / 147,671 rows folded in roughly three minutes** — 147,489
mentions, 155,840 parties, 174,868 winners repointed, 111,028 tenders touched.
Per row: ~0.37 s with foreign keys on (job 666) → ~1–2 ms with them off. The
proof was the whole cost. Verified by bounded reads: `aena, aeropuertos
espanoles y navegacion aerea` 3,767 → 1 NULL-country provisional row,
`aeroports de paris` 3,577 → 1; names sorting later (`berliner wasserbetriebe`,
`european commission`) untouched, as the slice order says. Residual plan
re-recorded at 484,400 groups. The uncapped run is next: one 45-min plan walk
plus ~1 h of folding, stop flag and residual re-record as the net.

## The uncapped run (job 668, 4,885 s) — the stock fold is complete

| | |
| --- | --- |
| plan (residual after the two slices) | 484,400 groups / 3,254,420 rows |
| folded | **484,400 groups, 3,254,420 rows removed** |
| references moved | 3,254,123 mentions · 3,471,777 parties · 2,168,088 winners · 2,464,465 tenders touched |
| pace | ~2,550 s planning + ~2,300 s folding ≈ 1,400 rows/s |
| parity | held at 484,400; residual re-recorded at 0 |

Over the three runs: **516,400 names folded to one row each, 3,407,962
provisional rows gone** — 43% of the 8.0M-row class, and every name the
tier gate admits. Bounded reads after the run: `ville de paris` 4,604 → 1,
`tribunal administratif de paris` 23,400 → 1, `european commission` 28,219 →
1; the refused shapes stand exactly as before (`tendsign` 16,597,
`amministrazione comunale` 13,863, `infructueux` 6,027). Health green
throughout; the change cursor advanced by ~6.2M events (2.46M tenders touched
plus the org removals), which downstream readers will see as one large burst.

What stands: **57,669 over-wall names, ~2.9M rows**, each needing a verdict
(unit 5). Sunday's `build-org-match-keys` rebuilds the wall's carrier counts
without the 3.4M folded rows, so the census's echo class and the E0 plan's
`admitted_echo` will both move; the next weekly report is the read.

## Unit 5: the over-wall backlog

**Post-fold census (job 669, 960 s, listing cap 2,000):** 4,625,100 rows under
1,714,593 names; **57,692 names in more than one row, 2,968,199 rows** (a
fold removes 2,910,507); sizes 2: 5,313 · 3–5: 6,461 · 6–20: 10,981 · 21–100:
28,991 · 101+: 5,946. Every one of the 2,000 largest is over the wall (1,977)
or verdict-refused (23); they carry 1,031,395 mentions. Beyond the first
cohort's 200 names the listing holds 1,958 over-wall names with 874,162 rows.

**Cohort 2 (`over-wall-2-2026-09-04`, the next 400 by rows, 1,313 → 596
rows each):** 328 `single` (councils, departmental and regional councils,
ministries with a country in the name, NHS boards, Staatsbauämter, Polish
pharma wholesalers with a legal form, universities, the US supply centres),
15 `generic` (`Mairie`, `Hôtel de ville`, `COMMUNAUTE URBAINE`, `Centre
communal d'action sociale`, `Ministry of Justice`, `DEPARTMENT OF TRANSPORT`,
the bare `Comité consultatif interrégional…`, the Polish class names for a
hospital), 18 `non-name` (`Product (A)`, `Price(s): Confidential`, `siehe
I.1)`, `j.w.`, `Not applicable`, `Voir TA de Strasbourg`, `PHARE`…), 39
`unclear` (bare multinationals — `Pfizer`, `Roche`, `Abbott`, `Boston
Scientific`, `Covidien`, `Schindler`, `Bureau Veritas` — whose country-less
mentions could be any national subsidiary, plus `la poste`, `Post Office`,
`COMMISSION`, `FED`). 273,509 rows admitted. Dry re-plan job 670, then the
wet run.
