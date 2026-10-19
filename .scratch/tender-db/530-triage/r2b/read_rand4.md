# 530 R2b: fresh random planned rows, items 45-59 (read_rand4)

Source: `guid_twin_plan_2660.json` (Build 1b, deployed 8e9222c, dry job 2660), `body.r2_random[45..59]` (0-based), salt
530003, R2's 60 reads excluded. Items 45-49 are the last five rows of the review_or_info stratum (the row's own section is
TenderReceipt, AddInfo or ReviewInfo). Items 50-59 are the whole "other or tenderer" stratum: 50-55 and 58 are
`Procedure-SProvider` sections ("other"), and 56, 57 and 59 are tenderer_winner rows. For every row the plan says: DE,
2 mentions (the TED and DÖE copies of one notice), 1 Tender, `planned: whole`, `refused: 2`, not in KEEP, not stable,
`twin: true`, `twin_after: false`, one target. All 12 distinct targets exist, so none of the 15 is a missing key that
Build 2 would mint.

Question: would rehoming all of the row's mentions to its target, the identifier-NULL `(name_norm, DE)` row, be
- **R**: the target is the same body, and the GUID row was a per-notice split;
- **N**: cannot tell; or
- **W**: the target name denotes more than one body, the target would receive another body's mentions, a different body is
  printed under this name in this notice's role, or the GUID is a real stable key?

**Method.**
- `/v1/organizations/{row}` for all 15 rows: each still holds its GUID, `identifier_kind` national, 2 mentions, not
  provisional. `/v1/organizations/{target}` for all 12 targets: each has identifier NULL, is provisional, is DE and carries
  exactly the row's name.
- Both copies of every notice through `/v1/notices/{id}/content` (14 notices, 28 copies). A script took every ORG section's
  name, BT-501 (DÖE: `PartyLegalEntity-CompanyID`), contact, phone, e-mail, street, postcode and city, plus the role references:
  OPT-300/301 in the TED copy, the DE1 party references in the DÖE copy, and for awards the TenderingParty / Tenderer /
  SubContractor / LotTender / LotResult chain. It matched each row's identifier to exactly one section in each copy and listed
  the twin group (the same folded name under several values in one copy). For all 15 rows both copies hold the same group under
  the same values, every value is a v4 UUID, and every value differs from notice to notice.
- Target evidence: for the 3 targets in `r2_top_targets` (8100, 1168328, 1167697) and the one city-flagged target (8808564),
  R2's reads of their newest mentions; for 16342601 and 13304619, one sibling planned notice each from the truncated
  `planned_mention_keys` (fetched here); for 20509663, 20038121 and 4791242, the sibling notices R2 / R2b rand1 already read.
  Targets 2399618, 10392760 and 1592402 have no listed mentions and no sibling in the truncated key list, so they rest on the
  name, the mention count and the city screen.
- City screen: `city.flagged` is complete (36 = `flagged_count`). Only 8808564 is on it. For the other 11 targets the
  newest-50 city set is empty or exactly the planned city, and no key's planned mentions show 2 cities.
- Role guard (A7): `roles.incoherent` is empty and `tenderer_mixed` is 0. In 12 of the 15 groups every section plays a
  buyer-side role (Buyer, SProvider, AddInfo, TenderReceipt), sometimes with Lot-ReviewInfo printing the buyer's own address.
  No group holds a Lot-ReviewOrg section; in every notice the ReviewOrg is a review chamber printed under its own name. The
  three tenderer groups (56/59, 57) hold only Tenderer, Tenderer-MainCont and Tenderer-SubCont.
