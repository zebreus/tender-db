# 530 R2: city-flagged targets, items 0-13 (read_city1)

Source: `guid_twin_plan_2659.json` `body.r2_city_flagged[0..13]`. These are the targets (or target keys) flagged by the A6 city screen:
- flag a: a planned mention's city is missing from the target's city set (`city_absent`);
- flag b: the target's newest 50 mentions show 2 or more cities (`multi_city`);
- flag c: the key's planned mentions alone show 2 or more cities (`multi_city_planned`).

Every item is DE, and every `target_id` is an existing identifier-NULL provisional row. None is a missing key.

Question: is the target one body (**R**), unclear (**N**), or a homonym or umbrella collecting different bodies (**W**)? Under A6,
a target that already holds two bodies scores W, even when the folded mentions are right.

**Duplicates.** Items 0-2 are also in `r2_top_targets` (indices 0, 3 and 9). They were read there (`read_top1.md`, R each) and are
not re-scored here.

**Evidence for each item.**
- `/v1/organizations/{target}`: name, identifier NULL, mention count.
- The target's newest mentions (`notice_ids`) through `/v1/notices/{id}/content`: name, street, postcode, city, e-mail,
  BT-501 and OPT-300/301 roles.
- At least one notice of a planned row, plus one planned GUID row through `/v1/organizations/{id}`. The planned notices come from
  `planned_mention_keys`, which is cut at 2,000 keys (notices up to about 24.34M, i.e. 2024-10/11).
  - So planned mentions in later notices could not be found. That covers several of the off-city mentions behind flags a and c.
  - The allowed endpoints have no per-name listing.
- eForms-DE 2.1 notices carry BT-501 in `ND-CompanyLegalEntity#n` subsections, as `read_rand2.md` noted.
- Some targets' "newest" notices are 2010-2011 text-era notices, re-inserted by the 526 re-parse. Those notices print a town
  but no BT-513, so they add nothing to the city screen.

**GETs.** 57 public GETs under tag `city1`: 20 organizations and 37 notice contents.
- All returned 200; there were no 408s.
- Each waited at least 2.1 s behind the shared `.api.lock`/`.api.last` spacing, and all are logged in `api_requests.log`.
- GSI's org rows and three notices (20316475, 23761350, 22851889, 25897330, 26964318) were already in `bodies/` from rand2 and were
  reused, not fetched again.

## Tally

**Items 3-13 (11 read): R 10, N 0, W 1. Duplicates: 3 (items 0-2).**

The W is item 9, **3353580 "Vergabekammer NRW"**. NRW has two review chambers, and this target holds both: Rheinland in Köln and
Westfalen in Münster, at different addresses. Where a Leitweg-ID is printed, it differs (05315 Köln vs 05515 Münster). Under the
decision's R2 rule:
- the name, or the shape "Vergabekammer <Land>" for a Land with more than one chamber, goes into `GUID_TWIN_UNSAFE_NAMES` via
  `extra_unsafe_names`;
- R1 is re-run;
- a fresh random 60 is read at salt 530003.

The same key is also `r2_office_two_words[5]`, so the office reader should reach the same finding.

