# 530 R2c top-up: independent planned rows, picks 0-9 (read_c1)

Source: `guid_twin_plan_2660.json` (Build 1b, deployed 8e9222c, dry job 2660) and `r2c_picks.json` `picks[0..9]` (salt 541337;
no pick within 5 ids of the 120 rows R2/R2b read). Stored mentions are from `r2c/q1.out.json`. Each pick has exactly 2
mentions: the TED eForms copy and the DÖE DE1 copy of one notice, with the same section and the same raw BT-501 in both. All 10
are in `planned_row_ids`. Each row has one target, and all 10 targets exist (identifier NULL, provisional, DE, exactly the row's
name), so none is a missing key that Build 2 would mint.

Strata (plan role class: Procedure-Buyer → buyer, every `Lot-` role → review_or_info): buyer 5 (picks 2, 3, 7, 8, 9),
review_or_info 5 (picks 0, 1, 4, 5, 6).

Question: would rehoming all the row's mentions to its target, the identifier-NULL `(org_name_norm(name), DE)` row, be right?
- **R**: same body; the GUID row was a per-notice split.
- **N**: cannot tell.
- **W**: one of these holds:
  - the target name denotes more than one body;
  - the target would receive another body's mentions;
  - a review-org slot prints another body's details;
  - the GUID is a real stable key.

**How each target was identified.**
- Directly from `planned_mention_keys` (truncated at org id 22817622): 22775107 → 15234277 and 22815410 → 622.
- From `r2_top_targets`, the entry for the same `(name_norm, DE)` key: 622, 2443792, 2251109, 3233, 3856, 15234277, 1405749,
  1222044.
- RWTH (pick 2): `planned_mention_keys` maps the same key's rows in the sibling notice 24332037 (22809267, 22809268, 22809271)
  to 1547849. The name_prefix listing shows 1547849 as the only identifier-NULL DE row with that exact name.
- JVA (pick 7): the name_prefix listing. 13200890 is the only identifier-NULL DE row with the exact name. The key is not in
  `missing_target_samples`.

