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