| # | target | name | mentions → +planned (rows) | flags | verdict |
|---|---|---|---|---|---|
| 0 | 7615 | Vergabekammer Rheinland | 29,617 → +1,346 (673) | b | duplicate (top 0) |
| 1 | 1165385 | Deutsche Gesellschaft für internationale Zusammenarbeit (GIZ) GmbH | 4,851 → +488 (244) | a 2, c | duplicate (top 3) |
| 2 | 30899495 | Stadt Aachen - Zentrale Vergabestelle FB60/300 | 64 → +302 (151) | a 1, c | duplicate (top 9) |
| 3 | 5180467 | Leinemann Partner Rechtsanwälte mbB | 250 → +88 (26) | b (10179, berlin) | R |
| 4 | 16003275 | Stadt Achim c/o Landkreis Verden - Zentrale Vergabestelle | 21 → +80 (40) | c (verden, verden aller) | R |
| 5 | 11195375 | Wald und Holz NRW | 302 → +76 (38) | a 10 (bad driburg), c | R |
| 6 | 1178976 | Bezirksregierung Münster | 1,138 → +50 (25) | b (munster, munster westfalen) | R |
| 7 | 1496424 | AOK NordWest - Die Gesundheitskasse | 164 → +40 (20) | b (dortmund, hagen) | R |
| 8 | 31365873 | FREISTAAT BAYERN, VERTRETEN DURCH DIE UNIVERSITÄT ERLANGEN-NÜRNBERG | 3 → +36 (10) | c (erlangen, nurnberg) | R |
| 9 | 3353580 | Vergabekammer NRW | 675 → +32 (16) | b (koln, munster) | **W** |
| 10 | 1314444 | Universität Duisburg-Essen | 362 → +26 (13) | a 13 (duisburg vs essen) | R |
| 11 | 4193988 | Verband der Ersatzkassen e. V. (vdek) | 146 → +24 (12) | b (4 cities), c | R |
| 12 | 8223129 | SVLFG, als landwirtschaftliche Krankenkasse (LKK) | 45 → +16 (8) | a 4 (kassel), c | R |
| 13 | 20316475 | GSI - Gesellschaft für Schweißtechnik International mbH | 3 → +16 (8) | c (duisburg, munchen) | R |

## Items

**0 — 7615 Vergabekammer Rheinland — duplicate.** Also `r2_top_targets[0]`. Read in `read_top1.md` item 0 (R): one chamber at
Bezirksregierung Köln, and the "50667" city is Köln's postcode typed into BT-513. Not re-scored.

**1 — 1165385 GIZ GmbH — duplicate.** Also `r2_top_targets[3]` and `r2_random[16]`. Read in `read_top1.md` item 3 (R): one company
with seats in Bonn and Eschborn. Not re-scored.

**2 — 30899495 Stadt Aachen - Zentrale Vergabestelle FB60/300 — duplicate.** Also `r2_top_targets[9]`. Read in `read_top1.md`
item 9 (R). Not re-scored.

**3 — 5180467 Leinemann Partner Rechtsanwälte mbB — R.** One law firm.
- Target (250 mentions): its newest mentions, 45876070 and 45868370 (2026-07, MEDICLIN Müritz-Klinikum and Plau am See IT
  tenders), print Leinemann at Alte Jakobstraße 83-84, 10179 Berlin, it-beschaffung@leinemann-partner.de, as Lot-AddInfo and
  Lot-ReviewOrg. The BT-501 there is `DE 984426879`; it was evidently not taken as a key, since the mention sits on this
  identifier-NULL row.
- Planned notices: 23554422 (Asklepios, 2023-11) and 24214207 (UKGM, 2024-09) print the twin ORG-0002 Lot-ReviewOrg and ORG-0003
  Lot-AddInfo at the firm's earlier Berlin address, Friedrichstraße 185-190, 10117. Both use the same it-beschaffung@ e-mail.
  Planned row 22727942 has 4 mentions.
- Flag b: "10179" is the new postcode typed into BT-513, so both "cities" are Berlin. The planned city set is {berlin}.
- The firm is the hospitals' procurement agent, and the platform prints it in the review-body slot. That is a role-quality issue,
  not a second body.

**4 — 16003275 Stadt Achim c/o Landkreis Verden - Zentrale Vergabestelle — R.** One buyer: the town of Achim, buying through the
county's central procurement office.
- Target (21 mentions): its newest mentions, TED 2023/S 203-635375 and 206-647651 (notices 23510955 and 23507701, Ganztagsausbau
  GS Baden), give Lindhooper Straße 67, 27283 Verden, vergabestelle@landkreis-verden.de. These are TED forms with no BT-513, which
  is why the target's city set is empty.
