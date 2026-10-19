# 577 — /v1/notices/{id}/content labels the procedure-level contract nature `BT-23-Lot` on SDK 1.13/1.14 notices

Status: needs-triage — filed 2026-10-19 from the hourly audit (workflow wf_90e506fc-209, auditor + skeptic, confirmed;
`.scratch/tender-db/audits/2026-10-19-recent-tenders/verify_ted.md`). Next: unit 1, find the alias graft that adds the
predicated sibling and fix the branch ordering so an SDK-declared field id wins at its own path; re-parse scope.
Kind: data quality (notice content: field ids in the source's own vocabulary); low severity — the Tender's served nature is right
Relates to:
- 368 (BT-23 for FTS), 465.

## Verify

    curl -s https://tenders.zebreus.click/v1/notices/47414082/content | jq -c '[.sections[] | select(.section_id=="PROCEDURE") | .fields[]? | select(.field_id|test("^BT-23")) | .field_id]'

(adjust the jq path to the content shape). **done**: prints `["BT-23-Procedure"]`. **open (2026-10-19)**: the PROCEDURE
section carries `{field_id: BT-23-Lot, list: contract-nature, code: services}`.

## What was found

- Notices 47414082 (SDK 1.14, subtype 4, tender 8852009) and 47413608 (SDK 1.13, subtype 4, tender 8852005): the root PROCEDURE
  section serves BT-23-Lot; neither notice has a Lot section or a BT-23-Procedure row. The other procedure-level terms (BT-21,
  BT-24, BT-262, BT-22) carry the -Procedure suffix. Notice 47413157 (subtype 7, SDK 1.13, tender 8851996) shows BT-23-Lot in
  PROCEDURE as well as on its lots, and no BT-23-Procedure — systemic.
- Per the repo's own `fields-1.13.0.json` / `fields-1.14.0.json`: BT-23-Procedure is the predicate-free root
  `/*/cac:ProcurementProject/cbc:ProcurementTypeCode` (mandatory on subtype 4); BT-23-Lot is the predicated Lot path (forbidden
  on subtypes 1-6, E2, X01, X02). OpenAPI promises content field ids "in the source's own vocabulary … the notice's profile names
  which", so the served id contradicts the notice's own SDK.
- Likely cause (skeptic's read of the code): `crates/ingest/src/eforms/index.rs` ALIASES — the comment says an SDK-declared field
  at the target always wins, but `insert_field`'s gap_only check is per exact branch, so the graft adds a predicated sibling
  carrying BT-23-Lot, and `Branch::matching` sorts most-predicated first, so BT-23-Lot beats BT-23-Procedure.
- Not affected: the Tender's nature rows (`project.rs` contract_nature accepts both ids and takes scope from the section).

## Proposed units

1. **Fix (S).** A failing test on an SDK 1.13 subtype-4 fixture (root ProcurementTypeCode → BT-23-Procedure), the gap_only /
   matching fix, gate. Check which other aliased fields share the shape.
2. **Re-parse (S/M).** Size the affected notices (SDK versions where the alias fires) and re-parse them so stored content ids
   are right; no fold change expected (verify 0 corrected).
