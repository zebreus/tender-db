# 255 — the award DECISION date has no canonical home, in any era

Status: SLICE 1 DONE 2026-08-20 (eForms BT-1451 → `tender_version_contracts.decided_*`, migration +
projection + tests + falsified). Slices 2 and 3 open: the legacy form eras' `CONTRACT_AWARD_DATE` and
the text era's `Date of award:` prose, both of which sit on the award BLOCK and want a
`lot_results`-scope home rather than a contract one
Kind: canonical modelling gap (a published fact with nowhere to land), spanning every era
Blocked by: —
Relates to: 244 (the text era's award date, listed there as needing "a canonical destination
decision" — this issue is that decision), 13 (results layer), ADR-0001 (traceability), 236/ADR-0011

## What

Three eras publish when a contract was AWARDED, and none of it reaches the canonical layer.

**eForms** — `BT-1451`, claimed by the parse layer on the settled-contract section of every CAN,
beside `BT-145` (the conclusion date) which HAS a home:

    can-maximal-sdk17          BT-1451-Contract  2023-03-22+02:00   CON-0001, CON-0002
                               BT-145-Contract   2023-03-23+02:00   → tender_version_contracts.concluded_*
    can-subdesc-00570953-2025  BT-1451-Contract  2025-01-20+02:00   (47 days before its signature)
    can-cvd-lot-00054478-2025  BT-1451-Contract  2024-11-14+01:00

**r208/r209** — `<CONTRACT_AWARD_DATE><DAY>14</DAY><MONTH>12</MONTH><YEAR>2018</YEAR>` inside
`AWARD_OF_CONTRACT`, in 24 places across the committed fixtures. `rules.rs` types it as a date, so it
is in the parse layer under `TED-CONTRACT_AWARD_DATE` and `DATES` has never had an entry for it.

**text 1993–2010** — `3. Date of award: 30.3.2001.` in the body prose, ~2,991 bodies per package
(measured for issue 244).

`DATES` maps six canonical names — submission_deadline, participation_deadline, opening_date,
additional_information_deadline, duration_start, duration_end — and not one of them is an award date.
So the answer to "when was this awarded?" is unavailable for the entire corpus, in every era, while
being published in all three.

## The trap, which cost me a wrong assertion before I found it

UBL 2.3 forces a `cac:TenderResult/cbc:AwardDate` onto every CAN, and the SDK models it only to
swallow it — `OPT-999`, documented in `eforms/value.rs` as *"not award data (the real award date is
`efbc:AwardDate` in the result extension)"*. My first test asserted the decision date on
`eforms-chain/4-can-29-380868-2026.xml`, whose ONLY `cbc:AwardDate` is that dummy: it would have
recorded `2026-06-02` — a publisher's placeholder — as the buyer's decision. The results-layer test
now asserts the opposite for that notice: a CAN carrying only the dummy stores NULL.

## Slice 1 — eForms, done

Three columns on `tender_version_contracts` beside the conclusion date, because BT-1451 is
contract-scoped in the SDK (section `CON-000n`), and the plumbing for a contract-scoped date already
existed for BT-145:

    decided_utc  decided_offset  decided_has_time

`ContractState.decided`, the projection arm on `("BT-1451", NoticeValue::Date { .. })`, the fold's
insert (9 → 12 columns), the read path's `ContractRow.decided`, and `"decided"` in the `/v1` contract
JSON. Migration adds the columns to an existing table; a row folded before them reads NULL, which is
the honest answer and is what the migration test asserts — *"we did not record it"* and *"the same day
as the signature"* are different claims.

Gated by `the_winner_decision_date_lands_beside_the_conclusion_date`, which asserts both contracts of
`can-maximal-sdk17` carry both dates and that `decided_utc < concluded_utc` — the buyer decides before
the contract is signed, which is also what makes the two visibly distinct facts rather than one
restated. Falsified: removing the projection arm fails it with `left: 0, right: 2`.

Needs an eForms refold for the acceptance number (a projection change, not a parse one).

## Slices 2 and 3 — the eras whose award date is NOT contract-scoped

Both remaining eras put the date on the award BLOCK, which the projection reads as a `LotResult`, and
neither has a contract graph at all (*"no bid/contract graph in the legacy schema"*, `read_legacy_results`).
So slice 1's home is wrong for them, and the right one is a `decided_*` triple on
`tender_version_lot_results` — the same shape one table over.

- **Slice 2, r208/r209**: `TED-CONTRACT_AWARD_DATE` into `LotResultState`, which needs the same
  three columns on the lot-result satellite. Volume: r2.0.9 alone publishes 2,098,599 award notices at
  100 % result density, so this is the largest cohort of the three.
- **Slice 3, text era**: extract `Date of award:` from the body the way `awarded_value` extracts the
  price (issue 244 slices 4–9), then route it through the same `LotResultState` field. Deliberately
  after slice 2, so the destination exists before the extraction does.

One more thing the legacy fixture settles for issue 244, recorded here because I found it while
reading the same block: `<OFFERS_RECEIVED_NUMBER>6</OFFERS_RECEIVED_NUMBER>` sits right beside
`CONTRACT_AWARD_DATE`, and the legacy reader ALREADY projects it as a result statistic
(`LotResultState.statistics`). So the text era's tenders-received count — the other field issue 244
lists as needing a destination decision — needs no new column at all: it has a home, and only the
extraction is missing.
