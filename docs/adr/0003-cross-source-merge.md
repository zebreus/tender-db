# The same procedure on multiple Sources merges into one Tender

When a strong explicit cross-reference exists (e.g. a German portal citing the
TED/OJEU notice identifier), the records collapse into a single canonical
Tender fed by Notices from multiple Sources. Without such a reference,
per-source Tenders stay separate — merging is never heuristic.

The alternative (one Tender per Source, linked "same procedure") was cleaner —
no field conflicts, per-source version timelines — but pushes deduplication
onto every consumer, and duplicate procedures would silently inflate exactly
the market statistics the product exists for.

Consequences: the versioned canonical layer must interleave notice timelines
from several Sources, and field-level conflicts need explicit precedence rules
(to be defined with the schema; likely a per-source authority order). Because
Notices are append-only per Source, an incorrect merge can be undone by
re-projecting.

Verified (2026-07-19, docs/research/german-portals.md): oeffentlichevergabe.de
and TED publish the same procedure under identical notice and procedure UUIDs,
so for our chosen Sources the strong cross-reference is exact UUID equality —
no heuristics needed.

Verified (2026-09-07, docs/research/uk-fts.md §3): TED and the UK's Find a
Tender Service share no notices. Procedures started on TED before 2021 had to
finish on TED, and FTS holds nothing dated before 2021-01-02, so no TED↔FTS
merge exists to make. The two meet only at organization level: the same UK
buyers and suppliers, which the mention resolver keys on identifier, not
notice. An FTS Tender's procedure key is its OCDS `ocid`, which no TED notice
carries, so an FTS Tender is single-source.

Precedence (decided 2026-07-19; folding order signed off 2026-07-21 by the
project owner under transferred product authority): per field class —
publication-identity fields from TED (OJS gazette ids); German national content
(national codelists, DEX extension fields) from DÖE, the richer original
(docs/research/eforms-de-profile.md). For shared eForms fields, conflicts
resolve through the supersession fold, not a separate rule: a Tender's notices
are ordered by publication date (dispatch date only as a fallback when a notice
carries no publication date), and on an equal publication instant a fixed source
rank breaks the tie so the TED reading folds last and thereby wins the shared
fields and the publication identity (TED > DÖE, the authoritative gazette). This
is what the projection implements (`ingest::project`). Every resolved value
stays traceable to its Notice.

## Amendment (2026-10-02): a measured match is a merge warrant too

Lennart, 2026-10-02: "The fuzzy matches are probably fine if we are really really sure it's the same one. Nothing is
deliberately forbidden if it is correct." The rule above ("merging is never heuristic") is therefore replaced by
this one:

- **Declared** links stay the first warrant: a shared procedure key (BT-04, a UUID ContractFolderID), or a published
  cross-reference (ADR-0011's `OPP-090`, or a national notice citing a TED number).
- **A matched link is also a warrant when its precision is measured, not assumed.** The matcher is calibrated
  against ground truth before it writes anything. The ground truth is the TED↔DÖE pairs that merged on a shared
  UUID, as labelled positives, and same-buyer different-procedure pairs as labelled negatives. A match is admitted
  only in a band whose measured precision is near-certain: no false merge in the calibration sample, and a review
  read of the boundary cases. Everything below that band stays separate, and at most is served as a "possible
  duplicate" signal.
- **Every link is reversible.** Declared and matched links alike are rows in one edge ledger with their evidence and
  a rule name, which the fold reads. A wrong link is undone by deleting its row and refolding, the same discipline
  as the organization merge ledger (`org_merge_log`) and its dry → review → wet runs.

The org-level rules keep their own walls (genericness, name gates, verdicts; issues 234 and 300). This amendment
changes Tender identity across Sources only. Issue 481 carries the measurement and the build.
