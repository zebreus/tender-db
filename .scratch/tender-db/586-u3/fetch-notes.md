# 586 unit 3 / 576 unit 5: Verify reads, salted 30 minted (576-u5-) and salted 30 aliased (586-u3-) — fetch notes (2026-10-24, 18:11–18:26Z)

Outputs, all in this directory:
- `verify.json`: the Verify reads A1–A4, plus three extra reads (A2's other hub orgs, A5 the two TenderNed label orgs now,
  A6 576 U5's buyer witness 4592524).
- `minted.json`: 30 salted MINTED touchpoint parties now served (B01–B30).
- `aliased.json`: 30 salted ALIASED touchpoint roles that 586 created (C01–C30).
- `picks.json`: the pools and the pick, with every candidate's rank.
- `raw/`: every body in full (`tender/`, `org/`, `content/`, `extra/`, `sql/` with each statement's `.sql`, `.plan`, `.out`),
  `gets.log` (every GET), `fetch.log`, `state/sql_state.json` (the /v1/sql send log).
- `tools/`: sq.py (plan-probe + send), get.sh (serial GET), win.py (windows), classify.py (586-u1/final.py's decision),
  pick.py, fetch.py, build.py, verify.py, notes_sql.py. `sqlsrc/`: the statements as sent and each window's salt metadata.

Read-only. No POST, no admin endpoint, no git, no cargo, no box write. Writes only under `.scratch/tender-db/586-u3/`.
No verdicts here: each item carries mechanical checks only. The R/N/W rubric is the judges' step.

## Budget used

- **Public API:** 193 GETs, strictly serial through one flock'd helper (≥ 2.2 s from one response's end to the next
  request's start). The smallest gap between two request starts was 2.62 s. 181 answered 200 and 12 answered 404, all
  12 Verify organizations (below).
- **/v1/sql:** 18 of 20 statements, sent ≥ 3.5 s apart. Every answer was JSON with rows and columns, truncated=false,
  taking 0.37–2.03 s. No 408, no shed, no retry.
- **Plans:** every statement was planned locally with turso `plan-probe` (sha256 50a8094e…, the binary 576 U4 used) on two
  copies:
  1. `plan/plan-copy.db`, a byte copy of `577-triage/plan-copy.db` (sha256 80391c50…, identical);
  2. `plan/plan-idx.db`, the same schema plus prod's deferred indexes (`organization_mentions_notice`,
     `tender_version_parties_mention_key` / `_org_role`, `organizations_*`), copied from 576 U4's `plan_prod.db`.

  The original `577-triage/plan-copy.db` was never opened by this run. The `plan-copy.db-shm` / `-wal` that appeared
  beside it at 18:13:07Z came from a parallel helper, `519-m6/tools/sq.py`, which plans on that file directly.

  The helper refuses any step that is not SEARCH, a sorter, a hash table or a compound marker. It refused one draft,
  which is recorded under "Statements" below. Every statement sent planned as PK or index SEARCH on both copies.

## Verify (A)

| # | read | expected | got | result |
|---|---|---|---|---|
| A1 | GET /v1/tenders/540072 (v4), Procedure-Buyer + Lot-ReviewOrg org ids | `[22198704]` | `[22198704]`: Procedure-Buyer plus Lot-ReviewOrg on LOT-0001…0004, every one 22198704, mention_name "Sociedad Aragonesa de Gestión Agroambiental, S.L.U. (SARGA)" | **PASS** |
| A2 | GET /v1/organizations/31974991 ("Dirección Principal") | mentions 0 or 404 | **404** "no such organization" | **PASS**; **sweep 2772 has run**, because retire_mentions never deletes an org |
| A3 | GET /v1/tenders/346591 (v1) and /361790 (v5): no party on 18044191 / 18044192 in a Lot-* role | none | none on either org, in any role. Both now serve Mediator 20882145 "Klachtenmeldpunt Rijksvastgoedbedrijf" and ReviewInfo + ReviewOrg 1209767 "Burgerlijke Rechter te 's-Gravenhage" (TPO parties) | **PASS** |
| A4 | statement v01_labels: tender_version_parties by organization_id = 18044191 / 18044192, all versions | only legacy further-information / tender-receipt, no Lot-* | 18044191: `further-information` 193 rows (101 on current versions), tenders 7,226,068–8,529,842, sections ORG-2/ORG-3. 18044192: `tender-receipt` 175 (83 current), same tender range, ORG-3. **0 Lot-* rows** (before the refold: 1,032 Lot-AddInfo and 1,076 Lot-TenderReceipt, from `576-u4/u5_buyer_cohort.json`) | **PASS** |

Extra reads, beyond what the task asked:
- **A2, the other hub and self-duplicate orgs** in decision.md's unit-4 list: 31975008 "Principal", 31975059 "Palacio
  Provincial", 31975006, 31975019, 31975180, 31975009, 31975004, 31975568, 31975060 and 31975949. All **404**, as is
  31975096, the old SARGA duplicate "Sociedad Aragonesa de Gestión Agroambiental S.L.U" (127 mentions while open).
- **A1, the SQL side** (statement x540072). Each Lot-ReviewOrg row is anchored at ORG-0001 of a different notice:
  LOT-0001 at 23563650, LOT-0002 at 23589619, LOT-0003 at 23576127 and LOT-0004 at 23658810. In each of those notices the
  lot's `OPT-301-Lot-ReviewOrg` names TPO-0001, "Sociedad Aragonesa de Gestión Agroambiental S.L.U", under ORG-0001
  "…, S.L.U. (SARGA)". final.py puts the pair in tier A. s_mentions_c shows that notice 23563650 has no TPO-0001 mention,
  only ORG-0001 → 22198704 and ORG-0002 → 8173536.
- **A5, the label orgs now.** GET /v1/organizations/18044191 gives mentions 101 and 18044192 gives 83. Both still exist,
  and the counts equal A4's current-version legacy rows: only the ORG-path rows of issue 585 remain.
- **A6, 576 U5's buyer witness** (refold-notices 2766/2767). GET /v1/tenders/4592524 v3 serves `buyer` 31484168 "Raad van
  State, Algemene Rekenkamer en het Kabinet der Koningin p/a Het NIC BV". The sentence org 31431409 is no longer the
  buyer, but it is still served as `further-information`. That is a legacy ORG-path row, issue 585's class.

## Windows

The tender-id windows reuse 576 U4's mapping: the concatenation of A = [1, 1,210,000] and B = [8,530,000, 8,869,999],
L = 1,550,000. The start is s = int(sha256(SALT + str(i))[:16], 16) mod L, and a = 1 + s if s < 1,210,000, else
8,530,000 + (s − 1,210,000). The task wrote the first block as "~1.0–1.21M". 576 U4's tender probes show eForms TED from
id ~1 (50k, 250k, 500k, 750k and 1.0M are all eForms), so the whole of U4's segment A was used. Segment B interleaves fts
(UK) tenders, which carry no touchpoints, so windows there can come back empty.

## B: sampling as run (salt `576-u5-`)

- **General windows** (statement = 576 U4's `s01` shape; W = 3,000; LIMIT 40 groups, grouped by (tender, seq, role, org,
  notice, section); current versions only; `p.mention_section_id LIKE 'TPO-%'`):
  - m0 (8,754,149) and m5 (8,738,136) returned 0 rows (segment B, fts).
  - m1 (906,786), m2 (327,124), m3 (8,563,228) and m4 (8,828,668) returned 40 each.
- **NL AddInfo/TenderReceipt window** n6 (166,370..186,370, W = 20,000), as U4's s07: 17 rows, all NLD buyers. None is a
  TenderNed "deze aanbesteding" label.
- **Other-roles window** o8 (839,591..859,591, W = 20,000, `+p.role NOT IN ('Lot-ReviewOrg','Lot-ReviewInfo')`): 40 rows.
  This window is new against U4. It was added because the general windows were 76% ReviewOrg, and the task asks for the
  rest to be spread.
- **Pool:** 217 rows, all distinct keys.
  - By role: ReviewOrg 117, ReviewInfo 36, AddInfo 22, Mediator 17, TenderReceipt 9, DocProvider 6, TenderEval 4, and
    Employ/Environ/FiscalLegis 2 each.
  - By org country: ES 113, NL 62, FR 17, PL 15, DE 7, IT, PT and MK 1 each.
  - 19 rows have no own BT-514. 19 rows sit on orgs with id ≥ 31,974,972 (minted since 576 U3).
- **Pick** (`tools/pick.py`, deterministic). Rank = sha256('576-u5-pick-' + 'tender|seq|role|org|notice|section').
  - Caps: ≤ 2 per tender, ≤ 1 per organization_id, ≤ 1 per touchpoint section.
  - Strata, as in U4: NL AddInfo/TenderReceipt 6; no own BT-514 3; Lot-ReviewOrg 6; then the rest 15, round-robin over
    roles (alphabetical, ReviewOrg last), taking the least-used country each turn.
- **Result (30 items, 29 tenders, 30 orgs).**
  - NL AddInfo/TenderReceipt: 7 (B01–B06, plus B30 from the rest).
  - Lot-ReviewOrg: 8. Country fallback: 9 (B07, B08, B09, B18, B20, B25, B27, B28, B29).
  - By role: ReviewOrg 8, AddInfo 7, TenderReceipt 3, DocProvider 3, ReviewInfo 3, Mediator 2, TenderEval 2,
    EmployLegis 1, EnvironLegis 1.
  - By org country: NL 8, ES 6, PL 6, FR 5, DE 3, IT 1, MK 1. 6 items are on orgs minted since 576 U3.

## C: sampling as run (salt `586-u3-`)

- **Statement** (`tools/win.py`, kind a; W = 3,000; LIMIT 60 groups):
  - The joins are tenders `t` (id range) CROSS JOIN tender_version_parties `p` (current seq), CROSS JOIN lots `l` (PK on
    p.lot_id), CROSS JOIN notice_ids `i` (PK seek on (notice_id = p.mention_notice_id, section_id = l.lot_key,
    field_id = 'OPT-301-' || p.role), with `+i.value LIKE 'TPO-%'`), CROSS JOIN notice_sections `s` (PK on the TPO), and
    CROSS JOIN notice_texts twice (PK: the TPO's BT-500-Organization-TouchPoint ordinal 0 and the anchor ORG's
    BT-500-Organization-Company ordinal 0).
  - Filters: `p.mention_section_id LIKE 'ORG-%'`, role in the 10 touchpoint roles, `s.parent_section_id =
    p.mention_section_id`, `s.kind = 'Touchpoint'`, and `lower(tn.value) <> lower(pn.value)`.
  - In words, it keeps a party row anchored at an ORG section whose own lot's referencing field names a TPO under that
    same ORG, with a printed name that differs beyond case.
- **Windows:**
  - a0 (206,362): 29 rows. a1 (8,722,412, segment B): 0. a2 (89,967): 20. a3 (94,463): 38. a5 (816,222): 35.
    a6 (570,205): 19.
  - a4 (segment B) was not sent. Every window that answered rows returned fewer than its LIMIT, so each is complete.
  - Plus x540072, the same statement on tender 540072 alone: 4 rows, the SARGA exhibit.
- **Classification:** every row went through 586-u1/final.py `decision(tp_name, parent_name)`.
  - Pool of 145 groups: C1 102, A 16, C2 9, C4 7, norm 5, C3 3, C0 2, C3r 1.
  - The 5 `norm` rows are pre-586 same-name aliases (case-only, or a trailing ".") and were excluded: "Komenda Główna
    Policji" ×4 and "…Cultura O.A.". No row in the pool was decided `mint` or `drop`.
  - Country: the 30 picked orgs are all ES except C10 (IT). By name, the pool is Spanish bodies except the Italian C3r
    row and the Polish norm rows.
- **Pick.** Rank = sha256('586-u3-pick-' + 'tender|seq|role|org|notice|section|tpo'). Caps: ≤ 2 per tender, ≤ 1 per org,
  ≤ 1 per (notice, TPO). The strata ran in this order:
  1. SARGA 540072: 1 (C01).
  2. Tier A: 6 by rank, with roles round-robin.
  3. Tier C sub-tier quotas: C0 1, C3 1, C3r 1, C2 3, C4 4.
  4. C1 fills the rest: 13.
- **Result:**
  - **Tier A 7** (C01–C07): SARGA; TARJA, the tribunal under its own ORG; Manacor accent; AVRA "(PUNTO DE CONTACTO)";
    Thyssen with "F.S.P." and a hyphen; Consejería in capitals with accents; Jefatura with articles dropped.
  - **Tier C 23** (C08–C30): C0 1, C3 1, C3r 1, C2 3, C4 4, C1 13.
  - By role: ReviewOrg 16, ReviewInfo 10, Mediator 3, AddInfo 1.

## Per-item fetches

Each item got one GET of the tender (/v1/tenders/{id}), one of the organization (/v1/organizations/{id}) and one of the
mention notice's content (/v1/notices/{id}/content). Shared tenders and notices were fetched once. Each object in
`minted.json` / `aliased.json` keeps:
- the TPO section and its parent ORG section, all values (BT-500 in every language, BT-514, BT-16, city);
- the buyer ORG section(s), named by OPT-300-Procedure-Buyer;
- every field that references the TPO (`referencing_fields`), and the role's OPT-301 field on every lot
  (`role_field_by_lot`);
- the notice's other touchpoints;
- the served parties (all of them, plus the ones for this role and org), the tender's versions, the organization body;
- the organization_mentions rows at the TPO, parent and buyer keys (statements s_mentions_b / s_mentions_c);
- final.py's decision over every (TPO name, parent name) pair;
- `checks`.

## Checks over all 60 (mechanical, no verdicts)

- **Current version.** The tender GET's `version` equals the row's seq on 60/60.
- **Lots.** The served lots for (role, org) equal the party rows the window statements found for that (tender, seq, role,
  org) on 60/60 (`sql_rows_cover_served_lots`).
  - Three aliased items serve lots that their own notice does not reference: C01 (540072), C11 (206565) and C24
    (817901). Each is a multi-notice tender whose lots are anchored at different notices, and the SQL rows show that
    every anchoring notice names the TPO on that lot. C24's 22 ReviewInfo lots come from notices 561 (6), 25,747,014 (4),
    27,524,483 (10) and 29,922,117 (2).
  - **No party is served at a lot that its anchoring notice does not reference.**
- **B (minted), 30/30:**
  - the served mention_name equals the TPO's BT-500;
  - the TPO's own mention row is bound to the party's org;
  - final.py says `mint` on every name pair, so none should have aliased or met the D586-2 gate;
  - all orgs are provisional and identifier-less.
  - The 9 country-fallback items carry the parent ORG's BT-514 country.
  - The TPO's parent is the buyer on 28. B27 (KIO under the ORG "Urząd Zamówień Publicznych Krajowa Izba Odwoławcza")
    and B28 (under the ORG "Tribunal administratif de poitiers") sit under a non-buyer ORG.
- **C (aliased), 30/30:**
  - the TPO has NO organization_mentions row, which is the alias retirement;
  - the parent ORG mention is bound to the party's org;
  - the served mention_name is the parent's printed BT-500, the issue-456 side effect decision.md predicts;
  - no name pair is equal under `org_name_norm`, so every item is a 586 tier-A/C alias and not a pre-586 (i) alias;
  - every party org is identified (scheme national).
  - The party org is also the served Procedure-Buyer on 29/30, the "buyer as its own review body" serve decision.md
    names as intended. C02 is the exception: TARJA's touchpoint sits under its own ORG-0002, so it aliases to the
    tribunal 2660.
- **TenderNed labels.** No served name in either sample holds "deze aanbesteding" or "dashboard", and neither does any of
  the 17 n6 rows.

## Oddities and things the judges should look at

1. **B28, a tribunal self-duplicate with a raw newline.** TPO-0002 sits under the ORG "Tribunal administratif de poitiers"
   (mention → org 2644) and prints "Tribunal administratif de Poitiers\nInstance chargée de l'introduction des recours".
   That minted or bound 31975366, which has 38 mentions, is provisional and keeps the `\n` in both its org name and the
   served mention_name. This is tier B (parent + descriptor), which D586-1 leaves minting. The raw newline is the same
   class U4 noted (item 18).
2. **Generic department labels pooled across buyers** (older ORG-path rows now also bound by touchpoints):
   - B07 "Direction de la Commande Publique" → 1166638, 1,356 mentions (DocProvider for "Mairie du Bourget");
   - B14 "unidad de contratación" → 1223007 "Unidad de Contratación", 410 mentions, served as **Lot-ReviewOrg** for
     "Intendente de Rota";
   - B29 "Dział Zamówień Publicznych i Zaopatrzenia" → 9327844, 37 mentions.

   decision.md lists these as accepted residue ("Unidad de Contratación"; U4 judged a pooled "Afd. Inkoop" N). B14 also
   serves a contracting unit in the review role.
3. **Buyer self-duplicates under a variant** (still minted by design: acronym, variant or typo):
   - B01 "Stichting Quadraam" → 1327872 (27 mentions), beside buyer 4966589 "Quadraam Gelderse Onderwijsgroep";
   - B04 "IBMN" → 20951176 (129), as in U4 item 6;
   - B03 "Team Inkoop gemeente Westland" → 10359863 (527), as in U4 item 4.
4. **Persons and address-bearing names in AddInfo:**
   - B05 "Alpha Adviesbureau t.a.v. Martine Godlieb, Pieter Sneeuwplein 12, 9461 KA Veendam" → 13300018 (21): a
     consultancy plus "for the attention of" a person plus a street address, all in one name;
   - B06 "P.D. van Buuren" → 17378824 (19): a bare personal name, accepted residue;
   - B30 "DAS makkelijk" (TenderReceipt, same notice as B06).
5. **Branches and departments of the buyer under their own name** (tier B, N by D576-1):
   - B18 "Centrum Usług Logistycznych - centrala w Warszawie", in the Employ/Environ/FiscalLegis roles, without a country;
   - B21 "Deutsche Bundesbank, Leiter des Zentralbereichs Beschaffungen", ReviewInfo;
   - B16, B23 and B26, the regional branches of Wody Polskie and AMW;
   - B25 "Bureau des achats immobiliers et prestations (BAIP)".
6. **B08** "Tribunal Arbitral de Barcelona (TAB)" (31976119, minted since 576 U3, 62 mentions) is the ReviewOrg for the
   "Secretariat of the Union for the Mediterranean". It is an arbitral body in the review role, and its country ES comes
   from the parent's BT-514.
7. **C, the served name is now the parent's** (decision.md's issue-456 side effect, N). C09 prints "Malpartida de Cáceres"
   and serves mention_name "Pleno del Ayuntamiento de Malpartida de Cáceres". C16 prints a bare "Ayuntamiento" (C4) and
   serves "Alcaldia del Ayuntamiento de Fregenal de la Sierra".
   - Several identified buyer orgs carry an organ as their canonical name: 22766649 "Pleno del Ayuntamiento de Malpartida
     de Cáceres", 22380833 "Junta de Gobierno Local de l'Ajuntament d'Artà", 22701226 "Alcaldia del Ayuntamiento de
     Fregenal…". So the review role now names the organ.
   - The org names predate 586; the touchpoint printed the entity.
