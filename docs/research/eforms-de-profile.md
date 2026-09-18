# eForms-DE and the DÖE feed: schema & importer requirements

Research, 2026-07-19. Goal: pin down what supporting eForms-DE and the
oeffentlichevergabe.de (DÖE/Bekanntmachungsservice) feed concretely requires
from our schema and importer. Follows up docs/research/german-portals.md;
feeds the ADR-0002 amendment (per-version/per-profile completeness
checklists) and ADR-0004 (per-profile definition of "fully mapped").

Method: hands-on on the VPS against the real DÖE monthly exports
(`/opt/tender-db/samples/oeffentlichevergabe/`, now 11 months from 2022-12 to
2026-07-18, ~1 GB), a full clone of SDK-DE
(`/opt/tender-db/sdk-de`, gitlab.opencode.de/OC000008125155/SDK-eforms-de,
HEAD = tag 1.14.4) diffed against the EU eForms SDK checked out at the
matching tag (`/opt/tender-db/eforms-sdk-1.14.2`), plus primary docs
(xeinkauf.de, Bekanntmachungsservice OpenAPI, eForms-DE spec 1.0.1 PDF).
Every load-bearing claim was verified hands-on unless marked otherwise.
Scan scripts and raw results live on the VPS (`/opt/tender-db/scan2.py`,
`scan2.json`, `diff_fields.py`).

---

## 1. SDK-DE mechanics (verified from the repo)

### Fork/lock model

SDK-DE is a patched fork of the EU eForms SDK with the identical layout
(`fields/`, `notice-types/`, `codelists/`, `schematrons/`, `schemas/`, …)
under an `sdk/` subdirectory. Each release's Readme states both the
implemented national standard and the EU SDK it is compatible with, e.g.
1.14.4: "implementiert den Standard eforms-DE 2.1.0 … vollständig kompatibel
mit dem SDK 1.14.2 der EU". `fields.json` self-identifies via
`"sdkVersion": "eforms-de-2.1.0"`.

Tag → version map (verified per tag from Readme + fields.json; the repo's
first commit is 2022-09-28 but **tags only exist from 1.12.0 on** — there are
no SDK-DE artifacts on this GitLab for eForms-DE 1.0/1.1/1.2):

| SDK-DE tags | eForms-DE | EU SDK base | DEX fields |
|---|---|---|---|
| 1.12.0–1.12.6 | 2.0(.0) | 1.12.x | no |
| 1.13.0–1.13.3 | 2.1(.0) | 1.13.x | no |
| 1.14.0–1.14.4 | 2.1.0 | 1.14.1/1.14.2 | **yes (from 1.14.0)** |

The official whole-history mapping is embedded in the Bekanntmachungsservice
OpenAPI description itself (verified, `/opt/tender-db/samples/…/openapi.json`):

> eForms-DE 2.1 → EU 1.14 *and* 1.13; 2.0 → 1.12; 1.2 → 1.10; 1.1 → 1.7;
> 1.0 → 1.5; eForms-EU 1.14/1.13/1.12/1.10/1.0 → themselves; **eForms-EU 0.1
> → 0.1**.

Note the service classifies the legacy `eforms-sdk-0.1` encoding as an
"eForms-EU" kind, not an eForms-DE version (see §3). Note also eForms-DE 2.1
maps to *two* EU bases: notices carry `cbc:ProfileID` (`eforms-sdk-1.13` or
`eforms-sdk-1.14`) to disambiguate — but ProfileID is only reliable from
eForms-DE 2.0 on and is sometimes absent even then (2026-06: 9,260 × 2.1
with ProfileID 1.13, 799 × 1.14, **1,858 × absent**; all eforms-de-1.x
notices lack it entirely). The importer needs its own DE→EU fallback table.

### fields.json delta vs EU 1.14.2 (verified via mechanical diff)

EU 1.14.2: 1256 fields, 307 nodes. SDK-DE 1.14.4: **1260 fields, 311 nodes.
Zero fields or nodes removed.** The delta:

**4 added fields:**

