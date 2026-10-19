# 530 R2c top-up: independent planned rows, picks 10-19 (read_c2)

Source: `530-triage/r2c_picks.json` (salt 541337; no pick within 5 ids of the 120 rows R2/R2b read), picks[10..19]; the
stored mentions in `r2c/q1.out.json` (two per row: the TED eForms copy and the DÖE DE1 copy, same name and raw BT-501 in
both); plan `guid_twin_plan_2660.json` (Build 1b, deployed 8e9222c, dry job 2660). All ten rows are in `planned_row_ids`,
DE, 2 mentions, GUID still in place (`/v1/organizations/{row}`: identifier_kind national, not provisional).

Question per pick: would rehoming both mentions to the identifier-NULL `(org_name_norm(name), DE)` row be
- **R**: same body (the GUID row is a per-notice split);
- **N**: cannot tell;
- **W**: the target name denotes more than one body, the target would receive another body's mentions, the section is a
  review-org slot printing another body's details, or the GUID is a real stable key.

**How the target was found.** `body.targets` in this file is a count (554), not a list. Targets came from:
- the plan's own lists. `r2_top_targets` gave 7615, 2443792 and 1168329, and `r2_random` gave 17012302 and 1203308, each
  matched by exact name. The truncated `planned_mention_keys` lists the TED mention of pick 17 itself (22782551 → 1460530)
  and of pick 18 itself (22753860 → 18261510);
- for the other three, `/v1/organizations?name_prefix=…&country=DE`. That gave the only identifier-NULL row with the exact
  name: Stadt Ahlen 4307071, Samtgemeinde Bothel 8437842, and VK Rheinland bei der Bezirksregierung Köln 7473. For 7473 I
  also read page 2, which holds 100 more rows of the exact name, all GUID-identified.

Every target was re-read by `/v1/organizations/{id}`: identifier NULL, provisional, DE, exactly the row's name. No target
is missing, so none depends on Build 2 minting a row.

**Evidence read for each pick.**
- Both copies of its notice through `/v1/notices/{id}/content`. In the TED copy, every ORG section's name, BT-501, street,
  postcode, city, e-mail, phone and NUTS. In the DÖE copy, the same contact block, plus the name and CompanyID from the
  `ND-PartyName` / `ND-PartyLegalEntity` children.
- The role references: OPT-300/301 in the TED copy, and the DE1 party refs in the DÖE copy.
- The twin group (same name, other GUID, same notice).
- The planned neighbours (`planned_row_ids` within ±6).
- The plan's city screen. `city.flagged` is complete (36 of 36), and of these ten targets only 7615 is on it, with
  ["50667", "koln"]: Köln's postcode typed into BT-513 (R2 read_top1 item 0). So every other target's newest-50 city set is
  empty or exactly the planned city.

**Common picture.** The pick's GUID and its twins' GUIDs are identical in the TED and DÖE copies and fresh in each notice.
No group mixes a buyer-side role with Lot-ReviewOrg:
- the buyer-side groups are Buyer + AddInfo (+ TenderReceipt);
- the review groups are ReviewInfo + ReviewOrg.

In all ten notices the ReviewOrg is a real chamber printed under its own name and address. Neither R2 W shape occurs:
- no umbrella name collects different chambers;
- no review slot prints another body's details.

**Independence.**
- **Notices:** no pick shares a notice with any row R2 or R2b read (all 20 notices were checked against
  `r2/read_*.md`, `r2b/read_*.md`, the result JSONs and both `bodies/` directories). No pick shares a notice with another
  r2c pick.
- **Targets:** five targets were already read at target level. 7615 (R2 read_top1 item 0), 2443792 (R2 read_top2; R2b
  read_rand1 item 9), 17012302 (R2b read_rand1 item 11), 1168329 (R2 read_top1 item 11) and 1203308 (R2 read_rand1 item
  0; R2b read_rand3 item 33). These picks were still read fresh from their own notices. The other five targets (4307071,
  8437842, 1460530, 18261510, 7473) are new to the R2 reads.
