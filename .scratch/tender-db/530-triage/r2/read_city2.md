# 530 R2: city-flagged targets, items 14-27 (read_city2)

Source: `guid_twin_plan_2659.json` `body.r2_city_flagged[14..27]` (0-based), 14 of the 41 targets the A6 city screen flagged.
Each one is a DE or FR identifier-NULL row that exists today, so none is a missing key.

**Question.** Is the target one body (**R**), unclear (**N**), or a homonym or umbrella that collects different bodies (**W**)?
Under A6, a target that already holds two bodies scores W, and so does a target that the fold would make hold two bodies.

**Skip rule.** I applied it as: an item that another R2 reader has already read (a top target, an office two-word key, or the
target of an `r2_random` row) is cited, not read again. None of items 14-27 is in `r2_top_targets` or `r2_office_two_words`, and
none is the target of an `r2_random` row, so I read all 14. None of the 14 names appears in `r2_incoherent` (A7).

**Method.**
- `/v1/organizations/{target}`: every target has identifier NULL and provisional true, with the report's mention count unchanged.
- The target's existing mentions: the notices the report lists for the target (`notice_ids`), through `/v1/notices/{id}/content`.
  For each one I took name, street, postcode, town, e-mail and roles (OPT-300/301 and the DE1 party references).
- The planned side: a planned row's notice from `planned_mention_keys`, plus one planned GUID row through `/v1/organizations/{row}`.
  That list is cut at 2,000 keys (notices up to 24341274), so only 5 of the 14 items have a planned notice in it: 16, 19, 20, 21
  and 23. For the other 9, the only planned-side evidence is the report's `planned_cities`.
- The report's city screen: `cities` is the target's city set over its newest 50 mentions (BT-513, folded).

**GETs.** 54 public GETs, all 200 and none 408: 19 organizations and 35 notice contents. They are logged as `reader=city2` in
`api_requests.log`, 18:03:41Z-18:08:52Z, and went through the shared `.api.lock`/`.api.last` pacing (2.2 s after any reader's last
GET). The smallest gap in that window is 2 s at the log's 1-second resolution. The bodies are in `bodies/`. I also reused cached
bodies without fetching them again:
- 45852543 (city1);
- 24336021 (top1);
- 24375779, 24313135 and 24363093 (rand2);
- the GIZ notices 24254237, 24337934, 24369991, 26340016 and 26742076.

## Tally

**R 13, N 0, W 1.** The flagged-target bar is 0 W, and item 15 fails it. Under A6, `Geschäftsstelle der Vergabekammer` goes into
`GUID_TWIN_UNSAFE_NAMES` (or into `extra_unsafe_names` for an R1 re-run). Item 19 is R, but its name is a latent homonym; the note
is in its paragraph.

| # | target | name | mentions | planned rows / mentions | target cities → planned cities | verdict |
|---|---|---|---|---|---|---|
| 14 | 1366195 | Regierungspräsidium Stuttgart | 465 | 5 / 10 | heilbronn, stuttgart → stuttgart | R |
| 15 | 6622773 | Geschäftsstelle der Vergabekammer | 105 | 5 / 10 | weimar → munster (5 absent) | **W** |
| 16 | 1171471 | Klinikum Esslingen GmbH | 71 | 4 / 8 | esslingen → esslingen a n | R |
| 17 | 1176914 | Capgemini Deutschland GmbH | 163 | 4 / 8 | berlin → berlin, koln, troisdorf | R |
| 18 | 1252070 | AOK Bayern - Die Gesundheitskasse | 946 | 4 / 8 | munchen → nurnberg | R |
| 19 | 1437249 | Gemeinde Ensdorf | 12 | 4 / 8 | ensdorf saar → ensdorf | R (latent homonym) |
| 20 | 1450065 | AOK - Die Gesundheitskasse in Hessen | 423 | 4 / 8 | homburg → gros gerau | R |
| 21 | 2146835 | NRW.Urban GmbH | 100 | 3 / 6 | dortmund → dusseldorf | R |
| 22 | 2156517 | Malteser Hilfsdienst gGmbH | 287 | 3 / 6 | dresden, grafelfing, regensburg → grafelfing, ingolstadt, muhldorf | R |
| 23 | 9147905 | Stadt Lübbenau/Spreewald | 225 | 3 / 6 | 03222 lubbenau spreewald, lubbenau spreewald → lubbenau spreewald | R |
| 24 | 16989550 | JVA Werl | 13 | 3 / 6 | munster, werl → werl | R |
| 25 | 2217 | Vergabekammer des Bundes beim Bundeskartellamt | 90,279 | 2 / 4 | bonn, eschborn → bonn | R |
| 26 | 9190 | Vergabekammer Schleswig-Holstein | 13,382 | 2 / 4 | kiel, kiel dusternbrook → kiel | R |
| 27 | 1163070 | Tribunal administratif de Toulouse (FR) | 32,566 | 4 / 4 | toulouse + 3 CEDEX spellings → toulouse | R |

