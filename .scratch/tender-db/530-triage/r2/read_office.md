# 530 R2: office two-word keys, all 8 items (read_office)

Source: `guid_twin_plan_2659.json` `body.r2_office_two_words[0..7]`, every planned key with exactly 2 content words whose first word
is an office noun (decision.md R2, "Also read"; `r2_office_two_words_count` = 8, so no salted subset was needed). All 8 are
"Vergabekammer <region>" names, all DE, and each has an existing identifier-NULL provisional target. None is a missing key.

**Question.** Is the target one body (**R**), unclear (**N**), or a homonym or umbrella that collects different bodies (**W**)? Under
A6, a target that already holds two bodies scores W, even when the folded mentions are right.

**Duplicates (skip rule as in read_city1-3).** Four items were already read as top or city targets. They are cited, not re-read:
- item 0, 7615 Vergabekammer Rheinland: `r2_top_targets[0]` and `r2_city_flagged[0]` (read_top1 item 0, R);
- item 1, 3233 Vergabekammer Westfalen: `r2_top_targets[16]` (read_top2 item 16, R);
- item 2, 622 Vergabekammer des Bundes: `r2_top_targets[19]` (read_top2 item 19, R);
- item 5, 3353580 Vergabekammer NRW: `r2_city_flagged[9]` (read_city1 item 9, **W**).

I read the other 4: items 3, 4, 6 and 7. None of the 8 names appears in `r2_incoherent` (A7).

**Method.**
- `/v1/organizations/{target}`: every target has identifier NULL, provisional true, country DE, and the report's mention count, unchanged.
- The target's existing mentions: all three notices the report lists (`notice_ids`), through `/v1/notices/{id}/content`. For each I
  took name, street, postcode, town, e-mail, BT-501 and roles (OPT-300/301, or the DE1 party references in eForms-DE 1.1).
- The planned side: a planned notice from `planned_mention_keys`, plus the planned GUID row through `/v1/organizations/{row}`.
  - That list is cut at 2,000 keys (notices up to 24341274). Item 3 has 2 notices in it and item 4 has 1.
  - Items 6 and 7 have none. Each has 2 planned rows, which are not in `planned_mention_keys`, `r2_random`, the controls, the
    census samples or any other list in the report. Finding them blind would mean probing the 9,119 planned row ids above
    22817047, so I stopped after one probe of an unread consecutive pair (24482814: a Freistaat Bayern/FAU row, not theirs).
  - For items 6 and 7, the only planned-side evidence is the report's city screen: `planned_cities` equals the target's city set,
    and `city_absent` = 0.

**GETs.** 23 public GETs, all 200 and none 408: 9 organizations and 14 notice contents. They are logged as `reader=office` in
`api_requests.log`, 18:23:44Z-18:26:53Z, and went through the shared `.api.lock`/`.api.last` pacing (at least 2.2 s after any
reader's last GET; no other reader logged in that window). The smallest gap is 2 s at the log's 1-second resolution. Bodies are in
`bodies/`.

## Tally

**Read (items 3, 4, 6, 7): R 4, N 0, W 0. Duplicates: 4 (items 0, 1, 2 at R; item 5 at W).**

Across all 8 keys, the one W is item 5, 3353580 "Vergabekammer NRW", which read_city1 already found and remedied. The other seven
names each denote one chamber with one seat. NRW is the exception because it has two chambers, Rheinland (Köln) and Westfalen
(Münster). So the bare office two-word shape "Vergabekammer <X>" is safe when X names one chamber's district (Rheinland, Westfalen,
Südbayern, Nordbayern, des Bundes) or a Land that has one chamber (Niedersachsen, BW). It is an umbrella only when X is a Land
with more than one chamber.

| # | target | name | mentions → +planned (rows) | target cities / planned cities | verdict |
|---|---|---|---|---|---|
| 0 | 7615 | Vergabekammer Rheinland | 29,617 → +1,346 (673) | 50667, koln / koln | duplicate: R (read_top1 0) |
| 1 | 3233 | Vergabekammer Westfalen | 37,696 → +244 (122) | munster / munster | duplicate: R (read_top2 16) |
| 2 | 622 | Vergabekammer des Bundes | 111,305 → +214 (107) | bonn / bonn | duplicate: R (read_top2 19) |
| 3 | 1222120 | Vergabekammer Niedersachsen | 7,937 → +94 (41) | luneburg / luneburg | **R** |
| 4 | 2175136 | Vergabekammer BW | 469 → +48 (24) | karlsruhe / karlsruhe | **R** |
| 5 | 3353580 | Vergabekammer NRW | 675 → +32 (16) | koln, munster / munster | duplicate: **W** (read_city1 9) |
| 6 | 3180 | Vergabekammer Südbayern | 42,828 → +4 (2) | munchen / munchen | **R** |
| 7 | 1163917 | Vergabekammer Nordbayern | 24,306 → +4 (2) | ansbach / ansbach | **R** |

## Items