- Name guard: no name is generic, an umbrella or on `GUID_TWIN_UNSAFE_NAMES`; each names one legal body or one office of one.
- **GETs:** 49 public GETs (25 `/v1/organizations/{id}`, 24 `/v1/notices/{id}/content`), all 200, no 408, no retry, logged
  as `reader=rand4` in `r2b/api_requests.log` (20:55:17Z to 21:00:40Z). They went through the shared `r2b/.api.lock` /
  `.api.last` with a 2.1 s floor; the smallest logged gap between a rand4 GET and any neighbour is 2.229 s. Reused, not
  re-fetched: notice copies 24378715, 26510744, 24445281, 26520551, 24367908 and 26494828 from `r2/bodies/`, and organizations
  20509663 and 1168328 already fetched into `r2b/bodies/` by sibling readers. Bodies are in `r2b/bodies/`. No `/v1/sql`, ssh,
  POST, git or cargo.

## Tally

**R 15, N 0, W 0.**

| # | row | name | stratum | own role | target (mentions) | verdict |
|---|---|---|---|---|---|---|
| 45 | 22846853 | Rheinland-Pfälzische Technische Universität Kaiserslautern-Landau | review_or_info | Lot-TenderReceipt | 20509663 (33) | R |
| 46 | 22898598 | Kreisstadt Unna - Zentrale Vergabestelle | review_or_info | Lot-AddInfo | 2399618 (27) | R |
| 47 | 22825834 | Medizinische Hochschule Hannover | review_or_info | Lot-ReviewInfo | 1167697 (480) | R |
| 48 | 22858540 | Stadtverwaltung Kaiserslautern - Stabstelle IV.1 Zentrale Vergabestelle | review_or_info | Lot-AddInfo | 16342601 (404) | R |
| 49 | 22838237 | Wasserverband Eifel-Rur - Zentrale Vergabestelle - | review_or_info | Lot-AddInfo | 20038121 (72) | R |
| 50 | 23096323 | Stiftung Preußische Schlösser und Gärten Berlin-Brandenburg | other | Procedure-SProvider | 8100 (721) | R |
| 51 | 22856874 | Zentraldienst der Polizei des Landes Brandenburg | other | Procedure-SProvider | 1168328 (2,154) | R |
| 52 | 22825576 | Stadt Bielefeld - Amt für Organisation, IT und Zentrale Leistungen | other | Procedure-SProvider | 13304619 (401) | R |
| 53 | 22934649 | Stadtwerke Schwerin GmbH (SWS) | other | Procedure-SProvider | 4791242 (335) | R |
| 54 | 22825013 | Zentraldienst der Polizei des Landes Brandenburg | other | Procedure-SProvider | 1168328 (2,154) | R |
| 55 | 23113381 | Stiftung Preußische Schlösser und Gärten Berlin-Brandenburg | other | Procedure-SProvider | 8100 (721) | R |
| 56 | 22942475 | Deloitte GmbH Wirtschaftsprüfungsgesellschaft | tenderer_winner | Tenderer + MainCont | 8808564 (99) | R |
| 57 | 22807887 | Götz-Gebäudemanagement RMG GmbH | tenderer_winner | Tenderer-SubCont | 10392760 (84) | R |
| 58 | 23070360 | Landesbetrieb Information und Technik NRW | other | Procedure-SProvider | 1592402 (251) | R |
| 59 | 22942480 | Deloitte GmbH  Wirtschaftsprüfungsgesellschaft | tenderer_winner | Tenderer-SubCont | 8808564 (99) | R |

Points for the tally:
- Neither R2 W shape occurs. No target is an umbrella chamber name: every review chamber in these notices sits in its own name
  group, outside the planned group. No review slot prints another body's details: the five ReviewInfo sections inside a group
  (47, 48, 51, 54, 58) print the buyer's own address, e-mail and phone.
- Items 56 and 59 are the two rows of one twin group, and both of its sections are planned to the same target. Item 49 is the
  AddInfo twin of R2's row 22838236 (R2 rand1 item 5, R), in the same notice and with the same target. Item 46 and item 45
  share notices with R2's rand4 items 49 and 47, which read the chamber groups there.
- The only city-flagged target here is 8808564 (Deloitte). It is one legal entity, and the flagged Düsseldorf mention is its
  Düsseldorf office (item 56).

## Items