## Items

**14: 1366195 "Regierungspräsidium Stuttgart": R.**
- The flag is multi-city (heilbronn, stuttgart), with 0 planned mentions absent. All 10 planned mentions are in Stuttgart.
- All three of the target's report notices print Regierungspräsidium Stuttgart, Ruppmannstr. 21, 70565 Stuttgart, with e-mails at
  rps.bwl.de: 4027281 and 4068205 (2010 award notices, referat13@rps.bwl.de) and 4155393 (2010 prior information notice).
- Only one body carries this name: the Land authority for the Regierungsbezirk Stuttgart, and Heilbronn lies inside that district.
  The Heilbronn mention is somewhere in the newest 50, which the report does not list by id, so I could not read it.
- No planned row is in the truncated key list, so the planned side is the screen's Stuttgart city alone.
- Not a homonym. The fold adds Stuttgart-addressed mentions to the Stuttgart authority.

**15: 6622773 "Geschäftsstelle der Vergabekammer": W.**
- The target has 105 mentions and its screen city set is {weimar}. All 5 planned rows (10 mentions) are in {munster}, and every
  planned row is city-absent.
- All three of the target's report notices are the Thuringian review chamber:
  - 26223383 (DÖE) and its TED copy 23604678 (00740128-2023, buyer Landratsamt Saale-Holzland-Kreis) print 99423 Weimar,
    vergabekammer@tlvwa.thueringen.de (Thüringer Landesverwaltungsamt), in the ReviewOrg, ReviewInfo and Mediator roles.
  - 23464988 (Stadt Jena, 2023/S 196-613559) names it as review body, in Weimar.
- The planned rows are the Vergabekammer Westfalen at the Bezirksregierung Münster. That chamber prints "Geschäftsstelle der
  Vergabekammer" as its department (BT-502) in two cached 2024 notices that other readers read. In both, its name is a different
  one, so neither notice is on this key:
  - 24336021 (top1), Gelsenkirchen: "Vergabekammer Westfalen bei der Bezirksregierung Münster", Domplatz 1-3, 48143 Münster;
  - 24375779 (rand2), Sundern: "Vergabekammer Westfalen", Albrecht-Thaer-Str. 9, 48147 Münster, @brms.nrw.de.
- The twin key is the published name, so the 5 planned rows print that string as the name (BT-500), with town Münster.
- None of the 5 planned notices is in the truncated key list, so I could not read one. The screen's Münster city on all 5 is the
  direct evidence.
- "Geschäftsstelle der Vergabekammer" is a function name, not a body. It denotes at least two review chambers in two Länder:
  Thüringen in Weimar and Westfalen in Münster. A third Land's office shows the same naming habit under a longer, distinct name:
  "Geschäftsstelle der Vergabekammern beim Ministerium für Wirtschaft, … Mecklenburg-Vorpommern" in Schwerin (24313135 and
  24363093; rand2 item 26, target 23021560). The fold would put Westfalen's 10 mentions onto the Thuringian row, a review body
  attached to the wrong Land.