**0 — 7615 Vergabekammer Rheinland — duplicate, R (read_top1 item 0; also city1 item 0).** This target receives the most planned
mentions in the plan (+1,346 from 673 rows). read_top1 found one chamber, NRW's review chamber for the Rheinland districts at the
Bezirksregierung Köln (Zeughausstraße 2-10, 50667 Köln, VKRheinland@/vergabekammer@bezreg-koeln.nrw.de), in both the newest mentions
and the planned twins (ORG-0003 Lot-ReviewInfo plus ORG-0004 Lot-ReviewOrg). The b flag's second "city" is Köln's postcode printed
in BT-513, not a second body. Two `r2_random` rows that fold into it (22897538 and 22966357) were read R in read_rand3 items 42 and
43. I did not re-read it.

**1 — 3233 Vergabekammer Westfalen — duplicate, R (read_top2 item 16).** read_top2 read its newest mention 47414774 (Albrecht-Thaer-
Straße 9, 48147 Münster, vergabekammer@bezreg-muenster.nrw.de) and the planned notice 24332400, whose twin sits at the same address
and e-mail. There is one Vergabekammer Westfalen, at the Bezirksregierung Münster. The same body also sits under the longer name
6136373 (top item 8), which is a split that already exists, not a homonym. I did not re-read it.

**2 — 622 Vergabekammer des Bundes — duplicate, R (read_top2 item 19).** read_top2 read its newest mention 47414764 (Villemombler
Straße 76, 53123 Bonn, vk@bundeskartellamt.bund.de) and the planned notice 24263691 (twin rows 22760267/68, same address). It is the
federal review chamber institution at the Bundeskartellamt, one body with numbered panels. The stable GUID row 255 is the same body
and is spared by KEEP. The fold lands on 622, never on 255. I did not re-read it.

**3 — 1222120 Vergabekammer Niedersachsen — R.** One chamber, the Land's single review chamber at its economics ministry, seated in
Lüneburg. The target has 7,937 mentions, and the plan adds 94 from 41 rows.
- All three of the target's newest mentions print it at Auf der Hude 2, 21339 Lüneburg, vergabekammer@mw.niedersachsen.de, NUTS
  DE935, with BT-501 `t:04131153308` (a Lüneburg telephone number):
  - 47414678 (2026-10-16, Stadt Langenhagen), Lot-ReviewOrg;
  - 47414415 (2026-10-16, Göttinger Entsorgungsbetriebe), Lot-ReviewInfo plus Lot-ReviewOrg, with BT-16 "Vergabekammer
    Niedersachsen beim Niedersächsischen Ministerium für Wirtschaft, Arbeit, Verkehr und Digitalisierung";
  - 47413611 (2026-10-15, OOWV), Mediator plus ReviewInfo plus ReviewOrg in one section.
- The planned notices are 24265007 (2024-10-28, OOWV vehicles) and 24312274 (2024-10-29, Hochschule Osnabrück). Each prints the name
  three times: ORG-0003 Lot-ReviewInfo, ORG-0004 Lot-ReviewOrg and ORG-0005 Lot-Mediator, each under its own v4 GUID, all at Auf der
  Hude 2, Lüneburg, with the same e-mail. That is the same chamber printed once per role, exactly as 47413611 prints it in one section.
- The planned rows 22761479 (`6FCACBC1…` = 24265007 ORG-0003) and 22795253 (`4697AA1B…` = 24312274 ORG-0003) each hold 2 mentions.
- The city sets agree ({luneburg} on both sides). Different buyers across the Land (Langenhagen, Göttingen, Brake, Osnabrück) all
  name the same address.

**4 — 2175136 Vergabekammer BW — R.** One chamber, the Vergabekammer Baden-Württemberg at the Regierungspräsidium Karlsruhe. "BW"
is the Land abbreviation, and the Land has one chamber. The target has 469 mentions, and the plan adds 48 from 24 rows.
- All three of the target's newest mentions are eForms-DE 1.1 notices from 2024:
  - 26469722 (2024-10-02) and 26430055 (2024-08-16), both Universitätsklinikum Freiburg;
  - 26374039 (2024-06-13), Gemeinde Dachsberg.
  Each prints "Vergabekammer BW" at Durlacher Allee 100 (once "Alle"), 76137 Karlsruhe, vergabekammer@rpk.bwl.de, as
  AppealInformationParty, AppealReceiverParty and MediationParty. Two unrelated buyers name the same chamber.
- The planned notice is 24277214 (2024-10-23, Universitätsklinikum Freiburg, HPLC/MS/MS). It prints the name as ORG-0003
  Lot-ReviewInfo, ORG-0004 Lot-ReviewOrg and ORG-0005 Lot-Mediator, three v4 GUIDs, all at Durlacher Allee 100, Karlsruhe, with the
  same e-mail. That is one chamber printed per role.
- The planned row 22770347 (`CDAF5AC4…` = ORG-0003) holds 2 mentions.
- The city sets agree ({karlsruhe} on both sides).

