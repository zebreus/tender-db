# 532 — a place of performance published as a country alone never projects, so the tender serves `country: []` and `?country=` misses it

Status: ready-for-agent — filed 2026-10-14 from an external data review (verified: verifier + adversarial refute). Next: unit 1, decide the destination (S; the decision is the step).
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
