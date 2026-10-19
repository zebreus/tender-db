# 545 unit 3 — reviewer 6 of 6 (2026-10-19)

Scope: u2b/verdicts2b.json rows 369, 412, 419, 440, 444, 446, 455. That is 7 rows and 8 mentions.

Reads:
- Public API only. 26 GETs, all rc=200, sequential and at least 2 s apart. Each one is logged in `api_requests.log` and its body saved under `bodies/`.
- Three notice bodies were already cached by other reviewers and reused: 46794970, 46839286 and 46798192.
- Helpers in this directory: `get_r6.sh` (fetches) and `r6_who.py` (summarises a notice). `show_notice.py` was reused.
- No /v1/sql, no admin endpoints, no POSTs, no git.

Rule-3 check: for every evidence (notice, section), a script confirmed three things.
- The section carries exactly one name, one identifier and the country.
- It is referenced exactly once, as OPT-300-Tenderer.
- Two sections share an identifier with another org section in the same notice; both are flagged in the rows below (369 and 419). In both cases the other section is separate (different section id).

**Result: all 7 rows CONFIRM. All 8 mentions rehome at high confidence. No CHANGE-DESTINATION and no PARK.**

## 369 — "Eastern Counties Scaffodling Services" on 30915302 NORWICH CITY SERVICES LIMITED → 31573542

- **Notice 46810761** (UK7, "Open order Various Special Access Works"):
  - Section `ORG-GB-PPON-PWTD-8252-JQTZ` carries only "Eastern Counties Scaffodling Services" and GB-PPON-PWTD-8252-JQTZ. Its one role is Tenderer (TPA-1-0).
  - The buyer is in a separate section, `ORG-GB-COH-12656036`: NCSL as Procedure-Buyer, carrying GB-COH-12656036 and also the same PPON PWTD-8252-JQTZ.
  - That shared PPON explains why the holder has GBPPONPWTD8252JQTZ in merged_identifiers.
  - The buyer reused one PPON in two sections. The mention's section is still a distinct section id with a single party, so it is **rehome**.
- **Rule 1:** the two are different legal entities.
  - NCSL is a council-owned services company. ECSS is a scaffolding contractor supplying it.
  - Their only relation is buyer and supplier. That is not group, agent/principal or consortium.
- **Rule 2:** 31573542 is a sound destination.
  - It is identified by its own, different PPON, PHGP-1225-WWPN, and has 4 mentions.
  - Its name is exactly "Eastern Counties Scaffodling Services", same typo included. It is the only pool candidate.
  - The region is plausible: lot NUTS is UKH15 (Norwich).
- **Verdict:** CONFIRM, high.
- **Note:** the holder keeps PWTD-8252-JQTZ as a merged identifier. If a later re-fold binds by PPON instead of by the reviewed (notice, section) verdict, it could pull this mention back.

## 412 — "Crown Paints Ltd" on 31597395 PPG Architectural Coatings UK Ltd (GBCOH436135) → 10890039

- **Notice 46951039** (EEM0005 decorating materials framework, 2023 award):
  - Lot 1 has three tenderers. They are `ORG-GB-COH-436135` ("Crown Paints Ltd", GB-COH-436135), `ORG-GB-COH-00436135` ("PPG Architectural Coatings UK Ltd", GB-COH-00436135) and Brewers.
  - Lot 4 lists Crown Paints again, under its real number: `ORG-GB-COH-6624576` ("Crown Paints Ltd", GB-COH-6624576).
  - 436135 is PPG's number with the zeros stripped.
  - Section `ORG-GB-COH-436135` is referenced once, as Tenderer for Lot 1, and holds only Crown, so it is **rehome**.
- **Rule 1:** Crown Paints Ltd (Hempel group) and PPG Architectural Coatings UK (PPG group) are competitors with no group relation. Both tender separately in the same lot.
  - The holder's other 3 mentions all print a PPG name.
- **Rule 2:** 10890039 is a sound destination.
  - It is "Crown Paints Ltd", keyed 06624576, with PPON PVZT-1732-ZPCG and 15 mentions.
  - Its number agrees with the 6624576 printed for Crown in the same notice.
  - The other pool rows are the same company number in other key formats: 11354334 "6624576" and 31623833 "GBCOH6624576" (read: Crown Paints Ltd, 1 mention).
  - The most-referenced row is the right pick. The duplicates are an altid matter, not a reason to park.
- **Verdict:** CONFIRM, high.

## 419 — "Thomas Bow Limited" on 31632003 Telent Technology Services Ltd. (GBCOH703317) → 14183429

- **Notice 46980862** (Derby CC TD1705 highways framework, 2022):
  - Section `ORG-GB-COH-703317#2` carries "Thomas Bow Limited" with GB-COH-703317. Its one role is Tenderer in Lot 6 (Carriageway Maintenance).
  - Telent has its own section, `ORG-GB-COH-703317` (Tenderer in Lot 4, Traffic Signals). It shares the number but is a distinct section id.
  - Thomas Bow also appears under its real number in `ORG-GB-COH-4503934` (Lot 5).
  - The "#2" suffix shows the buyer had two parties under one number.
  - The mention's section holds only Thomas Bow, so it is **rehome**.
- **Rule 1:** Thomas Bow is a Nottingham surfacing contractor and Telent is a telecoms and traffic-technology firm. There is no relation, and they won different lots.
- **Rule 2:** 14183429 is a sound destination.
  - It is "Thomas Bow Ltd", keyed 04503934, with 19 mentions. Its number agrees with the 4503934 printed in the same notice.
  - The pool alternatives carry the same number: 10319217 "4503934" and 31600286 "GBCOH4503934" (read: Thomas Bow Limited, 2 mentions).
- **Verdict:** CONFIRM, high.

