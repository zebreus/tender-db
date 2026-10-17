# 568 — legacy R2.0.8 contract award notices never attach their awards to a lot, even on a multi-lot Tender

Status: needs-triage — filed 2026-10-17 from issue 567's measurement (wf_3865d965-f11). Next: size it (R2.0.8 CANs whose
Tender has 2 + lots and whose award blocks carry LOT_NUMBER) and decide whether the award's LOT_NUMBER may be matched to
the contract notice's annex-B lots (a cross-notice match, not a same-notice one).
Kind: completeness (legacy lot results: R2.0.8 awards served tender-scoped)
Relates to:
- 567 (same-notice ITEM/LOT_NO mapping in read_legacy_results);
- 534 / 490 (legacy result reading).

## Verify

    curl -s https://tenders.zebreus.click/v1/tenders/4748635 | python3 -c "import json,sys; d=json.load(sys.stdin); print([r.get('lot') for r in d.get('lot_results', [])])"

- **done**: an R2.0.8 award block with LOT_NUMBER n on a Tender whose contract notice published lot n serves its result on
  that lot (or the issue records why a cross-notice match is refused).
- **open (2026-10-17)**: every R2.0.8 award result serves `lot: null`.

## What was found

R2.0.8 award forms repeat `AWARD_OF_CONTRACT` with `LOT_NUMBER` and carry no `OBJECT_DESCR` lot section, so
`read_legacy_results`' same-notice lot map is empty and every award resolves to `lot_key = None`. In 72 of 100 sampled
R2.0.8 multi-lot CANs (2012–2015) the Tender has 2 or more lots from its contract notice's annex B; their awards stay
tender-scoped. Spot-checked on the API: 4748635, 4730243, 4945774, 4840245, 5128122, 5086044, 5340243, 5274468 — every
result row from the CAN serves `lot: null`.
