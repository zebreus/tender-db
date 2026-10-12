# 525 build notes: void templates, the nested retire key, and the walk contract (2026-10-12)

Built against `.scratch/tender-db/issues/525-ted-multilingual-not-awarded-templates-survive-the-void-fold.md` and the
verified diagnosis `../520-award-summaries/void-residue-diagnosis-wf_8a719bd1-4b6.json`. All four parts are in the
working tree, **not committed**. The tests were written first and seen RED before each fix (runs below). Nothing on prod
was touched beyond the paced public `/content` GETs listed under "Fixtures".

## What changed

### 1. Fold lexicon (classes A and C): `crates/ingest/src/partyname.rs`

- `VOID_WHOLE` grows from 50 to 148 entries. The 98 new ones are listed in the table below.
- Every entry is a whole value. No stem was added, so a company whose name opens with a template and goes on with a
  capitalised word or a legal form stays a name (`Nicht Vergeben GmbH`, `Lote No Adjudicado SL`).
- Each entry is a copy of a party the fold KEPT, collected from the notice's public `/content` and folded by the module's
  own `fold`. The `every_entry_is_folded_and_every_stem_padded` test checks each entry is folded and in one list only.
- The 18344075 template prints "not applicable — not awarded". Its copies are added as whole compounds, because the
  "not applicable" opener alone is no void phrase and `reason_head` cuts at the dash.
- `no award was made` is added whole, so case no longer decides 3322691's `NO AWARD WAS MADE`. `void_prefix` stays
  case-sensitive, as the diagnosis advised (`DESIERTO OÜ`).
- Doc updates:
  - The module's "Matching" paragraph now says TED's own templates are carried in every copy.
  - `is_void_lot`'s doc now says it is the rule for ONE name. Since this unit it decides only the walk's orgs that have
    no mention.

### 2. Fold retire keys (class B): `crates/ingest/src/project.rs`

- In `NoticeState::mentions_and_dropped`, the retire list (`skipped`) is now built from every dropped party section,
  nested inner halves included: the `!alias.contains_key` term is gone from that filter only.
- Minting still skips both halves, and `DroppedParties` still counts outer sections (`decision.outer`).
- Docs updated: `mentions_and_void`, and `DroppedParties` ("at most one retired mention per key handed over").

### 3. Walk contract: `ingest::project`, `store`, `crates/app/src/supervisor.rs`

New in `ingest::project`:

- `pub fn fold_parties(profile, &Parsed) -> FoldParties`. It calls the fold's own `drop_decision` with the fold's inputs:
  - `is_sdk01_profile(profile)`, the same flag `Ident::read` sets;
  - the section map;
  - `nested_org_aliases`.
- `FoldParties::verdict(section) -> PartyVerdict`, which is one of:
  - `Dropped(class)`: the section is in `decision.sections`, outer or inner. Since part 2 these are exactly the keys the
    fold retires; the unit test below proves it against `mentions_and_dropped`.
  - `Kept(KeptParty { void, names, summary, identified })`: the fold keeps the party.
  - `Unknown`: there is no party section by that id in the parse.
- `KeptParty::reason()` gives the exemption text:
  - `party kept: 6/23 names void`;
  - with `, an award summary among them` or `, a gated identifier` appended when either kept the party;
  - `party kept: no name` for a party with no name.
- `drop_decision` gained an `explain: bool`. Only `fold_parties` passes `true`. The fold's own two calls pass `false`, so
  the hot loop allocates nothing new; it only computes the void count once before the summary check.

New in the store (`crates/store/src/canonical.rs`):

- `Db::mention_sections_of_orgs(orgs) -> Vec<(org, notice, section)>`. It is `mention_notices_of_orgs` plus
  `section_id`, through `organization_mentions_org` in IN chunks, sorted.

Changes in the supervisor (`run_refold_non_names`, void class):

- The window walk now only collects the net's hits (`mentions_void` on the head name, unchanged).
- `Supervisor::void_split` then:
  - reads every hit's mentions with their sections;
  - reads the mentioned notices' parses through `parsed_by_ids`, in chunks of `VOID_PARSE_CHUNK = 200`;
  - stops between chunks if the job is cancelled; nothing is written before the split;
  - asks `fold_parties` about each mention;
  - hands the answers to the pure `split_void_hits`.
- `split_void_hits` decides each hit:
  - A mention is re-queued on `Dropped(_)` of any class (the retire is class-blind) or on `Unknown`, which covers a
    missing parse, an unreadable one, and a section that is not a party.
  - A mention is exempted on `Kept`, with the fold's reason. Distinct reasons are joined with `; `.
  - An org with re-queued mentions goes to `orgs`. The pairs are the re-queued `(org, notice)`.
  - An org with kept mentions goes to `exempted`. Its fields are `mentions` (the kept ones), `reason`, and `notices`:
    up to `KEPT_NOTICES = 3` of the notices where the fold kept it.
  - An org with both kinds of mention is in both lists.
  - A hit mentioned nowhere keeps 510's head rule: `is_void_lot` means counted (with 0 mentions, for the orphan sweep).
    Otherwise it is exempted with `names an award`.
- The job line now reads `… N more exempted: the fold keeps their party or the name names an award), M mention(s) to
  retire on K notice(s) …`.
- The placeholder class is unchanged: it still splits by the head name (see Deviations).
- The `Spec::RefoldVoidNames` doc, the `run_refold_non_names` doc, the `NonNames::net` doc (the widening note) and the
  `NonNames::requeues` doc are rewritten.
- The new future is `Box::pin`ned inside the off-frame family fn. `run_spec_futures_stay_inside_their_size_budgets` and
  `an_execute_without_an_expected_count_is_refused` are green.

### 4. Docs: `docs/operations.md`

- The 510 section's intro names TED's templates.
- The walk paragraph no longer claims "the fold's own predicate". It now describes:
  - the net as a name test;
  - the split as the fold's party rule (all copies are non-names, or at least 4 copies with half void);
  - re-queue on drop or unreadable;
  - `exempted` with its reasons and up to 3 notices;
  - 510's head rule for orgs mentioned nowhere;
  - convergence (2351 → 2354 is the failure case);
  - the head-only net, as a note.
- The 511 section says the placeholder walk keeps its head split.

## Every added wording (98)

Copies verbatim from public `/content` (2026-10-12), in first-seen order. The fold is the entry as stored.

Lines 1–17 are 16893864 ORG-2..5 (orgs 1920418–21). Lines 18–27 are 16891918 ORG-2 (org 1914495). Lines 28–35 are
16967678 ORG-4 (org 2184993). Line 36 is the LV copy of 16967678 ORG-2. Lines 37–55 are 12563312 ORG-23 (org 1579624).
Lines 56–76 are 12466332 ORG-2 (org 1229591). Lines 77–96 are 18344075 ORG-4..7 (org 6709213). Line 97 is
12501289 ORG-2/3/7/8/11 and 4416766 ORG-3/4 (orgs 1354108/10/11/15/16, 31408169/70). Line 98 is 3322691 ORG-2
(org 27220329).

