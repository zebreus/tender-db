# Audit: 576 U3 organization fan-out, verdict (2026-10-24)

**Verdict: there is a real defect. It is not a flood, and it is not about countries.** Job 2759's touchpoint
mint did not spray organizations: 3,751 new rows, 0 exact duplicates, and 89.7% of the minted mentions bound
rows that already existed. But it does two things the decision did not mean to allow, and no existing job
will ever undo either:

- **D1, self-duplicates of the buyer (mostly N).** A touchpoint that names its own parent body in any spelling
  `org_name_norm` does not fold gets minted as a second organization, or binds an older row that has no
  identifier. The certain floor is 74–107 new orgs; the classifier floor is 1,381 (37%); a hand read gives 60%.
- **D2, a gap in the touchpoint name gate (W pocket).** Contact labels, people, prose and platforms are minted
  as organizations, and three of them link unrelated buyers into one organization.

I sent no network calls, used no `/v1/sql` statements and made no GETs. I read only the three agents' saved
answers and re-tallied them locally, and I wrote only this file.

## Numbers that survived both skeptics

**Size of the mint**
- **New rows.** 3,751 organizations, ids 31,974,972–31,978,722, contiguous. 3,745 came from 2759 (created
  11:46:51Z) and 6 from a fold at 11:35:26Z.
- **No other creations.** The change log has exactly 3,751 `added` and 0 `removed` events, none above
  31,978,722 at 13:13Z, and ids are AUTOINCREMENT.
- **Shape of the new rows.** All are provisional, none has an identifier, all have a country, and every
  mention on them is a touchpoint mention: 24,101 from 2759, 24,664 on all 3,751 at 13:20Z.
- **Where the minted mentions went.** About 233k mentions were minted, about 2.1 per role reference (2759
  counted 491,287 minted, 67,434 aliased and 27,378 dropped references). 10.3% bound new orgs and about 209k
  (89.7%) bound existing rows. Three independent windows agree: 11.2%, 8.3% and 14.8% went to new orgs.

**Exact duplicates**
- There are 0 exact (`name_norm`, country) duplicates among the new orgs, upheld byte for byte.
- One Unicode miss: 31977139 and 31978194 (Gmina Panki) differ only by U+037E against ';'. `org_name_norm`
  does no Unicode normalisation, a gap that predates 576.

**Older same-country twins**
- 97 new orgs share their exact name with an older row in the same country. Every same-country twin (93 of 93)
  has an identifier, so the resolver never binds it by name.

**D576-1's alias works as written**
- 0 new orgs equal their own parent under `org_name_norm`. The gap is in the decision's key, not in the code.

## D1: self-duplicates of the buyer

**Mechanism.** D576-1 (i) aliases a touchpoint to its parent ORG only when the names agree under
`store::org_name_norm` (case, whitespace, trailing `.`, `,`, `;`). Everything else goes to (ii) mint. A
touchpoint that prints the parent under another spelling, legal form, bracket, acronym, or as the parent's legal
entity, becomes its own provisional row.

**Exhibit.** Tender 540072 serves the buyer twice under two organization ids:
- `Procedure-Buyer` is 22198704, "Sociedad Aragonesa de Gestión Agroambiental, S.L.U. (SARGA)".
- `Lot-ReviewOrg` on LOT-0001…0004 is 31975096, "Sociedad Aragonesa de Gestión Agroambiental S.L.U", which
  has 127 mentions.
- The two are equal once the comma and "(SARGA)" are dropped (`second-look/api/raw/tender-540072.json`).

**Size of the new orgs, by how the name relates to the touchpoint's own parent.** Both skeptics classified the
full population independently. Ranges are refute's key → second-look's key.