- **Correlated pair:** picks 17 and 18 have the same buyer, Neubrandenburger Wohnungsgesellschaft. Their notices,
  rows and targets differ, but each notice also holds rows planned to the other pick's target.

## Tally

**R 10, N 0, W 0.**

| # | org_id | name | section / role (twin) | target (mentions) | verdict |
|---|---|---|---|---|---|
| 10 | 22917421 | Vergabekammer Rheinland | ORG-0002 Lot-ReviewInfo (ReviewOrg) | 7615 (29,617) | R |
| 11 | 22923417 | Bau- und Liegenschaftsbetrieb NRW Aachen | ORG-0002 Lot-AddInfo (Buyer) | 2443792 (573) | R |
| 12 | 22840850 | Stadt Ahlen | ORG-0001 Procedure-Buyer (AddInfo, TenderReceipt) | 4307071 (177) | R |
| 13 | 22845049 | Samtgemeinde Bothel | ORG-0003 Lot-AddInfo (Buyer) | 8437842 (9) | R |
| 14 | 22853395 | SBO Servicebetriebe Oberhausen - Eigenbetrieb der Stadt Oberhausen | ORG-0002 Lot-AddInfo (Buyer) | 17012302 (80) | R |
| 15 | 22997423 | Vergabekammer des Landes Brandenburg | ORG-0004 Lot-ReviewOrg (ReviewInfo) | 1168329 (10,598) | R |
| 16 | 22922428 | Bau- und Liegenschaftsbetrieb NRW Bielefeld | ORG-0001 Procedure-Buyer (AddInfo) | 1203308 (796) | R |
| 17 | 22782551 | Neubrandenburger Wohnungsgesellschaft mbH | ORG-0001 Procedure-Buyer (AddInfo) | 1460530 (186) | R |
| 18 | 22753860 | Vergabekammern des Landes Mecklenburg-Vorpommern bei dem Ministerium für Wirtschaft, Infrastruktur, Tourismus und Arbeit | ORG-0003 Lot-ReviewInfo (ReviewOrg) | 18261510 (620) | R |
| 19 | 22938395 | Vergabekammer Rheinland bei der Bezirksregierung Köln | ORG-0003 Lot-ReviewOrg (ReviewInfo) | 7473 (6,101) | R |

## Items

**10. 22917421, Vergabekammer Rheinland → 7615 (29,617). R.** Norddeutscher Rundfunk (Hamburg) awarded "Stellung des
Brandschutzbeauftragten für die Funkhäuser Köln und Berlin von Deutschlandradio" to Meyer Brandschutzplanung, Wolfsburg (TED
24470203, DÖE 26549290). GUID `dbd007b6…` sits on ORG-0002, Lot-ReviewInfo on both lots; its twin is ORG-0003, Lot-ReviewOrg,
`e006bb2e…` (planned neighbour 22917422). Both print Zeughausstraße 2-10, 50667 Köln, VKRheinland@bezreg-koeln.nrw.de, fax +49
221147-2889 and DEA23, and the DÖE copy agrees field for field. The group is ReviewInfo + ReviewOrg with no buyer-side role, so
A7 rightly does not bind; NDR is printed under its own name. The target 7615 is the one chamber for the Rheinland districts at
Bezirksregierung Köln: R2 read it R (read_top1 item 0), and its newest notices 47408004 / 47404257 (Handwerkskammer Aachen)
print the same Köln chamber. Its flagged second "city" is the postcode 50667. "Vergabekammer Rheinland" is one of the
single-chamber names Build 1b pinned as safe.

