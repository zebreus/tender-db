# 586 unit 1: adversarial check of fold.md (a) and gate.md (b)

Local only (2026-10-24): no network, no box, no `/v1/sql`, no cargo, no git. Inputs: the saved answers under
`audit-576-fanout/`, the two halves' own scripts (`fold.py`, `apply.py`, `gate.py`, `pop.py`, imported unchanged), and
the code (`project.rs` `touchpoint_decision` / `touchpoint_gate` / `Touchpoints::retarget` / `touchpoint_mentions` /
`TouchpointRoles`, `role_census.rs`, `partyname::fold`, `title_key_nfkd.rs`, `canonical.rs` `retire_mentions`,
`supervisor.rs` `RefoldSections` / `touchpoint_suffix`, `docs/operations.md` `sweep-orphan-orgs`). Fresh salt
`586-u1-check`. Scripts: `check_fold.py` (salted samples, scans, synthetic pairs), `check_variants.py` (what the
changes below cost), `check_gate.py` (real-organization names, salted hits, synthetic names). None of them edits the
halves' files.

## Verdicts

- **(a) fold: OK on the saved data. Three changes are required before unit 2 ports it.** Each one closes a W shape the
  rules let through but no saved row hits. Together they cost 0 R on the 3,751 pairs and in both windows
  (`check_variants.py`).
  1. Take `gmina` and `miasto` out of `MUNI`.
  2. Allow a one-token entity in C1 only when the parent prints a legal form.
  3. Keep a parent bracket in the parent key when it names a department, a registry or a review organ.
- **(b) gate: OK on the saved data. Two changes are required.**
  1. LEGALREF (D) needs a guard. Without one it drops a real buyer name that is in the saved data.
  2. The bare organ and registry labels, a W hub of the same kind as "Ayuntamiento", need stems or an alias.

  Both cost 0 R.

## Reproduced

- `fold.py --selftest` passes 38/38.
- `apply.py` gives tier A 182 orgs / 743 mentions and tier C 684 / 2,613. By sub-tier: C0 9, C1 455, C2 71, C3 145,
  C3r 4. Windows: 166 of 2,081 and 71 of 904 mentions alias.
- Second-look's hand-labelled 40: 14 of its 24 SELF alias, and 0 of its 16 others.
- `gate.py` gives 92 new orgs / 3,491 mentions (verdict list alone: 74 / 3,304).
  - R3 top 60: 0 hits.
  - Class N rebuilds to 18 / 37 / 853.
  - Review pool: 28 clusters, 0 members hit.
  - MUST_KEEP: 0 hit. MUST_DROP: 0 missed.

All match the halves' reports.

## (a) The fold

### Fresh salted reads: 0 W

| Sample | Size | Result |
|---|---|---|
| Tier A, salt `586-u1-check-tierA` | 40 of 182 | 40 R |
| Tier C, salt `586-u1-check-tierC` | 40 of 684 | 40 R (C1 25, C3 9, C2 5, C3r 1) |
| Window A aliased groups, salt `586-u1-check-03` | 40 of 123 | 40 R |
| Window B aliased groups, salt `586-u1-check-05` | 40 of 45 | 40 R |

- **Truncated names read in full.** #31 FIIAPP "… Administración y Políticas Públicas, F.S.P." and FASAD "… y/o …
  Medio Propio" are both R.
- **Existing rows the windows re-point.** All 202 such mentions go from an identifier-less older row (`okind` NULL) to an
  identified parent (`pkind` national). For example, 12390998 "Ayuntamiento de Murcia" moves to the identified "Junta
  de Gobierno del Ayuntamiento de Murcia". That is the direction CONTEXT.md wants.

### Full scans of all 866 aliased pairs: 0 W

- **Review, court and registry words on either side.** There are 7 hits:
  - the Andalusian tribunal's own TPO under the tribunal ORG, 4 rows;
  - "Delegación del Gobierno en Asturias (registro)", 2 rows;
  - one false positive.

  All are R. No review organ aliases into a buyer.
