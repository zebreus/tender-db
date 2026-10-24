# 575 U2 decision: adversarial critique, data-correctness lens (pooling)

Read-only. Sources: `u2/decision.md`, `u2/targets.md` + `targets.json`, `u2/random.md`, `u2/buyer.md`, census 2770
(`575-triage/hex_census_2770.json`), code at HEAD `ba21699` (`project.rs` `content_words` / `generic_body_name` /
`PREFIX_FUNCTION_WORDS`, `guid_twin.rs` `guid_twin_name_words`, `canonical.rs` `fold_latin` / `org_name_norm`,
`hex_census.rs` A6 screen). No cargo, no box, no `/v1/sql`. 11 public GETs on tenders.zebreus.click, one at a time,
at least 2 s apart.

**How I checked the guard.** I wrote a Python copy of `cf35_keep_reason`: the §2 order, the 50 names and the 2
prefixes typed in from the decision's table, and the real fold. It reproduces the decision's numbers exactly:
- the 497 judged keys: 340 refused (335 R / 5 N / 0 W), 157 kept (committee 57, content_words 52, list 40, generic 7,
  prefix 1);
- the buyer sample: 1,836 of 2,000 rows refused.

So the arithmetic holds. The defects below are W folds that the guard lets through, and two blind spots in the screens
that are meant to catch them.

## D1 (major). A confirmed W in the buyer stratum: Rouvroy (Aisne) folds onto Rouvroy (Pas-de-Calais). Neither the A6 screen nor R2b can see this class

**Evidence**
- `samples.buyer` rows 22754304 and 22945986, "Commune de ROUVROY". They are Procedure-Buyer co-buyers in notices
  24255934 and 24505874 (Tender 1124918). Their co-buyers are Ville de Saint-Quentin, Fonsomme, Harly, Homblières,
  Remaucourt and Castres. GET `/v1/tenders/1124918` gives country `FRE21` (Aisne).
- The guard refuses both rows: 3 content words, not generic, not listed. Their fallback target is **31182564**
  "COMMUNE DE ROUVROY".
- GET `tenders?buyer=31182564` returns two 2004 tenders, "F-Rouvroy: repair and maintenance services of central heating"
  and "… public-lighting installations", both NUTS `FR302` (Pas-de-Calais).
- So these are two communes: Rouvroy 02, about 400 inhabitants, and Rouvroy 62, about 8,500. That is a W under the issue's
  own definition ("is a different body"). `buyer.md` scored these rows R, and decision §2 counts them in "0 onto a
  W-judged target or a W name".
- The same notices also carry "Commune de CASTRES" (rows 22754298 and 22945980), which is Castres, Aisne. Its target is
  missing, so the fold mints the FR row `commune de castres`.
  - Castres, Tarn, publishes under that head: GET `organizations?name_prefix=Commune de Castres` lists 31159380 "COMMUNE
    DE CASTRES - MANDATAIRE SEM 81" and 31163876 (81 = Tarn).
  - The same sample also holds "Mairie de Castres" (Tarn), target 1603348.
  - So this is a W by name, the standard `targets.md` applies. It is the same class as the listed "Mairie de
    Saint-Marcel" and "Commune de Saint-Just", which were listed on the strength of 1 buyer row each.

**Why the planned screens cannot find it**
- `hex_census.rs:1278` sets `city_absent = 0` when the target's city set is empty. Pre-eForms targets have no BT-513, so
  31182564 reads `cities = []`.
  - Of the 442 found targets in the report's lists, **207 have an empty city set**. On those, only a multi-city planned
    key can fire.
  - Even when both sides have cities, homonymous communes print the **same** BT-513 string ("Rouvroy", "Castres",
    "Merville"), so neither flag fires.
- R2b(c), the name screen, reads "Commune de Rouvroy" as place-bearing and therefore R.
- R2b(a) draws 15 random rows from about 10k refused buyer rows, which has no power at this rate.
- So R2b can PASS while this W stands. "every city-flagged target: 0 W … exhaustive, needs no re-draw" is exhaustive only
  over flags that cannot fire for this shape.

