# 530 R2b: fresh random planned rows, items 0-14 (read_rand1)

Source: `guid_twin_plan_2660.json` (Build 1b, deployed 8e9222c, dry job 2660), `body.r2_random[0..14]`, salt 530003, R2's 60
reads excluded. All 15 rows fall in the buyer stratum. For every row the plan says: DE, 2 mentions, 1 Tender, `planned: whole`,
`refused: 2`, not in KEEP, not stable, `twin: true`, `twin_after: false`. Each row has one target, and all 15 targets exist,
so none is a missing key that Build 2 would mint.

Question: would rehoming all of the row's mentions to its target, the identifier-NULL `(name_norm, DE)` row, be right?
- **R**: the target is the same body, and the GUID row was a per-notice split.
- **N**: cannot tell.
- **W**: one of these holds:
  - the target name denotes more than one body;
  - the target would receive another body's mentions;
  - a different body is printed under this name in this notice's role;
  - the GUID is a real stable key.

**Evidence read for each row.**
- `/v1/organizations/{row}`: the GUID is still in place, with 2 mentions and `identifier_kind` national.
- `/v1/organizations/{target}`: identifier NULL, provisional, DE, and exactly the row's name.
- Both copies of the row's notice, the TED eForms copy and the DÖE DE1 copy, through `/v1/notices/{id}/content`. For each ORG
  section I read the name, BT-501, contact, street, postcode, city, e-mail, phone and NUTS. I also read the role references:
  OPT-300/301 in the TED copy and the DE1 party references in the DÖE copy.
- The target's newest mentions where the report lists them. Six targets are in `r2_top_targets`, which gives 3 notices each;
  I read 2 or 3 per target.
- For 5 targets with no listed mentions, one sibling planned notice from the truncated `planned_mention_keys`.
- The plan's city screen. The `city.flagged` list is complete (36 of 36), and none of the 15 targets is on it. So each target's
  newest-50 city set is either empty or exactly the planned city.

The same picture holds in all 15 notices.
- **Per-notice keys.** The row's GUID sits on ORG-0001 (Procedure-Buyer) in both copies, with the same value. The twin sections
  carry other fresh v4 values. These are the same in both copies and differ from notice to notice.
- **Role guard (A7).** Each twin group has only buyer-side roles: Procedure-Buyer, Lot-AddInfo and Lot-TenderReceipt.
  - In items 6, 12 and 14 the group also holds a Lot-ReviewInfo section. That section prints the buyer's own address, e-mail and
    phone.
  - No group has a Lot-ReviewOrg section. In every notice the ReviewOrg is a review chamber printed under its own name.
  - So neither W shape from R2 occurs here: no umbrella chamber name collecting different chambers, and no review slot printing
    another body's contact details. The Build 1b role guard rightly leaves all 15 refused.
- **Target names.** None of the 15 is a chamber or umbrella name. Each names one legal body, or one branch or office of one.

**GETs.** I made 66 public GETs: 29 `/v1/organizations/{id}` and 37 `/v1/notices/{id}/content`.
- All returned 200, with no 408 and no retry.
- They are logged in `r2b/api_requests.log` (tag `reader=rand1`, 20:45:36Z to 20:49:45Z), and the bodies are in `r2b/bodies/`.
- They went through the shared `r2b/.api.lock` and `.api.last`. The smallest gap between any two logged requests, from either
  reader, is 2.049 s.
- I reused bodies already on disk:
  - from R2 (`r2/bodies/`): TED copies 24522973, 24421187, 24429562 and 24277826, and target notices 23528496, 23526725,
    23509393, 26723175, 26595371, 4062571, 3439214, 3920878 and 3733750;
  - from reader rand2b in `r2b/bodies/`: organization 1262231.
- I did not use `/v1/sql`, ssh, POST, git or cargo.

## Tally

**R 15, N 0, W 0.**

