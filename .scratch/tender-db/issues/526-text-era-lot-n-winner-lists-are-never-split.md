# 526 — text-era `Lot n)` winner lists are never split, so one list mints one org

Status: ready-for-agent — UNIT 3 DONE 2026-10-14 (deployed 6792e5f; drain 2449–2486 all 0/0/0; auto sweep 2487 swept 8,523 of 8,523; the five exhibits read as built; residue `lots ` 1,016 → 667, `lot a`–`lot z` 1,338 → 869). NEXT: the residue list under "Residue left (unit 3's own)" if a unit 4 is warranted. Was: ready-for-agent — filed 2026-10-12. Units 1 (pre-2004) and 2 (2004–2010) are DONE. Unit 3 (lettered and
Roman labels, `à <Company>`, address columns, `3M`-style keyed names; see below) is BUILT, reviewed in three adversarial
rounds, gated green (148 suites) and DEPLOYED 2026-10-14 10:09 UTC as 6792e5f. Its DRAIN is QUEUED as jobs 2449–2486:
19 reparse + project rounds over fetches 186–401 (2004–2010 first, then pre-2004). NEXT: per round, read `unmatched`
and `re-keyed` (both 0); then the auto sweep (by hand if over 10,000), the residue counts and the exhibits below.
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

## Drain log

- **Deploy.** 214ab64 deployed (gate green, 148 suites).
- **Probe 2397** (`text after 291`, 1 package): fetch 292 re-parsed 16,472 notices, with 0 unmatched, 0 re-keyed and
  0 now failing. 109 text packages remain past it, so text runs through fetch 401.
  - It stamped 2,830,901 Tenders epoch-stale. That is per profile, as documented.
  - The exhibit 1831781 (20187-2002) now serves Ame Haslé, Geffrault, Brake France, Blin, Prodirest, Ouest
    Distribution ×2, Corol Jeager, Prodirest, Blin and Prodirest. The void lots name nobody.
- **Rounds queued.** Each round is a reparse followed by one project. They are 2398/2399 (after 269, 13
  packages), 2400/2401 (282, 13), 2402/2403 (295, 15), 2404/2405 (310, 17), 2406/2407 (327, 18), 2408/2409 (345, 17),
  2410/2411 (362, 18) and 2412/2413 (380, 21).
- **Drain finished at about 21:45 UTC.** The eight reparse rounds (2398, 2400, 2402, 2404, 2406, 2408, 2410 and
  2412) re-parsed 255,469 + 215,264 + 216,378 + 245,351 + 240,018 + about 216k + about 190k + 136,521 notices. Every
  round read 0 unmatched, 0 re-keyed and 0 now failing.
- **The eight folds** (2399–2413) re-bound 7,209, 10,840, 13,322, 11,001, 6,269, 3,164, 2,036 and 655 mentions:
  54,496 in all.
- **The auto sweep 2414** stopped at its cap. It counted 36,130 orphans to sweep, all country-less, plus 10 kept by a
  review table, and recorded the plan. The wet sweep **2415 swept 36,130 of 36,130**.
- **Exhibits.**
  - 1831781 (20187-2002) serves one winner per lot.
  - 2017396 (3118-2003) mentions Entreprise Malet (3138733) and Société Colas Midi Méditerranée (6764874), both
    existing organizations, which merged; Sacer Atlantique/Screg Sud Ouest, which is new; and Canton Ouest,
    which is known residue.
- **Verify** `name_prefix=Lot 1) Blin`: **0** (it was 1).
- **Baselines** (bounded `name_norm` range counts):

  | prefix | before | after |
  | --- | ---: | ---: |
  | `lot 1)` | 5,239 | 6 |
  | `lot 2)` | 1,487 | 14 |
  | `lot 3)` | 999 | 10 |
  | `lots ` | 8,929 | 1,363 |
  | `lote 1)` | 109 | 0 |
  | `lot n` | 192 | 177 |
  | `et 1` … `et 9` | 5,987 | 48 |
  | `-1` | 147 | 8 |

## What remains (units 2 and 3)

