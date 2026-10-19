# 545 unit 3: reviewer 3 of 6 (u2a/verdicts.json rows 200, 202, 204, 207, 208, 210, 212, 213, 215)

Read-only. I used the public API only: 41 GETs, all rc=200 with no retries, each at least 4 s apart under the shared `.get.lock` (`get_r3.sh`). They are logged in `api_requests.log` and the bodies are saved in `bodies/`. The 41 are:
- 9 notice contents, one for every evidence mention;
- 22 organizations: every holder, every destination, the pool alternatives, and 8878703;
- 9 tenders, to confirm how each mention is served today.

I also used u2a's r8 lookup and unit 1's q4 lookup (on disk) for name-range pools. Nothing was POSTed, and I made no SQL, admin, git or ssh reads.

All 9 tenders still serve the printed name under the holder org as Tenderer/winner (or Procedure-Buyer for Swansea), so none of these mentions has been fixed yet.

**Summary: 7 CONFIRM (7 mentions rehome, all high), 2 PARK (2 mentions keep), 0 CHANGE-DESTINATION.**

| idx | holder | printed name | notice / section | role | verdict | target |
|---|---|---|---|---|---|---|
| 200 | 30926847 Network Rail | Compendium Regeneration Limited | 46853619 / ORG-GB-PPON-PZLB-9446-RVML | Tenderer | CONFIRM, rehome high | 1307252 |
| 202 | 30926847 Network Rail | RSS International Ltd | 46709309 / ORG-GB-PPON-PZLB-9446-RVML | Tenderer | CONFIRM, rehome high | 30929573 |
| 204 | 30926847 Network Rail | The Law Society | 46855540 / ORG-GB-PPON-PZLB-9446-RVML | Tenderer | CONFIRM, rehome high | 14894295 |
| 207 | 30915227 Storz Medical (UK) Limited | SWANSEA BAY UNIVERSITY HEALTH BOARD | 46784746 / ORG-GB-PPON-PZZV-3765-WXZQ | Procedure-Buyer | CONFIRM, rehome high | 30916810 |
| 208 | 30913706 UK Research & Innovation | Jeol (U.K.) Limited | 46834978 / ORG-GB-PPON-PDQJ-7126-JDHG | Tenderer | CONFIRM, rehome high | 12991212 |
| 210 | 31549890 Institute for Fiscal Studies | Pensions Policy Institute | 46807946 / ORG-GB-PPON-PCLN-8377-ZPPT | Tenderer | **PARK**, keep | - |
| 212 | 30914626 Torfaen County Borough Council | Equal Education Partners | 47293389 / ORG-GB-PPON-PDNG-5431-HGYV | Tenderer | CONFIRM, rehome high | 30928130 |
| 213 | 30913629 NHS England | Portsmouth Hospitals University NHS Trust | 46832653 / ORG-GB-PPON-PDPT-3135-BWWY | Tenderer | CONFIRM, rehome high | 30971975 |
| 215 | 30916371 Rhondda Cynon Taf CBC | WATKINS COACHES LTD | 46755521 / ORG-GB-PPON-PDXZ-9391-VHZQ | Tenderer | **PARK**, keep | - |

In every one of the 9 sections, the section carries only the printed name, the holder's PPON and country GB. Exactly one party binds each section: a single role, or for Watkins the same Tenderer role on 6 lots. The holder's own name never appears in the section, so rule 3 passes everywhere. Each PARK comes from rule 1 or rule 2.

---

## 200: Compendium Regeneration Limited on 30926847 "Network Rail": CONFIRM 1307252

**Notice 46853619**
- Buyer: Thirteen Housing Group Limited (ORG-GB-MPR-7522, Procedure-Buyer).
- Procedure: "Leeway, Hull - New Build Development", described as "Packaged Land Deal. Brought to THG by Lovells."
- Tenderer: section ORG-GB-PPON-PZLB-9446-RVML, which carries only the name "Compendium Regeneration Limited" and GB-PPON-PZLB-9446-RVML. It is bound once, as Tenderer (TPA-1-0).