- Planned notices: 24260981 (IGS Achim, PV) and 24337642 (GS Bierden heating) print that exact address and e-mail. In 24260981 it
  sits on Procedure-Buyer and Lot-AddInfo; in 24337642 it sits on Procedure-Buyer, Lot-AddInfo and Lot-TenderReceipt, under 2 or 3
  GUIDs (planned row 22814308).
- Flag c: "verden" and "verden aller" are one town; "verden aller" is the folded official name "Verden (Aller)". I did not locate
  the planned mention that prints that form: it is past the key cut. In 24260981 the same form appears on the local winner,
  Volpro GmbH.

**5 — 11195375 Wald und Holz NRW — R.** One state forestry enterprise (Landesbetrieb) of NRW.
- Target (302 mentions): its newest mentions, 26866950 and 26798362 (2025-11 and 2026-02, Naturwaldzellen), give "Zentrale / FB I -
  Zentrale Vergabestelle", Albrecht-Thaer-Straße 34, 48147 Münster, vergabe@wald-und-holz.nrw.de.
- Planned notices: 24313214 (chainsaw framework), 24330412 and 24331436 (Regionalforstamt Hochstift / Rureifel works) print the same
  Münster address on Procedure-Buyer, Lot-AddInfo and Lot-TenderReceipt GUID sections. Planned row 22795961 has 2 mentions.
- Flag a: 10 planned mentions give Bad Driburg. They are past the 2,000-key cut, and I could not locate them.
  - Bad Driburg is the seat of the enterprise's own Regionalforstamt Hochstift, named in the titles of 24330412 and of the
    target's 44543854.
  - So those mentions are this body printing a regional office, not a homonym. "Wald und Holz NRW" has no second bearer.

**6 — 1178976 Bezirksregierung Münster — R.** One authority.
- Target (1,138 mentions): its newest mentions, 4082297, 4060639 and 4112908 (2010 text-era notices, re-inserted), print
  "BEZIRKSREGIERUNG MÜNSTER", MÜNSTER, Domplatz 1-3.
- Planned notice 24328902 (Betreuungsdienstleistung Notunterkünfte, 2024-10) prints Bezirksregierung Münster, "Dezernat 12 / Zentrale
  Vergabestelle", Domplatz 1-3, 48143 Münster. It sits on three GUID sections: Procedure-Buyer, Lot-AddInfo and Lot-ReviewInfo
  (planned row 22806772).
- The review body in that notice is a separately named section, "Vergabekammer Westfalen bei der Bezirksregierung", and is not in
  this group.
- Flag b: "munster" and "munster westfalen" are two spellings of one city.

**7 — 1496424 AOK NordWest - Die Gesundheitskasse — R.** One statutory health insurer.
- Target (164 mentions):
  - Its newest mentions, 45852543, 27351981 and 27339608 (2026, AOK-Gemeinschaft modification notices), list all 11 AOKs as
    separate, distinctly named buyers. AOK NordWest's entry is Kopenhagener Straße 1, 44269 Dortmund, its head office.
  - The target's other city, hagen, belongs to AOK NordWest's own procurement unit: "FB Beschaffungsmanagement", 58079 Hagen
    (a large-customer postcode), vergabestelle@nw.aok.de, as its planned notices print it.
- Planned notices: 24312523 (post services Schleswig-Holstein) and the joint contrast-media notices 24293738 and 24294429. Each prints
  "AOK NordWest - Die Gesundheitskasse." with a trailing period and the Hagen address. The name sits on Procedure-Buyer, Lot-AddInfo
  and Lot-TenderReceipt (ORG-0001/0020/0022 in the joint notices). Planned row 22782410 has 2 mentions.
- Flag b: Dortmund (head office) and Hagen (procurement unit) are two sites of one Körperschaft. The other AOKs keep their own names
  and do not fall into this key.

**8 — 31365873 FREISTAAT BAYERN, VERTRETEN DURCH DIE UNIVERSITÄT ERLANGEN-NÜRNBERG — R (planned rows not located).** One body:
FAU, acting for the Free State.
- Target (3 mentions): 4206714 and 4247041 (2010, identification-badge service) and 4364687 (TED 2011/S 9-012558, award) give
  Schlossplatz 4, 91054 Erlangen, with an RRZE contact (daniel.de.west@rrze.uni-erlangen.de). These are text-era and TED forms, so
  the target's city set is empty.
