# 530 R2: role-incoherent groups, items 15-29 (read_inc2)

Source: `guid_twin_plan_2659.json` `body.r2_incoherent[15..29]` (0-based; read_inc1 holds 0-14). Key = `notice_id|name_norm`.
Each item is a twin group refused by rule 530-T whose sections mix a buyer-family role with Lot-ReviewOrg (A7). All 15 are
DE. They cover 6 names and 6 procedures: one copy of the JVA Rheinbach award, then IKK classic, KWA Regio, GSI and Kerpen in
both copies (TED + DÖE), and the Handwerkskammer Niederbayern-Oberpfalz procedure in three notices × two copies. Every org id
in these groups is in `planned_row_ids` except 22331352. That row is the Handwerkskammer's own KEEP value
(stable+multi_tender, 116 mentions, 27 Tenders), so groups 24-29 refuse ORG-0002 only. In total the 15 items plan 12 rows
whole: 11 rows at 2 mentions and 23055099 at 6, which makes 28 mentions.

**Question (A7).** Is refusing every non-KEEP section of the group right, because one body was printed once per role
(**R**)? Or is it wrong, because two different bodies share the name in that notice, for example a buyer and a separate
review body (**W**)? This includes inc1's AOK shape (item 8): a review-body section that keeps the buyer's name but carries
another body's phone or NUTS.

**Method.**
- `/v1/notices/{id}/content` for every notice in the range. The TED copies and some DÖE copies use eForms BT fields, with
  roles from OPT-300/OPT-301. The other DÖE copies use eforms-de-1.1 DE1 fields, with roles from `ContractingParty`,
  `AdditionalInformationParty`, `AppealReceiverParty` (= Lot-ReviewOrg), `AppealInformationParty` and `MediationParty`.
- Each section's BT-501 / DE1 `CompanyID` matched the report's `values`, in order, in all 15 items.
- A script compared every refused section with the buyer section field by field, after mapping both formats onto the same
  fields: name, street, postcode, city, NUTS, country, e-mail, phone, fax, URL, contact and department. Code-valued fields
  (NUTS, country) sit under `code`, not `value`.
- `/v1/organizations/{id}` for every GUID row in the groups, plus the targets the report names: IKK 1206438 (via
  `r2_city_flagged`) and Kerpen 2383569 (via `r2_random`).
- The JVA Rheinbach, KWA, GSI and Handwerkskammer keys lie beyond the truncated `planned_mention_keys` (cut at notice
  24341274). They are not in any sample list, including `missing_target_samples`, and the allowed endpoints have no name
  lookup. So their targets were not located, and for those keys it is unknown whether a target exists or is missing. That
  does not change the A7 question, which is about the sections of the group.

**GETs.** 22 public GETs tagged `inc2`: 12 notice contents and 10 organizations, between 18:22:26Z and 18:23:29Z.
- All returned 200, with no 408 and no retry.
- They were paced at least 2.1 s after any reader's last GET, behind the shared `.api.lock`/`.api.last`, and every one is
  logged in `api_requests.log`. Bodies are in `bodies/`.
- Reused from other readers, not fetched again:
  - notice 26549982 and org 22963581 (inc1);
  - notices 24345339 and 26508153, and orgs 22820369, 22820370 and 1206438 (city3);
  - org 2383569 (rand1).

## Tally

**Items 15-29: R 15, N 0, W 0.**

- In every item the refused section is the buyer printing itself into the review slot. Its phone, NUTS, postcode and city
  are always the buyer's own.
  - In the 9 items 15-23 (JVA, IKK, KWA, GSI, Kerpen) every other field is also identical, with two small exceptions. Kerpen's buyer
    section alone carries the URL. KWA's ReviewOrg fax drops one digit of the buyer's fax (`+492842940100` →
    `+49284940100`).
  - In the 6 Handwerkskammer items the review section is the chamber's general contact, entered separately: `info@hwkno.de`
    and "Ditthornstr. 10", where the buyer section has a named officer and "Ditthornstrasse 10". Every value is still the
    chamber's own.
- Where the real review chamber is printed, it has its own name and its own section, so it falls in another group. That is
  the Vergabekammer des Bundes for GSI and the Vergabekammer Rheinland for Kerpen. No refused section carries a review
  chamber's phone or NUTS, which is what made inc1's AOK group W.

**Combined with read_inc1 (all 30 A7 items): R 28, W 2.** The two W are the AOK Niedersachsen group in its TED and DÖE
copies (inc1 items 8 and 12). A7 says any W puts a role-coherence guard into `guid_twin_refusals` before Build 2. The guard's
shape decides what happens to these 28 R items:

