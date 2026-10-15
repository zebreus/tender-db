# 534 — legacy award winners serve the element name `AWARD_AND_CONTRACT_VALUE` as their role, and three more party-role gaps the 393 fold left

Status: ready-for-agent — units 1, 2, 5 and 6 DEPLOYED `4d4fe13` 2026-10-15 02:04 UTC. Unit 3 sized at 2,006,441 notices (job 2494), and its refold is RUNNING: `refold-fields` 2512 is done (2,006,441 re-queued, 1,117,095 tenders stamped) and `project` 2513 started 07:47 UTC, ~7.5 h. Next: when 2513 ends, run `## Verify`, re-read band 5.2M and move the docs caveat to RETIRED. Unit 4 (the repeated-row decision) is open.
Was: ready-for-agent — UNITS 1, 2, 5 (the test variant) AND 6 BUILT 2026-10-14 and REVIEWED (two reviewers; the review round's fixes are in, see "Review round"; uncommitted in the worktree; focused tests green; see "Units 1/2/5/6 — built"). The fold grew past the two named elements: three more R2.0.8 wrappers prod serves verbatim (`BODY_RESPONSIBLE_APPEAL`, `BODY_RESPONSIBLE_MEDIATION`, `SERVICE_INFORMATION_LODGING_APPEALS`) fold too, and seven elements sit on an explicit refusal list (the review added `PARTY_NAME_ADDRESS`). Next: `ops/check.sh`, commit, deploy with the batch. Then unit 3, the refold — NOT run: a ~7.5 h full projection (job 1616 took ~442 min), to schedule after 524's post-deploy census (522 and 536 are done) in a window that spans no 07:35 UTC daily tick, with the field list widened below; when it lands, the docs' pre-refold caveat goes (unit 3). Unit 4 (the repeated-row decision) is still open. Was: ready-for-agent — filed 2026-10-14 from an external data review (verified: verifier + adversarial refute). Next: unit 1, fold `AWARD_AND_CONTRACT_VALUE` → `winner` and `RESPONSIBLE_FOR_MEDIATION_PROCEDURES` → `mediation-body` in `legacy_role` (S; build and test, the refold is unit 3).
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
   `TED-AWARD_AND_CONTRACT_VALUE`, `TED-RESPONSIBLE_FOR_MEDIATION_PROCEDURES` and `TED-ADDRESS_REVIEW_INFO`,
   plus (as built, see below) `TED-BODY_RESPONSIBLE_APPEAL`, `TED-BODY_RESPONSIBLE_MEDIATION` and
   `TED-SERVICE_INFORMATION_LODGING_APPEALS`.
   - Dry-run it with `expect: 1` first (operations.md, SIZE FIRST). The `ADDRESS_REVIEW_INFO` cohort is about the
     whole r209 era, so expect millions of notices, past `LEGACY_CLOSURE_CAP` (500,000), and the fallback to a FULL
     projection. 393's precedent: dry job 1474 sized 6,998,915 notices. **Wall time ~7.5 h**: the latest full
     fallback, job 1616, took ~442 min (`docs/operations.md` ~1066, and the runbook's "~7.5 h" at ~1084); the
     5.2 h of 2026-09-10 is stale.
   - **Window:** the run must span no daily tick (09:35 Berlin = 07:35 UTC until 2026-10-25, 08:35 UTC after).
     Either start it once the day's daily chain has drained (it then ends late afternoon UTC and overlaps none), or
     start it by ~22:30 UTC (the 07:35 UTC tick leaves almost no margin for a 7.5 h run started at 00:00; 08:35 UTC from 2026-10-25). Never a start that a 7.5 h run carries across a tick.
   - Queue it after 524's census (526 and 522 are done). Parse rows are unchanged; no re-parse.
   - Then run `## Verify`, and re-read band 5.2M: `AWARD_AND_CONTRACT_VALUE` 0 and
     `RESPONSIBLE_FOR_MEDIATION_PROCEDURES` 0 (from 279 and 13).
   - **Docs, in the same change as the Verify:** remove the sentence "Until the issue-534 refold has run, a tender
     projected before it may still serve R2.0.9's appeal-information slot (`ADDRESS_REVIEW_INFO`) as `review-body`,
     and a legacy award winner as `AWARD_AND_CONTRACT_VALUE`." from openapi `parties[]` and the `sql.rs`
     `tender_version_parties.role` note; move its phrase from the positive pin in
     `the_role_docs_send_winner_questions_to_the_award_decisions` to `docs.rs` RETIRED ("refold has run, a tender
     projected before it", with the job id as the reason).
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

## Units 1/2/5/6 — built (2026-10-14)

Uncommitted in the worktree. Unit 3 (the refold) is NOT run: it is a ~7.5 h full projection, to schedule after
the 522/524/536 post-deploy jobs in a window that spans no daily tick (unit 3). Until it runs, stored rows keep the
old roles; the fold applies to every notice folded after the deploy, and both role texts say so (unit 6).

### Pre-build checks

- **Does any form put a non-contractor org block directly under `AWARD_AND_CONTRACT_VALUE`? No, on every source
  this run could read.** The walker names a role after the award element only for a `CONTACT_DATA*` block that is
  its DIRECT child (`r209/parse.rs`, `Rule::Org`: "the role lives on the wrapper"). Any other org element there would
  be served under its own name. The inventory lists `AWARD_AND_CONTRACT_VALUE` as r208-only (`@ITEM`), and it records
  names, not parent edges, so the answer comes from bytes:
  - r207 fixture `r208/f06-r207-070248-2010.xml` (R2.0.7.S02.E01): 17 award blocks, all with the same children:
    `CONTRACT_NO, TITLE_CONTRACT, DATE_OF_CONTRACT_AWARD, NUMBER_OF_OFFERS_RECEIVED,
    CONTACT_DATA_WITHOUT_RESPONSIBLE_NAME_CHP, NO_CONTRACT_LIKELY_SUB_CONTRACTED`. That is one org block per award,
    the contractor (Amtrain Midlands, MGB Signalling, Bridgen Training, Catalis, ...).
  - R2.0.8 fixture `r209/f06-002856-2017.xml` (R2.0.8.S02.E01): one block, children `CONTRACT_NO, TITLE_CONTRACT,
    DATE_OF_CONTRACT_AWARD, OFFERS_RECEIVED_NUMBER, OFFERS_RECEIVED_NUMBER_MEANING,
    CONTACT_DATA_WITHOUT_RESPONSIBLE_NAME_CHP, INFORMATION_VALUE_CONTRACT, NO_CONTRACT_LIKELY_SUB_CONTRACTED`. Again
    one org block, the contractor (ELPATOR, pinned by `r209.rs:183-218`).
  - The sub-contracting slot carries no org block in either form. Prod's 279/279 (Repro) was not repeated.
  - Not read: the mirrored XSD on the VPS (no ssh in this run). A form with a second org block directly under the
    element would show up as a served winner that is no result winner. The issue's cross-check SQL is the probe.
- **`internal_ojs.rs:145` (`AWARD_AND_CONTRACT_VALUE_SUM`): folds with no extra entry.** `Walk::alias` normalises the
  name before the rule lookup and before it becomes the children's `ctx.parent` (`parse.rs`: `let name =
  (self.alias)(raw)`), so a `CONTACT_DATA*` block under the `_SUM` element emits `TED-AWARD_AND_CONTRACT_VALUE` and
  hits the same arm. Whether 2008 summaries put a contractor there is still unmeasured. None of the six internal-ojs
  fixtures has the element; 114382's award is `AWARD_OF_CONTRACT_SUM/ECONOMIC_OPERATOR_NAME_ADDRESS`, already
  `winner`.
- **More verbatim roles than the two this issue named.** The 511 placeholder census (`.scratch/tender-db/
  511-placeholders/census.json`, `u45/plan-dry.json`, prod reads of 2026-10-11) serves `BODY_RESPONSIBLE_APPEAL`,
  `BODY_RESPONSIBLE_MEDIATION`, `SERVICE_INFORMATION_LODGING_APPEALS` and `AWARD_CONTRACT_PI_MOVE` as roles too.
  Each is an r208 `Rule::Group`. Since only a `CONTACT_DATA*` child writes a wrapper's name as a role, each one is a
  party-bearing parent. By name, the first three are a third R2.0.8 spelling of VI.4's review
  body / mediation body / appeal-information service. The other two spellings are `APPEAL_PROCEDURE_BODY_RESPONSIBLE`
  / `MEDIATION_PROCEDURE_BODY_RESPONSIBLE` / `LODGING_INFORMATION_FOR_SERVICE` (under `PROCEDURES_FOR_APPEAL`) and
  `RESPONSIBLE_FOR_*` / `SERVICE_FROM_INFORMATION` (under `APPEAL_PROCEDURES`). They fold the same way. Their own
  parent (presumably the r208 group `PROCEDURES_APPEAL`) and their form are not read: no fixture carries them.

### The fold as built (`legacy_role`, `crates/ingest/src/project.rs`)

| element | era | was served as | now |
| --- | --- | --- | --- |
| `AWARD_AND_CONTRACT_VALUE` (and the 2008 `_SUM` via the alias) | R2.0.7/R2.0.8 F06 | itself | `winner` |
| `RESPONSIBLE_FOR_MEDIATION_PROCEDURES` | R2.0.8 VI.4 (the `RESPONSIBLE_FOR_*` spelling) | itself | `mediation-body` |
| `BODY_RESPONSIBLE_MEDIATION` | R2.0.8 VI.4 (the `BODY_RESPONSIBLE_*` spelling) | itself | `mediation-body` |
| `BODY_RESPONSIBLE_APPEAL` | R2.0.8 VI.4 (the `BODY_RESPONSIBLE_*` spelling) | itself | `review-body` |
| `SERVICE_INFORMATION_LODGING_APPEALS` | R2.0.8 VI.4 (the `BODY_RESPONSIBLE_*` spelling) | itself | `appeal-information` |
| `ADDRESS_REVIEW_INFO` | R2.0.9 VI.4.4 | `review-body` | `appeal-information` |
| `ADDRESS_REVIEW_BODY` | R2.0.9 VI.4.1 | `review-body` | `review-body` (unchanged, asserted) |

`RESPONSIBLE_FOR_MEDIATION_PROCEDURES` matches the existing spelling (`MEDIATION_PROCEDURE_BODY_RESPONSIBLE` and
`ADDRESS_MEDIATION_BODY` already fold to `mediation-body`). The canonical vocabulary is now a constant,
`LEGACY_PARTY_ROLES` (12 names, the list the docs publish). The `legacy_role` comment no longer says the sieve
catches a verbatim name: it says the sieve cannot, and names the test that does.

**Refused, verbatim on purpose** (`LEGACY_ROLE_REFUSED`, each with its reason in the code):
- `TRANSLITERATED_ADDR`: stays refused, as before. It is dropped at the parse layer since 393 unit 1.
- `ADDRESS_PARTY` (candidate 1, r209 `Rule::Org`): **refused.** It is r209-only: the R2.0.9.S03 reception schema
  (`MOVE.xsd`, T02) gives each `CONTRACTOR` an `ADDRESS_CONTRACTOR`, then `OWNERSHIP`, then 0-100 `ADDRESS_PARTY`.
  Its R2.0.8 twin `PARTY_NAME_ADDRESS` is documented as "the party or parties exercising legal control over the
  selected operator" (Regulation 1370/2007 Art. 7(3)(a)). So it is an owner of the operator: not the winner, not
  the buyer, and no canonical role means it. No fixture carries it, and no role census read so far (511's, this
  issue's bands) shows a served row. (Review round: the schema read replaces "most plausibly".)
- `PARTY_NAME_ADDRESS` (r208 group; added in the review round): **refused**, same reason as its twin. The R2.0.8.S02
  eSender schema's T02 section V.3 holds 0-99 `CONTACT_DATA_WITHOUT_RESPONSIBLE_NAME` blocks directly, so it writes
  its own name as a role; the operator itself is V.2, `ECONOMIC_OPERATOR_NAME_ADDRESS`, already `winner`. Prod
  serves it: 7 rows on 6 of 505 deadline-less 2016 `cpv=6011` tenders, every one the operator's public owner.
  5782055 (382248-2016, notice 18999280: `TED-PARTY_NAME_ADDRESS` → ORG-3 Landkreis Barnim and ORG-4 Landkreis
  Märkisch-Oderland, `TED-ECONOMIC_OPERATOR_NAME_ADDRESS` → ORG-2 Barnimer Busgesellschaft). 5804664 (427137-2016)
  has the same buyer, operator and adviser as 5466221's PI, so it reads as the award that PI announced: VGS as
  `winner`, Landratsamt Kyffhäuserkreis as the owner. Issue 259's sampled window (line 71) showed the role too.
- `AWARD_CONTRACT_PI_MOVE`: **refused, NOT a winner.** It is the R2.0.8 T01 (prior-information, Art. 7(2)) section V,
  "name and address of the selected operator". The payloads read name the operator the buyer intends to award
  directly, ahead of the award: 5466221 (221723-2015, notice 18341410: Kyffhäuserkreis names Verkehrsgesellschaft
  Südharz, `PT_DA_INTERNAL_OPERATOR`, service from 2017), 5572333 (438548-2015, Verkehrsbetriebe Hamburg-Holstein),
  5303167 (343689-2014, Vogtlandbahn). These are real operator names on single-version tenders with no award decision
  and no deadline: ~14 rows in ~180 sampled 2012-2015 transport tenders (reviewer), and 43 rows on 43 of 520
  deadline-less 2015 `cpv=6011` tenders (this round). An award intended, not made: folded to `winner` it would name a
  winner no award notice published. (Review round: replaces "unread / 2 placeholder rows".)
- `ADDRESS_NOT_MANDATORY`, `ADDRESS_NOT_STRUCT`, `USER`: **refused.** All three are r208 `Rule::Org` names with no
  party meaning of their own. The first two have been seen only inside prose, which `Rule::Text` consumes whole
  (internal-ojs 114238, `r208/oth-not-000030-2014`). `USER` is published only under `SENDER`, which `r209::rules`
  drops whole.
- Candidate 2, `SERVICE_INFORMATION_LODGING_APPEALS` (r208 group): **folded** to `appeal-information`, see above.

### Unit 2 — the eForms side

`EFORMS_ROLE_MEANING` gains `("Lot-ReviewInfo", "appeal-information")`. The test asserts that `Lot-ReviewInfo` and
`ADDRESS_REVIEW_INFO` mean the same, and that `Lot-ReviewOrg` and `ADDRESS_REVIEW_BODY` mean the same. The docs had
no mapping table, only example lists. Both role texts now name the three review meanings (see unit 6).
`role_census.rs`'s `LEGACY_ELEMENT_KINDS` comment is updated. The table is unchanged: the census still reads the
element, so its kinds do not hang on the fold. Census impact from the deploy (the census classifies the parse's field ids, not stored rows):
`RESPONSIBLE_FOR_MEDIATION_PROCEDURES`, `BODY_RESPONSIBLE_MEDIATION` and `SERVICE_INFORMATION_LODGING_APPEALS` gain
the `ReviewAdjacent` kind through `ROLE_KINDS`, and `AWARD_AND_CONTRACT_VALUE` gains `Contractor`.
`BODY_RESPONSIBLE_APPEAL` gets no census kind, because `ROLE_KINDS` has no `review-body` row and
`LEGACY_ELEMENT_KINDS` does not list the element. It had none before either. Adding it is 483's call, not done here.

### Unit 5 — the inventory walk (test variant)

`every_legacy_party_element_folds_or_is_refused` (project.rs tests):
- **Derived half:** every element of the vendored `ted-export-inventory.json` ruled `Rule::Org` that is not a
  `CONTACT_DATA*` block. Its role is its own name.
- **Declared half:** `CONTACT_DATA_WRAPPERS`, the 21 parents of a `CONTACT_DATA*` block (20, plus
  `PARTY_NAME_ADDRESS` from the review round). Each entry says where it was seen (a fixture, a served role, or a
  read schema). Each must be an inventory element and a `Group`/`Section` the walker descends into. The inventory has
  no parent edges, so this half cannot be derived.
- **Read, not wrappers:** `NOT_CONTACT_DATA_WRAPPERS` records the groups read and found to hold no `CONTACT_DATA*`
  child of their own (`AWARD_CONTRACT_MOVE`, `ADDR_CORR`, `MODIFIED_ADDRESS`), each with what was read; the test
  keeps them out of the party-bearing set.
- **The check:** each party-bearing element folds onto `LEGACY_PARTY_ROLES` or is on `LEGACY_ROLE_REFUSED`. A refused
  element must stay verbatim and give a reason. Every canonical role must be produced by some element (no dead
  vocabulary).
- **The fixture walk:** every r208, r209 and internal-ojs fixture (30) is parsed with the real walker. Every `TED-*`
  id-ref onto an `Organization` section must name a party-bearing element (prefixed F20
  `DESCRIPTION_PROCUREMENT.ADDRESS_CONTRACTOR` by its last segment) and must fold or be refused. So a new wrapper in
  any committed fixture fails the test until it is listed.

Mutation check: with `LODGING_INFORMATION_FOR_SERVICE` taken out of `CONTACT_DATA_WRAPPERS`, the test fails with
"a fixture emits the role LODGING_INFORMATION_FOR_SERVICE, which the party-bearing set lacks: add its wrapper to
CONTACT_DATA_WRAPPERS" (GATE-EXIT=101). The entry was then restored.

The limit is stated in the test's doc: a wrapper that appears in no fixture, served role or read schema is not
caught. The declared half is a hand list: the first cut missed `PARTY_NAME_ADDRESS` although issue 259 had recorded it as a served role, so a listed role in a census is not enough until someone adds it. The comments in `legacy_role` and on
`LEGACY_ROLE_REFUSED` now say "every element the walk knows", not "every party-bearing element". The served-row
detector (the unit's other variant, a data-quality line counting roles outside the vocabulary) would catch it. So
the docs keep "not yet folded".

Unit 1's carry-forward pin: `a_later_winner_replaces_the_f06_award_winners_on_the_parties` parses the real r207
F06 and asserts:
- 17 `winner` parties and no `AWARD_AND_CONTRACT_VALUE` party;
- the winner parties equal the round's winners exactly (the fixture's 279/279);
- a later chain notice publishing one `ADDRESS_CONTRACTOR` leaves exactly that winner on the parties;
- the buyer, which the later notice is silent on, still carries;
- the accumulated rounds keep all 17 F06 winners, so `lot_results` and `?winner=` keep them.

### Unit 6 — docs

- **openapi `TenderDetail.parties`:** "a winner is `winner` or `Tenderer`" is dropped. It now reads: "A `Tenderer` is a
  bidder, won or not (a losing bidder keeps the role), so do not read who won from this array: ask
  `lot_results[].winners`, or `?winner=` on `/v1/tenders`."
  - The pre-393 caveat is dropped.
  - The text names `Lot-ReviewInfo` and the three review meanings.
  - "Any other value is a source element name not yet folded" is kept.
- **`sql.rs` `tender_version_parties.role`:** the same changes, pointing at `tender_version_result_winners`
  (`v_lot_results`, `v_awards`) "as ?winner= does". It also gains the "not yet folded" sentence, which only openapi
  had, and (review round) names the three legislation roles one by one instead of "the three
  '*-legislation-information' bodies", so the list can be checked.
- **Pre-refold caveat (review round):** both texts add "Until the issue-534 refold has run, a tender projected before
  it may still serve R2.0.9's appeal-information slot (`ADDRESS_REVIEW_INFO`) as `review-body`, and a legacy award
  winner as `AWARD_AND_CONTRACT_VALUE`." The `review-body` half is not covered by "not yet folded", because
  `review-body` is canonical. Pinned positively; unit 3 removes it and moves it to RETIRED.
- **The role list is the constant (review round):** `LEGACY_PARTY_ROLES` is now a `pub const` in
  `ingest::project` (no longer test-only). The docs test cuts the list between "fold onto canonical roles:" and
  "eForms is served" on both surfaces and asserts it equals the constant exactly, with no duplicates. A 13th role
  without a docs line fails, and so does a docs role nothing folds onto.
- **`docs.rs`:** RETIRED gains "a winner is `winner` or `Tenderer`" (the openapi spelling), "winners are 'winner' OR
  'Tenderer'" (the sql.rs spelling), "a winner is <code>winner</code> or <code>Tenderer</code>" (the `/docs`
  spelling, review round: the walk uses `squash()`, which keeps tags, so the HTML form needs its own entry; a wrapped
  HTML sample is pinned against the list) and "before the issue-393 re-projection". The positive half is
  `the_role_docs_send_winner_questions_to_the_award_decisions`. It reads only the `parties[]` description and the
  role note, and pins six phrases on each: the bidder sentence, "so do not read who won from this", the
  surface's own winner pointer, the ReviewInfo meaning, "not yet folded" and (review round) the pre-refold caveat.
  It also holds each surface's legacy role list to `ingest::project::LEGACY_PARTY_ROLES` (see above).
- **`/docs` unchanged:** it has no role-vocabulary paragraph. Its two "(role `winner` or `Tenderer`)" mentions are
  about where a published name lives, not a winner equivalence, and they stay true.

### Review round (2026-10-14)

Two reviewers read the uncommitted diff. Both found the folds right for every legacy era. Their findings and what
was done:

1. **`PARTY_NAME_ADDRESS` was party-bearing and neither folded nor refused** (both reviewers). It is now in
   `CONTACT_DATA_WRAPPERS` and on `LEGACY_ROLE_REFUSED`; see the refusal list above. Read for it:
   - the R2.0.8.S02 eSender XSD (`T02_CONTRACT_MOVE.xsd`, public mirror): V.3 "name and address of the party or
     parties exercising legal control over the selected operator", 0-99 `CONTACT_DATA_WITHOUT_RESPONSIBLE_NAME`;
   - prod: 7 rows on 6 of 505 deadline-less 2016 `cpv=6011` tenders, every one the operator's public owner.

   Also read, as the reviewers asked (decisions in `NOT_CONTACT_DATA_WRAPPERS`):
   - `AWARD_CONTRACT_MOVE`: no `CONTACT_DATA*` child of its own. Per the T02 XSD its operator sits under
     `ECONOMIC_OPERATOR_NAME_ADDRESS` (already `winner`) and the owners under `PARTY_NAME_ADDRESS`. None of the 2,427
     sampled tenders serves it as a role.
   - `ADDR_CORR` and `MODIFIED_ADDRESS` (R2.0.8 F14): no `CONTACT_DATA*` child of their own. `ADDR_CORR` holds `WHERE`
     and `MODIFIED_ADDRESS`; `MODIFIED_ADDRESS` holds `CA_CE_CONCESSIONAIRE_PROFILE` and `MODIFIED_INTERNET`.
   - **Side observation, not built and unmeasured:** per that schema, an F14 that corrects an address publishes the
     new block as `CA_CE_CONCESSIONAIRE_PROFILE`. That folds to `buyer` whichever address `WHERE` names (a review
     body's, say). No F14 payload has been read.
   - **Second side observation:** a T02 MOVE award writes its operator as a `winner` party but no award decision
     (`AWARD_CONTRACT_MOVE` is a group, not a result section). So `lot_results` / `?winner=` do not name it. Seen on
     all six T02 tenders (`lot_results` empty). Not built; the same shape as 242.

   The two comments that overclaimed (`legacy_role`'s fallback and `LEGACY_ROLE_REFUSED`'s doc) now say "every
   element the walk knows", and the test doc states the limit with this miss as the example.
   **Closure check over the sample:** 2,427 `cpv=6011` tenders (2013: 487, 2014: 435, 2015: 1,000, 2016: 505) serve
   six verbatim role names: `AWARD_AND_CONTRACT_VALUE`, `BODY_RESPONSIBLE_APPEAL`,
   `RESPONSIBLE_FOR_MEDIATION_PROCEDURES` and `SERVICE_INFORMATION_LODGING_APPEALS` are now folded;
   `AWARD_CONTRACT_PI_MOVE` (121 rows) and `PARTY_NAME_ADDRESS` (7) are refused. Nothing in the sample is neither.
2. **The `AWARD_CONTRACT_PI_MOVE` reason was wrong** ("unread / 2 placeholder rows"). It now records what the payload
   shows: T01 prior-information notices naming the operator the buyer intends to award directly (5466221, 5572333,
   5303167; notice 18341410 read), with real names. It stays refused, with "NOT `winner`" in the reason. 5804664
   (same buyer, operator and adviser as 5466221) reads as the T02 award that PI announced, and there the operator is
   `winner` through `ECONOMIC_OPERATOR_NAME_ADDRESS`.
3. **Docs between deploy and unit 3:** both role texts carry the pre-refold caveat (unit 6). Unit 3 records its removal
   and its move to RETIRED.
4. **Stale wall time:** unit 3 now says ~7.5 h (job 1616, ~442 min) and a window that spans no daily tick.
5. **The HTML spelling was not retired:** RETIRED has the `<code>` form, pinned against a wrapped HTML sample.
6. **Docs not tied to the constant:** `LEGACY_PARTY_ROLES` is `pub` in `ingest::project`, and the docs test asserts
   each surface's list equals it.

Mutation checks (focused runs, each reverted, `cmp` confirming the restore):
- A 13th role `procurement-agent` added to `LEGACY_PARTY_ROLES` fails two tests: the walk ("no party-bearing
  element folds to `procurement-agent`") and the docs test ("TenderDetail.parties's legacy role list must be exactly
  ingest::project::LEGACY_PARTY_ROLES"). GATE-EXIT=101.
- The `PARTY_NAME_ADDRESS` refusal removed fails the walk ("legacy party element PARTY_NAME_ADDRESS is served as
  `PARTY_NAME_ADDRESS`, outside the canonical vocabulary"). GATE-EXIT=101.

### Focused tests (gate flags + gate package set, `--lib`)

Final tree, review round:

    CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p model -p store -p ingest \
      -p tender-db --features tender-db/server --lib -- legacy f06_award_winners role_docs retired_claim \
      role_vocabulary eforms_role the_docs_say v1::sql v1::docs
    ingest:    test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 537 filtered out
    model:     test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out
    store:     test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 184 filtered out
    tender-db: test result: ok. 40 passed; 0 failed; 0 ignored; 0 measured; 180 filtered out
    GATE-EXIT=0

Build round (before the review), the same filters without `v1::sql v1::docs`: ingest 10, store 1, tender-db 9
passed, GATE-EXIT=0.

## Re-check round 2 (2026-10-14 ~23:2x UTC)

No blocking defects. Low items fixed in place:
- the coverage comments now say the declared half is a hand list;
- the refusal-list doc says what a refusal means (no canonical role fits, or no party is named);
- the unit-3 window: start by ~22:30 UTC, or after the daily chain has drained;
- the stale queue dependencies.

The two side observations from the review round are filed as their own issues: 540 (R2.0.8 corrected address served
as `buyer`) and 541 (T02 transport awards name a `winner` party but write no award decision).

## Deployed, and unit 3 sized (2026-10-15)

- **Deployed** `4d4fe13` at 02:04 UTC, with 542 unit 1 and 543 unit 1. Health ok on that rev.
- **Sizing run 2494** (`refold-fields`, `tables: ["notice_ids"]`, the six field ids, `expect: 1`) took 531 s and
  aborted as designed: **2,006,441 notices** carry the six elements, and nothing was written. Its paired `project`
  (2495) was empty.
- **The real run** is the same request with `"expect": 2006441`. That is past `LEGACY_CLOSURE_CAP`, so the fold falls
  back to a full projection, ~7.5 h. Window rule: start it once today's (2026-10-15) daily chain has drained, so it
  spans no 07:35 UTC tick.
- **The real run started** 2026-10-15 07:47 UTC, after the daily chain drained (2502–2511 all ok, last at 07:47:12):
  `refold-fields` 2512 with `expect: 2006441`, then `project` 2513, a full-projection fallback of ~7.5 h. That ends
  well before the next 07:35 tick. When 2513 finishes: run `## Verify`, re-read band 5.2M, and move the docs caveat
  to RETIRED (unit 3's docs step).
