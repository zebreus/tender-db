# 530 R2: random planned GUID rows, items 30-44 (read_rand3)

Scope: `body.r2_random[30..44]` (0-based, so the 31st to the 45th row) of `guid_twin_plan_2659.json`. All 15 rows are
in the review_or_info stratum (items 25-49). Every row is planned whole, `twin:true`, `twin_after:false`, 2 mentions
(the TED and DÖE copies of one notice), 1 Tender, not in KEEP and not stable. Every row has an identifier-NULL target,
so none of them depends on the Build 2 mint-or-keep choice.

Question for each row: if all its mentions move to the target (the identifier-NULL row for (name_norm, DE)), is that
**R** (the target is the same body and the GUID was a per-section mint), **N** (cannot tell) or **W** (the target name
covers more than one body, the target would take another body's mentions, or the GUID is a real stable key)?

Method:
- For each row: GET the GUID row, GET the target, and GET the TED copy's `/v1/notices/{id}/content` (the first id in
  `notice_ids`). The notice gives every ORG section's name, BT-501, department, address, e-mail and its OPT-300/301
  role. For every row, the GUID row's identifier matches its section's BT-501 exactly, after case folding and with the
  dashes removed.
- Targets with an empty city set in the plan (15234277 Bonn, 14256311 BLB): I read the target's newest notice from
  the plan (23528496, 23509393). Both are 2023 TED F14 forms. They carry the town in `TED-TOWN`, not BT-513, which is
  why the city screen saw no city.
- Target 7615: I also read its newest notice from the plan (47408004).
- Target city sets and flags come from the plan. Only one of these 14 distinct targets is on `city.flagged`: 7615
  (items 42, 43). The other 13 passed both screens.
- Reused bodies, not fetched again: `organizations_2383569` (read_rand1), `organizations_15234277` and
  `notices_23528496` (read_rand1/2).
- 44 GETs, all 200, no 408s. They are logged as `reader=rand3` in `r2/api_requests.log`. Each GET took the shared
  `.api.lock` and waited until 2.1 s after `.api.last`, so it never overlapped another reader's GET. Bodies are in
  `r2/bodies/`.

**Tally: 15 R, 0 N, 0 W.**
- By the role of the row's own section: Lot-AddInfo 5, Lot-ReviewOrg 5, Lot-ReviewInfo 2, Lot-TenderReceipt 2,
  Lot-Mediator 1. Each one is twinned with sections of the same body.
- None of the 15 groups mixes a buyer-family role with Lot-ReviewOrg (A7).
- Wherever a review chamber appears, it is a different name, so a different group.

## Items

**30: 22959819 "Kolpingstadt Kerpen" (review_or_info), target 2383569: R.**
TED 00009531-2025 (notice 24522973) is a contract award notice for a turntable ladder for the Kerpen fire service.
The row is ORG-0002 Lot-AddInfo (`d217b47d…`). It is twinned with ORG-0001 Procedure-Buyer (`5cfc5bae…`). Both give
Abtl. 20.4 Zentrale Vergabestelle, Jahnplatz 1, 50171 Kerpen, vergabestelle@stadt-kerpen.de. The review body
(Vergabekammer Rheinland, Köln), the mediator (Rhein-Erft-Kreis, Bergheim) and the winner (Rosenbauer) each have their
own name. The target 2383569 has the same name, DE, identifier NULL, provisional and 97 mentions, and its city screen
is clean. "Kolpingstadt" is the title of the NRW town only. Kerpen (Eifel) is an Ortsgemeinde with another name, so
"Kolpingstadt Kerpen" names one body.

**31: 22879585 "Verwaltungs-Berufsgenossenschaft (VBG) - Hauptverwaltung Hamburg" (review_or_info), target 10749091: R.**
TED 00793853-2024 (notice 24421187) is a contract notice for a software-development framework. The row is ORG-0002
Lot-AddInfo (`35cbe58f…`). It is twinned with ORG-0001 Procedure-Buyer (`d35adc25…`). Both give Beschaffung und
Nachhaltigkeit, Massaquoipassage 1, 22305 Hamburg, vergabestelle@vbg.de. The "Vergabekammern des Bundes" sections
(three UUIDs) and the DTVP TenderReceipt section have other names. The target 10749091 has identifier NULL and 88
mentions, and its city screen is clean. The name pins the VBG's head office, not its regional Bezirksverwaltungen, so
the target is one body.

