# 545 unit 3, reviewer 4 of 6: nine REHOME-HIGH rows, read mention by mention (2026-10-19)

Read-only. 53 public GETs, all rc=200, no retries (`r4_gets.log`; each is also a line in the shared
`api_requests.log`). Script `get_r4.sh`: shared flock `.get.lock`, >=2 s after the shared log's last line plus
`sleep 2`, bodies cached under `bodies/`. Notices dumped with `r4_show.py`. No /v1/sql, ssh, admin, POST or git;
nothing written outside `u3/`.

Read for every row: the content of every evidence notice, the holder org, the destination org, the other
destination-pool rows, and the evidence tender. Where it settled a question I also read the holder's own notices
next to the evidence notice (CAM 46806010/46806023, eftec 46772281/46763589) and the later versions of a tender
(Big Life 46806631, Cotswold 46822563). Local u2a/u2b slice files gave the holder's other mention names.

## Verdicts

| # | Row | Holder -> destination | Verdict | Mentions |
|---|---|---|---|---|
| 1 | u2a 218 WAVEMAKER LIMITED t/a WPP 650 | MoJ 30913753 -> 31555041 | CONFIRM | 2 rehome (high) |
| 2 | u2b 26 Pullman Rail Ltd | TfW 30915848 -> 31575835 | CONFIRM | 3 rehome (high) |
| 3 | u2b 27 HOSPITAL INNOVATIONS LIMITED | SBUHB 30926961 -> 30974714 | CONFIRM | 3 rehome (high) |
| 4 | u2b 93 Monthind Clean Ltd | Cotswold Archaeology 31537076 -> 30971976 | **PARK** | 1 keep (high) |
| 5 | u2b 95 Generator Power Ltd | NGED (East Midlands) 31580163 -> 31550198 | CONFIRM | 1 rehome (high) |
| 6 | u2b 97 Ballicom Limited | Getech 30914453 -> 30917190 | CONFIRM | 1 rehome (high), unbound |
| 7 | u2b 107 Big Life | Citizens Advice Manchester 31535435 -> 31570142 | CONFIRM | 1 rehome (high) |
| 8 | u2b 119 Jeremy Benn Associates Limited | eftec 31552420 -> 10270770 | CONFIRM | 1 rehome (high) |
| 9 | u2b 121 TSI Instruments Ltd | NPL Management 30916544 -> 31581156 | CONFIRM | 3 rehome (high) |

**Totals: 9 rows, 16 mentions. 8 rows CONFIRM with 15 rehome mentions. 1 row PARKED with 1 keep mention (Monthind).
No CHANGE-DESTINATION.**

## 1. WAVEMAKER LIMITED t/a WPP 650 on Ministry of Justice: CONFIRM

- **Holder.** 30913753 "Ministry of Justice", GBPPONPDNN2773HVYN, 706 mentions. Unit 1's slice shows 704 of them
  printed as "Ministry of Justice" on this PPON. So PDNN-2773-HVYN is MoJ's own PPON.
- **Notices 46697985 and 47158784.** Both are tender 8582477, "Media Buying Services contract": a call-off under
  GCA framework RM6364 Lot 1. The buyer is UKHSA, in its own section ORG-GB-PPON-PBMQ-7576-DVGW. Section
  ORG-GB-PPON-PDNN-2773-HVYN holds only "WAVEMAKER LIMITED t/a WPP 650" with MoJ's PPON, and it is the single
  Tenderer and winner. MoJ appears nowhere else on either notice.
- **1. Different entity:** yes. A WPP media agency is not a government department, and MoJ plays no part in this
  procurement, so no agent/principal relation fits.
- **2. Destination:** 31555041 "WAVEMAKER LIMITED", GBCOH04078547, with a merged PPON (PPJT-1182-CPHN), 62
  mentions. The names agree once the trading-name suffix is dropped. It is the only candidate (identified-unique).
- **3. Section:** one party, role Tenderer. Both mentions **rehome, high**.

## 2. Pullman Rail Ltd on Transport for Wales: CONFIRM

- **Holder.** 30915848 "Transport for Wales", GBPPONPPHD4211XYXJ, 106 mentions. The slice shows about 100 of them as
  TfW spellings on this PPON. PPHD-4211-XYXJ is the TfW group's PPON.
