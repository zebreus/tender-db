# 587 unit 1 — measure: bare contracting-unit labels vs a body's department

2026-10-24 ~22:00Z. Measurer only: nothing written on the box. Inputs: census 2781 (`census_2781.json`,
`job_2781_summary.txt`), 19 public `name_prefix` listings + 1 tender listing (20/20 GETs), 7 bounded `/v1/sql`
statements (7/12). Every request is in `requests.log`; raw bodies under `raw/get/` and `raw/sql/` (each statement's
`.sql`, its local `EXPLAIN QUERY PLAN` on both schema copies `.plan`, and the answer `.out`). Numbers: `measure.json`
(built by `tools/measure.py` from `tools/classify.py`, `tools/names.py`). Sends were refused unless every plan step on
both copies was a SEARCH: `tender_version_parties_org`, `tender_version_parties_version`, `organization_mentions_org`,
plus INTEGER PRIMARY KEY (rowid) seeks on `tenders` (served version, `current_seq`) and `organizations` (buyer name).
Every answer carried `rows`/`columns`, none truncated, no 408 (0.37-1.96 s each).

## TL;DR

- **The census listing does not show the bare labels.** `name_phrase_census` returns `hits.into_values().take(cap)` over
  a `BTreeMap` keyed by org id (canonical.rs, pass 2 end), so the 500 rows are the **500 lowest org ids (884-79,376)**,
  not the largest: 491 are German `vergabestelle` rows, and 4 of the 500 are bare. Useless for sizing, so the bare
  population was enumerated from 19 prefix listings instead (6,010 orgs). The 4 bare census rows all appear there.
- **Bare population (lower bound): 2,823 orgs / 21,049 mentions** (BARE_STRICT 1,791 / 12,673; BARE_EXT 1,032 / 8,376).
  This is 9.4% of the census's 29,641 phrase orgs (2,784 hold a census phrase; 39 / 372 are "Service des marchés"
  forms without one). 2,543 of them have exactly one mention.
  - **The pooling risk sits in 212 identifier-less, name-keyed rows with a country: 16,746 mentions (80%).**
  - 2,278 identifier-less rows with a NULL country hold 3,094 mentions, nearly all singletons (one mention, one Tender
    each). They are not hubs: no reuse without a country.
  - 333 identified rows hold 1,209 mentions. They are keyed per identifier, so they serve one buyer each.
- **Distinct buyers.** 37 bare orgs were read: the top 15 by mentions, 7 extras (the Verify exhibit 9327844 plus one per
  language), and the next 15 name-keyed rows. Together they hold 15,755 of the 21,049 bare mentions (74.8%).
  - **W hub: 24 orgs / 12,594 mentions (80% of what was measured).**
  - Dominant buyer plus a pooled tail: 8 / 1,843.
  - N (one buyer or one administration): 5 / 1,318.
  - In the top 15 alone: 9 W (10,424 mentions), 4 dominant+tail (1,458), 2 N (1,105).
  - The 4 DEPT/OTHER controls ("… del Ayuntamiento de Vigo", "Vergabestelle Landeshauptstadt Stuttgart",
    "Unidad de Contratación del H. U. Ramón y Cajal", "Órgano de contratación de ADIF") each serve one buyer or one
    group, which confirms D576-1's accepted N.
- **Roles: the labels live on the ORG path, not the touchpoint path.** Over the 37 bare orgs, role-tenders split:
  - 10,287 (88.6%) TED R2 address/role-block roles: appeal-information 5,118, further-information 2,247, tender-receipt
    1,446, specifications-provider 803, review-body 209, mediation-body 109, buyer 355;
  - 816 (7.0%) eForms ORG-section roles;
  - **510 (4.4%) eForms touchpoint (TPO) roles**: Lot-AddInfo, Lot-ReviewOrg, Lot-TenderEval, Lot-ReviewInfo,
    Lot-DocProvider.
  - Mentions of the 22 orgs read in s03: ORG 13,040 / TPO 599 / ND- 292.
  - So 586 D2's touchpoint gate alone cannot retire these hubs. The ORG path (585's question) carries about 95%.

