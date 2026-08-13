# 195 — eForms UBL unclaimed residue: ~311 rows on CommonAggregate/CommonExtension paths

Status: in-progress
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

**2026-08-14 ~01:4x CEST (orchestrator) — both sdk-1.10 classes pinned to exact xpaths;
implementation spec.** (a) 81 rows: `…/TenderingTerms/…/EformsExtension/StrategicProcurement/
StrategicProcurementInformation/ProcurementDetails` — the CVD statistics block
(AssetCategoryCode, StrategicProcurementStatistics/StatisticsCode+Numeric). sdk-1.10 declares
these fields ONLY at LotResult (BT-723/OPT-155/OPT-156-LotResult); the 2025 dailies also
publish them forward-looking at the Lot. (b) 75 rows: `…/TenderingProcess/…/EformsExtension/
SelectionCriteria` — the SDK and the index declare lot SelectionCriteria under TenderingTerms;
these publishers put the identical block under TenderingProcess. Both are the
published-beyond-the-SDK class: claim the alternate locations empirically (map onto the same
BT/OPT ids — the notice-parsed layer stores the source's own terms), mirroring how
crates/ingest/src/eforms/index.rs already declares extension nodes (lot StrategicProcurement is
at index.rs:145; SelectionCriteria/TenderingTerms entries at :290+). Add fixtures from members
00054478_2025 (extracted, in scratchpad) and one TenderingProcess/SelectionCriteria member;
verify with diag139; reprocess drains ~156 of the 310. The remaining classes (sdk-1.12
EformsExtension 89, sdk-0.1 SubcontractTerms 10, shortlist 11, RealizedLocation 7,
ProcurementAdditionalType 6, AppealTerms 9) each need the same one-query pinning first.

**2026-08-14 ~02:2x CEST (orchestrator) — both sdk-1.10 classes implemented as ALIASES
grafts.** Two new entries in `crates/ingest/src/eforms/index.rs`: LotResult
`efac:StrategicProcurement` → lot TenderingTerms `efac:StrategicProcurement` (the CVD
statistics: BT-723 AssetCategoryCode + OPT-155/OPT-156 pairs now claimed at the lot,
gap-fill keeps BT-717/BT-735 exact — the mirror of the pre-existing Lot→LotResult graft),
and lot TenderingTerms `efac:SelectionCriteria` → lot TenderingProcess
`efac:SelectionCriteria` (BT-40/747/748/749/750/752/7531/7532 + the UBL- extras claimed at
the unenumerated mount; note the sampled publisher duplicates the block at BOTH mounts, so
the stored duplication is source content). Verified with diag139 on both members:
00054478_2025 (sdk-1.10 CAN, was `unclaimed …ProcurementDetails`) → 68 sections/354 values
with the vehicle statistics under ND-StrategicProcurementInformationLot sections;
00157944_2024 (sdk-1.8 CN, was `unclaimed …SelectionCriteria`) → 37 sections/170 values,
grafted copies hang off their LOT sections (no node graft — the alias loop rewrites fields
only, same semantic as the nested-LotTender graft). Fixtures committed
(`can-cvd-lot-00054478-2025.xml`, `cn-selc-tp-00157944-2024.xml`), corpus 15→17, test
`sibling_mounted_extension_blocks_are_claimed`. Ingest gate fully green. Next: deploy when
queue idle, reprocess reason=unclaimed-content (expect ~156 of 310 to drain — the 81+75
sdk-1.10/1.8 classes; the 2-row `…EformsExtension/StrategicProcurement` class on other
minors may also drain if those minors declare the lot StrategicProcurementInformation).
Then pin the remaining classes (sdk-1.12 EformsExtension 89, SubcontractTerms 10,
shortlist 11, RealizedLocation 7, ProcurementAdditionalType 6, AppealTerms 9).