- **Notices 46734421, 46734752 and 46739172.** These are versions 1-3 of tender 8603276, "Class 230 Monoblock Wheel
  Pans". The buyer is "Transport for Wales Rail Limited (Utility Buyer)", in its own section with a different PPON,
  PVJP-5451-JYWQ. The text says "Existing stock available from Pullman (Rail)". Section ORG-GB-PPON-PPHD-4211-XYXJ
  holds only "Pullman Rail Ltd" with TfW's PPON, and it is the single Tenderer and winner.
- **1. Different entity:** yes. Pullman is a rail engineering supplier and TfW is the buyer's parent.
- **2. Destination:** 31575835 "Pullman Rail Ltd", GBCOH02820938, 3 mentions. The other pool row is 31761395
  "Pullman Rail Limited (PRL)", GBPPONPVRP7662JHMT, 1 mention. It is the same firm under its own PPON, which
  confirms that PPHD is not Pullman's. Both rows name the same entity, so the thin tiebreak (3 against 1) cannot land
  on another company. I prefer the COH-keyed row.
- **3. Section:** one party, role Tenderer, on every version. All 3 mentions **rehome, high**.

## 3. HOSPITAL INNOVATIONS LIMITED on Swansea Bay UHB: CONFIRM

- **Holder.** 30926961 "Swansea Bay University Health Board", GBPPONPQDY1678DHXN. Twelve of its 15 mentions are
  SBUHB spellings.
- **Notices 46830621, 46840995 and 46847682.** These are versions 1-3 of tender 8662629, "Cryopreserved Skin -
  Cryoskin" for the Burns Centre. The buyer is NWSSP-Procurement Services, in its own section with a different PPON,
  PXWV-6492-CGMN. Section ORG-GB-PPON-PQDY-1678-DHXN holds only "HOSPITAL INNOVATIONS LIMITED" with SBUHB's PPON,
  and it is the single Tenderer and winner.
- **1. Different entity:** yes. Hospital Innovations is a private tissue/allograft supplier. This is not the A
  pattern, in which NWSSP's name sits on a health board's PPON or the reverse: here the supplier's name sits on the
  board's PPON.
- **2. Destination:** 30974714 "Hospital Innovations Limited", GBCOH4261709 (that is 04261709 with the leading zero
  dropped), 12 mentions. The other pool row is 31588488 "Hospital Innovations Ltd", GBPPONPYYJ9986HDBX, 1 mention:
  the same firm under its own PPON. 30974714 is right.
- **3. Section:** one party, role Tenderer. All 3 mentions **rehome, high**.

## 4. Monthind Clean Ltd on Cotswold Archaeology: PARK (the printed NAME is the error, not the identifier)

- **Holder.** 31537076 "COTSWOLD ARCHAEOLOGY LIMITED", GBCOH02362531, with merged PPON PDHL-3945-QCTM. Cotswold
  prints that PPON itself: the slice has "Cotswold Archaeology (Ltd)" 3 times under GB-PPON.
- **Notice 46819477** is version 2 of tender 8629739, Ipswich BC's "Consultant - Conservation Area Appraisals". It
  prints "Monthind Clean Ltd" with Cotswold's PPON as Tenderer and winner, contract value GBP 15,840.
- **Tender 8629739, version 3 (notice 46822563, 2026-05-26).** Ipswich republished the same contract (CON-1) on the
  same PPON section with the name **"Cotswold Archaeology Ltd"** and GBP 48,912.
- So the buyer corrected the name and kept the PPON. The work, conservation-area appraisal consultancy, is
  Cotswold Archaeology's trade and not a cleaning firm's. Either the v2 name was carried over from another form, or
  v2 is a mis-filed award. In neither reading is "the name is right, the identifier is wrong" (the 545 premise)
  established. Rehoming would move a Cotswold award to Monthind.
- **Destination, if it ever mattered.** The tiebreak is thin: 30971976 PPON 2 refs against 18210098 "11911532"
  1 ref, plus several provisional Monthind rows.
- **Verdict.** Keep. Reason: NOT-545 / name-error, confirmed by the buyer's own corrected notice.

## 5. Generator Power Ltd on NGED (East Midlands): CONFIRM