- **Keyed on the role set alone** (buyer-family + Lot-ReviewOrg ⇒ bind): all 28 R items stay split. From this half that is
  12 rows and 28 mentions.
- **inc1's exact-agreement variant** (refuse only when phone, e-mail, NUTS, street, postcode and city all agree): it would
  wrongly bind the 6 Handwerkskammer items, because the e-mail and the street spelling differ there.
- **A guard on phone + NUTS + postcode + city**: it still binds AOK, whose ReviewOrg phone and NUTS are the VK Bund's. It
  refuses all 28 R items. It must not compare fax, URL, contact or department, given KWA's fax typo and Kerpen's URL.

| # | notice | name | GUID rows (mentions) | sections and roles | target | refused section vs buyer | verdict |
|---|---|---|---|---|---|---|---|
| 15 | 26549982 (DÖE, award) | Justizvollzugsanstalt Rheinbach | 22963579/80/81 (2 each) | 0001 Buyer, 0002 AddInfo, 0003 AppealReceiver (ReviewOrg) | not located | identical | R |
| 16 | 24345339 (TED, change) | IKK classic | 22820369/70 (2 each) | 0001 Buyer+AddInfo, 0002 ReviewOrg | 1206438 (2,117) | identical | R |
| 17 | 24441480 (TED, award) | Kreis Weseler Abfallgesellschaft Regio mbH | 22895799/800 (2 each) | 0001 Buyer+AddInfo, 0002 ReviewOrg | not located | identical except a fax typo | R |
| 18 | 24460436 (TED, award) | GSI Helmholtzzentrum für Schwerionenforschung GmbH | 22909793/95 (2 each) | 0001 Buyer+AddInfo, 0003 ReviewOrg | not located | identical | R |
| 19 | 24537105 (TED, award) | Kolpingstadt Kerpen | 22970624/26 (2 each) | 0001 Buyer+AddInfo, 0003 ReviewOrg | 2383569 (97) | identical except the buyer's URL | R |
| 20 | 26508153 (DÖE) | IKK classic | as 16 | as 16 | 1206438 | identical | R |
| 21 | 26532531 (DÖE) | GSI Helmholtzzentrum … | as 18 | as 18 | not located | identical | R |
| 22 | 26535235 (DÖE) | Kreis Weseler Abfallgesellschaft Regio mbH | as 17 | as 17 | not located | as 17 | R |
| 23 | 26578966 (DÖE) | Kolpingstadt Kerpen | as 19 | as 19 | 2383569 | as 19 | R |
| 24 | 24657948 (TED, contract notice) | Handwerkskammer Niederbayern-Oberpfalz | 23055099 (6) refused; 22331352 KEEP | 0001 Buyer+AddInfo (KEEP), 0002 ReviewOrg+ReviewInfo | not located | same phone, NUTS, postcode, city and house; e-mail, street spelling, dept and URL differ | R |
| 25 | 24725278 (TED, corrigendum) | as 24 | as 24 | as 24 | not located | as 24 | R |
| 26 | 26605236 (DÖE, contract notice) | as 24 | as 24 | as 24 | not located | as 24 | R |
| 27 | 26617325 (DÖE, corrigendum) | as 24 | as 24 | as 24 | not located | as 24 | R |
| 28 | 26675286 (DÖE, award) | as 24 | as 24 | as 24 | not located | as 24 | R |
| 29 | 44557159 (TED, award) | as 24 | as 24 | as 24 | not located | as 24 | R |

## Items

**15 — 26549982|justizvollzugsanstalt rheinbach — R.** This is the DÖE copy (eforms-de-1.1, can-standard subtype 29, issued
2025-01-15) of the award for procedure `613bc252…`, "Beschaffung eines CNC-gesteuerten Bearbeitungszentrums" (internal ID
2024/4299/F0011). The winner is Maschinen-Kaul GmbH & Co. KG, Düsseldorf-Heerdt. Its TED copy, 24527963, is inc1 item 11 (R);
inc1 fetched this copy only to check that item and did not score it. Three sections carry the PartyName "Justizvollzugsanstalt Rheinbach": ORG-0001 ContractingParty
(`3c804278…`), ORG-0002 AdditionalInformationParty (`1a683d71…`) and ORG-0003 AppealReceiverParty (`cf2c3511…`, the
Lot-ReviewOrg). All three give Aachener Straße 47, 53359 Rheinbach, NUTS DEA2C, vergabe@jva-rheinbach.nrw.de and
+492226-86261, and the field comparison finds no difference. No review chamber is printed anywhere; the only other ORG is the
winner. So the ReviewOrg section is the prison naming itself as appeal receiver, not a second body under its name. GUID rows
22963579, 22963580 and 22963581 each hold 2 mentions (this copy and the TED copy), under the identifiers above. The same
procedure's contract notice (inc1 items 1 and 5) minted four more rows, 22834907-10, so one procurement has spread one prison
over seven GUID rows. The target was not located (see Method). A JVA named with its town is a single institution. Refusing all
three sections sends one body's mentions to that body's 234 row. R.

