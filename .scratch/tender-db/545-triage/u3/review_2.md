# 545 unit 3: review 2 of 6 (2026-10-19)

This review covers u2a/verdicts.json rows 114, 124, 127, 134, 136, 174, 176, 190 and 195: 9 rows, 11 mentions.

## Method
- I used the public API only, through `get_r2.sh`. It sleeps 2 s before each GET under a flock, logs every request to `api_requests.log`, saves each body under `bodies/` and reuses a saved body.
- I made 41 GETs, all rc=200, with no retries. Two bodies were reused (31549426 and tender 8690126, fetched by another reviewer).
- What I read:
  - Every evidence notice's `/content`.
  - The holder and destination orgs.
  - Two pool alternatives (31538013 and 13175098).
  - The tender of every row, because the tender is where corrected re-publications show up. This is how I found the Tech Against Terrorism change.
  - Two extra notices on those tenders: 46697744 and 46875517.
- I made no /v1/sql calls, no POSTs, no git calls, and no edits outside u3.
- In every evidence notice the supplier sits in its own ORG section. That section carries only the printed name and the holder's identifier, and the only thing that references it is `OPT-300-Tenderer` via TPA-1-0. The buyer or buyers always have separate section ids. So guard G1 (shared section) never fires on these 11 mentions.

## Summary

| idx | printed name | holder | verdict | target | mentions |
|---|---|---|---|---|---|
| 114 | Hologic Ltd | 31549426 Teleflex | CONFIRM | 30914006 | 2 rehome high |
| 124 | Renishaw Neuro Solutions Limited | 31538012 GOSH | CONFIRM | 31552561 | 2 rehome high |
| 127 | Xiel | 30913740 Northampton GH | CONFIRM | 30974378 | 1 rehome high |
| 134 | Tech Against Terrorism | 30916247 Ofcom | **CHANGE-DESTINATION** | **30973170** (not provisional 31570425) | 1 rehome high |
| 136 | ERMC Ltd | 31571667 IoW NHS Trust | CONFIRM | 18928807 | 1 rehome high |
| 174 | Vohkus | 30926693 GMC | CONFIRM | 31551980 | 1 rehome high |
| 176 | Bolton NHS Foundation Trust | 30913561 MFT | CONFIRM | 30917930 | 1 rehome high |
| 190 | Gama Healthcare Ltd | 31551518 Finsen | **PARK** | (31551510) | 1 keep medium |
| 195 | Virgin Media Limited | 30913648 DVLA | **PARK** | (31540412) | 1 keep medium |

That gives 9 mentions to rehome at high confidence and 2 kept.

## Per row

### 114: Hologic Ltd on 31549426 "Teleflex Group Ltd T/A Teleflex Medical" (GBPPONPPCP8863HDGN, 7 mentions) → 30914006
- **Notices.**
  - 46877124 (UK6) and 46882065 (UK7) belong to tender 8690126, "Minimally Invasive Gynaecology Consumables for Myosure and Fluent". The buyer is BHRUT, in section ORG-GB-PPON-PBMX-8832-GMZL.
  - Section ORG-GB-PPON-PPCP-8863-HDGN reads "Hologic Ltd" / GB-PPON-PPCP-8863-HDGN / GB. It is referenced only as Tenderer.
  - MyoSure and Fluent are Hologic products, so the printed name is the real supplier.
  - The UK5 notice 46875517 on the same tender has no supplier section.
- **Holder vs name.** Teleflex and Hologic are unrelated companies; both are separate US-listed medtech groups. Hologic has its own PPON: the destination's merged identifier is GBPPONPZBR3269JJND. So PPCP-8863-HDGN is not Hologic's own registration.
- **Destination.** 30914006 "HOLOGIC LTD." GBCOH02722343, 37 mentions. The other pool rows are 14381876 "2722343", the same company number without the leading zero, and 15303476, a VAT number. They are all the same entity, so 30914006 is a sound tiebreak.
- **Verdict.** Both mentions rehome at high confidence.

### 124: Renishaw Neuro Solutions Limited on 31538012 GOSH (GBPPONPPYL1513MZZL, 34 mentions) → 31552561
- **Notices.**
  - 46795678 (UK5) and 46819442 (UK6) belong to tender 8642804, "Neurosurgical Robot". The buyer is Sheffield Teaching Hospitals, in section PYZV-3517-VQCJ.
  - Section ORG-GB-PPON-PPYL-1513-MZZL reads "Renishaw Neuro Solutions Limited" and is referenced as Tenderer only.
