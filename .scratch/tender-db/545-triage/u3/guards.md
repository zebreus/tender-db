# 545 unit 3: review of the judgement changes from units 2a-tail and 2b (2026-10-19)

This review was read-only. It made **20 public GETs, all rc=200, with no retries** (listed in `guards_gets.log`; each one is also a line in the shared `api_requests.log`, which holds the sibling reviewer's lines too). The bodies are in `bodies/` and were read with `guards_show.py`. Requests were spaced at least 3 s after the last line of the shared log, whichever agent wrote it.

Nothing else was touched: no /v1/sql, no ssh, no admin, no POST, no git, and no edits outside `u3/`.

The local re-computations (G1 over first/last vs all notices; G4 over 2a; the number normalisation over every distinct-entities pool) were run against the committed `u2a/verdicts.json` and `u2b/verdicts2b.json`.

## Verdicts

| Item | Verdict | Rows moved |
|---|---|---|
| (a) G1, side and unbound over all notices | **ADOPT** | 0 |
| (b) SHARED_PROCUREMENT_2B, 5 pairs | **ADOPT-WITH-CHANGE** | Hounslow ×3 and Wyre Forest ×2 move to REHOME-HIGH. Wyre Forest's 3rd mention stays keep (medium). Create PT, Bolton Impact Trust and Univ. of Essex stay parked, with a new reason. |
| (c) G4, holder's number on the destination | **ADOPT** | 0. Capita/Conexia stays parked; it is really V. |
| (d) REFUSE_DEST | **ADOPT** for PwC; **ADOPT-WITH-CHANGE** for Hull | Hull ×1 moves to REHOME-HIGH, to 30914698 |
| (e) Gleeds ×4 on College of Policing, F→A | **ADOPT** | 0. Stays PARK-agent-principal. |
| (f) Tenderer flags | **ADOPT** the three REHOME-HIGH rows; the WYCA row 31565528 is **a different defect** | 0 |
| (g) distinct-entities on malformed numbers | **ADOPT-WITH-CHANGE** for Accenture; Ingeus stays parked | Accenture ×3 moves to REHOME-HIGH, to 15715826 |
| (h) 456 exhibit, Schneider on Sellafield | **leave PARKED** | 0 |

**Effect on the combined REHOME-HIGH.**
- Before: 52 rows, 51 keys, 65 mentions.
- Added: 4 keys, 9 mentions. Hounslow 3, Wyre Forest 2 of 3, Hull 1, Accenture 3.
- After: **56 rows, 55 keys, 74 rehome mentions**, plus 1 keep mention on the Wyre Forest row.

## (a) G1, side and unbound over ALL notices of a key: ADOPT (0 rows move)

I recomputed collision, side and unbound for the 51 F rows of 2a-tail and 2b, once over first/last only and once over all notices.
- They agree on every row.
- Of the 8 keys held 3 or more times, none has a collision on a middle notice. These are Accenture, TSI, LHP, CDDFT, Pullman, Hospital Innovations, Wyre Forest and Hounslow.
- LHP is the case that shows why the change matters. q8 has no last_notice column, so first/last saw 1 of its 4 notices.

The rule is strictly more conservative than first/last, so I adopt it.

Two notes:
1. Parking granularity. When any notice of a key collides, 2b parks the whole key. The cohort is per mention, so only the colliding (notice, section) strictly has to park. Parking the whole key is stricter, and that is acceptable: all 5 shared-section rows have n=1.
2. A blind spot. G1 counts distinct role strings per (org, notice, section), so two parties with the SAME role on one section are not seen. Notices themselves break a reused id into separate `#N` sections (notice 46867590 shows `ORG-GB-COH-3829536#2`), so this rarely bites. It is still not a proof.

## (b) The 5 new G2 pairs: ADOPT-WITH-CHANGE

What the notices print decides each pair. G2's premise is "one body running another's tender under its own account". That holds only when the holder is printed on the notice or named as the procuring body. Two of the five pairs fail that test.

**London Borough of Hounslow ×3 on Wandsworth: a real misbind, REHOME-HIGH to 30918222.**
- Notice 46381132 is "2425-0045 CCTV - The Supply Installation and Maintenance of Equipment", a framework.
- It prints four buyers: Islington (PBMD-2513-CWJJ), Ealing (PRHZ-3346-MGDQ), **"London Borough of Hounslow" on PVLQ-8225-QVHT (Wandsworth's PPON)** and Southwark (PXTW-7647-VDLV). The Tenderer is Tyco.
- Wandsworth appears nowhere: 0 occurrences in the body.
- So this is not Wandsworth running a tender. Hounslow, one of four named boroughs, carries the wrong PPON.
- All 3 mentions are on the same tender, 8639243, with the same section and a single role (Procedure-Buyer, from r10).
- The destination is identified-unique: 30918222, GBPPONPQYY1684DYLV, "London Borough of Hounslow".

**Wyre Forest DC ×3 on Shropshire F&RS: a real misbind. 2 mentions REHOME-HIGH to 31550990, 1 keep (medium).**
- Notice 46826419 is "Lion Joint Venture - SPV - Wyre Forest District Council". Its text says "The Authority (Wyre Forest District Council) wishes to appoint a Development Partner…".
- It prints three buyers: West Mercia Police, **Wyre Forest DC on PTQY-8812-XHLH (Shropshire F&RS's PPON)**, and Wyre Forest DC again on **PTVB-7542-QLLX, its own PPON**. That own PPON is exactly the destination row 31550990.
- Shropshire F&RS is never printed, and the words "Shropshire" and "Fire" occur 0 times.
- The PTQY section is a duplicate buyer entry with a foreign PPON. It is not shared procurement.
- 46826428 is the same tender (8660381) as 46826419, so the read covers it.
- 46842882 is a different tender (8677275) and was not read. Because G2 is the class where only a notice read lifts the park, it stays **keep, medium** until one GET confirms it.
- Note for unit 4: after the move, tender 8660381 shows WFDC twice as Procedure-Buyer, from 2 sections. The dry plan should show this. The duplicate is harmless and replaces a false Shropshire F&RS buyer.

**Create Partnership Trust on The Howard Partnership Trust: stays PARKED, but the reason is not G2.**
- Notice 46758153 is "Create Partnership Trust - Fixed Electricity Supply Contracts October 2025". It prints Create PT with GB-COH-07597068, which is the holder's own number, and the broker **Ginger Energy** as a second Procedure-Buyer. The Tenderer is Drax.
- Two readings fit, and both park:
  - **Rename.** The holder renamed itself, so this is V, not 545, and the PPON row 31544183 "Create Partnership Trust" is the same entity split across rows.
  - **Broker copy-paste.** Ginger Energy is already an A-class broker in 2a.
- Relabel the reason from shared-procurement to "V-or-A, holder's own number".

**The Bolton Impact Trust ×2 and University of Essex ×2 on Embark MAT: stay PARKED (A or shared procurement), with a flag.**
- Notice 46851503 is the **University of Essex's own** "Energy Management Solutions Dynamic Market". Essex prints itself with PYLY-4964-TXGH, and Dukefield Energy is co-buyer (the agent).
- So the PPON on Embark's org (keyed GBCOH11717748) is printed by Essex as its own. The schools' mentions look like call-offs under Essex's market.
- This is a real shared-procurement vehicle, so parking is right.
- **Different defect to check before anything moves:** PYLY-4964-TXGH may be Essex's (or Dukefield's) PPON, wrongly attached to Embark's org as an altid (469/481 territory).
- The Essex pool also shows a normalisation gap: `RC000652` vs `GBCHCRC000652` are the same royal-charter number, but they count as distinct "other" ids.

**Change to G2.** A public-on-public buyer-side F parks only when the holder is printed on the same notice or named in its procedure text as the procuring body. Otherwise treat it as F.
- 2a's 7 hand pairs were not re-read; I had no GETs left. They stay parked:
  - Castle Point ×2 on Braintree;
  - Uttlesford on Chelmsford;
  - Hartlepool on North Tyneside;
  - Alder Hey on the Walton Centre;
  - BCUHB on Bangor University;
  - Waterways Ireland on DoJ NI.
- Re-reading one notice of each, with the test above, is a candidate small unit.

## (c) G4: ADOPT (Capita/Conexia stays parked)

- Notice 46980990 is a University of Derby award for its "Executive and Academic Recruitment" framework, from a 2022 procedure.
- It prints "CAPITA RESOURCING LTD (T/A VEREDUS)" with **GB-COH-03949686**. That is Conexia's number: the holder is 30916935, GBCOH03949686.
- Identified row 8881761, "Capita Resourcing Limited t/a Veredus Executive Resourcing", is keyed by the same 3949686. adv2b's c2 confirmed this.
- So two separate places pair this name with 03949686. It is the holder's former or trading name, which makes it V, not F, and it must not be rehomed.
- G4 is a sound rule: an exact number match after id_kind normalisation.
- Applied to u2a's verdicts it fires 0 times. Across 2a and 2b it fires exactly once, on this row.

## (d) REFUSE_DEST

**PwC LLP on Coforge (notice 46750576): ADOPT. It stays parked.**
- Unbound: no party row.
- The identified PwC rows carry different numbers: OC303525, OC3030525 (an extra digit, not a truncation), GBCOHOC309347, the VAT-shaped GB243823957, and 02319585 for Consulting Services.
- Even after the normalisation in (g) they stay distinct, so neither the tiebreak nor the provisional "PWC LLP" is allowed.
- A rehome would change a name only.

**Kingston upon Hull City Council on Artifax: ADOPT-WITH-CHANGE. Keep the refusal of 31541038, but REHOME-HIGH to 30914698.**
- Notice 46760078 is "The provision of Event Management Software … via the Crown Commercial Service G-Cloud 14" (Artifax software).
- The buyer section prints "Kingston Upon Hull City Council" (the council's legal name) with **GB-COH-02007321**, which is the supplier Artifax's number and the holder's own identifier.
- The **same notice** prints "Hull City Council" as review body (ORG-GB-FTS-41663).
- This is not public-on-public, the section carries a single role, and no tenderer is printed. So Artifax is falsely credited as the buyer.
- Hull is split over three identified rows:

  | Row | Identifier | Mentions | Source |
  |---|---|---|---|
  | 30914698 | GBPPONPRNP1874YZZR | 360 | adv2b c2 |
  | 18819350 | `168524345` | 1 | GET |
  | 31541038 | GBCOH168524345 | 1 | adv2b c2 |

- The most-referenced rule picks 30914698 as the unique maximum. That is the same rule that sent Jeol to 12991212.
- Change: REFUSE_DEST becomes a reviewed alias table: `{(30975996, 'Kingston Upon Hull City Council'): 30914698}`.

## (e) Gleeds Cost Management ×4 on College of Policing, F→A: ADOPT

- Notice 46693105 (tender 8580124) covers distribution boards, heating works and gymnasium lighting at College of Policing's Ryton site. Its text says "College of Policing seeks supplier(s)…".
- It prints **College of Policing in its own buyer section**, carrying both GB-COH-08235199 and GB-PPON-PJNL-2763-PVHM. Gleeds is a **second Procedure-Buyer**, on CoP's PPON.
- So Gleeds' section is not a mis-keyed CoP section, because CoP is present on its own. Gleeds is a co-listed consultant buyer. That settles the adversarial check's "both readings" in favour of A.
- G1 does not fire, because the sections differ.
- Rehoming would show Gleeds as co-buyer. Whether agents should be shown that way is the A-class decision, so the row parks: 1 row, 4 mentions.

## (f) Tenderer flags: all three are genuine suppliers on someone else's number

None of the three is a public-body oddity. Each named body really is the supplier, and the printed identifier belongs to another org. **ADOPT** the REHOME-HIGH rows as they stand; no rows move.

- **West Yorkshire CA on Bristol Temple Quarter LLP (OC451571) goes to 30913749** (182 mentions against 1).
  - Notice 46794970: the buyer is the Police & Crime Commissioner for Sussex.
  - Its text reads "Subscription to the Police National Legal Database" and "Direct award under a national agreement contract with PNLD with the West Yorkshire Combined Authority".
  - WYCA, which hosts PNLD since it took over the West Yorkshire PCC's functions, is the real single Tenderer. OC451571 is a mis-keyed number.
- **The mis-keyed WYCA row 31565528, "GBCOHCO451571" (1 mention), is a different defect.**
  - The name and its own row agree. `CO451571` is not a valid company-number shape; it looks like the same publisher's transposition of OC451571.
  - This is a junk-identifier split of WYCA, which is the 452/466 identifier-verdict class (or the 470/473 split class). It is not a 545 misbind, and it is **not in the 545 cohort**.
- **University of Liverpool ×2 on Cardiff University goes to 30953698** (42 mentions against 1; both rows carry RC000660).
  - Notice 46839286 is "Masters in Public health Qualification - North". It says "HEIW intends to award a contract to the University of Liverpool".
  - The Tenderer is printed with Cardiff's PPON PHVT-4343-HWYR.
  - 46843620 is the same tender, 8667254.
- **Royal College of Psychiatrists on Sussex goes to 31561689** (GBCOHRC000463, identified-unique).
  - Notice 46798192: the buyer is DHSC. Its text names the "Addiction Psychiatry credential … delivered by Royal College of Psychiatrists (RCPsych)".
  - The Tenderer is printed with GB-COH-RC000672, the University of Sussex's charter number.

## (g) Distinct-entities on malformed variants of one number

**Change to `distinct_entities`.** Before comparing COH numbers:
1. strip a trailing UK postcode (for example `04757301EC3M3BD` becomes `04757301`);
2. treat a number that is a one-digit-short prefix of another pool number as a truncation of it (`0475730` against `04757301`).

Nothing else changes. A one-digit substitution or insertion still counts as a distinct number.

Applied to every distinct-entities pool in 2a and 2b, this collapses **only Accenture**. Schneider, BMBC, UKCEH, CDDFT, Ingeus, Newcastle, Haringey, Kew and Essex all stay distinct.

**Accenture (UK) Ltd ×3 on Walnut Unlimited: REHOME-HIGH to 15715826.**
- Notice 46768534 is the Bank of England's "Document Management System (DMS) Migration". Its single Tenderer prints "Accenture (UK) Ltd" with **GB-COH-01317137**, Walnut's number. Walnut's own 5 mentions print that number.
- The same Tenderer also prints **GB-PPON-PNPX-2447-RCXR**, which is row 31565018, "ACCENTURE (UK) LTD - FCA".
- The BoE is in its own section, so there is no collision.
- Mention counts, from GETs:

  | Row | Identifier | Mentions | Note |
  |---|---|---|---|
  | **15715826** | `04757301` | **86** | carries merged PPON PYHC-2313-BXRY |
  | 9766497 | `4757301` | 11 | |
  | 31549294 | GBCOH4757301 | 8 | |
  | 31565018 | PPON PNPX-2447-RCXR | 2 | outside the vote-key pool |
  | 22952038 | `04757301EC3M3BD` | 1 | |
  | 23157128 | `0475730` | 1 | |

- 15715826 is the unique maximum.
- The other two mentions, 46788047 and 47286834 (tender 8595979), were not read. r10 shows the same section with Tenderer as the single role, which is the pipeline's usual supplier-side bar.

**Ingeus UK Ltd on ID Medical: stays PARKED (a different defect).**
- `4320553` against `04320853` is a one-digit substitution, a valid-shaped distinct company number, so the normalisation rightly keeps the pool distinct.
- The deeper defect is in notice 46867590, the NHS SBS mental-health framework award (SBS10239). It prints ID Medical and Ingeus with the **same id**, GB-COH-3829536, in sections `ORG-GB-COH-3829536` and `…#2`.
- Every Tenderer ref (lots 1, 2 and 3) points at that shared id and binds to ID Medical's section. That leaves Ingeus' mention unbound.
- A rehome would move a name only. The lot awards cannot be attributed from the notice, so this is an in-notice id collision and not fixable through 545.

## (h) Issue 456 exhibit 8751605, Schneider Electric on Sellafield: leave PARKED

- Notice 46966800 is "UKRI-6398 Meter Integration Boiler Plantrooms". The buyer is UKRI on PDQJ-7126-JDHG.
- For the Tenderer it prints only **"Schneider Electric", GB-PPON-PWYP-8439-MZWY and GB**: no company number and no address.
- Nothing printed distinguishes the candidate rows:
  - 12539471 "Schneider Electric Ltd", 02886434, 10 mentions;
  - 30916476 "SCHNEIDER ELECTRIC LIMITED", GBCOH01407228, 33 mentions;
  - and a third, 10736313 "Schneider Electric Systems UK Limited", 00293260.
- The register is outside this unit's read rules. 456 stays open, with 8751605 as a read-back.
- **Unverified premise:** that PWYP-8439-MZWY is Sellafield's own PPON. If it is really Schneider's PPON merged into Sellafield (5718658, keyed `01002607`), the defect is that merge and not this mention. One GET of `/v1/organizations/5718658` (merged_identifiers) settles it.

## Additions to the cohort (per mention, issue 317 shape)

Every one of these sections carries a single role (r10), and the holder is not bound to the same section id in another role.

```json
[
 {"org_id": 30913851, "notice_id": 46381132, "section_id": "ORG-GB-PPON-PVLQ-8225-QVHT", "action": "rehome", "target_org_id": 30918222, "target_name": "London Borough of Hounslow", "confidence": "high",
  "rationale": "545: Hounslow is one of four named buyers (Islington, Ealing, Hounslow, Southwark) on framework 8639243, printed with Wandsworth's PPON; Wandsworth is printed nowhere on the notice; single role Procedure-Buyer; identified-unique destination"},
 {"org_id": 30913851, "notice_id": 46381196, "section_id": "ORG-GB-PPON-PVLQ-8225-QVHT", "action": "rehome", "target_org_id": 30918222, "target_name": "London Borough of Hounslow", "confidence": "high",
  "rationale": "545: same tender 8639243 and section as 46381132 (read); single role Procedure-Buyer"},
 {"org_id": 30913851, "notice_id": 46825561, "section_id": "ORG-GB-PPON-PVLQ-8225-QVHT", "action": "rehome", "target_org_id": 30918222, "target_name": "London Borough of Hounslow", "confidence": "high",
  "rationale": "545: same tender 8639243 and section as 46381132 (read); single role Procedure-Buyer"},
 {"org_id": 31560408, "notice_id": 46826419, "section_id": "ORG-GB-PPON-PTQY-8812-XHLH", "action": "rehome", "target_org_id": 31550990, "target_name": "Wyre Forest District Council", "confidence": "high",
  "rationale": "545: 'The Authority (Wyre Forest District Council)'; WFDC printed twice, once on its own PPON PTVB-7542-QLLX (the destination) and once on Shropshire F&RS's PPON; Shropshire never printed; single role Procedure-Buyer"},
 {"org_id": 31560408, "notice_id": 46826428, "section_id": "ORG-GB-PPON-PTQY-8812-XHLH", "action": "rehome", "target_org_id": 31550990, "target_name": "Wyre Forest District Council", "confidence": "high",
  "rationale": "545: same tender 8660381 and section as 46826419 (read); single role Procedure-Buyer"},
 {"org_id": 31560408, "notice_id": 46842882, "section_id": "ORG-GB-PPON-PTQY-8812-XHLH", "action": "keep", "target_org_id": 31550990, "target_name": "Wyre Forest District Council", "confidence": "medium",
  "rationale": "G2 class, different tender 8677275, notice not read; one GET to lift"},
 {"org_id": 30975996, "notice_id": 46760078, "section_id": "ORG-GB-COH-02007321", "action": "rehome", "target_org_id": 30914698, "target_name": "Hull City Council", "confidence": "high",
  "rationale": "545: buyer 'Kingston Upon Hull City Council' printed with supplier Artifax's company number on a G-Cloud buy of Artifax software; the same notice prints 'Hull City Council' as review body; Hull split over 30914698 (360) / 18819350 (1) / 31541038 (1, refused VAT-shaped key): most-referenced"},
 {"org_id": 31537611, "notice_id": 46768534, "section_id": "ORG-GB-COH-01317137", "action": "rehome", "target_org_id": 15715826, "target_name": "Accenture UK Ltd", "confidence": "high",
  "rationale": "545: BoE's single Tenderer 'Accenture (UK) Ltd' printed with Walnut's number 01317137 plus an Accenture PPON (31565018); the pool is one entity, 04757301, after postcode and truncation normalisation; most-referenced 15715826 (86 against 11/8/1/1)"},
 {"org_id": 31537611, "notice_id": 46788047, "section_id": "ORG-GB-COH-01317137", "action": "rehome", "target_org_id": 15715826, "target_name": "Accenture UK Ltd", "confidence": "high",
  "rationale": "545: same section and printed name, single role Tenderer (r10); tender 8595979"},
 {"org_id": 31537611, "notice_id": 47286834, "section_id": "ORG-GB-COH-01317137", "action": "rehome", "target_org_id": 15715826, "target_name": "Accenture UK Ltd", "confidence": "high",
  "rationale": "545: same section and printed name, single role Tenderer (r10); tender 8595979"}
]
```

**Removed or relabelled, not added:**
- Capita/Conexia: V, not F, and stays out.
- Create PT: V-or-A.
- Embark pair: A, plus the altid flag.
- WYCA 31565528: 452/466 class.
- Ingeus: in-notice id collision.

## Follow-ups this review leaves (not 545 cohort work)

- **1 GET each, to lift or confirm:**
  - 46842882, Wyre Forest tender 8677275;
  - `/v1/organizations/5718658`, to check that PWYP-8439-MZWY is Sellafield's own PPON;
  - one notice for each of 2a's 7 G2 pairs, under the new G2 test.
- **Altid check:** PYLY-4964-TXGH on Embark's org (31577532), which the University of Essex prints as its own PPON.
- **Normalisation gaps beyond (g):** `RC000652` vs `GBCHCRC000652` (Essex), and `OC303525` vs `OC3030525` (PwC). Both belong to the identifier-split classes, not 545.
- **Accenture's own PPON split:** PNPX-2447-RCXR (row 31565018) against PYHC-2313-BXRY, which is merged on 15715826. Note it for 470/473.
