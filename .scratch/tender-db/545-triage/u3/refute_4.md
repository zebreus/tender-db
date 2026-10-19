# 545 unit 3: adversarial pass over review 4 (2026-10-19)

**Scope.** u2a/verdicts.json row 218, plus u2b/verdicts2b.json rows 26, 27, 93, 95, 97, 107, 119 and 121, as judged in `review_4.md`. I tried to refute every CONFIRM and every mention marked `rehome`. The one PARK (Monthind) is keep, the conservative outcome, so I checked it but did not attack it.

**Method.**
- I re-read the saved bodies first: every evidence notice, the evidence tenders, the holder and destination orgs, and the sibling notices review 4 cited.
- Then I made **7 new public GETs**, all rc=200, with no retries. They went through `refute4_get.sh`, which takes the shared flock `.get.lock`, waits at least 2 s after the shared log's last line, and then sleeps 2 s more. Each is logged in `api_requests.log` and `refute4_gets.log`, and its body is in `bodies/`:
  - `/v1/notices/46841202/content`: TSI destination
  - `/v1/notices/46826361/content`: Pullman COH destination
  - `/v1/notices/47366331/content`: Pullman's own PPON
  - `/v1/notices/46758223/content`: Generator Power destination
  - `/v1/notices/46702981/content`: Hospital Innovations destination
  - `/v1/notices/46849381/content`: Hospital Innovations' own PPON
  - `/v1/notices/46819542/content`: Monthind's own notice
- I made no /v1/sql, ssh, admin, POST or git calls, and I wrote nothing outside `u3/`.

**Result: 1 of 8 CONFIRM rows refuted. That row is JBA on eftec (u2b 119), 1 mention.** The other 7 rows (14 rehome mentions) stand. The Monthind PARK stands, and a new read explains where the wrong name came from.

## Checks run on every row

- **(a) Shared section.** `u2b/r10_sec` and the bodies show that every evidence (notice, section) carries exactly one party role: Tenderer, or none for Ballicom. That role is referenced by a single TPA. The buyer always sits in its own section, with a different section id. Wavemaker is the same on both notices, read from the bodies and tender 8582477.
- **(b) Which field is wrong, the name or the identifier?** This is the Monthind failure mode. The question for each row is whether something independent of the inference anchors the printed name. A row is settled when one of these holds:
  - the notice text or title names the printed firm;
  - the buyer corrected the identifier in a later version;
  - the identifier's holder cannot be the supplier at all (a buyer, the buyer's group, or a beneficiary).
- **(c) Destination.** Where it was not already proved, I checked the destination with a notice that prints the destination's identifier together with the same name, and if possible together with the firm's own second identifier.
- **(d) Relation.** I looked for a trading name, former name, group link, agent or consortium. I found none on any row.

## Per row

| row | name | review 4 | my result |
|---|---|---|---|
| u2a 218 | WAVEMAKER LIMITED t/a WPP 650 (MoJ) | CONFIRM → 31555041 | not refuted |
| u2b 26 | Pullman Rail Ltd (TfW) | CONFIRM → 31575835 | not refuted, strengthened by 2 GETs |
| u2b 27 | HOSPITAL INNOVATIONS LIMITED (SBUHB) | CONFIRM → 30974714 | not refuted, strengthened by 2 GETs |
| u2b 93 | Monthind Clean Ltd (Cotswold Arch.) | PARK (keep) | park stands, strengthened by 1 GET |
| u2b 95 | Generator Power Ltd (NGED EM) | CONFIRM → 31550198 | not refuted, strengthened by 1 GET |
| u2b 97 | Ballicom Limited (Getech) | CONFIRM → 30917190 (unbound) | not refuted |
| u2b 107 | Big Life (CAM) | CONFIRM → 31570142 | not refuted |
| u2b 119 | Jeremy Benn Associates Limited (eftec) | CONFIRM → 10270770 | **REFUTED**: keep, medium |
| u2b 121 | TSI Instruments Ltd (NPL) | CONFIRM → 31581156 | not refuted, strengthened by 1 GET |

