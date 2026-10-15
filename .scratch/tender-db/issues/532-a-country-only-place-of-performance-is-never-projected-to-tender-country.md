# 532 — a place of performance published as a country alone never projects, so the tender serves `country: []` and `?country=` misses it

Status: ready-for-agent — unit 1 DECIDED 2026-10-15: scheme `nuts`, field `place_country`. The code is the NUTS level-0 code, taken from an exact ISO alpha-3 code and served for the 37 NUTS countries only (GRC → `EL`, GBR → `UK`); third countries get nothing. In `fold`, after the carry, every derived row is dropped from any version that has another `nuts` row. Next: unit 2, build the derivation (M) per the build spec below: the `place_country` function, the arm in `NoticeState::read`, the drop in `fold`, the 5 ids plus 2 DE-1.x aliases in `has_destination`'s Code arm, the docs and the CHANGELOG. Then run `ops/check.sh`.
Kind: coverage (projection): a modelled concept with an unmapped spelling, the 18/85/177/231/368 shape. The fix is a
Code-branch derivation in the shape of 397's `contract_nature`, not a `CLASSIFICATIONS` row.
Relates to:
- 368 (DONE 2026-09-18): its section-13 unmapped-field listing (line 228) shows `BT-5141-Lot`, 36,931 window rows
  (sdk-1.13), grouped with postal-address parts as "correctly out of scope". That was a blanket call; BT-5141 was
  never one of the hand-read fields. This issue reverses it for BT-5141.
- 48 (RESOLVED 2026-09-01): `?country=` is a NUTS place-code prefix (`DE`, not `DEU`; docs.rs:167). That is why a
  NUTS level-0 code is the right destination. It never covered country-only places.
- 319 (DONE 2026-09-29): the org-side alpha-3 to alpha-2 fold. Its table is reusable, but its EL→GR direction is the
  opposite of NUTS. Org layer only.
- 397 (RESOLVED-VERIFIED 2026-09-28) / 479 (done 2026-10-09): the precedent for turning a Code into a
  `Fact::Classification` (`contract_nature`, project.rs:5276-5298, :7645). This fix takes that shape.
- 29 / 231 (VERIFIED 2026-08-18 / CLOSED 2026-08-27): mapped DÖE sdk-0.1's RealizedLocation NUTS subentity to
  `place`, never its country-only spelling.
- 437 (RESOLVED-VERIFIED 2026-09-28): FTS award-item region. Its "some UK rows publish no region anywhere" is source
  reality. It did not touch a delivery address that carries a country and no region.
- 336 (CLOSED NOT-WORTH-IT 2026-09-01): an unmatchable filter value reads as "no results". Adjacent only: here the
  value matches and the data is missing.

## Verify

    curl -s --max-time 20 "https://tenders.zebreus.click/v1/tenders?country=LT&publication_id=00493114-2024" | python3 -c "import json,sys; d=json.load(sys.stdin); print([(i['id'], i['country']) for i in d['items']])"

- **done**: `[(1058757, ['LT'])]`. Needs units 2 and 3 (the derivation, then a refold of notice 24132122).
- **open (2026-10-14)**: `[]`.

## Repro

Public GETs, 2026-10-14:
- The Verify request without `country=` gives `[(1058757, [])]`. With `country=LT` the response's `ignored_filters`
  is `[]`, so the filter was applied: the tender exists, and the filter has nothing to match.
- `curl -s https://tenders.zebreus.click/v1/tenders/<id> | jq -c .country` gives `[]` for 288572, 258568, 1365142
  and 357950. The DÖE witness for unit 3 is 1365142 (done: `["DE"]`).
- Legacy contrast: 8432519 (104355-2019) gives `["FRI12","FRK22"]` (see "Not this issue").

## What was found

The tender's `country` is the version's `nuts`-scheme classifications: the `?country=` filter is an EXISTS on
`tender_version_classifications` with `scheme = 'nuts' AND code LIKE '<prefix>%'` (read.rs:1077-1083), and the list
echo is the same rows (read.rs:2159-2164). Both key on the scheme only, not on the field.

The projection maps only NUTS spellings to `place` (`CLASSIFICATIONS`, project.rs:352-374): BT-5071, TED `*_NUTS`,
TXT-RC, and the SDK01/DE1 `...RealizedLocation-Address-CountrySubentityCode` (DE1 alias at project.rs:1301). A
notice may state the place of performance as a country alone:
- eForms: `BT-5141-Lot` / `BT-5141-Procedure` (code list `country`, alpha-3);
- DÖE sdk-0.1: `SDK01-ProcurementProject[Lot-ProcurementProject]-RealizedLocation-Address-Country-IdentificationCode`.

