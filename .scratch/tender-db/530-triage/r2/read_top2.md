# 530 R2: top targets, items 15-29 (read_top2)

Scope: `body.r2_top_targets[15..29]` (0-based) of `guid_twin_plan_2659.json`. These are the 16th to 30th targets by planned
mentions: 160 to 254 planned mentions each, from 80 to 127 GUID rows. All 15 are DE identifier-NULL rows that exist today,
so none is a missing key. None is on `city.flagged` (41 entries). None of their names appears in `roles.incoherent` (30
groups), and `tenderer_mixed` is 0. Two are office two-word keys that also appear in `r2_office_two_words`: 3233 Vergabekammer
Westfalen and 622 Vergabekammer des Bundes.

Question: is the target one body (**R**), unclear (**N**), or a homonym or umbrella that collects different bodies (**W**)?

**Method.**
- `/v1/organizations/{target}`. Every target has identifier NULL, provisional true, country DE, and the report's mention
  count, unchanged.
- **The target's existing mentions.** I read the first notice the report lists for each target through
  `/v1/notices/{id}/content`, taking name, street, postcode, town, e-mail, contact and roles.
  - For the eForms-era targets, that notice is a DÖE notice of 2026-10-15/16.
  - For the others, it is a TED-form (2023) or text-era (2008-2010) notice. Those carry no BT-513, so their city sets are
    empty and the city screen could not compare. I compared TED-TOWN or the address in the text instead.
- **The planned side.** For each target I read at least one planned row's notice (the TED copy), taken from
  `planned_mention_keys` or `r2_random`.
  - I read every ORG section: the twin group under the target's name, its BT-501 values, the address and e-mail, and the
    OPT-300/301 roles.
  - I also checked one planned GUID row per target with `/v1/organizations/{row}`. Every one still holds its GUID and 2
    mentions.
- **Reused bodies.** Seven targets have a planned row in `r2_random` that another reader already read: read_rand1 items 7,
  10 and 11, read_rand3 items 36 and 37, and read_rand4 items 48 and 52. I re-read those cached bodies locally. My verdicts
  agree with theirs, all R.
- **GETs.** 38, all 200 and none 408, logged as `reader=top2` in `r2/api_requests.log`. They went through the shared
  `.api.lock`/`.api.last` pacing (2.2 s or more after any reader's last GET). The log window 17:51:32Z-17:56:00Z holds 39
  lines, one of them from top1. The smallest gap is 2 s at the log's 1-s resolution. Bodies are in `r2/bodies/`.

## Tally

**R 15, N 0, W 0.** The top-target bar is 0 W, and this half meets it.

| # | target | name | mentions | planned rows / mentions | target cities | verdict |
|---|---|---|---|---|---|---|
| 15 | 1262231 | Universität Stuttgart | 420 | 127 / 254 | stuttgart | R |
| 16 | 3233 | Vergabekammer Westfalen (office two-word) | 37,696 | 122 / 244 | munster | R |
| 17 | 2443792 | Bau- und Liegenschaftsbetrieb NRW Aachen | 573 | 118 / 236 | none (old notices) | R |
| 18 | 14256311 | Brandenburgischer Landesbetrieb für Liegenschaften und Bauen (BLB) - Zentrale Vergabestelle | 371 | 114 / 228 | none (TED form) | R |
| 19 | 622 | Vergabekammer des Bundes (office two-word) | 111,305 | 107 / 214 | bonn | R |
| 20 | 1175826 | Vergabekammer des Landes Hessen bei dem Regierungspräsidium Darmstadt | 6,648 | 101 / 202 | darmstadt | R |
| 21 | 3856 | Vergabekammern des Bundes | 17,642 | 96 / 192 | bonn | R |
| 22 | 5880993 | Vergabekammer des Landes Brandenburg beim Ministerium für Wirtschaft und Energie | 14,414 | 96 / 192 | potsdam | R |
| 23 | 1222044 | Bau- und Liegenschaftsbetrieb NRW Münster | 261 | 94 / 188 | munster | R |
| 24 | 1190366 | Stadt Salzgitter | 1,003 | 91 / 182 | salzgitter | R |
| 25 | 3796660 | Kupferstadt Stolberg (Rhld.) | 52 | 85 / 170 | none (TED form) | R |
| 26 | 1167697 | Medizinische Hochschule Hannover | 480 | 84 / 168 | none (text era) | R |
| 27 | 1164082 | Bau- und Liegenschaftsbetrieb NRW Duisburg | 726 | 82 / 164 | none (text era) | R |
| 28 | 21317603 | Zentrale Vergabestelle - Kooperation Landkreis Lüneburg - Harburg | 2 | 82 / 164 | none (TED form) | R |
| 29 | 3645 | Vergabekammer Baden-Württemberg beim Regierungspräsidium Karlsruhe | 49,411 | 80 / 160 | karlsruhe | R |

**Across the items.**
- **Every planned twin group is role-coherent.**
  - Buyer-family groups combine Procedure-Buyer with AddInfo, TenderReceipt, ReviewInfo and, for Stolberg,
    Procedure-SProvider.
  - Review-family groups combine ReviewInfo with ReviewOrg and Mediator.
  - In every notice read, all sections of a group give one address and one e-mail. Review bodies, tenderers and DÖE are
    always printed under other names.
- **Several targets are one body under a variant name of another target.** Examples: 3233 and item 8's 6136373 (Vergabekammer
  Westfalen); 622 and 3856 (the federal chambers, singular and plural), plus the stable GUID row 255; 5880993 and item 11's
  1168329 (Brandenburg). These splits exist already. The plan neither creates nor closes them, and none is a homonym, so none
  scores W.