**16 — 24345339|ikk classic — R.** This is TED 00703483-2024 (2024-11-18), a cn-standard change notice (BT-140
`update-add`, lots 7, 8 and 31 added) for "Arzneimittelrabattverträge gem. § 130a Abs. 8 SGB V im Rahmen von
Open-House-Verfahren" (BT-22 2024_05_01_78, 35 lots, DTVP). ORG-0001 (`f4993229…`) is Procedure-Buyer and Lot-AddInfo on all
35 lots. ORG-0002 (`bb78567a…`) is Lot-ReviewOrg on every lot. Both are "IKK classic", 01099 Dresden, NUTS DED21,
open-house@ikk-classic.de, 0800 455 1111; neither prints a street, and nothing differs. The only other ORG is DÖE as
SProvider. No review chamber is named: an Open-House admission procedure has no Vergabekammer, and the publisher filled the
ReviewOrg slot with the buyer. Rows 22820369 and 22820370 hold 2 mentions each. The target is 1206438, "IKK classic", DE,
identifier NULL, provisional, 2,117 mentions. read_city3 item 29 scored that target R, as one statutory insurer with a Dresden
head office and an Erfurt procurement unit, and already noted that this notice prints the insurer in both roles. R.

**17 — 24441480|kreis weseler abfallgesellschaft regio mbh — R.** This is TED 00781910-2024 (2024-12-18), a can-standard
award notice for "Übernahme und Entsorgung von Problemabfällen aus Haushaltungen und Kleingewerbe aus dem Kreis Wesel"
(VH2024-pp-PA25-26). The winner is Schönmackers Umweltdienste GmbH & Co. KG, Kempen. ORG-0001 (`99941839…`) is
Procedure-Buyer + Lot-AddInfo, and ORG-0002 (`03b219ef…`) is Lot-ReviewOrg. Both give Graftstr. 25, 47475 Kamp-Lintfort,
NUTS DEA1F, contact "Vergabestelle", prior@aez-asdonkshof.de (the Asdonkshof waste centre the company runs) and
+492842940225. The one difference is the fax: `+492842940100` in the buyer section and `+49284940100` in the ReviewOrg
section. That is the same number with a dropped "2", a retyping slip, not another body's number. No review chamber is
printed; the other ORGs are the winner and DÖE. Rows 22895799 and 22895800 hold 2 mentions each. The target was not located.
The name is one specific GmbH. R.

**18 — 24460436|gsi helmholtzzentrum für schwerionenforschung gmbh — R.** This is TED 00772449-2024 (2024-12-16), a
can-standard award notice for "Cryo-coolers (Cryogenic Stopping Cell)" (38/2600018542). The winner is CryoVac GmbH & Co. KG,
Troisdorf. ORG-0001 (`f11e7fc6…`) is Procedure-Buyer + Lot-AddInfo, and ORG-0003 (`fa947898…`) is Lot-ReviewOrg. Both give
Planckstraße 1, 64291 Darmstadt, NUTS DE711, b.neumayer@gsi.de and 0615971357: no difference. The real review chamber is
printed separately under its own name and data: "Vergabekammer des Bundes", Villemomblerstr. 76, 53123 Bonn,
vk@bundeskartellamt.bund.de, +49 22894990. It appears as ORG-0002 Lot-ReviewInfo (`347e6cca…`) and as ORG-0004 Lot-Mediator
(`f828a3a3…`). That is a different name and another group; its target 622 was scored R in read_top2 item 19. This is the
contrast with inc1's AOK W: there the ReviewOrg section carried the VK Bund's phone and NUTS, while here the GSI ReviewOrg
section carries GSI's phone and NUTS, and the VK Bund's values sit only in the VK Bund's own sections. So the buyer printed
itself into the ReviewOrg slot. Rows 22909793 and 22909795 hold 2 mentions each. The target was not located. The name is one
research GmbH. It is not the unrelated "GSI - Gesellschaft für Schweißtechnik International mbH" (target 20316475, read_city1
item 13), which has a different `name_norm`. R.

