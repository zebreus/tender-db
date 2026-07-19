# Notices are append-only; the canonical layer keeps full version history

Sources publish immutable Notices (new versions, corrigenda, award notices);
users query canonical Tenders/Lots/Bids/Organizations. We store both layers
fully: Notices append-only (raw payload + parsed relational form), and a
*versioned* canonical layer — every canonical row carries its validity period,
each version traceable to the Notice that caused it.

We chose versioning over a current-state-only projection (which was cheaper and
rebuildable-by-definition) because time-travel questions — "what did this
tender look like before the corrigendum?", "how did the deadline move?" — are a
core use case and must be answerable in plain SQL against the canonical shape,
including through the public read-only SQL endpoint, without re-deriving state
from notice history per query.

Consequences: canonical tables carry temporal columns and writes update
validity ranges; the canonical layer must still be deterministically
rebuildable from the Notice archive (the archive remains the source of truth,
versioning does not replace it).

Amendment (2026-07-19, from docs/research/ted-empirical-checks.md): change
scoping is diff-based — what changed between canonical versions is computed by
diffing, never taken from the notices' own change declarations (BT-13716 names
changed sections in only ~58% of real change notices; when present it is
merely a cross-check).
