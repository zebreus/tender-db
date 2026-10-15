# 550 — R2.0.9's standard award blocks lose their award date, and a winner-silent award reads `clos-nw`

Status: ready-for-agent — unit 1 MEASURED 2026-10-15 (below): of a 2016–2023 public sample, 86 % of legacy results carry `DATE_CONCLUSION_CONTRACT` and all of them read `decided` NULL; 78 of their 82 `clos-nw` are winner-silent awards (≈ 12 % of the standard R2.0.9 results wrongly claim no award). Decided: build unit 2 (the projection fix) next — ahead of 533's build, which reads `clos-nw` — then size the refold. Next: unit 2.
and the owner's confirmation on the public API and the committed fixtures). Next: unit 1, below.
Kind: data quality (legacy TED results: `tender_version_lot_results.decided_*` and `decision`)
Relates to:
- 255 (DONE): slice 2 mapped `TED-CONTRACT_AWARD_DATE` (r208, and the r209 DEFENCE form F18) into
  `tender_version_lot_results.decided_*`. Its gate is the F18 fixture; the standard r209 forms use another element.
- 257 (DONE): an award block with an award DATE but no winner and no value is NULL, not `clos-nw`; `clos-nw` is the
  default only for a block with no award evidence at all.
- 533: its closing rule closes a head whose own results are all `clos-nw`, so an inferred `clos-nw` on a block that
  in fact awarded closes a Tender wrongly (7241490, a Prüfsystem, is the case its judge found).
- 384: the destination predicate must list whatever the legacy reader consumes.

## What is wrong

R2.0.9's standard forms (F03 contract award, F06 utilities award, F15/F20 and kin, TED 2014–2023) put the award
outcome in `AWARD_CONTRACT/AWARDED_CONTRACT`, with the date as `DATE_CONCLUSION_CONTRACT`. Only the defence form
(F18) writes `CONTRACT_AWARD_DATE`. The parse layer types `DATE_CONCLUSION_CONTRACT` as a date
(`crates/ingest/src/r209/rules.rs:416`), so it is stored as `TED-DATE_CONCLUSION_CONTRACT`; `read_legacy_results`
(`crates/ingest/src/project.rs`, the `LEGACY_AWARD_DATE_FIELD | LEGACY_AWARD_DATE_FIELD_R207` arm) reads only
`TED-CONTRACT_AWARD_DATE` and the R2.0.7 `TED-DATE_OF_CONTRACT_AWARD`. Two consequences:

1. **No award date for the standard R2.0.9 era.** Every standard-form result reads `decided` NULL — winners included.
2. **A winner-silent award reads `clos-nw`.** A block that says `AWARDED_CONTRACT` (stored as the marker
   `TED-AWARDED_CONTRACT` = 1), with a contract number and a conclusion date but no contractor and no value, has no
   award evidence the reader recognises, so the end-of-function default makes it `clos-nw` (no award) — against the
   issue-257 rule, which would read it NULL had the date been read.

Committed fixtures with `DATE_CONCLUSION_CONTRACT`: `r209/f03-000988-2019.xml`, `r209/f06-208243-2017.xml`,
`r209/f20-000591-2019.xml`. With `CONTRACT_AWARD_DATE`: only `r209/f18-defence-001420-2019.xml` (and the r208 files).

## Repro (public API, 2026-10-15)

