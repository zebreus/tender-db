# 593 — a bare department label printed as the buyer of record, or named by a result, stays one pooled row

Status: needs-triage — filed 2026-10-24 from 587 unit 1's decision (F1, `587-u1/decision.md` "Stays for later"). 587's
alias joins a bare label to the notice's own buyer; where the label IS the buyer of record, or a result (winner, tenderer)
points at it, there is no buyer to join and 587 leaves it as today. Next: unit 1, size it from 587 unit 3's dry census
(`kept_buyer` count) and decide: key such a mention by (label, town, country), or promote a named body the notice prints
elsewhere (483's promotion territory, its own decision).
Kind: data quality / organizations (W where one label row is the buyer of many towns' Tenders)
Relates to: 587 (D587-2 keeps this shape), 483 (buyer promotion), 576 / 586 (labels on the touchpoint path), 456.

## What was found (587 unit 1, s07–s11)

- DÖE 19553 "Zentrale Vergabestelle" (no country) is the buyer of record on 291 Tenders with no other party; their titles
  name Wuppertal, Leverkusen, Schwerte, Aulendorf and others.
- TED labels as buyer: 2997642 (27+1), 1804271 (8+7), 1340282 (10), 1166638 (6), 1416875 (6), 3443 (3), 2680514 (2),
  1363144 (1); DÖE 55205 "Vergabestelle" (NULL country) on 14.
- Label as a result: 1164742 "Servicio de Contratación" holds 4 `Tenderer` rows.

## Verify

    curl -s https://tenders.zebreus.click/v1/organizations/19553 | jq -c '[.id, .name, .mentions]'

- **open (2026-10-24)**: one row named "Zentrale Vergabestelle" serves as buyer of 291 Tenders of many towns.
- **done**: no bare label row is the buyer of record for Tenders of more than one town (keyed per town, or promoted).
