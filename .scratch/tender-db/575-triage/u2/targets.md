# 575 U2 — fallback-target and homonym judge (hex-census 2770)

Read-only. Source: `575-triage/hex_census_2770.json` → `targets.top[:30]`, `targets.ccira` (52), `targets.city_flagged` (436). Standard: 530 decision §A5 (top targets: bar 0 W) and §A6 (a target already holding two bodies scores W; a name that denotes more than one body scores W; every W name goes on the unsafe list, whose cf35 key is then KEPT). Verdict basis: **evidence** (target/planned cities or a GET), **name** (placeless kind-of-body name or homonym), **agreement** (place-bearing name, cities agree up to spelling/cedex). 15 public GETs to tenders.zebreus.click, one at a time, 2 s apart, all 200.

## Counts

| list | keys | R | N | W | W cf35 mentions | N cf35 mentions |
|---|---:|---:|---:|---:|---:|---:|
| top 30 by cf35 mentions | 30 | 30 | 0 | 0 | 0 | 0 |
| CCIRA keys | 52 | 33 | 2 | 17 | 2,361 | 9 |
| city-flagged | 436 | 354 | 19 | 63 | 2,238 | 196 |
| **unique keys (lists overlap)** | 497 | 401 | 21 | 75 | 4,056 | 205 |

- **Top 30: 30 R, 0 W.** Avenue-Web Systèmes (71,601 mentions) folds onto its own one-mention NULL row; the other 29 are seat-named tribunals and CCIRA de Bordeaux. TA Grenoble's city_absent 10,005 and TA Saint-Denis's 1,642 are spelling (grenoble cedex / grenible; saint denis = saint denis de la réunion, the only TA of that name).
- **City-flagged: all 436 read, none skipped.** The top 60 and the 164 entries whose name carries no place word of their own cities were judged one by one; the other 195 place-bearing entries were read in one pass and scored R by name/city agreement, with 5 overrides (hôtel de ville W, mairie de saint denis W, mairie de saint- gervais N, epci de corse N, the Mégalis URL N).
- **CCIRA: every placeless spelling whose target exists is W.** The name denotes all seven committees, and most targets' older mentions carry no city. Four are W on evidence: 1169609 plans Bordeaux, Marseille, Nantes and Paris; 8866068 plans Lyon and Nantes; 1614631 is Nancy's committee and plans Bordeaux; 2432686 already reads Paris and Saint-Denis. Seat-named spellings are R. Placeless spellings with no target yet and one planned city are N.
- **The issue's named CCIRA homonyms.** 19913046 (the SGAR form, 122 planned mentions, Marseille only) is W by name: a SGAR sits in every region. The triage GET's 56 SGAR one-mention hex rows fold onto it, and the prefix keeps them. 1163289 (the bare `Comité consultatif interrégional`, 407 mentions) is no cf35 key's target: no cf35 mention prints the bare name. 22394451 carries the stable `1331485` and is no fallback target, because its identifier is not NULL. The short `ccira` key 1222149 (2,941 mentions; city Nantes, planned Nancy) is W.

## The compiled 575 unsafe-name list (keep the cf35 key)

