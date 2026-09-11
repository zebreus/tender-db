# 380 — the sentinel discovery sweep cannot see the bottom of the range, and the reason it gives for not looking is refuted

Status: **DONE 2026-09-11** — built (`5491fc6`), deployed, and run twice on the corpus (jobs 1305
and 1306, 0 labels unmeasured both times). It found a real class on its first run, filed as issue
381. The residual-share column that issue 381 asked for landed in `2130cf4`. Was: ready-for-agent
(filed 2026-09-11 by the owner, after issue 379 found 112,244 Tenders in exactly the region this
instrument declines to search)
Kind: defect (instrument blind spot) — the sweep is the thing that is supposed to find an unimagined
placeholder shape, and the largest one in the corpus sat outside its scope for its whole life
Blocked by: nothing

## What the instrument does, and what it skips

`sentinel_amounts_sql` (issue 366 unit 6) is the discovery half of the sentinel work: the standing
rule holds only shapes somebody imagined, so this groups published `(currency, cents)` by frequency
and ranks them, to find a convention nobody predicted. Its scope:

```sql
WHERE a.cents < 0
   OR a.eur_cents >= 100_000_000_000            -- €100 bn
   OR (a.eur_cents IS NULL AND a.cents >= …)
```

Negatives at every magnitude, and the top of the range. **Nothing between −0.01 and €100 bn**, which
is where issue 379's 112,244 Tenders live.

## The reason given is honest, documented — and wrong

The doc does not hide the gap. It says:

> The cost of the bound is stated rather than hidden: this cannot see a LOW-magnitude sentinel. In
> that region a frequency ranking is dominated by genuine round budgets anyway (measured — see
> `SENTINEL_AMOUNT_FLOOR`), so frequency alone would not identify one there; it would need a
> different discriminator, which is left as the open half of issue 366.

**"Frequency alone would not identify one there" is false, and issue 379 is the counter-example.**
Frequency identified the class instantly and unambiguously — a ranking of head values from 1 to 1,000
EUR cents:

| head (EUR cents) | Tenders |
| --- | --- |
| 1 (€0.01) | **59,030** |
| 100 (€1.00) | **53,214** |
| 10 (€0.10) | 1,463 |
| 200 (€2.00) | 658 |

Two spikes, then a **36× cliff**. No discriminator beyond frequency was needed to see it; the buyer
counts and subtypes came afterwards, to decide what it MEANT, not to find it.

## Why the measurement behind the claim misled

The claim is not careless — it rests on a real measurement, and the measurement is right about what it
measured. `SENTINEL_AMOUNT_FLOOR`'s note records a frequency sweep of the converted column that
"returned nothing but genuine round budgets (€2 M ×10,114, €1.2 M ×8,908, €1.5 M ×7,996)".

**That sweep ranked the WHOLE column.** Over the whole column the top of a frequency ranking is
owned by ordinary round budgets, because ordinary round budgets are the most common thing in a
procurement corpus. The conclusion drawn — "frequency does not work below the floor" — is a
conclusion about the whole range wearing the label of the low range.

**At the bottom there are no genuine round budgets to compete with**, because nothing real costs one
cent. That is exactly why frequency is decisive there and not in the middle. Same instrument, a
different neighbourhood, an opposite answer.

This is the sampling failure `docs/agents/prod-box-reads.md` already names — *pick the sample that
contains the phenomenon, then check that it does* — in its subtlest form yet: the sample was not
wrong about itself, only about the region the conclusion was applied to.

## The fix, and why the state-blow-up argument does not block it

The floor exists for a real reason: `GROUP BY currency, cents` over the full corpus is millions of
distinct values, which is issue 278's state blow-up and issue 337's leaked temp database. **That
argument does not reach a bounded low-end arm.** Distinct `(currency, cents)` pairs under, say, €10
number in the low thousands — the same order as the existing high arm — so a ceiling bounds the state
just as a floor does.

```sql
WHERE a.cents < 0
   OR a.eur_cents >= {FLOOR}
   OR a.eur_cents <= {CEILING}                 -- new: the bottom, bounded the same way
   OR (a.eur_cents IS NULL AND a.cents >= {FLOOR})
```

Open questions for whoever builds it:

1. **The ceiling's value.** €10 covers both 379 spikes with room; €100 would too and costs little.
   Measure the distinct-pair count at each before choosing, the way the floor was chosen.
2. **The `eur_cents IS NULL` arm needs a low twin** (`… AND a.cents <= CEILING`), or every era awaiting
   its conversion backfill leaves the low scope silently — the same quiet narrowing the existing arm
   was added to prevent.
3. **The listing cap is now shared by three regions.** With `SENTINEL_LISTING_CAP` at 40 and one
   ranking, a loud low-end class could crowd out the high end or vice versa. The per-scope quota
   issue 347 built for the census listing is the precedent; a quota per region is probably right.
4. **Expect the known classes to fill it.** 0, 1 and 100 are now refused by `sentinel_amount`, so
   they will dominate the new arm and say nothing. Either exclude what the rule already refuses — at
   the cost of a second implementation of the rule, which this family keeps warning about — or let
   them stand as a visible baseline and quota around them. **Prefer the baseline**: a discovery
   instrument that hides what is already known cannot show you when a known class starts growing again.

## Why this is worth doing rather than noting

Both of this instrument's blind spots have now produced a real class: the converted-column blindness
(a non-EUR sentinel smears off its round published figure) is recorded on issue 366 and still open,
and this one cost 112,244 Tenders that were found by hand. **An instrument whose stated purpose is to
find what nobody imagined should not decline to look where nobody has looked.**

## Closed (2026-09-11)

Built as a SEPARATE query rather than a fourth `OR`, which gave each region its own listing cap —
the per-scope quota of issue 347, reached by separation. Bounded on the published figure and
currency-blind, the inverse of the floor and for the inverse reason. The known classes were left in
rather than filtered, and the section note says why in those words.

**It worked on its first run.** The baseline sat at the top, labelled, and directly under it were
the per-unit rates now tracked as issue 381 — a class the exact-value design cannot reach and which
nothing in the corpus had ever named.

Open questions 1–4 from the filing all resolved in the build:

1. **Ceiling** — 10.00 as published: 3,090 distinct `(currency, cents)` pairs at or below 1,000
   against 22,361 at or below 10,000, so it stays the same order as the high arm's "few thousand".
2. **The `eur_cents IS NULL` twin DISSOLVED.** Bounding on published cents means the arm never reads
   `eur_cents` at all, so an era awaiting its conversion backfill stays in scope for free. The
   question only existed because the high arm's shape was assumed to carry over.
3. **The shared cap** — solved by separation, as above.
4. **The known classes dominating** — they do, exactly as predicted, and that is the design. Issue
   381 then showed the missing piece was not filtering them but NORMALISING past them, which is what
   the residual-share column does.