Copies a template shares with an earlier one are listed once, at their first notice. For example, `Appalto non
aggiudicato` in 16891918 and `Zakázka nebyla zadána` in 16967678 ORG-4 are listed under 16893864.

| # | language | published copy | whole value (fold) | source notice / section | template |
|---:|---|---|---|---|---|
| 1 | DA | `Kontrakten ikke tildelt.` | `kontrakten ikke tildelt` | 16893864 ORG-2 | `Contract not awarded.` / `Marché non attribué.` (ORG-2..5) |
| 2 | DE | `Der Auftrag wurde nicht vergeben.` | `der auftrag wurde nicht vergeben` | 16893864 ORG-2 | `Contract not awarded.` / `Marché non attribué.` (ORG-2..5) |
| 3 | ES | `El contrato no ha sido adjudicado.` | `el contrato no ha sido adjudicado` | 16893864 ORG-2 | `Contract not awarded.` / `Marché non attribué.` (ORG-2..5) |
| 4 | EL | `Η σύμβαση δεν κατακυρώθηκε.` | `η συμβαση δεν κατακυρωθηκε` | 16893864 ORG-2 | `Contract not awarded.` / `Marché non attribué.` (ORG-2..5) |
| 5 | IT | `Appalto non aggiudicato.` | `appalto non aggiudicato` | 16893864 ORG-2 | `Contract not awarded.` / `Marché non attribué.` (ORG-2..5) |
| 6 | PT | `Contrato não adjudicado.` | `contrato nao adjudicado` | 16893864 ORG-2 | `Contract not awarded.` / `Marché non attribué.` (ORG-2..5) |
| 7 | SV | `Kontraktet har inte tilldelats.` | `kontraktet har inte tilldelats` | 16893864 ORG-2 | `Contract not awarded.` / `Marché non attribué.` (ORG-2..5) |
| 8 | CS | `Zakázka nebyla zadána.` | `zakazka nebyla zadana` | 16893864 ORG-2 | `Contract not awarded.` / `Marché non attribué.` (ORG-2..5) |
| 9 | HU | `A szerződést nem ítélték oda.` | `a szerzodest nem iteltek oda` | 16893864 ORG-2 | `Contract not awarded.` / `Marché non attribué.` (ORG-2..5) |
| 10 | LT | `Sutartis nepaskirta.` | `sutartis nepaskirta` | 16893864 ORG-2 | `Contract not awarded.` / `Marché non attribué.` (ORG-2..5) |
| 11 | LV | `Līgums nav piešķirts.` | `ligums nav pieskirts` | 16893864 ORG-2 | `Contract not awarded.` / `Marché non attribué.` (ORG-2..5) |
| 12 | MT | `Il-kuntratt ma ngħatax.` | `il kuntratt ma ngħatax` | 16893864 ORG-2 | `Contract not awarded.` / `Marché non attribué.` (ORG-2..5) |
| 13 | SK | `Obstarávanie nebolo pridelené.` | `obstaravanie nebolo pridelene` | 16893864 ORG-2 | `Contract not awarded.` / `Marché non attribué.` (ORG-2..5) |
| 14 | SL | `Pogodba ni dodeljena.` | `pogodba ni dodeljena` | 16893864 ORG-2 | `Contract not awarded.` / `Marché non attribué.` (ORG-2..5) |
| 15 | GA | `Níor dámhadh an Conradh.` | `nior damhadh an conradh` | 16893864 ORG-2 | `Contract not awarded.` / `Marché non attribué.` (ORG-2..5) |
| 16 | BG | `Поръчката не беше възложена.` | `поръчката не беше възложена` | 16893864 ORG-2 | `Contract not awarded.` / `Marché non attribué.` (ORG-2..5) |
| 17 | RO | `Contract neatribuit.` | `contract neatribuit` | 16893864 ORG-2 | `Contract not awarded.` / `Marché non attribué.` (ORG-2..5) |
| 18 | DA | `Kontrakten blev ikke tildelt.` | `kontrakten blev ikke tildelt` | 16891918 ORG-2 | `Contract not awarded` (later wording) |
| 19 | DE | `Es wurde kein Auftrag vergeben` | `es wurde kein auftrag vergeben` | 16891918 ORG-2 | `Contract not awarded` (later wording) |
| 20 | ES | `No se ha adjudicado ningún contrato` | `no se ha adjudicado ningun contrato` | 16891918 ORG-2 | `Contract not awarded` (later wording) |
| 21 | FR | `Le marché n'a pas été attribué` | `le marche n a pas ete attribue` | 16891918 ORG-2 | `Contract not awarded` (later wording) |
| 22 | EL | `Η σύμβαση δεν ανατέθηκε` | `η συμβαση δεν ανατεθηκε` | 16891918 ORG-2 | `Contract not awarded` (later wording) |
| 23 | SV | `Inget kontrakt har tilldelats` | `inget kontrakt har tilldelats` | 16891918 ORG-2 | `Contract not awarded` (later wording) |
| 24 | LV | `Līguma slēgšanas tiesības nav piešķirtas` | `liguma slegsanas tiesibas nav pieskirtas` | 16891918 ORG-2 | `Contract not awarded` (later wording) |
| 25 | PL | `Zamówienie nie zostało udzielone` | `zamowienie nie zostalo udzielone` | 16891918 ORG-2 | `Contract not awarded` (later wording) |
| 26 | SK | `Zmluva nebola udelená` | `zmluva nebola udelena` | 16891918 ORG-2 | `Contract not awarded` (later wording) |
| 27 | BG | `Поръчката не бе възложена` | `поръчката не бе възложена` | 16891918 ORG-2 | `Contract not awarded` (later wording) |
| 28 | DA | `Ingen kontrakt tildelt.` | `ingen kontrakt tildelt` | 16967678 ORG-4 | `No contract awarded.` |
| 29 | FR | `Aucun marché n'a été attribué.` | `aucun marche n a ete attribue` | 16967678 ORG-4 | `No contract awarded.` |
| 30 | IT | `Non è stato aggiudicato alcun contratto.` | `non e stato aggiudicato alcun contratto` | 16967678 ORG-4 | `No contract awarded.` |
| 31 | NL | `Er is geen opdracht gegund.` | `er is geen opdracht gegund` | 16967678 ORG-4 | `No contract awarded.` |
| 32 | PT | `Não foi adjudicado nenhum contrato.` | `nao foi adjudicado nenhum contrato` | 16967678 ORG-4 | `No contract awarded.` |
| 33 | HU | `Szerződés odaítélésére nem került sor.` | `szerzodes odaitelesere nem kerult sor` | 16967678 ORG-4 | `No contract awarded.` |
| 34 | GA | `Níor dámhadh aon chonradh.` | `nior damhadh aon chonradh` | 16967678 ORG-4 | `No contract awarded.` |
| 35 | BG | `Поръчката не е възложена.` | `поръчката не е възложена` | 16967678 ORG-4 | `No contract awarded.` |
| 36 | LV | `Nav piešķirta.` | `nav pieskirta` | 16967678 ORG-2 | 17674598's template, LV copy |
| 37 | DA | `Partiet blev ikke tildelt` | `partiet blev ikke tildelt` | 12563312 ORG-23 | `Lot not awarded` |
| 38 | DE | `Los wurde nicht vergeben` | `los wurde nicht vergeben` | 12563312 ORG-23 | `Lot not awarded` |
| 39 | ES | `Lote no adjudicado` | `lote no adjudicado` | 12563312 ORG-23 | `Lot not awarded` |
| 40 | FI | `Erästä ei tehty sopimusta` | `erasta ei tehty sopimusta` | 12563312 ORG-23 | `Lot not awarded` |
| 41 | EL | `Η παρτίδα δεν ανατέθηκε` | `η παρτιδα δεν ανατεθηκε` | 12563312 ORG-23 | `Lot not awarded` |
| 42 | PT | `Lote não adjudicado` | `lote nao adjudicado` | 12563312 ORG-23 | `Lot not awarded` |
| 43 | SV | `Delen har inte tilldelats` | `delen har inte tilldelats` | 12563312 ORG-23 | `Lot not awarded` |
| 44 | CS | `Položka nebyla zadána` | `polozka nebyla zadana` | 12563312 ORG-23 | `Lot not awarded` |
| 45 | ET | `Osa kohta lepingut ei sõlmitud` | `osa kohta lepingut ei solmitud` | 12563312 ORG-23 | `Lot not awarded` |
| 46 | HU | `A tétel nem került odaítélésre` | `a tetel nem kerult odaitelesre` | 12563312 ORG-23 | `Lot not awarded` |
| 47 | LT | `Dalis nepaskirta` | `dalis nepaskirta` | 12563312 ORG-23 | `Lot not awarded` |
| 48 | LV | `Daļa netika piešķirta` | `dala netika pieskirta` | 12563312 ORG-23 | `Lot not awarded` |
| 49 | MT | `Il-lott ma ngħatax` | `il lott ma ngħatax` | 12563312 ORG-23 | `Lot not awarded` |
| 50 | PL | `Na tę część nie udzielono zamówienia` | `na te czesc nie udzielono zamowienia` | 12563312 ORG-23 | `Lot not awarded` |
| 51 | SK | `Časť nebola pridelená` | `cast nebola pridelena` | 12563312 ORG-23 | `Lot not awarded` |
| 52 | SL | `Sklop ni bil oddan` | `sklop ni bil oddan` | 12563312 ORG-23 | `Lot not awarded` |
| 53 | GA | `Níor dámhadh an beart` | `nior damhadh an beart` | 12563312 ORG-23 | `Lot not awarded` |
| 54 | BG | `Партидата не е възложена` | `партидата не е възложена` | 12563312 ORG-23 | `Lot not awarded` |
| 55 | RO | `Lot neatribuit` | `lot neatribuit` | 12563312 ORG-23 | `Lot not awarded` |
| 56 | DA | `Dette parti er ikke blevet tildelt.` | `dette parti er ikke blevet tildelt` | 12466332 ORG-2 | `Ce lot est infructueux.` |
| 57 | DE | `Dieses Los blieb erfolglos.` | `dieses los blieb erfolglos` | 12466332 ORG-2 | `Ce lot est infructueux.` |
| 58 | EN | `This lot was unsuccessful.` | `this lot was unsuccessful` | 12466332 ORG-2 | `Ce lot est infructueux.` |
| 59 | FI | `Tämä erä oli tulokseton.` | `tama era oli tulokseton` | 12466332 ORG-2 | `Ce lot est infructueux.` |
| 60 | EL | `Η διαδικασία ανάθεσης της παρτίδας είναι ανεπιτυχής.` | `η διαδικασια αναθεσησ τησ παρτιδασ ειναι ανεπιτυχησ` | 12466332 ORG-2 | `Ce lot est infructueux.` |
| 61 | IT | `Questo lotto è risultato infruttuoso.` | `questo lotto e risultato infruttuoso` | 12466332 ORG-2 | `Ce lot est infructueux.` |
| 62 | NL | `Het perceel bleef zonder resultaat.` | `het perceel bleef zonder resultaat` | 12466332 ORG-2 | `Ce lot est infructueux.` |
| 63 | PT | `Este lote foi considerado infrutífero.` | `este lote foi considerado infrutifero` | 12466332 ORG-2 | `Ce lot est infructueux.` |
| 64 | SV | `Denna del har inte givit något resultat.` | `denna del har inte givit nagot resultat` | 12466332 ORG-2 | `Ce lot est infructueux.` |
| 65 | CS | `Tato položka byla neúspěšná.` | `tato polozka byla neuspesna` | 12466332 ORG-2 | `Ce lot est infructueux.` |
| 66 | ET | `Selle osa menetlus luhtus.` | `selle osa menetlus luhtus` | 12466332 ORG-2 | `Ce lot est infructueux.` |
| 67 | HU | `A tétel sikertelen volt.` | `a tetel sikertelen volt` | 12466332 ORG-2 | `Ce lot est infructueux.` |
| 68 | LT | `Ši dalis buvo nesėkminga.` | `si dalis buvo nesekminga` | 12466332 ORG-2 | `Ce lot est infructueux.` |
| 69 | LV | `Šī daļa bija neveiksmīga.` | `si dala bija neveiksmiga` | 12466332 ORG-2 | `Ce lot est infructueux.` |
| 70 | MT | `Dan il-lott kien mingħajr suċċess.` | `dan il lott kien mingħajr suċċess` | 12466332 ORG-2 | `Ce lot est infructueux.` |
| 71 | PL | `Zamówienie dla tej części uznano za nieskuteczne.` | `zamowienie dla tej czesci uznano za nieskuteczne` | 12466332 ORG-2 | `Ce lot est infructueux.` |
| 72 | SK | `Táto časť bola neúspešná.` | `tato cast bola neuspesna` | 12466332 ORG-2 | `Ce lot est infructueux.` |
| 73 | SL | `Ta sklop je bil neuspešen.` | `ta sklop je bil neuspesen` | 12466332 ORG-2 | `Ce lot est infructueux.` |
| 74 | GA | `Bhí an beart seo neamhrathúil.` | `bhi an beart seo neamhrathuil` | 12466332 ORG-2 | `Ce lot est infructueux.` |
| 75 | BG | `Тази партида беше неуспешна.` | `тази партида беше неуспешна` | 12466332 ORG-2 | `Ce lot est infructueux.` |
| 76 | RO | `Acest lot a fost nereuşit.` | `acest lot a fost nereusit` | 12466332 ORG-2 | `Ce lot est infructueux.` |
| 77 | DA | `finder ikke anvendelse — partiet blev ikke tildelt` | `finder ikke anvendelse partiet blev ikke tildelt` | 18344075 ORG-4 | `Non applicable — Lot non attribué` (ORG-4..7) |
| 78 | DE | `entfällt — Los nicht vergeben` | `entfallt los nicht vergeben` | 18344075 ORG-4 | `Non applicable — Lot non attribué` (ORG-4..7) |
| 79 | ES | `No procede: lote no adjudicado` | `no procede lote no adjudicado` | 18344075 ORG-4 | `Non applicable — Lot non attribué` (ORG-4..7) |
| 80 | FI | `ei sovelleta – erästä ei tehty sopimusta` | `ei sovelleta erasta ei tehty sopimusta` | 18344075 ORG-4 | `Non applicable — Lot non attribué` (ORG-4..7) |
| 81 | EL | `Άνευ αντικειμένου — Το τμήμα δεν ανατέθηκε` | `ανευ αντικειμενου το τμημα δεν ανατεθηκε` | 18344075 ORG-4 | `Non applicable — Lot non attribué` (ORG-4..7) |
| 82 | IT | `Non applicabile — lotto non aggiudicato` | `non applicabile lotto non aggiudicato` | 18344075 ORG-4 | `Non applicable — Lot non attribué` (ORG-4..7) |
| 83 | PT | `Não aplicável — Lote não adjudicado` | `nao aplicavel lote nao adjudicado` | 18344075 ORG-4 | `Non applicable — Lot non attribué` (ORG-4..7) |
| 84 | SV | `Ej tillämpligt – delen har inte tilldelats` | `ej tillampligt delen har inte tilldelats` | 18344075 ORG-4 | `Non applicable — Lot non attribué` (ORG-4..7) |
| 85 | CS | `nepoužije se – položka nebyla zadána` | `nepouzije se polozka nebyla zadana` | 18344075 ORG-4 | `Non applicable — Lot non attribué` (ORG-4..7) |
| 86 | ET | `Ei kohaldata – osa lepingut ei sõlmitud` | `ei kohaldata osa lepingut ei solmitud` | 18344075 ORG-4 | `Non applicable — Lot non attribué` (ORG-4..7) |
| 87 | HU | `nem alkalmazható – a tétel nem került odaítélésre` | `nem alkalmazhato a tetel nem kerult odaitelesre` | 18344075 ORG-4 | `Non applicable — Lot non attribué` (ORG-4..7) |
| 88 | LT | `netaikoma, nes dalis nebuvo paskirta.` | `netaikoma nes dalis nebuvo paskirta` | 18344075 ORG-4 | `Non applicable — Lot non attribué` (ORG-4..7) |
| 89 | LV | `Nav piemērojams – līguma slēgšanas tiesības par šo daļu nav piešķirtas` | `nav piemerojams liguma slegsanas tiesibas par so dalu nav pieskirtas` | 18344075 ORG-4 | `Non applicable — Lot non attribué` (ORG-4..7) |
| 90 | MT | `mhux applikabbli — il-lott ma ngħatax` | `mhux applikabbli il lott ma ngħatax` | 18344075 ORG-4 | `Non applicable — Lot non attribué` (ORG-4..7) |
| 91 | SK | `neuplatniteľné – časť nebola pridelená` | `neuplatnitelne cast nebola pridelena` | 18344075 ORG-4 | `Non applicable — Lot non attribué` (ORG-4..7) |
| 92 | SL | `Ni navedeno – sklop ni bil oddan` | `ni navedeno sklop ni bil oddan` | 18344075 ORG-4 | `Non applicable — Lot non attribué` (ORG-4..7) |
| 93 | GA | `neamhbhainteach — níor dámhadh beart` | `neamhbhainteach nior damhadh beart` | 18344075 ORG-4 | `Non applicable — Lot non attribué` (ORG-4..7) |
| 94 | BG | `не е приложимо – партидата не е възложена` | `не е приложимо партидата не е възложена` | 18344075 ORG-4 | `Non applicable — Lot non attribué` (ORG-4..7) |
| 95 | RO | `neaplicabil – lot neatribuit` | `neaplicabil lot neatribuit` | 18344075 ORG-4 | `Non applicable — Lot non attribué` (ORG-4..7) |
| 96 | HR | `nije primjenjivo — grupa nije dodijeljena` | `nije primjenjivo grupa nije dodijeljena` | 18344075 ORG-4 | `Non applicable — Lot non attribué` (ORG-4..7) |
| 97 | DE | `Zuschlag nicht vergeben` | `zuschlag nicht vergeben` | 12501289 ORG-2 | South Tyrol IT/DE pair (also 4416766) |
| 98 | EN (no label) | `NO AWARD WAS MADE` | `no award was made` | 3322691 ORG-2 | text era all-caps |


