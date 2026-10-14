# 534 — legacy award winners serve the element name `AWARD_AND_CONTRACT_VALUE` as their role, and three more party-role gaps the 393 fold left

Status: ready-for-agent — filed 2026-10-14 from an external data review (verified: verifier + adversarial refute). Next: unit 1, fold `AWARD_AND_CONTRACT_VALUE` → `winner` and `RESPONSIBLE_FOR_MEDIATION_PROCEDURES` → `mediation-body` in `legacy_role` (S; build and test, the refold is unit 3).
Kind: defect (projection vocabulary): `legacy_role` in `crates/ingest/src/project.rs:8985`; the party read
`PARTIES_SQL` (`crates/store/src/read.rs:2785`); the role docs in `crates/app/data/openapi.json` (`parties[]`) and
`crates/app/src/v1/sql.rs:1103-1113`.
Relates to:
- 393 (done 2026-09-28): the legacy role fold and the deliberate eForms second vocabulary. Its unit-2 band table
  listed `AWARD_AND_CONTRACT_VALUE` (280 rows) with no meaning and left it verbatim; it never listed
  `RESPONSIBLE_FOR_MEDIATION_PROCEDURES`. The eForms decision is NOT reopened here.
- 368 (done): the unmapped-vocabulary sieve `legacy_role`'s comment defers to. Its own write-up (lines 531-540)
  records that the pointer channel accepts any `TED-` id, so an unfolded role never reaches it.
- 483 (ready-for-agent): its census notes (line 67) that legacy `ADDRESS_REVIEW_INFO` is not a review body and reads
  the element to work around the fold. It never filed the fold fix.
- 484 (done): the `is_buyer` flag (`read.rs:3034`) is keyed on `"winner" | "Tenderer"`, so it never fires on an
  `AWARD_AND_CONTRACT_VALUE` winner. Unit 1 fixes that without touching 484's code.
- 242: the F06 award container. Its fixture `r209/f06-002856-2017.xml` and `crates/ingest/tests/r209.rs:183-218` pin
  the `AWARD_AND_CONTRACT_VALUE` org as the winner.
- 364: tender 2816628 carries 15,287 `AWARD_AND_CONTRACT_VALUE` rows / 1,986 orgs; it noted the raw legacy names.
- 527 (ready-for-agent): tender 229981's `(niet gegunde partij)` losers keep `Tenderer`; unit 6's counter-example.
- 259 (closed): legacy winner doubling from nested org sections; another mechanism, same surface as item 4.

## Verify

    curl -s --max-time 30 https://tenders.zebreus.click/v1/tenders/5200474 https://tenders.zebreus.click/v1/tenders/8400010 | jq -c '[.id, ([.parties[]|select(.organization_id==1182393 or .organization_id==1163392)|.role]|sort)]'

Covers units 1-3 (the folds and their refold). Units 4-6 are a decision, a detector and docs; check them in place.
- **done**: `[5200474,["winner"]]` and `[8400010,["appeal-information","review-body"]]`.
- **open (2026-10-14)**: `[5200474,["AWARD_AND_CONTRACT_VALUE"]]` and `[8400010,["review-body","review-body"]]`.

## Repro

Public GETs, 2026-10-14:
- 4116961 (293363-2009): org 28338338 is served 5 times and 28338340 10 times, all `winner`, all `lot: null`.
- 4116007 (031880-2011): `AWARD_AND_CONTRACT_VALUE` 1860509, 31391341 ×2 = RES-2, RES-1/RES-3 winners.
- 5200476 (201629-2014): `AWARD_AND_CONTRACT_VALUE` 5271194 twice, both `lot: null`.

Bounded reads (`echo SQL | ssh root@zebreus.click /root/sq.sh`), from the verifier and refute:
- Band 5,200,000–5,203,000, roles: `AWARD_AND_CONTRACT_VALUE` 279 rows / 118 tenders, `winner` 6,622 / 1,626,
  `review-body` 3,690 / 1,971, `appeal-information` 2,403 / 1,227, `RESPONSIBLE_FOR_MEDIATION_PROCEDURES` 13.
