# Organization merge-key error rates — measured (issue 168)

On-box bounded reads, 2026-08-24, corpus of **12,634,921 organizations** (90.8%
provisional / no identifier; only **1,161,042 carry an identifier**). This closes
the empirical half of issue 168 for the rates that FIT the 10s SQL sandbox, and is
honest about the one that does not.

## The headline correction to data-profile-2026-08.md §1

The study's alarming fake-country specimens (2026-08-09) have **largely been cleaned**,
almost certainly by issue 234's provisional-organization merge (95.3% of the org layer,
landed after the study). Measured now:

| study claim (2026-08-09) | measured now (2026-08-24) |
|---|---|
| "fake-country register prefixes ≥ 50,700 org rows; 96.6% of HR-vat orgs are German HRB" | HR is now **9,595 genuine Croatian** national-id orgs (11-digit OIB, Croatian names) + **475 HR-vat junk** (TED-internal ids prefixed `HR0…`, some German entities e.g. "Schleswiger Stadtwerke"). Register-prefix leak into a WRONG country is now **~89 rows** total (HRB-numbers under AT/SE/NL/LV/BE); 11,358 HRB orgs correctly sit under DE. |

So the "tens of thousands of fake-country rows" class is gone; what remains is small
and enumerable.

## Measured rates

- **False-split** (one real identifier spread across multiple org rows — the merge key
  failed to unify): **8,416 identifier clusters across 18,018 org rows = 1.55% of
  identifier-bearing orgs.** Corpus-wide, exact. Note this is a floor: it only counts
  splits where BOTH rows kept the identifier; a split where one side lost its id to the
  provisional pool is invisible here.
- **Fake-country, ISO-3 leakage**: a few hundred rows carry 3-letter codes (MCO, ARE,
  SGP, ZAF…) in the 2-letter country field, plus `1A` (TED "not specified"). Small,
  enumerable, a normalization-table fix.
- **Placeholder-VAT false-merge** (one junk identifier merging distinct real entities):
  **persists.** Specimen `DE123456789` → org 15176 carries **144 distinct mention
  names** (the study saw 138 — it has grown, so nothing suppresses it). This is the
  merge-key's worst class and it is NOT cleaned.

## What could not be measured on-box (honest limitation)

The **corpus-wide false-merge rate** — for every identifier-bearing org, how many
distinct real entities it wrongly unifies — requires joining 12.6M orgs against the
mentions table and times out the 10s SQL budget every way I bounded it (even id-range
slices, because the mentions scan is the cost). Placeholder-VAT specimens confirm the
class is real and unbounded per-org; the rate needs an offline pass over a DB copy
(a `data-quality`-style windowed job, or the snapshot the study asked for). Filed as
the residual of 168.

## Feeds B8 normalization

The B8 validation rules the study seeded are confirmed and re-prioritised by scale:
1. **Reject placeholder identifiers** (DE123456789 and its family) BEFORE they become a
   merge key — this is the highest-value rule; it is the only large uncleaned class.
2. Normalize country to ISO-2 (fold ISO-3 and `1A`) — small, mechanical.
3. The register-prefix→country confusion is largely self-resolved post issue-234; a
   guard against `HR0…`-style TED-internal ids entering the VAT field mops up the 475.
