# 182 — the 94 `translation-structure-mismatch` rows have no diagnosis

Status: RESOLVED-SUPERSEDED (2026-08-14 — the 201 sweep retired the reason entirely: 94 co-original/defective-original members reclaimed via section adoption, ledger "Form copies diverging from their original")
Kind: data-quality investigation (suspected-gap bucket)
Blocked by: —
Relates to: 30 (classification), 137 (measured: 94 rows, 0 reclaimed), 180/181 (same sweep can cover all three)

## Why

Same shape as issue 181, smaller: `translation-structure-mismatch` classes as `SuspectedGap` on
the dashboard (still-held = 94, public `/api/dashboard` 2026-08-10), zero reclaimed, and no issue
ever owned it. The reason name says a notice's translation block disagreed structurally with its
original — which sounds like real notices lost to a structural assumption, exactly the class the
investigate-then-fix discipline exists for. A cause is a cause regardless of its size (issue 84's
7-row lesson); 94 rows silently riding the suspected total are unfindable when someone finally
asks what "suspected" is made of.

## What

Sample all 94 (they fit in a handful of bounded reads), attribute the mismatch (per profile, per
vintage), and either fix-and-reclaim or document the keep with a ledger row. Cheap to bundle with
the 180/181 sweep — one archive pass can pull samples for all three buckets.