**19 — 24537105|kolpingstadt kerpen — R.** This is TED 00110272-2025 (2025-02-18), a can-standard award notice for
"Sicherheitsdienst für kommunale Flüchtlingsunterkunft in Kerpen Bruchhöhe" (2024_098, procedure `206a6602…`). It is the same
procedure as inc1 items 3 and 7, whose contract notice of 2024-12-03 has the same BT-04. The winner is AGSUS Agentur für
Sicherheit und Schutz GmbH, Köln. ORG-0001 (`e369fdfd…`) is Procedure-Buyer + Lot-AddInfo, and ORG-0003 (`3b2b31f3…`) is
Lot-ReviewOrg. Both give Jahnplatz 1, 50171 Kerpen, NUTS DEA27, "Abtl. 20.4 - Zentrale Vergabestelle",
vergabestelle@stadt-kerpen.de, +49 2237-580 and fax +49 2237-58102. The only difference is that the buyer section adds the
URL stadt-kerpen.de/ausschreibungen. The review chamber and the mediator are printed under their own names: Vergabekammer
Rheinland (Zeughausstraße 2-10, 50667 Köln, VKRheinland@bezreg-koeln.nrw.de, its own phone) as ORG-0002 Lot-ReviewInfo, and
"Rhein-Erft-Kreis, Der Landrat", Bergheim, as ORG-0004 Lot-Mediator. Rows 22970624 and 22970626 hold 2 mentions each. The
target is 2383569, "Kolpingstadt Kerpen", DE, identifier NULL, provisional, 97 mentions. read_rand1 item 6 and read_rand3
item 30 scored it R as the one NRW town with the title. One procedure has now spread the town over six GUID rows: four in the
contract notice and two here. R.

**20 — 26508153|ikk classic — R.** This is the DÖE copy of item 16 (eforms-de-1.1, ContractFolderID `2ccd5ebb…`, the BT-04
of item 16, issued 2024-11-18, Bekanntmachungs-ID CXP4Y7AH9MZ). It carries the same two values. ContractingParty and
AdditionalInformationParty (35 lots) point to ORG-0001, and AppealReceiverParty (36 references) points to ORG-0002. Both are
"IKK classic", 01099 Dresden, DED21, open-house@ikk-classic.de, 0800 455 1111, with no difference. The copy agrees with
item 16 field for field and is decided alone with the same outcome. Target 1206438. R.

**21 — 26532531|gsi helmholtzzentrum für schwerionenforschung gmbh — R.** This is the DÖE copy of item 18 (ContractFolderID
`3dbb03ce…`, issued 2024-12-16). ORG-0001 (ContractingParty + AdditionalInformationParty) and ORG-0003 (AppealReceiverParty)
carry the same two values. Both are Planckstraße 1, 64291 Darmstadt, DE711, b.neumayer@gsi.de, 0615971357, with no
difference. The Vergabekammer des Bundes is again printed under its own name with its Bonn data, as ORG-0002
(AppealInformationParty) and ORG-0004 (MediationParty). The copy agrees with item 18. R.

**22 — 26535235|kreis weseler abfallgesellschaft regio mbh — R.** This is the DÖE copy of item 17 (ContractFolderID
`499d6659…`, issued 2024-12-18, Bekanntmachungs-ID CXS0YEHYTY8XKAW2). ORG-0001 is ContractingParty +
AdditionalInformationParty, ORG-0002 is AppealReceiverParty, and both carry item 17's values and data at Graftstr. 25,
Kamp-Lintfort. The fax shows the same `+49284940100` slip in ORG-0002, and nothing else differs. The winner Schönmackers is
the only other ORG. R.

**23 — 26578966|kolpingstadt kerpen — R.** This is the DÖE copy of item 19 (ContractFolderID `206a6602…`, issued
2025-02-18). ORG-0001 (ContractingParty + AdditionalInformationParty) and ORG-0003 (AppealReceiverParty) carry item 19's
values and the same Jahnplatz 1 data; again only the buyer section has the URL. Vergabekammer Rheinland is printed as
AppealInformationParty and Rhein-Erft-Kreis as MediationParty, each under its own name. The copy agrees with item 19. Target
2383569. R.

