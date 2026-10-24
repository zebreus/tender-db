# 594 — the Junta de Andalucía's shared NIF S4111001F binds Consejería buyers to org 2660, the TARC Andalucía review tribunal

Status: ready-for-agent — **unit 2 DECIDED 2026-10-24 ~23:25Z (wf_c8e589b4-7b1: reader over ALL 9,103 mentions / 40,265 party rows, author, skeptic A1–A4 applied; `594-u1/decision.md`)**: identity unchanged (S4111001F is the Junta's NIF, one legal person; Consejerías and TARC are its organs; 572-L holds, no mention moves). 2660 got TARC's name at mint (notice 995's spelling), not by vote, and every vote rule keeps TARC, so the mechanism is a NEW reviewed head pin: compiled `REVIEWED_HEADS` (store, keyed (country, kind, identifier)) read by `elect_head` as rule P0 (epoch 524u2-v15; table digest pinned; P0 added to ORG_NAME_REFRESH_EXCLUDABLE_RULES), applied by refresh-org-names dry → wet (run back to back), no refold (parties read names live). Pinned to "Junta de Andalucía": 2660, 22341455, 22419088, 11044446 (+ 22235590 → TARC's full name, droppable); 22525033 NOT pinned (A3: all TARC). Expected: 3,334 buyer rows on 3,333 Tenders W→N, ~9,600 current rows W→R/N; ~3,490 TARC review rows R→N (mention_name keeps the tribunal). Admission rule (d) (A4) keeps 584's UZP/KIO case out. Tails (46 mentions: Consejo Audiovisual, Consejo de Transparencia — own legal personality, A1 — companies, TSJA) → U4, a 584-style rehoming cohort. Follow-ups filed: 595 (TARC split over 10+ rows), 596 (one Junta row); F3 (buyer printed as Tenderer/winner, 28 rows) is 584 §5's shape. Banner up. NEXT: U2 build (S) with the next code batch (after deploy #5's tree; deploy #6 with 587 U2), U4 tails cohort in parallel. Was: ready-for-agent — **unit 1 SIZED 2026-10-24 ~23:00Z (3 bounded reads, `594-u1/reads.json`)**: org 2660 is "Tribunal Administrativo de Recursos Contractuales de Andalucía", ES national S4111001F — the Junta de Andalucía's own NIF, which every Consejería prints. It serves **buyer on 2,323 Tenders (4,591 rows) + Procedure-Buyer on 1,067 (2,244 rows)**, plus Contract-Signatory 238, LotResult-Paying 96 / Financing 73, DocProvider 238, TenderReceipt 170 — beside its genuine review roles (Lot-ReviewOrg 1,175 Tenders, ReviewInfo 695, Mediator 131). Mention names: TARC spellings (~1,600 mentions in the top 40) and many Consejería names (Fomento 191, Salud y Familias 160, Agricultura/DG Infraestructuras del Agua 136+127+109+77, Empleo 104, Hacienda/DG Contratación 82 …). The head election picked a TARC spelling because review-role mentions dominate. Direction to decide (unit 2): the NIF identifies the Junta (one legal entity), so the row should SERVE the Junta (rename/route the head to "Junta de Andalucía", with Consejerías its departments under D576-1's accepted N) rather than split mentions; and TARC's own review mentions either stay (a body of the Junta) or move to a TARC row of their own. Compare 584's KIO/UZP (UZP's NIF, KIO inside it) and 572's buying-agent rule. Was: needs-triage — filed 2026-10-24 from 587 unit 1's decision (F2, `587-u1/decision.md`; read s08). Next: unit 1, size
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
