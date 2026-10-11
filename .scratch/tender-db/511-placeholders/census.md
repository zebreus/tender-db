# Issue 511 — placeholder-name census (prod, 2026-10-11)

Bounded, indexed reads through `/root/sq.sh` (`/v1/sql`). **36 calls, all answered: 0 × 408, 0 truncated,
0 error bodies, slowest 2.54 s wall (ssh included).** Machine-readable twin: `census.json` (every org, its
mentions, its full role split).

## Headline

- **270 exact `name_norm` keys** probed (510's deferred list, the task's extra spellings, their case /
  punctuation / accent variants, and the real-name traps). **169 keys hold orgs: 5,114 orgs, 36,556 mentions,
  46,258 party rows.** 101 keys hold nothing (listed at the end).
- **The class sits in review and mediation slots, not in award slots:** review/mediation 38,616 party rows
  (83 %), winner/bidder 5,657 (12 %), other contact slots 1,275, buyer-side other (`purchasing-body`,
  `Procedure-CPB`) 424, **buyer 286** (`buyer` 40 + `Procedure-Buyer` 246).
- **Ten keys carry 61 % of the mentions (22,468):** `nie dotyczy` 5,721 · `no procede` 3,209 · `n/a` 3,080 ·
  `not applicable` 2,169 · `x` 1,751 · `entfällt` 1,475 · `.` 1,424 · `s.o` 1,352 · `siehe oben` 1,234 · `0` 1,053.
- **Single hubs dominate:** 1215973 `nie dotyczy` PL (5,174 mentions, 6,960 party rows: mediation 5,723, review
  1,205), 1213676 `No procede` ES (3,192; mediation 4,352), 1257377 `N/A` GB (2,129; review 1,155,
  Lot-ReviewOrg 479, mediation 438, winner 21, buyer 6).
- **The long tail is one org per mention.** Country-less copies are not reused (351's genericness wall):
  `nie dotyczy` 548 orgs (546 country-less), `not applicable` 908 (892), `ingen` 517 (512), `néant` 495 (492),
  `siehe oben` 401 (399), `entfällt` 352 (350).
- **The two existing lists cover little of this.**
  - `partyname::not_a_name` already returns `Placeholder` for **11 of the 169 keys** (stems `not applicable`,
    `see section`, `perfil del contratante`, `voir autres informations`; the whole value `various`): 2,772
    mentions.
  - 13 of `NON_NAME_FOLDS`' 15 folds hold orgs here: 18 keys, 8,412 mentions. Only `not published` and
    `not disclosed` hold none. The fold `nil` also takes the real **NIL SAS** (FR SIRET).
  - **Neither list covers 27,614 mentions (76 %).**

## Method

**Keys.** `organizations.name_norm` is `store::org_name_norm` (`canonical.rs:4568`): Unicode lowercase,
whitespace runs collapsed, trailing `.` `,` `;` stripped repeatedly. Accents and inner punctuation are kept.
So `N/A` → `n/a`, `N.A.` → `n.a`, `k.A.` → `k.a`, `Néant.` → `néant`, `./.` → `./`. Each spelling is its own key.
The keys were computed with a Python copy of that function (`keys.py`) and compared by equality only.

**Reads.** All of them are single-table and index-served. Each shape's plan was read locally before it was
sent: `target/debug/plan-probe plan` on a scratch file carrying the real DDL and indexes. That file has no
`sqlite_stat1`, which is prod's state too (issue 429's ANALYZE is not deployed).

| step | shape | calls | local `EXPLAIN QUERY PLAN` |
|---|---|---:|---|
| orgs by key | `SELECT 'k', id, name, country, identifier_kind, identifier, provisional FROM organizations WHERE name_norm = 'k'` × 39–60 arms joined by `UNION ALL` | 5 | each arm `SEARCH organizations USING INDEX organizations_name_country (name_norm=?)` |
| mentions per org | `SELECT organization_id, COUNT(*) FROM organization_mentions WHERE organization_id IN (≤400 ids) GROUP BY organization_id` | 14 | `SEARCH organization_mentions USING INDEX organization_mentions_org (organization_id=?)`, `USE SORTER FOR GROUP BY`. The same IN-chunk shape as the store's own `mention_notices_of_orgs` (510's cohort, 512-wide) |
| role split per org | `SELECT organization_id, role, COUNT(*) FROM tender_version_parties WHERE organization_id IN (≤400 ids) GROUP BY organization_id, role` | 16 | `SEARCH tender_version_parties USING INDEX tender_version_parties_org_role (organization_id=?)` (covering `(organization_id, role, tender_id)`), `USE SORTER FOR GROUP BY` |
| trap read | `SELECT organization_id, notice_id, section_id, name, country, raw_identifier, scheme FROM organization_mentions WHERE organization_id IN (12 ids)` | 1 | as for the mention counts |

- **Name keys use `UNION ALL` of equalities, not `name_norm IN (…)`.** That is the doc's cure for the turso
  IN trap, and the plan reads one seek per arm.
- **The role split departs from the brief.** The brief offered the `_mention_key` index, which would have
  needed one probe per mention (36,556). I used `tender_version_parties_org_role` instead: the same per-org seek
  as `organization_mentions_org`, covered by the index, and bounded by the org's own rows (6,960 at most).
- **The role split covers every org, not just the top ones.** The first pass took the top 265 orgs (≥ 5
  mentions, plus every trap and identifier-bearing org) and left `néant` at 49 % coverage, `ingen` at 44 % and
  `not applicable` at 58 %. The tails are single-mention orgs whose roles differ from their hubs. For example,
  `ingen`'s DK hub is mediation, but its tail is mostly winner. So the rest went in 400-wide chunks, 13 more
  calls. Every `role_split` in the JSON is complete.
- **Buckets.**
  - buyer = `buyer`, `Procedure-Buyer` (`project.rs` `BUYER_ROLES`).
  - buyer-side other = `purchasing-body` (`PURCHASING_ON_BEHALF_YES`), `Procedure-CPB`.
  - winner/bidder = `winner`, `Tenderer`, `AWARD_AND_CONTRACT_VALUE` (internal-ojs unfolded), `AWARD_CONTRACT_PI_MOVE`, `Tenderer-SubCont`.
  - review/mediation = `mediation-body`, `review-body`, `Lot-ReviewOrg`, `appeal-information`, `Lot-Mediator`, `RESPONSIBLE_FOR_MEDIATION_PROCEDURES`, `Lot-ReviewInfo`, `Part-ReviewOrg`, `Part-Mediator`, `Part-ReviewInfo`.
  - other contact = everything else (`specifications-provider`, `tender-receipt`, `further-information`, `Lot-TenderReceipt`, …).
- **fold / today columns.** `fold` is `partyname::fold` (`buyer_name_fold(match_norm(s))`), re-implemented in
  Python (`foldx.py`). The **today** column reads:
  - `P`: `partyname::not_a_name` already returns `Placeholder`. This is a re-implementation of the placeholder
    lists only; none of the keys hits a `VoidLot` entry.
  - `N`: in `role_census::NON_NAME_FOLDS`.
  - `∅`: the fold is empty, so `not_a_name` returns `None` by construction.

**Limits and caveats (no 408s and no row caps were hit).**
- **Exact values only.** No `LIKE` and no prefix reads were run, so qualified forms (`Néant (lot 3)`,
  `nie dotyczy - brak`, `N/A - see VI.4`) and spellings nobody enumerated are not counted. 510 measured its
  prefix census undercounting by about 15 % of mentions and 24 % of orgs. Expect at least that here, and more
  for review/mediation prose. A drain must enumerate by a full org walk with the predicate, as 510 did, not
  from this list.
