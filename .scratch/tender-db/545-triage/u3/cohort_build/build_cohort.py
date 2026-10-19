"""Issue 545 unit 3: build cohort.json and touched_tenders.json from the six batch reviews,
the six adversarial (refute) passes and the guard review.

Inputs (read-only): ../../u2a/verdicts.json, ../../u2b/verdicts2b.json, ../bodies/*.json (saved
public-API bodies). The per-row reviewer / skeptic outcomes below are transcribed from the batch
results the orchestrator handed to unit 3 (the same outcomes as review_N.md / refute_N.md).

Rules applied (mechanically, in `decide`):
  rehome/high  only if the row is CONFIRM (or CHANGE-DESTINATION, using new_destination), the skeptic
               did not refute the row or the mention, and the reviewer marked the mention rehome/high
               with a target_org_id;
  keep         every other reviewed mention (no target_org_id, target_name null), confidence as the
               reviewer gave it, except a skeptic-refuted mention, which takes the skeptic's downgrade
               (JBA: keep, medium);
  additions    guard-review additions go in only when the guard reviewer marked them high; they are
               held to the same bar (rehome/high/target) and tagged separately.
Run: python3 -I build_cohort.py   (from anywhere; paths are absolute)
"""
import json
import os
import sys

ROOT = "/home/user/tender-db/.scratch/tender-db/545-triage"
U3 = ROOT + "/u3"
BODIES = U3 + "/bodies"

R = "rehome"
K = "keep"

