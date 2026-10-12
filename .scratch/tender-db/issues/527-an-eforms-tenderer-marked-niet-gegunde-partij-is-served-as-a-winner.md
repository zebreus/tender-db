# 527 — an eForms tenderer marked "(niet gegunde partij)" is served as a winner

Status: needs-triage — filed 2026-10-12 from 520 unit 1 (§4.5).
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
`curl -s "https://tenders.zebreus.click/v1/organizations?name_prefix=Humble%20Inpect&limit=10" | jq '[.items[].name]'`
Open: includes "(niet gegunde partij)". Done: the annotation is gone and the org holds no winner role on 24400562.
