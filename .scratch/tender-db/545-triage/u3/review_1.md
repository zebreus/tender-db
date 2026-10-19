# 545 unit 3, reviewer 1 of 6

Rows reviewed: u2a/verdicts.json indexes 7, 11, 32, 84, 90, 96, 105, 112 and 113. That is 9 rows and 9 mentions.

I read every evidence notice through `/v1/notices/{id}/content`. I also read the holder and the destination of each row, every alternative row in each destination pool, and the tender of each mention. I made 47 public GETs, all rc=200, each at least 2 s after the previous one. The request log `api_requests.log` and the cached `bodies/` directory are shared with the other reviewers. Everything I fetched is in `bodies/`.

Result: CONFIRM 7 rows (7 mentions rehome, all high) and PARK 2 rows (the two CUH mentions, keep).

| idx | holder | printed name | verdict | mention action | target |
|---|---|---|---|---|---|
| 7 | 30914265 Mid and South Essex NHS FT | Abbott Medical UK Limited | CONFIRM | rehome, high | 11546762 |
| 11 | 30914406 Crown Commercial Service | Thomas Silvey | CONFIRM | rehome, high | 31560264 |
| 32 | 31560765 HFMA | TIAA LTD | CONFIRM | rehome, high | 11853802 |
| 84 | 30916588 Valuation Office Agency | NLA MEDIA ACCESS LIMITED | CONFIRM | rehome, high | 30915240 |
| 90 | 30929662 South Eastern HSC Trust | PPL/PRS | CONFIRM | rehome, high | 31546963 |
| 96 | 30914046 Department for Transport | FIRE SERVICE COLLEGE LIMITED | CONFIRM | rehome, high | 11314818 |
| 105 | 30913583 DEFRA | Red Kite Veterinary Consultants Ltd | CONFIRM | rehome, high | 31578178 |
| 112 | 31549426 "Teleflex" (CUH's own PPON) | CAMBRIDGE UNIVERSITY HOSPITALS NHS FOUNDATION TRUST | PARK | keep | (1168300 would be right) |
| 113 | 31549426 "Teleflex" (CUH's own PPON) | Cambridge University Hospitals NHS Foundation Trust | PARK | keep | (1168300 would be right) |

For each confirmed row, the evidence section holds three fields and nothing else: the printed name (BT-500), the holder's PPON (BT-501) and GB (BT-514). The section is referenced once, by OPT-300-Tenderer. The buyer has its own section with its own PPON. No section is shared and no Buyer role points at a supplier.

## Per row

**7: Abbott Medical UK Limited on MSE's PPON** (notice 46791071, section ORG-GB-PPON-PBVJ-9475-ZCMD, Tenderer, tender 8637230)
- Buyer: Norfolk and Norwich University Hospitals (ORG-GB-PPON-PQLV-1688-MQYV).
- The title reads "Abbott Medical U.K. Ltd - Claris EP Recording System and EP4 Cardiac Stimulator maintenance", which is Abbott/St Jude EP kit.
- The holder 30914265 has 437 mentions and merged identifier GBNHSRAJ, so the PPON belongs to MSE, a buyer that plays no part in this notice. MSE and Abbott have no relation.
- Destination 11546762 "Abbott Medical UK Limited" is keyed 02423907 and carries merged identifier GBPPONPBMG2473WGVP (Abbott's own PPON). The name matches exactly and the row is the only candidate.
- The tender currently serves MSE as the Tenderer. **CONFIRM.**

**11: Thomas Silvey on CCS's PPON** (46842097, ORG-GB-PPON-PBZB-4962-TVLR, Tenderer, tender 8668647)
- Buyer: NHS Cornwall and Isles of Scilly ICB, which has its own section.
- The lot text reads "Fuel Cards and Associated Services VII Lot 1", a call-off from CCS's fuel-card framework. CCS acts as the buyer's framework authority. It is not Thomas Silvey's agent, parent or group, and the A class does not apply.
- Destination 31560264 "THOMAS SILVEY,LIMITED" GBCOH00360922 (8 mentions) is the only candidate. The names agree once the legal form is folded away. **CONFIRM.**

**32: TIAA LTD on HFMA's PPON** (46818192, ORG-GB-PPON-PHGQ-9419-WJLV, Tenderer, tender 8596135)
- Buyer: NHS England. The procurement is "External Investigation HR Services".
- HFMA owns PHGQ-9419-WJLV. Its own notice 46783223 (NHS England, One NHS Finance) names HFMA as Tenderer under that PPON, and the holder's slice rows show HFMA x4, TIAA x1 and the Academy for Healthcare Science x1.
- TIAA is a separate audit and investigations firm. It is the sole tenderer here, so there is no consortium.
- Pool: 11853802 "04546319" (11 mentions), 31546403 GBPPONPBRR3532NWYQ "TIAA LTD" (10), 14730774 "4546319" (6) and 31622768 GBCOH4546319 (1). Three of these rows carry the same company number, and the PPON row has the same name, so every candidate is the same entity.
- The most-referenced row is the destination. Its margin over the PPON row is thin (11 against 10) but harmless. **CONFIRM.** Side note for 470/473: one number is split across 3 rows.

**84: NLA MEDIA ACCESS LIMITED on VOA's PPON** (46834269, ORG-GB-PPON-PLBJ-4721-TQQJ, Tenderer, tender 8664618)
- Buyer: National Highways. The procurement is "NLA Licence 2026-2027".
- The Valuation Office Agency has no relation to NLA.
- Destination 30915240 "NLA media access Ltd" GBCOH03003569 (41 mentions) carries merged identifier GBPPONPLMM3896RGZM. It is the only candidate. **CONFIRM.**

**90: PPL/PRS on SEHSCT's PPON** (46763175, ORG-GB-PPON-PLGY6381WWWR, Tenderer, tender 8620561)
- SEHSCT is the buyer in its own section, hyphenated: ORG-GB-PPON-PLGY-6381-WWWR. PPL/PRS sits in the unhyphenated section id, so the two section ids differ. The holder's name is not in the mention's section; only its identifier is.
- The procurement is "Music Licence", which is TheMusicLicence issued by PPL PRS Ltd, so the slash name refers to that one JV company.
- Destination 31546963 "PPL PRS ltd" GBCOH10376001 (5 mentions). The alternative is 31554686 "PPL PRS" GBPPONPVTY2243HBDQ (1 mention). The same procedure's later notice 46768689 prints that PPON for "PPL PRS", and the tender currently serves 31554686 as its Tenderer.
- The pool missed a third row: provisional 31552269 "PPL/PRS" (1 mention), from notice 46763209, which prints the junk identifier "GB-PPON-PPL/PRS".
- All three rows are the same entity. The most-referenced identified row is sound. After the rehome, the tender's 3 notices sit on 3 PPL PRS rows; a later merge (469-style crosswalk) would fix that, and the rehome is not the place to do it. **CONFIRM.**

**96: FIRE SERVICE COLLEGE LIMITED on DfT's PPON** (46811712, ORG-GB-PPON-PLTC-5358-LTWZ, Tenderer, tender 8651794)
- Buyers: UKRI and UK SBS, each in its own section. The procurement is "Aircraft RFFS Low-Cat Training".
- DfT (388 mentions) has no relation to FSC Ltd, which was privatised from DCLG in 2013.
- Pool: 11314818 "08102633" (3 mentions) and 31573940 GBPPONPTQB6315ZCDD "Fire Service College Ltd" (2). Both have the same name and are the same entity. The margin is thin but harmless. **CONFIRM.**

**105: Red Kite Veterinary Consultants Ltd on DEFRA's PPON** (46868149, ORG-GB-PPON-PNBD-3289-CDGX, Tenderer, tender 8595247)
- Buyer: APHA (ORG-GB-PPON-PBHC-6261-BLLQ), a DEFRA agency. The publisher put the parent department's PPON on the supplier. DEFRA is APHA's parent, not Red Kite's.
- Destination 31578178 "RED KITE VETERINARY CONSULTANTS LIMITED" GBCOH08716366 (1 mention) matches exactly.
- The pool missed provisional 31545639 "Red Kite Veterinary Consultants Ltd" (2 mentions). The procedure's other notice 46748484 (section ORG-GB-FTS-161890, no identifier) binds there, and the tender currently serves that row as its Tenderer.
- The identified row comes first under G3, so the destination stands. **CONFIRM.**

**112 / 113: CUH on PPON PPCP-8863-HDGN, holder 31549426 headed "Teleflex Group Ltd T/A Teleflex Medical": PARK.**
- The premise is inverted, so rule 1 fails. On notice 46815909 (CUH's own "Provision of Bicarbonate of Soda" ITT), CUH's own section ORG-GB-NHS-RGT prints **both GB-NHS-RGT and GB-PPON-PPCP-8863-HDGN**. A second section, ORG-GB-PPON-PPCP-8863-HDGN, repeats CUH with that PPON. Both are Procedure-Buyer.
- Notice 46796632, co-bought with the East of England Collaborative Hub, again prints CUH with PPCP-8863-HDGN.
- The PPON is therefore most plausibly CUH's own. The holder row is CUH's PPON row, mis-headed by the suppliers that other buyers listed under it:
  - Teleflex x3, e.g. Yorkshire Ambulance notice 46756087 (Tenderer);
  - Hologic x2 on BHRUT tender 8690126 (Tenderer).
  - The holder has only 7 mentions: Teleflex 3, Hologic 2, CUH 1+1.
- The printed name (CUH) is therefore at least plausibly the very entity the holder's identifier names, not "a different legal entity", and rule 1 says to PARK.
- The destination is not the problem. 1168300 is provisional but carries merged identifier GBNHSRGT and 1,497 mentions, and tender 8654110 already serves CUH's NHS-RGT section as 1168300. If the panel resolves the PPON's owner differently, 1168300 is still the right target.
- The right fix sits outside this cohort: an identifier verdict giving GBPPONPPCP8863HDGN to CUH (452/513-style), which carries these 2 mentions to 1168300. After that, the Teleflex x3 become the 545 F on CUH's row.
- Rehoming CUH's 2 mentions now would empty CUH from its own PPON row and leave it 100% supplier names.
- Hologic x2 (index 114, not mine) stays a valid rehome either way, since Hologic is distinct from both CUH and Teleflex.
- Rules 3 and 4 pass: the section binds only CUH, and the role is Procedure-Buyer on a buying body.

## Notes for the panel
- The destination pools miss provisional rows that share the name (Red Kite 31545639, PPL/PRS 31552269). This is harmless here, because an identified row exists and G3 prefers it. It does mean "identified-unique" really means "unique among identified rows".
- PPON PPCP-8863-HDGN gathers three claimants from three publishers (CUH, Teleflex, Hologic). It belongs with the magnet-PPON note on 30926847, not with the one-off copy-paste premise.