- The same band, cross-check: `SELECT count(*) AS n_rows, count(DISTINCT p.tender_id) AS tenders, sum(CASE WHEN EXISTS
  (SELECT 1 FROM tender_version_result_winners w WHERE w.tender_id = p.tender_id AND w.seq = p.seq AND
  w.organization_id = p.organization_id) THEN 1 ELSE 0 END) AS rows_also_result_winner FROM tender_version_parties p
  WHERE p.tender_id BETWEEN 5200000 AND 5203000 AND p.role = 'AWARD_AND_CONTRACT_VALUE'` gives **279 / 118 / 279**.
- Band 8,400,000–8,403,000: `review-body` 9,025 / 2,711 of 2,927 tenders, `appeal-information` 389 / 166,
  `AWARD_AND_CONTRACT_VALUE` 41 / 24.

## What was found

### 1. F06 award winners are served under the role `AWARD_AND_CONTRACT_VALUE`

`legacy_role` ends in `other => other.to_owned()`. Its comment (`project.rs:9026`) says the element is "a section
wrapper whose party meaning nobody has established. An unfolded name still reaches issue 368's sieve". Both halves
are wrong:
- **The meaning is established.** `r209.rs:183-218` pins the `TED-AWARD_AND_CONTRACT_VALUE` org ref as the winner
  ("F06 nests the contractor address directly under AWARD_AND_CONTRACT_VALUE"). The only org block under the element
  is `CONTACT_DATA_WITHOUT_RESPONSIBLE_NAME_CHP`, the contractor. `read_legacy_results` (`project.rs:6874`, ~6908)
  already reads any inline org reference in an award block as a winner.
- **Prod agrees, every row.** In band 5.2M, 279 of 279 rows are also a result winner of the same tender and version
  (Repro). 5200474 (289468-2014) serves 1182393, 1269150 and 5558949 under the element; its `lot_results` name
  5558949 (RES-1), 1269150 (RES-2) and 1182393 (RES-3). 4116007 and 5200476 match the same way.
- **The sieve cannot see it.** The pointer channel tests `role_name(field_id).is_some()` (`project.rs:~7538`), and
  `role_name` (`:8210`) accepts any `TED-` id.

Era: F06 (utilities award) notices of R2.0.7 and R2.0.8. 4116007's mention notice 4384563 declares R2.0.7.S03.E01
(profile `ted-export-r208`); 242's fixture is the R2.0.8 shape. Rows by band: 10 / 3 tenders (4.116M, 2009-11),
279 / 118 (5.2M, 2014), 41 / 24 (8.4M, 2017-18). The r207 fixture `f06-r207-070248-2010.xml` has the same shape.
`internal_ojs.rs:145` aliases `AWARD_AND_CONTRACT_VALUE_SUM` onto the element for the 2008 era; not measured.

Impact: `?winner=` and `lot_results` are fine, because they read `tender_version_result_winners`. What misses these
rows is `parties[]`, any role filter on `/v1/sql`, and 484's `is_buyer` flag. The openapi text already hedges:
"Any other value is a source element name not yet folded, kept verbatim rather than guessed at: treat it as unknown,
not as canonical." So the defect is not a broken promise of a closed set. It is a verbatim role whose meaning is
established and should be folded.

### 2. `RESPONSIBLE_FOR_MEDIATION_PROCEDURES` is served verbatim too

A full `GROUP BY role` over band 5.2M returns 14 strings, 2 of them verbatim: `AWARD_AND_CONTRACT_VALUE` (279) and
`RESPONSIBLE_FOR_MEDIATION_PROCEDURES` (13). The second is the R2.0.8 defence twin of
`RESPONSIBLE_FOR_APPEAL_PROCEDURES`, which `legacy_role` folds to `review-body`. `MEDIATION_PROCEDURE_BODY_RESPONSIBLE`
and `ADDRESS_MEDIATION_BODY` fold to `mediation-body` (`:9004`); this one does not. Band 8.4M has 10 strings with 1
verbatim, and band 4.116M has 7 with 1 verbatim (both `AWARD_AND_CONTRACT_VALUE`). No pre-393 raw names remain in
these bands.

### 3. r209's `ADDRESS_REVIEW_INFO` folds to `review-body`; its r208 equivalent folds to `appeal-information`