### u2a 218 Wavemaker ×2 on MoJ (46697985, 47158784; tender 8582477): not refuted
- **Anchor.** BT-24 itself says "Call-off contract awarded to Wavemaker Limited t/a WPP650 under Lot 1 ... RM6364". MoJ cannot be a media agency, and it has no part in this UKHSA call-off. PDNN-2773-HVYN is MoJ's own PPON: q3_slice has 704 of its 706 mentions printed "Ministry of Justice".
- **Destination.** 31555041 GBCOH04078547 carries its own merged PPON, PPJT-1182-CPHN. q8_cohslice shows it already holds the trading-as spellings:
  - "Wavemaker Limited T/a WPP 650" ×3 (COH) and "Wavemaker Limited T/A WPP 650" ×2 (PPON);
  - "WAVEMAKER LIMITED Trading as WPP650", "Wavemaker trading as WPP650" ×2 and "WPP 650" ×4.

  So "t/a WPP 650" is a trading name of the destination, not a separate entity.
- **Note for unit 4.** The printed name carries a t/a suffix. If the apply step name-gates, it matches as a t/a form.

### u2b 26 Pullman Rail ×3 on TfW (46734421, 46734752, 46739172; tender 8603276): not refuted
- **Anchor.** BT-24 reads "Existing stock available from Pullman (Rail)". The buyer, TfW Rail, prints its own PPON PVJP-5451-JYWQ, and the holder TfW is its parent. TfW's PPON PPHD-4211-XYXJ has about 100 TfW spellings.
- **New GET 46826361.** On Network Rail's "Supply and Overhaul of Fleet Wide Wheelsets", "Pullman Rail Ltd" is printed with **GB-COH-02820938**, as Tenderer. That is the destination's number, and the trade is consistent.
- **New GET 47366331 (2026-10-05).** **TfW Rail itself** prints "Pullman Rail Limited (PRL)" with **GB-PPON-PVRP-7662-JHMT** ("MK4 OH2 Minus"). That is Pullman's own PPON (31761395), so PPHD is not Pullman's identifier under any relation.
- **Same-entity check.** "Pullman Rail Ltd" and "Pullman Rail Limited" are the same registered name, so 31575835 and 31761395 are one firm split over a COH row and a PPON row. Either one is a sound target. Merging them is out of scope.

### u2b 27 Hospital Innovations ×3 on SBUHB (46830621, 46840995, 46847682; tender 8662629): not refuted
- **Attack: SBUHB is the beneficiary.** The Burns Centre named in BT-24 is SBUHB's, the Welsh Centre for Burns at Morriston. That explains why the PPON was copied: NWSSP buys for the board, and the clerk put the board's PPON on the supplier. It does not make this the agent pattern. The section is a Tenderer, and a health board cannot be the supplier of allograft Cryoskin. The printed supplier is the party.
- **New GET 46702981.** It prints "Hospital Innovations Limited" with **GB-COH-4261709 and GB-PPON-PYYJ-9986-HDBX together in one section**.
- **New GET 46849381.** It prints "Hospital Innovations Ltd" on PYYJ-9986-HDBX alone.
- So both pool rows, 30974714 and 31588488, are one firm, and neither carries PQDY. The destination is right.

### u2b 93 Monthind on Cotswold Archaeology (46819477): PARK stands
- **New GET 46819542.** This is Ipswich BC's "Window Cleaning at Layard House", awarded to "Monthind Clean Ltd" on **its own PPON PVRD-6865-QYLX**. It was published 2026-05-18 at 14:11, the **same day** as 46819477 (12:16).
- That is where the wrong name came from: the same buyer was handling a Monthind cleaning award that day. Version 3 (46822563) corrects the name back to Cotswold on the same PPON section.
- Rehoming would move a Cotswold award to a cleaning firm, so keep is right.

### u2b 95 Generator Power on NGED (East Midlands) (46982353; tender 8768492): not refuted
- **Anchor.** 02366923 is one of the four numbers in the buyer's own id string, "GB-COH-02366894; 02366985; 02366923; 03600574". The identifier's holder is part of the buying DNO group, so it cannot be the supplier. The section repeats "Generator Power Ltd" twice and carries no other name, and its Tenderer role covers all 4 lots. The buyer is served as 31632332, so there is no collision.
- **New GET 46758223.** Severn Trent's "Generators: Hire, Purchase, and Service Maintenance and Repair" prints "GENERATOR POWER LIMITED" with **GB-COH-03446582**, as Tenderer. That is the destination's number, and the trade is the same.

### u2b 97 Ballicom on Getech (46871836, section ORG-GB-COH-02436537#2): not refuted
- **Attack: the Monthind pattern.** Here the holder Getech is a plausible supplier: it is a lot-1 winner on the same framework. So the orphan #2 section could in principle be a misnamed Getech entry.
- **Why it does not matter.** Getech already appears correctly named in its own section ORG-GB-COH-02436537, with lot 1's TPA-1-1-1. No TPA, result or contract references the #2 section, and r10 shows roles [] and tenders []. So no bid or award moves in any reading. The move only takes the printed string "Ballicom Limited" off Getech's name list, and Ballicom's row 30917190 (GBCOH02799428, merged PPON) is the number this notice prints for Ballicom twice.
- **Result:** harmless in every reading, so not refuted. The mention keeps its names-only note.