**11. 22923417, Bau- und Liegenschaftsbetrieb NRW Aachen → 2443792 (573). R.** BLB NRW Aachen awarded "AC/Eschweiler/AC AG
Eschweiler Erweiterung/LV03 Dachabdichtung-Gründach" to Hamacher Bedachungen GmbH, Mönchengladbach (TED 24477699, DÖE
26558255). GUID `7bbcdca1…` is ORG-0002, Lot-AddInfo; its twin is ORG-0001, Procedure-Buyer, `1a416331…` (planned neighbour
22923416). Both print "Einkauf und Vertragsmanagement", Mies-van-der-Rohe-Straße 10, 52074 Aachen, AC.Einkauf@blb.nrw.de and
DEA2D, in both copies. Buyer + AddInfo is coherent. The review body is a separate triple, "Vergabekammer Rheinland, c/o
Bezirksregierung Köln·" (ReviewInfo / ReviewOrg / Mediator, 22923418-20); it is its own group and not this fold. The target
2443792 is the Aachen branch of the state building agency. R2 read it R (read_top2), and so did R2b (read_rand1 item 9, another
notice). Its newest-50 city set is empty (old text-era notices), and the planned city is {aachen}.

**12. 22840850, Stadt Ahlen → 4307071 (177). R.** The notice is "Sanierung einer ehemaligen Hauptschule,
Metallbauarbeiten" (TED 24371501, DÖE 26502917). GUID `e959faf2…` is ORG-0001, Procedure-Buyer. Its twins are ORG-0002,
Lot-AddInfo, `0a01ab52…` (22840851) and ORG-0004, Lot-TenderReceipt, `153ea571…` (22840853). All three print "Zentrale
Vergabestelle", Westenmauer 10, 59227 Ahlen, zentrale-vergabestelle@stadt.ahlen.de and DEA38, in both copies. The ReviewOrg is
Vergabekammer Westfalen, Albrecht-Thaer-Straße 9, Münster, under its own name. Buyer + AddInfo + TenderReceipt is coherent.
The target 4307071 is the only identifier-NULL "Stadt Ahlen" row. Beside it, the name_prefix page shows 18 other GUID rows
of the exact name, each with 2 mentions (the per-notice split this unit dissolves), three short numeric national ids (1-3
mentions), a Leitweg-ID row 22125264 (668) and a VAT row 22962208 (16). None of these identified rows is a target (300 §10),
so what stays split is between identified rows, not a homonym. There is one Stadt Ahlen in Germany (Kreis Warendorf, NRW),
and the target is not city-flagged.

**13. 22845049, Samtgemeinde Bothel → 8437842 (9). R.** The notice is the award "FW Westerwalsede (SG Bothel) - Lieferung
TSF-W" (a fire engine) to Wiss GmbH+Co. KG, Herbolzheim (TED 24376805, DÖE 26510749). GUID `4dd4b515…` is ORG-0003,
Lot-AddInfo; its twin is ORG-0001, Procedure-Buyer, `a9abed1b…` (row 22845047). Both print Horstweg 17, 27386 Bothel,
samtgemeinde@bothel.de, +49 42669831500 and DE937, in both copies. The other names have their own roles: BBS
Gefahrenabwehrplanung, Hamburg (SProvider), and VK Niedersachsen, Lüneburg (ReviewOrg). Buyer + AddInfo is coherent. The target
8437842 is the only identifier-NULL row of the name (9 mentions, not city-flagged), and Samtgemeinde Bothel (Landkreis
Rotenburg (Wümme)) is a unique name. Aside, out of scope: row 22695299 carries identifier `T4942669831500`, which is the
Samtgemeinde's phone number taken as a national key (6 mentions). It is not a target and stays apart.