**45: 22846853 "Rheinland-Pfälzische Technische Universität Kaiserslautern-Landau" (review_or_info), target 20509663: R.**
TED 00709786-2024 (notice 24378715; DÖE copy 26510744) is a 2024-11-20 contract notice from RPTU for a "DFG GG
600MHz-NMR-Konsole mit Probenkopf". The row is ORG-0006, Lot-TenderReceipt (`783b6696…`). The twins are ORG-0001
Procedure-Buyer (`5ac7a309…`) and ORG-0002 Lot-AddInfo (`0cee7b89…`). All three print Abt. 2.3 Vergabe und Beschaffung,
Gottlieb-Daimler-Straße 47, 67663 Kaiserslautern, zv-vergabeteam1@rptu.de, +49 6312052221. The Vergabekammer Rheinland-Pfalz
(Stiftsstraße 9, Mainz) holds ReviewInfo, ReviewOrg and Mediator under its own name; R2 rand4 item 47 read that group. The
SProvider is the DÖE e-sender. The target has the same name, identifier NULL, is provisional and has 33 mentions. It is not
city-flagged, and the planned city is kaiserslautern. R2b rand1 item 5 read the sibling planned notice 24331610 (two more fresh
values for the same office) and scored this target R. RPTU has been one university with two campuses since 2023, so the name
is one legal entity. The GUID is a per-notice mint. R.

**46: 22898598 "Kreisstadt Unna - Zentrale Vergabestelle" (review_or_info), target 2399618: R.** TED 00740622-2024 (notice
24445281; DÖE copy 26520551) is a 2024-12-03 award, "Bildungsstandort Hertinger Tor - Sportgeräte". The row is ORG-0002,
Lot-AddInfo (`253d3349…`). The twin is ORG-0001, Procedure-Buyer (`c645299a…`). Both print contact "Zentrale Vergabestelle",
Rathausplatz 1, 59423 Unna, vergabestelle@stadt-unna.de, +49 2303103-3025. ReviewInfo, ReviewOrg and Mediator are the
Vergabekammer Westfalen, Bezirksregierung Münster, in its own name group (R2 rand4 item 49, R). The winner Sportco GmbH, Hagen,
is a separate tenderer. The target has the same name, identifier NULL, is provisional and has 27 mentions. It is in no R2 list
and not city-flagged (planned city unna). No newest mention is listed and no sibling notice is in the truncated key list, so
the target rests on its name and the city screen. The name pins the central procurement office of the town of Unna, one body.
R.

**47: 22825834 "Medizinische Hochschule Hannover" (review_or_info), target 1167697: R.** TED 00696437-2024 (notice 24352086;
DÖE copy 26505008) is a 2024-11-14 contract notice, "MHH - Rahmenvertrag Müllbeutel aus Polyethylen". The row is ORG-0004,
Lot-ReviewInfo (`3d3e84a4…`). The twins are ORG-0001 Procedure-Buyer (`6336de40…`), ORG-0003 Lot-AddInfo (`c67c2673…`) and
ORG-0006 Lot-TenderReceipt (`3e77e3bf…`). All four print "GB IV - Logistik, Abteilung Zentraleinkauf OE 1260",
Carl-Neuberg-Straße 1, 30625 Hannover, zentraleinkauf@mh-hannover.de, +49 5115326364. The co-buyer MHH Service GmbH (ORG-0002,
mhh.service@mh-hannover.de) is a separate company under its own name and stays outside the group. The ReviewOrg is the
Vergabekammer Niedersachsen in Lüneburg (ORG-0005), under its own name. The ReviewInfo row is therefore the buyer's own
contact entry, not an A7 ReviewOrg mix. The target is MHH: identifier NULL, 480 mentions, `r2_top_targets[26]` (84 planned
rows, 168 mentions). Its city set is empty because its mentions are text-era. R2 read_top2 item 26 read its text-era notice
3223576 (Carl-Neuberg-Straße 1, Abt. Zentraleinkauf) and two planned notices of the same shape, and scored it R. One medical
school. R.

