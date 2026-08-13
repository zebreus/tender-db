# 195 — eForms UBL unclaimed residue: ~311 rows on CommonAggregate/CommonExtension paths

Status: needs-triage
Kind: parser gap investigation (eForms side of the 183 attribution)
Blocked by: —
Relates to: 183 (attribution pass), 144 (the value-era waves that drained the previous eForms tail), 188 (eforms-sdk-0.1 linkage, possibly same vintage)

## What (measured via /v1/sql, 2026-08-13)

Two eForms detail families remain in still-held `unclaimed-content`:

- 222 rows: `unclaimed element at /{…CommonAggregateComponents-2}…`
- 89 rows: `unclaimed element at /{…CommonExtensionComponents-2}…`

The 80-char truncation in the attribution readout hides the full paths — first step is a
deeper `/v1/sql` read grouping on a longer substr (or the full detail) to name the exact
elements and SDK versions, then decide fix-and-reclaim vs documented keep per element, the
issue-143/144 pattern. Bounded: 311 rows, reason-indexed.

## Comments

**2026-08-13 ~05:5x CEST (orchestrator) — element-level readout (post issue-194 drains, 310
rows).** Top classes, namespaces stripped: nested `UBLExtensions` under lot-level
`TenderingTerms` (81, sdk-1.10) and `TenderingProcess` (75, sdk-1.10); an
`EformsExtension` child under the root extension (89, sdk-1.12); `TenderingTerms/AppealTerms/
UBLExtensions` (9, sdk-1.0); `TenderingProcess/EconomicOperatorShortList/…` (11, sdk-1.7);
`TenderResult/SubcontractTerms/Amount` (10, sdk-0.1 — possibly issue 188's vintage);
`RealizedLocation/…` (7, sdk-1.7); `ProcurementAdditionalType/ProcurementTypeCode` (6,
sdk-1.8). Each class needs a claim-vs-documented-keep decision against its SDK's field
inventory (the 143/144 pattern); lot-level nested UBLExtensions look like one mechanism
covering ~165 rows.

**2026-08-14 ~00:3x CEST (orchestrator) — the big class is the Clean Vehicles Directive
block.** Extracted member 00054478_2025 (sdk-1.10, 2025-01 daily): the lot-level
`TenderingTerms/UBLExtensions/UBLExtension/ExtensionContent/EformsExtension/StrategicProcurement`
block carries `ApplicableLegalBasis listName="cvd-scope"`, `ProcurementCategoryCode
listName="cvd-contract-type"`, `AssetCategoryCode listName="vehicle-category"` — the strategic
procurement / CVD family (BT-717/BT-735/vehicle-category terms). These are REAL SDK fields; the
eForms walker evidently claims the ROOT-level EformsExtension but not lot-level extension blocks
under TenderingTerms/TenderingProcess. Next (implementation firing): check the vendored
sdk-1.10 fields.json for these xpaths — if present, the walker's extension descent is the gap
(fix-and-reclaim for ~165+ rows across the classes sharing the mechanism); if absent from the
vendored inventory, decide claim-vs-keep per the 143/144 pattern. The sdk-1.12 EformsExtension
class (89 rows) and AppealTerms/UBLExtensions (9, sdk-1.0) likely share the same mechanism.
