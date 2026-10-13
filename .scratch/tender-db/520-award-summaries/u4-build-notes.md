# 520 U4 build notes: A1 winner extraction (2026-10-13)

Built against `unit1-decision.md` §4.2 (the A1 rule), §5 U4, §6 U5, §7 and Appendix B, with the names of
`../525-void-templates/void-names-refold-after-dry.json` / `void-names-refold-2360-dry.json` (`exempted[]`) and the
classes of `unit1-classified.json` + Appendix A. In the working tree, **not committed**, `ops/check.sh` not run (out of
scope for this build), nothing on prod touched.

## What changed

### `crates/ingest/src/partyname.rs`

- **`pub fn award_winner(name) -> Option<String>`** (§4.2). Cues, on the fold's words with the raw runs beside them:
  - (a) an `attribué(e)(s)` that states an award — `award_verb`, the verb branch of `award_clause` lifted out one word at
    a time (byte-for-byte the same reading: 510's `non/pas/sans` floor, D2a's clause negation, the `ne … que` cancel,
    D2b's `déjà`) — with no `RELAUNCH_INTENT` / `FUTURE_AWARDS` (`seront`, `serait`, `seraient`, `être`) word before
    it in its clause, then `à` (as published, `à`/`À`, never the verb `a`) within `AWARD_REACH` = 8 words in its
    sentence, and an optional `:`;
  - (b) `d'attribuer ce lot à`;
  - (c) the labels: `attributaire` / `titulaire` straight before `:` (or `：`), not negated (`negated`), which covers
    `Nom du titulaire:`, `Nom de l'attributaire:`, `Attributaire:`, `titulaire:`; and `fournisseur retenu =`;
  - (d) `avec la société X` / `avec l'entreprise X` **closing the name**, after a past relaunch (`PAST_RELAUNCH`:
    `relancé(e)(s)`, `négocié(e)(s)`) and with no `RELAUNCH_INTENT` / `FUTURE_AWARDS` word anywhere before it; a
    (d) that does not close the text is no cue;
  - (e) `la société X a été retenue` / `l'entreprise X a été retenue` (subject within 7 words), with no relaunch to
    come after it in its sentence.
- **Strip** (`strip_winner`): the descriptor (`la Société`, `l'entreprise`, `L'entreprise`, `l'esat`, `la STÉ` — after
  an article in any case; bare, only lower-case `société`, so `Société Générale` would keep its name), a lower-case
  article (`la Sarltacavl` → `Sarltacavl`; `La Poste` keeps `La`), quotes (`"…"`, `«…»`, `“…”`), edge punctuation.
- **Terminators** (`winner_end`): `,`, ` (`, `;`, ` / `, ` - `/` – `/` — `, ` pour un montant`, whole-word ` suite` and
  ` SISE` (ASCII case-blind), ` le <digit>`, a five-digit postcode after a space, a sentence end (a `.` after no single
  letter: `A.E.C. Castellane`, `C.A.J.L` stay whole), the next cue's start, the end of the text.
- **None** when: no cue; the name opens with no letter (`29 fournisseurs`); its first word is a count
  (`COUNT_NOUNS`) or a pointer (`WINNER_POINTERS`: `voir`, `cf`, `see`, …); the name is itself a non-name
  (`not_a_name(name).is_some()` or `mentions_void(name)`); two or more distinct names (fold-compared); a list
  separator after the name opens a segment that is not void (`listed_after`, 31414415). A cue whose name fails any
  check makes the whole answer `None`.
- **`pub fn may_name_a_winner(name)`**: the allocation-free gate (`attribu`, `titulaire`, `retenu`, `avec l`, ASCII
  case-blind) the fold runs before it reads a party's copies; `award_winner` opens with it too.
- `award_clause` now calls `award_verb` per word; the body moved unchanged (the D2 tests and the pins are green).
- Module doc: one U4 paragraph. Tests: below.

### `crates/ingest/src/project.rs`

- `DropDecision.renamed: BTreeMap<outer section, winner>`, filled in `drop_decision` on **both kept branches** (a party
  with an award summary, and a party kept because one of its copies reads as a name — the `Pas d'offre … Attribué à X`
  rows) by the new `extracted_winner(names)`: gate on `may_name_a_winner`; read only the copies in the void net
  (`mentions_void`); any such copy with a dropping class (`VoidLot`/`Placeholder`/`Blank`) → `None`; every void-net copy
  must yield `Some` and the **same** string, else `None` (copies that disagree keep the summary). Dropped parties and the
  guard-3 `Blank` keep never extract.
- `PartyVerdict::Renamed(String)` (the enum is no longer `Copy`; `Clone` stays). `FoldParties.renamed`;
  `verdict(section)` answers `Renamed(winner)` at the **outer** key only (where the fold mints); an inner half of a
  renamed party answers `Kept` as before.
- `NoticeState::mentions_and_dropped` returns a 4th element, `ExtractedWinners`; for each renamed party the mention's
  head and every labelled variant that is in the void net take the winner's text (other copies keep theirs, e.g. `EN
  Promocash SAS`). The mention then goes through the normal resolver (issue 234 name+country / 434 refresh), so an
  existing same-name org absorbs it.
- `pub struct ExtractedWinners { count, sample ≤ EXTRACTED_SAMPLE (20): (notice, section, name as published, winner) }`
  with `add`; `Report.extracted: Box<ExtractedWinners>` (boxed — see Deviations 2), summed through `build_plan`
  (`PlanChunk.extracted`, the 10-tuple), `project_plan_only` and `incremental_plan_rows`.
- Test destructurings of `mentions_and_dropped` take the 4th element (`_` / `..`).

### `crates/app/src/supervisor.rs`

- `VoidSplit.extracted: Vec<(org, name, winner, notice)>`; `split_void_hits` pushes a `Renamed` mention there (not into
  `exempted`, not into `pairs`); `void_split` reads verdicts with `.cloned()`.
- `run_refold_non_names`: the re-queued notice set is `pairs` ∪ `extracted` notices (so `notices`,
  `notices_by_profile` and the wet `expect` check count them; dry/wet semantics otherwise unchanged); the
  `void-names-refold` body adds `"extracted": [{org, name (clipped 300), winner, notice}]` (the placeholder report
  carries an empty list); the void job line adds `; N extracted winner mention(s) of M org(s) to rebind (issue 520)`
  between the exempted clause and the retire clause, and its tail reads `the fold retires the mentions and rebinds the
  extracted ones`.
- The fold's job row: `extracted_suffix(&report.extracted)` → `; extracted winners (issue 520): N: notice/section
  'name' → 'winner', …` (silent at zero) — the place U5 reads `Report.extracted` on prod.
- The fold's counts line moved from `run_project` into `#[inline(never)] fn fold_summary(report, sweep)` (same text,
  same order; the auto-sweep push still runs in `run_project` before it) — see Deviations 2.
