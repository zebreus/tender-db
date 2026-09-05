# Rubric v3c — one identifier under several country codes (issues 311/326/355/357)

You are reviewing organization rows from a public-procurement database (EU TED + national
sources). Each CASE is the set of `organizations` rows that carry ONE identical register number
(`identifier`) under DIFFERENT country codes — two codes, or one heavy code plus a spray of
strangers. The names may differ (branches, transliterations, a buyer's spelling). Your job, per case and per row: decide
whether any row's `country` is WRONG — a publication or parsing artefact — and if so WHERE it
belongs. The decision is executed literally: a `move` rewrites that row's `country` to `to`,
nothing else. Be exact about which row (its `org` id) moves and where.

## The evidence in each member (read every field)

- `country`: the code the row stands under today. `from` in any move MUST equal this.
- `identifier_kind` / `identifier`: the register number as published (vat | national | NULL).
- `anchors`: the register schemes under which this identifier's CHECKSUM VALIDATES, e.g.
  `NO:orgnr`, `ES:cif`, `CZ:ico`. Formats-with-checksums only — many registers (DE, most
  national ids) have NO checksum and can never appear here. An anchor for country X is
  positive arithmetic evidence that the number is an X register number.
- `country_probed` / `country_agrees`: `agrees` = the number validates under the row's OWN
  country's register (the row is where it says). `probed && !agrees` = the row's own register
  has a scheme of this shape and REFUSED the number — the contamination signal, strongest when
  an anchor names another country. `!probed` = the row's country has no scheme of this shape:
  SILENCE, not evidence. Do not read silence as "wrong".
- `mentions`: how many notice mentions stand on this row. 29,547 vs 2 is one entity with a
  stray duplicate; 200 vs 180 is two real registrations. Weight is a tie-breaker and a VETO
  (a heavy row has standing), never on its own a reason to move.
- `variants`: language-tagged name spellings on the row; `notices`: a few publication ids.

## What the census says about the case (read it first)

- `codes` / `mentions`: every code holding the number, with mentions per code, heaviest first.
- `asked`: the codes whose register has a scheme of this value's shape — the codes the
  arithmetic could even test. A code not in `asked` is SILENCE for that row.
- `named` / `named_schemes`: the codes the checksum actually names (`BG:eik`, `LT:kodas`).
  `named_schemes` matters more than `named`: a Luhn pass (`SE:orgnr`) has no country
  semantics; a national mod-11 (`LT:kodas`, `BG:eik`, `NO:orgnr`) does.
- `verdict`: the census's own class — `no-one-letter-pair` (codes differ by more than one
  letter), `nobody-asked` (no scheme for any code), `anchor-names-one` (the arithmetic names
  one code; the strangers still standing are those it refused to move on shape alone),
  `anchor-names-several`, `asked-and-refused`.
- `heavy_one_letter` / `one_letter_pair`: a one-letter distance between codes — the
  transcription-slip signature (`SK`→`SG`, `BG`→`BF`).

## The spray shape (this cohort's own signature)

One heavy code (tens or hundreds of mentions) plus several codes with ONE or TWO mentions each,
all carrying the same number: a Bulgarian EIK under `BG` with `VA`, `VU`, `GA` beside it. The
strangers are publication artefacts — a buyer-side field, a sniffer minting a code from a word
(`GERICHT`→`GE`, `CHARITY`→`CH`, `ADRESA`→`AD`) — and they move to the heavy code. That is a
`move` with evidence `identical-identifier-weight` at HIGH when (a) the heavy code is `named`
by a national scheme OR carries ≥ 10× the stranger's mentions with the same name/legal form,
and (b) the stranger's own name does not say it is a distinct foreign branch or mission.
A stranger whose NAME names a foreign mission ("Embassy of…", "Botschaft…", "…Niederlassung
Deutschland", "…sivukonttori Suomessa", "…branch") is a foreign filing of one entity: KEEP
(verdict `same-entity-two-registrations`) — the census's footprint rule applied by hand.

## Verdicts (per case)

- `wrong-country`: at least one row's country is a mistake and the evidence names where it
  belongs → emit `country_moves`.
- `same-entity-two-registrations`: one legal entity genuinely registered in both countries
  (e.g. a Swedish AB with a German VAT number on its DE row — each identifier validates under
  its own country). NO move. (There is no execution path for a merge; record only.)
- `distinct-entities`: two different organizations that happen to share a name. NO move.
- `needs-more-evidence`: the packet cannot settle it. NO move. Say what would settle it.

## Moves: when, and how confident

A `move` {org, from, to} needs an evidence class. Use exactly one of these labels:

1. `arithmetic` — the identifier validates under `to`'s register (anchor `TO:*` present) and
   does NOT validate under `from` (`country_agrees` false). Strongest. HIGH when the number is
   byte-identical to a sibling row standing in `to` with `country_agrees` true (the sibling is
   the survivor; this row is its stray), or when `probed && !agrees` on this row.
2. `national-format` — the number's SHAPE is unmistakably `to`'s (a Spanish CIF `A82473349`:
   letter + 7 digits + control; a French SIREN 9 digits with a FR VAT prefix; `DE` + 9 digits
   is a German VAT id; an Austrian `ATU` + 8; a Dutch `NL…B01`; a Slovak/Czech 8-digit IČO
   under a one-letter-off code such as SG/CR). HIGH only when the format is specific to `to`
   AND the row's own country does not use that shape.
3. `identical-identifier-weight` — same byte-identical number as a much heavier sibling
   (≥10× mentions, this row ≤ 5 mentions) that stands in `to`, and no arithmetic contradicts
   it. HIGH only if the sibling's country is consistent with the name/language evidence;
   otherwise MEDIUM.
4. `name-language` — the name itself carries a legal form or language that names a country
   (`s.r.o.`, `Sp. z o.o.`, `ASA`, `Oy`), consistent with the sibling's country. MEDIUM at
   most on its own; raises another class to HIGH.

NEVER move a row whose `country_agrees` is true — the arithmetic says it is where it says.
NEVER move a row on weight alone. A row with ≥ 30 mentions has STANDING: moving it needs
class 1 with `probed && !agrees`, otherwise MEDIUM. A foreign VAT number names a real
jurisdiction: a row tagged with the country that ISSUED its VAT id is NOT contaminated, however
foreign its name looks. Silence (`!probed`, no anchors) is never evidence of contamination.

Confidence: `high` = you would execute it yourself; `medium` = probably right, wants a second
reader (NOT applied); `low` = a guess (NOT applied). Only HIGH moves are executed. Over-claiming
HIGH is the failure this rubric exists to prevent (the v1 rubric's "prefer the more specific
verdict" pushed defensible merges into unsupported wrong-country calls; that sentence is gone).

Multi-row cases: every row is judged separately; a case may move one row, several, or none.
The `to` of a move is normally the sibling's country; it may be a third country only when
class 1 or 2 names it explicitly.

Output exactly one entry per case IDENTIFIER (the `identifier` string is the case key), with
`country_moves` empty unless the verdict is `wrong-country`. Rationale: two or three sentences naming the evidence fields you used.
