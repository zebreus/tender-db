# 177 — r208-era tender values never project (and `VALUE_COST` is three different facts)

Status: fix implemented 2026-08-10 (orchestrator) — red→green tests + epoch 3;
awaiting deploy + r208 refold (which doubles as issue 175's first pipelined-fold
measurement). Implementation notes on top of the design below:
- `amount_target` in project.rs routes plain `TED-VALUE_COST` structurally: inside
  RESULT_KINDS → the binder owns it (no fact); object scope on a notice CARRYING
  result sections (award-family marker) → `result_value`; else `estimated_value`.
  Form codes were not needed — result-section presence is the era-honest signal.
- SEMANTIC FINDING the design notes missed: many r208 F02s publish their ONLY
  estimate in the framework block (`F02_FRAMEWORK/TOTAL_ESTIMATED/…VALUE_COST`,
  II.1.4) — the committed 2014 fixture is one; "restated copies lose" would have
  left them valueless. `TED-TOTAL_ESTIMATED.VALUE_COST` therefore maps to
  `estimated_value` in AMOUNTS (a framework CN's headline estimate), while the
  award-block `INITIAL_ESTIMATED_TOTAL_VALUE_CONTRACT` prefix stays unprojected.
- `RANGE_VALUE_COST` decision recorded: stays unprojected (a range is not one
  estimate).
- Tests: an_r208_contract_notice_projects_its_estimated_value (red-checked: None
  before the fix), an_award_notice_files_its_values_as_results_not_estimates (the
  F18 carries all three shapes at once: no estimate fact, TOTAL_FINAL_VALUE →
  result_value 168_110_000 RON, binder keeps awarded_cents), and the 176 matrix
  gained its value column.

Filed 2026-08-09 (orchestrator) — found while building issue 176's matrix.
Kind: correctness (silent per-era data gap on a headline canonical field)
Severity: MEDIUM — same class and era as issue 174, headline field `estimated_value`
Blocked by: —
Relates to: 174 (the class), 176 (the guard that will assert the fix), 132/134
(negative-amount analysis reads `estimated_value`), 99 (epoch ledger)

## The gap

`project.rs::AMOUNTS` maps `TED-VAL_ESTIMATED_TOTAL` (r209) and `BT-27`
(eForms) to `estimated_value` — and nothing from the r208 era. The r208 F02's
II.2.1 estimated value parses as a plain `TED-VALUE_COST` amount (inside
`COSTS_RANGE_AND_CURRENCY`, in the OBJECT section), reaches the parsed layer,
and projects to nothing. The committed fixture `r208/f02-000333-2014.xml`
carries `VALUE_COST FMTVAL="900000.00"` GBP; after projection the canonical
layer has zero `estimated_value` rows. Every 2011–2016 contract notice is in
the same state — exactly issue 174's shape, one column over.

## Why the fix is NOT one mapping line

`TED-VALUE_COST` is one field id carrying three different facts, told apart
only by context:

1. **CN, OBJECT section** (`COSTS_RANGE_AND_CURRENCY`): the II.2.1 estimate →
   should be `estimated_value`.
2. **Award block, plain** (inside `AWARD_OF_CONTRACT*` → RESULT_KINDS
   sections): the awarded value — ALREADY consumed by the results binder
   (project.rs `raw_results`, "take it only when VAL_TOTAL is absent"). An
   unconditional AMOUNTS entry would double-file every award value as a
   tender/lot `estimated_value` fact. A result-section exclusion in the fold
   handles this half.
3. **CAN, OBJECT section** (`TOTAL_FINAL_VALUE > COSTS_RANGE…`): the II.2
   TOTAL FINAL value of the procurement — a RESULT total, NOT an estimate,
   and it is NOT inside a RESULT_KINDS section (see the committed
   `r209/f18-defence-001420-2019.xml`, which has all three shapes at once).
   A section guard alone therefore still misfiles CAN totals as estimates.
   Arguably this one should become `result_value` at Tender scope — which
   r209 CANs get from the global `VAL_TOTAL`.

Already disambiguated by the parser (no action needed): the award block's
initial-estimate variant arrives PREFIXED
(`TED-INITIAL_ESTIMATED_TOTAL_VALUE_CONTRACT.VALUE_COST`) and the F02
framework restatement as `TED-TOTAL_ESTIMATED.VALUE_COST`
(`FIELD_PREFIX_WRAPPERS`, r209/parse.rs) — both stay unprojected by the same
form-value-wins rule as 174's `DT_DATE_FOR_SUBMISSION`.

## Recommended shape (for whoever implements)

Projection-side only — do NOT touch parser field ids: parsed rows are on disk
for 14.2M notices, and changing ids means a re-PARSE (reclaim-all class),
while a fold-mapping change is an epoch bump + refold (issue-99 machinery,
proven twice).

In the fold's Amount arm, route plain `TED-VALUE_COST` by context instead of
through the flat AMOUNTS table:
- enclosing section in RESULT_KINDS → skip (the results binder owns it);
- else if the notice is an award-family form (subtype/TD says CAN) → the
  TOTAL_FINAL_VALUE reading: `result_value`;
- else (CN family, OBJECT scope) → `estimated_value`.

The notice form is available at fold time (`notice_subtype` /
TD_DOCUMENT_TYPE codes ride the parsed layer). Check whether the coded-section
`VALUES_LIST/VALUES TYPE="GLOBAL"` (field `TED-VALUE`, captured `@TYPE`)
should stay unprojected — recommendation: yes, coded copies lose to form
values (174 precedent), but confirm density on a snapshot before deciding the
r208 CAN `result_value` story rests on TOTAL_FINAL_VALUE alone.

Also decide: `RANGE_VALUE_COST` (low/high ranges) — recommendation: leave
unprojected, a range is not one `estimated_value`; record the decision here.

Acceptance:
- red→green fixture test: f02-000333-2014 projects `estimated_value`
  90_000_000 cents GBP at Tender scope; f18-defence keeps its award value in
  `lot_results` AND gains no `estimated_value` row; its TOTAL_FINAL_VALUE
  routing decided and asserted either way.
- PROJECTION_EPOCH bump (ledger row in canonical.rs) + scoped r208 refold on
  deploy — which is also issue 175's first pipelined-refold measurement.
- issue 176's matrix gains the value column.
- issues 132/134's negative-amount ratio analysis re-checked after the era
  refold (their populations were measured without r208 estimates present).

## Deploy + refold, observed live (2026-08-10)

Deployed in e5dfac2 (with 116/87/176). `refold ted-export-r208` re-queued
2,694,814 notices in ~50 min. The follow-up `project rebuild=false` took the
issue-58-v1 legacy fallback (any legacy notice in the delta → full corpus)
and, with every tender epoch-stale from the 2→3 bump, is rewriting all
7,925,880 tenders through the chain-compare path: 674,798 tenders /
1,500,104 versions at t+170 min (~4.5K tenders/min in the legacy-heavy
segment), memory flat, journal clean. That full-corpus cost is filed as
issue 179 — this refold's wall-clock is the baseline there, NOT a clean
pipelined-fold measurement for issue 175 (the fallback path dominates).
Era verification (2011-2016 tenders serving estimated_value via API)
pending fold completion.