- No new `Spec` arm.

### `docs/operations.md`

One runbook paragraph in the `refold-void-names` section: the `extracted` listing, what the paired fold does with it,
and U5's read (`jq -r '.extracted[] | [.org, .winner, .notice] | @tsv'` against App. B, stop on one not there).

## The 47 A1 rows: expected (App. B) vs extracted

`award_winner` on the dry report's name (cut at 300; 4782526 is the only cut A1 row and its winner ends at ` SISE`
before the cut). Asserted exactly in `award_winner_names_the_one_awardee`; every row is also asserted in the walk's net
(`mentions_void`) and a name the fold keeps (`None` or `AwardSummary`). **47/47 exact; 49 mentions on 28 notices** — §6
U5's expected count. Cue split: a 37, c 6, d 2, b 1, e 1.

| org | notice | mentions | cue | App. B winner | extracted | exact |
|---|---|---:|---|---|---|---|
| 1397079 | 12513246 | 1 | d `avec la société` | B BRAUN | B BRAUN | yes |
| 1397080 | 12513246 | 1 | d `avec la société` | Vascutek | Vascutek | yes |
| 1435451 | 12523990 | 1 | a `attribué … à` | Arrow Génériques | Arrow Génériques | yes |
| 1435452 | 12523990 | 1 | a `attribué … à` | Abbott Products | Abbott Products | yes |
| 1435453 | 12523990 | 1 | a `attribué … à` | Fresenius Kabi | Fresenius Kabi | yes |
| 1435455 | 12523990 | 1 | a `attribué … à` | Sanofi Aventis | Sanofi Aventis | yes |
| 1435456 | 12523990 | 1 | a `attribué … à` | Grunenthal/CSP | Grunenthal/CSP | yes |
| 1435457 | 12523990 | 1 | a `attribué … à` | Sandoz | Sandoz | yes |
| 1484019 | 12537848 | 1 | a `attribué … à` | Bionobis | Bionobis | yes |
| 1484119 | 12537862 | 1 | a `attribué … à` | CSL Berhing | CSL Berhing | yes |
| 1484121 | 12537862 | 1 | a `attribué … à` | LFB BIOMEDICAMENTS | LFB BIOMEDICAMENTS | yes |
| 1503868 | 12542887 | 1 | a `attribué … à` | Henry Schein | Henry Schein | yes |
| 1621278 | 12573042 | 1 | c label | Brake France Service | Brake France Service | yes |
| 1668604 | 12584969 | 1 | a `attribué … à` | Sarltacavl | Sarltacavl | yes |
| 1986366 | 16912057 | 1 | a `attribué … à` | Peters | Peters | yes |
| 2381362 | 17022900 | 1 | a `attribué … à` | Pierre Fabre Dermatologie | Pierre Fabre Dermatologie | yes |
| 2973447 | 17196649 | 1 | a `attribué … à` | Les Ateliers de Polangis | Les Ateliers de Polangis | yes |
| 3863573 | 17488388 | 1 | a `attribué … à` | Thermo Onix LTD | Thermo Onix LTD | yes |
| 3982542 | 17523806 | 1 | c label | zoll medical France | zoll medical France | yes |
| 4552687 | 17695208 | 1 | a `attribué … à` | Assurances securité | Assurances securité | yes |
| 4782526 | 17758913 | 1 | b `d'attribuer ce lot à` | Eiffage Construction | Eiffage Construction | yes |
| 6191476 | 18184254 | 1 | c label | Keolis | Keolis | yes |
| 7030205 | 18440709 | 1 | c label | ARDP | ARDP | yes |
| 7030206 | 18440709 | 1 | c label | JV Distribution | JV Distribution | yes |
| 7151483 | 18476377 | 2 | a `attribué … à` | Léo Lagrange | Léo Lagrange | yes |
| 7151494 | 18476377 | 1 | a `attribué … à` | FAIL 13 | FAIL 13 | yes |
| 7151502 | 18476377 | 1 | a `attribué … à` | C.A.J.L | C.A.J.L | yes |
| 7151511 | 18476377 | 1 | a `attribué … à` | Apis | Apis | yes |
| 7151529 | 18476377 | 1 | a `attribué … à` | Léo Lagrange | Léo Lagrange | yes |
| 7151543 | 18476377 | 1 | a `attribué … à` | A.E.C. Castellane | A.E.C. Castellane | yes |
| 7151562 | 18476378 | 2 | a `attribué … à` | san see | san see | yes |
| 7279113 | 18513007 | 1 | a `attribué … à` | Somatico | Somatico | yes |
| 7791659 | 18662469 | 1 | a `attribué … à` | Transgourmet | Transgourmet | yes |
| 7791721 | 18662475 | 1 | a `attribué … à` | Lodfirais | Lodfirais | yes |
| 7791767 | 18662480 | 1 | a `attribué … à` | Promocash | Promocash | yes |
| 7805728 | 18666274 | 1 | c label | SMACL | SMACL | yes |
| 9104280 | 19071221 | 1 | a `attribué … à` | OTIS | OTIS | yes |
| 9104287 | 19071221 | 1 | a `attribué … à` | Menuiserie du Chalonnais | Menuiserie du Chalonnais | yes |
| 31381712 | 4366568 | 1 | a `attribué … à` | LD Bio Diagnostics | LD Bio Diagnostics | yes |
| 31393407 | 4388372 | 1 | a `attribué … à` | Nectarys | Nectarys | yes |
| 31482009 | 12396006 | 1 | a `attribué … à` | Faure/Annequin | Faure/Annequin | yes |
| 31482010 | 12396006 | 1 | a `attribué … à` | Berthelet | Berthelet | yes |
| 31482011 | 12396006 | 1 | a `attribué … à` | Segui | Segui | yes |
| 31482013 | 12396006 | 1 | a `attribué … à` | Cars du Trièves | Cars du Trièves | yes |
| 31482014 | 12396006 | 1 | a `attribué … à` | Faure/Eyraud | Faure/Eyraud | yes |
| 31482016 | 12396006 | 1 | a `attribué … à` | Eyraud/Perraud | Eyraud/Perraud | yes |
| 31738609 | 1869611 | 1 | e `a été retenue` | Storz | Storz | yes |

