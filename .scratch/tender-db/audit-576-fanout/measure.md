# Audit 576 U3 — did touchpoint minting fan out into duplicate organizations?

Date 2026-10-24, 12:50–13:02Z. Read-only: 18 bounded `/v1/sql` statements (budget 25, each >= 4 s apart, no 408,
no error body, longest 2.07 s), 4 public GETs (>= 2.5 s apart). No git, cargo, box or admin writes. Every write is under
this directory. Tools: `tools/box.py` (plan-probe first, then one ssh `/root/sq.sh` call, never retried) and `tools/get.sh`.
Raw bodies: `box/raw/NN-<name>.json`, `api/raw/*.json`. Logs: `box/queries.log`, `api/gets.log`.

**Verdict: CLEAN as far as a flood goes, with SOME duplicates.** The duplicates are spelling variants, and every one
follows the resolver's documented spelling-only key. Separately, a few generic touchpoint labels became hub orgs
that link unrelated buyers. That is a gate gap, not a fan-out (section 3).

## 1. The code path: what should happen to one tribunal named in 1,000 notices

- `project.rs` `touchpoint_decision` (7370-7470): when a touchpoint referenced by an OPT-301 Lot/Part role prints a
  name that differs from its parent ORG's, and the gate does not drop it, its verdict is
  `Mint(Mention { section_id: TPO-…, name: first BT-500-Organization-TouchPoint, variants per language,
  country: own BT-514 else the parent ORG's, identifier: None })`. `touchpoint_mentions` (6289) returns these mentions
  beside the ordinary ones, and Phase 1 records them through the same `Db::resolve_mentions`.
- `bind_organizations` (6320) binds the role to whatever organization the recorded `(notice, TPO-…)` mention resolved
  to. The results graph excludes touchpoint sections.
- `store/canonical.rs` `resolve_one_mention`, identifier-less arm (23593-23810):
  - The key is `(org_name_norm(name), register_jurisdiction(country))`. `org_name_norm` is Unicode lowercase, collapsed
    whitespace and trailing `.`/`,`/`;` stripped. It deliberately does NOT fold accents, legal forms, typos or
    translations (7381-7406).
  - Lookup order: the run-wide cache `name_of`, then
    `SELECT id FROM organizations WHERE name_norm=? AND country=? AND identifier IS NULL ORDER BY id LIMIT 1`
    (lowest-id twin, seek on `organizations_name_country`). A new provisional row is minted only when both miss, and
    it goes straight into `name_of`.
  - A country-less mention takes issue 351's genericness wall instead. Here that never happens: 0 of the new orgs
    lack a country, thanks to the parent-country fallback.
- **So the same tribunal named in 1,000 notices, spelled the same up to case, whitespace and trailing punctuation, in
  the same country, ends on ONE organization.** That is the lowest-id existing identifier-less row (plain TACRC → 3955,
  the compound TACRC/TSJ → 22077939), or one new row if none existed.
