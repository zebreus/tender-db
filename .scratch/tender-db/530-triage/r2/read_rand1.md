# 530 R2: random planned rows, items 0-14 (read_rand1)

Source: `guid_twin_plan_2659.json` `body.r2_random[0..14]` (salt 530002). All 15 rows are in the buyer stratum. Each one is DE,
2 mentions, 1 Tender, planned `whole`, not in KEEP, not stable, and `twin_after=false`.

Question: would rehoming all of the row's mentions to its target, the identifier-NULL `(name_norm, DE)` row, be right?
- **R**: same body, and the GUID row is a per-notice split.
- **N**: cannot tell.
- **W**: the target name covers more than one body, or the target would take another body's mentions, or the GUID is a
  real stable key.

**Evidence read for each row.**
- `/v1/organizations/{row}` and `/v1/organizations/{target}`. Every row still has its GUID and 2 mentions. Every target has
  identifier NULL, is provisional, is DE, and has the same name as the row.
- Both copies of the row's notice (TED and DÖE) through `/v1/notices/{id}/content`: the ORG sections' name, BT-501, contact,
  street, postcode, city and email, plus the role references (OPT-300/301 and the DE1 party references).
- One notice of the target's existing mentions where the report lists them. That covers the 5 targets in `r2_top_targets`.
- One notice of a sibling planned row going to the same target, for 6 of the targets that have no listed mentions.
- The report's city screen. None of these targets is city-flagged, so each target's newest-50 city set is empty or holds
  only the planned city.
- None of the 15 groups is in `roles.incoherent` (A7).

**GETs.** 68 public GETs, all 200 and none 408, at least 2 s apart. They are logged in `api_requests.log` (tag
`read_rand1`), and the bodies are under `bodies/`.

## Tally

**R 15, N 0, W 0.** One row (item 14) has no identifier-NULL target today, so its R is conditional on Build 2's mint path
(see that item).

| # | org_id | name | target | verdict |
|---|---|---|---|---|
| 0 | 22824042 | Bau- und Liegenschaftsbetrieb NRW Bielefeld | 1203308 (796) | R |
| 1 | 22842358 | Norddeutscher Rundfunk | 1202351 (540) | R |
| 2 | 22816858 | Stadt Kyritz | 7175314 (137) | R |
| 3 | 22891825 | Bau- und Liegenschaftsbetrieb NRW Dortmund | 8119131 (503) | R |
| 4 | 22832955 | Wasserverband Eifel-Rur - Zentrale Vergabestelle - | 20038121 (72) | R |
| 5 | 22838236 | Wasserverband Eifel-Rur - Zentrale Vergabestelle - | 20038121 (72) | R |
| 6 | 22914331 | Kolpingstadt Kerpen | 2383569 (97) | R |
| 7 | 22898749 | Bau- und Liegenschaftsbetrieb NRW Münster | 1222044 (261) | R |
| 8 | 22807165 | Bau- und Liegenschaftsbetrieb NRW Zentrale | 7195752 (1,101) | R |
| 9 | 22808070 | Landkreis Northeim | 4563 (1,028) | R |
| 10 | 22886247 | Universität Stuttgart | 1262231 (420) | R |
| 11 | 22860164 | Stadt Salzgitter | 1190366 (1,003) | R |
| 12 | 22914565 | Bundesstadt Bonn - Referat Vergabedienste | 15234277 (376) | R |
| 13 | 22898487 | Universität Münster | 13342269 (36) | R |
| 14 | 23053387 | Stadt Spremberg - Vergabestelle | none (missing key, 6 rows / 12 mentions) | R (conditional) |

## Items

**0. 22824042, BLB NRW Bielefeld → 1203308. R.**
- **Row.** GUID 9BD001B9… sits on ORG-0001 (Procedure-Buyer) of the BLB contract notice for the Universität Bielefeld main
  building, TGA ELT (TED 00706835-2024, notice 24349929; DÖE copy 26509277; same GUID in both, 2 mentions).
- **Twin.** In the same notice, ORG-0002 (Lot-AddInfo) and ORG-0005 (Lot-TenderReceipt) print the same name under fresh
  GUIDs 22792b7d… and 3006aa57…. All three sections share the contact "Einkauf und Vertragsmanagement", August-Bebel-Straße 91,
  33602 Bielefeld, and BLBVergabe@blb.nrw.de.