- **Party rows exceed mentions** (46,258 against 36,556). One mention binds several roles (mediation-body plus
  review-body) or several versions or lots. The role numbers are party rows, the unit 510 used.
- **39 orgs** have mentions but no party row.
- **Comparison with 510's census.** For the same keys, 510's mediation figures read lower: `néant` 582 against
  690 here plus 69 for `neant`; `ingen` 297 against 329; `sans objet` 106 against 126. `sans objet` keeps its 2
  identifier-bearing orgs, as 510 said. The gap is a different instrument (510's census was prefix-based) and
  10 days of folds.
- **Call budget:** 36 of the ~150 allowed: 5 org reads, 14 mention counts, 16 role splits, 1 trap read. Run sequentially; no retries were needed.

## By concept group

The groups are the task's candidate list. `traps` holds only the trap keys that do not collide with a placeholder. `ingen`, `nil` and `nd` include their real-name orgs; see observation 7.

| group | keys with orgs | orgs | mentions | party rows | review/mediation | winner/bidder | buyer | buyer-side other | other contact |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| junk punctuation | 13 | 260 | 6,413 | 8,342 | 5,948 | 983 | 247 | 191 | 973 |
| nie dotyczy | 1 | 548 | 5,721 | 7,687 | 7,667 | 8 | 4 | 8 | 0 |
| n/a | 7 | 199 | 3,966 | 4,604 | 4,234 | 299 | 8 | 33 | 30 |
| keine angabe | 14 | 567 | 3,949 | 5,088 | 4,753 | 283 | 23 | 4 | 25 |
| varios | 16 | 99 | 3,494 | 4,757 | 4,511 | 240 | 0 | 1 | 5 |
| not applicable / not known / unknown | 12 | 966 | 3,086 | 3,679 | 3,566 | 80 | 3 | 20 | 10 |
| sans objet | 13 | 33 | 1,827 | 2,285 | 2,061 | 115 | 1 | 4 | 104 |
| entfällt | 4 | 363 | 1,494 | 2,006 | 1,713 | 263 | 0 | 3 | 27 |
| nl | 5 | 270 | 1,329 | 1,498 | 1,165 | 319 | 0 | 1 | 13 |
| neant | 2 | 533 | 1,073 | 1,274 | 811 | 458 | 0 | 0 | 5 |
| ingen | 2 | 518 | 925 | 948 | 348 | 600 | 0 | 0 | 0 |
| nessuno | 12 | 112 | 867 | 1,037 | 895 | 139 | 0 | 3 | 0 |
| pointers (exact) | 17 | 207 | 787 | 876 | 160 | 556 | 0 | 81 | 79 |
| various | 13 | 104 | 375 | 454 | 7 | 431 | 0 | 16 | 0 |
| brak | 2 | 34 | 297 | 430 | 417 | 13 | 0 | 0 | 0 |
| withheld | 6 | 122 | 277 | 503 | 1 | 502 | 0 | 0 | 0 |
| none | 4 | 50 | 179 | 192 | 135 | 52 | 0 | 5 | 0 |
| diverse | 9 | 32 | 127 | 174 | 0 | 120 | 0 | 54 | 0 |
| aucun | 2 | 77 | 113 | 114 | 12 | 100 | 0 | 0 | 2 |
| traps | 3 | 4 | 73 | 77 | 0 | 77 | 0 | 0 | 0 |
| pt | 2 | 2 | 64 | 89 | 86 | 1 | 0 | 0 | 2 |
| sv/da/no | 3 | 4 | 61 | 79 | 78 | 1 | 0 | 0 | 0 |
| el | 2 | 4 | 38 | 44 | 44 | 0 | 0 | 0 | 0 |
| fi | 2 | 3 | 16 | 16 | 0 | 16 | 0 | 0 | 0 |
| ro | 1 | 1 | 3 | 3 | 3 | 0 | 0 | 0 | 0 |
| hu | 1 | 1 | 1 | 1 | 0 | 1 | 0 | 0 | 0 |
| hr/sl/cs/sk/lt | 1 | 1 | 1 | 1 | 1 | 0 | 0 | 0 | 0 |

## By fold (the unit a `partyname` whole value would match), folds with 50 or more mentions

| fold | name_norm keys | orgs | mentions | review/med | winner/bidder | buyer | buyer-side | other | in NON_NAME_FOLDS |
|---|---|---:|---:|---:|---:|---:|---:|---:|---|
| `nie dotyczy` | `nie dotyczy` | 548 | 5,721 | 7,667 | 8 | 4 | 8 | 0 |  |
| `n a` | `n/a`, `n.a`, `n a`, `n/ a`, `n /a`, `(n/a)` | 38 | 3,423 | 3,717 | 145 | 7 | 30 | 24 | yes |
| `no procede` | `no procede` | 3 | 3,209 | 4,420 | 14 | 0 | 0 | 5 |  |
| (empty) | `.`, `-`, `---`, `--`, `/`, `*` … | 65 | 2,804 | 1,850 | 701 | 246 | 181 | 963 |  |
| `not applicable` | `not applicable` | 908 | 2,169 | 2,626 | 9 | 1 | 0 | 6 | yes |
| `x` | `x` | 71 | 1,751 | 2,246 | 89 | 1 | 0 | 4 | yes |
| `entfallt` | `entfällt`, `-entfällt-`, `- entfällt -` | 362 | 1,489 | 1,713 | 258 | 0 | 3 | 27 |  |
| `s o` | `s.o`, `s/o` | 6 | 1,353 | 1,690 | 0 | 1 | 0 | 26 |  |
| `siehe oben` | `siehe oben` | 401 | 1,234 | 1,424 | 4 | 2 | 0 | 23 |  |
| `neant` | `néant`, `neant` | 533 | 1,073 | 811 | 458 | 0 | 0 | 5 |  |
| `0` | `0` | 33 | 1,053 | 1,122 | 41 | 0 | 7 | 0 |  |
| `ingen` | `ingen` | 517 | 924 | 347 | 600 | 0 | 0 | 0 |  |
| `keine angabe` | `keine angabe` | 4 | 792 | 817 | 238 | 21 | 0 | 0 |  |
| `keine` | `keine` | 24 | 751 | 1,000 | 8 | 0 | 4 | 2 |  |
| `n v t` | `n.v.t` | 65 | 688 | 709 | 72 | 0 | 0 | 4 |  |
| `xxx` | `xxx` | 79 | 656 | 600 | 123 | 0 | 3 | 6 |  |
| `na` | `na` | 161 | 543 | 517 | 154 | 1 | 3 | 6 | yes |
| `nicht vorhanden` | `nicht vorhanden` | 3 | 440 | 583 | 0 | 0 | 0 | 0 |  |
| `n d` | `n/d`, `n.d` | 21 | 375 | 458 | 3 | 0 | 0 | 0 |  |
| `tbc` | `tbc` | 26 | 349 | 369 | 25 | 0 | 8 | 2 |  |
| `niet van toepassing` | `niet van toepassing` | 100 | 313 | 223 | 117 | 0 | 0 | 3 |  |
| `brak` | `brak` | 33 | 296 | 417 | 12 | 0 | 0 | 0 |  |
| `nicht bekannt` | `nicht bekannt` | 1 | 291 | 335 | 3 | 0 | 0 | 0 |  |
| `nd` | `nd` | 4 | 265 | 311 | 16 | 0 | 2 | 0 |  |
| `sans objet` | `sans objet` | 5 | 223 | 146 | 87 | 0 | 1 | 28 |  |
| `no` | `no` | 5 | 196 | 215 | 17 | 0 | 0 | 0 |  |
| `ver perfil del contratante` | `ver perfil del contratante` | 176 | 177 | 1 | 177 | 0 | 0 | 0 |  |
| `nvt` | `nvt` | 5 | 176 | 185 | 17 | 0 | 1 | 6 |  |
| `see section vi 3 additional information` | `see section vi.3 additional information`, `see section vi.3) additional information` | 4 | 168 | 0 | 166 | 0 | 1 | 1 |  |
| `keine angaben` | `keine angaben` | 115 | 162 | 278 | 9 | 0 | 0 | 0 |  |
| `to be confirmed` | `to be confirmed` | 5 | 159 | 167 | 2 | 2 | 5 | 2 |  |
| `geen` | `geen` | 98 | 150 | 48 | 110 | 0 | 0 | 0 |  |
| `xx` | `xx` | 12 | 149 | 130 | 29 | 0 | 0 | 0 |  |
| `k a` | `k.a`, `k. a` | 4 | 149 | 182 | 4 | 0 | 0 | 0 |  |
| `various` | `various` | 4 | 146 | 2 | 147 | 0 | 9 | 0 | yes |
| `none` | `none` | 44 | 141 | 130 | 23 | 0 | 3 | 0 | yes |
| `withheld section94 supplier` | `withheld section94 supplier` | 2 | 125 | 0 | 347 | 0 | 0 | 0 |  |
| `varios` | `varios`, `vários` | 78 | 119 | 0 | 121 | 0 | 1 | 0 |  |
| `tba` | `tba` | 5 | 118 | 110 | 15 | 0 | 1 | 0 |  |
| `confidential` | `confidential` | 115 | 116 | 0 | 117 | 0 | 0 | 0 | yes |
| `see contracts finder notice for full supplier list` | `see contracts finder notice for full supplier list` | 1 | 115 | 0 | 127 | 0 | 0 | 0 |  |
| `aucun` | `aucun` | 76 | 111 | 12 | 100 | 0 | 0 | 0 |  |
| `see additional information` | `see additional information` | 4 | 84 | 1 | 52 | 0 | 28 | 23 |  |
| `diverse` | `diverse` | 22 | 81 | 0 | 77 | 0 | 46 | 0 |  |
| `various suppliers` | `various suppliers` | 77 | 81 | 0 | 84 | 0 | 0 | 0 |  |
| `non renseigne` | `non renseigné` | 2 | 79 | 55 | 1 | 0 | 0 | 45 |  |
| `nicht zutreffend` | `nicht zutreffend` | 4 | 77 | 82 | 6 | 0 | 0 | 0 |  |
| `see section vi 3` | `see section vi.3`, `see section vi.3)` | 5 | 74 | 33 | 6 | 0 | 28 | 29 |  |
| `non previsto` | `non previsto` | 23 | 66 | 52 | 22 | 0 | 0 | 0 |  |
| `ikke relevant` | `ikke relevant` | 2 | 59 | 75 | 1 | 0 | 0 | 0 |  |
| `see vi 3` | `see vi.3` | 4 | 57 | 27 | 0 | 0 | 18 | 22 |  |
| `nao aplicavel` | `não aplicável` | 1 | 55 | 75 | 1 | 0 | 0 | 2 |  |
| `so` | `so` | 3 | 54 | 56 | 0 | 0 | 0 | 0 |  |
| `non applicabile` | `non applicabile` | 2 | 50 | 60 | 0 | 0 | 0 | 0 |  |

## What the numbers say for unit 1 (observations, not decisions)

1. **"Nobody" in a winner slot reads as no award, not as a withheld winner.** In the winner/bidder bucket,
   the deferred list splits into two semantic classes:
   - **"none / nothing":** `ingen` 600 winner rows (of 947), `néant` + `neant` 458 (of 1,274), `geen` 110,
     `niet van toepassing` 117, `aucun` 100, `nessuno` 29, `none` 23, `n/a` family 145. That is "nobody was
     awarded", which is closer to `VoidLot` than to the issue's "a winner exists, its name is elsewhere".
   - **"elsewhere / withheld / many":** `Withheld Section94 Supplier` 345 `Tenderer` rows, `See Section VI.3
     Additional information` 166, `ver perfil del contratante` 177, `See Contracts Finder Notice for full
     supplier list` 127, `various` 147, `Multiple Suppliers` 82, `Multiple Providers` 53, `Diverse` 64,
     `varias empresas` 39, `Vários` 33. This is the issue's class proper: `selec-w` with no winner bound.

   So a single `Placeholder` class with one semantics would misread the first kind.
2. **In review and mediation slots, every key means the same thing: there is no such body.** That is 38,616
   rows, 83 % of the census. No winner, result or buyer token depends on them, so dropping them changes only
   party rows. Two hubs hold 23 % of the census's mentions on their own: `nie dotyczy` PL (5,174) and
   `No procede` ES (3,192).
3. **Buyer slots.** 286 rows across 14 orgs, plus 424 buyer-side other rows:
   - **11063 `.` AT, `Procedure-Buyer` × 241** (241 mentions: an eForms AT buyer whose name was published as
     `.`). It accounts for 84 % of the buyer rows.
   - The rest: 36188 `Keine Angabe.` (country-less) 21; 1257377 `N/A` GB 6; 1215973 `nie dotyczy` PL 4;
     1164345 `-` 3; `To be confirmed` GB 2; `siehe oben` DE 2; seven more with 1 each.

   Mapping or dropping these touches 286 buyer rows, 241 of them on that one AT `.` org.
4. **Punctuation-only names fold to an empty string, and `not_a_name` returns `None` for that by
   construction.** `.`, `-`, `--`, `---`, `/`, `*`, `-/-`, `./`, `?` together hold 2,804 mentions: 701
   winner/bidder rows, **246 buyer rows** (241 of them the AT `.`), 963 other-contact rows, 181 buyer-side other rows. A `Placeholder` rule built on
   the fold cannot see them. It needs its own "no alphanumeric character" arm. `x`, `xx`, `xxx` and `0` fold to
   themselves: 3,609 mentions, mostly mediation and review.
5. **One abbreviation is two languages.**
   - The `s o` fold: 1321632 `s.o.` DE (1,075 mentions) and the AT/DE copies are German *siehe oben* ("see
     above"). `SO` / `s.o` / `S/O` FR are French *sans objet*. Both are placeholders, but a reason or a
     language label must not assume French.
   - `nd` is Polish `nd.` (*nie dotyczy*, 226 mentions on 6425603) and also the real HR sole trader 14319858.
6. **Identifiers do not separate real names from placeholders.** 14 orgs carry one:
   - Junk with an id: **30914328 `Withheld Section94 Supplier`, PPON `GB-PPON-PBXY-3232-WNHZ` on all 123
     mentions** (345 `Tenderer` rows: the FTS publisher minted a PPON for the withheld placeholder);
     23018279 `Nicht vorhanden` DE `T04089980` (32 mentions, `Lot-Mediator` 46); 23307388 `keine` with a
     review-chamber string as its id; 24291539 `-` and 24291540 `.` DE (UUID ids); 23186509 `Keine Angabe`
     DE VAT; 22780502 / 23153045 `sans objet` FR; 23424157 `Various` IE `10003`; 22519580 `Nvt` NL;
     22525406 `No hay` ES.
   - Real companies: 16700736 `NIL` FR (SIRET 32511405600055; its mentions read `nettoyage industriel du
     littoral`, `NIL SAS`) and 14319858 `ND` HR (OIB 19645926433; `ND, OBRT ZA TRGOVINU, VL. NENA PLETIKOSA`).
   - Unknown: 23095417 `0` DK (CVR-shaped 38160540, 1 mention).

   **The trade-off.**
   - An identifier guard would protect the two real companies, since both carry an id. It would also keep
     the biggest withheld winner (Section94: 123 mentions, 345 rows) and the 10 junk orgs with ids.
   - Taking **`nil`** (which is in `NON_NAME_FOLDS` today) and **`nd`** off a whole-value list also protects
     them. That costs the junk they share a fold with: Polish `nd.`, 226 mentions on 6425603, and `nil`'s
     24 other mentions.
7. **The real-name traps, measured.** Every row they hold is `winner` or `Tenderer`.

   | org | name | country | mentions | roles | note |
   |---|---|---|---:|---|---|
   | 25263107, 28851107 | `InGen` | – | 1 each | winner 1 each | **same `name_norm` and fold as `ingen`**: only case tells them apart |
   | 24742909 / 31530567 | `NIL d.o.o.` | – / SI | 23 / 1 | winner 25 / 1 | fold `nil d o o` ≠ `nil`: safe under whole-value matching |
   | 12290440 | `N/A s. r. o.` | SK | 2 | winner 4 | fold `n a s r o` ≠ `n a`: safe under whole-value matching |
   | 8379831 | `Diverse Care Services Ltd` | GB | 47 | winner 47 | `diverse care services` and `… limited` absent |
   | 16700736 | `NIL` (NIL SAS) | FR | 9 | Tenderer 6, winner 3 | **fold `nil`, which is in `NON_NAME_FOLDS`** |
   | 14319858 | `ND` | HR | 8 | winner 12, Tenderer 2 | fold `nd`, the same as Polish `nd.` |
   | — | `Various Ltd` / `Various Limited` | | 0 | | absent |

8. **Unresolved: 1188404 `Ingen` FR.** 134 mentions, all FR, all `winner` (137 rows), on notices
   4354963 … 27178323. The spellings `Ingen` 69, `INGEN` 40, `ingen` 24 (the lowercase use argues for the
   Scandinavian "none"). Not read further: telling a French firm called *Ingen* from a Scandinavian "none"
   needs the notices, and that is a `notice_*` read outside this brief. It accounts for 137 of `ingen`'s 600
   winner rows. Every other `ingen` org is DK, SE, FI, NO or country-less.
9. **Folds collapse spellings, which is what a whole-value list wants.** `n a` takes six `name_norm` keys
   (3,423 mentions), `entfallt` three, `s o` two, `n d` two, `k a` two. The fold table above is the size of
   each candidate whole value.

## Every key with orgs (169), by mentions

`today`: P = already `Placeholder` in `partyname`, N = in `NON_NAME_FOLDS`, ∅ = empty fold. Role columns are party rows.

| name_norm key | group | fold | orgs | w/ id | mentions | review/med | winner/bidder | buyer | buyer-side | other | today | top org: id `name` country mentions |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---|---|
| `nie dotyczy` | nie dotyczy | `nie dotyczy` | 548 | 0 | 5,721 | 7,667 | 8 | 4 | 8 | 0 |  | 1215973 `nie dotyczy` PL 5,174 |
| `no procede` | varios | `no procede` | 3 | 0 | 3,209 | 4,420 | 14 | 0 | 0 | 5 |  | 1213676 `No procede` ES 3,192 |
| `n/a` | n/a | `n a` | 23 | 0 | 3,080 | 3,324 | 138 | 7 | 30 | 24 | N | 1257377 `N/A` GB 2,129 |
| `not applicable` | not applicable / not known / unknown | `not applicable` | 908 | 0 | 2,169 | 2,626 | 9 | 1 | 0 | 6 | P N | 2147121 `not applicable` GB 1,005 |
| `x` | junk punctuation | `x` | 71 | 0 | 1,751 | 2,246 | 89 | 1 | 0 | 4 | N | 8631351 `x` PL 910 |
| `entfällt` | entfällt | `entfallt` | 352 | 0 | 1,475 | 1,705 | 251 | 0 | 3 | 27 |  | 1765331 `entfällt` DE 1,118 |
| `.` | junk punctuation | (empty) | 18 | 1 | 1,424 | 824 | 241 | 241 | 153 | 509 | ∅ | 1191345 `.` - 446 |
| `s.o` | sans objet | `s o` | 5 | 0 | 1,352 | 1,689 | 0 | 1 | 0 | 26 |  | 1321632 `s.o.` DE 1,075 |
| `siehe oben` | keine angabe | `siehe oben` | 401 | 0 | 1,234 | 1,424 | 4 | 2 | 0 | 23 |  | 1350611 `siehe oben` DE 650 |
| `0` | junk punctuation | `0` | 33 | 1 | 1,053 | 1,122 | 41 | 0 | 7 | 0 |  | 1528718 `0` IT 865 |
| `néant` | neant | `neant` | 495 | 0 | 963 | 729 | 410 | 0 | 0 | 2 |  | 1166840 `néant` FR 272 |
| `ingen` | ingen | `ingen` | 517 | 0 | 924 | 347 | 600 | 0 | 0 | 0 |  | 1889418 `Ingen` DK 274 |
| `-` | junk punctuation | (empty) | 23 | 1 | 909 | 644 | 309 | 4 | 16 | 380 | ∅ | 9633 `-` DE 423 |
| `keine angabe` | keine angabe | `keine angabe` | 4 | 1 | 792 | 817 | 238 | 21 | 0 | 0 |  | 3992287 `keine Angabe.` DE 723 |
| `keine` | keine angabe | `keine` | 24 | 1 | 751 | 1,000 | 8 | 0 | 4 | 2 |  | 2391661 `keine` DE 670 |
| `n.v.t` | nl | `n v t` | 65 | 0 | 688 | 709 | 72 | 0 | 0 | 4 |  | 1166349 `n.v.t.` NL 609 |
| `xxx` | junk punctuation | `xxx` | 79 | 0 | 656 | 600 | 123 | 0 | 3 | 6 |  | 1669376 `xxx` DE 379 |
| `na` | n/a | `na` | 161 | 0 | 543 | 517 | 154 | 1 | 3 | 6 | N | 2111501 `NA` GB 182 |
| `nicht vorhanden` | keine angabe | `nicht vorhanden` | 3 | 1 | 440 | 583 | 0 | 0 | 0 | 0 |  | 8403085 `Nicht vorhanden` DE 406 |
| `n/d` | nessuno | `n d` | 20 | 0 | 367 | 452 | 0 | 0 | 0 | 0 |  | 8627968 `n/d` PL 207 |
| `tbc` | not applicable / not known / unknown | `tbc` | 26 | 0 | 349 | 369 | 25 | 0 | 8 | 2 |  | 1919927 `TBC` GB 305 |
| `n.a` | n/a | `n a` | 10 | 0 | 337 | 387 | 7 | 0 | 0 | 0 | N | 7757651 `N.A.` IT 218 |
| `niet van toepassing` | nl | `niet van toepassing` | 100 | 0 | 313 | 223 | 117 | 0 | 0 | 3 |  | 1220397 `Niet van toepassing` NL 156 |
| `brak` | brak | `brak` | 33 | 0 | 296 | 417 | 12 | 0 | 0 | 0 |  | 3013768 `brak` PL 264 |
| `nicht bekannt` | keine angabe | `nicht bekannt` | 1 | 0 | 291 | 335 | 3 | 0 | 0 | 0 |  | 3307720 `nicht bekannt` DE 291 |
| `---` | junk punctuation | (empty) | 5 | 0 | 276 | 196 | 107 | 0 | 11 | 53 | ∅ | 1173702 `---` - 173 |
| `nd` | nessuno | `nd` | 4 | 1 | 265 | 311 | 16 | 0 | 2 | 0 |  | 6425603 `nd.` PL 226 |
| `sans objet` | sans objet | `sans objet` | 5 | 2 | 223 | 146 | 87 | 0 | 1 | 28 |  | 1173463 `Sans objet` FR 130 |
| `no` | not applicable / not known / unknown | `no` | 5 | 0 | 196 | 215 | 17 | 0 | 0 | 0 |  | 11066035 `NO` IT 167 |
| `ver perfil del contratante` | pointers (exact) | `ver perfil del contratante` | 176 | 0 | 177 | 1 | 177 | 0 | 0 | 0 | P | 4282883 `Ver perfil del contratante` ES 2 |
| `nvt` | nl | `nvt` | 5 | 1 | 176 | 185 | 17 | 0 | 1 | 6 |  | 1219856 `NVT` NL 142 |
| `keine angaben` | keine angabe | `keine angaben` | 115 | 0 | 162 | 278 | 9 | 0 | 0 | 0 |  | 2922740 `keine Angaben` DE 44 |
| `to be confirmed` | not applicable / not known / unknown | `to be confirmed` | 5 | 0 | 159 | 167 | 2 | 2 | 5 | 2 |  | 1942945 `To be confirmed` GB 141 |
| `geen` | nl | `geen` | 98 | 0 | 150 | 48 | 110 | 0 | 0 | 0 |  | 10006727 `Geen` BE 42 |
| `xx` | junk punctuation | `xx` | 12 | 0 | 149 | 130 | 29 | 0 | 0 | 0 |  | 9341967 `xx` DK 58 |
| `various` | various | `various` | 4 | 1 | 146 | 2 | 147 | 0 | 9 | 0 | P N | 1173051 `Various` GB 105 |
| `see section vi.3 additional information` | pointers (exact) | `see section vi 3 additional information` | 3 | 0 | 144 | 0 | 142 | 0 | 1 | 1 | P | 10601576 `See Section VI.3 Additional information` LU 140 |
| `none` | none | `none` | 44 | 0 | 141 | 130 | 23 | 0 | 3 | 0 | N | 1357376 `None` GB 58 |
| `k.a` | keine angabe | `k a` | 3 | 0 | 133 | 165 | 4 | 0 | 0 | 0 |  | 9713865 `k.A.` DE 126 |
| `withheld section94 supplier` | withheld | `withheld section94 supplier` | 2 | 1 | 125 | 0 | 347 | 0 | 0 | 0 |  | 30914328 `Withheld Section94 Supplier` GB 123 |
| `tba` | not applicable / not known / unknown | `tba` | 5 | 0 | 118 | 110 | 15 | 0 | 1 | 0 |  | 2571521 `TBA` GB 102 |
| `confidential` | withheld | `confidential` | 115 | 0 | 116 | 0 | 117 | 0 | 0 | 0 | N | 13606939 `Confidential` GB 2 |
| `see contracts finder notice for full supplier list` | pointers (exact) | `see contracts finder notice for full supplier list` | 1 | 0 | 115 | 0 | 127 | 0 | 0 | 0 |  | 30918120 `See Contracts Finder Notice for full supplier list` GB 115 |
| `aucun` | aucun | `aucun` | 76 | 0 | 111 | 12 | 100 | 0 | 0 | 0 |  | 1581670 `aucun` FR 34 |
| `neant` | neant | `neant` | 38 | 0 | 110 | 82 | 48 | 0 | 0 | 3 |  | 1702621 `neant` FR 57 |
| `varios` | varios | `varios` | 76 | 0 | 84 | 0 | 87 | 0 | 0 | 0 |  | 1500956 `Varios` ES 9 |
| `see additional information` | pointers (exact) | `see additional information` | 4 | 0 | 84 | 1 | 52 | 0 | 28 | 23 |  | 2126826 `See additional information` GB 63 |
| `--` | junk punctuation | (empty) | 6 | 0 | 83 | 101 | 14 | 1 | 0 | 2 | ∅ | 7358415 `--` DE 66 |
| `diverse` | diverse | `diverse` | 22 | 0 | 81 | 0 | 77 | 0 | 46 | 0 |  | 1574187 `Diverse` DE 60 |
| `various suppliers` | various | `various suppliers` | 77 | 0 | 81 | 0 | 84 | 0 | 0 | 0 |  | 2231054 `Various Suppliers` GB 5 |
| `non renseigné` | sans objet | `non renseigne` | 2 | 0 | 79 | 55 | 1 | 0 | 0 | 45 |  | 4353667 `non renseigné` FR 70 |
| `nicht zutreffend` | keine angabe | `nicht zutreffend` | 4 | 0 | 77 | 82 | 6 | 0 | 0 | 0 |  | 2055170 `nicht zutreffend` DE 67 |
| `non previsto` | nessuno | `non previsto` | 23 | 0 | 66 | 52 | 22 | 0 | 0 | 0 |  | 8562558 `Non previsto` IT 44 |
| `see section vi.3` | pointers (exact) | `see section vi 3` | 2 | 0 | 65 | 29 | 5 | 0 | 20 | 29 | P | 2096920 `See section VI.3` GB 46 |
| `ikke relevant` | sv/da/no | `ikke relevant` | 2 | 0 | 59 | 75 | 1 | 0 | 0 | 0 |  | 4643771 `Ikke relevant` DK 45 |
| `see vi.3` | pointers (exact) | `see vi 3` | 4 | 0 | 57 | 27 | 0 | 0 | 18 | 22 |  | 1773631 `See VI.3` GB 38 |
| `não aplicável` | pt | `nao aplicavel` | 1 | 0 | 55 | 75 | 1 | 0 | 0 | 2 |  | 5027859 `Não aplicável` PT 55 |
| `so` | sans objet | `so` | 3 | 0 | 54 | 56 | 0 | 0 | 0 | 0 |  | 7958721 `SO` FR 51 |
| `non applicabile` | nessuno | `non applicabile` | 2 | 0 | 50 | 60 | 0 | 0 | 0 | 0 |  | 11089135 `non applicabile` IT 46 |
| `diverse care services ltd` | traps | `diverse care services ltd` | 1 | 0 | 47 | 0 | 47 | 0 | 0 | 0 |  | 8379831 `Diverse Care Services Ltd` GB 47 |
| `multiple suppliers` | various | `multiple suppliers` | 2 | 0 | 46 | 0 | 87 | 0 | 0 | 0 |  | 11378076 `Multiple Suppliers` GB 42 |
| `/` | junk punctuation | (empty) | 4 | 0 | 46 | 27 | 19 | 0 | 0 | 6 | ∅ | 1992065 `/` - 29 |
| `nessuno` | nessuno | `nessuno` | 32 | 0 | 44 | 14 | 29 | 0 | 0 | 0 |  | 10633068 `Nessuno` IT 13 |
| `multiple providers` | various | `multiple providers` | 2 | 0 | 43 | 0 | 62 | 0 | 0 | 0 |  | 11579964 `Multiple Providers` GB 38 |
| `see below` | pointers (exact) | `see below` | 3 | 0 | 43 | 46 | 4 | 0 | 0 | 2 |  | 1192030 `See Below` GB 39 |
| `not provided` | not applicable / not known / unknown | `not provided` | 2 | 0 | 39 | 34 | 6 | 0 | 0 | 0 | N | 1192451 `Not Provided` GB 31 |
| `ditte varie` | nessuno | `ditte varie` | 23 | 0 | 39 | 0 | 41 | 0 | 0 | 0 |  | 9780371 `Ditte varie` IT 17 |
| `vários` | varios | `varios` | 2 | 0 | 35 | 0 | 34 | 0 | 1 | 0 |  | 7382588 `Vários` PT 34 |
| `varios adjudicatarios` | varios | `varios adjudicatarios` | 2 | 0 | 35 | 0 | 37 | 0 | 0 | 0 |  | 1345326 `Varios adjudicatarios` - 21 |
| `δεν εφαρμόζεται` | el | `δεν εφαρμοζεται` | 3 | 0 | 35 | 41 | 0 | 0 | 0 | 0 |  | 13318327 `Δεν εφαρμόζεται` GR 28 |
| `*` | junk punctuation | (empty) | 6 | 0 | 35 | 25 | 11 | 0 | 1 | 0 | ∅ | 1456750 `*` - 12 |
| `nil` | none | `nil` | 3 | 1 | 33 | 0 | 28 | 0 | 0 | 0 | N | 28790764 `NIL` - 13 |
| `sans` | sans objet | `sans` | 2 | 0 | 33 | 37 | 3 | 0 | 0 | 0 |  | 16605391 `Sans` FR 30 |
| `ninguno` | varios | `ninguno` | 2 | 0 | 33 | 35 | 2 | 0 | 0 | 0 |  | 8024789 `Ninguno` ES 31 |
| `no aplica` | varios | `no aplica` | 1 | 0 | 32 | 41 | 0 | 0 | 0 | 0 |  | 9015545 `No aplica` ES 32 |
| `see section vi.4.1` | pointers (exact) | `see section vi 4 1` | 2 | 0 | 32 | 40 | 0 | 0 | 0 | 0 | P | 10155822 `See Section VI.4.1` GB 26 |
| `nc` | sans objet | `nc` | 2 | 0 | 31 | 42 | 0 | 0 | 0 | 0 |  | 7228557 `NC` FR 30 |
| `varias empresas` | varios | `varias empresas` | 2 | 0 | 31 | 0 | 46 | 0 | 0 | 0 |  | 7281280 `varias empresas` ES 24 |
| `./` | junk punctuation | (empty) | 1 | 0 | 27 | 27 | 0 | 0 | 0 | 12 | ∅ | 1958624 `./.` - 27 |
| `unknown` | not applicable / not known / unknown | `unknown` | 5 | 0 | 26 | 21 | 1 | 0 | 4 | 0 | N | 11909158 `Unknown` GB 11 |
| `see section vi.3) additional information` | pointers (exact) | `see section vi 3 additional information` | 1 | 0 | 24 | 0 | 24 | 0 | 0 | 0 | P | 12663778 `See Section VI.3) Additional information` LU 24 |
| `nil d.o.o` | traps | `nil d o o` | 2 | 0 | 24 | 0 | 26 | 0 | 0 | 0 |  | 24742909 `NIL d.o.o` - 23 |
| `non applicable` | sans objet | `non applicable` | 5 | 0 | 19 | 22 | 5 | 0 | 0 | 5 |  | 10823528 `Non applicable` BE 11 |
| `diverse firmen` | diverse | `diverse firmen` | 1 | 0 | 19 | 0 | 19 | 0 | 0 | 0 |  | 1574172 `Diverse Firmen` DE 19 |
| `k. a` | keine angabe | `k a` | 1 | 0 | 16 | 17 | 0 | 0 | 0 | 0 |  | 8841270 `K. A.` DE 16 |
| `ohne` | keine angabe | `ohne` | 2 | 0 | 15 | 14 | 3 | 0 | 0 | 0 |  | 8256760 `Ohne` DE 11 |
| `multiple` | various | `multiple` | 5 | 0 | 15 | 1 | 14 | 0 | 0 | 0 |  | 6095690 `Multiple` GB 8 |
| `ei` | fi | `ei` | 2 | 0 | 15 | 0 | 15 | 0 | 0 | 0 |  | 24706207 `EI` - 14 |
| `nicht anwendbar` | keine angabe | `nicht anwendbar` | 3 | 0 | 14 | 16 | 0 | 0 | 0 | 0 |  | 13604027 `Nicht anwendbar` DE 10 |
| `non communiqué` | sans objet | `non communique` | 2 | 0 | 14 | 0 | 14 | 0 | 0 | 0 |  | 4442778 `Non communiqué.` - 9 |
| `desconocido` | varios | `desconocido` | 1 | 0 | 14 | 14 | 0 | 0 | 0 | 0 |  | 8285841 `desconocido` ES 14 |
| `withheld` | withheld | `withheld` | 2 | 0 | 14 | 1 | 15 | 0 | 0 | 0 | N | 6938819 `Withheld` GB 8 |
| `nein` | keine angabe | `nein` | 2 | 0 | 14 | 8 | 7 | 0 | 0 | 0 |  | 26916769 `Nein` - 7 |
| `-entfällt-` | entfällt | `entfallt` | 8 | 0 | 12 | 8 | 5 | 0 | 0 | 0 |  | 8522433 `-Entfällt-` DE 5 |
| `siehe vergabeunterlagen` | pointers (exact) | `siehe vergabeunterlagen` | 1 | 0 | 12 | 12 | 0 | 0 | 6 | 0 |  | 4846687 `Siehe Vergabeunterlagen` DE 12 |
| `varie` | nessuno | `varie` | 2 | 0 | 12 | 0 | 11 | 0 | 1 | 0 |  | 2338426 `varie` - 11 |
| `diversos` | varios | `diversos` | 1 | 0 | 11 | 0 | 11 | 0 | 0 | 0 |  | 12497885 `Diversos` ES 11 |
| `redacted` | withheld | `redacted` | 1 | 0 | 11 | 0 | 12 | 0 | 0 | 0 |  | 15815320 `Redacted` GB 11 |
| `not available` | not applicable / not known / unknown | `not available` | 3 | 0 | 11 | 10 | 1 | 0 | 0 | 0 |  | 6751092 `Not available` - 8 |
| `see attached` | pointers (exact) | `see attached` | 1 | 0 | 11 | 0 | 11 | 0 | 0 | 0 |  | 10921095 `See attached` IE 11 |
| `vari` | nessuno | `vari` | 2 | 0 | 11 | 0 | 11 | 0 | 0 | 0 |  | 27191524 `Vari` - 8 |
| `please see below` | various | `please see below` | 3 | 0 | 11 | 4 | 1 | 0 | 7 | 0 |  | 4835796 `Please see below` GB 7 |
| `commercially sensitive` | withheld | `commercially sensitive` | 1 | 0 | 10 | 0 | 10 | 0 | 0 | 0 |  | 26831184 `Commercially sensitive` - 10 |
| `not specified` | not applicable / not known / unknown | `not specified` | 2 | 0 | 9 | 8 | 1 | 0 | 0 | 0 | N | 6524156 `Not Specified` - 5 |
| `see section vi.3)` | pointers (exact) | `see section vi 3` | 3 | 0 | 9 | 4 | 1 | 0 | 8 | 0 | P | 7534432 `See Section VI.3)` GB 7 |
| `não se aplica` | pt | `nao se aplica` | 1 | 0 | 9 | 11 | 0 | 0 | 0 | 0 |  | 19818715 `Não se aplica` PT 9 |
| `n.d` | nessuno | `n d` | 1 | 0 | 8 | 6 | 3 | 0 | 0 | 0 |  | 9727678 `N.D.` IT 8 |
| `various contractors` | various | `various contractors` | 3 | 0 | 8 | 0 | 7 | 0 | 0 | 0 |  | 25098813 `Various Contractors` - 5 |
| `non` | sans objet | `non` | 3 | 0 | 8 | 1 | 4 | 0 | 3 | 0 |  | 1521947 `Non` FR 6 |
| `n/c` | sans objet | `n c` | 1 | 0 | 8 | 8 | 0 | 0 | 0 | 0 |  | 12663399 `N/C` FR 8 |
| `nicht relevant` | keine angabe | `nicht relevant` | 1 | 0 | 7 | 11 | 0 | 0 | 0 | 0 |  | 12435498 `Nicht relevant` DE 7 |
| `various providers` | various | `various providers` | 1 | 0 | 7 | 0 | 7 | 0 | 0 | 0 |  | 12068039 `Various Providers` GB 7 |
| `siehe unten` | pointers (exact) | `siehe unten` | 2 | 0 | 7 | 0 | 7 | 0 | 0 | 0 |  | 11678848 `Siehe unten` - 4 |
| `mehrere bieter` | diverse | `mehrere bieter` | 1 | 0 | 6 | 0 | 6 | 0 | 0 | 0 |  | 23702315 `mehrere Bieter` DE 6 |
| `various companies` | various | `various companies` | 2 | 0 | 6 | 0 | 6 | 0 | 0 | 0 |  | 27011297 `Various companies` - 5 |
| `entfaellt` | entfällt | `entfaellt` | 1 | 0 | 5 | 0 | 5 | 0 | 0 | 0 |  | 27586308 `Entfaellt` - 5 |
| `verschiedene` | diverse | `verschiedene` | 2 | 0 | 5 | 0 | 3 | 0 | 5 | 0 |  | 2672439 `verschiedene` DE 3 |
| `multiple contractors` | various | `multiple contractors` | 2 | 0 | 5 | 0 | 5 | 0 | 0 | 0 |  | 14437316 `Multiple Contractors` GB 4 |
| `not required` | not applicable / not known / unknown | `not required` | 2 | 0 | 5 | 2 | 3 | 0 | 0 | 0 |  | 6681073 `Not required` - 3 |
| `non connu` | sans objet | `non connu` | 1 | 0 | 4 | 3 | 1 | 0 | 0 | 0 |  | 3191239 `non connu` FR 4 |
| `divers` | diverse | `divers` | 1 | 0 | 4 | 0 | 3 | 0 | 3 | 0 |  | 8288957 `Divers` - 4 |
| `diversi` | diverse | `diversi` | 1 | 0 | 4 | 0 | 4 | 0 | 0 | 0 |  | 1433627 `Diversi` - 4 |
| `véase perfil del contratante` | pointers (exact) | `vease perfil del contratante` | 1 | 0 | 4 | 0 | 4 | 0 | 0 | 0 | P | 31402254 `Véase perfil del contratante` - 4 |
| `various - see below` | various | `various see below` | 1 | 0 | 4 | 0 | 8 | 0 | 0 | 0 |  | 31373446 `Various - see below` GB 4 |
| `null` | none | `null` | 2 | 0 | 3 | 5 | 1 | 0 | 0 | 0 |  | 17101713 `null` DE 2 |
| `ka` | keine angabe | `ka` | 3 | 0 | 3 | 3 | 1 | 0 | 0 | 0 |  | 8478155 `kA` AT 1 |
| `nessuna` | nessuno | `nessuna` | 1 | 0 | 3 | 0 | 4 | 0 | 0 | 0 |  | 9544155 `Nessuna` - 3 |
| `ninguna` | varios | `ninguna` | 2 | 0 | 3 | 0 | 3 | 0 | 0 | 0 |  | 23001323 `NINGUNA` ES 2 |
| `verschiedene firmen` | diverse | `verschiedene firmen` | 1 | 0 | 3 | 0 | 3 | 0 | 0 | 0 |  | 27003605 `Verschiedene Firmen` - 3 |
| `mehrere` | diverse | `mehrere` | 2 | 0 | 3 | 0 | 3 | 0 | 0 | 0 |  | 4709962 `mehrere` DE 2 |
| `not relevant` | not applicable / not known / unknown | `not relevant` | 2 | 0 | 3 | 2 | 0 | 0 | 2 | 0 |  | 17178076 `Not relevant` DK 2 |
| `nu este cazul` | ro | `nu este cazul` | 1 | 0 | 3 | 3 | 0 | 0 | 0 | 0 |  | 20865941 `nu este cazul` RO 3 |
| `δεν απαιτείται` | el | `δεν απαιτειται` | 1 | 0 | 3 | 3 | 0 | 0 | 0 | 0 |  | 15720810 `Δεν απαιτείται` GR 3 |
| `-/-` | junk punctuation | (empty) | 1 | 0 | 3 | 3 | 0 | 0 | 0 | 1 | ∅ | 4260378 `-/-` DE 3 |
| `n a` | n/a | `n a` | 2 | 0 | 2 | 2 | 0 | 0 | 0 | 0 | N | 22072936 `N A` FR 1 |
| `n/ a` | n/a | `n a` | 1 | 0 | 2 | 2 | 0 | 0 | 0 | 0 | N | 12413045 `N/ A` GB 2 |
| `none specified` | none | `none specified` | 1 | 0 | 2 | 0 | 0 | 0 | 2 | 0 |  | 8845431 `None specified` GB 2 |
| `- entfällt -` | entfällt | `entfallt` | 2 | 0 | 2 | 0 | 2 | 0 | 0 | 0 |  | 1348757 `- entfällt -` - 1 |
| `aucune` | aucun | `aucune` | 1 | 0 | 2 | 0 | 0 | 0 | 0 | 2 |  | 31492942 `Aucune` FR 2 |
| `diverse bieter` | diverse | `diverse bieter` | 1 | 0 | 2 | 0 | 2 | 0 | 0 | 0 |  | 30758674 `diverse Bieter` - 2 |
| `not known` | not applicable / not known / unknown | `not known` | 1 | 0 | 2 | 2 | 0 | 0 | 0 | 0 | N | 7472305 `Not Known` GB 2 |
| `niet bekend` | nl | `niet bekend` | 2 | 0 | 2 | 0 | 3 | 0 | 0 | 0 |  | 31830801 `Niet bekend` - 1 |
| `n/a s. r. o` | traps | `n a s r o` | 1 | 0 | 2 | 0 | 4 | 0 | 0 | 0 |  | 12290440 `N/A s. r. o.` SK 2 |
| `varios licitadores` | varios | `varios licitadores` | 2 | 0 | 2 | 0 | 2 | 0 | 0 | 0 |  | 28751332 `Varios licitadores` - 1 |
| `see list below` | various | `see list below` | 1 | 0 | 2 | 0 | 2 | 0 | 0 | 0 |  | 28777793 `See list below` - 2 |
| `inget` | ingen | `inget` | 1 | 0 | 1 | 1 | 0 | 0 | 0 | 0 |  | 9820701 `Inget` SE 1 |
| `n /a` | n/a | `n a` | 1 | 0 | 1 | 1 | 0 | 0 | 0 | 0 | N | 10926061 `N /A` GB 1 |
| `(n/a)` | n/a | `n a` | 1 | 0 | 1 | 1 | 0 | 0 | 0 | 0 | N | 12951826 `(N/a)` GB 1 |
| `brak danych` | brak | `brak danych` | 1 | 0 | 1 | 0 | 1 | 0 | 0 | 0 |  | 8947151 `brak danych` PL 1 |
| `s/o` | sans objet | `s o` | 1 | 0 | 1 | 1 | 0 | 0 | 0 | 0 |  | 14218144 `S/O` FR 1 |
| `non concerné` | sans objet | `non concerne` | 1 | 0 | 1 | 1 | 0 | 0 | 0 | 0 |  | 12495535 `Non concerné` FR 1 |
| `non noto` | nessuno | `non noto` | 1 | 0 | 1 | 0 | 1 | 0 | 0 | 0 |  | 23591127 `NON NOTO` IT 1 |
| `varias` | varios | `varias` | 1 | 0 | 1 | 0 | 1 | 0 | 0 | 0 |  | 6387910 `Varias` ES 1 |
| `varios proveedores` | varios | `varios proveedores` | 1 | 0 | 1 | 0 | 1 | 0 | 0 | 0 |  | 29238424 `Varios proveedores` - 1 |
| `various organisations` | various | `various organisations` | 1 | 0 | 1 | 0 | 1 | 0 | 0 | 0 |  | 6915170 `Various organisations` GB 1 |
| `withheld section 94 supplier` | withheld | `withheld section 94 supplier` | 1 | 0 | 1 | 0 | 1 | 0 | 0 | 0 |  | 31572616 `Withheld Section 94 Supplier` GB 1 |
| `see section vi.3 below` | pointers (exact) | `see section vi 3 below` | 1 | 0 | 1 | 0 | 0 | 0 | 0 | 2 | P | 2342763 `See section VI.3 below` - 1 |
| `see attached list` | pointers (exact) | `see attached list` | 1 | 0 | 1 | 0 | 1 | 0 | 0 | 0 |  | 31609872 `See attached list` GB 1 |
| `voir autres informations` | pointers (exact) | `voir autres informations` | 1 | 0 | 1 | 0 | 1 | 0 | 0 | 0 | P | 7509824 `voir autres informations` - 1 |
| `?` | junk punctuation | (empty) | 1 | 0 | 1 | 3 | 0 | 0 | 0 | 0 | ∅ | 10572751 `?` - 1 |
| `ei ole` | fi | `ei ole` | 1 | 0 | 1 | 0 | 1 | 0 | 0 | 0 |  | 24815021 `ei ole` - 1 |
| `nem releváns` | hu | `nem relevans` | 1 | 0 | 1 | 0 | 1 | 0 | 0 | 0 |  | 3036953 `Nem releváns.` - 1 |
| `není` | hr/sl/cs/sk/lt | `neni` | 1 | 0 | 1 | 1 | 0 | 0 | 0 | 0 |  | 13094589 `není` CZ 1 |
| `ej aktuellt` | sv/da/no | `ej aktuellt` | 1 | 0 | 1 | 1 | 0 | 0 | 0 | 0 |  | 9913924 `Ej aktuellt` SE 1 |
| `ikke aktuelt` | sv/da/no | `ikke aktuelt` | 1 | 0 | 1 | 2 | 0 | 0 | 0 | 0 |  | 13724365 `Ikke aktuelt` NO 1 |
| `no hay` | varios | `no hay` | 1 | 1 | 1 | 1 | 0 | 0 | 0 | 0 |  | 22525406 `No hay` ES 1 |
| `sin` | varios | `sin` | 1 | 0 | 1 | 0 | 1 | 0 | 0 | 0 |  | 11261906 `SIN` FR 1 |
| `varios contratistas` | varios | `varios contratistas` | 1 | 0 | 1 | 0 | 1 | 0 | 0 | 0 |  | 31481260 `Varios contratistas` - 1 |
| `diverse ditte` | nessuno | `diverse ditte` | 1 | 0 | 1 | 0 | 1 | 0 | 0 | 0 |  | 8161054 `Diverse ditte` - 1 |

## Keys probed with no org (101)

`- néant -`, `-néant-`, `néant -`, `neant -`, `ingen uppgift`, `ingen angiven`, `n / a`, `n-a`, `#n/a`, `n\a`, `n/a -`, `- n/a -`, `none known`, `nothing`, `- nie dotyczy -`, `-nie dotyczy-`, `nie dotyczy -`, `nie dotyczy!`, `nie podano`, `nie wskazano`, `entfallt`, `entfällt -`, `keine angabe -`, `entfällt/keine angabe`, `- brak -`, `brak informacji`, `nie ma`, `non concerne`, `inconnu`, `sans objet -`, `- sans objet -`, `non pertinente`, `sin datos`, `no consta`, `diverse unternehmen`, `diverse auftragnehmer`, `diverses`, `diverses entreprises`, `mehrere firmen`, `various organizations`, `various suppliers - see below`, `multiple organisations`, `multiple bidders`, `multiple winners`, `several`, `several suppliers`, `withheld section94 suppliers`, `not disclosed`, `not published`, `not stated`, `unspecified`, `n/k`, `nk`, `see section vi.4`, `see list`, `see annex`, `please see attached`, `voir section vi.3`, `n v t`, `ei sovellu`, `ei sovelleta`, `nincs`, `nem alkalmazandó`, `tidak`, `tidak ada`, `nu se aplica`, `nu se aplică`, `nema`, `ne primjenjuje se`, `nevztahuje se`, `netaikoma`, `ne`, `ej tillämpligt`, `inte tillämpligt`, `nao aplicavel`, `nenhum`, `nenhuma`, `n/a s.r.o`, `diverse care services`, `diverse care services limited`, `various ltd`, `various limited`, `ingen ab`, `ingen ltd`, `n/a, s.r.o`, `n/a, s. r. o`, `n / a s.r.o`, `ingen as`, `ingen gmbh`, `diverse care`, `--/--`, `- / -`, `n/a n/a`, `na na`, `aucun organisme`, `sans objet / néant`, `nessun`, `various (see below)`, `various suppliers (see below)`, `multiple suppliers - see below`, `see attached schedule`

## Files

- `census.json`. It holds:
  - per candidate key: group, the spelling probed, `fold`, `in_NON_NAME_FOLDS`, `partyname_today`, orgs,
    identifier-bearing orgs, country-less orgs, mentions, party rows, the complete role split and buckets,
    countries, spellings, and **every org** (id, name, country, identifier_kind, identifier, provisional,
    mentions, party rows, roles);
  - `groups` (per concept), `by_fold` (per fold), `zero_keys`, `totals`, and `calls` (the full log: 36 entries,
    status, wall time, row count).
- Scripts and raw bodies, kept outside the repo in the session scratchpad
  (`/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/p511/`): `keys.py` (the key list and the
  `org_name_norm` copy), `foldx.py` (the fold copy), `sq.py` (the call wrapper; it never retries, and a body
  without `rows`/`columns`/`row_count` counts as a failure), `orgs_*.json`, `counts_*.json`, `roles_*.json`,
  `trap_mentions.json`, `calls.jsonl`.