## Items

**15. 1262231 Universität Stuttgart: R.** The target has 420 mentions, and the plan adds 254 from 127 rows; its city set is
{stuttgart}. Its listed mention 4062571 is a 2010 award (TED 101611-2010) by "Universität Stuttgart, Keplerstraße 7, 70174
Stuttgart", www.uni-stuttgart.de. In the planned notice 24264920 (TED 00658288-2024, a printer for quantum devices), three
sections print the name under three fresh GUIDs (636ebbb6…, 224a752e…, c0700306…): ORG-0001 Procedure-Buyer at Keplerstraße 7,
and ORG-0002 Lot-AddInfo and ORG-0005 Lot-TenderReceipt at Breitscheidstraße 2 C with contact "Zentrale Beschaffungsstelle".
All are in 70174 Stuttgart with beschaffung@verwaltung.uni-stuttgart.de. Notice 24429494 (VMware licences; row 22886247,
`E5C71442…`, 2 mentions) has the same shape (read_rand1 item 10). ORG-0003 "Universität Stuttgart, Zentrale
Beschaffungsstelle" (ReviewInfo) is another key. ORG-0004 is the Vergabekammer Baden-Württemberg. The two streets are the
university's administration and its procurement office, so the target is one university.

**16. 3233 Vergabekammer Westfalen: R.** The target has 37,696 mentions, and the plan adds 244 from 122 rows. It is an office
two-word key, and its city set is {munster}. Its newest mention, 47414774 (DÖE 2026-10-16, a BLB NRW roofing lot), prints it
as Lot-ReviewOrg with no BT-501, at Albrecht-Thaer-Straße 9, 48147 Münster, vergabekammer@bezreg-muenster.nrw.de. The
planned notice 24332400 (TED 00718435-2024, Hochschule Hamm-Lippstadt, MIMO equipment) prints it twice:
- ORG-0003 Lot-ReviewInfo, `d9a5b7c1…`, row 22809594 (2 mentions);
- ORG-0004 Lot-ReviewOrg, `c79ff1e8…`.

Both sections give the same address and e-mail. Although the key has only two words, "Westfalen" pins it: there is one
Vergabekammer Westfalen, the chamber at the Bezirksregierung Münster. The same body also sits under item 8's longer name
(6136373). That split existed before the plan and is not a homonym.

**17. 2443792 Bau- und Liegenschaftsbetrieb NRW Aachen: R.** The target has 573 mentions, and the plan adds 236 from 118 rows.
Its city set is empty, because its listed mentions are 2008 notices with no BT-513; the planned city is {aachen}. Its notice
3439214 (PIN 179119-2008) gives "Bau- und Liegenschaftsbetrieb NRW Aachen, Mies-van-der-Rohe-Straße 10, D-52074 Aachen" with
blb.nrw.de e-mails. The planned notice 24310554 (TED 00662130-2024, RWTH Cube 3 building automation) prints it twice:
- ORG-0001 Procedure-Buyer, `03023553…`, row 22794036 (2 mentions);
- ORG-0002 Lot-AddInfo, `61724432…`.