- **Parent brackets cut.** There are 35 (31 distinct), all acronyms, places or descriptors: SARGA, AVRA, INSST, "(Morón de la
  Frontera)", "(Turística y de Servicios)" and so on. None is a department or a review organ.
- **Touchpoint brackets cut.** Every one is on the allow-list: contact labels, provinces, the parent's own acronym, or
  "(Medio Propio)".
- **C1 by organ.** I read every non-Pleno, non-Alcaldía organ: presidencia 29, gerencia 33, direccion 15, direccion
  general 7, consejeria delegada 7, asamblea 3, junta general 2, vicepresidencia 2, and the rest.
  - Each one is an organ over its own mancomunidad, consortium, foundation, company or university.
  - None is a government department over its government.
  - The 7 one-token entities are all companies printed with S.A. or S.L.: GESPESA, Teidagua, Mercahierro, Obimace,
    Aquaoleiros, ARMUVISSA I, URVIPEXSA.
- **Municipal words.** Every C2 is an ES or CA or GL synonym, or FR mairie / ville / commune. Every C3r is IT "Comune
  di". No PL, DE, CZ or NL municipal word fires.
- **Department words on one side only.** There are 3, all R: "Asamblea de Concejales de la Mancomunidad …" ×2, and
  Tentudía's "(Turística y de Servicios)".
- **One-letter or numeral tokens that differ.** There are 24. All are articles or elisions (d', l', e/y, o/do), all R.
- **Legal-form tails that differ in tier A.** There are 33. All are the same entity: S.A. vs SAU, O.A. M.P., AöR,
  F.S.P., E.P.E., sp. z o.o. spelled out. None is a company against its owning council.
  - C1 needs T to equal the parent minus its organ exactly. A council can therefore never alias onto its company, and a
    company never onto its council.

### W shapes no saved row hits (synthetic, `check_fold.py synth`)

