# 264 — award→notice linkage reads WORST in the youngest eras (sdk-1.14: 45.7 %): decay or age confound?

Status: needs-triage — filed 2026-08-21 (owner, requested by Lennart: investigate data quality).
Kind: data-quality investigation (measurement honesty — the denominator may be time-biased)
Blocked by: —
Relates to: 230 (the measurement), 236/58-v2 (the chaining machinery), 187/188 (the two eras whose
unchained rates are DIAGNOSED as publisher reality — this issue is about the eras that are NOT)

## The anomaly (weekly data-quality run, 2026-08-21, section 2)

Linked share of award Tenders, by EU SDK era — which is also a rough TIME axis:

    eforms-sdk-1.10   75.1 %
    eforms-sdk-1.12   61.3 %
    eforms-sdk-1.13   65.3 %
    eforms-sdk-1.14   45.7 %   ← the newest, and by far the worst

Two readings, opposite conclusions:

- **Decay**: chaining quality is genuinely falling era over era — publishers reference procedures
  less, or a chaining input drifted — and the newest era is the warning.
- **Age confound**: an award links when its contract notice is in the corpus AND the chain has
  been folded; young awards' CNs may still be arriving (a CAN can precede its CN's ingestion in
  daily order) and chains accrete over months. Under this reading every era LOOKED like 45 % when
  it was three months old, and sdk-1.14's number is not comparable to sdk-1.10's at all — which
  would make the section-2 table systematically misleading at its newest rows, the rows an
  operator most reads.

## How to investigate

1. One windowed read: linkage rate for sdk-1.13 notices bucketed by PUBLICATION MONTH. If old
   1.13 months sit at ~65 % and its youngest months at ~45 %, the confound is proven with one era
   (no cross-era noise).
2. If confounded: re-read 1.14 restricted to awards older than ~90 days and compare like-for-like.
3. If NOT confounded (young 1.13 months link fine): diff the 1.14 reference-carrier inventory
   against 1.13's — the issue-100 class (a renamed node id silently unmaps a reference) is exactly
   how one era regresses alone.

## Acceptance

- The confound question answered with the bucketed numbers recorded here.
- If confounded: the report's section 2 gains an age caveat (or an age-restricted column), so the
  newest era's number stops reading as a regression when it is a maturation curve.
- If real decay: a filed extraction issue with the diffed inventory as evidence.
