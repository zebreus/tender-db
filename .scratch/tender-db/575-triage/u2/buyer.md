# 575 U2: the buyer stratum and the multi-Tender values (buyer-role and multi-tender analyst)

Inputs: census job 2770 (`575-triage/hex_census_2770.json`, body: `samples.buyer` 2,000 rows, `samples.multi_tender` 16,
`notice_mentions`, `targets`). Code at HEAD `af959ff`. Eleven bounded public GETs on `tenders.zebreus.click`: notice content
1382, 24189828, 24342470 and 24438463, and Tenders 471545, 507339, 695422, 347577, 8562829, 262541 and 1093234.
The responses are cached in this session's scratchpad. Nothing ran on the box: no ssh, no `/v1/sql`, no build.

## Verdict (short)

- **What the buyer cf35 mentions are.** They are the co-buyers of a joint procurement (*groupement de commandes*).
  They sit beside a lead buyer that carries a numeric account number. Some of them are the lead buyer printed a
  second time. None of them is the notice's only buyer.
- **Effect of refusing the key on a buyer mention.** The mention moves from its per-notice singleton row to the
  name row. For 1,911 of the 2,000 sample rows (95.5%) that row is the same body (R). For 62 (3.1%) it is a
  generic-name hub (W). 15 more (0.75%) carry homonym risk, and 12 are N.
- **The 369 buyer_key churn is not served.** `buyer_key` is a column in the plan table and is not an API field.
  It reaches a served `procedure_key` only through a refused key. Avenue-Web's BT-04 values are random v4 UUIDs, so
  none of these keys is refused, and the expected served churn is 0 Tenders. The change also goes in the right
  direction: today each notice's buyer set is unique because of the fresh keys, and after the refusal the set
  repeats across a procedure's notices, which is what 369 meant.
- **The 16 multi-Tender values need no keep-list.** 14 are one consultation's ORG keys re-sent in a later notice
  that has a new BT-04. 1 is the eSender key re-used in a third notice. 1 is a cross-publisher weld that the refusal
  undoes. Every target is R.
- **Placement.** The buyer stratum does **not** argue for option (d). For buyer mentions, (d) and `gated_identifier`
  rebind the mention in the same way. They differ only on `published_mentions`, and there the refusal is harmless or
  better. What the buyer stratum does argue for is a **name guard**: guard (i), or a compiled 575 unsafe-name list.
  R2's random 60 leaves out the buyer stratum (`HEX_CENSUS_R2_STRATA`, `crates/store/src/hex_census.rs:46`), so A5's
  0-W bar never sees the 3.1% W found here.

## 1. What the buyer stratum is

### Counts from the report

| measure | value | source |
|---|---|---|
| cf35 mentions with a buyer role | 10,989 | `notice_mentions.cf35_buyer_mentions` |
| role set | `Procedure-Buyer` alone, 10,989; never combined with another role | `notice_mentions.cf35_role_sets` |
| class rows in the buyer stratum | 10,989 rows, 10,989 mentions; one_mention 10,989; multi_notice 0; multi_tender 0; stable 0 | `class.by_stratum.buyer` |
| non-cf35 `Procedure-Buyer` mentions in the 71,615 cf35 notices | `digits` 71,614 + `other` 1 = 71,615 | `notice_mentions.by_shape_role` |
| scheme of the buyer cf35 values | `eu`, 2,000 of 2,000 in the sample | `samples.buyer[].scheme` |
| blank-name cf35 mentions | 0 | `notice_mentions.cf35_blank_name_mentions` |

So there is about one numeric (`digits`) buyer per cf35 notice: 71,615 numeric-or-other buyer mentions in 71,615
notices. The 10,989 cf35 buyer mentions come on top of that lead buyer.

### How the sample was drawn

`samples.buyer` is **not random**. It is the first 2,000 class rows by org id that have the buyer flag
(`hex_census.rs:1364`: `walk.rows.iter().filter(|r| r.buyer).take(GUID_TWIN_LIST_CAP)`). The draw leans towards
early rows: first_year 2024 has 1,516 rows, 2025 has 435 and 2026 has 49.

