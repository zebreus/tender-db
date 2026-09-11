# 378 — the derived EUR column reaches zero by ROUNDING, which the zero sentinel does not touch

Status: ready-for-agent (filed 2026-09-11 by the owner, found while building issue 366's zero drain —
the drain's own termination guard is what exposed it)
Kind: defect (derived values) — small (608 Tenders), but it falsifies the shape of the claim issue 366
just made, not merely its count
Blocked by: nothing

## What was found

Issue 366 decided that a published 0 is an absence and widened `sentinel_amount` to refuse it
(`55d239d`), so a Tender whose only figure is 0 serves **no known value** instead of €0. The drain's
cohort selector needed a published-cents guard to terminate, and that guard is what surfaced the gap:

| | |
| --- | --- |
| Tenders whose head value is exactly 0 | 24,647 |
| …of those, the elected amount is a published 0 (drained by `55d239d`) | 24,039 |
| **…of those, the elected amount is NOT zero — it ROUNDS to zero** | **608** |

The 608 are not touched by the zero leg, because the leg reads the PUBLISHED figure — deliberately, so
one currency's sentinel does not become another's ordinary number — and these published figures are
not sentinels by any rule here. They are tiny:

| currency | published | Tenders |
| --- | --- | --- |
| HUF | 1.00 | 547 |
| RON / PLN / CZK / NOK / DKK / SEK / LTL | 0.01 | 55 |
| CZK | 0.10 / 0.12, HUF 0.79 / 1.48, LIT 2.89 | 6 |

HUF 1.00 is about €0.0025, and the conversion rounds half-away-from-zero to the cent. So the derived
column asserts €0 for 608 Tenders **after** the change written to stop it asserting €0.

## Why this is worth a fix rather than a footnote

**The count is small; the shape is not.** Issue 366's unit 3 removed a payload-versus-filter
incoherence, and the zero decision removed a column-versus-documentation one. This is the same
family a third time: `/docs` now tells readers the derived column treats a 0 as an absence, and for
608 Tenders it does not, for a reason no reader can see from the published value.

It also means `?max_value=0` returns 608 rows rather than nothing. The CHANGELOG entry for `55d239d`
does not claim otherwise (it says the filter stops handing back thousands of contracts that are not
free, which is true), so nothing shipped is *wrong* — but a reader who infers "zero means nothing is
returned" will be surprised, and the surprise is real.

## Two candidate fixes, and the one I would not take

1. **Floor the conversion at one cent rather than at zero** — a positive published amount converts to
   at least 1 cent. Keeps "this Tender has a value, and it is tiny" distinct from "this Tender has no
   value", which is exactly the distinction the zero leg was written to protect. Touches every
   conversion, not just the head election, so it needs the rates tests read first.
2. **Refuse a zero DERIVED value the same way the published zero is refused** — one line at the end of
   the election. Simpler, and wrong for the same reason electing 0 was wrong: it throws away the fact
   that a figure was published.

**Not (2).** A published HUF 1.00 is probably itself a token — 547 Tenders on the identical value is
the same signature that convicted −1.00 and the repdigit maxima — but that is a claim about the
PUBLISHED figure and belongs in `sentinel_amount` with its own measurement, not smuggled in through a
rounding rule. **Whether HUF 1.00 (and the 55 one-minor-unit rows) is a publisher token is the open
question**, and it is answerable the way the others were: count the distinct buyers carrying it, and
read the titles.

## Next unit

Measure the carriers of HUF 1.00 — distinct buyers, distinct subjects, the spread over time — and
decide whether it joins `sentinel_amount` as a token leg. The rounding floor (fix 1) is the separate,
smaller change and does not depend on that answer.

## ADR-0010 is the precedent, and it argues for fix 1 rather than against it

The conversion rounds half-away-from-zero because ADR-0010's 2026-08-22 amendment adopted rounding
for the sub-cent class: *"the canonical layer is a projection of the byte-faithful archived member,
so traceability to the Notice is preserved by the archive while the projection trades ≤ half a cent
of precision for the notices' whole content."* That reasoning is sound and this issue does not
reopen it — the published HUF 1.00 is preserved in `amounts`, exactly as the ADR promises.

What the ADR did not consider is that **one of the rounded outputs is a value with a second
meaning**. Everywhere else, ≤ half a cent of error is invisible. At zero it is not, because zero is
now the derived column's word for "no figure". So the defect is not the rounding — it is that the
rounding's output range overlaps a sentinel.

That is an argument FOR fix 1 (floor a positive published amount at one cent) rather than against
it. The floor costs at most one cent of upward error on amounts already below half a cent, on the
same trade the ADR already made, and it buys back the distinction the zero leg was written to
create. It is also the narrower change: `sentinel_amount` keeps reading published figures only, and
the derived column stops being able to say "zero" for two different reasons.

## The exhibit, read after the zero drain (2026-09-11, rev `55d239d`)

`?max_value=0` no longer returns a single published zero. Its whole answer is now this class, and the
first page reads:

| tender | served `value` |
| --- | --- |
| 207359 | HUF 1.00 |
| 271406 | CZK 0.01 |
| 343991 | HUF 1.00 |

608 Tenders, `more: true`. **A reader asking for free contracts is handed contracts priced at one
forint** — which is a smaller and more legible wrong answer than the 24,039 it used to hand back, but
it is the same wrong answer in kind, and it now has nothing else hiding it.
