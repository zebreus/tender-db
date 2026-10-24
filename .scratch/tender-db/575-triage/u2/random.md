# 575 U2: random-60 fallback judge (hex-census 2770, `samples.r2_random`, salt 575001)

This read was read-only. Source: `575-triage/hex_census_2770.json` (job 2770, max_id 31,978,722), `samples.r2_random` (60 rows).
For each row's fallback target, the target fields come from `targets.top`, `targets.city_flagged` and `targets.ccira`. Those
lists are complete: 50, 436 and 52 entries, under the list cap of 2,000. A target that is absent from all three lists is
therefore not city-flagged: it has at most one planned city, at most one target city, and no city clash.

Standard: 530 `decision.md` §A5 R2 and §A6.
- **R**: the target is the same real body as the mention.
- **N**: cannot tell.
- **W**: the target's name denotes more than one body, or the target holds or would pool another body's mentions.
- The bar is 0 W and N ≤ 3 in the 60.

The "evidence-only" column scores the same rows on cities alone, without the name rule. The "guard (i) list" column says
whether the compiled 575 list in `u2/targets.md` would keep the row's cf35 key, meaning no refusal and no fold. That list
is the prefix `comite consultatif interregional` on `guid_twin_city_fold` plus 51 exact names. No GET, box read or
`/v1/sql` was used.

## Result: the bar FAILS, all of it in the mediator stratum

| stratum | rows | R | N | W | evidence-only R / N / W |
|---|---:|---:|---:|---:|---|
| sprovider | 15 | 15 | 0 | 0 | 15 / 0 / 0 |
| review | 15 | 15 | 0 | 0 | 15 / 0 / 0 |
| mediator | 15 | 7 | 0 | **8** | 9 / 4 / 2 |
| add_info | 15 | 15 | 0 | 0 | 15 / 0 / 0 |
| buyer | **0** | — | — | — | — |
| **all** | 60 | 52 | 0 | **8** | 54 / 4 / 2 |

- **Unguarded, option (ii):** 52 R, 0 N, 8 W. **FAIL.** Even the lenient city-only reading fails, with 2 W and 4 N > 3.
  - The 2 W that hold even on evidence are rows 33 and 40. Both fall on key 1163657, "comité consultatif interdépartemental
    de règlement amiable des litiges": its cf35 mentions plan Marseille and Paris, so two committees share one target.
  - The other 6 W are W by name. Each is a placeless committee name, which denotes every CCIRA:
    - 31 (1167628) and 38 (2585842). Each shows one city today, Nantes and Lyon respectively.
    - 34 (1196898), the fixture's long form. Its target has no city.
    - 45 (3227980, "inter-régional … (CCIRAL)"). Its target has no city.
    - 32 (1179658, bare `CCIRAL`).
    - 37 (12932404, "CCIRAL - comité consultatif interrégional …").
- **Guard (i) with the compiled list from `u2/targets.md`:** it keeps 6 rows (31, 33, 34, 36, 38, 40; row 36 is an R that
  gets spared). The other 54 are refused and fold: 51 R, 0 N, **3 W** (32, 37, 45). **FAIL.** Read on evidence only, the
  same 54 give 51 R / 3 N / 0 W, which passes only at the N limit.
- **The three W rows that escape the list show gaps in it, and each gap is proven from the report itself:**
  - `cciral` is missing. The list has `ccira` only.
  - The hyphenated "inter-régional" folds to `inter regional`. It misses the prefix, and 3227980 is absent from the
    census's own ccira list.
  - A leading acronym before the phrase defeats a prefix rule: `cciral - comité consultatif interrégional …`, 12932404, is
    also absent from the ccira list.
  - The fix: detect the committee anywhere in a dash-joined fold, and add exact `cciral`. Row 35, "LE COMITE CONSULTATIF
    INTERREGIONAL … DE MARSEILLE", is seat-named R. Today it is refused (no prefix match) and folds correctly. An
    "anywhere" rule would keep it, which costs a little but does no harm.