The rows span 646 notices and 531 Tenders. Sections run from ORG-0003 to ORG-0041; **no row is in ORG-0001 or
ORG-0002**. 462 of the 646 notices hold 2 or more cf35 buyers, and the largest holds 38. 64 rows are within-notice
twins.

### The four notices read, and what they show

Notices read via `/v1/notices/{id}/content`:

| notice | ORG-0001 | ORG-0002 (lead buyer) | cf35 co-buyers |
|---|---|---|---|
| 24438463 (BT-04 `71db71ee-1e0a-44fa-…`) | Avenue-Web, cf35, SProvider | **Eurométropole de Strasbourg `71821`** | ORG-0004 **Eurométropole de Strasbourg** (org 22893509, duplicate of the lead); CCAS de Strasbourg (22893510); Ville de Strasbourg (22893519); 35 more communes, SDIS67/68, Collectivité Européenne d'Alsace … |
| 24189828 (BT-04 `f121393a-2dca-449b-…`) | Avenue-Web, cf35 | **Toulouse Métropole `57038`** | ORG-0003 **TOULOUSE METROPOLE** (22712213, duplicate of the lead); ORG-0004 Commune de TOULOUSE (22712214) |
| 24342470 (BT-04 `94d8e6ca-c87b-4f55-…`) | Avenue-Web, cf35 | **Mairie de Besançon `65130`** | ORG-0005 **Ville de Besançon** (22818047) and ORG-0010 **BESANCON** (22818052), the lead printed twice more; ORG-0008 **CCAS** (22818050); 29 bare commune names (Gennes 22818063, Montfaucon 22818066, Saône, Venise …) |
| 1382 (BT-04 `b70c1fbb-5954-4851-…`) | Avenue-Web, cf35 | **COLMAR AGGLOMERATION `82642`** | ORG-0005 Mairie de Colmar (org 3544, a distinct body); plus an AddInfo "Colmar Agglomération (coordonnateur)" in cf35 |

The pattern in all four notices:

- ORG-0001 is the eSender, with a cf35 key.
- ORG-0002 is the lead buyer, with the platform's numeric account number.
- Then come the group's members, each with a fresh cf35 key.

The coordinator is listed again among the members in 3 of the 4 notices. The sample's names show the same thing:

- Coordinator self-labels: "… (coordonnateur) - CCAS de Lyon" ×4, "… coordonnateur du groupement de commandes", and
  "CHU de Bordeaux coordonnateur".
- Coordinator pairs that recur: "toulouse metropole" 49 rows next to "commune de toulouse" 50, and "eurométropole de
  strasbourg" 38 next to "ville de strasbourg" 40.

**Answer to question 1.** These are co-buyers beside the numeric `Procedure-Buyer`. A share of them are the lead
buyer printed again, never the lead buyer's own section. The lead keeps its numeric row under either option.