| # | org_id | name | target (mentions) | verdict |
|---|---|---|---|---|
| 0 | 22959818 | Kolpingstadt Kerpen | 2383569 (97) | R |
| 1 | 22879584 | Verwaltungs-Berufsgenossenschaft (VBG) - Hauptverwaltung Hamburg | 10749091 (88) | R |
| 2 | 22886294 | Bundesstadt Bonn - Referat Vergabedienste | 15234277 (376) | R |
| 3 | 22770770 | Brandenburgischer Landesbetrieb für Liegenschaften und Bauen (BLB) - Zentrale Vergabestelle | 14256311 (371) | R |
| 4 | 22956859 | Landkreis Göttingen | 2294259 (216) | R |
| 5 | 22929057 | Rheinland-Pfälzische Technische Universität Kaiserslautern-Landau | 20509663 (33) | R |
| 6 | 22917694 | Stadt Münster | 2251109 (192) | R |
| 7 | 22830289 | Handwerkskammer Ostmecklenburg-Vorpommern | 17956113 (8) | R |
| 8 | 22819953 | Universität Stuttgart | 1262231 (420) | R |
| 9 | 22952705 | Bau- und Liegenschaftsbetrieb NRW Aachen | 2443792 (573) | R |
| 10 | 22769455 | Landkreis Havelland - Der Landrat - | 7114335 (30) | R |
| 11 | 22818997 | SBO Servicebetriebe Oberhausen - Eigenbetrieb der Stadt Oberhausen | 17012302 (80) | R |
| 12 | 22899365 | Stadtverwaltung Cottbus | 1514797 (626) | R |
| 13 | 22910304 | Bau- und Liegenschaftsbetrieb NRW Zentrale | 7195752 (1,101) | R |
| 14 | 22957742 | Stadt Remscheid, Fachdienst Recht und Vergabe, Abteilung Zentrale Vergabestelle | 16921787 (805) | R |

## Items

**0. 22959818, Kolpingstadt Kerpen → 2383569 (97). R.** GUID `5cfc5bae…` sits on ORG-0001 Procedure-Buyer of the Kerpen award
"Lieferung einer DLAK 23/12" (a turntable ladder; TED notice 24522973, DÖE copy 26543360). ORG-0002 Lot-AddInfo prints the same
name under `d217b47d…`, and both copies agree. Both sections give "Abtl. 20.4 - Zentrale Vergabestelle", Jahnplatz 1, 50171 Kerpen,
vergabestelle@stadt-kerpen.de and +49 2237-580. The other names have their own roles: Vergabekammer Rheinland, Köln (ReviewOrg),
"Rhein-Erft-Kreis, Der Landrat", Bergheim (Mediator), and the winner Rosenbauer, Luckenwalde. The group is Buyer+AddInfo, so it is
role-coherent. R2's incoherent Kerpen group (notice 24453957, with a Kerpen-named ReviewOrg section) is a different notice, and
Build 1b now binds it. The target 2383569 is "Kolpingstadt Kerpen": identifier NULL, provisional, DE, 97 mentions, city screen
clean. R2 scored it R for this town twice (read_rand1 item 6, sibling notice 24466427; read_rand3 item 30). The "Kolpingstadt"
title belongs to Kerpen alone, so this is one town and one body.

**1. 22879584, VBG - Hauptverwaltung Hamburg → 10749091 (88). R.** GUID `d35adc25…` sits on ORG-0001 Procedure-Buyer of the VBG
framework for software development with SQL (TED 24421187, DÖE 26539887). ORG-0002 Lot-AddInfo prints the name under `35cbe58f…`.
Both give Beschaffung und Nachhaltigkeit, Massaquoipassage 1, 22305 Hamburg, vergabestelle@vbg.de and +49 405146-1585. The three
"Vergabekammern des Bundes" sections (ReviewInfo, ReviewOrg and Mediator, Bonn) and the DTVP TenderReceipt have their own names.
R2 read the AddInfo twin of this same notice (row 22879585, read_rand3 item 31) and scored it R. The target 10749091 has the same
name: identifier NULL, provisional, 88 mentions, city screen clean. `planned_samples` 12/13 (rows 22751849/50, notice 24253044)
are another VBG notice folding into it. The name pins the head office of one Berufsgenossenschaft, not a regional
Bezirksverwaltung, so it is one body.

**2. 22886294, Bundesstadt Bonn - Referat Vergabedienste → 15234277 (376). R.** GUID `05a74547…` sits on ORG-0001 Procedure-Buyer
of "Friedrich-List-Berufskolleg Sanierung 2. BA … Tischler / Innentüren" (TED 24429562, DÖE 26526605). ORG-0002 Lot-AddInfo prints
the name under `b8323eee…`. Both give Bertha-von-Suttner-Platz 2-4, 53111 Bonn, referatvergabedienste@bonn.de and +49 228772600.
Vergabekammer Rheinland (ReviewInfo and ReviewOrg) and the tenderer have other names. The target is in `r2_top_targets[2]`
(+492 mentions from 246 rows, planned city bonn). Its three listed newest mentions, 23528496, 23526725 and 23525740, are 2023
TED-form notices. Each prints "Bundesstadt Bonn - Referat Vergabedienste" with the same street, postcode, e-mail and phone as the
row. The city set is empty only because TED forms carry TED-TOWN, not BT-513. This is the city's central procurement office, one
body.