| Field id | BT | Type | Location |
|---|---|---|---|
| `BT-001-DEX-NoticeResult` | BT-001-DEX (Berichtseinheits-ID, 8 digits) | id | `/*/ext:UBLExtensions/…/defext:GermanEformsExtension/defext:ProcurementStatistics/defext:ReportingUnitId` |
| `BT-002-DEX-Lot` | BT-002-DEX (sustainability-criteria phase) | code (`sustainability-criteria-setting`) | under the Lot's `strategic-procurement` ProcurementAdditionalType, in a nested `defext:` extension |
| `BT-002-DEX-Lot-List` | BT-002-DEX | text | listName companion of the above |
| `OPT-002-notice-DET` | OPT-002-DET | id | `/*/cbc:ProfileID` — the EU-base declaration |

**4 added nodes:** `ND-ExtensionsRoot`, `ND-GermanExtension`,
`ND-GermanProcurementStatistics`, `ND-LotGermanProcurementStatistics`.

The DEX fields are allowed only on result-side subtypes (29–35, E4 per the
mandatory/forbidden constraints; forbidden on 4–28, 36–40, E3) and exist to
automate VergStatVO statistics reporting (go-live announced for H2 2026).
The extension XSD (`sdk/schemas/common/german-eforms-extension.xsd`) declares
`targetNamespace="german-eforms-extension"` — a **non-URL namespace string**;
elements: `GermanEformsExtension`, `ProcurementStatistics`,
`ReportingUnitId`, `SustainabilityCriteriaSetting`.

**Constraint tightening, not structure change:** 275 fields differ in
`mandatory`, 828 in `forbidden` (the BR-DE-* national rules), 11 in
`codeList` (BT-01(c)/BT-01-notice → german-legal-basis lists, BT-105 →
national procedure-type lists, BT-11 → buyer-legal-type-avv, BT-23/BT-531 →
contract-nature variants, BT-161-Currency). XPaths: **0 differences** —
every shared field sits at the identical location as in the EU SDK.

### Codelists added (verified by directory diff)

14 added `.gc` files: `german-legal-basis`, `german-legal-basis-eu`,
`german-legal-basis-nat`, `sustainability-criteria-setting`,
`buyer-legal-type_buyer-legal-type-avv`, three `contract-nature_*` variants,
`currency-notice-value`, and five tailored
`procurement-procedure-type_*-{uvgo,voba,vola,avv}_e{3,4}` lists. Key
national code values seen in the wild: legal-basis `vgv, vob-a-eu, sektvo,
konzvgv, vsvgv, vob-a-vs, uvgo, vob-a, vol-a, sgb-vi, svhv,
avv-bund-start-up`; procedure types like `de-open`, `de-comp-neg-wo-call`
(uvgo lists also reuse EU codes `open`, `restricted`, plus `oth-single`,
`oth-mult`); sustainability-criteria `leis-besch, eig-krit, zu-krit,
ausf-besch`. Three EU codelists are dropped (`procedure-type-e3/e4/e5`,
replaced by the tailored ones).

### Notice subtypes

EU SDK 1.14.2 defines 51 subtypes (1–40, CEI, E1–E6, T01/T02, X01/X02) —
the E-types are EU-standard "voluntary" below-threshold forms, not a German
invention. SDK-DE keeps **43**: it removes `1, 2, 3, CEI, E5, E6, X01, X02`
and keeps 4–40, T01/T02, and **E1–E4** (E1 pmc/PIN, E2 pin-only/PIN, E3
cn-standard/CN, E4 can-standard/CAN). So "national subtype" handling =
an open per-profile subtype domain; no DE-only subtype codes exist, but the
*valid set* is profile-specific.

---

## 2. Above-threshold eForms-DE notices in the wild (verified, samples)

Customization IDs actually present across 11 sampled months (full scan of
every XML, prefix-agnostic — an earlier `cbc:`-anchored scan misclassified
~40% of files; see §3 on serializers):

`eforms-de-1.0, 1.1, 1.2, 2.0, 2.1` · `eforms-sdk-0.1, 1.0, 1.10, 1.12,
1.13` — ten concurrent IDs, of which 4–6 are active in any given month.
The plain `eforms-sdk-1.x` stragglers are tiny (≤32/month): 1.10–1.13 are
almost exclusively T01/T02 transport notices (RegulatoryDomain 32007R1370),
`eforms-sdk-1.0` is a small stream of E2/E3 below-threshold notices (e.g.
vergabe.bremen.de) that matches the full-eForms Unterschwelle chapters of
the eForms-DE 1.0.1 spec PDF (§8.7/8.21/8.35 define E2/E3/E4-shaped forms
with proper eForms xpaths).