**Fix**
1. Add `Commune de Rouvroy` and `Commune de Castres` to `CF35_UNSAFE_NAMES` now.
2. Give the U3 census guard mode a **department screen**: for each refused key with a found target, compare the
   department of the planned mentions with the target's. The planned side reads BT-507 NUTS3 or the first 2 digits of
   BT-512. The target side reads its newest-50 mentions' BT-507/BT-512, falling back to its Tenders' NUTS, which is what
   caught 31182564.
3. Read every mismatch, exhaustively, as part of R2b, with bar 0 W.
4. Add the R0 test 3 names.

## D2 (major). Platform and URL labels are refused and fold sections of several buyers into one row. They are scored N, while the same shape is W elsewhere

**Evidence (`targets.city_flagged`; all refused by the emulation)**

| key | cf35 | target | planned cities |
|---|---:|---|---|
| `plateforme AWS` | 10 | 17943570 (2 m) | Montauban, Roubaix, Vire Normandie |
| `http://marches-publics.info` | 30 | 22125969 (1 m) | Bourges, Crépy-en-Valois |
| `http://agysoft.marches-publics.info` | 1 | 13197923, target city Marseille | Illkirch-Graffenstaden |
| `https://marches.megalis.bretagne.bzh` | 3 | minted | Rennes + junk |

- `targets.md`'s reason for N, "the planned cities are the buyers'", concedes that each mention is a different
  buyer's section. After the fold, one row holds sections from 3 regions.
- The readers scored the same shape **W**: `profil acheteur` (1728706, "not a body", Audun-le-Tiche + Dreux / Marseille,
  listed), `groupement de commande` (W, listed) and the bare **`aws`** (1400495, W). `plateforme AWS` is that same
  platform label with one word added.
- 586 D2 treats "a platform's name" as a hub label (`touchpoint_names_586.rs` PLATFORM). That gate reads TouchPoint
  sections only, so it does not cover these ORG sections.
- Scored consistently, the refused judged set holds **3–4 W keys, about 44 mentions**, and §4's "city-flagged 0 W" bar
  fails.

**Fix**
- Add a `platform` keep reason: a folded name holding `http`/`https`/`www`, a domain token (`info`, `fr`, `com`, `bzh`
  after a dot in the raw name), or a leading `plateforme`/`platform`. Alternatively, list the 4 names.
- Re-score these keys W.
- Add an R0 case for each.

## D3 (minor). A leading article defeats the `generic` test

**Evidence**
- `generic_body_name` compares the whole word list, function words included. So in the emulation:
  - `Le Centre communal d'action sociale` → refused, content fold `centre communal action sociale`, which matches no list
    entry;
  - `Le Conseil départemental` → refused;
  - `Centre communal d'action sociale de la ville` → refused.
