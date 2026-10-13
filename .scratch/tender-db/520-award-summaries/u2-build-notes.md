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

## Second review fixes (2026-10-13)

A second adversarial review of f57324f (520 U2, gated green) returned 8 verified findings and 2 nits (this session's
scratchpad, `520u2-review2.json`, keys `real[]` and `nits[]`; each verifier's minimal fix was read before building).
All are fixed in the working tree, **not committed**. `ops/check.sh` was not run (out of scope for this pass), and
nothing on prod was touched. Only `crates/ingest/src/partyname.rs` changed; `project.rs` is as U2 left it.

**How they were checked.** A temporary `#[ignore]` probe, a `#[path]` module inside partyname's test module,
compiled three copies of the non-test code into one test binary: fe4fe13's partyname (before 520, the base for the
invariant), f57324f's, and the worktree's. Each string got all three classes. The probe and its hook are deleted;
`git status` shows only `partyname.rs` and this file. The scratch copies, the probe and the outputs are in this
session's scratchpad (`r520fix2/`). Of the 58 specimens written from the findings, 40 read wrong at f57324f, each as
its finding said; the other 18 are near misses that f57324f already read right and that the fixes must keep.

### What was fixed

| finding | rule | fix (all in `partyname.rs`) |
|---|---|---|
| D1 later lot (should) | D1, `lot_title_void` | `later_lot`: the name is cut at `sentences` and then at ` - `, ` – `, ` — `. If any piece after the first opens on `lot`/`lots`, an optional number mark and a lot number (or a glued `lot2`, `lotno2`), D1 does not fire. The title's own designators stand in the opener's piece (`Le lot 102 : lot B10:`, `le lot 2 : Lot no2 - sorties …`). No D1 name among the 376 has a lot number after a break, and all 68 still move. |
| D6a comma (should) | D6a, `rejected_offer_spans` | An offer's span also ends at the first comma after its rejection has been read (`offres irrégulières, marché négocié avec Dupont SARL`). A comma before the rejection still does not cut, so the owner in `l'offre de la société X SA, jugée irrégulière, …` stays a bidder. |
| PAST (should) | D2c and D6b, `intent_spans` | New `PAST_PARTICIPLES` (`passé`, `signé`, `confié`, 12 forms) end an intent's span when no `sera`/`seront`/`être` stands between the intent word and the participle. They are not added to `PAST_AWARDS`, so 1506401 (`sera passé avec la STÉ France Télécom`) stays void. |
| D3 label complements (should) | D3, `labels(after, open)` | For an awardee noun, the complement before `:` may be any word that is not a `NEGATIONS`, `COPULAS` or `AWARD_NOUNS` word, still at most 6 words (`Titulaire du marché négocié :`, `Attributaire désigné :`, `attributaire unique :`). Participles keep the closed `LABEL_COMPLEMENTS`, so `gegund op totaal der percelen = perceel 3` (4802635, 4802636) stays nobody. |
| D3 copulas (should) | D3, `party_after_copula` | After a copula comes an optional article (`la`/`le`/`l`/`les`), then a `COMPANY_NOUNS` word or a capitalised word that is not a lot, negation or void word. `participle_names` uses it in place of "a capital right after" (`l'entreprise retenue est la société Dupont`). `noun_names` gains it as a new branch (`le titulaire est Dupont Frères`, `l'attributaire est l'entreprise Dupont Frères`). `le titulaire est défaillant` stays nobody. |
| D4b relaunch cut (should) | D4b, `entry_label` | A lot label followed by a capitalised word that is not void now also opens a summary entry, so the relaunch cut ends there (`…relancé, lot 2 Dupont Frères, lot 3 Martin`). The new procedure's lots in the D4b specimens are written with a dash and a lowercase title (`lot nº 06 — bâtiments`, `lot no 3 - agence`), so they stay cut. |
| D7 (must) | D7, `named_before_spill` | `named_last` now also requires `base_keeps`: the folded head before ` section vi ` holds no `VOID_STEMS` stem, which is exactly the case where 510 kept the name. `Infructueux : Voir section VI.3 …`, `Infructueux : LOT 3. SECTION VI …`, `Lot 3 déclaré sans suite : Se reporter à la section VI.3` and `Lot déclaré infructueux : Relance en procédure négociée. SECTION VI` are void again. The Acme `re-tendered:` case is still a name. |
| D2a (should) | D2a, `award_clause` | The 510 reading is the floor in both branches: `non`/`pas`/`sans` right before the verb negates it whatever stands between (`Lot 2 : Non - Attribué`, `non\nattribué`, `NON – ATTRIBUÉ`). The `ne … que` cancel applies only when the negation is `n'`/`ne` (`Lot 2 non attribué, que la commission relancera` stays negated). |
| nit 0 (trivial, fixed) | D2b and `ne … que`, `award_clause` | `déjà` blocks the award unless the verb is followed by `à`, an optional article, and a word that is neither `été` nor a lot designator (`déjà été attribué à l'entreprise Dupont Frères` keeps it; `déjà attribué au lot`, `aux lots` and `Lot déjà attribué a été déclaré` stay another procedure's). The restrictive `n'` accepts a `qu'`/`que` anywhere later in its sentence (new `sentence_end`: `;`, line break, `…`, `. `), so `n'a été attribué, après négociation, qu'à la société Martin` keeps it. |
| nit 1 (trivial, fixed) | D5, `offer_failure` | An `OFFER_NEGATIONS` word counts only within `OFFER_REACH` (4) words of an offer word (`seule offre reçue : Dupont Frères, retenue sans négociation` names its winner). Rejection and failure stems still count anywhere in the segment. All 12 D5 orgs still move. |

One shape moves past both readings: `… le lot n'a pas été attribué, que …` is now VoidLot. Both fe4fe13 and f57324f
read it AwardSummary, because f57324f took the `que` after `pas` as restrictive. It is a negated award naming no
one. It is the only literal whose class differs between f57324f and the worktree, and it is a test written in this pass.

### The invariant, by construction

Every arm of the worktree's `names_an_award` now implies an arm of fe4fe13's:
- **`award_clause`.** The verb branch requires `words[i-1] ∉ {non, pas, sans}`, which is 510's own test. The cue
  branch reads per-sentence folds whose words are the whole fold's words: `match_norm` splits on the same characters
  as `alphanumeric_runs`, and `fold_latin` maps one char to one char. So a cue found there is also in 510's padded fold.
- **The wrapped arms.** `award_awardee_noun` requires `awardee_noun(words)`. `award_several_lots` requires
  `several_lots(words)` or a numbered or lettered entry. `award_mixed_segments` requires `mixed_segments`, and
  `award_company_token` requires `company_token`.
- **`named_before_spill`.** The misaligned branch is 510's reading. In the aligned branch, `!mentions_void(head)`
  implies no stem in the head, since `mentions_void` checks the stems and no stem runs into the trailing `see` it
  drops. `named_last` now requires `base_keeps`.
- **`lot_title_void`** only returns false.
- **Every other arm** is unchanged byte for byte.

`classify` calls `names_an_award` only to choose AwardSummary (or None) over VoidLot. So a name that is VoidLot at
fe4fe13 is VoidLot now.

### Measured (probe, the final file)

The probe set has 32,256 distinct strings:
- every string literal of the 224 `.rs` files under `crates/`, the worktree with this pass's tests (31,822 not
  already among the inputs below);