**The other 329 of the 376** (173 D, 12 D residual, 10 A2, 99 B, 29 C, 6 F): `award_winner` returns `None` for every
one — asserted by `assert_class_and_no_winner` in `every_void_520_exemption_is_a_void_lot` and
`every_kept_520_exemption_keeps_its_class` (the four challenger tails included). The B rows that carry a cue refuse for
the decided reasons: two or more names (2381358/60/61, 6672986/89, 7512199, 23645704, 2450314's two `attributaire:`
labels — both inside the 300-character cut), a list after the name (31414415), no colon right after the label
(5425803 `titulaire 1:`).

**Every string literal of the repo** (temporary `#[ignore]` probe, deleted; 224 `.rs` files, 32,427 distinct
literals): 382 are in the void net and read `None`/`AwardSummary`; 83 extract. All 83 are 520 specimens or written
cases: the 47 A1 names (some twice), this build's own strip/terminator cases and fold-test literals, and U2's synthetic
D2/D3 near misses. Five of U2's synthetic award sentences extract a name with a procedure tail the decision lists no
terminator for — `attribué à Dupont Frères après négociation` → `Dupont Frères après négociation`, `attribué à Promocash
par marché négocié` → `Promocash par marché négocié` (Deviations 4). No D, B, C or F literal extracts.

## Tests

