# 589 — Find a Tender's `procuringEntity` is served as Procedure-Buyer, and its `roleDetails` is dropped

Status: needs-triage — filed 2026-10-24 from 572 unit 2's close (F1 of `572-triage/decision.md` "Follow-up issues";
`572-triage/u2/close.md`). Next: unit 1, measure with a dry census over the archived FTS releases (party role sets,
notices whose only buying party is a `procuringEntity`, `roleDetails` presence and shapes, and `procuringEntity`
parties that share a buyer's identifier). Then decide whether `procuringEntity` gets a distinct role or keeps the fold
into `OPT-300-Procedure-Buyer`, and what either does to `v_tender_buyers` (ADR-0015) and `?buyer=`. Owns the host-case
role (STAR on Trafford, Ardal on Cardiff), Gleeds' duplicate College of Policing party, and NWSSP's 2,123
`GB-NHS-RQF` procuringEntity mentions on provisional 15641552.
Kind: data model / source fidelity (FTS party roles, buyer side)
Relates to:
- 572 (rule 572-L: a buying agent's identifier binds the member body it buys for; F1 is its follow-up, and reopen
  trigger 3 fires if this issue adopts a distinct role);
- 456 (design 3: `mention_name` serves the printed name; the host and Gleeds cases bind by identifier);
- ADR-0015 (the SQL surface: `v_tender_buyers` keys on `role LIKE '%uyer%'`), ADR-0004 (strict ingestion and the
  checklist census);
- 513 / 519 (NWSSP is split across 30913738 and provisional 15641552), 470 / 473 (Gleeds' split), 386 (UVDB
  classifications under `parties[].details`).

## Verify

    curl -s https://tenders.zebreus.click/v1/tenders/8655231 | jq -c '[.parties[] | select(.role=="Procedure-Buyer") | [.organization_id, .mention_name]]'
    curl -s https://tenders.zebreus.click/v1/tenders/8580124 | jq -c '[.parties[] | select(.role=="Procedure-Buyer") | [.organization_id, .mention_name]]'

- **open (2026-10-24, read in 572 unit 2's close):**
  - Line 1 reads `[[30916230,"Rochdale Metropolitan Borough Council"],[30917275,"Star Procurement"]]`. STAR is the
    notice's `procuringEntity` ("Managing the Tender"), printed under Trafford Council's PPON, so the host Trafford is
    served as a second buyer of Rochdale's procurement.
  - Line 2 reads `[[30914436,"COLLEGE OF POLICING LIMITED"],[30914436,"GLEEDS COST MANAGEMENT LIMITED"]]`. Gleeds is
    the `procuringEntity` ("Specification preparation/ evaluation") under the College of Policing's PPON, so the
    College is served twice as Procedure-Buyer.
- **done**, whichever unit 1 decides:
  - **(a) Distinct role adopted.** On both lines the `procuringEntity` row no longer reads Procedure-Buyer: line 1
    keeps only `[30916230,…]` and line 2 keeps one College of Policing row. The decided role is documented in /docs,
    in `v_tender_buyers`' note and in `fts/checklist.rs` (whose `parties[].roleDetails` entry stops being "owed" if it
    is kept). 572 is re-read under its reopen trigger 3.
  - **(b) The fold is kept.** Both lines are unchanged. The decision and the census numbers are recorded here, and the
    /docs caveat (572's "Codes and identities" bullet) still says so.

## What was found

From 572 unit 1 (`572-triage/measure.md`, 13 exemplar notices):

- **Both roles fold into one.** The FTS parser maps both `buyer` and `procuringEntity` to `OPT-300-Procedure-Buyer`
  (`crates/ingest/src/fts/parse.rs:249`). It drops the free-text `roleDetails` ("on behalf of …") and the party's
  `details.classifications` (UK_CA_TYPE and the rest). Since 572 U2b (3fb8803, deployed) the FTS checklist records both
  drops as Ignored("owed: …") and says that `procuringEntity` becomes Procedure-Buyer.
- **7 of the 13 exemplars publish the agent as a separate `procuringEntity`, with `roleDetails`:** E4, E5, E6, E7, E9,
  E11 and E13. The other 6 have a buyer party only. tender-db serves both parties as Procedure-Buyer on all 7.
- **The host case.** An agent printed under its host council's PPON binds to the host, so the host is served as buyer
  of another body's procurement:
  - STAR on Trafford: 8655231, notice 46817944;
  - Ardal on Cardiff: 8621643, notice 46765017.
- **The duplicate party.** Gleeds as `procuringEntity` under the College of Policing's PPON gives the College two
  Procedure-Buyer rows on 8580124 (notice 46693105).
- **NWSSP's own procuringEntity sections** print Velindre's ODS code `GB-NHS-RQF`. They bind to provisional 15641552
  (2,123 mentions), not to NWSSP's PPON org 30913738.
- **No structured on-behalf-of field exists in the source.** `roleDetails` is the only statement of the relation.
- **Scale beyond the exemplars is unmeasured.** No census counts FTS releases by party role set today. The FTS
  checklist census in `tests/fts.rs` runs on fixtures, and no fixture publishes `roleDetails`.

## Proposed units

1. **Measure (S–M).** A dry census job over the archived FTS releases, not a `/v1/sql` loop: it is a corpus walk. It
   counts:
   - parties by role set: {buyer}, {procuringEntity}, {buyer + procuringEntity}, and `centralPurchasingBody`;
   - notices whose only buying party is a `procuringEntity`;
   - `roleDetails` presence, and its shapes ("on behalf of", "managing the tender", evaluation-only, …);
   - `procuringEntity` parties that share a buyer's identifier (the Gleeds shape) or print a third body's identifier
     (the host shape).

   Any new `Spec` arm wraps its body in `Box::pin` (CLAUDE.md).
2. **Decide.** A distinct role (for example an FTS analogue of a procurement service provider), with what it does to
   `v_tender_buyers` under ADR-0015, to `?buyer=` and to the role vocabulary, or keep the fold and record why. Include
   whether `roleDetails` is stored (as a party note) and whether UK_CA_TYPE is folded (the BT-11 analogue the eForms
   profile already stores).
3. **Build and drain** per the decision: a parser change, so a scoped FTS reparse plus project, with read-back bars on
   the exemplars above and a salted sample.