- every string literal of fe4fe13's `partyname.rs` and `project.rs` and of f57324f's `partyname.rs`;
- the 376 exempted names of `void-names-refold-after-dry.json`, 380 strings with the four tails;
- the 58 finding specimens.

1. **INVARIANT.** 0 strings that read VoidLot at fe4fe13 read anything else now. At f57324f there were 9 such
   strings: the D7 and D2a specimens.
2. **The 376.** They classify identically to f57324f, org for org, which is the decision's expectation:

   | | D (173) | D residual (12) | A (57) | B (99) | C (29) | F (6) |
   |---|---|---|---|---|---|---|
   | fe4fe13, as cut | AS 145, N 26, V 2 | AS 10, V 1, N 1 | AS 50, N 7 | AS 81, N 18 | AS 26, N 3 | AS 5, V 1 |
   | worktree, as cut | V 170, N 3 | AS 10, V 1, N 1 | AS 50, N 7 | AS 81, N 18 | AS 26, N 3 | AS 5, V 1 |
   | worktree, with the four tails | V 173 | AS 10, V 1 (7969717), N 1 | AS 50, N 7 | AS 81, N 18 | AS 26, N 3 | AS 6 |

   - All 173 D orgs read VoidLot, plus 7969717.
   - No A, B, C, F or residual org changes class against fe4fe13.
   - The 3 D rows that read N as cut are 2378132, 31450417 and 31491056, as before.
   - The F row that reads V as cut is 31375951, as before.