Structure of an `eforms-de-2.1` notice: standard eForms UBL, one
`efext:EformsExtension` with `efac:NoticeSubType`, `efac:Organizations`,
etc.; `cbc:CustomizationID eforms-de-2.1`; `cbc:ProfileID eforms-sdk-1.14`
(or 1.13); the `defext` namespace (`xmlns:defext="german-eforms-extension"`)
is declared in headers but **no notice in any sampled month contains a
`GermanEformsExtension` element — DEX fields are spec-only as of 2026-07**
(consistent with the VergStatVO go-live in H2 2026).

> **Superseded (2026-08-09 drift audit): DEX is live.** First wild
> `defext:GermanEformsExtension/defext:ProcurementStatistics/defext:ReportingUnitId`
> observed in the DÖE 2026-08-07 daily export (CAN, subtype 30, eforms-de-2.1,
> notice 74d0a833-27db-4ac1-9158-20845d3a21e0-01); days 07-22/07-29/08-03/08-05
> had zero. DÖE's SVS (Service Vergabestatistik) went operational and the
> extension is NOT stripped from the public export — resolving this doc's open
> question the observable way. Our importer already persists these fields (the
> SDK-DE 1.14 inventory carries them; decisions are kind-derived), verified by
> the 2026-08-08 pipeline run ingesting that notice with zero new quarantine.
> See docs/research/upstream-drift-2026-08.md.

German quirks inside
otherwise-EU content: national codes from the added codelists (e.g.
legal-basis `vob-a-eu`, buyer-legal-type `omu-bbeh`, exclusion-ground
`nati-ground`), `unused-id` placeholder IDs in
Fiscal/Environmental/EmploymentLegislationDocumentReference, and marker
notes like `#Besonders auch geeignet für:other-sme#` in `cbc:Note`.

---

## 3. The legacy `eforms-sdk-0.1` encoding (verified, samples + SDK history)

### What it is

`eforms-sdk-0.1` refers to the **EU's own provisional eForms SDK 0.1**
(tags 0.1.0/0.1.1 in the EU SDK repo, released as "provisional … interim
version of Pre-Award UBL 2.3" containing **only** `schemas/`,
`schematrons/`, `docs/`, `examples/` — no fields.json, no notice-types, no
codelists). The German dialect layered on it is a KoSIT/BeschA-era
below-threshold submission format predating eForms-DE 1.0: UBL 2.3
ContractNotice/ContractAwardNotice/PriorInformationNotice documents using
German codelist values (`de-open`, `de-comp-neg-wo-call`; RegulatoryDomain
`de-vob`, `de-vol`, `de-uvgo`, `de-hhr` or absent). No formal public spec of
this dialect was found on xeinkauf.de, bescha.bund.de, or evergabe-online.info
(the eForms-DE 1.0.1 spec's Unterschwelle chapters describe the *successor*
E-forms, not this) — the shape must be pinned empirically (moderate-high
confidence it stays as-is: it has been byte-stable in style across 2022-12 →
2026-07).

### Two distinct producer channels (verified, 2026-06: 8,832 vs 916 files)

| | "numeric" channel (~91%) | "uuid" channel (~9%) |
|---|---|---|
| Filename | `<numericId>-<v>.xml` (e.g. `25404836-1.xml`) | `<uuid>-<v>.xml` |
| Serializer | JAXB-style, single line, `ns2…ns9` prefixes | pretty-printed, `cbc:/cac:` prefixes |
| `cbc:ID` | numeric, **no schemeName** | notice UUID |
| `cbc:ContractFolderID` | **present but empty** | procedure UUID |
| `cbc:IssueDate` | absent | present |
| eSender block | none | `ServiceProviderParty` `ted-esen` = Beschaffungsamt des BMI |
| Origin (inferred) | platform imports (CallForTendersDocumentReference URIs point at evergabe.de, dtvp.de, subreport-elvis.de, vergabe.niedersachsen.de, … — the service.bund.de import ecosystem); moderate confidence | Vermittlungsdienst submissions (e.g. evergabe-online.de) |