**Evidence read for each pick.**
- `/v1/organizations/{pick}`: the GUID is still in place, `identifier_kind` national, 2 mentions, not provisional.
- `/v1/organizations/{target}`: identifier NULL, provisional, DE, and its mention count (read fresh; each count equals the
  plan's).
- Both copies through `/v1/notices/{id}/content`. For every ORG section I read the name, BT-501, contact, street, postcode,
  city, e-mail, phone and NUTS. I also read the role references: OPT-300/301 in the TED copy and the DE1 party references in
  the DÖE copy.
- The targets' newest mentions:
  - Eight targets are in `r2_top_targets`. R2 read their listed notices today (`read_top1` / `read_top2`). I re-read those
    bodies in `r2/bodies/` for street and e-mail: 47414764, 47414774, 47414721, 26723175, 23528496, 23972531, 4220339, 3439214
    and 44550117.
  - For 1547849 and 13200890 the report lists no notice, and the allowed endpoints give no mention listing. For RWTH I read the
    sibling planned notice 24332037 instead.
- The city screen. `city.flagged` is complete (36 of 36), and none of the 10 targets is on it. So each target's newest-50 city
  set is either empty or exactly the planned city.

**Independence.**
- None of the 20 pick notices, and not 24332037, appears in any R2/R2b read file, result JSON or body directory.
- The nearest R2/R2b-read row to any pick is 22824042 (BLB NRW Bielefeld, notice 24349929), 21 ids from pick 3, in a
  different notice.
- So all 10 are independent at row and notice level.
- **Caveat at target level:** 8 of the 10 targets are R2 top-30 targets that R2 already scored R. Several of them also
  received R2/R2b random picks: 15234277, 2251109, 2443792, 1405749 and 1222044. This read re-tests the fold of a fresh
  notice into them, not their homonym status beyond the city screen.

**What all 10 notices share.**
- **Per-notice keys.** The pick's GUID sits on the same section with the same value in both copies. The twin sections carry
  other fresh v4 values, the same in both copies and different from notice to notice. No pick's value recurs: 2 mentions, one
  notice.
- **Role guard (A7).**
  - Six groups are buyer-family: Procedure-Buyer + Lot-AddInfo, sometimes with Lot-TenderReceipt, and in pick 3 also the
    buyer's own Lot-ReviewInfo.
  - Pick 1 is buyer-family with its row on TenderReceipt.
  - Three groups are review-family: Lot-ReviewInfo + Lot-ReviewOrg + Lot-Mediator, all printing the chamber.
  - No group mixes a buyer-side role with Lot-ReviewOrg, and no group holds a Tenderer.
  - Every section of a group gives one street and one e-mail. Review chambers, tenderers and the eSender are always printed
    under their own names.
- **Neither R2 W shape occurs.**
  - No umbrella name: the three chamber targets are the federal chambers (singular and plural) and VK Westfalen. Each is one
    institution with one city and one e-mail domain.
  - No review slot prints another body's details.

**GETs.**
- 42 public GETs: 21 `/v1/organizations` (10 picks, 9 targets, 2 `name_prefix` listings) and 21 `/v1/notices/{id}/content`.
- All returned 200, with no 408 and no retry.
- They are logged in `r2c/api_requests.log` (tag `reader=c1`, 21:18:53Z to 21:24:17Z), and the bodies are in `r2c/bodies/`.
- They went through the shared `r2c/.api.lock` and `.api.last` together with reader c2. The smallest gap between any two logged
  requests, from either reader, is 2.05 s.
- Reused: c2's `organizations_2443792` (21:23:33Z) and R2's target-notice bodies listed above.
- I did not use `/v1/sql`, ssh, POST, git or cargo.

## Tally

**R 10, N 0, W 0** (buyer 5/5 R, review_or_info 5/5 R).

| # | org_id | name | section / role | target (mentions) | verdict |
|---|---|---|---|---|---|
| 0 | 22815410 | Vergabekammer des Bundes | ORG-0003 Lot-ReviewInfo | 622 (111,305) | R |
| 1 | 22890922 | Bau- und Liegenschaftsbetrieb NRW Aachen | ORG-0005 Lot-TenderReceipt | 2443792 (573) | R |
| 2 | 22840477 | RWTH Aachen University | ORG-0001 Procedure-Buyer | 1547849 (469) | R |
| 3 | 22824021 | Stadt Münster | ORG-0001 Procedure-Buyer | 2251109 (192) | R |
| 4 | 22843623 | Vergabekammer Westfalen | ORG-0005 Lot-ReviewOrg | 3233 (37,696) | R |
| 5 | 22904876 | Vergabekammern des Bundes | ORG-0003 Lot-ReviewInfo | 3856 (17,642) | R |
| 6 | 22775107 | Bundesstadt Bonn - Referat Vergabedienste | ORG-0002 Lot-AddInfo | 15234277 (376) | R |
| 7 | 22949668 | JVA Castrop-Rauxel Zentralstelle für das Beschaffungswesen im Justizvollzug | ORG-0001 Procedure-Buyer | 13200890 (315) | R |
| 8 | 22874677 | HIL Heeresinstandsetzungslogistik GmbH | ORG-0001 Procedure-Buyer | 1405749 (2,076) | R |
| 9 | 22827487 | Bau- und Liegenschaftsbetrieb NRW Münster | ORG-0001 Procedure-Buyer | 1222044 (261) | R |

## Items

**0. 22815410, Vergabekammer des Bundes → 622 (111,305). R. Independent.** GUID `166dae2a…` sits on ORG-0003 Lot-ReviewInfo of the
Knappschaftskrankenhaus Bottrop GmbH contract notice "TGA-Planung, LPH 1-8, § 55 HOAI zur Errichtung einer Endoskopie"
(cn-standard, 2024-11-27; TED 24339102, DÖE 26516806). ORG-0004 Lot-ReviewOrg (`4198bc5a…`) and ORG-0005 Lot-Mediator (`0c892429…`)
print the same name. In both copies all three give Villemombler Straße 76, 53123 Bonn, vk@bundeskartellamt.bund.de,
+49 2289499-0 and fax -163. The group is review-family only, with no buyer-side role, so Build 1b's guard rightly leaves it refused.
The buyer (Bottrop) and its agent Aurantia Rechtsanwaltsgesellschaft (Oberhausen) are other names; Aurantia forms its own twin
group as AddInfo and TenderReceipt. The target 622 has identifier NULL, is provisional and holds 111,305 mentions; its city set is
{bonn}. Its newest mention 47414764 (2026-10-16) prints "Vergabekammer des Bundes" at Villemombler Straße 76, Bonn,
vk@bundeskartellamt.bund.de. The name denotes the federal review chambers at the Bundeskartellamt: one institution with numbered
panels. It is an office two-word key that R2 scored R (read_top2 item 19, read_office item 2). The stable GUID row 255 is the same
body and KEEP spares it; the fold lands on 622 and never on 255.

**1. 22890922, Bau- und Liegenschaftsbetrieb NRW Aachen → 2443792 (573). R. Independent.** GUID `4014fe4a…` sits on ORG-0005
Lot-TenderReceipt of "BLB NRW AC/(Bund)/Aachen/KöKa/FktuAusbGeb/KfAusbH/Schlosser/005-24-00971 III" (cn-standard, 2024-12-20;
TED 24435144, DÖE 26539238). This is a locksmith lot that the Aachen branch runs for federal construction. ORG-0001 Procedure-Buyer
(`e87930e7…`) and ORG-0002 Lot-AddInfo (`4daea52e…`) print the same name. All three give "Einkauf und Vertragsmanagement",
Mies-van-der-Rohe-Straße 10, 52074 Aachen, AC.Einkauf@blb.nrw.de, in both copies. The group is Buyer + AddInfo + TenderReceipt:
buyer-side only, role-coherent. Because the work is federal construction, the review body is "Vergabekammern des Bundes beim
Bundeskartellamt" (ReviewInfo and ReviewOrg, Villemomblerstr. 76, Bonn). It is its own twin under another key. The target 2443792
has identifier NULL and 573 mentions; its city set is empty because its listed mentions are 2008 text-era notices. Its notice
3439214 gives "BAU- UND LIEGENSCHAFTSBETRIEB NRW AACHEN", Aachen. R2's read_top2 item 17 found the same Mies-van-der-Rohe-Straße
address in a 2024 planned notice. The target is one branch office of the NRW state building agency.

**2. 22840477, RWTH Aachen University → 1547849 (469). R. Independent.** GUID `7cde69c5…` sits on ORG-0001 Procedure-Buyer of
"153 - Schüttler und Fermenter" (two lots; cn-standard, 2024-11-12; TED 24370980, DÖE 26503061). ORG-0002 Lot-AddInfo (`d10541f9…`,
row 22840478) and ORG-0005 Lot-TenderReceipt (`b70ea354…`, row 22840481) print the same name. All three give "Abt. 7.3
Zentraleinkauf", Wüllnerstr. 5, 52062 Aachen, beschaffung@zhv.rwth-aachen.de and +49 2418094200, for both lots and in both copies.
VK Rheinland (Zeughausstr. 2-10, Köln) is ReviewInfo and ReviewOrg under its own name, as its own twin. The group is buyer-side
only. The plan lists this key in no sample list. `planned_mention_keys` maps the same key's three rows in the sibling notice
24332037 to 1547849: that notice is "174 - Sukzessivleistungsvertrag Büromaterialien 2025" (2024-11-21) and has the identical
three-section shape and address. The name_prefix listing shows 1547849 as the only identifier-NULL DE row named "RWTH Aachen
University" (469 mentions, provisional). The identified row 1807 (national `0533400020020600137`, 658 mentions) is the same
university under a real key. 300 §10 keeps the fold off it, so that split exists already and stays. The target is not
city-flagged, so its newest-50 city set is empty or {aachen}. I read no notice the target already holds: the report lists none
for this key, and the allowed endpoints have no mention listing. The name denotes one university, one legal body.

**3. 22824021, Stadt Münster → 2251109 (192). R. Independent.** GUID `06741c77…` sits on ORG-0001 Procedure-Buyer of
"Estricharbeiten - Neubau einer dreizügigen Grundschule in Albachten Ost" (cn-standard, 2024-11-19; TED 24349889, DÖE 26509803).
Three more sections print the same name: ORG-0002 Lot-AddInfo (`c71552be…`), ORG-0003 Lot-ReviewInfo (`8dc2aff4…`) and ORG-0005
Lot-TenderReceipt (`e03a8ddb…`). All four give "Zentrales Vergabemanagement", Klemensstraße 10, 48143 Münster,
vergaben@stadt-muenster.de and +49 251492-0. The ReviewInfo section is the city's own office, which answers questions about review
deadlines; it is not a review body. The ReviewOrg is a single section, "Vergabekammer Westfalen bei der Bezirksregierung Münster"
(Albrecht-Thaer-Straße 9), under its own name. The group holds no Lot-ReviewOrg, so the guard rightly does not bind. This is the
same shape as R2b read_rand3 item 40. The target 2251109 has identifier NULL and 192 mentions; its city set is {munster}. Its
listed mention 26723175 (2025-08-01) prints "Stadt Münster", 48143 Münster, vergaben@stadt-muenster.de. "Stadt Münster" names the
city of Münster in NRW; the other places called Münster are Gemeinden, not Städte.

**4. 22843623, Vergabekammer Westfalen → 3233 (37,696). R. Independent.** GUID `338386c7…` sits on ORG-0005 Lot-ReviewOrg of the
DMT Gesellschaft für Lehre und Bildung mbH (Bochum) notice "Elektro- und Nachrichtentechnik" (cn-standard, 2024-11-26; TED
24375366, DÖE 26516135). ORG-0004 Lot-ReviewInfo (`adf56f05…`) and ORG-0006 Lot-Mediator (`8638ff00…`) print the same name. All
three give Albrecht-Thaer-Straße 9, 48147 Münster, vergabekammer@bezreg-muenster.nrw.de and +49 2514110, in both copies. The group
is review-family only. The notice holds two more twin groups, each under its own name and its own key:
- DMT: Buyer ORG-0001 and TenderReceipt ORG-0007, Herner Straße 45, Bochum;
- Facility Care AG: SProvider ORG-0002 and AddInfo ORG-0003, Münster.

The target 3233 has identifier NULL and 37,696 mentions; its city set is {munster}. Its newest mention 47414774 (2026-10-16)
prints "Vergabekammer Westfalen" with vergabekammer@bezreg-muenster.nrw.de. It is an office two-word key, but "Westfalen" pins one
chamber: Build 1b pins it safe, and R2 scored it R (read_top2 item 16). It is not R2's W1 umbrella shape ("Vergabekammer NRW"
collecting the Köln and Münster chambers). VK Westfalen also sits under its longer name 6136373; that split exists already and is
not a homonym.