**14. 22853395, SBO Servicebetriebe Oberhausen - Eigenbetrieb der Stadt Oberhausen → 17012302 (80). R.** The notice is the
award "Theater Oberhausen, Objektplanung" to Schamp & Partner, Dortmund (TED 24387352, DÖE 26504764). GUID `a582b9f7…` is
ORG-0002, Lot-AddInfo; its twin is ORG-0001, Procedure-Buyer, `a010e3fe…` (22853394). Both print Bahnhofstraße 66, 46145
Oberhausen, vergabe-bm@sbo.oberhausen.de and DEA17, in both copies. The ReviewOrg is a single Vergabekammer Rheinland section
(Köln). Buyer + AddInfo is coherent. The target 17012302 is the city's operating company (Eigenbetrieb), and R2b read it R
(read_rand1 item 11). Its sibling planned notice 24331367, already on disk from R2b, prints the same Oberhausen block, plus a
"… / Submissionsstelle" section under another name. Not city-flagged.

**15. 22997423, Vergabekammer des Landes Brandenburg → 1168329 (10,598). R.** The notice is the Landesbetrieb Forst
Brandenburg (Eberswalde) framework "Persönliche Schutzausrüstungen Rahmenvertrag", 9 lots (TED 24573186, DÖE 26570320). GUID
`72b3d21b…` is ORG-0004, Lot-ReviewOrg; its twin is ORG-0003, Lot-ReviewInfo, `92749624…` (22997422). Both print
Heinrich-Mann-Allee 107, 14473 Potsdam, vergabekammer@MWAE.brandenburg.de, 03318661719 and DE404, in both copies. ReviewInfo +
ReviewOrg is coherent. The buyer is its own Buyer/AddInfo/TenderReceipt triple (22997420/21/25), and the Mediator is the MLUK
under its own name. The target 1168329 is one review chamber. R2 read it R (read_top1 item 11), and its newest mention 47414536
(Herzberger WAZV; body reused from `r2/bodies`) prints the same Potsdam chamber. City set {potsdam}.

**16. 22922428, Bau- und Liegenschaftsbetrieb NRW Bielefeld → 1203308 (796). R.** The notice is "BLB NRW BI / WE 1008 /
Finanzamt Bielefeld-Innenstadt / Objektplanung Freianlagen", awarded to brandenfels landscape + environment, Münster (TED
24476563, DÖE 26558176). GUID `73c91731…` is ORG-0001, Procedure-Buyer; its twin is ORG-0002, Lot-AddInfo, `6fb11dc0…`
(22922429). Both print "Einkauf und Vertragsmanagement", August-Bebel-Straße 91, 33602 Bielefeld, BLBVergabe@blb.nrw.de and
DEA41, in both copies. The ReviewOrg is VK Westfalen, Münster, under its own name. Buyer + AddInfo is coherent. The target
1203308 is the Bielefeld branch. R2 read it R (read_rand1 item 0), and so did R2b (read_rand3 item 33). Not city-flagged.