**32: 22965730 "Vergabekammer Niedersachsen beim Nds. Ministerium für Wirtschaft, Arbeit und Verkehr" (review_or_info), target 1166553: R.**
TED 00033345-2025 (notice 24530701) is a contract award notice for a diesel framework for Landkreis Northeim. The
chamber is printed three times under three UUIDs: ORG-0002 ReviewInfo, ORG-0003 Lot-ReviewOrg (`69c6792a…`, this row)
and ORG-0004 Mediator. All three give Auf der Hude 2, 21339 Lüneburg and the same rv-lg.niedersachsen.de contact. The
buyer (Landkreis Northeim, one section with both Buyer and AddInfo roles) is a different name. The target 1166553 has
identifier NULL and 13,668 mentions. Its city set is {luneburg} and the planned rows' cities are {luneburg}; it is in
the top 30 (128 planned rows). The name points to the ministry in Hannover, but the one Lower Saxony chamber sits in
Lüneburg, and every mention agrees. One body.

**33: 22868220 "Vergabekammer Westfalen bei der Bezirksregierung Münster" (review_or_info), target 6136373: R.**
TED 00754780-2024 (notice 24406486) is a contract award notice for electrical works at the Goetheschule Marl. The
chamber is printed three times: ORG-0002 ReviewInfo (`4cf7bd09…`, this row, dept "Rechtsbehelfstelle"), ORG-0003
ReviewOrg and ORG-0004 Mediator. All three give Albrecht-Thaer-Str. 9, 48147 Münster, vergabekammer@bezreg-muenster.de.
The buyer, Stadt Marl's Zentrale Vergabestelle, is a different name. The target 6136373 has identifier NULL and
15,723 mentions. Its city set is {munster} and the planned cities are {munster}; it is in the top 30 (158 planned
rows). It is the one Westphalian chamber.

**34: 22886295 "Bundesstadt Bonn - Referat Vergabedienste" (review_or_info), target 15234277: R.**
TED 00757561-2024 (notice 24429562) is a contract award notice for joinery at the Friedrich-List-Berufskolleg. The row
is ORG-0002 Lot-AddInfo (`b8323eee…`). It is twinned with ORG-0001 Procedure-Buyer (`05a74547…`). Both give
Bertha-von-Suttner-Platz 2-4, 53111 Bonn, referatvergabedienste@bonn.de. The Vergabekammer Rheinland pair
(ReviewInfo and ReviewOrg) forms its own group. The target 15234277 has identifier NULL and 376 mentions, and it is in
the top 30 (246 planned rows, planned cities {bonn}). The plan's target city set is empty, so I read its newest notice,
23528496 (a 2023 TED F14). Its ORG-1 is the same Referat Vergabedienste at the same address, with the same e-mail and
Bonn. One city procurement office.

**35: 22858436 "Vergabekammer bei der Finanzbehörde der Freien und Hansestadt Hamburg" (review_or_info), target 1653832: R.**
TED 00682838-2024 (notice 24394386) is a contract notice from Hamburg Messe und Congress GmbH for outdoor
advertising. The chamber is printed twice: ORG-0003 Lot-ReviewOrg (`94d9c5fe…`) and ORG-0004 Lot-Mediator
(`9d322b9e…`, this row). Both give Gänsemarkt 36, 20354 Hamburg, vergabekammer@fb.hamburg.de. The buyer prints itself
three times (Buyer, AddInfo, TenderReceipt) under its own name, which is a separate group. The target 1653832 has
identifier NULL and 263 mentions, and its city screen is clean. Hamburg's other chamber (at the building authority)
has another name, and so another key. The name contains "Freien und Hansestadt Hamburg", but it is not the bare polity
umbrella of A8. It names the one chamber at the Finanzbehörde.