**48: 22858540 "Stadtverwaltung Kaiserslautern - Stabstelle IV.1 Zentrale Vergabestelle" (review_or_info), target 16342601:
R.** TED 00680852-2024 (notice 24394508; DÖE copy 26499656) is a 2024-11-07 contract notice, "Clearinghaus 2025-2028". The row
is ORG-0002, Lot-AddInfo (`e516b872…`). The twins are ORG-0001 Procedure-Buyer (`d4ffc29e…`), ORG-0003 Lot-ReviewInfo
(`f514f323…`) and ORG-0006 Lot-TenderReceipt (`9474e917…`). All print Willy-Brandt-Platz 1, 67657 Kaiserslautern,
vergabestelle@kaiserslautern.de, +49 6313652481. ReviewOrg and Mediator are the Vergabekammer Rheinland-Pfalz (Mainz), under
its own name. The target has the same name, identifier NULL, is provisional, has 404 mentions and is not city-flagged. The
sibling planned notice 24337483 (TED 00714026-2024, 2024-11-21, an electric roll-off truck) prints the same office at the same
address and e-mail as Buyer, AddInfo, ReviewInfo and TenderReceipt, under four other fresh values (`120704b9…`, `078de4e9…`,
`ca34806b…`, `757884c6…`). The name is the city of Kaiserslautern's central procurement unit, one body. R.

**49: 22838237 "Wasserverband Eifel-Rur - Zentrale Vergabestelle -" (review_or_info), target 20038121: R.** TED 00667963-2024
(notice 24367908; DÖE copy 26494828) is a 2024-10-31 contract notice, "ST - Herstellung einer Hochwasserschutzmauer an der Wurm
in Geilenkirchen". The row is ORG-0002, Lot-AddInfo (`a8ba7282…`). The twins are ORG-0001 Procedure-Buyer (`cc363af1…`, which
is R2's row 22838236, rand1 item 5, R) and ORG-0005 Lot-TenderReceipt (`6e00e56b…`). All three print Eisenbahnstraße 5, 52353
Düren, ZentraleVergabe@wver.de, +49242149491133. ORG-0003, "Zentrale Vergabestelle des Wasserverband Eifel-Rur (WVER)"
(ReviewInfo, same address), is the same office under another spelling. It stays outside the group, which is an under-merge,
not a wrong one. The ReviewOrg is the Vergabekammer Rheinland in Köln. The target has the same name, identifier NULL and 72
mentions. Its city screen is clean (düren), and R2 rand1 items 4 and 5 read it (sibling notice 24296267). This is the same
group and the same target that R2 scored R for this notice's buyer section. R.

**50: 23096323 "Stiftung Preußische Schlösser und Gärten Berlin-Brandenburg" (other: Procedure-SProvider), target 8100: R.**
TED 00207778-2025 (notice 24719114; DÖE copy 26615206) is a 2025-03-31 contract notice, "Sicherung- und Schutzmaßnahmen Schloss
Cecilienhof". SPSG is its own service provider. The row is ORG-0002, Procedure-SProvider (`8b93ceff…`). The twins are ORG-0001
Procedure-Buyer (`85f8e15a…`), ORG-0003 Lot-AddInfo (`c02ae3e8…`) and ORG-0005 Lot-TenderReceipt (`c8542131…`). All print
"Vergabestelle", Allee nach Sanssouci 6, 14471 Potsdam, vergabestelle@spsg.de, +49 3319694-0. The ReviewOrg is the Vergabekammer
des Landes Brandenburg beim MWAE, under its own name. The target has the same name, identifier NULL and 721 mentions. It is
`r2_top_targets[1]` (358 rows, 716 mentions) with city set {potsdam}. R2 read_top1 item 1 read its newest mentions
(47414351, 47411263: SPSG at the same address as buyer and SProvider) and scored it R. A foundation under public law with a
unique name, one body. R.