`legacy_role` maps `ADDRESS_REVIEW_BODY | ADDRESS_REVIEW_INFO` → `review-body` (`:9000-9003`). `ADDRESS_REVIEW_INFO`
is R2.0.9's VI.4.4, the service that gives information about review procedures. That is the same slot as R2.0.8's
`LODGING_INFORMATION_FOR_SERVICE` / `SERVICE_FROM_INFORMATION`, which maps to `appeal-information` (`:9012`). eForms
also keeps `ReviewInfo` apart from `ReviewOrg`. The repo knows this in three places and works around it in each:
483 line 67, `docs/operations.md:1668`, and `role_census.rs:136-139` (`LEGACY_ELEMENT_KINDS` reads the element
"before role_name's fold" because the fold "puts ADDRESS_REVIEW_INFO ... under review-body beside the review body
proper").

Examples: 8400010 (233665-2018) serves 1163392 `Úrad pre verejné obstarávanie` twice as `review-body`; notice
19291350 points `TED-ADDRESS_REVIEW_BODY` → ORG-4 and `TED-ADDRESS_REVIEW_INFO` → ORG-5, two identical blocks.
8400009 serves two different orgs as `review-body`: 3955, and 1173633 `Servicio Juridico del SMS`, the information
service.

Size, band 8.4M: a `review-body` (seq, org, lot) group repeats on **582 of 2,927 tenders (19.9%)**, 1,465 groups,
each exactly twice, none lot-scoped. `appeal-information` there (389 rows) comes only from r208-profile notices.

### 4. A multi-award legacy notice repeats its winner once per award, with nothing to tell the rows apart

`PARTIES_SQL` serves every `tender_version_parties` row, and the projection writes one row per (scope, role, org
section). A legacy award notice has one inline org block per award, so a contractor with k awards gets k rows. The
award's lot almost never resolves (1 of 782 repeated `winner` groups in band 5.2M is lot-scoped; 5200476 has 2 lots
and every winner row reads `lot: null`), and the award key is not served. So the k rows look identical. 4116961 has
17 awards: Kuśmierz (28338338) appears 5 times (RES-8/9/10/13/15), the MGGP/MBI consortium (28338340) 10 times.

Tenders with a repeated (seq, role, org, lot) group in any version, per 3,000-id band:

| band | era | tenders | with a repeat | in a winner role |
| --- | --- | --- | --- | --- |
| 4,116,000–4,119,000 | 2009 legacy | 3,001 | 208 (6.9%) | 207 |
| 5,200,000–5,203,000 | 2014 r208 | 2,946 | 296 (10.0%) | 290 |
| 8,400,000–8,403,000 | 2017-18 r209 | 2,927 | 647 (22.1%) | 66 (582 are item 3) |
| 160,000–163,000 | eForms | 2,934 | 17 (0.6%) | 7 |

In band 8.4M the winner repeats are 796 groups on 65 tenders (305 groups lot-scoped); the largest repeats 490 times.
Not measured: a corpus-wide share, and a head-version-only share (needs a join to `tenders.current_seq`).

### What the external review got wrong

- **(a) The two vocabularies are a decision, not a defect.** 393 unit 2 serves eForms roles as published because
  `Lot-`/`Procedure-` carries scope, and openapi documents it. Do not re-file it. But "winner = `Tenderer`" is not an
  equivalence: `EFORMS_ROLE_MEANING` (`project.rs:9051`) reads `Tenderer` as `tenderer`, and 527's losers keep it.
- **(b) "2011-2016 TED winners" is the wrong scope.** It is F06 under R2.0.7 and R2.0.8, from 2009
  into 2017-18. The impact is narrower than implied: `?winner=` and `lot_results` already name these orgs.
- **(c) The repeats are not byte duplicates.** Each row is a separate award section whose lot and award key are not
  served. The 9.6% figure could not be checked corpus-wide; per band it runs 0.6% to 22.1%. In r209 most repeats
  (582 of 647 tenders) are the `ADDRESS_REVIEW_INFO` fold (item 3), not winners.

## Proposed units