- **Holder vs name.** An NHS foundation trust and a private device maker. GOSH is at most a customer, not a corporate relation.
- **Destination.** 31552561 GBCOH10375761, an exact name match, 6 mentions. The other pool row, 31538013 GBPPONPRJV6161YXYQ (4 mentions), has the identical name. UK company names are unique, so it is the same legal entity split across two orgs, and either is a sound target. The COH-keyed row is preferable.
  - Side note, out of scope: 31538013 and 31552561 want merging.
- **Verdict.** Both mentions rehome at high confidence.

### 127: Xiel on 30913740 Northampton General Hospital (GBPPONPQBY8547GYGP, 489 mentions, merged GB-NHS-RNS) → 30974378
- **Notice.**
  - 46835667 (UK5) belongs to tender 8665378, "C-Rad Catalyst (Maintenance of)". The buyers are Betsi Cadwaladr (GB-NHS-7A1) and NWSSP (GB-NHS-RQF), each in its own section.
  - Section ORG-GB-PPON-PQBY-8547-GYGP reads "Xiel" and is referenced as Tenderer only.
  - Xiel is a UK radiotherapy-equipment supplier (C-RAD). Northampton General Hospital is an NHS trust (ODS RNS), and the PPON is its own. There is no corporate relation.
- **Destination.** 30974378 "XIEL LTD" GBCOH11875702, 12 mentions, with merged PPON PGVW-7219-MLVN, which is Xiel's own registration. The other pool rows, 15942203 and 17415439, are "Xiel Ltd" keyed by a VAT number (327080811). That is no conflicting company number.
- **Verdict.** 1 mention rehomes at high confidence.

### 134: Tech Against Terrorism on 30916247 Ofcom (GBPPONPQQZ1663NWMN) → CHANGE-DESTINATION 30973170
- **Notice.** 47157838 (UK5, published 2026-09-01) belongs to tender 8582476, "Proactive Discovery and Bulk Referral Pilot". The buyer is the Home Office (PWGC-6513-PQLZ). Section ORG-GB-PPON-PQQZ-1663-NWMN reads "Tech Against Terrorism" and is referenced as Tenderer only; the bid is £192,000.
- **Correction found on the same tender.** The Home Office re-published the same procedure (ocds-h6vhtk-06efa9) as 46697744 (UK5, 2026-09-22) and 46703079 (UK6, 2026-09-23).
  - The tenderer is "Online Harms Foundation TA/ Tech Against Terrorism", with GB-COH-11656320, the same £192,000 and a single tenderer.
  - Those mentions are bound to **30973170** "Online Harms Foundation TA/ Tech Against Terrorism" GBCOH11656320, which is identified and has 2 mentions.
- **Holder vs name.** Ofcom is a statutory regulator. The Online Harms Foundation, trading as Tech Against Terrorism, is an independent company. They are not related.
- **Destination.** The proposed 31570425 is a provisional "Tech Against Terrorism" with no identifier and 1 mention. The identified legal entity behind the trading name is 30973170, so the mention should go there.
  - Side note, out of scope: 31570425 is probably the same entity.
- **Verdict.** 1 mention rehomes to 30973170 at high confidence.

### 136: ERMC Ltd on 31571667 "Isle of Wight NHS Trust" (GBPPONPRBY9272TQGX, 4 mentions) → 18928807
- **Notice.** 46819437 (UK7) belongs to tender 8656068, "Refurbishment of Westminster House – Quantity Surveying and Design", a care home in Newport. The buyer is Isle of Wight Council (PGYX-7387-YJXG). Section ORG-GB-PPON-PRBY-9272-TQGX reads "ERMC Ltd" and is referenced as Tenderer only.
- **Holder vs name.** A private surveying and design consultancy versus an NHS trust. They are not related.
- **Caveat.** The holder is small: 3 trust-named mentions against this 1. It carries no GB-NHS-R1F, so I cannot prove which entity really registered PRBY-9272-TQGX. Either way, the ERMC-named mention does not belong on an org served as "Isle of Wight NHS Trust".
- **Destination.** 18928807 "ERMC Limited", keyed 04489340, 13 mentions, and the only row with the vote key. The name is exact after legal-form folding, and company names are unique on the register. No number is printed in the section to contradict it.
- **Verdict.** 1 mention rehomes at high confidence.

### 174: Vohkus on 30926693 "General Medical Council (GMC)" (GBPPONPXLD1125PGBY, 83 mentions) → 31551980
- **Notice.**
  - 46771116 (UK7) belongs to tender 8639596, "IS Commodity Items". The buyer is GMC under GB-CHC-1089278, in its own section; that section is bound to the other GMC org, 30914860.
  - Section ORG-GB-PPON-PXLD-1125-PGBY reads "Vohkus" and is referenced as Tenderer only. GMC evidently put its own PPON on the supplier.
