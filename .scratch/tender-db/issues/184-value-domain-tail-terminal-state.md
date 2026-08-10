# 184 — declare the reclaim campaign's terminal state: the value-domain tail needs a policy, a final pass, and a disclosure

Status: needs-triage (the sub-cent policy is an owner/team-lead decision; the pass and disclosure are mechanical after it)
Kind: policy decision + reclaim completion + dashboard disclosure
Blocked by: —
Relates to: 144 (diagnosed the nine causes and defined this end-state), 132 (the 51 negative amounts, canonical), 87 (relabel-on-failed-reclaim, deployed), 40 (ledger)

## Why

Issue 144 defined what "done" looks like: *"the expected steady state is a tail consisting of F
(documented representation policy) plus small honest-keep residues — at which point … the reclaim
campaign can be declared complete, with F's share explicitly owned by the amounts-as-cents
decision."* The six mechanical fixes landed (825dd64). Nobody owns the three steps that remain,
and the dashboard currently presents the tail dishonestly in both directions
(public `/api/dashboard`, 2026-08-10):

- `unknown-customization` still-held = 1,907 — but the structural causes are fixed; these rows
  carry a *stale* reason from before the fixes. They read as a parser gap that no longer exists.
- `unrepresentable-value` still-held = 3,487 (grew from 3,213 as the DE-1.x pass relabeled its
  residue) — counted as undifferentiated *actionable* loss, though its plausibly-dominant slice
  (cause F, sub-cent amounts vs integer-cents representation) is a deliberate KEEP pending policy,
  and 51 rows are the source-published negative amounts (issue 132).

## What

1. **Policy** (owner decision): amounts-as-cents stands or a representation ADR changes it.
   Cause F's rows are then either a documented keep or a fix-and-reclaim.
2. **Final relabel pass**: reprocess the 1,907 `unknown-customization` residue. With issue 87's
   fix deployed, every row that still fails re-records its true current failure — the stale
   structural reasons drain into their real value-domain causes, and the sdk ledger row's
   outstanding goes to ~0.
3. **Disclosure**: a ledger row (or data-quality note, issue 101's precedent) that names the
   deliberate-keep share of `unrepresentable-value` and cites the policy — so the actionable
   headline stops counting a decision as a defect. Then declare the campaign complete, in the
   144 sense, with numbers.

Order matters: 2 before 3, so the disclosure documents the terminal composition, not a snapshot
mid-drain.
