# 258 — section 3's `named` rate divides by results that correctly have no winner

Status: CLOSED 2026-08-21 — the awardable denominator shipped with the DQ column work (`named`
divides by results a winner was possible for; `closed n/a` shown beside it; gate
`the_named_rate_divides_by_the_results_a_winner_was_possible_for`) and job 294's report shows it
behaving across every era (sdk-0.1: 132,600 `closed n/a` excluded, `named` 99.9 %; text era:
0 closed n/a, 100 % named where parsed). Was: needs-triage — filed 2026-08-20
Kind: measurement precision / honest denominator
Blocked by: 257 and 100 (both create NULL-decision populations this must not mis-handle — see the
sketch correction below, which they falsified before it was built)
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

**Correction to this issue's first sketch, before anyone builds it.** The sketch below originally
proposed a denominator of "versions whose result carries a decision that expects a winner":

    AND r.decision NOT IN ('no-rece', 'clos-nw') AND r.decision IS NOT NULL

That is wrong, and issue 100's fix is what makes it wrong. eForms-DE 1.x publishes **no**
`TenderResultCode` at all, so its results carry `decision IS NULL` — and after 100 they DO name
winners. Under that denominator the ~60k DE-1.1 award notices would land in the numerator
(`with_winner`) while being excluded from the denominator, and the rate would read **above 100 %**.
The same trap sits in sdk-0.1 after issue 257, whose ~125k unstated decisions are now NULL by design.

The denominator must therefore be defined by what it EXCLUDES, not by what it requires:

    -- exclude only results that positively deny an award AND name nobody
    SUM(CASE WHEN EXISTS(SELECT 1 FROM tender_version_lot_results r
                          WHERE r.tender_id = tv.tender_id AND r.seq = tv.seq
                            AND NOT (r.decision IN ('no-rece', 'clos-nw', 'open-nw')
                                     AND NOT EXISTS(SELECT 1 FROM tender_version_result_winners w
                                                     WHERE w.tender_id = r.tender_id
                                                       AND w.seq = r.seq)))
             THEN 1 ELSE 0 END) AS with_awardable_result

`named` then divides by `with_awardable_result`. Three properties, all of which matter:

- **The rate cannot exceed 100 %**: anything with a winner is in the denominator by construction, so
  the numerator is a subset. That is the same invariant issue 243's single-pass merge established for
  `density`, and for the same reason — a rate above 1 is not a number, it is a bug report.
- **An unstated decision counts as awardable.** Silence is not a denial (issues 100 and 257 both turn
  on that distinction), so a NULL-decision result stays in the denominator and its missing winner is
  counted honestly against us.
- **A publisher contradiction resolves toward inclusion.** `no-rece` WITH a named winner does occur —
  five of 2,895 in the sdk-0.1 2023-01 cross-tab. Keeping those in the denominator is the safe
  direction: excluding them while counting their winner is the >100 % bug again.

The difference between `with_results` and `with_awardable_result` is worth its own column: it is the
count of results the publisher closed without naming anybody, which is a real and interesting number
rather than a residue.

Keep `with_results` and `density` exactly as they are: they answer "did the block materialise", which
is a different question and already correct.

## Gate

Extend `a_result_block_with_no_winner_is_its_own_column` (ingest/src/data_quality.rs) with a row whose
results split across the three decision classes, and pin that a `no-rece` result does NOT depress
`named`. The existing 1,000/500/250-must-read-50 % assertion stays — it pins the choice this issue
refines rather than reverses.

## Deliberately not doing

Not touching the JSON's `winner_rate` shape (consumers may already read it); the new denominator ships
as an additional field beside it, and `winner_rate` is redefined only if the text and JSON would
otherwise disagree — which they must not.
