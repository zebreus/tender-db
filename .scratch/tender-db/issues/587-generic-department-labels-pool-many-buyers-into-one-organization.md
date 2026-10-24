# 587 — generic department labels pool many buyers into one organization

Status: ready-for-agent — **unit 1 QUEUED 2026-10-24 ~19:00Z: census 2781** (`name-phrase-census`, served names, 20 folded contracting-unit phrases FR/ES/PL/NL/DE/EN/IT; job file `.scratch/tender-db/587-u1/census.json`). It lists every organization whose served name holds a phrase, with its roles per Tender and mention count. NEXT: from 2781, split BARE labels (the folded name IS the phrase, or the phrase plus only punctuation/legal noise) from a buyer's department under its own name ("Ville de X — Direction de la Commande Publique", D576-1's accepted N); for the bare ones with many mentions, read distinct buyers per org (bounded `tender_version_parties_org` seeks for the top 20) and decide: 586 D2's whole-name list for the touchpoint gate + an ORG-path drop/alias-to-own-buyer. Was: needs-triage — filed 2026-10-24 from 576 U5 / 586 U3's read-back (wf_0fa63db1-03e, minted items B07, B14, B29).
Next: unit 1, size the label rows (organizations with no identifier whose name is a bare department/contracting-unit label
and whose mentions span several unrelated buyers) and decide: a whole-name stem list for the touchpoint gate (586 D2's
mechanism) and/or the ORG path (585's question), or an alias to the parent when a department label names no body.
Kind: data quality / organizations (W where one row conflates unrelated buyers; N where one buyer's department)
Relates to: 586 (D2 gate lists: labels, bare organ words), 585 (ORG-path labels), 576 (D576-1 (ii) accepted department
cost), 456.

## Verify

    for o in 1166638 1223007 9327844; do curl -s https://tenders.zebreus.click/v1/organizations/$o | jq -c '[.id, .name, .mention_count]'; sleep 2; done

- **open (2026-10-24)**: 1166638 "Direction de la Commande Publique" (1,356 mentions), 1223007 "Unidad de Contratación"
  (410, served as a `Lot-ReviewOrg` on B14), 9327844 "Dział Zamówień Publicznych i Zaopatrzenia" (37).
- **done**: no served party binds a bare department label shared by unrelated buyers (the rows are aliased to each
  notice's own buyer, or dropped, per unit 1's decision).

## What was found

- 586's gate drops the audit's nine labels and bare organ/registry words, but generic contracting-unit names
  ("Direction de la Commande Publique", "Unidad de Contratación", "Dział Zamówień Publicznych…", and likely "Afd. Inkoop",
  576 U4 item 27) still mint, or bind an old name-keyed row that many buyers share.
- 586's decision kept "a buyer department under its own name" as N (D576-1's accepted cost). A label that names no body
  and pools buyers is the W hub shape of 586 D2, not that cost.