| test | where | what it pins |
|---|---|---|
| `award_winner_names_the_one_awardee` | partyname | the 47 App. B rows exact + net + kept; strip/terminator cases (`La Poste`, a void `;` segment, ` - 69003`, a postcode, `« … »`, `à l'issue de … à`, a repeated same winner, a closing `avec la société … .`, `Attributaire :`); §4.2 near misses: 1506401, 3526206/10, 3526991, 4032000 (relaunch/future), `il sera attribué`, `retenue pour négocier`; 31527955/61, 1974244, 11251840, 1633917, 7395857 (bidders, liquidation); 31383538, `Attributaire : voir annexe` (pointers); `n'a pas été attribué`, 4409817, 31674188, 1341883, 31375951 (tail), 31486027 (no party / count / too far); a non-closing `avec la société …, pour un montant`; 2381358, 6672986, 7512199, 23645704, 5425803, `titulaire: SAGA titulaire: Chapelec`, 2450314, 31414415, `…; lot 2 : Martin SA` (several names / a list); `may_name_a_winner` both ways |
| `every_void_520_exemption_is_a_void_lot`, `every_kept_520_exemption_keeps_its_class` | partyname | now also: no D/B/C/F/A2/residual name extracts (`assert_class_and_no_winner`); the two A1 blocks keep `assert_class` |
| `a_summary_party_binds_its_extracted_winner` | project (r208 harness) | 7791767's summary in FR+DE beside an EN `Promocash SAS` → head and both void-net variants `Promocash`, EN kept; 7151483 → `Léo Lagrange`; ordinary party untouched; nothing dropped/retired; `ExtractedWinners{count 2, sample}`; result `selec-w` with all three winners; verdicts `Renamed` / `Kept` |
| `copies_that_disagree_keep_the_summary` | project (r208 harness) | two winners, a summary naming none, a TED template beside a summary → head stays the summary, verdict `Kept` (reasons pinned); agreeing copies → `Renamed("Dupont SARL")`; extracted count 1 |
| `a_refold_rebinds_the_mention_to_the_winner` | tests/project.rs (434 harness) | the stock as prod holds it (summary-named mention + org, written back by SQL as the 525 walk test does) and a standing `Indaeltrac` (RO) org from another notice; the walk's wet re-queue (`unmark_projected_by_ids` + `stamp_stale_for_notices`) and an incremental fold → refreshed 1, **rebound 1**, retired 0, `extracted` sample; the mention binds the **existing** same-name org (one `Indaeltrac`), the summary org has 0 mentions (orphaned), result winner and party rows moved, decision `selec-w` |
| `a_second_refold_of_a_renamed_party_rebinds_nothing` | tests/project.rs | re-queue + fold again → refreshed 0, rebound 0, retired 0, extracted 1, mention still on the winner org (idempotent) |
| `the_void_walk_requeues_an_extracted_summary` | supervisor (end to end) | an r208 award with 7791767's summary, stock written back; dry: job line `0 void-lot org(s)`, `0 exempted`, `1 extracted winner mention(s) of 1 org(s) to rebind (issue 520)`, `0 mention(s) to retire on 1 notice(s) (ted-export-r208 1)`, `would re-queue 1`; body `orgs 0 / mentions 0 / notices 1`, `exempted []`, `extracted [{org, name, winner Promocash, notice}]`; wet `expect 1` → re-queued 1, stamped 1; paired fold rebound 1, retired 0, extracted 1, mention on a new `Promocash` org, summary org orphaned; dry again `0 extracted …`, `would re-queue 0` |
| `the_extracted_suffix_names_the_count_and_the_sample` | supervisor | the fold row's suffix: silent at zero, count + sample, clipped at 60 |
| `run_spec_futures_stay_inside_their_size_budgets`, `an_execute_without_an_expected_count_is_refused` | supervisor | green (see Deviations 2 for the run_project poll depth) |

**Tests first.** The tests were written before the implementation compiled; the red evidence is the mutation table
(M1 is `award_winner` returning `None`, the pre-U4 state of every new assertion).

## EXITs (gate flags + gate package set, never piped, `> file 2>&1; echo EXIT=$?`)

Command prefix: `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p model -p store
-p ingest -p tender-db --features tender-db/server`.

| run | tail | EXIT | result |
|---|---|---:|---|
| partyname (award_winner first draft) | `--lib -- partyname::` | 0 | ingest 50 passed, 2 ignored |
| focused, first cut | `--lib -- partyname:: a_summary_party… copies_that… the_void_walk the_extracted_suffix run_spec_futures… an_execute_without…` | 101 | ingest 52 ok; tender_db **SIGABRT**: `issue 467: run_project's poll frame is over its 483328-byte stack budget` (Deviations 2) |
| focused, after the budget fix | same + `the_dropped_suffix refold_void_names` | 0 | ingest 52 passed; tender_db 8 passed (incl. the budget test) |
| all lib suites | `--lib` | 0 | ingest 524 passed / 4 ignored, model 3, store 182, tender_db 209; 0 failed |
| integration neighbours | `--test project --test text --test r208 --test r209 --test project_golden --test project_incremental` | 0 | project 104, project_golden 11, project_incremental 51, r208 15, r209 19, text 13; 0 failed |
| **final** all lib suites (final tree, after the doc edits) | `--lib` | 0 | ingest 524 / 4 ignored, model 3, store 182, tender_db 209; 0 failed |
| **final** integration neighbours | as above | 0 | project 104, project_golden 11, project_incremental 51, r208 15, r209 19, text 13; 0 failed |

No `error`/`FAILED`/`panicked` line in the green logs; the only warnings are two pre-existing ones in
`tests/project.rs:552` and `tests/project_incremental.rs:1217`. Disk after: target 14G, 6.3G free on `/`.

## Mutation check (one part off at a time; restore checked with `cmp`)

