## Unit 1: decision (2026-10-11, revised the same day after two adversarial reviews)

Suggested status line: `Status: ready-for-agent — unit 1 DECIDED 2026-10-11, revised after review (../511-placeholders/unit1-decision.md, "Review"). NEXT: unit 2 build (U2a predicate, U2b fold + NON_NAME_FOLDS; U2c plan readers and U2d job once 513 U2 is committed; one gate, one deploy), then unit 3 drain (§8: identified-placeholder read, dry, mandatory non-exact read, canary, wet, sweep).`

**In one paragraph.** The fold drops every party whose every published name is a dropping non-name, in every role and every era, the buyer slot included. It does this on 510's machinery with the class widened: one decider (`void_party_sections` becomes `dropped_party_sections`), the same skip in `NoticeState::mentions`, the same role and result filter in `NoticeState::read`, the same class-blind `Db::retire_mentions` and the same 443 sweep. `partyname::NotAName` gets four classes:
- `VoidLot` (510, unchanged);
- `Placeholder`, narrowed to "a winner exists; its name is elsewhere, withheld, missing or plural";
- `Blank` (new): "this slot holds nothing" (`n/a`, `none`, `néant`, `nie dotyczy`, punctuation-only);
- `AwardSummary` (new): the names 510's `names_an_award` keeps, and a pointer followed by a name. **It never enters the drop map**, so the fold keeps these parties.

A legacy or sdk-0.1 result left with no real winner reads the class. `VoidLot` gives `clos-nw`. `Placeholder` gives `selec-w` with no winner. `Blank` carries no marker, so the existing evidence rule decides, as if the slot were empty. eForms and FTS keep the publisher's BT-142 and the Bid. Four guards protect real names:
- whole-value matching for every new single word;
- stems anchored at the start of the name, plus a keep rule for a pointer followed by a name;
- a language scope plus a plain-case test for the three short words with measured real-name collisions;
- an identifier guard for `Blank` only, which refuses an identifier that echoes a non-name.

`NON_NAME_FOLDS` is deleted in favour of the predicate. Award summaries stay as they are until per-lot parsing exists (a follow-up issue). The drain is `refold-void-names` generalised by class (`refold-placeholder-names`): a read of every hit that is not an exact lexicon entry, a buyer-slot canary, the wet refold, then the 443 sweep.

**Basis.**
- `census.md` and `census.json` in this directory: 36 bounded calls on 2026-10-11, on exact `name_norm` keys. Wherever a figure below is called "admitted", it is that census re-classified under §4's lists, per org name and country, with the guards applied.
- The data review added 8 bounded reads the same day, kept in the session scratchpad under `r511/`:
  - a prefix-range seek per stem on `organizations_name_country` (1,493 names, 1,362 of them outside the census), and their role split;
  - 42 digit and punctuation equality arms, and their roles;
  - the 241 mentions of 11063;
  - the `keine angabe` winners and lot results.

  Figures from these reads are marked "(review read)".
- **Plan width.** The census planned its role-split IN shape at 100 ids (`plan4.sql`) but sent it at up to 400. The review re-planned it at 229 ids and it still read `SEARCH … USING INDEX tender_version_parties_org_role`. Every review read was planned at the width it was sent. Later reads for this issue record each plan at the width actually sent.

**Line numbers** refer to HEAD `493457c`; the cited source files are unchanged at `bdf35f5`. Uncommitted 513 U2 edits shift `canonical.rs` and `supervisor.rs` in the working tree, so find each site by the symbol named beside it.

### 1. Classes

| class | means | a legacy / sdk-0.1 result left with no real winner | specimens | admitted: orgs / mentions / party rows |
|---|---|---|---|---|
| `VoidLot` (510) | the lot was not awarded | `clos-nw` (a marker; it outranks value and date) | `Infructueux`, `Desierto` | drained by 510 |
| `Placeholder` (narrowed) | a winner exists; its name is elsewhere, withheld, missing or plural | **`selec-w`, no winner bound** (a marker) | `Withheld Section94 Supplier`, `See Section VI.3 …`, `ver perfil del contratante`, `siehe oben`, `keine Angabe`, `TBC`, `Various suppliers` | 1,172 / 6,631 / 8,277 |
| `Blank` (new) | the slot holds nothing | **no marker.** Legacy: the existing rule decides (value → `selec-w`; no date → `clos-nw`; date only → NULL; project.rs:6160-6172). **sdk-0.1: NULL unless coded** (issue 257: only a winner gives `selec-w` and only `void_lot` gives `clos-nw`, :6245-6251) | `N/A`, `nie dotyczy`, `néant`, `Ingen` (da/sv/no), `.`, `0` | 3,917 / 29,605 / 37,633 |
| `AwardSummary` (split out of `Placeholder`) | a non-name phrase beside a name: the award, the company or the body it names | kept as today; never enters the drop map | `Lot 1) Sarl Bremond. Lot 2) … Lot 3) Déclaré infructueux`, `siehe oben: NEB Betriebsgesellschaft mbH` | ~500 (job 2182 `exempted`), plus the keep rule's hits (§4) |

- **Class by winner-slot meaning.** The class matters only in a winner slot and for guard 3, so each word is classed by what it says in a winner slot:
  - a word saying the winner's name is missing, pending or withheld is `Placeholder`;
  - a word saying there is nothing or nobody to name, or a filler, is `Blank`.

  The re-check moved these words from `Blank` to `Placeholder`:
  - by ruling: `keine angabe` (a stem, which also takes `keine Angaben`), `tbc`, `tba`, `to be confirmed` and `not provided`;
  - by the same test: `k a` / `ka` (that is, *keine Angabe*), `not specified`, `not available`, `unknown`, `not known`, `nicht bekannt`, `niet bekend`, `non connu`, `non noto`, `desconocido`, `non renseigne` and `brak danych`.

  The moved whole values hold 66 winner/bidder rows between them; `keine angabe` holds 247.

  Every other `Blank` word stays, because each says "not applicable", "none" or nothing at all. `n a`, `na`, `nc`, `n d` and `none specified` are ambiguous in a winner slot, and they stay `Blank`, which asserts nothing.
- **`keine Angabe` is the usual German and Austrian wording for a withheld winner.** The review read 216 legacy/sdk-0.1 lot results whose only winner is such a text. All are `selec-w` today, and none has a co-winner. Specimens: 'Keine Angabe - der Zuschlag ging an mehrere Bieter, die weiterhin im Wettbewerb stehen' (9535761) and 'keine Angabe aus Gründen des Wettbewerbs' (14465310).
  - As `Blank`, 27 of them would become NULL and 7 would become `clos-nw`; 6 of those 7 contradict the text.
  - As `Placeholder`, all 216 stay `selec-w` with no winner. One of them, `… Los 3 wurde aufgehoben`, is truly void and still reads `selec-w`, as it does today.
