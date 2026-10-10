# 506 unit 1, code side — eForms money BTs from parser to fold to elections (2026-10-10)

Read-only code and fixture read (no cargo), spot-checked by the owner: `LOT_KINDS` (`store/src/read.rs:2411`),
the award-role `lot_of` precedent (`project.rs:4660-4675`), and the `can-cvd-legacy-00412845-2025.xml` lines.

## How a money value travels
- Parser: every SDK `amount` field is decided `Amounts` (`eforms/sdk.rs:213`) and stored in `notice_amounts` under its
  full SDK id (`eforms/parse.rs:366-373`).
- Sections: only repeatable SDK nodes open a section (`eforms/index.rs:871-877`), so BT-709 / BT-660 file under the
  `RES-xxxx` LotResult section, BT-118 / BT-1118 / BT-161 under `ND-Root`, BT-156 / BT-1561 under an anonymous
  `NoticeResultGroupFA#n` section.
- Fold: `amount_target` → `canonical_name(AMOUNTS, id)` (`project.rs:7144-7156`, stem match `6600-6605`); scope from
  `scope_of` (`project.rs:4424`, `5612-5617`), the nearest `Lot | LotsGroup | Part` ancestor. A LotResult has none, so
  ANYTHING MAPPED THROUGH PLAIN `AMOUNTS` FROM A LOT RESULT LANDS LOT-NULL.
- eForms-DE 1.x aliases only BT-27, BT-271, BT-161, BT-720 (`project.rs:1146-1156`, `1242-1243`).

## How the elections see an amount (`store/src/canonical.rs`)
- Head candidates: every Amount fact of the head version (`3176-3193`); a tender-scope figure equal to a head lot award
  is `Lot(None)` (`3179-3183`).
- Partners: every amount of every version plus lot awards (`add_version_figures`, `3590-3612`) — 492 decision (c):
  a lot-null amount equal to an award of the same version is a lot figure, any other lot-null amount a Procedure figure.
- One Procedure occurrence flips `lot_only` (`3505`), killing 492's framework-total exemption (`3783-3787`), and nulls
  `lot_keys`, killing the sibling-lot exemption (`3799-3808`).
- k≥3 corroboration: a different FIELD of the head with the same cents exempts (`3945-3949`) — a new field name makes
  new corroborations, reusing an existing one does not.

## Table

| BT | Meaning / scope | Notice-layer id | Fold today | Proposed + risk |
|---|---|---|---|---|
| BT-27 | Estimated value; Procedure/Lot/LotsGroup/Part | `BT-27-*` | `estimated_value` | keep |
| BT-271 | Framework maximum; Procedure/Lot/LotsGroup | `BT-271-*`, `UBL-FrameworkMaximumAmount` | `framework_maximum` | keep |
| BT-161 | Notice value; NoticeResult; forbidden when the lot is a framework (SDK 1.15) | `BT-161-NoticeResult` | `result_value`, tender scope | keep |
| BT-720 | Tender value; LotTender | `BT-720-Tender` | bids → `lot_results.awarded` | keep |
| **BT-709** | Framework maximum per lot result | `BT-709-LotResult` | nowhere | `framework_maximum` at the RESULT'S LOT via BT-13713 (`lot_of`), never lot-null (lot-null would poison `lot_only`/`lot_keys` for every genuine framework and add a Procedure head candidate). Same field as BT-271: no new corroboration. Fixes 395737 |
| **BT-660** | Framework re-estimated value per lot result | `BT-660-LotResult` | nowhere | lot scope as BT-709; field `estimated_value` (no new corroboration) or a new `framework_reestimated` (needs the corroboration flips measured). Often equals the lot award |
| **BT-118** | Notice framework maximum (sum over frameworks); NoticeResult; the framework counterpart of BT-161 | `BT-118-NoticeResult` | nowhere | tender-scope `framework_maximum`. Risk: a new Procedure head candidate on every framework award notice — heads move up |
| **BT-1118** | Notice framework re-estimated value; NoticeResult; SDK ≥ 1.3 | `BT-1118-NoticeResult` | nowhere | tender scope, same field choice as BT-660. 627219: BT-1118 = BT-660 = ×100 of BT-271-Lot |
| BT-156 / BT-1561 | Group framework maximum / re-estimate; NoticeResult/GroupFramework via BT-556 | `BT-156-NoticeResult` … | nowhere | defer (LotsGroup row via BT-556) or leave; no fixture |
| BT-157 | Group framework maximum, CN, LotsGroup | `BT-157-LotsGroup` (Lot spelling grafted as `UBL-FrameworkEstimatedMaximumValue`) | nowhere at LotsGroup | low risk; duplicates BT-271-LotsGroup |
| BT-710 / BT-711 | Lowest / highest tender received; LotResult | `BT-710/711-LotResult` | nowhere | LEAVE — a losing bid is no procurement value; BT-711 would become the head (4-can-29: €2.29 m vs an €816 k estimate) |
| BT-553, 160, 162, 644, 779, 782, 793, 795 | subcontracting, concession revenue, prize, payment, penalties, review | `-Tender`/`-Lot`/`-Review` | nowhere | leave |

Also unmapped: DÖE `SDK01-ProcurementProject-RequestedTenderTotal-EstimatedOverallFrameworkContractsAmount`.

## Fixtures (`crates/ingest/tests/fixtures/`)
- `eforms/can-cvd-legacy-00412845-2025.xml` (sdk-1.13, one lot, NOK): BT-1118 2 m (l.9), BT-118 4 m (l.10), BT-709 4 m
  (l.22), BT-660 2 m (l.23), BT-271 4 m procedure + lot (l.222, l.387); in the golden snapshot.
- `eforms/can-cvd-lot-00054478-2025.xml` (sdk-1.10, 5 framework lots): BT-1118 = Σ BT-660, each BT-660 = its lot's BT-720
  (the decision (c) shape).
- `eforms/can-dup-org-00305298-2024.xml`: BT-118 = BT-709 = BT-27-Lot = BT-27-Procedure.
- `eforms/can-pat-social-00250633-2024.xml`: BT-271 = BT-709 = 20 m; BT-161 = BT-118 = BT-1118 = BT-660 = 14 m (the
  publisher filed the re-estimate in BT-118).
- `eforms/can-maximal-sdk17.xml`: every field incl. BT-157 — sentinel values, mapping/scope tests only.
- `eforms-chain/4-can-29-380868-2026.xml`: BT-711 2,291,800 — the pin for "do not map BT-711".
- Mapping BT-709/118/660/1118 changes `fixtures/golden/project_apply.snapshot` and `project_refold.snapshot`.

## Unit 2's decisions to take (with the census)
1. BT-709-LotResult → `framework_maximum` at the result's lot (BT-13713), never lot-null.
2. BT-118-NoticeResult → tender-scope `framework_maximum`.
3. BT-660-LotResult → lot scope; BT-1118-NoticeResult → tender scope; field choice per the census.
4. Leave BT-710/711 and the others above with their reasons; defer BT-156/1561/157.
5. Add the DE1 aliases for every mapped field (`project.rs:1128`).
6. A stored lot value can stay wrong after unit 3 when the partner arrives only in a later version (492's
   "unreachable" class) — re-read the head version.
7. Fallback if heads move too broadly: partner-only figures scoped to their lot (a new channel).
