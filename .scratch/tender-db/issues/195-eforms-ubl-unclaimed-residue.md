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

**2026-08-14 ~03:1x CEST (orchestrator) — deployed (rev 342f11e), reclaim running; full-width
readout corrects the class attribution.** Grouping without the 80-char truncation shows the
two implemented classes span minors (the grafts are version-agnostic, so all should drain):
CVD ProcurementDetails = 43 sdk-1.12 + 32 sdk-1.13 + 6 sdk-1.10 (=81); TenderingProcess
SelectionCriteria = 37 sdk-1.10 + 27 sdk-1.8 + 11 sdk-1.13 (=75). The "sdk-1.12
EformsExtension 89" class dissolves into three now fully-named root-extension classes:
**84× sdk-1.7 `efbc:FrameworkMaximumAmount` directly under the root EformsExtension** (the
EXTRA table claims it only at the lot TenderingTerms extension — likely one EXTRA entry, but
check no minor declares a field at the root path first), 4× sdk-1.9 root-level
`efac:FieldsPrivacy` (withheld-field block at an unanchored position), 1× sdk-1.12
`NoticeSubType/efbc:SubTypeDescription`. Also newly visible below the old cut: 11× sdk-1.7
`TenderingProcess/EconomicOperatorShortList/cac:PreSelectedParty` (plain UBL), 10× sdk-0.1
`TenderResult/SubcontractTerms/cbc:Amount` (patch tables stay off 0.1 — inventory addition or
documented keep), 9× sdk-1.0 procedure-level `TenderingTerms/AppealTerms/ext:UBLExtensions`,
7× sdk-1.7 lot `RealizedLocation/Address/cbc:Description` (an EXTRA + procedure→lot alias
already exists — these rows may simply drain on this reprocess; verify), 6× sdk-1.8
`ProcurementAdditionalType/cbc:ProcurementTypeCode` (need the listName — the
eforms-contract-nature carve-out may not cover it), 5× sdk-1.7 lot
`ContractExtension/cbc:RenewalsIndicator`. Next firing: read the reclaim counts, then take
the 84-row FrameworkMaximumAmount class.

