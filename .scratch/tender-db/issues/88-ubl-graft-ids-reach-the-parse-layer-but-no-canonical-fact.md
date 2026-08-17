# 88 — the 47 grafted `UBL-*` field ids reach the parse layer but map to no canonical fact

Status: DISPOSITIONS SHIPPED 2026-08-17 (commit `767c261`, deployed rev `767c261`) — refold-scope
follow-up OPEN (below). The inventory had grown 47 → 68 ids since filing; every one now has its
ADR-0004 disposition, gate-enforced.

## Shipped (2026-08-17, owner)

- **Mapped (6):** `UBL-FrameworkMaximumAmount` + `UBL-FrameworkEstimatedMaximumValue` →
  `framework_maximum` (the BT-271 fact, same target the DE1 alias table routes to);
  `UBL-FundingProgram` → `funding_program`, `UBL-SelectionCriterionName` → `selection_criterion`,
  `UBL-TendererRequirementDescription` → `tenderer_requirement`, `UBL-AppealTermsDescription` →
  `appeal_terms` (new additive text facts). Full-id keyed like SDK01-*; scope from the section.
- **Explicitly ignored (62):** `UBL_PARSE_ONLY`, each with its reason — no canonical channel for
  code/indicator/integer/number types; org-contact PII stays parse-layer (issue-173 posture);
  results-layer statistics (Lower/HigherTenderAmount, ReceivedTenderQuantity, TenderResultStartDate)
  belong to the results binder, a fact row would misfile them; period fields
  (FrameworkDuration*/PlannedPeriod*) have unsettled semantics — mapping wrongly beats not mapping
  only in the wrong direction. All still served verbatim by `/v1/notices/{id}/content` (218-B).
- **Gate:** `ubl_grafts_are_all_mapped_or_ignored` reads index.rs's graft inventory from SOURCE and
  fails any id with no (or two) dispositions plus stale ledger entries — a new graft cannot ship
  undispositioned. Red-demonstrated three times during the pass itself (ContractExecutionDescription,
  the Electronic*Usage family, the PlannedPeriod pair — each missed by hand, caught by the gate).

## Follow-up (open): refold scope for the already-folded corpus

The 6 mapped facts apply to NEW ingests and any future era refold; already-folded notices carrying
these ids keep their pre-mapping content until refolded. The profile-scoped stamp (issue 179) does
not fit a field-level gap — the affected set is "notices whose parse layer carries one of the 6
mapped ids", cross-profile. Needs a bounded enumeration job (the field ids are not indexed; a
batched sweep over notice value tables), then requeue + `stamp_stale` for exactly their tenders.
Low urgency: the mapped ids are sdk-0.1/TenderResult-era mounts, a small slice of the corpus.

Was: open — DISCOVERED 2026-08-01 (sdk-vendor, incidental to issue 85; verified by proj-fix)
Kind: correctness / completeness (projection mapping)
Blocked by: —
Relates to: 85 (the DE1-* instance of this same class), ADR-0004, CONTEXT.md "nothing silently dropped"

## Symptom

`crates/ingest/src/eforms/index.rs` deliberately grafts **47 distinct synthetic `UBL-*` field ids**
onto eForms notices — content-bearing elements the SDK's inventory does not describe. The parse layer
stores them like any other value.

`crates/ingest/src/project.rs` contains **zero** `UBL-` entries (`grep -c 'UBL-' project.rs` → 0). So
every one of those values falls through `canonical_name()` unmapped and never becomes a canonical
fact. The data is captured and then silently dropped one layer later.

## Why it matters — the graft's own rationale argues against the current state

index.rs:45-56 states the intent explicitly:

> ADR-0004 allows only two dispositions, mapped or explicitly ignored, and "everything the source era
> publishes, nothing silently dropped" argues for mapping: a contact's job title and a buyer's PO box
> are real data.

The parse layer honours that; the projection does not. The values are neither *mapped* nor
*explicitly ignored* — they are implicitly dropped, which is the disposition ADR-0004 excludes.

Some of the 47 are document plumbing (`UBL-DocumentFileName`, `UBL-DocumentHash`,
`UBL-DocumentLanguageID`), but several are substantive procurement facts:

- `UBL-FrameworkMaximumAmount` — framework ceiling
- `UBL-AwardCriterionWeightNumeric` — award-criterion weights
- `UBL-SelectionCriterionName` / `UBL-SelectionCriterionUsage`
- `UBL-PlannedPeriodStartTime` / `UBL-PlannedPeriodEndTime`
- `UBL-TerminatedIndicator`
- `UBL-FundingProgram`
- `UBL-ElectronicCatalogueUsage` / `-OrderUsage` / `-InvoiceUsage` / `-PaymentUsage`

## Same class as issue 85, wider blast radius

Issue 85 was: field ids present in the parse layer, absent from `canonical_name()`, so the fold
emitted factless versions — 218,635 DE-1.x notices. This is the identical failure shape, but it
applies to **every eForms profile corpus-wide**, not one dialect. It is smaller per-notice (a handful
of fields rather than the whole vocabulary), which is exactly why it has gone unnoticed: it degrades
content quietly instead of producing visibly empty shells.

## Not data loss

The parse layer holds everything, so nothing needs re-fetching or re-parsing. Like issue 85 this is
projection-only: add the mappings, then re-fold the affected notices. Unlike issue 85 the affected
set is broad, so the re-fold scoping needs its own thought — a profile-cohort `refold` (issue 85's
op) does not obviously fit a corpus-wide field-level gap.

## Fix sketch

1. Triage the 47 into *mapped* (give each a canonical fact kind) and *explicitly ignored* (document
   why, so ADR-0004's two-disposition rule is satisfied either way). Plumbing ids are legitimate
   ignores; the procurement facts above are not.
2. Add the mapped ones to the projection tables, keyed by full id (they are path-shaped, so they
   collide under `stem()` — the same reason SDK01-* and DE1-* are full-id keyed; see project.rs:2230).
3. Decide the re-fold scope separately. Deliberately out of scope for the issue-85 recovery batch.

## Provenance

Spotted by sdk-vendor while verifying that graft tables could not inject a `BT-04` into DE-1.x
notices (they cannot — the only `BT-*` id any patch table injects is
`BT-165-Organization-Company`). They correctly scoped it out of issue 85 rather than widening that
change. Recorded here so it is not lost: it is the same class of defect that produced 218K empty
tenders, and it is currently invisible.
