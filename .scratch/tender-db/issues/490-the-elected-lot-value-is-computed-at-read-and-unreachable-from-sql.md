# 490 — the elected lot value is computed at read time and unreachable from /v1/sql

Status: ready-for-agent — filed 2026-10-08 from the owner's question about issue 471's scale
rule ("do we do extra calculation on each lookup? Does that work with the sql endpoint? … it needs
to be blazingly fast"). NEXT: unit 1, measure. How many current lots does a naive SQL
`MAX(eur_cents)` serve a figure the REST pick refuses? Use a window read, never a full scan of
`tender_version_amounts`.
Kind: SQL surface / performance / coherence
Relates to: 471 (the exact-10ᵏ rule, unit 4(a)), 389 (lot-level election made coherent with the
tender head, on the REST surface only), 372 (`quality = 'withheld'`), 50 (the analyst views)

## What is true today

**The tender value is stored.** `head_value_eur_cents` (canonical.rs) runs in the fold when it
writes a tender's head pointer. That covers the sentinel test, the ceiling, the exact-10ᵏ
`ScalePartners` rule and the max over tender and lot amounts. The elected figure lands in
`tenders.current_value_eur_cents`, indexed `(current_value_eur_cents, id)`. `/v1/sql` and
`/v1/tenders?min_value=` read that column and pay nothing for the rule. Measured on the box
2026-10-08, server side through `/v1/sql`:

| Query | Time |
|---|---|
| top 5 tenders by `current_value_eur_cents` | 3 ms |
| `COUNT(*)` over a €1 m–€10 m range (984,406 tenders) | 56–66 ms |
| point read by id | < 1 ms |

**The lot value is not stored.** The REST lot rows (`/v1/lots` and the lots of
`/v1/tenders/{id}`) pick their value in memory at read time in `read.rs::summarise`. It takes the
max-cents candidate over the version's amounts after the sentinel, ceiling and (since 471 unit
4(a)) `ScalePartners::refuses_amount` tests. The rule's extra inputs, the chain's amounts and lot
awards up to the version, are queried only when a candidate reaches the €1 bn gate. The REST
cost is small, but it is spent on every read, not once.

**`/v1/sql` has no elected lot value at all.** `v_lots` carries id, tender, key, kind, seq and
title, and no value. An analyst gets a lot's value by aggregating `v_tender_amounts` /
`tender_version_amounts` themselves. Those tables hold every published figure, including the ones
the fold and the REST pick refuse. That is deliberate (refused is not deleted, and the `quality`
vocabulary stays at the source's own `'withheld'`; see the `ScalePartners` doc comment). So a
SQL consumer's natural `MAX(eur_cents) … GROUP BY lot_id` serves the ×10ᵏ error that the
tender head and the REST lot row both refuse. To reproduce the REST figure in SQL, the consumer
would have to re-implement a rule that reads the whole chain. `/v1/sql` is the main way the data
is consumed, so this is the wrong way round. The fast surface has the incoherent figure, and the
coherent figure costs work on every REST read.

## Direction (to decide in unit 2)

Store the elected lot value once, at fold time, the way the tender head already is. Then expose
it on `v_lots` (e.g. `value_cents`, `value_currency`, `value_eur_cents`). `summarise` would then
read a column instead of re-deriving it.

Why it is likely sound: the per-lot pick for version N reads versions 1..=N only
(`seq <= ?` in `summarise`). A later notice therefore cannot change version N's figure, and
storing it on the version row (`tender_version_lots`) does not hit the objection the
`ScalePartners` doc raises against a per-`Fact` `quality` marker (the rule reads the whole chain,
so a later version would rewrite kept ones). Check before building:

- the fold's state key and the unchanged-chain early return. A new derived column must not make
  every kept version read as changed.
- the deadline fallback and the title pick in `summarise`. Store only the value; the rest stays.
- the backfill cost: a refold of all profiles, about 14.9 M notices, the issue-484 drain shape.
- whether an indexed `v_lots.value_eur_cents` should also back `/v1/lots?min_value=`, which
  today filters on the TENDER head column.

## Units

1. Measure the gap on a window: current lots whose naive SQL max differs from the REST pick.
2. Decide the storage shape against the checklist above. Record the decision here.
3. Build it with tests (fold writes it, the view exposes it, `summarise` reads it, and REST and
   SQL agree on a refused-figure fixture). Then gate, deploy, backfill, and do a closing read.