Where a co-buyer is the lead printed again, refusing its key moves it from a per-notice singleton (for example
22712213) to the name row (17592272 `toulouse metropole`). That is the same body (R). The Tender still lists the
body twice (the numeric row and the name row), but that self-duplicate exists today too (586's "buyer self-dups"),
and the refusal makes it stable instead of a new row per notice.

### Generic and distinctive names, with the fallback screen (2,000 rows)

The fallback target was found for 1,672 rows and missing for 328. A missing target means a NULL row is minted
once, and that row is R unless the name is generic.

Results from the A6 city screen, with `targets.city_flagged` covering all 436 flagged targets:

- 1,587 rows map to unflagged targets.
- 186 differ only by "cedex" or spelling. For example, Eurométropole de Strasbourg 6174352 has city_absent 200
  because "Strasbourg Cedex" and "Strasbourg" are written differently. These are R.
- 113 map to targets whose planned cities fold to 2 or more.
- 113 have city_absent with a single folded city.

| class | rows | names (sample rows) → target id (cf35 mentions, distinct planned cities across all strata) |
|---|---|---|
| **W, measured** (a generic name pooling several cities, or a city mismatch) | **44** | `ccas` 17 → **1206088** (121 cf35, 34 cities: Aimargues … Versailles; row "CCAS", 852 mentions); "Centre communal d'action sociale" 16 → **1174257** (93, 17 cities; city_absent 91; 2,565 mentions); `caisse des ecoles` 3 → 1201208 (34, 6 cities); `communauté urbaine` 4 → 2102005 (Caen + Le Havre); `ehpad` 2 → 1338203 (Ajain + Boussac; the two are twins in notice 24201102, orgs 22718629/22718630); "CCAS (ccas)" 1 → 1896679 (Arles + Noyon); CIAS 1 → 1244397 (planned Alençon, target Saint-Maixent-l'École), org 22802867 |
| **W, generic or junk name with no place** (single city so far) | **18** | "Profil d'acheteur :" 7 → 20566048 (orgs 22740545, 22745104, …); "(les) coordonnées des membres du groupement sont précisées dans le règlement…" 2; "1 place de la liberté" 1; "la société publique locale" 1 (22875396, missing); "etablissement public foncier local" 1; "syndicat d'adduction d'eau potable" 2 (9364, 13652, missing); "syndicat mixte de traitement des ordures ménagères et autres déchets" 2; "centre communale d'action sociale" 1; "groupement de commandes ville et ccas" 1 |
| **Homonym risk** (N, possibly W) | 15 | "lycée paul langevin" 7 → 9617375 (one city in the cf35 population, but several lycées carry this name nationally); Montfaucon 2 → 15935913; Gennes 1 → 19886083; "mairie de saint andré", "mairie de saint marcel", "commune de saint-just", "commune le vernet", "mairie de leval" (1 each) |
| **N** (short acronyms) | 12 | CHAC, EFM, SIGH, IRP, VRA, SLCI, SIVAC, SIABA, LEAV85, CD34 (→ missing; Gignac + Montpellier, both in Hérault), SMMAG |
| **R** | 1,911 | Commune/Ville/Mairie/CCAS **de X**, CH de X, metropoles, housing companies (AMLI, LIVIE, SEQENS …), and bare unique commune names (Deluz, Franois, Pugey, the Sarreguemines communes …). Multi-city targets that are one body: Bordeaux Métropole 6336369 (Bordeaux + Pessac), Collectivité Européenne d'Alsace 16490606 (Colmar + Strasbourg), Atlantique Habitations 1249377 (misspellings of Saint-Herblain), in'li 11816350, Batigère Habitats Solidaires 20732731 … |

- **Bare department labels.** None among the buyers. Those are AddInfo (587's class), for example
  `direction de la commande publique` 1166638 and `service de la commande publique` 3443. `mairie` 1163705 (64 cf35
  mentions, 36 cities) does not appear in the first 2,000 buyer rows.
- **Overall W rate.** 62 of 2,000 (3.1%). Extrapolated to 10,989 that is about 340 mentions, but the sample is the
  first 2,000 by id, not random.
- **What a 530-style guard would catch.** `guid_twin_unsafe_name` applies three tests: `content_words < 2`,
  `generic_body_name` (`project.rs:11266-11273` lists "Centre communal d'action sociale", "Centre intercommunal
  d'action sociale", "Mairie", "Commune", "Ville", "Lycée" …), and the list.
  - It **spares 37 of the 62** W rows: CCAS 17, the long CCAS form 16, CCAS (ccas) 1, CIAS 1 and EHPAD 2. It also
    spares Montfaucon and Gennes, plus about 90 one-word R rows, which would stay split (N, as today).
  - It **misses 25**: caisse des écoles, communauté urbaine, "profil d'acheteur", "coordonnées des membres…",
    "1 place de la liberté", "société publique locale", "établissement public foncier local", the two syndicats, the
    misspelled "centre communale", and "groupement de commandes ville et ccas". Those need a compiled 575 list, exact
    names or a prefix for the "coordonnées des membres" boilerplate.

## 2. What a refused key on a buyer mention changes (code, `af959ff`)

### The two paths

- `gated_identifier(raw, scheme, country)` (`crates/ingest/src/project.rs:5490-5492`) is called once per mention in
  `mentions_and_dropped_gated` (`:6269`). The 530-T gate block comes after it (`if gate { … }`, `:6279-6281`).
- **Binding path** (`gate=true`, `mentions_and_dropped` `:6084-6091`). The mention gets `identifier = None` and
  resolves by `(org_name_norm(name), register_jurisdiction(FR))`. This is the same under (d) and under
  `gated_identifier`.
- **Published path** (`gate=false`, `published_mentions` `:6102-6104`). It reaches `buyer_side_mentions` (`:10884`,
  reading `published_mentions` at `:10919`), then `GuardSide`, then:
  - **369 `buyer_key`**: `buyer_key_of` (`:10979-10985`), called from `Ident::read` (`:6937`), keys each buyer by
    `buyer_identifier_key` (`:10964`, `country:kind:value`) or else by `buyer_name_key` (`:10972`,
    `n2:country:match_norm(name)`);
  - **481 `buyer_tokens`**: `buyer_tokens_of` (`:11004`), through `buyer_guard_tokens` (`:11040-11061`) and
    `buyer_guard_evidence` (`:11105`).
  Under (d) none of this changes. Under `gated_identifier` all of it does.

### The buyer_key churn, concretely

For notice 24189828, approximately:

- before: `FR:national:57038|FR:national:9BAF3D1E9B7A2CEBEF1702E36A759225|FR:national:9BAF3D3C9C1C6055A4E419833C3ACE93`
- after: `FR:national:57038|n2:FR:commune de toulouse|n2:FR:toulouse metropole`

The lead's numeric key stays in every set.

**How many notices.** Every notice with at least one cf35 `Procedure-Buyer` mention. The report has no count per
notice. The sample has 3.10 rows per notice (3.05 in 2024, 3.30 in 2025, 2.88 in 2026), which gives **about 3,550
notices and about 2,900 Tenders**. The Tender figure is 10,989 × 531/2,000.

**Today these sets are unique per notice.** In the sample, 100 of 531 Tenders hold 2 or more cf35-buyer notices (87
hold 2, 11 hold 3, 2 hold 4). Each of those notices carries a different buyer set today. 369's premise is that "a
joint procurement repeats ONE set across its notices" (`project.rs:10797-10806`; `canonical.rs:19695-19697`). After
the refusal those notices share one set whenever the co-buyer names repeat. So the change only **lowers** the
`COUNT(DISTINCT buyer_key)` per key. It can lift a refusal and can never add one.

**Is it served? No.** `buyer_key` has no hit in `crates/app`, and the Tender JSON (`crates/app/src/v1/json.rs:109-130`)
has no such field. It lives in `plan_notice.buyer_key` (`crates/store/src/canonical.rs:18621`) and is read only:

- by the 369 gate (`:19700`, `WHERE key_shaped = 1 … HAVING COUNT(DISTINCT buyer_key) >= 3`);
- by the 386 gate (`:19737`, `source = 'fts'`);
- by `refused_key_labels` (`:20177`, `refused:{key}:{buyer}` at `:20206`);
- by 482's `uuid_hub_label` (`:2486`, the first member's `buyer_key`, for refused hubs only);
- by the incremental `refused_groups` (`project.rs:3619-3624`). That path is skipped for v4 keys, which take
  `hub_keys` because `is_uuid_hub_class` (`:11984-11986`) is true for them.

Otherwise `group_key = procedure_key` (`canonical.rs:19801`), which is the served `procedure_key`.

**Why Avenue-Web's keys are never refused.** Their BT-04 values are random v4 UUIDs. That holds for the 4 notices
above and the fixture (`d97ade65-146f-4b16-…`). It also holds for the Tenders behind the multi-Tender values
(`6eec7d5c-a3c4-4b8f-…`, `987877a5-7d85-4e2b-…`, `4b95a64e-80ca-4098-…`, `7ecb3eaa-d465-4992-…`,
`670431bc-b4f5-444a-…`). So:

- `is_placeholder_key` is false (`project.rs:12062-12067`: 6 or fewer free nibbles, or a run of 8 or more; a random
  key has about a 1.4e-9 chance), so `key_shaped = 0` and 369 does not apply;
- the source is `ted`, so 386 does not apply;
- 482 clusters by `buyer_guard` tokens, not by `buyer_key`, and needs `UUID_HUB_CLUSTERS = 3` (`canonical.rs:2457`)
  buyer-disjoint clusters under one key. One procedure's notices share the lead's tokens.

**Expected served churn: 0 Tenders.** U3's dry plan should assert it with one count: notices that have a cf35
`Procedure-Buyer` and a `procedure_key` in `plan_refused_key`. It should be 0. If it is not, the labels that move
should be written down.

### The other consumers

- **481 tokens.**
  - The per-notice `FR:national:<hex>` token disappears. It never recurs, so no overlap is lost.
  - The `r:` raw token (`project.rs:11115`) reads `raw_identifier` and does not change, and the name tokens do not
    change either.
  - The resolved-org token `o:<id>` (`add_buyer_org_tokens`, `:11510`) moves from the singleton row to the name row.
    That can only widen overlap, which is 530's F6 argument (`guid_twin.rs:40-47`).
- **511 guard 3** (`project.rs:5189-5196`). It applies only to blank-name parties, and
  `cf35_blank_name_mentions = 0`, so it changes nothing.
- **513 nhs-body canary** (`crates/app/src/supervisor.rs:14952`, `identity: ingest::project::gated_identifier`). It
  covers GB; the cf35 class is FR only (`class.by_country` = {FR}), so it changes nothing.
- **530.** `guid_twin.rs:287` and `:344` call `gated_identifier`. Under the refusal in `gated_identifier`, the
  cf35-v4 values (`class.v4_rows` 3,513) drop out of the 530-T candidates and out of `guid_twin_gated_v4`
  automatically. Under (d), `guid_twin_gated_v4` has to learn the shape, or the recurrence arm reports reverse drift
  (the issue's Decision (d)).

## 3. The 16 multi-Tender values

Facts from the report: `class.values_multi_tender` 16 and `values_multi_row` 0, so each value sits on **one** row.
Each row has 2–3 mentions in 2–3 notices. 46 rows span several notices, so 30 of them stay inside one Tender. Tender
pairs were read via `/v1/tenders/{id}`.

| org | name (first role) | Tenders | why it spans Tenders | target (screen) |
|---|---|---|---|---|
| 23126077 | TA de Toulon (ReviewOrg) | 507339 / 695422 | Same consultation ("Concours restreint … médiathèque … Saint Jean du Var"). 507339 is the CN/CAN chain (procedure key `6eec7d5c…`). 695422 is a **subtype 38 modification notice with a new BT-04** (`987877a5…`), so our grouping makes it a second Tender, and Avenue-Web re-sent the stored ORG keys | 1163191 TA Toulon, R |
| 23126078 | CCIRAL (Mediator) | 507339 / 695422 | the same notice pair | 1179658 `cciral`, not city-flagged; N (an acronym; check it in the CCIRA screen) |
| 23126075 | Avenue-Web Systèmes (SProvider) | 471545 / 507339 / 695422 | the same pair, **plus** 471545 (a different procurement: Toulon street lighting, subtype 38). The eSender key was re-used | 8301019 avenue-web systèmes, R |
| 23203079 | TA de Toulon (ReviewOrg) | 347577 / 8562829 | the same mechanism: 347577 (CN→CAN, key `4b95a64e…`, Collège Paul Cézanne, Brignoles) and 8562829 (subtype 38 modification, new key `7ecb3eaa…`, published 2026-07-09) | 1163191, R |
| 22838246, 22866414, 22972335, 23075937, 23192304, 23300013, 23324212, 23533887, 23607703, 23646816, 30937919 | Greffe TA Lyon; TA Montpellier ×3; Melun; Strasbourg; Rennes; Lille; Caen; Marseille; Poitiers (ReviewOrg/ReviewInfo) | pairs; one side is often a high id (8550247, 8560868, 8568845), a newly minted Tender for the later notice | the same re-emission. Each pair is one consultation's later notice landing in its own Tender | 1163242, 1163864, 1165479, 1163701, 1163090, 1163457, 1166355, 1162955, 1165316: all top targets with the city in the name, R |
| 23586902 | row name "TRIBUNAL JUDICIAIRE"; its cf35 mention is Avenue-Web, SProvider, ORG-0001 | 262541 / 1093234 | **A cross-publisher weld.** The cf35 key of the Avenue-Web SProvider in COL's notice (1093234) has the same 32 hex characters after dash-stripping as an `other`-shaped raw on a "TRIBUNAL JUDICIAIRE" mention in a CPAM de la Gironde notice (262541). That is the census's 1 `mixed_shape_rows` | the cf35 mention goes to 8301019. The refusal **separates** the two bodies |

**Decision input.** No keep-list is needed.

- In 15 of the 16 the recurrence joins the same body. Every one of those has a name target that is R (one city in
  the name, or the eSender), so after the refusal both mentions land on the same name row and the co-reference
  survives.
- The 16th is a weld that the refusal fixes.
- Keeping any of them would preserve nothing the name target does not already give.

530's KEEP rationale (a recurring value whose name fallback is unsafe) does not arise here. The mechanism behind
the recurrence is the BT-04 split of a modification notice; it is a 481/369 grouping matter, outside 575.

## 4. Option (d) or refusal in `gated_identifier`, from the buyer stratum's side

- **The issue's trigger.** "(d) … Choose it only if U1 finds cf35 on buyer sections." U1 found 10,989. But the
  reason (d) gives is "`published_mentions` would keep cf35 on buyer_key", and for this stratum that is the worse
  outcome:
  - it keeps about 3,550 notices' buyer sets unique per notice, against 369's design;
  - it protects no served field: `buyer_key` reaches `procedure_key` only through refused keys, and none of these
    keys is refused (see section 2);
  - it leaves 481 tokens carrying a dead per-notice token.
  The lead buyer's numeric key is in every set, so no notice loses its buyer identity.
- **Why 530 kept the buyer side on the published identity, and why that does not carry over.** 530 did it so that
  "the deploy moves no Tender key" (`guid_twin.rs:40-47`; test
  `the_guid_twin_gate_leaves_the_buyer_key_and_tokens_alone`, `:2289`). There, a refused GUID could be a notice's
  whole buyer key (the test's `DE:national:037C…`). Here the refused values are always co-buyers next to a numeric
  lead, and the keys are unrefused v4 UUIDs. The same "no Tender key moves" property holds without (d), and the
  dry-plan count above can confirm it.
- **A role carve-out?** (d) could spare `Procedure-Buyer` mentions, because the gate block has `parsed`, while
  `gated_identifier` has no role parameter (`:5490`). It would leave all 10,989 co-buyer rows as per-notice
  singletons, about 2,900 Tenders whose co-buyers cannot be found by organization. That is too high a price for a 3%
  W that a name list removes. Rejected.
- **What the buyer stratum actually decides.** The name guard. Unguarded (ii) carries about 3.1% W in this stratum,
  and R2's random strata leave buyers out (`hex_census.rs:46`). So:
  - choose guard (i) with 530's `guid_twin_unsafe_name` logic **plus** the 11 names listed in section 1, as a
    compiled 575 list. That brings the sample's W from 62 to 0, at the cost of about 90 one-word R rows staying
    split;
  - or add a buyer stratum of 15 to R2's random draw, or adopt this 2,000-row read as the buyer R2, and record the 62
    W as a cost under (ii).
  The generic co-buyer hubs (1206088 `CCAS`, 1174257 `Centre communal d'action sociale`, 1201208, 1338203, 2102005,
  1244397, and `mairie` 1163705 in other strata) are 587's sibling class: bare body kinds, where 587 has bare
  department labels. Either an unsafe-name binding here or 587's alias-to-own-buyer would cover them.
- **Recommendation from this stratum.** Refuse in `gated_identifier`, which is the issue's direction, with guard
  (i) or the compiled 575 list. Add the dry-plan assertion "0 cf35-buyer notices under a refused key". The buyer
  stratum gives no reason for (d).
