# 530 R2b: fresh random planned GUID rows, items 15-29 (read_rand2)

Scope: `body.r2_random[15..29]` (0-based) of `guid_twin_plan_2660.json` (Build 1b, deployed 8e9222c; salt 530003, the 60
rows R2 read at 530002 excluded). That is 10 rows from the buyer stratum (15-24) and 5 from the review_or_info stratum
(25-29). Every row is planned `whole`, `twin:true`, 2 mentions (the TED copy and the DÖE copy of one notice), 1 Tender,
`in_keep:false`, `stable:false`, `twin_after:false`, country DE, and every row has a found identifier-NULL target (no
missing key in this slice).

Question for each row: if all its mentions move to the target (the identifier-NULL row for (name_norm, DE)), is that
**R** (the target is the same body; the GUID was a per-section mint), **N** (cannot tell) or **W** (the target name
covers more than one body, the target would take another body's mentions, a different body is printed under this
name in this notice's role, or the GUID is a real stable key)?

Method:
- For each row I fetched the GUID row, the target and **both** notice copies (TED `/content` and DÖE `/content`). From the
  copies I took each ORG section's name, BT-501 / DE1 CompanyID, street, postcode, city, e-mail, phone and its
  OPT-300/301 (DE1 party reference) roles. In all 15 pairs the DÖE copy carries the same UUID on the same ORG section,
  with the same role and address as the TED copy. So both mentions of each row are the one section described below.
- Target side: `/v1/organizations/{id}` (identifier NULL, provisional, mention count). Then the plan's city screen
  (A6). `city.flagged` is complete (36 = `flagged_count`), and only item 16's target is on it. For every other target, no
  planned city is missing from the target's non-empty newest-50 city set, the set has fewer than 2 cities, and the
  key's planned mentions show one city. Where `planned_mention_keys` lists another planned notice of the same key, I read
  one (6 sibling notices) to see what else the target would receive. For targets that R2 already read (top-30 or
  random), I cite that read.
- A7: for each group I list the role set from the copy's own references. None of the 15 groups mixes a buyer-side role
  (Procedure-Buyer, Lot-AddInfo, Lot-TenderReceipt, Procedure-SProvider) with Lot-ReviewOrg. None has a tenderer.
  `roles.incoherent` and `r2_incoherent` are empty in 2660.
- F1: none of the 15 names is on the Build-1b unsafe list. Item 25 is the long MV key that R2's F1 said to leave
  alone.
- GETs: 66 public GETs, all 200, no 408 and no retry. Logged as `reader=rand2b` in `r2b/api_requests.log` (20:44:00Z to
  20:49:12Z). They are serialized with the other readers by `flock` on `r2b/.api.lock` and `.api.last` at ≥ 2.2 s; the
  smallest gap between any two lines in the shared log is 2.05 s. Bodies are in `r2b/bodies/`. I also read two cached R2
  bodies (`r2/bodies/notices_24274974`, `notices_24313135`) with 0 GETs. No `/v1/sql`, ssh, POST, git or cargo.

**Tally: 15 R, 0 N, 0 W.**

| # | row | name | role | target (mentions) | verdict |
|---|---|---|---|---|---|
| 15 | 22772664 | Universität Stuttgart | buyer | 1262231 (420) | R |
| 16 | 23001478 | Stadt Aachen - Zentrale Vergabestelle FB60/300 | buyer | 30899495 (64) | R |
| 17 | 22834338 | Ministerium für Umwelt, Naturschutz und Verkehr des Landes NRW | buyer | 19560123 (13) | R |
| 18 | 22859790 | IHP GmbH - Leibniz-Institut für innovative Mikroelektronik | buyer | 1648483 (149) | R |
| 19 | 22819228 | HIL Heeresinstandsetzungslogistik GmbH | buyer | 1405749 (2,076) | R |
| 20 | 22852201 | Oberfinanzdirektion Karlsruhe - LZfD | buyer | 14436538 (186) | R |
| 21 | 22846983 | Stadt Marl - Amt für Klimaschutz und Nachhaltigkeit - Zentrale Vergabestelle | buyer | 14467750 (159) | R |
| 22 | 22860702 | Polizeipräsidium Düsseldorf | buyer | 2463330 (30) | R |
| 23 | 22771735 | Landratsamt Ostalbkreis - Zentrale Vergabestelle | buyer | 17573857 (84) | R |
| 24 | 22850671 | Stadt Gütersloh | buyer | 2439295 (1,539) | R |
| 25 | 22834379 | Geschäftsstelle der Vergabekammern beim Ministerium für Wirtschaft, Infrastruktur, Tourismus und Arbeit Mecklenburg-Vorpommern | review_or_info | 23021560 (3) | R |
| 26 | 22957397 | Aufsichts- und Dienstleistungsdirektion | review_or_info | 1176684 (652) | R |
| 27 | 22975387 | Vergabekammer Rheinland · c/o Bezirksregierung Köln | review_or_info | 12774348 (495) | R |
| 28 | 22965731 | Vergabekammer Niedersachsen beim Nds. Ministerium für Wirtschaft, Arbeit und Verkehr | review_or_info | 1166553 (13,668) | R |
| 29 | 22868221 | Vergabekammer Westfalen bei der Bezirksregierung Münster | review_or_info | 6136373 (15,723) | R |

## Items

**15: 22772664 "Universität Stuttgart" (buyer), target 1262231: R.** TED 00649769-2024 (notice 24280399; DÖE 26488038)
is a 2024-10-24 award notice for a high-power-laser deflection system. The row is ORG-0001 Procedure-Buyer (`43a46e2b…`,
Keplerstraße 7, 70174 Stuttgart). Its twin is ORG-0002 Lot-AddInfo (`dd088ccb…`, contact "Zentrale Beschaffungsstelle",
Breitscheidstraße 2 C, 70174 Stuttgart). Both give beschaffung@verwaltung.uni-stuttgart.de and +49 7116850; the two
streets are the university's administration and its procurement office. Outside the group: ORG-0003 ReviewInfo
"Universität Stuttgart, Zentrale Beschaffungsstelle" (another name_norm), ORG-0004 ReviewOrg (VK BW, Karlsruhe) and
ORG-0005 Tenderer (SCANLAB). The role set is {Buyer, AddInfo}, so A7 is clean. Target 1262231 is identifier NULL with 420
mentions, top target 15 (+254 from 127 rows), city set {stuttgart} matching the planned city. R2 read it R (read_rand1
item 10, read_top2 item 15). One university.

**16: 23001478 "Stadt Aachen - Zentrale Vergabestelle FB60/300" (buyer), target 30899495: R.** TED 00081780-2025 (notice
24578735; DÖE 26567917) is a 2025-02-05 contract notice for drywall works at the Rhein-Maas-Gymnasium. The name, at
Lagerhausstraße 20, 52064 Aachen, vergabestelle@mail.aachen.de, is printed four times under four UUIDs: ORG-0001
Procedure-Buyer (`410d9661…`, this row), ORG-0002 AddInfo, ORG-0003 ReviewInfo and ORG-0005 TenderReceipt. ORG-0004
ReviewOrg is "Vergabekammer Rheinland bei der Bezirksregierung Köln", which has another name. The role set is
{Buyer, AddInfo, ReviewInfo, TenderReceipt}, with no ReviewOrg, so A7 is clean. Target 30899495 is identifier NULL with 64
mentions; its newest notices are 28670451, 28430755 and 26934904, and its city set is {aachen}. It is city-flagged
(a: 1 planned mention says Geilenkirchen; c: planned cities {aachen, geilenkirchen}) and is top target 9. R2 read it R
(read_top1 item 9, city1 item 2) and could not locate that one Geilenkirchen mention. This row prints Aachen in both
copies, so it is not that mention. The department code FB60/300 can only be Aachen's central procurement office. One
body.

**17: 22834338 "Ministerium für Umwelt, Naturschutz und Verkehr des Landes NRW" (buyer), target 19560123: R.** TED
00675398-2024 (notice 24363047; DÖE 26497080) is a 2024-11-04 contract notice for an events framework. The name, at
Emilie-Preyer-Platz 1, 40479 Düsseldorf, vergabestelle@munv.nrw.de, contact "Vergabestelle", is printed four times:
ORG-0001 Procedure-Buyer (`496c6302…`, this row), ORG-0002 AddInfo, ORG-0003 ReviewInfo and ORG-0005 TenderReceipt.
ORG-0004 ReviewOrg is "Vergabekammer Rheinland c/o Bezirksregierung Köln", which has another name. The role set has no
ReviewOrg, so A7 is clean; the buyer printing itself as ReviewInfo is the usual "ask us about deadlines" entry. Target
19560123 is identifier NULL with 13 mentions and is not city-flagged. No sibling planned notice is in the truncated key
list. The name is the full title of one NRW state ministry (MUNV), so a homonym is not possible. One body.

**18: 22859790 "IHP GmbH - Leibniz-Institut für innovative Mikroelektronik" (buyer), target 1648483: R.** TED
00683230-2024 (notice 24396035; DÖE 26499552) is a 2024-11-07 contract notice for clean-room clothing. The name, at Im
Technologiepark 25, 15236 Frankfurt (Oder), rohner@ihp-microelectronics.com, contact "Beschaffung", is printed four
times: ORG-0001 Procedure-Buyer (`c217081c…`, this row), ORG-0002 Procedure-SProvider (`21e2ce74…`, row 22859791, which
R2 read R as read_rand4 item 51), ORG-0003 AddInfo and ORG-0007 TenderReceipt. The Brandenburg chamber (ReviewInfo,
ReviewOrg and Mediator, Potsdam) has another name. The role set {Buyer, SProvider, AddInfo, TenderReceipt} is entirely
buyer-side, with no ReviewOrg, so A7 is clean. Target 1648483 "IHP GmbH - Leibniz-Institut für Innovative
Mikroelektronik" differs only in case and folds to the same name_norm. It is identifier NULL with 149 mentions and is not
flagged. The sibling planned notices 24265055 and 24339288 belong to the same institute (R2 read 24265055). One GmbH.

**19: 22819228 "HIL Heeresinstandsetzungslogistik GmbH" (buyer), target 1405749: R.** TED 00705752-2024 (notice
24343957; DÖE 26508432) is a 2024-11-18 contract notice for the depot repair of the Biber bridge-layer girders. The name,
at Josef-Wirmer-Straße 2-8, 53123 Bonn, vergabe@hilgmbh.de, is printed three times: ORG-0001 Procedure-Buyer
(`9f2714a0…`, this row), ORG-0002 AddInfo and ORG-0004 TenderReceipt. ORG-0003 ReviewOrg is "Bundeskartellamt
Vergabekammer des Bundes", which has another name. A7 is clean. Target 1405749 is identifier NULL with 2,076 mentions,
top target 6 (+430 from 215 rows), city sets {bonn}/{bonn}. R2 read it R (read_top1 item 6): its mentions run from the
2010 text era to 2024 and are the same company at the same address. One company.

**20: 22852201 "Oberfinanzdirektion Karlsruhe - LZfD" (buyer), target 14436538: R.** TED 00692632-2024 (notice
24385854; DÖE 26504494) is a 2024-11-13 contract notice for Tufin firewall automation. The name, at Moltkestraße 80,
76133 Karlsruhe, contact "Vergabestelle LZfD", Helene.Oberlaender@ofdka.bwl.de, is printed four times: ORG-0001
Procedure-Buyer (`d7c8f1d6…`, this row), ORG-0002 AddInfo, ORG-0003 ReviewInfo and ORG-0005 TenderReceipt. ORG-0004
ReviewOrg is "Regierungspräsidium Karlsruhe" (the VK BW, Durlacher Allee 100), which has another name. The role set
contains ReviewInfo but not ReviewOrg, so A7 is clean. Target 14436538 is identifier NULL with 186 mentions and is not
flagged. The sibling planned notice 24255887 (2024-10-18, a security-training framework) prints the same name and
address with another contact (Birgit.Koehler@ofdka.bwl.de) on Buyer, AddInfo and TenderReceipt. The LZfD is the
Landeszentrum für Datenverarbeitung inside OFD Karlsruhe: one office of one authority.

**21: 22846983 "Stadt Marl - Amt für Klimaschutz und Nachhaltigkeit - Zentrale Vergabestelle" (buyer), target 14467750:
R.** TED 00684165-2024 (notice 24378890; DÖE 26500702) is a 2024-11-08 contract notice for school transport. The name, at
Carl-Duisberg-Straße 165, Stadthaus 1, 45772 Marl, submission@marl.de, is printed three times: ORG-0001 Procedure-Buyer
(`13444284…`, this row), ORG-0002 AddInfo and ORG-0006 TenderReceipt. The VK Westfalen sections (ReviewInfo, ReviewOrg
and Mediator, Münster) form their own group. A7 is clean. Target 14467750 is identifier NULL with 159 mentions and is not
flagged. The sibling planned notice 24341478 (Goethe-Grundschule pool floor) shows the same office on Buyer, AddInfo and
TenderReceipt. R2's 24406486 (read_rand3 item 33) shows the same office as Buyer and AddInfo. One municipal procurement
office.

**22: 22860702 "Polizeipräsidium Düsseldorf" (buyer), target 2463330: R.** TED 00751035-2024 (notice 24397311; DÖE
26524064) is a 2024-12-05 contract notice for towing services. The name, contact "ZA 13 - Vergabe und Beschaffung",
Jürgensplatz 5-7, 40219 Düsseldorf, ZVST.duesseldorf@polizei.nrw.de, is printed three times: ORG-0001 Procedure-Buyer
(`50bf72f1…`, this row), ORG-0002 AddInfo and ORG-0006 TenderReceipt. The VK Rheinland sections (ReviewInfo, ReviewOrg and
Mediator) form their own group. A7 is clean. Target 2463330 is identifier NULL with 30 mentions and is not flagged. The
sibling planned notice 24332740 is the earlier (2024-11-21) towing notice of the same procedure, with the same three
sections and address. There is one police headquarters of that name. One body.

**23: 22771735 "Landratsamt Ostalbkreis - Zentrale Vergabestelle" (buyer), target 17573857: R.** TED 00651449-2024
(notice 24279165; DÖE 26488059) is a 2024-10-24 contract notice for façade metalwork at the Kreisberufsschulzentrum
Aalen. The name, at Stuttgarter Straße 41, 73430 Aalen, zentralevergabestelle@ostalbkreis.de, is printed three times:
ORG-0001 Procedure-Buyer (`9134cf9a…`, this row), ORG-0002 AddInfo and ORG-0004 TenderReceipt. ORG-0003 ReviewOrg is
"Vergabekammer Baden-Württemberg im RP Karlsruhe", which has another name. A7 is clean. Target 17573857 is identifier NULL
with 84 mentions and is not flagged. The sibling planned notice 24311385 (the same school's demolition lot) shows the same
three sections and address. Ostalbkreis is one Landkreis. One office.

**24: 22850671 "Stadt Gütersloh" (buyer), target 2439295: R.** TED 00700785-2024 (notice 24383886; DÖE 26507028) is a
2024-11-15 award notice for the façade of the Städtisches Gymnasium. The name, contact "Zentrale Vergabestelle", Berliner
Straße 70, 33330 Gütersloh, ZentraleVergabestelle@guetersloh.de, is printed twice: ORG-0001 Procedure-Buyer (`1add2c0a…`,
this row) and ORG-0002 AddInfo. "Vergabekammer Westfalen" (ReviewInfo and ReviewOrg) and the tenderer have other names.
A7 is clean. Target 2439295 is identifier NULL with 1,539 mentions and is not flagged. The sibling planned notice 24334190
(an iPad/display award) shows the same Buyer+AddInfo pair at the same address. There is one Gütersloh in DE, and the
large count is a city that buys a lot under one name. One municipality.

**25: 22834379 "Geschäftsstelle der Vergabekammern beim Ministerium für Wirtschaft, Infrastruktur, Tourismus und Arbeit
Mecklenburg-Vorpommern" (review_or_info), target 23021560: R.** TED 00675938-2024 (notice 24363093; DÖE 26496969) is a
2024-11-05 contract notice of Universitätsmedizin Greifswald for cleaning and disinfection units. The MV chambers' office,
at Johannes-Stelling-Str. 14, 19053 Schwerin, vergabekammer@wm.mv-regierung.de, +49 385588-15164, is printed three times:
ORG-0003 ReviewInfo, ORG-0004 ReviewOrg (`28e1f9fc…`, row 22834378, which R2 read R as read_rand2 item 26) and ORG-0005
Mediator (`f7a1b128…`, this row). The buyer has another name. The role set {ReviewInfo, ReviewOrg, Mediator} is all
review-side, with no buyer-side role, so A7 is clean. Target 23021560 is identifier NULL with 3 mentions and is not
flagged. The sibling planned notice 24254583 (the same buyer's earlier notice for the same goods) prints the same three
sections at the same address. The long name pins the one MV office (R2 F1: do not list this key; the exact-name list
leaves it out). An earlier name variant of the same chamber stays on other keys, so the fold under-merges and never
mis-merges. One body.

**26: 22957397 "Aufsichts- und Dienstleistungsdirektion" (review_or_info), target 1176684: R.** TED 00065133-2025
(notice 24520244; DÖE 26562011) is a 2025-01-29 contract notice of the ZBL Rheinland-Pfalz (Koblenz) for an oils and
lubricants framework. The ADD, at Willy-Brandt-Platz 3, 54290 Trier, poststelle@add.rlp.de, +49 65194940, is printed
twice: ORG-0003 ReviewInfo (`48af48ce…`, this row) and ORG-0005 Mediator (`fd88dc83…`). ORG-0004 ReviewOrg is the
"Vergabekammer Rheinland-Pfalz beim Ministerium …" (Mainz), which has another name. The ZBL sections form their own group
(R2 read 22957396 R as read_rand2 item 27). The role set {ReviewInfo, Mediator} has no buyer-side role and no ReviewOrg,
so A7 is clean. Target 1176684 is identifier NULL with 652 mentions and is not flagged, so its newest-50 city set holds
trier or nothing and no second city. The cached R2 body 24274974 (another ZBL framework) prints the same ADD,
same address, as Mediator. The name has 2 content words but is not generic: it is the title of one Rhineland-Palatinate
authority, seated in Trier, and no other Land uses it. One body.

**27: 22975387 "Vergabekammer Rheinland · c/o Bezirksregierung Köln" (review_or_info), target 12774348: R.** TED
00075529-2025 (notice 24542995; DÖE 26565531) is a 2025-02-03 award notice for tax advice to the Bergische Universität
Wuppertal. The chamber, at Zeughausstraße 2-10, 50667 Köln, vkrheinland@bezreg-koeln.nrw.de, +49 221147-3045, is printed
twice: ORG-0003 ReviewInfo (`791caa3e…`, row 22975386, which R2 read R as read_rand2 item 28) and ORG-0004 ReviewOrg
(`45fa652d…`, this row). The buyer (Bergische Universität, Buyer+AddInfo) and the tenderer (PwC) have other names. The
role set {ReviewInfo, ReviewOrg} has no buyer-side role, so A7 is clean. Target 12774348 is identifier NULL with 495
mentions and is not flagged. The "· c/o Bezirksregierung Köln" form names only the Köln chamber, and this is not the
umbrella "Vergabekammer NRW" that F1 listed. It stays a separate key from 7615 and 11823847, which are the same
chamber, as today. Nothing from another body is folded in. One chamber.

**28: 22965731 "Vergabekammer Niedersachsen beim Nds. Ministerium für Wirtschaft, Arbeit und Verkehr" (review_or_info),
target 1166553: R.** TED 00033345-2025 (notice 24530701; DÖE 26551471) is a 2025-01-15 award notice for Landkreis
Northeim's diesel framework. The chamber, at Auf der Hude 2, 21339 Lüneburg, Manuela.Heise-Kardinahl@rv-lg.niedersachsen.de,
+49 413115-1334, is printed three times: ORG-0002 ReviewInfo, ORG-0003 ReviewOrg (`69c6792a…`, row 22965730, which R2 read
R as read_rand3 item 32) and ORG-0004 Mediator (`4d35db4a…`, this row). The buyer (Landkreis Northeim, one section that is
both Buyer and AddInfo) and the tenderer have other names. The role set is review-side only, so A7 is clean. Target
1166553 is identifier NULL with 13,668 mentions, top target 14 (+256 from 128 rows), city sets {luneburg}/{luneburg}. R2
read it R (read_top1 item 14: newest mention 47414336, same address). Lower Saxony has one chamber, in Lüneburg. One body.

**29: 22868221 "Vergabekammer Westfalen bei der Bezirksregierung Münster" (review_or_info), target 6136373: R.** TED
00754780-2024 (notice 24406486; DÖE 26525811) is a 2024-12-09 award notice for electrical works at the Goetheschule Marl.
The chamber, at Albrecht-Thaer-Str. 9, 48147 Münster, vergabekammer@bezreg-muenster.de, +49 2514111691, is printed three
times: ORG-0002 ReviewInfo (`4cf7bd09…`, row 22868220, which R2 read R as read_rand3 item 33), ORG-0003 ReviewOrg
(`f3db2330…`, this row) and ORG-0004 Mediator. The buyer (Stadt Marl's Zentrale Vergabestelle) and three tenderers have
other names. The role set is review-side only, so A7 is clean. Target 6136373 is identifier NULL with 15,723 mentions,
top target 8 (+316 from 158 rows), city sets {munster}/{munster}. R2 read it R (read_top1 item 8: newest mention
47408324, same street). The long name pins the Westphalian chamber. It is not the umbrella 3353580 "Vergabekammer NRW"
(F1, now unsafe) nor the bare 6622773 "Geschäftsstelle der Vergabekammer". Neither id appears anywhere in 2660, and the
`vergabekammer nrw` twins now show up only in `guard_samples.unsafe_list`. One chamber.

## Notes for the R2b tally

- 0 W and 0 N in items 15-29, so nothing here goes into `GUID_TWIN_UNSAFE_NAMES`.
- Five of the 15 rows (18, 25, 27, 28, 29) are the in-notice partners of rows that R2 read at salt 530002. The reads
  agree: the partner verdicts were R, and the new rows sit in the same group, on the same address, going to the same
  target. Taking the next rows in salted order brings in a pick's notice-mate, so the fresh 60 is less independent of
  the first 60 than 60 rows drawn from fresh notices would be. Weigh that when the bound is restated.
- Item 16's target is the only city-flagged key in this slice. The off-city planned mention (Geilenkirchen) is still not
  located, and it is not this row.
- Evidence for the targets with no newest-mention list in the plan (19560123, 14436538, 14467750, 2463330, 17573857,
  2439295, 23021560, 1176684, 12774348) rests on three things: the name, the clean city screen, and a sibling planned
  notice where the truncated `planned_mention_keys` gives one. The allowed endpoints have no route from an organization to
  its newest notices. None of these names is generic, and none is an umbrella.