### Field inventory (empirical, 800-file random sample of 2026-06)

362 distinct element paths total, but the effective vocabulary is small — a
notice typically has 15–40 leaves. Always present: `UBLVersionID` (2.3),
`CustomizationID`, `ID`, `VersionID`, `RequestedPublicationDate`,
`NoticeTypeCode` (`cn-standard`/`can-standard`/`pin-only` — **no
NoticeSubType anywhere**: 121,660/121,660 files in 2022-12 lack it),
`ContractingParty/Party/PartyName/Name`, `ProcurementProject`
(Name/Description, often CPV, RealizedLocation), exactly one
`ProcurementProjectLot` (`LOT-0000`) duplicating the project block. Common:
`TenderingProcess/ProcedureCode`, `TenderSubmissionDeadlinePeriod`,
`CallForTendersDocumentReference` (link to origin platform),
`RegulatoryDomain`. Rare (mostly uuid-channel): buyer address/contact,
selection criteria (in a real `efext:EformsExtension`), guarantees, payment
terms, `efac:Change` corrigendum blocks, CAN `TenderResult/AwardDate` and
occasionally `WinningParty`. Notably absent: organizations extension,
NoticeResult, **all award values** (no LegalMonetaryTotal anywhere in the
sample), OPP/OPT fields, multi-lot structures.

Data-quality quirks to expect: CPV codes truncated to division level
(`45`, `43` under `listName="cpv"`), postcodes embedded in CityName, empty
mandatory elements, `No ID` literals.

### Verdict

`eforms-sdk-0.1` is **not** a transitional format that is dying out: monthly
volume is 8.7k–13.6k from 2023 through 2026 with no downward trend
(2026-06: 9,748 = 42%), while the proper below-threshold E-types remain
marginal (E1–E4 combined ≤ 150/month, 2026-06: 138). The below-threshold
platform ecosystem simply has not migrated. Plan for it as a permanent
first-class profile.

---

## 4. Volume and mix over time (verified, full scans)

Notice versions per month by customization ID (top rows; full data in
`/opt/tender-db/scan2.json`):

| Month | total | sdk-0.1 | de-1.1 | de-1.2 | de-2.0 | de-2.1 | eu-sdk 1.x |
|---|---|---|---|---|---|---|---|
| 2022-12 | 121,947 | 121,660 | — | — | — | — | 287 |
| 2023-01 | 14,300 | 14,162 | — | — | — | — | 138 |
| 2023-06 | 19,411 | 19,261 | — | — | — | — | 150 |
| 2023-11 | 20,030 | 10,629 | 9,305 | — | — | — | 96 |
| 2024-01 | 20,889 | 9,581 | 11,243 | — | — | — | 65 |
| 2024-06 | 27,090 | 13,581 | 11,462 | 1,964 | — | — | 83 |
| 2024-11 | 24,020 | 11,981 | 4,261 | 6,527 | 1,193 | — | 58 |
| 2025-04 | 27,462 | 13,532 | 2,341 | 4,293 | 7,201 | 24 | 71 |
| 2025-11 | 24,505 | 11,233 | — | — | 4,654 | 8,572 | 46 |
| 2026-01 | 20,997 | 8,730 | — | — | 2,988 | 9,221 | 81 |
| 2026-06 | 23,398 | 9,748 | — | — | 1,651 | 11,917 | 82 |

Readings:

- The 2022-12 bucket is a launch backfill of the pre-eForms-DE world:
  99.8% sdk-0.1, and in that era sdk-0.1 also carried **above-threshold**
  notices (13,648 × 32014L0024 + 356 × L0025 + 162 × L0081). From 2023-11 on,
  sdk-0.1 is ≥96% national domains (de-vob ≈ 60%, absent ≈ 25%, de-vol,
  de-uvgo, de-hhr) — above-threshold moved to eForms-DE when it became
  binding (2023-10-25; eforms-de-1.1 appears exactly between the 2023-06 and
  2023-11 samples).
- eForms-DE version migrations are **slow and overlapping**: 1.1 and 1.2 ran
  concurrently for ~a year; 2.0 and 2.1 still both live in 2026-06. At least
  two eForms-DE versions plus sdk-0.1 are live at any time; during
  transitions, four.