| Tier | Relation | New orgs | Their mentions | Defect? |
|---|---|---|---|---|
| A, spelling | Equal after folding accents, case, punctuation and brackets (second-look also drops legal forms) | 74 → 107 | 414 → 529 | **Yes, certain.** This is the (i) case D576-1 meant to alias. |
| B, parent + suffix | Parent's full name plus a department, address or truncation | 181 → 172 | 1,852 → 1,797 | Mostly the cost D576-1 accepted ("Stadt Fürth - Zentrale Vergabestelle", 556). An address, registry or "(Punto de contacto)" suffix is the parent itself. |
| C, part | A part of the parent's name, mostly the municipality or a bare town under its organ ("Marugán" under "Pleno del Ayuntamiento de Marugán") | 622 → 646 | 2,461 → 2,663 | **Yes by CONTEXT.md** ("one profile per real-world entity"). Aliasing it carries W risk (see unit 1). |
| D, overlap | Most of the parent's distinctive tokens | 456 (second-look only) | 2,159 | Mixed |
| **All self-shaped** | | **1,381 (36.8%)** | **7,148 (29% of new-org mentions)** | 1,369 of them sit under a parent that has an identifier |

**Hand read.** I re-tallied second-look's salted 40 (`raw/t2_salted40_labels.json`, salt
`576-second-look-salted-40`):
- Labels: SELF 24, DEPT 6, REVIEW 7, OTHER-BODY 1, LABEL 1, PLATFORM 1. SELF is 60% (95% interval about
  45–74%).
- I split the 24 SELF:
  - 5 are tier A: Ribagorza with or without "la"; IAJ with or without its bracket; Tías differing by accent
    and "del"; Comandancia Guardia Civil Cádiz; "AYUNTAMIENTO DE MADRID/DISTRITO DE CARABANCHEL".
  - 9 are a company or agency against its own organ or acronym: EPREMASA, GESPESA, Fira València, EMAYA,
    CHGUV, SEA 62, Consorcio Promedio, the La Laguna autonomous body, AMUE + SAJMP.
  - 9 are a municipality or bare town under its organ: Biar, Agüimes, Sant Antoni, Marugán, Bembibre, Estepa,
    Navas del Rey, Huéscar, Moaña.
  - 1 is a truncation that will turn into a label ("DIRECCION CENTRO PENITENCIARIO").
- So the indisputable tier is about 12% by hand, against 74–107 rows (2–3%) by classifier. The classifiers
  miss articles and prefixes.

**Existing rows too.** In two fresh notice windows, about 12% of the touchpoint mentions bound to existing rows
(232 of 1,909 and 91 of 770) went to an older row with no identifier that names the parent, for example 1217875
"Ayuntamiento de Alcobendas" while the buyer is identified 22179642.
- Scaled, that is about 25k of the 209k "safe" mentions.
- Only 0.3–0.5% are the spelling tier. 7–8% are tier C.

**Why it is a defect and not an accepted cost**
- D576-1 rejected "(b alone)" precisely to avoid "self-duplicates of the buyer", and priced the same-name share
  at ≥6%.
- In fact the alias caught 11.5% of references (67,434 of 586,099), and of what (ii) minted, 37% by
  classifier and 60% by hand name the parent or its legal entity. The accepted cost was meant to be a tail; it
  is the plurality of the mint.

**Permanent.** Refute checked each merge path against the code, and I confirmed the 351 scope in
`canonical.rs`. None will fold these rows:
- 351's `fold-provisional-echoes` folds only identical-name rows with a NULL country, and every new org has a
  country.
- 432's repair finds `name_norm` already correct on all 3,751.
- 234's merge uses the same key.
- `match_norm` folds only Greek tonos, and an N2 match is a candidate edge, never a merge (issue 300 §2.3 and
  §10).

The measurer's line "cleaning these is the job of the existing match and merge tools" (`measure.md:203`) is
refuted.

## D2: name-gate gap (labels, people, prose, platforms)

D576-1 (iv) gates only section-id shapes, URL, `www.` and `@`, and portal or platform names. Its bar was the
25 references in R3. None of the following has a stem.

