# 479 — a Tender carries no procedure type (BT-105), in any source: the filter procurement analysis reaches for first is missing

Status: ready-for-agent — UNIT 1 LANDED + REVIEW FIXES 2026-10-05, NOT DEPLOYED, uncommitted (see "Unit 1 — landed" and "Review fixes" below). NEXT: ops/check.sh → commit → deploy → the bounded legacy census (docs/operations.md, "Census before the backfill": PR/PROC code × procedure marker per profile, /v1/sql PK windows, team lead's word per read) → settle the PR 4 gate and any unmapped code's row, and if the table changes, gate + deploy again → `df` → ONE `refold` over every profile (expect=1 first), batched with any other corpus-wide fold change, off the daily tick (~7.5 h full fallback) → read the trailing project's `issue-479` line → the Verify, plus a timed rare code. Was: UNIT 1 LANDED 2026-10-05 (census deferred to the backfill fold — reversed by the review). Was: ready-for-agent — filed 2026-10-01 15:5x UTC from closing 465, which called it "a separate model question … out of scope". Nothing owns it. The first unit is the decision and the census: which code list, which scope (procedure, and whether per lot), how every era maps onto it, and what the backfill costs. Measure before building.
Kind: data model / API (a core attribute that is parsed and then dropped)
Relates to: 465 (FTS now parses BT-105 into the notice layer), 397 (contract nature, folded from every era: the
template), `.scratch/tender-db/api-dq-review-2026-09-15.md` (row dq-eforms: "BT-105, BT-23, BT-01, BT-36, BT-765/766,
BT-60 … have no place on the Tender", marked "not recorded")

## What is wrong

Open, restricted, negotiated with or without a call, competitive dialogue, innovation partnership, direct award:
the procedure type decides who could bid. It is the first filter a market analyst or a transparency reader applies
("show me the direct awards above €1 M"). Every source publishes it:
- eForms: BT-105-Procedure, `procurement-procedure-type`.
- DÖE: eForms-DE and sdk-0.1.
- FTS: `procurementMethodDetails`, mapped onto BT-105 since 465 (`b629d0b`).
- TED's legacy eras: the r209 standard forms' section IV.1.1 / `PROCEDURE` element, and r208's equivalents.
- The text era: a header field (to check).

tender-db parses it into the notice layer (`/v1/notices/{id}/content`) and drops it at the fold. Read 2026-10-01:
`/v1/tenders?limit=1` items carry `country, cpv, dispatched_at, id, kind, lots, notice_subtype, original_lang,
procedure_key, publication_id, published_at, source, submission_deadline, submission_deadline_scope, title, value,
version`. The detail adds satellites (`classifications` with `cpv | nuts | nature`, `amounts`, `dates`, `parties`, …),
and none of them is a procedure type. `crates/ingest/src/project.rs` has no reference to BT-105. A reader can only get
the procedure type by fetching and walking every notice's content.

## First unit (decide, then measure)

1. **Representation.** Prefer a classification scheme `procedure` (BT-105 codes, `field = 'procedure'`) beside 397's
   `nature`. It needs no new table: `tender_version_classifications` already versions per tender, it is already
   filterable, and it already folds through the supersession rule. Decide the list filter's name
   (`?procedure_type=open`), whether lots can differ (eForms has no lot-level BT-105; FTS has one procedure), and how
   the legacy codes map onto eForms' list (r209's `PT_OPEN`, `PT_RESTRICTED`, `PT_NEGOTIATED_WITH_PRIOR_CALL`,
   `PT_AWARD_CONTRACT_WITHOUT_CALL`, …). Unmappable legacy values go in an explicit "kept as published" column or are
   left unmapped and counted, never guessed.
2. **Census.** Count per profile how many notices publish a procedure type, with a bounded read per profile through
   the notice layer's field index, or a dry job. Record the per-era coverage this would give.
3. **Cost.** Adding a classification row per tender version needs the fold to rewrite every tender: a
   whole-corpus `project rebuild` (job 1616 took 442 min) or a scoped `refold-fields` per profile. Pick by measurement,
   off the daily tick.

Then build: fold, API filter, docs and OpenAPI, tests per era (one fixture each), and the refold.

## Verify

    curl -s 'https://tenders.zebreus.click/v1/tenders/8576017' | jq -c '[(.classifications // [])[] | select(.scheme == "procedure") | .code]'

- **open** (2026-10-01): `[]`. The notice says `BT-105-Procedure=open`, and the tender carries nothing.
- **done:** `["open"]`, and `/v1/tenders?procedure_type=open&limit=1` answers with that filter applied, not ignored.

## Unit 1 design (2026-10-05)

Decision only; no code changed. Evidence: the parsers and fold (`project.rs` :3906 nature pre-arm,
`contract_nature` :5967, `NATURE_STEMS` :5779, `DE1_FIELD_ALIASES` :895, `Fact::key` canonical.rs:2534),
`r209/rules.rs`, `text/rules.rs`, `fts/parse.rs` `procedure_type`, the vendored SDK field files, every committed
fixture, and 12 public reads (2026-10-05, `/v1/notices/{id}/content`, `/v1/tenders/{id}`, `/v1/notices?publication_id=`).

### What each era publishes (read, not assumed)

| era / profile | field id in the notice layer | real value read | scope |
|---|---|---|---|
| TED eForms (`eforms:eforms-sdk-1.x`) | `BT-105-Procedure`, list `procurement-procedure-type` | notice 23555356: `neg-w-call` | procedure (no lot-level BT-105 in the SDK) |
| DÖE eForms-DE 2.x | `BT-105-Procedure` (same id) | fixture `eforms-de-2.1-can-…`: `open`. Live notice 46429644 (tender 8575182) is a PIN (`Part`) and carries **none**: PINs publish no procedure type | procedure |
| DÖE eForms-DE 1.x | `DE1-TenderingProcess-ProcedureCode` (not aliased today) | fixtures `eforms-de-1.1-cn-*`: `open` | procedure |
| DÖE sdk-0.1 | `SDK01-TenderingProcess-ProcedureCode`, list `procurement-procedure-type` | fixtures: `open`, `de-restricted-wo-call`, `de-comp-wo-call`. Live notice 26544162 (tender 1542904): **free German text** `beabsichtigte Beschränkte Ausschreibung` under the same list name | procedure |
| FTS (`fts:ocds-1.1`) | `BT-105-Procedure` (since 465, closed label table) | tender 8576017 / notice 46697935: `open` | procedure |
| TED r209 | `TED-PR_PROC` (CODED_DATA_SECTION, ~100 % fill per ted-legacy-mapping.md §5.1) **and** the form's section-IV `TED-PT_*` marker (Integer 1) | notice 20320242 (100002-2019): `PR_PROC=1` + `PT_OPEN` | notice (= procedure) |
| TED r208 (incl. R2.0.7) | `TED-PR_PROC`; markers only on some forms | notice 17806795 (100002-2014): `PR_PROC=1`, no marker | notice |
| text era | `TXT-PR` (header `PR:`, same TED code list) | notice 2114278 (100002-2003, a CAN): `PR=7` = "Contract awards"; fixtures print `PR: 1 - Open procedure` … `PR: 0 - Prior information`, `7 - Contract awards`, `8 - General information` | notice |

Fixture cross-tab of `PR_PROC` against the form markers (code + label + marker in the same file):
`1 Open procedure` ↔ `PT_OPEN` (6/6); `2 Restricted procedure` ↔ `PT_RESTRICTED` (3/3);
`4 Negotiated procedure` ↔ `PT_NEGOTIATED_WITH_PRIOR_CALL` / `PT_NEGOTIATED_WITH_COMPETITION` (4/4, never a
without-call marker); `T Negotiated without a call for competition` ↔ `F03_/F15_PT_NEGOTIATED_WITHOUT_COMPETITION`
(2/2); `9 Not applicable` on F07/F08/F13/F19/F20 (no marker). F14 corrigenda carry a real PR code (`1`).

The eForms-DE SDK files name the national below-threshold codes in their rules: `us-open`, `us-res-tw`,
`us-res-no-tw`, `us-neg-w-call`, `us-neg-wo-call`, `us-free-tw`, `us-free-no-tw`, `us-hhr`. The EU SDK names
`open restricted neg-w-call neg-wo-call comp-dial innovation oth-single oth-mult`.

### 1. Representation

`Fact::Classification { field: "procedure", scheme: "procedure", code }` at **Tender scope only** — 397's shape,
`tender_version_classifications` unchanged (the `field`/`scheme` column comments gain `procedure`). No new
table, no head column. It versions per tender and supersedes as a unit (`Fact::key` = `("classification",
"procedure")`): a notice that publishes a procedure type replaces the carried one, a silent notice carries it
forward. That is the semantics wanted — and it is also why "unmapped ⇒ emit nothing" is correct rather than
lossy: a text-era CAN (`PR=7`) or an r209 F20 (`PR=9`) keeps the procedure type its contract notice stated.

**One code per notice.** The fold elects at most one procedure fact per notice (the first mapped value in
field-priority order `BT-105` > aliases > `PR_PROC` > `TXT-PR`); a second, *different* mapped value in the
same notice is not folded and is counted (none expected: each era has one field).

**Lots: no.** No source publishes a lot-level procedure type (eForms has no BT-105-Lot; FTS one per release;
legacy notice-level). A procedure value found inside a Lot section is still folded at Tender scope (it is a
procedure field by definition), not at the lot. A lot inherits its tender's value for filtering.

**Vocabulary: eForms' `procurement-procedure-type`, including its German extension, kept as published — no
cross-walk between national and EU codes.** The German below-threshold codes (`us-*`, sdk-0.1's `de-*`) are
codes of that same list's tailored variants; `us-res-no-tw` (restricted without a call) has no EU code, and
folding it into `neg-wo-call` or `oth-mult` would be a guess. They are stored verbatim. A "direct award across
vocabularies" facet (with/without a prior call) is a possible later unit, not this one.

### 2. Mapping tables (closed; anything else emits nothing and is counted)

`procedure_type(field_id, code) -> Option<&'static str>`, beside `contract_nature`, trimmed, ASCII-lowercased:

- **eForms id `BT-105-*`** (TED eForms, DÖE 2.x, FTS, and DE1 after the alias): `open`, `restricted`,
  `neg-w-call`, `neg-wo-call`, `comp-dial`, `innovation`, `oth-single`, `oth-mult`, `comp-tend` (if the
  census sees it), the eight `us-*` codes above → themselves. Anything else → nothing.
- **DE 1.x**: add `("DE1-TenderingProcess-ProcedureCode", "BT-105-Procedure")` to `DE1_FIELD_ALIASES` (fold
  side, no re-parse), then the BT-105 arm applies.
- **sdk-0.1 `SDK01-TenderingProcess-ProcedureCode`**: the EU codes above, `us-*`, and the `de-*` codes the
  census lists (seen: `de-restricted-wo-call`, `de-comp-wo-call`; documented: `de-open`,
  `de-comp-neg-wo-call`) → themselves. Free-text labels (`beabsichtigte Beschränkte Ausschreibung`, …) →
  nothing, counted; a German-label table is a follow-up only if the census shows it is a large share.
- **TED legacy `TED-PR_PROC` (r208, r209) and `TXT-PR` (text)** — one table, the eras share the code list
  (ted-legacy-mapping.md §4, as `NC` did for 397):

  | PR | label (fixture) | eForms | evidence |
  |---|---|---|---|
  | `1` | Open procedure | `open` | 6/6 with `PT_OPEN`; text fixtures |
  | `2` | Restricted procedure | `restricted` | 3/3 with `PT_RESTRICTED` |
  | `3` | Accelerated restricted procedure | `restricted` | text fixture label; eForms states acceleration separately (BT-106), the type is restricted |
  | `4` | Negotiated procedure | `neg-w-call` | 4/4 with a with-call marker; `T` is the without-call code. **Gated on the census**: if more than 1 % of PR 4 notices carry a without-call marker, PR 4 folds only through the marker (with-call → `neg-w-call`, without → `neg-wo-call`, none → nothing) |
  | `T` | Negotiated without a call for competition | `neg-wo-call` | 2/2 with `*_PT_NEGOTIATED_WITHOUT_COMPETITION` |
  | `6`, `B`, `C`, `G`, `V`, … | not in any fixture | **unmapped until the census cross-tab names them** (e.g. a code that co-occurs ≥ 99 % with `PT_COMPETITIVE_DIALOGUE` → `comp-dial`; `PT_INNOVATION_PARTNERSHIP` → `innovation`; `PT_AWARD_CONTRACT_WITHOUT_CALL` → `neg-wo-call`) | — |
  | `9` Not applicable, `Z`, text `0` PIN, `7` Contract awards, `8` General information | — | nothing (carry forward) | not procedure types |

  The `PT_*` markers are the census's evidence, not a second fold source: one field per era keeps one fact
  per notice and mirrors 397. (r208 publishes markers on only some forms; PR_PROC is on all of them.)
- **Drop sieve**: a `PROCEDURE_STEMS` list (`BT-105`, `DE1-TenderingProcess-ProcedureCode`,
  `SDK01-TenderingProcess-ProcedureCode`, `TED-PR_PROC`, `TXT-PR`) beside `NATURE_STEMS`, so the data-quality
  drop report stops listing them (465's `b4f2bba` lesson).

### 3. Census (before freezing the table) — bounded reads, no job to build

- **Table-deciding census**: sampled notice-id windows per profile through `/v1/sql` (team lead's word per
  read, low-traffic window, never retry a 408): `notice_codes` is keyed `(notice_id, …)`, so
  `WHERE c.notice_id BETWEEN a AND a+50000 AND c.field_id IN (…) GROUP BY n.profile, c.field_id, c.code` is a
  PK-range read. Window starts come from `/v1/notices?publication_id=…` (free; e.g. 100002-2014 → 17806795,
  100002-2019 → 20320242, 100002-2003 → 2114278). Two or three windows per profile (TED eForms, DÖE 2.x /
  1.x / sdk-0.1, FTS, r209, r208, text); for r208/r209 the same window joins `notice_integers` on
  `field_id LIKE 'TED-%PT_%'` for the PR × marker cross-tab that fills the `6/B/C/G/V` rows and gates PR 4.
- **Whole-corpus coverage**: the fold reports it for free — the backfill fold's report gains one line per
  profile, `procedure: N notices folded a code, M published an unmapped value, K published none`. That is
  the census of record (no separate corpus scan; the corpus-scale `notice_codes` walk refold-fields would do
  has no compliant on-box path as a read and costs ~46 min as a job).

Expected shape (to be replaced by the numbers): eForms CN/CAN ≈ all, PINs none; FTS 7 of 10 fixtures;
r208/r209 ≈ all CN/CAN, `9` on PIN/F20 etc.; text CNs yes, text CANs `7` (they inherit only when chained to
a CN; islands get nothing).

### 4. API

- **Filter `?procedure_type=<code>[,<code>…]`** on `/v1/tenders` and `/v1/lots` (a version predicate, so lots
  get it from `version_predicates` for free; `?procedure=` would collide with `kind=procedure`). Semantics:
  the tender's **current version** carries one of the codes at Tender scope —
  `EXISTS (… c.seq = <current seq> AND c.scheme = 'procedure' AND c.lot_id IS NULL AND c.code IN (…))`, i.e. the
  latest published value after supersession. Exact match (no prefix, no `LIKE`). Trimmed and lowercased once
  at the boundary (387's rule), each code shape-checked `[a-z0-9-]{1,32}`, at most 10 codes; junk is a 400; a
  well-shaped unknown code matches nothing (the `lang` posture: the vocabulary is open by design, `us-*`/`de-*`).
  Added to `provided_filters` and both collections' `honoured_params`, so it never echoes in `ignored_filters`.
- **Drive side**: a rare code (`innovation`, `comp-dial`) under an id-ordered walk would scan ~8.5M heads to
  the 15 s bound. Reuse the country-seed pattern (`with_country_seed`, `COUNTRY_SEED_CAP` 200,000): a capped
  count on the existing `(scheme, code)` index decides; under the cap, seed `t.id IN (SELECT tender_id FROM
  tender_version_classifications WHERE scheme='procedure' AND code IN (…))` and keep the current-seq EXISTS;
  over the cap (`open`), walk — dense codes fill a page fast. Measured on prod after the backfill, before the
  Verify is called done.
- **Echo**: list items and the detail gain `procedure_type` (string or null, the current version's Tender-scope
  code, one correlated seek on `tender_version_classifications_version` like the cpv/country echo). The
  detail's `classifications` serves the row as `{scheme: "procedure", field: "procedure", lot: null}`
  unchanged.
- **Docs/OpenAPI**: the parameter (with the EU codes listed and the German extension named), the `procedure`
  scheme in the `classifications` description, `docs.rs`, `sql.rs`'s `v_tender_classifications` and
  `CLASSIFICATION_FIELD_NOTE`, and a CONTEXT.md line (procedure type = eForms BT-105 vocabulary, legacy PR
  codes mapped by a closed table, unmapped kept silent).

### 5. Tests (one fixture per era)

`procedure_type` table unit test (each arm, a code off the list, a non-procedure id, the drop sieve agreeing);
a synthetic fold test (procedure value at Tender scope, a lot-section value lands at Tender, two different
codes in one notice fold one, a silent later notice carries forward). Per-era fixture tests through the fold:
TED eForms (a `neg-w-call` fixture), DÖE 2.1 (`open`), DE 1.1 (`open` via the alias), sdk-0.1 numeric
(`de-restricted-wo-call`) and a free-text label (nothing), FTS 029615-2025 (`open`) / 052408-2025
(`neg-w-call`), r209 f05-001315-2019 (PR 4 → `neg-w-call`), r208 f02-000333-2014 (`restricted`) and
veat-294050-2011 (`T` → `neg-wo-call`), r208 f07-185353-2013 (`9` → nothing), text 2008-cn-723-2008 (`1` →
`open`). API: `?procedure_type=open` filters and is not ignored; `?procedure_type=OPEN` folds; `?procedure_type=%`
→ 400; lots honour it.

### 6. Backfill — one full fold, no re-parse, no epoch bump

- **No re-parse.** Every era already stores its procedure field in the notice layer (FTS since 465); the DE1
  alias and the sdk-0.1 arm are fold-side. New ingests carry the fact from the deploy.
- **Scoped refold is not cheaper here.** Every era publishes the field, so the cohort is the corpus:
  `refold-fields` on the five ids sweeps the value tables first (~46 min, 2026-09-12) and then requeues
  ~all notices; `refold` with every profile skips the sweep. Either way the legacy delta is far over
  `LEGACY_CLOSURE_CAP` (500,000) and the incremental fold takes the FULL fallback — 1616's ~442 min. Per-era
  splitting does not help: the modern profiles alone are cheap (FTS 314k notices: 775 s, job 1814; the DÖE
  island 628k tenders: 37.6 min, job 1591), but the r208/r209/text eras hold most of the 8.56M tenders and
  only reach their standing rows through the full path.
- **Plan**: deploy, then ONE `{"kind":"refold","profiles":[<every profile>],"expect":<corpus notice count>}`,
  which requeues and scoped-stamps every tender stale (the issue-179 pair) and queues its trailing
  `project rebuild=false` → full fallback, ~7.5 h. Off the daily tick, on a hand-read idle queue (459), and
  **batched with any other corpus-wide fold change pending at that time** so the corpus pays one fold, as
  397 did with 1595–1597. No `PROJECTION_EPOCH` bump (397's reasoning: a bump owes the same rewrite on an
  unscheduled walk; the refold makes it explicit and guarded).
- Rejected: a bespoke "insert procedure rows" backfill writer — it would re-implement supersession/carry-
  forward outside the fold (two writers of one table) to save a fold that has a proven runbook.
- Before it: `df` (free) — ~14M new classification rows (one per version that has a code) plus two index
  entries each, order 1–1.5 GB; and note the fold emits a change for every rewritten tender, as 1616 did.

### Verify (unchanged, plus)

The issue's Verify on tender 8576017 (`["open"]`, `?procedure_type=open&limit=1` with `ignored_filters: []`),
and one tender per era read after the fold: TED eForms tender 2 (`neg-w-call`), r209 100002-2019's tender
(`open`), r208 100002-2014's tender (`open`), the text-era CAN 100002-2003 (`[]` unless chained to a CN — an
honest empty), DÖE 1542904 (`[]`: free-text label, counted).

## Unit 1 — landed (not deployed), 2026-10-05

Built to the Unit 1 design above. Uncommitted in the worktree; no gate (`ops/check.sh`) run yet, no
commit, no deploy.

**Fold (`crates/ingest/src/project.rs`).**
- `PROCEDURE_FIELDS` (exact ids, not stems: `BT-105-Procedure`, `DE1-TenderingProcess-ProcedureCode`,
  `SDK01-TenderingProcess-ProcedureCode`, `TED-PR_PROC`, `TXT-PR`), `procedure_type(field_id, code)` — the
  closed table of §2, `comp-tend` left out (not in any vendored SDK file; waits for the census) — and
  `elect_procedure_type(parsed)`: one code per notice, first mapped value in field order then published
  order, disagreeing mapped values counted as `conflicting`.
- `NoticeState::read` inserts `Fact::Classification { field: "procedure", scheme: "procedure" }` at Tender
  scope whatever section published it. Unmapped or absent → nothing → the earlier version's type carries
  forward by supersession (`Fact::key` = `("classification", "procedure")`).
- `DE1_FIELD_ALIASES` gains `DE1-TenderingProcess-ProcedureCode → BT-105-Procedure`.
- Drop sieve: `has_destination` reads `PROCEDURE_FIELDS` on the Code channel (the design's
  `PROCEDURE_STEMS`, as exact ids — `BT-105-Procedure-List` and `BT-195(BT-105)` must not count).
- Census of record: `ProcedureTally` (per family: `eforms`, `eforms-de-2x`, `eforms-de-1x`, `doe-sdk01`,
  `fts`, `r209`, `r208`, `internal-ojs`, `text`, `other` × folded / unmapped / none / conflicting),
  counted in `Ident::read` through the SAME `elect_procedure_type`, carried on `PlanChunk` / `build_plan` /
  the incremental plan into `Report.procedure`; the supervisor appends
  `; issue-479 procedure type: <family> F folded / U unmapped / N none[, C CONFLICTING], …` to the project
  job's counts line (`procedure_suffix`).
- No `PROJECTION_EPOCH` bump (397's reasoning; the golden still prints the unchanged epoch).

**Read layer (`crates/store/src/read.rs`).** `Filter.procedure_type: Vec<String>` (exact, any-of) →
`version_predicates` adds `EXISTS (… c.seq = <current> AND c.scheme = 'procedure' AND c.lot_id IS NULL AND
c.code IN (…))`, so Lots get it for free. Isolation: a new `Isolated::ProcedureType` (version-predicate set,
after `Currency`) with a guard leg `procedure_reachable` (one exact `(scheme, code)` seek per code).
Drive side: `procedure_seed` beside `country_seed` — `procedure_seed_viable` is the country seed's capped
count (`COUNTRY_SEED_CAP` 200,000) over exact codes; under the cap the tenders FROM seeds from
`procedure_seed_hits` (UNION ALL of exact seeks) and the lots read adds `l.tender_id IN hits`; below the
publication / org / country seeds in precedence; a seeded read is not band-bounded (`bounded_walk`).
`honoured_params` gains `procedure_type` on Tenders and Lots; `FILTER_CLASSIFICATION` 22 → 24.
`TenderRow.procedure_type` (column 24 of `tender_select_head`: `MIN(code)` at Tender scope) feeds the
list item and the detail.

**API (`crates/app/src/v1`).** `?procedure_type=` parsed by `procedure_types`: split on `,`, trimmed,
lowercased, each `[a-z0-9-]{1,32}`, repeats collapse, at most 10, junk / empty → 400; added to
`provided_filters`. `json::tender` serves `procedure_type`. OpenAPI: the `procedure_type` parameter on
`/v1/tenders` and `/v1/lots`, `Tender.procedure_type`, the `procedure` scheme in
`TenderDetail.classifications`. `/docs`: the filter row, the applies/ignored table (all four rows), the
detail paragraph. `/v1/sql`: `CLASSIFICATION_SCHEME_NOTE`, `CLASSIFICATION_FIELD_NOTE`, the
`v_tender_classifications` note; `canonical.rs` column/view comments. `CONTEXT.md`: a Data decision line.
`docs/operations.md`: the counts-line suffix and "The procedure type and its backfill (issue 479)".

**Tests** (all through the gate's flags and package set):
- `project::tests::the_procedure_type_maps_every_eras_code_onto_one_vocabulary` — every arm, codes off the
  table (`9 Z 0 7 8 6 B C G V`, `comp-tend`, a `de-*` code under BT-105, free text), non-procedure ids, the
  sieve agreeing.
- `project::tests::a_notice_folds_one_procedure_type_at_tender_scope` — lot-section value at Tender scope,
  two codes → one (BT-105 outranks PR_PROC) with `conflicting = 1`, unmapped vs silent in the tally, families.
- `tests/project.rs::every_era_folds_its_procedure_type` — one real fixture per era through dispatch,
  parse and fold, with its tally family: TED eForms `cn-renewals-00660164-2023` → `neg-w-call`; DÖE 2.1
  CAN → `open`; DE 1.1 CN → `open` (alias); sdk-0.1 numeric CN → `de-restricted-wo-call`; r209
  f05-001315-2019 (PR 4) → `neg-w-call`; r208 f02-000333-2014 → `restricted`; r208 veat-294050-2011 (T) →
  `neg-wo-call`; r208 f07-185353-2013 (9) → nothing, counted unmapped; text 2008-cn-723-2008 → `open`.
- `tests/project.rs::an_sdk01_free_text_procedure_label_folds_nothing_and_is_counted` — the numeric CN
  with its code replaced by `beabsichtigte Beschränkte Ausschreibung`.
- `tests/project.rs::the_procedure_type_carries_forward_through_a_silent_notice_and_supersedes` — a CN
  `1` + award `9` serves `open` on both versions; a CN `2` published in a lot section + award `T` serves
  `restricted` then `neg-wo-call`.
- `tests/fts.rs::an_fts_release_folds_its_category_as_the_contract_nature` (extended) — 029615-2025 `open`,
  052408-2025 `neg-w-call`, 028961-2025 none.
- `app/tests/api.rs::the_procedure_type_filter_applies_to_tenders_and_lots` — applied and not ignored, the
  echo, ` OPEN ` folds, comma any-of with repeats, absent codes empty, `%`/empty/trailing comma/space/`;`/11
  codes/33 chars → 400, lots honour it (seeded path), organizations/notices name it ignored, detail
  `procedure_type` + one Tender-scope `procedure` row.
- Updated: `store::tests::honoured_params_match_the_emitted_sql` (+`procedure_type`),
  `store/tests/isolation_routing.rs` (two cases).

**Golden diff (`fixtures/golden/project_apply.snapshot`, regenerated with `GOLDEN_CAPTURE=1`).** Exactly 7
added `tender_version_classifications` rows, nothing else (no changes-log row, no id moved, epoch
unchanged): `1|1` and `1|2` (the DÖE/TED pair, BT-105 `open` on both), `2|1`…`2|4` (the Maltese chain,
`open` on all four), `4|1` (the DE-1.1 CN, `open` through the new alias). Tender 3 (the DE-1.2 CAN) and the
PIN/BRIN islands publish no procedure type, so they carry none — the intended derived-layer change.

**Commands and exit codes.**
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p model -p store -p ingest -p tender-db --features tender-db/server procedure_type` → GATE-EXIT=0 (first build; 4 tests).
- the same, `-- procedure every_era_projects an_fts_release_folds fold_apply_output honoured_params …` → GATE-EXIT=101: only the golden (expected, see above).
- the same with `GOLDEN_CAPTURE=1 … fold_apply_output` → 101 (the capture run asserts against the
  compiled-in old golden after writing the new one).
- the same, `--no-fail-fast -- procedure every_era_projects an_fts_release_folds fold_apply_output
  honoured_params filter_classification isolat every_version_predicate every_de1_alias ubl_grafts
  the_report_sieve the_filters_narrow every_served_key every_published_example the_change_feed_names
  list_endpoints_name the_openapi_spec the_docs_page_names the_vendored_spec every_path_is_prefixed
  the_contract_nature` → GATE-EXIT=0, 48 tests ok.

**Not done / open.**
- `ops/check.sh` (the full gate) has not run; nothing is committed.
- The census (§3) is not run: PR 4 → `neg-w-call` rests on the fixtures; `6/B/C/G/V` stay unmapped and
  will show up as `unmapped` in the backfill fold's `issue-479` line, which is where to read their size.
- The procedure seed's crossover is unmeasured (as the country seed's is); time `?procedure_type=innovation`
  after the backfill. No store-level test pins the seeded SQL shape; the API test exercises the seeded path
  (a small DB is always under the cap), `honoured_params_match_the_emitted_sql` the unseeded one.
- Lot list items do not echo `procedure_type` (the Lot schema is unchanged); the filter applies to them.

**Prod steps (after the gate and a commit).** Deploy; `df -h`; on a hand-read idle queue, off the daily
tick: `{"kind":"refold","profiles":[<every profile>],"expect":1}` to size, then the real `expect`; it queues
`project rebuild=false` → full fallback (~7.5 h, like job 1616); read the project job's `issue-479` line;
run the Verify (8576017 → `["open"]`, `?procedure_type=open&limit=1` with `ignored_filters: []`, the one
tender per era list in the design's Verify). Runbook: docs/operations.md, "The procedure type and its
backfill (issue 479)".

## Review fixes (2026-10-05)

Eleven reviewer findings on Unit 1, each verified against the code, fixtures and (where cited) the
public reads. Uncommitted, not deployed.

1. **Census skipped before freezing the PR table (major) — FIXED (process).** True: the runbook made
   the ~7.5 h backfill fold its own census, so a census-driven table change would cost a second full
   fold. docs/operations.md now has "Census before the backfill (do not skip)": the bounded
   PK-window cross-tab (PR/PROC code × `TED-%PT_%` marker, per profile, `/v1/sql`, team lead's word
   per read) with what to read off it (the PR 4 gate, every unmapped code's marker distribution), and
   the runbook's steps put it ahead of the refold. Not run here (no `/v1/sql` from this session).
   Fix 2 also shrinks what the census can still change: marker-carrying notices no longer depend on
   the PR rows at all.
2. **Direct awards missed / docs overclaim (major) — FIXED.** Verified: `V` and the other off-table
   PR codes folded nothing, FTS's Procurement Act labels die in `fts::parse::procedure_type`, and
   docs.rs advertised `neg-wo-call` as "the direct awards". The fold now reads the r208/r209
   section-IV procedure checkboxes (`project::PROCEDURE_MARKERS`, the Integer-1 markers the r209
   walker already stores: `PT_OPEN`, `PT_RESTRICTED`/`PT_ACCELERATED_RESTRICTED`,
   `PT_COMPETITIVE_DIALOGUE`, `PT_INNOVATION_PARTNERSHIP`, the with-call
   `PT_NEGOTIATED_WITH_PRIOR_CALL`/`_WITH_COMPETITION`/`PT_COMPETITIVE_NEGOTIATION`, and the
   without-call `PT_AWARD_CONTRACT_WITHOUT_CALL`/`PT_NEGOTIATED_WITHOUT_PUBLICATION`/
   `F03_/F06_/F15_PT_NEGOTIATED_WITHOUT_COMPETITION`), ranked ABOVE the PR code in the election:
   the form's own statement beats TED's coding of it. So a PR 4 that ticked a without-call box folds
   `neg-wo-call` (the PR 4 gate, answered per notice), and `V`/`C`/`G` fold through their marker.
   Ambiguous markers (`PT_ACCELERATED_NEGOTIATED`, light-regime, concession, `PT_DA_*`) are left
   out. Every fixture's marker agrees with its PR code, so no fixture's result moved; a PR × marker
   disagreement is counted in the tally's `conflicting` column (the cross-tab, for free, in the fold
   line). FTS's Procurement Act labels stay unmapped (a parse-side change and a re-parse; not this
   unit), and the OpenAPI and /docs now say plainly that `neg-wo-call` is complete for eForms and not
   before it (legacy awards only via the marker; FTS PA routes not mapped). This reverses the design's
   "markers are evidence, not a second fold source": the election still yields one fact per notice.
3. **Latest-version semantics overstated (minor) — FIXED (docs).** True: an off-table procedure code
   emits nothing and the earlier type stays. OpenAPI, /docs, CONTEXT.md and `Filter`'s doc now say
   "the latest MAPPED type" and that it can predate the latest notice. The split into
   not-a-type vs unmapped-type-with-suppression was not built: suppression needs a tombstone the
   fold's supersession has no shape for, and fix 2 removes the main real case (`V` with its marker).
4. **FTS U structurally 0 (minor) — FIXED (docs).** True (`fts::parse::procedure_type` returns
   `None` before the notice layer). docs/operations.md names it: `fts` U is 0 by construction and the
   Procurement Act labels land in `fts` N.
5. **Unmapped-code list incomplete (minor) — FIXED.** `procedure_type`'s doc, the runbook and the
   unit test now say "every code but 1/2/3/4/T" (naming `6 B C E F G N V`); the census SQL groups by
   every code seen; the test's silent list adds `E F N 5 A` and lowercase `t`.
6. **sdk-0.1 `de-*` list closed and half unverified (minor) — FIXED.** Any well-shaped sdk-0.1 code
   `de-[a-z0-9-]+` (≤ 32 chars, the API filter's own shape) is kept verbatim — the design's
   "German national codes kept as published". `procedure_type` now returns
   `Option<Cow<'static, str>>` for it. BT-105 still rejects `de-*`. Free text fails the shape.
7. **internal-ojs `TED-PROC` never read (major) — FIXED.** Verified: `internal_ojs.rs` claims `PROC`
   as bare-text `CodeText`, fixtures carry `<PROC>1</PROC>` ×4 / 7 / 9, and the reviewer's live read
   (27161440 → `TED-PROC 9`). `TED-PROC` joins `PROCEDURE_FIELDS` beside `TED-PR_PROC` and shares
   the PR table; the sieve covers it; `every_era_folds_its_procedure_type` gains internal-ojs
   115165-2008 (`1` → `open`, family `internal-ojs`) and 114238-2008 (`9` → nothing, counted
   unmapped). Named in operations.md, OpenAPI, CONTEXT.md.
8. **Runbook drops the census (major) — FIXED** with 1 (same defect, the runbook half): step 1 of the
   backfill block is now "the census above is read and the table frozen from it (deployed)", and the
   section says that skipping it on purpose must be recorded here.
9. **`tenders()` seeds `Scope::At` reads (minor) — FIXED.** True: the SSE diff's single-row reads ran
   `with_country_seed`, now `procedure_seed_viable`'s capped count too. `read::tenders` seeds only
   `Scope::Page`, mirroring `lots_identity` (the unseeded At read returns the same row).
10. **Test gaps (minor) — FIXED.** `read::procedure_seed_tests::the_procedure_seed_never_admits_a_superseded_type`:
    four tenders with superseded chains (restricted→neg-wo-call, open→restricted, open→none), every
    list shape (`tenders_query`, `tenders_ordered_query`, `lots_query`) with `procedure_seed` forced
    true and false — same ids, an earlier version's code never matches, and the seeded SQL really is
    the seeded shape. `head_pointer_plan_tests::filters()` gains `procedure_type` and
    `procedure-seeded`, so the plan and aggregate checks cover both shapes. The API test adds
    `sort=published_at` (both orders).
11. **Resumed fold prints no census (minor) — FIXED (docs).** True (`ProcedureTally` fills in
    Phase 1). The runbook says so, that the stopped run's line is partial, and to take coverage from
    the census reads then.

**Tests after the fixes** (gate flags and package set):
- `… --no-fail-fast -- procedure every_era_projects an_fts_release_folds fold_apply_output honoured_params` → GATE-EXIT=0.
- `… --no-fail-fast -- read:: procedure every_era_projects an_fts_release_folds fold_apply_output honoured_params filter_classification isolat every_version_predicate every_de1_alias ubl_grafts the_report_sieve the_filters_narrow every_served_key every_published_example the_change_feed_names list_endpoints_name the_openapi_spec the_docs_page_names the_vendored_spec every_path_is_prefixed the_contract_nature internal_ojs sieve dropped` → GATE-EXIT=0.
- `… --no-fail-fast -- data_quality destination table_reads unmapped legacy r208 r209 marker` → GATE-EXIT=0 (119 tests; the sieve and legacy probes with the markers now read).
- `fold_apply_output_matches_the_committed_golden` passes against the Unit-1 golden: the fixes move none of its rows.


## Legacy census (2026-10-05, read)

Ran the runbook's PR × marker query over five bounded notice-id windows: r209 20320242+10k and
22500000+20k, r208 17806795+20k, text 2114278+20k, internal-ojs 27161440+20k. Each read took about 1 s
and none returned a 408. Rows are in `.scratch/tender-db/479-procedure/census-2026-10-05.txt`; the
first r209 window is in this issue's session log.

- **PR 4 gate passes.** 2,276 PR 4 notices carry a marker, all of them with-call
  (`NEGOTIATED_WITH_PRIOR_CALL` / `_WITH_COMPETITION` / `_WITH_PUBLICATION_CONTRACT_NOTICE`), plus 3
  carry `INVOLVING_NEGOTIATION`. None carries a without-call marker, so `4 → neg-w-call` stands.
- **Mapped from the cross-tab** (a code that goes with one type on ≥ 99 % of its marked notices):
  - `V → neg-wo-call`: 387 of 387 marked notices are `AWARD_CONTRACT_WITHOUT_CALL`. r208 has 631
    unmarked `V`.
  - `C → comp-dial`: 141 of 141 are `COMPETITIVE_DIALOGUE`.
  - `G → innovation`: 23 of 23 are `INNOVATION_PARTNERSHIP`.
  - `6 → neg-w-call`: 64 of 64 are `ACCELERATED_NEGOTIATED`. Acceleration shortens the time limits
    of a published call, so this procedure always has a call.
  - `B → neg-w-call`: 1,114 `COMPETITIVE_NEGOTIATION` and 47 `INVOLVING_NEGOTIATION`. Both are
    procedures with a call, and eForms' `neg-w-call` label includes "competitive procedure with
    negotiation".
- **Still unmapped:**
  - `E` (concession with prior publication, 60 of 168 marked): not a procedure type.
  - `F` (concession without publication, 3 notices).
  - `A`: the `PT_DA_*` direct-award grounds.
  - `Z`, `9`, `8`, `0`.
  - Text and internal-ojs `D`, `I`, `Q`, `R`, `N`: no marker to read them by.
- **internal-ojs and text `7` is not a procedure type.** It appears with `PT_OPEN` 373 times,
  `PT_RESTRICTED` 367 and `NEGOTIATED_WITH_COMPETITION` 103, which fits contract-award notices. Those
  notices fold through their marker, which outranks the code.

## Deploy and backfill (2026-10-05)

- **Gated and deployed f8b776d.** The first gate, on 87d06ff, aborted on issue 467's poll tripwire:
  the procedure fold pushed `run_project` past 472 KiB. The fix boxes the projection futures,
  boxes the chunked fold in its wrapper, and moves the plan-row loop out to
  `incremental_plan_rows`. The frame re-measured between 452 and 458 KiB. gdb on the overflow names
  the chain: fold frame 117 KiB, then turso's recursive expression parser at about 50 KiB per
  level. The second gate passed (GATE-EXIT=0).
- **Smoke test after the deploy.** `?procedure_type=open` answers with `ignored_filters: []` and
  matches nothing yet. Junk input answers 400.
- **Refold.** The `expect=1` sizing, job 1999, counted 14,887,661 notices across every profile
  (job 1311's list plus `fts:ocds-1.1`, `eforms-sdk-1.4` and `eforms-sdk-1.15`). The wet run,
  job 2001 → project 2002, started at 13:51 UTC. It takes the full fallback, about 7.5 h.
- **Next:** read 2002's `issue-479 procedure type` suffix, then run the Verify list from the
  runbook.
- **Progress read, 2026-10-05 18:47 UTC** (the box logs in CEST; this said 20:47 UTC at first). Phase 1 (planning) covered all 14,887,661 notices and finished
  around 19:51 UTC, about 6 h. Phase 2 (folding) is at 170,297 of 8,776,591 tenders: 400,008 versions and
  80.4 M leaf rows written. Its rate is rising, from 5 min per ~22k tenders at first to under 2 min now
  (the first tenders are the heaviest, about 470 leaf rows each). Projected end: around 08:00–10:00 UTC
  2026-10-06 (revised below). The 07:35 daily queues behind it. `/data` has 382 → 325 GB free; that crossed
  `tender-db-diskwatch`'s 80 % threshold, so the unit is red. The freed plan pages stay in the database
  file as free pages, so `df` will not recover after the fold. If 80 % used is the new resting level,
  revisit the threshold.
- **Second progress read, 19:40 UTC.** 430,455 of 8,776,591 tenders: 260k in 53 min, about 4.9k per minute.
  The rest projects to about 28 h, so it ends around 2026-10-07 00:00 UTC unless the rate rises. Job 1616's
  whole full fallback took 7.4 h. The fold is CPU-bound on one thread (`server` at 80 % CPU, IO pressure
  about 5 %, WAL 118 MB), so it is working, not stalled. `/data` has 319 GB free (−6 GB in 53 min). Read again
  next firing; if the rate stays below 1616's, compare the work per tender before the next corpus-wide fold.