**5. 22904876, Vergabekammern des Bundes → 3856 (17,642). R. Independent.** GUID `cb94fbdc…` sits on ORG-0003 Lot-ReviewInfo of
the VBG Hauptverwaltung Hamburg award "Erweiterung PCS-Komponenten (offline Türsysteme) für PREVIER" (can-standard, 2024-12-04;
TED 24453623, DÖE 26521991). The winner is PCS Systemtechnik GmbH, Essen. ORG-0004 Lot-ReviewOrg (`5ab107a1…`) and ORG-0005
Lot-Mediator (`fd012113…`) print the same name. All three give Bundeskanzlerplatz 2, 53113 Bonn, vk@bundeskartellamt.bund.de,
+49 228-9499-0 and fax -400. That street differs from the target's other mentions (Villemombler Straße 76, Kaiser-Friedrich-Straße
16). The phone, fax and e-mail are the Bundeskartellamt's, though, so this is the VBG template's address for the same federal
chambers, not another body. The group is review-family only. The target 3856 has identifier NULL and 17,642 mentions; its city set
is {bonn}. Its newest mention 47414721 (2026-10-16) prints "Vergabekammern des Bundes" at Kaiser-Friedrich-Straße 16, 53113 Bonn.
R2 read_top2 item 21 scored it R on another VBG notice, 24253044, which printed Villemomblerstr. 76 with
mail@bundeskartellamt.bund.de. So VBG's own notices print the chambers at two streets under one e-mail domain. It is the
institution of item 0 under the plural name; that split exists already.