Both give the same Mies-van-der-Rohe-Straße 10, 52074 Aachen, with AC.Einkauf@blb.nrw.de and "Einkauf und
Vertragsmanagement". The review chamber (VK Rheinland) and the tenderers have other names. The target is the Aachen branch of
the NRW state building agency, at the same address 16 years apart.

**18. 14256311 BLB Brandenburg - Zentrale Vergabestelle: R.** The target has 371 mentions, and the plan adds 228 from 114
rows. Its city set is empty and the planned city is {zossen}. Its notice 23509393 is a 2023 TED-form notice (2023/S
203-638823, BAM TTS Horstwalde). It carries the identical official name, An der Wache 2, 15806 Zossen, contact "Zentrale
Vergabestelle, Bereich BLD" and Info.Vergabe-BLD@BLB.Brandenburg.de. The planned notice 24277826 (TED 00650943-2024, site
clearance for the Bundespolizei at Blumberg) prints the name twice:
- ORG-0001 Procedure-Buyer, `7752e9a2…`;
- ORG-0002 Lot-AddInfo, `b64d10bb…`, row 22770771 (2 mentions).

Both sections give the same address, contact and e-mail (read_rand3 item 36). "BLB - Vergabeprüfstelle" (Potsdam,
ReviewInfo) and the Vergabekammer des Bundes are other keys. The target is one office of one Landesbetrieb.

**19. 622 Vergabekammer des Bundes: R.** The target has 111,305 mentions, and the plan adds 214 from 107 rows. It is an office
two-word key, and its city set is {bonn}. Its newest mention, 47414764 (DÖE 2026-10-16, Stiftung Preußischer Kulturbesitz,
X-ray scanner), prints it as Lot-ReviewOrg at Villemombler Straße 76, 53123 Bonn, vk@bundeskartellamt.bund.de. The planned
notice 24263691 (TED 00656320-2024, GSI Helmholtzzentrum, Green IT Cube) prints it twice:
- ORG-0003 Lot-ReviewInfo, `e8222edb…`, row 22760267 (2 mentions);
- ORG-0004 Lot-ReviewOrg, `0d7bd2d5…`.

Both give Villemomblerstr. 76, 53123 Bonn and the same e-mail. The name denotes the federal review chambers at the
Bundeskartellamt. They are one institution with numbered panels, which notices name in the singular here and in the plural
in item 21. The stable GUID row 255 (`0A9EA480…`) is the same body. The planned values are not 255's, KEEP spares 255, and the
fold lands on 622 and never on 255 (300 §10), so the 622/255 split exists already and stays as it is.

**20. 1175826 Vergabekammer des Landes Hessen bei dem Regierungspräsidium Darmstadt: R.** The target has 6,648 mentions, and
the plan adds 202 from 101 rows; its city set is {darmstadt}. Its newest mention, 47408352 (DÖE 2026-10-15, LBIH joinery),
prints it as Lot-ReviewOrg at "Fristenbriefkasten: Luisenplatz 2", 64283 Darmstadt. The planned notice 24578315 (TED
00083621-2025, Landkreis Gießen, GrS Lollar) prints it three times:
- ORG-0003 ReviewInfo;
- ORG-0004 Lot-ReviewOrg, `23bb7831…`, row 23001169 (2 mentions);
- ORG-0005 Lot-Mediator.

All three give Wilhelminenstraße 1-3, Fristenbriefkasten Luisenplatz 2, 64278 Darmstadt, vergabekammer@rpda.hessen.de
(read_rand3 item 37). Both notices name the same Luisenplatz letterbox of the Regierungspräsidium. The target is Hessen's one
review chamber.

**21. 3856 Vergabekammern des Bundes: R.** The target has 17,642 mentions, and the plan adds 192 from 96 rows; its city set
is {bonn}. Its newest mention, 47414721 (DÖE 2026-10-16, Fraunhofer, interior wooden doors), prints it as Lot-ReviewOrg at
Kaiser-Friedrich-Straße 16, 53113 Bonn, with no e-mail. The planned notice 24253044 (TED 00635881-2024, VBG, caretaker
services) prints it three times:
- ORG-0003 ReviewInfo, `3c16a1f9…`, row 22751851 (2 mentions);
- ORG-0004 ReviewOrg, `47fe2b0b…`;
- ORG-0005 Mediator, `8308c3e9…`.