Scripts in this session's scratchpad (`u4/mut/mutate.py`, `u4/mut/run.sh`): each mutant applied by exact-string
replace (asserted unique), the targeted tests run (`--lib --test project -- partyname:: a_summary_party_binds…
copies_that_disagree… the_void_walk_requeues_an_extracted_summary a_refold_rebinds… a_second_refold…`; M4–M6 with
`--no-fail-fast`), the file restored from its snapshot and compared. Every restore `identical`; the final three source
files `cmp`-identical to the snapshots; no mutant had a compile error (the only `error:` lines are cargo's `test failed`).

| mutant | part | EXIT | failing tests |
|---|---|---:|---|
| M1 `award_winner` always `None` | partyname | 101 | award_winner_names_the_one_awardee, a_summary_party_binds_its_extracted_winner, copies_that_disagree_keep_the_summary (fail-fast stopped before the app/integration binaries) |
| M2 no list-separator rule | partyname | 101 | award_winner_names_the_one_awardee, every_kept_520_exemption_keeps_its_class (31414415) |
| M3 no distinct-names rule | partyname | 101 | award_winner_names_the_one_awardee, every_kept_520_exemption_keeps_its_class |
| M4 disagreeing copies still rename | project | 101 | copies_that_disagree_keep_the_summary |
| M5 mention not renamed | project | 101 | a_summary_party_binds_its_extracted_winner, copies_that_disagree_keep_the_summary, a_refold_rebinds_the_mention_to_the_winner, a_second_refold_of_a_renamed_party_rebinds_nothing, the_void_walk_requeues_an_extracted_summary |
| M6 walk exempts `Renamed` | supervisor | 101 | the_void_walk_requeues_an_extracted_summary |

## Deviations and interpretations

1. **"Kept-as-summary" read as the decision's set.** §4.2: the fold consults `award_winner` for "an `AwardSummary` name,
   or a plain name after a void prefix (`Pas d'offre … Attribué à X`)". The seven 7151xxx rows (9 of the 49 mentions)
   read `None`, not `AwardSummary`, and their parties are kept on `0/1 names void`, not as summaries; restricting to
   `KeptParty.summary` would have lost them and U5's 47/49. So `extracted_winner` runs on both kept branches, reads only
   void-net copies, and refuses any void-net copy with a dropping class. A dropped party never extracts; an ordinary
   name (outside the net) is never read.
2. **run_project's poll budget (issue 467).** The first cut aborted `run_spec_futures_stay_inside_their_size_budgets`
   (472 KiB thread). Measured with a temporary 8 MiB stack paint (probe deleted): HEAD 460 KiB, first cut 462 KiB
   (future 3,688 → 3,744 B). The growth was `Report`'s inline 32 bytes in every `Report` slot of the fold's O0 frames,
   not the new suffix (replacing it with `String::new()` still read 462). Fix, both in the repo's own idiom: box the field
   (`Report.extracted: Box<ExtractedWinners>`, as 469 U5 boxed `MentionResolver.altid`) → 461 KiB; move the counts
   line out of `run_project`'s poll frame into `#[inline(never)] fn fold_summary` → **460 KiB, future 3,536 B** (below
   HEAD's 3,688). The budget number is unchanged. The headroom is HEAD's (about 1 KiB at 472 KiB): the next growth on
   this path will trip it; the lever is still splitting `project_incremental_chunked_observed`'s frame.
3. **The fold row's `extracted winners (issue 520)` suffix** is an addition: without it `Report.extracted` reaches no
   durable place on prod. Silent at zero, like `dropped parties`.
4. **Terminators exactly as decided.** No ` après ` / ` par ` terminator: §4.2 lists none, and no prod row (the 376)
   needs one. Five synthetic U2 literals extract a tail (`Dupont Frères après négociation`). U5's read against App. B
   stops on any such extraction; recorded here for review, not built.
5. **Two small guards beyond §4.2's list, both only ever refusing or skipping:** `NO_PARTY_AFTER` (`à l'issue de`,
   `à compter`, `à partir`, `à la suite`, `à hauteur`: the cue reads on to the next `à`, so `attribué à l'issue de la
   négociation à la société X` names `X`, not `issue de la négociation à la société X`) and `WINNER_POINTERS` (a name
   opening `voir`/`cf`/`see`/… is a pointer). No A1 row touches either.
6. **Renamed at the outer key only.** A recorded mention at a nested inner key of a renamed party reads `Kept`
   (exempted, as today): the fold mints at the outer key and never rewrites the inner row, so re-queuing it would
   re-list it on every walk. None of the 47 is known to stand at an inner key; U5's dry `extracted` count shows it if one
   does (it would read 48 or fewer mentions and the org would show in `exempted`).
7. **Same winner = the same string**, exactly; distinct names inside one copy compare on the fold (`Dupont` /
   `DUPONT` is one name).
8. **`notices` in the void report now counts the extracted mentions' notices too** (what `expect` holds); `mentions`
   stays the mentions to retire.

## Risks for U5

- **Multi-copy parties.** The fold renames a party only when every void-net copy names the same winner. An A1 party with
  a translated void-net copy (an EN `Lot declared unsuccessful …`) stays a summary and shows in `exempted`, not
  `extracted`. The 47 are FR/text single-copy as far as the dry report shows (it reads head names only), so U5's dry
  count is the measurement: expect 47 orgs / 49 mentions / 28 notices.
- **Identity.** The rebind goes through the resolver: `Promocash`, `Berthelet`, `Faure/Annequin`, `Eyraud/Perraud`,
  `Assurances securité` bind the standing same-name org of the same country (intended, §4.2); spellings (`Sarltacavl`,
  `Léo Lagrange`, `CSL Berhing`, `san see`) mint their own.
- **The budget headroom** (Deviations 2) is about 1 KiB on `run_project`'s path, as at HEAD.

## Commit file list

- `crates/ingest/src/partyname.rs` — `award_winner`, `may_name_a_winner`, `award_verb` (lifted from `award_clause`),
  helpers, consts, module doc, tests (+507/−44 against HEAD).
- `crates/ingest/src/project.rs` — `ExtractedWinners`, `Report.extracted`, `DropDecision.renamed`, `extracted_winner`,
  `PartyVerdict::Renamed`, `FoldParties.renamed`, the rename in `mentions_and_dropped`, plumbing, two fold tests (+255/−20).
- `crates/app/src/supervisor.rs` — `VoidSplit.extracted`, `split_void_hits`, the report and job line, `extracted_suffix`,
  `fold_summary`, two tests (+260/−60).
- `crates/ingest/tests/project.rs` — the stock helper, the walk's re-queue helper, two refold tests (+107).
- `docs/operations.md` — one runbook paragraph (+7).
- `.scratch/tender-db/520-award-summaries/u4-build-notes.md` — this file.

Before staging, run `git diff` on each source file: the tree held no other agent's hunks during this build
(`git status` showed only these five files). Next: `ops/check.sh`, then the deploy, then U5's dry run.

## Review fixes (2026-10-13, after three adversarial reviewers)

Still in the working tree, **not committed**, `ops/check.sh` not run, nothing on prod touched. I checked each of
the 22 findings against the code. For the 16 partyname findings I used a temporary `#[ignore]` probe (deleted). On
the build as it stood, it reproduced every output the reviewers quoted: `Some("n")`, `Some("article 35")`,
`Some("Générale")`, `Some("Ets")`, `Some("Dupont S.A. Montant HT : 48 225")`, and so on. For findings 10–12, I read
the code and pinned each with a test that fails on the build (mutations R1–R3).

**None of the 22 was rejected.** Three are fixed in a different way than the reviewer suggested; all three are
listed in the table below:
- 9: `à <town>` makes the candidate refused, not truncated;
- 10: an identified org is exempted, not carried in `extracted`;
- 13: only a descriptor that stands alone is refused.

### Per finding

| # | finding (severity) | verdict | fix | test |
|---|---|---|---|---|
| 1 | cue e takes the elided `n'` (must) | confirmed | cue e is skipped when `n'`/`ne` stands before `a été retenue`, or a negation/`nulle` stands within 2 words before `société`. Its name now also ends at `min(until, winner_end)` (finding 16). `plausible_winner` needs ≥ 2 alphanumerics. The undone tail check (finding 4) catches `puis s'est désistée` | `award_winner_reads_the_clause_around_the_name` |
| 2 | cue d takes any closing `avec la société` (must) | confirmed | cue d needs its last `PAST_RELAUNCH` participle **in the same sentence**, with only `RELAUNCH_PROCEDURE` words between it and `avec`. Allowed between: procedure vocabulary, numbers, number marks, roman numerals. Any other word blocks the cue: `mais déclaré sans suite … échec`, `négociations rompues`, `aucun accord`, `sans succès` | same |
| 3 | cues a/b miss a negation, refusal or intent further back (must) | confirmed | cue a: a word from `AWARD_DOUBTS` anywhere in the verb's clause before it blocks the cue. `AWARD_DOUBTS` holds `soit`, conditionals, `envisagé`, `prévu`, `proposé`, `pressenti`, `refusé`, `impossible`, `jamais`, `initialement`, `précédemment`, … Also blocking: a negation read past `encore`/`toujours` (`pas encore attribué`). Cue b: needs `a décidé d'attribuer` (`AWARD_DECISIONS`), not negated. `award_verb` is unchanged, so U2's classifier is unchanged | same |
| 4 | award undone after the name (should) | confirmed | cue a: `initialement`/`précédemment` before the verb (finding 3). Any cue: the cue is skipped when the rest of its sentence after the name holds an `AWARD_UNDONE` stem (`désist`, `annul`, `résili`, `liquidation`, `défaill`, `retir`, …) or `sans suite`. Cue c: skipped after `ancien`/`précédent`/`ex`/… (`LABEL_QUALIFIERS`) | same |
| 5 | one-cue B/C lists split by `,`/` - `/`. ` (should) | confirmed | new `lot_entry_names_another`. It reads the lot entries as D4c's `lot_label` does. An entry that belongs to no cue makes the answer `None` when it names someone: it is not void at its head, and does not open on a relaunch. An entry belongs to a cue when the cue opens in it, or when it opens in the cue's clause (`pour le lot 2`). A cut entry (`Lots 2)`) also makes it `None`. Glued pairs: a candidate holding a standalone `à` or `lot <digits>` is refused | same |
| 6 | `Société`/`Entreprise`/`STE` of the legal name stripped (should) | confirmed | a descriptor stays (without its article) when a `NAME_ADJECTIVES` word follows it (`Générale`, `Française`, `Nouvelle`, …), or a lower-case connector (`des`/`de`/`du`/`d'`). Cue e's name opens at the descriptor in that case. `De Dietrich` keeps its name | `award_winner_ends_and_strips_the_name_as_published` |
| 7 | generic phrases become shared junk orgs (should) | confirmed | the name's first word is checked against three lists. `COUNT_NOUNS` gained number words, articles and determiners. `WINNER_POINTERS` gained `idem`, `ibid`, `ci`, `identique`, `même`, `suivant(e)`, `se`, `consulter`, `reporter`, … The new `WINNER_ROLES` holds `titulaire`, `attributaire`, `soumissionnaire`, `candidat`, `unique`, `mieux`, `ayant`, `offre`, `montant`, `cao`, `séance`, `pouvoir`, `monsieur`, `ce`, `en`, `non`, `pas`, `personne`, `rien`, `inconnu`, `confidentiel`, … | `…reads_the_clause…` (40+ cases) |
| 8 | cue a takes an adverbial `à` (should) | confirmed | `NO_PARTY_BEFORE` (`conformément`, `suite`, `grâce`, `jusqu'`, `recours`, …) and a wider `NO_PARTY_AFTER` (`titre`, `nouveau`, `unanimité`, `majorité`, `occasion`, `défaut`, `article`, `fin`, `date`, …). The cue then reads on to the next `à` within its reach | `…ends_and_strips…` |
| 9 | `winner_end` stops (nit) | confirmed | the name now also ends at `\n`/`\r`, at `:`, at `pour` + a figure or lot, at `en date`, `qui`, `dont`, `a été`/`ont été`, and after a dotted legal form when the next field opens. It also ends at `après`/`par` + a procedure word, which closes the build's deviation 4. `… à Lyon` refuses the candidate (standalone `à`) rather than truncate it | both |
| 10 | identified Renamed party re-queued forever (should) | confirmed by reading (resolver binds `org_of[identifier]` first; nothing updates `organizations.name`) | `split_void_hits`: a `Renamed` mention whose org carries an identifier (`NamedOrg.2`) is **exempted**. Its reason reads `party kept under its award summary's winner '<w>', but its identifier binds the mention (issue 520)`, so U5's read sees it. The fold still renames the mention, so new data mints the winner with its identifier. The runbook line gives the `jq` for these | `an_identified_summary_party_is_exempted_with_its_winner` (end to end, two dry walks identical), `the_void_split_exempts_an_identified_rename` |
| 11 | copies outside the net never compared (nit) | confirmed | `extracted_winner`: every copy outside the net must name the same company: its folded words contain the winner's, or the reverse. So `Promocash SAS` binds and `Transgourmet Opérations` keeps the summary. This is §4.2's "the party's copies disagree" | `copies_that_disagree_keep_the_summary` (ORG-5 kept, ORG-6 `Dupont SARL Lyon` renamed) |
| 12 | job line counts (nit) | confirmed | `both` counts orgs with an extracted mention as re-queued. The clause now reads `N extracted winner mention(s) of M org(s) (issue 520); P mention(s) to retire and N to rebind on F notice(s)` | `an_org_kept_and_extracted_is_counted_on_both_sides` (`1 exempted, 1 of them also re-queued`), `the_void_walk_requeues_an_extracted_summary` |
| 13 | bare/qualified descriptor becomes the winner (should) | confirmed | a one-word descriptor or company abbreviation is refused. A qualifier left after the strip is refused by its first word (`mieux`, `suivante`, `même`, `un`, `attributaire`). A capitalised bare `Entreprise Dupont` still reads as before | `…reads_the_clause…` |
| 14 | `Ets. Dupont` → `Ets` (should) | confirmed | a `.` after a `NAME_ABBREVIATIONS` word (`ets`, `ste`/`sté`, `cie`, `st`, `assoc`, `etab`, `soc`) is no sentence end; that word alone is refused | `…ends_and_strips…` |
| 15 | dotted forms/initials run on, final dot cut (should) | confirmed | a `.` after a single letter ends the name when whitespace and a capitalised word follow, and either that word is a `FIELD_OPENERS` word (`Montant`, `Date`, `Le`, …) or the dotted letters spell a `COMPANY_FORMS_DOTTED` form. The dot is kept (`trim_winner`). `A.E.C. Castellane` and `C.A.J.L` stay whole | same |
| 16 | cue e skips every terminator (should) | confirmed | `end = until.min(winner_end(..))` | same |
| 17 | `LA SOCIÉTÉ` kept (should) | confirmed | `LA ` added to the articles (`La Poste` and `LA POSTE` keep theirs) | same |
| 18 | terminators need an ASCII space (should) | confirmed | `winner_end` matches its spaced terminators after any whitespace character, and ends at `\n`/`\r` | same |
| 19 | unbalanced quotes (nit) | confirmed | quotes inside the name are dropped as a pair and the gap collapsed (`Les Jardins SARL`, `Les Jardins de Cocagne`) | same |
| 20 | `à titre provisoire à` (nit) | confirmed | same fix as finding 8 | same |
| 21 | `l´entreprise` (nit) | confirmed | `´` and `ʼ` article forms | same |
| 22 | quadratic on long names (nit) | confirmed | Each word's clause, its sentence, and the next future or doubt word are read once (`winner_cues`). `award_verb_in` takes the precomputed clause start, and `award_verb`/`award_clause` keep their reading byte for byte. `winner_end`, `undone_after` and `award_circumstance` read at most `WINNER_SCAN` = 240 / 48 bytes. `listed_after` is read once. `lot_entry_names_another` is read once. A candidate over 120 characters is refused. Timings: 32 KB 4.2 s → 95 ms; 128 KB 66 s (77 s measured here) → 0.36 s; 1,000 non-closing `avec` cues at 85 KB → 52 ms | `award_winner_reads_a_long_text_once` (3 × 4,000 units, < 5 s each) |

