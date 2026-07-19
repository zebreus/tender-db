# The eForms data model — research for the tender-db canonical schema

Research date: 2026-07-19.

Primary sources, in order of authority:

1. **The eForms SDK itself**, cloned at `/opt/tender-db/eforms-sdk` on the VPS
   (`ssh root@zebreus.click`). The checkout is the `develop` branch at commit
   `612f530`, whose `fields/fields.json` reports `"sdkVersion": "eforms-sdk-1.15.0"`
   (1.15.0 was tagged 2026-07-15; all release tags were fetched into the clone
   during this research). Where numbers below are computed, the `jq` command is
   shown; unless stated otherwise it was run in that directory.
2. **Official docs** at <https://docs.ted.europa.eu/eforms/latest/> (currently
   documents SDK 1.14) and <https://docs.ted.europa.eu/eforms-common/>.
3. OCDS and TED/ePO material, cited inline in §8.

Confidence markers: statements without a marker are read directly from SDK files
or docs pages. *(inferred)* = my conclusion from evidence; *(unverified)* =
plausible but not confirmed against a primary source.

---

## 1. Big picture: eForms is a UBL 2.3 customization plus one giant extension block

An eForms notice is a UBL 2.3 XML document. There are only **four root document
types** (`notice-types/notice-types.json` → `documentTypes`):