8. **C, an organ cut over a non-governing body.**
   - C28: "Mutua Montañesa" under "Dirección General de Mutua Montañesa" (C1, cutting `direccion general`).
   - C25: "Palacio de Congresos de Valencia" under "Gerencia del Palacio de Congresos de Valencia" (cutting `gerencia`).
   - Pool rows not picked: "Aguas de Lorca" under "Dirección Gerente de Aguas de Lorca, S.A."; "Spet, Turismo de
     Tenerife" under "Consejo de Administración de Spet, Turismo de Tenerife, S.A."; "Suma Gestión Tributaria Diputación
     de Alicante" under "Presidencia de Suma Gestión Tributaria. Diputación de Alicante".

   Each aliases the entity to an org whose name is its management organ. Judges should confirm these read as "the body
   under its organ" and not "a department".
9. **C24's notice id 561.** Tender 817901 v2 is caused by notice id **561** (publication 00497339-2026, a 2026 eForms
   notice). It sits beside notice ids in the 25–30M range, which suggests notice-id reuse. This is outside 576/586 and
   only noted.
10. **C02, a tier-A alias onto a tribunal.** The TARJA touchpoint "…DE RECURSOS CONTRACTUALES DE LA JUNTA DE ANDALUCÍA"
    sits under ORG-0002 "…DE LOS RECURSOS CONTRACTUALES DE LA JUNTA…" and aliases to 2660. The parent is the tribunal,
    not the buyer, as decision.md expects ("the alias lands on the tribunal").