- **The "nobody" words stay `Blank`.** Census observation 1 reads them as "nobody was awarded": `ingen` 600 winner rows, `néant` 458, `niet van toepassing` 117, `geen` 110, `aucun` 100. 137 of `ingen`'s rows sit on 1188404, which is probably a French firm (§4, guard 2). These words fill review and mediation slots just as often (`ingen` 347 rows, `néant` 811, `nie dotyczy` 7,667), so they fail 510's `VOID_WHOLE` admission ("no other-role use"), and nobody has read their value/date split. Reading the slot as empty asserts nothing the publisher did not:
  - a legacy block with no value and no date still reads `clos-nw` under the existing rule (on 510's four hubs, 6,266 of 8,128 rows had that shape);
  - a block with a value reads `selec-w` with no winner.

  Promoting any of `ingen`, `néant`, `geen`, `aucun` or `niet van toepassing` to `VoidLot` first needs 510's per-word value/date table. This is a follow-up measurement (§8.7).
- **Why `Placeholder` carries a marker.** The text says a winner exists. Left to the evidence rule, a dateless legacy block would read `clos-nw`, which contradicts the publisher.
- **The decider and mixed parties.**
  - A party enters the drop map only if every name is `VoidLot`, `Placeholder` or `Blank`. A real name (`None`) or an `AwardSummary` name keeps the party.
  - 510's majority rule (four or more names, at least half of them `VoidLot`) is the one other way in, and it stays `VoidLot`-only.
  - A party with any `AwardSummary` name never enters, whatever its other names are.
  - Among the dropping names, the party takes `Placeholder` over `VoidLot` over `Blank`: a withheld winner beside a void phrase means a winner exists.
  - At the result, a `Placeholder` ranks with the real winners, so `selec-w` beats the void marker, as it does for 510's real co-winner.

### 2. Per role: drop the party in every role

Admitted party rows, `Blank / Placeholder`:

| role | rows | decision | what else changes |
|---|---:|---|---|
| `winner` (legacy direct winner; sdk-0.1 `WinningParty`) | 2,480 / 1,280 | drop | the result per §1; no `tender_version_result_winners` row |
| `AWARD_AND_CONTRACT_VALUE` (internal-ojs, unfolded) | 335 / 82 | drop | caught by the role filter, as in 510 |
| `Tenderer` (eForms/FTS `TenderingParty` member), `Tenderer-SubCont` | 334 / 851 | drop the bid party | the Bid and BT-142 stay (510's eForms rule). Section94's 345 rows keep the publisher's decision and bind no winner |
| `review-body`, `mediation-body`, `appeal-information`, `Lot-ReviewOrg`, `Lot-Mediator`, `Lot-ReviewInfo`, `RESPONSIBLE_FOR_MEDIATION_PROCEDURES`, `Part-*` | 6,436/1,490, 22,702/2,762, 828/673, 1,683/428, 503/239, 336/26, 361/65, 22/7 | drop | nothing: no result, token or key reads them |
| `purchasing-body`, `Procedure-CPB` | 250/171, 3/0 | drop | nothing: neither is in `BUYER_ROLES` (project.rs:7385) or `GUARD_SIGNATORY_ROLES` (:7472) |
| `specifications-provider`, `further-information`, `tender-receipt`, `Lot-TenderReceipt`, `Lot-DocProvider`, … | 559/57, 248/52, 251/68, … | drop | nothing |
| `buyer`, `Procedure-Buyer` | 260 / 26 | **drop** (§3) | buyer tokens, the buyer key, 482's refused groups |

- **Keep** is decided by the party's shape, not by its role. Two shapes are kept:
  - an `AwardSummary` name (§5, and the pointer keep rule in §4);
  - a `Blank` party with a gated identifier that does not echo a non-name (§4, guard 3).
- **Map** applies to no role. A placeholder names no organization to map to. The identifier is the only evidence that could map it, and guard 3 already keeps the party binding by it.
- **Why one rule for every role.** The class belongs to the party section, which `void_party_sections` already decides once for all roles (project.rs:4379-4409). A per-role exception would put "is this a name?" in two places, which 510 §1 rejected. Keeping these parties has a measured cost: hubs that join unrelated Tenders.
  - 1215973 `nie dotyczy` PL sits on 6,960 party rows.
  - 1213676 `No procede` ES has 3,192 mentions.
  - 1257377 `N/A` GB has 2,129 mentions across review, mediation, winner and buyer slots.

### 3. Buyer slot: drop, no exception

286 rows on 14 orgs (260 `Blank`, 26 `Placeholder`):
- 241 on 11063 `.` AT, an eForms `Procedure-Buyer` whose raw identifier is also `.` on all 241 mentions (review read). The gate rejects that identifier today, and guard 3's test pins it.
- 21 on 36188 `Keine Angabe.` (now `Placeholder`), 6 on 1257377 `N/A` GB, 4 on 1215973 `nie dotyczy`, 3 on 1164345 `-`, 2 on `To be confirmed` GB and 2 on `siehe oben` DE.
- 1 each on seven more.

Digit-only buyers are not in this count: `2` GB (`Procedure-Buyer` 8), `5` BE (2), `2` IT (2) and `1` CH (1) (review read). They stay names (§4).

**What the placeholder does to grouping today.**
- **Tokens.**
  - `.` yields no name token, because `buyer_guard_tokens` skips an empty fold (project.rs:7633). Its only token is therefore `o:11063` (`add_org_tokens`, :8093-8103), and all 241 notices share it. The 481 guard (`store::buyer_tokens_disjoint`) reads them as one buyer among themselves and as a different buyer from every other notice.
  - `N/A` GB yields `n2:GB:n a`, which every GB notice with that buyer shares.
- **Key.** `buyer_key_of` (:7557) gives `N/A` GB the key `n2:GB:n a`. Under a refused procedure key, 482's `refused:{key}:{buyer}` (:3233) then groups every such notice as if they had one buyer.

**After the drop**, the notice's buyer side is its other buyers, or none.
- Empty tokens mean "unknown", which the guard never refuses (`buyer_tokens_disjoint`; 481 2b, project.rs:7573).
- **A signatory alone never makes the notice known** (481 2c). `buyer_side_mentions` returns early only when the raw buyer references are empty (:7502-7504). After a placeholder buyer is dropped, the references are still there but no buyer mention survives. A separate, real `Contract-Signatory` would then fall through to `signing` and feed 481's tokens on its own. U2b therefore returns `(vec![], vec![])` whenever the folded buyer list is empty. That also closes the same hole for a 483 demote with no promotion.
- A `None` buyer key means "names no buyer" to 369's gate.

These are the designed states for a notice that names no buyer, so the drop replaces a false identity with the designed unknown. The change starts at the ~290 notices that had a placeholder buyer, but it can reach their procedure-key families. Dropping a placeholder buyer key lowers that key's `count(DISTINCT buyer_key)` (369's gate, project.rs:7387-7400), which can flip the whole family from refused to admitted. A cited link refused today as buyer-disjoint becomes admitted, and a refused group keyed on a placeholder buyer dissolves. The canary in §8 measures the notices and their key families before the main run.

**Rejected alternatives.**
- **Keep:** leaves the false shared token and key, and serves `.` as the buyer of 241 Tenders.
- **Map:** has no target. None of the 14 orgs carries a gated identifier. Promoting another party is 483's `real-buyer-elsewhere` decision, which still runs on the remaining parties after the drop (project.rs:4766).

### 4. The predicate and the lexicon

```text
not_a_name_in(name, lang) -> Option<NotAName>   // the fold; lang = project::original_lang(parsed) (project.rs:9039)
  t = name.trim()
  t non-empty and (no alphanumeric char, or t in {0, 00, 000, 0000}) -> Blank   // NEW, before the fold (an empty fold is None, partyname.rs:282)
  f = fold(name); empty -> None
  VOID_STEMS hit (left word boundary) -> AwardSummary if names_an_award else VoidLot   // 510; only the class name changes (:290)
  h = f after one optional leading lot designator     // a LOT_DESIGNATORS word, up to two NUMBER_MARKS, a number: `lot 3 …`, `lote n 2 …`
  PLACEHOLDER_STEMS hit -> AwardSummary if pointer_names(name) or summary_shape else Placeholder
                           // anchored: h starts with the stem; `perfil del/de contratante` and `would prejudice` keep 510's left word boundary
  BLANK_STEMS hit (anchored) -> AwardSummary if summary_shape else Blank
  w = fold(strip_lot_qualifier(name)) or f
  VOID_WHOLE / reason_head / void_prefix -> 510, unchanged (its award case stays None: `DESIERTO, S.L.`)
  w in PLACEHOLDER_WHOLE                 -> Placeholder
  w in BLANK_WHOLE                       -> Blank
  w in SCOPED_WHOLE, lang in its languages, plain_case(name) -> Blank
  None
not_a_name(name)          = not_a_name_in(name, None)   // text parser, census, buyer_equal: scoped entries never match
mentions_placeholder(name)                              // the walk's net (fn(&str) -> bool, canonical.rs:16428): the raw-name arm,
                                                        // any placeholder or blank stem at ANY left word boundary, any whole value;
                                                        // before the anchor, the keep rule, summary_shape and the scope
summary_shape(raw, f) = company_token || polish_company_form || several_lots || numbered_entries || lettered_entries
                        (partyname.rs:587, :415 (inline today, extracted by U2a), :622, :479, :533)
plain_case(raw)       = no lowercase letter directly followed by an uppercase one      // keeps `InGen`
```

`summary_shape` is `names_an_award` without its award-word tests (`award_clause`, `awardee_noun`). Those tests would read "the list of **awarded** suppliers" inside a pointer as an award; `placeholders_are_their_own_class` (partyname.rs:953) pins that pointer as a `Placeholder` today. The shape helpers are reused unchanged. So a multi-entry string made only of placeholders stays `AwardSummary`, which errs toward keeping the party, as 510's exemption does.

**Anchored stems.**
- **The problem.** A stem matched at any left word boundary drops a real name followed by the phrase: `Firma Handlowa Jan Kowalski – nie dotyczy`, `Siemens – see section VI.3`. `summary_shape` protects only company forms and list shapes, so it does not save them.
- **The anchor.** Anchoring at the start, after an optional leading lot designator, still catches the qualified forms the stems exist for: `Nie dotyczy - brak`, `Lot 3: not applicable`, `Withheld for Security Reasons`.
- **The exceptions.** Two keep 510's match because they never open a name: `Véase / Ver perfil del contratante` and `Disclosure would prejudice …`. Anchoring `would prejudice` would retire it from both the fold and the text parser.
- **What the anchor costs.** A hit in the middle of a name cannot be read with an index. The walk's net still catches it, so the dry plan lists it (§8.2).

**The pointer keep rule** (`pointer_names`). It applies to the pointer stems only: `see section`, `siehe oben`, `see contracts finder`, `voir autres informations`, `voir renseignements`, `ver informaci`, `perfil del contratante` and `perfil de contratante`.
1. Take the rest after the stem and the word it ends in.
2. Strip the section references: `VI.4.1`, `VI.3)`, `1)`, `Ziff.`/`Ziffer`, `Punkt`, `Abschnitt`, `unter`, `Nr.`, `below`/`above` (and `unten`/`oben`), and one lot designator with its number (`- Lot 2`).
3. If the rest then opens with a separator (`:`, ` - ` or its en and em dashes, `(`), strip the references again.
4. If what remains names something, the name is `AwardSummary` and the party is kept. Naming something means a word of two or more letters that starts with a capital, or a legal form (`mbH`, `GmbH`, `AG`, `KG`, `e.V.`, `Ltd`, `SA`, …).

A `see section` with no section reference at all needs no separator (`see section Royal Courts of Justice`).

On the review's 1,362 prefix names outside the census, the rule keeps:
- 7 real bodies: NEB Betriebsgesellschaft mbH (a winner), Vergabekammer Südbayern, WSA Toenning, VKS, UVS (×2) and Royal Courts of Justice;
- 2 junk names, `See section VI.3 (Additional Information)` and `Siehe oben II („Ausschreibungs- …“)`. `summary_shape` keeps 3 more junk pointers (`see section 1 … Viridian Group Ltd contacts` ×2, `see sections ll.2.4 Lot 2 and Lot 3`). Both err toward keeping the party.

It drops `siehe oben (VI.4.1)`, `See section VI.3 - Lot 2` and the 140-mention `See Section VI.3 Additional information`.

**Guards.**
1. **Whole-value only**, after the lot strip, for every new single word or abbreviation. A legal form changes the fold, so whole values need no separate legal-form guard. These all stay names: `N/A s. r. o.` (fold `n a s r o`), `NIL d.o.o.` (`nil d o o`), `Diverse Care Services Ltd`, `Brake` (≠ `brak`), `VÁRIOS MUNDOS UNIP. LDA.`, `Confidential Waste Services`, `Multiplex`, `Unknown Architects`.
2. **Language scope plus plain case** for `ingen` and `inget` (DAN, SWE, NOR, NOB, NNO) and for `nd` (POL, ITA). The language is the notice's own, which `NoticeState::read` already computes. An unknown language means no match, so the party is kept. This guard protects:
   - 1188404 `Ingen` FR (134 mentions, 137 winner rows), if its notices are French. The census's trap read shows notice 4354963 awarding at least 6 ORG sections to 'INGEN', all with FR addresses, which points to a real French firm;
   - the two `InGen` orgs, by case;
   - `ND` HR, by language (and by guard 3).

   `ND` IT (9119141, 29 mentions) and Polish `nd.` are dropped, which is consistent with `N.D.` IT (fold `n d`, unscoped). The dry plan's `scoped` uses the org's country as a proxy for the language (§7, U2d).
3. **Identifier guard, `Blank` only.** A `Blank` party that carries a gated identifier is kept, unless the identifier echoes a non-name:
   - the identifier is itself a non-name (`not_a_name_in(raw_identifier, lang).is_some()`: `.`, `N/A`, `ENTFAELLT`); or
   - its space-free fold contains the party name's space-free fold, when that name has a letter: `KEINE…` on `keine`, `NIEDOTYCZY` on `nie dotyczy`. A normalised identifier has no spaces, and a no-letter name has nothing to echo, so `0` keeps its `38160540`.

   The gate at project.rs:4969-4974 is factored into one function used by both callers. A party that says nothing but carries an identity is a real party with a missing name. The census has examples:
   - the DE platform GUIDs on `.` and `-` (24291540, 24291539); idgate.rs:402-413 records that such GUIDs are per-organization keys and do the linking;
   - `0` DK with the CVR-shaped `38160540` (23095417);
   - `Nicht vorhanden` DE with `T04089980` (23018279).

   Measured: 10 orgs and 51 mentions are kept, `ND` HR included. Three are review/mediation junk (`keine`, `Nicht vorhanden`, `No hay`: 35 mentions), which is accepted. 23307388 `keine` carries a review chamber's name as its identifier (`2VERGABEKAMMERBEIM…SACHSENANHALT`), which neither echo test catches.

   Publishers do type the placeholder into the identifier field: `KEINEANGABEGEM62ABS3BVERGG2018` on 17888810 AT and 17660142 DE, `111222333` on 22763666 (review read). Those names are `Placeholder` now, so no guard applies to them.

   **`Placeholder` gets no identifier guard.** Its text says the identity is withheld, elsewhere, missing or plural, so an identifier beside it belongs to the placeholder, not to a supplier.
   - `Withheld Section94 Supplier` carries one PPON on all 123 mentions, which cover different withheld suppliers. `Withheld Section94 Supplier 2` carries another (31548264, review read).
   - `Various` IE carries `10003`.

   **The cost of moving `keine angabe`:** 23186509 `Keine Angabe` DE, with VAT `DE198511192` (2 `Tenderer` rows), loses guard 3 and is dropped. The VAT may be the withheld winner's own. That is not decided blind: 23186509 is on the dry plan's read list (§8.2). And this evidence about identifiers comes only from orgs whose head name is the placeholder, so the fold also counts the `Placeholder` parties it drops while they carry a gated identifier (`Report.dropped.placeholder_identified`, U2b). §8.1 reads a sample of them before the wet run.
4. **Stems only for multiword phrases**, anchored as above. Each is guarded by `summary_shape`, and the pointer stems also by the keep rule.

**Lexicon.** Admitted census figures. The last numeric column is review-mediation / winner-bidder / buyer party rows.

| entries (fold) | class, match | orgs / mentions | rm / wb / buyer | trap → guard |
|---|---|---:|---|---|
| no alphanumeric: `.` `-` `--` `---` `/` `*` `?` `./.` `-/-`; only zeros: `0` (and `00` `000` `0000`, absent from the census) | Blank, raw-name arm | 95 / 3,852 | 2,972 / 737 / 246 | a nameless party with an identifier → 3 |
| `nie dotyczy` | Blank, stem | 548 / 5,721 | 7,667 / 8 / 4 | — |
| `not applicable` (510's Placeholder stem, reclassed) | Blank, stem | 908 / 2,169 | 2,626 / 9 / 1 | — |
| `niet van toepassing` | Blank, stem | 100 / 313 | 223 / 117 / 0 | — |
| `n a` (`N/A`, `N.A.`, `n/ a`, `(N/a)`) | Blank, whole | 38 / 3,423 | 3,717 / 145 / 7 | `N/A s. r. o.`, `Bank of America, N.A.` → 1 |
| `no procede` | Blank, whole | 3 / 3,209 | 4,420 / 14 / 0 | — |
| `x` `xx` `xxx` | Blank, whole | 162 / 2,556 | 2,976 / 241 / 1 | `X Corp` → 1 |
| `entfallt` `entfaellt` | Blank, whole | 363 / 1,494 | 1,713 / 263 / 0 | — |
| `neant` | Blank, whole | 533 / 1,073 | 811 / 458 / 0 | — |
| `keine` `nicht vorhanden` `nicht zutreffend` `nicht anwendbar` `nicht relevant` `ohne` `nein` | Blank, whole | 37 / 1,284 | 1,660 / 24 / 0 | the identifier-bearing ones → 3 |
| `na` `none` `none specified` `not required` `not relevant` | Blank, whole | 210 / 694 | 651 / 180 / 1 | — |
| `n v t` `nvt` `geen` | Blank, whole | 167 / 1,013 | 942 / 198 / 0 | — |
| `brak` `n d` | Blank, whole | 54 / 671 | 875 / 15 / 0 | `Brake` → 1 |
| `sans objet` `so` `sans` `non` `nc` `n c` `aucun` `aucune` `non concerne` `non applicable` | Blank, whole | 97 / 488 | 325 / 197 / 0 | the two `sans objet` orgs with FR identifiers → 3 |
| `nessuno` `nessuna` `non previsto` `non applicabile` `ninguno` `ninguna` `no aplica` `no hay` `no` `null` | Blank, whole | 70 / 430 | 422 / 78 / 0 | — |
| `ikke relevant` `ikke aktuelt` `ej aktuellt` `nao aplicavel` `nao se aplica` `δεν εφαρμοζεται` `δεν απαιτειται` `nu este cazul` `nem relevans` `neni` `ei ole` | Blank, whole | 14 / 169 | 212 / 4 / 0 | — |
| `ingen` `inget` | Blank, whole, scoped da/sv/no, plain case | 515 / 789 | 348 / 461 / 0 | 1188404 `Ingen` FR, `InGen` ×2 → 2 |
| `nd` | Blank, whole, scoped pl/it | 3 / 257 | 311 / 2 / 0 | `ND` HR (OIB) → 2, 3 |
| 510's stems: `see section`, `voir autres informations`, `voir renseignements`, `ver informaci` (anchored); `perfil del contratante`, `perfil de contratante`, `would prejudice` (left word boundary) | Placeholder, stem | 190 / 457 | 74 / 354 / 0 | `SEE LAUER`, `See & Go` (510); a following name → keep rule |
| `withheld` | Placeholder, stem | 5 / 140 | 1 / 363 / 0 | Section94's PPON → no guard, by design |
| `see contracts finder` | Placeholder, stem | 1 / 115 | 0 / 127 / 0 | — |
| `siehe oben` | Placeholder, stem | 401 / 1,234 | 1,424 / 4 / 2 | `siehe oben: NEB … mbH`, `siehe oben (Vergabekammer Südbayern)` → keep rule |
| `keine angabe` (also takes `keine Angaben`; moved from Blank) | Placeholder, stem | 119 / 954 | 1,095 / 247 / 21 | 23186509's VAT → read list (§8.2) |
| name missing: `k a` `ka` `tbc` `tba` `to be confirmed` `not provided` `not specified` `not available` `unknown` `not known` `nicht bekannt` `niet bekend` `non connu` `non noto` `desconocido` `non renseigne` `brak danych` (moved from Blank) | Placeholder, whole | 65 / 1,257 | 1,313 / 66 / 2 | `Unknown Architects` → 1 |
| `s o` (the DE hub is `s.o.` = *siehe oben*), `see below`, `please see below`, `see attached`, `see attached list`, `see list below`, `see additional information`, `see vi 3`, `siehe unten`, `siehe vergabeunterlagen` | Placeholder, whole | 26 / 1,581 | 1,780 / 78 / 1 | — |
| `confidential` `redacted` `commercially sensitive` `not published` `not disclosed` `non communique` | Placeholder, whole | 119 / 151 | 0 / 153 / 0 | `Confidential Waste Services` → 1 |
| `various` (510); `various suppliers`/`contractors`/`providers`/`companies`/`organisations`/`see below`; `multiple`, `multiple suppliers`/`providers`/`contractors`; `varios` (`vários`), `varias`, `varios adjudicatarios`/`licitadores`/`proveedores`/`contratistas`, `varias empresas`, `diversos`; `diverse`, `diverse firmen`/`bieter`/`ditte`, `diversi`, `divers`; `verschiedene`, `verschiedene firmen`, `mehrere`, `mehrere bieter`; `ditte varie`, `varie` | Placeholder, whole | 246 / 742 | 3 / 821 / 0 | `Diverse Care Services Ltd` (47 winner rows), `Diversey`, `VÁRIOS MUNDOS`, `Variosun GmbH`, `Multiplex` → 1 |

**Excluded: digit-only names** (review read). The raw-name arm takes no digit except zeros, so these stay names (42 orgs):
- `1` (15 orgs, including 176 internal-ojs winner rows on 1163029), `2` (7), `3` (3), `9` (2), and `4`, `5`, `7`, `8`, `12`, `123`, `2000` (1 each);
- `360` FR (6 orgs, five with distinct national ids), `365` NL (3 winner rows) and `24/7` GB (1 `Tenderer` row).

Among them are 13 buyer-slot rows on 4 orgs (§3).

**Taken but not in the census:**
- the zeros `00` (1 org), `000` (6) and `0000` (3), with 74 party rows (63 review/mediation, 9 winner);
- the no-alphanumeric names `..` (5), `...` (4) and `_` (4), whose roles were not read.

**Totals.** Admitted: 5,089 orgs, 36,236 mentions and 45,910 party rows (`Blank` 3,917 / 29,605 / 37,633; `Placeholder` 1,172 / 6,631 / 8,277). Kept by guards: 13 orgs and 187 mentions (guard 3: 10 / 51; guard 2: 3 / 136). The census net is 5,102 orgs and 36,423 mentions. Not admitted: 133 mentions (the trap keys and the deferred words).

**Deferred (not admitted, 60 mentions).**
- `nil` (3 orgs / 33): 16700736 is NIL SAS, with a SIRET, and the country-less `NIL` with 13 winner mentions may be NIL d.o.o.
- `ei` (2 / 15): in French, `EI` is a sole-trader legal form.
- `sin` (1 / 1): `SIN` FR.
- `vari` (2 / 11): `VARI` is a brand.

510's void wordings under 20 mentions stay `VoidLot` residuals; they are not this issue's.

**Text parser.** `plausible_name` (parse.rs:146-154) keeps `not_a_name(name).is_none()`, so it refuses every class, `AwardSummary` included. Its letter rule already refuses every digit-only name, which is stricter than the fold, as before. Two changes reach it, and only when a notice is reparsed:
- the new classes are refused;
- the anchor narrows 510's pointer stems. A text-era name with `see section`, `voir …` or `ver informaci` in the middle of the string is no longer refused. This is accepted: no reparse is part of this issue, and a name minted that way would show up in the next walk's `non_exact` list.

The scoped words are kept there, because the text parser passes no language.

### 5. Award summaries: keep until per-lot parsing exists

The ~500 orgs `names_an_award` keeps (job 2182's `exempted`) become `AwardSummary`, and the fold keeps them exactly as today: `AwardSummary` never enters the drop map (U2b). The keep rule's pointer-plus-name hits join them. They are not dropped as winners, for three reasons:
- **Each is a leaf.** There is one org per summary string, mostly with one mention, so it joins no unrelated Tenders, unlike the hubs above.
- **It is the only place the real winners' names reach the served Tender** (`Lot 1) Sarl Bremond. Lot 2) S.A. Les Rapides Varois …`). Dropping it turns a readable award into `selec-w` with no winner and no way back.
- **The right outcome needs separate work.** Per-lot winners need the legacy reader to split the string and key each part to a lot (`several_lots`' designators are a starting point), and that raises its own precision questions. File it as a new issue together with unit 2.

They stay listed in `void-names-refold`'s `exempted`. The placeholder walk lists its own `AwardSummary` exemptions (`summary_shape` and the keep rule) and never re-queues either set. The text parser keeps refusing them (510 §4's accepted asymmetry).

### 6. `NON_NAME_FOLDS`: unify

- **Delete** `NON_NAME_FOLDS` (role_census.rs:546-562). `buyer_equal` (:537-539) then refuses name equality when `crate::partyname::not_a_name(&buyer.name).is_some()`.
- **Coverage.** 14 of its 15 folds are in §4:
  - as `Blank`: `n a`, `na`, `none`, `not applicable`, `x`;
  - as `Placeholder`: `unknown`, `not known`, `not specified`, `not provided`, `confidential`, `withheld`, `not published`, `not disclosed`, `various`.
- **`nil` leaves.** It is a real company (NIL SAS), and the census found it only in winner and Tenderer slots, so no buyer-equal verdict moves.
- **Where the check stays.** Once the fold drops these parties, `buyer_winner_sections` (project.rs:4279) never sees them. `buyer_equal`'s only caller is `buyer_equal_winners` (:578), which is the fold path. There the check remains for the parties the fold keeps: a `Blank` with an identifier, an `AwardSummary`, and a scoped word in another language.
- **The census never calls `buyer_equal`.** Its `contractor-org-same-name` and `contractor-name` classes call `same_name` directly (:682-685) and never read `NON_NAME_FOLDS`, so they keep flagging `N/A` against `N/A` among kept parties, as they do today. Aligning them is not part of this issue.

One list, one fold, no drift.

### 7. Build (unit 2), on 510's machinery

**Sequencing.** U2a and U2b touch only `ingest`. U2c (`canonical.rs`) and U2d (`supervisor.rs`) edit the two files that hold 513 U2's uncommitted work, so they are built after 513 U2 is committed (CLAUDE.md: never sweep in a co-worker's hunks). There is one gate and one deploy for U2a–U2d.

**U2a: predicate** (`crates/ingest/src/partyname.rs`). Pure.
- Add `NotAName::{VoidLot, Placeholder, Blank, AwardSummary}`. A void stem that names an award returns `AwardSummary` (:290).
- Add `BLANK_STEMS`, `BLANK_WHOLE` and `SCOPED_WHOLE: &[(&str, &[&str])]`, plus the new `PLACEHOLDER_STEMS` / `PLACEHOLDER_WHOLE` entries (:134, :146), each stem marked anchored or not. ` not applicable` moves to `BLANK_STEMS`.
- Add the raw-name arm (no alphanumeric, or only zeros), the anchor, the pointer keep rule, `not_a_name_in`, `not_a_name`, `mentions_placeholder`, `summary_shape` and `plain_case`. Extract the inline Polish-forms test of `names_an_award` (:415) into `polish_company_form(padded)`, which both of them use.
- **Rewrite the assertions that flip.**
  - The 57 void-plus-award specimens of 510's two award tests now expect `AwardSummary`: 4 in `a_void_phrase_that_goes_on_to_name_an_award_is_a_placeholder` and 53 in `a_summary_that_names_an_award_beside_a_void_lot_is_a_placeholder` (the review estimated about 45). So does that test's Sublote-only assertion (:928-932). Rename both tests.
  - `Not applicable` moves out of `placeholders_are_their_own_class` (:953) into a Blank test.
  - `not_a_name("  ...  ")` (:1019) now returns `Blank`.
- **New tests.**
  - `every_measured_placeholder_spelling_has_its_class`: one prod specimen per entry. It includes `See Section VI.3) Additional information`, which also checks that `lettered_entries` stays quiet on `VI.3)`, and the review's `keine Angabe` specimens ('Keine Angabe - der Zuschlag ging an mehrere Bieter, die weiterhin im Wettbewerb stehen', 'keine Angabe nach § 39 (6) 2 VgV, Bieter Nummer 1/2/3', 'Keine Angabe gemäß § 61 Abs 4 / § 62 Abs. 3 BVergG 2018', 'keine Angabe aus Gründen des Wettbewerbs'), each `Placeholder`.
  - `punctuation_only_names_are_blank`: `.`, `-/-`, `..`, `_`, `0`, `000` are Blank; `""` and `"  "` are None; `1`, `360`, `24/7`, `2000` are None.
  - `scoped_words_need_their_language_and_a_plain_case`:
    - `Ingen` in DAN is Blank; in FRA or with no language it is None;
    - `InGen` in DAN is None;
    - `nd.` in POL and `ND` in ITA are Blank;
    - `ND` in HRV is None.
  - `stems_open_the_name`:
    - `Firma Handlowa Jan Kowalski – nie dotyczy` and `Siemens – see section VI.3` are None;
    - `Nie dotyczy - brak`, `Lot 3: not applicable` and `Withheld for Security Reasons` take their class;
    - `Disclosure would prejudice commercial interests` and `Véase perfil del contratante` stay `Placeholder`.
  - `a_pointer_followed_by_a_name_is_kept`:
    - `AwardSummary`: `siehe oben: NEB Betriebsgesellschaft mbH (die hiesige Angabe …)`, `siehe oben (Vergabekammer Südbayern)`, `see section Royal Courts of Justice`, `Siehe oben – VKS`;
    - `Placeholder`: `siehe oben (VI.4.1)`, `siehe oben unter Ziffer VI.4.1`, `See section VI.3 - Lot 2`, `See Section VI.3) Additional information`, `See Contracts Finder Notice for full supplier list`.
  - `a_stem_inside_a_summary_is_an_award_summary`:
    - `Lot 2: not applicable. Lot 1: ACME Ltd` and `Nie dotyczy (część 2); część 1: X sp. z o.o.` are `AwardSummary`;
    - `Lot 1: ACME Ltd; Lot 2: not applicable` is None: the anchor misses it, and it stays a name, as today.
  - Extend `real_names_beside_the_phrases_are_names` (:970) with `NIL`, `N/A s. r. o.`, `Bank of America, N.A.`, `X Corp`, `VARI, a.s.`, `EI`, `SIN`, `365`, `24/7`.
  - Extend `every_entry_is_folded_and_every_stem_padded` (:1034) to the new lists.
  - `the_former_non_name_folds_are_classified`: every former `NON_NAME_FOLDS` entry except `nil`.

**U2b: fold** (`crates/ingest/src/project.rs`, `project/role_census.rs`).
- `void_party_sections` (:4379) becomes `dropped_party_sections(..) -> BTreeMap<String, NotAName>`, covering outer and inner sections. It remains the one decider.
  - **The rule, exactly.** A party enters only if every non-empty name is `VoidLot`, `Placeholder` or `Blank`, or through 510's `VoidLot`-only majority rule (:4401-4405). A party with any `AwardSummary` name never enters. The class combines per §1.
  - It reads the notice language once.
  - Per party, it reads the identifier fields that `mentions_and_void` reads (:4913-4945) through the factored gate, and applies guard 3 with its echo refusal.
- `mentions_and_void` (:4855) skips every key in the map, and its second value (the retire keys) carries every class. The plan sweeps keep their shape.
- In `NoticeState::read`, the role filter (:4760-4762) and `read_results` (:4773) take the map.
- Add `RawLotResult.withheld_winner` beside `void_lot` (:5871).
  - Legacy (:6102-6107) and sdk-0.1 (:6226-6231): `VoidLot` sets `void_lot`, `Placeholder` sets `withheld_winner`, and `Blank` is skipped.
  - Legacy derivation (:6160-6172): a real winner or `withheld_winner` gives `selec-w`; then `void_lot` gives `clos-nw`; then the existing rule applies.
  - sdk-0.1 derivation (:6244-6251): a real winner or `withheld_winner` gives `selec-w`; `void_lot` gives `clos-nw`; otherwise the result is NULL unless `TenderResultCode` states a decision.
  - The eForms/FTS member skip (:6021-6028) covers every class.
- `buyer_side_mentions` (:7477): after the fold into `(buyers, signing)`, return `(vec![], vec![])` when `buyers` is empty (§3).
- Add `Report.dropped {void, placeholder, blank, placeholder_identified}` beside `mentions_retired` (:82). It counts the parties the decider dropped across planned notices, on **outer** sections only, since the map also holds the inner aliases (:4405-4406). `placeholder_identified` counts the `Placeholder` parties dropped while they carry a gated identifier. The report keeps a bounded sample of those (up to 20 per fold: notice, section, name, identifier) for §8.1. This is the daily fold's measure.
- In `role_census`, delete `NON_NAME_FOLDS`; `buyer_equal` calls the predicate (§6).
- Tests, on 510's harness (project.rs:13377 and `ingest/tests/project.rs`):
  - `the_decider_never_drops_an_award_summary`:
    - an `AwardSummary`-only party is kept, and so are `{AwardSummary, VoidLot}`, `{AwardSummary, Placeholder}` and `{real name, Blank}`;
    - `{Placeholder, VoidLot}` enters as `Placeholder`.
  - `a_withheld_winner_keeps_the_result_selected_with_no_winner` (legacy, dateless; once with `Withheld`, once with `keine Angabe`).
  - `a_blank_winner_is_judged_on_the_results_other_evidence`: legacy with a value, with no date, and with a date only; sdk-0.1 is NULL unless coded.
  - `a_placeholder_review_body_mints_nothing` (an eForms `nie dotyczy` mediation-body).
  - `a_blank_party_with_a_gated_identifier_is_kept`:
    - negative cases, each dropped: a `.` party whose identifier is `.` (11063's shape), `keine` with `KEINE…`, `nie dotyczy` with `NIEDOTYCZY`;
    - `0` with `38160540` is kept.
  - `a_placeholder_party_with_an_identifier_is_dropped`, which also checks the `placeholder_identified` count and sample.
  - `a_placeholder_buyer_leaves_no_buyer_tokens`: `GuardSide` tokens and `buyer_key`, with and without a second, real buyer. It includes the signatory case: a `.` buyer plus a distinct real `Contract-Signatory` gives empty tokens and `buyer_key` None.
  - `a_scoped_word_follows_the_notice_language`.
  - `an_eforms_placeholder_tenderer_keeps_the_publishers_decision`.
  - `a_standing_placeholder_mention_is_retired_by_a_refold` (`Refold::{Incremental, Full}`, on the 434 harness).
  - `the_dropped_report_counts_outer_sections` (a party with a nested alias counts once).
  - `buyer_equal_refuses_every_non_name`.

**U2c: store** (`crates/store/src/canonical.rs`). Build it after 513 U2 is committed.
- `retire_mentions` is class-blind and does not change.
- Add two plan readers on `tender_version_parties_org_role`, in `IN_CHUNK` chunks:
  - `party_role_counts_of_orgs(orgs)`: `SELECT organization_id, role, COUNT(*) … WHERE organization_id IN (…) GROUP BY organization_id, role`. This is a covering seek and the census's own plan-checked shape.
  - `buyer_notices_of_orgs(orgs)`: `SELECT DISTINCT mention_notice_id, tender_id … WHERE organization_id IN (…) AND role IN ('buyer','Procedure-Buyer')`. This is a seek, but non-covering: `mention_notice_id` needs a table lookup per row, which is cheap at ~290 rows.
- That index is in `DEFERRED_TENDER_INDEXES`, which a full rebuild drops (§8.2).
- Test both on a fixture with one buyer row and one mediation row.

**U2d: job** (`crates/app/src/supervisor.rs`). Build it after 513 U2 is committed.
- `Spec` is serialized, so nothing is renamed. Add `Spec::RefoldPlaceholderNames { dry_run, expect }` and the request kind `refold-placeholder-names`, routed beside `refold-void-names` with the same dry default and `expect` rule.
- **Register the kind everywhere 510's kind is registered:**
  - `STOPPABLE_KINDS`: the walk polls the stop flag, and without the entry a cancel answers 409;
  - `heavy_write_kind`: the drain re-queues tens of thousands of notices, and issue 281's WAL hazard needs the coverage refresh held back meanwhile;
  - the kind list in `cancelling_a_kind_with_no_checkpoint_is_refused_rather_than_promised`;
  - in `docs/operations.md`: `placeholder-names-refold` in the report-kind inventory, and a runbook section beside the `refold-void-names` one (dry, read, canary, wet + fold, sweep).
- Both arms call one `run_refold_non_names(job, class, dry_run, expect)`, generalised from `run_refold_void_names`. The class chooses:
  - the net: `mentions_void` or `mentions_placeholder`;
  - the split: `is_void_lot`; or, for placeholders, `AwardSummary` goes to `exempted` and everything else is re-queued, so the fold decides the anchor, the scope and the identifier;
  - the report: `void-names-refold` or `placeholder-names-refold`.
- The placeholder plan adds:
  - `by_class`;
  - `identified`: `Placeholder` orgs with an identifier, with the identifier values;
  - `identified_blank`: with the identifier values and whether guard 3's echo test refuses each;
  - `scoped`: the `SCOPED_WHOLE` hits with country and mentions, split by the org's country as a proxy for the notice language:
    - `in_scope`: DK, SE, NO or FI for `ingen`/`inget` (a FI "Ingen" is Swedish); PL or IT for `nd`;
    - `out_of_scope`: another country, or not plain case; these are expected to be kept;
    - `unknown`: no country;
  - `non_exact`: every distinct hit whose fold is not exactly a lexicon entry, with its class, roles and mentions. It covers stem hits, mid-string hits the anchor refuses, and keep-rule and `summary_shape` names;
  - `roles`, from U2c;
  - `buyer_notices`, each with its Tender and its `procedure_key`, read by the projection's own `Ident`, as 482's census does.
- 510's parts are reused unchanged: the wet path, the `expect` gate, the chunked `unmark_projected_by_ids` + `stamp_stale_for_notices`, and the paired `project rebuild=false`.
- The new arm goes through `off_frame` and `Box::pin`, per the CLAUDE.md stack note.
- `sweep_after_fold` does not change.
- Tests: re-run 510's routing and dry/wet tests for the new kind, and check that `refold-void-names` still routes to the void class.

**Gate and deploy.** Gate through `ops/check.sh` only: supervisor.rs needs `--features server`, and a bare `cargo check -p tender-db` does not compile it. One deploy for U2a–U2d.

### 8. Drain (unit 3)

The census is exact-key only, so 510's ×1.3 undercount factor does not carry over. The review's prefix reads replace it: the known gaps alone add about 1,370 orgs.

1. **Deploy.** From then on the daily fold mints no placeholder party, and `Report.dropped` shows the daily rate. A full-fallback fold would retire corpus-wide; that is correct, and its `mentions_retired` will be large.
   - **Read the identified placeholders.** Between deploy and the wet run, read the `dropped.placeholder_identified` sample in every daily fold. For each one, look up the organization its identifier keys (`organizations_identity`) and read its name. **Stop if any is a real supplier**: that shape needs an identifier guard on `Placeholder` before the wet run.
   - **Expect a small dip** in `data_quality`'s winner-chain rate (`with_winner / with_awardable`). `Placeholder` results become `selec-w` with no winner and dated `Blank` results become NULL, and both stay "awardable". These are the withheld and blank results the publisher left without a name, not a regression. No alarm fires, because `presence_step_changes` watches factless versions only.
2. **`refold-placeholder-names`, dry.** Run it straight after deploy. Otherwise add the intervening folds' `mentions_retired` back to the counts, because those folds retire placeholder mentions and their auto sweeps take the one-mention tail. Do not run it during a full rebuild, which drops U2c's index.
   - Expected, counting re-queued hits (the net minus `exempted`):
     - **orgs at least about 6,475.** That is the census net of 5,102 (5,089 admitted + 10 identified Blank + 3 kept by scope or case), plus about 1,350 prefix hits outside the census that the revised predicate drops (review read: `see section` 1,128, `siehe oben` 56, `keine angabe` 56, `withheld` 50, `not applicable` 36, `niet van toepassing` 15, `nie dotyczy` 8, `see contracts finder` 1), plus 23 unprobed no-alphanumeric and zero names. Spellings nobody enumerated add more.
     - **mentions at least about 37,800**: 36,423, plus at least one per added org.
     - `exempted`: about 12 from the prefix reads (the keep rule's 9 and `summary_shape`'s 3), and more from unread spellings.
     - buyer rows: about 286 on about 14 orgs, 241 of them on 11063.
     - `identified`: at least 30914328 and 31548264 (the two Section94 PPONs), 23424157, 23186509, 17888810, 17660142, 17855508 and 22763666.
     - `identified_blank`: guard 3's 10 orgs, with their identifiers.
     - `scoped`:
       - `out_of_scope`: 1188404 FR (134), 14319858 HR (8; also identified) and `InGen` ×2;
       - `in_scope`: 7 orgs / 534 mentions (1889418 DK 274, 6425603 PL 226, 9119141 IT 29, …);
       - `unknown`: about 511 country-less orgs with one mention each.
     - top orgs: 1215973, 1213676, 1257377, 1765331, 1321632, 2147121, 8631351.
   - **Stop if** orgs fall below 6,450 or mentions below about 37,000, after the add-back: the predicate or the walk regressed.
   - **Read before the wet run (mandatory):**
     - every distinct `non_exact` fold, all of them. The absolute floor, if the list ever proves unreadable, is 200 random ones plus every one with winner, Tenderer or buyer rows, and reading less than all is recorded with its reason;
     - every `identified` and `identified_blank` org with its identifier values, including 23186509's VAT;
     - every `legal_form` org.

     **Stop and add a keep test** if any of them reads as a real party.
   - **Stop if** buyer rows exceed 1,000: the net reached real buyers.
3. **Buyer canary.**
   - **Record the pre-image.** These are bounded reads. `tender_versions_notice` and `tenders_procedure_key` are both deferred indexes, so these reads too must not run during a full rebuild.
     - For the plan's `buyer_notices`: `SELECT caused_by_notice_id, tender_id FROM tender_versions WHERE caused_by_notice_id IN (…)` (≤ 300 ids; `tender_versions_notice`).
     - For each distinct `procedure_key` of those notices, from the plan, read the key family:
       - its Tenders: `SELECT id, procedure_key FROM tenders WHERE procedure_key = ?`, plus a range seek on `refused_key_range(key)`;
       - their notices: `SELECT tender_id, caused_by_notice_id FROM tender_versions WHERE tender_id IN (…)` (primary key).
   - Run `refold-notices` on the buyer notices (cap 1,000), let its fold run, then re-read the same rows.
   - **Stop if** more than 10 % of the buyer notices changed Tender, or any key family changed beyond them. Read each change before the main run: a moved notice is a cited link the guard now admits as unknown, and a regrouped family is a key that 369's gate now admits. Decide whether placeholder buyers need an exception.
4. **Wet.** Run `refold-placeholder-names expect=<dry notices>` (it refuses above +25 %), then its paired fold.
   - **`mentions_retired` floor:** at least the dry re-queued mentions, minus the `identified_blank` mentions that carry a gated identifier, minus the `scoped.out_of_scope` and `scoped.unknown` mentions. A country-less tail org on a non-Scandinavian notice is kept.
   - **Above the dry mentions is a read trigger, not a stop.** Touched-expansion notices legitimately add retirements, and so do identifier-bound placeholder mentions on real-headed orgs, which the org walk cannot see. Read the excess, starting with `dropped.placeholder_identified`, before the sweep.
   - `mentions_rebound`: about 0.
   - `dropped.void + dropped.placeholder + dropped.blank` ≥ `mentions_retired`.
   - `retire_corrections` > 0 (ADR-0017 D5).
   - Duration: roughly 30–50 minutes (510: about 1 minute per 1,000 notices).
   - **Stop if** `mentions_retired` falls below the floor, and read the gap before the sweep.
5. **Sweep.** The fold queues `sweep-orphan-orgs` (auto) because it retired mentions.
   - Expect `counted` to be about the dry orgs, minus the orgs a guard keeps, minus the orgs the intervening sweeps already took.
   - Under `AUTO_SWEEP_CAP` (10,000) the job sweeps on its own count, as designed.
   - Above the cap, the plan is only recorded: read `protected`, then run wet (510's pattern).
6. **Verify**, with bounded `/v1/sql` reads and the API.
   - These all read 0:
     - `SELECT COUNT(*) FROM organization_mentions WHERE organization_id IN (1215973, 1213676, 1257377, 2147121, 8631351, 1765331, 1191345, 1321632, 1350611, 1528718, 1166840, 11063, 30914328, 30918120, 1173051, 3992287)` (`organization_mentions_org`);
     - the same id set on `tender_version_parties` (`_org_role`);
     - `tender_version_result_winners WHERE organization_id IN (30914328, 1173051, 1166840, 30918120)` (`_org`).
   - **1889418 `Ingen` DK (274 mentions): any residual must come from non-DAN/SWE/NOR notices.** Read the language of the notices behind every remaining mention. A remaining mention on a Danish, Swedish or Norwegian notice is a miss.
   - Before step 4, capture up to 20 Tender ids each for 30914328 and 1166840 (`SELECT DISTINCT tender_id FROM tender_version_parties WHERE organization_id = ? LIMIT 20`). Afterwards, Section94's Tenders keep their published result status with no winner, and néant's Tenders read according to the evidence rule.
   - **The identified-Blank orgs, per mention.** Read `SELECT notice_id, section_id, raw_identifier FROM organization_mentions WHERE organization_id IN (<guard 3's 10>)`. The mentions that carry a gated identifier remain; the ones without it are retired.
   - 1188404 keeps its 134 mentions if its notices are French (the scope held), or drops to 0 if they are Scandinavian. Record which.
   - `name_prefix=` `nie%20dotyczy`, `N%2FA`, `Withheld`, `Various` and `Keine%20Angabe` return real names only (such as `N/A s. r. o.`).
7. **Follow-up measurement** (file it with unit 2). For each "nobody" word (`ingen`, `néant`, `geen`, `aucun`, `niet van toepassing`), build 510's per-word value/date table over its legacy/sdk-0.1 winner-slot results. No word is promoted to `VoidLot` without that table.

### 9. Risks and pins

| risk | pin |
|---|---|
| A real company is dropped | Whole values only for single words; anchored stems; the pointer keep rule; three scoped words; the real-name test; the dry plan's mandatory `non_exact` read and its `identified` / `legal_form` stop rule |
| A real party with a missing name is dropped | Guard 3 (Blank + a gated identifier that does not echo a non-name) and its test |
| A real supplier's identifier sits on a Placeholder name | `Report.dropped.placeholder_identified` and its sample, read with a stop rule before the wet run; 23186509 on the read list |
| A withheld winner is read as no award | `withheld_winner` → `selec-w`; `keine angabe` and the name-missing words are `Placeholder`; the test |
| "Nobody" becomes a positive `clos-nw` claim | `Blank` carries no marker, and sdk-0.1 stays NULL unless coded; promotion needs the per-word table (§8.7); the test |
| A summary's real winners are lost | `AwardSummary` never enters the drop map; `summary_shape` guards every non-void stem; the decider test |
| A buyer drop moves Tender grouping | The canary on the buyer notices and their procedure-key families, with its stop rule; the tokens test, signatory case included |
| `mentions()` and `read()` disagree | Both read `dropped_party_sections` |
| The job runs half-registered | `STOPPABLE_KINDS`, `heavy_write_kind`, the checkpointed-kinds test, the operations.md inventory and runbook |
| The new arm overflows `run_spec` | `Box::pin`; `ops/check.sh` |

### Review (2026-10-11)

Two adversarial reviews (workflow `wf_9b2cb650`) both returned "sound with fixes": `code` with 14 findings, `data` with 8. The owner's rulings on them changed:
- **The decider** (code 0): `AwardSummary` never enters the drop map, and any `AwardSummary` name keeps the party (§1, U2b and its test).
- **Buyer side** (code 1, 10): an empty folded buyer list returns no signatories either (481 2c). The canary also reads the buyer notices' procedure-key families.
- **The job** (code 2, 11, 12): registered in `STOPPABLE_KINDS`, `heavy_write_kind`, the checkpointed-kinds test and operations.md. The second reader is a non-covering seek, and no step runs during a full rebuild. U2c and U2d wait for 513 U2's commit, and sites are found by symbol.
- **Identifiers** (code 3, data 3):
  - `Report.dropped.placeholder_identified`, with a sample read and a stop rule after deploy;
  - the wet run's upper bound is a read trigger, not a stop;
  - identified-Blank orgs are verified per mention;
  - guard 3 refuses an identifier that echoes a non-name.
- **Stems** (code 4, data 1): anchored at the start, with a pointer keep rule. The NEB, Vergabekammer Südbayern and Royal Courts specimens have keep tests. Every non-exact hit is read before the wet run.
- **Classes** (data 0): classed by what each word means in a winner slot. `keine angabe`, `tbc`, `tba`, `to be confirmed`, `not provided` and 12 more name-missing words move to `Placeholder`, and 23186509's VAT goes on the read list. The "nobody" words stay `Blank` until a per-word table exists.
- **Smaller fixes:**
  - the raw-name arm takes no alphanumeric or only zeros, and 42 digit-only names are excluded (code 5, data 4);
  - the §6 wording is corrected (code 6);
  - U2a's test rewrites and `polish_company_form` are listed (code 7);
  - Verify: the 1889418 residual check, the dry run straight after deploy or with an add-back, outer sections only, and a three-class inequality (code 8);
  - sdk-0.1 `Blank` reads NULL unless coded (code 9);
  - the `data_quality` dip is noted (code 13);
  - `nd` is scoped to {POL, ITA} (data 5);
  - `scoped` uses the org's country as a proxy (data 6);
  - the dry floor rises to about 6,450 orgs, and the size-triggered sample becomes a required read (data 2);
  - the plan-width note is in Basis (data 7).
- **Net effect on the figures.** The census net is unchanged (5,102 orgs / 36,423 mentions). 183 orgs and 2,209 mentions move from `Blank` to `Placeholder`. 23186509 moves from guard 3 to admitted, so admitted is now 5,089 / 36,236 / 45,910, and guard 3 keeps 10 / 51.
- **Choices beyond the rulings' letter:**
  - `would prejudice` stays unanchored, beside the `perfil` family; anchoring would retire it and flip a 510 assertion;
  - the keep rule also strips a lot designator, accepts en and em dashes, and lets a `see section` with no section number go without a separator (Royal Courts);
  - guard 3's echo test compares space-free folds and skips names with no letter (otherwise `0`'s CVR would be refused);
  - the `scoped` proxy counts FI as Swedish-language;
  - the anchor narrows the text parser's 510 pointer refusals on reparse.