- **On any W, 530 §A5 applies:** add the names, re-run the plan, and read a fresh random 60 at salt **575002**, excluding
  these 60. That read must come back 0 W and N ≤ 3. The list fixes above are tuned on this sample, so they cannot be
  scored as a pass on it.
- **The mediator stratum is under-screened by the target lists.** The lists hold about 3.3k W-by-name mediator-type
  mentions: ccira W 2,361, plus 1163657 (278), 3227980 (490) and 1171446 (134). That is roughly 20% of the 16,140 mediator
  mentions. The sample finds 8/15. At p = 0.17, P(≥ 8 of 15) ≈ 0.0014.
  - So placeless committee keys that sit below the top 50 and print one city each carry much more of the stratum than the
    lists show. 1179658 and 12932404 are in no list.
  - Recommendation: the dry plan should **enumerate every key of the mediator stratum**, about 16k rows. Equivalently,
    enumerate every key whose fold contains `comite consultatif`, `ccira` or `cciral`, instead of sampling it.

## Strata against U2's ask

- **Composition.** The draw is `HEX_CENSUS_R2_STRATA` = sprovider 15, review 15, mediator 15, add_info 15. That is exactly
  the four roles U2 named ("SProvider, review, mediator, AddInfo"). Each stratum is in salted order
  (`guid_twin_sample_key(org_id, 575001)`, 530's formula).
  - Every row has 1 mention, 1 notice and 1 Tender, scheme `eu`, country FR, and a cf35 raw that equals the stored
    identifier once the dashes are stripped. Every target exists, and every target_country is FR.
  - Sections: sprovider is always ORG-0001, add_info always ORG-0003, review ORG-0003/4/5, mediator ORG-0004/5.
  - 9 rows are twins (7 review, 1 mediator, 1 add_info, 0 sprovider). The 60 rows go to 39 distinct targets.
- **The buyer stratum is missing.** U2 was written when buyers were thought to be free of the shape (0 of 6 sampled).
  - U1 found 10,989 buyer-role cf35 rows, 4.9% of the class. This sample gives **no bound** on them.
  - `samples.buyer` is the first 2,000 buyer rows **in id order**, not a random draw. It skews to the oldest ids, so a
    buyer bound needs its own salted stratum.
- **Not proportional.** The class rows are review 109,922 (49.0%), sprovider 71,581 (31.9%), mediator 16,132 (7.2%),
  add_info 15,924 (7.1%) and buyer 10,989 (4.9%). They sum exactly to the 224,548 class rows.
  - Proportional allocation of 60 would be about review 29, sprovider 19, mediator 4, add_info 4, buyer 3.
  - The equal 15s therefore oversample mediator and add_info about 3.5×, undersample review 2×, and omit buyer.
  - Equal allocation gives each stratum its own bound, but 530's "0 in 60 → about 5%" does not hold here. Over the four
    sampled strata the effective n is about 38.6. So even 0 W in all 60 would bound the class-weighted rate at about 7.8%,
    not 5%.
- **Error bound per stratum** (rule of three: 0 in 15 → 3/15 = 20%; exact one-sided 95% is 18.1%):
  - **sprovider:** 0/15, so ≤ 20% from the sample alone. In practice the error is 0 by enumeration: 71,601 of the 71,602
    sprovider cf35 mentions fold to the single target 8301019, the eSender's own NULL row, judged R in the top 30.
  - **review:** 0/15, so ≤ 20%, about 22.0k of 109,922 rows. The top-30 targets corroborate this: they are seat-named TAs,
    all R.
  - **add_info:** 0/15, so ≤ 20%, about 3.2k of 15,924 rows. This is weak coverage of the class where generic labels live
    (CCAS, mairie, service de la commande publique, hôtel de ville; issue 587). The compiled list catches some of them,
    but this sample hit none.
  - **mediator:** 8/15 W by the 530 rule, Clopper-Pearson 95% 26.6%–78.7%. The point estimate is about 8.6k of 16,132
    rows, 3.8% of the class. On evidence only it is 2/15 W (1.7%–40.5%) plus 4 N.
  - **buyer:** not sampled, so unbounded here.

## Twin groups (30) and the mixed-shape row

- **The twins confirm per-section minting.** In all 30 groups, one body printed 2 or 3 times in one notice carries 2 or 3
  different values.
  - Within each group the values rise strictly in section and org-id order. The shared hex prefix is 6 characters in 24
    groups, 5 in 5 and 4 in 1. The org ids are adjacent, with a gap of 1 to 3.
  - Names: 25 TA de <seat>, 4 TJ de <seat> (Mamoudzou, Rennes, Marseille, Bordeaux) and 1 Département de la Haute-Savoie.
    All are single bodies. The sizes are 27 pairs and 3 triples (TA Grenoble 919, TA Versailles 993 and 1045).
  - Caveat: these are the **first 30 met in notice order** (notices 131–1165), not a random draw.
  - Census-wide: 35,580 groups, 73,613 rows, `sprovider_rows` 0. The eSender is printed once per notice and is never twinned.
- **Nothing contradicts "category: per-section key".**
  - `values_multi_row` is 0, so no value sits on two org rows.
  - All 46 multi-notice rows fall in the mixed-SDK profile buckets: 1.10+1.12: 2, +1.13: 3, +1.14: 9; 1.12+1.13: 21,
    +1.14: 11. Every single-profile bucket shows 0 multi-notice rows.
  - So a value recurs only when a later notice of the same procedure, published under a newer SDK, re-publishes the
    original ORG sections. The 16 multi-Tender rows are that copy landing on a second Tender. For example, 23126075
    (Avenue-Web), 23126077 (TA Toulon) and 23126078 (CCIRAL) share the Tender pair [507339, 695422]: one ORG set was copied
    whole.
  - The key names a section of one procedure's notice lineage. It never names an organization across procedures.
- **Mixed shape (1 row): 23586902, stored A4413527D09C3E29D452F3633423E371.**
  - It has 2 mentions in 2 notices on 2 Tenders (262541, 1093234). One mention is the Avenue-Web SProvider block (cf35,
    ORG-0001 of notice 25659608; its target is 8301019). The other is the class's only non-cf35 mention
    (`other_shape_mentions` 1). The row is served as "TRIBUNAL JUDICIAIRE".
  - One value therefore labels two different bodies. That *strengthens* the case that the value is not an organization
    key, and it is one of the 16 multi-Tender rows.
  - Gap: the gate refuses by raw shape, so the non-cf35 mention keeps the value. After U4 the row stays as a one-mention
    "TRIBUNAL JUDICIAIRE" carrying a hex identifier. That is 1 row, harmless, and U4's per-mention rehome should expect it.
    The bare "tribunal judiciaire" is also entry #3 of the compiled list.

## The 60 rows

Row numbers are 1-based. Target mention counts (`m`) are shown where the target appears in a report list.

| # | org_id | name | stratum | target_id | verdict | evidence-only | guard (i) list | reason |
|---:|---:|---|---|---:|:-:|:-:|---|---|
| 1 | 23102802 | Avenue-Web Systèmes | sprovider | 8301019 (1 m) | **R** | R | - | the eSender itself (ORG-0001, Procedure-SProvider); target 8301019 is Avenue-Web's own NULL row (1 mention); all 71,601 cf35 mentions under the key plan one city, Seyssinet-Pariset (AWS's seat) |
| 2 | 23211831 | Avenue-Web Systèmes | sprovider | 8301019 (1 m) | **R** | R | - | as row 1 (Avenue-Web Systèmes, ORG-0001, target 8301019) |
| 3 | 30958411 | Avenue-Web Systèmes | sprovider | 8301019 (1 m) | **R** | R | - | as row 1 (Avenue-Web Systèmes, ORG-0001, target 8301019) |
| 4 | 23092950 | Avenue-Web Systèmes | sprovider | 8301019 (1 m) | **R** | R | - | as row 1 (Avenue-Web Systèmes, ORG-0001, target 8301019) |
| 5 | 24492642 | Avenue-Web Systèmes | sprovider | 8301019 (1 m) | **R** | R | - | as row 1 (Avenue-Web Systèmes, ORG-0001, target 8301019) |
| 6 | 23602212 | Avenue-Web Systèmes | sprovider | 8301019 (1 m) | **R** | R | - | as row 1 (Avenue-Web Systèmes, ORG-0001, target 8301019) |
| 7 | 30907215 | Avenue-Web Systèmes | sprovider | 8301019 (1 m) | **R** | R | - | as row 1 (Avenue-Web Systèmes, ORG-0001, target 8301019) |
| 8 | 23720408 | Avenue-Web Systèmes | sprovider | 8301019 (1 m) | **R** | R | - | as row 1 (Avenue-Web Systèmes, ORG-0001, target 8301019) |
| 9 | 23433211 | Avenue-Web Systèmes | sprovider | 8301019 (1 m) | **R** | R | - | as row 1 (Avenue-Web Systèmes, ORG-0001, target 8301019) |
| 10 | 23538140 | Avenue-Web Systèmes | sprovider | 8301019 (1 m) | **R** | R | - | as row 1 (Avenue-Web Systèmes, ORG-0001, target 8301019) |
| 11 | 22778782 | Avenue-Web Systèmes | sprovider | 8301019 (1 m) | **R** | R | - | as row 1 (Avenue-Web Systèmes, ORG-0001, target 8301019) |
| 12 | 30966703 | Avenue-Web Systèmes | sprovider | 8301019 (1 m) | **R** | R | - | as row 1 (Avenue-Web Systèmes, ORG-0001, target 8301019) |
| 13 | 24500934 | Avenue-Web Systèmes | sprovider | 8301019 (1 m) | **R** | R | - | as row 1 (Avenue-Web Systèmes, ORG-0001, target 8301019) |
| 14 | 23010753 | Avenue-Web Systèmes | sprovider | 8301019 (1 m) | **R** | R | - | as row 1 (Avenue-Web Systèmes, ORG-0001, target 8301019) |
| 15 | 23436334 | Avenue-Web Systèmes | sprovider | 8301019 (1 m) | **R** | R | - | as row 1 (Avenue-Web Systèmes, ORG-0001, target 8301019) |
| 16 | 23589489 | Tribunal Judiciaire d'Albertville | review | 15213555 | **R** | R | - | seat-named court (one TJ d'Albertville); key not city-flagged (one planned city at most, no clash) |
| 17 | 23075815 | Tribunal administratif de Montpellier | review | 1163864 (27,265 m) | **R** | R | - | seat-named TA; planned montpellier / montpelllier (typo); top#5, judged R |
| 18 | 22879449 | Greffe du Tribunal administratif de TOULON | review | 1166302 | **R** | R | - | the registry of TA Toulon, target is that registry's own name; key not city-flagged |
| 19 | 30912986 | Tribunal administratif de Toulouse | review | 1163070 (32,677 m) | **R** | R | - | seat-named TA; planned toulouse only; top#4 |
| 20 | 23150318 | Tribunal administratif de Lille | review | 1163457 (52,385 m) | **R** | R | - | seat-named TA; planned lille / lille cedex |
| 21 | 23197083 | Tribunal administratif de Nîmes | review | 1163138 (14,493 m) | **R** | R | - | seat-named TA; target nimes / nimes cedex, planned nimes |
| 22 | 23410971 | Tribunal administratif de Rennes | review | 1163090 (34,410 m) | **R** | R | - | seat-named TA; planned rennes only |
| 23 | 24502510 | Tribunal administratif de Melun | review | 1165479 (34,275 m) | **R** | R | - | seat-named TA; planned melun only |
| 24 | 23292090 | Tribunal administratif de Lyon | review | 1163241 (46,596 m) | **R** | R | - | seat-named TA; planned lyon / lyon france |
| 25 | 23757948 | Tribunal administratif de Rennes | review | 1163090 (34,410 m) | **R** | R | - | seat-named TA; planned rennes only |
| 26 | 23437910 | Tribunal administratif de Grenoble | review | 1163404 (40,316 m) | **R** | R | - | seat-named TA; city_absent 10,005 is spelling (target 'grenoble cedex' vs grenoble / grenible) |
| 27 | 22812165 | Tribunal administratif d'Orléans | review | 1163223 (26,883 m) | **R** | R | - | seat-named TA; planned orleans only; target row name is the clean spelling (not the 573 mojibake head) |
| 28 | 23584738 | Tribunal administratif de Marseille | review | 1162955 (35,999 m) | **R** | R | - | seat-named TA; planned marseille only |
| 29 | 22784493 | Tribunal administratif de Bordeaux | review | 1163131 (30,364 m) | **R** | R | - | seat-named TA; planned bordeaux / bordeaux cedex |
| 30 | 23390448 | Tribunal administratif d'Orléans | review | 1163223 (26,883 m) | **R** | R | - | seat-named TA; planned orleans only |
| 31 | 23045043 | Comité Consultatif Interrégional pour le règlement amiable des marchés publics | mediator | 1167628 (1,119 m) | **W** | R | kept (prefix) | placeless CCIRA spelling: the name denotes all the committees (530 R2: W). Evidence today is Nantes only (target newest 50 and all 355 planned), so this mention is Nantes'; the target's older 1,069 mentions are unread |
| 32 | 23517621 | CCIRAL | mediator | 1179658 | **W** | N | NOT kept | bare acronym 'CCIRAL' (comité consultatif interrégional de règlement amiable des litiges): names any committee, no seat. Key not city-flagged, so its own mentions show at most one city; still placeless. The 575 prefix does not bind it and the compiled list has `ccira`, not `cciral` |
| 33 | 22976247 | Comité consultatif interdépartemental de règlement amiable des litiges | mediator | 1163657 (1,323 m) | **W** | W | kept (exact #2) | placeless; the key's cf35 mentions plan Marseille AND Paris (multi_city_planned): two committees under one key, the target (1,323 mentions, no city in its newest 50) pools them |
| 34 | 22928785 | Comité consultatif interrégional de réglement amiable des différends ou litiges relatifs aux marchés publics | mediator | 1196898 (82 m) | **W** | N | kept (prefix) | the fixture's long CCIRA form, placeless; target 82 mentions with no city in its newest 50; planned paris only (206) |
| 35 | 23224318 | LE COMITE CONSULTATIF INTERREGIONAL DE REGLEMENT AMIABLE DES DIFFERENDS RELATIFS AUX MARCHES PUBLICS DE MARSEILLE | mediator | 21298621 | **R** | R | refused (no prefix match: 'le …') | names its seat (… DE MARSEILLE); seat-named target; key not city-flagged |
| 36 | 23340599 | Comité consultatif interrégional de règlement amiable des différends ou Litiges relatifs aux marchés publics préfecture de la région Provence - alpes - côte d'azur | mediator | 16701687 (32 m) | **R** | R | kept (prefix; R spared) | names its seat (préfecture de la région PACA = Marseille's committee); planned marseille |
| 37 | 22956020 | CCIRAL - Comité consultatif interrégional de règlement amiable des litiges | mediator | 12932404 | **W** | N | NOT kept | 'CCIRAL - comité consultatif interrégional …', placeless: names any committee. Key not city-flagged. Its fold starts with 'cciral', so the CCIRA prefix misses it (absent from the census's own ccira list) |
| 38 | 23490717 | Comité Consultatif Interrégional de Réglement | mediator | 2585842 (448 m) | **W** | R | kept (prefix) | truncated placeless CCIRA spelling, names any committee (530 R2: W). Evidence today is Lyon only (target newest 50 and all 472 planned) |
| 39 | 23268715 | Greffe du Tribunal Judiciaire de Lille | mediator | 14638458 (881 m) | **R** | R | - | seat-named registry (greffe du TJ de Lille) printed as Lot-Mediator; target is the same registry, planned lille |
| 40 | 23277613 | Comité Consultatif Interdépartemental de Règlement Amiable des Litiges | mediator | 1163657 (1,323 m) | **W** | W | kept (exact #2) | same key as row 33: planned Marseille + Paris under one placeless name |
| 41 | 30957997 | Tribunal administratif de Grenoble | mediator | 1163404 (40,316 m) | **R** | R | - | TA Grenoble printed as Lot-Mediator (twin in its notice); the target is the same TA, role mislabel only |
| 42 | 23029050 | CCIRA de Bordeaux | mediator | 1176664 (5,619 m) | **R** | R | - | seat-named (CCIRA de Bordeaux); target bordeaux / bordeaux cedex / merignac are its address variants |
| 43 | 23252407 | CCIRA de Bordeaux | mediator | 1176664 (5,619 m) | **R** | R | - | as row 42 |
| 44 | 23493336 | CCIRA de Bordeaux | mediator | 1176664 (5,619 m) | **R** | R | - | as row 42 |
| 45 | 23232732 | Comité consultatif Inter-régional de Règlement Amiable des Litiges (CCIRAL) | mediator | 3227980 (88 m) | **W** | N | NOT kept | placeless ('… inter-régional … (CCIRAL)'); target 88 mentions with no city in its newest 50, planned marseille (490). The hyphen folds to 'inter regional', so the CCIRA prefix misses it (absent from ccira list) |
| 46 | 23107001 | CH PIERRE OUDOT | add_info | 15049351 | **R** | R | - | proper name (Centre hospitalier Pierre Oudot, one hospital); key not city-flagged |
| 47 | 23456108 | SIMACUR | add_info | 4382715 | **R** | R | - | distinctive syndicate acronym; key not city-flagged |
| 48 | 22857366 | Département de Loire-Atlantique | add_info | 2411587 | **R** | R | - | one département; key not city-flagged |
| 49 | 23454532 | CA du Pays de Dreux | add_info | 22650131 | **R** | R | - | one agglomeration community (Pays de Dreux); key not city-flagged |
| 50 | 31534291 | NANTES MÉTROPOLE HABITAT Office Public de l'Habitat de la Métropole Nantaise | add_info | 9180838 | **R** | R | - | one OPH (Nantes Métropole Habitat), full legal name |
| 51 | 22744492 | Metz Métropole | add_info | 10845771 | **R** | R | - | one metropolis |
| 52 | 23170061 | Eaux de Vienne - SIVEER | add_info | 8759339 (54 m) | **R** | R | - | one syndicate; target and planned poitiers / poitiers cedex |
| 53 | 23132521 | Département de la Manche | add_info | 1190509 | **R** | R | - | one département; key not city-flagged |
| 54 | 22832998 | Tours métropole Val de Loire | add_info | 9685765 (445 m) | **R** | R | - | one metropolis; city_absent 43 is 'tours cedex 3' vs 'tours' |
| 55 | 23036283 | Atlantique Habitations | add_info | 1249377 (494 m) | **R** | R | - | one ESH; planned cities are misspellings of Saint-Herblain |
| 56 | 31626938 | CA du Pays de Dreux | add_info | 22650131 | **R** | R | - | as row 49 |
| 57 | 23159756 | Département de la Haute-Savoie | add_info | 1177949 (3,277 m) | **R** | R | - | one département; planned annecy / annecy cedex (AddInfo twin in its notice) |
| 58 | 23589075 | Eau du Grand Lyon - La Régie | add_info | 19458556 | **R** | R | - | one régie (Eau du Grand Lyon) |
| 59 | 30956421 | Ville de Bastia | add_info | 1289614 | **R** | R | - | one commune (Bastia, a unique name) |
| 60 | 22805349 | Ville du Bouscat | add_info | 1201411 | **R** | R | - | one commune (Le Bouscat, a unique name) |