## (1) Classification

Fold = `partyname::fold` ported (lowercase, alnum runs, `store::fold_latin`). The census phrase(s) are removed as whole
words, then articles/prepositions, a generic suffix or contact/legal word (`service`, `département`, `secrétariat`,
`responsable`, `team`, `afd`, `abteilung`, `sa`, `gmbh` …) and, for BARE_EXT, generic procurement-function words
(`achats`, `marchés`, `contrats`, `juridique`, `logistique`, `compras`, `suministros`, `patrimonio`, `convenios`,
`asuntos generales`, `administrativa`, `zaopatrzenia`, `inkoop`, `aanbestedingszaken`, `contratti`, `appalti`,
`hôtel`/`mairie`/`ville` without a place …; `tools/classify.py` has the lists).
- **BARE_STRICT**: nothing is left (the folded name IS the phrase plus noise or a generic suffix).
- **BARE_EXT**: only generic function words are left ("Servicio de Contratación y Compras", "Dział Zamówień Publicznych
  i Zaopatrzenia", "Direction des achats et de la commande publique"). These name no body either.
- **DEPT** (D576-1's accepted N): a named body is left ("Ville de Lyon - Direction de la commande publique", "Stadt
  Erlangen - Zentrale Vergabestelle").
- **OTHER_ORGAN**: "Mesa / Órgano de contratación de|del X" (the organ of a named body, the task's OTHER example).

### Census 2781 rows (capped listing)

| class | orgs | mentions |
| --- | --- | --- |
| BARE_STRICT | 4 | 1,338 (3443 "Service de la Commande publique" 1,026; 19553 "Zentrale Vergabestelle" 293; 55205 "Vergabestelle" 17; 8981 "Unidad de Contratación" 2, identified) |
| DEPT | 496 | 59,150 |

By first phrase: vergabestelle 174 rows (1 bare), zentrale vergabestelle 317 (1 bare), ufficio gare 5, one each for
service de la commande publique (bare), direction des achats, unidad de contratacion (bare), direction de la commande
publique. 4 rows are served as winners on 1–7 Tenders. All 4 are buyer departments under their own name (Stadt Aachen,
Bochum, Senatsverwaltung Berlin, "Vergabestelle tp"), not companies. The 500 rows cover 1.7% of the 29,641 orgs and are
the lowest ids, so no estimate rests on them.

### Uncapped: 19 `name_prefix` listings (public GET, prefix on `name_norm`, limit 1000)

Prefixes: direction de la commande publique, service de la commande publique, service des march, direction des achats,
service des achats, unidad de contrataci, servicio de contrataci, mesa de contrataci, órgano / organo de contrataci,
dział zamówień, afdeling inkoop, afd. inkoop, zentrale vergabestelle, vergabestelle, procurement department, purchasing
department, procurement team, ufficio gare. Only g01 (direction de la commande publique) hit the 1000 cap. The 736
exact-name rows sort first; the tail after "… et des achats mairie d'orl…" is unseen, so DCP's BARE_EXT count is a
lower bound.

| class | orgs | mentions | identified | no id + NULL country | no id + country (name-keyed) |
| --- | --- | --- | --- | --- | --- |
| BARE_STRICT | 1,791 | 12,673 | 304 / 916 | 1,433 / 2,089 | **54 / 9,668** |
| BARE_EXT | 1,032 | 8,376 | 29 / 293 | 845 / 1,005 | **158 / 7,078** |
| DEPT | 2,712 | 35,856 | 304 / 7,410 | 510 / 5,556 | 1,898 / 22,890 |
| OTHER_ORGAN | 475 | 11,565 | 9 / 406 | 21 / 93 | 445 / 11,066 |

Bare mention histogram: 2,543 orgs with 1 mention, 190 with 2-9 (657), 59 with 10-99 (2,225), **31 with 100+ (15,624)**.
Example: "Direction de la commande publique" exists as 736 rows: one name-keyed FR row, 1166638, holds 1,356 mentions;
643 NULL-country singletons; 90 identified FR rows (105 mentions).

**Estimate per census phrase** (bare orgs found / census `by_phrase` orgs; a lower bound, because prefix-anchored
listings miss bare labels with a leading article or code, e.g. "La Direction …", "DCP - …"):

| phrase | census orgs | bare orgs | bare share | bare mentions | name-keyed bare rows | their mentions |
| --- | --- | --- | --- | --- | --- | --- |
| direction de la commande publique | 4,877 | 880 | 0.18 | 2,508 | 20 | 1,624 |
| service de la commande publique | 2,117 | 486 | 0.23 | 1,910 | 26 | 1,409 |
| service des marches publics | 2,860 | 605 | 0.212 | 1,708 | 24 | 1,080 |
| direction des achats | 5,054 | 220 | 0.044 | 1,050 | 51 | 512 |
| service des achats | 1,057 | 433 | 0.41 | 2,035 | 13 | 1,295 |
| unidad de contratacion | 541 | 81 | 0.15 | 928 | 5 | 795 |
| servicio de contratacion | 1,672 | 49 | 0.029 | 7,727 | 31 | 7,605 |
| mesa de contratacion | 414 | 3 | 0.007 | 97 | 3 | 97 |
| organo de contratacion | 389 | 12 | 0.031 | 292 | 3 | 264 |
| dzial zamowien publicznych | 236 | 3 | 0.013 | 434 | 3 | 434 |
| zespol zamowien publicznych | 16 | 0 | 0.0 | 0 | 0 | 0 |
| biuro zamowien publicznych | 125 | 0 | 0.0 | 0 | 0 | 0 |
| afdeling inkoop | 269 | 4 | 0.015 | 190 | 4 | 190 |
| team inkoop | 12 | 0 | 0.0 | 0 | 0 | 0 |
| zentrale vergabestelle | 4,095 | 9 | 0.002 | 898 | 2 | 550 |
| vergabestelle | 8,535 | 16 | 0.002 | 1,408 | 5 | 773 |
| procurement department | 629 | 5 | 0.008 | 81 | 4 | 76 |
| purchasing department | 428 | 2 | 0.005 | 11 | 1 | 8 |
| procurement team | 195 | 1 | 0.005 | 49 | 1 | 49 |
| ufficio gare | 387 | 9 | 0.023 | 304 | 6 | 286 |

Total: **2,784 of 29,641 phrase orgs are bare (9.4%)**. Bare share is high for the French unit phrases (18–41%),
low for the German (0.2%: the DE convention is "<Body> - Zentrale Vergabestelle"), and the Spanish "servicio de
contratación" has few bare rows (49) but the largest mass (7,727 mentions, 31 name-keyed rows).

## (2) Distinct buyers beside each bare label

For each org: its first 200 distinct served (tender, seq) pairs (`tender_version_parties_org` seek; `current_seq` via
the tenders PK; LIMIT 200 inside the arm), then those versions' `Procedure-Buyer` / `buyer` rows
(`tender_version_parties_version`) and the buyer's served name (organizations PK). "Buyers" counts distinct folded
buyer names (distinct buyer org ids in brackets); "top share" is the largest buyer's share of the buyer-tender mass;
"self" counts sampled Tenders on which the label itself is the buyer. Verdicts: W = many unrelated buyers; DOMINANT+TAIL
= one buyer ≥ 80% plus unrelated others; N = one buyer or one administration. Hand reads override the folded-name
rule where spellings of one body split it (listed in `measure.json` `verdict_hand`). Role-tenders by mention path
(ORG / TPO / ND-) come from s01/s06.

| group | org | name | class | key | mentions | ORG/TPO/ND role-tenders | sampled | buyers (ids) | top share | self | verdict |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| top15 | 1164742 | Servicio de Contratación | BARE_STRICT | name+ES | 3,236 | 2052/14/0 | 200 | 43 (51) | 0.1 | 0 | W |
| top15 | 1171563 | Servicio de Contratación Administrativa | BARE_EXT | name+ES | 2,060 | 1068/1/0 | 200 | 22 (31) | 0.43 | 0 | W |
| top15 | 1166638 | Direction de la commande publique | BARE_STRICT | name+FR | 1,356 | 1253/4/0 | 200 | 34 (35) | 0.174 | 0 | W |
| top15 | 3443 | Service de la Commande publique | BARE_STRICT | name+FR | 1,026 | 956/28/0 | 200 | 55 (58) | 0.196 | 1 | W |
| top15 | 1181455 | Service des marchés publics | BARE_STRICT | name+FR | 852 | 807/0/0 | 200 | 49 (49) | 0.255 | 0 | W |
| top15 | 1165758 | Service des achats et de la commande publique | BARE_EXT | name+FR | 785 | 779/0/0 | 200 | 2 (2) | 0.5 | 0 | N |
| top15 | 2997642 | Zentrale Vergabestelle | BARE_STRICT | name+DE | 545 | 448/22/1 | 187 | 43 (46) | 0.424 | 29 | W |
| top15 | 1241271 | Servicio de Contratación y Compras | BARE_EXT | name+ES | 541 | 290/12/0 | 200 | 15 (23) | 0.32 | 0 | W |
| top15 | 1223007 | Unidad de Contratación | BARE_STRICT | name+ES | 410 | 155/104/0 | 200 | 36 (43) | 0.256 | 0 | W |
| top15 | 1214290 | Servicio de Contratación y Asuntos Generales | BARE_EXT | name+ES | 398 | 236/0/0 | 200 | 15 (25) | 0.19 | 0 | W |
| top15 | 3975457 | Dział Zamówień Publicznych | BARE_STRICT | name+PL | 390 | 223/91/0 | 200 | 8 (9) | 0.93 | 0 | DOMINANT+TAIL |
| top15 | 1244992 | Service des achats et approvisionnements | BARE_EXT | name+BE | 387 | 380/0/0 | 200 | 4 (7) | 0.895 | 0 | DOMINANT+TAIL |
| top15 | 1193769 | Servicio de Contratación y Convenios | BARE_EXT | name+ES | 383 | 135/0/0 | 135 | 5 (5) | 0.867 | 0 | DOMINANT+TAIL |
| top15 | 22506338 | Service des Achats Responsables | BARE_STRICT | id FRF2108000 | 320 | 320/0/0 | 107 | 5 (5) | 0.72 | 0 | N |
| top15 | 1397643 | Unidad de Contratación Administrativa | BARE_EXT | name+ES | 298 | 172/1/0 | 165 | 8 (10) | 0.891 | 0 | DOMINANT+TAIL |
| extra | 9327844 | Dział Zamówień Publicznych i Zaopatrzenia | BARE_EXT | name+PL | 37 | 4/17/0 | 20 | 3 (3) | 0.75 | 0 | W |
| extra | 1369148 | Órgano de Contratación | BARE_STRICT | name+ES | 231 | 153/5/0 | 152 | 70 (73) | 0.107 | 0 | W |
| extra | 2005009 | Ufficio gare | BARE_STRICT | name+IT | 156 | 91/1/0 | 90 | 20 (34) | 0.467 | 0 | W |
| extra | 3801469 | Afdeling Inkoop | BARE_STRICT | name+NL | 121 | 38/47/0 | 73 | 6 (7) | 0.466 | 0 | W |
| extra | 19463077 | Afd. Inkoop | BARE_EXT | name+NL | 33 | 5/15/0 | 20 | 1 (1) | 1.0 | 0 | N |
| extra | 1416875 | Procurement Department | BARE_STRICT | name+GB | 73 | 39/0/0 | 35 | 23 (23) | 0.147 | 6 | W |
| extra | 19553 | Zentrale Vergabestelle | BARE_STRICT | name, NULL country | 293 | 2/0/291 | 200 | 0 (0) | - | 200 | W |
| control | 1606217 | Unidad de Contratación del Hospital Universitario Ramón  | DEPT | name+ES | 726 | 324/0/0 | 200 | 4 (15) | 0.355 | 0 | N |
| control | 1182792 | Órgano de contratación de ADIF | OTHER_ORGAN | name+ES | 4,080 | 2924/0/0 | 200 | 15 (23) | 0.593 | 1 | N |
| control | 5766 | Vergabestelle Landeshauptstadt Stuttgart | DEPT | id T4971121689746 | 2,003 | 1961/0/0 | 200 | 1 (1) | 1.0 | 192 | N |
| control | 1515541 | Servicio de Contratación del Ayuntamiento de Vigo | DEPT | name+ES | 283 | 202/0/0 | 200 | 1 (3) | 1.0 | 0 | N |
| next15 | 1768256 | Servicio de Contratación y Suministros | BARE_EXT | name+ES | 217 | 79/67/0 | 138 | 5 (9) | 0.413 | 0 | W |
| next15 | 1804271 | Vergabestelle | BARE_STRICT | name+DE | 209 | 154/22/0 | 137 | 31 (31) | 0.328 | 15 | W |
| next15 | 1229388 | Servicio de Contratación y Patrimonio | BARE_EXT | name+ES | 204 | 141/4/0 | 140 | 28 (38) | 0.143 | 0 | W |
| next15 | 1340282 | Direction des achats | BARE_STRICT | name+FR | 126 | 114/3/0 | 99 | 20 (22) | 0.229 | 10 | W |
| next15 | 4154069 | Service de la commande publique: responsable du service | BARE_STRICT | name+FR | 122 | 63/0/0 | 63 | 4 (4) | 0.924 | 0 | DOMINANT+TAIL |
| next15 | 1575233 | Servicio de Contratacion | BARE_STRICT | name+ES | 122 | 64/0/0 | 64 | 7 (8) | 0.781 | 0 | W |
| next15 | 1378491 | Servicio de Contratación Centralizada | BARE_EXT | name+ES | 113 | 69/7/0 | 76 | 12 (23) | 0.303 | 0 | W |
| next15 | 1168433 | Service des marchés | BARE_EXT | name+BE | 100 | 99/0/0 | 89 | 11 (11) | 0.573 | 0 | W |
| next15 | 7229588 | Servicio de Contratación y Gestión Patrimonial | BARE_EXT | name+ES | 100 | 54/0/0 | 54 | 4 (10) | 0.926 | 0 | DOMINANT+TAIL |
| next15 | 1228608 | Direction de la commande publique - service des marchés | BARE_EXT | name+FR | 90 | 90/0/0 | 46 | 2 (2) | 0.978 | 0 | N |
| next15 | 1204150 | Servicio de Contratación, Compras y Patrimonio | BARE_EXT | name+ES | 90 | 47/0/0 | 47 | 1 (4) | 1.0 | 0 | N |
| next15 | 1193704 | Mesa de contratación | BARE_STRICT | name+ES | 85 | 67/0/0 | 35 | 12 (12) | 0.286 | 0 | W |
| next15 | 1363144 | Service des marchés | BARE_EXT | name+FR | 83 | 79/1/0 | 71 | 30 (30) | 0.325 | 1 | W |
| next15 | 1163163 | Service de la commande publique et des marchés | BARE_EXT | name+FR | 82 | 82/0/0 | 45 | 3 (4) | 0.867 | 0 | DOMINANT+TAIL |
| next15 | 2680514 | Unidad de Contratacion | BARE_STRICT | name+ES | 81 | 3/44/0 | 46 | 4 (4) | 0.886 | 2 | DOMINANT+TAIL |

Reading the hubs (top buyers from `buyers_top8`):
- **1164742 "Servicio de Contratación"** (ES): 43 buyers in 200 Tenders, none above 10%. The top ones are CH Tajo,
  Diputación Foral de Bizkaia, SERGAS, U. Alcalá, UPV and Ayuntamiento de Madrid. 2,052 of its role-tenders are
  appeal-information (TED R2 VI.4.3).
- **1166638 "Direction de la commande publique"** (Verify exhibit, FR): 34 buyers. Top: Ville de Levallois 35, Grand
  Toulouse 26, Nantes Métropole 23, CG Eure-et-Loir 14, Villeurbanne 13. Roles: tender-receipt 469, further-information
  345, appeal-information 225, specifications-provider 193.
- **1223007 "Unidad de Contratación"** (Verify exhibit, ES): 36 buyers. Top: Intendente de Rota 64, H.U. Ramón y Cajal
  19, Base Naval de Rota 16, Ayto. Córdoba 13. It is the only top-15 hub with a large TPO share: Lot-ReviewOrg on 88
  Tenders, 199 of 410 mentions TPO.
- **9327844 "Dział Zamówień Publicznych i Zaopatrzenia"** (Verify exhibit, PL): 3 unrelated buyers (Szpital
  Uniwersytecki nr 2 Bydgoszcz 15, Uzdrowisko Lądek-Długopole 4, SPWZOZ Stargard 1). 17 of its 21 role-tenders are TPO
  (Lot-TenderEval).
- **1369148 "Órgano de Contratación"**: 70 buyers in 152 Tenders, the purest hub (review-body 93, appeal-information
  29).
- **19553 "Zentrale Vergabestelle"** (NULL country, `doe` source, mentions on `ND-` sections): the label IS the buyer of
  record on 291 Tenders (200/200 sampled), with no other party on them (s04). g20's 100 titles name Wuppertal (67, the
  B/L/F/U-nnnn-yy file numbers), Leverkusen (15), Schwerte 2, Reepsholt 2, Aulendorf, Rheinzabern, Bad Segeberg, Klein
  Nordende-Lieth and Priemelsfehn. That is a W hub of a different shape: there is no own buyer in the notice to alias
  to.
- **Dominant + tail.**
  - 3975457 "Dział Zamówień Publicznych": Uniwersytet Jagielloński 186/200, plus 7 others (NIO, Instytut Kardiologii, …).
  - 1244992: ISPPC Charleroi 188 in two spellings, plus Humani 11 and Tauron 1.
  - 1397643: SERMAS Atención Primaria 147/165, plus 7 others.
  - 4154069, 7229588, 1163163 and 2680514 follow the same pattern.
- **N.**
  - 1165758 "Service des achats et de la commande publique": Ville de Strasbourg + CU de Strasbourg, 100/100 Tenders each
    (the shared service).
  - 22506338 "Service des Achats Responsables": identified FRF2108000; Ardenne Métropole 77 + Charleville-Mézières 24.
  - 19463077 "Afd. Inkoop" (576 U4 item 27): one buyer, Stichting 113 Zelfmoordpreventie. It is single-publisher, but
    576 U4 saw "Afd. Inkoop" pooled across buyers; 3801469 "Afdeling Inkoop" is that hub (6 buyers: Cyclus, Horizon
    College, U. Maastricht, GenomeScan …).
  - 1228608 and 1204150: one buyer each.
- **Side observation, not verified:** beside 1193769 "Servicio de Contratación y Convenios" (appeal-information), the
  served buyer on 117 of 135 Tenders is "Tribunal Administrativo de Recursos Contractuales de Andalucía". A review
  tribunal served as Procedure-Buyer/buyer looks like a 584-shaped misbinding. Worth a separate read.

## (3) Roles: touchpoint vs ORG path

Over the 37 bare orgs (role-tenders: one Tender counted once per role and path, s01 + s06):

| path | role family | role-tenders | roles |
| --- | --- | --- | --- |
| ORG (TED R2 sections) | address/role blocks | 10,287 (88.6%) | appeal-information 5,118 · further-information 2,247 · tender-receipt 1,446 · specifications-provider 803 · review-body 209 · mediation-body 109 · buyer 63 (+292 on `ND-`) |
| ORG (eForms ORG-nnnn) | eForms party roles | 816 (7.0%) | Lot-AddInfo · Lot-TenderReceipt · Lot-DocProvider · Procedure-SProvider 84 · Lot-ReviewInfo · Procedure-Buyer 8 |
| TPO (eForms touchpoint, D576-1 (ii) minted or bound) | touchpoint roles | 510 (4.4%) | Lot-AddInfo · Lot-ReviewOrg (102, mostly 1223007) · Lot-TenderEval (49) · Lot-ReviewInfo · Lot-DocProvider · Lot-TenderReceipt |

- **TPO-heavy labels.** These are the ones the touchpoint gate can reach: 1223007 Unidad de Contratación 104 TPO
  role-tenders, 3975457 Dział Zamówień Publicznych 91, 1768256 Servicio de Contratación y Suministros 67, 3801469
  Afdeling Inkoop 47, 2680514 Unidad de Contratacion 44, 3443 28, 2997642 22, 1804271 Vergabestelle 22, 9327844 17,
  19463077 Afd. Inkoop 15, 1164742 14, 1241271 12.
- **Bare labels as buyer**: `buyer` on 355 role-tenders. Of these, 292 are on `ND-` (291 are 19553/DÖE). The 63 TED
  ones are 2997642 (27), 1340282 (10), 1804271 (8), 1166638 (6), 1416875 (6), 3443 (3), 2680514 (2) and 1363144 (1).
  `Procedure-Buyer` adds 8 more (1804271 7, 2997642 1).
- **Across all 29,641 phrase orgs** (the census `roles`, departments included): TED R2 legacy roles 198,059 vs eForms
  roles 50,755 distinct (role, Tender) pairs.
- **Conclusion for the decision.** A whole-name touchpoint list under 586 D2 stops the TPO share only, which is about
  4% here. The hubs are made on the ORG path, chiefly TED R2.0.x address blocks: VI.4.3 appeal service; Annex A I/II/III
  further information, tenders to, documents from. The publisher prints its contracting unit as OFFICIALNAME with its
  own address, the same shape as 585's TenderNed labels.
  - For those roles the notice's own buyer is the natural alias target: the label is that buyer's unit. Exceptions are
    appeal-information, where a separate review body can also be printed, and the DÖE buyer-of-record shape (19553),
    which has no other buyer.

## Whole-name candidates (folded, from the measured hubs; for unit 2 to decide)

`servicio de contratacion`, `servicio de contratacion administrativa`, `direction de la commande publique`, `service de
la commande publique`, `service des marches publics`, `service des marches`, `zentrale vergabestelle`, `vergabestelle`,
`servicio de contratacion y compras`, `unidad de contratacion`, `servicio de contratacion y asuntos generales`,
`dzial zamowien publicznych`, `dzial zamowien publicznych i zaopatrzenia`, `organo de contratacion`, `mesa de
contratacion`, `ufficio gare`, `afdeling inkoop`, `afd inkoop`, `procurement department` (already in 586 WHOLE,
touchpoint only), `direction des achats`, `servicio de contratacion y suministros`, `servicio de contratacion y
patrimonio`, `servicio de contratacion centralizada`, `service de la commande publique responsable du service`.
- Do not list as whole names:
  - "Service des achats et de la commande publique" (Strasbourg), "Service des Achats Responsables" (Ardenne, identified)
    and "Servicio de Contratación, Compras y Patrimonio": single-administration rows, N.
  - A bare label carrying an identifier: keyed per identifier, so one buyer.
- A generic-vocabulary rule (BARE_EXT) generalises better than a list, but it would need its own salted 0-W read.

## Caveats

- **Listings.** They are prefix-anchored (lower bound), and g01 was truncated at 1000.
- **Classifier.** It is vocabulary-based. A residual person or street address (e.g. "Service des marchés publics Eric
  Baleine", "Direction de la commande publique 1 place Salvador Allende 94010 Créteil") falls to DEPT. These are small
  (single digits to tens of mentions each).
- **The 200-Tender sample** is the first 200 served (tender, seq) pairs in `tender_version_parties_org` order, so older
  Tenders come first. For 25 of the 41 orgs it is the whole served set (sampled < 200).
- **Buyer counts** are distinct folded served names. Two spellings of one body count twice (hand overrides in
  `verdict_hand`). Distinct org ids also include provisional duplicates.
- **Roles** are counted on the served version only. Role-tenders count a Tender once per role and path.
- **Budget.** SQL 7/12 (s03_mentions, s01a_roles, s01b_roles, s02_buyers, s04_selfbuyer, s05_buyers_next15,
  s06_roles_next15). GET 20/20 (g01–g19 listings, g20 `/v1/tenders?buyer=19553&limit=100`). No retries, no 408/429.