- **Holder.** 31580163 "NATIONAL GRID ELECTRICITY DISTRIBUTION (EAST MIDLANDS) PLC", GBCOH02366923. Twelve of its
  13 mentions are NGED spellings.
- **Notice 46982353** is tender 8768492, 2022, "LV Generator Hire, Operation & Live Connection", 4 lots.
  - The buyer section is "WESTERN POWER DISTRIBUTION PLC" with the id string "GB-COH-02366894; 02366985; 02366923;
    03600574", the four DNO companies. Its section id is that whole string, which is different from the supplier's.
  - Section ORG-GB-COH-02366923 holds only "Generator Power Ltd", twice (the same name repeated per lot), with
    02366923, the holder's number taken from the buyer's list. It is Tenderer and winner on all 4 lots.
- **1. Different entity:** yes. Generator Power is an independent generator-hire firm, a supplier to the DNO group.
- **2. Destination:** 31550198 "GENERATOR POWER LIMITED", GBCOH03446582, 4 mentions. The other identified row is
  9038592 "Generator Power Ltd", keyed "698938639", a VAT-shaped number, 2 mentions; it is the same firm. The
  COH-keyed, more-referenced row is right. The tender also serves the buyer correctly as 31632332, so it cannot
  collide.
- **3. Section:** one party, role Tenderer. **Rehome, high.**

## 6. Ballicom Limited on Getech: CONFIRM (unbound mention; the move changes names only)

- **Holder.** 30914453 "GETECH LIMITED", GBCOH02436537, with merged PPON PTQW-1117-WCCQ. Eleven of its 12
  mentions are Getech spellings.
- **Notice 46871836** is South Tyneside's "Framework for the Supply of IT Hardware and Peripherals", 4 lots.
  - Getech bids lot 1 in its own section ORG-GB-COH-02436537 ("Getech Ltd").
  - Ballicom bids lots 2-4 under **its own number 02799428**, in ORG-GB-COH-02799428 ("Ballicom Ltd") and
    ORG-GB-COH-02799428#2 ("Ballicom Limited").
  - The evidence section **ORG-GB-COH-02436537#2** holds only "Ballicom Limited" with Getech's number. **No
    TenderingParty references it**, so it has no role.
- **1. Different entity:** yes. Two distinct company numbers are printed on the same notice, and the two firms are
  separate competing bidders on the same framework.
- **2. Destination:** 30917190 "BALLICOM LIMITED", GBCOH02799428, 22 mentions. It is exactly the number this notice
  prints for Ballicom twice. 31561349 GBCOH2799428 is the same number without the leading zero (the 470/473 class),
  with 1 mention.
- **3. Section:** the #2 section id is distinct from Getech's own section, and it binds no party. **Rehome, high.**
  The move only removes a foreign name from Getech.

## 7. Big Life on Citizens Advice Manchester: CONFIRM

- **Holder.** 31535435 "CITIZENS ADVICE MANCHESTER", GBCOH02697257, with merged PPON PJNQ-4269-GYNV.
- **CAM's own notices.** MFT published two Welfare Advice notices to CAM on this PPON on 2026-04-02 at 16:12 and
  16:23 (46806010 and 46806023, both read).
- **Notice 46806034** was published by MFT at 16:36 on the same afternoon. It is tender 8648639 v1, "Health
  Guidance and Social Prescribing Services", and its text reads "Big Life are the provider of ... BeWell". Section
  ORG-GB-PPON-PJNQ-4269-GYNV holds only "Big Life" with CAM's PPON, as single Tenderer. MFT, the buyer, has its own
  section. This is a copy-paste from the two notices before it.
- **Tender 8648639, v2 and v3 (46806631 read; 46806650).** MFT republished on 2026-04-08 with "Big Life" on
  **GB-PPON-PPMM-8322-CXDT**. The API already serves those two versions as 31570142.
- **1. Different entity:** yes. These are two different providers of two different MFT services.
- **2. Destination:** 31570142 "Big Life", GBPPONPPMM8322CXDT, the very PPON the buyer used in its corrected
  versions. Exact name. The other Big Life rows in the pool are separate entities and none fits better: Big Life
  Centres 03311884, Big Life Group 14868354, Big Life Schools PXLL-1195-QZGT.
