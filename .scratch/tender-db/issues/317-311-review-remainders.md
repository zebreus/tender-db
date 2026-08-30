# 317 — The 311 campaign's unfinished halves: re-homing, escalations, the medium band

Status: ready-for-agent (three separable units, sized below)
Kind: data quality (organization layer)
Relates to: 311 (produced them), 312, 300

Filed 2026-08-30 because these lived only as a "REMAINING:" paragraph
inside 311's log. The campaign is done; these are what it left standing.

## Unit A — solo-mention re-homing (the fusion repair)

The campaign's flagship finding: a consortium vehicle carrying a lead
member's identifier ALSO captured that member's standalone mentions through
the shared literal (R&K Ingenieure: 7 of 8 mentions are the member alone;
Dobler: 7 solo mentions). Stripping the identifier stopped the fusion from
growing; it did not un-fuse what was already bound. Those mentions still
point at the vehicle org.

Every re-homing candidate is already NAMED: the 823 verdicts in
`311-batch-verdicts.json` carry a `handling` string that says which mentions
belong to which member. This is dissolve-adjacent machinery (re-point a
mention to a different org, mint the member row if absent) and it writes
entity references, so it needs its own adversarial panel round and a
dry-first plan like every other write path in this campaign.

## Unit B — the escalations queue

17 verdicts came back `unclear-escalate` (plus 16 wrong-identifier/low).
Nothing consumes them: they sit in `org_case_reviews`, permanently outside
the apply job's safe subset, with no surface that lists them. Minimum
viable: a report kind (`case-escalations`) listing them with their evidence
so they are reviewable at all, and a policy for what closes one.

## Unit C — the medium-confidence band

102 verdicts are `consortium-vehicle-wrong-identifier` at MEDIUM confidence
— the reviewer's honest "best explanation, single-sourced". They are
correctly not auto-applied. But "parked forever" is not a policy either.

The measured shape (both audit rounds) is that mediums cluster on
register-format impossibility: an FN/HRB/HRA number on a GbR/GesbR-shaped
consortium name, where the reading is decisive about the CLASS but not about
WHICH member holds the number. Two ways to convert them, both worth a
measurement first:

1. **Evidence enrichment** — a register lookup (Firmenbuch/Handelsregister)
   turns "some member's number" into "this member's number" and promotes the
   verdict to high with a citation. External dependency; check licensing.
2. **Structural corroboration** — if the same identifier stands on a
   standalone member row elsewhere in the corpus, that IS the missing
   evidence, and it is a bounded local query. Cheaper; do this one first.

## What NOT to do

Do not lower the apply job's confidence bar to sweep the medium band in.
The bar is why 83 audited cases produced zero false strips, and issue 312
is what happens when an apply action outruns its evidence.
