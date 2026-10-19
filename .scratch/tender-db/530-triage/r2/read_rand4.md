# 530 R2: random planned GUID rows, items 45-59 (read_rand4)

Scope: `body.r2_random[45..59]` (0-based) of `guid_twin_plan_2659.json` (salt 530002). Items 45-49 are the last five rows
of the review_or_info stratum. Items 50-58 are the "other" stratum, and every one of them is a `Procedure-SProvider`
section. Item 59 is the one tenderer_winner row. All 15 rows are DE, have 2 mentions (the TED and DÖE copies of one
notice), span 1 Tender, are planned `whole`, are not in KEEP and are not stable. All have `twin_after=false` and one
target each.

Question: if all the row's mentions move to its target (the identifier-NULL row for (name_norm, DE)), is that
- **R**: the target is the same body and the GUID was a per-section mint;
- **N**: cannot tell; or
- **W**: the target name covers more than one body, the target would take another body's mentions, or the GUID is a real
  stable key?

**Method.**
- For each row I made these GETs:
  - `/v1/organizations/{row}` and `/v1/organizations/{target}`;
  - both copies of the notice through `/v1/notices/{id}/content`.
- From the notices I took every ORG section's name, BT-501, street, postcode, city, e-mail and contact, plus the role
  references:
  - TED copy: OPT-300/301;
  - DÖE copy: the `DE1-…PartyIdentification-ID` references, with the name and CompanyID in its `PartyName` /
    `PartyLegalEntity` subsections.
- A script matched each row's identifier to the dashed lowercase BT-501 of exactly one section. It did so in both copies,
  and it listed the twin group (the same name under several values in one copy). Both copies carry the same group and the
  same values for all 15 rows.
- Evidence for the targets:
  - For the 6 targets in `r2_top_targets` (11823847, 1166261, 5880993, 1168328, 3796660, 8100), I read the target's
    newest listed notice.
  - For 3 targets with no listed notices (1648483, 17833093, 4791242), I read a sibling planned notice from
    `planned_mention_keys`.
  - The other 5 targets (1310644, 6441314, 1341058, 1213069, 13150074) rest on the name, the mention count and the city
    screen.
- City screen: `city.flagged` is complete (41 = `flagged_count`), and **none of these 14 targets is on it**. So no
  planned city is missing from a non-empty newest-50 target set, no target set has 2 or more cities, and no key's planned
  mentions show 2 or more cities.
- None of the 15 groups is in `roles.incoherent` (A7), and none is in `tenderer_mixed`.
- **GETs:** 67, all 200, no 408s, logged as `reader=rand4` in `r2/api_requests.log`. The logger shares `.api.lock` /
  `.api.last` with the other readers, so all readers' GETs are at least 2.2 s apart. 1166261 was already cached
  (fetched by rand3). Bodies are in `r2/bodies/`.

## Tally

**R 15, N 0, W 0.**

Every row's twin group is one body printed once per role, with identical address and e-mail in every section of the group.
Every target is identifier NULL, provisional, DE, carries exactly the row's name (case-folded), and shows the same address
wherever I could read its mentions.

Points for other readers:
- The "other" stratum here is entirely `Procedure-SProvider`. In 6 of the 9 rows the service provider is a different body
  from the buyer (ZDPol, POT, Facility Care, the LBM Vergabeprüfstelle, SWS ×2). The buyer has its own name, so it is
  outside the group and outside the fold.
- One target's empty city set (Stolberg, 3796660) comes from TED-era mentions with no BT-513, not from missing data.

## Items

**45: 22851601 "Stadt Falkensee" (review_or_info), target 1310644: R.**
TED notice 24385094 (DÖE copy 26507184) is a 2024-11-15 cn-standard for "Neubau Sporthalle Oberschule Los 33". The row
is ORG-0004, Lot-TenderReceipt (`fadfbd13…`). The twins are ORG-0001 Procedure-Buyer (`1f336c90…`) and ORG-0002
Lot-AddInfo (`5405dd93…`). All three give Falkenhagener Straße 43/49, 14612 Falkensee, hochbau@falkensee.de and, in the
DÖE copy, contact "Fachbereich Hochbau". The review body is a different name (the Vergabekammer Brandenburg, Potsdam).
Target 1310644 is "Stadt Falkensee", identifier NULL, 254 mentions, and is not city-flagged. Falkensee is a single town
in Germany (Havelland). One municipality, so the GUID is a per-section mint.

