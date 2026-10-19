# Audit: one recent DÖE Tender and one recent FTS Tender against their source notices (2026-10-19)

The audit used only public GETs: 15 of the 30 allowed, at least 2 s apart, with no 408s and no retries. Every request is
in `api_requests.log` on the lines tagged `agent=doe-fts`. Another agent was writing to the same log and the same `raw/`
directory at the same time, so this audit's responses are under `raw/doe-fts/`. Two notes on the log:

- Lines #1 and #2 (`/docs` and `/v1/openapi.json`) were written after the requests. A bug in the wrapper dropped the
  original lines.
- Number #7 is skipped. A NOTE line was counted by mistake, so the numbers end at #16 but there were 15 requests.

**Result: 0 discrepancies.** On both Tenders, every served field either equals its source notice value or has a
documented rule that explains the difference. The explained differences are listed at the end of each section.

## Selection (deterministic)

- **How the list was read.** The cursor on `/v1/changes` is opaque. `/docs` says not to do arithmetic on it, so the
  only way to reach recent events would be to page forward from `since=0`. This audit used the documented list endpoint
  instead, newest first by publication time:
  - `GET /v1/tenders?source=doe&published_before=now&limit=10`
  - `GET /v1/tenders?source=fts&published_before=now&limit=10`
  - `published_before` on its own implies `sort=published_at` in descending order. `now` keeps out any future or
    placeholder dates.
- **What qualifies.** A row qualifies if it is served with `source` equal to `doe` (or `fts`), meaning a notice from
  that source caused its current version. Rows count in the order served.
- **Which items.** "The 3rd and 7th qualifying items" was read as the **3rd DÖE row** and the **7th FTS row**:
  - **DÖE: Tender 8852013.** Publication `25944788-1`, published 2026-10-17, one version. Its notice is
    **47414315**: a DÖE eForms sdk-0.1 `can-standard` result notice from the numeric channel (an island).
  - **FTS: Tender 8590779.** Publication `098239-2026`, published 2026-10-18T19:22:34+01:00, four versions. Its
    notices are **46714265** (UK3), **46857338** (UK3), **47414815** (UK4) and **47414817** (UK4, the current
    version). The OCDS id is `ocds-h6vhtk-077b0d`.

All four FTS notices were read, not only the current one. That way, a value carried forward from an earlier notice, or
a stale one left behind, would show up.

---

## 1. DÖE: Tender 8852013 ← notice 47414315 (sdk-0.1, `can-standard`)