- **3. Section:** one party, role Tenderer. **Rehome, high.**

## 8. Jeremy Benn Associates Limited on eftec: CONFIRM

- **Holder.** 31552420 "ECONOMICS FOR THE ENVIRONMENT CONSULTANCY LIMITED" (eftec), GBCOH02917907, 3 mentions.
  - Notice 46763589 (Natural England) prints eftec with 02917907 and its own PPON, PDPX-3582-HMGT.
  - Notice 46772281 is OEP's "OEP102-01 ... 2050 target for woodland", awarded to eftec on 02917907 and published
    2025-12-16 14:57.
- **Notice 46772495** is OEP's "OEP111-01 Increasing freshwater species abundance through river habitat
  improvements", tender 8625862, a single version published 2025-12-17 10:29. Section ORG-GB-COH-02917907 holds only
  "Jeremy Benn Associates Limited" with eftec's number, as single Tenderer. OEP has its own section. Same buyer, next
  day, previous supplier's number carried over.
- **1. Different entity:** yes. Neither notice prints a consortium or group link: eftec has its own PPON, and the
  OEP102 contract went to eftec alone. The number is fully explained by the copy-paste.
- **2. Destination:** 10270770 "Jeremy Benn Associates Limited (t/a JBA Consulting)", keyed bare "03246693", with a
  merged PPON (PQBC-8538-JPBV), 64 mentions. 31535143 GBCOH3246693 (5) and 22770955 "3246693" (2) carry the same
  number, so all three rows are one entity split by the leading-zero class, and the tiebreak cannot choose a
  different company.
  - The most-referenced row follows the Jeol precedent (12991212 "00939456").
  - If unit 4 wants a GBCOH-keyed row, 31535143 is the same entity. I do not change the destination.
- **3. Section:** one party, role Tenderer. **Rehome, high.**

## 9. TSI Instruments Ltd on NPL Management: CONFIRM

- **Holder.** 30916544 "NPL Management Limited", GBCOH02937881, with merged PPON PYDB-8862-JBYM. NPL prints that
  PPON itself about 24 times ("National Physical Laboratory" ×14, "NPL Management Ltd/Limited" ×8 and others).
- **Notices 46792203, 46792211 and 46796560.** These are versions 1-3 of tender 8637891: Yorkshire Ambulance
  Service, "YAS 335 ... TSI Quantitive Fit Tester" and "TSI Portacount Machine" (HART). YAS is the buyer in its own
  section, PXHX-4264-BLQJ. Section ORG-GB-PPON-PYDB-8862-JBYM holds only "TSI Instruments Ltd" with NPL's PPON, as
  single Tenderer. NPL takes no part in the procurement.
- **1. Different entity:** yes. TSI is a US instrument maker's UK company and NPL is the national measurement
  laboratory.
- **2. Destination:** 31581156 "TSI INSTRUMENTS LTD", GBCOH04486008, 1 mention. It is the only identified row; the
  rest of the pool is provisional "TSI Instruments Ltd/Limited/Lts".
- **3. Section:** one party, role Tenderer. All 3 mentions **rehome, high**.

## Notes for the panel / unit 4

- **The Monthind pattern is a failure mode the guards do not catch.** A buyer's later version of the same tender
  can correct the name and keep the identifier. Reading every version of the tender, not only the versions that
  carry the candidate key, is what catches it. Rows with n=1 on a tender that has later versions deserve this read.
- **Two rows were confirmed by the buyer's own sibling notices.** In Big Life (CAM) and JBA (eftec) the holder's
  correct notice comes minutes or a day before the misprinted one, from the same buyer. In Big Life the buyer also
  corrected the PPON in later versions to the destination's own identifier.
- **Ballicom is unbound.** No party row exists, so the dry plan should show a names-only move for (46871836,
  "ORG-GB-COH-02436537#2").
- **Read-backs after apply.** These tenders should then serve the destination as Tenderer and winner:
  - 8582477 (Wavemaker);
  - 8603276 (Pullman);
  - 8662629 (Hospital Innovations);
  - 8768492 (Generator Power);
  - 8648639 v1 (Big Life, matching v2/v3);
  - 8625862 (JBA);
  - 8637891 (TSI).

  8629739 must still serve Cotswold Archaeology on both versions.