- Duplicates can come only from:
  - spelling variants that differ beyond that key (accents, typos, suffixes such as "(Punto de contacto)", "-
    Secretaría", translations);
  - a body known only through identified rows (`identifier IS NULL` never name-binds them);
  - a different or wrong country.
  None of these is new to 576: ORG sections fragment the same way.

## 2. Measurements

### 2a. Organizations created during 2759 (statements #1-#4)

- PK probes found the range: max(id) = **31,978,722** at 13:00Z, so nothing has been minted since 2759.
- #4 read every row with id 31,974,376-31,978,722 (4,347 rows, one PK range):

| created_at | rows | ids | what |
| --- | --- | --- | --- |
| 1792741179 (10-23 07:39Z) | 455 | 31974376-31974830 (window start) | daily ingest |
| 1792827571 (10-24 07:39Z) | 141 | 31974831-31974971 | daily ingest |
| 1792841726 (10-24 11:35:26Z) | 6 | 31974972-31974977 | touchpoint-only orgs minted by a fold on the new code 11 min before 2759 (deploy 11:32Z) |
| **1792842411 (11:46:51Z)** | **3,745** | **31,974,978-31,978,722** | **job 2759** (range [1792842410, 1792845134]) |

- **New-org count for 2759: 3,745.** The range lo = 31,974,978, hi = 31,978,722 is contiguous with 0 gaps.
- Every one of the 3,745 is provisional and identifier-less, and has a country (0 NULL).
- Every one has >= 1 mention, and **all of their 24,101 mentions are touchpoint mentions** (`section_id LIKE 'TPO-%'`).
  No new org carries an ORG-section mention, so 2759 minted no organization except for touchpoints.
- Countries: ES 2,709, FR 282, IT 173, DE 144, NL 118, PL 118, PT 54, GR 28, FI 19, SK 14, MD 14, MK 11, DK 10, and 13 more.
- TPO mentions per new org: 1,317 have 1; 1,750 have 2-5; 510 have 6-20; 138 have 21-100; 30 have more than 100.
  The median is 2.
- With the 6 orgs from 11:35Z there are 3,751 touchpoint-only orgs since the deploy, holding 24,664 TPO mentions.

### 2b. Most repeated (name_norm, country) pairs among the new orgs

- **Exact (name_norm, country) duplicates among the 3,745: 0.** No name_norm occurs under two countries either. This
  is computed locally from #4's full rows; one statement covered the whole range, so no windowed GROUP BY was needed.
- So the "top 30 pairs" are all count 1. The real duplicate candidates come from a looser key (NFKD accent fold,
  lowercase, parenthetical removed, punctuation to spaces, same country): **124 clusters holding 271 orgs, which is
  147 surplus rows (3.9% of the 3,745).** The top ones:

| n | loose key (country) | members (id: TPO mentions) |
| --- | --- | --- |
| 5 | agencia de vivienda y rehabilitacion de andalucia (ES) | 31975748:21 "…(Punto de contacto)", 31975794:1, 31975933:1 "(PC)", 31976931:1 "(otro punto de contacto)", +1 |
| 4 | consejeria de hacienda transicion economica y transformacion digital (ES) | 31975200:44, 31975271:66, 31978061:11, 31978067:7 (accent variants) |
| 4 | daj cellule marches (FR) | 31975448:4, 31975471:4, 31975578:8, 31976088:3; also 31974975:26 from 11:35Z |
| 4 | tribunal administrativo de recursos contractuales de la junta de andalucia (ES) | 31975749:20, 31975768:4, 31975792:2, 31975934:1. Pre-existing 3601 holds the plain name |
| 4 | consejeria de economia hacienda fondos europeos y transformacion digital (ES) | 31976235:49, 31976399:1, 31977289:1, 31977743:1 |
| 3 | ajuntament de la vall d uixo (ES) | 31975411:2, 31975634:16, 31978039:4 |
| 3 | d g de contratacion del ayuntamiento de logrono o juzgado … (ES) | 31975430:51, 31975574:5, 31975644:2 |
| 3 | tribunal judiciaire … 54035 nancy cedex (FR) | 31975462:40, 31976356:1, 31977484:1 |
| 3 | tribunal administrativo de contratos publicos de aragon (ES) | 31976677:4, 31976731:14, 31978367:3. Pre-existing 7846 |
| 3 | area de contratacion de maz m c s s no 11 (ES) | 31976989:4, 31978190:1, 31978236:2. Pre-existing 18371131 |
| 3 | instituto nacional de seguridad y salud en el trabajo oa mp (ES) | 31977122:14, 31977830:10, 31978249:4 |
| 3 | tribunal adtvo central de recursos contractuales (ES) | 31977668:2, 31978046:3, 31978060:2 |
| 2 | direccion principal (ES) | 31974991:420, 31975075:13 |
| 2 | fe ei w beschaffung infrastruktur west (DE) | 31975097:101, 31976555:7 |
| 2 | unidad de contratacion de la intendencia de cartagena (ES) | 31975128:96, 31977542:7 |

  The full cluster list is reproducible from `box/raw/04-s04_newrows.json`.
- Translations do not cluster under any key. One example: Moldova's review authority appears as 6 new rows ("National
  Authority for the Resolution of Appeals" 9, "… of Complaints" 7, "National Appeals Authority" 3, "Autoritatea
  Națională de soluționare a Contestațiilor" 1, "… pentru Soluționarea Contestațiilor" 5, "National Agency for the
  Handling of Complaints" 1).

### 2c. Do the new orgs duplicate organizations that already existed? (#10, #11, #16)

- **#16, every new org (3,751), exact name_norm seek against id < 31,974,972:** 150 new orgs (4.0%) have a
  pre-existing row with the same name_norm. Those 150 hold 2,030 TPO mentions, 0.9% of the minted mentions.
  - 97 match a same-country row. By construction that row is identified, because an identifier-less same-country row
    would have been reused. Examples: 31975409 "Aena. Dirección de Contratación" (505 mentions) beside identified 6745;
    31975283 "Área de Gobierno de Políticas de Vivienda del Ayuntamiento de Madrid" (56) beside 22236441; MD
    ministries 31975483/31975577 (48/46). These hold 1,071 TPO mentions in all.
  - 37 match a country-less row. Examples: "Bilbao" 42, "Torrent" 17, "Viator" 10 (99 country-less twins), plus
    "contact" (NL) and "bic" (NL). These hold 169.
  - 19 match only rows of other countries. Examples: "Principal" ES 715, whose 3 earlier rows are under other
    countries; MK "Body responsible for mediation procedures" 28; "Krajowa Izba Odwoławcza" under **MD** (3 mentions;
    KIO has 267 pre-existing rows of that name_norm). These hold 798.
- **#10, top 10 by mentions plus a salted random 10** (`random.Random('576-fanout-audit')`): only 1 of 20 has a
  same-country pre-existing row (Aena, identified 6745). One more has an other-country match (Principal). The other 18
  are names new to the table.
- **#11, near-identical families by name_norm prefix** (`organizations_name_country` range seeks):

| family (prefix) | rows before 2759 (identifier-less) | new rows | their TPO mentions |
| --- | --- | --- | --- |
| tribunal administrativo central de re… (TACRC) | 477 (436) | 51 | 457 |
| krajowa izba odwo… (KIO) | 1,039 (778) | 4 | 8 |
| tribunal administrativo de contratación pública de la comunidad de madrid… (2261) | 77 (57) | 2 | 12 |
| tribunal administrativo de recursos contractuales de la junta de andaluc… | 60 (40) | 5 | 36 |
| tribunal català de contractes del sector p… | 50 (23) | 2 | — |
| tribunal administrativo de contratos públicos de arag… | 36 (36) | 4 | — |
| … contratos públicos de la comunidad autónoma de canarias | 18 (17) | 3 | — |
| rechtbank oost-brabant… | 28 (24) | 2 | — |
| stadt fürth… | 129 (124) | 1 | — |
| aena… | 206 (181) | 1 | — |
| principal… | 62 (52) | 1 | — |

  The new TACRC-family rows are typos and suffixes: "recuersos" 86, "- recurso especial" 76 and 48, "recursoscontractuales"
  40, "/ibermuta" 27, "(tacrc" 26, "… y art. 16 de la ley 39/2015" 17. **The families were already fragmented before
  576**, for example 477 TACRC-prefix rows. Minting added about 1-11% to each family and did not create the
  fragmentation.

### 2d. Do the minted mentions bind new orgs or pre-existing ones? (#5-#8, #12-#15)

- **Exact denominator, by mention rowid.** `organization_mentions` is a rowid table that appends. #12-#15 probed rowids
  and found 2759's run of touchpoint mentions:
  - It starts in (50,437,500, 50,437,625]. The row at 50,437,500 is FTS ORG-GB-COH on notice 47435924; the row at
    50,437,625 is TPO-0003 on notice 23466093, the lowest carrier, so the projection runs in notice order.
  - It ends at a max rowid in [50,670,875, 50,670,999].
  - All 103 distinct probe points inside the run are TPO sections, in ascending notice order 23.47M → 47.40M.
  - **So 2759 minted ≈ 233,251–233,499 touchpoint mentions.** That assumes the run is contiguous: nothing deletes inside
    it, and retire_mentions handles only touchpoints that mint nothing.
  - Cross-check: 491,287 minted role references / 233.4k ≈ 2.10 references per minted mention. Unit 1's window gave
    1,179 references over 567 sections, about 2.1.
- **Split: 24,101 (10.3%) bind the 3,745 new orgs; ≈ 209.2k (89.7%) bind organizations that already existed.**
  Counting the 6 orgs from 11:35Z as new gives at most 24,664 (10.6%).
- **Independent sample**, joining touchpoint sections (`notice_sections_kind_notice`) to mentions over three notice
  windows that #5's density probe showed to hold touchpoints:

| window (notice ids) | touchpoint sections | no mention (aliased, dropped or unreferenced) | TPO mentions | to new orgs | distinct new / pre-existing orgs |
| --- | --- | --- | --- | --- | --- |
| 24,000,000-24,019,999 | 2,087 | 320 | 1,767 | 180 (10.2%) | 127 / 477 |
| 25,000,000-25,039,999 | 4,084 | 625 | 3,459 | 388 (11.2%) | 191 / 586 |
| 47,390,000-47,419,999 | 2,188 | 315 | 1,873 | 226 (12.1%) | 129 / 365 |
| **total** | 8,359 | 1,260 | **7,099** | **794 (11.2%)** | 994 pre-existing orgs, 6,305 mentions |

  The sample's 11.2% agrees with the exact 10.3%.
- **The pre-existing targets are the canonical review bodies** (#9, the top 30 by sample mentions plus a salted 15):
  - 3955 TACRC (1,450 of 6,305 sample mentions, 23%), 1166464 Rechtbank Den Haag 199, 2261 Madrid 172, 1617 KIO 146,
    3453018 Rb Midden-Nederland 130, 1163905 Rb Rotterdam 126, 3601 TARC Andalucía 113, 22077939 TACRC/TSJ 110,
    6724282 Canarias 95, and so on.
  - All 45 rows read are identifier-less provisional rows from before 576. 44 carry the rebuild epoch 2026-08-15; one
    is 31974977, from 11:35Z.
  - The top 10 hold 42% of pre-existing-bound sample mentions, the top 30 hold 56%.

### 2e. The two known examples (GETs and #18)

- **22077939** "Tribunal Administrativo Central de Recursos Contractuales (TACRC) o Tribunal Superior de Justicia (TSJ).",
  ES, provisional:
  - It is a pre-existing row (rebuild epoch). It now has **2,534 mentions on 2,534 notices, 2,531 of them touchpoint
    mentions** (#18). Before 576 it had only 3.
  - The compound named in about 2.5k notices bound ONE org.
  - `GET /v1/organizations?name_prefix=tribunal administrativo central de recursos contractuales (tacrc) o` returns
    exactly 1 item, itself.
  - The wider TACRC family is the 477 + 51 rows of 2c.
- **2261** "Tribunal Administrativo de Contratación Pública de la Comunidad de Madrid", ES, provisional:
  - It has **28,331 mentions** (GET). In the sample windows it took 172 touchpoint mentions.
  - `GET ?name_prefix=tribunal administrativo de contratación pública de la comunidad de madrid&limit=200` returns
    **79 rows**:
    - identified twins such as 6604 S2813600J (152), 9945 S7800001 (781), 22536888 (102) and 22566627 (49);
    - country-less 1480171 (30); EE and CO rows (8 each);
    - many suffix and prose variants, for example 1223846 "… o Sala de lo Contencioso-Administrat…" (371), 4552463 (200),
      13083979 (199) and 2364715 (111).
  - Only 2 of the 79 are new: 31975690 "… o Tribunales de lo Contencioso Admin…" (9) and 31978209 "…MadridORGANISMO" (3).

## 3. Assessment: flood, some duplicates, or clean?

**No flood.**
- 2759 minted ≈ 233.4k touchpoint mentions (491,287 role references) and created only **3,745 organizations**, about
  1 new org per 62 minted mentions.
- **89.7% of the minted mentions bound organizations that already existed**, mostly the one canonical identifier-less
  row of each tribunal (3955, 2261, 1617, the Rechtbanken).
- Among the new orgs there are **0 exact (name_norm, country) duplicates**, which the resolver's `name_of` cache and
  its lowest-id probe guarantee.

**Some duplicates, all of the kind the key allows by design:**
- 147 surplus accent, case, punctuation and suffix variants among the new orgs (124 loose clusters);
- 51 more TACRC-family typo and suffix rows on top of the 477 that already existed;
- 97 same-name twins of identified rows, plus 37 of country-less rows. They hold 1,240 mentions, 0.5% of those minted;
- translations, such as Moldova's ANSC in 6 rows;
- a few wrong countries: a Polish KIO name under MD, Vergabekammer Baden-Württemberg under BD.

Cleaning these is the job of the existing match and merge tools (N2 `match_norm`, the 234/351 rules). This drain
did not make it worse in kind; it added about 4% new rows.

**One finding that is not a duplicate (gate gap in D576-1 (iv)): generic touchpoint labels minted as hub orgs** (#17):

| org | name | TPO mentions | notices | distinct parent ORGs |
| --- | --- | --- | --- | --- |
| 31974991 | Dirección Principal (ES) | 420 | 420 | **18** |
| 31975006 | Dirección Postal (ES) | 221 | 220 | **9** |
| 31975008 | Principal (ES) | 715 | 715 | **6** |
| 31975019 | Sede Tibidabo (ES) | 621 | 621 | 1 |
| 31975180 | Info Adicional (ES) | 294 | 294 | 1 |
| 31975009 | RECURSOS (ES) | 268 | 268 | 1 |
| 31975004 | Dirección Recepción de Recursos (ES) | 143 | 143 | 1 |
| 31975568 | Dirección Recurso (ES) | 105 | 105 | 1 |
| 31975031 | Stadt Fürth - Zentrale Vergabestelle (DE) | 556 | 556 | 2 |
| 31975696 | Directeur Strategische Inkoop Amsterdam UMC (NL) | 513 | 236 | 1 |

- The first three are labels, not bodies. Each links touchpoints of 18, 9 and 6 unrelated buyers into one
  organization, so the party names served per 456 are what was printed, but the org-level rollups mix buyers.
- The single-parent ones are one publisher's labelling convention. They are N under D576-1's accepted cost, or a
  non-name in the Info Adicional and RECURSOS cases.
- Others of the same shape: "Presentación de recursos", "contact" (NL), MK "Body responsible for mediation
  procedures", MK "Service from which information about the review procedure may be obtained".
- `touchpoint_gate` has no generic-label stem, so these pass. A small lexicon added to the gate, plus a re-mint of
  these few orgs, would close it. The candidate stems are principal, dirección principal/postal, info(rmación)
  adicional, recursos, dirección recurso, contact, and "body responsible for…" / "service from which…".

### Caveats
- `plan/plan-copy.db` lacks prod's deferred organization indexes (`organizations_name_norm_id`,
  `organizations_name_country`, `organization_mentions_notice`, …; canonical.rs 17706-17726). So the name_norm
  statements (#10, #11, #16) plan as SCAN on it.
  - I sent them on the plan from `plan/plan-idx.db`, the same copy with exactly those DDLs applied through turso's
    plan-probe `exec`, which plans SEARCH on those indexes.
  - Prod having a name_norm index is evidenced by unit 1's R4, a name_norm equality on prod that answered. All three
    statements answered in 0.38-1.26 s.
  - In #10, #11 and #16 `+o.id` / `+e.id` keep the planner from driving by the rowid range.
- `organizations.created_at` is one stamp per resolver run (2759's rows all read 1792842411). The 2c and 2d boundaries
  use the id range and the rowid band, not a timestamp scan.
- The 233.4k denominator assumes the 2759 rowid band is contiguous and all touchpoint. 103 of 103 distinct probe points were,
  and the 11.2% sample agrees with the resulting 10.3%.
- The 11:35:26Z fold that minted 6 touchpoint-only orgs is not identified here.

## Appendix: every statement, its local plans, its answer
### #1 `s01_coarse` — 2026-10-24T12:52:50.705209Z, 0.37 s, ok=True, rows=20, truncated=False

```sql
WITH probe(x) AS (VALUES (30000000),(31000000),(32000000),(32500000),(33000000),(33500000),(34000000),(34500000),(35000000),(35500000),(36000000),(36500000),(37000000),(37500000),(38000000),(39000000),(40000000),(42000000),(45000000),(50000000)) SELECT p.x, (SELECT o.id FROM organizations o WHERE o.id >= p.x ORDER BY o.id LIMIT 1) AS id, (SELECT o.created_at FROM organizations o WHERE o.id >= p.x ORDER BY o.id LIMIT 1) AS created_at FROM probe p
```

Local plan, `plan/plan-copy.db` (byte copy of 577-triage/plan-copy.db, sha256 80391c50…):
```
SCAN probe AS p
CORRELATED SCALAR SUBQUERY 1
  SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
CORRELATED SCALAR SUBQUERY 2
  SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
```

Local plan, `plan/plan-idx.db` (same copy + prod's deferred organization indexes):
```
SCAN probe AS p
CORRELATED SCALAR SUBQUERY 1
  SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
CORRELATED SCALAR SUBQUERY 2
  SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
```

Answer (columns x, id, created_at):
```
[30000000, 30000005, 1787202689]
[31000000, 31000000, 1790580956]
[32000000, null, null]
[32500000, null, null]
[33000000, null, null]
[33500000, null, null]
[34000000, null, null]
[34500000, null, null]
[35000000, null, null]
[35500000, null, null]
[36000000, null, null]
[36500000, null, null]
[37000000, null, null]
[37500000, null, null]
[38000000, null, null]
[39000000, null, null]
[40000000, null, null]
[42000000, null, null]
[45000000, null, null]
[50000000, null, null]
```


### #2 `s02_fine` — 2026-10-24T12:52:57.443138Z, 0.38 s, ok=True, rows=40, truncated=False

```sql
WITH probe(x) AS (VALUES (31025000),(31050000),(31075000),(31100000),(31125000),(31150000),(31175000),(31200000),(31225000),(31250000),(31275000),(31300000),(31325000),(31350000),(31375000),(31400000),(31425000),(31450000),(31475000),(31500000),(31525000),(31550000),(31575000),(31600000),(31625000),(31650000),(31675000),(31700000),(31725000),(31750000),(31775000),(31800000),(31825000),(31850000),(31875000),(31900000),(31925000),(31950000),(31975000),(32000000)) SELECT p.x, (SELECT o.id FROM organizations o WHERE o.id >= p.x ORDER BY o.id LIMIT 1) AS id, (SELECT o.created_at FROM organizations o WHERE o.id >= p.x ORDER BY o.id LIMIT 1) AS created_at FROM probe p
```

Local plan, `plan/plan-copy.db` (byte copy of 577-triage/plan-copy.db, sha256 80391c50…):
```
SCAN probe AS p
CORRELATED SCALAR SUBQUERY 1
  SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
CORRELATED SCALAR SUBQUERY 2
  SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
```

Local plan, `plan/plan-idx.db` (same copy + prod's deferred organization indexes):
```
SCAN probe AS p
CORRELATED SCALAR SUBQUERY 1
  SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
CORRELATED SCALAR SUBQUERY 2
  SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
```

Answer (40 rows; the rows around each change shown, full body in box/raw/02-s02_fine.json; columns x, id, created_at):
```
[31025000, 31025000, 1790580956]
[31475000, 31475000, 1790580956]
[31500000, 31500000, 1790582275]
[31525000, 31525000, 1790582275]
[31550000, 31550000, 1790734697]
[31575000, 31575000, 1790744920]
[31600000, 31600000, 1790784534]
[31625000, 31625000, 1790804345]
[31650000, 31650000, 1791115407]
[31675000, 31675000, 1791120612]
[31725000, 31725000, 1791120612]
[31750000, 31750000, 1791147676]
[31775000, 31775000, 1791611361]
[31825000, 31825000, 1791611361]
[31850000, 31850000, 1791618760]
[31900000, 31900000, 1791618760]
[31925000, 31925000, 1791913537]
[31950000, 31950000, 1791922500]
[31975000, 31975000, 1792842411]
[32000000, null, null]
```


### #3 `s03_finer` — 2026-10-24T12:53:05.447570Z, 0.37 s, ok=True, rows=80, truncated=False

```sql
WITH probe(x) AS (VALUES (31950625),(31951250),(31951875),(31952500),(31953125),(31953750),(31954375),(31955000),(31955625),(31956250),(31956875),(31957500),(31958125),(31958750),(31959375),(31960000),(31960625),(31961250),(31961875),(31962500),(31963125),(31963750),(31964375),(31965000),(31965625),(31966250),(31966875),(31967500),(31968125),(31968750),(31969375),(31970000),(31970625),(31971250),(31971875),(31972500),(31973125),(31973750),(31974375),(31975000),(31975625),(31976250),(31976875),(31977500),(31978125),(31978750),(31979375),(31980000),(31980625),(31981250),(31981875),(31982500),(31983125),(31983750),(31984375),(31985000),(31985625),(31986250),(31986875),(31987500),(31988125),(31988750),(31989375),(31990000),(31990625),(31991250),(31991875),(31992500),(31993125),(31993750),(31994375),(31995000),(31995625),(31996250),(31996875),(31997500),(31998125),(31998750),(31999375),(32000000)) SELECT p.x, (SELECT o.id FROM organizations o WHERE o.id >= p.x ORDER BY o.id LIMIT 1) AS id, (SELECT o.created_at FROM organizations o WHERE o.id >= p.x ORDER BY o.id LIMIT 1) AS created_at FROM probe p
```

Local plan, `plan/plan-copy.db` (byte copy of 577-triage/plan-copy.db, sha256 80391c50…):
```
SCAN probe AS p
CORRELATED SCALAR SUBQUERY 1
  SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
CORRELATED SCALAR SUBQUERY 2
  SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
```

Local plan, `plan/plan-idx.db` (same copy + prod's deferred organization indexes):
```
SCAN probe AS p
CORRELATED SCALAR SUBQUERY 1
  SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
CORRELATED SCALAR SUBQUERY 2
  SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
```

Answer (80 rows; the rows around each change shown, full body in box/raw/03-s03_finer.json; columns x, id, created_at):
```
[31950625, 31950625, 1791922500]
[31952500, 31952500, 1791922500]
[31953125, 31953125, 1791924730]
[31954375, 31954375, 1791924730]
[31955000, 31955000, 1791925489]
[31955625, 31955625, 1791925489]
[31956250, 31956250, 1791927514]
[31956875, 31956875, 1791948638]
[31957500, 31957500, 1791955554]
[31958125, 31958125, 1791963601]
[31958750, 31958750, 1791963601]
[31959375, 31959375, 1791996867]
[31960000, 31960000, 1792001138]
[31960625, 31960625, 1792003287]
[31961875, 31961875, 1792003287]
[31962500, 31962500, 1792005457]
[31963125, 31963125, 1792007661]
[31963750, 31963750, 1792007661]
[31964375, 31964375, 1792009933]
[31965000, 31965000, 1792012146]
[31965625, 31965625, 1792012146]
[31966250, 31966250, 1792014854]
[31966875, 31966875, 1792049976]
[31967500, 31967500, 1792049976]
[31968125, 31968125, 1792136367]
[31968750, 31968750, 1792136367]
[31969375, 31969375, 1792395355]
[31970000, 31970000, 1792395355]
[31970625, 31970625, 1792482087]
[31971250, 31971250, 1792482087]
[31971875, 31971875, 1792568363]
[31972500, 31972500, 1792568363]
[31973125, 31973125, 1792654836]
[31973750, 31973750, 1792741179]
[31974375, 31974375, 1792741179]
[31975000, 31975000, 1792842411]
[31978125, 31978125, 1792842411]
[31978750, null, null]
[32000000, null, null]
```


### #4 `s04_newrows` — 2026-10-24T12:53:26.959399Z, 1.37 s, ok=True, rows=4347, truncated=False

```sql
SELECT o.id, o.created_at, o.country, o.identifier_kind, o.identifier, o.provisional, o.name, o.name_norm, (SELECT count(*) FROM organization_mentions m WHERE m.organization_id = o.id) AS mentions, (SELECT count(*) FROM organization_mentions m WHERE m.organization_id = o.id AND m.section_id LIKE 'TPO-%') AS tpo_mentions FROM organizations o WHERE o.id BETWEEN 31974376 AND 31984375 ORDER BY o.id
```

Local plan, `plan/plan-copy.db` (byte copy of 577-triage/plan-copy.db, sha256 80391c50…):
```
SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
CORRELATED SCALAR SUBQUERY 1
  SEARCH m USING INDEX organization_mentions_org (organization_id=?)
CORRELATED SCALAR SUBQUERY 2
  SEARCH m USING INDEX organization_mentions_org (organization_id=?)
```

Local plan, `plan/plan-idx.db` (same copy + prod's deferred organization indexes):
```
SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
CORRELATED SCALAR SUBQUERY 1
  SEARCH m USING INDEX organization_mentions_org (organization_id=?)
CORRELATED SCALAR SUBQUERY 2
  SEARCH m USING INDEX organization_mentions_org (organization_id=?)
```

Answer: Full rows of 4,347 orgs (ids 31974376-31978722); summarized in section 2a-2d; raw: box/raw/04-s04_newrows.json


### #5 `s05_density` — 2026-10-24T12:54:48.708534Z, 0.38 s, ok=True, rows=29, truncated=False

```sql
WITH probe(x) AS (VALUES (20000000),(21000000),(22000000),(23000000),(24000000),(25000000),(26000000),(27000000),(28000000),(29000000),(30000000),(31000000),(32000000),(33000000),(34000000),(35000000),(36000000),(37000000),(38000000),(39000000),(40000000),(41000000),(42000000),(43000000),(44000000),(45000000),(46000000),(47000000),(47400000)) SELECT p.x, (SELECT count(*) FROM notice_sections s WHERE s.kind = 'Touchpoint' AND s.notice_id BETWEEN p.x AND p.x + 4999) AS tp_sections FROM probe p
```

Local plan, `plan/plan-copy.db` (byte copy of 577-triage/plan-copy.db, sha256 80391c50…):
```
SCAN probe AS p
CORRELATED SCALAR SUBQUERY 1
  SEARCH s USING INDEX notice_sections_kind_notice (kind=? AND notice_id>=? AND notice_id<=?)
```

Local plan, `plan/plan-idx.db` (same copy + prod's deferred organization indexes):
```
SCAN probe AS p
CORRELATED SCALAR SUBQUERY 1
  SEARCH s USING INDEX notice_sections_kind_notice (kind=? AND notice_id>=? AND notice_id<=?)
```

Answer (columns x, tp_sections):
```
[20000000, 0]
[21000000, 0]
[22000000, 0]
[23000000, 0]
[24000000, 604]
[25000000, 244]
[26000000, 0]
[27000000, 9]
[28000000, 0]
[29000000, 0]
[30000000, 0]
[31000000, 0]
[32000000, 0]
[33000000, 0]
[34000000, 0]
[35000000, 0]
[36000000, 0]
[37000000, 0]
[38000000, 0]
[39000000, 0]
[40000000, 0]
[41000000, 0]
[42000000, 0]
[43000000, 0]
[44000000, 0]
[45000000, 0]
[46000000, 0]
[47000000, 0]
[47400000, 357]
```


### #6 `s06_split_w1` — 2026-10-24T12:55:18.995498Z, 0.61 s, ok=True, rows=605, truncated=False

```sql
SELECT m.organization_id AS org, count(*) AS n, count(DISTINCT s.notice_id) AS notices FROM notice_sections s LEFT JOIN organization_mentions m ON m.notice_id = s.notice_id AND m.section_id = s.section_id WHERE s.kind = 'Touchpoint' AND s.notice_id BETWEEN 24000000 AND 24019999 GROUP BY +m.organization_id ORDER BY n DESC
```

Local plan, `plan/plan-copy.db` (byte copy of 577-triage/plan-copy.db, sha256 80391c50…):
```
SEARCH s USING INDEX notice_sections_kind_notice (kind=? AND notice_id>=? AND notice_id<=?)
SEARCH m USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=? AND section_id=?) LEFT-JOIN
USE SORTER FOR GROUP BY
USE HASH TABLE FOR count(DISTINCT)
USE SORTER FOR ORDER BY
```

Local plan, `plan/plan-idx.db` (same copy + prod's deferred organization indexes):
```
SEARCH s USING INDEX notice_sections_kind_notice (kind=? AND notice_id>=? AND notice_id<=?)
SEARCH m USING INDEX organization_mentions_notice (notice_id=?) LEFT-JOIN
USE SORTER FOR GROUP BY
USE HASH TABLE FOR count(DISTINCT)
USE SORTER FOR ORDER BY
```

Answer: 605 groups; summarized in 2d; raw: box/raw/06-s06_split_w1.json


### #7 `s06_split_w2` — 2026-10-24T12:55:23.936033Z, 0.77 s, ok=True, rows=778, truncated=False

```sql
SELECT m.organization_id AS org, count(*) AS n, count(DISTINCT s.notice_id) AS notices FROM notice_sections s LEFT JOIN organization_mentions m ON m.notice_id = s.notice_id AND m.section_id = s.section_id WHERE s.kind = 'Touchpoint' AND s.notice_id BETWEEN 25000000 AND 25039999 GROUP BY +m.organization_id ORDER BY n DESC
```

Local plan, `plan/plan-copy.db` (byte copy of 577-triage/plan-copy.db, sha256 80391c50…):
```
SEARCH s USING INDEX notice_sections_kind_notice (kind=? AND notice_id>=? AND notice_id<=?)
SEARCH m USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=? AND section_id=?) LEFT-JOIN
USE SORTER FOR GROUP BY
USE HASH TABLE FOR count(DISTINCT)
USE SORTER FOR ORDER BY
```

Local plan, `plan/plan-idx.db` (same copy + prod's deferred organization indexes):
```
SEARCH s USING INDEX notice_sections_kind_notice (kind=? AND notice_id>=? AND notice_id<=?)
SEARCH m USING INDEX organization_mentions_notice (notice_id=?) LEFT-JOIN
USE SORTER FOR GROUP BY
USE HASH TABLE FOR count(DISTINCT)
USE SORTER FOR ORDER BY
```

Answer: 778 groups; summarized in 2d; raw: box/raw/07-s06_split_w2.json


### #8 `s06_split_w3` — 2026-10-24T12:55:29.731943Z, 0.64 s, ok=True, rows=495, truncated=False

```sql
SELECT m.organization_id AS org, count(*) AS n, count(DISTINCT s.notice_id) AS notices FROM notice_sections s LEFT JOIN organization_mentions m ON m.notice_id = s.notice_id AND m.section_id = s.section_id WHERE s.kind = 'Touchpoint' AND s.notice_id BETWEEN 47390000 AND 47419999 GROUP BY +m.organization_id ORDER BY n DESC
```

Local plan, `plan/plan-copy.db` (byte copy of 577-triage/plan-copy.db, sha256 80391c50…):
```
SEARCH s USING INDEX notice_sections_kind_notice (kind=? AND notice_id>=? AND notice_id<=?)
SEARCH m USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=? AND section_id=?) LEFT-JOIN
USE SORTER FOR GROUP BY
USE HASH TABLE FOR count(DISTINCT)
USE SORTER FOR ORDER BY
```

Local plan, `plan/plan-idx.db` (same copy + prod's deferred organization indexes):
```
SEARCH s USING INDEX notice_sections_kind_notice (kind=? AND notice_id>=? AND notice_id<=?)
SEARCH m USING INDEX organization_mentions_notice (notice_id=?) LEFT-JOIN
USE SORTER FOR GROUP BY
USE HASH TABLE FOR count(DISTINCT)
USE SORTER FOR ORDER BY
```

Answer: 495 groups; summarized in 2d; raw: box/raw/08-s06_split_w3.json


### #9 `s09_pre_names` — 2026-10-24T12:56:33.712817Z, 0.38 s, ok=True, rows=45, truncated=False

```sql
WITH probe(x) AS (VALUES (3955),(1166464),(2261),(1617),(3453018),(1163905),(3601),(22077939),(6724282),(3885177),(1209767),(7846),(3169887),(2168879),(3291033),(1164161),(1174728),(3241594),(2720295),(3301449),(18704536),(3085354),(4439929),(11128836),(6041),(4087357),(10951616),(626),(3329871),(3154066),(1164189),(3601588),(18371131),(1215995),(21854128),(4644451),(31974977),(1172306),(13410817),(1168967),(5751247),(6866591),(19486371),(19467504),(5109301)) SELECT o.id, o.country, o.identifier_kind, o.identifier, o.provisional, o.created_at, o.name FROM probe p JOIN organizations o ON o.id = p.x
```

Local plan, `plan/plan-copy.db` (byte copy of 577-triage/plan-copy.db, sha256 80391c50…):
```
SCAN probe AS p
SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
```

Local plan, `plan/plan-idx.db` (same copy + prod's deferred organization indexes):
```
SCAN probe AS p
SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
```

Answer (columns id, country, identifier_kind, identifier, provisional, created_at, name):
```
[3955, "ES", null, null, 1, 1786785861, "Tribunal Administrativo Central de Recursos Contractuales"]
[1166464, "NL", null, null, 1, 1786785861, "Rechtbank Den Haag"]
[2261, "ES", null, null, 1, 1786785861, "Tribunal Administrativo de Contratación Pública de la Comunidad de Madrid"]
[1617, "PL", null, null, 1, 1786785861, "Krajowa Izba Odwoławcza"]
[3453018, "NL", null, null, 1, 1786785861, "Rechtbank Midden-Nederland"]
[1163905, "NL", null, null, 1, 1786785861, "Rechtbank Rotterdam"]
[3601, "ES", null, null, 1, 1786785861, "Tribunal Administrativo de Recursos Contractuales de la Junta de Andalucía."]
[22077939, "ES", null, null, 1, 1786785861, "Tribunal Administrativo Central de Recursos Contractuales (TACRC) o Tribunal Superior de Justicia (TSJ)."]
[6724282, "ES", null, null, 1, 1786785861, "Tribunal Administrativo de Contratos Públicos de la Comunidad Autónoma de Canarias"]
[3885177, "NL", null, null, 1, 1786785861, "Rechtbank Gelderland"]
[1209767, "NL", null, null, 1, 1786785861, "burgerlijke rechter te 's-Gravenhage"]
[7846, "ES", null, null, 1, 1786785861, "Tribunal Administrativo de Contratos Públicos de Aragón"]
[3169887, "NL", null, null, 1, 1786785861, "Rechtbank midden Nederland"]
[2168879, "ES", null, null, 1, 1786785861, "Tribunal Administrativo de Recursos Contractuales de Castilla y León"]
[3291033, "NL", null, null, 1, 1786785861, "Rechtbank Noord-Holland"]
[1164161, "NL", null, null, 1, 1786785861, "Rechtbank Amsterdam"]
[1174728, "NL", null, null, 1, 1786785861, "Rechtbank 's-Gravenhage"]
[3241594, "NL", null, null, 1, 1786785861, "Rechtbank Limburg"]
[2720295, "ES", null, null, 1, 1786785861, "Tribunal Administrativo Central de Recursos Contractuales (TACRC)"]
[3301449, "NL", null, null, 1, 1786785861, "Rechtbank Oost-Brabant"]
[18704536, "ES", null, null, 1, 1786785861, "Tribunal Administrativo Central de Recursos Contractuales - Secretaria"]
[3085354, "NL", null, null, 1, 1786785861, "Rechtbank Zeeland-West-Brabant"]
[4439929, "NL", null, null, 1, 1786785861, "Commissie van Aanbestedingsexperts"]
[11128836, "ES", null, null, 1, 1786785861, "Tribunal Administrativo Central de Recursos Contractuales – Secretaría"]
[6041, "ES", null, null, 1, 1786785861, "Tribunal Administrativo de Contratación Pública"]
[4087357, "ES", null, null, 1, 1786785861, "Tribunal Administrativo de Recursos Contractuales del Ayuntamiento de Málaga"]
[10951616, "ES", null, null, 1, 1786785861, "Tribunal Administrativo Central de Recursos Contractuales - Secretaría"]
[626, "ES", null, null, 1, 1786785861, "Tribunal Català de Contractes del Sector Públic"]
[3329871, "NL", null, null, 1, 1786785861, "Rechtbank Noord Nederland"]
[3154066, "ES", null, null, 1, 1786785861, "TARCYL"]
[1164189, "ES", null, null, 1, 1786785861, "Serveis Ferroviaris de Mallorca"]
[3601588, "ES", null, null, 1, 1786785861, "JIAESUR"]
[18371131, "ES", null, null, 1, 1786785861, "Área Contratación de MAZ M.C.S.S. Nº 11"]
[1215995, "PL", null, null, 1, 1786785861, "Regionalny Zarząd Gospodarki Wodnej we Wrocławiu"]
[21854128, "ES", null, null, 1, 1786785861, "Consejería de Economía, Hacienda e Innovación"]
[4644451, "ES", null, null, 1, 1786785861, "Servicio de Emergencias del Principado de Asturias"]
[31974977, "ES", null, null, 1, 1792841726, "En los lugares indicados en el artículo 51.3 de la Ley 9/2017, de 8 de noviembre, de Contratos del Sector Público"]
[1172306, "NL", null, null, 1, 1786785861, "Rechtbank Almelo"]
[13410817, "ES", null, null, 1, 1786785861, "Gerencia de Atención Sanitaria de Ceuta"]
[1168967, "DE", null, null, 1, 1786785861, "Regierung von Oberbayern, Vergabekammer Südbayern"]
[5751247, "ES", null, null, 1, 1786785861, "Tribunal Administrativo de Recursos Contractuales de la Diputación de Granada"]
[6866591, "ES", null, null, 1, 1786785861, "Tribunal Administrativo Recursos Contractuales"]
[19486371, "PL", null, null, 1, 1786785861, "Prezes Urzędu Zamówień Publicznych, Krajowa Izba Odwoławcza"]
[19467504, "ES", null, null, 1, 1786785861, "Tribunal Administrativo de Contratos Públicos de la CCAA de Madrid"]
[5109301, "ES", null, null, 1, 1786785861, "Ayuntamiento de Buñol"]
```


### #10 `s10_exact` — 2026-10-24T12:57:16.857340Z, 0.38 s, ok=True, rows=20, truncated=False

```sql
WITH probe(x, n, c) AS (VALUES (31975008, 'principal', 'ES'),(31975019, 'sede tibidabo', 'ES'),(31975031, 'stadt fürth - zentrale vergabestelle', 'DE'),(31975696, 'directeur strategische inkoop amsterdam umc', 'NL'),(31975409, 'aena. dirección de contratación', 'ES'),(31974991, 'dirección principal', 'ES'),(31975180, 'info adicional', 'ES'),(31975040, 'financiën en inkoop', 'NL'),(31975009, 'recursos', 'ES'),(31975370, 'dirección general de renfe fabricación y mantenimiento sociedad mercantil estatal, s.a', 'ES'),(31976779, 'registro general ayuntamiento de sevilla (recurso resposicion)', 'ES'),(31976976, 'comandancia de la guardia civil de cádiz', 'ES'),(31977284, 'gerencia territorial sepad de badajoz', 'ES'),(31977220, 'juzgados civiles de barcelona', 'ES'),(31977379, 'consejeria economia, hacienda innovación', 'ES'),(31975668, 'sede fundació mallorca turisme', 'ES'),(31976882, 'sede electrónica registro ministerio de política territorial/ tribunal administrativo central', 'ES'),(31976972, 'recurso especial en materia de contratación ante el organo de contratación', 'ES'),(31975111, '„merydian” brokerski dom ubezpieczeniowy s.a. - pełnomocnik zamawiającego', 'PL'),(31977009, 'olías del rey', 'ES')) SELECT p.x, p.n, p.c, (SELECT count(*) FROM organizations o WHERE o.name_norm = p.n AND +o.id < 31974972) AS pre_same_name, (SELECT count(*) FROM organizations o WHERE o.name_norm = p.n AND +o.id < 31974972 AND o.country = p.c) AS pre_same_name_cc, (SELECT count(*) FROM organizations o WHERE o.name_norm = p.n AND +o.id < 31974972 AND o.country = p.c AND o.identifier IS NOT NULL) AS pre_same_name_cc_identified, (SELECT min(o.id) FROM organizations o WHERE o.name_norm = p.n AND +o.id < 31974972) AS pre_min_id FROM probe p
```

Local plan, `plan/plan-copy.db` (byte copy of 577-triage/plan-copy.db, sha256 80391c50…):
```
SCAN probe AS p
CORRELATED SCALAR SUBQUERY 1
  SCAN organizations AS o
CORRELATED SCALAR SUBQUERY 2
  SCAN organizations AS o
CORRELATED SCALAR SUBQUERY 3
  SCAN organizations AS o
CORRELATED SCALAR SUBQUERY 4
  SCAN organizations AS o
```

Local plan, `plan/plan-idx.db` (same copy + prod's deferred organization indexes):
```
SCAN probe AS p
CORRELATED SCALAR SUBQUERY 1
  SEARCH o USING INDEX organizations_name_country (name_norm=?)
CORRELATED SCALAR SUBQUERY 2
  SEARCH o USING INDEX organizations_name_country (name_norm=? AND country=?)
CORRELATED SCALAR SUBQUERY 3
  SEARCH o USING INDEX organizations_name_country (name_norm=? AND country=?)
CORRELATED SCALAR SUBQUERY 4
  SEARCH o USING INDEX organizations_name_norm_id (name_norm=?)
```

Answer (columns x, n, c, pre_same_name, pre_same_name_cc, pre_same_name_cc_identified, pre_min_id):
```
[31975008, "principal", "ES", 3, 0, 0, 2211810]
[31975019, "sede tibidabo", "ES", 0, 0, 0, null]
[31975031, "stadt fürth - zentrale vergabestelle", "DE", 0, 0, 0, null]
[31975696, "directeur strategische inkoop amsterdam umc", "NL", 0, 0, 0, null]
[31975409, "aena. dirección de contratación", "ES", 1, 1, 1, 6745]
[31974991, "dirección principal", "ES", 0, 0, 0, null]
[31975180, "info adicional", "ES", 0, 0, 0, null]
[31975040, "financiën en inkoop", "NL", 0, 0, 0, null]
[31975009, "recursos", "ES", 0, 0, 0, null]
[31975370, "dirección general de renfe fabricación y mantenimiento sociedad mercantil estatal, s.a", "ES", 0, 0, 0, null]
[31976779, "registro general ayuntamiento de sevilla (recurso resposicion)", "ES", 0, 0, 0, null]
[31976976, "comandancia de la guardia civil de cádiz", "ES", 0, 0, 0, null]
[31977284, "gerencia territorial sepad de badajoz", "ES", 0, 0, 0, null]
[31977220, "juzgados civiles de barcelona", "ES", 0, 0, 0, null]
[31977379, "consejeria economia, hacienda innovación", "ES", 0, 0, 0, null]
[31975668, "sede fundació mallorca turisme", "ES", 0, 0, 0, null]
[31976882, "sede electrónica registro ministerio de política territorial/ tribunal administrativo central", "ES", 0, 0, 0, null]
[31976972, "recurso especial en materia de contratación ante el organo de contratación", "ES", 0, 0, 0, null]
[31975111, "„merydian” brokerski dom ubezpieczeniowy s.a. - pełnomocnik zamawiającego", "PL", 0, 0, 0, null]
[31977009, "olías del rey", "ES", 0, 0, 0, null]
```


### #11 `s11_prefix` — 2026-10-24T12:57:59.152442Z, 0.5 s, ok=True, rows=12, truncated=False

```sql
WITH probe(k, a, b) AS (VALUES ('tacrc', 'tribunal administrativo central de re', 'tribunal administrativo central de rf'),('andalucia', 'tribunal administrativo de recursos contractuales de la junta de andaluc', 'tribunal administrativo de recursos contractuales de la junta de andalud'),('aragon', 'tribunal administrativo de contratos públicos de arag', 'tribunal administrativo de contratos públicos de arah'),('catalan', 'tribunal català de contractes del sector p', 'tribunal català de contractes del sector q'),('madrid2261', 'tribunal administrativo de contratación pública de la comunidad de madrid', 'tribunal administrativo de contratación pública de la comunidad de madrie'),('malaga_dip', 'tribunal administrativo de recursos contractuales de la diputación provincial de m', 'tribunal administrativo de recursos contractuales de la diputación provincial de n'),('canarias', 'tribunal administrativo de contratos públicos de la comunidad autónoma de canarias', 'tribunal administrativo de contratos públicos de la comunidad autónoma de canariat'),('rb_oostbrabant', 'rechtbank oost-brabant', 'rechtbank oost-braban~'),('kio', 'krajowa izba odwo', 'krajowa izba odwp'),('fuerth', 'stadt fürth', 'stadt fürti'),('aena', 'aena', 'aenb'),('principal', 'principal', 'principam')) SELECT p.k, (SELECT count(*) FROM organizations o WHERE o.name_norm >= p.a AND o.name_norm < p.b AND +o.id < 31974972) AS pre_rows, (SELECT count(*) FROM organizations o WHERE o.name_norm >= p.a AND o.name_norm < p.b AND +o.id < 31974972 AND +o.identifier IS NULL) AS pre_identless, (SELECT count(*) FROM organizations o WHERE o.name_norm >= p.a AND o.name_norm < p.b AND +o.id >= 31974972) AS new_rows FROM probe p
```

Local plan, `plan/plan-copy.db` (byte copy of 577-triage/plan-copy.db, sha256 80391c50…):
```
SCAN probe AS p
CORRELATED SCALAR SUBQUERY 1
  SCAN organizations AS o
CORRELATED SCALAR SUBQUERY 2
  SCAN organizations AS o
CORRELATED SCALAR SUBQUERY 3
  SCAN organizations AS o
```

Local plan, `plan/plan-idx.db` (same copy + prod's deferred organization indexes):
```
SCAN probe AS p
CORRELATED SCALAR SUBQUERY 1
  SEARCH o USING INDEX organizations_name_country (name_norm>=? AND name_norm<?)
CORRELATED SCALAR SUBQUERY 2
  SEARCH o USING INDEX organizations_name_country (name_norm>=? AND name_norm<?)
CORRELATED SCALAR SUBQUERY 3
  SEARCH o USING INDEX organizations_name_country (name_norm>=? AND name_norm<?)
```

Answer (columns k, pre_rows, pre_identless, new_rows):
```
["tacrc", 477, 436, 51]
["andalucia", 60, 40, 5]
["aragon", 36, 36, 4]
["catalan", 50, 23, 2]
["madrid2261", 77, 57, 2]
["malaga_dip", 0, 0, 1]
["canarias", 18, 17, 3]
["rb_oostbrabant", 28, 24, 2]
["kio", 1039, 778, 4]
["fuerth", 129, 124, 1]
["aena", 206, 181, 1]
["principal", 62, 52, 1]
```


### #12 `s12_mrowid_coarse` — 2026-10-24T12:58:50.889848Z, 0.39 s, ok=True, rows=16, truncated=False

```sql
WITH probe(x) AS (VALUES (40000000),(60000000),(80000000),(100000000),(120000000),(140000000),(160000000),(180000000),(200000000),(225000000),(250000000),(300000000),(400000000),(500000000),(750000000),(1000000000)) SELECT p.x, (SELECT m.rowid FROM organization_mentions m WHERE m.rowid >= p.x ORDER BY m.rowid LIMIT 1) AS rid, (SELECT m.section_id FROM organization_mentions m WHERE m.rowid >= p.x ORDER BY m.rowid LIMIT 1) AS section_id, (SELECT m.notice_id FROM organization_mentions m WHERE m.rowid >= p.x ORDER BY m.rowid LIMIT 1) AS notice_id FROM probe p
```

Local plan, `plan/plan-copy.db` (byte copy of 577-triage/plan-copy.db, sha256 80391c50…):
```
SCAN probe AS p
CORRELATED SCALAR SUBQUERY 1
  SEARCH m USING INTEGER PRIMARY KEY (rowid=?)
CORRELATED SCALAR SUBQUERY 2
  SEARCH m USING INTEGER PRIMARY KEY (rowid=?)
CORRELATED SCALAR SUBQUERY 3
  SEARCH m USING INTEGER PRIMARY KEY (rowid=?)
```

Local plan, `plan/plan-idx.db` (same copy + prod's deferred organization indexes):
```
SCAN probe AS p
CORRELATED SCALAR SUBQUERY 1
  SEARCH m USING INTEGER PRIMARY KEY (rowid=?)
CORRELATED SCALAR SUBQUERY 2
  SEARCH m USING INTEGER PRIMARY KEY (rowid=?)
CORRELATED SCALAR SUBQUERY 3
  SEARCH m USING INTEGER PRIMARY KEY (rowid=?)
```

Answer (columns x, rid, section_id, notice_id):
```
[40000000, 40000000, "ND-ContractingParty#0", 26581185]
[60000000, null, null, null]
[80000000, null, null, null]
[100000000, null, null, null]
[120000000, null, null, null]
[140000000, null, null, null]
[160000000, null, null, null]
[180000000, null, null, null]
[200000000, null, null, null]
[225000000, null, null, null]
[250000000, null, null, null]
[300000000, null, null, null]
[400000000, null, null, null]
[500000000, null, null, null]
[750000000, null, null, null]
[1000000000, null, null, null]
```


### #13 `s13_mrowid_fine` — 2026-10-24T12:59:01.423226Z, 0.38 s, ok=True, rows=80, truncated=False

```sql
WITH probe(x) AS (VALUES (40250000),(40500000),(40750000),(41000000),(41250000),(41500000),(41750000),(42000000),(42250000),(42500000),(42750000),(43000000),(43250000),(43500000),(43750000),(44000000),(44250000),(44500000),(44750000),(45000000),(45250000),(45500000),(45750000),(46000000),(46250000),(46500000),(46750000),(47000000),(47250000),(47500000),(47750000),(48000000),(48250000),(48500000),(48750000),(49000000),(49250000),(49500000),(49750000),(50000000),(50250000),(50500000),(50750000),(51000000),(51250000),(51500000),(51750000),(52000000),(52250000),(52500000),(52750000),(53000000),(53250000),(53500000),(53750000),(54000000),(54250000),(54500000),(54750000),(55000000),(55250000),(55500000),(55750000),(56000000),(56250000),(56500000),(56750000),(57000000),(57250000),(57500000),(57750000),(58000000),(58250000),(58500000),(58750000),(59000000),(59250000),(59500000),(59750000),(60000000)) SELECT p.x, (SELECT m.rowid FROM organization_mentions m WHERE m.rowid >= p.x ORDER BY m.rowid LIMIT 1) AS rid, (SELECT m.section_id FROM organization_mentions m WHERE m.rowid >= p.x ORDER BY m.rowid LIMIT 1) AS section_id, (SELECT m.notice_id FROM organization_mentions m WHERE m.rowid >= p.x ORDER BY m.rowid LIMIT 1) AS notice_id, (SELECT m.organization_id FROM organization_mentions m WHERE m.rowid >= p.x ORDER BY m.rowid LIMIT 1) AS org FROM probe p
```

Local plan, `plan/plan-copy.db` (byte copy of 577-triage/plan-copy.db, sha256 80391c50…):
```
SCAN probe AS p
CORRELATED SCALAR SUBQUERY 1
  SEARCH m USING INTEGER PRIMARY KEY (rowid=?)
CORRELATED SCALAR SUBQUERY 2
  SEARCH m USING INTEGER PRIMARY KEY (rowid=?)
CORRELATED SCALAR SUBQUERY 3
  SEARCH m USING INTEGER PRIMARY KEY (rowid=?)
CORRELATED SCALAR SUBQUERY 4
  SEARCH m USING INTEGER PRIMARY KEY (rowid=?)
```

Local plan, `plan/plan-idx.db` (same copy + prod's deferred organization indexes):
```
SCAN probe AS p
CORRELATED SCALAR SUBQUERY 1
  SEARCH m USING INTEGER PRIMARY KEY (rowid=?)
CORRELATED SCALAR SUBQUERY 2
  SEARCH m USING INTEGER PRIMARY KEY (rowid=?)
CORRELATED SCALAR SUBQUERY 3
  SEARCH m USING INTEGER PRIMARY KEY (rowid=?)
CORRELATED SCALAR SUBQUERY 4
  SEARCH m USING INTEGER PRIMARY KEY (rowid=?)
```

Answer (80 rows; the rows around each change shown, full body in box/raw/13-s13_mrowid_fine.json; columns x, rid, section_id, notice_id, org):
```
[40250000, 40250000, "ORG-0004", 26705708, 23180982]
[50250000, 50250000, "ORG-3", 1979542, 24983803]
[50500000, 50500000, "TPO-0001", 24253793, 5523485]
[50750000, null, null, null, null]
[60000000, null, null, null, null]
```


### #14 `s14_mrowid_finer` — 2026-10-24T12:59:20.457420Z, 0.42 s, ok=True, rows=80, truncated=False

```sql
WITH probe(x) AS (VALUES (50256250),(50262500),(50268750),(50275000),(50281250),(50287500),(50293750),(50300000),(50306250),(50312500),(50318750),(50325000),(50331250),(50337500),(50343750),(50350000),(50356250),(50362500),(50368750),(50375000),(50381250),(50387500),(50393750),(50400000),(50406250),(50412500),(50418750),(50425000),(50431250),(50437500),(50443750),(50450000),(50456250),(50462500),(50468750),(50475000),(50481250),(50487500),(50493750),(50500000),(50506250),(50512500),(50518750),(50525000),(50531250),(50537500),(50543750),(50550000),(50556250),(50562500),(50568750),(50575000),(50581250),(50587500),(50593750),(50600000),(50606250),(50612500),(50618750),(50625000),(50631250),(50637500),(50643750),(50650000),(50656250),(50662500),(50668750),(50675000),(50681250),(50687500),(50693750),(50700000),(50706250),(50712500),(50718750),(50725000),(50731250),(50737500),(50743750),(50750000)) SELECT p.x, (SELECT m.rowid FROM organization_mentions m WHERE m.rowid >= p.x ORDER BY m.rowid LIMIT 1) AS rid, (SELECT m.section_id FROM organization_mentions m WHERE m.rowid >= p.x ORDER BY m.rowid LIMIT 1) AS section_id, (SELECT m.notice_id FROM organization_mentions m WHERE m.rowid >= p.x ORDER BY m.rowid LIMIT 1) AS notice_id, (SELECT m.organization_id FROM organization_mentions m WHERE m.rowid >= p.x ORDER BY m.rowid LIMIT 1) AS org FROM probe p
```

Local plan, `plan/plan-copy.db` (byte copy of 577-triage/plan-copy.db, sha256 80391c50…):
```
SCAN probe AS p
CORRELATED SCALAR SUBQUERY 1
  SEARCH m USING INTEGER PRIMARY KEY (rowid=?)
CORRELATED SCALAR SUBQUERY 2
  SEARCH m USING INTEGER PRIMARY KEY (rowid=?)
CORRELATED SCALAR SUBQUERY 3
  SEARCH m USING INTEGER PRIMARY KEY (rowid=?)
CORRELATED SCALAR SUBQUERY 4
  SEARCH m USING INTEGER PRIMARY KEY (rowid=?)
```

Local plan, `plan/plan-idx.db` (same copy + prod's deferred organization indexes):
```
SCAN probe AS p
CORRELATED SCALAR SUBQUERY 1
  SEARCH m USING INTEGER PRIMARY KEY (rowid=?)
CORRELATED SCALAR SUBQUERY 2
  SEARCH m USING INTEGER PRIMARY KEY (rowid=?)
CORRELATED SCALAR SUBQUERY 3
  SEARCH m USING INTEGER PRIMARY KEY (rowid=?)
CORRELATED SCALAR SUBQUERY 4
  SEARCH m USING INTEGER PRIMARY KEY (rowid=?)
```

Answer (80 rows; the rows around each change shown, full body in box/raw/14-s14_mrowid_finer.json; columns x, rid, section_id, notice_id, org):
```
[50256250, 50256250, "ORG-3", 1780188, 4180625]
[50437500, 50437500, "ORG-GB-COH-01846493", 47435924, 9520549]
[50443750, 50443750, "TPO-0003", 23594707, 1209767]
[50668750, 50668750, "TPO-0001", 47399469, 1166464]
[50675000, null, null, null, null]
[50750000, null, null, null, null]
```


### #15 `s15_mrowid_edges` — 2026-10-24T12:59:45.584741Z, 0.37 s, ok=True, rows=100, truncated=False

```sql
WITH probe(x) AS (VALUES (50437625),(50437750),(50437875),(50438000),(50438125),(50438250),(50438375),(50438500),(50438625),(50438750),(50438875),(50439000),(50439125),(50439250),(50439375),(50439500),(50439625),(50439750),(50439875),(50440000),(50440125),(50440250),(50440375),(50440500),(50440625),(50440750),(50440875),(50441000),(50441125),(50441250),(50441375),(50441500),(50441625),(50441750),(50441875),(50442000),(50442125),(50442250),(50442375),(50442500),(50442625),(50442750),(50442875),(50443000),(50443125),(50443250),(50443375),(50443500),(50443625),(50443750),(50668875),(50669000),(50669125),(50669250),(50669375),(50669500),(50669625),(50669750),(50669875),(50670000),(50670125),(50670250),(50670375),(50670500),(50670625),(50670750),(50670875),(50671000),(50671125),(50671250),(50671375),(50671500),(50671625),(50671750),(50671875),(50672000),(50672125),(50672250),(50672375),(50672500),(50672625),(50672750),(50672875),(50673000),(50673125),(50673250),(50673375),(50673500),(50673625),(50673750),(50673875),(50674000),(50674125),(50674250),(50674375),(50674500),(50674625),(50674750),(50674875),(50675000)) SELECT p.x, (SELECT m.rowid FROM organization_mentions m WHERE m.rowid >= p.x ORDER BY m.rowid LIMIT 1) AS rid, (SELECT m.section_id FROM organization_mentions m WHERE m.rowid >= p.x ORDER BY m.rowid LIMIT 1) AS section_id, (SELECT m.notice_id FROM organization_mentions m WHERE m.rowid >= p.x ORDER BY m.rowid LIMIT 1) AS notice_id FROM probe p
```

Local plan, `plan/plan-copy.db` (byte copy of 577-triage/plan-copy.db, sha256 80391c50…):
```
SCAN probe AS p
CORRELATED SCALAR SUBQUERY 1
  SEARCH m USING INTEGER PRIMARY KEY (rowid=?)
CORRELATED SCALAR SUBQUERY 2
  SEARCH m USING INTEGER PRIMARY KEY (rowid=?)
CORRELATED SCALAR SUBQUERY 3
  SEARCH m USING INTEGER PRIMARY KEY (rowid=?)
```

Local plan, `plan/plan-idx.db` (same copy + prod's deferred organization indexes):
```
SCAN probe AS p
CORRELATED SCALAR SUBQUERY 1
  SEARCH m USING INTEGER PRIMARY KEY (rowid=?)
CORRELATED SCALAR SUBQUERY 2
  SEARCH m USING INTEGER PRIMARY KEY (rowid=?)
CORRELATED SCALAR SUBQUERY 3
  SEARCH m USING INTEGER PRIMARY KEY (rowid=?)
```

Answer (100 rows; the rows around each change shown, full body in box/raw/15-s15_mrowid_edges.json; columns x, rid, section_id, notice_id):
```
[50437625, 50437625, "TPO-0003", 23466093]
[50670875, 50670875, "TPO-0001", 47431744]
[50671000, null, null, null]
[50675000, null, null, null]
```


### #16 `s16_exact_all` — 2026-10-24T13:00:16.437190Z, 1.26 s, ok=True, rows=3751, truncated=False

```sql
SELECT o.id, (SELECT count(*) FROM organizations e WHERE e.name_norm = o.name_norm AND +e.id < 31974972) AS pre_same_name, (SELECT count(*) FROM organizations e WHERE e.name_norm = o.name_norm AND e.country = o.country AND +e.id < 31974972) AS pre_same_name_cc, (SELECT count(*) FROM organizations e WHERE e.name_norm = o.name_norm AND +e.id < 31974972 AND +e.country IS NULL) AS pre_same_name_nocc, (SELECT min(e.id) FROM organizations e WHERE e.name_norm = o.name_norm AND +e.id < 31974972) AS pre_min_id FROM organizations o WHERE o.id BETWEEN 31974972 AND 31978722
```

Local plan, `plan/plan-copy.db` (byte copy of 577-triage/plan-copy.db, sha256 80391c50…):
```
SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
CORRELATED SCALAR SUBQUERY 1
  SCAN organizations AS e
CORRELATED SCALAR SUBQUERY 2
  SCAN organizations AS e
CORRELATED SCALAR SUBQUERY 3
  SCAN organizations AS e
CORRELATED SCALAR SUBQUERY 4
  SCAN organizations AS e
```

Local plan, `plan/plan-idx.db` (same copy + prod's deferred organization indexes):
```
SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
CORRELATED SCALAR SUBQUERY 1
  SEARCH e USING INDEX organizations_name_country (name_norm=?)
CORRELATED SCALAR SUBQUERY 2
  SEARCH e USING INDEX organizations_name_country (name_norm=? AND country=?)
CORRELATED SCALAR SUBQUERY 3
  SEARCH e USING INDEX organizations_name_country (name_norm=?)
CORRELATED SCALAR SUBQUERY 4
  SEARCH e USING INDEX organizations_name_norm_id (name_norm=?)
```

Answer: 3,751 rows (one per new org); summarized in 2c; raw: box/raw/16-s16_exact_all.json


### #17 `s17_hubs` — 2026-10-24T13:00:43.781002Z, 2.07 s, ok=True, rows=10, truncated=False

```sql
WITH probe(x) AS (VALUES (31975008),(31975019),(31974991),(31975180),(31975009),(31975006),(31975004),(31975568),(31975031),(31975696)) SELECT p.x, count(*) AS tpo_mentions, count(DISTINCT m.notice_id) AS notices, count(DISTINCT b.organization_id) AS parent_orgs, count(b.organization_id) AS with_parent_mention FROM probe p CROSS JOIN organization_mentions m ON m.organization_id = p.x CROSS JOIN notice_sections s ON s.notice_id = m.notice_id AND s.section_id = m.section_id LEFT JOIN organization_mentions b ON b.notice_id = s.notice_id AND b.section_id = s.parent_section_id GROUP BY +p.x
```

Local plan, `plan/plan-copy.db` (byte copy of 577-triage/plan-copy.db, sha256 80391c50…):
```
SCAN probe AS p
SEARCH m USING INDEX organization_mentions_org (organization_id=?)
SEARCH s USING INDEX sqlite_autoindex_notice_sections_1 (notice_id=? AND section_id=?)
SEARCH b USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=? AND section_id=?) LEFT-JOIN
USE SORTER FOR GROUP BY
USE HASH TABLE FOR count(DISTINCT)
USE HASH TABLE FOR count(DISTINCT)
```

Local plan, `plan/plan-idx.db` (same copy + prod's deferred organization indexes):
```
SCAN probe AS p
SEARCH m USING INDEX organization_mentions_org (organization_id=?)
SEARCH s USING INDEX sqlite_autoindex_notice_sections_1 (notice_id=? AND section_id=?)
SEARCH b USING INDEX organization_mentions_notice (notice_id=?) LEFT-JOIN
USE SORTER FOR GROUP BY
USE HASH TABLE FOR count(DISTINCT)
USE HASH TABLE FOR count(DISTINCT)
```

Answer (columns x, tpo_mentions, notices, parent_orgs, with_parent_mention):
```
[31974991, 420, 420, 18, 420]
[31975004, 143, 143, 1, 143]
[31975006, 221, 220, 9, 221]
[31975008, 715, 715, 6, 715]
[31975009, 268, 268, 1, 268]
[31975019, 621, 621, 1, 621]
[31975031, 556, 556, 2, 556]
[31975180, 294, 294, 1, 294]
[31975568, 105, 105, 1, 105]
[31975696, 513, 236, 1, 513]
```


### #18 `s18_tacrc_tsj` — 2026-10-24T13:01:35.360717Z, 0.38 s, ok=True, rows=1, truncated=False

```sql
SELECT count(*) AS mentions, sum(m.section_id LIKE 'TPO-%') AS tpo_mentions, count(DISTINCT m.notice_id) AS notices FROM organization_mentions m WHERE m.organization_id = 22077939
```

Local plan, `plan/plan-copy.db` (byte copy of 577-triage/plan-copy.db, sha256 80391c50…):
```
SEARCH m USING INDEX organization_mentions_org (organization_id=?)
USE HASH TABLE FOR count(DISTINCT)
```

Local plan, `plan/plan-idx.db` (same copy + prod's deferred organization indexes):
```
SEARCH m USING INDEX organization_mentions_org (organization_id=?)
USE HASH TABLE FOR count(DISTINCT)
```

Answer (columns mentions, tpo_mentions, notices):
```
[2534, 2531, 2534]
```

