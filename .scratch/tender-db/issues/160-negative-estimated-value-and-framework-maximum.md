# 160 — SUPERSEDED by 132 (duplicate: the 51 negative ceiling amounts)

Status: superseded — **see [132](132-fifty-one-negative-estimated-and-framework-amounts.md)**, which
is the canonical record for this follow-up.

proj-fix and I filed the same follow-up within minutes of each other, from opposite ends of #33: I
from the triage side (team-lead's "those 51 are NOT closed by the re-spec"), they from the fix side.
Neither commit could see the other. Second numbering collision today after 121/122 — the tracker has
no allocator, so with parallel agents this is when-not-if, and per-agent bands only help when both
agents file in their own band, which is exactly what a shared follow-up does not do.

**132 wins** because the follow-up work sits with the fix owner, and it asks the right question first:
*do the 51 satisfy P4 against the chain's parse layer?* That is nearly free with the existing query
9/10 harness and is decisive before any distribution work.

## Content worth lifting into 132, if useful — recorded here rather than lost

**Already established, so nobody re-derives it:**

* **Not the fold** — P4 = 0, and 17,738/17,738 have an exact magnitude match in the chain's parse
  layer. (Though note: those figures are for the *whole* set. Whether the 51 individually satisfy P4
  is precisely 132's opening question, and the whole-set result does not answer it.)
* **Not a parser-version defect** — the full set scatters across 13 SDK versions (1.6→1.14 plus DE
  variants) and 4 years. A version-specific mapping fault would cluster.
* **Magnitudes differ from the benign mass** — these are €420k and €11.8M scale, against a
  `result_value` population that is 99.6% between €1 and €100.

**Three open questions, the third being the one I would chase first:**

1. Do the 51 cluster on a profile/version/publisher *once isolated from the 17,687*? Whole-set
   scatter does not rule out a cluster inside the subset, and nobody has looked.
2. Is a negative `estimated_value` a published **sentinel** rather than a value? The recurring
   exactly-`-100` (−1.00) pattern across several currencies in the larger set suggests sentinels
   exist in this data.
3. **Is some field whose published semantic is a DELTA being mapped as an absolute ceiling?** That
   would be a real defect, it is invisible from the sign alone, and it is the hypothesis that best
   explains why a *ceiling* — which cannot meaningfully be negative — carries one.

**And the framing worth keeping verbatim** (team-lead): *17,738 unexplained → 51 worth explaining is
progress, not resolution.* The failure it guards is using a mostly-benign explanation to wave through
the part it does not cover.
