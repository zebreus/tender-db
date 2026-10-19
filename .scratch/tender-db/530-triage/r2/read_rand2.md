# 530 R2: random planned GUID rows, items 15-29 (read_rand2)

Scope: `body.r2_random[15..29]` (0-based, so the 16th to the 30th row) of `guid_twin_plan_2659.json`. That is 10 rows
from the buyer stratum (15-24) and 5 from the review_or_info stratum (25-29). Every row is planned whole, `twin:true`,
2 mentions (the TED and DÖE copies of one notice), 1 Tender, not in KEEP and not stable.

Question for each row: if all its mentions move to the target (the identifier-NULL row for (name_norm, DE)), is that
**R** (the target is the same body and the GUID was a per-section mint), **N** (cannot tell) or **W** (the target name
covers more than one body, the target would take another body's mentions, or the GUID is a real stable key)?

Method:
- For each row: GET the GUID row, GET the target, and GET the TED copy's `/v1/notices/{id}/content`. The notice gives
  every ORG section's name, BT-501, address, e-mail and its OPT-300/301 role.
- For GSI (item 25), I also read the DÖE copy and the target's newest notice from the plan (22851889).
- Target city sets come from the plan's city screen. `city.flagged` is complete (41 = `flagged_count`). Only two of
  these 15 targets are on it: GIZ (16) and GSI (25). The others passed both screens: no planned city is missing from
  the target's newest-50 city set, and the set has fewer than 2 cities.
- 44 GETs, all 200, no 408s. They are logged as `reader=rand2` in `r2/api_requests.log`. The logger takes a lock, so
  GETs from all parallel readers are at least 2.1 s apart. Bodies are in `r2/bodies/`.

Note for other readers: in eForms-DE 2.1 / SDK 1.13 notices, such as GSI's 2026 notice, `/content` carries BT-501 in
`CompanyLegalEntity` subsections (`ND-CompanyLegalEntity#n`), not in the `Organization` section. Older 2024 notices
carry it in the `Organization` section.

**Tally: 15 R, 0 N, 0 W.** Two rows (17, 24) have no identifier-NULL target yet (missing key). They are R on identity
only if Build 2 lets the 234 path mint the fallback row.

## Items

**15: 22858429 "Landkreis Vorpommern-Greifswald, Der Landrat" (buyer), target 21537255: R.**
- TED 00681457-2024 (notice 24394385), a contract notice for the Gesundheitsamt platform. The same name and the same
  office are printed three times, each under its own v4 UUID:
  - ORG-0001 Procedure-Buyer, `fd37e5ff…` (this row);
  - ORG-0002 Lot-AddInfo, `1655532f…`;
  - ORG-0004 Lot-TenderReceipt, `6fa07045…`.
- All three give Rechtsamt / Sachgebiet Zentrale Vergabestelle, Feldstraße 85a, 17489 Greifswald, ZentraleVergabestelle@kreis-vg.de.
- The review body is a different name (the MV Vergabekammern in Schwerin) and is not in the group.
- Target 21537255: same name, DE, identifier NULL, provisional, 15 mentions. Its city screen is clean (Greifswald).
- One county administration. The GUID is a per-section mint.

**16: 22839698 "Deutsche Gesellschaft für Internationale Zusammenarbeit (GIZ) GmbH" (buyer), target 1165385: R.**
- TED 00690717-2024 (notice 24369991), Bolivia ProTransición. Three sections each carry their own UUID:
  - ORG-0001 Procedure-Buyer, `d01a1a41…` (this row);
  - ORG-0002 AddInfo;
  - ORG-0004 TenderReceipt.
- All three give Dag-Hammarskjöld-Weg 1-5, 65760 Eschborn and the same contact.
- Target 1165385: identifier NULL, 4,851 mentions, target city set {eschborn}. It takes 244 planned rows.
- The key is city-flagged only because some planned mentions say Bonn. GIZ is one GmbH with registered seats in Bonn
  and Eschborn, so that does not mean a homonym.
- This row is Eschborn and matches the target set. Same body.

**17: 22856837 "Gemeinde Kreien über Amt Eldenburg -Lübz" (buyer), no target (missing key): R, conditional on a mint.**
- TED 00672656-2024 (notice 24392251), the Kreien fire station design. The group:
  - ORG-0001 Procedure-Buyer, `7890b8f1…` (this row);
  - ORG-0003 Lot-AddInfo, `f0b8d1bc…`.
- Both give Am Markt 22, 19386 Lübz and vergabestelle@sis-schwerin.de.
- ORG-0005 TenderReceipt is "Gemeinde Kreien über Amt Eldenburg -Lübz c/o SIS … Zentrale Vergabestelle". That is
  another name_norm, so it stays outside the group.
- The partner row 22856839 is also planned. No identifier-NULL row exists for the key, so the 234 path would mint one
  row for both. That row would hold only this municipality's mentions.
- Kreien is one Gemeinde in Amt Eldenburg Lübz, so the minted row is one body.
- If Build 2 keeps missing-key rows instead, nothing moves.

**18: 22776193 "Stadt Radevormwald" (buyer), target 6048109: R.**
- TED 00645411-2024 (notice 24285243), gigabit network works. Three sections each carry their own UUID:
  - ORG-0001 Procedure-Buyer (this row);
  - ORG-0002 AddInfo;
  - ORG-0006 TenderReceipt.
- All three give Zentrale Vergabestelle, Hohenfuhrstraße 13, 42477 Radevormwald.
- The Vergabekammer Rheinland sections have another name and form their own group.
- Target 6048109: identifier NULL, 60 mentions, city screen clean. There is only one Radevormwald in DE.

**19: 22835704 "Stadt Remscheid, Fachdienst Recht und Vergabe, Abteilung Zentrale Vergabestelle" (buyer), target 16921787: R.**
- TED 00676147-2024 (notice 24364655), Digitalpakt Schule. The same name, at Theodor-Heuss-Platz 1, 42853 Remscheid,
  ausschreibung@remscheid.de, appears four times under four UUIDs:
  - ORG-0001 Procedure-Buyer (this row);
  - ORG-0002 AddInfo;
  - ORG-0003 Lot-ReviewInfo;
  - ORG-0006 TenderReceipt.
- The ReviewOrg is Vergabekammer Rheinland (Köln), which is a different name.
- A buyer printing itself as ReviewInfo is the usual "where to ask about review deadlines" entry. It is not an A7
  ReviewOrg mix.
- Target 16921787: identifier NULL, 805 mentions, city screen clean. It is one city procurement office.

**20: 22810161 "Bau- und Liegenschaftsbetrieb NRW Dortmund" (buyer), target 8119131: R.**
- TED 00717712-2024 (notice 24333044), BLB NRW / NL Dortmund / JVK Fröndenberg. The group:
  - ORG-0001 Procedure-Buyer (this row);
  - ORG-0002 AddInfo.
- Both give Einkauf und Vertragsmanagement, Emil-Figge-Str. 91, 44227 Dortmund, do.einkauf@blb.nrw.de.
- Two other sections have different names and stay outside the group: "BLB NRW - Justiziariat" (Düsseldorf,
  ReviewInfo) and "Vergabesatellit BLB NRW" (TenderReceipt).
- Target 8119131: identifier NULL, 503 mentions, city screen clean.
- The name carries the branch city, so the target is the Dortmund Niederlassung of the one Landesbetrieb, not BLB NRW
  as a whole.

**21: 22822634 "Stadtverwaltung Brilon" (buyer), target 13011247: R.**
- TED 00706186-2024 (notice 24348159), Schulzentrum Brilon ventilation. Three sections each carry their own UUID:
  - ORG-0001 Procedure-Buyer (this row);
  - ORG-0002 AddInfo;
  - ORG-0004 TenderReceipt.
- All three give Zentrale Vergabestelle, Am Markt 1, 59929 Brilon, submission@brilon.de.
- Target 13011247: identifier NULL, 20 mentions, city screen clean. It is one town administration.

**22: 22843947 "Stadt Sundern" (buyer), target 8028818: R.**
- TED 00711122-2024 (notice 24375779), Technische Dienste Sundern timber/steel works. Three sections each carry their
  own UUID:
  - ORG-0001 Procedure-Buyer (this row);
  - ORG-0002 AddInfo;
  - ORG-0005 TenderReceipt.
- All three give Abteilung 2.2 Vergabeservice, Rathausplatz 1, 59846 Sundern.
- Target 8028818: identifier NULL, 33 mentions, city screen clean. There is only one Sundern (Sauerland).

**23: 22840993 "Land Niedersachsen vertreten durch IT.Niedersachsen FG 24 - Zentrale Vergabestelle IT" (buyer), target 7652: R.**
- TED 00691890-2024 (notice 24371743), GIS software. Three sections each carry their own UUID:
  - ORG-0001 Procedure-Buyer (this row);
  - ORG-0002 AddInfo;
  - ORG-0004 TenderReceipt.
- All three give Göttinger Chaussee 259, 30459 Hannover and one it.niedersachsen.de contact.
- The ReviewOrg is the VK Niedersachsen in Lüneburg, a different name.
- Target 7652: identifier NULL, 577 mentions, city screen clean.
- The name pins IT.N's FG 24 procurement unit, not the Land at large. It is one office.

**24: 22895106 "Sport- und Bäderbetriebe Essen, Verfahrensbegleitung durch die GVE Grundstücksverwaltung Stadt Essen GmbH" (buyer), no target (missing key): R, conditional on a mint.**
- TED 00780697-2024 (notice 24440612), Baldeneysee grandstand. Three sections each carry their own UUID:
  - ORG-0001 Procedure-Buyer (this row);
  - ORG-0002 AddInfo;
  - ORG-0005 TenderReceipt.
- All three give Rottstraße 17, 45127 Essen, vergabe@gve.essen.de.
- In `missing_target_samples` the key has 6 planned rows and 12 mentions, planned cities {essen}. The 234 mint would
  give one row for one buyer-and-agent pair, and the name is long and specific.
- Same body. It depends on Build 2 minting rather than keeping.

**25: 23761350 "GSI - Gesellschaft für Schweißtechnik International mbH" (review_or_info), target 20316475: R.**
- TED 00378637-2026 (notice 25897330, an eForms-DE 2.1 contract notice for phased-array ultrasound units). The DÖE copy
  26964318 is identical.
- The BT-501 values sit in `CompanyLegalEntity` #0-#3:
  - ORG-0001 Procedure-Buyer: `13251`, a non-GUID value, at the HQ address Aachener Straße 172, 40223 Düsseldorf;
  - ORG-0002 Lot-ReviewOrg: `2d9133e3…`, Bismarckstraße 85, 47057 Duisburg;
  - ORG-0003 Lot-AddInfo: `84c9608a…` (this row), at the same Duisburg address.
- All three give wiacker@slv-duisburg.de.
- So the twin group is {ReviewOrg, AddInfo}: the buyer named its own SLV Duisburg branch as the review body. It is the
  same name, address and e-mail, not a separate body.
- Target 20316475: identifier NULL, 3 mentions, all from TED 2022 notice 22851889. That is GSI's X-ray unit tender,
  with ORG-1 Düsseldorf and ORG-2/3 Bismarckstraße 85 Duisburg, the same e-mail and phone. So the target is GSI.
- The key is city-flagged because its planned mentions show {duisburg, munchen}. GSI is one GmbH with Niederlassungen
  (SLV Duisburg, SLV München), so München rows are the same legal entity.
- Two side notes:
  - This group is "Lot-AddInfo+Lot-ReviewOrg", one of the 66 groups with that role set in `roles.by_role_set`. A7's
    list does not include it, because neither role is in the buyer family. Here it is one body, but those 66 groups
    are not screened by A7.
  - The buyer's own ORG-0001 binds through `13251` and stays apart from the GSI fallback row. That split comes from
    300 §10 and is outside 530.

**26: 22834378 "Geschäftsstelle der Vergabekammern beim Ministerium für Wirtschaft, Infrastruktur, Tourismus und Arbeit Mecklenburg-Vorpommern" (review_or_info), target 23021560: R.**
- TED 00675938-2024 (notice 24363093). The buyer is Universitätsmedizin Greifswald.
- The review chamber office is printed three times under three UUIDs:
  - ORG-0003 ReviewInfo;
  - ORG-0004 Lot-ReviewOrg, `28e1f9fc…` (this row);
  - ORG-0005 Mediator.
- All three give Johannes-Stelling-Str. 14, 19053 Schwerin, vergabekammer@wm.mv-regierung.de.
- Target 23021560: identifier NULL, 3 mentions, city screen clean.
- One body. The same chamber also appears under other ministry-era names (items 15 and 17). Those are different keys,
  so the fold under-merges and never mis-merges.

**27: 22957396 "Zentrale Beschaffungsstelle des Landes Rheinland-Pfalz (ZBL) im Landesbetrieb Mobilität Rheinland-Pfalz" (review_or_info), target 8335910: R.**
- TED 00065133-2025 (notice 24520244), a framework for oils and lubricants. Three sections each carry their own UUID:
  - ORG-0001 Procedure-Buyer;
  - ORG-0002 Lot-AddInfo, `6691143d…` (this row);
  - ORG-0006 TenderReceipt.
- All three give Friedrich-Ebert-Ring 14-20, 56068 Koblenz, zbl@lbm.rlp.de.
- ReviewInfo/Mediator (ADD Trier) and ReviewOrg (VK RLP Mainz) are other names.
- Target 8335910: identifier NULL, 655 mentions, city screen clean. It is the one central procurement office of RLP.

**28: 22975386 "Vergabekammer Rheinland · c/o Bezirksregierung Köln" (review_or_info), target 12774348: R.**
- TED 00075529-2025 (notice 24542995), a contract award notice for tax advice for Bergische Universität Wuppertal.
- The group:
  - ORG-0003 Lot-ReviewInfo, `791caa3e…` (this row);
  - ORG-0004 Lot-ReviewOrg, `45fa652d…`.
- Both give Zeughausstraße 2-10, 50667 Köln, vkrheinland@bezreg-koeln.nrw.de.
- Target 12774348: identifier NULL, 495 mentions, city screen clean.
- The "c/o Bezirksregierung Köln" suffix pins the Köln chamber. This is a different key from 7615 "Vergabekammer
  Rheinland", so it stays a separate row of the same body, as today. Nothing foreign is folded in.

**29: 22795050 "Dresdner Verkehrsbetriebe AG" (review_or_info), target 1230216: R.**
- TED 00661682-2024 (notice 24311959), Lübecker Straße track renewal. Three sections each carry their own UUID:
  - ORG-0001 Procedure-Buyer;
  - ORG-0002 Lot-AddInfo, `35d31537…` (this row);
  - ORG-0004 TenderReceipt.
- All three give Trachenberger Straße 40, 01129 Dresden, peter.melde@dvbag.de.
- The ReviewOrg is the 1. Vergabekammer Sachsen in Leipzig, a different name.
- Target 1230216: identifier NULL, 1,079 mentions, city screen clean. It is one AG.
