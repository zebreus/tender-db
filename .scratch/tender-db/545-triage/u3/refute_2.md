# 545 unit 3: adversarial pass over review 2 (2026-10-19)

Scope: u2a/verdicts.json rows 114, 124, 127, 134, 136, 174, 176, 190, 195, as judged in `review_2.md`. I tried to refute every CONFIRM, the one CHANGE-DESTINATION, and each mention marked `rehome`. The two PARK rows are recorded at the end; parking is the conservative outcome, so nothing there was attacked.

## Method
- I re-read the saved bodies first: all 11 evidence notices, the 9 tenders, the holder orgs, and the destination and pool orgs that were already in `bodies/`.
- I made **2 new GETs**, both rc=200, with no retries. Each sat at least 2 s after the previous request under the shared flock (`get_x2.sh`). They are logged in `api_requests.log` and in `refute_2_gets.log`.
  - `/v1/notices/46703079/content`: the Tech Against Terrorism UK6. Review 2 cited it but had not saved its body.
  - `/v1/notices/46742173/content`: the Vohkus tender's UK4. It shows how GMC identifies itself in that procedure.
- I made no /v1/sql calls, no POSTs, no admin calls, no git calls, and no edits outside u3.

## The shared-section check (G1), re-done mechanically
For every evidence notice I counted occurrences of the holder's PPON in the raw body. All 11 give exactly **3**: the section id, the BT-501 value, and one `OPT-300-Tenderer` reference from TPA-1-0. There is:
- no second section carrying that PPON (no `#2` variant);
- no Buyer reference to it;
- no other role.

The buyers always sit in their own sections. So no mention here binds more than one party, and none needs parking on that rule.

## Per row

| idx | name | review 2 | my result |
|---|---|---|---|
| 114 | Hologic Ltd | CONFIRM → 30914006 | not refuted |
| 124 | Renishaw Neuro Solutions Limited | CONFIRM → 31552561 | not refuted |
| 127 | Xiel | CONFIRM → 30974378 | not refuted |
| 134 | Tech Against Terrorism | CHANGE-DEST → 30973170 | not refuted; strengthened by a new GET |
| 136 | ERMC Ltd | CONFIRM → 18928807 | not refuted; weakest row, caveat below |
| 174 | Vohkus | CONFIRM → 31551980 | not refuted; strengthened by a new GET |
| 176 | Bolton NHS Foundation Trust | CONFIRM → 30917930 | not refuted |
| 190 | Gama Healthcare Ltd | PARK (keep) | park stands |
| 195 | Virgin Media Limited | PARK (keep) | park stands |

### 114 Hologic ×2 (46877124 UK6, 46882065 UK7; tender 8690126), holder 31549426 "Teleflex…"
- **Attack: whose PPON is PPCP-8863-HDGN?** Review 1 showed it is a magnet PPON with three claimants: Teleflex ×3, CUH ×2 and Hologic ×2, from different publishers. If it were Hologic's own second registration, the holder org would be the wrong one to strip. But the mentions would still belong on a Hologic org, and the destination is Hologic. So the fix is right in every reading.
- **Attack: is the destination the right entity?** 30914006 "HOLOGIC LTD." GBCOH02722343 carries a merged PPON, PZBR-3269-JJND, which proves a live Procurement-Act-era registrant. The other pool rows are the same company number without the leading zero (2722343) and a VAT number. MyoSure and Fluent are Hologic products.
- **Attack: agent, group or trading name?** Teleflex, Hologic and CUH are unrelated.
- **Result:** not refuted.

### 124 Renishaw ×2 (46795678 UK5, 46819442 UK6; tender 8642804), holder 31538012 GOSH
- **Attack: two same-named destinations.** 31538013 (PPON PRJV-6161-YXYQ, 4 mentions) and 31552561 (GBCOH10375761, 6 mentions) both read exactly "Renishaw Neuro Solutions Limited". A UK registered name is unique at any one time, so these are one entity split over two orgs, and either target is sound.
- **Supporting detail:** GOSH 31538012 and Renishaw 31538013 have consecutive ids, so they were very likely created from one GOSH notice where each printed its own PPON. That makes PPYL GOSH's PPON and PRJV Renishaw's. Sheffield then printed Renishaw with GOSH's PPON.
- **Attack: corporate relation?** GOSH is at most a customer, not a corporate relation. The role is Tenderer only.
- **Result:** not refuted. The merge of 31538013 into 31552561 stays out of scope.