Both are parsed and stored as `NoticeValue::Code` rows in `notice_codes` (eforms/value.rs: only the cpv/nuts lists
become classifications). Nothing reads them: `grep 5141 crates/ingest/src/project.rs` gives 0 hits, and the only
`Country-IdentificationCode` ids mapped are the org-address ones (project.rs:1189-1190, :1328). `CLASSIFICATIONS` is
consulted only inside `if let NoticeValue::Classification` (project.rs:5304), so adding these ids there would map
nothing.

Witnesses, all serving `country: []` (verifier and refute re-read each notice):

| tender | publication | notice | place published | NUTS rows on the notice |
| --- | --- | --- | --- | --- |
| 1058757 | 00493114-2024 | 24132122 | BT-5141-Lot/-Procedure LTU | org BT-507 LT026, NO081 only |
| 288572 | 00033085-2026 | 25120713, 25475433 | BT-5141-Lot/-Procedure NLD | org BT-507 NL350, NL224 only |
| 258568 | 00464503-2024 | 23784789 | BT-5141-Lot/-Procedure NLD | org BT-507 NL329 only |
| 1365142 | doe 20758446-1 | 26168799 (sdk-0.1) | both SDK01 RealizedLocation Country ids DEU | none |
| 357950 | doe | 26096037 (sdk-1.0) | BT-5141-Lot DEU | none (no BT-5071) |

Size, from bounded notice-id windows (one window each; not corpus figures):
- **24,500,000–24,500,999** (TED eForms sdk-1.7..1.12): 849 publish BT-5141, 791 BT-5071. **68 (6.8%) are
  country-only**: NLD 16, CZE 13, FRA 5, FIN 5, NOR 4, IRL 4, BEL 3, AUT 3, …, 1 EGY.
- **25,470,000–25,470,999** (TED sdk-1.12 226 / sdk-1.13 774): 938 BT-5141, 879 BT-5071, 0 BT-5071 without BT-5141.
  **59 (5.9%) country-only** (NLD 9, CZE 8, DEU 8, SVK 8, AUT 7, …); 40 of the 59 are CAN-family subtypes. 62 publish
  no place at all.
- **26,100,000–26,100,999** (DÖE: 992 sdk-0.1, 8 sdk-1.0): 862 publish a place country, 855 a place NUTS. **87
  (8.7%) country-only**, all DEU (83 sdk-0.1, 4 sdk-1.0). 58 publish neither.
- **Tender level, one TED day** (2026-01-14, first 200 rows by `published_at`): 31 (15.5%) serve `country: []`. Of
  the 13 whose notice falls in the 25.47M window, 5 are country-only (AUT, AUT, DEU, NLD, FRA) and 8 publish no place
  (5 PINs, 2 X02, 1 CAN). None of the country-only five had a NUTS `country` carried from an earlier version.