The fixture parties' remaining copies were already classed before this unit:

- 17674598's list (`sopimusta ei tehty`, `lepingut ei solmitud`, `zamowienia nie udzielono`, `narocilo ni bilo oddano`, …);
- 510's stems (`non attribu`, `niet gegun`, `infructu`);
- `Blank` copies (EN `not applicable — lot not awarded`, PL `nie dotyczy — …`).

## Fixtures (public `/content`, paced 0.5 s, no retries)

- **200:** 16893864, 12501289, 4416766, 16967678, 12563312, 16891918, 12466332, 18344075, 3322691, 23151933, 17674598.
- **404, not retried:** 7400280, 6023876, 7969717. These are organization ids, not notices: the diagnosis lists them
  among the orgs, and diagnosis 1's "6709213's 4 mentions on 18344075, 7400280, …" misread them.

A temporary probe (since deleted, with its test binary) built `Parsed` from these 11 `/content` documents and ran
`fold_parties`, using the gate flags and package set (`--test probe_525`, EXIT=0):

- **Now dropped (every located party):**
  - 12501289 ORG-2/3/7/8/11;
  - 4416766 ORG-3/4;
  - 16893864 ORG-2..5;
  - 16967678 ORG-2 and ORG-4;
  - 12563312 ORG-23;
  - 16891918 ORG-2;
  - 12466332 ORG-2;
  - 18344075 ORG-4..7;
  - 23151933 ORG-2 and ORG-3 (nested);
  - 3322691 ORG-2.
