# 575 unit 2 — decision (2026-10-24)

Inputs:
- the issue (Status, Root cause, Decision, Units);
- census job 2770 (`575-triage/hex_census_2770.json`, decoded body; max_id 31,978,722);
- the three readers in this directory: `buyer.md`, `targets.md` + `targets.json` (497 judged keys) and `random.md` + `random.json`;
- 530's `530-triage/decision.md` §A5–A8;
- code at HEAD `af959ff`: `project.rs` `gated_identifier` :5490 and its callers, `project/guid_twin.rs`, `store/src/hex_census.rs`
  and `guid_twins.rs`;
- issue 587.

To check the guard below, I ran it over the readers' data (scratch script, not committed): over the 497 judged keys, the
random 60, the 2,000 buyer rows and the 16 multi-Tender rows. I wrote no repo file other than this one and made no box
read or GET.

## Decision

1. **Refuse** the cf35 key.
   - **What is refused.** A BT-501 whose raw value has the 35-character 8-4-4-16 shape (`store::is_cf35`) is not an
     identifier. The refusal sits in **`project::gated_identifier`**.
   - **Guard (i).** A compiled, 575-only name guard. When the guard hits, the key is **kept**: the mention binds or mints
     its own per-notice row, exactly as today. A hit never folds the mention into a name row.
   - **Option (d) is declined** on the buyer-stratum evidence (§3).
2. **The R2 random 60 failed** as drawn: 8 W, all in the mediator stratum. The guard below was amended on that read,
   so 530 §A5 applies.
   - A **fresh read at salt 575002** must pass before U4's wet run: a random 60 over the refused rows, with a buyer
     stratum, plus two exhaustive screens (§5).
   - The fresh read does **not** hold U3's deploy (the velocity ruling). U3 ships the gate wired, and U4 converges
     anything the read later changes.
3. **No keep-list.** All 16 multi-Tender values are explained, and keeping any of them would preserve nothing (§4).
4. **The 369 buyer_key churn is written down.** About 3,550 notices / 2,900 Tenders change their plan column; 0 served
   change is expected and is asserted by a U3 census bar.

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
- 75 of the 497 judged target keys are W, holding 4,056 mentions.
- The buyer rows read 62 W in 2,000 (3.1%).

The 0-W bar cannot be met without a guard.

**What a guard hit does.** `gated_identifier` returns the key as today: a `national` FR identifier. The mention binds
or mints its per-notice row, which is the status-quo split. It never lands on the pooled `(name_norm, FR)`
identifier-NULL row, and never on another body's row. Every hit can be undone later by a list removal, through U4's
two-direction rehome.

**The guard, `cf35_keep_reason(name) -> Option<&'static str>`.** The checks run in this order; the first that matches
names the reason:

