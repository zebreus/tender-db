# 462 — X02 BRIN notices are minted as procedure Tenders: 368's `kind_of` unit was never built

Status: ready-for-agent — filed 2026-10-01 from the owner's board survey (workflow wf_4eac8781-4d0, verified by an adversarial pass). The first unit is the code: `kind_of` classifies by the BRIN class (X01 and X02), and an incremental fold corrects a stored `kind`, pinned by the X02 fixture; the scoped refold follows the deploy.
Kind: defect (projection: Tender kind), a unit of closed issue 368 that was never built
Relates to: 368 (Units item 3 and Done-when bullet 3; closed 2026-09-18 without them), CONTEXT.md:156 "BRIN notices
become minimal Tenders of a distinct kind", 237 (`refold-sections`, the index-backed cohort job)

## What is wrong

`crates/ingest/src/project.rs:973` holds `const REGISTRATION_SUBTYPE: &str = "X01";`, and `kind_of` (:3681–3686)
matches only that literal. Every other subtype falls to `_ => "procedure"`. Both fold paths call it: the plan path
(:2522) and the bucket path (:3024).

The eForms SDK defines the Business Registration Information Notice as two subtypes. In all 14 vendored EU SDKs
(`crates/ingest/sdk/fields-1.0.0.json` … `fields-1.15.0.json`), `OPP-100-Business` (Notice Purpose) is mandatory
for exactly `["X01", "X02"]`. In sdk-1.13, 56 fields, `BT-01(c)-Procedure` among them, are forbidden for exactly
that pair. The repo has one fixture of each. Both carry `NoticeTypeCode listName="bri"`:
`brin-x01-00497689-2026.xml` is `brin-eeig` / X01, and `brin-eu-00568126-2023.xml` is `brin-ecs` / X02.

Live, read 2026-10-01 11:55 UTC (`/health` rev 9b44528):

| tender | publication (fixture) | subtype | kind | title | parties / texts |
| --- | --- | --- | --- | --- | --- |
| 1167110 | 00497689-2026 (brin-x01) | X01 | registration | null | 0 / 0 |
| 1167207 | 00568126-2023 (brin-eu) | X02 | **procedure** | null | 0 / 0 |

The two records have the same shape. Only `kind` differs, so 1167207 lists on `/v1/tenders` as an empty
procurement procedure, and `?kind=registration` misses it.

**How it fell through.** Issue 368 Units item 3 (line 46) reads: "`kind_of` keys on the BRIN class, not the literal
— X01 and X02 both … Pin with the existing `brin-eu-00568126-2023.xml` fixture." Its Done-when list (line 181) has
"X02 notices are `kind='registration'`". The 2026-09-12 re-read (lines 680–681) recorded it as "not touched by this
unit — still open under this issue's unit for `kind_of`". The 2026-09-18 closure counted "unit 3" as the r208
no-title answer, a different unit 3, and wrote "nothing remains". No commit message mentions X02 or `kind_of`, and on
the board only 368 names X02. The 2026-09-15 API review listed this defect
(`.scratch/tender-db/api-dq-review-2026-09-15.md:266`) under "Refuted", with no reason recorded.

**A `kind_of` change alone does not change prod.** `tenders.kind` is written only when a Tender row is inserted. On
an incremental fold, `Db::tender_identity` (`crates/store/src/canonical.rs:11514`) finds the existing row by
procedure key or by `(source, island_notice_id)`. It keeps `source` current (:11557–11560) and returns. Neither the
epoch-stale rewrite nor the unchanged early return (:11341) in `apply_tender_tx` touches `kind`. Only
`project rebuild=true` mints the row again. So a refold after a `kind_of`-only fix re-folds the X02 notices and
leaves `kind = 'procedure'`.

**Size.** 368 counted 285 titleless X02 "procedures" in tender ids 1.0–1.5M (2026-09-07, sdk-1.8 … sdk-1.14). On
that day the same band held 119 of the corpus's 131 X01 registrations. The corpus total is not measured. Unit 4
measures it.

## Proposed fix

1. **Classify by the class, with the SDK as the source of truth.** Replace the literal with
   `REGISTRATION_SUBTYPES: &[&str] = &["X01", "X02"]` and test membership in `kind_of`. The test
   `registration_subtypes_are_the_sdks_brin_class` (project.rs tests) reads every vendored `fields-1.*.json` and
   asserts that the `noticeTypes` of `OPP-100-Business`'s mandatory constraint equal the set. If a future SDK adds a
   BRIN subtype, the gate fails instead of the projection quietly minting procedures. Leave `data_quality.rs`
   alone: X02 is unclassified there on purpose, in the award/doc-type vocabulary
   (`doc_type_coverage_counts_a_code_in_neither_list`).
2. **Make an incremental fold correct a stored kind.** In `tender_identity`, read `kind` in the two identity SELECTs.
   When it differs from `p.kind`, update it, the same way `source` is kept current. This is the root of the refold
   path. Without it, any future change to `kind_of` also needs a full rebuild.
3. **Tests.** Extend `notices_without_a_procedure_key_become_island_tenders` (`crates/ingest/tests/project.rs:936`)
   with `eforms/brin-eu-00568126-2023.xml`: four notices, three islands, two `kind = 'registration'`. Add
   `a_refold_corrects_a_stored_tender_kind`: project the X02 fixture, set its Tender's `kind` to `'procedure'` (the
   prod state), call `unmark_projected_by_ids` and `stamp_stale_for_notices`, project incrementally, and assert
   `'registration'`.
4. **Scoped refold, after the deploy.** `POST /admin/jobs` with kind `refold-sections`, profiles
   `["BusinessCapability"]`. It pairs its own `project rebuild=false`. The cohort is index-backed
   (`notice_sections_kind_notice`, the 15 ms shape 237 measured). The SDK makes it X02-only: `OPP-105-Business`
   (Sector of activity), under the repeatable `ND-BusinessCapability`, is mandatory for X02 and forbidden for every
   other subtype, X01 included. The X02 fixture carries a `cac:BusinessCapability` and the X01 fixture does not.
   - **Gap:** sdk-1.2 … 1.7 define no `ND-BusinessCapability` node (`OPP-105-Business` sits under `ND-Root`), so an
     X02 published on those SDKs carries no such section. 368 found X02 only on sdk-1.8 … 1.14.
   - **The count:** the job's result line ("N notice(s) carry a ["BusinessCapability"] section …") is the corpus
     count. Record it here. To count before the refold, run one index-range
     `COUNT(DISTINCT notice_id)` on `notice_sections` with `kind = 'BusinessCapability'` through `/v1/sql`, on the
     terms of `docs/agents/prod-box-reads.md`. It was not run for this filing because it reads data pages.

## Verify

    curl -s https://tenders.zebreus.click/v1/tenders/1167207 | jq -c '[.kind, .notice_subtype, .publication_id]'

- **open** (2026-10-01 11:55 UTC): `["procedure","X02","00568126-2023"]`
- **done:** `["registration","X02","00568126-2023"]`, after units 1–3 are deployed AND the unit-4 refold has run. A
  deploy without the refold still prints `procedure` (see unit 2).