**Rule 1 (different entity):** Compendium is a Lovell-linked housing regeneration company and Network Rail is the rail infrastructure manager, so they are not related. 30926847 is a magnet PPON: u2a found 7 or more unrelated firms printed on it (Compendium, RSS, The Law Society, M Group, Taylor Wimpey, Bupa...) by unrelated buyers. That pattern fits a placeholder or copied identifier, not a relation.

**Rule 2 (destination):** 1307252 is provisional, GB, 1 mention, with the exact name "Compendium Regeneration Limited". In r8's `compendium` prefix range it is the only row with this name. The other GB rows have different names: Compendium Group 1307773, Compendium Living 16970543, Compendium Education & Training 31634969. No identified Compendium row exists in GB.

**Mention:** rehome, high.

## 202: RSS International Ltd on 30926847 "Network Rail": CONFIRM 30929573

**Notice 46709309**
- Buyer: National Highways Limited (Procedure-Buyer).
- Procedure: "M5 J20-21 MP164 St George's Rail Deck - Main Scheme - RSS Int.", £10,000.
- Tenderer: section PZLB-9446-RVML carries only "RSS International Ltd" and is bound once, as Tenderer.

**Rule 1:** RSS International is a private firm and has no legal relation to Network Rail. The scheme sits over a railway, which raised the question of whether Network Rail acts as an agent. I rule that out: the same buyer, National Highways, printed this PPON for The Law Society too (row 204), so it is the magnet or default PPON, not an agent relation.

**Rule 2:** 30929573 GBCOH06707779 "RSS INTERNATIONAL LTD" is the only identified row in the r8 range. The other two rows are provisional: "RSS International" 31546684 and "RSS INTERNATIONAL LTD" 31540612. The name agrees after folding, and no company number is printed in the section that could contradict 06707779.

**Mention:** rehome, high.

## 204: The Law Society on 30926847 "Network Rail": CONFIRM 14894295

**Notice 46855540**
- Buyer: National Highways Limited.
- Procedure: "The Law Society - Practising Certificates (SRA)".
- Tenderer: section PZLB-9446-RVML carries only "The Law Society" and is bound once, as Tenderer.

**Rule 1:** the chartered professional body has no relation to Network Rail.

**Rule 2:** 14894295 is provisional, GB, 1 mention, with the exact name "The Law Society". In the r8 range it is the only row named exactly that. The other rows have different names and all are provisional:
- "Law Society of England" 12964211 and "Law Society of England and Wales" 31856251 (no country). These look like the same chartered body under longer names, so the exact-name row cannot land on a different entity.
- "The Law Society of Ireland", "Law Society of Scotland" and others. These are different bodies, ruled out by the name and by the SRA/England context.

**Mention:** rehome, high. Side note: the Law Society is split across 3 provisional rows. That is a 470/511-style consolidation note, not 545 work.

## 207: SWANSEA BAY UNIVERSITY HEALTH BOARD on 30915227 "Storz Medical (UK) Limited": CONFIRM 30916810

**Notice 46784746**
- Procedure: "OR1 Theatre upgrade", a direct award to Storz ("Storz's systems ... embedded within our operational environment").
- Procedure-Buyers: two, ORG-GB-PPON-PZZV-3765-WXZQ (the printed name) and NWSSP-Procurement Services (PXWV-6492-CGMN, its own section).
- Tenderer: KARL STORZ ENDOSCOPY (UK) LIMITED (PXPT-1481-QGZD, its own section, served today as 30914709).
- Section PZZV-3765-WXZQ carries only "SWANSEA BAY UNIVERSITY HEALTH BOARD" and that PPON. It is bound once, as Procedure-Buyer.

**Rule 1:** a health board is not related to Storz Medical (UK) Ltd. A Storz-family relation, if one exists, links the holder and the Tenderer, not the holder and the health board. The likely cause is that the buyer looked up "Storz" and pasted Storz Medical's PPON into its own entry.

