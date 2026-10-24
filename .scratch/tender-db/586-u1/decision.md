# 586 unit 1 — decisions D586-1 (alias fold) and D586-2 (touchpoint gate), and the build and drain spec

Decided 2026-10-24 from the saved answers only: no network, no box, no `/v1/sql`, no cargo, no git. This file
merges three inputs:
- the two halves, `fold.md` (a) and `gate.md` (b);
- the adversarial check, `check.md`, whose five required changes are all adopted here;
- one more measurement of mine, `final.py`, which finds and closes two more gaps (see "What this file changes" at the
  end).

**The oracle is `final.py`.** It imports `fold.py`, `gate.py` and `pop.py` unchanged and layers the amendments on top.
- `python3 -I final.py --selftest` passes 80/80.
- `--report` writes `final.out`.
- `--jsonl fixtures` writes unit 2's two fixtures: `fixtures/vectors.jsonl` (all 3,751 saved pairs: touchpoint,
  parent, tier, gated) and `fixtures/selftest.jsonl` (the 80 named cases).
- fold.py's own 38 selftest pairs give the same answers under the final rules.

All paths in this file are relative to `.scratch/tender-db/586-u1/` unless they start with `crates/`.

## Summary

| | Orgs (of the 3,751 new) | Their touchpoint mentions (of 24,664) |
|---|---:|---:|
| D586-1 aliases, tier A (spelling, brackets, legal forms, articles) | 182 | 743 |
| D586-1 aliases, tier C (entity under its organ, town hall, bare town, the parent's own organ) | 685 | 2,614 |
| D586-2 gate drops (labels, people, prose, legal references, platforms, bare organ words) | 92 | 3,491 |
| Still minted (tier B departments, typos, acronyms, persons without a title, …) | 2,792 | 17,816 |

- **Two fresh notice windows.** In the window over notices 24,020,000–24,039,999, 173 of 2,081 touchpoint mentions
  alias (8.3%) and 24 are dropped. In the window over notices 47,420,000–47,439,999, 74 of 904 alias (8.2%) and 16
  are dropped. About 90% of the aliased mentions bound an OLDER identifier-less row that names the parent. The alias
  moves them to the identified parent organization.
- **W: 0.** Every salted read is 40 R / 0 W:
  - fold.md's two reads (salts `586-u1-tierA`, `586-u1-tierC`);
  - the check's four reads (salt `586-u1-check-*`: tier A, tier C and both windows);
  - the check's read of 40 gate hits.

  Two full reads also found no W: fold.md read all 866 rows that aliased before the amendments, and the check scanned
  them. The six rows that tier C4 adds (below) were all read: 6 R.
- **Bars.** R3's top 60 has 0 hits. The tribunal pool has 0 hits. `MUST_KEEP` has 0 hits and `MUST_DROP` 0 misses.
  Among 7,060 real organization names, no real body is hit: the 10 names that are hit are the hub rows themselves and
  older rows named with prose.
- **Exhibit.** Tender 540072's `Lot-ReviewOrg` ×4 alias at tier A to 22198704. That is the issue's done state,
  `[22198704]`.

## D586-1 — the touchpoint alias fold

**Rule.** A touchpoint that a Lot or Part role names (D576-1 (iii)) is compared ONLY with its own parent
Organization section in the same notice. This is the `parent` map in `touchpoint_decision`. The comparison runs name
by name over every language either side publishes. The first test that holds decides:
1. **D576-1 (i), unchanged.** The names are equal under `store::org_name_norm`. Counted under `aliased` only.
2. **Tier A.** The fold keys are equal. Counted `aliased_a`.
3. **Tier C.** One of C0–C4 holds. Counted `aliased_c`.
4. **Otherwise D576-1's existing steps follow:** the gate (D586-2) and the 510/511/520/525 drop classes drop it, or
   (ii) mints it.

Over several name pairs the strongest result wins, in the order norm, A, C. Alias still comes before drop, as today.
`store::org_name_norm` stays at spelling. canonical.rs:7432 says why: the cross-notice reuse arm has no wall. The fold
lives in ingest and compares one nested pair inside one notice, which issue 300 §10 does not forbid.

### The key (tier A): fold.md "Tier A: the key", steps 1–9, with one amendment

The steps run in this order:
1. Decode HTML entities.
2. Lower-case. Apply `title_key_nfkd::NFKD_STRIPPED`, then the 14 letters of `EXTRA_FOLD` (`ł ø đ ð ħ ı ŧ ĸ ŋ ß æ œ
   þ ς`), then lower-case again.
3. Brackets, by side:
   - **Parent side:** cut every bracket, EXCEPT a bracket that names a department, a registry or a review organ
     (`DEPT_OR_REVIEW`, amendment **F3**). Such a bracket stays in the parent key. The words, matched whole on the
     bracket's folded tokens:

     `contratacion compras servicio servei servizio area concejalia regidoria departament* seccion unidad unitat
     negociado oficina distrito secretaria delegacion direccion gerencia tribunal tarcas tacrc tarc recurso* registro
     juzgado mesa abteilung amt afdeling dienst wydzial biuro ufficio settore`, plus the phrase `junta de contratacion`.

     A deny-list is safe on this side: all 35 parent brackets in the saved data are acronyms, places or descriptors.
   - **Touchpoint side:** cut a bracket only when it is on the allow-list: a contact or registry label, a Spanish
     province, legal-form tokens only, or one of the parent's own brackets.
4. Every non-alphanumeric character becomes a space.
5. Drop spelled-out legal forms anywhere.
6. Drop abbreviated legal forms at the end only, repeatedly, never down to nothing.
7. Drop articles and honorifics anywhere.
8. Join runs of one-letter tokens into one token ("C.U.C." becomes `cuc`).
9. Touchpoint side only: cut a leading `punto contacto` or `otro punto contacto`.

The exact lists are fold.md's; `fold.py` is their source of truth.

### Tier C (only when A fails; T = the touchpoint's key, P = the parent's key)

- **C0, the same organ under another spelling.** `junta gobierno local` is `junta gobierno`; `pleno municipal` is
  `pleno`; `alcaldia presidencia` is `alcaldia`.
- **E** is P minus ONE leading organ phrase: the longest one that leaves something (`ORGAN_PHRASES`: governing and
  collegiate organs and their officers, never a department).
- **C1, the entity under its organ.** An organ was cut and T == E, with two guards:
  - **F2:** a one-token E only when the parent prints a legal form. "GESPESA SAU" under "Gerencia de GESPESA, S.A.U."
    aliases; "Madrid" under "Asamblea de Madrid" does not.
  - **F4:** no cut under a presidency, direction, assembly, general council or officer organ when E starts with
    `gobierno govern generalitat xunta principado comunidad comunitat region estado administracion junta`.
    "Gobierno de Canarias" under "Vicepresidencia del Gobierno de Canarias" mints. Check.md only recommended F4; it is
    adopted because it costs 0.

  The F4 organs are `presidencia vicepresidencia direccion direccion general direccion ejecutiva junta general
  asamblea asamblea general asamblea representantes consejo general presidente presidenta vicepresidente
  vicepresidenta director directora director general directora general`. Other organs keep their cut, so "Consejo de
  Gobierno del Principado …" is still an organ over its entity.
- **Municipal words.** `MUNI` words at the head of E and of T are read as `ayuntamiento`:
  `ayuntamiento ajuntament concello ayto aj comune commune mairie ville gemeente stadt gemeinde municipio concelho obec
  mesto kommune`. Amendment **F1** removes `gmina` and `miasto`: in Poland a rural gmina and the town of the same name
  are two units, each with its own REGON and NIP.
  - **C2, the town hall under another municipal word:** E starts with `ayuntamiento` plus a place, and T == E.
  - **C3, the bare town under its town hall:** T == the place.
  - **C3r, the town hall of a parent printed as a bare place:** "COMUNE DI LAURINO" under "LAURINO".
- **C4, the parent's own organ or its town-hall word.** This is new: check.md's preferred fix for the bare-label hubs.
  Two shapes qualify:
  - T is exactly P's leading organ phrase. Synonyms are canonicalised on both sides, including when the synonym is
    the whole of T. Examples: "Pleno" under "Pleno de la Diputación Provincial de Huelva"; "Junta Gobierno Local"
    under "Junta de Gobierno del Ayuntamiento de León"; "Rectorado" under "Rectorado de la UPM".
  - T is exactly one municipal word, and E starts with `ayuntamiento` plus a place: "Ayuntamiento" under "Alcaldía
    del Ayuntamiento de Valverde de Leganés".

  "Alcaldía" under "Pleno del Ayuntamiento de X" is another organ and never aliases. It meets D586-2's BARE rule
  instead and is dropped.
- **Never alias:**
  - an organ cut from the TOUCHPOINT;
  - a bare place under anything but a town hall;
  - a prefix, a suffix, an overlap, a typo or an acronym match;
  - tier B: the parent plus a department, an address or a branch ("Stadt Fürth - Zentrale Vergabestelle"; "DIRECCIÓN
    PROVINCIAL DE AVRA EN SEVILLA (punto de contacto)"; "Gmina Panki ul. Tysiąclecia 5; 42-140 Panki" under "Gmina
    Panki").
- **Doc fix to fold.md.** `direccion` IS in `ORGAN_PHRASES`, so `organ_split` cuts it from "Dirección Provincial de
  AVRA en Sevilla" and leaves `provincial avra sevilla`. That is harmless, because no touchpoint prints that remainder.
  But fold.md's line "Dirección Provincial / Territorial are deliberately not organs" is wrong, and the code comment
  must not repeat it.

### Numbers under the final rules

- **The 3,751 new orgs.** By tier:

  | Tier | Orgs | Mentions |
  |---|---:|---:|
  | A | 182 | 743 |
  | C0 | 9 | 46 |
  | C1 | 455 | 1,778 |
  | C2 | 71 | 258 |
  | C3 | 145 | 526 |
  | C3r | 4 | 5 |
  | C4 | 1 | 1 |
  | All aliased | 867 (23.1%) | 3,357 (13.6%) |

  - F1–F4 change none of the 3,751 rows (check_variants.py and final.out agree).
  - C4 adds one org: 31975799 "Alcaldia -Presidencia" under "Alcaldía del Ayuntamiento de Noja", R.
- **The windows.**
  - Window A: tier A 10 mentions, tier C 163 (of them C4 7), out of 2,081.
  - Window B: tier A 7, tier C 67 (of them C4 3), out of 904.
  - The C4 groups re-point older cross-buyer rows to the parent: 1166995 "Junta de Gobierno Local", 1689043
    "Ayuntamiento" (4), 18945275 "Rectorado" (2), 1733704 "Junta Gobierno Local" (2) and 2130626 "Pleno".

### Side effects to expect (all N, none W)

- **`Touchpoints::retarget` is unchanged.** An aliased touchpoint at any tier is re-pointed to the parent ORG section.
  For a `Lot-ReviewOrg`, that section is usually the buyer. This is the intended serve:
  - the publisher printed the buyer itself as its review body ("recurso ante el órgano de contratación");
  - 540072's done state is one organization for the buyer and the review role.

  Where the parent is the tribunal (the Andalusian TARCJA touchpoints), the alias lands on the tribunal, which is
  also right.
- **The anchor shows the parent's name (issue 456).** The party row is anchored at the parent's mention, so its
  `mention_name` is the parent's printed name. A tier-C row therefore reads "Pleno del Ayuntamiento de Marugán" where
  the notice printed "Marugán". Under (i) the two names were equal, so this is new, and it is N.
- **The guid-twin key sees more rows.** `guid_twin_parties_sql` sees more Lot roles on the buyer's key (576 already
  noted this for (i)). Expect the delta; do not filter those rows out.
- **Census isolation is unchanged (D576-1 (v)).** An aliased touchpoint gets no mention, and its section is retired
  through `touchpoint_mentions`, exactly as (i) is today.
- **Residue that still mints.** The rows are listed in fold.md under "Not aliased, and why". They are tier B, typos,
  acronyms, a parent minus a trailing government qualifier, mancomunidad and comarca bare names, and company short
  names. Gmina Panki's two live rows (31977139 / 31978194) are tier B and survive as two orgs. Aliasing an address
  suffix would need its own decision; this issue does not make it.

## D586-2 — the touchpoint gate

**Mechanism.** Six touchpoint-only tests sit in a new ingest module. `touchpoint_gate` reads them beside its D576-1
(iv) tests, and nothing else reads them.
- **Not in `role_census::NAME_PATTERNS`.** Every `NameList` kind has readers outside the gate: `judge`, `buyer_fix`'s
  `review_or_platform_name`, `promotable` and the census tallies. A stem there would move served buyers on any later
  refold. Promoting a stem into `NameList::Portal` would be its own change, with its own `promotable` census.
- **Drop rule.** A touchpoint is dropped only when EVERY name it publishes is gated (today's `all(dropping)`).
- **Order.** The gate runs after the alias, so a label that is its parent's own name or organ still aliases.
- **Counting.** Every gated reference adds to `TouchpointRoles::dropped`. There is no new field.

Each test reads the name as published. "fold" means `crate::partyname::fold`, which is `match_norm` plus Latin
diacritics folded, the same as `role_census::fold`.

| # | Test | Exact rule | Stems |
|---|---|---|---|
| A | **WHOLE** label | fold(name) equals an entry | `principal`, `sede tibidabo`, `direccion principal`, `direcci n principal`, `info adicional`, `informacion adicional`, `recursos`, `direccion postal`, `direccion recepcion de recursos`, `direccion recurso`, `direccion de recursos`, `presentacion de recursos`, `direccion para presentacion de recursos`, `direccion informacion adicional`, `palacio provincial`, `ayuntamiento`, `procurement department`, `contact`, `beschaffende stelle`, `body responsible for mediation procedures`, `service from which information about the review procedure may be obtained`, `organizacion que proporciona informacion adicional sobre el procedimiento de contratacion` |
| B | **HONORIFIC** lead | gate.py's `HONORIFIC_RX` on the raw name (dot-bearing titles matched case-insensitively, bare `Mr`/`Mrs` case-sensitively, more text must follow, `m.` not before a one-letter abbreviation such as "M. I."). Guards: no `role_census::has_commercial_form`, no public-law stem. **G3** (check.md recommended): no hit when the word after the title folds to `buro`, `buero`, `bureau` or `studio` ("Ing. Büro Schneider" mints) | `monsieur madame mademoiselle mme mme. mlle mlle. herr frau dott dott. dott.ssa dottssa sig. sig.ra ing. m. mr. mrs. ms.`, `Mr`, `Mrs` |
| C | **PROSE** | a whole-word phrase anywhere in fold(name) | `procede recurso`, `ver apartado`, `en los lugares`, `lugares establecidos`, `lugares indicados`, `lugares previstos`, `lugares habilitados` |
| D | **LEGALREF** | gate.py's rule, read outside every `(…)`: `art` + `.`/`-` + digit after a non-alphanumeric character, `articulo(s)`/`artigo`/`articolo`/`artikel` + a digit token, or `el`/`del`/`y`/`en`/`segun`/`al` + `art` + a digit token. **G1** (check.md required): no hit when fold(name) names an EU act (`verordnung richtlinie regulation directive reglamento regolamento directiva direttiva eg ewg eu ce ue cee` as tokens), holds a public-law stem or holds a commercial form. **G1b** (new): a hit also needs a statute cue: a token `ley lcsp trlcsp lrjsp lpacap decreto rd lgs gesetz gwb vgv ustawa pzp loi codice tribunal tar juzgado vergabekammer`, or a numbered act or sub-article in the raw unbracketed name (`\d\s*[/.]\s*\d`: `39/2015`, `16.4`) | the rule |
| E | **PLATFORM** | as C | `tuttogare`, `plataforma electronica de contratacion`, `platforma zakupowa`, `platforma przetargowa`, `e zamowienia`, `ezamowienia`, `logintrade`, `achizitii md` |
| F | **BARE** organ, officer, town-hall or registry word (**G2**, check.md required) | the name's D586-1 key, computed WITHOUT the legal-form steps, is exactly one entry. That key is steps 1–4, 7 and 8, touchpoint side, with no parent brackets. Without the legal-form steps, "Gemeente Epe" keeps `epe` and does not reduce to a bare `gemeente`. C4 has already aliased the parent's own organ, so what reaches F is another body's label | every `ORGAN_PHRASES` entry and `ORGAN_SYNONYMS` key, every word of the FULL `MUNI` (`gmina` and `miasto` included: a bare one is a label), and `registro`, `registro general`, `registro electronico`, `sede electronica`, `secretaria general` |

Why each mechanism is shaped this way is in gate.md, "Why each mechanism is shaped this way". Two new pieces need
their reasons here:

- **G1b, the statute cue.** All 16 hits that only LEGALREF makes carry a cue (`Ley`, `LCSP`, `D. Lgs.`, `39/2015`,
  `51.3` or a tribunal word). check_gate.py's synthetic real bodies carry none, and without G1b these would drop:
  "Fundación Artículo 1", "Art-3 Gallery", "Asociación Art. 27 de la Constitución", "Artículo 14 Asociación",
  "Galería del Art 21", "ART. 25 Arquitectos SLP" and "Artikel 1 Gleichstellungsbüro". G1b costs 0 on the saved data.
- **F, the bare label.** In the windows, five bare labels bind older identifier-less rows that hold only that word,
  under unrelated buyers:

  | Label | Older row |
  |---|---|
  | "Pleno" | 2130626 |
  | "Rectorado" | 18945275 |
  | "Junta (de) Gobierno Local" | 1166995 and 1733704 |
  | "Registro General" | 1180214 |
  | "Secretaría General" | 1246716 |

  This is the 1689043 "Ayuntamiento" mechanism, and it is W by the verdict's own definition of a hub. The check named
  the first four. I found "Secretaría General" (window B, under "Rectorado de la Universidad de Salamanca"). Under
  the parent's own organ, C4 aliases these labels. Under any other parent, F drops them, because minting would bind
  the cross-buyer row. "Registro General (Registro del TARCAS)" keeps its bracket in the key, so it is not bare and
  still mints (residue: N, single publisher).

**Numbers.**
- **New orgs:** 92 orgs / 3,491 mentions dropped, the same rows as gate.py. G1, G1b, G2 and G3 change none of them.
- **Windows:** 24 of 2,081 and 16 of 904 mentions dropped. The older rows unbound are 1180214 "Registro General",
  1246716 "Secretaría General", 13863806, 17553080 and 18120239 (prose), plus the C4 rows above, which alias instead.
- **Bars:**
  - R3's top 60: 0 hits.
  - Class N rebuilds exactly (18 clusters / 37 orgs / 853 mentions).
  - The tribunal pool (28 clusters / 62 orgs): 0 hits.
  - 576 U4's 330 served touchpoint parties: 0 hits.
  - `MUST_KEEP`: 0 of 55 hit. `MUST_DROP`: 0 of 35 missed.
  - Real organization names: 0 real bodies hit. The Dortmund mandate name is no longer hit.

**The T judgment stands: drop.** These are 20 names that print a tribunal inside prose or a compound ("TACRC y art.
16 Ley 39/2015", 137 mentions). A drop is counted and undone by any later refold. A minted prose-named twin of TACRC
or TACGal is permanent (the verdict's "Permanent" section).

**Accepted residue, counted and not gated.**
- Bare personal names: 77 orgs / 285 mentions, a floor.
- Titles in the middle of a name (the APRR rows).
- Single-publisher site labels ("Hospital Real" 227, "Edificio San Francisco").
- Generic department names other than "Secretaría General" ("Financiën en Inkoop", "Biuro Zakupów", "Unidad de
  Contratación"; 576 U4 judged a pooled "Afd. Inkoop" N). A later census that counts parents per label can gate the
  ones that span buyers.
- Prose the stems miss (≤ 26 mentions).
- HONORIFIC on implausible touchpoint names: "Madame Tussauds", "Monsieur Bricolage", "Mr. Wash", "Herr der Ringe
  Stiftung". The check judged these implausible, and the saved data has none.

## Unit 2 — the build

The work lives in one new module and touches three existing ingest files, plus the counter suffix in `supervisor.rs`,
two docs surfaces and the CHANGELOG. There is no new crate dependency: ingest has no `regex` or `unicode-normalization`,
and adding one would re-hash Cargo.lock and force a clean ~35 min gate. There is no PROJECTION_EPOCH bump: unit 3's
`refold-sections` refolds the cohort, as 576 U2 did. `Box::pin` is not needed, because no `Spec` arm is added to
`supervisor::run_spec`; the suffix is a plain function.

### Files and anchors (line numbers as of 2026-10-24, main)

| File | Anchor | Change |
|---|---|---|
| `crates/ingest/src/project/touchpoint_names_586.rs` | new | The pure port: `alias_tier(touchpoint, organization) -> Option<AliasTier>` (A, C0, C1, C2, C3, C3r, C4, each `.counted()` as `Tier::A` or `Tier::C`) and `gated(name) -> bool` (D586-2 A–F). Also the lists as `const`s of folded tokens, the HTML-entity decoder, the bracket splitter and the key. Unit tests over the two fixtures |
| `crates/ingest/src/project.rs` | 946–973 (`mod` lines) | `mod touchpoint_names_586;` beside `mod title_key_nfkd;` (973). The new module reads `super::title_key_nfkd::NFKD_STRIPPED`, which is `pub(super)` and so visible to a sibling under `project` |
| `crates/ingest/src/project.rs` | 225–257 `TouchpointRoles` | Add `pub aliased_a: u64` and `pub aliased_c: u64`, both "of `aliased`" (norm = the rest). `add` sums them. `total()` is unchanged. Doc: (i) now covers issue 586's tiers |
| `crates/ingest/src/project.rs` | 6298–6311 `touchpoint_mentions` | Tally by tier: `Alias(_, tier)` adds n to `aliased`, and to `aliased_a` or `aliased_c` |
| `crates/ingest/src/project.rs` | 7276–7297 `touchpoint_gate` | `… \|\| touchpoint_names_586::gated(name)`. Extend the doc list with the six D586-2 tests |
| `crates/ingest/src/project.rs` | 7299–7314 `TouchpointVerdict` | `Alias(String)` becomes `Alias(String, AliasTier)`, with `enum AliasTier { Norm, A, C }`. Rewrite the doc of (i) |
| `crates/ingest/src/project.rs` | 7335–7352 `Touchpoints::retarget` | The pattern only (`Alias(organization, _)`). Behaviour is unchanged |
| `crates/ingest/src/project.rs` | 7355–7490 `touchpoint_decision` | Replace the `same` closure (7446–7449) with the best tier over the pairs: any `org_name_norm` equality gives `Norm`, else the best `alias_tier` over every (touchpoint name, Organization name) pair, A before C. The verdict (7457–7458) carries the tier. Rewrite the doc bullet (i) |
| `crates/ingest/src/project/role_census.rs` | 397–410 | Add `pub(super) fn has_public_stem(name: &str) -> bool { public(&fold(name)) }` beside `has_commercial_form`, for the B and D guards. `NAME_PATTERNS`, `portal_name` and `fold` are unchanged |
| `crates/ingest/src/partyname.rs` | 968 `fold`, 975 `not_a_name` | Reused, unchanged. The drop classes still run via `drop_decision_of(PartyShape::Touchpoint)` |
| `crates/store/src/canonical.rs` | 7448 `org_name_norm` | Unchanged, on purpose |
| `crates/app/src/supervisor.rs` | 6190–6202 `touchpoint_suffix`; test 23363 | Print the split (below). supervisor.rs is a 551 flip file, so the switch-off twin rebuilt after this lands must carry the new suffix |
| `crates/app/src/v1/docs.rs` | 469–490 paragraph; test 2045–2086 | The new alias and gate sentences (below) |
| `crates/app/data/openapi.json` | `TenderDetail.parties.description` | The same sentences (the docs test reads both surfaces) |
| `CHANGELOG.md` | top | `## Unreleased (issue 586) — …` (below) |
| `crates/ingest/tests/fixtures/touchpoint_586/` | new | `vectors.jsonl` (3,751 rows, 452 KB) and `selftest.jsonl` (80 rows), copied from `fixtures/` here and regenerated by `python3 -I final.py --jsonl <dir>` |

### Port notes for `touchpoint_names_586.rs`

**Characters.**
- Run `str::to_lowercase` on the whole string. Do not use `char::to_lowercase`: the final-sigma rule needs the
  string version, and `EXTRA_FOLD`'s `ς→σ` covers the rest.
- Then binary-search `NFKD_STRIPPED` for each char, then apply `EXTRA_FOLD`, then `to_lowercase` again.
- Decode HTML entities first. The forms are `&#NNN;`, `&#xHHH;`, `&amp; &quot; &apos; &lt; &gt; &nbsp;`. An invalid
  code point stays literal.
- `char::is_alphanumeric` stands for Python's `str.isalnum`. They differ only on Other_Alphabetic marks outside the
  NFKD blocks, which EU notices do not reach.

**Brackets.**
- The opener's first closer of the same kind ends a bracket. An unclosed opener runs to the end. A stray closer is a
  space.
- The parent's brackets are compared after joining initials, so "(O.A.G.E.R.)" equals "(OAGER)".
- F3's check runs on the bracket's tokens joined by single spaces, with whole-word matches. The words are a list,
  not a regex: `departament*` and `recurso*` are prefix entries.

**Lists.**
- Every list entry is written as folded tokens. Phrases match token by token, longest first.
- Trailing legal forms are removed in a loop that never empties the name.
- The legal forms run BEFORE the articles, because `sociedad de responsabilidad limitada` contains `de`.
- A unit test holds each entry to its own fold, in the style of `the_name_patterns_are_folded_whole_word_phrases`.

**Organ cut.** Cut one phrase only: the longest that leads P and leaves at least one token. C4's synonym
canonicalisation also applies when the synonym is all of T, which `fold._organ_syn` never does. final.py's
`_syn_whole` shows how.

**HONORIFIC without regex.**
- Take the trimmed raw name's first whitespace token, and compare it case-insensitively with the list. `Mr` and
  `Mrs` must match exactly, case included.
- A second token must exist and start with an alphanumeric character.
- For `m.`, the second token must not be one letter plus a dot.
- G3: the second token, folded, must not be `buro`, `buero`, `bureau` or `studio`.
- Then the two guards: no commercial form, no public stem.

**LEGALREF without regex.**
- Strip `(…)` spans innermost first, repeatedly.
- Scan the raw remainder for `art` that has a non-alphanumeric character (or nothing) before it, optional spaces,
  `.` or `-`, optional spaces, then a digit. Also scan for `articulo(s)`, `artigo`, `articolo` or `artikel`
  (diacritics folded) followed by a token that starts with a digit.
- Then scan the folded remainder for `(el|del|y|en|segun|al) art <digit…>`.
- Then apply G1 and G1b.

### The counter and its job-row line

`touchpoint_suffix` keeps every word the drain greps:

    ; touchpoint roles (issue 576): 108123 aliased (issue 586: 3012 by spelling, 37677 by organ), 444051 minted (… without a country of their own took the organization's), 34210 dropped

- The `(issue 586: …)` bracket is printed only when `aliased_a + aliased_c > 0`. The existing test's literal (4
  aliased, 0/0) therefore keeps its string.
- Add a second assertion with a split.

### Tests

Run them focused with the gate's flags and package set:

    CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p model -p store -p ingest -p tender-db --features tender-db/server touchpoint

Then run `ops/check.sh` (redirect to a file, and read `GATE-EXIT=`).

**`project/touchpoint_names_586.rs` (unit):**
- `the_port_matches_the_reference_on_every_saved_pair`: the tier and the gated flag of all 3,751 rows of
  `vectors.jsonl`, with the first 20 mismatches printed.
- `the_named_cases_hold`: the 80 rows of `selftest.jsonl`.
- `every_list_entry_is_its_own_fold`.
- `the_unicode_traps_fold_alike`, from check.md's pairs 31–42:
  - Turkish İ;
  - ß against SS;
  - Greek final sigma and tonos;
  - Catalan l·l and ŀ;
  - decomposed accents;
  - fullwidth letters;
  - U+037E against `;`.

**`project.rs` `mod touchpoint_576` gains `mod issue_586 { use super::*; … }`.** A child module sees the parent's
private helpers `notice`, `bound`, `parties` and `party`.
- **`sarga_540072_review_roles_bind_the_buyer_by_spelling`.**
  - Fixture: buyer "Sociedad Aragonesa de Gestión Agroambiental, S.L.U. (SARGA)", and TPO-0001 "Sociedad Aragonesa de
    Gestión Agroambiental S.L.U" named by `OPT-301-Lot-ReviewOrg` on LOT-0001…0004.
  - Counts: `TouchpointRoles { aliased: 4, aliased_a: 4, .. }`. No mention; `retired == ["TPO-0001"]`.
  - Parties: Procedure-Buyer plus four `Lot-ReviewOrg`, all organization 11 at ORG-0001. That is the Verify's done
    state in miniature.
- **`avra_six_touchpoints_alias_and_its_provincial_branch_mints`.**
  - Under "AGENCIA DE VIVIENDA Y REHABILITACIÓN DE ANDALUCÍA (AVRA)", six TPOs alias:
    - "(Punto de contacto)";
    - "(PUNTO DE CONTACTO)";
    - "(PC)";
    - "(otro punto de contacto)";
    - "AGENCIA DE VIVIENDA DE REHABILITACION DE ANDALUCIA (punto de contacto)";
    - the accent-only variant.
  - "DIRECCIÓN PROVINCIAL DE AVRA EN SEVILLA (punto de contacto)" mints.
  - Counts: aliased 6 (a 6), minted 1. The issue said ×5; the saved data has six (fold.md, check.md).
- **`gmina_panki_u037e_aliases_and_the_live_address_row_still_mints`.**
  - "Gmina Panki ul. Tysiąclecia 5\u{37e} 42-140 Panki" under "Gmina Panki ul. Tysiąclecia 5; 42-140 Panki" aliases at
    tier A.
  - The live shape under "Gmina Panki" mints (tier B).
- **`tarcas_and_the_review_organs_still_mint`.**
  - Under "Junta de Gobierno del Ayuntamiento de Sevilla", these mint: "TARCAS. AYUNTAMIENTO DE SEVILLA", "Ayuntamiento
    de Sevilla - TARCAS", "Ayuntamiento de Sevilla (TARCAS)" and "Registro del Tribunal Administrativo de Recursos
    Contractuales (TARCAS)".
  - F3: "Ayuntamiento de Sevilla" under "Ayuntamiento de Sevilla (TARCAS)" mints. So does "Ayuntamiento de Alcobendas"
    under "Ayuntamiento de Alcobendas (Contratación y Compras)".
  - "Tribunal Administrativo de Recursos Contractuales de la Excma. Diputación de Cádiz" mints.
- **`a_department_branch_other_organ_or_place_still_mints`.** Each of these mints:
  - "Stadt Fürth - Zentrale Vergabestelle" under "Stadt Fürth";
  - "Alcaldía del Ayuntamiento de Segorbe" under "Pleno del Ayuntamiento de Segorbe";
  - "Bilbao" under "Presidencia de la Autoridad Portuaria de Bilbao";
  - "Ayuntamiento Benalmádena" under "Concejalía de Hacienda del Ayuntamiento de Benalmádena";
  - F1: "Miasto Kłodzko" and "Kłodzko" under "Gmina Kłodzko";
  - F2: "Madrid" under "Asamblea de Madrid";
  - F4: "Gobierno de Canarias" under "Vicepresidencia del Gobierno de Canarias".
- **`the_entity_under_its_organ_aliases_and_serves_the_parents_printed_name`.**
  - Each shape aliases at its tier:
    - C1: "Ayuntamiento de Ezcaray" under "Junta de Gobierno Local del Ayuntamiento de Ezcaray";
    - C3: "Marugán" under "Pleno del Ayuntamiento de Marugán";
    - C2: "Ajuntament de Biar" under "Junta de Gobierno del Ayuntamiento de Biar";
    - C3r: "COMUNE DI LAURINO" under "LAURINO";
    - C0: "Junta de Gobierno Local del Ayuntamiento de Altea" under "Junta de Gobierno del Ayuntamiento de Altea";
    - C1 (one token, with a legal form): "GESPESA SAU" under "Gerencia de GESPESA, S.A.U.";
    - C4: "Pleno" under "Pleno de la Diputación Provincial de Huelva" and "Ayuntamiento" under "Alcaldía del
      Ayuntamiento de Valverde de Leganés".
  - Each one counts in `aliased_c`.
  - Marugán's party is anchored at ORG-0001. This pins the issue-456 side effect: the served `mention_name` is the
    parent's.
- **The existing `the_touchpoint_gate_drops_prose_section_ids_and_void_names_and_no_tribunal` grows.**
  - Its buyer is "Gemeente Utrecht".
  - New dropped names:
    - "Dirección Principal";
    - "Principal";
    - "Palacio Provincial";
    - "Registro General";
    - "Secretaría General";
    - "Pleno";
    - "Rectorado";
    - "Monsieur Anthony LINTILHAC";
    - "DOTT. ANGELO GAMBARDELLA";
    - "TACRC y art. 16 Ley 39/2015";
    - "Ver apartado 17 de la HRC. Rector de la Universidad de Vigo";
    - "TUTTOGARE RACCONIGI";
    - "Platforma Zakupowa Logintrade".
  - Not bare "Ayuntamiento": under "Gemeente Utrecht" it aliases at C4, because `gemeente` is a municipal word. Test
    it as a drop under "Concejalía de Hacienda del Ayuntamiento de Benalmádena" instead.
  - New names that keep minting:
    - "Ing. Büro SoFah GmbH & Co. KG";
    - "Ing. Büro Schneider";
    - "M. I. Ayuntamiento de Elche";
    - "Principal Financial Group";
    - "Palacio de Congresos de Valencia";
    - "Tribunal Administrativo Central de Recursos Contractuales (Según artículo 50.1.a) y b) de la LCSP)";
    - "Art 21 Gallery";
    - "Fundación Artículo 1";
    - "Gemeente Epe";
    - the Dortmund mandate name ("Stadt Dortmund, vertreten durch … nach Art. 2 b) Verordnung (EG) Nr. 1370/2007").
  - Every R3 tribunal and both R3 non-tribunal names stay in the keep list.
- **Literal updates.** The three full struct literals get the new fields (`..Default::default()` or `aliased_a: 0,
  aliased_c: 0`): project.rs:25094, supervisor.rs:23363 and tests/project.rs:7313. The literals that already use
  `..Default::default()` (25034, 25126, 25161) need nothing.

**`crates/ingest/tests/project.rs`:**
- **`a_touchpoint_role_is_served_through_the_touchpoints_own_identity` (576's unit-3 re-mint drain test) must still
  pass unchanged** except for its literal. Its TPO-0002 prints the buyer's name exactly, so it stays a norm alias,
  and the counts stay 2 aliased and 3 minted.
- **New: `a_touchpoint_minted_before_586_is_retired_and_rebound_by_the_refold`.** This is unit 3's drain in miniature,
  in the incremental fold's Phase 1:
  1. Record a notice whose TPO-0001 prints a name that does not alias ("SARGA - Registro", which mints) under the SARGA
     buyer, and whose TPO-0002 prints "Sede Tibidabo Norte" (which mints). Run the full fold. Assert minted 2 and two
     TPO mentions.
  2. Stand in for 586's rule change. With the raw connection (as 576's test does), run `UPDATE notice_texts SET value =
     'Sociedad Aragonesa de Gestión Agroambiental S.L.U' WHERE section_id = 'TPO-0001' AND field_id =
     'BT-500-Organization-TouchPoint'`, and set TPO-0002's value to `'Dirección Principal'`.
  3. Run `notice_ids_with_section_kind(["Touchpoint"])`, `unmark_projected_by_ids`, `stamp_stale_for_notices` and
     `project_incremental`.
  4. Assert:
     - `touchpoint_roles == { aliased: n, aliased_a: n, dropped: m, .. }`;
     - no `organization_mentions` row at either TPO key;
     - every `Lot-ReviewOrg` party has the buyer's `organization_id` at `ORG-0001`;
     - the `Lot-AddInfo` role on TPO-0002 is gone;
     - the two formerly minted organizations have no mention left, so a dry `sweep-orphan-orgs` count includes both;
     - every party and bid-party row's mention key names a recorded mention.

### Docs

The `/docs` paragraph `tender-detail-touchpoint` (docs.rs 469–490) and `TenderDetail.parties.description` in
openapi.json both describe the alias rule as "up to case, whitespace and a trailing `.`, `,` or `;`". That is no longer
true. Replace the alias sentence on both surfaces with this, and update the docs test's phrase list to match:

> A touchpoint that prints its organization's own name, in any language either prints, serves that organization's
> own party: its `organization_id`, anchored at the organization's block, so `mention_name` is the name the
> organization printed. Its own name is that name up to case, accents, punctuation, a contact label or acronym in
> brackets, a legal form and articles (`Sociedad Aragonesa de Gestión Agroambiental S.L.U` under `…, S.L.U. (SARGA)`),
> or the organization's body under its own governing organ: the body, its town hall or the bare town under the organ,
> or the organ's own name (`Ayuntamiento de Ezcaray` or `Ezcaray` under `Junta de Gobierno Local del Ayuntamiento de
> Ezcaray`; `Pleno` under `Pleno de la Diputación Provincial de Huelva`) (issue 586). A department, a branch, another
> organ or a review body printed under the organization keeps its own name.

Extend the dropped-names sentence with:

> … and, since issue 586, a contact or address label (`Dirección Principal`), a bare organ, town-hall or registry word
> that is not the organization's own (`Registro General`), a person addressed by title (`Monsieur …`), filing prose or
> a legal reference (`Procede recurso especial…`, `… y art. 16 Ley 39/2015`) and an e-procurement platform's name
> (`TUTTOGARE`).

Pre-refold sentence:
- Add "until the issue-586 refold reaches it, a Tender can still serve such a touchpoint under its own provisional
  organization", keyed `issue-586 refold`.
- Retire it through RETIRED after unit 3's drain and unit 4's Verify, as 576's sentence is retired.

CHANGELOG entry, `## Unreleased (issue 586) — a touchpoint that names its own organization in another spelling, or as
the body under its organ, serves that organization; contact labels, people, filing prose and platform names are no
longer served as organizations`:
- Exhibit: 540072 `Lot-ReviewOrg` moves from 31975096 to 22198704.
- The hubs 31974991 / 31975008 / 31975059 are removed (`organization removed`).
- The tier-C `mention_name` note.
- Stored Tenders change once refolded, each announced as a `version: null` `changed` row.

## Unit 3 — the drain

**One `refold-sections` run serves both 576 U5 and 586.** The job is `{"kind":"refold-sections","profiles":["Touchpoint"]}`
(job file `576-u3/refold_sections.json`). It re-queues every Touchpoint carrier (198,857 at 2758, more since) and
stamps their Tenders. The `project` job that follows refolds them in about 45 min.
- Post it once, after the deploy that carries BOTH 576 U5 (deploy #3) and 586 U2.
- Under the velocity ruling, do not hold U5's refold for 586. If deploy #3 is live and 586 U2 is not yet gated, run
  U5's refold now and a second one after 586's deploy. The second costs one 45-minute queue slot, class N.
- A refold is a writer: one at a time, through the job queue.

**What the refold does, all through existing code:**
- Phase 1's `touchpoint_mentions` (plan sweep, project.rs:2452; incremental, 3505) returns every touchpoint section
  that no longer mints, aliased or gated, as `retired`.
- `Db::retire_mentions` (canonical.rs:22804) deletes those mention rows and the party and bid-party rows keyed to them,
  writes ADR-0017 D5 correction rows and stamps the Tenders stale.
- `NoticeState::read`'s `retarget` re-points the aliased roles to the parent ORG. Phase 2 rewrites the Tenders.
- **retire_mentions never deletes an organization.** The fold queues `sweep-orphan-orgs auto (after a fold re-bound N
  and retired M)` by itself (operations.md:3270). It sweeps wet at or under `AUTO_SWEEP_CAP` = 10,000 orphans.
  - gate.md's wording "which `retire_mentions` or the sweep then removes" and the issue's "Confirm orphaned new orgs
    are retired by `retire_mentions`, else sweep" should both read: retire_mentions retires the mentions; the
    auto-queued sweep removes the orgs.

**Expected read**, from the project job row's suffix. Two figures are hard; the rest are predictions scaled from the
two windows (8.2–8.3% of minted mentions alias, 1.2–1.8% are gated; 491,287 references, about 2.1 per mention):
- **Cohort:** at least 198,857 carriers. If it grew, scale the predictions below by cohort / 198,857.
- **aliased > 67,434 (hard).** Predicted about 107.6k–108.3k, a gain of 40.2k–40.8k:
  - `aliased_a` about 2.4k–3.8k;
  - `aliased_c` about 36.4k–38.5k;
  - the norm remainder about 67.4k, unchanged.

  fold.md's +39k before C4 sits inside this.
- **minted < 491,287 (hard).** Predicted about 442.4k–444.8k: minus the alias gain and minus 5.7k–8.7k gated.
- **dropped:** predicted about 33.0k–36.1k (27,378 plus 5.7k–8.7k), plus U5's TenderNed dashboard labels if they ride
  the same run (not sized here).
- **Sum:** aliased + minted + dropped equals 586,099 plus the references of any carriers added since 2759.
- **Retired mention rows (the auto sweep's M):** about 21.8k–23.5k, plus U5's.
- **Swept organizations:** at most 959 of the 3,751 new ones (867 aliased plus 92 gated). Every gated hub is gated by
  name, so it empties entirely. An aliased org that also holds mentions under a non-aliasing parent survives, so
  expect roughly 850–959. These older rows lose only their touchpoint mentions and stay: 1689043, 2130626, 18945275,
  1166995, 1733704, 1180214 and 1246716.

A delta outside +30k…+52k for aliased, or +4k…+12k for dropped, is a check result, not a stop. Put up the banner,
read a salted 20 of the moved references (salt `586-u3-`), and fix.

**Verify (unit 4):**
- `curl -s https://tenders.zebreus.click/v1/tenders/540072 | jq -c '[.parties[] | select(.role=="Procedure-Buyer" or
  .role=="Lot-ReviewOrg") | .organization_id] | unique'` returns `[22198704]`.
- `/v1/organizations/31974991`, `31975008` and `31975059` return `.mentions` 0, or 404 once swept. So do 31975006,
  31975019, 31975180, 31975009, 31975004, 31975568, 31975060 and 31975949.
- Re-run `final.py`'s tier classifier over the surviving new orgs: tier A and tier C come out 0.
- Read a salted 30 of the surviving minted touchpoint orgs (salt `586-u4-`) under 576 U4's R/N/W rubric, bar 0 W.

## Interactions

- **D576-1.**
  - (i) widens from spelling to the D586-1 tiers. The order alias → gate → drop classes → mint is unchanged.
  - (ii), (iii) and (v) are unchanged.
  - (iv) gains D586-2's six tests, still touchpoint-only.
- **`Touchpoints::retarget`.** Behaviour is unchanged. Every alias tier re-points to the parent section (above).
- **`role_census` / `buyer_fix` / 369 / 481.** Unchanged. Touchpoint mentions stay off `NoticeState::mentions`, and
  the gate lists are not `NameList`s.
- **551.** Winners, bids and totals do not move: touchpoint roles are Lot and Part roles only, and results bind
  Organizations only. The 586 drain's corrections are party rows. supervisor.rs (the suffix) is a flip file, so the
  switch-off twin must be re-based to carry it.
- **456.** The tier-C `mention_name` side effect is above. Minted parties keep their own published names.
- **300 §10 / 351 / 234 / 432.** Nothing cross-notice changes. `org_name_norm` and `match_norm` are untouched.

## What this file changes against the halves and the check

1. **check.md's fold changes 1–3 are adopted (F1, F2, F3), and its recommended change 4 too (F4).** Each costs 0 on
   the 3,751 pairs and both windows.
2. **check.md's gate change 1 is adopted as G1, plus G1b.** G1b is a new statute-cue guard, costing 0 on the saved
   data. It stops LEGALREF dropping bodies named after an article, a shape that check_gate.py's synthetic list showed
   and check.md did not discuss.
3. **check.md's gate change 2 is adopted in both its forms: C4 (the preferred alias) and F/G2 (the bare stems).**
   - G2 is keyed on the D586-1 key without legal forms, not on whole fold strings. That way "Junta Gobierno Local"
     (no "de") and "El Pleno" are covered.
   - It covers every organ, officer and municipal word, plus five registry and department labels: `registro`,
     `registro general`, `registro electronico`, `sede electronica` and `secretaria general`.
   - "Secretaría General" → 1246716 is a fifth hub, which check.md missed.
   - Leaving out the legal-form steps keeps "Gemeente Epe", where `epe` reads as a legal form, from becoming a bare
     `gemeente`.
4. **check.md's recommended G3 (the Ing. Büro exemption) is adopted.** HONORIFIC on implausible names is accepted as
   residue.
5. **The counter keeps `aliased` as the total and adds `aliased_a` / `aliased_c`** (check.md), not fold.md's rename to
   `aliased_norm`. That keeps the job-row word `aliased` and the drain's comparison like with like.
6. **AVRA is six, not five.** Gmina Panki is tested on the synthetic U+037E pair. The fold.md doc nit about
   `direccion` is recorded above.
