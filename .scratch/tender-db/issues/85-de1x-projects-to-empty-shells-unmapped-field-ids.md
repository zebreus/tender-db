# 85 — eForms-DE 1.x reclaim projects to EMPTY tender shells (unmapped DE1-* field ids)

Status: open — DISCOVERED 2026-08-01 (sdk-vendor, from snapshot 531). ~218K German tenders have zero facts. Parse layer is correct; projection-only fix + re-fold.
Kind: correctness / completeness (projection mapping)
Blocked by: —
Relates to: 75 (DE-1.x empirical inventory), 78 (DÖE grafts), the SDK01/sdk-0.1 projection-mapping precedent (project.rs:2232), ADR-0009

## Symptom (snapshot 531)

DE-1.x reclaim landed clean in the PARSE layer — **218,635 reclaimed** (de-1.1 145,717 · de-1.2 72,887 ·
de-1.0 31; 241 held), all `parse_state=parsed`, `projected=1`, and genuinely rich (sample notice 26195620:
45 sections, 42 codes, 29 ids, 8 classifications, 4 dates, full German title + description).

But the TENDER layer for the cohort is empty shells: of 110 DE-1.x notices sampled across 5 offset windows,
109/110 have a `tender_versions` row but **0/110 have any `tender_version_texts` / `_classifications` /
`_parties` / `_amounts` / `_lots`**. `current_published_at=0`, `notice_subtype` NULL. Baseline reclaimed
sdk-1.7 at the same offsets: 56/60 with full texts+classifications; arbitrary non-DE tenders across the whole
id range all have details. So the fold ran for DE-1.x but emitted version rows with **no facts** — the
dashboard has nothing to render (no title, CPV/NUTS, amounts, lots, buyer parties).

## Root cause (confirmed) — recurrence of the SDK01 projection gap for the DE dialect

The vendored eforms-de-1.x metadata (issue 75) emits **path-shaped** field ids prefixed `DE1-` instead of
eForms BT ids:

- DE-1.x: `DE1-ProcurementProject-Name`, `DE1-ProcurementProject-Description`,
  `DE1-ProcurementProject-MainCommodityClassification-ItemClassificationCode` (cpv),
  `DE1-ProcurementProject-RealizedLocation-Address-CountrySubentityCode` (nuts),
  `DE1-Organizations-Organization-Company-PartyName-Name`.
- Working profiles: `BT-21-Lot`, `BT-24-Lot`, `BT-262-Lot`, `BT-500-Organization-Company`.

`canonical_name()` in `crates/ingest/src/project.rs` (the `("BT-21","title")`-style table, matched by full
id then `stem()`) has **no `DE1-*` entries**, so every DE-1.x fact falls through unmapped and the fold emits
a factless version. The doc comment at **project.rs:2232 already records the identical fix for the DÖE
sdk-0.1 dialect** (`SDK01-ProcurementProject-Name` keyed by **full id** because path-shaped ids collide under
the coarse `stem()`). DE-1.x needs the same treatment.

## Fix

1. Add `DE1-*` → canonical mappings to the projection tables in project.rs, **keyed by full id** (like the
   SDK01-* entries), covering at minimum: name/title, description, main-object CPV, realized-location NUTS,
   organization/company party name — plus the rest of the DE-1.x field inventory (sdk-vendor owns the full
   list from issue 75's `fields-de-1.x.json`; map each `DE1-*` leaf to its canonical fact kind).
2. **Re-fold the DE-1.x cohort only** — projection-only, NO re-parse (the parse layer is already complete
   and correct). Design the re-fold with proj-fix: mark the ~218K DE-1.x notices `projected=0` and run an
   incremental `project rebuild=false` (bounded — the cohort touches its own tenders, not corpus-wide, so it
   avoids the issue-62/81 corpus-wide-touch pathology), OR a scoped re-projection. Do NOT do a full
   `rebuild=true` (wipes + re-folds all 8.1M).

## Validation

After the fix + re-fold: a sample of DE-1.1 and DE-1.2 tenders render fully (German title/description, CPV,
NUTS, amounts, lots, buyer party resolving to its org, e.g. ORG "Städtisches Klinikum Görlitz gGmbH"); zero
DE-1.x tenders with a version row but no facts. Add a projection test with a DE-1.x fixture asserting the
facts land (guards the mapping the way the SDK01 case should be guarded).

## Note

This is content-completeness, not data loss — the ~218K are safe and complete in the parse layer, just
mis-projected. It's the German dialect (the largest cohort), so it matters for the "full data" goal. The
~1.5M non-DE reclaimed (SDK/OC/2008-opoce) folded WITH content (verified), so this is scoped to DE-1.x.