**46: 22974691 "Vergabekammer Rheinland c/o Bezirksregierung Köln" (review_or_info), target 11823847: R.**
TED notice 24542054 (DÖE 26564547) is a 2025-01-31 award notice from Polizeipräsidium Aachen (towing services). The row
is ORG-0002, Lot-ReviewInfo (`97f96ddb…`). The twins are ORG-0003 Lot-ReviewOrg and ORG-0004 Lot-Mediator. All three give
Zeughausstraße 2-10, 50667 Köln, VKRheinland@bezreg-koeln.nrw.de. Target 11823847 has the same name, identifier NULL and
6,223 mentions. It is a top target (148 planned rows, 296 mentions) with city set {koln}. Its newest notice 47402058
(2026-10-13, Regionalverband Ruhr) prints the same chamber at the same address and e-mail, with a phone number as BT-501
(`t:00492211470`). The same chamber also exists under other name keys: 7615 "Vergabekammer Rheinland" and "… bei der
Bezirksregierung Köln". Those are separate keys and the fold does not touch them, so this leaves a fragmentation, not a
homonym. One review chamber.

**47: 22846852 "Vergabekammer Rheinland-Pfalz beim Ministerium für Wirtschaft, Verkehr, Landwirtschaft und Weinbau" (review_or_info), target 1166261: R.**
TED notice 24378715 (DÖE 26510744) is a 2024-11-20 cn-standard from RPTU Kaiserslautern-Landau (NMR console). The row is
ORG-0005, Lot-Mediator (`41bb7b56…`). The twins are ORG-0003 Lot-ReviewInfo and ORG-0004 Lot-ReviewOrg. All give
Stiftsstraße 9, 55116 Mainz, vergabekammer.rlp@mwvlw.rlp.de and, in the DÖE copy, contact "Geschäftsstelle". The buyer's
own three sections (buyer, AddInfo, TenderReceipt) are a separate name group. Target 1166261 has the same name, identifier
NULL and 8,500 mentions, with city set {mainz} (130 planned rows). Its newest notice 47414270 (2026-10-16, Stadt
Pirmasens) prints the chamber at Stiftsstr. 9, Mainz, with BT-501 "keine Angabe". There is one Vergabekammer for the
Land, so this is R.