- **Per-notice minting.** A sibling planned BLB Bielefeld notice for the same building project (24311122, Bau Ausbau) mints
  three other GUIDs (ecdca058…, 710ac05d…, ac211d34…) at the same address.
- **Target.** 1203308 "Bau- und Liegenschaftsbetrieb NRW Bielefeld": identifier NULL, provisional, 796 mentions. The name
  is the Bielefeld branch of the state building agency, one body. City screen clean (bielefeld).

**1. 22842358, Norddeutscher Rundfunk → 1202351. R.**
- **Row.** GUID 2726CBAB… sits on ORG-0001 (Procedure-Buyer) of the NDR award "Wartung und Instandhaltung der Kälteerzeuger"
  (TED 00726570-2024, notice 24373729; DÖE copy 26515848).
- **Twin.** ORG-0002 (Lot-AddInfo) prints "Norddeutscher Rundfunk" under 94ad5587…. Both sections give Rothenbaumchaussee
  132-134, 20149 Hamburg, and e-vergaben@ndr.de.
- **Per-notice minting.** Sibling planned notice 24254106 (NDR, project management for Deutschlandradio's Köln/Berlin
  buildings) mints 3 other GUIDs for NDR at the same address and email.
- **Target.** 1202351 "Norddeutscher Rundfunk": identifier NULL, provisional, 540 mentions. NDR is a single public
  broadcaster, so the name is one legal entity even where a regional Landesfunkhaus prints it. City screen clean (hamburg).

**2. 22816858, Stadt Kyritz → 7175314. R.**
- **Row.** GUID 4AC42EAE… sits on ORG-0001 (Procedure-Buyer) of "Los N007 Tischlerarbeiten - Kultur|Kloster|Kyritz" (TED
  00678169-2024, notice 24340998; DÖE copy 26498110).
- **Twin.** ORG-0002 (Lot-AddInfo) prints the same name under 955fff2b…. Both give Vergabestelle, Marktplatz 1, 16866 Kyritz,
  and vergabe@kyritz.de.
- **Per-notice minting.** Sibling planned notice 24265979 (Los K007, same project) mints 807ec76f… and 949ce023… for the same
  office.
- **Target.** 7175314 "Stadt Kyritz": identifier NULL, provisional, 137 mentions. Kyritz is one town, and no other body is
  called that. City screen clean (kyritz).

**3. 22891825, BLB NRW Dortmund → 8119131. R.**
- **Row.** GUID 85F31433… sits on ORG-0001 (Procedure-Buyer) of the BLB Dortmund award for Landesstelle Unna-Massen
  open-space planning (TED 00790366-2024, notice 24436295; DÖE copy 26538973).
- **Twin.** ORG-0002 (Lot-AddInfo) prints the same name under ff533a5e…. Both give Einkauf und Vertragsmanagement,
  Emil-Figge-Str. 91, 44227 Dortmund, and do.einkauf@blb.nrw.de.
- **Not grouped.** The notice's other BLB section, "Bau- und Liegenschaftsbetrieb NRW - Justiziariat" (ReviewInfo, Düsseldorf),
  has a different name_norm and is not in the group.
- **Per-notice minting.** Sibling planned notice 24333044 (JVK Fröndenberg) mints 23acd387… and ded5f963… at the same address.
- **Target.** 8119131 "Bau- und Liegenschaftsbetrieb NRW Dortmund": identifier NULL, provisional, 503 mentions. The name is
  the Dortmund branch, one body. City screen clean (dortmund).

**4. 22832955, Wasserverband Eifel-Rur - Zentrale Vergabestelle - → 20038121. R.**
- **Row.** GUID 51BE2048… sits on ORG-0001 (Procedure-Buyer) of "TS - Gewässerunterhaltung der Gewässer sonstiger Ordnung"
  (TED 00719820-2024, notice 24361404; DÖE copy 26514198).
- **Twin.** ORG-0002 (Lot-AddInfo, d30a3c7d…) and ORG-0005 (Lot-TenderReceipt, a045d926…) print the identical name. All
  give Eisenbahnstraße 5, 52353 Düren, and ZentraleVergabe@wver.de.
