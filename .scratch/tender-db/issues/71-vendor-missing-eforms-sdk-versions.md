# 71 — vendor missing eForms SDK versions (recover ~1.2M quarantined notices)

Status: IN PROGRESS 2026-07-29 — all versions vendored + deployed (issues 74/75 + f85ab07/dcea702). Reclaiming into the parse layer via reclaim-all (ADR-0009): SDK 1.0/1.7/1.10 done (556,986 reclaimed), blanket unknown-customization job running for the rest (1.6/1.8/1.9/1.11/1.3/1.5 + DE 1.x). Tender-layer fold pending the final rebuild.
Kind: completeness / data-quality
Blocked by: —
Relates to: ADR-0004 (strict quarantine), CONTEXT.md (all eForms BTs representable)
Owner: (assign)

## Finding (2026-07-29 snapshot analysis)

Total quarantine = **2,419,327 notices** (~14.6% of ~16.6M attempted). Top reason is
`unknown-customization` = **1,201,071**, and it is almost entirely eForms notices whose
declared SDK version we do NOT vendor. Quarantine detail: *"no vendored SDK metadata for
eforms-sdk-X"*. Breakdown by profile:

| profile | quarantined |
|---|---|
| eforms:eforms-sdk-1.7 | 344,517 |
| eforms:eforms-sdk-1.10 | 255,426 |
| eforms:eforms-de-1.1 | 145,859 |
| eforms:eforms-sdk-1.8 | 140,834 |
| eforms:eforms-sdk-1.11 | 107,969 |
| eforms:eforms-sdk-1.9 | 87,438 |
| eforms:eforms-de-1.2 | 72,986 |
| eforms:eforms-sdk-1.6 | 37,669 |
| eforms:eforms-sdk-1.3 | 4,834 |
| eforms:eforms-sdk-1.0 | 3,470 |
| eforms:eforms-sdk-1.5 | 35 |
| eforms:eforms-de-1.0 | 31 |

We currently vendor only **1.12–1.15** + **eForms-DE 2.0/2.1** + **sdk-0.1**
(`crates/ingest/src/eforms/sdk.rs` `ACCEPTED`, backed by `crates/ingest/sdk/fields-*.json`).
Everything declaring SDK 1.0–1.11 or eForms-DE 1.0–1.2 quarantines as `unknown-customization`.

These are in-scope eForms notices (CONTEXT.md: all eForms BTs must be representable). They are
quarantined CORRECTLY per ADR-0004 (kept whole, raw payload in the archive, `reprocessed_at`
NULL, never silently dropped) — but they are recoverable data we should ingest. This is the
single biggest completeness lever in the corpus (~1.2M notices ≈ 7.7%), incl. ~219K German
(eForms-DE 1.0–1.2, part of the ~40% German volume).

## The fix (per-version: vendor + accept + reprocess)

1. Obtain `fields.json` for each missing version from OP-TED/eForms-SDK (GitHub tags:
   `1.6.0`…`1.11.0`, and the SDK-DE lines for eForms-DE 1.0/1.1/1.2). Vendor verbatim into
   `crates/ingest/sdk/` (same naming as the existing `fields-1.12.0.json`).
2. Add each to `ACCEPTED` in `crates/ingest/src/eforms/sdk.rs` (and the DE→EU map in `resolve`
   if the DE dialect needs a ProfileID split like 2.1 does).
3. Reprocess the quarantined notices from the archive (the importer re-parses without
   re-downloading; `reprocessed_at` gates it). ~1.2M reprocess → ingest → project.

## Feasibility caveat (MUST check per version)

`sdk.rs:41-46` documents that **SDK 1.0 is deliberately NOT vendored**: its `fields.json`
predicates use descendant axes + boolean `or` that `eforms::xpath` does not model, so vendoring
1.0 needs an xpath-grammar extension, not just the JSON. **Each missing version must be checked**:
does its `fields.json` parse + evaluate under our xpath grammar? The bulk (1.6–1.11) sit
adjacent to the vendored 1.12–1.15, so are likely compatible, but VERIFY — a version needing
grammar work is a bigger slice. Prioritise by volume: 1.7/1.10/1.8/1.11/1.9/1.6 + de-1.1/1.2
(~1.19M) first; 1.0/1.3/1.5/de-1.0 (~8K) last (1.0 needs grammar work per the note).

## Validation

- Per version: a fixture notice of that CustomizationID parses to a Notice with the expected
  BTs (add to the eforms test fixtures), no new quarantine.
- After reprocess: `unknown-customization` count drops by ~the vendored volume; notices/tenders
  grow; re-run the projection; spot-check a few recovered tenders on the dashboard.
- No regression on the already-vendored 1.12–1.15 path (byte-identity suite stays green).

## Progress (2026-07-29, sdk-vendor)

Spike + clean-cohort done. The 12 versions split three ways by grammar feasibility:

- **CLEAN — vendored (JSON only, no grammar change):** SDK **1.8/1.9/1.10/1.11** (~591K).
  Each deserializes into `struct Sdk`, all field types map, every xpath folds into the match
  index (eforms suite 9/9 green). Commits: `f85ab07` (1.10), `dcea702` (1.8/1.9/1.11).
  → handed to team-lead for deploy + archive reprocess (ops step, sequenced after the daily tick).
- **GRAMMAR-BLOCKED → issue 74:** SDK **1.0/1.3/1.5/1.6/1.7** (~390K, incl. the biggest slice
  1.7=344K). One field each — `efbc:CompanySizeCode` (BT-165), a Tenderer/Subcontractor join
  predicate using descendant axis `//` + boolean `or` that `eforms::xpath` doesn't model.
  Confirmed the construct is absent from 1.8+. Filed as issue 74 (recommend extending the grammar).
- **SEPARATE SOURCING → issue 75:** **eforms-de 1.0/1.1/1.2** (~219K). Not on OP-TED github; SDK-DE
  fork at gitlab.opencode.de, and `eforms.rs:256` says DE 1.x may have no artifact at all. Filed as
  issue 75 (sourcing/feasibility spike — vendor vs empirical inventory like sdk-0.1).

Net: ~591K recoverable now (vendored), ~390K via issue 74, ~219K via issue 75 — fully mapped.