**Rule 4:** a Procedure-Buyer role, but the destination is a buyer body (a health board), not a supplier. The notice clearly shows the board buying, together with NWSSP. G2 (shared procurement) does not apply because the holder is a supplier, not a public buyer.

**Rule 2:** two identified rows exist, and both are the same legal entity:
- 30916810, GBNHS7A3 (the ODS code for Swansea Bay UHB), 59 mentions;
- 30926961, GBPPONPQDY1678DHXN, 15 mentions. This is the board's own PPON, which also shows that PZZV-3765-WXZQ is not the board's.

Most-referenced picks 30916810. Either row is correct, so there is no reason to change. The provisional rows 13558212 and 31578762 are not preferred over identified ones.

**Mention:** rehome, high.

## 208: Jeol (U.K.) Limited on 30913706 "UK Research & Innovation": CONFIRM 12991212

**Notice 46834978**
- Buyer: SoS for Health and Social Care through UKHSA (Procedure-Buyer).
- Procedure: "Service and Maintenance x JEOL".
- Tenderer: section PDQJ-7126-JDHG carries only "Jeol (U.K.) Limited" and is bound once, as Tenderer.
- Tender 8664993 still serves the Tenderer and winner as UKRI. This is the issue's own exhibit.

**Rule 1:** JEOL's UK subsidiary is a vendor of electron microscopes and is not related to UKRI.

**Rule 2:** 12991212 is identifier "00939456", named "JEOL (U.K.) Limited" (exact after case folding), 8 mentions. Its merged identifiers include GBPPONPJXB8739JZGH, which is JEOL's own PPON. The other pool rows are the same company number or the same company:
- 9534435 "939456";
- 31620129 "GBCOH939456";
- 17870866 "226825653", a VAT-shaped number for JEOL UK Ltd.

No pool row carries a different company number. 12991212 is the best row: exact name, the company number with full zero padding, its own PPON merged in, and the most mentions.

**Mention:** rehome, high.

## 210: Pensions Policy Institute on 31549890 "Institute for Fiscal Studies": PARK

**Notice 46807946**
- Buyer: The Pensions Regulator.
- Procedure: "Retirement Saving Research Consortium". The description begins: "The research undertaken by the Consortium ... 2026–28 research agenda ... four research strands". The strands are employer DC contributions, access to private pensions before state pension age, health and pension decisions after 50, and self-employment and retirement.
- Tenderer: section PCLN-8377-ZPPT carries only "Pensions Policy Institute" and IFS's PPON. It is bound once, as Tenderer.

**Rule 1 fails.** PPI and IFS are different legal entities. But the procedure is a research consortium, and the Retirement Saving Consortium is (to my knowledge) the IFS's own multi-funder pensions research programme. The strands are IFS-style work. So two readings are open:
- the identifier (IFS) is right and the printed name is wrong;
- PPI and IFS are partners or members in the consortium.

Either way, rehoming to PPI may move a correct IFS award onto the wrong entity. The rule says to park when a consortium relation is plausible. This is an inference, because nothing in the notice names IFS; hence medium confidence.

**Rule 2 (moot):** 31587812 is provisional, 2 mentions, exact name, and the only "pensions policy institute" row in q4.

**Mention:** keep, medium.

## 212: "Equal Education Partners " on 30914626 "Torfaen County Borough Council": CONFIRM 30928130

**Notice 47293389**
- Buyer: Blaenau Gwent County Borough Council (Procedure-Buyer).
- Procedure: "The Provision of Home Tuition/Online Teaching Services within Blaenau Gwent".
- Tenderer: section PDNG-5431-HGYV carries only "Equal Education Partners " (trailing space) and Torfaen's PPON. It is bound once, as Tenderer.

**Rule 1:** Torfaen is a neighbouring council, and a private tuition provider is not related to it. The buyer evidently copied the neighbour's PPON.