- **Not grouped.** ORG-0003 "Zentrale Vergabestelle des Wasserverband Eifel-Rur (WVER)" (ReviewInfo) is the same office under
  another spelling. It stays outside the group, which is an under-merge and not a wrong one.
- **Name guard.** The name is not the bare 530 unsafe entry `Zentrale Vergabestelle`. It names the water board.
- **Per-notice minting.** Sibling planned notice 24296267 mints 087682fd… and 067a444f… for the same office.
- **Target.** 20038121, same name: identifier NULL, provisional, 72 mentions. One body (WVER's central procurement office).
  City screen clean (duren).

**5. 22838236, Wasserverband Eifel-Rur - Zentrale Vergabestelle - → 20038121. R.**
- **Row.** GUID CC363AF1… sits on ORG-0001 (Procedure-Buyer) of "ST - Herstellung einer Hochwasserschutzmauer an der Wurm in
  Geilenkirchen" (TED 00667963-2024, notice 24367908; DÖE copy 26494828).
- **Twin.** ORG-0002 (Lot-AddInfo, a8ba7282…) and ORG-0005 (Lot-TenderReceipt, 6e00e56b…) print the identical name at
  Eisenbahnstraße 5, Düren, with ZentraleVergabe@wver.de. The "Zentrale Vergabestelle des … (WVER)" ReviewInfo section again
  stays outside the group.
- **Per-notice minting.** This row and item 4 are two notices of the same office with two different buyer GUIDs.
- **Target.** Same target and same reasoning as item 4.

**6. 22914331, Kolpingstadt Kerpen → 2383569. R.**
- **Row.** GUID 2032D74B… sits on ORG-0001 (Procedure-Buyer) of "Generalplanerleistungen Neubau & Sanierung der Feuer- und
  Rettungswache in Kerpen" (TED 00738315-2024, notice 24466427; DÖE copy 26519425).
- **Twin.** ORG-0002 (Lot-AddInfo) prints the same name under 9a11b6ee…. Both give "Abtl. 20.4 - Zentrale Vergabestelle",
  Jahnplatz 1, 50171 Kerpen, and vergabestelle@stadt-kerpen.de.
- **Other groups in the notice.** Vergabekammer Rheinland is printed twice (ReviewInfo and ReviewOrg), and Rhein-Erft-Kreis is
  the mediator. Both are separate names.
- **Target.** 2383569 "Kolpingstadt Kerpen": identifier NULL, provisional, 97 mentions. Kerpen is one town, and the
  "Kolpingstadt" title belongs to it alone. City screen clean (kerpen).

**7. 22898749, BLB NRW Münster → 1222044. R.**
- **Row.** GUID BA3EFE1E… sits on ORG-0001 (Procedure-Buyer) of "BLB NRW Münster / Universität Münster - Forschungsbau CMM /
  Estrich- und Bodenbelagsarbeiten" (TED 00738975-2024, notice 24445522; DÖE copy 26520945).
- **Twin.** ORG-0002 (Lot-AddInfo) prints the same name under c76759e4…. Both give Einkauf und Vertragsmanagement,
  Hohenzollernring 80, 48145 Münster, and BLBVergabe@blb.nrw.de.
- **Target.** 1222044 "Bau- und Liegenschaftsbetrieb NRW Münster": identifier NULL, provisional, 261 mentions.
  - Its listed existing mention 44550117 (TED 00353597-2025, May 2025, MRV Bedburg-Hau) prints BLB NRW Münster with no
    BT-501, at the same Hohenzollernring 80 address and email, as buyer, AddInfo and TenderReceipt. That notice's Justiziariat
    section is also at Hohenzollernring 80.
  - The target already holds this body's identifier-less mentions. City set [munster] equals the planned city.

**8. 22807165, BLB NRW Zentrale → 7195752. R.**
- **Row.** GUID 313C4FC3… sits on ORG-0001 (Procedure-Buyer) of "BLB /Z /NL Düsseldorf / Unterhaltsreinigung Los 1.26" (TED
  00662544-2024, notice 24329406; DÖE copy 26493537).
- **Twin.** ORG-0002 (Lot-AddInfo) prints the same name under 51f51c6f…. Both give "Servicecenter Facheinkauf", Mercedesstraße
  12, 40470 Düsseldorf, and BLBVergabe@blb.nrw.de.
- **Target.** 7195752 "Bau- und Liegenschaftsbetrieb NRW Zentrale": identifier NULL, provisional, 1,101 mentions, and in
  `r2_top_targets` (+392 mentions from 196 rows, planned city düsseldorf only).
  - Its listed existing mention 3920878 is a 2009 award (321249-2009) by "BAU- UND LIEGENSCHAFTSBETRIEB NRW ZENTRALE",
    DUESSELDORF, www.blb.nrw.de. The target city set is empty because its newest mentions are pre-eForms text.
  - The target is BLB's headquarters, one body.

**9. 22808070, Landkreis Northeim → 4563. R.**
- **Row.** GUID C4DE6801… sits on ORG-0001 (Procedure-Buyer) of "Landkreis Northeim - Brandschutz-/Asbestsanierung Kreishaus -
  Abbrucharbeiten" (TED 00662812-2024, notice 24330564; DÖE copy 26493381).
- **Twin.** ORG-0002 (Lot-AddInfo, b6146035…) and ORG-0006 (Lot-TenderReceipt, 4d222a11…) print the same name. All give
  Zentrale Vergabestelle, Medenheimer Str. 6/8, 37154 Northeim, and vergabestelle@landkreis-northeim.de.
- **Sibling notice 24312760.** The buyer there is Stadt Bad Gandersheim, and Landkreis Northeim appears in AddInfo and
  TenderReceipt under 8f3a91d0… and 82ce51bf…. This shows the district's procurement office acting for a municipality. Those
  mentions are still the district, so the target gets service roles of the same body, not another body's mentions.
- **Target.** 4563 "Landkreis Northeim": identifier NULL, provisional, 1,028 mentions. One district. City screen clean
  (northeim).

**10. 22886247, Universität Stuttgart → 1262231. R.**
- **Row.** GUID E5C71442… sits on ORG-0001 (Procedure-Buyer; Keplerstraße 7, 70174 Stuttgart) of "VMware-Lizenzen" (TED
  00756131-2024, notice 24429494; DÖE copy 26526736).
- **Twin.** ORG-0002 (Lot-AddInfo, d643c5f0…) and ORG-0005 (Lot-TenderReceipt, 195a651b…) print "Universität Stuttgart" with
  contact "Zentrale Beschaffungsstelle", Breitscheidstraße 2 C, 70174 Stuttgart. All three use the email
  beschaffung@verwaltung.uni-stuttgart.de.
  - The two street addresses are the university's administration and its procurement office: one legal entity, one city.
- **Not grouped.** ORG-0003 "Universität Stuttgart, Zentrale Beschaffungsstelle" (ReviewInfo) has another name_norm and stays
  out.
- **Target.** 1262231 "Universität Stuttgart": identifier NULL, provisional, 420 mentions, in `r2_top_targets` (+254 from 127
  rows). Its listed existing mention 4062571 is a 2010 award by "UNIVERSITÄT STUTTGART", STUTTGART, www.uni-stuttgart.de.
  City set [stuttgart] equals the planned city.

**11. 22860164, Stadt Salzgitter → 1190366. R.**
- **Row.** GUID BF4B868B… sits on ORG-0001 (Procedure-Buyer) of "Trockenbauarbeiten" (TED 00681593-2024, notice 24396555; DÖE
  copy 26499570).
- **Twin.** ORG-0002 (Lot-AddInfo) prints the same name under 7d16acc6…. Both give Joachim-Campe-Straße 6-8, 38226 Salzgitter,
  and submission@stadt.salzgitter.de.
- **Target.** 1190366 "Stadt Salzgitter": identifier NULL, provisional, 1,003 mentions, in `r2_top_targets` (+182 from 91 rows).
  - Its newest listed mention 47414618 is a DÖE notice of 2026-10-16 (Grundschule Waldschule, Salzgitter-Bad). It prints
    Stadt Salzgitter with no GUID, at the same Joachim-Campe-Straße 6-8 address and email, as buyer, AddInfo and
    TenderReceipt.
  - The platform now publishes this body without a per-section GUID, and those mentions already land on the target. City
    set [salzgitter] equals the planned city.

**12. 22914565, Bundesstadt Bonn - Referat Vergabedienste → 15234277. R.**
- **Row.** GUID 79210831… sits on ORG-0001 (Procedure-Buyer) of the Bonn award for a managed SOC/SIEM (TED 00002036-2025,
  notice 24466739; DÖE copy 26541396).
- **Twin.** ORG-0002 (Lot-AddInfo) prints the same name under 7efcb058…. Both give Bertha-von-Suttner-Platz 2-4, 53111 Bonn,
  and referatvergabedienste@bonn.de.
- **Target.** 15234277, same name: identifier NULL, provisional, 376 mentions, in `r2_top_targets` (+492 from 246 rows,
  planned city bonn only).
  - Its listed existing mention 23528496 is a 2023 TED-XML F14 (2023/S 208-654641) by "Bundesstadt Bonn - Referat
    Vergabedienste" at the same address, email and www.bonn.de.
  - The target is the city's central procurement office, one body. It buys for many city departments, but under its own
    name.

**13. 22898487, Universität Münster → 13342269. R.**
- **Row.** GUID 18F9A00A… sits on ORG-0001 (Procedure-Buyer and Lot-AddInfo) of "Rahmenvertrag: Catering Dienstleistungen an
  der Universität Münster" (TED 00740976-2024, notice 24445141; DÖE copy 26520771).
- **Twin.** ORG-0002 (Lot-ReviewInfo) prints "Universität Münster" under c5763aca…. Both give "Dez. 5.6 - Zentraler Einkauf",
  Schlossplatz 2, 48149 Münster, and zentraler.einkauf@uni-muenster.de.
- **Roles.** Buyer + ReviewInfo is the buyer naming itself as the contact for review information. It is not a Lot-ReviewOrg
  (the review body here is Vergabekammer Westfalen, ORG-0003), so it is not an A7 case.
- **Target.** 13342269 "Universität Münster": identifier NULL, provisional, 36 mentions. This is the university's post-2023
  name, and only one body carries it. City screen clean (munster).

**14. 23053387, Stadt Spremberg - Vergabestelle → no target (missing key). R, conditional.**
- **Row.** GUID 0DBF92F6… sits on ORG-0001 (Procedure-Buyer) of "Bildungscampus BOS - archäologische Baubegleitung" (TED
  00163978-2025, notice 24655256; DÖE copy 26598820).
- **Twin.** ORG-0002 (Lot-AddInfo, a1a1ce96…) and ORG-0006 (Lot-TenderReceipt, 9280b592…) print the same name. All give
  Zentrale Vergabestelle, Am Markt 1, 03130 Spremberg, and vergabestelle@stadt-spremberg.de.
  - The notice's Vergabekammer Land Brandenburg triple is a separate group.
- **No target today.** The key (stadt spremberg - vergabestelle, DE) has no identifier-NULL row. `missing_target_samples` lists
  it with 6 planned rows / 12 mentions and one planned city, spremberg.
- **Verdict.** If Build 2 lets the 234 path mint the fallback row, that new row would hold only these 6 rows of the one
  Spremberg procurement office: R. If Build 2 keeps missing-target rows instead, nothing moves. Either way the fold is not W.
  The verdict rests on the notice and the key's planned-city set, not on an existing target.

## Notes for the R2 tally

- All 15 are the DÖE+TED dashed-lower platform shape. The TED and DÖE copies always carry the same GUID per section, and that
  GUID never recurs in the same body's sibling notices. So the GUID identifies one ORG section of one notice, not the body.
- No group mixes in a review body or a winner. The twins are buyer + AddInfo (+ TenderReceipt, + ReviewInfo once), all at one
  address and email.
- Several notices print one body's procurement office under a second spelling (WVER "Zentrale Vergabestelle des …",
  "Universität Stuttgart, Zentrale Beschaffungsstelle", "BLB NRW - Justiziariat"). Those sections fall outside the group and
  keep their GUID rows. That is residual splitting under (c), not a precision issue.
