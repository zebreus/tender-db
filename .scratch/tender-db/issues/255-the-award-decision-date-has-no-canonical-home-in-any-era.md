# 255 — the award DECISION date has no canonical home, in any era

Status: ALL THREE SLICES DONE 2026-08-20 — eForms BT-1451 → `tender_version_contracts.decided_*`;
the legacy form eras' `CONTRACT_AWARD_DATE` → `tender_version_lot_results.decided_*` (also in
`v_lot_results` and `v_awards`); and the text era's prose date → the same legacy field id, so it rides
slice 2's projection with no mapping change. All migrated, projected, tested against committed
fixtures and falsified. What remains is machine time: a refold for the form eras, a re-parse for the
text era, and then the coverage numbers
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

- **Slice 2, r208/r209 — DONE**: `TED-CONTRACT_AWARD_DATE` into `RawLotResult.decided` →
  `LotResultState.decided` → `tender_version_lot_results.decided_utc/offset/has_time`, plus the same
  three columns in `v_lot_results` and `v_awards` so "who won what, for how much, WHEN" is one query.
  Volume: r2.0.9 alone publishes 2,098,599 award notices at 100 % result density, the largest cohort
  of the three. Gated by `the_legacy_award_block_carries_its_decision_date` on the r209 defence
  fixture (14.12.2018, offsetless, read through `v_awards` as well as the satellite) and falsified:
  without the projection arm the column reads NULL.
- **Slice 3, text era — DONE**: `award_date` + `read_dmy` in `text/parse.rs`, a post-pass gated on
  `TXT-TD == "7"` exactly like `claim_awarded_value`, emitting **`TED-CONTRACT_AWARD_DATE`** — the
  legacy eras' own field id — into every result block the body yielded. So it rides slice 2's
  projection arm to `tender_version_lot_results.decided_*` with no mapping change at all, the same
  trick the price plays with `TED-VAL_TOTAL`.

  Both label spellings are verified against committed fixtures rather than guessed: the 1993 daily
  writes `Date of award:` in dozens of its 199 records, and `2005-can-154-2005` writes
  `VI.3)  Date of contract award: 25.11.2004.` — so one label list covers the numbered and the
  sectioned form, and the shorter entry is a prefix of `Date of award of the contract:` too.

  Gates: the 1993 daily yields **72 award dates from 199 records**, every one on a `RES-` block and
  every one inside 1990–1994 (a rolled-over typo or a mis-scanned year lands outside that window,
  which is what makes the test a gate rather than a count); the 2005 CAN yields its date TWICE,
  because it names two suppliers and each award carries the notice's single date.

  **The trap here was the shared date parser.** `value::date_from_parts` NORMALISES out-of-range
  parts instead of refusing them: `30.13.2001` comes back as 2002-01-30 and `31.2.2001` as
  2001-03-03. Reading dates out of XML that a schema already validated, that leniency never shows;
  reading them out of 3.8M prose bodies, it would file a typo as a real day in a neighbouring month.
  So `read_dmy` range-checks day and month (with a leap-year rule) BEFORE handing the parts over, and
  the test pins all four refusals. Left alone in the shared helper, deliberately: the XML callers
  rely on nothing here, and tightening a parser five eras use is not this issue's business.

  Two things it deliberately does not do: a two-digit year is refused (ambiguous across an era
  spanning 1993–2010), and a body stating two DIFFERENT dates claims nothing (the same rule the price
  follows). A body with no winner has no result block, so its date stays in the prose — the model
  hangs an award date on an award, not on a Tender.

One more thing the legacy fixture settles for issue 244, recorded here because I found it while
reading the same block: `<OFFERS_RECEIVED_NUMBER>6</OFFERS_RECEIVED_NUMBER>` sits right beside
`CONTRACT_AWARD_DATE`, and the legacy reader ALREADY projects it as a result statistic
(`LotResultState.statistics`). So the text era's tenders-received count — the other field issue 244
lists as needing a destination decision — needs no new column at all: it has a home, and only the
extraction is missing. Slice 2's test asserts that too rather than trusting my reading of the code:
`SELECT count FROM tender_version_result_stats WHERE kind = 'tenders'` is 6 on the same fixture.

### Both dates, side by side, and why they are not one column

An eForms CAN states BOTH: BT-1451 on the settled contract and BT-145 beside it. A legacy award
notice states ONE, on the award block, and has no contract row at all. A reader asking "when was this
awarded?" therefore looks in `tender_version_lot_results.decided_*` for 1993–2016 and in
`tender_version_contracts.decided_*` for 2023 onward — which is not a wart but the shape of the
sources: the legacy form has no notion of a settled contract, and eForms' decision date is a property
of one. Collapsing them into a single column would have to invent a contract for the legacy eras or
throw away which contract an eForms decision belongs to.
