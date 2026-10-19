# 530 R2: city-flagged targets, items 28-40 (read_city3)

Source: `guid_twin_plan_2659.json` `body.r2_city_flagged[28..40]` (0-based), the last 13 of the 41 keys the A6 city screen
flagged. Seven are DE, five are FR courts (four tribunaux administratifs and one tribunal judiciaire), and one is PT. Twelve have an existing identifier-NULL target. Item 34
(`]init[ DCP LDA`, PT) is a missing key: Build 2 would mint one row for its planned mentions.

**Question.** Is the target one body (**R**), unclear (**N**), or a homonym or umbrella that collects different bodies (**W**)?
Under A6, a target that already holds two bodies scores W, even when the folded mentions are right.

**Skip rule** (as in read_city1 and read_city2). An item that another R2 reader has already read is cited, not read again. That
covers a top target, an office two-word key, and the target of an `r2_random` row. None of items 28-40 is in `r2_top_targets` or
`r2_office_two_words`, and none is the target of an `r2_random` row, so I read all 13. One name is also in `r2_incoherent` (A7):
`ikk classic` (item 29) is incoherent groups 16 and 20. The incoherent reader scores that group's refusal; I score only the target
here and record what the notice shows.

**Method.**
- `/v1/organizations/{target}`: every target has identifier NULL and provisional true, and its mention count matches the report.
- The target's existing mentions: the three newest notices the report lists (`notice_ids`), through `/v1/notices/{id}/content`.
  For each I took name, street, postcode, town, e-mail and roles (OPT-300/301, DE1 party references, TED address roles).
- Planned rows, located in one of three ways:
  - `planned_mention_keys`: only item 38 (KUBUS) is there, because the list is cut at 2,000 keys (notices up to 24341274);
  - `r2_incoherent`: item 29 (IKK), whose group `org_ids` are exactly its 2 planned rows (4 mentions);
  - probing `/v1/organizations/{id}` on `planned_row_ids` above 23.4M, where the list thins out. This found item 34's sample row
    22942473 and its twin 22942474. Beside them are the Deloitte rows of items 30 and 31: 22942475/80 and 22942476/77. It also found two of the four
    Lyon rows (item 28), 30961121/22.
- No allowed endpoint gives a planned row's notices, so the probe-found rows have names, countries and counts but no notice text.
  For items 32, 33, 35-37, 39 and 40, the planned side is only the report's `planned_cities`.