# (batch, holder org, printed name, row verdict, new destination, row refuted,
#  [(notice, section, reviewer action, reviewer confidence, reviewer target, mention refuted, rationale)])
ROWS = [
 # ---------------- batch 1 ----------------
 (1, 30914265, "Abbott Medical UK Limited", "CONFIRM", None, False, [
  (46791071, "ORG-GB-PPON-PBVJ-9475-ZCMD", R, "high", 11546762, False,
   "Notice 46791071 (NNUH, Claris EP system maintenance) prints 'Abbott Medical UK Limited' alone in this section with Mid and South Essex NHS FT's PPON, referenced once as Tenderer, so the bid belongs on Abbott's own row 02423907 (Abbott PPON merged)."),
 ]),
 (1, 30914406, "Thomas Silvey", "CONFIRM", None, False, [
  (46842097, "ORG-GB-PPON-PBZB-4962-TVLR", R, "high", 31560264, False,
   "Notice 46842097 (NHS Cornwall ICB call-off from CCS Fuel Cards VII Lot 1) prints 'Thomas Silvey' alone with Crown Commercial Service's PPON as the single Tenderer, and CCS is only the framework authority, so the bid belongs on THOMAS SILVEY,LIMITED (GBCOH00360922)."),
 ]),
 (1, 31560765, "TIAA LTD", "CONFIRM", None, False, [
  (46818192, "ORG-GB-PPON-PHGQ-9419-WJLV", R, "high", 11853802, False,
   "Notice 46818192 (NHS England, External Investigation HR Services) prints 'TIAA LTD' alone with HFMA's PPON PHGQ-9419-WJLV as the single Tenderer; TIAA has its own PPON row and every pool row is TIAA Limited 04546319, so the most-referenced row 11853802 takes it."),
 ]),
 (1, 30916588, "NLA MEDIA ACCESS LIMITED", "CONFIRM", None, False, [
  (46834269, "ORG-GB-PPON-PLBJ-4721-TQQJ", R, "high", 30915240, False,
   "Notice 46834269 (National Highways NLA licence renewal) prints 'NLA MEDIA ACCESS LIMITED' alone with the Valuation Office Agency's PPON as the single Tenderer; VOA has no part in the notice and NLA's own PPON PLMM-3896-RGZM is merged on 30915240."),
 ]),
 (1, 30929662, "PPL/PRS", "CONFIRM", None, False, [
  (46763175, "ORG-GB-PPON-PLGY6381WWWR", R, "high", 31546963, False,
   "Notice 46763175 (SEHSCT buys TheMusicLicence) prints 'PPL/PRS' alone in the unhyphenated section ORG-GB-PPON-PLGY6381WWWR with SEHSCT's PPON as Tenderer, while SEHSCT itself is Procedure-Buyer in the separate hyphenated section, so only the Tenderer moves, to PPL PRS ltd (GBCOH10376001)."),
 ]),
 (1, 30914046, "FIRE SERVICE COLLEGE LIMITED", "CONFIRM", None, False, [
  (46811712, "ORG-GB-PPON-PLTC-5358-LTWZ", R, "high", 11314818, False,
   "Notice 46811712 (UKRI and UK SBS buy aircraft RFFS training) prints 'FIRE SERVICE COLLEGE LIMITED' alone with DfT's PPON as the single Tenderer; DfT has no relation to FSC, and the most-referenced FSC row 11314818 (08102633) takes it."),
 ]),
 (1, 30913583, "Red Kite Veterinary Consultants Ltd", "CONFIRM", None, False, [
  (46868149, "ORG-GB-PPON-PNBD-3289-CDGX", R, "high", 31578178, False,
   "Notice 46868149 (APHA buyer) prints 'Red Kite Veterinary Consultants Ltd' alone with DEFRA's PPON as the single Tenderer; DEFRA is APHA's parent, not Red Kite's, so the bid belongs on RED KITE VETERINARY CONSULTANTS LIMITED (GBCOH08716366)."),
 ]),
 (1, 31549426, "CAMBRIDGE UNIVERSITY HOSPITALS NHS FOUNDATION TRUST", "PARK", None, False, [
  (46815909, "ORG-GB-PPON-PPCP-8863-HDGN", K, "high", None, False,
   "Parked row: on its own ITT 46815909 CUH prints GB-PPON-PPCP-8863-HDGN beside GB-NHS-RGT in its buyer section ORG-GB-NHS-RGT, so the printed name is the identifier's owner, not a different entity; the fix is an identifier verdict giving PPCP-8863-HDGN to CUH, not a 545 rehome."),
 ]),
 (1, 31549426, "Cambridge University Hospitals NHS Foundation Trust", "PARK", None, False, [
  (46796632, "ORG-GB-PPON-PPCP-8863-HDGN", K, "high", None, False,
   "Parked row: notice 46796632 prints 'Cambridge University Hospitals NHS Foundation Trust' with PPCP-8863-HDGN as Procedure-Buyer, a second CUH self-use of that PPON, so the name is the identifier's owner and the fix is an identifier verdict, not a 545 rehome."),
 ]),
 # ---------------- batch 2 ----------------
 (2, 31549426, "Hologic Ltd", "CONFIRM", None, False, [
  (46877124, "ORG-GB-PPON-PPCP-8863-HDGN", R, "high", 30914006, False,
   "Notice 46877124 (BHRUT, MyoSure/Fluent consumables) prints 'Hologic Ltd' alone with PPCP-8863-HDGN, referenced only by OPT-300-Tenderer, and Hologic's own PPON PZBR-3269-JJND is merged on HOLOGIC LTD. (GBCOH02722343)."),
  (46882065, "ORG-GB-PPON-PPCP-8863-HDGN", R, "high", 30914006, False,
   "Notice 46882065 (UK7 of tender 8690126) prints the same single-party Tenderer section, 'Hologic Ltd' on PPCP-8863-HDGN, with the buyer BHRUT in its own section."),
 ]),
 (2, 31538012, "Renishaw Neuro Solutions Limited", "CONFIRM", None, False, [
  (46795678, "ORG-GB-PPON-PPYL-1513-MZZL", R, "high", 31552561, False,
   "Notice 46795678 (Sheffield Teaching Hospitals, Neurosurgery Robot) prints 'Renishaw Neuro Solutions Limited' alone with Great Ormond Street's PPON as the single Tenderer, so the bid belongs on the exact-name company row GBCOH10375761."),
  (46819442, "ORG-GB-PPON-PPYL-1513-MZZL", R, "high", 31552561, False,
   "Notice 46819442 (UK6 of tender 8642804) prints the same single-party Tenderer section, 'Renishaw Neuro Solutions Limited' on GOSH's PPON, with the buyer in its own section."),
 ]),
 (2, 30913740, "Xiel", "CONFIRM", None, False, [
  (46835667, "ORG-GB-PPON-PQBY-8547-GYGP", R, "high", 30974378, False,
   "Notice 46835667 (BCUHB/NWSSP, C-Rad Catalyst maintenance) prints 'Xiel' alone with Northampton General Hospital's PPON as the single Tenderer, and XIEL LTD (GBCOH11875702) carries Xiel's own PPON PGVW-7219-MLVN."),
 ]),
 (2, 30916247, "Tech Against Terrorism", "CHANGE-DESTINATION", 30973170, False, [
  (47157838, "ORG-GB-PPON-PQQZ-1663-NWMN", R, "high", 30973170, False,
   "Notice 47157838 (Home Office) prints 'Tech Against Terrorism' alone with Ofcom's PPON as the single Tenderer, and the Home Office's corrected re-publications 46697744/46703079 of tender 8582476 print the same tenderer as 'Online Harms Foundation TA/ Tech Against Terrorism' with GB-COH-11656320, which is row 30973170 (re-destined from provisional 31570425)."),
 ]),
 (2, 31571667, "ERMC Ltd", "CONFIRM", None, False, [
  (46819437, "ORG-GB-PPON-PRBY-9272-TQGX", R, "high", 18928807, False,
   "Notice 46819437 (Isle of Wight Council, Westminster House QS and design) prints 'ERMC Ltd' alone with PRBY-9272-TQGX as the single Tenderer, so the bid moves off the NHS-trust holder to the only ERMC row, ERMC Limited (04489340)."),
 ]),
 (2, 30926693, "Vohkus", "CONFIRM", None, False, [
  (46771116, "ORG-GB-PPON-PXLD-1125-PGBY", R, "high", 31551980, False,
   "Notice 46771116 (GMC, IS Commodity Items) prints 'Vohkus' alone with GMC's PPON PXLD-1125-PGBY as the single Tenderer while GMC buys under GB-CHC-1089278, and VOHKUS LIMITED (GBCOH04142508) carries Vohkus's own PPON."),
 ]),
 (2, 30913561, "Bolton NHS Foundation Trust", "CONFIRM", None, False, [
  (47366319, "ORG-GB-PPON-PXQG-2592-VYYY", R, "high", 30917930, False,
   "Notice 47366319 (The Christie, 'Surv Endoscopy (Bolton)') prints 'Bolton NHS Foundation Trust' alone with Manchester University NHS FT's PPON as the single Tenderer, so the bid belongs on Bolton FT's row 30917930 (merged GB-NHS-RMC)."),
 ]),
 (2, 31551518, "Gama Healthcare Ltd", "PARK", None, False, [
  (46842943, "ORG-GB-PPON-PYMW-5475-ZMBP", K, "medium", None, False,
   "Parked row: notice 46842943 (NWSSP, Surface Cleaning Wipes) prints 'Gama Healthcare Ltd' alone on Finsen's PPON, but a Finsen-GAMA distributor or group relation in the same infection-prevention niche cannot be ruled out from the permitted reads."),
 ]),
 (2, 30913648, "Virgin Media Limited", "PARK", None, False, [
  (47157734, "ORG-GB-PPON-PYVZ-4215-PGYM", K, "medium", None, False,
   "Parked row: notice 47157734 prints 'Virgin Media Limited' alone on DVLA's PPON, but the only destination row 31540412 is keyed GBCOH06024812, which does not agree with Virgin Media Limited (02591237), so there is no sound destination."),
 ]),
 # ---------------- batch 3 ----------------
 (3, 30926847, "Compendium Regeneration Limited", "CONFIRM", None, False, [
  (46853619, "ORG-GB-PPON-PZLB-9446-RVML", R, "high", 1307252, False,
   "Notice 46853619 (Thirteen Housing Group) prints 'Compendium Regeneration Limited' alone with the magnet PPON PZLB-9446-RVML (Network Rail's row) as the single Tenderer, so the bid moves to the only exact-name GB row, provisional 1307252."),
 ]),
 (3, 30926847, "RSS International Ltd", "CONFIRM", None, False, [
  (46709309, "ORG-GB-PPON-PZLB-9446-RVML", R, "high", 30929573, False,
   "Notice 46709309 (National Highways, 'M5 J20-21 St George's Rail Deck - RSS Int.') prints 'RSS International Ltd' alone with PZLB-9446-RVML as the single Tenderer, so the bid belongs on RSS INTERNATIONAL LTD (GBCOH06707779)."),
 ]),
 (3, 30926847, "The Law Society", "CONFIRM", None, False, [
  (46855540, "ORG-GB-PPON-PZLB-9446-RVML", R, "high", 14894295, False,
   "Notice 46855540 (National Highways, 'The Law Society - Practising Certificates (SRA)') prints 'The Law Society' alone with PZLB-9446-RVML as the single Tenderer, so the bid moves to the only exact-name row, provisional 14894295."),
 ]),
 (3, 30915227, "SWANSEA BAY UNIVERSITY HEALTH BOARD", "CONFIRM", None, False, [
  (46784746, "ORG-GB-PPON-PZZV-3765-WXZQ", R, "high", 30916810, False,
   "Notice 46784746 (OR1 Theatre upgrade) prints 'SWANSEA BAY UNIVERSITY HEALTH BOARD' alone with PZZV-3765-WXZQ (Storz Medical's row) as Procedure-Buyer, while NWSSP and the Tenderer Karl Storz Endoscopy have their own sections, so the buyer role moves to the board's row 30916810 (GBNHS7A3)."),
 ]),
 (3, 30913706, "Jeol (U.K.) Limited", "CONFIRM", None, False, [
  (46834978, "ORG-GB-PPON-PDQJ-7126-JDHG", R, "high", 12991212, False,
   "Notice 46834978 (UKHSA, 'Service and Maintenance x JEOL') prints 'Jeol (U.K.) Limited' alone with UKRI's PPON PDQJ-7126-JDHG as the single Tenderer, so the bid belongs on JEOL (U.K.) Limited 12991212 (00939456, JEOL's own PPON merged)."),
 ]),
 (3, 31549890, "Pensions Policy Institute", "PARK", None, False, [
  (46807946, "ORG-GB-PPON-PCLN-8377-ZPPT", K, "medium", None, False,
   "Parked row: notice 46807946 (The Pensions Regulator, 'Retirement Saving Research Consortium') prints 'Pensions Policy Institute' on the Institute for Fiscal Studies' PPON, and the consortium title points at IFS as the likely supplier or lead, so a consortium relation cannot be excluded."),
 ]),
 (3, 30914626, "Equal Education Partners ", "CONFIRM", None, False, [
  (47293389, "ORG-GB-PPON-PDNG-5431-HGYV", R, "high", 30928130, False,
   "Notice 47293389 (Blaenau Gwent CBC, home tuition) prints 'Equal Education Partners ' alone with Torfaen CBC's PPON as the single Tenderer, so the bid moves to Equal Education Partners Limited 30928130 (both identified EEP rows are one company)."),
 ]),
 (3, 30913629, "Portsmouth Hospitals University NHS Trust", "CONFIRM", None, False, [
  (46832653, "ORG-GB-PPON-PDPT-3135-BWWY", R, "high", 30971975, False,
   "Notice 46832653 (Isle of Wight Council, Smokefree Lead Role 'hosted by Portsmouth Hospitals University NHS Trust') prints PHUT alone with NHS England's PPON as the single Tenderer, so the bid belongs on PHUT's own PPON row 30971975."),
 ]),
 (3, 30916371, "WATKINS COACHES LTD", "PARK", None, False, [
  (46755521, "ORG-GB-PPON-PDXZ-9391-VHZQ", K, "high", None, False,
   "Parked row: notice 46755521 prints 'WATKINS COACHES LTD' alone on Rhondda Cynon Taf CBC's PPON, but two GB provisional destinations exist (2479435 'Watkins Coaches Ltd', 8878703 'Watkins Coaches') and no identified row settles which, so the destination is not unique."),
 ]),
 # ---------------- batch 4 ----------------
 (4, 30913753, "WAVEMAKER LIMITED t/a WPP 650", "CONFIRM", None, False, [
  (46697985, "ORG-GB-PPON-PDNN-2773-HVYN", R, "high", 31555041, False,
   "Notice 46697985 (UKHSA call-off 'awarded to Wavemaker Limited t/a WPP650' under RM6364 Lot 1) prints 'WAVEMAKER LIMITED t/a WPP 650' alone with the Ministry of Justice's PPON as the single Tenderer, so the bid belongs on WAVEMAKER LIMITED (GBCOH04078547)."),
  (47158784, "ORG-GB-PPON-PDNN-2773-HVYN", R, "high", 31555041, False,
   "Notice 47158784 (v1 of tender 8582477) prints the same single-party Tenderer section, 'WAVEMAKER LIMITED t/a WPP 650' on MoJ's PPON, with UKHSA in its own section."),
 ]),
 (4, 30915848, "Pullman Rail Ltd", "CONFIRM", None, False, [
  (46734421, "ORG-GB-PPON-PPHD-4211-XYXJ", R, "high", 31575835, False,
   "Notice 46734421 (TfW Rail, Class 230 Monoblock Wheel Pans, v1) prints 'Pullman Rail Ltd' alone with Transport for Wales' PPON as the single Tenderer while TfW Rail buys in its own section PVJP-5451-JYWQ, so the bid belongs on Pullman Rail Ltd (GBCOH02820938)."),
  (46734752, "ORG-GB-PPON-PPHD-4211-XYXJ", R, "high", 31575835, False,
   "Notice 46734752 (v2 of tender 8603276) prints the same single-party Tenderer section, 'Pullman Rail Ltd' on TfW's PPON."),
  (46739172, "ORG-GB-PPON-PPHD-4211-XYXJ", R, "high", 31575835, False,
   "Notice 46739172 (v3 award, 'Existing stock available from Pullman Rail') prints the same single-party Tenderer section, 'Pullman Rail Ltd' on TfW's PPON."),
 ]),
 (4, 30926961, "HOSPITAL INNOVATIONS LIMITED", "CONFIRM", None, False, [
  (46830621, "ORG-GB-PPON-PQDY-1678-DHXN", R, "high", 30974714, False,
   "Notice 46830621 (NWSSP, Cryopreserved Skin - Cryoskin, v1) prints 'HOSPITAL INNOVATIONS LIMITED' alone with Swansea Bay UHB's PPON as the single Tenderer, so the bid belongs on Hospital Innovations Limited (GBCOH4261709)."),
  (46840995, "ORG-GB-PPON-PQDY-1678-DHXN", R, "high", 30974714, False,
   "Notice 46840995 (v2 of tender 8662629) prints the same single-party Tenderer section, 'HOSPITAL INNOVATIONS LIMITED' on Swansea Bay UHB's PPON."),
  (46847682, "ORG-GB-PPON-PQDY-1678-DHXN", R, "high", 30974714, False,
   "Notice 46847682 (v3 award) prints the same single-party Tenderer section, 'HOSPITAL INNOVATIONS LIMITED' on Swansea Bay UHB's PPON."),
 ]),
 (4, 31537076, "Monthind Clean Ltd", "PARK", None, False, [
  (46819477, "ORG-GB-PPON-PDHL-3945-QCTM", K, "high", None, False,
   "Parked row: notice 46819477 prints 'Monthind Clean Ltd' on Cotswold Archaeology's own PPON for a conservation-area appraisal consultancy, and v3 of the same tender (46822563) prints 'Cotswold Archaeology Ltd' on the same section, so the name, not the identifier, is the error."),
 ]),
 (4, 31580163, "Generator Power Ltd", "CONFIRM", None, False, [
  (46982353, "ORG-GB-COH-02366923", R, "high", 31550198, False,
   "Notice 46982353 (WPD, LV Generator Hire) prints 'Generator Power Ltd' alone in section ORG-GB-COH-02366923, a number lifted from the buyer's own multi-number id string, as Tenderer on all 4 lots, so the bids belong on GENERATOR POWER LIMITED (GBCOH03446582)."),
 ]),
 (4, 30914453, "Ballicom Limited", "CONFIRM", None, False, [
  (46871836, "ORG-GB-COH-02436537#2", R, "high", 30917190, False,
   "Notice 46871836 (South Tyneside IT hardware framework) prints 'Ballicom Limited' alone in the unreferenced section ORG-GB-COH-02436537#2 with Getech's number while it prints Ballicom twice with 02799428, the destination's number, so this names-only move takes the name off Getech."),
 ]),
 (4, 31535435, "Big Life", "CONFIRM", None, False, [
  (46806034, "ORG-GB-PPON-PJNQ-4269-GYNV", R, "high", 31570142, False,
   "Notice 46806034 (MFT, 'Big Life are the provider of ... BeWell') prints 'Big Life' alone with CAM's PPON as the single Tenderer, and MFT's later versions of the same tender print Big Life on PPMM-8322-CXDT, the destination's own PPON."),
 ]),
 (4, 31552420, "Jeremy Benn Associates Limited", "CONFIRM", None, True, [
  (46772495, "ORG-GB-COH-02917907", R, "high", 10270770, True,
   "Refuted by the skeptic: notice 46772495 (OEP111-01, single version) prints 'Jeremy Benn Associates Limited' with eftec's number 02917907, but no text names the supplier and eftec, which the same buyer had awarded the day before, is a plausible winner, so the printed name cannot be shown to be the right field."),
 ]),
 (4, 30916544, "TSI Instruments Ltd", "CONFIRM", None, False, [
  (46792203, "ORG-GB-PPON-PYDB-8862-JBYM", R, "high", 31581156, False,
   "Notice 46792203 (YAS, TSI Quantitative Fit Tester, v1) prints 'TSI Instruments Ltd' alone with the National Physical Laboratory's PPON as the single Tenderer, so the bid belongs on TSI INSTRUMENTS LTD (GBCOH04486008)."),
  (46792211, "ORG-GB-PPON-PYDB-8862-JBYM", R, "high", 31581156, False,
   "Notice 46792211 (v2 of tender 8637891) prints the same single-party Tenderer section, 'TSI Instruments Ltd' on NPL's PPON."),
  (46796560, "ORG-GB-PPON-PYDB-8862-JBYM", R, "high", 31581156, False,
   "Notice 46796560 (v3 award, 'TSI Portacount Machine') prints the same single-party Tenderer section, 'TSI Instruments Ltd' on NPL's PPON."),
 ]),
 # ---------------- batch 5 ----------------
 (5, 30915240, "Prosci UK Limited", "CONFIRM", None, False, [
  (46768774, "ORG-GB-PPON-PLMM-3896-RGZM", R, "high", 31575934, False,
   "Notice 46768774 (NHSBSA, Enterprise Change Management Services) prints 'Prosci UK Limited' alone with NLA Media Access's merged PPON PLMM-3896-RGZM as the single Tenderer, so the bid belongs on Prosci Uk Ltd (PPON PPJX-4251-TMBX)."),
 ]),
 (5, 31568872, "DDM Health Ltd", "CONFIRM", None, False, [
  (46890437, "ORG-GB-COH-04100611", R, "high", 30953794, False,
   "Notice 46890437 (Norfolk CC) prints 'DDM Health Ltd' alone with Norwich City Community Sports Foundation's number 04100611 as the single Tenderer, and the same tender's award notice 46893016 already serves DDM on 30953794 (GBCOH07975193)."),
 ]),
 (5, 30926702, "Diabetes UK", "CONFIRM", None, False, [
  (46786666, "ORG-GB-COH-04358614", R, "high", 31562654, False,
   "Notice 46786666 (NHS England, User involvement in the NHS Diabetes Programme) prints 'Diabetes UK' alone with The NHS Confederation's number 04358614 as the single Tenderer, so the bid belongs on Diabetes UK's PPON row 31562654."),
 ]),
 (5, 31545104, "LINCOLNSHIRE HOUSING PARTNERSHIP LIMITED", "CONFIRM", None, False, [
  (46792286, "ORG-GB-COH-04546648", R, "high", 30917202, False,
   "Notice 46792286 (LHP buys Allpay Electronic Payment Services) prints 'LINCOLNSHIRE HOUSING PARTNERSHIP LIMITED' alone with Mobysoft's number 04546648 as Procedure-Buyer, with the Tenderer Allpay in its own section, so the buyer role moves to LHP's row 30917202 (RS007748)."),
  (46796698, "ORG-GB-COH-04546648", R, "high", 30917202, False,
   "Notice 46796698 (LHP, Azure subscriptions and service support) prints LHP alone with Mobysoft's number 04546648 as Procedure-Buyer, with the Tenderer Jungle IT in its own section."),
  (46796727, "ORG-GB-COH-04546648", R, "high", 30917202, False,
   "Notice 46796727 (LHP, Security Operations Centre) prints LHP alone with Mobysoft's number 04546648 as Procedure-Buyer, with the Tenderer Phoenix Software in its own section."),
  (47297256, "ORG-GB-COH-04546648", R, "high", 30917202, False,
   "Notice 47297256 (second notice of the SOC procedure, tender 8643395) prints the same single-party Procedure-Buyer section, LHP on Mobysoft's number 04546648."),
 ]),
 (5, 31551788, "LAND USE CONSULTANTS LIMITED", "CONFIRM", None, False, [
  (46762813, "ORG-GB-COH-04693096", R, "high", 15715155, False,
   "Notice 46762813 (Luton Council, BT1810 Infrastructure Strategy Study) prints 'LAND USE CONSULTANTS LIMITED' alone with Enventure Research's number 04693096 as the single Tenderer, so the bid belongs on Land Use Consultants Ltd 15715155 (02549296)."),
 ]),
 (5, 31540233, "TPS Fruit & Veg Ltd", "CONFIRM", None, False, [
  (46961181, "ORG-GB-COH-08987402", R, "high", 21727460, False,
   "Notice 46961181 (APUC Fresh Fruit & Vegetables, lot 2) prints 'TPS Fruit & Veg Ltd' alone with Explorance Global's number 08987402 as Tenderer, so the bid moves to the only exact-name row, provisional 21727460."),
 ]),
 (5, 30930397, "Don Whitley Scientific", "CONFIRM", None, False, [
  (46870983, "ORG-GB-COH-09021526", R, "high", 13064119, False,
   "Notice 46870983 (Countess of Chester, maintenance of Whitley A35 anaerobic workstations) prints 'Don Whitley Scientific' alone with Whistlejacket London's number 09021526 as the single Tenderer, so the bid belongs on Don Whitley Scientific Ltd (01342672)."),
 ]),
 (5, 31568585, "University of Northumbria at Newcastle", "PARK", None, False, [
  (46960679, "ORG-GB-COH-10167191", K, "high", None, False,
   "Parked row: notice 46960679 prints 'University of Northumbria at Newcastle' with 10167191, Northumbria University Services Limited, the university's own subsidiary, so name and number are parent and subsidiary, which parks."),
 ]),
 (5, 31549084, "Action Deafness", "CONFIRM", None, False, [
  (46874082, "ORG-GB-COH-11618190", R, "high", 31540259, False,
   "Notice 46874082 (Derby supported-living framework) prints 'Action Deafness' alone in section ORG-GB-COH-11618190 with Angel and Star Care Services' number while Angel and Star has its own section ORG-GB-COH-11618190#2, so the lot-3 bid belongs on ACTION DEAFNESS (GBCOH06706042)."),
 ]),
 # ---------------- batch 6 ----------------
 (6, 30915302, "Eastern Counties Scaffodling Services", "CONFIRM", None, False, [
  (46810761, "ORG-GB-PPON-PWTD-8252-JQTZ", R, "high", 31573542, False,
   "Notice 46810761 (Norwich City Services Ltd buys scaffolding) prints 'Eastern Counties Scaffodling Services' alone with PWTD-8252-JQTZ as the single Tenderer while NCSL is Procedure-Buyer in its own section ORG-GB-COH-12656036, so the bid moves to the identically spelled row 31573542 (own PPON PHGP-1225-WWPN)."),
 ]),
 (6, 31597395, "Crown Paints Ltd", "CONFIRM", None, False, [
  (46951039, "ORG-GB-COH-436135", R, "high", 10890039, False,
   "Notice 46951039 (EEM decorating framework) prints 'Crown Paints Ltd' alone in section ORG-GB-COH-436135 (PPG's number without its leading zeros) as a lot-1 Tenderer while PPG has its own section and Crown appears again under its own number 6624576, so the bid belongs on Crown Paints Ltd 10890039 (06624576)."),
 ]),
 (6, 31632003, "Thomas Bow Limited", "CONFIRM", None, False, [
  (46980862, "ORG-GB-COH-703317#2", R, "high", 14183429, False,
   "Notice 46980862 (Derby highways framework) prints 'Thomas Bow Limited' alone in section ORG-GB-COH-703317#2 with Telent's number as the lot-6 Tenderer while Telent has its own section and Thomas Bow appears under 4503934, so the bid belongs on Thomas Bow Ltd 14183429 (04503934)."),
 ]),
 (6, 31582801, "Dell Corporation", "CONFIRM", None, False, [
  (46844315, "ORG-GB-COH-OC403062", R, "high", 31535972, False,
   "Notice 46844315 (MOD, Dell hardware support: 'Direct engagement with Dell ... Dell supplied a quote') prints 'Dell Corporation' alone with Hand and Millar's number OC403062 as the single Tenderer, so the bid belongs on DELL CORPORATION LIMITED (GBCOH02081369)."),
 ]),
 (6, 31548301, "West Yorkshire Combined Authority", "CONFIRM", None, False, [
  (46794970, "ORG-GB-COH-OC451571", R, "high", 30913749, False,
   "Notice 46794970 (Sussex PCC: 'Direct award under a national agreement contract with PNLD with the West Yorkshire Combined Authority') prints WYCA alone with Bristol Temple Quarter LLP's number OC451571 as the single Tenderer, so the bid belongs on WYCA's PPON row 30913749."),
 ]),
 (6, 30914001, "University of Liverpool", "CONFIRM", None, False, [
  (46839286, "ORG-GB-PPON-PHVT-4343-HWYR", R, "high", 30953698, False,
   "Notice 46839286 (HEIW, MPH Qualification - North: 'intends to award a contract to the University of Liverpool') prints 'University of Liverpool' alone with Cardiff University's PPON as the single Tenderer, so the bid belongs on UNIVERSITY OF LIVERPOOL (RC000660)."),
  (46843620, "ORG-GB-PPON-PHVT-4343-HWYR", R, "high", 30953698, False,
   "Notice 46843620 (UK7: 'awarded the contract to the University of Liverpool') prints the same single-party Tenderer section, 'University of Liverpool' on Cardiff University's PPON."),
 ]),
 (6, 30913587, "Royal College of Psychiatrists", "CONFIRM", None, False, [
  (46798192, "ORG-GB-COH-RC000672", R, "high", 31561689, False,
   "Notice 46798192 (DHSC, Addiction Psychiatry Credential 'delivered by Royal College of Psychiatrists (RCPsych)') prints RCPsych alone with the University of Sussex's charter number RC000672 as the single Tenderer, so the bid belongs on ROYAL COLLEGE OF PSYCHIATRISTS (THE) (RC000463)."),
 ]),
]

