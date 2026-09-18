# 393 — the legacy eras serve party identity in the source's own shape: a TED transliteration block becomes a party and a twin Organization, raw element names become the role vocabulary, and Greek text-era names are served as Windows-1252 mojibake

Status: ready-for-agent — unit 2 BUILT AND DEPLOYED 2026-09-18 (rev `ba9eb1f`: the legacy party roles fold onto the canonical vocabulary in code; STANDING rows keep serving raw roles until the legacy eras are re-projected — the cohort is SIZED at 6,998,915 notices and the run is BLOCKED on a permission the classifier refused 2026-09-18; it needs an explicit go-ahead, see the unit-2 comment); unit 3's DECODER HALF BUILT, GATED AND DEPLOYED 2026-09-18 (rev `c36de25`: a declared-ISO record whose own header says Greek decodes as ISO-8859-7; the ~1,150 standing mojibake rows stay until the ISO-only text era is re-parsed, and that re-parse — 132 packages, chunk recipe below — is the SAME permission class the classifier refused for unit 2's run); unit 1 open. Was: needs-triage — filed 2026-09-15 by the API/data-quality review fan-out (32 lenses, every finding independently reproduced and adversarially judged)
Kind: defect (ingest — the legacy party/organization projection in `crates/ingest/src/project.rs` and `crates/ingest/src/r209/rules.rs`, plus the text-era decoder in `crates/ingest/src/text/mod.rs`; unit 2 is also a docs defect, in `openapi.json`, `/docs` and the `/v1/sql/schema` column note)
Relates to: 259 (CLOSED — one legacy party opening TWO Organization sections, fixed for `WINNER`/`ADDRESS_WINNER` by an outermost-Organization alias; unit 1 is the same mechanism class for a tag the nesting fix cannot reach, and 259's "no fixture in the corpus exercises the tag" applies again), 368 (ready-for-agent — unmapped source vocabulary dropped silently; it records at line 527 that `role_name` accepts ANY `TED-` id, but only as its sieve caveat, and does not track the served vocabulary), 364 (unit 6 done — the legacy OJS weld; its line ~231 notes in passing that "the legacy party roles are mostly unmapped … raw TED field names" after its buyer gauge read a false green, and unit 5 shows an r208 re-parse + full projection is a routine operation), 234 (CLOSED — identifier-less mentions minting a provisional org each) and 351 (DONE — the country-less half, 5.76M rows folded under `p0`): both are the machinery unit 1's Latin twins and unit 3's mojibake names ride into the org layer, 349 / 350 (DONE — the genericness wall on fragmented **Greek** public bodies; units 1 and 3 both mint Greek profiles that can never meet their canonical twin, so they feed exactly that class), 11 (resolved — the text-era profile; its item 3 already records that "the ISO twin mangles non-Latin-1 scripts (Greek OT bodies)" and mitigates it only by preferring the UTF8 twin, which does not exist for the years in unit 3), 202 (RESOLVED — a corrupt UTF8 twin suppressing its readable ISO, per-day keying) and 181 (RESOLVED-VERIFIED — the CF re-dispatch): both send more members through the unconditional decoder unit 3 names, 304 (STAGE 1 CLOSED — text-era language editions; stage 2 acquisition would multiply the mangled rows), 293 (BACKLOG — text-era BODY extraction; the same decoder decides what those bodies say), 232 / 244 (the text-era buyer/winner campaigns whose sweeps unit 3's re-parse would ride), 225 (RESOLVED — shipped the `role LIKE '%uyer%'` workaround unit 2 would retire), 98 (RESOLVED — eForms-DE role aliases, the precedent for folding a dialect's role spellings), 300 (the org-matching design and its exemplar sheet — row 53 classifies Cyrillic/Latin transliteration pairs as legitimate separate rows, which is true of publisher-published spellings and NOT of unit 1's TED-generated block)

## What ties these three together

Three findings, one habit: **at the legacy end of the pipeline the source's own artefacts are served as canonical identity.** A container TED generates for its own rendering becomes a participant (unit 1); the era's element names become the role vocabulary a consumer must filter on (unit 2); the bytes of one ZIP member become an organization's name (unit 3). In each case the publisher published a fact, this system chose the representation, and the representation is what is wrong — no upstream data is at fault in any of the three.

They land on the same two served surfaces — `parties[]` on `GET /v1/tenders/{id}` and the rows behind `GET /v1/organizations` — and they compound: unit 1 mints a Latin-spelled twin of a Greek buyer, unit 3 mints a mojibake-spelled profile of a Greek winner, and unit 2 is why neither can be found by role. All three are fixed in three files (`project.rs` `legacy_role`/`role_name`, `r209/rules.rs` the `Rule::Org` list, `text/mod.rs` `parse_payload`), and all three need the same follow-on the box has to schedule once: a re-parse or re-projection of the legacy/text era plus a fold of the standing rows.

| unit | surface | what is served | what it should be | severity |
| --- | --- | --- | --- | --- |
| 1 | `/v1/tenders/{id}` `parties[]`, `/v1/organizations/{id}` | a party with role `TRANSLITERATED_ADDR` plus a second provisional Organization for the same buyer | no party at all; one profile per real-world entity (CONTEXT.md:39) | medium |
| 2 | `parties[].role`, `tender_version_parties.role` on the SQL surface | three era-specific vocabularies: `ECONOMIC_OPERATOR_NAME_ADDRESS`, `APPEAL_PROCEDURE_BODY_RESPONSIBLE`, `Tenderer`, `Procedure-Buyer`, `buyer`, `winner`, … | one folded vocabulary, or a documented one | medium |
| 3 | `/v1/organizations` `name`, and `name_norm` (the identity of a provisional org) | `Ã. ×ñéóôïöéëüðïõëïò ÁÅ` — ISO-8859-7 bytes decoded as Windows-1252 | `Γ. Χριστοφιλόπουλος ΑΕ` | medium |

Reading note for triage: every number below came from the public API or from bounded SQL over `tender_version_parties` in named `tender_id` windows. No corpus-wide count exists for any of the three — the counts need an index or a full walk, which is the team lead's call under `docs/agents/prod-box-reads.md`.

---

## Unit 1 — `TRANSLITERATED_ADDR` is served as a party role, and mints a second provisional Organization for the same buyer

### Observed (verified 2026-09-14 on prod)