**24 — 24657948|handwerkskammer niederbayern-oberpfalz — R.** This is TED 00179653-2025 (2025-03-18), a cn-standard contract
notice for "Unterhalts- und Grundreinigung" (2025001421, procedure `fa27c6b2…`, 12 lots).
ORG-0001 (`9fd83f8b…`) is Procedure-Buyer and Lot-AddInfo on all 12 lots. Its value is the chamber's own stable platform key:
row 22331352, 116 mentions, 27 Tenders, in KEEP as stable+multi_tender. So it is not refused, and it stays bound to 22331352.
ORG-0002 (`9ac410bf…`, row 23055099) is Lot-ReviewOrg and Lot-ReviewInfo on all 12 lots, and the group refuses it alone.
Both sections give "Handwerkskammer Niederbayern-Oberpfalz", NUTS DE232, 93055 Regensburg, the same switchboard
+49 94179650 and house number 10 on Ditthornstraße. They differ in four places: the buyer section spells "Ditthornstrasse 10",
names the department/contact "Abteilung - IDI/2b", gives a named officer's address (oliver.fidorra@hwkno.de) and the URL
https://hwkno.de; the review section has "Ditthornstr. 10" and info@hwkno.de, with no department or URL. So the review
section is the chamber's general contact, entered separately rather than copied. Every differing value is still the chamber's
own: the hwkno.de domain, the same building, phone and NUTS. No review chamber is printed, and the only other ORG is DÖE.
The value `9ac410bf…` recurs on all three notices of this one procedure (this contract notice, the 2025-04-01 corrigendum and
the 2025-06-10 award), in both copies. That gives 23055099 its 6 mentions under one Tender. It is minted per procedure rather
than per notice, but it is still not an entity key, since the chamber's key is ORG-0001's.
The refusal moves those 6 mentions to the chamber's identifier-NULL row. That target was not located; if it is missing,
Build 2's missing-target choice applies. The mentions never go onto 22331352 (300 §10). The chamber then stays split between
its stable row and its 234 row, which it already is between 22331352 and 23055099.
For the A7 guard this is the R item that an exact-agreement test would wrongly bind (e-mail and street spelling differ). A
test on phone + NUTS + postcode + city refuses it. R.

**25 — 24725278|handwerkskammer niederbayern-oberpfalz — R.** This is TED 00213172-2025 (2025-04-01), the buyer's corrigendum
(BT-140 `cor-buy`, changing notice `2071c863…-01`) to item 24. It has the same two sections, the same values (`9fd83f8b…` KEEP
and `9ac410bf…` refused), the same roles on the 12 lots and the same field comparison: phone, NUTS, postcode, city and house
number agree, and e-mail, street spelling, department and URL differ as in item 24. The only other ORG is DÖE. R.

**26 — 26605236|handwerkskammer niederbayern-oberpfalz — R.** This is the DÖE copy of item 24 (same BT-04 and BT-701,
2025-03-18, published in eForms BT form). ORG-0001 is Buyer + AddInfo (KEEP) and ORG-0002 is ReviewOrg + ReviewInfo
(refused), with the same values and the same field comparison as item 24. R.

**27 — 26617325|handwerkskammer niederbayern-oberpfalz — R.** This is the DÖE copy of the corrigendum, item 25 (BT-140
`cor-buy`, 2025-04-01). It has the same sections, values, roles and field comparison as items 24 and 25. R.

**28 — 26675286|handwerkskammer niederbayern-oberpfalz — R.** This is the DÖE copy, in eForms BT form, of the can-standard
award of 2025-06-10 (BT-701 `b46572d3…`) for the same procedure. The tenderers are Krätschmer Gebäudereinigung +
Dienstleistungs GmbH (Neutraubling) and four Götz-Gebäudemanagement companies at Hofer Straße 10, Regensburg (Nordbayern GmbH &
Co KG, Ostbayern GmbH, Südbayern GmbH, RSL GmbH). Each has its own name and its own section, and none shares the chamber's
name. The two chamber sections are as in item 24: ORG-0001 Buyer + AddInfo on the KEEP value, and ORG-0002 ReviewOrg +
ReviewInfo on `9ac410bf…`, refused, with the same field comparison. R.

**29 — 44557159|handwerkskammer niederbayern-oberpfalz — R.** This is TED 00373984-2025, the TED copy of item 28 (same
BT-701, 2025-06-10). It has the same tenderers, the same two chamber sections and values, and the same field comparison; the
eighth ORG is DÖE as SProvider. This is the sixth and last mention of 23055099. The row is planned whole: 6 of 6 mentions
refused, and its partner value stays in KEEP. R.