- Why the guard missed it:
  - It has 2 content words (geschäftsstelle, vergabekammer).
  - It is not a `generic_body_name`, and it is not on the 530 list.
  - It escaped the office two-word read because `geschäftsstelle` is not in `GUID_TWIN_OFFICE_NOUNS`.
- **Action:** put `Geschäftsstelle der Vergabekammer` into `GUID_TWIN_UNSAFE_NAMES`. Consider adding `geschäftsstelle` to the office
  nouns, so that a sibling key is read in R2.

**16: 1171471 "Klinikum Esslingen GmbH": R.**
- The flag is a spelling: the planned city "esslingen a n" (Esslingen a. N.) against the target's "esslingen".
- The target's notices 26286614 (DÖE) and 23839918 (TED 00129945-2024, glass cleaning) print the buyer at Hirschlandstr. 97, 73730
  Esslingen, investitionen@klinikum-esslingen.de.
- The planned notice 24263748 (00656817-2024, pharmacy picking system) has the twin ORG-0001 (Procedure-Buyer) and ORG-0003
  (Lot-AddInfo): both are Klinikum Esslingen GmbH, Hirschlandstraße 97, 73730 Esslingen a. N., with the consultant's contact
  e-mail (bgmed.de).
- GUID row 22760311 still holds GUID 5D6702E8… with 2 mentions.
- Same address, same hospital company.

**17: 1176914 "Capgemini Deutschland GmbH": R.**
- The target's city set is {berlin}. The 8 planned mentions are in berlin, koln and troisdorf, and 2 are absent.
- All three of the target's report notices print the tenderer as Capgemini Deutschland GmbH, Potsdamer Platz 5, 10785 Berlin
  (placeholder id 123456): 26735068 (award notice 2025-08, register-modernisation consulting), 26696298 (VEAT, same procedure) and
  26569990 (award notice 2025-02, Mein ELSTER).
- A name with its legal form is one registered company, and its Köln and Troisdorf addresses read as offices of that company.
  Capgemini's other German companies carry other names, so they have other keys. The role is tenderer only.
- No planned row is in the truncated key list, so the planned side is the screen's cities, and I could not locate the two off-city
  mentions by id.
- One company: R.