- **Still kept, all with 0 void copies:** every real party (Fater SpA, SILC SpA, Deloitte, Mercedes-Benz Leudelange,
  Demathieu & Bard, Soprema, Rowastores, the EU bodies, review bodies). Nothing real was dropped.

## Tests

### Written first, seen RED, now GREEN

`crates/ingest/src/partyname.rs`:

- `teds_other_not_awarded_templates_are_void`: one specimen per template and language family, `NO AWARD WAS MADE`, a
  whole value followed by its reason, and the 510 net (`mentions_void`) holding each.
- `real_names_beside_teds_templates_are_names` (GREEN before and after, a guard):
  - `Nicht Vergeben GmbH`, `Zuschlag Nicht Vergeben GmbH`, `NO AWARD WAS MADE LTD`;
  - `Award Ltd`, `Awarded Contracts Ltd`, `LOT Polish Airlines`, `Polskie Linie Lotnicze LOT S.A.`;
  - `Lote Uno SL`, `Lote No Adjudicado SL`, `Contract Neatribuit SRL`, `Sklop d.o.o.`, `Partiet Ingeniører ApS`,
    `Dalis UAB`, `Unsuccessful Ventures Ltd`.

`crates/ingest/src/project.rs`, with fixtures `TED_MARCHE_NON_ATTRIBUE`, `TED_CONTRACT_NOT_AWARDED`,
`TED_NO_CONTRACT_AWARDED`, `TED_NOT_AWARDED`, `TED_LOT_NOT_AWARDED`, `TED_LOT_UNSUCCESSFUL` and
`TED_NOT_APPLICABLE_LOT_NOT_AWARDED` (verbatim, in published order):