| # | reason | test | what it catches (evidence) |
|---|---|---|---|
| 1 | `blank` | `guid_twin_name_words(name)` is empty | no fallback on an empty name; `cf35_blank_name_mentions` = 0, so guard 3 is unaffected |
| 2 | `content_words` | `content_words(words) < 2` (530's test) | `ccas`, `mairie`, `commune`, `ccira`, **`cciral`** (random #32), `semag`, `cdc`, `aws`, `ccsb`, `ehpad`, bare commune names (`Montfaucon`, `Gennes`) |
| 3 | `generic` | `generic_body_name(words)` (481's list, reused as 530 reuses it) | `Centre communal d'action sociale` (± `(CCAS)`), `Centre intercommunal d'action sociale`, `Conseil départemental`, `Centre hospitalier` |
| 4 | `committee` | `words.concat()` contains **`comiteconsultatif`** (`CF35_COMMITTEE_STEM`) | every review-committee spelling: all 52 CCIRA keys, `Inter-régional … (CCIRAL)` (3227980, random #45), `CCIRAL - Comité consultatif …` (12932404, #37), `… interdépartemental …` (1163657, #33/#40), `Comité consultatif de règlement amiable …` (1171446, 1201220), the fixture's long form, the SGAR form (19913046) |
| 5 | `list` | the content-word fold equals an entry of `CF35_UNSAFE_NAMES` (50 entries) | bare court names, generic bodies, department labels, homonyms (below) |
| 6 | `list_prefix` | the content-word fold starts with an entry of `CF35_UNSAFE_PREFIXES` (2 entries) | boilerplate sentences published as names |

**Normalisation.**
- `words` is `guid_twin::guid_twin_name_words(name)`, 530's fold: `match_norm`, then `store::buyer_name_fold`, then
  split on spaces, `merge_initials`, legal forms dropped.
- The committee test concatenates those words, so a hyphen (`inter-régional`, `Comité-consultatif`) and a leading
  acronym cannot defeat it.
- The list and prefix tests compare the **content words**: `words` without `PREFIX_FUNCTION_WORDS`, joined by single
  spaces, with the entries folded the same way in a `OnceLock`. So `Le tribunal administratif`, `Tribunal administratif
  de`, `La société publique locale` and `Service marchés` / `Service des marchés` each meet one entry.

**Why the census prefix is not reused.** `HEX_CENSUS_CCIRA_PREFIX` matches `starts_with` on `guid_twin_city_fold`, and
it let 3 of the random 60's W through: the hyphen (#45), the leading `CCIRAL -` (#37) and bare `cciral` (#32). That
constant stays as U1's report ruler. The gate gets its own stem.

**Why not `GUID_TWIN_UNSAFE_NAMES`.**
- Editing 530's list moves 530 and obliges a re-run of 530's recurrence arm.
- 575 needs none of its German entries.
- 575 calls the shared structural helpers (`content_words`, `generic_body_name`, `guid_twin_name_words`) directly, and
  **not** `guid_twin_unsafe_name`, which would pull 530's list in.
- `Tribunal administratif` sits on both lists, on purpose.

**Where it lives.** A new module, `crates/ingest/src/project/cf35.rs` (`pub mod cf35;` beside `pub mod guid_twin;` at
`project.rs:972`). It holds `pub const CF35_UNSAFE_NAMES: &[&str]`, `pub const CF35_UNSAFE_PREFIXES: &[&str]` and
`pub const CF35_COMMITTEE_STEM: &str = "comiteconsultatif"`.

### `CF35_UNSAFE_NAMES`: 50 entries, each with its own content fold (pinned distinct)

Evidence: `T` = a judged target key in `targets.json` (id, verdict, cf35 mentions); `B` = rows in the 2,000-row buyer
sample; `P` = a precaution with no reader row (a name that denotes several bodies by construction).

| group | entries (spelling as listed) | evidence |
|---|---|---|
| bare court / registry names | `Tribunal administratif`; `Tribunal judiciaire`; `Tribunal de grande instance`; `Greffe du tribunal administratif`; `Greffe du tribunal judiciaire`; `Cour administrative d'appel` | T 1163218 W 281; T 10082662 W 148; T 1163205 W 1; P; P; P |
| CCAS family outside tests 2–3 | `Centre communal d&apos;action sociale`; `Centre communale d'action sociale`; `Commune et CCAS`; `Groupement de commandes ville et CCAS` | T (missing) W 25; B 1; T 31140454 W 2; B 1 |
| generic municipal / departmental bodies | `Hôtel de ville`; `Hôtel du département`; `Caisse des écoles`; `Communauté urbaine`; `Restaurant scolaire`; `Centre technique municipal`; `Service départemental d'incendie et de secours`; `Sce départemental incendie et secours`; `Centre de gestion de la fonction publique territoriale`; `Syndicat d'adduction d'eau potable`; `Syndicat mixte de traitement des ordures ménagères et autres déchets`; `Société publique locale`; `Établissement public foncier local`; `Préfecture de région`; `EPCI de Corse` | T 1163279 W 31 + 1228976 W 17; T 1185018 W 24 + 8262162 W 8; T 1201208 W 34, B 3; T 2102005 W 7, B 4; T 3457028 W 2; T 1179286 W 1; T 1299400 W 4; T 9485683 W 1; T 2560557 W 2; B 2; B 2; B 1; B 1; P; T (missing) N 9 |
| department labels (587's class) | `Service de la commande publique`; `Direction de la commande publique`; `Direction commande publique mutualisée`; `Commande publique`; `Service des marchés`; `Service des marchés publics`; `Service achats`; `Service achats/marchés`; `Service juridique et commande publique`; `Service commande publique et achat responsable`; `Direction de l'achat public`; `Direction des services techniques`; `Service technique de la ville`; `Groupement de commande`; `Profil acheteur`; `1 place de la Liberté` | T 3443 W 97 + 1229850 W 41; T 1166638 W 57 + 3426182 W 2; T 22569106 W 13; T 1186120 W 3; T 1363144 W 11 + 1179591 W 7; T 1163277 W 7 + 1181455 W 4 + 1298079 W 4; T 1163276 W 8; T 31975711 W 6; T 1419596 W 6; T (missing) N 2; T 9224696 W 6; T 1251437 W 8; T (missing) W 3; T 3416016 W 1; T 1728706 W 1, B 7 (`Profil d'acheteur :`); B 1 |
| homonyms (one name, several bodies) | `Mairie de Saint-Denis`; `Mairie de Saint-André`; `Mairie de Saint-Marcel`; `Mairie de Saint-Gervais`; `Commune de Saint-Just`; `Commune Le Vernet`; `Mairie de Leval`; `Collège George Sand`; `Lycée Paul Langevin` | T 31429325 W 4 (GET: 93 and Réunion); B 1; B 1; T 22680256 N 2; B 1; B 1; B 1; T 3490748 W 3 (GET: FR82 + FRI34); B 7 |

**`CF35_UNSAFE_PREFIXES` (2):**
- `Pour obtenir tous renseignements complémentaires`: T (missing), W 10. The census row is truncated, so an exact entry
  could miss it.
- `Coordonnées des membres du groupement`: B 2, covering both the `Les …` and the bare spelling.

**What the guard covers, checked over the readers' files with the scratch emulation.**

| set | refused (folds) | kept (stays split, as today) |
|---|---|---|
| 497 judged target keys | 340 keys: **335 R** (180,794 mentions), 5 N (47: platform URLs as names, `plateforme aws`, `dajap - pap`), **0 W** | 157 keys: **all 75 W** (4,056), 66 R (2,119), 16 N (158). By reason: committee 57, content_words 52, list 40, generic 7, prefix 1 |
| random 60 | 49 rows, **49 R** | 11 rows: all 8 W, plus 3 R (#35 and #36, committees naming their seat; #47 `SIMACUR`) |
| buyer 2,000 | 1,836 rows; **0** onto a W-judged target or a W name. All 62 W, the 15 homonym-risk rows and the 12 N acronyms are kept | 164 rows: content_words 109, list 35, generic 18, prefix 2 (8.2%; about 90 of them are one-word R names) |
| multi-Tender 16 | 15 (13 seat-named TA / greffe → R targets; 23126075 eSender → 8301019; 23586902's cf35 mention, named "Avenue-Web Systèmes", → 8301019) | 23126078 `CCIRAL` (content_words): its two mentions keep their one row |

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
  - it protects no served field: `buyer_key` reaches a served `procedure_key` only for a refused key, which needs
    `key_shaped = 1`, and Avenue-Web's BT-04 values are random v4 UUIDs;
  - it leaves 481 a dead per-notice token.
- **530.** (d) would also need `guid_twin_gated_v4` to learn the shape, or 530's recurrence arm would report every
  refused cf35 v4 mention as reverse drift.
- **A role carve-out** (spare `Procedure-Buyer`) would keep about 2,900 Tenders' co-buyers split to dodge a 3.1% W that
  the guard already removes. It is declined.
- **530's precedent does not carry over.** 530 kept the buyer side on the published identity because there a refused
  GUID could be a notice's **whole** buyer key. Here a refused key is always a co-buyer beside a numeric lead.

**The result.** One identity for every consumer:
- the fold;
- `published_mentions` (369, 481);
- 511 guard 3;
- the 513 canary;
- 530-T's candidates and `guid_twin_gated_v4` (the recurrence arm).

The guard reads the name, so the name joins the signature. Identity stays a **pure function of the stored row** (raw,
scheme, country, name). The canary and guard 3 therefore agree with the fold without the "contextual identity" caveat
530 carries.

**530 interplay, stated.**
- 530-T is unchanged. Its candidates come from `gated_identifier`, so a refused cf35 value is never a candidate (the
  3,513 cf35-v4 values leave the candidate set).
- A **kept** cf35 v4 value stays a candidate under 530's own guard. That keeps parity with 530's stock: its 18 dissolved
  FR rows rebuild onto the same targets.
- `GUID_TWIN_UNSAFE_NAMES` is untouched.

## 4. The U2 bars

| bar | measured (job 2770, readers) | verdict |
|---|---|---|
| 0 multi-Tender, or a frozen keep-list | 16 values, 16 rows, 2–3 mentions each. 14 are one consultation's ORG keys re-sent by a subtype-38 modification that carries a new BT-04, so it lands in a second Tender; the targets are seat-named TA / greffe rows (R), plus CCIRAL 23126078. 1 is the eSender key re-used in a third notice (23126075 → 8301019, R). 1 is the cross-publisher weld 23586902, which the refusal separates | **PASS, no keep-list.** Every target is R, or the row is kept by the guard (23126078); keeping a value would preserve nothing its name row does not give. The re-emission is a 481/369 grouping matter, outside 575 |
| 0 stable | 0 | **PASS** |
| 0 KEEP_530 overlap | 0 | **PASS** |
| 0 buyer-role mentions, or 369's churn written down | 10,989 co-buyer mentions | **PASS, cost written down.** The plan column `buyer_key` changes for about 3,550 notices / about 2,900 Tenders: each co-buyer `FR:national:<hex>` becomes `n2:FR:<name>`, and the lead's digits key stays. `COUNT(DISTINCT buyer_key)` per key can only fall, so no refusal can be added. **Served churn expected: 0.** U3's census bar: 0 class Tenders with a cf35 buyer whose served `procedure_key` starts with `refused:`. Buyer W 62/2,000 are all kept by the guard |
| 0 blank-name mentions | 0 | **PASS** (the guard keeps on blank; guard 3 is unaffected) |
| R2 random 60: 0 W, N ≤ 3 | unguarded 52 R / 0 N / 8 W (mediator 7 R / 8 W). Under `targets.md`'s list: 54 refused, 51 R / 0 N / 3 W. Under the decided guard: 49 refused, all R; 11 kept | **FAIL as drawn.** The guard is tuned on it, so the 49/49 cannot be scored as a pass: **fresh 60 at salt 575002** (§5) |
| R2 top 30 targets: 0 W | 30 R / 0 W; all 30 refused under the guard; 168,076 mentions | **PASS** |
| every city-flagged target: 0 W | 436 read: 354 R / 19 N / 63 W | **PASS after the list.** Every W is kept (§2), and the refused residue is 0 W. The screen was exhaustive, so this needs no re-draw |
| every CCIRA homonym: 0 W | 52 keys: 33 R / 2 N / 17 W | **PASS.** All 52 are kept by the committee stem, so none folds. Cost: 1,224 R mentions stay split |
| buyer stratum (not in U2's R2 strata; `samples.buyer` is the first 2,000 by id, not random) | 1,911 R / 12 N / 62 W / 15 homonym risk | **Read, not a sample.** All non-R are kept (0 W refused), and the fresh 60 carries a buyer stratum of 15 |

## 5. The R2 failure: what changed, and the fresh read (required)

**What changed** after the 8 W:
- the committee **stem** (contains, on the hyphen-joined fold) replaces the census prefix;
- 530's `content_words < 2` is reused, which catches `cciral` and the acronyms;
- `generic_body_name` is reused;
- the 50 names and 2 prefixes come from all three readers.

Under 530 §A5 a list tuned on a sample cannot pass on that sample, so a fresh read is **required**.

**R2b**, at salt **575002**, read from the U3 census guard mode (§6.5) right after U3's deploy:

- **(a) A random 60 over REFUSED class rows** (the guard applied), `guid_twin_sample_key(org_id, 575002)`.
  - Strata: review 20, sprovider 5, mediator 10, add_info 10, buyer 15.
  - The 60 rows of the 575001 draw are excluded.
  - Scoring: 530's R/N/W, name plus city.
  - **Bar: 0 W, N ≤ 3.**
  - Why sprovider gets only 5: it is closed by enumeration (71,601 of its 71,602 mentions go to 8301019, R). The 15 for
    buyers close U1's gap.
- **(b) Enumeration:** every refused key holding a mediator-stratum mention, read whole. This is the random reader's
  finding: the target lists under-screened that stratum, p ≈ 0.0014. **Bar: 0 W.**
- **(c) Name screen:** every refused key not among the 497 judged in `targets.json`, read by name for placeless
  kind-of-body names, department labels, homonyms, acronyms and junk, plus the census city flags. This closes
  `targets.md`'s stated limit. **Bar: 0 W.**
- **(d) The 16 multi-Tender rows and the canaries,** re-listed: 23126078 kept, the other 15 refused onto R targets.

**On a W:**
- **In (a):** the name joins `CF35_UNSAFE_NAMES` in a compiled commit, and a fresh 60 is drawn at **575003**,
  excluding every row already read.
- **In (b) or (c):** the name joins the list with no re-draw, because those screens are exhaustive.
- **How additions ship:** always as compiled commits (gate, deploy). The gate never reads a runtime name list (530 A3
  iii). The census guard mode may take `extra_unsafe_names` for a dry measurement only.

**Timing (the velocity ruling).**
- U3 deploys with the gate wired, and R2b runs alongside it. R2b holds only **U4's wet run**.
- A W found after the deploy gets an operator-banner line ("575: new cf35 mentions under `<name>` folded by name since
  deploy #k; fix in deploy #k+1") and a list commit.
- U4's two-direction rehome (§6.9) moves those mentions back onto their keys.
- Exposure: about 290 cf35 mentions a day in total (87,327 rows in 2026 to 10-24), and a W rate under the guard well
  below 1%.

## 6. U3 build spec (M/L)

### 6.1 `crates/ingest/src/project/cf35.rs` (new)

- `pub const CF35_UNSAFE_NAMES: &[&str]` (§2, the 50 entries in the table's order, grouped by comment);
  `pub const CF35_UNSAFE_PREFIXES: &[&str]` (2); `pub const CF35_COMMITTEE_STEM: &str = "comiteconsultatif"`.
- `pub fn cf35_keep_reason(name: &str) -> Option<&'static str>` returns `"blank" | "content_words" | "generic" |
  "committee" | "list" | "list_prefix"`, in §2's order. The folded entry sets are built once in a `OnceLock`.
- `pub fn cf35_refused(raw: &str, name: &str) -> bool { store::is_cf35(raw) && cf35_keep_reason(name).is_none() }`.
- **Module doc.** It says:
  - the category argument (§1);
  - what a guard hit does (keep the key, the status-quo split, never a pooled merge);
  - why the list is its own (§2);
  - that the census `HEX_CENSUS_CCIRA_PREFIX` is a report ruler, not the gate's;
  - that a list change is a compiled commit and requires the R2b discipline;
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
published name, the one the resolver falls back on. `guid_twin_gated_v4` gains `name: &str` and passes it through.

### 6.3 Every caller (file:line at HEAD `af959ff`; `canonical.rs` carries another agent's uncommitted edits, so its working-tree line is given too)

| # | site | change |
|---|---|---|
| 1 | `crates/ingest/src/project.rs:5490` | the definition (§6.2) |
| 2 | `crates/ingest/src/project.rs:5193` | guard 3 / placeholder identity: pass `names[0].0`. For a Blank or Placeholder party the name is blank or void, so the guard keeps the key and guard 3's verdicts are unchanged |
| 3 | `crates/ingest/src/project.rs:6269` | the mention map: pass `&m.name` (after the renamed-heads loop: the name the resolver uses) |
| 4 | `crates/ingest/src/project.rs:19239`, `:19257` | tests: pass `""` (non-cf35 raws, unchanged) |
| 5 | `crates/ingest/src/project/guid_twin.rs:287` | `guid_twin_decide`: pass `s.name` |
| 6 | `crates/ingest/src/project/guid_twin.rs:342-345` | `guid_twin_gated_v4(raw, scheme, country, name)` |
| 7 | `crates/ingest/src/project/guid_twin.rs:2274-2279`, `:2475` | tests: add a safe DE name (`Landkreis Göttingen`) |
| 8 | `crates/ingest/src/project/guid_twin.rs:2511` | `gated_v4: guid_twin_gated_v4`: retypes, no text change |
| 9 | `crates/ingest/src/project/guid_twin.rs:2631` | parity test: pass `&m.name` |
| 10 | `crates/ingest/src/idgate.rs:1594`, `:1617` | tests: pass `""` |
| 11 | `crates/store/src/canonical.rs:9579` (tree :9635) | `NhsBodyCanaryArgs::identity: fn(Option<&str>, Option<&str>, Option<&str>, &str) -> Option<Identifier>` |
| 12 | `crates/store/src/canonical.rs:32967` (tree :33082) | canary re-resolve: pass `&name` |
| 13 | `crates/store/tests/nhs_body.rs:2280`, `:3651` | the closures take a fourth `_name` |
| 14 | `crates/app/src/supervisor.rs:14952` | `identity: ingest::project::gated_identifier`: retypes, no text change |
| 15 | `crates/store/src/guid_twins.rs:3334` | `GuidTwinGatedV4 = fn(Option<&str>, Option<&str>, Option<&str>, &str) -> Option<(String, Option<String>)>` |
| 16 | `crates/store/src/guid_twins.rs:3729`, `:3943` | pass `&m.name` |
| 17 | `crates/store/src/guid_twins.rs:3490-3495` and `:3855` | `guid_twin_target_mentions_sql` also selects `name` (column 6); pass `opt_text_of(&row, 6).unwrap_or_default()` |
| 18 | `crates/store/src/guid_twins.rs:4244` | test stub `gated_v4` takes `_name: &str` |
| 19 | `crates/app/src/supervisor.rs:15179`, `:15465`, `:17447` | `gated_v4: …guid_twin_gated_v4`: retypes, no text change |
| 20 | doc text | `project.rs:4876`, `:6093`, `:6268`; `guid_twin.rs:50`, `:264`, `:338-341`; `supervisor.rs:2955`; `canonical.rs:9577-9578` (tree :9633-9634); `guid_twins.rs:3333`, `:3339`, `:3416`, `:3620`, `:3839`, `:4242`: say `(raw, scheme, country, name)` |

The grep is complete: no other `gated_identifier` or `GuidTwinGatedV4` use exists under `crates/`.

### 6.4 Behaviour that must not move

- `RecordedMention::publishes` compares name, country, raw and scheme, never the identifier. So the deploy moves **no
  standing mention**, and 551, 539 and 561 refolds cannot half-dissolve the stock.
- From the deploy, new notices fold under the gate.
- The fixture is not in `project_golden`, so no snapshot moves.

### 6.5 hex-census guard mode (dry; the R2b source)

- **`HexCensusArgs`** (`hex_census.rs:451`) gains:
  - `keep_name: Option<fn(&str) -> Option<&'static str>>`. `None` is U1's behaviour; the job injects
    `ingest::project::cf35::cf35_keep_reason`;
  - `extra_unsafe_names: &[String]`, for a dry measurement only;
  - `salt: u64`.
- **`Spec::HexCensus`** (`supervisor.rs:521`; parse :4965; run :16650-16680) gains `#[serde(default)] guard: bool` and
  `#[serde(default)] salt: Option<u64>`. The default is `HEX_CENSUS_SALT`, so stored specs still parse.
- **Each class row** is judged on its cf35 mention's **published name**, not the row name (23586902).
- **The report** gains a `guard` block:
  - kept rows and mentions by reason × stratum; refused rows and mentions by stratum;
  - `kept_keys` (target key, reason, cf35 mentions, target id; cap 5,000);
  - `refused_keys` (U1's target fields plus mentions per stratum; cap 5,000);
  - `r2_refused`: 60 rows at `salt` under `HEX_CENSUS_R2_GUARD_STRATA = [("review",20),("sprovider",5),("mediator",10),
    ("add_info",10),("buyer",15)]`, over refused rows only, excluding the rows U1's draw (`HEX_CENSUS_R2_STRATA` at
    `HEX_CENSUS_SALT`) picks;
  - **`buyer_refused_key_tenders`**: class Tenders that hold a cf35 `Procedure-Buyer` mention and whose
    `tenders.procedure_key` starts with `refused:`, read by primary key over those Tenders. **Bar 0.**
- **The summary line** prints the kept share and the bar.
- **The run:** one dry run on the box right after the deploy (class Z), `{"guard": true, "salt": 575002}`.

### 6.6 R0 tests (`ops/check.sh`)

- **In `cf35.rs`:**
  1. **Refused under a safe name.** `gated_identifier(Some("D0E86056-B06B-F5C3-30CF1345EE066316"), Some("eu"), Some("FR"),
     "Tribunal administratif de Saint-Denis")` is `None`. The same holds for `Avenue-Web Systèmes`, and for the raw
     lower-cased or wrapped in spaces.
  2. **Kept by every reason.** Each of these returns `Some` with a FR `national` value of 32 hex:
     - blank: `""`;
     - content_words: `CCAS`, `CCIRAL`;
     - generic: `Centre communal d'action sociale (CCAS)`;
     - committee: the fixture's long CCIRA form, `Comité consultatif Inter-régional de Règlement Amiable des Litiges
       (CCIRAL)`, `CCIRAL - Comité consultatif interrégional de règlement amiable des litiges`, `Comité consultatif
       interdépartemental de règlement amiable des litiges`;
     - list: `Le tribunal administratif`, `Service des marchés publics`;
     - list_prefix: `Les coordonnées des membres du groupement sont précisées dans le règlement de la consultation
       (article 1-1)`.
  3. **Every convicted name is kept.** A test table holds every W name in this directory (the 75 W keys' `name_norm` in
     `targets.json`, the 62-row W names in `buyer.md`, random #31–#34, #37, #38, #40 and #45). Each must return
     `cf35_keep_reason(..).is_some()`. This pins the list against drift in `GENERIC_BODY_NAMES` or
     `PREFIX_FUNCTION_WORDS`.
  4. **The right folds still refuse:** `CCIRA de Bordeaux`, `Tribunal administratif de Paris`, `Greffe du Tribunal
     administratif de TOULON`, `Ville de Bastia`, `Département de la Haute-Savoie`, `Toulouse Métropole`, `Commune de
     Toulouse`, `Avenue-Web Systèmes`.
  5. **The list is well formed.** Every list and prefix entry folds non-empty and distinct, and every list entry's
     reason is `list`, meaning no structural test shadows it.
  6. **Other shapes are untouched** under a safe name: the RFC dashed lower-case v4 `fb197f94-7578-4673-8a57-4642ae120532`,
     `30184`, `1748141-1-0-1` (U5's class), plain 32-hex, and a 36-character 8-4-4-4-12 value.
- **In `guid_twin.rs`:**
  7. 277's v4-passing cf35, `guid_twin_gated_v4(Some("FD29FC2F-A2BF-49A1-9B234B870B531C27"), Some("eu"), Some("FRA"),
     "Tribunal administratif de Paris")`, is `None`. Under `Tribunal administratif` it is `Some((…, Some("FR")))`.
  8. `guid_twin_decide` over 277's TA Paris pair under a safe name has 0 candidates.
- **In `project.rs`:**
  9. Guard 3: a blank-name party carrying a cf35 raw stays "identified blank".
- **Resolver (store or `project_incremental`):**
  10. A refused cf35 mention resolves onto the `(org_name_norm, FR)` identifier-NULL row, never an identified row.
  11. Two notices' TA Paris mentions land on one row.
  12. A refold of a recorded cf35 mention with unchanged raw keeps its organization (`publishes`).
- **In `hex_census.rs`:**
  13. On the existing fixture (`:1606` on), guard mode keeps `CCIRA_LONG` and `CCIRA_SGAR` (committee) and refuses the
      eSender and the TA Paris twin.
  14. `r2_refused` excludes U1's draw.
  15. `buyer_refused_key_tenders` counts a `refused:` Tender.
- **Existing suites:** the 530 recurrence, planner and restore tests and the `nhs_body` canary tests stay green with the
  new fn types.

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
  - **The exception.** Under a name not safe as a name key (blank; fewer than two content words; a generic body name;
    any `Comité consultatif …`; 50 listed names and 2 listed phrases), the value keeps binding its own organization, as
    before.
  - **The buyer key.** In a joint procurement, a co-buyer's plan buyer key now reads by name. No served field changes.
  - **The standing rows** move in unit 4.
- **/docs**, `crates/app/src/v1/docs.rs` `identifier` bullet (:313): append after the 530 sentence. The docs file is
  edited in the working tree by another agent, so rebase onto it.

  > A value published in the 35-character 8-4-4-16 form (Avenue-Web's AWS-Achat eSender, marches-publics.info, mints
  > one per organization section per notice) is not used as an identifier: the mention joins the body's name-keyed
  > organization, except under a name too generic to stand for one body (a bare `CCAS` or `Tribunal administratif`, a
  > review committee named without its seat), where the value keeps its own one-mention organization (issue 575).

  Pin it with a docs test beside `the_docs_say_a_v4_uuid_is_a_platform_key_and_a_twin_drops_it` (:1714 in the tree),
  asserting:
  - "is not used as an identifier";
  - "mints one per organization section per notice";
  - "where the value keeps its own one-mention organization (issue 575)."

**After the deploy, alongside the work:**
- the guard-mode census (§6.5), then R2b (§5);
- one `guid-twin-recurrence` run, whose reverse drift must be unchanged or lower (refused cf35 v4 values leave the
  candidate set).

### 6.9 What stays for U4 (dissolve; blocked by R2b PASS, 579, and 551's W-on port-diff rule)

- **530's mechanics:**
  - an id-set selector, windows by target key, the 543 stop checkpoint, a `Box::pin` arm in `STOPPABLE_KINDS`;
  - `dry_run` true by default, a restore manifest, a wet re-plan;
  - the A2 per-mention rehome through `DissolveTarget::Resolver` (578), not 440's lookup.
- **One ruler, in both directions.** The planner re-derives each class notice's final identity with the gate's own
  functions: `gated_identifier` with the name, then `guid_twin_refusals`. It rehomes every mention whose bound org
  disagrees with that identity:
  - an identified singleton the gate refuses goes to its name row;
  - a mention on an identifier-NULL row whose key the compiled guard keeps goes back to its key. That covers mentions
    folded after the deploy under a name added later; 530's 18 rows rebuild where they stand.
  - A row is dissolved whole when every mention moves.
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
  - a fresh guard-mode census: 0 class rows identified where the gate refuses (the residue is the guard's kept rows);
  - `buyer_refused_key_tenders` 0;
  - the 530 recurrence quiet.
- **U5** (the Avenue-Web tenderer keys `<procedure>-1-<lot>-<n>`) is its own issue and is unchanged.

## Status line for the issue

> U2 DECIDED 2026-10-24 (`575-triage/u2/decision.md`):
> - refuse cf35 in `gated_identifier`, guard (i);
> - a compiled `cf35.rs` guard: blank / content_words / generic / `comiteconsultatif` stem / 50 names / 2 prefixes;
>   kept key = status-quo split;
> - no keep-list (16 multi-Tender explained);
> - buyer churn written down (served 0);
> - R2 random 60 FAILED 8 W (mediator) → guard amended → fresh 60 at 575002 plus mediator enumeration plus name screen
>   required before U4's wet run.
>
> NEXT: U3 build (signature change, 20 call sites, census guard mode, R0, fixture, CHANGELOG, docs).