### Accepted costs (each refuses, never extracts wrong)

- **An undo word anywhere after the name in its sentence refuses the cue.** Example: `attribué à la société Dupont suite
  à la défaillance de la société Martin` gives `None`.
- **A name holding a standalone `à` is refused.** Examples: `La Boîte à Outils`, `Dupont à Lyon`.
- **A first word on a head list is refused.** Examples: `Six Construct`, `CE Cedille`, `Unique Services`.
- **A word after `lot`/`zone`/`secteur` that holds a digit is refused.** Example: `Zone 3 Services`.
- **The closed vocabularies refuse wordings they do not list:**
  - cue d's procedure words (`RELAUNCH_PROCEDURE`);
  - cue b's decision verbs (`AWARD_DECISIONS`);
  - the name adjectives (`NAME_ADJECTIVES`).

  An adjective missing from that last list strips the descriptor, as the build did.

In each of these cases the party keeps its summary, which was the state before U4.

### Re-check of the 47 App. B rows and of D/B/C/F

- **All 376 dry-report names** (`../525-void-templates/void-names-refold-after-dry.json` `exempted[]`; the 203 of
  `void-names-refold-2360-dry.json` are a byte-identical subset) were run through `award_winner` with the
  final extractor and compared to App. A classes and App. B winners:
  - **376/376 match.**
  - **47 extract, each exactly its App. B string.**
  - **329 read `None`:** 170 D (D1–D7), 3 two-rule D, 12 D residual, 99 B, 29 C, 6 F, 10 A2.