**5 — 3353580 Vergabekammer NRW — duplicate, W (read_city1 item 9).** read_city1 found that the target already holds both NRW
chambers:
- 46376226 and 46082162 print "Vergabekammer NRW" at Zeughausstraße 2-10, 50667 Köln (Rheinland, Leitweg 05315-…);
- 44543854 prints it at Albrecht-Thaer-Str. 9, 48147 Münster (Westfalen).

So the b flag {koln, munster} is two bodies under one name. The 16 planned rows (Wald und Holz NRW notices 24313214 and 24330412)
are the Münster chamber printed per role. The refusal inside each notice is right, but the fold adds 32 Westfalen mentions to a row
that already mixes Westfalen and Rheinland. A6 scores that W. The name guard misses it ("vergabekammer nrw" has 2 content words and
is on no generic list).

I did not re-read it. The office-key reading agrees: of the eight "Vergabekammer <X>" keys, this is the only one whose X is a Land
with more than one chamber. Remedy, as read_city1 gives it: `Vergabekammer NRW` goes into `GUID_TWIN_UNSAFE_NAMES` via
`extra_unsafe_names`, or the shape "Vergabekammer <Land with more than one chamber>" does (Bayern, as the umbrella over items 6 and 7,
is the other one in this data). Then R1 is re-run and a fresh random 60 is read at salt 530003.

**6 — 3180 Vergabekammer Südbayern — R.** One chamber, the Vergabekammer Südbayern at the Regierung von Oberbayern in München
(Maximilianstraße 39, Regierung PO box 80534). The target has 42,828 mentions, and the plan adds 4 from 2 rows.
- All three of the target's newest mentions print it with vergabekammer.suedbayern@reg-ob.bayern.de, NUTS DE212, München:
  - 47414804 (2026-10-16, Klinikverbund Allgäu, Mindelheim), Lot-ReviewInfo plus Lot-ReviewOrg, BT-501 `002`, postcode 80534;
  - 47414801 (2026-10-16, Zweckverband Berufliches Schulzentrum Kempten), the same roles, BT-501 `001`, Maximilianstraße 39,
    80335;
  - 47408434 (2026-10-15, Münchner Wohnen), Lot-ReviewOrg, BT-501 `t:08921762411`, street lines "Regierung von Oberbayern /
    Vergabekammer Südbayern / Maximilianstrasse 39", 80534.
  In 47408434 the mediator is a separately named section, "Schlichtungsstelle", under the same e-mail. It is outside this key.
- Buyers from Schwaben (Mindelheim, Kempten) and Oberbayern (München) all name the one chamber. Its district is Oberbayern,
  Niederbayern and Schwaben, so
  "Südbayern" is a named district, not an umbrella over several chambers.
- The planned side could not be read (see Method). The report's planned city set is {munchen}, equal to the target's, with
  `city_absent` 0. With 2 rows and 4 mentions, the fold's reach is minimal.
- The same body also sits under other rows: the Südbayern 234 row 9766 (10,423 mentions, judge_coverage) and the stable GUID row
  1348 "Regierung von Oberbayern - Vergabekammer Südbayern" (4,430 mentions, 991 Tenders, census 2655). Those are splits that
  already exist, not homonyms. The not-planned controls 30971684/734/757 (that
  longer name) stay not planned in the report.

**7 — 1163917 Vergabekammer Nordbayern — R.** One chamber, the Vergabekammer Nordbayern at the Regierung von Mittelfranken in Ansbach
(Promenade 27, 91522 Ansbach). The target has 24,306 mentions, and the plan adds 4 from 2 rows.
- All three of the target's newest mentions print it at Promenade 27, 91522 Ansbach, vergabekammer.nordbayern@reg-mfr.bayern.de,
  NUTS DE251:
  - 47414645 and 47412601 (2026-10-16, two copies of the Gemeinde Schlammersdorf / VG Kirchenthumbach fire-engine notice), Lot-
    ReviewInfo plus Lot-ReviewOrg, BT-501 `VergabeNord`, with the Regierung von Mittelfranken chamber URL;
  - 47408535 (2026-10-15, Stadt Würzburg), Lot-ReviewOrg, BT-501 `000`, with BT-16 "Regierung von Mittelfranken - Sachgebiet 21
    - Handel und Gewerbe, Vergabekammer Nordbayern".
- Buyers from the Oberpfalz and Unterfranken name the one chamber. Its district is the three Franconian Regierungsbezirke plus the
  Oberpfalz, so "Nordbayern" is a named district, not an umbrella.
- The planned side could not be read (see Method). The report's planned city set is {ansbach}, equal to the target's, with
  `city_absent` 0.
- Census 2655 lists the longer name "Regierung von Mittelfranken - Vergabekammer Nordbayern" as the stable GUID row 2767 (2,732
  mentions, 637 Tenders), a split of the same body that already exists, and a not-twin GUID row 22134004 under this exact name (2
  mentions). Neither is in the plan's 11,103 rows.
