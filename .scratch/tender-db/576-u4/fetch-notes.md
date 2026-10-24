# 576 unit 4: sample build and fetch notes (2026-10-24, 12:46–13:02Z)

Output: `sample.json` (30 items, one object each), built from `picks.json` (the pick, with every candidate's rank) and
the raw bodies under `raw/` (tender/, org/, content/, extra/, sql/, probe/; `raw/gets.log` has every GET).
Scripts that reproduce the windows, the pick and the build are in `scripts/` (win.py, pick.py, build.py, sq.py, get.sh; copies of the ones run from the session scratchpad, whose paths they still name).
Read-only: no POST, no admin endpoint, no git, no cargo. Writes only under `.scratch/tender-db/576-u4/`.

## Budget used

- Public API: 139 GETs, strictly serial through one flock'd helper; the smallest gap between two request starts was 2.62 s.
  137 answered 200; 2 range probes answered 404 (tender 1 and tender 8,870,000 do not exist).
- /v1/sql: 8 of 12 statements, all answered with JSON rows+columns (truncated=false), 0.38–1.56 s each. No 408.

## Verify reads (unit 4's first step)

- 8851944 v1: `Lot-ReviewOrg` LOT-0000 → org 22077939, "Tribunal Administrativo Central de Recursos Contractuales
  (TACRC) o Tribunal Superior de Justicia (TSJ)." (the compound name). Also Procedure-Buyer 6614 and Tenderer 12032625.
- 8851996 v1: `Lot-ReviewOrg` ×4 (LOT-0001..0004) → org 2261, "Tribunal Administrativo de Contratación Pública de la
  Comunidad de Madrid". `Lot-AddInfo` ×4 → org 22213489, the buyer's own organization (the same org as Procedure-Buyer).
- Bodies: `raw/tender/8851944.json`, `raw/tender/8851996.json`.

## Where the cohort lives (the task's ">~7.5M" premise does not hold)

The tender-id ranges were located by single tender GETs (`raw/probe/`).
- **Segment A, ids ~1–1.21M: eForms TED.** 50k, 250k, 500k, 750k, 1.0M, 1.1M, 1.15M and 1.2M all have numeric or T01
  subtypes, roles `Lot-ReviewOrg`/`Lot-AddInfo`/…, and were published 2024-05 to 2026-08.
- **1.225M–1.75M: DÖE.** Legacy-shaped, role `buyer`.
- **2.0M–8.52M: legacy TED.** Subtype null, roles `buyer`/`review-body`. 7.0M, 7.25M, 7.5M, 7.75M, 8.0M, 8.25M, 8.5M and 8.52M
  are all legacy, published 2010–2023.
- **Segment B, ids ~8.54M–8.87M: newly ingested eForms TED interleaved with `fts` (UK).** 8.54M, 8.56M, 8.75M, 8.83M and 8.86M
  are TED eForms. 8.58M, 8.6M, 8.65M, 8.7M, 8.8M and 8.85M are fts. The two Verify tenders sit here.

The windows were therefore mapped onto the concatenation of A = [1, 1,210,000] and B = [8,530,000, 8,869,999], with
L = 1,550,000. The window start is `s = int(sha256("576-u4-" + str(i))[:16], 16) mod L`. Then a = 1 + s if s < 1,210,000,
else 8,530,000 + (s − 1,210,000).

## Sampling as run

- **General windows i = 0..5, W = 3,000.** One statement per window: tenders `t` (the id range) CROSS JOIN
  tender_version_parties `p` on (tender_id, seq = t.current_seq), so only current versions count. Then CROSS JOIN
  organizations (PK) for name and country, and LEFT JOIN notice_codes (PK) for the touchpoint's own
  `BT-514-Organization-TouchPoint`. Filter: `p.mention_section_id LIKE 'TPO-%'`.
  - Rows are grouped by (tender, seq, role, org, notice, section). This collapses the lots of one multi-lot role:
    `lot_id_min` is the first lot and `n_rows` the number of party rows. Without the grouping, one 29-lot tender would fill
    LIMIT 40 by itself.
  - The GROUP BY uses a sorter, so the whole window is walked (0.4–0.7 s). LIMIT 40 keeps the first 40 groups in id order,
    which is about the first 340 ids of each window.
  - Window i=4 (8,626,880..8,629,880, segment B) returned 0 rows. Every other general window returned 40.
- **NL AddInfo/TenderReceipt window i = 6, W = 20,000.** It ran because the general windows held only 1 NL
  AddInfo/TenderReceipt group. Same statement plus `+p.role IN ('Lot-AddInfo','Lot-TenderReceipt') AND +o.country = 'NL'`.
  It returned 17 rows in 1.56 s. Org country NL was the proxy; the buyer section's BT-514 = NLD was confirmed afterwards for
  all 7 NL AddInfo/TenderReceipt picks.
- **Pool.** 217 rows over the 7 windows, all distinct keys.
  - By role: ReviewOrg 135, ReviewInfo 39, AddInfo 16, TenderReceipt 12, Mediator 10, TenderEval 4, DocProvider 1.
  - By org country: ES 121, NL 57, PL 17, DE 9, FR 7, GR, IT and PT 2 each.
  - 11 groups have no touchpoint BT-514.
- **Pick** (`scripts/pick.py`, deterministic). Rank = `sha256("576-u4-pick-" + "tender|seq|role|org|notice|section")` ascending.
  - Caps: ≤ 2 items per tender, stricter than the asked ≤ 3; ≤ 1 per organization_id; ≤ 1 per touchpoint section (one
    minting decision, one item).
  - Strata in order:
    1. NL AddInfo/TenderReceipt, 6 by rank.
    2. No own BT-514, 3 by rank.
    3. Lot-ReviewOrg, 6 by rank.
    4. Rest, 15: round-robin over roles (alphabetical, ReviewOrg last), each turn taking the least-used country, with ties
       broken by rank.
  - Nothing had to be replaced: every NL pick's buyer is NLD.
- **Resulting counts (30 items, 30 tenders, 30 organizations).**
  - Strata: nl_addinfo_tenderreceipt 6, country_fallback 3, review_org 6, rest 15.
  - Across the 30:
    - NL AddInfo/TenderReceipt 7 (item 27 "Afd. Inkoop" came in through rest);
    - Lot-ReviewOrg 10;
    - no own country (fallback) 5: items 7, 8, 9, 20 and 28.
  - By role: ReviewOrg 10, AddInfo 6, ReviewInfo 5, TenderReceipt 4, Mediator 3, DocProvider 1, TenderEval 1.
  - By org country: NL 8, ES 7, PL 5, DE 4, FR 3, IT, PT and GR 1 each.
  - All items come from segment A. Segment B's one general window was empty; the two Verify tenders cover B.

## Per-item fetches

For each item: GET /v1/tenders/{id}, GET /v1/organizations/{organization_id}, GET /v1/notices/{mention_notice_id}/content.
All bodies are kept in full under `raw/`, at 2.2 MB in total.
- `sample.json` keeps per item:
  - the touchpoint section, its parent ORG section and the buyer ORG section(s) (from OPT-300-Procedure-Buyer);
  - every reference to the touchpoint (`referencing_fields`);
  - the notice's other touchpoints, compact;
  - the served rows for (role, org) and the served buyers;
  - the s08 mention rows of the notice: which organization each ORG/TPO section resolved to;
  - `checks`.
- Extra fetches:
  - for item 15, the 8 sibling notices of tender 306270;
  - for item 21, org 22208093;
  - for item 24, org 49.

## Checks over all 30 (mechanical, no verdicts)

- The current version matches on every item: the tender GET's `version` equals the row's seq.
- The party is served at the window's lot on all 30. On 29, the served lots for (role, org) equal the lots whose same-role
  OPT-301 field in this notice points at the TPO.
  - Item 15 (306270) is served on LOT-0001..0009 while this notice references only LOT-0001.
  - The 8 sibling notices were fetched: each is a one-lot notice whose ReviewOrg points at its own TPO-0001 with the same
    name. That is the per-lot-notice shape, not a stray lot.
- **No party is served at a lot that its notices do not reference.**
- The served `mention_name` equals the touchpoint's BT-500 on all 30.
- Country fallback (TPO without BT-514):
  - items 7 (IT), 8 (FR), 9 (ES), 20 (FR) and 28 (PL);
  - the org country equals the parent ORG's BT-514 on each;
  - items 8 and 28 bind pre-existing orgs: 1163158 Tribunal administratif de Versailles, and KIO 1617.
- The touchpoint's parent ORG is the buyer on 29 of 30. The exception is item 21 (see below).
- Minted orgs are all provisional and identifier-less.
  - Several are long-standing name-keyed rows that the drain bound to: 3955 TACRC, now 122,448 mentions (77,706 in the
    10-21 triage), and KIO 1617 at 620,903.
  - Rows with ids ≥ 31,97x,xxx (items 7, 9, 20, 21, 22, 25, 30) are recent mints, most likely by the unit-3 drain.

## Oddities and things the judges should look at

1. **Prose served as a party.** The gate (iv) does not catch either of these:
   - item 1, AddInfo, "Vraag- en antwoordmodule in het dashboard van deze aanbesteding", org 18044191 with 575 mentions;
   - item 2, TenderReceipt, "Dashboard van deze aanbesteding", org 18044192 with 370 mentions.

   Both orgs predate the drain (id 18.04M), so the same strings were already organizations, from ORG sections.
   The NL window shows more of the same family:
   - 18044191 on tenders 351100, 361015 and 361790;
   - item 2's org on 351100;
   - tender 354191, AddInfo "Dit betreft een vrijwillige transparantie vooraf (onderhandeling zonder bekendmaking). Er is
     geen Inschrijving mogelijk." (org 31975020, not picked: rank/cap).
2. **Natural persons as the organization:** item 5, "Carlo van Acker" (NL AddInfo, 19 mentions); item 20, "PICON PATRICE"
   (FR TenderEval + TenderReceipt, 2 mentions, no own country).
3. **A platform operator in a review role:** item 25, "Staatsanzeiger eServices GmbH", Lot-ReviewInfo on a Bad Neustadt
   (DE) notice, 31 mentions, section id `TPO-0000`. It is not on the portal/platform list.
4. **A generic department label pooled across buyers:** item 27, "Afd. Inkoop", org 19463077 with 33 mentions. One
   name-keyed org stands for any NL buyer's purchasing department.
5. **Buyer self-duplicates under a name variant** (minted because the name differs from the parent's; D576-1 counts a
   department as N, but these are the buyer itself):
   - item 3, the typo "Stiching ROC Midden Nederland" → 17395874 (57 mentions), beside buyer 1226 "Stichting ROC Midden
     Nederland";
   - item 6, the acronym "IBMN" → 20951176, beside buyer 20951175 "Stichting Inkoopbureau Midden Nederland" (the notice's
     TPO-0001 and TPO-0004 are both IBMN);
   - item 9, "Consejería de Economía, Hacienda y Empleo. Comunidad de Madrid" → 31975557, beside buyer 391 "Consejería de
     Economía, Hacienda y Empleo" (that notice's ReviewOrg is ORG-0002, the Madrid tribunal);
   - item 13, ReviewOrg "concello de Mos" → 8682116, for buyer "Junta de Gobierno Local del Concello de Mos".
6. **Item 21 (306251): the same body is served as two organizations in one tender.**
   - The notice's buyer is ORG-0007 "Stadt Erlangen - EBE" → org 22208093 (identifier 09562EBE87). TPO-0001 sits under
     ORG-0001 "Stadt Erlangen - Zentrale Vergabestelle", which is not the buyer, and prints "Stadt Erlangen - EBE".
   - The (i) alias compares with the parent only, so the touchpoint minted 31976959 (2 mentions).
   - The tender now serves Lot-TenderEval → 22208093 and Lot-TenderReceipt → 31976959, both named "Stadt Erlangen - EBE".
   - The served Procedure-Buyer is 1062 (the Zentrale Vergabestelle): v4 comes from notice 24800254, and this party is
     anchored at notice 24590568 (v2's notice).
7. **Item 24 (306091), outside 576:** the served Procedure-Buyer, mention "Uniwersytet Warszawski" with NIP 5262239325, is
   organization 49, named "Krajowa Izba Odwoławcza", whose identifier is that NIP (30,567 mentions). This is an org-layer
   name/identity error on the buyer. The touchpoint party itself, Mediator "Prokuratoria Generalna Rzeczpospolitej
   Polskiej", is unaffected.
8. **Other review-role names that are not tribunals:**
   - item 15, "Registro General de la Consejería de Hacienda, Industria y Energía" (a registry office) as ReviewOrg for the
     Diputación de Jaén;
   - item 7, "COMUNE DI VISCIANO (NA)" as ReviewInfo for "Agenzia Locale di Sviluppo dei Comuni …";
   - item 19, "Secretário de Estado dos Assuntos Fiscais" as ReviewInfo for the Autoridade Tributária.
9. **Raw newlines survive in served names:** item 18's mention_name is "Vergabekammer Schleswig-Holstein\nMinisterium für …"
   (the org name has a space); item 22's touchpoint BT-500 also carries a newline.
10. **TACRC appears under four name-keyed orgs in this sample alone:** 3955 (item 10), 11128836 "– Secretaría" (item 11),
    2720295 "(TACRC)" (item 29), and 22077939, the compound name from the Verify. These are name variants minted per D576-1
    (ii), not new rows.

## The 30 items

| item | stratum | tender | seq | lot | role | org | org country | TPO BT-514 | notice / section | served name | window (statement, row) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | nl_addinfo_tenderreceipt | 346591 | 1 | LOT-0000 | Lot-AddInfo | 18044191 | NL | NLD | 25679676 / TPO-0004 | Vraag- en antwoordmodule in het dashboard van deze aanbesteding | s07_n6 row 2 |
| 2 | nl_addinfo_tenderreceipt | 361790 | 5 | LOT-0000 | Lot-TenderReceipt | 18044192 | NL | NLD | 24843538 / TPO-0001 | Dashboard van deze aanbesteding | s07_n6 row 14 |
| 3 | nl_addinfo_tenderreceipt | 306207 | 2 | LOT-0000 | Lot-TenderReceipt | 17395874 | NL | NLD | 25296822 / TPO-0001 | Stiching ROC Midden Nederland | s03_g2 row 20 |
| 4 | nl_addinfo_tenderreceipt | 360597 | 2 | LOT-0000 | Lot-AddInfo | 10359863 | NL | NLD | 46700884 / TPO-0002 | Team Inkoop gemeente Westland | s07_n6 row 8 |
| 5 | nl_addinfo_tenderreceipt | 344433 | 4 | LOT-0000 | Lot-AddInfo | 12053613 | NL | NLD | 44531077 / TPO-0002 | Carlo van Acker | s07_n6 row 0 |
| 6 | nl_addinfo_tenderreceipt | 361722 | 3 | LOT-0000 | Lot-TenderReceipt | 20951176 | NL | NLD | 25426064 / TPO-0001 | IBMN | s07_n6 row 12 |
| 7 | country_fallback | 1163389 | 1 | LOT-0001 | Lot-ReviewInfo | 31977976 | IT | — | 25610055 / TPO-0001 | COMUNE DI VISCIANO (NA) | s04_g3 row 4 |
| 8 | country_fallback | 849304 | 2 | LOT-0001 | Lot-ReviewOrg | 1163158 | FR | — | 24069622 / TPO-0001 | Tribunal Administratif de Versailles | s06_g5 row 23 |
| 9 | country_fallback | 491439 | 1 | LOT-0001 | Lot-ReviewInfo | 31975557 | ES | — | 24053421 / TPO-0001 | Consejería de Economía, Hacienda y Empleo. Comunidad de Madrid | s01_g0 row 9 |
| 10 | review_org | 442325 | 1 | LOT-0000 | Lot-ReviewOrg | 3955 | ES | ESP | 24968208 / TPO-0001 | Tribunal Administrativo Central de Recursos Contractuales | s02_g1 row 30 |
| 11 | review_org | 491553 | 2 | LOT-0000 | Lot-ReviewOrg | 11128836 | ES | ESP | 30069882 / TPO-0001 | Tribunal Administrativo Central de Recursos Contractuales – Secretaría | s01_g0 row 21 |
| 12 | review_org | 306237 | 1 | LOT-0001 | Lot-ReviewOrg | 3483852 | NL | NLD | 24747742 / TPO-0001 | Rechtbank Overijssel | s03_g2 row 21 |
| 13 | review_org | 849168 | 2 | LOT-0000 | Lot-ReviewOrg | 8682116 | ES | ESP | 24711654 / TPO-0001 | concello de Mos | s06_g5 row 11 |
| 14 | review_org | 1163849 | 2 | LOT-0000 | Lot-ReviewOrg | 3708 | ES | ESP | 25020511 / TPO-0001 | Tribunal Administrativo de Recursos Contractuales | s04_g3 row 38 |
| 15 | review_org | 306270 | 10 | LOT-0001 | Lot-ReviewOrg | 12892828 | ES | ESP | 24855819 / TPO-0001 | Registro General de la Consejería de Hacienda, Industria y Energía | s03_g2 row 26 |
| 16 | rest | 491603 | 7 | LOT-0002 | Lot-AddInfo | 8075131 | PL | POL | 25459203 / TPO-0003 | Agencja Mienia Wojskowego Oddział Regionalny w Lublinie | s01_g0 row 24 |
| 17 | rest | 1163775 | 2 | LOT-0001 | Lot-DocProvider | 1298093 | PL | POL | 25290510 / TPO-0001 | Regionalny Zarząd Gospodarki Wodnej w Poznaniu | s04_g3 row 35 |
| 18 | rest | 849196 | 2 | LOT-0001 | Lot-Mediator | 8946945 | DE | DEU | 24912640 / TPO-0002 | Vergabekammer Schleswig-Holstein Ministerium für Wissenschaft, Wirtsch | s06_g5 row 12 |
| 19 | rest | 849402 | 1 | LOT-0001 | Lot-ReviewInfo | 8134860 | PT | PRT | 23879962 / TPO-0001 | Secretário de Estado dos Assuntos Fiscais | s06_g5 row 38 |
| 20 | rest | 849218 | 1 | LOT-0001 | Lot-TenderEval | 31977562 | FR | — | 25036645 / TPO-0001 | PICON PATRICE | s06_g5 row 16 |
| 21 | rest | 306251 | 4 | LOT-0000 | Lot-TenderReceipt | 31976959 | DE | DEU | 24590568 / TPO-0001 | Stadt Erlangen - EBE | s03_g2 row 22 |
| 22 | rest | 442212 | 1 | LOT-0001 | Lot-ReviewOrg | 31976178 | GR | GRC | 24418993 / TPO-0001 | ΕΝΙΑΙΑ ΑΡΧΗ ΔΗΜΟΣΙΩΝ ΣΥΜΒΑΣΕΩΝ (ΕΑΔΗΣΥ / ΟΡΓΑΝΙΣΜΟΣ ΠΡΟΣΦΥΓΗΣ) | s02_g1 row 21 |
| 23 | rest | 442039 | 5 | LOT-0001 | Lot-AddInfo | 14122634 | PL | POL | 28927254 / TPO-0001 | Państwowe Gospodarstwo Wodne Wody Polskie Regionalny Zarząd Gospodarki | s02_g1 row 1 |
| 24 | rest | 306091 | 3 | LOT-0001 | Lot-Mediator | 11005609 | PL | POL | 25710042 / TPO-0002 | Prokuratoria Generalna Rzeczpospolitej Polskiej | s03_g2 row 8 |
| 25 | rest | 306117 | 2 | LOT-0000 | Lot-ReviewInfo | 31975167 | DE | DEU | 24065603 / TPO-0000 | Staatsanzeiger eServices GmbH | s03_g2 row 14 |
| 26 | rest | 1163544 | 1 | LOT-0001 | Lot-ReviewOrg | 1165479 | FR | FRA | 24815364 / TPO-0001 | Tribunal Administratif de Melun | s04_g3 row 21 |
| 27 | rest | 345266 | 2 | LOT-0000 | Lot-AddInfo | 19463077 | NL | NLD | 25856080 / TPO-0003 | Afd. Inkoop | s07_n6 row 1 |
| 28 | rest | 1163722 | 3 | LOT-0001 | Lot-Mediator | 1617 | PL | — | 24414920 / TPO-0001 | Krajowa Izba Odwoławcza | s04_g3 row 32 |
| 29 | rest | 442058 | 1 | LOT-0000 | Lot-ReviewInfo | 2720295 | ES | ESP | 24171168 / TPO-0001 | Tribunal Administrativo Central de Recursos Contractuales (TACRC) | s02_g1 row 2 |
| 30 | rest | 442005 | 10 | LOT-0000 | Lot-ReviewOrg | 31975627 | DE | DEU | 27017752 / TPO-0001 | Vergabeprüfstelle (Emscher) | s02_g1 row 0 |

## SQL statements sent (8 of 12; each plan-probed locally before sending)

Plans: turso `plan-probe` (a copy of target/debug/plan-probe) on (1) a byte copy of `577-triage/plan-copy.db` and (2) the same copy with prod's deferred indexes added through turso (organization_mentions_notice, organizations_country_id/kind_id/identifier_id/name_norm_id/name_country/identity, tender_version_parties_mention_key, tender_version_parties_org_role, tenders_current_deadline/island/procedure_key/source_deadline/source_id/source_published), because plan-copy.db lacks them. The helper (`scripts/sq.py`) refuses any plan step that starts with SCAN. No SCAN appeared. A draft that also joined organization_mentions planned as `SEARCH m USING INDEX organization_mentions_notice (notice_id=?)` on the prod-index copy (a walk of the notice's mentions instead of the PK seek), so that join was dropped before anything was sent.

| # | name | window | a..b | sent (UTC) | secs | rows | answer |
|---|---|---|---|---|---|---|---|
| 1 | s01_g0 | general i=0 (sha256 fbdcd51e41aa9b75, s=491381) | 491382..494382 | 2026-10-24T12:52:36Z | 0.6 | 40 | JSON rows+columns, truncated=false |
| 2 | s02_g1 | general i=1 (sha256 4c2ab1317feed0ba, s=441978) | 441979..444979 | 2026-10-24T12:52:44Z | 0.63 | 40 | JSON rows+columns, truncated=false |
| 3 | s03_g2 | general i=2 (sha256 a1da4ca40e1269ff, s=306031) | 306032..309032 | 2026-10-24T12:52:48Z | 0.72 | 40 | JSON rows+columns, truncated=false |
| 4 | s04_g3 | general i=3 (sha256 f6de6839109995e7, s=1163303) | 1163304..1166304 | 2026-10-24T12:52:53Z | 0.71 | 40 | JSON rows+columns, truncated=false |
| 5 | s05_g4 | general i=4 (sha256 27ca4cdc82e44830, s=1306880) | 8626880..8629880 | 2026-10-24T12:52:57Z | 0.42 | 0 | JSON rows+columns, truncated=false |
| 6 | s06_g5 | general i=5 (sha256 98d9270dfba59324, s=849060) | 849061..852061 | 2026-10-24T12:53:01Z | 0.68 | 40 | JSON rows+columns, truncated=false |
| 7 | s07_n6 | NL AddInfo/TenderReceipt i=6 (sha256 0277899befe79d67, s=343863) | 343864..363864 | 2026-10-24T12:53:29Z | 1.56 | 17 | JSON rows+columns, truncated=false |
| 8 | s08_mentions | organization_mentions of the 30 picked notices | 30 PK probes | 2026-10-24T13:01:21Z | 0.38 | 113 | JSON rows+columns, truncated=false |

No 408, no shed, no retry. Statements were >= 3.5 s apart (the helper enforces it).

### s01_g0

```sql
SELECT t.id AS tender_id, p.seq AS seq, p.role AS role, p.organization_id AS org_id, p.mention_notice_id AS notice_id, p.mention_section_id AS section_id, min(p.lot_id) AS lot_id_min, count(*) AS n_rows, o.country AS org_country, o.name AS org_name, c.code AS tp_bt514 FROM tenders t CROSS JOIN tender_version_parties p ON p.tender_id = t.id AND p.seq = t.current_seq CROSS JOIN organizations o ON o.id = p.organization_id LEFT JOIN notice_codes c ON c.notice_id = p.mention_notice_id AND c.section_id = p.mention_section_id AND c.field_id = 'BT-514-Organization-TouchPoint' AND c.ordinal = 0 WHERE t.id BETWEEN 491382 AND 494382 AND p.mention_section_id LIKE 'TPO-%' GROUP BY t.id, p.seq, p.role, p.organization_id, p.mention_notice_id, p.mention_section_id LIMIT 40
```

Local plan:

```
# plan-copy
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
SEARCH c USING INDEX sqlite_autoindex_notice_codes_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?) LEFT-JOIN
USE SORTER FOR GROUP BY
# plan-copy+prod-indexes
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
SEARCH c USING INDEX sqlite_autoindex_notice_codes_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?) LEFT-JOIN
USE SORTER FOR GROUP BY
```

### s02_g1

```sql
SELECT t.id AS tender_id, p.seq AS seq, p.role AS role, p.organization_id AS org_id, p.mention_notice_id AS notice_id, p.mention_section_id AS section_id, min(p.lot_id) AS lot_id_min, count(*) AS n_rows, o.country AS org_country, o.name AS org_name, c.code AS tp_bt514 FROM tenders t CROSS JOIN tender_version_parties p ON p.tender_id = t.id AND p.seq = t.current_seq CROSS JOIN organizations o ON o.id = p.organization_id LEFT JOIN notice_codes c ON c.notice_id = p.mention_notice_id AND c.section_id = p.mention_section_id AND c.field_id = 'BT-514-Organization-TouchPoint' AND c.ordinal = 0 WHERE t.id BETWEEN 441979 AND 444979 AND p.mention_section_id LIKE 'TPO-%' GROUP BY t.id, p.seq, p.role, p.organization_id, p.mention_notice_id, p.mention_section_id LIMIT 40
```

Local plan:

```
# plan-copy
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
SEARCH c USING INDEX sqlite_autoindex_notice_codes_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?) LEFT-JOIN
USE SORTER FOR GROUP BY
# plan-copy+prod-indexes
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
SEARCH c USING INDEX sqlite_autoindex_notice_codes_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?) LEFT-JOIN
USE SORTER FOR GROUP BY
```

### s03_g2

```sql
SELECT t.id AS tender_id, p.seq AS seq, p.role AS role, p.organization_id AS org_id, p.mention_notice_id AS notice_id, p.mention_section_id AS section_id, min(p.lot_id) AS lot_id_min, count(*) AS n_rows, o.country AS org_country, o.name AS org_name, c.code AS tp_bt514 FROM tenders t CROSS JOIN tender_version_parties p ON p.tender_id = t.id AND p.seq = t.current_seq CROSS JOIN organizations o ON o.id = p.organization_id LEFT JOIN notice_codes c ON c.notice_id = p.mention_notice_id AND c.section_id = p.mention_section_id AND c.field_id = 'BT-514-Organization-TouchPoint' AND c.ordinal = 0 WHERE t.id BETWEEN 306032 AND 309032 AND p.mention_section_id LIKE 'TPO-%' GROUP BY t.id, p.seq, p.role, p.organization_id, p.mention_notice_id, p.mention_section_id LIMIT 40
```

Local plan:

```
# plan-copy
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
SEARCH c USING INDEX sqlite_autoindex_notice_codes_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?) LEFT-JOIN
USE SORTER FOR GROUP BY
# plan-copy+prod-indexes
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
SEARCH c USING INDEX sqlite_autoindex_notice_codes_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?) LEFT-JOIN
USE SORTER FOR GROUP BY
```

### s04_g3

```sql
SELECT t.id AS tender_id, p.seq AS seq, p.role AS role, p.organization_id AS org_id, p.mention_notice_id AS notice_id, p.mention_section_id AS section_id, min(p.lot_id) AS lot_id_min, count(*) AS n_rows, o.country AS org_country, o.name AS org_name, c.code AS tp_bt514 FROM tenders t CROSS JOIN tender_version_parties p ON p.tender_id = t.id AND p.seq = t.current_seq CROSS JOIN organizations o ON o.id = p.organization_id LEFT JOIN notice_codes c ON c.notice_id = p.mention_notice_id AND c.section_id = p.mention_section_id AND c.field_id = 'BT-514-Organization-TouchPoint' AND c.ordinal = 0 WHERE t.id BETWEEN 1163304 AND 1166304 AND p.mention_section_id LIKE 'TPO-%' GROUP BY t.id, p.seq, p.role, p.organization_id, p.mention_notice_id, p.mention_section_id LIMIT 40
```

Local plan:

```
# plan-copy
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
SEARCH c USING INDEX sqlite_autoindex_notice_codes_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?) LEFT-JOIN
USE SORTER FOR GROUP BY
# plan-copy+prod-indexes
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
SEARCH c USING INDEX sqlite_autoindex_notice_codes_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?) LEFT-JOIN
USE SORTER FOR GROUP BY
```

### s05_g4

```sql
SELECT t.id AS tender_id, p.seq AS seq, p.role AS role, p.organization_id AS org_id, p.mention_notice_id AS notice_id, p.mention_section_id AS section_id, min(p.lot_id) AS lot_id_min, count(*) AS n_rows, o.country AS org_country, o.name AS org_name, c.code AS tp_bt514 FROM tenders t CROSS JOIN tender_version_parties p ON p.tender_id = t.id AND p.seq = t.current_seq CROSS JOIN organizations o ON o.id = p.organization_id LEFT JOIN notice_codes c ON c.notice_id = p.mention_notice_id AND c.section_id = p.mention_section_id AND c.field_id = 'BT-514-Organization-TouchPoint' AND c.ordinal = 0 WHERE t.id BETWEEN 8626880 AND 8629880 AND p.mention_section_id LIKE 'TPO-%' GROUP BY t.id, p.seq, p.role, p.organization_id, p.mention_notice_id, p.mention_section_id LIMIT 40
```

Local plan:

```
# plan-copy
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
SEARCH c USING INDEX sqlite_autoindex_notice_codes_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?) LEFT-JOIN
USE SORTER FOR GROUP BY
# plan-copy+prod-indexes
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
SEARCH c USING INDEX sqlite_autoindex_notice_codes_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?) LEFT-JOIN
USE SORTER FOR GROUP BY
```

### s06_g5

```sql
SELECT t.id AS tender_id, p.seq AS seq, p.role AS role, p.organization_id AS org_id, p.mention_notice_id AS notice_id, p.mention_section_id AS section_id, min(p.lot_id) AS lot_id_min, count(*) AS n_rows, o.country AS org_country, o.name AS org_name, c.code AS tp_bt514 FROM tenders t CROSS JOIN tender_version_parties p ON p.tender_id = t.id AND p.seq = t.current_seq CROSS JOIN organizations o ON o.id = p.organization_id LEFT JOIN notice_codes c ON c.notice_id = p.mention_notice_id AND c.section_id = p.mention_section_id AND c.field_id = 'BT-514-Organization-TouchPoint' AND c.ordinal = 0 WHERE t.id BETWEEN 849061 AND 852061 AND p.mention_section_id LIKE 'TPO-%' GROUP BY t.id, p.seq, p.role, p.organization_id, p.mention_notice_id, p.mention_section_id LIMIT 40
```

Local plan:

```
# plan-copy
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
SEARCH c USING INDEX sqlite_autoindex_notice_codes_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?) LEFT-JOIN
USE SORTER FOR GROUP BY
# plan-copy+prod-indexes
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
SEARCH c USING INDEX sqlite_autoindex_notice_codes_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?) LEFT-JOIN
USE SORTER FOR GROUP BY
```

### s07_n6

```sql
SELECT t.id AS tender_id, p.seq AS seq, p.role AS role, p.organization_id AS org_id, p.mention_notice_id AS notice_id, p.mention_section_id AS section_id, min(p.lot_id) AS lot_id_min, count(*) AS n_rows, o.country AS org_country, o.name AS org_name, c.code AS tp_bt514 FROM tenders t CROSS JOIN tender_version_parties p ON p.tender_id = t.id AND p.seq = t.current_seq CROSS JOIN organizations o ON o.id = p.organization_id LEFT JOIN notice_codes c ON c.notice_id = p.mention_notice_id AND c.section_id = p.mention_section_id AND c.field_id = 'BT-514-Organization-TouchPoint' AND c.ordinal = 0 WHERE t.id BETWEEN 343864 AND 363864 AND p.mention_section_id LIKE 'TPO-%' AND +p.role IN ('Lot-AddInfo', 'Lot-TenderReceipt') AND +o.country = 'NL' GROUP BY t.id, p.seq, p.role, p.organization_id, p.mention_notice_id, p.mention_section_id LIMIT 40
```

Local plan:

```
# plan-copy
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
SEARCH c USING INDEX sqlite_autoindex_notice_codes_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?) LEFT-JOIN
USE SORTER FOR GROUP BY
# plan-copy+prod-indexes
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH p USING INDEX tender_version_parties_version (tender_id=? AND seq=?)
SEARCH o USING INTEGER PRIMARY KEY (rowid=?)
SEARCH c USING INDEX sqlite_autoindex_notice_codes_1 (notice_id=? AND section_id=? AND field_id=? AND ordinal=?) LEFT-JOIN
USE SORTER FOR GROUP BY
```

### s08_mentions

```sql
SELECT notice_id, section_id, organization_id, name, country, scheme, raw_identifier FROM organization_mentions WHERE notice_id = 25679676 UNION ALL SELECT notice_id, section_id, organization_id, name, country, scheme, raw_identifier FROM organization_mentions WHERE notice_id = 24843538 UNION ALL SELECT notice_id, section_id, organization_id, name, country, scheme, raw_identifier FROM organization_mentions WHERE notice_id = 25296822 UNION ALL SELECT notice_id, section_id, organization_id, name, country, scheme, raw_identifier FROM organization_mentions WHERE notice_id = 46700884 UNION ALL SELECT notice_id, section_id, organization_id, name, country, scheme, raw_identifier FROM organization_mentions WHERE notice_id = 44531077 UNION ALL SELECT notice_id, section_id, organization_id, name, country, scheme, raw_identifier FROM organization_mentions WHERE notice_id = 25426064 UNION ALL SELECT notice_id, section_id, organization_id, name, country, scheme, raw_identifier FROM organization_mentions WHERE notice_id = 25610055 UNION ALL SELECT notice_id, section_id, organization_id, name, country, scheme, raw_identifier FROM organization_mentions WHERE notice_id = 24069622 UNION ALL SELECT notice_id, section_id, organization_id, name, country, scheme, raw_identifier FROM organization_mentions WHERE notice_id = 24053421 UNION ALL SELECT notice_id, section_id, organization_id, name, country, scheme, raw_identifier FROM organization_mentions WHERE notice_id = 24968208 UNION ALL SELECT notice_id, section_id, organization_id, name, country, scheme, raw_identifier FROM organization_mentions WHERE notice_id = 30069882 UNION ALL SELECT notice_id, section_id, organization_id, name, country, scheme, raw_identifier FROM organization_mentions WHERE notice_id = 24747742 UNION ALL SELECT notice_id, section_id, organization_id, name, country, scheme, raw_identifier FROM organization_mentions WHERE notice_id = 24711654 UNION ALL SELECT notice_id, section_id, organization_id, name, country, scheme, raw_identifier FROM organization_mentions WHERE notice_id = 25020511 UNION ALL SELECT notice_id, section_id, organization_id, name, country, scheme, raw_identifier FROM organization_mentions WHERE notice_id = 24855819 UNION ALL SELECT notice_id, section_id, organization_id, name, country, scheme, raw_identifier FROM organization_mentions WHERE notice_id = 25459203 UNION ALL SELECT notice_id, section_id, organization_id, name, country, scheme, raw_identifier FROM organization_mentions WHERE notice_id = 25290510 UNION ALL SELECT notice_id, section_id, organization_id, name, country, scheme, raw_identifier FROM organization_mentions WHERE notice_id = 24912640 UNION ALL SELECT notice_id, section_id, organization_id, name, country, scheme, raw_identifier FROM organization_mentions WHERE notice_id = 23879962 UNION ALL SELECT notice_id, section_id, organization_id, name, country, scheme, raw_identifier FROM organization_mentions WHERE notice_id = 25036645 UNION ALL SELECT notice_id, section_id, organization_id, name, country, scheme, raw_identifier FROM organization_mentions WHERE notice_id = 24590568 UNION ALL SELECT notice_id, section_id, organization_id, name, country, scheme, raw_identifier FROM organization_mentions WHERE notice_id = 24418993 UNION ALL SELECT notice_id, section_id, organization_id, name, country, scheme, raw_identifier FROM organization_mentions WHERE notice_id = 28927254 UNION ALL SELECT notice_id, section_id, organization_id, name, country, scheme, raw_identifier FROM organization_mentions WHERE notice_id = 25710042 UNION ALL SELECT notice_id, section_id, organization_id, name, country, scheme, raw_identifier FROM organization_mentions WHERE notice_id = 24065603 UNION ALL SELECT notice_id, section_id, organization_id, name, country, scheme, raw_identifier FROM organization_mentions WHERE notice_id = 24815364 UNION ALL SELECT notice_id, section_id, organization_id, name, country, scheme, raw_identifier FROM organization_mentions WHERE notice_id = 25856080 UNION ALL SELECT notice_id, section_id, organization_id, name, country, scheme, raw_identifier FROM organization_mentions WHERE notice_id = 24414920 UNION ALL SELECT notice_id, section_id, organization_id, name, country, scheme, raw_identifier FROM organization_mentions WHERE notice_id = 24171168 UNION ALL SELECT notice_id, section_id, organization_id, name, country, scheme, raw_identifier FROM organization_mentions WHERE notice_id = 27017752
```

Local plan:

```
# plan-copy
COMPOUND QUERY
  LEFT-MOST SUBQUERY
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
# plan-copy+prod-indexes
COMPOUND QUERY
  LEFT-MOST SUBQUERY
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
  UNION ALL
    SEARCH organization_mentions USING INDEX sqlite_autoindex_organization_mentions_1 (notice_id=?)
```