**48: 22865917 "Vergabekammer des Landes Brandenburg beim Ministerium für Wirtschaft und Energie" (review_or_info), target 5880993: R.**
TED notice 24403517 (DÖE 26528034) is a 2024-12-11 award notice from ProPotsdam GmbH. The row is ORG-0002,
Lot-ReviewInfo (`238124b1…`). The twin is ORG-0003, Lot-ReviewOrg (`4085060d…`). Both give Heinrich-Mann-Allee 107,
14473 Potsdam, vergabekammer@mwae.brandenburg.de. Target 5880993 has the same name, identifier NULL and 14,414 mentions,
with city set {potsdam} (96 planned rows). Its newest notice 47414670 (2026-10-16, Gemeinde Michendorf) prints the same
chamber twice (ReviewInfo and ReviewOrg) at the same address, with phone numbers as BT-501. The chamber's other
ministry-name variants (MWAE "Wirtschaft, Arbeit und Energie", "Wirtschaft und Europaangelegenheiten", bare "Vergabekammer
des Landes Brandenburg") seen in items 50, 51, 53 and 57 are different keys and stay apart. One body.

**49: 22898599 "Vergabekammer Westfalen, Bezirksregierung Münster" (review_or_info), target 6441314: R.**
TED notice 24445281 (DÖE 26520551) is a 2024-12-03 award notice from Kreisstadt Unna (sports equipment). The row is
ORG-0003, Lot-ReviewInfo (`e81327d2…`). The twins are ORG-0004 Lot-ReviewOrg and ORG-0005 Lot-Mediator. All give
Albrecht-Thaer-Str. 9, 48147 Münster, vergabekammer@bezreg-muenster.nrw.de. Target 6441314 has the same name, identifier
NULL and 307 mentions. It is in no R2 target list and is not city-flagged. The name pins the chamber to the Bezirksregierung
Münster, the one Vergabekammer Westfalen. The bare two-word key "Vergabekammer Westfalen" is a different key, read
separately in `r2_office_two_words`. R.

**50: 23029156 "Zentraldienst der Polizei des Landes Brandenburg" (other: Procedure-SProvider), target 1168328: R.**
TED notice 24619312 (DÖE 26594848) is a 2025-03-07 cn-standard. The buyer is a different body: Landesbetrieb
Straßenwesen Brandenburg, Hoppegarten. The ZDPol procures for it. The row is ORG-0002, Procedure-SProvider
(`b231ceef…`). The twins are ORG-0003 Lot-AddInfo, ORG-0004 Lot-ReviewInfo and ORG-0007 Lot-TenderReceipt. All give Am
Baruther Tor 20, 15806 Zossen, vergabestelle.zdpol@polizei.brandenburg.de. Target 1168328 has the same name, identifier
NULL and 2,154 mentions, with city set {zossen} (218 planned rows). Its newest notice 45879415 (2026-07-10) is the same
shape: LS Brandenburg buyer, ZDPol as SProvider, AddInfo and ReviewInfo at the same address, with placeholder BT-501s
"1237867" and "123456789". It is one Land authority. The buyer has its own name, so the fold never touches it. R.

**51: 22859791 "IHP GmbH - Leibniz-Institut für innovative Mikroelektronik" (other: Procedure-SProvider), target 1648483: R.**
TED notice 24396035 (DÖE 26499552) is a 2024-11-07 cn-standard for clean-room clothing. The row is ORG-0002,
Procedure-SProvider (`21e2ce74…`). The twins are ORG-0001 Procedure-Buyer, ORG-0003 Lot-AddInfo and ORG-0007
Lot-TenderReceipt. All give Im Technologiepark 25, 15236 Frankfurt (Oder), rohner@ihp-microelectronics.com, DÖE contact
"Beschaffung". The institute is its own buyer and its own service provider. Target 1648483 is "IHP GmbH -
Leibniz-Institut für Innovative Mikroelektronik", which differs only in the case of "Innovative" and folds to the same
name_norm. It has identifier NULL and 149 mentions. The sibling planned notice 24265055 (2024-10-28, maintenance
contract) shows the same four-section group at the same address and e-mail. One GmbH. R.

**52: 23011460 "Kupferstadt Stolberg (Rhld.)" (other: Procedure-SProvider), target 3796660: R.**
TED notice 24592945 (DÖE 26573037) is a 2025-02-11 award notice for the sanitary works of the historic town hall. The
row is ORG-0002, Procedure-SProvider (`29708471…`). The twins are ORG-0001 Procedure-Buyer+Lot-AddInfo and ORG-0003
Lot-ReviewInfo. All give Rathausstr. 11-13, 52222 Stolberg, vergabestelle@stolberg.de, DÖE contact "A65.5 - Zentrale
Vergabestelle". The review body is the Vergabekammer Rheinland, a different name. Target 3796660 has the same name,
identifier NULL and 52 mentions. It is a top target (85 planned rows, 170 mentions), and its city set is **empty**. The
target's newest listed notice 23528675 shows why: it is a 2023 TED-form notice (sludge disposal), which carries
TED-TOWN "Stolberg" and no BT-513. It has the same Rathausstr. 11-13, 52222 address and the same e-mail. One town. R.

**53: 22831663 "POT Project on time GmbH & Co. KG" (other: Procedure-SProvider), target 17833093: R.**
TED notice 24359826 (DÖE 26514111) is a 2024-11-25 award notice. The buyer is GLG Gesellschaft für Leben und Gesundheit
mbH, Eberswalde, a different body. POT, a procurement consultancy, runs the procedure. The row is ORG-0002,
Procedure-SProvider (`480be65a…`). The twin is ORG-0003, Lot-AddInfo. Both give Buchenweg 10, 58285 Gevelsberg,
vergabe@projectontime.de. Target 17833093 has the same name, identifier NULL and 78 mentions, and is not city-flagged.
The sibling planned notice 24311562 (2024-10-29) has a different client, Kreisklinik Fürstenfeldbruck, and shows the same
POT SProvider+AddInfo pair at the same Gevelsberg address and e-mail. That is one consultancy serving many buyers. Only
POT's mentions move. R.

**54: 22987736 "Facility Care AG" (other: Procedure-SProvider), target 1341058: R.**
TED notice 24559608 (DÖE 26585135) is a 2025-02-24 award notice. The buyer is DMT Gesellschaft für Lehre und Bildung
mbH, Bochum, a different body. The row is ORG-0002, Procedure-SProvider (`022e8f5f…`). The twin is ORG-0003,
Lot-AddInfo. Both give Fridtjof-Nansen-Weg 7, 48155 Münster, info@facility-care.com. Target 1341058 has the same name,
identifier NULL and 71 mentions. It is in no target list and is not city-flagged, so its newest-50 mentions show no city
other than Münster. The name is a registered AG firm. I know of no second German "Facility Care AG", and the clean city
screen argues against one inside the target. This is the weakest-evidenced R of the 15: I read no target mention
directly, and the name is fairly generic for a company. It is still an R on the name plus the screen, not an N.

**55: 22845186 "Vergabeprüfstelle beim Landesbetrieb Mobilität Rheinland-Pfalz" (other: Procedure-SProvider), target 1213069: R.**
TED notice 24377002 (DÖE 26510428) is a 2024-11-19 cn-standard for safety shoes. The buyer is "LBM Rheinland-Pfalz
Zentrale", a different name with its own three-section group. The row is ORG-0002, Procedure-SProvider (`a2f035ac…`). The
twins are ORG-0004 Lot-ReviewInfo and ORG-0006 Lot-Mediator. All give Friedrich-Ebert-Ring 14-20, 56068 Koblenz,
lbm@lbm.rlp.de. The ReviewOrg is the Vergabekammer RLP in Mainz, a different name. Printing the review office in the
service-provider slot is the platform's role misuse, not a second body: the three sections share address, e-mail and name.
Target 1213069 has the same name, identifier NULL and 609 mentions, and is not city-flagged. The name pins it to the one
LBM. One unit. R.

**56: 22811966 "Stadtwerke Schwerin GmbH (SWS)" (other: Procedure-SProvider), target 4791242: R.**
TED notice 24335203 (DÖE 26511656) is a 2024-11-20 award notice for road planning at the Paulskirche. The buyer is LGE
Mecklenburg-Vorpommern GmbH, a different name. The row is ORG-0002, Procedure-SProvider (`1b1fccb5…`). The twin is
ORG-0003, Lot-AddInfo. Both give PSF 160205, 19092 Schwerin, submission@swsn.de, DÖE contact "Vergabestelle". Target
4791242 has the same name, identifier NULL and 335 mentions, and is not city-flagged. The sibling planned notice 24274885
(2024-10-25) has a third client, WGS Wohnungsgesellschaft Schwerin, and shows SWS as SProvider, AddInfo and
TenderReceipt at the same PO box and e-mail. SWS runs procurement for the Schwerin municipal group. The clients keep their
own names and are not folded. One GmbH. R.

**57: 23070066 "Stiftung Preußische Schlösser und Gärten Berlin-Brandenburg" (other: Procedure-SProvider), target 8100: R.**
TED notice 24680322 (DÖE 26624305) is a 2025-04-08 cn-standard for ventilation at the Orangerieschloss. The row is
ORG-0002, Procedure-SProvider (`f6186caa…`). The twins are ORG-0001 Procedure-Buyer, ORG-0003 Lot-AddInfo and ORG-0005
Lot-TenderReceipt. All give Allee nach Sanssouci 6, 14471 Potsdam, vergabestelle@spsg.de, DÖE contact "Vergabestelle".
Target 8100 has the same name, identifier NULL and 721 mentions, with city set {potsdam}. It is the top target that gains
the most relative to its size: 358 planned rows, 716 planned mentions. Its newest notice 47414351 (2026-10-16) prints
SPSG as buyer and SProvider at the same address, with junk BT-501s "Allee" and "DE138408360_". One foundation. R.

**58: 22760026 "Stadtwerke Schwerin GmbH (SWS)" (other: Procedure-SProvider), target 4791242: R.**
TED notice 24263428 (DÖE 26483581) is a 2024-10-21 award notice for a winter-service framework. The buyer is "SDS -
Stadtwirtschaftliche Dienstleistungen Schwerin, Eigenbetrieb der Landeshauptstadt Schwerin", a different name. The row is
ORG-0002, Procedure-SProvider (`4d3423cb…`). The twin is ORG-0003, Lot-AddInfo. Both give PSF 160205, 19092 Schwerin,
submission@swsn.de. The row is the first planned key for target 4791242 in `planned_mention_keys`. The target and its
evidence are the same as item 56. R.

**59: 22933591 "P.S.A. Bauunternehmung GmbH" (tenderer_winner), target 13150074: R.**
TED notice 24490391 (DÖE 26547807) is a 2025-01-13 award notice from Stadt Leverkusen for the façade works at
Werner-Heisenberg-Gymnasium. LOT-0001 has one LotResult (selec-w) that points to 5 tenders and 5 contracts, each by its
own tendering party. Each party is a separate ORG section named "P.S.A. Bauunternehmung GmbH" with its own v4 UUID:
ORG-0004 (this row, `616a229a…`, Tenderer#0) through ORG-0008 (Tenderer#4). All five give Hauptstr. 24, 53819
Neunkirchen-Seelscheid, kontakt@psa-gmbh.de, so the platform minted one UUID per contract for one company. The group is
all tenderer roles, so it is not `tenderer_mixed`. Target 13150074 has the same name, identifier NULL and 8 mentions, and
is not city-flagged. One construction company. R. A side effect, not an identity issue: once folded, the 5 contracts show
one winner org 5 times.