**17. 22782551, Neubrandenburger Wohnungsgesellschaft mbH → 1460530 (186). R.** The notice is "Leasing Kraftfahrzeuge", 3
lots, awarded to Autohaus Piahowiak and Schröder Automobil, both Neubrandenburg (TED 24293938, DÖE 26485121). GUID
`9484e607…` is ORG-0001, Procedure-Buyer; its twin is ORG-0002, Lot-AddInfo, `01bbd74d…` (22782552). Both print Heidenstr.
6, 17034 Neubrandenburg, ausschreibung@neuwoges.de and DE80J, in both copies. The review side is a separate triple of the MV
chambers in Schwerin (ReviewInfo / ReviewOrg / Mediator, 22782553-55 → 18261510). Buyer + AddInfo is coherent. The target
1460530 is the only identifier-NULL row of the exact name. The name_prefix page shows 45 GUID rows of the name, each with 2
mentions (the platform's per-notice split), and a VAT row 22569970 (76), which is not a target. The sibling planned notice
24253163 ("Einsteinstraße 21-23 Los 1", rows 22751962/63/66 → 1460530) prints only the company's own address in its Buyer,
AddInfo and TenderReceipt slots, so the target receives no other body's mentions. Not city-flagged.

**18. 22753860, Vergabekammern des Landes Mecklenburg-Vorpommern bei dem Ministerium für Wirtschaft, Infrastruktur,
Tourismus und Arbeit → 18261510 (620). R.** The notice is the Neubrandenburger Wohnungsgesellschaft notice "Einsteinstraße
21-23 Los 5 - Dachdecker" (TED 24255388, DÖE 26482768). GUID `2dcf9761…` is ORG-0003, Lot-ReviewInfo; its twin is ORG-0004,
Lot-ReviewOrg, `22423aba…` (22753861). Both print Johannes-Stelling-Str. 14, 19053 Schwerin, vergabekammer@wm.mv-regierung.de,
+49 3855885160 and DE80J, in both copies. ReviewInfo + ReviewOrg is coherent, and the buyer's own triple is separate
(22753858/59/62 → 1460530). The plural "Vergabekammern" is the official designation of MV's review chambers. They sit together
at the economics ministry under one address, one e-mail and one phone, and the printed name never separates them. That is not
R2's W shape: "Vergabekammer NRW" pooled two chambers with different seats and e-mails, Köln and Münster. The name_prefix lookup
shows the same institution under about 20 spellings, for example "Vergabekammern des Landes Mecklenburg-Vorpommern" 12248823
(638) and the older ministry name 10445888 (1,807); these are separate keys and are not folded here. The target is not
city-flagged, so its newest-50 set is empty or {schwerin}. The sibling planned rows 22751964/65 (notice 24253163) print the
same Schwerin office. This pick and pick 17 share a buyer.

**19. 22938395, Vergabekammer Rheinland bei der Bezirksregierung Köln → 7473 (6,101). R.** The notice is Universitätsklinikum
Köln AöR's "Sanierung Bauteil B | Metallbauarbeiten", awarded to Gebr. Hübenthal GmbH, Kassel (TED 24496663, DÖE 26557085). The
buyer is printed under its own name at Kerpener Str. 62. GUID `266c50cc…` is ORG-0003, Lot-ReviewOrg; its twin is ORG-0002,
Lot-ReviewInfo, `f11d4680…` (22938394). Both print "Geschäftsstelle Vergabekammer Rheinland", Zeughausstr. 2-10, 50667 Köln,
VKRheinland@bezreg-koeln.nrw.de, +49 2211473055 and DEA23, in both copies. ReviewInfo + ReviewOrg is coherent. The target 7473
is the only identifier-NULL row of the exact name. Page 1 of name_prefix also shows 71 GUID rows of the name, VAT rows
22146032 / 22534695, and several Köln chamber phone numbers taken as national keys (22318808, 22539480/81, 22575782, 22717048,
22785935). Page 2 holds 100 more two-mention GUID rows, and more pages follow. That is the per-notice split, all of it the same
Zeughausstraße chamber by name; none of it is a target. 7473 is not city-flagged. The keys "… bei der Bezirksregierung
Düsseldorf" (6173504, Spruchkörper Düsseldorf) and the bare "Vergabekammer Rheinland" (7615) stay apart. R2 read_top1 already
noted this name as a remaining split of the one Köln chamber, not a homonym.

## GETs

I made 44 public GETs as reader `c2`, from 21:18:48Z to 21:25:09Z: 17 `/v1/organizations/{id}`, 21
`/v1/notices/{id}/content` and 6 `/v1/organizations?name_prefix=…&country=DE`, one of them a cursor page 2.
- All returned 200, with no 408 and no retry.
- They are logged in `r2c/api_requests.log`, and the bodies are in `r2c/bodies/`.
- They went through the shared `r2c/.api.lock` and `.api.last` with reader c1. The smallest gap between any two logged
  requests, from either reader, is 2.050 s.
- I reused bodies already on disk: `r2/bodies/notices_47408004_content.json` and `notices_47414536_content.json`, and
  `r2b/bodies/notices_24331367_content.json`.
- I did not use `/v1/sql`, ssh, POST, git or cargo.