11. **576 U5's buyer witness 4592524** (A6) still serves two other legacy prose parties: `specifications-provider`
    31378371 "Een ondernemer dient zich eerst te registreren op het aanbestedingsplatform…" and `tender-receipt` 1163085
    "Zie de uitnodigingsbrief…". They are ORG-path legacy rows (issue 585's class) and are not touched by U5.
12. **The TenderNed label orgs still exist** (A5) with 101 and 83 mentions, all legacy `further-information` /
    `tender-receipt` (issue 585). Only their touchpoint rows are gone.

## B: the 30 minted items

| item | stratum | tender | seq | lots | role | org | org country | org mentions | TPO BT-514 | notice / TPO (parent) | served name (= TPO BT-500) | parent ORG name | window row |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| B01 | nl_addinfo_tenderreceipt | 180546 | 1 | LOT-0000 | Lot-AddInfo | 1327872 | NL | 27 | NLD | 25284773 / TPO-0002 (ORG-0001) | Stichting Quadraam | Quadraam Gelderse Onderwijsgroep | n6 row 9 |
| B02 | nl_addinfo_tenderreceipt | 186355 | 2 | LOT-0000 | Lot-AddInfo | 3703630 | NL | 3 | NLD | 23884997 / TPO-0002 (ORG-0001) | Ingenion | Stichting Vrijescholen Ithaka | n6 row 16 |
| B03 | nl_addinfo_tenderreceipt | 183445 | 2 | LOT-0000 | Lot-AddInfo | 10359863 | NL | 527 | NLD | 25869133 / TPO-0002 (ORG-0001) | Team Inkoop gemeente Westland | gemeente Westland | n6 row 14 |
| B04 | nl_addinfo_tenderreceipt | 181653 | 2 | LOT-0000 | Lot-TenderReceipt | 20951176 | NL | 129 | NLD | 25570206 / TPO-0001 (ORG-0001) | IBMN | Stichting Inkoopbureau Midden Nederland | n6 row 12 |
| B05 | nl_addinfo_tenderreceipt | 840364 | 1 | LOT-0000 | Lot-AddInfo | 13300018 | NL | 21 | NLD | 25896618 / TPO-0002 (ORG-0001) | Alpha Adviesbureau t.a.v. Martine Godlieb, Pieter Sneeuwplein 12, 9461 KA Veendam | Stichting Openbaar Primair en Speciaal Onderwijs Leiden | o8 row 8 |
| B06 | nl_addinfo_tenderreceipt | 172840 | 2 | LOT-0000 | Lot-AddInfo | 17378824 | NL | 19 | NLD | 27019561 / TPO-0004 (ORG-0001) | P.D. van Buuren | Stichting Scholen voor Christelijk Basisonderwijs Zeist en o | n6 row 2 |
| B07 | country_fallback | 841758 | 2 | LOT-0001 | Lot-DocProvider | 1166638 | FR | 1356 | — | 24981133 / TPO-0001 (ORG-0001) | Direction de la Commande Publique | Mairie du Bourget | o8 row 25 |
| B08 | country_fallback | 327448 | 1 | LOT-0001 | Lot-ReviewOrg | 31976119 | ES | 62 | — | 24769528 / TPO-0001 (ORG-0001) | Tribunal Arbitral de Barcelona (TAB) | Secretariat of the Union for the Mediterranean (UfM Secretar | m2 row 21 |
| B09 | country_fallback | 907162 | 2 | LOT-0001 | Lot-ReviewInfo | 8835 | FR | 11764 | — | 23836878 / TPO-0001 (ORG-0001) | Tribunal judiciaire de Paris | France Télévisions | m1 row 34 |
| B10 | review_org | 327552 | 1 | LOT-0000 | Lot-ReviewOrg | 3955 | ES | 122448 | ESP | 23987806 / TPO-0001 (ORG-0001) | Tribunal Administrativo Central de Recursos Contractuales | Jefatura de Asuntos Económicos de la Guardia Civil | m2 row 25 |
| B11 | review_org | 8563433 | 1 | LOT-0000 | Lot-ReviewOrg | 22077939 | ES | 2534 | ESP | 45847060 / TPO-0001 (ORG-0001) | Tribunal Administrativo Central de Recursos Contractuales (TACRC) o Tribunal Superior de J | Dirección General de Racionalización y Centralización de la  | m3 row 21 |
| B12 | review_org | 8563314 | 2 | LOT-0000 | Lot-ReviewOrg | 3940880 | NL | 594 | NLD | 46713198 / TPO-0001 (ORG-0001) | Rechtbank Noord-Nederland, locatie Groningen | Universitair Medisch Centrum Groningen (UMCG) | m3 row 11 |
| B13 | review_org | 906994 | 2 | LOT-0000 | Lot-ReviewOrg | 6724282 | ES | 8442 | ESP | 25276503 / TPO-0001 (ORG-0001) | Tribunal Administrativo de Contratos Públicos de la Comunidad Autónoma de Canarias | Junta de Gobierno del Ayuntamiento de Arona | m1 row 18 |
| B14 | review_org | 8829047 | 1 | LOT-0001,LOT-0002,LOT-0003 | Lot-ReviewOrg | 1223007 | ES | 410 | ESP | 47365057 / TPO-0002 (ORG-0001) | unidad de contratación | Intendente de Rota | m4 row 29 |
| B15 | review_org | 8829020 | 1 | LOT-0000 | Lot-ReviewOrg | 31974996 | ES | 159 | ESP | 47364669 / TPO-0002 (ORG-0001) | Tribunal Administrativo de Recursos Contratación Junta de Andalucía | Junta de Gobierno del Ayuntamiento de Dos Hermanas | m4 row 27 |
| B16 | rest | 842165 | 2 | LOT-0001,LOT-0002,LOT-0003 | Lot-AddInfo | 1298093 | PL | 228 | POL | 25356877 / TPO-0001 (ORG-0001) | Regionalny Zarząd Gospodarki Wodnej w Poznaniu | Państwowe Gospodarstwo Wodne Wody Polskie | o8 row 33 |
| B17 | rest | 840276 | 4 | LOT-0001 | Lot-DocProvider | 21794740 | DE | 528 | DEU | 24752607 / TPO-0004 (ORG-0004) | Hitzler Ingenieure GmbH & Co. KG | Hessing Stiftung | o8 row 6 |
| B18 | rest | 841315 | 1 | LOT-0005 | Lot-EmployLegis | 31975092 | PL | 159 | — | 25516725 / TPO-0001 (ORG-0001) | Centrum Usług Logistycznych - centrala w Warszawie | Centrum Usług Logistycznych | o8 row 19 |
| B19 | rest | 842454 | 2 | LOT-0001 | Lot-EnvironLegis | 1163223 | FR | 26883 | FRA | 25493325 / TPO-0001 (ORG-0001) | Tribunal Administratif d'Orléans | Université d'Orléans | o8 row 36 |
| B20 | rest | 841726 | 1 | LOT-0001,LOT-0002,LOT-0003 | Lot-Mediator | 12190 | MK | 482 | — | 25523400 / TPO-0001 (ORG-0001) | State Commission for Public Procurement Appeals | Ministry of Health of the Republic of North Macedonia | o8 row 23 |
| B21 | rest | 8828989 | 2 | LOT-0000 | Lot-ReviewInfo | 19204686 | DE | 803 | DEU | 47365117 / TPO-0000 (ORG-0000) | Deutsche Bundesbank, Leiter des Zentralbereichs Beschaffungen | Deutsche Bundesbank | m4 row 22 |
| B22 | rest | 841789 | 6 | LOT-0000 | Lot-TenderEval | 16649900 | DE | 174 | DEU | 24152112 / TPO-0000 (ORG-0000) | SachsenNetze GmbH | DREWAG - Stadtwerke Dresden GmbH | o8 row 27 |
| B23 | rest | 840627 | 2 | LOT-0001 | Lot-TenderReceipt | 8377627 | PL | 95 | POL | 24915334 / TPO-0001 (ORG-0001) | Agencja Mienia Wojskowego Oddział Regionalny w Poznaniu | Agencja Mienia Wojskowego | o8 row 12 |
| B24 | rest | 327623 | 2 | LOT-0001 | Lot-ReviewOrg | 2937423 | IT | 777 | ITA | 24320141 / TPO-0001 (ORG-0001) | T.A.R. VENETO | Città Metropolitana di Venezia | m2 row 38 |
| B25 | rest | 842057 | 2 | LOT-0001 | Lot-AddInfo | 31975451 | FR | 14 | — | 24465060 / TPO-0001 (ORG-0001) | Bureau des achats immobiliers et prestations (BAIP) | Ministère de l'intérieur et des outre-­mer | o8 row 30 |
| B26 | rest | 840723 | 2 | LOT-0003 | Lot-DocProvider | 31976551 | PL | 8 | POL | 24331180 / TPO-0002 (ORG-0001) | Państwowe Gospodarstwo Wodne Wody Polskie - RZGW Rzeszów | Państwowe Gospodarstwo Wodne Wody Polskie | o8 row 14 |
| B27 | rest | 841073 | 4 | LOT-0001 | Lot-Mediator | 1617 | PL | 620903 | — | 24335977 / TPO-0005 (ORG-0005, not the buyer) | Krajowa Izba Odwoławcza | Urząd Zamówień Publicznych Krajowa Izba Odwoławcza | o8 row 18 |
| B28 | rest | 327531 | 2 | LOT-0001,LOT-0002 | Lot-ReviewInfo | 31975366 | FR | 38 | — | 24169526 / TPO-0002 (ORG-0002, not the buyer) | Tribunal administratif de Poitiers\nInstance chargée de l'introduction des recours | Tribunal administratif de poitiers | m2 row 24 |
| B29 | rest | 840680 | 2 | LOT-0001,LOT-0002 | Lot-TenderEval | 9327844 | PL | 37 | — | 23740275 / TPO-0001 (ORG-0001) | Dział Zamówień Publicznych i Zaopatrzenia | Samodzielny Publiczny Zakład Opieki Zdrowotnej - Szpital Uni | o8 row 13 |
| B30 | rest | 172840 | 2 | LOT-0000 | Lot-TenderReceipt | 17925189 | NL | 14 | NLD | 27019561 / TPO-0001 (ORG-0001) | DAS makkelijk | Stichting Scholen voor Christelijk Basisonderwijs Zeist en o | n6 row 3 |

## C: the 30 aliased items

| item | stratum | final.py tier | tender | seq | lots | role | org | org mentions | notice / TPO → parent | TPO BT-500 (printed) | parent ORG BT-500 (served mention_name) | window row |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| C01 | sarga_540072 | A | 540072 | 4 | 4 (LOT-0001…) | Lot-ReviewOrg | 22198704 | 127 | 23563650 / TPO-0001 → ORG-0001 | Sociedad Aragonesa de Gestión Agroambiental S.L.U | Sociedad Aragonesa de Gestión Agroambiental, S.L.U. (SARGA) | x540072 row 0 |
| C02 | tier_a | A | 90787 | 1 | 1 (LOT-0000) | Lot-ReviewInfo | 2660 | 9103 | 23902466 / TPO-0002 → ORG-0002 | TRIBUNAL ADMINISTRATIVO DE RECURSOS CONTRACTUALES DE LA JUNTA DE ANDALUCIA | TRIBUNAL ADMINISTRATIVO DE LOS RECURSOS CONTRACTUALES DE LA JUNTA DE ANDALU | a2 row 4 |
| C03 | tier_a | A | 208487 | 2 | 1 (LOT-0000) | Lot-ReviewOrg | 22373292 | 38 | 25234946 / TPO-0001 → ORG-0001 | Alcaldia del Ayuntamiento de Manacor | Alcaldía del Ayuntamiento de Manacor | a0 row 24 |
| C04 | tier_a | A | 90787 | 1 | 1 (LOT-0000) | Lot-AddInfo | 1400401 | 177 | 23902466 / TPO-0001 → ORG-0001 | AGENCIA DE VIVIENDA Y REHABILITACION DE ANDALUCIA (PUNTO DE CONTACTO) | AGENCIA DE VIVIENDA Y REHABILITACION DE ANDALUCIA | a2 row 3 |
| C05 | tier_a | A | 818005 | 2 | 1 (LOT-0000) | Lot-ReviewInfo | 22204151 | 98 | 25881740 / TPO-0001 → ORG-0001 | Fundación Colección Thyssen-Bornemisza, F.S.P. | Fundación Colección Thyssen Bornemisza | a5 row 24 |
| C06 | tier_a | A | 92339 | 2 | 1 (LOT-0000) | Lot-ReviewOrg | 22210400 | 77 | 25412962 / TPO-0001 → ORG-0001 | CONSEJERIA DE INFRAESTRUCTURAS, TRANSPORTE Y VIVIENDA | Consejería de Infraestructuras, Transporte y Vivienda | a2 row 18 |
| C07 | tier_a | A | 90277 | 2 | 1 (LOT-0000) | Lot-ReviewOrg | 12184 | 716 | 24447031 / TPO-0001 → ORG-0001 | Direccion  Gestion Economica de la Jefatura de Apoyo Logistico de la Armada | Dirección de Gestión Económica de la Jefatura de Apoyo Logístico de la Arma | a2 row 1 |
| C08 | tier_C0 | C0 | 570420 | 2 | 1 (LOT-0000) | Lot-ReviewInfo | 1840849 | 49 | 25740680 / TPO-0001 → ORG-0001 | Junta de Gobierno Local del Ayuntamiento de Petrer | Junta de Gobierno del Ayuntamiento de Petrer | a6 row 1 |
| C09 | tier_C3 | C3 | 816406 | 2 | 1 (LOT-0000) | Lot-ReviewInfo | 22766649 | 6 | 25890602 / TPO-0001 → ORG-0001 | Malpartida de Cáceres | Pleno del Ayuntamiento de Malpartida de Cáceres | a5 row 2 |
| C10 | tier_C3r | C3r | 571640 | 1 | 1 (LOT-0001) | Lot-ReviewOrg | 22417802 | 4 | 23860749 / TPO-0001 → ORG-0001 | Comune di Cancello ed Arnone | CANCELLO ED ARNONE | a6 row 7 |
| C11 | tier_C2 | C2 | 206565 | 5 | 4 (LOT-0001…) | Lot-ReviewOrg | 22380833 | 16 | 25641431 / TPO-0001 → ORG-0001 | Ayuntamiento de Artà | Junta de Gobierno Local de l'Ajuntament d'Artà | a0 row 1 |
| C12 | tier_C2 | C2 | 207309 | 3 | 1 (LOT-0000) | Lot-ReviewInfo | 23596430 | 3 | 25799531 / TPO-0001 → ORG-0001 | Ajuntament de Miramar | Pleno del Ayuntamiento de Miramar | a0 row 13 |
| C13 | tier_C2 | C2 | 91153 | 1 | 1 (LOT-0000) | Lot-ReviewOrg | 23639113 | 1 | 25701105 / TPO-0002 → ORG-0001 | Concello de Sada | Pleno del Ayuntamiento de Sada | a2 row 7 |
| C14 | tier_C4 | C4 | 96226 | 1 | 1 (LOT-0000) | Lot-ReviewOrg | 8434 | 227 | 23713543 / TPO-0001 → ORG-0001 | Rectorado | Rectorado de la Universidad Politécnica de Madrid | a3 row 18 |
| C15 | tier_C4 | C4 | 819222 | 1 | 1 (LOT-0000) | Lot-ReviewOrg | 22206492 | 257 | 24065285 / TPO-0001 → ORG-0001 | Consejo de Gobierno Insular | Consejo de Gobierno Insular del Cabildo Insular de Tenerife | a5 row 34 |
| C16 | tier_C4 | C4 | 94591 | 1 | 1 (LOT-0000) | Lot-ReviewOrg | 22701226 | 2 | 25660854 / TPO-0001 → ORG-0001 | Ayuntamiento | Alcaldia del Ayuntamiento de Fregenal de la Sierra | a3 row 1 |
| C17 | tier_C4 | C4 | 94863 | 2 | 1 (LOT-0000) | Lot-ReviewOrg | 22212613 | 54 | 24527798 / TPO-0001 → ORG-0001 | JUNTA DE GOBIERNO LOCAL | Junta de Gobierno del Ayuntamiento de Alcoy | a3 row 4 |
| C18 | tier_C1 | C1 | 572953 | 2 | 1 (LOT-0000) | Lot-ReviewInfo | 22608797 | 18 | 45599465 / TPO-0001 → ORG-0001 | Ayuntamiento Villa de Breña Alta | Alcaldía del Ayuntamiento de La Villa de Breña Alta | a6 row 17 |
| C19 | tier_C1 | C1 | 90507 | 1 | 1 (LOT-0000) | Lot-ReviewOrg | 22553459 | 52 | 24113268 / TPO-0001 → ORG-0001 | Ayuntamiento de Tres Cantos | Junta de Gobierno Local del Ayuntamiento de Tres Cantos | a2 row 2 |
| C20 | tier_C1 | C1 | 208143 | 2 | 1 (LOT-0000) | Lot-Mediator | 22263342 | 15 | 24377243 / TPO-0001 → ORG-0001 | Ayuntamiento de Ayamonte | Alcaldia del Ayuntamiento de Ayamonte | a0 row 22 |
| C21 | tier_C1 | C1 | 207120 | 1 | 1 (LOT-0000) | Lot-ReviewOrg | 22357667 | 8 | 23651566 / TPO-0002 → ORG-0001 | AYUNTAMIENTO DE LALÍN | Junta de Gobierno del Ayuntamiento de Lalín | a0 row 9 |
| C22 | tier_C1 | C1 | 209192 | 2 | 1 (LOT-0000) | Lot-ReviewInfo | 22236010 | 6 | 24250162 / TPO-0001 → ORG-0001 | AYUNTAMIENTO DE COBEÑA | Junta de Gobierno del Ayuntamiento de Cobeña | a0 row 27 |
| C23 | tier_C1 | C1 | 91287 | 1 | 1 (LOT-0000) | Lot-Mediator | 22599973 | 1 | 23943126 / TPO-0001 → ORG-0001 | Ayuntamiento de Bellver de Cerdanya | Alcaldía del Ayuntamiento de Bellver de Cerdanya | a2 row 8 |
| C24 | tier_C1 | C1 | 817901 | 4 | 22 (LOT-0001…) | Lot-ReviewInfo | 1624 | 18 | 27524483 / TPO-0001 → ORG-0001 | Ayuntamiento Puente Genil | Junta de Gobierno Local de Ayuntamiento de Puente Genil | a5 row 17 |
| C25 | tier_C1 | C1 | 97240 | 2 | 1 (LOT-0000) | Lot-ReviewOrg | 22571902 | 11 | 24576081 / TPO-0001 → ORG-0001 | Palacio de Congresos de Valencia | Gerencia del Palacio de Congresos de Valencia | a3 row 31 |
| C26 | tier_C1 | C1 | 818800 | 2 | 1 (LOT-0000) | Lot-Mediator | 22312322 | 4 | 23627296 / TPO-0003 → ORG-0001 | AYUNTAMIENTO DE FINES | Pleno del Ayuntamiento de Fines | a5 row 30 |
| C27 | tier_C1 | C1 | 95712 | 1 | 1 (LOT-0000) | Lot-ReviewOrg | 11369 | 60 | 24911882 / TPO-0001 → ORG-0001 | Ayuntamiento de Badajoz | Alcaldía del Ayuntamiento de Badajoz | a3 row 14 |
| C28 | tier_C1 | C1 | 206590 | 2 | 1 (LOT-0000) | Lot-ReviewInfo | 4147 | 170 | 25259657 / TPO-0001 → ORG-0001 | Mutua Montañesa | Dirección General de Mutua Montañesa | a0 row 4 |
| C29 | tier_C1 | C1 | 571778 | 2 | 1 (LOT-0000) | Lot-ReviewOrg | 23049649 | 2 | 25134461 / TPO-0002 → ORG-0001 | AYUNTAMIENTO DE ANTEQUERA | Junta de Gobierno del Ayuntamiento de Antequera | a6 row 10 |
| C30 | tier_C1 | C1 | 96857 | 3 | 1 (LOT-0000) | Lot-ReviewInfo | 22147295 | 196 | 23744570 / TPO-0001 → ORG-0001 | Ayuntamiento de Torrejón de Ardoz | Junta de Gobierno del Ayuntamiento de Torrejón de Ardoz | a3 row 25 |

## Statements sent (18 of 20; each plan-probed locally on plan-copy.db and plan-idx.db before sending)

One draft was refused locally and never sent. It was the aliased statement with LEFT JOINs to notice_texts plus the
`lower(tn.value) <> lower(pn.value)` filter, and it planned as `SCAN notice_texts AS pn` plus `MULTI-INDEX AND t` on both
copies: the filter turned the outer joins inner and the planner drove from notice_texts. Rewritten as CROSS JOINs, the
plan is all SEARCH (statements a0–a6 and x540072). An earlier draft that seeked notice_ids by (notice_id, section_id
range) was replaced, before any send, with the lots-table join, which seeks (notice_id, section_id, field_id) exactly.

| # | name | purpose / window | sent (UTC) | secs | rows | answer |
|---|---|---|---|---|---|---|
| 1 | v01_labels | A4: tender_version_parties roles of orgs 18044191 / 18044192 (two equality probes, UNION ALL) | 2026-10-24T18:11:51Z | 0.4 | 2 | JSON rows+columns, truncated=false |
| 2 | m0 | B minted general i=0 (salt 576-u5-, sha256 71e538163db32755, s=1434149) ids 8754149..8757149 | 2026-10-24T18:13:33Z | 0.43 | 0 | JSON rows+columns, truncated=false |
| 3 | m1 | B minted general i=1 (salt 576-u5-, sha256 01fedec5cdffe151, s=906785) ids 906786..909786 | 2026-10-24T18:13:37Z | 0.63 | 40 | JSON rows+columns, truncated=false |
| 4 | m2 | B minted general i=2 (salt 576-u5-, sha256 a919246df72458a3, s=327123) ids 327124..330124 | 2026-10-24T18:13:41Z | 0.65 | 40 | JSON rows+columns, truncated=false |
| 5 | m3 | B minted general i=3 (salt 576-u5-, sha256 efdd950f3b603f9c, s=1243228) ids 8563228..8566228 | 2026-10-24T18:13:45Z | 0.85 | 40 | JSON rows+columns, truncated=false |
| 6 | m4 | B minted general i=4 (salt 576-u5-, sha256 7dbffefc8e6adb7c, s=1508668) ids 8828668..8831668 | 2026-10-24T18:13:49Z | 0.68 | 40 | JSON rows+columns, truncated=false |
| 7 | m5 | B minted general i=5 (salt 576-u5-, sha256 ce480480319aafa8, s=1418136) ids 8738136..8741136 | 2026-10-24T18:13:54Z | 0.41 | 0 | JSON rows+columns, truncated=false |
| 8 | n6 | B minted NL AddInfo/TenderReceipt i=6 (salt 576-u5-, sha256 59628c5321ce32e1, s=166369) ids 166370..186370 | 2026-10-24T18:14:23Z | 1.44 | 17 | JSON rows+columns, truncated=false |
| 9 | o8 | B minted other roles i=8 (salt 576-u5-, sha256 cb52729126d876d6, s=839590) ids 839591..859591 | 2026-10-24T18:14:28Z | 1.5 | 40 | JSON rows+columns, truncated=false |
| 10 | a0 | C aliased i=0 (salt 586-u3-, sha256 3c0f30a755322149, s=206361) ids 206362..209362 | 2026-10-24T18:14:34Z | 2.03 | 29 | JSON rows+columns, truncated=false |
| 11 | a1 | C aliased i=1 (salt 586-u3-, sha256 bd7834bfb9c37abc, s=1402412) ids 8722412..8725412 | 2026-10-24T18:14:40Z | 0.43 | 0 | JSON rows+columns, truncated=false |
| 12 | a2 | C aliased i=2 (salt 586-u3-, sha256 6930941ef824dd1e, s=89966) ids 89967..92967 | 2026-10-24T18:14:44Z | 1.78 | 20 | JSON rows+columns, truncated=false |
| 13 | a3 | C aliased i=3 (salt 586-u3-, sha256 ff8380b8dfcd80ee, s=94462) ids 94463..97463 | 2026-10-24T18:14:49Z | 1.7 | 38 | JSON rows+columns, truncated=false |
| 14 | a5 | C aliased i=5 (salt 586-u3-, sha256 6cf17b4c18a4bf6d, s=816221) ids 816222..819222 | 2026-10-24T18:15:12Z | 1.73 | 35 | JSON rows+columns, truncated=false |
| 15 | a6 | C aliased i=6 (salt 586-u3-, sha256 58ce0a14096b8b8c, s=570204) ids 570205..573205 | 2026-10-24T18:15:17Z | 1.61 | 19 | JSON rows+columns, truncated=false |
| 16 | x540072 | C fixed: the aliased statement on tender 540072 alone (SARGA exhibit) | 2026-10-24T18:15:33Z | 0.38 | 4 | JSON rows+columns, truncated=false |
| 17 | s_mentions_b | organization_mentions of the 29 minted-sample notices (PK prefix probes, UNION ALL) | 2026-10-24T18:16:58Z | 0.37 | 110 | JSON rows+columns, truncated=false |
| 18 | s_mentions_c | organization_mentions of the 29 aliased-sample notices (PK prefix probes, UNION ALL) | 2026-10-24T18:17:02Z | 0.39 | 55 | JSON rows+columns, truncated=false |

### 1. v01_labels

```sql
SELECT p.organization_id AS org_id, p.role AS role, count(*) AS n_rows, sum(p.seq = t.current_seq) AS n_current_rows, min(p.tender_id) AS min_tender, max(p.tender_id) AS max_tender, min(p.mention_section_id) AS min_section, max(p.mention_section_id) AS max_section FROM tender_version_parties p CROSS JOIN tenders t ON t.id = p.tender_id WHERE p.organization_id = 18044191 GROUP BY p.role UNION ALL SELECT p.organization_id AS org_id, p.role AS role, count(*) AS n_rows, sum(p.seq = t.current_seq) AS n_current_rows, min(p.tender_id) AS min_tender, max(p.tender_id) AS max_tender, min(p.mention_section_id) AS min_section, max(p.mention_section_id) AS max_section FROM tender_version_parties p CROSS JOIN tenders t ON t.id = p.tender_id WHERE p.organization_id = 18044192 GROUP BY p.role
```

Local EXPLAIN QUERY PLAN (turso plan-probe):

```
# plan-copy
COMPOUND QUERY
  LEFT-MOST SUBQUERY
    SEARCH p USING INDEX tender_version_parties_org (organization_id=?)
    SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
    USE SORTER FOR GROUP BY
  UNION ALL
    SEARCH p USING INDEX tender_version_parties_org (organization_id=?)
    SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
    USE SORTER FOR GROUP BY
# plan-idx
COMPOUND QUERY
  LEFT-MOST SUBQUERY
    SEARCH p USING INDEX tender_version_parties_org (organization_id=?)
    SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
    USE SORTER FOR GROUP BY
  UNION ALL
    SEARCH p USING INDEX tender_version_parties_org (organization_id=?)
    SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
    USE SORTER FOR GROUP BY
```

### 2. m0

```sql
SELECT t.id AS tender_id, p.seq AS seq, p.role AS role, p.organization_id AS org_id, p.mention_notice_id AS notice_id, p.mention_section_id AS section_id, min(p.lot_id) AS lot_id_min, count(*) AS n_rows, o.country AS org_country, o.name AS org_name, o.provisional AS org_provisional, c.code AS tp_bt514 FROM tenders t CROSS JOIN tender_version_parties p ON p.tender_id = t.id AND p.seq = t.current_seq CROSS JOIN organizations o ON o.id = p.organization_id LEFT JOIN notice_codes c ON c.notice_id = p.mention_notice_id AND c.section_id = p.mention_section_id AND c.field_id = 'BT-514-Organization-TouchPoint' AND c.ordinal = 0 WHERE t.id BETWEEN 8754149 AND 8757149 AND p.mention_section_id LIKE 'TPO-%' GROUP BY t.id, p.seq, p.role, p.organization_id, p.mention_notice_id, p.mention_section_id LIMIT 40
```

Local EXPLAIN QUERY PLAN (turso plan-probe):

```
# plan-copy
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
SEARCH c USING INDEX sqlite_autoindex_notice_codes_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?) LEFT-JOIN
USE SORTER FOR GROUP BY
# plan-idx
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
SEARCH c USING INDEX sqlite_autoindex_notice_codes_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?) LEFT-JOIN
USE SORTER FOR GROUP BY
```

### 3. m1

```sql
SELECT t.id AS tender_id, p.seq AS seq, p.role AS role, p.organization_id AS org_id, p.mention_notice_id AS notice_id, p.mention_section_id AS section_id, min(p.lot_id) AS lot_id_min, count(*) AS n_rows, o.country AS org_country, o.name AS org_name, o.provisional AS org_provisional, c.code AS tp_bt514 FROM tenders t CROSS JOIN tender_version_parties p ON p.tender_id = t.id AND p.seq = t.current_seq CROSS JOIN organizations o ON o.id = p.organization_id LEFT JOIN notice_codes c ON c.notice_id = p.mention_notice_id AND c.section_id = p.mention_section_id AND c.field_id = 'BT-514-Organization-TouchPoint' AND c.ordinal = 0 WHERE t.id BETWEEN 906786 AND 909786 AND p.mention_section_id LIKE 'TPO-%' GROUP BY t.id, p.seq, p.role, p.organization_id, p.mention_notice_id, p.mention_section_id LIMIT 40
```

Local EXPLAIN QUERY PLAN (turso plan-probe):

```
# plan-copy
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
SEARCH c USING INDEX sqlite_autoindex_notice_codes_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?) LEFT-JOIN
USE SORTER FOR GROUP BY
# plan-idx
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
SEARCH c USING INDEX sqlite_autoindex_notice_codes_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?) LEFT-JOIN
USE SORTER FOR GROUP BY
```

### 4. m2

```sql
SELECT t.id AS tender_id, p.seq AS seq, p.role AS role, p.organization_id AS org_id, p.mention_notice_id AS notice_id, p.mention_section_id AS section_id, min(p.lot_id) AS lot_id_min, count(*) AS n_rows, o.country AS org_country, o.name AS org_name, o.provisional AS org_provisional, c.code AS tp_bt514 FROM tenders t CROSS JOIN tender_version_parties p ON p.tender_id = t.id AND p.seq = t.current_seq CROSS JOIN organizations o ON o.id = p.organization_id LEFT JOIN notice_codes c ON c.notice_id = p.mention_notice_id AND c.section_id = p.mention_section_id AND c.field_id = 'BT-514-Organization-TouchPoint' AND c.ordinal = 0 WHERE t.id BETWEEN 327124 AND 330124 AND p.mention_section_id LIKE 'TPO-%' GROUP BY t.id, p.seq, p.role, p.organization_id, p.mention_notice_id, p.mention_section_id LIMIT 40
```

Local EXPLAIN QUERY PLAN (turso plan-probe):

```
# plan-copy
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
SEARCH c USING INDEX sqlite_autoindex_notice_codes_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?) LEFT-JOIN
USE SORTER FOR GROUP BY
# plan-idx
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
SEARCH c USING INDEX sqlite_autoindex_notice_codes_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?) LEFT-JOIN
USE SORTER FOR GROUP BY
```

### 5. m3

```sql
SELECT t.id AS tender_id, p.seq AS seq, p.role AS role, p.organization_id AS org_id, p.mention_notice_id AS notice_id, p.mention_section_id AS section_id, min(p.lot_id) AS lot_id_min, count(*) AS n_rows, o.country AS org_country, o.name AS org_name, o.provisional AS org_provisional, c.code AS tp_bt514 FROM tenders t CROSS JOIN tender_version_parties p ON p.tender_id = t.id AND p.seq = t.current_seq CROSS JOIN organizations o ON o.id = p.organization_id LEFT JOIN notice_codes c ON c.notice_id = p.mention_notice_id AND c.section_id = p.mention_section_id AND c.field_id = 'BT-514-Organization-TouchPoint' AND c.ordinal = 0 WHERE t.id BETWEEN 8563228 AND 8566228 AND p.mention_section_id LIKE 'TPO-%' GROUP BY t.id, p.seq, p.role, p.organization_id, p.mention_notice_id, p.mention_section_id LIMIT 40
```

Local EXPLAIN QUERY PLAN (turso plan-probe):

```
# plan-copy
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
SEARCH c USING INDEX sqlite_autoindex_notice_codes_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?) LEFT-JOIN
USE SORTER FOR GROUP BY
# plan-idx
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
SEARCH c USING INDEX sqlite_autoindex_notice_codes_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?) LEFT-JOIN
USE SORTER FOR GROUP BY
```

### 6. m4

```sql
SELECT t.id AS tender_id, p.seq AS seq, p.role AS role, p.organization_id AS org_id, p.mention_notice_id AS notice_id, p.mention_section_id AS section_id, min(p.lot_id) AS lot_id_min, count(*) AS n_rows, o.country AS org_country, o.name AS org_name, o.provisional AS org_provisional, c.code AS tp_bt514 FROM tenders t CROSS JOIN tender_version_parties p ON p.tender_id = t.id AND p.seq = t.current_seq CROSS JOIN organizations o ON o.id = p.organization_id LEFT JOIN notice_codes c ON c.notice_id = p.mention_notice_id AND c.section_id = p.mention_section_id AND c.field_id = 'BT-514-Organization-TouchPoint' AND c.ordinal = 0 WHERE t.id BETWEEN 8828668 AND 8831668 AND p.mention_section_id LIKE 'TPO-%' GROUP BY t.id, p.seq, p.role, p.organization_id, p.mention_notice_id, p.mention_section_id LIMIT 40
```

Local EXPLAIN QUERY PLAN (turso plan-probe):

```
# plan-copy
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
SEARCH c USING INDEX sqlite_autoindex_notice_codes_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?) LEFT-JOIN
USE SORTER FOR GROUP BY
# plan-idx
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
SEARCH c USING INDEX sqlite_autoindex_notice_codes_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?) LEFT-JOIN
USE SORTER FOR GROUP BY
```

### 7. m5

```sql
SELECT t.id AS tender_id, p.seq AS seq, p.role AS role, p.organization_id AS org_id, p.mention_notice_id AS notice_id, p.mention_section_id AS section_id, min(p.lot_id) AS lot_id_min, count(*) AS n_rows, o.country AS org_country, o.name AS org_name, o.provisional AS org_provisional, c.code AS tp_bt514 FROM tenders t CROSS JOIN tender_version_parties p ON p.tender_id = t.id AND p.seq = t.current_seq CROSS JOIN organizations o ON o.id = p.organization_id LEFT JOIN notice_codes c ON c.notice_id = p.mention_notice_id AND c.section_id = p.mention_section_id AND c.field_id = 'BT-514-Organization-TouchPoint' AND c.ordinal = 0 WHERE t.id BETWEEN 8738136 AND 8741136 AND p.mention_section_id LIKE 'TPO-%' GROUP BY t.id, p.seq, p.role, p.organization_id, p.mention_notice_id, p.mention_section_id LIMIT 40
```

Local EXPLAIN QUERY PLAN (turso plan-probe):

```
# plan-copy
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
SEARCH c USING INDEX sqlite_autoindex_notice_codes_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?) LEFT-JOIN
USE SORTER FOR GROUP BY
# plan-idx
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
SEARCH c USING INDEX sqlite_autoindex_notice_codes_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?) LEFT-JOIN
USE SORTER FOR GROUP BY
```

### 8. n6

```sql
SELECT t.id AS tender_id, p.seq AS seq, p.role AS role, p.organization_id AS org_id, p.mention_notice_id AS notice_id, p.mention_section_id AS section_id, min(p.lot_id) AS lot_id_min, count(*) AS n_rows, o.country AS org_country, o.name AS org_name, o.provisional AS org_provisional, c.code AS tp_bt514 FROM tenders t CROSS JOIN tender_version_parties p ON p.tender_id = t.id AND p.seq = t.current_seq CROSS JOIN organizations o ON o.id = p.organization_id LEFT JOIN notice_codes c ON c.notice_id = p.mention_notice_id AND c.section_id = p.mention_section_id AND c.field_id = 'BT-514-Organization-TouchPoint' AND c.ordinal = 0 WHERE t.id BETWEEN 166370 AND 186370 AND p.mention_section_id LIKE 'TPO-%' AND +p.role IN ('Lot-AddInfo', 'Lot-TenderReceipt') AND +o.country = 'NL' GROUP BY t.id, p.seq, p.role, p.organization_id, p.mention_notice_id, p.mention_section_id LIMIT 40
```

Local EXPLAIN QUERY PLAN (turso plan-probe):

```
# plan-copy
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
SEARCH c USING INDEX sqlite_autoindex_notice_codes_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?) LEFT-JOIN
USE SORTER FOR GROUP BY
# plan-idx
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
SEARCH c USING INDEX sqlite_autoindex_notice_codes_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?) LEFT-JOIN
USE SORTER FOR GROUP BY
```

### 9. o8

```sql
SELECT t.id AS tender_id, p.seq AS seq, p.role AS role, p.organization_id AS org_id, p.mention_notice_id AS notice_id, p.mention_section_id AS section_id, min(p.lot_id) AS lot_id_min, count(*) AS n_rows, o.country AS org_country, o.name AS org_name, o.provisional AS org_provisional, c.code AS tp_bt514 FROM tenders t CROSS JOIN tender_version_parties p ON p.tender_id = t.id AND p.seq = t.current_seq CROSS JOIN organizations o ON o.id = p.organization_id LEFT JOIN notice_codes c ON c.notice_id = p.mention_notice_id AND c.section_id = p.mention_section_id AND c.field_id = 'BT-514-Organization-TouchPoint' AND c.ordinal = 0 WHERE t.id BETWEEN 839591 AND 859591 AND p.mention_section_id LIKE 'TPO-%' AND +p.role NOT IN ('Lot-ReviewOrg', 'Lot-ReviewInfo') GROUP BY t.id, p.seq, p.role, p.organization_id, p.mention_notice_id, p.mention_section_id LIMIT 40
```

Local EXPLAIN QUERY PLAN (turso plan-probe):

```
# plan-copy
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
SEARCH c USING INDEX sqlite_autoindex_notice_codes_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?) LEFT-JOIN
USE SORTER FOR GROUP BY
# plan-idx
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
SEARCH c USING INDEX sqlite_autoindex_notice_codes_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?) LEFT-JOIN
USE SORTER FOR GROUP BY
```

### 10. a0

```sql
SELECT t.id AS tender_id, p.seq AS seq, p.role AS role, p.organization_id AS org_id, p.mention_notice_id AS notice_id, p.mention_section_id AS section_id, min(l.lot_key) AS lot_min, count(*) AS n_rows, i.value AS tpo, tn.value AS tp_name, pn.value AS parent_name FROM tenders t CROSS JOIN tender_version_parties p ON p.tender_id = t.id AND p.seq = t.current_seq CROSS JOIN lots l ON l.id = p.lot_id CROSS JOIN notice_ids i ON i.notice_id = p.mention_notice_id AND i.section_id = l.lot_key AND i.field_id = 'OPT-301-' || p.role AND +i.value LIKE 'TPO-%' CROSS JOIN notice_sections s ON s.notice_id = i.notice_id AND s.section_id = i.value CROSS JOIN notice_texts tn ON tn.notice_id = i.notice_id AND tn.section_id = i.value AND tn.field_id = 'BT-500-Organization-TouchPoint' AND tn.ordinal = 0 CROSS JOIN notice_texts pn ON pn.notice_id = i.notice_id AND pn.section_id = p.mention_section_id AND pn.field_id = 'BT-500-Organization-Company' AND pn.ordinal = 0 WHERE t.id BETWEEN 206362 AND 209362 AND +p.mention_section_id LIKE 'ORG-%' AND +p.role IN ('Lot-ReviewOrg', 'Lot-ReviewInfo', 'Lot-AddInfo', 'Lot-TenderReceipt', 'Lot-DocProvider', 'Lot-Mediator', 'Lot-TenderEval', 'Lot-FiscalLegis', 'Lot-EnvironLegis', 'Lot-EmployLegis') AND +s.parent_section_id = p.mention_section_id AND +s.kind = 'Touchpoint' AND lower(tn.value) <> lower(pn.value) GROUP BY t.id, p.seq, p.role, p.organization_id, p.mention_notice_id, p.mention_section_id, i.value LIMIT 60
```

Local EXPLAIN QUERY PLAN (turso plan-probe):

```
# plan-copy
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH l USING INTEGER PRIMARY KEY (rowid=?)
SEARCH i USING INDEX sqlite_autoindex_notice_ids_1 (notice_id=? AND section_id=? AND field_id=?)
SEARCH s USING INDEX sqlite_autoindex_notice_sections_1 (notice_id=? AND section_id=?)
SEARCH tn USING INDEX sqlite_autoindex_notice_texts_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?)
SEARCH pn USING INDEX sqlite_autoindex_notice_texts_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?)
USE SORTER FOR GROUP BY
# plan-idx
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH l USING INTEGER PRIMARY KEY (rowid=?)
SEARCH i USING INDEX sqlite_autoindex_notice_ids_1 (notice_id=? AND section_id=? AND field_id=?)
SEARCH s USING INDEX sqlite_autoindex_notice_sections_1 (notice_id=? AND section_id=?)
SEARCH tn USING INDEX sqlite_autoindex_notice_texts_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?)
SEARCH pn USING INDEX sqlite_autoindex_notice_texts_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?)
USE SORTER FOR GROUP BY
```

### 11. a1

```sql
SELECT t.id AS tender_id, p.seq AS seq, p.role AS role, p.organization_id AS org_id, p.mention_notice_id AS notice_id, p.mention_section_id AS section_id, min(l.lot_key) AS lot_min, count(*) AS n_rows, i.value AS tpo, tn.value AS tp_name, pn.value AS parent_name FROM tenders t CROSS JOIN tender_version_parties p ON p.tender_id = t.id AND p.seq = t.current_seq CROSS JOIN lots l ON l.id = p.lot_id CROSS JOIN notice_ids i ON i.notice_id = p.mention_notice_id AND i.section_id = l.lot_key AND i.field_id = 'OPT-301-' || p.role AND +i.value LIKE 'TPO-%' CROSS JOIN notice_sections s ON s.notice_id = i.notice_id AND s.section_id = i.value CROSS JOIN notice_texts tn ON tn.notice_id = i.notice_id AND tn.section_id = i.value AND tn.field_id = 'BT-500-Organization-TouchPoint' AND tn.ordinal = 0 CROSS JOIN notice_texts pn ON pn.notice_id = i.notice_id AND pn.section_id = p.mention_section_id AND pn.field_id = 'BT-500-Organization-Company' AND pn.ordinal = 0 WHERE t.id BETWEEN 8722412 AND 8725412 AND +p.mention_section_id LIKE 'ORG-%' AND +p.role IN ('Lot-ReviewOrg', 'Lot-ReviewInfo', 'Lot-AddInfo', 'Lot-TenderReceipt', 'Lot-DocProvider', 'Lot-Mediator', 'Lot-TenderEval', 'Lot-FiscalLegis', 'Lot-EnvironLegis', 'Lot-EmployLegis') AND +s.parent_section_id = p.mention_section_id AND +s.kind = 'Touchpoint' AND lower(tn.value) <> lower(pn.value) GROUP BY t.id, p.seq, p.role, p.organization_id, p.mention_notice_id, p.mention_section_id, i.value LIMIT 60
```

Local EXPLAIN QUERY PLAN (turso plan-probe):

```
# plan-copy
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH l USING INTEGER PRIMARY KEY (rowid=?)
SEARCH i USING INDEX sqlite_autoindex_notice_ids_1 (notice_id=? AND section_id=? AND field_id=?)
SEARCH s USING INDEX sqlite_autoindex_notice_sections_1 (notice_id=? AND section_id=?)
SEARCH tn USING INDEX sqlite_autoindex_notice_texts_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?)
SEARCH pn USING INDEX sqlite_autoindex_notice_texts_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?)
USE SORTER FOR GROUP BY
# plan-idx
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH l USING INTEGER PRIMARY KEY (rowid=?)
SEARCH i USING INDEX sqlite_autoindex_notice_ids_1 (notice_id=? AND section_id=? AND field_id=?)
SEARCH s USING INDEX sqlite_autoindex_notice_sections_1 (notice_id=? AND section_id=?)
SEARCH tn USING INDEX sqlite_autoindex_notice_texts_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?)
SEARCH pn USING INDEX sqlite_autoindex_notice_texts_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?)
USE SORTER FOR GROUP BY
```

### 12. a2

```sql
SELECT t.id AS tender_id, p.seq AS seq, p.role AS role, p.organization_id AS org_id, p.mention_notice_id AS notice_id, p.mention_section_id AS section_id, min(l.lot_key) AS lot_min, count(*) AS n_rows, i.value AS tpo, tn.value AS tp_name, pn.value AS parent_name FROM tenders t CROSS JOIN tender_version_parties p ON p.tender_id = t.id AND p.seq = t.current_seq CROSS JOIN lots l ON l.id = p.lot_id CROSS JOIN notice_ids i ON i.notice_id = p.mention_notice_id AND i.section_id = l.lot_key AND i.field_id = 'OPT-301-' || p.role AND +i.value LIKE 'TPO-%' CROSS JOIN notice_sections s ON s.notice_id = i.notice_id AND s.section_id = i.value CROSS JOIN notice_texts tn ON tn.notice_id = i.notice_id AND tn.section_id = i.value AND tn.field_id = 'BT-500-Organization-TouchPoint' AND tn.ordinal = 0 CROSS JOIN notice_texts pn ON pn.notice_id = i.notice_id AND pn.section_id = p.mention_section_id AND pn.field_id = 'BT-500-Organization-Company' AND pn.ordinal = 0 WHERE t.id BETWEEN 89967 AND 92967 AND +p.mention_section_id LIKE 'ORG-%' AND +p.role IN ('Lot-ReviewOrg', 'Lot-ReviewInfo', 'Lot-AddInfo', 'Lot-TenderReceipt', 'Lot-DocProvider', 'Lot-Mediator', 'Lot-TenderEval', 'Lot-FiscalLegis', 'Lot-EnvironLegis', 'Lot-EmployLegis') AND +s.parent_section_id = p.mention_section_id AND +s.kind = 'Touchpoint' AND lower(tn.value) <> lower(pn.value) GROUP BY t.id, p.seq, p.role, p.organization_id, p.mention_notice_id, p.mention_section_id, i.value LIMIT 60
```

Local EXPLAIN QUERY PLAN (turso plan-probe):

```
# plan-copy
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH l USING INTEGER PRIMARY KEY (rowid=?)
SEARCH i USING INDEX sqlite_autoindex_notice_ids_1 (notice_id=? AND section_id=? AND field_id=?)
SEARCH s USING INDEX sqlite_autoindex_notice_sections_1 (notice_id=? AND section_id=?)
SEARCH tn USING INDEX sqlite_autoindex_notice_texts_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?)
SEARCH pn USING INDEX sqlite_autoindex_notice_texts_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?)
USE SORTER FOR GROUP BY
# plan-idx
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH l USING INTEGER PRIMARY KEY (rowid=?)
SEARCH i USING INDEX sqlite_autoindex_notice_ids_1 (notice_id=? AND section_id=? AND field_id=?)
SEARCH s USING INDEX sqlite_autoindex_notice_sections_1 (notice_id=? AND section_id=?)
SEARCH tn USING INDEX sqlite_autoindex_notice_texts_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?)
SEARCH pn USING INDEX sqlite_autoindex_notice_texts_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?)
USE SORTER FOR GROUP BY
```

### 13. a3

```sql
SELECT t.id AS tender_id, p.seq AS seq, p.role AS role, p.organization_id AS org_id, p.mention_notice_id AS notice_id, p.mention_section_id AS section_id, min(l.lot_key) AS lot_min, count(*) AS n_rows, i.value AS tpo, tn.value AS tp_name, pn.value AS parent_name FROM tenders t CROSS JOIN tender_version_parties p ON p.tender_id = t.id AND p.seq = t.current_seq CROSS JOIN lots l ON l.id = p.lot_id CROSS JOIN notice_ids i ON i.notice_id = p.mention_notice_id AND i.section_id = l.lot_key AND i.field_id = 'OPT-301-' || p.role AND +i.value LIKE 'TPO-%' CROSS JOIN notice_sections s ON s.notice_id = i.notice_id AND s.section_id = i.value CROSS JOIN notice_texts tn ON tn.notice_id = i.notice_id AND tn.section_id = i.value AND tn.field_id = 'BT-500-Organization-TouchPoint' AND tn.ordinal = 0 CROSS JOIN notice_texts pn ON pn.notice_id = i.notice_id AND pn.section_id = p.mention_section_id AND pn.field_id = 'BT-500-Organization-Company' AND pn.ordinal = 0 WHERE t.id BETWEEN 94463 AND 97463 AND +p.mention_section_id LIKE 'ORG-%' AND +p.role IN ('Lot-ReviewOrg', 'Lot-ReviewInfo', 'Lot-AddInfo', 'Lot-TenderReceipt', 'Lot-DocProvider', 'Lot-Mediator', 'Lot-TenderEval', 'Lot-FiscalLegis', 'Lot-EnvironLegis', 'Lot-EmployLegis') AND +s.parent_section_id = p.mention_section_id AND +s.kind = 'Touchpoint' AND lower(tn.value) <> lower(pn.value) GROUP BY t.id, p.seq, p.role, p.organization_id, p.mention_notice_id, p.mention_section_id, i.value LIMIT 60
```

Local EXPLAIN QUERY PLAN (turso plan-probe):

```
# plan-copy
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH l USING INTEGER PRIMARY KEY (rowid=?)
SEARCH i USING INDEX sqlite_autoindex_notice_ids_1 (notice_id=? AND section_id=? AND field_id=?)
SEARCH s USING INDEX sqlite_autoindex_notice_sections_1 (notice_id=? AND section_id=?)
SEARCH tn USING INDEX sqlite_autoindex_notice_texts_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?)
SEARCH pn USING INDEX sqlite_autoindex_notice_texts_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?)
USE SORTER FOR GROUP BY
# plan-idx
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH l USING INTEGER PRIMARY KEY (rowid=?)
SEARCH i USING INDEX sqlite_autoindex_notice_ids_1 (notice_id=? AND section_id=? AND field_id=?)
SEARCH s USING INDEX sqlite_autoindex_notice_sections_1 (notice_id=? AND section_id=?)
SEARCH tn USING INDEX sqlite_autoindex_notice_texts_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?)
SEARCH pn USING INDEX sqlite_autoindex_notice_texts_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?)
USE SORTER FOR GROUP BY
```

### 14. a5

```sql
SELECT t.id AS tender_id, p.seq AS seq, p.role AS role, p.organization_id AS org_id, p.mention_notice_id AS notice_id, p.mention_section_id AS section_id, min(l.lot_key) AS lot_min, count(*) AS n_rows, i.value AS tpo, tn.value AS tp_name, pn.value AS parent_name FROM tenders t CROSS JOIN tender_version_parties p ON p.tender_id = t.id AND p.seq = t.current_seq CROSS JOIN lots l ON l.id = p.lot_id CROSS JOIN notice_ids i ON i.notice_id = p.mention_notice_id AND i.section_id = l.lot_key AND i.field_id = 'OPT-301-' || p.role AND +i.value LIKE 'TPO-%' CROSS JOIN notice_sections s ON s.notice_id = i.notice_id AND s.section_id = i.value CROSS JOIN notice_texts tn ON tn.notice_id = i.notice_id AND tn.section_id = i.value AND tn.field_id = 'BT-500-Organization-TouchPoint' AND tn.ordinal = 0 CROSS JOIN notice_texts pn ON pn.notice_id = i.notice_id AND pn.section_id = p.mention_section_id AND pn.field_id = 'BT-500-Organization-Company' AND pn.ordinal = 0 WHERE t.id BETWEEN 816222 AND 819222 AND +p.mention_section_id LIKE 'ORG-%' AND +p.role IN ('Lot-ReviewOrg', 'Lot-ReviewInfo', 'Lot-AddInfo', 'Lot-TenderReceipt', 'Lot-DocProvider', 'Lot-Mediator', 'Lot-TenderEval', 'Lot-FiscalLegis', 'Lot-EnvironLegis', 'Lot-EmployLegis') AND +s.parent_section_id = p.mention_section_id AND +s.kind = 'Touchpoint' AND lower(tn.value) <> lower(pn.value) GROUP BY t.id, p.seq, p.role, p.organization_id, p.mention_notice_id, p.mention_section_id, i.value LIMIT 60
```

Local EXPLAIN QUERY PLAN (turso plan-probe):

```
# plan-copy
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH l USING INTEGER PRIMARY KEY (rowid=?)
SEARCH i USING INDEX sqlite_autoindex_notice_ids_1 (notice_id=? AND section_id=? AND field_id=?)
SEARCH s USING INDEX sqlite_autoindex_notice_sections_1 (notice_id=? AND section_id=?)
SEARCH tn USING INDEX sqlite_autoindex_notice_texts_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?)
SEARCH pn USING INDEX sqlite_autoindex_notice_texts_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?)
USE SORTER FOR GROUP BY
# plan-idx
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH l USING INTEGER PRIMARY KEY (rowid=?)
SEARCH i USING INDEX sqlite_autoindex_notice_ids_1 (notice_id=? AND section_id=? AND field_id=?)
SEARCH s USING INDEX sqlite_autoindex_notice_sections_1 (notice_id=? AND section_id=?)
SEARCH tn USING INDEX sqlite_autoindex_notice_texts_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?)
SEARCH pn USING INDEX sqlite_autoindex_notice_texts_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?)
USE SORTER FOR GROUP BY
```

### 15. a6

```sql
SELECT t.id AS tender_id, p.seq AS seq, p.role AS role, p.organization_id AS org_id, p.mention_notice_id AS notice_id, p.mention_section_id AS section_id, min(l.lot_key) AS lot_min, count(*) AS n_rows, i.value AS tpo, tn.value AS tp_name, pn.value AS parent_name FROM tenders t CROSS JOIN tender_version_parties p ON p.tender_id = t.id AND p.seq = t.current_seq CROSS JOIN lots l ON l.id = p.lot_id CROSS JOIN notice_ids i ON i.notice_id = p.mention_notice_id AND i.section_id = l.lot_key AND i.field_id = 'OPT-301-' || p.role AND +i.value LIKE 'TPO-%' CROSS JOIN notice_sections s ON s.notice_id = i.notice_id AND s.section_id = i.value CROSS JOIN notice_texts tn ON tn.notice_id = i.notice_id AND tn.section_id = i.value AND tn.field_id = 'BT-500-Organization-TouchPoint' AND tn.ordinal = 0 CROSS JOIN notice_texts pn ON pn.notice_id = i.notice_id AND pn.section_id = p.mention_section_id AND pn.field_id = 'BT-500-Organization-Company' AND pn.ordinal = 0 WHERE t.id BETWEEN 570205 AND 573205 AND +p.mention_section_id LIKE 'ORG-%' AND +p.role IN ('Lot-ReviewOrg', 'Lot-ReviewInfo', 'Lot-AddInfo', 'Lot-TenderReceipt', 'Lot-DocProvider', 'Lot-Mediator', 'Lot-TenderEval', 'Lot-FiscalLegis', 'Lot-EnvironLegis', 'Lot-EmployLegis') AND +s.parent_section_id = p.mention_section_id AND +s.kind = 'Touchpoint' AND lower(tn.value) <> lower(pn.value) GROUP BY t.id, p.seq, p.role, p.organization_id, p.mention_notice_id, p.mention_section_id, i.value LIMIT 60
```

Local EXPLAIN QUERY PLAN (turso plan-probe):

```
# plan-copy
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH l USING INTEGER PRIMARY KEY (rowid=?)
SEARCH i USING INDEX sqlite_autoindex_notice_ids_1 (notice_id=? AND section_id=? AND field_id=?)
SEARCH s USING INDEX sqlite_autoindex_notice_sections_1 (notice_id=? AND section_id=?)
SEARCH tn USING INDEX sqlite_autoindex_notice_texts_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?)
SEARCH pn USING INDEX sqlite_autoindex_notice_texts_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?)
USE SORTER FOR GROUP BY
# plan-idx
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH l USING INTEGER PRIMARY KEY (rowid=?)
SEARCH i USING INDEX sqlite_autoindex_notice_ids_1 (notice_id=? AND section_id=? AND field_id=?)
SEARCH s USING INDEX sqlite_autoindex_notice_sections_1 (notice_id=? AND section_id=?)
SEARCH tn USING INDEX sqlite_autoindex_notice_texts_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?)
SEARCH pn USING INDEX sqlite_autoindex_notice_texts_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?)
USE SORTER FOR GROUP BY
```

### 16. x540072

```sql
SELECT t.id AS tender_id, p.seq AS seq, p.role AS role, p.organization_id AS org_id, p.mention_notice_id AS notice_id, p.mention_section_id AS section_id, min(l.lot_key) AS lot_min, count(*) AS n_rows, i.value AS tpo, tn.value AS tp_name, pn.value AS parent_name FROM tenders t CROSS JOIN tender_version_parties p ON p.tender_id = t.id AND p.seq = t.current_seq CROSS JOIN lots l ON l.id = p.lot_id CROSS JOIN notice_ids i ON i.notice_id = p.mention_notice_id AND i.section_id = l.lot_key AND i.field_id = 'OPT-301-' || p.role AND +i.value LIKE 'TPO-%' CROSS JOIN notice_sections s ON s.notice_id = i.notice_id AND s.section_id = i.value CROSS JOIN notice_texts tn ON tn.notice_id = i.notice_id AND tn.section_id = i.value AND tn.field_id = 'BT-500-Organization-TouchPoint' AND tn.ordinal = 0 CROSS JOIN notice_texts pn ON pn.notice_id = i.notice_id AND pn.section_id = p.mention_section_id AND pn.field_id = 'BT-500-Organization-Company' AND pn.ordinal = 0 WHERE t.id BETWEEN 540072 AND 540072 AND +p.mention_section_id LIKE 'ORG-%' AND +p.role IN ('Lot-ReviewOrg', 'Lot-ReviewInfo', 'Lot-AddInfo', 'Lot-TenderReceipt', 'Lot-DocProvider', 'Lot-Mediator', 'Lot-TenderEval', 'Lot-FiscalLegis', 'Lot-EnvironLegis', 'Lot-EmployLegis') AND +s.parent_section_id = p.mention_section_id AND +s.kind = 'Touchpoint' AND lower(tn.value) <> lower(pn.value) GROUP BY t.id, p.seq, p.role, p.organization_id, p.mention_notice_id, p.mention_section_id, i.value LIMIT 60
```

Local EXPLAIN QUERY PLAN (turso plan-probe):

```
# plan-copy
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH l USING INTEGER PRIMARY KEY (rowid=?)
SEARCH i USING INDEX sqlite_autoindex_notice_ids_1 (notice_id=? AND section_id=? AND field_id=?)
SEARCH s USING INDEX sqlite_autoindex_notice_sections_1 (notice_id=? AND section_id=?)
SEARCH tn USING INDEX sqlite_autoindex_notice_texts_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?)
SEARCH pn USING INDEX sqlite_autoindex_notice_texts_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?)
USE SORTER FOR GROUP BY
# plan-idx
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH l USING INTEGER PRIMARY KEY (rowid=?)
SEARCH i USING INDEX sqlite_autoindex_notice_ids_1 (notice_id=? AND section_id=? AND field_id=?)
SEARCH s USING INDEX sqlite_autoindex_notice_sections_1 (notice_id=? AND section_id=?)
SEARCH tn USING INDEX sqlite_autoindex_notice_texts_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?)
SEARCH pn USING INDEX sqlite_autoindex_notice_texts_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?)
USE SORTER FOR GROUP BY
```

### 17. s_mentions_b

```sql
SELECT notice_id, section_id, organization_id, name, country, scheme, raw_identifier FROM organization_mentions WHERE notice_id = 25284773 UNION ALL … (29 branches, one per notice; full text in raw/sql/s_mentions_b.sql)
```

Local EXPLAIN QUERY PLAN (turso plan-probe):

```
# plan-copy
COMPOUND QUERY
  LEFT-MOST SUBQUERY
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
  … the same SEARCH (notice_id=?) for each of the 29 branches
# plan-idx
COMPOUND QUERY
  LEFT-MOST SUBQUERY
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
  …
```

### 18. s_mentions_c

```sql
SELECT notice_id, section_id, organization_id, name, country, scheme, raw_identifier FROM organization_mentions WHERE notice_id = 23563650 UNION ALL … (29 branches, one per notice; full text in raw/sql/s_mentions_c.sql)
```

Local EXPLAIN QUERY PLAN (turso plan-probe):

```
# plan-copy
COMPOUND QUERY
  LEFT-MOST SUBQUERY
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
  … the same SEARCH (notice_id=?) for each of the 29 branches
# plan-idx
COMPOUND QUERY
  LEFT-MOST SUBQUERY
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
  …
```


