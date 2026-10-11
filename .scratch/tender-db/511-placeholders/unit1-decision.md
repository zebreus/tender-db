## Unit 1: decision (2026-10-11)

Suggested status line: `Status: ready-for-agent — unit 1 DECIDED 2026-10-11 (../511-placeholders/unit1-decision.md). NEXT: unit 2 build (U2a predicate, U2b fold + NON_NAME_FOLDS, U2c two plan readers, U2d job; one gate, one deploy), then unit 3 drain (§8).`

**In one paragraph.** The fold drops every party whose every published name is a non-name, in every role and every era, the buyer slot included. It does this on 510's machinery with the class widened: one decider (`void_party_sections` becomes `dropped_party_sections`), the same skip in `NoticeState::mentions`, the same role and result filter in `NoticeState::read`, the same class-blind `Db::retire_mentions` and the same 443 sweep. `partyname::NotAName` gets four classes:
- `VoidLot` (510, unchanged);
- `Placeholder`, narrowed to "a winner exists; its name is elsewhere, withheld or plural";
- `Blank` (new): "this slot holds nothing" (`n/a`, `none`, `néant`, `nie dotyczy`, punctuation-only);
- `AwardSummary` (new): the names 510's `names_an_award` keeps, which the fold keeps.

A legacy or sdk-0.1 result left with no real winner reads the class. `VoidLot` gives `clos-nw`. `Placeholder` gives `selec-w` with no winner. `Blank` carries no marker, so the existing evidence rule decides, as if the slot were empty. eForms and FTS keep the publisher's BT-142 and the Bid. Three guards protect real names: whole-value matching for every new single word; a language scope plus a plain-case test for the three short words with measured real-name collisions; and an identifier guard for `Blank` only. `NON_NAME_FOLDS` is deleted in favour of the predicate. Award summaries stay as they are until per-lot parsing exists (a follow-up issue). The drain is `refold-void-names` generalised by class (`refold-placeholder-names`): a buyer-slot canary, the wet refold, then the 443 sweep.

**Basis.** `census.md` and `census.json` in this directory: 36 bounded calls on 2026-10-11, exact `name_norm` keys. Wherever a figure below is called "admitted", it is that census re-classified under §4's lists: the census's `fold` per key, the lists in §4, and the guards applied per org.

**Line numbers** refer to HEAD `493457c`. A concurrent, uncommitted 513 U2 edit already shifts `canonical.rs` after its line 617 (by +7 at `buyer_tokens_disjoint`, and by +81 from about line 7,400 on). 513's pending supervisor job will shift `supervisor.rs` too. Find each site by the symbol named next to it.

### 1. Classes

| class | means | a legacy / sdk-0.1 result left with no real winner | specimens | admitted: orgs / mentions / party rows |
|---|---|---|---|---|
| `VoidLot` (510) | the lot was not awarded | `clos-nw` (a marker; outranks value and date) | `Infructueux`, `Desierto` | drained by 510 |
| `Placeholder` (narrowed) | a winner exists; its name is elsewhere, withheld or plural | **`selec-w`, no winner bound** (a marker) | `Withheld Section94 Supplier`, `See Section VI.3 …`, `ver perfil del contratante`, `Various suppliers`, `siehe oben` | 988 / 4,420 / 5,466 |
| `Blank` (new) | the slot holds nothing | **no marker.** The existing rule decides (value → `selec-w`; no date → `clos-nw`; date only → NULL; project.rs:6160-6172) | `N/A`, `nie dotyczy`, `néant`, `Ingen` (da/sv/no), `.`, `0` | 4,100 / 31,814 / 40,442 |
| `AwardSummary` (split out of `Placeholder`) | a non-name phrase inside a name that names an award or a company | kept as today | `Lot 1) Sarl Bremond. Lot 2) … Lot 3) Déclaré infructueux` | ~500 (job 2182 `exempted`) |

