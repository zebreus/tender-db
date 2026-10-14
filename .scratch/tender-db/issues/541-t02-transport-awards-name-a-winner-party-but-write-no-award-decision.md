# 541 — 2016 public-transport awards (T02) name their operator as a `winner` party but write no award decision

Status: needs-triage — filed 2026-10-14 from issue 534's review round (all six sampled T02 tenders). Next: size the
T02 cohort and read whether its section V carries enough to write a result.
Kind: coverage (legacy results)
Relates to:
- 534 (where this was seen);
- 242 (the same shape for another legacy award container);
- 531 (`?winner=` reads result winners only).

## Verify

    curl -s https://tenders.zebreus.click/v1/tenders/5782055 | jq -c '[([.parties[]|select(.role=="winner")|.organization_id]), (.lot_results|length)]'

- **done**: the operator is in `lot_results[].winners`, so `?winner=` finds it.
- **open (2026-10-14)**: `[[1953406],0]`: the `winner` party is present and `lot_results` is empty.

## What was found

A T02 award (R2.0.8 MOVE, Regulation 1370/2007 public-transport contracts) names its operator under
`ECONOMIC_OPERATOR_NAME_ADDRESS`. That folds to `winner`, so the operator shows as a `winner` party. But
`AWARD_CONTRACT_MOVE` is a group, not a result section, so no award decision is written: `lot_results` is empty and
`?winner=` misses the operator.
- **Sample.** All six T02 tenders sampled in 534's 2,427-tender transport read look like this, for example 5782055
  (382248-2016, Barnimer Busgesellschaft) and 5804664 (427137-2016).
- **Owners.** The operator's public owners are served under `PARTY_NAME_ADDRESS`, refused in 534.

## Proposed units

1. **Size (S).** Count the T02 cohort (`TED-AWARD_CONTRACT_MOVE` field ids in `notice_ids`, with an in-app or bounded
   read).
2. **Build (S–M).** Write a result from the T02 section V, as 242 did for its container: one lot result whose winner
   is `ECONOMIC_OPERATOR_NAME_ADDRESS`, plus the award value and date if published. Then refold the cohort.
