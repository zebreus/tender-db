# 587 unit 1 — decision D587: a bare contracting-unit label serves as the notice's own buyer (option b)

Decided 2026-10-24 ~22:25Z by the decision author, from `measure.md` / `measure.json` and three more bounded reads of
mine: s07, s08 and s09 (SQL 10 of 12 used in total, 0 GETs; GETs stand at 20 of 20). Nothing was written on the box,
no cargo was run, no git command was run. Paths below are relative to `.scratch/tender-db/587-u1/` unless they start
with `crates/`.

**Challenged 2026-10-24 ~22:40Z by the skeptic** (section **Challenge** at the end): 8 defects applied in place (C1–C8,
each marked `[C#]` where it changed the text), 9 objections refuted (R1–R9). Two more box reads, s10 and s11, so SQL
stands at 12 of 12; 0 GETs. The oracle now passes `--selftest` 124/124, and the pre-challenge files are kept in
`raw/pre_challenge/`.

**The oracle is `tools/label.py`.**
- `python3 -I tools/label.py --selftest` passes 124/124: 49 MUST_HIT, 65 MUST_KEEP, 6 town cases and 4 COORD cases
  [C2, C3]. It passed 117/117 before the challenge.
- `--report` writes `label.out` and `label.out.json` over the 6,010 listed orgs.
- `fixtures/names.jsonl` (3,014 distinct listed spellings with their `bare` and `coord` flags) and
  `fixtures/selftest.jsonl` (124 rows) are unit 2's port fixtures. Both were regenerated after the challenge: 3
  spellings flipped to not-bare [C3], and 2 are `coord` [C2].

## Decision

**Option (b).** One rule, the whole-name label test D587-1, decides on both paths:
- **ORG path (D587-2).** An identifier-less party section whose every published name is a bare label is folded into
  the notice's own buyer. Its roles bind the buyer's organization, anchored at the buyer's section, and the section
  mints no mention.
- **Touchpoint path (D587-3).** A bare-label touchpoint aliases to its own parent Organization. This is a new tier
  after 586's tiers and before 586's gate.
- **Fallback (c) for the ambiguous notice.** The label's roles are dropped (counted, served as an omission, never
  bound to the pooled row) in these cases:
  - the notice names no buyer, several buyers, or a buyer in another town;
  - the notice itself says the section is another organization: any reference to it is a `*-SProvider`,
    `purchasing-body` or `Procedure-CPB` role. Then **all** of that section's roles drop, not only that one
    [C1, C5];
  - the label names a central service that buys for other bodies (COORD: `centralizada`) [C2].
- **The label printed as the buyer itself, or as a result party (winner, tenderer),** is not touched (F1, later)
  [C7].

Rejected:
- **(a) alone.** A touchpoint-only stem list reaches 4.4% of the measured label roles: 510 of 11,613 role-tenders.
  The hubs are made on the ORG path: 88.6% sit in TED R2 address blocks and 7.0% in eForms ORG sections.
- **(c) alone.** It would drop about 10.7k ORG-path role-tenders over the 37 measured orgs (appeal-information,
  further-information, tender-receipt, specifications-provider and the rest) that the notice attributes to its own
  buyer's unit. A publisher that prints its full name
  in the same blocks keeps those roles on the buyer today, so (c) would make role coverage depend on how the unit was
  spelled. (c) is kept only where the buyer is ambiguous, and there an omission is the safe answer (D576-1's ranking:
  omission is not W).
- **(d) decline.** Of the 37 bare orgs read, 24 hold 80% of the measured mentions and pool unrelated buyers. That is
  W by 586 D2's own definition of a hub, not D576-1's accepted N.

## Key numbers

