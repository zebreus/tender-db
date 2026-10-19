# Audit: two recent award (result) tenders vs their source notices

- Date: 2026-10-19 (GETs made 21:58Z to 22:02Z). Agent tag in `api_requests.log`: `agent=award`.
- Scope: read-only public API GETs only. 14 GETs (#1 to #14), all 200, no 408s, at least 2.1 s apart.
  Raw bodies are in `raw/award/`.
- Reference texts: `raw/docs.html` and `raw/openapi.json`, fetched today at 21:50Z by the shared log's
  first agent. The copies under `raw/ted/` and `raw/doe-fts/` have the same md5, so I re-read them
  instead of spending GETs. I also read repo `CHANGELOG.md` and `CONTEXT.md`. I read code only to
  confirm intent, never as a documented explanation.

## Verdict

| Tender | Notice (source) | Fields compared | Discrepancies | Explained differences |
|---|---|---|---|---|
| 8851944 (ES, Compusof) | 47411306 = TED 00722535-2026, eForms SDK 1.13, subtype 29 | all served fields | **1** (review body missing from `parties`) | 4 |
| 8851923 (AT, NetApp) | 47410357 = TED 00719637-2026, eForms SDK 1.14, subtype 29 | all served fields | 0 | 3 |

Each tender has a single version (`versions[]` has one entry), so exactly one notice produced each
one. Every served value was checked against that notice's `/v1/notices/{id}/content`.

## How the two tenders were picked (deterministic)

1. `GET /v1/tenders?published_before=now&limit=200`. A single bound implies `sort=published_at`,
   newest first. All 200 rows were TED, published 2026-10-19.
2. A row qualifies if it is an award (result): `notice_subtype` is an eForms result subtype (29 to 37),
   or an FTS award form or tag. It must also serve a non-null `value`. I walked the rows in served order.
   The qualifying rows were: 8852025 (29), 8851947 (29), **8851944 (29) = 3rd**, 8851940 (29),
   8851936 (29), 8851930 (29), **8851923 (29) = 7th**. 8851922 (29) has `value: null` and did not qualify.
   VEAT subtypes 25 and 26 are not results and were skipped.
3. Both details have `lot_results[].winners` with an awarded value, so neither pick needed replacing.

## Tender 8851944: "2026/Sp02002001/00001475E - Adquisición De Estaciones De Trabajo Para Omp"

Notice 47411306 (`/v1/notices/47411306`): source `ted`, `parse_state: parsed`, `quarantine: null`,
profile `eforms:eforms-sdk-1.13`, member `20261019_202/00722535_2026.xml`.

| Served field | Served value | Source (notice 47411306) | Result |
|---|---|---|---|
| title (row, `texts` title, lot title) | "2026/Sp02002001/00001475E - Adquisición De Estaciones De Trabajo Para Omp\n (Ministerio De Defensa-Estado Mayor De La Defensa)" | BT-21-Procedure, BT-21-Lot (SPA): same string, byte for byte | match |
| texts description (tender + lot) | same string | BT-24-Procedure, BT-24-Lot | match |
| source / publication_id / notice_subtype | ted / 00722535-2026 / 29 | OPP-010 00722535-2026, OPP-070 29 | match |
| procedure_key | ff8efdce-2ee1-4fe1-9c1e-6eb3636ea1f7 | BT-04 | match |
| procedure_type, classification procedure | open | BT-105 open | match |
| kind | procedure | subtype 29 (not a registration notice) | match |
| original_lang | SPA | BT-702(a) SPA | match |
| published_at | 2026-10-19 (date only) | OPP-012 2026-10-19 (date only) | match (the date-only convention in /docs) |
| dispatched_at | 2026-10-16T08:38:09+02:00 | BT-05(a) 2026-10-16T08:38:09+02:00 | match |
| submission_deadline / scope / `dates[]` | null / null / [] | no deadline field (award notice); no contract notice in the chain | match |
| status (via `?status=closed`, GET #13) | closed | no deadline; result notice | match (the /docs status rule) |
| cpv (row + classifications main, tender + lot) | 30214000 | BT-262-Procedure, BT-262-Lot 30214000 | match |
| classifications nature (tender + lot) | supplies | BT-23 supplies | match |
| country / nuts place (tender + lot) | ES300 | BT-5071-Procedure, BT-5071-Lot ES300 (BT-5141 ESP) | match |
| dps classification | none served | BT-766-Lot `none` | match (only dps-list / dps-nlist produce a row) |
| parties Procedure-Buyer | org 6614, name and mention "Dirección General de Racionalización y Centralización de la Contratación-Junta de Contratación Centralizada" | ORG-0001 BT-500, same name; OPT-300-Procedure-Buyer → ORG-0001 | match |
| org 6614 (GET #8) | ES, identifier S2801213F (national), not provisional | ORG-0001 BT-501 S2801213F (NIF), BT-514 ESP | match |
| parties Tenderer (LOT-0000) | org 12032625, mention "Compusof, S.A.", head "Compusof, S. A." | ORG-0002 BT-500 "Compusof, S.A."; TPA-0001 → ORG-0002 | mention matches; head differs (**explained E2**) |
| org 12032625 (GET #9) | ES, A28793917 (national) | ORG-0002 BT-501 A28793917 (NIF), ESP | match |
| parties: review body | **none served** | LOT-0000 OPT-301-Lot-ReviewOrg → TPO-0001, a touchpoint of ORG-0001: "Tribunal Administrativo Central de Recursos Contractuales (TACRC) o Tribunal Superior de Justicia (TSJ).", Madrid ES300 | **DISCREPANCY D1** |
| lots / lot_details | 1; LOT-0000, kind Lot, title as above, value EUR 475,266.00, no deadline | BT-137-Lot LOT-0000, BT-21-Lot, BT-27-Lot 47526600 EUR | match |
| amounts | result_value (lot null) EUR 474,977.00; estimated_value (LOT-0000) EUR 475,266.00 | BT-161-NoticeResult 47497700 EUR; BT-27-Lot 47526600 EUR | match |
| value (headline) | EUR 475,266.00, value_derivation null | the largest published figure (BT-27-Lot); the award is 474,977.00 | differs from the award (**explained E1**) |
| lot_results RES-0001 | LOT-0000, decision selec-w, awarded EUR 474,977.00, decided null, reason null | BT-13713 LOT-0000, BT-142 selec-w, award via OPT-320 → TEN-0001 BT-720 47497700 EUR; no BT-1451 or BT-144 | match |
| lot_results statistics | t-esubm 2, t-sme 0, tenders 2; withheld 0 | BT-759/760: t-esubm 2, t-sme 0, tenders 2 | match |
| winners | org 12032625 "Compusof, S. A." (no is_buyer) | TEN-0001 → TPA-0001 → ORG-0002 (not the buyer) | match (head name: E2) |
| bids TEN-0001 | LOT-0000, EUR 474,977.00, tenderer 12032625 (mention "Compusof, S.A."), quality null | BT-13714 LOT-0000, BT-720 47497700 EUR | match |
| contracts CON-0001 | buyer_contract_id A4126/2026, concluded 2026-10-15, decided null, value EUR 474,977.00 | BT-150 A4126/2026, BT-145 2026-10-15, no BT-1451; the contract has no amount of its own. BT-3202 → TEN-0001, whose BT-720 is 47497700 | match (the value is derived: **E4**) |
| versions[0] | seq 1, caused_by 47411306, same dates, subtype and lang as above | notice 47411306 | match |
| documents / links | no such field on the detail | no BT-15 documents URL. BT-508 buyer-profile URL is in notice content only | not served (**E3**) |

Observations that are not served fields, so I did not judge them:
- OPP-090 names a previous notice, 5261bdfd-…-01, which is not in this Tender's chain. BT-765 `fa-mix`
  and the winner's address `compusof_am02_2023@` point to a call-off under a centralised framework
  (AM 02/2023) with its own BT-04. That is outside the linkage CONTEXT.md promises (shared procedure
  key), and the public API cannot resolve a BT-701 UUID, so I did not verify it.
- The parse layer files procedure-level `BT-23-Lot`, `BT-36-Lot` (39 DAY) and `BT-60-Lot` under the
  `PROCEDURE` section. That is a field-id label quirk of the notice layer. Only `nature` reaches the
  Tender, and it is correct.

## Tender 8851923: "NetAPP"

Notice 47410357 (`/v1/notices/47410357`): source `ted`, `parse_state: parsed`, `quarantine: null`,
profile `eforms:eforms-sdk-1.14`, member `20261019_202/00719637_2026.xml`.

| Served field | Served value | Source (notice 47410357) | Result |
|---|---|---|---|
| title (row, `texts`, lot) | "NetAPP" | BT-21-Procedure, BT-21-Lot (DEU) "NetAPP" | match |
| texts description (tender + lot) | "Abruf aus der BBG-Rahmenvereinbarung GZ 3401.04940.001.01 … sowie Installationsdienstleistung." | BT-24-Procedure, BT-24-Lot: identical | match |
| source / publication_id / notice_subtype | ted / 00719637-2026 / 29 | OPP-010, OPP-070 | match |
| procedure_key | fcdc1b3a-a9cc-4f41-8d5c-1d0b99b7a542 | BT-04 (equal to BT-701 here) | match |
| procedure_type, classification procedure | oth-single | BT-105 oth-single | match |
| original_lang | DEU | BT-702(a) DEU | match |
| published_at | 2026-10-19 | OPP-012 2026-10-19 | match |
| dispatched_at | 2026-10-16T08:46:49+02:00 | BT-05(a) same | match |
| submission_deadline / scope / `dates[]` | null / null / [] | no deadline published | match |
| status (via `?status=closed`, GET #14) | closed | no deadline; result notice | match |
| cpv (row + main, tender + lot) | 48820000 | BT-262-Procedure, BT-262-Lot | match |
| nature (tender + lot) | supplies | BT-23 supplies | match |
| country / nuts place (tender + lot) | AT123 | BT-5071-Procedure, BT-5071-Lot AT123 (BT-5141 AUT) | match |
| parties Procedure-Buyer | org 31969111 "NÖ LGA UK St. Pölten" (head and mention) | ORG-0001 BT-500 "NÖ LGA UK St. Pölten" | match |
| org 31969111 (GET #10) | AT, ATU75348768 (vat) | ORG-0001 BT-501 ATU75348768, AUT | match |
| parties Lot-ReviewOrg + Lot-ReviewInfo (LOT-0001) | org 47 "Bundesverwaltungsgericht" | OPT-301-Lot-ReviewOrg and OPT-301-Lot-ReviewInfo → ORG-0003 "Bundesverwaltungsgericht" | match |
| org 47 (GET #11) | AT, 9110008059823 (national) | ORG-0003 BT-501 9110008059823, AUT | match |
| parties Tenderer (LOT-0001) | org 31762953 "NetApp Austria" (head and mention) | ORG-0002 BT-500 "NetApp Austria" | match |
| org 31762953 (GET #12) | AT, ATU48548807 (vat) | ORG-0002 BT-501 ATU48548807, AUT | match |
| lots / lot_details | 1; LOT-0001, kind Lot, title "NetAPP", **value null**, no deadline | BT-137-Lot LOT-0001; no BT-27-Lot or other lot figure | match (null lot value: **E5**) |
| amounts | result_value (lot null) EUR 59,342.94 | BT-161-NoticeResult 5934294 EUR | match |
| value (headline) | EUR 59,342.94, value_derivation null | BT-161 / BT-720 5934294 EUR (the only figure) | match |
| lot_results RES-0001 | LOT-0001, selec-w, awarded EUR 59,342.94, decided null, reason null | BT-13713 LOT-0001, BT-142 selec-w, TEN-0001 BT-720 5934294 | match |
| statistics | tenders 1, t-sme 0, t-esubm 0, t-oth-eea 0, t-no-eea 0; withheld 0 | BT-759/760: same five | match |
| winners | org 31762953 "NetApp Austria" (no is_buyer) | TEN-0001 → TPA-0001 → ORG-0002 | match |
| bids TEN-0001 | LOT-0001, EUR 59,342.94, tenderer 31762953 (mention "NetApp Austria") | BT-13714 LOT-0001, BT-720 5934294 | match |
| contracts CON-0001 | buyer_contract_id 118241BN-000000429, concluded 2026-10-16, decided 2026-10-16, value EUR 59,342.94 | BT-150 same, BT-145 2026-10-16, BT-1451-Contract 2026-10-16; BT-3202 → TEN-0001 (BT-720 5934294) | match (the value is derived: E4) |
| versions[0] | seq 1, caused_by 47410357 | notice 47410357 | match |
| documents / links | no such field on the detail | no BT-15. BT-505 internet addresses are in notice content only | not served (E3) |

Not served, so not judged: BT-721-Contract title "NetAPP"; OPT-100-Contract `521254-2024`, the BBG
framework notice this call-off draws on (not linked, since it has a different BT-04); OPT-999
`2000-01-01`, which looks like the SDK's dummy award date. Not serving that dummy date is correct.

## Discrepancies (served value contradicts the source, and no documented rule explains it)

**D1: tender 8851944 `parties[]` has no review body. The notice names one.**
- Served: `parties[]` holds only `Procedure-Buyer` (6614) and `Tenderer` (12032625). It has no
  `Lot-ReviewOrg`.
- Source: notice 47411306, LOT-0000 `OPT-301-Lot-ReviewOrg` = `TPO-0001`. That is a touchpoint under
  ORG-0001 named "Tribunal Administrativo Central de Recursos Contractuales (TACRC) o Tribunal Superior
  de Justicia (TSJ).", 28000 Madrid, ES300.
- Contrast: on tender 8851923 the same role pointing at an `ORG-` section is served (org 47
  Bundesverwaltungsgericht, `Lot-ReviewOrg` and `Lot-ReviewInfo`). So the role is modelled, and only
  the touchpoint form of the reference is lost.
- Why it is not explained:
  - The OpenAPI `parties` description says it lists "the organizations this version names, and the
    role each plays", with `Lot-ReviewOrg` in its vocabulary.
  - Neither /docs, nor the OpenAPI, nor CHANGELOG.md mentions touchpoints (grep returns 0 hits).
  - /docs says notice content is where "a field the canonical layer does not model" lives. That does
    not cover this case, because `Lot-ReviewOrg` is modelled and is served when it points at an
    `ORG-` section.
- The drop is deliberate in code. `crates/ingest/src/project.rs:6111-6113` (`bind_organizations`)
  reads: "A reference to something that is not an Organization section (a touchpoint, a lot, a
  result) is notice-layer detail, not a party." So this is an undocumented rule, not a bug in the
  parse.
- Severity: low. It is an omission, not a wrong value: no wrong organization is served. It is probably
  systematic, though. Spanish PLACSP eForms notices usually publish the review body as a buyer
  touchpoint, so a consumer asking "who is the review body" gets nothing for those tenders. Fix by
  documenting the rule in the `parties` description, or by minting the touchpoint as a party.

## Explained differences

- **E1: 8851944 `value` is EUR 475,266.00, the lot's estimate (BT-27-Lot), not the awarded
  EUR 474,977.00** (BT-161-NoticeResult / BT-720).
  - CHANGELOG.md (issue 551 intro): "`value` was always ONE published figure: the largest the value
    election accepts."
  - /docs Amounts: "A Tender's value is the ONE figure elected from them."
  - Both figures are published. The election takes the larger one (code: `elect_head_figure` in
    `crates/store/src/canonical.rs` is a max over tender-scope and lot amounts).
  - The award is still served exactly in `amounts`, `lot_results[].awarded`, `bids[].value` and
    `contracts[].value`.
  - Consumer caveat: on an award tender, `value` (and `min_value` / `max_value`) can be the estimate
    rather than the awarded price whenever the estimate is larger.
- **E2: 8851944 winner `organization_name` is "Compusof, S. A."; the notice prints "Compusof, S.A."**
  The head name is elected from the organization's mentions (issue 524; /docs Tender detail; OpenAPI
  `Organization.name` and `parties[].organization_name`). `mention_name` serves the notice's exact
  spelling on both `parties[]` and `bids[].parties[]`. The identifier A28793917 matches.
- **E3: neither detail serves documents or links.** The notices publish no BT-15 documents URL.
  8851944's BT-508 buyer-profile URL and 8851923's BT-505 web addresses are only in
  `/v1/notices/{id}/content`. CONTEXT.md says "Metadata only: the PDF/document attachments of tenders
  are out of scope". /docs says notice content is where "a field the canonical layer does not model"
  lives.
- **E4 (both tenders): `contracts[].value` is served although the notice's SettledContract (CON-0001)
  states no amount.** The value is the BT-720 of the bid the contract settles (BT-3202 → TEN-0001).
  It equals the notice's own figure on both tenders (474,977.00 and 59,342.94), so it does not
  contradict the source. However, the OpenAPI leaves `contracts[]` item properties undocumented, and
  no doc states the derivation. The code comment is at `crates/ingest/src/fts/parse.rs:365`.
- **E5: 8851923 `lot_details[0].value` is null although the lot was awarded EUR 59,342.94.** /docs
  (SQL → Values, "Two things the lot value is not"): the lot value "is chosen from the lot's own
  amounts, never from award results … its award is in v_lot_results.awarded_cents". The notice
  publishes no lot estimate.

## Files

- `raw/award/list_recent_p1.json`: the selection page
- `raw/award/tender_8851944.json`, `raw/award/tender_8851923.json`: the two details
- `raw/award/content_47411306.json`, `raw/award/content_47410357.json`, plus `notice_*.json`: the
  source notices and their metadata
- `raw/award/org_{6614,12032625,31969111,47,31762953}.json`: the organizations
- `raw/award/status_closed_*.json`: the status checks