# Guard-review additions: (holder org, printed name, [(notice, section, action, confidence, target, tenders, rationale)])
ADDITIONS = [
 (30913851, "London Borough of Hounslow", [
  (46381132, "ORG-GB-PPON-PVLQ-8225-QVHT", R, "high", 30918222, [8639243],
   "Notice 46381132 (CCTV framework 2425-0045) prints 'London Borough of Hounslow' as one of four named borough buyers with Wandsworth's PPON PVLQ-8225-QVHT, referenced once as Procedure-Buyer, and never prints Wandsworth, so the buyer role belongs on Hounslow's own PPON row 30918222."),
  (46381196, "ORG-GB-PPON-PVLQ-8225-QVHT", R, "high", 30918222, [8639243],
   "Notice 46381196 (same tender 8639243, read in unit 3) prints the same single-party Procedure-Buyer section, 'London Borough of Hounslow' on Wandsworth's PPON, and never prints Wandsworth."),
  (46825561, "ORG-GB-PPON-PVLQ-8225-QVHT", R, "high", 30918222, [8639243],
   "Notice 46825561 (same tender 8639243, read in unit 3) prints the same single-party Procedure-Buyer section, 'London Borough of Hounslow' on Wandsworth's PPON, and never prints Wandsworth."),
 ]),
 (31560408, "Wyre Forest District Council", [
  (46826419, "ORG-GB-PPON-PTQY-8812-XHLH", R, "high", 31550990, [8660381],
   "Notice 46826419 ('Lion Joint Venture - SPV - Wyre Forest District Council') prints Wyre Forest DC twice as Procedure-Buyer, once on its own PPON PTVB-7542-QLLX (the destination) and once on Shropshire F&RS's PPON PTQY-8812-XHLH, and never prints Shropshire."),
  (46826428, "ORG-GB-PPON-PTQY-8812-XHLH", R, "high", 31550990, [8660381],
   "Notice 46826428 (same tender 8660381, read in unit 3) prints the same duplicate Wyre Forest DC buyer entry on Shropshire F&RS's PPON beside WFDC on its own PPON PTVB-7542-QLLX, and never prints Shropshire."),
  (46842882, "ORG-GB-PPON-PTQY-8812-XHLH", K, "medium", None, [8677275],
   "Not read: different tender 8677275; the G2 class needs a notice read to lift the park."),
 ]),
 (30975996, "Kingston Upon Hull City Council", [
  (46760078, "ORG-GB-COH-02007321", R, "high", 30914698, [8639942],
   "Notice 46760078 (G-Cloud 14 buy of Artifax event management software) prints the buyer 'Kingston Upon Hull City Council' with supplier Artifax's number 02007321 as the single Procedure-Buyer and 'Hull City Council' as review body, so the buyer role moves to Hull's most-referenced row 30914698."),
 ]),
 (31537611, "Accenture (UK) Ltd", [
  (46768534, "ORG-GB-COH-01317137", R, "high", 15715826, [8623644],
   "Notice 46768534 (Bank of England, DMS Migration) prints 'Accenture (UK) Ltd' alone with Walnut Unlimited's number 01317137 plus an Accenture PPON PNPX-2447-RCXR as the single Tenderer, so the bid moves to the most-referenced Accenture row 15715826 (04757301)."),
  (46788047, "ORG-GB-COH-01317137", R, "high", 15715826, [8595979],
   "Notice 46788047 (CCS Media and Creative Services, read in unit 3) prints 'Accenture (UK) Ltd' alone with 01317137 plus Accenture's PPON PNPX-2447-RCXR as a single lot-5 Tenderer and never prints Walnut."),
  (47286834, "ORG-GB-COH-01317137", R, "high", 15715826, [8595979],
   "Notice 47286834 (CCS Media and Creative Services, read in unit 3) prints 'Accenture (UK) Ltd' alone with 01317137 plus Accenture's PPON PNPX-2447-RCXR as a single lot-5 Tenderer and never prints Walnut."),
 ]),
]