**Rule.** Keep the key when the folded name (`guid_twin_name_words`) equals an exact entry, or when `guid_twin_city_fold(name)` starts with `comite consultatif interregional` (= `store::HEX_CENSUS_CCIRA_PREFIX`, the census's own CCIRA ruler). The list is 575's own: `GUID_TWIN_UNSAFE_NAMES` stays untouched. That list already holds `Tribunal administratif`, but 575 needs its own copy.

**Prefix rather than exact spellings.** Seven committees publish under 52 spellings. Some are typos (`interrégionalde`, `diffférends`, `demarseille`, `réglement`), and the fixture's long form is just one of them. An exact list would miss the next spelling, and that spelling would mint a fresh fallback row open to every region. The prefix binds the fixture form (key 1196898, 206 mentions) and the run-together typo. Reach: 52 keys, 3,594 mentions (17 W / 2 N / 33 R; W 2,361, N 9, R 1,224).

- *Cost:* the 1,224 mentions under seat-named spellings would fold correctly, but they stay split (today's state).
- *Seat-exempt variant:* keep the key only when the name names no seat. It keeps 19 keys / 2,370 mentions and recovers 1,224, at the price of a seat list in the gate.
- *Broader `comite consultatif` prefix: not recommended.* It would also keep the R Versailles interdepartmental committee (1165581, 413 mentions). The three W committee names outside the CCIRA prefix are listed exactly instead.

| # | folded entry | spellings (judged keys) | target ids | cf35 mentions | already caught by |
|---:|---|---|---|---:|---|
| 1 | `tribunal administratif` | tribunal administratif | 1163218 | 281 | GUID_TWIN_UNSAFE_NAMES (530) |
| 2 | `comite consultatif interdepartemental de reglement amiable des litiges` | comité consultatif interdépartemental de règlement amiable d | 1163657 | 278 | — |
| 3 | `tribunal judiciaire` | tribunal judiciaire | 10082662 | 148 | — |
| 4 | `comite consultatif de reglement amiable des litiges` | comité consultatif de règlement amiable des litiges | 1171446 | 134 | — |
| 5 | `ccas` | c.c.a.s; ccas | 1206088, 3574992 | 128 | content_words<2 |
| 6 | `service de la commande publique` | service de la commande publique | 3443 | 97 | — |
| 7 | `centre communal d action sociale` | centre communal d action sociale; centre communal d' action sociale; centre communal d'action sociale | 1174257, 8939560, 2817853 | 95 | generic_body_name |
| 8 | `conseil departemental` | conseil departemental; conseil départemental | 8168454, 6879005 | 65 | generic_body_name |
| 9 | `mairie` | mairie | 1163705 | 64 | content_words<2, generic_body_name |
| 10 | `direction de la commande publique` | direction de la commande publique | 1166638 | 57 | — |
| 11 | `hotel de ville` | hotel de ville; hôtel de ville | 1163279, 1228976 | 48 | — |
| 12 | `service commande publique` | service commande publique | 1229850 | 41 | — |
| 13 | `caisse des ecoles` | caisse des ecoles | 1201208 | 34 | — |
| 14 | `hotel du departement` | hotel du département; hôtel du département | 1185018, 8262162 | 32 | — |
| 15 | `centre communal d apos action sociale` | centre communal d&apos;action sociale | None | 25 | — |
| 16 | `direction commande publique mutualisee` | direction commande publique mutualisée | 22569106 | 13 | — |
| 17 | `service des marches` | service des marchés | 1363144 | 11 | — |
| 18 | `service marches publics` | service marches publics; service marchés publics | 1163277, 1298079 | 11 | — |
| 19 | `pour obtenir tous renseignements complementaires qui leur seraient necessaires au cours de` | pour obtenir tous renseignements complémentaires qui leur se | None | 10 | — |
| 20 | `commune` | commune | 1228180 | 8 | content_words<2, generic_body_name |
| 21 | `direction des services techniques` | direction des services techniques | 1251437 | 8 | — |
| 22 | `semag` | semag | 1279835 | 8 | content_words<2 |
| 23 | `service achats` | service achats | 1163276 | 8 | — |
| 24 | `communaute urbaine` | communauté urbaine | 2102005 | 7 | — |
| 25 | `service marches` | service marchés | 1179591 | 7 | — |
| 26 | `ccira` | ccira | 1222149 | 6 | content_words<2 |
| 27 | `centre communal d action sociale ccas` | centre communal d'action sociale (ccas) | 1896679 | 6 | generic_body_name |
| 28 | `direction de l achat public` | direction de l'achat public | 9224696 | 6 | — |
| 29 | `service achats marches` | service achats/marchés | 31975711 | 6 | — |
| 30 | `service juridique et commande publique` | service juridique et commande publique | 1419596 | 6 | — |
| 31 | `mairie de saint denis` | mairie de saint denis | 31429325 | 4 | — |
| 32 | `service des marches publics` | service des marchés publics | 1181455 | 4 | — |
| 33 | `service departemental d incendie et de secours` | service départemental d'incendie et de secours | 1299400 | 4 | — |
| 34 | `ccsb` | ccsb | None | 3 | content_words<2 |
| 35 | `centre intercommunal d action sociale` | centre intercommunal d'action sociale | 1244397 | 3 | generic_body_name |
| 36 | `college george sand` | collège george sand | 3490748 | 3 | — |
| 37 | `commande publique` | commande publique | 1186120 | 3 | — |
| 38 | `service technique de la ville` | service technique de la ville | None | 3 | — |
| 39 | `aws` | aws | 1400495 | 2 | content_words<2 |
| 40 | `cdc` | cdc | 1549719 | 2 | content_words<2 |
| 41 | `centre de gestion de la fonction publique territoriale` | centre de gestion de la fonction publique territoriale | 2560557 | 2 | — |
| 42 | `commune et ccas` | commune et ccas | 31140454 | 2 | — |
| 43 | `direction commande publique` | direction commande publique | 3426182 | 2 | — |
| 44 | `ehpad` | ehpad | 1338203 | 2 | content_words<2 |
| 45 | `restaurant scolaire` | restaurant scolaire | 3457028 | 2 | — |
| 46 | `centre technique municipal` | centre technique municipal | 1179286 | 1 | — |
| 47 | `comite consultatif de reglement amiable des litiges relatifs aux marches publics` | comité consultatif de règlement amiable des litiges relatifs | 1201220 | 1 | — |
| 48 | `groupement de commande` | groupement de commande | 3416016 | 1 | — |
| 49 | `profil acheteur` | profil acheteur | 1728706 | 1 | — |
| 50 | `sce departemental incendie et secours` | sce departemental incendie et secours | 9485683 | 1 | — |
| 51 | `tribunal de grande instance` | tribunal de grande instance | 1163205 | 1 | — |

**Mentions spared** (their cf35 key kept instead of refused): prefix 3,594 plus exact 1,695 = **5,289**, 2.35% of the 224,611-mention class. Of those, 4,056 are W mentions; the rest are the prefix's N and R spellings.

**Reusing 530's structural tests.** 530 also guards with `content_words<2` and `generic_body_name`. On these keys, `generic_body_name` costs nothing. `content_words<2` would also keep 42 R/N one-word names (577 mentions: brgm, cnfpt, gpmm, dalkia, orange …) split. The exact list stands alone; the last column shows what a reused test would already catch.

**Limits.**
- only the targets the report lists (top 50, the 436 city-flagged, the 52 CCIRA keys) were screened; a placeless generic name whose cf35 mentions all print ONE city and whose target shows no other city is in no list (e.g. a bare greffe); U3/U4 should run the guard over all 4,847 keys and list placeless names it does not catch
- target cities come from the newest 50 mentions only (report not_measured); many big targets read cities=[] (pre-eForms mentions), so W-by-name verdicts stand in for unreadable older mentions
- centre communal d&apos;action sociale is an HTML-escaped spelling: listed as folded (d apos action); decoding entities before the fold would make the plain entry cover it
- the boilerplate sentence entry is a junk name better refused by a non-name gate (586/587) than listed
- Homonym candidates seen in the buyer / r2 samples, not judged here: mairie de saint andré (several communes Saint-André (incl. La Réunion); buyer sample only); mairie de saint marcel (several communes Saint-Marcel; buyer sample only); commune de saint-just (several communes Saint-Just; buyer sample only); montfaucon / gennes / saône / venise (bare commune names as buyer names) (homonymous communes (Gennes: Doubs and Maine-et-Loire; Montfaucon: several); buyer-role, the buyer judge's area); cciral (bare acronym (r2_random); like ccira it can name more than one committee); profil d'acheteur : (label, not a body (buyer sample, 7 mentions); same class as profil acheteur).

## GETs (15, public API, 2 s apart, all 200)

1 `/v1`; 2 `/v1/organizations/1163197` (no city fields: review-body targets cannot be read by GET); 3 `/v1/openapi.json` (no review-body filter); 4 `tenders?buyer=31429325` (empty: not a buyer); 5 `tenders?buyer=1279835` → SEMAG target already holds Gennevilliers, Abymes and Gardanne tenders; 6 `tenders?buyer=1400495` (empty); 7 `tenders?buyer=3490748` → Collège George Sand target already holds FR82 and FRI34 tenders; 8 `tenders?buyer=22680256` (empty); 9 `tenders?buyer=1549719` → CDC target is a Donzy (Nièvre) buyer; 10 `tenders?buyer=2102005` (empty); 11 `tenders?buyer=1206088` → CCAS target holds Besançon, Châtellerault, Saint-Rémy-de-Provence …; 12 `organizations?name_prefix=Mairie de Saint-Denis` → both Saint-Denis (93) and Saint-Denis (Réunion) rows; 13 `tenders?buyer=2560557` → CDG target holds Saint-Grégoire, Belfort, Aix, Luisant, Pau, Gap; 14 `tenders?buyer=1299400` → SDIS target is SDIS 71 (Sancé); 15 `tenders?buyer=1338203` → EHPAD target holds a dozen towns.

## Every judged key

Order: top 30, then CCIRA, then city-flagged (a key in several lists appears once, at its first list). `lists` gives every list#rank.

| verdict | basis | name_norm | target id | cf35 | target mentions | lists | reason |
|---|---|---|---:|---:|---:|---|---|
| R | evidence | avenue-web systèmes | 8301019 | 71,601 | 1 | top#1 | the eSender itself; target 8301019 is the platform's own NULL row (1 mention), planned city Seyssinet-Pariset = AWS head office |
| R | agreement | tribunal administratif de grenoble | 1163404 | 10,076 | 40,316 | top#2, city_flagged#1 | one tribunal; city_absent 10,005 is spelling only (target 'grenoble cedex' vs planned grenoble / grenible / grenoble cedex) |
| R | agreement | tribunal administratif de lyon | 1163241 | 6,777 | 46,596 | top#3, city_flagged#2 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | tribunal administratif de toulouse | 1163070 | 5,728 | 32,677 | top#4 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | tribunal administratif de montpellier | 1163864 | 5,553 | 27,265 | top#5, city_flagged#3 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | tribunal administratif de marseille | 1162955 | 4,886 | 35,999 | top#6 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | tribunal administratif d'orléans | 1163223 | 4,510 | 26,883 | top#7 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | tribunal administratif de rouen | 1163582 | 4,442 | 23,256 | top#8 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | tribunal administratif de nantes | 1163502 | 4,431 | 47,816 | top#9, city_flagged#4 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | tribunal administratif de lille | 1163457 | 4,338 | 52,385 | top#10, city_flagged#5 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | tribunal administratif de bordeaux | 1163131 | 4,223 | 30,364 | top#11, city_flagged#6 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | tribunal administratif de strasbourg | 1163701 | 4,119 | 29,128 | top#12 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | tribunal administratif de versailles | 1163158 | 3,445 | 36,248 | top#13, city_flagged#7 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | tribunal administratif de caen | 1166355 | 3,409 | 14,538 | top#14 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | tribunal administratif de cergy-pontoise | 1163111 | 3,288 | 35,546 | top#15, city_flagged#8 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | tribunal judiciaire de paris | 8835 | 2,722 | 11,764 | top#16 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | tribunal administratif de pau | 1163196 | 2,443 | 18,015 | top#17, city_flagged#9 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | tribunal administratif de poitiers | 1165316 | 2,377 | 14,287 | top#18 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | tribunal administratif de melun | 1165479 | 2,140 | 34,275 | top#19 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | tribunal administratif de clermont-ferrand | 1163465 | 1,787 | 10,395 | top#20 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | tribunal administratif de toulon | 1163191 | 1,726 | 22,310 | top#21, city_flagged#10 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | tribunal administratif de nîmes | 1163138 | 1,679 | 14,493 | top#22, city_flagged#11 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | tribunal administratif de paris | 1163143 | 1,675 | 94,831 | top#23 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | tribunal administratif de saint-denis | 1164108 | 1,642 | 5,922 | top#24, city_flagged#12 | the only TA named Saint-Denis is La Réunion's (Seine-Saint-Denis is TA Montreuil); 'saint denis' vs 'saint denis de la reunion' is one city |
| R | agreement | tribunal administratif d'amiens | 1164778 | 1,608 | 17,436 | top#25 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | ccira de bordeaux | 1176664 | 1,562 | 5,619 | top#26, city_flagged#13 | one committee; Mérignac / Bordeaux cedex are its own address variants |
| R | agreement | tribunal judiciaire de marseille | 14658291 | 1,542 | 2,248 | top#27 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | tribunal administratif de bastia | 1163175 | 1,523 | 10,110 | top#28 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | tribunal administratif de besançon | 1165058 | 1,494 | 10,852 | top#29 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | tribunal administratif de montreuil | 1164672 | 1,330 | 26,965 | top#30 | one tribunal; target and planned cities agree up to spelling / cedex |
| W | name | comité consultatif interrégional de réglement | 2585842 | 472 | 448 | ccira#1 | placeless CCIRA name denotes all seven committees; truncated name, denotes any committee; Lyon-only evidence today (448-mention target) |
| W | name | comité consultatif interrégional pour le règlement amiable des marchés publics | 1167628 | 355 | 1,119 | ccira#2 | placeless CCIRA name denotes all seven committees; nantes-only evidence today (1119-mention target, older cities unread) |
| W | name | comité consultatif interrégional de règlement amiable des différends ou litiges relatifs aux marchés publics | 1169310 | 279 | 4,732 | ccira#3, city_flagged#24 | placeless CCIRA name denotes all seven committees; the official long name of all seven committees; 4,732-mention target (newest 50 Nantes, older mentions unread) |
| W | name | comité consultatif interrégional de règlement amiables des litiges | 1196158 | 220 | 662 | ccira#4 | placeless CCIRA name denotes all seven committees; nantes-only evidence today (662-mention target, older cities unread) |
| W | name | comité consultatif interrégional de réglement amiable des différends ou litiges relatifs aux marchés publics | 1196898 | 206 | 82 | ccira#5 | placeless CCIRA name denotes all seven committees; the fixture's long form (é typo); denotes any committee; Paris-only evidence today |
| R | agreement | comité consultatif interrégional de paris, règlement amiable des litiges | 1952935 | 186 | 488 | ccira#6 | names its seat; target/planned cities agree up to cedex |
| W | evidence | comité consultatif interrégional de règlement amiable des litiges | 1169609 | 178 | 4,979 | ccira#7, city_flagged#34 | placeless CCIRA name; planned bordeaux, marseille, nantes, paris |
| W | name | comité consultatif interrégional - prefecture de région sgar | 19913046 | 122 | 81 | ccira#8 | placeless CCIRA name denotes all seven committees; a SGAR sits in every region; Marseille-only evidence today; the 56 SGAR one-mention rows of the triage GET fold here |
| R | agreement | comité consultatif interrégional de règlement amiable de nancy | 1240088 | 112 | 678 | ccira#9 | names its seat; target/planned cities agree up to cedex |
| W | name | comité consultatif interrégional des litiges en matière de marchés publics | 4867485 | 110 | 423 | ccira#10 | placeless CCIRA name denotes all seven committees; lyon cedex-only evidence today (423-mention target, older cities unread) |
| R | agreement | comité consultatif interrégional de règlement amiable des différends ou litiges relatifs aux marchés publics d | 8152284 | 98 | 166 | ccira#11 | names its seat; target/planned cities agree up to cedex |
| W | name | comité consultatif interrégional de règlement amiable des litiges des marchés publics | 1307171 | 97 | 611 | ccira#12 | placeless CCIRA name denotes all seven committees; paris-only evidence today (611-mention target, older cities unread) |
| W | name | comité consultatif interrégional de règlement amiable des litiges relatifs aux marchés publics | 1163197 | 90 | 2,636 | ccira#13 | placeless CCIRA name denotes all seven committees; placeless; 2,636-mention target (cities unread); planned Ajaccio |
| R | agreement | comité consultatif interrégional de règlement amiable des différents relatifs aux marchés publics de marseille | 21482118 | 85 | 18 | ccira#14 | names its seat; target/planned cities agree up to cedex |
| W | name | comité consultatif interrégional du règlement amiable | 1446562 | 70 | 310 | ccira#15 | placeless CCIRA name denotes all seven committees; lyon-only evidence today (310-mention target, older cities unread) |
| R | agreement | comité consultatif interrégional de règlement amiable des différends ou litiges relatif aux marchés publics de | 10581604 | 67 | 190 | ccira#16 | names its seat; target/planned cities agree up to cedex |
| R | agreement | comité consultatif interrégional de règlement amiable des différends ou litiges relatifs aux marchés publics p | 16701687 | 63 | 32 | ccira#17 | names its seat; target/planned cities agree up to cedex |
| R | agreement | comité consultatif interrégional de règlement amiable des différends relatifs aux marchés publics de marseille | 17544562 | 63 | 66 | ccira#18 | names its seat; target/planned cities agree up to cedex |
| R | agreement | comité consultatif interrégional de règlement amiable des différends ou litiges relatifs aux marchés publics - | 13190471 | 60 | 61 | ccira#19 | names its seat; target/planned cities agree up to cedex |
| R | agreement | comité consultatif interrégional de règlement amiable des différends ou litiges relatifs aux marchés publics d | 1184505 | 58 | 757 | ccira#20 | names its seat; target/planned cities agree up to cedex |
| W | evidence | comité consultatif interrégional de règlement amiable des litiges relatifs aux marchés publics (ccira) | 1614631 | 57 | 78 | ccira#21, city_flagged#68 | target city Nancy (Nancy's committee), planned Bordeaux cedex: a different committee |
| R | agreement | comité consultatif interrégional de règlement amiable des différends relatifs aux marchés publics de lyon | 13400922 | 56 | 231 | ccira#22, city_flagged#70 | names its seat; target/planned cities agree up to cedex |
| W | name | comité consultatif interrégional du règlement amiable des litiges | 1247936 | 56 | 851 | ccira#23 | placeless CCIRA name denotes all seven committees; nancy-only evidence today (851-mention target, older cities unread) |
| R | agreement | comité consultatif interrégional de réglement amiable des différends ou litiges relatifs aux marchés publics d | 1483978 | 54 | 49 | ccira#24 | names its seat; target/planned cities agree up to cedex |
| R | agreement | comité consultatif interrégional de règlement amiable des différends ou litiges relatifs aux marchés publics d | 9894992 | 43 | 42 | ccira#25 | names its seat; target/planned cities agree up to cedex (Versailles committee seated in Paris 15) |
| R | agreement | comité consultatif interrégional de règlement des différents de paris | 14986738 | 38 | 29 | ccira#26 | names its seat; target/planned cities agree up to cedex |
| R | agreement | comité consultatif interrégional de règlement amiable des différends ou litiges relatifs aux marchés publics d | 1502440 | 36 | 1,309 | ccira#27 | names its seat; target/planned cities agree up to cedex |
| R | agreement | comité consultatif interrégional de règlement amiable de lyon | 14242692 | 35 | 2 | ccira#28 | names its seat; target/planned cities agree up to cedex |
| R | agreement | comité consultatif interrégional de réglement amiable des litiges de bordeaux | 14959705 | 35 | 28 | ccira#29 | names its seat; target/planned cities agree up to cedex (Bordeaux committee, Mérignac address) |
| R | agreement | comité consultatif interrégional de règlement amiable des litiges relatifs aux marchés publics de nantes | 1224999 | 20 | 219 | ccira#30 | names its seat; target/planned cities agree up to cedex |
| W | evidence | comité consultatif interrégional de règlement amiable des litiges relatif aux marchés publics | 2432686 | 19 | 45 | ccira#31, city_flagged#148 | placeless CCIRA name; target cities Paris + Saint-Denis already |
| R | agreement | comité consultatif interrégional de règlement amiable des différents ou litiges relatifs aux marchés publics d | 1450776 | 16 | 80 | ccira#32 | names its seat; target/planned cities agree up to cedex |
| W | name | comité consultatif interrégional de règlement amiable des litiges (cciral) | 1227238 | 16 | 405 | ccira#33 | placeless CCIRA name denotes all seven committees; placeless (CCIRAL is an acronym, not a seat; Marseille's committee uses it); 405-mention target |
| R | agreement | comité consultatif interrégional de règlement amiable des litiges relatifs aux marchés publics de bordeaux | 1168993 | 14 | 1,430 | ccira#34 | names its seat; target/planned cities agree up to cedex |
| R | agreement | comité consultatif interrégional de règlement amiable des différends ou litiges relatifs aux marchés publics d | 2103651 | 13 | 299 | ccira#35, city_flagged#182 | names its seat; target/planned cities agree up to cedex |
| R | agreement | comité consultatif interrégional de nancy de règlement amiable des litiges relatifs aux marchés publics | 16078359 | 12 | 35 | ccira#36 | names its seat; target/planned cities agree up to cedex |
| R | agreement | comité consultatif interrégional de règlement amiable des litiges de bordeaux | 8254035 | 12 | 76 | ccira#37 | names its seat; target/planned cities agree up to cedex |
| W | evidence | comité consultatif interrégional de règlement amiable des différends relatifs aux marchés publics | 8866068 | 10 | 340 | ccira#38, city_flagged#217 | placeless CCIRA name; planned Lyon + Nantes |
| N | name | comité consultatif interrégional de règlement amiable des diffférends et litiges relatif aux marchés publics | missing | 8 | 0 | ccira#39 | placeless CCIRA spelling; missing target would be minted for one city (marseille) today |
| R | agreement | comité consultatif interrégional de règlement amiable des litiges en matière de marchés publics de la région r | 12570830 | 8 | 69 | ccira#40, city_flagged#239 | names its seat; target/planned cities agree up to cedex |
| R | agreement | comité consultatif interrégional de règlement amiable des litiges relatifs marches publics de lyon | missing | 7 | 0 | ccira#41 | names its seat; target/planned cities agree up to cedex |
| R | agreement | comite consultatif interregional de reglement amiable des litiges relatifs aux marches publics de lyon | 2398700 | 6 | 5 | ccira#42 | names its seat; target/planned cities agree up to cedex |
| R | agreement | comité consultatif interrégional de règlement amiable des différends et litiges relatifs aux marchés publics ( | 8997820 | 6 | 13 | ccira#43 | names its seat; target/planned cities agree up to cedex |
| R | agreement | comité consultatif interrégional de règlement amiable des différends ou litiges relatifs aux marchés publics d | 9633473 | 5 | 62 | ccira#44 | names its seat; target/planned cities agree up to cedex |
| R | agreement | comité consultatif interrégional de règlement amiable des différends et litiges en matière de marchés publics  | 18836122 | 4 | 5 | ccira#45 | names its seat; target/planned cities agree up to cedex |
| R | agreement | comité consultatif interrégional de règlement amiable des différends relatifs aux marchés publics demarseille | missing | 4 | 0 | ccira#46 | names its seat; target/planned cities agree up to cedex |
| W | name | comité consultatif interrégional de règlement des litiges en matière de marchés publics | 1318162 | 4 | 71 | ccira#47 | placeless CCIRA name denotes all seven committees; marseille-only evidence today (71-mention target, older cities unread) |
| R | agreement | comité consultatif interrégional de paris | 1166008 | 3 | 105 | ccira#48 | names its seat; target/planned cities agree up to cedex |
| R | agreement | comité consultatif interrégional de règlement amiable des différends relatifs aux marchés publics de paris. pr | missing | 3 | 0 | ccira#49 | names its seat; target/planned cities agree up to cedex |
| R | agreement | comité consultatif interrégional de règlement amiable des différends ou litiges relatifs aux marchés publics d | 1169171 | 1 | 134 | ccira#50 | names its seat; target/planned cities agree up to cedex |
| R | agreement | comité consultatif interrégional de règlement des litiges en matière de marchés publics, préfecture de la régi | missing | 1 | 0 | ccira#51 | names its seat; target/planned cities agree up to cedex |
| N | name | comité consultatif interrégionalde règlement amiable des diférents ou litige relatifs aux marchés publics | missing | 1 | 0 | ccira#52 | placeless CCIRA spelling; missing target would be minted for one city (lyon) today |
| R | agreement | tribunal judiciaire de nanterre | 14609486 | 1,119 | 5,425 | city_flagged#14 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | tribunal administratif de nice | 1163597 | 983 | 20,672 | city_flagged#15 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | département de la haute-savoie | 1177949 | 642 | 3,277 | city_flagged#16 | one département (Annecy) |
| R | agreement | tribunal judiciaire de lyon | 14610123 | 590 | 2,368 | city_flagged#17 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | tribunal administratif de nancy | 1163795 | 570 | 14,378 | city_flagged#18 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | greffe du tribunal administratif de lyon | 1163242 | 528 | 5,248 | city_flagged#19 | TA Lyon's registry; city_absent 528 is 'lyon' vs 'lyon cedex 3' |
| R | evidence | comité consultatif interdépartemental de versailles de règlement amiable des litiges relatifs aux marchés publ | 1165581 | 413 | 2,796 | city_flagged#20 | the Versailles committee sits at the Île-de-France prefecture, 5 rue Leblanc, Paris 15: target cities are that address |
| R | agreement | département de la vendée | 1640823 | 304 | 43 | city_flagged#21 | one département; Les Sables-d'Olonne / Luçon are its local sites |
| R | agreement | médiation des entreprises | 8204036 | 288 | 47 | city_flagged#22 | one national service (Médiateur des entreprises); 'nantes' vs 'nantes cedex 1' |
| W | evidence | tribunal administratif | 1163218 | 281 | 99,159 | city_flagged#23 | bare name of every TA: target 1163218 (99,159 mentions) and planned Bastia, Clermont-Ferrand, Grenoble, Montpellier, Saint-Denis; already on 530 GUID_TWIN_UNSAFE_NAMES |
| W | evidence | comité consultatif interdépartemental de règlement amiable des litiges | 1163657 | 278 | 1,323 | city_flagged#25 | placeless; planned Marseille + Paris: two committees under one key (1,323-mention target) |
| R | agreement | tribunal judiciaire de rennes | 14590372 | 264 | 1,317 | city_flagged#26 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | tribunal administratif de fort-de-france | 1175105 | 250 | 1,922 | city_flagged#27 | TA de la Martinique sits in Schoelcher (Fort-de-France agglomeration); one tribunal |
| R | agreement | tribunal judiciaire de toulouse | 14932995 | 235 | 298 | city_flagged#28 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | ccira de marseille | 8064428 | 223 | 380 | city_flagged#29 | one committee; 'marseille' vs 'marseille cedex 06' |
| R | agreement | tribunal judiciaire de metz | 15762268 | 211 | 657 | city_flagged#30 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | eurométropole de strasbourg | 6174352 | 205 | 926 | city_flagged#31 | one body; strasbourg vs strasbourg cedex |
| R | agreement | ville de strasbourg | 1174297 | 204 | 2,681 | city_flagged#32 | one commune |
| R | agreement | greffe du tribunal judiciaire de paris | 14596190 | 186 | 3,356 | city_flagged#33 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | tribunal de grande instance de bordeaux | 1163888 | 160 | 2,078 | city_flagged#35 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | ville de limoges - service de la commande publique | 1192068 | 153 | 260 | city_flagged#36 | Limoges named; cedex variant |
| R | agreement | conseil départemental du nord | 6710430 | 150 | 457 | city_flagged#37 | one département (Lille) |
| W | evidence | tribunal judiciaire | 10082662 | 148 | 5,327 | city_flagged#38 | bare name of every TJ; planned Lille + Lyon; 5,327-mention target |
| R | agreement | tribunal judiciaire d'annecy | 14834187 | 146 | 72 | city_flagged#39 | one tribunal; target and planned cities agree up to spelling / cedex |
| W | evidence | comité consultatif de règlement amiable des litiges | 1171446 | 134 | 1,896 | city_flagged#40 | placeless; target city Nancy, planned Lyon + Nantes |
| R | agreement | tribunal de grande instance de nanterre | 1180450 | 131 | 8,484 | city_flagged#41 | one tribunal; target and planned cities agree up to spelling / cedex |
| W | evidence | ccas | 1206088 | 121 | 852 | city_flagged#42 | placeless; planned 35 cities; GET: target 1206088 already pools CCAS / communes of Besançon, Châtellerault, Saint-Rémy-de-Provence ... |
| R | agreement | oph var habitat | 1327327 | 119 | 430 | city_flagged#43 | one OPH (Toulon; La Valette-du-Var is adjacent) |
| R | agreement | tours métropole val de loire | 9685765 | 117 | 445 | city_flagged#44 | one body |
| R | agreement | région grand est | 8151783 | 110 | 984 | city_flagged#45 | one region with three seats (Strasbourg, Metz, Châlons) |
| R | agreement | saint-etienne metropole | 19715094 | 107 | 78 | city_flagged#46 | one body |
| R | agreement | bordeaux métropole | 6336369 | 97 | 2,048 | city_flagged#47 | one body (Pessac is a member site) |
| W | evidence | service de la commande publique | 3443 | 97 | 1,026 | city_flagged#48 | generic department label (587); target cities Argentan, Hazebrouck, Les Mureaux, Val-de-Reuil; planned 12 more cities |
| R | agreement | métropole rouen normandie | 6128976 | 94 | 620 | city_flagged#49 | one body |
| W | evidence | centre communal d'action sociale | 1174257 | 93 | 2,565 | city_flagged#50 | placeless; target cities Levallois + Versailles, planned 18 cities |
| R | agreement | ville de montpellier | 1207939 | 92 | 1,395 | city_flagged#51 | one commune |
| R | agreement | conseil départemental du val-d'oise | 7980854 | 91 | 12 | city_flagged#52 | one département (Cergy) |
| R | agreement | département des yvelines | 1303178 | 90 | 142 | city_flagged#53 | one département |
| R | agreement | sytral mobilités | 20357836 | 85 | 14 | city_flagged#54 | one body (Lyon) |
| R | agreement | tribunal administratif de la réunion | 1190585 | 84 | 2,298 | city_flagged#55 | one tribunal (Saint-Denis de La Réunion) |
| R | agreement | conseil départemental de l'essonne | 6891187 | 81 | 110 | city_flagged#56 | one département (Évry) |
| R | agreement | ville de limoges | 1190852 | 81 | 1,243 | city_flagged#57 | one commune |
| R | agreement | gpmm | 1383923 | 76 | 19 | city_flagged#58 | Grand Port Maritime de Marseille; Port-Saint-Louis-du-Rhône is its western basin |
| R | agreement | département des hauts-de-seine | 1192140 | 74 | 170 | city_flagged#59 | one département (Nanterre) |
| R | agreement | mairie d'annecy | 12719408 | 72 | 23 | city_flagged#60 | one commune |
| R | agreement | centre hospitalier de cayenne | 1405548 | 70 | 320 | city_flagged#61 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | ville de montauban | 1175472 | 68 | 1,066 | city_flagged#62 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | régie immobilière ville de paris | 17096451 | 65 | 51 | city_flagged#63 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| W | evidence | mairie | 1163705 | 64 | 1,722 | city_flagged#64 | placeless; planned 37 communes; 1,722-mention target |
| R | agreement | département du nord | 4039622 | 63 | 50 | city_flagged#65 | one département (Lille) |
| R | agreement | brgm | 1166409 | 59 | 472 | city_flagged#66 | one national body (Orléans head office, regional sites) |
| R | agreement | haute-savoie habitat | 2456177 | 58 | 167 | city_flagged#67 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| W | evidence | direction de la commande publique | 1166638 | 57 | 1,356 | city_flagged#69 | 587 example row 1166638; target cities Levallois, Lyon, Saint-Denis, Saint-Tropez, Sainte-Clotilde |
| R | agreement | grand annecy | 21179068 | 55 | 6 | city_flagged#71 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | eau de paris | 1246448 | 51 | 630 | city_flagged#72 | one régie; Montigny-sur-Loing / Montreuil are its sites |
| R | agreement | métropole de lyon | 6202537 | 51 | 2,321 | city_flagged#73 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | ville de bègles | 1227111 | 51 | 163 | city_flagged#74 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | centre hospitalier de kourou | 19113595 | 50 | 42 | city_flagged#75 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | conseil départemental de la dordogne | 22504965 | 50 | 2 | city_flagged#76 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | region grand est | 9095418 | 50 | 642 | city_flagged#77 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | ville de saint-nazaire | 1551255 | 50 | 215 | city_flagged#78 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| W | evidence | conseil departemental | 8168454 | 49 | 18 | city_flagged#79 | placeless; target Angoulême + Arras, planned Aurillac, Valence ... |
| R | agreement | centre hospitalier universitaire de bordeaux | 8237204 | 47 | 340 | city_flagged#80 | CHU Bordeaux (Talence site) |
| R | agreement | département de la moselle | 1232798 | 46 | 249 | city_flagged#81 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | eaux de vienne - siveer | 8759339 | 46 | 54 | city_flagged#82 | one syndicate (Poitiers) |
| R | agreement | loire forez agglomération | 14800 | 46 | 98 | city_flagged#83 | one body (Montbrison) |
| R | agreement | département de la sarthe | 4734519 | 44 | 165 | city_flagged#84 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | erilia | 1882308 | 43 | 151 | city_flagged#85 | one ESH (Marseille) |
| R | agreement | centre hospitalier de l'ouest guyanais | 1892648 | 42 | 43 | city_flagged#86 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | communauté urbaine le havre seine métropole | 12853130 | 42 | 43 | city_flagged#87 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | ville de colmar | 1192106 | 42 | 810 | city_flagged#88 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| N | name | comité consultatif inter-départemental de règlement amiable des différends ou litiges | 8223723 | 41 | 71 | city_flagged#89 | 'interdépartemental' names the Versailles committee (seated in Paris), planned Paris only; but 1163657 shows notices also misname others so; 71-mention target, cities unknown |
| R | agreement | grand besançon métropole | 13803966 | 41 | 216 | city_flagged#90 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| W | evidence | service commande publique | 1229850 | 41 | 416 | city_flagged#91 | generic department / service label (587 class): names a kind of unit every buyer has; target and planned span 16+ towns |
| R | agreement | atlantique habitations | 1249377 | 38 | 494 | city_flagged#92 | one ESH (Saint-Herblain, misspellings) |
| R | agreement | saint-nazaire agglomération - la carene | 22399706 | 38 | 12 | city_flagged#93 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | communauté d'agglomération bourges plus | 8147036 | 36 | 71 | city_flagged#94 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | departement de la haute-savoie | 1579563 | 35 | 688 | city_flagged#95 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| W | evidence | caisse des ecoles | 1201208 | 34 | 301 | city_flagged#96 | placeless; planned Deuil-la-Barre, Évreux, Levallois, Saint-Denis, Septèmes, Le Tampon |
| R | agreement | communauté d'agglomération cannes-pays de lérins | 9718349 | 34 | 15 | city_flagged#97 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | grand montauban communauté d'agglomération | 1173273 | 34 | 186 | city_flagged#98 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | grand port maritime guadeloupe | 22694497 | 34 | 2 | city_flagged#99 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | spl arac occitanie | 15990990 | 34 | 6 | city_flagged#100 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | tarn et garonne habitat | 1228264 | 34 | 67 | city_flagged#101 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | tribunal de grande instance de metz | 1202255 | 34 | 672 | city_flagged#102 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | colmar agglomération (coordonnateur) | missing | 33 | 0 | city_flagged#103 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | société du canal de provence et d'aménagement de la région provençale | 1167768 | 33 | 24 | city_flagged#104 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | batigere habitats solidaires | 20732731 | 32 | 14 | city_flagged#105 | one ESH (Metz head office, Paris office) |
| R | agreement | igesa | 1406921 | 32 | 459 | city_flagged#106 | one national body (Bastia) |
| R | agreement | sarthe habitat | 1260914 | 32 | 273 | city_flagged#107 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | ville de rouen | 1164348 | 32 | 1,288 | city_flagged#108 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| W | evidence | hôtel de ville | 1163279 | 31 | 789 | city_flagged#109 | placeless; target Manosque + Villefontaine, planned 14 communes |
| R | agreement | communauté d'agglomération seine eure | 2111424 | 30 | 34 | city_flagged#110 | one body (Louviers) |
| N | name | http://marches-publics.info | 22125969 | 30 | 1 | city_flagged#111 | the platform's URL as a name: one platform, the planned cities are the buyers' |
| R | agreement | dijon metropole | 14074961 | 29 | 52 | city_flagged#112 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | odyssi | 12838335 | 29 | 75 | city_flagged#113 | one body (Fort-de-France) |
| R | agreement | saint-lo agglo | 13274341 | 29 | 9 | city_flagged#114 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | ville du havre | 1198364 | 29 | 1,551 | city_flagged#115 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | tribunal de grande instance de paris | 1162930 | 28 | 34,404 | city_flagged#116 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | ville de saint-quentin | 1168312 | 28 | 490 | city_flagged#117 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | ville narbonne | 17992576 | 28 | 4 | city_flagged#118 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | colmar agglomération | 7207805 | 27 | 93 | city_flagged#119 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | groupement hospitalier de territoire d'eure-et-loir | 11825193 | 27 | 29 | city_flagged#120 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | ville de saint nazaire | missing | 27 | 0 | city_flagged#121 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | logemloiret | 1185967 | 26 | 335 | city_flagged#122 | one OPH (Orléans) |
| R | agreement | port autonome de strasbourg | 1218882 | 26 | 556 | city_flagged#123 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | thonon agglomeration | 19377205 | 26 | 80 | city_flagged#124 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| W | evidence | centre communal d&apos;action sociale | missing | 25 | 0 | city_flagged#125 | entity-escaped CCAS; missing target would be minted for Bourges, Castres, Limoges, Montauban, Saint-Étienne, Saint-Quentin |
| R | agreement | communauté d&apos;agglomération cannes-pays de lérins | missing | 25 | 0 | city_flagged#126 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | région occitanie | 8817678 | 25 | 907 | city_flagged#127 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| W | evidence | hôtel du département | 1185018 | 24 | 132 | city_flagged#128 | placeless; target Guéret + Nanterre, planned 9 départements |
| R | agreement | mairie de brive | 1298500 | 23 | 131 | city_flagged#129 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | université d'artois | 1250228 | 23 | 342 | city_flagged#130 | one university (Arras) |
| R | agreement | ville de chambéry | 1300728 | 23 | 321 | city_flagged#131 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | carcassonne agglo | 1290791 | 22 | 280 | city_flagged#132 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | communauté urbaine d'alençon | 1256170 | 22 | 133 | city_flagged#133 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | evreux portes de normandie | 10112482 | 22 | 29 | city_flagged#134 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | université côte d'azur | 14916635 | 22 | 4 | city_flagged#135 | one university (Nice) |
| R | agreement | ville de belfort | 1257548 | 22 | 349 | city_flagged#136 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | centre hospitalier de dieppe | 1191859 | 21 | 244 | city_flagged#137 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | conseil départemental des hauts-de-seine | 8135361 | 21 | 576 | city_flagged#138 | one département |
| R | agreement | in'li | 11816350 | 21 | 57 | city_flagged#139 | one company (La Défense / Puteaux) |
| N | name | mfla | 19230791 | 21 | 2 | city_flagged#140 | acronym; planned Saint-Herblain spellings only; 2-mention target |
| R | agreement | saint nazaire agglomeration - la carene | missing | 21 | 0 | city_flagged#141 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | tribunal administratif de martinique | 8084700 | 21 | 1,392 | city_flagged#142 | one tribunal (Schoelcher) |
| R | agreement | communaute d'agglomeration de la rochelle | 1637076 | 20 | 227 | city_flagged#143 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | communauté d'agglomération lisieux normandie | 9988956 | 20 | 42 | city_flagged#144 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | communauté de communes de la vallée du garon | 4360809 | 20 | 14 | city_flagged#145 | named body (Brignais); one Belleville mention is address noise |
| R | agreement | commune d'évry-courcouronnes | 13456036 | 20 | 5 | city_flagged#146 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | etablissement public foncier paca | 1224110 | 20 | 51 | city_flagged#147 | one body (Marseille) |
| R | agreement | haropa port \| paris | missing | 19 | 0 | city_flagged#149 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | communauté de communes du genevois | 1858706 | 18 | 68 | city_flagged#150 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | haropa port \| le havre | missing | 18 | 0 | city_flagged#151 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | tarn habitat | 1392468 | 18 | 344 | city_flagged#152 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | tribunal de grande instance d'annecy | 1578940 | 18 | 372 | city_flagged#153 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | agglomeration d'agen | 7128722 | 17 | 2 | city_flagged#154 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | casc | 1646259 | 17 | 17 | city_flagged#155 | one agglomeration (Sarreguemines) |
| R | agreement | grand chambéry | 15919766 | 17 | 6 | city_flagged#156 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| W | evidence | hotel de ville | 1228976 | 17 | 372 | city_flagged#157 | placeless; planned 8 communes |
| R | agreement | ville de lyon | 1163238 | 17 | 3,174 | city_flagged#158 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | camg | 9095469 | 16 | 4 | city_flagged#159 | CA Marne et Gondoire (Bussy-Saint-Martin) |
| R | agreement | centre hospitalier d&apos;agen-nerac | missing | 16 | 0 | city_flagged#160 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| W | evidence | conseil départemental | 6879005 | 16 | 57 | city_flagged#161 | placeless; target Arras, Toulouse, Tours; planned Aurillac, Cergy, Lille ... |
| R | agreement | département de seine-saint-denis | 1339272 | 16 | 22 | city_flagged#162 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| N | name | livie | 14630961 | 16 | 22 | city_flagged#163 | brand name; Levallois / Paris; cannot tell |
| R | agreement | t2c - régie des transports urbains de l'agglomération clermontoise | 3190860 | 16 | 10 | city_flagged#164 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | ville de pessac | 1394268 | 16 | 449 | city_flagged#165 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | ville de valence | 1175137 | 16 | 474 | city_flagged#166 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | centre hospitalier de l&apos;ouest guyanais | missing | 15 | 0 | city_flagged#167 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | cnfpt | 1163448 | 15 | 685 | city_flagged#168 | one national établissement (delegations are part of it) |
| R | agreement | commune d'istres | 12487329 | 15 | 10 | city_flagged#169 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | conseil départemental du cantal | 6980437 | 15 | 180 | city_flagged#170 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | haropa port le havre | missing | 15 | 0 | city_flagged#171 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | carcassonne agglo solidarité | 10948359 | 14 | 26 | city_flagged#172 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | cd57 | missing | 14 | 0 | city_flagged#173 | Moselle (Metz) |
| R | agreement | centre hospitalier de mont de marsan | 1367058 | 14 | 62 | city_flagged#174 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | collectivité européenne d'alsace | 16490606 | 14 | 449 | city_flagged#175 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| N | name | epcic | missing | 14 | 0 | city_flagged#176 | acronym; Bastia only; minted target |
| R | agreement | ports de normandie | 16436567 | 14 | 21 | city_flagged#177 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | rennes métropole | 1224470 | 14 | 915 | city_flagged#178 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | saint-louis agglomération | 9882863 | 14 | 68 | city_flagged#179 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | ville d'alençon | 1805646 | 14 | 101 | city_flagged#180 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | ville de villeurbanne | 1166637 | 14 | 593 | city_flagged#181 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | communauté d'agglomération seine-eure | 1225493 | 13 | 276 | city_flagged#183 | one body (Louviers) |
| R | agreement | commune de colmar | 17934513 | 13 | 16 | city_flagged#184 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | commune du tampon | 1195046 | 13 | 1,028 | city_flagged#185 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | cté d'agglomération de cergy pontoise | 22078850 | 13 | 4 | city_flagged#186 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| W | name | direction commande publique mutualisée | 22569106 | 13 | 2 | city_flagged#187 | generic department / service label (587 class): names a kind of unit every buyer has; Frontignan / Sète today |
| R | agreement | département du cher | missing | 13 | 0 | city_flagged#188 | one département (Bourges) |
| R | agreement | mairie d'agen | 1912221 | 13 | 33 | city_flagged#189 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | solendi expansion | 1187287 | 13 | 13 | city_flagged#190 | one company |
| R | agreement | apec | 1579843 | 12 | 123 | city_flagged#191 | one national body |
| R | agreement | communauté d'agglomération de cergy-pontoise | 9186711 | 12 | 61 | city_flagged#192 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | communauté d'agglomération du cotentin | 9638367 | 12 | 155 | city_flagged#193 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | communauté d'agglomération du grand villeneuvois | 1959164 | 12 | 33 | city_flagged#194 | one body (Villeneuve-sur-Lot) |
| R | agreement | conseil départemental d'ille et vilaine | 7032335 | 12 | 7 | city_flagged#195 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | département de l'ardèche | 1168144 | 12 | 698 | city_flagged#196 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | grand port fluvio-maritime de l'axe seine, direction territoriale du havre | missing | 12 | 0 | city_flagged#197 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | mairie de perpignan | 2387601 | 12 | 330 | city_flagged#198 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | mont de marsan agglomération | 7942386 | 12 | 9 | city_flagged#199 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | sdea alsace moselle | 5456808 | 12 | 436 | city_flagged#200 | one syndicate (Schiltigheim, Strasbourg suburb) |
| R | agreement | société canal de provence | 2891932 | 12 | 46 | city_flagged#201 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | sybert | 1228720 | 12 | 186 | city_flagged#202 | one syndicate (Besançon) |
| R | agreement | tribunal judiciaire de nice | 18153443 | 12 | 45 | city_flagged#203 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | ville de mont de marsan | 1181384 | 12 | 76 | city_flagged#204 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | ccas de rouen | 2488366 | 11 | 29 | city_flagged#205 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | cda de la rochelle | 8785854 | 11 | 7 | city_flagged#206 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | communaute d'agglomeration de marne et gondoire | 22566618 | 11 | 24 | city_flagged#207 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | communauté d'agglomération pays basque | 9856662 | 11 | 27 | city_flagged#208 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | département de la réunion | 1184070 | 11 | 517 | city_flagged#209 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | mairie de villemomble | 2046932 | 11 | 47 | city_flagged#210 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | promologis | 1240163 | 11 | 414 | city_flagged#211 | one ESH (Toulouse) |
| R | agreement | sdec energie | 2858484 | 11 | 32 | city_flagged#212 | one syndicate (Caen) |
| W | evidence | service des marchés | 1363144 | 11 | 83 | city_flagged#213 | generic department / service label (587 class): names a kind of unit every buyer has; Aurillac, Montpellier, Six-Fours |
| R | agreement | alceane | 9197802 | 10 | 21 | city_flagged#214 | one OPH (Le Havre) |
| R | agreement | cap atlantique | 1302420 | 10 | 183 | city_flagged#215 | one body (La Baule) |
| R | agreement | cd34 | missing | 10 | 0 | city_flagged#216 | Hérault (Montpellier; Gignac site) |
| R | agreement | communauté d'agglomération de versailles grand parc | 2439140 | 10 | 15 | city_flagged#218 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| N | name | dcamg | missing | 10 | 0 | city_flagged#219 | acronym; Valence only; minted target |
| R | agreement | hôtel de ville de romans-sur-isère | missing | 10 | 0 | city_flagged#220 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | mairie de vallauris golfe-juan | 22681397 | 10 | 5 | city_flagged#221 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| N | name | plateforme aws | 17943570 | 10 | 2 | city_flagged#222 | the platform (AWS-Achat); the planned cities are the buyers' |
| W | evidence | pour obtenir tous renseignements complémentaires qui leur seraient nécessaires au cours de leur étude, les can | missing | 10 | 0 | city_flagged#223 | boilerplate sentence, not a body; a minted row would pool Dijon and Fontaine-lès-Dijon buyers (better refused as a non-name by 586/587) |
| R | agreement | smdea | 1428476 | 10 | 120 | city_flagged#224 | one syndicate (Saint-Paul-de-Jarrat) |
| R | agreement | valence romans habitat | 9376641 | 10 | 182 | city_flagged#225 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | ville de louviers | 1721083 | 10 | 67 | city_flagged#226 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| N | name | abri | missing | 9 | 0 | city_flagged#227 | short name; Livry-Gargan only; minted target |
| R | agreement | caen la mer | 22387389 | 9 | 2 | city_flagged#228 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | communauté d'agglomération riom limagne et volcans | 11452016 | 9 | 15 | city_flagged#229 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | communauté de communes porte de drômardèche | 9939007 | 9 | 27 | city_flagged#230 | one body |
| R | agreement | conseil départemental d'eure-et-loir | 9309479 | 9 | 7 | city_flagged#231 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | département de la dordogne | missing | 9 | 0 | city_flagged#232 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | département de la seine-saint-denis | 1418521 | 9 | 37 | city_flagged#233 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| N | name | epci de corse | missing | 9 | 0 | city_flagged#234 | label for any Corsican EPCI; Bastia only today; minted target |
| R | agreement | haropa port | missing | 9 | 0 | city_flagged#235 | one établissement since 2021 (Le Havre, Rouen, Paris) |
| R | agreement | spl les eaux du sage | 20949940 | 9 | 2 | city_flagged#236 | one SPL (Roques) |
| R | agreement | arac occitanie | 20666337 | 8 | 3 | city_flagged#237 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | cd93 | missing | 8 | 0 | city_flagged#238 | Seine-Saint-Denis (Bobigny) |
| R | agreement | communauté d'agglomération cannes pays de lérins | 8999044 | 8 | 41 | city_flagged#240 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | communauté d'agglomération du puy-en-velay | 1215418 | 8 | 215 | city_flagged#241 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| W | evidence | commune | 1228180 | 8 | 332 | city_flagged#242 | placeless; planned 6 communes |
| W | evidence | direction des services techniques | 1251437 | 8 | 119 | city_flagged#243 | generic department / service label (587 class): names a kind of unit every buyer has; Rognac + Val-d'Isère |
| R | agreement | epms la vergnière | missing | 8 | 0 | city_flagged#244 | one establishment (Foix) |
| W | evidence | hotel du département | 8262162 | 8 | 8 | city_flagged#245 | placeless; target Albi + Nanterre; planned Bobigny, La Roche-sur-Yon |
| R | agreement | mairie de corte | 2503448 | 8 | 4 | city_flagged#246 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | mairie de valence | 1499784 | 8 | 101 | city_flagged#247 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | norevie | 2191990 | 8 | 141 | city_flagged#248 | one ESH (Douai) |
| R | agreement | sdis de la mayenne | 1233115 | 8 | 69 | city_flagged#249 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| W | evidence | semag | 1279835 | 8 | 203 | city_flagged#250 | GET: target 1279835 already holds Gardanne, Abymes (Guadeloupe) and Gennevilliers buyers; planned Les Abymes |
| W | evidence | service achats | 1163276 | 8 | 106 | city_flagged#251 | generic department / service label (587 class): names a kind of unit every buyer has; target Sarcelles + Valence; planned Les Abymes |
| R | agreement | service départemental incendie et secours 66 (sdis 66) | 21997876 | 8 | 1 | city_flagged#252 | SDIS 66 (Perpignan) |
| R | agreement | batigere developpement | 17604765 | 7 | 9 | city_flagged#253 | one company (Levallois) |
| W | evidence | c.c.a.s | 3574992 | 7 | 16 | city_flagged#254 | folds to ccas; target Saint-Gervais, planned Grenoble |
| R | agreement | caisse des ecoles de levallois | missing | 7 | 0 | city_flagged#255 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| W | evidence | communauté urbaine | 2102005 | 7 | 2 | city_flagged#256 | placeless; planned Caen + Le Havre |
| R | agreement | commune de mougins | 4643989 | 7 | 11 | city_flagged#257 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| N | name | dajap | 11493242 | 7 | 6 | city_flagged#258 | acronym; Lille only |
| R | agreement | departement de la sarthe | 19344860 | 7 | 11 | city_flagged#259 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | ideis | 3966668 | 7 | 19 | city_flagged#260 | one body (Annecy) |
| N | name | noalis | 14020207 | 7 | 14 | city_flagged#261 | one ESH most likely; Angoulême vs Limoges cannot be told from the report |
| R | agreement | presqu'île habitat | 1166385 | 7 | 134 | city_flagged#262 | one OPH (Cherbourg) |
| R | agreement | saint-nazaire agglomeration - la carene | missing | 7 | 0 | city_flagged#263 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | sccv le clos tolosan | 22187588 | 7 | 1 | city_flagged#264 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | sdis cantal | 12190707 | 7 | 7 | city_flagged#265 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| W | evidence | service marchés | 1179591 | 7 | 404 | city_flagged#266 | generic department / service label (587 class): names a kind of unit every buyer has; target Quimper, planned Brignais, Lyon, Marseille |
| W | evidence | service marchés publics | 1163277 | 7 | 493 | city_flagged#267 | generic department / service label (587 class): names a kind of unit every buyer has; 9 towns |
| R | agreement | thionville fensch agglomération | missing | 7 | 0 | city_flagged#268 | one body (Yutz) |
| R | agreement | ville de narbonne | 8176971 | 7 | 179 | city_flagged#269 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | ville de vannes | 1233174 | 7 | 561 | city_flagged#270 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | capbp | missing | 6 | 0 | city_flagged#271 | CA Pau Béarn Pyrénées |
| W | evidence | ccira | 1222149 | 6 | 2,941 | city_flagged#272 | bare acronym of every committee; target 1222149 (2,941 mentions) city Nantes, planned Nancy |
| W | evidence | centre communal d'action sociale (ccas) | 1896679 | 6 | 138 | city_flagged#273 | placeless; planned Arles + Noyon |
| R | agreement | centre hospitalier de blois | 1223543 | 6 | 618 | city_flagged#274 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | centre hospitalier intercommunal aix-pertuis | 17724482 | 6 | 2 | city_flagged#275 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | chu de la reunion-direction des travaux et des services techniques | missing | 6 | 0 | city_flagged#276 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | communauté de communes entre bièvre et rhône | 21009898 | 6 | 6 | city_flagged#277 | one body |
| R | agreement | conseil departemental de la creuse | missing | 6 | 0 | city_flagged#278 | one département (Guéret) |
| R | agreement | conseil département du nord | missing | 6 | 0 | city_flagged#279 | one département (Lille) |
| R | agreement | departement du lot | 11493892 | 6 | 324 | city_flagged#280 | one département (Cahors) |
| W | name | direction de l'achat public | 9224696 | 6 | 2 | city_flagged#281 | generic department / service label (587 class): names a kind of unit every buyer has; Sainte-Clotilde only today |
| R | agreement | département de le haute-savoie | 1494174 | 6 | 36 | city_flagged#282 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | fondation de madame amicie lebaudy | missing | 6 | 0 | city_flagged#283 | one foundation |
| R | agreement | gcs achats du centre | 1790002 | 6 | 208 | city_flagged#284 | one GCS (Blois) |
| R | agreement | la région grand est | 9138318 | 6 | 12 | city_flagged#285 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | le centre communal d'action sociale de vendôme | missing | 6 | 0 | city_flagged#286 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | logelia | 22695348 | 6 | 1 | city_flagged#287 | one ESH (Angoulême) |
| R | agreement | mairie de la plagne tarentaise | 8502134 | 6 | 2 | city_flagged#288 | name names the commune; Aime-la-Plagne city on the 2-mention target is address noise |
| R | agreement | mairie de marignane | 7255776 | 6 | 39 | city_flagged#289 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | mairie de thuir | missing | 6 | 0 | city_flagged#290 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | paris saclay cancer cluster | missing | 6 | 0 | city_flagged#291 | one body |
| R | agreement | région grand-est | 9141905 | 6 | 11 | city_flagged#292 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | sdis de l'ardèche | 1233047 | 6 | 94 | city_flagged#293 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| W | evidence | service achats/marchés | 31975711 | 6 | 2 | city_flagged#294 | generic department / service label (587 class): names a kind of unit every buyer has; Avoine + Mitry-Mory |
| W | evidence | service juridique et commande publique | 1419596 | 6 | 21 | city_flagged#295 | generic department / service label (587 class): names a kind of unit every buyer has; Le Mans + Publier |
| R | agreement | ville de poitiers | 1204130 | 6 | 522 | city_flagged#296 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | centre hospitalier du centre bretagne | 1986123 | 5 | 92 | city_flagged#297 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | communauté de communes entre juine et renarde | 11354021 | 5 | 7 | city_flagged#298 | one body (Étréchy) |
| R | agreement | communauté de communes marche et combraille en aquitaine | 23363705 | 5 | 1 | city_flagged#299 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | communauté de communes rumilly terre de savoie | 12096604 | 5 | 8 | city_flagged#300 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | commune de saint-denis de la réunion 26 rue lislet geoffroy | missing | 5 | 0 | city_flagged#301 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | fonds de garantie des assurances obligatoires de dommages | 15341181 | 5 | 3 | city_flagged#302 | one national fund (Vincennes) |
| R | agreement | greenalp | 13370846 | 5 | 21 | city_flagged#303 | one company (Grenoble) |
| R | agreement | habitat 44 | 1173370 | 5 | 82 | city_flagged#304 | one OPH (Loire-Atlantique) |
| R | agreement | logelia charente | 1906946 | 5 | 100 | city_flagged#305 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | mairie de la baule-escoublac | 21614995 | 5 | 1 | city_flagged#306 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | mairie de voiron | 21193177 | 5 | 8 | city_flagged#307 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | mairie du puy-en-velay | 13293082 | 5 | 4 | city_flagged#308 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | métropole de lyon - délégation ressources humaines et moyens généraux - direction ressources | missing | 5 | 0 | city_flagged#309 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | oph31 | 2097247 | 5 | 12 | city_flagged#310 | one OPH (Balma) |
| R | agreement | résidence autonomie les floralies | missing | 5 | 0 | city_flagged#311 | one residence (Le Pontet) |
| R | agreement | savoisienne habitat | 1958768 | 5 | 46 | city_flagged#312 | one ESH (Chambéry) |
| R | agreement | sdis53 | 18203293 | 5 | 2 | city_flagged#313 | SDIS Mayenne |
| R | agreement | sem4v | 3538757 | 5 | 6 | city_flagged#314 | one SEM (Albertville) |
| R | agreement | spl hauts-de-france amenagement | missing | 5 | 0 | city_flagged#315 | one SPL |
| R | agreement | tribunal judiciaire d'evry | 17578241 | 5 | 34 | city_flagged#316 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | vichy communauté | 9187999 | 5 | 178 | city_flagged#317 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | arche agglo | 13180030 | 4 | 55 | city_flagged#318 | one body |
| R | agreement | ccas de castanet-tolosan | 3719682 | 4 | 10 | city_flagged#319 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | ccas de la ville de limoges | 4952451 | 4 | 12 | city_flagged#320 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | centre hospitalier rochefort | 2409878 | 4 | 1 | city_flagged#321 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | commune de belleville-en-beaujolais | 20078845 | 4 | 2 | city_flagged#322 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | dalkia | 1176034 | 4 | 2,173 | city_flagged#323 | one company; its target cities are its sites |
| R | agreement | departement de la charente | 2257611 | 4 | 42 | city_flagged#324 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | département de la seine saint denis | 11106374 | 4 | 5 | city_flagged#325 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | emmaus habitat | 1309535 | 4 | 102 | city_flagged#326 | one ESH (Clichy) |
| N | name | gcspa | 20636467 | 4 | 3 | city_flagged#327 | acronym; Aix-en-Provence area only |
| R | agreement | gip un ete au havre | missing | 4 | 0 | city_flagged#328 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | hôtel de la communauté urbaine le havre seine métropole | 17469021 | 4 | 2 | city_flagged#329 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| N | name | jaxed | missing | 4 | 0 | city_flagged#330 | short name; Angers area only; minted target |
| R | agreement | mairie de castanet-tolosan | 4762340 | 4 | 29 | city_flagged#331 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | mairie de crolles | 31137104 | 4 | 9 | city_flagged#332 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| W | evidence | mairie de saint denis | 31429325 | 4 | 1 | city_flagged#333 | homonym: GET name_prefix shows both Saint-Denis (93) and Saint-Denis (Réunion) publishing as 'Mairie de Saint-Denis'; planned are Réunion (Sainte-Clotilde) |
| R | agreement | midi habitat groupe | 9797718 | 4 | 1 | city_flagged#334 | one group |
| R | agreement | métropole de lyon - drhmg - unité marchés | missing | 4 | 0 | city_flagged#335 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | pays de montbéliard agglomération | 1278751 | 4 | 122 | city_flagged#336 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | sdeg 16 | 2877841 | 4 | 5 | city_flagged#337 | one syndicate (Angoulême) |
| W | evidence | service des marchés publics | 1181455 | 4 | 852 | city_flagged#338 | generic department / service label (587 class): names a kind of unit every buyer has; Noisy-le-Sec, Saint-Raphaël, Six-Fours |
| W | evidence | service départemental d'incendie et de secours | 1299400 | 4 | 318 | city_flagged#339 | placeless; GET: target 1299400 is SDIS 71 (Sancé); planned Cahors + Digne-les-Bains |
| W | evidence | service marches publics | 1298079 | 4 | 15 | city_flagged#340 | generic department / service label (587 class): names a kind of unit every buyer has; Brive, La Grande-Motte, Saint-Laurent-Blangy |
| R | agreement | ville d'agen | 1384134 | 4 | 82 | city_flagged#341 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | ville de châteaudun | 1237760 | 4 | 32 | city_flagged#342 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | ville de firminy | 1244100 | 4 | 86 | city_flagged#343 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | ville de lattes | 22738346 | 4 | 2 | city_flagged#344 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | bordeaux metropole pto | missing | 3 | 0 | city_flagged#345 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| W | evidence | ccsb | missing | 3 | 0 | city_flagged#346 | acronym of two bodies: Belleville-en-Beaujolais (CC Saône-Beaujolais) and Lazer (CC Sisteronais-Buëch) |
| R | agreement | centre hospitalier de haguenau | 1201604 | 3 | 325 | city_flagged#347 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | centre hospitalier de sarrebourg | 2081492 | 3 | 66 | city_flagged#348 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| W | evidence | centre intercommunal d'action sociale | 1244397 | 3 | 130 | city_flagged#349 | placeless; target Saint-Maixent-l'École, planned Alençon |
| W | evidence | collège george sand | 3490748 | 3 | 18 | city_flagged#350 | GET: target 3490748 already holds FR82 (PACA) and FRI34 tenders; planned Avord + Toulon: two colleges |
| W | evidence | commande publique | 1186120 | 3 | 163 | city_flagged#351 | generic department / service label (587 class): names a kind of unit every buyer has; target Buxerolles, Les Mureaux, Saint-André; planned Dijon, Sainte-Clotilde |
| R | agreement | communauté de communes albères côte vermeille illibéris | 16343857 | 3 | 5 | city_flagged#352 | one body |
| R | agreement | commune d'ancenis-saint-géréon | 20086685 | 3 | 4 | city_flagged#353 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | commune d'eybens | 2088342 | 3 | 13 | city_flagged#354 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | conseil departemental de la haute-loire | missing | 3 | 0 | city_flagged#355 | one département |
| N | name | dajap - pap | missing | 3 | 0 | city_flagged#356 | acronym; Lille only; minted target |
| R | agreement | epdahaa | 6328439 | 3 | 113 | city_flagged#357 | one body (Arras) |
| R | agreement | ght d'eure et loir | 21379406 | 3 | 2 | city_flagged#358 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| N | name | gpfmas | missing | 3 | 0 | city_flagged#359 | acronym; Rouen area only; minted target |
| R | agreement | grand auch coeur de gascogne | 9188753 | 3 | 33 | city_flagged#360 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | grand besancon metropole | 17580465 | 3 | 18 | city_flagged#361 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | hlm solidarite et logement | missing | 3 | 0 | city_flagged#362 | one body (Meudon) |
| N | name | https://marches.megalis.bretagne.bzh | missing | 3 | 0 | city_flagged#363 | platform URL as a name (Mégalis Bretagne) |
| R | agreement | mairie de beziers | 19203575 | 3 | 10 | city_flagged#364 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | mairie de saint jean de la ruelle | 17953119 | 3 | 3 | city_flagged#365 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | mairie de toulouse | 1675905 | 3 | 2,058 | city_flagged#366 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | mairie de voreppe | 30922265 | 3 | 19 | city_flagged#367 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | mairie des lilas | 13843709 | 3 | 17 | city_flagged#368 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | méribel tourisme | 19262445 | 3 | 8 | city_flagged#369 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | opera de limoges | missing | 3 | 0 | city_flagged#370 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | ophis du puy de dome | 31287264 | 3 | 52 | city_flagged#371 | one OPH |
| R | agreement | roannaise de l'eau | 1226304 | 3 | 60 | city_flagged#372 | one body (Roanne) |
| R | agreement | régie départementale du train du montenvers | 30921298 | 3 | 7 | city_flagged#373 | one régie (Chamonix) |
| R | agreement | scic de la haute garonne | 22187585 | 3 | 1 | city_flagged#374 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | sdis du loiret | 1213346 | 3 | 120 | city_flagged#375 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| W | evidence | service technique de la ville | missing | 3 | 0 | city_flagged#376 | generic department / service label (587 class): names a kind of unit every buyer has; Blanquefort + Le Taillan-Médoc |
| R | agreement | val touraine habitat | 1173221 | 3 | 209 | city_flagged#377 | one OPH (Tours) |
| R | agreement | ville de la rochelle | 1186789 | 3 | 329 | city_flagged#378 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | ville de voiron | 1895 | 3 | 35 | city_flagged#379 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| W | evidence | aws | 1400495 | 2 | 13 | city_flagged#380 | acronym; target city Poitiers, planned Le Tholonet (Société du Canal de Provence's address) |
| W | evidence | cdc | 1549719 | 2 | 10 | city_flagged#381 | acronym; GET: target 1549719 is a Donzy (Nièvre) buyer; planned Corse |
| W | evidence | centre de gestion de la fonction publique territoriale | 2560557 | 2 | 26 | city_flagged#382 | GET: target 2560557 already holds Saint-Grégoire, Belfort, Aix, Luisant, Pau, Gap CDGs; planned Annecy |
| R | agreement | cirad | 1228237 | 2 | 86 | city_flagged#383 | one national body |
| R | agreement | communauté d'agglomération de la région dieppoise | 1433484 | 2 | 39 | city_flagged#384 | one body |
| R | agreement | communauté de communes cluses arve et montagnes | 4412488 | 2 | 29 | city_flagged#385 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | communauté de communes du frontonnais | 4195795 | 2 | 21 | city_flagged#386 | one body |
| R | agreement | communauté de communes pyrénées haut garonnaises | missing | 2 | 0 | city_flagged#387 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | communauté de communes saône beaujolais | 6443462 | 2 | 22 | city_flagged#388 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | commune de bourg saint maurice | missing | 2 | 0 | city_flagged#389 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | commune des allues | 22564282 | 2 | 20 | city_flagged#390 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| W | evidence | commune et ccas | 31140454 | 2 | 1 | city_flagged#391 | placeless; Brie-Comte-Robert + Chevry-Cossigny: two communes |
| R | agreement | departement de la haute-loire | 10117206 | 2 | 38 | city_flagged#392 | one département |
| W | evidence | direction commande publique | 3426182 | 2 | 114 | city_flagged#393 | generic department / service label (587 class): names a kind of unit every buyer has; target Saint-Malo, planned Avignon |
| R | agreement | direction de l'achat public de la région grand est | 22657503 | 2 | 1 | city_flagged#394 | Région Grand Est (Metz / Châlons seats) |
| R | agreement | département de la drôme | 2358745 | 2 | 82 | city_flagged#395 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| W | evidence | ehpad | 1338203 | 2 | 277 | city_flagged#396 | GET: target 1338203 already holds EHPADs of Moutiers-Saint-Jean, Effiat, Hondschoote, Lezay ...; planned Ajain + Boussac |
| R | agreement | epfl 73 | missing | 2 | 0 | city_flagged#397 | one EPF (Savoie) |
| R | agreement | eppdcsi - universcience | missing | 2 | 0 | city_flagged#398 | one établissement |
| R | agreement | fondation amicie lebaudy | missing | 2 | 0 | city_flagged#399 | one foundation |
| R | agreement | greffe du tribunal administratif de bordeaux | 1165328 | 2 | 2,583 | city_flagged#400 | one tribunal; target and planned cities agree up to spelling / cedex |
| R | agreement | grenoble alpes métropole | 1163592 | 2 | 606 | city_flagged#401 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | logélia | missing | 2 | 0 | city_flagged#402 | one ESH (Angoulême) |
| R | agreement | mairie d'aix-en-provence | 1394721 | 2 | 86 | city_flagged#403 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| N | name | mairie de saint- gervais | 22680256 | 2 | 1 | city_flagged#404 | several communes are Saint-Gervais; target and planned both read Saint-Gervais(-les-Bains); 2 mentions |
| R | agreement | montlucon communaute | 22710360 | 2 | 1 | city_flagged#405 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | odarc - etablissement public à caractère industriel et commercial | missing | 2 | 0 | city_flagged#406 | one office (Corsica) |
| R | agreement | office de tourisme de marne et gondoire | 15638333 | 2 | 4 | city_flagged#407 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | orange | 1194048 | 2 | 1,372 | city_flagged#408 | one company |
| W | evidence | restaurant scolaire | 3457028 | 2 | 3 | city_flagged#409 | placeless; Commentry + Luc-sur-Mer |
| R | agreement | sdis 45 loiret | 22634584 | 2 | 1 | city_flagged#410 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| N | name | service commande publique et achat responsable | missing | 2 | 0 | city_flagged#411 | generic department / service label (587 class): names a kind of unit every buyer has; one body (Métropole de Lyon) today and a missing target, so the minted row would hold one buyer |
| R | agreement | syndicat mixte ouvert - seine et yvelines voirie | missing | 2 | 0 | city_flagged#412 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | ville de mauguio | 1174274 | 2 | 27 | city_flagged#413 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | ville de romans-sur-isère | 1263986 | 2 | 72 | city_flagged#414 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| W | evidence | centre communal d action sociale | 8939560 | 1 | 54 | city_flagged#415 | CCAS variant; target Coutances, planned Saint-Étienne |
| W | evidence | centre communal d' action sociale | 2817853 | 1 | 4 | city_flagged#416 | CCAS variant; target Carentan, planned Limoges |
| W | evidence | centre technique municipal | 1179286 | 1 | 107 | city_flagged#417 | placeless; target Les Gets, planned Arles |
| W | evidence | comité consultatif de règlement amiable des litiges relatifs aux marchés publics | 1201220 | 1 | 1,653 | city_flagged#418 | placeless committee name; target city Paris, planned Nantes |
| R | agreement | conseil départemental du val d'oise | 7960288 | 1 | 35 | city_flagged#419 | one département |
| R | agreement | ept paris terres d'envol | 22577992 | 1 | 1 | city_flagged#420 | one EPT (Aulnay) |
| R | agreement | groupe hospitalier bretagne sud | 11018308 | 1 | 145 | city_flagged#421 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| W | evidence | groupement de commande | 3416016 | 1 | 21 | city_flagged#422 | not a body; target Prades, planned Aix-les-Bains |
| R | agreement | habitat76 | 1488606 | 1 | 43 | city_flagged#423 | one OPH (Rouen) |
| R | agreement | habitats de haute-alsace | 1173236 | 1 | 188 | city_flagged#424 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| N | name | http://agysoft.marches-publics.info | 13197923 | 1 | 2 | city_flagged#425 | platform URL as a name; the cities are buyers' |
| R | agreement | leman habitat | 10299286 | 1 | 6 | city_flagged#426 | one OPH (Thonon) |
| R | agreement | mairie de saverdun | 30923319 | 1 | 4 | city_flagged#427 | name names Saverdun; the one Pamiers city is address noise |
| W | evidence | profil acheteur | 1728706 | 1 | 39 | city_flagged#428 | not a body; target Audun-le-Tiche + Dreux, planned Marseille |
| R | agreement | sa hlm irp | 1249611 | 1 | 70 | city_flagged#429 | one ESH (Meudon) |
| R | agreement | safer reunion | 16333730 | 1 | 2 | city_flagged#430 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
| R | agreement | sccv chartreuse de parme | 30923207 | 1 | 1 | city_flagged#431 | one SCCV |
| W | evidence | sce departemental incendie et secours | 9485683 | 1 | 239 | city_flagged#432 | placeless SDIS; target Vailhauquès (SDIS 34), planned Saint-Brieuc (SDIS 22) |
| R | agreement | société d'habitation des alpes | 1180987 | 1 | 119 | city_flagged#433 | one ESH (Voiron) |
| R | agreement | syndicat mixte des eaux de la bombarde | 22897725 | 1 | 3 | city_flagged#434 | one syndicate |
| W | evidence | tribunal de grande instance | 1163205 | 1 | 20,431 | city_flagged#435 | bare name of every TGI; target Boulogne-sur-Mer + Rennes, planned Chambéry |
| R | agreement | ville de granville | 1532854 | 1 | 127 | city_flagged#436 | name carries its place; target and planned cities agree up to spelling / cedex / member commune |
