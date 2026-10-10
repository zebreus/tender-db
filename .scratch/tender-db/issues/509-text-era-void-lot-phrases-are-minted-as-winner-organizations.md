# 509 — text-era void-lot phrases (`Infructueux`, `Sans suite`, `Desierto`) are minted as winner organizations era-wide

Status: ready-for-agent — filed 2026-10-10 from issue 508's drain audit. The parser fix landed with 508 (`c21411c`,
`76c44ed`). It applies to a notice only when that notice is re-parsed. 508's two rounds clean 2009-12 → 2010. The
rest of the era (1997–2009) still carries the junk until an era-wide re-parse.
Kind: data quality (organizations; text era)
Relates to: 508 (where it was found and fixed), 244 (the text-era winner campaign that minted them; its NAME_REJECTS
rule asks for exactly this, "added with its own evidence"), 234 (identifier-less organizations)

## What is wrong

A text-era award body often fills the winner slot with something that is not a company. The lot was void
(`Infructueux`, `Lot déclaré infructueux`, `Sans suite`, `Non attribué`, `Desierto`, `Declarado desierto`,
`Nessuna aggiudicazione`), or the slot points elsewhere (`Véase perfil del contratante`, `See Section VI.2)
Additional information`, `Voir autres informations`). `plausible_name` refused only `Various` and two
withheld-value phrases, so each became an identifier-less organization served as the winner. They are many orgs per
spelling: `name_prefix=Infructueux` lists 1169469 (FR), 1176430, 1176434, 1176435 …; likewise `Sans suite`,
`desierto`, `non attribue`, `Lot infructueux`, `Véase perfil de contratante` (31817859).

## Measured (2026-10-10, bounded windows of stored `TED-OFFICIALNAME`, 50k notice ids each)

| window | junk winner names in it |
|---|---:|
| 1,000,000 (1997) | ~10 |
| 2,000,000 (2002) | ~20 |
| 2,600,000 (2006) | ~120 |
| 3,200,000 (2007) | ~700 |
| 3,800,000 (2009) | ~400 |
| 4,200,000 (2010, before 508's fix) | ~400 |

In 508's first drained round (fetches 186–192) there are about 3,000 such names, 2 % of its winners. A 300k-name
scan for false positives found none: every match was junk.

## Fix

Parser: done in 508. `NAME_REJECTS` gained the substrings and `NAME_WHOLE_REJECTS` the whole-value forms,
pinned by `a_void_lot_or_a_pointer_in_the_winner_slot_names_nobody`.

## Units

1. **Size the era re-parse.** Text packages are fetches 186 … 372 (issue 244's era pass: 173 packages, 12–16 h,
   chunked so that each fold stays under the 500 k legacy-closure line; for 2010 issue 244's rule was ≤ 8 packages
   per fold). A reparse of only the packages that hold the phrases is not addressable from the parse layer, so plan
   the whole sectioned era (2004–2009) in fold-sized rounds, in queue gaps, never beside another heavy job.
2. **Drain** round by round, reading `unmatched` / `re-keyed` (must be 0) per reparse and each fold's
   `[project] legacy closure` line.
3. **Sweep** the organizations left with no mention (`orphan-org-sweep`, dry first).
4. **Verify** below.

## Verify

    ssh root@zebreus.click 'curl -s "localhost:8080/v1/organizations?name_prefix=Infructueux&limit=50" | jq ".items | length"'

- **open** (2026-10-10): `50` (the page is full; many orgs per spelling).
- **done:** `0` after the era drain and the orphan sweep.
