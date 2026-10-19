# 572 — a buying agent's or consortium's identifier binds the member bodies it buys for (and vice versa)

Status: ready-for-agent — filed 2026-10-19 from issue 545 unit 2a (wf_fde95594-dbf: `.scratch/tender-db/545-triage/u2a/`,
class "A", 52 kept rows / 53 keys / 79 mentions in the GB-PPON half alone). Next: unit 1, measure the class and decide its
semantics (it is NOT 545's one-off copy-paste class, so 545's reviewed rehoming does not apply).
Kind: data quality (organization identity, buyer side)
Relates to:
- 545 (the GB-identifier-names-another-organization class; 2a split this pattern out as "A", agent and principal);
- 456 (a mention that publishes another org's identifier binds to that org);
- 482 / 481 (buyer clusters and links), 513 / 519 (NHS bodies).

## Verify

    (unit 1 defines it: e.g. the count of member-body names held under Northern Housing Consortium's identifier)

- **done**: unit 1's decision is applied and its Verify reads clean.
- **open (2026-10-19)**: 545 2a's A-class rows bind member bodies' names under the agent's identifier.

## What was found

In the GB-PPON half of 545's candidate listing, 52 kept rows are a buying agent or consortium and the body it buys for:
Northern Housing Consortium's member councils and housing associations (34 keys), NWSSP on Welsh health boards, Star,
Ardal, Welland, and brokers. The publisher prints the principal's name with the agent's identifier (or the reverse), so
the mention binds to the identifier's holder whatever its name says. Each such pair is systematic (the same agent across
many notices), unlike 545's one-off copy-pastes, so rehoming each mention would fight the publisher every week.

## Proposed units

1. **Measure and decide (S–M).** Size the A class across GBPPON and GBCOH (reuse 545's candidates.py with the A labels) and
   read how the buyer/agent relation is published (eForms BT-765/role codes, "on behalf of" fields, the agent as
   Organization with a "procurement service provider" role). Decide: serve the principal as buyer and the agent as a
   separate role (if the data carries the role), park, or leave as published. Record the decision.
2. **Build (M)** per the decision, then a cohort drain.
