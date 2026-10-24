# 527 — an eForms tenderer marked "(niet gegunde partij)" is served as a winner

Status: ready-for-agent — UNIT 2 DONE 2026-10-13. b849341 (gate green, review fixes) and the fix 1c0b1ce (gate green) are deployed. The classifiers now read a name without the mark: the first refold, 2386/2387, had dropped `Facilicom Solutions (niet gegunde partij)` as a void lot under 510's Dutch stem. Refold 2389/2390 of 24400562 verified on tender 229981: v3's RES-0001 serves Planon (16387876) alone, and all five keep their bids and their `Tenderer` role. Facilicom is back on TEN-0001/0006. The five mention names read clean. NEXT: unit 3, a decision. v2's RES-0001 (24424017, unmarked) still serves all five as winners because both award notices' rounds stand, under different BT-701. Measure first: how many tenders carry two award notices whose results cover the same lot with the same contract (BT-150). Then decide whether a later award notice's result supersedes an earlier one's. Org heads 30533823/4 stay the marked spellings until 524. The value-sum nit (marked losers' bids still summed) has no reach today (all bids value null).
Was: ready-for-agent — triaged 2026-10-13, unit 1 MEASURED with the new `name-phrase-census` (17a7c1f; job 2385, `../527-niet-gegunde/name-phrase-census-2385.json`). It walked 5,679,437 organization names and 49,600,595 mention names. The annotation reaches exactly one tender: 229981, Gemeente Maastricht's MJOP inspections. Every annotated name sits on its award notice v3, 24400562 (00798764-2024, published 2024-12-30): four losers `(niet gegunde partij)` (Facilicom → org 6734977, PVM Eindhoven → 7201067, Humble Inpect → 30533823, Provadi → 30533824) and the winner `(gegunde partij)` (Planon → 16387876). The earlier award notice v2, 24424017 (00792573-2024), names the same five without any annotation. Both RES-0001 serve all five as winners. NEXT: unit 2, the build (S); see "Triage decision" below.
Was: needs-triage — filed 2026-10-12 from 520 unit 1 (§4.5).
Kind: data quality (results; roles)
Relates to: 520, 524 (served spelling).

## What is wrong
In Gemeente Maastricht 24400562 (and 24424017), RES-0001 (BT-142 selec-w) has OPT-320 listing TEN-0001..0006, and
CON-0001's BT-3202 lists TEN-0001..0005. The publisher names the real winner `Planon Building Management B.V.
(gegunde partij)` (ORG-0003), but it also puts these tenders in the contract:
- TEN-0002 → ORG-0004 `Humble Inpect B.V. (niet gegunde partij)` (KvK 69341172, org 30533823);
- TEN-0004 → ORG-0005 `Provadi B.V. (niet gegunde partij)` (KvK 61820210, org 30533824);
- ORG-0006 PVM Eindhoven B.V. and ORG-0007 Facilicom Solutions have the same shape.
The fold serves each of them as a winning bidder. The annotation is the only sign that they lost.

## First unit
Measure how far the annotation spreads (name_norm range for `gegunde partij`, with roles). Then decide whether to strip the
annotation from the served name and drop the winner role of a `niet gegunde` party, keeping its Bid. The partyname
specimen `Humble Inpect B.V. (niet gegunde partij)` must stay a kept name.

## Verify

    curl -s https://tenders.zebreus.click/v1/tenders/229981 | jq -c '[.lot_results[] | [.notice_id, .key, [.winners[].organization_id]]]'

This reads unit 3, the last open unit: does the later award notice's result supersede the earlier one's? Unit 2's
part is already done (v3's RES-0001 serves Planon alone). The filed check, `name_prefix=Humble Inpect` showing the
annotation gone, is not this issue's to close: the decision keeps the org heads 30533823 and 30533824 on their
first-seen marked spelling until 524.
- **done**: RES-0001 names Planon (16387876) alone in every row for the lot. Either v2's (24424017) row is
  superseded, or its winners narrow to 16387876. If unit 3 rules against superseding, the ruling is the closure, and
  the open output below becomes the accepted state.
- **open** (2026-10-13, from unit 2's read; not re-run on 2026-10-24 because this pass made no HTTP request): two
  RES-0001 rows. `[24400562,"RES-0001",[16387876]]` is v3, Planon alone. v2's 24424017 row lists all five tenderers
  as winners: Planon 16387876, Facilicom 6734977 (unit 1's census), PVM Eindhoven 7201067, Humble Inpect 30533823 and
  Provadi 30533824. No later commit or issue records a unit-3 measurement or ruling (checked 2026-10-24: git log
  shows 527 commits only through 55c1fa3; 534 still lists 527 as ready-for-agent).

## Triage decision (2026-10-13, owner)

**Measured.** `name-phrase-census` job 2385 asked for `niet gegunde partij`, `gegunde partij`, `niet gegund` and `gegund`, and also walked mentions.
- `niet gegunde partij` reaches 4 organizations: 2 by the served name (30533823 and 30533824, whose heads are the annotated spellings) and 2 only through a mention. Facilicom's mention binds to 6734977, whose head reads `Breijer Bouw en Installatie BV`; that is head drift, so it goes to 524.
- `gegunde partij` alone reaches 1 organization, Planon.
- The bare `gegund` reaches 166 organizations. These are Dutch award prose in winner slots, not this defect, and are filed as issue 528.
- The annotation is TenderNed's correction practice: v3 re-published the award with each tenderer marked.

**Decision: build (S).** The publisher's own marking is the evidence.
1. In an eForms result, a party whose published name carries `(niet gegunde partij)` is not a winner. Its Tenderer and bid roles stay.
2. The mention's name drops both annotations, `(niet gegunde partij)` and `(gegunde partij)`, so it serves the company's name. The org heads 30533823 and 30533824 stay their first-seen spelling until 524.
3. Refold notice 24400562 (`refold-notices`).

**Verify after unit 2.**
- Tender 229981's v3 RES-0001 serves Planon (16387876) alone.
- The four losers keep `Tenderer` and `bid:tenderer`.
- The annotated mention names read clean.
- Re-run `name-phrase-census` with the four annotation phrases and `mentions`: `niet gegunde partij` should reach no winner.
- If v2's RES-0001, which is unannotated and also served, still lists the losers as winners, unit 3 decides whether a later award notice's result for the same lot supersedes an earlier one's.