- Above/below-threshold ratio is stable at roughly 55–60% eForms-DE
  (above + a sliver of E-types) vs 40–45% sdk-0.1 (below) since 2023-11.
- Corrigenda: version suffixes in 2026-06 sdk-0.1: `-1` 9,462, `-2` 260,
  `-3` 25, `-4` 1 — multi-version notices exist in every profile
  (eForms-DE uses `efac:Change` blocks; sdk-0.1 numeric channel bumps
  VersionID with `NoticeTypeCode listName="change"`).

---

## 5. DÖE vs TED for the same notice (verified, one pair)

Pair: DÖE `ebb72363-832d-4cea-8db6-04999414ea8c-01` (eforms-de-2.1,
2026-06) ↔ TED `373130-2026` (the ADR-0003 reference pair). Leaf-level
diff (path+value multisets): 247 leaves (DÖE) vs 261 (TED); **not
byte-identical, semantically near-identical, and the differences are
systematic**:

- `CustomizationID` rewritten: `eforms-de-2.1` → `eforms-sdk-1.13`;
  `ProfileID` `eforms-sdk-1.13` kept identical on both sides.
- **National code values are converted, with information loss toward TED**:
  buyer-legal-type `omu-bbeh` (DÖE) → `cga` (TED); exclusion-ground
  `nati-ground` → `exg-natl-bre-nat-law`. Same BT, different codelist value
  — the DÖE original is the more precise national classification.
- TED adds publication metadata DÖE lacks: `efac:Publication`
  (`GazetteID ojs-id 103/2026`, `NoticePublicationID 00373130-2026`,
  `PublicationDate`) plus an injected ORG-0002 for the eSender
  ("Datenservice Öffentlicher Einkauf …", role `ted-esen`).
- Everything else — all UUIDs, dates, amounts, texts — identical.

Consequence: for German above-threshold notices DÖE is the *authoritative
original* (richer national codes; and once DEX statistics fields go live in
H2 2026 they will exist **only** there — they are stripped/never forwarded
to TED), while TED is authoritative for OJS publication identity (gazette
number, OJS notice id). Merge precedence should be per-field-class, not
per-source (single pair verified; systematic pattern is high-confidence for
code conversion since it follows the eSender-Hub's published role, but more
pairs — ideally one with award data — should be spot-checked).

---

## Implications for tender-db

Concrete schema requirements:

1. **Notice provenance columns**: store `customization_id` (10 known values,
   open set), `profile_id` (nullable), `regulatory_domain` (open set:
   EU CELEX ids + `de-vob`, `de-vol`, `de-uvgo`, `de-hhr`, `other`, absent),
   and the resolved `(profile, eu_base_version)` pair the notice was mapped
   under. Resolution = ProfileID when present, else the OpenAPI mapping
   table, recorded per notice for checklist accounting.
2. **Notice subtype is a per-profile open domain** (EU 1–40/T/X/CEI/E1–E6;
   DE subset of 43) and **nullable**: every sdk-0.1 notice has none.
3. **One generic national-extension satellite pattern**: a
   `notice_national_fields`-style satellite keyed by Notice (and lot id where
   applicable) for DEX BTs — today exactly BT-001-DEX (8-digit reporting-unit
   id, notice-level) and BT-002-DEX (code, lot-level). Zero wild instances
   yet, so the shape can stay minimal, but the importer must recognise
   `german-eforms-extension`-namespace content the day it appears (H2 2026)
   rather than quarantining the whole feed.
4. **Codelists are (profile, list) scoped**: 14 German lists on top of the
   EU ones, and 11 BTs whose valid values differ by profile. Canonical-layer
   code columns must not assume the EU value set; store codes as-is and keep
   the profile-scoped list as validation metadata, since the *same BT* can
   arrive as `omu-bbeh` from DÖE and `cga` from TED for the same notice.
5. **Award values may be legitimately absent for an entire profile**
   (sdk-0.1 has none); anything keyed "mandatory for CAN" in EU terms is
   profile-conditional.

Concrete importer requirements:

6. **Namespace-aware XML handling, never prefix-matching**: the numeric
   channel serializes with `ns2…ns9` prefixes and a default namespace that
   isn't UBL's. (This bug class is real: our first scan misbucketed 40% of
   files.)
