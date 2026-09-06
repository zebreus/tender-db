# 359 — the label-prefix class is not German: `NIP…` (18k rows), `PIVA…`, `CFEPIVA…`, `CIF…`, `NUMERNIP…`

Status: BUILDING 2026-09-06 00:3x UTC (owner) — vocabulary extended, guard widened by the ES CIF/NIF shape, tests green in `ingest`; gate, deploy and the `repair-label-prefixes` dry/wet + R2 follow in this firing. Filed 2026-09-06 00:2x from the 357 campaign's last slices.
Kind: data quality (organization layer identifiers) — rule-shaped, repair job exists
Relates to: 328 (the German half of the same class: vocabulary + `repair-label-prefixes`), 345 (the renormalisation repair, sibling), 357 (where the shape surfaced), 300 Stage 2 (R2 folds the reunions), 329 (E0 folds what R2 cannot key)

## Where it came from

Slices 6 and 7 of the 357 campaign carried clusters keyed by identifiers such as
`NIPA41015322` (Ayesa — a Spanish CIF with the Polish field name `NIP` glued on, on
BOTH the ES and the PL row), `NUMERNIPDE312308370` (Acandis GmbH: "Numer NIP: DE…"),
`CFEPIVA10548370963` and `PIVA10548370963` (Lloyd's Insurance Company's Italian branch,
"CF e P.IVA"), `CIFA48283964` (IDOM), `VATIDGB287249363` (Therakos). Issue 328 had
named this exact phenomenon — a publisher writes the field name in front of the
value — and fixed it for the German vocabulary (`USTID…`, `UMSATZSTEUER…`, `STNR…`,
5,766 rows). Its census table listed `NIPNUMER 1` and stopped; nobody looked for
`NIP` alone.

## The class

Prefix reads on 2026-09-06 00:2x UTC (box idle, 7 range predicates on `identifier` —
**these were sequential scans, ~1 s each, not the bounded shape the prod-read rule
asks for**; the dry run of the repair job is the census proper and replaces them):

| prefix | rows | distinct |
|---|---|---|
| `NIP` | **18,332** | 18,318 |
| `PIVA` | 998 | 997 |
| `NUMERNIP` | 291 | 290 |
| `CFEPIVA` | 283 | 281 |
| `CIF` | 264 | 259 |
| `VATID` | 23 | 22 |
| `VATBE` | 9 | 8 |

Three times issue 328's class, and the mechanism is the same: the labelled row and
its bare twin are two `organizations` rows for one entity, and the crosswalk's PL/IT
arms key `digits_only` values only, so the labelled row never gets an E1 key and R2
can never reunite them.

## Why the normaliser kept them

`REGISTER_PREFIXES` in `project.rs` lists `KRS`, `NIP`, `REGON`, `CIF`, `NIF` as
register TAGS — "a scheme tag, not a country" — and classifies a value starting with
one as `national` with the tag kept, the way `HRB…` keeps its division. That is right
for `HRB`/`HRA`/`FN`/`VR` (the division carries meaning and the bare number is
ambiguous without it) and wrong for these five: a 10-digit PL national IS a NIP, a
9-digit one a REGON, a 0-led 10-digit a KRS serial, a letter-and-eight under ES a
CIF — the shape classifies the bare value identically, so the tag is a field name.
`UST` sits in the same table and 328 already stripped `USTID…` first; this issue does
the same for the other five.

## The fix (this firing)

1. `countries::LABEL_PREFIXES` gains `CODICEFISCALE`, `PARTITAIVA`, `NUMERNIP`,
   `NIPNUMER`, `CFEPIVA`, `VATID`, `REGON`, `PIVA`, `NIP`, `KRS`, `CIF`, `NIF`, `CF`
   (length-ordered; the longest-first test pins it). `VAT` alone is NOT added:
   `VATNO…` is the Norwegian country prefix as often as the English word.