**Labels that became organizations.** 9 orgs hold 2,926 mentions:

| Org | Name | Mentions | Distinct bodies behind it |
|---|---|---|---|
| 31975008 | Principal | 715 | About 4: umivale, URJC, Universitat de València, Alicante bodies. **Hub.** |
| 31975019 | Sede Tibidabo | 621 | 1 |
| 31974991 | Dirección Principal | 420 | 4: Madrid city organs, Diputación de Badajoz, Ayuntamiento de Mérida, Cabildo de Fuerteventura. **Hub.** |
| 31975180 | Info Adicional | 294 | 1 |
| 31975009 | RECURSOS | 268 | 1 |
| 31975006 | Dirección Postal | 221 | 1 (9 organs of the Cabildo de Gran Canaria). The measurer called it a hub; refute showed it is not. |
| 31975004 | Dirección Recepción de Recursos | 143 | 1 |
| 31975059 | Palacio Provincial | 139 | 2: Diputación de Cáceres 124, Diputación de Badajoz 15. **Hub the measurer missed.** |
| (id not re-read) | Dirección Recurso | 105 | 1 |

- **Older generic rows grew too.** 1689043 "Ayuntamiento" gained 125 of its 301 mentions from 26 parent orgs.
- **Refute's class N**: 18 clusters, 37 orgs, 853 mentions:
  - people: Melinda Wartan(-)Borbely; "Direction des Opérations – Franck DUJOUX / Nicolas BROSSIER";
  - prose: "Procede recurso especial…", "Ver apartado 17 de la HRC…" ×6;
  - compounds: "TACRC y art. 16 Ley 39/2015", "RED SARA/Comisión Jurídica", Logroño's "D.G. … o Juzgado";
  - a platform: "Plataforma Electrónica de Contratación de la Autoridad Portuaria de Barcelona";
  - registry offices.
- **People and prose outside the clusters**:
  - people: 31977749 Michaela Cechova (19), Miranda Vleeming (16), 31975438 "Monsieur Anthony LINTILHAC",
    31975293 "DOTT. ANGELO GAMBARDELLA", 31975359 "Judith COMMINSOLI";
  - a platform: 31978431 "TUTTOGARE RACCONIGI";
  - prose: "En los lugares establecidos en el artículo 16.4…".

## What is not a defect (refuted, or accepted by design)

- **A flood.** Refuted by all three agents (numbers above).
- **Wrong countries from the fallback.** Refuted:
  - 528 new orgs (14%) took the parent's country; 0 of the 25 with the most mentions are in the wrong country.
  - 20 new orgs (48 mentions) have a country other than their parent's. Most of these are genuine: Danish
    embassies, foreign art dealers.
  - 6 are wrongly coded, with 7 mentions in all: 31975548 Rechtbank Den Haag AU, 31975633 Rechtbank
    Gelderland MM, 31975146 Vergabekammer Baden-Württemberg BD, 31976459 EMAYA S.A. SE, 31976517 TACRC US and
    31977978 Tribunal Administratif de Melun PF. The last two are my own find in
    `second-look/raw/t2_classified.json`.
  - Every one of the 6 comes from the touchpoint's own published BT-514, which the ORG path reads the same way.
    That is publisher data, under 0.2% of the new orgs, and not 576's logic.
- **Higher served `mentions` on existing tribunal rows.** For example 2261 +4,394 (now 28,331), 2720295 +1,471,
  1209767 +2,986, 3708 +829, 6041 +987, and an estimated 40–48k on 3955. This is the intended effect of (ii):
  a touchpoint mention is an attestation of the body. Served names did not change, because heads are elected
  only on rows with an identifier.
- **Spelling variants of one tribunal.** Refute's class X: 23 clusters, 50 orgs, 340 mentions; the TACRC
  family gained 51 typo rows; Moldova's appeals authority has 6 translations. Issue 300 §10 accepts these (no
  fuzzy cross-notice merges), the same as on the ORG path.
