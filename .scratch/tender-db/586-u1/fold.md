# 586 unit 1 (a): the touchpoint alias fold, tiers A and C

Local analysis only (2026-10-24): no network, no box, no `/v1/sql`, no cargo, no git. Inputs are the saved answers
under `.scratch/tender-db/audit-576-fanout/`: `second-look/raw/t2_classified.json` (the 3,751 new orgs, each with
the printed name of its first mention's parent ORG in the same notice; `refute/raw/04-r4_parent_one.json` names the
same parent for 3,751 of 3,751), `second-look/raw/03-t3_window_a.json` and `05-t5_window_b.json` (every touchpoint
mention of two notice windows, with its parent's printed name), `second-look/raw/t2_salted40_labels.json` (an
independent hand read). Code read: `project.rs` `touchpoint_decision` / `touchpoint_gate` / `Touchpoints::retarget`
/ `TouchpointRoles`, `role_census.rs` `NAME_PATTERNS` / `fold` / `portal_name`, `canonical.rs` `org_name_norm`,
`partyname.rs` `not_a_name`, `title_key_nfkd.rs`.

Files (all in `.scratch/tender-db/586-u1/`): `fold.py` (the reference implementation, `--selftest` 38/38),
`apply.py` (the counts, `--salted`, `--vectors`), `variants.py` (the tightening log), `tables.py` (the salted
tables below), `vectors.json` (both keys and the tier of all 3,751 pairs, for the Rust port), `apply.out`,
`variants.out`, `final_tiers.json`. The `gate*`, `persons*`, `pop.py`, `explore*`, `table*` files beside them are
the (b) half's, not this one's.

## Decision

The alias fold compares a touchpoint ONLY with its own parent ORG section in the same notice, name by name over
every language either side publishes. `store::org_name_norm` stays at spelling and its equality stays the first
test (D576-1 (i), counted `aliased_norm`); then tier A, then tier C; then (iv)'s gate and the drop classes; else (ii)
mint. Alias before drop, as today.

On the 3,751 new orgs (24,664 touchpoint mentions):

| Tier | Rule | Orgs | Their mentions |
|---|---|---|---|
| A | same key (spelling, brackets, legal forms, articles) | **182** | **743** |
| C0 | same organ, other spelling ("Junta de Gobierno Local" / "Junta de Gobierno", "Pleno Municipal" / "Pleno") | 9 | 46 |
| C1 | the entity under its organ (parent minus a leading organ phrase) | 455 | 1,778 |
| C2 | the town hall under another municipal word (Ajuntament / Ayuntamiento / Concello / Ayto. / Comune / Mairie …) | 71 | 258 |
| C3 | the bare town under its town hall | 145 | 526 |
| C3r | the town hall of a bare-named parent ("COMUNE DI LAURINO" under "LAURINO") | 4 | 5 |
| **C** | **all of tier C** | **684** | **2,613** |
| none | still minted | 2,885 | 21,308 |

So 866 of the 3,751 new orgs (23.1%) and 3,356 of their 24,664 mentions (13.6%) alias. The mention figure is an
upper bound: an org's mentions are counted whole, though only its first mention's parent was compared (the hubs,
which have several parents, alias under none of these rules).

**W bar: 0.** Both salted 40s are 40 R / 0 W (tables below). Beyond the bar, every one of the 866 aliased orgs was
read by hand (the full dump, `python3 -I apply.py … --dump`): 0 W. Every aliased group of the two notice windows
was read too (237 mentions): 0 W.