3. **Every class change against fe4fe13.** 200 strings change, all of them to VoidLot (172 AwardSummary → VoidLot,
   28 None → VoidLot). No string moves between AwardSummary and None.
   - **171 are names among the 376, all D:** D1 68, D3 38, D7 14, D5 12, D4 10, D6 9, D2a 8, D2c 6, D2b 3, D3+D4 2,
     D2c+D6b 1. The cut 3720532 and 4885215, and 7969717, already read VoidLot at fe4fe13.
   - **29 are other literals, all test strings of 520:**
     - the decision's two intended test flips: Sindarro (both forms) and `Sublote 9.1 … 9.2: Declarado desierto.`;
     - the U2 notes' documented four: `Les lots 2 et 11 sont déclarés sans suite. Le lot 10 est déclaré
       infructueux`, `Infructueux. Date d'attribution : CAO du 17.4.2013.`, and TED's EN and PT pointed copies in
       `project.rs` (Deviations 1);
     - U2's own void near misses (13): `Le lot 3 : façade brique (sous lot nob06) …`, `Lot infructueux : l'offre a
       été retenue puis retirée`, `Lot 2 : sans suite. Le candidat retenu s'est désisté`, `Pas d'attributaire retenu
       pour l'instant. …`, `Lot 1 : infructueux. Lot 2 : sans suite`, `Lot 1 et Lot 2 : infructueux`, `Lots 1 et 2 :
       infructueux. Lot 3 : sans suite`, `Lote 1: declarado desierto. Lote 2: declarado desierto`, `Lot 1 : déclaré
       infructueux (offre de Dupont SARL irrégulière). Lot 2 : sans suite`, `Not awarded: no bids received. SECTION
       VI …`, `Desierto: Ninguna oferta. SECTION VI …`, `L'offre de la société Dupont SA, jugée irrégulière, a été
       écartée. Lot infructueux`, `Lot sans suite (déjà été attribué au lot A 12)`;
     - this pass's void near misses (9): `Le lot 4 : Voirie ; lotissement des Prés …`, `Lot 1 infructueux et relancé,
       Lot 2 Sans suite`, `Lot infructueux : attributaire non désigné : voir avis ultérieur`, `Lot infructueux :
       l'attributaire n'est pas désigné`, `Lot infructueux : l'offre de Martin SA, irrégulière, a été écartée`, `Lot
       infructueux, le titulaire est défaillant`, `Lot infructueux, à relancer : le marché sera ensuite passé avec la
       société Dupont`, `Lot sans suite, déjà attribué aux lots 3 et 4`, `Lot infructueux : le lot n'a pas été
       attribué, que la commission relancera`.

   Each of the 29 is void prose that names no winner. Every one except the last already read VoidLot at f57324f.
   Between f57324f and the worktree, the literals that were already in the tree changed 0 times.

### Tests added (partyname.rs, near misses with id 0)

| test | finding | kept (`AwardSummary` / `None`) | void (`VoidLot`) |
|---|---|---|---|
| `one_lots_void_sentence_is_void` | D1 | the reviewer's 5, plus ` - lot 2 (Sols)`, ` – lot n° 2 :`, `. Lot2 :` | `- lots techniques`, `; lotissement des Prés` (no lot number) |
| `a_negated_earlier_or_intended_award_is_no_award` | D2a, PAST (D2c), nit 0 | `… marché passé avec la société Dupont`, `n'a été attribué, après négociation, qu'à …`, `déjà été attribué à l'entreprise Dupont Frères` | `Non - Attribué`, `non\nattribué`, `NON – ATTRIBUÉ`, `non attribué, que …`, `n'a pas été attribué, que …`, `sera ensuite passé avec la société Dupont`, `déjà attribué aux lots 3 et 4` |
| `an_awardee_word_must_label_a_party` | D3 labels, D3 copulas | 6 labels and 4 copula forms, each also `awardee_labels` | `attributaire non désigné :`, `le titulaire est défaillant`, `l'attributaire n'est pas désigné`, each also `!awardee_labels` |
| `a_list_of_void_lots_is_void` | D4b | `…, lot 2 Dupont Frères, lot 3 Martin`, `Lots 1 et 2 … relancés, lot 3 Dupont Frères` | `…relancé, Lot 2 Sans suite` |
| `a_reason_segment_names_nobody` | nit 1 | `seule offre reçue : Dupont Frères, retenue sans négociation`, also `award_mixed_segments` | — |
| `a_bidder_is_no_winner` | D6a, PAST (D6b) | the reviewer's 4 D6a names and 3 `passé`/`signé`/`confié` names, each also `award_company_token` | `l'offre de Martin SA, irrégulière, a été écartée`, also `!award_company_token` |
| `a_void_head_before_a_spilled_section_is_void` | D7 | the Acme `None` (unchanged) | the reviewer's 4 and `Déclaré sans suite : Motif d'intérêt général. SECTION VI …`, each also `!spilled` |

**Red check.** The final test module was run on f57324f's non-test code: 42 passed and 7 failed, the 7 being the
rule tests. EXIT 101. The file was restored byte for byte (`cmp`).

**Mutation check.** Each fix was turned off in turn (13 mutants) and the `partyname::` tests were run. Every mutant
had EXIT 101, no compile error, and the file restored identical (`cmp`). The mutants are in `r520fix2/mut/`.

| off | failing tests |
|---|---|
| D1 later lot | one_lots_void_sentence_is_void |
| D2a floor | a_negated_earlier_or_intended_award_is_no_award |
| D2a restrictive `n'` only | a_negated_earlier_or_intended_award_is_no_award |
| nit 0 `que` later in the sentence | a_negated_earlier_or_intended_award_is_no_award |
| nit 0 `déjà` to a party | a_negated_earlier_or_intended_award_is_no_award |
| D3 free label words | an_awardee_word_must_label_a_party |
| D3 noun copula | an_awardee_word_must_label_a_party |
| D4b capitalised entry | a_list_of_void_lots_is_void |
| D6a comma | a_bidder_is_no_winner |
| PAST participles | a_bidder_is_no_winner, a_negated_earlier_or_intended_award_is_no_award |
| PAST future guard | a_bidder_is_no_winner, a_negated_earlier_or_intended_award_is_no_award, every_void_520_exemption_is_a_void_lot (1506401) |
| D7 `base_keeps` | a_void_head_before_a_spilled_section_is_void |
| nit 1 offer reach | a_reason_segment_names_nobody |