- **Holder vs name.** Vohkus is an IT reseller and GMC is a regulator. They are not related.
- **Destination.** 31551980 "VOHKUS LIMITED" GBCOH04142508, with merged PPON PYHT-6272-TXNX, which is Vohkus's own registration. The other pool row, 31616633, is keyed GBCOH4142508: the same number, so the same entity.
- **Verdict.** 1 mention rehomes at high confidence.

### 176: Bolton NHS Foundation Trust on 30913561 Manchester University NHS FT (GBPPONPXQG2592VYYY, merged GB-NHS-R0A) → 30917930
- **Notice.** 47366319 (UK7) belongs to tender 8829421, "Improvements in Operational Performance – Surv Endoscopy (Bolton)". It is GM Cancer Alliance funding. The buyer is The Christie NHS FT (PTTQ-9511-YPYH). Section ORG-GB-PPON-PXQG-2592-VYYY reads "Bolton NHS Foundation Trust" and is referenced as Tenderer only.
- **Role check.** A public body as Tenderer is atypical, but the notice plainly shows Bolton as the recipient of the Bolton-site funding. This is not a Buyer or Procedure-Buyer role, so the red flag does not apply.
- **Holder vs name.** Bolton FT (ODS RMC) and MFT (ODS R0A) are separate statutory foundation trusts. There is no parent, subsidiary or group relation, and Bolton is not in an MFT group. The likely cause is MFT's PPON copied from a sibling notice in the same Christie funding series.
- **Destination.** 30917930 "Bolton NHS Foundation Trust", PPON PTGM-8148-JHHN, merged GB-NHS-RMC. This is exactly the entity.
- **Verdict.** 1 mention rehomes at high confidence.

### 190: Gama Healthcare Ltd on 31551518 FINSEN TECHNOLOGIES LIMITED (GBPPONPYMW5475ZMBP, 4 mentions) → PARK
- **Notice.** 46842943 (UK5) belongs to tender 8669104, "Provision of Surface Cleaning Wipes". The buyers are NWSSP (GB-NHS-RQF and PXWV-6492-CGMN). Section ORG-GB-PPON-PYMW-5475-ZMBP reads "Gama Healthcare Ltd" and is referenced as Tenderer only.
- **Destination.** 31551510 "GAMA Healthcare Ltd" PPBZ-6256-TJCR (7 mentions) would be sound. The other GAMA rows all carry 05316871 or 5316871, so there is no conflicting number.
- **Why it parks: criterion 1 is not clear.**
  - Finsen (UV-C disinfection) and GAMA (Clinell infection-prevention products) are UK NHS suppliers in the same infection-prevention niche. A distributor, agent or group link between them is plausible, and none of the permitted reads can rule it out.
  - This matches the standard unit 2b applied to Pharmagraph on Acquisition Systems, which it classed F?.
  - Wipes are GAMA's product, so the rehome is probably right in substance. It can be promoted if a register check shows no common ownership or agency (Finsen's PSCs and officers against GAMA Healthcare Ltd 05316871).
- **Verdict.** 1 mention kept, medium confidence.

### 195: Virgin Media Limited on 30913648 DVLA (GBPPONPYVZ4215PGYM, 107 mentions) → PARK
- **Notice.** 47157734 (UK7) belongs to tender 8806962, "C3, A38 Dean Prior Bridge Replacement – Virgin Media". This is an NRSWA utility estimate. The buyer is National Highways Limited (PGMJ-3793-VBBN). Section ORG-GB-PPON-PYVZ-4215-PGYM reads "Virgin Media Limited" and is referenced as Tenderer only.
- **Holder vs name.** Criteria 1 and 3 are clear: Virgin Media and the DVLA are not related, and the section binds Virgin Media alone.
- **Why it parks: criterion 2 fails.**
  - The only pool row is 31540412 "Virgin Media Ltd", keyed GBCOH06024812, with 1 mention.
  - As far as I know, Virgin Media Limited (the network operator, and the undertaker on a C3 estimate) is company 02591237. 06024812 is the number of Virgin Media Payments Ltd, the group billing company whose number appears on Virgin Media bills.
  - So the destination row looks like a "Virgin Media Ltd" name keyed with a sister company's number. The company number does not agree with the printed entity.
  - No better destination could be found with the permitted reads: there is no name search, and the 02591237 org id is unknown.
- **Verdict.** 1 mention kept, medium confidence. To resolve it, check 06024812 against the register. If it is Virgin Media Limited after all, the row can be confirmed.