- **Unit 2: the 2004–2010 sectioned text era**, which is not in this drain. Of the 1,363 remaining `lots ` orgs, 742
  first appear there (2004–2010 ids). So do the remaining `lot 1)` / `lot 2)` names, for example
  `Lot 2) V.1) Award and contract value V.1.1) Name and address …` (2306702) and `lot 2) bière et cidre. V.1) …`.
  - First, measure on a local body sample (harness as in `dry-run.md`). The sectioned form is where the V.4 pairing
    and the CONTRACT NO / LOT NO blocks apply, and the third review found the empty-slot duplicate there.
  - Then drain fetches 186–269. 509's rounds are the template: after 198×8, 206×8, … 263×6, plus 186–198 for
    2009-12 → 2010.
- **Unit 3: pre-2004 shapes the reader leaves as published** (405 of the remaining `lots ` orgs). Each needs its own
  measurement before a rule:
  - no terminator: `Lots 1 à 5 - Taxis Doublet`, `Lots 1, 2, … et 10 Taxi JMC`;
  - `à <company>`: `Lots 117-118-252-271 à Orgentec`;
  - a spaced `: ` after a dash list: `Lots 110 - 203 - 212 : Cook France`;
  - 4- or 5-digit lots: `lots 3300 à 3305`, `lots 21037) et 21038) Lupetti`;
  - dotted lists: `Lots 15.19.20.21 …`;
  - bare references: `Lots 39`, `lots 2`;
  - lettered and Roman labels (`Lot A)`, `Lot III:`, about 250 orgs);
  - a subject followed by an unmarked company (`Champagne`, `Canton Ouest`).

## Unit 2 — the 2004–2010 sectioned era measured (2026-10-14)

The method is the same as `dry-run.md`. The data is `../526-lot-word-references/u2-sample-{lot,ctrl}.jsonl`, plus
the stored `RES` figures pulled beside them.
- **Lot sample:** 800 notices, the first-mention notices of `lot*` organizations with ids from 2.24M to 4.5M.
- **Control:** 529 notices, every 4,000th text id in the same range.

**Results.**
- **Control:** the names are identical in all 529 notices (519 unchanged and 10 reordered), and so are the figures
  (529 of 529).
- **Lot sample:** 0 clean names are lost, and no gained name lacks a capital. Names that keep a lot reference fall from
  1,709 to 785. Every name in 3 or more notices is a company, among them `LOT-ORIEL`, a real firm that stays whole.
- **Figures:** 783 match. In 16 notices a figure is lost: it was attached to a junk name (`Lot 10`, `Lot 2:
  collecte`, `Lot 01: terrassements`), or the block now holds two winners (`Extreme Gaming Arena` and `Avanzit
  Tecnología`, a joint award). 1 notice gains a figure. Spot checks show that each contract block pairs its one name
  with its own V.4: 3994805 gives `Lots 1 et 4 à Ansell` 138,747 EUR and `Lot 16 à CAIR LGL` 14,400 EUR.
- **Residue that is not a 526 regression:**
  - relaunch and status phrases (`Lot relancé en marché négocié …`, `Lot reconsulté en Mapa`, `Lot en cours
    d'analyse`) belong to the 510/511 lexicon;
  - run-on form items (`Lot 24 V.1) Award and contract value …`) were in the old parse too;
  - dash shapes (`Lot 1 - Aksel Sp. z o.o.`);
  - `Lot(s) <list> à <Company>` (`Lots 2, 5 … et 27 à Asept In Med`) occurs in 9 of 800 lot bodies and 0 of the
    control, so it stays in unit 3.

**Drain queued (on 214ab64).** Each round is a reparse followed by a project, after × packages:

| round | jobs | after × packages |
| --- | --- | --- |
| 1 | 2416/2417 | 185×7 |
| 2 | 2418/2419 | 192×6 |
| 3 | 2420/2421 | 198×8 |
| 4 | 2422/2423 | 206×8 |
| 5 | 2424/2425 | 214×8 |
| 6 | 2426/2427 | 223×8 |
| 7 | 2428/2429 | 231×8 |
| 8 | 2430/2431 | 239×8 |
| 9 | 2432/2433 | 247×8 |
| 10 | 2434/2435 | 255×8 |
| 11 | 2436/2437 | 263×6 |

