# 530 R2: top targets, items 0-14 (read_top1)

Source: `guid_twin_plan_2659.json` `body.r2_top_targets[0..14]`, the 15 targets that receive the most planned mentions.
Key = target org id. Every item is DE, and its target is an identifier-NULL provisional row whose name equals the planned rows' name.

Question: is the target one body (**R**), unclear (**N**), or a homonym or umbrella that collects different bodies (**W**)? A
target that already holds two bodies scores W even when the folded mentions are right (A6).

**Evidence for each item.**
- `/v1/organizations/{target}`: name, identifier NULL, mentions.
- At least one of the target's newest mentions (`notice_ids` in the report) through `/v1/notices/{id}/content`: ORG name,
  street, postcode, city, email and roles (OPT-300/301, DE1 party references).
- At least one notice of a planned row going to the target. These come from `planned_mention_keys`, which is truncated to
  2,000 keys, so they are 2024-10/11 notices, plus the `r2_random` rows that point at the same target (other readers' bodies, reused).
- The report's city screen: `cities` is the target's set over its newest 50 mentions, and `planned_cities` comes from the planned mentions.
- None of the 15 names is in `r2_incoherent` (A7).

**GETs.** 44 public GETs under tag `top1`: 3 organizations and 41 notice contents. All returned 200, none 408. Each waited at least 2 s
under the shared `.api.lock`/`.api.last` spacing, and all are logged in `api_requests.log`. The 12 other target orgs and 22 of the evidence notices
were already in `bodies/` from the rand readers and were reused, not fetched again.

## Tally

**R 15, N 0, W 0.** Two flagged items (3 GIZ and 9 Aachen) each have 1-2 planned mentions in an off-target city that I could not
locate through the public per-id endpoints. Item 0's second "city" is a postcode.

| # | target | name | mentions → +planned (rows) | flags | verdict |
|---|---|---|---|---|---|
| 0 | 7615 | Vergabekammer Rheinland | 29,617 → +1,346 (673) | multi_city (50667, koln), office_two_words | R |
| 1 | 8100 | Stiftung Preußische Schlösser und Gärten Berlin-Brandenburg | 721 → +716 (358) | none | R |
| 2 | 15234277 | Bundesstadt Bonn - Referat Vergabedienste | 376 → +492 (246) | target city set empty | R |
| 3 | 1165385 | Deutsche Gesellschaft für internationale Zusammenarbeit (GIZ) GmbH | 4,851 → +488 (244) | city_absent 2 (bonn), multi_city_planned | R |
| 4 | 2251109 | Stadt Münster | 192 → +474 (237) | none | R |
| 5 | 1168328 | Zentraldienst der Polizei des Landes Brandenburg | 2,154 → +436 (218) | none | R |
| 6 | 1405749 | HIL Heeresinstandsetzungslogistik GmbH | 2,076 → +430 (215) | none | R |
| 7 | 7195752 | Bau- und Liegenschaftsbetrieb NRW Zentrale | 1,101 → +392 (196) | target city set empty | R |
| 8 | 6136373 | Vergabekammer Westfalen bei der Bezirksregierung Münster | 15,723 → +316 (158) | none | R |
| 9 | 30899495 | Stadt Aachen - Zentrale Vergabestelle FB60/300 | 64 → +302 (151) | city_absent 1 (geilenkirchen), multi_city_planned | R |
| 10 | 11823847 | Vergabekammer Rheinland c/o Bezirksregierung Köln | 6,223 → +296 (148) | none | R |
| 11 | 1168329 | Vergabekammer des Landes Brandenburg | 10,598 → +272 (136) | none | R |
| 12 | 1166261 | Vergabekammer Rheinland-Pfalz beim Ministerium für Wirtschaft, Verkehr, Landwirtschaft und Weinbau | 8,500 → +260 (130) | none | R |
| 13 | 20040451 | Stadt Osnabrück - FD Öffentliche Aufträge | 321 → +260 (130) | none | R |
| 14 | 1166553 | Vergabekammer Niedersachsen beim Nds. Ministerium für Wirtschaft, Arbeit und Verkehr | 13,668 → +256 (128) | none | R |

## Items

**0 — 7615 Vergabekammer Rheinland — R.** The target's newest mentions all print one body.
- Newest mentions: notices 47408004 and 47404257 (Handwerkskammer Aachen, 2026-10-14) print "c/o Bezirksregierung Köln, 50606 Köln,
  VKRheinland@bezreg-koeln.nrw.de" as Lot-ReviewOrg.
- Planned notices: 24263125 (NDR), 24443775 and 24531553 (Bonn) print the twin as ORG-0003 Lot-ReviewInfo plus ORG-0004
  Lot-ReviewOrg. Both sections give Zeughausstraße 2-10, 50667 Köln, with the vergabekammer@ or VKRheinland@ bezreg-koeln address.
  That is one review chamber printed once per role.
- Flags: the b flag (`cities` = ["50667", "koln"]) comes from a mention that put Köln's postcode in BT-513. It is the same city, not
  a second body. The planned city set is {koln}. "Vergabekammer Rheinland" is a specific body: NRW's chamber for the Rheinland
  districts, seated at Bezirksregierung Köln. It is not a generic name, even though it counts as an office two-word key.
- Not a W: the same body also sits under other name_norms (item 10, and "… bei der Bezirksregierung Köln" in the Aachen notices).
  That is a remaining split, not a homonym.

**1 — 8100 Stiftung Preußische Schlösser und Gärten Berlin-Brandenburg — R.** One body throughout.
- Newest mentions: 47414351 and 47411263 (2026-10-16) give SPSG at Allee nach Sanssouci 6, 14471 Potsdam, vergabestelle@spsg.de,
  as buyer and SProvider.
- Planned notices: 24278041, and 24680322 with its DÖE copy 26624305, print the same address and email on 3-4 GUID sections:
  Procedure-Buyer, Lot-AddInfo, Lot-ReviewInfo, Lot-Mediator, SProvider and TenderReceipt.
- Flags: target and planned city sets are both {potsdam}. The name is unique, a foundation of public law.

**2 — 15234277 Bundesstadt Bonn - Referat Vergabedienste — R.** One body.
- Newest mentions: 23528496 and 23526725 (2023, TED forms) give TED-TOWN Bonn, Bertha-von-Suttner-Platz 2-4, 53111,
  referatvergabedienste@bonn.de. The target's `cities` set is empty only because these TED-era mentions carry no BT-513.
- Planned notices: 24264122, 24466739 and 24429562 (2024-25) print that exact address and email on the Procedure-Buyer,
  Lot-AddInfo and Lot-TenderReceipt GUID sections.
- Flags: the planned city set is {bonn}. The name is the procurement unit of the city of Bonn, and only one exists.

**3 — 1165385 Deutsche Gesellschaft für internationale Zusammenarbeit (GIZ) GmbH — R.** One legal entity.
- Newest mentions: 26742076 and 26340016 (DÖE) give Dag-Hammarskjöld-Weg 1-5, 65760 Eschborn.
- Planned notices: 24254237, 24337934 and 24369991 print the same Eschborn address (giz.de contacts) on buyer, AddInfo and
  TenderReceipt GUID sections. name_norm folds the case of "Internationale" vs "internationale".
- Flags: 2 planned mentions give city Bonn, absent from the target's {eschborn}. GIZ GmbH is one company with two registered seats,
  Bonn and Eschborn, so a Bonn address is still GIZ. I did not locate those 2 mentions: the report names no row for them, and the
  public API has no per-name listing in scope.

**4 — 2251109 Stadt Münster — R.** One body.
- Newest mentions: 26723175 and its DÖE copy 26595371, the CIVITAS/CORE joint procurement led by Osnabrück, list Stadt Münster as
  one of 8 co-buyers at Klemensstraße 10, 48143 Münster, vergaben@stadt-muenster.de.
- Planned notices: 24332313 and 24357981 print that address and email on the Procedure-Buyer, Lot-AddInfo, Lot-ReviewInfo and
  Lot-TenderReceipt GUID sections. Contact: Zentrales Vergabemanagement.
- Flags: both city sets are {munster}. Homonym check: no other German municipality is a "Stadt Münster". Münster (Hessen) and
  Münster am Lech are Gemeinden and print as such. The ReviewInfo section is the city's own office, not a separate review body.

**5 — 1168328 Zentraldienst der Polizei des Landes Brandenburg — R.** One body.
- Newest mentions: 45879415 and 44546922 give Am Baruther Tor 20, 15806 Zossen, vergabestelle.zdpol@polizei.brandenburg.de, as
  SProvider, AddInfo and ReviewInfo for other Brandenburg buyers (Landesbetrieb Straßenwesen, MIK).
- Planned notices: 24265601 (Staatskanzlei), 24340456, 24619312 and 24466923 print the same address and email on 3-5 GUID sections:
  SProvider, AddInfo, ReviewInfo and TenderReceipt, plus buyer when it buys for itself.
- Flags: both city sets are {zossen}. Rehoming keeps the central-procurement roles on the body that performs them.

**6 — 1405749 HIL Heeresinstandsetzungslogistik GmbH — R.** One company.
- Newest mentions: 23972531 (2024) gives Josef-Wirmer-Straße 2-8, 53123 Bonn, vergabe@hilgmbh.de. The TED text notice 4220339
  (2010) gives "HIL HEERESINSTANDSETZUNGSLOGISTIK GMBH, BONN". The row spans 14 years of the same buyer.
- Planned notices: 24263636 and 24332689 print the same address on Procedure-Buyer, Lot-AddInfo and Lot-TenderReceipt.
- Flags: both city sets are {bonn}.

**7 — 7195752 Bau- und Liegenschaftsbetrieb NRW Zentrale — R.** One body.
- Newest mentions: the TED text notices 3733750 and 3920878 give "BAU- UND LIEGENSCHAFTSBETRIEB NRW ZENTRALE", DUESSELDORF. The
  `cities` set is empty because text-era mentions carry no BT-513.
- Planned notices: 24329406 and 24334184 print Mercedesstraße 12, 40470 Düsseldorf, BLBVergabe@blb.nrw.de, as Procedure-Buyer and
  Lot-AddInfo. The Zentrale buys for its branches (NL Düsseldorf, NL Münster in the titles).
- Flags: the planned city set is {dusseldorf}. The Niederlassungen carry their own names and therefore their own name_norm targets.
  Item 0 of read_rand1 is NRW Bielefeld, which goes to 1203308.

**8 — 6136373 Vergabekammer Westfalen bei der Bezirksregierung Münster — R.** One review chamber.
- Newest mention: 47408324 (2026-10-15) gives Albrecht-Thaer-Straße 9, 48147 Münster, as Lot-ReviewOrg.
- Planned notices: 24336021 (Stadt Gelsenkirchen) prints Domplatz 1-3, 48143 Münster, the Bezirksregierung's main address, on the
  ReviewInfo and ReviewOrg twin. 24406486 (Stadt Marl) prints Albrecht-Thaer-Str. 9 on ReviewInfo, ReviewOrg and Mediator.
- Flags: both city sets are {munster}. The two street addresses are two addresses of one Bezirksregierung, not two bodies.

**9 — 30899495 Stadt Aachen - Zentrale Vergabestelle FB60/300 — R.** One body; caveat on one mention.
- Newest mentions: 28670451 and 26934904 (2026) give Lagerhausstraße 20, 52064 Aachen, vergabestelle@mail.aachen.de, as buyer,
  AddInfo and ReviewInfo.
- Planned notices: 24275405, 24286099, 24327692 and 24334373 are all four notices this target has in `planned_mention_keys`. Each
  prints that address and email on 4 GUID sections: Buyer, AddInfo, ReviewInfo and TenderReceipt.
- Flags: 1 of the 302 planned mentions gives city Geilenkirchen. I could not locate it. It is not in these four notices, and no
  cached body ties Geilenkirchen to this name. The only Geilenkirchen notice in `bodies/` is a Wasserverband Eifel-Rur procedure
  (24367908/26494828) and does not involve Aachen.
- Why still R: the name, with the department code FB60/300, can denote only Aachen's central procurement office, and the target's
  newest-50 set is {aachen}. At worst that single mention is a mistyped address of the same office.

**10 — 11823847 Vergabekammer Rheinland c/o Bezirksregierung Köln — R.** One body (the same one as item 0, under its long name).
- Newest mentions: 47402058 (Regionalverband Ruhr) and 47401822 (Stadt Jüchen) give Zeughausstraße 2-10, 50667 Köln,
  VKRheinland@bezreg-koeln.nrw.de.
- Planned notices: 24312910 (HHU Düsseldorf) and 24542054 (Polizeipräsidium Aachen) print the same address on 3 GUID sections:
  ReviewInfo, ReviewOrg and Mediator.
- Flags: both city sets are {koln}.

**11 — 1168329 Vergabekammer des Landes Brandenburg — R.** One review chamber.
- Newest mention: 47414536 (Herzberger WAZV, 2026-10-16) gives Heinrich-Mann-Allee 107, 14473 Potsdam, as ReviewInfo plus ReviewOrg.
- Planned notices: 24265601, 24340456, 24619312 and 24466923 print the same address, vergabekammer@mwae.brandenburg.de, on the
  ReviewOrg plus Mediator twin.
- Flags: both city sets are {potsdam}. The longer "… beim Ministerium für Wirtschaft, Arbeit und Energie" spelling (in the SPSG
  notices) is a different name_norm and is not folded here.

**12 — 1166261 Vergabekammer Rheinland-Pfalz beim Ministerium für Wirtschaft, Verkehr, Landwirtschaft und Weinbau — R.** One
review chamber.
- Newest mentions: 47414270 and 47411954 (Stadt Pirmasens, 2026-10-16) give Stiftsstr. 9, 55116 Mainz,
  vergabekammer.rlp@mwvlw.rlp.de.
- Planned notices: 24274974 (ZBL Koblenz) and 24371427 (RPTU) print the same address on the ReviewInfo, ReviewOrg and Mediator twin.
  In 24274974 the Mediator is a differently named body (ADD Trier) and is not part of the group.
- Flags: both city sets are {mainz}.

**13 — 20040451 Stadt Osnabrück - FD Öffentliche Aufträge — R.** One body.
- Newest mentions: 46376391 (2026-09) and 26478863 (DÖE) give Bierstraße 2, 2. OG, 49074 Osnabrück,
  oeffentlicheauftraege@osnabrueck.de.
- Planned notices: 24256626 and 24337726 (2024) give Luisenstrasse 18, 2. OG, 49074 Osnabrück, with the same email, on Buyer,
  AddInfo and TenderReceipt. The office moved; the body and its email stayed the same.
- Flags: both city sets are {osnabruck}.

**14 — 1166553 Vergabekammer Niedersachsen beim Nds. Ministerium für Wirtschaft, Arbeit und Verkehr — R.** One review chamber.
- Newest mention: 47414336 (NOW IT, 2026-10-16) gives Auf der Hude 2, 21339 Lüneburg, as Lot-ReviewOrg.
- Planned notices: 24254908 and 24530701 (Landkreis Northeim) print the same address on the ReviewInfo, ReviewOrg and Mediator twin.
- Flags: both city sets are {luneburg}.

## Notes for the R2 tally

- 0 W among top targets 0-14, so nothing here goes into `GUID_TWIN_UNSAFE_NAMES`.
- Item 0 is the largest target (+1,346) and an office two-word key. Its b flag is a postcode in the city field (50667 = Köln), not a
  second city. A city screen that folded postcode-only BT-513 values would drop this false flag.
- Items 3 and 9 each leave 1-2 off-city planned mentions unread (GIZ Bonn, a registered seat; Aachen/Geilenkirchen, unexplained). A
  reader with the planned row list per target (which the report does not carry) could close them. Neither changes the verdict: the
  names denote one body each.
- Most of the 15 targets are review chambers or procurement offices whose twin is ReviewInfo + ReviewOrg (+ Mediator), or Buyer +
  AddInfo + TenderReceipt. In both shapes one body's sections are printed once per role, which is the case 530-T is built for.