**18: 1252070 "AOK Bayern - Die Gesundheitskasse": R.**
- The target's city set is {munchen}. All 8 planned mentions are in {nurnberg}, and 4 are absent.
- The target's notices:
  - 45852543 (joint AOK contract modification, 2026-06; city1's body) has ORG-0002 AOK Bayern, Carl-Wery-Straße 28, 81739 München,
    as Procedure-Buyer. Ten other AOKs sit beside it, each under its own Land-specific name.
  - 3293921 (2008 award) and 3166597 (2007) print Carl-Wery-Str. 28, München, @by.aok.de.
- AOK Bayern is one statutory health insurer for Bavaria, and Nürnberg reads as one of its regional sites. The name cannot collide
  with another AOK, because each carries its region.
- No planned row is in the truncated key list.
- R.

**19: 1437249 "Gemeinde Ensdorf": R (latent homonym).**
- The flag is the "/Saar" suffix only: the target city is "ensdorf saar" and the planned city is "ensdorf".
- The target's notices:
  - 26894145 (DÖE 2026, gigabit broadband) prints Provinzialstraße 101a, 66806 Ensdorf/Saar, kwoll@gemeinde-ensdorf.de, with review
    body Vergabekammer des Saarlandes.
  - 4069956 (2010 contract notice) and 4161159 (2010 award notice, refuse services) print Provinzialstr. 101a, 66806 Ensdorf,
    region Saarlouis, rmaas@gemeinde-ensdorf.de.
- The planned notice 24328093 (00664616-2024, Grundschule Ensdorf extension):
  - ORG-0001 (Procedure-Buyer) and ORG-0002 (Lot-AddInfo) are both at Provinzialstr. 101a, 66806 Ensdorf, info@gemeinde-ensdorf.de.
  - The review body is the Saarland Vergabekammer, and tender receipt is the Zentrale Vergabestelle im Landkreis Saarlouis.
  - GUID row 22806138 still holds GUID 0B4B6797… with 2 mentions.
- Saar folds into Saar, so the fold is right, and the target holds one body today.
- **Caveat:** Ensdorf in the Oberpfalz (Landkreis Amberg-Sulzbach, Bavaria) is also a Gemeinde Ensdorf. That is the Stadt Neustadt
  shape of A6. Nothing I read puts it on this row. The city screen could not tell the two apart if the Bavarian one printed a plain
  "Ensdorf".
- A strict reading of the decision's W clause ("the target's name denotes more than one body") could score this W. I score R
  because both the target and every planned row I could read are the Saarland municipality. If the panel reads the clause
  strictly, the cost is 4 rows left split.

**20: 1450065 "AOK - Die Gesundheitskasse in Hessen": R.**
- The target's city set is {homburg}. All 8 planned mentions are in {gros gerau}, and 4 are absent.
- The target's notices:
  - 45852543 has ORG-0004 at Basler Str. 2, 61352 (Bad) Homburg, the head office, as buyer.
  - 3224420 (2007, building cleaning) prints AOK - Die Gesundheitskasse in Hessen, Gernsheimer Straße 43, D-64521 Groß-Gerau.
- So the target has held Groß-Gerau mentions since 2007.
- The planned notice 24284335 (00648232-2024, HR-marketing agency): ORG-0001 (Procedure-Buyer) and ORG-0002 (Lot-AddInfo) are both
  at Gernsheimer Straße 43, 64521 Groß-Gerau, beschaffungsstelle@he.aok.de. GUID row 22775561 still holds GUID E7CE1A00… with 2
  mentions.
- One insurer with two sites, both already on the row.

**21: 2146835 "NRW.Urban GmbH": R.**
- The target's city set is {dortmund}. All 6 planned mentions are in {dusseldorf}, and 3 are absent.
- The target's report notices are 2010 text-era: 3989353 (prior information notice) and 4202050 (contract notice) print NRW.URBAN
  GmbH, Schanzenstr. 131, 40549 Düsseldorf, @nrw-urban.de. So the target already holds Düsseldorf mentions.
- The Dortmund city comes from newer eForms mentions that I could not locate by id.
- The planned notice 24332213 (00718510-2024, Leverkusen Innovationspark road works): ORG-0001 (Procedure-Buyer), ORG-0002
  (Lot-AddInfo) and ORG-0004 (Lot-TenderReceipt) are all NRW.URBAN GmbH, Fritz-Vomfelde-Straße 10, 40547 Düsseldorf,
  zentralevergabe@nrw-urban.de. GUID row 22809425 still holds GUID A810EC38… with 2 mentions.
- One company with the same domain throughout, at two sites.

**22: 2156517 "Malteser Hilfsdienst gGmbH": R.**
- The target's city set is {dresden, grafelfing, regensburg}. The 6 planned mentions are in {grafelfing, ingolstadt, muhldorf}, and
  4 are absent.
- All three of the target's report notices are school-transport awards with Malteser as tenderer:
  - 26960723 (2026, Landkreis Bad Tölz-Wolfratshausen): 82166 Gräfelfing.
  - 26760472 (2025, Selb): Further Str. 10, 93059 Regensburg, @malteser.org.
  - 26384676 (DÖE 2024, Hoyerswerda): Leipziger Straße 33, 01097 Dresden, hoyerswerda@malteser.org.
- This is one registered gGmbH, printed with the address of whichever regional or local unit bid. The planned Bavarian towns are
  more of the same, and Gräfelfing is already on the row. The role is tenderer throughout.
- No planned row is in the truncated key list.
- One legal entity: R.

**23: 9147905 "Stadt Lübbenau/Spreewald": R.**
- The two "cities" are one town. "03222 lubbenau spreewald" is a postcode printed inside BT-513: 46128318 prints "03222 03222
  Lübbenau/Spreewald".
- The target's notices 47408368 (award notice 2026-10-15, footbridge at Leipe) and 46128318 (award notice 2026-09-15, Kita) print
  the buyer at Kirchplatz 1, 03222 Lübbenau/Spreewald, @luebbenau-spreewald.de.
- The planned notice 24275277 (00653879-2024, landscaping lot 1): ORG-0001 (Procedure-Buyer), ORG-0002 (Lot-AddInfo) and ORG-0004
  (Lot-TenderReceipt) are all Technisches Gebäudemanagement, Kirchplatz 1. GUID row 22768843 still holds GUID A2C334DF… with 2
  mentions.
- R.

**24: 16989550 "JVA Werl": R.**
- The target's cities are {munster, werl}. All 6 planned mentions are in {werl}.
- The target's notices are all one procedure, furniture removal and storage:
  - 24195769 (TED award notice 00525431-2024) and its DÖE copy 26441073 print JVA Werl, Belgische Str. 4, 59457 Werl,
    poststelle@jva-werl.nrw.de.
  - 26363486 (DÖE contract notice 2024-06) prints the same street and e-mail, but with 48147 Münster. That is the postcode of the
    review body, Vergabekammer Westfalen, Albrecht-Thaer-Str. 9, 48147 Münster, copied into the buyer's address.
- So the Münster city is a data-entry slip, not a second body. The prison's name is unique.
- No planned row is in the truncated key list, and the planned city is Werl.
- R.

**25: 2217 "Vergabekammer des Bundes beim Bundeskartellamt": R.**
- The target's cities are {bonn, eschborn}, and all 4 planned mentions are in {bonn}.
- The newest notice, 47414783 (prior information notice 2026-10-16, Die Autobahn GmbH des Bundes), prints Part-ReviewOrg at
  Villemombler Straße 76, 53123 Bonn, vk@bundeskartellamt.bund.de.
- This is the one federal review body.
- The Eschborn city is in the newest 50, which I could not reach by id. Eschborn is the seat of federal buyers such as GIZ, so a
  buyer's town printed into the review section, as in item 24, is the likely cause.
- The cached Eschborn-side GIZ notices (24254237, 24337934, 24369991, 26340016, 26742076) print the federal chamber in Bonn, under
  the different name "(Die) Vergabekammern des Bundes", so they are not on this key.
- No planned row is in the truncated key list.
- Not a homonym: R.

**26: 9190 "Vergabekammer Schleswig-Holstein": R.**
- The target's cities are {kiel, kiel dusternbrook}, and all 4 planned mentions are in {kiel}.
- The newest notice, 47414580 (contract notice 2026-10-16, FEK Neumünster RIS/PACS), prints Lot-ReviewOrg at Düsternbrooker Weg 92,
  24105 Kiel, vergabekammer@wimi.landsh.de.
- "Kiel-Düsternbrook" is the district of that street, so it is one city and one body.
- No planned row is in the truncated key list.
- R.

**27: 1163070 "Tribunal administratif de Toulouse" (FR): R.**
- The four target cities (toulouse, toulouse cedex 07, toulouse cedex 7, toulouse cx7) are CEDEX spellings of one postal address.
  The 4 planned rows have 1 mention each, all in {toulouse}.
- The newest notice, 47390867 (00705596-2026, Le Muretain Agglo), prints Lot-ReviewOrg and Lot-ReviewInfo at 68 rue Raymond IV BP
  7007, 31068 Toulouse Cedex 7, greffe.ta-toulouse@juradm.fr.
- One court.
- No planned row is in the truncated key list.
- R.
