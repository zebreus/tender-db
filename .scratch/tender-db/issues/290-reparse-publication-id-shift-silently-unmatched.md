# 290 — a parser change that shifts `publication_id` derivation makes `reparse_notice` silently no-op (counted as benign `unmatched`)

Status: ANALYSIS (2026-08-26, owner — adversarial reclaim review; partly inherent to identity-keyed reparse; LOW confidence it ever bites)
Kind: correctness hazard (a future parser change could pass a re-parse as "complete" while a cohort keeps its old fold)
Severity: LOW (requires an uncommon kind of parser change; nothing current triggers it)
Relates to: 100 (the re-parse mechanism), 272 (reparse resume/dry-run operator traps)
Found by: the 2026-08-26 adversarial reclaim/quarantine review.

## The hazard

`reparse_notice` (lib.rs ~1693) re-keys its target by the NEW parse's
`(source, publication_id, content_hash)`. `content_hash` is stable (same
bytes), but `publication_id` is parser-EXTRACTED — if the very parser change
being re-folded also changes how `publication_id` is derived, the lookup misses
the existing notice row, `reparse_notice` returns false, and the record is
counted as `unmatched` (process.rs ~585), which the `ReparseReport` doc treats
as benign. The stale parsed layer under the OLD publication_id is never
replaced.

## Why only ANALYSIS

This is partly intrinsic to identity-keyed in-place reparse, and no current or
planned parser change moves publication_id derivation. The risk is the
REPORTING: `unmatched` is a non-event, so such a shift would not be noticed.

## Guard direction (cheap)

When a re-parse job finishes with `unmatched > 0`, surface the count
prominently in the job summary (it may already be there — verify) and document
in the reparse runbook: "an unexpected unmatched spike on a re-parse means the
parser changed identity derivation — stop and investigate, do not treat the run
as complete." Optionally: fall back to a `(source, content_hash)`-only lookup
when the exact triple misses and the hash is unique — the same fallback shape
issue 193 gave reclaim-attempt stamping.