### u2b 107 Big Life on CAM (46806034; tender 8648639): not refuted
- **Anchor.** BT-24 opens "Big Life. Big Life are the provider of ... BeWell".
- **Correction by the buyer.** The buyer, MFT, corrected the identifier in versions 2 and 3: 46806631 prints "Big Life" on **PPMM-8322-CXDT** under the same procedure, ocds-h6vhtk-067c4d. That is the destination's own PPON.
- **CAM's PPON.** MFT's two CAM notices from the same afternoon (46806010, 46806023) print CAM on PJNQ.

### u2b 119 Jeremy Benn Associates on eftec (46772495, ORG-GB-COH-02917907; tender 8625862): REFUTED
- **What review 4 cites.**
  - The number 02917907 is eftec's.
  - OEP awarded eftec OEP102-01 on that number in a notice published the previous afternoon, 46772281 (2025-12-16 14:57).
  - From that, review 4 infers that the number was carried over to OEP111-01.
- **Why that does not settle which field is wrong.**
  - **The holder is a plausible supplier.** eftec, unlike MoJ, TfW, SBUHB, NGED or NPL, could have won this study. It is an environmental-economics consultancy that the same buyer had used the day before. "eftec won both consecutive OEP studies" is a coherent reading, and in it the number is simply correct.
  - **No anchor for the printed name.** BT-21 and BT-24 name no supplier. The tender has **one version only** (seq 1, 083664-2025), so the version check that caught Monthind cannot run. OEP prints only a COH number, with no PPON, so the details were typed by hand, not pulled from a registry.
  - **This batch already holds a counter-example.** Monthind is exactly this configuration. A buyer printed a firm it was dealing with that same day on another, correct identifier. That firm's name had no visible source on the notice; the source turned up only on a sibling notice (46819542). For JBA I cannot list OEP's other notices, so I cannot exclude a concurrent OEP award to JBA as the source of the name.
- **What favours JBA, and why it is not enough.** The subject, river habitat improvements for freshwater species abundance, fits JBA's river and ecology practice better than eftec's economics. And a stale-number carry-over is a common error. I put the probability that the name is right at about 90%. That is probable, not settled. The decision applies only HIGH verdicts, and "everything else stays parked".
- **Verdict.** Mention (46772495, ORG-GB-COH-02917907): **keep, confidence medium**.
- **What would lift it to high:**
  - a later OEP version or a correction notice for OEP111-01;
  - an OEP publication naming the supplier;
  - a JBA-side notice that ties JBA to OEP111.
- **The destination itself is not in doubt.** 10270770 "03246693" carries a merged PPON (PQBC-8538-JPBV) and 64 references, and the other two pool rows are the same number.

### u2b 121 TSI Instruments ×3 on NPL (46792203, 46792211, 46796560; tender 8637891): not refuted
- **Anchor.** The titles name TSI's products: "TSI Quantitive Fit Tester" and "TSI Portacount Machine". NPL is not a party, and its PPON PYDB-8862-JBYM has about 24 NPL spellings.
- **New GET 46841202.** UKRI's "Replacement of Soft X-ray Source Tube" prints "TSI INSTRUMENTS LTD" with **GB-COH-04486008 and its own PPON GB-PPON-PGJR-8558-MDTJ** together, as single-source Tenderer. A soft X-ray source is a TSI aerosol-neutralizer part.
- This settles the single-mention destination 31581156. It also shows TSI's own PPON is PGJR, not PYDB.

## Net
- **Refuted:** u2b 119 JBA. Its 1 mention (46772495, ORG-GB-COH-02917907) moves to **keep, medium**.
- **Not refuted, 14 rehome mentions:**
  - Wavemaker ×2, Pullman ×3, Hospital Innovations ×3 and TSI ×3;
  - Generator Power ×1 and Big Life ×1;
  - Ballicom ×1, names-only.
- **Monthind:** the keep stands.
- **Out-of-scope same-entity splits seen along the way:**
  - Pullman: 31575835 and 31761395.
  - Hospital Innovations: 30974714 and 31588488, whose COH and PPON are co-printed on 46702981.
  - TSI: 31581156, and its PPON PGJR, which is not merged.
