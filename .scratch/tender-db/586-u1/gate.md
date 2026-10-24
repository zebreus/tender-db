# 586 unit 1 (b): touchpoint gate stems (drop, never mint)

Local analysis only (2026-10-24): no network, no box, no `/v1/sql`, no cargo, no git. Inputs are the saved answers under
`audit-576-fanout/` (`second-look/raw/t2_classified.json`, `03-`/`05-*-classified.json`, `04-t4_served.json`;
`refute/clusters.json`, `raw/03`, `raw/05`, `raw/06`; `box/raw/09`, `box/raw/17`), `576-triage/r3.out` and
`576-u4/raw/sql/s0*.out`. Scripts: `pop.py` (populations, a byte-for-byte Python copy of `partyname::fold`),
`gate.py` (the stems, the bars, every hit; `python3 gate.py --hits > gate.out`), `table.py` (the judged table below,
`table.md`), `persons.py` (bare personal names, counted and not gated), `explore.py` (the first broad probes).

## Decision (proposed)

Five lists, all touchpoint-only, read by `touchpoint_gate` and nothing else. A touchpoint is dropped only when EVERY
name it publishes is gated (today's `all(dropping)`), and the gate still runs after the (i) alias check, so a label
that equals its own parent's name still aliases.

| # | Mechanism | Exact test | Stems |
|---|---|---|---|
| A | **WHOLE** label | `partyname::fold(name)` equals an entry | verdict: `principal`, `sede tibidabo`, `direccion principal`, `info adicional`, `recursos`, `direccion postal`, `direccion recepcion de recursos`, `palacio provincial`, `direccion recurso`, `ayuntamiento`, `procurement department`. Found in the same data: `direcci n principal` (the `Direcci¿n` mojibake), `direccion de recursos`, `presentacion de recursos` (a hub: Valdemorillo and Ineco), `direccion informacion adicional`, `informacion adicional`, `contact`, `beschaffende stelle` (a hub: 4 DB companies), and three eForms role labels pasted as names: `body responsible for mediation procedures`, `service from which information about the review procedure may be obtained`, `organizacion que proporciona informacion adicional sobre el procedimiento de contratacion`, plus `direccion para presentacion de recursos` |
| B | **HONORIFIC** lead | The trimmed raw name's first whitespace token, compared case-insensitively, is one of `monsieur madame mademoiselle mme mme. mlle mlle. herr frau dott dott. dott.ssa dottssa sig. sig.ra ing. m. mr. mrs. ms.`, or is exactly `Mr` or `Mrs` (case-sensitive). More text must follow. `m.` does not count when the next token is one letter plus a dot (`M. I. Ayuntamiento`). Guards: `!role_census::has_commercial_form(name)` and no public-law stem (`role_census::public`, which needs to become `pub(super)`) | the list |
| C | **PROSE** phrase | A whole-word phrase anywhere in the folded name (the `" {phrase} "` padding `name_pattern` uses) | `procede recurso`, `ver apartado`, `en los lugares`, `lugares establecidos`, `lugares indicados`, `lugares previstos`, `lugares habilitados` |
| D | **LEGALREF** | Remove every `(…)` span first (innermost first, repeated). Then the test passes on any of: a raw `art` + `.` or `-` + a digit, where `art` follows a non-alphanumeric character (`art. 16`, `art- 51.3`); a folded `articulo`/`articulos`/`artigo`/`articolo`/`artikel` token followed by a token that starts with a digit; or folded `el`/`del`/`y`/`en`/`segun`/`al` + `art` + a digit token (`el art 44`, `y art 16`) | the rule |
| E | **PLATFORM** phrase | As C | verdict: `tuttogare`, `plataforma electronica de contratacion`. Found: `platforma zakupowa`, `platforma przetargowa`, `e zamowienia`, `ezamowienia`, `logintrade`, `achizitii md` |

Why each mechanism is shaped this way:
- **Labels are whole-name only.** The word alone is everywhere. In these populations, `recursos` sits inside 429
  kept names with weight 8,504: every "Tribunal … de Recursos Contractuales". `ayuntamiento` sits inside 615 kept names
  (3,188) and `principal` inside 6 ("Sede principal del Patronato"). The verdict's own risk cases, "Principal
  Financial" and "Palacio de Congresos", would need a phrase match to be hit. No other mechanism carries that risk:
  prose and platform phrases do not occur in a body's name.
- **Honorifics are matched on the raw lead, never as a phrase.**
  - The dot carries the meaning. Folded, `m`, `ing`, `sig` and `ms` lead real names ("M-30", "ING Bank", "SIG
    Combibloc"), so those four count only with their dot.
  - The legal-form guard keeps "Ing. Büro SoFah GmbH & Co. KG", a real engineering firm and a hit without the guard.
    The public-stem guard keeps "M. I. Ayuntamiento de …".
  - Left out on purpose:
    - `Dr.`: "Dr. Bauer & Partner" is a law firm, and "Dr. Negrín" is in a hospital's name.
    - `D.`/`Don`/`Doña`: "Valencia de Don Juan", "Puebla de Don Fadrique", initials.
    - `Excmo.`/`Ilmo.`: they lead bodies ("Excmo. Ayuntamiento de Algeciras").
    - Mid-name titles ("Direction des opérations … Monsieur Nicolas BROSSIER"): see Residue.
  - Uppercase `MR`/`MRS` without a dot does not count ("MRS Logística").
- **Legal references are read outside brackets, and need a dot or a Spanish article before `art`.**
  - A bracketed reference annotates a body's real name: "Tribunal Administrativo Central de Recursos Contractuales
    (Según artículo 50.1.a) y b) de la LCSP)" binds the older row 22186722 in both windows, and stays.
  - A bare "Art 21" can lead a gallery's name.
  - `article` (EN/FR/CA) is left out because of the NGO "ARTICLE 19".