**51: 22856874 "Zentraldienst der Polizei des Landes Brandenburg" (other: Procedure-SProvider), target 1168328: R.** TED
00669364-2024 (notice 24392283; DÖE copy 26494954) is a 2024-11-01 contract notice for Feuerwehr work clothing. The DÖE copy
names the project "Arbeitshemden, Arbeitsblusen", but it has the same issue time and the same ORG values, so it is the same
notice. Here the ZDPol buys for itself. The row is ORG-0002, Procedure-SProvider (`3ee40bc1…`). The twins are ORG-0001
Procedure-Buyer (`fde4e7c6…`), ORG-0003 Lot-AddInfo (`b83cd52b…`), ORG-0004 Lot-ReviewInfo (`958d691c…`) and ORG-0007
Lot-TenderReceipt (`ce0f61b5…`). All print Am Baruther Tor 20, 15806 Zossen and vergabestelle.zdpol@polizei.brandenburg.de,
with the placeholder phone +49 000. ReviewOrg and Mediator are "Vergabekammer des Landes Brandenburg" (Heinrich-Mann-Allee 107,
Potsdam), under its own name. The target has the same name, identifier NULL and 2,154 mentions. It is `r2_top_targets[5]`
(218 rows, 436 mentions) with city set {zossen}. R2 read_top1 item 5 and R2 rand4 item 50 read it, R. One Land authority. R.