**6. 22775107, Bundesstadt Bonn - Referat Vergabedienste → 15234277 (376). R. Independent.** GUID `37371690…` sits on ORG-0002
Lot-AddInfo of the Bonn award "Bundesstadt Bonn: Beethovenhalle - VEA14, Restaurierung Wandmalerei" (can-standard, 2024-10-23;
TED 24283696, DÖE 26486969). The winner is Restaurierungsatelier Vulkaneifel, Demerath. ORG-0001 Procedure-Buyer (`48b63298…`)
prints the same name. Both give Bertha-von-Suttner-Platz 2-4, 53111 Bonn, referatvergabedienste@bonn.de, +49 228772600 and fax
+49 228779619666. VK Rheinland (Köln) is ReviewInfo and ReviewOrg under its own name. The group is Buyer + AddInfo. The target
15234277 has identifier NULL and 376 mentions; its city set is empty. Its listed 2023 TED-form notice 23528496 gives the same name
at Bertha-von-Suttner-Platz 2-4, with the same e-mail and fax. The target is one office of the city of Bonn (R2 read_top1 item 2;
R2/R2b random picks on this target were all R).

**7. 22949668, JVA Castrop-Rauxel Zentralstelle für das Beschaffungswesen im Justizvollzug → 13200890 (315). R. Independent.**
GUID `ee292884…` sits on ORG-0001 Procedure-Buyer of the award "Belieferung der Justizvollzugsanstalten NRW mit schwer
entzündlichen Matratzen und Keilkissen inkl. PU-Bezüge" (can-standard, 2025-01-21; TED 24509994, DÖE 26555175). The winner is
SOFTLINE-Schaum GmbH & Co. KG, Storkow. The buyer is marked as an awarding central purchasing body (OPP-051, and
AwardingCPBIndicator=1 in the DÖE copy). ORG-0002 Lot-AddInfo (`924799e8…`, row 22949669) prints the same name. Both give
Lerchenstraße, 44581 Castrop-Rauxel, zentralebeschaffung@jva-castrop-rauxel.nrw.de and +49 2305983922. The ReviewOrg is VK
Westfalen (Münster), under its own name. The group is Buyer + AddInfo.

