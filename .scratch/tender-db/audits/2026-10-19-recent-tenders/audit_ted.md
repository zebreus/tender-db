# Audit: two recent TED eForms Tenders against their source notices (2026-10-19)

Agent `ted`. Audit only: public GETs against https://tenders.zebreus.click, 14 GETs in all
(cap 30), spaced at least 2.2 s apart, no 408s, nothing retried. Every GET is logged in
`api_requests.log`, which is shared with agent `doe-fts`. My lines are the untagged
`#1 #2 #5 #6 #7 #8 #9 #11 #13` (the shared file threw off the numbering, and a NOTE line in
the log says so) and the lines tagged `agent=ted #10`–`#14`. The raw response bodies are in
`raw/ted/`.

## Result

**No unexplained discrepancy on either Tender.** Every field `/v1/tenders/{id}` serves
matches the notice content that caused it, or differs only in a way /docs, the OpenAPI
descriptions or CONTEXT.md documents. Both picks are subtype-4 PIN-only notices, so the
award-side fields (lots' winners, bids, contracts, values, deadlines) are empty on both sides.
That emptiness is correct, but it means this sample does not exercise the award path.

One side observation is outside the Tender comparison (§4): the notice-content endpoint
labels the procedure-level contract nature `BT-23-Lot` where the SDK's id is
`BT-23-Procedure`. The Tender's `nature` row is not affected.

## 1. Selection

* `/v1/changes` was not used to find recent items. Its cursor is learn order and opaque, and
  /docs forbids arithmetic on it. Reaching the head means paging from `since=0`, which
  this GET budget can't cover. The task allows the documented list endpoints, so I used
  `GET /v1/tenders?source=ted&published_after=2026-10-16T21:50:00Z&limit=20`. The bound
  is now minus 72 h. A bound alone implies `sort=published_at`, newest first.
* All 20 rows have `published_at: "2026-10-19"`, the Monday OJ S 202/2026 package. TED does
  not publish at weekends, and the package of Friday 16 Oct is date-only `2026-10-16`,
  midnight UTC, so it falls before the bound. Within that tie the page runs in descending id.
* The qualifying test: `source=ted`, an 8-digit TED eForms publication number, an eForms
  `notice_subtype`, and a current version published inside the window. All 20 rows pass the
  row-level test. For the two picks I confirmed the profile on `/v1/notices/{id}`:
  `eforms:eforms-sdk-1.14` and `eforms:eforms-sdk-1.13`, `parse_state: parsed`.
* **3rd qualifying row:** Tender **8852009**, `00719572-2026`, notice 47414082.
* **7th qualifying row:** Tender **8852005**, `00721763-2026`, notice 47413608.
* Both are `version: 1`, so each was produced by exactly one notice.

## 2. Tender 8852009 vs notice 47414082 (LV, Valsts robežsardze, catering PIN)

| Served field | Served value | Notice content | Verdict |
|---|---|---|---|
| source / publication_id | `ted` / `00719572-2026` | OPP-010 `00719572-2026`; notice `source: ted` | match |
| notice_subtype | `4` | OPP-070 `4` (BT-02 `pin-only`) | match |
| original_lang | `LAV` | BT-702(a) `LAV` | match |
| title (row, texts title, lot title) | "Ēdināšanas pakalpojumu sniegšana VRS RIP AĀIC “Mucenieki” (4 gadus)" | BT-21-Procedure and BT-21-Part, same text | match |
| texts description (procedure, PAR-0000) | same text as title | BT-24-Procedure and BT-24-Part, same text | match |
| cpv / classifications cpv main (proc + PAR-0000) | `55520000` | BT-262-Procedure and BT-262-Part `55520000`; no BT-263 | match |
| classifications nature (proc + PAR-0000) | `services` | `services` at PROCEDURE (labelled BT-23-Lot, see §4) and BT-23-Part `services` | match |
| country / classifications nuts place | `LV00A` (proc + PAR-0000) | BT-5071-Procedure and BT-5071-Part `LV00A` (BT-5141 `LVA` also published) | match; no `place_country` row because a NUTS code exists, as documented |
| procedure_type | null | no BT-105 in the notice | match, documented: "Null where no notice of the Tender published a mapped one" |
| procedure_key | null | no BT-04. SDK 1.14 *forbids* BT-04-notice on subtype 4 | match: an island, as documented ("null for an island") |
| published_at | `2026-10-19` (date only) | OPP-012 `2026-10-19` | match |
| dispatched_at | `2026-10-16T14:58:07+00:00` | BT-05(a) `2026-10-16T14:58:07+00:00` (BT-803(d) the same) | match |
| submission_deadline / _scope | null / null | no BT-131 / BT-1311 | match |
| dates[] | empty | only BT-127-notice `2026-10-23` (future notice date) | not a contradiction. BT-127 is not in the projection's DATES table, so it is not modelled |
| value / value_derivation / amounts[] | null / null / empty | no amount fields | match |
| lots / lot_details | `1`; PAR-0000 kind `Part`, value null, deadline null | one Part section PAR-0000 (BT-137-Part) | match (docs: an eForms single-part PIN serves 1; Parts are Lots with a kind flag) |
| kind | `procedure` | planning notice for one procurement | consistent |
| parties: Procedure-Buyer | org 4545 "Valsts robežsardze" (head = mention) | OPT-300-Procedure-Buyer → ORG-0001, BT-500 "Valsts robežsardze" | match |
| parties: Part-AddInfo, Part-EmployLegis, Part-EnvironLegis, Part-FiscalLegis | org 4545 | OPT-301-Part-AddInfo/EmployLegis/EnvironLegis/FiscalLegis → ORG-0001 | match (roles served verbatim, as documented) |
| parties: Part-ReviewOrg | org 88 "Iepirkumu uzraudzības birojs" | OPT-301-Part-ReviewOrg → ORG-0002, BT-500 same | match |
| org 4545 (`/v1/organizations/4545`) | identifier `90000086402`, country `LV` | ORG-0001 BT-501 `90000086402`, BT-514 `LVA` | match (alpha-2 is documented) |
| org 88 | identifier `90001263305`, country `LV` | ORG-0002 BT-501 `90001263305`, BT-514 `LVA` | match |
| lot_results / bids / contracts | empty | no LotResult / LotTender / SettledContract sections | match |
| versions[0] | seq 1, caused_by 47414082, same pub id / dates / subtype / lang | as above | match |
| documents / links | (no such field on the detail) | BT-15-Part `https://www.eis.gov.lv/EKEIS/Supplier/Organizer/354` (BT-14 non-restricted) | not served; nothing to contradict |

## 3. Tender 8852005 vs notice 47413608 (NO, Oslo kommune v/ Boligbygg Oslo KF, market-dialogue PIN)

| Served field | Served value | Notice content | Verdict |
|---|---|---|---|
| source / publication_id | `ted` / `00721763-2026` | OPP-010 `00721763-2026` | match |
| notice_subtype | `4` | OPP-070 `4` (BT-02 `pin-only`) | match |
| original_lang | `ENG` | BT-702(a) `ENG` | match |
| title (row, texts title, lot title) | "Temporary accommodation and flat hotel" | BT-21-Procedure and BT-21-Part, same text | match |
| texts description (procedure, PAR-0000) | "Oslo municipality c/o Boligbygg Oslo KF invites tenderers to a written market dialogue …" | BT-24-Procedure and BT-24-Part, same text | match |
| cpv main | `55110000` (proc + PAR-0000) | BT-262-Procedure and BT-262-Part `55110000` | match |
| cpv additional | 13 codes: 44211110 44211200 45212400 45212410 55000000 55100000 55130000 55242000 55250000 55270000 70311000 70333000 85300000 (proc + PAR-0000) | 13 BT-263-Procedure codes and the same 13 BT-263-Part codes | match (same set; the served array is sorted, the notice's order differs) |
| nature | `services` (proc + PAR-0000) | `services` at PROCEDURE (labelled BT-23-Lot, §4) and BT-23-Part | match |
| country / nuts place | `NO081` (proc + PAR-0000) | BT-5071-Procedure and BT-5071-Part `NO081` (BT-5141 `NOR`) | match |
| procedure_type | null | no BT-105 | match, documented |
| procedure_key | null | no BT-04 (forbidden on subtype 4) | match: an island |
| published_at | `2026-10-19` | OPP-012 `2026-10-19` | match |
| dispatched_at | `2026-10-16T10:45:48+00:00` | BT-05(a) `2026-10-16T10:45:48+00:00` | match |
| submission_deadline / _scope | null | none published | match |
| dates[] | empty | BT-127-notice `2027-05-27` only | not modelled (as for 8852009) |
| value / amounts | null / empty | no amount fields | match |
| lots / lot_details | `1`; PAR-0000 `Part`, title "Temporary accommodation and flat hotel" | Part PAR-0000, BT-21-Part same | match |
| parties: Procedure-Buyer | org 4643720 "Oslo kommune v/ Boligbygg Oslo KF" (head = mention) | OPT-300-Procedure-Buyer → ORG-0001, BT-500 same | match |
| parties: Procedure-SProvider | org 1772 "Artifik AS" | ServiceProviderParty OPT-030 `serv-prov`, OPT-300-Procedure-SProvider → ORG-0003 "Artifik AS" | match (role verbatim, as documented) |
| parties: Part-ReviewInfo | org 4643720 | OPT-301-Part-ReviewInfo → ORG-0001 | match |
| parties: Part-ReviewOrg | org 398, organization_name **"Oslo Tingrett"**, mention_name "Oslo tingrett" | OPT-301-Part-ReviewOrg → ORG-0002, BT-500 "Oslo tingrett" | **explained**: the head name is elected across mentions, clustered across casing (issue 524). mention_name keeps the notice's spelling |
| org 4643720 | identifier `974780747`, `NO` | ORG-0001 BT-501 `974780747`, BT-514 `NOR` | match |
| org 1772 | identifier `925364967`, `NO` | ORG-0003 BT-501 `925364967`, BT-514 `NOR` | match |
| org 398 | identifier `926725939`, `NO`, name "Oslo Tingrett" | ORG-0002 BT-501 `926725939`, BT-514 `NOR`, BT-500 "Oslo tingrett" | identifier and country match; name explained (above) |
| lot_results / bids / contracts | empty | none | match |
| versions[0] | seq 1, caused_by 47413608 | as above | match |
| documents / links | (no such field) | BT-615-Part `https://app.artifik.no/procurements/9925` (restricted document), OPT-140 `9925` | not served; nothing to contradict |

## Explained differences (documented, not discrepancies)

1. **8852005, Part-ReviewOrg org 398:** `organization_name` "Oslo Tingrett" against the
   notice's "Oslo tingrett". The head name is elected from the organization's mentions and
   clustered across casing (issue 524: /docs → Tender detail, and OpenAPI `Organization.name`
   and `parties[].organization_name`). `mention_name` serves the notice's spelling.
2. **`procedure_type: null` on both.** A PIN-only notice publishes no BT-105. OpenAPI
   `Tender.procedure_type`: "Null where no notice of the Tender published a mapped one".
3. **`procedure_key: null` on both.** SDK 1.13 and 1.14 forbid BT-04-notice on subtype 4, so
   there is no grouping key. OpenAPI: "null for an island". CONTEXT.md: every Notice yields a
   Tender even without linkage identifiers.
4. **Party roles `Part-AddInfo`, `Part-EmployLegis`, `Part-EnvironLegis`, `Part-FiscalLegis`,
   `Part-ReviewInfo`, `Part-ReviewOrg`, `Procedure-SProvider`** are served as the eForms
   OPT-301 / OPT-300 suffixes, not canonical roles. OpenAPI `parties`: eForms roles are
   "served AS PUBLISHED"; any other value is kept verbatim, not folded.
5. **Organization `country` `LV`/`NO` against BT-514 `LVA`/`NOR`.** The OpenAPI
   `Organization.country` is ISO 3166-1 alpha-2, so this is the same country in another code.
6. **`lots: 1` with `lot_details[0].kind: "Part"` on both.** Parts are Lots with a kind flag.
   An eForms PIN with a part serves 1, and only one with no lot or part serves 0 (/docs →
   Coverage, issue 535).
7. **`country` serves the NUTS code only, not BT-5141 `LVA`/`NOR`.** `country` is NUTS place
   codes, and a level-0 `place_country` row appears only when no NUTS code exists (issue 532,
   OpenAPI `classifications`).
8. **Notice fields not modelled on the Tender:** BT-127 future-notice date, BT-15/BT-615
   document URLs and BT-14, BT-22 internal reference, BT-508 buyer profile, BT-10/BT-11 buyer
   activity and legal type, BT-71 reserved procurement, BT-01 legal basis, BT-728 place text,
   and contact details. These are omissions, not contradictions. /docs → Notice content says
   to use `/v1/notices/{id}/content` "when the projected tender is not enough: to see a field
   the canonical layer does not model". CONTEXT.md scopes documents as "metadata only". So
   the "documents/links" item the task asked about has no served counterpart on
   `/v1/tenders/{id}`.

## 4. Side observation: field id on the notice content, not the Tender

On both notices, `/v1/notices/{id}/content` places the procedure-level contract nature in
section `PROCEDURE` under field id **`BT-23-Lot`**. The SDK's id for the root
`cac:ProcurementProject/cbc:ProcurementTypeCode` is **`BT-23-Procedure`**, and in SDK 1.13
and 1.14 `fields-*.json`, BT-23-Lot is *forbidden* on subtypes 1–6, which includes subtype 4.
The likely mechanism is in `crates/ingest/src/eforms/index.rs` `ALIASES`. The Lot
`ProcurementProject` subtree is grafted onto the root to "fill gaps", and the predicated
BT-23-Lot leaf (`[@listName='contract-nature']`) is a different branch from the
predicate-free BT-23-Procedure leaf, so it fills in and wins the match. The code comment there
says "the value's true context is carried by its section, not by the SDK field id's suffix".
I found no statement of this in /docs, the OpenAPI or CHANGELOG.md. /docs says content field
ids are "in the source's own vocabulary".

The served Tender is not affected: `contract_nature()` in `project.rs` accepts both ids, and
the scope comes from the section. A consumer reading the content endpoint by field id would
miss BT-23-Procedure on these notices. I could not check this against the raw XML, because
TED is outside the allowed hosts. The inference rests on the section placement and the SDK
constraints. Severity is low.

## Files

* `raw/ted/list_tenders_p1.json`: the selection page
* `raw/ted/tender_8852009.json`, `raw/ted/tender_8852005.json`
* `raw/ted/notice_47414082.json`, `raw/ted/notice_47413608.json` (profiles)
* `raw/ted/content_47414082.json`, `raw/ted/content_47413608.json`
* `raw/ted/org_{4545,88,4643720,1772,398}.json`
* `raw/ted/openapi.json`, `raw/ted/docs.html`: the documentation as read at audit time
