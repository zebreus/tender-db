# 322 — The `member-row-mislabelled` class has a verdict but no repair

Status: ready-for-agent (measured by the issue-317 Unit A census, 2026-08-30)
Kind: data quality (organization layer)
Relates to: 317 Unit A (re-homing), 311 (produced the verdicts), 300 Stage 4

## The shape

The 311 campaign returned 58 cases as `member-row-mislabelled`, and the Unit A
`fusion-census` reproduced the shape from the data without being told about
it: org 13011025 holds **28 of 29** judgeable mentions naming
`Dipl.Ing. Wilhelm Sedlak Gesellschaft m.b.H.`; org 9610149 holds **20 of 21**
naming `Dobler GmbH & Co. KG Bauunternehmung`.

At that ratio the row is not a consortium vehicle that captured a member's
mentions. It IS the member's row, wearing a consortium name — because the
first notice the resolver saw spelled it that way, and `organizations.name` is
the head it minted from.

## Why re-homing is the wrong repair for it

`apply-rehoming` (issue 317 Unit A) moves mentions off a row. Applied to a
28-of-29 case it would move 28 mentions to some destination row and leave a
one-mention husk behind under the consortium name — while the destination it
moved them to is, in the cases the census surfaced, frequently a row that does
not exist. The repair is the other direction: **rename the row to the member**,
and let the vehicle be minted separately if a notice ever needs one.

So this is not a variant of Unit A. It is a different write, on a different
column, gated by a different piece of evidence — which is why Unit A's
`org_mention_rehoming` deliberately admits only `rehome` and `keep`.

## What a repair would have to establish

1. **That the row is the member's, not the vehicle's.** The ratio is the
   signal, but it needs a floor and a denominator rule — 28-of-29 is decisive,
   3-of-4 is not, and `mentions` already excludes unjudgeable names.
2. **Which name to rename it TO.** The dominant group's most-published
   spelling is the obvious answer and is probably right; it is also how a
   typo becomes an org's identity, so it wants a review gate, not a rule.
3. **What the identifier means afterwards.** These rows reached the campaign
   because they carried an identifier the reviewer judged wrong, and 311
   already stripped some of them. A rename does not re-open that.
4. **What happens to the vehicle.** Nothing, in v1: minting a consortium row
   nobody asked for is the same invention Unit A refuses.

## Cost of leaving it

The row keeps a name it does not have, so `organizations.name` — the field the
API serves and the Stage-4 keys are built from — is wrong for 58 known rows,
and the N2/N3 keys built from that name go on matching the wrong things.
Unit A's review campaign will also keep meeting these cases and having no
verdict that fits, which is how a class gets flattened into `keep` and lost.

## Do not

Do not fold this into `apply-rehoming` as a third action. A rename writes a
published entity field on evidence of a different kind; it needs its own
dry-first plan and its own panel round, exactly as the move did.
