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