| documentType | root element | namespace |
|---|---|---|
| PIN | `PriorInformationNotice` | `urn:oasis:...:PriorInformationNotice-2` |
| CN | `ContractNotice` | `urn:oasis:...:ContractNotice-2` |
| CAN | `ContractAwardNotice` | `urn:oasis:...:ContractAwardNotice-2` |
| BRIN | `BusinessRegistrationInformationNotice` | `http://data.europa.eu/p27/eforms-business-registration-information-notice/1` (eForms' own XSD, not UBL) |

Everything eForms needs beyond stock UBL lives under UBL's generic extension
point: `ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/…`,
using namespaces `efext`/`efac`/`efbc` (`http://data.europa.eu/p27/eforms-ubl-extension*`;
see `schemas/common/EFORMS-Extension*.xsd` and
<https://docs.ted.europa.eu/eforms/latest/schema/schemas.html>). Crucially, the
**entire tendering-result data set** (who bid, who won, contracts), the
**Organizations block**, change notices, contract modifications and reviews live
inside this extension, not in regular UBL elements. The extension node is
`ND-RootExtension` in fields.json.

The SDK (`fields/`, `notice-types/`, `codelists/`, `schemas/`, `schematrons/`,
`translations/`, `view-templates/`, `examples/`, `efx-grammar/`) is the single
machine-readable definition of all of this. `fields/fields.json` has four data
keys: `xmlStructure` (323 nodes), `fields` (1256 fields), `businessEntities`
(96 entities — **new-ish metadata, present in 1.15/develop; do not assume it in
older SDKs**), plus `sdkVersion`/`ublVersion`/`metadataDatabase`.

```sh
jq '.xmlStructure | length' fields/fields.json   # 323
jq '.fields | length' fields/fields.json         # 1256
jq '[.fields[].btId] | unique | length'          # 357
jq '.businessEntities | length'                  # 96
```

---

## 2. The node graph and the real entity graph

### 2.1 Mechanics

Every node in `xmlStructure` has `id` (ND-…), `parentId` (all except `ND-Root`),
`xpathAbsolute`/`xpathRelative`, `repeatable` (bool), `xsdSequenceOrder`; 56
nodes carry a `businessEntityId`, 13 carry an `identifierFieldId` (the field
holding that entity's technical identifier), 11 a `captionFieldId`. The nodes
form a strict tree (single parent). 97 of 323 nodes are repeatable:

```sh
jq '[.xmlStructure[] | select(.repeatable)] | length' fields/fields.json  # 97
```

Every field has `parentNodeId`; a field instance repeats when any ancestor node
is repeatable (field-level `repeatable.value == true` exists for exactly **one**
field, `BT-702(b)-notice`, additional notice languages). So *the node tree, not
the field list, defines the relational shape*: each repeatable node is a
candidate table, each non-repeatable node just groups columns.

Many nodes are pure XML plumbing (wrappers like `ND-SenderContact`, and ~60
`…Unpublish` nodes that exist only to attach the "unpublished field" privacy
blocks, §3.6). The **business entities** are the real graph. The full annotated
tree (rendered from fields.json during this research) collapses to the
following.

### 2.2 The entity graph (derived from the SDK, not guessed)

Legend: `[R]` = repeatable node = 1..N; identifiers in parentheses are the
`identifierFieldId` / idScheme. "→" arrows are `id-ref` fields whose
`idSchemes`/`referencedBusinessEntityIds` are declared in fields.json
(`jq '[.fields[] | select(.type=="id-ref") | {id, idSchemes, refs: .referencedBusinessEntityIds}]'`).

```text
Notice (ND-Root)
├── ND-ContractingParty [R]  = Buyer role          → Organization (OPT-300-Procedure-Buyer, ORG)
│   └── ND-ServiceProviderParty [R]                → Organization (esender/procurement service provider)
├── ND-ProcedureProcurementScope        procedure-wide scope (CPV, nature, value, place)
├── ND-ProcedureTenderingProcess  = Procedure      (BT-105 procedure type, direct-award justifications,
│   └── ND-PreviousNoticeReference [R]              previous planning notice refs BT-125)
├── ND-ProcedureTerms                              (lot distribution, cross-border law, legal basis,
│   └── ND-LotDistribution/ND-GroupComposition [R] → groups' lot composition; exclusion grounds)
├── ND-Lot [R]        Lot        (BT-137-Lot,       LOT-0000..)
│   ├── ND-LotProcurementScope   (CPV, value, place, duration, strategic proc.)
│   ├── ND-LotTenderingProcess   (deadlines, FA/DPS, auction, submission)
│   └── ND-LotTenderingTerms     (award criteria, selection criteria, EU funds [R],
│                                 docs, reserved participation, OPT-301-* org role refs → ORG/TPO)
├── ND-LotsGroup [R]  LotsGroup  (BT-137-LotsGroup, GLO-0000..)   award criteria + value only
├── ND-Part [R]       Part       (BT-137-Part,      PAR-0000..)   PIN-only "future lot" scope
└── ND-RootExtension (efext:EformsExtension)
    ├── ND-Organizations
    │   ├── ND-Organization [R]  Organization (OPT-200, ORG-0000..)
    │   │   ├── ND-Company            (name, BT-501 registration id, address, contact, legal entity [R])
    │   │   ├── ND-Touchpoint [R]     Touchpoint (OPT-201, TPO-0000..) (per-touchpoint address/contact)
    │   │   └── ND-OrganizationUboReference [R]  → UBO
    │   └── ND-UBO [R]           UltimateBeneficialOwner (OPT-202, UBO-0000..) (person: name, nationality [R], address)
    ├── ND-NoticeResult          NoticeResult (0..1; notice-level totals BT-161, BT-118, BT-1118)
    │   ├── ND-NoticeResultGroupFA [R]  per-LotsGroup framework values  → LotsGroup (BT-556)
    │   ├── ND-LotResult [R]     LotResult (OPT-322, RES-0000..)
    │   │   │                    winner-chosen status BT-142, not-awarded reason BT-144,
    │   │   │                    received-submission statistics [R], review statistics [R]
    │   │   ├── → Lot            BT-13713-LotResult (exactly 1 lot)
    │   │   ├── → Tender         OPT-320-LotResult  (ND-LotResultTenderReference [R])
    │   │   ├── → Contract       OPT-315-LotResult  (ND-LotResultContractReference [R])
    │   │   └── ND-FinancingParty [R] / ND-PayerParty [R]  → Organization (EU funds financing/paying)
    │   ├── ND-LotTender [R]     Tender = a submitted bid (OPT-321, TEN-0000..)
    │   │   │                    BT-3201 buyer-assigned tender id, value BT-720, rank, variant,
    │   │   │                    origin country [R], concession revenue, subcontracting block
    │   │   ├── → Lot or LotsGroup  BT-13714-Tender
    │   │   └── → TenderingParty    OPT-310-Tender
    │   ├── ND-TenderingParty [R]  TenderingParty (OPT-210, TPA-0000..) = bidder consortium
    │   │   ├── ND-Tenderer [R]       member: → Organization (OPT-300-Tenderer) + leader flag OPT-170
    │   │   └── ND-SubContractor [R]  → Organization (OPT-301-Tenderer-SubCont)
    │   │       └── ND-SubContractorTakerReference [R]  → main contractor ORG
    │   └── ND-SettledContract [R]  Contract (OPT-316, CON-0000..)
    │       │                    BT-150 buyer contract id, conclusion date BT-145, title,
    │       │                    framework flag, EU funds [R], assets [R]
    │       ├── → Tender         BT-3202-Contract (ND-SettledContractTenderReference [R])
    │       └── ND-ContractSignatory [R]  → Organization (OPT-300-Contract-Signatory)
    ├── ND-Changes  (0..1, change notices only)
    │   ├── ND-Change [R]        (BT-141 description, BT-718/719 procurement docs changed)
    │   │   └── ND-ChangedSection [R]  BT-13716 → section id of the *previous* notice
    │   └── ND-ChangeReason      (BT-140 code, BT-762 description)
    ├── ND-ContractModification [R]  (subtypes 38–40, E6)
    │   ├── → Contract           BT-1501(c)-Contract (CON of previous notice)
    │   ├── ND-Modification/ND-ModifiedSection [R]  BT-1501(p) → section refs
    │   └── ND-ModificationReason  (BT-200 code, BT-201/202 descriptions)
    ├── ND-ReviewRequests / ND-ReviewStatus [R]  Review (BT-804, REV-…)
    │   │                        review/appeal outcomes; irregularities [R], remedies [R]
    │   ├── → any section        BT-786-Review (idSchemes: CON,GLO,LOT,ORG,PAR,RES,TEN,TPA,TPO,UBO)
    │   └── ND-AppealingParty [R], ND-AppealProcessingParty  → Organization (BT-807/BT-808)
    └── ND-Publication           OPP-010 publication number, OPP-011 OJ S issue, OPP-012 date
Notice (BRIN X01/X02 only)
├── ND-BusinessParty  (the registered EEIG/company, gazette reference, EU/local entity)
└── ND-BusinessCapability [R]
```

Cardinality rules that are *not* visible in the tree but encoded as business
rules (SDK 1.15.0 `CHANGELOG.md`, "Updates on Business Rules"):

- every Lot in a result notice must have **exactly one** LotResult unless a
  framework agreement / DPS is involved;
- a LotResult may reference multiple contracts **only** for FA/DPS;
- BT-3202 (contract→tender ref) unique within the notice.

So the canonical relational reading is *(inferred, high confidence)*:

- **Procedure** 1—N **Lot**; 0—N **LotsGroup** (a grouping of lots for combined
  award, membership via GroupComposition); 0—N **Part** (PIN-only subdivisions
  of planned procurement — Parts are *not* under Lot; a notice has either lots
  or parts depending on subtype).
- **LotResult** = the award decision for one Lot (N LotResults per notice).
- **Tender (tender-db: Bid)** N—1 Lot/LotsGroup, N—1 TenderingParty.
- **TenderingParty** = consortium; M—N Organization via Tenderer members and
  Subcontractor links.
- **SettledContract** M—N Tender (a contract can settle several bids and, under
  FA/DPS, a LotResult can point at several contracts).
- **Organization** is a flat per-notice register; every party role anywhere in
  the notice (buyer, service provider, tenderer, subcontractor, signatory,
  review body, financing/paying party, document provider, mediator, …) is an
  `id-ref` into it — 44 `id-ref` fields total, of which the `OPT-300-*`/
  `OPT-301-*` family covers organization roles. **Buyer** is a role wrapper
  (ND-ContractingParty holds buyer-specific attributes like legal type and
  activity, plus the ORG reference), not a separate organization record.
- **Touchpoint** = an addressable sub-office/contact of an Organization; role
  refs may point at either ORG or TPO (`idSchemes: ["ORG","TPO"]`).

The 96 `businessEntities` entries confirm this list and add a
`changeIdentification` block (`identifyInChangeNotice`, `useInstanceIdentifier`)
marking which entities are addressed by instance identifier in change notices.

Field volume per entity (`jq -r '.fields[].businessEntityId' | sort | uniq -c | sort -rn`):
Lot 356, Tender 141, LotResult 134, LotsGroup 113, Notice 101, Part 89,
Procedure 68, NoticeResult 54, Organisation 53, ReviewStatus 27, BusinessParty
25, Contract 23, Touchpoint 20, UBO 17, ContractModification 11, Buyer 11,
Changes 10, TenderingParty 3. The Lot is by far the widest entity — most
procurement content (scope, terms, process) is lot-level, which supports
tender-db's Lot-centric API design.

---

## 3. fields.json field semantics

### 3.1 Attributes present on field objects (SDK 1.15.0/develop)

```sh
jq '[.fields[] | keys] | add | group_by(.) | map({key: .[0], count: length}) | sort_by(-.count)' fields/fields.json
```

Always present (1256): `id`, `btId`, `name`, `businessEntityId`, `parentNodeId`,
`repeatable`, `type`, `xpathAbsolute`, `xpathRelative`, `xsdSequenceOrder`.
Frequent: `legalType` 1090, `forbidden` 766, `codeList` 493, `attributeName`/
`attributeOf`/`attributes` 480, `presetValue` 364, `mandatory` 357, `maxLength`
247, `pattern` 212, `assert` 197, `privacy` 61, `idSchemes`+
`referencedBusinessEntityIds` 44, `numericRange` 12, `idScheme`+`schemeName` 11,
`dateFieldId`/`timeFieldId` 9, `inChangeNotice` 1.

**Field id naming**: `<btId>-<context>` with optional letter qualifiers —
`BT-36-Lot` vs `BT-36-Part`; `BT-01(c)-Procedure`; `BT-137-Lot` vs
`BT-137-LotsGroup`; split date/time pairs `BT-13(d)-Lot` / `BT-13(t)-Lot`
(linked by `dateFieldId`/`timeFieldId`). One business term therefore maps to
1..n fields; 357 distinct `btId`s expand to 1256 fields.

**Attributes-as-fields**: 480 of the 1256 fields are XML *attributes* of another
field (`attributeOf`), namely `listName` 247, `languageID` 134, `schemeName` 68,
`currencyID` 26, `unitCode` 5. Most are fixed-value plumbing (they carry
`presetValue`, e.g. every code field's `@listName`). Only `currencyID` and
`unitCode` carry real data (currency of an amount, unit of a duration). Three
further `OPA-*` fields ("OPA" prefix, e.g. `OPA-36-Lot-Number`) are virtual
numeric views of measures. **Net content-bearing fields: 1256 − 480 − 3 = 773**
*(computed)*. The ADR-0002 completeness checklist should treat attribute fields
as covered by their parent field's mapping.

### 3.2 Field types and SQL representation

Distribution (`jq '[.fields[].type] | group_by(.) | map({type: .[0], count: length})'`):
code 460, text 342, text-multilingual 134, date 87, id 49, id-ref 44, indicator
33, amount 26, url 23, number 22, integer 11, time 9, phone 8, email 5,
measure 3. (`legalType` — the Regulation-Annex type — is coarser: CODE,
TEXT, IDENTIFIER, DATE, INDICATOR, VALUE, NUMBER, URL, DURATION.)

| SDK type | XML shape | lexical form | suggested SQLite (STRICT) representation |
|---|---|---|---|
| `text` | element | free string, `maxLength` 30–10000 (buckets: 30×12, 100×20, 400×91, 1000×7, 4000×3, 6000×110, 10000×4) | `TEXT` (+ CHECK length only if desired) |
| `text-multilingual` | element repeats once per language with `@languageID` (3-letter code, `eu-official-language` list) even though `repeatable=false` | — | satellite table `(owner_id, field, lang, text)` or `(…, lang, title, description, …)` per entity; **never a single column** |
| `code` | element with `@listName` | token from a genericode list | `TEXT` + FK to codelist-values table (§5) |
| `id` | element with `@schemeName` | pattern e.g. `^LOT-[0-9]{4}$` | keep for lineage; canonical layer replaces with surrogate PK |
| `id-ref` | element with `@schemeName` | same patterns | resolve to real FK at import (ADR: idiomatic SQL) |
| `indicator` | element | `true`/`false` | `INTEGER` 0/1 (SQLite has no BOOLEAN in STRICT) |
| `date` | element | ISO 8601 date **with mandatory zone offset**, e.g. `2019-11-26+01:00` (docs: all-in-one spec §dates; offset `Z` or ±HH:MM up to 14:00) | `TEXT` ISO 8601; keep the offset (see below) |
| `time` | element | `HH:MM:SS±offset` | `TEXT`; usually paired with a date field via `dateFieldId` — canonically merge into one timestamp column |
| `amount` | element + `@currencyID` (code field, `currency` list) | decimal | value `TEXT`/`INTEGER`-cents + `currency TEXT` column pair (decision needed, §Open questions) |
| `measure` | element + `@unitCode` (code field, `duration-unit` list: DAY/MONTH/YEAR…) | decimal + unit — **durations are value+unit, not ISO 8601 `P30D` strings** | `(value NUMERIC, unit TEXT)` pair |
| `number` | element | decimal (weights, percentages) | `REAL` or `TEXT` (exactness decision) |
| `integer` | element | positive whole | `INTEGER` |
| `url`, `phone`, `email` | element | free string | `TEXT` |

Only 3 `measure` fields exist (`BT-36-Lot/Part` duration, `BT-98-Lot` tender
validity), and 9 `time` fields, each paired with a date field.

Date/time caveat: offsets carry meaning (local time at the buyer); normalising
to UTC loses the wall-clock deadline. Store the original offset-bearing string;
add a derived UTC column if range queries need it. *(recommendation)*

### 3.3 Constraint encoding — how "forbidden/mandatory per notice type" works

Every dynamic property (`repeatable`, `mandatory`, `forbidden`, `codeList`,
`pattern`, `assert`, `inChangeNotice`) uses the same structure: a base
`{value, severity}` plus optional `constraints: [{noticeTypes: [...], condition?,
value, severity, message?}]`. `noticeTypes` enumerates notice **subtypes**
("16", "29", …, "E3", "T01"); `condition` is an EFX expression evaluated against
the notice (e.g. `{ND-LocalLegalBasisNoID} ${not(BT-01(e)-Procedure is not present)}`).

- The base `mandatory.value` is `false` for *all* fields
  (`jq '[.fields[] | select(.mandatory.value==true)] | length'` → 0): every
  mandatoriness is per-subtype, possibly per-condition. 357 fields have some
  mandatory rule, 766 have forbidden rules.
- **Field applicability per subtype is therefore computable**: a field can occur
  in subtype S iff no forbidden constraint with `value:true` matches S. This is
  what the Schematron files in `schematrons/` are generated from
  (<https://docs.ted.europa.eu/eforms/latest/schematrons/index.html>).
- `assert` holds cross-field EFX rules (uniqueness, referential integrity like
  "BT-137-Lot in BT-13713-LotResult", arithmetic checks on statistics).
  tender-db does **not** need to re-implement these — TED already validated
  published notices — but they document invariants the canonical schema may rely
  on (and the strict importer may re-check cheap ones).

### 3.4 Codelist association

493 fields carry `codeList: {value: {id, type: "flat"|…}, severity}`; only **1**
field has subtype-dependent codelists (`jq '[.fields[] | select(.codeList.constraints)] | length'` → 1),
so codelist-per-column is effectively static.

### 3.5 Preset values

364 fields have `presetValue` — fixed technical content (`@listName` names,
`@schemeName` names, `{NOW}` for dispatch date, `LocalLegalBasis`). These need
no columns; the importer should verify them and otherwise ignore. *(inferred)*

### 3.6 The privacy / "unpublished field" mechanism (BT-195…BT-198)

61 fields have a `privacy` block, e.g. for `BT-09(b)-Procedure`:
`{code, unpublishedFieldId: "BT-195(BT-09)-Procedure", reasonCodeFieldId:
"BT-197(…)", reasonDescriptionFieldId: "BT-196(…)", publicationDateFieldId:
"BT-198(…)"}`. Meaning: the value of a publishable field may be withheld; the
notice then carries an `efac:FieldsPrivacy` block (the `…Unpublish` nodes in the
tree) stating *which* field is unpublished (BT-195), *why* (BT-197 code, BT-196
text) and *until when* (BT-198 date). The withheld value appears in a later
republication. tender-db needs a generic `withheld_field` satellite table
(entity, field id, reason code, reason text, available-from date) rather than
4 extra columns × 61 fields. *(recommendation)*

---

## 4. Notice types: the 51 subtypes

`notice-types/notice-types.json` → `noticeSubTypes`: **51 subtypes** = numeric
"1"–"40" (the Implementing-Regulation forms) + `CEI, E1–E6, T01, T02, X01, X02`.
Each has `subTypeId`, `type` (form name, e.g. `cn-standard`), `formType`
(lifecycle phase), `documentType` (root schema), `legalBasis` (CELEX id).

Taxonomy (subtype → phase → root):

| formType | subtypes | meaning | documentType |
|---|---|---|---|
| `planning` | 1–9 (PIN buyer/only/reduced-time-limit), E2, T01 | prior information | PIN |
| `competition` | 10–24 (PIN-CfC, qualification system 15, CN, subcontract 22, design contest 23–24), CEI, E3 | call for competition | PIN (10–14, CEI) / CN (15–24, E3) |
| `dir-awa-pre` | 25–28 (VEAT) | voluntary ex-ante transparency | CAN |
| `result` | 29–37, E4, T02 | award results | CAN |
| `cont-modif` | 38–40, E6 | contract modification | CAN |
| `consultation` | E1 (preliminary market consultation) | pre-planning | PIN |
| `completion` | E5 (contract completion) | post-execution | CAN |
| `bri` | X01, X02 | EEIG / European-company registration notices | BRIN |

The triplicated numeric forms differ only by legal basis (directives
2014/24/EU, 2014/25/EU, 2009/81/EC, 2014/23/EU). `E*` subtypes are the
**voluntary below-threshold / extra forms** with `legalBasis: "other"` (E1–E6
added in SDK 1.13 per the TED FAQ; verified absent in ≤1.10:
`jq '[.noticeSubTypes[].subTypeId]'` on 1.10.0 shows 45 subtypes). T01/T02 are
passenger-transport notices (Reg. 1370/2007); X01/X02 are business-registration
notices with a substantially different body (ND-BusinessParty subtree); CEI is
the EU-institutions call for expression of interest (legal basis 32024R2509).

**How subtype determines content.** Two complementary artifacts:

1. `fields.json` `forbidden`/`mandatory` constraints keyed by subtype (§3.3) —
   the machine-checkable truth, enforced by Schematron at TED.
2. `notice-types/<subTypeId>.json` (51 files) — the *form definition*: a
   `metadata` list (notice-level readonly fields) plus a `content` tree of
   groups (`displayType: SECTION|GROUP`, with `nodeId`, `_repeatable`,
   `_identifierFieldId`) and field entries (TEXTBOX/COMBOBOX/…, `readOnly`,
   `hidden`). It drives rendering (eNotices2 UI), not validation.

The notice's own subtype is `OPP-070-notice`
(`efac:NoticeSubType/cbc:SubTypeCode`); BT-02 (notice-type code, `notice-type`
codelist, values like `cn-standard`) and BT-03 (`form-type` codelist) plus the
legal basis BT-01 restate the classification inside the notice.

For tender-db the subtype matters mainly as (a) the driver of which canonical
updates a notice can cause (planning vs competition vs result vs modification),
and (b) an importer sanity check — content outside the subtype's field set means
quarantine. It should **not** produce per-subtype tables. *(recommendation)*

---

## 5. Codelists

`codelists/` contains **275 `.gc` files** + `codelists.json` index (274 entries;
`jq '.codelists | length'`). Format: OASIS genericode 1.0
(<https://docs.ted.europa.eu/eforms/latest/codelists/index.html>).

- Each `.gc` file: `Identification` (ShortName, `Version` — e.g. `notice-type`
  is version `20250924-0`, `cpv` is `2008`, `nuts` is `2024`; `CanonicalUri`
  pointing at `publications.europa.eu/resource/authority/...`; Agency = OP) and
  a `ColumnSet`+`SimpleCodeList` with a `code` column plus a label column per EU
  language (`eng_label`, `deu_label`, … 24 languages).
- **Source of truth is EU Vocabularies** (authority tables at
  <https://op.europa.eu/en/web/eu-vocabularies/e-procurement/tables>); the SDK
  ships filtered copies (only eForms-relevant codes) and resyncs each SDK
  release ("Synchronisation of the code lists with their latest versions on EU
  vocabularies", SDK 1.15.0 CHANGELOG).
- **Tailored lists**: 188 of 274 entries declare a `parentId`
  (`jq '[.codelists[] | select(.parentId)] | length'`); filename pattern
  `parent_child.gc` (e.g. `country_eea-country.gc`,
  `main-activity` → `authority-activity`). The parent is also declared inside
  the file via `LongName Identifier="eFormsParentId"`.
- **Dynamic vs static**: within one SDK release everything is frozen; there is
  no runtime-updated codelist. The "dynamic" ones in practice are the big EU
  Vocabularies tables that keep evolving upstream and get re-synced per release
  (NUTS, CPV, currency, country, language) — i.e. codelist *contents differ
  across SDK versions*. The docs do not define any out-of-band update channel
  *(verified absence in docs; flagged by the docs-research pass)*.
- Hierarchical lists: CPV and NUTS encode hierarchy by code structure
  (CPV digits, NUTS prefix nesting), not by parent columns.

**Storage/validation recommendation** *(inferred)*: one `codelist` +
`codelist_value` pair of tables (list id, SDK version or list `Version`, code,
optional English label; other languages only if the dashboard needs them), FK
from canonical columns to `codelist_value`. Because contents vary per SDK
version, validate a notice against the codelist versions of *its* SDK version
(union of all accepted versions is the pragmatic FK target; strict per-version
validation can live in the importer). Do **not** generate one table per list
(275 tables): a single values table with `(list_id, code)` PK is idiomatic and
sufficient, since only 1 field has version-dependent list binding.

---

## 6. Identifier systems (critical for the canonical layer)

Source: <https://docs.ted.europa.eu/eforms/latest/schema/identifiers.html>
("Identifiers, References and Pointless Business Terms") + fields.json patterns.

### 6.1 Notice-scoped technical identifiers

All `id`-type fields with an `idScheme` follow `^<PREFIX>-[0-9]{4}$`
(`jq '[.fields[] | select(.type=="id" and .idScheme) | {id, idScheme, pattern: .pattern.value}]'`):

| scheme | field | entity |
|---|---|---|
| `LOT-XXXX` | BT-137-Lot | Lot |
| `GLO-XXXX` | BT-137-LotsGroup | LotsGroup |
| `PAR-XXXX` | BT-137-Part | Part |
| `ORG-XXXX` | OPT-200-Organization-Company | Organization |
| `TPO-XXXX` | OPT-201-Organization-TouchPoint | Touchpoint |
| `UBO-XXXX` | OPT-202-UBO | UBO |
| `TPA-XXXX` | OPT-210-Tenderer | TenderingParty |
| `TEN-XXXX` | OPT-321-Tender | Tender (Bid) |
| `CON-XXXX` | OPT-316-Contract | SettledContract |
| `RES-XXXX` | OPT-322-LotResult | LotResult |
| `REV-…` | BT-804-Review (no pattern) | Review |

These exist so `id-ref` fields can point at them *within the same notice*
(`@schemeName` names the target scheme). **Stability across notices differs by
kind** *(inferred from the reference mechanisms, medium-high confidence)*:

- `LOT-XXXX` / `PAR-XXXX` / `GLO-XXXX` ("section identifiers", BT-137 "Purpose
  Lot Identifier") are **procedure-scoped in practice**: change notices
  (BT-13716) and modification notices (BT-1501(p)) reference sections *of a
  previous notice* by exactly these ids, and result notices reference the lots
  competed in earlier notices. The OCDS eForms profile likewise assumes lot ids
  are stable per procedure. No docs sentence states this outright — treat as
  strong convention enforced by cross-notice reference rules, and verify
  empirically on TED data (open question).
- `ORG-/TPO-/UBO-/TPA-/TEN-/CON-/RES-XXXX` are **notice-local**. Nothing forces
  `ORG-0001` in notice v2 to be the same organization as in v1. Cross-notice
  organization identity must come from content (BT-501 registration id, VAT,
  name) — exactly tender-db's OrganizationMention→Organization design.
  For results entities the *business* ids (BT-3201 tender id, BT-150 contract
  id, both buyer-assigned, "should be unique within the procedure") are the
  cross-notice keys, not TEN-/CON- numbers.

### 6.2 Notice and procedure identity

- **BT-04 Procedure Identifier**: UUID v4, assigned by OP/eSender, constant for
  the procedure — *the* natural grouping key for tender-db's Tender entity
  (TED-sourced). X01/X02 (BRIN) notices have no procedure.
- **BT-701 Notice Identifier**: UUID v4 per notice; **BT-757 Notice Version**:
  2-digit counter starting `01`. A published *logical* notice is identified by
  `UUID-vv`. Corrections republish the same BT-701 with higher BT-757
  *(and/or a change notice with its own identity — see open question)*.
- **BT-758 Change Notice Version Identifier** (in change notices) and
  **BT-1501(n)** (in modification notices) reference the changed/modified
  previous notice as `UUID-vv` (scheme `notice-id-ref`) or by publication
  number.
- **OPP-010 Notice Publication Number**: assigned by OP at publication,
  `NNNNNNNN-YYYY` (8 digits in examples/practice; the docs' identifier table
  says 10 X's — width not firmly specified). **OPP-011** OJ S issue
  (`123/2023`, scheme `ojs-id`), **OPP-012** publication date. These are the
  public citation ids ("2023/S 123-…") and TED-API keys.
- **BT-125(i)/BT-1251** link a competition notice back to the planning notice
  (part) it follows — the planning→competition chain (per-lot, references a
  previous notice['s part]).

### 6.3 Which identifier does what for tender-db

| purpose | identifier |
|---|---|
| group notices into a Tender (procedure) | BT-04 |
| identify a Notice logically | BT-701 + BT-757 (also keep OPP-010) |
| identify a Lot inside a Tender | BT-04 + LOT-XXXX (verify stability) |
| Organization resolution evidence | BT-501/BT-502…, never ORG-XXXX |
| Bid/Contract continuity across CAN + modification notices | BT-3201 / BT-150 (buyer-assigned), CON-ref via BT-1501(c) within the modification notice |
| section addressing in change notices | BT-13716 values = section ids incl. `LOT-0001`-style and `PROCEDURE` |

### 6.4 "Pointless business terms"

The identifiers docs page lists Annex BTs the SDK deliberately does not
implement because UBL structure already conveys them: BT-53, BT-557, BT-724,
BT-778, BT-5561, and the BT-137 reference family **BT-1371–BT-1374,
BT-1376–BT-1379, BT-13710–BT-13712, BT-13715, BT-13717–BT-13722**. ADR-0002's
completeness test must whitelist exactly this documented set.

---

## 7. SDK versioning and what a multi-version importer faces

### 7.1 Versions in the wild

Git tags (fetched into the VPS clone; dates via `git log -1 --format=%cs <tag>`):

| minor | released | fields | nodes | distinct btIds |
|---|---|---|---|---|
| 0.x | 2021 – pre-production | | | |
| 1.0.0 | 2022-08-05 | 708 | 211 | 321 |
| 1.1.0 | 2022-09-15 | | | |
| 1.2.0 | 2022-10-07 | | | |
| 1.3.0 | 2022-11-03 | 741 | 236 | 337 |
| 1.4.0 | 2022-11-25 | | | |
| 1.5.0 | 2022-12-16 | 743 | 248 | 338 |
| 1.6.0 | 2023-03-06 | | | |
| 1.7.0 | 2023-05-11 | 748 | 245 | 339 |
| 1.8.0 | 2023-07-26 | | | |
| 1.9.0 | 2023-10-09 | 1224 | 286 | 339 |
| 1.10.0 | 2023-12-01 | 1226 | 286 | 340 |
| 1.11.0 | 2024-04-22 | 1229 | 286 | 341 |
| 1.12.0 | 2024-07-18 | 1234 | 291 | 343 |
| 1.13.0 | 2024-11-28 | 1256 | 307 | 356 |
| 1.14.0 | 2025-12-02 | 1256 | 307 | 357 |
| 1.15.0 | 2026-07-15 | 1256 | 323 | 357 |
| 2.0.0-alpha.2 | 2026-03-26 | (pre-release) | | |

(Counts: `curl raw.githubusercontent.com/OP-TED/eforms-sdk/<tag>/fields/fields.json | jq '{fields: (.fields|length), nodes: (.xmlStructure|length), btIds: ([.fields[].btId]|unique|length)}'`,
run on the VPS.) The 1.8→1.9 jump (748→1224 fields) is the introduction of
attributes-as-fields (0 → 466 `attributeName` fields), not new content.

TED accepted eForms notices from **14 Nov 2022** (eNotices2/TED API go-live; the
first eForms notices appear in OJ S issues 218/220 of late Oct/early Nov 2022);
eForms became mandatory for above-threshold notices on **25 Oct 2023**. So the
archive spans SDK **1.1 … 1.15** customization ids. Several minors are accepted
concurrently: each version has a ~12-month (extendable) lifespan
(<https://docs.ted.europa.eu/eforms-common/active-versions/index.html> —
currently 1.12, 1.13, 1.14 active; the live set is queryable via the TED API
`version-range` operation).

### 7.2 How a notice declares its version

`cbc:CustomizationID` = `eforms-sdk-<major>.<minor>` (revision deliberately
omitted), e.g. `eforms-sdk-1.7`
(<https://docs.ted.europa.eu/eforms-common/versioning/index.html>). Consumers
should apply the latest revision of that minor. The `VERSION` file at the repo
root of a checkout has the authoritative SDK version for that tree (in tagged
releases; the develop branch has an unexpanded `${project.version}` — use
`fields.json .sdkVersion`).

### 7.3 What changes between minors (empirical)

Field-id set difference 1.0.0 → 1.15.0 (`LC_ALL=C comm` on sorted id lists):
**25 field ids removed**, **573 added** (466 of the additions are attribute
fields). Removals are real: e.g. `BT-541-Lot`/`-LotsGroup` and its privacy
companions (award-criterion number restructure), `BT-747/748/749/752-Lot`
(selection-criteria restructure into repeatable `ND-SelectionCriteria`),
`OPT-050/090/091/092/150/999`. Nodes get added and *re-arranged*: 1.15.0
"creation of new node definitions and update to existing nodes and field paths"
for BT-15, BT-615, BT-10, … (CHANGELOG.md), i.e. **xpaths for the same BT can
change between minors**. Types can change (`OPT-156-LotResult` became
`integer` in 1.15). Rules and codelists change every minor. `notice-types.json`
gains subtypes (E1–E6 in 1.13).

**Consequences for the importer** *(inferred)*:

1. Parse per-notice against the fields.json of the notice's declared minor
   (bundle all SDK versions' metadata; they are small). A single hand-written
   parser targeting the union of xpaths is viable but must know per-version
   xpath variants for the handful of restructured BTs.
2. The ADR-0002 completeness checklist must be **per SDK version** (all
   versions' field ids must map, including removed ones like BT-541 that exist
   in 2022–2023 archive notices).
3. Quarantine triggers must key on (SDK version, unmapped xpath), since an
   unknown xpath in a 1.15 notice may be a known one in 1.13 terms.
4. SDK 2.0 (alphas exist; UBL 2.4/breaking changes) will eventually need a
   second parser generation — architecture should keep version-specific mapping
   tables data-driven.

---

## 8. Extension mechanism / national profiles

- eForms itself demonstrates the mechanism: arbitrary content under
  `ext:UBLExtensions/…/ext:ExtensionContent`, identified by
  `cbc:CustomizationID`. National profiles (eForms-DE etc.) are **downstream
  customizations of the SDK**: they subset/constrain fields, add national
  Schematron layers, and use their own CustomizationID (e.g. an
  `eforms-de-<version>` string) and, where needed, their own extension
  namespaces riding the same UBLExtensions point. The TED docs explicitly
  disclaim any SDK-level support: "National specificities … are outside OP's
  remit"; tailoring "would need to be applied locally"
  (<https://docs.ted.europa.eu/eforms-common/FAQ/index.html>).
- Practical import consequence *(inferred)*: a notice fetched from a national
  portal may carry (a) a non-TED CustomizationID, (b) additional national
  fields inside the extension block, (c) stricter cardinalities. The strict
  parser must map or explicitly-ignore national extension elements per source
  profile; the eForms-DE specifics are covered by a separate research task.

---

## 9. Prior art: OCDS, and TED's own downstream models

(Condensed from the dedicated prior-art research pass; URLs cited there were
fetched and verified 2026-07-19.)

### 9.1 OCDS core model

<https://standard.open-contracting.org/latest/en/>. OCDS models one
*contracting process* (id: `ocid` = registered prefix + publisher-local id) as a
sequence of **immutable releases** (per publication event, tagged `planning`,
`tender`, `award`, `contract`, `*Amendment`, …) that merge into a
**compiledRelease** (current state) and a **versionedRelease** (full field
history) — <https://standard.open-contracting.org/latest/en/schema/merging/>.
Merge semantics: last-write-wins per field; `null` deletes; arrays of objects
merge **by stable item `id`** (else `wholeListMerge` replaces the array).
Release sections: `parties[]` (each org once, with `roles[]`; referenced
elsewhere via `{id, name}` OrganizationReference), `planning`, `tender`,
`awards[]`, `contracts[]` (+ per-contract `implementation`). Lots are an
extension: flat `tender.lots[]` plus `relatedLot(s)` pointer fields on items,
documents, awards — not containers
(<https://extensions.open-contracting.org/en/extensions/lots/master/>).

The official **eForms→OCDS profile**
(<https://standard.open-contracting.org/profiles/eforms/latest/en/>) is a
field-by-field crosswalk that independently confirms the §2 entity reading:
`ND-Lot` → `tender.lots[]`, **`ND-LotResult` → `awards[]`**,
**`ND-SettledContract` → `contracts[]`**, **`ND-LotTender` → bids extension**
(`bids.details[]`), `ND-TenderingParty`/`ND-Organization` → `parties[]`, UBOs →
`parties[].beneficialOwners[]`, BT-140/141 change info → `Amendment` objects.
It also documents the identity traps: planning and contracting processes get
separate ocids; **each competition round of a framework/DPS is a separate OCDS
process while eForms keeps one procedure**; party ids must be *made* consistent
across notices by the converter (OCDS assumes the implementer maintains an org
register — the ORG-XXXX ids alone are insufficient).

Relational-storage lessons: OCP itself stores OCDS as JSON in Postgres and
compiles (Kingfisher Process); every SQL-flattening tool converges on
one-table-per-array with synthetic keys (Flatterer `_link` columns, Kingfisher
Summarize `*_summary` tables) to avoid array-multiplication double counting.

**Adopt** *(inferred)*: append-only publications + deterministic merge into
current state (matches ADR-0001 exactly); stable-id-per-array-item as the merge
key (for tender-db: procedure-scoped LOT ids, buyer-assigned contract/tender
ids, resolved org ids); parties-with-roles as one table + role link tables;
flat awards/contracts linked to lots by FK rather than nesting.
**Avoid**: OCDS's nested-JSON physical shape; its ocid-splitting of frameworks
(tender-db keeps eForms' one-procedure view and can expose rounds as a derived
concept); field-level versionedRelease (tender-db's row-validity versioning is
the SQL-native equivalent).

### 9.2 TED's own models

TED's canonical reuse format **is the eForms UBL XML itself** — bulk packages
(daily/monthly at `ted.europa.eu/packages/...`) and the TED Search API deliver
notice XML (eForms notices named `NNNNNNNN_YYYY.xml`, legacy TED-schema notices
with 6-digit names in the same archives — a pre-eForms format tender-db will
meet the moment backfill goes past 2022–2024). There is **no official EU
relational model**. The EU's structured offering is RDF: the **eProcurement
Ontology (ePO)** (<https://docs.ted.europa.eu/EPO/latest/index.html>, v5.2.0)
with the TED Open Data Service / CELLAR SPARQL endpoint, produced by the
TED-SWS RML mapping pipeline (<https://github.com/OP-TED/ted-rdf-conversion-pipeline>).
Useful as a semantic cross-check of entity readings; not a storage model to
copy. ("TED-XML" refers to the *legacy* pre-eForms schema, not an internal
eForms mapping.)

---

## 10. Cross-check of `~/Downloads/ted-non-exhaustive-overview.md`

The overview is directionally useful but contains invented specifics. Errors a
schema designer must not inherit (each checked against fields.json 1.15.0 with
`jq -r '[.fields[].btId] | unique'` and field-name lookups):

1. **"~500 BTs"** (also in our own CONTEXT.md!): wrong for the SDK. fields.json
   has **357 distinct btIds** = 287 `BT-*` + 33 `OPP-*` + 37 `OPT-*`, expanded
   into 1256 context-specific fields. No official "~500" figure exists in the
   docs; the Regulation Annex has on the order of ~270 BTs *(docs list
   BT-01…BT-815 sparsely; exact Annex count unverified)*.
2. **BT-13712 "Communication Lot Identifier", BT-13718, BT-13719** — do **not
   exist** in fields.json. They are in the documented "pointless business
   terms" list (§6.4): the doc presents deliberately-unimplemented Annex BTs as
   fields to model, and even builds its XML⇔DB example table on BT-13712.
3. **BT-3201 mapped to `/*/cac:Tender/cbc:ID`** — the xpath is invented; there
   is no `cac:Tender` element. BT-3201-Tender lives at
   `efac:NoticeResult/efac:LotTender/cbc:ID` inside the UBLExtension.
4. **BT-750 as "contract value" / "is_subcontractor (BT-750 INDICATOR)"** —
   BT-750 is "Selection Criteria Description" (text-multilingual). Contract
   values are BT-720 (tender value) / BT-735 etc.; subcontracting indicators
   are the BT-773 family. Similarly its `Tenders.price REAL -- BT-58` is wrong:
   BT-58 is "Renewal Maximum" (integer).
5. **BT-89, BT-849, BT-850, BT-61 do not exist** as btIds (claimed as
   "Justification", "Contract ID", "Contract Value", price fields).
   BT-800/BT-801 exist but mean "Deadline Receipt Answers" / "Non Disclosure
   Agreement", not the "Change Notice" semantics the doc implies; the real
   modification fields are BT-200/201/202 and BT-1501.
6. **BT-14 as "Variants"** — BT-14 is "Documents Restricted". Variants is
   BT-63. **BT-17 as CODE "Submission Electronic"** is right by luck (it is a
   code, `permission` list), but its claimed grouping is fabricated.
7. **Business Groups (BG-…) as an SDK structure** — fields.json contains **zero**
   BG references (`grep -c '"BG-' fields/fields.json` → 0). BGs exist only in
   the Regulation Annex tables. The doc's specific BG assignments (BG-1 Notice,
   BG-100 Communication, BG-102 Submission Terms, BG-137 Lots, BG-709 Second
   Stage, "BG-800+ Change") are part invention, part Annex memory; do not
   organize anything around them. Node/entity structure (§2) is the real
   grouping.
8. **BT-98 as DURATION "P30D"** — eForms durations are *measures*
   (value + `@unitCode` from `duration-unit`), never ISO 8601 period strings.
9. **BT-702 xpath "`/cac:Language` or `<efbc:LanguageCode>`"** — invented;
   BT-702(a) is `/*/cbc:NoticeLanguageCode`.
10. **"SDK Version 1.0" dated 2019** — SDK 1.0.0 is 2022-08-05; 2019 is the
    Regulation. Its timeline ("2024 consolidated procurement directive",
    "migration to new procedure types") is invented.
11. Schema-sketch errors that would corrupt a design if copied: `Lots.notice_id`
    as if lots were notice-owned children with globally-unique `LOT-0001` PKs
    (they are procedure-scoped labels, unique only within a notice/procedure);
    `Organizations.org_id TEXT PRIMARY KEY -- 'ORG-1234'` (notice-local id used
    as a global PK — precisely the mistake tender-db's OrganizationMention
    design exists to avoid); TenderingParties modeled without the consortium
    (member/leader/subcontractor) structure; UBOs given an `owner_org_id` FK
    "to the actual owner Organization" (UBOs are natural persons, not
    Organizations).

What it gets *right* (worth keeping): the general normalisation direction
(codelist tables, multilingual satellite table, separate address/contact
storage), BT-04/BT-701 as UUIDs, the ORG/TPO/UBO/TPA/CON prefix inventory, and
the existence of the pointless-BT concept.

---

## 11. Implications for tender-db

**Canonical entity set.** The SDK's own entity graph maps cleanly onto the
CONTEXT.md vocabulary and yields the canonical table backbone:
Tender(=Procedure, keyed by BT-04) — Lot / LotsGroup / Part — LotResult —
Bid(=LotTender) — BiddingParty(=TenderingParty, with member and subcontractor
link tables) — Contract(=SettledContract) — OrganizationMention(=per-notice
ORG) → Organization — Touchpoint — UBO — Review — plus notice-level
Change/Modification records. LotsGroup and Part must be first-class (not
folded into Lot): Parts appear only in planning notices, LotsGroups carry award
criteria and framework values, and group↔lot membership is M:N
(GroupComposition).

**Completeness accounting (ADR-0002).** Denominator per SDK version, not one
global list. Attribute fields (480) + OPA fields (3) map "via parent field";
preset-value plumbing maps "verified constant"; the documented pointless-BT
list is the only legitimate "not in SDK" exclusion. The checklist walk is
trivially mechanical: every `fields[].id` → {column | satellite table | via
parent | deliberate exclusion}.

**Importer (ADR-0004).**
- Parse against the notice's `cbc:CustomizationID` minor; bundle every SDK
  minor's `fields.json`/`codelists` (from git tags) as import metadata.
- Notice identity: (BT-701, BT-757); procedure grouping: BT-04; keep OPP-010/011
  as public citation keys. Change notices reference their predecessor via
  BT-758; modification notices via BT-1501(n).
- Section-level change tracking (ADR-0001 versioning) gets first-class source
  data: change notices *tell you which sections changed* (BT-13716) — use it
  to scope canonical-version diffs, but never trust it exclusively; diff
  content too *(recommendation)*.
- The strict "every element consumed" rule needs an explicit-ignore list seeded
  with: preset-value attributes, `…Unpublish` privacy plumbing (mapped to the
  withheld-fields table), signature/envelope elements if any, and per-source
  national extensions.

**Schema specifics.**
- Multilingual text: one satellite table per canonical layer (owner entity +
  field discriminator + language), driven by the 134 text-multilingual fields.
- Withheld values: generic `withheld_field` table (61 privacy-capable fields;
  BT-195/196/197/198 pattern) — republication later fills the real value, a
  canonical version event.
- Amounts: `(value, currency)` pairs; durations: `(value, unit)` pairs; dates:
  ISO 8601 TEXT **with original offset** (+ optional derived UTC).
- Codes: single `(list_id, code)` codelist-value table, FK'd from all code
  columns; import codelist contents per SDK version from the `.gc` files.
- Notice-local ids (ORG-, TEN-, TPA-, CON-, RES-, TPO-, UBO-) never become
  canonical keys — canonical rows get surrogate keys, with the notice-local id
  kept on the Notice-layer (parsed) rows for traceability. LOT-/GLO-/PAR- ids
  are the *only* section ids usable as natural keys within a procedure, pending
  empirical confirmation of their stability.
- Organization resolution evidence: BT-501 (company registration id; also
  `BT-501-Business-European`/`-National` on BRIN notices), plus name/address —
  matching exactly as CONTEXT.md prescribes (exact official identifiers only).

**Scope warnings.** X01/X02 (BRIN) notices are not procurement procedures (no
BT-04): model as a separate small satellite or explicitly out-of-scope
(user decision). Legacy pre-eForms TED-XML notices (the entire archive before
~Nov 2022 and most of it until Oct 2023) are a different format needing their
own importer and BT mapping — this research covers eForms only.

## 12. Open questions

### Needs more research

1. **Empirical stability of LOT-XXXX across notices of one procedure** — check
   a sample of real TED procedures (CN → corrigendum → CAN chains) via the TED
   API on the VPS. Also: how corrections are actually published (new version of
   same BT-701 vs change-notice subtypes with own BT-701?) and whether TED bulk
   XML carries both versions.
2. **Framework/DPS round semantics on real data**: how CANs within a framework
   reference the original procedure (same BT-04?) and how often one LotResult →
   many contracts occurs; decides whether tender-db needs a "competition round"
   derived entity (the OCDS profile splits here).
3. **Legacy TED-XML (pre-eForms) format**: structure, mapping to the same
   canonical entities, and the BT coverage gap — required for backfill beyond
   late 2022; entirely unresearched.
4. **eForms-DE / national portal profiles**: CustomizationID values, extra
   fields, deviations (separate task, in progress elsewhere).
5. **TED API / bulk download mechanics**: exact envelope metadata (publication
   number ↔ BT-701 mapping in bulk packages), rate limits, historical coverage
   of eForms XML.
6. **SDK 2.0**: scope of breaking changes (UBL 2.4? field renames?) — track the
   2.0.0-alpha changelogs before freezing importer architecture.
7. **Exact Regulation-Annex BT count** (for documentation accuracy; ~270,
   unverified) and whether any Annex BTs beyond the pointless list are
   unimplemented.

### Needs a user decision

1. **Money representation**: TEXT decimal vs INTEGER minor units vs REAL for
   amounts (STRICT tables; SQL endpoint users will aggregate — REAL is lossy,
   INTEGER-cents complicates currencies without cents).
2. **BRIN notices (X01/X02)**: in or out of v1 scope? They satisfy "all TED
   notice types in scope" but are not procurement.
3. **Codelist labels**: English-only vs all 24 languages in the DB (24× size;
   dashboard/API language policy).
4. **Timezone policy for the API**: expose original-offset timestamps, UTC, or
   both.
5. **Reviews (REV) and E5 contract-completion data**: first-class canonical
   entities or notice-layer-only in v1? (Small field counts, but they extend
   the lifecycle past award.)
6. **Whether the canonical layer models Parts as Lots-with-kind or a separate
   table** (they are structurally parallel but semantically "planned lots";
   affects the public API shape promised around Tenders and Lots).