**Where they live: a new block in `crates/ingest/src/project.rs` beside `touchpoint_gate`, not `NAME_PATTERNS`.**
- `NAME_PATTERNS` is phrase-anywhere only, through `name_pattern` and its padded copy. Whole-name (A) and anchored-lead
  (B) tests would need a match-mode column on all 77 rows. D is a rule, not a phrase.
- Every `NameList` kind already has readers outside the gate:
  - `ReviewBody`: `judge`'s review-body classes, the census's pattern tallies, `review_or_platform_name` (the cheap
    gate of `buyer_fix`), and `promotable`.
  - `Platform`: `judge`'s decisive `platform-name` class, `review_or_platform_name`, and `portal_label` (so
    `promotable` and the gate).
  - `Portal`: `portal_label`, so `promotable` (buyer_fix's promotion refusal) and the gate.
  - So TUTTOGARE or `platforma zakupowa` in `Platform` would make `platform-name` decisive on such a buyer and move the
    census and `buyer_fix`. In `Portal` they would change `promotable` picks, and with them served buyers on any
    notice refolded later.
- D576-1 (v) keeps touchpoint mentions out of `buyer_fix` and the census, so a touchpoint-only list is isolated by
  construction. A fourth `NameList` read by no census path would also work for C and E, but not for A, B or D.
- The new lists reuse `crate::partyname::fold`, which is already the census fold, `role_census::has_commercial_form`
  (already `pub`), and `role_census::public` (make it `pub(super)`). A test should hold each list entry to its own fold,
  as `the_name_patterns_are_folded_whole_word_phrases` does.
- Promoting the platform stems into `NameList::Portal` later is its own change, with its own `promotable` census.

## Hit counts

New orgs: the 3,751 minted 31974972–31978722, 24,664 touchpoint mentions. An org can meet two mechanisms.

| Mechanism | New orgs | Mentions | Verdict list alone |
|---|---:|---:|---:|
| A WHOLE | 21 | 3,104 | 11 / 2,940 (the 9 labels 2,926 + "Direccion Principal" 13 + Procurement Department 1) |
| B HONORIFIC | 8 | 11 | 8 / 11 |
| C PROSE | 36 | 229 | all |
| D LEGALREF | 36 | 172 | all |
| E PLATFORM | 12 | 74 | 4 / 51 |
| **All** | **92** | **3,491 (14.2%)** | **74 / 3,304** |

By what was dropped: 21 labels (3,104), 8 people or office holders (11), 12 platforms (74), 31 pure prose (165), and 20
prose or compound names that also name a tribunal (137, judged in the T note below).

Other populations (`gate.out`):

| Population | Names | Hit names | Hit weight / all |
|---|---:|---:|---:|
| Window A (all touchpoint mentions, existing and new rows) | 1,164 | 14 | 27 / 2,081 |
| Window B | 533 | 8 | 15 / 904 |
| R3 top 60 (576 unit 1) | 60 | **0** | 0 / 589 refs |
| Refute's 125 loose clusters (274 members) | 274 | 16 | 620 / 2,801, all in class N |
| 576 U4 served touchpoint parties (6 windows) and its mention read | 330 | **0** | 0 / 449 |
| Older rows 2759 bound (s09, t4) and the same-name twins (r5) | 161 | 5 | "Principal", "Dirección Principal", **1689043 "Ayuntamiento" (+125 from 2759)**, two r5 twins (prose, "presentación de recursos") |

In the windows, the gate unbinds only these older rows: 1689043 "Ayuntamiento" and three older prose-named rows
(13863806 "En los lugares establecidos…", 17553080 "Procede recurso especial…", 18120239 "Sede del TACRC o Registro
General del Ayuntamiento de Siero o Lugares habilitados…"). Every other window hit bound a new org.

## Bars

1. **R3's top 60 (D576-1 (iv)'s bar):** the new stems hit 0 of 60 rows. Today's gate still drops rows 21, 34, 45, 47 and
   55: the four TenderNed prose strings and ORG-0001, 25 refs, unchanged. Every R3 tribunal and both of its
   non-tribunal names ("Stadt Fürth - Zentrale Vergabestelle", "Agentur Wieseke") are minted as before.
2. **Class X (refute):** refute saved its clusters but not its labels, so I rebuilt the classes from its stated totals.
   - **Class N matches exactly: 18 clusters, 37 orgs, 853 mentions** (clusters 8, 29, 41, 51, 54, 55, 78, 86, 87, 89,
     90, 91, 92, 106, 107, 118, 119, 120).
   - For X I used a superset: every other cluster whose key names a tribunal, juzgado, Vergabekammer,
     Vergabeprüfstelle, médiateur, competition agency, αρχη, "recursos contractuales" or TARCAS. That is 28
     clusters, 62 orgs and 582 mentions, against X's 23 / 50 / 340. **0 of its members are hit.**
   - All 16 cluster hits fall in 8 N clusters (51, 86, 87, 106, 107, 118, 119, 120). The verdict lists these as defects:
     Dirección Principal, the Port of Barcelona platform, "Procede recurso…", "TACRC y art. 16 Ley 39/2015" ×2 and
     "Ver apartado 17 de la HRC…" ×3.
   - 0 hits among the other tribunal rows in the saved data either: s09 (45 older rows bound), t4 (19 served rows), and
     U4 (330 served parties and mentions).
3. **Real organizations:** `gate.py`'s `MUST_KEEP` has 55 names, all kept. Among them:
   - the risk cases: Principal Financial Group, Principal Global Investors, three Palacios de Congresos/Festivales,
     Dirección General de Recursos Humanos, Procurement Investment International;
   - the guard cases: Ing. Büro SoFah GmbH & Co. KG, ING Bank, M-30, MRS Logística, Dr. Bauer & Partner, Mr. Bricolage
     SA, Herrera de Pisuerga, Frauenklinik, M. I. Ayuntamiento de Elche, ARTICLE 19, Art 21 Gallery, Plataforma
     Oceánica de Canarias, Plataforma Logística de Zaragoza;
   - the D576-1 test tribunals, TARCAS, Bundesverwaltungsgericht, and the bare "Rechtbank".

   `MUST_DROP` has 35 names, all dropped. The real-data containment counts under "Why each mechanism…" above are the
   same evidence from production names.

**T, the one judgment the decision makes.** 20 dropped new orgs (137 mentions, 0.56% of the new-org mentions) name a
tribunal inside prose or a compound:
- "TACRC y art. 16 Ley 39/2015" ×8;
- "TRIBUNAL ADMINISTRATIVO CENTRAL … DE ACUERDO CON EL ART 44 DE LA LCSP, SALVO QUE SEAN…" (13);
- "Ver apartado 17 de la HRC - Tribunal … de Galicia" ×2 (47);
- "Tribunal … de Galicia artículo 44 / 51.3";
- "Sede del TACRC o Registro General del Ayuntamiento de Siero…";
- "Cualquier lugar habilitado … o sede del TACRC";
- "TAR territorialmente competente ai sensi dell'art. 120" (no single body).

None is class X. The ones refute clustered are class N. Dropping loses that lot's review role, which is counted in
`dropped` and so not silent. Minting keeps a prose-named duplicate of TACRC/TACGal that no fold ever merges (the
verdict's "Permanent" section). A drop can be undone by a later refold, for instance one that strips the prose and
re-mints; a minted junk row cannot. So: drop. The rejected variant exempts any name holding a tribunal word from C and D,
and would keep these 20 orgs.

## Every new-org hit, judged

`list`: v = the verdict's candidate list, f = found in the same data. Judgments: *label, no body* (a contact or address
label, or an eForms role label); *person*; *platform / portal label*; *prose*; *prose/compound naming a tribunal (T)*,
see T above. All 92 are correct drops under the decision. No hit is a body that should have been minted.

| mentions | org | name | mechanism | list | parent (first) | judgment |
|---:|---|---|---|---|---|---|
| 715 | 31975008 | Principal | WHOLE | v | Gerencia umivale Activa, Mutua Colaboradora c | label, no body |
| 621 | 31975019 | Sede Tibidabo | WHOLE | v | Mutua Universal Mugenat, Mutua Colaboradora c | label, no body |
| 420 | 31974991 | Dirección Principal | WHOLE | v | Distrito de Usera | label, no body |
| 294 | 31975180 | Info Adicional | WHOLE | v | Empresa Municipal de Transportes de Madrid, S | label, no body |
| 268 | 31975009 | RECURSOS | WHOLE | v | Consejo de Administración de EMAYA, Empresa M | label, no body |
| 221 | 31975006 | Dirección Postal | WHOLE | v | Consejo de Gobierno Insular del Cabildo de Gr | label, no body |
| 143 | 31975004 | Dirección Recepción de Recursos | WHOLE | v | Rector de la Universidad Carlos III de Madrid | label, no body |
| 139 | 31975059 | Palacio Provincial | WHOLE | v | Presidencia de la Diputación Provincial de Cá | label, no body |
| 105 | 31975568 | Dirección Recurso | WHOLE | v | Gerencia de la Universidad de Alcalá | label, no body |
| 53 | 31975060 | Beschaffende Stelle | WHOLE | f | DB Netz AG (Bukr 16) | label, no body |
| 46 | 31975069 | Plataforma Electrónica de Contratación de la Autoridad Portuaria de Barcelona (PORT DE BARCELON | PLATFORM | v | Presidencia del Consejo de Administración de  | platform / portal label |
| 42 | 31975984 | Ver apartado 17 de la HRC - Tribunal Administrativo de Contratación Pública de la Comunidad Aut | PROSE | v | Rectorado de la Universidad de Vigo | prose/compound naming a tribunal (T) |
| 36 | 31976056 | Ver apartado 17 de la HRC. Rector de la Universidad de Vigo | PROSE | v | Rectorado de la Universidad de Vigo | prose |
| 30 | 31975949 | Presentación de recursos | WHOLE | f | Junta de Gobierno Local del Ayuntamiento de V | label, no body |
| 28 | 31975513 | Body responsible for mediation procedures | WHOLE | f | PHI Clinical Hopspital Shtip | label, no body |
| 26 | 31975564 | Service from which information about the review procedure may be obtained | WHOLE | f | PHI Clinical Hopspital Shtip | label, no body |
| 25 | 31976055 | Ver apartado 17 de la HRC - Rector de la Universidad de Vigo | PROSE | v | Rectorado de la Universidad de Vigo | prose |
| 17 | 31975872 | Tribunal Administrativo Central de Recursos Contractuales y art. 16 de la Ley 39/2015 | LEGALREF | v | Junta de Contratación del Ministerio de Indus | prose/compound naming a tribunal (T) |
| 15 | 31976686 | En los lugares indicados en el artículo 51.3 del LCSP | LEGALREF,PROSE | v | Dirección del Instituto Nacional de Gestión S | prose |
| 14 | 31974977 | En los lugares indicados en el artículo 51.3 de la Ley 9/2017, de 8 de noviembre, de Contratos  | LEGALREF,PROSE | v | Dirección del Instituto Nacional de Gestión S | prose |
| 13 | 31975075 | Direccion Principal | WHOLE | v | Junta de Gobierno del Ayuntamiento de Leganés | label, no body |
| 13 | 31975579 | TRIBUNAL ADMINISTRATIVO CENTRAL DE RECURSOS CONTRACTUALES DE ACUERDO CON EL ART 44 DE LA LCSP,  | LEGALREF | v | Junta de Gobierno Local del Ayuntamiento de V | prose/compound naming a tribunal (T) |
| 12 | 31977524 | Tribunal Administrativo Central de Recursos Contractuales y art. 16 Ley 39/2015 de 1 octubre | LEGALREF | v | Junta de Contratación del Ministerio de Indus | prose/compound naming a tribunal (T) |
| 11 | 31975206 | En los lugares establecidos en el artículo 16.4 de la Ley 39/2015, de 1 de octubre, del Procedi | LEGALREF,PROSE | v | Alcaldia del Ayuntamiento de Muro de Alcoy | prose |
| 10 | 31977536 | Tribunal Central Recursos Contractuales Madrid, Ayuntamiento de Manacor y lugares establecidos  | LEGALREF,PROSE | v | Alcaldía del Ayuntamiento de Manacor | prose/compound naming a tribunal (T) |
| 8 | 31976137 | Contact | WHOLE | f | Energie Data Services Nederland (EDSN) B.V. | label, no body |
| 8 | 31977949 | El escrito de interposición podrá presentarse en los lugares establecidos en el artículo 16.4 d | LEGALREF,PROSE | v | Gerencia de Viviendas Sociales e Infraestruct | prose |
| 7 | 31975189 | PLATFORMA ZAKUPOWA | PLATFORM | f | Krajowa Izba Odwoławcza | platform / portal label |
| 7 | 31978125 | Direcci¿n Principal | WHOLE | f | Área de Gobierno de Vicealcaldía, Portavoz, S | label, no body |
| 6 | 31975083 | Platforma Zakupowa Logintrade | PLATFORM | f | Wodociągi Miasta Krakowa S.A. | platform / portal label |
| 6 | 31977721 | Ayuntamiento de Manacor y lugares establecidos en el art 16.4 ley 39/2015 | LEGALREF,PROSE | v | Consejo de Administración del Patronat Munici | prose |
| 5 | 31975751 | Dirección de Recursos | WHOLE | f | Empresa Municipal de la Vivienda y Suelo de M | label, no body |
| 5 | 31976908 | Ver apartado 17 de la HRC. Tribunal Administrativo de Contratación Pública de la Comunidad Autó | PROSE | v | Rectorado de la Universidad de Vigo | prose/compound naming a tribunal (T) |
| 5 | 31977109 | En los lugares establecidos en el artículo 16.4 de la Ley 39/2015 de procedimiento administrati | LEGALREF,PROSE | v | Departamento de Salud de Xàtiva-Ontinyent. Di | prose |
| 5 | 31977330 | Lugares previstos en el artículo 51.3 de la Ley 9/2017 | LEGALREF,PROSE | v | Junta de Gobierno del Ayuntamiento de Siero | prose |
| 5 | 31977541 | En los lugares indicados en el artículo 51.3 de la LCSP | LEGALREF,PROSE | v | Dirección del Instituto Nacional de Gestión S | prose |
| 4 | 31975438 | Monsieur Anthony LINTILHAC | HONORIFIC | v | LA CIOTAT SHIPYARDS | person |
| 4 | 31976493 | Tribunal Administrativo de Contratación Pública de Galicia artículo 44 | LEGALREF | v | Pleno del Ayuntamiento de Vilarmaior | prose/compound naming a tribunal (T) |
| 4 | 31976494 | Tribunal Administrativo de Contratación Pública de Galicia artículo 51.3 | LEGALREF | v | Pleno del Ayuntamiento de Vilarmaior | prose/compound naming a tribunal (T) |
| 4 | 31976705 | Tribunal Administrativo Central de Recursos Contractuales y el art. 16 de la Ley 39/2015 | LEGALREF | v | Junta de Contratación del Ministerio de Indus | prose/compound naming a tribunal (T) |
| 4 | 31978277 | Procede recurso de reposición y, en su caso, recurso contencioso administrativo | PROSE | v | Dirección del Instituto Nacional de Gestión S | prose |
| 3 | 31975575 | Sede del Tribunal Administrativo Central de recursos contractuales o Registro General del Ayunt | PROSE | v | Junta de Gobierno del Ayuntamiento de Siero | prose/compound naming a tribunal (T) |
| 3 | 31975614 | Portal e-zamówienia | PLATFORM | f | Szpital Powiatowy im. Prałata J. Glowatzkiego | platform / portal label |
| 3 | 31975615 | Sede del Tribunal Administrativo Central de Recursos Contractuales o Registro General del Ayunt | PROSE | v | Junta de Gobierno del Ayuntamiento de Siero | prose/compound naming a tribunal (T) |
| 3 | 31977061 | PLATAFORMA ELECTRÓNICA DE CONTRATACIÓN de la Autoridad Portuaria de Barcelona | PLATFORM | v | Presidencia del Consejo de Administración de  | platform / portal label |
| 3 | 31977162 | Cualquier lugar habilitado en el artículo 16 de la Ley 39/2015 o sede del Tribunal Administrati | LEGALREF | v | Alcaldía del Ayuntamiento de Siero | prose/compound naming a tribunal (T) |
| 3 | 31977213 | Procede recurso especial en materia de contratación, de acuerdo con lo establecido en los artíc | LEGALREF,PROSE | v | Dirección del Instituto Nacional de Gestión S | prose |
| 3 | 31977291 | TAR territorialmente competente ai sensi dell'art. 120 del D. Lgs. n. 104/2010 | LEGALREF | v | RAI - Radiotelevisione Italiana S.p.A. - A/TE | prose/compound naming a tribunal (T) |
| 3 | 31977540 | Procede recurso especial en materia de contratación, de acuerdo con lo indicado en los artículo | LEGALREF,PROSE | v | Dirección del Instituto Nacional de Gestión S | prose |
| 3 | 31978098 | Dirección información adicional | WHOLE | f | Distrito de Retiro | label, no body |
| 3 | 31978324 | VER APARTADO CORRESPONDIENTE PCAP | PROSE | v | Servicio Murciano de Salud - Gerencia | prose |
| 2 | 31975755 | Sede del Tribunal Administrativo Central / Registro Órgano de Contratación / o lugares habilita | PROSE | v | Concejalía Delegada del Área de Hacienda, Con | prose/compound naming a tribunal (T) |
| 2 | 31975885 | Tribunal administrativo central de recursos contractuales y artículo 16 Ley 39/2015 | LEGALREF | v | Junta de Contratación del Ministerio de Indus | prose/compound naming a tribunal (T) |
| 2 | 31976044 | Platforma eZamówienia | PLATFORM | f | Gmina Miasto Tomaszów Maz. | platform / portal label |
| 2 | 31976191 | Tribunal Administrativo Central de Recursos Contractuales y art. 16 Ley 39-2015 | LEGALREF | v | Junta de Contratación del Ministerio de Indus | prose/compound naming a tribunal (T) |
| 2 | 31976193 | Tribunal Administrativo Central de Recursos Contractuales y art. 16 de la Ley 39-2015 | LEGALREF | v | Junta de Contratación del Ministerio de Indus | prose/compound naming a tribunal (T) |
| 2 | 31976339 | Tribunal Administrativo Central de Recursos Contractuales y art. 16 Ley 39/2015 | LEGALREF | v | Junta de Contratación del Ministerio de Indus | prose/compound naming a tribunal (T) |
| 2 | 31976636 | El recurso se presentará en los lugares indicados en el artículo 51.3 de la LCSP | LEGALREF,PROSE | v | Dirección del Instituto Nacional de Gestión S | prose |
| 2 | 31976701 | Tribunal Administrativo Central de Recurso Contractuales y art. 16 Ley 39/2015 | LEGALREF | v | Junta de Contratación del Ministerio de Indus | prose/compound naming a tribunal (T) |
| 2 | 31977191 | Ver apartado 17 de la HRC. Recurso de reposición o alzada | PROSE | v | Rectorado de la Universidad de Vigo | prose |
| 2 | 31977474 | En los lugares indicados en el artículo 51.3 de la Ley9/2017, de 8 de noviembre, de Contratos d | LEGALREF,PROSE | v | Dirección del Instituto Nacional de Gestión S | prose |
| 2 | 31977754 | Procede recurso potestativo de reposición, según lo dispuesto en los artículos 123 y 124 de la  | LEGALREF,PROSE | v | Dirección del Instituto Nacional de Gestión S | prose |
| 2 | 31977848 | Cualquier lugar previsto en el artículo 16 de la Ley 39/2015 o sede del Tribunal Central de Rec | LEGALREF | v | Junta de Gobierno del Ayuntamiento de Siero | prose/compound naming a tribunal (T) |
| 2 | 31977938 | Achiziții.md | PLATFORM | f | Serviciul Special pentru influenţe Active asu | platform / portal label |
| 2 | 31977967 | Dirección para presentación de recursos | WHOLE | f | Consejera Delegada de Asistencia Integral Tri | label, no body |
| 2 | 31978285 | En los lugares indicado en el artículo 51.3 de la Ley 9/2017, de 8 de noviembre, de Contratos d | LEGALREF,PROSE | v | Dirección del Instituto Nacional de Gestión S | prose |
| 2 | 31978310 | Organización que proporciona información adicional sobre el procedimiento de contratación | WHOLE | f | Ayuntamiento de Usurbil | label, no body |
| 1 | 31975293 | DOTT. ANGELO GAMBARDELLA | HONORIFIC | v | Agenzia Locale di Sviluppo dei Comuni dell'Ar | person |
| 1 | 31975351 | PLATFORMA E ZAMÓWIENIA | PLATFORM | f | Miejski Zakład Komunikacyjny w Tomaszowie Maz | platform / portal label |
| 1 | 31975420 | M. Damien SIMON | HONORIFIC | v | APRR. | person |
| 1 | 31975504 | Platforma Przetargowa LOGINTRADE | PLATFORM | f | PARK NARODOWY GÓR STOŁOWYCH | platform / portal label |
| 1 | 31975923 | Monsieur MAHE Michael | HONORIFIC | v | Mairie de Mennecy | person |
| 1 | 31975928 | M. le Médiateur interne de l UGAP | HONORIFIC | v | Union des groupements d achats publics UGAP | office holder by title (UGAP mediator) |
| 1 | 31976973 | En los lugares indicados en el artículo 51.3 de la Ley de Contratos del Sector Público | LEGALREF,PROSE | v | Dirección del Instituto Nacional de Gestión S | prose |
| 1 | 31976984 | Secretaría General del ISCIII o lugares establecidos según art- 51.3 de la LCSP | LEGALREF,PROSE | v | Dirección del Instituto de Salud Carlos III | prose |
| 1 | 31977055 | Plataforma electrónica de contratación de la Autoridad Portuari de Barcelona | PLATFORM | v | Presidencia del Consejo de Administración de  | platform / portal label |
| 1 | 31977163 | En los lugares indicado en el artliculo 51.3 de la LCSP | PROSE | v | Dirección del Instituto Nacional de Gestión S | prose |
| 1 | 31977190 | Ver apartado 17 de la HRC - Recurso de reposición o alzada | PROSE | v | Rectorado de la Universidad de Vigo | prose |
| 1 | 31977395 | M. Kuiper | HONORIFIC | v | Sportbedrijf Deventer N.V. | person |
| 1 | 31977501 | Procede recurso especial en materia de contratación, de acuerdo con lo establecido en los artíc | LEGALREF,PROSE | v | Dirección del Instituto Nacional de Gestión S | prose |
| 1 | 31977599 | ING. PAMELA COMI | HONORIFIC | v | COMUNE DI PRESICCE-ACQUARICA | person |
| 1 | 31977602 | Lugares habilitados en el el art 16 de la Ley 39/2015 | LEGALREF,PROSE | v | Junta de Gobierno del Ayuntamiento de Siero | prose |
| 1 | 31977728 | Procede recurso de reposición de acuerdo con lo establecido en los artículos 123 y 124 de la le | LEGALREF,PROSE | v | Dirección del Instituto Nacional de Gestión S | prose |
| 1 | 31977731 | Procede recurso especial en materia de contratación, según lo dispuesto en los artículos 44 y s | LEGALREF,PROSE | v | Dirección del Instituto Nacional de Gestión S | prose |
| 1 | 31977732 | En los ligares indicados en el artículo 51.3 de la LCSP | LEGALREF | v | Dirección del Instituto Nacional de Gestión S | prose |
| 1 | 31978264 | Platforma Zakupowa Zamawiającego | PLATFORM | f | Radomskie Przedsiębiorstwo Energetyki Cieplne | platform / portal label |
| 1 | 31978420 | Procede recurso especial en materia de contratación (vigente lote 1). Procede recurso de reposi | PROSE | v | IGSS - Intervención General de la Seguridad S | prose |
| 1 | 31978431 | TUTTOGARE RACCONIGI | PLATFORM | v | Comune di Racconigi | platform / portal label |
| 1 | 31978475 | Procurement Department | WHOLE | v | Museros Town Council | label, no body |
| 1 | 31978493 | Procede recurso de reposicion o contencioso administrativo (lotes 1 y 2) | PROSE | v | IGSS - Intervención General de la Seguridad S | prose |
| 1 | 31978514 | ver apartado PCAP | PROSE | v | Servicio Murciano de Salud - Gerencia | prose |
| 1 | 31978637 | dott marco lesto | HONORIFIC | v | CUC UNIONE DEI COMUNI DI MONTEDORO | person |

92 orgs, 3491 mentions

The windows, the clusters and the older rows hold the same names, and no others (`gate.out`, section "Every hit").

## Residue: seen, not gated (counted)

- **Bare personal names, out of scope as decided:** 77 new orgs, 285 mentions, by hand from `persons.py`'s 296
  heuristic candidates (`persons.out`). Examples: Michaela Cechova 19, Melinda Wartan(-)Borbely 18+11, Miranda Vleeming
  16, Rob van der Kooi 15, Judith COMMINSOLI, "RUP Prete Fernanda", "Ten. Col. Lorenzo Mancini". This is a floor: the
  heuristic skips all-caps names and names of five or more words.
- **Titles in the middle of a name:** about 10 APRR rows ("DGA-IC - … - Monsieur Yvan Duranton.", "Pôle Chaussées, Mme
  Julie ROLLIN"), "Dirigente ing. Giuseppe Pavone", "Rechtsanwalt Prof. Dr. Holger Zuck" and "D. Jose Ignacio García
  Alonso". Refute's clusters 54/55 are the APRR ones. A phrase-anywhere `monsieur`/`mme` would catch them, but `frau`
  anywhere hits "Unserer Lieben Frau", so the lead-only rule stands.
- **Single-publisher building and site labels (N, one body each, not hubs):** "Hospital Real" 227 (Universidad de
  Granada's seat; whole-name would risk a real hospital), "Edificio San Francisco" 61, "Oficina LPGC" 22, "Sede
  GESPLAN", "Sede AEBOE", "Sede Central OAR", "Sede Fundació Mallorca Turisme".
- **Generic department names without their body (N under D576-1, a hub risk if they repeat across buyers):**
  "Financiën en Inkoop" 271, "Biuro Zakupów" 110, "Servicios Centrales", "Central de compras", "Oficina Contratación",
  "Servicio de Informática", "Personeelszaken", "Bedrijfsjurist". A later census could gate the ones that span parents.
- **Prose and pointers the stems miss (≤ 26 mentions in all):** "Ayuntamiento de Alovera/TARC/Lugares ar. 16.4 de la Ley
  39/2015" 24+2 (the typo `ar.`), "No se prevé conforme a la normativa" 3, "Conforme a lo indicado en la cláusula 45 del
  PCAP" 2, "INDICADO EN EL PUJTO" 2, "Véase pliego de prescripciones administrativas" 1, "Apartado T. del Cuadro resumen"
  2, "La fecha de presentación de recursos es orientativa…" 2, the Catalan "… i en els llocs establerts en l'article
  16.4" 3, and "Tribunales competentes … / Tribunal … (art. 44 LCSP)." (kept on purpose: bracketed).
- **Portals the stems miss:** "Sede electrónica …" ×8 names, where some are self (bracketed: half (a) aliases them) and
  some compound with a tribunal; "(En el) perfil del contratante …" ×3; "Asamblea de Madrid - Licitación Electrónica
  Plataforma"; "Piattaforma Asmecomm"; "Grupul de lucru pentru achizitii publice" (a working group, a body).

## Interactions for unit 2

- **Order.** Keep `same` (the alias) first, then `dropping`. If half (a)'s tier C aliases a bare "Ayuntamiento" under
  "Alcaldía del Ayuntamiento de X", that alias wins, and the WHOLE drop is the fallback.
  - The labels are by spec contact points of their parent, so aliasing them to it would serve AddInfo/TenderReceipt
    right. A ReviewOrg naming "RECURSOS" would then serve the buyer as its own review body (530's AOK shape). Drop is
    the role-independent choice.
- **All languages.** A touchpoint that prints "Dirección Postal" in ES and a non-label in CA still mints, as today's
  gate does. The saved data has first names only, so this is unmeasured.
- **Counter.** Everything gated adds to `TouchpointRoles::dropped`, with no new field. The drain verifies by reading the
  orgs.
- **Drain expectation (unit 3).** Refolding the Touchpoint carriers retires the mentions of these 92 new orgs (3,491
  mentions; the verdict list alone is 74 / 3,304), which `retire_mentions` or the sweep then removes. 1689043
  "Ayuntamiento" loses 2759's 125 touchpoint mentions, and stays with its own ORG mentions. The hubs 31974991,
  31975008 and 31975059 go to 0, and so do 31975006, 31975060 and 31975949.
- **Tests:**
  - drop: "Dirección Principal", "Principal", "Palacio Provincial", "Ayuntamiento", "Monsieur Anthony LINTILHAC",
    "DOTT. ANGELO GAMBARDELLA", "TACRC y art. 16 Ley 39/2015", "Ver apartado 17 de la HRC. Rector de la Universidad de
    Vigo", "TUTTOGARE RACCONIGI", "Platforma Zakupowa Logintrade";
  - keep minting: "Ing. Büro SoFah GmbH & Co. KG", "M. I. Ayuntamiento de Elche", "Principal Financial Group",
    "Palacio de Congresos de Valencia", "Tribunal Administrativo Central de Recursos Contractuales (Según artículo
    50.1.a) y b) de la LCSP)", "Art 21 Gallery", and every name in the existing
    `the_touchpoint_gate_drops_prose_section_ids_and_void_names_and_no_tribunal` keep-list;
  - each list entry is written as its own fold.
