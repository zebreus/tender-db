# 526 — text-era `Lot n)` winner lists are never split, so one list mints one org

Status: ready-for-agent — filed 2026-10-12 from 520 unit 1 (§4.4). NEXT: unit 1 build (S–M), then the 1997–2003 reparse.
Kind: data quality (text parser; organizations)
Relates to: 520 (unit 1 §4.4, `../520-award-summaries/unit1-decision.md`) (award summaries; its A1 extractor `partyname::award_winner`), 484/485/244 (`lot_prefix_len`, `winner_segments`), 491 (the reparse path), 510/511 (`plausible_name` refuses every non-name).

## What is wrong
`lot_prefix_len` (text/parse.rs:540) accepts a `)` reference only when the head is all digits. So `Lot 1)` is never
stripped, and `winner_segments` (:660) never splits at `. Lot 2)`. One list becomes one org (31735007
`Lot 1) Blin. Lot 2) Prodirest. …`, 20187-2002), and every prefixed name splits an identifier-less company
(`Lot 1) SARL Bonsignore & Olivetta` 30587843 beside `Sarl Bonsignore & Olivetta` 25920173). Since a3153bc, a reparse
refuses a void-bearing list whole (`plausible_name`, :148), so its real winners are lost.
Size (520 triage, 2026-10-12, name_norm ranges): 5,239 orgs start with `lot 1)`, 994 of them lists and 17 with a void word;
141 bare `Lot 1)` and 19 bare `lot 2)`; about 92% text era, mostly 1997–2003. 520 unit 1 classed 29 void-bearing lists (its App. A, C).

## Measured variants (520's 29)
`Lot n)`; `Lote n)` (30687978); `Lot n° n)` (31740189); `Lot 8bis)` (30689270); `Lots 1) et 2)` and the typo `Lor 4)`
(30690240); Roman `Lote I:` / `Lote II:` (26966868); lettered `A)`, `B1) et B2)` (30695615); headings
`Marché <category>: A)` (31731990, 31731993; 1801614 lost Eurocash), `Marché n:` (31734422), `Procédure de marché négocié
après appel d'offres infructueux:` (31689502, 31689490); `lot n) la société X a été retenue` (31738609 and siblings);
a list cut at `Lots 2)` (30698459; TXT-CO goes on `Lots 2), 3) et 4) SA EVM - Les Pépinières des Vallées`).

## Fix
1. `lot_prefix_len`: in the `)` and `:` branches, accept an optional `Lot`/`Lots`/`Lote`/`Lotes` word (any case) and an
   optional number mark (`n°`, `nº`, `no`) before the digits, a `bis` or one-letter suffix, and the multi-reference
   `Lots 1) et 2)` / `Lots 2), 3) et 4)`. Stay within LOT_PREFIX_MAX (24).
2. `winner_segments` then splits at `. Lot m)`.
3. Where `plausible_name` refuses an award-summary segment, take `partyname::award_winner(segment)` when it names one
   (`la société Storz a été retenue` → `Storz`).
4. Lettered and heading shapes, and cut lists: refuse as today or keep the segment. NEVER a `VoidLot` drop: these results
   carry awarded lots.

## Tests
`Lot 1) X. Lot 2) Y`; `Lots 2) 8) 20) X`; `Lote 1) Declarado desierto. Lote 2) Limpiezas Luz`; `Lot n° 3) X`;
`Lot 8bis) Willecocq`; a bare `Lot 1)` mints nothing; `Lot A)`, `Lot unique: X`, `LOT Polish Airlines` and
`Lot-et-Garonne` keep their text; the existing `1) CGC` (:3383).

## Drain
Reparse the 1997–2003 text fetches (270+) through 491's path: a dry run first, then read a stride sample of the
re-minted winners, then the wet run and the 443 sweep. This must land before any other pre-2004 reparse.

## Verify
`curl -s "https://tenders.zebreus.click/v1/organizations?name_prefix=Lot%201)%20Blin&limit=10" | jq '.items|length'`
Open: 1 (31735007, 2026-10-12). Done: 0.
