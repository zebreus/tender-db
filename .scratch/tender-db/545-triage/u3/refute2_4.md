# 545 unit 3, second skeptic pass: Accenture x3 on Walnut (refute2_4, 2026-10-19)

Row: holder 31537611 (WALNUT UNLIMITED LTD, GBCOH01317137), printed "Accenture (UK) Ltd", rehome to 15715826
("Accenture UK Ltd", keyed `04757301`). Guard review item (g).

| notice | section | tender | role | target |
|---|---|---|---|---|
| 46768534 | ORG-GB-COH-01317137 | 8623644 | Tenderer | 15715826 |
| 46788047 | ORG-GB-COH-01317137 | 8595979 | Tenderer | 15715826 |
| 47286834 | ORG-GB-COH-01317137 | 8595979 | Tenderer | 15715826 |

**Result: all 3 refuted (refuted=true). Hold them back from the POST.**

The cohort's premise is wrong. Its rationale calls GB-PPON-PNPX-2447-RCXR "Accenture's PPON", and u3.md guesses
"it probably comes from Accenture's own supplier registration". But Ordnance Survey notice 46731177 (2025-07-22)
prints the **same pair, GB-COH-01317137 plus GB-PPON-PNPX-2447-RCXR, under "WALNUT UNLIMITED LTD"**.

So the identifiers on these 3 sections are the holder's own supplier record, not a buyer's mis-print of someone else's
number. By 2025-12 that record shows "Accenture (UK) Ltd", and later "ACCENTURE (UK) LTD - FCA". This looks like a
renamed or taken-over record, probably after an acquisition or a group relation. It is not the 545 defect (a name
printed with an unrelated holder's identifier).

The public data does not settle which legal entity signed these contracts:
- Walnut Unlimited Ltd, 01317137: the record's company number;
- Accenture (UK) Ltd, 04757301: the record's name.

## GETs (4 new, all rc=200, no retries; script `refute2_4_get.sh`, log `refute2_4_gets.log`)

1. `/v1/organizations/31537611`: WALNUT UNLIMITED LTD, GBCOH01317137, 8 mentions, not provisional, no merged identifiers.
   The holder is unchanged since unit 2b: 5 mentions printed as Walnut, and the 3 Accenture mentions.
2. `/v1/notices/46794414/content`: FCA/BoE "Consultancy & Skilled Persons Framework 3", UK6, 2026-03-03. It prints
   "ACCENTURE (UK) LTD - FCA" on PNPX-2447-RCXR only. Every supplier on this notice is PPON-only, so it shows no company number.
   This is one of row 31565018's 2 mentions.
3. `/v1/notices/46731177/content`: Ordnance Survey "Insight Framework for Customer Intelligent and Strategic Insight",
   UK6, 2025-07-22. The single Tenderer is **"WALNUT UNLIMITED LTD", GB-COH-01317137 + GB-PPON-PNPX-2447-RCXR**, lot value £400k.
4. `/v1/notices/46832046/content`: Sport England "SE1264 - This Girl Can - Place Pilots Evaluation", UK5, 2026-06-19.
   The single Tenderer is "Walnut Unlimited LTD", GB-COH-01317137, with no PPON.

Reused: `v1_notices_{46768534,46788047,47286834}_content.json` and
`v1_organizations_{15715826,31565018,9766497,31549294,22952038,23157128}.json`. I also used `u2b/b1.out.json`
(the holder's name pool and notice spans) and `u2a/s3.out.json` (31565018's notices 46794414 and 46794546).

## Timeline of the record (COH 01317137 / PPON PNPX-2447-RCXR)

| date | notice | buyer | printed name | ids printed | subject |
|---|---|---|---|---|---|
| 2025-07-22 | 46731177 | Ordnance Survey | WALNUT UNLIMITED LTD | COH + PNPX | customer/strategic insight framework (research) |
| 2025-12-04 | 46768534 | Bank of England | Accenture (UK) Ltd | COH + PNPX | DMS migration, £4m (IT) |
| 2026-02-12 | 46788047, 47286834 | CCS | Accenture (UK) Ltd | COH + PNPX | Media and Creative Services, lot 4 (with AMV BBDO, VCCP, M&C Saatchi, Havas…) |
| 2026-03-03 | 46794414 (+46794546) | FCA | ACCENTURE (UK) LTD - FCA | PNPX only | consultancy framework |
| 2026-06-19 | 46832046 | Sport England | Walnut Unlimited LTD | COH only | evaluation (research) |

Unit 2b's name pool shows two more Walnut-named mentions on the holder: 46743175 and 46751142. I did not read them.

## Checks

| Question | Finding |
|---|---|
| Is the printed name a different legal entity from the holder? | **Not settled.** Accenture (UK) Ltd (04757301) and Walnut Unlimited Ltd (01317137) are different companies. But the name sits on the holder's own supplier record: the same COH + PPON pair is printed as Walnut 4.5 months earlier. That leaves three readings, which the task lists as grounds to refute: (i) Walnut's record renamed to its new parent's name ("the holder itself" under a new label, so the legal party is 01317137); (ii) Walnut's business moved into Accenture (UK) Ltd, which reuses the record with a stale company number (the legal party is 04757301); (iii) some other group or sister arrangement. Walnut still bids under its own name in 2026-06 (46832046), so it was not wound up. |
| Is the target the same legal entity as the printed name? | Yes, if the printed name is taken at face value. 15715826 serves "Accenture UK Ltd", is keyed `04757301`, has 86 mentions and has PPON PYHC-2313-BXRY merged. 04757301 is Accenture (UK) Limited's number. The postcode-suffixed row 22952038 `04757301EC3M3BD` (EC3M 3BD is Accenture's Fenchurch St office) and four Accenture-named rows on 4757301 / 0475730 agree. This check passes. |
| Does the section bind only this party? | Yes, in all 3. `ORG-GB-COH-01317137` has one BT-500 and is referenced once: OPT-300-Tenderer (TPA-1-0 on BoE, TPA-5-2 on CCS). The buyer sits in its own section (BoE RC000042 / CCS PBZB-4962-TVLR). 47286834 is identical to 46788047 section by section (same OCDS id ocds-h6vhtk-0502d0, same OPP-012), so it is a duplicate publication. |
| Would moving lose or falsify information? | Possibly. Under reading (i), the move takes a £4m BoE award and a CCS framework place away from the legal party that the record's company number names. Under reading (ii), it is right. In either case the move does not touch the cause: the PNPX record. New notices from that record will keep binding to Walnut (u3.md already says so), and the FCA row 31565018 (PNPX, 2 mentions) stays split from both. |
| Does the subject matter help? | It favours Accenture. A £4m DMS migration is IT work. CCS lot 4's other winners are all advertising agencies, which fits Accenture Song. Walnut's own notices are research or evaluation work. So the real-world bidder was probably the Accenture side. But that is an inference about who did the work, not about the legal entity, and the record's identifiers say otherwise. |