**2026-08-14 ~03:5x CEST (orchestrator) — reclaim complete: 156/310 drained, exactly the two
implemented classes.** Job 649: 68 packages, 156 reclaimed, 0 skipped; trailing fold (job
650) projected 156 notices → 132 tenders, 287 versions. Zero "stamped NO ledger rows"
journal warnings; /health/deep green. unclaimed-content now 206, fully mapped: 84× sdk-1.7
root `efbc:FrameworkMaximumAmount` (NEXT SLICE — check no minor declares a root-level field
there, then likely one EXTRA entry), 11× shortlist PreSelectedParty (1.7), 10× sdk-0.1
SubcontractTerms/Amount, 9× sdk-1.0 procedure AppealTerms/UBLExtensions, 7× sdk-1.7 lot
RealizedLocation/Address/Description (did NOT drain — the issue-18 EXTRA + procedure→lot
alias evidently doesn't reach sdk-1.7; investigate why), 6+3× ProcurementAdditionalType/
ProcurementTypeCode (1.8 procedure / 1.10 lot — pin the listName), 5× sdk-1.7 lot
ContractExtension/RenewalsIndicator, 4× sdk-1.9 root FieldsPrivacy, 2× sdk-1.12
StrategicProcurement under *AwardingCriterion* extension (a third mount), 2× sdk-1.12
doubly-nested ServiceProviderParty, plus 2× text and 2× r208 rows tracked elsewhere.

**2026-08-14 ~04:5x CEST (orchestrator) — slice 2 shipped: root-level FrameworkMaximumAmount
drained 84/84.** The class is the eForms-DE cross-dialect pattern: German eSenders declaring
plain EU minors publish `efbc:FrameworkMaximumAmount` directly under the root EformsExtension;
the vendored eforms-de-1.x inventory declares exactly this path (DE1-FrameworkMaximumAmount).
Claimed via gap-filled `insert_extra` in `build()` as UBL-FrameworkMaximumAmount (the id the
element already gets at its lot-TenderingTerms mount); eforms-de-1.x keeps its DE1 id. Fixture
`cn-fma-root-00660539-2023.xml` (sdk-1.7 DE CN, EUR 400000.00), corpus 17→18, assertion added
to `sibling_mounted_extension_blocks_are_claimed`. Deployed rev a12422f; reprocess job 651: 84
reclaimed, 0 still held; fold job 652: 84 notices → 75 tenders, 198 versions; journal clean.
unclaimed-content now ~122. Remaining classes: 11× shortlist PreSelectedParty (1.7), 10×
sdk-0.1 SubcontractTerms/Amount, 9× sdk-1.0 AppealTerms/UBLExtensions, 7× sdk-1.7 lot
RealizedLocation/Address/Description (why doesn't the issue-18 EXTRA+alias reach 1.7?), 6+3×
ProcurementAdditionalType/ProcurementTypeCode, 5× ContractExtension/RenewalsIndicator, 4×
root FieldsPrivacy (1.9), 2× StrategicProcurement under AwardingCriterion (1.12), 2× nested
ServiceProviderParty (1.12), + small text/r208 rows tracked elsewhere.

**2026-08-14 ~06:5x CEST (orchestrator) — three more classes implemented (not yet
deployed).** (1) Lot `ContractExtension/cbc:RenewalsIndicator` (5 rows, sdk-1.7 DE) — the
eForms-DE cross-dialect class again, claimed as UBL-RenewalsIndicator behind the
PLDR-style predicate-free-construct guard. (2) `RealizedLocation/Address/cbc:Description`
on *Parts* (7 rows, sdk-1.7 LV PINs) — root cause: ALIASES do not compose, so the
procedure-level UBL-AddressDescription EXTRA never reached Part via procedure→Lot→Part;
the entry is now also written at Lot level, which the Lot→Part/LotsGroup aliases mirror.
(3) Design-contest shortlist merged under TenderingProcess (11 rows, sdk-1.7 FR — the
publisher abuses BT-47 PreSelectedParty names to carry criteria text; stored as published
under BT-47-Lot): new ALIASES graft TenderingTerms→TenderingProcess EconomicOperatorShortList.
Fixtures ×3, corpus 18→21, gate green, pushed (commit "three more sibling-mount classes").
Deploy + reclaim next firing when the queue idles. Remaining after that: 10× sdk-0.1
SubcontractTerms/Amount, 9× sdk-1.0 AppealTerms/UBLExtensions, 6+3×
ProcurementAdditionalType listName, 4× root FieldsPrivacy (1.9), 2× AwardingCriterion
StrategicProcurement (1.12), 2× nested ServiceProviderParty (1.12), 1× SubTypeDescription.

**2026-08-14 ~03:1x CEST (orchestrator) — three-class slice deployed (rev 0a2acc7),
reclaim running.** Deploy also carries the store fresh-record-path benign-zero guard (see
issue 139 thread). Reprocess reason=unclaimed-content enqueued (job 1 of the new process,
60 packages) with trailing incremental project (job 2). Expected drain: ~23 rows (5×
RenewalsIndicator DE, 7× Part RealizedLocation Description, 11× shortlist TenderingProcess).
Counts next poll.

**2026-08-14 ~03:3x CEST (orchestrator) — three-class reclaim complete: 23/23 drained.**
Job "reprocess unclaimed-content" (rev 0a2acc7): 60 packages, 23 reclaimed, 101 still held,
34,543 already parsed, 0 policy-skipped. The 23 = exactly the three implemented classes
(5× lot RenewalsIndicator DE, 7× Part RealizedLocation Address Description, 11× design-
contest shortlist TenderingProcess). Residue now 101: 99 unclaimed-content + 2 rows whose
re-parse fails as missing-publication-id (issue-87 current-reason rewrite — new class,
needs a look). Trailing incremental project running. Next slice: pin the remaining classes
(10× sdk-0.1 SubcontractTerms/Amount, 9× sdk-1.0 AppealTerms/UBLExtensions, 6+3×
ProcurementAdditionalType listName, 4× root FieldsPrivacy, 2× AwardingCriterion
StrategicProcurement, 2× nested ServiceProviderParty, 1× SubTypeDescription, + the tail
below the old readout cut) and the 2 missing-publication-id rows.

**2026-08-14 ~04:0x CEST (orchestrator) — ledger audit: all six drained classes now have
dashboard entries.** Prompted by Lennart's check-in question, audited every resolved
quarantine population against the vendored ledger keys: the 195 waves had NO dashboard
rows. Added six entries (CVD 81, SelC 75 — its prose also carries the 2 AwardingCriterion-
TypeCode + 1 nested-UBLExtensions side-drains, whose suffixes collide with still-open
classes — root FMA 84, shortlist 11, Part address descriptions 7, DE RenewalsIndicator 5),
plus entries for 194's internal-ojs summaries, 181's CF trio, 180's COR class and the
1999 not-utf8 drain, and outstanding namings for 196 + the parked concession pair
(commit "ledger: document the missing resolution decisions"). Keys verified disjoint and
exact against the live DB. Renders after the next deploy.