- Tender 8437191 (`Bf Niederwalgern, Umbau und barrierefreier Ausbau, Planungsleistung`, 2019): result notice
  20406301's `RES-1` holds `TED-AWARDED_CONTRACT 1`, `TED-CONTRACT_NO 19FEI38126`, `TED-DATE_CONCLUSION_CONTRACT
  2019-05-20`, `TED-LOT_NO 1`. Served: `lot_results[0]` = `decision "clos-nw"`, `decided null`, no winner.
- Tender 7241490 (2022, a qualification system): result notice 23099117's `RES-1` holds `TED-AWARDED_CONTRACT 1`,
  `TED-CONTRACT_NO 4600002099`, `TED-DATE_CONCLUSION_CONTRACT 2022-06-22`. Served: `clos-nw`, `decided null`.
- Tender 8437184 (2019, a named winner): `selec-w`, `decided null` — the date is lost even when the outcome is right.
- The neighbouring 2019 tenders 8437187–8437190 read the same `clos-nw`/null shape.

## Unit 1 — measured (2026-10-15, public API sample)

`sample_550.py` (session scratchpad): five TED Tenders with results per month, 2015-01 to 2023-08, every result block
read on `/v1/notices/{id}/content` — 778 results in 329 Tenders.
- 732 results are legacy-shaped. 632 (86 %) carry `TED-DATE_CONCLUSION_CONTRACT` (2016–2023), 40 carry the R2.0.8
  `TED-CONTRACT_AWARD_DATE` (2015), 60 carry neither.
- **All 632 read `decided` NULL** — the era has no award date (550 results `selec-w`, 82 `clos-nw`).
- **78 of the 82 `clos-nw` are winner-silent awards**: `TED-AWARDED_CONTRACT` present, no `TED-NO_AWARDED_CONTRACT`, no
  winner and no value. With the date read they read NULL under 257's rule. That is ≈ 12 % of the standard R2.0.9
  results claiming "no award" for a concluded contract, spread evenly over 2016–2022.
- Scaled to issue 255's 2,098,599 R2.0.9 award notices: on the order of 2 M results gain a date and a quarter-million
  `clos-nw` become NULL — the size of 534's refold class, so the rollout needs its own sized refold in a queue gap.

**Decision.** Build unit 2 now, ahead of 533's build: 533 closes an open head on an all-`clos-nw` result, and the
projection fix is pure and cheap. The refold is a separate, sized step (unit 3); 533's own `refold-open-head` re-projects
the open heads it touches, so 533 does not wait for the corpus refold.

## Units

1. **Measure (bounded).** On a sample of R2.0.9 standard award notices (one month's package is enough: count
   `notice_dates`/`notice_*` rows with `field_id = 'TED-DATE_CONCLUSION_CONTRACT'` per notice id range through a
   plan-checked bounded read, and the share of their result blocks with no contractor and no value), estimate how
   many results gain a date and how many `clos-nw` become NULL. Decide whether a refold of the R2.0.9 award notices
   is needed now or rides the next corpus refold, and whether 533's build must wait for it (533 closes a head only on
   an all-`clos-nw` result, so every wrongly inferred `clos-nw` on an open head is a wrongly closed Tender).
2. **Fix the projection.** Read `TED-DATE_CONCLUSION_CONTRACT` as `decided` in the same arm (named constant, the
   destination predicate updated per 384), and count the `TED-AWARDED_CONTRACT` marker as award evidence, so a
   block with it and no winner/value reads NULL (257) rather than `clos-nw`. Tests on the three r209 fixtures: the
   date lands in `tender_version_lot_results.decided_*` and `v_awards`; a winner-silent `AWARDED_CONTRACT` block reads
   NULL; a block with `NO_AWARDED_CONTRACT` still reads `clos-nw`; an R2.0.8 `CONTRACT_AWARD_DATE` block is unchanged.
3. **Roll out** per unit 1's decision (`refold-fields` over the notices carrying `TED-DATE_CONCLUSION_CONTRACT`, sized
   first; it is a corpus-scale writer, so it needs a queue gap and the 543 stop points), then the Verify.

## Verify

- `curl -s https://tenders.zebreus.click/v1/tenders/8437191 | jq -c '.lot_results[0] | [.decision, .decided]'`
  reads `[null,"2019-05-20…"]` (today `["clos-nw",null]`).
- `…/v1/tenders/8437184` reads a `decided` date beside its `selec-w`.
