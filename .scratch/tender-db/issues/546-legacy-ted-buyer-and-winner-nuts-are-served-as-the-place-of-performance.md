# 546 — legacy TED serves the buyer's and the winner's address NUTS as the tender's `country`, so `?country=` over-matches

Status: needs-triage — filed 2026-10-15 from issue 532's unit-1 decision (D7), which confirmed the precedent and left
it out of 532's scope. Next: measure how many legacy tenders serve a buyer/winner-only NUTS, then decide.
Kind: API semantics (the `country` field and `?country=` on legacy TED)
Relates to:
- 532 (country-only place; D7 records the interaction: a carried legacy buyer NUTS suppresses a later eForms
  `place_country` row, a miss, not a wrong value);
- 48 (`?country=` is a NUTS place-code prefix);
- 437 (FTS award-item region).

## Verify

    curl -s --max-time 20 "https://tenders.zebreus.click/v1/tenders/8432519" | python3 -c "import json,sys; print(json.load(sys.stdin)['country'])"

- **done**: `['FRK22']`, the performance NUTS only. The fix may also give a documented alternative, e.g. a separate
  buyer-region field.
- **open (2026-10-15)**: `['FRI12', 'FRK22']`. FRI12 is the buyer's (ORG-1 `TED-NUTS`, PROCEDURE `TED-CA_CE_NUTS`);
  only FRK22 is `TED-PERFORMANCE_NUTS`.

## What was found

`CLASSIFICATIONS` maps `TED-NUTS`, `TED-CA_CE_NUTS` and `TED-TENDERER_NUTS` to field `place`, scheme `nuts`
(crates/ingest/src/project.rs:361-366). This includes `TED-NUTS` inside ORG sections. r209/rules.rs:390 parses them
as Nuts. So in the legacy era a tender's `country` is the union of three things:
- the place of performance;
- the buyer's region;
- the winner's region.

eForms and DÖE serve only the place of performance. `country` therefore means different things in different eras.
`?country=FRI` finds 8432519 although its works are in FRK22. This is partly why legacy TED looks fuller than eForms
in the empty-`country` rate (532's sample: TED 3.6% vs eForms 15.5%).

The /docs `country` row (docs.rs:170) says "NUTS place codes" and does not mention buyer or winner addresses.

## Units

1. **Measure (S).** Over a bounded legacy window, count:
   - tenders whose `place` rows come only from buyer/winner fields;
   - tenders with both kinds of row;
   - tenders with performance only.

   This needs the field id per row. `tender_version_classifications` stores only `field='place'`, so read the
   notice-layer ids for the window.
2. **Decide.** Options:
   - Re-map the buyer/winner ids to a distinct field (e.g. `buyer_place`), which `?country=` stops matching unless the
     version has no performance place. That is the 532 D4 shape: fallback only on empty.
   - Keep the behaviour and document it per era.
   - Drop the buyer/winner rows.

   A re-map needs a legacy refold (about 7M tenders), so size it first.
