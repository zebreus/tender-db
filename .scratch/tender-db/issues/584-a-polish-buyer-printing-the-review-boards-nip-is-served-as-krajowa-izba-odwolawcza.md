# 584 — a Polish buyer printing the review board's NIP is served as "Krajowa Izba Odwoławcza"

Status: ready-for-agent — **unit 1 SIZED 2026-10-24 (hourly audit, 7 bounded keyset reads of organization_mentions by organization_id = 49, `584-u1/`)**: of org 49's 30,567 mentions, 30,529 are KIO/UZP spellings (26,807 "Krajowa Izba Odwoławcza") — the identity is right; only **38 carry another name**, of which 6 are still KIO/UZP (5 "National Appeals Chamber", 1 typo) and **32 from 15 bodies are mis-keyed by KIO's NIP 5262239325**: Uniwersytet Warszawski 13, MPWiK Warszawa 5, Terra Mota 2, and 12 companies with 1 each (`584-u1/org49_foreign.json`). NEXT: decide the remedy — rehome those 32 mentions through the existing reviewed rehoming packet (545/519 precedent) and/or a name-agreement guard on `national` PL identifier binds that refuses a mention whose name shares no token with the identifier's organization; then drain. Small W pocket, not a corpus defect. Was: needs-triage — filed 2026-10-24 from issue 576 unit 4's read-back (wf_a3ca374b-a30, item 24; re-read by hand the
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
