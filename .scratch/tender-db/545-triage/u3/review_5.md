# 545 unit 3, reviewer 5 of 6

Rows reviewed: u2b/verdicts2b.json indexes 122, 182, 188, 195, 200, 302, 303, 322 and 347. That is 9 rows and 12 mentions.

What I read:
- Every evidence notice, through `/v1/notices/{id}/content`.
- The holder and destination of each row, and every alternative row in each destination pool. I also read the second provisional "TPS Fruit and Veg Ltd" row (11707140), which the pool did not list.
- The tender of every mention.
- Three sibling notices: DDM's later notice 46893016, NCCSF's own notice 46803407 and Prosci's UK11 notice 46804992.

That was 51 public GETs, all rc=200, through `get1.sh` (sleep 2 before each, logged, cached). My wrapper is `get_r5.sh`. No body was fetched twice; 30915240 was already cached.

Local cross-checks (read-only files, no new reads):
- The register snapshots `448-campaign/companies-house-2026-09-30.json` and `469-registry-pairs/u1/companies-house.json`.
- The 452/466 identifier verdicts in `519-nhs-digit-codes/measure/m3-raw/admin_identifier.json`.
- u2b's `r11_dest.out.json`, the destination-pool name scan.

Result: CONFIRM 8 rows (11 mentions rehome, all high) and PARK 1 row (Northumbria, 1 mention kept).

| idx | holder | printed name | role(s) | verdict | mentions | target |
|---|---|---|---|---|---|---|
| 122 | 30915240 NLA media access Ltd (PPON PLMM-3896-RGZM) | Prosci UK Limited | Tenderer | CONFIRM | 1 rehome, high | 31575934 |
| 182 | 31568872 Norwich City Community Sports Foundation (04100611) | DDM Health Ltd | Tenderer | CONFIRM | 1 rehome, high | 30953794 |
| 188 | 30926702 NHS Confederation (04358614) | Diabetes UK | Tenderer | CONFIRM | 1 rehome, high | 31562654 |
| 195 | 31545104 Mobysoft Ltd (04546648) | LINCOLNSHIRE HOUSING PARTNERSHIP LIMITED | Procedure-Buyer x4 | CONFIRM | 4 rehome, high | 30917202 |
| 200 | 31551788 Enventure Research Ltd (04693096) | LAND USE CONSULTANTS LIMITED | Tenderer | CONFIRM | 1 rehome, high | 15715155 |
| 302 | 31540233 Explorance Global Ltd (08987402) | TPS Fruit & Veg Ltd | Tenderer | CONFIRM | 1 rehome, high | 21727460 (provisional) |
| 303 | 30930397 Whistlejacket London (09021526) | Don Whitley Scientific | Tenderer | CONFIRM | 1 rehome, high | 13064119 |
| 322 | 31568585 "TASS College House" (10167191) | University of Northumbria at Newcastle | Tenderer | **PARK** | 1 keep | none |
| 347 | 31549084 Angel and Star Care Services (11618190) | Action Deafness | Tenderer | CONFIRM | 1 rehome, high | 31540259 |