The rounds follow 508's and 509's sizing (106k–237k notices each). The 07:35 dailies queue behind them.

## Unit 2 — drain log (2026-10-14)

- **Rounds 2416–2437** (11 reparse + project pairs on 214ab64) re-parsed 234,469 + 187,532 + 248,655 + 245,571 +
  239,390 + 219,834 + 197,404 + 187,768 + 175,041 + 164,951 + 117,139 = **2,217,754 notices**. Every round read 0
  unmatched, 0 re-keyed and 0 now failing. The fetch ids are contiguous: each round's `after` is the previous round's
  last package, ending at 269.
- **The folds re-bound** 102, 274, 233, 514, 197, 202, 214, 362, 457, 371 and 362 mentions: **3,288** in all.
- **The auto sweep 2438** stayed under its cap. It walked 5,678,631 organizations and swept **2,203 of 2,203**
  orphans (all country-less), keeping 10 by a review table. No sweep by hand was needed.
- **Residue** (bounded `name_norm` range counts, 06:50 UTC):

  | prefix | after unit 1 | after unit 2 |
  | --- | ---: | ---: |
  | `lot 1)` | 6 | 0 |
  | `lot 2)` | 14 | 0 |
  | `lot 3)` | 10 | 0 |
  | `lots ` | 1,363 | 1,016 |
  | `lote 1)` | 0 | 0 |
  | `et 1` … `et 9` | 48 | 48 |
  | `-1` | 8 | 8 |
  | `lot a` … `lot z` (whole range, words too) | 1,450 (05:50, mid-drain) | 1,338 |

  - `lot n` (`'lot n'` to `'lot o'`) reads 280. That is not comparable with the 177 above, which was a narrower range.
    By first mention, 101 are structured-era, 131 come from 2004–2010 and 48 are pre-2004, almost all old
    organizations: `Lot No 1 — Telelink AD`, `Lot n°1 Quadria`, `Lot non encore attribué`. A few were minted by this
    drain, `Lot not applicable. V.5) The contract is likely to be subcontracted: No` (3426018) among them.
  - **`lots ` by first mention:** 404 pre-2004, 396 from 2004–2010, 216 structured.
    - In 2004–2010 the largest shape is `Lots <list> à <Company>`: 161 organizations.
    - Dash or colon lists come next, with 89.
    - Pre-2004, 142 are lettered labels, which unit 3a reads; 38 are 4- or 5-digit lots and 29 are bare references.

## Unit 3a — lettered and Roman labels, measured (2026-10-14)

- **Scope.** In the `lot a`–`lot z` range, 344 organizations carry a lettered label and 57 a Roman one (by regex).
  Of those 401, 377 first appear pre-2004, 23 in 2004–2010 and 1 is structured.
- **Rule.**
  - `lot_number_end` falls back to `lot_label_end`: one letter plus up to two digits (`Lot A)`, `lot B21)`, `lot
    C7b)`, `lot M00.14)`), or 2–5 Roman letters (`Lot III:`). No letter may follow (`Lot Atlas`, `Lot IVECO`).
  - `keyed_name` stops at a field labelled with the next lot (`…, SA, Lote II: …`) and hops a parenthesised subject.
  - `Lote II::X` is one terminator.
  - The heading rule skips entries after a lot-word entry and entries with their own label.
  - `keyed_bar` reads a name opening on a company's digits and capitals (`3M France`) and a quoted company with a
    legal form (`'Sagem' SA`).
- **Sample.** 217 first-mention notices of those organizations were pulled with `pull.py` and kept in the session
  scratchpad. The harness is the one in `../526-lot-word-references/dry-run.md`.

  | sample | notices changed | names | lettered names | figures |
  | --- | ---: | --- | --- | --- |
  | letter sample (217) | 210 | 705 → 902 | 453 → 0 | 9 → 9 |
  | unit 1 lot sample, pre-2004 (912) | 33 | 5,530 → 5,555 | 84 → 0 | — |
  | unit 1 control (596) | 1 | | | |
  | unit 2 lot sample, 2004–2010 (800) | 19 | 4,045 → 4,054 | 6 → 0 | 812 → 810 |
  | unit 2 control (529) | 0 | | | |

  - **Unit 1 control:** the one change splits 1468001 into its four groupements. In this first build each name
    still carried the `groupement d'entreprises:` label; the second review fixed that, and the shipped build names
    `Falconnet Denis` and the other three.
  - **Unit 2 lot sample:** the 2 figures move off blocks that now name both of their winners (`Lotes I y II:
    Señalizaciones Villar … Lotes III y IV: Postigo`, 3099855), as in unit 2.