### 127 Xiel ×1 (46835667 UK5; tender 8665378), holder 30913740 NGH
- **Holder:** it carries merged GB-NHS-RNS (NGH's ODS code) and 489 mentions, so the PPON is NGH's.
- **Destination:** XIEL LTD GBCOH11875702 carries Xiel's own merged PPON PGVW-7219-MLVN. That shows a live entity in 2025-26, which rules out a dissolved predecessor. The other pool rows hold only a VAT number.
- **The notice:** it is a Welsh buyer (BCU and NWSSP), and the section binds Xiel alone.
- **Result:** not refuted.

### 134 Tech Against Terrorism ×1 (47157838 UK5, seq 1; tender 8582476), holder 30916247 Ofcom → 30973170
- **New GET 46703079 (UK6, seq 3, 2026-09-23).** It prints the tenderer as "Online Harms Foundation TA/ Tech Against Terrorism" with **GB-COH-11656320**: same tender, same GBP 192,000 bid, single tenderer. Seq 2 (46697744) prints the same.
- So the Home Office itself corrected the seq-1 notice, which carried Ofcom's PPON (130 Ofcom mentions on the holder).
- 30973170 is GBCOH11656320 and already holds both corrected mentions. The provisional 31570425 has no identifier and is the weaker target.
- **Attack: is this the same entity, or a trading-name or agent case?** The printed text is the trading-as form of the same company.
- **Operational note for unit 4:** target_name is "Online Harms Foundation TA/ Tech Against Terrorism", while the printed name is "Tech Against Terrorism". If the apply step name-gates, this is a TA-form match, not a fold match.
- **Result:** not refuted. The CHANGE-DESTINATION is correct.

### 136 ERMC ×1 (46819437 UK7; tender 8656068), holder 31571667 "Isle of Wight NHS Trust" (4 mentions)
- **Attack 1: ownership of PRBY-9272-TQGX is unproven.**
  - The holder is tiny and has no merged GB-NHS-R1F. A large trust buying under its own PPON would show far more than 3 mentions.
  - So PRBY could even be ERMC's own PPON, with the 3 trust mentions as the strays.
  - Even then, the ERMC mention belongs on an ERMC org. Rehoming it to the ERMC org is consistent with any later identifier fix that gives PRBY to ERMC. Moot for this verdict.
- **Attack 2: is the destination the same legal entity?** This is the weakest point of the 7.
  - 18928807 "ERMC Limited" is keyed by a bare "04489340". Its id range points to the pre-PPON era.
  - It has no merged PPON, and the notice prints no company number, so there is no modern corroboration.
  - A dissolved predecessor whose name was later reused cannot be excluded with the permitted reads. There is no name search and no org→notice listing.
  - However, it is the only ERMC row (identified-unique), the name is exact after legal-form folding, and nothing points to a second ERMC.
- **Result:** not refuted, on the cohort's identified-unique standard. If unit 4 wants a stricter bar, this is the row to drop to medium first.

### 174 Vohkus ×1 (46771116 UK7; tender 8639596), holder 30926693 "GMC" (83 mentions)
- **New GET 46742173 (UK4 of the same procedure):** GMC identifies itself only as GB-CHC-1089278. PXLD-1125-PGBY appears nowhere except on the supplier section of the UK7.
- **Attack: could PXLD be Vohkus's PPON?** The holder has 83 GMC-named mentions under PXLD, and the destination carries Vohkus's own PPON, PYHT-6272-TXNX. PXLD is GMC's registration, put on the supplier.
- **Destination:** 31551980 GBCOH04142508. The other pool row, 31616633, has the same number without the zero.
- **Result:** not refuted.

### 176 Bolton NHS FT ×1 (47366319 UK7; tender 8829421), holder 30913561 MFT (merged GB-NHS-R0A)
- **Attack: could the funding really go to MFT, for a service at a Bolton site?** The printed name and the title "(Bolton)" agree. Only the PPON points at MFT.
- **Attack: group relation?** Bolton FT (RMC) and MFT (R0A) are separate foundation trusts with no group relation.
- **Role:** Tenderer, meaning recipient of GM Cancer Alliance funding from The Christie, which is consistent with a public-body destination.
- **Destination:** 30917930 carries merged GB-NHS-RMC.
- **Result:** not refuted.

### 190 Gama, PARK: stands
- Not attacked, since keep is the conservative outcome.
- One observation: 31551510 (GAMA PPON) and 31551518 (Finsen PPON) have near-adjacent ids, so they were created in the same ingest window. That fits a copy from an adjacent row, but it neither proves nor rules out a Finsen–GAMA relation.

### 195 Virgin Media, PARK: stands
- Not attacked.
- To my knowledge, GBCOH06024812 is Virgin Media Payments Ltd and Virgin Media Limited is 02591237. This is from memory and not register-checked, so the destination's number disagrees with the printed entity, and keep is right.

## Net
None of the 9 rehome mentions is refuted: 114 ×2, 124 ×2, 127, 134 (to 30973170), 136, 174 and 176. The 2 keep mentions (190, 195) stay kept. ERMC (136) is the one HIGH that rests on name equality alone.
