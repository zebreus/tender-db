# 484 — the buyer is served as its own contractor (the winner slot repeats the authority)

Status: ready-for-agent — NEXT: census (dry): for every award whose winner mention is the buyer's organization or
folded name, classify by Source × profile × procedure type, and read 30 samples per cell against the raw notice —
split the parser defects (fixable at parse) from genuine in-house awards (correct as served).
Kind: data correctness (awards, parties)
Relates to: 483 (found by its census: the `contractor-*` classes), 456 (mention binding), the served `awards[]`

## What is wrong

The 483 census (job 1942, stride 10, 2026-10-03; `.scratch/tender-db/483-roles/census-1942.json`) flagged 263
notices where the buyer is ALSO the contractor (`contractor-org-same-name` 135, `contractor-name` 125,
`contractor-same-section` 3) — ~2,600 across the corpus. In every one of the 63 samples the buyer is the right one
and the winner is the doubtful mention:
- **2002406** (legacy TED text, 2002): `TXT-CO` reads "Supplier(s): Contrato n° S-036/02-DJ.\n1: Gobierno Vasco, A
  la atención de Mesa de…" — the legacy text parser takes the first name after "Supplier(s):", which here is the
  authority's contact line. Spanish legacy notices dominate `contractor-name` (Ministerio de Justicia, Consejería de
  Educación y Ciencia, Autoridad Portuaria de Málaga, Universitat de Barcelona). Likely a PARSE defect.
- **23011756** Stadt Hilden (F20 modification): the contractor block names the city (the contract moved to the
  city's own company, per the modification text). Possibly the source's own entry.
- **SPMS ×3, Comunidade Intermunicipal ×3, Comune, Município** (legacy): central purchasing bodies and in-house
  awards, where buyer = contractor may be correct as published.
- **UK award updates** (Kirklees, Clackmannanshire, Cabinet Office): the same Organization referenced as buyer and
  supplier.

Served effect: `awards[].supplier` names the buyer; supplier statistics count authorities as winners.

## Census (unit 1, dry)

Reuse the `buyer-role-census` walk (its `contractor-*` classes already find these): add per-cell samples of the RAW
winner field (legacy `TXT-CO` text, `ADDRESS_CONTRACTOR`, eForms `Tenderer` / `LotTender`) so the read can tell
"the parser picked the wrong line" from "the notice says so". Fix by source: parser defects at parse (re-project),
genuine in-house awards left as served (an in-house award to oneself is correct data — mark it, don't drop it).

## Verify

    curl -s https://tenders.zebreus.click/v1/tenders/2959772 | jq -c '[.parties[] | [.role, .organization_name]]'

(Tender 2959772 is notice 2002406, TED 190812-2002.)
- **open** (2026-10-03): `[["buyer","Gobierno Vasco"],["winner","GOBIERNO VASCO"]]` — the authority as its own winner.
- **done:** the winner is the supplier the text names after the authority's contact, or none.