**Existing rows too (P2).** The fold acts before the resolver, so it also catches touchpoints that today bind an
older identifier-less row naming the parent (1217875 "Ayuntamiento de Alcobendas" under "Junta de Gobierno del
Ayuntamiento de Alcobendas"):

| Window | Minted touchpoint mentions | Would alias | of them, bound to older rows | bound to new orgs |
|---|---|---|---|---|
| notices 24,020,000–24,039,999 | 2,081 | 166 (8.0%) | 143 of 1,909 (7.5%) | 23 of 172 |
| notices 47,420,000–47,439,999 | 904 | 71 (7.9%) | 59 of 770 (7.7%) | 12 of 134 |

That is 62–65% of the verdict's "older row naming the parent" share (232 of 1,909, 91 of 770). Scaled to 2759's
~233k minted mentions: about 18–19k mentions (≈ 39k of the 491,287 minted references at 2.1 per mention) move to
aliased; unit 3's drain should see `aliased` rise from 67,434 by roughly that much.

**Independent check.** Second-look's own salted 40 (salt `576-second-look-salted-40`, labelled before this work):
of its 24 SELF, 14 alias (A 4, C 10); of its 16 others (REVIEW 7, DEPT 6, OTHER-BODY, LABEL, PLATFORM), 0 alias.

## Tier A: the key

`fold.key_tokens(name, side)`; tier A is `key(touchpoint, 'tp') == key(parent, 'parent')`, both non-empty.

1. **Entities.** `&#NNN;`, `&#xHHH;`, `&amp;` `&quot;` `&apos;` `&lt;` `&gt;` `&nbsp;` decoded (the parent of
   31977769 prints `l&#8217;attuazione`).
2. **Characters.** Lower-case the whole string; map each char through NFKD with every combining char dropped,
   for exactly the blocks of `title_key_nfkd.rs` (the port reuses `title_key_nfkd::NFKD_STRIPPED`: accents go,
   U+037E becomes `;`, fullwidth becomes ASCII, `º` becomes `o`); then the letters NFKD leaves whole: `ł→l ø→o
   đ→d ð→d ħ→h ı→i ŧ→t ĸ→k ŋ→n ß→ss æ→ae œ→oe þ→th ς→σ`; lower-case again.
3. **Brackets** `()` `[]` `{}`: the text from an opener to the first closer of the same kind (or to the end, when
   unclosed) is cut or kept by the side rule; a stray closer is a space.
   - Parent side: every bracket is cut. What the parent prints in brackets describes the parent: "(SARGA)",
     "(AVRA)", "(INSST)", "(Agrupación de Base)".
   - Touchpoint side: cut only when the content (steps 1–2 and 4, one space between words) is
     - a contact or registry label: `punto de contacto`, `otro punto de contacto`, `punto contacto`, `pc`, `tpo`,
       `contact point`, `point de contact`, `punt de contacte`, `sede electronica`, `registro`,
       `registro electronico`, `registro general`, `preguntas y respuestas`, `a traves de placsp`, `na`, `n a`;
     - exactly a Spanish province or autonomous community (`TP_BRACKET_PLACES`, 80 spellings: "(Córdoba)",
       "(GUADALAJARA)", "(MADRID)");
     - empty, or only legal-form and article tokens ("(Medio Propio)");
     - or the same as one of the parent's own brackets after step 8 ("(ICA)"/"(ICA)", "(OAGER)"/"(O.A.G.E.R.)").
   - Anything else stays in the touchpoint's key: "(TARCAS)", "(TACRC)", "(Contratación y Compras)",
     "(Area de Contratación)", "( SERVCIO P..CIUDADANA)", "(PROCASA)", "(J. Arends)", "(Emscher)", "(abt.14)".
4. **Punctuation.** Every char that is not alphanumeric (`str.isalnum` / `char::is_alphanumeric`) is a space;
   split on spaces. Quotes, `:`, `;`, U+037E, `/`, `·` and the spacing around hyphens all fall out here.
5. **Spelled-out legal forms, ANYWHERE** (leftmost, longest first): `sociedad anonima unipersonal`,
   `sociedad limitada unipersonal`, `sociedad anonima municipal`, `sociedad anonima`, `sociedad limitada`,
   `sociedad de responsabilidad limitada`, `sociedad cooperativa`, `societat anonima`, `societat limitada`,
   `sociedade anonima`, `sociedade unipessoal`, `spolka z ograniczona odpowiedzialnoscia`, `spolka akcyjna`,
   `spolka komandytowa`, `spolka jawna`, `akciova spolecnost`, `spolecnost s rucenim omezenym`,
   `spolocnost s rucenim obmedzenym`, `societa per azioni`, `societa a responsabilita limitata`,
   `societa cooperativa`, `societa consortile`, `aktiengesellschaft`, `gesellschaft mit beschrankter haftung`,
   `anstalt des offentlichen rechts`, `anstalt offentlichen rechts`, `korperschaft des offentlichen rechts`,
   `naamloze vennootschap`, `besloten vennootschap`, `societe anonyme`, `societe par actions simplifiee`,
   `societe a responsabilite limitee`, `public limited company`, `medio propio personificado`, `medio propio`,
   `mitja propi`. Multi-word and unambiguous, so position does not matter ("MIEJSKIE PRZEDSIĘBIORSTWO … SPÓŁKA
   AKCYJNA W KRAKOWIE").
6. **Abbreviated legal forms, at the END only**, repeatedly, longest first, never down to nothing ("…, S.A.U.,
   M.P." → "…"). Written as folded tokens, spaced and joined forms both listed:
   - ES / PT / IT / FR: `s a u` `sau` `s l u` `slu` `s a` `sa` `s l` `sl` `s l l` `sll` `s l p` `slp` `s coop`
     `soc coop` `s a l` `sal` `s a m` `sam` `s m e` `sme` `m p` `mp` `e p e` `epe` `e p e l` `epel` `o a` `oa`
     `o a l` `oal` `e m` `em` `a i e` `aie` `e i r l` `eirl` `lda` `unipessoal lda` `f s p` `fsp` `i p` `ip`
     `s p a` `spa` `s r l` `srl` `s c a r l` `scarl` `s c r l` `scrl` `onlus` `s n c` `snc` `s a s` `sas`
     `s a r l` `sarl` `eurl` `sasu` `spl` `sem` `scop` `gie`;
   - DE / AT / NL / BE: `gmbh` `ggmbh` `mbh` `gmbh co kg` `co kg` `kg` `ag` `e v` `ev` `a o r` `aor` `k d o r`
     `kdor` `b v` `bv` `n v` `nv` `u a` `ua` `v o f` `vof` `bvba` `sprl` `cvba` `vzw` `asbl`;
   - UK / IE: `ltd` `limited` `plc` `llp` `llc` `inc` `cic` `dac` `clg`;
   - PL / CZ / SK / HU / HR / SI / MK / BG / RO: `sp z o o` `sp z oo` `spzoo` `sp k` `spk` `sp j` `s k a` `s r o`
     `sro` `spol s r o` `a s` `as` `p o` `o p s` `kft` `zrt` `nyrt` `d o o` `doo` `dooel` `eood` `ood` `ead`
     `еоод` `оод` `еад`;
   - Nordic / Baltic / GR: `asa` `ab` `publ` `oy` `oyj` `aps` `uab` `sia` `α ε` `αε` `ε π ε` `επε` `ι κ ε` `ικε`
     `ο ε` `οε`.
   - Left out on purpose (two letters that also end a Spanish name as a department or region, and none fired):
     `sc` (Servicio de Contratación), `uc` (Unidad de Contratación), `se` (Secretaría de Estado), `cv`
     (Comunitat Valenciana), `s p`, `z s`, `i s`, `dd`, `bt`, `eg`, `ou`, `ad`.
   - Fired on the aliases: `s a` 101, `s a u` 29, `sa` 26, `s l` 21, `s l u` 11, `sau` 9, then M.P., S.A.M.,
     sp. z o.o., F.S.P., O.A., A.I.E., E.P.E., U.A., B.V., AöR, I.P., Soc. Coop., E.M., S.r.l.
7. **Articles, prepositions, conjunctions and honorifics, ANYWHERE** (after the legal forms, which contain some
   of them):
   - es: `el la los las lo de del al y e en con`; ca: `els les l d dels i`; gl/pt: `o a os as do da dos das`;
     fr: `le du des et au aux`; it: `il gli di dello della dei degli delle dell ed`; de: `der die das dem den
     und`; nl: `het van`; en: `the of and`; pl: `w we`;
   - honorifics: `excmo excma exmo exma excelentisimo excelentisima ilmo ilma iltmo iltma iltre ilustrisimo
     ilustrisima ilustre`.
   - None of these distinguishes two bodies inside one notice; dropping them only makes the pair equal.
8. **Initials.** Every run of two or more one-character tokens becomes one token: "C.U.C." = "CUC", "S.R.R" =
   "SRR".
9. **Touchpoint side only:** a leading contact label `punto contacto` / `otro punto contacto` is cut ("PUNTO DE
   CONTACTO TRIBUNAL ADMINISTRATIVO … JUNTA DE ANDALUCIA" under the tribunal itself).

## Tier C: the entity under its organ

Only when A fails, on the same key tokens (T = touchpoint key, P = parent key):

- **C0, same organ:** T and P are equal once a leading organ synonym is canonicalised on both: `junta gobierno
  local` → `junta gobierno` (and `junta govern local`, `xunta goberno local`), `pleno municipal` → `pleno`,
  `alcaldia presidencia` → `alcaldia`.
- **E** = P minus ONE leading organ phrase, the longest that leads P and leaves something (else E = P).
  - Organs: `presidencia consejo administracion`, `gerencia consejo administracion`, `alcaldia presidencia`,
    `junta gobierno local`, `junta govern local`, `xunta goberno local`, `junta gobierno`, `junta govern`,
    `xunta goberno`, `pleno`, `ple`, `pleno municipal`, `alcaldia`, `presidencia`, `vicepresidencia`, `gerencia`,
    `direccion gerencia`, `direccion gerente`, `consejo administracion`, `consell administracio`,
    `consello administracion`, `consejo rector`, `consell rector`, `consello reitor`, `rectorado`,
    `consejo gobierno`, `consell govern`, `consello goberno`, `consejo gobierno insular`, `comision ejecutiva`,
    `comision gestora`, `comision permanente`, `consejero delegado`, `consejera delegada`,
    `consejeria delegada`, `direccion general`, `direccion ejecutiva`, `direccion`, `asamblea`,
    `asamblea concejales`, `asamblea representantes`, `asamblea general`, `asamblea vecinal`, `junta general`,
    `junta vecinal`, `junta directiva`, `junta plenaria`, `junta rectora`, `consejo general`, `consejo comarcal`,
    `consejo directivo`, `comite directivo`, `administrador unico`, `organo administracion`.
  - Officers: `rector`, `rectora`, `presidente`, `presidenta`, `vicepresidente`, `vicepresidenta`, `alcalde`,
    `alcaldesa`, `alcalde presidente`, `gerente`, `director`, `directora`, `director gerente`,
    `directora gerente`, `director general`, `directora general`.
  - Deliberately NOT organs (departments keep their words, so a department never peels down to its entity):
    Secretaría General (Técnica), Concejalía, Regidoría, Área (de Gobierno), Servicio, Distrito, Dirección
    Provincial / Territorial, Delegación, Mesa / Junta de Contratación, Concejal.
- **C1:** an organ was cut, E is two or more tokens or one token of four or more characters, and T == E
  ("Ayuntamiento de Ezcaray" under "Junta de Gobierno Local del Ayuntamiento de Ezcaray"; "GESPESA SAU" under
  "Gerencia de GESPESA, S.A.U.").
- **Municipal words** (`MUNI`) at the head of E and of T are canonicalised to `ayuntamiento`: `ayuntamiento`,
  `ajuntament`, `concello`, `ayto`, `aj`, and the same word in the population's other languages: `comune`,
  `commune`, `mairie`, `ville`, `gemeente`, `gmina`, `miasto`, `stadt`, `gemeinde`, `municipio`, `concelho`,
  `obec`, `mesto`, `kommune`.
- **C2:** E (canonicalised) starts with `ayuntamiento`, has a place after it, and T (canonicalised) == E
  ("Ajuntament de Biar", "ayto carmona", "Concello de Lalín" under "Junta de Gobierno del Ayuntamiento de …").
- **C3:** E (canonicalised) starts with `ayuntamiento` and T == the place after it ("Marugán" under "Pleno del
  Ayuntamiento de Marugán").
- **C3r:** T (canonicalised) starts with `ayuntamiento`, P has no municipal head, and P == the place after it
  ("COMUNE DI LAURINO" under "LAURINO").
- **Never:**
  - an organ cut from the TOUCHPOINT ("Alcaldía del Ayuntamiento de Segorbe" under "Pleno del Ayuntamiento de
    Segorbe" is another organ and mints);
  - a bare place under anything but a town hall ("Bilbao" under "Presidencia de la Autoridad Portuaria de
    Bilbao" mints);
  - a prefix, suffix, overlap or fuzzy match.

## How the rules were tightened (the 0-W bar)

V0 is the issue's text read literally:
- every touchpoint bracket cut;
- an organ cut from either side;
- "the bare place" = any tail of the parent key;
- any non-empty entity.

Each step below is one tightening, with what it removed from the 3,751 (`variants.py`, `variants.out`).
Judgements are from reading every removed row.

| Step | Tier A orgs / m | Tier C orgs / m | Removed W | Cost in R (orgs / m) |
|---|---|---|---|---|
| V0 literal | 186 / 774 | 768 / 3,080 | | |
| T1 a bare place only under a town hall | 186 / 774 | 706 / 2,662 | 5 (53 m) | 57 / 365 |
| T2 no organ cut on the touchpoint | 186 / 774 | 693 / 2,636 | 9 (+3 N) | 1 / 1 |
| T3 touchpoint brackets: allow-list only | 182 / 743 | 675 / 2,598 | 5 (9 m) | A 4 / 31, C 13 / 29 |
| T3b + a bracket naming exactly a province | 182 / 743 | 684 / 2,613 | 0 | recovers 9 / 15 |
| T4 a one-token entity needs 4+ chars | 182 / 743 | 684 / 2,613 | 0 | 0 (a guard: "Presidencia de la CE" / "CE") |

**T1, a bare place only under a town hall** (removed 62 C orgs).
- W removed (5):
  - 31975176 "Bilbao" under the Port Authority of Bilbao, 42 m: the city's name;
  - 31977762 "Ajuntament Alcúdia" under "Regidoria Delegada d'Hisenda… de l'Ajuntament d'Alcúdia";
  - 31978681 "Ayuntamiento Benalmádena" under "Concejalía de Hacienda…";
  - 31978200 "Oliva" under "Concejal de Contratación del Ayuntamiento de Oliva".
    In these three the parent is a department, and the town hall would land on it.
  - 31977712 "Guadalquivir" under "Junta de Gobierno Local de la Mancomunidad Guadalquivir": also the river
    basin authority's name.
- R lost (57 orgs, 365 m): 14 mancomunidad or comarca bare names ("Alto Palancia", "Tierras Altas de Soria");
  company short names ("Aguas de Burgos" 81, "CANTUR" 81 + 4, "CENIEH"); "Nadleśnictwo X" under "Państwowe
  Gospodarstwo Leśne Lasy Państwowe - Nadleśnictwo X".

**T2, no organ cut on the touchpoint** (removed 13 C orgs).
- W removed (9): another organ of the same entity.
  - "Alcaldía" under "Pleno" or "Junta de Gobierno (Local)", 7 rows: Noja, La Pobla de Vallbona, Almadén,
    Villanueva de la Reina, Puebla de Sanabria, Laxe, Segorbe;
  - "Junta de Gobierno" under "Gerencia" (O.A. Consejo Insular de Aguas de El Hierro);
  - "Gerencia" under "Consejo de Administración" (Zaragoza Vivienda).
- N removed (3): an officer of the parent named by the touchpoint ("Director/a Gerente del IASS", "Presidente do
  IGFEJ", "Consejería Delegada de GMR").
- R lost (1): "Presidencia de la Institución Ferial de Tenerife" under "Presidencia del Consejo de Administración
  de …".

**T3, touchpoint brackets on an allow-list only** (removed 18 C and 4 A orgs).
- W removed (5): a department in the bracket.
  - "Ayuntamiento de Alcobendas (Contratación y Compras)";
  - "Ayuntamiento de Alfacar (Area de Contratación)";
  - "ayuntamiento de sevilla (servicio mujer)";
  - "Ayuntamiento de Sevilla (Servicio P. Ciudadana)";
  - "AYUNTAMIENTO DE SEVILLA ( SERVCIO P..CIUDADANA)": a typo that a deny-list of department words would have
    let through, which is why this is an allow-list.
- R lost, tier C (13 orgs; 9 come back with T3b): the 9 province brackets; "(Mdrid)"; "(PROCASA)" (an acronym
  the parent does not print); "(IMGEMA)" against the parent's "(INGEMA)"; "(solo para recursos administrativos
  ordinarios)".
- R lost, tier A (4 orgs, 31 m): "(SITVAL)", "(MUVISA)" 24 m, "(DIRSO)" (acronyms the parent does not print), "(J.
  Arends)" (a person).
- An acronym test could recover those four, but would also pass "Ayuntamiento de Sevilla (TARCAS)".

**The salted 40s.**
- V0's own salted 40s (same salts):
  - tier A: 40 R / 0 W. V0's whole tier A holds no W; T3's four tier-A removals are R.
  - tier C: 37 R / 3 W:
    - #24 31975419 "Junta de Gobierno del O.A. Consejo Insular de Aguas de El Hierro" under "Gerencia del O.A. …"
      (another organ, T2);
    - #30 31978113 "AYUNTAMIENTO DE SEVILLA ( SERVCIO P..CIUDADANA)" under "Junta de Gobierno del Ayuntamiento
      de Sevilla" (department bracket, T3);
    - #37 31978486 "Alcaldía del Ayuntamiento de Segorbe" under "Pleno del Ayuntamiento de Segorbe" (another
      organ, T2).
- After T1–T4, both salted 40s are 0 W.
- T1's five W sat outside the salted 40, so the full read set the bar, not the sample.

**Extensions kept because the full read found them all R:**
- Rules: C0; the officer and collegiate-organ phrases; the non-Spanish municipal words; `en` and `con`; F.S.P. and
  I.P.; initials joining; the touchpoint's head contact label; the parent's own bracket after initials.
- Together: tier A 170 → 182 orgs, tier C 638 → 684 orgs.

## Salted 40 per tier (final rules)

Population: the tier's orgs ordered by `sha256("<salt>:<org id>")` hex, the first 40 (`tables.py`). R = the
touchpoint is the parent's own body; W = a distinct body (a review organ, a registry of another body, a distinct
department or organ) that must not alias. Read 2026-10-24.

#### Tier A: salt `586-u1-tierA`, 40 of 182

| # | org | tier | m | parent (printed, same notice) | touchpoint | judged |
|---|---|---|---|---|---|---|
| 1 | 31976215 | A | 1 | Consejo de Administración de la Empresa Municipal de la Vivienda de Coslada S.A.U | Consejo de Administración de la Empresa Municipal de la Vivienda de Coslada | R: same name, spelling only |
| 2 | 31976370 | A | 1 | Asamblea de Representantes de la Mancomunidad Norte Duero | Asamblea de Representantes de la Mancomunidad Norte-Duero | R: same name, spelling only |
| 3 | 31975754 | A | 1 | Espaço Municipal - Renovação Urbana e Gestão do Patrimonio, E. M., S. A. | ESPAÇO MUNICIPAL RENOVAÇÃO URBANA E GESTÃO DO PATRIMÓNIO, EM.SA | R: same name, spelling only |
| 4 | 31978151 | A | 1 | Patronato Provincial Turismo de Castellón | O.A.L. PATRONATO PROVINCIAL DE TURISMO DE CASTELLÓN | R: same name, spelling only |
| 5 | 31975704 | A | 3 | Dirección de Gestión Económica de la Jefatura de Apoyo Logístico de la Armada | Dirección Gestión Económica de la Jefatura de Apoyo Logístico de la Armada | R: same name, spelling only |
| 6 | 31977531 | A | 6 | Fundació per a l'Esport Balear | Fundacio per a l'Esport Balear | R: same name, spelling only |
| 7 | 31977224 | A | 1 | Gestión de Medio Rural de Canarias S.A.U. (GMR Canarias) | Gestión de Medio Rural de Canarias S.A.U. | R: same name, spelling only |
| 8 | 31978214 | A | 4 | Comandancia Guardia Civil de Castellón | Comandancia de la Guardia Civil de Castellón. | R: same name, spelling only |
| 9 | 31975289 | A | 1 | Dirección General de Relaciones con la Administración de Justicia | DIRECCION GENERAL RELACIONES ADMINISTRACION DE JUSTICIA | R: same name, spelling only |
| 10 | 31978249 | A | 4 | Instituto Nacional de Seguridad y Salud en el Trabajo O.A. M.P. (INSST) | Instituto Nacional de Seguridad y Salud en el Trabajo, OA, MP | R: same name, spelling only |
| 11 | 31976985 | A | 1 | Gerencia de Atención Primaria de León | Gerencia de Atencion Primaria de León | R: same name, spelling only |
| 12 | 31976673 | A | 3 | Instituto de Fomento de la Región de Murcia (INFO) | Instituto de Fomento de la Región de Murcia | R: same name, spelling only |
| 13 | 31978334 | A | 1 | AGENCIA DE VIVIENDA Y REHABILITACIÓN DE ANDALUCÍA | AGENCIA DE VIVIENDA Y REHABILITACIÓN DE ANDALUCIA | R: same name, spelling only |
| 14 | 31976426 | A | 1 | CUC - Comune di Fano | CUC -Comune di Fano | R: same name, spelling only |
| 15 | 31976204 | A | 1 | C.U.C. tra i Comuni di Arcevia, Barbara, Ostra, Ostra Vetere, Serra de’ Conti e Senigallia c/o il Comune di Senigallia | C.U.C. tra i Comuni di Arcevia, Barbara, Ostra, Ostra Vetere, Serra de' Conti e Senigallia c/o il Comune di Senigallia | R: same name, spelling only |
| 16 | 31976263 | A | 61 | Consejería de Infraestructuras, Transporte y Vivienda | CONSEJERIA DE INFRAESTRUCTURAS, TRANSPORTE Y VIVIENDA | R: same name, spelling only |
| 17 | 31978315 | A | 1 | Agencia de Contratación Sanitaria de la Comunidad de Madrid | Agencia de Contratación Sanitaria de la Comunidad de Madrid (TPO) | R: same name, spelling only |
| 18 | 31977373 | A | 7 | TRIBUNAL ADMINISTRATIVO DE RECURSOS CONTRACTUALES DE LA JUNTA DE ANDALUCIA | PUNTO DE CONTACTO TRIBUNAL ADMINISTRATIVO DE RECURSOS CONTRACTUALES DE LA JUNTA DE ANDALUCIA | R: same name, spelling only |
| 19 | 31977325 | A | 1 | Ayuntamiento Premia de Mar | Ayuntamiento de Premiá de Mar | R: same name, spelling only |
| 20 | 31977775 | A | 2 | Pleno del Ayuntamiento de Sevilla la Nueva | Pleno Ayuntamiento Sevilla la Nueva | R: same name, spelling only |
| 21 | 31978166 | A | 5 | INSS-Dirección provincial de Granada | INSS - Dirección Provincial de Granada | R: same name, spelling only |
| 22 | 31977666 | A | 3 | Agencia Tributaria de la Región de Murcia (ATRM) | Agencia Tributaria de la Region de Murcia | R: same name, spelling only |
| 23 | 31978218 | A | 2 | Empresa de Transformación Agraria S.A.,S.M.E., M.P., (TRAGSA) | Empresa de Transformación Agraria SA | R: same name, spelling only |
| 24 | 31978189 | A | 3 | Comandancia Guardia Civil de Lugo | Comandancia Guardia Civil Lugo | R: same name, spelling only |
| 25 | 31977096 | A | 2 | Coöperatie KIEN UA | Cooperatie KIEN UA | R: same name, spelling only |
| 26 | 31977081 | A | 2 | Universitatea "Vasile Alecsandri" din Bacau | Universitatea Vasile Alecsandri din Bacau | R: same name, spelling only |
| 27 | 31978450 | A | 1 | ASL1 AVEZZANO-SULMONA-L'AQUILA | ASL1 Avezzano, Sulmona L'Aquila | R: same name, spelling only |
| 28 | 31978284 | A | 2 | Alcaldia del Ayuntamiento de Tías | Alcaldía Ayuntamiento de Tías | R: same name, spelling only |
| 29 | 31975768 | A | 4 | Tribunal Administrativo de los Recursos Contractuales de la Junta de Andalucía | TRIBUNAL ADMINISTRATIVO DE RECURSOS CONTRACTUALES DE LA JUNTA DE ANDALUCÍA (Punto de contacto) | R: same name, spelling only |
| 30 | 31975819 | A | 2 | COMMUNE DE SERRES CASTET | COMMUNE DE SERRES-CASTET | R: same name, spelling only |
| 31 | 31978542 | A | 1 | SNCF RESEAU | SNCF Réseau SA | R: same name, spelling only |
| 32 | 31976997 | A | 3 | AGENCIA DE VIVIENDA Y REHABILITACIÓN DE ANDALUCÍA | AGENCIA DE VIVIENDA DE REHABILITACION DE ANDALUCIA (punto de contacto) | R: same name, spelling only |
| 33 | 31977282 | A | 1 | Alcaldía del Ayuntamiento de Alhaurín de la Torre | Alcaldia de Ayuntamiento de Alhaurin de la Torre | R: same name, spelling only |
| 34 | 31978281 | A | 2 | Gerencia del Área de Salud de Mérida | GERENCIA AREA DE SALUD DE MERIDA | R: same name, spelling only |
| 35 | 31976518 | A | 5 | Alcaldía del Ayuntamiento de Yaiza | Alcaldía Ayuntamiento de Yaiza | R: same name, spelling only |
| 36 | 31975870 | A | 1 | C.U.C. COMUNI DI TORRE DEL GRECO E TRECASE | CUC DEI COMUNI DI TORRE DEL GRECO E TRECASE | R: same name, spelling only |
| 37 | 31975150 | A | 1 | JUNTA DE ANDALUCÍA, CONSEJERÍA DE DESARROLLO EDUCATIVO Y FORMACIÓN  PROFESIONAL | JUNTA DE ANDALUCÍA. CONSEJERÍA DE DESARROLLO EDUCATIVO Y FORMACIÓN  PROFESIONAL | R: same name, spelling only |
| 38 | 31975115 | A | 1 | Zentrum für Psychiatrie Weinsberg, AöR | Zentrum für Psychiatrie Weinsberg | R: same name, spelling only |
| 39 | 31975406 | A | 7 | ÁREA SANITARIA DE PONTEVEDRA E O SALNÉS | Área Sanitaria de Pontevedra y O Salnés | R: same name, spelling only |
| 40 | 31975899 | A | 12 | Consejería de Agricultura, Ganadería y Desarrollo Sostenible | Consejería Agricultura, Ganadería y Desarrollo Sostenible | R: same name, spelling only |

#### Tier C: salt `586-u1-tierC`, 40 of 684

| # | org | tier | m | parent (printed, same notice) | touchpoint | judged |
|---|---|---|---|---|---|---|
| 1 | 31975893 | C1 | 7 | Asamblea del Consorcio para el Desarrollo de la Vega Sierra Elvira | Consorcio para el Desarrollo de la Vega Sierra Elvira | R: the entity under its organ |
| 2 | 31976706 | C1 | 8 | Alcaldía del Ayuntamiento de Villanueva del Duque | Ayuntamiento de Villanueva del Duque | R: the entity under its organ |
| 3 | 31976323 | C1 | 5 | Junta de Gobierno Local de l'Ajuntament d'Artà | Ajuntament d'Artà | R: the entity under its organ |
| 4 | 31976292 | C3 | 3 | Pleno del Ayuntamiento de Vilaboa | vilaboa | R: the bare town under its town hall |
| 5 | 31977872 | C1 | 2 | Presidencia de la Mancomunidad de Recogida y Tratamiento de Basuras de Colmenar del Arroyo, Fresnedillas de la Oliva, Navalagamella y Zarzalejo | Mancomunidad de Recogida y Tratamiento de Basuras de Colmenar del Arroyo, Fresnedillas de la Oliva, Navalagamella y Zarzalejo | R: the entity under its organ |
| 6 | 31975292 | C0 | 8 | Junta de Gobierno del Ayuntamiento de Altea | Junta de Gobierno Local del Ayuntamiento de Altea | R: same organ, JGL/Pleno spelling |
| 7 | 31976075 | C1 | 1 | Alcaldía del Ayuntamiento de Almodóvar del Río | Ayuntamiento de Almodovar del Rio | R: the entity under its organ |
| 8 | 31978541 | C1 | 1 | Alcaldía del Ayuntamiento de Chinchón | Ayuntamiento de Chinchon | R: the entity under its organ |
| 9 | 31975684 | C3 | 2 | Junta de Gobierno Local del Ayuntamiento de Yuncler | Yuncler | R: the bare town under its town hall |
| 10 | 31978263 | C3 | 2 | Junta de Gobierno del Ayuntamiento de Lardero | Lardero | R: the bare town under its town hall |
| 11 | 31976703 | C1 | 3 | Alcaldía del Ayuntamiento de Canals | Ayuntamiento de Canals | R: the entity under its organ |
| 12 | 31975007 | C1 | 3 | Junta de Gobierno Local del Ayuntamiento de Ezcaray | Ayuntamiento de Ezcaray | R: the entity under its organ |
| 13 | 31977595 | C1 | 2 | Junta General de la Mancomunidad de Servicios Los Olmos | Mancomunidad de Servicios Los Olmos | R: the entity under its organ |
| 14 | 31975649 | C2 | 1 | Pleno del Ayuntamiento de Daimús | ajuntament daimus | R: the town hall, another word for it |
| 15 | 31975679 | C2 | 4 | Junta de Gobierno del Ayuntamiento de Els Poblets | Ajuntament Els Poblets | R: the town hall, another word for it |
| 16 | 31977053 | C2 | 1 | Pleno del Ayuntamiento des Migjorn Gran | Ajuntament des Migjorn Gran | R: the town hall, another word for it |
| 17 | 31977324 | C1 | 2 | Pleno del Ayuntamiento de Tuéjar | Ayuntamiento de Tuéjar | R: the entity under its organ |
| 18 | 31976066 | C1 | 1 | Dirección de la Fundación General de la Universidad de León y de la Empresa | FUNDACIÓN GENERAL DE LA UNIVERSIDAD DE LEON Y DE LA EMPRESA | R: the entity under its organ |
| 19 | 31978287 | C1 | 3 | Pleno del del Ayuntamiento de Herencia | Excmo. Ayuntamiento de Herencia | R: the entity under its organ |
| 20 | 31976992 | C1 | 18 | Presidencia del Consejo de Administración de Guaguas Municipales S.A. | Guaguas Municipales S.A. | R: the entity under its organ |
| 21 | 31978184 | C1 | 1 | Gerencia de la Fundación para la Investigación Biomédica de Córdoba (FIBICO) | Fundacion para la investigacion biomedica de Córdoba (FIBICO) | R: the entity under its organ |
| 22 | 31976571 | C1 | 3 | Presidencia del Consejo de Administración de Aparcamientos Municipales y Gestíon Vial de Ceuta, S.A. (AMGEVICESA) | APARCAMIENTOS MUNICIPALES Y GESTIÓN VIAL DE CEUTA SA | R: the entity under its organ |
| 23 | 31976903 | C1 | 3 | Pleno del Ayuntamiento de Caso | ayuntamiento de caso | R: the entity under its organ |
| 24 | 31975195 | C1 | 1 | Pleno de la Mancomunidad Puente la Unión | Mancomunidad Puente la Unión | R: the entity under its organ |
| 25 | 31977553 | C1 | 1 | Consejo Comarcal de la Comarca de Andorra-Sierra de Arcos | Comarca de Andorra Sierra de Arcos | R: the entity under its organ |
| 26 | 31976767 | C3 | 2 | Pleno del Ayuntamiento de Valdeavero | Valdeavero | R: the bare town under its town hall |
| 27 | 31977031 | C1 | 2 | Gerencia de Benissa Impuls S.A. | BENISSA IMPULS S.A | R: the entity under its organ |
| 28 | 31976391 | C1 | 4 | Pleno del Ayuntamiento de Villatobas | Ayuntamiento de Villatobas | R: the entity under its organ |
| 29 | 31976402 | C3 | 1 | Junta de Gobierno Local del Ayuntamiento de As Neves | Neves | R: the bare town under its town hall |
| 30 | 31977172 | C1 | 1 | Pleno del Ayuntamiento de Torrecillas de la Tiesa | Ayuntamiento de Torrecillas de la Tiesa | R: the entity under its organ |
| 31 | 31978481 | C1 | 1 | Asamblea de Concejales de la Mancomunidad Pueblos de la Vecindad de Burgos | Mancomunidad de Pueblos de la Vecindad de Burgos | R: the entity under its organ |
| 32 | 31977016 | C3 | 2 | Alcaldía del Ayuntamiento de El Campillo | El Campillo | R: the bare town under its town hall |
| 33 | 31978702 | C2 | 2 | Junta de Gobierno del Ayuntamiento de Galapagar | ayto galapagar | R: the town hall, another word for it |
| 34 | 31976407 | C3r | 1 | Castilenti | Comune di Castilenti | R: the town hall of the bare-named parent |
| 35 | 31976406 | C1 | 1 | Gerencia de Viviendas Sociales e Infraestructuras de Canarias S.A.U. | VIVIENDAS SOCIALES E INFRAESTRUCTURAS DE CANARIAS, SAU | R: the entity under its organ |
| 36 | 31978436 | C1 | 1 | Alcaldía del Ayuntamiento de Santa Cruz de Mudela | Ayuntamiento Santa Cruz de Mudela | R: the entity under its organ |
| 37 | 31977007 | C1 | 1 | Pleno del Ayuntamiento de Belinchón | Ayuntamiento de Belinchón | R: the entity under its organ |
| 38 | 31976561 | C2 | 1 | Alcaldía del Ayuntamiento de Casarrubuelos | ayto de Casarrubuelos | R: the town hall, another word for it |
| 39 | 31977502 | C1 | 1 | Gerencia de Móstoles Desarrollo Promoción Económica S.A. | MÓSTOLES DESARROLLO PROMOCIÓN ECONÓMICA, S.A. | R: the entity under its organ |
| 40 | 31978333 | C3 | 1 | Pleno del Ayuntamiento de La Almunia de Doña Godina | Almunia de Doña Godina | R: the bare town under its town hall |

**Result: tier A 40 R / 0 W; tier C 40 R / 0 W.**

## Unit 2's named tests against these rules

- **540072 SARGA aliases: yes, tier A.** "Sociedad Aragonesa de Gestión Agroambiental S.L.U" under "…, S.L.U.
  (SARGA)"; both keys are `sociedad aragonesa gestion agroambiental`.
- **AVRA under 1400401: six alias, tier A.** The six:
  - 31975748 "(Punto de contacto)";
  - 31975794 "(PUNTO DE CONTACTO)";
  - 31975933 "(PC)" against the parent's "(AVRA)";
  - 31976931 "(otro punto de contacto)";
  - 31976997 "VIVIENDA DE REHABILITACION";
  - 31978334, which differs by accent only.

  31977372 "DIRECCIÓN PROVINCIAL DE AVRA EN SEVILLA (punto de contacto)" still mints: it is a provincial branch.
- **Gmina Panki: the U+037E is folded, but the live rows do not alias.**
  - The two keys are equal; the selftest aliases a touchpoint printing U+037E under a parent printing `;`.
  - 31977139 / 31978194 "Gmina Panki ul. Tysiąclecia 5; 42-140 Panki" sit under the parent "Gmina Panki": that
    is parent + address, tier B, and it still mints.
  - Unit 2 should test the synthetic pair. An address-suffix alias was not designed here.
- **TARCAS still mints: yes, all 20 TARCAS rows.** Among them:
  - "TARCAS. AYUNTAMIENTO DE SEVILLA" and "Ayuntamiento de Sevilla - TARCAS";
  - "Registro del Tribunal Administrativo de Recursos Contractuales (TARCAS)";
  - "Tribunal … (TARCAS)del Ayuntamiento de Sevilla".
- **"Stadt Fürth - Zentrale Vergabestelle" still mints: yes** (tier B).
- **"Tribunal Administrativo de Recursos Contractuales de la Excma. Diputación de Cádiz" still mints: yes.**
- **The counter split:** `TouchpointRoles.aliased` becomes `aliased_norm` (D576-1 (i)) + `aliased_a` +
  `aliased_c`, with `aliased()` their sum, so the drain and the read-back see each tier's size.

## Not aliased, and why (the self-shaped residue)

Second-look's classes (SELF-overlap 276, SELF-part 173, SELF-plus 161, SELF-spelling 18) still mint. The shapes,
none of which tiers A or C may take without W:

- **Typos (41 orgs at key similarity ≥ 0.97, 61 at ≥ 0.95).** "Junata de Gobierno", "Rectotado", "Diputación Provicnial",
  "Ayuntamineto". An edit-distance tier would also alias:
  - 31975370 "Dirección General de Renfe Fabricación y Mantenimiento" under "… Renfe Ingeniería y Mantenimiento"
    (248 m, two directorates);
  - "Gmina Strzałowo" under "Gmina Strzałkowo";
  - "PGE Energetyka Kolejowa Holding" under "PGE Energetyka Kolejowa";
  - "Consejería de Cultura, Política Llingüística y Turismo" under "… y Deporte".

  Rejected.
- **Parent minus a trailing government qualifier.** 31976235 "Consejería de Economía, Hacienda, Fondos Europeos y
  Transformación Digital" under "… de la Región de Murcia" (49 m), the Asturias and Canarias consejerías, "Mesa de
  las Cortes" under "… de Aragón". This is the reverse of tier B. A prefix rule would also alias "Junta de
  Andalucía" under "Junta de Andalucía. Consejería X" (a broader body).
- **An acronym touchpoint, or the parent + its acronym.** EPREMASA, CHGUV, "(MUVISA)", "\nSAJMP". There is no
  safe test.
- **Another organ of the same entity, and the town hall + a district.** "Alcaldía" under "Pleno"; "AYUNTAMIENTO
  DE MADRID/DISTRITO DE CARABANCHEL" under "Distrito de Carabanchel". These are T2's class.
- **Mancomunidad and comarca bare names (14) and company short names.** These are T1's R cost. A later rule
  could take "bare name under a mancomunidad", but "Guadalquivir" shows its risk.
- **Parent + department or address (tier B).** D576-1 accepted this. An address suffix (Gmina Panki) would be a
  separate decision.

## Notes for the Rust port (unit 2)

- **Where.** In `touchpoint_decision`'s `same` closure, per (touchpoint name, Organization name) pair over every
  language:
  - `org_name_norm` equal → Alias, counted `aliased_norm`;
  - else tier A → Alias, `aliased_a`;
  - else tier C → Alias, `aliased_c`;
  - the best tier over the pairs wins.

  The touchpoint bracket rule compares against the brackets of the Organization name in that same pair.
- **Characters.**
  - Run `to_lowercase`, then `title_key_nfkd::NFKD_STRIPPED` (the same blocks as fold.py's `NFKD_BLOCKS`), then
    the 14-letter `EXTRA_FOLD`, then `to_lowercase` again.
  - Decode HTML entities before all of it.
  - `char::is_alphanumeric` stands for `str.isalnum`.
  - Python's `lower` and Rust's `to_lowercase` both apply the final-sigma rule, and `EXTRA_FOLD` maps `ς` to `σ`
    anyway.
- **Lists.**
  - Every list is written as folded tokens. Match phrases token by token, longest first.
  - Trailing legal forms are removed in a loop that never empties the name.
  - The legal forms run before the articles, because `sociedad de responsabilidad limitada` contains `de`.
- **Organ cut.** One phrase only, the longest that leads the parent key and leaves at least one token. A
  repeated cut would add two R rows here and the "Pleno del Consejo Comarcal del Bierzo" / "Bierzo" risk.
- **Check.**
  - `vectors.json` has 3,751 rows with `tp`, `parent`, `key_tp`, `key_parent` and `tier`.
  - The port's keys and tiers must match it row for row, and must pass fold.py's 38 selftest pairs.
  - Regenerate both with `python3 -I apply.py ../audit-576-fanout --vectors vectors.json` and `python3 -I fold.py
    --selftest`.
