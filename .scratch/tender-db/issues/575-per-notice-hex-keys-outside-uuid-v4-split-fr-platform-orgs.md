# 575 — per-notice hex keys outside `uuid_v4` (the FR 8-4-4-16 platform shape) split one organization into a row per notice

Status: ready-for-agent — **BUILT 2026-10-24 (a5c50ae on the session branch): the dry hex-census job (store hex_census.rs + supervisor arm).** Drafted from `build-specs/575.md` (spec + adversarial check, wf_b8e47a6e-d46) in an isolated worktree (wf_5cd7b201-7c5), merged, compiled and focused-tested (91 passed across the deploy #3 batch); full gate and a 3-lens adversarial review (wf_c35791a6-e18) running. NEXT: deploy #3 when 546 U4's project 2765 ends; after deploy #3: one hex-census run on the box (class Z), then U2 decides from the stored report. Was: ready-for-agent — TRIAGED 2026-10-21 (wf triage-575-578: a measurer and two adversarial skeptics; `.scratch/tender-db/575-triage/`). The raw 8-4-4-16 BT-501 shape (`cf35`, scheme `eu`) is the key Avenue-Web's eSender (marches-publics.info, ColdFusion) mints for each ORG section of each notice. In the 6 sampled ORG sets the values rise in section order, the same body printed twice in one notice gets 2 values (3 of the 6), and none of census 2655's 3,511 v4-passing rows spans 2 notices (those rows are a uniform 1/64 subsample of the shape). By extrapolation the class is about 217k–223k org rows with about 1 mention each. 62k–68k of them are the eSender "Avenue-Web Systèmes" itself (334/428). Buyers carry stable platform account numbers (0 of 6 sampled carry the shape); AddInfo is mixed (1 of 2). Direction: refuse by raw shape in `project::gated_identifier` (not a 530-T widening, not `idgate::condemns`), measured first. NEXT: unit 1, a dry `hex-census` cf35 arm over every length-32 identifier row. It classifies the raw shape directly and reports by country, role, year and name, plus buyer-role, blank-name, KEEP_530-overlap and fallback-target counts. Unit 2 then decides the name-guard design and the bars. Blocked by: 539's drain, which takes the single writer next (scheduling only; U1 can be built meanwhile). 551 B6's U7 code freeze is over (969afba: U7 drained, U8 PASS). Was: needs-triage — filed 2026-10-19 from issue 530 unit 2's decision (`.scratch/tender-db/530-triage/decision.md`, "530-F"), which declined to refuse 1/64 of a platform inside a v4-scoped issue. Next: unit 1 — size the raw 35-character 8-4-4-16 BT-501 shape across the whole hex-hash class (not only the `uuid_v4` passes), by platform, with 530's census definitions.
Blocked by: 539 (drain slot on the single writer; no data dependency)
Kind: data quality (organization identity, FR): identifier admission, split side; severity medium
Relates to:
- 334 (closed): its open question 1 is answered here. The rows are the platform's own eSender (Procedure-SProvider) block, and the
  identifier is that block's per-section key, not a buyer's identifier;
- 428: the issue-234 fallback lookup walks a whole name group (`avenue-web systèmes` 68,239 rows, `tribunal administratif` 25,660);
- 573 (mojibake heads, e.g. 22466821 "Tribunal administratif d'Orlans", among the fallback targets), 578 and 579 (dissolve-vs-rebuild
  parity, which unit 4 inherits);
- 369 / 481 / 511 / 513: consumers of `gated_identifier` (buyer_key and buyer tokens, guard 3, the nhs-body canary);
- 530 (the v4 sub-class; its census `guid-census` job 2655 counted 3,511 single-use FR "ted other" rows of this shape, 0 spanning
  several Tenders, 0 stable, and 10,548 `nibble_only` values — the 3:1 ratio random hex gives);
- 312 / 300 design §84-85 (the "merge-inert singletons" stance; 530 upheld it for the class and amended it for the twin
  sub-class);
- 365 (the `national` catch-all that admits these values).

## Verify

    python3 -c "import json;d=json.load(open('.scratch/tender-db/530-triage/guid_census_2655.json'));b=d.get('body',d);b=json.loads(b) if isinstance(b,str) else b;print(b.get('nibble_only'), [p for p in (b.get('by_platform') or []) if 'other' in str(p)][:3])"

- **done**: a decision is recorded here (refuse the shape at the gate with an R5-style read passed, or decline with the cost
  written down) and, if refused, the standing rows are dissolved and a fresh census reads 0 single-use rows of the shape.
- **open (2026-10-19)**: job 2655 reports 10,548 nibble-only values and 3,511 single-use `ted other` v4 rows of the FR shape;
  the whole shape population (estimated ~225k) is unmeasured.

## What was found (issue 530 unit 2)

- One FR platform (Avenue-Web, per the 530 judges' reads; notice 277 carries `FD29FC2F-A2BF-49A1-9B234B870B531C27`) publishes
  BT-501 as 35 hex characters with dashes at offsets 8, 13 and 18 only (8-4-4-16), apparently a fresh random key per ORG section
  per notice.
- About 1/64 of random hex passes `idgate::uuid_v4` (version nibble 4 and variant 8-b), 3/64 carry the version nibble but fail the
  variant (`nibble_only`). Job 2655: 3,511 v4 passes in the FR `ted other` cell (all single-use, 0 multi-Tender, 0 stable) and
  10,548 nibble-only values — consistent with ~225k rows of the shape overall.
- Avenue-Web Systèmes also has 191 one-mention 32-hex rows (6 of them v4). *(Triage 2026-10-21: this figure comes from a
  530 judge's read (`530-triage/judge_coverage.md:194`) and its provenance is not recorded. It is not the class size: the name
  holds 62,080 rows (334, job 574, 2026-09-01) and the `avenue-web systèmes` name_norm group 68,239 (428, all countries).)*

## Proposed units

1. **Size (S/M).** Extend `guid-census` (or a sibling arm) to the raw-shape class: every organization whose stored identifier is
   32 hex and whose mentions' raw BT-501 is the 8-4-4-16 shape, regardless of `uuid_v4`; report single-use / multi-Tender / stable
   by platform and year, and whether any value recurs.
2. **Decide (S).** Starting bar = 530-F's R5: every mention carries the 35-character shape, 0 rows of the shape span ≥ 2 Tenders,
   0 are stable, and 15 FR fallback targets read 0 W. Refuse at the gate by the raw shape (the publisher's own format, a category
   argument like idgate's `notice_number`) or decline with the cost recorded.
3. **Build and dissolve** per 530's mechanics (per-mention rehome/dissolve, frozen keep-list if any value recurs).

## Triage (2026-10-21, wf triage-575-578)

Files: `.scratch/tender-db/575-triage/` (local_scan.py/.json, read1..4 *.out, get3_ccira.json). One measurer and two adversarial
skeptics took part, and both skeptics found the decision sound. The owner checked every code line below at HEAD 969afba and
re-read the cited data files. The Status line above is the corrected one; skeptic claims that could not be confirmed were
dropped (listed at the end). No build was run and nothing was written to the box.

### How the value is keyed (code, HEAD 969afba)

- A mention's identity is set at `project.rs:6073` from `gated_identifier` (`:5322`, signature `(raw, scheme, country)`, which
  takes **no name**). `DENIED_SCHEMES` is empty (`:12111`), so the publisher's scheme `eu` does not stop the value.
- `normalise_identifier` (`:11835`) → `normalise_identifier_with` (`:11854`) strips every non-alphanumeric and upper-cases the
  rest (`:11889-11891`). That leaves 32 hex. The VAT arm cannot take it (the body is 30 characters, over `VAT_BODY_MAX` 14 at
  `:12214`), no `REGISTER_PREFIXES` entry matches, and the value ends in the 365 catch-all `gated(national())` (`:12060`) under
  `register_jurisdiction(FR)`. The stored identifier cannot tell 8-4-4-16 from 8-4-4-4-12. Only
  `organization_mentions.raw_identifier` keeps the shape.
- `idgate::census` flags the value `hex_hash` (`idgate.rs:137`: 16 or more characters, all 0-9/A-F, at least one letter), but
  that flag feeds the census only. `condemns` (`:453`) leaves hex out on purpose (312). `uuid_v4` (`:479`: nibble 13 = `4`,
  nibble 17 in 8-b) passes 1/64 of random hex.
- 530-T's candidate test is `national` + `uuid_v4` over `gated_identifier` (`guid_twin.rs:287-288`; `guid_twin_gated_v4`
  `:342-345`; the gate's pre-check `:473`). It also needs 2 or more distinct v4 values for one name in one copy. Notice 277's TA
  Paris pair has one v4 value (`FD29FC2F-A2BF-49A1-9B23…`) and one non-v4 value (`FD29FC01-04FA-2E6D-…`, nibble 13 = `2`), so
  the gate returns early. 530 reaches almost none of this class.
- Every other consumer runs `gated_identifier` too:
  - `published_mentions` (`:5906`, `mentions_and_dropped_gated(gate=false)`), which feeds 369's buyer_key and 481's tokens;
  - 511 guard 3 (`:5023-5025`);
  - 513's nhs-body canary;
  - store's `GuidTwinGatedV4` fn-pointer type (`guid_twins.rs:3331`), used three times (`:3726`, `:3852`, `:3940`), all
    inside `guid_twin_recurrence` (`:3634`). Restore does not use it.

### Local data (no box load)

- `local_scan.py` over the cached notice bodies under `.scratch/tender-db`: 1,800 notices (a skeptic's re-run got 1,801) and
  5,115 ORG BT-501 values; 13 are cf35, all FRA, in 4 notices (24690547, 24887461, 27863637, 44493442), none v4, none on a
  buyer. The fixture `crates/ingest/tests/fixtures/eforms/cn-fa-expected-00586487-2024.xml` (marches-publics.info/mpiaws/
  index.cfm) and box notice 277 (read 1) bring the sample to **6 ORG sets: 5 notices + 1 fixture**. The read-4 notices
  returned single sections only and are not counted.
- Role map: per-notice ORG sections from the copy's own OPT-300/OPT-301 refs. Bodies are under `551-lot-sum/b5|b6/…`, read only.

| copy | SProvider | Buyer | AddInfo | ReviewOrg / Mediator / ReviewInfo | Tenderers |
|---|---|---|---|---|---|
| 24690547 | 45DD4EEE-… | 50902 | TARN HABITAT **45DD4F77-…** | TA Toulouse 45DD4F87-… | — |
| 24887461 | F469B2AA-… | 50902 | TARN HABITAT **1923911** | TA Toulouse ×3: F469B2D8- / F469B2E7- / F469B2F5-… | 1633220-1-{1,2,3}-1 |
| 27863637 | 35FACF6C-… | 5041 | — | TA Paris 35FACF8B- / — / Greffe TA Paris 35FACF9C-… | 1668803-1-{1,2}-1 |
| 44493442 | 54BD44A7-… | 5041 | — | TA Paris 54BD44C8- / — / Greffe 54BD44DC-… | — |
| 277 (box) | FD29FBDB-… | 67034 | — | TA Paris ×2: FD29FC01- (ORG-0003), FD29FC2F- (ORG-0004) | 1748141-1-0-1 |
| fixture 00586487 | D0E86039-… (`ted-esen`) | 30184 | — | TA Saint-Denis D0E86056- / CCIRA long form D0E86064- / TA Saint-Denis D0E86078-… | — |

- **Minting:** in all 6 copies the values rise in section order. The shared prefix is 6 hex in four copies and 5 in two (45DD4
  in 24690547, FD29F in 277). The 35-character 8-4-4-16 format is ColdFusion `CreateUUID`'s, and the platform is a .cfm app;
  that the key is CreateUUID output is an inference.
- **Twins:** one body printed twice in one notice gets different values in 3 of the 6 copies (TA Toulouse ×3, TA Paris ×2,
  TA Saint-Denis ×2).
- **Recurrence:** no value recurs anywhere in the sample.
- **Roles with cf35:** SProvider 6/6, Lot-ReviewOrg, Lot-ReviewInfo and Lot-Mediator every time. **AddInfo is mixed:** the
  same TARN HABITAT section carries cf35 in 24690547 and the numeric `1923911` in 24887461, while the buyer is 50902 in both.
- **Buyers:** 0 of 6 carry the shape. Each carries a platform account number, stable across notices: 5041 binds org 1166503
  in both 27863637 and 44493442 (skeptic read).
- **Tenderers:** they carry a per-procedure `<procedure>-1-<lot>-<n>` key, a separate class (U5).

### Box reads and GETs (each box read plan-probed locally first; no 408/429)

1. `organization_mentions WHERE notice_id=277 LIMIT 40` → 5 rows (the 277 line above). ORG-0001..0005 → orgs 859..863.
2. `organizations WHERE id IN (859..863)` → 5 rows, all FR / `national` / provisional 0. Stored values:
   `FD29FBDB9AE296A7…`, `67034`, `FD29FC01…`, `FD29FC2F…`, `1748141101` (the tenderer key with its dashes stripped).
3. `organization_mentions WHERE organization_id IN (22763203, 22778244, 22780012, 22394451) LIMIT 20` → 20 rows, all of
   22394451 (the LIMIT cut the query short). The same CCIRA name ("Comité Consultatif Interrégional - Prefecture de Région
   SGAR") carries a stable raw `1331485` (scheme NULL) on ORG-0004 across notices 23668886…23895037.
4. The same seek on the three hex rows → 3 rows: `C779DE6D-F66E-FD50-…`, `613592CD-D798-998F-…`, `708087E0-F304-63DA-…`, scheme
   `eu`, ORG-0004, notices 24267121 / 24288162 / 24290540. So the same body went from a stable number to per-section keys
   between ~23.9M and ~24.27M. That points to an earlier publishing era; U1's year split will show it.
5. (skeptic) `organization_mentions WHERE notice_id IN (27863637, 44493442)` → 10 rows. Each cf35 section mints its own
   consecutive row (24606517–19, 30934846–48), so Avenue-Web Systèmes gets a new row in each notice. Buyer 5041 binds 1166503 in
   both. The tenderer keys mint rows too (24606520–21).

- GETs (3 of 6): 1 and 2 went to `tender-db.zebreus.click` and never reached the server (one empty response, one TLS host-name
  mismatch). The serving host is `tenders.zebreus.click`.
- GET 3: `/v1/organizations?name_prefix=Comité consultatif interrégional&country=FR&limit=60` → 200:
  - 1163289 "Comité consultatif interrégional", identifier NULL, 407 mentions;
  - 19913046 "…Prefecture de Région Sgar", NULL, 81;
  - 22394451 (`1331485`), 41;
  - 56 one-mention rows of the SGAR name, each with its own 32-hex key; `more: true`.

### 530 records

- **Census 2655** (`530-triage/guid_census_2655.json`, org max_id 31,970,152):
  - 301,045 candidates (rows with `length(identifier) IN (32, 36)`, **any content**: `guid_census.rs:92`);
  - 81,323 pass v4; 10,548 are `nibble_only`;
  - the `ted other` cell: 3,511 rows / 3,511 mentions, multi_notice 0, multi_tender 0, stable 0, one_tender_twin 18.
  - That cell is defined as `notices.source = ted` plus raw formatting `other`, meaning neither a 36-character dashed nor a
    32-character plain value (`raw_shape`, `guid_census.rs:470-481`). It is **not** a FR cell or a cf35 cell by definition;
    tying it to cf35 rests on the samples and the nibble ratio.
  - `by_country` FR: 3,606 rows (3,597 single-use, twin 18), so whether all 3,511 are FR is unverified.
- **Plans 2660 / 2671:** `planned_by_country` FR = 18 rows / 18 mentions. Targets are big identifier-NULL review-body rows:
  TA Toulouse 1163070 (32,566 mentions), Lyon 46,473, Cergy-Pontoise 35,410, Toulon 22,272 (22,269 in 2660), Montpellier 27,192.
- **KEEP_530** (`crates/ingest/sdk/guid-keep-530.tsv`): 9 FR values, all `multi_tender`. The `ted other` cell has 0
  multi_tender, so none of the 9 is from it.
  - The three `doe+ted …other` cells hold 22 multi-Tender rows of unknown raw shape, and the KEEP lookup ignores country (A4).
    So U1 *measures* the overlap rather than assuming 0.
- **334** (job 574): 62,080 rows named "Avenue Web Systèmes" under 62,080 distinct identifiers. **428**: the
  `avenue-web systèmes` name_norm group holds 68,239 rows across all countries (Sep 20 snapshot).
- **365** (`:312-313`) records the raw `eu`/`EU` scheme at 252,658 mentions, with no scope stated. If that count were
  corpus-wide, ~220k cf35 mentions plus the Avenue-Web buyer and tenderer `eu` mentions would exceed it, so U1 reports the
  `eu` total beside the cf35 count.

### Root cause

- **Publisher side.** Avenue-Web Systèmes' TED eSender (AWS-Achat, marches-publics.info) mints a fresh 8-4-4-16 key for every ORG
  section of every notice it generates, and publishes it as BT-501 under scheme `eu`:
  - the platform's own Procedure-SProvider block;
  - the ReviewOrg, ReviewInfo and Mediator blocks;
  - sometimes the AddInfo block.
  Buyers get a stable account number; tenderers get a per-procedure key.
- **Our side, the split.** `normalise_identifier_with` strips the dashes and the 365 catch-all admits the value as a `national`
  FR key. The resolver binds by (FR, national, value), and every value is unique, so every mention mints its own row. The value
  never recurs, so these merge-inert singletons (300 §84-85) never heal.
- **Our side, nothing refuses it.** `condemns` omits hex on purpose (312). `uuid_v4` covers only the 1/64 that pass by chance,
  and 530-T fires only when 2 or more of those share one name in one copy.
- **Where the shape is visible.** `condemns` sees only the stored, undashed value, so a value-only rule there cannot express
  the shape. Both `normalise_identifier` and `gated_identifier` see the raw string. `gated_identifier` is the right one: its doc
  (`project.rs:5314-5318`) makes it the layer that decides mention admission, and `normalise_identifier` also feeds callers
  whose witnesses would shift: the repair planners (`supervisor.rs:11471`, `:11829`, `:11937`), the `register_identity`
  injections (`:14149`, `:14575`, `:14675`) and `crosswalk.rs:1171` / `:1278`.

### Size

About **217k–223k org rows** of the cf35 shape, each with about 1 mention (so about 220k mentions), as of org max_id 31,970,152
(census 2655, 2026-10-19). This is an **extrapolation, not a measurement**.

How the range is derived:
- **Estimate:** 64 × 3,511 = 224.7k (95% interval 217k–232k).
- **Cap:** 301,045 candidates − 77,812 v4 rows outside the `ted other` cell = 223,233. The cap covers every other 32/36-character
  identifier, hex or not, so other sources come to at most about 6k.
- **Cross-checks:**
  - `nibble_only` 10,548 ≈ 3 × 3,516;
  - 3/63 × ~221k non-v4 random hex ≈ 10.5k expected.

Recurrence and the comparison with 300's class:
- **Recurrence.** v4 is a uniform 1/64 random subsample, and none of its 3,511 rows spans 2 notices. So at 95% at most 0.085%
  of the class can recur (rule of three, ~190 rows). Stable rows: 0.
- **Against 300's hex class.** That is 273,982 rows (run 1332, 2026-08-28; 23.5% of ~1.17M id-bearing orgs then), and both
  v4 and cf35 fall inside it. So cf35 is ≈80% of that class and ≈19% of the August id-bearing denominator. But cf35 ~220k plus
  non-cf35 v4 77.8k ≈ 298k exceeds 273,982, which fits only if the class grew by at least ~24k since August. That is
  plausible (the vendor name went 62,080 → 68,239 between 334 and 428) but unverified; U1 re-takes the class total.

Composition:
- 62k–68k rows are the eSender row. That rests on SProvider cf35 in 6 of 6 copies and census samples 498, 6402, 6644, 7414,
  7442 (role other); 334 never recorded the identifier shape.
- The rest are TA de <city> rows, greffes, CCIRA rows and AddInfo copies of buyers. That fits about 3.3 cf35 sections per
  notice over roughly 65k Avenue-Web TED notices.

Growth: the class grows with every Avenue-Web notice (44493442, a recent one, carries it); the monthly rate is not measured.

Confidence:
- High on the order of magnitude, on single use, and on the mechanism.
- Medium on the role split and on buyers being free of the shape (6 ORG sets).

### Decision

**Unit 1 first:** a dry census arm that sizes the raw cf35 shape over every 32-hex row, not just the uuid_v4 passes. It is the
input to unit 2. **The direction is a platform-key refusal by raw shape in `project::gated_identifier`, not a 530-T-style
rule.** The value is proven per section per notice: sequential minting in 6/6 copies, two values for one body in one notice
in 3/6, and 0 recurrence in a 1/64 random subsample of 3,511 rows. Most of the harm is the cross-notice split, one eSender row
per notice (~65k), which no within-notice rule reaches. Placing the rule in `gated_identifier` means everything downstream sees
the refusal: published identity (buyer_key), guard 3, the canary and 530's `guid_twin_gated_v4`. The rule rests on what the
value is (a category argument, like idgate's `notice_number`), not on name evidence.

Alternatives, one line each:
- (a) Decide from the 1/64 subsample alone. It bounds recurrence at ≤0.085%, but not the buyer-role share, the targets, the
  homonyms or the exact dissolve set. Rejected.
- (b) Widen 530-T to the raw shape. It reaches only copies that print a body twice (3 of 6 sampled), never the eSender.
  Rejected; U1 measures its reach for comparison only.
- (c) A rule in `idgate::condemns`. It sees only the stored undashed value. Impossible.
- (d) A rule in the gate block beside 530-T (`if gate {…}`, `:6083`). `published_mentions` (gate=false) would keep cf35 on
  buyer_key, and `guid_twin_recurrence` would report every refused cf35-v4 mention on a NULL row as reverse drift unless
  `guid_twin_gated_v4` learned the shape. Choose it only if U1 finds cf35 on buyer sections.
- (e) Decline now. That leaves ~220k rows, growing, too large to decline unmeasured.
- (f) A rule at the top of `normalise_identifier_with`. It sees the raw value too, but it would move the repair-planner,
  register_identity and crosswalk witnesses listed above, and it is the string converter, not the admission layer. Rejected.

**The name guard is open, and U2 settles it before U1 counts the blank-name case.** `gated_identifier` has no name parameter.
530's A6 city screen is a planner-side DB read: `notice_texts` BT-513 plus a reverse walk over the target's mentions
(`530-triage/decision.md` §A6). The gate itself must stay pure (`guid_twin.rs:280`). So:
- (i) **Guarded.** Add the mention's name to `gated_identifier` and to every caller: guard 3, the canary,
  `guid_twin.rs:287`/`:344`, test `:2631`, and store's `GuidTwinGatedV4` type with its three `guid_twin_recurrence` sites.
  Only a *compiled* 575 list goes into the gate; A6 runs in the dry plan, and any W name it finds joins the list.
- (ii) **Unguarded.** Keep the signature. Parity stays automatic across every consumer, and the homonym risk is carried by
  U1's target screen and R2 at 0 W.

Either way the 575 list must be its own: editing `GUID_TWIN_UNSAFE_NAMES` changes 530 and obliges a re-run of 530's
recurrence arm. It must also bind the fixture's long CCIRA form ("Comité consultatif interrégional de réglement amiable des
différends ou litiges relatifs aux marchés publics"). An exact match on the bare "Comité consultatif interrégional" would not,
so the list needs a prefix rule or that spelling.

### Units

- **U1 Size (S/M, dry, in-app).** Add a `hex-census` arm, or a guid-census arm with an `is_guid` = 32-hex predicate. It walks
  the `GUID_CENSUS_ORGS_SQL` windows and reads mentions through `organization_mentions_org`. Each `raw_identifier`, trimmed and
  case-blind, is classified as cf35 (`^[0-9A-F]{8}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{16}$`), rfc36, plain hex32 or other.
  The report:
  - **Rows:** rows, mentions, notices and Tenders; single-use, multi-notice, multi-Tender and stable.
  - **Splits:** by country, by first year/profile (CCIRA's earlier `1331485` era), by platform and by role.
    - Role comes from `tender_version_parties` at the mention's notice and section: SProvider, ReviewOrg/ReviewInfo/Mediator,
      AddInfo, Buyer, Tenderer.
    - Buyer and AddInfo counts are split by shape.
    - Top 50 names.
  - **Cross-checks:** recurring values; rows mixing shapes; KEEP_530 overlap (bar 0); the `eu`-scheme mention total; the
    `hex_hash` class total (vs 273,982).
  - **Bars for U2:**
    - cf35 mentions on a buyer role (the bar for placing the rule in `gated_identifier`);
    - blank-name cf35 mentions (511 guard 3; matters only under U2 option ii);
    - within-notice twins (the reach of option b).
  - **Targets:** per target key, the `(name_norm, register_jurisdiction(country))` identifier-NULL row (id, mentions, found or
    missing), with A6 city flags.
  - **Other classes:** plain hex32 one-mention rows by platform, and a count of the `<n>-<n>-<n>-<n>` tenderer shape (for U5).
  - Status: ready-for-agent (next) · Blocked by: 539's drain for its deploy and run (build any time).
- **U2 Decide (S).** Proposed rule: refuse by raw shape in `gated_identifier`, beside `scheme_never_keys`. The argument is
  category; the proof is sequential minting, two values for one body in one notice, and 0 recurrence.
  - Choose guard (i) or (ii) (Decision).
  - Bars: U1 shows 0 multi-Tender (or a frozen value keep-list), 0 stable, 0 KEEP_530 overlap, and 0 buyer-role mentions (or
    369's buyer_key churn written down).
  - R2 to A5's standard, at 0 W:
    - a random 60 stratified by role (SProvider, review, mediator, AddInfo);
    - the top 30 targets;
    - every city-flagged target and every CCIRA homonym (1163289 / 19913046 / 22394451 and the SGAR rows).
  - Otherwise decline, with the cost recorded.
  - Status: ready-for-agent · Blocked by: U1.
- **U3 Build (M; M/L under guard i).** The gate in `gated_identifier`, with R0 tests.
  - **Fixture:** the cn-fa fixture already has the SProvider case (ORG-0001 Avenue-Web, `ted-esen`, D0E86039-…), the
    buyer-numeric case (ORG-0002 Département de la Réunion, 30184), 4 cf35 values and a TA Saint-Denis twin. Only assertions
    are missing.
    - Today it is used parse-only (`tests/eforms.rs:89` sweep, `:1133`, `:1492`) and is not in `project_golden`, so the gate
      changes none of this fixture's current expectations.
  - **New tests:**
    - the guard binding a CCIRA name (guard i);
    - 530-T's candidates and `guid_twin_gated_v4` dropping cf35 v4 values (277's `FD29FC2F-…` shape).
  - **Also:** a /docs sentence and a CHANGELOG entry. The planner arm reuses 530's id-set dissolve with the A2 per-mention
    rehome, through `DissolveTarget::Resolver`, not 440's lookup (578).
  - **Stock safety:** a refold re-resolves a recorded mention only when its name, country, raw identifier or scheme changed
    (`RecordedMention::publishes`, `canonical.rs:15644`). So deploying the gate moves no standing mention, and no 551/539/561
    refold can half-dissolve the stock. Only a from-archive rebuild diverges until U4.
  - **Ship with or just before U4.** The resolver's fallback lookup (`canonical.rs:23694`) seeks `(name_norm, country)` in id
    order with `identifier IS NULL` as a residual filter. While the identified `avenue-web systèmes` rows stand, a NULL row
    minted at the newest id is found only after walking them (428's cost). That happens once per fold run per key: the run
    cache `name_of` (`:15370`) absorbs repeats.
  - Status: ready-for-agent · Blocked by: U2.
- **U4 Dissolve (L).** Uses 530's mechanics: id-set selector, windows by target key, 543 stop checkpoint, restore manifest, wet
  re-plan, tripwires (1 Tender, <100 mentions).
  - **Expected:** about 220k rows and 220k mentions moved, about 65k Tenders changed, and about 220k ids answering 404 once with
    no redirect (a served change). That is about 20× 530's 10,995 rows.
  - **Ordering:**
    - on the single writer after 539's drain;
    - outside 560's drain-free mornings;
    - with 551's standing rule satisfied: org moves feed path W's R1/R6, so a port-diff re-run comes first while W is on (530's
      Status, 579).
  - **Parity:** inherits 579. 530's wet path left head tables that a refold rewrites, so U4 waits for 579's decision or refolds
    its touched Tenders.
  - **R3:** a fresh census reads 0 single-use cf35 rows except the guard's exclusions.
  - Status: ready-for-agent · Blocked by: U3; 539; 579.
- **U5 File** a new issue: Avenue-Web tenderer keys `<procedure>-1-<lot>-<n>` (stored dashless, e.g. 1748141101, FR national)
  give a winner a row per procedure (27863637: 24606520–21). Dash-stripping can also collide inside one procedure: lot 11 /
  n 1 and lot 1 / n 11 both store `<procedure>11111`. These rows are higher-value than this class. Size them with U1's count.
  - Status: ready-for-agent · Blocked by: U1 (count).
- **U6 Correct the issue text: DONE 2026-10-21.** The Relates-to entries (334, 428, 573, 578/579, 369/481/511/513) were added and
  the "191" bullet annotated, both in this edit.

### Interactions

- **551 B6.** The flip is on main (e718684, deployed). U7 drained and U8 passed per 969afba (10:55Z); 551's Status line is not yet
  rewritten, so re-check it before landing code.
  - The held switch-off twin d569107 (`hold/551-switchoff`) touches `supervisor.rs`, `v1/docs.rs`, `v1/sql.rs` and
    `canonical.rs`. U1's job arm and U3's docs sentence need a text rebase if it ever lands.
  - No data overlap: 551 is lot-sum award values, 575 is organization identity.
  - U4 is a W-input drain, so it takes 551's port-diff rule.
- **539.** Its drain (repair-notice-instants plus the DÖE R3 refold) holds the single writer next. U1's deploy and run queue
  behind it. This is a scheduling constraint, not a data dependency.
- **530 (DONE).**
  - The 530-T gate, planner, recurrence arm (`guid_twin_recurrence` via `gated_v4`) and restore all sit downstream of
    `gated_identifier` or the gate. Under guard (ii), a refusal there drops cf35 v4 values from every 530 candidate set at once,
    and the recurrence arm's reverse-drift half stays quiet with no store change. Under guard (i), store's `GuidTwinGatedV4`
    type changes. Either placement touches 530's store code once a name is involved.
  - The 18 FR rows 530 dissolved stay where they are: a rebuild under the new rule lands them on the same NULL targets.
  - KEEP_530: the 9 FR values are not from the `ted other` cell; U1 asserts 0 overlap overall.
  - Do not edit `GUID_TWIN_UNSAFE_NAMES`.
- **579 / 578.** Dissolve-vs-rebuild parity. U4 uses the Resolver arm and waits on 579.
- **369 / 481.** buyer_key and buyer tokens follow `published_mentions`, which is harmless only if buyers never carry cf35 (0 of 6
  sampled; U1 measures it).
- **511 guard 3 / 513 canary.** Both rebuild identity through `gated_identifier`. The guard design decides whether a blank-name
  cf35 party keeps "identified blank".
- **428.** Per-run fallback walk (U3).
- **334.** Answered (Relates to).
- **573.** Targets are chosen by the mention's published name, while served heads may carry mojibake.
- **365 / 312 / 300 §84-85.** The catch-all, the hex stance and the singleton stance this issue amends for one platform class.
- **572.** No conflict.

### What the skeptics corrected (confirmed and applied above)

1. The sample is 6 ORG sets (5 notices + 1 fixture), not "6 notices plus 1 fixture" or "3 of 7": twins are in 3 of 6,
   sequential minting in 6 of 6, and the shared prefix in 24690547 is 45DD4 (`45DD4EEE-…`).
2. AddInfo is mixed: TARN HABITAT carries `1923911` in 24887461 and cf35 in 24690547, with buyer 50902 in both. The root
   cause now says "sometimes the AddInfo block".
3. The fixture already has the SProvider and buyer-numeric cases; only assertions are missing, and its uses are parse-only.
4. The fixture's CCIRA is the long form, so the 575 name list needs a prefix rule or that spelling.
5. "19% of 300's 273,982 class" was garbled. cf35 is ≈80% of that class and ≈19% of August's ~1.17M id-bearing orgs (a stale
   denominator). The v4 + cf35 total implies the class grew by ≥ ~24k since August; U1 re-takes it.
6. The candidates are any 32/36-character identifiers (`guid_census.rs:92`), and `ted other` is a TED + raw-shape-`other`
   cell (`:470-481`), not a FR or cf35 cell. FR has 3,606 rows in all, so U1 splits by country and classifies the shape
   directly. The size is labelled an extrapolation.
7. `normalise_identifier` also sees the raw value. That makes placement (f) a third option, rejected for the reasons above,
   and the root-cause sentence was rewritten.
8. `gated_identifier` takes no name, and A6 is a DB read, so U2/U3 were restated as guard (i) or (ii) with a compiled list
   only. The 511 guard-3 effect depends on that choice. "Parity holds everywhere at once" holds only under (ii).
9. Stock safety (`publishes` compares raw fields only) and the 551 file overlap were added; the flip itself is already on
   main, so only the held d569107 remains a rebase risk.
10. The CCIRA `1331485` stable-number era, 365's unscoped `eu` total and the KEEP overlap through the `doe+ted …other` cells
    became U1 report lines.

Corrected or dropped, because the owner's check disagreed or could not confirm:
- The skeptic said `GuidTwinGatedV4` is used by "the recurrence and restore arms". All three sites are in
  `guid_twin_recurrence`; restore does not use it.
- The skeptic said "every later avenue-web lookup still walks ~68k rows". It is once per fold run per key: the run cache
  `name_of` (`canonical.rs:15370`) absorbs repeats.
- The measurer explained the "191 one-mention rows" as "a 200-row GET page". No request log backs that, so the explanation
  was dropped and only the class-size correction kept.

### Verify note

The `## Verify` block above is unchanged. It reads a frozen local file, so it can only print the open state. Once U1 lands, point it
at that unit's census report; once U4 lands, at U4's R3 census, the last open unit.