- **Found while measuring: a unit 1 loss.** `plausible_winner` opens a name only on a letter. So unit 1's keyed
  entries refused `3M France` and `3M Santé`, which the old parser had kept inside a junk `Lot 2) 3M France`. The
  pre-2004 sample shows 11 such notices (`3M` ×2, `3M France` ×4, `3M Santé` ×5). They come back under unit 3a's
  `keyed_probe`, and so do `2SDA`, `4S Bilgisayar … Ltd. Sti`, `'Helisota' Ltd` and `'Sagem' SA`.
- **Residue left as the numbered form leaves it:**
  - `X à <town> (NN)`: `plausible_winner` refuses another party's `à` (520), and 1750421's 8 junk names now read
    as none;
  - `X et lot E5) Y` (1562647);
  - a contract number before the company (`Lot B) marché n° 2002/106CUMPM. Bergerat Monnoyeur`);
  - two-letter and slash labels (`PB)`, `EN.01)`, `A.1/01)`);
  - ` / ` chains (`Lot A) / lot B) / lot C):`);
  - ordinals (`36e heure`);
  - quoted names with no listed legal form (`'Donserv'`, `'Nota Bene' UAB`).

## Unit 3 — built, reviewed, gated, deployed (2026-10-14)

- **Reviews.** Three adversarial rounds, each folded in before the next:
  - wf_aeb4902e-97e: 27 confirmed, 19 caused by the change;
  - wf_3318a4b4-ffb: 41 confirmed, of which 1 major (the address stop missed 3500348);
  - wf_58082746-03b: 15 confirmed, all minor or nit.
  The rules each round added or narrowed are listed in commit 6792e5f's message and in the unit 3 test
  (`a_lettered_or_roman_lot_label_is_a_lot_reference`, one case per rule, each with a real notice id).
- **Final measurement** (n17 against main):

  | sample | notices changed | names | figures |
  | --- | ---: | --- | --- |
  | letter sample (217) | 210 | 705 → 927 | 9 → 8 |
  | unit 1 lot sample, pre-2004 (912) | 79 | 5,530 → 5,655 | — |
  | unit 1 control (596) | 4 | 354 → 367 | — |
  | unit 2 lot sample, 2004–2010 (800) | 98 | 4,045 → 4,174 | 812 → 808 |
  | unit 2 control (529) | 0 | | 281 → 281 |

  - **Letter sample:** names that keep a lettered label fall from 453 to 0.
  - **`à` sample (16 notices):** names that keep a lot reference fall from 197 to 11 (OCR-broken lists,
    lower-case companies).
  - **Unit 1 control:** the 4 changes are Italian `Lotto`/`Lotti` lists now split (1242001: `Lotti 1` → 6
    companies) and 1468001.
  - **Unit 2 lot sample:** every figure lost sits on a block that now names two or more winners, as in unit 2.
    3776831 and 3994805 gain 6 and 4 figures.
- **Gate** green at 6792e5f (1,506 s, 148 suites). **Deployed** 10:09 UTC: health 200, rev 6792e5f.
- **Drain queued (on 6792e5f).** Each round is a reparse followed by a project, after × packages, reusing unit 2's
  and unit 1's job files (`/root/526u2/r*.json`, `/root/526d/r*.json`):

  | jobs | era | rounds |
  | --- | --- | --- |
  | 2449–2470 | 2004–2010, fetches 186–269 | 185×7, 192×6, 198×8, 206×8, 214×8, 223×8, 231×8, 239×8, 247×8, 255×8, 263×6 |
  | 2471–2486 | pre-2004, fetches 270–401 | 269×13, 282×13, 295×15, 310×17, 327×18, 345×17, 362×18, 380×21 |

  - **Expected:** about 12 h, finishing before the 2026-10-15 07:35 dailies.
  - **Exhibits to read after the drain:**
    - 3994805 (Ansell … SEGAP each with its own figure; Asept In Med, Medwin, Teleflex Medical, Vygon added);
    - 1829247 (`Lot A) Somaco. Lot B) Geneton …`);
    - 109697 (`3M Italia SpA`, `Kodak`);
    - 2874436 (Frantour ×3, no address names);
    - 3500348 (Informance, Planificación de entornos tecnológicos, Getronics).
  - **Residue counts:** `lot a`–`lot z` (1,338 before), `lots ` (1,016 before).
