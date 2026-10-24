# 594 — the Junta de Andalucía's shared NIF S4111001F binds Consejería buyers to org 2660, the TARC Andalucía review tribunal

Status: ready-for-agent — **unit 1 SIZED 2026-10-24 ~23:00Z (3 bounded reads, `594-u1/reads.json`)**: org 2660 is "Tribunal Administrativo de Recursos Contractuales de Andalucía", ES national S4111001F — the Junta de Andalucía's own NIF, which every Consejería prints. It serves **buyer on 2,323 Tenders (4,591 rows) + Procedure-Buyer on 1,067 (2,244 rows)**, plus Contract-Signatory 238, LotResult-Paying 96 / Financing 73, DocProvider 238, TenderReceipt 170 — beside its genuine review roles (Lot-ReviewOrg 1,175 Tenders, ReviewInfo 695, Mediator 131). Mention names: TARC spellings (~1,600 mentions in the top 40) and many Consejería names (Fomento 191, Salud y Familias 160, Agricultura/DG Infraestructuras del Agua 136+127+109+77, Empleo 104, Hacienda/DG Contratación 82 …). The head election picked a TARC spelling because review-role mentions dominate. Direction to decide (unit 2): the NIF identifies the Junta (one legal entity), so the row should SERVE the Junta (rename/route the head to "Junta de Andalucía", with Consejerías its departments under D576-1's accepted N) rather than split mentions; and TARC's own review mentions either stay (a body of the Junta) or move to a TARC row of their own. Compare 584's KIO/UZP (UZP's NIF, KIO inside it) and 572's buying-agent rule. Was: needs-triage — filed 2026-10-24 from 587 unit 1's decision (F2, `587-u1/decision.md`; read s08). Next: unit 1, size
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
