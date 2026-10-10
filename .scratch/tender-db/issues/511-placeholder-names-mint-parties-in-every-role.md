# 511 — placeholder names (pointers, withheld, `Various`, award summaries) mint parties in every role

Status: needs-triage — filed 2026-10-10 from issue 510's unit-1 decision (§9, `../510-void-names/unit1-decision.md`),
which split non-names into two classes and took only `VoidLot` ("the lot was not awarded"). This is the other class.
Kind: data quality (organizations; all eras)
Relates to: 510 (the `partyname::not_a_name` predicate and its `Placeholder` class), 484 (`NON_NAME_FOLDS` in
`project/role_census.rs`, a fold-level placeholder list for the buyer-equal test), 483 (buyer-slot fixes)

## What is wrong

A party slot that names no organization — a pointer (`See section VI.3`, `Voir autres informations`, `Véase perfil del
contratante`, `See Contracts Finder Notice for full supplier list`), a withheld name (`Not applicable`, `Withheld
Section94 Supplier`, `Keine Angabe`), a summary (`Various`, `Various suppliers`, `Multiple Suppliers`, `Diverse
Firmen`) or a relaunch summary (`Lot déclaré infructueux … puis attribué à la Société X`) — is minted as an
organization in every role.

510's census (2026-10-10): 2,952 orgs, 5,672 mentions. Unlike the void class, these are mostly NOT winners:
review-body 1,782, mediation-body 1,252, winner 1,214, appeal-information 403, Lot-ReviewOrg 293,
RESPONSIBLE_FOR_MEDIATION_PROCEDURES 194, purchasing-body 172, specifications-provider 160, further-information 46,
Tenderer 46, tender-receipt 35, **buyer 20**.

## The semantics differ from 510

A winner was chosen; its name is elsewhere or withheld. So a result keeps `selec-w`, but nothing should be bound as
the winner. A buyer-slot placeholder changes buyer tokens and grouping, so dropping one needs its own decision.

## Units

1. **Decide** per role: drop the party (winner, review/mediation slots), keep it, or map it (a buyer placeholder).
   Unify `NON_NAME_FOLDS` with `partyname::Placeholder`. Carry 510's deferred list (`neant`, `ingen`, `n a`, `none`,
   `nie dotyczy`, `entfällt`, `keine Angabe`, `brak`, `sans objet`, `aucun`, `nessuno`, `varios`, `diverse`,
   `various suppliers`, `multiple …`, `withheld …`), each with its role split and its real-name traps (`InGen`,
   `NIL d.o.o.`, `N/A s.r.o.`, `Diverse Care Services`).
2. **Build** on 510's module and retire path.
3. **Drain** with 510's `refold-void-names` job generalised, then the 443 sweep.