- **Why `Blank` is not `VoidLot`.** The census's observation 1 points the other way: `ingen` has 600 winner rows, `néant` 458, `geen` 110, `aucun` 100. But these words fill review and mediation slots just as often (`ingen` 347 rows, `néant` 811, `nie dotyczy` 7,667), so they fail 510's `VOID_WHOLE` admission ("no other-role use"). The census also read no value/date split for them. Reading the slot as empty asserts nothing the publisher did not:
  - a block with no value and no date still reads `clos-nw` under the existing rule (on 510's four hubs, 6,266 of 8,128 rows had that shape);
  - a block with a value reads `selec-w` with no winner.

  Promoting a single word to `VoidLot` later needs 510's value/date table for that word.
- **Why `Placeholder` carries a marker.** The text says a winner exists. Left to the evidence rule, a dateless legacy block would read `clos-nw`, which contradicts the publisher.
- **Mixed parties.** When one party's names fall in different classes, the party takes `Placeholder` over `VoidLot` over `Blank`: a withheld winner beside a void phrase means a winner exists. At the result, a `Placeholder` ranks with real winners, so `selec-w` beats the void marker, as 510's real co-winner does.

### 2. Per role: drop the party in every role

Admitted party rows, `Blank / Placeholder`:

| role | rows | decision | what else changes |
|---|---:|---|---|
| `winner` (legacy direct winner; sdk-0.1 `WinningParty`) | 2,572 / 1,188 | drop | result per §1; no `tender_version_result_winners` row |
| `AWARD_AND_CONTRACT_VALUE` (internal-ojs, unfolded) | 336 / 81 | drop | caught by the role filter, as in 510 |
| `Tenderer` (eForms/FTS `TenderingParty` member), `Tenderer-SubCont` | 551 / 631 | drop the bid party | the Bid and BT-142 stay (510's eForms rule). Section94's 345 rows keep the publisher's decision and bind no winner |
| `review-body`, `mediation-body`, `appeal-information`, `Lot-ReviewOrg`, `Lot-Mediator`, `Lot-ReviewInfo`, `RESPONSIBLE_FOR_MEDIATION_PROCEDURES`, `Part-*` | 7,062/864, 24,041/1,423, 893/608, 1,946/165, 594/148, 348/14, 366/60, 29/0 | drop | nothing: no result, token or key reads them |
| `purchasing-body`, `Procedure-CPB` | 268/153, 3/0 | drop | nothing: neither is in `BUYER_ROLES` (project.rs:7385) or `GUARD_SIGNATORY_ROLES` (:7472) |
| `specifications-provider`, `further-information`, `tender-receipt`, `Lot-TenderReceipt`, `Lot-DocProvider`, … | 569/47, 272/28, 266/53, … | drop | nothing |
| `buyer`, `Procedure-Buyer` | 283 / 3 | **drop** (§3) | buyer tokens, buyer key, 482's refused groups |

- **Keep** is decided by the party's shape, not by its role. Two shapes are kept: the `AwardSummary` class (§5) and a `Blank` party that carries a gated identifier (§4, guard 3).
- **Map** applies to no role. A placeholder names no organization to map to. The identifier is the only evidence that could map it, and guard 3 already keeps the party binding by it.
- **Why one rule for every role.** The class belongs to the party section, which `void_party_sections` already decides once for all roles (project.rs:4379-4409). A per-role exception would put "is this a name?" in two places, which 510 §1 rejected. Keeping these parties has a measured cost: hubs that join unrelated Tenders. 1215973 `nie dotyczy` PL sits on 6,960 party rows; 1213676 `No procede` ES has 3,192 mentions; 1257377 `N/A` GB has 2,129 mentions across review, mediation, winner and buyer slots.

### 3. Buyer slot: drop, no exception

286 rows on 14 orgs (283 `Blank` + 3 `Placeholder`):
- 241 on 11063 `.` AT, an eForms `Procedure-Buyer` with no identifier;
- 21 on 36188 `Keine Angabe.`, 6 on 1257377 `N/A` GB, 4 on 1215973 `nie dotyczy`, 3 on 1164345 `-`, 2 on `To be confirmed` GB, 2 on `siehe oben` DE;
- 1 each on seven more.

**What the placeholder does to grouping today.**
- **Tokens.**
  - `.` yields no name token, because `buyer_guard_tokens` skips an empty fold (project.rs:7633). Its only token is therefore `o:11063` (`add_org_tokens`, :8093-8103), and all 241 notices share it. The 481 guard (`store::buyer_tokens_disjoint`, canonical.rs:2060) reads them as one buyer among themselves and as a different buyer from every other notice.
  - `N/A` GB yields `n2:GB:n a`, which every GB notice with that buyer shares.
- **Key.** `buyer_key_of` (:7557) gives `N/A` GB the key `n2:GB:n a`. Under a refused procedure key, 482's `refused:{key}:{buyer}` (:3233) then groups every such notice as if they had one buyer.

**After the drop**, the notice's buyer side is its other buyers, or none:
- Empty tokens mean "unknown", which the guard never refuses (canonical.rs:2061-2063; 481 2b, project.rs:7573).
- A `None` buyer key means "names no buyer" to 369's gate.

Both are the designed states for a notice that names no buyer, so the drop replaces a false identity with the designed unknown. The grouping change is confined to the ~290 notices that had a placeholder buyer. A cited link refused today as buyer-disjoint becomes admitted, and a refused group keyed on a placeholder buyer dissolves. The canary in §8 measures this before the main run.

**Rejected alternatives.**
- **Keep:** leaves the false shared token and key, and serves `.` as the buyer of 241 Tenders.
- **Map:** has no target. None of the 14 orgs carries an identifier. Promoting another party is 483's `real-buyer-elsewhere` decision, which still runs on the remaining parties after the drop (project.rs:4766).

### 4. The predicate and the lexicon

```text
not_a_name_in(name, lang) -> Option<NotAName>   // the fold; lang = project::original_lang(parsed) (project.rs:9039)
  if name.trim() is non-empty and has no alphabetic char -> Blank   // NEW, before the fold (an empty fold returns None, partyname.rs:282)
  f = fold(name); empty -> None
  VOID_STEMS hit        -> AwardSummary if names_an_award else VoidLot     // 510; only the class name changes (:290)
  PLACEHOLDER_STEMS hit -> AwardSummary if summary_shape else Placeholder
  BLANK_STEMS hit       -> AwardSummary if summary_shape else Blank
  w = fold(strip_lot_qualifier(name)) or f
  VOID_WHOLE / reason_head / void_prefix                     -> 510, unchanged (its award case stays None: `DESIERTO, S.L.`)
  w in PLACEHOLDER_WHOLE                                     -> Placeholder
  w in BLANK_WHOLE                                           -> Blank
  w in SCOPED_WHOLE, lang in its languages, plain_case(name) -> Blank
  None
not_a_name(name)          = not_a_name_in(name, None)   // text parser, census, buyer_equal: scoped entries never match
mentions_placeholder(name)                              // the walk's net (fn(&str) -> bool, canonical.rs:16428): any placeholder or blank
                                                        // stem or whole value, before summary_shape and the scope
summary_shape(raw, f) = company_token || the Polish forms || several_lots || numbered_entries || lettered_entries
                        (partyname.rs:587, :415, :622, :479, :533; reused unchanged)
plain_case(raw)       = no lowercase letter directly followed by an uppercase one      // keeps `InGen`
```

`summary_shape` is `names_an_award` without its award-word tests (`award_clause`, `awardee_noun`). Those tests would read "the list of **awarded** suppliers" inside a pointer as an award; `placeholders_are_their_own_class` (partyname.rs:953) pins that pointer as a `Placeholder` today. The shape helpers are reused unchanged. So a multi-entry string made only of placeholders stays `AwardSummary`, which errs toward keeping the party, as 510's exemption does.

**Guards.**
1. **Whole-value only**, after the lot strip, for every new single word or abbreviation. A legal form changes the fold, so whole values need no separate legal-form guard. These all stay names: `N/A s. r. o.` (fold `n a s r o`), `NIL d.o.o.` (`nil d o o`), `Diverse Care Services Ltd`, `Brake` (≠ `brak`), `VÁRIOS MUNDOS UNIP. LDA.`, `Confidential Waste Services`, `Multiplex`, `Unknown Architects`.
2. **Language scope plus plain case** for `ingen` and `inget` (DAN, SWE, NOR, NOB, NNO) and for `nd` (POL). The language is the notice's own, which `NoticeState::read` already computes. An unknown language means no match, so the party is kept. This guard protects:
   - 1188404 `Ingen` FR (134 mentions, 137 winner rows; the census did not read its notices), if those notices are French;
   - the two `InGen` orgs, by case;
   - `ND` HR, by language (and by guard 3).
3. **Identifier guard, `Blank` only.** A `Blank` party that carries a gated identifier is kept. The gate at project.rs:4969-4974 is factored into one function used by both callers. A party that says nothing but carries an identity is a real party with a missing name. The census has examples:
   - the DE platform GUIDs on `.` and `-` (24291540, 24291539); idgate.rs:402-413 records that such GUIDs are per-organization keys and do the linking;
   - `0` DK with a CVR-shaped identifier;
   - `Keine Angabe` with a DE VAT number.

   Measured: 11 orgs and 53 mentions are kept, `ND` HR included. Three of them are review/mediation junk (`keine`, `Nicht vorhanden`, `No hay`: 35 mentions), which is accepted.

   `Placeholder` gets no identifier guard. Its text says the identity is withheld, elsewhere or plural, so an identifier beside it belongs to the placeholder, not to a supplier. `Withheld Section94 Supplier` carries one PPON on all 123 mentions, which cover different withheld suppliers. `Various` IE carries `10003`. This settles the trade-off in the census's observation 6 by class.
4. **Stems only for multiword phrases** that no company name contains, and each one is guarded by `summary_shape`.

**Lexicon.** Admitted census figures. The last numeric column is review-mediation / winner-bidder / buyer party rows.

| entries (fold) | class, match | orgs / mentions | rm / wb / buyer | trap → guard |
|---|---|---:|---|---|
| no letter at all: `.` `-` `--` `---` `/` `*` `?` `./.` `-/-` `0` | Blank, raw-name arm (the text parser's letter rule, parse.rs:147) | 95 / 3,852 | 2,972 / 737 / 246 | a nameless party with an identifier → 3 |
| `nie dotyczy` | Blank, stem | 548 / 5,721 | 7,667 / 8 / 4 | — |
| `not applicable` (510's Placeholder stem, reclassed) | Blank, stem | 908 / 2,169 | 2,626 / 9 / 1 | — |
| `keine angabe` (also takes `keine Angaben`) | Blank, stem | 118 / 952 | 1,095 / 245 / 21 | — |
| `niet van toepassing` | Blank, stem | 100 / 313 | 223 / 117 / 0 | — |
| `n a` (`N/A`, `N.A.`, `n/ a`, `(N/a)`) | Blank, whole | 38 / 3,423 | 3,717 / 145 / 7 | `N/A s. r. o.`, `Bank of America, N.A.` → 1 |
| `no procede` | Blank, whole | 3 / 3,209 | 4,420 / 14 / 0 | — |
| `x` `xx` `xxx` | Blank, whole | 162 / 2,556 | 2,976 / 241 / 1 | `X Corp` → 1 |
| `entfallt` `entfaellt` | Blank, whole | 363 / 1,494 | 1,713 / 263 / 0 | — |
| `neant` | Blank, whole | 533 / 1,073 | 811 / 458 / 0 | — |
| `keine` `nicht vorhanden` `nicht bekannt` `nicht zutreffend` `nicht anwendbar` `nicht relevant` `ohne` `nein` `k a` `ka` | Blank, whole | 45 / 1,727 | 2,180 / 32 / 0 | the identifier-bearing ones → 3 |
| `na` `none` `none specified` `unknown` `not known` `not specified` `not provided` `not available` `not required` `not relevant` `tbc` `tba` `to be confirmed` | Blank, whole | 259 / 1,407 | 1,372 / 231 / 3 | `Unknown Architects` → 1 |
| `n v t` `nvt` `geen` `niet bekend` | Blank, whole | 169 / 1,015 | 942 / 201 / 0 | — |
| `brak` `brak danych` `n d` | Blank, whole | 55 / 672 | 875 / 16 / 0 | `Brake` → 1 |
| `sans objet` `so` `sans` `non` `nc` `n c` `aucun` `aucune` `non renseigne` `non connu` `non concerne` `non applicable` | Blank, whole | 100 / 571 | 383 / 199 / 0 | the two `sans objet` orgs with FR identifiers → 3 |
| `nessuno` `nessuna` `non previsto` `non applicabile` `non noto` `ninguno` `ninguna` `no aplica` `no hay` `desconocido` `no` `null` | Blank, whole | 72 / 445 | 436 / 79 / 0 | — |
| `ikke relevant` `ikke aktuelt` `ej aktuellt` `nao aplicavel` `nao se aplica` `δεν εφαρμοζεται` `δεν απαιτειται` `nu este cazul` `nem relevans` `neni` `ei ole` | Blank, whole | 14 / 169 | 212 / 4 / 0 | — |
| `ingen` `inget` | Blank, whole, scoped da/sv/no, plain case | 514 / 788 | 347 / 461 / 0 | 1188404 `Ingen` FR, `InGen` ×2 → 2 |
| `nd` | Blank, whole, scoped pl | 3 / 257 | 311 / 2 / 0 | `ND` HR (OIB) → 2, 3 |
| 510's stems `see section`, `perfil del contratante`, `perfil de contratante`, `voir autres informations`, `voir renseignements`, `ver informaci`, `would prejudice` | Placeholder, stem (unchanged) | 190 / 457 | 74 / 354 / 0 | `SEE LAUER`, `See & Go` (510) |
| `withheld` | Placeholder, stem | 5 / 140 | 1 / 363 / 0 | Section94's PPON → no guard, by design |
| `see contracts finder` | Placeholder, stem | 1 / 115 | 0 / 127 / 0 | — |
| `siehe oben` | Placeholder, stem | 401 / 1,234 | 1,424 / 4 / 2 | — |
| `s o` (the DE hub is `s.o.` = *siehe oben*), `see below`, `please see below`, `see attached`, `see attached list`, `see list below`, `see additional information`, `see vi 3`, `siehe unten`, `siehe vergabeunterlagen` | Placeholder, whole | 26 / 1,581 | 1,780 / 78 / 1 | — |
| `confidential` `redacted` `commercially sensitive` `not published` `not disclosed` `non communique` | Placeholder, whole | 119 / 151 | 0 / 153 / 0 | `Confidential Waste Services` → 1 |
| `various` (510); `various suppliers`/`contractors`/`providers`/`companies`/`organisations`/`see below`; `multiple`, `multiple suppliers`/`providers`/`contractors`; `varios` (`vários`), `varias`, `varios adjudicatarios`/`licitadores`/`proveedores`/`contratistas`, `varias empresas`, `diversos`; `diverse`, `diverse firmen`/`bieter`/`ditte`, `diversi`, `divers`; `verschiedene`, `verschiedene firmen`, `mehrere`, `mehrere bieter`; `ditte varie`, `varie` | Placeholder, whole | 246 / 742 | 3 / 821 / 0 | `Diverse Care Services Ltd` (47 winner rows), `Diversey`, `VÁRIOS MUNDOS`, `Variosun GmbH`, `Multiplex` → 1 |

**Totals.** Admitted: 5,088 orgs, 36,234 mentions, 45,908 party rows. Kept by guards: 14 orgs and 189 mentions (guard 3: 11 / 53; guard 2: 3 / 136). Not admitted: 133 mentions (the trap keys and the deferred words).

**Deferred (not admitted, 60 mentions).**
- `nil` (3 orgs / 33): 16700736 is NIL SAS, with a SIRET, and the country-less `NIL` with 13 winner mentions may be NIL d.o.o.
- `ei` (2 / 15): in French, `EI` is a sole-trader legal form.
- `sin` (1 / 1): `SIN` FR.
- `vari` (2 / 11): `VARI` is a brand.

510's void wordings under 20 mentions stay `VoidLot` residuals; they are not this issue's.

**Text parser.** `plausible_name` (parse.rs:146-154) keeps `not_a_name(name).is_none()`. It refuses every class, so by construction no refusal is lost. The new refusals apply only when a notice is reparsed. The scoped words are kept there, because the text parser passes no language.

### 5. Award summaries: keep until per-lot parsing exists

The ~500 orgs `names_an_award` keeps (job 2182's `exempted`) become `AwardSummary`, and the fold keeps them exactly as today. They are not dropped as winners, for three reasons:
- **Each is a leaf.** There is one org per summary string, mostly with one mention, so it joins no unrelated Tenders, unlike the hubs above.
- **It is the only place the real winners' names reach the served Tender** (`Lot 1) Sarl Bremond. Lot 2) S.A. Les Rapides Varois …`). Dropping it turns a readable award into `selec-w` with no winner and no way back.
- **The right outcome needs separate work.** Per-lot winners need the legacy reader to split the string and key each part to a lot (`several_lots`' designators are a starting point), and that raises its own precision questions. File it as a new issue together with unit 2.

They stay listed in `void-names-refold`'s `exempted`. The placeholder walk lists its own `summary_shape` exemptions and never re-queues either set. The text parser keeps refusing them (510 §4's accepted asymmetry).

### 6. `NON_NAME_FOLDS`: unify

- **Delete** `NON_NAME_FOLDS` (role_census.rs:546-562). `buyer_equal` (:537-539) then refuses name equality when `crate::partyname::not_a_name(&buyer.name).is_some()`.
- **Coverage.** 14 of its 15 folds are in §4:
  - as `Blank`: `n a`, `na`, `none`, `unknown`, `not known`, `not specified`, `not provided`, `not applicable`, `x`;
  - as `Placeholder`: `confidential`, `withheld`, `not published`, `not disclosed`, `various`.
- **`nil` leaves.** It is a real company (NIL SAS), and the census found it only in winner and Tenderer slots, so no buyer-equal verdict moves.
- **Why the check stays.** Once the fold drops these parties, `buyer_winner_sections` (project.rs:4279) never sees them. The check remains for the census path and for the parties that are kept (a `Blank` with an identifier, an `AwardSummary`). One list, one fold, no drift.

### 7. Build (unit 2), on 510's machinery

**U2a: predicate** (`crates/ingest/src/partyname.rs`). Pure.
- Add `NotAName::{VoidLot, Placeholder, Blank, AwardSummary}`. A void stem that names an award returns `AwardSummary` (:290).
- Add `BLANK_STEMS`, `BLANK_WHOLE` and `SCOPED_WHOLE: &[(&str, &[&str])]`, plus the new `PLACEHOLDER_STEMS` / `PLACEHOLDER_WHOLE` entries (:134, :146). ` not applicable` moves to `BLANK_STEMS`.
- Add the no-letter arm, `not_a_name_in`, `not_a_name`, `mentions_placeholder`, `summary_shape` and `plain_case`.
- Tests:
  - `every_measured_placeholder_spelling_has_its_class`: one prod specimen per entry, including `See Section VI.3) Additional information`, which also checks that `lettered_entries` stays quiet on `VI.3)`;
  - `punctuation_only_names_are_blank`, with `""` and `"  "` still returning `None`;
  - `scoped_words_need_their_language_and_a_plain_case`: `Ingen` in DAN → Blank; in FRA or with no language → None; `InGen` in DAN → None; `nd.` in POL → Blank; `ND` in HRV → None;
  - `a_stem_inside_a_summary_is_an_award_summary`: `Lot 1: ACME Ltd; Lot 2: not applicable`, `Część 1: X sp. z o.o.; część 2: nie dotyczy`;
  - `award_summaries_are_their_own_class`: 510's Sandoz, Bremond and Promocash shapes;
  - extend `real_names_beside_the_phrases_are_names` (:970) with `NIL`, `N/A s. r. o.`, `Bank of America, N.A.`, `X Corp`, `VARI, a.s.`, `EI`, `SIN`;
  - move `Not applicable` out of `placeholders_are_their_own_class` (:953) into a Blank test;
  - extend `every_entry_is_folded_and_every_stem_padded` (:1034) to the new lists;
  - `the_former_non_name_folds_are_classified`: every former `NON_NAME_FOLDS` entry except `nil`.

**U2b: fold** (`crates/ingest/src/project.rs`, `project/role_census.rs`).
- `void_party_sections` (:4379) becomes `dropped_party_sections(..) -> BTreeMap<String, NotAName>`, covering outer and inner sections, with the class combined per §1. It reads the notice language once. Per party, it reads the identifier fields that `mentions_and_void` reads (:4913-4945) through the factored gate, and applies guard 3. 510's majority rule (:4401-4405) stays `VoidLot`-only. It remains the one decider.
- `mentions_and_void` (:4855) skips every key in the map, and its second value (the retire keys) carries every class. The plan sweeps keep their shape (full :2114 / :2179; incremental :3130 / :3377).
- In `NoticeState::read`, the role filter (:4760-4762) and `read_results` (:4773) take the map.
- Add `RawLotResult.withheld_winner` beside `void_lot` (:5871).
  - Legacy (:6102-6107) and sdk-0.1 (:6226-6231): `VoidLot` sets `void_lot`, `Placeholder` sets `withheld_winner`, `Blank` is skipped.
  - Derivations (:6160-6172, :6244-6251): a real winner or `withheld_winner` → `selec-w`; then `void_lot` → `clos-nw`; then the existing rule.
  - The eForms/FTS member skip (:6021-6028) covers every class.
- Add `Report.dropped {void, placeholder, blank}`: the parties the decider dropped across planned notices, beside `mentions_retired` (:82). This is the daily fold's measure.
- In `role_census`, delete `NON_NAME_FOLDS`; `buyer_equal` calls the predicate (§6).
- Tests, on 510's harness (project.rs:13377 and `ingest/tests/project.rs`):
  - `a_withheld_winner_keeps_the_result_selected_with_no_winner` (legacy, dateless);
  - `a_blank_winner_is_judged_on_the_results_other_evidence` (value, no date, date only);
  - `a_placeholder_review_body_mints_nothing` (an eForms `nie dotyczy` mediation-body);
  - `a_blank_party_with_a_gated_identifier_is_kept` and `a_placeholder_party_with_an_identifier_is_dropped`;
  - `a_placeholder_buyer_leaves_no_buyer_tokens`: `GuardSide` tokens and `buyer_key`, with and without a second, real buyer;
  - `a_scoped_word_follows_the_notice_language`;
  - `an_eforms_placeholder_tenderer_keeps_the_publishers_decision`;
  - `a_standing_placeholder_mention_is_retired_by_a_refold` (`Refold::{Incremental, Full}`, 434 harness);
  - `buyer_equal_refuses_every_non_name`.

**U2c: store** (`crates/store/src/canonical.rs`).
- `retire_mentions` (:16504) is class-blind and does not change.
- Two plan readers, both seeks on `tender_version_parties_org_role` (:12457) in `IN_CHUNK` chunks. The first is the census's own plan-checked shape.
  - `party_role_counts_of_orgs(orgs)`: `SELECT organization_id, role, COUNT(*) … WHERE organization_id IN (…) GROUP BY organization_id, role`.
  - `buyer_notices_of_orgs(orgs)`: `SELECT DISTINCT mention_notice_id, tender_id … WHERE organization_id IN (…) AND role IN ('buyer','Procedure-Buyer')`.
- Test both on a fixture with one buyer row and one mediation row.

**U2d: job** (`crates/app/src/supervisor.rs`).
- `Spec` is serialized (:231), so nothing is renamed. Add `Spec::RefoldPlaceholderNames { dry_run, expect }` and the request kind `refold-placeholder-names`, routed beside :2101-2125 with the same dry default and `expect` rule.
- Both arms call one `run_refold_non_names(job, class, dry_run, expect)`, generalised from `run_refold_void_names` (:5056). The class chooses:
  - the net: `mentions_void` or `mentions_placeholder`;
  - the split: `is_void_lot`, or `AwardSummary` → `exempted` with everything else re-queued, so the fold decides scope and identifier;
  - the report: `void-names-refold` or `placeholder-names-refold`.
- The placeholder plan adds:
  - `by_class`;
  - `identified` (Placeholder orgs with an identifier) and `identified_blank`;
  - `scoped` (net hits that `not_a_name` calls names, with country and mentions);
  - `roles`, from U2c;
  - `buyer_notices`, each with its Tender.
- 510's parts are reused unchanged: the wet path, the `expect` gate (:5168-5174), the chunked `unmark_projected_by_ids` + `stamp_stale_for_notices`, and the paired `project rebuild=false`.
- The new arm goes through `off_frame` and `Box::pin` (:5472; the CLAUDE.md stack note).
- `sweep_after_fold` (:3164, called at :4819 on `mentions_rebound + mentions_retired`) does not change.
- Tests: re-run 510's test (:15896) for the new kind, and check that `refold-void-names` still routes to the void class.

**Gate and deploy.** Gate through `ops/check.sh` only: supervisor.rs needs `--features server`, and a bare `cargo check -p tender-db` does not compile it. One deploy for U2a–U2d.

### 8. Drain (unit 3)

The figures here are the admitted census (§4). 510's census undercounted its drain by about 25 % (24,565 census mentions; drain 1 retired 30,755), so expect up to about 1.3× here.

1. **Deploy.** From then on the daily fold mints no placeholder party, and `Report.dropped` shows the daily rate. A full-fallback fold would retire corpus-wide; that is correct, and its `mentions_retired` will be large.
2. **`refold-placeholder-names`, dry.**
   - Expected:
     - orgs from 5,102 (5,088 admitted + 11 identified Blank + 3 scoped) up to about 6.6k;
     - mentions from 36,423 up to about 47k;
     - buyer rows about 286 on about 14 orgs, 241 of them on 11063;
     - `identified` = 30914328 and 23424157; `identified_blank` = the 11 orgs in §4; top of `scoped` = 1188404 FR (134);
     - top orgs: 1215973, 1213676, 1257377, 2147121, 8631351, 1765331, 1191345.
   - **Stop if** orgs or mentions fall more than 2 % below 5,102 / 36,423: the predicate or the walk regressed.
   - **Read 30 random `distinct` folds** if orgs exceed 8,000 or mentions exceed 60,000.
   - **Stop and add a keep test** if an `identified` or `legal_form` org reads as a company.
   - **Stop if** buyer rows exceed 1,000: the net reached real buyers.
3. **Buyer canary.**
   - Record the pre-image: for the plan's `buyer_notices`, `SELECT caused_by_notice_id, tender_id FROM tender_versions WHERE caused_by_notice_id IN (…)` (≤ 300 ids; index `tender_versions_notice`, canonical.rs:206).
   - Run `refold-notices` on those ids (cap 1,000, supervisor.rs:229), let its fold run, then re-read the same rows.
   - **Stop if** more than 10 % of these notices changed Tender. Read each one before the main run: it is a cited link the guard now admits as unknown. Decide whether placeholder buyers need an exception.
4. **Wet.** Run `refold-placeholder-names expect=<dry notices>` (it refuses above +25 %), then its paired fold.
   - Expected:
     - `mentions_retired` at least the dry mentions minus the `identified_blank` and `scoped` mentions, and at most the dry mentions (510 measured 949/949 agreement between org names and mention names);
     - `mentions_rebound` about 0;
     - `dropped.placeholder + dropped.blank` ≥ `mentions_retired`;
     - `retire_corrections` > 0 (ADR-0017 D5);
     - roughly 30–45 minutes (510: about 1 minute per 1,000 notices).
   - **Stop if** `mentions_retired` falls outside that band, and read the gap before the sweep.
5. **Sweep.** The fold queues `sweep-orphan-orgs` (auto) because it retired mentions. Expect `counted` to be about the dry orgs minus the orgs a guard keeps. Under `AUTO_SWEEP_CAP` (10,000, supervisor.rs:3147) the job sweeps on its own count, as designed. Above it, the plan is only recorded: read `protected`, then run wet (510's pattern).
6. **Verify**, with bounded `/v1/sql` reads and the API:
   - These all read 0:
     - `SELECT COUNT(*) FROM organization_mentions WHERE organization_id IN (1215973, 1213676, 1257377, 2147121, 8631351, 1765331, 1191345, 1321632, 1350611, 1528718, 1889418, 1166840, 11063, 30914328, 30918120, 1173051)` (`organization_mentions_org`);
     - the same id set on `tender_version_parties` (`_org_role`);
     - `tender_version_result_winners WHERE organization_id IN (30914328, 1173051, 1889418, 1166840, 30918120)` (`_org`).
   - Before step 4, capture up to 20 Tender ids each for 30914328 and 1166840 (`SELECT DISTINCT tender_id FROM tender_version_parties WHERE organization_id = ? LIMIT 20`). Afterwards, Section94's Tenders keep their published result status with no winner, and néant's read according to the evidence rule.
   - The 11 identified-Blank orgs keep their mentions.
   - 1188404 keeps 134 mentions if its notices are French (the scope held), or 0 if they are Scandinavian. Record which.
   - `name_prefix=` `nie%20dotyczy`, `N%2FA`, `Withheld` and `Various` return real names only (`N/A s. r. o.`).

### 9. Risks and pins

| risk | pin |
|---|---|
| A real company is dropped | Whole values only for single words; three scoped words; the real-name test; the dry plan's `identified` / `legal_form` stop rule |
| A real party with a missing name is dropped | Guard 3 (Blank + gated identifier) and its test |
| A withheld winner is read as no award | `withheld_winner` → `selec-w`, and its test |
| "Nobody" becomes a positive `clos-nw` claim | `Blank` carries no marker; its test |
| A summary's real winners are lost | `AwardSummary` is kept; `summary_shape` guards every non-void stem; its test |
| A buyer drop moves Tender grouping | The canary with its 10 % stop; the tokens test |
| `mentions()` and `read()` disagree | Both read `dropped_party_sections` |
| The new arm overflows `run_spec` | `Box::pin`; `ops/check.sh` |
