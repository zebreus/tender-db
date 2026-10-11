# 521 — "nobody" words in a winner slot (`ingen`, `néant`, `geen`, `aucun`, `niet van toepassing`) may mean "not awarded"

Status: needs-triage — filed 2026-10-12 with issue 511 unit 2 (decision §8.7, owner ruling 16).
Kind: data quality (results)
Relates to:
- 511 (these words are `Blank`, so the existing evidence rule decides);
- 510 (`VoidLot` and its per-word value/date table).

## What is open

511 classes the "nobody" words as `Blank`. With that class, a legacy result whose only winner slot reads one of them
is decided by the existing evidence rule:
- a value → `selec-w`;
- no date → `clos-nw`;
- date only → NULL.

Promoting a word to `VoidLot` would mark those results `clos-nw` outright, outranking a value or a date. That is
correct only if the word, in a winner slot, means "the lot was not awarded" rather than "the name is missing".

Census rows in winner slots (511 census, 2026-10-11): `ingen` 461 admitted, `néant` 458, `niet van toepassing` 117,
`geen` 110, `aucun` 100.

## Proposed unit

Build 510's per-word table for these words. For each word, the count of its winner-slot results that carry:
- a value (and how many are exactly 0);
- a date only;
- neither.

Then decide per word, as 510 did for its whole values: a word whose results are mostly value-less and dateless joins
`VoidLot`.