- §2's "`Le tribunal administratif` … meet one entry" holds for the **list** only, not for 481's generic names (CCAS,
  CIAS, Conseil départemental/régional, Communauté de communes/d'agglomération, OPH, CH, CHU).
- The class publishes these article forms: `le centre communal d'action sociale de vendôme` (city_flagged, 6),
  `le centre intercommunal d'action sociale de territoires vendômois` (buyer, 4 rows), and `la société publique locale`
  (buyer, W, which the list catches only through its content fold).
- No bare article-plus-generic name is in the 2770 data. This is a gap in the mechanism.

**Fix:** also test `generic_body_name` on the words with leading `PREFIX_FUNCTION_WORDS` stripped. Add R0 cases `Le
Centre communal d'action sociale` and `Le Conseil départemental` → kept.

## D4 (minor). HTML-entity spellings escape every generic and list entry that contains an apostrophe

**Evidence**
- 7 names in the report carry `&apos;`: the W `Centre Communal d&apos;Action Sociale` (25 mentions), `Communauté
  d&apos;agglomération Cannes-Pays de Lérins`, `Centre Hospitalier d&apos;Agen-Nerac`, `Ville d&apos;Ambarès et Lagrave`
  and others. So the escaping comes from the publisher and recurs; it is not a one-off.
- The guard handles only the one W, by an exact escaped entry. Each of these escapes the guard in the emulation:
  `Centre intercommunal d&apos;action sociale` (escaped twin of the W 1244397), `Communauté d&apos;agglomération`,
  `Office public de l&apos;habitat`, `Service départemental d&apos;incendie et de secours`, `Cour administrative
  d&apos;appel`, `Direction de l&apos;achat public`.
- `targets.md` "Limits" already said that decoding would fix this.

**Fix:** decode `&apos;`, `&#39;`, `&amp;` and `&quot;` in the guard's fold only (not in the fallback key). Then drop
the escaped list entry. Add an R0 case.

## D5 (minor). The committee stem does not cover "every review-committee spelling", and the largest refused key nobody judged is a committee

**Evidence**
- `CCRA 54`: top #41, 494 cf35 mentions, target 8168470 holding 1,655 mentions with `cities = []`, planned Nancy.
  - The guard refuses it: 2 content words, no `comiteconsultatif`.
  - No reader judged it: the top-30 bar stops at 30, and it is not city-flagged.
- The stem also misses the plural `Comités consultatifs …` (`comitesconsultatifs` does not contain
  `comiteconsultatif`) and any typo in `consultatif`/`comité`. The 52 spellings already include typos in other words
  (`diffférends`, `interrégionalde`).

**Fix**
- Judge `CCRA 54` now. Until then, keep it (list it) or add the shape `ccra|ccira|cciral` + one number to the committee
  test.
- Reword §2's claim.
- Add a plural spelling to the R0 committee cases.

## D6 (minor). "A hit never folds the mention into a name row" is not true for a kept value that passes v4

**Evidence**
- §3 keeps a kept cf35 v4 value (about 1/64 of values pass v4) as a 530-T candidate under **530's** guard.
- 530's guard has no committee stem and no 575 list. So a twin of `Comité consultatif interrégional de règlement amiable
  des litiges`, `Tribunal judiciaire` or `Service de la commande publique` is safe for 530. Its GUIDs are refused, and
  the mentions fold onto 1169609 / 10082662 / 3443, which are exactly the W rows 575 keeps them off.
- The expected count is tiny: 2 v4 passes in one twin group under a kept name.
- R0 test 3 checks only `cf35_keep_reason`.

**Fix:** in `guid_twin_decide`, drop a candidate whose raw is cf35 and whose name 575 keeps. 530's 18 dissolved FR rows
are seat-named TAs, which 575 refuses, so parity is untouched. Alternatively, state the exception and pin it with a
test.

## D7 (minor). The exposure estimate has no evidence behind it

§5 says "a W rate under the guard well below 1%". The guard was tuned on every row read, the only random read failed,
and D1 shows a W inside the R-judged buyer rows. Drop the figure until the R2b department and name screens have run.

## Checked and found sound

- Every one of the 75 judged W keys, the 8 random W rows and the 62 buyer W rows is kept.
- All 52 CCIRA keys and the three escaped random spellings (#32, #37, #45) are kept.
- Top 30: all R and all refused.
- Refused multi-city R keys (place-agreement pass, about 70): I found none that pools; they are seat-variant / cedex /
  member-commune spellings.
- Some homonym-prone refused buyer names have targets that GET shows are the same body: Valence (Drôme), Vernon (FR231),
  Bonneville (Haute-Savoie, FR718/FRK28), CH Saint-Louis (FRJ21, the Ariège GHT), EHPAD Victor Hugo (FR714).
- Merville (Nord): the target is inconclusive (1258004 has an FR 2008 tender with no NUTS).