| Served field | Served value | Source (notice 47414315 content) | Verdict |
|---|---|---|---|
| source / kind | `doe` / `procedure` | DÖE sdk-0.1 (`SDK01-CustomizationID` eforms-sdk-0.1) | match |
| publication_id | `25944788-1` | `SDK01-ID` 25944788 + `SDK01-VersionID` 1 | match (DÖE `<number>-<version>`) |
| procedure_key | null | no linkage key (numeric channel) | match: island, as documented |
| version / versions[] | 1; seq 1 ← 47414315 | the only notice | match |
| title (row, detail, texts proc) | "Brandschutzmaßnahmen östlicher Collegienflügel TGA Planung" | `SDK01-ProcurementProject-Name` | byte-equal |
| texts description (proc) | "Elektrofachplanung für …" | `SDK01-ProcurementProject-Description` | byte-equal |
| texts title/description (LOT-0000) | same as above | `SDK01-ProcurementProjectLot-ProcurementProject-Name` / `-Description` | byte-equal |
| texts lang / original_lang | null / null | no language stated anywhere in the notice | match (explained E4) |
| notice_subtype | null | `SDK01-NoticeTypeCode` = `can-standard` | explained E3 |
| buyer (parties) | role `buyer`, organization 18174, organization_name and mention_name both "Landkreis Helmstedt", lot null | `ND-ContractingParty#0` `SDK01-ContractingParty-Party-PartyName-Name` "Landkreis Helmstedt" (no identifier, no country) | match (role vocabulary: E2) |
| organization 18174 | name "Landkreis Helmstedt", identifier null, country null, provisional true, 555 mentions | name only, no identifier, no party country | match (E6) |
| other parties / winners / bids / contracts | none / `[]` / `[]` / `[]` | no tenderer, winner or contract published | match (sdk-0.1 publisher silence is documented) |
| CPV | main 71240000, additional 71300000, at procedure and at LOT-0000; row `cpv` [71300000, 71240000] | `…MainCommodityClassification` 71240000, `…AdditionalCommodityClassification` 71300000, at both levels | match |
| country / place | `DE9` (nuts place, procedure and lot); row `country` [DE9] | `RealizedLocation-Address-CountrySubentityCode` DE9 (nuts); also `Country-IdentificationCode` DEU and city "38350 Helmstedt" | match (DEU and the city: E5 / E7) |
| procedure_type / `procedure` classification | null / no row | no ProcedureCode in the notice | match ("null where no notice published a mapped one") |
| nature classification | no row | no contract-nature code in the notice | match |
| status | closed (`?publication_id=25944788-1&source=doe&status=closed` returns 8852013) | no submission deadline published | match (status rule: no deadline in the future means closed) |
| published_at | `2026-10-17` (date only) | `SDK01-RequestedPublicationDate` 2026-10-17; `SDK01-IssueDate` 2026-10-17T08:57:46+02:00 | match under issue 539 (E1) |
| dispatched_at (row and versions[]) | `2026-10-17T08:57:46+02:00` | `SDK01-IssueDate` 2026-10-17T08:57:46+02:00 | match, with the offset as published |
| submission_deadline / scope | null / null | none (no EndDate either) | match |
| dates[] | `[]` | only IssueDate, RequestedPublicationDate and AwardDate, each served in its own field (published_at, dispatched_at, lot_results.decided) | match |
| value / amounts / value_derivation | null / `[]` / null | no amount published | match |
| lots / lot_details | 1; LOT-0000, kind Lot, same title, value null, deadline null | one `LOT-0000` | match (sdk-0.1 single lot serves 1, as documented) |
| lot_results | one entry: key `ND-TenderResult#0`, lot null, decided `2026-10-17T08:57:46+02:00`, decision/awarded/reason null, winners `[]`, statistics `{}`, notice_id 47414315 | `ND-TenderResult#0` (parent PROCEDURE) `SDK01-TenderResult-AwardDate` 2026-10-17T08:57:46+02:00; nothing else | match |
| documents / links | not served (no field) | `CallForTendersDocumentReference`: URI (deutsche-evergabe deeplink …/awardnotice?lot=), ID 25944788, DocumentType non-restricted-document | not modelled (E7) |

**Discrepancies: none.**

**Explained differences (DÖE):**

- **E1.** `published_at` is the date-only `2026-10-17`, while the notice's IssueDate carries a time. A DÖE
  `published_at` is the requested publication date. It is raised to the IssueDate's day, as a date only, only when that
  day is later. Here both are 2026-10-17, so the requested date is served unchanged, as a date only. Sources: /docs
  Conventions and caveats → Dates ("DÖE publishes no journal date…"); OpenAPI `Tender.published_at`; CHANGELOG issue
  539. `dispatched_at` is the IssueDate itself ("falls on or before the day of published_at").
- **E2.** The party role is served as `buyer`. The notice calls it a UBL `ContractingParty`. Per OpenAPI
  `TenderDetail.parties`, the canonical vocabulary includes `buyer` ("So a buyer is `buyer` or `Procedure-Buyer`").
- **E3.** `notice_subtype` is null although the notice carries `SDK01-NoticeTypeCode` = `can-standard`. Per /docs
  caveats → Coverage and OpenAPI `Tender.notice_subtype` (issue 535), it is null for DÖE sdk-0.1, and the document type
  is kept in the notice content under `SDK01-NoticeTypeCode`, which is exactly where it is.
- **E4.** `original_lang` and `texts[].lang` are null. Per OpenAPI `original_lang`, the value is null where the notice
  states no language, "as many DÖE sdk-0.1 notices do not".
- **E5.** The country is served as the NUTS code `DE9`, not the published ISO code `DEU`. Per /docs (filter `country`)
  and OpenAPI `Tender.country`, NUTS place codes are served, and the level-0 fallback applies only when a version has no
  NUTS code.
- **E6.** The buyer's organization is provisional, with a null identifier and a null country. The notice publishes a
  name only, with no identifier and no party address. Per CONTEXT.md and /docs caveats → Codes and identities,
  identifier-less mentions resolve by normalised name and country to one `provisional` row (555 mentions is allowed).
  Per OpenAPI `Organization.country`, the country is "null where no publication stated one".
