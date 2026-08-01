# 88 — the 47 grafted `UBL-*` field ids reach the parse layer but map to no canonical fact

Status: open — DISCOVERED 2026-08-01 (sdk-vendor, incidental to issue 85; verified by proj-fix)
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