| | orgs | mentions |
|---|---:|---:|
| Measured BARE (measure.md, 19 prefix listings) | 2,823 | 21,049 |
| **D587-1 hits among the 6,010 listed orgs** (was 2,648 / 20,807 before [C3]) | **2,645** | **20,787 (98.8% of BARE)** |
| … name-keyed with a country (the hubs; the cohort's core) | 195 | 16,648 |
| … of which COORD, dropped and never aliased [C2] | 2 | 117 |
| … identifier-less with a NULL country (singletons; folded too) | 2,117 | 2,930 |
| … identified (D587-2 keeps identified sections) | 333 | 1,209 |
| Hits among DEPT (2,712 / 35,856) and OTHER_ORGAN (475 / 11,565) | 2 / 0 | 4 / 0 |
| Residue: measured BARE that the rule misses (was 177 / 246 before [C3]) | 180 | 266 |

- **The 2 DEPT hits are labels when read.** "Unidad de Contratación, Dirección de Gestión y Servicios Generales" and
  "Servicio de Contratación y Compra" (the classifier lacked `compra`). So the rule hits 0 real bodies in the
  listings.
- **Every one of the 37 measured bare orgs hits:** 24 W, 8 dominant+tail and 5 N. The 4 department controls do not
  hit (Ramón y Cajal, ADIF, Stuttgart, Vigo).
- **Residue.** Names that carry a building word ("Service des marchés publics: Hôtel du Département" 53, "…: hôtel de
  ville"), "SERVICE DES MARCHES CENTRALISES" and similar. They are counted and not gated.
- **Lower bound.** The listings are prefix-anchored, so labels led by an article or a code ("La Direction…",
  "DCP - …") are unseen. Unit 3's census sizes the full cohort.

**Buyer per notice (s07, s09).** s07 covers 2,479 (Tender, notice) pairs where one of 14 labels is served:
- 1,632 have exactly one buyer section in the same notice.
- **42 (1.7%): the label IS the buyer.**
- **4 (0.2%) have two or more buyers.** These are 1369148's joint notices with 2, 6 and 14 buyers.
- 801 have no buyer row from that notice. s09 rechecked 800 pairs over 4 labels: every pair without a same-notice
  buyer has the version's buyer row anchored at another notice of the same Tender (498 of 498). No version lacks a
  buyer. Served rows reflect supersession across notices, not the fold's per-notice view. So 1.7% and 0.2% are the
  served-side figures, and the census measures the per-notice ones from the parse.

**The inherited-buyer case (s08).** On label 1193769 "Servicio de Contratación y Convenios", 118 buyer rows print
Junta de Andalucía Consejería names ("Consejería de Agricultura, Ganadería, Pesca y Desarrollo Sostenible. Dirección
General de Infraestructuras del Agua", …). They carry identifier **S4111001F** and bind org **2660 "Tribunal
Administrativo de Recursos Contractuales de Andalucía"**. This is the 572/584 shape: a shared administration
identifier binds the buyer to the review tribunal. D587-2 aliases to the buyer SECTION, so the label's roles follow
whatever that section binds. Fixing the buyer binding fixes both on the next refold. Filed as F2.

## D587-1 — the label test (`bare_label(name)`), exact

1. **Normalise.** `crate::partyname::fold(name)`: lowercase, alphanumeric runs joined by one space, Latin diacritics
   folded. This is the fold 586's gate uses; `tools/classify.py`'s `fold` is the port. No bracket cut: a bracket that
   names a body leaves its words, so the name is not bare.
   - **Trailing connector [C3].** When the LAST folded token is a CONNECT token, the name is not bare. Such a name was
     either cut ("Servicio de Contratación del Área de", "Direction de la commande publique et d") or ends on an
     acronym that collides with a connector ("MESA DE CONTRATACIÓN DEL ET": ET is the Ejército de Tierra, and `et`
     is the French connector). This hits 3 listed orgs / 20 mentions, and they are now in MUST_KEEP.
2. **Drop CONNECT tokens anywhere** (articles, prepositions, conjunctions):
   `la le les l de du des d et au aux a en el los las lo del al y e i o dels els da do das dos di dei della delle
   degli dello dell ed il gli per der die das den dem und fur im het van voor the of and for w z ze oraz dla`.
3. **Drop NOISE tokens anywhere:** `responsable responsables contact contacto contacte punto point attn attention`.
4. **Test.** The rest must be non-empty. Every token must be in UNIT ∪ FUNC ∪ COMPOUND. It must hold a COMPOUND token,
   or at least one UNIT token and one FUNC token.
   - **UNIT:** `service services servicio servicios servizio servizi servei direction direccion direccio unidad unitat
     unite unidade departement department departamento departament dept division bureau cellule pole section seccion
     area oficina negociado ufficio settore sezione wydzial dzial zespol biuro sekcja referat abteilung sachgebiet
     fachbereich fachdienst afdeling afd team dienst mesa organo secretariat`
   - **FUNC:** `contratacion contrataciones contratos contrato contractacio compras compra suministros suministro
     adquisiciones patrimonio patrimonial convenios asuntos generales administrativa administrativo administrativas
     administrativos centralizada gestion economica economico financiera commande commandes publique publiques public
     publics marches marche achats achat approvisionnement approvisionnements contrats contrat juridique juridiques
     affaires logistique moyens generaux zamowien zamowienia publicznych zaopatrzenia inkoop aanbestedingen
     aanbestedingszaken aanbesteding zentrale zentraler vergabe einkauf beschaffung procurement purchasing contracts
     contract gare appalti contratti acquisti provveditorato economato forniture lavori assurances assurance
     transversaux ressources materielles travaux fournitures investissements mutualise mutualisee umow bau
     inkoopondersteuning`
   - **COMPOUND:** `vergabestelle vergabestellen beschaffungsstelle`
   - **COORD [C2]:** `centralizada`. This is a subset of FUNC, so the name still passes `bare_label`. A label that
     holds a COORD token is `coord_label` and is **dropped on both paths, never aliased**: it names a central service
     that buys for other bodies of its administration in the same capital, so the town guard cannot separate them.
     The listed hits are 1378491 "Servicio de Contratación Centralizada" (113) and 2941741 "Unidad de Contratación
     Centralizada" (4).
5. **Left out on purpose.** None of these words is in any list, because each can be what makes a name a body:
   - names of the state or a level of government: `etat estado federal finances regional`;
   - central and joint bodies: `central centrale unica committenza appaltante groupement union agence plataforma
     sector`;
   - organs and offices: `junta consultiva independiente consejo comision secretaria gerencia alcaldia`;
   - `general` alone, `hotel ville mairie departement` (as a body word), and any place, person or number.
   Adding a word is a decision change. It needs a MUST_KEEP run and a regenerated fixture.

Unit 2 must check that every list entry is its own fold under `partyname::fold`, for example that `Dział` folds to
`dzial`. The check is `every_list_entry_is_its_own_fold`, in the style of 586's. It must also check that
COORD ⊆ FUNC.

## D587-2 — the ORG path

The rule lives in a new function, `unit_label_decision(sdk01, sections, org_alias, parsed, notice_id) ->
UnitLabels`. It is pure, so it can be called from both places that need it (unit 2). sdk-0.1 is a no-op: its party
sections are the buyer and winners themselves.

It runs over the outer party sections, after issue 259's `nested_org_aliases`, the 510/511/520/525 void drop,
`Touchpoints::retarget` and `apply_buyer_fix`. The steps run in order, and the first that applies decides:

1. **Candidate.** An Organization section (with its nested inner halves) qualifies when:
   - every name it publishes, in every language, passes `bare_label`;
   - and the mention it would mint carries no identifier after the existing gates (`gated_identifier`, 530-T, 575's
     cf35 refusal).
   An identified label is keyed per identifier, so it is one buyer (333 orgs / 1,209 mentions, N). It is kept as
   today.
2. **The label is the buyer.** A `BUYER_ROLES` reference (`Procedure-Buyer`, `buyer`) names the section, through
   `org_alias` and after the buyer fix. It is **kept** as today and counted `kept_buyer` (F1).
3. **A result names it.** The result roles are `RoleKind::Contractor` (`Tenderer`, `Tenderer-MainCont`,
   `Tenderer-SubCont`, `winner`) and `bid:tenderer`. It is **kept** and counted `kept_results`. The results graph
   binds Organizations as it does today. On the 198 hubs' served rows this is 4 `Tenderer` rows on 1164742 (s10), F1.
   - **Not results [C4]:** `Contract-Signatory`, `LotResult-Paying` and `LotResult-Financing` are buyer-side.
     project.rs's `GUARD_SIGNATORY_ROLES` reads the signatory as "the buyer that signs the contract". These roles
     follow step 7 like any other role.
   - Counting them as results would keep the section minting the pooled hub for ALL its roles on that notice, since
     the first step that applies decides. s10 finds 1 such Tender (1804271); Paying and Financing are 0.
4. **No role names it.** It **mints nothing** (`unreferenced`). This keeps the hub rows from being held alive by
   mentions that serve nothing.
5. **Find the buyer.** B is the set of distinct outer sections that `BUYER_ROLES` name after the buyer fix, minus
   label sections.
   - |B| = 0: **drop** (`dropped_no_buyer`).
   - |B| ≥ 2: **drop** (`dropped_buyers`). In a joint notice, which member owns the unit is unknown.
6. **Town guard.** Compare the label section's town (`TED-TOWN` in legacy and R2, `BT-513-Organization-Company` in
   eForms; unit 2 pins the field ids against fixtures, and a dialect with no town field reads as no evidence) with
   b's town.
   - When both print a town and `same_town` fails, **drop** (`dropped_town`).
   - `same_town`: fold both, drop `cedex` and tokens that are all digits, then one key must be a token prefix of the
     other. "Paris Cedex 07" matches "Paris"; "Saint-Denis" does not match "Saint-Étienne".
   - When either side prints no town, there is no evidence against the alias.
6a. **Another organization by the notice's own words [C1, C5].** If ANY reference to the label section, or to one
   of its inner halves, is a `*-SProvider`, `purchasing-body` or `Procedure-CPB` role, **drop every role** of that
   section (`dropped_sprovider`, counted per section and per role). Each of those roles names an organization other
   than the notice's buyer by definition. project.rs's `legacy_role` says it of `purchasing-body`: "NOT this
   notice's buyer". If the section is that other organization, its `Lot-AddInfo`, `Lot-TenderReceipt` and
   `Lot-DocProvider` references point at it too.
   - Measured on 2997642 "Zentrale Vergabestelle" (s11): 83 served label sections carry `Procedure-SProvider`, and 78
     of them carry 1–4 other roles as well.
   - On 51 of those 83 Tenders the buyer is the Landkreis Wunsiedel i. Fichtelgebirge, whose Zentrale Vergabestelle
     this mostly is.
   - On about 32 the buyer is another body, counted as distinct Tenders per buyer org. Most of those bodies sit
     inside that Landkreis: Kirchenlamitz (10), the Verein "Hilfe für das lernbehinderte Kind im Landkreis
     Wunsiedel" (17, in 11 spellings), Markt Schirnding, Markt Thiersheim, the Kommunalunternehmen KUFi and a
     Fichtelgebirge GmbH. The last is Gemeinde Odenthal, in NRW.
   - The pre-challenge rule removed only the SProvider row and aliased the rest to these buyers. Where the town guard
     has no town or the same town (the Verein, KUFi, the GmbH), that is the "coordinating body's unit bound to a
     member" W.
   - Cost, accepted as omission: on the Landkreis's own 51 Tenders, the publisher filed its own unit as SProvider,
     and those roles are now dropped instead of aliased.
6b. **COORD [C2].** If the section's names are `coord_label`, **drop** (`dropped_coord`).
7. **Alias.** Every role reference whose target is the label section, or one of its inner halves, is retargeted to
   b. After steps 6a and 6b, no `*-SProvider`, `purchasing-body` or `Procedure-CPB` reference is left here.
   - The label section mints no mention and joins the retire keys (`skipped`), as a void section does. A recorded
     mention at its key is retired by the refold. Every drop (steps 5, 6, 6a, 6b) also mints nothing and joins
     `skipped`.
   - Counted `aliased`, with the aliased roles tallied by `RoleKind` (see R1).

Consequences, accepted:
- The party row is anchored at b's section, so the served `mention_name` is the buyer's printed name. This is D576-1
  (i)'s shape, and 586's tier-C note says the same.
- The unit's own wording ("Direction de la commande publique") is no longer served on the party. It stays in the
  notice.
- **Never through `org_alias`.** `nested_org_aliases` / `enclosing` would add the label's names as variants of the
  buyer's mention, which is D576-1's warning. The alias is a role retarget only. The buyer's mention, names and
  variants do not change.
- **Census isolation, as D576-1 (v).** `role_census::buyer_fix`, `buyer_side_mentions` (369's `buyer_key`, 481's
  tokens) and the 530 guards read what they read today. The label decision runs after `apply_buyer_fix`, consumes its
  result, and never feeds it.

## D587-3 — the touchpoint path

In `touchpoint_decision`, after D576-1 (i) and 586's tiers A and C, and before the gate: when every name a touchpoint
publishes passes `bare_label`, the verdict is `Alias(parent, AliasTier::Label)`, where the parent is the enclosing
Organization section in the same notice.
- The parent is the unit's structural owner, and anchoring there is 576's FK-safe shape.
- **Same carve-outs as D587-2 [C6].** The verdict is `Drop` (counted `TouchpointRoles::dropped_label`) in two cases:
  - any reference to the touchpoint is a `*-SProvider`, `purchasing-body` or `Procedure-CPB` role (D587-2 6a);
  - the touchpoint's names are `coord_label` (D587-2 6b).

  Otherwise a TPO "Vergabestelle" named as `Procedure-SProvider` would serve its buyer parent as its own service
  provider, which is the contradiction D587-2 removes. 0 such TPO roles were measured, so this is a latent case.
- **Chain.** When the parent is itself a D587-2 label section, `Touchpoints::retarget` runs first and D587-2's
  retarget second, so the role ends on the buyer, or drops by D587-2's rules. A parent that is itself the buyer label
  is F1's shape.
- **Effect on 586.** `procurement department` in 586 WHOLE now aliases instead of dropping. 586's pure-function port
  test (`alias_tier`, `gated`) does not change, because the new tier sits in `touchpoint_decision`, not in
  `alias_tier`.
- **Counted** `TouchpointRoles::aliased_label`, part of `aliased`.

## Why the risk of a real body named like a department is small, and what catches the rest

1. **Whole names only.** A real body's name holds at least one word that is neither a unit nor a function: a place,
   "de l'État", a ministry, a person, a number. That word fails D587-1.
   - "Direction des Achats de l'État" leaves `etat` and is not a label.
   - So are "Service des achats de l'État", UGAP, "Junta de Contratación Centralizada", "Junta Consultiva de
     Contratación Administrativa", DGRCC, OIReScon, "Plataforma de Contratación del Sector Público", "Centrale Unica
     di Committenza", "Stazione Unica Appaltante", "Beschaffungsamt des BMI", "Zentrale Vergabestelle des Landes
     Brandenburg" and "La Poste - Direction des achats".
   - All are in MUST_KEEP (62/62 pass).
   - Measured: 0 real bodies among the 3,187 DEPT and OTHER_ORGAN listed orgs. The rule hits 2 of them, and both are
     labels when read.
2. **Coordinating bodies.** A district's Zentrale Vergabestelle or a shared service printed bare in a member's notice
   is the remaining W shape. Three steps turn it into an omission:
   - the town guard (step 6) catches a member in another town;
   - step 6a [C1, C5] catches a section the notice itself files as a service provider, purchasing body or CPB, and
     drops ALL that section's roles. This is the Landkreis Wunsiedel shape in s11;
   - step 6b [C2] catches a COORD label (`centralizada`).
3. **A body printed bare AS the buyer is never aliased** (step 2). This covers DÖE 19553 "Zentrale Vergabestelle",
   the buyer of record on 291 Tenders.
4. **Identified sections are never aliased** (step 1): an identifier is a body's identity.
5. **Residual, accepted.** A label in a same-town notice that is really another body's unit: a city's Vergabestelle
   receiving tenders for a municipal company, or a bare "Direction des achats" of the DAE in a Paris ministry notice.
   The label itself names no body, so reading it as the publisher's own unit is the notice's plain reading. The
   read-back's coordination stratum measures this shape.
6. **Reversible.** The alias is recomputed on every refold. A wrong vocabulary word is removed and its cohort
   refolded; the census lists the cohort.

## Unit 2 — build (M)

### Files and anchors (main, 2026-10-24)

| File | Anchor | Change |
|---|---|---|
| `crates/ingest/src/project/unit_labels_587.rs` | new | `bare_label(name)` (with the trailing-connector guard [C3]), `coord_label(name)` [C2], `same_town(a, b)` and the CONNECT/NOISE/UNIT/FUNC/COMPOUND/COORD `const`s, as folded tokens, plus `OTHER_ORG_ROLES = ["*-SProvider", "purchasing-body", "Procedure-CPB"]` [C1, C5]. Port of `tools/label.py` |
| `crates/ingest/src/project.rs` | `mod` lines beside `mod touchpoint_names_586;` | `mod unit_labels_587;` |
| `crates/ingest/src/project.rs` | `NoticeState::read`, after `apply_buyer_fix` (~6046–6052) | `let labels = unit_label_decision(…); labels.retarget(&mut raw_roles);` (D587-2 steps 5–7) |
| `crates/ingest/src/project.rs` | `mentions_and_dropped_gated` (~6189–6250) | Exclude aliased, dropped and unreferenced label sections from `mentions`, and add them to `skipped` (the retire keys). This uses the SAME `unit_label_decision`, as `touchpoint_decision` serves both sides. Both gate variants (`published_mentions` too) |
| `crates/ingest/src/project.rs` | `TouchpointRoles` (231), `touchpoint_mentions` (6392), `AliasTier` (7434), `touchpoint_decision` (7522) | `AliasTier::Label`; `aliased_label` field and tally; the D587-3 tier |
| `crates/ingest/src/project.rs` | `Report` (52) | `pub unit_labels: Box<UnitLabels>` with `aliased, aliased_by_kind` (a small map by `RoleKind`; R1), `dropped_town, dropped_buyers, dropped_no_buyer, dropped_sprovider` (sections and roles, [C1, C5]), `dropped_coord` [C2], `unreferenced, kept_buyer, kept_results, kept_identified`, plus `add`. `TouchpointRoles` gains `dropped_label` [C6] |
| `crates/app/src/supervisor.rs` | `touchpoint_suffix` (6344) and a new `unit_label_suffix` | `…; touchpoint roles (issue 576): … aliased (issue 586: … by spelling, … by organ; issue 587: … by label) …, dropped (…; issue 587: … label as another org or central service)` and `; unit labels (issue 587): A aliased to the notice's buyer (r of them review/mediation roles), D dropped (t town, b several buyers, n no buyer, s section named as service provider / purchasing body / CPB, c central service), U unreferenced, K kept (buyer k1, identified k2, result k3)`. supervisor.rs is a 551 flip file, so the next switch-off twin carries both |
| `crates/app/src/supervisor.rs` | new `Spec::UnitLabelCensus` | Read-only census (unit 3). Its arm body goes in `Box::pin(async move { … }).await` (CLAUDE.md: `run_spec`'s one future) |
| `crates/store/src/canonical.rs` | `name_phrase_census` (38750), `take(cap)` at 38980 | F3 rides along: sort rows by mentions DESC before `take(cap)`. The unit-label census reuses this scan with the `bare_label` predicate and its own stored-report kind |
| `crates/app/src/v1/docs.rs`, `crates/app/data/openapi.json` `TenderDetail.parties.description`, `CHANGELOG.md` | | One sentence for each surface: "A party or touchpoint whose whole name is a contracting-unit label (…) is served as the notice's own buyer (a touchpoint: its organization) in that role; where the notice names no buyer, several, or one in another town, where the notice itself names that party as a procurement service provider, purchasing body or central purchasing body, or where the label names a central purchasing service, its roles are not served." [C1, C2, C5] |
| `crates/ingest/tests/fixtures/unit_labels_587/` | new | `names.jsonl` and `selftest.jsonl`, copied from `fixtures/` here |

There is no new dependency, and no PROJECTION_EPOCH bump: unit 3 refolds the cohort, and the daily fold applies the
rule to new notices.

### R0 tests (all must pass focused, then `ops/check.sh` redirected to a file, reading `GATE-EXIT=`)

Focused run:

    CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p model -p store -p ingest -p tender-db --features tender-db/server unit_label

**Unit tests in `unit_labels_587.rs`:**
- `the_port_matches_the_oracle_on_every_listed_name`: all 3,014 rows, on both `bare` and `coord`. Print the first 20
  mismatches; 0 allowed.
- `the_named_cases_hold`: 124 rows (MUST_HIT, MUST_KEEP, TOWN, COORD).
- `every_list_entry_is_its_own_fold`, and COORD ⊆ FUNC.

**`project.rs` `mod issue_587`:**

| # | Test | Asserts |
|---|---|---|
| T1 | R2.0.9 F02 | The buyer block is "Ville de Levallois" (town Levallois-Perret). `ADDRESS_FURTHER_INFO` and `ADDRESS_PARTICIPATION` both read "Direction de la commande publique", same town. Both roles bind the buyer org at the buyer section. No mention at the label sections, and both are in `skipped`. `aliased 2` |
| T2 | eForms | ORG-0001 Procedure-Buyer "Hospital Universitario Ramón y Cajal". ORG-0002 "Unidad de Contratación" under `Lot-AddInfo` and `Lot-TenderReceipt` aliases to ORG-0001 |
| T3 | Touchpoint | TPO-0001 "Unidad de Contratación" under ORG-0001 "Intendencia de Rota", as `Lot-ReviewOrg`: `Alias(ORG-0001, Label)`, `aliased_label 1`, `aliased_a`/`aliased_c` 0 |
| T4 | Town guard | "Zentrale Vergabestelle" (Ratzeburg) beside the buyer "Gemeinde X" (Mölln): dropped, `dropped_town 1`, binds nothing, no mention |
| T5 | Joint notice | Two buyers: `dropped_buyers` |
| T6 | Label as buyer | The label is the buyer, in both the TED shape and the DÖE `ND-` shape: kept, minted as today (`kept_buyer`) |
| T7 | Service provider [C1, C5] | A label section named by `Procedure-SProvider` AND `Lot-AddInfo` + `Lot-TenderReceipt` (2997642's s11 shape): **every** role drops, nothing binds the buyer or the pooled row, no mention, `dropped_sprovider` 1 section / 3 roles. The same holds for a legacy `purchasing-body` block and for `Procedure-CPB`. (Before the challenge, T7 asserted that AddInfo aliases.) |
| T8 | Identified | An identified label ("Service des Achats Responsables" with a SIRET) is kept |
| T9 | Result party | A label that a result references (winner or tenderer) is kept |
| T10 | Unreferenced | An unreferenced label mints nothing |
| T11 | Departments and bodies still mint | "Stadt Fürth - Zentrale Vergabestelle", "Servicio de Contratación del Ayuntamiento de Vigo", "Direction des Achats de l'État", "Centrale Unica di Committenza" and "Junta de Contratación Centralizada" as further-information: each still mints |
| T12 | Multilingual | One language variant names a body: mints |
| T13 | Nested legacy block | A label in a nested legacy block decides on the outer party. The buyer mention's `name` and `variants` are unchanged: the `org_alias` trap |
| T14 | Census isolation | A review-named buyer beside a label AddInfo leaves `BuyerFix`, `buyer_key` and the guard tokens unchanged |
| T15 | Agreement | `NoticeState::read` and `mentions_and_dropped_gated` agree: no retargeted role whose label section still mints, and no skipped label section with a surviving role reference. The FK holds |
| T16 | Chain | A touchpoint label under an ORG label ends on the buyer |
| T17 | 586 unchanged | 586's `the_port_matches_the_reference_on_every_saved_pair` is unchanged. The existing touchpoint-gate test still drops its prose and person rows |
| T18 | COORD [C2] | "Servicio de Contratación Centralizada" beside the buyer "Consejería de Educación", same town: dropped, `dropped_coord 1`, binds nothing, no mention. On the touchpoint path it is `Drop` (`dropped_label`) |
| T19 | Trailing connector [C3] | "MESA DE CONTRATACIÓN DEL ET" as further-information mints as today (not a label) |
| T20 | Signatory is not a result [C4] | A label named by `Contract-Signatory` and `Lot-AddInfo` aliases both to the buyer. It is not `kept_results`, and no pooled mention is minted. A label named by `Tenderer` is still `kept_results` |
| T21 | Touchpoint carve-out [C6] | A TPO label under the buyer, named by `Procedure-SProvider`: `Drop`, not `Alias(parent, Label)` |

**Supervisor:** the suffix strings, and a smoke run of the census arm on the test DB.

**Review.** The build runs the usual lenses with a skeptic each. One lens is told to hunt a real body that D587-1
hits.

## Unit 3 — dry census, then the drain (M)

1. **Deploy** on the next deploy (it may ride the same gate as anything else ready). There is no banner: nothing
   regresses, and the drain only removes pooled rows.
2. **Dry census.** `POST unit-label-census`, read-only, before any refold. Its stored report holds:
   - every org whose served name passes `bare_label`, with mentions and roles by path (ORG, TPO, ND), sorted by
     mentions DESC;
   - the full cohort: every (notice, section) mention on those orgs, as notice-id chunks of ≤ 1,000;
   - a verdict tally from re-reading each cohort notice's parse through the same `unit_label_decision` /
     `touchpoint_decision`: aliased, aliased_label, the drops by kind, unreferenced and the three kept kinds.

   **Hard bars** (a fail stops the drain and leads to a fix and a re-census, not a hold on other work):
   - **H1.** Cohort notices between 15,000 and 60,000. The listed lower bound is 19,578 identifier-less mentions
     (19,598 before [C3]).
     - The bar counts notices, not mentions. s03's distinct notices per mention on the 22 measured label orgs are
       0.906 in aggregate, ranging 0.64–1.0 per org and section kind. That puts the listed floor at about 17.7k
       notices, about 15% above the bar (R2).
     - A fail near 15,000 is therefore read against that ratio before anything is "fixed".
   - **H2.** All 24 measured W hubs are in the cohort. Over the 23 that are not DÖE (19553 excluded), kept ≤ 5% of
     their label sections; the measured buyer-of-record share is 0.8% (70 of 8,816 role-tenders).
   - **H3.** Verdict coverage ≥ 99% of cohort sections. A section without a parse is listed.

   **Informing numbers** (they change no step; a surprise becomes a follow-up and an operator banner only if served
   data gets worse):
   - the town-guard drop share (expected < 10%);
   - several-buyer drops (< 1%);
   - no-buyer drops;
   - service-provider / purchasing-body / CPB section drops, as sections and roles (expected about 85 sections,
     83 of them 2997642's) [C1, C5];
   - COORD drops (expected about 117 mentions on 2 orgs, listed) [C2];
   - aliased roles by `RoleKind`, with the review and mediation share (expected about 4.5%; 601 of the 198 hubs'
     13,293 served rows, s10) (R1);
   - kept_buyer (F1's size);
   - aliases whose buyer section binds a review-named org (F2's size).
3. **Drain.** `refold-notices` over the cohort chunks (`REFOLD_NOTICES_CAP` = 1,000, so about 20–30 posts). Each
   post queues its own `project`, and the job queue serializes them. If 585 unit 2 is in the same deploy, union its
   hit notices into the same chunks. Expected:
   - mentions retired ≥ about 19.2k: 19,578 minus about 400 kept as buyer, on listed orgs alone. Drops retire
     mentions too, so C1 and C2 do not change this;
   - role rows retargeted ≥ about 10k. This is about 250 fewer than before the challenge: about 180 non-SProvider
     roles on 2997642's SProvider sections [C1], and about 76 COORD role-tenders [C2];
   - `aliased_label` ≥ 510 role-tenders.
4. **Sweep.** `sweep-orphan-orgs` (wet). Expected ≥ about 2,300 orgs: the label rows left with 0 mentions.

## Unit 4 — Verify and read-back (S)

**Verify (the issue's).**
- 1223007 and 9327844 answer 404, or 0 mentions.
- 1166638 serves no role but `buyer`: it is the buyer on 6 Tenders, which is F1's shape. Its `mention_count` goes
  from 1,356 to ≤ 10.
- **Amend the done line to:** "no served party other than a buyer or a result party (winner, tenderer) binds a bare
  department label shared by unrelated buyers; label-as-buyer and label-as-result rows are F1's." [C7]
  - Step 3 keeps a label section that a result names, and that section keeps minting the pooled row. s10 finds 4
    `Tenderer` rows on 1164742 "Servicio de Contratación".
  - The pre-challenge line ("no served NON-BUYER party …") would fail on those 4 rows.

**Read-back: random 30 of the affected mentions at 0 W.**
- Salt `587-u4-`. The sample is the first 30 (notice, section) cohort rows whose census verdict is alias or drop,
  ordered by `sha256(salt + notice_id + section_id)`.
- Each case is read on the served Tender of that notice (public GET, 3 s apart, never retry 408/429) and against the
  notice.
- **R.** Every role the label carried now binds the organization of the notice's own buyer section (or the
  touchpoint's parent), and the notice is consistent with the label being that body's unit (same publisher, address
  or town). Or the role is absent by a D587-2 drop rule, and no other party took its place.
- **N.** Served as designed with nothing wrong added: kept by steps 1–3. Or **inherited**: the alias landed on the
  buyer section correctly, but that section's own binding is wrong by an open issue, such as F2's S4111001F → 2660.
  An inherited case is listed with its issue and does not count toward 587's bar.
- **W.** One of these:
  - the role now binds a body the label is not a unit of, per the notice (another member of a joint notice, a
    coordinating body's unit bound to a member, a different buyer);
  - a real body's mention was retired or aliased away;
  - a label mention still binds a pooled label org after the sweep.
- **Bar: 0 W.**

**Coordination stratum, 10 more at 0 W.** Salt `587-u4-coord-`. These are aliased cases whose label is `zentrale
vergabestelle`, `vergabestelle`, `direction|service de la commande publique`, `service des achats` or
`direction des achats` [C8]: the shapes where a shared or district unit is likeliest.
- `direction des achats` was missing. It covers a French GHT member printing the support hospital's purchasing
  direction bare, and a ministry printing the interministerial DAE bare.
- 1340282 "Direction des achats" is a W hub: 20 buyers in 99 sampled Tenders, and the label is itself the buyer on
  10.
- At least 2 of the 10 picks come from `direction des achats` when the cohort has them.

**A W is fixed, not frozen on** (CLAUDE.md). Narrow the word or the guard that caused it, refold the census's
affected subset, re-read those cases, and banner only if served data got worse in the meantime.

Then close 587 with `Status: **DONE <date>**`.

## Stays for later (to file; none blocks 587)

- **F1 — a bare label printed as the buyer of record, or named by a result.**
  - Sizes: DÖE 19553, 291 Tenders with no other party, whose titles name Wuppertal, Leverkusen and others. TED
    labels as buyer: 2997642 27+1, 1804271 8+7, 1340282 10, 1166638 6, 1416875 6, 3443 3, 2680514 2, 1363144 1.
    DÖE 55205 "Vergabestelle" (NULL country) is buyer on 14 (census 2781 rows).
  - Label as a result: 1164742 `Tenderer` 4 (s10) [C7].
  - Candidate fixes: key such a mention by (label, town, country), or promote a named body that the notice prints
    elsewhere. The second is 483's promotion territory, which is its own decision.
  - 587's done line excludes this shape.
- **F2 — S4111001F.** The Junta de Andalucía's shared NIF binds Consejería buyers to org 2660, TARC Andalucía: 118
  buyer rows beside 1193769 alone (s08). It belongs to the 572/584 family. D587-2 inherits it until it is fixed.
- **F3 — `name_phrase_census` row order.** Sort by mentions DESC before `take(cap)` (canonical.rs:38980). This rides
  unit 2.
- **F4 — residue.** 180 listed orgs / 266 mentions: building words, "… centralisés", labels with a leading article
  or code, and the 3 trailing-connector names [C3]. Counted, not gated. Add words only through MUST_KEEP plus the
  fixture.
- **F5 — 585 (TenderNed dashboard prose on the ORG path).** A sibling drop class (`not_a_name`), not a label. It
  shares unit 3's refold if it is deployed together.
- **Unchanged N (D576-1's accepted cost):**
  - DEPT rows: a buyer's unit under its own name, 2,712 orgs / 35,856 mentions.
  - OTHER_ORGAN "Mesa / Órgano de contratación de X": 475 / 11,565.
  - Identified label rows: 333 / 1,209.

## Requests (this unit's decision step)

- SQL 8 `s07_multibuyer`, 9 `s08_tarc` and 10 `s09_buyer_notice` (of 12). Each was plan-probed index-only on both
  schema copies: `tender_version_parties_org`, `tender_version_parties_version`, the `tenders` rowid, and the
  `organization_mentions` PK / `organization_mentions_notice`.
- All three were sent on stdin through `tools/sq.py`, 0.38–0.42 s each, with no truncation and no 408.
- The statement, plan and answer files are `raw/sql/s07*`, `raw/sql/s08*` and `raw/sql/s09*`; the log is
  `requests.log`.
- GETs: 0 (20 of 20 used by the measurer).
- **The skeptic: SQL 11 `s10_hub_roles` and 12 `s11_sprovider_sections` (12 of 12; none left).**
  - Both were sent on stdin through `tools/sq_skeptic.py`, a copy of `sq.py` that keeps its plan-DB copies and
    `.stmts` under `raw/plan_skeptic/` and shares `raw/sql_state.json` and the budget.
  - Plans were index-only on both copies: `tender_version_parties_org`, the `tenders` rowid,
    `tender_version_parties_version` and the `organizations` rowid. On the prod copy the planner picks
    `tender_version_parties_org_role`, still an `organization_id` prefix seek.
  - Each took 0.68 s and 0.38 s; neither was truncated and neither got a 408.
  - Files are under `raw/sql/s10*` and `raw/sql/s11*`. GETs: 0.

## Challenge (skeptic, 2026-10-24 ~22:40Z)

**What was read.**
- This decision, `measure.md` / `measure.json`, issues 587, 586 (D2 lists, and `touchpoint_names_586.rs` WHOLE,
  BARE_LABELS and DEPT_OR_REVIEW), 576 (D576-1 (i)–(v) and the accepted cost) and 585 (the ORG path).
- `tools/label.py`.
- Census 2781's stored report: 500 capped rows, `roles` and `by_phrase`.
- The saved answers s01–s09.
- project.rs's role tables: `legacy_role`, `GUARD_SIGNATORY_ROLES`, role_census `ROLE_KINDS`.

**Requests, budget and files.**
- New reads: s10 (the role profile of all 195+3 name-keyed hub rows' served versions) and s11 (2997642's SProvider
  sections and their buyers). SQL is at 12 of 12, with 0 GETs left (the measurer used 20 of 20).
- Oracle changes in `tools/label.py`:
  - the trailing-connector guard and `COORD` / `coord_label`;
  - selftest 124/124;
  - `label.out`, `label.out.json` and both fixtures regenerated;
  - the old copies in `raw/pre_challenge/`.
- Every distinct folded hit name is listed in `raw/skeptic_hits_by_name.txt` (141 names, read one by one).

### Applied (confirmed): the decision text above carries each one, marked `[C#]`

- **C1 — A service-provider reference is a fact about the SECTION, not only the role.**
  - The decision dropped the `Procedure-SProvider` row "because that role names another organization by
    definition", yet aliased the same section's AddInfo, TenderReceipt and DocProvider to the buyer.
  - s11, on 2997642 "Zentrale Vergabestelle": 83 served label sections carry `Procedure-SProvider`, and 78 of them
    carry 1–4 other roles.
  - The buyers on those Tenders: Landkreis Wunsiedel 51. On about 32 the buyer is another body: Kirchenlamitz 10,
    the Verein "Hilfe für das lernbehinderte Kind im Landkreis Wunsiedel" 17 (in 11 spellings), Markt Schirnding,
    Markt Thiersheim, KUFi, a Fichtelgebirge GmbH and Gemeinde Odenthal.
  - That is the Landkreis's Zentrale Vergabestelle working as a real procurement service provider for bodies in its
    district. The town guard can separate the municipalities, but not the Verein, KUFi or the GmbH when they sit in
    the Kreisstadt.
  - Fix: step 6a drops every role of a section that any `*-SProvider`, `purchasing-body` or `Procedure-CPB`
    reference names. T7 is rewritten.
  - Cost: about 180 roles on the Landkreis's own 51 Tenders become omissions instead of aliases. That is safe by
    D576-1's ranking.
- **C2 — `centralizada` is a coordinating-body word in FUNC.** The decision kept `central` and `centrale` out on
  purpose, yet listed `centralizada`.
  - 1378491 "Servicio de Contratación Centralizada" (113 mentions, a W hub: 12 buyers / 23 ids) is the central
    purchasing service of a regional Hacienda department. The listings print its own spellings as "… Consejería de
    Hacienda y Administración Pública" and "… Dirección General de Contratación".
  - Its served buyers include other departments in the same capital: Consejería de Educación 23, Extremadura
    Consejería de Fomento 6, Consorcio de Gestión de Servicios Medioambientales de Badajoz 3 (of 76 sampled,
    `measure.json`).
  - The town guard cannot separate them. Aliasing them is the decision's own W ("a coordinating body's unit bound to
    a member").
  - Removing the word would leave the W hub pooled. So it stays a label but is drop-only: the `COORD` set, step 6b,
    D587-3, `dropped_coord`, T18.
  - Hits: 2 orgs / 117 mentions.
- **C3 — A trailing connector swallows acronyms and cut names.** CONNECT is a union across languages.
  - "MESA DE CONTRATACIÓN DEL ET" (ET is the Ejército de Tierra, a real organ) hit because `et` is the French
    connector.
  - "Servicio de Contratación del Área de" and "Direction de la commande publique et d" are cut names whose body word
    was lost.
  - Fix: a name whose last folded token is a CONNECT token is not bare. 3 orgs / 20 mentions flip to not-bare, and
    no MUST_HIT ends on a connector. They are now in MUST_KEEP, and T19 covers them.
- **C4 — `Contract-Signatory` is buyer-side, not a result.** project.rs `GUARD_SIGNATORY_ROLES` reads it as "the
  buyer that signs the contract".
  - In step 3 it kept the label section minting the pooled hub for ALL its roles on that notice.
  - Fix: results are `RoleKind::Contractor` + `bid:tenderer`. Signatory, `LotResult-Paying` and
    `LotResult-Financing` alias at step 7. T20 covers it.
  - On the hubs (s10): 1 signatory Tender (1804271) and 0 Paying/Financing.
- **C5 — `purchasing-body` and `Procedure-CPB` name another organization by definition**, like SProvider.
  `legacy_role`: "NOT this notice's buyer".
  - Aliasing such a role to the buyer would serve the buyer as purchasing on its own behalf.
  - They join the step-6a set. On the hubs' served rows they are 0 (s10); census 2781 counts 67 and 2 Tenders across
    all phrase orgs. Latent, and cheap to close.
- **C6 — D587-3 lacked D587-2's carve-outs.** A TPO label named by `Procedure-SProvider` would alias to its buyer
  parent. The same 6a/6b drops now apply on the touchpoint tier (`dropped_label`, T21). 0 measured; latent.
- **C7 — The done line contradicted step 3.** A label section that a result names (1164742 `Tenderer` on 4 Tenders,
  s10) keeps minting the pooled row. So "no served NON-BUYER party binds a pooled label" would fail Verify.
  - The done line now excludes label-as-result as well. F1 lists the 4 rows, and 55205 joins F1's label-as-buyer
    sizes.
- **C8 — The coordination stratum missed `direction des achats`.**
  - 1340282 is a W hub: 20 buyers in 99 Tenders, 10 as buyer.
  - It is the shape of the GHT support hospital and of the DAE.
  - It is added, with at least 2 of the 10 picks.

### Refuted, with the evidence

- **R1 — "Review and mediation roles alias the buyer as its own review or mediation body."** On the hubs:
  `review-body` 251, `Lot-ReviewOrg` 203, `mediation-body` 138 and `Lot-Mediator` 9 served rows, 601 of 13,293
  (4.5%, s10). Refuted as a defect:
  - D576-1 (i) and 586's tier C (DONE, read-back 30/30 R) already serve the buyer in `Lot-ReviewOrg` when the notice
    names the buyer's own unit or organ there;
  - a bare contracting-unit label names no tribunal ("Órgano / Unidad / Servicio de Contratación", "Procurement
    Department");
  - the Spanish recurso de reposición lies with the órgano de contratación;
  - the 530/576 (a) W was a section that WAS another body.

  Kept as an informing number (`aliased_by_kind`, review share expected about 4.5%) so the census shows it.
- **R2 — "H1's 15,000-notice floor is sized in mentions."** s03 already counted distinct notices: 0.906 per mention
  on the measured labels, so the listed floor is about 17.7k notices, about 15% above the bar. The H1 text now says
  so.
- **R3 — "The list catches real or national bodies."**
  - The 141 distinct folded hit names over 6,010 listed orgs were read: no national body beyond C2 and C3.
  - UZP is "Urząd …", and `urzad` is not a unit. DAE and SAE keep `etat`; Junta, Central, CUC and SUA are kept;
    "Dirección General …" keeps `general`.
  - Census 2781's 500 rows hold 4 hits, all labels: 3443; 8981 (identified, kept); and 19553 and 55205 (label as
    buyer, F1).
- **R4 — "A cheaper option exists."**
  - Re-keying label mentions per (label, buyer org) in the resolver: it mints thousands of rows, and the buyer org is
    unknown at resolve time.
  - An org-level remap of a hub: impossible, because the hubs are multi-buyer.
  - Folding only country-keyed sections: no code is saved, and 2,117 one-mention junk rows stay.
  - (a) and (d) stay rejected on the measured numbers. The per-notice rule is pure and local.
- **R5 — "Aliasing the NULL-country singletons (2,117 / 2,930) adds risk without removing pooling."** It is the same
  rule with the same plain reading (the notice's own unit), and the same 6/6a/6b drops apply. The sweep removes junk
  rows that each serve a label as a party name. No new W shape.
- **R6 — "Legislation-information roles name distinct bodies."** There are 7 served rows on the hubs (3975457
  `Lot-EnvironLegis` and `Lot-EmployLegis`, s10). A bare procurement unit printed there is the buyer's unit by the
  notice's reading, so aliasing is fine.
- **R7 — "The town guard creates false splits."** "A Coruña" vs "La Coruña" and "München" vs "Muenchen" fail the
  prefix test because articles and transliterations are kept. The outcome is a drop, which is an omission and not W,
  and it is measured by the informing town-drop share (< 10%). Not a defect; a candidate refinement only if that
  share surprises.
- **R8 — "The F2 inheritance worsens served data."** On 1193769, `appeal-information` will bind TARC Andalucía (org
  2660) through the buyer section on about 117 Tenders. Today it binds a pooled label org. Both are wrong. The
  census's F2-size number and the read-back's "inherited" class already keep it out of 587's bar, and F2 fixes both
  on the next refold.
- **R9 — "`zentrale` and `mutualise(e)` are coordinating words too."**
  - A city's own "Zentrale Vergabestelle" is the common case. The Kreis-for-Gemeinde case is caught by the town guard
    and, where the notice files it as a provider, by C1. The coordination stratum samples it.
  - A "service … mutualisé" is co-owned by the notice's buyer (Ville and EPCI), so aliasing to that buyer is R (cf.
    1165758 Strasbourg, N).

### Headline after the challenge

The decision is still **option (b)**: one whole-name label test aliases a bare contracting-unit label to the notice's
own single buyer (a touchpoint: its parent). The challenge narrowed it in three ways:
- a label that the notice itself files as a service provider, purchasing body or CPB drops ALL its roles (C1, C5,
  C6);
- a `centralizada` label is drop-only (C2);
- a name that ends on a connector is never a label (C3).

Signatory, Paying and Financing are buyer-side, not results (C4). The done line excludes label-as-result as well as
label-as-buyer (C7). The coordination stratum adds `direction des achats` (C8).