All three give Villemomblerstr. 76, 53123 Bonn, mail@bundeskartellamt.bund.de. The two Bonn street addresses belong to the
same federal chambers. The plural name is the institution of item 19, kept apart by its name. It is one body.

**22. 5880993 Vergabekammer des Landes Brandenburg beim Ministerium für Wirtschaft und Energie: R.** The target has 14,414
mentions, and the plan adds 192 from 96 rows; its city set is {potsdam}. Its newest mention, 47414670 (DÖE 2026-10-16,
Gemeinde Michendorf, metalwork), prints it twice, as ReviewInfo and ReviewOrg, at Heinrich-Mann-Allee 107, 14473 Potsdam,
poststelle@mwae.brandenburg.de. The planned notice 24403517 (TED 00761636-2024, ProPotsdam) prints it twice:
- ORG-0002 ReviewInfo, `238124b1…`, row 22865917 (2 mentions);
- ORG-0003 ReviewOrg, `4085060d…`.

Both give the same address with vergabekammer@mwae.brandenburg.de (read_rand4 item 48). The target is the Brandenburg state
chamber. Its other ministry-name variants, such as item 11's 1168329, are separate keys.

**23. 1222044 Bau- und Liegenschaftsbetrieb NRW Münster: R.** The target has 261 mentions, and the plan adds 188 from 94 rows;
its city set is {munster}. Its listed mention 44550117 (TED 00353597-2025, MRV Bedburg-Hau) has no BT-501 and prints the name
twice: ORG-0001 as Buyer, AddInfo and TenderReceipt, and ORG-0002 as ReviewInfo (Justiziariat). Both are at Hohenzollernring
80, 48145 Münster, BLBVergabe@blb.nrw.de. So the target already holds this body's per-role duplicates. The planned notice
24445522 (TED 00738975-2024, Universität Münster CMM, screed and flooring) prints it twice:
- ORG-0001 Buyer, `ba3efe1e…`, row 22898749 (2 mentions);
- ORG-0002 AddInfo, `c76759e4…`.

Both give the same address and e-mail (read_rand1 item 7). The target is the Münster branch of BLB NRW.

**24. 1190366 Stadt Salzgitter: R.** The target has 1,003 mentions, and the plan adds 182 from 91 rows; its city set is
{salzgitter}. Its newest mention, 47414618 (DÖE 2026-10-16, Grundschule Waldschule), prints the city as Buyer, AddInfo and
TenderReceipt at Joachim-Campe-Straße 6-8, 38226 Salzgitter, submission@stadt.salzgitter.de, with no GUID. The planned notice
24396555 (TED 00681593-2024, drywall) prints it twice:
- ORG-0001 Buyer, `bf4b868b…`, row 22860164 (2 mentions);
- ORG-0002 AddInfo, `7d16acc6…`.

Both give the same address and e-mail (read_rand1 item 11). Salzgitter is one city.

**25. 3796660 Kupferstadt Stolberg (Rhld.): R.** The target has 52 mentions, and the plan adds 170 from 85 rows, more than
quadrupling it. Its city set is empty and the planned city is {stolberg}. Its notice 23528675 (2023 TED form) gives Rathausstr.
11-13, 52222 Stolberg, vergabestelle@stolberg.de, contact "A65.5 - Zentrale Vergabestelle". The planned notice 24266537 (TED
00656947-2024, historic town hall, heating) prints the town in five sections, each under its own GUID:
- ORG-0001 Procedure-Buyer;
- ORG-0002 Procedure-SProvider;
- ORG-0003 Lot-AddInfo;
- ORG-0004 Lot-ReviewInfo;
- ORG-0006 Lot-TenderReceipt.

All five give the same address, e-mail and contact. ORG-0005, the Vergabekammer Rheinland, is the review body. Notice 24592945
(TED 00095995-2025, sanitary works; row 23011460, the SProvider section) has the same shape (read_rand4 item 52). "(Rhld.)"
sets it apart from Stolberg (Harz). The target is one town that acts as its own service provider.

