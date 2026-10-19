# 545 unit 3: adversarial refutation of reviewer 6 (2026-10-19)

**Scope.** u2b/verdicts2b.json rows 369, 412, 419, 440, 444, 446 and 455: 7 rows, 8 mentions. Reviewer 6 confirmed all 7 rows and marked all 8 mentions rehome, high.

**Reads.**
- First I re-read the bodies already saved under `bodies/`: 7 notices and 17 orgs.
- Then I made **7 new public GETs**, all rc=200 with no retries, using `refute6_get.sh`.
  - Each GET waits at least 2 s, and also at least 2 s after the last line of the shared `api_requests.log`.
  - Each one is logged in `api_requests.log` and in `refute6_gets.log`, and its body is saved under `bodies/`.
  - The 7 paths: /v1/tenders/8669742, 8644216, 8715367, 8651250, 8735507 and 8642384, plus /v1/notices/46844176/content.
- No /v1/sql, no ssh, no admin endpoints, no POSTs, no git, and no edits outside `u3/`.

**Result: 0 of 7 rows refuted and 0 of 8 mentions refuted.**
- I tried every attack angle on every row.
- Shared section: each mention's section carries one name, one identifier and the country. Each is referenced exactly once, as OPT-300-Tenderer, in the notices I re-read.
- Same-entity check: two rows (RCPsych and Thomas Bow) are now independently confirmed by later notices of the same tender. These already bind the printed name to the proposed destination.
- One part of reviewer 6's reasoning on Dell is wrong (see 440). The conclusion still stands on other evidence.

## 369: "Eastern Counties Scaffodling Services" on 30915302 NCSL → 31573542. NOT REFUTED

- **Shared section.** Notice 46810761 has two org sections:
  - `ORG-GB-COH-12656036`: NCSL, with COH 12656036 and PPON PWTD-8252-JQTZ, referenced only as Procedure-Buyer.
  - `ORG-GB-PPON-PWTD-8252-JQTZ`: ECSS only, referenced only by TPA-1-0 as Tenderer.
  - The PPON is reused across sections, but the section ids differ. That is not a shared section.
- **Tender 8651250 (new GET):** one version only, 46810761. It shows NCSL as Procedure-Buyer and "Eastern Counties Scaffodling Services" as Tenderer, both bound to 30915302. That is exactly the misbind.
- **Holder relation.** NCSL is a council-owned (Teckal) repairs and environmental-services company buying from a scaffolding contractor. A buyer and its own supplier are not one entity. The destination row has its own PPON, which rules out a trading name of NCSL.
- **Destination.** 31573542 is keyed GBPPONPHGP1225WWPN, has 4 mentions, and its served name repeats the same misspelling.
  - A misspelling repeated exactly is strong evidence that it is the same printed entity.
  - The public API cannot list that row's notices, so who registered PHGP-1225-WWPN is not proven. Nothing contradicts it.
  - A correctly spelled "Scaffolding" row, if one exists, would not be in the name pool. Later it could only be a duplicate to merge, not a different entity.
- **Residual risk:** this is the reviewer's own note. The holder keeps PWTD-8252-JQTZ in merged_identifiers, so any re-fold keyed on PPON rather than on the (notice, section) verdict would pull the mention back.

## 412: "Crown Paints Ltd" on 31597395 (GBCOH436135, PPG) → 10890039. NOT REFUTED

- **Shared section.** In notice 46951039, Lot 1 has three tenders (STAT: 3 tenders):
  - TPA 1-0 → `ORG-GB-COH-436135` (Crown only);
  - TPA 1-1 → `ORG-GB-COH-00436135` (PPG);
  - TPA 1-2 → Brewers.
  - So Crown and PPG are two separate tenders in the same lot. The section is not shared.