def load(p):
    with open(p) as f:
        return json.load(f)


def org_body(oid):
    p = "%s/v1_organizations_%d.json" % (BODIES, oid)
    if not os.path.exists(p):
        sys.exit("no saved org body for %d" % oid)
    return load(p)


def check_section(notice, section, printed):
    """The section exists under exactly this id, prints exactly this name, and is referenced by
    one party in one role only (several lots of the same party count once)."""
    d = load("%s/v1_notices_%d_content.json" % (BODIES, notice))
    secs = [s for s in d["sections"] if s["section_id"] == section]
    assert len(secs) == 1, (notice, section, len(secs))
    names = {v["value"] for v in secs[0]["values"] if v["field_id"].startswith("BT-500")}
    assert names == {printed}, (notice, section, names, printed)
    roles = set()
    for s in d["sections"]:
        for v in s["values"]:
            if v.get("is_ref") and v.get("value") == section:
                roles.add(v["field_id"])
    assert len(roles) <= 1, (notice, section, roles)
    return sorted(roles)


def main():
    evid = {}
    for f in (ROOT + "/u2a/verdicts.json", ROOT + "/u2b/verdicts2b.json"):
        for r in load(f):
            if r.get("verdict") != "REHOME-HIGH":
                continue
            for e in r["evidence"]:
                evid[(r["org"], r["name"], e["notice"], e["section"])] = (r, e)
    assert len(evid) == 65, len(evid)

    out, rec, seen = [], [], set()

    def emit(org, notice, section, action, conf, tgt, why, src, batch, printed, tenders, rowv):
        key = (notice, section, org)
        assert key not in seen, key
        seen.add(key)
        assert action in ("rehome", "keep") and conf in ("high", "medium", "low")
        roles = check_section(notice, section, printed)
        tname = None
        if action == "rehome":
            assert tgt is not None and conf == "high"
            b = org_body(tgt)
            tname = b["name"]
            if b["provisional"]:
                # a provisional destination must carry exactly the printed name
                assert b["name"].strip().lower() == printed.strip().lower(), (tgt, b["name"], printed)
        else:
            tgt = None
        out.append({"org_id": org, "notice_id": notice, "section_id": section, "action": action,
                    "target_org_id": tgt, "target_name": tname, "rationale": why, "confidence": conf})
        rec.append({"src": src, "batch": batch, "row_verdict": rowv, "org_id": org, "printed": printed,
                    "notice_id": notice, "section_id": section, "action": action, "confidence": conf,
                    "target_org_id": tgt, "target_name": tname, "tenders": tenders, "roles": roles})

    used = set()
    for batch, org, name, rowv, newd, rrow, ms in ROWS:
        assert rowv in ("CONFIRM", "CHANGE-DESTINATION", "PARK")
        for notice, section, act, conf, tgt, mref, why in ms:
            k = (org, name, notice, section)
            assert k in evid, k
            used.add(k)
            r, e = evid[k]
            ok = (rowv in ("CONFIRM", "CHANGE-DESTINATION") and not rrow and not mref
                  and act == "rehome" and conf == "high" and tgt is not None)
            if ok and rowv == "CHANGE-DESTINATION":
                assert tgt == newd, (k, tgt, newd)
            if ok and rowv == "CONFIRM":
                assert tgt == r["destination"], (k, tgt, r["destination"])
            if ok:
                emit(org, notice, section, "rehome", "high", tgt, why, "batch", batch, name, e["tenders"], rowv)
            else:
                # refuted mention: the skeptic's downgrade (keep, medium); otherwise the reviewer's confidence
                kconf = "medium" if (mref or rrow) else conf
                emit(org, notice, section, "keep", kconf, None, why, "batch", batch, name, e["tenders"],
                     rowv + ("/REFUTED" if (mref or rrow) else ""))
    assert used == set(evid), set(evid) - used

    excluded = []
    for org, name, ms in ADDITIONS:
        for notice, section, act, conf, tgt, tenders, why in ms:
            if conf != "high":
                excluded.append({"org_id": org, "notice_id": notice, "section_id": section,
                                 "action": act, "confidence": conf, "tenders": tenders, "why": why})
                continue
            assert act == "rehome" and tgt is not None
            emit(org, notice, section, "rehome", "high", tgt, why, "addition", "guard", name, tenders, "GUARD-ADD")

    for v in out:
        if v["action"] == "rehome":
            assert isinstance(v["target_org_id"], int) and v["target_name"]
        else:
            assert v["target_org_id"] is None and v["target_name"] is None

    cohort = {"cohort": "545", "verdicts": out}
    with open(U3 + "/cohort.json", "w") as f:
        json.dump(cohort, f, indent=1, ensure_ascii=False)
        f.write("\n")
    touched = sorted({t for r in rec if r["action"] == "rehome" for t in r["tenders"]})
    with open(U3 + "/touched_tenders.json", "w") as f:
        json.dump(touched, f)
        f.write("\n")
    with open(U3 + "/cohort_build/record.json", "w") as f:
        json.dump({"rows": rec, "excluded_additions": excluded}, f, indent=1, ensure_ascii=False)
        f.write("\n")

    # re-read and validate what was written
    c = load(U3 + "/cohort.json")
    assert c["cohort"] == "545"
    keys = [(v["notice_id"], v["section_id"], v["org_id"]) for v in c["verdicts"]]
    assert len(keys) == len(set(keys))
    for v in c["verdicts"]:
        assert set(v) == {"org_id", "notice_id", "section_id", "action", "target_org_id", "target_name",
                          "rationale", "confidence"}
        assert v["action"] in ("rehome", "keep") and v["confidence"] in ("high", "medium", "low")
        if v["action"] == "rehome":
            assert v["target_org_id"] is not None
    t = load(U3 + "/touched_tenders.json")
    assert t == sorted(set(t)) and all(isinstance(x, int) for x in t)
    nre = sum(v["action"] == "rehome" for v in c["verdicts"])
    nadd = sum(r["src"] == "addition" for r in rec)
    print("verdicts", len(c["verdicts"]), "rehome", nre, "keep", len(c["verdicts"]) - nre,
          "additions", nadd, "excluded additions", len(excluded), "touched tenders", len(t))


if __name__ == "__main__":
    main()