**3. 22770770, BLB Brandenburg - Zentrale Vergabestelle → 14256311 (371). R.** GUID `7752e9a2…` sits on ORG-0001 Procedure-Buyer
of the Blumberg contract (Los 1/2, IHB extension; TED 24277826, DÖE 26488243). ORG-0002 Lot-AddInfo prints the name under
`b64d10bb…`, which is row 22770771. Both give the contact "Zentrale Vergabestelle, Bereich BLD", An der Wache 2, 15806 Zossen,
Info.Vergabe-BLD@BLB.Brandenburg.de. Two other sections have other keys: "BLB … - Vergabeprüfstelle" (Potsdam, ReviewInfo) and the
Vergabekammer des Bundes (ReviewOrg). The target is in `r2_top_targets[18]`. Its listed mentions 23509393 and 23496637 are 2023
TED forms (BAM TTS Horstwalde). Both print the identical official name, An der Wache 2, Zossen, the same contact point and the
same e-mail. R2's read_top2 item 18 used this very notice as the target's planned example. The target is one office of one
Landesbetrieb.

**4. 22956859, Landkreis Göttingen → 2294259 (216). R.** GUID `337d5410…` sits on ORG-0001 Procedure-Buyer of a postal-services
award. Its two lots are the Kreishaus Göttingen and the Kreishaus Osterode am Harz; Osterode has been part of the district since
2016 (TED 24519603, DÖE 26561846). ORG-0002 Lot-AddInfo prints the name under `e04da957…`. Both give Zentrale Vergabestelle,
Reinhäuser Landstraße 4, 37083 Göttingen, vergabe@landkreisgoettingen.de. The VK Niedersachsen sections (Lüneburg) and the two
tenderers have other names. A sibling planned notice, 24263362 (Eichsfeld Gymnasium Duderstadt, rows 22759988/89), prints the same
office under fresh values `2f815bcf…` and `7c1a98c1…`. The target 2294259 has identifier NULL, is provisional, DE, and has 216
mentions. It is the 234 fallback row that the issue's target audit and the R1 controls name (22759261/62 → 2294259); the city
screen gives gottingen. The VAT row 22183653 is the same district but is excluded as a target by 300 §10. That is a known split
and not a W of this fold. "Stadt Göttingen" is another name, so the target is one Landkreis.

**5. 22929057, RPTU Kaiserslautern-Landau → 20509663 (33). R.** GUID `798efe63…` sits on ORG-0001 Procedure-Buyer of "Durchführung
einer Befragung zur sozialen Akzeptanz von Geothermie" (TED 24484865, DÖE 26559479). ORG-0002 Lot-AddInfo prints the name under
`f48b2a0b…`. Both give Abt. 2.3 Vergabe und Beschaffung, Gottlieb-Daimler-Straße 47, 67663 Kaiserslautern, zv-vergabeteam1@rptu.de.
The Vergabekammer Rheinland-Pfalz (Mainz) holds ReviewInfo, ReviewOrg and Mediator under its own name. A sibling planned notice,
24331610 (Abwesenheitsmanagementsystem, rows 22808898/99), shows the same office under two more fresh values, `7d35407a…` and
`27f92445…`. The target has identifier NULL, is provisional, and has 33 mentions. It is not city-flagged, so its mentions show no
city other than kaiserslautern. RPTU is one university (since 2023) with two campuses, so the name is one legal entity. R.

**6. 22917694, Stadt Münster → 2251109 (192). R.** The notice is "Projektbegleitung SAP S/4HANA Migration für die Städte Hamm und
Münster" (TED 24470558, DÖE 26549296). GUID `d9bbf7ab…` sits on ORG-0001, which plays both Procedure-Buyer and Lot-AddInfo.
ORG-0003 Lot-ReviewInfo prints the same name under `db799b4a…`. Both give Zentrales Vergabemanagement, Klemensstraße 10, 48143
Münster, vergaben@stadt-muenster.de. The co-buyer Stadt Hamm (ORG-0002) and Vergabekammer Westfalen (ORG-0004 ReviewOrg) are
separate names, so the group takes nothing of theirs. A buyer printing itself as ReviewInfo is the usual "ask us about deadlines"
entry, not a ReviewOrg mix. The target is in `r2_top_targets[4]`, with city set {munster} and 237 planned rows. Its listed newest
mentions are the joint procurement CIVITAS/CORE 2.0: TED 26723175 and 26723160, and DÖE 26595371. There Stadt Münster is one of
eight city co-buyers under its own section, at 48143 Münster, Klemensstraße 10, vergaben@stadt-muenster.de. R2 scored another row
of this target R (read_rand3 item 40). "Stadt Münster" is only the NRW city, so the target is one body.

