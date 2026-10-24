# 584 — a Polish buyer printing the review board's NIP is served as "Krajowa Izba Odwoławcza"

Status: ready-for-agent — **cohort 584 APPLIED and READ BACK 2026-10-24 ~22:40Z**: wet 2788 moved 32 mentions (261 party + 19 bid-party rows, 18 Tenders, 49 notices re-queued); project 2789 corrected the 18 (parties 33, bid parties 15, result winners 14 version tables). Read-back: mentions 38/38 as expected (RB-EXIT 0); 306091 buyer [7788] (Verify done), 914542 tenderer/winner/bid all 13575097 (WTU), 997224 buyers [848, 3889], 241175 bids J&M 18347269 + BICO 15949738; no non-review party on 49. NEXT: unit 3 — org 36 (same NIP, 205,496 mentions, 133 foreign tenderer mentions on 91 Tenders): an in-app census of national-identifier binds on 36 + 49 by role and name disagreement, a second cohort under 545's rules, then the guard decision (`584-u2/guard-spec.md`). Previously: in-progress — **cohort 584 POSTED 2026-10-24 21:5xZ** (`{"cohort":"584-2026-10-24","recorded":38}`; precheck: pending 181 before, 0 foreign high rehomes, 0 of 38 keys present, expected dry pending 219 / eligible 32); apply-rehoming dry 2785: 219 pending, 32 eligible, 32 moves, 0 no-op, 4 missing_target (older cohorts'); `plan_check.py` → PLAN == COHORT (32/32, extra 0, missing 0), PLAN-EXIT 0, plan age 124 s. Wet 2788 + project 2789 queued (behind 519 M6's 2786/2787). Was: ready-for-agent — **unit 2 COHORT READY 2026-10-24 ~20:40Z (wf_d711e530-9fc: preparer, challenger, finalizer; `584-u2/`)**: cohort `584-2026-10-24` = 32 rehome to 15 existing organizations (each keyed by the body's own register number, confirmed against KRS / the MF VAT white list; Uniwersytet Warszawski 7788 ×13, MPWiK 848 ×5, Terra Mota 14553663 ×2, 12 others ×1; 18 Tenders) + 6 keep (KIO/UZP spellings in review roles), 0 parked; challenger GO with no row change; validated against `record_rehoming` (`final/validate.py`), sha256 cad0149f…. NOTE the NIP 5262239325 is UZP's (KIO sits inside UZP), so UZP-named mentions are right. Runbook `584-u2/runbook.md` (POST → apply-rehoming dry → `plan_check.py` PLAN == COHORT by tuple, not count → wet → project → read-back 306091 buyer 7788, 914542 WTU 13575097); runs when the queue is idle (after 577 slices and 519 M6). **Scope widened: org 36** ("Krajowa Izba Odwoławcza", the SAME NIP at a lower id, 205,496 mentions) takes every new bind of this number (lowest id per identifier, canonical.rs:21255): 133 foreign-named tenderer mentions on 91 Tenders (125 names: BGK, Politechnika Gdańska, Remondis KROeko …), 309 winner rows on 96 Tenders, buyer side unsized (Gmina Skaryszew, Ministerstwo Finansów, Platforma eZamówienia seen this week). Unit 1 read only org 49. The guard spec (`584-u2/guard-spec.md`, revised for challenge D1–D6: model org 36, require the review-role condition — notice 25028774's "Krajowa Izba Gospodarcza" section is really KIO — stem matching, the VAT-form key) waits on a census over 36 + 49; a 36/49 merge must not come before the guard. Also: mirror case org 11786 (KIO name, UW's REGON, 173 mentions), unmerged twins Zarys 976/9241, Remondis 15146954/20376473, Digitex 20111694/19005947. NEXT: (1) apply cohort 584 per the runbook when the queue idles; (2) unit 3: census of national-identifier binds on 36 + 49 (an in-app dry job: name disagreement per mention, by role) → second cohort for org 36 under 545's rules (consortium and sister-company parks) → guard decision. Was: ready-for-agent — **unit 1 SIZED 2026-10-24 (hourly audit, 7 bounded keyset reads of organization_mentions by organization_id = 49, `584-u1/`)**: of org 49's 30,567 mentions, 30,529 are KIO/UZP spellings (26,807 "Krajowa Izba Odwoławcza") — the identity is right; only **38 carry another name**, of which 6 are still KIO/UZP (5 "National Appeals Chamber", 1 typo) and **32 from 15 bodies are mis-keyed by KIO's NIP 5262239325**: Uniwersytet Warszawski 13, MPWiK Warszawa 5, Terra Mota 2, and 12 companies with 1 each (`584-u1/org49_foreign.json`). NEXT: decide the remedy — rehome those 32 mentions through the existing reviewed rehoming packet (545/519 precedent) and/or a name-agreement guard on `national` PL identifier binds that refuses a mention whose name shares no token with the identifier's organization; then drain. Small W pocket, not a corpus defect. Was: needs-triage — filed 2026-10-24 from issue 576 unit 4's read-back (wf_a3ca374b-a30, item 24; re-read by hand the
same hour). Next: unit 1, size it (how many served Procedure-Buyer / buyer-shaped parties bind to org 49 while their own
mention name is another body) and find where the NIP comes from (publisher, eForms field, or our identifier read).
Kind: data quality / wrong organization served (W: a consumer reading the buyer gets the review board)
Relates to:
- 469 / 514 (identifier-keyed binding: a national identifier decides the organization);
- 576 (found by its read-back; not caused by it — the buyer binding predates the touchpoint fold);
- 545 / 513 (earlier mis-keyed identity cohorts).

## Verify

    curl -s https://tenders.zebreus.click/v1/tenders/306091 | jq -c '[.parties[] | select(.role=="Procedure-Buyer") | .organization_id]'

- **open (2026-10-24)**: `[49]` — org 49 is "Krajowa Izba Odwoławcza" (PL, national identifier 5262239325, 30,567 mentions).
- **done**: the buyer is served as an organization named for Uniwersytet Warszawski (or another org whose head name is the
  buyer's published name), and the cohort unit 1 sizes no longer binds to org 49.

## What was found

- Tender 306091 ("Dostawa trenażerów … Wydziału Medycznego Uniwersytetu Warszawskiego"), notice 25710042: the buyer
  section ORG-0001 prints BT-500 "Uniwersytet Warszawski", Krakowskie Przedmieście 26/28, 00-927 Warszawa,
  dzpmed@uw.edu.pl — and the national identifier 5262239325 (576-u4 `fetch-notes.md` item 7; `sample.json` item 24).
- Org 49 holds identifier_kind `national`, identifier `5262239325`, head name "Krajowa Izba Odwoławcza", 30,567 mentions.
  The identifier-keyed bind sends the university's buyer mention there, so `Procedure-Buyer`, `Lot-TenderEval`,
  `Lot-TenderReceipt` and `Lot-DocProvider` on 306091 all serve "Krajowa Izba Odwoławcza".
- 5262239325 is plausibly the NIP of Urząd Zamówień Publicznych (KIO sits inside UZP), i.e. the review body's number,
  printed by the publisher (or a platform template) into the buyer's identifier field. To confirm in unit 1: read the
  raw BT-501 of ORG-0001 in 25710042, and count how many distinct buyer names print it.
- Scale unknown. Org 49's 30,567 mentions are expected for the review board (most Polish eForms notices name KIO); the
  question is how many of them are buyer sections with another name.

## Proposed units

1. **Size and source (S).** Bounded reads: `tender_version_parties` by `organization_id = 49` (index
   `tender_version_parties_org`, windowed) for roles Procedure-Buyer/Lot-TenderReceipt/Lot-TenderEval/Lot-DocProvider, with
   their mention names (organization_mentions PK); count distinct mention names ≠ KIO/UZP. Read the raw identifier field
   for 3 of them. Decide: (a) treat a known review-body/authority identifier printed under another body's name as not
   an identity for that mention (a reviewed exclusion list, like 545's), or (b) a name-agreement guard on
   identifier-keyed binds for `national` PL identifiers.
2. **Build (S/M)** the chosen guard with tests; **3. drain** (refold of the cohort's notices) and Verify.
