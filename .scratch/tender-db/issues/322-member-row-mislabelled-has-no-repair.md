# 322 — The `member-row-mislabelled` class has a verdict but no repair

Status: CLOSED 2026-09-06 (owner board sweep) — the premise dissolved on the 2026-08-31 re-measure: ~2 decisive cases are an exception list, the 22 vehicle-named cases belong to the Stage-4 name keys / 321, and nothing here wants a job. Reopen rule at the bottom. Was: RE-MEASURED 2026-08-31 — the premise has largely dissolved.
RECOMMENDATION: do NOT build the rename machinery; see the distribution
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


## RE-MEASURED (2026-08-31, from the live `fusion-candidates` report, 10.6 h old)

No new job needed: `FusionCandidate` already carries the judgeable denominator
(`mentions`), the off-name count, and the off-name GROUPS with counts. The
ratio this issue asks for is `groups[0] / mentions`, and it was computable from
the report already on the box.

**The census now finds 45 fused cases, not 58, and the exemplars have moved:**

    org 13011025 (this issue's 28-of-29): still present, now 26 of 29 → 0.90
    org  9610149 (this issue's 20-of-21): GONE from the fused set entirely

9610149 dropped out because the issue-317 Unit A campaign re-homed its Dobler
mentions off it. `truncated: false`, so this is the complete set — the row is
no longer fused at all. **The 317 re-homing WAS the repair for that case.**

### The distribution, which is the finding

    dominant-share  cases
    >= 0.90             2   (incl. 13011025 at exactly 0.90)
    0.75 - 0.90         2
    0.50 - 0.75        28
    < 0.50             14

    cases with fewer than 5 judgeable mentions:                    26 of 45
    cases whose DOMINANT off-name is itself consortium-shaped:      22 of 45

The 0.50 bucket is almost entirely **2-of-4**. That is not the 28-of-29 shape
this issue was written around; it is a coin flip on four mentions. And in 22 of
45 cases the dominant off-name is another VEHICLE spelling —
`'Bietergemeinschaft bestehend aus…'`, `'ARGE Pöyry Infra GmbH & 3G…'`,
`'BIEGE Seidlbau Tulln BaugesmbH/Leyrer+Graf…'`. Renaming a vehicle to a
different vehicle name is not the repair described above; it is a spelling
variant problem wearing this issue's clothes.

## Recommendation: do not build it

The rename machinery this issue specifies — ratio floor, denominator rule,
name-choice review gate, identifier policy — would be built for a population of
**two**, one of which sits exactly on any plausible floor. Machinery with a
threshold, a review schema and an undo path, for two rows, is worse than a
hand-reviewed exception: the threshold itself becomes a thing to maintain and
to be wrong about.

What the measurement says instead:

1. **The 317 campaign already repaired most of this class.** Moving mentions
   off a vehicle and letting the fold re-derive is what "the row was carrying
   someone else's mentions" needed, and it worked — one of the two exemplars
   here is gone because of it.
2. **The ~2 decisive cases are an exception list, not a job.** 13011025 at
   26-of-29 is a real finding and deserves a rename; it does not deserve a
   framework.
3. **The 22 vehicle-named-dominant cases are a different issue** — variant
   spellings of the same consortium, which is what the Stage-4 name keys and
   the 321 satellite work address. They should not be counted as
   member-row-mislabelled at all, and counting them was what made the class
   look big.

If a future re-measurement shows the >= 0.90 bucket growing past a handful,
this issue is the place to reopen — the ratio is cheap to recompute from a
report that already exists.