- **The TACRC/TSJ compound 22077939**, which gained 2,531 mentions, was anticipated in D576-1's Size section.
- **Distinct bodies the loose key wrongly merged** (refute's class D: Consejería Salud Asturias against Murcia,
  Emscher against Lippe, …). The measurer over-counted here.

## Corrections to the measurer

- "Cleaned up by existing match and merge tools" is wrong (see "Permanent" above).
- Its 147 surplus rows (124 loose clusters) compared new orgs only with each other. They missed the main defect
  (D1), which needs a comparison with each org's own parent.
- About 40% of the loose clusters are real defects: refute's S 23 + N 18 of 125. 3 clusters are distinct
  bodies.
- On hubs: Dirección Postal is one publisher, Palacio Provincial is a hub it missed, and it did not check older
  generic rows such as "Ayuntamiento".
- 65 new orgs first appear on notices with ids 1–9,929, below the density probe's range. Their counts are
  unaffected.

## Proposed issue

**Title:** A touchpoint that names its own buyer in another spelling mints a second organization, and contact
labels mint hub organizations

**Kind:** data quality, organizations.
- Duplicate provisional profiles of the buyer (N, permanent).
- Label, person and prose organizations, three of which conflate unrelated buyers (W).

**Relates to:**
- 576 (D576-1 (i) and (iv); its U4 salted 30 should count D1 rows as N and D2 rows as W);
- 300 §10, 351, 234, 432 (none folds these rows);
- 456 (each party keeps its own published name);
- 510/511/520/525 (drop classes).

### What was found

As above:
- 3,751 new orgs, 0 exact duplicates.
- D1: 74–107 certain spelling self-duplicates (414–529 mentions); 1,381 self-shaped (7,148 mentions) by
  classifier; 60% by a salted read of 40; plus about 25k mentions bound to older identifier-less rows that
  name the parent.
- D2: 9 label orgs with 2,926 mentions (3 of them cross-body hubs), class N with 37 orgs and 853 mentions, and
  at least 7 people.
- Exhibit: tender 540072.

### Verify

    curl -s https://tenders.zebreus.click/v1/tenders/540072 | jq -c '[.parties[] | select(.role=="Procedure-Buyer" or .role=="Lot-ReviewOrg") | .organization_id] | unique'

- **done**: `[22198704]`. The review role binds the buyer's own organization.
- **open (2026-10-24 13:20Z)**: `[22198704,31975096]`.

Unit 4 reads the D2 half: `curl -s https://tenders.zebreus.click/v1/organizations/31974991 | jq .mentions`
answers 420 while open, and 0 or 404 when done.

### Proposed units

**1. Measure and decide (S).** It runs over the saved answers (`refute/raw/04`, `second-look/raw/02`,
`second-look/raw/t2_classified.json`), so it costs no box reads.