**26. 1167697 Medizinische Hochschule Hannover: R.** The target has 480 mentions, and the plan adds 168 from 84 rows. Its city
set is empty and the planned city is {hannover}. Its notice 3223576 is a text-era award: "Medizinische Hochschule Hannover,
Carl-Neuberg-Straße 1, Abt. Zentraleinkauf, D-30625 Hannover". The planned notices are 24263303 (TED 00639745-2024, K20 dental
clinic refurbishment, building controls) and 24294091 (TED 00644502-2024, joinery). In each, four sections print MHH under
four fresh GUIDs: ORG-0001 Buyer, ORG-0002 AddInfo, ORG-0003 ReviewInfo and ORG-0005 TenderReceipt. One of them is
`9c9c6e29…`, row 22759926 (2 mentions). All four give Carl-Neuberg-Straße 1, 30625 Hannover, zentraleinkauf@mh-hannover.de
and "GB IV - Logistik, Abteilung Zentraleinkauf OE 1260". ORG-0004, the ReviewOrg, is the Vergabekammer Niedersachsen. The
target is one medical school.

**27. 1164082 Bau- und Liegenschaftsbetrieb NRW Duisburg: R.** The target has 726 mentions, and the plan adds 164 from 82
rows. Its city set is empty and the planned city is {duisburg}. Its notice 3309620 (a 2008 text-era contract notice,
22570-2008) gives "Bau- und Liegenschaftsbetrieb NRW Duisburg, Lotharstraße 53, D-47057 Duisburg", kr.einkauf@blb.nrw.de,
www.blb.nrw.de. The planned notice 24330844 (TED 00663420-2024, Uedem, electrical installation) prints it twice:
- ORG-0001 Buyer, `1c8d71b0…`, row 22808267 (2 mentions);
- ORG-0002 AddInfo, `781a88c9…`.

Both give Am Silberpalais 1, 47057 Duisburg, BLBVergabe@blb.nrw.de. The street differs after 16 years, but the postcode and
the agency are the same. ReviewInfo and Mediator are the OFD NRW (Bauabteilung), and ReviewOrg is the federal chambers, all
other names. The target is the Duisburg branch.

**28. 21317603 Zentrale Vergabestelle - Kooperation Landkreis Lüneburg - Harburg: R.** The target has only 2 mentions today,
and the plan adds 164 from 82 rows, the largest relative growth in this half (2 → 166). Its city set is empty and the planned
city is {winsen luhe}. Both existing mentions, 23410408 and 23222237 (2023 TED-form notices), give Schloßplatz 6, 21423
Winsen/Luhe, vergabestelle@lkharburg.de. Two planned notices give the same address and e-mail:
- 24254305 (TED 00637471-2024, IGS Embsen, drywall): ORG-0001 Buyer `fe53800f…`, row 22752914 (2 mentions), and ORG-0002
  AddInfo `d83e8cf1…`;
- 24265888 (TED 00656621-2024, postal services for the Landkreis Lüneburg): `bed31373…` and `87ab7d54…`.

The name denotes the joint procurement office of two Landkreise, housed at the Landkreis Harburg in Winsen. It runs
procedures for either Landkreis; both planned notices are Lüneburg projects. The name denotes that one office at one address,
not either Landkreis, so the target is one body. Bare "Zentrale Vergabestelle" is on the unsafe list, but this longer name is
specific and the guard correctly lets it through.

**29. 3645 Vergabekammer Baden-Württemberg beim Regierungspräsidium Karlsruhe: R.** The target has 49,411 mentions, and the
plan adds 160 from 80 rows; its city set is {karlsruhe}. Its stored name has a line break ("beim\nRegierungspräsidium") that
`org_name_norm` folds to the planned name. Its newest mention, 47414734 (DÖE 2026-10-16, Stadt Karlsruhe Konzerthaus), prints
it as Lot-ReviewOrg and Lot-Mediator at Kapellenstraße 17, 76131 Karlsruhe, vergabekammer@rpk.bwl.de. The planned notice
24274826 (TED 00653428-2024, Landratsamt Böblingen, print services) prints it twice:
- ORG-0003 ReviewInfo, `e90098a9…`, row 22768467 (2 mentions);
- ORG-0004 ReviewOrg, `93d1bec9…`.

Both give Durlacher Allee 100, 76137 Karlsruhe, the same e-mail and "Referat 15 - Vergabekammer". The two Karlsruhe
addresses belong to the same Regierungspräsidium under one e-mail. Baden-Württemberg has one Vergabekammer. The
"…im Regierungspräsidium Karlsruhe" variant in the Stuttgart notices (item 15) is another key.
