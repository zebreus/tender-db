# 594 — the Junta de Andalucía's shared NIF S4111001F binds Consejería buyers to org 2660, the TARC Andalucía review tribunal

Status: needs-triage — filed 2026-10-24 from 587 unit 1's decision (F2, `587-u1/decision.md`; read s08). Next: unit 1, size
it (Procedure-Buyer / buyer rows on org 2660 whose mention name is not TARC: bounded `tender_version_parties_org` seek +
`organization_mentions_org` names) and decide the remedy, as 584 did for KIO's NIP: a reviewed rehoming cohort (545/584
precedent) and/or the name-agreement guard 584 unit 3 is designing for shared national identifiers.
Kind: data quality / wrong organization served (W: a consumer reading the buyer gets the review tribunal)
Relates to: 584 (KIO/UZP NIP: the same shape, a body's identifier shared by others; its guard spec), 572 (an identifier
standing for another body), 587 (D587-2 inherits this binding: a label aliased to the notice's buyer section serves TARC
where that section binds 2660).

## What was found

- S4111001F is the Junta de Andalucía's NIF, printed by many Consejerías. Mentions printing it resolve to org 2660, named
  "TARC Andalucía" (Tribunal Administrativo de Recursos Contractuales), so Consejería buyers are served as the tribunal:
  118 buyer rows beside the label org 1193769 alone (587 s08); 117 of 135 Tenders next to 1193769 serve TARC as buyer.

## Verify

    curl -s https://tenders.zebreus.click/v1/organizations/2660 | jq -c '[.id, .name, .identifier]'

- **open (2026-10-24)**: org 2660 "TARC Andalucía" holds S4111001F and is served as the buyer of Consejería Tenders.
- **done**: Consejería mentions printing S4111001F bind their own bodies (or the Junta), and 2660 serves only TARC's own
  review roles.