| # | Pair (T under P) | Tier today | Why it is W | Change | R cost |
|---|---|---|---|---|---|
| 1 | "Miasto Kłodzko" under "Gmina Kłodzko" (also Chełm, Siedlce, Sanok, Mielec, Zamość …); "Kłodzko" under "Gmina Kłodzko" | C2; C3 | In Poland a rural gmina and the town it is named after are two local-government units, each with its own REGON and NIP. `MUNI` makes `gmina` ≡ `miasto` | **Required:** drop `gmina` and `miasto` from `MUNI`. The rest of the list holds one entity per word | 0 (no PL C2, C3 or C3r fires) |
| 2 | "Madrid" under "Asamblea de Madrid"; "Extremadura" under "Asamblea de Extremadura" | C1 (one-token entity, 4+ chars) | A bare place under a regional parliament. fold.md's own "Never" says no bare place under anything but a town hall, and T4's 4-char guard does not enforce it (the same shape as T1's "Bilbao" W) | **Required:** a one-token entity only when the parent prints a legal form (all 7 saved ones do) | 0 |
| 3 | "Ayuntamiento de Alcobendas" under "Ayuntamiento de Alcobendas (Contratación y Compras)"; "Ayuntamiento de Sevilla" under "Ayuntamiento de Sevilla (TARCAS)" | A | Every parent bracket is cut, so the entity lands on a department or a review organ. This is T1/T3's W from the parent side, and the same pair reversed is already W under T3 | **Required:** keep a parent bracket in the parent key when it holds a department, registry or review word (`check_variants.py` `DEPT_OR_REVIEW`). A deny-list is acceptable on this side: the 35 saved parent brackets are acronyms and places | 0 |
| 4 | "Gobierno de Canarias" under "Vicepresidencia del Gobierno de Canarias"; "Principado de Asturias" under "Junta General del Principado de Asturias" | C1 | `presidencia` / `vicepresidencia` / `direccion general` / `junta general` / `asamblea` over a regional government or a parliament is a department or another institution, not an organ of that entity | **Recommended:** no cut under those organs when the entity starts with gobierno, govern, generalitat, xunta, principado, comunidad, comunitat, region or estado, or with junta plus a region. Keep "Consejo de Gobierno del Principado" (an organ, R) | 0 |
| 5 | "Gerencia … Área I" under "Gerencia … Área"; "Zona A" under "Zona"; "Hotel Balneario Spa" under "Hotel Balneario" | A | The one-letter article set drops a Roman numeral I and single capitals (ARMUVISSA I already folds to `armuvissa`). The trailing forms `spa`, `as`, `sal`, `em` are also words | Note only. It needs two such bodies inside one notice, parent and touchpoint, and no saved row is one | — |

- **Doc nit.** fold.md says "Dirección Provincial / Territorial" are deliberately not organs. But `direccion` is an
  organ, so `organ_split` cuts it from "Dirección Provincial de AVRA en Sevilla" and leaves "provincial avra sevilla".
  That is harmless, because no touchpoint prints that remainder, but the text is wrong.

### Unicode and locale: no trap

All of these come out as fold.md says (`check_fold.py synth`, pairs 31–42):
- **Turkish İ.** `lower()` gives `i` + U+0307, and U+0307 is in `NFKD_STRIPPED`'s 0300–036F block, so it goes. Neither
  Python nor Rust applies Turkish casing, so `I` becomes `i` on both.
- **ß / SS.** Both give `ss`.
- **Greek final sigma and tonos.**
- **Catalan l·l.** U+00B7 becomes a space; ŀ U+0140 decomposes to `l·` through NFKD.
- **Also:** decomposed accents, fullwidth letters and U+037E.

Notes for the port:
- `title_key_nfkd.rs` drops characters of non-zero combining class, the same rule as `unicodedata.combining() != 0`.
  It was generated from Unicode 14.0.
- Use `str::to_lowercase`, not `char::to_lowercase`. `EXTRA_FOLD`'s ς→σ makes the difference moot anyway.
- `char::is_alphanumeric` and `str.isalnum` differ only outside the table's blocks (Other_Alphabetic marks such as
  U+0345 and Indic vowel signs). `vectors.json` cannot see that, and EU notices do not reach it.
- "Il·lustre Col·legi" loses `il` as an Italian article. Both sides lose it alike, so nothing is wrongly equated.

## Interactions (both halves)

### D576-1 (i)

- The same-name alias stays the first test, and the tiers sit in the same `same` closure over every pair of names. The
  order alias → gate → drop classes → mint is unchanged.
- An aliased touchpoint gets no mention and is put in `retired` (`touchpoint_mentions`), exactly as (i) is today.

### `Touchpoints::retarget` and the ReviewOrg serve

- **Yes, an aliased Lot-ReviewOrg is re-pointed to the parent ORG's section**, and in these populations that section is
  the buyer.
- **It is the intended serve.**
  - The issue's done state for 540072 is `[22198704]`: the buyer and its review role are one organization.
  - In the saved pairs the touchpoint names the buyer entity itself. That is the "recurso ante el órgano de contratación"
    case, where the buyer is its own review body.
  - 530's W3 was a twin dissolve that merged rows with mixed roles. This is different: here the publisher put the body
    under the buyer's own ORG.
  - Where the parent ORG is the tribunal (the Andalusian TARCJA touchpoints), the alias goes to the tribunal. That is
    right too.
- **Side effect to put in the decision.** The party row is anchored at the parent's mention, so 456 design 3's
  `mention_name` serves the PARENT's printed name.
  - A tier-C ReviewOrg therefore reads "Pleno del Ayuntamiento de Marugán" where the notice printed "Marugán".
  - Under (i) the two names were equal under `org_name_norm`. Under tier C the touchpoint's own wording is not served.
  - That is N, not W.

### Counter