- Planned rows (10 rows, 36 mentions, cities Erlangen and Nürnberg) are not in `planned_mention_keys` or any sample list. I found
  none of them through the allowed endpoints.
- Flag c: the two planned cities are the two cities in the university's own name, and the name_norm can denote only that university.
- This is not the polity-umbrella shape. A8 guards a bare "Freistaat X"; here the represented body is named. The sibling key
  "… vertreten durch die Friedrich-Alexander-Universität Erlangen-Nürnberg" (a missing target, sample 24482814) is a remaining
  split, not a homonym.
- Weaker than the others: R rests on the target's notices, the name and the city pair, not on a planned notice.

**9 — 3353580 Vergabekammer NRW — W.** An umbrella name. The target already holds two different review chambers.
- Target (675 mentions), newest mentions:
  - 46376226 and 46082162 (2026-09, Technische Betriebe Remscheid) print "Vergabekammer NRW" at Zeughausstraße 2-10, 50667 Köln,
    with BT-501 `Leitweg-ID 05315-03002-81` (05315 = Köln). That is Vergabekammer Rheinland at Bezirksregierung Köln.
  - 44543854 (2025-06, Wald und Holz NRW, Regionalforstamt Hochstift) prints "Vergabekammer NRW" at Albrecht-Thaer-Str. 9, 48147
    Münster, vergabekammer@bezreg-muenster.nrw.de. Its BT-501 is `DEA33`, a NUTS code. That is Vergabekammer Westfalen at
    Bezirksregierung Münster.
  - Wald und Holz's 26866950 (2026-02) prints the same Münster chamber with BT-501 `05515-03004-07` (05515 = Münster), against
    Köln's `05315-…`. I did not confirm that this mention binds to 3353580; it corroborates only.
  - So flag b's {koln, munster} is two bodies under one printed name. NRW has no single "Vergabekammer NRW". The data model keeps
    the two chambers apart elsewhere: 7615 and 11823847 for Rheinland, 6136373 and 3233 for Westfalen.
- Planned notices: 24313214 and 24330412 (Wald und Holz NRW, 2024-10). The twin is ORG-0003 Lot-ReviewOrg plus ORG-0004
  Lot-Mediator, both at the Münster chamber's address and e-mail. Planned row 22795963 has 2 mentions.
  - Inside each notice, the refusal is therefore right: one chamber, printed once per role.
  - The fallback is wrong: it adds 32 Westfalen mentions to a row that already mixes Westfalen and Rheinland. A6 scores that W
    ("the fold extends a wrong merge").
- The name guard misses it: "vergabekammer nrw" has 2 content words and is on no generic list.
- Remedy under the decision: add `Vergabekammer NRW` to `GUID_TWIN_UNSAFE_NAMES`. Better, add the shape "Vergabekammer <Land>"
  for a Land with more than one chamber. "Vergabekammer Bayern" would be the same umbrella over Nordbayern and Südbayern.
  Then re-run R1 and read a fresh random 60 at salt 530003.

**10 — 1314444 Universität Duisburg-Essen — R.** One university with two campuses.
- Target (362 mentions): its newest mentions, 4108216 and 4128803 (2010 text-era), print "UNIVERSITÄT DUISBURG-ESSEN", DUISBURG.
  The target's BT-513 set {essen} comes from its eForms mentions at the Essen campus.
- Planned notice 24313286 (glass cleaning, Campus Duisburg, 2024-10) prints "Universität Duisburg-Essen", Sachgebiet Einkauf,
  Forsthausweg 2, 47057 Duisburg, einkauf@uni-due.de. It sits on Procedure-Buyer, Lot-AddInfo and Lot-TenderReceipt GUID sections
  (planned row 22796025). The review body there is the separate "Vergabekammer Rheinland" section.
