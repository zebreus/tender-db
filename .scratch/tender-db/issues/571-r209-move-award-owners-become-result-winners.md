# 571 — an R2.0.9 T02 award's operator owners (`ADDRESS_PARTY`) become `lot_results` winners

Status: **DONE 2026-10-19** — unit 2 (21b2537, gate green 1,392 s) deployed 05:45Z (rev 3f32b59) and drained: refold-fields 2635 (67 TED-ADDRESS_PARTY carriers) + project 2636 (67 written: 66 corrected, 1 verified); 15/15 salted corrections serve result winners = their winner parties, owners only under ADDRESS_PARTY. Was: ready-for-agent — **unit 1 SIZED 2026-10-19**: `refold-fields` 2633 (`TED-ADDRESS_PARTY` in notice_ids, `expect: 1`, 04:49–04:57Z) aborted with **67** carriers, nothing written. Unit 2 (the guard + a 67-notice drain) in progress. Was: ready-for-agent — filed 2026-10-19 from issue 541 unit 2's review (wf_a8f42aac-9fa, scope lens, confirmed by an
adversarial verifier; pre-existing, not introduced by 541). Prevalence unknown: no fixture or census has shown an R2.0.9
T02 with owners yet. Next: size it (unit 1), then the guard (unit 2).
Kind: correctness (legacy results: a non-winner bound as a winner)
Relates to:
- 541 (the R2.0.8 twin: T02 section V read off the notice, its owners `PARTY_NAME_ADDRESS` kept out);
- 534 (owners refused as a role: `LEGACY_ROLE_REFUSED` lists `ADDRESS_PARTY`);
- 531 (`?winner=` reads result winners).

## Verify

    curl -s 'https://tenders.zebreus.click/v1/tenders/<an R2.0.9 T02 tender with owners>' | jq -c '[.parties[]|select(.role=="winner")|.organization_id], [.lot_results[].winners]'

- **done**: the result's winners are the operator(s) only; the owners appear in neither list.
- **open (2026-10-19, from the code)**: the owners' organizations would sit in `lot_results[].winners` beside the
  operator. No live exhibit is known yet — unit 1 finds one or shows there is none.

## What was found

An R2.0.9 MOVE T02 holds its award in `AWARD_CONTRACT`, which the r209 walker opens as a `LotResult` (`RES-n`,
`crates/ingest/src/r209/rules.rs` ~345). Inside it `AWARDED_CONTRACT` (a Marker, children walked), `CONTRACTORS` and
`CONTRACTOR` (Groups) hold `ADDRESS_CONTRACTOR` and then 0–100 `ADDRESS_PARTY` (`contact_party_move`, the operator's
owners; R2.0.9 MOVE.xsd `award_contract_move`). `ADDRESS_PARTY` is `Rule::Org`, so its id-ref is recorded on `RES-n`.
`read_legacy_results`' first arm, `(_, NoticeValue::Id { is_ref: true, scheme }) if scheme != Some("ojs")`, takes every
ref in the block as a direct winner, filtered only by the `dropped` name verdicts (void / placeholder / blank), which a
real owner name ("Landkreis X") never trips. So the owners become result winners and `?winner=` hits on them, while the
role layer refuses `ADDRESS_PARTY` for exactly this reason (`LEGACY_ROLE_REFUSED`, `#[cfg(test)]` documentation only — it
never reaches the results reader).

## Proposed units

1. **Size (S).** Count R2.0.9 notices carrying `TED-ADDRESS_PARTY` (`refold-fields` with `expect: 1` on
   `TED-ADDRESS_PARTY` in `notice_ids`, which aborts with the carrier count and writes nothing; a whole-table scan, so in
   a queue gap). Read one exhibit's served `lot_results` to confirm the owners are bound.
2. **Guard (S).** Exclude owner fields (`TED-ADDRESS_PARTY`, and any other `LEGACY_ROLE_REFUSED` entry that names an
   owner rather than a winner) from the wildcard winner arm; test with an R2.0.9-shaped `RES-1` carrying
   `ADDRESS_CONTRACTOR` and `ADDRESS_PARTY` (only the contractor is a winner). Drain the unit-1 cohort with the same
   `refold-fields` job run wet.

## Unit 1 — sized (2026-10-19)

`refold-fields` 2633 `{"profiles":["TED-ADDRESS_PARTY"],"tables":["notice_ids"],"expect":1}`: **67 notices** carry
the field (the sweep took ~8 min; project 2634 ran 0 notices). `ADDRESS_PARTY` is a MOVE-only element in R2.0.9, so these
are the R2.0.9 T02 awards that name owners. No bounded way to list them without the field index; the drain's corrections
(unit 2) name them.

## RESOLVED-VERIFIED (2026-10-19)

- **Unit 2** (21b2537): `LEGACY_NON_WINNER_REFS = ["TED-ADDRESS_PARTY"]` guards read_legacy_results' winner arm (the only
  non-winner Org element that can sit inside a LotResult, checked against r209/rules.rs and the refused-role list); tests pin
  the R2.0.9 T02 contractor-plus-owners block, an owners-only block (bare: clos-nw; with AWARDED_CONTRACT: NULL), every
  R2.0.8/R2.0.9 contractor field including R2.0.9 F13's nested WINNER > ADDRESS_WINNER, and the issue-384 guard. Built by
  wf_32e39867-ef0 (two review nits fixed). Gate green 1,392 s; deployed 05:45Z.
- **Drain:** refold-fields 2635 (`{"profiles":["TED-ADDRESS_PARTY"],"tables":["notice_ids"],"expect":67}`) + project 2636:
  67 written, **66 corrected**, 1 verified (D7 complete). `/v1/changes` 958809905 → 958810037: 66 tender changed rows
  (`551-lot-sum/b6/l_changed_571_ids.json`: these join issue 551 B6's L_changed, since winners feed path W's R1/R5/R6).
- **Verify:** 15 salted corrections (salt `571-`): each serves its owners only under the refused `ADDRESS_PARTY` role, and its
  `lot_results` winners equal its `winner` parties (e.g. 6386700 → 3302058 Trenitalia; 6874627 → 1189921 + 22396754).