`TouchpointRoles` is built as a struct literal in tests (`supervisor.rs:23363`, `project.rs:25094`,
`tests/project.rs:7313`), and `touchpoint_suffix` prints `aliased` into the job row that unit 3 reads.
- Keep `aliased` as the total and add `aliased_a` / `aliased_c`. The norm count is the rest.
- Print the split in the suffix.
- Then the drain's "aliased above 67,434" compares like with like.

### Issue scope left uncovered (N, not W)

- The issue asked that an address, registry or contact-label SUFFIX on tier B be treated as the parent itself. The fold
  covers that only inside brackets and as a head label.
- "Gmina Panki ul. Tysiąclecia 5; 42-140 Panki" under "Gmina Panki" still mints, as fold.md says. The two Panki twins
  (31977139 / 31978194) therefore survive the drain as two orgs.
- Unit 2's named test has to use the synthetic U+037E pair.

## (b) The gate

### Fresh salted read of hits: 0 W

The read is a salt-ordered 40 of the 100 distinct hit names over every touchpoint population. All 40 are labels,
eForms role labels, prose, platform labels, one person ("ING. PAMELA COMI") or T-class compounds ("TACRC y art. 16 Ley
39/2015" ×5, "TACGal artículo 51.3", "TAR … ai sensi dell'art. 120"). The T class is the half's flagged judgment. I
agree with dropping them:
- the loss is a counted `dropped` review role, which is N;
- a prose-named TACRC duplicate is permanent.

### W in the saved data: LEGALREF drops a real buyer's name

- **The hit.** `check_gate.py orgs` runs the stems over 7,060 distinct names printed as ORGANIZATION sections or served
  as heads. These are real bodies: t2 parents, the windows' parents and older rows, and r4. LEGALREF hits:
  > "Stadt Dortmund, vertreten durch den Zweckverband Verkehrsverbund Rhein-Ruhr, dieser wiederum vertreten durch die
  > Verkehrsverbund Rhein-Ruhr AöR, handelnd als Gruppe von Behörden nach Art. 2 b) Verordnung (EG) Nr. 1370/2007"