**7. 22830289, Handwerkskammer Ostmecklenburg-Vorpommern → 17956113 (8). R.** GUID `93e6fed4…` sits on ORG-0001 Procedure-Buyer of
the "Bildungszentrum Rostock / Los 501" award (TED 24357988, DÖE 26515375). ORG-0002 Lot-AddInfo prints the name under `2988138a…`.
Both give Schwaaner Landstraße 8, 18055 Rostock, vergabe@hwk-omv.de and +49 38145490. The MV Vergabekammern office (Schwerin,
ReviewOrg), the winner TEK and the subcontractor have their own names. A sibling planned notice, 24275668 (the same project's
Los 302, rows 22769172/73/75), prints the chamber three times under `6347acad…`, `b7a2180b…` and `d7ca0eb1…` (Buyer, AddInfo and
TenderReceipt). The target is small: identifier NULL, provisional, 8 mentions, not city-flagged. The name is one
Körperschaft, the crafts chamber for eastern Mecklenburg-Vorpommern seated in Rostock. R.

**8. 22819953, Universität Stuttgart → 1262231 (420). R.** GUID `6c5e4990…` sits on ORG-0001 Procedure-Buyer (Keplerstraße 7) of
"Automatisierte Laborsprühanlage" (TED 24344812, DÖE 26508487). Two more sections print the name: ORG-0002 Lot-AddInfo
(`41fcdd6d…`) and ORG-0005 Lot-TenderReceipt (`c9b0aeeb…`), both "Zentrale Beschaffungsstelle", Breitscheidstraße 2 C. All three are
in 70174 Stuttgart with beschaffung@verwaltung.uni-stuttgart.de; the two streets are the administration and its procurement
office. ORG-0003, "Universität Stuttgart, Zentrale Beschaffungsstelle" (ReviewInfo), has another name_norm and stays out. The
ReviewOrg is VK Baden-Württemberg, Karlsruhe. The target is in `r2_top_targets[15]` with city set {stuttgart}. Its listed mentions
4062571 and 4043479 are 2010 text-era notices by "UNIVERSITÄT STUTTGART", STUTTGART. `r2_random[15]` (row 22772664) folds into the
same target. One university.

**9. 22952705, BLB NRW Aachen → 2443792 (573). R.** GUID `cd7af324…` sits on ORG-0001 Procedure-Buyer of "BLB NRW AC / Aachen /
RWTH Karman Auditorium / Innentüren Stahltüren" (TED 24514128, DÖE 26545617). ORG-0002 Lot-AddInfo prints the name under
`1e552fa2…`. Both give Einkauf und Vertragsmanagement, Mies-van-der-Rohe-Straße 10, 52074 Aachen, AC.Einkauf@blb.nrw.de. The
ReviewOrg is "Vergabekammer Rheinland, c/o Bezirksregierung Köln", Köln, and the tenderer is Hübenthal. The target is in
`r2_top_targets[17]`. Its listed mentions 3439214 and 3484606 are 2008 notices by "BAU- UND LIEGENSCHAFTSBETRIEB NRW AACHEN",
AACHEN; the city set is empty because they are text-era. The name carries the branch city, so the target is the Aachen
Niederlassung of one Landesbetrieb. R.

**10. 22769455, Landkreis Havelland - Der Landrat - → 7114335 (30). R.** GUID `0f34bca9…` sits on ORG-0001 Procedure-Buyer of a
planning contract (building/interior and HVAC planning under HOAI; TED 24276020, DÖE 26489832). Two more sections print the name:
ORG-0002 Lot-AddInfo (`74f72db8…`) and ORG-0004 Lot-TenderReceipt (`51423a43…`). All three give "Dezernat I, Zentrale
Vergabestelle", Platz der Freiheit 1, 14712 Rathenow, vergabestelle@havelland.de. The ReviewOrg is the Vergabekammer des Landes
Brandenburg, Potsdam. The target has identifier NULL, is provisional, and has 30 mentions; it is not city-flagged, so it shows no
city other than rathenow, the district seat. "Der Landrat" is the district's own authority, so the name is one Landkreis. R.