- `teds_other_not_awarded_templates_drop_their_party`: every copy is void (the two Blank copies named). The party
  drops as `VoidLot` with any copy heading it (rotated), the result reads `clos-nw`, and a real party beside it stays.
- `notice_16967678_drops_both_its_void_winners`: ORG-2 still drops, now on every copy. ORG-4 now drops. Deloitte is the
  only winner, `selec-w`.
- `a_bilingual_not_awarded_pair_drops_and_a_company_beside_one_keeps_its_party`: the 12501289 pair drops.
  `non aggiudicato` / `Siemens` is kept: the 2-copy rule is not relaxed.
- `a_nested_void_winner_retires_both_its_halves` (23151933's shape): the retire keys are `[ORG-2, ORG-3]`, the mentions
  are ORG-1 and ORG-4, and the count is `dropped.void == 1`.
- `the_walks_verdict_is_the_folds_own_drop_decision`:
  - over r208, the nested r209 shape and sdk-0.1, the sections `fold_parties` reports Dropped equal the retire keys
    `mentions_and_dropped` returns;
  - the kept reasons (`1/2`, an award summary, no name, a gated identifier);
  - `Unknown` for a non-party section and for a missing one.

`crates/ingest/tests/project.rs`:

- `a_standing_inner_half_mention_is_retired_by_a_refold`, run as `an_incremental_refold_…` and `a_full_fold_…`.
  - Setup: a first fold of 23151933's recorded layout (ORG-3 alone, a real name), then `reparse_notice` to the nested
    void shape.
  - Assertions: `mentions_retired == 1` on both folds; no mention at ORG-2 or ORG-3; the org's mentions at 0;
    `dropped.void == 1`; 0 result winners. A second refold retires 0.
  - RED before part 2: `mentions_retired` was 0 on BOTH folds, which is prod's `0 non-name retired`.

`crates/app/src/supervisor.rs`:

- `the_void_walk_requeues_what_the_fold_drops_and_exempts_what_it_keeps` (pure `split_void_hits` with real
  `fold_parties` verdicts):
  - The fixtures are a dropped TED party, a kept bilingual party, a nested inner-key mention, a missing parse, an org
    with mixed mentions, a void head mentioned nowhere, and an award summary mentioned nowhere.
  - It asserts the exact pairs, `orgs`, `exempted`, `kept`, reasons and `kept_notices`, and that every Dropped mention is
    re-queued.
- `refold_void_names_requeues_only_the_folds_drops_and_converges` (end to end, scratch DB):
  - Setup:
    - an eForms bilingual award, where one party becomes 1/2 void and is kept, and one becomes 2/2 void and is dropped;
    - an r209 notice re-shaped into 23151933's nesting, with the mention at the inner key.
  - First dry run: 2 orgs, 2 mentions, 2 notices, and `exempted` is exactly
    `{org, name, mentions: 1, reason: "party kept: 1/2 names void", notices: [id]}`.
  - Wet run, then `project_incremental`: `mentions_retired == 2`. The kept org keeps its mention.
  - Dry re-walk: 0 mentions on 0 notices, and the kept party is still exempted. The walk converges.
- **RED check:** I temporarily restored 510's head rule inside `split_void_hits`. Both tests failed:
  - the pure test re-queued `(11,1), (13,1)` as well;
  - the end-to-end test listed `3 void-lot org(s) … 0 more exempted … 3 mention(s)`.
  - The probe line was then removed.

### Updated (they asserted the old outer-only keys or the old walk)

- `project::tests::a_void_lot_party_is_dropped_on_every_fold_branch`: the nested case's retire keys are now
  `[ORG-1, ORG-2]` ("both halves retire").
- `project::tests::the_dropped_report_counts_outer_sections`: `skipped == [ORG-1, ORG-2]`. The count is unchanged at
  (1, 1).
- `supervisor::tests::refold_void_names_counts_dry_refuses_over_expect_and_requeues`:
  - The fixture now also writes the void phrase into the stored parse (`UPDATE notice_texts`), because the walk now asks
    the parse, not the head name.
  - The exempted award summary carries `"reason": "names an award"`.

## Runs and their EXIT values

Every run used `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p model -p store
-p ingest -p tender-db --features tender-db/server …`, with output redirected to a file and the EXIT read from it.

| run | filters | EXIT |
|---|---|---:|
| probe 1 (temporary, classify every fixture copy before the change) | `--test probe_525` | 0 |
| RED 1 (new ingest tests, before parts 1 and 2) | `teds_other_not_awarded real_names_beside_teds notice_16967678 a_bilingual_not_awarded a_nested_void_winner the_walks_verdict the_dropped_report_counts_outer a_void_lot_party_is_dropped_on_every_fold_branch inner_half_mention` | 101 (8 failed as designed, 1 guard passed) |
| RED 2 (nested integration, before part 2) | `--test project -- inner_half_mention` | 101 (0 retired on both folds) |
| GREEN ingest | `--no-fail-fast -- partyname:: project::tests:: inner_half_mention standing_void_mention standing_placeholder_mention` | 0 (163 lib, 6 integration) |
| supervisor walk + stack | `--no-fail-fast -- refold_void_names refold_placeholder_names the_void_walk run_spec_futures_stay_inside_their_size_budgets an_execute_without_an_expected_count_is_refused` | 0 (6) |
| RED 3 (old head rule temporarily restored in the split) | `-- refold_void_names_requeues_only the_void_walk` | 101 (both failed as designed) |
| all lib suites of the gate's package set | `--no-fail-fast --lib` | 0 (ingest 510 + 4 ignored, model 3, store 182, tender-db 207) |
| ingest integration neighbours | `--no-fail-fast --test project --test text --test r208 --test r209 --test project_golden --test project_incremental --test project_equivalence --test data_quality` | 0 (102, 13, 15, 19, 11, 51, 1, 12) |
| probe 2 (temporary, `fold_parties` on the 11 real `/content` docs) | `--test probe_525 -- --nocapture` | 0 |
| supervisor walk (after adding `notices`) | as above | 0 (6) |
| final focused | `--no-fail-fast -- partyname:: project::tests:: refold_void_names refold_placeholder_names the_void_walk run_spec_futures_stay_inside_their_size_budgets an_execute_without_an_expected_count_is_refused inner_half_mention standing_void_mention standing_placeholder_mention a_void_lot_winner_mints_no_organization` | 0 (163, 7, 6) |
| final supervisor | `--no-fail-fast --lib -- supervisor::` | 0 (114) |

- `the_walks_net_holds_every_name_the_fold_drops` (partyname) scans every string literal in the partyname and project
  tests, the new fixtures included, and passed. So every new VoidLot copy is a hit of 510's net, and every Blank copy is
  a hit of 511's net.
- `ops/check.sh` was not run, as the task instructed. It still has to run before the commit.

## Deviations and notes

1. **Copies collected for the named notices only.**
   - Only `/content` GETs were allowed, and the diagnosis names the notices of 21 of the 40 mentions (10 of the 26
     notices). The other 19 mentions are 19 orgs on 16 r208 notices, which diagnosis 2 lists by org id only (2063070,
     2754895, 3530158, 3934255, 4439187, 4826424, 5286300, 6023876, 7400280, 7520025, 7520040, 7520041, 7969717,
     7985877, 8636082, 31405157, 31424358, 31424359, 31475976).
   - Never seen, so not added:
     - 8 more `lot not awarded` orgs, 2 more `no contract awarded`, 1 more `marche non attribue`. These may already
       match the collected copies.
     - `non applicable marche non attribue` ×2, `aucune offre recue`, `lot non attribue`, `lot non attribue voir section
       vi point vi 2 autres informations`, `non applicable aucune offre n a ete recue lot 5 non attribue`, `non
       attribution du marche`, `procedure infructueuse`.
   - Any of these parties the fold still keeps is now EXEMPTED, with its reason and up to 3 notice ids, so the dry run
     converges anyway. Read their copies from the post-deploy dry run's `exempted` (`/v1/notices/<id>/content`); any
     template wording left over is a follow-up lexicon add.
2. **The party split is the void class's alone.**
   - `refold-placeholder-names` keeps its head-name split, which re-queues every hit but an award summary. That is its
     documented superset rule; a kept hit costs a refold.
   - Its plan (`by_class`, the buyer canary, `roles`) and its test fixtures are org-level.
   - Moving it to `fold_parties` is a follow-up candidate, with the same code path (`split_void_hits` is class-neutral
     apart from 510's head rule for mention-less orgs).
3. **Net not widened.**
   - `organization_names` is about 38.7M rows (issue 169; 442 counted about 78M rowids), several times the 7.37M-org
     walk (348 s, job 1628). The widening is safe, since `fold_parties` filters, but not cheap. A note is left in
     `NonNames::net`'s doc and in operations.md.
   - The gap stays: a party the majority rule drops whose head copy reads as a name is outside the net. None of the 40 is
     that shape, and the lexicon now makes the DE-original head `Der Auftrag wurde nicht vergeben.` void by itself.
4. **Re-queue on any drop class**, not only `VoidLot`. The retire is class-blind, so a void-netted party dropped as
   `Placeholder` (a placeholder copy among void ones) is retired too. Re-queuing it keeps the re-queued set a superset of
   the fold's retires.
5. **Mention-less hits keep 510's head rule.** It is counted when the head is void, so the dry run still shows what the
   orphan sweep takes; otherwise it is exempted as `names an award`. After a drain and before `sweep-orphan-orgs`, the
   dry run lists the emptied orgs with 0 mentions on 0 notices. That is the convergence signal, asserted in the
   end-to-end test.
6. **Extra, not asked for:** `notices` (up to 3) on void `exempted` entries, so the kept copies can be read.
7. **Part 2's corpus-wide effect:**
   - Any refold of a notice whose dropped party, of any class, has a mention at its INNER key now retires that row and
     its party and bid-party rows, with the stale stamp and D5 announcements of `retire_mentions`.
   - A full fold can retire a batch of issue-259-repaired inner mentions at once. The verifier's sizing advice stands:
     read the fold's `retired` count against the dry run's mention count.
   - The daily fold only touches its delta.
8. **Cost:**
   - `VOID_WHOLE` linear compares grow from 50 to 148 per whole lookup. `void_prefix` does up to 12 lookups per name, but
     each also builds a fold, which dominates. Not measured.
   - The walk now reads one parse per mentioned notice of every hit, exempted ones included: 26 + the 375 exempted orgs'
     notices at dry 2351's scale. That is a few hundred `parsed_by_ids` rows in 200-notice chunks.
9. **The risk the issue names is unchanged:** a template wording that is ALSO a company's exact name would turn an award
   into no-award. All 98 are full TED form sentences, none plausible as a name. Read the drained list before the wet run,
   as 510 did.

## Commit file list (stage these by name; `git diff <file>` first)

- `crates/ingest/src/partyname.rs`
- `crates/ingest/src/project.rs`
- `crates/ingest/tests/project.rs`
- `crates/store/src/canonical.rs`
- `crates/app/src/supervisor.rs`
- `docs/operations.md`
- `.scratch/tender-db/525-void-templates/build-notes.md` (this file)

Not committed. No temporary files are left in the tree: both `crates/ingest/tests/probe_525.rs` copies were deleted, with
their test binaries.

## Review fixes (2026-10-12, three adversarial reviewers)

Every finding was checked against the code and reproduced before it was fixed. Nothing is committed. Nothing on prod was
touched: no new `/content` GETs, because the copies were already in diagnosis 2's scratch data. Each item below gives the
verdict, the fix and the tests.

### 1. Six residue orgs still kept (should-fix): CONFIRMED, fixed; one named residue

- **Reproduced.** Diagnosis 2 saved every copy of the 40 residue parties in the session scratchpad: `d2/parties.json`,
  366 copies from public `/content`. Deviation 1 above says these copies were "never seen", and that is wrong.
- **Probe.** A temporary integration probe, `crates/ingest/tests/probe_525r.rs` (since deleted), built each party as
  an r208 `Parsed` and asked `fold_parties`. It used the gate flags and package set, `--test probe_525r`, EXIT=0.
- **Result at the build.** 33 of the 39 parties with copies dropped. 23151933's ORG-3 has no copy list; it is class B,
  covered by part 2. Six were KEPT, exactly as the reviewer said:

  | notice | section | org | template | void copies |
  |---|---|---|---|---|
  | 17359294 | ORG-2 | 3530158 | `Non-attribution du marché` | 6/23 |
  | 18137696 | ORG-2 | 6023876 | `non applicable: marché non attribué.` | 2/24 |
  | 18549508 | ORG-2 | 7400280 | `Non applicable — Marché non attribué` | 3/24 |
  | 18236286 | ORG-3 | 5286300 | `Aucune offre reçue` | 1/24 |
  | 18715886 | ORG-6 | 7969717 | `Non applicable — Aucune offre n'a été reçue — Lot 5 non attribué` | 2/24 |
  | 18720683 | ORG-2 | 7985877 | `Procédure infructueuse` | 2/24 |

- **Fix.** 93 more exact values in `VOID_TEMPLATES` (see item 2), one comment group per template:
  - 17359294: 17 copies;
  - 18137696 (the colon spelling): 20 copies;
  - 18549508 (the dash spelling): 11 copies not shared with the colon spelling;
  - 18236286: 23 copies;
  - 18720683: 22 copies.

  Each value is the module's own `fold` of a published copy, printed by the probe.
- **Named residue: 18715886 ORG-6 (org 7969717, 1 mention).** Every copy carries lot 5's number, mid-sentence (`entfällt —
  kein Angebot eingegangen — Los 5 nicht vergeben`). An exact value would hold one lot of one notice, and the lot-qualifier
  strip reads only a trailing `(lot 5)`. So it is not added.
  - After the deploy, the dry walk lists it under `exempted` with `party kept: 2/24 names void` and notice 18715886. It
    is NOT counted among the void-lot orgs.
  - The issue's Verify ("0 void-lot orgs") therefore reads done with this one void party still served. I recommend the
    Verify say so: **done = 0 void-lot orgs, and `exempted` holds no void party but org 7969717**.
- **After the fix:** the probe gives 38 of 39 dropped, with 18715886 the one kept.
- **Tests:**
  - `project::tests::teds_other_not_awarded_templates_drop_their_party` runs the five templates verbatim, as fixtures
    `TED_NON_ATTRIBUTION`, `TED_NOT_APPLICABLE_CONTRACT_COLON`, `TED_NOT_APPLICABLE_CONTRACT_DASH`, `TED_NO_TENDERS_RECEIVED`
    and `TED_PROCEDURE_UNSUCCESSFUL`. For each, every copy must read void (four EN/PL "not applicable" copies `Blank`), the
    party drops whichever copy heads it, and the result reads `clos-nw`.
  - New: `project::tests::a_template_with_teds_pointer_drops_and_the_lot_numbered_compound_is_kept`. 18715886's
    party (`TED_NOT_APPLICABLE_NO_TENDERS_LOT_5`) stays kept, with the walk's reason `party kept: 2/24 names void`.
  - `partyname::tests::teds_other_not_awarded_templates_are_void` gains one specimen per new template.
  - RED: probe 0 (below) shows the six kept at the build.

### 2. The 98 whole values also worked as reason heads and prefixes (should-fix): CONFIRMED, fixed

- **Reproduced.** Probe 0 classed all 8 of the reviewer's probes `VoidLot` at the build. HEAD reads each as a name.
- **Fix: exact-only.**
  - The 98 are moved out of `VOID_WHOLE`. `VOID_WHOLE` is back to HEAD's 50, diff-identical to `git show HEAD`.
  - They now sit, with item 1's 93, in a new `VOID_TEMPLATES` (191). It matches the whole fold, or the fold before a
    BARE lot qualifier (`bare_lot_qualifier`: `Lote no adjudicado (lote 3)`).
  - `reason_head` and `void_prefix` never read it. So a name that opens with a template and goes on is a name again,
    whatever the language.
- **Other readers updated:**
  - `mentions_void` (the net) reads the templates exact.
  - `mixed_segments` treats a template segment as void.
  - The doc comments (module, `not_a_name_in` item 6, the list's own) say exact.
- **The reviewer's claim that exact-only loses none of the 40 is not quite right.** It lost 12381194's ORG-4 (org 31475976,
  23 copies), falling from a majority drop to 3/23.
  - Its copies print the template followed by TED's own bracketed pointer: `Los wurde nicht vergeben (siehe Abschnitt VI
    Ziffer VI.2 „Sonstige Informationen“).`, `Lot neatribuit (a se vedea secţiunea VI, punctul VI.2 …).`
  - At the build they read void only through `reason_head`.
- **Added `template_pointed`, a narrow exception for that pointer.** A template reads void when it is followed by ONE
  bracketed group that closes the name and that:
  - has the pointer's translation shape (`translation_shaped`: a pointer word first, a dozen words at most, no legal form
    by any of the crate's readings);
  - holds a form section reference (`has_form_reference`);
  - names nothing after its last reference (`names_after_reference`).

  `names_an_award` still guards it, as it guards a `VOID_WHOLE` reason head. With it, 12381194 drops at 15 of 23. The 8
  copies whose pointer the lists do not read (the DA `Jf.`, the EL, PT, MT, LV, PL, SK and EN copies) stay names.
  `mentions_void` holds the rule too, so the net still covers the fold.
- **Tests:**
  - `partyname::tests::real_names_beside_teds_templates_are_names` gains:
    - the reviewer's 8 probes;
    - `Verfahren erfolglos, neu vergeben an Schmidt Bau`;
    - `Lote no adjudicado por falta de ofertas`;
    - a non-bare qualifier: `(lote 3 adjudicado a Pérez SL)`;
    - a pointer group that names a company: `Los wurde nicht vergeben (siehe Müller Bau AG, VI.2)`.
  - `teds_other_not_awarded_templates_are_void` loses its "a whole value with its reason after it is void" assertion. It
    gains the bare-qualifier case and three pointer copies.
  - `every_entry_is_folded_and_every_stem_padded` covers `VOID_TEMPLATES` (folded, each in one list only).
  - `the_walks_net_holds_every_name_the_fold_drops` is green.
- **Collision scan, local data only.** I compared the 191 templates, folded, against every string in the local plans
  (511 `u4/plan-dry.json`, `u45/plan-residual.json`, the other 511/510 scratch JSON and dry 2351's report): 13,076
  strings. The only hit is `no award was made` itself, the residue name. This measures nothing about prod: the wet run
  still needs its read of the drained list, as 510's did.

### 3 and 5. The walk skipped the fold's `normalise_de1` (must-fix, two reviewers): CONFIRMED, fixed

- **Reproduced in the code.**
  - `parsed_by_ids` returns the stored `DE1-…-PartyName-Name`. `party_name_field` reads only BT-500 and the legacy fields.
  - Every fold path normalises before `mentions_and_dropped`: project.rs `build_plan`, the incremental pass and phase 2.
  - The walk did not, so every DE-1.x party read `Kept(no name)`.
- **Fix.**
  - `normalise_de1` now calls a per-parse `normalise_de1_parse`, with the same body.
  - `fold_parties` runs it on a clone when the profile is DE-1.x. A non-DE-1.x parse is not cloned.
  - Normalising twice is a no-op, since aliased ids no longer match.
  - The `fold_parties` doc and operations.md now name the input: "the parse as the fold reads it".
- **Tests:**
  - `project::tests::the_walks_verdict_is_the_folds_own_drop_decision` gains `eforms:eforms-de-1.2`. The stored parse
    has its names under the DE1 field and a `Los wurde nicht vergeben` winner. The fold's side is now the fold's own
    read: the chunk through `normalise_de1`, then `mentions_and_dropped`. The kept DE-1.x party reads `party kept: 0/1
    names void`, not `no name`.
  - `supervisor::tests::refold_void_names_requeues_only_the_folds_drops_and_converges` gains a DE-1.x notice, stored with
    the DE1 field and folded. Its winner turns void in the parse.
    - The dry run lists 3 orgs / 3 mentions / 3 notices, with `notices_by_profile` holding `eforms:eforms-de-1.2: 1`.
    - The wet run (`expect` 3) is followed by the fold: `mentions_retired == 3`, and the DE-1.x org keeps 0 mentions.
    - A dry re-walk lists 0 notices.
- **RED.** With the normalisation disabled (`if false && is_de1_profile`, reverted since), both tests failed as the
  reviewers described. EXIT=101:
  - the parity test: `eforms:eforms-de-1.2` left `[]`, right `["ORG-0003"]`;
  - the end-to-end test: `2 void-lot org(s) … 2 exempted …`, the DE-1.x drop exempted.

### 4. "Exactly what the paired fold retires" (nit): CONFIRMED, fixed

- operations.md now says every mention of the net's orgs that the fold will retire is re-queued, and that these are a
  subset of what the fold retires.
- The fold retires every dropped key of each re-queued notice: every class and org, inner halves included, and whatever
  else its delta drops.
- A `retired` count above the dry count is expected. One below it means re-queues on an unreadable parse, or a corpus
  that moved between the runs.
- Deviation 7 above: read the fold's `retired` count as at least the dry run's mention count, not as equal to it.

### 6. "N more exempted" counted orgs in both lists (nit): CONFIRMED, fixed

- The void job line now reads `…, K long); N exempted, B of them also re-queued (the fold keeps their party or the name
  names an award); M mention(s) to retire …`.
- The three job-line assertions are updated (`1 exempted, 0 of them also re-queued`).
- Not pinned with B > 0 in a job-line test: the overlap itself is pinned in the pure split test (org 13), and the line
  only formats it. The placeholder line is unchanged: its split is a partition.

### Rejected

None outright. Item 2's remedy was taken with one correction: exact-only lost one of the 40 parties, and
`template_pointed` brings it back.

### Superseded statements in the notes above

- "`VOID_WHOLE` grows from 50 to 148": it stays at HEAD's 50. The templates are `VOID_TEMPLATES` (191), exact.
- "a company whose name opens with a template and goes on … stays a name": true only since this fix. At the build it held
  only for a capitalised next word, and `Los wurde nicht vergeben e.K.` read void.
- Deviation 1's "never seen, so not added" list:
  - every one of those parties is in diagnosis 2's `d2/parties.json`;
  - all now drop except 18715886 (the named residue);
  - the 8 `lot not awarded`, 2 `no contract awarded` and 1 `marche non attribue` parties already dropped at the build.
- The job line's text (item 6) and the "exactly" wording (item 4).
- Drain sizing: the new exact values also catch any OTHER org whose head is exactly one of them (`no tenders were
  received`, `Verfahren erfolglos`). The dry run lists them. Read the list before the wet run.

### Runs (gate flags and package set, output to a file, EXIT read from it)

| run | filters | EXIT |
|---|---|---:|
| probe 0 (temporary, build state) | `--test probe_525r -- --nocapture` | 0: 33 dropped / 6 kept; all 8 reviewer probes `VoidLot` |
| probe 1 (exact-only) | same | 0: 32 / 7 (12381194 lost) |
| probes 2–3 (12381194's pointer tails: `section_reference_pointer`, `pointer_led`) | `-- probe_525r_tails` | 0 |
| probe 4 (`template_pointed`) | `-- --exact probe_525r` | 0: 33 / 6 |
| probe 5 (with item 1's five templates) | same | 0: 38 / 1 |
| probe 6 (after every lexicon change) | same | 0: 38 / 1 |
| focused | `--no-fail-fast -- partyname:: project::tests:: refold_void_names refold_placeholder_names the_void_walk run_spec_futures_stay_inside_their_size_budgets an_execute_without_an_expected_count_is_refused inner_half_mention standing_void_mention standing_placeholder_mention a_void_lot_winner_mints_no_organization` | 0 (164 lib, 7 integration, 6 supervisor) |
| RED (DE-1.x normalisation disabled) | `--no-fail-fast -- the_walks_verdict_is_the_folds_own_drop_decision refold_void_names_requeues_only_the_folds_drops_and_converges` | 101 (both failed as designed) |
| every lib suite of the package set | `--no-fail-fast --lib` | 0 (ingest 511 + 4 ignored, model 3, store 182, tender-db 207) |
| ingest integration neighbours | `--no-fail-fast --test project --test text --test r208 --test r209 --test project_golden --test project_incremental --test project_equivalence --test data_quality --test doe` | 0 (102, 13, 15, 19, 11, 51, 1, 12, 12) |
| final focused (after the last doc edit) | as focused | 0 (164, 7, 6) |

- The probe file is deleted, and its test binaries go with the next gate's prune. `git status` shows the same six files
  plus this directory.
- `ops/check.sh` was not run, as the task instructed. It still has to run before the commit.
- The commit file list is unchanged.

## Second review (fix delta 80157d3..1st commit; wf_151870b7-e19, 2 lenses, findings verified)

- **`template_pointed` read from the first bracket to the last** (verified real, should-fix). So
  `Los wurde nicht vergeben (siehe VI.2). Los 2: Müller Bau (siehe VI.3)` read VoidLot and would drop a real winner.
  The Swedish shape did the same. The pointer must now be ONE group closing the name: a second `(`/`[` inside it
  refuses. None of the 23 pointed TED copies has one.
- **`mentions_void` read a template after ANY lot group** (nit; fixed because it is the same risk class).
  `names_an_award` uses it as its "this entry is void" test, so `1) nessuna aggiudicazione - 2) procedura infruttuosa
  (lotto 3 Rossi Costruzioni)` lost its award. A template now counts there only as `classify` reads one: whole, or
  before a bare lot qualifier. The walk net still covers every template the fold drops.
- Tests: both multi-group names and both numbered summaries were added to
  `real_names_beside_teds_templates_are_names`. Each fix's mutation fails its case (EXIT=101), and `partyname.rs`
  was restored byte-identical. Focused partyname + walk runs: EXIT=0.
- The walk lens found nothing.