**Rule 2:** two identified rows exist:
- 30928130 GBPPONPVWC8236ZZJZ "Equal Education Partners Limited", 2 mentions;
- 31582974 GBCOH07611972 "Equal Education Partners", 1 mention.

There is also a provisional row 31611028. Companies House names are unique once the legal-form suffix is ignored, so both identified rows are the one company. The tiebreak (2 vs 1, a thin margin) cannot land on a different entity. Neither row is clearly better: one has the PPON, the other the company number and the exact printed name. No change.

**Mention:** rehome, high. Side note: the two rows are a consolidation candidate.

## 213: Portsmouth Hospitals University NHS Trust on 30913629 "NHS England": CONFIRM 30971975

**Notice 46832653**
- Buyer: Isle of Wight Council (Procedure-Buyer).
- Procedure: "Smoking Cessation Project: Smokefree Lead Role (PHUT/IOW NHS Trust)". The description says "The role is hosted by Portsmouth Hospitals University NHS Trust".
- Tenderer: section PDPT-3135-BWWY carries only the PHUT name and NHS England's PPON (NHS England has 1,887 mentions). It is bound once, as Tenderer.

**Rule 1:** an NHS trust is its own statutory corporation, not a subsidiary or agent of NHS England, and NHS England plays no part in this notice. The PHUT/IOW group-model link is between PHUT and the IOW NHS Trust, not the holder. This is consistent with u2a's REHOME of Bolton NHS FT on MFT's PPON.

**Rule 2:** 30971975 GBPPONPBDT9319TTHZ "Portsmouth Hospitals University NHS Trust", 8 mentions, exact name. It is the only identified row in q4's "portsmouth hospitals" range of about 45 rows; the rest are provisional, including an exact-name provisional row 15706218. PHUT has its own PPON, which also shows PDPT-3135-BWWY is not PHUT's.

**Mention:** rehome, high.

## 215: WATKINS COACHES LTD on 30916371 "Rhondda Cynon Taf CBC": PARK (destination not unique)

**Notice 46755521**
- Buyer: City & County of Swansea.
- Procedure: "Framework Agreement for the Provision of Home to School Transport Lots 1-7".
- Tenderer: section PDXZ-9391-VHZQ carries only "WATKINS COACHES LTD" and RCT's PPON. It is bound 6 times, as Tenderer on lots 1-6 (TPA-1-6 ... TPA-6-11), and is the same party throughout. Rule 3 passes and rule 1 passes: a coach operator is not related to the council.

**Rule 2 fails.** u2a's notes say "Watkins → 2479435 (provisional; the only GB row of the two)". The API contradicts that: `/v1/organizations/8878703` returns country GB, provisional, "Watkins Coaches", 8 mentions. Unit 1's q4 lookup also lists both rows as GB. So there are two GB provisional candidates:
- 2479435 "Watkins Coaches Ltd", 2 mentions;
- 8878703 "Watkins Coaches", 8 mentions.

The unit 1 skeptic asked for a tiebreak (most-referenced, same country) or a park. Most-referenced would pick 8878703, while the exact name picks 2479435. A provisional destination has to be the exact name and the only candidate, and here it is not. "Watkins Coaches" without "Ltd" could be a different operator, and no identified row exists to settle it.

**Mention:** keep, high (parking is required by the rule).

---

## Notes for the panel

- Magnet PPON 30926847 ("Network Rail", 9 mentions). Rows 200, 202 and 204 are all Tenderer under two unrelated buyers. Rehoming them is sound per mention. Whether the head "Network Rail" is itself right is out of scope here, and the Bupa x2 tie (u2b ties.json) stays unreviewed.
- Rows that would need a separate consolidation, not this cohort:
  - Swansea Bay UHB: 30916810 / 30926961;
  - Equal Education Partners: 30928130 / 31582974;
  - JEOL: 12991212 / 9534435 / 31620129 / 17870866;
  - The Law Society: 14894295 / 12964211 / 31856251;
  - Watkins: 2479435 / 8878703.