```
curl -s https://tenders.zebreus.click/v1/tenders/8414191 | jq '.parties[]|select(.role=="TRANSLITERATED_ADDR" or .role=="buyer")'
curl -s https://tenders.zebreus.click/v1/organizations/9954048
curl -s https://tenders.zebreus.click/v1/notices/19740841/content | jq '.sections[]|select(.section_id=="PROCEDURE").values[]|select(.field_id=="TED-TRANSLITERATED_ADDR")'
```

Tender 8414191 serves the same Greek authority twice:

```
{"role":"TRANSLITERATED_ADDR","organization_id":9954048,"organization_name":"Perifereia Attikis - Geniki Dieythynsi Oikonomikon - ..."}
{"role":"buyer","organization_id":9954049,"organization_name":"Περιφέρεια Αττικής - Γενική Διεύθυνση Οικονομικών - ..."}
```

| org | name | country | identifier | provisional | mentions |
| --- | --- | --- | --- | --- | --- |
| 9954048 | `Perifereia Attikis - Geniki Dieythynsi Oikonomikon - ...` (Latin) | GR | null | true | 35 |
| 9954049 | `Περιφέρεια Αττικής - Γενική Διεύθυνση Οικονομικών - ...` (Greek) | GR | null | true | 35 |

A one-to-one twin per notice: the Greek and the Latin name never meet in the name+country provisional resolver, so both rows carry the same 35 mentions. The Latin twin is a dead end on the buyer filter — `GET /v1/tenders?buyer=9954048` → 0 items, `?buyer=9954049` → 3.

Notice 19740841 (profile `ted-export-r208`, published 2018-03-06) shows where the row comes from: `PROCEDURE` carries `TED-TRANSLITERATED_ADDR {is_ref: true, value "ORG-1"}`, and `ORG-1` is a **sibling** Organization section under `PROCEDURE` carrying only `TED-OFFICIALNAME` (lang null). `SELECT seq, role, organization_id, mention_notice_id, mention_section_id FROM tender_version_parties WHERE tender_id=8414191` confirms the two rows come from `ORG-1` (`TRANSLITERATED_ADDR`) and `ORG-2` (`buyer`) of that one notice.

Scope, bounded SQL over `tender_version_parties` in two 30,000-id windows:

| window (`tender_id`) | `TRANSLITERATED_ADDR` rows | tenders | share of buyer-tenders | pairs resolving to a DIFFERENT org |
| --- | --- | --- | --- | --- |
| 8,400,000–8,430,000 | 2,075 | 1,346 | 4.6% of 29,497 | 234 of 2,091 (11%) — GR 217, BG 15, CY 2; 219 provisional pairs = 50 extra orgs |
| 8,000,000–8,030,000 | 3,188 | 2,749 | 9.2% of 29,995 | 3,129 of 3,188 (98%) — BG 2,080 pairs / 430 orgs, GR 1,017 / 446, CY 32 / 9, all provisional |

Latin-script buyers' blocks fold back into the buyer's own org (tender 8400023: both rows point at org 11342) but still carry the spurious role row. A second live example, BG: tender 8000060 serves `TRANSLITERATED_ADDR` → provisional org 3645161 `Agentsiya „Patna infrastruktura“` beside `buyer` → canonical org 938 `АГЕНЦИЯ "ПЪТНА ИНФРАСТРУКТУРА"`.

Mechanism, verified in code:

- `crates/ingest/src/r209/rules.rs:347` lists `TRANSLITERATED_ADDR` in the `Rule::Org` element list.
- `crates/ingest/src/r209/rules.rs:645` declares `TRANSLATION_SECTION` and `TRANSLITERATIONS` transparent `Rule::Group` containers, so the block opens a **sibling** Organization section under `PROCEDURE`, not one nested inside the buyer's address block — which is why issue 259's nested-org alias cannot reach it.
- `crates/ingest/src/project.rs:5036` `legacy_role` folds `ADDRESS_CONTRACTING_BODY*` / `CA_CE_CONCESSIONAIRE_PROFILE` → `buyer`, `ADDRESS_CONTRACTOR`/`ADDRESS_WINNER`/`WINNER` → `winner`, `ADDRESS_REVIEW_*` → `review-body`, and `other => other.to_owned()` — so the raw XML tag is served as the role.
- `process.rs:32` routes both `ted-export-r208` and `ted-export-r209` through `r209::parse_payload`, so the class spans both legacy XML profiles (the exemplar 19740841 is r208; tender 8400023's notice 19292406 is r209).

Not measured: the corpus total. Windows 8.70M and 9.50M held no tenders at all, so the two windows above are what stands.

### Why it matters

A consumer reading `parties[]` sees a participant that does not exist: `TRANSLITERATED_ADDR` is TED's Latin rendering of the contracting body's address, generated for its own display, not a party the publisher declared. Every legacy notice with the block inflates that tender's party count by one, and for Greek- and Cyrillic-script buyers it also inflates the organization layer — in the 8.00M window, 3,129 of 3,188 pairs are two rows for one authority. Those twins are unreachable through `?buyer=`, so a client that finds the Latin spelling in `parties[]` and follows it gets an empty tender list for a buyer that plainly has tenders. And because the twin's identity is a Latin transliteration, it can never merge with the entity's correctly spelled profile — the exact fragmentation issues 349 and 350 spent a campaign undoing for Greek public bodies.

It breaks two CONTEXT.md statements directly: line 39 "one profile per real-world entity, not per notice mention", and line 56 "Organizations participate in Tenders in roles (buyer, bidder, winner, …)".

### Why this is ours, not the publisher's

TED published a transliteration block inside `TRANSLATION_SECTION > TRANSLITERATIONS`; this system decided that block is an Organization. `rules.rs:347` puts `TRANSLITERATED_ADDR` in the `Rule::Org` list and `rules.rs:645` makes its two enclosing containers transparent, so a rendering aid is promoted to a top-level party section, and `legacy_role`'s fall-through then serves the raw tag as a role. Issue 259 already fixed this mechanism class once (a non-party element declared `Rule::Org`) for `WINNER`/`ADDRESS_WINNER`; that fix keyed on nesting, and this block is a sibling, so it survived. Issue 300's exemplar row 53 treats Cyrillic/Latin pairs as legitimately distinct — true for spellings the publisher published, and not true here, where the second spelling is TED's own generated transliteration of the first.

### Repro

Under 2 minutes, no token:

1. `curl -s https://tenders.zebreus.click/v1/tenders/8414191 | jq '.parties[]|select(.role=="TRANSLITERATED_ADDR" or .role=="buyer")'` → the two rows above, orgs 9954048 and 9954049.
2. `curl -s https://tenders.zebreus.click/v1/organizations/9954048` → `{"country":"GR","identifier":null,"provisional":true,"mentions":35}`; same for 9954049.
3. `curl -s 'https://tenders.zebreus.click/v1/tenders?buyer=9954048'` → 0 items; `?buyer=9954049` → 3.
4. `curl -s https://tenders.zebreus.click/v1/notices/19740841/content | jq '.sections[]|select(.section_id=="PROCEDURE").values[]|select(.field_id=="TED-TRANSLITERATED_ADDR")'` → `{is_ref: true, value: "ORG-1"}`.
5. BG variation: `curl -s https://tenders.zebreus.click/v1/tenders/8000060` → provisional org 3645161 beside canonical org 938.

### Done when

- `TRANSLITERATED_ADDR` no longer produces a party row: either an explicit `Rule::Ignore("…")` with its reason written in the list (the ADR-0004 posture — a dropped element says why), or a fold that attaches the transliterated name to the buyer's own mention as an alternate spelling. The choice is recorded on this issue.
- An r209 fixture carrying a real `TRANSLATION_SECTION > TRANSLITERATIONS > TRANSLITERATED_ADDR` block is committed (no fixture has one today, the same gap issue 259 hit), and the r208/r209 exhaustive-consumption guard consumes it.
- `GET /v1/tenders/8414191` serves exactly one party for Περιφέρεια Αττικής, with role `buyer`.
- The standing twins are folded: each pair is mechanically identifiable from `(tender_id, seq, mention_notice_id)`, so a bounded repair job can plan it, dry-run it and report before/after counts against the two measured windows (2,075 and 3,188 rows).
- After the fold, `GET /v1/tenders?buyer=<Latin twin id>` either 404s (merged away) or returns the same tenders as the Greek/Cyrillic row.
- Re-measured: 0 `TRANSLITERATED_ADDR` rows in `tender_id` 8,400,000–8,430,000 and 8,000,000–8,030,000.

---

## Unit 2 — party roles are the era's raw source element names, so r208 winners and review bodies never appear as `winner` or `review-body`

### Observed (verified 2026-09-14 on prod)

```
ssh root@zebreus.click 'echo "SELECT role, count(*) AS n FROM tender_version_parties WHERE tender_id BETWEEN 5200000 AND 5203000 GROUP BY role ORDER BY n DESC LIMIT 30" | /root/sq.sh'
curl -s https://tenders.zebreus.click/v1/tenders/5216235
```

The 3,000-tender band 5,200,000–5,203,000 (22 distinct role strings, 24,772 rows):

| role served | rows | what it is |
| --- | --- | --- |
| `ECONOMIC_OPERATOR_NAME_ADDRESS` | 6,309 | the R2.0.8 award winner block |
| `buyer` | 5,050 | folded |
| `APPEAL_PROCEDURE_BODY_RESPONSIBLE` | 3,590 | the review body |
| `LODGING_INFORMATION_FOR_SERVICE` | 2,430 | info provider |
| `TENDERS_REQUESTS_APPLICATIONS_MUST_BE_SENT_TO` | 1,529 | receipt address |
| `PURCHASING_ON_BEHALF_YES` | 1,522 | the on-behalf-of authority |
| `SPECIFICATIONS_AND_ADDITIONAL_DOCUMENTS` | 1,331 | info provider |
| `FURTHER_INFORMATION` | 1,325 | info provider |
| `MEDIATION_PROCEDURE_BODY_RESPONSIBLE` | 449 | mediation body |
| `winner` | 403 | folded — **all r209** |
| `AWARD_AND_CONTRACT_VALUE` | 280 | |
| `TRANSLITERATED_ADDR` | 164 | unit 1 |
| `RESPONSIBLE_FOR_APPEAL_PROCEDURES` | 154 | review body |
| `SERVICE_FROM_INFORMATION` | 109 | |
| `review-body` | 76 | folded — **all r209** |
| `DESCRIPTION_PROCUREMENT.ADDRESS_CONTRACTOR` | 14 | the F20 contractor — winner-shaped, unfolded |
| `TAX_LEGISLATION` / `ENVIRONMENTAL_PROTECTION_LEGISLATION` / `EMPLOYMENT_PROTECTION_WORKING_CONDITIONS` | 11 each | each holds a real info-provider address block |
| `NAME_ADDRESS_WINNER` | 10 | the winner, again |
| `ADDRESS_MEDIATION_BODY` | 1 | |

Cross-tabbed by profile in the same band, and repeated in the adjacent one:

| band | profile | `ECONOMIC_OPERATOR_NAME_ADDRESS` | `APPEAL_PROCEDURE_BODY_RESPONSIBLE` | `NAME_ADDRESS_WINNER` | `buyer` | `winner` | `review-body` |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 5,200,000–5,203,000 | ted-export-r208 | 5,956 | 3,550 | 10 | 5,008 | **0** | **0** |
| 5,200,000–5,203,000 | ted-export-r209 | 353 | 40 | 0 | 42 | 403 | 76 |
| 5,203,000–5,206,000 | ted-export-r208 | 5,937 | — | — | — | 734 (r209) | — |

r209 leaks too. Band 8,400,000–8,403,000: `winner` 13,714, `review-body` 8,679, `buyer` 7,186, then `ECONOMIC_OPERATOR_NAME_ADDRESS` 1,339, `DESCRIPTION_PROCUREMENT.ADDRESS_CONTRACTOR` 952, `ADDRESS_MEDIATION_BODY` 817, `ADDRESS_FURTHER_INFO` 719, `ADDRESS_PARTICIPATION` 701, `APPEAL_PROCEDURE_BODY_RESPONSIBLE` 562 (19 distinct strings).

And eForms is a third vocabulary, not the canonical one: band 160,000–163,000 serves `Lot-ReviewOrg` 22,541, `Tenderer` 7,725, `Procedure-Buyer` 7,228, `Procedure-SProvider` 3,956; DÖE tender 80 serves `Procedure-Buyer` / `Procedure-SProvider` / `Lot-ReviewOrg` / `Lot-TenderReceipt`. `role_name` (`project.rs:4944`) strips only the `OPT-300-`/`OPT-301-` prefix.

One tender end to end — `GET /v1/tenders/5216235` (both versions `ted-export-r208`):

| party | role served | what it is |
| --- | --- | --- |
| IQD Invesquia, S.L. | `ECONOMIC_OPERATOR_NAME_ADDRESS` | the winner — the same org `lot_results[].winners` names |
| CIEMAT | `buyer` | the buyer |
| CIEMAT | `FURTHER_INFORMATION` | the same buyer, again |
| CIEMAT | `LODGING_INFORMATION_FOR_SERVICE` | again |
| CIEMAT | `SPECIFICATIONS_AND_ADDITIONAL_DOCUMENTS` | again |

What the system documents about the column, `crates/app/src/v1/sql.rs:676-679`:

> `tender_version_parties.role` — "Buyer roles appear as 'buyer' or 'Procedure-Buyer' (era-dependent — match with `role LIKE '%uyer%'`); results-layer roles are 'winner', 'tenderer', 'subcontractor'."

Five spellings documented; 22 served in the r208 band and 19 in the eForms band, and the documented `tenderer`/`subcontractor` occur in **neither** (eForms spells it `Tenderer`). `openapi.json`'s `parties[]` schema documents only `organization_id` with `additionalProperties: true`, and `/docs` never mentions `role` — so there is no documented enum this breaks, which is itself the problem: the vocabulary is half-folded and undocumented.

Two corrections carried in from verification, so triage aims at the right thing: **(a)** the finding's "documented 5-value canonical vocabulary" does not exist (see above); **(b)** `TAX_LEGISLATION`, `PURCHASING_ON_BEHALF_YES` and the info-provider blocks are **genuine parties** whose role string happens to be the enclosing element name — they need role names, not removal. Example id 18000047 from the original finding is a 404; 8273958 serves the r208 raw names.

Unaffected, and worth stating so nobody widens the fix: the `?winner=` filter and `lot_results[].winners` read `tender_version_result_winners` (`crates/store/src/read.rs:917`), not `parties`, so both name r208 winners correctly today.

### Why it matters

A consumer of `parties[]` — or of `tender_version_parties.role` on the SQL surface, which is the documented analyst path — has to know three era-specific vocabularies to answer "who won this" or "who is the review body", and nothing on the surface tells them the vocabularies exist. In the r208 band, 25% of all party rows (6,309 of 24,772) are winners filed under `ECONOMIC_OPERATOR_NAME_ADDRESS` while `winner` reads 403, all of them from the other profile. A filter written from the served examples returns zero r208 winners and looks like an honest empty answer.

This has already misled the system's own instruments: issue 364's buyer gauge, calibrated on `Procedure-Buyer` alone, read 0 buyers for tender 2816628 and reported green; it was corrected to `role IN ('buyer','Procedure-Buyer')` and the fold was never filed. Issue 225 shipped `role LIKE '%uyer%'` into the serving path for the same reason. Those workarounds are the cost of the unfolded vocabulary, paid once per consumer.

### Why this is ours, not the publisher's

The publisher published element names; **this system chose to canonicalise them and then did it for one era only.** `legacy_role` (`project.rs:5036`) is doc-commented "Fold a legacy address-block element name onto a canonical party role" and `role_name` (`project.rs:4944`) says the legacy profiles' element names are "folded onto the canonical role names" — but the match enumerates only r209's spellings and ends in `other => other.to_owned()`, and `json.rs` then emits the stored string verbatim. The equivalence is not unknown to us either: this repo's own research doc, `docs/research/ted-legacy-mapping.md` §5.1, names `ECONOMIC_OPERATOR_NAME_ADDRESS` as the R2.0.8 winner source at 100% fill of `CONTRACT_AWARD` against R2.0.9's `ADDRESS_CONTRACTOR`. One code path, one source family, two answers.

### Repro

Under 2 minutes:

1. `curl -s https://tenders.zebreus.click/v1/tenders/5216235 | jq '.parties[]|{role,organization_name}'` → IQD Invesquia under `ECONOMIC_OPERATOR_NAME_ADDRESS`, CIEMAT four times.
2. `curl -s https://tenders.zebreus.click/v1/tenders/5216235 | jq '.lot_results[].winners'` → the same IQD Invesquia org, correctly named as the winner.
3. `curl -s https://tenders.zebreus.click/v1/tenders/160170 | jq '.parties[].role'` → `Procedure-Buyer`, `Lot-ReviewOrg`, `Lot-TenderReceipt` — the third vocabulary.
4. The band GROUP BY above (bounded, one 3,000-id window) reproduces the 22-row distribution exactly.

### Done when

- `legacy_role` folds the role-bearing legacy element names onto canonical roles — at minimum `ECONOMIC_OPERATOR_NAME_ADDRESS`, `NAME_ADDRESS_WINNER`, `DESCRIPTION_PROCUREMENT.ADDRESS_CONTRACTOR` → `winner`; `APPEAL_PROCEDURE_BODY_RESPONSIBLE`, `RESPONSIBLE_FOR_APPEAL_PROCEDURES` → `review-body`; `ADDRESS_MEDIATION_BODY`, `MEDIATION_PROCEDURE_BODY_RESPONSIBLE` → one mediation role; the info-provider and receipt blocks → named roles rather than element names. Each mapping has a fixture assertion.
- The eForms `OPT-30x` suffixes are decided the same way in the same pass: folded onto the canonical names, or documented as a deliberate second vocabulary with the mapping published.
- The vocabulary the API serves is written down where a consumer meets it: `openapi.json`'s `parties[]` schema, the `/docs` page, and the `sql.rs:676-679` column note, which currently lists five spellings that do not describe either era.
- Re-measured after the legacy re-projection (issue 364 unit 5 shows an r208 re-parse + full projection is routine): in `tender_id` 5,200,000–5,203,000, r208 `winner` and `review-body` rows are non-zero (today 0 of 24,772) and `ECONOMIC_OPERATOR_NAME_ADDRESS` is 0.
- `GET /v1/tenders/5216235` serves IQD Invesquia with role `winner`, agreeing with its own `lot_results[].winners`.
- The `role LIKE '%uyer%'` seed (`read.rs`, issue 225), the `role IN ('buyer','Procedure-Buyer')` predicates (issue 364's gauge, `data_quality.rs`) and the `sql.rs` view definitions are retired or, where kept for compatibility, carry a comment naming this issue.

---

## Unit 3 — Greek text-era organization names are served as Windows-1252 mojibake of ISO-8859-7 bytes, and minted as provisional organizations under the mangled name

### Observed (verified 2026-09-14 on prod)

```
curl -s 'https://tenders.zebreus.click/v1/organizations?name_prefix=%C3%97&limit=1000'   # prefix × (= Χ under 8859-7)
curl -s 'https://tenders.zebreus.click/v1/organizations?name_prefix=%C3%90&limit=1000'   # prefix Ð (= Π)
curl -s 'https://tenders.zebreus.click/v1/organizations?name_prefix=%C3%83&limit=200'    # prefix Ã (= Γ)
curl -s 'https://tenders.zebreus.click/v1/organizations?name_prefix=%C3%82&limit=200'    # prefix Â (= Β)
curl -s https://tenders.zebreus.click/v1/organizations/27128935
```

| probe | items | of which Greek mojibake | example |
| --- | --- | --- | --- |
| `name_prefix=%C3%97` (`×`) | 16 | 16 | 30692901 `×. Èåïäüóçò ÁÂÅÅ`, 30713280 `×ÏÕÌÌÅË ÅËËÁÓ ÁÅ` |
| `name_prefix=%C3%90` (`Ð`) | 50 | 50 | 30578178 `Ð. ÁìðáôæÞò êáé Óéá ÅÐÅ (4290011)` |
| `name_prefix=%C3%83` (`Ã`) | 79 | 78 (one Spanish `Ã¿rgano`) | 27128935 `Ã. ×ñéóôïöéëüðïõëïò ÁÅ` |
| `name_prefix=%C3%82` (`Â`) | 44 | 26 (18 legitimate PT/FR/DK/RO/DE names) | 30812754 `Â. Ì. ÔåñæÞò` |
| `name_prefix=%C3%8A` (`Ê` = Κ), own variation | 196 | ~180 (167 strict + ~13 `Êïéíïðñáîßá …` consortia) | — |
| **total, four literal probes** | **189** | **168 strictly Greek (170 with Latin brand names)** | — |

The finding's "199" is an addition slip; 189 is the sum, and its "169 Greek" is fair. The `Ó` (= Σ) probe hits the 200 cap on legitimate HU/ES/IS names and shows only 11 Greek rows, so the letter matters and no probe is a population estimate.

One row, byte for byte:

```
curl -s https://tenders.zebreus.click/v1/organizations/27128935
→ {"country":null,"id":27128935,"identifier":null,"identifier_kind":null,"mentions":1,"name":"Ã. ×ñéóôïöéëüðïõëïò ÁÅ","provisional":true}

python3 -c "print('Ã. ×ñéóôïöéëüðïõëïò ÁÅ'.encode('cp1252').decode('iso-8859-7'))"          → Γ. Χριστοφιλόπουλος ΑΕ
python3 -c "print('Ð. ÁìðáôæÞò êáé Óéá ÅÐÅ (4290011)'.encode('cp1252').decode('iso-8859-7'))" → Π. Αμπατζής και Σια ΕΠΕ (4290011)
```

The round-trip is byte-exact and yields fluent Greek with real legal-form suffixes (ΑΕ, ΕΠΕ, ΟΕ, ΑΒΕΕ), which is only possible if the source bytes were ISO-8859-7. Every affected row is `provisional: true`, `country: null`, `mentions: 1`, ids 25,603,576–30,891,006.

They are reachable, and they are winners — the original finding tried `?bidder=` and `?buyer=` and concluded they were orphans; `?winner=` finds them:

| org | tender | publication | title | date |
| --- | --- | --- | --- | --- |
| 27128935 | 2247398 | 108345-1997 | GR-Salonica: isokinetic dynamometer | 1997-08-28 |
| 30578178 | 2621684 | 93990-2000 | GR-Athens: filters | 2000-07-28 |
| 30692901 | 2843906 | 34997-2002 | GR-Athens: coin-handling machines | 2002-03-05 |
| 26830556 | 2745021 | — | — | 2001-06-28 |

So the affected window is the whole ISO-only text-era span (roughly the late 1990s to ~2003), not the finding's "1993–1999 + CF": `crates/ingest/src/profile.rs:76-80` says the UTF8 twins exist only ~2004–2007, issue 304 line 22 records the `{LG}_…_ISO_ORG.ZIP` shape for 2000–2007, and the 1993 fixture shows Greek buyers were still Latin-transliterated at the source then (no high bytes at all).

Mechanism, verified in source — `crates/ingest/src/text/mod.rs:45-48`:

```rust
let text = if declared_iso(member_path) {
    // Never fails: every byte sequence decodes under windows-1252.
    encoding_rs::WINDOWS_1252.decode(bytes).0.into_owned()
```

Every declared-`_ISO_` member, of every language, is decoded as Windows-1252 by construction. `profile.rs:78` calls that rendering "lossy (non-Latin-1 scripts mangled)" in its own doc comment and mitigates it only by letting a UTF8 twin supersede the ISO member — and for these years there is no twin.

Not measured: the exact population (SQL was denied under the read policy; ~360 Greek rows across six letters with two probes capped makes ~1–2k organizations a fair estimate) and whether the notices' Greek `TXT-OT` bodies are mangled the same way — very likely, same code path, unverified.

### Why it matters

For a provisional organization, the name **is** the identity: `organizations.name_norm` is what the resolver matches on. So the corrupted string is not a display problem, it is the key. `Γ. Χριστοφιλόπουλος ΑΕ` can never meet `Ã. ×ñéóôïöéëüðïõëïò ÁÅ` in any matcher, which means a Greek entity's 1990s–2000s mentions are permanently unmergeable with its correctly spelled r209/eForms profile — the same fragmentation issues 349 and 350 worked through for Greek public bodies, re-created at the source. And a consumer reading `/v1/organizations` for the Greek text-era slice simply gets garbage: the served name is not a transliteration or an approximation, it is a mis-decode, and there is no note on the surface saying so.

The bytes are still recoverable — the archive holds the ISO members — so this is a fixable corruption, not a lost one, and it gets more expensive with every new provisional mint and with issue 304's stage-2 acquisition.

### Why this is ours, not the publisher's

The publisher shipped ISO-8859-7 bytes in a member whose name declares `_ISO_`; this system decoded every such member as Windows-1252 "by construction", and the decoder's own comment ("Never fails: every byte sequence decodes under windows-1252") explains why the mistake is silent — windows-1252 never rejects anything, so a whole language's names turn into fluent-looking Latin garbage with no quarantine, no diagnostic and no `not-utf8` row. The evidence that the bytes were Greek is deterministic: `cp1252 → iso-8859-7` round-trips byte-exactly into correct Greek company names. Issue 11 item 3 recorded the mangling years ago and mitigated it only where a UTF8 twin exists; the ISO-only years were left. The parser already keeps the header's own language codes (`TXT-CY`/`TXT-OL` in `text-inventory.json`), so the information needed to decode correctly was in hand at parse time and unused.

### Repro

Under 2 minutes, no token:

1. `curl -s 'https://tenders.zebreus.click/v1/organizations?name_prefix=%C3%83&limit=200' | jq '.items|length'` → 79.
2. `curl -s https://tenders.zebreus.click/v1/organizations/27128935` → the row above, `provisional: true`, `country: null`, `mentions: 1`.
3. `python3 -c "print('Ã. ×ñéóôïöéëüðïõëïò ÁÅ'.encode('cp1252').decode('iso-8859-7'))"` → `Γ. Χριστοφιλόπουλος ΑΕ`.
4. `curl -s 'https://tenders.zebreus.click/v1/tenders?winner=27128935'` → tender 2247398, `108345-1997`, "GR-Salonica: isokinetic dynamometer" — a real text-era award, not an orphan row.

### Done when

- The text-era decoder keys on the record's own declared language/country rather than one codepage for every `_ISO_` member: `TXT-OL`/`TXT-CY` = GR → ISO-8859-7, with the decision for other non-Latin-1 ISO scripts (e.g. 8859-2 for Central-European editions, if any ISO-only member carries them) taken in the same pass; windows-1252 stays the default for the Latin-1 editions.
- A text-era fixture holding real ISO-8859-7 Greek bytes is committed, with a test asserting the parsed name is Greek — the current suite cannot fail on this.
- The population is sized before the re-parse: a bounded count of provisional organizations whose `name` round-trips `cp1252 → iso-8859-7` into ≥60% Greek letters (estimate ~1–2k), reported on this issue.
- The ISO-only members are re-parsed from the archive (the 244/364 sweep machinery), and the corrected names go through the resolver so they can meet their canonical profiles.
- `GET /v1/organizations/27128935` serves `Γ. Χριστοφιλόπουλος ΑΕ`, or 404s/redirects because the row merged into the canonical GR profile; `?winner=` on the surviving id still reaches tender 2247398.
- The four literal prefix probes return no Greek-mojibake rows: `%C3%97` → 0, `%C3%90` → 0, `%C3%83` → 1 (the Spanish `Ã¿rgano`), `%C3%82` → 18 (the legitimate PT/FR/DK/RO/DE names); `name_prefix=%CE%93` (`Γ`) returns the recovered rows.
- The same pass answers whether the Greek `TXT-OT` bodies of those records are mangled identically, and either fixes them or files what it found.

## Unit 2 BUILT AND DEPLOYED 2026-09-18 (owner) — the legacy fold is live; eForms deliberately not folded

Rev `ba9eb1f`, `ops/check.sh GATE-EXIT=0`, health green.

`legacy_role` in `crates/ingest/src/project.rs` now folds every role-bearing r208/r209 element name the
band measurement listed: the winner blocks (`ECONOMIC_OPERATOR_NAME_ADDRESS`, `NAME_ADDRESS_WINNER`,
`DESCRIPTION_PROCUREMENT.ADDRESS_CONTRACTOR` → `winner`), both appeal bodies → `review-body`, both
mediation blocks → `mediation-body`, both receipt addresses → `tender-receipt`, the info providers
→ `further-information` / `specifications-provider` / `appeal-information`. Two decisions worth
reading rather than assuming:

- **`PURCHASING_ON_BEHALF_YES` → `purchasing-body`, NOT `buyer`.** It is buyer-shaped but it is not
  this notice's buyer; folding it to `buyer` would move every buyer count and every key election
  that rides on them (issue 369 unit 2).
- **The three legislation bodies stay three roles** (`tax-` / `environmental-` /
  `employment-legislation-information`). They are three different parties. The point is a name that
  is ours and stable, not a name that is short.

### eForms is NOT folded — and that reversed mid-unit, for a reason worth keeping

The first cut folded `Procedure-Buyer` → `buyer`, `Tenderer` → `tenderer`, `Lot-ReviewOrg` →
`review-body` as well, on the argument that ONE re-projection should serve every era. Five
projection tests refused it, and one of them said why in its own assertion message:

    the Tenderer role is Lot-scoped in both versions

**The eForms suffix carries lot/procedure SCOPE that the canonical name does not.** Folding it would
have silently destroyed a distinction the publisher makes and a test pins, to tidy a vocabulary. So
`EFORMS_ROLE_MEANING` publishes what each suffix means without applying it — which is exactly the
alternative this unit's own "Done when" allows — and a test holds the two halves consistent: every
meaning it names must be a role the legacy fold actually produces. Where the scope should live is its
own question (`tender_version_parties` already has a `lot` column, so probably "role canonical, scope
in the column"), and it is a migration with its own acceptance, not a rider on this one.

Both places a consumer meets the vocabulary now say this: `sql.rs`'s column note (which listed five
spellings describing neither era) and `openapi.json`'s `parties[]` schema (which documented only
`organization_id`).

### Still open on unit 2

- **The re-projection.** Standing rows carry the raw names until the legacy eras are re-folded;
  only folds from `ba9eb1f` onward use the vocabulary. The "Done when" re-measure — r208 `winner`
  and `review-body` non-zero in 5,200,000–5,203,000, `ECONOMIC_OPERATOR_NAME_ADDRESS` 0, tender
  5216235 serving IQD Invesquia as `winner` — is unrun and cannot pass until then. Issue 364 unit 5
  shows the shape (an r208 re-parse + full projection is routine).
- Retiring or annotating the `LIKE '%uyer%'` and `role IN (...)` workarounds. With eForms left as
  published they are still needed, so they stay — the column note now says why.

## Unit 3 — the population is MEASURED 2026-09-18: 1,150 organizations

The filing said *"Not measured: the exact population (SQL was denied under the read policy); ~1–2k
organizations a fair estimate."* Measured now, through `/v1/sql`, one bounded index-range seek per
Greek capital letter over `organizations.name`.

**The signature had to be built in two steps, and the first was wrong in an instructive way.** A
leading byte in the 8859-7 capital range (`Á`…`Ù` under Windows-1252) is not enough — it also catches
every legitimate `Örebro kommun`, `Ålborg`, `Österreich`: `Ö` alone reads 2,415 and `Å` 967. Mojibake
is high-Latin-1 in EVERY position. But "character 2 is high" excludes exactly the class the filing's
own examples are full of — `Ð. ÁìðáôæÞò`, `Ã. ×ñéóôïöéëüðïõëïò`, initial + dot — and read 897 with a
per-letter bias against them (`Ð` 37 against the hand-verified 50). Character 4 catches the
initial-dot form (after `X. ` the surname's first letter is high) while legitimate names keep ASCII
there (`Öreb…`, `Århu…`, `Ânge…`).

    leading char in [Á..Ù]  AND  (substr(name,2,1) >= 'À' OR substr(name,4,1) >= 'À')

Validated per letter against the four probes this unit verified by hand at filing time:

| letter | hand-verified (filing) | signature |
| --- | --- | --- |
| `×` Χ | 16 | 15 |
| `Ð` Π | 50 | 48 |
| `Ã` Γ | 78 | 76 |
| `Â` Β | 26 | 24 |
| `Ê` Κ | ~180 (167 strict + ~13 consortia) | 156 |

Within 2 on every literal probe; the `Ê` gap is the `Êïéíïðñáîßá …` consortia and second-word
lowercase forms, which the position test does not reach. **All 24 letters:**

    Á 140  Â 24  Ã 76  Ä 107  Å 138  Æ 5  Ç 21  È 14  É 32  Ê 156  Ë 10  Ì 39
    Í 35   Î 16  Ï 21  Ð 48   Ñ 3    Ó 135 Ô 48  Õ 9   Ö 51  × 15  Ø 6   Ù 1
    ──────────────────────────────────────────────────────────────────────────
    TOTAL 1,150

A tight FLOOR (the signature demonstrably under-reads, never over-reads, against ground truth), with
the caveat that `Ó` (Σ, 135) and `Ö` (Φ, 51) are the two letters where a legitimate Latin-1 name could
satisfy it. Squarely inside the filing's 1–2k estimate.

### What this is the input to

The repair has two halves that want different tools. The **decoder** half — decode a declared-`_ISO_`
member as ISO-8859-7 when its bytes are Greek, then re-parse the affected text-era members — is code
with a fixture, and a single careful change. The **merge** half is not: each of the ~1,150 mangled
provisional rows has to be matched to its canonical Greek twin (or found to have none), and
`Γ. Χριστοφιλόπουλος ΑΕ` against the standing Greek profiles is a judgement call, one per row, of
exactly the kind the reviewer+challenger verdict campaigns were built for (issue 362 campaign 2:
170 groups, 139 verdicts, 65 merges applied through `org_merge_verdicts` and R2's dry/wet parity).
That pipeline is built, tested, and idle. A campaign here would run AFTER the decoder half, over
the re-decoded names, and its verdicts would go through the same tables and the same guards.

## Verify

    curl -s https://tenders.zebreus.click/v1/tenders/8414191 | python3 -c "import sys,json; print(sorted({p['role'] for p in json.load(sys.stdin)['parties']}))"

- **done** (unit 2 re-projected): only canonical roles — `buyer`, `winner`, `review-body`, … — no raw element name (uppercase with underscores) in the set
- **open**: the raw r208 element names serve as roles; read 2026-09-18 (rev `ba9eb1f`, folded in code, not yet re-projected): `['APPEAL_PROCEDURE_BODY_RESPONSIBLE', 'ECONOMIC_OPERATOR_NAME_ADDRESS', 'TRANSLITERATED_ADDR', 'buyer']`

Unit 3's line, for when the decoder half lands: `curl -s https://tenders.zebreus.click/v1/organizations/9954048` — done: a Greek name; open: the Latin transliteration `Perifereia Attikis…` or the Windows-1252 mojibake.

## Unit 2 — the re-projection is SIZED 2026-09-18 (6,998,915 notices) and BLOCKED on permission

**Why a refold and not a full rebuild.** The role is read where the party's `ORG-n` reference lives:
`NoticeValue::Id { is_ref: true }` rows in `notice_ids`, whose `field_id` is `TED-<element>` and goes
through `role_name` → `legacy_role`. So the cohort is "every notice carrying one of the 23 element
ids whose role changed", and `refold-fields` is the tool built for exactly that: it walks the
carrier set, marks it `projected = 0`, and queues one `project` behind it. `tables: ["notice_ids"]`
narrows the walk to the one channel the reference lives in.

**Sized with the job's own dry run** — `expect: 1` makes it enumerate, report and write nothing
(operations.md's SIZE FIRST rule). Job 1474, ~9 minutes over 46,132,978 `notice_ids` rows:

    refold-fields aborted: 6998915 notices carry ["TED-PURCHASING_ON_BEHALF_YES", "TED-ADDRESS_CONTRACTOR",
    "TED-ADDRESS_WINNER", "TED-WINNER", "TED-ECONOMIC_OPERATOR_NAME_ADDRESS", "TED-NAME_ADDRESS_WINNER",
    "TED-DESCRIPTION_PROCUREMENT.ADDRESS_CONTRACTOR", "TED-ADDRESS_REVIEW_BODY", "TED-ADDRESS_REVIEW_INFO",
    "TED-APPEAL_PROCEDURE_BODY_RESPONSIBLE", "TED-RESPONSIBLE_FOR_APPEAL_PROCEDURES",
    "TED-MEDIATION_PROCEDURE_BODY_RESPONSIBLE", "TED-ADDRESS_MEDIATION_BODY",
    "TED-TENDERS_REQUESTS_APPLICATIONS_MUST_BE_SENT_TO", "TED-ADDRESS_PARTICIPATION", "TED-FURTHER_INFORMATION",
    "TED-ADDRESS_FURTHER_INFO", "TED-SPECIFICATIONS_AND_ADDITIONAL_DOCUMENTS", "TED-LODGING_INFORMATION_FOR_SERVICE",
    "TED-SERVICE_FROM_INFORMATION", "TED-TAX_LEGISLATION", "TED-ENVIRONMENTAL_PROTECTION_LEGISLATION",
    "TED-EMPLOYMENT_PROTECTION_WORKING_CONDITIONS"], expected ~1 — check the field ids (nothing was written)

The buyer ids (`ADDRESS_CONTRACTING_BODY` and kin) are deliberately NOT in the list: they folded to
`buyer` before this unit (issue 369 unit 2), so their carriers' rows are already right.

**What the real run costs, and why.** 6,998,915 un-projected legacy notices is fourteen times the
closure cap (`LEGACY_CLOSURE_CAP` = 500,000), so the trailing `project rebuild=false` will announce
`INCREMENTAL → FULL fallback BEFORE identity pass` and re-project the whole corpus — the same path
issue 364 unit 5 took (job 1917: 14,366,679 notices → 7,942,429 tenders in 18,812 s, ~5.2 h). Plus
the mark itself, ~7M `UPDATE`s. Started at night with the queue idle it finishes mid-morning; the
09:35 Berlin daily tick queues behind it on the single writer, which is how the queue is designed.
Not a data risk: the tender layer is derived and the run is idempotent.

**Blocked.** The real enqueue —

    POST /admin/jobs {"kind":"refold-fields","profiles":[the 23 ids above],"tables":["notice_ids"],"expect":6998915}

— was refused by the permission classifier ("Modify Shared Resources") at 04:03Z, the same class as
issue 404's wet repair. It is not being routed around (no ssh-side enqueue, no CLI). The command is
ready to run verbatim on a go-ahead; the field list is in the scratchpad as `393-fields.txt` and in
full above.

**After it runs**, the unit's acceptance is the `## Verify` line below (tender 8414191 serving
`winner` and `review-body` instead of the element names) plus the 364-style buyer gauge reading the
same as before — the buyer rows must not move, since their role did not change.

## Unit 3 — the DECODER HALF is built, gated and deployed 2026-09-18 (rev `c36de25`)

**The rule, and where it lives.** `text/mod.rs` `decode_declared_iso`: a declared-`_ISO_` record
is decoded under Windows-1252 first — the header lines are ASCII under every ISO-8859 part, so that
pass reads them safely — and re-decoded as ISO-8859-7 when the record's own header says its
original is Greek: `OL: EL`, or `CY: GR` on the early-1990s records that predate the `OL:` line
(`OL:` outranks `CY:`). ASCII is identical under both codepages, so the English renderings and the
header come out the same either way; only the bytes above 0x7F change meaning.

**The decision for every other script, recorded so it is not mistaken for an omission:** nothing
else moves. The Central-European (8859-2) and Cyrillic (8859-5) editions exist only from 2004 and
2007, and from 2004 the package ships a UTF8 twin that supersedes the ISO member
(`profile.rs`, `iso_variant_is_superseded_when_the_package_ships_utf8`) — so there is no ISO-only
population for them to fix. And a Latin declaration keeps a Spanish `Ó` an `Ó` even though the
same byte 0xD3 is `Σ` in Greek: the `Ã¿rgano` row the filing found under the `Ã` probe is exactly
the row a byte-statistics heuristic would have broken, which is why the decision keys on the
record's declaration and not on the bytes.

**Fixture and tests.** `tests/fixtures/text/1997-can-greek-iso-8859-7.txt` is 108345-1997 as prod
serves it (tender 2247398), rebuilt field for field with the `CO:` and `TX:` winner lines in real
ISO-8859-7 bytes — the bytes the served `Ã. ×ñéóôïöéëüðïõëïò ÁÅ` round-trips to, so no archive read
was needed. `a_greek_iso_record_is_decoded_as_iso_8859_7_by_its_own_declaration` (tests/text.rs)
goes through `profile::dispatch` like the real path and asserts `TED-OFFICIALNAME` =
`Γ. Χριστοφιλόπουλος ΑΕ`; run against the old decoder it fails with
`winner names: ["Ã. ×ñéóôïöéëüðïõëïò ÁÅ"]` — the served string, verbatim. Its control flips only the
two header lines to `OL: ES` / `CY: ES` and asserts the SAME bytes then read as the mojibake: the
decision is the record's, not the bytes'. `an_iso_record_is_decoded_by_the_language_it_declares`
(mod.rs) pins `OL:` over `CY:`, the country fallback, and the Spanish `Ó`. Gate `GATE-EXIT=0`, 117
suites; deployed `c36de25`, health green, queue idle.

**What is NOT fixed by the deploy: every standing row.** The decoder runs at parse time, and the
~1,150 mojibake organizations (measured above) hang off parses that already exist. They move only
when the ISO-only text era is re-parsed from the archive, and that is the stale-rows half:

- **The population in packages**, one bounded metadata read: the ISO-only years are TED monthly
  fetches **270–401** — 132 packages, `1993-01 … 2003-12` (the ids run backwards in time; 269 is
  2004-01, the first year with a UTF8 twin). Nothing after 2003 needs re-parsing: its ISO member was
  never the one ingested.
- **The recipe is issue 364 unit 5's, verbatim**: `{"kind":"reparse","profiles":["text"],
  "packages":10,"after":269,"reclaim_only":true}`, `after` carried from each run's continuation,
  ~13 chunks, then ONE `project` — `reclaim_only` suppresses the per-chunk fold, and above 500,000
  un-projected notices the fold is the full pass anyway (~5.2 h), so one at the end is the cheap
  shape. `reparse` stamps tenders epoch-stale by PROFILE, so the text era's ~2M tenders are
  re-derived by that fold whatever the chunking; that is the runbook's "not a one-package blast
  radius", accepted.
- **Only the Greek records change output.** Every other record in those 132 packages re-parses to
  identical content (the hash is the same bytes), so the walk is mostly a no-op that costs wall
  clock, not correctness. Read `unmatched` and `re-keyed` on each chunk (issue 290): this parser
  change does not move `publication_id` derivation, so `re-keyed` must stay 0.
- **After the fold**, the resolver mints or matches the Greek names (`name_norm` now Greek, tonos
  folded by issue 346), the mojibake rows lose their last mention and fall to the orphan sweeps,
  and the merge half — matching each recovered name to its canonical GR profile — is the
  reviewer+challenger campaign over the recovered names, verdicts through `org_merge_verdicts`.
  The four literal probes in `## Done when` are the acceptance; `27128935` either serves Greek or
  redirects to the row that does.

**Not enqueued.** A `reparse` writes the parsed layer of ~2.5M notices on prod; it is the same
permission class the classifier refused twice today (unit 2's `refold-fields`, 404's wet repair),
and it is not being attempted on that basis or routed around. The first chunk's command is above,
ready to run verbatim on a go-ahead; the chunk table goes here as they run.