**Not this issue: tenders whose notices publish no place at all.** By notice that is 62 vs 59 country-only for TED and
58 vs 87 for DÖE, so this defect is **roughly 40–60% of the eForms/DÖE empties**, not a small residue. The rest have
only a buyer address (TED 7874408, r209: org TED-COUNTRY plus notice-level TED-ISO_COUNTRY FR; TED 8814656: buyer
BT-514 BGR; DÖE 1302675, 1015385; FTS 8690102 and 8721263: no `deliveryAddresses`, only the buyer's). Serving the
buyer's country there would be a fallback, and that is its own decision.

**Untracked precedent the decision must know about:** the legacy TED projection already serves buyer and winner
address NUTS as the place. `TED-CA_CE_NUTS`, `TED-TENDERER_NUTS` and `TED-NUTS` (including inside ORG sections) all map
to `place` (project.rs:361-366, r209/rules.rs:390). Tender 8432519 serves `["FRI12","FRK22"]`: FRI12 is the buyer's
(ORG-1 TED-NUTS, PROCEDURE TED-CA_CE_NUTS), and only FRK22 is TED-PERFORMANCE_NUTS. So `country` already means
different things in different eras, which partly explains why legacy TED looks fuller than eForms, and `?country=`
over-matches legacy TED. The board has 0 hits for CA_CE_NUTS or TENDERER_NUTS. Not built here; file it separately if
unit 1 wants it settled.

**FTS is different.** fts/parse.rs:590-591 skips a delivery address with no `region` before anything is stored.
`Address.country` is deserialized (parse.rs:944-947) but never emitted, and fts/checklist.rs:82/152 marks
`deliveryAddresses[].country` Ignored on the premise "the NUTS code implies it", which fails when `region` is absent.
So FTS needs a parser change and an FTS re-parse, not a refold. Unmeasured: 0 of the 2 FTS empties sampled had any
delivery address.

### What the external review got wrong

- "Countries get dropped" is half right. For eForms and DÖE the country is kept at the parse layer (`notice_codes`);
  only the projection has nowhere to put it, so the fix is a projection change plus a refold, with no re-parse.
- It is not the API contradicting its docs. `country` and `?country=` are documented as NUTS place codes (docs.rs:167,
  issue 48). This is a coverage gap in the projection.
- "17% of 2023+ tenders have no country" is unmeasured (a corpus count has no compliant path from the agent). An
  8-day, 4,000-row cluster sample gave 11.3% (DÖE 13.4%, FTS 11.7%, TED 3.6%), but its TED rate is unrepresentative:
  one TED eForms day gave 15.5%. So 17% is plausible and still unverified. Either way the empties mix two causes,
  and only about 40–60% of the eForms/DÖE empties are this defect.
- The implied FTS ("UK") case is not shown. FTS does drop a region-less delivery address at parse, but no sampled
  FTS empty had one.
- An ISO alpha-2 mapping would be wrong for Greece and the UK. The stored code must be the NUTS level-0 spelling:
  GRC → `EL`, GBR → `UK` (docs.rs:167 already says `GB` matches nothing).
- Do not re-file the first-pass fix "add BT-5141 to `CLASSIFICATIONS`": BT-5141 is a Code, and that table is read
  only for Classification values. Do not re-file "store it as `field='place'`" either (see unit 1).

## Decision (unit 1, 2026-10-15)

Decided by a judge panel (independent proposals from distinct angles, then a skeptical judge that re-checked every disputed code claim against the tree at f3d5d06).

I compared two independent proposals and re-checked every code claim they disagree on against the tree (HEAD f3d5d06). Where they differ, the pick and a one-line reason are given under each point. Line numbers are current as of 2026-10-15; the issue's drifted refs are corrected at the end.

### D1. Destination: scheme `nuts`, field `place_country`
- Each derived row is `Fact::Classification { field: "place_country", scheme: "nuts", code }`, filed at the scope that published it.
- **Why the field must be distinct.** This is the refute's reason, and it is confirmed:
  - `Fact::key` is `("classification", field)` (crates/store/src/canonical.rs:3117-3125).
  - `supersede` replaces every carried fact whose key the notice republishes (crates/ingest/src/project.rs:6556-6560).
  - So under `place`, a country-only notice would overwrite a carried NUTS code.
- **No read path changes.** Every path keys on `scheme = 'nuts'` alone, with no `field` or `lot_id` predicate:
  - the `?country=` EXISTS: crates/store/src/read.rs:1176-1182;
  - the list echo: read.rs:2400-2403;
  - the reachability probe: read.rs:1473 and :1606-1616;
  - the sparse-country seed: read.rs:2557-2563 and :2611-2615;
  - the lots open-head path: read.rs:3875-3882.
- No production code reads `field = 'place'`. A grep over app, store and ingest finds it only in CLASSIFICATIONS.
- On `/v1/sql`, `scheme='nuts' AND field='place'` still returns exactly the codes as published, so the rule "NUTS is stored as published" (project.rs:5304) stays true for `place`.
- **Spelling: `place_country`, with an underscore.** Every canonical field name is snake_case. The issue's `place-country` was only a placeholder.
- **Leave the DDL comment at canonical.rs:360 alone.** It is never served, and leaving it keeps unit 2 out of the store crate. `/v1/sql/schema` serves the sql.rs notes, and those are updated in D6.

### D2. Code: an exact alpha-3 code, mapped to its NUTS level-0 spelling
`pub fn place_country(field_id: &str, code: &str) -> Option<&'static str>` goes beside `contract_nature` (project.rs:7646). It works in four steps:
1. `field_id` must be in `PLACE_COUNTRY_FIELDS` (D5).
2. Trim and ASCII-uppercase the code, then look it up exactly in the generated `crate::countries::ALPHA3_TO_ALPHA2` (countries.rs:17).
3. The ISO alpha-2 result must be in `NUTS_LEVEL0_ISO`, a closed list of 37 codes:
   - the EU-27: AT BE BG CY CZ DE DK EE ES FI FR GR HR HU IE IT LT LU LV MT NL PL PT RO SE SI SK;
   - the UK: GB;
   - the EFTA countries: CH IS LI NO;
   - the candidate countries: AL ME MK RS TR.
4. Rename GR to `EL` and GB to `UK`. Every other code is served as its alpha-2.

So LTU→LT, GRC→EL, GBR→UK, NLD→NL, NOR→NO, and " ltu " also gives LT. Everything else returns None, which follows 397's rule "left unfolded, not invented" (project.rs:7643-7645). That covers:
- an alpha-2 input;
- `1A0` (the EU authority table's Kosovo);
- `XKX`;
- any unknown code.

The list and the function live in project.rs, not in countries.rs: that file is generated and says "regenerate rather than edit" (countries.rs:1-4).

- **Proposal 2 routed through `canonical_country` + `is_alpha2`. Rejected.** That function is 319's org fold: it maps EL→GR, accepts country names, and passes unknown input through (project.rs:9595-9632). Tying served place codes to it means a later org-layer change could move them. The field's own code lists (`country`, and `eforms-country` in SDK 1.3–1.10) are alpha-3, so an exact table lookup is enough.

### D3. Third countries: skip them and emit nothing
Countries outside the 37 serve nothing from this derivation: EGY, USA, SEN, XKX, and so on. Overseas regions with their own ISO code (REU, GLP, MTQ, GUF, MYT, MAF, ALA) are not folded into FR or FI either.

- **Proposal 2 emitted the ISO alpha-2 (EG, US, SN). Rejected.** Skipping costs only coverage: 1 EGY among the 68 country-only notices in the 24.5M window. Emitting would put non-NUTS codes under scheme `nuts`, against four places that promise NUTS codes today:
  - docs.rs:159-161 and :170;
  - openapi.json:963 ("NUTS place codes.");
  - sql.rs:1033 ("the EU region code").
- **The skip can be reversed; the emit cannot.** Extending the list later is additive. Taking served codes back would break clients.
- **Why the 37-code list is safe.** It is a closed definitional set, not a sample of "common" codes, and a test pins it (see the tests below).
  - No ISO alpha-2 equals another country's NUTS code. EL and UK are not assigned in ISO, and GR and GB are never emitted.
  - So even a wrong inclusion can only serve a country's own code, never another country's.
  - Fixtures show NO, AL and UK codes under `listName="nuts"` (NO072, NO081, AL014, UKI62).

### D4. When a row is kept: after the carry, across the whole version
- **Emit without a guard.** `NoticeState::read` emits a derived row for every mapped country Code, at the scope it was published, with no per-notice check.
- **Drop in `fold`.** The check runs after the Tender supersede (project.rs:6469), the lot loop and the lot value corrections (ending around :6501), and before the rounds block (:6503).
  - The test: does the version hold any `Fact::Classification` with `scheme == "nuts"` and `field != "place_country"`, counting the Tender facts and every lot's facts?
  - If it does, remove every `place_country` fact, from the Tender facts and from every lot.
- **The test mirrors the read predicates exactly (`scheme = 'nuts'`).** So:
  - every version that serves a non-empty `country` today serves the identical list afterwards;
  - only versions that serve `[]` gain a value.
- **The drop is permanent and safe.** A non-derived nuts row, once in a chain, never leaves it: `supersede` only swaps a key's set for a non-empty republished set, and lots are only added or updated (project.rs:6480-6488). So dropping the derived rows from the carried set changes nothing later.
- **The issue's per-notice, per-scope guard is rejected. It serves a wrong value.** Take a CN that is country-only DEU, then a CAN with BT-5071 AT130 and BT-5141 AUT at the same scope. The guard suppresses the CAN's derived row, so the CN's `DE` carries and the version serves `[AT130, DE]`.
- **It also changes NUTS-bearing tenders.** Fixture `eforms/veat-25-00496202-2026.xml` has Procedure ITC4A/ITA and Lot ITA "anyw-cou". The per-scope guard serves `[IT, ITC4A]`; this rule serves `[ITC4A]`.
- **Proposal 2's prefix rule is rejected.** It dropped only the derived codes that prefix a NUTS code. That still adds codes to tenders that already serve a NUTS country:
  - `eforms/cn-pt-national-00245088-2024.xml` (Procedure DE128, Lot SEN) would change;
  - a NL329 + BEL publisher slip would serve BE, and Proposal 2's own risk list admits `?country=BE` then over-matches;
  - a lot-scope DEU from the CN would carry beside a procedure-scope AT130 from the CAN as a stale `DE`.

  Losing those cases is the price of keeping "unchanged wherever non-empty today".
- **Accepted miss.** A country-only lot beside a NUTS place in another country is not served. That is a miss, the same as today, not a wrong value.

### D5. Ids and registration
- **Use exact ids, as 479 did (PROCEDURE_FIELDS, project.rs:7666-7680).** Matching on the stem would also admit `BT-5141-*-List`, an attribute id in SDK 1.9+ (crates/ingest/sdk/fields-1.9.0.json and later). `PLACE_COUNTRY_FIELDS` holds:
  - `BT-5141-Procedure`, `BT-5141-Lot`, `BT-5141-Part`;
  - `SDK01-ProcurementProject-RealizedLocation-Address-Country-IdentificationCode`;
  - `SDK01-ProcurementProjectLot-ProcurementProject-RealizedLocation-Address-Country-IdentificationCode`.

  All are typed `code`, list `country`/`eforms-country`. `decide` turns them into Codes (eforms/sdk.rs:203-211), and they are stored as `NoticeValue::Code` (eforms/value.rs:41-47).
- **Add two DE-1.x aliases after project.rs:1305. This is not conditional:** crates/ingest/sdk/fields-de-1.x.json carries both ids, typed code/country. The chunk resolves aliases at project.rs:9231.
  - `DE1-ProcurementProject-RealizedLocation-Address-Country-IdentificationCode` → `BT-5141-Procedure`;
  - `DE1-ProcurementProjectLot-ProcurementProject-RealizedLocation-Address-Country-IdentificationCode` → `BT-5141-Lot`.
- **Register the ids in `has_destination`'s `Channel::Code` arm** (project.rs:7509-7525): `|| PLACE_COUNTRY_FIELDS.contains(&field_id)`.
  - 368's sieve is `table_reads` → `has_destination` (project.rs:7591), and data_quality.rs uses it.
  - `any_channel_reads` is `#[cfg(test)]` (project.rs:7608-7609), so the issue's unit 2 step would have stopped nothing.
  - The DE-1.x alias gate (`every_de1_alias_target_is_a_field_the_projection_reads`, project.rs:12706) then passes on its own.

### D6. Docs, in the same commit as the code
Put the same text on both surfaces, so the phrase pins match.
- **docs.rs:170**, appended to the `country` cell: "A tender whose notices give the place of performance only as a country (eForms BT-5141; D&Ouml;E's realized-location country), and whose version carries no NUTS place code, is served that country's <strong>NUTS level-0 code</strong>, so <code>LT</code> finds it and <code>LT0</code> does not (Greece <code>EL</code>, the United Kingdom <code>UK</code>). Only the EU-27, the United Kingdom, Iceland, Liechtenstein, Norway, Switzerland, Albania, Montenegro, North Macedonia, Serbia and T&uuml;rkiye are served this way: a place in any other country, e.g. Egypt or the USA, carries none."
- **openapi.json:725**: the same text as markdown, after "e.g. `DE1` or `DEB35`.".
- **openapi.json:963**: "NUTS place codes; a level-0 code (`LT`) where the version's notices gave the place only as a country (issue 532)."
- **openapi.json:992**: change "`nuts` (field `place`, as published)" to "`nuts` (field `place`, as published; or field `place_country`: the NUTS level-0 code of a place the notices gave only as a country, present only on a version with no other `nuts` row, issue 532)".
- **sql.rs:1039-1040** (`CLASSIFICATION_FIELD_NOTE`): add "place_country (nuts: the level-0 code of a place published only as a country, only on a version with no other nuts row; issue 532)". sql.rs:1033 stays true as written.
- **A docs test** in the pattern of docs.rs:1557, using `assert_says` (docs.rs:1299):
  - name: `the_docs_say_a_country_only_place_is_served_as_its_nuts_level_0_code`;
  - read from the `country` row and the openapi `country` parameter, not the whole surface;
  - pins: "only as a country", "nuts level-0 code", "e.g. egypt or the usa, carries none".
- **CHANGELOG.md**: add "## Unreleased (issue 532) — a place of performance published only as a country is served in `country` and found by `?country=`". The body says:
  - only on versions with no NUTS place;
  - as the level-0 code;
  - third countries carry none;
  - on `/v1/sql` it is field `place_country`, and `field='place'` is unchanged;
  - stored tenders change only after unit 3's refold.

### D7. The r209 buyer/winner-as-place precedent
It changes nothing here.
- The precedent is confirmed: project.rs:361/364/366 map `TED-NUTS`, `TED-CA_CE_NUTS` and `TED-TENDERER_NUTS` to `place`, and r209/rules.rs:390 parses them as Nuts.
- Its one interaction: in a chain that crosses eras, a carried legacy buyer NUTS suppresses a later eForms country-only row under D4. That is a miss, not a wrong value.
- The precedent itself over-matches `?country=` on legacy TED (tender 8432519). **File it as its own issue.** It is not built here.

### D8. Epoch
Do not bump `PROJECTION_EPOCH` (canonical.rs:1851). This is a profile-scoped mapping fix, and canonical.rs:1810-1816 says that kind must ride a scoped refold. Unit 3 picks the route.

### Unit 2 build spec (no re-deciding)
1. **project.rs, beside `contract_nature`.** Add:
   - `const PLACE_COUNTRY_FIELD: &str = "place_country"`;
   - `PLACE_COUNTRY_FIELDS` (the 5 ids);
   - `NUTS_LEVEL0_ISO` (the 37 codes);
   - `pub fn place_country` (D2).
   Do not touch countries.rs or `canonical_country`.
2. **`NoticeState::read`.** Right after the `contract_nature` arm (ends at project.rs:5300), add an arm of the same shape. It inserts the fact into `facts` (Tender scope) or into `lots.get_mut(key)` (Lot scope). It does not `continue`, because a Code value falls through to `_ => None` (around :5431).
3. **`fold`.** Add a small helper over `(&mut facts, &mut lots)` with the D4 drop, placed after the lot value corrections and before the rounds block.
4. **Aliases and registration.** Add the two DE1 aliases and the line in `has_destination`'s Code arm (D5).
5. **Docs.** Make the D6 changes.

Tests:
- **The pure function:**
  - LTU→LT, GRC→EL, GBR→UK, NLD→NL, " ltu "→LT;
  - NOR→NO, CHE→CH, ISL→IS, LIE→LI, TUR→TR, MKD→MK, SRB→RS, MNE→ME, ALB→AL;
  - None for EGY, USA, SEN, REU, XKX, 1A0, `LT` and "";
  - None for the wrong ids `BT-5141-Lot-List`, `BT-514-Organization-Company` and the SDK01 PostalAddress country.
- **The table:**
  - 37 entries, each in `countries::ALPHA2` and reached from exactly one alpha-3;
  - over all of `ALPHA3_TO_ALPHA2`, no output is GR or GB;
  - the output set equals the literal 37-code NUTS set (with EL and UK).
- **`table_reads("notice_codes", id)`** is true for the 5 ids and for both DE1 ids.
- **The fold** (template: project.rs:11026-11069):
  - (a) BT-5071-Lot NL329 + BT-5141-Lot NLD on one notice gives {NL329}.
  - (b) A CN with NL329/NLD, then a CAN with NLD only: v2 is exactly {NL329}.
  - (c) A CN with NLD only, then a CAN with NL329 + NLD: v1 {NL}, v2 {NL329}.
  - (d) A CN with DEU only, then a CAN with AUT only, same scope: v2 {AT}.
  - (e) A CN with DEU only, then a CAN with AT130 + AUT: v2 {AT130}. This kills the per-notice guard.
  - (f) A CN with DE212, then an AUT-only notice and an FRA-only notice: every version {DE212}.
  - (g) Procedure NLD + Lot NL329 on one notice gives {NL329}.
  - (h) A CN with NLD only, then a CAN silent on place: NL carries.
  - (i) Procedure NL329 + Lot BEL gives {NL329}. This pins the accepted miss.
- **Fixtures** (each was read in the XML):
  - `eforms/cn-selc-tp-00157944-2024.xml` serves ["BE"];
  - `eforms/can-pat-social-00250633-2024.xml` serves ["DK"];
  - `eforms/can-inline-org-00530983-2024.xml` serves ["NO"];
  - `eforms/doe-sdk10-tenderrecipient.xml` serves ["DE"];
  - `eforms/veat-25-00496202-2026.xml` serves ["ITC4A"] only;
  - `eforms/cn-pt-national-00245088-2024.xml` serves ["DE128"] only;
  - `eforms/can-cvd-legacy-00381774-2025.xml` serves ["NO072"] only.
- **sdk-0.1:** a synthetic parse with both SDK01 country ids DEU and no subentity gives DE at Tender and at Lot scope.
- **The API, in crates/app/tests/api.rs next to 1066-1095:**
  - a country-only tender is found by `?country=BE`, echoes ["BE"], and is not found by `?country=BE1`;
  - its detail carries `{field:"place_country", scheme:"nuts", code:"BE"}`;
  - a GBR place is found by `?country=UK` and not by `?country=GB`.
- **The golden:** `project_golden`'s snapshot must stay byte-identical. Every RealizedLocation in its corpus carries a NUTS subentity (MT001, LV00A, DE263, DED2D, DE212), and brin-x01 has none. Run `ops/check.sh`.

### Risks carried to units 2 and 3
- **Coverage lost to D3:** EGY, USA and SEN places, and overseas ISO regions, keep serving `[]`.
- **Inclusions taken on trust:** CH, IS, LI, ME, MK, RS and TR are in the list because Eurostat's statistical-regions table covers them. They are not seen in-repo.
- **Inputs that are skipped:** an alpha-2 or other non-alpha-3 value is skipped. Unit 3 samples the stored values for the 7 ids.
- **DE-1.x evidence:** these ids are known only from the vendored empirical inventory. Confirm them on one DE-1.x notice's `notice_codes`.
- **Unit 3 rewrites more than empty heads.** The refold rewrites every tender that has any NUTS-free version gaining a code, including the v1 of a chain whose head has NUTS. That is announced as a correction, so read the dry-run's corrected count with this in mind.
- **The seed cap.** The added level-0 rows grow per-prefix counts. A small country's prefix near `COUNTRY_SEED_CAP` (200,000, read.rs:2603) could flip from the sparse seed to the walk. Unit 3 checks counts near the cap and the latency of `?country=` for those prefixes after the refold.
- **Ordinary carry across scopes still applies.** A CN's country-only lot and a CAN's country-only procedure place in another country both serve.
- **FTS is untouched** (unit 4). fts/parse.rs:590-591 still drops a delivery address with no region.

### Line refs in the issue that have drifted
| issue cites | now |
| --- | --- |
| docs.rs:167 | docs.rs:170 |
| read.rs:1077-1083 | 1176-1182 |
| read.rs:2159-2164 | 2400-2403 |
| canonical.rs:3090-3098 | 3117-3125 |
| project.rs:6555-6559 | 6556-6560 |
| `contract_nature` arm 5276-5298 | 5277-5300 |
| `contract_nature` fn :7645 | 7646 |
| CLASSIFICATIONS check :5304 | :5305 |
| "any_channel_reads (7608)" as the place to register | the sieve is `has_destination`'s Code arm, 7509-7525 |

### Rejected at decision
- The issue's guard (checked per notice and per scope, before the carry). It can serve a stale country: a CN that is country-only DEU, then a CAN with AT130 + AUT, serves [AT130, DE]. It also changes tenders that already have NUTS: the veat-25-00496202-2026 fixture would serve [IT, ITC4A].
- Proposal 2's prefix rule (drop a derived code only when it is a prefix of a NUTS code on the version). It adds codes to tenders that already serve a NUTS country: cn-pt-national would gain SN, a NL329 + BEL publisher slip would serve BE, and a lot-scope DE carried beside a procedure-scope AT130 would be stale. That loses two things: the guarantee that every version serving a non-empty country today is unchanged, and the match between unit 3's corrected count and the cohort of former empties.
- Emitting the ISO alpha-2 for third countries (EG, US, SN, RE) under scheme nuts. It contradicts the documented 'NUTS place codes' and 'EU region code', and served codes cannot be withdrawn later without breaking clients. Skipping costs only 1 of 68 country-only notices, and the list can be extended later.
- Deriving the code through canonical_country plus is_alpha2. It ties served place codes to the 319 org fold, which accepts names, passes alpha-2 through and maps EL to GR. Use an exact alpha-3 lookup in ALPHA3_TO_ALPHA2 instead.
- Matching BT-5141 by stem. It also admits BT-5141-*-List, an attribute id in SDK 1.9 and later. Use exact ids, as 479 did.
- Registering the ids in any_channel_reads. It is #[cfg(test)] and is not 368's sieve. Use the Code arm of has_destination, which table_reads calls.
- Storing the derived row as field 'place'. supersede would then overwrite a carried NUTS code (the refute's case).
- The spelling 'place-country'. Every canonical field name is snake_case.
- Folding overseas regions with their own ISO code (REU, GLP, MTQ, GUF, MYT, MAF, ALA) into FR or FI. That needs a hand-kept table whose NUTS membership is uncertain; skip them instead.
- Making the DE-1.x aliases conditional. fields-de-1.x.json carries both RealizedLocation Country ids, typed code/country.
- Bumping PROJECTION_EPOCH. canonical.rs:1810-1816 says a profile-scoped mapping fix must ride a scoped refold.
- Editing the DDL comment at canonical.rs:360. It is never served and would pull unit 2 into the store crate; /v1/sql/schema serves the sql.rs notes, which are updated.
- Fixing the r209 buyer/winner-as-place precedent in this issue. File it as its own issue.

## Proposed units

1. **Decide the destination (S).** Record the decision here. Fixed by the refute:
   - **Scheme `nuts`, a distinct field (e.g. `place-country`). This is required, not one option among several.**
     `Fact::key` for a Classification is `("classification", field)` (canonical.rs:3090-3098), and `supersede`
     replaces the whole keyed set (project.rs:6555-6559). Under `field='place'`, a later country-only notice would
     replace a precise NUTS code carried from an earlier one: a CN with BT-5071 NL329 followed by a CAN with only
     BT-5141 NLD would leave the head at `NL`. A distinct field avoids that, and the filter and echo still work with
     no read-path change because they key on the scheme only. It also keeps the "NUTS stored as published" rule on
     `place` (project.rs ~5302) true and the derived row traceable. Magnitude is probably small (0 of 4 country-only
     CANs read, 884214, 539783, 532187, 313708, had a NUTS-bearing earlier version), but a test pins it.
   - **The code** is the NUTS level-0 spelling: alpha-2 except GRC → `EL` and GBR → `UK`. Not ISO, and not 319's
     EL → GR direction. Decide third countries (EGY, USA, …): emit alpha-2, or skip.
   - **When to emit:** only when the same scope (Procedure, or that Lot) on the same notice publishes no NUTS place.
     This guard is per notice, before the carry. So a tender with a NUTS place at that scope on every notice keeps its
     served `country` unchanged, but a country-only CN followed by a NUTS-bearing CAN serves both `NL` and `NL329`
     (redundant, prefix-consistent). Accept that, or evaluate the guard after the carry. Do not promise
     "byte-for-byte unchanged" for the NUTS-bearing majority unless it is checked after the carry.
   - **Docs:** one sentence on docs.rs:167 and the openapi `country` parameter: a place published as a country alone
     is served as its NUTS level-0 code.
   - Record whether the r209 buyer/winner-as-place precedent changes anything for this issue (expected: no; it is
     its own issue).
2. **Build the derivation (M).** A Code-branch arm beside `contract_nature` (project.rs:5276-5298) that emits a
   `Fact::Classification` at the scope published. Ids: `BT-5141-Lot`, `BT-5141-Procedure`, and both SDK01
   `...RealizedLocation-Address-Country-IdentificationCode` ids (Tender and Lot; confirm the exact spellings against
   notice 26168799's `notice_codes`). Check whether the DE1 parser emits a RealizedLocation country; the DE1 alias
   table (project.rs ~1301) maps only the subentity today. Register the ids in the Code channel of
   `any_channel_reads` (project.rs:7608) so 368's sieve stops listing BT-5141. Tests:
   - LTU → `LT`, GRC → `EL`, GBR → `UK`, and the third-country rule unit 1 picks;
   - no derived row beside a BT-5071 at the same scope on the same notice;
   - a CN with BT-5071 NL329, then a CAN with only BT-5141 NLD: the head keeps NL329;
   - a country-only tender is found by `?country=NL` and echoes `NL`.
   Run the gate (`ops/check.sh`), not a bare `cargo test`.
3. **Refold (size first).** BT-5141 is on about 85–94% of eForms notices in the windows above, so a `refold-fields`
   over it is effectively the whole eForms and DÖE era. Dry-run it for the size. Either ride the next era-wide fold,
   or build the narrow cohort (notices with a country-only place) first. Then run `## Verify` and the DÖE witness
   (`/v1/tenders/1365142` serves `["DE"]`).
4. **FTS: measure, then decide (S, then M).** Count releases with a delivery address that has `country` and no
   `region`, over recorded pages (an in-app read, not a corpus scan from the agent). Map it only if the count is
   non-zero. That needs a parser change (emit the country at parse.rs:590-591, fix checklist.rs:82/152) and an FTS
   re-parse, not a refold.
5. **Optional standing measure (S).** Add a `place` field (`tender_version_classifications`, scheme `nuts`) to
   `data_quality.rs` FIELDS (6 today). The weekly job then gives the per-profile empty rate, which replaces the
   external review's unmeasurable 17% with a number that recomputes itself.
