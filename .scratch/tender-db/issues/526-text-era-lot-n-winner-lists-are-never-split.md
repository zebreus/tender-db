# 526 — text-era `Lot n)` winner lists are never split, so one list mints one org

Status: ready-for-agent — filed 2026-10-12 from 520 unit 1 (§4.4). Unit 1 BUILT 2026-10-13 (9a87f98 + the review commit;
measured on 1,508 real bodies, `../526-lot-word-references/dry-run.md`). NEXT: gate, deploy, then the pre-2004 text drain below.
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

## Unit 1 — built and measured (2026-10-13)

- **First build (9a87f98)**: the `Lot n)` word form. An adversarial review found five costly paths:
  - a cue after the comma steals the head;
  - a cut window is read past its comma;
  - the per-lot split mints pointers, statuses and prices (`Lot 2) Idem`, `en cours`, `Lot 1: 12 véhicules`);
  - the `.` split drops an abbreviation's dot (`S.A.` → `S.A`);
  - partly read lists leave `à 4) X`.
- **The review commit** fixes all five, each with a test. A keyed entry's name is held to 520's
  `plausible_winner` bar. The award reader sees the entry WITH its reference, and only when the entry is whole.
  The dot stays on its segment for `trim_sentence_period` to judge.
- **The real-body sample** (`dry-run.md`) widened the reader to what the 1993–2003 print actually holds:
  - number lists (`lots 7, 8, 9 and 10:`, `Lot 1, lot 2 and lot 5:`, `lot 7.2:`, ` :`);
  - long `)` chains (no 24-byte bound for a structured list);
  - bare-digit chains (`10) et 11) Maquet`, about 6,000 `et <n>)` organizations on prod);
  - sub-lot letters (`lot 04) a:`);
  - a second reference (`Lot 9: 1: X`, `2) lots 3) X`);
  - a bare `Lot 3` as no name;
  - a keyed head with no capital as the lot's description.
- **Result on the 912-notice lot sample**: names carrying a reference fell from 4,423 to 208, every new hub is a
  company, and no real winner was lost. The 596-notice control showed no regression.
- **Kept as published**: lettered and Roman labels (`Lot A)`, `Lot III:`, about 250 organizations).

## Drain plan (unit 2)

Text fetches 270 … 401 (2003-12 back to 1992; the fetch map is
`../508-2010-text-vintage/text-fetch-map-1900000-3899999.txt`, extended by notice probes). There are about 1.85 M
notices. Each round is one `reparse` (`profiles:["text"]`, `reclaim_only:true`, `after`, `packages`) followed by
one `project`, sized well under the 500 k FULL-fallback line:

| round | after | packages |
| --- | ---: | ---: |
| 1 | 269 | 13 |
| 2 | 282 | 13 |
| 3 | 295 | 15 |
| 4 | 310 | 17 |
| 5 | 327 | 18 |
| 6 | 345 | 17 |
| 7 | 362 | 18 |
| 8 | 380 | 21 |

- **Overlap is harmless**: a reparse is idempotent. A gap is not: check that each job's covered fetch ids reach the
  next round's `after`.
- **Per round**: `unmatched` ≈ 0 and `re-keyed` 0. The change does not touch `publication_id`, so it does not race
  the daily ingest (issue 404).
- **The auto orphan sweep** queues behind the first fold that re-binds anything. It caps at 10,000; above that
  it records a plan, then dry and wet by hand.
- **After the drain**, re-read the baseline counts in `dry-run.md` and the Verify below.