1. **Fold `AWARD_AND_CONTRACT_VALUE` → `winner` and `RESPONSIBLE_FOR_MEDIATION_PROCEDURES` → `mediation-body` (S).**
   - Correct the `legacy_role` comment. Add both to `the_legacy_party_roles_fold_onto_a_canonical_vocabulary`
     (`project.rs:14372`). `TRANSLITERATED_ADDR` stays deliberately unfolded.
   - Before building, read the inventory and the r207 fixture: does any form put a non-contractor org block directly
     under `AWARD_AND_CONTRACT_VALUE`? The prod cross-check (279/279) is already done; do not repeat it.
   - Parties carry forward per role. After the fold, a chain whose later notice publishes `winner` stops carrying the
     earlier F06 rows forward, the same rule every other legacy winner follows. Pin it in a test.
   - 484's `is_buyer` starts to fire on these rows with no code change. That is intended.
2. **Fold r209 `ADDRESS_REVIEW_INFO` → `appeal-information` (S).**
   - Consumers are already checked: no Rust code keys on `'review-body'` (only `sql.rs` doc text, `legacy_role` and
     `EFORMS_ROLE_MEANING`). `role_census` `ROLE_KINDS` maps `appeal-information` to `ReviewAdjacent`, so the
     refold moves these rows into the right census kind. 483's census reads the element and is unaffected.
   - Add `('Lot-ReviewInfo', 'appeal-information')` to `EFORMS_ROLE_MEANING` and to the openapi / `sql.rs` mapping.
   - `ADDRESS_REVIEW_BODY` must still fold to `review-body`; assert both in the test.
   - Build units 1 and 2 together and run `ops/check.sh` once.
3. **Refold, size first (M, mostly wall time).** One `refold-fields` over `tables: ["notice_ids"]` for
   `TED-AWARD_AND_CONTRACT_VALUE`, `TED-RESPONSIBLE_FOR_MEDIATION_PROCEDURES` and `TED-ADDRESS_REVIEW_INFO`.
   - Dry-run it with `expect: 1` first (operations.md, SIZE FIRST). The `ADDRESS_REVIEW_INFO` cohort is about the
     whole r209 era, so expect millions of notices, past `LEGACY_CLOSURE_CAP` (500,000), and the fallback to a FULL
     projection. 393's precedent: dry job 1474 sized 6,998,915 notices; a full projection runs about 5.2 h.
   - Queue it behind the running drains (526, 522, 524). Parse rows are unchanged; no re-parse.
   - Then run `## Verify`, and re-read band 5.2M: `AWARD_AND_CONTRACT_VALUE` 0 and
     `RESPONSIBLE_FOR_MEDIATION_PROCEDURES` 0 (from 279 and 13).
4. **Decide the repeated-row shape of `parties[]` (S, a decision).** Record it here. Options:
   - (a) collapse identical (role, org, lot) rows at read, keeping one `mention_name`;
   - (b) keep one row per mention and serve the award/result key;
   - (c) document per-mention semantics.
   Unit 3 removes the r209 `review-body` repeats on its own, so decide on the winner repeats that remain. Why a legacy
   award's `lot` does not resolve belongs to the external review's legacy-lot finding, not here.
5. **Make a verbatim role visible (S-M).** The fall-through relies on 368's sieve, which cannot see it. Either:
   - a test that walks the r208/r209 inventory's party-bearing parents and requires each to fold, or to sit on an
     explicit refusal list with its reason (`TRANSLITERATED_ADDR` is today's only one); unverified candidates to
     check: `ADDRESS_PARTY` (an r209 `Rule::Org` element missing from `legacy_role`) and
     `SERVICE_INFORMATION_LODGING_APPEALS` (an r208 group);
   - or a data-quality line counting served roles outside the published vocabulary.
6. **Docs (S).** In openapi `parties[]` and `sql.rs:1104-1113`:
   - Drop "a winner is `winner` or `Tenderer`" and point winner questions at `lot_results[].winners` / `?winner=`
     (527's tender 229981 is the counter-example).
   - Drop the "rows written before the issue-393 re-projection" caveat; that re-projection ran (job 1616), and the
     band role lists show no stale names.
   - Keep "any other value is a source element name not yet folded" until unit 5's detector exists.