The sections were checked by script over all 12 mentions:
- Every evidence section holds exactly three fields: BT-500 (the printed name), BT-501 (the holder's number) and BT-514 (GB).
- Each section is referenced exactly once. That is one OPT-300-Tenderer, or for LHP one OPT-300-Procedure-Buyer.
- The buyer always sits in its own section with its own identifier. No holder name appears in any evidence section.
- The only same-identifier sibling is Action Deafness's notice. It carries a distinct section `ORG-GB-COH-11618190#2` for Angel and Star, which is bound by its own TenderingParty.

## Per row

**122: Prosci UK Limited on NLA's PPON** (46768774, ORG-GB-PPON-PLMM-3896-RGZM, Tenderer, tender 8623803)
- Buyer: NHS Business Services Authority (ORG-GB-PPON-PRLZ-1599-JGTT). The procurement is "W213310 Enterprise Change Management Services & Change Management Practitioner", a single tender action. That is Prosci's change-management methodology and certification.
- The holder 30915240 "NLA media access Ltd" GBCOH03003569 (register: NLA MEDIA ACCESS LIMITED, formerly The Newspaper Licensing Agency) carries merged PPON GBPPONPLMM3896RGZM. So the PPON is NLA's.
- A newspaper licensing agency and a change-management firm have no relation.
- The tender's other notice, 46804992 (UK11), carries only the buyer.
- Destination 31575934 "Prosci Uk Ltd" GBPPONPPJX4251TMBX (4 mentions) is the only identified candidate. The name agrees once Limited/Ltd is folded. r11 also shows a provisional "PROSCI UK LIMITED" 31539166; the identified row is the better home. Nothing is US or Ireland (Proscia is a different firm). **CONFIRM.**

**182: DDM Health Ltd on NCCSF's number** (46890437, ORG-GB-COH-04100611, Tenderer, tender 8700852)
- Buyer: Norfolk County Council. The procurement is "Tier 2 Healthy Weight Service for Children and Young People in Norfolk".
- The register gives 04100611 as Norwich City Community Sports Foundation, and 07975193 as DDM HEALTH LIMITED (formerly Data Driven Medicine / Diabetes Digital Media).
- The same tender's later award notice 46893016 prints the same wrong number, unpadded (GB-COH-4100611), for "DDM Health ltd". Unit 466's register review already judged that number wrong for DDM: "04100611 is NORWICH CITY COMMUNITY SPORTS FOUNDATION, a football charity company with no link to DDM Health", correct 07975193, applied. That mention now serves 30953794, and the tender's winner is DDM Health Limited 30953794.
- NCCSF's own appearance (46803407) is as a Tenderer on an unrelated Norfolk alternative-provision framework. There it prints its own number and CHC 1088239.
- Neither notice names a consortium or subcontractor.
- Destination 30953794 "DDM Health Limited" GBCOH07975193 (15 mentions, merged PPON PBZZ7259JWZY). The alternative 31546127 GBCOH7975193 (1 mention) is the same number unpadded. So the tiebreak is between two rows of one entity, and it is harmless. **CONFIRM.**

**188: Diabetes UK on NHS Confederation's number** (46786666, ORG-GB-COH-04358614, Tenderer, tender 8634494)
- Buyer: NHS England (its own PPON section). The procurement is "User involvement in the NHS Diabetes Programme".
- The register gives 04358614 as THE NHS CONFEDERATION. The holder's other 5 mentions are all NHS Confederation, NHS Employers or NHS Alliance spellings.
- Diabetes UK is a separate national charity, not a member, subsidiary or agent of the Confederation. The notice has a sole tenderer and no consortium.
- Destination 31562654 "Diabetes UK" GBPPONPPNH5783YDNJ (3 mentions) is the only identified candidate, and the name is exact.
- r11 also shows provisional 31612228 "Diabetes UK" and 31629163 "Diabetes UK (charity)". These are the same name, and the identified row is the better home.
- No company number is printed to cross-check. The PPON row's self-registered name is exact. **CONFIRM.**

**195: Lincolnshire Housing Partnership on Mobysoft's number** (4 mentions, ORG-GB-COH-04546648, Procedure-Buyer on all)
- 46792286: Allpay Electronic Payment Services (tender 8637949).
- 46796698: Azure subscriptions and service support (8643380).
- 46796727 and 47297256: Security Operations Centre. These are two notices of the same procedure ocds-h6vhtk-0666ef (8643395).
- The register gives 04546648 as MOBYSOFT LIMITED, a housing-software SaaS supplier.
- LHP is unmistakably the buyer in every notice. The texts read "pay LHP rent and charges", "LHP are require Azure hosting" and "secure LHP's DATA". The Tenderers (Allpay, Jungle IT, Phoenix Software) each have their own sections, and the Lot-ReviewOrg is a separate unidentified "Lincolnshire Housing Partnership" section (FTS-38715).
- Mobysoft is at most LHP's software vendor. That is no legal, group, agent or consortium relation, and Mobysoft runs none of these procurements.
- Rule 4 does not bite. The Procedure-Buyer role moves from a supplier holder to a housing association, which is the buyer the notices show.
- Destination 30917202 "LINCOLNSHIRE HOUSING PARTNERSHIP LIMITED" GBCOHRS007748 (43 mentions). The register gives RS007748 as LINCOLNSHIRE HOUSING PARTNERSHIP LIMITED. The alternative 31534971 GBMPRRS007748 (3) is the same society number under the MPR scheme. The tiebreak is between two rows of one entity. **CONFIRM, all 4 mentions.**

**200: Land Use Consultants on Enventure Research's number** (46762813, ORG-GB-COH-04693096, Tenderer, tender 8620339)
- Buyer: Luton Council (its own PPON section). The procurement is "BT1810 - Infrastructure Strategy Study", a planning-evidence consultancy with a sole tenderer.
- Enventure Research is a market-research firm. Nothing in the notice suggests a joint bid, and LUC is the one named supplier.
- Destination 15715155 "Land Use Consultants Ltd" 02549296 (39 mentions, merged PPON PQGG8255HYBN). The register gives 02549296 as LAND USE CONSULTANTS LIMITED, Active.
- The pool alternative 7799672 "2549296" (3 mentions) is the same number. 31593929 GBCOH2549296 "Land Use Consultants Ltd (LUC) (appointed to Lot 10)" (1 mention) is also the same number and missing from the pool, which is harmless. All three are one entity, and the destination is the best of them. **CONFIRM.**

**302: TPS Fruit & Veg Ltd on Explorance's number** (46961181, ORG-GB-COH-08987402, Tenderer on lot 2 "Aberdeen & Grampians", tender 8740716)
- Buyer: APUC Limited (its own SC314764 section, Procedure-Buyer and CPB). The procurement is "Fresh Fruit & Vegetables (APUC)".
- Explorance Global is a course-evaluation software company. Its other 3 mentions are as itself, presumably on APUC's software framework. It has no relation to a produce wholesaler.
- The evidence section binds only TPS. The other six fruit and veg tenderers each have their own sections.
- Destination 21727460 is provisional "TPS Fruit & Veg Ltd" (1 mention, GB). It is the exact printed name and the only exact-name row, so it is where a name-scoped mention lands.
- **Caveat:** r11 also lists provisional 11707140 "TPS Fruit and Veg Ltd" (GB, 2 mentions), the "and" spelling. It is the same entity, not a competing one, so it does not make the choice ambiguous. The two rows should meet in a later provisional merge, not in this rehome. No identified TPS row exists in the scan.
- If the panel reads "only candidate" as "only row of any spelling", this row drops to PARK. I read it as the only exact-name row. **CONFIRM.**

**303: Don Whitley Scientific on Whistlejacket London's number** (46870983, ORG-GB-COH-09021526, Tenderer, tender 8687936)
- Buyer: Countess of Chester Hospital NHS FT (its own NHS and PPON section). The procurement is "Maintenance of Whitley A35 Anaerobic workstations", which is Don Whitley Scientific's own product line.
- Whistlejacket London's other 8 mentions are all its own spellings. "Whistlejacket" sorts directly before "Whitley", which makes a mis-pick in a supplier picker the obvious mechanism. The two firms have no relation.
- Destination 13064119 "Don Whitley Scientific Ltd" 01342672 (4 mentions) is the only identified row. The name agrees once Ltd is folded. 01342672 is not in the local register snapshot.
- r11 also shows 7 provisional Don Whitley rows, including 31596039 with exactly "Don Whitley Scientific". The identified company row is the better home. **CONFIRM.**

**322: University of Northumbria at Newcastle on "TASS College House" (10167191): PARK** (46960679, ORG-GB-COH-10167191, Tenderer, tender 8747330)
- Buyer: The University of Birmingham. The procurement is "Out of Hours IT Helpdesk Managed Service", a negotiated award without call (2023).
- The register (452 case 31568585:10167191) gives 10167191 as **NORTHUMBRIA UNIVERSITY SERVICES LIMITED**, a company controlled by Northumbria University.
- The 452 review recorded verdict `related` (medium). Its rationale: TASS is a programme hosted by the university, with no company of its own, so the number belongs to the host university's subsidiary. The holder serves `identifier_status: related_entity` on the API.
- So the printed name (the university) is the parent of the company that owns the printed number, and the holder (TASS) is a programme of that same university. That is exactly the parent/subsidiary/hosted relation that rule 1 parks.
- The out-of-hours helpdesk is the kind of commercial service a university trading subsidiary contracts for. So the legal tenderer may well be Northumbria University Services Ltd itself, not the university.
- The destination side is also unsettled. Besides 31541654 (PPON PNBP4638XHJP, 37 mentions) and 31631700 (UKPRN 10001282), r11 shows two more identified university rows with other PPONs: 30930101 "Northumbria University" GBPPONPCCM8851PBTR and 31548807 "Northumbria University at Newcastle" GBPPONPJMW5792YVBL. It also shows a provisional "NORTHUMBRIA UNIVERSITY DIGITAL SERVICES LIMITED" 31564673.
- **PARK, keep.** The right fix belongs to 452/466's related-number path: re-key the holder off 10167191, then decide whether this mention is the university or its services company.

**347: Action Deafness on Angel and Star Care Services' number** (46874082, ORG-GB-COH-11618190, Tenderer on lot 3 "High Needs", tender 8683366)
- Buyer: Derby City Council. The procurement is the "Flexible Open Framework for the Provision of Supported Living Services", with 35 awards.
- The notice prints 11618190 twice, in two distinct sections:
  - `ORG-GB-COH-11618190` holds "Action Deafness", the evidence section.
  - `ORG-GB-COH-11618190#2` holds "Angel and Star Care Services".
  - Each is bound by its own TenderingParty, so the evidence section carries only Action Deafness.
- The number's holder, Angel and Star, keeps its own mention from section #2. "Action..." sorts directly before "Angel...", the same mis-pick shape as Whitley.
- Action Deafness is a Leicester deaf-community charity company. The register gives 06706042 as ACTION DEAFNESS, Active. It has no relation to a small care agency.
- Destination 31540259 "ACTION DEAFNESS" GBCOH06706042 (12 mentions, merged PPON PTBZ5663ZVYN). 31648790 GBCOH6706042 is the same number.
- 31619666 GBCHC1163758 "Action Deafness" (1 mention) is a charity-number row. I could not tie it to the company from here. Even if it were a successor CIO, the company row holds the current PPON and the register shows it Active. So the most-referenced company row is the sound home. **CONFIRM.**

## Notes for the panel
- 182 and 347 both have independent support for the destination. For 182 it is the 466 register review of the sibling notice. For 347 it is the register name for 06706042.
- 302's provisional sibling 11707140 and 200's unlisted 31593929 are same-entity splits that the rehome neither causes nor worsens.
- 322 is the only park. It is a related-entity number (452 verdict `related`), not a stray name.