- Flag a: all 13 planned mentions are Duisburg and the target set is {essen}. These are the university's two campuses, both in its
  name, and the name has no other bearer.

**11 — 4193988 Verband der Ersatzkassen e. V. (vdek) — R.** One registered association. Its state offices (Landesvertretungen) are
units of it, not separate bodies.
- Target (146 mentions): its newest mentions are 45647132 and 32094941 (2026-09, contrast-media frameworks), 44539049 and 44501561
  (2025-06). Their BT-501 is each office's phone number (`t:0431…`, `t:06131…`, `t:0681…`, `t:0211…`), which the gate does not
  take as a key. So these mentions already put four Landesvertretungen on this row:
  - Kiel, Wall 55;
  - Mainz, Wilhelm-Theodor-Römheld-Str. 22;
  - Saarbrücken, Heinrich-Böcking-Str. 6-8;
  - Düsseldorf, Ludwig-Erhardt-Allee 9 (as printed).
  Each is a co-buyer, with url www.vdek.com.
- Planned notices: 24293738 and 24294429 (2024-10, the same joint contrast-media procedure, led by AOK NordWest) print the same three
  Kiel, Mainz and Saarbrücken addresses as ORG-0009/0010/0011 Procedure-Buyer under three GUIDs (planned row 22782418).
- Flags b and c: every city is a vdek state office. The fold does what the identifier-less copies of the same procedures already do.
  A legal-entity key for the e.V. would also join them.
- Same treatment as A7's "buyer and its department", as GIZ's two seats (top item 3) and as GSI's branches (item 13). A reader who
  counts each Landesvertretung as a separate buyer would score this N, not W: the name denotes no other organisation.

**12 — 8223129 Sozialversicherung für Landwirtschaft, Forsten und Gartenbau (SVLFG), als landwirtschaftliche Krankenkasse (LKK) — R.**
One federal social-insurance carrier.
- Target (45 mentions): its newest mentions, 45647132 and 32094941 (2026-09) and 44498728 (2025-06, Open-House RLP/Saarland), print
  SVLFG at Theodor-Heuss-Str. 1, 67346 Speyer, www.svlfg.de. The BT-501 there is `t:056178500`, a 0561 Kassel phone number, i.e.
  the head office's number on the Speyer section.
- In the same notices, the Kassel section carries VAT `DE264833463` and binds to that VAT row. That is why the target's city set is
  {speyer}.
- Planned notices: 24293738 and 24294429 print it twice as Procedure-Buyer:
  - ORG-0018 at Weißensteinstr. 70-72, 34131 Kassel (head office);
  - ORG-0019 at the Speyer office.
  Planned rows 22782427 and 22782428 hold 2 mentions each. "Landwirtschaftliche" vs "landwirtschaftliche" folds in name_norm.
- Flag a: Kassel is missing from the target's newest-50 set {speyer}. It is SVLFG's own seat, not another body. The split between
  the VAT row and this fallback row is the 300 §10 split, outside 530.

**13 — 20316475 GSI - Gesellschaft für Schweißtechnik International mbH — R.** One company (GmbH) with branch institutes (SLV).
- Target (3 mentions): all from TED 2022/S 252-734469 (notice 22851889, X-ray unit, reused body):
  - ORG-1 Aachener Straße 172, 40223 Düsseldorf (head office), contracting body;
  - ORG-2 and ORG-3 Bismarckstraße 85, 47057 Duisburg (SLV Duisburg), further-info and review-body slots.
  All share wiacker@slv-duisburg.de and the gsi-slv.de site.
- Planned notice: the `r2_random[25]` row 23761350 (notices 25897330 and 26964318, TED 00378637-2026 plus its DÖE copy, reused
  bodies, R in `read_rand2.md` item 25). It prints the same HQ buyer and twin ReviewOrg + AddInfo sections at the Duisburg branch,
  with the same e-mail.
- Flag c: {duisburg, munchen}. München is GSI's SLV München branch. I could not locate that planned mention (past the key cut), but
  GSI is one legal entity, and the name has no second bearer.