**52: 22825576 "Stadt Bielefeld - Amt für Organisation, IT und Zentrale Leistungen" (other: Procedure-SProvider), target
13304619: R.** TED 00699036-2024 (notice 24351728; DÖE copy 26504968) is a 2024-11-14 contract notice, "Grundschule Plaß_OGS
Ausbau_Fliesen und Plattenarbeiten". The row is ORG-0002, Procedure-SProvider (`d3fd83ce…`). The twins are ORG-0001
Procedure-Buyer (`ea34ae78…`), ORG-0003 Lot-AddInfo (`09750e22…`) and ORG-0006 Lot-TenderReceipt (`9c32c8b4…`). All print
"Zentrale Vergabestelle", Werner-Bock-Str. 38, 33602 Bielefeld, mario.guenther@bielefeld.de, +49 521515097. ReviewInfo and
ReviewOrg are the Vergabekammer Westfalen bei der Bezirksregierung Münster, under its own name. The target has the same name,
identifier NULL, is provisional, has 401 mentions and is not city-flagged. The sibling planned notice 24335826 (TED
00713972-2024, 2024-11-21, the same school's drywall lot) prints the same office at Werner-Bock-Str. 38 as Buyer, AddInfo and
TenderReceipt, under three other fresh values (`cabc6ee8…`, `16c4260f…`, `94412f2f…`) and with another clerk's e-mail. One
office of the city of Bielefeld. R.

**53: 22934649 "Stadtwerke Schwerin GmbH (SWS)" (other: Procedure-SProvider), target 4791242: R.** TED 00022352-2025 (notice
24491770; DÖE copy 26547701) is a 2025-01-13 award for general planning of the Kantor-Carl-Ehrich school extension in Plau am
See. The buyer is Stadt Plau am See (ORG-0001, Markt 2, 19395 Plau am See). It gives SWS's submission@swsn.de as its e-mail,
but it has its own name, so it is outside the group. The row is ORG-0002, Procedure-SProvider (`a9f11ff6…`). The twin is
ORG-0003, Lot-AddInfo (`107f4a93…`). Both print "Vergabestelle", PSF 160205, 19092 Schwerin, submission@swsn.de,
+49 3856331491. The ReviewOrg is the Vergabekammer Mecklenburg-Vorpommern, and the winner is Lemke-Uphaus GmbH with its
subcontractors; each sits in its own name group. The target has the same name, identifier NULL and 335 mentions, and is not
city-flagged. R2 rand4 items 56 and 58 read sibling notices 24274885, 24263428 and 24335203, where SWS is SProvider for other
Schwerin clients at the same PO box and e-mail, and scored it R. SWS runs procurement for its clients, who keep their own
names. One GmbH. R.

**54: 22825013 "Zentraldienst der Polizei des Landes Brandenburg" (other: Procedure-SProvider), target 1168328: R.** TED
00696108-2024 (notice 24351117; DÖE copy 26504953) is a 2024-11-13 contract notice, "Vorausrüstwagen (VRW) für die Stadt
Liebenwalde". The buyer is "Liebenwalde - Stadt am Finowkanal" (Marktplatz 20), a different name that stays outside the group.
The row is ORG-0002, Procedure-SProvider (`b0b2f058…`). The twins are ORG-0003 Lot-AddInfo (`105c2007…`), ORG-0004
Lot-ReviewInfo (`50027db8…`) and ORG-0007 Lot-TenderReceipt (`4231c091…`). All print Am Baruther Tor 20, 15806 Zossen and
vergabestelle.zdpol@polizei.brandenburg.de. ReviewOrg and Mediator are the Vergabekammer des Landes Brandenburg, under its own
name. The target and its evidence are as in item 51. The ZDPol procures for a Brandenburg town, and the fold keeps that role
on the body that performs it. R.

**55: 23113381 "Stiftung Preußische Schlösser und Gärten Berlin-Brandenburg" (other: Procedure-SProvider), target 8100: R.**
TED 00221470-2025 (notice 24744260; DÖE copy 26621630) is a 2025-04-04 contract notice, "Restauratorische Putz- und
Stuckarbeiten BA1+BA2 Orangerieschloss". The row is ORG-0002, Procedure-SProvider (`41b0f2cb…`). The twins are ORG-0001
Procedure-Buyer (`c7b9c6fb…`), ORG-0003 Lot-AddInfo (`0fdb6ff2…`) and ORG-0005 Lot-TenderReceipt (`ac9ece00…`). All print
"Vergabestelle", Allee nach Sanssouci 6, 14471 Potsdam, vergabestelle@spsg.de, https://spsg.de. The ReviewOrg is the
Vergabekammer des Landes Brandenburg beim MWAE, under its own name. The target and its evidence are as in item 50. R.

**56: 22942475 "Deloitte GmbH Wirtschaftsprüfungsgesellschaft" (tenderer_winner), target 8808564: R.** TED 00013099-2025
(notice 24501447; DÖE copy 26544642) is the 2025-01-07 award of d-NRW AöR's multi-supplier framework "RV 2024-2028 OZG und
E-Government" (7 lots, 60 ORG sections). The row is ORG-0013 (`d3bd683d…`), OPT-300-Tenderer of TPA-0003 and its MainCont,
at Schwannstrasse 6, 40476 Düsseldorf, ausschreibungen@deloitte.de, 0211 8772 2429. TPA-0003's tender TEN-0007 is among the
LOT-0005 results. The twin is ORG-0018 (`e21d44ab…`, item 59), printed "Deloitte GmbH  Wirtschaftsprüfungsgesellschaft"; the
double space folds to the same key. ORG-0018 is a Tenderer-SubCont in TPA-0004, led by Deloitte Consulting GmbH (ORG-0015,
Berlin), whose tenders are among the results of lots 1, 2, 3 and 6. ORG-0018 gives 81669 München, kontakt@deloitte.de,
089 29036-0. Deloitte GmbH WPG is one audit firm, seated in München, with a Düsseldorf office. Deloitte Consulting GmbH (ORG-0014
and ORG-0015) is a different legal entity under a different name and key: target 1289810, which R2 city3 item 30 read as R. It
is outside this group, so the fold takes nothing of it. The group plays only Tenderer, MainCont and SubCont roles, consistent
with `tenderer_mixed` 0. The target has identifier NULL, is provisional and has 99 mentions. It is `r2_city_flagged[26]`: its
set is {münchen}, the planned cities are {düsseldorf, münchen}, and the one absent city is this row's Düsseldorf. R2 read_city3
item 31 read its newest mentions and scored it R: 26985701 prints Tenderer, Rosenheimer Platz 4, 81669 München,
ausschreibungen@deloitte.de, with printed BT-501 DE129274148 (the firm's real key, not a GUID), and 26529959 and 26146632 print
the same München address. The row's e-mail ausschreibungen@deloitte.de is the target's own. One legal entity; the GUID is a
per-notice mint. R.

**57: 22807887 "Götz-Gebäudemanagement RMG GmbH" (tenderer_winner), target 10392760: R.** TED 00665175-2024 (notice 24330354; DÖE
copy 26493556) is a 2024-10-30 award, "Unterhaltsreinigung TU Darmstadt" (7 lots). The row is ORG-0010 (`890dd045…`), a
Tenderer-SubCont under main contractor Spelters Gebäudereinigung GmbH (TPA-0004, whose tender is the LOT-0004 result), at
Justus-von-Liebig-Straße 24-26, 63128 Dietzenbach, frankfurt@goetz-fm.com, +49 60743017880. The twin is ORG-0006 (`8a02fc66…`,
row 22807883, also planned whole to the same target), the Tenderer of TPA-0001 (results of lots 6 and 2). It prints the same
street ("Justus-von-Liebig-Str. 24 - 26") in 63128 Dietzenbach, with the group's Regensburg bid desk as contact (0941 64040,
VID-Regensburg03@goetz-dl.de). Same company name, same registered address; the two sections are one firm bidding alone on two
lots and as a subcontractor on a third. The group plays only Tenderer and SubCont roles. The target has the same name,
identifier NULL, is provisional and has 84 mentions. It is not city-flagged, so its city set is empty or {dietzenbach}. No
mention is listed, so it rests on the name and the screen. "RMG" (Rhein-Main) names one GmbH of the Götz group, and its
sister companies carry other names, so they cannot fall to this key. R.

**58: 23070360 "Landesbetrieb Information und Technik NRW" (other: Procedure-SProvider), target 1592402: R.** TED 00231266-2025
(notice 24680705; DÖE copy 26624027) is a 2025-04-08 award, "Checkpoint Rahmenvertrag". The row is ORG-0002,
Procedure-SProvider (`2d9a1105…`). The twins are ORG-0001, Procedure-Buyer and Lot-AddInfo (`1db823f7…`), and ORG-0003,
Lot-ReviewInfo (`e163c7e5…`). All print "Z3 Sourcing", Mauerstraße 51, 40476 Düsseldorf, ausschreibung@it.nrw.de,
+49 2119449-01, https://www.it.nrw/. The ReviewOrg is the Vergabekammer Rheinland (Köln), under its own name. The two Bechtle
GmbH winner sections are a separate group. The target has the same name, identifier NULL and 251 mentions. It is not
city-flagged, so its city set is empty or {düsseldorf}. No mention or sibling is listed, so it rests on the name and the screen.
IT.NRW is one Landesbetrieb of the Land NRW with a unique name. R.

**59: 22942480 "Deloitte GmbH  Wirtschaftsprüfungsgesellschaft" (tenderer_winner), target 8808564: R.** This is the second
section of item 56's group, in the same notice copies (24501447 / 26544642). The row is ORG-0018 (`e21d44ab…`), a
Tenderer-SubCont in TPA-0004 under Deloitte Consulting GmbH. It is printed with a double space that folds to the target's key,
at 81669 München, kontakt@deloitte.de, 089 29036-0. München is the target's own city, and 81669 is the postcode of the
Rosenheimer Platz 4 seat printed on the target's newest mentions. The subcontractor slot names Deloitte GmbH WPG itself, not
the lead tenderer and not another Deloitte company. Deloitte Consulting GmbH appears in this notice under its own name (ORG-0014
in München, ORG-0015 in Berlin) and is not in the group. The target and its evidence are as in item 56: one legal entity, and
the GUID is a per-notice mint. R.