**GETs.** 70 public GETs, all 200 and none 408: 30 organizations and 40 notice contents.
- They are logged as `reader=city3` in `api_requests.log`, 18:09:55Z-18:15:12Z, behind the shared `.api.lock`/`.api.last` pacing (at least
  2.1 s after any reader's last GET).
- Bodies are in `bodies/`.
- 13 GETs were probes of `planned_row_ids` and their neighbours. Four of them hit rows outside items 28-40 (see the end).

## Tally

**Items 28-40 (13 read): R 13, N 0, W 0.** None is duplicated elsewhere. The A6 flags here are all false alarms:
- spelling variants of one town: CEDEX forms, `19061 schwerin`, the typo `stutttgar`;
- or two offices of one legal entity: IKK classic in Dresden and Erfurt, and the Deloitte, Cassini and Drees & Sommer offices.

Seven verdicts rest on the target and the name only, because the planned rows were not located: items 32, 33 and 39, and the FR
items 35-37 and 40 (one court per name). Item 34's rows were located, but not their notice.

| # | target | name | mentions | planned rows / mentions | target cities → planned cities | flags | verdict |
|---|---|---|---|---|---|---|---|
| 28 | 1163241 | Tribunal administratif de Lyon | 46,473 | 4 / 4 | lyon, lyon 03, lyon cedex, lyon cedex 03 → lyon | b | R |
| 29 | 1206438 | IKK classic | 2,117 | 2 / 4 | dresden, erfurt → dresden | b | R |
| 30 | 1289810 | Deloitte Consulting GmbH | 135 | 2 / 4 | dusseldorf → berlin, munchen | a 2, c | R |
| 31 | 8808564 | Deloitte GmbH Wirtschaftsprüfungsgesellschaft | 99 | 2 / 4 | munchen → dusseldorf, munchen | a 1, c | R |
| 32 | 13918251 | Cassini Consulting AG | 26 | 2 / 4 | berlin, dusseldorf → dusseldorf | b | R |
| 33 | 13957982 | Drees & Sommer SE | 959 | 2 / 4 | berlin, erfurt, koln, stutttgar → berlin | b | R |
| 34 | (missing; rows 22942473/74) | ]init[ DCP LDA (PT) | 0 | 2 / 4 | — → lissabon, porto | c | R |
| 35 | 1163111 | Tribunal administratif de Cergy-Pontoise | 35,410 | 2 / 2 | cergy pontoise, cergy pontoise cedex → cergy pontoise | b | R |
| 36 | 1163191 | Tribunal administratif de Toulon | 22,269 | 2 / 2 | toulon, toulon cedex 9, toulon cedex9 → toulon | b | R |
| 37 | 1163864 | Tribunal administratif de Montpellier | 27,192 | 2 / 2 | montpellier, montpellier cedex 2 → montpellier | b | R |
| 38 | 2126721 | KUBUS Kommunalberatung und Service GmbH | 131 | 1 / 2 | schwerin → 19061 schwerin | a 1 | R |
| 39 | 8545951 | BIB Augsburg gGmbH | 60 | 1 / 2 | augsburg → gersthofen | a 2 | R |
| 40 | 14658291 | Tribunal judiciaire de Marseille | 2,223 | 2 / 2 | marseille, marseille cedex 06 → marseille | b | R |

## Items

**28 — 1163241 Tribunal administratif de Lyon — R.** One administrative court. The target has 46,473 mentions. Its newest are
45870943 and 45864375 (TED 00471272-2026 and 00466526-2026, AMO réhabilitation énergétique, 2026-07) and 44512113 (TED
00386839-2025, Commissariat de Villeurbanne maintenance, 2025-06). Each prints the court once, as ORG-0002 at 184 rue Duguesclin,
69003 LYON (or 69433 Lyon cedex 03), url lyon.tribunal-administratif.fr, greffe.ta-lyon@juradm.fr. That one section carries
Lot-ReviewOrg, Lot-ReviewInfo and Lot-Mediator. Its BT-501 is the section's own id `ORG-0002`, not a key, which is why these
mentions sit on the identifier-NULL row. Planned side: the probe found rows 30961121 and 30961122, `Tribunal administratif de Lyon`,
FR, 1 mention each (the FR copies have no DÖE twin). Their identifiers are `31EFEB12…` and `31EFEB23…`. They share a prefix, so the
platform's generator is sequential, but each row is used once. 30961124 and 30961125 are the adjacent planned ids, presumably the
other two of the 4 rows, but I did not GET them. Flag b: the four "cities" are Lyon with and without its CEDEX and arrondissement
suffixes. Only one body is called "Tribunal administratif de Lyon"; the bare `Tribunal administratif` is already in
`GUID_TWIN_UNSAFE_NAMES`.

**29 — 1206438 IKK classic — R.** One statutory health insurer (Körperschaft). The target has 2,117 mentions. Its newest are
47401851 and its TED copy 47393440 (00710250-2026, Reinigungsdienstleistungen Bayern, 2026-10), and 47363720 (00688381-2026, §
73b Bereinigung). Each prints IKK classic at Mittelhäuser Straße 68, 99089 Erfurt, vergabestelle@ikk-classic.de, as
Procedure-Buyer + Lot-AddInfo, which is the insurer's procurement unit. Planned rows 22820369 and 22820370 hold 2 mentions each.
They are the `r2_incoherent` 16/20 group, in TED 00703483-2024 (notice 24345339, 2024-11-18, Open-House
Arzneimittelrabattverträge § 130a Abs. 8 SGB V) and its DÖE copy 26508153. ORG-0001 (`f4993229…`, Procedure-Buyer + Lot-AddInfo)
and ORG-0002 (`bb78567a…`, Lot-ReviewOrg) are both "IKK classic", 01099 Dresden, open-house@ikk-classic.de, with the same phone
(0800 455 1111). So the "review body" section is the insurer itself: an Open-House procedure names no Vergabekammer, and the
platform filled the slot with the buyer. That is one body printed per role, which bears on the incoherent reader's R/W for groups
16 and 20. Flag b: Dresden is the head office and Erfurt the procurement office, both of one insurer, and no other body bears the
name.

**30 — 1289810 Deloitte Consulting GmbH — R.** One company. The target has 135 mentions. Its newest are 27624868 and its TED copy
27592748 (00549947-2026, Rahmenvertrag Unternehmensberatung, 2026-08) and 26926735 (2026-04, GenAI Strategieberatung). Each prints
Deloitte Consulting GmbH as Tenderer at 40476 Düsseldorf (Schwannstraße 6, consultingausschreibung@deloitte.de). Planned rows
22942476 and 22942477, `Deloitte Consulting GmbH`, DE, hold 2 mentions each, which is exactly the report's 2 rows / 4 mentions.
They sit in one id run with item 31's rows, item 34's rows, `]init[ AG für Digitale Kommunikation` (22942472, not planned) and
`Explicatis GmbH` (22942478, not planned). So it is one award notice (a TED copy and a DÖE copy) that lists each bidder twice at
different offices. By id adjacency the notice is near 24.50M (late 2024); it was not located. Flags a and c: the planned Berlin
and München mentions are two Deloitte Consulting offices, and the target's Düsseldorf is its seat. Same treatment as GIZ's two
seats (top 3) and vdek's state offices (city1 item 11): the name denotes no second organisation.

**31 — 8808564 Deloitte GmbH Wirtschaftsprüfungsgesellschaft — R.** One audit firm. The target has 99 mentions. Its newest is
26985701 (2026-06, Abschlussprüfung der HGV 2026ff): ORG-0003, Tenderer, Rosenheimer Platz 4, 81669 München,
ausschreibungen@deloitte.de. Its printed BT-501 is `DE129274148`, evidently not taken as a key, as with Leinemann in city1 item 3.
The next two are the DÖE notice 26529959 (DORA-Beratungsleistung, 2024-12; the same München address and e-mail) and the 2023 SDK01
award 26146632 (Beschaffungsamt des BMI, Software Asset Management; winner at Rosenheimer Platz 4, München). Planned rows: 22942475
and 22942480 (the second printed with a double space, `Deloitte GmbH  Wirtschaftsprüfungsgesellschaft`, which folds to the same
key). They hold 2 mentions each, in the same award notice as item 30. Flags a and c: the planned Düsseldorf is a Deloitte office,
and München is the seat the target already holds. One legal entity.

**32 — 13918251 Cassini Consulting AG — R (planned rows not located).** One company. The target has 26 mentions. Its newest are
30107987 and its TED copy 29923057 (00604737-2026, IT-Roadmap der Sächsischen Krankenhäuser, 2026-09): Tenderer at Invalidenstraße
74, 10557 Berlin (salma.nosseir@cassini.de). Then the DÖE award 26470005 (2024-10, Modernisierung des
Datenschutzmanagementsystems): Tenderer at Bennigsen-Platz 1, 40474 Düsseldorf, bid-office@cassini.de. Flag b: the target already
holds the Düsseldorf head office and the Berlin office. The planned rows (2 rows, 4 mentions, Düsseldorf) are past the key cut and
not in the probed tail. Their city is one the target already holds.

**33 — 13957982 Drees & Sommer SE — R (planned rows not located).** One company. The target has 959 mentions. Its newest are
47362307, 47358626 (TED 00683093-2026) and 47292407 (2026-10, Neubau Biophotonik-Gebäude VE 4710 and VE 4300). Each prints Drees &
Sommer SE as ORG-7005, 99084 Erfurt, robert.steffens@dreso.de, in the Lot-ReviewInfo slot: the buyer's project manager placed in
the review-information role, which is a role-quality issue, not a second body. Flag b: Berlin, Erfurt, Köln and "stutttgar" (a typo
of Stuttgart, the head office) are Drees & Sommer offices. The planned rows (2 rows, 4 mentions, Berlin) were not located. Their
city is one the target already holds.

**34 — missing key `]init[ dcp lda` (PT), rows 22942473/74 — R (rows located, notice not).** There is no identifier-NULL row, so
Build 2 would mint one row for these 4 mentions. The probe found the rows: 22942473 (`626BF4F0…`) and 22942474 (`C64B9450…`), both
`]init[ DCP LDA`, country PT, 2 mentions each. They sit directly after 22942472 `]init[ AG für Digitale Kommunikation` (DE, single
GUID, not planned), in the same award notice as the Deloitte rows of items 30 and 31. Flag c: the planned cities are Lissabon and
Porto, the German spelling of Lisbon in a DE notice. The notice prints the Portuguese company of the ]init[ group twice, at two
offices, the same way it prints each Deloitte company twice. A Portuguese "Lda" firm name is unique in its register, and nothing
else carries this name, so the new row would hold one company. This is weaker than the others: the notice text was not seen.

**35 — 1163111 Tribunal administratif de Cergy-Pontoise — R (planned rows not located).** One court. The target has 35,410
mentions. Its newest are 47362601 (00689132-2026, Médiateur national de l'énergie, 2026-10), 46861150 (00673177-2026, élagage Ville
de Bois-Colombes) and 30763746 (00614944-2026, assurances Bois-Colombes). Each prints the court at 95027 Cergy-Pontoise (Cedex),
greffe.ta-cergy-pontoise@juradm.fr, in Lot-ReviewOrg and/or Lot-ReviewInfo. Flag b: "cergy pontoise cedex" is the CEDEX form. The 2
planned FR rows were not located.

**36 — 1163191 Tribunal administratif de Toulon — R (planned rows not located).** One court. The target has 22,269 mentions. Its
newest are 47403900 (00715418-2026, prestations topographiques), 47401296 (00712843-2026, salins d'Hyères) and 46964269
(00676555-2026, SDIS du Var). Each prints the court at 5 rue (Jean) Racine, 83041 TOULON CEDEX 9 or 83000 Toulon
(greffe.ta-toulon@juradm.fr in 47401296), as Lot-ReviewOrg (+ ReviewInfo). Flag b: "toulon cedex 9" and "toulon cedex9" are CEDEX spellings.

**37 — 1163864 Tribunal administratif de Montpellier — R (planned rows not located).** One court. The target has 27,192 mentions.
Its newest are 45599491 (00638664-2026, Office de Tourisme de Saint-Cyprien) and 44551510 (00405074-2025, assurances Saint-Cyprien):
ORG-0003 at 6 rue Pitot, 34000 Montpellier, greffe.ta-montpellier@juradm.fr, as Lot-ReviewOrg + ReviewInfo + Mediator. The third,
27161228, is a TED-schema notice that prints the court three times (ORG-3/4/5: appeal body, mediation body, lodging information),
at 6 rue Pitot, 34063 Montpellier. Flag b: "montpellier cedex 2" is the CEDEX form.

**38 — 2126721 KUBUS Kommunalberatung und Service GmbH — R.** One company, a municipal procurement consultancy in Schwerin. The
target has 131 mentions. Its newest are 44533244 (00359144-2025, LF 20 for Kaltenkirchen and DLK for Bad Segeberg) and 30763643
with its copy 30644687 (00614908-2026, WLF Wyk auf Föhr). Each prints KUBUS at Bertha-von-Suttner-Str. 5, 19061 Schwerin, as the
municipalities' agent (Procedure-SProvider, Lot-TenderReceipt). The sister firm "Kubus Rechtsanwaltsgesellschaft mbH" in the same
notices has a different name_norm and stays out. The planned row 22238340 (`4934B784…`, 2 mentions) is ORG-0001 Procedure-Buyer in
TED 00725704-2023 (notice 23588596, 2023-11-28, Strombündelausschreibung "Bayern, Ökostrom", for the Bayerischer Gemeindetag) and
its DÖE copy 26218469, same address, info@kubus-mv.de. Its twin, ORG-0003 Lot-AddInfo (`27a676a5…`, energie@kubus-mv.de), is the
identifier of the stable KEEP row 11107 (KUBUS, 1,885 mentions), so it stays bound there. The fold sends only the fresh value to the
identifier-NULL row, never to 11107 (300 §10); the 11107/2126721 split itself is outside 530. Flag a: "19061 schwerin" is the
postcode typed into BT-513, the same town.

**39 — 8545951 BIB Augsburg gGmbH — R (planned row not located).** One non-profit training and integration provider. The target
has 60 mentions. Its newest are the DÖE award 26390095 (2024-07, Bundesagentur für Arbeit REZ Bayern, 701-24-AsAflex): ORG-0021
Tenderer, Memminger Str. 6, 86159 Augsburg, bib@bibaugsburg.de. Then two SDK01 awards, 26141974 (REZ Bayern, AA Weilheim, 2023-07,
reglin@bibaugsburg.de) and 26137156 (701-23-BvB, 2023-06), with BIB as winner of several lots. The planned row (1 row, 2 mentions:
a twin whose partner value is spared, probably by KEEP) is in Gersthofen. It is past the key cut and not in the probed tail.
Gersthofen borders Augsburg, and a BA-contracted provider runs course sites in the district. The name is a single gGmbH with no
second bearer. This is weaker than the others: the off-city mention itself was not seen.

**40 — 14658291 Tribunal judiciaire de Marseille — R (planned rows not located).** One court. The target has 2,223 mentions. Its
newest are 46699643 (00658973-2026, façades Nef A, Chantiers Navals de La Ciotat), 44523412 (00399457-2025, élévateur à sangles
400 t) and 31453484 (00627849-2026, espace d'exposition Port-Vieux). The buyers are the La Ciotat shipyard and port bodies, which
name the civil court as review body. Each prints Tribunal judiciaire de Marseille at 6 Rue Joseph Autran, 13006 Marseille,
tj-marseille@justice.fr, as Lot-ReviewOrg. Flag b: "marseille cedex 06" is the CEDEX form of the same 13006 address.

## By-products of the probes (outside items 28-40)

- **24502348 and 24503832** are `Freistaat Bayern, vertreten durch die Universität Erlangen-Nürnberg`, DE, 4 mentions each. Both
  are on `planned_row_ids`, and 24502349 is too. They are evidently planned rows of `r2_city_flagged[8]` (target 31365873, 10 rows /
  36 mentions), which read_city1 scored R with "planned rows not located". That item's planned side now has three located rows.
- **31534368** is `Stadt Nürnberg, Stadtplanungsamt` (2 mentions; 31534369 is also planned). **30912383** is `Stiftungsverwaltung
  Freiburg` (2 mentions). Neither is in items 28-40.

## Notes for the R2 record

- The A6 screen flags CEDEX variants (FR), a postcode typed into BT-513 (`19061 schwerin`) and plain typos (`stutttgar`) as
  separate cities, and so do the cities flagged in read_city1. Removing a trailing `cedex N` or arrondissement number and a leading
  5-digit postcode before comparing would clear 6 of these 13 flags (28, 35, 36, 37, 38, 40) without hiding a real homonym. The office cases (IKK, Deloitte ×2, Cassini, Drees &
  Sommer, BIB) still need a read.
- The tenderer twins in the Deloitte/]init[ notice print one company at two offices under two GUIDs. Folding them is right. A
  winner-side homonym would need two companies with one exact registered name in one country, and none appeared.
