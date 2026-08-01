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
   and correct). Design: **`.scratch/tender-db/design/de1x-refold.md`** (proj-fix, 2026-08-01). Confirmed
   mark-`projected=0` → incremental `project rebuild=false` is correct and bounded (the legacy full-corpus
   fallback is unreachable for `eforms:eforms-de-1.x`; RAM flat via the issue-81 chunking). Do NOT do a full
   `rebuild=true` (wipes + re-folds all 8.1M). **Three hard preconditions** — see the note:
   - `tenders_procedure_key` + `tenders_island` must exist first (the incremental per-Tender identity probe,
     canonical.rs:2357-2372, full-scans 8.1M tenders without them) → the issue-82/83 reindex op is a
     prerequisite, not cleanup;
   - `rebuild_in_progress` must be clear (supervisor.rs:687 — `salvage` OUTRANKS `rebuild=false` and would
     `reset_tender_layer()` the whole 8.1M layer);
   - the `DE1-*` mappings must be deployed BEFORE marking, or the cohort re-folds to empty shells again and
     silently re-marks itself `projected=1`.

## Validation

After the fix + re-fold: a sample of DE-1.1 and DE-1.2 tenders render fully (German title/description, CPV,
NUTS, amounts, lots, buyer party resolving to its org, e.g. ORG "Städtisches Klinikum Görlitz gGmbH"); zero
DE-1.x tenders with a version row but no facts. Add a projection test with a DE-1.x fixture asserting the
facts land (guards the mapping the way the SDK01 case should be guarded).

## Note

This is content-completeness, not data loss — the ~218K are safe and complete in the parse layer, just
mis-projected. It's the German dialect (the largest cohort), so it matters for the "full data" goal. The
~1.5M non-DE reclaimed (SDK/OC/2008-opoce) folded WITH content (verified), so this is scoped to DE-1.x.

## Comments

### 2026-08-01 — sdk-vendor — fix implemented, committed, tests green (not deployed)

**Commits:** `43d36dd` (the vocabulary fold) and `029d2a7` (the uuid gate). `cargo test -p ingest` green,
including `project_golden` and `project_equivalence` — the byte-identity gates, so no existing fixture's
grouping or fold output changed.

**Implementation deviates from the obvious approach, deliberately.** The brief was to add `DE1-*` entries to
`canonical_name()`'s tables. That alone would not have fixed it: those tables are one of ~15 sites keying on
eForms ids — the results graph matches on `stem()`, org mentions on `ORG_NAME_FIELD`/`BT-501`/`BT-514`, party
roles on the `OPT-300/301` prefixes, and subtype / procedure-key / instants on their own scalars. That path
yields title + CPV + NUTS and still no buyer, no lots, no awards, no procedure key.

Instead the dialect is folded onto the eForms vocabulary **once**, in `normalise_de1`, at the 5 sites where
the projection loads a parsed chunk — in place on memory the projection already owns, so no notice is cloned
and a non-DE-1.x notice costs one string compare. Downstream, every existing rule applies unchanged. 51
aliases, full-id keyed per the SDK01 precedent. The stored notice layer keeps its `DE1-*` ids: they are the
source's own names, and the parse layer records what the publisher sent.

**Second gap found during implementation, not in the original diagnosis.** The EU SDK splits
`cac:ProcurementProjectLot` into Lot / LotsGroup / Part with an `[cbc:ID/@schemeName=…]` predicate. The
empirical DE-1.x inventory is predicate-free (issue 75), so all three collapse onto one node whose kind is
`ProcurementProjectLot` — which matches no `LOT_KIND`, so **every DE-1.x lot was dropped regardless of field
mapping**. Fixed by reading the distinction back from the section id's own prefix (`LOT-`/`GLO-`/`PAR-`),
which is exactly what the predicate would have tested.

**The uuid gate (`029d2a7`).** Aliasing `DE1-ContractFolderID` onto `BT-04-notice` routed it through a path
that accepts any non-empty string, because on TED BT-04 is a spec-guaranteed uuid. eForms-DE 1.x carries no
such guarantee. A portal-local reference number would have keyed a Tender on a notice-local string,
collapsing every notice sharing it into one Tender — issue 34's wrong merge, at cohort scale, inside the run
that also retires ~218K islands and renumbers. So the folder id is kept OUT of the alias table (keying a
Tender is not the same trust decision as mapping a fact) and keyed through the gated dialect path, exactly as
sdk-0.1's has been since issue 34. The gate has no failure direction: a genuine TED twin shares that twin's
uuid and passes untouched, while a failing id leaves the notice an island — its state today.

Evidence deliberately did **not** drive this: all six `ContractFolderID` samples in the empirical scan are
well-formed uuids, but the sampler keeps only the first six distinct values of 216,691, so that is consistent
with all-uuid and nowhere near proof. Recorded here so nobody later unpicks the gate citing the samples.

**Correction to the symptom figures above.** The "109/110" in the Symptom section is 109 of 110 **notices**,
not of 110 *parsed* notices — the sampling query filtered on `profile` only, never on `parse_state`, so the
sample included some of the 241 still-quarantined notices, which have no `tender_versions` row by design. The
single miss may therefore be entirely benign. The substantive finding is unaffected: **0/110 had any facts**,
across every window, regardless of parse state. Being chased as a possible breach of the documented invariant
at store/lib.rs:417 (*"A parsed notice always causes exactly one version"*) — proj-fix holds a discriminating
query that filters `parse_state='parsed'`.

**Known diagnostic, not a blocker — key matching is case-sensitive.** `procedure_key` returns the raw stored
string (`first_id(...).filter(...)` does not transform), and `tenders.procedure_key` is matched under BINARY
collation. `is_uuid` accepts upper-case hex, so `550E8400-…` would not equal `550e8400-…`. Fails safe (a
split, not a wrong merge) so it is a yield risk, not a correctness risk. Whitespace is **not** a risk:
`eforms::value::convert()` trims at value.rs:24 and is the sole constructor of `Value::Id` in the eForms
module (one call site, parse.rs:244). Being measured as binary vs `COLLATE NOCASE`; if the counts differ it
gets its own issue — cross-source key normalisation, pre-existing, SDK01 affected equally, honest fix a
corpus-wide key rewrite, not a passenger on this batch.

**Ledger.** `crates/app/data/quarantine-ledger.json` already carries an `eForms-DE 1.x` entry marked
`"resolved": "2026-07-29"` whose diagnosis claims "the recovery rebuild (ADR-0009) folds them in" — false
until this refold lands. Decision (team-lead): no edit this batch; nginx comes up as the last step, strictly
after the cohort verifiably renders facts, so the claim becomes true before it is ever served publicly. The
ledger is `include_str!`-compiled (app/src/ledger.rs:15), so any correction can only ship with a deploy.

**Still open:** deploy + the projection-only re-fold (proj-fix owns the mechanism; it regroups rather than
just re-deriving facts, since the cohort goes from keyless islands to uuid-keyed procedures that can merge
with TED twins under ADR-0003), then live re-verification against the served layer.