## 440 — "Dell Corporation" on 31582801 HAND AND MILLAR MANAGEMENT CONSULTING LLP (OC403062) → 31535972

- **Notice 46844315** (MOD, UK5, "715892482-Dell HW support for Spectrum", £8,291.44):
  - Section `ORG-GB-COH-OC403062` carries "Dell Corporation" and GB-COH-OC403062. Its one role is Tenderer. The buyer is in a separate section (MOD PPON PHVX-4316-ZVGZ).
  - The text says "Direct engagement with Dell as the hardware is exclusively supplied by Dell ... Dell supplied a quote".
- **Rule 1:** I checked for an agent/principal relation, because the text says the contract was "awarded based on TEPAS 2 FW". I read the holder's own notices (r10 lists 3 more mentions, on notices 46842841 and 46844339):
  - 46842841: CCS DOS7 framework award, with H&M as Tenderer.
  - 46844339: MOD UK6 "C4ISR - Approvals and Assurance lead", supplier HAND AND MILLAR, RTM "Audit & Assurance Services 2 RM6310".
  - That MOD notice comes from the same buyer PPON and was published 30 minutes after the Dell notice (15:38 against 15:08 on 2026-07-24).
  - So H&M is an assurance consultancy on a different route to market, not a Dell reseller. The OC number is a carry-over from the buyer's next notice.
  - The two are different legal entities and no relation is plausible.
- **Rule 2:** 31535972 is a sound destination.
  - It is "DELL CORPORATION LIMITED", keyed GBCOH02081369, with PPON PWNY-9527-CNQX and 42 mentions.
  - The printed "Dell Corporation" matches after legal-form folding. No number of Dell's own is printed.
  - The pool alternatives are the same entity: 7955216 "Dell Corporation Limited" keyed 2081369 (read, 8 mentions), and 8223346 keyed on a VAT-shaped number.
- **Verdict:** CONFIRM, high.

## 444 — "West Yorkshire Combined Authority" on 31548301 BRISTOL TEMPLE QUARTER LLP (OC451571) → 30913749

- **Notice 46794970** (Sussex PCC, neg-wo-call award, "Subscription to the Police National Legal Database"):
  - Lot text: "Direct award under a national agreement contract with PNLD with the West Yorkshire Combined Authority".
  - Section `ORG-GB-COH-OC451571` carries only WYCA and GB-COH-OC451571. Its one role is Tenderer. The buyer (Sussex PCC) is in separate FTS sections.
  - So the section is **rehome**.
- **Rule 4:** WYCA, a public body, sits on the supplier side here. That is not the red flag, which is a Buyer role on a supplier.
  - The notice clearly shows WYCA as the contractor: PNLD is a WYCA-hosted service, and the procedure title is WYCA itself.
- **Rule 1:** BTQ LLP is a Bristol regeneration partnership. It has no relation to WYCA.
- **Rule 2:** 30913749 is a sound destination.
  - It is "West Yorkshire Combined Authority", keyed on PPON PHYG-7328-ZLQJ, with 182 mentions. The name matches exactly. WYCA is a statutory body with no company number.
  - The other pool row, 31565528 (read), is keyed "GBCOHCO451571" and carries identifier_status=register_mismatch. It is a corrupted copy of BTQ's number, so it is not a better destination.
- **Verdict:** CONFIRM, high.

## 446 — "University of Liverpool" x2 on 30914001 CARDIFF UNIVERSITY (RC000089) → 30953698

- **Notices 46839286 (UK6) and 46843620 (UK7):** both belong to procedure ocds-h6vhtk-06c8ec, HEIW "Masters in Public health Qualification - North".
  - In each, section `ORG-GB-PPON-PHVT-4343-HWYR` carries only "University of Liverpool" and GB-PPON-PHVT-4343-HWYR. Its one role is Tenderer.
  - The buyers, HEIW and NWSSP, are in separate sections. Both mentions are **rehome**.
  - The text names "the University of Liverpool" as the sole provider for North Wales trainees.
  - Cardiff is presumably the "South" provider. The buyer most likely cloned that notice and kept Cardiff's PPON (PHVT-4343-HWYR is in Cardiff's merged_identifiers).
- **Rule 1:** these are two separate chartered universities. There is no consortium, and the text names one provider only.
- **Rule 2:** 30953698 is a sound destination.
  - It is "UNIVERSITY OF LIVERPOOL", keyed GBCOHRC000660, with its own PPON PYNH-9859-XVMD and 42 mentions.
  - The other pool row, 10667831 "University of Liverpool" keyed ROYALCHARTERNO000660 (read, 1 mention), is the same charter, so it is the same entity.
- **Verdict:** CONFIRM, high.

## 455 — "Royal College of Psychiatrists" on 30913587 UNIVERSITY OF SUSSEX(THE) (RC000672) → 31561689

- **Notice 46798192** (DHSC, UK5, "Addiction Psychiatry Credential and Regional Tutors", £1.63m):
  - Section `ORG-GB-COH-RC000672` carries only "Royal College of Psychiatrists" and GB-COH-RC000672. Its one role is Tenderer. The buyer (DHSC PPON) is in a separate section.
  - The text says "delivered by Royal College of Psychiatrists (RCPsych)". Sussex is not mentioned anywhere.
  - So the section is **rehome**.
- **Rule 1:** a royal college and a university are separate chartered bodies with no relation.
  - The holder's other 136 mentions all print a Sussex name.
- **Rule 2:** 31561689 is a sound destination.
  - It is "ROYAL COLLEGE OF PSYCHIATRISTS (THE)", keyed GBCOHRC000463, with PPON PNYR-7511-BDRN and 19 mentions.
  - The name matches after folding "(THE)". It is the only pool candidate.
- **Verdict:** CONFIRM, high.