- **In the unit tests:** `award_winner_names_the_one_awardee` (the 47, exact) and `every_void_520_exemption_is_a_void_lot` /
  `every_kept_520_exemption_keeps_its_class` (`assert_class_and_no_winner`) are unchanged and green.
- **Repo literal corpus:** 303 distinct cue-bearing string literals under `crates/`, run on the build and on the
  final code. The only differences are the five `après négociation` / `par marché négocié` tails, now trimmed
  (`Dupont Frères`, `Promocash`). No D, B, C or F literal extracts.
- **U5's expectations:** still 47 orgs / 49 mentions / 28 notices, with two exceptions, both new and both readable
  in `exempted` with their reasons:
  - an A1 org that carries an identifier now shows under `exempted` with the `issue 520` reason;
  - a party with a translated copy outside the net that names another company stays a summary.

### Mutations (one at a time, restored and `cmp`-checked; `--lib --no-fail-fast`)

| mutant | EXIT | failing |
|---|---:|---|
| R1 identified guard off (`if false && org.2.is_some()`) | 101 | an_identified_summary_party_is_exempted_with_its_winner, the_void_split_exempts_an_identified_rename |
| R2 `both` without the extracted orgs | 101 | an_org_kept_and_extracted_is_counted_on_both_sides |
| R3 copies outside the net not compared | 101 | copies_that_disagree_keep_the_summary, an_org_kept_and_extracted_is_counted_on_both_sides |
| R4 the three new partyname tests against the build's extractor | 101 | award_winner_reads_the_clause_around_the_name (`Some("n")`), award_winner_ends_and_strips_the_name_as_published (`Some("article 35")`), award_winner_reads_a_long_text_once (76.8 s); award_winner_names_the_one_awardee passed |