In the name_prefix listing, 13200890 (315 mentions, provisional) is the only identifier-NULL DE row with this exact name. The
other 12 exact-name hits are GUID rows with 2 mentions each: per-notice pairs like this one (22832738/39, 22835568/69,
22848052/53/55, 22950132/33, 23016472 and the pick's own pair). The variant names are other keys, and the fold leaves them alone:
- 22258397, identified `0204ZBS`: "JVA Castrop-Rauxel\nZentralstelle für das Beschaffungswesen";
- 8385342, an abbreviated spelling;
- 9975474, the bare "JVA Castrop-Rauxel";
- 19468057 and 21268293, a different office ("Zentralstelle Arbeitsverwaltung …").

The target is not city-flagged, so its city set is empty or {castrop-rauxel}. I read no notice the target holds, because none is
listed. The name denotes one office: the NRW prison service's central procurement office at JVA Castrop-Rauxel.

**8. 22874677, HIL Heeresinstandsetzungslogistik GmbH → 1405749 (2,076). R. Independent.** GUID `63050a66…` sits on ORG-0001
Procedure-Buyer of "BG_1450_EU/24 - Werksinstandsetzung von Achsstrebe, Fahrzeug des Systems GTK Boxer" (two lots; can-standard,
2024-12-17; TED 24414612, DÖE 26533931). The winners are Rheinmetall Landsysteme GmbH (Unterlüß) and SCHÜLER Wehrtechnik GmbH
(Limburg). ORG-0002 Lot-AddInfo (`c943a099…`) prints the same name. Both give Josef-Wirmer-Straße 2-8, 53123 Bonn,
vergabe@hilgmbh.de and +49 228-4463-2237. The ReviewOrg is "Bundeskartellamt Vergabekammer des Bundes" (Kaiser-Friedrich-Str. 16),
under its own name and on both lots. The group is Buyer + AddInfo. The target 1405749 has identifier NULL and 2,076 mentions; its
city set is {bonn}. Its listed mentions show HIL in Bonn: 4220339 is text-era, and 23972531 (2024-05-14) gives
Josef-Wirmer-Straße 2-8 with vergabe@hilgmbh.de. The target is one company (R2 read_top1 item 6).

**9. 22827487, Bau- und Liegenschaftsbetrieb NRW Münster → 1222044 (261). R. Independent.** GUID `939657ca…` sits on ORG-0001
Procedure-Buyer of "BLB NRW Münster / Universität Münster - Sanierung Hüfferstiftung / Dachdecker- und Zimmerarbeiten"
(can-standard, 2024-11-11; TED 24354265, DÖE 26502152). The winner is Behrens GmbH & Co. KG, Ahlen. ORG-0002 Lot-AddInfo
(`5138d6ee…`) prints the same name. Both give "Einkauf und Vertragsmanagement", Hohenzollernring 80, 48145 Münster,
BLBVergabe@blb.nrw.de. The ReviewOrg is VK Westfalen, under its own name. The group is Buyer + AddInfo. The target 1222044 has
identifier NULL and 261 mentions; its city set is {munster}. Its listed mention 44550117 (2025-05-28) has no BT-501 and prints the
name twice at Hohenzollernring 80 with BLBVergabe@blb.nrw.de: once as buyer and once as the Justiziariat. So the target already
holds this body's per-role duplicates (R2 read_top2 item 23). It is the Münster branch of BLB NRW.