- **Name or number wrong?** If 436135 were really PPG and the name were the error, PPG would hold two of the three Lot 1 tenders. Lot 4 lists Crown (6624576) and PPG (00436135) side by side, which matches two competing paint suppliers. So the name is right and the number (PPG's, with its zeros stripped) is wrong.
- **Holder relation.** Crown (Hempel) and PPG are rivals. There is no group relation.
- **Tender 8735507 (new GET):**
  - The Lot 1 Crown bid is bound to 31597395, the misbind. The Lot 4 Crown bid is bound to **31623833** (GBCOH6624576).
  - The pick of 10890039 ("06624576", PPON PVZT-1732-ZPCG, 15 mentions) is still right, because both rows carry the same company number.
  - Side effect: after the rehome, this notice's two Crown mentions sit on two duplicate rows of one entity. That is for the altid backlog, not a reason to refute.

## 419: "Thomas Bow Limited" on 31632003 (GBCOH703317, Telent) → 14183429. NOT REFUTED

- **Shared section.** In notice 46980862:
  - Lot 4's TPA 4-1 → `ORG-GB-COH-703317` (Telent).
  - Lot 6's TPA 6-1 → `ORG-GB-COH-703317#2` (Thomas Bow only, nrefs=1).
  - Lot 5 has Thomas Bow under its own number, 4503934.
  - These are distinct section ids, so the section is not shared.
  - The parser split the reused id by name. Lot 6, Carriageway Maintenance, fits a surfacing contractor. Lot 4, Traffic Signals, fits Telent.
- **Tender 8715367 (new GET):**
  - The 2026 awardUpdate+contractUpdate notice 47327056 lists "THOMAS BOW LIMITED" bound to **14183429**, the proposed destination.
  - The same tender also binds Thomas Bow in Lot 5 to 31600286 (GBCOH4503934), which has the same number.
  - This is independent confirmation that the destination is right.
- **Holder relation.** Thomas Bow (a Nottingham surfacing contractor) and Telent (telecoms and traffic technology) are unrelated.

## 440: "Dell Corporation" on 31582801 (OC403062, Hand and Millar) → 31535972. NOT REFUTED, but one of the reviewer's arguments fails

**Flaw in reviewer 6's agent check.** It called OC403062 "a carry-over from the buyer's next notice", for two reasons:
- The H&M notice 46844339 has the "same buyer PPON".
- That notice was "published 30 minutes later".

Neither reason holds up:
- PHVX-4316-ZVGZ is the MOD-wide PPON, shared by many unrelated MOD commercial teams, so "same buyer" means very little.
- The H&M notice (15:38) came out after the Dell UK5 (15:08). A carry-over normally runs from an earlier draft into a later one.

So that argument proves nothing about agency.

**Why the row still stands:**
- The text names Dell three times as the party dealt with: "Direct engagement with Dell as the hardware is exclusively supplied by Dell", "Dell supplied a quote", and "awarded based on TEPAS 2 FW".
- "TEPAS 2" / "TEPS 2" most likely means CCS RM6098, Technology Products & Associated Services 2. That is a framework for technology products and resellers, and Dell Corporation Ltd sits on it directly.
- H&M's own notices show an assurance consultancy: DOS7 (46842841) and RM6310 Audit & Assurance (46844339). That is not a hardware reseller. A consultancy reselling Dell hardware support under a products framework is implausible.
- Tender 8669742 (new GET) has two versions:
  - UK2 46844176 (12:30): read in full. It has no supplier and repeats the same Dell text.
  - UK5 46844315.
  - No other notice names H&M as the contractor.
- The section holds only "Dell Corporation" with OC403062 and is referenced once, as Tenderer.

**Destination.** 31535972 is DELL CORPORATION LIMITED, GBCOH02081369, with PPON PWNY-9527-CNQX and 42 mentions. This is the UK entity, the right counterparty for an MOD contract. The pool alternatives carry the same number or a VAT-shaped key for the same entity.

## 444: "West Yorkshire Combined Authority" on 31548301 (OC451571, BTQ LLP) → 30913749. NOT REFUTED

- **Notice 46794970.** Buyer: Sussex PCC (FTS sections). The Tenderer section `ORG-GB-COH-OC451571` holds WYCA only and is referenced once.
  - The lot text says "Direct award under a national agreement contract with PNLD with the West Yorkshire Combined Authority".
  - The procedure title is literally "West Yorkshire Combined Authority".
- **Tender 8642384 (new GET):** one version only, 46794970. WYCA is the Tenderer, bound to 31548301 (BTQ).
- **Role.** This is a public body on the supplier side, which is genuine: PNLD is run within WYCA's policing function. The flag that matters is a Buyer role on a supplier's identifier, and it does not apply.
- **Holder relation.** BTQ LLP is a Bristol regeneration partnership with no relation to WYCA.
- **Destination.** 30913749, keyed on PPON PHYG-7328-ZLQJ, with 182 mentions and an exact name match.
  - The other row, 31565528 (GBCOHCO451571, register_mismatch, 1 mention), is a junk-key split of WYCA. That makes it worse, not a different entity.
  - Note: a second publisher printed WYCA with "CO451571". So WYCA seems to be printed with a BTQ-like number more than once, possibly from a bad supplier-directory record. That does not change this verdict.

## 446: "University of Liverpool" ×2 on 30914001 (Cardiff, PPON PHVT-4343-HWYR) → 30953698. NOT REFUTED

- **Notices 46839286 (UK6) and 46843620 (UK7)** are both on procedure ocds-h6vhtk-06c8ec, "MPH Qualification - North".
  - In each, section `ORG-GB-PPON-PHVT-4343-HWYR` holds only "University of Liverpool" and is referenced once, as Tenderer.
  - The buyers HEIW and NWSSP are in their own sections.
  - Neither body contains the word "Cardiff". The text calls Liverpool "the only appropriate provider ... North Wales".
- **Holder relation.** There is no joint programme and no consortium.
- **If PHVT-4343-HWYR were really Liverpool's PPON** and Cardiff's merge were the error, the mention would still belong on Liverpool. The verdict holds either way.
- **Destination.** 30953698 is keyed GBCOHRC000660, with PPON PYNH-9859-XVMD and 42 mentions. 10667831 carries the same charter number.

## 455: "Royal College of Psychiatrists" on 30913587 (RC000672, Sussex) → 31561689. NOT REFUTED

- **Notice 46798192 (UK5, DHSC).** The section `ORG-GB-COH-RC000672` holds only RCPsych and is referenced once, as Tenderer. The text says "delivered by Royal College of Psychiatrists (RCPsych)" and never mentions Sussex.
- **Tender 8644216 (new GET):** the same procedure's UK6 (46798275) and UK7 (46807006) both bind "Royal College of Psychiatrists" to **31561689**, the proposed destination. Only the UK5 binds it to Sussex.
  - This is independent confirmation by the publisher's own later notices.

## Summary

| idx | holder | name | destination | refuted |
|---|---|---|---|---|
| 369 | 30915302 | Eastern Counties Scaffodling Services | 31573542 | no |
| 412 | 31597395 | Crown Paints Ltd | 10890039 | no |
| 419 | 31632003 | Thomas Bow Limited | 14183429 | no (confirmed by 47327056) |
| 440 | 31582801 | Dell Corporation | 31535972 | no (reviewer's carry-over argument fails; the text and framework carry it) |
| 444 | 31548301 | West Yorkshire Combined Authority | 30913749 | no |
| 446 | 30914001 | University of Liverpool ×2 | 30953698 | no |
| 455 | 30913587 | Royal College of Psychiatrists | 31561689 | no (confirmed by 46798275 and 46807006) |
