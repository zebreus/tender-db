# 510 — void-lot phrases (`Infructueux`, `Sans suite`, `Desierto`) mint winner organizations in every era, not only the text era

Status: ready-for-agent — filed 2026-10-10 from issue 509's Verify, which could not pass. The text-era parser's rejects
(issue 508) cleaned the 2004–2010 text notices, but the same phrases arrive as winner names from the XML eras
(r208 2011–2021, r209), `internal-ojs` and eForms, where no name check exists. The largest hubs are XML-era. Unit 1 is
the decision below: one fold-level name rule for every era, applied at mint time and through one refold, instead of a
reject list per parser.
Kind: data quality (organizations; all eras)
Relates to: 509 (the text-era drain that exposed this), 508 (`NAME_REJECTS` / `NAME_WHOLE_REJECTS` in
`crates/ingest/src/text/parse.rs`), 484 (`NON_NAME_FOLDS` in `project/role_census.rs`, the fold-level precedent for
"a placeholder is not a name"), 434 (a fold re-resolves recorded mentions), 443 (the orphan sweep that removes what a
refold leaves)

## What is wrong

A winner slot that says the lot was void (`Infructueux`, `Lot infructueux`, `Sans suite`, `Non attribué`,
`Desierto`, `Declarado desierto`, `Nessuna aggiudicazione`) or points elsewhere (`Véase perfil del contratante`,
`See Section VI.2`) is minted as an identifier-less organization and served as the winner. Read 2026-10-10 after
509's drain (mention counts per profile, bounded reads through `/v1/sql`):

| org | name | mentions |
|---|---|---|
| 1169469 | infructueux (FR) | r208 3,219, internal-ojs 126, r209 22 |
| 1170715 | Sans suite | r208 2,122, internal-ojs 26, r209 5, eforms 1 |
| 1177920 | Lot infructueux | r208 1,025, internal-ojs 47, r209 4 |
| 1185001 | Desierto | r208 1,004, text 99, internal-ojs 6 |
| 1247907 | Desierto | r208 99, eforms 12, r209 3 |

plus hundreds of singleton orgs of the same spellings (`name_prefix=Infructueux` returns a full page of 50).

Three gaps:
1. **No check outside the text parser.** `plausible_name` and its two lists live in `text/parse.rs` only. XML and eForms
   names reach the fold's mention resolver unchecked.
2. **Variants the text rejects miss.** `Desierto.` with a trailing period (2005, fetches 253 and 255, inside 509's drained
   range) and `Desierto (lotes VIII y IX)` / `desierto (lote 3)` survive: `NAME_WHOLE_REJECTS` compares the raw value
   whole.
3. **Pre-2004 text** (fetches 270+) was never drained (509 measured ~10–20 junk names per 50k notices there).

## Proposed fix

**Unit 1: decide where the rule lives.** The candidate is the fold: a mention whose every published name is a void-lot
or pointer phrase mints no organization and records no winner role. That is one rule for every era, it applies through
`project` (a refold, no reparse), and the 443 sweep removes the orgs it orphans. The rule is the text lists generalised:
compare a folded form (case and accents folded, trailing punctuation and a trailing parenthetical lot qualifier
`(lote 3)` / `(lots 2 et 4)` stripped) whole against the whole-value list, and the raw form against the substring list.
Decide in this unit:
- whether the text parser keeps its own lists (it drops the slot before a mention exists) or defers to the fold;
- what happens to the RESULT: the lot result keeps its decision (`not awarded` is often stated separately) but names no
  winner;
- the false-positive guard: the 300k-name scan from 508/509 found none for the text era; repeat it over an XML-era
  window before building.

**Unit 2: build and drain.** Refold the affected Tenders: `refold-notices` over the notices carrying a mention of the
listed orgs, or a profile-scoped refold. Then `sweep-orphan-orgs` (dry, read, wet).

## Verify

    ssh root@zebreus.click 'curl -s "localhost:8080/v1/organizations?name_prefix=Infructueux&limit=50" | jq ".items | length"'

- **open** (2026-10-10): `50`.
- **done:** `0` after the refold and the sweep.