### Left as they are

- **D4b, a lowercase winner after a relaunch** (`…relancé, lot 2 dupont`) stays cut, as the D4b verifier noted.
  Opening an entry on any word without a dash would come close to the D4b specimens' own shape.
- **D5, a negation inside `OFFER_REACH`** still reads as a reason (`offre de Dupont SARL non anormalement basse`).
- **D1's later-lot check reads `lot`/`lots` only.** BOAMP's sentence is French, and the other designators
  (`tranche`, `partie`, `secteur`) are common in titles.

### EXITs (gate flags + gate package set, never piped, `> file; echo EXIT=$?`)

| run | command tail | EXIT | result |
|---|---|---:|---|
| red check: final tests on f57324f's non-test code | `--lib -- partyname::` | 101 | ingest 42 passed, 7 failed (the 7 rule tests); file restored, `cmp` identical |
| mutation sweep (13 mutants) | `--lib -- partyname::` | 101 each | each fails its rule's test (table above); file restored, `cmp` identical |
| partyname tests | `--lib -- partyname::` | 0 | ingest 49 passed, 2 ignored (511's simulations), 0 failed |
| all lib suites | `--lib` | 0 | ingest 521, model 3, store 182, tender_db (server) 207; 0 failed |
| integration neighbours | `--test project --test text --test r208 --test r209 --test project_golden --test project_incremental` | 0 | project 102, project_golden 11, project_incremental 51, r208 15, r209 19, text 13; 0 failed |

None of the three green logs holds an `error`, `FAILED` or `panicked` line. The probe runs (EXIT 0 each) are not
counted. Disk after: target 14G, 6.9G free on `/`.

### Commit file list (updated)

- `crates/ingest/src/partyname.rs`: U2's rules, the first review's fixes and these fixes, with docs and tests.
  The change is +2074/−38 against fe4fe13 and +262/−46 against f57324f.
- `.scratch/tender-db/520-award-summaries/u2-build-notes.md`: this file.

`project.rs` has no change against f57324f. Before staging, run `git diff` on `partyname.rs`: the tree held no other
agent's hunks during this pass. Next comes `ops/check.sh`, and F18's prod measurement must still come before U3's wet run.

## Third check (wf_cd50049c-c58's independent check, then the owner)

- The independent check confirmed all 8 second-review fixes. It also confirmed the invariant (no name that reads
  VoidLot at fe4fe13 reads anything else now) two ways: by construction, and over 32,363 strings compiled side by side.
- It found one new problem in the costly direction (low). `party_after_copula` refused any capitalised party opening on
  a lot designator word ('le candidat retenu est Zone Ouest Transports' / 'Secteur Nord BTP'), and a name cut right
  after its article. Fixed by the owner: a designator is refused only as a lot with its number, and a capitalised
  article that ends the name stands for the party. Both specimens were added to the D3 kept near misses. Reverting the
  fix makes `an_awardee_word_must_label_a_party` fail (EXIT=101), and the file was restored byte-identical.
- The check's other findings are recorded, not fixed. All are in the cheap direction (a junk leaf name stays kept;
  never a lost award, never an un-void against fe4fe13):
  - the 'ne … que' cancel reaching a subordinating 'que';
  - OFFER_REACH=4 missing far negations;
  - D4b's capitalised-title end of a relaunch;
  - open noun labels whose complement names nobody ('attributaire pressenti : aucun');
  - `to_party` after 'déjà … attribué à';
  - the D6a comma cut keeping a bidder named after it.
  The U3 dry run's listing shows their cost on prod.