- **Residue left (unit 3's own):**
  - entries with no terminator (`Lots 1 à 5 - Taxis Doublet`, `lote 2 Sanofi Pasteur MSD`);
  - `X à <town> (NN)`;
  - space-joined lower-case references (`société Jouve lot n°3 : société …`, 2602352);
  - quoted names without a listed legal form (`'Donserv'`, `'Nota Bene' UAB`);
  - OCR-broken numbers (`113-1\n16`);
  - a misspelt void (`Déclaré infrucuteux`, 2027050);
  - a Dutch address glued to its name with no comma (2513708).


## Unit 3 drain — 2004–2010 half read (2026-10-14 18:5x UTC)

Jobs 2449–2470, the 2004–2010 rounds, are done. Every reparse read 0 unmatched / 0 re-keyed / 0 failing. The
projects re-bound 14, 131, 170, 31 … recorded mentions per round (issue 434). The 2004–2010 exhibits, read on the
public API by notice → tender:
- notice 3994805 (31946-2010) → tender 4107566. Nine results, each with its own figure: Ansell, Asept In Med,
  CAIR LGL, Fresenius Kabi France, Medwin, Nutricia nutrition clinique, SEGAP, Teleflex Medical, Vygon.
- notice 2874436 (162665-2006) → tender 3441486: Frantour, Frantour/Est Voyages, Frantour/Dernouvel. No address
  names.
- notice 3500348 (240253-2008) → tender 3798796: Informance, Planificación de Entornos Tecnológicos, Getronics.

The exhibit numbers in this file are NOTICE ids. Read them via `/v1/notices/{id}`, then
`/v1/tenders?publication_id=<it>`. Still to come after the pre-2004 rounds (2471–2486): 1829247 (17653-2002)
and 109697 (24056-1994), then the sweep and the residue counts.

## Unit 3 drain — done (2026-10-14 22:2x UTC)

- **Pre-2004 rounds 2471–2486** re-parsed 0 unmatched / 0 re-keyed / 0 failing, as did every 2004–2010 round. The
  drain ended at 22:23 UTC, about 9 h before the 07:35 dailies.
- **The auto sweep 2487** walked 5,685,007 organizations. It swept 8,523 of the 8,523 orphans it counted (all
  country-less) and kept 10 by a review table. It stayed under the 10,000 cap, so no sweep by hand was needed.
- **The pre-2004 exhibits** read as built:
  - notice 1829247 (17653-2002) → tender 2830358 serves Somaco, Geneton, PPS, SGFI, La Louisiane, UTB, ALS and Apie
    (`Lot A) Somaco. Lot B) Geneton …`);
  - notice 109697 (24056-1994) → tender 1870006 serves `3M Italia SpA` and `Kodak`.
  - The 2004–2010 exhibits were read above.
- **Residue** (bounded `name_norm` range counts, 22:3x UTC):

  | prefix | after unit 2 | after unit 3 |
  | --- | ---: | ---: |
  | `lots ` | 1,016 | 667 |
  | `lot a` … `lot z` (whole range, words too) | 1,338 | 869 |
  | `lot 1)` | 0 | 0 |
  | `et 1` … `et 9` | 48 | 47 |

What remains is the unit-3 residue list above: no terminator, `X à <town>`, space-joined lower-case references,
unlisted quoted forms, OCR-broken numbers, a misspelt void, and a Dutch address glued to a name. Some of these rows
are real company names that begin with `Lot` or `Lots`.