## Why refuted rather than a caveat

- The two other pieces of evidence the cohort leans on do not survive.
  - The PPON is not an independent Accenture identifier. It is the holder's own PPON (46731177).
  - "Two unrelated buyers print the pair" now cuts the other way. Three unrelated buyers print it (OS, BoE, CCS), under
    two different names. So it comes from the supplier record, and that record was Walnut's first.
- What is left is the free-text name on that record against its company number. That is the "former name / sister body /
  the holder itself" situation the bar excludes. Unit 3 already parks precedents like this: Northumbria's own subsidiary
  (452/466), Gama/Finsen ("a group relation cannot be ruled out"), and the CUH inverted-premise PPON (an identifier
  verdict, not a rehome).
- The right fix belongs to the identifier classes (470/473 PPON ownership, 469 altid), not 545. Decide who PNPX-2447-RCXR
  denotes now. If that is Accenture (UK) Ltd, an identifier verdict moves these 3 mentions, the FCA row 31565018 and future
  notices together, and treats 01317137 on PNPX-bearing sections as stale.

## What would lift it

- A register read of 01317137 (Walnut Unlimited Ltd): its name history, PSC/parent and filing status. If it shows Walnut
  hived up into Accenture (UK) Ltd, or dormant since late 2025, reading (ii) holds and the rehome, better the identifier
  verdict, is right.
- Or a buyer-side document (BoE or CCS award or contract) naming the contracting company number.

## Notes for unit 4 (not refutations)

- If the 3 are dropped, also drop tenders 8623644 and 8595979 from `touched_tenders.json`. Both appear only in this row
  (u3.md, "Only from the additions").
- Correct `guards.md` (g) and the u3.md observation. PNPX-2447-RCXR is printed as Walnut's on 46731177, so "Accenture's
  own PPON split (PNPX vs PYHC)" is really "Walnut's record renamed to Accenture". The 470/473 follow-up should say so.
- The holder has 8 mentions and none was read before this pass except these 3. 46743175 and 46751142 are still unread.

## Verdicts

| notice | section | refuted | why |
|---|---|---|---|
| 46768534 | ORG-GB-COH-01317137 | **true** | The section prints the holder's whole identifier set (01317137 + PNPX-2447-RCXR), and OS 46731177 prints that same pair as WALNUT UNLIMITED LTD in 2025-07. "Accenture's PPON" is wrong. This is Walnut's supplier record relabelled "Accenture (UK) Ltd", so the legal party (01317137 or 04757301) is not settled. The subject (IT, £4m) favours Accenture, but that is not proof. |
| 46788047 | ORG-GB-COH-01317137 | **true** | Same record and same pair under the Accenture name. A single Tenderer ref (TPA-5-2) on CCS lot 4. Same unsettled identity: a renamed holder record, not a mis-printed identifier. Needs an identifier-level decision on PNPX-2447-RCXR first. |
| 47286834 | ORG-GB-COH-01317137 | **true** | Identical to 46788047 section by section (same OCDS id and OPP-012), so it stands or falls with it. |
