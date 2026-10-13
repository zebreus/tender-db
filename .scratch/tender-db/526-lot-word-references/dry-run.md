# 526 unit 1: the parser measured on real bodies before the reparse (2026-10-13)

`reparse` has no dry run (operations.md, issue 484), so the change was measured locally on stored bodies.

## Method

1. **Lot sample.**
   - 912 notices: a stride over the first-mention notice of every fifth organization whose `name_norm` starts
     with `lot ` (`id % 5 = 0`, 4,172 organizations, 2,735 distinct text-era notices).
   - Text-era bands only: ids 17k–500k and 870k–2.24M.
2. **Control sample.**
   - 596 text notices: every 2,000th id in 1.2M–2.24M and every 4,000th in 20k–500k.
   - 37 of the ids were not found, and 7 had no `TXT-TX`.
3. **Pull.** `pull.py` ran on the box. It fetched each `GET /v1/notices/{id}/content`, throttled to 0.2 s and
   backed off on 429. It kept the `TXT-TX` body and the stored winner names: `TED-OFFICIALNAME` under `RES-*`,
   which is the old parser's output.
4. **Parse.** A temporary `#[ignore]` test (not committed) ran `awarded_names_and_values` on each body. This is
   the same call the `TX` prose rule makes. Its output, `{id, old, new}` per notice, is in
   `dry-sample-lot.jsonl` and `dry-sample-ctrl.jsonl`. `analyze.py` summarizes it.

## Result (final build)

| | lot sample (912) | control (596) |
| --- | ---: | ---: |
| notices unchanged | 37 | 581 |
| winner names, old → new | 4,644 → 5,258 | 310 → 354 |
| names carrying a lot reference, old → new | 4,423 → 208 | 9 → 4 |
| names shared by ≥ 3 notices | old: `Lots 1)` 48, `lots 2)` 46, `lots 1)` 27 … (all references); new: Vygon 17, Becton Dickinson 16, Medtronic 16 … (all companies) | none either side |

- **No real winner is lost.** Every old name absent from the new output is itself a list fragment: `Pomona
  Episaveurs. Lot 9: 1: Prodirest`, `rabais 0 %. Lot 4) Wanadoo Interactive`, `Marché numéro 03001. Lot 1) Lyreco`.
- **Control diffs** are the same class, or the same names in a different order.
- **The review's costly cases**, each now refused and tested:
  - `Lot 2) Idem`, `en cours`, `sera attribué ultérieurement`, `même titulaire`;
  - a price after `Lot 1:`;
  - a cue after the comma stealing the head;
  - a cut window read past its comma;
  - the dropped `S.A.` dot;
  - a reference left in the name.
- **Found by the sample and fixed before commit.** The first build still minted the hub `lots 1)` (lists longer
  than 24 bytes), `lots 7` (`lots 7, 8, 9 and 10: Carabelli`), `et 11) Maquet` (about 6,000 organizations on prod
  start `et <n>)`), `Lot 3`, `1: Prodirest` and the lower-case lot descriptions `papier` and `imprimés` (1644391).
- **Residue kept as published (unchanged or better than before):**
  - lettered and Roman labels (`Lot A)`, `Lot III:`; about 250 organizations on prod);
  - heading prose before the company (`contrôle technique: Alpes Contrôle`, `n° de marchés 030180: Edwards
    Lifesciences`);
  - a price tail (`ditta … di Palermo per 346 337 600 LIT`);
  - a description in the provider slot with the company in another item (`local bus service`, 150979);
  - `no action`. These cases were junk before too.

## Prod baseline (bounded `name_norm` range counts, before the reparse)

| prefix | orgs |
| --- | ---: |
| `lot 1)` | 5,239 |
| `lot 2)` | 1,487 |
| `lot 3)` | 999 |
| `lots ` | 8,929 |
| `lote 1)` | 109 |
| `lot n` | 192 |
| `et 1` … `et 9` | 987, 1,596, 1,033, 803, 517, 351, 284, 232, 184 |
| `-1` | 147 |

Verify exhibit: `Lot 1) Blin` → 1 (31735007).