- **E7.** The document link, the city and the regulatory domain (`32014L0024`) are not on the detail. These are fields
  the canonical layer does not model. /docs → Notice content says to use the content endpoint for them, and CONTEXT.md
  says documents and attachments are out of scope. Nothing served contradicts them.

---

## 2. FTS: Tender 8590779 ← notices 46714265, 46857338, 47414815, 47414817 (current)

The current version (seq 4) is caused by notice 47414817. Every current-version field below is checked against that
notice. Versions 1 to 3 are checked against their own notices for the `versions[]` metadata and for supersession.

| Served field | Served value | Source (47414817 unless noted) | Verdict |
|---|---|---|---|
| source / kind | `fts` / `procedure` | FTS OCDS release (`BT-01-notice` 2023/54 UKPGA) | match |
| publication_id | `098239-2026` | not in the content (no release-id field) | content cannot verify it; `?publication_id=098239-2026&source=fts` resolves to 8590779 (consistent, but not independent) |
| procedure_key | `ocds-h6vhtk-077b0d` | `BT-04-notice` `ocds-h6vhtk-077b0d` (same in all 4 notices) | match |
| version | 4 | 4 notices | match |
| versions[].published_at | seq1 2026-09-27T21:15:28+01:00, seq2 2026-09-29T09:03:27+01:00, seq3 2026-10-18T16:17:27+01:00, seq4 2026-10-18T19:22:34+01:00 | `OPP-012-notice` of 46714265, 46857338, 47414815, 47414817: identical values | match |
| versions[].notice_subtype | UK3, UK3, UK4, UK4 | `OPP-070-notice` UK3, UK3, UK4, UK4 | match |
| versions[].original_lang | ENG ×4 | `BT-702(a)-notice` `en` ×4 | match (E9) |
| title (row, detail) | "For the provision of Out-of-Hour Emergency Service … telephone service" | `BT-21-Procedure` | byte-equal (v1/v2 titles differ: superseded, E11) |
| texts description | "To meet legal requirements … does not guarantee the Contract value or spend." | `BT-24-Procedure` of 47414817 | byte-equal; v3's text has trailing spaces before `\n`, so the newest notice's text is the one served |
| texts lang | `ENG` | `en` | explained E9 |
| buyer (parties) | role `Procedure-Buyer`, organization 30915379, organization_name and mention_name both "Oldham Metropolitan Borough Council", lot null | `OPT-300-Procedure-Buyer` → `ORG-GB-PPON-PMRJ-8362-DVNT`: `BT-500` "Oldham Metropolitan Borough Council", `BT-501` GB-PPON-PMRJ-8362-DVNT (scheme GB-PPON), `BT-514` GB | match |
| organization 30915379 | name "Oldham Metropolitan Borough Council", identifier `GBPPONPMRJ8362DVNT`, identifier_kind `national`, country GB, provisional false, identifier_status null | as above | match (spelling: E10) |
| other parties / bids / lot_results / contracts | none / `[]` / `[]` / `[]` | UK4 tender notice: no tenderer, award or contract | match |
| CPV | lot 1: main 79510000, additional 98340000; row `cpv` [98340000, 79510000]; no Tender-scope CPV | `BT-262-Lot` 79510000, `BT-263-Lot` 98340000; no procedure-level CPV | match (v1/v2's 79512000 / 98341000 correctly not served: E11) |
| country / place | `UKD` (nuts place, lot 1); row `country` [UKD] | `BT-5071-Lot` UKD | match |
| procedure_type / `procedure` classification | `open` / `open` | `BT-105-Procedure` `open` | match |
| nature | `services` | `BT-23-Procedure` `services` | match |
| status | open (`?publication_id=098239-2026&source=fts&status=open` returns 8590779) | deadline 2026-11-16 in the future; newest notice is a UK4 tender notice with no results | match |
| published_at | `2026-10-18T19:22:34+01:00` | `OPP-012-notice` 2026-10-18T19:22:34+01:00 | match, with the offset as published |
| dispatched_at | null (row and every version) | no dispatch date in any of the 4 notices | match (no value to serve; E12) |
| submission_deadline / scope | `2026-11-16T17:00:00+00:00` / `procedure` | `BT-131(d)-Procedure` 2026-11-16T17:00:00+00:00 | match (v1/v2's 2026-11-20 superseded) |
| dates[] | additional_information_deadline 2026-11-06T17:00:00+00:00; submission_deadline 2026-11-16T17:00:00+00:00; lot 1 duration_start 2027-01-01T00:00:00+00:00; lot 1 duration_end 2029-12-31T23:59:59+00:00 | `BT-13(d)-Procedure`, `BT-131(d)-Procedure`, `BT-536-Lot`, `BT-537-Lot` (v4: 2029-12-31; v3: 2030-12-31) | match: v4's corrected end date is served, not v3's |
| value | £750,000.00 (`75000000` GBP cents) | `BT-27-Procedure` 75000000 GBP | match (v1/v2 £130,000 superseded) |
| amounts | estimated_value (Tender) 75000000 GBP; estimated_value (lot 1) 75000000 GBP | `BT-27-Procedure` and `BT-27-Lot`, both 75000000 GBP | match |
| value_derivation | null | (published figure) | match |
| lots / lot_details | 1; lot `1`, kind Lot, title null, value 75000000 GBP, deadline 2026-11-16T17:00:00+00:00 scope `procedure` | lot section `1` with no lot title; `BT-27-Lot` 75000000 GBP; only a procedure-level deadline | match (lot deadline: E13) |
| documents / links | not served | none published in any of the 4 notices | match |

**Discrepancies: none.**

**Explained differences (FTS):**

- **E9.** `original_lang` and `texts[].lang` are served as `ENG`, while the source publishes `en`. Per OpenAPI
  `Tender.original_lang` (ADR-0013), languages map to uppercase ISO 639-2/T. Per /docs `lang`, `de` and `DEU` both work.
  Same language, different spelling.
- **E10.** The identifier is served as `GBPPONPMRJ8362DVNT`, while the source publishes `GB-PPON-PMRJ-8362-DVNT`
  (scheme GB-PPON). Per /docs Lookups (identifier) and OpenAPI `Organization.identifier` (issue 300), identifiers are
  stored normalised: upper-case letters and digits, no separators. `identifier_kind` uses the documented register
  vocabulary (`national`, `vat`, `eu`, …).
- **E11.** Versions 1 and 2 (UK3 planned-procurement notices) published other values: title, description, CPV
  79512000/98341000, £130,000 tax excl., deadline 2026-11-20T00:00:00+00:00, and duration 2026-12-01 to 2028-11-30.
  None of these is served now. The detail is the current version, and the newest notice states each of these facts
  again (CONTEXT.md Decisions, supersession fold; OpenAPI `Tender` "the current version of one procedure"). The newer
  values replace the older ones and are not merged with them.
- **E12.** `dispatched_at` is null. FTS's OCDS release carries no dispatch date (none of the 4 notices has one), so
  there is nothing to serve. This is not documented as such for FTS, but it does not contradict the source.
- **E13.** Lot 1's `submission_deadline` is the procedure's date, with scope `procedure`. Per /docs caveats → Dates
  (issue 389) and OpenAPI `submission_deadline_scope`, a lot without its own deadline serves the procedure's. The
  source publishes only `BT-131(d)-Procedure`.

**Not served, no contradiction:**

- **Tax basis.** The notice's tax basis (`TED-VAL_TOTAL_TAX_BASIS` = `incl` on v3/v4; `excl` on v1/v2) does not appear
  on the detail's `amounts[]` entries. /docs caveats → Amounts describes `tax_basis` without saying which surface
  carries it, so this is an omission, not a contradiction.
- **Legal basis.** `BT-01-notice` (`2023/54`, UKPGA) is not modelled on the detail.

---

## Limits of this audit

- **The DÖE side is thin.** The 3rd DÖE row happens to be an island sdk-0.1 result notice. It publishes no value,
  winner, deadline, procedure type or language, so for DÖE the value, winner and deadline checks only confirm that
  nothing was invented.
- **FTS `publication_id` was not checked against the source.** The notice content carries no release id, so it can only
  be checked against the server's own lookup, which is not independent.
- **Status comes from the filter.** It is not a field on the detail, so it was checked through `?status=` on a
  `publication_id` lookup, one GET per Tender.