### EXITs (gate flags + gate package set, never piped; `> file 2>&1; echo EXIT=$?`)

Prefix as above (`CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p model -p store
-p ingest -p tender-db --features tender-db/server`).

| run | tail | EXIT | result |
|---|---|---:|---|
| probe on the build (temporary) | `--lib -- zz_u4rf_probe --ignored` | 0 | 152 reviewer cases: every quoted output reproduced |
| partyname after the rework | `--lib -- partyname::` | 0 | 50 passed |
| partyname with the new tests | `--lib -- partyname::` | 0 | 53 passed |
| focused | `--lib --test project -- partyname:: a_summary_party_binds… copies_that_disagree… the_void_walk_requeues… an_identified_summary_party an_org_kept_and_extracted the_void_split the_extracted_suffix run_spec_futures_stay_inside_their_size_budgets an_execute_without_an_expected_count_is_refused refold_void_names a_refold_rebinds a_second_refold the_dropped_suffix` | 0 | ingest 55, project 2, tender_db 10 (budget test included) |
| all lib suites | `--lib` | 0 | ingest 527 / 4 ignored, model 3, store 182, tender_db 212; 0 failed |
| integration neighbours | `--test project --test text --test r208 --test r209 --test project_golden --test project_incremental` | 0 | project 104, project_golden 11, project_incremental 51, r208 15, r209 19, text 13; 0 failed |
| **final** focused (final tree) | as above | 0 | ingest 55, project 2, tender_db 10 |
| **final** all lib suites | `--lib` | 0 | ingest 527 / 4 ignored, model 3, store 182, tender_db 212; 0 failed |
| **final** integration neighbours | as above | 0 | project 104, project_golden 11, project_incremental 51, r208 15, r209 19, text 13; 0 failed |

- **Warnings:** only the two pre-existing ones (`tests/project.rs:552`, `tests/project_incremental.rs:1217`).
- **Stack budget:** `run_spec_futures_stay_inside_their_size_budgets` is green. This pass adds no `Spec` arm, and
  `run_refold_non_names` changed only in its job-line arithmetic.
- **Disk after:** target 14G, 5.4G free on `/` (6.3G before this pass).

### Files this pass touched

- `crates/ingest/src/partyname.rs`:
  - new extractor pieces: the review's constants, `winner_cues` with precomputed clauses, `undone_after`,
    `winner_end` rework, `award_circumstance`, `floor_boundary`, `trim_winner`, `strip_winner` (now returns
    `String`), `descriptor_names`, `plausible_winner`, `lot_entry_names_another`;
  - `award_verb` → `award_verb_in` with the clause start (same reading);
  - module doc;
  - three tests.
- `crates/ingest/src/project.rs`: `extracted_winner` (copies outside the net), doc, the ORG-5/ORG-6 cases.
- `crates/app/src/supervisor.rs`:
  - `split_void_hits`: the identified exemption;
  - `both` and the job-line wording;
  - docs;
  - the walk test refactored onto `record_a1_award` / `as_prod_holds_it` / `void_walk`, plus three new tests.
- `docs/operations.md`: the runbook paragraph names the identified exemption and its `jq`.

Probe, mutation and log files are in this session's scratchpad (`u4rf/`). Before staging, run `git diff` on each
file as before. Next steps: `ops/check.sh`, then the deploy, then U5's dry run, read against App. B and against
`exempted[]` reasons containing `issue 520`.