**36: 22770771 "Brandenburgischer Landesbetrieb für Liegenschaften und Bauen (BLB) - Zentrale Vergabestelle" (review_or_info), target 14256311: R.**
TED 00650943-2024 (notice 24277826) is a contract award notice for site clearance for the Bundespolizei in Blumberg.
The row is ORG-0002 Lot-AddInfo (`b64d10bb…`). It is twinned with ORG-0001 Procedure-Buyer (`7752e9a2…`). Both give
"Zentrale Vergabestelle, Bereich BLD", An der Wache 2, 15806 Zossen, Info.Vergabe-BLD@BLB.Brandenburg.de. Two other
sections have different names and stay outside the group: "BLB - Vergabeprüfstelle" (Potsdam, ReviewInfo) and
Vergabekammer des Bundes (ReviewOrg). The target 14256311 has identifier NULL and 371 mentions, and it is in the top
30 (114 planned rows, planned cities {zossen}). Its city set is empty, so I read its newest notice, 23509393 (2023
TED F14, a BAM Horstwalde build). Its ORG-1 is the same office: same contact point "Zentrale Vergabestelle, Bereich
BLD", An der Wache 2, Zossen, same e-mail. The target is one office of one Landesbetrieb.

**37: 23001169 "Vergabekammer des Landes Hessen bei dem Regierungspräsidium Darmstadt" (review_or_info), target 1175826: R.**
TED 00083621-2025 (notice 24578315) is a contract award notice from Landkreis Gießen for a school foundation. The
chamber is printed three times: ORG-0003 ReviewInfo, ORG-0004 Lot-ReviewOrg (`23bb7831…`, this row) and ORG-0005
Mediator. All three give Wilhelminenstraße 1-3, 64278 Darmstadt, vergabekammer@rpda.hessen.de. The buyer's pair
(Buyer and AddInfo) is a different name. The target 1175826 has identifier NULL and 6,648 mentions. Its city set is
{darmstadt} and the planned cities are {darmstadt}; it is in the top 30. It is the one Hessian chamber.

**38: 22840798 "Vergabekammer Rheinland-Pfalz beim Ministerium für Wirtschaft, Verkehr, Landwirtschaft und Weinbau" (review_or_info), target 1166261: R.**
TED 00692184-2024 (notice 24371427) is a contract notice from RPTU Kaiserslautern-Landau for building cleaning. The
chamber is printed three times: ORG-0003 ReviewInfo, ORG-0004 Lot-ReviewOrg (`83729810…`, this row) and ORG-0005
Mediator. All three give Geschäftsstelle, Stiftsstraße 9, 55116 Mainz, vergabekammer.rlp@mwvlw.rlp.de. RPTU prints
itself three times (Buyer, AddInfo, TenderReceipt), which is a separate group. The target 1166261 has identifier NULL
and 8,500 mentions. Its city set is {mainz} and the planned cities are {mainz}; it is in the top 30. One chamber.

**39: 22808978 "Universitätsklinikum Köln AöR" (review_or_info), target 1281557: R.**
TED 00718534-2024 (notice 24331701) is a contract notice for PCR equipment. The hospital is printed three times:
ORG-0001 Procedure-Buyer, ORG-0002 Lot-AddInfo and ORG-0004 Lot-TenderReceipt (`ed56638f…`, this row). All three give
Zentrale Beschaffung/Vergabestelle, Kerpener Straße 62, 50937 Köln, vergabestelle@uk-koeln.de. The ReviewOrg is
"Vergabekammer Rheinland (Spruchkörper Köln) bei der Bezirksregierung Köln", a different name. The target 1281557 has
identifier NULL and 655 mentions, and its city screen is clean. It is one Anstalt des öffentlichen Rechts.

**40: 22830288 "Stadt Münster" (review_or_info), target 2251109: R.**
TED 00724238-2024 (notice 24357981) is a contract notice for ventilation at the Albachten primary school. The city is
printed four times under four UUIDs: ORG-0001 Procedure-Buyer, ORG-0002 AddInfo, ORG-0003 Lot-ReviewInfo and ORG-0005
Lot-TenderReceipt (`d8b0418d…`, this row). All four give Zentrales Vergabemanagement, Klemensstraße 10, 48143 Münster,
vergaben@stadt-muenster.de. The ReviewOrg is Vergabekammer Westfalen, a different name. The buyer printing itself as
ReviewInfo is the usual entry for questions about review deadlines; it is not a Lot-ReviewOrg mix. The target 2251109
has identifier NULL and 192 mentions, and it is in the top 30 (237 planned rows). Its city set is {munster} and the
planned cities are {munster}. "Stadt Münster" is the NRW city only: the Hessian and Bavarian Münsters are Gemeinden
with other names. One body.

