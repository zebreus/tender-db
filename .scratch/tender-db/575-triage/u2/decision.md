# 575 unit 2 — decision (2026-10-24, reconciled after two critiques)

Inputs:
- the issue (Status, Root cause, Decision, Units);
- census job 2770 (`575-triage/hex_census_2770.json`, decoded body; max_id 31,978,722);
- the three readers in this directory: `buyer.md`, `targets.md` + `targets.json` (497 judged keys) and `random.md` + `random.json`;
- 530's `530-triage/decision.md` §A5–A8;
- code at HEAD `af959ff`: `project.rs` `gated_identifier` :5490 and its callers, `project/guid_twin.rs`, `store/src/hex_census.rs`
  and `guid_twins.rs`;
- issue 587;
- the two critiques `critique-pooling.md` and `critique-code.md`, each defect verified and applied or refuted (§ Challenge).

To check the guard below, I ran it over the readers' data (scratch script, not committed): over the 497 judged keys, the
random 60, the 2,000 buyer rows and the 16 multi-Tender rows. The draft wrote no repo file other than this one and made
no box read or GET. The reconciliation re-ran an independent emulation (it reproduces the draft's numbers exactly before
the amendments) and made 2 public GETs, at least 2 s apart: `tenders?buyer=31182564` and `organizations/8168470`.

## Decision

1. **Refuse** the cf35 key.
   - **What is refused.** A BT-501 whose raw value has the 35-character 8-4-4-16 shape (`store::is_cf35`) is not an
     identifier. The refusal sits in **`project::gated_identifier`**.
   - **Guard (i).** A compiled, 575-only name guard. When the guard hits, the key is **kept**: the mention binds or mints
     its own per-notice row, exactly as today. A hit never folds the mention into a name row, because a cf35 raw is
     also **outside 530-T** (§3): no other rule can refuse a kept key.
   - **Option (d) is declined** on the buyer-stratum evidence (§3).
2. **The R2 random 60 failed** as drawn: 8 W, all in the mediator stratum. The guard below was amended on that read and
   again on the critiques, so 530 §A5 applies.
   - A **fresh read at salt 575002** must pass before U4's wet run: a random 60 over the refused rows, with a buyer
     stratum, plus three exhaustive screens (mediator enumeration, name screen, **department screen**) (§5).
   - The fresh read does **not** hold U3's deploy (the velocity ruling). U3 ships the gate wired, and U4's rehome
     (re-runnable) converges anything the read later changes.
3. **No keep-list.** All 16 multi-Tender values are explained, and keeping any of them would preserve nothing (§4).
4. **The 369 buyer_key churn is written down.** About 3,550 notices / 2,900 Tenders change their plan column; 0 served
   change is expected and is asserted by a U3 census bar read **before any re-plan**.

## 1. Refuse: the category argument

**What the value is.** The value is the record key that Avenue-Web's AWS-Achat eSender (marches-publics.info, a
ColdFusion app) mints for **each ORG section of each notice**. It is published as BT-501 under scheme `eu`, in the
format of ColdFusion's `CreateUUID` (8-4-4-16). That shape is the publisher's own format, so the argument is about a
category, as for idgate's `notice_number`. It does not rest on name evidence. Census 2770 proves it at full scale:

| fact | job 2770 |
|---|---|
| class rows / cf35 mentions | 224,548 rows (all FR, all `ted`) / 224,610 mentions in 71,615 notices of 44,422 Tenders |
| single-use | one_mention 224,502; one_tender 224,532 |
| a value on two org rows | `values_multi_row` **0** |
| stable (≥ 100 mentions) | **0** |
| KEEP_530 overlap | **0** |
| multi-notice rows | 46, **all** in mixed-SDK profile buckets: a later notice of the same procedure re-publishing the original ORG sections; every single-profile bucket has 0 |
| one body under 2–3 values in one notice | 35,580 twin groups / 73,613 rows (values rise in section order, 4–6 hex prefix shared, adjacent ids) |
| one value naming two bodies | 23586902: an Avenue-Web SProvider cf35 key equals the dash-stripped `other`-shape raw of a CPAM Gironde "TRIBUNAL JUDICIAIRE" |
| growth | 2024 28,785 rows, 2025 108,436, **2026 87,327 by 10-24** |

So the key names one section of one notice lineage. It never names an organization across procedures.
- **What declining would cost.** About 224.5k one-mention rows, growing about 9k a month. These include 71,581 rows of
  Avenue-Web Systèmes itself and a row per notice for every tribunal administratif.
- **What 300 §84-85 still says.** Its stance ("merge-inert singletons") stands for hex values in general, and `condemns`
  and `uuid_v4` are unchanged.
- **How this amends it.** For this one platform shape the key is refused **at admission**, by raw shape. It is not
  condemned or dissolved by value. `raw_identifier` and `scheme` stay on every mention.
- **Scope.** The rule applies to any role and any country, and reads the raw shape only. The stored identifier cannot
  tell 8-4-4-16 from RFC 4122, but `gated_identifier` sees the raw value.

## 2. Guard (i): a compiled 575 list

**Why (ii), unguarded, is out.**
- The random 60 read 52 R / 0 N / **8 W**.
- 79 of the 497 judged target keys are W (75 as read, plus the 4 platform keys re-scored in the challenge), holding
  4,100 mentions.
- The buyer rows read 66 W in 2,000 (3.3%; 62 as read, plus 4 Rouvroy / Castres rows re-scored in the challenge).

The 0-W bar cannot be met without a guard.

**What a guard hit does.** `gated_identifier` returns the key as today: a `national` FR identifier. The mention binds
or mints its per-notice row, which is the status-quo split. It never lands on the pooled `(name_norm, FR)`
identifier-NULL row, and never on another body's row: 530-T never sees a cf35 raw (§3), so no twin refusal can fold it
either. Every hit can be undone later by a list removal, through U4's two-direction rehome.

**The guard, `cf35_keep_reason(name) -> Option<&'static str>`.** The checks run in this order; the first that matches
names the reason:

| # | reason | test | what it catches (evidence) |
|---|---|---|---|
| 1 | `blank` | `guid_twin_name_words(decoded)` is empty | no fallback on an empty name |
| 2 | `not_a_name` | `crate::partyname::not_a_name(name).is_some()` | 511's Blank / Placeholder values with two or more content words (`Sans objet`, `Non concerné`, `Non applicable`, `None specified`: 29 of `BLANK_WHOLE`'s 60). Keeps guard 3's verdicts unchanged (challenge C-D2). 0 hits in the readers' data |
| 3 | `content_words` | `content_words(words) < 2` (530's test) | `ccas`, `mairie`, `commune`, `ccira`, **`cciral`** (random #32), `semag`, `cdc`, `aws`, `ccsb`, `ehpad`, bare commune names (`Montfaucon`, `Gennes`) |
| 4 | `generic` | `generic_body_name(words)` **or** `generic_body_name(words with leading PREFIX_FUNCTION_WORDS dropped)` (481's list, reused as 530 reuses it) | `Centre communal d'action sociale` (± `(CCAS)`), `Centre intercommunal d'action sociale`, `Conseil départemental`, `Centre hospitalier`; with the strip, `Le Centre communal d'action sociale`, `Le Conseil départemental`, `La Communauté de communes` (challenge P-D3) |
| 5 | `committee` | `words.concat()` contains **`comiteconsultatif`** or **`comitesconsultatifs`** (`CF35_COMMITTEE_STEMS`) | every committee spelling in the data: all 52 CCIRA keys, `Inter-régional … (CCIRAL)` (3227980, random #45), `CCIRAL - Comité consultatif …` (12932404, #37), `… interdépartemental …` (1163657, #33/#40), `Comité consultatif de règlement amiable …` (1171446, 1201220), the fixture's long form, the SGAR form (19913046); and the plural `Comités consultatifs …` (no row in the data). A typo inside `consultatif` / `comité` escapes it; R2b(c) reads those (challenge P-D5) |
| 6 | `list` | the content-word fold equals an entry of `CF35_UNSAFE_NAMES` (51 entries) | bare court names, generic bodies, department labels, homonyms (below) |
| 7 | `list_prefix` | the content-word fold starts with an entry of `CF35_UNSAFE_PREFIXES` (2 entries) | boilerplate sentences published as names |
| 8 | `platform` | the name, lower-cased, contains `://` or `www.`; or the first content word is `plateforme` / `platform`, or the first two words are `plate forme` | a platform's name or URL published as the section's name: `plateforme AWS` (17943570), `http://marches-publics.info` (22125969), `http://agysoft.marches-publics.info` (13197923), `https://marches.megalis.bretagne.bzh` (minted). Each pools several buyers' sections (587's class; 586 D2 treats platform names as hub labels on touchpoints) (challenge P-D2) |

**Normalisation.**
- `decoded` is `name` with `&apos;`, `&#39;`, `&#039;`, `&amp;` and `&quot;` decoded. The decode is in the **guard's fold
  only**: the fallback key stays `org_name_norm(name)` of the published name, so the resolver is unchanged (challenge
  P-D4). Without it, `match_norm` reads `d&apos;action` as `d apos action`, and every generic or list entry with an
  apostrophe misses (`Communauté d&apos;agglomération`, `Office public de l&apos;habitat`, `Cour administrative
  d&apos;appel`; 7 escaped names are in the report).
- `words` is `guid_twin::guid_twin_name_words(decoded)`, 530's fold: `match_norm`, then `store::buyer_name_fold`, then
  split on spaces, `merge_initials`, legal forms dropped.
- The committee test concatenates those words, so a hyphen (`inter-régional`, `Comité-consultatif`) and a leading
  acronym cannot defeat it.
- The list and prefix tests compare the **content words**: `words` without `PREFIX_FUNCTION_WORDS`, joined by single
  spaces, with the entries folded the same way in a `OnceLock`. So `Le tribunal administratif`, `Tribunal administratif
  de`, `La société publique locale` and `Service marchés` / `Service des marchés` each meet one entry.
- The `platform` test reads the published name for `://` / `www.` (the fold has already turned them into spaces) and
  the folded words for the leading word.
- Not covered by any test: a placeless **trailing** qualifier (`CCAS de la ville`, `Centre communal d'action sociale de
  la ville`). No such name is in the data; R2b(c) reads placeless names.

**Why the census prefix is not reused.** `HEX_CENSUS_CCIRA_PREFIX` matches `starts_with` on `guid_twin_city_fold`, and
it let 3 of the random 60's W through: the hyphen (#45), the leading `CCIRAL -` (#37) and bare `cciral` (#32). That
constant stays as U1's report ruler. The gate gets its own stems.

**Why not `GUID_TWIN_UNSAFE_NAMES`.**
- Editing 530's list moves 530 and obliges a re-run of 530's recurrence arm.
- 575 needs none of its German entries.
- 575 calls the shared structural helpers (`content_words`, `generic_body_name`, `guid_twin_name_words`) directly, and
  **not** `guid_twin_unsafe_name`, which would pull 530's list in.
- `Tribunal administratif` sits on both lists, on purpose.

**Where it lives.** A new module, `crates/ingest/src/project/cf35.rs` (`pub mod cf35;` beside `pub mod guid_twin;` at
`project.rs:972`). It holds `pub const CF35_UNSAFE_NAMES: &[&str]`, `pub const CF35_UNSAFE_PREFIXES: &[&str]` and
`pub const CF35_COMMITTEE_STEMS: &[&str] = &["comiteconsultatif", "comitesconsultatifs"]`.

### `CF35_UNSAFE_NAMES`: 51 entries, each with its own content fold (pinned distinct)

Evidence: `T` = a judged target key in `targets.json` (id, verdict, cf35 mentions); `B` = rows in the 2,000-row buyer
sample; `P` = a precaution with no reader row (a name that denotes several bodies by construction).

| group | entries (spelling as listed) | evidence |
|---|---|---|
| bare court / registry names | `Tribunal administratif`; `Tribunal judiciaire`; `Tribunal de grande instance`; `Greffe du tribunal administratif`; `Greffe du tribunal judiciaire`; `Cour administrative d'appel` | T 1163218 W 281; T 10082662 W 148; T 1163205 W 1; P; P; P |
| CCAS family outside tests 2–4 | `Centre communale d'action sociale`; `Commune et CCAS`; `Groupement de commandes ville et CCAS` | B 1; T 31140454 W 2; B 1. (The draft's escaped `Centre communal d&apos;action sociale`, T missing W 25, is dropped: after the decode it is `generic`, and R0 #5 requires every entry's reason to be `list`.) |
| generic municipal / departmental bodies | `Hôtel de ville`; `Hôtel du département`; `Caisse des écoles`; `Communauté urbaine`; `Restaurant scolaire`; `Centre technique municipal`; `Service départemental d'incendie et de secours`; `Sce départemental incendie et secours`; `Centre de gestion de la fonction publique territoriale`; `Syndicat d'adduction d'eau potable`; `Syndicat mixte de traitement des ordures ménagères et autres déchets`; `Société publique locale`; `Établissement public foncier local`; `Préfecture de région`; `EPCI de Corse` | T 1163279 W 31 + 1228976 W 17; T 1185018 W 24 + 8262162 W 8; T 1201208 W 34, B 3; T 2102005 W 7, B 4; T 3457028 W 2; T 1179286 W 1; T 1299400 W 4; T 9485683 W 1; T 2560557 W 2; B 2; B 2; B 1; B 1; P; T (missing) N 9 |
| department labels (587's class) | `Service de la commande publique`; `Direction de la commande publique`; `Direction commande publique mutualisée`; `Commande publique`; `Service des marchés`; `Service des marchés publics`; `Service achats`; `Service achats/marchés`; `Service juridique et commande publique`; `Service commande publique et achat responsable`; `Direction de l'achat public`; `Direction des services techniques`; `Service technique de la ville`; `Groupement de commande`; `Profil acheteur`; `1 place de la Liberté` | T 3443 W 97 + 1229850 W 41; T 1166638 W 57 + 3426182 W 2; T 22569106 W 13; T 1186120 W 3; T 1363144 W 11 + 1179591 W 7; T 1163277 W 7 + 1181455 W 4 + 1298079 W 4; T 1163276 W 8; T 31975711 W 6; T 1419596 W 6; T (missing) N 2; T 9224696 W 6; T 1251437 W 8; T (missing) W 3; T 3416016 W 1; T 1728706 W 1, B 7 (`Profil d'acheteur :`); B 1 |
| homonyms (one name, several bodies) | `Mairie de Saint-Denis`; `Mairie de Saint-André`; `Mairie de Saint-Marcel`; `Mairie de Saint-Gervais`; `Commune de Saint-Just`; `Commune Le Vernet`; `Mairie de Leval`; `Collège George Sand`; `Lycée Paul Langevin`; **`Commune de Rouvroy`**; **`Commune de Castres`** | T 31429325 W 4 (GET: 93 and Réunion); B 1; B 1; T 22680256 N 2; B 1; B 1; B 1; T 3490748 W 3 (GET: FR82 + FRI34); B 7; **B 2 W** (rows 22754304, 22945986: Rouvroy, Aisne, co-buyers of Saint-Quentin's groupement in notices 24255934 / 24505874, Tender 1124918 FRE21; target 31182564 holds Rouvroy, Pas-de-Calais: GET `tenders?buyer=31182564` = two 2004 tenders, FR302); **B 2 W by name** (rows 22754298, 22945980: Castres, Aisne, would mint `commune de castres`; Castres, Tarn, publishes `Commune de Castres - …` and `Mairie de Castres`) |

**`CF35_UNSAFE_PREFIXES` (2):**
- `Pour obtenir tous renseignements complémentaires`: T (missing), W 10. The census row is truncated, so an exact entry
  could miss it.
- `Coordonnées des membres du groupement`: B 2, covering both the `Les …` and the bare spelling.

**What the guard covers, checked over the readers' files with the scratch emulation** (reconciled guard; the 4 platform
keys and the 4 Rouvroy / Castres rows re-scored W).

| set | refused (folds) | kept (stays split, as today) |
|---|---|---|
| 497 judged target keys | 336 keys: **335 R** (180,794 mentions), 1 N (3: `dajap - pap`), **0 W** | 161 keys: **all 79 W** (4,100), 66 R (2,119), 16 N (158). By reason: committee 57, content_words 52, list 39, generic 8, platform 4, prefix 1 |
| random 60 | 49 rows, **49 R** | 11 rows: all 8 W, plus 3 R (#35 and #36, committees naming their seat; #47 `SIMACUR`) |
| buyer 2,000 | 1,832 rows; **0** onto a W-judged target or a W name. All 66 W, the 15 homonym-risk rows and the 12 N acronyms are kept | 168 rows: content_words 109, list 39, generic 18, prefix 2 (8.4%; about 90 of them are one-word R names) |
| multi-Tender 16 | 15 (13 seat-named TA / greffe → R targets; 23126075 eSender → 8301019; 23586902's cf35 mention, named "Avenue-Web Systèmes", → 8301019) | 23126078 `CCIRAL` (content_words): its two mentions keep their one row |

The reconciliation's amendments (not_a_name, the article strip, the decode, the plural stem, `platform`, the two names)
change **no R verdict** in any of these sets: they add only the 4 platform keys (44 mentions) and the 4 buyer rows to
the kept side. The `not_a_name` count is an approximation (whole `BLANK_WHOLE` values; 0 hits in the 2,614 names in the
report); U3's census gives the exact figure.

**The cost of the guard, written down.**
- About **5% of the class stays split** (estimate; U3's census gives the exact figure). The mediator stratum is the bulk:
  9 of its 15 random rows are committee names, so roughly 10k of its 16,140 mentions stay split. Add about 8% of the
  buyer rows and a few percent of add_info. About **212k of the 224.6k mentions fold**.
- The seat-named committee spellings stay split too: 1,224 mentions among the judged keys, plus the R Versailles
  interdepartmental committee (413).
- **No seat exemption now.** A seat word list in the gate is a new ruler. Reopen it if U3's census shows a committee keep
  above 10k mentions **and** a seat list read at 0 W.

## 3. Placement: `gated_identifier`, not option (d)

The issue said "choose (d) only if U1 finds cf35 on buyer sections". U1 found 10,989, and `buyer.md` shows what they are:
- **Every one is a co-buyer** in a *groupement de commandes*. The role set is `Procedure-Buyer` alone, never combined.
- They are listed after a numeric lead buyer: 71,614 `digits` plus 1 `other` buyer mentions in the 71,615 notices.
- Their sections run ORG-0003 to ORG-0041, never ORG-0001 or ORG-0002.
- In 3 of the 4 notices read, the lead is printed again as a cf35 co-buyer. The lead keeps its numeric row under either
  placement.

**Comparing the two placements.**
- **Binding path.** It is identical under both: the mention gets `identifier = None` and resolves by name.
- **Published path** (`published_mentions` → `buyer_side_mentions` → 369's `buyer_key_of`, 481's tokens). Option (d)
  would keep a fresh hex key in each co-buyer's `buyer_key`. That has three costs:
  - each notice's buyer set stays unique: 100 of the sample's 531 Tenders hold 2–4 distinct sets, against 369's premise
    that a joint procurement repeats one set;
  - it protects no served field that the refusal endangers: `buyer_key` reaches a served `procedure_key` only through a
    refusal label, 369's (`key_shaped = 1`, `COUNT(DISTINCT buyer_key) >= 3`) or 482's UUID-hub label (`key_shaped =
    0`, the first member's `buyer_key`, `canonical.rs` `uuid_hub_label` / `refused:{key}:{label}`). Avenue-Web's BT-04
    values are per-procedure random v4 UUIDs, so neither is expected on the class, and the U3 bar (§4) catches both by
    the `refused:` prefix;
  - it leaves 481 a dead per-notice token.
- **530.** (d) would also need `guid_twin_gated_v4` to learn the shape, or 530's recurrence arm would report every
  refused cf35 v4 mention as reverse drift.
- **A role carve-out** (spare `Procedure-Buyer`) would keep about 2,900 Tenders' co-buyers split to dodge a 3.3% W that
  the guard already removes. It is declined.
- **530's precedent does not carry over.** 530 kept the buyer side on the published identity because there a refused
  GUID could be a notice's **whole** buyer key. Here a refused key is always a co-buyer beside a numeric lead.

**The result.** One identity for every consumer:
- the fold;
- `published_mentions` (369, 481);
- 511 guard 3;
- the 513 canary;
- 530-T's candidates and `guid_twin_gated_v4` (the recurrence arm), which exclude cf35 outright (below).

The guard reads the name, so the name joins the signature. Identity stays a **pure function of the stored row** (raw,
scheme, country, name). The canary and guard 3 therefore agree with the fold without the "contextual identity" caveat
530 carries.

**530 interplay: a cf35 raw is outside 530-T (amended in the challenge, C-D1 / C-D6 / P-D6).**
- `guid_twin_decide` skips a section whose raw is cf35 (`store::is_cf35`) before its candidate test, and
  `guid_twin_gated_v4` returns `None` for a cf35 raw. A refused cf35 value was no candidate anyway; a **kept** one is
  now no candidate either, so it binds its own key. Without this, a kept cf35 v4 pair under a name safe for 530 but
  kept by 575 (a committee, `Tribunal judiciaire`, `Service de la commande publique`) would be refused by 530-T and
  pooled onto the W rows 1163289 / 10082662 / 3443 (rare: about (1/64)² per twin group, but it falsified "a hit never
  folds").
- **`guid_twin_gated_v4` keeps its 3-argument signature.** For a non-cf35 raw the name cannot change
  `gated_identifier`'s answer, so it passes `""` inside. Store's `GuidTwinGatedV4`, its recurrence sites, the restore and
  `guid_twin_target_mentions_sql` are **unchanged** (§6.3 shrinks accordingly).
- **A later CF35 list addition can never show up as 530 reverse drift**, and 530's restore can never re-split a cf35
  raw. Mentions folded under a name that is listed later belong to 575's rehome (§6.9), re-run after each list commit.
- **Parity with 530's stock.** Its 18 dissolved FR rows sit under seat-named tribunals (plan 2671's FR targets: TA
  Toulouse, Lyon, Cergy-Pontoise, Toulon, Montpellier; TJ Marseille), which 575 refuses, so a rebuild lands them on the
  same NULL targets. Any one under a 575-kept name would rebuild split; U4's rehome converges it to the gate identity.
- KEEP_530 holds no cf35 value (overlap 0). `GUID_TWIN_UNSAFE_NAMES` is untouched. The change to 530-T's candidate set
  is covered by the one `guid-twin-recurrence` run after the deploy (§6.8), as 530's standing rule asks after a rule
  change.

## 4. The U2 bars

| bar | measured (job 2770, readers) | verdict |
|---|---|---|
| 0 multi-Tender, or a frozen keep-list | 16 values, 16 rows, 2–3 mentions each. 14 are one consultation's ORG keys re-sent by a subtype-38 modification that carries a new BT-04, so it lands in a second Tender; the targets are seat-named TA / greffe rows (R), plus CCIRAL 23126078. 1 is the eSender key re-used in a third notice (23126075 → 8301019, R). 1 is the cross-publisher weld 23586902, which the refusal separates | **PASS, no keep-list.** Every target is R, or the row is kept by the guard (23126078); keeping a value would preserve nothing its name row does not give. The re-emission is a 481/369 grouping matter, outside 575 |
| 0 stable | 0 | **PASS** |
| 0 KEEP_530 overlap | 0 | **PASS** |
| 0 buyer-role mentions, or 369's churn written down | 10,989 co-buyer mentions | **PASS, cost written down.** The plan column `buyer_key` changes for about 3,550 notices / about 2,900 Tenders: each co-buyer `FR:national:<hex>` becomes `n2:FR:<name>`, and the lead's digits key stays. `COUNT(DISTINCT buyer_key)` per key can only fall, so no 369 refusal can be added; one can be **lifted**, and a 482 UUID-hub label can be **renamed** (it embeds the first member's `buyer_key`). Either is a served `procedure_key` change. **Served churn expected: 0** (per-procedure random BT-04s form neither refusal). U3's census bar: 0 class Tenders with a cf35 buyer whose stored `procedure_key` starts with `refused:`, read as the **first job after the deploy**, before any `project` or daily job re-plans class Tenders (a re-plan would lift the refusal first and the bar would read 0 after the churn). Buyer W 66/2,000 are all kept by the guard |
| 0 blank-name mentions | 0 (`cf35_blank_name_mentions` counts an empty `org_name_norm` only, not 511's Blank class) | **PASS.** The guard keeps on blank **and** on any `partyname::not_a_name` value, so guard 3's verdicts cannot move: every Blank-class party name is either a `not_a_name` hit or a one-word scoped value (`ingen`, `nd`), which `content_words` keeps |
| R2 random 60: 0 W, N ≤ 3 | unguarded 52 R / 0 N / 8 W (mediator 7 R / 8 W). Under `targets.md`'s list: 54 refused, 51 R / 0 N / 3 W. Under the decided guard: 49 refused, all R; 11 kept | **FAIL as drawn.** The guard is tuned on it, so the 49/49 cannot be scored as a pass: **fresh 60 at salt 575002** (§5) |
| R2 top 30 targets: 0 W | 30 R / 0 W; all 30 refused under the guard; 168,076 mentions | **PASS.** Beyond the top 30: the 10 unjudged refused keys of top #31–#50 are 9 seat-named tribunals / greffes (R by name) and `CCRA 54` (#41, 494 mentions, target 8168470, 1,655 mentions, no readable city), judged **R** in the challenge: a numbered seat (54 = Meurthe-et-Moselle), planned city Nancy only, and the target is that same spelling's NULL row. R2b(c) re-reads it |
| every city-flagged target: 0 W | 436 read: 354 R / 15 N / 67 W (the draft's 19 N / 63 W, with the 4 platform keys re-scored W, 44 mentions) | **PASS after the list and the `platform` reason.** Every W is kept (§2), and the refused residue is 0 W. **Exhaustive over the A6 flags only:** a flag cannot fire on a target with no city (`hex_census.rs:1278`; 207 of the 442 found targets in the report's lists) or on two homonymous communes that print the same city. Rouvroy is that miss. R2b(e) closes it |
| every CCIRA homonym: 0 W | 52 keys: 33 R / 2 N / 17 W | **PASS.** All 52 are kept by the committee stems, so none folds. Cost: 1,224 R mentions stay split |
| buyer stratum (not in U2's R2 strata; `samples.buyer` is the first 2,000 by id, not random) | 1,907 R / 12 N / 66 W / 15 homonym risk (`buyer.md`'s 1,911 R / 62 W, with Rouvroy 2 + Castres 2 rows re-scored W in the challenge) | **Read, not a sample.** All non-R are kept (0 W refused), and the fresh 60 carries a buyer stratum of 15. `buyer.md`'s R verdicts on place-bearing communes rest on the name, which cannot see a homonym in another department: R2b(e) is the screen for that |

## 5. The R2 failure: what changed, and the fresh read (required)

**What changed** after the 8 W:
- the committee **stems** (contains, on the hyphen-joined fold; singular and plural) replace the census prefix;
- 530's `content_words < 2` is reused, which catches `cciral` and the acronyms;
- `generic_body_name` is reused, also on the words after a leading article;
- the 51 names and 2 prefixes come from all three readers and the challenge;
- the challenge added `not_a_name`, the entity decode and `platform`.

Under 530 §A5 a list tuned on a sample cannot pass on that sample, so a fresh read is **required**.

**R2b**, at salt **575002**, read from the U3 census guard mode (§6.5) right after U3's deploy:

- **(a) A random 60 over REFUSED class rows** (the guard applied), `guid_twin_sample_key(org_id, 575002)`.
  - Strata: review 20, sprovider 5, mediator 10, add_info 10, buyer 15.
  - The 60 rows of the 575001 draw are excluded.
  - Scoring: 530's R/N/W, name plus city, **plus department** (a commune name is not R on the name alone).
  - **Bar: 0 W, N ≤ 3.**
  - Why sprovider gets only 5: it is closed by enumeration (71,601 of its 71,602 mentions go to 8301019, R). The 15 for
    buyers close U1's gap.
- **(b) Enumeration:** every refused key holding a mediator-stratum mention, read whole. This is the random reader's
  finding: the target lists under-screened that stratum, p ≈ 0.0014. **Bar: 0 W.**
- **(c) Name screen:** every refused key not among the 497 judged in `targets.json`, read by name for placeless
  kind-of-body names (including a trailing `de la ville` / `de la commune`), department labels, homonyms, acronyms,
  committee spellings with a typo, platform names and junk, plus the census city flags. This closes `targets.md`'s
  stated limit. **Bar: 0 W.** It fails if the census list is truncated.
- **(d) The 16 multi-Tender rows and the canaries,** re-listed: 23126078 kept, the other 15 refused onto R targets.
- **(e) Department screen (added in the challenge, P-D1):** every refused key with a found target whose planned
  departments are not all among the target's. Planned side: the mention's ORG-section BT-512 postal department (first 2
  digits; 3 for 97x), else its BT-507 NUTS3. Target side: the same over its newest `GUID_TWIN_TARGET_NEWEST` mentions,
  else the NUTS3 of its Tenders (what caught 31182564). Every mismatch is read whole; keys whose target side has no
  department are counted (`dept_unscreenable`) and the top 100 by cf35 mentions are read. **Bar: 0 W.** It fails if the
  list is truncated.

**On a W:**
- **In (a):** the name joins `CF35_UNSAFE_NAMES` in a compiled commit, and a fresh 60 is drawn at **575003**,
  excluding every row already read.
- **In (b), (c) or (e):** the name joins the list with no re-draw, because those screens are exhaustive.
- **How additions ship:** always as compiled commits (gate, deploy). The gate never reads a runtime name list (530 A3
  iii). The census guard mode may take `extra_unsafe_names` for a dry measurement only.
- **After each list commit:** run 575's rehome (U4's job, dry then wet; its dry mode is 575's recurrence arm), which
  moves the mentions folded meanwhile under that name back onto their keys. 530 is not involved (§3).

**Timing (the velocity ruling).**
- U3 deploys with the gate wired, and R2b runs alongside it. R2b holds only **U4's wet run**.
- A W found after the deploy gets an operator-banner line ("575: new cf35 mentions under `<name>` folded by name since
  deploy #k; fix in deploy #k+1") and a list commit.
- U4's two-direction rehome (§6.9) moves those mentions back onto their keys.
- Exposure: about 290 cf35 mentions a day in total (87,327 rows in 2026 to 10-24). The W rate under the guard is
  **unmeasured** until R2b runs: the guard was tuned on every row read, and the challenge found a W (Rouvroy) among the
  R-judged buyer rows.

## 6. U3 build spec (M/L)

### 6.1 `crates/ingest/src/project/cf35.rs` (new)

- `pub const CF35_UNSAFE_NAMES: &[&str]` (§2, the 51 entries in the table's order, grouped by comment);
  `pub const CF35_UNSAFE_PREFIXES: &[&str]` (2); `pub const CF35_COMMITTEE_STEMS: &[&str] = &["comiteconsultatif",
  "comitesconsultatifs"]`.
- `fn cf35_guard_decode(name: &str) -> Cow<str>`: the five entities of §2, nothing else.
- `pub fn cf35_keep_reason(name: &str) -> Option<&'static str>` returns `"blank" | "not_a_name" | "content_words" |
  "generic" | "committee" | "list" | "list_prefix" | "platform"`, in §2's order. The folded entry sets are built once in
  a `OnceLock`.
- `pub fn cf35_refused(raw: &str, name: &str) -> bool { store::is_cf35(raw) && cf35_keep_reason(name).is_none() }`.
- **Module doc.** It says:
  - the category argument (§1);
  - what a guard hit does (keep the key, the status-quo split, never a pooled merge) and why that holds (530-T never
    sees a cf35 raw);
  - why the list is its own (§2);
  - that the census `HEX_CENSUS_CCIRA_PREFIX` is a report ruler, not the gate's;
  - that the entity decode is the guard's only and never the fallback key's;
  - that a list change is a compiled commit, requires the R2b discipline, and is followed by a 575 rehome run;
  - a pointer to this file.

### 6.2 The signature change

```rust
pub fn gated_identifier(raw: Option<&str>, scheme: Option<&str>, country: Option<&str>, name: &str) -> Option<Identifier> {
    raw.filter(|_| !scheme_never_keys(scheme))
        .filter(|r| !cf35::cf35_refused(r, name))
        .and_then(|raw| normalise_identifier(raw, country))
}
```

The doc comment gains the 575 paragraph: the refusal by raw shape, the guard, and that the name is the mention's
published name, the one the resolver falls back on.

530-T's two entry points exclude cf35 (§3) and keep their signatures:

```rust
// guid_twin_decide, first line of the candidate loop:
if s.raw_identifier.is_some_and(store::is_cf35) { continue; }   // issue 575: cf35 is 575's class
let Some(id) = gated_identifier(s.raw_identifier, s.scheme, country.as_deref(), s.name) else { continue };

pub fn guid_twin_gated_v4(raw: Option<&str>, scheme: Option<&str>, country: Option<&str>) -> Option<(String, Option<String>)> {
    if raw.is_some_and(store::is_cf35) { return None; }   // issue 575
    let country = country.map(canonical_country);
    let id = gated_identifier(raw, scheme, country.as_deref(), "")?;   // name is moot for a non-cf35 raw
    (id.kind == "national" && crate::idgate::uuid_v4(&id.value)).then_some((id.value, id.country))
}
```

`store::is_cf35` must be the census predicate (trimmed, case-blind), so the gate, 530-T and the census agree.

### 6.3 Every caller (file:line at HEAD `af959ff`; `canonical.rs` **and** `project.rs` carry another agent's uncommitted edits, so their working-tree lines are given too: `project.rs` sits 62 lines below `af959ff` from `:5193` on)

| # | site | change |
|---|---|---|
| 1 | `crates/ingest/src/project.rs:5490` (tree :5552) | the definition (§6.2) |
| 2 | `crates/ingest/src/project.rs:5193` (tree :5255) | guard 3 / placeholder identity: pass `names[0].0`. A Blank party's name is either a `not_a_name` hit or a one-word scoped value, both kept, so guard 3's verdicts are unchanged |
| 3 | `crates/ingest/src/project.rs:6269` (tree :6331) | the mention map: pass `&m.name` (after the renamed-heads loop: the name the resolver uses) |
| 4 | `crates/ingest/src/project.rs:19239`, `:19257` (tree +62) | tests: pass `""` (non-cf35 raws, unchanged) |
| 5 | `crates/ingest/src/project/guid_twin.rs:287` | `guid_twin_decide`: the cf35 skip, then pass `s.name` (§6.2) |
| 6 | `crates/ingest/src/project/guid_twin.rs:342-345` | `guid_twin_gated_v4`: the cf35 early return and `""` (§6.2); **signature unchanged** |
| 7 | `crates/ingest/src/project/guid_twin.rs:2631` | parity test (calls `gated_identifier` directly): pass `&m.name` |
| 8 | `crates/ingest/src/idgate.rs:1594`, `:1617` | tests: pass `""` |
| 9 | `crates/store/src/canonical.rs:9579` (tree :9635) | `NhsBodyCanaryArgs::identity: fn(Option<&str>, Option<&str>, Option<&str>, &str) -> Option<Identifier>` |
| 10 | `crates/store/src/canonical.rs:32967` (tree :33082) | canary re-resolve: pass `&name` |
| 11 | `crates/store/tests/nhs_body.rs:2280`, `:3651` | the closures take a fourth `_name` |
| 12 | `crates/app/src/supervisor.rs:14952` | `identity: ingest::project::gated_identifier`: retypes, no text change |
| 13 | doc text | `project.rs:4876`, `:6093`, `:6268` (tree +62 where below :5193); `guid_twin.rs:50`, `:264`, `:338-341` (the gated_v4 doc says cf35 is excluded); `supervisor.rs:2955`; `canonical.rs:9577-9578` (tree :9633-9634): say `(raw, scheme, country, name)` |

**No change** (the draft's sites 7, 8 and 15–19): the `guid_twin_gated_v4` tests (`guid_twin.rs:2274-2279`, `:2475`) and
its fn-pointer use (`:2511`); store's `GuidTwinGatedV4` (`guid_twins.rs:3334`), its sites `:3729`, `:3943`, the target
mentions SQL (`:3490-3495`, `:3855`), the test stub (`:4244`); `supervisor.rs:15179`, `:15465`, `:17447`.

The grep is complete: no other `gated_identifier` or `guid_twin_gated_v4` use exists under `crates/` (confirmed by the
code critique).

### 6.4 Behaviour that must not move

- `RecordedMention::publishes` compares name, country, raw and scheme, never the identifier. So the deploy moves **no
  standing mention** through the resolver, and 551, 539 and 561 refolds cannot half-dissolve the stock.
- Guard 3 cannot retire a standing identified-blank party: the 525 retire keys act before the resolver, which is why the
  `not_a_name` keep is required (§2).
- From the deploy, new notices fold under the gate.
- The fixture is not in `project_golden`, so no snapshot moves.

### 6.5 hex-census guard mode (dry; the R2b source)

- **`HexCensusArgs`** (`hex_census.rs:451`) gains:
  - `keep_name: Option<fn(&str, &[String]) -> Option<&'static str>>`. `None` is U1's behaviour; the job injects an
    ingest wrapper over `cf35_keep_reason` that also folds the run's extra names the gate's way (store cannot fold them:
    `match_norm` and `merge_initials` live in ingest), as `GuidTwinRule` does (`guid_twins.rs:283`);
  - `extra_unsafe_names: &'a [String]`, for a dry measurement only;
  - `salt: u64`.
- **The request.** `JobRequest` (`supervisor.rs:1229`, `#[serde(default)]`, unknown fields ignored) has no `guard` or
  `salt`, so as drafted `{"guard": true, "salt": 575002}` would be silently dropped and the job would run in U1 mode.
  Add `guard: Option<bool>` and `salt: Option<u64>` to `JobRequest` (`extra_unsafe_names` exists); read them in the
  parse arm (`:4946-4967`); print them in the params string (`hex-census dry guard=… salt=… extra=…`).
- **`Spec::HexCensus`** (`supervisor.rs:521`; run :16650-16680, :17379) gains `#[serde(default)] guard: bool`,
  `#[serde(default)] salt: Option<u64>` and `#[serde(default)] extra_unsafe_names: Vec<String>`. The default salt is
  `HEX_CENSUS_SALT`, so stored specs still parse. **Literals to update:** `supervisor.rs:25600`, `:25694` (built without
  `..`) and the `hex_census.rs` test `args()` literal (`:1737`).
- **Each cf35 mention** is judged on its own **published name**, not the row name (23586902; a 2–3-mention row may carry
  different names). A row with refused and kept mentions is reported as `mixed`.
- **The report** gains a `guard` block:
  - kept rows and mentions by reason × stratum; refused rows and mentions by stratum;
  - `kept_keys` (target key, reason, cf35 mentions, target id) and `refused_keys` (U1's target fields plus mentions per
    stratum), each with its **total** and a `truncated` flag (cap 5,000; job 2770 already had 4,847 target keys);
  - `dept_mismatch` (R2b(e): key, target id, planned departments, target departments and their source) with total and
    `truncated`, and `dept_unscreenable` (count, top 100 by cf35 mentions);
  - `r2_refused`: 60 rows at `salt` under `HEX_CENSUS_R2_GUARD_STRATA = [("review",20),("sprovider",5),("mediator",10),
    ("add_info",10),("buyer",15)]`, over refused rows only, excluding the rows U1's draw (`HEX_CENSUS_R2_STRATA` at
    `HEX_CENSUS_SALT`) picks;
  - **`buyer_refused_key_tenders`**: class Tenders that hold a cf35 `Procedure-Buyer` mention and whose
    `tenders.procedure_key` starts with `refused:` (369's and 482's labels alike), read by primary key over those
    Tenders. **Bar 0.**
- **The summary line** prints the kept share, the bar and any truncation.
- **The run:** one dry run on the box right after the deploy (class Z), `{"kind":"hex-census","guard":true,"salt":575002}`,
  queued **ahead of any `project` or daily job** (the queue serializes, so this costs no wait). If one ran first,
  `buyer_refused_key_tenders` is reported unmeasured on the banner.

### 6.6 R0 tests (`ops/check.sh`)

- **In `cf35.rs`:**
  1. **Refused under a safe name.** `gated_identifier(Some("D0E86056-B06B-F5C3-30CF1345EE066316"), Some("eu"), Some("FR"),
     "Tribunal administratif de Saint-Denis")` is `None`. The same holds for `Avenue-Web Systèmes`, and for the raw
     lower-cased or wrapped in spaces.
  2. **Kept by every reason.** Each of these returns `Some` with a FR `national` value of 32 hex:
     - blank: `""`;
     - not_a_name: `Sans objet`, `Non applicable`;
     - content_words: `CCAS`, `CCIRAL`;
     - generic: `Centre communal d'action sociale (CCAS)`, `Le Centre communal d'action sociale`, `Le Conseil
       départemental`, `Centre intercommunal d&apos;action sociale`;
     - committee: the fixture's long CCIRA form, `Comité consultatif Inter-régional de Règlement Amiable des Litiges
       (CCIRAL)`, `CCIRAL - Comité consultatif interrégional de règlement amiable des litiges`, `Comité consultatif
       interdépartemental de règlement amiable des litiges`, `Comités consultatifs interrégionaux de règlement amiable
       des litiges`;
     - list: `Le tribunal administratif`, `Service des marchés publics`, `Cour administrative d&apos;appel`, `Commune de
       ROUVROY`;
     - list_prefix: `Les coordonnées des membres du groupement sont précisées dans le règlement de la consultation
       (article 1-1)`;
     - platform: `plateforme AWS`, `http://marches-publics.info`, `https://marches.megalis.bretagne.bzh`.
  3. **Every convicted name is kept.** A test table holds every W name in this directory (the 79 W keys' `name_norm` in
     `targets.json` with the 4 platform keys, the 66-row W names in `buyer.md` with `Commune de ROUVROY` and `Commune de
     CASTRES`, random #31–#34, #37, #38, #40 and #45). Each must return `cf35_keep_reason(..).is_some()`. This pins the
     list against drift in `GENERIC_BODY_NAMES`, `PREFIX_FUNCTION_WORDS` or `partyname`.
  4. **The right folds still refuse:** `CCIRA de Bordeaux`, `CCRA 54`, `Tribunal administratif de Paris`, `Greffe du
     Tribunal administratif de TOULON`, `Ville de Bastia`, `Département de la Haute-Savoie`, `Toulouse Métropole`,
     `Commune de Toulouse`, `Avenue-Web Systèmes`, `Le centre communal d'action sociale de Vendôme` (the article strip
     does not over-keep), `Communauté d&apos;agglomération Cannes-Pays de Lérins` (the decode does not over-keep).
  5. **The list is well formed.** Every list and prefix entry folds non-empty and distinct (51 and 2), and every list
     entry's reason is `list`, meaning no structural test shadows it (no escaped entry remains).
  6. **Other shapes are untouched** under a safe name: the RFC dashed lower-case v4 `fb197f94-7578-4673-8a57-4642ae120532`,
     `30184`, `1748141-1-0-1` (U5's class), plain 32-hex, and a 36-character 8-4-4-4-12 value.
  7. **The decode is the guard's only:** a refused `Communauté d&apos;agglomération Cannes-Pays de Lérins` mention's
     fallback key is `org_name_norm` of the escaped published name (resolver unchanged).
- **In `guid_twin.rs`:**
  8. 277's v4-passing cf35, `guid_twin_gated_v4(Some("FD29FC2F-A2BF-49A1-9B234B870B531C27"), Some("eu"), Some("FRA"))`,
     is `None` (any cf35 raw is outside 530-T, whatever the name).
  9. `guid_twin_decide` over 277's TA Paris pair has 0 candidates, and so does a cf35 v4 pair under `Comité consultatif
     interrégional de règlement amiable des litiges` (kept by 575, safe for 530): the kept keys stay bound.
- **In `project.rs`:**
  10. Guard 3: a blank-name party and a `Sans objet` party, each carrying a cf35 raw, stay "identified blank".
- **Resolver (store or `project_incremental`):**
  11. A refused cf35 mention resolves onto the `(org_name_norm, FR)` identifier-NULL row, never an identified row.
  12. Two notices' TA Paris mentions land on one row.
  13. A refold of a recorded cf35 mention with unchanged raw keeps its organization (`publishes`).
- **In `hex_census.rs`:**
  14. On the existing fixture (`:1606` on), guard mode keeps `CCIRA_LONG` and `CCIRA_SGAR` (committee) and refuses the
      eSender and the TA Paris twin.
  15. `r2_refused` excludes U1's draw.
  16. `buyer_refused_key_tenders` counts a `refused:` Tender.
  17. A department mismatch (a fixture target whose mentions carry BT-512 `62…`, planned `02…`) is listed in
      `dept_mismatch`; a truncated list sets `truncated`.
- **In `supervisor.rs`:** 18. `{"kind":"hex-census","guard":true,"salt":575002}` parses to `Spec::HexCensus { guard:
  true, salt: Some(575002), .. }` and a stored U1 spec still parses.
- **Existing suites:** the 530 recurrence, planner and restore tests and the `nhs_body` canary tests stay green (the
  canary with the new fn type; 530's with unchanged types).

### 6.7 The fixture (`crates/ingest/tests/fixtures/eforms/cn-fa-expected-00586487-2024.xml`, issue 575 assertions)

Parse it with `crate::eforms::parse_payload("eforms:eforms-sdk-1.10", include_bytes!("../../tests/fixtures/eforms/…"))`.
Then, through `NoticeState::mentions_and_dropped(false, …)` **and** `published_mentions`:

| section | body | raw (scheme `eu`) | identifier |
|---|---|---|---|
| ORG-0001 | Avenue-Web Systèmes (SProvider, `ted-esen`) | `D0E86039-C408-988C-36D640329E1FF921` | **None**, raw and scheme kept |
| ORG-0002 | Département de la Réunion (buyer) | `30184` | `Some(FR national 30184)`, unchanged |
| ORG-0003 and ORG-0005 | Tribunal administratif de Saint-Denis (twin) | `D0E86056-…`, `D0E86078-…` | **None** for both; one shared fallback key `(org_name_norm, FR)` |
| ORG-0004 | Comité consultatif interrégional de réglement amiable … (Mediator) | `D0E86064-A547-F896-993DD7E12F048721` | **Some** (committee keep) |

### 6.8 CHANGELOG and /docs

- **CHANGELOG** (`## Unreleased (issue 575) — Avenue-Web's per-section platform keys no longer give a body a row per
  notice`):
  - **What happened.** The AWS-Achat eSender publishes a fresh 35-character 8-4-4-16 BT-501 for every ORG section of
    every notice. It was admitted as a FR `national` key, so census 2770 counted 224,548 one-mention organizations: 71,581
    rows of Avenue-Web Systèmes and a row per notice for each tribunal administratif.
  - **What changes.** From the deploy, such a value is not an identifier. The published value stays on the mention, and
    the mention joins the body's name-keyed organization.
  - **The exception.** Under a name not safe as a name key (blank or a non-name; fewer than two content words; a generic
    body name, with or without a leading article; any `Comité(s) consultatif(s) …`; a platform's name or URL; 51 listed
    names and 2 listed phrases), the value keeps binding its own organization, as before.
  - **Issue 530's twin rule** no longer reads this shape: it is 575's class alone.
  - **The buyer key.** In a joint procurement, a co-buyer's plan buyer key now reads by name. No served field changes.
  - **The standing rows** move in unit 4.
- **/docs**, `crates/app/src/v1/docs.rs` `identifier` bullet (:313): append after the 530 sentence. The docs file is
  edited in the working tree by another agent, so rebase onto it.

  > A value published in the 35-character 8-4-4-16 form (Avenue-Web's AWS-Achat eSender, marches-publics.info, mints
  > one per organization section per notice) is not used as an identifier: the mention joins the body's name-keyed
  > organization, except under a name too generic to stand for one body (a bare `CCAS` or `Tribunal administratif`, a
  > `Comité consultatif` review committee, a platform's name), where the value keeps its own one-mention organization
  > (issue 575).

  (The draft said "a review committee named without its seat"; the stems keep every committee spelling, seat-named or
  not.) Pin it with a docs test beside `the_docs_say_a_v4_uuid_is_a_platform_key_and_a_twin_drops_it` (:1714 in the
  tree), asserting:
  - "is not used as an identifier";
  - "mints one per organization section per notice";
  - "where the value keeps its own one-mention organization (issue 575)."

**After the deploy, alongside the work:**
- the guard-mode census (§6.5), queued first, then R2b (§5);
- one `guid-twin-recurrence` run (530's standing rule after a rule change), whose reverse drift must be unchanged or
  lower: every cf35 v4 value leaves the candidate set.

### 6.9 What stays for U4 (dissolve; blocked by R2b PASS, 579, and 551's W-on port-diff rule)

- **530's mechanics:**
  - an id-set selector, windows by target key, the 543 stop checkpoint, a `Box::pin` arm in `STOPPABLE_KINDS`;
  - `dry_run` true by default, a restore manifest, a wet re-plan;
  - the A2 per-mention rehome through `DissolveTarget::Resolver` (578), not 440's lookup.
- **One ruler, in both directions.** The planner re-derives each class notice's final identity with the gate's own
  functions: `gated_identifier` with the name (530-T no longer refuses a cf35 raw). It rehomes every mention whose bound
  org disagrees with that identity:
  - an identified singleton the gate refuses goes to its name row;
  - a mention on an identifier-NULL row whose key the compiled guard keeps goes back to its key. That covers mentions
    folded after the deploy under a name added later.
  - A row is dissolved whole when every mention moves.
- **Carve-outs (530's, challenge C-D7):** a mention a reviewed rehoming verdict names (`org_mention_rehoming`, as 530
  skips it: `reverse_drift_verdict_skipped`, `guid_twin_has_verdict`) is never moved; a mention that sits on the
  survivor of a merge that absorbed its class row is left there (counted, not moved). The census guard mode counts both.
- **Re-runnable.** U4's planner is a job, not a one-shot: its dry mode is 575's recurrence arm, and it runs (dry, then
  wet) after **every** CF35 list commit, before anyone reads a 575 R3. 530's arm and restore never see a cf35 raw (§3).
- **Tripwires:**
  - fewer than 100 mentions (the class maximum is 3);
  - multi-Tender only for the 16 ids of `samples.multi_tender` and rows minted above 31,978,722;
  - **23586902 is partial**: its non-cf35 "TRIBUNAL JUDICIAIRE" mention keeps the value as a one-mention hex row,
    harmless.
- **Expected:** about 212k rows and mentions moved, ≤ 44,422 Tenders changed, about 212k ids answering 404 once with no
  redirect. 8301019 (Avenue-Web Systèmes, identifier NULL) takes about 71.6k mentions.
- **428's fallback walk.** Some class rows sit below 8301019; they go as low as id 385 and 859 (re-read early
  notices). Until U4 removes them, 428's fallback walk passes those identified rows once per fold run per key; the run
  cache `name_of` absorbs repeats. That is a bounded cost, not a correctness issue.
- **579 parity:** U4 waits for 579's decision, or refolds the Tenders it touched.
- **R3:**
  - a fresh guard-mode census: 0 class rows identified where the gate refuses (the residue is the guard's kept rows),
    and R2b(e)'s department screen re-run over the name rows the fold minted since the deploy (a homonym that joined a
    minted row later, e.g. a second `Commune de <X>`);
  - `buyer_refused_key_tenders` 0;
  - the 530 recurrence quiet (unaffected by 575 list commits, §3).
- **U5** (the Avenue-Web tenderer keys `<procedure>-1-<lot>-<n>`) is its own issue and is unchanged.

## Challenge (2026-10-24: `critique-pooling.md` P-D1…D7, `critique-code.md` C-D1…D7)

Each defect was checked against the code at `af959ff` / the tree, the 2770 report, the readers' files and an independent
emulation of the guard (which reproduces the draft's 335 R / 5 N / 0 W refused, 157 kept, 49/49 random, 1,836 buyer
rows refused, before any amendment).

| # | defect | verdict | what was done |
|---|---|---|---|
| P-D1 major | Rouvroy (Aisne) folds onto Rouvroy (Pas-de-Calais); the A6 screen and R2b cannot see it | **APPLIED.** Verified: rows 22754304 / 22945986 are co-buyers with Saint-Quentin, Fonsomme, Harly, Homblières in notices 24255934 / 24505874; target 31182564; my GET `tenders?buyer=31182564` returns two 2004 FR302 tenders. `hex_census.rs:1278` sets `city_absent` 0 for an empty city set; 207 of the 442 found targets in the lists have none. `Commune de CASTRES` (22754298 / 22945980) mints a row Castres (Tarn) can join | `Commune de Rouvroy` and `Commune de Castres` listed (51 entries); buyer verdicts 1,907 R / 66 W; R2b(e) department screen and its census output; R0 #2/#3/#17; R3 re-runs the screen over minted rows |
| P-D2 major | platform / URL names refused while the same shape is W | **APPLIED.** Verified: the 4 keys are refused by the draft guard and scored N; each pools several buyers' sections (Montauban / Roubaix / Vire; Bourges / Crépy-en-Valois; an Illkirch mention onto a Marseille target). That is 587's pooling class, and 586 D2 already treats platform names as labels. Only these 4 (plus the boilerplate, already a prefix) carry a URL or a leading `plateforme` in the report | re-scored W (city-flagged 354 R / 15 N / 67 W); new `platform` reason (#8, last, so the boilerplate stays `list_prefix`); R0 #2/#3 |
| P-D3 / C-D3 minor | a leading article defeats `generic` | **APPLIED.** Verified in `generic_body_name` (`project.rs:11307-11327` at af959ff: it matches the whole word list, function words included). No bare article-plus-generic name is in the data; article-led names are (`La commune de Recy`, `Le centre communal d'action sociale de Vendôme`) | generic test also on the words after leading function words; R0 #2 and a not-over-keep case in #4. A placeless trailing qualifier (`CCAS de la ville`) stays with R2b(c) |
| P-D4 minor | `&apos;` spellings escape every apostrophe entry | **APPLIED.** Verified: `match_norm` reads `d&apos;` as `d apos`; 7 escaped names in the report; all six example generics/list names were refused | decode in the guard's fold only; escaped list entry dropped; R0 #2/#4/#5/#7 |
| P-D5 minor | the committee stem misses the plural; `CCRA 54` refused and unjudged | **APPLIED IN PART.** Verified: `comitesconsultatifs` does not contain the stem; no plural is in the data; `CCRA 54` is top #41, 494 mentions, refused, in no reader file. **Refuted:** listing it or adding a `ccra|ccira + number` keep shape: I judged it R (54 = Meurthe-et-Moselle, planned Nancy only, the target is that spelling's NULL row; GET `organizations/8168470`: name `ccra 54`, 1,655 mentions), so keeping it would only split 494 R mentions | plural stem added (`CF35_COMMITTEE_STEMS`); §2's "every spelling" reworded to "every spelling in the data", typos sent to R2b(c); `CCRA 54` judged in §4 and pinned as refused in R0 #4 |
| P-D6 / C-D6 minor | "a hit never folds" is false for a kept cf35 v4 twin under a 530-safe name | **APPLIED (stronger fix than either proposal).** Verified: `guid_twin_decide` (`guid_twin.rs:287-288`) takes any gated v4 value as a candidate, and committee names are not on 530's list. Fix: 530-T and `guid_twin_gated_v4` exclude every cf35 raw (§3, §6.2). Parity holds: 530's FR targets in plan 2671 are seat-named tribunals, which 575 refuses | §2 and §3 claims now true; R0 #8/#9 |
| P-D7 minor | the "W rate well below 1%" exposure figure has no evidence | **APPLIED.** The only random read failed and P-D1 found a W among R-judged rows | figure dropped (§5) |
| C-D1 major | a CF35 list addition after the deploy surfaces as 530 reverse drift, blocks 530's wet path, and 530's restore would re-split the v4 1/64 | **APPLIED** through the same cf35 exclusion: with `guid_twin_gated_v4` `None` for any cf35 raw, no 575 list change can reach 530's drift, wet path or restore. The remaining need (moving mentions folded before a list commit) is 575's: U4's rehome is a re-runnable job, run after every list commit. Side effect: `guid_twin_gated_v4` keeps its signature, so 7 of the draft's 20 call-site rows drop out of U3 (20 → 13) | §3, §5 "after each list commit", §6.2, §6.3, §6.9 |
| C-D2 minor | guard 3 is not unaffected: 511's two-word Blank values are refused | **APPLIED.** Verified: `BLANK_WHOLE` holds `sans objet`, `non concerne`, `non applicable` (`partyname.rs:627`, `:635`, `:636`); 29 of its 60 values have two or more content words and the draft guard refuses them; guard 3 (`project.rs:5191-5206` at af959ff) keeps a Blank party only on a gated identifier, and the 525 retire keys (`:6133`) act before the resolver, so a standing identified-blank mention would be retired on refold | `not_a_name` reason (#2); §4 blank bar and §6.3 #2 corrected; §6.4 line; R0 #2/#10 |
| C-D4 minor | census guard-mode spec gaps | **APPLIED, all five.** Verified: store cannot fold extra names (ingest's `match_norm`); `JobRequest` has no `guard` / `salt` and is `serde(default)` (`supervisor.rs:1229-1233`); `Spec::HexCensus` literals at `:25600`, `:25694` lack `..`; `hex_census.rs:1737` builds `HexCensusArgs`; 4,847 target keys against a 5,000 cap; rows can carry several names | `keep_name: fn(&str, &[String])`; `JobRequest` + parse arm + params; literals listed; totals and `truncated`, R2b(c)/(e) fail on truncation; per-mention judgement (§6.5, R0 #18) |
| C-D5 minor | the served-churn bar can read 0 after the churn; the "only key_shaped = 1" rationale is wrong | **APPLIED.** Verified: 482's `uuid_hub_label` (`canonical.rs:2486-2491` tree) labels a refused UUID-hub cluster with its first member's `buyer_key` (`refused:{key}:{label}`, `:20310` tree), a `key_shaped = 0` path; a re-plan can lift a 369 refusal before the census reads | rationale names 369 and 482 (§3, §4); the guard census is queued first, else the bar is reported unmeasured (§6.5) |
| C-D7 minor | U4's "rehome every disagreeing mention" needs 530's carve-outs | **APPLIED.** Verified: 530 skips verdict-named mentions (`guid_twins.rs:3450`, `:3527`) | carve-outs written into §6.9 |
| C-note | `project.rs` also carries uncommitted edits (+62 lines from `:5193`) | **APPLIED.** Verified: `gated_identifier` is at tree :5552 against af959ff :5490 | §6.3 header and tree lines |

Nothing in either critique was refuted outright; the only refusals are P-D5's proposed keep of `CCRA 54` (judged R
instead) and the two weaker fix variants of P-D6 / C-D6 / C-D1, superseded by the cf35 exclusion from 530-T.

## Status line for the issue

> U2 DECIDED 2026-10-24 (`575-triage/u2/decision.md`, reconciled with two critiques):
> - refuse cf35 in `gated_identifier`, guard (i); 530-T and `guid_twin_gated_v4` exclude every cf35 raw (cf35 is 575's
>   class alone; `guid_twin_gated_v4` keeps its signature);
> - a compiled `cf35.rs` guard: blank / not_a_name / content_words / generic (article-stripped) / committee stems
>   (singular + plural) / 51 names / 2 prefixes / platform name or URL; guard-only entity decode; kept key = status-quo
>   split;
> - no keep-list (16 multi-Tender explained);
> - buyer churn written down (served 0; bar read first after the deploy);
> - R2 random 60 FAILED 8 W (mediator) → guard amended → fresh 60 at 575002 plus mediator enumeration plus name screen
>   plus department screen required before U4's wet run.
>
> NEXT: U3 build (signature change, 13 call-site rows, the 530-T cf35 exclusion, census guard mode with the department
> screen, R0, fixture, CHANGELOG, docs).