7. **Per-profile mapping dispatch on CustomizationID** with clean quarantine
   of unknown IDs (ADR-0004). Known set today: 5 × eforms-de, 5 × eforms-sdk.
8. **An sdk-0.1 mapping profile defined empirically**, not from any SDK:
   the 362-path inventory (VPS `/opt/tender-db/…`) is the checklist source.
   Its notice identity rules differ per channel: uuid-channel = UUID + real
   ContractFolderID (procedure id); numeric-channel = numeric id, **no
   procedure identifier at all** (empty ContractFolderID → no cross-source
   merge possible, by design of ADR-0003's strong-reference rule).
9. **Tolerate data-quality quirks without quarantine** where they are
   systematic: truncated CPVs, empty elements, `No ID`/`unused-id`
   placeholders — these are "legitimately dirty", not "unmapped".
   **CPV representation is normalised at the fold, not tolerated** (issue 394
   unit 2): the island's check-digit form (`45421146-9`), division-only codes
   (`50`) and glued multi-code strings fold to the bare 8-digit code, one per
   row, the spelling every other era publishes — named here so the next
   dialect cannot arrive unnormalised without a decision.
10. **Fetcher**: monthly/daily ZIPs as raw payloads; one file per notice
    *version*; both filename schemes (`uuid-NN.xml`, `numeric-N.xml`).

ADR-0002 checklist design (the amendment made concrete):

- Completeness is asserted per `(profile, version)` actually ingested:
  - `eforms-sdk-1.x` (TED + strays): EU SDK `fields.json` at that tag.
  - `eforms-de-2.0/2.1`: **SDK-DE** `fields.json` at the matching tag
    (superset of EU; the walk gains the 4 DEX/OPT fields and the constraint
    metadata; same tooling, different repo).
  - `eforms-de-1.0/1.1/1.2`: no SDK-DE artifact exists on GitLab; use the EU
    base fields.json (1.5/1.7/1.10) as the structural checklist + the
    eForms-DE spec PDFs for the national deltas (needs the spec ZIPs from
    projekte.kosit.org Maven; open question below).
  - `eforms-sdk-0.1`: no fields.json exists anywhere; the checklist is the
    **empirical path inventory** committed as a fixture, with every path
    mapped or documented-excluded. "Fully mapped" for this profile means
    "all 362 observed paths handled"; a *new* never-seen path in a future
    notice quarantines that notice and extends the inventory — that is the
    per-profile ADR-0004 strictness definition: absence of a field present
    in richer profiles is never a failure; presence of an unknown path is.

## Open questions

Needs research:

- Locate SDK/spec artifacts for eForms-DE 1.0/1.1/1.2 (projekte.kosit.org
  Maven registry `de/xeinkauf/eforms-de/<version>`; the 1.0.1 spec PDF is
  on xeinkauf.de) and decide the checklist source for those versions.
- A formal spec for the German sdk-0.1 dialect (codelists used, channel
  semantics). Nothing public found; possibly only internal BeschA
  documentation exists — worth one email to
  support@datenservice-oeffentlicher-einkauf.de, which could also answer
  whether the numeric channel is the service.bund.de import path (currently
  inferred from CallForTendersDocumentReference URLs).
- Verify the DÖE↔TED conversion pattern on more pairs, especially a CAN
  with award values, and confirm DEX stripping empirically once VergStatVO
  reporting goes live (H2 2026).
- Whether pre-2023-11 sdk-0.1 *above-threshold* notices (13.6k in 2022-12)
  have TED counterparts and how they'd merge (they predate the eSender-Hub
  UUID regime; numeric ids only → likely unmergeable).
- EU SDK 2.0 (alpha tags exist upstream) and the next eForms-DE major:
  watch for `ProfileID`/CustomizationID scheme changes.

Needs user decision:

- Whether sdk-0.1's numeric-channel notices (no procedure id, dirty CPVs,
  ~91% of below-threshold volume) go into the canonical Tender layer at
  launch or stay Notice-layer-only until organization/procedure identity
  is worked out.
- Merge precedence policy per field class (proposed here: DÖE wins for
  national code values and future DEX satellite data; TED wins for OJS
  publication identity; conflict on anything else = flag for review).