**11. 22818997, SBO Servicebetriebe Oberhausen → 17012302 (80). R.** GUID `422bb7c4…` sits on ORG-0001 Procedure-Buyer of
"Elsa-Brändström-Gymnasium … Erweiterung G9, Technische Gebäudeausrüstung" (TED 24343674, DÖE 26498127). ORG-0002 Lot-AddInfo
prints the name under `5d934b44…`. Both give Bahnhofstraße 66, 46145 Oberhausen, vergabe-bm@sbo.oberhausen.de. The ReviewOrg is
Vergabekammer Rheinland, and the tenderer is an engineering office. A sibling planned notice, 24331367 (Gesamtschule Osterfeld,
rows 22808689/90), has the same pair under `f999fbe8…` and `02b49b09…`. Its TenderReceipt "… / Submissionsstelle" is another
name_norm and is not grouped. The target has identifier NULL, is provisional, has 80 mentions, and its city screen is clean. The
name is one municipal Eigenbetrieb. R.

**12. 22899365, Stadtverwaltung Cottbus → 1514797 (626). R.** GUID `e16f5cb1…` sits on ORG-0001 Procedure-Buyer of "VHVmöTw 302/2024
- Umbau, Fahrgestell- und Technikrefresh Feuerwehr ELW2" (TED 24446287, DÖE 26520395). Two more sections print the name: ORG-0002
Lot-AddInfo (`f443ae4f…`) and ORG-0003 Lot-ReviewInfo (`ebdcc961…`). All three give Zentrales Vergabemanagement, Neumarkt 5,
03046 Cottbus, mario.romberg@cottbus.de and +49 3556122990. The ReviewOrg (VK Brandenburg, Potsdam) and the TenderReceipt
"Vergabemarktplatz Brandenburg" are other names. A sibling planned notice, 24279422 (kommunaler Wärmeplan, rows 22771914/15/16),
repeats the triple under three fresh values with another clerk (ivonne.specht@cottbus.de). The target has identifier NULL, is
provisional, has 626 mentions, and is not city-flagged. The name is the one city administration. R.

**13. 22910304, BLB NRW Zentrale → 7195752 (1,101). R.** GUID `3c904a3b…` sits on ORG-0001 Procedure-Buyer of "BLB NRW Z/NL
Münster/Technisches Gebäudemanagement … Los 11: Dachbau" (TED 24461122, DÖE 26532756). ORG-0002 Lot-AddInfo prints the name under
`471adf20…`. Both give Servicecenter Zentraler Einkauf, Mercedesstraße 12, 40470 Düsseldorf, BLBVergabe@blb.nrw.de. The
headquarters buys here for the Münster branch, but every section of the group prints the Zentrale in Düsseldorf, and the Münster
branch is a separate name (`Bau- und Liegenschaftsbetrieb NRW Münster`). The target is in `r2_top_targets[7]`. Its listed mentions
3920878, 3733750 and 3731071 are 2009 notices by "BAU- UND LIEGENSCHAFTSBETRIEB NRW ZENTRALE", DUESSELDORF. Planned sibling rows
22807165 (R2 read_rand1 item 8, R) and 22811082 fold into it too. One headquarters body.

**14. 22957742, Stadt Remscheid, … Zentrale Vergabestelle → 16921787 (805). R.** The notice is "Zimmerer- und Holzbauarbeiten,
Außenwände Röntgen-Gymnasium Remscheid" (TED 24520618, DÖE 26543501). GUID `3e93a7f0…` sits on ORG-0001, which plays both
Procedure-Buyer and Lot-AddInfo. ORG-0002 Lot-ReviewInfo prints the name under `7882686f…`. Both give Theodor-Heuss-Platz 1, 42853
Remscheid, ausschreibung@remscheid.de, and the same placeholder phone 000. Vergabekammer Rheinland, Köln, is ReviewOrg and Mediator
under its own name, and the tenderer is "tischleraktiv", Kleve. The group is coherent: the buyer printing itself as ReviewInfo is
not a ReviewOrg mix. The target has identifier NULL, is provisional, has 805 mentions, and its city screen is clean. R2 scored a
sibling row R (read_rand2 item 19, notice 24364655, the same office under four values). The target is one city procurement office.