**41: 22795894 "Universitätsmedizin Greifswald KöR" (review_or_info), target 1476299: R.**
TED 00660733-2024 (notice 24313135) is a contract award notice for surgical instruments (10 lots). The row is ORG-0002
Lot-AddInfo (`676c7db5…`). It is twinned with ORG-0001 Procedure-Buyer (`9eb525b7…`). Both give Fleischmannstraße 8,
17489 Greifswald, dirk.evers@med.uni-greifswald.de. The ReviewOrg is the MV chamber office in Schwerin, and the two
winners (B. Braun, Geister) have their own names. The target 1476299 is spelled "…KÖR". It is the same key after case
folding: identifier NULL, 149 mentions, city screen clean. One body.

**42: 22897538 "Vergabekammer Rheinland" (review_or_info), target 7615: R.**
TED 00741236-2024 (notice 24443775) is a contract notice from Bundesstadt Bonn (WCCB communications system). The
chamber is printed twice: ORG-0003 Lot-ReviewInfo (`17ed73ab…`, this row) and ORG-0004 Lot-ReviewOrg (`8df7d451…`).
Both give Zeughausstraße 2-10, 50667 Köln, vergabekammer@bezreg-koeln.nrw.de. Bonn's Referat Vergabedienste prints
itself three times (Buyer, AddInfo, TenderReceipt), which is a separate group. The target 7615 has identifier NULL and
29,617 mentions; it is the largest target (+1,346 from 673 rows). It is on the city, top and office-two-word lists.
Its city flag is (b): the set is {50667, koln}. 50667 is the Zeughausstraße postcode printed in the town field, so it
is one city. The planned cities are {koln}. Its newest notice, 47408004 (a 2026-10-14 contract award notice for
Handwerkskammer Aachen), prints "Vergabekammer Rheinland, c/o Bezirksregierung Köln, 50606 Köln" with the same
VKRheinland@ address. Vergabekammer Rheinland is one review authority at the Bezirksregierung Köln; its Köln and
Düsseldorf Spruchkörper are panels of it, not separate bodies. Notice 24331701 prints the panel-specific name as its own
name_norm, which stays a separate key: the fold can only under-merge there. This row is R. The target as a whole is
for the top-target and flagged readers, but nothing here shows a second body under the name.

**43: 22966357 "Vergabekammer Rheinland" (review_or_info), target 7615: R.**
TED 00033865-2025 (notice 24531553) is a contract award notice from Bundesstadt Bonn for interior painting at "Bonns
Fünfte". It has the same shape as item 42: ORG-0003 Lot-ReviewInfo (`0b9ad13d…`) and ORG-0004 Lot-ReviewOrg
(`16bd2059…`, this row). Both give Zeughausstraße 2-10, 50667 Köln, vergabekammer@bezreg-koeln.nrw.de. Here the Bonn
buyer prints two sections (Buyer, AddInfo), and the winner is Malerwerkstätte Wallbrück. The target is 7615, as in
item 42. Same body.

**44: 22914741 "Vergabekammer des Landes Brandenburg" (review_or_info), target 1168329: R.**
TED 00003422-2025 (notice 24466923) is a contract award notice for data-carrier destruction for the Brandenburg
police. The chamber is printed twice: ORG-0005 Lot-ReviewOrg (`d6379d4c…`, this row) and ORG-0006 Mediator
(`caa86447…`). Both give Heinrich-Mann-Allee 107, 14473 Potsdam, vergabekammer@mwae.brandenburg.de. The target 1168329
has identifier NULL and 10,598 mentions. Its city set is {potsdam} and the planned cities are {potsdam}; it is in the
top 30 (136 planned rows). It is the one Brandenburg chamber.

Side note for the incoherent and other readers: in this notice the buyer, "Zentraldienst der Polizei des Landes
Brandenburg" (Am Baruther Tor 20, Zossen), is printed four times. Its sections are ORG-0001 Procedure-Buyer, ORG-0002
Procedure-SProvider, ORG-0003 AddInfo and ORG-0004 ReviewInfo, all at the same address and e-mail. It is one body that
names itself as its own service provider. The role set has no Lot-ReviewOrg, so A7 does not list it, and it is not a
mix.
