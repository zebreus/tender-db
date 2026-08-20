# 258 — section 3's `named` rate divides by results that correctly have no winner

Status: needs-triage — filed 2026-08-20 out of issue 257's diagnosis. Small, well-understood, and it
makes an already-useful column truer. Not urgent: the column is directionally right today
Kind: measurement precision / honest denominator
Blocked by: 257 (whose fix creates part of the population this is about)
Relates to: 101 (the column), 242 (the same argument one column to the left), 100, 257

## The defect

Section 3 of the data-quality report reads, per era:

    award-notices   with lot_results   density   no block parsed   with winner   named

`named` is `with_winner / with_results`. Issue 101 chose that denominator deliberately and for a good
reason — dividing by *award notices* would blame the winner chain for a missing result block, which is
already counted one column left. That argument is right and stands.

It just does not go far enough. Among the notices that DID materialise a result, some have **no winner
by the publisher's own statement**, and those belong in the denominator no more than a missing block
does:

- `decision = 'no-rece'` — no tenders were received. There is no winner to name, and the publisher said
  so explicitly.
- `decision = 'clos-nw'` — closed, no award. Likewise.
- `decision IS NULL` — after 257, the ~125k sdk-0.1 results whose block states only an award date. The
  publisher stated nothing about selection, so counting them as un-named winners attributes to our
  extraction a silence that is theirs.

Measured on the DÖE archive's 2023-01 (issue 257's cross-tab), 21 of 2,895 award notices are `no-rece`
— small there. But the NULL population that 257 creates is ~125,000 rows in one era, which is large
enough to move that era's `named` rate materially, and in the wrong direction: the fix that stopped us
fabricating `clos-nw` would, left alone, make our own report look *worse*.

## Why it matters more than the arithmetic suggests

This is the same failure mode issue 242 built the `no block parsed` column to prevent: a single rate
whose numerator and denominator answer different questions, so a reader cannot tell a publication gap
from an extraction gap. 242 separated "published nothing" from "we dropped it". This separates
"published no winner, and said so" from "published a winner we failed to resolve" — and only the second
is ours.

## Shape of the fix

`awards_template` already aggregates per era in one pass over `tender_versions`. Add a fourth `SUM`
over the versions whose result carries a decision that *expects* a winner, and make that the `named`
denominator:

    -- resolvable: the publisher asserted a selection, so a missing winner IS our gap
    SUM(CASE WHEN EXISTS(SELECT 1 FROM tender_version_lot_results r
                          WHERE r.tender_id = tv.tender_id AND r.seq = tv.seq
                            AND r.decision NOT IN ('no-rece', 'clos-nw')
                            AND r.decision IS NOT NULL)
             THEN 1 ELSE 0 END) AS with_awarded_result

Then `named = with_winner / with_awarded_result`, and the difference between `with_results` and
`with_awarded_result` is worth its own column — it is the count of results the publisher closed without
naming anybody, which is a real and interesting number rather than a residue.

Keep `with_results` and `density` exactly as they are: they answer "did the block materialise", which is
a different question and already correct.

## Gate

Extend `a_result_block_with_no_winner_is_its_own_column` (ingest/src/data_quality.rs) with a row whose
results split across the three decision classes, and pin that a `no-rece` result does NOT depress
`named`. The existing 1,000/500/250-must-read-50 % assertion stays — it pins the choice this issue
refines rather than reverses.

## Deliberately not doing

Not touching the JSON's `winner_rate` shape (consumers may already read it); the new denominator ships
as an additional field beside it, and `winner_rate` is redefined only if the text and JSON would
otherwise disagree — which they must not.