- **Where it comes from.** This is the t2 parent of 31977017. German PSO buyers print their mandate this way ("zuständige
  Behörde nach Art. 2 lit. b VO (EG) 1370/2007").
- **Why it matters.** A touchpoint printing that name under any parent except one equal to it is dropped. That is a real
  body dropped.
- **Required fix:** LEGALREF does not fire on a name that cites an EU act (`verordnung`, `richtlinie`, `regulation`,
  `directive`, `reglamento`, `regolamento`, `directiva`, `direttiva`, or `eg` / `ewg` / `eu` / `ce` / `ue` / `cee` as a
  token). It also takes B's two guards: no public-law stem, no commercial form.
- **Cost:** 0. All 16 LEGALREF-only hits in every population carry none of these. Every LEGALREF+PROSE hit is still
  dropped by PROSE.

### W the half did not look for: bare organ and registry labels bind older generic rows

The half judged only hits on new orgs. In window A and window B, touchpoints printed as a bare organ or registry word
bind OLDER identifier-less rows that carry only that word:

| Touchpoint | Older row it binds | Its parent in the window |
|---|---|---|
| "Pleno" | 2130626 "Pleno" | Pleno de la Diputación Provincial de Huelva |
| "Rectorado" | 18945275 "Rectorado" | Rectorado de la UPM |
| "Junta de Gobierno Local" | 1166995 "Junta de Gobierno Local" | Junta de Gobierno del Ayuntamiento de La Nucía |
| "Registro General" | 1180214 "Registro General" | Junta de Gobierno del Ayuntamiento de Sevilla |

- **Why it is W.** It is the same mechanism as 1689043 "Ayuntamiento", which the decision gates: every buyer's bare
  label lands on one cross-body row by (name, country). That is W by the verdict's own hub definition.
- **Size.** 5 of 2,985 window mentions, counting "Ayuntamiento". Scaled to 2759's mint that is about 390 mentions on 4+
  rows.
- **Required fix**, one of:
  - (preferred, recovers R) a tier-(a) alias "T is exactly the parent's leading organ phrase, after synonyms, or the
    municipal head of its entity". That aliases Pleno, Rectorado, Junta de Gobierno (Local) and "Ayuntamiento" under
    "Alcaldía del Ayuntamiento de X" to their own parent. It is safe: "Alcaldía" under "Pleno" still differs.
  - or WHOLE stems: `pleno`, `rectorado`, `junta de gobierno`, `junta de gobierno local`, `alcaldia`, `presidencia`,
    `gerencia`, `registro`, `registro general`, `registro electronico`, `sede electronica`, `concello`, `ajuntament`.

  Either way "Registro General" needs the WHOLE stem.

### Recommended (0 in data)

- **B, honorific.** "Ing. Büro Schneider" drops: it has a space and no legal form. German planning offices are a
  plausible AddInfo or DocProvider touchpoint, and the half's own keep-list holds the "Ing. Büro … GmbH" shape. Do not
  fire when the next token folds to `buro` / `buero` / `bureau` / `studio`.
  - "Madame Tussauds" and "Monsieur Bricolage" without a form also drop. They are implausible as touchpoints, so no
    change is needed.
- **"M. le Médiateur interne de l UGAP"** (1 mention) is an institutional mediator under UGAP, the body a Lot-Mediator
  role names, not a private person. Exempting review and mediation words (`mediat`, `tribunal`, `recours`) from B would
  mint it instead of dropping it. This is N either way.
- **Platform own names** (Logintrade S.A., TUTTOGARE) drop as companies. That matches D576-1 (iv)'s existing treatment
  of "Mercell Norge AS" and "cosinex GmbH", so it is not a new W.

## Drain (unit 3)

- **retire_mentions retires mentions, not orgs.**
  - `refold-sections` re-queues the carriers. The incremental fold's `touchpoint_mentions` returns every Touchpoint
    section that no longer mints (aliased, gated or dropped) as `retired`.
  - `Db::retire_mentions` (`canonical.rs:22804`) then deletes those mention rows and the party and bid-party rows keyed
    to them, and stamps their Tenders stale.
  - It **never deletes an organization**. Its doc says so: "the organizations they bound are left to
    `sweep-orphan-orgs`".
- **The sweep is automatic.** A fold that retired any mention queues `sweep-orphan-orgs auto` itself
  (operations.md:3270). At or under 10,000 orphans (`AUTO_SWEEP_CAP`) it sweeps wet with no plan; above that it only
  records the plan.
  - **Expected:** at most about 866 + 92 orphans (minus the overlap) from the 3,751, plus 576 U5's share if it rides
    the same drain. That is well under the cap.
  - **Unit 3 reads** the auto-sweep job row (swept count) and a 404 on 31974991, 31975008 and 31975059.
  - **Not swept:**
    - aliased orgs that also hold mentions under other parents (fold.md's 3,356 is an upper bound);
    - older rows that keep their ORG mentions: 1689043, and the four bare-label rows above.
- **Wording to fix.** gate.md's "which `retire_mentions` or the sweep then removes" and the issue's "Confirm orphaned
  new orgs are retired by `retire_mentions`, else sweep" should both read: retire_mentions retires the mentions; the
  auto-queued sweep removes the orgs.

## Unit 2's named tests, adjusted

- AVRA: 6 alias, not 5.
- Gmina Panki: use the synthetic U+037E-vs-`;` pair, because the live rows are tier B.
- Add three must-not-alias tests: "Miasto Kłodzko" under "Gmina Kłodzko", "Madrid" under "Asamblea de Madrid", and
  "Ayuntamiento de Sevilla" under "Ayuntamiento de Sevilla (TARCAS)".
- Add a gate keep test: the Dortmund mandate name.
- Add a gate drop test, or an alias test if the tier is chosen: "Pleno" under "Pleno de la Diputación Provincial de
  Huelva".
- Regenerate `vectors.json` after changes 1–3. On the 3,751 rows they change no tier.
