# 520 U2 build notes: D1–D7, the void prose `names_an_award` misread (2026-10-12)

Built against `unit1-decision.md` §4.1 (the D1–D7 table, "Where the rules live", the residual), §5 U2, §7 and
Appendix A, with the names of `../525-void-templates/void-names-refold-after-dry.json` (`exempted[]`, cut at 300
characters) and the classes of `unit1-classified.json`. In the working tree, **not committed**, no gate run
(`ops/check.sh` was out of scope for this build), nothing on prod touched.

## What changed

All rule code is in `crates/ingest/src/partyname.rs`, inside `names_an_award`'s own arms. One test pin in
`crates/ingest/src/project.rs` moved with it (see Deviations 1).

| rule | where | what it does |
|---|---|---|
| D1 | `lot_title_void` (+ `lot_opener`), first line of `names_an_award` | The raw text opens `Le lot <n°?> <digits> :`, holds no other such opener, the fold closes on `a ete declare(e)(s) infructueux/infructueuse(s)` or `… sans suite`, and the fixed `award_clause` is quiet. Then `names_an_award` is false, so the name is `VoidLot`. |
| D2 | `award_clause(raw, folded)`, changed in place | (a) The negation is the nearest word that is not a `NEGATION_FILLERS` word, read against `NEGATIONS`, through the new `negated` (shared with `awardee_noun`). (b) `deja` right before the word means no award. (c) The `avec la/l'/les société(s)` cue does not count after `relancer`/`negocier`/`sera` in the same sentence (`sentences`). |
| D3 | `award_awardee_noun` (wraps `awardee_noun`) | An awardee word counts only as: a label (`:` or `=` after an optional entry number), but never `attribution` as a label; `(mandataire)`; `attribution à`; or a participle (`AWARD_PARTICIPLES`) before a `PARTY_PREPOSITIONS` word. 510's negation test stays. If the raw runs and the fold's words do not line up, it falls back to `awardee_noun`. |
| D4 | `award_several_lots` (wraps `several_lots`, `numbered_entries`, `lettered_entries`) | (a) `glued_number`: `no3` reads as `3`. (b) `words_before_relaunch`: a `RELAUNCH_MARKERS` phrase drops the rest of its sentence before `several_lots` counts. (c) `every_lot_void`: every lot entry is void, so the arm is quiet. An entry runs from a designator plus its number (`lot_label`, read as `several_lots` reads one) to the next. An entry holding only numbers and conjunctions joins the next one. The last such entry keeps the summary. Any entry whose text is not itself `is_void_lot` keeps it. |
| D5 | `award_mixed_segments` (wraps `mixed_segments`) | Split at `;` outside parentheses, after decoding `&amp;` in any case. A segment names nobody when `mentions_void`, a `BLANK_WHOLE` value, an `OFFER_WORDS` stem, or a `RELAUNCH_NOTES` phrase (510's five plus `annonce`, `joue`, `boamp`) holds. |
| D6 | `award_company_token` (wraps `company_token`), `before_a_bidder`, `bidder_before_void_lot` | (a) A form after an `OFFER_WORDS` stem, in a sentence with an `OFFER_REJECTIONS` stem, does not count. (b) A form after `relancer`/`negocier`/`sera` in its sentence does not count. (c) `Société`/`Sté` plus one to four digit-free words, then `Lot déclaré(e) infructueux`, with no designator-plus-number anywhere and no award clause, makes the whole arm quiet. |
| D7 | `named_before_spill(raw, folded)`, changed in place | The head before `section vi` names a party only when it has letters and `!mentions_void(head)` on the raw head, with a trailing `see` dropped. If the runs do not line up, it falls back to the 510 reading. |

Other changes:
- Constants: `AWARD_PARTICIPLES`, `PARTY_PREPOSITIONS`, `RELAUNCH_INTENT`, `RELAUNCH_MARKERS`, `RELAUNCH_NOTES`,
  `OFFER_WORDS`, `OFFER_REJECTIONS`.
- Helpers: `negated`, `labels`, `digits_only`, `lot_number_word`, `sentences`. `sentences` cuts after a `.` that a space
  or the end follows, and at `;`. A cut too many only lets a keep signal count again.
- `numbered_entries` and `lettered_entries` moved from `names_an_award`'s arm list into `award_several_lots`, which is
  the D4 gate.
- Docs: the module doc has a 520 paragraph, the `names_an_award` doc covers D1–D7 and the four untouched shared
  helpers, and each new function and constant has its own doc with prod specimens.

**The shared helpers are unchanged byte for byte.** A function-level compare against HEAD covers `summary_shape`,
`named_before_stem`, `names_a_legal_form`, `names_something`, and their callees: `company_token`,
`polish_company_form`, `several_lots`, `numbered_entries`, `lettered_entries`, `legal_form_reading`,
`alphanumeric_runs`, `roman_with_digits`, `mentions_void`, `classify`, `mixed_segments`. Only four functions changed:
`names_an_award`, `award_clause`, `named_before_spill`, and `awardee_noun`. `awardee_noun` was refactored to call
`negated`, with the same behaviour. No existing constant changed.

## Tests (partyname.rs)

New rule tests, with prod specimens by org id (id 0 marks a near miss written for the test), through
`assert_class`:

| test | rule | void specimens | kept near misses |
|---|---|---:|---|
| `one_lots_void_sentence_is_void` | D1 | 11 | 31738609 (no opener), two openers, an award clause |
| `a_negated_earlier_or_intended_award_is_no_award` | D2 | 11 (a, b, c) | 1435457 (`puis attribué`), 1397079 (past relaunch `avec la société B BRAUN`), `Sans suite, marché passé avec la société Dupont`, 4552687 (`attribué le 9/12/13 à`), 4409817 (`attribué le 28.10.2013`), 1341883, 31486027 |
| `an_awardee_word_must_label_a_party` | D3 | 12, each also `!award_awardee_noun` | 6 labelled prod rows (6191476, 7030205, 3982542, 7805728, 1621278, 5425803), plus direct arm checks: `(mandataire)`, `attribution à:`, `adjudicado a`, `aggiudicato alla`, `awarded to`, `gegund aan`; `Date d'attribution :` and `Pas d'attributaire` are false |
| `a_list_of_void_lots_is_void` | D4 | 13 (incl. 1537305, 3556403, the old doc's `Les lots 2 et 11 …`) | 30698459 (cut list), 31731990 (lettered), `Lot 1) Dupont. Lot 2) infructueux`, 2957546, 4 C rows, residual 31391828, 23532535, 28310858 (`None`) |
| `a_reason_segment_names_nobody` | D5 | 12, each also `!award_mixed_segments` | 4836476 (`Bp innovis ; infructueux`), 1722465, the `2013023201/zone …` literal, each also `award_mixed_segments` |
| `a_bidder_is_no_winner` | D6 | 10 (incl. 3526210), each also `!award_company_token` | `Yvelin SA. Lot 2 : infructueux`, 6603163, 1844226, `Lot infructueux, sera relancé. Lot 2: Dupont SARL`, a numbered `Société … Lot déclaré infructueux, lot 2 : Martin`, `DESIERTO, S.L.` (`None`) |
| `a_void_head_before_a_spilled_section_is_void` | D7 | 9, each also `!named_before_spill` | Zundel Et Kohler, Getinge France, and `Desierto Florido SL. SECTION VI …` (arm true) |

Pins:
- `every_void_520_exemption_is_a_void_lot`: all 173 orgs of Appendix A's D lists (D1 68, D2 17, D3 39, D4 11, D5 12,
  D6 9, D7 14, two-rule 3) read `VoidLot`.
- `every_kept_520_exemption_keeps_its_class`: every A (57), B (99), C (29) and F (6) org, and 11 of the 12 residual D,
  keep the class they read at HEAD (`AwardSummary`, or `None` for the 29 plain names). 7969717 is asserted
  `VoidLot`, as at HEAD (Deviations 3).
- `the_shared_shape_helpers_are_unchanged`: `summary_shape`, `names_a_legal_form` and `names_something` on 511's
  specimens and on 520's D names, and `named_before_stem` through the boundary stems.

Flipped assertions in `a_summary_that_names_an_award_beside_a_void_lot_is_an_award_summary`:
- Sindarro moved from the summary list to the void list.
- `Sublote 9.1 … 9.2: Declarado desierto.` now expects `VoidLot`; it was the "accepted cost".

**Test literals.** I ran all 1,198 distinct string literals of HEAD's partyname test module through `not_a_name`
before and after. Exactly 2 flip: Sindarro and Sublote 9.x, AwardSummary → VoidLot. These are the decision's two.

## EXITs (gate flags + gate package set, never piped, `> file; echo EXIT=$?`)

| run | command tail | EXIT | result |
|---|---|---:|---|
| partyname tests | `--lib -- partyname::` | 0 | ingest 49 passed, 2 ignored (511's simulations), 0 failed |
| all lib suites, first | `--lib` | 101 | ingest 520 passed, 1 failed: `project::tests::a_template_with_teds_pointer_drops_and_the_lot_numbered_compound_is_kept` (17 ≠ 15; Deviations 1) |
| all lib suites, after the pin | `--lib` | 0 | ingest 521, model 3, store 182, tender_db (server) 207; 0 failed |
| integration neighbours | `--test project --test text --test r208 --test r209 --test project_golden --test project_incremental` | 0 | project 102, project_golden 11, project_incremental 51, r208 15, r209 19, text 13; 0 failed |

Disk after: target 14G, 6.6G free on `/`.

## Mutation check (each rule off in turn; `partyname::` tests; restore checked with `cmp`)

The mutants are in the scratchpad (`mut/mutations.py`, `mut/run.sh`). For every mutant, the file was restored
byte-identical (`restored=identical`), and the final file matched the snapshot. Each run's only `error:` line is
cargo's `test failed`; no mutant had a compile error.

| off | EXIT | failing tests |
|---|---:|---|
| D1 | 101 | one_lots_void_sentence_is_void, every_void_520… |
| D2 (a+b+c) | 101 | a_negated_earlier_or_intended_award_is_no_award, a_bidder_is_no_winner (3526210), every_void_520… |
| D2a | 101 | a_negated_earlier…, every_void_520… |
| D2b | 101 | a_negated_earlier…, every_void_520… |
| D2c | 101 | a_negated_earlier…, a_bidder_is_no_winner, every_void_520… |
| D3 | 101 | an_awardee_word_must_label_a_party, a_list_of_void_lots_is_void (1537305/3556403), every_void_520… |
| D4 (a+b+c) | 101 | a_list_of_void_lots_is_void, a_summary_that_names… (Sublote), every_void_520… |
| D4a | 101 | a_list_of_void_lots_is_void, every_void_520… |
| D4b | 101 | a_list_of_void_lots_is_void, every_void_520… |
| D4c | 101 | a_list_of_void_lots_is_void, a_summary_that_names…, every_void_520… |
| D5 | 101 | a_reason_segment_names_nobody, every_void_520… |
| D6 (a+b+c) | 101 | a_bidder_is_no_winner, a_summary_that_names… (Sindarro), every_void_520… |
| D6a | 101 | a_bidder_is_no_winner, every_void_520… |
| D6b | 101 | a_bidder_is_no_winner, every_void_520… |
| D6c | 101 | a_bidder_is_no_winner, a_summary_that_names…, every_void_520… |
| D7 | 101 | a_void_head_before_a_spilled_section_is_void, every_void_520… |
| all seven | 101 | all 7 rule tests, every_void_520…, a_summary_that_names…: 9 failed, 40 passed |

With all seven rules off (HEAD's behaviour), `every_kept_520_exemption_keeps_its_class` and
`the_shared_shape_helpers_are_unchanged` stay green. So the kept pin records HEAD's classes, and the helpers
are untouched.

## The 376 under the new code

Each name was classified alone with `not_a_name`, using the dry report's names as given (cut at 300).

| decision class | VoidLot | AwardSummary | None (a name) |
|---|---:|---:|---:|
| D (173 to move) | 170 (HEAD: 2) | 0 (HEAD: 145) | 3 (HEAD: 26) |
| D residual (12) | 1 (7969717, as at HEAD) | 10 | 1 (28310858) |
| A (57) | 0 | 50 | 7 |
| B (99) | 0 | 81 | 18 |
| C (29) | 0 | 26 | 3 |
| F (6) | 1 (31375951, cut name only, as at HEAD) | 5 | 0 |
| **total** | **172** (HEAD 4) | **172** (HEAD 317) | **32** (HEAD 55) |

- **The 3 D `None`** are the cut D1 rows 2378132, 31450417 and 31491056: their cut text holds no void stem. With the tail
  a challenger read past the cut, all three read VoidLot.
- **With those tails and 31375951's** (`61 lots attribués à 29 fournisseurs` → AwardSummary): VoidLot 174 (the 173 D plus
  7969717), AwardSummary 173, None 29.
- **No A, B, C or F org changes class**, and none of the 11 residual orgs does.
- **Matches the decision:** 173 orgs move, and 203 stay exempted (57 A + 99 B + 29 C + 6 F + 12 residual), as §6 expects.

## Deviations and interpretations

1. **`project.rs` changed (test only), against "partyname.rs only".**
   - `a_template_with_teds_pointer_drops_and_the_lot_numbered_compound_is_kept` pins how many of 12381194 ORG-4's 23
     copies read void. That count went from 15 to 17:
     - The EN copy `Lot not awarded (see section VI, point VI.2 …)` was kept by `named_before_spill`, whose head
       `lot not awarded see` held no stem. D7 drops the `see` and reads the void whole value.
     - The PT copy `Lote não adjudicado (consultar …)` was kept by `awardee_noun`, because `não` is not in `NEGATIONS`.
       D3 finds no party preposition after `adjudicado`.
   - Both copies are TED's void template. The party already dropped (15/23) and still drops. The 18715886 residue
     stays at 2/24.
   - The change is the count and its doc comment, 6+/4−.
   - The decision's simulation covered partyname's literals only, so it could not see this. It also means a
     multi-copy party's tally can move. U3's dry run measures that; see Risks.
2. **D4 interpretations.**
   - (b) Relaunch markers are scoped to their sentence, not the rest of the name. `Lots 1) et 2) infructueux et
     relancés. Lot 3) Dupont` keeps its winner. All D4 specimens still move.
   - (c) "Carries a void verdict" means the entry's text is itself `is_void_lot`. A plain `mentions_void` read
     `lot268 sanofi av - … - 282 sans suite` as void (one entry with a later stem), which flips an existing summary.
   - The text before the first designator is no entry (`Les`, `Marché numéro: 01072 -`). This matches the
     decision's simulated residual: 31497872's verdict sits in that prefix, and it stays kept.
3. **Pin carve-outs.**
   - 7969717 (residual) is asserted `VoidLot`, not "non-VoidLot". Its name reads void at HEAD too. The fold keeps its
     party by the 2/24 majority, which is 525's template tail, not `names_an_award`.
   - The four rows a challenger read in full carry the quoted tail joined with `…`:
     - 31450417: `… ascenseurs a été déclaré infructueux`;
     - 31491056: `… a été déclaré Sans Suite`;
     - 2378132: `… trois (3) titulaires seront désignés … a été déclaré infructueux`; the order of the two quoted
       pieces is assumed;
     - 31375951: `… 61 lots attribués à 29 fournisseurs`.
   - All other cut names are tested as cut.
4. **D2a uses `NEGATIONS`** (the `awardee_noun` set: `non`, `pas`, `sans`, `aucun`, `n`, `ne`, `no`, `not`, …) where
   HEAD used `non`/`pas`/`sans`, as "as `awardee_noun` already does" reads. It moved nothing outside D on the 376 or
   the literals.
5. **D6c's "no lot number"** means no designator followed by a digit word anywhere in the name.
6. **Already void on their cut names at HEAD:** 3720532 (D3) and 4885215 (D4). Their keep signal lies past the
   300-character cut, so the pin cannot test it. U3's dry run reads full values.
7. **Tests first.** The rule tests were written after a first implementation draft, not seen red before it. The
   red evidence is the mutation table: every rule test fails with its rule off, and all of them fail with all seven
   off.

## Risks for U3

- **Multi-copy parties.** D3 and D7 also quiet copies whose org is not one of the 376, as the EN and PT copies of the
  TED pointed template show. A kept multi-copy party can therefore cross the majority rule. *Corrected by the review
  (F18, below):* §6's stop band cannot catch a party whose head copy is outside the walk's net, because the walk reads
  head names only. Measure such parties on prod before the wet run (see Review fixes, "Rejected or not fixed in
  code"). Read every re-queued org that is not on Appendix A's D list, as §6 already requires.
- **Cut names.** Of the 63 cut names, the dry run sees the full values. 3720532 and 4885215 above, and every cut A, B, C
  and F row, are covered only by §6's stop rule: all of A, B, C and F must stay in `exempted`.

## Commit file list

- `crates/ingest/src/partyname.rs`: the rules, docs and tests (+1290/−37).
- `crates/ingest/src/project.rs`: the one test pin (+6/−4; Deviations 1).
- `.scratch/tender-db/520-award-summaries/u2-build-notes.md`: this file (scratch).

Before staging, run `git diff` on both source files: the tree held no other agent's hunks when this build started.

## Review fixes (2026-10-13)

Three adversarial reviewers sent 29 findings. Several are the same defect seen twice: F1 and F20; F5 and F14; F6, F16
and F22; F8 and F21; F9 and F27; F10, F17 and F23. Each finding was checked against the code, still in the working
tree and **not committed**. No gate was run, and nothing on prod was touched.

**How they were checked.** A temporary `#[ignore]` probe inside partyname's test module compiled three versions in
one test binary: HEAD's partyname (`git show HEAD`), the U2 build (a snapshot taken before these fixes), and the
fixed code. Each name read from a file got all three classes. The probe and its two module copies are deleted, and
the scratch copies stay in this session's scratchpad (`r520fix/`). All 96 strings in the findings reproduced exactly
as reported on the U2 build. The new tests were also run against the U2 code, with the fixed test module on top of
the U2 non-test code. All 7 rule tests fail there (42 passed, 7 failed). The file was restored byte for byte
(`cmp`).

### What was fixed

Every finding rated must-fix or should-fix was accepted. Each has a test, written as a near-miss block in its rule's
test with id 0.

| finding(s) | rule | fix (all in `partyname.rs`) |
|---|---|---|
| F1, F19/F20 (must) | D4c | `numbered_entries` and `lettered_entries` count again as at HEAD, outside the gate. Each already needs an entry that names something, and an entry the designators miss (`- 44 reckitt …`, `/ 2 : sans suite`) is one of theirs. Only `several_lots` is gated by `every_lot_void`. |
| F2 (must) | D4c | An entry with nothing after its label keeps the summary, wherever it stands: `Paredes lot 506 - …`, `Orthomed : lots nº 25 et 26 / …`. The label no longer swallows a trailing conjunction. An entry whose text is only a conjunction joins the next one (`Perceel 2 en Perceel 4`); as the last entry it keeps. |
| F3 (should) | D4c | `lot_label` runs on past the first number only across a number's punctuation (`number_gap`: spaces, `. - / , ° º`, no `. `). It runs on only over `label_number` (digits, digits plus a sub-lot letter, a lone letter, a Roman numeral), a number mark that a number follows, or a conjunction that a number follows. A `:` or `)` ends the label, so `Lot 1 : 3M` and `Lot 1 : A.C.H.` keep their winner. |
| F4 (should) | D4c | `void_entry`: an entry is void only when its head (`reason_head`) says void, so no name stands before the verdict, **and** the whole entry reads void. `: Dupont (marché négocié, AO infructueux)` now keeps the list. |
| F15 (should) | D4 | `several_lots(words) && several_lots(own)`. The arm only ever quiets, so a name that is VoidLot at HEAD stays VoidLot (`Infructueux. Lot 3 : 2 offres reçues. Lot relancé. 4 offres`). |
| F5, F14 (must/should), F25 (should), F26 (nit) | D2a, D2b | The negation is looked for only in the verb's clause, after the last `clause_break`: `, ; : ( ) [ ] … – —`, a line break, `. ` or ` - `. Only `CLAUSE_FILLERS` are skipped, which is `NEGATION_FILLERS` without its nouns. `qu`/`que` right after the verb cancels the negation (`n'a été attribué qu'après`). `déjà` is found past the fillers too (`déjà été attribué` → VoidLot). When the runs and the fold's words do not line up, HEAD's one-word-back reading applies, which errs toward keeping. |
| F12 (nit), F11b (should), F24 (should) | D2c, D6b | A new `intent_spans` replaces "an intent anywhere earlier in the sentence". A span runs from the intent word to the first award already made (`PAST_AWARDS`, `a été`, `ont été`) or the first new lot label, or else to the end of the sentence. A lot label still belongs to the span when the intent's own `avec les sociétés` cue comes before it, so 3526210 keeps its D2c+D6b reading. D2c's `avec la société` cue and D6b's company forms are read through it. |
| F24 (should) | sentences, D4b | `sentences` also cuts at a line break, at `…`, and at a `.` glued between a word of two or more letters and a capital (`relancé.Lot`; the dots of `S.A.` and `S.p.A.` do not cut). D4b's relaunch cut ends at the summary's next entry label (`entry_label`: a lot label followed by `:` or `)`, as in `Lot 1 infructueux et relancé, lot 2 : Dupont`). The new procedure's own lots (`lot nº 06 — …`, `lot no 3 - agence …`) are not written that way. |
| F6, F16, F22 (should/nit) | D3 labels | `labels` accepts a `(s)`, an entry number with its mark (`n°1`, `n° 1`), up to six `LABEL_COMPLEMENTS` words (`du marché`, `/ organisme`, `après négociation`), a full-width `：`, and a spaced dash before a capitalised word (`Attributaire - Keolis`). |
| F6, F8, F21 (should/must) | D3 prose | `noun_names`: a noun followed straight away by a capitalised word that is no lot, negation or void word (`titulaire Dupont Frères`), or by `COMPANY_NOUNS` after an optional article (`adjudicataria la empresa X`, `aggiudicataria la ditta X`). For `retenu(e)` (`participle_names`): a copula before it whose subject is no offer, candidate, awardee noun or lot (`la société Storz a été retenue`); a copula and then a capitalised word after it (`l'entreprise retenue est Dupont Frères`); or a capitalised name right before it (`Keolis retenu`). `(mandataire solidaire)` also counts. |
| F7 (should) | D3 participles | `PARTY_PREPOSITIONS` are now set per language (`awarded`→`to`, `gegund`→`aan`, `adjudicad*`→`a/al`, `aggiudicat*`/`attribuit*`→ the Italian set). The preposition may stand up to `PARTICIPLE_REACH` (6) words on, in the same clause. `awarded as part of a new project` and `wordt gegund op totaal der percelen` still name nobody. |
| F9, F27 (should) | D5 | A segment counts as a reason only when `offer_failure` holds: an offer word beside a rejection (`OFFER_REJECTIONS`), a failure stem (`OFFER_FAILURES`: `odrzuc`, `wyklucz`, `brak`, `zadn`, `renunci`, …) or a negation (`OFFER_NEGATIONS`: the `NEGATIONS` that cannot be a number mark, plus `nie`, `geen`). All 12 D5 orgs still move. |
| F10, F17, F23 (should/nit/must) | D1 | `le lot` may appear only once, at a word boundary, with or without its colon. The verdict's subject must not be another lot (`lot 2`, `lot n 2`, `lot no2`, `lot2`, `lot 2b`). A sub-lot code that closes the title is not another lot: the first draft of the subject check moved 5772779, 5772782 and 5772784 (`(sous lot nob06) a été déclaré`) to AwardSummary, the recount caught it, and the check now reads only a lot number. |
| F11a, F11c (should) | D6a, D6c | `rejected_offer_spans`: a form counts as a bidder only inside a rejected offer's span. The span runs from the offer word to the next offer word, to an award (`PAST_AWARDS`, `retenu…`), or to the end of the clause, where commas do not cut (`l'offre de la société X SA, jugée irrégulière, …` stays a bidder). A rejection before the first offer word of the clause still counts (11251840). `bidder_before_void_lot` now also requires that nothing after the verdict holds a company form or an award word (`marché notifié à Société Dupont` keeps). |
| F13 (nit) | D7 | The part of the head after its last `:` or `;` keeps the party when it names something (`names_something`) and says nothing void, gives no negation and names no offer (`re-tendered: Acme Construction` keeps; `Desierto: Ninguna oferta` stays void). |

Beyond the findings, 18 near misses of my own were run in both directions. All of them read as expected, and the
ones that belong to a rule are in that rule's test. Examples: `L'offre de la société Dupont SA, jugée irrégulière, a
été écartée` stays VoidLot; `Lot 1 et Lot 2 : infructueux` stays VoidLot; `Infructueux, 1 offre irrégulière,
relancé … conclu avec Promocash SAS` reads AwardSummary.

### Rejected or not fixed in code

- **F24, in part.** No sentence cut at ` - ` or `,`. 3526206 to 3526210 put both inside the intent's own clause
  (`à relancer …, en application de l'article 35 - i - 1er alinéa …, avec les sociétés`), and a cut there would lose
  D2c. The lot-label stop in `intent_spans` and in D4b covers the reported ` - Lot 2:` and `, lot 2 :` shapes instead.
- **F28 (nit, HTML entities): not fixed.** The fix is not trivial. `n&#39;a` folds to `n 39 a`, so decoding would
  have to happen before every arm, which changes the input of the HEAD helpers as well. The error leans toward
  keeping the party, and no name among the 376 holds `&#`.
- **F18 (nit): a note correction, no code.** The Risks paragraph above was wrong. The walk's net reads only the
  org's head name (supervisor.rs `NonNames::net`). The fold drops a party with 4 or more copies once half of them read
  void (project.rs `drop_decision`). So a party whose head copy is outside the net, while other copies change from a
  name to VoidLot under U2, can be dropped on any refold without `refold-void-names` ever listing it. §6's band of 165
  to 181 does not see it. The probe found which copies change. Over 31,720 literals, the changes outside the 376
  are 2 `None → VoidLot` (TED's EN and PT pointed copies) and 5 `AwardSummary → VoidLot`. Among the 376, 23 plain
  names (`None`) and 145 summaries move. A multi-copy party can be affected only through a copy that read None or
  AwardSummary at HEAD and now reads VoidLot. A party with any copy still read as AwardSummary is kept whatever its
  tally (`drop_decision`). **Before U3's wet run,
  measure it on prod:** list the parties with 4 or more copies whose head name is outside the net but where some
  copy is `mentions_void` (a bounded `organization_names` read per such party, or a fold dry run that reports
  `KeptParty` tallies changing between builds). Only then can U3 be said to converge. This is not measured here
  (no prod access in this unit).

### The 376 under the fixed code

Each name was classified alone with `not_a_name`, using the dry report's names (cut at 300 characters). The result
is **identical to the U2 build, org for org**. No org changed class between U2 and the fix.

| class | VoidLot | AwardSummary | None |
|---|---:|---:|---:|
| D1 (68) | 65 | 0 | 3 (the cut 2378132, 31450417, 31491056) |
| D2a 8 / D2b 3 / D2c 6 | 17 | 0 | 0 |
| D3 (39) | 39 | 0 | 0 |
| D4 (11) | 11 | 0 | 0 |
| D5 (12) | 12 | 0 | 0 |
| D6 (9) | 9 | 0 | 0 |
| D7 (14) | 14 | 0 | 0 |
| D3+D4 (2), D2c+D6b (1) | 3 | 0 | 0 |
| D residual (12) | 1 (7969717, as at HEAD) | 10 | 1 (28310858) |
| A1 (47) | 0 | 40 | 7 |
| A2 (10) | 0 | 10 | 0 |
| B1 13 / B2 10 / B3 76 | 0 | 81 | 18 (all B3) |
| C (29) | 0 | 26 | 3 |
| F-P (4) | 1 (31375951, cut name only) | 3 | 0 |
| F-role (2) | 0 | 2 | 0 |
| **total** | **172** | **172** | **32** |

With the four tails a challenger read: VoidLot 174 (all 173 D plus 7969717), AwardSummary 173, None 29. That is 173
orgs moving and 203 staying exempted, as §6 expects.

### Differential over all string literals

The probe read 31,720 distinct string literals: every `.rs` file under `crates/` plus HEAD's partyname.
- **U2 → fixed:** 0 changes.
- **HEAD → fixed:** 7 changes, all to VoidLot, the same 7 as HEAD → U2:
  - the two flips the decision intends (Sindarro; `Sublote 9.1 … 9.2`);
  - Sindarro's short form;
  - `Les lots 2 et 11 sont déclarés sans suite. Le lot 10 est déclaré infructueux`;
  - `Infructueux. Date d'attribution : CAO du 17.4.2013.`;
  - TED's EN and PT pointed copies (Deviations 1).

**One-way invariant, as measured.** No probed name (the 31,720 literals, the 380 names of the 376 with their tails,
and the 114 specimens) that is VoidLot at HEAD reads otherwise on the fixed build. No name moved between AwardSummary
and None either.

### EXITs (gate flags + gate package set, never piped, `> file; echo EXIT=$?`)

| run | command tail | EXIT | result |
|---|---|---:|---|
| red check: fixed tests on the U2 code | `--lib -- partyname::` | 101 | ingest 42 passed, 7 failed (all 7 rule tests); file restored, `cmp` identical |
| partyname tests | `--lib -- partyname::` | 0 | ingest 49 passed, 2 ignored (511's simulations), 0 failed |
| all lib suites | `--lib` | 0 | ingest 521 (the project.rs TED pin still 17), model 3, store 182, tender_db (server) 207; 0 failed |
| integration neighbours | `--test project --test text --test r208 --test r209 --test project_golden --test project_incremental` | 0 | project 102, project_golden 11, project_incremental 51, r208 15, r209 19, text 13; 0 failed |

None of the logs holds an `error`, `FAILED` or `panicked` line. The probe runs (EXIT 0 each) are not counted here;
the probe is gone. Disk after: target 14G, 6.2G free on `/`. `project.rs` is unchanged since the U2 build.

### Commit file list (updated)

- `crates/ingest/src/partyname.rs`: U2's rules and these fixes, with docs and tests. Against HEAD the change is
  +1858/−38.
- `crates/ingest/src/project.rs`: the one test pin (+6/−4), unchanged.
- `.scratch/tender-db/520-award-summaries/u2-build-notes.md`: this file.

Before staging, run `git diff` on both source files. The tree held no other agent's hunks during this pass. The
next step is `ops/check.sh`; F18's prod measurement must come before U3's wet run.