2. The recognisable-remainder guard (328's "sharper than it parses") accepts one more
   shape besides a real scheme and pure digits: the Spanish CIF/NIF/NIE (nine
   characters, letter-seven-digits-check or eight-digits-letter). `CIFA48283964`
   strips to a value the national arm classifies by shape; no leftover label
   fragment has that shape.
3. Tests: the strips in `countries.rs`; in `project.rs` the pairing test (labelled
   keys exactly as bare, ten prod values), the compound `NIP…REGON…` field and the
   bare field name left alone, the shape test on both sides.
4. Deploy, then `repair-label-prefixes` dry (the plan lists every fix with its
   pre-image — read it by label before the wet run), wet with parity, then
   `match-org-identifiers` r2 dry/wet for the reunions R2 can key (PL NIP/REGON, IT
   P.IVA, DE/GB VAT), `project`. What R2 cannot key (ES CIF is E2 by design — the
   DIR3 collision) becomes same-triple pairs for E0 (issue 329).

## Do not

- Do not strip `HRB`/`HRA`/`FN`/`VR`/`PR` (328's reasoning stands).
- Do not add `VAT` or `VATNO` (country-prefix ambiguity); `VATNUMBER`/`COMPANYNUMBER`
  and the other English labels from 328's table stay out until measured on their
  own — their remainders are bare digits under GB, where the bare form is itself a
  soft identity.
- Do not run the repair while ingest/fold jobs run (parity abort is the safety net,
  not the plan).

## Repaired, and the fold stopped at the plan (2026-09-06 00:4x UTC, rev b9c0928)

Deployed at 00:41. `repair-label-prefixes` dry (job 746, 1 s): **34,375 rows carry a label,
28,085 planned, 6,290 kept as published** (the guard refused the remainder — compounds such
as `KRS0000001201NIP7270126358`, `CF` + a 30-hex GUID on FR/DE rows, `CF` + a UK postcode),
**16,366 land on an identity that already stands**. The plan listing is capped at 400, so the
risky labels were read through bounded identity-index seeks instead (country + kind fixed,
identifier range): `CF`+digit under IT is 886 rows of which 699 leave an 11-digit P.IVA and the
rest compounds; under FR (393) and DE (103) it is hex GUIDs, all refused; `REGON` under PL is
9,456 rows (the biggest class after `NIP`, unmeasured before the dry run); `KRS` 1,030; `NIP`
18,170; `NIF` 181 ES + 51 PT; `CIF` 220 ES + 5 RO. Wet (job 747, 7 s): **28,085 applied,
0 skipped**.

Then `match-org-identifiers` r2 dry (job 748): E1-keyed rows 365,627 → 392,068, **16,179
groups planned** — 231,664 mentions, 1.73 M party rows, 242k winners to repoint — and the
listing plus sample (569 groups) read by hand: 502 with identical name cores, ~45 with
partial overlap (a long form and a short one), and **11 whose members' names share nothing**:
`Powiat Wadowicki` with `REKORD SI Sp. z o.o.`, `Wodociągi i Kanalizacja w Opolu` with `WTE
Wassertechnik (Polska)`, `Adam Biedrzycki Chemosynteza` with `Instytut Ogrodnictwa`, a county
with a care home, a town with a school. That is the buyer's NIP written into the winner's
identifier field — a WRONG IDENTIFIER on one row, and R2 would fuse the two organizations.
~1.4 % of 16k groups is ~230 false merges. **The wet R2 was not run.**

### The R2 name gate (built the same night)

R2 had no name test: its E1 key was merge-grade by design, and every run before tonight
folded either the standing bare-identifier stock or rows a campaign had just read one by one.
E0 (issue 329) has the name rule; R2 now takes its DENY half — `agree` and `contained`
(a branch carrying the parent's NIP) merge as before, `unnamed` merges, **`disagree` denies**
(the N3 keys of the named members are neither one key nor one another's token subsets), and
the denied groups are LISTED in the `r2-merge-plan` report (`denied_names_listing`, capped at
500) as the review queue. Test `r2_name_gate.rs`: agree, contained, disagree, a stranger in
an agreeing pair (group-atomic deny), unnamed. Deny direction only; recall lost on renamed or
translated names goes to review rather than to a fuse.