- **(a) The alias fold.** Compare a touchpoint only with its own parent ORG in the same notice:
  - Tier A folds accents (NFKC plus combining marks), case, all punctuation (U+037E, `:`, quotes, spacing
    around hyphens), bracketed text including "(Punto de contacto)" and "(PC)", legal-form tokens and
    articles.
  - Tier C covers the entity under its organ. The touchpoint is either:
    - the parent minus a leading organ phrase (Pleno, Alcaldía, Junta de Gobierno (Local), Presidencia,
      Gerencia, Consejo de Administración, Consejo Rector, Rectorado);
    - "Ayuntamiento/Ajuntament/Concello de <place>";
    - or the bare place name.
  - Read a salted 40 of what each tier would alias, against a 0-W bar: no review organ ("TARCAS. AYUNTAMIENTO
    DE SEVILLA", "Tribunal … de la Excma. Diputación", "Registro del Tribunal …") and no distinct department
    may alias to the buyer.
  - Tier B (parent plus a department) stays minted as D576-1 accepted, except an address, registry or
    contact-label suffix.
  - `store::org_name_norm` stays at spelling: canonical.rs:7432 says so on purpose, because the cross-notice
    reuse arm has no wall. The new fold lives in project.rs and compares only a nested pair within one notice,
    which issue 300 §10 does not forbid.
- **(b) Gate lists.** Add stems for:
  - the 9 labels above, "Ayuntamiento" alone and "Procurement Department";
  - honorifics: Monsieur, Madame, M., Mme, Dott., Dott.ssa, Sig., Herr, Frau, Mr, Mrs, Ing.;
  - prose: "procede recurso", "ver apartado", "en los lugares establecidos", references led by "art.";
  - platform names: TUTTOGARE, "Plataforma Electrónica de Contratación".

  The bar: it drops no class-X tribunal and no body in R3's top 60. Bare personal names with no honorific stay
  out of scope; count them.
- Record the result as a decision.

**2. Build (M).** In `crates/ingest/src/project.rs`, change `touchpoint_decision` (alias fold) and
`touchpoint_gate` (lists).
- Tests:
  - 540072 SARGA aliases;
  - AVRA ×5 under 1400401 alias;
  - Gmina Panki with U+037E aliases;
  - TARCAS under "Junta de Gobierno del Ayuntamiento de Sevilla" still mints;
  - "Stadt Fürth - Zentrale Vergabestelle" still mints;
  - "Dirección Principal" and "Monsieur Anthony LINTILHAC" drop;
  - the unit-3 re-mint drain test passes again.
- Split the `touchpoint_roles` aliased counter by tier.
- Gate through `ops/check.sh`.

**3. Drain (M).** Deploy, then run `refold-sections` with `profiles=["Touchpoint"]`: the same 198,857
carriers, about 45 min as 2758/2759 took.
- Confirm that the new orgs left with no mention are deleted by `retire_mentions` (unit 2's note); otherwise
  sweep them.
- Expect:
  - aliased above 67,434 and minted below 491,287;
  - the 3,751 new orgs to fall by at least tier A plus the gated rows (about 107 + 50, more if tier C is
    adopted);
  - the hubs to empty.
- Record the drain's tender ids for 551's switch-off list if it is still armed.

**4. Verify and read back (S).**
- Run the Verify line.
- Read 31974991, 31975008 and 31975059: each should be at 0 mentions or gone.
- Re-run the tier classifier over the surviving new orgs; tier A should be 0.
- Read a salted 30 of the surviving minted touchpoint orgs under 576 U4's R/N/W bar, 0 W.

**Out of scope, noted.**
- `org_name_norm`'s Unicode gap (U+037E; edge `:`, `.`, quotes) predates 576, and only a wall could make it
  safe.
- Bare generic tribunal names grew: 3708 by 829 from 98 parents, 6041 by 987 from 46. That is the existing
  reuse arm, not 576.
- Publisher-coded wrong BT-514, the same as on the ORG path.

## Evidence

All under `/home/user/tender-db/.scratch/tender-db/audit-576-fanout/`:
- `measure.md` and `box/`: the measurer;
- `refute/`: `selfdup_classes.json`, `clusters_with_parents.txt`, `raw/03-r3_hubs.json`,
  `raw/04-r4_parent_one.json`;
- `second-look/`: `raw/01-t1_changes.json`, `raw/t2_classified.json`, `raw/t2_salted40_labels.json`, the
  window files `raw/03-*-classified.json` and `raw/05-*-classified.json`, `raw/04-t4_served.json`,
  `api/raw/tender-540072.json`;
- code: `crates/ingest/src/project.rs` (`touchpoint_gate` near 7288, `touchpoint_decision` near 7370);
  `crates/store/src/canonical.rs` (`org_name_norm` at 7432, `fold_provisional_echoes` near 37883).
