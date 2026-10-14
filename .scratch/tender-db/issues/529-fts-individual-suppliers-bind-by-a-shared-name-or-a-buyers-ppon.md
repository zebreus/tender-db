# 529 — FTS `Individual` suppliers bind by a shared name or by a buyer's PPON

Status: ready-for-agent — filed 2026-10-14 from issue 522 unit 1's reads. Next: the measure step (below).
Kind: data quality (organization identity)
Relates to:
- 522 (the s.94 placeholder PPONs stop keying, so their `Individual`s bind by name);
- 511 (placeholder and blank names; guard 3 keeps a `Blank` party with an admitted identifier);
- 452 (identifier verdicts cannot move an exact-triple bind).

## Verify

    echo "SELECT id, country, identifier, provisional FROM organizations WHERE name_norm = 'individual' LIMIT 50" | ssh root@zebreus.click /root/sq.sh

- **done**: no identifier-less `individual` row gathers withheld suppliers across notices. UKRI (30913706) holds no
  mention named `Individual`. Read the second half from the two notices below: their ORG-GB-PPON-PDQJ-7126-JDHG
  mention.
- **open (2026-10-14)**: 2 rows, both identified (31570158 GB `GBPPONPMGL5552GJZX`, 18934187 GH `P0004469968`).
  After 522's dissolve, expect a third: a GB provisional `Individual` holding the four s.94 mentions.

## What was found

Find a Tender publishes a sole-trader supplier under the name `Individual`. Notices 46806664 and 46819715 (one
framework, about 40 suppliers each) carry four `Individual` slots, each under a different PPON:

| PPON | org it binds today | what the PPON is |
| --- | --- | --- |
| `PBXY-3232-WNHZ` | 30914328 "Withheld Section94 Supplier" | s.94 placeholder (522 condemns it) |
| `PGYY-2165-QXNQ` | 31548264 "Withheld Section94 Supplier 2" | s.94 placeholder, 2nd profile (522 condemns it) |
| `PDQJ-7126-JDHG` | **30913706 UK Research & Innovation (3,548 mentions)** | UKRI's own PPON |
| `PMGL-5552-GJZX` | 31570158 "Individual" | unknown: the supplier's own, or another stand-in |

Two problems follow:
1. **A shared name.** Once 522 lands, the s.94 `Individual`s carry no identifier and bind by `(name_norm, country)`.
   - Every withheld sole trader in the UK then lands on ONE provisional `Individual` row.
   - On these two notices that already joins two different suppliers, and their winner rows on one result collapse.
   - Today no identifier-less `individual` org exists anywhere: the Verify read returns only identified rows. So the
     class is new and small, but it grows with every s.94 individual FTS publishes.
2. **A buyer's PPON on a supplier.** UKRI's own PPON is published on an `Individual` supplier, so a withheld
   individual binds UKRI's organization.
   - A 452 `wrong` verdict cannot move it, because the resolver binds the exact triple before verdicts (see 522 unit 1).
   - It is the same shape as 466/452's wrong-identifier class, with a placeholder name as the tell.

## Proposed units

1. **Measure.** Run `name-phrase-census` with `{"phrases":["individual"],"mentions":true}`, an in-app corpus walk,
   when the queue is idle. Count, from its output:
   - whole-name `Individual` mentions;
   - how many carry an identifier;
   - how many of those bind an org whose names never include `Individual` (the UKRI shape);
   - their roles.
   Check first whether the census reports whole-name matches apart from substrings (`Individual Solutions Ltd`).
2. **Decide the name rule from the measurement.** The candidate is `individual` in `BLANK_WHOLE`.
   - Guard 3 then keeps an `Individual` with an admitted identifier (31570158's) and drops a bare one, which covers
     the s.94 ones after 522.
   - `Placeholder` would be wrong: it has no identifier guard, so it would drop identified sole traders too.
   - Cost: a withheld individual winner stops being a party. The lot keeps its other winners.
3. **Decide the buyer-PPON case separately.** A guard needs to know that a PPON is a buyer's, for example an
   `Individual`-named mention whose identifier's org serves only buyer roles. Size it from unit 1 before building
   anything.
